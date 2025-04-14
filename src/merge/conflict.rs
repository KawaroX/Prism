//! Conflict detection and resolution for merging docx documents.

use similar::{Algorithm, ChangeTag, TextDiff};
use uuid::Uuid;

use crate::docx::model::{Document, Paragraph, Run, RunContent, RunProperties};
use crate::error::{Error, Result};
use crate::merge::{
    Conflict, ConflictContent, ConflictLocation, ConflictType, FormattingProperties, Resolution,
};

/// Helper for conflict detection and resolution
pub struct ConflictResolver;

impl ConflictResolver {
    /// Create a new conflict resolver
    pub fn new() -> Self {
        ConflictResolver
    }

    /// Detect conflicts between two sets of changes
    pub fn detect_conflicts(
        &self,
        base: &Document,
        doc1: &Document,
        doc2: &Document,
        auto_resolve: bool,
    ) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // Compare paragraphs to detect conflicts
        let base_paragraphs = &base.body.paragraphs;
        let doc1_paragraphs = &doc1.body.paragraphs;
        let doc2_paragraphs = &doc2.body.paragraphs;

        let max_paragraphs = base_paragraphs
            .len()
            .max(doc1_paragraphs.len())
            .max(doc2_paragraphs.len());

        for i in 0..max_paragraphs {
            let base_para = base_paragraphs.get(i);
            let doc1_para = doc1_paragraphs.get(i);
            let doc2_para = doc2_paragraphs.get(i);

            match (base_para, doc1_para, doc2_para) {
                // 都存在，检查内容冲突
                (Some(base_p), Some(doc1_p), Some(doc2_p)) => {
                    // 提取文本用于比较
                    let base_text = self.extract_paragraph_text(base_p);
                    let doc1_text = self.extract_paragraph_text(doc1_p);
                    let doc2_text = self.extract_paragraph_text(doc2_p);

                    // 检查是否都从基础版本改变了，并且彼此不同
                    if doc1_text != base_text && doc2_text != base_text && doc1_text != doc2_text {
                        // 使用差异算法确定冲突的更精确位置
                        let conflict_segments =
                            self.find_conflict_segments(&base_text, &doc1_text, &doc2_text);

                        for segment in conflict_segments {
                            conflicts.push(Conflict {
                                id: Uuid::new_v4().to_string(),
                                location: ConflictLocation {
                                    paragraph_index: i,
                                    start_pos: segment.0,
                                    end_pos: segment.1,
                                },
                                content1: ConflictContent::Paragraph(
                                    doc1_text[segment.0..segment.1].to_string(),
                                ),
                                content2: ConflictContent::Paragraph(
                                    doc2_text[segment.0..segment.1].to_string(),
                                ),
                                base_content: ConflictContent::Paragraph(
                                    if segment.0 < base_text.len() && segment.1 <= base_text.len() {
                                        base_text[segment.0..segment.1].to_string()
                                    } else {
                                        "".to_string()
                                    },
                                ),
                                conflict_type: ConflictType::ContentConflict,
                                format1: None,
                                format2: None,
                                base_format: None,
                            });
                        }
                    }

                    // 检查格式冲突
                    self.detect_formatting_conflicts(i, base_p, doc1_p, doc2_p, &mut conflicts);
                }

                // Doc1删除了，而Doc2修改了
                (Some(base_p), None, Some(doc2_p)) => {
                    conflicts.push(Conflict {
                        id: Uuid::new_v4().to_string(),
                        location: ConflictLocation {
                            paragraph_index: i,
                            start_pos: 0,
                            end_pos: self.extract_paragraph_text(base_p).len(),
                        },
                        content1: ConflictContent::Paragraph("".to_string()), // 已删除
                        content2: ConflictContent::Paragraph(self.extract_paragraph_text(doc2_p)),
                        base_content: ConflictContent::Paragraph(
                            self.extract_paragraph_text(base_p),
                        ),
                        conflict_type: ConflictType::ModifyDeleteConflict,
                        format1: None,
                        format2: None,
                        base_format: None,
                    });
                }

                // Doc2删除了，而Doc1修改了
                (Some(base_p), Some(doc1_p), None) => {
                    conflicts.push(Conflict {
                        id: Uuid::new_v4().to_string(),
                        location: ConflictLocation {
                            paragraph_index: i,
                            start_pos: 0,
                            end_pos: self.extract_paragraph_text(base_p).len(),
                        },
                        content1: ConflictContent::Paragraph(self.extract_paragraph_text(doc1_p)),
                        content2: ConflictContent::Paragraph("".to_string()), // 已删除
                        base_content: ConflictContent::Paragraph(
                            self.extract_paragraph_text(base_p),
                        ),
                        conflict_type: ConflictType::ModifyDeleteConflict,
                        format1: None,
                        format2: None,
                        base_format: None,
                    });
                }

                // 两者都添加了新段落在相同位置
                (None, Some(doc1_p), Some(doc2_p)) => {
                    let doc1_text = self.extract_paragraph_text(doc1_p);
                    let doc2_text = self.extract_paragraph_text(doc2_p);

                    // 只有当内容不同时才算冲突
                    if doc1_text != doc2_text {
                        conflicts.push(Conflict {
                            id: Uuid::new_v4().to_string(),
                            location: ConflictLocation {
                                paragraph_index: i,
                                start_pos: 0,
                                end_pos: 0, // 新段落
                            },
                            content1: ConflictContent::Paragraph(doc1_text),
                            content2: ConflictContent::Paragraph(doc2_text),
                            base_content: ConflictContent::Paragraph("".to_string()),
                            conflict_type: ConflictType::InsertInsertConflict,
                            format1: None,
                            format2: None,
                            base_format: None,
                        });
                    }
                }

                // 检测结构冲突 - 当文档结构变化很大时
                (_, _, _) => {
                    // 只有当段落索引在基础文档中存在，但在至少一个修改文档中被明显地改变结构时
                    if base_para.is_some() && (doc1_para.is_some() != doc2_para.is_some()) {
                        // 这里可能是结构冲突，但我们需要更详细的分析
                        if self.is_structure_change(
                            i,
                            base_paragraphs,
                            doc1_paragraphs,
                            doc2_paragraphs,
                        ) {
                            conflicts.push(Conflict {
                                id: Uuid::new_v4().to_string(),
                                location: ConflictLocation {
                                    paragraph_index: i,
                                    start_pos: 0,
                                    end_pos: 0,
                                },
                                content1: ConflictContent::Structure(format!(
                                    "结构变更 Doc1: {}",
                                    if doc1_para.is_some() {
                                        "段落存在"
                                    } else {
                                        "段落不存在"
                                    }
                                )),
                                content2: ConflictContent::Structure(format!(
                                    "结构变更 Doc2: {}",
                                    if doc2_para.is_some() {
                                        "段落存在"
                                    } else {
                                        "段落不存在"
                                    }
                                )),
                                base_content: ConflictContent::Structure(
                                    "Base: 段落存在".to_string(),
                                ),
                                conflict_type: ConflictType::StructureConflict,
                                format1: None,
                                format2: None,
                                base_format: None,
                            });
                        }
                    }
                }
            }
        }

        // 检查表格和其他结构元素的冲突
        self.detect_table_conflicts(base, doc1, doc2, &mut conflicts);

        // 如果启用了自动解决，尝试解决简单冲突
        if auto_resolve {
            conflicts = self.auto_resolve_conflicts(conflicts);
        }

        conflicts
    }

    /// 查找具体的冲突片段位置
    fn find_conflict_segments(
        &self,
        base_text: &str,
        text1: &str,
        text2: &str,
    ) -> Vec<(usize, usize)> {
        let mut segments = Vec::new();

        // 使用差异算法找出具体冲突位置
        let diff1 = TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .timeout(std::time::Duration::from_secs(2))
            .diff_chars(base_text, text1);

        let diff2 = TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .timeout(std::time::Duration::from_secs(2))
            .diff_chars(base_text, text2);

        // 创建两个文档的变更映射
        let mut changes1 = Vec::new();
        let mut changes2 = Vec::new();

        let mut pos1 = 0;
        for change in diff1.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    changes1.push((pos1, pos1 + change.value().len(), ChangeTag::Delete));
                    pos1 += change.value().len();
                }
                ChangeTag::Insert => {
                    changes1.push((pos1, pos1, ChangeTag::Insert));
                }
                ChangeTag::Equal => {
                    pos1 += change.value().len();
                }
            }
        }

        let mut pos2 = 0;
        for change in diff2.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    changes2.push((pos2, pos2 + change.value().len(), ChangeTag::Delete));
                    pos2 += change.value().len();
                }
                ChangeTag::Insert => {
                    changes2.push((pos2, pos2, ChangeTag::Insert));
                }
                ChangeTag::Equal => {
                    pos2 += change.value().len();
                }
            }
        }

        // 找出同时在两个文档中被修改的片段
        for (start1, end1, tag1) in &changes1 {
            for (start2, end2, tag2) in &changes2 {
                if start1 == start2 || (start1 < end2 && start2 < end1) {
                    // 重叠区域表示冲突
                    let conflict_start = start1.min(start2);
                    let conflict_end = end1.max(end2);

                    // 如果不是相同类型的改变，或者是同类型但在不同位置，那就是冲突
                    if tag1 != tag2 || (tag1 == tag2 && start1 != start2) {
                        segments.push((*conflict_start, *conflict_end));
                    }
                }
            }
        }

        // 合并重叠的片段
        if !segments.is_empty() {
            segments.sort_by(|a, b| a.0.cmp(&b.0));

            let mut merged_segments = Vec::new();
            let mut current = segments[0];

            for &(start, end) in segments.iter().skip(1) {
                if start <= current.1 {
                    // 片段重叠，合并
                    current.1 = current.1.max(end);
                } else {
                    // 没有重叠，添加当前片段并开始新的
                    merged_segments.push(current);
                    current = (start, end);
                }
            }

            merged_segments.push(current);
            return merged_segments;
        }

        // 如果没找到具体片段但确实有冲突，返回整个段落
        if base_text != text1 && base_text != text2 && text1 != text2 {
            return vec![(0, base_text.len().max(text1.len().max(text2.len())))];
        }

        Vec::new()
    }

    /// 检测格式冲突
    fn detect_formatting_conflicts(
        &self,
        para_idx: usize,
        base_para: &Paragraph,
        doc1_para: &Paragraph,
        doc2_para: &Paragraph,
        conflicts: &mut Vec<Conflict>,
    ) {
        // 提取格式属性
        for i in 0..base_para
            .runs
            .len()
            .min(doc1_para.runs.len())
            .min(doc2_para.runs.len())
        {
            // 获取运行
            let base_run = &base_para.runs[i];
            let doc1_run = &doc1_para.runs[i];
            let doc2_run = &doc2_para.runs[i];

            // 检查格式是否发生了变化
            if self.props_changed(&base_run.properties, &doc1_run.properties)
                && self.props_changed(&base_run.properties, &doc2_run.properties)
                && !self.props_equal(&doc1_run.properties, &doc2_run.properties)
            {
                // 计算运行在段落中的位置
                let mut run_start = 0;
                for j in 0..i {
                    run_start += self.extract_run_text(&base_para.runs[j]).len();
                }
                let run_end = run_start + self.extract_run_text(base_run).len();

                // 创建格式属性
                let base_format = FormattingProperties {
                    text: self.extract_run_text(base_run),
                    bold: base_run.properties.bold,
                    italic: base_run.properties.italic,
                    underline: base_run.properties.underline,
                    font_size: base_run.properties.size,
                    font_name: base_run.properties.font.clone(),
                    color: base_run.properties.color.clone(),
                };

                let format1 = FormattingProperties {
                    text: self.extract_run_text(doc1_run),
                    bold: doc1_run.properties.bold,
                    italic: doc1_run.properties.italic,
                    underline: doc1_run.properties.underline,
                    font_size: doc1_run.properties.size,
                    font_name: doc1_run.properties.font.clone(),
                    color: doc1_run.properties.color.clone(),
                };

                let format2 = FormattingProperties {
                    text: self.extract_run_text(doc2_run),
                    bold: doc2_run.properties.bold,
                    italic: doc2_run.properties.italic,
                    underline: doc2_run.properties.underline,
                    font_size: doc2_run.properties.size,
                    font_name: doc2_run.properties.font.clone(),
                    color: doc2_run.properties.color.clone(),
                };

                // 添加冲突
                conflicts.push(Conflict {
                    id: Uuid::new_v4().to_string(),
                    location: ConflictLocation {
                        paragraph_index: para_idx,
                        start_pos: run_start,
                        end_pos: run_end,
                    },
                    content1: ConflictContent::Text(format1.text.clone()),
                    content2: ConflictContent::Text(format2.text.clone()),
                    base_content: ConflictContent::Text(base_format.text.clone()),
                    conflict_type: ConflictType::FormattingConflict,
                    format1: Some(format1),
                    format2: Some(format2),
                    base_format: Some(base_format),
                });
            }
        }
    }

    /// 检查属性是否已更改
    fn props_changed(&self, base_props: &RunProperties, new_props: &RunProperties) -> bool {
        base_props.bold != new_props.bold
            || base_props.italic != new_props.italic
            || base_props.underline != new_props.underline
            || base_props.size != new_props.size
            || base_props.font != new_props.font
            || base_props.color != new_props.color
    }

    /// 检查两组属性是否相等
    fn props_equal(&self, props1: &RunProperties, props2: &RunProperties) -> bool {
        props1.bold == props2.bold
            && props1.italic == props2.italic
            && props1.underline == props2.underline
            && props1.size == props2.size
            && props1.font == props2.font
            && props1.color == props2.color
    }

    /// 将格式属性转换为字符串
    fn format_to_string(&self, formats: Vec<&RunProperties>) -> String {
        let mut result = String::new();
        for (i, format) in formats.iter().enumerate() {
            result.push_str(&format!(
                "Run {}: Bold={}, Italic={}, Underline={}, Size={:?}, Font={:?}, Color={:?}\n",
                i + 1,
                format.bold,
                format.italic,
                format.underline,
                format.size,
                format.font,
                format.color
            ));
        }
        result
    }

    /// 检测结构是否有较大改变
    fn is_structure_change(
        &self,
        index: usize,
        base_paragraphs: &[Paragraph],
        doc1_paragraphs: &[Paragraph],
        doc2_paragraphs: &[Paragraph],
    ) -> bool {
        // 检查这个位置周围的段落是否有大的变化
        let context = 2; // 检查上下两个段落

        let start = if index > context { index - context } else { 0 };
        let end = (index + context + 1).min(base_paragraphs.len());

        let mut structure_count_base = 0;
        let mut structure_count_doc1 = 0;
        let mut structure_count_doc2 = 0;

        // 统计区域内段落数量
        for i in start..end {
            if i < base_paragraphs.len() {
                structure_count_base += 1;
            }

            if i < doc1_paragraphs.len() {
                structure_count_doc1 += 1;
            }

            if i < doc2_paragraphs.len() {
                structure_count_doc2 += 1;
            }
        }

        // 如果区域段落数量差异较大，可能是结构变化
        let diff1 = (structure_count_base as i32 - structure_count_doc1 as i32).abs();
        let diff2 = (structure_count_base as i32 - structure_count_doc2 as i32).abs();
        let diff12 = (structure_count_doc1 as i32 - structure_count_doc2 as i32).abs();

        diff1 > 0 && diff2 > 0 && diff12 > 0
    }

    /// 检测表格冲突
    fn detect_table_conflicts(
        &self,
        base: &Document,
        doc1: &Document,
        doc2: &Document,
        conflicts: &mut Vec<Conflict>,
    ) {
        // 检查表格数量
        if base.body.tables.len() != doc1.body.tables.len()
            && base.body.tables.len() != doc2.body.tables.len()
            && doc1.body.tables.len() != doc2.body.tables.len()
        {
            // 表格数量发生变化，添加结构冲突
            conflicts.push(Conflict {
                id: Uuid::new_v4().to_string(),
                location: ConflictLocation {
                    paragraph_index: 0, // 表格不在段落中
                    start_pos: 0,
                    end_pos: 0,
                },
                content1: ConflictContent::Structure(format!(
                    "Doc1: {} 个表格",
                    doc1.body.tables.len()
                )),
                content2: ConflictContent::Structure(format!(
                    "Doc2: {} 个表格",
                    doc2.body.tables.len()
                )),
                base_content: ConflictContent::Structure(format!(
                    "Base: {} 个表格",
                    base.body.tables.len()
                )),
                conflict_type: ConflictType::StructureConflict,
                format1: None,
                format2: None,
                base_format: None,
            });
            return;
        }

        // 检查每个表格的内容
        for (i, base_table) in base.body.tables.iter().enumerate() {
            if let (Some(table1), Some(table2)) = (doc1.body.tables.get(i), doc2.body.tables.get(i))
            {
                // 检查行数
                if base_table.rows.len() != table1.rows.len()
                    && base_table.rows.len() != table2.rows.len()
                    && table1.rows.len() != table2.rows.len()
                {
                    conflicts.push(Conflict {
                        id: Uuid::new_v4().to_string(),
                        location: ConflictLocation {
                            paragraph_index: 0,
                            start_pos: 0,
                            end_pos: 0,
                        },
                        content1: ConflictContent::Structure(format!(
                            "Doc1表格{}: {} 行",
                            i + 1,
                            table1.rows.len()
                        )),
                        content2: ConflictContent::Structure(format!(
                            "Doc2表格{}: {} 行",
                            i + 1,
                            table2.rows.len()
                        )),
                        base_content: ConflictContent::Structure(format!(
                            "Base表格{}: {} 行",
                            i + 1,
                            base_table.rows.len()
                        )),
                        conflict_type: ConflictType::StructureConflict,
                        format1: None,
                        format2: None,
                        base_format: None,
                    });
                    continue;
                }

                // 检查每行的单元格数量
                for (j, base_row) in base_table.rows.iter().enumerate() {
                    if let (Some(row1), Some(row2)) = (table1.rows.get(j), table2.rows.get(j)) {
                        if base_row.cells.len() != row1.cells.len()
                            && base_row.cells.len() != row2.cells.len()
                            && row1.cells.len() != row2.cells.len()
                        {
                            conflicts.push(Conflict {
                                id: Uuid::new_v4().to_string(),
                                location: ConflictLocation {
                                    paragraph_index: 0,
                                    start_pos: 0,
                                    end_pos: 0,
                                },
                                content1: ConflictContent::Structure(format!(
                                    "Doc1表格{}行{}: {} 个单元格",
                                    i + 1,
                                    j + 1,
                                    row1.cells.len()
                                )),
                                content2: ConflictContent::Structure(format!(
                                    "Doc2表格{}行{}: {} 个单元格",
                                    i + 1,
                                    j + 1,
                                    row2.cells.len()
                                )),
                                base_content: ConflictContent::Structure(format!(
                                    "Base表格{}行{}: {} 个单元格",
                                    i + 1,
                                    j + 1,
                                    base_row.cells.len()
                                )),
                                conflict_type: ConflictType::StructureConflict,
                                format1: None,
                                format2: None,
                                base_format: None,
                            });
                            continue;
                        }

                        // 检查单元格内容
                        for (k, base_cell) in base_row.cells.iter().enumerate() {
                            if let (Some(cell1), Some(cell2)) =
                                (row1.cells.get(k), row2.cells.get(k))
                            {
                                // 提取单元格文本
                                let base_text = self.extract_cell_text(base_cell);
                                let doc1_text = self.extract_cell_text(cell1);
                                let doc2_text = self.extract_cell_text(cell2);

                                if base_text != doc1_text
                                    && base_text != doc2_text
                                    && doc1_text != doc2_text
                                {
                                    conflicts.push(Conflict {
                                        id: Uuid::new_v4().to_string(),
                                        location: ConflictLocation {
                                            paragraph_index: 0,
                                            start_pos: 0,
                                            end_pos: 0,
                                        },
                                        content1: ConflictContent::Paragraph(doc1_text),
                                        content2: ConflictContent::Paragraph(doc2_text),
                                        base_content: ConflictContent::Paragraph(base_text),
                                        conflict_type: ConflictType::ContentConflict,
                                        format1: None,
                                        format2: None,
                                        base_format: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// 提取单元格文本
    fn extract_cell_text(&self, cell: &crate::docx::model::TableCell) -> String {
        let mut text = String::new();
        for paragraph in &cell.paragraphs {
            text.push_str(&self.extract_paragraph_text(paragraph));
            text.push('\n');
        }
        text
    }

    /// 尝试自动解决简单冲突
    fn auto_resolve_conflicts(&self, conflicts: Vec<Conflict>) -> Vec<Conflict> {
        let mut unresolved_conflicts = Vec::new();

        for conflict in conflicts {
            match conflict.conflict_type {
                // 尝试自动解决插入冲突
                ConflictType::InsertInsertConflict => {
                    // 如果两个插入内容有明显的关联（如一个是另一个的扩展），我们可以选择更长的那个
                    if let (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) =
                        (&conflict.content1, &conflict.content2)
                    {
                        if text1.contains(text2) || text2.contains(text1) {
                            // 一个是另一个的子字符串，我们可以自动解决
                            // 不添加到未解决冲突列表中
                            continue;
                        }

                        // 标题行对应关系检查
                        if (text1.starts_with("1.") && text2.starts_with("1."))
                            || (text1.starts_with("一、") && text2.starts_with("一、"))
                        {
                            // 可能是相同层次的标题，我们需要用户决定
                            unresolved_conflicts.push(conflict);
                            continue;
                        }

                        // 简单文本差异不大
                        if self.text_similarity(text1, text2) > 0.7 {
                            // 内容相似度高，可以自动选择更长的
                            continue;
                        }
                    }

                    // 无法自动解决
                    unresolved_conflicts.push(conflict);
                }

                // 尝试自动解决修改/删除冲突
                ConflictType::ModifyDeleteConflict => {
                    // 通常保留修改而不是删除更安全
                    // 但这需要用户确认，所以还是添加到未解决列表
                    unresolved_conflicts.push(conflict);
                }

                // 尝试解决格式冲突
                ConflictType::FormattingConflict => {
                    // 格式冲突通常涉及复杂权衡，很难自动解决
                    unresolved_conflicts.push(conflict);
                }

                // 结构冲突几乎不可能自动解决
                ConflictType::StructureConflict => {
                    unresolved_conflicts.push(conflict);
                }

                // 内容冲突
                ConflictType::ContentConflict => {
                    // 尝试检查是否是简单的添加和删除组合
                    if let (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) =
                        (&conflict.content1, &conflict.content2)
                    {
                        // 计算文本相似度，如果相似度高可能可以自动解决
                        let similarity = self.text_similarity(text1, text2);
                        if similarity > 0.8 {
                            // 内容非常相似，可以选择较长的文本
                            continue;
                        }

                        // 如果一个明显包含另一个，可能是添加和编辑的组合
                        if text1.contains(text2) || text2.contains(text1) {
                            // 可以选择较长的文本，不添加到未解决列表
                            continue;
                        }
                    }

                    // 无法自动解决
                    unresolved_conflicts.push(conflict);
                }
            }
        }

        unresolved_conflicts
    }

    /// 计算两个文本的相似度 (0.0-1.0)
    fn text_similarity(&self, text1: &str, text2: &str) -> f64 {
        if text1.is_empty() && text2.is_empty() {
            return 1.0;
        }

        if text1.is_empty() || text2.is_empty() {
            return 0.0;
        }

        // 简单的最长公共子序列相似度计算
        let lcs_length = self.longest_common_subsequence(text1, text2);
        let max_length = text1.len().max(text2.len());

        lcs_length as f64 / max_length as f64
    }

    /// 计算最长公共子序列长度
    fn longest_common_subsequence(&self, text1: &str, text2: &str) -> usize {
        let chars1: Vec<char> = text1.chars().collect();
        let chars2: Vec<char> = text2.chars().collect();

        let m = chars1.len();
        let n = chars2.len();

        // 创建DP表
        let mut dp = vec![vec![0; n + 1]; m + 1];

        for i in 1..=m {
            for j in 1..=n {
                if chars1[i - 1] == chars2[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        dp[m][n]
    }

    /// 从段落中提取文本
    fn extract_paragraph_text(&self, paragraph: &crate::docx::model::Paragraph) -> String {
        let mut text = String::new();

        for run in &paragraph.runs {
            for content in &run.contents {
                if let crate::docx::model::RunContent::Text(t) = content {
                    text.push_str(t);
                } else if let crate::docx::model::RunContent::Break = content {
                    text.push('\n');
                } else if let crate::docx::model::RunContent::Tab = content {
                    text.push('\t');
                }
            }
        }

        text
    }

    /// 从运行中提取文本
    fn extract_run_text(&self, run: &Run) -> String {
        let mut text = String::new();
        for content in &run.contents {
            if let RunContent::Text(t) = content {
                text.push_str(t);
            } else if let RunContent::Break = content {
                text.push('\n');
            } else if let RunContent::Tab = content {
                text.push('\t');
            }
        }
        text
    }

    /// 应用分辨设置到冲突
    // 修改 apply_resolution 方法的签名，不再期望可变引用，而是返回新文档
    pub fn apply_resolution(
        &self,
        document: &Document,
        conflict: &Conflict,
        resolution: &Resolution,
    ) -> Result<Document> {
        // 创建文档副本
        let mut result = document.clone();

        // 根据解决方案应用更改
        match resolution {
            Resolution::UseFirst => {
                // 使用文档1的内容
                self.apply_content(&mut result, conflict, &conflict.content1)?;

                // 如果是格式冲突且有格式信息，应用格式
                if conflict.conflict_type == ConflictType::FormattingConflict {
                    if let Some(format) = &conflict.format1 {
                        self.apply_formatting(&mut result, conflict, format)?;
                    }
                }
            }
            Resolution::UseSecond => {
                // 使用文档2的内容
                self.apply_content(&mut result, conflict, &conflict.content2)?;

                // 如果是格式冲突且有格式信息，应用格式
                if conflict.conflict_type == ConflictType::FormattingConflict {
                    if let Some(format) = &conflict.format2 {
                        self.apply_formatting(&mut result, conflict, format)?;
                    }
                }
            }
            Resolution::UseBase => {
                // 使用基础版本的内容
                self.apply_content(&mut result, conflict, &conflict.base_content)?;

                // 如果是格式冲突且有格式信息，应用格式
                if conflict.conflict_type == ConflictType::FormattingConflict {
                    if let Some(format) = &conflict.base_format {
                        self.apply_formatting(&mut result, conflict, format)?;
                    }
                }
            }
            Resolution::UseCustom(custom_content) => {
                // 使用自定义内容
                self.apply_custom_content(&mut result, conflict, custom_content)?;
            }
            Resolution::MergeBoth => {
                // 尝试合并两个内容
                self.apply_merged_content(&mut result, conflict)?;

                // 如果是格式冲突且有格式信息，合并并应用格式
                if conflict.conflict_type == ConflictType::FormattingConflict {
                    if let (Some(format1), Some(format2)) = (&conflict.format1, &conflict.format2) {
                        let merged_format = self.merge_formatting(format1, format2);
                        self.apply_formatting(&mut result, conflict, &merged_format)?;
                    }
                }
            }
        }

        Ok(result)
    }

    // 全部使用 Text 或 Paragraph 替代 Formatting
    fn apply_content(
        &self,
        document: &mut Document,
        conflict: &Conflict,
        content: &ConflictContent,
    ) -> Result<()> {
        match (content, conflict.conflict_type.clone()) {
            // 格式冲突使用普通文本处理
            (ConflictContent::Text(text), ConflictType::FormattingConflict) => {
                if let Some(paragraph) = document
                    .body
                    .paragraphs
                    .get_mut(conflict.location.paragraph_index)
                {
                    // 更新或插入特定位置的文本
                    let start_pos = conflict.location.start_pos;
                    let end_pos = conflict.location.end_pos;

                    // 构建段落文本
                    let mut para_text = self.extract_paragraph_text(paragraph);

                    // 确保位置有效
                    if start_pos <= para_text.len() && end_pos <= para_text.len() {
                        // 替换文本片段
                        let prefix = &para_text[..start_pos];
                        let suffix = if end_pos < para_text.len() {
                            &para_text[end_pos..]
                        } else {
                            ""
                        };

                        para_text = format!("{}{}{}", prefix, text, suffix);

                        // 更新段落
                        paragraph.runs.clear();
                        paragraph.runs.push(Run {
                            properties: Default::default(),
                            contents: vec![RunContent::Text(para_text)],
                        });
                    }
                }
            }
            // 其他内容类型保持不变
            (ConflictContent::Paragraph(text), _) => {
                // 处理段落内容冲突
                if let Some(paragraph) = document
                    .body
                    .paragraphs
                    .get_mut(conflict.location.paragraph_index)
                {
                    if !paragraph.runs.is_empty() {
                        // 更新第一个运行的文本
                        if let Some(run) = paragraph.runs.first_mut() {
                            run.contents.clear();
                            run.contents.push(RunContent::Text(text.clone()));
                        }

                        // 移除其他运行
                        paragraph.runs.truncate(1);
                    } else {
                        // 如果需要创建新运行
                        let run = Run {
                            properties: Default::default(),
                            contents: vec![RunContent::Text(text.clone())],
                        };
                        paragraph.runs.push(run);
                    }
                } else if conflict.location.paragraph_index == document.body.paragraphs.len()
                    && !text.is_empty()
                {
                    // 在文档末尾添加新段落
                    let run = Run {
                        properties: Default::default(),
                        contents: vec![RunContent::Text(text.clone())],
                    };
                    let new_paragraph = Paragraph {
                        id: None,
                        style_id: Some("Normal".to_string()),
                        properties: Default::default(),
                        runs: vec![run],
                    };
                    document.body.paragraphs.push(new_paragraph);
                }
            }
            (ConflictContent::Text(text), _) => {
                // 处理文本片段冲突
                if let Some(paragraph) = document
                    .body
                    .paragraphs
                    .get_mut(conflict.location.paragraph_index)
                {
                    // 更新或插入特定位置的文本
                    let start_pos = conflict.location.start_pos;
                    let end_pos = conflict.location.end_pos;

                    // 构建段落文本
                    let mut para_text = self.extract_paragraph_text(paragraph);

                    // 确保位置有效
                    if start_pos <= para_text.len() && end_pos <= para_text.len() {
                        // 替换文本片段
                        let prefix = &para_text[..start_pos];
                        let suffix = if end_pos < para_text.len() {
                            &para_text[end_pos..]
                        } else {
                            ""
                        };

                        para_text = format!("{}{}{}", prefix, text, suffix);

                        // 更新段落
                        paragraph.runs.clear();
                        paragraph.runs.push(Run {
                            properties: Default::default(),
                            contents: vec![RunContent::Text(para_text)],
                        });
                    }
                }
            }
            (ConflictContent::Structure(structure_text), ConflictType::StructureConflict) => {
                // 结构冲突通常需要更复杂的处理
                // 简化处理：添加注释段落
                let comment_text = format!("[结构冲突解决: {}]", structure_text);

                // 创建注释段落
                let comment_run = Run {
                    properties: {
                        let mut props = RunProperties::default();
                        props.italic = true;
                        props.color = Some("808080".to_string()); // 灰色
                        props
                    },
                    contents: vec![RunContent::Text(comment_text)],
                };

                let comment_para = Paragraph {
                    id: None,
                    style_id: Some("Normal".to_string()),
                    properties: Default::default(),
                    runs: vec![comment_run],
                };

                // 插入到冲突位置
                if conflict.location.paragraph_index < document.body.paragraphs.len() {
                    document
                        .body
                        .paragraphs
                        .insert(conflict.location.paragraph_index, comment_para);
                } else {
                    document.body.paragraphs.push(comment_para);
                }
            }
            _ => {
                return Err(Error::MergeConflict(format!(
                    "无法解决类型为 {:?} 的冲突",
                    conflict.conflict_type
                )));
            }
        }

        Ok(())
    }

    // 修改 apply_merged_content 方法
    fn apply_merged_content(&self, document: &mut Document, conflict: &Conflict) -> Result<()> {
        // 使用模式匹配处理不同类型的冲突
        match (&conflict.content1, &conflict.content2) {
            // 段落内容合并
            (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) => {
                // 更智能的段落合并
                let merged_text = self.intelligent_merge_text(text1, text2, &conflict.base_content);
                let content = ConflictContent::Paragraph(merged_text);
                self.apply_content(document, conflict, &content)?;
            }

            // 文本片段合并
            (ConflictContent::Text(text1), ConflictContent::Text(text2)) => {
                // 文本片段合并
                let merged_text = self.intelligent_merge_text(text1, text2, &conflict.base_content);
                let content = ConflictContent::Text(merged_text);
                self.apply_content(document, conflict, &content)?;
            }

            // 结构合并（通常需要创建新结构）
            (ConflictContent::Structure(struct1), ConflictContent::Structure(struct2)) => {
                // 结构冲突较难自动合并，创建一个注释说明两种结构
                let merged_text = format!(
                    "[结构合并: 来自Doc1的\"{}\"与来自Doc2的\"{}\"]",
                    struct1, struct2
                );
                let content = ConflictContent::Structure(merged_text);
                self.apply_content(document, conflict, &content)?;
            }

            // 其他类型组合
            _ => {
                // 尝试提取文本内容进行智能合并
                let text1 = match &conflict.content1 {
                    ConflictContent::Text(t) => Some(t),
                    ConflictContent::Paragraph(t) => Some(t),
                    _ => None,
                };

                let text2 = match &conflict.content2 {
                    ConflictContent::Text(t) => Some(t),
                    ConflictContent::Paragraph(t) => Some(t),
                    _ => None,
                };

                if let (Some(t1), Some(t2)) = (text1, text2) {
                    // 可以进行文本合并
                    let merged_text = self.intelligent_merge_text(t1, t2, &conflict.base_content);

                    let content = if matches!(conflict.content1, ConflictContent::Paragraph(_)) {
                        ConflictContent::Paragraph(merged_text)
                    } else {
                        ConflictContent::Text(merged_text)
                    };

                    self.apply_content(document, conflict, &content)?;
                } else {
                    // 无法合并，使用第一个内容
                    self.apply_content(document, conflict, &conflict.content1)?;
                }
            }
        }

        Ok(())
    }

    // 新增 apply_formatting 方法
    fn apply_formatting(
        &self,
        document: &mut Document,
        conflict: &Conflict,
        format: &FormattingProperties,
    ) -> Result<()> {
        if let Some(paragraph) = document
            .body
            .paragraphs
            .get_mut(conflict.location.paragraph_index)
        {
            // 创建新的运行属性
            let mut props = RunProperties::default();
            props.bold = format.bold;
            props.italic = format.italic;
            props.underline = format.underline;
            props.size = format.font_size;
            props.font = format.font_name.clone();
            props.color = format.color.clone();

            // 创建一个新的运行
            let run = Run {
                properties: props,
                contents: vec![RunContent::Text(format.text.clone())],
            };

            // 确定段落中的位置
            let start_pos = conflict.location.start_pos;
            let end_pos = conflict.location.end_pos;

            // 更新段落
            let mut current_pos = 0;
            let mut new_runs = Vec::new();

            for old_run in &paragraph.runs {
                let run_text = self.extract_run_text(old_run);
                let run_len = run_text.len();

                if current_pos + run_len <= start_pos || current_pos >= end_pos {
                    // 这个运行在冲突区域外，保持不变
                    new_runs.push(old_run.clone());
                } else if current_pos >= start_pos && current_pos + run_len <= end_pos {
                    // 这个运行完全在冲突区域内，替换它
                    if new_runs.is_empty() || new_runs.last().unwrap().properties != run.properties
                    {
                        new_runs.push(run.clone());
                    }
                } else {
                    // 这个运行部分在冲突区域内，需要分割
                    // 简化处理：只添加新运行
                    if current_pos <= start_pos && current_pos + run_len > start_pos {
                        new_runs.push(run.clone());
                    }
                }

                current_pos += run_len;
            }

            // 更新段落中的运行
            if !new_runs.is_empty() {
                paragraph.runs = new_runs;
            }
        }

        Ok(())
    }

    // 新增 merge_formatting 方法
    fn merge_formatting(
        &self,
        format1: &FormattingProperties,
        format2: &FormattingProperties,
    ) -> FormattingProperties {
        FormattingProperties {
            text: format1.text.clone(),                         // 使用第一个格式的文本
            bold: format1.bold || format2.bold,                 // 如果任一为粗体则使用粗体
            italic: format1.italic || format2.italic,           // 如果任一为斜体则使用斜体
            underline: format1.underline || format2.underline,  // 如果任一有下划线则使用下划线
            font_size: format1.font_size.or(format2.font_size), // 优先使用第一个字体大小
            font_name: format1
                .font_name
                .clone()
                .or_else(|| format2.font_name.clone()), // 优先使用第一个字体
            color: format1.color.clone().or_else(|| format2.color.clone()), // 优先使用第一个颜色
        }
    }

    /// 应用自定义内容解决冲突
    fn apply_custom_content(
        &self,
        document: &mut Document,
        conflict: &Conflict,
        custom_content: &str,
    ) -> Result<()> {
        // 创建段落内容并使用标准方法应用
        let content = match conflict.conflict_type {
            ConflictType::FormattingConflict => {
                // 格式冲突使用普通文本，格式信息单独处理
                ConflictContent::Text(custom_content.to_string())
                // 格式信息会在apply_resolution中使用format1/format2字段处理
            }
            ConflictType::StructureConflict => {
                ConflictContent::Structure(custom_content.to_string())
            }
            _ => ConflictContent::Paragraph(custom_content.to_string()),
        };

        self.apply_content(document, conflict, &content)
    }

    /// 从格式属性字符串中提取字体大小
    fn extract_size_from_props(&self, props: &str) -> Option<u32> {
        if let Some(size_start) = props.find("Size=Some(") {
            if let Some(size_end) = props[size_start..].find(")") {
                if let Ok(size) = props[size_start + 10..size_start + size_end].parse::<u32>() {
                    return Some(size);
                }
            }
        }
        None
    }

    /// 从格式属性字符串中提取颜色
    fn extract_color_from_props(&self, props: &str) -> Option<String> {
        if let Some(color_start) = props.find("Color=Some(\"") {
            if let Some(color_end) = props[color_start..].find("\")") {
                return Some(props[color_start + 12..color_start + color_end].to_string());
            }
        }
        None
    }

    /// 从格式属性字符串中提取字体
    fn extract_font_from_props(&self, props: &str) -> Option<String> {
        if let Some(font_start) = props.find("Font=Some(\"") {
            if let Some(font_end) = props[font_start..].find("\")") {
                return Some(props[font_start + 11..font_start + font_end].to_string());
            }
        }
        None
    }

    /// 智能合并文本内容
    fn intelligent_merge_text(
        &self,
        text1: &str,
        text2: &str,
        base_content: &ConflictContent,
    ) -> String {
        // 获取基础文本
        let base_text = match base_content {
            ConflictContent::Paragraph(text) => text,
            ConflictContent::Text(text) => text,
            _ => "",
        };

        // 如果文本完全一样，直接返回
        if text1 == text2 {
            return text1.to_string();
        }

        // 使用差异比较算法找出两个文本间的差异
        let diff = TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .timeout(std::time::Duration::from_secs(2))
            .diff_chars(text1, text2);

        let mut merged = String::new();
        let mut has_conflict = false;

        // 添加所有来自两方的变更内容
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    // 两个文本中共同的部分
                    merged.push_str(change.value());
                }
                ChangeTag::Delete => {
                    // 只在text1中有的部分
                    // 避免删除基础版本中有的内容，除非text2也删除了
                    if !base_text.contains(change.value()) ||
                                       // 如果base_text包含此内容但text2明确删除了它
                                       (base_text.contains(change.value()) && !text2.contains(change.value()))
                    {
                        // 增加分隔标记，表示内容来自文档1
                        if !has_conflict {
                            merged.push_str("[合并:");
                            has_conflict = true;
                        }
                        merged.push_str(&format!(" (1)\"{}\"", change.value()));
                    }
                }
                ChangeTag::Insert => {
                    // 只在text2中有的部分
                    if !base_text.contains(change.value()) {
                        // 增加分隔标记，表示内容来自文档2
                        if !has_conflict {
                            merged.push_str("[合并:");
                            has_conflict = true;
                        }
                        merged.push_str(&format!(" (2)\"{}\"", change.value()));
                    } else {
                        // 如果是基础版本中存在的内容，则保留
                        merged.push_str(change.value());
                    }
                }
            }
        }

        if has_conflict {
            merged.push_str("]");
        }

        merged
    }

    /// 获取用户友好的冲突描述
    pub fn get_conflict_description(&self, conflict: &Conflict) -> String {
        match conflict.conflict_type {
            ConflictType::ContentConflict => {
                format!("内容冲突：两个版本以不同方式修改了同一处内容")
            }
            ConflictType::ModifyDeleteConflict => {
                format!("修改/删除冲突：一个版本修改了内容，而另一个版本删除了它")
            }
            ConflictType::InsertInsertConflict => {
                format!("插入冲突：两个版本在同一位置插入了不同内容")
            }
            ConflictType::FormattingConflict => {
                format!("格式冲突：两个版本对同一内容应用了不同的格式")
            }
            ConflictType::StructureConflict => {
                format!("结构冲突：两个版本对文档结构做了不同的修改")
            }
        }
    }

    /// 获取冲突解决选项的描述
    pub fn get_resolution_options(&self, conflict: &Conflict) -> Vec<(Resolution, String)> {
        let mut options = Vec::new();

        match conflict.conflict_type {
            ConflictType::InsertInsertConflict => {
                options.push((Resolution::UseFirst, "使用第一个文档的内容".to_string()));
                options.push((Resolution::UseSecond, "使用第二个文档的内容".to_string()));
                options.push((Resolution::MergeBoth, "合并两者的内容".to_string()));
                options.push((Resolution::UseBase, "不使用任何插入内容".to_string()));

                // 检查内容特征，提供智能选项
                if let ConflictContent::Paragraph(text1) = &conflict.content1 {
                    // 标题行处理
                    if text1.contains(". ")
                        && text1.split(". ").next().unwrap().parse::<usize>().is_ok()
                    {
                        options.push((
                            Resolution::UseCustom("重新编号".to_string()),
                            "重新编号并保留两者".to_string(),
                        ));
                    }

                    // 如果内容看起来像列表项
                    if text1.starts_with("- ")
                        || text1.starts_with("* ")
                        || (text1.starts_with("1.") && text1.contains("\n2."))
                    {
                        options.push((
                            Resolution::UseCustom("合并列表".to_string()),
                            "合并为单个列表".to_string(),
                        ));
                    }
                }
            }
            ConflictType::ModifyDeleteConflict => {
                options.push((Resolution::UseFirst, "使用第一个文档的版本".to_string()));
                options.push((Resolution::UseSecond, "使用第二个文档的版本".to_string()));
                options.push((Resolution::UseBase, "使用原始版本".to_string()));

                // 如果一方删除内容，提供额外选项
                if let (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) =
                    (&conflict.content1, &conflict.content2)
                {
                    if text1.is_empty() {
                        // 文档1删除了内容
                        options.push((
                            Resolution::UseCustom("[已删除]".to_string()),
                            "保留删除，但添加删除标记".to_string(),
                        ));
                    } else if text2.is_empty() {
                        // 文档2删除了内容
                        options.push((
                            Resolution::UseCustom("[已删除]".to_string()),
                            "保留删除，但添加删除标记".to_string(),
                        ));
                    }
                }
            }
            ConflictType::FormattingConflict => {
                options.push((Resolution::UseFirst, "使用第一个文档的格式".to_string()));
                options.push((Resolution::UseSecond, "使用第二个文档的格式".to_string()));
                options.push((Resolution::UseBase, "使用原始格式".to_string()));
                options.push((Resolution::MergeBoth, "合并两种格式".to_string()));

                // 具体格式相关选项
                if conflict.conflict_type == ConflictType::FormattingConflict {
                    if let (Some(format1), Some(format2)) = (&conflict.format1, &conflict.format2) {
                        // 粗体冲突
                        if format1.bold && !format2.bold {
                            options.push((
                                Resolution::UseCustom("应用粗体".to_string()),
                                "只应用粗体格式".to_string(),
                            ));
                        }

                        // 斜体冲突
                        if format1.italic && !format2.italic {
                            options.push((
                                Resolution::UseCustom("应用斜体".to_string()),
                                "只应用斜体格式".to_string(),
                            ));
                        }

                        // 字体颜色冲突
                        if let (Some(color1), Some(color2)) = (&format1.color, &format2.color) {
                            if color1 != color2 {
                                options.push((
                                    Resolution::UseCustom(format!("使用颜色: {}", color1)),
                                    format!("应用颜色 {} (文档1)", color1),
                                ));
                                options.push((
                                    Resolution::UseCustom(format!("使用颜色: {}", color2)),
                                    format!("应用颜色 {} (文档2)", color2),
                                ));
                            }
                        }
                    }
                }
            }
            ConflictType::ContentConflict => {
                options.push((Resolution::UseFirst, "使用第一个文档的内容".to_string()));
                options.push((Resolution::UseSecond, "使用第二个文档的内容".to_string()));
                options.push((Resolution::UseBase, "使用原始内容".to_string()));
                options.push((Resolution::MergeBoth, "尝试合并两种内容".to_string()));

                // 添加智能选项
                if let (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) =
                    (&conflict.content1, &conflict.content2)
                {
                    // 检查是否一个内容包含另一个（可能是内容添加）
                    if text1.contains(text2) {
                        options
                            .push((Resolution::UseFirst, "使用更完整的内容 (文档1)".to_string()));
                    } else if text2.contains(text1) {
                        options.push((
                            Resolution::UseSecond,
                            "使用更完整的内容 (文档2)".to_string(),
                        ));
                    }

                    // 如果相似度较高
                    let similarity = self.text_similarity(text1, text2);
                    if similarity > 0.7 {
                        options.push((
                            Resolution::UseCustom("高亮差异".to_string()),
                            "保留共同内容并高亮差异".to_string(),
                        ));
                    }
                }
            }
            ConflictType::StructureConflict => {
                options.push((Resolution::UseFirst, "使用第一个文档的结构".to_string()));
                options.push((Resolution::UseSecond, "使用第二个文档的结构".to_string()));
                options.push((Resolution::UseBase, "使用原始结构".to_string()));

                // 结构冲突通常很复杂，提供更具描述性的选项
                if let (ConflictContent::Structure(struct1), ConflictContent::Structure(struct2)) =
                    (&conflict.content1, &conflict.content2)
                {
                    if struct1.contains("表格") && struct2.contains("表格") {
                        // 表格相关冲突
                        options.push((
                            Resolution::UseCustom("合并表格".to_string()),
                            "尝试合并两个表格结构".to_string(),
                        ));
                    } else if struct1.contains("段落") && struct2.contains("段落") {
                        // 段落结构冲突
                        options.push((
                            Resolution::UseCustom("保留两者".to_string()),
                            "保留两种段落结构并添加注释".to_string(),
                        ));
                    }
                }

                // 添加自定义结构选项
                options.push((
                    Resolution::UseCustom("手动合并".to_string()),
                    "手动指定如何合并结构".to_string(),
                ));
            }
        }

        options
    }

    /// 获取冲突的详细描述和内容差异
    pub fn get_detailed_conflict_info(&self, conflict: &Conflict) -> String {
        let mut info = String::new();

        // 基本冲突信息
        info.push_str(&format!("冲突ID: {}\n", conflict.id));
        info.push_str(&format!("冲突类型: {:?}\n", conflict.conflict_type));
        info.push_str(&format!(
            "位置: 段落 {}, 字符位置 {}-{}\n\n",
            conflict.location.paragraph_index + 1,
            conflict.location.start_pos,
            conflict.location.end_pos
        ));

        // 内容差异
        match &conflict.content1 {
            ConflictContent::Paragraph(text) => {
                info.push_str(&format!("文档1内容:\n{}\n\n", text));
            }
            ConflictContent::Text(text) => {
                info.push_str(&format!("文档1文本:\n{}\n\n", text));
            }
            ConflictContent::Structure(text) => {
                info.push_str(&format!("文档1结构:\n{}\n\n", text));
            }
        }

        // 如果有格式信息，单独处理
        if let Some(format) = &conflict.format1 {
            if conflict.conflict_type == ConflictType::FormattingConflict {
                info.push_str(&format!("文档1格式:\n"));
                info.push_str(&format!(
                    "  粗体: {}\n",
                    if format.bold { "是" } else { "否" }
                ));
                info.push_str(&format!(
                    "  斜体: {}\n",
                    if format.italic { "是" } else { "否" }
                ));
                info.push_str(&format!(
                    "  下划线: {}\n",
                    if format.underline { "是" } else { "否" }
                ));
                if let Some(size) = format.font_size {
                    info.push_str(&format!("  字体大小: {}\n", size));
                }
                if let Some(font) = &format.font_name {
                    info.push_str(&format!("  字体: {}\n", font));
                }
                if let Some(color) = &format.color {
                    info.push_str(&format!("  颜色: {}\n", color));
                }
                info.push_str("\n");
            }
        }

        // 内容差异
        match &conflict.content2 {
            ConflictContent::Paragraph(text) => {
                info.push_str(&format!("文档1内容:\n{}\n\n", text));
            }
            ConflictContent::Text(text) => {
                info.push_str(&format!("文档1文本:\n{}\n\n", text));
            }
            ConflictContent::Structure(text) => {
                info.push_str(&format!("文档1结构:\n{}\n\n", text));
            }
        }

        // 如果有格式信息，单独处理
        if let Some(format) = &conflict.format2 {
            if conflict.conflict_type == ConflictType::FormattingConflict {
                info.push_str(&format!("文档1格式:\n"));
                info.push_str(&format!(
                    "  粗体: {}\n",
                    if format.bold { "是" } else { "否" }
                ));
                info.push_str(&format!(
                    "  斜体: {}\n",
                    if format.italic { "是" } else { "否" }
                ));
                info.push_str(&format!(
                    "  下划线: {}\n",
                    if format.underline { "是" } else { "否" }
                ));
                if let Some(size) = format.font_size {
                    info.push_str(&format!("  字体大小: {}\n", size));
                }
                if let Some(font) = &format.font_name {
                    info.push_str(&format!("  字体: {}\n", font));
                }
                if let Some(color) = &format.color {
                    info.push_str(&format!("  颜色: {}\n", color));
                }
                info.push_str("\n");
            }
        }

        // 内容差异
        match &conflict.base_content {
            ConflictContent::Paragraph(text) => {
                info.push_str(&format!("文档1内容:\n{}\n\n", text));
            }
            ConflictContent::Text(text) => {
                info.push_str(&format!("文档1文本:\n{}\n\n", text));
            }
            ConflictContent::Structure(text) => {
                info.push_str(&format!("文档1结构:\n{}\n\n", text));
            }
        }

        // 如果有格式信息，单独处理
        if let Some(format) = &conflict.base_format {
            if conflict.conflict_type == ConflictType::FormattingConflict {
                info.push_str(&format!("文档1格式:\n"));
                info.push_str(&format!(
                    "  粗体: {}\n",
                    if format.bold { "是" } else { "否" }
                ));
                info.push_str(&format!(
                    "  斜体: {}\n",
                    if format.italic { "是" } else { "否" }
                ));
                info.push_str(&format!(
                    "  下划线: {}\n",
                    if format.underline { "是" } else { "否" }
                ));
                if let Some(size) = format.font_size {
                    info.push_str(&format!("  字体大小: {}\n", size));
                }
                if let Some(font) = &format.font_name {
                    info.push_str(&format!("  字体: {}\n", font));
                }
                if let Some(color) = &format.color {
                    info.push_str(&format!("  颜色: {}\n", color));
                }
                info.push_str("\n");
            }
        }

        info
    }

    /// 生成用于可视化的冲突预览
    pub fn generate_conflict_preview(&self, conflict: &Conflict) -> Result<String> {
        let mut preview = String::new();

        match conflict.conflict_type {
            ConflictType::ContentConflict
            | ConflictType::ModifyDeleteConflict
            | ConflictType::InsertInsertConflict => {
                // 生成并排比较视图
                preview.push_str("<div class=\"conflict-preview\">\n");
                preview.push_str("  <div class=\"conflict-versions\">\n");

                // 文档1内容
                preview.push_str("    <div class=\"version version-1\">\n");
                preview.push_str("      <h4>文档1</h4>\n");
                if let ConflictContent::Paragraph(text) = &conflict.content1 {
                    preview.push_str(&format!(
                        "      <div class=\"content\">{}</div>\n",
                        html_escape::encode_text(text)
                    ));
                } else if let ConflictContent::Text(text) = &conflict.content1 {
                    preview.push_str(&format!(
                        "      <div class=\"content\">{}</div>\n",
                        html_escape::encode_text(text)
                    ));
                }
                preview.push_str("    </div>\n");

                // 文档2内容
                preview.push_str("    <div class=\"version version-2\">\n");
                preview.push_str("      <h4>文档2</h4>\n");
                if let ConflictContent::Paragraph(text) = &conflict.content2 {
                    preview.push_str(&format!(
                        "      <div class=\"content\">{}</div>\n",
                        html_escape::encode_text(text)
                    ));
                } else if let ConflictContent::Text(text) = &conflict.content2 {
                    preview.push_str(&format!(
                        "      <div class=\"content\">{}</div>\n",
                        html_escape::encode_text(text)
                    ));
                }
                preview.push_str("    </div>\n");

                // 原始内容
                preview.push_str("    <div class=\"version version-base\">\n");
                preview.push_str("      <h4>原始版本</h4>\n");
                if let ConflictContent::Paragraph(text) = &conflict.base_content {
                    preview.push_str(&format!(
                        "      <div class=\"content\">{}</div>\n",
                        html_escape::encode_text(text)
                    ));
                } else if let ConflictContent::Text(text) = &conflict.base_content {
                    preview.push_str(&format!(
                        "      <div class=\"content\">{}</div>\n",
                        html_escape::encode_text(text)
                    ));
                }
                preview.push_str("    </div>\n");

                preview.push_str("  </div>\n");

                // 细节比较
                if let (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) =
                    (&conflict.content1, &conflict.content2)
                {
                    // 使用差异算法生成详细比较
                    let diff = TextDiff::configure()
                        .algorithm(Algorithm::Myers)
                        .timeout(std::time::Duration::from_secs(1))
                        .diff_lines(text1, text2);

                    preview.push_str("  <div class=\"diff-details\">\n");
                    preview.push_str("    <h4>详细差异</h4>\n");
                    preview.push_str("    <pre class=\"diff\">\n");

                    for change in diff.iter_all_changes() {
                        match change.tag() {
                            ChangeTag::Equal => {
                                preview.push_str(&format!(
                                    "      {}\n",
                                    html_escape::encode_text(change.value())
                                ));
                            }
                            ChangeTag::Delete => {
                                preview.push_str(&format!(
                                    "      <span class=\"deletion\">- {}</span>\n",
                                    html_escape::encode_text(change.value())
                                ));
                            }
                            ChangeTag::Insert => {
                                preview.push_str(&format!(
                                    "      <span class=\"addition\">+ {}</span>\n",
                                    html_escape::encode_text(change.value())
                                ));
                            }
                        }
                    }

                    preview.push_str("    </pre>\n");
                    preview.push_str("  </div>\n");
                }

                preview.push_str("</div>\n");
            }

            ConflictType::FormattingConflict => {
                // 格式冲突预览
                preview.push_str("<div class=\"conflict-preview formatting\">\n");
                preview.push_str("  <h4>格式冲突</h4>\n");

                if let (Some(format1), Some(format2)) = (&conflict.format1, &conflict.format2) {
                    preview.push_str("  <div class=\"formatting-examples\">\n");
                    preview.push_str(&format!("    <div class=\"example\"><span class=\"label\">文档1:</span> <span class=\"formatted format-1\">{}</span></div>\n",
                                    html_escape::encode_text(&format1.text)));
                    preview.push_str(&format!("    <div class=\"example\"><span class=\"label\">文档2:</span> <span class=\"formatted format-2\">{}</span></div>\n",
                                    html_escape::encode_text(&format2.text)));
                    preview.push_str("  </div>\n");

                    // 显示格式属性差异
                    if let (Some(format1), Some(format2)) = (&conflict.format1, &conflict.format2) {
                        preview.push_str("  <div class=\"format-details\">\n");
                        preview.push_str("    <h5>格式属性:</h5>\n");
                        preview.push_str("    <table>\n");
                        preview
                            .push_str("      <tr><th>属性</th><th>文档1</th><th>文档2</th></tr>\n");

                        // 粗体
                        preview.push_str("      <tr>\n");
                        preview.push_str("        <td>粗体</td>\n");
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            if format1.bold { "是" } else { "否" }
                        ));
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            if format2.bold { "是" } else { "否" }
                        ));
                        preview.push_str("      </tr>\n");

                        // 斜体
                        preview.push_str("      <tr>\n");
                        preview.push_str("        <td>斜体</td>\n");
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            if format1.italic { "是" } else { "否" }
                        ));
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            if format2.italic { "是" } else { "否" }
                        ));
                        preview.push_str("      </tr>\n");

                        // 下划线
                        preview.push_str("      <tr>\n");
                        preview.push_str("        <td>下划线</td>\n");
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            if format1.underline { "是" } else { "否" }
                        ));
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            if format2.underline { "是" } else { "否" }
                        ));
                        preview.push_str("      </tr>\n");

                        // 字体大小
                        preview.push_str("      <tr>\n");
                        preview.push_str("        <td>字体大小</td>\n");
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            format1
                                .font_size
                                .map_or("默认".to_string(), |s| s.to_string())
                        ));
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            format2
                                .font_size
                                .map_or("默认".to_string(), |s| s.to_string())
                        ));
                        preview.push_str("      </tr>\n");

                        // 颜色
                        preview.push_str("      <tr>\n");
                        preview.push_str("        <td>颜色</td>\n");
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            format1.color.clone().unwrap_or_else(|| "默认".to_string())
                        ));
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            format2.color.clone().unwrap_or_else(|| "默认".to_string())
                        ));
                        preview.push_str("      </tr>\n");

                        // 字体
                        preview.push_str("      <tr>\n");
                        preview.push_str("        <td>字体</td>\n");
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            format1
                                .font_name
                                .clone()
                                .unwrap_or_else(|| "默认".to_string())
                        ));
                        preview.push_str(&format!(
                            "        <td>{}</td>\n",
                            format2
                                .font_name
                                .clone()
                                .unwrap_or_else(|| "默认".to_string())
                        ));
                        preview.push_str("      </tr>\n");

                        preview.push_str("    </table>\n");
                        preview.push_str("  </div>\n");
                    }
                }

                preview.push_str("</div>\n");
            }

            ConflictType::StructureConflict => {
                // 结构冲突预览
                preview.push_str("<div class=\"conflict-preview structure\">\n");
                preview.push_str("  <h4>结构冲突</h4>\n");

                if let (ConflictContent::Structure(struct1), ConflictContent::Structure(struct2)) =
                    (&conflict.content1, &conflict.content2)
                {
                    preview.push_str("  <div class=\"structure-comparison\">\n");
                    preview.push_str(&format!("    <div class=\"structure\"><span class=\"label\">文档1结构:</span> {}</div>\n", html_escape::encode_text(struct1)));
                    preview.push_str(&format!("    <div class=\"structure\"><span class=\"label\">文档2结构:</span> {}</div>\n", html_escape::encode_text(struct2)));
                    preview.push_str("  </div>\n");

                    // 如果是表格结构冲突，添加可视化表示
                    if struct1.contains("表格") || struct2.contains("表格") {
                        preview.push_str("  <div class=\"table-structure-visualization\">\n");
                        preview.push_str("    <h5>表格结构可视化</h5>\n");
                        // 这里可以添加表格结构的可视化表示
                        preview.push_str("    <div class=\"visualization-placeholder\">表格结构可视化将在合并编辑器中提供</div>\n");
                        preview.push_str("  </div>\n");
                    }
                }

                preview.push_str("</div>\n");
            }
        }

        Ok(preview)
    }
}

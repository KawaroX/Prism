//! Strategies for merging docx documents.

use crate::DocxBuilder;
use crate::DocxParser;
use crate::diff::{DiffOperation, DiffOptions, DocumentDiffer, DocxDiffer}; // 添加 DocumentDiffer
use crate::docx::model::{Document, Paragraph, Run, RunContent, RunProperties};
use crate::error::Result;
use crate::merge::InsertConflictHandling::{KeepBoth, KeepFirst, KeepSecond, SkipBoth};
use crate::merge::conflict::ConflictResolver;
use crate::merge::{
    Conflict, ConflictContent, ConflictType, DocumentMerger, MergeOptions, MergeResult, Resolution,
};

/// A three-way merge strategy for docx documents
pub struct ThreeWayMerger {
    /// The differ used to compare documents
    differ: DocxDiffer,
    /// The conflict resolver
    conflict_resolver: ConflictResolver,
}

impl ThreeWayMerger {
    /// Create a new three-way merger
    pub fn new() -> Self {
        ThreeWayMerger {
            differ: DocxDiffer::new(),
            conflict_resolver: ConflictResolver::new(),
        }
    }

    /// Helper to extract all paragraph text differences
    fn extract_paragraph_diffs(
        &self,
        base: &Document,
        doc: &Document,
    ) -> Result<Vec<(usize, String, String)>> {
        let mut results = Vec::new();
        let options = DiffOptions::default();

        let diff = self.differ.diff(base, doc, &options)?;

        for op in diff.operations {
            match op {
                DiffOperation::Addition { location, content } => {
                    if let crate::diff::DiffContent::Text(text) = content {
                        results.push((location.paragraph_index, "".to_string(), text));
                    }
                }
                DiffOperation::Deletion { location, content } => {
                    if let crate::diff::DiffContent::Text(text) = content {
                        results.push((location.paragraph_index, text, "".to_string()));
                    }
                }
                DiffOperation::Modification {
                    location,
                    old_content,
                    new_content,
                } => {
                    if let (
                        crate::diff::DiffContent::Text(old_text),
                        crate::diff::DiffContent::Text(new_text),
                    ) = (old_content, new_content)
                    {
                        results.push((location.paragraph_index, old_text, new_text));
                    }
                }
                DiffOperation::Equal { .. } => {
                    // Ignore equal parts
                }
            }
        }

        Ok(results)
    }

    /// Apply changes from a diff to a document
    fn apply_changes(
        &self,
        base: &Document,
        changes: &[(usize, String, String)],
    ) -> Result<Document> {
        // Create a copy of the base document
        let mut result = base.clone();

        for (para_idx, _old_text, new_text) in changes {
            if let Some(paragraph) = result.body.paragraphs.get_mut(*para_idx) {
                // For simplicity, we just replace the entire paragraph text
                // In a more advanced implementation, we would update only the changed parts

                if new_text.is_empty() {
                    // This is a deletion
                    // We could remove the paragraph entirely, but for now we'll just clear it
                    paragraph.runs.clear();
                } else {
                    // This is an addition or modification
                    if !paragraph.runs.is_empty() {
                        // Replace text in the first run
                        if let Some(run) = paragraph.runs.first_mut() {
                            run.contents.clear();
                            run.contents.push(RunContent::Text(new_text.clone()));
                        }

                        // Remove other runs
                        paragraph.runs.truncate(1);
                    } else {
                        // Create a new run if needed
                        let run = Run {
                            properties: Default::default(),
                            contents: vec![RunContent::Text(new_text.clone())],
                        };
                        paragraph.runs.push(run);
                    }
                }
            } else if *para_idx >= result.body.paragraphs.len() && !new_text.is_empty() {
                // This is an addition at the end
                let run = Run {
                    properties: Default::default(),
                    contents: vec![RunContent::Text(new_text.clone())],
                };

                let paragraph = Paragraph {
                    id: None,
                    style_id: None,
                    properties: Default::default(),
                    runs: vec![run],
                };

                result.body.paragraphs.push(paragraph);
            }
        }

        Ok(result)
    }

    /// 合并文档，返回结果和冲突信息
    pub fn merge_documents(
        &self,
        base_path: &str,
        doc1_path: &str,
        doc2_path: &str,
        options: Option<MergeOptions>,
        output_path: Option<&str>,
    ) -> Result<(Document, Vec<Conflict>)> {
        // 解析文档
        let base_doc = DocxParser::parse_file(base_path)?;
        let doc1 = DocxParser::parse_file(doc1_path)?;
        let doc2 = DocxParser::parse_file(doc2_path)?;

        // 使用提供的选项或默认选项
        let options = options.unwrap_or_default();

        // 执行合并
        let merge_result = self.merge(&base_doc, &doc1, &doc2, &options)?;

        if let Some(path) = output_path {
            self.save_merged_document(&merge_result.document, path)?;
        }

        // 返回合并结果和冲突列表
        return Ok((merge_result.document, merge_result.conflicts));
    }

    /// 保存合并结果
    pub fn save_merged_document(&self, document: &Document, output_path: &str) -> Result<()> {
        DocxBuilder::build(document, output_path)
    }

    /// 解决特定冲突
    pub fn resolve_specific_conflict(
        &self,
        document: &Document,
        conflict: &Conflict,
        resolution: Resolution,
    ) -> Result<Document> {
        self.conflict_resolver
            .apply_resolution(document, conflict, &resolution)
    }

    /// 解决所有冲突并保存结果
    pub fn resolve_all_conflicts(
        &self,
        document: &Document,
        conflicts: &[Conflict],
        resolutions: &[Resolution],
        output_path: &str,
    ) -> Result<()> {
        let mut doc = document.clone();

        for (conflict, resolution) in conflicts.iter().zip(resolutions.iter()) {
            doc = self
                .conflict_resolver
                .apply_resolution(&doc, conflict, resolution)?;
        }

        DocxBuilder::build(&doc, output_path)
    }

    /// 获取冲突摘要信息
    pub fn get_conflict_summary(&self, merge_result: &MergeResult) -> Vec<String> {
        let mut summaries = Vec::new();

        for conflict in &merge_result.conflicts {
            let summary = match conflict.conflict_type {
                ConflictType::ContentConflict => format!(
                    "内容冲突：段落 {} 中的内容被不同方式修改",
                    conflict.location.paragraph_index + 1
                ),
                ConflictType::ModifyDeleteConflict => format!(
                    "修改/删除冲突：段落 {} 被一方修改，被另一方删除",
                    conflict.location.paragraph_index + 1
                ),
                ConflictType::InsertInsertConflict => {
                    let mut summary = format!(
                        "插入冲突：段落 {} 处有不同的插入内容",
                        conflict.location.paragraph_index + 1
                    );

                    if let ConflictContent::Paragraph(text1) = &conflict.content1 {
                        summary.push_str(&format!("\n文档1: {}", text1));
                    }

                    if let ConflictContent::Paragraph(text2) = &conflict.content2 {
                        summary.push_str(&format!("\n文档2: {}", text2));
                    }

                    summary
                }
                ConflictType::FormattingConflict => format!(
                    "格式冲突：段落 {} 的格式被不同方式修改",
                    conflict.location.paragraph_index + 1
                ),
                ConflictType::StructureConflict => format!(
                    "结构冲突：段落 {} 的结构被不同方式修改",
                    conflict.location.paragraph_index + 1
                ),
            };

            summaries.push(summary);
        }

        summaries
    }
}

impl DocumentMerger for ThreeWayMerger {
    fn merge(
        &self,
        base: &Document,
        doc1: &Document,
        doc2: &Document,
        options: &MergeOptions,
    ) -> Result<MergeResult> {
        // 1. Detect differences between base and each document
        let base_to_doc1 = self.extract_paragraph_diffs(base, doc1)?;
        let base_to_doc2 = self.extract_paragraph_diffs(base, doc2)?;

        // 2. Detect conflicts
        let conflicts =
            self.conflict_resolver
                .detect_conflicts(base, doc1, doc2, options.auto_resolve);
        let has_conflicts = !conflicts.is_empty();

        // 3. Start with base document
        let mut merged = base.clone();

        // 4. Apply non-conflicting changes from doc1
        for (para_idx, old_text, new_text) in &base_to_doc1 {
            // Skip if this paragraph is in a conflict
            if conflicts
                .iter()
                .any(|c| c.location.paragraph_index == *para_idx)
            {
                continue;
            }

            if let Some(paragraph) = merged.body.paragraphs.get_mut(*para_idx) {
                // Apply the change if it's not in conflict
                if !paragraph.runs.is_empty() {
                    // Replace text in the first run
                    if let Some(run) = paragraph.runs.first_mut() {
                        run.contents.clear();
                        run.contents.push(RunContent::Text(new_text.clone()));
                    }

                    // Remove other runs
                    paragraph.runs.truncate(1);
                } else if !new_text.is_empty() {
                    // Create a new run if needed
                    let run = Run {
                        properties: Default::default(),
                        contents: vec![RunContent::Text(new_text.clone())],
                    };
                    paragraph.runs.push(run);
                }
            } else if *para_idx >= merged.body.paragraphs.len() && !new_text.is_empty() {
                // This is an addition at the end
                let run = Run {
                    properties: Default::default(),
                    contents: vec![RunContent::Text(new_text.clone())],
                };

                let paragraph = Paragraph {
                    id: None,
                    style_id: None,
                    properties: Default::default(),
                    runs: vec![run],
                };

                merged.body.paragraphs.push(paragraph);
            }
        }

        // 5. Apply non-conflicting changes from doc2
        for (para_idx, old_text, new_text) in &base_to_doc2 {
            // Skip if this paragraph is in a conflict
            if conflicts
                .iter()
                .any(|c| c.location.paragraph_index == *para_idx)
            {
                continue;
            }

            if let Some(paragraph) = merged.body.paragraphs.get_mut(*para_idx) {
                // Apply the change if it's not in conflict
                if !paragraph.runs.is_empty() {
                    // Replace text in the first run
                    if let Some(run) = paragraph.runs.first_mut() {
                        run.contents.clear();
                        run.contents.push(RunContent::Text(new_text.clone()));
                    }

                    // Remove other runs
                    paragraph.runs.truncate(1);
                } else if !new_text.is_empty() {
                    // Create a new run if needed
                    let run = Run {
                        properties: Default::default(),
                        contents: vec![RunContent::Text(new_text.clone())],
                    };
                    paragraph.runs.push(run);
                }
            } else if *para_idx >= merged.body.paragraphs.len() && !new_text.is_empty() {
                // This is an addition at the end
                let run = Run {
                    properties: Default::default(),
                    contents: vec![RunContent::Text(new_text.clone())],
                };

                let paragraph = Paragraph {
                    id: None,
                    style_id: None,
                    properties: Default::default(),
                    runs: vec![run],
                };

                merged.body.paragraphs.push(paragraph);
            }
        }

        // 6. Apply resolution for conflicts if auto-resolve is enabled
        if options.auto_resolve {
            for conflict in &conflicts {
                match conflict.conflict_type {
                    ConflictType::InsertInsertConflict => {
                        // 处理插入冲突
                        match options.insert_conflict_handling {
                            KeepBoth => {
                                // 找到冲突位置
                                let index = conflict.location.paragraph_index;

                                // 先确保索引有效
                                if index <= merged.body.paragraphs.len() {
                                    // 处理第一个文档的内容
                                    if let ConflictContent::Paragraph(text1) = &conflict.content1 {
                                        let para1 = Paragraph {
                                            id: None,
                                            style_id: Some(
                                                if text1.trim().starts_with(|c: char| {
                                                    c.is_digit(10) || c == '.'
                                                }) {
                                                    "Heading2".to_string()
                                                } else {
                                                    "Normal".to_string()
                                                },
                                            ),
                                            properties: Default::default(),
                                            runs: vec![Run {
                                                properties: {
                                                    let mut props = RunProperties::default();
                                                    if text1.trim().starts_with(|c: char| {
                                                        c.is_digit(10) || c == '.'
                                                    }) {
                                                        props.bold = true;
                                                    }
                                                    props
                                                },
                                                contents: vec![RunContent::Text(format!(
                                                    "[文档1] {}",
                                                    text1
                                                ))],
                                            }],
                                        };

                                        // 插入第一个文档的段落
                                        if index == merged.body.paragraphs.len() {
                                            merged.body.paragraphs.push(para1);
                                        } else {
                                            merged.body.paragraphs.insert(index, para1);
                                        }
                                    }

                                    // 处理第二个文档的内容
                                    if let ConflictContent::Paragraph(text2) = &conflict.content2 {
                                        let para2 = Paragraph {
                                            id: None,
                                            style_id: Some(
                                                if text2.trim().starts_with(|c: char| {
                                                    c.is_digit(10) || c == '.'
                                                }) {
                                                    "Heading2".to_string()
                                                } else {
                                                    "Normal".to_string()
                                                },
                                            ),
                                            properties: Default::default(),
                                            runs: vec![Run {
                                                properties: {
                                                    let mut props = RunProperties::default();
                                                    if text2.trim().starts_with(|c: char| {
                                                        c.is_digit(10) || c == '.'
                                                    }) {
                                                        props.bold = true;
                                                    }
                                                    props
                                                },
                                                contents: vec![RunContent::Text(format!(
                                                    "[文档2] {}",
                                                    text2
                                                ))],
                                            }],
                                        };

                                        // 在第一个文档的内容后面插入第二个文档的段落
                                        merged.body.paragraphs.insert(index + 1, para2);
                                    }
                                }
                            }
                            KeepFirst => {
                                // 只保留第一个文档的内容
                                if let ConflictContent::Paragraph(text1) = &conflict.content1 {
                                    let para = Paragraph {
                                        id: None,
                                        style_id: Some(
                                            if text1
                                                .trim()
                                                .starts_with(|c: char| c.is_digit(10) || c == '.')
                                            {
                                                "Heading2".to_string()
                                            } else {
                                                "Normal".to_string()
                                            },
                                        ),
                                        properties: Default::default(),
                                        runs: vec![Run {
                                            properties: {
                                                let mut props = RunProperties::default();
                                                if text1.trim().starts_with(|c: char| {
                                                    c.is_digit(10) || c == '.'
                                                }) {
                                                    props.bold = true;
                                                }
                                                props
                                            },
                                            contents: vec![RunContent::Text(text1.clone())],
                                        }],
                                    };

                                    // 插入段落
                                    let index = conflict.location.paragraph_index;
                                    if index == merged.body.paragraphs.len() {
                                        merged.body.paragraphs.push(para);
                                    } else if index < merged.body.paragraphs.len() {
                                        merged.body.paragraphs.insert(index, para);
                                    }
                                }
                            }
                            KeepSecond => {
                                // 只保留第二个文档的内容
                                if let ConflictContent::Paragraph(text2) = &conflict.content2 {
                                    let para = Paragraph {
                                        id: None,
                                        style_id: Some(
                                            if text2
                                                .trim()
                                                .starts_with(|c: char| c.is_digit(10) || c == '.')
                                            {
                                                "Heading2".to_string()
                                            } else {
                                                "Normal".to_string()
                                            },
                                        ),
                                        properties: Default::default(),
                                        runs: vec![Run {
                                            properties: {
                                                let mut props = RunProperties::default();
                                                if text2.trim().starts_with(|c: char| {
                                                    c.is_digit(10) || c == '.'
                                                }) {
                                                    props.bold = true;
                                                }
                                                props
                                            },
                                            contents: vec![RunContent::Text(text2.clone())],
                                        }],
                                    };

                                    // 插入段落
                                    let index = conflict.location.paragraph_index;
                                    if index == merged.body.paragraphs.len() {
                                        merged.body.paragraphs.push(para);
                                    } else if index < merged.body.paragraphs.len() {
                                        merged.body.paragraphs.insert(index, para);
                                    }
                                }
                            }
                            SkipBoth => {
                                // 不插入任何内容，保持原样
                            }
                        }
                    }
                    ConflictType::ContentConflict => {
                        // 处理内容冲突
                        let resolution = if options.favor_base {
                            Resolution::UseBase
                        } else {
                            // 默认使用第二个文档的内容
                            Resolution::UseSecond
                        };

                        merged = self.conflict_resolver.apply_resolution(
                            &merged,
                            conflict,
                            &resolution,
                        )?;

                        // 如果需要在文档中标记冲突
                        if options.track_revisions {
                            // 在冲突位置添加冲突标记
                            if let Some(paragraph) = merged
                                .body
                                .paragraphs
                                .get_mut(conflict.location.paragraph_index)
                            {
                                if !paragraph.runs.is_empty() {
                                    if let Some(run) = paragraph.runs.first_mut() {
                                        for content in &mut run.contents {
                                            if let RunContent::Text(text) = content {
                                                *text = format!("[冲突: {}] {}", conflict.id, text);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ConflictType::ModifyDeleteConflict => {
                        // 处理修改/删除冲突
                        let resolution = if options.favor_base {
                            Resolution::UseBase
                        } else {
                            // 默认使用修改而不是删除
                            if let ConflictContent::Paragraph(text) = &conflict.content1 {
                                if text.is_empty() {
                                    Resolution::UseSecond
                                } else {
                                    Resolution::UseFirst
                                }
                            } else {
                                Resolution::UseSecond
                            }
                        };

                        merged = self.conflict_resolver.apply_resolution(
                            &merged,
                            conflict,
                            &resolution,
                        )?;

                        // 如果需要在文档中标记冲突
                        if options.track_revisions {
                            // 在冲突位置添加冲突标记
                            if let Some(paragraph) = merged
                                .body
                                .paragraphs
                                .get_mut(conflict.location.paragraph_index)
                            {
                                if !paragraph.runs.is_empty() {
                                    if let Some(run) = paragraph.runs.first_mut() {
                                        for content in &mut run.contents {
                                            if let RunContent::Text(text) = content {
                                                *text = format!("[冲突: {}] {}", conflict.id, text);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ConflictType::FormattingConflict | ConflictType::StructureConflict => {
                        // 处理格式和结构冲突
                        let resolution = if options.favor_base {
                            Resolution::UseBase
                        } else {
                            // 默认使用第二个文档的格式
                            Resolution::UseSecond
                        };

                        merged = self.conflict_resolver.apply_resolution(
                            &merged,
                            conflict,
                            &resolution,
                        )?;

                        // 如果需要在文档中标记冲突
                        if options.track_revisions {
                            // 在冲突位置添加冲突标记
                            if let Some(paragraph) = merged
                                .body
                                .paragraphs
                                .get_mut(conflict.location.paragraph_index)
                            {
                                if !paragraph.runs.is_empty() {
                                    if let Some(run) = paragraph.runs.first_mut() {
                                        for content in &mut run.contents {
                                            if let RunContent::Text(text) = content {
                                                *text =
                                                    format!("[格式冲突: {}] {}", conflict.id, text);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if options.track_revisions {
            // 如果不自动解决冲突但需要标记
            for conflict in &conflicts {
                // 在冲突位置添加冲突标记
                if let Some(paragraph) = merged
                    .body
                    .paragraphs
                    .get_mut(conflict.location.paragraph_index)
                {
                    if !paragraph.runs.is_empty() {
                        if let Some(run) = paragraph.runs.first_mut() {
                            for content in &mut run.contents {
                                if let RunContent::Text(text) = content {
                                    *text = format!("[未解决冲突: {}] {}", conflict.id, text);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 7. Mark conflicts in the document if track_revisions is enabled
        if options.preserve_conflicts && !conflicts.is_empty() {
            // 在文档末尾添加冲突信息部分
            let conflict_heading = Paragraph {
                id: None,
                style_id: Some("Heading1".to_string()),
                properties: Default::default(),
                runs: vec![Run {
                    properties: {
                        let mut props = RunProperties::default();
                        props.bold = true;
                        props
                    },
                    contents: vec![RunContent::Text("文档合并冲突".to_string())],
                }],
            };
            merged.body.paragraphs.push(conflict_heading);

            // 添加每个冲突的信息
            for (i, conflict) in conflicts.iter().enumerate() {
                let conflict_info = Paragraph {
                    id: None,
                    style_id: Some("Heading2".to_string()),
                    properties: Default::default(),
                    runs: vec![Run {
                        properties: {
                            let mut props = RunProperties::default();
                            props.bold = true;
                            props
                        },
                        contents: vec![RunContent::Text(format!(
                            "冲突 #{} (ID: {})",
                            i + 1,
                            conflict.id
                        ))],
                    }],
                };
                merged.body.paragraphs.push(conflict_info);

                // 添加冲突类型
                let conflict_type = Paragraph {
                    id: None,
                    style_id: Some("Normal".to_string()),
                    properties: Default::default(),
                    runs: vec![Run {
                        properties: Default::default(),
                        contents: vec![RunContent::Text(format!(
                            "冲突类型: {:?}",
                            conflict.conflict_type
                        ))],
                    }],
                };
                merged.body.paragraphs.push(conflict_type);

                // 添加冲突内容
                if let ConflictContent::Paragraph(text1) = &conflict.content1 {
                    let content1 = Paragraph {
                        id: None,
                        style_id: Some("Normal".to_string()),
                        properties: Default::default(),
                        runs: vec![Run {
                            properties: Default::default(),
                            contents: vec![RunContent::Text(format!("文档1: {}", text1))],
                        }],
                    };
                    merged.body.paragraphs.push(content1);
                }

                if let ConflictContent::Paragraph(text2) = &conflict.content2 {
                    let content2 = Paragraph {
                        id: None,
                        style_id: Some("Normal".to_string()),
                        properties: Default::default(),
                        runs: vec![Run {
                            properties: Default::default(),
                            contents: vec![RunContent::Text(format!("文档2: {}", text2))],
                        }],
                    };
                    merged.body.paragraphs.push(content2);
                }

                // 添加分隔符
                let separator = Paragraph {
                    id: None,
                    style_id: Some("Normal".to_string()),
                    properties: Default::default(),
                    runs: vec![Run {
                        properties: Default::default(),
                        contents: vec![RunContent::Text(
                            "------------------------------".to_string(),
                        )],
                    }],
                };
                merged.body.paragraphs.push(separator);
            }
        }

        Ok(MergeResult {
            document: merged,
            has_conflicts,
            conflicts,
        })
    }

    fn resolve_conflict(&self, conflict: &Conflict, resolution: Resolution) -> Result<Document> {
        // Create a simple document with just the conflicting paragraph
        let mut doc = Document::new();
        let paragraph = Paragraph {
            id: None,
            style_id: None,
            properties: Default::default(),
            runs: vec![Run {
                properties: Default::default(),
                contents: vec![RunContent::Text("Placeholder text".to_string())],
            }],
        };
        doc.body.paragraphs.push(paragraph);

        // Apply the resolution
        self.conflict_resolver
            .apply_resolution(&doc, conflict, &resolution)
    }
}

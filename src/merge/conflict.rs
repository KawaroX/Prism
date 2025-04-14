//! Conflict detection and resolution for merging docx documents.

use uuid::Uuid;

use crate::docx::model::Document;
use crate::error::Result;
use crate::merge::{Conflict, ConflictContent, ConflictLocation, ConflictType, Resolution};

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
                // Both docs modified the same paragraph
                (Some(base_p), Some(doc1_p), Some(doc2_p)) => {
                    // Extract text for comparison
                    let base_text = self.extract_paragraph_text(base_p);
                    let doc1_text = self.extract_paragraph_text(doc1_p);
                    let doc2_text = self.extract_paragraph_text(doc2_p);

                    // Check if both changed from base and differently from each other
                    if doc1_text != base_text && doc2_text != base_text && doc1_text != doc2_text {
                        conflicts.push(Conflict {
                            id: Uuid::new_v4().to_string(),
                            location: ConflictLocation {
                                paragraph_index: i,
                                start_pos: 0,
                                end_pos: base_text.len(),
                            },
                            content1: ConflictContent::Paragraph(doc1_text.clone()),
                            content2: ConflictContent::Paragraph(doc2_text.clone()),
                            base_content: ConflictContent::Paragraph(base_text),
                            conflict_type: ConflictType::ContentConflict,
                        });
                    }
                }
                // Doc1 deleted a paragraph that doc2 modified
                (Some(base_p), None, Some(doc2_p)) => {
                    conflicts.push(Conflict {
                        id: Uuid::new_v4().to_string(),
                        location: ConflictLocation {
                            paragraph_index: i,
                            start_pos: 0,
                            end_pos: self.extract_paragraph_text(base_p).len(),
                        },
                        content1: ConflictContent::Paragraph("".to_string()), // Deleted
                        content2: ConflictContent::Paragraph(self.extract_paragraph_text(doc2_p)),
                        base_content: ConflictContent::Paragraph(
                            self.extract_paragraph_text(base_p),
                        ),
                        conflict_type: ConflictType::ModifyDeleteConflict,
                    });
                }
                // Doc2 deleted a paragraph that doc1 modified
                (Some(base_p), Some(doc1_p), None) => {
                    conflicts.push(Conflict {
                        id: Uuid::new_v4().to_string(),
                        location: ConflictLocation {
                            paragraph_index: i,
                            start_pos: 0,
                            end_pos: self.extract_paragraph_text(base_p).len(),
                        },
                        content1: ConflictContent::Paragraph(self.extract_paragraph_text(doc1_p)),
                        content2: ConflictContent::Paragraph("".to_string()), // Deleted
                        base_content: ConflictContent::Paragraph(
                            self.extract_paragraph_text(base_p),
                        ),
                        conflict_type: ConflictType::ModifyDeleteConflict,
                    });
                }
                // Both added a new paragraph at the same position
                (None, Some(doc1_p), Some(doc2_p)) => {
                    let doc1_text = self.extract_paragraph_text(doc1_p);
                    let doc2_text = self.extract_paragraph_text(doc2_p);

                    // Only conflict if they added different content
                    if doc1_text != doc2_text {
                        conflicts.push(Conflict {
                            id: Uuid::new_v4().to_string(),
                            location: ConflictLocation {
                                paragraph_index: i,
                                start_pos: 0,
                                end_pos: 0, // New paragraph
                            },
                            content1: ConflictContent::Paragraph(doc1_text),
                            content2: ConflictContent::Paragraph(doc2_text),
                            base_content: ConflictContent::Paragraph("".to_string()),
                            conflict_type: ConflictType::InsertInsertConflict,
                        });
                    }
                }
                // Other cases (e.g., only one doc added/removed a paragraph)
                // don't generate conflicts and will be handled by the merger
                _ => {}
            }
        }

        // If auto-resolution is enabled, try to resolve simple conflicts
        if auto_resolve {
            conflicts = self.auto_resolve_conflicts(conflicts);
        }

        conflicts
    }

    /// Try to automatically resolve simple conflicts
    fn auto_resolve_conflicts(&self, conflicts: Vec<Conflict>) -> Vec<Conflict> {
        // For now, we just keep all conflicts
        // In a more advanced implementation, we would try to resolve
        // simple conflicts automatically
        conflicts
    }

    /// Extract text from a paragraph
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

    /// Apply a resolution to a conflict
    pub fn apply_resolution(
        &self,
        document: &Document,
        conflict: &Conflict,
        resolution: &Resolution,
    ) -> Result<Document> {
        // Create a copy of the document
        let mut result = document.clone();

        // Apply the resolution based on the conflict type and resolution choice
        match resolution {
            Resolution::UseFirst => {
                // Use content from doc1
                self.apply_content(&mut result, conflict, &conflict.content1)?;
            }
            Resolution::UseSecond => {
                // Use content from doc2
                self.apply_content(&mut result, conflict, &conflict.content2)?;
            }
            Resolution::UseBase => {
                // Use content from base
                self.apply_content(&mut result, conflict, &conflict.base_content)?;
            }
            Resolution::UseCustom(custom_content) => {
                // Use custom content
                self.apply_custom_content(&mut result, conflict, custom_content)?;
            }
            Resolution::MergeBoth => {
                // Try to merge both contents (for now, just concatenate)
                self.apply_merged_content(&mut result, conflict)?;
            }
        }

        Ok(result)
    }

    /// Apply content to resolve a conflict
    fn apply_content(
        &self,
        document: &mut Document,
        conflict: &Conflict,
        content: &ConflictContent,
    ) -> Result<()> {
        match content {
            ConflictContent::Paragraph(text) => {
                // For simplicity, we just replace the entire paragraph text
                if let Some(paragraph) = document
                    .body
                    .paragraphs
                    .get_mut(conflict.location.paragraph_index)
                {
                    if !paragraph.runs.is_empty() {
                        // Replace text in the first run
                        if let Some(run) = paragraph.runs.first_mut() {
                            run.contents.clear();
                            run.contents
                                .push(crate::docx::model::RunContent::Text(text.clone()));
                        }

                        // Remove other runs
                        paragraph.runs.truncate(1);
                    } else {
                        // Create a new run if needed
                        let run = crate::docx::model::Run {
                            properties: Default::default(),
                            contents: vec![crate::docx::model::RunContent::Text(text.clone())],
                        };
                        paragraph.runs.push(run);
                    }
                }
            }
            ConflictContent::Text(text) => {
                // For text fragments within paragraphs
                if let Some(paragraph) = document
                    .body
                    .paragraphs
                    .get_mut(conflict.location.paragraph_index)
                {
                    if !paragraph.runs.is_empty() {
                        // For simplicity, we'll just update the first run
                        if let Some(run) = paragraph.runs.first_mut() {
                            run.contents.clear();
                            run.contents
                                .push(crate::docx::model::RunContent::Text(text.clone()));
                        }
                    }
                }
            }
            ConflictContent::Structure(_) => {
                // Structure conflicts would require more complex handling
                // For now, we'll just leave it as is
            }
        }

        Ok(())
    }

    /// Apply custom content to resolve a conflict
    fn apply_custom_content(
        &self,
        document: &mut Document,
        conflict: &Conflict,
        custom_content: &str,
    ) -> Result<()> {
        // Create a paragraph content and use the standard apply method
        let content = ConflictContent::Paragraph(custom_content.to_string());
        self.apply_content(document, conflict, &content)
    }

    /// Merge both contents (simple concatenation for now)
    fn apply_merged_content(&self, document: &mut Document, conflict: &Conflict) -> Result<()> {
        match (&conflict.content1, &conflict.content2) {
            (ConflictContent::Paragraph(text1), ConflictContent::Paragraph(text2)) => {
                let merged = format!("{} [MERGED] {}", text1, text2);
                let content = ConflictContent::Paragraph(merged);
                self.apply_content(document, conflict, &content)?;
            }
            (ConflictContent::Text(text1), ConflictContent::Text(text2)) => {
                let merged = format!("{} [MERGED] {}", text1, text2);
                let content = ConflictContent::Text(merged);
                self.apply_content(document, conflict, &content)?;
            }
            _ => {
                // For other combinations, just use the first content for now
                self.apply_content(document, conflict, &conflict.content1)?;
            }
        }

        Ok(())
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

                // 如果内容是标题，提供额外选项
                if let ConflictContent::Paragraph(text1) = &conflict.content1 {
                    if text1.contains(". ")
                        && text1.split(". ").next().unwrap().parse::<usize>().is_ok()
                    {
                        options.push((
                            Resolution::UseCustom("重新编号".to_string()),
                            "重新编号并保留两者".to_string(),
                        ));
                    }
                }
            }
            // ... 其他冲突类型的选项 ...
            _ => {
                options.push((Resolution::UseFirst, "使用第一个文档的版本".to_string()));
                options.push((Resolution::UseSecond, "使用第二个文档的版本".to_string()));
                options.push((Resolution::UseBase, "使用原始版本".to_string()));
            }
        }

        options
    }
}

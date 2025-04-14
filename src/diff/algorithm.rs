//! Implements the difference detection algorithm for docx documents.

use similar::{Algorithm, ChangeTag, TextDiff};

use crate::diff::{
    DiffContent, DiffLocation, DiffOperation, DiffOptions, DocumentDiff, DocumentDiffer,
};
use crate::docx::model::{Document, Paragraph, RunContent};
use crate::error::Result;

/// Implementation of the document difference detector
pub struct DocxDiffer;

impl DocxDiffer {
    /// Create a new document differ
    pub fn new() -> Self {
        DocxDiffer
    }

    /// Extract text from a paragraph
    fn extract_text_from_paragraph(paragraph: &Paragraph) -> String {
        let mut text = String::new();

        for run in &paragraph.runs {
            for content in &run.contents {
                if let RunContent::Text(t) = content {
                    text.push_str(t);
                } else if let RunContent::Break = content {
                    text.push('\n');
                } else if let RunContent::Tab = content {
                    text.push('\t');
                }
            }
        }

        text
    }

    /// Prepare document for diffing by extracting paragraphs and their text
    fn prepare_document(&self, doc: &Document, options: &DiffOptions) -> Vec<(usize, String)> {
        let mut result = Vec::new();

        for (i, paragraph) in doc.body.paragraphs.iter().enumerate() {
            let mut text = Self::extract_text_from_paragraph(paragraph);

            if options.ignore_whitespace {
                text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            }

            if options.ignore_case {
                text = text.to_lowercase();
            }

            result.push((i, text));
        }

        result
    }

    /// Compare two paragraphs and generate text-level diffs
    fn diff_paragraphs(
        &self,
        para1: &Paragraph,
        para2: &Paragraph,
        options: &DiffOptions,
    ) -> Vec<DiffOperation> {
        let text1 = Self::extract_text_from_paragraph(para1);
        let text2 = Self::extract_text_from_paragraph(para2);

        // Use the similar crate to get character-level diffs
        let diff = TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .timeout(std::time::Duration::from_secs(5))
            .diff_chars(&text1, &text2);

        let mut operations = Vec::new();
        let mut pos1 = 0;
        let mut pos2 = 0;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    let text = change.value().to_string();
                    operations.push(DiffOperation::Deletion {
                        location: DiffLocation {
                            paragraph_index: 0, // Will be set later
                            start_pos: pos1,
                            end_pos: pos1 + text.len(),
                        },
                        content: DiffContent::Text(text),
                    });
                    pos1 += change.value().len();
                }
                ChangeTag::Insert => {
                    let text = change.value().to_string();
                    operations.push(DiffOperation::Addition {
                        location: DiffLocation {
                            paragraph_index: 0, // Will be set later
                            start_pos: pos2,
                            end_pos: pos2 + text.len(),
                        },
                        content: DiffContent::Text(text),
                    });
                    pos2 += change.value().len();
                }
                ChangeTag::Equal => {
                    let text = change.value().to_string();
                    operations.push(DiffOperation::Equal {
                        location: DiffLocation {
                            paragraph_index: 0, // Will be set later
                            start_pos: pos1,
                            end_pos: pos1 + text.len(),
                        },
                        content: DiffContent::Text(text),
                    });
                    pos1 += change.value().len();
                    pos2 += change.value().len();
                }
            }
        }

        // If we're not ignoring formatting, check for formatting changes in equal sections
        if !options.ignore_formatting {
            operations = self.check_formatting_changes(para1, para2, operations);
        }

        operations
    }

    /// Check for formatting changes in the equal sections
    fn check_formatting_changes(
        &self,
        para1: &Paragraph,
        para2: &Paragraph,
        operations: Vec<DiffOperation>,
    ) -> Vec<DiffOperation> {
        let mut result = Vec::new();

        // This is a simplified implementation - a complete one would map character positions to runs
        // and compare the formatting of each character

        // For now, we'll just check if any run properties are different
        let runs1 = &para1.runs;
        let runs2 = &para2.runs;

        // Simple case: check if the number of runs is different
        if runs1.len() != runs2.len() {
            // Just return the original operations for now
            return operations;
        }

        for op in operations {
            match op {
                DiffOperation::Equal {
                    location,
                    content: DiffContent::Text(text),
                } => {
                    // In a real implementation, we would check the formatting of each character
                    // in this equal section by mapping to the runs
                    // For now, just pass it through
                    result.push(DiffOperation::Equal {
                        location,
                        content: DiffContent::Text(text),
                    });
                }
                _ => result.push(op),
            }
        }

        result
    }
}

impl DocumentDiffer for DocxDiffer {
    fn diff(
        &self,
        doc1: &Document,
        doc2: &Document,
        options: &DiffOptions,
    ) -> Result<DocumentDiff> {
        // Prepare documents for comparison
        let paras1 = self.prepare_document(doc1, options);
        let paras2 = self.prepare_document(doc2, options);

        // 修复临时值借用问题：先创建持久化的字符串
        let text1 = paras1
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let text2 = paras2
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // 使用持久化的字符串变量
        let diff = TextDiff::configure()
            .algorithm(Algorithm::Patience)
            .diff_lines(&text1, &text2);

        let mut operations = Vec::new();

        // Process paragraph-level diffs
        let mut para_index1 = 0;
        let mut para_index2 = 0;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    // Find the original paragraph index
                    if let Some((orig_index, _)) = paras1.get(para_index1) {
                        if let Some(paragraph) = doc1.body.paragraphs.get(*orig_index) {
                            operations.push(DiffOperation::Deletion {
                                location: DiffLocation {
                                    paragraph_index: *orig_index,
                                    start_pos: 0,
                                    end_pos: Self::extract_text_from_paragraph(paragraph).len(),
                                },
                                content: DiffContent::Text(Self::extract_text_from_paragraph(
                                    paragraph,
                                )),
                            });
                        }
                    }
                    para_index1 += 1;
                }
                ChangeTag::Insert => {
                    // Find the original paragraph index
                    if let Some((orig_index, _)) = paras2.get(para_index2) {
                        if let Some(paragraph) = doc2.body.paragraphs.get(*orig_index) {
                            operations.push(DiffOperation::Addition {
                                location: DiffLocation {
                                    paragraph_index: *orig_index,
                                    start_pos: 0,
                                    end_pos: Self::extract_text_from_paragraph(paragraph).len(),
                                },
                                content: DiffContent::Text(Self::extract_text_from_paragraph(
                                    paragraph,
                                )),
                            });
                        }
                    }
                    para_index2 += 1;
                }
                ChangeTag::Equal => {
                    // Get both original paragraph indices
                    if let (Some((orig_index1, _)), Some((orig_index2, _))) =
                        (paras1.get(para_index1), paras2.get(para_index2))
                    {
                        if let (Some(paragraph1), Some(paragraph2)) = (
                            doc1.body.paragraphs.get(*orig_index1),
                            doc2.body.paragraphs.get(*orig_index2),
                        ) {
                            // For equal paragraphs, we still want to check for character-level diffs
                            let mut para_ops =
                                self.diff_paragraphs(paragraph1, paragraph2, options);

                            // Update paragraph indices in the operations
                            for op in &mut para_ops {
                                match op {
                                    DiffOperation::Addition { location, .. }
                                    | DiffOperation::Deletion { location, .. }
                                    | DiffOperation::Modification { location, .. }
                                    | DiffOperation::Equal { location, .. } => {
                                        location.paragraph_index = *orig_index1;
                                    }
                                }
                            }

                            operations.extend(para_ops);
                        }
                    }
                    para_index1 += 1;
                    para_index2 += 1;
                }
            }
        }

        Ok(DocumentDiff { operations })
    }

    fn are_equal(&self, doc1: &Document, doc2: &Document, options: &DiffOptions) -> Result<bool> {
        let diff = self.diff(doc1, doc2, options)?;

        // Documents are equal if there are no additions, deletions, or modifications
        Ok(!diff.operations.iter().any(|op| {
            matches!(
                op,
                DiffOperation::Addition { .. }
                    | DiffOperation::Deletion { .. }
                    | DiffOperation::Modification { .. }
            )
        }))
    }
}

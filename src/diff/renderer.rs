// src/diff/renderer.rs 修正版
//! Renders document differences in various formats.

use crate::diff::{DiffContent, DiffOperation, DocumentDiff};
use crate::error::Result;

/// Renderer for document differences
pub struct DiffRenderer;

impl DiffRenderer {
    /// Create a new diff renderer
    pub fn new() -> Self {
        DiffRenderer
    }
    
    /// Render diff as colored text (for terminal output)
    pub fn render_as_text(&self, diff: &DocumentDiff) -> Result<String> {
        let mut result = String::new();
        
        for op in &diff.operations {
            match op {
                DiffOperation::Addition { location, content } => {
                    if let DiffContent::Text(text) = content {
                        result.push_str(&format!("\u{001b}[32m+ [P{}:{}] {}\u{001b}[0m\n", 
                            location.paragraph_index + 1, 
                            location.start_pos,
                            text
                        ));
                    }
                }
                DiffOperation::Deletion { location, content } => {
                    if let DiffContent::Text(text) = content {
                        result.push_str(&format!("\u{001b}[31m- [P{}:{}] {}\u{001b}[0m\n", 
                            location.paragraph_index + 1, 
                            location.start_pos,
                            text
                        ));
                    }
                }
                DiffOperation::Modification { location, old_content, new_content } => {
                    if let (DiffContent::Text(old_text), DiffContent::Text(new_text)) = (old_content, new_content) {
                        result.push_str(&format!("\u{001b}[33m~ [P{}:{}] {} -> {}\u{001b}[0m\n", 
                            location.paragraph_index + 1, 
                            location.start_pos,
                            old_text,
                            new_text
                        ));
                    }
                }
                DiffOperation::Equal { .. } => {
                    // Skip equal parts for brevity in text output
                }
            }
        }
        
        Ok(result)
    }
    
    /// Render diff as HTML
    pub fn render_as_html(&self, diff: &DocumentDiff) -> Result<String> {
        let mut result = String::from("<html><head><style>\n");
        result.push_str(".addition { background-color: #ccffcc; }\n");
        result.push_str(".deletion { background-color: #ffcccc; text-decoration: line-through; }\n");
        result.push_str(".modification { background-color: #ffffcc; }\n");
        result.push_str("</style></head><body><div class='diff'>\n");
        
        let mut current_paragraph = 0;
        let mut paragraph_open = false;
        
        // 修复模式匹配：分别处理每种操作类型
        for op in &diff.operations {
            // 检查是否需要开始新段落
            match op {
                DiffOperation::Addition { location, .. } |
                DiffOperation::Deletion { location, .. } |
                DiffOperation::Modification { location, .. } |
                DiffOperation::Equal { location, .. } => {
                    if location.paragraph_index != current_paragraph {
                        if paragraph_open {
                            result.push_str("</p>\n");
                        }
                        result.push_str(&format!("<p class='paragraph' data-index='{}'>\n", location.paragraph_index + 1));
                        paragraph_open = true;
                        current_paragraph = location.paragraph_index;
                    }
                }
            }
            
            // 添加带有适当样式的内容
            match op {
                DiffOperation::Addition { content, .. } => {
                    if let DiffContent::Text(text) = content {
                        result.push_str(&format!("<span class='addition'>{}</span>", html_escape::encode_text(text)));
                    }
                }
                DiffOperation::Deletion { content, .. } => {
                    if let DiffContent::Text(text) = content {
                        result.push_str(&format!("<span class='deletion'>{}</span>", html_escape::encode_text(text)));
                    }
                }
                DiffOperation::Modification { old_content, new_content, .. } => {
                    if let (DiffContent::Text(old_text), DiffContent::Text(new_text)) = (old_content, new_content) {
                        result.push_str(&format!(
                            "<span class='modification' title='Was: {}'>{}</span>",
                            html_escape::encode_text(old_text),
                            html_escape::encode_text(new_text)
                        ));
                    }
                }
                DiffOperation::Equal { content, .. } => {
                    if let DiffContent::Text(text) = content {
                        result.push_str(&html_escape::encode_text(text));
                    }
                }
            }
        }
        
        if paragraph_open {
            result.push_str("</p>\n");
        }
        
        result.push_str("</div></body></html>");
        
        Ok(result)
    }
    
    /// Render diff in Word change tracking format (simplified)
    pub fn render_as_word_tracking(&self, diff: &DocumentDiff) -> Result<String> {
        // This would actually generate a new docx with change tracking
        // For now, just return a placeholder
        Ok("Word change tracking format not implemented yet".to_string())
    }
}
//! Convert docx documents to Markdown.

use crate::convert::ConversionOptions;
use crate::docx::model::{Document, Paragraph, Run, RunContent};
use crate::error::Result;

/// Converter for docx to Markdown
pub struct DocxToMarkdownConverter;

impl DocxToMarkdownConverter {
    /// Create a new docx to Markdown converter
    pub fn new() -> Self {
        DocxToMarkdownConverter
    }

    /// Convert a document to Markdown
    pub fn convert(&self, document: &Document, options: &ConversionOptions) -> Result<String> {
        let mut markdown = String::new();

        for paragraph in &document.body.paragraphs {
            // 检测是否是标题
            let heading_level = self.detect_heading_level(paragraph);

            if heading_level > 0 {
                // 添加Markdown标题
                markdown.push_str(&format!("{} ", "#".repeat(heading_level)));
            }

            // 处理段落文本
            let mut paragraph_text = String::new();

            // 检查是否是列表项
            let (is_list_item, list_prefix) = self.detect_list_item(paragraph);
            if is_list_item {
                markdown.push_str(&list_prefix);
                markdown.push(' ');
            }

            for run in &paragraph.runs {
                let text = self.extract_run_text(run);
                let formatted_text = self.format_text(run, &text);
                paragraph_text.push_str(&formatted_text);
            }

            markdown.push_str(&paragraph_text);

            // 段落结束添加两个换行
            markdown.push_str("\n\n");
        }

        // 删除末尾多余的换行
        if markdown.ends_with("\n\n") {
            markdown.truncate(markdown.len() - 2);
        }

        Ok(markdown)
    }

    /// 从段落样式检测标题级别
    fn detect_heading_level(&self, paragraph: &Paragraph) -> usize {
        // 基于段落样式检测标题级别
        if let Some(style_id) = &paragraph.style_id {
            match style_id.as_str() {
                "Heading1" | "heading 1" | "Title" => return 1,
                "Heading2" | "heading 2" | "Subtitle" => return 2,
                "Heading3" | "heading 3" => return 3,
                "Heading4" | "heading 4" => return 4,
                "Heading5" | "heading 5" => return 5,
                "Heading6" | "heading 6" => return 6,
                _ => {}
            }
        }

        // 根据段落格式进一步检测
        // 有些文档不使用样式ID，而是直接设置字体大小、粗体等
        // 这里简化处理，只检查第一个run
        if let Some(run) = paragraph.runs.first() {
            if run.properties.bold && run.properties.size.map_or(false, |size| size > 24) {
                return 1; // 大号粗体可能是标题1
            } else if run.properties.bold && run.properties.size.map_or(false, |size| size > 20) {
                return 2; // 中号粗体可能是标题2
            } else if run.properties.bold {
                return 3; // 普通粗体可能是标题3
            }
        }

        0 // 不是标题
    }

    /// 提取运行中的文本
    fn extract_run_text(&self, run: &Run) -> String {
        let mut text = String::new();
        for content in &run.contents {
            match content {
                RunContent::Text(t) => text.push_str(t),
                RunContent::Break => text.push('\n'),
                RunContent::Tab => text.push_str("    "),
                RunContent::FootnoteReference(id) => {
                    text.push_str(&format!("[^{}]", id));
                }
                _ => {} // 忽略其他内容类型
            }
        }
        text
    }

    /// 格式化文本，应用Markdown格式
    fn format_text(&self, run: &Run, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        let mut result = text.to_string();

        // 应用格式
        if run.properties.bold && run.properties.italic {
            result = format!("***{}***", result);
        } else if run.properties.bold {
            result = format!("**{}**", result);
        } else if run.properties.italic {
            result = format!("*{}*", result);
        }

        // 下划线可以用HTML标签表示
        if run.properties.underline {
            result = format!("<u>{}</u>", result);
        }

        result
    }

    /// 检测是否是列表项
    fn detect_list_item(&self, paragraph: &Paragraph) -> (bool, String) {
        // 检查段落是否有编号属性
        if let Some(numbering) = &paragraph.properties.numbering {
            // 简化处理，这里只返回通用的列表标记
            // 更复杂的实现应该根据numbering ID和level返回适当的标记
            (true, "* ".to_string()) // 默认使用无序列表
        } else {
            // 检查段落文本是否以列表标记开头
            if let Some(first_run) = paragraph.runs.first() {
                if let Some(RunContent::Text(text)) = first_run.contents.first() {
                    let trimmed = text.trim_start();
                    if trimmed.starts_with("•")
                        || trimmed.starts_with("-")
                        || trimmed.starts_with("*")
                    {
                        return (true, "*".to_string());
                    } else if trimmed.starts_with("1.") || trimmed.starts_with("1)") {
                        return (true, "1.".to_string());
                    }
                }
            }

            (false, String::new())
        }
    }
}

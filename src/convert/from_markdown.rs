//! Convert Markdown to docx documents.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::convert::{ConversionOptions, FormatConverter, StandardConverter};
use crate::docx::model::{Document, Paragraph, Run, RunContent, RunProperties};
use crate::error::Result;

// 注意：我们只在这个文件中提供 FormatConverter 的实现
impl FormatConverter for StandardConverter {
    fn docx_to_markdown(&self, document: &Document, options: &ConversionOptions) -> Result<String> {
        // 创建一个新的转换器来处理 docx 到 markdown 的转换
        let converter = crate::convert::to_markdown::DocxToMarkdownConverter::new();
        converter.convert(document, options)
    }

    fn markdown_to_docx(
        &self,
        markdown: &str,
        template: Option<&Document>,
        options: &ConversionOptions,
    ) -> Result<Document> {
        // 创建一个新文档，或者从模板克隆
        let mut document = if let Some(tmpl) = template {
            // 从模板克隆，但清除内容
            let mut doc = tmpl.clone();
            doc.body.paragraphs.clear();
            doc
        } else {
            Document::new()
        };

        // 解析Markdown
        let parser = Parser::new(markdown);

        // 当前段落和运行的状态
        let mut current_paragraph = Paragraph {
            id: None,
            style_id: None,
            properties: Default::default(),
            runs: Vec::new(),
        };

        // 当前格式状态
        let mut current_props = RunProperties::default();

        // 是否在段落内
        let mut in_paragraph = false;

        // 当前标题级别
        let mut heading_level = 0;

        // 处理解析事件
        for event in parser {
            match event {
                Event::Start(Tag::Paragraph) => {
                    in_paragraph = true;
                    current_paragraph = Paragraph {
                        id: None,
                        style_id: Some("Normal".to_string()),
                        properties: Default::default(),
                        runs: Vec::new(),
                    };
                }
                Event::End(TagEnd::Paragraph) => {
                    in_paragraph = false;
                    if !current_paragraph.runs.is_empty() {
                        document.body.paragraphs.push(current_paragraph.clone());
                        current_paragraph.runs.clear();
                    }
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    in_paragraph = true;
                    // 将 HeadingLevel 转换为数值
                    heading_level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };

                    current_paragraph = Paragraph {
                        id: None,
                        style_id: Some(format!("Heading{}", heading_level)),
                        properties: Default::default(),
                        runs: Vec::new(),
                    };
                }
                Event::End(TagEnd::Heading { .. }) => {
                    in_paragraph = false;
                    heading_level = 0;
                    if !current_paragraph.runs.is_empty() {
                        document.body.paragraphs.push(current_paragraph.clone());
                        current_paragraph.runs.clear();
                    }
                }
                Event::Start(Tag::Emphasis) => {
                    current_props.italic = true;
                }
                Event::End(TagEnd::Emphasis) => {
                    current_props.italic = false;
                }
                Event::Start(Tag::Strong) => {
                    current_props.bold = true;
                }
                Event::End(TagEnd::Strong) => {
                    current_props.bold = false;
                }
                Event::Start(Tag::Item) => {
                    in_paragraph = true;
                    current_paragraph = Paragraph {
                        id: None,
                        style_id: Some("ListParagraph".to_string()),
                        properties: Default::default(),
                        runs: Vec::new(),
                    };

                    // 添加列表项标记
                    let mut run = Run {
                        properties: current_props.clone(),
                        contents: vec![RunContent::Text("• ".to_string())],
                    };
                    current_paragraph.runs.push(run);
                }
                Event::End(TagEnd::Item) => {
                    in_paragraph = false;
                    if !current_paragraph.runs.is_empty() {
                        document.body.paragraphs.push(current_paragraph.clone());
                        current_paragraph.runs.clear();
                    }
                }
                Event::Text(text) => {
                    if in_paragraph {
                        let run = Run {
                            properties: current_props.clone(),
                            contents: vec![RunContent::Text(text.to_string())],
                        };
                        current_paragraph.runs.push(run);
                    }
                }
                Event::SoftBreak => {
                    if in_paragraph {
                        let run = Run {
                            properties: current_props.clone(),
                            contents: vec![RunContent::Text(" ".to_string())],
                        };
                        current_paragraph.runs.push(run);
                    }
                }
                Event::HardBreak => {
                    if in_paragraph {
                        let run = Run {
                            properties: current_props.clone(),
                            contents: vec![RunContent::Break],
                        };
                        current_paragraph.runs.push(run);
                    }
                }
                // 其他事件类型可以根据需要添加
                _ => {}
            }
        }

        Ok(document)
    }
}

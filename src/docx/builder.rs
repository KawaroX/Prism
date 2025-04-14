//! Builder for creating docx files.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use zip::{CompressionMethod, ZipWriter, write::FileOptions};

use crate::docx::model::*;
use crate::error::Result;

/// Builder for docx files
pub struct DocxBuilder;

impl DocxBuilder {
    /// Create a new docx file from a Document
    pub fn build<P: AsRef<Path>>(document: &Document, path: P) -> Result<()> {
        // 创建一个新的 zip 文件
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);

        // 设置文件选项
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // 添加必要的文件

        // [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(indoc::indoc!(r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
                <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
                <Default Extension="xml" ContentType="application/xml"/>
                <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
                <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
                <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
                <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
            </Types>
        "#).as_bytes())?;

        // _rels/.rels
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(indoc::indoc!(r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
                <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
                <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
                <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
            </Relationships>
        "#).as_bytes())?;

        // word/_rels/document.xml.rels
        zip.start_file("word/_rels/document.xml.rels", options)?;
        zip.write_all(indoc::indoc!(r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
                <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
            </Relationships>
        "#).as_bytes())?;

        // docProps/app.xml
        zip.start_file("docProps/app.xml", options)?;
        zip.write_all(indoc::indoc!(r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
                <Application>Prism</Application>
                <AppVersion>0.1.0</AppVersion>
            </Properties>
        "#).as_bytes())?;

        // docProps/core.xml
        zip.start_file("docProps/core.xml", options)?;
        let created = document.properties.created.as_deref().unwrap_or_default();
        let modified = document.properties.modified.as_deref().unwrap_or_default();
        let title = document.properties.title.as_deref().unwrap_or_default();
        let author = document.properties.author.as_deref().unwrap_or_default();

        zip.write_all(format!(indoc::indoc!(r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
                <dc:title>{}</dc:title>
                <dc:creator>{}</dc:creator>
                <dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>
                <dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified>
            </cp:coreProperties>
        "#), title, author, created, modified).as_bytes())?;

        // word/styles.xml
        zip.start_file("word/styles.xml", options)?;
        zip.write_all(
            indoc::indoc!(
                r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
                    <w:name w:val="Normal"/>
                    <w:pPr/>
                    <w:rPr>
                        <w:sz w:val="24"/>
                        <w:szCs w:val="24"/>
                        <w:lang w:val="en-US" w:eastAsia="en-US" w:bidi="ar-SA"/>
                    </w:rPr>
                </w:style>
            </w:styles>
        "#
            )
            .as_bytes(),
        )?;

        // word/document.xml
        zip.start_file("word/document.xml", options)?;

        // 开始写入文档内容
        zip.write_all(
            indoc::indoc!(
                r#"
            <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:body>
        "#
            )
            .as_bytes(),
        )?;

        // 写入段落
        for paragraph in &document.body.paragraphs {
            zip.write_all(b"<w:p>")?;

            // 段落属性
            if paragraph.style_id.is_some() || paragraph.properties.alignment.is_some() {
                zip.write_all(b"<w:pPr>")?;

                if let Some(style_id) = &paragraph.style_id {
                    zip.write_all(format!("<w:pStyle w:val=\"{}\"/>", style_id).as_bytes())?;
                }

                if let Some(alignment) = &paragraph.properties.alignment {
                    zip.write_all(format!("<w:jc w:val=\"{}\"/>", alignment).as_bytes())?;
                }

                zip.write_all(b"</w:pPr>")?;
            }

            // 写入运行
            for run in &paragraph.runs {
                zip.write_all(b"<w:r>")?;

                // 运行属性
                let props = &run.properties;
                if props.bold
                    || props.italic
                    || props.underline
                    || props.size.is_some()
                    || props.font.is_some()
                    || props.color.is_some()
                {
                    zip.write_all(b"<w:rPr>")?;

                    if props.bold {
                        zip.write_all(b"<w:b/>")?;
                    }

                    if props.italic {
                        zip.write_all(b"<w:i/>")?;
                    }

                    if props.underline {
                        zip.write_all(b"<w:u w:val=\"single\"/>")?;
                    }

                    if let Some(size) = props.size {
                        zip.write_all(format!("<w:sz w:val=\"{}\"/>", size).as_bytes())?;
                    }

                    if let Some(font) = &props.font {
                        zip.write_all(
                            format!("<w:rFonts w:ascii=\"{}\" w:hAnsi=\"{}\"/>", font, font)
                                .as_bytes(),
                        )?;
                    }

                    if let Some(color) = &props.color {
                        zip.write_all(format!("<w:color w:val=\"{}\"/>", color).as_bytes())?;
                    }

                    zip.write_all(b"</w:rPr>")?;
                }

                // 运行内容
                for content in &run.contents {
                    match content {
                        RunContent::Text(text) => {
                            zip.write_all(format!("<w:t>{}</w:t>", escape_xml(text)).as_bytes())?;
                        }
                        RunContent::Break => {
                            zip.write_all(b"<w:br/>")?;
                        }
                        RunContent::Tab => {
                            zip.write_all(b"<w:tab/>")?;
                        }
                        _ => { /* 忽略其他内容类型 */ }
                    }
                }

                zip.write_all(b"</w:r>")?;
            }

            zip.write_all(b"</w:p>")?;
        }

        // 结束文档
        zip.write_all(
            indoc::indoc!(
                r#"
                </w:body>
            </w:document>
        "#
            )
            .as_bytes(),
        )?;

        // 完成 zip 文件
        zip.finish()?;

        Ok(())
    }
}

// 辅助函数：转义 XML 特殊字符
fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

use prism::DocxBuilder;
use prism::convert::{ConversionOptions, FormatConverter, StandardConverter};
use prism::docx::RunProperties;
use prism::docx::{Document, Paragraph, Run, RunContent};
use prism::error::Result;

use tempfile::TempDir;

#[test]
fn test_docx_to_markdown() -> Result<()> {
    // 创建一个简单的文档用于测试
    let mut document = Document::new();

    // 添加一个标题
    let heading = Paragraph {
        id: None,
        style_id: Some("Heading1".to_string()),
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text("Test Heading".to_string())],
        }],
    };

    // 添加一个普通段落
    let paragraph = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![
            Run {
                properties: Default::default(),
                contents: vec![RunContent::Text("This is a ".to_string())],
            },
            Run {
                properties: {
                    let mut props: RunProperties = Default::default();
                    props.bold = true;
                    props
                },
                contents: vec![RunContent::Text("bold".to_string())],
            },
            Run {
                properties: Default::default(),
                contents: vec![RunContent::Text(" and ".to_string())],
            },
            Run {
                properties: {
                    let mut props: RunProperties = Default::default();
                    props.italic = true;
                    props
                },
                contents: vec![RunContent::Text("italic".to_string())],
            },
            Run {
                properties: Default::default(),
                contents: vec![RunContent::Text(" text.".to_string())],
            },
        ],
    };

    document.body.paragraphs.push(heading);
    document.body.paragraphs.push(paragraph);

    // 转换为Markdown
    let converter = StandardConverter::new();
    let options = ConversionOptions::default();
    let markdown = converter.docx_to_markdown(&document, &options)?;

    // 验证结果
    assert!(
        markdown.contains("# Test Heading"),
        "Heading not converted correctly"
    );
    assert!(
        markdown.contains("This is a **bold** and *italic* text"),
        "Formatting not converted correctly"
    );

    Ok(())
}

#[test]
fn test_markdown_to_docx() -> Result<()> {
    // 创建一个简单的Markdown文本
    let markdown =
        "# Test Heading\n\nThis is a **bold** and *italic* text.\n\n* List item 1\n* List item 2";

    // 转换为docx
    let converter = StandardConverter::new();
    let options = ConversionOptions::default();
    let document = converter.markdown_to_docx(markdown, None, &options)?;

    // 验证结果
    assert_eq!(document.body.paragraphs.len(), 4, "Expected 4 paragraphs");

    // 检查标题
    assert_eq!(
        document.body.paragraphs[0].style_id,
        Some("Heading1".to_string()),
        "Heading style not set correctly"
    );

    // 检查列表项 - 使用匹配模式而不是 to_string()
    if let RunContent::Text(text) = &document.body.paragraphs[2].runs[0].contents[0] {
        assert!(text.contains("•"), "List item not converted correctly");
    } else {
        panic!("Expected Text content in list item");
    }

    // 保存为临时文件以手动检查
    let temp_dir = TempDir::new()?;
    let doc_path = temp_dir.path().join("test_md_to_docx.docx");
    DocxBuilder::build(&document, &doc_path)?;

    println!("Document saved to: {:?}", doc_path);

    Ok(())
}

#[test]
fn test_roundtrip_conversion() -> Result<()> {
    // 测试文档->Markdown->文档的往返转换

    // 创建简单文档
    let mut document = Document::new();
    let paragraph = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text("This is a test document.".to_string())],
        }],
    };
    document.body.paragraphs.push(paragraph);

    // 文档 -> Markdown
    let converter = StandardConverter::new();
    let options = ConversionOptions::default();
    let markdown = converter.docx_to_markdown(&document, &options)?;

    // Markdown -> 文档
    let converted_document = converter.markdown_to_docx(&markdown, None, &options)?;

    // 验证结果
    assert_eq!(
        converted_document.body.paragraphs.len(),
        document.body.paragraphs.len(),
        "Paragraph count mismatch after roundtrip conversion"
    );

    // 比较文本内容
    let original_text = extract_document_text(&document);
    let converted_text = extract_document_text(&converted_document);

    assert_eq!(
        original_text, converted_text,
        "Text content changed after roundtrip conversion"
    );

    Ok(())
}

// 辅助函数：提取文档中的所有文本
fn extract_document_text(document: &Document) -> String {
    let mut text = String::new();

    for paragraph in &document.body.paragraphs {
        for run in &paragraph.runs {
            for content in &run.contents {
                if let RunContent::Text(t) = content {
                    text.push_str(t);
                } else if let RunContent::Break = content {
                    text.push('\n');
                } else if let RunContent::Tab = content {
                    text.push_str("    ");
                }
            }
        }
        text.push('\n');
    }

    text
}

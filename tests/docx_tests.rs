use prism::docx::DocxParser;
use prism::error::Result;
use std::path::Path;

#[test]
fn test_parse_docx() -> Result<()> {
    let path = Path::new("tests/resources/test_document.docx");
    
    if !path.exists() {
        eprintln!("Test document not found at {:?}", path);
        return Ok(());
    }
    
    let document = DocxParser::parse_file(path)?;
    
    // Basic validation of the parsed document
    assert!(!document.body.paragraphs.is_empty(), "Document should have at least one paragraph");
    
    // Print some basic information about the document
    println!("Document successfully parsed:");
    println!("  - Paragraphs: {}", document.body.paragraphs.len());
    println!("  - Tables: {}", document.body.tables.len());
    println!("  - Sections: {}", document.body.sections.len());
    
    // Print the content of the first paragraph
    if let Some(paragraph) = document.body.paragraphs.first() {
        println!("First paragraph:");
        for run in &paragraph.runs {
            for content in &run.contents {
                if let prism::docx::model::RunContent::Text(text) = content {
                    println!("    - Text: {}", text);
                }
            }
        }
    }
    
    Ok(())
}
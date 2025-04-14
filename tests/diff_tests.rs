use prism::docx::{Document, DocxParser, Paragraph, Run, RunContent};
use prism::diff::{DiffOptions, DocxDiffer, DocumentDiffer};
use prism::error::Result;
use std::path::Path;

#[test]
fn test_simple_diff() -> Result<()> {
    // Create two simple documents for testing
    let mut doc1 = Document::new();
    let mut doc2 = Document::new();
    
    // Add paragraphs to doc1
    let paragraph1 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![
            Run {
                properties: Default::default(),
                contents: vec![
                    RunContent::Text("This is a test document.".to_string())
                ],
            }
        ],
    };
    
    let paragraph2 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![
            Run {
                properties: Default::default(),
                contents: vec![
                    RunContent::Text("This paragraph will be modified.".to_string())
                ],
            }
        ],
    };
    
    doc1.body.paragraphs.push(paragraph1.clone());
    doc1.body.paragraphs.push(paragraph2);
    
    // Add paragraphs to doc2 (with a change in the second paragraph)
    doc2.body.paragraphs.push(paragraph1);
    
    let paragraph2_modified = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![
            Run {
                properties: Default::default(),
                contents: vec![
                    RunContent::Text("This paragraph has been modified.".to_string())
                ],
            }
        ],
    };
    
    doc2.body.paragraphs.push(paragraph2_modified);
    
    // Add a new paragraph to doc2
    let paragraph3 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![
            Run {
                properties: Default::default(),
                contents: vec![
                    RunContent::Text("This is a new paragraph.".to_string())
                ],
            }
        ],
    };
    
    doc2.body.paragraphs.push(paragraph3);
    
    // Diff the documents
    let differ = DocxDiffer::new();
    let options = DiffOptions::default();
    let diff = differ.diff(&doc1, &doc2, &options)?;
    
    // Check that the diff contains the expected operations
    assert!(!diff.operations.is_empty(), "Diff should have operations");
    
    // Print the diff for debugging
    println!("Diff operations: {}", diff.operations.len());
    for (i, op) in diff.operations.iter().enumerate() {
        println!("Operation {}: {:?}", i, op);
    }
    
    Ok(())
}

#[test]
fn test_document_diff() -> Result<()> {
    let path = Path::new("tests/resources/test_document.docx");
    
    if !path.exists() {
        eprintln!("Test document not found at {:?}, skipping test", path);
        return Ok(());
    }
    
    // Parse the original document
    let doc1 = DocxParser::parse_file(path)?;
    
    // Create a modified copy of the document
    let mut doc2 = doc1.clone();
    
    // Modify a paragraph in doc2 if one exists
    if !doc2.body.paragraphs.is_empty() {
        if let Some(run) = doc2.body.paragraphs[0].runs.first_mut() {
            for content in &mut run.contents {
                if let RunContent::Text(text) = content {
                    // Modify the text
                    *text = format!("{} [MODIFIED]", text);
                    break;
                }
            }
        }
    }
    
    // Diff the documents
    let differ = DocxDiffer::new();
    let options = DiffOptions::default();
    let diff = differ.diff(&doc1, &doc2, &options)?;
    
    // Verify the documents are different
    assert!(!differ.are_equal(&doc1, &doc2, &options)?, "Documents should be different");
    
    // Verify against the same document (should be equal)
    assert!(differ.are_equal(&doc1, &doc1, &options)?, "Document should be equal to itself");
    
    // Print the diff for debugging
    println!("Diff operations: {}", diff.operations.len());
    
    Ok(())
}
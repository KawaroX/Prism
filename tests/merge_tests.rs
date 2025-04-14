use prism::docx::{Document, Paragraph, Run, RunContent};
use prism::error::Result;
use prism::merge::InsertConflictHandling::KeepBoth;
use prism::merge::strategy::ThreeWayMerger;
use prism::merge::{DocumentMerger, MergeOptions, Resolution};

#[test]
fn test_simple_merge() -> Result<()> {
    // Create three simple documents for testing
    let mut base = Document::new();
    let mut doc1 = Document::new();
    let mut doc2 = Document::new();

    // Add paragraphs to base
    let paragraph1 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text("This is a base paragraph.".to_string())],
        }],
    };

    let paragraph2 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This paragraph will be modified differently.".to_string(),
            )],
        }],
    };

    base.body.paragraphs.push(paragraph1.clone());
    base.body.paragraphs.push(paragraph2.clone());

    // Doc1 modifies paragraph 2
    doc1.body.paragraphs.push(paragraph1.clone());

    let paragraph2_doc1 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This paragraph was modified by doc1.".to_string(),
            )],
        }],
    };

    doc1.body.paragraphs.push(paragraph2_doc1);

    // Doc2 adds a new paragraph and leaves paragraph 2 unchanged
    doc2.body.paragraphs.push(paragraph1.clone());
    doc2.body.paragraphs.push(paragraph2.clone());

    let paragraph3 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This is a new paragraph added by doc2.".to_string(),
            )],
        }],
    };

    doc2.body.paragraphs.push(paragraph3);

    // Merge the documents
    let merger = ThreeWayMerger::new();
    let options = MergeOptions::default();
    let result = merger.merge(&base, &doc1, &doc2, &options)?;

    // Check the merge result
    let merged_doc = result.document;

    // Should have 3 paragraphs in the merged document
    assert_eq!(
        merged_doc.body.paragraphs.len(),
        3,
        "Merged document should have 3 paragraphs"
    );

    // First paragraph should be unchanged
    let first_para_text = extract_paragraph_text(&merged_doc.body.paragraphs[0]);
    assert_eq!(
        first_para_text, "This is a base paragraph.",
        "First paragraph should be unchanged"
    );

    // Second paragraph should be modified by doc1
    let second_para_text = extract_paragraph_text(&merged_doc.body.paragraphs[1]);
    assert_eq!(
        second_para_text, "This paragraph was modified by doc1.",
        "Second paragraph should be modified by doc1"
    );

    // Third paragraph should be added by doc2
    let third_para_text = extract_paragraph_text(&merged_doc.body.paragraphs[2]);
    assert_eq!(
        third_para_text, "This is a new paragraph added by doc2.",
        "Third paragraph should be from doc2"
    );

    // Check for conflicts
    assert!(
        !result.has_conflicts,
        "Should not have conflicts in this simple case"
    );

    Ok(())
}

#[test]
fn test_conflict_merge() -> Result<()> {
    // Create three simple documents with conflicting changes
    let mut base = Document::new();
    let mut doc1 = Document::new();
    let mut doc2 = Document::new();

    // Add paragraphs to base
    let paragraph = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This is a base paragraph that will be modified differently.".to_string(),
            )],
        }],
    };

    base.body.paragraphs.push(paragraph.clone());

    // Doc1 modifies the paragraph
    let paragraph_doc1 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This paragraph was modified by doc1.".to_string(),
            )],
        }],
    };

    doc1.body.paragraphs.push(paragraph_doc1);

    // Doc2 modifies the paragraph differently
    let paragraph_doc2 = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This paragraph was modified by doc2.".to_string(),
            )],
        }],
    };

    doc2.body.paragraphs.push(paragraph_doc2);

    // Merge with auto-resolve disabled
    let merger = ThreeWayMerger::new();
    let options = MergeOptions {
        auto_resolve: false,
        favor_base: false,
        track_revisions: true,
        insert_conflict_handling: KeepBoth,
        preserve_conflicts: true,
    };

    let result = merger.merge(&base, &doc1, &doc2, &options)?;

    // Check the merge result
    assert!(result.has_conflicts, "Should have conflicts");
    assert_eq!(
        result.conflicts.len(),
        1,
        "Should have exactly one conflict"
    );

    // Now try to resolve the conflict
    let conflict = &result.conflicts[0];
    let resolved = merger.resolve_conflict(conflict, Resolution::UseFirst)?;

    // Check the resolved document
    let resolved_text = extract_paragraph_text(&resolved.body.paragraphs[0]);
    assert!(
        resolved_text.contains("doc1"),
        "Resolved text should use doc1's content"
    );

    Ok(())
}

// Helper function to extract text from a paragraph
fn extract_paragraph_text(paragraph: &prism::docx::model::Paragraph) -> String {
    let mut text = String::new();

    for run in &paragraph.runs {
        for content in &run.contents {
            if let prism::docx::model::RunContent::Text(t) = content {
                text.push_str(t);
            }
        }
    }

    text
}

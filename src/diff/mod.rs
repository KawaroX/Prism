//! Difference detection between docx documents.

pub mod algorithm;
pub mod renderer;

pub use algorithm::DocxDiffer;
pub use renderer::DiffRenderer;

use crate::docx::model::Document;
use crate::error::Result;

/// Options for controlling the diff algorithm
#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Whether to ignore whitespace changes
    pub ignore_whitespace: bool,
    /// Whether to ignore formatting changes
    pub ignore_formatting: bool,
    /// Whether to ignore case changes
    pub ignore_case: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        DiffOptions {
            ignore_whitespace: false,
            ignore_formatting: false,
            ignore_case: false,
        }
    }
}

/// Represents a difference between two documents
#[derive(Debug, Clone)]
pub struct DocumentDiff {
    /// The type of the diff operation
    pub operations: Vec<DiffOperation>,
}

/// Represents a single diff operation
#[derive(Debug, Clone)]
pub enum DiffOperation {
    /// Content was added (only in the second document)
    Addition {
        /// The location of the addition
        location: DiffLocation,
        /// The content that was added
        content: DiffContent,
    },
    /// Content was removed (only in the first document)
    Deletion {
        /// The location of the deletion
        location: DiffLocation,
        /// The content that was deleted
        content: DiffContent,
    },
    /// Content was modified
    Modification {
        /// The location of the modification
        location: DiffLocation,
        /// The old content
        old_content: DiffContent,
        /// The new content
        new_content: DiffContent,
    },
    /// Content remained the same
    Equal {
        /// The location in both documents
        location: DiffLocation,
        /// The content that remained the same
        content: DiffContent,
    },
}

/// The location of a diff operation in the document
#[derive(Debug, Clone)]
pub struct DiffLocation {
    /// Paragraph index
    pub paragraph_index: usize,
    /// Start character position in the paragraph
    pub start_pos: usize,
    /// End character position in the paragraph
    pub end_pos: usize,
}

/// The content involved in a diff operation
#[derive(Debug, Clone)]
pub enum DiffContent {
    /// Text content
    Text(String),
    /// Formatting change
    Formatting {
        /// The text that had formatting changed
        text: String,
        /// Old formatting properties
        old_properties: Option<FormattingProperties>,
        /// New formatting properties
        new_properties: Option<FormattingProperties>,
    },
    /// Structural change (paragraph, table, etc.)
    Structure(StructureType),
}

/// Types of structural elements that can be compared
#[derive(Debug, Clone)]
pub enum StructureType {
    /// A paragraph
    Paragraph,
    /// A table
    Table,
    /// A section
    Section,
}

/// Simplified representation of formatting properties for diff
#[derive(Debug, Clone, PartialEq)]
pub struct FormattingProperties {
    /// Bold formatting
    pub bold: bool,
    /// Italic formatting
    pub italic: bool,
    /// Underline formatting
    pub underline: bool,
    /// Font size in half-points
    pub size: Option<u32>,
    /// Font name
    pub font: Option<String>,
    /// Text color
    pub color: Option<String>,
}

impl From<&crate::docx::model::RunProperties> for FormattingProperties {
    fn from(props: &crate::docx::model::RunProperties) -> Self {
        FormattingProperties {
            bold: props.bold,
            italic: props.italic,
            underline: props.underline,
            size: props.size,
            font: props.font.clone(),
            color: props.color.clone(),
        }
    }
}

/// Main interface for document diffing
pub trait DocumentDiffer {
    /// Compare two documents and generate a diff
    fn diff(&self, doc1: &Document, doc2: &Document, options: &DiffOptions)
    -> Result<DocumentDiff>;

    /// Check if two documents are equal according to the options
    fn are_equal(&self, doc1: &Document, doc2: &Document, options: &DiffOptions) -> Result<bool>;
}

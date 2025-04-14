//! Prism - A version control system for docx files
//!
//! This library provides functionality to parse, compare, merge, and version control
//! Microsoft Word (.docx) documents, with special focus on academic writing needs.

pub mod convert;
pub mod diff;
pub mod docx;
pub mod error;
pub mod merge;
pub mod utils;
pub mod vcs;

// Re-export main types for easier usage
pub use convert::template::TemplateManager;
pub use convert::{ConversionOptions, FormatConverter, StandardConverter};
pub use diff::{DiffOptions, DiffRenderer, DocumentDiff, DocxDiffer};
pub use docx::builder::DocxBuilder;
pub use docx::model::{Document, Paragraph, Run, RunContent};
pub use docx::parser::DocxParser;
pub use error::{Error, Result};
pub use merge::strategy::ThreeWayMerger;
pub use merge::{Conflict, DocumentMerger, MergeOptions, Resolution};
pub use vcs::git::GitDocumentRepository;
pub use vcs::history::{HistoryEntry, HistoryManager};
pub use vcs::{Branch, Commit, DocumentRepository, Repository};

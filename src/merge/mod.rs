//! Merge functionality for docx documents.

pub mod conflict;
pub mod strategy;

use crate::docx::model::Document;
use crate::error::Result;

/// Represents the result of a merge operation
#[derive(Debug)]
pub struct MergeResult {
    /// The merged document
    pub document: Document,
    /// Whether conflicts were detected
    pub has_conflicts: bool,
    /// List of conflicts if any
    pub conflicts: Vec<Conflict>,
}

/// Options for controlling the merge algorithm
#[derive(Debug, Clone)]
pub struct MergeOptions {
    /// Whether to auto-resolve conflicts when possible
    pub auto_resolve: bool,
    /// Whether to favor the base document in conflict resolution
    pub favor_base: bool,
    /// Whether to include revision tracking information
    pub track_revisions: bool,
    /// How to handle insert conflicts
    pub insert_conflict_handling: InsertConflictHandling,
    /// Whether to preserve all conflicts for manual resolution
    pub preserve_conflicts: bool,
}

/// Strategy for handling insert conflicts
#[derive(Debug, Clone, PartialEq)]
pub enum InsertConflictHandling {
    /// Keep both insertions, marking them with source info
    KeepBoth,
    /// Keep only the first document's insertion
    KeepFirst,
    /// Keep only the second document's insertion
    KeepSecond,
    /// Skip both insertions
    SkipBoth,
}

impl Default for MergeOptions {
    fn default() -> Self {
        MergeOptions {
            auto_resolve: true,
            favor_base: false,
            track_revisions: true,
            insert_conflict_handling: InsertConflictHandling::KeepBoth,
            preserve_conflicts: false,
        }
    }
}

/// Main interface for document merging
pub trait DocumentMerger {
    /// Merge two documents with a common base
    fn merge(
        &self,
        base: &Document,
        doc1: &Document,
        doc2: &Document,
        options: &MergeOptions,
    ) -> Result<MergeResult>;

    /// Try to resolve a conflict
    fn resolve_conflict(&self, conflict: &Conflict, resolution: Resolution) -> Result<Document>;
}

/// Represents a conflict detected during merging
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Unique identifier for the conflict
    pub id: String,
    /// Location of the conflict in the document
    pub location: ConflictLocation,
    /// The conflicting content from doc1
    pub content1: ConflictContent,
    /// The conflicting content from doc2
    pub content2: ConflictContent,
    /// The base content
    pub base_content: ConflictContent,
    /// Conflict type
    pub conflict_type: ConflictType,
}

/// The location of a conflict in the document
#[derive(Debug, Clone)]
pub struct ConflictLocation {
    /// Paragraph index
    pub paragraph_index: usize,
    /// Start character position
    pub start_pos: usize,
    /// End character position
    pub end_pos: usize,
}

/// Content involved in a conflict
#[derive(Debug, Clone)]
pub enum ConflictContent {
    /// Text content
    Text(String),
    /// Paragraph content
    Paragraph(String),
    /// Structural content (table, section, etc.)
    Structure(String),
}

/// Types of conflicts that can occur
#[derive(Debug, Clone)]
pub enum ConflictType {
    /// Both documents modified the same content
    ContentConflict,
    /// One document modified content that the other deleted
    ModifyDeleteConflict,
    /// Both documents added content at the same position but with different values
    InsertInsertConflict,
    /// Formatting conflict
    FormattingConflict,
    /// Structure conflict (paragraphs, tables, etc.)
    StructureConflict,
}

/// Resolution for a conflict
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Use the content from doc1
    UseFirst,
    /// Use the content from doc2
    UseSecond,
    /// Use the content from the base
    UseBase,
    /// Use custom content
    UseCustom(String),
    /// Merge both contents (when possible)
    MergeBoth,
}

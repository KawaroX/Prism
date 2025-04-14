//! Version control system integration for docx documents.

pub mod git;
pub mod history;

use std::path::{Path, PathBuf};

use crate::docx::model::Document;
use crate::error::Result;

/// Repository for versioning documents
pub trait DocumentRepository {
    /// Initialize a new repository
    fn init<P: AsRef<Path>>(&self, path: P) -> Result<Repository>;

    /// Open an existing repository
    fn open<P: AsRef<Path>>(&self, path: P) -> Result<Repository>;

    /// Get the status of files in the repository
    fn status(&self, repo: &Repository) -> Result<RepositoryStatus>;

    /// Add a document to version control
    fn add_document<P: AsRef<Path>>(&self, repo: &Repository, path: P) -> Result<()>;

    /// Commit changes to version control
    fn commit(&self, repo: &Repository, message: &str) -> Result<Commit>;

    /// Get the document at a specific version
    fn get_document(&self, repo: &Repository, version: Option<&str>) -> Result<Document>;

    /// Get history of a document
    fn get_history(&self, repo: &Repository) -> Result<Vec<CommitInfo>>;

    /// Create a new branch
    fn create_branch(&self, repo: &Repository, name: &str) -> Result<Branch>;

    /// Switch to a different branch
    fn switch_branch(&self, repo: &Repository, branch: &Branch) -> Result<()>;

    /// Merge changes from another branch
    fn merge_branch(&self, repo: &Repository, branch: &Branch) -> Result<MergeResult>;
}

/// Represents a document repository
#[derive(Debug, Clone)]
pub struct Repository {
    /// Path to the repository
    pub path: PathBuf,
    /// Current branch
    pub current_branch: Branch,
}

/// Represents a repository branch
#[derive(Debug, Clone)]
pub struct Branch {
    /// Branch name
    pub name: String,
    /// Branch ID
    pub id: String,
}

/// Represents a commit in the repository
#[derive(Debug, Clone)]
pub struct Commit {
    /// Commit ID (hash)
    pub id: String,
    /// Commit message
    pub message: String,
    /// Commit timestamp
    pub timestamp: u64,
    /// Author name
    pub author: String,
}

/// Simplified commit info for display
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Commit ID (hash)
    pub id: String,
    /// Short commit message
    pub short_message: String,
    /// Commit timestamp
    pub timestamp: u64,
    /// Author name
    pub author: String,
}

/// Status of the repository
#[derive(Debug, Clone)]
pub struct RepositoryStatus {
    /// Files that are tracked
    pub tracked_files: Vec<FileStatus>,
    /// Files that are not tracked
    pub untracked_files: Vec<PathBuf>,
}

/// Status of a file in the repository
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// Path to the file
    pub path: PathBuf,
    /// Status code
    pub status: StatusCode,
}

/// Status codes for files
#[derive(Debug, Clone, PartialEq)]
pub enum StatusCode {
    /// File is unmodified
    Unmodified,
    /// File is modified
    Modified,
    /// File is added
    Added,
    /// File is deleted
    Deleted,
    /// File is renamed
    Renamed,
    /// File has conflicts
    Conflicted,
}

/// Result of a merge operation
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Whether the merge was successful
    pub success: bool,
    /// Whether there were conflicts
    pub has_conflicts: bool,
    /// Files that have conflicts
    pub conflict_files: Vec<PathBuf>,
}

impl Repository {
    /// Get the path to the document in the repository
    pub fn document_path(&self) -> PathBuf {
        self.path.join("document.docx")
    }

    /// Get the path to the repository's .git directory
    pub fn git_dir(&self) -> PathBuf {
        self.path.join(".git")
    }
}

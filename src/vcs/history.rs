//! History and version tracking for documents.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::docx::model::Document;
use crate::error::Result;
use crate::vcs::{CommitInfo, DocumentRepository, Repository};

/// Helper for document history operations
pub struct HistoryManager<R: DocumentRepository> {
    /// The document repository
    repository: R,
}

impl<R: DocumentRepository> HistoryManager<R> {
    /// Create a new history manager
    pub fn new(repository: R) -> Self {
        HistoryManager { repository }
    }

    /// Initialize a repository
    pub fn init_repository<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        self.repository.init(path)
    }

    /// Open an existing repository
    pub fn open_repository<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        self.repository.open(path)
    }

    /// Get the document history
    pub fn get_history(&self, repo: &Repository) -> Result<Vec<HistoryEntry>> {
        let history = self.repository.get_history(repo)?;

        let mut entries = Vec::new();
        for commit in history {
            let timestamp = UNIX_EPOCH + Duration::from_secs(commit.timestamp);
            let document = match self.repository.get_document(repo, Some(&commit.id)) {
                Ok(doc) => Some(doc),
                Err(_) => None, // Skip if can't load document for this commit
            };

            entries.push(HistoryEntry {
                commit,
                timestamp,
                document,
            });
        }

        Ok(entries)
    }

    /// Get a specific version of the document
    pub fn get_version(&self, repo: &Repository, version: &str) -> Result<Document> {
        self.repository.get_document(repo, Some(version))
    }

    /// Create a new commit with the current document
    pub fn commit_document<P: AsRef<Path>>(
        &self,
        repo: &Repository,
        document_path: P,
        message: &str,
    ) -> Result<()> {
        self.repository.add_document(repo, document_path)?;
        self.repository.commit(repo, message)?;
        Ok(())
    }
}

/// Entry in the document history
#[derive(Debug)]
pub struct HistoryEntry {
    /// Commit information
    pub commit: CommitInfo,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Document at this point in history (if available)
    pub document: Option<Document>,
}

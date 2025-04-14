//! Git integration for document version control.

use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use git2::{Commit as Git2Commit, ObjectType, Repository as Git2Repository, Signature, Status};
use tempfile::TempDir;

use crate::docx::model::Document;
use crate::docx::parser::DocxParser;
use crate::error::{Error, Result};
use crate::vcs::{
    Branch, Commit, CommitInfo, DocumentRepository, FileStatus, MergeResult, Repository,
    RepositoryStatus, StatusCode,
};

/// Git-based document repository
pub struct GitDocumentRepository;

impl GitDocumentRepository {
    /// Create a new Git document repository
    pub fn new() -> Self {
        GitDocumentRepository
    }

    /// Helper to convert git2::Status to our StatusCode
    fn convert_status(status: Status) -> StatusCode {
        if status.is_index_new() || status.is_wt_new() {
            StatusCode::Added
        } else if status.is_index_modified() || status.is_wt_modified() {
            StatusCode::Modified
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            StatusCode::Deleted
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            StatusCode::Renamed
        } else if status.is_conflicted() {
            StatusCode::Conflicted
        } else {
            StatusCode::Unmodified
        }
    }

    /// Helper to convert git2::Commit to our Commit
    fn convert_commit(commit: &Git2Commit) -> Result<Commit> {
        let id = commit.id().to_string();
        let message = commit.message().unwrap_or_default().to_string();
        let timestamp = commit.time().seconds() as u64;
        let author = commit.author().name().unwrap_or_default().to_string();

        Ok(Commit {
            id,
            message,
            timestamp,
            author,
        })
    }

    /// Helper to convert git2::Commit to our CommitInfo
    fn convert_commit_info(commit: &Git2Commit) -> Result<CommitInfo> {
        let id = commit.id().to_string();
        let message = commit.message().unwrap_or_default().to_string();
        let short_message = message.lines().next().unwrap_or_default().to_string();
        let timestamp = commit.time().seconds() as u64;
        let author = commit.author().name().unwrap_or_default().to_string();

        Ok(CommitInfo {
            id,
            short_message,
            timestamp,
            author,
        })
    }
}

impl DocumentRepository for GitDocumentRepository {
    fn init<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        let path = path.as_ref().to_path_buf();

        // Create the directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        // Initialize a Git repository
        let git_repo = Git2Repository::init(&path)?;

        // Create an initial commit if the repository is empty
        let head = git_repo.head();
        if head.is_err() {
            // Add .gitignore
            let gitignore_path = path.join(".gitignore");
            fs::write(&gitignore_path, "*.tmp\n")?;

            // Create a README
            let readme_path = path.join("README.md");
            fs::write(&readme_path, "# Document Repository\n\nManaged by Prism.\n")?;

            // Add the files
            let mut index = git_repo.index()?;
            index.add_path(Path::new(".gitignore"))?;
            index.add_path(Path::new("README.md"))?;
            index.write()?;

            // Create the initial commit
            let tree_id = index.write_tree()?;
            let tree = git_repo.find_tree(tree_id)?;

            let signature = Signature::now("Prism", "prism@example.com")?;
            git_repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )?;
        }

        // Get the current branch
        let head = git_repo.head()?;
        let branch_name = if head.is_branch() {
            head.shorthand().unwrap_or("master").to_string()
        } else {
            "master".to_string()
        };

        let branch_id = head.target().map(|oid| oid.to_string()).unwrap_or_default();

        Ok(Repository {
            path,
            current_branch: Branch {
                name: branch_name,
                id: branch_id,
            },
        })
    }

    fn open<P: AsRef<Path>>(&self, path: P) -> Result<Repository> {
        let path = path.as_ref().to_path_buf();

        // Verify the path exists and is a directory
        if !path.exists() || !path.is_dir() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Repository path not found",
            )));
        }

        // Open the Git repository
        let git_repo = Git2Repository::open(&path)?;

        // Get the current branch
        let head = git_repo.head()?;
        let branch_name = if head.is_branch() {
            head.shorthand().unwrap_or("master").to_string()
        } else {
            "master".to_string()
        };

        let branch_id = head.target().map(|oid| oid.to_string()).unwrap_or_default();

        Ok(Repository {
            path,
            current_branch: Branch {
                name: branch_name,
                id: branch_id,
            },
        })
    }

    fn status(&self, repo: &Repository) -> Result<RepositoryStatus> {
        let git_repo = Git2Repository::open(&repo.path)?;
        let mut tracked_files = Vec::new();
        let mut untracked_files = Vec::new();

        for entry in git_repo.statuses(None)?.iter() {
            let path = PathBuf::from(entry.path().unwrap_or_default());
            let status = Self::convert_status(entry.status());

            if entry.status().is_wt_new() {
                untracked_files.push(path);
            } else {
                tracked_files.push(FileStatus { path, status });
            }
        }

        Ok(RepositoryStatus {
            tracked_files,
            untracked_files,
        })
    }

    fn add_document<P: AsRef<Path>>(&self, repo: &Repository, path: P) -> Result<()> {
        let git_repo = Git2Repository::open(&repo.path)?;
        let rel_path = path
            .as_ref()
            .strip_prefix(&repo.path)
            .unwrap_or(path.as_ref());

        // Add the file to the index
        let mut index = git_repo.index()?;
        index.add_path(rel_path)?;
        index.write()?;

        Ok(())
    }

    fn commit(&self, repo: &Repository, message: &str) -> Result<Commit> {
        let git_repo = Git2Repository::open(&repo.path)?;

        // Get the index and write the tree
        let mut index = git_repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = git_repo.find_tree(tree_id)?;

        // Get the parent commit(s)
        let head = git_repo.head();
        let mut parents = Vec::new();
        if let Ok(head_ref) = head {
            if let Ok(head_commit) = git_repo.find_commit(head_ref.target().unwrap()) {
                parents.push(head_commit);
            }
        }

        // Create the commit
        let signature = Signature::now("Prism", "prism@example.com")?;
        let parent_refs: Vec<&Git2Commit> = parents.iter().collect();

        let commit_id = git_repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )?;

        // Find the commit we just created
        let commit = git_repo.find_commit(commit_id)?;

        Self::convert_commit(&commit)
    }

    fn get_document(&self, repo: &Repository, version: Option<&str>) -> Result<Document> {
        let git_repo = Git2Repository::open(&repo.path)?;
        let document_path = repo.document_path();
        let rel_path = document_path
            .strip_prefix(&repo.path)
            .unwrap_or(&document_path);

        // If no version specified, just read the file from disk
        if version.is_none() && document_path.exists() {
            return DocxParser::parse_file(&document_path);
        }

        // Get the specified commit, or HEAD if none specified
        let obj = match version {
            Some(version) => git_repo.revparse_single(version)?,
            None => git_repo.head()?.peel(ObjectType::Commit)?,
        };

        // Get the commit
        let commit = obj
            .as_commit()
            .ok_or_else(|| Error::Unsupported("Specified version is not a commit".to_string()))?;

        // Get the tree for this commit
        let tree = commit.tree()?;

        // Try to find the document in the tree
        let entry = tree.get_path(rel_path)?;
        let blob = entry.to_object(&git_repo)?.peel(ObjectType::Blob)?;
        let blob = blob
            .as_blob()
            .ok_or_else(|| Error::Unsupported("Document is not a blob".to_string()))?;

        // Create a temporary file with the blob contents
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().join("document.docx");
        fs::write(&temp_path, blob.content())?;

        // Parse the document
        DocxParser::parse_file(&temp_path)
    }

    fn get_history(&self, repo: &Repository) -> Result<Vec<CommitInfo>> {
        let git_repo = Git2Repository::open(&repo.path)?;
        let mut revwalk = git_repo.revwalk()?;
        revwalk.push_head()?;

        let mut history = Vec::new();

        for oid in revwalk {
            let commit = git_repo.find_commit(oid?)?;
            history.push(Self::convert_commit_info(&commit)?);
        }

        Ok(history)
    }

    fn create_branch(&self, repo: &Repository, name: &str) -> Result<Branch> {
        let git_repo = Git2Repository::open(&repo.path)?;

        // Get the HEAD commit
        let head = git_repo.head()?;
        let head_commit = git_repo.find_commit(head.target().unwrap())?;

        // Create the branch
        let git_branch = git_repo.branch(name, &head_commit, false)?;
        let branch_id = git_branch.get().target().unwrap().to_string();

        Ok(Branch {
            name: name.to_string(),
            id: branch_id,
        })
    }

    fn switch_branch(&self, repo: &Repository, branch: &Branch) -> Result<()> {
        let git_repo = Git2Repository::open(&repo.path)?;

        // Find the branch
        let git_branch = git_repo.find_branch(&branch.name, git2::BranchType::Local)?;

        // Checkout the branch
        let obj =
            git_repo.find_object(git2::Oid::from_str(&branch.id)?, Some(ObjectType::Commit))?;

        git_repo.checkout_tree(&obj, None)?;
        git_repo.set_head(git_branch.get().name().unwrap())?;

        Ok(())
    }

    fn merge_branch(&self, repo: &Repository, branch: &Branch) -> Result<MergeResult> {
        let git_repo = Git2Repository::open(&repo.path)?;

        // Find the branch to merge
        let git_branch = git_repo.find_branch(&branch.name, git2::BranchType::Local)?;
        let branch_commit = git_repo.find_commit(git_branch.get().target().unwrap())?;

        // Perform the merge
        let head = git_repo.head()?;
        let head_commit = git_repo.find_commit(head.target().unwrap())?;

        let ancestor =
            git_repo.find_commit(git_repo.merge_base(head_commit.id(), branch_commit.id())?)?;

        let mut index = git_repo.merge_trees(
            &ancestor.tree()?,
            &head_commit.tree()?,
            &branch_commit.tree()?,
            None,
        )?;

        // Check for conflicts
        let has_conflicts = index.has_conflicts();

        let mut conflict_files = Vec::new();
        if has_conflicts {
            // 修复冲突迭代方式
            let mut conflicts = index.conflicts()?;
            while let Some(conflict_entry) = conflicts.next() {
                if let Ok(conflict) = conflict_entry {
                    if let Some(our) = conflict.our {
                        if let Ok(path) = std::str::from_utf8(our.path.as_ref()) {
                            conflict_files.push(PathBuf::from(path));
                        }
                    } else if let Some(their) = conflict.their {
                        if let Ok(path) = std::str::from_utf8(their.path.as_ref()) {
                            conflict_files.push(PathBuf::from(path));
                        }
                    }
                }
            }
        }

        // If there are no conflicts, commit the merge
        let success = !has_conflicts;
        if success {
            let tree_id = index.write_tree()?;
            let tree = git_repo.find_tree(tree_id)?;

            let signature = Signature::now("Prism", "prism@example.com")?;
            git_repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("Merge branch '{}'", branch.name),
                &tree,
                &[&head_commit, &branch_commit],
            )?;
        }

        Ok(MergeResult {
            success,
            has_conflicts,
            conflict_files,
        })
    }
}

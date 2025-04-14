use prism::DocxBuilder;
use prism::docx::{Document, Paragraph, Run, RunContent};
use prism::error::Result;
use prism::vcs::DocumentRepository;
use prism::vcs::git::GitDocumentRepository;
use prism::vcs::history::HistoryManager;

use tempfile::TempDir;

#[test]
fn test_git_repository() -> Result<()> {
    // 创建临时目录
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();

    // 初始化仓库
    let git_repo = GitDocumentRepository::new();
    let repo = git_repo.init(repo_path)?;

    // 检查仓库是否创建
    assert!(
        repo_path.join(".git").exists(),
        "Git repository was not created"
    );

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

    // 保存文档到仓库
    let doc_path = repo_path.join("document.docx");
    DocxBuilder::build(&document, &doc_path)?;

    // 添加并提交文档
    git_repo.add_document(&repo, &doc_path)?;
    let commit = git_repo.commit(&repo, "Add initial document")?;

    // 验证提交
    assert!(!commit.id.is_empty(), "Commit ID should not be empty");
    assert_eq!(
        commit.message, "Add initial document",
        "Commit message mismatch"
    );

    // 修改文档
    let mut modified_document = document.clone();
    let modified_paragraph = Paragraph {
        id: None,
        style_id: None,
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(
                "This document has been modified.".to_string(),
            )],
        }],
    };
    modified_document.body.paragraphs.push(modified_paragraph);

    // 保存修改的文档
    DocxBuilder::build(&modified_document, &doc_path)?;

    // 提交更改
    git_repo.add_document(&repo, &doc_path)?;
    let second_commit = git_repo.commit(&repo, "Update document")?;

    // 验证第二个提交
    assert!(
        !second_commit.id.is_empty(),
        "Second commit ID should not be empty"
    );
    assert_eq!(
        second_commit.message, "Update document",
        "Second commit message mismatch"
    );

    // 获取文档历史 - 修复：创建新的 GitDocumentRepository 实例
    let history_repo = GitDocumentRepository::new();
    let history_manager = HistoryManager::new(history_repo);
    let history = history_manager.get_history(&repo)?;

    // 验证历史
    assert_eq!(
        history.len(),
        3,
        "History should have 3 entries (initial + 2 commits)"
    );

    // 获取原始版本的文档
    let original_doc = git_repo.get_document(&repo, Some(&commit.id))?;
    assert_eq!(
        original_doc.body.paragraphs.len(),
        1,
        "Original document should have 1 paragraph"
    );

    // 获取最新版本的文档
    let latest_doc = git_repo.get_document(&repo, None)?;
    assert_eq!(
        latest_doc.body.paragraphs.len(),
        2,
        "Latest document should have 2 paragraphs"
    );

    Ok(())
}

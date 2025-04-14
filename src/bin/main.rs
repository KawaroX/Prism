use std::fs;
use std::path::PathBuf;
use std::process;

use prism::DocumentRepository;
use prism::FormatConverter;
use prism::convert::{ConversionOptions, StandardConverter};
use prism::diff::DocumentDiffer;
use prism::docx::DocxParser;
use prism::docx::builder::DocxBuilder;
use prism::vcs::git::GitDocumentRepository;
use prism::vcs::history::HistoryManager;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    match args[1].as_str() {
        "parse" => {
            if args.len() < 3 {
                eprintln!("Error: Missing docx file path");
                print_usage(&args[0]);
                process::exit(1);
            }
            parse_command(&args[2]);
        }
        "diff" => {
            if args.len() < 4 {
                eprintln!("Error: Missing docx file paths");
                print_usage(&args[0]);
                process::exit(1);
            }
            diff_command(&args[2], &args[3]);
        }
        "merge" => {
            if args.len() < 5 {
                eprintln!("Error: Missing docx file paths");
                print_usage(&args[0]);
                process::exit(1);
            }
            merge_command(&args[2], &args[3], &args[4]);
        }
        "init" => {
            if args.len() < 3 {
                eprintln!("Error: Missing repository path");
                print_usage(&args[0]);
                process::exit(1);
            }
            init_command(&args[2]);
        }
        "add" => {
            if args.len() < 4 {
                eprintln!("Error: Missing repository path and document path");
                print_usage(&args[0]);
                process::exit(1);
            }
            add_command(&args[2], &args[3]);
        }
        "commit" => {
            if args.len() < 4 {
                eprintln!("Error: Missing repository path and commit message");
                print_usage(&args[0]);
                process::exit(1);
            }
            commit_command(&args[2], &args[3]);
        }
        "history" => {
            if args.len() < 3 {
                eprintln!("Error: Missing repository path");
                print_usage(&args[0]);
                process::exit(1);
            }
            history_command(&args[2]);
        }
        "show" => {
            if args.len() < 4 {
                eprintln!("Error: Missing repository path and version");
                print_usage(&args[0]);
                process::exit(1);
            }
            show_command(&args[2], &args[3]);
        }
        "convert" => {
            if args.len() < 4 {
                eprintln!("Error: Missing input and output file paths");
                print_usage(&args[0]);
                process::exit(1);
            }
            convert_command(&args[2], &args[3]);
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", args[1]);
            print_usage(&args[0]);
            process::exit(1);
        }
    }
}

fn print_usage(program_name: &str) {
    eprintln!("Usage:");
    eprintln!("  {} parse <docx_file>", program_name);
    eprintln!("  {} diff <docx_file1> <docx_file2>", program_name);
    eprintln!(
        "  {} merge <base_docx> <docx_file1> <docx_file2>",
        program_name
    );
    eprintln!("  {} init <repo_path>", program_name);
    eprintln!("  {} add <repo_path> <docx_file>", program_name);
    eprintln!("  {} commit <repo_path> <message>", program_name);
    eprintln!("  {} history <repo_path>", program_name);
    eprintln!("  {} show <repo_path> <version>", program_name);
    eprintln!("  {} convert <input_file> <output_file>", program_name);
}

fn parse_command(file_path: &str) {
    let path = PathBuf::from(file_path);

    if !path.exists() {
        eprintln!("File not found: {:?}", path);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&path) {
        eprintln!("Not a docx file: {:?}", path);
        process::exit(1);
    }

    match DocxParser::parse_file(&path) {
        Ok(document) => {
            println!("Successfully parsed document: {:?}", path);
            println!(
                "Document contains {} paragraphs",
                document.body.paragraphs.len()
            );

            // 打印文档内容摘要
            println!("\nDocument content summary:");
            for (i, paragraph) in document.body.paragraphs.iter().enumerate().take(5) {
                print!("Paragraph {}: ", i + 1);

                for run in &paragraph.runs {
                    for content in &run.contents {
                        if let prism::docx::model::RunContent::Text(text) = content {
                            print!("{}", text);
                        }
                    }
                }

                println!();
            }

            if document.body.paragraphs.len() > 5 {
                println!(
                    "... and {} more paragraphs",
                    document.body.paragraphs.len() - 5
                );
            }
        }
        Err(err) => {
            eprintln!("Error parsing document: {}", err);
            process::exit(1);
        }
    }
}

fn diff_command(file_path1: &str, file_path2: &str) {
    let path1 = PathBuf::from(file_path1);
    let path2 = PathBuf::from(file_path2);

    if !path1.exists() {
        eprintln!("File not found: {:?}", path1);
        process::exit(1);
    }

    if !path2.exists() {
        eprintln!("File not found: {:?}", path2);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&path1) {
        eprintln!("Not a docx file: {:?}", path1);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&path2) {
        eprintln!("Not a docx file: {:?}", path2);
        process::exit(1);
    }

    match (
        DocxParser::parse_file(&path1),
        DocxParser::parse_file(&path2),
    ) {
        (Ok(doc1), Ok(doc2)) => {
            println!("Comparing documents:");
            println!("  - {:?}", path1);
            println!("  - {:?}", path2);

            let differ = prism::diff::DocxDiffer::new();
            let options = prism::diff::DiffOptions::default();

            match differ.diff(&doc1, &doc2, &options) {
                Ok(diff) => {
                    println!("\nFound {} differences", diff.operations.len());

                    let renderer = prism::diff::DiffRenderer::new();
                    match renderer.render_as_text(&diff) {
                        Ok(text) => {
                            println!("\nDifferences:");
                            println!("{}", text);
                        }
                        Err(err) => {
                            eprintln!("Error rendering diff: {}", err);
                            process::exit(1);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error diffing documents: {}", err);
                    process::exit(1);
                }
            }
        }
        (Err(err), _) => {
            eprintln!("Error parsing first document: {}", err);
            process::exit(1);
        }
        (_, Err(err)) => {
            eprintln!("Error parsing second document: {}", err);
            process::exit(1);
        }
    }
}

fn merge_command(base_path: &str, file_path1: &str, file_path2: &str) {
    let path_base = PathBuf::from(base_path);
    let path1 = PathBuf::from(file_path1);
    let path2 = PathBuf::from(file_path2);

    if !path_base.exists() {
        eprintln!("File not found: {:?}", path_base);
        process::exit(1);
    }

    if !path1.exists() {
        eprintln!("File not found: {:?}", path1);
        process::exit(1);
    }

    if !path2.exists() {
        eprintln!("File not found: {:?}", path2);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&path_base) {
        eprintln!("Not a docx file: {:?}", path_base);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&path1) {
        eprintln!("Not a docx file: {:?}", path1);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&path2) {
        eprintln!("Not a docx file: {:?}", path2);
        process::exit(1);
    }

    // 创建合并引擎
    let merger = prism::merge::strategy::ThreeWayMerger::new();

    println!("Merging documents:");
    println!("  - Base: {:?}", path_base);
    println!("  - Doc1: {:?}", path1);
    println!("  - Doc2: {:?}", path2);

    // 使用简化的合并方法
    // 注意：这假设我们已经实现了之前讨论的改进API
    match merger.merge_documents(
        base_path,
        file_path1,
        file_path2,
        Some(prism::merge::MergeOptions {
            auto_resolve: true,
            favor_base: false,
            track_revisions: true,
            insert_conflict_handling: prism::merge::InsertConflictHandling::KeepBoth,
            preserve_conflicts: true,
        }),
        Some("merged_document.docx"),
    ) {
        Ok((doc, conflicts)) => {
            if !conflicts.is_empty() {
                println!("\nMerge completed with {} conflicts:", conflicts.len());
                for (i, conflict) in conflicts.iter().enumerate() {
                    println!(
                        "  Conflict #{}: {:?} at paragraph {}",
                        i + 1,
                        conflict.conflict_type,
                        conflict.location.paragraph_index + 1
                    );
                }
                println!("\nConflicts are marked in the merged document.");
            } else {
                println!("\nMerge completed successfully without conflicts.");
            }

            println!("Merged document saved as: merged_document.docx");
            println!(
                "Merged document has {} paragraphs",
                doc.body.paragraphs.len()
            );
        }
        Err(err) => {
            eprintln!("Error merging documents: {}", err);
            process::exit(1);
        }
    }
}

fn init_command(repo_path: &str) {
    let path = PathBuf::from(repo_path);

    let git_repo = GitDocumentRepository::new();
    match git_repo.init(&path) {
        Ok(repo) => {
            println!("Initialized empty document repository at: {:?}", path);
            println!("Current branch: {}", repo.current_branch.name);
        }
        Err(err) => {
            eprintln!("Error initializing repository: {}", err);
            process::exit(1);
        }
    }
}

fn add_command(repo_path: &str, doc_path: &str) {
    let repo_path = PathBuf::from(repo_path);
    let doc_path = PathBuf::from(doc_path);

    if !doc_path.exists() {
        eprintln!("Document not found: {:?}", doc_path);
        process::exit(1);
    }

    if !prism::utils::is_docx_file(&doc_path) {
        eprintln!("Not a docx file: {:?}", doc_path);
        process::exit(1);
    }

    let git_repo = GitDocumentRepository::new();
    match git_repo.open(&repo_path) {
        Ok(repo) => match git_repo.add_document(&repo, &doc_path) {
            Ok(_) => {
                println!("Added document to repository: {:?}", doc_path);
            }
            Err(err) => {
                eprintln!("Error adding document: {}", err);
                process::exit(1);
            }
        },
        Err(err) => {
            eprintln!("Error opening repository: {}", err);
            process::exit(1);
        }
    }
}

fn commit_command(repo_path: &str, message: &str) {
    let repo_path = PathBuf::from(repo_path);

    let git_repo = GitDocumentRepository::new();
    match git_repo.open(&repo_path) {
        Ok(repo) => match git_repo.commit(&repo, message) {
            Ok(commit) => {
                println!("Created commit: {}", commit.id);
                println!("Message: {}", commit.message);
            }
            Err(err) => {
                eprintln!("Error creating commit: {}", err);
                process::exit(1);
            }
        },
        Err(err) => {
            eprintln!("Error opening repository: {}", err);
            process::exit(1);
        }
    }
}

fn history_command(repo_path: &str) {
    let repo_path = PathBuf::from(repo_path);

    let git_repo = GitDocumentRepository::new();
    let history_manager = HistoryManager::new(git_repo);

    match history_manager.open_repository(&repo_path) {
        Ok(repo) => match history_manager.get_history(&repo) {
            Ok(history) => {
                println!("Document history ({} versions):", history.len());
                for (i, entry) in history.iter().enumerate() {
                    let timestamp = entry
                        .timestamp
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    let time_str =
                        chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "unknown time".to_string());

                    println!(
                        "{}: {} - {} ({})",
                        i + 1,
                        &entry.commit.id[..8],
                        entry.commit.short_message,
                        time_str
                    );
                }
            }
            Err(err) => {
                eprintln!("Error retrieving history: {}", err);
                process::exit(1);
            }
        },
        Err(err) => {
            eprintln!("Error opening repository: {}", err);
            process::exit(1);
        }
    }
}

fn show_command(repo_path: &str, version: &str) {
    let repo_path = PathBuf::from(repo_path);

    let git_repo = GitDocumentRepository::new();

    match git_repo.open(&repo_path) {
        Ok(repo) => {
            match git_repo.get_document(&repo, Some(version)) {
                Ok(document) => {
                    println!("Document version: {}", version);
                    println!("Number of paragraphs: {}", document.body.paragraphs.len());

                    // Print a summary of the document
                    println!("\nDocument content summary:");
                    for (i, paragraph) in document.body.paragraphs.iter().enumerate().take(5) {
                        print!("Paragraph {}: ", i + 1);

                        for run in &paragraph.runs {
                            for content in &run.contents {
                                if let prism::docx::model::RunContent::Text(text) = content {
                                    print!("{}", text);
                                }
                            }
                        }

                        println!();
                    }

                    if document.body.paragraphs.len() > 5 {
                        println!(
                            "... and {} more paragraphs",
                            document.body.paragraphs.len() - 5
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Error retrieving document: {}", err);
                    process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("Error opening repository: {}", err);
            process::exit(1);
        }
    }
}

fn convert_command(input_path: &str, output_path: &str) {
    let input = PathBuf::from(input_path);
    let output = PathBuf::from(output_path);

    if !input.exists() {
        eprintln!("Input file not found: {:?}", input);
        process::exit(1);
    }

    // 检查文件类型
    let input_is_docx = prism::utils::is_docx_file(&input);
    let output_is_docx = output.extension().map_or(false, |ext| ext == "docx");

    if input_is_docx && !output_is_docx {
        // docx -> md 转换
        docx_to_md_command(&input, &output);
    } else if !input_is_docx && output_is_docx {
        // md -> docx 转换
        md_to_docx_command(&input, &output);
    } else {
        eprintln!(
            "Unsupported conversion: {:?} -> {:?}",
            input.extension().unwrap_or_default(),
            output.extension().unwrap_or_default()
        );
        process::exit(1);
    }
}

fn docx_to_md_command(input: &PathBuf, output: &PathBuf) {
    // docx -> md 转换
    match DocxParser::parse_file(input) {
        Ok(document) => {
            let converter = StandardConverter::new();
            let options = ConversionOptions::default();

            match converter.docx_to_markdown(&document, &options) {
                Ok(markdown) => {
                    if let Err(err) = fs::write(output, markdown) {
                        eprintln!("Error writing output file: {}", err);
                        process::exit(1);
                    }
                    println!("Successfully converted {:?} to {:?}", input, output);
                }
                Err(err) => {
                    eprintln!("Error converting to Markdown: {}", err);
                    process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("Error parsing docx file: {}", err);
            process::exit(1);
        }
    }
}

fn md_to_docx_command(input: &PathBuf, output: &PathBuf) {
    // md -> docx 转换
    match fs::read_to_string(input) {
        Ok(markdown) => {
            let converter = StandardConverter::new();
            let options = ConversionOptions::default();

            match converter.markdown_to_docx(&markdown, None, &options) {
                Ok(document) => {
                    if let Err(err) = DocxBuilder::build(&document, output) {
                        eprintln!("Error building docx file: {}", err);
                        process::exit(1);
                    }
                    println!("Successfully converted {:?} to {:?}", input, output);
                }
                Err(err) => {
                    eprintln!("Error converting to docx: {}", err);
                    process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("Error reading Markdown file: {}", err);
            process::exit(1);
        }
    }
}

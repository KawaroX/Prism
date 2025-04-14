# Prism: Version Control for Word Documents

Prism is a specialized version control system designed for Microsoft Word (.docx) documents, with a focus on academic writing workflows. It aims to provide Git-like versioning capabilities for documents while being accessible to users who may not be familiar with traditional code-based version control systems.

## Features

- **Document Parsing and Analysis**: Extract content and structure from .docx files
- **Difference Detection**: Compare document versions and visualize changes
- **Three-Way Merging**: Merge changes from multiple contributors with conflict resolution
- **Version Control**: Git-based document history tracking and management
- **Format Conversion**: Convert between .docx and Markdown formats

## Why Prism?

Traditional version control systems like Git are powerful but have a steep learning curve, especially for non-technical users in fields like humanities. Most academic writers use inefficient methods like emailing document versions or relying on track changes, leading to confusion and lost work.

Prism bridges this gap with:

- Focus on the document formats academics actually use (.docx)
- Simplified workflows for common document collaboration tasks
- Intelligent conflict detection and resolution for text documents
- Preservation of document formatting and structure
- Tools to visualize changes between versions

## Getting Started

### Installation

```bash
# Coming soon
cargo install prism
```

### Basic Commands

```bash
# Initialize a new document repository
prism init my-paper

# Add a document to version control
prism add my-paper document.docx

# Commit changes
prism commit my-paper "Added introduction section"

# View document history
prism history my-paper

# Compare two versions
prism diff old_version.docx new_version.docx

# Merge documents
prism merge base.docx variant1.docx variant2.docx
```

## Core Components

- **DocxParser**: Extract content from .docx files
- **DocxBuilder**: Create .docx files programmatically
- **DocxDiffer**: Compare documents and identify changes
- **ThreeWayMerger**: Merge changes from multiple document versions
- **GitDocumentRepository**: Store document history using Git
- **FormatConverter**: Convert between .docx and other formats

## Academic Use Case

Prism shines in academic collaboration scenarios:

1. A professor and multiple students work on a research paper
2. Each contributor edits their own copy of the document
3. Prism helps merge their changes and resolve conflicts
4. The version history is preserved for reference and attribution

## Format Conversion

Prism can convert between .docx and Markdown:

```bash
# Convert docx to Markdown
prism convert paper.docx paper.md

# Convert Markdown to docx
prism convert paper.md paper.docx
```

## Development Status

Prism is currently in early development. The core functionality is being implemented, but the API and CLI interface may change.

## Built With

- **Rust**: For performance and reliability
- **zip crate**: For working with .docx files (which are ZIP archives)
- **quick-xml/roxmltree**: For parsing and manipulating XML within .docx files
- **similar/diffy**: For difference detection and comparison
- **git2**: For Git integration
- **tokio**: For asynchronous operations
- **pulldown-cmark**: For Markdown parsing

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

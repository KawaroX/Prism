//! Utility functions for the Prism library.

use std::path::Path;

/// Check if a file has a .docx extension
pub fn is_docx_file<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("docx"))
        .unwrap_or(false)
}

/// Get a temporary directory for working with docx files
pub fn get_temp_dir() -> std::io::Result<tempfile::TempDir> {
    tempfile::Builder::new()
        .prefix("prism_")
        .tempdir()
}
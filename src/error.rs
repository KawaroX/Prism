use std::io;
use thiserror::Error;

/// Custom error types for the Prism library
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("XML parsing error: {0}")]
    XmlTree(#[from] roxmltree::Error),

    #[error("Document parsing error: {0}")]
    DocxParse(String),

    #[error("Git operation error: {0}")]
    Git(#[from] git2::Error),

    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}

/// Result type alias for Prism operations
pub type Result<T> = std::result::Result<T, Error>;
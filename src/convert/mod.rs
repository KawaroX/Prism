//! Format conversion between docx and Markdown.

pub mod from_markdown;
pub mod template;
pub mod to_markdown;

use crate::docx::model::Document;
use crate::error::Result;

pub use self::to_markdown::DocxToMarkdownConverter;

/// Options for controlling the conversion process
#[derive(Debug, Clone)]
pub struct ConversionOptions {
    /// Whether to preserve images
    pub preserve_images: bool,
    /// Whether to handle tables
    pub handle_tables: bool,
    /// Whether to handle links
    pub handle_links: bool,
    /// Whether to handle footnotes
    pub handle_footnotes: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        ConversionOptions {
            preserve_images: true,
            handle_tables: true,
            handle_links: true,
            handle_footnotes: true,
        }
    }
}

/// Interface for format conversion
pub trait FormatConverter {
    /// Convert docx to Markdown
    fn docx_to_markdown(&self, document: &Document, options: &ConversionOptions) -> Result<String>;

    /// Convert Markdown to docx
    fn markdown_to_docx(
        &self,
        markdown: &str,
        template: Option<&Document>,
        options: &ConversionOptions,
    ) -> Result<Document>;
}

/// Standard format converter
#[derive(Debug, Clone)]
pub struct StandardConverter;

impl StandardConverter {
    /// Create a new standard converter
    pub fn new() -> Self {
        StandardConverter
    }
}

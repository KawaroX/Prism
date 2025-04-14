//! Template management for document conversion.

use std::fs;
use std::path::{Path, PathBuf};

use crate::docx::model::Document;
use crate::docx::parser::DocxParser;
use crate::error::{Error, Result};

/// Template manager for storing and retrieving document templates
pub struct TemplateManager {
    /// Directory where templates are stored
    template_dir: PathBuf,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new<P: AsRef<Path>>(template_dir: P) -> Result<Self> {
        let dir = template_dir.as_ref().to_path_buf();

        // 确保目录存在
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(TemplateManager { template_dir: dir })
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Result<Document> {
        let path = self.template_dir.join(format!("{}.docx", name));

        if !path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Template '{}' not found", name),
            )));
        }

        DocxParser::parse_file(path)
    }

    /// Save a document as a template
    pub fn save_template(&self, document: &Document, name: &str) -> Result<()> {
        let path = self.template_dir.join(format!("{}.docx", name));

        // 使用DocxBuilder保存文档
        crate::docx::builder::DocxBuilder::build(document, path)?;

        Ok(())
    }

    /// Get the default template
    pub fn get_default_template(&self) -> Result<Document> {
        // 尝试加载默认模板
        if let Ok(template) = self.get_template("default") {
            return Ok(template);
        }

        // 如果没有默认模板，创建一个基本模板
        let mut document = Document::new();

        // 添加一些基本样式
        // 在实际应用中，我们应该添加更多的样式信息
        // 但这里为了简单起见，我们只使用空文档

        Ok(document)
    }

    /// List all available templates
    pub fn list_templates(&self) -> Result<Vec<String>> {
        let mut templates = Vec::new();

        for entry in fs::read_dir(&self.template_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "docx") {
                if let Some(filename) = path.file_stem() {
                    if let Some(name) = filename.to_str() {
                        templates.push(name.to_string());
                    }
                }
            }
        }

        Ok(templates)
    }
}

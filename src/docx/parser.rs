use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

use roxmltree::{Document as XmlDocument, Node};
use zip::ZipArchive;

use crate::docx::model::*;
use crate::error::{Error, Result};

/// The main parser for docx files
pub struct DocxParser;

impl DocxParser {
    /// Parse a docx file from a path
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Document> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader)
    }

    /// Parse a docx file from a reader
    pub fn parse_reader<R: Read + Seek>(reader: R) -> Result<Document> {
        let mut archive = ZipArchive::new(reader)?;
        
        // Initialize a new document
        let mut document = Document::new();
        
        // Parse the main document
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/document.xml") {
            Self::parse_document_xml(&content, &mut document)?;
        } else {
            return Err(Error::DocxParse("Missing document.xml".to_string()));
        }
        
        // Parse styles (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/styles.xml") {
            Self::parse_styles_xml(&content, &mut document)?;
        }
        
        // Parse numbering (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/numbering.xml") {
            Self::parse_numbering_xml(&content, &mut document)?;
        }
        
        // Parse relationships (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/_rels/document.xml.rels") {
            Self::parse_relationships_xml(&content, &mut document)?;
        }
        
        // Parse footnotes (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/footnotes.xml") {
            Self::parse_footnotes_xml(&content, &mut document)?;
        }
        
        // Parse endnotes (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/endnotes.xml") {
            Self::parse_endnotes_xml(&content, &mut document)?;
        }
        
        // Parse comments (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "word/comments.xml") {
            Self::parse_comments_xml(&content, &mut document)?;
        }
        
        // Parse document properties (optional)
        if let Ok(content) = Self::extract_file_content(&mut archive, "docProps/core.xml") {
            Self::parse_core_properties_xml(&content, &mut document)?;
        }
        
        Ok(document)
    }

    // Helper function to extract a file's content from the zip archive
    fn extract_file_content<R: Read + Seek>(
        archive: &mut ZipArchive<R>,
        path: &str,
    ) -> Result<String> {
        let mut file = archive.by_name(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }

    // Parse the main document.xml file
    fn parse_document_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        // Find the body element
        if let Some(body_node) = root.children().find(|n| n.has_tag_name("body")) {
            Self::parse_body(body_node, document)?;
        } else {
            return Err(Error::DocxParse("Missing body element".to_string()));
        }
        
        Ok(())
    }

    // Parse the body element of document.xml
    fn parse_body(body_node: Node, document: &mut Document) -> Result<()> {
        for child in body_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "p" => {
                        let paragraph = Self::parse_paragraph(child)?;
                        document.body.paragraphs.push(paragraph);
                    }
                    "tbl" => {
                        let table = Self::parse_table(child)?;
                        document.body.tables.push(table);
                    }
                    "sectPr" => {
                        let section = Self::parse_section(child)?;
                        document.body.sections.push(section);
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        Ok(())
    }

    // Parse a paragraph element
    fn parse_paragraph(p_node: Node) -> Result<Paragraph> {
        let mut paragraph = Paragraph {
            id: None,
            style_id: None,
            properties: ParagraphProperties::default(),
            runs: vec![],
        };
        
        // Extract paragraph ID if present
        if let Some(para_id) = p_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "paraId")) {
            paragraph.id = Some(para_id.to_string());
        }
        
        for child in p_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "pPr" => {
                        // Parse paragraph properties
                        Self::parse_paragraph_properties(child, &mut paragraph)?;
                    }
                    "r" => {
                        // Parse runs
                        if let Ok(run) = Self::parse_run(child) {
                            paragraph.runs.push(run);
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(paragraph)
    }

    // Parse paragraph properties
    fn parse_paragraph_properties(pPr_node: Node, paragraph: &mut Paragraph) -> Result<()> {
        for child in pPr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "pStyle" => {
                        if let Some(style_id) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            paragraph.style_id = Some(style_id.to_string());
                        }
                    }
                    "jc" => {
                        if let Some(alignment) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            paragraph.properties.alignment = Some(alignment.to_string());
                        }
                    }
                    "ind" => {
                        let left = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "left"))
                            .and_then(|v| v.parse::<i32>().ok());
                        let right = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "right"))
                            .and_then(|v| v.parse::<i32>().ok());
                        let first_line = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "firstLine"))
                            .and_then(|v| v.parse::<i32>().ok());
                        let hanging = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "hanging"))
                            .and_then(|v| v.parse::<i32>().ok());
                        
                        paragraph.properties.indentation = Some(Indentation {
                            left,
                            right,
                            first_line,
                            hanging,
                        });
                    }
                    "numPr" => {
                        let mut numbering_id = None;
                        let mut level = None;
                        
                        for num_child in child.children() {
                            if num_child.is_element() {
                                match num_child.tag_name().name() {
                                    "numId" => {
                                        numbering_id = num_child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val"))
                                            .map(|v| v.to_string());
                                    }
                                    "ilvl" => {
                                        level = num_child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val"))
                                            .and_then(|v| v.parse::<usize>().ok());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        
                        if let (Some(id), Some(lvl)) = (numbering_id, level) {
                            paragraph.properties.numbering = Some(NumberingProperties {
                                id,
                                level: lvl,
                            });
                        }
                    }
                    "spacing" => {
                        let line = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "line"))
                            .and_then(|v| v.parse::<u32>().ok());
                        let line_rule = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "lineRule"))
                            .map(|v| v.to_string());
                        let before = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "before"))
                            .and_then(|v| v.parse::<u32>().ok());
                        let after = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "after"))
                            .and_then(|v| v.parse::<u32>().ok());
                        
                        paragraph.properties.spacing = Some(Spacing {
                            line,
                            line_rule,
                            before,
                            after,
                        });
                    }
                    _ => {
                        // Store other properties as key-value pairs
                        let name = child.tag_name().name().to_string();
                        if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            paragraph.properties.other_properties.insert(name, val.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    // Parse a run element
    fn parse_run(r_node: Node) -> Result<Run> {
        let mut run = Run {
            properties: RunProperties::default(),
            contents: vec![],
        };
        
        for child in r_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "rPr" => {
                        // Parse run properties
                        Self::parse_run_properties(child, &mut run)?;
                    }
                    "t" => {
                        // Parse text content
                        let text = child.text().unwrap_or_default().to_string();
                        run.contents.push(RunContent::Text(text));
                    }
                    "br" => {
                        run.contents.push(RunContent::Break);
                    }
                    "tab" => {
                        run.contents.push(RunContent::Tab);
                    }
                    "drawing" => {
                        if let Ok(drawing) = Self::parse_drawing(child) {
                            run.contents.push(RunContent::Drawing(drawing));
                        }
                    }
                    "footnoteReference" => {
                        if let Some(id) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "id")) {
                            run.contents.push(RunContent::FootnoteReference(id.to_string()));
                        }
                    }
                    "sym" => {
                        if let Some(char) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "char")) {
                            run.contents.push(RunContent::Symbol(char.to_string()));
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(run)
    }

    // Parse run properties
    fn parse_run_properties(rPr_node: Node, run: &mut Run) -> Result<()> {
        for child in rPr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "b" => {
                        run.properties.bold = true;
                    }
                    "i" => {
                        run.properties.italic = true;
                    }
                    "u" => {
                        run.properties.underline = true;
                    }
                    "sz" => {
                        if let Some(size) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            run.properties.size = size.parse::<u32>().ok();
                        }
                    }
                    "rFonts" => {
                        if let Some(font) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "ascii")) {
                            run.properties.font = Some(font.to_string());
                        } else if let Some(font) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "hAnsi")) {
                            run.properties.font = Some(font.to_string());
                        }
                    }
                    "color" => {
                        if let Some(color) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            run.properties.color = Some(color.to_string());
                        }
                    }
                    _ => {
                        // Store other properties as key-value pairs
                        let name = child.tag_name().name().to_string();
                        if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            run.properties.other_properties.insert(name, val.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    // Parse a drawing element
    fn parse_drawing(drawing_node: Node) -> Result<Drawing> {
        let mut drawing = Drawing {
            id: "".to_string(),
            description: None,
            name: None,
        };
        
        // This is a simplified implementation - real drawing parsing is more complex
        for child in drawing_node.descendants() {
            if child.is_element() {
                match child.tag_name().name() {
                    "docPr" => {
                        if let Some(id) = child.attribute(("http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing", "id")) {
                            drawing.id = id.to_string();
                        }
                        if let Some(name) = child.attribute(("http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing", "name")) {
                            drawing.name = Some(name.to_string());
                        }
                        if let Some(descr) = child.attribute(("http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing", "descr")) {
                            drawing.description = Some(descr.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(drawing)
    }

    // Parse a table element
    fn parse_table(tbl_node: Node) -> Result<Table> {
        let mut table = Table {
            properties: TableProperties::default(),
            rows: vec![],
        };
        
        for child in tbl_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "tblPr" => {
                        Self::parse_table_properties(child, &mut table)?;
                    }
                    "tr" => {
                        if let Ok(row) = Self::parse_table_row(child) {
                            table.rows.push(row);
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(table)
    }

    // Parse table properties
    fn parse_table_properties(tblPr_node: Node, table: &mut Table) -> Result<()> {
        for child in tblPr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "tblStyle" => {
                        if let Some(style_id) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            table.properties.style_id = Some(style_id.to_string());
                        }
                    }
                    "jc" => {
                        if let Some(alignment) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            table.properties.alignment = Some(alignment.to_string());
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(())
    }

    // Parse a table row
    fn parse_table_row(tr_node: Node) -> Result<TableRow> {
        let mut row = TableRow {
            properties: TableRowProperties::default(),
            cells: vec![],
        };
        
        for child in tr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "trPr" => {
                        Self::parse_table_row_properties(child, &mut row)?;
                    }
                    "tc" => {
                        if let Ok(cell) = Self::parse_table_cell(child) {
                            row.cells.push(cell);
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(row)
    }

    // Parse table row properties
    fn parse_table_row_properties(trPr_node: Node, row: &mut TableRow) -> Result<()> {
        for child in trPr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "trHeight" => {
                        if let Some(height) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            row.properties.height = height.parse::<u32>().ok();
                        }
                        if let Some(rule) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "hRule")) {
                            row.properties.height_rule = Some(rule.to_string());
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(())
    }

    // Parse a table cell
    fn parse_table_cell(tc_node: Node) -> Result<TableCell> {
        let mut cell = TableCell {
            properties: TableCellProperties::default(),
            paragraphs: vec![],
        };
        
        for child in tc_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "tcPr" => {
                        Self::parse_table_cell_properties(child, &mut cell)?;
                    }
                    "p" => {
                        if let Ok(paragraph) = Self::parse_paragraph(child) {
                            cell.paragraphs.push(paragraph);
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(cell)
    }

    // Parse table cell properties
    fn parse_table_cell_properties(tcPr_node: Node, cell: &mut TableCell) -> Result<()> {
        for child in tcPr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "tcW" => {
                        if let (Some(width), Some(width_type)) = (
                            child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "w")).and_then(|w| w.parse::<u32>().ok()),
                            child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "type")).map(|t| t.to_string())
                        ) {
                            cell.properties.width = Some(TableWidth {
                                value: width,
                                width_type,
                            });
                        }
                    }
                    "vAlign" => {
                        if let Some(alignment) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            cell.properties.vertical_alignment = Some(alignment.to_string());
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(())
    }

    // Parse a section
    fn parse_section(sectPr_node: Node) -> Result<Section> {
        let mut section = Section {
            properties: SectionProperties::default(),
        };
        
        for child in sectPr_node.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "pgSz" => {
                        let width = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "w"))
                            .and_then(|w| w.parse::<u32>().ok())
                            .unwrap_or(12240); // Default is A4 width
                        
                        let height = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "h"))
                            .and_then(|h| h.parse::<u32>().ok())
                            .unwrap_or(15840); // Default is A4 height
                        
                        let orientation = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "orient"))
                            .unwrap_or("portrait")
                            .to_string();
                        
                        section.properties.page_size = Some(PageSize {
                            width,
                            height,
                            orientation,
                        });
                    }
                    "pgMar" => {
                        let top = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "top"))
                            .and_then(|t| t.parse::<u32>().ok())
                            .unwrap_or(1440); // Default 1 inch
                        
                        let right = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "right"))
                            .and_then(|r| r.parse::<u32>().ok())
                            .unwrap_or(1440);
                        
                        let bottom = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "bottom"))
                            .and_then(|b| b.parse::<u32>().ok())
                            .unwrap_or(1440);
                        
                        let left = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "left"))
                            .and_then(|l| l.parse::<u32>().ok())
                            .unwrap_or(1440);
                        
                        section.properties.page_margins = Some(PageMargins {
                            top,
                            right,
                            bottom,
                            left,
                        });
                    }
                    "type" => {
                        if let Some(section_type) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                            section.properties.section_type = Some(section_type.to_string());
                        }
                    }
                    _ => { /* Ignore other elements for now */ }
                }
            }
        }
        
        Ok(section)
    }

    // Parse styles.xml
    fn parse_styles_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        for style_node in root.children().filter(|n| n.has_tag_name("style")) {
            let id = style_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "styleId"))
                .unwrap_or("")
                .to_string();
            
            let style_type = style_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "type"))
                .unwrap_or("")
                .to_string();
            
            let mut name = String::new();
            let mut based_on = None;
            
            for child in style_node.children() {
                if child.is_element() {
                    match child.tag_name().name() {
                        "name" => {
                            if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                                name = val.to_string();
                            }
                        }
                        "basedOn" => {
                            if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                                based_on = Some(val.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            let style = Style {
                id: id.clone(),
                name,
                style_type,
                based_on,
            };
            
            document.styles.styles.insert(id, style);
        }
        
        Ok(())
    }

    // Parse numbering.xml
    fn parse_numbering_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        let mut numbering = Numbering {
            abstract_numberings: HashMap::new(),
            numberings: HashMap::new(),
        };
        
        // Parse abstract numbering definitions
        for abstract_num_node in root.children().filter(|n| n.has_tag_name("abstractNum")) {
            let id = abstract_num_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "abstractNumId"))
                .unwrap_or("")
                .to_string();
            
            let mut abstract_numbering = AbstractNumbering {
                id: id.clone(),
                levels: HashMap::new(),
            };
            
            for lvl_node in abstract_num_node.children().filter(|n| n.has_tag_name("lvl")) {
                if let Some(level_val) = lvl_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "ilvl"))
                    .and_then(|l| l.parse::<usize>().ok()) {
                    
                    let mut format = String::new();
                    let mut text = String::new();
                    
                    for child in lvl_node.children() {
                        if child.is_element() {
                            match child.tag_name().name() {
                                "numFmt" => {
                                    if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                                        format = val.to_string();
                                    }
                                }
                                "lvlText" => {
                                    if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                                        text = val.to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    
                    let level = NumberingLevel {
                        level: level_val,
                        format,
                        text,
                    };
                    
                    abstract_numbering.levels.insert(level_val, level);
                }
            }
            
            numbering.abstract_numberings.insert(id, abstract_numbering);
        }
        
        // Parse numbering instances
        for num_node in root.children().filter(|n| n.has_tag_name("num")) {
            let id = num_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "numId"))
                .unwrap_or("")
                .to_string();
            
            let mut abstract_numbering_id = String::new();
            
            for child in num_node.children() {
                if child.is_element() && child.tag_name().name() == "abstractNumId" {
                    if let Some(val) = child.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "val")) {
                        abstract_numbering_id = val.to_string();
                    }
                }
            }
            
            let numbering_instance = NumberingInstance {
                id: id.clone(),
                abstract_numbering_id,
            };
            
            numbering.numberings.insert(id, numbering_instance);
        }
        
        document.numbering = Some(numbering);
        
        Ok(())
    }

    // Parse relationships xml
    fn parse_relationships_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        for relationship_node in root.children().filter(|n| n.has_tag_name("Relationship")) {
            let id = relationship_node.attribute("Id").unwrap_or("").to_string();
            let relationship_type = relationship_node.attribute("Type").unwrap_or("").to_string();
            let target = relationship_node.attribute("Target").unwrap_or("").to_string();
            
            let relationship = Relationship {
                id: id.clone(),
                relationship_type,
                target,
            };
            
            document.relationships.insert(id, relationship);
        }
        
        Ok(())
    }

    // Parse footnotes.xml
    fn parse_footnotes_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        for footnote_node in root.children().filter(|n| n.has_tag_name("footnote")) {
            let id = footnote_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "id"))
                .unwrap_or("")
                .to_string();
            
            let mut paragraphs = vec![];
            
            for child in footnote_node.children() {
                if child.is_element() && child.tag_name().name() == "p" {
                    if let Ok(paragraph) = Self::parse_paragraph(child) {
                        paragraphs.push(paragraph);
                    }
                }
            }
            
            let footnote = Footnote {
                id: id.clone(),
                paragraphs,
            };
            
            document.footnotes.push(footnote);
        }
        
        Ok(())
    }

    // Parse endnotes.xml
    fn parse_endnotes_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        for endnote_node in root.children().filter(|n| n.has_tag_name("endnote")) {
            let id = endnote_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "id"))
                .unwrap_or("")
                .to_string();
            
            let mut paragraphs = vec![];
            
            for child in endnote_node.children() {
                if child.is_element() && child.tag_name().name() == "p" {
                    if let Ok(paragraph) = Self::parse_paragraph(child) {
                        paragraphs.push(paragraph);
                    }
                }
            }
            
            let endnote = Endnote {
                id: id.clone(),
                paragraphs,
            };
            
            document.endnotes.push(endnote);
        }
        
        Ok(())
    }

    // Parse comments.xml
    fn parse_comments_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        for comment_node in root.children().filter(|n| n.has_tag_name("comment")) {
            let id = comment_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "id"))
                .unwrap_or("")
                .to_string();
            
            let author = comment_node.attribute(("http://schemas.openxmlformats.org/wordprocessingml/2006/main", "author"))
                .unwrap_or("")
                .to_string();
            
            let mut paragraphs = vec![];
            
            for child in comment_node.children() {
                if child.is_element() && child.tag_name().name() == "p" {
                    if let Ok(paragraph) = Self::parse_paragraph(child) {
                        paragraphs.push(paragraph);
                    }
                }
            }
            
            let comment = Comment {
                id: id.clone(),
                author,
                paragraphs,
            };
            
            document.comments.push(comment);
        }
        
        Ok(())
    }

    // Parse core properties
    fn parse_core_properties_xml(xml_content: &str, document: &mut Document) -> Result<()> {
        let xml_doc = XmlDocument::parse(xml_content)?;
        let root = xml_doc.root_element();
        
        for child in root.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "title" => {
                        document.properties.title = child.text().map(|s| s.to_string());
                    }
                    "creator" => {
                        document.properties.author = child.text().map(|s| s.to_string());
                    }
                    "created" => {
                        document.properties.created = child.text().map(|s| s.to_string());
                    }
                    "modified" => {
                        document.properties.modified = child.text().map(|s| s.to_string());
                    }
                    _ => {
                        // Store other properties
                        if let Some(text) = child.text() {
                            let name = child.tag_name().name().to_string();
                            document.properties.other_properties.insert(name, text.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}
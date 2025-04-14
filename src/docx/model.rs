use std::collections::HashMap;

/// Represents a complete docx document
#[derive(Debug, Clone)]
pub struct Document {
    /// Document body content
    pub body: Body,
    /// Document styles
    pub styles: Styles,
    /// Document numbering definitions
    pub numbering: Option<Numbering>,
    /// Document relationships (links to external resources)
    pub relationships: HashMap<String, Relationship>,
    /// Document footnotes
    pub footnotes: Vec<Footnote>,
    /// Document endnotes
    pub endnotes: Vec<Endnote>,
    /// Document comments
    pub comments: Vec<Comment>,
    /// Document properties
    pub properties: DocumentProperties,
}

/// Represents the body of a document
#[derive(Debug, Clone)]
pub struct Body {
    /// Paragraphs in the document
    pub paragraphs: Vec<Paragraph>,
    /// Tables in the document
    pub tables: Vec<Table>,
    /// Sections in the document
    pub sections: Vec<Section>,
}

/// Represents a paragraph in a document
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// Paragraph ID
    pub id: Option<String>,
    /// Paragraph style ID
    pub style_id: Option<String>,
    /// Paragraph properties
    pub properties: ParagraphProperties,
    /// Runs of text in the paragraph
    pub runs: Vec<Run>,
}

/// Represents properties of a paragraph
#[derive(Debug, Clone, Default)]
pub struct ParagraphProperties {
    /// Paragraph alignment
    pub alignment: Option<String>,
    /// Paragraph indentation
    pub indentation: Option<Indentation>,
    /// Paragraph spacing
    pub spacing: Option<Spacing>,
    /// Numbering properties
    pub numbering: Option<NumberingProperties>,
    /// Other properties as key-value pairs
    pub other_properties: HashMap<String, String>,
}

/// Represents a run of text with consistent formatting
#[derive(Debug, Clone)]
pub struct Run {
    /// Run properties
    pub properties: RunProperties,
    /// Run content (text, breaks, etc.)
    pub contents: Vec<RunContent>,
}

/// Represents the content of a run
#[derive(Debug, Clone)]
pub enum RunContent {
    /// Plain text
    Text(String),
    /// Line break
    Break,
    /// Tab character
    Tab,
    /// Drawing/image
    Drawing(Drawing),
    /// Footnote reference
    FootnoteReference(String),
    /// Symbol
    Symbol(String),
}

/// Represents properties of a run
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunProperties {
    /// Bold formatting
    pub bold: bool,
    /// Italic formatting
    pub italic: bool,
    /// Underline formatting
    pub underline: bool,
    /// Font size in half-points
    pub size: Option<u32>,
    /// Font name
    pub font: Option<String>,
    /// Text color
    pub color: Option<String>,
    /// Other properties as key-value pairs
    pub other_properties: HashMap<String, String>,
}

/// Represents a table in a document
#[derive(Debug, Clone)]
pub struct Table {
    /// Table properties
    pub properties: TableProperties,
    /// Rows in the table
    pub rows: Vec<TableRow>,
}

/// Represents a row in a table
#[derive(Debug, Clone)]
pub struct TableRow {
    /// Row properties
    pub properties: TableRowProperties,
    /// Cells in the row
    pub cells: Vec<TableCell>,
}

/// Represents a cell in a table row
#[derive(Debug, Clone)]
pub struct TableCell {
    /// Cell properties
    pub properties: TableCellProperties,
    /// Paragraphs in the cell
    pub paragraphs: Vec<Paragraph>,
}

// Placeholder structures for other document components
#[derive(Debug, Clone, Default)]
pub struct Styles {
    /// Map of style IDs to style definitions
    pub styles: HashMap<String, Style>,
}

#[derive(Debug, Clone)]
pub struct Style {
    /// Style ID
    pub id: String,
    /// Style name
    pub name: String,
    /// Style type (paragraph, character, table, etc.)
    pub style_type: String,
    /// Parent style ID
    pub based_on: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Numbering {
    /// Map of abstract numbering definitions
    pub abstract_numberings: HashMap<String, AbstractNumbering>,
    /// Map of numbering instances
    pub numberings: HashMap<String, NumberingInstance>,
}

#[derive(Debug, Clone)]
pub struct AbstractNumbering {
    /// Abstract numbering ID
    pub id: String,
    /// Numbering levels
    pub levels: HashMap<usize, NumberingLevel>,
}

#[derive(Debug, Clone)]
pub struct NumberingLevel {
    /// Level number
    pub level: usize,
    /// Numbering format
    pub format: String,
    /// Text formatting
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct NumberingInstance {
    /// Numbering instance ID
    pub id: String,
    /// Abstract numbering ID
    pub abstract_numbering_id: String,
}

#[derive(Debug, Clone)]
pub struct NumberingProperties {
    /// Numbering ID
    pub id: String,
    /// Numbering level
    pub level: usize,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    /// Relationship ID
    pub id: String,
    /// Relationship type
    pub relationship_type: String,
    /// Target URI
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct Footnote {
    /// Footnote ID
    pub id: String,
    /// Footnote paragraphs
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone)]
pub struct Endnote {
    /// Endnote ID
    pub id: String,
    /// Endnote paragraphs
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone)]
pub struct Comment {
    /// Comment ID
    pub id: String,
    /// Comment author
    pub author: String,
    /// Comment paragraphs
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone)]
pub struct Drawing {
    /// Drawing ID
    pub id: String,
    /// Drawing description
    pub description: Option<String>,
    /// Drawing name
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Section {
    /// Section properties
    pub properties: SectionProperties,
}

#[derive(Debug, Clone, Default)]
pub struct SectionProperties {
    /// Page size
    pub page_size: Option<PageSize>,
    /// Page margins
    pub page_margins: Option<PageMargins>,
    /// Section type
    pub section_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PageSize {
    /// Page width in twentieths of a point
    pub width: u32,
    /// Page height in twentieths of a point
    pub height: u32,
    /// Page orientation
    pub orientation: String,
}

#[derive(Debug, Clone)]
pub struct PageMargins {
    /// Top margin in twentieths of a point
    pub top: u32,
    /// Right margin in twentieths of a point
    pub right: u32,
    /// Bottom margin in twentieths of a point
    pub bottom: u32,
    /// Left margin in twentieths of a point
    pub left: u32,
}

#[derive(Debug, Clone)]
pub struct Indentation {
    /// Left indentation in twentieths of a point
    pub left: Option<i32>,
    /// Right indentation in twentieths of a point
    pub right: Option<i32>,
    /// First line indentation in twentieths of a point
    pub first_line: Option<i32>,
    /// Hanging indentation in twentieths of a point
    pub hanging: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct Spacing {
    /// Line spacing in twentieths of a point
    pub line: Option<u32>,
    /// Line spacing rule
    pub line_rule: Option<String>,
    /// Space before paragraph in twentieths of a point
    pub before: Option<u32>,
    /// Space after paragraph in twentieths of a point
    pub after: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct TableProperties {
    /// Table style ID
    pub style_id: Option<String>,
    /// Table width
    pub width: Option<TableWidth>,
    /// Table borders
    pub borders: Option<TableBorders>,
    /// Table alignment
    pub alignment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TableWidth {
    /// Width value
    pub value: u32,
    /// Width type
    pub width_type: String,
}

#[derive(Debug, Clone)]
pub struct TableBorders {
    /// Top border
    pub top: Option<Border>,
    /// Right border
    pub right: Option<Border>,
    /// Bottom border
    pub bottom: Option<Border>,
    /// Left border
    pub left: Option<Border>,
    /// Inside horizontal borders
    pub inside_h: Option<Border>,
    /// Inside vertical borders
    pub inside_v: Option<Border>,
}

#[derive(Debug, Clone)]
pub struct Border {
    /// Border style
    pub style: String,
    /// Border width
    pub width: u32,
    /// Border color
    pub color: String,
}

#[derive(Debug, Clone, Default)]
pub struct TableRowProperties {
    /// Row height
    pub height: Option<u32>,
    /// Row height rule
    pub height_rule: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TableCellProperties {
    /// Cell width
    pub width: Option<TableWidth>,
    /// Cell vertical alignment
    pub vertical_alignment: Option<String>,
    /// Cell borders
    pub borders: Option<TableBorders>,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentProperties {
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document creation date
    pub created: Option<String>,
    /// Document last modified date
    pub modified: Option<String>,
    /// Other properties as key-value pairs
    pub other_properties: HashMap<String, String>,
}

impl Document {
    /// Creates a new empty document
    pub fn new() -> Self {
        Document {
            body: Body {
                paragraphs: vec![],
                tables: vec![],
                sections: vec![],
            },
            styles: Styles::default(),
            numbering: None,
            relationships: HashMap::new(),
            footnotes: vec![],
            endnotes: vec![],
            comments: vec![],
            properties: DocumentProperties::default(),
        }
    }
}

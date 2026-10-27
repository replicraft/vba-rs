use crate::Preserved;
use crate::standalone::{self, StandaloneError};

/// The role of a module within a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModuleKind {
    /// A procedural (`.bas`) module.
    Standard,
    /// A class (`.cls`) module.
    Class,
    /// A document module that extends a host document object (for example a
    /// worksheet).
    Document,
    /// A designer module such as a user form (`.frm`).
    Designer,
}

impl ModuleKind {
    /// Lower-case name of the kind.
    pub fn name(self) -> &'static str {
        match self {
            ModuleKind::Standard => "standard",
            ModuleKind::Class => "class",
            ModuleKind::Document => "document",
            ModuleKind::Designer => "designer",
        }
    }
}

/// One module of a VBA project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// Module name (a VBA identifier; VBA compares names case-insensitively).
    pub name: String,
    /// Role of the module.
    pub kind: ModuleKind,
    /// Source text exactly as stored, with its original line endings.
    ///
    /// For modules inside a project this includes the `Attribute` lines. It
    /// does not include an export file header (see [`Module::file_header`]).
    pub source: String,
    /// Whether the source of this module is actually present. A project whose
    /// source was stripped keeps its modules but has no text to show; then
    /// `source` is empty and this is `false`.
    pub has_source: bool,
    /// Description of the module.
    pub doc_string: String,
    /// Help topic identifier of the module.
    pub help_context: u32,
    /// Whether the module is flagged read-only.
    pub read_only: bool,
    /// Whether the module is flagged private to the project.
    pub private: bool,
    /// Text that precedes the code in an exported `.cls` or `.frm` file
    /// (`VERSION` line and `BEGIN`/`Begin` ... `END`/`End` block). `None` for
    /// modules read from a project and for `.bas` files.
    pub file_header: Option<String>,
    /// Data attached by a format layer.
    pub preserved: Preserved,
}

impl Module {
    /// Creates a module with the given name, kind and source text.
    pub fn new(name: impl Into<String>, kind: ModuleKind, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind,
            source: source.into(),
            has_source: true,
            doc_string: String::new(),
            help_context: 0,
            read_only: false,
            private: false,
            file_header: None,
            preserved: Preserved::new(),
        }
    }

    /// Builds a module from the text of a standalone `.bas`, `.cls` or `.frm`
    /// file.
    ///
    /// The kind is chosen from `file_name`'s extension (case-insensitive). The
    /// module name is taken from the `Attribute VB_Name = "..."` line, falling
    /// back to the file stem. For `.cls` and `.frm` the export header is split
    /// off into [`Module::file_header`]. Designer binary data in a companion
    /// `.frx` file is not read.
    pub fn from_source_file(file_name: &str, text: &str) -> Result<Self, StandaloneError> {
        standalone::parse(file_name, text)
    }

    /// Source text as it would be written to a standalone file: the file
    /// header (if any) followed by the source.
    pub fn to_source_file(&self) -> String {
        match &self.file_header {
            Some(h) => format!("{h}{}", self.source),
            None => self.source.clone(),
        }
    }

    /// Conventional file extension (without the dot) for the module kind.
    pub fn file_extension(&self) -> &'static str {
        match self.kind {
            ModuleKind::Standard => "bas",
            ModuleKind::Class | ModuleKind::Document => "cls",
            ModuleKind::Designer => "frm",
        }
    }
}

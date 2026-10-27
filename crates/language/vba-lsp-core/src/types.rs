//! Plain result types of the language features. They mirror what an editor
//! protocol needs but carry no protocol dependency; positions are UTF-16
//! based (see [`crate::text`]).

use crate::text::Range;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// An error.
    Error,
    /// A warning.
    Warning,
    /// Informational.
    Info,
    /// A hint.
    Hint,
}

/// A diagnostic with an editor range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Where the problem is.
    pub range: Range,
    /// Severity.
    pub severity: Severity,
    /// Stable code (`SYN002`, `LEX001`, ...).
    pub code: String,
    /// Message, with notes appended on following lines.
    pub message: String,
    /// Secondary locations with messages.
    pub related: Vec<(Range, String)>,
}

/// The kind of a document symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// `Sub`.
    Method,
    /// `Function` and `Declare`d procedures.
    Function,
    /// `Property Get/Let/Set`.
    Property,
    /// Module-level variable.
    Field,
    /// Local variable or parameter.
    Variable,
    /// `Const`.
    Constant,
    /// `Enum`.
    Enum,
    /// Enum member.
    EnumMember,
    /// `Type` (user-defined type).
    Struct,
    /// `Event`.
    Event,
    /// A standard module.
    Module,
    /// A class, document or designer module.
    Class,
}

/// A symbol in the outline of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    /// Name as written.
    pub name: String,
    /// Short description such as `Function`, `Private Const` or the type.
    pub detail: String,
    /// Kind.
    pub kind: SymbolKind,
    /// The whole declaration.
    pub range: Range,
    /// The name only.
    pub selection_range: Range,
    /// Nested symbols (type members, enum members, locals).
    pub children: Vec<DocumentSymbol>,
}

/// What a folding range folds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldingKind {
    /// A run of comment lines.
    Comment,
    /// Anything else (procedures, blocks, conditional compilation).
    Region,
}

/// A folding range in whole lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoldingRange {
    /// First line (stays visible).
    pub start_line: u32,
    /// Last line (hidden when folded).
    pub end_line: u32,
    /// Kind.
    pub kind: FoldingKind,
}

/// Semantic token types, in legend order.
pub const TOKEN_TYPES: &[&str] = &[
    "keyword",
    "comment",
    "string",
    "number",
    "operator",
    "type",
    "function",
    "variable",
    "parameter",
    "property",
    "enumMember",
    "label",
    "macro",
];

/// Semantic token modifiers, in legend order (bit `n` is entry `n`).
pub const TOKEN_MODIFIERS: &[&str] = &["declaration", "readonly", "defaultLibrary", "inactive"];

/// Index into [`TOKEN_TYPES`].
pub mod token_type {
    /// `keyword`
    pub const KEYWORD: u32 = 0;
    /// `comment`
    pub const COMMENT: u32 = 1;
    /// `string`
    pub const STRING: u32 = 2;
    /// `number`
    pub const NUMBER: u32 = 3;
    /// `operator`
    pub const OPERATOR: u32 = 4;
    /// `type`
    pub const TYPE: u32 = 5;
    /// `function`
    pub const FUNCTION: u32 = 6;
    /// `variable`
    pub const VARIABLE: u32 = 7;
    /// `parameter`
    pub const PARAMETER: u32 = 8;
    /// `property`
    pub const PROPERTY: u32 = 9;
    /// `enumMember`
    pub const ENUM_MEMBER: u32 = 10;
    /// `label`
    pub const LABEL: u32 = 11;
    /// `macro`
    pub const MACRO: u32 = 12;
}

/// Bit masks for [`TOKEN_MODIFIERS`].
pub mod token_modifier {
    /// `declaration`
    pub const DECLARATION: u32 = 1;
    /// `readonly`
    pub const READONLY: u32 = 1 << 1;
    /// `defaultLibrary`
    pub const DEFAULT_LIBRARY: u32 = 1 << 2;
    /// `inactive`
    pub const INACTIVE: u32 = 1 << 3;
}

/// One semantic token on a single line (absolute position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticToken {
    /// Zero-based line.
    pub line: u32,
    /// Start column in UTF-16 units.
    pub start: u32,
    /// Length in UTF-16 units.
    pub length: u32,
    /// Index into [`TOKEN_TYPES`].
    pub token_type: u32,
    /// Bit set of [`TOKEN_MODIFIERS`].
    pub modifiers: u32,
}

/// Hover content (Markdown) for a range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Markdown text.
    pub contents: String,
    /// The hovered token.
    pub range: Range,
}

/// The kind of a completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    /// A reserved word.
    Keyword,
    /// An intrinsic function.
    Function,
    /// A procedure declared in the file.
    Procedure,
    /// A variable, parameter or local.
    Variable,
    /// A constant.
    Constant,
    /// A type or enum name.
    Type,
    /// An enum member.
    EnumMember,
    /// A library or module qualifier.
    Module,
    /// A property of a class module.
    Property,
    /// A public variable of a class or a field of a user-defined type.
    Field,
}

/// A completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Text shown and inserted.
    pub label: String,
    /// Kind.
    pub kind: CompletionKind,
    /// Signature or description.
    pub detail: Option<String>,
}

/// The completion list and the range the typed prefix occupies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completions {
    /// Range of the identifier prefix to be replaced.
    pub replace: Range,
    /// Candidates, sorted by label.
    pub items: Vec<CompletionItem>,
}

/// One parameter of a signature: byte-free label offsets into the signature
/// label, counted in UTF-16 units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterInfo {
    /// Start offset in the signature label.
    pub start: u32,
    /// End offset in the signature label.
    pub end: u32,
}

/// A callable signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInfo {
    /// Full label such as `Left(String, Length As Long) As String`.
    pub label: String,
    /// Documentation (Markdown).
    pub documentation: Option<String>,
    /// Parameter label ranges.
    pub parameters: Vec<ParameterInfo>,
}

/// Signature help result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// The signature(s) of the callee.
    pub signatures: Vec<SignatureInfo>,
    /// Index of the parameter at the cursor, if it names one.
    pub active_parameter: Option<u32>,
}

/// A text replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Range to replace.
    pub range: Range,
    /// Replacement text.
    pub new_text: String,
}

/// A range in a document of the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// URI of the document.
    pub uri: String,
    /// The range.
    pub range: Range,
}

/// The edits one rename makes to one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEdit {
    /// URI of the document.
    pub uri: String,
    /// Non-overlapping edits, in source order.
    pub edits: Vec<TextEdit>,
}

/// A declaration found by a workspace-wide symbol search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    /// Name as written.
    pub name: String,
    /// Kind.
    pub kind: SymbolKind,
    /// Declaring module (or enum/type for members and fields).
    pub container: String,
    /// Where the name is declared.
    pub location: Location,
}

/// A selection range with its enclosing ranges, innermost first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionRange {
    /// Innermost first, strictly growing.
    pub chain: Vec<Range>,
}

/// Formatting options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    /// Spaces per indent level (also the tab width when indenting with tabs).
    pub tab_size: u32,
    /// Indent with spaces (true) or tabs.
    pub insert_spaces: bool,
    /// Rewrite reserved words to their canonical spelling (`elseif` becomes
    /// `ElseIf`). Only the case of letters changes; VBA is case-insensitive.
    /// Words after `.` or `!`, attribute names and inactive text are left
    /// alone.
    pub keyword_case: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            tab_size: 4,
            insert_spaces: true,
            keyword_case: false,
        }
    }
}

/// Budgets of the checked formatting entry points
/// ([`View::try_format`](crate::View::try_format)). `None` means unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FormatLimits {
    /// Bytes of the document.
    pub max_source_bytes: Option<u64>,
    /// Width of one line's indentation, in characters (spaces, or tabs when
    /// indenting with tabs). Guards against a huge `tab_size`; a line that
    /// would be indented wider refuses the whole request.
    pub max_indent_width: Option<u64>,
}

/// Document size of [`FormatLimits::untrusted`].
pub const UNTRUSTED_FORMAT_SOURCE_BYTES: u64 = 8 * 1024 * 1024;

/// Indent width of [`FormatLimits::untrusted`].
pub const UNTRUSTED_FORMAT_INDENT_WIDTH: u64 = 1024;

impl FormatLimits {
    /// No budget.
    pub const fn unlimited() -> Self {
        FormatLimits {
            max_source_bytes: None,
            max_indent_width: None,
        }
    }

    /// Budgets for source that is not trusted: [`UNTRUSTED_FORMAT_SOURCE_BYTES`]
    /// bytes and an indent of at most [`UNTRUSTED_FORMAT_INDENT_WIDTH`]
    /// characters. The source size matches `ResourceLimits::untrusted()`.
    pub const fn untrusted() -> Self {
        FormatLimits {
            max_source_bytes: Some(UNTRUSTED_FORMAT_SOURCE_BYTES),
            max_indent_width: Some(UNTRUSTED_FORMAT_INDENT_WIDTH),
        }
    }

    /// Sets [`max_source_bytes`](FormatLimits::max_source_bytes).
    #[must_use]
    pub const fn with_max_source_bytes(mut self, n: Option<u64>) -> Self {
        self.max_source_bytes = n;
        self
    }

    /// Sets [`max_indent_width`](FormatLimits::max_indent_width).
    #[must_use]
    pub const fn with_max_indent_width(mut self, n: Option<u64>) -> Self {
        self.max_indent_width = n;
        self
    }
}

/// Why a checked formatting request was refused before any work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatError {
    /// The document is larger than [`FormatLimits::max_source_bytes`].
    TooLarge {
        /// The budget.
        limit: u64,
        /// The size of the document.
        actual: u64,
    },
    /// The options would indent a line wider than
    /// [`FormatLimits::max_indent_width`].
    IndentTooWide {
        /// The budget.
        limit: u64,
        /// The width the options ask for at the deepest allowed nesting.
        wanted: u64,
    },
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::TooLarge { limit, actual } => write!(
                f,
                "the document has {actual} bytes; the formatting limit is {limit}"
            ),
            FormatError::IndentTooWide { limit, wanted } => write!(
                f,
                "the indentation would be {wanted} characters wide; the limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for FormatError {}

/// Why a request could not be answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The document is not open.
    NotOpen(String),
    /// The request is valid but this implementation cannot do it.
    Unsupported(String),
    /// The request is invalid for the position or arguments given.
    Invalid(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotOpen(u) => write!(f, "document is not open: {u}"),
            Error::Unsupported(m) | Error::Invalid(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

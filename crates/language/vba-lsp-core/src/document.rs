//! Open documents: a byte buffer, its version and a per-version parse cache.

use std::sync::{Arc, OnceLock};

use vba_profile::Profile;
use vba_source::{FileId, SourceEncoding, Span};
use vba_syntax::{Parse, ParseOptions, SyntaxTree};

use crate::scope::ScopeIndex;
use crate::text::{Range, TextMap, decode, encode};

/// The file id every in-memory document is parsed under. Diagnostics from
/// plugged-in providers must use the same id.
pub const FILE_ID: FileId = FileId(0);

/// One edit of a document. `range: None` replaces the whole text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChange {
    /// Replaced range, or `None` for a full-text change.
    pub range: Option<Range>,
    /// New text.
    pub text: String,
}

/// The result of parsing one version of a document.
#[derive(Debug)]
pub struct Analysis {
    /// The tolerant parse (tree and syntax/lexer diagnostics).
    pub parse: Parse,
    scopes: OnceLock<ScopeIndex>,
}

impl Analysis {
    /// The syntax tree.
    pub fn tree(&self) -> &SyntaxTree {
        &self.parse.tree
    }

    /// The syntactic scope index, built on first use.
    pub(crate) fn scopes(&self, enc: SourceEncoding) -> &ScopeIndex {
        self.scopes
            .get_or_init(|| ScopeIndex::build(&self.parse.tree, enc))
    }
}

/// The settings that influence parsing.
#[derive(Debug, Clone)]
pub struct Config {
    /// The VBA profile (version, host, bitness) used for lexing, parsing
    /// and conditional compilation.
    pub profile: Profile,
    /// Encoding assumed for documents opened after this setting is made.
    pub encoding: SourceEncoding,
    /// Project-level `#Const` values.
    pub constants: Vec<(String, i64)>,
    /// Whether open documents are analysed as one project by `vba-sema`
    /// (semantic diagnostics, cross-module navigation, type-aware hover and
    /// completion). When off, only the syntactic features are available.
    pub semantic: bool,
    /// Project name used for `Project.Module.Name` qualification.
    pub project_name: String,
    /// Module kind overrides. A pattern is a full URI, a file name or
    /// `*.ext`; the first match wins. Without a match the kind comes from
    /// the file extension (`.bas` standard, `.cls` class, `.frm`
    /// designer) or the export header, and is standard otherwise.
    pub module_kinds: Vec<(String, crate::ModuleKind)>,
    /// The described class that is the `Me` of a module, by module name:
    /// inside such a module the controls and members of that class are
    /// completed, hovered and checked, and the other modules' controls are
    /// not. The class is looked up in the workspace's type provider.
    pub module_hosts: Vec<vba_sema::ModuleHost>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            profile: *vba_profile::default_profile(),
            encoding: SourceEncoding::Utf8,
            constants: Vec::new(),
            semantic: true,
            project_name: "VBAProject".to_string(),
            module_kinds: Vec::new(),
            module_hosts: Vec::new(),
        }
    }
}

impl Config {
    /// Selects a built-in profile by identifier (`vba7.1-excel-win64`).
    pub fn set_profile(&mut self, id: &str) -> Result<(), crate::Error> {
        match vba_profile::find_profile(id) {
            Some(p) => {
                self.profile = *p;
                Ok(())
            }
            None => Err(crate::Error::Invalid(format!(
                "unknown VBA profile `{id}`; known profiles: {}",
                vba_profile::profiles()
                    .iter()
                    .map(|p| p.id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }
}

/// Parses an encoding name: `utf-8`, `cp1252`, `windows-1252`, `932`, ...
pub fn parse_encoding(name: &str) -> Option<SourceEncoding> {
    let n = name.trim().to_ascii_lowercase();
    if matches!(n.as_str(), "utf-8" | "utf8") {
        return Some(SourceEncoding::Utf8);
    }
    let digits = n
        .strip_prefix("windows-")
        .or_else(|| n.strip_prefix("cp"))
        .or_else(|| n.strip_prefix("ibm"))
        .unwrap_or(&n);
    digits.parse::<u16>().ok().map(SourceEncoding::CodePage)
}

/// An open document.
#[derive(Debug)]
pub struct Document {
    version: i32,
    bytes: Vec<u8>,
    map: TextMap,
    analysis: OnceLock<Arc<Analysis>>,
}

impl Document {
    /// Creates a document from editor text, encoded to `encoding`.
    pub fn new(text: &str, version: i32, encoding: SourceEncoding) -> Self {
        Self::from_bytes(encode(text, encoding), version, encoding)
    }

    /// Creates a document from a buffer already in `encoding`.
    pub(crate) fn from_bytes(bytes: Vec<u8>, version: i32, encoding: SourceEncoding) -> Self {
        let map = TextMap::new(&bytes, encoding);
        Document {
            version,
            bytes,
            map,
            analysis: OnceLock::new(),
        }
    }

    /// The document version given by the client.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// The buffer bytes in the document encoding.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The position map.
    pub fn text_map(&self) -> &TextMap {
        &self.map
    }

    /// The buffer decoded to a string.
    pub fn text(&self) -> String {
        decode(&self.bytes, self.map.encoding())
    }

    /// Applies edits in order and sets the new version. Ranges beyond the
    /// text clamp to its end. Invalidates the parse cache.
    pub fn apply_changes(&mut self, version: i32, changes: &[TextChange]) {
        let enc = self.map.encoding();
        for c in changes {
            let new = encode(&c.text, enc);
            match c.range {
                None => self.bytes = new,
                Some(r) => {
                    let Span { start, end } = self.map.span(&self.bytes, r);
                    self.bytes.splice(start as usize..end as usize, new);
                }
            }
            self.map = TextMap::new(&self.bytes, enc);
        }
        self.version = version;
        self.analysis = OnceLock::new();
    }

    /// Drops the cached parse (after a configuration change).
    pub(crate) fn invalidate(&mut self) {
        self.analysis = OnceLock::new();
    }

    /// The parse of the current version, computed on first use.
    pub fn analysis(&self, config: &Config) -> Arc<Analysis> {
        self.analysis
            .get_or_init(|| {
                let mut opts = ParseOptions::tolerant();
                opts.constants = config.constants.clone();
                Arc::new(Analysis {
                    parse: vba_syntax::parse(FILE_ID, &self.bytes, &config.profile, &opts),
                    scopes: OnceLock::new(),
                })
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Position;

    fn r(a: u32, b: u32, c: u32, d: u32) -> Option<Range> {
        Some(Range::new(Position::new(a, b), Position::new(c, d)))
    }

    #[test]
    fn incremental_edits_apply_in_order() {
        let mut d = Document::new("Sub A()\nEnd Sub\n", 1, SourceEncoding::Utf8);
        d.apply_changes(
            2,
            &[
                TextChange {
                    range: r(0, 4, 0, 5),
                    text: "Foo".into(),
                },
                TextChange {
                    range: r(1, 0, 1, 0),
                    text: "    x = 1\n".into(),
                },
            ],
        );
        assert_eq!(d.text(), "Sub Foo()\n    x = 1\nEnd Sub\n");
        assert_eq!(d.version(), 2);
    }

    #[test]
    fn edits_use_utf16_columns() {
        let mut d = Document::new("'😀 x\n", 1, SourceEncoding::Utf8);
        // '😀' is two UTF-16 units: replace the `x` at column 5.
        d.apply_changes(
            2,
            &[TextChange {
                range: r(0, 4, 0, 5),
                text: "y".into(),
            }],
        );
        assert_eq!(d.text(), "'😀 y\n");
    }

    #[test]
    fn full_change_and_out_of_range_edit() {
        let mut d = Document::new("a", 1, SourceEncoding::Utf8);
        d.apply_changes(
            2,
            &[TextChange {
                range: r(9, 0, 9, 5),
                text: "b".into(),
            }],
        );
        assert_eq!(d.text(), "ab");
        d.apply_changes(
            3,
            &[TextChange {
                range: None,
                text: "z".into(),
            }],
        );
        assert_eq!(d.text(), "z");
    }

    #[test]
    fn parse_is_cached_per_version() {
        let cfg = Config::default();
        let mut d = Document::new("Sub A()\nEnd Sub\n", 1, SourceEncoding::Utf8);
        let a = d.analysis(&cfg);
        assert!(Arc::ptr_eq(&a, &d.analysis(&cfg)));
        d.apply_changes(
            2,
            &[TextChange {
                range: None,
                text: "Sub B()\nEnd Sub\n".into(),
            }],
        );
        assert!(!Arc::ptr_eq(&a, &d.analysis(&cfg)));
    }

    #[test]
    fn encodings_parse() {
        assert_eq!(parse_encoding("UTF-8"), Some(SourceEncoding::Utf8));
        assert_eq!(
            parse_encoding("cp1252"),
            Some(SourceEncoding::CodePage(1252))
        );
        assert_eq!(
            parse_encoding("windows-932"),
            Some(SourceEncoding::CodePage(932))
        );
        assert_eq!(parse_encoding("nonsense"), None);
    }
}

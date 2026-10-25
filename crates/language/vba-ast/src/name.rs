//! Case-insensitive names that keep their original spelling.

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use vba_lexer::{Keyword, TypeSuffix};
use vba_source::Span;

/// A VBA name: compares and hashes case-insensitively (ASCII folding) and
/// keeps the spelling as written.
///
/// VBA is case-insensitive and the editor rewrites identifiers to the case
/// of their declaration, so the original spelling matters for display but
/// never for lookup. Only ASCII letters fold; other bytes (code-page
/// characters) compare exactly, because folding them would need the code
/// page, which the AST does not know.
#[derive(Debug, Clone)]
pub struct Name {
    original: Box<[u8]>,
}

impl Name {
    /// Creates a name from its spelling.
    pub fn new(spelling: impl AsRef<[u8]>) -> Name {
        Name {
            original: spelling.as_ref().into(),
        }
    }

    /// The spelling as written.
    pub fn as_bytes(&self) -> &[u8] {
        &self.original
    }

    /// The spelling as text, replacing invalid UTF-8.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.original).into_owned()
    }

    /// The ASCII-lower-cased spelling, the canonical lookup key.
    pub fn folded(&self) -> Vec<u8> {
        self.original.to_ascii_lowercase()
    }

    /// Whether the name equals `other` ignoring ASCII case.
    pub fn eq_ignore_case(&self, other: impl AsRef<[u8]>) -> bool {
        self.original.eq_ignore_ascii_case(other.as_ref())
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.original.eq_ignore_ascii_case(&other.original)
    }
}

impl Eq for Name {}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for b in self.original.iter() {
            state.write_u8(b.to_ascii_lowercase());
        }
        state.write_u8(0xff);
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Name) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Name) -> Ordering {
        self.original
            .iter()
            .map(u8::to_ascii_lowercase)
            .cmp(other.original.iter().map(u8::to_ascii_lowercase))
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

/// How an identifier was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentKind {
    /// A plain identifier.
    Plain,
    /// A bracketed foreign name `[like this]`; the brackets are not part
    /// of [`Ident::name`].
    Foreign,
    /// A reserved word used as a name (`Len`, `Date`, `Me`, a member after
    /// `.`, ...).
    Keyword(Keyword),
}

/// An identifier occurrence: name, type-declaration suffix and location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    /// The name without its type suffix (and without brackets).
    pub name: Name,
    /// The type-declaration suffix (`$`, `%`, ...) if written.
    pub suffix: Option<TypeSuffix>,
    /// How the identifier was written.
    pub kind: IdentKind,
    /// Source span of the whole token, suffix and brackets included.
    pub span: Span,
}

impl Ident {
    /// A synthetic identifier for incomplete syntax (empty name).
    pub fn missing(span: Span) -> Ident {
        Ident {
            name: Name::new(b""),
            suffix: None,
            kind: IdentKind::Plain,
            span,
        }
    }

    /// Whether the identifier stands for missing syntax.
    pub fn is_missing(&self) -> bool {
        self.name.as_bytes().is_empty()
    }
}

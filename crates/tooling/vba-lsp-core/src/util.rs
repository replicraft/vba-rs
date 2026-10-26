//! Small helpers over the syntax tree shared by the features.

use vba_lexer::{Keyword, TokenKind};
use vba_syntax::{NodeKind, SyntaxElement, SyntaxNode, SyntaxToken};

/// The kind of a procedure-like declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcKind {
    Sub,
    Function,
    PropertyGet,
    PropertyLet,
    PropertySet,
}

impl ProcKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            ProcKind::Sub => "Sub",
            ProcKind::Function => "Function",
            ProcKind::PropertyGet => "Property Get",
            ProcKind::PropertyLet => "Property Let",
            ProcKind::PropertySet => "Property Set",
        }
    }
}

fn is_property_word(t: &SyntaxToken<'_>) -> bool {
    t.kind() == TokenKind::Ident && t.text_eq("Property")
}

/// For a `ProcDecl` or `DeclareStmt`: the procedure kind and the name token.
pub(crate) fn proc_header<'t>(node: SyntaxNode<'t>) -> Option<(ProcKind, SyntaxToken<'t>)> {
    let mut toks = node.significant_tokens();
    while let Some(t) = toks.next() {
        let kind = match t.kind() {
            TokenKind::Keyword(Keyword::Sub) => ProcKind::Sub,
            TokenKind::Keyword(Keyword::Function) => ProcKind::Function,
            _ if is_property_word(&t) => match toks.next()?.kind() {
                TokenKind::Keyword(Keyword::Get) => ProcKind::PropertyGet,
                TokenKind::Keyword(Keyword::Let) => ProcKind::PropertyLet,
                TokenKind::Keyword(Keyword::Set) => ProcKind::PropertySet,
                _ => return None,
            },
            _ => continue,
        };
        let name = toks.next()?;
        return is_name_like(name.kind()).then_some((kind, name));
    }
    None
}

/// Whether a token can be a declared name.
pub(crate) fn is_name_like(k: TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
    )
}

/// The name token of a declaration node (`ConstItem`, `VarDeclarator`,
/// `TypeMember`, `EnumMember`, `Param`, `EventStmt`, `TypeStmt`,
/// `EnumStmt`, `LabelStmt`).
pub(crate) fn decl_name<'t>(node: SyntaxNode<'t>) -> Option<SyntaxToken<'t>> {
    match node.kind() {
        NodeKind::ProcDecl | NodeKind::DeclareStmt => proc_header(node).map(|(_, t)| t),
        NodeKind::EventStmt | NodeKind::TypeStmt | NodeKind::EnumStmt => {
            let mut it = node.significant_tokens();
            it.find(|t| {
                matches!(
                    t.kind(),
                    TokenKind::Keyword(Keyword::Event | Keyword::Type | Keyword::Enum)
                )
            })?;
            it.next().filter(|t| is_name_like(t.kind()))
        }
        NodeKind::Param => node.significant_tokens().find(|t| {
            !matches!(
                t.kind(),
                TokenKind::Keyword(
                    Keyword::Optional | Keyword::ByVal | Keyword::ByRef | Keyword::ParamArray
                )
            )
        }),
        NodeKind::ConstItem
        | NodeKind::VarDeclarator
        | NodeKind::TypeMember
        | NodeKind::EnumMember
        | NodeKind::LabelStmt => node.significant_tokens().next(),
        _ => None,
    }
}

/// Source text of the header of a declaration on one line: tokens up to the
/// body (`Block`, members), comments dropped, runs of whitespace and line
/// continuations collapsed to one space.
pub(crate) fn header_text(node: SyntaxNode<'_>, decode: impl Fn(&[u8]) -> String) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    'outer: for el in node.children() {
        match el {
            SyntaxElement::Node(n) => {
                if matches!(
                    n.kind(),
                    NodeKind::Block | NodeKind::EnumMember | NodeKind::TypeMember
                ) {
                    break 'outer;
                }
                for t in n.descendant_tokens() {
                    push_token(&mut out, &mut pending_space, t, &decode);
                }
            }
            SyntaxElement::Token(t) => push_token(&mut out, &mut pending_space, t, &decode),
        }
    }
    out
}

fn push_token(
    out: &mut String,
    pending_space: &mut bool,
    t: SyntaxToken<'_>,
    decode: &impl Fn(&[u8]) -> String,
) {
    match t.kind() {
        TokenKind::Whitespace | TokenKind::LineContinuation | TokenKind::Newline => {
            *pending_space = !out.is_empty();
        }
        TokenKind::Comment | TokenKind::RemComment => {}
        _ => {
            if *pending_space {
                out.push(' ');
                *pending_space = false;
            }
            out.push_str(&decode(t.text()));
        }
    }
}

/// The name of an identifier for comparisons: ASCII lower case, type
/// suffix removed, brackets of a foreign name removed.
pub(crate) fn normalize(text: &[u8]) -> String {
    let mut s = String::from_utf8_lossy(text).to_ascii_lowercase();
    if s.starts_with('[') && s.ends_with(']') && s.len() >= 2 {
        s = s[1..s.len() - 1].to_string();
    } else if let Some(c) = s.chars().last()
        && matches!(c, '%' | '&' | '^' | '!' | '#' | '@' | '$')
        && s.len() > 1
    {
        s.pop();
    }
    s
}

/// Length in bytes of the type suffix of an identifier token, 0 if none.
pub(crate) fn suffix_len(t: &SyntaxToken<'_>) -> u32 {
    if t.kind() == TokenKind::Ident
        && t.text().len() > 1
        && matches!(
            t.text().last(),
            Some(b'%' | b'&' | b'^' | b'!' | b'#' | b'@' | b'$')
        )
    {
        1
    } else {
        0
    }
}

/// The dotted name ending at token `i` (`VBA.Strings.Left`), made of
/// directly adjacent words and dots only. `obj.Left` yields `obj.Left`; a
/// `With` member (`.Left`) yields just `Left` with `leading_dot` set.
pub(crate) fn qualified_name(tokens: &[SyntaxToken<'_>], i: usize) -> (String, bool) {
    let text = |t: &SyntaxToken<'_>| String::from_utf8_lossy(t.text()).into_owned();
    let mut parts = vec![text(&tokens[i])];
    let mut j = i;
    while j >= 2
        && tokens[j - 1].kind() == TokenKind::Dot
        && crate::workspace::is_word(tokens[j - 2].kind())
    {
        parts.insert(0, text(&tokens[j - 2]));
        j -= 2;
    }
    let leading_dot = j >= 1 && tokens[j - 1].kind() == TokenKind::Dot;
    (parts.join("."), leading_dot)
}

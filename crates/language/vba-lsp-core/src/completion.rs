//! Completion: keywords valid at the cursor, library functions and names
//! declared in the file. Purely syntactic; there is no type information, so
//! `obj.` offers nothing except for enums and library qualifiers.

use std::collections::HashSet;

use vba_lexer::{Keyword, TokenKind};
use vba_profile::Feature;
use vba_syntax::{NodeKind, SyntaxToken};

use crate::builtin::std_signature;
use crate::scope::SymKind;
use crate::text::{Position, Range};
use crate::types::{CompletionItem, CompletionKind, Completions};
use crate::util::{ProcKind, normalize, proc_header};
use crate::workspace::{View, is_word};

const STATEMENT_KEYWORDS: &[&str] = &[
    "Call",
    "Close",
    "Const",
    "Debug",
    "Dim",
    "Do",
    "Erase",
    "Exit",
    "For",
    "Get",
    "GoSub",
    "GoTo",
    "If",
    "Input",
    "Let",
    "Lock",
    "On",
    "Open",
    "Print",
    "Put",
    "RaiseEvent",
    "ReDim",
    "Resume",
    "Return",
    "Seek",
    "Select",
    "Set",
    "Static",
    "Stop",
    "Unlock",
    "While",
    "With",
    "Write",
    "End",
];

const MODULE_KEYWORDS: &[&str] = &[
    "Option",
    "Attribute",
    "Public",
    "Private",
    "Friend",
    "Global",
    "Dim",
    "Const",
    "Declare",
    "Type",
    "Enum",
    "Event",
    "Sub",
    "Function",
    "Property",
    "Implements",
    "Static",
    "DefBool",
    "DefByte",
    "DefInt",
    "DefLng",
    "DefCur",
    "DefSng",
    "DefDbl",
    "DefDate",
    "DefStr",
    "DefObj",
    "DefVar",
];

const EXPRESSION_KEYWORDS: &[&str] = &[
    "AddressOf",
    "Empty",
    "False",
    "Me",
    "New",
    "Not",
    "Nothing",
    "Null",
    "True",
    "TypeOf",
];

const OPERATOR_KEYWORDS: &[&str] = &["And", "Eqv", "Imp", "Is", "Like", "Mod", "Or", "Xor"];

const TYPE_KEYWORDS: &[&str] = &[
    "Boolean", "Byte", "Currency", "Date", "Double", "Integer", "Long", "LongLong", "LongPtr",
    "Object", "Single", "String", "Variant",
];

pub(crate) const QUALIFIERS: &[&str] = &[
    "VBA",
    "Strings",
    "Math",
    "DateTime",
    "Information",
    "Conversion",
    "Financial",
    "Interaction",
];

fn kw(label: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: CompletionKind::Keyword,
        detail: None,
    }
}

/// Keywords that can follow a given word, and whether an expression may
/// follow as well.
fn follows(prev: &str) -> Option<(&'static [&'static str], bool)> {
    Some(match prev {
        "on" => (&["Error"], false),
        "error" => (&["GoTo", "Resume"], false),
        "resume" => (&["Next"], false),
        "option" => (&["Explicit", "Base", "Compare", "Private"], false),
        "compare" => (&["Binary", "Text", "Database"], false),
        "property" => (&["Get", "Let", "Set"], false),
        "select" => (&["Case"], false),
        "do" | "loop" => (&["While", "Until"], false),
        "case" => (&["Else", "Is"], true),
        "for" => (&["Each"], true),
        "dim" | "redim" | "const" | "static" | "preserve" => (&[], false),
        "set" | "let" | "call" | "if" | "elseif" | "while" | "until" | "return" | "with"
        | "print" | "write" | "to" | "step" | "in" => (&[], true),
        _ => return None,
    })
}

impl View<'_> {
    /// Completion candidates at the cursor, filtered by the identifier
    /// prefix already typed. Nothing is offered inside comments and string
    /// literals.
    pub fn completions(&self, pos: Position) -> Completions {
        let offset = self.offset(pos);
        let tokens = self.tokens();
        let here = Range::new(pos, pos);
        let nothing = Completions {
            replace: here,
            items: Vec::new(),
        };

        // The token the cursor is in or directly after.
        let end = tokens.partition_point(|t| t.span().start < offset);
        let mut word: Option<usize> = None;
        if end > 0 {
            let t = tokens[end - 1];
            let inside = t.span().end > offset;
            match t.kind() {
                TokenKind::Comment | TokenKind::RemComment => return nothing,
                TokenKind::StringLiteral
                | TokenKind::DateLiteral
                | TokenKind::UnterminatedString
                    if inside =>
                {
                    return nothing;
                }
                k if is_word(k) => word = Some(end - 1),
                _ => {}
            }
        }
        let (replace, prefix, before_end) = match word {
            Some(w) => {
                let t = tokens[w];
                let s = t.span();
                let upto = (offset.min(s.end) - s.start) as usize;
                let prefix = String::from_utf8_lossy(&t.text()[..upto]).to_ascii_lowercase();
                (self.range(s), prefix, w)
            }
            None => (here, String::new(), end),
        };
        let sig_before: Vec<usize> = (0..before_end)
            .rev()
            .filter(|&k| !tokens[k].kind().is_trivia())
            .take(3)
            .collect();
        let prev = sig_before.first().map(|&k| tokens[k]);
        let prev_kind = prev.map(|t| t.kind());
        let prev_word: Option<String> = prev
            .filter(|t| is_word(t.kind()))
            .map(|t| String::from_utf8_lossy(t.text()).to_ascii_lowercase());
        let scopes = self.scopes();
        let ancestors: Vec<NodeKind> = match prev {
            Some(t) => {
                let mut v = vec![t.parent()];
                while let Some(p) = v.last().and_then(|n| n.parent()) {
                    v.push(p);
                }
                v.iter().map(|n| n.kind()).collect()
            }
            None => vec![NodeKind::Module],
        };
        let in_proc =
            ancestors.contains(&NodeKind::Block) && ancestors.contains(&NodeKind::ProcDecl);

        let mut items: Vec<CompletionItem> = Vec::new();
        let add_symbols = |items: &mut Vec<CompletionItem>, only: &dyn Fn(SymKind) -> bool| {
            for i in scopes.visible_at(offset) {
                let s = &scopes.symbols[i as usize];
                if !only(s.kind) {
                    continue;
                }
                items.push(CompletionItem {
                    label: s.name.clone(),
                    kind: match s.kind {
                        SymKind::Proc(_) | SymKind::Declare | SymKind::Event => {
                            CompletionKind::Procedure
                        }
                        SymKind::Variable | SymKind::Param | SymKind::Label => {
                            CompletionKind::Variable
                        }
                        SymKind::Const => CompletionKind::Constant,
                        SymKind::Type | SymKind::Enum => CompletionKind::Type,
                        SymKind::EnumMember => CompletionKind::EnumMember,
                    },
                    detail: Some(s.detail.clone()),
                });
            }
        };
        let add_functions = |items: &mut Vec<CompletionItem>, module: Option<&str>| {
            for f in vba_stdlib::registry::functions() {
                if module.is_some_and(|m| !f.module.eq_ignore_ascii_case(m)) {
                    continue;
                }
                items.push(CompletionItem {
                    label: f.name.to_string(),
                    kind: CompletionKind::Function,
                    detail: Some(std_signature(f).label),
                });
            }
        };
        let expression = |items: &mut Vec<CompletionItem>| {
            items.extend(EXPRESSION_KEYWORDS.iter().map(|s| kw(s)));
            add_symbols(items, &|k| k != SymKind::Label);
            items.extend(self.semantic_global_items(false));
            add_functions(items, None);
        };
        let proc_word = |items: &mut Vec<CompletionItem>| {
            if let Some(p) = self.enclosing_proc_kind(prev) {
                items.push(kw(match p {
                    ProcKind::Sub => "Sub",
                    ProcKind::Function => "Function",
                    _ => "Property",
                }));
            }
        };

        match prev_kind {
            Some(TokenKind::Dot) => {
                let q = sig_before.get(1).map(|&k| tokens[k]);
                let typed = sig_before
                    .first()
                    .and_then(|&d| self.semantic_member_items(&tokens, d));
                if let Some(typed) = typed {
                    items.extend(typed);
                } else if let Some(q) = q.filter(|q| is_word(q.kind())) {
                    let name = String::from_utf8_lossy(q.text()).to_string();
                    let enum_sym = scopes
                        .lookup_at(q.span().start, &normalize(q.text()))
                        .filter(|&i| scopes.symbols[i as usize].kind == SymKind::Enum);
                    if let Some(e) = enum_sym {
                        let enum_norm = &scopes.symbols[e as usize].norm;
                        for s in &scopes.symbols {
                            if s.kind == SymKind::EnumMember
                                && s.container.as_deref() == Some(enum_norm.as_str())
                            {
                                items.push(CompletionItem {
                                    label: s.name.clone(),
                                    kind: CompletionKind::EnumMember,
                                    detail: Some(s.detail.clone()),
                                });
                            }
                        }
                    } else if name.eq_ignore_ascii_case("vba") {
                        items.extend(QUALIFIERS[1..].iter().map(|q| CompletionItem {
                            label: q.to_string(),
                            kind: CompletionKind::Module,
                            detail: None,
                        }));
                        add_functions(&mut items, None);
                    } else if QUALIFIERS.iter().any(|q| q.eq_ignore_ascii_case(&name)) {
                        add_functions(&mut items, Some(&name));
                    }
                }
            }
            Some(TokenKind::Keyword(Keyword::As | Keyword::New | Keyword::Implements)) => {
                items.extend(
                    TYPE_KEYWORDS
                        .iter()
                        .filter(|t| !self.type_hidden(t))
                        .map(|t| kw(t)),
                );
                add_symbols(&mut items, &|k| matches!(k, SymKind::Type | SymKind::Enum));
                items.extend(self.semantic_global_items(true));
            }
            Some(TokenKind::Keyword(Keyword::GoTo | Keyword::GoSub | Keyword::Resume)) => {
                let proc = scopes.proc_at(offset);
                for s in &scopes.symbols {
                    if s.kind == SymKind::Label
                        && proc.is_some_and(|p| s.scope == crate::scope::Scope::Proc(p))
                    {
                        items.push(CompletionItem {
                            label: s.name.clone(),
                            kind: CompletionKind::Variable,
                            detail: Some(s.detail.clone()),
                        });
                    }
                }
                if prev_word.as_deref() == Some("resume") {
                    items.push(kw("Next"));
                }
            }
            Some(TokenKind::Keyword(Keyword::End)) => {
                for a in &ancestors {
                    match a {
                        NodeKind::IfStmt => items.push(kw("If")),
                        NodeKind::SelectStmt => items.push(kw("Select")),
                        NodeKind::WithStmt => items.push(kw("With")),
                        NodeKind::TypeStmt => items.push(kw("Type")),
                        NodeKind::EnumStmt => items.push(kw("Enum")),
                        _ => {}
                    }
                }
                proc_word(&mut items);
            }
            Some(TokenKind::Keyword(Keyword::Exit)) => {
                if ancestors.contains(&NodeKind::ForStmt)
                    || ancestors.contains(&NodeKind::ForEachStmt)
                {
                    items.push(kw("For"));
                }
                if ancestors.contains(&NodeKind::DoStmt) {
                    items.push(kw("Do"));
                }
                proc_word(&mut items);
            }
            Some(TokenKind::Keyword(
                Keyword::Public | Keyword::Private | Keyword::Friend | Keyword::Global,
            )) if !in_proc => {
                items.extend(
                    [
                        "Sub",
                        "Function",
                        "Property",
                        "Const",
                        "Enum",
                        "Type",
                        "Declare",
                        "Event",
                        "Static",
                        "WithEvents",
                    ]
                    .iter()
                    .map(|s| kw(s)),
                );
            }
            Some(TokenKind::Keyword(Keyword::Declare)) => {
                items.push(kw("Sub"));
                items.push(kw("Function"));
                if self.profile().supports(Feature::PtrSafe) {
                    items.push(kw("PtrSafe"));
                }
            }
            _ if prev_word.as_deref().and_then(follows).is_some() => {
                let (list, expr) = prev_word
                    .as_deref()
                    .and_then(follows)
                    .unwrap_or((&[], false));
                items.extend(list.iter().map(|s| kw(s)));
                if expr {
                    expression(&mut items);
                }
            }
            _ => {
                let statement_start = matches!(
                    prev_kind,
                    None | Some(TokenKind::Newline | TokenKind::Colon)
                        | Some(TokenKind::Keyword(Keyword::Then | Keyword::Else))
                );
                let after_operand = matches!(
                    prev_kind,
                    Some(
                        TokenKind::Ident
                            | TokenKind::ForeignName
                            | TokenKind::RParen
                            | TokenKind::StringLiteral
                            | TokenKind::DateLiteral
                            | TokenKind::IntegerLiteral
                            | TokenKind::FloatLiteral
                            | TokenKind::HexLiteral
                            | TokenKind::OctalLiteral
                            | TokenKind::Keyword(
                                Keyword::True
                                    | Keyword::False
                                    | Keyword::Nothing
                                    | Keyword::Empty
                                    | Keyword::Null
                                    | Keyword::Me
                            )
                    )
                );
                if statement_start {
                    if in_proc {
                        items.extend(STATEMENT_KEYWORDS.iter().map(|s| kw(s)));
                        for a in &ancestors {
                            match a {
                                NodeKind::IfStmt => {
                                    items.push(kw("Else"));
                                    items.push(kw("ElseIf"));
                                }
                                NodeKind::SelectStmt | NodeKind::CaseClause => {
                                    items.push(kw("Case"))
                                }
                                NodeKind::ForStmt | NodeKind::ForEachStmt => items.push(kw("Next")),
                                NodeKind::DoStmt => items.push(kw("Loop")),
                                NodeKind::WhileStmt => items.push(kw("Wend")),
                                _ => {}
                            }
                        }
                        add_symbols(&mut items, &|k| k != SymKind::Label);
                        items.extend(self.semantic_global_items(false));
                        add_functions(&mut items, None);
                    } else {
                        items.extend(MODULE_KEYWORDS.iter().map(|s| kw(s)));
                    }
                } else if after_operand {
                    items.extend(OPERATOR_KEYWORDS.iter().map(|s| kw(s)));
                    if ancestors.iter().any(|a| {
                        matches!(
                            a,
                            NodeKind::VarDeclarator
                                | NodeKind::ConstItem
                                | NodeKind::Param
                                | NodeKind::TypeMember
                        )
                    }) {
                        items.push(kw("As"));
                    }
                } else if in_proc {
                    expression(&mut items);
                }
            }
        }

        // Filter by prefix and drop duplicates; a keyword that is also a
        // library function (`Date`, `String`) is listed once, as the function.
        items.retain(|i| i.label.to_ascii_lowercase().starts_with(&prefix));
        let named: HashSet<String> = items
            .iter()
            .filter(|i| i.kind != CompletionKind::Keyword)
            .map(|i| i.label.to_ascii_lowercase())
            .collect();
        items.retain(|i| {
            i.kind != CompletionKind::Keyword || !named.contains(&i.label.to_ascii_lowercase())
        });
        let mut seen = HashSet::new();
        items.retain(|i| seen.insert((i.label.to_ascii_lowercase(), i.kind)));
        items.sort_by(|a, b| {
            a.label
                .to_ascii_lowercase()
                .cmp(&b.label.to_ascii_lowercase())
                .then(a.label.cmp(&b.label))
        });
        Completions { replace, items }
    }

    fn type_hidden(&self, t: &str) -> bool {
        match t {
            "LongPtr" => !self.profile().supports(Feature::LongPtr),
            "LongLong" => !self.profile().supports(Feature::LongLong),
            _ => false,
        }
    }

    fn enclosing_proc_kind(&self, tok: Option<SyntaxToken<'_>>) -> Option<ProcKind> {
        let mut n = Some(tok?.parent());
        while let Some(x) = n {
            if x.kind() == NodeKind::ProcDecl {
                return proc_header(x).map(|(k, _)| k);
            }
            n = x.parent();
        }
        None
    }
}

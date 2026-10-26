//! The syntactic scope index.
//!
//! This is a name-based walk over the concrete syntax tree, not semantic
//! analysis: it knows the declarations of the file (procedures, module and
//! local variables, constants, parameters, user types, enums and their
//! members, `Declare`s, events, labels) and resolves identifier tokens to
//! them by case-insensitive name, innermost scope first. It does not know
//! types, so `obj.Member` is never resolved (only `Enum.Member`), references
//! into other modules are invisible, and `With` blocks are not tracked.
//! Full resolution arrives with `vba-sema`.

use std::collections::HashMap;

use vba_lexer::TokenKind;
use vba_source::{SourceEncoding, Span};
use vba_syntax::{NodeKind, SyntaxNode, SyntaxToken, SyntaxTree};

use crate::text::decode;
use crate::util::{ProcKind, decl_name, header_text, normalize, proc_header, suffix_len};

/// Where a symbol is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Module,
    Proc(u32),
}

/// What a symbol is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SymKind {
    Proc(ProcKind),
    Declare,
    Event,
    Variable,
    Const,
    Param,
    Type,
    Enum,
    EnumMember,
    Label,
}

/// One parameter of a declared procedure.
#[derive(Debug, Clone)]
pub(crate) struct ParamSig {
    pub name: String,
    pub text: String,
}

/// The signature of one declaration of a procedure-like symbol.
#[derive(Debug, Clone)]
pub(crate) struct SigData {
    pub keyword: &'static str,
    pub params: Vec<ParamSig>,
    pub ret: Option<String>,
}

/// A declared name.
#[derive(Debug, Clone)]
pub(crate) struct Symbol {
    pub name: String,
    pub norm: String,
    pub kind: SymKind,
    pub scope: Scope,
    /// Name spans of every declaration (suffix excluded).
    pub decls: Vec<Span>,
    pub detail: String,
    pub sigs: Vec<SigData>,
    pub container: Option<String>,
}

/// One use (or declaration) of a symbol.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Occurrence {
    pub span: Span,
    pub symbol: u32,
    pub is_decl: bool,
}

/// The index of one document version.
#[derive(Debug, Default)]
pub(crate) struct ScopeIndex {
    pub symbols: Vec<Symbol>,
    pub occurrences: Vec<Occurrence>,
    proc_spans: Vec<Span>,
}

type Key = (Scope, String, bool);

struct Builder<'a> {
    enc: SourceEncoding,
    symbols: Vec<Symbol>,
    keys: HashMap<Key, u32>,
    decl_at: HashMap<u32, u32>,
    skip: std::collections::HashSet<u32>,
    proc_spans: Vec<Span>,
    _tree: &'a SyntaxTree,
}

fn name_span(t: &SyntaxToken<'_>) -> Span {
    let s = t.span();
    Span::new(s.start, s.end - suffix_len(t))
}

impl Builder<'_> {
    fn dec(&self, b: &[u8]) -> String {
        decode(b, self.enc)
    }

    fn header(&self, n: SyntaxNode<'_>) -> String {
        header_text(n, |b| self.dec(b))
    }

    fn declare(
        &mut self,
        scope: Scope,
        tok: SyntaxToken<'_>,
        kind: SymKind,
        detail: String,
        sig: Option<SigData>,
        container: Option<String>,
    ) {
        let norm = normalize(tok.text());
        let is_label = kind == SymKind::Label;
        let span = name_span(&tok);
        let idx = match self.keys.get(&(scope, norm.clone(), is_label)) {
            Some(&i) => {
                self.symbols[i as usize].decls.push(span);
                i
            }
            None => {
                let i = self.symbols.len() as u32;
                let display = self.dec(&tok.text()[..(span.end - span.start) as usize]);
                self.symbols.push(Symbol {
                    name: display,
                    norm: norm.clone(),
                    kind,
                    scope,
                    decls: vec![span],
                    detail,
                    sigs: Vec::new(),
                    container,
                });
                self.keys.insert((scope, norm, is_label), i);
                i
            }
        };
        if let Some(s) = sig {
            self.symbols[idx as usize].sigs.push(s);
        }
        self.decl_at.insert(span.start, idx);
    }

    fn sig_of(&self, node: SyntaxNode<'_>, keyword: &'static str) -> SigData {
        let params = node
            .child_node(NodeKind::ParamList)
            .map(|pl| {
                pl.child_nodes()
                    .filter(|p| p.kind() == NodeKind::Param)
                    .filter_map(|p| {
                        let n = decl_name(p)?;
                        Some(ParamSig {
                            name: normalize(n.text()),
                            text: self.header(p),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let ret = node.child_node(NodeKind::AsClause).map(|a| self.header(a));
        SigData {
            keyword,
            params,
            ret,
        }
    }

    /// Text of the modifier keywords that lead a statement
    /// (`Private Const`, `Dim`, ...).
    fn stmt_prefix(&self, stmt: SyntaxNode<'_>) -> String {
        stmt.significant_tokens()
            .take_while(|t| matches!(t.kind(), TokenKind::Keyword(_)))
            .map(|t| self.dec(t.text()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn module_decl(&mut self, n: SyntaxNode<'_>) {
        match n.kind() {
            NodeKind::ProcDecl => {
                let Some((pk, name)) = proc_header(n) else {
                    return;
                };
                let idx = self.proc_spans.len() as u32;
                self.proc_spans.push(n.span());
                let sig = self.sig_of(n, pk.label());
                let detail = self.header(n);
                self.declare(
                    Scope::Module,
                    name,
                    SymKind::Proc(pk),
                    detail,
                    Some(sig),
                    None,
                );
                self.proc_locals(n, idx);
            }
            NodeKind::DeclareStmt => {
                if let Some((pk, name)) = proc_header(n) {
                    let sig = self.sig_of(n, pk.label());
                    let detail = self.header(n);
                    self.declare(
                        Scope::Module,
                        name,
                        SymKind::Declare,
                        detail,
                        Some(sig),
                        None,
                    );
                    self.skip_params(n);
                }
            }
            NodeKind::EventStmt => {
                if let Some(name) = decl_name(n) {
                    let sig = self.sig_of(n, "Event");
                    let detail = self.header(n);
                    self.declare(Scope::Module, name, SymKind::Event, detail, Some(sig), None);
                    self.skip_params(n);
                }
            }
            NodeKind::TypeStmt => {
                if let Some(name) = decl_name(n) {
                    let detail = self.header(n);
                    self.declare(Scope::Module, name, SymKind::Type, detail, None, None);
                    for m in n.child_nodes().filter(|m| m.kind() == NodeKind::TypeMember) {
                        if let Some(t) = decl_name(m) {
                            self.skip.insert(t.span().start);
                        }
                    }
                }
            }
            NodeKind::EnumStmt => {
                if let Some(name) = decl_name(n) {
                    let enum_norm = normalize(name.text());
                    let detail = self.header(n);
                    self.declare(Scope::Module, name, SymKind::Enum, detail, None, None);
                    for m in n.child_nodes().filter(|m| m.kind() == NodeKind::EnumMember) {
                        if let Some(t) = decl_name(m) {
                            let detail = format!("(enum member) {}", self.header(m));
                            self.declare(
                                Scope::Module,
                                t,
                                SymKind::EnumMember,
                                detail,
                                None,
                                Some(enum_norm.clone()),
                            );
                        }
                    }
                }
            }
            NodeKind::ConstStmt | NodeKind::VarDeclStmt => self.var_decls(Scope::Module, n),
            _ => {}
        }
    }

    fn var_decls(&mut self, scope: Scope, stmt: SyntaxNode<'_>) {
        let prefix = self.stmt_prefix(stmt);
        let (item_kind, kind) = if stmt.kind() == NodeKind::ConstStmt {
            (NodeKind::ConstItem, SymKind::Const)
        } else {
            (NodeKind::VarDeclarator, SymKind::Variable)
        };
        for d in stmt.child_nodes().filter(|d| d.kind() == item_kind) {
            if let Some(t) = decl_name(d) {
                let detail = format!("{prefix} {}", self.header(d));
                self.declare(scope, t, kind, detail, None, None);
            }
        }
    }

    fn skip_params(&mut self, n: SyntaxNode<'_>) {
        if let Some(pl) = n.child_node(NodeKind::ParamList) {
            for p in pl.child_nodes().filter(|p| p.kind() == NodeKind::Param) {
                if let Some(t) = decl_name(p) {
                    self.skip.insert(t.span().start);
                }
            }
        }
    }

    fn proc_locals(&mut self, n: SyntaxNode<'_>, idx: u32) {
        let scope = Scope::Proc(idx);
        if let Some(pl) = n.child_node(NodeKind::ParamList) {
            for p in pl.child_nodes().filter(|p| p.kind() == NodeKind::Param) {
                if let Some(t) = decl_name(p) {
                    let detail = format!("(parameter) {}", self.header(p));
                    self.declare(scope, t, SymKind::Param, detail, None, None);
                }
            }
        }
        for d in n.descendants() {
            match d.kind() {
                NodeKind::VarDeclStmt | NodeKind::ConstStmt => self.var_decls(scope, d),
                NodeKind::LabelStmt => {
                    if let Some(t) = decl_name(d) {
                        let detail = format!("{}:", self.dec(t.text()));
                        self.declare(scope, t, SymKind::Label, detail, None, None);
                    }
                }
                _ => {}
            }
        }
    }
}

fn is_label_context(k: NodeKind) -> bool {
    matches!(
        k,
        NodeKind::GoToStmt
            | NodeKind::GoSubStmt
            | NodeKind::OnErrorStmt
            | NodeKind::OnGoToStmt
            | NodeKind::ResumeStmt
    )
}

impl ScopeIndex {
    pub(crate) fn build(tree: &SyntaxTree, enc: SourceEncoding) -> ScopeIndex {
        let mut b = Builder {
            enc,
            symbols: Vec::new(),
            keys: HashMap::new(),
            decl_at: HashMap::new(),
            skip: Default::default(),
            proc_spans: Vec::new(),
            _tree: tree,
        };
        let root = tree.root();
        for n in root.child_nodes() {
            b.module_decl(n);
        }

        let tokens = root.descendant_tokens();
        let sig: Vec<usize> = (0..tokens.len())
            .filter(|&i| !tokens[i].kind().is_trivia())
            .collect();
        let mut occurrences = Vec::new();
        let mut resolved: HashMap<u32, u32> = HashMap::new();
        for (si, &i) in sig.iter().enumerate() {
            let t = tokens[i];
            let parent = t.parent();
            let pk = parent.kind();
            if pk == NodeKind::InactiveText {
                continue;
            }
            let start = t.span().start;
            if let Some(&sym) = b.decl_at.get(&start) {
                occurrences.push(Occurrence {
                    span: name_span(&t),
                    symbol: sym,
                    is_decl: true,
                });
                resolved.insert(start, sym);
                continue;
            }
            if b.skip.contains(&start) {
                continue;
            }
            let label_ctx = is_label_context(pk)
                && matches!(t.kind(), TokenKind::Ident | TokenKind::IntegerLiteral);
            if !label_ctx && !matches!(t.kind(), TokenKind::Ident | TokenKind::ForeignName) {
                continue;
            }
            if matches!(
                pk,
                NodeKind::ProcDecl
                    | NodeKind::DeclareStmt
                    | NodeKind::EventStmt
                    | NodeKind::TypeStmt
                    | NodeKind::EnumStmt
                    | NodeKind::OptionStmt
                    | NodeKind::AttributeStmt
                    | NodeKind::DefTypeStmt
                    | NodeKind::DefRange
                    | NodeKind::VersionLine
                    | NodeKind::HeaderBlock
                    | NodeKind::TypeMember
                    | NodeKind::EnumMember
                    | NodeKind::Param
                    | NodeKind::ConstItem
                    | NodeKind::VarDeclarator
                    | NodeKind::LabelStmt
            ) {
                continue;
            }
            let prev = si.checked_sub(1).map(|p| tokens[sig[p]]);
            let next = sig.get(si + 1).map(|&n| tokens[n]);
            if next.is_some_and(|n| n.kind() == TokenKind::ColonEq) && pk == NodeKind::Arg {
                continue;
            }
            let norm = normalize(t.text());
            let proc = proc_index(&b.proc_spans, start);
            let found = if label_ctx {
                proc.and_then(|p| b.keys.get(&(Scope::Proc(p), norm.clone(), true)))
                    .copied()
            } else if let Some(p) =
                prev.filter(|p| matches!(p.kind(), TokenKind::Dot | TokenKind::Bang))
            {
                // Only `Enum.Member` resolves.
                let before = si.checked_sub(2).map(|p| tokens[sig[p]]);
                match (p.kind(), before.and_then(|q| resolved.get(&q.span().start))) {
                    (TokenKind::Dot, Some(&e)) if b.symbols[e as usize].kind == SymKind::Enum => b
                        .keys
                        .get(&(Scope::Module, norm.clone(), false))
                        .copied()
                        .filter(|&m| {
                            let s = &b.symbols[m as usize];
                            s.kind == SymKind::EnumMember
                                && s.container.as_deref() == Some(&b.symbols[e as usize].norm)
                        }),
                    _ => None,
                }
            } else {
                proc.and_then(|p| b.keys.get(&(Scope::Proc(p), norm.clone(), false)))
                    .or_else(|| b.keys.get(&(Scope::Module, norm.clone(), false)))
                    .copied()
            };
            if let Some(sym) = found {
                occurrences.push(Occurrence {
                    span: name_span(&t),
                    symbol: sym,
                    is_decl: false,
                });
                resolved.insert(start, sym);
            }
        }
        ScopeIndex {
            symbols: b.symbols,
            occurrences,
            proc_spans: b.proc_spans,
        }
    }

    /// The occurrence whose span touches `offset` (end inclusive).
    pub(crate) fn occurrence_at(&self, offset: u32) -> Option<&Occurrence> {
        let i = self.occurrences.partition_point(|o| o.span.end < offset);
        self.occurrences
            .get(i)
            .filter(|o| o.span.start <= offset && offset <= o.span.end)
    }

    /// Occurrences of one symbol in source order.
    pub(crate) fn occurrences_of(&self, symbol: u32) -> impl Iterator<Item = &Occurrence> {
        self.occurrences.iter().filter(move |o| o.symbol == symbol)
    }

    /// Index of the procedure containing `offset`.
    pub(crate) fn proc_at(&self, offset: u32) -> Option<u32> {
        proc_index(&self.proc_spans, offset)
    }

    /// Symbols visible at `offset` (locals shadow module symbols), labels
    /// excluded, in declaration order.
    pub(crate) fn visible_at(&self, offset: u32) -> Vec<u32> {
        let proc = self.proc_at(offset);
        let mut seen: std::collections::HashSet<&str> = Default::default();
        let mut out = Vec::new();
        let locals = self.symbols.iter().enumerate().filter(|(_, s)| {
            s.kind != SymKind::Label && proc.is_some_and(|p| s.scope == Scope::Proc(p))
        });
        let globals = self
            .symbols
            .iter()
            .enumerate()
            .filter(|(_, s)| s.scope == Scope::Module);
        for (i, s) in locals.chain(globals) {
            if seen.insert(&s.norm) {
                out.push(i as u32);
            }
        }
        out
    }

    /// Looks a name up as seen from `offset`.
    pub(crate) fn lookup_at(&self, offset: u32, norm: &str) -> Option<u32> {
        self.visible_at(offset)
            .into_iter()
            .find(|&i| self.symbols[i as usize].norm == norm)
    }
}

fn proc_index(spans: &[Span], offset: u32) -> Option<u32> {
    let i = spans.partition_point(|s| s.end <= offset);
    spans.get(i).filter(|s| s.start <= offset).map(|_| i as u32)
}

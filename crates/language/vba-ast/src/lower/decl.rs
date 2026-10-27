//! Declaration lowering (MS-VBAL §4.2, §5.2, §5.3).

use vba_lexer::{Keyword, TokenKind};
use vba_source::Span;
use vba_syntax::{NodeKind, SyntaxNode, SyntaxToken};

use super::{
    AST_INCOMPLETE, Lowerer, expr_nodes, ident, is_kw, is_name_token, nodes_of, toks, visibility_of,
};
use crate::ast::*;
use crate::name::Ident;

fn missing_ident(span: Span) -> Ident {
    Ident::missing(Span::empty(span.end))
}

impl Lowerer<'_> {
    /// Lowers one child of the `Module` node.
    pub(crate) fn item(&mut self, n: SyntaxNode<'_>) -> Option<Item> {
        Some(match n.kind() {
            NodeKind::OptionStmt => Item::Option(self.option(n)),
            NodeKind::AttributeStmt => Item::Attribute(self.attribute(n)),
            NodeKind::DefTypeStmt => Item::DefType(self.def_type(n)),
            NodeKind::ImplementsStmt => Item::Implements(ImplementsDecl {
                span: n.span(),
                interface: self.type_ref_child(n),
            }),
            NodeKind::DeclareStmt => Item::Declare(self.declare(n)),
            NodeKind::EventStmt => Item::Event(self.event(n)),
            NodeKind::ConstStmt => Item::Const(self.const_decl(n)),
            NodeKind::VarDeclStmt => Item::Var(self.var_decl(n)),
            NodeKind::TypeStmt => Item::Type(self.type_decl(n)),
            NodeKind::EnumStmt => Item::Enum(self.enum_decl(n)),
            NodeKind::ProcDecl => Item::Proc(self.proc_decl(n)),
            NodeKind::InactiveText => Item::Inactive(n.span()),
            NodeKind::Error => Item::Error(n.span()),
            k => {
                if let Some(d) = self.directive(n) {
                    Item::Directive(d)
                } else {
                    self.report(
                        AST_INCOMPLETE,
                        n.span(),
                        format!("{k:?} is not allowed at module level"),
                    );
                    Item::Error(n.span())
                }
            }
        })
    }

    fn option(&mut self, n: SyntaxNode<'_>) -> OptionDecl {
        let t = toks(n);
        let word = |i: usize| t.get(i).map(|x| x.text().to_ascii_lowercase());
        let kind = match (word(1).as_deref(), word(2).as_deref()) {
            (Some(b"explicit"), _) => Some(OptionKind::Explicit),
            (Some(b"base"), Some(b"0")) => Some(OptionKind::Base(0)),
            (Some(b"base"), Some(b"1")) => Some(OptionKind::Base(1)),
            (Some(b"compare"), Some(b"binary")) => Some(OptionKind::Compare(CompareMode::Binary)),
            (Some(b"compare"), Some(b"text")) => Some(OptionKind::Compare(CompareMode::Text)),
            (Some(b"compare"), Some(b"database")) => {
                Some(OptionKind::Compare(CompareMode::Database))
            }
            (Some(b"private"), Some(b"module")) => Some(OptionKind::PrivateModule),
            _ => None,
        };
        if kind.is_none() {
            self.report(AST_INCOMPLETE, n.span(), "malformed Option statement");
        }
        OptionDecl {
            span: n.span(),
            kind,
        }
    }

    pub(crate) fn attribute(&mut self, n: SyntaxNode<'_>) -> AttributeDecl {
        let t = toks(n);
        // Names between `Attribute` and `=`, split by `.`.
        let names: Vec<SyntaxToken<'_>> = t
            .iter()
            .skip(1)
            .take_while(|x| x.kind() != TokenKind::Eq)
            .filter(|x| x.kind() != TokenKind::Dot)
            .copied()
            .collect();
        let (object, name) = match names.as_slice() {
            [name] => (None, ident(*name)),
            [obj, name, ..] => (Some(ident(*obj)), ident(*name)),
            [] => {
                self.report(AST_INCOMPLETE, n.span(), "expected an attribute name");
                (None, missing_ident(n.span()))
            }
        };
        let values = expr_nodes(n).into_iter().map(|e| self.expr(e)).collect();
        AttributeDecl {
            span: n.span(),
            object,
            name,
            values,
        }
    }

    fn def_type(&mut self, n: SyntaxNode<'_>) -> DefTypeDecl {
        let ty = match toks(n).first().map(|t| t.kind()) {
            Some(TokenKind::Keyword(k)) => match k {
                Keyword::DefBool => DefType::Bool,
                Keyword::DefByte => DefType::Byte,
                Keyword::DefCur => DefType::Cur,
                Keyword::DefDate => DefType::Date,
                Keyword::DefDbl => DefType::Dbl,
                Keyword::DefInt => DefType::Int,
                Keyword::DefLng => DefType::Lng,
                Keyword::DefLngLng => DefType::LngLng,
                Keyword::DefLngPtr => DefType::LngPtr,
                Keyword::DefObj => DefType::Obj,
                Keyword::DefSng => DefType::Sng,
                Keyword::DefStr => DefType::Str,
                Keyword::DefDec => DefType::Dec,
                _ => DefType::Var,
            },
            _ => DefType::Var,
        };
        let mut ranges = Vec::new();
        for r in nodes_of(n, NodeKind::DefRange) {
            let letters: Vec<u8> = toks(r)
                .iter()
                .filter(|t| t.kind() == TokenKind::Ident && t.text().len() == 1)
                .map(|t| t.text()[0].to_ascii_uppercase())
                .collect();
            match letters.as_slice() {
                [a] => ranges.push((*a, *a)),
                [a, b] => ranges.push((*a, *b)),
                _ => self.report(
                    AST_INCOMPLETE,
                    r.span(),
                    "expected a letter or letter range",
                ),
            }
        }
        DefTypeDecl {
            span: n.span(),
            ty,
            ranges,
        }
    }

    fn visibility(&self, n: SyntaxNode<'_>) -> Option<Visibility> {
        toks(n).iter().find_map(visibility_of)
    }

    /// The first name token after the token satisfying `after`.
    fn name_after<'t>(
        &mut self,
        n: SyntaxNode<'t>,
        after: impl Fn(&SyntaxToken<'t>) -> bool,
    ) -> Ident {
        let t = toks(n);
        let pos = t.iter().position(&after);
        match pos
            .and_then(|p| t.get(p + 1..))
            .and_then(|rest| rest.iter().find(|x| is_name_token(x.kind())))
        {
            Some(x) => ident(*x),
            None => {
                self.report(AST_INCOMPLETE, n.span(), "expected a name");
                missing_ident(n.span())
            }
        }
    }

    pub(crate) fn params(&mut self, n: SyntaxNode<'_>) -> Vec<Param> {
        let Some(list) = n.child_node(NodeKind::ParamList) else {
            return Vec::new();
        };
        nodes_of(list, NodeKind::Param)
            .into_iter()
            .map(|p| self.param(p))
            .collect()
    }

    fn param(&mut self, p: SyntaxNode<'_>) -> Param {
        let t = toks(p);
        let name = match t.iter().find(|x| is_name_token(x.kind())) {
            Some(x) => ident(*x),
            None => {
                self.report(AST_INCOMPLETE, p.span(), "expected a parameter name");
                missing_ident(p.span())
            }
        };
        let pass_by = t.iter().find_map(|x| match x.kind() {
            TokenKind::Keyword(Keyword::ByVal) => Some(PassBy::ByVal),
            TokenKind::Keyword(Keyword::ByRef) => Some(PassBy::ByRef),
            _ => None,
        });
        Param {
            span: p.span(),
            optional: t.iter().any(|x| is_kw(x, Keyword::Optional)),
            pass_by,
            param_array: t.iter().any(|x| is_kw(x, Keyword::ParamArray)),
            name,
            is_array: t.iter().any(|x| x.kind() == TokenKind::LParen),
            as_clause: self.as_clause_opt(p),
            default: self.expr_opt(p, 0),
        }
    }

    fn declare(&mut self, n: SyntaxNode<'_>) -> DeclareDecl {
        let t = toks(n);
        let kind = if t.iter().any(|x| is_kw(x, Keyword::Function)) {
            DeclareKind::Function
        } else {
            DeclareKind::Sub
        };
        let name = self.name_after(n, |x| is_kw(x, Keyword::Sub) || is_kw(x, Keyword::Function));
        let string_after = |word: &str| -> Option<Vec<u8>> {
            let p = t
                .iter()
                .position(|x| x.kind() == TokenKind::Ident && x.text_eq(word))?;
            let s = t.get(p + 1)?;
            (s.kind() == TokenKind::StringLiteral).then(|| unquote(s.text()))
        };
        DeclareDecl {
            span: n.span(),
            visibility: self.visibility(n),
            ptr_safe: t.iter().any(|x| {
                is_kw(x, Keyword::PtrSafe) || (x.kind() == TokenKind::Ident && x.text_eq("ptrsafe"))
            }),
            kind,
            name,
            lib: string_after("lib").unwrap_or_default(),
            alias: string_after("alias"),
            params: self.params(n),
            return_type: self.as_clause_opt(n),
        }
    }

    fn event(&mut self, n: SyntaxNode<'_>) -> EventDecl {
        EventDecl {
            span: n.span(),
            visibility: self.visibility(n),
            name: self.name_after(n, |x| is_kw(x, Keyword::Event)),
            params: self.params(n),
        }
    }

    pub(crate) fn const_decl(&mut self, n: SyntaxNode<'_>) -> ConstDecl {
        let items = nodes_of(n, NodeKind::ConstItem)
            .into_iter()
            .map(|c| {
                let name = match toks(c).iter().find(|x| is_name_token(x.kind())) {
                    Some(x) => ident(*x),
                    None => {
                        self.report(AST_INCOMPLETE, c.span(), "expected a constant name");
                        missing_ident(c.span())
                    }
                };
                // The as-clause may hold an expression (fixed length), so
                // the value is the expression child of the item itself.
                ConstItem {
                    span: c.span(),
                    name,
                    as_clause: self.as_clause_opt(c),
                    value: self.expr_child(c, 0),
                }
            })
            .collect();
        ConstDecl {
            span: n.span(),
            visibility: self.visibility(n),
            items,
        }
    }

    pub(crate) fn var_decl(&mut self, n: SyntaxNode<'_>) -> VarDecl {
        let t = toks(n);
        let keyword = t
            .iter()
            .find_map(|x| match x.kind() {
                TokenKind::Keyword(Keyword::Dim) => Some(VarKeyword::Dim),
                TokenKind::Keyword(Keyword::Private) => Some(VarKeyword::Private),
                TokenKind::Keyword(Keyword::Public) => Some(VarKeyword::Public),
                TokenKind::Keyword(Keyword::Global) => Some(VarKeyword::Global),
                TokenKind::Keyword(Keyword::Static) => Some(VarKeyword::Static),
                _ => None,
            })
            .unwrap_or(VarKeyword::Dim);
        let declarators = nodes_of(n, NodeKind::VarDeclarator)
            .into_iter()
            .map(|d| self.declarator(d))
            .collect();
        VarDecl {
            span: n.span(),
            keyword,
            visibility: self.visibility(n),
            is_static: t.iter().any(|x| is_kw(x, Keyword::Static)),
            shared: t.iter().any(|x| is_kw(x, Keyword::Shared)),
            with_events: t.iter().any(|x| is_kw(x, Keyword::WithEvents)),
            declarators,
        }
    }

    pub(crate) fn declarator(&mut self, d: SyntaxNode<'_>) -> Declarator {
        let is_member = d.kind() == NodeKind::TypeMember;
        let name = match toks(d).iter().find(|x| {
            is_name_token(x.kind()) || (is_member && matches!(x.kind(), TokenKind::Keyword(_)))
        }) {
            Some(x) => ident(*x),
            None => {
                self.report(AST_INCOMPLETE, d.span(), "expected a name");
                missing_ident(d.span())
            }
        };
        let bounds = d.child_node(NodeKind::ArgList).map(|l| self.bounds(l));
        Declarator {
            span: d.span(),
            name,
            bounds,
            as_clause: self.as_clause_opt(d),
        }
    }

    fn type_decl(&mut self, n: SyntaxNode<'_>) -> TypeDecl {
        let members = nodes_of(n, NodeKind::TypeMember)
            .into_iter()
            .map(|m| self.declarator(m))
            .collect();
        TypeDecl {
            span: n.span(),
            visibility: self.visibility(n),
            name: self.name_after(n, |x| is_kw(x, Keyword::Type)),
            members,
            directives: n.child_nodes().filter_map(|c| self.directive(c)).collect(),
        }
    }

    fn enum_decl(&mut self, n: SyntaxNode<'_>) -> EnumDecl {
        let members = nodes_of(n, NodeKind::EnumMember)
            .into_iter()
            .map(|m| {
                let name = match toks(m).iter().find(|x| is_name_token(x.kind())) {
                    Some(x) => ident(*x),
                    None => missing_ident(m.span()),
                };
                EnumMember {
                    span: m.span(),
                    name,
                    value: self.expr_opt(m, 0),
                }
            })
            .collect();
        EnumDecl {
            span: n.span(),
            visibility: self.visibility(n),
            name: self.name_after(n, |x| is_kw(x, Keyword::Enum)),
            members,
            directives: n.child_nodes().filter_map(|c| self.directive(c)).collect(),
        }
    }

    fn proc_decl(&mut self, n: SyntaxNode<'_>) -> ProcDecl {
        let t = toks(n);
        let kind_pos = t.iter().position(|x| {
            is_kw(x, Keyword::Sub)
                || is_kw(x, Keyword::Function)
                || (x.kind() == TokenKind::Ident && x.text_eq("property"))
        });
        let kind = match kind_pos.map(|p| &t[p]) {
            Some(x) if is_kw(x, Keyword::Sub) => ProcKind::Sub,
            Some(x) if is_kw(x, Keyword::Function) => ProcKind::Function,
            _ => match kind_pos.and_then(|p| t.get(p + 1)).map(|x| x.kind()) {
                Some(TokenKind::Keyword(Keyword::Let)) => ProcKind::PropertyLet,
                Some(TokenKind::Keyword(Keyword::Set)) => ProcKind::PropertySet,
                _ => ProcKind::PropertyGet,
            },
        };
        let skip = if matches!(kind, ProcKind::Sub | ProcKind::Function) {
            1
        } else {
            2
        };
        let name = match kind_pos.and_then(|p| t.get(p + skip)) {
            Some(x) if is_name_token(x.kind()) => ident(*x),
            _ => {
                self.report(AST_INCOMPLETE, n.span(), "expected a procedure name");
                missing_ident(n.span())
            }
        };
        let body = match n.child_node(NodeKind::Block) {
            Some(b) => self.block(b),
            None => Block {
                span: Span::empty(n.span().end),
                stmts: Vec::new(),
            },
        };
        ProcDecl {
            span: n.span(),
            visibility: t.iter().take(kind_pos.unwrap_or(0)).find_map(visibility_of),
            is_static: t
                .iter()
                .take(kind_pos.unwrap_or(0))
                .any(|x| is_kw(x, Keyword::Static)),
            kind,
            name,
            params: self.params(n),
            return_type: self.as_clause_opt(n),
            body,
        }
    }
}

/// Removes the quotes of a string literal token and collapses `""`.
pub(crate) fn unquote(text: &[u8]) -> Vec<u8> {
    let inner = if text.len() >= 2 && text[text.len() - 1] == b'"' {
        &text[1..text.len() - 1]
    } else {
        &text[1.min(text.len())..]
    };
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        out.push(inner[i]);
        if inner[i] == b'"' && inner.get(i + 1) == Some(&b'"') {
            i += 1;
        }
        i += 1;
    }
    out
}

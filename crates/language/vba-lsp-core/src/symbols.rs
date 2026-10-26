//! Document symbols (outline).

use vba_syntax::{NodeKind, SyntaxNode};

use crate::types::{DocumentSymbol, SymbolKind};
use crate::util::{ProcKind, decl_name, header_text, proc_header};
use crate::workspace::View;

impl View<'_> {
    /// The outline: procedures, types, enums, constants, variables,
    /// declares and events, with type/enum members and procedure locals as
    /// children. Declarations in inactive `#If` branches are not parsed and
    /// therefore not listed.
    pub fn document_symbols(&self) -> Vec<DocumentSymbol> {
        let mut out = Vec::new();
        for n in self.tree().root().child_nodes() {
            self.collect_symbols(n, true, &mut out);
        }
        out
    }

    fn collect_symbols(
        &self,
        n: SyntaxNode<'_>,
        module_level: bool,
        out: &mut Vec<DocumentSymbol>,
    ) {
        match n.kind() {
            NodeKind::ProcDecl => {
                let Some((pk, name)) = proc_header(n) else {
                    return;
                };
                let kind = match pk {
                    ProcKind::Sub => SymbolKind::Method,
                    ProcKind::Function => SymbolKind::Function,
                    _ => SymbolKind::Property,
                };
                let mut children = Vec::new();
                for d in n.descendants() {
                    if matches!(d.kind(), NodeKind::VarDeclStmt | NodeKind::ConstStmt) {
                        self.collect_symbols(d, false, &mut children);
                    }
                }
                out.push(self.symbol(n, name, kind, self.header(n), children));
            }
            NodeKind::DeclareStmt => {
                if let Some((_, name)) = proc_header(n) {
                    out.push(self.symbol(n, name, SymbolKind::Function, self.header(n), vec![]));
                }
            }
            NodeKind::EventStmt => {
                if let Some(name) = decl_name(n) {
                    out.push(self.symbol(n, name, SymbolKind::Event, self.header(n), vec![]));
                }
            }
            NodeKind::TypeStmt => {
                if let Some(name) = decl_name(n) {
                    let children = n
                        .child_nodes()
                        .filter(|m| m.kind() == NodeKind::TypeMember)
                        .filter_map(|m| {
                            let t = decl_name(m)?;
                            Some(self.symbol(m, t, SymbolKind::Field, self.as_text(m), vec![]))
                        })
                        .collect();
                    out.push(self.symbol(n, name, SymbolKind::Struct, self.header(n), children));
                }
            }
            NodeKind::EnumStmt => {
                if let Some(name) = decl_name(n) {
                    let children = n
                        .child_nodes()
                        .filter(|m| m.kind() == NodeKind::EnumMember)
                        .filter_map(|m| {
                            let t = decl_name(m)?;
                            Some(self.symbol(m, t, SymbolKind::EnumMember, String::new(), vec![]))
                        })
                        .collect();
                    out.push(self.symbol(n, name, SymbolKind::Enum, self.header(n), children));
                }
            }
            NodeKind::ConstStmt => {
                for item in n.child_nodes().filter(|m| m.kind() == NodeKind::ConstItem) {
                    if let Some(t) = decl_name(item) {
                        out.push(self.symbol(
                            item,
                            t,
                            SymbolKind::Constant,
                            self.as_text(item),
                            vec![],
                        ));
                    }
                }
            }
            NodeKind::VarDeclStmt => {
                let kind = if module_level {
                    SymbolKind::Field
                } else {
                    SymbolKind::Variable
                };
                for d in n
                    .child_nodes()
                    .filter(|m| m.kind() == NodeKind::VarDeclarator)
                {
                    if let Some(t) = decl_name(d) {
                        out.push(self.symbol(d, t, kind, self.as_text(d), vec![]));
                    }
                }
            }
            _ => {}
        }
    }

    fn header(&self, n: SyntaxNode<'_>) -> String {
        let enc = self.document().text_map().encoding();
        header_text(n, |b| crate::text::decode(b, enc))
    }

    /// The `As ...` clause text of a declarator, or `As Variant`.
    fn as_text(&self, n: SyntaxNode<'_>) -> String {
        match n.child_node(NodeKind::AsClause) {
            Some(a) => self.header(a),
            None => "As Variant".to_string(),
        }
    }

    fn symbol(
        &self,
        decl: SyntaxNode<'_>,
        name: vba_syntax::SyntaxToken<'_>,
        kind: SymbolKind,
        detail: String,
        children: Vec<DocumentSymbol>,
    ) -> DocumentSymbol {
        DocumentSymbol {
            name: self.text(name.span()),
            detail,
            kind,
            range: self.range(decl.span()),
            selection_range: self.range(name.span()),
            children,
        }
    }
}

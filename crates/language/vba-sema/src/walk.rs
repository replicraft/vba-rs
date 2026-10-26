//! Statement traversal helpers over the AST.

use vba_ast::{Block, Stmt, StmtKind};

/// Calls `f` for every statement of `block`, nested blocks included, in
/// source (pre-)order.
pub fn walk_block(block: &Block, f: &mut dyn FnMut(&Stmt)) {
    for s in &block.stmts {
        f(s);
        match &s.kind {
            StmtKind::If(i) => {
                walk_block(&i.then_body, f);
                for e in &i.else_ifs {
                    walk_block(&e.body, f);
                }
                if let Some(b) = &i.else_body {
                    walk_block(b, f);
                }
            }
            StmtKind::Select(sel) => {
                for c in &sel.cases {
                    walk_block(&c.body, f);
                }
            }
            StmtKind::For(x) => walk_block(&x.body, f),
            StmtKind::ForEach(x) => walk_block(&x.body, f),
            StmtKind::Do(x) => walk_block(&x.body, f),
            StmtKind::While { body, .. } | StmtKind::With { body, .. } => walk_block(body, f),
            _ => {}
        }
    }
}

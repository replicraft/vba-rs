//! The Phase 2 consistency score: do the operands of the decoded
//! instructions of a line agree with the tokens of the source line?
//!
//! A source line is a *statement* when it has at least one significant token
//! or a comment. A statement is *consistent* when
//!
//! * forward: every comment operand equals the source comment, every string
//!   operand equals the content of a string literal of the line, every
//!   integer operand equals a numeric literal of the line, and every
//!   resolved name operand (user identifier, confirmed built-in, `Me`)
//!   equals an identifier or keyword token of the line (case-insensitive,
//!   type suffix and brackets ignored); and
//! * reverse: the comment, every string literal, and every identifier token
//!   that is a project identifier all occur among the operands.
//!
//! Name operands that cannot be resolved (unknown built-in index) are
//! neither pass nor fail; a statement with such operands is consistent in
//! the lenient score and not in the strict score.

#![allow(dead_code, missing_docs)]

use std::collections::BTreeSet;

use vba_lexer::TokenKind;
use vba_pcode::{Disassembler, ModuleCache, Name, Operand, Radix, ResolvedType};

use super::SrcLine;

#[derive(Debug, Default, Clone)]
pub struct Score {
    pub statements: usize,
    pub consistent: usize,
    pub strict: usize,
    pub no_code: usize,
    pub forward_fail: usize,
    pub reverse_miss: usize,
    pub unresolved_names: usize,
    pub name_ops: usize,
    pub name_ops_ok: usize,
    pub text_ops: usize,
    pub text_ops_ok: usize,
    pub comment_ops: usize,
    pub comment_ops_ok: usize,
    pub int_ops: usize,
    pub int_ops_ok: usize,
    pub failures: Vec<String>,
}

impl Score {
    pub fn add(&mut self, o: &Score) {
        self.statements += o.statements;
        self.consistent += o.consistent;
        self.strict += o.strict;
        self.no_code += o.no_code;
        self.forward_fail += o.forward_fail;
        self.reverse_miss += o.reverse_miss;
        self.unresolved_names += o.unresolved_names;
        self.name_ops += o.name_ops;
        self.name_ops_ok += o.name_ops_ok;
        self.text_ops += o.text_ops;
        self.text_ops_ok += o.text_ops_ok;
        self.comment_ops += o.comment_ops;
        self.comment_ops_ok += o.comment_ops_ok;
        self.int_ops += o.int_ops;
        self.int_ops_ok += o.int_ops_ok;
        self.failures.extend(o.failures.iter().cloned());
    }
}

fn strip_name(t: &[u8]) -> String {
    let s: String = t
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .to_lowercase();
    let s = s.trim_start_matches('[').trim_end_matches(']');
    s.trim_end_matches(|c| "%&!#@$".contains(c)).to_string()
}

fn unquote(t: &[u8]) -> Vec<u8> {
    let inner = &t[1..t.len().saturating_sub(1).max(1)];
    let mut out = Vec::new();
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

fn parse_int(kind: TokenKind, t: &[u8]) -> Option<i64> {
    let s = String::from_utf8_lossy(t).to_lowercase();
    let s = s.trim_end_matches(|c| "%&^".contains(c));
    match kind {
        TokenKind::IntegerLiteral => s.parse().ok(),
        TokenKind::HexLiteral => i64::from_str_radix(s.trim_start_matches("&h"), 16).ok(),
        TokenKind::OctalLiteral => {
            i64::from_str_radix(s.trim_start_matches("&o").trim_start_matches('&'), 8).ok()
        }
        _ => None,
    }
}

fn collect(op: &Operand, out: &mut Vec<Operand>) {
    match op {
        Operand::Procedure(p) => {
            out.push(Operand::Name {
                name: p.name.clone(),
                suffix: None,
                bracketed: false,
            });
            if let Some(l) = &p.library {
                out.push(Operand::Text(
                    l.spelling().unwrap_or("").as_bytes().to_vec(),
                ));
            }
            if let Some(e) = p
                .entry_point
                .as_ref()
                .filter(|e| p.name.spelling().map(str::as_bytes) != Some(e.as_slice()))
            {
                out.push(Operand::Text(e.clone()));
            }
            for a in &p.args {
                out.push(Operand::Name {
                    name: a.name.clone(),
                    suffix: None,
                    bracketed: false,
                });
                push_type(&a.ty, out);
            }
            push_type(&p.return_type, out);
        }
        Operand::Variable(v) => {
            out.push(Operand::Name {
                name: v.name.clone(),
                suffix: None,
                bracketed: false,
            });
            push_type(&v.ty, out);
        }
        Operand::Words(_) => {}
        Operand::Interface(t) => push_type(t, out),
        Operand::UserType { name, .. } => out.push(Operand::Name {
            name: name.clone(),
            suffix: None,
            bracketed: false,
        }),
        Operand::Object { class: Some(c), .. } => out.push(Operand::Name {
            name: c.clone(),
            suffix: None,
            bracketed: false,
        }),
        Operand::Object {
            qualified: Some(q), ..
        } => push_type(&ResolvedType::Qualified(q.clone()), out),
        Operand::NameList(v) => {
            for n in v {
                out.push(Operand::Name {
                    name: n.clone(),
                    suffix: None,
                    bracketed: false,
                });
            }
        }
        other => out.push(other.clone()),
    }
}

/// Scores one module. `user_idents` is the lower-cased set of project
/// identifiers.
pub fn score_module(
    name: &str,
    cache: &ModuleCache<'_>,
    dis: &Disassembler<'_>,
    src: &[SrcLine],
    user_idents: &BTreeSet<String>,
) -> Score {
    let mut sc = Score::default();
    for (i, sl) in src.iter().enumerate() {
        let is_statement = !sl.toks.is_empty() || sl.comment.is_some();
        if !is_statement {
            continue;
        }
        sc.statements += 1;
        let listing = dis.line(i);
        if i >= cache.lines.lines.len() || listing.instructions.is_empty() {
            sc.no_code += 1;
            sc.failures
                .push(format!("{name}:{i} no code for statement"));
            continue;
        }
        let mut ops = Vec::new();
        for ins in &listing.instructions {
            for o in &ins.operands {
                collect(o, &mut ops);
            }
        }
        // Source side.
        // Numeric tokens count as names too: numeric line labels are
        // identifiers in the project table.
        let idents: BTreeSet<String> = sl
            .toks
            .iter()
            .filter(|t| !matches!(t.kind, TokenKind::StringLiteral | TokenKind::DateLiteral))
            .map(|t| strip_name(&t.text))
            .collect();
        let strings: Vec<Vec<u8>> = sl
            .toks
            .iter()
            .filter(|t| t.kind == TokenKind::StringLiteral)
            .map(|t| unquote(&t.text))
            .collect();
        let ints: Vec<i64> = sl
            .toks
            .iter()
            .filter_map(|t| parse_int(t.kind, &t.text))
            .collect();
        let mut fwd_fail = Vec::new();
        let mut unresolved = 0usize;
        let mut seen_names: BTreeSet<String> = BTreeSet::new();
        let mut seen_text: Vec<Vec<u8>> = Vec::new();
        let mut seen_comment = false;
        for o in &ops {
            match o {
                Operand::Name { name: n, .. } => match n {
                    Name::User(_) | Name::Builtin(_) | Name::Me => {
                        sc.name_ops += 1;
                        let s = n.spelling().unwrap().to_lowercase();
                        if idents.contains(&s) {
                            sc.name_ops_ok += 1;
                            seen_names.insert(s);
                        } else {
                            fwd_fail.push(format!("name {s:?}"));
                        }
                    }
                    Name::None => {}
                    _ => {
                        unresolved += 1;
                        sc.unresolved_names += 1;
                    }
                },
                Operand::Text(t) => {
                    sc.text_ops += 1;
                    if strings.contains(t) {
                        sc.text_ops_ok += 1;
                        seen_text.push(t.clone());
                    } else {
                        fwd_fail.push(format!("text {:?}", String::from_utf8_lossy(t)));
                    }
                }
                Operand::Comment(t) => {
                    sc.comment_ops += 1;
                    if sl.comment.as_deref() == Some(t.as_slice()) {
                        sc.comment_ops_ok += 1;
                        seen_comment = true;
                    } else {
                        fwd_fail.push(format!("comment {:?}", String::from_utf8_lossy(t)));
                    }
                }
                Operand::Int { value, radix } => {
                    sc.int_ops += 1;
                    let _ = radix == &Radix::Decimal;
                    if ints.contains(value) {
                        sc.int_ops_ok += 1;
                    } else {
                        fwd_fail.push(format!("int {value}"));
                    }
                }
                _ => {}
            }
        }
        // Reverse.
        let mut rev_miss = Vec::new();
        if let Some(c) = sl.comment.as_ref().filter(|_| !seen_comment) {
            rev_miss.push(format!("comment {:?}", String::from_utf8_lossy(c)));
        }
        for s in &strings {
            if !seen_text.contains(s) {
                rev_miss.push(format!("string {:?}", String::from_utf8_lossy(s)));
            }
        }
        for t in &sl.toks {
            if t.kind == TokenKind::Ident || t.kind == TokenKind::ForeignName {
                let n = strip_name(&t.text);
                if user_idents.contains(&n) && !seen_names.contains(&n) {
                    rev_miss.push(format!("ident {n:?}"));
                }
            }
        }
        if !fwd_fail.is_empty() {
            sc.forward_fail += 1;
        }
        if !rev_miss.is_empty() {
            sc.reverse_miss += 1;
        }
        if fwd_fail.is_empty() && rev_miss.is_empty() {
            sc.consistent += 1;
            if unresolved == 0 {
                sc.strict += 1;
            }
        } else {
            sc.failures.push(format!(
                "{name}:{i} fwd={fwd_fail:?} rev={rev_miss:?} | {}",
                String::from_utf8_lossy(&sl.raw)
            ));
        }
    }
    sc
}

/// The names a resolved type contributes: the type, and the qualifier of a
/// library-qualified one.
fn push_type(t: &ResolvedType, out: &mut Vec<Operand>) {
    let name = |n: &Name| Operand::Name {
        name: n.clone(),
        suffix: None,
        bracketed: false,
    };
    match t {
        ResolvedType::Named(n) => out.push(name(n)),
        ResolvedType::Qualified(q) => {
            out.push(name(&q.qualifier));
            out.push(name(&q.name));
        }
        _ => {}
    }
}

//! Differential verification of operand order and stack effects on the local
//! material (ignored): the instruction stream of a statement is replayed on
//! an expression stack and compared with the canonical form of the source
//! expression.
#![allow(missing_docs)]
mod common;

use std::collections::BTreeMap;

use common::expr::{Ex, parse_source, replay};
use vba_lexer::{Keyword, TokenKind};
use vba_pcode::*;

fn kw(t: &common::Tok, k: Keyword) -> bool {
    t.kind == TokenKind::Keyword(k)
}

/// What the source line is, and the expression tokens it carries.
enum Shape<'a> {
    Assign(String, &'a [common::Tok]),
    If(&'a [common::Tok]),
    SingleIf(&'a [common::Tok]),
    ElseIf(&'a [common::Tok]),
    DoWhile(&'a [common::Tok]),
    DoUntil(&'a [common::Tok]),
    LoopWhile(&'a [common::Tok]),
    LoopUntil(&'a [common::Tok]),
    While(&'a [common::Tok]),
    SelectCase(&'a [common::Tok]),
}

fn shape(t: &[common::Tok]) -> Option<Shape<'_>> {
    let first = t.first()?;
    if kw(first, Keyword::If) && t.last().is_some_and(|l| kw(l, Keyword::Then)) {
        return Some(Shape::If(&t[1..t.len() - 1]));
    }
    if kw(first, Keyword::If)
        && let Some(pos) = t.iter().position(|x| kw(x, Keyword::Then))
        && pos + 1 < t.len()
    {
        return Some(Shape::SingleIf(&t[1..pos]));
    }
    if kw(first, Keyword::ElseIf) && t.last().is_some_and(|l| kw(l, Keyword::Then)) {
        return Some(Shape::ElseIf(&t[1..t.len() - 1]));
    }
    if kw(first, Keyword::Do) && t.len() > 2 {
        if kw(&t[1], Keyword::While) {
            return Some(Shape::DoWhile(&t[2..]));
        }
        if kw(&t[1], Keyword::Until) {
            return Some(Shape::DoUntil(&t[2..]));
        }
    }
    if kw(first, Keyword::Loop) && t.len() > 2 {
        if kw(&t[1], Keyword::While) {
            return Some(Shape::LoopWhile(&t[2..]));
        }
        if kw(&t[1], Keyword::Until) {
            return Some(Shape::LoopUntil(&t[2..]));
        }
    }
    if kw(first, Keyword::While) && t.len() > 1 {
        return Some(Shape::While(&t[1..]));
    }
    if kw(first, Keyword::Select) && t.len() > 3 && kw(&t[1], Keyword::Case) {
        return Some(Shape::SelectCase(&t[2..]));
    }
    if matches!(first.kind, TokenKind::Ident)
        && t.len() > 2
        && t[1].kind == TokenKind::Eq
        && !t.iter().any(|x| x.kind == TokenKind::Colon)
    {
        let n = String::from_utf8_lossy(&first.text)
            .to_lowercase()
            .trim_end_matches(|c| "%&!#@$".contains(c))
            .to_string();
        return Some(Shape::Assign(n, &t[2..]));
    }
    None
}

#[test]
#[ignore = "needs the local project (VBA_PCODE_PROJECT)"]
fn expression_streams_agree_with_source() {
    let Some(p) = common::load() else { return };
    let v = version_word(&p.project_stream).unwrap();
    let layout = Layout::select(v, p.sys_kind).unwrap();
    let idents = parse_identifiers(&p.project_stream, &layout).unwrap();
    let mut by_shape: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut op_ok: BTreeMap<Opcode, usize> = BTreeMap::new();
    let mut op_bad: BTreeMap<Opcode, usize> = BTreeMap::new();
    let mut fails: Vec<String> = Vec::new();
    for m in &p.modules {
        let cache = ModuleCache::parse(&m.cache, &layout).unwrap();
        let dis = Disassembler::new(&cache, &idents);
        let src = common::logical_lines(&m.source);
        for (i, sl) in src.iter().enumerate() {
            let Some(sh) = shape(&sl.toks) else { continue };
            let mut ins = dis.line(i).instructions;
            if ins.last().is_some_and(|x| x.opcode == Opcode::CommentQuote) {
                ins.pop();
            }
            // Single-statement lines only: the instruction list must not
            // contain statement separators.
            if ins.iter().any(|x| x.opcode == Opcode::StmtBegin) {
                continue;
            }
            let (name, toks, tail): (&str, &[common::Tok], &[Opcode]) = match &sh {
                Shape::Assign(_, t) => ("assign", t, &[Opcode::WriteSimple]),
                Shape::If(t) => ("if", t, &[Opcode::IfBlockOpen]),
                Shape::SingleIf(t) => ("single-if", t, &[Opcode::IfSingleLine]),
                Shape::ElseIf(t) => ("elseif", t, &[Opcode::BlockElseifBranch]),
                Shape::DoWhile(t) => ("do-while", t, &[Opcode::DoOpenWhile]),
                Shape::DoUntil(t) => ("do-until", t, &[Opcode::DoOpenUntil]),
                Shape::LoopWhile(t) => ("loop-while", t, &[Opcode::DoCloseWhile]),
                Shape::LoopUntil(t) => ("loop-until", t, &[Opcode::DoCloseUntil]),
                Shape::While(t) => ("while", t, &[Opcode::WhileOpen]),
                Shape::SelectCase(t) => ("select", t, &[Opcode::SwitchBegin]),
            };
            let Some(want) = parse_source(toks) else {
                continue;
            };
            let e = by_shape.entry(name).or_default();
            e.0 += 1;
            let got = replay(&ins);
            let ok = match &got {
                Some((st, k))
                    if st.len() == 1
                        && (ins.len() == k + 1 || matches!(sh, Shape::SingleIf(_)))
                        && ins.get(*k).is_some_and(|x| x.opcode == tail[0]) =>
                {
                    let name_ok = match (&sh, ins[*k].operands.first()) {
                        (Shape::Assign(n, _), Some(op)) => {
                            common::expr::name_of(op).as_deref() == Some(n.as_str())
                        }
                        _ => true,
                    };
                    name_ok && st[0] == want
                }
                _ => false,
            };
            if ok {
                e.1 += 1;
                for x in &ins {
                    *op_ok.entry(x.opcode).or_default() += 1;
                }
            } else {
                for x in &ins {
                    *op_bad.entry(x.opcode).or_default() += 1;
                }
                if fails.len() < 40 {
                    let g: Option<Vec<Ex>> = got.map(|g| g.0);
                    fails.push(format!(
                        "{name}: want {} got {:?} | {}",
                        want.show(),
                        g.map(|v| v.iter().map(Ex::show).collect::<Vec<_>>()),
                        String::from_utf8_lossy(&sl.raw).trim()
                    ));
                }
            }
        }
    }
    println!("statements compared (shape: total, agree): {by_shape:?}");
    for f in &fails {
        println!("FAIL {f}");
    }
    println!("opcodes in agreeing statements:");
    for (o, n) in &op_ok {
        println!(
            "  OK {:3} {:30} {}  (in failing: {})",
            o.canonical(),
            o.label(),
            n,
            op_bad.get(o).unwrap_or(&0)
        );
    }
}

//! More differential checks of operand order: `For`, `Case`, `Debug.Print`
//! and call statements (ignored; needs the local material).
#![allow(missing_docs)]
mod common;

use std::collections::BTreeMap;

use common::expr::{Ex, parse_source, replay};
use vba_lexer::{Keyword, TokenKind};
use vba_pcode::*;

fn is_kw(t: &common::Tok, k: Keyword) -> bool {
    t.kind == TokenKind::Keyword(k)
}

fn lc(t: &common::Tok) -> String {
    String::from_utf8_lossy(&t.text).to_lowercase()
}

/// Splits tokens at top-level occurrences of a separator predicate.
fn split_top<'a>(
    t: &'a [common::Tok],
    sep: &dyn Fn(&common::Tok) -> bool,
) -> Vec<&'a [common::Tok]> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, x) in t.iter().enumerate() {
        match x.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => depth -= 1,
            _ if depth == 0 && sep(x) => {
                out.push(&t[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&t[start..]);
    out
}

fn show(v: &[Ex]) -> Vec<String> {
    v.iter().map(Ex::show).collect()
}

#[test]
#[ignore = "needs the local project (VBA_PCODE_PROJECT)"]
fn for_case_print_call_streams_agree() {
    let Some(p) = common::load() else { return };
    let v = version_word(&p.project_stream).unwrap();
    let layout = Layout::select(v, p.sys_kind).unwrap();
    let idents = parse_identifiers(&p.project_stream, &layout).unwrap();
    let mut tally: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut ops_ok: BTreeMap<Opcode, usize> = BTreeMap::new();
    let mut fails = Vec::new();
    for m in &p.modules {
        let cache = ModuleCache::parse(&m.cache, &layout).unwrap();
        let dis = Disassembler::new(&cache, &idents);
        let src = common::logical_lines(&m.source);
        for (i, sl) in src.iter().enumerate() {
            let t = &sl.toks;
            if t.is_empty() || t.iter().any(|x| x.kind == TokenKind::Colon) {
                continue;
            }
            let mut ins = dis.line(i).instructions;
            if ins.last().is_some_and(|x| x.opcode == Opcode::CommentQuote) {
                ins.pop();
            }
            // ---- For v = a To b [Step s] ----
            if is_kw(&t[0], Keyword::For)
                && t.len() > 4
                && !is_kw(&t[1], Keyword::Each)
                && t[2].kind == TokenKind::Eq
            {
                let var = lc(&t[1])
                    .trim_end_matches(|c| "%&!#@$".contains(c))
                    .to_string();
                let body = &t[3..];
                let parts = split_top(body, &|x| is_kw(x, Keyword::To));
                if parts.len() != 2 {
                    continue;
                }
                let tail = split_top(parts[1], &|x| lc(x) == "step");
                let mut want: Vec<Ex> = Vec::new();
                let mut okp = true;
                for seg in [parts[0], tail[0]].into_iter().chain(tail.get(1).copied()) {
                    match parse_source(seg) {
                        Some(e) => want.push(e),
                        None => okp = false,
                    }
                }
                if !okp {
                    continue;
                }
                let e = tally.entry("for").or_default();
                e.0 += 1;
                let open_ok = ins.len() > 3
                    && ins[0].opcode == Opcode::ForVarExprOpen
                    && ins[1].opcode == Opcode::ReadSimple
                    && common::expr::name_of(&ins[1].operands[0]).as_deref() == Some(var.as_str())
                    && ins[2].opcode == Opcode::ForVarExprClose;
                let ok = open_ok
                    && replay(&ins[3..]).is_some_and(|(st, k)| {
                        st == want
                            && ins.len() == 3 + k + 1
                            && ins[3 + k].opcode
                                == if want.len() == 3 {
                                    Opcode::ForCountedStep
                                } else {
                                    Opcode::ForCounted
                                }
                    });
                if ok {
                    e.1 += 1;
                    for x in &ins {
                        *ops_ok.entry(x.opcode).or_default() += 1;
                    }
                } else if fails.len() < 20 {
                    fails.push(format!("for: {}", String::from_utf8_lossy(&sl.raw).trim()));
                }
                continue;
            }
            // ---- Case list ----
            if is_kw(&t[0], Keyword::Case) && !(t.len() > 1 && is_kw(&t[1], Keyword::Else)) {
                let items = split_top(&t[1..], &|x| x.kind == TokenKind::Comma);
                let mut want: Vec<(Opcode, Vec<Ex>)> = Vec::new();
                let mut okp = true;
                for it in items {
                    if it.first().is_some_and(|x| is_kw(x, Keyword::Is)) && it.len() > 2 {
                        let op = match it[1].kind {
                            TokenKind::Gt => Opcode::CaseIsGreater,
                            TokenKind::Lt => Opcode::CaseIsLess,
                            TokenKind::Ge => Opcode::CaseIsGreaterEqual,
                            TokenKind::Le => Opcode::CaseIsLessEqual,
                            TokenKind::Ne => Opcode::CaseIsNotEqual,
                            TokenKind::Eq => Opcode::CaseIsEqual,
                            _ => {
                                okp = false;
                                continue;
                            }
                        };
                        match parse_source(&it[2..]) {
                            Some(e) => want.push((op, vec![e])),
                            None => okp = false,
                        }
                        continue;
                    }
                    let parts = split_top(it, &|x| is_kw(x, Keyword::To));
                    let mut es = Vec::new();
                    for seg in &parts {
                        match parse_source(seg) {
                            Some(e) => es.push(e),
                            None => okp = false,
                        }
                    }
                    let op = if es.len() == 2 {
                        Opcode::CaseRange
                    } else {
                        Opcode::CaseValue
                    };
                    want.push((op, es));
                }
                if !okp {
                    continue;
                }
                let e = tally.entry("case").or_default();
                e.0 += 1;
                // Replay item by item.
                let mut k = 0;
                let mut got: Vec<(Opcode, Vec<Ex>)> = Vec::new();
                let mut good = true;
                while k < ins.len() && ins[k].opcode != Opcode::CaseListEnd {
                    let Some((st, used)) = replay(&ins[k..]) else {
                        good = false;
                        break;
                    };
                    let Some(op) = ins.get(k + used) else {
                        good = false;
                        break;
                    };
                    got.push((op.opcode, st));
                    k += used + 1;
                }
                good &= k + 1 == ins.len() && got == want;
                if good {
                    e.1 += 1;
                    for x in &ins {
                        *ops_ok.entry(x.opcode).or_default() += 1;
                    }
                } else if fails.len() < 20 {
                    fails.push(format!(
                        "case: {} got {:?}",
                        String::from_utf8_lossy(&sl.raw).trim(),
                        got.iter()
                            .map(|(o, s)| (o.label(), show(s)))
                            .collect::<Vec<_>>()
                    ));
                }
                continue;
            }
            // ---- Debug.Print items ----
            if t.len() > 3
                && lc(&t[0]) == "debug"
                && t[1].kind == TokenKind::Dot
                && lc(&t[2]) == "print"
            {
                let body = &t[3..];
                let items = split_top(body, &|x| {
                    matches!(x.kind, TokenKind::Comma | TokenKind::Semicolon)
                });
                let seps: Vec<TokenKind> = body
                    .iter()
                    .scan(0i32, |d, x| {
                        match x.kind {
                            TokenKind::LParen => *d += 1,
                            TokenKind::RParen => *d -= 1,
                            _ => {}
                        }
                        Some((*d, x.kind))
                    })
                    .filter(|(d, k)| {
                        *d == 0 && matches!(k, TokenKind::Comma | TokenKind::Semicolon)
                    })
                    .map(|(_, k)| k)
                    .collect();
                let mut want: Vec<Ex> = Vec::new();
                let mut okp = true;
                for it in &items {
                    match parse_source(it) {
                        Some(e) => want.push(e),
                        None => okp = false,
                    }
                }
                if !okp {
                    continue;
                }
                let e = tally.entry("debug.print").or_default();
                e.0 += 1;
                let mut good = ins.len() > 3
                    && ins[0].opcode == Opcode::DebugPrefix
                    && ins[1].opcode == Opcode::OutTargetObject;
                let mut k = 2;
                let mut got: Vec<Ex> = Vec::new();
                let mut got_seps = Vec::new();
                while good && k < ins.len() {
                    let Some((mut st, used)) = replay(&ins[k..]) else {
                        good = false;
                        break;
                    };
                    let Some(op) = ins.get(k + used) else {
                        good = false;
                        break;
                    };
                    if st.len() != 1 {
                        good = false;
                        break;
                    }
                    got.append(&mut st);
                    match op.opcode {
                        Opcode::OutItemThenComma => got_seps.push(TokenKind::Comma),
                        Opcode::OutItemThenSemicolon => got_seps.push(TokenKind::Semicolon),
                        Opcode::OutItemLast => {
                            good &= k + used + 1 == ins.len();
                        }
                        _ => good = false,
                    }
                    k += used + 1;
                }
                good &= got == want && got_seps == seps;
                if good {
                    e.1 += 1;
                    for x in &ins {
                        *ops_ok.entry(x.opcode).or_default() += 1;
                    }
                } else if fails.len() < 20 {
                    fails.push(format!(
                        "print: {}",
                        String::from_utf8_lossy(&sl.raw).trim()
                    ));
                }
                continue;
            }
            // ---- call statements `Name args` / `Call Name(args)` ----
            let last = ins.last();
            if let Some(l) = last
                && l.opcode == Opcode::InvokeSimple
                && ins.iter().all(|x| x.opcode != Opcode::StmtBegin)
            {
                let (call_kw, rest) = if is_kw(&t[0], Keyword::Call) {
                    (true, &t[1..])
                } else {
                    (false, &t[..])
                };
                let Some(first) = rest.first() else { continue };
                let name = lc(first)
                    .trim_end_matches(|c| "%&!#@$".contains(c))
                    .to_string();
                let arg_toks: &[common::Tok] = if call_kw {
                    if rest.len() >= 3
                        && rest[1].kind == TokenKind::LParen
                        && rest.last().is_some_and(|x| x.kind == TokenKind::RParen)
                    {
                        &rest[2..rest.len() - 1]
                    } else if rest.len() == 1 {
                        &[]
                    } else {
                        continue;
                    }
                } else {
                    &rest[1..]
                };
                let parts: Vec<&[common::Tok]> = if arg_toks.is_empty() {
                    Vec::new()
                } else {
                    split_top(arg_toks, &|x| x.kind == TokenKind::Comma)
                };
                let mut want = Vec::new();
                let mut okp = true;
                for seg in parts {
                    match parse_source(seg) {
                        Some(e) => want.push(e),
                        None => okp = false,
                    }
                }
                if !okp {
                    continue;
                }
                let e = tally.entry("call-stmt").or_default();
                e.0 += 1;
                let n = ins.len() - 1;
                let nm = common::expr::name_of(&l.operands[0]);
                let argc = match l.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => usize::MAX,
                };
                let good = replay(&ins[..n]).is_some_and(|(st, k)| k == n && st == want)
                    && nm.as_deref() == Some(name.as_str())
                    && argc == want.len()
                    && (l.modifier < 16) == call_kw;
                if good {
                    e.1 += 1;
                    for x in &ins {
                        *ops_ok.entry(x.opcode).or_default() += 1;
                    }
                } else if fails.len() < 20 {
                    fails.push(format!("call: {}", String::from_utf8_lossy(&sl.raw).trim()));
                }
            }
        }
    }
    println!("(shape: total, agree) {tally:?}");
    for f in &fails {
        println!("FAIL {f}");
    }
    for (o, n) in &ops_ok {
        println!("OK2 {:3} {:28} {n}", o.canonical(), o.label());
    }
}

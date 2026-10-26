//! Differential check of statement markers against the keyword that starts
//! the source line (ignored; needs the local material).
#![allow(missing_docs)]
mod common;

use vba_lexer::TokenKind;
use vba_pcode::*;

type Pred = Box<dyn Fn(&[common::Tok]) -> bool>;

fn lc(t: &common::Tok) -> String {
    String::from_utf8_lossy(&t.text).to_lowercase()
}

fn w(t: &[common::Tok], i: usize) -> String {
    t.get(i).map(lc).unwrap_or_default()
}

fn skip_mods(t: &[common::Tok]) -> usize {
    let mut i = 0;
    while matches!(
        t.get(i).map(lc).as_deref(),
        Some("public" | "private" | "friend" | "static" | "global")
    ) {
        i += 1;
    }
    i
}

fn two(a: &'static str, b: &'static str) -> Pred {
    Box::new(move |t| w(t, 0) == a && w(t, 1) == b)
}

fn alone(a: &'static str) -> Pred {
    Box::new(move |t| t.len() == 1 && w(t, 0) == a)
}

fn first(a: &'static str) -> Pred {
    Box::new(move |t| w(t, 0) == a)
}

fn predicates() -> Vec<(Opcode, &'static str, Pred)> {
    let mut v: Vec<(Opcode, &'static str, Pred)> = Vec::new();
    v.push((
        Opcode::ProcHeader,
        "Sub/Function/Property header",
        Box::new(|t| {
            let i = skip_mods(t);
            let i = if w(t, i) == "declare" { i + 1 } else { i };
            matches!(w(t, i).as_str(), "sub" | "function" | "property")
        }),
    ));
    v.push((Opcode::ProcEndSub, "End Sub", two("end", "sub")));
    v.push((
        Opcode::ProcEndFunction,
        "End Function",
        two("end", "function"),
    ));
    v.push((
        Opcode::ProcEndProperty,
        "End Property",
        two("end", "property"),
    ));
    v.push((
        Opcode::IfBlockOpen,
        "If..Then (block)",
        Box::new(|t| w(t, 0) == "if" && t.last().map(lc).as_deref() == Some("then")),
    ));
    v.push((
        Opcode::IfSingleLine,
        "If..Then stmt (single line)",
        Box::new(|t| w(t, 0) == "if" && t.last().map(lc).as_deref() != Some("then")),
    ));
    v.push((
        Opcode::EndifSingleLine,
        "If..Then stmt (single line)",
        Box::new(|t| w(t, 0) == "if" && t.last().map(lc).as_deref() != Some("then")),
    ));
    v.push((
        Opcode::ElseSingleLine,
        "single-line If with Else",
        Box::new(|t| {
            w(t, 0) == "if"
                && t.last().map(lc).as_deref() != Some("then")
                && t.iter().any(|x| lc(x) == "else")
        }),
    ));
    v.push((Opcode::BlockElseBranch, "Else alone", alone("else")));
    v.push((Opcode::BlockElseifBranch, "ElseIf", first("elseif")));
    v.push((Opcode::BlockIfClose, "End If", two("end", "if")));
    v.push((
        Opcode::ForCounted,
        "For (no Step, no Each)",
        Box::new(|t| w(t, 0) == "for" && w(t, 1) != "each" && !t.iter().any(|x| lc(x) == "step")),
    ));
    v.push((
        Opcode::ForCountedStep,
        "For..Step",
        Box::new(|t| w(t, 0) == "for" && w(t, 1) != "each" && t.iter().any(|x| lc(x) == "step")),
    ));
    v.push((Opcode::ForCollection, "For Each", two("for", "each")));
    v.push((Opcode::ForAdvance, "Next alone", alone("next")));
    v.push((
        Opcode::ForAdvanceNamed,
        "Next v",
        Box::new(|t| w(t, 0) == "next" && t.len() > 1),
    ));
    v.push((Opcode::DoOpen, "Do alone", alone("do")));
    v.push((Opcode::DoOpenWhile, "Do While", two("do", "while")));
    v.push((Opcode::DoOpenUntil, "Do Until", two("do", "until")));
    v.push((Opcode::DoClose, "Loop alone", alone("loop")));
    v.push((Opcode::DoCloseWhile, "Loop While", two("loop", "while")));
    v.push((Opcode::DoCloseUntil, "Loop Until", two("loop", "until")));
    v.push((Opcode::WhileOpen, "While", first("while")));
    v.push((Opcode::WhileClose, "Wend", first("wend")));
    v.push((Opcode::SwitchBegin, "Select Case", two("select", "case")));
    v.push((Opcode::SwitchEnd, "End Select", two("end", "select")));
    v.push((Opcode::CaseDefault, "Case Else", two("case", "else")));
    v.push((Opcode::CaseListEnd, "Case ...", first("case")));
    v.push((
        Opcode::LeaveFunction,
        "Exit Function",
        two("exit", "function"),
    ));
    v.push((Opcode::LeaveSub, "Exit Sub", two("exit", "sub")));
    v.push((Opcode::LeaveFor, "Exit For", two("exit", "for")));
    v.push((Opcode::LeaveDo, "Exit Do", two("exit", "do")));
    v.push((
        Opcode::LeaveProperty,
        "Exit Property",
        two("exit", "property"),
    ));
    v.push((Opcode::WithOpen, "With", first("with")));
    v.push((Opcode::WithClose, "End With", two("end", "with")));
    v.push((Opcode::DebugPrefix, "Debug.", first("debug")));
    v.push((Opcode::HaltEnd, "End alone", alone("end")));
    v.push((Opcode::HaltStop, "Stop", alone("stop")));
    v.push((Opcode::OptionStmt, "Option", first("option")));
    v.push((Opcode::DimMarker, "Dim", first("dim")));
    v.push((Opcode::BranchGoto, "GoTo", first("goto")));
    v
}

#[test]
#[ignore = "needs the local reference project"]
fn statement_markers_agree_with_source() {
    let Some(p) = common::load() else { return };
    let v = version_word(&p.project_stream).unwrap();
    let layout = Layout::select(v, p.sys_kind).unwrap();
    let idents = parse_identifiers(&p.project_stream, &layout).unwrap();
    let preds = predicates();
    let mut tab = vec![(0usize, 0usize, 0usize); preds.len()];
    for m in &p.modules {
        let cache = ModuleCache::parse(&m.cache, &layout).unwrap();
        let dis = Disassembler::new(&cache, &idents);
        let src = common::logical_lines(&m.source);
        for (i, sl) in src.iter().enumerate() {
            let ins = dis.line(i).instructions;
            if sl.toks.iter().any(|t| t.kind == TokenKind::Colon) {
                continue;
            }
            for (k, (op, _, pred)) in preds.iter().enumerate() {
                let has = ins.iter().any(|x| x.opcode == *op);
                let want = pred(&sl.toks);
                match (has, want) {
                    (true, true) => tab[k].0 += 1,
                    (true, false) => tab[k].1 += 1,
                    (false, true) => tab[k].2 += 1,
                    _ => {}
                }
            }
        }
    }
    for ((op, what, _), (tp, fp, fnn)) in preds.iter().zip(&tab) {
        println!(
            "MARK {:3} {:26} {:22} agree {tp:5} cache-only {fp:4} source-only {fnn:4}",
            op.canonical(),
            op.label(),
            what
        );
    }
}

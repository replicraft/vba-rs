//! Metrics against a local VBA 7.0, 32-bit project.
//!
//! The project is local only and never committed, so every test here is
//! `#[ignore]`d and returns early when `VBA_PCODE_PROJECT` is unset or the
//! file is absent. Run with
//! `VBA_PCODE_PROJECT=<file> cargo test -p vba-pcode --test real_project -- --ignored --nocapture`.
//! The tests print numbers only, never project content beyond short
//! failure samples.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use vba_pcode::*;

use common::{SrcLine, load};

struct Ctx {
    layout: Layout,
    idents: IdentTable,
    project: common::Project,
}

fn ctx() -> Option<Ctx> {
    let project = load()?;
    let v = version_word(&project.project_stream).expect("version word");
    let layout = Layout::select(v, project.sys_kind).expect("layout");
    let idents = parse_identifiers(&project.project_stream, &layout).expect("identifiers");
    Some(Ctx {
        layout,
        idents,
        project,
    })
}

/// Lower-cased token text without type suffix and brackets.
fn norm(t: &[u8]) -> String {
    let s: String = t
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .to_lowercase();
    s.trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches(|c| "%&!#@$".contains(c))
        .to_string()
}

/// Whether the logical line is a procedure header (including `Declare`).
fn is_proc_header(l: &SrcLine) -> bool {
    let words: Vec<String> = l
        .toks
        .iter()
        .take(6)
        .map(|t| String::from_utf8_lossy(&t.text).to_lowercase())
        .collect();
    let mut i = 0;
    while i < words.len()
        && matches!(
            words[i].as_str(),
            "public" | "private" | "friend" | "static" | "global"
        )
    {
        i += 1;
    }
    if words.get(i).map(String::as_str) == Some("declare") {
        i += 1;
    }
    matches!(
        words.get(i).map(String::as_str),
        Some("sub" | "function" | "property")
    )
}

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn phase1_structure() {
    let Some(c) = ctx() else { return };
    assert_eq!(c.layout.verification(), Verification::Verified);
    println!(
        "layout {:?} {:?} version {:#x}",
        c.layout.family, c.layout.bitness, c.layout.version
    );
    println!(
        "identifier table: announced {} read {} dummies {} walk end {} of {} (trailing {})",
        c.idents.announced,
        c.idents.names.len(),
        c.idents.dummies,
        c.idents.end,
        c.idents.stream_len,
        c.idents.trailing_bytes()
    );
    assert_eq!(c.idents.names.len(), c.idents.announced);

    let (mut total, mut interp, mut opaque, mut unattr) = (0usize, 0usize, 0usize, 0usize);
    let (mut line_ok, mut proc_ok, mut tiled) = (0, 0, 0);
    let (mut procs_cache, mut procs_src) = (0, 0);
    let mut worst = (100.0f64, String::new());
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let (mut ind_total, mut ind_covered) = (0usize, 0usize);
    let mut trailers: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut post_code: BTreeMap<usize, usize> = BTreeMap::new();
    let mut rec_counts: BTreeMap<RecordKind, usize> = BTreeMap::new();
    let (mut min_unexpl, mut max_unexpl) = (usize::MAX, 0usize);
    for m in &c.project.modules {
        let cache = ModuleCache::parse(&m.cache, &c.layout).expect("module parses");
        // Gap-free tiling.
        let mut pos = 0;
        for r in &cache.regions {
            assert_eq!(r.span.start, pos, "gap in {}", m.name);
            pos = r.span.end;
        }
        assert_eq!(pos, m.cache.len());
        tiled += 1;
        let cv = cache.coverage();
        total += cv.total;
        interp += cv.interpreted;
        opaque += cv.opaque;
        unattr += cv.unattributed;
        for r in &cache.regions {
            match r.kind {
                RegionKind::Trailer => {
                    trailers.insert(m.cache[r.span.clone()].to_vec());
                }
                RegionKind::Opaque(OpaqueKind::PostCode) => {
                    *post_code.entry(r.span.len()).or_default() += 1;
                }
                _ => {}
            }
            *by_kind.entry(format!("{:?}", r.kind)).or_default() += r.span.len();
        }
        min_unexpl = min_unexpl.min(cv.unexplained());
        max_unexpl = max_unexpl.max(cv.unexplained());
        if cv.interpreted_percent() < worst.0 {
            worst = (cv.interpreted_percent(), m.name.clone());
        }
        let rm = RecordMap::collect(&cache);
        assert!(rm.conflicts.is_empty(), "{}", m.name);
        let rc = rm.coverage(&cache).unwrap();
        assert_eq!(rc.irregular, 0, "{}", m.name);
        ind_total += rc.table;
        ind_covered += rc.covered;
        for (k, n) in rm.counts() {
            *rec_counts.entry(k).or_default() += n;
        }
        let chk = cache.code_area_check();
        assert_eq!((chk.holes, chk.overlaps), (0, 0), "{}", m.name);
        assert!(cache.references.is_some(), "{}", m.name);
        let src = common::logical_lines(&m.source);
        if src.len() == cache.lines.lines.len() {
            line_ok += 1;
        }
        // Procedure lines: cache lines starting a PROC_HEADER vs source.
        let dis = Disassembler::new(&cache, &c.idents);
        let cache_procs: BTreeSet<usize> = (0..cache.lines.lines.len())
            .filter(|&i| {
                dis.line(i)
                    .instructions
                    .iter()
                    .any(|x| matches!(x.opcode, Opcode::ProcHeader | Opcode::ProcHeaderAlt))
            })
            .collect();
        let src_procs: BTreeSet<usize> = src
            .iter()
            .enumerate()
            .filter(|(_, l)| is_proc_header(l))
            .map(|(i, _)| i)
            .collect();
        procs_cache += cache_procs.len();
        procs_src += src_procs.len();
        if cache_procs == src_procs {
            proc_ok += 1;
        } else {
            println!("{}: procedure lines differ", m.name);
        }
    }
    let n = c.project.modules.len();
    println!("modules {n}, gap-free tilings {tiled}/{n}");
    println!(
        "bytes {total}: interpreted {interp} ({:.2}%), delimited-but-opaque {opaque}, unattributed {unattr}; unexplained {} ({:.2}%)",
        interp as f64 * 100.0 / total as f64,
        opaque + unattr,
        (opaque + unattr) as f64 * 100.0 / total as f64
    );
    println!(
        "indirect table: {ind_total} bytes, {ind_covered} ({:.1}%) covered by records reachable from code; records {rec_counts:?}",
        ind_covered as f64 * 100.0 / ind_total as f64
    );
    println!("unexplained bytes per module: min {min_unexpl}, max {max_unexpl}");
    println!(
        "distinct final trailers: {trailers:?}; post-code block sizes (size: modules): {post_code:?}"
    );
    println!("bytes by region kind: {by_kind:?}");
    println!("lowest interpreted share: {:.1}% ({})", worst.0, worst.1);
    println!("line count agreement: {line_ok}/{n}");
    println!("procedure line agreement: {proc_ok}/{n} (cache {procs_cache}, source {procs_src})");
    assert_eq!(tiled, n);
    assert_eq!(line_ok, n);
    assert_eq!(proc_ok, n);
    assert_eq!(procs_cache, procs_src);
}

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn phase1_records_agree_with_source() {
    let Some(c) = ctx() else { return };
    let res = Resolver::new(&c.idents, c.layout);
    let (mut procs, mut name_ok, mut kind_ok) = (0, 0, 0);
    let (mut declares_src, mut declares_rec, mut declare_agree) = (0, 0, 0);
    let (mut vars, mut var_name_ok) = (0, 0);
    let (mut args, mut arg_name_ok) = (0, 0);
    let (mut intrinsic_vars, mut intrinsic_ok) = (0, 0);
    for m in &c.project.modules {
        let cache = ModuleCache::parse(&m.cache, &c.layout).unwrap();
        let ind = &m.cache[cache.tables.indirect.clone()];
        let obj = &m.cache[cache.tables.objects.clone()];
        let src = common::logical_lines(&m.source);
        let dis = Disassembler::new(&cache, &c.idents);
        for (i, sl) in src.iter().enumerate() {
            let toks: Vec<String> = sl.toks.iter().map(|t| norm(&t.text)).collect();
            let Some(d) = dis.decode(i) else { continue };
            for ins in &d.instructions {
                match (&ins.operands, ins.opcode) {
                    (Operands::Rec(r), Opcode::ProcHeader) => {
                        procs += 1;
                        let f = FunctionRecord::parse(ind, r.0, &c.layout).unwrap();
                        let nm = res.resolve(f.name);
                        if nm
                            .spelling()
                            .is_some_and(|s| toks.contains(&s.to_lowercase()))
                        {
                            name_ok += 1;
                        }
                        let src_declare = toks.iter().any(|t| t == "declare");
                        declares_src += src_declare as usize;
                        declares_rec += f.is_declare() as usize;
                        declare_agree += (f.is_declare() == src_declare) as usize;
                        // Kind: the keywords of the header line.
                        let prop =
                            |w: &str| toks.windows(2).any(|p| p[0] == "property" && p[1] == w);
                        let want = if prop("get") {
                            ProcKind::PropertyGet
                        } else if prop("let") {
                            ProcKind::PropertyLet
                        } else if prop("set") {
                            ProcKind::PropertySet
                        } else {
                            ProcKind::SubOrFunction
                        };
                        let is_fn = toks.iter().any(|t| t == "function");
                        let header_fn = ins.modifier & 7 == 2 || ins.modifier & 7 == 6;
                        if f.kind() == want
                            && (is_fn == header_fn || want != ProcKind::SubOrFunction)
                        {
                            kind_ok += 1;
                        }
                        for a in argument_chain(ind, f.first_arg, &c.layout, 4096).unwrap() {
                            if let Some(s) = res.resolve(a.name).spelling() {
                                args += 1;
                                if toks.contains(&s.to_lowercase()) {
                                    arg_name_ok += 1;
                                }
                            }
                        }
                    }
                    (Operands::VarDecl(r, _), Opcode::VarDecl) => {
                        vars += 1;
                        let v = VariableRecord::parse(ind, r.0, &c.layout).unwrap();
                        if res
                            .resolve(v.name)
                            .spelling()
                            .is_some_and(|s| toks.contains(&s.to_lowercase()))
                        {
                            var_name_ok += 1;
                        }
                        if let Ok(TypeRef::Intrinsic(t)) = variable_type(&v, ind, obj, &c.layout)
                            && let Some(w) = t.intrinsic.name().map(str::to_lowercase)
                        {
                            intrinsic_vars += 1;
                            if toks.contains(&w) {
                                intrinsic_ok += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    println!("procedure records {procs}: name agrees {name_ok}, kind agrees {kind_ok}");
    println!(
        "Declare detection: source {declares_src}, records {declares_rec}, agree on {declare_agree} of {procs}"
    );
    assert_eq!(declare_agree, procs);
    println!("argument records {args}: name agrees {arg_name_ok}");
    println!(
        "variable records {vars}: name agrees {var_name_ok}; intrinsic typed {intrinsic_vars}: type token agrees {intrinsic_ok}"
    );
    assert_eq!(name_ok, procs);
    assert_eq!(kind_ok, procs);
    assert_eq!(arg_name_ok, args);
    assert_eq!(var_name_ok, vars);
}

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn phase2_decode_and_consistency() {
    let Some(c) = ctx() else { return };
    let user: BTreeSet<String> = c
        .idents
        .names
        .iter()
        .map(|n| {
            n.iter()
                .map(|&b| char::from(b))
                .collect::<String>()
                .to_lowercase()
        })
        .collect();
    let mut total = common::consistency::Score::default();
    let mut hist: BTreeMap<Opcode, usize> = BTreeMap::new();
    let (mut lines, mut bad_lines, mut instrs) = (0, 0, 0);
    let mut builtin_evidence: BTreeMap<u16, (usize, usize)> = BTreeMap::new();
    let mut unknown_builtin_ops = 0;
    let mut out_of_range_ops = 0;
    for m in &c.project.modules {
        let cache = ModuleCache::parse(&m.cache, &c.layout).unwrap();
        let dis = Disassembler::new(&cache, &c.idents);
        let src = common::logical_lines(&m.source);
        #[allow(clippy::needless_range_loop)]
        for i in 0..cache.lines.lines.len() {
            let l = dis.line(i);
            if !l.instructions.is_empty() || l.error.is_some() {
                lines += 1;
            }
            if l.error.is_some() {
                bad_lines += 1;
            }
            for ins in &l.instructions {
                instrs += 1;
                *hist.entry(ins.opcode).or_default() += 1;
                for o in &ins.operands {
                    if let Operand::Name { name, .. } = o {
                        match name {
                            Name::Builtin(s) => {
                                // Every use of a confirmed built-in must sit on
                                // a line that has the spelling as a token.
                                let key =
                                    CONFIRMED_BUILTINS.iter().find(|(_, n)| n == s).unwrap().0;
                                let e = builtin_evidence.entry(key).or_default();
                                e.0 += 1;
                                let want = s.to_lowercase();
                                if src[i].toks.iter().any(|t| norm(&t.text) == want) {
                                    e.1 += 1;
                                }
                            }
                            Name::UnknownBuiltin(_) => unknown_builtin_ops += 1,
                            Name::OutOfRange(_) => out_of_range_ops += 1,
                            _ => {}
                        }
                    }
                }
            }
        }
        let s = common::consistency::score_module(&m.name, &cache, &dis, &src, &user);
        total.add(&s);
    }
    let t = &total;
    println!(
        "lines with code {lines}, lines that stop decoding {bad_lines}, instructions {instrs}"
    );
    println!(
        "statements {}: consistent {} ({:.2}%), strict {} ({:.2}%), forward failures {}, reverse misses {}, without code {}",
        t.statements,
        t.consistent,
        t.consistent as f64 * 100.0 / t.statements as f64,
        t.strict,
        t.strict as f64 * 100.0 / t.statements as f64,
        t.forward_fail,
        t.reverse_miss,
        t.no_code
    );
    println!(
        "operand checks: names {}/{}, strings {}/{}, comments {}/{}, integers {}/{}; unresolved name operands {}",
        t.name_ops_ok,
        t.name_ops,
        t.text_ops_ok,
        t.text_ops,
        t.comment_ops_ok,
        t.comment_ops,
        t.int_ops_ok,
        t.int_ops,
        t.unresolved_names
    );
    println!(
        "confirmed built-in uses (index: uses/agreeing): {builtin_evidence:?}; unknown built-in operands {unknown_builtin_ops}, out-of-range identifier operands {out_of_range_ops}"
    );
    println!("opcodes seen: {} of {}", hist.len(), Opcode::ALL.len());
    for (op, n) in &hist {
        println!("  {:3} {:32} {}", op.canonical(), op.label(), n);
    }
    let never: Vec<&str> = Opcode::ALL
        .iter()
        .filter(|o| !hist.contains_key(o))
        .map(|o| o.label())
        .collect();
    println!("opcodes never seen ({}): {}", never.len(), never.join(" "));
    assert_eq!(bad_lines, 0);
    for (k, (uses, ok)) in &builtin_evidence {
        assert_eq!(uses, ok, "built-in {k} contradicted");
    }
    assert!(t.consistent as f64 / t.statements as f64 >= 0.95);
}

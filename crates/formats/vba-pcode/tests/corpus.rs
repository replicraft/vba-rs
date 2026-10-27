//! Structure and decoding checks over the project files below a local
//! directory (never committed): documents of any supported container or
//! standalone project storages. Set `VBA_PCODE_CORPUS` to the directory; the
//! test returns early when it is unset. One line per file is printed, then
//! totals per layout; for layouts listed in `VERIFIED_LAYOUTS` the invariants
//! that held on the verification material are asserted.
//!
//! `cargo test -p vba-pcode --test corpus -- --ignored --nocapture`
//!
//! Debugging aids (environment): `SHOW_FAIL=<file number>` lists the
//! statements that disagree with their source, `SHOW_REC=1` the record
//! disagreements, `SHOW_LINES=1` modules whose line count differs.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use vba_pcode::*;

use common::{corpus_files, load_path};

fn is_proc_header(words: &[String]) -> bool {
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

fn records(
    cache: &ModuleCache<'_>,
    idents: &IdentTable,
    layout: &Layout,
    src: &[common::SrcLine],
    dis: &Disassembler<'_>,
    t: &mut Tally,
) {
    let res = Resolver::new(idents, *layout);
    let ind = &cache.bytes[cache.tables.indirect.clone()];
    let obj = &cache.bytes[cache.tables.objects.clone()];
    for (i, sl) in src.iter().enumerate() {
        let toks: Vec<String> = sl.toks.iter().map(|t| norm(&t.text)).collect();
        let Some(d) = dis.decode(i) else { continue };
        for ins in &d.instructions {
            match (&ins.operands, ins.opcode) {
                (Operands::Rec(r), Opcode::ProcHeader) => {
                    let Ok(f) = FunctionRecord::parse(ind, r.0, layout) else {
                        t.rec_procs += 1;
                        continue;
                    };
                    t.rec_procs += 1;
                    if res
                        .resolve_symbol(f.name)
                        .spelling()
                        .is_some_and(|s| toks.contains(&s.to_lowercase()))
                    {
                        t.rec_name_ok += 1;
                    } else if std::env::var_os("SHOW_REC").is_some() {
                        println!(
                            "    RECFAIL proc {:?} | {}",
                            res.resolve_symbol(f.name),
                            String::from_utf8_lossy(&sl.raw)
                        );
                    }
                    // Attributes: the keywords before `Sub`/`Function`/`Property`.
                    let head: Vec<&String> = toks
                        .iter()
                        .take_while(|t| !matches!(t.as_str(), "sub" | "function" | "property"))
                        .collect();
                    let has = |w: &str| head.iter().any(|t| t.as_str() == w);
                    // `Friend` procedures have the public bit as well.
                    let src_public = !has("private");
                    t.attr_procs += 1;
                    t.attr_public_rec += usize::from(f.is_public() == src_public);
                    if f.is_public() != src_public && std::env::var_os("SHOW_REC").is_some() {
                        println!(
                            "    RECFAIL public {} flags6={:#x} mod={:#x} | {}",
                            f.is_public(),
                            f.flags6,
                            ins.modifier,
                            String::from_utf8_lossy(&sl.raw)
                        );
                    }
                    t.attr_public_mod += usize::from((ins.modifier & 4 != 0) == src_public);
                    t.attr_friend += usize::from(f.is_friend() == has("friend"));
                    t.attr_static += usize::from(f.is_static() == has("static"));
                    if toks.iter().any(|t| t == "declare") {
                        t.attr_declares += 1;
                        t.attr_ptrsafe += usize::from(
                            f.is_ptr_safe() == has("ptrsafe")
                                || toks.iter().any(|t| t == "ptrsafe") == f.is_ptr_safe(),
                        );
                    }
                    let src_declare = toks.iter().any(|t| t == "declare");
                    t.rec_declare_ok += usize::from(f.is_declare() == src_declare);
                    let prop = |w: &str| toks.windows(2).any(|p| p[0] == "property" && p[1] == w);
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
                    if f.kind() == want && (is_fn == header_fn || want != ProcKind::SubOrFunction) {
                        t.rec_kind_ok += 1;
                    }
                    if let Ok(chain) = argument_chain(ind, f.first_arg, layout, 4096) {
                        for a in chain {
                            if let Some(s) = res.resolve_symbol(a.name).spelling() {
                                t.rec_args += 1;
                                if toks.contains(&s.to_lowercase()) {
                                    t.rec_args_ok += 1;
                                } else if std::env::var_os("SHOW_REC").is_some() {
                                    println!(
                                        "    RECFAIL arg {s:?} | {}",
                                        String::from_utf8_lossy(&sl.raw)
                                    );
                                }
                            }
                        }
                    }
                }
                (Operands::VarDecl(r, _), Opcode::VarDecl) => {
                    let Ok(v) = VariableRecord::parse(ind, r.0, layout) else {
                        t.rec_vars += 1;
                        continue;
                    };
                    t.rec_vars += 1;
                    if res
                        .resolve_symbol(v.name)
                        .spelling()
                        .is_some_and(|s| toks.contains(&s.to_lowercase()))
                    {
                        t.rec_vars_ok += 1;
                    } else if std::env::var_os("SHOW_REC").is_some() {
                        println!(
                            "    RECFAIL var {:?} | {}",
                            res.resolve_symbol(v.name),
                            String::from_utf8_lossy(&sl.raw)
                        );
                    }
                    if let Ok(TypeRef::Intrinsic(ty)) = variable_type(&v, ind, obj, layout)
                        && let Some(w) = ty.intrinsic.name().map(str::to_lowercase)
                    {
                        // `LongLong` with the pointer flag is `LongPtr`.
                        let w = if matches!(ty.intrinsic, IntrinsicType::LongLong | IntrinsicType::Long)
                            && ty.flags & 0x80 != 0
                        {
                            "longptr".to_string()
                        } else {
                            w
                        };
                        t.rec_types += 1;
                        if toks.contains(&w) {
                            t.rec_types_ok += 1;
                        } else if std::env::var_os("SHOW_REC").is_some() {
                            println!(
                                "    RECFAIL type {w} flags {:#x} | {}",
                                ty.flags,
                                String::from_utf8_lossy(&sl.raw)
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Default, Debug)]
struct Tally {
    modules: usize,
    parsed: usize,
    tiled: usize,
    holes: usize,
    line_count_ok: usize,
    proc_lines_ok: usize,
    lines_with_code: usize,
    lines_decoded: usize,
    procs_cache: usize,
    procs_src: usize,
    score: common::consistency::Score,
    rec_procs: usize,
    rec_name_ok: usize,
    rec_kind_ok: usize,
    rec_declare_ok: usize,
    rec_args: usize,
    rec_args_ok: usize,
    rec_vars: usize,
    rec_vars_ok: usize,
    rec_types: usize,
    rec_types_ok: usize,
    attr_procs: usize,
    attr_public_rec: usize,
    attr_public_mod: usize,
    attr_friend: usize,
    attr_static: usize,
    attr_ptrsafe: usize,
    attr_declares: usize,
    tables_only: usize,
    named: usize,
    named_ok: usize,
    named_unres: usize,
    errors: Vec<String>,
}

fn scan(p: &common::Project, t: &mut Tally) -> Option<Layout> {
    let v = version_word(&p.project_stream).ok()?;
    let layout = match Layout::select(v, p.sys_kind) {
        Ok(l) => l,
        Err(e) => {
            t.errors.push(format!("layout: {e}"));
            return None;
        }
    };
    let idents = match parse_identifiers(&p.project_stream, &layout) {
        Ok(i) => i,
        Err(e) => {
            t.errors.push(format!("idents: {e}"));
            return Some(layout);
        }
    };
    if idents.names.len() != idents.announced {
        t.errors.push(format!(
            "idents announced {} read {}",
            idents.announced,
            idents.names.len()
        ));
    }
    for m in &p.modules {
        t.modules += 1;
        let cache = match ModuleCache::parse(&m.cache, &layout) {
            Ok(c) => c,
            Err(e) => {
                // The module streams of a compiled-only project carry the
                // header tables but no usable line table (the stream ends
                // 6 bytes after the line-table pointer, or the pointer leads
                // to no marker): only the location of the tables is checked.
                if p.compiled_only && TableLocations::locate(&m.cache, &layout).is_ok() {
                    t.tables_only += 1;
                    continue;
                }
                t.errors.push(format!("module {}: {e}", m.name));
                continue;
            }
        };
        t.parsed += 1;
        let mut pos = 0;
        let mut tiled = true;
        for r in &cache.regions {
            tiled &= r.span.start == pos;
            pos = r.span.end;
        }
        tiled &= pos == m.cache.len();
        t.tiled += usize::from(tiled);
        let chk = cache.code_area_check();
        t.holes += chk.holes + chk.overlaps;
        if p.compiled_only {
            // No source: structure and decoding only.
            let dis = Disassembler::new(&cache, &idents);
            t.line_count_ok += 1;
            for i in 0..cache.lines.lines.len() {
                let l = dis.line(i);
                if cache.lines.lines[i].length > 0 {
                    t.lines_with_code += 1;
                    t.lines_decoded += usize::from(l.error.is_none());
                }
                if l.instructions
                    .iter()
                    .any(|x| matches!(x.opcode, Opcode::ProcHeader | Opcode::ProcHeaderAlt))
                {
                    t.procs_cache += 1;
                }
            }
            continue;
        }
        let src = common::logical_lines(&m.source);
        t.line_count_ok += usize::from(src.len() == cache.lines.lines.len());
        if src.len() != cache.lines.lines.len() && std::env::var_os("SHOW_LINES").is_some() {
            println!(
                "    linecount {}: source {} cache {}",
                m.name,
                src.len(),
                cache.lines.lines.len()
            );
        }
        let dis = Disassembler::new(&cache, &idents);
        let mut cache_procs = BTreeSet::new();
        for i in 0..cache.lines.lines.len() {
            let l = dis.line(i);
            if cache.lines.lines[i].length > 0 {
                t.lines_with_code += 1;
                t.lines_decoded += usize::from(l.error.is_none());
            }
            if l.instructions
                .iter()
                .any(|x| matches!(x.opcode, Opcode::ProcHeader | Opcode::ProcHeaderAlt))
            {
                cache_procs.insert(i);
            }
        }
        let src_procs: BTreeSet<usize> = src
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                let w: Vec<String> = l
                    .toks
                    .iter()
                    .take(6)
                    .map(|t| String::from_utf8_lossy(&t.text).to_lowercase())
                    .collect();
                is_proc_header(&w)
            })
            .map(|(i, _)| i)
            .collect();
        let user: BTreeSet<String> = idents
            .names
            .iter()
            .map(|n| {
                n.iter()
                    .map(|&b| char::from(b))
                    .collect::<String>()
                    .to_lowercase()
            })
            .collect();
        let sc = common::consistency::score_module(&m.name, &cache, &dis, &src, &user);
        t.score.add(&sc);
        records(&cache, &idents, &layout, &src, &dis, t);
        for (i, sl) in src.iter().enumerate() {
            let toks: Vec<String> = sl.toks.iter().map(|t| norm(&t.text)).collect();
            for ins in &dis.line(i).instructions {
                for o in &ins.operands {
                    if let Operand::Variable(v) = o
                        && let ResolvedType::Named(n) = &v.ty
                    {
                        t.named += 1;
                        match n.spelling() {
                            Some(sp) if toks.contains(&sp.to_lowercase()) => t.named_ok += 1,
                            Some(sp) => {
                                if std::env::var_os("SHOW_REC").is_some() {
                                    println!(
                                        "    RECFAIL named {sp} | {}",
                                        String::from_utf8_lossy(&sl.raw)
                                    );
                                }
                            }
                            None => {
                                t.named_unres += 1;
                                if std::env::var_os("SHOW_REC").is_some() {
                                    println!(
                                        "    RECFAIL named-unresolved {n:?} | {}",
                                        String::from_utf8_lossy(&sl.raw)
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        if cache_procs != src_procs && std::env::var_os("SHOW_REC").is_some() {
            for &i in cache_procs.symmetric_difference(&src_procs) {
                println!(
                    "    RECFAIL procline {} {i} in-cache={} | {}",
                    m.name,
                    cache_procs.contains(&i),
                    src.get(i)
                        .map_or(String::new(), |l| String::from_utf8_lossy(&l.raw)
                            .into_owned())
                );
            }
        }
        t.procs_cache += cache_procs.len();
        t.procs_src += src_procs.len();
        t.proc_lines_ok += usize::from(cache_procs == src_procs);
    }
    Some(layout)
}

/// Per layout totals over the files of the corpus.
#[derive(Default, Debug)]
struct Totals {
    files: usize,
    modules: usize,
    lines_with_code: usize,
    statements: usize,
    consistent: usize,
    procedures: usize,
    named_types: usize,
}

#[test]
#[ignore = "needs a local corpus (VBA_PCODE_CORPUS)"]
fn corpus_structure() {
    let mut totals: BTreeMap<String, Totals> = BTreeMap::new();
    let mut fails: Vec<String> = Vec::new();
    macro_rules! chk_eq {
        ($f:expr, $a:expr, $b:expr, $($m:tt)+) => {
            if $a != $b {
                $f.push(format!("{}: {} vs {}", format!($($m)+), $a, $b));
            }
        };
    }
    for (n, path) in corpus_files().iter().enumerate() {
        if std::env::var_os("SHOW_PATHS").is_some() {
            println!("#{n} = {}", path.display());
        }
        let Some(p) = load_path(path) else {
            println!("#{n} unreadable");
            continue;
        };
        let mut t = Tally::default();
        let layout = scan(&p, &mut t);
        let desc = layout.map_or("-".to_string(), |l| {
            format!("{:?}/{:?}/{:#x}", l.family, l.bitness, l.version)
        });
        let sc = &t.score;
        if std::env::var_os("SHOW_REC").is_some() {
            for e in t.errors.iter().take(4) {
                println!("    ERROR {e}");
            }
        }
        println!(
            "#{n}{} {desc} modules {} parsed {} tiled {} unclaimed-code-bytes {} line-count {} \
             decoded {}/{} procedures {} tables-only {}",
            if p.compiled_only { " (compiled-only)" } else { "" },
            t.modules,
            t.parsed,
            t.tiled,
            t.holes,
            t.line_count_ok,
            t.lines_decoded,
            t.lines_with_code,
            t.rec_procs,
            t.tables_only,
        );
        if let Some(only) = std::env::var("SHOW_FAIL")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            && only == n
        {
            for f in sc.failures.iter().take(60) {
                println!("    FAIL {f}");
            }
        }
        println!(
            "    statements {} consistent {} strict {} forward-fail {} reverse-miss {} | operands: \
             names {}/{} text {}/{} int {}/{} | records: procedure names {}/{} kinds {} declares {} \
             arguments {}/{} variables {}/{} intrinsic types {}/{} named types {}/{} | attributes of \
             {} procedures: public {} friend {} static {} ptrsafe {}/{}",
            sc.statements,
            sc.consistent,
            sc.strict,
            sc.forward_fail,
            sc.reverse_miss,
            sc.name_ops_ok,
            sc.name_ops,
            sc.text_ops_ok,
            sc.text_ops,
            sc.int_ops_ok,
            sc.int_ops,
            t.rec_name_ok,
            t.rec_procs,
            t.rec_kind_ok,
            t.rec_declare_ok,
            t.rec_args_ok,
            t.rec_args,
            t.rec_vars_ok,
            t.rec_vars,
            t.rec_types_ok,
            t.rec_types,
            t.named_ok,
            t.named,
            t.attr_procs,
            t.attr_public_rec,
            t.attr_friend,
            t.attr_static,
            t.attr_ptrsafe,
            t.attr_declares
        );
        let Some(l) = layout else { continue };
        let e = totals.entry(desc.clone()).or_default();
        e.files += 1;
        e.modules += t.modules;
        e.lines_with_code += t.lines_with_code;
        e.statements += sc.statements;
        e.consistent += sc.consistent;
        e.procedures += t.rec_procs;
        e.named_types += t.named;
        if l.verification() != Verification::Verified {
            continue;
        }
        // The invariants of a verified layout.
        if !t.errors.is_empty() {
            fails.push(format!("#{n} {desc}: {:?}", t.errors));
        }
        chk_eq!(
            fails,
            t.parsed + t.tables_only,
            t.modules,
            "#{n} {desc}: modules that parse"
        );
        chk_eq!(
            fails,
            t.tiled + t.tables_only,
            t.modules,
            "#{n} {desc}: gap-free tiling"
        );
        chk_eq!(
            fails,
            t.line_count_ok + t.tables_only,
            t.modules,
            "#{n} {desc}: line counts"
        );
        chk_eq!(fails, t.lines_decoded, t.lines_with_code, "#{n} {desc}: decoding");
        if p.compiled_only {
            continue;
        }
        chk_eq!(fails, t.rec_name_ok, t.rec_procs, "#{n} {desc}: procedure names");
        chk_eq!(fails, t.rec_kind_ok, t.rec_procs, "#{n} {desc}: procedure kinds");
        chk_eq!(fails, t.rec_declare_ok, t.rec_procs, "#{n} {desc}: declares");
        chk_eq!(fails, t.rec_args_ok, t.rec_args, "#{n} {desc}: argument names");
        chk_eq!(fails, t.rec_vars_ok, t.rec_vars, "#{n} {desc}: variable names");
        chk_eq!(fails, t.rec_types_ok, t.rec_types, "#{n} {desc}: intrinsic types");
        // Type names that are built-in keywords (`Module`, `Property`) have no
        // confirmed spelling; they must not resolve to a wrong name.
        chk_eq!(
            fails,
            t.named_ok + t.named_unres,
            t.named,
            "#{n} {desc}: named types"
        );
        chk_eq!(fails, t.attr_public_rec, t.attr_procs, "#{n} {desc}: visibility");
        chk_eq!(fails, t.attr_friend, t.attr_procs, "#{n} {desc}: Friend");
        chk_eq!(fails, t.attr_static, t.attr_procs, "#{n} {desc}: Static");
        chk_eq!(fails, t.attr_ptrsafe, t.attr_declares, "#{n} {desc}: PtrSafe");
        chk_eq!(fails, sc.name_ops_ok, sc.name_ops, "#{n} {desc}: name operands");
        chk_eq!(fails, sc.text_ops_ok, sc.text_ops, "#{n} {desc}: string operands");
        // The graphics `Line` statement carries a synthetic zero argument.
        if sc.int_ops_ok * 1000 < sc.int_ops * 999 {
            fails.push(format!(
                "#{n} {desc}: integer operands {} vs {}",
                sc.int_ops_ok, sc.int_ops
            ));
        }
        if sc.statements >= 20 {
            if sc.consistent * 10 < sc.statements * 9 {
                fails.push(format!(
                    "#{n} {desc}: {} of {} statements consistent",
                    sc.consistent, sc.statements
                ));
            }
        }
    }
    for (k, v) in &totals {
        println!("{k}: {v:?}");
    }
    for f in &fails {
        println!("INVARIANT FAILED {f}");
    }
    assert!(fails.is_empty(), "{} invariant failures", fails.len());
}

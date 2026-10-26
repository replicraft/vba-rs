//! Support code for the differential-testing (oracle) tooling.
//!
//! Shared by the `oracle` example (the command line) and the `oracle_tooling`
//! test (synthetic-data tests). See `docs/tooling/differential-testing.md`.
//!
//! Three jobs:
//!
//! 1. `prepare`: classify every conformance program as capturable or not, and
//!    for the capturable ones emit module files in which every `Debug.Print`
//!    statement is rewritten into `Print #255, ...` (a real VBA file write, so
//!    the host's own formatting rules apply), plus a generated entry module.
//!    Anything the rewriter cannot translate faithfully is refused.
//! 2. `report`: compare oracle results (JSON lines) with our engine.
//! 3. `apply`: upgrade `' SOURCE:` provenance of matching programs to
//!    `oracle`, append the environment to `ENVIRONMENTS.md` and write the
//!    mismatch report. Mismatches are never auto-fixed.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vba::runtime::Error as RunError;
use vba::{Capability, Engine, Error, ModuleKind as VbaModuleKind, Project};

/// The file number the harness reserves for the captured log.
pub const LOG_FILE_NUMBER: u32 = 255;

// ------------------------------------------------------------------ corpus

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Standard,
    Class,
    Document,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub id: String,
    pub path: PathBuf,
    pub text: String,
    pub source_kind: String,
    pub source_ref: String,
    pub expect: Vec<String>,
    pub entry: String,
    pub error: Option<String>,
    pub grants: Vec<String>,
    pub max_steps: Option<u64>,
    pub xfail: bool,
    pub has_raise: bool,
    pub modules: Vec<(String, Kind, String)>,
}

impl Program {
    pub fn area(&self) -> &str {
        self.id.split('/').next().unwrap_or("")
    }
    pub fn digest(&self) -> String {
        fnv1a(self.text.as_bytes())
    }
}

/// FNV-1a 64 as 16 hex digits: a cheap change detector, not a security hash.
pub fn fnv1a(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "bas") {
            out.push(p);
        }
    }
    Ok(())
}

/// Parses one conformance program (same header conventions as the conformance runner).
pub fn parse_program(base: &Path, path: &Path) -> Result<Program, String> {
    let id = path
        .strip_prefix(base)
        .map_err(|e| e.to_string())?
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    let text = std::fs::read_to_string(path).map_err(|e| format!("{id}: {e}"))?;
    let mut p = Program {
        id: id.clone(),
        path: path.to_path_buf(),
        text: text.clone(),
        source_kind: String::new(),
        source_ref: String::new(),
        expect: Vec::new(),
        entry: "Main".into(),
        error: None,
        grants: Vec::new(),
        max_steps: None,
        xfail: false,
        has_raise: false,
        modules: Vec::new(),
    };
    let mut current: (String, Kind, String) = ("Module1".into(), Kind::Standard, String::new());
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("'#MODULE ") {
            let mut it = rest.split_whitespace();
            let name = it.next().ok_or(format!("{id}: #MODULE without a name"))?;
            let kind = match it.next() {
                Some("standard") | None => Kind::Standard,
                Some("class") => Kind::Class,
                Some("document") => Kind::Document,
                Some(k) => return Err(format!("{id}: unknown module kind `{k}`")),
            };
            let previous = std::mem::replace(&mut current, (name.to_owned(), kind, String::new()));
            let blank = previous
                .2
                .lines()
                .all(|l| l.trim().is_empty() || l.trim_start().starts_with('\''));
            if !(blank && p.modules.is_empty() && previous.0 == "Module1") {
                p.modules.push(previous);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("' ")
            && let Some((key, value)) = rest.split_once(':')
            && !key.is_empty()
            && key.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        {
            let value = value.strip_prefix(' ').unwrap_or(value);
            let value = value.trim_end_matches('\r');
            match key {
                "SOURCE" => {
                    let (k, r) = value.split_once(' ').unwrap_or((value, ""));
                    p.source_kind = k.to_owned();
                    p.source_ref = r.trim().to_owned();
                }
                "EXPECT" => p.expect.push(value.to_owned()),
                "ENTRY" => p.entry = value.to_owned(),
                "ERROR" => p.error = Some(value.trim().to_owned()),
                "RAISE" => p.has_raise = true,
                "GRANT" => p.grants.push(value.trim().to_owned()),
                "MAX_STEPS" => p.max_steps = value.trim().parse().ok(),
                "XFAIL" => p.xfail = true,
                _ => {}
            }
        }
        current.2.push_str(line);
        current.2.push_str("\r\n");
    }
    p.modules.push(current);
    Ok(p)
}

/// Loads every program under `vba_dir` (sorted by id).
pub fn load_corpus(vba_dir: &Path) -> Result<Vec<Program>, String> {
    let mut files = Vec::new();
    collect(vba_dir, &mut files)?;
    files.iter().map(|f| parse_program(vba_dir, f)).collect()
}

// --------------------------------------------------------------- scanning

/// Blanks string-literal contents and comments (same byte length for ASCII).
pub fn mask_code(line: &str) -> String {
    let b: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == '"' {
                if i + 1 < b.len() && b[i + 1] == '"' {
                    out.push_str("  ");
                    i += 2;
                    continue;
                }
                in_str = false;
                out.push('"');
            } else {
                out.push(' ');
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_str = true;
            out.push('"');
        } else if c == '\''
            || ((c == 'R' || c == 'r')
                && i + 2 < b.len()
                && b[i + 1].eq_ignore_ascii_case(&'e')
                && b[i + 2].eq_ignore_ascii_case(&'m')
                && (i + 3 == b.len() || !(b[i + 3].is_alphanumeric() || b[i + 3] == '_'))
                && {
                    let t = out.trim_end();
                    t.is_empty() || t.ends_with(':')
                })
        {
            // A comment (`'` or `Rem` at the start of a statement) ends the code.
            out.extend(std::iter::repeat_n(' ', b.len() - i));
            break;
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// A word (identifier or number) in masked code.
#[derive(Debug, Clone)]
pub struct Word {
    pub start: usize,
    pub end: usize,
    pub lower: String,
    pub after_dot: bool,
}

pub fn words(masked: &str) -> Vec<Word> {
    let bytes = masked.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
            let s = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let after_dot = s > 0 && bytes[s - 1] == b'.';
            out.push(Word {
                start: s,
                end: i,
                lower: masked[s..i].to_ascii_lowercase(),
                after_dot,
            });
        } else {
            i += 1;
        }
    }
    out
}

/// Why a program cannot (or must not) be run in an oracle host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// `safety`, `ui`, `host`, `engine-only`, `untranslatable` or `manual`.
    pub category: &'static str,
    pub reason: String,
}

fn refuse(category: &'static str, reason: impl Into<String>) -> Refusal {
    Refusal {
        category,
        reason: reason.into(),
    }
}

/// Words that make a program unsafe or unrunnable unattended (default deny).
const DENY_WORDS: &[(&str, &str, &str)] = &[
    ("kill", "safety", "Kill deletes files"),
    ("shell", "safety", "Shell starts processes"),
    ("sendkeys", "safety", "SendKeys drives the UI"),
    ("appactivate", "safety", "AppActivate drives the UI"),
    ("savesetting", "safety", "SaveSetting writes the registry"),
    (
        "deletesetting",
        "safety",
        "DeleteSetting writes the registry",
    ),
    ("getsetting", "safety", "GetSetting reads the registry"),
    (
        "getallsettings",
        "safety",
        "GetAllSettings reads the registry",
    ),
    ("mkdir", "safety", "MkDir changes the file system"),
    ("rmdir", "safety", "RmDir changes the file system"),
    ("chdir", "safety", "ChDir changes the process state"),
    ("chdrive", "safety", "ChDrive changes the process state"),
    ("filecopy", "safety", "FileCopy writes files"),
    ("setattr", "safety", "SetAttr changes files"),
    ("getattr", "safety", "GetAttr reads the file system"),
    ("fileattr", "safety", "FileAttr uses file handles"),
    ("filelen", "safety", "FileLen reads the file system"),
    (
        "filedatetime",
        "safety",
        "FileDateTime reads the file system",
    ),
    ("dir", "safety", "Dir reads the file system"),
    ("open", "safety", "Open uses the file system"),
    (
        "close",
        "safety",
        "Close would also close the reserved log file",
    ),
    (
        "reset",
        "safety",
        "Reset would also close the reserved log file",
    ),
    ("lock", "safety", "Lock uses files"),
    ("unlock", "safety", "Unlock uses files"),
    ("environ", "safety", "Environ is machine specific"),
    ("command", "safety", "Command is machine specific"),
    ("curdir", "safety", "CurDir is machine specific"),
    (
        "createobject",
        "safety",
        "CreateObject instantiates COM classes",
    ),
    ("getobject", "safety", "GetObject binds COM objects"),
    ("declare", "safety", "Declare calls native code"),
    ("msgbox", "ui", "MsgBox opens a modal dialog"),
    ("inputbox", "ui", "InputBox opens a modal dialog"),
    ("stop", "ui", "Stop breaks into the debugger"),
    ("beep", "ui", "Beep makes a sound"),
    ("load", "ui", "Load needs a UserForm"),
    ("unload", "ui", "Unload needs a UserForm"),
    (
        "application",
        "host",
        "refers to the host Application object",
    ),
    ("thisworkbook", "host", "refers to the host workbook"),
    ("activeworkbook", "host", "refers to the host workbook"),
    ("workbooks", "host", "refers to the host workbooks"),
    ("worksheets", "host", "refers to the host worksheets"),
    ("docmd", "host", "refers to the Access DoCmd object"),
    ("currentdb", "host", "refers to the Access database"),
];

/// Reserved names a program must not use for its own modules.
const RESERVED_MODULES: &[&str] = &["oracleharness", "oracleentry"];

pub fn has_non_ascii(p: &Program) -> bool {
    !p.text.is_ascii()
}

/// Rewrites the `Debug.Print` statements of one line; errors explain why not.
pub fn rewrite_line(line: &str) -> Result<String, String> {
    if !line.is_ascii() {
        return Err("non-ASCII source text".into());
    }
    let masked = mask_code(line);
    let ws = words(&masked);
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    for (idx, w) in ws.iter().enumerate() {
        if w.lower != "debug" || w.after_dot {
            continue;
        }
        let mb = masked.as_bytes();
        let mut j = w.end;
        while j < mb.len() && mb[j] == b' ' {
            j += 1;
        }
        if j >= mb.len() || mb[j] != b'.' {
            return Err("`Debug` is used other than as `Debug.Print`".into());
        }
        j += 1;
        while j < mb.len() && mb[j] == b' ' {
            j += 1;
        }
        let Some(next) = ws.get(idx + 1).filter(|n| n.start == j) else {
            return Err("`Debug.` is not followed by a member name".into());
        };
        if next.lower != "print" {
            return Err(format!(
                "Debug.{} cannot be captured",
                &masked[next.start..next.end]
            ));
        }
        let prefix = masked[..w.start].trim_end();
        let last_word = prefix
            .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let at_start = prefix.is_empty()
            || prefix.ends_with(':')
            || prefix.trim_start().chars().all(|c| c.is_ascii_digit())
            || last_word == "then"
            || last_word == "else";
        if !at_start {
            return Err("Debug.Print is not at the start of a statement".into());
        }
        let rest = masked[next.end..].trim_start();
        let rest_word = ws.get(idx + 2).filter(|n| n.start >= next.end);
        let no_args = rest.is_empty()
            || rest.starts_with(':')
            || rest_word
                .is_some_and(|n| n.lower == "else" && masked[next.end..n.start].trim().is_empty());
        if rest.starts_with(';') || rest.starts_with(',') {
            return Err("Debug.Print starts with a separator".into());
        }
        let replacement = if no_args {
            // `Print #n, ""` ends the line exactly as an argument-less `Debug.Print`
            // does, without relying on how a bare `Print #n,` parses before `:`.
            format!("Print #{LOG_FILE_NUMBER}, \"\"")
        } else {
            format!("Print #{LOG_FILE_NUMBER},")
        };
        edits.push((w.start, next.end, replacement));
    }
    let mut out = line.to_owned();
    for (s, e, r) in edits.into_iter().rev() {
        out.replace_range(s..e, &r);
    }
    Ok(out)
}

/// Facts about a program that can be derived without running it.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub entry_module: String,
    pub entry: String,
    /// (module name, kind, rewritten code with CRLF line ends)
    pub modules: Vec<(String, Kind, String)>,
}

/// Decides whether a program can be run in an oracle host.
pub fn classify(p: &Program) -> Result<Prepared, Vec<Refusal>> {
    let mut why: Vec<Refusal> = Vec::new();
    for g in &p.grants {
        why.push(refuse("safety", format!("needs the `{g}` grant")));
    }
    match p.error.as_deref() {
        Some("load") => why.push(refuse(
            "engine-only",
            "expects a compile error (needs a modal dialog in a real host)",
        )),
        Some(e @ ("limit" | "cancelled" | "unsupported" | "bridge")) => why.push(refuse(
            "engine-only",
            format!("expects the engine-specific outcome `{e}`"),
        )),
        _ => {}
    }
    if p.max_steps.is_some() {
        why.push(refuse("engine-only", "tests the engine step budget"));
    }
    if p.has_raise {
        why.push(refuse(
            "host",
            "needs a host event delivered by the harness",
        ));
    }
    if p.modules.iter().any(|m| m.1 == Kind::Document) {
        why.push(refuse("host", "uses a document module"));
    }
    if has_non_ascii(p) {
        why.push(refuse(
            "untranslatable",
            "non-ASCII source text (the log is ANSI)",
        ));
    }
    for (name, _, _) in &p.modules {
        if RESERVED_MODULES.contains(&name.to_ascii_lowercase().as_str()) {
            why.push(refuse(
                "untranslatable",
                format!("module name `{name}` is reserved by the harness"),
            ));
        }
    }
    if !why.is_empty() && why.iter().any(|r| r.category != "untranslatable") {
        // Do not bother analysing the code of a program that is excluded anyway.
        return Err(dedup(why));
    }

    let mut denied: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let mut modules = Vec::new();
    let mut entry_hits: Vec<(String, bool)> = Vec::new();
    for (name, kind, code) in &p.modules {
        let mut rewritten = String::new();
        let mut cc_depth = 0i32;
        for line in code.lines() {
            let masked = mask_code(line);
            let lt = masked.trim_start().to_ascii_lowercase();
            if lt.starts_with("#if") {
                cc_depth += 1;
            } else if lt.starts_with("#end") {
                cc_depth -= 1;
            }
            let ws = words(&masked);
            for w in &ws {
                if w.after_dot {
                    continue;
                }
                if let Some((_, cat, reason)) = DENY_WORDS.iter().find(|(d, _, _)| *d == w.lower) {
                    denied.insert((cat, (*reason).to_owned()));
                }
            }
            // `Name a As b` renames files.
            if ws.first().is_some_and(|w| w.lower == "name")
                && ws.iter().any(|w| w.lower == "as")
                && !masked.contains('=')
            {
                denied.insert(("safety", "Name ... As renames files".into()));
            }
            if masked.replace(' ', "").contains("#255") {
                why.push(refuse(
                    "untranslatable",
                    "uses file number 255, reserved for the log",
                ));
            }
            if *kind == Kind::Standard {
                let mut k = 0;
                while ws.get(k).is_some_and(|w| {
                    ["public", "private", "friend", "static"].contains(&w.lower.as_str())
                }) {
                    k += 1;
                }
                if cc_depth > 0 {
                    // Conditional compilation depends on host constants.
                } else if ws
                    .get(k)
                    .is_some_and(|w| w.lower == "sub" || w.lower == "function")
                    && ws
                        .get(k + 1)
                        .is_some_and(|w| w.lower == p.entry.to_ascii_lowercase())
                {
                    let private = ws[..k].iter().any(|w| w.lower == "private");
                    let after = masked[ws[k + 1].end..].trim_start();
                    let no_params = after.is_empty()
                        || after.starts_with("()")
                        || (after.starts_with('(') && after[1..].trim_start().starts_with(')'))
                        || !after.starts_with('(');
                    if !no_params || masked.trim_end().ends_with('_') {
                        why.push(refuse(
                            "untranslatable",
                            "the entry procedure takes parameters",
                        ));
                    }
                    entry_hits.push((name.clone(), private));
                }
            }
            match rewrite_line(line) {
                Ok(l) => {
                    rewritten.push_str(&l);
                    rewritten.push_str("\r\n");
                }
                Err(e) => {
                    why.push(refuse("untranslatable", format!("{name}: {e}")));
                    rewritten.push_str(line);
                    rewritten.push_str("\r\n");
                }
            }
        }
        modules.push((name.clone(), *kind, rewritten));
    }
    for (cat, reason) in denied {
        why.push(refuse(cat, reason));
    }
    if why.is_empty()
        && p.modules
            .iter()
            .any(|m| m.2.to_ascii_lowercase().contains("class_terminate"))
        && run_engine(p).is_ok_and(|o| o.teardown_output)
    {
        why.push(refuse(
            "untranslatable",
            "prints from Class_Terminate after the entry procedure returned (project teardown is not logged)",
        ));
    }
    let entry_module = match entry_hits.as_slice() {
        [(m, false)] => m.clone(),
        [] => {
            why.push(refuse(
                "untranslatable",
                format!(
                    "no `{}` procedure in a standard module outside conditional compilation",
                    p.entry
                ),
            ));
            String::new()
        }
        [(_, true)] => {
            why.push(refuse("untranslatable", "the entry procedure is Private"));
            String::new()
        }
        _ => {
            why.push(refuse(
                "untranslatable",
                "the entry procedure name is ambiguous",
            ));
            String::new()
        }
    };
    if !why.is_empty() {
        return Err(dedup(why));
    }
    Ok(Prepared {
        entry_module,
        entry: p.entry.clone(),
        modules,
    })
}

fn dedup(mut v: Vec<Refusal>) -> Vec<Refusal> {
    let mut seen = BTreeSet::new();
    v.retain(|r| seen.insert((r.category, r.reason.clone())));
    v
}

/// Explicit exclusions from `oracle-skip.txt`: id, category, reason.
pub fn parse_skip_list(text: &str) -> BTreeMap<String, (String, String)> {
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.splitn(3, '\t');
        let id = it.next().unwrap_or("").trim().to_owned();
        let cat = it.next().unwrap_or("manual").trim().to_owned();
        let reason = it.next().unwrap_or("").trim().to_owned();
        if !id.is_empty() {
            m.insert(id, (cat, reason));
        }
    }
    m
}

/// The text of `oracle-skip.txt` the scanner derives (one line per refused program).
pub fn render_skip_list(progs: &[Program], manual: &BTreeMap<String, (String, String)>) -> String {
    let mut s = String::from(
        "# Programs excluded from automatic oracle runs (default deny).\n\
         # Format: <program id> TAB <category> TAB <reasons>\n\
         # Categories: safety, ui, host, engine-only, untranslatable, manual.\n\
         # Generated by `cargo run -p vba-rs --example oracle -- skiplist`; lines added by\n\
         # hand are kept by that command. The preparer and the driver both refuse every id here.\n",
    );
    let mut rows: BTreeMap<String, (String, String)> = BTreeMap::new();
    for p in progs {
        if let Err(r) = classify(p) {
            let cat = r[0].category.to_owned();
            let reasons = r
                .iter()
                .map(|x| x.reason.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            rows.insert(p.id.clone(), (cat, reasons));
        }
    }
    for (id, v) in manual.iter().filter(|(_, v)| v.0 == "manual") {
        rows.entry(id.clone()).or_insert_with(|| v.clone());
    }
    for (id, (c, r)) in rows {
        let _ = writeln!(s, "{id}\t{c}\t{r}");
    }
    s
}

// ----------------------------------------------------------------- bundle

const VBA_CLASS_HEADER: &str = "VERSION 1.0 CLASS\r\nBEGIN\r\n  MultiUse = -1  'True\r\nEND\r\n";

/// The importable file content of one module.
pub fn module_file(name: &str, kind: Kind, code: &str) -> (String, String) {
    match kind {
        Kind::Class => (
            format!("{name}.cls"),
            format!(
                "{VBA_CLASS_HEADER}Attribute VB_Name = \"{name}\"\r\nAttribute VB_GlobalNameSpace = False\r\nAttribute VB_Creatable = False\r\nAttribute VB_PredeclaredId = False\r\nAttribute VB_Exposed = False\r\n{code}"
            ),
        ),
        _ => (
            format!("{name}.bas"),
            format!("Attribute VB_Name = \"{name}\"\r\n{code}"),
        ),
    }
}

/// The generated entry module that wraps the program's entry procedure.
pub fn entry_module_source(entry_module: &str, entry: &str) -> String {
    format!(
        "Attribute VB_Name = \"OracleEntry\"\r\n\
Option Explicit\r\n\
' Generated by the oracle bundle preparer. UNTESTED against a real host.\r\n\
Public Sub OracleRun(ByVal logPath As String, ByVal statusPath As String)\r\n\
    Dim n As Long\r\n\
    Dim d As String\r\n\
    Dim s As String\r\n\
    OracleHarness.Begin logPath, statusPath\r\n\
    On Error GoTo EH\r\n\
    {entry_module}.{entry}\r\n\
    OracleHarness.Finish 0, \"\", \"\"\r\n\
    Exit Sub\r\n\
EH:\r\n\
    n = Err.Number\r\n\
    s = Err.Source\r\n\
    d = Err.Description\r\n\
    OracleHarness.Finish n, d, s\r\n\
End Sub\r\n"
    )
}

pub fn bundle_dir_name(id: &str) -> String {
    id.replace('/', "__")
}

/// Writes the bundle (manifest and module files) and returns the manifest.
pub fn write_bundle(
    progs: &[Program],
    skip: &BTreeMap<String, (String, String)>,
    harness: &Path,
    out: &Path,
) -> Result<Value, String> {
    std::fs::create_dir_all(out.join("programs")).map_err(|e| e.to_string())?;
    std::fs::copy(harness, out.join("OracleHarness.bas")).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for p in progs {
        let mut e = json!({"id": p.id, "digest": p.digest()});
        let verdict = match skip.get(&p.id) {
            Some((c, r)) => Err(vec![Refusal {
                category: match c.as_str() {
                    "safety" => "safety",
                    "ui" => "ui",
                    "host" => "host",
                    "engine-only" => "engine-only",
                    "untranslatable" => "untranslatable",
                    _ => "manual",
                },
                reason: format!("oracle-skip.txt: {r}"),
            }]),
            None => classify(p),
        };
        match verdict {
            Err(r) => {
                e["status"] = "not-capturable".into();
                e["category"] = r[0].category.into();
                e["reasons"] = r
                    .iter()
                    .map(|x| x.reason.clone())
                    .collect::<Vec<_>>()
                    .into();
            }
            Ok(prep) => {
                let dir = out.join("programs").join(bundle_dir_name(&p.id));
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let mut files = Vec::new();
                for (name, kind, code) in &prep.modules {
                    let (file, content) = module_file(name, *kind, code);
                    std::fs::write(dir.join(&file), content).map_err(|e| e.to_string())?;
                    files.push(file);
                }
                std::fs::write(
                    dir.join("OracleEntry.bas"),
                    entry_module_source(&prep.entry_module, &prep.entry),
                )
                .map_err(|e| e.to_string())?;
                files.push("OracleEntry.bas".into());
                e["status"] = "capturable".into();
                e["dir"] = format!("programs/{}", bundle_dir_name(&p.id)).into();
                e["files"] = files.into();
            }
        }
        entries.push(e);
    }
    let manifest = json!({"schema": 1, "programs": entries});
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(manifest)
}

// ---------------------------------------------------------------- engine

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub output: String,
    pub error: Option<String>,
    /// Output produced while the engine shut down, after the entry procedure returned.
    pub teardown_output: bool,
}

/// Runs a program through our engine (no grants: capturable programs need none).
pub fn run_engine(p: &Program) -> Result<Outcome, String> {
    let mut project = Project::new();
    for (name, kind, src) in &p.modules {
        let k = match kind {
            Kind::Standard => VbaModuleKind::Standard,
            Kind::Class => VbaModuleKind::Class,
            Kind::Document => VbaModuleKind::Document,
        };
        project
            .add_module(name, k, src)
            .map_err(|e| format!("cannot add module {name}: {e}"))?;
    }
    let mut engine = Engine::new();
    for g in &p.grants {
        let cap = match g.split('=').next().unwrap_or("") {
            "file-read" => Capability::FileRead,
            "file-write" => Capability::FileWrite,
            other => return Err(format!("unsupported grant `{other}` in the oracle tooling")),
        };
        engine = match g.split_once('=') {
            Some((_, prefix)) => engine.grant_prefixes(cap, [prefix.to_owned()]),
            None => engine.grant(cap),
        };
    }
    if let Some(n) = p.max_steps {
        engine = engine.max_steps(n);
    }
    let mut inst = engine
        .load(&project)
        .map_err(|e| format!("load failed: {e}"))?;
    inst.trust();
    let result = inst.run(&p.entry, vec![]).map(|_| ());
    let before_shutdown = inst.output();
    let _ = inst.shutdown();
    let output = inst.output();
    let teardown_output = output != before_shutdown;
    let error = match &result {
        Ok(_) => None,
        Err(Error::Run(RunError::Vba(r) | RunError::Host(r) | RunError::PermissionDenied(r))) => {
            Some(r.number.to_string())
        }
        Err(Error::Run(RunError::Limit(_))) => Some("limit".to_owned()),
        Err(Error::Run(RunError::Cancelled(_))) => Some("cancelled".to_owned()),
        Err(Error::Run(RunError::Bridge(_))) => Some("bridge".to_owned()),
        Err(Error::Run(RunError::Unsupported(_))) => Some("unsupported".to_owned()),
        Err(e) => return Err(format!("unexpected failure: {e}")),
    };
    Ok(Outcome {
        output,
        error,
        teardown_output,
    })
}

// --------------------------------------------------------------- results

#[derive(Debug, Clone)]
pub struct OracleResult {
    pub id: String,
    pub env: String,
    pub status: String,
    pub output: Option<String>,
    pub error_number: Option<i64>,
    pub error_description: Option<String>,
    pub reason: Option<String>,
    pub digest: Option<String>,
}

fn opt_str(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

/// Parses a results file (JSON lines). The last line for an id wins.
pub fn parse_results(text: &str) -> Result<BTreeMap<String, OracleResult>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut m = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value =
            serde_json::from_str(line).map_err(|e| format!("results line {}: {e}", n + 1))?;
        let id = opt_str(&v, "id").ok_or(format!("results line {}: no id", n + 1))?;
        let status = opt_str(&v, "status").ok_or(format!("results line {}: no status", n + 1))?;
        m.insert(
            id.clone(),
            OracleResult {
                id,
                env: opt_str(&v, "env").unwrap_or_default(),
                status,
                output: opt_str(&v, "output"),
                error_number: v.get("error_number").and_then(Value::as_i64),
                error_description: opt_str(&v, "error_description"),
                reason: opt_str(&v, "reason"),
                digest: opt_str(&v, "digest"),
            },
        );
    }
    Ok(m)
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub env_id: String,
    pub host: String,
    pub version: String,
    pub build: String,
    pub vba_version: String,
    pub bitness: String,
    pub os_version: String,
    pub locale_name: String,
    pub lcid: String,
    pub locale_override: bool,
    pub captured_at: String,
}

pub fn parse_environment(text: &str) -> Result<Environment, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let v: Value = serde_json::from_str(text).map_err(|e| format!("environment: {e}"))?;
    let g = |k: &str| {
        v.get(k)
            .map(|x| match x {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .ok_or(format!("environment: missing `{k}`"))
    };
    let env_id = g("env_id")?;
    if env_id.is_empty()
        || !env_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    {
        return Err(format!("environment: bad env_id `{env_id}`"));
    }
    Ok(Environment {
        env_id,
        host: g("host")?,
        version: g("version")?,
        build: g("build")?,
        vba_version: g("vba_version").unwrap_or_default(),
        bitness: g("bitness")?,
        os_version: g("os_version")?,
        locale_name: g("locale_name")?,
        lcid: g("lcid")?,
        locale_override: v
            .get("locale_override")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        captured_at: g("captured_at")?,
    })
}

// ------------------------------------------------------------- comparison

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Match,
    Mismatch,
    NotCapturable,
    ErrorDiffers,
    OracleFailed,
    NoResult,
    Stale,
}

impl Verdict {
    pub const ALL: [Verdict; 7] = [
        Verdict::Match,
        Verdict::Mismatch,
        Verdict::ErrorDiffers,
        Verdict::NotCapturable,
        Verdict::OracleFailed,
        Verdict::NoResult,
        Verdict::Stale,
    ];
    pub fn name(&self) -> &'static str {
        match self {
            Verdict::Match => "MATCH",
            Verdict::Mismatch => "MISMATCH",
            Verdict::NotCapturable => "NOT-CAPTURABLE",
            Verdict::ErrorDiffers => "ERROR-DIFFERS",
            Verdict::OracleFailed => "ORACLE-FAILED",
            Verdict::NoResult => "NO-RESULT",
            Verdict::Stale => "STALE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub area: String,
    pub provenance: String,
    pub xfail: bool,
    pub verdict: Verdict,
    pub detail: String,
}

/// Line ends are not significant: the log file ends lines with CR LF.
pub fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n")
}

const CP1252_HIGH: &str = "€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ";

/// True when the ANSI log can carry the character exactly.
pub fn cp1252_exact(c: char) -> bool {
    c.is_ascii() || ('\u{a0}'..='\u{ff}').contains(&c) || CP1252_HIGH.contains(c)
}

/// A line diff of oracle (`-`) against engine (`+`); each line is shown between bars.
pub fn line_diff(oracle: &str, engine: &str) -> String {
    let a: Vec<&str> = oracle.split('\n').collect();
    let b: Vec<&str> = engine.split('\n').collect();
    let (n, m) = (a.len(), b.len());
    if n * m > 4_000_000 {
        return "(output too large to diff)\n".into();
    }
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut s = String::new();
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            let _ = writeln!(s, "  |{}|", a[i]);
            i += 1;
            j += 1;
        } else if j >= m || (i < n && lcs[i + 1][j] >= lcs[i][j + 1]) {
            let _ = writeln!(s, "- |{}|", a[i]);
            i += 1;
        } else {
            let _ = writeln!(s, "+ |{}|", b[j]);
            j += 1;
        }
    }
    s
}

fn provenance_name(p: &Program) -> String {
    p.source_kind.clone()
}

/// Compares every program of the corpus with the oracle results.
pub fn compare(
    progs: &[Program],
    skip: &BTreeMap<String, (String, String)>,
    results: &BTreeMap<String, OracleResult>,
) -> Vec<Row> {
    let mut rows = Vec::new();
    for p in progs {
        let mut row = Row {
            id: p.id.clone(),
            area: p.area().to_owned(),
            provenance: provenance_name(p),
            xfail: p.xfail,
            verdict: Verdict::NotCapturable,
            detail: String::new(),
        };
        let refusal = skip
            .get(&p.id)
            .map(|(c, r)| format!("{c}: {r}"))
            .or_else(|| {
                classify(p).err().map(|r| {
                    r.iter()
                        .map(|x| format!("{}: {}", x.category, x.reason))
                        .collect::<Vec<_>>()
                        .join("; ")
                })
            });
        if let Some(r) = refusal {
            row.detail = r;
            rows.push(row);
            continue;
        }
        let Some(res) = results.get(&p.id) else {
            row.verdict = Verdict::NoResult;
            row.detail = "no line for this program in the results file".into();
            rows.push(row);
            continue;
        };
        if res.digest.as_deref().is_some_and(|d| d != p.digest()) {
            row.verdict = Verdict::Stale;
            row.detail = "the program changed since the bundle was prepared".into();
            rows.push(row);
            continue;
        }
        match res.status.as_str() {
            "not-capturable" => {
                row.detail = res
                    .reason
                    .clone()
                    .unwrap_or_else(|| "the driver refused the program".into());
                rows.push(row);
                continue;
            }
            "ok" | "error" | "ended" => {}
            other => {
                row.verdict = Verdict::OracleFailed;
                row.detail = format!(
                    "oracle status `{other}`: {}",
                    res.error_description.clone().unwrap_or_default()
                );
                rows.push(row);
                continue;
            }
        }
        let engine = match run_engine(p) {
            Ok(o) => o,
            Err(e) => {
                row.verdict = Verdict::Mismatch;
                row.detail = format!("our engine failed to run the program: {e}");
                rows.push(row);
                continue;
            }
        };
        if engine.output.chars().any(|c| !cp1252_exact(c)) {
            row.detail = "our output has characters the ANSI log cannot carry exactly".into();
            rows.push(row);
            continue;
        }
        let oracle_out = normalize(res.output.as_deref().unwrap_or(""));
        let engine_out = normalize(&engine.output);
        let oracle_err = if res.status == "error" {
            res.error_number.map(|n| n.to_string())
        } else {
            None
        };
        let out_same = oracle_out == engine_out;
        let err_same = oracle_err == engine.error;
        let mut detail = String::new();
        if !err_same {
            let _ = writeln!(
                detail,
                "error: oracle {} (\"{}\"), engine {}",
                oracle_err.as_deref().unwrap_or("none"),
                res.error_description.as_deref().unwrap_or(""),
                engine.error.as_deref().unwrap_or("none")
            );
        }
        if !out_same {
            detail.push_str("output diff (- oracle, + engine):\n");
            detail.push_str(&line_diff(&oracle_out, &engine_out));
        }
        row.verdict = match (out_same, err_same) {
            (true, true) => Verdict::Match,
            (_, false) => Verdict::ErrorDiffers,
            (false, true) => Verdict::Mismatch,
        };
        row.detail = detail;
        rows.push(row);
    }
    rows
}

fn count(rows: &[Row], f: impl Fn(&Row) -> bool, v: &Verdict) -> usize {
    rows.iter().filter(|r| f(r) && &r.verdict == v).count()
}

/// The markdown comparison report.
pub fn render_report(rows: &[Row], env: Option<&Environment>) -> String {
    let mut s = String::from("# Oracle comparison report\n\n");
    if let Some(e) = env {
        let _ = writeln!(
            s,
            "Environment `{}`: {} {} build {} (VBA {}), {}, OS {}, locale {} (LCID {}){}, captured {}.\n",
            e.env_id,
            e.host,
            e.version,
            e.build,
            e.vba_version,
            e.bitness,
            e.os_version,
            e.locale_name,
            e.lcid,
            if e.locale_override {
                " [LOCALE OVERRIDE]"
            } else {
                ""
            },
            e.captured_at
        );
    }
    let _ = writeln!(s, "## Summary\n\n| Verdict | Programs |\n| --- | ---: |");
    for v in Verdict::ALL {
        let _ = writeln!(s, "| {} | {} |", v.name(), count(rows, |_| true, &v));
    }
    let _ = writeln!(s, "| total | {} |", rows.len());

    let header = |s: &mut String, first: &str| {
        let _ = write!(s, "| {first} | programs |");
        for v in Verdict::ALL {
            let _ = write!(s, " {} |", v.name());
        }
        let _ = writeln!(s);
        let _ = write!(s, "| --- | ---: |");
        for _ in Verdict::ALL {
            let _ = write!(s, " ---: |");
        }
        let _ = writeln!(s);
    };
    let _ = writeln!(s, "\n## By area\n");
    header(&mut s, "area");
    let areas: BTreeSet<&str> = rows.iter().map(|r| r.area.as_str()).collect();
    for a in areas {
        let n = rows.iter().filter(|r| r.area == a).count();
        let _ = write!(s, "| {a} | {n} |");
        for v in Verdict::ALL {
            let _ = write!(s, " {} |", count(rows, |r| r.area == a, &v));
        }
        let _ = writeln!(s);
    }
    let _ = writeln!(s, "\n## By current provenance\n");
    header(&mut s, "provenance");
    let provs: BTreeSet<&str> = rows.iter().map(|r| r.provenance.as_str()).collect();
    for pv in provs {
        let n = rows.iter().filter(|r| r.provenance == pv).count();
        let _ = write!(s, "| {pv} | {n} |");
        for v in Verdict::ALL {
            let _ = write!(s, " {} |", count(rows, |r| r.provenance == pv, &v));
        }
        let _ = writeln!(s);
    }
    for v in [
        Verdict::Mismatch,
        Verdict::ErrorDiffers,
        Verdict::OracleFailed,
        Verdict::Stale,
        Verdict::NoResult,
    ] {
        let list: Vec<&Row> = rows.iter().filter(|r| r.verdict == v).collect();
        if list.is_empty() {
            continue;
        }
        let _ = writeln!(s, "\n## {}\n", v.name());
        for r in list {
            let _ = writeln!(
                s,
                "### `{}` (provenance {}{})\n\n```text\n{}```\n",
                r.id,
                r.provenance,
                if r.xfail { ", known deviation" } else { "" },
                if r.detail.ends_with('\n') || r.detail.is_empty() {
                    r.detail.clone()
                } else {
                    format!("{}\n", r.detail)
                }
            );
        }
    }
    let nc: Vec<&Row> = rows
        .iter()
        .filter(|r| r.verdict == Verdict::NotCapturable)
        .collect();
    if !nc.is_empty() {
        let _ = writeln!(s, "\n## NOT-CAPTURABLE\n");
        for r in nc {
            let _ = writeln!(s, "* `{}`: {}", r.id, r.detail);
        }
    }
    s
}

// ------------------------------------------------------------------ apply

/// What `apply` did.
#[derive(Debug, Default)]
pub struct ApplyOutcome {
    pub upgraded: Vec<String>,
    pub already_oracle: Vec<String>,
    pub environment_row_added: bool,
    pub mismatch_report: Option<PathBuf>,
}

const ENV_HEADER: &str = "# Oracle environments\n\n\
Append-only record of the real-VBA environments whose results were applied to the corpus\n\
(see `docs/tooling/differential-testing.md`). A program with `' SOURCE: oracle <env id> ...`\n\
was confirmed by exactly that environment. VBA behavior depends on host version, build,\n\
bitness and locale, so none of these may be omitted or edited after the fact.\n\n\
| Env id | Host | Version | Build | VBA | Bitness | OS | Locale (LCID) | Captured | Upgraded | Not matching |\n\
| --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: | ---: |\n";

/// Upgrades provenance, appends the environment and writes the mismatch report.
///
/// `oracle_dir` is `tests/oracle`; the corpus files live at `Program::path`.
pub fn apply(
    progs: &[Program],
    rows: &[Row],
    env: &Environment,
    oracle_dir: &Path,
    allow_locale: bool,
) -> Result<ApplyOutcome, String> {
    if (env.locale_name != "en-US" || env.locale_override) && !allow_locale {
        return Err(format!(
            "environment `{}` ran with locale {} (override: {}); the documented locale is en-US. \
             Use --allow-locale to record it anyway",
            env.env_id, env.locale_name, env.locale_override
        ));
    }
    let mut out = ApplyOutcome::default();
    let by_id: BTreeMap<&str, &Program> = progs.iter().map(|p| (p.id.as_str(), p)).collect();
    for r in rows.iter().filter(|r| r.verdict == Verdict::Match) {
        let p = by_id[r.id.as_str()];
        if p.xfail {
            continue;
        }
        if p.source_kind == "oracle" {
            out.already_oracle.push(p.id.clone());
            continue;
        }
        let text = std::fs::read_to_string(&p.path).map_err(|e| e.to_string())?;
        let mut done = false;
        let mut new_text = String::with_capacity(text.len() + 64);
        for line in text.split_inclusive('\n') {
            if !done && line.starts_with("' SOURCE:") {
                let (body, eol) = match line.strip_suffix("\r\n") {
                    Some(b) => (b, "\r\n"),
                    None => match line.strip_suffix('\n') {
                        Some(b) => (b, "\n"),
                        None => (line, ""),
                    },
                };
                let was = body.trim_start_matches("' SOURCE:").trim();
                let _ = write!(new_text, "' SOURCE: oracle {} was: {was}{eol}", env.env_id);
                done = true;
            } else {
                new_text.push_str(line);
            }
        }
        if !done {
            return Err(format!("{}: no `' SOURCE:` line to rewrite", p.id));
        }
        std::fs::write(&p.path, new_text).map_err(|e| e.to_string())?;
        out.upgraded.push(p.id.clone());
    }
    // ENVIRONMENTS.md: append-only.
    let envs = oracle_dir.join("ENVIRONMENTS.md");
    let mut text = std::fs::read_to_string(&envs).unwrap_or_else(|_| ENV_HEADER.to_owned());
    if !text.contains(&format!("| `{}` |", env.env_id)) {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let not_matching = rows
            .iter()
            .filter(|r| {
                matches!(
                    r.verdict,
                    Verdict::Mismatch | Verdict::ErrorDiffers | Verdict::OracleFailed
                )
            })
            .count();
        let _ = writeln!(
            text,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} ({}){} | {} | {} | {} |",
            env.env_id,
            env.host,
            env.version,
            env.build,
            env.vba_version,
            env.bitness,
            env.os_version,
            env.locale_name,
            env.lcid,
            if env.locale_override { " override" } else { "" },
            env.captured_at,
            out.upgraded.len() + out.already_oracle.len(),
            not_matching
        );
        std::fs::write(&envs, text).map_err(|e| e.to_string())?;
        out.environment_row_added = true;
    }
    // Mismatches are reported, never auto-fixed.
    let bad: Vec<Row> = rows
        .iter()
        .filter(|r| {
            matches!(
                r.verdict,
                Verdict::Mismatch | Verdict::ErrorDiffers | Verdict::OracleFailed | Verdict::Stale
            )
        })
        .cloned()
        .collect();
    if !bad.is_empty() {
        let dir = oracle_dir.join("mismatches");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.md", env.env_id));
        let mut s = render_report(&bad, Some(env));
        s.insert_str(
            0,
            "<!-- Generated by `oracle apply`. Triage each entry: fix the engine, or document an intentional difference. Do not edit expectations to hide a difference. -->\n\n",
        );
        std::fs::write(&path, s).map_err(|e| e.to_string())?;
        out.mismatch_report = Some(path);
    }
    Ok(out)
}

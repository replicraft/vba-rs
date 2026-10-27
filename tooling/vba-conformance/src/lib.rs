//! The conformance corpus runner.
//!
//! Runs every program under a corpus directory (`tests/vba/` in this repository)
//! through the public facade and compares its output with the expectation
//! embedded in the program. The engine is supplied by the caller as a factory,
//! so an embedding application can run the same corpus under its own engine
//! configuration (policy, limits, host objects). See
//! `docs/tooling/conformance.md` for the semantics.
//!
//! Program format (comment lines at the top of a `.bas` file):
//!
//! ```text
//! ' INV: stmt.if, op.eq          inventory ids this program tests
//! ' SOURCE: doc https://...      spec <ref> | doc <url> | derived <why> | unverified <note>
//! ' EXPECT: text                 one line of Debug.Print output (repeatable, exact text)
//! ' ENTRY: Main                  procedure to run (default Main)
//! ' ERROR: 11                    the run must end with this unhandled VBA error number,
//!                                or `load` (does not compile), `limit`, `cancelled`, `unsupported`
//! ' GRANT: file-write            capability grants (repeatable; `name=prefix` scopes it)
//! ' REQUIRES: host-clock         host features the program needs (comma list, repeatable);
//!                                the program is skipped when the run does not provide one
//! ' MAX_STEPS: 100000            step budget
//! ' CLOCK: 2024-06-15 12:30:00   fix the clock (default 2000-01-01 00:00:00)
//! ' COMMAND: a b                 the text `Command` returns (default: empty)
//! ' INPUTBOX: answer             one queued answer per `InputBox` call (repeatable, in call
//!                                order; `<cancel>` is the Cancel button, `<default>`, `<prompt>`
//!                                and `<title>` answer with that argument of the call; an
//!                                empty queue is error 70 as in a headless host)
//! ' STRICT_SPEC: true            analyse with `Engine::strict_spec(true)`: the spec-strict
//!                                diagnostics are errors (default: warnings, the program runs)
//! ' XFAIL: wrong|unsupported why known deviation: the program must still fail
//! ```
//!
//! A line `'#MODULE Name standard|class` starts another module in the same file.

use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use vba::automation::ErrInfo;
use vba::runtime::Error as RunError;
use vba::{
    Capability, Engine, Error, HostResult, InputBoxRequest, ModuleKind, MsgBoxRequest, Project,
    UiHooks,
};

// ---------------------------------------------------------------- programs

/// Where an expected value comes from; the order is strongest to weakest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Provenance {
    /// Confirmed by running the program in real VBA.
    Oracle,
    /// Stated by MS-VBAL.
    Spec,
    /// Stated by a Microsoft language-reference page.
    Doc,
    /// Derived by reasoning from documented semantics.
    Derived,
    /// Only the current behavior of the implementation.
    Unverified,
}

impl Provenance {
    /// The name used in the `SOURCE` header and the inventory.
    pub fn name(self) -> &'static str {
        match self {
            Provenance::Oracle => "oracle",
            Provenance::Spec => "spec",
            Provenance::Doc => "doc",
            Provenance::Derived => "derived",
            Provenance::Unverified => "unverified",
        }
    }
    /// Every provenance, strongest first.
    pub const ALL: [Provenance; 5] = [
        Provenance::Oracle,
        Provenance::Spec,
        Provenance::Doc,
        Provenance::Derived,
        Provenance::Unverified,
    ];
}

/// The kind of a recorded known deviation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Xfail {
    /// The implementation produces a different result.
    Wrong,
    /// The construct is not supported.
    Unsupported,
}

/// One parsed corpus program.
#[derive(Debug)]
pub struct Program {
    /// The path below the corpus directory, without extension, with `/`.
    pub id: String,
    /// Inventory ids the program tests.
    pub inv: Vec<String>,
    /// Provenance of the expected values.
    pub provenance: Provenance,
    /// The reference or note of the `SOURCE` header.
    pub source_ref: String,
    /// Expected `Debug.Print` lines.
    pub expect: Vec<String>,
    /// Procedure to run.
    pub entry: String,
    /// Expected unhandled error (`11`, `load`, `limit`, ...), if any.
    pub error: Option<String>,
    /// Capability grants (`name` or `name=prefix`).
    pub grants: Vec<String>,
    /// Host features the program needs (`REQUIRES`).
    pub requires: Vec<String>,
    /// Step budget override.
    pub max_steps: Option<u64>,
    /// Analyse with the spec-strict diagnostics as errors.
    pub strict_spec: bool,
    /// The text `Command` returns (`COMMAND`).
    pub command: Option<String>,
    /// Queued `InputBox` answers (`INPUTBOX`); `None` leaves the engine's own UI.
    pub inputbox: Option<Vec<String>>,
    /// Fixed clock `(year, month, day, hour, minute, second)`.
    pub clock: Option<(i32, u32, u32, u32, u32, u32)>,
    /// A recorded known deviation and its reason.
    pub xfail: Option<(Xfail, String)>,
    /// Events to raise after the entry procedure: `(source, event)`.
    pub raises: Vec<(String, String)>,
    /// The modules `(name, kind, source text)`.
    pub modules: Vec<(String, ModuleKind, String)>,
}

impl Program {
    /// Every host feature the program needs: the `REQUIRES` tokens plus an
    /// implied `grant:<capability>` for each `GRANT`.
    pub fn required_features(&self) -> Vec<String> {
        let mut out: Vec<String> = self.requires.clone();
        for g in &self.grants {
            let name = g.split_once('=').map_or(g.as_str(), |(n, _)| n);
            out.push(format!("grant:{name}"));
        }
        out
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "bas") {
            out.push(p);
        }
    }
}

fn capability(name: &str) -> Option<Capability> {
    Some(match name {
        "file-read" => Capability::FileRead,
        "file-write" => Capability::FileWrite,
        "process-spawn" => Capability::ProcessSpawn,
        "network" => Capability::Network,
        "com-create" => Capability::ComCreate,
        "native-declare" => Capability::NativeDeclare,
        "ui-automation" => Capability::UiAutomation,
        _ => return None,
    })
}

/// Parses one program; `base` is the corpus directory (it names the id).
pub fn parse_program(base: &Path, path: &Path) -> Result<Program, String> {
    let id = path
        .strip_prefix(base)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    let text = std::fs::read_to_string(path).map_err(|e| format!("{id}: {e}"))?;
    let mut p = Program {
        id: id.clone(),
        inv: Vec::new(),
        provenance: Provenance::Unverified,
        source_ref: String::new(),
        expect: Vec::new(),
        entry: "Main".into(),
        error: None,
        grants: Vec::new(),
        requires: Vec::new(),
        max_steps: None,
        strict_spec: false,
        clock: None,
        command: None,
        inputbox: None,
        xfail: None,
        raises: Vec::new(),
        modules: Vec::new(),
    };
    let mut have_source = false;
    let mut current: (String, ModuleKind, String) =
        ("Module1".into(), ModuleKind::Standard, String::new());
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("'#MODULE ") {
            let mut it = rest.split_whitespace();
            let name = it.next().ok_or(format!("{id}: #MODULE without a name"))?;
            let kind = match it.next() {
                Some("standard") | None => ModuleKind::Standard,
                Some("class") => ModuleKind::Class,
                Some("document") => ModuleKind::Document,
                Some(k) => return Err(format!("{id}: unknown module kind `{k}`")),
            };
            let previous = std::mem::replace(&mut current, (name.to_owned(), kind, String::new()));
            // The header comments before the first marker are not a module.
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
            && key.chars().all(|c| c.is_ascii_uppercase() || c == '_')
            && !key.is_empty()
        {
            let value = value.strip_prefix(' ').unwrap_or(value);
            let value = value.trim_end_matches('\r');
            match key {
                "INV" => p.inv.extend(
                    value
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty()),
                ),
                "SOURCE" => {
                    have_source = true;
                    let (kind, r) = value.split_once(' ').unwrap_or((value, ""));
                    p.provenance = match kind {
                        "oracle" => Provenance::Oracle,
                        "spec" => Provenance::Spec,
                        "doc" => Provenance::Doc,
                        "derived" => Provenance::Derived,
                        "unverified" => Provenance::Unverified,
                        k => return Err(format!("{id}: unknown provenance `{k}`")),
                    };
                    if r.trim().is_empty() {
                        return Err(format!("{id}: SOURCE needs a reference or note"));
                    }
                    p.source_ref = r.trim().to_owned();
                }
                "EXPECT" => p.expect.push(value.to_owned()),
                "ENTRY" => p.entry = value.to_owned(),
                "ERROR" => p.error = Some(value.trim().to_owned()),
                "RAISE" => {
                    let (a, b) = value
                        .split_once(' ')
                        .ok_or(format!("{id}: RAISE needs a source and an event"))?;
                    p.raises.push((a.to_owned(), b.trim().to_owned()));
                }
                "GRANT" => {
                    let g = value.trim();
                    let name = g.split_once('=').map_or(g, |(n, _)| n);
                    if capability(name).is_none() {
                        return Err(format!("{id}: unknown capability `{name}`"));
                    }
                    p.grants.push(g.to_owned());
                }
                "REQUIRES" => {
                    let tokens: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if tokens.is_empty() {
                        return Err(format!("{id}: REQUIRES needs at least one feature"));
                    }
                    p.requires.extend(tokens);
                }
                "MAX_STEPS" => {
                    p.max_steps = Some(
                        value
                            .trim()
                            .parse()
                            .map_err(|_| format!("{id}: bad MAX_STEPS"))?,
                    )
                }
                "CLOCK" => {
                    let n: Vec<u32> = value
                        .split(|c: char| !c.is_ascii_digit())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse().map_err(|_| format!("{id}: bad CLOCK")))
                        .collect::<Result<_, _>>()?;
                    if n.len() < 3 || n.len() > 6 {
                        return Err(format!("{id}: CLOCK is `Y-M-D [h:m:s]`"));
                    }
                    let at = |i: usize| n.get(i).copied().unwrap_or(0);
                    p.clock = Some((n[0] as i32, n[1], n[2], at(3), at(4), at(5)));
                }
                "COMMAND" => p.command = Some(value.to_owned()),
                "INPUTBOX" => p
                    .inputbox
                    .get_or_insert_with(Vec::new)
                    .push(value.to_owned()),
                "STRICT_SPEC" => p.strict_spec = value.trim() == "true",
                "XFAIL" => {
                    let (k, why) = value.split_once(' ').unwrap_or((value, ""));
                    let kind = match k {
                        "wrong" => Xfail::Wrong,
                        "unsupported" => Xfail::Unsupported,
                        _ => return Err(format!("{id}: XFAIL must start with wrong|unsupported")),
                    };
                    p.xfail = Some((kind, why.to_owned()));
                }
                _ => {}
            }
        }
        current.2.push_str(line);
        current.2.push_str("\r\n");
    }
    p.modules.push(current);
    if !have_source {
        return Err(format!("{id}: missing SOURCE"));
    }
    if p.inv.is_empty() {
        return Err(format!("{id}: missing INV"));
    }
    if matches!(p.provenance, Provenance::Spec) && !p.source_ref.starts_with("ms-vbal") {
        return Err(format!("{id}: a `spec` SOURCE must cite ms-vbal"));
    }
    if matches!(p.provenance, Provenance::Doc) && !p.source_ref.starts_with("https://") {
        return Err(format!("{id}: a `doc` SOURCE must cite a URL"));
    }
    Ok(p)
}

/// Parses every `.bas` program below `corpus`, in path order. `Err` lists the
/// malformed programs.
pub fn load_programs(corpus: &Path) -> Result<Vec<Program>, Vec<String>> {
    let mut files = Vec::new();
    collect(corpus, &mut files);
    let mut errors = Vec::new();
    let mut out = Vec::new();
    for f in files {
        match parse_program(corpus, &f) {
            Ok(p) => out.push(p),
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

// ------------------------------------------------------------------- runs

/// The UI of a program with `INPUTBOX` headers: `InputBox` takes the queued
/// answers in order, `MsgBox` stays unavailable (error 70).
struct QueuedUi {
    answers: RefCell<VecDeque<String>>,
}

impl QueuedUi {
    fn new(answers: &[String]) -> Self {
        Self {
            answers: RefCell::new(answers.iter().cloned().collect()),
        }
    }

    fn unavailable(what: &str) -> ErrInfo {
        ErrInfo::new(70, format!("{what} is not available: no queued answer"))
    }
}

impl UiHooks for QueuedUi {
    fn msg_box(&self, _: &MsgBoxRequest) -> HostResult<i32> {
        Err(Self::unavailable("MsgBox"))
    }

    fn input_box(&self, req: &InputBoxRequest) -> HostResult<Option<String>> {
        let Some(answer) = self.answers.borrow_mut().pop_front() else {
            return Err(Self::unavailable("InputBox"));
        };
        Ok(match answer.as_str() {
            "<cancel>" => None,
            "<default>" => Some(req.default.clone()),
            "<prompt>" => Some(req.prompt.clone()),
            "<title>" => Some(req.title.clone()),
            _ => Some(answer),
        })
    }
}

/// Runs one program on an engine from `factory`; `Err` describes the mismatch.
///
/// The program's own settings (`STRICT_SPEC`, `GRANT`, `CLOCK`, `MAX_STEPS`)
/// are applied on top of the factory's engine.
pub fn run_program(p: &Program, factory: &dyn Fn() -> Engine) -> Result<(), String> {
    let mut project = Project::new();
    for (name, kind, src) in &p.modules {
        project
            .add_module(name, *kind, src)
            .map_err(|e| format!("cannot add module {name}: {e}"))?;
    }
    let mut engine = factory().strict_spec(p.strict_spec);
    for g in &p.grants {
        let (name, prefix) = match g.split_once('=') {
            Some((n, prefix)) => (n, Some(prefix)),
            None => (g.as_str(), None),
        };
        let cap = capability(name).ok_or(format!("unknown capability `{name}`"))?;
        engine = match prefix {
            Some(prefix) => engine.grant_prefixes(cap, [prefix.to_owned()]),
            None => engine.grant(cap),
        };
    }
    if let Some((y, mo, d, h, mi, s)) = p.clock {
        let now = vba::Date::from_ymd_hms(y, mo, d, h, mi, s)
            .map_err(|_| format!("{}: bad CLOCK", p.id))?;
        engine = engine.fixed_clock(now);
    }
    if let Some(c) = &p.command {
        engine = engine.command_line(c);
    }
    if let Some(answers) = &p.inputbox {
        engine = engine.ui(QueuedUi::new(answers));
    }
    // A program that does not terminate must fail, not hang the suite.
    engine = engine.max_steps(p.max_steps.unwrap_or(20_000_000));
    let report = engine.analyze(&project);
    if p.error.as_deref() == Some("load") {
        return if report.has_errors() {
            Ok(())
        } else {
            Err("expected the project not to compile, but it did".into())
        };
    }
    let mut inst = engine
        .load(&project)
        .map_err(|e| format!("load failed: {e}"))?;
    inst.trust();
    let mut result = inst.run(&p.entry, vec![]).map(|_| ());
    if result.is_ok() {
        for (src, ev) in &p.raises {
            if let Err(e) = inst.raise_event(src, ev, vec![]) {
                result = Err(e);
                break;
            }
        }
    }
    let _ = inst.shutdown();
    let output = inst.output();
    let expected: String = p.expect.iter().map(|l| format!("{l}\n")).collect();
    let actual_error = match &result {
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
    if actual_error != p.error {
        return Err(format!(
            "error mismatch: expected {:?}, got {:?} ({:?})\noutput so far: {:?}",
            p.error,
            actual_error,
            result.err().map(|e| e.to_string()),
            output
        ));
    }
    if output != expected {
        return Err(format!(
            "output mismatch\n--- expected\n{expected:?}\n--- actual\n{output:?}"
        ));
    }
    Ok(())
}

/// Which programs to run and which host features the run provides.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Run only programs whose id contains one of these substrings (all when empty).
    pub filter: Vec<String>,
    /// The host features the engine provides, matched against `REQUIRES` tokens
    /// and the implied `grant:<capability>` tokens. `None` (the default) means
    /// every feature is provided and nothing is skipped.
    pub provides: Option<BTreeSet<String>>,
}

impl Options {
    /// Run everything, with every feature provided (what the repository's own test does).
    pub fn all() -> Self {
        Self::default()
    }

    /// The features a program needs that this run does not provide.
    fn missing(&self, p: &Program) -> Vec<String> {
        match &self.provides {
            None => Vec::new(),
            Some(have) => p
                .required_features()
                .into_iter()
                .filter(|f| !have.contains(f))
                .collect(),
        }
    }
}

/// The outcome of one program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Matched its expectation.
    Passed,
    /// Did not match; the message says how.
    Failed(String),
    /// A recorded known deviation (`XFAIL`) that still fails, as it must.
    KnownDeviation,
    /// An `XFAIL` program that now passes: its `XFAIL` line must be removed.
    UnexpectedPass(String),
    /// Not run: the features it needs that the run does not provide.
    Skipped(Vec<String>),
}

/// The result of [`run_corpus`], in program order.
#[derive(Debug, Default)]
pub struct Report {
    /// `(program id, outcome)` for every selected program.
    pub outcomes: Vec<(String, Outcome)>,
}

impl Report {
    /// Programs that matched.
    pub fn passed(&self) -> usize {
        self.count(|o| matches!(o, Outcome::Passed))
    }
    /// Programs recorded as known deviations that still fail.
    pub fn known_deviations(&self) -> usize {
        self.count(|o| matches!(o, Outcome::KnownDeviation))
    }
    /// Programs not run because of missing host features.
    pub fn skipped(&self) -> usize {
        self.count(|o| matches!(o, Outcome::Skipped(_)))
    }
    /// Programs that failed (including unexpected passes of `XFAIL` programs).
    pub fn failed(&self) -> usize {
        self.count(|o| matches!(o, Outcome::Failed(_) | Outcome::UnexpectedPass(_)))
    }
    fn count(&self, f: impl Fn(&Outcome) -> bool) -> usize {
        self.outcomes.iter().filter(|(_, o)| f(o)).count()
    }
    /// True when no program failed (skips and known deviations are not failures).
    pub fn is_ok(&self) -> bool {
        self.failed() == 0
    }
    /// One message per failing program.
    pub fn failures(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .filter_map(|(id, o)| match o {
                Outcome::Failed(e) => Some(format!("{id}: {e}")),
                Outcome::UnexpectedPass(why) => Some(format!(
                    "{id}: expected to fail ({why}) but passes; remove its XFAIL"
                )),
                _ => None,
            })
            .collect()
    }
    /// A one-line count summary.
    pub fn summary(&self) -> String {
        format!(
            "{} programs: {} passed, {} failed, {} known deviations, {} skipped",
            self.outcomes.len(),
            self.passed(),
            self.failed(),
            self.known_deviations(),
            self.skipped()
        )
    }
}

/// Runs the programs of `programs` that `options` select, each on a fresh
/// engine from `factory`.
pub fn run_programs(
    programs: &[Program],
    factory: impl Fn() -> Engine,
    options: &Options,
) -> Report {
    let mut report = Report::default();
    for p in programs {
        if !options.filter.is_empty() && !options.filter.iter().any(|f| p.id.contains(f.as_str())) {
            continue;
        }
        let missing = options.missing(p);
        let outcome = if !missing.is_empty() {
            Outcome::Skipped(missing)
        } else {
            match (&p.xfail, run_program(p, &factory)) {
                (None, Ok(())) => Outcome::Passed,
                (None, Err(e)) => Outcome::Failed(e),
                (Some(_), Err(_)) => Outcome::KnownDeviation,
                (Some((_, why)), Ok(())) => Outcome::UnexpectedPass(why.clone()),
            }
        };
        report.outcomes.push((p.id.clone(), outcome));
    }
    report
}

/// Loads the corpus at `corpus` and runs it: the reusable entry point for an
/// embedding application. `Err` lists malformed programs.
pub fn run_corpus(
    corpus: &Path,
    factory: impl Fn() -> Engine,
    options: &Options,
) -> Result<Report, Vec<String>> {
    let programs = load_programs(corpus)?;
    Ok(run_programs(&programs, factory, options))
}

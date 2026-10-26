//! Shared helpers for the session tests.

#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use vba_host::{Capability, HeadlessHost, Host, Policy, ResourceLimits};
use vba_project::ModuleKind;
use vba_runtime::*;
use vba_sema::SourceModule;
use vba_vm::{BufferSink, RunOutput};

/// A `MemFs` the test keeps a handle to after the session took ownership.
#[derive(Clone, Default)]
pub struct SharedFs(pub Rc<RefCell<MemFs>>);

impl Vfs for SharedFs {
    fn canonical(&mut self, p: &str) -> String {
        self.0.borrow_mut().canonical(p)
    }
    fn initial_dir(&self) -> String {
        self.0.borrow().initial_dir()
    }
    fn stat(&mut self, p: &str) -> VfsResult<Option<Stat>> {
        self.0.borrow_mut().stat(p)
    }
    fn read(&mut self, p: &str) -> VfsResult<Vec<u8>> {
        self.0.borrow_mut().read(p)
    }
    fn write(&mut self, p: &str, d: &[u8]) -> VfsResult<()> {
        self.0.borrow_mut().write(p, d)
    }
    fn remove_file(&mut self, p: &str) -> VfsResult<()> {
        self.0.borrow_mut().remove_file(p)
    }
    fn rename(&mut self, a: &str, b: &str) -> VfsResult<()> {
        self.0.borrow_mut().rename(a, b)
    }
    fn copy(&mut self, a: &str, b: &str) -> VfsResult<()> {
        self.0.borrow_mut().copy(a, b)
    }
    fn create_dir(&mut self, p: &str) -> VfsResult<()> {
        self.0.borrow_mut().create_dir(p)
    }
    fn remove_dir(&mut self, p: &str) -> VfsResult<()> {
        self.0.borrow_mut().remove_dir(p)
    }
    fn list(&mut self, p: &str) -> VfsResult<Vec<DirEntry>> {
        self.0.borrow_mut().list(p)
    }
    fn set_attributes(&mut self, p: &str, a: u8) -> VfsResult<()> {
        self.0.borrow_mut().set_attributes(p, a)
    }
}

impl SharedFs {
    pub fn with(files: &[(&str, &str)]) -> SharedFs {
        let mut fs = MemFs::new();
        for (p, c) in files {
            fs = fs.with_file(p, c.as_bytes());
        }
        SharedFs(Rc::new(RefCell::new(fs)))
    }

    pub fn text(&self, path: &str) -> Option<String> {
        self.0
            .borrow_mut()
            .read(path)
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    pub fn bytes(&self, path: &str) -> Option<Vec<u8>> {
        self.0.borrow_mut().read(path).ok()
    }
}

pub fn module(name: &str, kind: ModuleKind, src: &str) -> SourceModule {
    SourceModule {
        name: name.to_owned(),
        kind,
        source: src.as_bytes().to_vec(),
    }
}

pub fn std_module(src: &str) -> SourceModule {
    module("M", ModuleKind::Standard, src)
}

pub fn all_files() -> Policy {
    Policy::deny_all()
        .grant(Capability::FileRead)
        .grant(Capability::FileWrite)
}

pub fn host(policy: Policy) -> Rc<dyn Host> {
    Rc::new(
        HeadlessHost::new()
            .with_policy(policy)
            .with_limits(ResourceLimits {
                max_steps: Some(5_000_000),
                ..ResourceLimits::default()
            }),
    )
}

pub struct Outcome {
    pub out: String,
    pub result: Result<RunOutput, Error>,
    pub session: Session,
    pub fs: SharedFs,
}

pub fn run_modules(mods: &[SourceModule], fs: SharedFs, policy: Policy, entry: &str) -> Outcome {
    let sink = BufferSink::new();
    let mut session = Session::builder()
        .host(host(policy))
        .vfs(fs.clone())
        .debug_sink(sink.clone())
        .build_sources(mods)
        .unwrap_or_else(|e| panic!("load failed: {e}"));
    session.trust();
    let result = session.run_procedure(entry, vec![]);
    Outcome {
        out: sink.contents(),
        result,
        session,
        fs,
    }
}

/// Runs `Main` of a single standard module.
pub fn run(src: &str, fs: SharedFs, policy: Policy) -> Outcome {
    run_modules(&[std_module(src)], fs, policy, "Main")
}

pub struct FCase {
    pub name: &'static str,
    pub src: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub out: &'static str,
    pub after: &'static [(&'static str, &'static str)],
}

pub const fn fcase(
    name: &'static str,
    src: &'static str,
    files: &'static [(&'static str, &'static str)],
    out: &'static str,
    after: &'static [(&'static str, &'static str)],
) -> FCase {
    FCase {
        name,
        src,
        files,
        out,
        after,
    }
}

/// Runs each case with the given initial files and file permissions on a
/// fresh session; compares the printed text and the final contents of the
/// listed files.
pub fn check_files(cases: &[FCase]) {
    let mut failures = Vec::new();
    for c in cases {
        let fs = SharedFs::with(c.files);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(c.src, fs.clone(), all_files())
        }));
        let o = match r {
            Ok(o) => o,
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                failures.push(format!("{}: {msg}", c.name));
                continue;
            }
        };
        if let Err(e) = &o.result {
            failures.push(format!("{}: failed with {e}; output {:?}", c.name, o.out));
            continue;
        }
        if o.out != c.out {
            failures.push(format!("{}: expected {:?}, got {:?}", c.name, c.out, o.out));
        }
        for (path, want) in c.after {
            let got = fs.text(path);
            if got.as_deref() != Some(*want) {
                failures.push(format!(
                    "{}: file {path}: expected {want:?}, got {got:?}",
                    c.name
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

//! `vbars run`: executes a procedure with explicitly granted capabilities.

use std::io::Write as _;
use std::path::PathBuf;
use std::rc::Rc;

use vba_host::{Capability, HeadlessHost, HostMetadata, Policy, ResourceLimits};
use vba_runtime::{Error, MemFs, RealFs, Session, StdShell, SystemHost};
use vba_value::Variant;
use vba_vm::{DebugSink, RunStatus};

use crate::load;
use crate::tools::{ToolError, profile_arg};

/// Exit codes of `vbars run`.
pub mod exit {
    /// The procedure ran to completion.
    pub const OK: u8 = 0;
    /// Bad input: the files cannot be read or the project does not compile.
    pub const INPUT: u8 = 1;
    /// An unhandled VBA run-time error, or an error reported by the host.
    pub const RUNTIME: u8 = 2;
    /// A capability that was not granted was needed.
    pub const DENIED: u8 = 3;
    /// A resource limit was exceeded or the run was cancelled.
    pub const LIMIT: u8 = 4;
    /// A feature that is not implemented, or an internal error.
    pub const UNSUPPORTED: u8 = 5;
    /// The project, or the module a call needs, has no source text
    /// (compiled-only or stripped).
    pub const COMPILED_ONLY: u8 = 6;
}

/// What `vbars run` was asked to do.
pub struct RunArgs {
    pub files: Vec<PathBuf>,
    pub entry: String,
    pub args: Vec<String>,
    pub grants: Vec<String>,
    pub profile: Option<String>,
    pub memfs: bool,
    pub max_steps: Option<u64>,
    pub max_memory: Option<u64>,
    pub max_call_depth: Option<u32>,
    pub unlimited: bool,
    pub system_env: bool,
    pub allow_shell: bool,
    pub command: Option<String>,
    pub allow_partial: bool,
}

struct StdoutSink;

impl DebugSink for StdoutSink {
    fn write(&mut self, text: &str) {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(text.as_bytes());
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

/// Turns `file-read` or `file-read=/dir` into a grant. The scope is written
/// the way the capability's resource is: a path for the file capabilities
/// (made absolute with forward slashes; an existing directory gets a trailing
/// slash so the grant covers its contents and nothing next to it), a program
/// for `process-spawn`, a ProgID (`Scripting.Dictionary`, `Scripting.*`) for
/// `com-create`, `library!procedure` or `library!*` for `native-declare`, an
/// action (`SendKeys`, `AppActivate`) for `ui-automation` and a host for
/// `network`.
fn apply_grant(policy: Policy, spec: &str) -> Result<Policy, String> {
    let (name, scope) = match spec.split_once('=') {
        Some((n, p)) => (n, Some(p)),
        None => (spec, None),
    };
    let cap = capability(name).ok_or_else(|| {
        format!(
            "unknown capability `{name}` (expected file-read, file-write, process-spawn, network, com-create, native-declare or ui-automation)"
        )
    })?;
    match scope {
        None => Ok(policy.grant(cap)),
        Some("") => Err(format!("`--grant {spec}` has an empty scope")),
        Some(p) => {
            vba_host::scope::validate_scope(cap, p)
                .map_err(|e| format!("`--grant {spec}`: {e}"))?;
            if !matches!(cap, Capability::FileRead | Capability::FileWrite) {
                return Ok(policy.grant_prefixes(cap, [p.to_owned()]));
            }
            let mut resolved = std::fs::canonicalize(p)
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or_else(|_| p.to_owned());
            resolved = resolved.replace('\\', "/");
            if std::path::Path::new(&resolved).is_dir() && !resolved.ends_with('/') {
                resolved.push('/');
            }
            Ok(policy.grant_prefixes(cap, [resolved]))
        }
    }
}

fn guess(arg: &str) -> Variant {
    if let Ok(n) = arg.parse::<i32>() {
        Variant::Long(n)
    } else if let Ok(f) = arg.parse::<f64>() {
        Variant::Double(f)
    } else {
        Variant::from_text(arg)
    }
}

fn exit_code(e: &Error) -> u8 {
    match e {
        Error::Load(_)
        | Error::NotTrusted
        | Error::NoSuchProcedure(_)
        | Error::AmbiguousProcedure(_)
        | Error::NotCallable(..) => exit::INPUT,
        Error::CompiledOnly { .. } => exit::COMPILED_ONLY,
        Error::Vba(_) | Error::Host(_) => exit::RUNTIME,
        Error::PermissionDenied(_) => exit::DENIED,
        Error::Limit(_) | Error::Cancelled(_) => exit::LIMIT,
        Error::Bridge(_) | Error::Unsupported(_) | Error::Internal(_) => exit::UNSUPPORTED,
    }
}

/// Runs the program; returns the exit code. Diagnostics and errors go to
/// standard error, `Debug.Print` to standard output.
pub fn run(a: RunArgs) -> u8 {
    let profile = match profile_arg(a.profile.as_deref()) {
        Ok(p) => p,
        Err(e @ ToolError::UnknownProfile(_)) => {
            eprintln!("error: {e}");
            return exit::INPUT;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return exit::INPUT;
        }
    };
    let mut policy = Policy::deny_all();
    for g in &a.grants {
        policy = match apply_grant(policy, g) {
            Ok(p) => p,
            Err(m) => {
                eprintln!("error: {m}");
                return exit::INPUT;
            }
        };
    }
    let loaded = match load::load(&a.files) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: {e}");
            return exit::INPUT;
        }
    };
    // The code is untrusted: finite limits unless the user opts out.
    let base = if a.unlimited {
        ResourceLimits::unlimited()
    } else {
        ResourceLimits::untrusted()
    };
    let limits = ResourceLimits {
        max_steps: a.max_steps.or(base.max_steps),
        max_memory_bytes: a.max_memory.or(base.max_memory_bytes),
        max_call_depth: a.max_call_depth.unwrap_or(base.max_call_depth),
    };
    let host = HeadlessHost::new()
        .with_metadata(HostMetadata::new(
            "vbars",
            env!("CARGO_PKG_VERSION"),
            profile,
        ))
        .with_policy(policy)
        .with_limits(limits);
    let mut builder = Session::builder()
        .host(Rc::new(host))
        .debug_sink(StdoutSink)
        .command_line(a.command.as_deref().unwrap_or(""));
    builder = if a.memfs {
        builder.vfs(MemFs::new())
    } else {
        builder.vfs(RealFs::new())
    };
    if a.system_env {
        builder = builder.std_host(SystemHost::new());
    }
    if a.allow_shell {
        builder = builder.shell(StdShell);
    }
    if a.allow_partial {
        builder = builder.allow_partial();
    }
    let built = match &loaded.project {
        Some(p) => builder.build_project(p),
        None => builder.build_sources(&loaded.modules),
    };
    let mut session = match built {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return exit_code(&e);
        }
    };
    for w in session.warnings() {
        eprintln!("{w}");
    }
    // Running is the explicit opt-in: `vbars run` is the embedder that
    // trusts the code it was told to run.
    session.trust();
    let args: Vec<Variant> = a.args.iter().map(|s| guess(s)).collect();
    let result = session.run_procedure(&a.entry, args);
    let shutdown = session.shutdown();
    let mut code = exit::OK;
    match result {
        Ok(out) => match out.status {
            RunStatus::Completed | RunStatus::Ended => {}
            RunStatus::Stopped => {
                let why = session.vm().stop_reason().unwrap_or("Stop").to_owned();
                eprintln!("stopped: {why}");
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            code = exit_code(&e);
        }
    }
    if let Err(e) = shutdown
        && code == exit::OK
    {
        eprintln!("error: {e}");
        code = exit_code(&e);
    }
    let _ = std::io::stdout().flush();
    code
}

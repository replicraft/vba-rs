//! [`Session`]: one loaded project with its own VM state.

use std::cell::RefCell;
use std::rc::Rc;

use vba_ast::{ProcKind, Visibility};
use vba_diagnostics::{Severity, render_plain};
use vba_host::{HeadlessHost, Host};
use vba_ir::IrProgram;
use vba_project::VbaProject;
use vba_sema::{NoLibraries, Options, SourceModule, TypeProvider};
use vba_value::{ObjectRef, Variant};
use vba_vm::{Arg, DebugSink, NullSink, RunOutput, Vm, VmConfig, VmError};

use crate::error::{Error, LoadError};
use crate::files::{FileSystem, FsServices};
use crate::memfs::MemFs;
use crate::shell::{NoShell, ShellHook};
use crate::vfs::Vfs;

/// A procedure that can be run by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureInfo {
    /// The module name.
    pub module: String,
    /// The procedure name.
    pub name: String,
    /// `Sub`, `Function` or a property kind.
    pub kind: ProcKind,
    /// Whether the procedure is `Public` (callable by name).
    pub public: bool,
    /// The number of parameters.
    pub params: usize,
    /// Whether the module is a class-like module (its procedures need an
    /// instance).
    pub class_like: bool,
}

/// Collects everything a [`Session`] needs; nothing runs until the embedder
/// builds the session, trusts it and asks it to run something.
pub struct SessionBuilder {
    host: Rc<dyn Host>,
    provider: Box<dyn TypeProvider>,
    vfs: Box<dyn Vfs>,
    std_host: Box<dyn vba_stdlib::Host>,
    debug: Box<dyn DebugSink>,
    shell: Box<dyn ShellHook>,
    command: String,
    config: VmConfig,
}

impl Default for SessionBuilder {
    fn default() -> Self {
        SessionBuilder::new()
    }
}

impl SessionBuilder {
    /// The defaults: a headless host that grants nothing, no libraries, an
    /// empty in-memory file system, a fixed clock (2000-01-01) and empty
    /// environment, `Debug.Print` discarded, no `Shell`.
    pub fn new() -> SessionBuilder {
        SessionBuilder {
            host: Rc::new(HeadlessHost::new()),
            provider: Box::new(NoLibraries),
            vfs: Box::new(MemFs::new()),
            std_host: Box::new(vba_stdlib::FixedHost::default()),
            debug: Box::new(NullSink),
            shell: Box::new(NoShell),
            command: String::new(),
            config: VmConfig::default(),
        }
    }

    /// The host: policy, limits, objects, UI and `Declare` hooks.
    #[must_use]
    pub fn host(mut self, host: Rc<dyn Host>) -> Self {
        self.host = host;
        self
    }

    /// The external type libraries visible to the analysis.
    #[must_use]
    pub fn provider(mut self, p: impl TypeProvider + 'static) -> Self {
        self.provider = Box::new(p);
        self
    }

    /// The file system the file statements run against.
    #[must_use]
    pub fn vfs(mut self, v: impl Vfs + 'static) -> Self {
        self.vfs = Box::new(v);
        self
    }

    /// Clock, environment and locale for the standard library.
    #[must_use]
    pub fn std_host(mut self, h: impl vba_stdlib::Host + 'static) -> Self {
        self.std_host = Box::new(h);
        self
    }

    /// Where `Debug.Print` writes.
    #[must_use]
    pub fn debug_sink(mut self, s: impl DebugSink + 'static) -> Self {
        self.debug = Box::new(s);
        self
    }

    /// The launcher behind `Shell`.
    #[must_use]
    pub fn shell(mut self, s: impl ShellHook + 'static) -> Self {
        self.shell = Box::new(s);
        self
    }

    /// The text `Command` returns.
    #[must_use]
    pub fn command_line(mut self, c: &str) -> Self {
        self.command = c.to_owned();
        self
    }

    /// Interpreter settings.
    #[must_use]
    pub fn vm_config(mut self, c: VmConfig) -> Self {
        self.config = c;
        self
    }

    /// Loads a project.
    pub fn build_project(mut self, project: &VbaProject) -> Result<Session, Error> {
        self.config.project_name = project.name.clone();
        let modules: Vec<SourceModule> = project
            .modules
            .iter()
            .map(|m| SourceModule {
                name: m.name.clone(),
                kind: m.kind,
                source: m.source.clone().into_bytes(),
            })
            .collect();
        let options = Options {
            project_name: project.name.clone(),
            constants: vba_sema::parse_project_constants(&project.constants),
            ..Options::default()
        };
        self.build(&modules, &options)
    }

    /// Loads source modules.
    pub fn build_sources(self, modules: &[SourceModule]) -> Result<Session, Error> {
        self.build(modules, &Options::default())
    }

    fn build(self, modules: &[SourceModule], options: &Options) -> Result<Session, Error> {
        let profile = self.host.metadata().profile;
        let analysis = vba_sema::analyze_sources(modules, &profile, &*self.provider, options);
        let render = |sev: Severity| -> Vec<String> {
            analysis
                .diagnostics
                .iter()
                .filter(|d| d.severity == sev)
                .map(|d| render_plain(d, &analysis.sources))
                .collect()
        };
        let errors = render(Severity::Error);
        if !errors.is_empty() {
            return Err(Error::Load(LoadError {
                diagnostics: errors,
            }));
        }
        let warnings = render(Severity::Warning);
        let ir = vba_ir::lower(&analysis).map_err(|e| {
            Error::Load(LoadError {
                diagnostics: vec![e.to_string()],
            })
        })?;
        vba_ir::verify(&ir).map_err(|e| Error::Internal(format!("IR verification: {e:?}")))?;
        let ir = Rc::new(ir);
        let fs = Rc::new(RefCell::new(FileSystem::new(
            self.host.clone(),
            self.vfs,
            self.shell,
        )));
        fs.borrow_mut().set_command(&self.command);
        let vm = Vm::new(
            ir.clone(),
            self.host.clone(),
            Box::new(FsServices(fs.clone())),
            self.std_host,
            self.debug,
            self.config,
        )?;
        Ok(Session {
            vm,
            fs,
            ir,
            host: self.host,
            warnings,
            trusted: false,
        })
    }
}

/// A loaded project: its own module variables, objects, `Err`, random
/// generator and open files. Creating a session runs no user code; the
/// embedder must [`trust`](Session::trust) it before it runs anything.
pub struct Session {
    vm: Vm,
    fs: Rc<RefCell<FileSystem>>,
    ir: Rc<IrProgram>,
    host: Rc<dyn Host>,
    warnings: Vec<String>,
    trusted: bool,
}

impl Session {
    /// A builder with the deny-by-default configuration.
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }

    /// Allows this session to execute code. Imported code never runs
    /// automatically: without this call every run request is
    /// [`Error::NotTrusted`].
    pub fn trust(&mut self) {
        self.trusted = true;
    }

    /// Whether [`trust`](Session::trust) was called.
    pub fn is_trusted(&self) -> bool {
        self.trusted
    }

    /// Rendered warnings of the analysis.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// The compiled program.
    pub fn program(&self) -> &IrProgram {
        &self.ir
    }

    /// The host the session was built with.
    pub fn host(&self) -> &Rc<dyn Host> {
        &self.host
    }

    /// The file system state (open files, working directory, the `Vfs`).
    pub fn file_system(&self) -> &Rc<RefCell<FileSystem>> {
        &self.fs
    }

    /// Direct access to the interpreter, for embedders that need it.
    pub fn vm(&mut self) -> &mut Vm {
        &mut self.vm
    }

    /// Every procedure of the project.
    pub fn procedures(&self) -> Vec<ProcedureInfo> {
        let p = &self.ir.program;
        p.procs
            .iter()
            .map(|pr| {
                let m = p.module(pr.module);
                ProcedureInfo {
                    module: m.name.to_string(),
                    name: pr.name.to_string(),
                    kind: pr.kind,
                    public: !matches!(pr.visibility, Visibility::Private),
                    params: pr.params.len(),
                    class_like: m.is_class_like(),
                }
            })
            .collect()
    }

    fn require_trust(&self) -> Result<(), Error> {
        if self.trusted {
            Ok(())
        } else {
            Err(Error::NotTrusted)
        }
    }

    fn find(&self, name: &str) -> Result<vba_sema::ProcId, Error> {
        let p = &self.ir.program;
        let callable =
            |id: vba_sema::ProcId| matches!(p.proc(id).kind, ProcKind::Sub | ProcKind::Function);
        let matches: Vec<vba_sema::ProcId> = if let Some((m, n)) = name.split_once('.') {
            let Some(module) = p.module_by_name(m) else {
                return Err(Error::NoSuchProcedure(name.to_owned()));
            };
            module
                .procs
                .iter()
                .copied()
                .filter(|id| callable(*id) && p.proc(*id).name.eq_ignore_case(n))
                .collect()
        } else {
            p.modules
                .iter()
                .filter(|m| !m.is_class_like())
                .flat_map(|m| m.procs.iter().copied())
                .filter(|id| callable(*id) && p.proc(*id).name.eq_ignore_case(name))
                .collect()
        };
        match matches.as_slice() {
            [] => Err(Error::NoSuchProcedure(name.to_owned())),
            [one] => Ok(*one),
            _ => Err(Error::AmbiguousProcedure(name.to_owned())),
        }
    }

    /// Runs a `Sub` or `Function` by name, case-insensitively, like
    /// `Application.Run`: `Proc` (the standard modules) or `Module.Proc`.
    /// Only public procedures run; a procedure of a document or designer
    /// module runs on the module's default instance, a class module's does
    /// not run by name.
    pub fn run_procedure(&mut self, name: &str, args: Vec<Variant>) -> Result<RunOutput, Error> {
        self.require_trust()?;
        let id = self.find(name)?;
        let pr = self.ir.program.proc(id);
        if matches!(pr.visibility, Visibility::Private) {
            return Err(Error::NotCallable(name.to_owned(), "it is private"));
        }
        let module = self.ir.program.module(pr.module);
        let args: Vec<Arg> = args.into_iter().map(Arg::Value).collect();
        if module.is_class_like() {
            if !module.predeclared {
                return Err(Error::NotCallable(
                    name.to_owned(),
                    "it belongs to a class module without a default instance",
                ));
            }
            let obj = self.vm.default_instance_object(pr.module)?;
            return Ok(self.vm.run_method(obj, id, args)?);
        }
        Ok(self.vm.run_procedure(id, args)?)
    }

    /// Delivers a host event, for example `Workbook` / `Open`, to the
    /// handler `Workbook_Open` of a document or designer module (on its
    /// default instance). `Ok(None)` means nothing handles the event.
    pub fn raise_event(
        &mut self,
        source: &str,
        event: &str,
        args: Vec<Variant>,
    ) -> Result<Option<RunOutput>, Error> {
        self.require_trust()?;
        let wanted = format!("{source}_{event}");
        let p = &self.ir.program;
        let found = p
            .modules
            .iter()
            .filter(|m| m.is_class_like() && m.predeclared)
            .find_map(|m| {
                m.procs
                    .iter()
                    .copied()
                    .find(|id| p.proc(*id).name.eq_ignore_case(&wanted))
                    .map(|id| (m.id, id))
            });
        let Some((module, id)) = found else {
            return Ok(None);
        };
        let obj: ObjectRef = self.vm.default_instance_object(module)?;
        let args: Vec<Arg> = args.into_iter().map(Arg::Value).collect();
        Ok(Some(self.vm.run_method(obj, id, args)?))
    }

    /// Closes all open files (writing them back) and releases module-level
    /// objects, running `Class_Terminate` where it applies.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        self.require_trust()?;
        let r = self.vm.shutdown();
        let closed = self.fs.borrow_mut().close_all();
        r.map_err(Error::from)?;
        closed.map_err(|e| match e {
            vba_vm::ServiceError::Vba(i) => Error::Host(Box::new(crate::error::ErrorReport {
                number: i.number,
                description: i.description,
                source: i.source,
                procedure: String::new(),
                line: 0,
                stack: Vec::new(),
            })),
            vba_vm::ServiceError::Unsupported(s) => Error::Unsupported(s),
            vba_vm::ServiceError::Limit(l) => Error::Limit(l),
        })
    }
}

impl From<&VmError> for Error {
    fn from(e: &VmError) -> Error {
        e.clone().into()
    }
}

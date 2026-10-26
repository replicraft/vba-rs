//! [`Session`]: one loaded project with its own VM state.

use std::cell::RefCell;
use std::rc::Rc;

use vba_ast::{ProcKind, Visibility};
use vba_automation::Dispatch;
use vba_diagnostics::{Severity, render_plain};
use vba_host::{HeadlessHost, Host};
use vba_ir::IrProgram;
use vba_project::VbaProject;
use vba_scripting::ScriptingHost;
use vba_sema::{NoLibraries, Options, SourceModule, TypeProvider};
use vba_value::{ObjectRef, Variant};
use vba_vm::{Arg, DebugSink, NullSink, RunOutput, Vm, VmConfig, VmError};

use crate::error::{Error, ErrorReport, LoadError};
use crate::files::{FileSystem, FsServices, ScriptFsAdapter};
use crate::memfs::MemFs;
use crate::ops::{Ops, Target};
use crate::shell::{NoShell, ShellHook};
use crate::vfs::Vfs;

/// A procedure that can be run by name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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

/// Two owned providers consulted in order.
struct OwnedChain(Box<dyn TypeProvider>, Box<dyn TypeProvider>);

impl TypeProvider for OwnedChain {
    fn resolve_type(&self, q: Option<&str>, n: &str) -> Option<vba_sema::LibraryType> {
        self.0
            .resolve_type(q, n)
            .or_else(|| self.1.resolve_type(q, n))
    }
    fn has_library(&self, n: &str) -> bool {
        self.0.has_library(n) || self.1.has_library(n)
    }
    fn resolve_declared(&self, q: Option<&str>, n: &str) -> Option<vba_sema::Type> {
        self.0
            .resolve_declared(q, n)
            .or_else(|| self.1.resolve_declared(q, n))
    }
    fn enum_members(&self, t: &vba_sema::LibraryType) -> Option<Vec<(String, i64)>> {
        self.0.enum_members(t).or_else(|| self.1.enum_members(t))
    }
    fn record(&self, t: &vba_sema::LibraryType) -> Option<vba_sema::LibRecord> {
        self.0.record(t).or_else(|| self.1.record(t))
    }
    fn class_info(&self, t: &vba_sema::LibraryType) -> Option<vba_sema::LibClass> {
        self.0.class_info(t).or_else(|| self.1.class_info(t))
    }
    fn member(&self, t: &vba_sema::LibraryType, n: &str) -> Option<vba_sema::LibMember> {
        self.0.member(t, n).or_else(|| self.1.member(t, n))
    }
    fn default_member(&self, t: &vba_sema::LibraryType) -> Option<vba_sema::LibMember> {
        self.0
            .default_member(t)
            .or_else(|| self.1.default_member(t))
    }
    fn describes_members(&self, t: &vba_sema::LibraryType) -> bool {
        self.0.describes_members(t) || self.1.describes_members(t)
    }
    fn interface_members(&self, t: &vba_sema::LibraryType) -> Option<Vec<vba_sema::LibMember>> {
        self.0
            .interface_members(t)
            .or_else(|| self.1.interface_members(t))
    }
    fn events(&self, t: &vba_sema::LibraryType) -> Option<Vec<vba_sema::LibEvent>> {
        self.0.events(t).or_else(|| self.1.events(t))
    }
    fn global(&self, n: &str) -> Option<vba_sema::LibGlobal> {
        self.0.global(n).or_else(|| self.1.global(n))
    }
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
    allow_partial: bool,
    host_members: Vec<String>,
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
            allow_partial: false,
            host_members: Vec::new(),
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

    /// Adds a type provider that is consulted **before** the one already
    /// configured (for example the library descriptions of an embedding
    /// application, next to the global names a facade engine registered).
    /// [`provider`](SessionBuilder::provider) replaces everything instead.
    #[must_use]
    pub fn extra_provider(mut self, p: impl TypeProvider + 'static) -> Self {
        let old = std::mem::replace(&mut self.provider, Box::new(NoLibraries));
        self.provider = Box::new(OwnedChain(Box::new(p), old));
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

    /// Names the embedding application provides as members of document and
    /// designer modules (control names, base-class properties). Inside such
    /// a module an unqualified use of one of them, read or assigned, is
    /// bound late to the host base object of the instance, even under
    /// `Option Explicit`. Other undeclared names keep the normal rules, so
    /// typos are still reported. See [`vba_sema::Options::host_members`].
    #[must_use]
    pub fn host_members<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.host_members = names.into_iter().map(Into::into).collect();
        self
    }

    /// Lets [`build_project`](SessionBuilder::build_project) accept a
    /// *mixed* project (some modules have no source). The modules without
    /// source are left out of the program, and running a procedure that
    /// cannot be found among the others (or naming such a module) is
    /// [`Error::CompiledOnly`]. Code of the present modules that refers to a
    /// missing module still fails to compile. A fully compiled-only project
    /// is refused regardless.
    #[must_use]
    pub fn allow_partial(mut self) -> Self {
        self.allow_partial = true;
        self
    }

    /// Loads a project.
    ///
    /// A project whose modules lack source text (compiled-only, stripped) is
    /// refused with [`Error::CompiledOnly`] naming the modules, before any
    /// analysis. A mixed project is accepted only after
    /// [`allow_partial`](SessionBuilder::allow_partial).
    pub fn build_project(mut self, project: &VbaProject) -> Result<Session, Error> {
        use vba_project::ProjectSourceAvailability as A;
        let absent: Vec<String> = project
            .modules_without_source()
            .map(|m| m.name.clone())
            .collect();
        let refuse = match project.source_availability() {
            A::FullSource => false,
            A::Mixed => !self.allow_partial,
            A::CompiledOnly => true,
        };
        if refuse {
            return Err(Error::CompiledOnly {
                project: project.name.clone(),
                modules: absent,
            });
        }
        self.config.project_name = project.name.clone();
        let modules: Vec<SourceModule> = project
            .modules
            .iter()
            .filter(|m| !m.source_availability().lacks_source())
            .map(|m| SourceModule {
                name: m.name.clone(),
                kind: m.kind,
                source: m.source.clone().into_bytes(),
            })
            .collect();
        let options = Options {
            project_name: project.name.clone(),
            constants: vba_sema::parse_project_constants(&project.constants),
            host_members: self.host_members.clone(),
            ..Options::default()
        };
        let (project_name, absent_modules) = (project.name.clone(), absent);
        let mut session = self.build(&modules, &options)?;
        session.absent = Some((project_name, absent_modules));
        Ok(session)
    }

    /// Loads source modules.
    pub fn build_sources(self, modules: &[SourceModule]) -> Result<Session, Error> {
        let options = Options {
            host_members: self.host_members.clone(),
            ..Options::default()
        };
        self.build(modules, &options)
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
        // `CreateObject("Scripting.Dictionary" / "Scripting.FileSystemObject")`
        // answers with the native classes; their file access is this
        // session's policy-checked file system.
        let scripting = ScriptingHost::new(self.host.clone());
        scripting.set_fs(Rc::new(ScriptFsAdapter(Rc::downgrade(&fs))));
        let host: Rc<dyn Host> = scripting;
        let source_lines = statement_lines(&ir, &analysis.sources);
        let mut vm = Vm::new(
            ir.clone(),
            host.clone(),
            Box::new(FsServices(fs.clone())),
            self.std_host,
            self.debug,
            self.config,
        )?;
        vm.set_source_lines(source_lines);
        Ok(Session {
            vm,
            fs,
            ir,
            host,
            warnings,
            trusted: false,
            absent: None,
        })
    }
}

/// The 1-based source line of every statement of every function, for error
/// locations.
fn statement_lines(ir: &IrProgram, sources: &vba_source::SourceMap) -> Vec<Vec<u32>> {
    ir.functions
        .iter()
        .map(|f| {
            let module = match f.kind {
                vba_ir::FuncKind::Proc(p) => ir.program.proc(p).module,
                vba_ir::FuncKind::ModuleInit(m) => m,
            };
            let file = sources.get(ir.program.module(module).file);
            f.stmts
                .iter()
                .map(|s| file.map_or(0, |file| file.line_col(s.span.start).line + 1))
                .collect()
        })
        .collect()
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
    /// Project name and modules left out because they have no source.
    absent: Option<(String, Vec<String>)>,
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

    pub(crate) fn vm_ref(&self) -> &Vm {
        &self.vm
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

    pub(crate) fn require_trust(&self) -> Result<(), Error> {
        if self.trusted {
            Ok(())
        } else {
            Err(Error::NotTrusted)
        }
    }

    fn ops(&mut self) -> Ops<'_> {
        Ops::with_absent(&mut self.vm, self.absent.as_ref())
    }

    /// Runs a `Sub` or `Function` by name, case-insensitively, like
    /// `Application.Run`: `Proc` (the standard modules) or `Module.Proc`.
    /// Only public procedures run; a procedure of a document or designer
    /// module runs on the module's default instance, a class module's does
    /// not run by name.
    pub fn run_procedure(&mut self, name: &str, args: Vec<Variant>) -> Result<RunOutput, Error> {
        let args = args.into_iter().map(Arg::Value).collect();
        self.call(&Target::Project, name, args, Vec::new())
    }

    /// Calls a procedure with full control over the arguments: values,
    /// `ByRef` variables whose final values come back in
    /// [`RunOutput::byref`], omitted (`Missing`) and named arguments.
    ///
    /// With [`Target::Project`] the name is resolved like
    /// [`run_procedure`](Session::run_procedure). With [`Target::Object`]
    /// the procedure is looked up in the object's own class-like module
    /// (`Sub`, `Function` or `Property Get`, `Public` only).
    pub fn call(
        &mut self,
        target: &Target,
        name: &str,
        positional: Vec<Arg>,
        named: Vec<(String, Arg)>,
    ) -> Result<RunOutput, Error> {
        self.require_trust()?;
        self.ops().call(target, name, positional, named)
    }

    /// Creates a **new** instance of a class, document or designer module,
    /// with its own module-level state; `Class_Initialize` runs before this
    /// returns. `host_base` is bound as the instance's host base: members
    /// the module does not define (`Me.Caption`, control names, `Me!Field`,
    /// the default member) dispatch to it, late bound.
    ///
    /// The caller owns one reference; give it back with
    /// [`destroy_object`](Session::destroy_object).
    pub fn create_object(
        &mut self,
        module: &str,
        host_base: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        self.require_trust()?;
        self.ops().create_object(module, host_base)
    }

    /// The default (predeclared) instance of a document, designer or
    /// predeclared class module, created on first use. The reference is
    /// borrowed: it lives as long as the session and cannot be destroyed
    /// with [`destroy_object`](Session::destroy_object). A `host_base` is
    /// bound when the instance has none yet (before `Class_Initialize` if
    /// this call creates the instance).
    pub fn default_instance(
        &mut self,
        module: &str,
        host_base: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        self.require_trust()?;
        self.ops().default_instance(module, host_base)
    }

    /// Gives back the reference [`create_object`](Session::create_object)
    /// returned; `Class_Terminate` runs now unless VBA code still holds the
    /// instance. A default instance cannot be destroyed.
    pub fn destroy_object(&mut self, object: &ObjectRef) -> Result<(), Error> {
        self.require_trust()?;
        self.ops().destroy_object(object)
    }

    /// Whether `object` has a `Sub`, `Function` or `Property Get` called
    /// `name`, of any visibility (so a host can discover private event
    /// handlers such as `Form_Current`).
    pub fn has_procedure(&self, object: &ObjectRef, name: &str) -> bool {
        crate::ops::resolve_in_object(&self.ir, &self.vm, object, name, false).is_ok()
    }

    /// Runs the handler `<source>_<event>` (for example `Form` /
    /// `BeforeUpdate`, `Command1` / `Click`) on that specific instance,
    /// whatever its visibility. `Ok(None)` means the instance has no such
    /// handler.
    pub fn raise_event_on(
        &mut self,
        object: &ObjectRef,
        source: &str,
        event: &str,
        positional: Vec<Arg>,
        named: Vec<(String, Arg)>,
    ) -> Result<Option<RunOutput>, Error> {
        self.require_trust()?;
        self.ops()
            .raise_event_on(object, source, event, positional, named)
    }

    /// Reads a public variable or `Property Get` without parameters of an
    /// instance by name; names the module lacks go to its host base.
    pub fn get_member(&mut self, object: &ObjectRef, name: &str) -> Result<Variant, Error> {
        self.require_trust()?;
        self.ops().get_member(object, name)
    }

    /// Assigns a public variable or property of an instance by name.
    pub fn set_member(
        &mut self,
        object: &ObjectRef,
        name: &str,
        value: Variant,
    ) -> Result<(), Error> {
        self.require_trust()?;
        self.ops().set_member(object, name, value)
    }

    /// Reads a public module-level variable or constant of a standard
    /// module: `Module.Name`, or `Name` when exactly one standard module
    /// declares it. An object value carries a reference the caller owns
    /// (see [`Session::release`]).
    ///
    /// Errors: unknown name ([`Error::NoSuchMember`]), a name declared in
    /// several modules ([`Error::AmbiguousMember`]), a private variable, a
    /// user-defined type or a variable of a class-like module
    /// ([`Error::NotAccessible`]).
    pub fn get_global(&mut self, name: &str) -> Result<Variant, Error> {
        self.require_trust()?;
        self.ops().get_global(name)
    }

    /// Assigns a public module-level variable of a standard module. The
    /// value is converted to the declared type exactly as an assignment in
    /// VBA would (`Long` to an `Integer` variable, error 6 on overflow,
    /// error 13 when it cannot be converted), and an object is assigned
    /// with `Set` semantics. A constant, a fixed-size array and a
    /// user-defined type cannot be assigned ([`Error::NotAccessible`]).
    pub fn set_global(&mut self, name: &str, value: Variant) -> Result<(), Error> {
        self.require_trust()?;
        self.ops().set_global(name, value)
    }

    /// The error the `Err` object holds now, with where it was raised and
    /// the call stack at that point, or `None` when `Err.Number` is 0. After
    /// a call that returned normally this is an error the program handled
    /// (`On Error Resume Next`, `On Error GoTo`); `Err` keeps it until the
    /// program clears it or another error replaces it, exactly as in VBA.
    pub fn last_error(&self) -> Option<ErrorReport> {
        self.vm_ref().current_error().map(ErrorReport::from)
    }

    /// Delivers the events host objects raised while the session was not
    /// waiting in a call of the raising object, in the order they were
    /// raised. (Events raised during such a call are delivered at once.)
    pub fn deliver_pending_events(&mut self) -> Result<(), Error> {
        self.require_trust()?;
        Ok(self.vm.deliver_pending_events()?)
    }

    /// Releases the object references inside a value a call returned (the
    /// result or a `ByRef` value).
    pub fn release(&mut self, value: &Variant) {
        self.vm.release(value);
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
        let closed = self.close_files();
        r.map_err(Error::from)?;
        closed
    }

    /// Discards all state of the program and starts over: module variables
    /// get their initial values again, every object is dropped **without**
    /// `Class_Terminate`, open files are closed, `Err` and the budget
    /// counters are cleared. The trust decision is kept. See
    /// [`Vm::reset`](vba_vm::Vm::reset).
    pub fn reset(&mut self) -> Result<(), Error> {
        let r = self.vm.reset();
        let closed = self.close_files();
        r.map_err(Error::from)?;
        closed
    }

    fn close_files(&mut self) -> Result<(), Error> {
        let closed = self.fs.borrow_mut().close_all();
        closed.map_err(|e| match e {
            vba_vm::ServiceError::Vba(i) => Error::Host(Box::new(
                crate::error::ErrorReport::plain(i.number, i.description, i.source),
            )),
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

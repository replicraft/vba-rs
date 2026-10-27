# Embedding guide

How an application embeds vba-rs end to end: load a project, analyse it, trust
it, create form-like instances whose `Me` is a host object, expose host objects
(`Application`, `DoCmd`), raise events with `ByRef` results (`Cancel`), read
variables, deal with errors, limits and cancellation, and call back into the
running session from host code.

Every Rust block below is compiled and run as a doctest of the `vba-rs` crate
(`cargo test -p vba-rs --doc`). The larger flow is the runnable example
`crates/api/vba-rs/examples/form_events.rs`. The facade items are listed in
[public-api.md](public-api.md); the contract a host implements is in
[host.md](host.md).

Statements about behavior that were not compared with a real VBA host are marked
**(unverified)**.

## The shape of an embedding

```text
 your application                          vba-rs
 ----------------                          ------
 project bytes / streams / source ──────►  Project            (read, edit, preserve)
                                           Engine::analyze    (diagnostics, nothing runs)
 Engine::new().grant(..).limits(..)        Engine::load       (compiled, UNTRUSTED)
 host objects (Dispatch) ───────────────►  Instance::trust    (your decision)
 form Me (Dispatch) ────────────────────►  create_object(module, Some(me))
                                           raise_event_on / call_on / get_global ...
 read Cancel / results / errors ◄───────   CallResult, EventOutcome, Error (+ stack)
 your transactions, your storage           (the VM never touches them)
```

One `Instance` is one loaded project with its own module variables, objects,
`Err`, random generator, open files and budget. Create it, use it and drop it on
one thread.

## Depending on the crate

```toml
[dependencies]
# The facade crate. Pin a revision; it is not published to a registry.
vba-rs = { git = "https://example.com/your-org/vba-rs.git", rev = "<commit>" }
# With optional features (all off by default, none grants authority):
# vba-rs = { git = "...", rev = "...", features = ["winapi"] }
```

The library is imported as `vba`. MSRV is the workspace `rust-version` (1.99,
edition 2024). Features: `winapi` (the Win32 behavior catalog, `vba::raw::winapi`),
`native-call` (the native stage of `Declare`, the only `unsafe` in the workspace,
still behind a capability and a host-issued permission) and `wine` (implies
`winapi`). The default build is the pure interpreter.

**Stability.** Items exported by the facade (`vba::*`, `vba::automation`,
`vba::runtime`, `vba::prelude`) follow semver for the `0.x` series: a breaking
change bumps the minor version and is listed in `CHANGELOG.md` under "Breaking
changes" (the list of everything so far is there). Result and
error structs and enums are `#[non_exhaustive]`: match enums with a wildcard arm
and build structs with the constructors, not literals. `vba::raw::*` re-exports
the lower crates and carries the promises of those crates only. Behavior marked
**(unverified)** may change when it is compared with a real host.

## 1. Load a project

A project comes from source text, from a `vbaProject.bin`, or from a set of
streams your application extracted from its own storage.

```rust
use vba::{ModuleKind, Project, ProjectStreams, ReadLimits, WriteOptions};

// From source: one standard module.
let from_source = Project::from_source("Module1", "Public Sub Main()\nEnd Sub\n");
assert_eq!(from_source.model().modules.len(), 1);

// Build a project, with a form (document) module.
let mut project = Project::new();
project.add_module("Module1", ModuleKind::Standard, "Public Sub Main()\r\nEnd Sub\r\n").unwrap();
project.add_module("Form_Orders", ModuleKind::Document,
    "Private Sub Form_Current()\r\nEnd Sub\r\n").unwrap();

// From a vbaProject.bin (a file you read, or bytes from your store).
let bytes = project.to_bytes().unwrap();
let from_bin = Project::from_bytes(&bytes).unwrap();
assert_eq!(from_bin.module_source("Module1"), project.module_source("Module1"));

// From streams: the `VBA/dir`, `VBA/_VBA_PROJECT`, `VBA/<module>`, `PROJECT`
// streams your application keeps, for example in a table. No container is
// involved.
let (streams, _report) = from_bin.to_streams(&WriteOptions::default()).unwrap();
let from_streams = Project::from_streams(&streams, &ReadLimits::default()).unwrap();
assert_eq!(from_streams.module_source("Form_Orders"), project.module_source("Form_Orders"));
# let _ = ProjectStreams::new();
```

Reading never executes anything and keeps every byte the model does not
interpret, so an unedited project writes back byte-identical. Source-less
(compiled-only) modules are refused when loading into an engine; see
[compiled-projects.md](../runtime/compiled-projects.md).

## 2. Analyse, load, trust

`Engine::analyze` reports diagnostics without running anything. `Engine::load`
compiles into an **untrusted** `Instance`: no user code has run, not even
`Class_Initialize`. The embedder's explicit `trust()` is what lets code run.

```rust
use vba::{Engine, Project};

let project = Project::from_source(
    "Module1",
    "Public Function Twice(ByVal n As Long) As Long\n  Twice = n * 2\nEnd Function\n",
);
let engine = Engine::new(); // deny-all policy, fixed clock, empty environment, in-memory files
let report = engine.analyze(&project);
assert!(!report.has_errors(), "{:?}", report.rendered());

let mut inst = engine.load(&project).unwrap();
assert!(inst.run("Twice", vec![2.into()]).is_err()); // NotTrusted
inst.trust();
assert_eq!(inst.run("Twice", vec![21.into()]).unwrap().value, vba::Variant::Long(42));
```

The engine decides what the program can reach: `grant(Capability::..)`,
`grant_prefixes`, `limits`/`max_steps`/`call_timeout`/`unlimited()` (finite by default),
`vm_config`, `real_files()`, `system_environment()`, `fixed_clock(date)` (also the year of date
literals such as `#3/4#`), `strict_spec(true)`. Nothing is ambient. `analyze` uses the same engine
configuration (registered globals, host members) as `load`.

## 3. Host objects: `Application`-like globals and `DoCmd`-like objects

A host object is any `vba::automation::Dispatch` implementation. Register it as
a global name (`Application`, `DoCmd`) or under a ProgID for `CreateObject`.
`MemObject` is a ready-made object for tests and small hosts; real hosts
implement `Dispatch` themselves (see section 6).

The VBA side calls members with positional, named and omitted arguments. The
host declares its parameters (`ParamInfo`), and the call machinery binds them,
applies defaults and writes `ByRef` results back to the caller's variables.

```rust
use std::cell::RefCell;
use std::rc::Rc;
use vba::automation::{testing::MemObject, ParamInfo};
use vba::{Engine, Project, Variant};

let calls = Rc::new(RefCell::new(Vec::<String>::new()));
let log = calls.clone();
let engine = Engine::new().dispatch_global("DoCmd", move || {
    let log = log.clone();
    MemObject::new("DoCmd").method(
        "OpenForm",
        vec![
            ParamInfo::required("FormName").by_val(),
            ParamInfo::optional("View").by_val().with_default(Variant::Long(0)),
            ParamInfo::optional("OpenArgs").by_val(),
        ],
        move |_table, call| {
            let name = String::try_from(&call.args[0].value).unwrap();
            let view = i32::try_from(&call.args[1].value).unwrap();
            let args = if call.args[2].is_missing() { "-".to_owned() } else { String::try_from(&call.args[2].value).unwrap() };
            log.borrow_mut().push(format!("{name} view={view} args={args}"));
            Ok(Variant::Empty)
        },
    )
}).dispatch_global("Application", || {
    // `ByRef` works both ways: the host writes the argument back.
    MemObject::new("Application").method("Next", vec![ParamInfo::required("n")], |_t, call| {
        let n = i32::try_from(&call.args[0].value).unwrap();
        call.args[0].value = Variant::Long(n + 1);
        Ok(Variant::Empty)
    })
});

let project = Project::from_source(
    "Module1",
    "Public Function Go() As Long\n  Dim n As Long\n  n = 41\n  DoCmd.OpenForm \"Orders\"\n  DoCmd.OpenForm \"Details\", OpenArgs:=\"A1\"\n  DoCmd.OpenForm \"Report\", 2, \"x\"\n  Application.Next n\n  Go = n\nEnd Function\n",
);
let mut inst = engine.load(&project).unwrap();
inst.trust();
assert_eq!(inst.run("Go", vec![]).unwrap().value, Variant::Long(42));
assert_eq!(*calls.borrow(), ["Orders view=0 args=-", "Details view=0 args=A1", "Report view=2 args=x"]);
```

Registered global names are made known to the analysis, so `Option Explicit`
code that mentions `DoCmd` compiles. `CreateObject` additionally needs
`Capability::ComCreate`.

## 4. Form-like instances with a host `Me`

A form is a document or designer module. `create_object` makes a **new**
instance (own module variables, own `Class_Initialize`); the optional host
object becomes the instance's *host base*: everything the module does not define
itself dispatches to it, late bound.

* `Me.Caption`, `Me.Controls("Name")`, `Me!Name`, `Me("Name")` always reach the
  host base (an info diagnostic says they are late bound).
* An *unqualified* name (`CustomerName`, `Dirty`) is a host member only if the
  module does not declare it and the engine was told about it with
  `Engine::host_members([..])`. Other undeclared names keep the normal rules, so
  typos under `Option Explicit` are still errors. Without `Option Explicit` an
  unlisted name is an implicit local variable.
* A name the module defines (a `Public` variable, a procedure) wins.
* With no host base, such an access is VBA error 438.

Event handlers are ordinary procedures named `<Source>_<Event>` (`Form_Current`,
`Form_BeforeUpdate`, `Command1_Click`), usually `Private`. `raise_event_on` runs
the handler on **that instance** whatever its visibility and returns the final
values of `ByRef` arguments. Without a handler nothing runs and `handled` is
false.

```rust
use std::rc::Rc;
use vba::automation::testing::MemObject;
use vba::{Arg, Engine, ModuleKind, Project, Variant};

let mut project = Project::new();
project.add_module("Form_Orders", ModuleKind::Document, "\
Option Explicit
Private Sub Form_BeforeUpdate(Cancel As Integer)
    If IsNull(Me.CustomerName) Then Cancel = True
    Debug.Print \"BeforeUpdate cancel=\" & Cancel
End Sub
").unwrap();

let mut inst = Engine::new().load(&project).unwrap();
inst.trust();

// The form's Me: a host object with a CustomerName property.
let me = Rc::new(MemObject::new("Form").property("CustomerName", Variant::Null));
let form = inst.create_object("Form_Orders", Some(me)).unwrap();
assert!(inst.has_procedure(form, "Form_BeforeUpdate"));

// The host raises BeforeUpdate with a ByRef Cancel and reads it back.
let out = inst
    .raise_event_on(form, "Form", "BeforeUpdate", vec![Arg::by_ref(Variant::Integer(0))])
    .unwrap();
assert!(out.handled);
assert_eq!(out.byref, vec![Variant::Integer(-1)]); // the handler set Cancel = True
assert_eq!(out.output, "BeforeUpdate cancel=-1\n");

// A second instance has its own state; destroy runs Class_Terminate.
let other = inst.create_object("Form_Orders", None).unwrap();
assert_ne!(form, other);
inst.destroy_object(other).unwrap();
inst.destroy_object(form).unwrap();
```

`ByRef` rules of `Instance::call`, `call_on` and `raise_event_on`: an argument
passed with `Arg::by_ref(value)` is a caller-owned variable. Its value is first
converted to the declared type of the parameter (a `Boolean` for an `Integer`
parameter becomes an `Integer`; text that is not a number is error 13); a
`ByRef` parameter writes through and the final value, converted to the declared
type, is returned in `byref` in argument order. `ByVal` parameters and
`ParamArray` elements get a copy and are reported unchanged. `Arg::missing()`
leaves an `Optional` parameter at its default, `Arg::named("Name", ..)` binds by
parameter name. A wrong argument count is error 450, a missing required argument
449, an unknown name 448, a name given twice 446.

**Default instances.** `Form_Orders` / `Forms!Orders`-style predeclared
instances remain: `default_instance("Form_Orders", Some(me))` returns the
borrowed reference of the predeclared instance (created on first use), and
`call(CallTarget::Project, "Form_Orders.Proc", ..)` runs on it. A default
instance lives as long as the session and cannot be destroyed.
**(unverified)** `Class_Initialize` and `Class_Terminate` also run for document
and designer modules, because an embedding application's forms are classes; the
behavior of document modules in other hosts was not compared.

**Object lifetime.** The reference `create_object` returns is yours. VBA code
may keep the instance (`Set held = Me`); `destroy_object` gives your reference
back and `Class_Terminate` runs when the last reference goes. Objects and
`ByRef` object values returned to you carry one reference you own;
`Instance::release(&value)` gives it back.

## 5. Variables and project metadata

```rust
use vba::{Engine, Project, Variant};

let project = Project::from_source("Config", "\
Public Const Limit As Long = 10
Public Counter As Integer
Private Sub Form_Current()
End Sub
Public Sub Bump()
  Counter = Counter + 1
End Sub
");
let mut inst = Engine::new().load(&project).unwrap();
inst.trust();
inst.run("Bump", vec![]).unwrap();

// Public variables and constants of standard modules, `Module.Name` or `Name`
// when unambiguous. Assignment converts like VBA (error 6 on overflow).
assert_eq!(inst.get_global("Counter").unwrap(), Variant::Integer(1));
assert_eq!(inst.get_global("Config.Limit").unwrap(), Variant::Long(10));
inst.set_global("Counter", Variant::Long(41)).unwrap();
assert_eq!(inst.get_global("Counter").unwrap(), Variant::Integer(41));

// Discover procedures, including private event handlers.
let members = inst.modules();
let config = members.iter().find(|m| m.name == "Config").unwrap();
assert!(config.procedure("Form_Current").is_some_and(|p| !p.public));
assert_eq!(inst.list_public_members()[0].procedures.len(), 1); // only Bump
assert_eq!(inst.modules_defining("Form_Current").len(), 0);     // class-like modules only
```

`get_global`/`set_global` refuse private variables, constants (for writing),
fixed-size arrays and user-defined types with `NotAccessible`; unknown names are
`NoSuchMember`, bare names declared in several modules `AmbiguousMember`. The
fields of an instance are reached with `get_member`/`set_member`.
`modules_defining("Form_Current")` answers "which forms define this handler".

Values cross the boundary as `vba::Variant`. Constructors and conversions:
`Variant::from(7_i32)` (`Long`), `from("x")`, `from(true)`, `from(Some(x))` /
`from(None::<i32>)` (`Null`), `from(vec![..])` (a zero-based array),
`Variant::empty()/null()/nothing()`, `array_one_based(..)`,
`array_with_lower_bound(..)`, `date_from_ymd_hms(..)`, `date_from_serial(..)`;
back with `i32::try_from(&v)`, `String::try_from(&v)`, `v.to_vec()`... following
the VBA conversion functions (`CLng`, `CStr`...). `Variant` implements
`Display` for logs. Dates are OLE serials (`Date::serial`), with
`Date::from_unix_seconds`/`unix_seconds` and calendar `parts()`; there is no
time-zone handling.

## 6. Re-entrancy: a host member that calls back

Most host members only read and write their own state. Some must run VBA while
the VBA statement that called them is still waiting: `DoCmd.OpenForm` creates a
form instance and runs its `Open`/`Load` handlers before it returns; `Update`
raises `BeforeUpdate` and returns whether `Cancel` was set. The interpreter
supports this through `HostContext`.

```rust
use std::cell::Cell;
use std::rc::Rc;
use vba::automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectRef, ObjectTable,
};
use vba::{Arg, Engine, HostContext, ModuleKind, Project, Variant};

/// `Me` of a form with an `Update()` that raises BeforeUpdate synchronously.
struct FormHost {
    vba_form: Cell<Option<ObjectRef>>,
}

impl Dispatch for FormHost {
    fn class_name(&self) -> String { "Form".into() }
    // 1. Declare that the member may call back: the interpreter then waits
    //    in a way that lets the context run VBA.
    fn reentrant(&self) -> bool { true }
    fn member_id(&self, name: &str) -> Option<DispId> {
        name.eq_ignore_ascii_case("Update").then_some(DispId(1))
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        (id == DispId(1)).then(|| {
            MemberInfo::new("Update", id, InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET, vec![])
        })
    }
    fn invoke(&self, table: &ObjectTable, _id: DispId, _f: InvokeFlags, _p: &mut DispParams)
        -> Result<Variant, DispError>
    {
        // 2. The context of the session that is calling us right now.
        let ctx = HostContext::current(table)
            .ok_or_else(|| DispError::raise(5, "not called from a session", "Form"))?;
        let form = self.vba_form.get().expect("bound after create_object");
        // 3. Runs to completion, nested; ByRef results come back.
        let out = ctx
            .raise_event_on(form, "Form", "BeforeUpdate", vec![Arg::by_ref(Variant::Integer(0))])
            // 4. A failure maps to a trappable VBA error for the outer code.
            .map_err(DispError::from)?;
        Ok(Variant::Boolean(out.byref[0] == Variant::Integer(0)))
    }
}

let mut project = Project::new();
project.add_module("Form_Orders", ModuleKind::Document, "\
Private Sub Form_BeforeUpdate(Cancel As Integer)
    Debug.Print \"checking\"
    Cancel = True
End Sub
Public Sub Save()
    If Me.Update() Then Debug.Print \"saved\" Else Debug.Print \"refused\"
End Sub
").unwrap();
let mut inst = Engine::new().load(&project).unwrap();
inst.trust();

let host = Rc::new(FormHost { vba_form: Cell::new(None) });
let form = inst.create_object("Form_Orders", Some(host.clone())).unwrap();
host.vba_form.set(Some(form));

let r = inst.call_on(form, "Save", vec![]).unwrap();
// BeforeUpdate ran inside the `Me.Update()` statement, before "refused".
assert_eq!(r.output, "checking\nrefused\n");
```

### The rules

* **Opt in.** Only members of objects whose `Dispatch::reentrant()` is `true`
  suspend the interpreter; for every other object the call costs nothing extra.
  `HostContext::current(table)` is `None` otherwise.
* **Where it is valid.** During `Dispatch::invoke` (and the events that member
  raises). Not during `terminate`, `held_references` or enumerators. Once the
  member returns, the context is stale: its methods fail with
  `RunError::Reentrancy`. It is not `Send`.
* **What it offers.** The same operations as `Instance`: `create_object`,
  `default_instance`, `destroy_object`, `call`, `call_on`, `raise_event_on`,
  `has_procedure`, `get_global`, `set_global`, `get_member`, `set_member`,
  `release`.
* **Nested and synchronous.** Everything runs inline in the order the host calls
  it, on the same stack of module state, `Err` object and `Debug.Print` sink.
  Output printed by nested code appears in the `output` of the outermost
  `Instance` call; a nested `CallResult::output` is empty. Nothing is queued.
* **Bounded.** `VmConfig::max_reentrancy_depth` (default 4) bounds how many
  nested runs may be active; the next fails with `RunError::Reentrancy`. Nested
  runs also count against `VmConfig::max_native_depth` and
  `VmConfig::max_native_stack_bytes`; lower the latter when the session runs on a thread with
  a stack smaller than 2 MiB.
* **One budget.** Steps, memory and call depth are charged to the instance's one
  budget, whatever the nesting.
* **Errors.** An unhandled VBA error in nested code comes back as `Err`. Return
  it from your member with `DispError::from(err)` and the outer VBA code sees an
  ordinary run-time error with the nested number, description and source
  (`On Error` can trap it, `Err.Number` is the nested one); handle it in the
  host and the outer code never knows. A **fatal** fault (resource limit,
  cancellation, an unsupported feature, `End`, `Stop`) also comes back as `Err`
  but additionally ends the outer run once your member returns, whatever you did
  with the error: `On Error` cannot swallow it, and `End` in a handler ends the
  whole program (the outer call reports `RunStatus::Ended`).
* **Events from host objects.** A `WithEvents` handler for an event your object
  raises through its `ConnectionPoint` is delivered **at once, with `ByRef`
  write-back**, when the raising object is `reentrant()` and the event is raised
  while the VBA statement that called it is waiting: the handler runs inside
  `fire`, and the argument list you passed (`Cancel`) holds the handler's final
  values when `fire` returns. For other objects the previous behaviour stays:
  the event is queued and delivered right after the host call returns, without
  write-back. Events raised while the session is not waiting in such a call (your
  own code between calls) are queued; `Instance::deliver_pending_events()` runs
  them, in order. Queued events are always delivered before a newer one, so the
  order is deterministic.
* **No lock-up.** The interpreter is never borrowed twice: a context used outside
  the host call, or deeper than the limit, returns an error instead of racing.

## 7. Errors, stack traces and `Err`

An unhandled error ends the call with `Err(vba::Error::Run(e))`. For the
families that come from running code (`Vba`, `Host`, `PermissionDenied`)
`e.report()` has the details:

* `number`, `description`, `source` (the project name unless the error named
  one), `help_file`, `help_context`, `last_dll_error`;
* `module`, `procedure`, `line` (the `Erl` line number when the procedure uses
  line numbers), `source_line` (the 1-based line of the failing statement in the
  module source as it was given to the engine);
* `frames`: the call stack, innermost first, each with module, procedure, `Erl`
  line and source line. Host code has no frames; a call that went through a host
  object shows the VBA frames only.

`e.error_info()` gives the same as a `VbaErrorInfo`; `{}` prints one line,
`e.detail()` / `{:#}` adds source and stack. `Instance::last_error()` is the
current content of `Err` with the place it was raised: how a host sees an error
the program *handled* (`On Error Resume Next`, `On Error GoTo`). Like VBA's
`Err`, it stays until the program clears it (`Err.Clear`, `Resume Next`, leaving a procedure whose
handler is active) or another error replaces it, so use it for the call that
just returned `Ok`, not as a history.

```rust
use vba::{CallTarget, Engine, Error, Project};

let project = Project::from_source("Module1", "\
Public Sub Outer()
  Inner 5
End Sub
Public Sub Inner(ByVal n As Long)
  Dim x As Long
  x = n \\ 0
End Sub
Public Sub Custom()
  Err.Raise 5000, \"MyApp.Custom\", \"custom failure\"
End Sub
Public Sub Tolerant()
  On Error Resume Next
  Inner 1
  Debug.Print \"went on, Err=\" & Err.Number
End Sub
");
let mut inst = Engine::new().load(&project).unwrap();
inst.trust();

let err = inst.call(CallTarget::Project, "Outer", vec![]).unwrap_err();
let Error::Run(run) = &err else { panic!("{err:?}") };
let report = run.report().unwrap();
assert_eq!(report.number, 11);
let stack: Vec<String> = report.frames.iter().map(|f| format!("{}.{}:{}", f.module, f.procedure, f.source_line)).collect();
assert_eq!(stack, ["Module1.Inner:6", "Module1.Outer:2"]);
assert_eq!(err.to_string(), "Run-time error '11': Division by zero (in Module1.Inner, source line 6)");
assert!(run.detail().contains("\n  at Module1.Outer (source line 2)"));

let err = inst.call(CallTarget::Project, "Custom", vec![]).unwrap_err();
let Error::Run(run) = &err else { panic!() };
let info = run.error_info().unwrap();
assert_eq!((info.number, info.source.as_str()), (5000, "MyApp.Custom"));

// A handled error is visible through Err.
let ok = inst.call(CallTarget::Project, "Tolerant", vec![]).unwrap();
assert_eq!(ok.output, "went on, Err=11\n");
assert_eq!(inst.last_error().unwrap().frames[0].procedure, "Inner");
```

`Error::kind()` returns a stable short name (see the taxonomy in
[public-api.md](public-api.md)). Beyond it, `Error::Poisoned` (below) and the
embedding errors `NoSuchMember`, `AmbiguousMember`, `NotAccessible` and
`Reentrancy` exist.

## 8. Limits, cancellation and the state after a fault

* **Budgets are per instance.** `Engine::limits` / `max_steps` / `vm_config`
  configure each instance the engine loads; every instance has its own meter.
  `Engine::new()` starts from the finite `ResourceLimits::untrusted()` set (100
  million steps, 256 MiB, call depth 1000) and gives every call from the embedding
  application a wall-clock allowance of 60 seconds (`UNTRUSTED_CALL_TIMEOUT`).
  To raise them: `max_steps(n)`, `max_memory(bytes)`, `limits(ResourceLimits::untrusted().with_max_steps(Some(n)))` and
  `call_timeout(Some(Duration::from_secs(300)))`; to remove them for code you trust,
  `Engine::unlimited()` (also `call_timeout(None)` for the clock alone). A
  `CallLimits::timeout` replaces the default allowance inside its scope, longer or shorter.
  An instance built with `Instance::from_session` gets the same 60-second allowance
  (a hand-built host may carry no limits at all); `Instance::default_call_timeout(None)`
  removes it and `Some(d)` changes it. The step count spans
  the whole life of the instance, nested calls included, so a long-lived
  instance that serves many events eventually spends it: give each event its own
  allowance with `reset_step_counter()` (below). `Instance::reset_step_counter()` starts a fresh allowance (give each
  event handler its own), module state is kept.
* **Per-call allowances.** `Instance::with_call_limits(CallLimits::new()
  .max_steps(n).max_memory(bytes).timeout(d), |i| ..)` bounds the calls made
  inside the closure without touching the instance's counters: the step and
  memory allowances count from the start of the scope, the timeout is checked
  every `CANCEL_POLL_INTERVAL` steps (steps charged for work count, so a statement
  that scans or builds a large value polls sooner), inside long library calls and at
  `DoEvents` (it ends the call with `Cancelled`, the token is not set), and the instance stays usable afterwards.
  A scope only narrows what the engine allows, and scopes nest.
  `Instance::cancel_token()` returns the token the instance polls.
* **Cancellation.** `Engine::cancel_token()` is `Send + Sync + Clone`. The
  session polls it before every call into it (including nested runs), at every
  `DoEvents`, and every `CANCEL_POLL_INTERVAL` (1024) interpreter steps, so a
  loop that never yields is stopped too. A cancelled run ends with
  `RunError::Cancelled`, which `On Error` cannot swallow. It cannot interrupt a
  single long interpreter operation (one huge string expression) or a host
  member that blocks: bound the first with limits and make the second return.
  The token stays set until `reset()`; calls fail at once meanwhile.
* **After a fatal fault** (resource limit, cancellation, `End` in a nested run)
  the run stops at once and discards its frames **without running code**: objects
  only those frames held are never terminated, and module state may be half
  updated, as after any aborted procedure. Unhandled VBA errors, in contrast,
  unwind properly (locals released, `Class_Terminate` run).
* **`Instance::is_usable()`** is `false` while the step allowance is spent, the
  cancel token is set, or the instance is *poisoned* (an internal error, a bug in
  vba-rs, never a mistake of the program: calls then fail with `Error::Poisoned`).
  **(untested)** No test provokes an internal error, so poisoning is specified but
  not exercised.
* **`Instance::reset()`** discards all state and makes the instance usable again:
  module variables return to their initial values, every object (including those
  from `create_object`) is dropped without `Class_Terminate`, `Err`, open files,
  budget counters and the poison flag are cleared, and references VBA variables
  held to host objects are released. The output buffer, the trust decision and
  your token are untouched. What the host keeps itself is the host's to clean up.

```rust
use std::time::Duration;
use vba::{CallTarget, Engine, Error, Project};
use vba::runtime::Error as RunError;

let project = Project::from_source("Module1", "\
Public Sub Spin()
  Do
  Loop
End Sub
Public Sub Quick()
End Sub
");
let engine = Engine::new();
let token = engine.cancel_token();
let mut inst = engine.load(&project).unwrap();
inst.trust();

// Another thread cancels while the handler loops without yielding.
let canceller = std::thread::spawn({
    let token = token.clone();
    move || { std::thread::sleep(Duration::from_millis(20)); token.cancel(); }
});
let err = inst.call(CallTarget::Project, "Spin", vec![]).unwrap_err();
canceller.join().unwrap();
assert!(matches!(err, Error::Run(RunError::Cancelled(_))));
assert!(!inst.is_usable());          // the token is still set
token.reset();
assert!(inst.is_usable());
inst.call(CallTarget::Project, "Quick", vec![]).unwrap();

// A step budget bounds a handler that cannot be cancelled from outside.
let mut limited = Engine::new().max_steps(10_000).load(&project).unwrap();
limited.trust();
assert!(matches!(
    limited.call(CallTarget::Project, "Spin", vec![]).unwrap_err(),
    Error::Run(RunError::Limit(_))
));
assert!(!limited.is_usable());       // the allowance is spent
limited.reset_step_counter();        // a fresh allowance for the next handler
assert!(limited.is_usable());
```

## 9. Threading

**What is `Send` and `Sync`** (pinned by `tests/send_sync.rs`):

| Type | `Send` | `Sync` | Why |
| --- | --- | --- | --- |
| `Project`, `Report`, `Diagnostic`, `Error`, `CallLimits`, `Policy`, `ResourceLimits`, `Profile`, `Capability`, `ModuleKind`, `RunStatus` | yes | yes | plain data |
| `CancelToken` | yes | yes | one `AtomicBool` behind an `Arc`; the handle meant to be used from another thread |
| `Variant`, `Arg`, `CallTarget`, `Outcome`, `CallResult`, `EventOutcome` | yes | not promised | values; an `ObjectRef` inside is a handle that means something only to the instance that issued it |
| `Engine` | no | no | it holds host-object factories (`Rc<dyn Fn>`), so it is built on the thread that uses it |
| `Instance`, `Session`, `HostContext`, host objects (`Dispatch`) | no | no | one session is one interpreter: `Rc`/`RefCell` object tables, a re-entrant call stack and a per-thread memory meter that charges the strings and arrays created while it runs. Moving it would break all three, so the compiler refuses it (`compile_fail` doctest on `Instance`) |
| `thread::InstanceThread` | yes | yes | owns a channel, not the instance (below) |

Sessions are independent: module variables, default instances, `Err`, the random
generator, open files and the budget are per session, so several `Instance`s may
run at the same time on different threads, one thread each.

**The pattern for an application with a multi-threaded runtime** is a dedicated
interpreter thread that owns the instance, and a command channel: other threads
send closures, the interpreter thread runs them one at a time in order and sends
the results back. `vba::thread::InstanceThread` is that pattern on `std` alone (one
thread, `std::sync::mpsc`; no runtime, no dependency). Build the engine, the project
and the host objects *inside* the builder closure, because they are not `Send`:

```rust
use std::sync::Arc;
use vba::thread::InstanceThread;
use vba::{Engine, Project, Variant};

let worker = Arc::new(
    InstanceThread::spawn(|| {
        let project = Project::from_source(
            "Module1",
            "Public Total As Long\n\
             Public Function Add(ByVal n As Long) As Long\n\
             \x20 Total = Total + n\n\
             \x20 Add = Total\n\
             End Function\n\
             Public Sub Spin()\n\
             \x20 Do\n\
             \x20 Loop\n\
             End Sub\n",
        );
        let mut instance = Engine::new().load(&project)?;
        instance.trust();
        Ok(instance)
    })
    .unwrap(),
);

// Any thread (a runtime's worker, a request handler) may submit commands; they
// are serialized, so no two ever run at once and the state is consistent.
let handles: Vec<_> = (1..=4)
    .map(|n| {
        let worker = worker.clone();
        std::thread::spawn(move || {
            worker.call(move |i| i.run("Add", vec![Variant::Long(n)])).unwrap().unwrap()
        })
    })
    .collect();
for h in handles {
    h.join().unwrap();
}
let total = worker.call(|i| i.get_global("Total")).unwrap().unwrap();
assert_eq!(total, Variant::Long(10));

// Cancellation crosses threads through the token; the running command ends with a
// classified error, and the instance is usable again once the token is reset.
let token = worker.cancel_token();
let pending = worker.submit(|i| i.run("Spin", vec![])).unwrap();
token.cancel();
assert!(matches!(pending.wait().unwrap(), Err(vba::Error::Run(vba::RunError::Cancelled(_)))));
token.reset();
worker.call(|i| i.run("Add", vec![Variant::Long(0)])).unwrap().unwrap();
```

From an async runtime: `call` blocks, so run it where blocking is allowed
(`spawn_blocking` or an equivalent), or `submit` and hand the `Reply` to such a
thread. Nothing in vba-rs depends on a runtime. Rules of the helper (details in
the module documentation): commands run in submission order; a command that
panics stops the thread (the instance may be half updated) and later commands
fail with `ThreadError::Stopped`; closing or dropping the handle lets the running
command finish, drops the queued ones and drops the instance without
`Class_Terminate` (run `Instance::shutdown` in a last command when that
matters); closing does not cancel the token. State that host objects share with
other threads is your design: make the shared part `Send + Sync` (an
`Arc<Mutex<..>>` or a channel to your own service) and keep the `Dispatch`
wrapper on the interpreter thread.

Without the helper the pattern is the same: spawn a thread, build everything in
it, loop on `Receiver::recv`, and keep a `CancelToken` clone outside. There is no
async runtime and no background thread inside vba-rs; a call returns when the
VBA code, and any nested work, has finished.

## 10. Determinism and the permission model

The default engine is deterministic: fixed clock (2000-01-01), empty environment,
in-memory file system, the language's seeded random generator. The same
project, arguments, host objects and configuration give the same output and
the same order of host calls. `system_environment()` and `real_files()` are the
opt-ins that remove that; nothing else reads the machine.

The program reaches only what the embedder hands over: files, `Shell`,
`CreateObject`, `Declare`, network and UI automation are refused with VBA error
70 unless the `Policy` grants them, and host objects see only what you pass.
Imported code never runs by itself: an instance refuses every call until
`trust()`. See [security-model.md](security-model.md) for the guarantees and the
tests that pin them.

## 11. Transactions belong to the host

The VM never owns a database transaction, a lock or a file handle of your
application. VBA code runs inside your calls and reaches your data only through
host methods, in whatever order the program calls them. Therefore:

* Open, commit and roll back transactions in **your** code (see
  `examples/form_events.rs`: the host stages the update, raises `BeforeUpdate`,
  reads `Cancel` and commits or discards).
* Define nested-call and rollback rules for your host methods: a handler may call
  `Update` while another `Update` is in progress, a method may fail after a
  partial change, a re-entrant member may be reached while you hold a borrow. Use
  savepoints or refuse nested use with an error; do not hold a `RefCell` borrow
  across a call that can re-enter (the VBA code you call may call back into the
  same object).
* An unhandled VBA error rolls back nothing by itself. If the failure should
  discard staged work, do it where you see `Err` from `call`/`raise_event_on`.
* A fatal fault or `reset()` drops VBA state without running `Class_Terminate`;
  do not rely on VBA cleanup code to release host resources.

## 12. What is not supported

* Real COM: host objects are `Dispatch` values in an in-process table, no
  `IUnknown`/`ITypeInfo`/apartments. Host members are bound late unless a
  `TypeProvider` describes the type (`Engine::load_with(.., |b| b.extra_provider(..))`).
* Host-defined members of document modules are known to the analysis only by name
  (`host_members`), not by type; there is no form description import.
* Subscribing the host to events that **VBA** classes raise (`RaiseEvent`); the
  other direction (host objects raising to `WithEvents` handlers) works.
* Re-entry from `terminate`, `held_references` and enumerators of host objects.
* Output deltas for nested calls (they are part of the outer call's output).
* User-defined type values and multi-dimensional arrays as host values
  (`Variant` helpers cover one-dimensional arrays); `ParamArray` elements do not
  write back.
* Calling `Property Let`/`Set` procedures through `call_on`; use
  `set_member`/`get_member`.
* Source lines are those of the module text as given to the engine; a project
  read from a file shows the line numbers of the code part, without attribute
  lines.
* Async, multi-threaded sharing of one instance, and preemptive timeouts other
  than the step budget and the cancel token.
* Compiled-only (source-less) modules, callbacks from native code (`AddressOf` serves the
  emulated `Declare` shims only), a native `Declare` backend
  and the other gaps listed in [vm.md](../runtime/vm.md) and
  [conformance.md](../tooling/conformance.md).

## Phased integration

A suggested order for an embedding application (a database application with
forms, reports and a recordset-like data layer) that wraps vba-rs in a thin
adapter crate behind a Cargo feature. Each phase is usable on its own and ends
with something you can test. `crates/api/vba-rs/examples/embedded_table.rs`
exercises phases 1 to 4 with an in-memory table, and
`crates/api/vba-rs/tests/host_app.rs` pins the behavior.

| Phase | Goal | Facade items | Done when |
| --- | --- | --- | --- |
| 1. Source and analysis | Module text comes from your storage; diagnostics are shown, nothing runs | `Project::from_modules`, `Project::from_streams`, `Engine::analyze`, `Report` | a broken module is reported and `Engine::load` refuses it (`Error::kind() == "load"`) |
| 2. Run Public procedures | Call a procedure by name with `ByRef` arguments, read results and printed text | `Engine::load`, `Instance::trust`, `Instance::call`, `Arg::by_ref`, `CallResult::byref` | a `Function`/`Sub` round-trips values; nothing runs before `trust()` |
| 3. Limits and cancellation | Every handler is bounded and can be stopped | `Instance::with_call_limits(CallLimits::new().max_steps(..).max_memory(..).timeout(..), ..)`, `Instance::cancel_token`, `Instance::is_usable` | a runaway loop ends with `limit` or `cancelled` and the instance serves the next call |
| 4. Host objects | `Application`, `DoCmd`, `Workspace`, recordsets and collections as `Dispatch` objects; transactions are yours | `Engine::dispatch_global`, `Dispatch`, `DispError::raise`, `Error::error_info` | `On Error` handlers see your numbers, descriptions and sources in `Err`; `For Each` and `rs!Field` work |
| 5. Forms, reports and events | `Me` is a host object; events carry `ByRef` results such as `Cancel` | `Instance::create_object(module, Some(me))`, `raise_event_on`, `EventOutcome`, `HostContext` | `BeforeUpdate` can cancel your update; a handler can call back into the form |
| 6. Persistence of code | Edited module text goes back to your storage | `Project::set_module_source`, `to_streams`, `WriteReport::changed_streams` | an unedited project writes back byte-identical streams |

Guidance for the adapter:

* Keep the adapter free of VBA semantics: it translates your objects into
  `Dispatch` implementations and your error type into `vba::Error` (and back with
  `Error::error_info()`), nothing else. Database-specific behavior belongs in
  the adapter, never in vba-rs.
* Build one `Engine` per configuration and one `Instance` per open document;
  instances are single-threaded, so give each its own thread or keep it on the
  application's UI thread. Share only the `CancelToken`.
* Put `with_call_limits` around every entry from the application (event
  handler, button click, immediate-window command) rather than relying on the
  engine-wide budget, which spans the whole life of the instance.
* Match `vba::Error` and the `RunStatus`/`LimitKind` enums with a wildcard arm;
  they are `#[non_exhaustive]` so new variants are not a breaking change.
* Transactions: expose `BeginTrans`/`CommitTrans`/`Rollback` as host methods and
  see section 11 for who owns what. A host error raised with
  `DispError::raise(number, description, source)` is trappable and lands in
  `Err.Number`, `Err.Description` and `Err.Source` unchanged.

## Reference for the pieces used above

| Need | Item |
| --- | --- |
| Create / destroy instances | `Instance::create_object`, `default_instance`, `destroy_object` |
| Call | `Instance::call(CallTarget, name, Vec<Arg>)`, `call_on`, `run`; `Arg::val`, `by_ref`, `missing`, `named`; `CallResult { value, status, byref, output }` |
| Events | `raise_event_on` -> `EventOutcome { handled, status, byref, output }`, `has_procedure`, `deliver_pending_events` |
| Variables | `get_global`, `set_global`, `get_member`, `set_member`, `release` |
| Metadata | `modules`, `list_public_members`, `modules_defining`; `ModuleSummary`, `ProcedureSummary`, `ParamSummary`, `VariableSummary` |
| Errors | `Error`, `runtime::Error::{report, error_info, stack, detail}`, `ErrorReport`, `VbaErrorInfo`, `StackFrame`, `Instance::last_error` |
| Re-entrancy | `Dispatch::reentrant`, `HostContext::current`, `VmConfig::max_reentrancy_depth` |
| Limits | `Engine::limits/max_steps/vm_config/cancel_token`, `Instance::is_usable/reset_step_counter/reset/with_call_limits/cancel_token`, `CallLimits` |
| Source text | `Project::from_modules`, `add_module`, `set_module_source` |
| Values | `Variant` constructors, `From`/`TryFrom`, `Display` |

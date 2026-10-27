//! The skeleton of an adapter between an embedding application and vba-rs.
//!
//! Run it with `cargo run -p vba-rs --example adapter_skeleton`; the same
//! flow is asserted by `cargo test -p vba-rs --examples`. The ordered checklist
//! it follows is `docs/architecture/adapter-checklist.md`; every numbered
//! comment below is a step of that list.
//!
//! The application is simulated by a `Store`: a table of (stream path, bytes)
//! rows, which is all that crosses the seam. Everything vba-rs related lives in
//! `Adapter`, so the rest of the application never sees a `vba` type.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vba::automation::ParamInfo;
use vba::automation::testing::MemObject;
use vba::thread::{InstanceThread, ThreadError};
use vba::{
    Arg, CallLimits, Capability, Engine, ModuleKind, Project, ProjectStreams, ReadLimits, Variant,
    WriteOptions,
};

// ---- the application side ---------------------------------------------------------

/// The application's table of project streams.
#[derive(Default)]
struct Store {
    rows: BTreeMap<String, Vec<u8>>,
}

/// What the application sees of a failure. Map once, here, and keep `vba::Error`
/// out of the application (step 9).
#[derive(Debug, PartialEq)]
enum AdapterError {
    /// The stored project is unreadable or does not compile; one text per diagnostic.
    InvalidProject(Vec<String>),
    /// The user's code failed: VBA error number, description, procedure.
    Script {
        number: i32,
        description: String,
        procedure: String,
    },
    /// A resource limit (steps, memory, depth, size) ended the call.
    LimitExceeded(String),
    /// The call was cancelled or ran out of time.
    Cancelled,
    /// The code asked for something the application did not grant.
    Denied(String),
    /// A request the application made wrongly (unknown procedure, ...).
    Rejected(String),
    /// A bug or a stopped worker; log it and offer the minimal repro (step 15).
    Internal(String),
}

fn map_error(e: &vba::Error) -> AdapterError {
    match e.kind() {
        "load" | "format" | "model" | "source" => AdapterError::InvalidProject(vec![e.to_string()]),
        "vba" | "host" => {
            let info = e.error_info();
            AdapterError::Script {
                number: info.as_ref().map_or(0, |i| i.number),
                description: info
                    .as_ref()
                    .map_or_else(|| e.to_string(), |i| i.description.clone()),
                procedure: info.map(|i| i.procedure).unwrap_or_default(),
            }
        }
        "limit" => AdapterError::LimitExceeded(e.to_string()),
        "cancelled" => AdapterError::Cancelled,
        "permission-denied" => AdapterError::Denied(e.to_string()),
        "internal" | "poisoned" | "bridge" | "unsupported" | "io" => {
            AdapterError::Internal(e.to_string())
        }
        // not-trusted, no-such-procedure, ... and kinds added later.
        _ => AdapterError::Rejected(e.to_string()),
    }
}

fn map_thread(e: &ThreadError) -> AdapterError {
    match e {
        ThreadError::Build(inner) => map_error(inner),
        other => AdapterError::Internal(other.to_string()),
    }
}

// ---- the adapter -----------------------------------------------------------------------

/// Everything the application decides, in one place (steps 2, 7 and 8).
struct Settings {
    /// The wall-clock allowance of one call.
    call_timeout: Duration,
    /// The step allowance of one call.
    call_steps: u64,
    /// The memory allowance of one call, in bytes.
    call_memory: u64,
    /// Whether the user's code may call `CreateObject` for the application's classes.
    allow_create_object: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            call_timeout: Duration::from_secs(5),
            call_steps: 5_000_000,
            call_memory: 32 << 20,
            allow_create_object: false,
        }
    }
}

struct Adapter {
    worker: InstanceThread,
    settings: Settings,
    /// What host objects wrote; shared with the interpreter thread (step 12).
    log: Arc<Mutex<Vec<String>>>,
}

impl Adapter {
    /// Steps 1 to 6: read the streams, analyse, build the engine, load, trust.
    fn open(store: &Store, settings: Settings) -> Result<Adapter, AdapterError> {
        // 1. Streams from the table. Reading never runs anything and is bounded by `ReadLimits`.
        let mut streams = ProjectStreams::new();
        for (path, bytes) in &store.rows {
            streams.insert(path.clone(), bytes.clone());
        }
        let limits = ReadLimits::default();
        let project = Project::from_streams(&streams, &limits)
            .map_err(|e| AdapterError::InvalidProject(vec![e.to_string()]))?;

        // 3. Diagnostics before anything runs. Refuse a project that does not compile.
        let probe = Engine::new().host_members(["Dirty"]);
        let report = probe.analyze(&project);
        if report.has_errors() {
            return Err(AdapterError::InvalidProject(report.rendered().to_vec()));
        }

        let log = Arc::new(Mutex::new(Vec::new()));
        let host_log = log.clone();
        let allow_create = settings.allow_create_object;
        // 5. The interpreter thread owns the instance. Everything that is not `Send`
        //    (the engine, the host objects) is built inside the closure.
        let worker = InstanceThread::spawn(move || {
            // 2. Security defaults: `Engine::new()` is deny-all with finite limits and a
            //    60 s call timeout; grant only what the application needs, as narrowly as it can.
            let mut engine = Engine::new().host_members(["Dirty"]);
            if allow_create {
                engine = engine.grant_prefixes(Capability::ComCreate, ["Demo.*"]);
            }
            // 4. Host objects: a global the code can name, and a class for `CreateObject`.
            engine = engine.dispatch_global("Application", move || {
                let log = host_log.clone();
                MemObject::new("Application")
                    .property("Name", Variant::from("Demo"))
                    .method(
                        "Log",
                        vec![ParamInfo::required("Message").by_val()],
                        move |_, call| {
                            let text = String::try_from(&call.args[0].value).unwrap_or_default();
                            log.lock().unwrap().push(text);
                            Ok(Variant::Empty)
                        },
                    )
            });
            engine = engine.dispatch_class("Demo.Counter", || {
                MemObject::new("Counter").property("Value", Variant::Long(0))
            });
            let mut instance = engine.load(&project)?;
            // 6. Trust is the application's decision, made after the checks above.
            instance.trust();
            Ok(instance)
        })
        .map_err(|e| map_thread(&e))?;
        Ok(Adapter {
            worker,
            settings,
            log,
        })
    }

    /// Steps 7 and 8: every call has its own allowance; map the result.
    fn call(&self, procedure: &str, args: Vec<Variant>) -> Result<Variant, AdapterError> {
        let procedure = procedure.to_owned();
        let limits = CallLimits::new()
            .max_steps(self.settings.call_steps)
            .max_memory(self.settings.call_memory)
            .timeout(self.settings.call_timeout);
        self.worker
            .call(move |i| {
                let r = i.with_call_limits(limits, |i| i.run(&procedure, args));
                // 8. A spent budget does not make the instance unusable; a fatal fault
                //    leaves half-run state, so start over when it matters.
                if !i.is_usable() {
                    let _ = i.reset();
                }
                r
            })
            .map_err(|e| map_thread(&e))?
            .map(|outcome| outcome.value)
            .map_err(|e| map_error(&e))
    }

    /// Step 10: events with `ByRef` results. Returns whether the handler set `Cancel`.
    fn before_update(&self, form_module: &str) -> Result<bool, AdapterError> {
        let module = form_module.to_owned();
        let limits = CallLimits::new().max_steps(self.settings.call_steps);
        self.worker
            .call(move |i| {
                let me = std::rc::Rc::new(
                    MemObject::new("Form").property("Dirty", Variant::Boolean(true)),
                );
                let form = i.create_object(&module, Some(me))?;
                let r = i.with_call_limits(limits, |i| {
                    i.raise_event_on(
                        form,
                        "Form",
                        "BeforeUpdate",
                        vec![Arg::by_ref(Variant::Integer(0))],
                    )
                });
                i.destroy_object(form)?;
                Ok::<_, vba::Error>(r?.byref[0] != Variant::Integer(0))
            })
            .map_err(|e| map_thread(&e))?
            .map_err(|e| map_error(&e))
    }

    /// Step 8: another thread stops a running call.
    fn cancel(&self) {
        self.worker.cancel_token().cancel();
    }

    fn reset_cancel(&self) {
        self.worker.cancel_token().reset();
    }

    fn close(self) {
        let _ = self.worker.close();
    }
}

/// Step 13: write an edit back. Stores the streams that changed, deletes the ones
/// the write dropped (invalidated compiled caches), leaves the rest alone.
fn edit_module(store: &mut Store, module: &str, source: &str) -> Result<usize, AdapterError> {
    let mut streams = ProjectStreams::new();
    for (path, bytes) in &store.rows {
        streams.insert(path.clone(), bytes.clone());
    }
    let wrap = |e: vba::Error| AdapterError::InvalidProject(vec![e.to_string()]);
    let mut project = Project::from_streams(&streams, &ReadLimits::default()).map_err(wrap)?;
    project.set_module_source(module, source).map_err(wrap)?;
    let (out, report) = project.to_streams(&WriteOptions::default()).map_err(wrap)?;
    for path in &report.changed_streams {
        match out.get(path) {
            Some(bytes) => {
                store.rows.insert(path.clone(), bytes.to_vec());
            }
            None => {
                store.rows.remove(path);
            }
        }
    }
    Ok(report.changed_streams.len())
}

// ---- the flow --------------------------------------------------------------------------

const MODULES: [(&str, ModuleKind, &str); 2] = [
    (
        "Module1",
        ModuleKind::Standard,
        "Public Function Twice(ByVal n As Long) As Long\r\n  Twice = n * 2\r\nEnd Function\r\n\
         Public Sub Hello()\r\n  Application.Log \"hello from \" & Application.Name\r\nEnd Sub\r\n\
         Public Sub Spin()\r\n  Do\r\n  Loop\r\nEnd Sub\r\n\
         Public Function Fail() As Long\r\n  Fail = 1 \\ 0\r\nEnd Function\r\n\
         Public Sub Make()\r\n  Dim c As Object\r\n  Set c = CreateObject(\"Demo.Counter\")\r\nEnd Sub\r\n",
    ),
    (
        "Form_Orders",
        ModuleKind::Document,
        "Private Sub Form_BeforeUpdate(Cancel As Integer)\r\n  If Dirty Then Cancel = True\r\nEnd Sub\r\n",
    ),
];

/// What the flow observed, for the test.
#[derive(Debug, Default)]
struct Observed {
    twice: Option<Variant>,
    script_error: Option<AdapterError>,
    limit_error: Option<AdapterError>,
    denied: Option<AdapterError>,
    cancelled: Option<AdapterError>,
    cancel_event: bool,
    log: Vec<String>,
    changed_streams: usize,
    after_edit: Option<Variant>,
}

fn run() -> Result<Observed, AdapterError> {
    let mut seen = Observed::default();

    // The application stores a project the way it stores everything else.
    let mut store = Store::default();
    let project = Project::from_modules("Demo", MODULES).map_err(|e| map_error(&e))?;
    let (streams, _) = project
        .to_streams(&WriteOptions::default())
        .map_err(|e| map_error(&e))?;
    for (path, bytes) in streams.iter() {
        store.rows.insert(path.to_string(), bytes.to_vec());
    }

    // 14. Pinning: log the revision that is linked.
    println!("vba-rs {}", vba::VERSION);

    let settings = Settings {
        call_timeout: Duration::from_secs(5),
        call_steps: 200_000,
        ..Settings::default()
    };
    let adapter = Adapter::open(&store, settings)?;

    seen.twice = Some(adapter.call("Twice", vec![Variant::Long(21)])?);
    adapter.call("Hello", vec![])?;

    // 9. Errors come back mapped; the instance stays usable.
    seen.script_error = adapter.call("Fail", vec![]).err();
    seen.limit_error = adapter.call("Spin", vec![]).err();
    seen.denied = adapter.call("Make", vec![]).err();

    // 8. Cancellation from another thread.
    let spin = std::thread::scope(|s| {
        let canceller = s.spawn(|| {
            std::thread::sleep(Duration::from_millis(20));
            adapter.cancel();
        });
        // A generous step budget so that the cancel, not the budget, ends it.
        let r = adapter.worker.call(|i| i.run("Spin", vec![])).ok();
        let _ = canceller.join();
        r
    });
    if let Some(Err(e)) = spin {
        seen.cancelled = Some(map_error(&e));
    }
    adapter.reset_cancel();

    seen.cancel_event = adapter.before_update("Form_Orders")?;

    seen.log = adapter.log.lock().unwrap().clone();
    adapter.close();

    // 13. Edit a module and write back only what changed.
    seen.changed_streams = edit_module(
        &mut store,
        "Module1",
        "Public Function Twice(ByVal n As Long) As Long\r\n  Twice = n * 3\r\nEnd Function\r\n",
    )?;
    let reopened = Adapter::open(&store, Settings::default())?;
    seen.after_edit = Some(reopened.call("Twice", vec![Variant::Long(7)])?);
    reopened.close();
    Ok(seen)
}

fn main() {
    match run() {
        Ok(seen) => println!("{seen:#?}"),
        Err(e) => {
            eprintln!("adapter failed: {e:?}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vba::RunError;

    #[test]
    fn the_skeleton_runs() {
        let seen = run().expect("the flow completes");
        assert_eq!(seen.twice, Some(Variant::Long(42)));
        assert!(matches!(
            seen.script_error,
            Some(AdapterError::Script { number: 11, .. })
        ));
        assert!(matches!(
            seen.limit_error,
            Some(AdapterError::LimitExceeded(_))
        ));
        assert!(matches!(seen.denied, Some(AdapterError::Denied(_))));
        assert_eq!(seen.cancelled, Some(AdapterError::Cancelled));
        assert!(seen.cancel_event);
        assert_eq!(seen.log, vec!["hello from Demo".to_owned()]);
        assert!(seen.changed_streams >= 1);
        assert_eq!(seen.after_edit, Some(Variant::Long(21)));
    }

    #[test]
    fn a_broken_project_is_refused_before_anything_runs() {
        let mut store = Store::default();
        // Unreadable rows are an error of the stored data, not a panic.
        store.rows.insert("PROJECT".into(), vec![1, 2, 3]);
        assert!(matches!(
            Adapter::open(&store, Settings::default()),
            Err(AdapterError::InvalidProject(_))
        ));
    }

    #[test]
    fn run_errors_have_a_stable_mapping() {
        let e = vba::Error::Run(RunError::NotTrusted);
        assert!(matches!(map_error(&e), AdapterError::Rejected(_)));
    }
}

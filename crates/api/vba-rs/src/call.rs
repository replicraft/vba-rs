//! Arguments, targets and results of [`Instance::call`](crate::Instance::call).

use std::time::Duration;

use vba_runtime::Target as RtTarget;
use vba_value::{ObjectRef, Variant};
use vba_vm::{Arg as VmArg, RunStatus};

/// One argument of a call from the embedder.
///
/// Positional arguments come first, in parameter order; [`Arg::named`]
/// arguments follow and may appear in any order, as in VBA
/// (`Proc 1, Name:=2`). A [`Arg::missing`] argument leaves an `Optional`
/// parameter at its default (`IsMissing` for a `Variant` without one).
///
/// ```
/// use vba::{Arg, Variant};
///
/// let args = vec![Arg::val(1), Arg::by_ref(Variant::Long(0)), Arg::named("Mode", Arg::val("x"))];
/// assert_eq!(args.len(), 3);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Arg {
    /// A value passed by value (a `ByRef` parameter receives a copy).
    Val(Variant),
    /// A caller-owned variable: a `ByRef` parameter writes through to it
    /// and the final value comes back in [`CallResult::byref`]. A `ByVal`
    /// parameter, or a `ParamArray` element, gets a copy and is reported
    /// unchanged.
    ByRef(Variant),
    /// An omitted argument.
    Missing,
    /// An argument passed by parameter name.
    Named(String, Box<Arg>),
}

impl Arg {
    /// A by-value argument.
    pub fn val(v: impl Into<Variant>) -> Arg {
        Arg::Val(v.into())
    }

    /// A `ByRef` slot holding `v`.
    pub fn by_ref(v: impl Into<Variant>) -> Arg {
        Arg::ByRef(v.into())
    }

    /// An omitted argument.
    pub fn missing() -> Arg {
        Arg::Missing
    }

    /// `name:=arg`.
    pub fn named(name: &str, arg: Arg) -> Arg {
        Arg::Named(name.to_owned(), Box::new(arg))
    }
}

impl From<Variant> for Arg {
    fn from(v: Variant) -> Arg {
        Arg::Val(v)
    }
}

/// Splits an argument list into positional and named arguments for the VM.
/// A positional argument after a named one, or a named argument inside a
/// named one, is a misuse of the call.
pub(crate) type SplitArgs = (Vec<VmArg>, Vec<(String, VmArg)>);

pub(crate) fn split_args(args: Vec<Arg>) -> Result<SplitArgs, &'static str> {
    fn plain(a: Arg) -> Result<VmArg, &'static str> {
        match a {
            Arg::Val(v) => Ok(VmArg::Value(v)),
            Arg::ByRef(v) => Ok(VmArg::ByRef(v)),
            Arg::Missing => Ok(VmArg::Missing),
            Arg::Named(..) => Err("a named argument cannot be nested in another"),
        }
    }
    let mut positional = Vec::new();
    let mut named = Vec::new();
    for a in args {
        match a {
            Arg::Named(n, inner) => named.push((n, plain(*inner)?)),
            other => {
                if !named.is_empty() {
                    return Err("a positional argument follows a named argument");
                }
                positional.push(plain(other)?);
            }
        }
    }
    Ok((positional, named))
}

/// Allowances for one scope of calls on an instance, applied with
/// [`Instance::with_call_limits`](crate::Instance::with_call_limits).
///
/// Every field is optional and narrows what the engine already enforces; a
/// scope never grants more than [`Engine::limits`](crate::Engine::limits)
/// allows. The counters of the instance are untouched, so a spent scope does
/// not make the instance unusable.
///
/// ```
/// use std::time::Duration;
/// use vba::CallLimits;
///
/// let per_handler = CallLimits::new()
///     .max_steps(1_000_000)
///     .max_memory(8 * 1024 * 1024)
///     .timeout(Duration::from_millis(250));
/// assert_eq!(per_handler.steps(), Some(1_000_000));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CallLimits {
    steps: Option<u64>,
    memory: Option<u64>,
    timeout: Option<Duration>,
}

impl CallLimits {
    /// No allowance beyond the engine's.
    pub fn new() -> CallLimits {
        CallLimits::default()
    }

    /// At most `steps` interpreter steps from the start of the scope. When
    /// exhausted the call ends with `Error::Run(Limit(Steps))`.
    pub fn max_steps(mut self, steps: u64) -> CallLimits {
        self.steps = Some(steps);
        self
    }

    /// At most `bytes` of VBA-visible memory more than is in use when the
    /// scope starts. When exceeded the call ends with `Error::Run(Limit(Memory))`.
    pub fn max_memory(mut self, bytes: u64) -> CallLimits {
        self.memory = Some(bytes);
        self
    }

    /// At most this much wall-clock time, checked cooperatively every
    /// [`CANCEL_POLL_INTERVAL`](vba_host::POLL_INTERVAL) steps and at
    /// `DoEvents`. When exceeded the call ends with `Error::Run(Cancelled)`;
    /// the [`CancelToken`](crate::CancelToken) is not touched. A host member
    /// that blocks is not interrupted.
    pub fn timeout(mut self, timeout: Duration) -> CallLimits {
        self.timeout = Some(timeout);
        self
    }

    /// The step allowance, if set.
    pub fn steps(&self) -> Option<u64> {
        self.steps
    }

    /// The memory allowance in bytes, if set.
    pub fn memory(&self) -> Option<u64> {
        self.memory
    }

    /// The time allowance, if set.
    pub fn time(&self) -> Option<Duration> {
        self.timeout
    }

    pub(crate) fn to_scope(self) -> vba_host::ScopeLimits {
        vba_host::ScopeLimits {
            max_steps: self.steps,
            max_memory_bytes: self.memory,
            timeout: self.timeout,
        }
    }
}

/// What a call is addressed to.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CallTarget {
    /// The project: `Proc` of the standard modules or `Module.Proc`; a
    /// procedure of a document or designer module runs on its default
    /// instance.
    Project,
    /// One instance created with
    /// [`Instance::create_object`](crate::Instance::create_object) (or a
    /// default instance obtained with
    /// [`Instance::default_instance`](crate::Instance::default_instance)).
    Object(ObjectRef),
}

impl CallTarget {
    pub(crate) fn to_runtime(&self) -> RtTarget {
        match self {
            CallTarget::Project => RtTarget::Project,
            CallTarget::Object(o) => RtTarget::Object(*o),
        }
    }
}

/// What one call produced.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CallResult {
    /// The function result (`Empty` for a `Sub`, or when the run did not
    /// complete). An object result carries one reference the caller owns
    /// (see [`Instance::release`](crate::Instance::release)).
    pub value: Variant,
    /// How the procedure finished.
    pub status: RunStatus,
    /// The final values of the [`Arg::ByRef`] arguments, in argument
    /// order. See [`Arg::ByRef`] for what a `ByVal` parameter reports.
    pub byref: Vec<Variant>,
    /// The `Debug.Print` text written during this call.
    pub output: String,
}

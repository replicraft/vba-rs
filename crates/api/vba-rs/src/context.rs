//! [`HostContext`]: calling back into the running session from a host
//! object.

use std::rc::Rc;

use vba_automation::{DispError, Dispatch, ObjectTable};
use vba_runtime::HostContext as RtContext;
use vba_value::{ObjectRef, Variant};
use vba_vm::RunStatus;

use crate::Error;
use crate::call::{Arg, CallResult, CallTarget, split_args};
use crate::objects::EventOutcome;

/// A re-entrant handle to the session that is running the host object
/// holding it: the same operations as [`Instance`](crate::Instance), usable
/// **while a VBA statement is waiting for the host call to return**.
///
/// A host object obtains one inside its `Dispatch::invoke` with
/// [`HostContext::current`]; it must also return `true` from
/// [`Dispatch::reentrant`](crate::automation::Dispatch::reentrant), which
/// is what makes the interpreter wait in a way that allows the call-back.
///
/// # Semantics
///
/// * **Synchronous and nested.** A call, event or instance creation made
///   through the context runs to completion before the context method
///   returns, on the same call stack, `Err` object and module state as the
///   outer VBA code. Output printed by nested code is part of the output of
///   the outermost [`Instance`](crate::Instance) call; `CallResult::output`
///   of a nested call is empty.
/// * **One budget.** Steps, memory and call depth are shared with the outer
///   run. Nesting is limited by `VmConfig::max_reentrancy_depth` (default
///   4); beyond it the context methods fail with
///   [`RunError::Reentrancy`](crate::RunError::Reentrancy).
/// * **Errors.** An unhandled VBA error in the nested code comes back as
///   `Err`. Return it from the host member with `DispError::from(err)` and
///   the outer VBA code sees a trappable run-time error (`On Error` works);
///   handle it in the host and the outer code never knows. A *fatal* fault
///   (resource limit, cancellation, `End`, `Stop`, an unsupported feature)
///   also comes back as `Err`, but ends the outer run too, once the host
///   call returns, whatever the host did with it.
/// * **Lifetime.** The context is only valid while the host member runs;
///   afterwards its methods fail with `RunError::Reentrancy`. It is not
///   `Send`.
/// * **Order is deterministic.** Everything runs inline in the order the
///   host calls it; nothing is queued.
#[derive(Clone)]
pub struct HostContext(RtContext);

impl std::fmt::Debug for HostContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostContext")
            .field("active", &self.is_active())
            .field("depth", &self.depth())
            .finish()
    }
}

impl HostContext {
    /// The context of the session that is calling the host object right
    /// now. `None` when the table belongs to no running session or the
    /// object did not declare itself `reentrant`.
    #[must_use]
    pub fn current(table: &ObjectTable) -> Option<HostContext> {
        RtContext::current(table).map(HostContext)
    }

    /// Whether the session is still waiting in the host call.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.0.is_active()
    }

    /// Nested runs active right now.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.0.depth()
    }

    /// [`Instance::create_object`](crate::Instance::create_object).
    pub fn create_object(
        &self,
        module: &str,
        host_me: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        self.0
            .with(|ops| ops.create_object(module, host_me))
            .map_err(Error::Run)
    }

    /// [`Instance::default_instance`](crate::Instance::default_instance).
    pub fn default_instance(
        &self,
        module: &str,
        host_me: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        self.0
            .with(|ops| ops.default_instance(module, host_me))
            .map_err(Error::Run)
    }

    /// [`Instance::destroy_object`](crate::Instance::destroy_object).
    pub fn destroy_object(&self, object: ObjectRef) -> Result<(), Error> {
        self.0
            .with(|ops| ops.destroy_object(&object))
            .map_err(Error::Run)
    }

    /// [`Instance::call`](crate::Instance::call).
    pub fn call(
        &self,
        target: CallTarget,
        procedure: &str,
        args: Vec<Arg>,
    ) -> Result<CallResult, Error> {
        let (positional, named) = split_args(args).map_err(|why| {
            Error::Run(vba_runtime::Error::NotCallable(procedure.to_owned(), why))
        })?;
        let out = self
            .0
            .with(|ops| ops.call(&target.to_runtime(), procedure, positional, named))
            .map_err(Error::Run)?;
        Ok(CallResult {
            value: out.value,
            status: out.status,
            byref: out.byref,
            output: String::new(),
        })
    }

    /// [`Instance::call_on`](crate::Instance::call_on).
    pub fn call_on(
        &self,
        object: ObjectRef,
        procedure: &str,
        args: Vec<Arg>,
    ) -> Result<CallResult, Error> {
        self.call(CallTarget::Object(object), procedure, args)
    }

    /// [`Instance::has_procedure`](crate::Instance::has_procedure).
    #[must_use]
    pub fn has_procedure(&self, object: ObjectRef, name: &str) -> bool {
        self.0
            .with(|ops| Ok(ops.has_procedure(&object, name)))
            .unwrap_or(false)
    }

    /// [`Instance::raise_event_on`](crate::Instance::raise_event_on): the
    /// handler runs now and its `ByRef` arguments (`Cancel`) are in the
    /// outcome when this returns.
    pub fn raise_event_on(
        &self,
        object: ObjectRef,
        source: &str,
        event: &str,
        args: Vec<Arg>,
    ) -> Result<EventOutcome, Error> {
        let (positional, named) = split_args(args).map_err(|why| {
            Error::Run(vba_runtime::Error::NotCallable(
                format!("{source}_{event}"),
                why,
            ))
        })?;
        let originals = crate::objects::by_ref_values(&positional, &named);
        let r = self
            .0
            .with(|ops| ops.raise_event_on(&object, source, event, positional, named))
            .map_err(Error::Run)?;
        Ok(match r {
            Some(out) => EventOutcome {
                handled: true,
                status: out.status,
                byref: out.byref,
                output: String::new(),
            },
            None => EventOutcome {
                handled: false,
                status: RunStatus::Completed,
                byref: originals,
                output: String::new(),
            },
        })
    }

    /// [`Instance::get_global`](crate::Instance::get_global).
    pub fn get_global(&self, name: &str) -> Result<Variant, Error> {
        self.0.with(|ops| ops.get_global(name)).map_err(Error::Run)
    }

    /// [`Instance::set_global`](crate::Instance::set_global).
    pub fn set_global(&self, name: &str, value: Variant) -> Result<(), Error> {
        self.0
            .with(|ops| ops.set_global(name, value))
            .map_err(Error::Run)
    }

    /// [`Instance::get_member`](crate::Instance::get_member).
    pub fn get_member(&self, object: ObjectRef, name: &str) -> Result<Variant, Error> {
        self.0
            .with(|ops| ops.get_member(&object, name))
            .map_err(Error::Run)
    }

    /// [`Instance::set_member`](crate::Instance::set_member).
    pub fn set_member(&self, object: ObjectRef, name: &str, value: Variant) -> Result<(), Error> {
        self.0
            .with(|ops| ops.set_member(&object, name, value))
            .map_err(Error::Run)
    }

    /// [`Instance::release`](crate::Instance::release).
    pub fn release(&self, value: &Variant) {
        let _ = self.0.with(|ops| {
            ops.release(value);
            Ok(())
        });
    }
}

impl From<&Error> for DispError {
    /// See `From<&runtime::Error>`: the Automation error a host member
    /// returns for a failed nested operation.
    fn from(e: &Error) -> DispError {
        match e {
            Error::Run(r) => DispError::from(r),
            other => DispError::raise(5, other.to_string(), "vba"),
        }
    }
}

impl From<Error> for DispError {
    fn from(e: Error) -> DispError {
        DispError::from(&e)
    }
}

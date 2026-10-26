//! [`HostContext`]: the handle a host object uses to call back into the
//! session that is running it.

use std::rc::Rc;

use vba_automation::{DispError, ExcepInfo, ObjectTable};
use vba_vm::{LimitKind, ReentryError, ReentrySlot};

use crate::error::Error;
use crate::ops::Ops;

/// A re-entrant handle to the running session, valid while the host member
/// that obtained it executes.
///
/// A host object gets one inside `Dispatch::invoke` with
/// [`HostContext::current`], provided it declared
/// [`Dispatch::reentrant`](vba_automation::Dispatch::reentrant). The
/// interpreter is *suspended* at the VBA statement that made the call; the
/// context lets the host create instances, raise events and call
/// procedures, which run to completion before the host call returns. See
/// `docs/architecture/embedding-guide.md` for the semantics.
#[derive(Clone)]
pub struct HostContext {
    slot: Rc<ReentrySlot>,
}

impl HostContext {
    /// The context of the session that is calling the host object right
    /// now, or `None` when the table belongs to no running session, or the
    /// object did not declare itself re-entrant.
    pub fn current(table: &ObjectTable) -> Option<HostContext> {
        let slot = ReentrySlot::from_table(table)?;
        slot.is_suspended().then_some(HostContext { slot })
    }

    /// Whether the session is still suspended in the host call (a context
    /// kept past its call stops working).
    pub fn is_active(&self) -> bool {
        self.slot.is_suspended()
    }

    /// How many nested runs are active, counting the one being started.
    pub fn depth(&self) -> u32 {
        self.slot.depth()
    }

    /// Runs `f` on the suspended session. Fails with
    /// [`Error::Reentrancy`] when the context is stale or the nesting limit
    /// ([`VmConfig::max_reentrancy_depth`](vba_vm::VmConfig::max_reentrancy_depth))
    /// is reached.
    pub fn with<R>(&self, f: impl FnOnce(&mut Ops<'_>) -> Result<R, Error>) -> Result<R, Error> {
        match self.slot.with_vm(|vm| f(&mut Ops::new(vm))) {
            Ok(r) => r,
            Err(e @ (ReentryError::NotSuspended | ReentryError::TooDeep { .. })) => {
                Err(Error::Reentrancy(e.to_string()))
            }
        }
    }
}

impl From<&Error> for DispError {
    /// The Automation error a host member returns for a failed nested
    /// operation, so the outer VBA code sees an ordinary run-time error
    /// (`On Error` can trap it): the number, description, source and help
    /// of an unhandled VBA error; a stand-in number for the other
    /// families. A fatal fault (limit, cancellation, `End`) is additionally
    /// re-raised untrappably by the interpreter when the host call returns.
    fn from(e: &Error) -> DispError {
        match e {
            Error::Vba(r) | Error::Host(r) | Error::PermissionDenied(r) => {
                DispError::Exception(ExcepInfo {
                    code: 0,
                    scode: vba_automation::hresult::number_to_hresult(r.number),
                    source: r.source.clone(),
                    description: r.description.clone(),
                    help_file: r.help_file.clone(),
                    help_context: r.help_context as u32,
                })
            }
            Error::Limit(l) => {
                let n = match l {
                    LimitKind::Steps => 18,
                    LimitKind::Memory | LimitKind::StringLength | LimitKind::ArraySize => 7,
                    LimitKind::CallDepth => 28,
                    // Any limit added later is an out-of-resource condition too.
                    _ => 7,
                };
                DispError::raise(n, l.to_string(), "vba")
            }
            Error::Cancelled(s) => DispError::raise(18, format!("cancelled: {s}"), "vba"),
            other => DispError::raise(5, other.to_string(), "vba"),
        }
    }
}

impl From<Error> for DispError {
    fn from(e: Error) -> DispError {
        DispError::from(&e)
    }
}

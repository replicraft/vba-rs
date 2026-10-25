//! Application-agnostic event dispatch.
//!
//! Events are identified by an opaque source name and an event name; the host
//! application decides what sources exist (a form, a workbook, a timer ...).

use crate::host::HostResult;
use std::cell::RefCell;
use std::rc::Rc;
use vba_automation::{Arg, ObjectTable, names_equal};

/// Delivers an event to whatever handles it.
pub trait EventDispatch {
    /// `true` if some handler exists for the event.
    fn has_handler(&self, source: &str, event: &str) -> bool;

    /// Delivers the event. Having no handler is not an error. ByRef
    /// arguments (such as `Cancel`) are updated in place.
    fn dispatch(
        &self,
        table: &ObjectTable,
        source: &str,
        event: &str,
        args: &mut [Arg],
    ) -> HostResult<()>;
}

type Handler = Rc<dyn Fn(&ObjectTable, &mut [Arg]) -> HostResult<()>>;

/// A registry of handlers keyed by (source, event), matched
/// case-insensitively, invoked in registration order; the first error stops
/// delivery.
#[derive(Default)]
pub struct EventRouter {
    handlers: RefCell<Vec<(String, String, Handler)>>,
}

impl EventRouter {
    /// An empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler.
    pub fn register(
        &self,
        source: &str,
        event: &str,
        handler: impl Fn(&ObjectTable, &mut [Arg]) -> HostResult<()> + 'static,
    ) {
        self.handlers
            .borrow_mut()
            .push((source.to_string(), event.to_string(), Rc::new(handler)));
    }

    /// Removes every handler of a source.
    pub fn remove_source(&self, source: &str) {
        self.handlers
            .borrow_mut()
            .retain(|(s, _, _)| !names_equal(s, source));
    }
}

impl EventDispatch for EventRouter {
    fn has_handler(&self, source: &str, event: &str) -> bool {
        self.handlers
            .borrow()
            .iter()
            .any(|(s, e, _)| names_equal(s, source) && names_equal(e, event))
    }

    fn dispatch(
        &self,
        table: &ObjectTable,
        source: &str,
        event: &str,
        args: &mut [Arg],
    ) -> HostResult<()> {
        let matching: Vec<Handler> = self
            .handlers
            .borrow()
            .iter()
            .filter(|(s, e, _)| names_equal(s, source) && names_equal(e, event))
            .map(|(_, _, h)| h.clone())
            .collect();
        for h in matching {
            h(table, args)?;
        }
        Ok(())
    }
}

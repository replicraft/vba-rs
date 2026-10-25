//! Connection points and `WithEvents` (MS-OAUT connection point model,
//! abstracted).
//!
//! An [`EventSource`] is the object's connection-point container for its
//! default source interface. A VBA `WithEvents` variable advises a sink that
//! routes each event to the handler procedure `Var_Event`. Sinks are called
//! in the order they advised. Event arguments are `Arg`s so ByRef parameters
//! such as `Cancel` are written back to the raiser.

use crate::dispatch::Arg;
use crate::error::DispError;
use crate::table::ObjectTable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use vba_core::VbaResult;
use vba_value::{ObjectHandle, ObjectRef, Variant};

/// Connection token (`dwCookie`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cookie(pub u32);

/// Receiver of events.
pub trait EventSink {
    /// `true` if a handler for `event` exists (an absent handler is skipped,
    /// not an error).
    fn handles(&self, event: &str) -> bool;

    /// Delivers an event. ByRef arguments may be modified.
    fn on_event(
        &self,
        table: &ObjectTable,
        event: &str,
        args: &mut [Arg],
    ) -> Result<Variant, DispError>;
}

/// `IConnectionPointContainer` for one source interface.
pub trait EventSource {
    /// The source interface name, e.g. `"CommandButtonEvents"`.
    fn event_interface(&self) -> String;

    /// `Advise`.
    fn advise(&self, sink: Rc<dyn EventSink>) -> Result<Cookie, DispError>;

    /// `Unadvise`; an unknown cookie is `CONNECT_E_NOCONNECTION`
    /// (reported as a generic failure).
    fn unadvise(&self, cookie: Cookie) -> Result<(), DispError>;
}

/// A ready-made connection point that stores sinks and fires events.
pub struct ConnectionPoint {
    interface: String,
    next: Cell<u32>,
    sinks: RefCell<Vec<(Cookie, Rc<dyn EventSink>)>>,
}

impl ConnectionPoint {
    /// A connection point for the named source interface.
    pub fn new(interface: &str) -> Self {
        ConnectionPoint {
            interface: interface.to_string(),
            next: Cell::new(1),
            sinks: RefCell::default(),
        }
    }

    /// Number of connected sinks.
    pub fn sink_count(&self) -> usize {
        self.sinks.borrow().len()
    }

    /// Raises `event` to every sink that handles it, in advise order. The
    /// sink list is snapshotted, so sinks may unadvise while handling. The
    /// first handler error stops delivery and is returned.
    pub fn fire(
        &self,
        table: &ObjectTable,
        event: &str,
        args: &mut [Arg],
    ) -> Result<(), DispError> {
        let snapshot: Vec<Rc<dyn EventSink>> =
            self.sinks.borrow().iter().map(|(_, s)| s.clone()).collect();
        for s in snapshot {
            if s.handles(event) {
                s.on_event(table, event, args)?;
            }
        }
        Ok(())
    }
}

impl EventSource for ConnectionPoint {
    fn event_interface(&self) -> String {
        self.interface.clone()
    }

    fn advise(&self, sink: Rc<dyn EventSink>) -> Result<Cookie, DispError> {
        let c = Cookie(self.next.get());
        self.next.set(c.0 + 1);
        self.sinks.borrow_mut().push((c, sink));
        Ok(c)
    }

    fn unadvise(&self, cookie: Cookie) -> Result<(), DispError> {
        let mut s = self.sinks.borrow_mut();
        let before = s.len();
        s.retain(|(c, _)| *c != cookie);
        if s.len() == before {
            // CONNECT_E_NOCONNECTION
            return Err(DispError::Hresult(0x8004_0200_u32 as i32));
        }
        Ok(())
    }
}

/// A `WithEvents` variable: holds a reference to the source object and its
/// advise cookie, and keeps both consistent across `Set`.
#[derive(Default)]
pub struct WithEventsVar {
    target: Option<(ObjectHandle, Cookie)>,
}

impl WithEventsVar {
    /// An unbound variable (`Nothing`).
    pub fn new() -> Self {
        Self::default()
    }

    /// The bound object, if any.
    pub fn object(&self) -> ObjectRef {
        match self.target {
            Some((h, _)) => ObjectRef::Handle(h),
            None => ObjectRef::Nothing,
        }
    }

    /// `Set var = new`: disconnects from the old source, releases it, then
    /// takes a reference to `new` and advises `sink` on its event source.
    ///
    /// An object that exposes no event source is error 459 ("Object or class
    /// does not support the set of events") and the variable ends up
    /// `Nothing`.
    pub fn set(
        &mut self,
        table: &ObjectTable,
        new: ObjectRef,
        sink: Rc<dyn EventSink>,
    ) -> VbaResult<()> {
        self.clear(table)?;
        let ObjectRef::Handle(h) = new else {
            return Ok(());
        };
        let obj = table.get(h)?;
        let source = obj.event_source().ok_or(vba_core::VbaError::new(459))?;
        let cookie = source
            .advise(sink)
            .map_err(|_| vba_core::VbaError::new(459))?;
        table.add_ref(h)?;
        self.target = Some((h, cookie));
        Ok(())
    }

    /// `Set var = Nothing`: unadvises and releases.
    pub fn clear(&mut self, table: &ObjectTable) -> VbaResult<()> {
        if let Some((h, cookie)) = self.target.take() {
            if let Ok(obj) = table.get(h)
                && let Some(src) = obj.event_source()
            {
                let _ = src.unadvise(cookie);
            }
            table.release(h)?;
        }
        Ok(())
    }
}

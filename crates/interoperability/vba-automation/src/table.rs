//! The object table: identity, reference counts and termination.
//!
//! Handles are never reused, so a stale handle is detected (error 91) rather
//! than aliasing a newer object. Reference counting follows COM: a new object
//! starts with one reference owned by the creator; `Set a = b` adds a
//! reference to `b` before releasing the old target of `a`; when the count
//! reaches zero `Class_Terminate` ([`Dispatch::terminate`]) runs while the
//! object is still addressable, then the object is removed and the references
//! it holds are released in order. Reference cycles are not collected.

use crate::dispatch::{Dispatch, DispatchRef};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use vba_core::{VbaError, VbaResult};
use vba_value::{ObjectHandle, ObjectRef, Variant};

struct Entry {
    obj: DispatchRef,
    refs: u32,
    terminating: bool,
}

/// Table of live automation objects.
#[derive(Default)]
pub struct ObjectTable {
    next: Cell<u64>,
    entries: RefCell<HashMap<u64, Entry>>,
}

impl ObjectTable {
    /// An empty table.
    pub fn new() -> Self {
        ObjectTable {
            next: Cell::new(1),
            entries: RefCell::default(),
        }
    }

    /// Registers an object with one reference owned by the caller.
    pub fn insert(&self, obj: DispatchRef) -> ObjectHandle {
        let id = self.next.get();
        self.next.set(id + 1);
        self.entries.borrow_mut().insert(
            id,
            Entry {
                obj,
                refs: 1,
                terminating: false,
            },
        );
        ObjectHandle(id)
    }

    /// Registers a concrete object (see [`ObjectTable::insert`]).
    pub fn insert_obj<T: Dispatch + 'static>(&self, obj: T) -> ObjectHandle {
        self.insert(Rc::new(obj))
    }

    /// The object behind a handle; error 91 if the handle is stale.
    pub fn get(&self, h: ObjectHandle) -> VbaResult<DispatchRef> {
        self.entries
            .borrow()
            .get(&h.0)
            .map(|e| e.obj.clone())
            .ok_or(VbaError::new(91))
    }

    /// The current reference count, or `None` for a stale handle.
    pub fn ref_count(&self, h: ObjectHandle) -> Option<u32> {
        self.entries.borrow().get(&h.0).map(|e| e.refs)
    }

    /// Number of live objects.
    pub fn live_count(&self) -> usize {
        self.entries.borrow().len()
    }

    /// `true` if the handle refers to a live object.
    pub fn is_live(&self, h: ObjectHandle) -> bool {
        self.entries.borrow().contains_key(&h.0)
    }

    /// `AddRef`. Returns the new count; stale handle is error 91.
    pub fn add_ref(&self, h: ObjectHandle) -> VbaResult<u32> {
        let mut e = self.entries.borrow_mut();
        let entry = e.get_mut(&h.0).ok_or(VbaError::new(91))?;
        entry.refs += 1;
        Ok(entry.refs)
    }

    /// `Release`. Returns the new count. At zero the object is terminated and
    /// removed unless `terminate` took a new reference to it.
    pub fn release(&self, h: ObjectHandle) -> VbaResult<u32> {
        let obj = {
            let mut e = self.entries.borrow_mut();
            let entry = e.get_mut(&h.0).ok_or(VbaError::new(91))?;
            entry.refs = entry.refs.saturating_sub(1);
            if entry.refs > 0 || entry.terminating {
                return Ok(entry.refs);
            }
            entry.terminating = true;
            entry.obj.clone()
        };
        obj.terminate(self);
        let removed = {
            let mut e = self.entries.borrow_mut();
            match e.get_mut(&h.0) {
                Some(entry) if entry.refs == 0 => e.remove(&h.0),
                Some(entry) => {
                    entry.terminating = false;
                    None
                }
                None => None,
            }
        };
        match removed {
            Some(entry) => {
                for held in entry.obj.held_references() {
                    let _ = self.release(held);
                }
                Ok(0)
            }
            None => Ok(self.ref_count(h).unwrap_or(0)),
        }
    }

    /// Adds a reference if `v` holds an object (arrays are traversed).
    pub fn add_ref_value(&self, v: &Variant) {
        match v {
            Variant::Object(ObjectRef::Handle(h)) => {
                let _ = self.add_ref(*h);
            }
            Variant::Array(a) => a.iter().for_each(|x| self.add_ref_value(x)),
            _ => {}
        }
    }

    /// Releases a reference if `v` holds an object (arrays are traversed).
    pub fn release_value(&self, v: &Variant) {
        match v {
            Variant::Object(ObjectRef::Handle(h)) => {
                let _ = self.release(*h);
            }
            Variant::Array(a) => a.iter().for_each(|x| self.release_value(x)),
            _ => {}
        }
    }

    /// `Set slot = new`: takes a reference to `new`, then releases the
    /// previous target. Assigning an object to itself is safe.
    pub fn assign(&self, slot: &mut ObjectRef, new: ObjectRef) -> VbaResult<()> {
        if let ObjectRef::Handle(h) = new {
            self.add_ref(h)?;
        }
        let old = std::mem::replace(slot, new);
        if let ObjectRef::Handle(h) = old {
            self.release(h)?;
        }
        Ok(())
    }

    /// `Set slot = Nothing`.
    pub fn set_nothing(&self, slot: &mut ObjectRef) -> VbaResult<()> {
        self.assign(slot, ObjectRef::Nothing)
    }

    /// The `Is` operator: object identity. `Nothing Is Nothing` is true.
    pub fn is_same(&self, a: ObjectRef, b: ObjectRef) -> bool {
        a == b
    }
}

/// An owned reference that is released on drop.
pub struct Owned<'t> {
    table: &'t ObjectTable,
    handle: ObjectHandle,
}

impl<'t> Owned<'t> {
    /// Adopts a reference the caller already owns.
    pub fn adopt(table: &'t ObjectTable, handle: ObjectHandle) -> Self {
        Owned { table, handle }
    }

    /// Takes a new reference to `handle`.
    pub fn new(table: &'t ObjectTable, handle: ObjectHandle) -> VbaResult<Self> {
        table.add_ref(handle)?;
        Ok(Owned { table, handle })
    }

    /// The handle.
    pub fn handle(&self) -> ObjectHandle {
        self.handle
    }

    /// Gives up ownership without releasing.
    pub fn into_handle(self) -> ObjectHandle {
        let h = self.handle;
        std::mem::forget(self);
        h
    }
}

impl Drop for Owned<'_> {
    fn drop(&mut self) {
        let _ = self.table.release(self.handle);
    }
}

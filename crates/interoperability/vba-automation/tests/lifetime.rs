//! Reference counting, `Set x = Nothing`, terminate ordering and events.

use std::cell::RefCell;
use std::rc::Rc;
use vba_automation::testing::{MemObject, TerminateLog};
use vba_automation::{
    Arg, ConnectionPoint, DispError, EventSink, EventSource, ObjectTable, Owned, WithEventsVar,
};
use vba_value::{ObjectRef, Variant};

fn log() -> TerminateLog {
    Rc::new(RefCell::new(Vec::new()))
}

#[test]
fn set_assign_and_nothing() {
    let t = ObjectTable::new();
    let l = log();
    let h = t.insert_obj(MemObject::new("A").log_terminate(&l)); // refs = 1 (creator)
    let mut x = ObjectRef::Nothing;
    let mut y = ObjectRef::Nothing;
    t.assign(&mut x, ObjectRef::Handle(h)).unwrap(); // Set x = a
    t.assign(&mut y, x).unwrap(); // Set y = x
    assert_eq!(t.ref_count(h), Some(3));
    t.release(h).unwrap(); // creator drops its reference
    t.set_nothing(&mut x).unwrap();
    assert_eq!(t.ref_count(h), Some(1));
    assert!(l.borrow().is_empty());
    assert!(t.is_same(y, ObjectRef::Handle(h)));
    t.set_nothing(&mut y).unwrap();
    assert_eq!(*l.borrow(), ["A"]);
    assert_eq!(t.live_count(), 0);
    assert_eq!(t.ref_count(h), None);
    // Stale handle: error 91, and handles are not reused.
    assert_eq!(t.add_ref(h).unwrap_err().number(), 91);
    let h2 = t.insert_obj(MemObject::new("B"));
    assert_ne!(h, h2);
}

#[test]
fn self_assignment_is_safe() {
    let t = ObjectTable::new();
    let h = t.insert_obj(MemObject::new("A"));
    let mut x = ObjectRef::Handle(h);
    // x already owns the creator's reference.
    t.assign(&mut x, ObjectRef::Handle(h)).unwrap();
    assert_eq!(t.ref_count(h), Some(1));
    t.set_nothing(&mut x).unwrap();
    assert_eq!(t.ref_count(h), None);
}

#[test]
fn terminate_runs_before_held_references_are_released() {
    let t = ObjectTable::new();
    let l = log();
    let child1 = t.insert_obj(MemObject::new("Child1").log_terminate(&l));
    let child2 = t.insert_obj(MemObject::new("Child2").log_terminate(&l));
    let parent = t.insert_obj(
        MemObject::new("Parent")
            .log_terminate(&l)
            .hold(child1)
            .hold(child2),
    );
    assert_eq!(t.live_count(), 3);
    t.release(parent).unwrap();
    assert_eq!(*l.borrow(), ["Parent", "Child1", "Child2"]);
    assert_eq!(t.live_count(), 0);
}

#[test]
fn child_survives_while_externally_referenced() {
    let t = ObjectTable::new();
    let l = log();
    let child = t.insert_obj(MemObject::new("Child").log_terminate(&l));
    let parent = t.insert_obj(MemObject::new("Parent").log_terminate(&l).hold(child));
    t.add_ref(child).unwrap(); // external reference
    t.release(parent).unwrap();
    assert_eq!(*l.borrow(), ["Parent"]);
    assert_eq!(t.ref_count(child), Some(1));
    t.release(child).unwrap();
    assert_eq!(*l.borrow(), ["Parent", "Child"]);
}

#[test]
fn owned_guard_releases_in_reverse_declaration_order() {
    let t = ObjectTable::new();
    let l = log();
    {
        let a = Owned::adopt(&t, t.insert_obj(MemObject::new("A").log_terminate(&l)));
        let b = Owned::adopt(&t, t.insert_obj(MemObject::new("B").log_terminate(&l)));
        assert_ne!(a.handle(), b.handle());
    }
    // Locals are destroyed in reverse order of declaration, like VBA locals
    // going out of scope in unspecified-but-stable order for this model.
    assert_eq!(*l.borrow(), ["B", "A"]);
}

#[test]
fn leaked_cycle_is_not_collected() {
    let t = ObjectTable::new();
    let l = log();
    let a = t.insert_obj(MemObject::new("A").log_terminate(&l));
    let b = t.insert_obj(MemObject::new("B").log_terminate(&l).hold(a));
    // `hold` transfers a's reference to b; a's back reference to b is an
    // extra reference on b.
    t.add_ref(b).unwrap();
    t.release(b).unwrap(); // one reference to b remains
    assert!(l.borrow().is_empty());
    assert_eq!(t.live_count(), 2);
}

#[test]
fn values_with_objects_are_counted() {
    let t = ObjectTable::new();
    let a = t.insert_obj(MemObject::new("A"));
    let v = Variant::Object(ObjectRef::Handle(a));
    t.add_ref_value(&v);
    assert_eq!(t.ref_count(a), Some(2));
    t.release_value(&v);
    t.release_value(&Variant::Long(1)); // no-op
    assert_eq!(t.ref_count(a), Some(1));
}

struct Recorder {
    seen: RefCell<Vec<String>>,
    handle: &'static str,
    cancel: bool,
}

impl EventSink for Recorder {
    fn handles(&self, event: &str) -> bool {
        !event.eq_ignore_ascii_case("Ignored")
    }
    fn on_event(
        &self,
        _: &ObjectTable,
        event: &str,
        args: &mut [Arg],
    ) -> Result<Variant, DispError> {
        self.seen
            .borrow_mut()
            .push(format!("{}:{event}", self.handle));
        if self.cancel {
            args[0].value = Variant::Boolean(true);
        }
        Ok(Variant::Empty)
    }
}

#[test]
fn connection_point_delivery_and_byref_cancel() {
    let t = ObjectTable::new();
    let cp = ConnectionPoint::new("ButtonEvents");
    let r1 = Rc::new(Recorder {
        seen: RefCell::default(),
        handle: "one",
        cancel: false,
    });
    let r2 = Rc::new(Recorder {
        seen: RefCell::default(),
        handle: "two",
        cancel: true,
    });
    let c1 = cp.advise(r1.clone()).unwrap();
    let _c2 = cp.advise(r2.clone()).unwrap();
    let mut args = [Arg::by_ref(false)];
    cp.fire(&t, "Click", &mut args).unwrap();
    cp.fire(&t, "ignored", &mut args).unwrap();
    assert_eq!(args[0].value, Variant::Boolean(true));
    assert_eq!(*r1.seen.borrow(), ["one:Click"]);
    cp.unadvise(c1).unwrap();
    assert!(cp.unadvise(c1).is_err());
    assert_eq!(cp.sink_count(), 1);
    assert_eq!(cp.event_interface(), "ButtonEvents");
}

#[test]
fn with_events_set_and_nothing() {
    let t = ObjectTable::new();
    let button = Rc::new(MemObject::new("Button"));
    let src = t.insert(button.clone());
    let plain = t.insert_obj(MemObject::new("X"));
    let sink = Rc::new(Recorder {
        seen: RefCell::default(),
        handle: "w",
        cancel: false,
    });
    let mut var = WithEventsVar::new();
    var.set(&t, ObjectRef::Handle(src), sink.clone()).unwrap();
    assert_eq!(t.ref_count(src), Some(2));
    let mut args = [Arg::by_ref(false)];
    button.events().fire(&t, "Click", &mut args).unwrap();
    assert_eq!(*sink.seen.borrow(), ["w:Click"]);
    // Re-binding to another object disconnects and releases the first.
    var.set(&t, ObjectRef::Handle(plain), sink.clone()).unwrap();
    assert_eq!(t.ref_count(src), Some(1));
    button.events().fire(&t, "Click", &mut args).unwrap();
    assert_eq!(sink.seen.borrow().len(), 1); // disconnected from the old source
    var.clear(&t).unwrap();
    assert_eq!(var.object(), ObjectRef::Nothing);
    assert_eq!(t.ref_count(plain), Some(1));
}

#[test]
fn raising_events_from_an_object() {
    let t = ObjectTable::new();
    let o = MemObject::new("Button");
    let sink = Rc::new(Recorder {
        seen: RefCell::default(),
        handle: "s",
        cancel: true,
    });
    o.events().advise(sink.clone()).unwrap();
    let mut args = [Arg::by_ref(false)];
    o.events().fire(&t, "BeforeClick", &mut args).unwrap();
    assert_eq!(*sink.seen.borrow(), ["s:BeforeClick"]);
    assert_eq!(args[0].value, Variant::Boolean(true));
}

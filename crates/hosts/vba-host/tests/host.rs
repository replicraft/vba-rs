//! Permission, limit, headless/scripted host and event tests.

use proptest::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use vba_automation::testing::MemObject;
use vba_automation::{Arg, ErrInfo};
use vba_host::{
    Budget, Capability, DeclareRequest, EventDispatch, EventRouter, HeadlessHost, Host,
    InputBoxRequest, LimitExceeded, MsgBoxRequest, Policy, ResourceLimits, ScopeLimits,
    ScriptedHost,
};
use vba_value::{ObjectRef, Variant};

fn decl() -> DeclareRequest {
    DeclareRequest {
        library: "user32".into(),
        procedure: "MessageBeep".into(),
        args: vec![],
        return_type: None,
    }
}

fn num<T>(r: Result<T, ErrInfo>) -> i32 {
    match r {
        Ok(_) => 0,
        Err(e) => e.number,
    }
}

#[test]
fn default_policy_denies_every_entry_point() {
    let h = ScriptedHost::new()
        .with_factory("Any.Thing", |_| Ok(Variant::Long(1)))
        .with_named_object("c:/f.txt", |_| Ok(Variant::Long(1)))
        .with_declare("user32", "MessageBeep", |_| Ok(Variant::Long(1)));
    assert_eq!(num(h.create_object("Any.Thing", None)), 70);
    assert_eq!(num(h.get_object(Some("c:/f.txt"), None)), 70);
    assert_eq!(num(h.get_object(None, Some("Any.Thing"))), 70);
    assert_eq!(num(h.call_declare(&mut decl())), 70);
    for cap in Capability::ALL {
        assert_eq!(num(h.check(cap, "x")), 70, "{cap:?}");
    }
    // The hooks were never reached.
    assert!(h.log().is_empty());
    // Denial text.
    let e = h.create_object("Any.Thing", None).unwrap_err();
    assert_eq!(e.description, "Permission denied");
    assert!(e.source.contains("COM object creation"));
    assert!(e.source.contains("Any.Thing"));
}

#[test]
fn grants_enable_exactly_their_entry_point() {
    let mk = |p: Policy| {
        ScriptedHost::new()
            .with_policy(p)
            .with_factory("Any.Thing", |_| Ok(Variant::Long(1)))
            .with_named_object("c:/f.txt", |_| Ok(Variant::Long(2)))
            .with_declare("user32", "MessageBeep", |_| Ok(Variant::Long(3)))
    };
    let h = mk(Policy::deny_all().grant(Capability::ComCreate));
    assert_eq!(
        h.create_object("any.thing", None).unwrap(),
        Variant::Long(1)
    );
    assert_eq!(num(h.get_object(Some("c:/f.txt"), None)), 70);
    assert_eq!(num(h.call_declare(&mut decl())), 70);

    let h = mk(Policy::deny_all().grant(Capability::FileRead));
    assert_eq!(
        h.get_object(Some("c:/f.txt"), None).unwrap(),
        Variant::Long(2)
    );
    assert_eq!(num(h.create_object("Any.Thing", None)), 70);

    let h = mk(Policy::deny_all().grant(Capability::NativeDeclare));
    assert_eq!(h.call_declare(&mut decl()).unwrap(), Variant::Long(3));
    assert_eq!(h.log(), ["Declare:user32!MessageBeep"]);

    // A remote server also needs Network.
    let h = mk(Policy::deny_all().grant(Capability::ComCreate));
    assert_eq!(num(h.create_object("Any.Thing", Some("remote-box"))), 70);
    let h = mk(Policy::allow_all());
    assert!(h.create_object("Any.Thing", Some("remote-box")).is_ok());
}

#[test]
fn scoped_grants() {
    let p = Policy::deny_all()
        .grant_prefixes(Capability::FileRead, ["/data/", "C:\\Work\\"])
        .grant_prefixes(Capability::ComCreate, ["Scripting."]);
    assert!(p.check(Capability::FileRead, "/data/a.txt").is_ok());
    assert!(p.check(Capability::FileRead, "c:\\work\\x.csv").is_ok());
    assert!(p.check(Capability::FileRead, "/etc/passwd").is_err());
    assert!(
        p.check(Capability::FileRead, "/data/../etc/passwd")
            .is_err()
    );
    assert!(
        p.check(Capability::FileRead, "C:\\Work\\..\\Secret")
            .is_err()
    );
    assert!(
        p.check(Capability::ComCreate, "Scripting.FileSystemObject")
            .is_ok()
    );
    assert!(p.check(Capability::ComCreate, "WScript.Shell").is_err());
    assert!(p.check(Capability::FileWrite, "/data/a.txt").is_err());
    // grant() later widens; revoke removes.
    let p = p.grant(Capability::FileRead);
    assert!(p.check(Capability::FileRead, "/etc/passwd").is_ok());
    assert!(
        !p.revoke(Capability::FileRead)
            .is_granted(Capability::FileRead)
    );
}

#[test]
fn file_prefix_grants_match_whole_path_segments() {
    let p = Policy::deny_all().grant_prefixes(Capability::FileRead, ["/data", "C:\\Work"]);
    assert!(p.check(Capability::FileRead, "/data").is_ok());
    assert!(p.check(Capability::FileRead, "/data/").is_ok());
    assert!(p.check(Capability::FileRead, "/DATA/a.txt").is_ok());
    assert!(p.check(Capability::FileRead, "c:/work/a.txt").is_ok());
    assert!(p.check(Capability::FileRead, "c:\\work\\a.txt").is_ok());
    assert!(p.check(Capability::FileRead, "/data-secret/a.txt").is_err());
    assert!(p.check(Capability::FileRead, "/data2").is_err());
    assert!(p.check(Capability::FileRead, "c:/workshop/a.txt").is_err());
    // The other capabilities have their own scopes (see `scopes.rs`).
    let q = Policy::deny_all().grant_prefixes(Capability::ProcessSpawn, ["notepad"]);
    assert!(q.check(Capability::ProcessSpawn, "notepad.exe x").is_ok());
    assert!(q.check(Capability::ProcessSpawn, "notepad-x.exe").is_err());
}

#[test]
fn headless_host_fails_explicitly() {
    let h = HeadlessHost::new().with_policy(Policy::allow_all());
    assert_eq!(num(h.create_object("X.Y", None)), 429);
    assert_eq!(num(h.get_object(Some("a"), None)), 432);
    assert_eq!(num(h.call_declare(&mut decl())), 48);
    assert_eq!(num(h.get_object(None, None)), 5);
    let req = MsgBoxRequest {
        prompt: "hi".into(),
        buttons: 0,
        title: "t".into(),
    };
    let e = h.ui().msg_box(&req).unwrap_err();
    assert_eq!(e.number, 70);
    assert!(e.description.contains("MsgBox"));
    let req = InputBoxRequest {
        prompt: "?".into(),
        title: "t".into(),
        default: "".into(),
    };
    assert_eq!(h.ui().input_box(&req).unwrap_err().number, 70);
    assert_eq!(h.lookup_global("Application").unwrap(), None);
    assert!(h.metadata().name.contains("headless"));
    assert_eq!(h.metadata().profile, *vba_profile::default_profile());
}

#[test]
fn globals_resolve_case_insensitively_with_ref_counts() {
    let t_obj;
    let h = {
        let h = HeadlessHost::new();
        t_obj = h.objects().insert_obj(MemObject::new("App"));
        h.with_global("Application", Variant::Object(ObjectRef::Handle(t_obj)))
    };
    let v = h.resolve_global("APPLICATION").unwrap().unwrap();
    assert_eq!(v, Variant::Object(ObjectRef::Handle(t_obj)));
    assert_eq!(h.objects().ref_count(t_obj), Some(2)); // host + caller
    h.objects().release_value(&v);
    assert_eq!(h.resolve_global("Nope").unwrap(), None);
}

#[test]
fn scripted_factories_return_objects() {
    let h = ScriptedHost::new()
        .with_policy(Policy::deny_all().grant(Capability::ComCreate))
        .with_factory("Lib.Thing", |t| {
            Ok(Variant::Object(ObjectRef::Handle(
                t.insert_obj(MemObject::new("Thing")),
            )))
        });
    let v = h.create_object("lib.thing", None).unwrap();
    let Variant::Object(ObjectRef::Handle(o)) = v else {
        panic!()
    };
    assert_eq!(h.objects().get(o).unwrap().class_name(), "Thing");
    assert_eq!(num(h.create_object("Lib.Other", None)), 429);
    assert_eq!(
        h.log(),
        ["CreateObject:lib.thing", "CreateObject:Lib.Other"]
    );
}

#[test]
fn scripted_ui_answers_in_order_and_then_fails() {
    let h = ScriptedHost::new()
        .with_msg_box_answers([6, 7])
        .with_input_box_answers([Some("x".to_string()), None]);
    let m = MsgBoxRequest {
        prompt: "a".into(),
        buttons: 4,
        title: "".into(),
    };
    let i = InputBoxRequest {
        prompt: "b".into(),
        title: "".into(),
        default: "".into(),
    };
    assert_eq!(h.ui().msg_box(&m).unwrap(), 6);
    assert_eq!(h.ui().msg_box(&m).unwrap(), 7);
    assert_eq!(h.ui().msg_box(&m).unwrap_err().number, 5);
    assert_eq!(h.ui().input_box(&i).unwrap(), Some("x".into()));
    assert_eq!(h.ui().input_box(&i).unwrap(), None);
    assert!(h.ui().input_box(&i).is_err());
    assert_eq!(h.scripted_ui().prompts.borrow().len(), 6);
}

#[test]
fn do_events_is_a_cancellable_safe_point_and_costs_a_step() {
    let h = ScriptedHost::new().cancel_after_idles(3);
    assert_eq!(h.do_events().unwrap(), 0);
    assert_eq!(h.do_events().unwrap(), 0);
    assert_eq!(h.do_events().unwrap_err().number, 18);
    assert_eq!(h.budget().steps_used(), 3);

    let h = HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(2),
        ..ResourceLimits::default()
    });
    assert!(h.do_events().is_ok());
    assert!(h.do_events().is_ok());
    assert_eq!(h.do_events().unwrap_err().number, 18);
    assert_eq!(h.limits().max_steps, Some(2));
}

#[test]
fn a_scope_narrows_the_limits_and_restores_them() {
    let b = Budget::new(ResourceLimits {
        max_steps: Some(100),
        max_memory_bytes: Some(1000),
        max_call_depth: 10,
    });
    b.charge_steps(40).unwrap();
    b.alloc(100).unwrap();
    let outer = b.enter_scope(ScopeLimits {
        max_steps: Some(10),
        max_memory_bytes: Some(50),
        ..ScopeLimits::default()
    });
    // A wider inner scope cannot widen the outer one.
    let inner = b.enter_scope(ScopeLimits {
        max_steps: Some(1000),
        ..ScopeLimits::default()
    });
    assert!(b.charge_steps(10).is_ok());
    assert_eq!(b.charge_step(), Err(LimitExceeded::Steps));
    b.leave_scope(inner);
    assert_eq!(b.charge_step(), Err(LimitExceeded::Steps));
    assert_eq!(b.alloc(60), Err(LimitExceeded::Memory));
    assert!(b.alloc(50).is_ok());
    b.leave_scope(outer);
    // Back to the construction limits: 40 + 12 steps so far, 150 bytes.
    assert!(b.charge_steps(40).is_ok());
    assert!(b.alloc(500).is_ok());
    assert!(!b.deadline_passed());
    let t = b.enter_scope(ScopeLimits {
        timeout: Some(std::time::Duration::ZERO),
        ..ScopeLimits::default()
    });
    assert!(b.deadline_passed());
    b.leave_scope(t);
    assert!(!b.deadline_passed());
}

#[test]
fn budget_limits() {
    let b = Budget::new(ResourceLimits {
        max_steps: Some(10),
        max_memory_bytes: Some(100),
        max_call_depth: 2,
    });
    assert!(b.charge_steps(10).is_ok());
    assert_eq!(b.charge_step(), Err(LimitExceeded::Steps));
    assert!(b.enter_call().is_ok());
    assert!(b.enter_call().is_ok());
    assert_eq!(b.enter_call(), Err(LimitExceeded::CallDepth));
    assert_eq!(b.depth(), 2);
    b.leave_call();
    assert!(b.enter_call().is_ok());
    assert!(b.alloc(60).is_ok());
    assert_eq!(b.alloc(60), Err(LimitExceeded::Memory));
    assert_eq!(b.memory_used(), 60);
    b.free(60);
    assert!(b.alloc(100).is_ok());
    assert_eq!(LimitExceeded::CallDepth.into_err_info().number, 28);
    assert_eq!(LimitExceeded::Memory.into_err_info().number, 7);
    assert_eq!(LimitExceeded::Steps.into_err_info().number, 18);
    // Unlimited by default except for call depth.
    let d = ResourceLimits::default();
    assert_eq!((d.max_steps, d.max_memory_bytes), (None, None));
}

#[test]
fn event_router_dispatch() {
    let r = EventRouter::new();
    let t = vba_automation::ObjectTable::new();
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    r.register("Form1", "BeforeUpdate", move |_, args| {
        c.set(c.get() + 1);
        args[0].value = Variant::Boolean(true); // Cancel = True
        Ok(())
    });
    let c = count.clone();
    r.register("form1", "beforeupdate", move |_, _| {
        c.set(c.get() + 10);
        Ok(())
    });
    r.register("Form1", "Fail", |_, _| Err(ErrInfo::standard(5)));
    assert!(r.has_handler("FORM1", "BEFOREUPDATE"));
    assert!(!r.has_handler("Form2", "BeforeUpdate"));
    let mut args = [Arg::by_ref(false)];
    r.dispatch(&t, "Form1", "BeforeUpdate", &mut args).unwrap();
    assert_eq!(count.get(), 11);
    assert_eq!(args[0].value, Variant::Boolean(true));
    // No handler is not an error; handler errors propagate.
    assert!(r.dispatch(&t, "Form9", "Load", &mut []).is_ok());
    assert_eq!(
        r.dispatch(&t, "Form1", "Fail", &mut []).unwrap_err().number,
        5
    );
    r.remove_source("FORM1");
    assert!(!r.has_handler("Form1", "BeforeUpdate"));
}

proptest! {
    #[test]
    fn deny_all_denies_everything(res in ".{0,40}", i in 0usize..7) {
        let cap = Capability::ALL[i];
        let e = Policy::deny_all().check(cap, &res).unwrap_err();
        prop_assert_eq!(e.into_err_info().number, 70);
    }

    #[test]
    fn prefix_grants_are_case_insensitive_and_reject_parent_segments(
        prefix in "[a-z]{1,8}/",
        rest in "[a-z]{0,8}",
        mask in proptest::collection::vec(any::<bool>(), 1..16),
    ) {
        let p = Policy::deny_all().grant_prefixes(Capability::FileRead, [prefix.clone()]);
        let res: String = format!("{prefix}{rest}")
            .chars()
            .zip(mask.iter().cycle())
            .map(|(c, up)| if *up { c.to_ascii_uppercase() } else { c })
            .collect();
        prop_assert!(p.check(Capability::FileRead, &res).is_ok());
        let escaped = format!("{prefix}../{rest}");
        prop_assert!(p.check(Capability::FileRead, &escaped).is_err());
    }

    #[test]
    fn budget_never_exceeds_limits(max in 0u64..50, charges in proptest::collection::vec(0u64..10, 0..30)) {
        let b = Budget::new(ResourceLimits { max_steps: Some(max), ..ResourceLimits::default() });
        let mut spent = 0u64;
        for c in charges {
            spent += c;
            let r = b.charge_steps(c);
            prop_assert_eq!(r.is_ok(), spent <= max);
        }
    }
}

#[test]
fn ui_automation_and_clock_are_refused_by_default() {
    use vba_host::ClockPart;
    let h = HeadlessHost::new();
    assert_eq!(num(h.send_keys("abc", false)), 70);
    assert_eq!(num(h.app_activate("Notepad", false)), 70);
    // Granting the capability still does not send input: the default hook
    // has no UI to drive.
    let g = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::UiAutomation));
    let e = g.send_keys("abc", true).unwrap_err();
    assert_eq!(e.number, 70);
    assert!(e.description.contains("no UI"));
    assert_eq!(num(g.app_activate("Notepad", true)), 70);
    // The system clock is never set by default, whatever is granted.
    let all = HeadlessHost::new().with_policy(Policy::allow_all());
    let d = vba_value::Date::from_ymd_hms(2001, 2, 3, 0, 0, 0).unwrap();
    assert_eq!(num(all.set_clock(ClockPart::Date, d)), 70);
    assert_eq!(num(all.set_clock(ClockPart::Time, d)), 70);
    assert!(all.settings().is_none());
}

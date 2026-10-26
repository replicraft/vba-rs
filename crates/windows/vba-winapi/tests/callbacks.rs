//! The timer pump and the window list.

#![allow(missing_docs)]

use vba_profile::Target;
use vba_winapi::{
    CallContext, CallbackError, CallbackHost, CallbackSig, Identity, ManualClock, Window,
    WindowList, dispatch_timers,
};

struct Log(Vec<(i64, Vec<i64>)>, bool);

impl CallbackHost for Log {
    fn validate(&mut self, _: i64, _: &CallbackSig) -> Result<(), CallbackError> {
        Ok(())
    }
    fn invoke(
        &mut self,
        _: &mut CallContext,
        token: i64,
        sig: &CallbackSig,
        args: &[i64],
    ) -> Result<i64, CallbackError> {
        assert_eq!(sig.name, "TIMERPROC");
        self.0.push((token, args.to_vec()));
        if self.1 {
            Err(CallbackError::Aborted)
        } else {
            Ok(0)
        }
    }
}

fn ctx(clock: &ManualClock) -> CallContext {
    CallContext::new(
        Target::WIN64,
        Box::new(clock.clone()),
        Identity::new("u", "C").unwrap(),
    )
}

#[test]
fn timers_fire_when_due_once_per_pump_and_reschedule() {
    let clock = ManualClock::at(0);
    let mut c = ctx(&clock);
    c.timers.set(0, 1, 100, 0, 11);
    c.timers.set(0, 2, 250, 0, 22);
    let mut log = Log(Vec::new(), false);
    assert_eq!(dispatch_timers(&mut c, &mut log), Ok(0));
    clock.advance(260);
    assert_eq!(dispatch_timers(&mut c, &mut log), Ok(2));
    // Late by 2.6 intervals, delivered once; rescheduled from now.
    assert_eq!(dispatch_timers(&mut c, &mut log), Ok(0));
    assert_eq!(log.0[0].0, 11);
    assert_eq!(log.0[0].1, vec![0, 0x113, 1, 260]);
    assert_eq!(log.0[1].0, 22);
    clock.advance(100);
    assert_eq!(dispatch_timers(&mut c, &mut log), Ok(1));
}

#[test]
fn an_aborted_timer_callback_stops_the_pump() {
    let clock = ManualClock::at(0);
    let mut c = ctx(&clock);
    c.timers.set(0, 1, 10, 0, 5);
    c.timers.set(0, 2, 10, 0, 6);
    clock.advance(10);
    let mut log = Log(Vec::new(), true);
    assert_eq!(
        dispatch_timers(&mut c, &mut log),
        Err(CallbackError::Aborted)
    );
    assert_eq!(log.0.len(), 1);
}

#[test]
fn window_list_rejects_duplicates_and_cycles_terminate() {
    let mut w = WindowList::default();
    assert!(w.add(Window {
        hwnd: 1,
        parent: 0,
        thread: 1
    }));
    assert!(!w.add(Window {
        hwnd: 1,
        parent: 0,
        thread: 1
    }));
    assert!(!w.add(Window {
        hwnd: 0,
        parent: 0,
        thread: 1
    }));
    // A cycle (2 -> 3 -> 2) cannot loop the enumeration.
    assert!(w.add(Window {
        hwnd: 2,
        parent: 3,
        thread: 1
    }));
    assert!(w.add(Window {
        hwnd: 3,
        parent: 2,
        thread: 1
    }));
    assert_eq!(w.descendants(2), vec![3]);
    assert_eq!(w.top_level(), vec![1]);
}

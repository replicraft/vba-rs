//! The deadline clock can be stopped while the host waits for a person.

use std::time::Duration;

use vba_host::{Budget, ResourceLimits, ScopeLimits};

#[test]
fn suspended_time_does_not_burn_the_timeout() {
    let b = Budget::new(ResourceLimits::unlimited());
    let t = b.enter_scope(ScopeLimits::new().with_timeout(Some(Duration::from_millis(150))));
    {
        let _stopped = b.suspend_deadline();
        std::thread::sleep(Duration::from_millis(300));
    }
    assert!(!b.deadline_passed(), "the paused wait was charged");
    assert!(b.time_left().unwrap() > Duration::from_millis(50));
    std::thread::sleep(Duration::from_millis(200));
    assert!(b.deadline_passed(), "running time must still count");
    b.leave_scope(t);
}

#[test]
fn an_outer_deadline_restored_by_leave_scope_keeps_the_credit() {
    let b = Budget::new(ResourceLimits::unlimited());
    let outer = b.enter_scope(ScopeLimits::new().with_timeout(Some(Duration::from_millis(150))));
    let inner = b.enter_scope(ScopeLimits::new().with_timeout(Some(Duration::from_secs(60))));
    {
        let _stopped = b.suspend_deadline();
        std::thread::sleep(Duration::from_millis(300));
    }
    b.leave_scope(inner);
    assert!(!b.deadline_passed());
    b.leave_scope(outer);
}

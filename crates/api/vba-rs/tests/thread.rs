//! `vba::thread::InstanceThread`: ordering, cancellation, panics, closing.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vba::thread::{InstanceThread, ThreadError};
use vba::{Engine, Error, Project, RunError, Variant};

const SOURCE: &str = "\
Public Counter As Long
Public Function Bump() As Long
  Counter = Counter + 1
  Bump = Counter
End Function
Public Sub Spin()
  Do
  Loop
End Sub
";

fn worker() -> InstanceThread {
    InstanceThread::spawn(|| {
        let project = Project::from_source("Module1", SOURCE);
        let mut instance = Engine::new().load(&project)?;
        instance.trust();
        Ok(instance)
    })
    .unwrap()
}

fn bump(w: &InstanceThread) -> i32 {
    match w.call(|i| i.run("Bump", vec![])).unwrap().unwrap().value {
        Variant::Long(n) => n,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn handle_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    check::<InstanceThread>();
    check::<ThreadError>();
}

#[test]
fn commands_run_in_order_and_share_state() {
    let w = worker();
    assert_eq!(bump(&w), 1);
    assert_eq!(bump(&w), 2);
    // Submitted without waiting: still executed in submission order.
    let replies: Vec<_> = (0..5)
        .map(|_| w.submit(|i| i.run("Bump", vec![]).unwrap().value).unwrap())
        .collect();
    let values: Vec<_> = replies.into_iter().map(|r| r.wait().unwrap()).collect();
    assert_eq!(
        values,
        (3..=7).map(Variant::Long).collect::<Vec<_>>(),
        "queue order"
    );
    w.close().unwrap();
}

#[test]
fn many_threads_share_one_interpreter() {
    let w = Arc::new(worker());
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let w = w.clone();
            std::thread::spawn(move || (0..25).map(|_| bump(&w)).collect::<Vec<_>>())
        })
        .collect();
    let mut all: Vec<i32> = handles
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect();
    all.sort_unstable();
    // Each command saw a distinct value: nothing ran concurrently.
    assert_eq!(all, (1..=100).collect::<Vec<_>>());
}

#[test]
fn cancelling_from_another_thread_stops_a_running_command() {
    let w = worker();
    let token = w.cancel_token();
    let reply = w.submit(|i| i.run("Spin", vec![])).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    token.cancel();
    let started = Instant::now();
    let err = reply.wait().unwrap().unwrap_err();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert!(started.elapsed() < Duration::from_secs(10));
    token.reset();
    assert_eq!(bump(&w), 1);
}

#[test]
fn a_failed_build_is_reported_by_spawn() {
    let err = InstanceThread::spawn(|| {
        let project = Project::from_source("Module1", "Public Sub A(\n");
        Engine::new().load(&project)
    })
    .unwrap_err();
    assert!(matches!(err, ThreadError::Build(_)), "{err:?}");
    assert!(std::error::Error::source(&err).is_some());
}

#[test]
fn a_panicking_builder_is_reported_by_spawn() {
    let err = InstanceThread::spawn(|| panic!("builder boom")).unwrap_err();
    match err {
        ThreadError::Panicked(m) => assert_eq!(m, "builder boom"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_panicking_command_stops_the_thread() {
    let w = worker();
    assert_eq!(bump(&w), 1);
    let err = w.call(|_| -> () { panic!("command boom") }).unwrap_err();
    assert!(matches!(&err, ThreadError::Panicked(m) if m == "command boom"));
    assert!(matches!(w.call(|_| 1), Err(ThreadError::Stopped)));
    assert!(matches!(w.submit(|_| 1), Err(ThreadError::Stopped)));
    w.close().unwrap();
}

#[test]
fn closing_skips_queued_commands() {
    let w = worker();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let slow = w
        .submit(move |i| {
            started_tx.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(100));
            i.run("Bump", vec![]).unwrap().value
        })
        .unwrap();
    started_rx.recv().unwrap(); // the first command is running
    let queued = w.submit(|i| i.run("Bump", vec![]).unwrap().value).unwrap();
    w.close().unwrap();
    assert_eq!(slow.wait().unwrap(), Variant::Long(1));
    assert!(matches!(queued.wait(), Err(ThreadError::Stopped)));
}

#[test]
fn try_wait_does_not_block() {
    let w = worker();
    let mut reply = w
        .submit(|_| std::thread::sleep(Duration::from_millis(100)))
        .unwrap();
    assert!(reply.try_wait().is_none());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(r) = reply.try_wait() {
            r.unwrap();
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(reply.try_wait(), Some(Err(ThreadError::Stopped))));
}

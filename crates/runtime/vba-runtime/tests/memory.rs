//! `max_memory_bytes` also covers what lives outside the interpreter's
//! heap: the contents of an in-memory file system and
//! `Scripting.Dictionary` items.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{Capability, HeadlessHost, Host, Policy, ResourceLimits};
use vba_runtime::*;
use vba_vm::LimitKind;

fn limited_host(max_memory: u64, policy: Policy) -> Rc<HeadlessHost> {
    Rc::new(
        HeadlessHost::new().with_policy(policy).with_limits(
            ResourceLimits::default()
                .with_max_steps(Some(50_000_000))
                .with_max_memory_bytes(Some(max_memory)),
        ),
    )
}

fn session(src: &str, host: Rc<HeadlessHost>) -> Session {
    let mut s = Session::builder()
        .host(host)
        .vfs(MemFs::new())
        .build_sources(&[std_module(src)])
        .unwrap();
    s.trust();
    s
}

const WRITE_FILES: &str = "Sub Main()\nDim i As Long\nFor i = 1 To 30\nOpen \"/f\" & i For Output As #1\nPrint #1, String$(1000000, \"x\")\nClose #1\nNext\nDebug.Print \"done\"\nEnd Sub\n";

#[test]
fn files_kept_in_the_memory_file_system_are_charged() {
    // 30 files of 1 MB under 10 MB (the shape of 30 x 20 MB under 100 MB).
    let mut s = session(WRITE_FILES, limited_host(10_000_000, all_files()));
    let r = s.run_procedure("Main", vec![]);
    assert!(
        matches!(r, Err(Error::Limit(LimitKind::Memory))),
        "{:?}",
        r.map(|o| o.status)
    );
}

#[test]
fn the_memory_file_system_gives_memory_back_when_files_are_deleted() {
    let host = limited_host(10_000_000, all_files());
    let mut s = session(
        "Sub Main()\nDim i As Long\nFor i = 1 To 30\nOpen \"/f\" For Output As #1\nPrint #1, String$(1000000, \"x\")\nClose #1\nKill \"/f\"\nNext\nEnd Sub\n",
        host.clone(),
    );
    s.run_procedure("Main", vec![]).unwrap();
    assert!(
        host.budget().memory_used() < 100_000,
        "left behind: {}",
        host.budget().memory_used()
    );
}

#[test]
fn files_fit_when_they_fit() {
    let host = limited_host(10_000_000, all_files());
    let mut s = session(
        "Sub Main()\nDim i As Long\nFor i = 1 To 3\nOpen \"/f\" & i For Output As #1\nPrint #1, String$(1000000, \"x\")\nClose #1\nNext\nEnd Sub\n",
        host.clone(),
    );
    s.run_procedure("Main", vec![]).unwrap();
    let used = host.budget().memory_used();
    assert!((3_000_000..4_000_000).contains(&used), "used: {used}");
}

#[test]
fn dictionary_items_are_charged() {
    // 100000 items under 1 MB (the shape of 3 million under 1 MB).
    let host = limited_host(
        1_000_000,
        Policy::deny_all().grant_prefixes(Capability::ComCreate, ["Scripting.Dictionary"]),
    );
    let mut s = session(
        "Sub Main()\nDim d As Object, i As Long\nSet d = CreateObject(\"Scripting.Dictionary\")\nFor i = 1 To 100000\nd.Add i, i\nNext\nDebug.Print d.Count\nEnd Sub\n",
        host,
    );
    let r = s.run_procedure("Main", vec![]);
    assert!(
        matches!(r, Err(Error::Limit(LimitKind::Memory))),
        "{:?}",
        r.map(|o| o.status)
    );
}

#[test]
fn dictionary_strings_are_charged_and_returned() {
    let host = limited_host(
        10_000_000,
        Policy::deny_all().grant_prefixes(Capability::ComCreate, ["Scripting.Dictionary"]),
    );
    let mut s = session(
        "Sub Main()\nDim d As Object, i As Long\nSet d = CreateObject(\"Scripting.Dictionary\")\nFor i = 1 To 60\nd.Add i, String$(1000000, \"x\")\nNext\nEnd Sub\n",
        host.clone(),
    );
    assert!(matches!(
        s.run_procedure("Main", vec![]),
        Err(Error::Limit(LimitKind::Memory))
    ));
    // The failed run released its frames; a dictionary that was dropped
    // returns what it held.
    let mut s = session(
        "Sub Main()\nDim d As Object, i As Long\nSet d = CreateObject(\"Scripting.Dictionary\")\nFor i = 1 To 5\nd.Add i, String$(100000, \"x\")\nNext\nd.RemoveAll\nEnd Sub\n",
        host.clone(),
    );
    let before = host.budget().memory_used();
    s.run_procedure("Main", vec![]).unwrap();
    assert!(
        host.budget().memory_used() <= before + 10_000,
        "before {before}, after {}",
        host.budget().memory_used()
    );
}

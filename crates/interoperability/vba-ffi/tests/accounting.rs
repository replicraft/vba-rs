//! Emulated native memory and handles count against the run's memory meter.

use vba_ffi::*;

#[test]
fn emulated_memory_and_handles_are_charged_to_the_run_meter() {
    let meter = vba_core::MemoryMeter::new(Some(10_000));
    let _scope = meter.enter();
    let mut m = Memory::new(vba_profile::Bitness::B64);
    let a = m.alloc(4096).unwrap();
    assert!(meter.used() >= 4096);
    let mut t = HandleTable::new();
    let h = t.mint(ProviderId("test"), "file");
    let with_handle = meter.used();
    assert!(with_handle > 4096);
    t.close(ProviderId("test"), &h).unwrap();
    assert!(meter.used() < with_handle);
    m.free(a).unwrap();
    assert_eq!(meter.used(), 0);
    m.alloc(20_000).unwrap();
    assert!(
        meter.exceeded(),
        "the interpreter stops the run at its next step"
    );
}

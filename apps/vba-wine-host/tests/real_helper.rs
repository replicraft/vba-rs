//! The client against the real `vba-wine-host` binary, launched natively.
//!
//! On non-Windows hosts the helper runs in protocol-only mode, so these tests
//! exercise the real process loop, framing, timeouts and crash handling, but
//! never a real library load (that path is cfg(windows) and unverified).

use std::time::{Duration, Instant};
use vba_wine::protocol::{CAP_NATIVE_CALLS, PROTOCOL_VERSION, ProcRef, RetKind, WireConv};
use vba_wine::{Session, SessionOptions, WineConfig, WineError};

fn helper() -> &'static str {
    env!("CARGO_BIN_EXE_vba-wine-host")
}

fn opts(ms: u64) -> SessionOptions {
    SessionOptions {
        timeout: Duration::from_millis(ms),
        ..Default::default()
    }
}

fn spawn(args: &[&str], ms: u64) -> Result<Session, WineError> {
    Session::spawn(&WineConfig::direct(helper()).with_args(args), opts(ms))
}

#[test]
fn hello_and_ping() {
    let mut s = spawn(&[], 5000).unwrap();
    assert_eq!(s.info().version, PROTOCOL_VERSION);
    assert!(matches!(s.info().bitness, 32 | 64));
    if cfg!(not(windows)) {
        assert_eq!(s.info().caps & CAP_NATIVE_CALLS, 0);
        assert_eq!(s.info().os, std::env::consts::OS);
    }
    s.ping().unwrap();
    s.shutdown().unwrap();
}

#[cfg(not(windows))]
#[test]
fn protocol_only_answers_unsupported_everywhere() {
    let mut s = spawn(&[], 5000).unwrap();
    let e = s.load_library("kernel32").unwrap_err();
    assert!(matches!(e, WineError::Unsupported(_)), "{e}");
    // Not fatal: the loop keeps serving.
    s.ping().unwrap();
    assert!(!s.is_closed());
}

#[test]
fn forced_protocol_only_flag() {
    let mut s = spawn(&["--protocol-only"], 5000).unwrap();
    assert_eq!(s.info().caps & CAP_NATIVE_CALLS, 0);
    assert!(matches!(
        s.load_library("kernel32"),
        Err(WineError::Unsupported(_))
    ));
}

#[test]
fn version_mismatch_with_real_binary() {
    let e = spawn(&["--advertise-version=42"], 5000).err().unwrap();
    assert_eq!(
        e,
        WineError::VersionMismatch {
            client: PROTOCOL_VERSION,
            helper: 42
        }
    );
}

#[test]
fn helper_crash_is_reported() {
    let mut s = spawn(&["--inject=exit"], 5000).unwrap();
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::HelperCrashed(_)), "{e}");
    assert!(s.is_closed());
}

#[test]
fn helper_hang_times_out_and_is_killed() {
    let mut s = spawn(&["--inject=hang"], 300).unwrap();
    let t = Instant::now();
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::Timeout { op: "Ping", .. }), "{e}");
    assert!(t.elapsed() < Duration::from_secs(5));
    assert_eq!(s.ping().unwrap_err(), WineError::SessionClosed);
}

#[test]
fn helper_garbage_is_a_protocol_error() {
    let mut s = spawn(&["--inject=garbage"], 5000).unwrap();
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::Protocol(_)), "{e}");
}

#[test]
fn unknown_helper_argument_makes_the_helper_exit() {
    let e = spawn(&["--bogus"], 2000).err().unwrap();
    assert!(
        matches!(e, WineError::HelperCrashed(_) | WineError::Transport(_)),
        "{e}"
    );
}

#[test]
fn missing_helper_is_unavailable_and_nothing_starts() {
    let e = Session::spawn(&WineConfig::direct("/nonexistent/vba-wine-host"), opts(100))
        .err()
        .unwrap();
    assert!(matches!(e, WineError::Unavailable(_)), "{e}");
    let e = Session::spawn(&WineConfig::wine("/nonexistent/wine", helper()), opts(100))
        .err()
        .unwrap();
    assert!(matches!(e, WineError::Unavailable(_)), "{e}");
}

#[test]
fn drop_kills_the_helper() {
    // A hung helper must not outlive its session: dropping returns promptly.
    let s = spawn(&["--inject=hang"], 5000).unwrap();
    let t = Instant::now();
    drop(s);
    assert!(t.elapsed() < Duration::from_secs(5));
}

#[test]
fn procref_and_conv_are_usable_from_the_public_api() {
    // Compile-time check that the types the docs mention are exported.
    let _ = (ProcRef::Ordinal(1), WireConv::Cdecl, RetKind::Void);
}

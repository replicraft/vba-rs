//! End-to-end tests against the real `vba-wine-host.exe` running under Wine.
//!
//! They run only when both `VBA_WINE` (path to the `wine` executable) and
//! `VBA_WINE_HELPER` (path to a Windows build of `vba-wine-host`) are set;
//! `VBA_WINE_PREFIX` optionally selects the `WINEPREFIX` (use a scratch one:
//! Wine is not a sandbox). Otherwise every test returns immediately.

use std::time::Duration;
use vba_ffi::Handle;
use vba_wine::protocol::{ProcRef, RetKind, WireArg, WireConv, WireRet};
use vba_wine::{Session, SessionOptions, WineConfig, WineError};

fn session() -> Option<Session> {
    let wine = std::env::var_os("VBA_WINE")?;
    let helper = std::env::var_os("VBA_WINE_HELPER")?;
    let mut cfg = WineConfig::wine(wine, helper)
        .with_env("WINEDEBUG", "-all")
        .with_env("WINEDLLOVERRIDES", "mscoree,mshtml=");
    if let Some(p) = std::env::var_os("VBA_WINE_PREFIX") {
        cfg = cfg.with_prefix(p);
    }
    let opts = SessionOptions {
        timeout: Duration::from_secs(60),
        ..SessionOptions::default()
    };
    Some(Session::spawn(&cfg, opts).expect("helper under Wine must start"))
}

fn proc(s: &mut Session, lib: &str, name: &str) -> Handle {
    let l = s.load_library(lib).unwrap_or_else(|e| panic!("{lib}: {e}"));
    s.get_proc(&l, ProcRef::Name(name.into()))
        .unwrap_or_else(|e| panic!("{lib}!{name}: {e}"))
}

fn int(r: WireRet) -> i64 {
    match r {
        WireRet::Int(v) => v,
        other => panic!("expected an integer, got {other:?}"),
    }
}

/// The low 32 bits: a C `int`/`DWORD` return leaves the upper half of RAX
/// unspecified, and the bridge returns RAX unnarrowed.
fn low32(r: WireRet) -> i64 {
    int(r) & 0xFFFF_FFFF
}

fn cstr(s: &str) -> WireArg {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    WireArg::BufIn(b)
}

#[test]
fn handshake_reports_windows_helper_under_wine() {
    let Some(mut s) = session() else { return };
    let i = s.info().clone();
    assert_eq!(i.os, "windows");
    assert_eq!(i.bitness, 64);
    assert!(!i.wine_version.is_empty(), "wine_get_version: {i:?}");
    s.ping().unwrap();
    s.shutdown().unwrap();
}

#[test]
fn kernel32_scalars_and_buffers() {
    let Some(mut s) = session() else { return };
    let tick = proc(&mut s, "kernel32.dll", "GetTickCount");
    let r = s.call(&tick, WireConv::Stdcall, RetKind::Int, &[]).unwrap();
    assert_ne!(low32(r.ret), 0);

    let len = proc(&mut s, "kernel32", "lstrlenA");
    let r = s
        .call(&len, WireConv::Stdcall, RetKind::Int, &[cstr("hello wine")])
        .unwrap();
    assert_eq!(low32(r.ret), 10);

    // MultiByteToWideChar(CP_UTF8, 0, "h\xC3\xA9", -1, out, 8) -> 3 units.
    let mb = proc(&mut s, "kernel32", "MultiByteToWideChar");
    let r = s
        .call(
            &mb,
            WireConv::Stdcall,
            RetKind::Int,
            &[
                WireArg::Int(65001),
                WireArg::Int(0),
                WireArg::BufIn(b"h\xC3\xA9\0".to_vec()),
                WireArg::Int(-1),
                WireArg::BufOut(16),
                WireArg::Int(8),
            ],
        )
        .unwrap();
    assert_eq!(low32(r.ret), 3);
    let (idx, buf) = &r.buffers[0];
    assert_eq!(*idx, 4);
    assert_eq!(&buf[..6], &[b'h', 0, 0xE9, 0, 0, 0]);

    // WideCharToMultiByte with eight arguments, two of them null pointers.
    let wc = proc(&mut s, "kernel32", "WideCharToMultiByte");
    let r = s
        .call(
            &wc,
            WireConv::Stdcall,
            RetKind::Int,
            &[
                WireArg::Int(65001),
                WireArg::Int(0),
                WireArg::BufIn(vec![b'h', 0, 0xE9, 0, 0, 0]),
                WireArg::Int(-1),
                WireArg::BufOut(16),
                WireArg::Int(16),
                WireArg::Int(0),
                WireArg::Int(0),
            ],
        )
        .unwrap();
    assert_eq!(low32(r.ret), 4);
    assert_eq!(&r.buffers[0].1[..4], b"h\xC3\xA9\0");
    s.shutdown().unwrap();
}

#[test]
fn last_error_is_captured_and_reset() {
    let Some(mut s) = session() else { return };
    let set = proc(&mut s, "kernel32", "SetLastError");
    let r = s
        .call(
            &set,
            WireConv::Stdcall,
            RetKind::Void,
            &[WireArg::Int(1234)],
        )
        .unwrap();
    assert_eq!((r.ret, r.last_error), (WireRet::Void, 1234));

    let attrs = proc(&mut s, "kernel32", "GetFileAttributesA");
    let r = s
        .call(
            &attrs,
            WireConv::Stdcall,
            RetKind::Int,
            &[cstr("C:\\vba_wine_no_such_file")],
        )
        .unwrap();
    assert_eq!(low32(r.ret), 0xFFFF_FFFF); // INVALID_FILE_ATTRIBUTES
    assert!(matches!(r.last_error, 2 | 3), "error {}", r.last_error);

    // A call that sets no error reports 0, not the previous value.
    let tick = proc(&mut s, "kernel32", "GetTickCount");
    let r = s.call(&tick, WireConv::Stdcall, RetKind::Int, &[]).unwrap();
    assert_eq!(r.last_error, 0);
    s.shutdown().unwrap();
}

#[test]
fn cdecl_msvcrt_including_variadic_and_sixteen_args() {
    let Some(mut s) = session() else { return };
    let strlen = proc(&mut s, "msvcrt.dll", "strlen");
    let r = s
        .call(&strlen, WireConv::Cdecl, RetKind::Int, &[cstr("abcdef")])
        .unwrap();
    assert_eq!(int(r.ret), 6);

    let abs = proc(&mut s, "msvcrt", "abs");
    let r = s
        .call(&abs, WireConv::Cdecl, RetKind::Int, &[WireArg::Int(-5)])
        .unwrap();
    assert_eq!(low32(r.ret), 5);

    // sprintf(buf, "%d-%d", 7, 9): variadic cdecl with an output buffer.
    let sprintf = proc(&mut s, "msvcrt", "sprintf");
    let r = s
        .call(
            &sprintf,
            WireConv::Cdecl,
            RetKind::Int,
            &[
                WireArg::BufOut(32),
                cstr("%d-%d"),
                WireArg::Int(7),
                WireArg::Int(9),
            ],
        )
        .unwrap();
    assert_eq!(low32(r.ret), 3);
    assert_eq!(&r.buffers[0].1[..4], b"7-9\0");

    // 16 arguments: 14 integers after the buffer and format exercise the stack.
    let mut args = vec![WireArg::BufOut(128), cstr(&"%d,".repeat(14))];
    args.extend((1..=14).map(WireArg::Int));
    assert_eq!(args.len(), 16);
    let r = s
        .call(&sprintf, WireConv::Cdecl, RetKind::Int, &args)
        .unwrap();
    let text = String::from_utf8_lossy(&r.buffers[0].1);
    assert!(
        text.starts_with("1,2,3,4,5,6,7,8,9,10,11,12,13,14,\0"),
        "{text:?}"
    );

    // Seventeen arguments are refused before anything reaches the helper.
    let too_many: Vec<WireArg> = (0..17).map(WireArg::Int).collect();
    assert!(
        s.call(&abs, WireConv::Cdecl, RetKind::Int, &too_many)
            .is_err()
    );
    s.shutdown().unwrap_or(());
}

#[test]
fn floating_point_arguments_and_returns() {
    let Some(mut s) = session() else { return };
    // double pow(double, double): both arguments in XMM0/XMM1.
    let pow = proc(&mut s, "msvcrt", "pow");
    let r = s
        .call(
            &pow,
            WireConv::Cdecl,
            RetKind::F64,
            &[WireArg::F64(2.0), WireArg::F64(10.0)],
        )
        .unwrap();
    assert_eq!(r.ret, WireRet::Float(1024.0));

    // double ldexp(double, int): a float at position 0, an int at position 1.
    let ldexp = proc(&mut s, "msvcrt", "ldexp");
    let r = s
        .call(
            &ldexp,
            WireConv::Cdecl,
            RetKind::F64,
            &[WireArg::F64(1.5), WireArg::Int(4)],
        )
        .unwrap();
    assert_eq!(r.ret, WireRet::Float(24.0));

    // float sqrtf(float).
    let sqrtf = proc(&mut s, "ucrtbase", "sqrtf");
    let r = s
        .call(&sqrtf, WireConv::Cdecl, RetKind::F32, &[WireArg::F32(2.25)])
        .unwrap();
    assert_eq!(r.ret, WireRet::Float(1.5));

    // Integer return, float argument: long lround(double).
    let lround = proc(&mut s, "ucrtbase", "lround");
    let r = s
        .call(&lround, WireConv::Cdecl, RetKind::Int, &[WireArg::F64(2.5)])
        .unwrap();
    assert_eq!(low32(r.ret), 3);
    s.shutdown().unwrap_or(());
}

#[test]
fn missing_library_and_symbol_are_distinct_errors() {
    let Some(mut s) = session() else { return };
    match s.load_library("vba_wine_no_such_library.dll") {
        Err(WineError::LibraryNotFound(m)) => assert!(m.contains("126"), "{m}"),
        other => panic!("{other:?}"),
    }
    let k = s.load_library("kernel32").unwrap();
    match s.get_proc(&k, ProcRef::Name("NoSuchExport".into())) {
        Err(WineError::SymbolNotFound(m)) => assert!(m.contains("127"), "{m}"),
        other => panic!("{other:?}"),
    }
    match s.get_proc(&k, ProcRef::Ordinal(0xFFF0)) {
        Err(WineError::SymbolNotFound(_)) => {}
        other => panic!("{other:?}"),
    }
    // The session survives remote errors.
    s.ping().unwrap();
    s.shutdown().unwrap();
}

#[test]
fn free_library_invalidates_procs() {
    let Some(mut s) = session() else { return };
    let l = s.load_library("kernel32").unwrap();
    let p = s
        .get_proc(&l, ProcRef::Name("GetTickCount".into()))
        .unwrap();
    s.free_library(&l).unwrap();
    assert!(matches!(
        s.call(&p, WireConv::Stdcall, RetKind::Int, &[]),
        Err(WineError::InvalidHandle(_))
    ));
    s.shutdown().unwrap();
}

#[test]
fn a_dying_helper_is_reported_not_hidden() {
    let Some(mut s) = session() else { return };
    let exit = proc(&mut s, "kernel32", "ExitProcess");
    let e = s
        .call(&exit, WireConv::Stdcall, RetKind::Void, &[WireArg::Int(7)])
        .unwrap_err();
    assert!(matches!(e, WineError::HelperCrashed(_)), "{e:?}");
    assert!(s.is_closed());
}

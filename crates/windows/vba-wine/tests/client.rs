//! The client against an in-process mock helper (and scripted bad helpers).

use std::time::Duration;
use vba_ffi::{NativeArg, NativeReturn};
use vba_host::{Capability, Policy};
use vba_profile::Target;
use vba_winapi::{CallContext, Identity, ManualClock, Provider, Registry, WinApiError};
use vba_wine::helper::{Backend, Server};
use vba_wine::protocol::{
    CAP_NATIVE_CALLS, HelloInfo, MAX_FRAME, PROTOCOL_VERSION, ProcRef, RemoteError,
    RemoteErrorKind, Request, Response, RetKind, WireArg, WireConv, WireRet,
};
use vba_wine::{
    ChannelTransport, ParamKind, Session, SessionOptions, WineError, WineExport, WineProvider,
};

/// A fake "native world": library `mock` with `add`, `fill`, `swap`.
struct Mock {
    bitness: u8,
    libs: Vec<&'static str>,
}

impl Mock {
    fn new() -> Self {
        Mock {
            bitness: 64,
            libs: Vec::new(),
        }
    }
}

impl Backend for Mock {
    fn info(&self) -> HelloInfo {
        HelloInfo {
            version: PROTOCOL_VERSION,
            bitness: self.bitness,
            os: "mock".into(),
            wine_version: "mock-9.0".into(),
            helper: "mock".into(),
            caps: CAP_NATIVE_CALLS,
        }
    }
    fn load_library(&mut self, name: &str) -> Result<u64, RemoteError> {
        if name == "mock" {
            self.libs.push("mock");
            Ok(0xDEAD_0000 + self.libs.len() as u64)
        } else {
            Err(RemoteError {
                kind: RemoteErrorKind::LibraryNotFound,
                code: 126,
                message: format!("{name} missing"),
            })
        }
    }
    fn get_proc(&mut self, _lib: u64, proc: &ProcRef) -> Result<u64, RemoteError> {
        match proc {
            ProcRef::Name(n) if n == "add" => Ok(1),
            ProcRef::Name(n) if n == "fill" => Ok(2),
            ProcRef::Name(n) if n == "boom" => Ok(3),
            _ => Err(RemoteError::new(
                RemoteErrorKind::SymbolNotFound,
                "no such symbol",
            )),
        }
    }
    fn call(
        &mut self,
        proc: u64,
        _conv: WireConv,
        _ret: RetKind,
        args: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError> {
        match proc {
            1 => match args {
                [WireArg::Int(a), WireArg::Int(b)] => Ok((WireRet::Int(*a + *b), 183)),
                _ => Err(RemoteError::new(RemoteErrorKind::CallFault, "bad args")),
            },
            2 => {
                if let [WireArg::BufInOut(b)] = args {
                    for (i, x) in b.iter_mut().enumerate() {
                        *x = i as u8 + 1;
                    }
                }
                Ok((WireRet::Void, 0))
            }
            _ => Err(RemoteError::new(RemoteErrorKind::CallFault, "boom")),
        }
    }
    fn free_library(&mut self, _: u64) -> Result<(), RemoteError> {
        Ok(())
    }
}

fn spawn_server(backend: impl Backend + Send + 'static) -> ChannelTransport {
    let (t, peer) = ChannelTransport::pair();
    std::thread::spawn(move || {
        let mut s = Server::new(backend);
        while let Some(f) = peer.recv() {
            let (out, bad) = s.handle_frame(&f);
            peer.send(out);
            if bad.is_some() || s.is_stopped() {
                break;
            }
        }
    });
    t
}

fn opts(ms: u64) -> SessionOptions {
    SessionOptions {
        timeout: Duration::from_millis(ms),
        ..Default::default()
    }
}

fn session() -> Session {
    Session::connect(Box::new(spawn_server(Mock::new())), opts(2000)).unwrap()
}

#[test]
fn hello_reports_helper_facts() {
    let s = session();
    assert_eq!(s.info().bitness, 64);
    assert_eq!(s.info().wine_version, "mock-9.0");
    assert_eq!(s.info().caps, CAP_NATIVE_CALLS);
}

#[test]
fn load_resolve_call_free() {
    let mut s = session();
    s.ping().unwrap();
    let lib = s.load_library("mock").unwrap();
    let add = s.get_proc(&lib, ProcRef::Name("add".into())).unwrap();
    let r = s
        .call(
            &add,
            WireConv::Stdcall,
            RetKind::Int,
            &[WireArg::Int(40), WireArg::Int(2)],
        )
        .unwrap();
    assert_eq!(r.ret, WireRet::Int(42));
    assert_eq!(r.last_error, 183);
    let fill = s.get_proc(&lib, ProcRef::Name("fill".into())).unwrap();
    let r = s
        .call(&fill, WireConv::Cdecl, RetKind::Void, &[WireArg::BufOut(4)])
        .unwrap();
    assert_eq!(r.buffers, vec![(0, vec![1, 2, 3, 4])]);
    s.free_library(&lib).unwrap();
    // Handles die with the library; nothing leaks to the helper.
    let e = s
        .call(&add, WireConv::Stdcall, RetKind::Int, &[])
        .unwrap_err();
    assert!(matches!(e, WineError::InvalidHandle(_)), "{e}");
    s.shutdown().unwrap();
}

#[test]
fn remote_errors_are_distinct_and_not_fatal() {
    let mut s = session();
    let e = s.load_library("nope").unwrap_err();
    assert_eq!(e.kind(), "LibraryNotFound");
    assert!(e.to_string().contains("126"));
    let lib = s.load_library("mock").unwrap();
    let e = s.get_proc(&lib, ProcRef::Name("zzz".into())).unwrap_err();
    assert_eq!(e.kind(), "SymbolNotFound");
    let boom = s.get_proc(&lib, ProcRef::Name("boom".into())).unwrap();
    let e = s
        .call(&boom, WireConv::Stdcall, RetKind::Void, &[])
        .unwrap_err();
    assert_eq!(e.kind(), "CallFault");
    // Still usable.
    s.ping().unwrap();
    assert!(!s.is_closed());
}

#[test]
fn handles_are_session_scoped() {
    let mut a = session();
    let mut b = session();
    let la = a.load_library("mock").unwrap();
    let _lb = b.load_library("mock").unwrap();
    let e = b.get_proc(&la, ProcRef::Name("add".into())).unwrap_err();
    assert!(matches!(e, WineError::InvalidHandle(_)), "{e}");
    // The bare integer VBA would see is not valid in another session either.
    assert!(a.get_proc(&la, ProcRef::Name("add".into())).is_ok());
}

#[test]
fn local_validation_precedes_sending() {
    let mut s = session();
    let lib = s.load_library("mock").unwrap();
    let add = s.get_proc(&lib, ProcRef::Name("add".into())).unwrap();
    let too_many = vec![WireArg::Int(0); 61];
    assert_eq!(
        s.call(&add, WireConv::Stdcall, RetKind::Int, &too_many)
            .unwrap_err()
            .kind(),
        "BadRequest"
    );
    let huge = vec![WireArg::BufIn(vec![0; MAX_FRAME])];
    assert_eq!(
        s.call(&add, WireConv::Stdcall, RetKind::Int, &huge)
            .unwrap_err()
            .kind(),
        "BadRequest"
    );
    s.ping().unwrap();
}

#[test]
fn thirty_two_bit_helper_rejects_wide_ints() {
    let mut m = Mock::new();
    m.bitness = 32;
    let mut s = Session::connect(Box::new(spawn_server(m)), opts(2000)).unwrap();
    let lib = s.load_library("mock").unwrap();
    let add = s.get_proc(&lib, ProcRef::Name("add".into())).unwrap();
    let e = s
        .call(
            &add,
            WireConv::Stdcall,
            RetKind::Int,
            &[WireArg::Int(1 << 40), WireArg::Int(0)],
        )
        .unwrap_err();
    assert_eq!(e.kind(), "BadRequest");
    assert!(
        s.call(
            &add,
            WireConv::Stdcall,
            RetKind::Int,
            &[WireArg::Int(0xFFFF_FFFF), WireArg::Int(0)]
        )
        .is_ok()
    );
}

// ---- failure modes -------------------------------------------------------

#[test]
fn timeout_kills_and_closes() {
    let (t, peer) = ChannelTransport::pair();
    // Answer Hello, then go silent.
    std::thread::spawn(move || {
        let mut s = Server::new(Mock::new());
        let f = peer.recv().unwrap();
        peer.send(s.handle_frame(&f).0);
        std::thread::sleep(Duration::from_secs(5));
        drop(peer);
    });
    let mut s = Session::connect(Box::new(t), opts(100)).unwrap();
    let e = s.load_library("mock").unwrap_err();
    assert!(
        matches!(
            e,
            WineError::Timeout {
                op: "LoadLibrary",
                ..
            }
        ),
        "{e}"
    );
    assert!(s.is_closed());
    assert_eq!(s.ping().unwrap_err(), WineError::SessionClosed);
}

#[test]
fn hello_timeout_is_reported() {
    let (t, _peer) = ChannelTransport::pair();
    let e = Session::connect(Box::new(t), opts(50)).err().unwrap();
    assert!(matches!(e, WineError::Timeout { op: "Hello", .. }), "{e}");
}

#[test]
fn helper_crash_is_distinct() {
    let (t, peer) = ChannelTransport::pair();
    std::thread::spawn(move || {
        let mut s = Server::new(Mock::new());
        let f = peer.recv().unwrap();
        peer.send(s.handle_frame(&f).0);
        let _ = peer.recv(); // take the next request, then die
    });
    let mut s = Session::connect(Box::new(t), opts(2000)).unwrap();
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::HelperCrashed(_)), "{e}");
    assert!(s.is_closed());
}

#[test]
fn version_mismatch_both_ways() {
    // Helper answers Hello with a different version.
    struct Liar(Mock);
    impl Backend for Liar {
        fn info(&self) -> HelloInfo {
            HelloInfo {
                version: 99,
                ..self.0.info()
            }
        }
        fn load_library(&mut self, n: &str) -> Result<u64, RemoteError> {
            self.0.load_library(n)
        }
        fn get_proc(&mut self, l: u64, p: &ProcRef) -> Result<u64, RemoteError> {
            self.0.get_proc(l, p)
        }
        fn call(
            &mut self,
            p: u64,
            c: WireConv,
            r: RetKind,
            a: &mut [WireArg],
        ) -> Result<(WireRet, u32), RemoteError> {
            self.0.call(p, c, r, a)
        }
        fn free_library(&mut self, l: u64) -> Result<(), RemoteError> {
            self.0.free_library(l)
        }
    }
    let e = Session::connect(Box::new(spawn_server(Liar(Mock::new()))), opts(2000))
        .err()
        .unwrap();
    assert_eq!(
        e,
        WineError::VersionMismatch {
            client: PROTOCOL_VERSION,
            helper: 99
        }
    );
    // Helper refuses an unknown client version.
    let mut srv = Server::new(Mock::new());
    let r = srv.handle(Request::Hello {
        version: 7,
        session: 1,
    });
    let Response::Error(re) = r else { panic!() };
    assert_eq!(re.kind, RemoteErrorKind::VersionMismatch);
    assert_eq!(re.code, u32::from(PROTOCOL_VERSION));
}

/// A scripted peer: answers Hello properly, then replies with `frame_for`.
fn scripted(frame_for: impl Fn(u64) -> Vec<u8> + Send + 'static) -> Session {
    let (t, peer) = ChannelTransport::pair();
    std::thread::spawn(move || {
        let mut s = Server::new(Mock::new());
        let f = peer.recv().unwrap();
        peer.send(s.handle_frame(&f).0);
        while let Some(f) = peer.recv() {
            let (id, _) = Request::decode(&f).unwrap_or((0, Request::Ping));
            peer.send(frame_for(id));
        }
    });
    Session::connect(Box::new(t), opts(2000)).unwrap()
}

#[test]
fn malformed_response_is_a_protocol_error() {
    let mut s = scripted(|_| vec![0xEE, 0, 1]);
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::Protocol(_)), "{e}");
    assert!(s.is_closed());
}

#[test]
fn mismatched_response_id_is_a_protocol_error() {
    let mut s = scripted(|id| Response::Ok.encode(id + 1000));
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::Protocol(m) if m.contains("does not match")));
}

#[test]
fn wrong_response_type_is_a_protocol_error() {
    let mut s = scripted(|id| Response::Library(5).encode(id));
    let e = s.ping().unwrap_err();
    assert!(matches!(e, WineError::Protocol(_)));
    assert!(s.is_closed());
}

#[test]
fn undecodable_request_answer_with_id_zero_is_fatal() {
    let err = Response::Error(RemoteError::new(RemoteErrorKind::ProtocolViolation, "bad"));
    let mut s = scripted(move |_| err.encode(0));
    assert!(matches!(s.ping().unwrap_err(), WineError::Protocol(_)));
}

#[test]
fn helper_returning_unrequested_buffer_is_a_protocol_error() {
    // Answer the LoadLibrary/GetProc honestly via a real server, but lie in Call.
    let (t, peer) = ChannelTransport::pair();
    std::thread::spawn(move || {
        let mut s = Server::new(Mock::new());
        while let Some(f) = peer.recv() {
            let (id, req) = Request::decode(&f).unwrap();
            if matches!(req, Request::Call { .. }) {
                peer.send(
                    Response::CallResult {
                        ret: WireRet::Int(0),
                        last_error: 0,
                        buffers: vec![(0, vec![1])],
                    }
                    .encode(id),
                );
            } else {
                peer.send(s.handle_frame(&f).0);
            }
        }
    });
    let mut s = Session::connect(Box::new(t), opts(2000)).unwrap();
    let lib = s.load_library("mock").unwrap();
    let add = s.get_proc(&lib, ProcRef::Name("add".into())).unwrap();
    let e = s
        .call(&add, WireConv::Stdcall, RetKind::Int, &[WireArg::Int(1)])
        .unwrap_err();
    assert!(matches!(e, WineError::Protocol(_)), "{e}");
}

#[test]
fn server_enforces_hello_first_and_unknown_ids() {
    let mut srv = Server::new(Mock::new());
    let Response::Error(e) = srv.handle(Request::Ping) else {
        panic!()
    };
    assert_eq!(e.kind, RemoteErrorKind::ProtocolViolation);
    srv.handle(Request::Hello {
        version: PROTOCOL_VERSION,
        session: 1,
    });
    let Response::Error(e) = srv.handle(Request::Hello {
        version: PROTOCOL_VERSION,
        session: 1,
    }) else {
        panic!()
    };
    assert_eq!(e.kind, RemoteErrorKind::ProtocolViolation);
    let Response::Error(e) = srv.handle(Request::GetProc {
        lib: 0xDEAD_0001,
        proc: ProcRef::Name("add".into()),
    }) else {
        panic!()
    };
    assert_eq!(e.kind, RemoteErrorKind::InvalidHandle);
    // A raw backend id is not a valid wire id.
    let Response::Library(l) = srv.handle(Request::LoadLibrary {
        name: "mock".into(),
    }) else {
        panic!()
    };
    assert_ne!(l, 0xDEAD_0001);
}

// ---- provider ------------------------------------------------------------

fn exports() -> Vec<WineExport> {
    vec![
        WineExport {
            library: "mock",
            symbol: "add",
            conv: WireConv::Stdcall,
            params: vec![ParamKind::Int, ParamKind::Int],
            ret: RetKind::Int,
        },
        WineExport {
            library: "mock",
            symbol: "fill",
            conv: WireConv::Stdcall,
            params: vec![ParamKind::BufOut],
            ret: RetKind::Void,
        },
    ]
}

fn ctx(t: Target) -> CallContext {
    CallContext::new(
        t,
        Box::new(ManualClock::at(0)),
        Identity::new("u", "C").unwrap(),
    )
}

#[test]
fn provider_requires_grant_and_does_not_start_anything() {
    let e = WineProvider::new(Policy::deny_all(), session(), exports())
        .err()
        .unwrap();
    assert_eq!(e.kind(), "PermissionDenied");
    // Other capabilities do not help.
    let p = Policy::deny_all()
        .grant(Capability::ProcessSpawn)
        .grant(Capability::ComCreate);
    assert!(WineProvider::new(p, session(), exports()).is_err());
}

#[test]
fn provider_export_table_is_validated() {
    let mut e = exports();
    e.push(e[0].clone());
    let r = WineProvider::new(Policy::allow_all(), session(), e);
    assert_eq!(r.err().unwrap().kind(), "BadRequest");
    let mut e = exports();
    e[0].library = "Mock.dll";
    assert!(WineProvider::new(Policy::allow_all(), session(), e).is_err());
}

#[test]
fn provider_call_through_registry() {
    let p = WineProvider::new(Policy::allow_all(), session(), exports()).unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(p)).unwrap();
    let mut c = ctx(Target::WIN64);
    let resolved = reg
        .resolve(
            Target::WIN64.os,
            "MOCK.DLL",
            &[vba_ffi::EntryCandidate::Symbol("add".into())],
        )
        .unwrap();
    let r = reg
        .call(&mut c, &resolved, &[NativeArg::Int(2), NativeArg::Int(3)])
        .unwrap();
    assert_eq!(r, NativeReturn::Int(5));
    assert_eq!(c.last_error(), 183, "helper's GetLastError is surfaced");
}

#[test]
fn provider_scoped_grant_limits_symbols() {
    let policy = Policy::deny_all().grant_prefixes(Capability::NativeDeclare, ["mock!add"]);
    let p = WineProvider::new(policy, session(), exports()).unwrap();
    let mut c = ctx(Target::WIN64);
    assert!(
        p.call(
            &mut c,
            "mock",
            "add",
            &[NativeArg::Int(1), NativeArg::Int(1)]
        )
        .is_ok()
    );
    assert_eq!(
        p.call_with_buffers("mock", "fill", &[WireArg::BufOut(2)])
            .unwrap_err()
            .kind(),
        "PermissionDenied"
    );
}

#[test]
fn provider_failures_are_visible_not_fake_success() {
    let p = WineProvider::new(Policy::allow_all(), session(), exports()).unwrap();
    let mut c = ctx(Target::WIN64);
    // Buffer exports cannot be reached with emulated-memory pointers.
    let e = p
        .call(&mut c, "mock", "fill", &[NativeArg::Int(0x20000)])
        .unwrap_err();
    assert!(
        matches!(&e, WinApiError::Unsupported { reason, .. } if reason.starts_with("wine bridge Unsupported"))
    );
    // Wrong kinds / arity.
    assert!(matches!(
        p.call(
            &mut c,
            "mock",
            "add",
            &[NativeArg::F64(1.0), NativeArg::Int(1)]
        ),
        Err(WinApiError::BadArguments { .. })
    ));
    assert!(matches!(
        p.call(&mut c, "mock", "add", &[]),
        Err(WinApiError::BadArguments { .. })
    ));
    // Declared 32-bit target against a 64-bit helper.
    let mut c32 = ctx(Target::WIN32);
    let e = p
        .call(
            &mut c32,
            "mock",
            "add",
            &[NativeArg::Int(1), NativeArg::Int(1)],
        )
        .unwrap_err();
    assert!(matches!(&e, WinApiError::Unsupported { reason, .. } if reason.contains("helper")));
}

#[test]
fn provider_buffer_call_and_unregistered_export() {
    let p = WineProvider::new(Policy::allow_all(), session(), exports()).unwrap();
    let r = p
        .call_with_buffers("mock", "fill", &[WireArg::BufOut(3)])
        .unwrap();
    assert_eq!(r.buffers, vec![(0, vec![1, 2, 3])]);
    assert_eq!(
        p.call_with_buffers("mock", "fill", &[WireArg::Int(1)])
            .unwrap_err()
            .kind(),
        "BadRequest"
    );
    assert_eq!(
        p.call_with_buffers("mock", "nope", &[]).unwrap_err().kind(),
        "Unsupported"
    );
}

#[test]
fn provider_reports_bridge_crash_with_prefix() {
    let (t, peer) = ChannelTransport::pair();
    std::thread::spawn(move || {
        let mut s = Server::new(Mock::new());
        // Hello, LoadLibrary, GetProc succeed; the Call kills the peer.
        for _ in 0..3 {
            let f = peer.recv().unwrap();
            peer.send(s.handle_frame(&f).0);
        }
        let _ = peer.recv();
    });
    let session = Session::connect(Box::new(t), opts(2000)).unwrap();
    let p = WineProvider::new(Policy::allow_all(), session, exports()).unwrap();
    let mut c = ctx(Target::WIN64);
    let e = p
        .call(
            &mut c,
            "mock",
            "add",
            &[NativeArg::Int(1), NativeArg::Int(1)],
        )
        .unwrap_err();
    assert!(
        matches!(&e, WinApiError::Unsupported { reason, .. } if reason.starts_with("wine bridge HelperCrashed")),
        "{e}"
    );
    // And it stays failed.
    let e = p
        .call(
            &mut c,
            "mock",
            "add",
            &[NativeArg::Int(1), NativeArg::Int(1)],
        )
        .unwrap_err();
    assert!(
        matches!(&e, WinApiError::Unsupported { reason, .. } if reason.contains("SessionClosed"))
    );
}

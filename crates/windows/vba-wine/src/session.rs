//! A client session: one helper, one request in flight.
//!
//! `Session` methods take `&mut self`, so a session can never have two
//! requests pending; share one behind a `Mutex` (as `WineProvider` does) or
//! run one session per thread. Any transport, protocol, crash or timeout
//! failure kills the helper and closes the session permanently; remote
//! errors such as `LibraryNotFound` leave it usable.

use crate::config::WineConfig;
use crate::error::WineError;
use crate::protocol::{
    HelloInfo, MAX_ARGS, MAX_FRAME, PROTOCOL_VERSION, ProcRef, Request, Response, RetKind, WireArg,
    WireConv, WireRet,
};
use crate::transport::{ProcessTransport, Transport};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use vba_ffi::{Handle, HandleTable, ProviderId};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// Provider id used for session handles unless configured otherwise.
pub const DEFAULT_PROVIDER: ProviderId = ProviderId("wine");

/// Session tuning.
#[derive(Debug, Clone, Copy)]
pub struct SessionOptions {
    /// Per-request timeout, including the Hello.
    pub timeout: Duration,
    /// Provider id for handle affinity.
    pub provider: ProviderId,
}

impl Default for SessionOptions {
    fn default() -> Self {
        SessionOptions {
            timeout: Duration::from_secs(10),
            provider: DEFAULT_PROVIDER,
        }
    }
}

/// The result of a call.
#[derive(Debug, Clone, PartialEq)]
pub struct CallReply {
    /// Return value.
    pub ret: WireRet,
    /// `GetLastError` captured by the helper right after the call.
    pub last_error: u32,
    /// `(argument index, bytes)` for every in/out and out buffer.
    pub buffers: Vec<(u32, Vec<u8>)>,
}

/// A connected helper.
pub struct Session {
    transport: Box<dyn Transport>,
    opts: SessionOptions,
    next_req: u64,
    info: HelloInfo,
    session_id: u64,
    handles: HandleTable,
    libs: HashMap<u64, u64>,
    procs: HashMap<u64, (u64, u64)>,
    closed: bool,
}

impl Session {
    /// Spawns the configured helper and performs the Hello. This is the only
    /// place a process is started, and only because the embedder called it.
    pub fn spawn(config: &WineConfig, opts: SessionOptions) -> Result<Session, WineError> {
        Self::connect(Box::new(ProcessTransport::spawn(config)?), opts)
    }

    /// Performs the Hello over an existing transport.
    pub fn connect(
        transport: Box<dyn Transport>,
        opts: SessionOptions,
    ) -> Result<Session, WineError> {
        let session_id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        let mut s = Session {
            transport,
            opts,
            next_req: 1,
            info: HelloInfo {
                version: 0,
                bitness: 0,
                os: String::new(),
                wine_version: String::new(),
                helper: String::new(),
                caps: 0,
            },
            session_id,
            handles: HandleTable::new(),
            libs: HashMap::new(),
            procs: HashMap::new(),
            closed: false,
        };
        let resp = s.request(
            "Hello",
            Request::Hello {
                version: PROTOCOL_VERSION,
                session: session_id,
            },
        )?;
        let Response::Hello(info) = resp else {
            return Err(s.fatal(WineError::Protocol(
                "Hello answered with another message".into(),
            )));
        };
        if info.version != PROTOCOL_VERSION {
            let e = WineError::VersionMismatch {
                client: PROTOCOL_VERSION,
                helper: info.version,
            };
            return Err(s.fatal(e));
        }
        if info.bitness != 32 && info.bitness != 64 {
            let e = WineError::Protocol(format!("helper reported bitness {}", info.bitness));
            return Err(s.fatal(e));
        }
        s.info = info;
        Ok(s)
    }

    /// Changes the per-request timeout for subsequent requests. Lets a caller
    /// use a generous timeout for the Hello (process start-up) and a tight one
    /// for later requests.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.opts.timeout = timeout;
    }

    /// What the helper reported (bitness, OS, Wine version, capabilities).
    pub fn info(&self) -> &HelloInfo {
        &self.info
    }

    /// The client-chosen session id sent in Hello.
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// The provider id that owns this session's handles.
    pub fn provider(&self) -> ProviderId {
        self.opts.provider
    }

    /// Whether the session has failed or been shut down.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    fn fatal(&mut self, e: WineError) -> WineError {
        self.closed = true;
        self.transport.kill();
        e
    }

    fn request(&mut self, op: &'static str, req: Request) -> Result<Response, WineError> {
        if self.closed {
            return Err(WineError::SessionClosed);
        }
        let id = self.next_req;
        self.next_req += 1;
        let body = req.encode(id);
        if body.len() > MAX_FRAME {
            return Err(WineError::BadRequest(format!(
                "request of {} bytes exceeds the {MAX_FRAME} byte frame limit",
                body.len()
            )));
        }
        if let Err(e) = self.transport.send(body) {
            return Err(self.fatal(e));
        }
        let frame = match self.transport.recv(self.opts.timeout) {
            Ok(Some(f)) => f,
            Ok(None) => {
                let e = WineError::Timeout {
                    op,
                    after: self.opts.timeout,
                };
                return Err(self.fatal(e));
            }
            Err(e) => return Err(self.fatal(e)),
        };
        let (rid, resp) = match Response::decode(&frame) {
            Ok(x) => x,
            Err(e) => return Err(self.fatal(e.into())),
        };
        match resp {
            // The helper answers an undecodable request with id 0.
            Response::Error(e) if rid == 0 => Err(self.fatal(e.into())),
            _ if rid != id => Err(self.fatal(WineError::Protocol(format!(
                "response id {rid} does not match request id {id}"
            )))),
            Response::Error(e) => {
                let e: WineError = e.into();
                if e.is_fatal() {
                    Err(self.fatal(e))
                } else {
                    Err(e)
                }
            }
            other => Ok(other),
        }
    }

    fn unexpected(&mut self, op: &str) -> WineError {
        self.fatal(WineError::Protocol(format!(
            "unexpected response type to {op}"
        )))
    }

    /// Liveness check.
    pub fn ping(&mut self) -> Result<(), WineError> {
        match self.request("Ping", Request::Ping)? {
            Response::Ok => Ok(()),
            _ => Err(self.unexpected("Ping")),
        }
    }

    /// `LoadLibrary(name)` in the helper. The handle is session- and
    /// provider-scoped; the helper's module address is never visible.
    pub fn load_library(&mut self, name: &str) -> Result<Handle, WineError> {
        match self.request("LoadLibrary", Request::LoadLibrary { name: name.into() })? {
            Response::Library(remote) => {
                let h = self.handles.mint(self.opts.provider, "wine-library");
                self.libs.insert(h.value(), remote);
                Ok(h)
            }
            _ => Err(self.unexpected("LoadLibrary")),
        }
    }

    /// `GetProcAddress` by name or ordinal.
    pub fn get_proc(&mut self, lib: &Handle, proc: ProcRef) -> Result<Handle, WineError> {
        let kind = self.handles.resolve(self.opts.provider, lib)?;
        let remote = match (kind, self.libs.get(&lib.value())) {
            ("wine-library", Some(r)) => *r,
            _ => return Err(WineError::InvalidHandle("not a live library handle".into())),
        };
        match self.request("GetProcAddress", Request::GetProc { lib: remote, proc })? {
            Response::Proc(p) => {
                let h = self.handles.mint(self.opts.provider, "wine-proc");
                self.procs.insert(h.value(), (lib.value(), p));
                Ok(h)
            }
            _ => Err(self.unexpected("GetProcAddress")),
        }
    }

    /// Calls a procedure. Integer arguments must fit the helper's pointer
    /// width (as signed or unsigned), otherwise the request is rejected
    /// locally. Pointers are never passed: buffers are copied.
    pub fn call(
        &mut self,
        proc: &Handle,
        conv: WireConv,
        want: RetKind,
        args: &[WireArg],
    ) -> Result<CallReply, WineError> {
        let kind = self.handles.resolve(self.opts.provider, proc)?;
        let remote = match (kind, self.procs.get(&proc.value())) {
            ("wine-proc", Some((_, r))) => *r,
            _ => {
                return Err(WineError::InvalidHandle(
                    "not a live procedure handle".into(),
                ));
            }
        };
        if args.len() > MAX_ARGS {
            return Err(WineError::BadRequest(format!(
                "{} arguments exceed the limit of {MAX_ARGS}",
                args.len()
            )));
        }
        if self.info.bitness == 32 {
            for (i, a) in args.iter().enumerate() {
                if let WireArg::Int(v) = a
                    && i32::try_from(*v).is_err()
                    && u32::try_from(*v).is_err()
                {
                    return Err(WineError::BadRequest(format!(
                        "argument {i} ({v}) does not fit a 32-bit helper"
                    )));
                }
            }
        }
        match self.request(
            "Call",
            Request::Call {
                proc: remote,
                conv,
                ret: want,
                args: args.to_vec(),
            },
        )? {
            Response::CallResult {
                ret,
                last_error,
                buffers,
            } => {
                // Validate what came back against what was asked.
                for (i, _) in &buffers {
                    let ok = matches!(
                        args.get(*i as usize),
                        Some(WireArg::BufInOut(_) | WireArg::BufOut(_))
                    );
                    if !ok {
                        let e = WineError::Protocol(format!(
                            "helper returned a buffer for argument {i}, which is not an out buffer"
                        ));
                        return Err(self.fatal(e));
                    }
                }
                let ret_ok = matches!(
                    (want, &ret),
                    (RetKind::Void, WireRet::Void)
                        | (RetKind::Int, WireRet::Int(_))
                        | (RetKind::F32 | RetKind::F64, WireRet::Float(_))
                );
                if !ret_ok {
                    let e = WineError::Protocol(format!(
                        "helper returned {ret:?} for a call declared {want:?}"
                    ));
                    return Err(self.fatal(e));
                }
                Ok(CallReply {
                    ret,
                    last_error,
                    buffers,
                })
            }
            _ => Err(self.unexpected("Call")),
        }
    }

    /// `FreeLibrary`; its procedure handles become invalid.
    pub fn free_library(&mut self, lib: &Handle) -> Result<(), WineError> {
        self.handles.resolve(self.opts.provider, lib)?;
        let remote = *self
            .libs
            .get(&lib.value())
            .ok_or_else(|| WineError::InvalidHandle("not a live library handle".into()))?;
        match self.request("FreeLibrary", Request::FreeLibrary { lib: remote })? {
            Response::Ok => {
                self.libs.remove(&lib.value());
                let dead: Vec<u64> = self
                    .procs
                    .iter()
                    .filter(|(_, (l, _))| *l == lib.value())
                    .map(|(k, _)| *k)
                    .collect();
                for k in dead {
                    self.procs.remove(&k);
                }
                self.handles.close(self.opts.provider, lib)?;
                Ok(())
            }
            _ => Err(self.unexpected("FreeLibrary")),
        }
    }

    /// Asks the helper to exit, then reaps it. The session is closed either way.
    pub fn shutdown(mut self) -> Result<(), WineError> {
        let r = match self.request("Shutdown", Request::Shutdown) {
            Ok(Response::Ok) => Ok(()),
            Ok(_) => Err(WineError::Protocol(
                "unexpected response to Shutdown".into(),
            )),
            Err(e) => Err(e),
        };
        self.closed = true;
        self.transport.kill();
        r
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.transport.kill();
    }
}

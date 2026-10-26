//! The helper side of the protocol: a request dispatcher over a [`Backend`].
//!
//! The helper binary (`vba-wine-host`) implements [`Backend`] (really loading
//! libraries on Windows, or answering `Unsupported` in protocol-only mode); the
//! tests implement it with mocks. [`Server`] owns everything that must be true
//! regardless of backend: Hello first, version check, session-scoped opaque
//! ids (the backend's raw module/procedure addresses never leave this process),
//! argument and buffer limits, and strict request validation.

use crate::protocol::{
    HelloInfo, MAX_ARGS, MAX_FRAME, PROTOCOL_VERSION, ProcRef, ProtocolError, RemoteError,
    RemoteErrorKind, Request, Response, RetKind, WireArg, WireConv, WireRet, read_frame,
    write_frame,
};
use std::collections::HashMap;
use std::io::{Read, Write};

/// What actually loads libraries and makes calls.
///
/// Backend ids are raw (module handles, procedure addresses) and stay inside
/// the helper; [`Server`] maps them to opaque wire ids.
pub trait Backend {
    /// Static description reported in the Hello response (its `version` field
    /// is what the helper claims to speak).
    fn info(&self) -> HelloInfo;
    /// Loads a library; returns a raw backend id.
    fn load_library(&mut self, name: &str) -> Result<u64, RemoteError>;
    /// Resolves a procedure in a backend library id.
    fn get_proc(&mut self, lib: u64, proc: &ProcRef) -> Result<u64, RemoteError>;
    /// Calls a backend procedure id. Buffer arguments are `BufIn`/`BufInOut`
    /// (`BufOut` has already been expanded to zero-filled `BufInOut`); the
    /// backend writes results back into the `BufInOut` vectors. Returns the
    /// value and `GetLastError`.
    fn call(
        &mut self,
        proc: u64,
        conv: WireConv,
        ret: RetKind,
        args: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError>;
    /// Frees a backend library id.
    fn free_library(&mut self, lib: u64) -> Result<(), RemoteError>;
}

impl<B: Backend + ?Sized> Backend for Box<B> {
    fn info(&self) -> HelloInfo {
        (**self).info()
    }
    fn load_library(&mut self, name: &str) -> Result<u64, RemoteError> {
        (**self).load_library(name)
    }
    fn get_proc(&mut self, lib: u64, proc: &ProcRef) -> Result<u64, RemoteError> {
        (**self).get_proc(lib, proc)
    }
    fn call(
        &mut self,
        proc: u64,
        conv: WireConv,
        ret: RetKind,
        args: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError> {
        (**self).call(proc, conv, ret, args)
    }
    fn free_library(&mut self, lib: u64) -> Result<(), RemoteError> {
        (**self).free_library(lib)
    }
}

/// Dispatches decoded requests to a backend.
pub struct Server<B> {
    backend: B,
    greeted: bool,
    stopped: bool,
    next_id: u64,
    libs: HashMap<u64, u64>,
    procs: HashMap<u64, (u64, u64)>,
}

fn err(kind: RemoteErrorKind, msg: impl Into<String>) -> Response {
    Response::Error(RemoteError::new(kind, msg))
}

impl<B: Backend> Server<B> {
    /// Creates a server in the "waiting for Hello" state.
    pub fn new(backend: B) -> Self {
        Server {
            backend,
            greeted: false,
            stopped: false,
            next_id: 1,
            libs: HashMap::new(),
            procs: HashMap::new(),
        }
    }

    /// Whether Shutdown was processed.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// The backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    fn mint(&mut self) -> u64 {
        // Opaque, never reused, never zero, not an address of anything.
        let id = self.next_id;
        self.next_id += 1;
        id * 8 + 0x1000
    }

    /// Handles one request.
    pub fn handle(&mut self, req: Request) -> Response {
        if !self.greeted && !matches!(req, Request::Hello { .. }) {
            return err(RemoteErrorKind::ProtocolViolation, "request before Hello");
        }
        match req {
            Request::Hello { version, .. } => {
                if self.greeted {
                    return err(RemoteErrorKind::ProtocolViolation, "duplicate Hello");
                }
                if version != PROTOCOL_VERSION {
                    return Response::Error(RemoteError {
                        kind: RemoteErrorKind::VersionMismatch,
                        code: u32::from(PROTOCOL_VERSION),
                        message: format!(
                            "helper speaks protocol {PROTOCOL_VERSION}, client sent {version}"
                        ),
                    });
                }
                self.greeted = true;
                Response::Hello(self.backend.info())
            }
            Request::Ping => Response::Ok,
            Request::LoadLibrary { name } => match self.backend.load_library(&name) {
                Ok(raw) => {
                    let id = self.mint();
                    self.libs.insert(id, raw);
                    Response::Library(id)
                }
                Err(e) => Response::Error(e),
            },
            Request::GetProc { lib, proc } => {
                let Some(&raw_lib) = self.libs.get(&lib) else {
                    return err(RemoteErrorKind::InvalidHandle, "unknown library id");
                };
                match self.backend.get_proc(raw_lib, &proc) {
                    Ok(raw) => {
                        let id = self.mint();
                        self.procs.insert(id, (lib, raw));
                        Response::Proc(id)
                    }
                    Err(e) => Response::Error(e),
                }
            }
            Request::Call {
                proc,
                conv,
                ret,
                mut args,
            } => {
                let Some(&(_, raw)) = self.procs.get(&proc) else {
                    return err(RemoteErrorKind::InvalidHandle, "unknown procedure id");
                };
                if args.len() > MAX_ARGS {
                    return err(RemoteErrorKind::CallFault, "too many arguments");
                }
                // Expand Out buffers and bound the total returned size.
                let mut back = Vec::new();
                let mut total = 0usize;
                for (i, a) in args.iter_mut().enumerate() {
                    match a {
                        WireArg::BufOut(n) => {
                            let n = *n as usize;
                            total = total.saturating_add(n);
                            if total > MAX_FRAME / 2 {
                                return err(RemoteErrorKind::CallFault, "out buffers too large");
                            }
                            *a = WireArg::BufInOut(vec![0; n]);
                            back.push(i);
                        }
                        WireArg::BufInOut(b) => {
                            total = total.saturating_add(b.len());
                            if total > MAX_FRAME / 2 {
                                return err(RemoteErrorKind::CallFault, "buffers too large");
                            }
                            back.push(i);
                        }
                        _ => {}
                    }
                }
                match self.backend.call(raw, conv, ret, &mut args) {
                    Ok((ret, last_error)) => {
                        let buffers = back
                            .into_iter()
                            .filter_map(|i| match &mut args[i] {
                                WireArg::BufInOut(b) => Some((i as u32, std::mem::take(b))),
                                _ => None,
                            })
                            .collect();
                        Response::CallResult {
                            ret,
                            last_error,
                            buffers,
                        }
                    }
                    Err(e) => Response::Error(e),
                }
            }
            Request::FreeLibrary { lib } => {
                let Some(raw) = self.libs.remove(&lib) else {
                    return err(RemoteErrorKind::InvalidHandle, "unknown library id");
                };
                self.procs.retain(|_, (l, _)| *l != lib);
                match self.backend.free_library(raw) {
                    Ok(()) => Response::Ok,
                    Err(e) => Response::Error(e),
                }
            }
            Request::Shutdown => {
                self.stopped = true;
                Response::Ok
            }
        }
    }

    /// Decodes a frame body, handles it and returns the encoded response body.
    /// A malformed body yields a `ProtocolViolation` error response (id 0) and
    /// the error, so the caller can stop serving.
    pub fn handle_frame(&mut self, body: &[u8]) -> (Vec<u8>, Option<ProtocolError>) {
        match Request::decode(body) {
            Ok((id, req)) => {
                let resp = self.handle(req);
                let mut out = resp.encode(id);
                if out.len() > MAX_FRAME {
                    out = err(RemoteErrorKind::CallFault, "response too large").encode(id);
                }
                (out, None)
            }
            Err(e) => (
                err(RemoteErrorKind::ProtocolViolation, e.to_string()).encode(0),
                Some(e),
            ),
        }
    }

    /// Serves frames from `r` to `w` until Shutdown, EOF or a protocol error.
    pub fn serve<Rd: Read, Wr: Write>(
        &mut self,
        r: &mut Rd,
        w: &mut Wr,
    ) -> Result<(), ProtocolError> {
        while !self.stopped {
            let Some(body) = read_frame(r)? else {
                return Ok(());
            };
            let (out, bad) = self.handle_frame(&body);
            write_frame(w, &out).map_err(|e| ProtocolError::Io(e.to_string()))?;
            if let Some(e) = bad {
                return Err(e);
            }
        }
        Ok(())
    }
}

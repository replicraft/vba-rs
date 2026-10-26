//! The wire protocol shared by the native client and the Windows helper.
//!
//! Framing: a `u32` little-endian length followed by that many body bytes.
//! Body: `tag: u8`, `request id: u64`, then tag-specific fields. All integers
//! are little-endian; strings are `u16` length + UTF-8. Responses echo the
//! request id. Frames above [`MAX_FRAME`] are rejected before any allocation.
//!
//! Raw pointers never appear: buffers are copied by value ([`WireArg`]) and
//! libraries/procedures are session-scoped opaque ids minted by the helper.
//! The decoder is total: any byte string yields a value or a [`ProtocolError`],
//! never a panic.

use std::io::{self, Read, Write};
use thiserror::Error;

/// Protocol version spoken by this build. Bumped on any incompatible change.
pub const PROTOCOL_VERSION: u16 = 1;
/// Largest accepted frame body (16 MiB).
pub const MAX_FRAME: usize = 16 * 1024 * 1024;
/// Longest accepted string in bytes.
pub const MAX_STRING: usize = 4096;
/// Most arguments in one call (VBA allows 60 parameters).
pub const MAX_ARGS: usize = 60;
/// Feature bit: the helper can really load libraries and call them.
pub const CAP_NATIVE_CALLS: u32 = 1;

/// A malformed or oversized frame.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProtocolError {
    /// The frame length exceeds [`MAX_FRAME`].
    #[error("frame of {0} bytes exceeds the {MAX_FRAME} byte limit")]
    FrameTooLarge(usize),
    /// The stream ended inside a frame.
    #[error("stream ended inside a frame")]
    Truncated,
    /// The body ended before all fields were read.
    #[error("message body too short")]
    ShortBody,
    /// Bytes remain after the last field.
    #[error("{0} trailing bytes after message")]
    Trailing(usize),
    /// Unknown message tag.
    #[error("unknown message tag {0:#04x}")]
    UnknownTag(u8),
    /// Unknown enum discriminant inside a message.
    #[error("invalid {what} value {value}")]
    BadValue {
        /// Which field.
        what: &'static str,
        /// The value found.
        value: u64,
    },
    /// A string is not UTF-8 or too long.
    #[error("invalid string field")]
    BadString,
    /// A count field exceeds its limit.
    #[error("{what} count {count} exceeds limit {limit}")]
    TooMany {
        /// Which collection.
        what: &'static str,
        /// Count claimed.
        count: usize,
        /// Limit.
        limit: usize,
    },
    /// An I/O error other than clean EOF.
    #[error("i/o error: {0}")]
    Io(String),
}

/// Calling convention requested for a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireConv {
    /// `__stdcall` (the only convention on 64-bit).
    Stdcall,
    /// `__cdecl`.
    Cdecl,
}

/// How the return value is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetKind {
    /// `Sub`.
    Void,
    /// Integer class, pointer width, returned unnarrowed.
    Int,
    /// `Single` (`xmm0` / `st0`).
    F32,
    /// `Double`.
    F64,
}

/// A call argument. Buffers are copied; the helper passes the address of its
/// own copy and the client never learns it.
#[derive(Debug, Clone, PartialEq)]
pub enum WireArg {
    /// Integer passed by value, truncated to the helper's pointer width.
    Int(i64),
    /// `Single` by value.
    F32(f32),
    /// `Double` by value.
    F64(f64),
    /// Read-only buffer; the helper passes a pointer to a copy.
    BufIn(Vec<u8>),
    /// Buffer copied in and returned after the call.
    BufInOut(Vec<u8>),
    /// Zero-filled buffer of this length, returned after the call.
    BufOut(u32),
}

/// A return value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WireRet {
    /// No value.
    Void,
    /// Integer class, zero- or sign-extended per the helper's width.
    Int(i64),
    /// Floating point.
    Float(f64),
}

/// A procedure reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcRef {
    /// By export name.
    Name(String),
    /// By ordinal.
    Ordinal(u16),
}

/// Failure categories that the helper can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemoteErrorKind {
    /// The peer violated the protocol (for example a request before Hello).
    ProtocolViolation,
    /// `LoadLibrary` failed.
    LibraryNotFound,
    /// `GetProcAddress` failed.
    SymbolNotFound,
    /// The helper refused or could not perform the call (bad buffer, too many
    /// arguments, value out of range for the helper's width).
    CallFault,
    /// The helper's own policy refused the request.
    PermissionDenied,
    /// Not supported by this helper (for example protocol-only mode).
    Unsupported,
    /// Unknown or freed library/procedure id.
    InvalidHandle,
    /// Protocol versions differ.
    VersionMismatch,
}

/// A structured error carried in a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteError {
    /// Category.
    pub kind: RemoteErrorKind,
    /// OS error code when there is one (`GetLastError`), else 0.
    pub code: u32,
    /// Human-readable detail.
    pub message: String,
}

impl RemoteError {
    /// Builds an error without an OS code.
    pub fn new(kind: RemoteErrorKind, message: impl Into<String>) -> Self {
        RemoteError {
            kind,
            code: 0,
            message: message.into(),
        }
    }
}

/// What the helper reports about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelloInfo {
    /// Protocol version the helper speaks.
    pub version: u16,
    /// Pointer width of the helper process: 32 or 64.
    pub bitness: u8,
    /// `"windows"` for a PE build, otherwise the host OS name (protocol-only).
    pub os: String,
    /// Wine version when running under Wine (`wine_get_version`), else empty.
    pub wine_version: String,
    /// Helper build identification.
    pub helper: String,
    /// Feature bits ([`CAP_NATIVE_CALLS`]).
    pub caps: u32,
}

/// A request from the native side.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// First message: version negotiation. `session` is chosen by the client.
    Hello {
        /// Client protocol version.
        version: u16,
        /// Client-chosen session id (informational).
        session: u64,
    },
    /// Liveness probe.
    Ping,
    /// `LoadLibrary(name)`.
    LoadLibrary {
        /// Library name or path as the helper sees it.
        name: String,
    },
    /// `GetProcAddress(lib, proc)`.
    GetProc {
        /// Library id from [`Response::Library`].
        lib: u64,
        /// Name or ordinal.
        proc: ProcRef,
    },
    /// Calls a procedure.
    Call {
        /// Procedure id from [`Response::Proc`].
        proc: u64,
        /// Convention.
        conv: WireConv,
        /// Return kind.
        ret: RetKind,
        /// Arguments.
        args: Vec<WireArg>,
    },
    /// `FreeLibrary`; invalidates its procedure ids.
    FreeLibrary {
        /// Library id.
        lib: u64,
    },
    /// Ends the session; the helper answers [`Response::Ok`] and exits.
    Shutdown,
}

/// A response from the helper.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// Answer to Hello.
    Hello(HelloInfo),
    /// Generic success (Ping, FreeLibrary, Shutdown).
    Ok,
    /// A loaded library id.
    Library(u64),
    /// A procedure id.
    Proc(u64),
    /// A completed call.
    CallResult {
        /// Return value.
        ret: WireRet,
        /// `GetLastError` captured right after the call.
        last_error: u32,
        /// `(argument index, bytes)` for every `BufInOut` / `BufOut` argument.
        buffers: Vec<(u32, Vec<u8>)>,
    },
    /// A failure.
    Error(RemoteError),
}

// ---------------------------------------------------------------- encoding

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        let b = s.as_bytes();
        let b = &b[..floor_char_boundary(s, b.len().min(MAX_STRING))];
        self.u16(b.len() as u16);
        self.0.extend_from_slice(b);
    }
    fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
}

fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

struct R<'a>(&'a [u8]);

impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ProtocolError> {
        if self.0.len() < n {
            return Err(ProtocolError::ShortBody);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn str(&mut self) -> Result<String, ProtocolError> {
        let n = self.u16()? as usize;
        if n > MAX_STRING {
            return Err(ProtocolError::BadString);
        }
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| ProtocolError::BadString)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, ProtocolError> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    fn finish(self) -> Result<(), ProtocolError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ProtocolError::Trailing(self.0.len()))
        }
    }
}

fn bad(what: &'static str, value: u64) -> ProtocolError {
    ProtocolError::BadValue { what, value }
}

impl WireConv {
    fn code(self) -> u8 {
        match self {
            WireConv::Stdcall => 0,
            WireConv::Cdecl => 1,
        }
    }
    fn from_code(c: u8) -> Result<Self, ProtocolError> {
        match c {
            0 => Ok(WireConv::Stdcall),
            1 => Ok(WireConv::Cdecl),
            _ => Err(bad("calling convention", c.into())),
        }
    }
}

impl RetKind {
    fn code(self) -> u8 {
        match self {
            RetKind::Void => 0,
            RetKind::Int => 1,
            RetKind::F32 => 2,
            RetKind::F64 => 3,
        }
    }
    fn from_code(c: u8) -> Result<Self, ProtocolError> {
        match c {
            0 => Ok(RetKind::Void),
            1 => Ok(RetKind::Int),
            2 => Ok(RetKind::F32),
            3 => Ok(RetKind::F64),
            _ => Err(bad("return kind", c.into())),
        }
    }
}

impl RemoteErrorKind {
    const ALL: [RemoteErrorKind; 8] = [
        RemoteErrorKind::ProtocolViolation,
        RemoteErrorKind::LibraryNotFound,
        RemoteErrorKind::SymbolNotFound,
        RemoteErrorKind::CallFault,
        RemoteErrorKind::PermissionDenied,
        RemoteErrorKind::Unsupported,
        RemoteErrorKind::InvalidHandle,
        RemoteErrorKind::VersionMismatch,
    ];
    fn code(self) -> u8 {
        Self::ALL.iter().position(|k| *k == self).unwrap() as u8
    }
    fn from_code(c: u8) -> Result<Self, ProtocolError> {
        Self::ALL
            .get(c as usize)
            .copied()
            .ok_or_else(|| bad("error kind", c.into()))
    }
}

fn put_arg(w: &mut W, a: &WireArg) {
    match a {
        WireArg::Int(v) => {
            w.u8(0);
            w.u64(*v as u64);
        }
        WireArg::F32(v) => {
            w.u8(1);
            w.u32(v.to_bits());
        }
        WireArg::F64(v) => {
            w.u8(2);
            w.u64(v.to_bits());
        }
        WireArg::BufIn(b) => {
            w.u8(3);
            w.bytes(b);
        }
        WireArg::BufInOut(b) => {
            w.u8(4);
            w.bytes(b);
        }
        WireArg::BufOut(n) => {
            w.u8(5);
            w.u32(*n);
        }
    }
}

fn get_arg(r: &mut R) -> Result<WireArg, ProtocolError> {
    Ok(match r.u8()? {
        0 => WireArg::Int(r.u64()? as i64),
        1 => WireArg::F32(f32::from_bits(r.u32()?)),
        2 => WireArg::F64(f64::from_bits(r.u64()?)),
        3 => WireArg::BufIn(r.bytes()?),
        4 => WireArg::BufInOut(r.bytes()?),
        5 => WireArg::BufOut(r.u32()?),
        t => return Err(bad("argument kind", t.into())),
    })
}

impl Request {
    /// Encodes the body (without the length prefix) for request `id`.
    pub fn encode(&self, id: u64) -> Vec<u8> {
        let mut w = W(Vec::new());
        match self {
            Request::Hello { version, session } => {
                w.u8(1);
                w.u64(id);
                w.u16(*version);
                w.u64(*session);
            }
            Request::Ping => {
                w.u8(2);
                w.u64(id);
            }
            Request::LoadLibrary { name } => {
                w.u8(3);
                w.u64(id);
                w.str(name);
            }
            Request::GetProc { lib, proc } => {
                w.u8(4);
                w.u64(id);
                w.u64(*lib);
                match proc {
                    ProcRef::Name(n) => {
                        w.u8(0);
                        w.str(n);
                    }
                    ProcRef::Ordinal(o) => {
                        w.u8(1);
                        w.u16(*o);
                    }
                }
            }
            Request::Call {
                proc,
                conv,
                ret,
                args,
            } => {
                w.u8(5);
                w.u64(id);
                w.u64(*proc);
                w.u8(conv.code());
                w.u8(ret.code());
                w.u16(args.len() as u16);
                for a in args {
                    put_arg(&mut w, a);
                }
            }
            Request::FreeLibrary { lib } => {
                w.u8(6);
                w.u64(id);
                w.u64(*lib);
            }
            Request::Shutdown => {
                w.u8(7);
                w.u64(id);
            }
        }
        w.0
    }

    /// Decodes a body into `(request id, request)`.
    pub fn decode(body: &[u8]) -> Result<(u64, Request), ProtocolError> {
        let mut r = R(body);
        let tag = r.u8()?;
        let id = r.u64()?;
        let req = match tag {
            1 => Request::Hello {
                version: r.u16()?,
                session: r.u64()?,
            },
            2 => Request::Ping,
            3 => Request::LoadLibrary { name: r.str()? },
            4 => {
                let lib = r.u64()?;
                let proc = match r.u8()? {
                    0 => ProcRef::Name(r.str()?),
                    1 => ProcRef::Ordinal(r.u16()?),
                    t => return Err(bad("proc ref", t.into())),
                };
                Request::GetProc { lib, proc }
            }
            5 => {
                let proc = r.u64()?;
                let conv = WireConv::from_code(r.u8()?)?;
                let ret = RetKind::from_code(r.u8()?)?;
                let n = r.u16()? as usize;
                if n > MAX_ARGS {
                    return Err(ProtocolError::TooMany {
                        what: "argument",
                        count: n,
                        limit: MAX_ARGS,
                    });
                }
                let mut args = Vec::with_capacity(n);
                for _ in 0..n {
                    args.push(get_arg(&mut r)?);
                }
                Request::Call {
                    proc,
                    conv,
                    ret,
                    args,
                }
            }
            6 => Request::FreeLibrary { lib: r.u64()? },
            7 => Request::Shutdown,
            t => return Err(ProtocolError::UnknownTag(t)),
        };
        r.finish()?;
        Ok((id, req))
    }
}

impl Response {
    /// Encodes the body (without the length prefix) answering request `id`.
    pub fn encode(&self, id: u64) -> Vec<u8> {
        let mut w = W(Vec::new());
        match self {
            Response::Hello(h) => {
                w.u8(0x81);
                w.u64(id);
                w.u16(h.version);
                w.u8(h.bitness);
                w.str(&h.os);
                w.str(&h.wine_version);
                w.str(&h.helper);
                w.u32(h.caps);
            }
            Response::Ok => {
                w.u8(0x82);
                w.u64(id);
            }
            Response::Library(l) => {
                w.u8(0x83);
                w.u64(id);
                w.u64(*l);
            }
            Response::Proc(p) => {
                w.u8(0x84);
                w.u64(id);
                w.u64(*p);
            }
            Response::CallResult {
                ret,
                last_error,
                buffers,
            } => {
                w.u8(0x85);
                w.u64(id);
                match ret {
                    WireRet::Void => w.u8(0),
                    WireRet::Int(v) => {
                        w.u8(1);
                        w.u64(*v as u64);
                    }
                    WireRet::Float(v) => {
                        w.u8(2);
                        w.u64(v.to_bits());
                    }
                }
                w.u32(*last_error);
                w.u16(buffers.len() as u16);
                for (i, b) in buffers {
                    w.u32(*i);
                    w.bytes(b);
                }
            }
            Response::Error(e) => {
                w.u8(0x86);
                w.u64(id);
                w.u8(e.kind.code());
                w.u32(e.code);
                w.str(&e.message);
            }
        }
        w.0
    }

    /// Decodes a body into `(request id, response)`.
    pub fn decode(body: &[u8]) -> Result<(u64, Response), ProtocolError> {
        let mut r = R(body);
        let tag = r.u8()?;
        let id = r.u64()?;
        let resp = match tag {
            0x81 => Response::Hello(HelloInfo {
                version: r.u16()?,
                bitness: r.u8()?,
                os: r.str()?,
                wine_version: r.str()?,
                helper: r.str()?,
                caps: r.u32()?,
            }),
            0x82 => Response::Ok,
            0x83 => Response::Library(r.u64()?),
            0x84 => Response::Proc(r.u64()?),
            0x85 => {
                let ret = match r.u8()? {
                    0 => WireRet::Void,
                    1 => WireRet::Int(r.u64()? as i64),
                    2 => WireRet::Float(f64::from_bits(r.u64()?)),
                    t => return Err(bad("return value", t.into())),
                };
                let last_error = r.u32()?;
                let n = r.u16()? as usize;
                if n > MAX_ARGS {
                    return Err(ProtocolError::TooMany {
                        what: "buffer",
                        count: n,
                        limit: MAX_ARGS,
                    });
                }
                let mut buffers = Vec::with_capacity(n);
                for _ in 0..n {
                    let i = r.u32()?;
                    buffers.push((i, r.bytes()?));
                }
                Response::CallResult {
                    ret,
                    last_error,
                    buffers,
                }
            }
            0x86 => Response::Error(RemoteError {
                kind: RemoteErrorKind::from_code(r.u8()?)?,
                code: r.u32()?,
                message: r.str()?,
            }),
            t => return Err(ProtocolError::UnknownTag(t)),
        };
        r.finish()?;
        Ok((id, resp))
    }
}

// ----------------------------------------------------------------- framing

/// Writes one frame (length prefix + body) and flushes.
pub fn write_frame<Wr: Write>(w: &mut Wr, body: &[u8]) -> io::Result<()> {
    if body.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            ProtocolError::FrameTooLarge(body.len()),
        ));
    }
    w.write_all(&(body.len() as u32).to_le_bytes())?;
    w.write_all(body)?;
    w.flush()
}

/// Reads one frame body. `Ok(None)` is a clean EOF at a frame boundary. The
/// length is checked against [`MAX_FRAME`] before the body is allocated, and
/// the body is read incrementally so a lying length cannot force a large
/// allocation.
pub fn read_frame<Rd: Read>(r: &mut Rd) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut len = [0u8; 4];
    let mut got = 0;
    while got < 4 {
        match r.read(&mut len[got..]) {
            Ok(0) if got == 0 => return Ok(None),
            Ok(0) => return Err(ProtocolError::Truncated),
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(ProtocolError::Io(e.to_string())),
        }
    }
    let n = u32::from_le_bytes(len) as usize;
    if n > MAX_FRAME {
        return Err(ProtocolError::FrameTooLarge(n));
    }
    let mut body = Vec::new();
    r.take(n as u64)
        .read_to_end(&mut body)
        .map_err(|e| ProtocolError::Io(e.to_string()))?;
    if body.len() < n {
        return Err(ProtocolError::Truncated);
    }
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_requests() -> Vec<Request> {
        vec![
            Request::Hello {
                version: 1,
                session: 9,
            },
            Request::Ping,
            Request::LoadLibrary {
                name: "kernel32".into(),
            },
            Request::GetProc {
                lib: 7,
                proc: ProcRef::Name("GetTickCount".into()),
            },
            Request::GetProc {
                lib: 7,
                proc: ProcRef::Ordinal(12),
            },
            Request::Call {
                proc: 3,
                conv: WireConv::Cdecl,
                ret: RetKind::F64,
                args: vec![
                    WireArg::Int(-5),
                    WireArg::F32(1.5),
                    WireArg::F64(-2.25),
                    WireArg::BufIn(vec![1, 2]),
                    WireArg::BufInOut(vec![]),
                    WireArg::BufOut(16),
                ],
            },
            Request::FreeLibrary { lib: 1 },
            Request::Shutdown,
        ]
    }

    #[test]
    fn request_round_trip() {
        for (i, r) in sample_requests().into_iter().enumerate() {
            let (id, back) = Request::decode(&r.encode(i as u64)).unwrap();
            assert_eq!(id, i as u64);
            assert_eq!(back, r);
        }
    }

    #[test]
    fn response_round_trip() {
        let all = vec![
            Response::Hello(HelloInfo {
                version: 1,
                bitness: 64,
                os: "windows".into(),
                wine_version: "9.0".into(),
                helper: "h".into(),
                caps: CAP_NATIVE_CALLS,
            }),
            Response::Ok,
            Response::Library(5),
            Response::Proc(6),
            Response::CallResult {
                ret: WireRet::Float(0.5),
                last_error: 5,
                buffers: vec![(1, vec![9, 9])],
            },
            Response::CallResult {
                ret: WireRet::Int(-1),
                last_error: 0,
                buffers: vec![],
            },
            Response::CallResult {
                ret: WireRet::Void,
                last_error: 0,
                buffers: vec![],
            },
        ]
        .into_iter()
        .chain(RemoteErrorKind::ALL.map(|kind| {
            Response::Error(RemoteError {
                kind,
                code: 2,
                message: "x".into(),
            })
        }));
        for r in all {
            let (_, back) = Response::decode(&r.encode(1)).unwrap();
            assert_eq!(back, r);
        }
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        for r in sample_requests() {
            let b = r.encode(1);
            for cut in 0..b.len() {
                assert!(Request::decode(&b[..cut]).is_err(), "{r:?} cut {cut}");
            }
        }
    }

    #[test]
    fn trailing_bytes_and_unknown_tags_rejected() {
        let mut b = Request::Ping.encode(1);
        b.push(0);
        assert_eq!(Request::decode(&b), Err(ProtocolError::Trailing(1)));
        assert_eq!(
            Request::decode(&[0xEE, 0, 0, 0, 0, 0, 0, 0, 0]),
            Err(ProtocolError::UnknownTag(0xEE))
        );
        assert!(Response::decode(&[0x82]).is_err());
    }

    #[test]
    fn hostile_counts_do_not_allocate_or_panic() {
        // Call with 0xFFFF args.
        let mut b = vec![5];
        b.extend_from_slice(&1u64.to_le_bytes());
        b.extend_from_slice(&1u64.to_le_bytes());
        b.extend_from_slice(&[0, 0]);
        b.extend_from_slice(&0xFFFFu16.to_le_bytes());
        assert!(matches!(
            Request::decode(&b),
            Err(ProtocolError::TooMany { .. })
        ));
        // A buffer claiming 4 GiB.
        let mut b = vec![5];
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&[0, 0]);
        b.extend_from_slice(&1u16.to_le_bytes());
        b.push(3);
        b.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Request::decode(&b), Err(ProtocolError::ShortBody));
    }

    #[test]
    fn bad_utf8_and_overlong_strings() {
        let mut b = vec![3];
        b.extend_from_slice(&1u64.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&[0xFF, 0xFE]);
        assert_eq!(Request::decode(&b), Err(ProtocolError::BadString));
        let long = "a".repeat(MAX_STRING + 100);
        let enc = Request::LoadLibrary { name: long }.encode(1);
        let (_, Request::LoadLibrary { name }) = Request::decode(&enc).unwrap() else {
            panic!()
        };
        assert_eq!(name.len(), MAX_STRING);
    }

    #[test]
    fn frames() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"hello").unwrap();
        let mut c = io::Cursor::new(buf.clone());
        assert_eq!(read_frame(&mut c).unwrap().unwrap(), b"hello");
        assert_eq!(read_frame(&mut c).unwrap(), None);
        // Truncated body and truncated header.
        let mut c = io::Cursor::new(&buf[..buf.len() - 1]);
        assert_eq!(read_frame(&mut c), Err(ProtocolError::Truncated));
        let mut c = io::Cursor::new(&buf[..2]);
        assert_eq!(read_frame(&mut c), Err(ProtocolError::Truncated));
        // Oversized length is rejected without reading further.
        let huge = (MAX_FRAME as u32 + 1).to_le_bytes();
        let mut c = io::Cursor::new(huge.to_vec());
        assert_eq!(
            read_frame(&mut c),
            Err(ProtocolError::FrameTooLarge(MAX_FRAME + 1))
        );
        assert!(write_frame(&mut Vec::new(), &vec![0; MAX_FRAME + 1]).is_err());
    }

    #[test]
    fn pseudo_random_garbage_never_panics() {
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for len in 0..200 {
            let mut v = Vec::new();
            for _ in 0..len {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                v.push(x as u8);
            }
            let _ = Request::decode(&v);
            let _ = Response::decode(&v);
            // Force valid tags to exercise field decoding.
            for tag in (1..=7u8).chain(0x81..=0x86) {
                if !v.is_empty() {
                    v[0] = tag;
                    let _ = Request::decode(&v);
                    let _ = Response::decode(&v);
                }
            }
            let _ = read_frame(&mut io::Cursor::new(v));
        }
    }
}

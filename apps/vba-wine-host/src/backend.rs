//! Backend selection and the protocol-only backend.

use vba_wine::helper::Backend;
use vba_wine::protocol::{
    HelloInfo, PROTOCOL_VERSION, ProcRef, RemoteError, RemoteErrorKind, RetKind, WireArg, WireConv,
    WireRet,
};

/// Wraps a backend to override the advertised protocol version (test switch).
pub struct Advertise<B> {
    inner: B,
    version: Option<u16>,
}

impl<B: Backend> Backend for Advertise<B> {
    fn info(&self) -> HelloInfo {
        let mut i = self.inner.info();
        if let Some(v) = self.version {
            i.version = v;
        }
        i
    }
    fn load_library(&mut self, name: &str) -> Result<u64, RemoteError> {
        self.inner.load_library(name)
    }
    fn get_proc(&mut self, lib: u64, proc: &ProcRef) -> Result<u64, RemoteError> {
        self.inner.get_proc(lib, proc)
    }
    fn call(
        &mut self,
        proc: u64,
        conv: WireConv,
        ret: RetKind,
        args: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError> {
        self.inner.call(proc, conv, ret, args)
    }
    fn free_library(&mut self, lib: u64) -> Result<(), RemoteError> {
        self.inner.free_library(lib)
    }
}

/// Answers every library operation with `Unsupported`.
pub struct ProtocolOnly;

fn unsupported<T>() -> Result<T, RemoteError> {
    Err(RemoteError::new(
        RemoteErrorKind::Unsupported,
        "protocol-only helper: this build/host cannot load or call native libraries",
    ))
}

impl Backend for ProtocolOnly {
    fn info(&self) -> HelloInfo {
        HelloInfo {
            version: PROTOCOL_VERSION,
            bitness: if cfg!(target_pointer_width = "32") {
                32
            } else {
                64
            },
            os: std::env::consts::OS.to_string(),
            wine_version: String::new(),
            helper: format!(
                "vba-wine-host {} (protocol-only)",
                env!("CARGO_PKG_VERSION")
            ),
            caps: 0,
        }
    }
    fn load_library(&mut self, _: &str) -> Result<u64, RemoteError> {
        unsupported()
    }
    fn get_proc(&mut self, _: u64, _: &ProcRef) -> Result<u64, RemoteError> {
        unsupported()
    }
    fn call(
        &mut self,
        _: u64,
        _: WireConv,
        _: RetKind,
        _: &mut [WireArg],
    ) -> Result<(WireRet, u32), RemoteError> {
        unsupported()
    }
    fn free_library(&mut self, _: u64) -> Result<(), RemoteError> {
        unsupported()
    }
}

/// Builds the backend for this run.
#[cfg_attr(not(windows), allow(unused_variables))]
pub fn make(protocol_only: bool, advertise: Option<u16>) -> Advertise<Box<dyn Backend>> {
    #[cfg(windows)]
    if !protocol_only {
        return Advertise {
            inner: Box::new(crate::native::Native::new()),
            version: advertise,
        };
    }
    Advertise {
        inner: Box::new(ProtocolOnly),
        version: advertise,
    }
}

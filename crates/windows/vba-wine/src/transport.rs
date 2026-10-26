//! Transports: the real child-process pipes and an in-process loopback.

use crate::config::WineConfig;
use crate::error::WineError;
use crate::protocol::{ProtocolError, read_frame, write_frame};
use std::process::{Child, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

/// A frame-oriented channel to a helper. One request is in flight at a time;
/// the session guarantees that.
pub trait Transport: Send {
    /// Queues one frame body. Must not block on a stalled peer.
    fn send(&mut self, frame: Vec<u8>) -> Result<(), WineError>;
    /// Waits up to `timeout` for the next frame body.
    ///
    /// Returns `Ok(None)` on timeout (the session turns that into
    /// [`WineError::Timeout`]), `HelperCrashed` when the peer is gone and
    /// `Protocol` for an oversized frame.
    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, WineError>;
    /// Terminates the peer. Idempotent.
    fn kill(&mut self);
}

type Incoming = Result<Option<Vec<u8>>, ProtocolError>;

/// Pipes to a spawned helper process (Wine, or the helper run natively).
///
/// Dropping it kills and reaps the child.
pub struct ProcessTransport {
    child: Child,
    tx: Option<Sender<Vec<u8>>>,
    rx: Receiver<Incoming>,
    threads: Vec<JoinHandle<()>>,
}

impl ProcessTransport {
    /// Spawns the configured command with piped stdin/stdout. stderr is
    /// discarded (the helper must communicate only through frames).
    pub fn spawn(config: &WineConfig) -> Result<Self, WineError> {
        config.validate()?;
        let mut cmd = config.command();
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .map_err(|e| WineError::Unavailable(format!("cannot start helper: {e}")))?;
        let (Some(mut stdin), Some(mut stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(WineError::Transport("helper pipes unavailable".into()));
        };
        let (rtx, rx) = mpsc::channel::<Incoming>();
        let reader = std::thread::spawn(move || {
            loop {
                let r = read_frame(&mut stdout);
                let end = !matches!(r, Ok(Some(_)));
                if rtx.send(r).is_err() || end {
                    break;
                }
            }
        });
        let (wtx, wrx) = mpsc::channel::<Vec<u8>>();
        let writer = std::thread::spawn(move || {
            while let Ok(frame) = wrx.recv() {
                if write_frame(&mut stdin, &frame).is_err() {
                    break;
                }
            }
        });
        Ok(ProcessTransport {
            child,
            tx: Some(wtx),
            rx,
            threads: vec![reader, writer],
        })
    }

    fn exit_note(&mut self) -> String {
        match self.child.try_wait() {
            Ok(Some(status)) => format!("helper exited with {status}"),
            Ok(None) => "helper closed its output".to_string(),
            Err(e) => format!("helper state unknown: {e}"),
        }
    }
}

impl Transport for ProcessTransport {
    fn send(&mut self, frame: Vec<u8>) -> Result<(), WineError> {
        match &self.tx {
            Some(tx) if tx.send(frame).is_ok() => Ok(()),
            _ => Err(WineError::HelperCrashed(self.exit_note())),
        }
    }

    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, WineError> {
        match self.rx.recv_timeout(timeout) {
            Ok(Ok(Some(f))) => Ok(Some(f)),
            Ok(Ok(None)) | Err(RecvTimeoutError::Disconnected) => {
                // Give the OS a moment to publish the exit status.
                std::thread::sleep(Duration::from_millis(20));
                Err(WineError::HelperCrashed(self.exit_note()))
            }
            Ok(Err(e)) => Err(e.into()),
            Err(RecvTimeoutError::Timeout) => Ok(None),
        }
    }

    fn kill(&mut self) {
        self.tx = None;
        // Already-exited children make kill() fail; that is fine.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ProcessTransport {
    fn drop(&mut self) {
        self.kill();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

/// In-process loopback transport, for embedding a helper `Server` in the same
/// process (tests, mocks).
pub struct ChannelTransport {
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
    killed: bool,
}

/// The peer end of a [`ChannelTransport`].
pub struct ChannelPeer {
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl ChannelTransport {
    /// Creates a connected client end and peer end.
    pub fn pair() -> (ChannelTransport, ChannelPeer) {
        let (ctx, prx) = mpsc::channel();
        let (ptx, crx) = mpsc::channel();
        (
            ChannelTransport {
                tx: ctx,
                rx: crx,
                killed: false,
            },
            ChannelPeer { tx: ptx, rx: prx },
        )
    }
}

impl ChannelPeer {
    /// Next client frame; `None` when the client end is gone or killed.
    pub fn recv(&self) -> Option<Vec<u8>> {
        self.rx.recv().ok()
    }
    /// Sends a frame to the client; `false` if it is gone.
    pub fn send(&self, frame: Vec<u8>) -> bool {
        self.tx.send(frame).is_ok()
    }
}

impl Transport for ChannelTransport {
    fn send(&mut self, frame: Vec<u8>) -> Result<(), WineError> {
        if self.killed {
            return Err(WineError::HelperCrashed("peer was killed".into()));
        }
        self.tx
            .send(frame)
            .map_err(|_| WineError::HelperCrashed("peer is gone".into()))
    }

    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, WineError> {
        match self.rx.recv_timeout(timeout) {
            Ok(f) => Ok(Some(f)),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(WineError::HelperCrashed("peer is gone".into()))
            }
        }
    }

    fn kill(&mut self) {
        self.killed = true;
    }
}

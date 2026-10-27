//! [`InstanceThread`]: an [`Instance`] owned by a dedicated thread, driven
//! through a command channel.
//!
//! An [`Instance`] is built on `Rc` and `RefCell` and is not `Send`, so an
//! application with a multi-threaded runtime cannot move one between worker
//! threads. The pattern that works is to give the instance a thread of its
//! own and send it *commands*: closures that run on that thread, one at a
//! time, in the order they were submitted. This module is that pattern, on
//! `std` alone (one `std::thread`, two `std::sync::mpsc` channels per
//! command, no runtime, no new dependency).
//!
//! ```
//! use vba::thread::InstanceThread;
//! use vba::{Engine, Project, Variant};
//!
//! // The engine and the project are built *on* the interpreter thread: an
//! // `Engine` may hold non-`Send` host factories.
//! let worker = InstanceThread::spawn(|| {
//!     let project = Project::from_source(
//!         "Module1",
//!         "Public Function Twice(ByVal n As Long) As Long\n  Twice = n * 2\nEnd Function\n",
//!     );
//!     let mut instance = Engine::new().load(&project)?;
//!     instance.trust();
//!     Ok(instance)
//! })
//! .unwrap();
//!
//! // Commands are `Send` closures; their results must be `Send` too
//! // (`Variant`, `Outcome` and `Error` are).
//! let out = worker.call(|i| i.run("Twice", vec![Variant::Long(21)])).unwrap().unwrap();
//! assert_eq!(out.value, Variant::Long(42));
//!
//! // Another thread may cancel a running command through the token.
//! let token = worker.cancel_token();
//! assert!(!token.is_cancelled());
//!
//! worker.close().unwrap();
//! ```
//!
//! # Guarantees
//!
//! * **Order.** Commands run one at a time in the order `call` / `submit`
//!   returned for them; commands submitted from one thread run in that
//!   thread's order. Nothing runs concurrently with a command, so the
//!   instance needs no lock and its behavior is as deterministic as on a
//!   thread of your own.
//! * **No ambient state.** The thread owns the instance; the handle owns only
//!   a channel and the [`CancelToken`]. The handle is `Send + Sync`, so it
//!   may sit in an `Arc` shared by tasks of any runtime.
//! * **Blocking is yours to place.** [`call`](InstanceThread::call) blocks the
//!   calling thread until the command has finished. From an async runtime use
//!   [`submit`](InstanceThread::submit) and wait for the [`Reply`] on a
//!   blocking-capable thread (`spawn_blocking` or equivalent); this crate
//!   does not depend on any runtime.
//! * **Panics stop the thread, not the process.** A command that panics
//!   yields [`ThreadError::Panicked`]; the instance may be half updated, so
//!   the thread then stops and every later command fails with
//!   [`ThreadError::Stopped`].
//! * **Closing.** [`close`](InstanceThread::close) and dropping the handle
//!   finish the command that is running, skip the queued ones (their callers
//!   see [`ThreadError::Stopped`]) and drop the instance on its own thread,
//!   without running `Class_Terminate`. Call `Instance::shutdown` in a last
//!   command first when the program's terminators matter. Neither cancels the
//!   [`CancelToken`]: if the running command may be long, cancel first.
//!
//! Limits, the call timeout and cancellation are those of the instance; the
//! thread adds none. A command that blocks forever (a host member that never
//! returns) blocks the thread forever.

use std::any::Any;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::{CancelToken, Error, Instance};

/// One queued command: runs on the interpreter thread and returns whether
/// the thread may continue (`false` after a panic).
type Job = Box<dyn FnOnce(&mut Instance) -> bool + Send>;

/// Why a command could not be run or its result could not be delivered.
///
/// Marked non-exhaustive: match with a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ThreadError {
    /// The operating system refused to start the thread.
    #[error("could not start the interpreter thread: {0}")]
    Spawn(#[source] io::Error),
    /// The builder given to [`InstanceThread::spawn`] failed.
    #[error("could not build the instance: {0}")]
    Build(#[source] Error),
    /// The builder panicked, or a command did; the text is the panic message
    /// when it was a string. After this the thread is stopped.
    #[error("the interpreter thread panicked: {0}")]
    Panicked(String),
    /// The thread is closed or stopped, or the command was dropped unrun.
    #[error("the interpreter thread is stopped")]
    Stopped,
}

/// An [`Instance`] on a thread of its own. See the [module
/// documentation](self) for the pattern and its guarantees.
pub struct InstanceThread {
    // `Option` so that `close` and `Drop` can drop the sender before joining.
    commands: Option<Sender<Job>>,
    join: Mutex<Option<JoinHandle<()>>>,
    closing: Arc<AtomicBool>,
    cancel: CancelToken,
}

impl std::fmt::Debug for InstanceThread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceThread")
            .field("closing", &self.closing.load(Ordering::SeqCst))
            .field("cancelled", &self.cancel.is_cancelled())
            .finish_non_exhaustive()
    }
}

/// The pending result of a command submitted with
/// [`InstanceThread::submit`].
#[derive(Debug)]
pub struct Reply<R> {
    receiver: Receiver<Result<R, ThreadError>>,
}

impl<R> Reply<R> {
    /// Blocks until the command has finished and returns its result.
    pub fn wait(self) -> Result<R, ThreadError> {
        self.receiver.recv().unwrap_or(Err(ThreadError::Stopped))
    }

    /// The result if the command has finished, without blocking. `None`
    /// while it is queued or running. The result is delivered once: after
    /// it was returned, later calls report [`ThreadError::Stopped`].
    pub fn try_wait(&mut self) -> Option<Result<R, ThreadError>> {
        match self.receiver.try_recv() {
            Ok(r) => Some(r),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(ThreadError::Stopped)),
        }
    }
}

fn panic_text(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(s) => *s,
        Err(p) => match p.downcast::<&'static str>() {
            Ok(s) => (*s).to_owned(),
            Err(_) => "non-string panic payload".to_owned(),
        },
    }
}

impl InstanceThread {
    /// Starts the interpreter thread and builds the instance **on it** with
    /// `build`, then returns once the instance exists (or failed to). The
    /// builder is the place to create the [`Engine`](crate::Engine), the
    /// [`Project`](crate::Project) and any non-`Send` host objects.
    pub fn spawn(
        build: impl FnOnce() -> Result<Instance, Error> + Send + 'static,
    ) -> Result<InstanceThread, ThreadError> {
        InstanceThread::spawn_with_stack(None, build)
    }

    /// [`spawn`](InstanceThread::spawn) with an explicit stack size in bytes
    /// for the interpreter thread (`None`: the standard library default).
    /// The interpreter bounds its own native recursion
    /// (`VmConfig::max_native_stack_bytes`, 1 MiB by default), so the default
    /// suffices unless that is raised.
    pub fn spawn_with_stack(
        stack_size: Option<usize>,
        build: impl FnOnce() -> Result<Instance, Error> + Send + 'static,
    ) -> Result<InstanceThread, ThreadError> {
        let (commands, receiver) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<CancelToken, ThreadError>>();
        let closing = Arc::new(AtomicBool::new(false));
        let thread_closing = closing.clone();

        let mut builder = std::thread::Builder::new().name("vba-instance".to_owned());
        if let Some(size) = stack_size {
            builder = builder.stack_size(size);
        }
        let join = builder
            .spawn(move || {
                let built = catch_unwind(AssertUnwindSafe(build));
                let mut instance = match built {
                    Ok(Ok(instance)) => instance,
                    Ok(Err(e)) => {
                        let _ = ready_tx.send(Err(ThreadError::Build(e)));
                        return;
                    }
                    Err(p) => {
                        let _ = ready_tx.send(Err(ThreadError::Panicked(panic_text(p))));
                        return;
                    }
                };
                if ready_tx.send(Ok(instance.cancel_token())).is_err() {
                    return;
                }
                while let Ok(job) = receiver.recv() {
                    if thread_closing.load(Ordering::SeqCst) || !job(&mut instance) {
                        break;
                    }
                }
                // Dropping the receiver and the instance here, on this
                // thread, drops the queued jobs and with them their reply
                // senders: waiting callers see `Stopped`.
            })
            .map_err(ThreadError::Spawn)?;

        match ready_rx.recv() {
            Ok(Ok(cancel)) => Ok(InstanceThread {
                commands: Some(commands),
                join: Mutex::new(Some(join)),
                closing,
                cancel,
            }),
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            Err(_) => {
                let _ = join.join();
                Err(ThreadError::Stopped)
            }
        }
    }

    /// The token the instance polls; clone it to cancel a running command
    /// from any thread (see [`CancelToken`]). It stays set until
    /// [`CancelToken::reset`].
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Queues `command` and returns at once with a handle to its result.
    pub fn submit<R: Send + 'static>(
        &self,
        command: impl FnOnce(&mut Instance) -> R + Send + 'static,
    ) -> Result<Reply<R>, ThreadError> {
        let sender = self.commands.as_ref().ok_or(ThreadError::Stopped)?;
        let (reply_tx, reply_rx) = mpsc::channel();
        let job: Job =
            Box::new(
                move |instance| match catch_unwind(AssertUnwindSafe(|| command(instance))) {
                    Ok(value) => {
                        let _ = reply_tx.send(Ok(value));
                        true
                    }
                    Err(p) => {
                        let _ = reply_tx.send(Err(ThreadError::Panicked(panic_text(p))));
                        false
                    }
                },
            );
        sender.send(job).map_err(|_| ThreadError::Stopped)?;
        Ok(Reply { receiver: reply_rx })
    }

    /// Runs `command` on the interpreter thread and blocks until it has
    /// finished. The command returns whatever it needs to (`Outcome`,
    /// `Variant`, `Error`... are `Send`); the instance itself never leaves
    /// its thread.
    pub fn call<R: Send + 'static>(
        &self,
        command: impl FnOnce(&mut Instance) -> R + Send + 'static,
    ) -> Result<R, ThreadError> {
        self.submit(command)?.wait()
    }

    /// Stops the thread and waits for it: the running command finishes,
    /// queued ones are dropped and the instance is dropped on its thread.
    /// Returns [`ThreadError::Panicked`] when the thread ended by a panic
    /// that no command reported.
    pub fn close(mut self) -> Result<(), ThreadError> {
        self.stop()
    }

    fn stop(&mut self) -> Result<(), ThreadError> {
        self.closing.store(true, Ordering::SeqCst);
        self.commands = None;
        let handle = self.join.lock().ok().and_then(|mut j| j.take());
        match handle {
            Some(h) => h.join().map_err(|p| ThreadError::Panicked(panic_text(p))),
            None => Ok(()),
        }
    }
}

impl Drop for InstanceThread {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

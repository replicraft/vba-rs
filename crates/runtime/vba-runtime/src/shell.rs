//! `Shell`: starting another program. Denied unless the host grants
//! `ProcessSpawn` and the embedder installed a launcher.

use crate::error::FsError;

/// Starts programs for the `Shell` function. The `ProcessSpawn` permission
/// has been checked before a hook is called.
pub trait ShellHook {
    /// Starts `command_line` without waiting for it; returns a process id.
    fn spawn(&mut self, command_line: &str, window_style: i16) -> Result<u32, FsError>;
}

/// The default: no launcher is configured, so `Shell` is an explicit
/// "unsupported" error even when the permission was granted.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoShell;

impl ShellHook for NoShell {
    fn spawn(&mut self, _: &str, _: i16) -> Result<u32, FsError> {
        Err(FsError::Unsupported(
            "Shell: no process launcher is configured for this session".into(),
        ))
    }
}

/// A launcher over the operating system (`sh -c` or `cmd /C`). Installing it
/// is the embedder's explicit decision; the program is detached and its
/// output is discarded. The window style is ignored.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdShell;

impl ShellHook for StdShell {
    fn spawn(&mut self, command_line: &str, _: i16) -> Result<u32, FsError> {
        use std::process::{Command, Stdio};
        let mut cmd;
        if cfg!(windows) {
            cmd = Command::new("cmd");
            cmd.arg("/C").arg(command_line);
        } else {
            cmd = Command::new("/bin/sh");
            cmd.arg("-c").arg(command_line);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match cmd.spawn() {
            Ok(child) => Ok(child.id()),
            Err(_) => Err(FsError::Vba(53)),
        }
    }
}

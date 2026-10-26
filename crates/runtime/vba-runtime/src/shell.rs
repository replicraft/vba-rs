//! `Shell`: starting another program. Denied unless the host grants
//! `ProcessSpawn` and the embedder installed a launcher.

use crate::error::FsError;
use vba_host::command::split_command_line;

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

/// A launcher over the operating system. Installing it is the embedder's
/// explicit decision; the program is detached and its output is discarded.
/// The window style is ignored.
///
/// The command line is split into a program and arguments by the same reader
/// the permission check uses ([`vba_host::command::split_command_line`]) and
/// the program is started directly on every platform, never through a shell
/// (`sh -c` or `cmd /C`), so shell syntax in it (`;`, `|`, `&&`, `&`, `$(...)`)
/// has no effect and a `ProcessSpawn` grant scoped to a program really names
/// the program that runs. Built-in commands of `cmd.exe` (`dir`, `copy`) are
/// therefore not available; start `cmd.exe` itself (and grant it) if a
/// program needs them.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdShell;

impl ShellHook for StdShell {
    fn spawn(&mut self, command_line: &str, _: i16) -> Result<u32, FsError> {
        use std::process::{Command, Stdio};
        let mut parts = split_command_line(command_line).into_iter();
        let Some(program) = parts.next() else {
            return Err(FsError::Vba(53));
        };
        let mut cmd = Command::new(program);
        cmd.args(parts);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match cmd.spawn() {
            Ok(child) => Ok(child.id()),
            Err(_) => Err(FsError::Vba(53)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn shell_syntax_is_not_interpreted() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");
        let m = marker.to_string_lossy();
        // A command separator must not start a second program.
        let mut sh = StdShell;
        let pid = sh
            .spawn(&format!("/bin/echo hi; /usr/bin/touch {m}"), 0)
            .unwrap();
        assert!(pid > 0);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!marker.exists());
        // A plain program with arguments does run.
        sh.spawn(&format!("/usr/bin/touch {m}"), 0).unwrap();
        for _ in 0..50 {
            if marker.exists() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("the program did not run");
    }
}

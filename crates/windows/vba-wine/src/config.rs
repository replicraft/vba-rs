//! Locating Wine and the helper. Nothing here is ambient: paths are given
//! explicitly, and `PATH` is consulted only when the embedder calls
//! [`find_wine_in_path`] / [`find_wine_in_env`].

use crate::error::WineError;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// How the helper is started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// `wine <helper.exe>`: the real configuration.
    Wine {
        /// Path to the `wine` / `wine64` executable.
        wine: PathBuf,
        /// Path to the helper PE (`vba-wine-host.exe`).
        helper: PathBuf,
    },
    /// Run the helper as a native executable, with no Wine involved. The
    /// helper then runs in protocol-only mode on non-Windows hosts. For tests
    /// and on genuine Windows hosts; it provides no Wine bridge on Unix.
    Direct {
        /// Path to the helper executable.
        helper: PathBuf,
    },
}

/// Everything needed to start a helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WineConfig {
    launch: Launch,
    prefix: Option<PathBuf>,
    args: Vec<OsString>,
    env: Vec<(OsString, OsString)>,
    inherit_env: bool,
}

impl WineConfig {
    /// Wine launch with explicit executables.
    pub fn wine(wine: impl Into<PathBuf>, helper: impl Into<PathBuf>) -> Self {
        Self::new(Launch::Wine {
            wine: wine.into(),
            helper: helper.into(),
        })
    }

    /// Native launch of the helper (see [`Launch::Direct`]).
    pub fn direct(helper: impl Into<PathBuf>) -> Self {
        Self::new(Launch::Direct {
            helper: helper.into(),
        })
    }

    fn new(launch: Launch) -> Self {
        WineConfig {
            launch,
            prefix: None,
            args: Vec::new(),
            env: Vec::new(),
            inherit_env: true,
        }
    }

    /// Sets `WINEPREFIX` (Wine launches only). A scratch prefix is advised.
    pub fn with_prefix(mut self, prefix: impl Into<PathBuf>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Extra helper command-line arguments.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets an environment variable for the child.
    pub fn with_env(mut self, k: impl Into<OsString>, v: impl Into<OsString>) -> Self {
        self.env.push((k.into(), v.into()));
        self
    }

    /// Whether the child inherits this process's environment (default yes;
    /// Wine needs `HOME`, `DISPLAY` and similar). `false` passes only the
    /// variables set through this config, which usually requires `HOME` too.
    pub fn inherit_env(mut self, yes: bool) -> Self {
        self.inherit_env = yes;
        self
    }

    /// The launch mode.
    pub fn launch(&self) -> &Launch {
        &self.launch
    }

    /// Checks that the configured files exist. Nothing is started. Missing
    /// pieces are [`WineError::Unavailable`].
    pub fn validate(&self) -> Result<(), WineError> {
        let need = |p: &Path, what: &str| {
            if p.is_file() {
                Ok(())
            } else {
                Err(WineError::Unavailable(format!(
                    "{what} not found at {}",
                    p.display()
                )))
            }
        };
        match &self.launch {
            Launch::Wine { wine, helper } => {
                need(wine, "wine executable")?;
                need(helper, "helper executable")
            }
            Launch::Direct { helper } => need(helper, "helper executable"),
        }
    }

    pub(crate) fn command(&self) -> Command {
        let mut cmd = match &self.launch {
            Launch::Wine { wine, helper } => {
                let mut c = Command::new(wine);
                c.arg(helper);
                c
            }
            Launch::Direct { helper } => Command::new(helper),
        };
        cmd.args(&self.args);
        if !self.inherit_env {
            cmd.env_clear();
        }
        if matches!(self.launch, Launch::Wine { .. }) {
            if let Some(p) = &self.prefix {
                cmd.env("WINEPREFIX", p);
            }
            // Wine's diagnostics would only pollute stderr, which is discarded.
            cmd.env("WINEDEBUG", "-all");
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }
}

/// Searches a `PATH`-style value for `wine64` then `wine`. Pure: it reads only
/// the value it is given (and checks that candidates are files).
pub fn find_wine_in_path(path_var: &OsStr) -> Option<PathBuf> {
    for name in ["wine64", "wine"] {
        for dir in std::env::split_paths(path_var) {
            let cand = dir.join(name);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// Like [`find_wine_in_path`] but reads the process `PATH`. Call it only when
/// the embedder has decided that environment discovery is acceptable.
pub fn find_wine_in_env() -> Option<PathBuf> {
    find_wine_in_path(&std::env::var_os("PATH")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_files_are_unavailable() {
        let c = WineConfig::wine("/nonexistent/wine", "/nonexistent/h.exe");
        assert!(matches!(c.validate(), Err(WineError::Unavailable(_))));
        let c = WineConfig::direct("/nonexistent/h");
        assert!(matches!(c.validate(), Err(WineError::Unavailable(_))));
    }

    #[test]
    fn path_search_is_pure_and_prefers_wine64() {
        let d = std::env::temp_dir().join(format!("vba-wine-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(find_wine_in_path(d.as_os_str()), None);
        std::fs::write(d.join("wine"), b"").unwrap();
        assert_eq!(find_wine_in_path(d.as_os_str()), Some(d.join("wine")));
        std::fs::write(d.join("wine64"), b"").unwrap();
        assert_eq!(find_wine_in_path(d.as_os_str()), Some(d.join("wine64")));
        std::fs::remove_dir_all(&d).unwrap();
    }
}

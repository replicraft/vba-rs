//! Opt-in access to the real clock and environment variables.
//!
//! A session defaults to a fixed clock and an empty environment
//! (`vba_stdlib::FixedHost`) so a run is reproducible and cannot read the
//! machine. Installing a [`SystemHost`] is the embedder's explicit decision.

use std::time::{SystemTime, UNIX_EPOCH};

use vba_stdlib::{Clock, Environment, Host, Locale};
use vba_value::Date;

/// The system clock (UTC) and process environment, with the en-US locale.
#[derive(Debug, Clone)]
pub struct SystemHost {
    locale: Locale,
}

impl Default for SystemHost {
    fn default() -> Self {
        SystemHost::new()
    }
}

impl SystemHost {
    /// A host reading the real clock and environment.
    pub fn new() -> SystemHost {
        SystemHost {
            locale: Locale::en_us(),
        }
    }
}

fn now_serial() -> f64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    25569.0 + secs / 86_400.0
}

impl Clock for SystemHost {
    fn now(&self) -> Date {
        Date::from_serial(now_serial()).unwrap_or_default()
    }
}

impl Environment for SystemHost {
    fn environ(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn environ_index(&self, index: usize) -> Option<String> {
        let mut vars: Vec<(String, String)> = std::env::vars().collect();
        vars.sort();
        let (k, v) = vars.get(index.checked_sub(1)?)?;
        Some(format!("{k}={v}"))
    }
}

impl Host for SystemHost {
    fn locale(&self) -> &Locale {
        &self.locale
    }
}

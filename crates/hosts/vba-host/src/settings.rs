//! The store behind `SaveSetting`, `GetSetting`, `GetAllSettings` and
//! `DeleteSetting`.
//!
//! The statements are documented to use the Windows registry key
//! `HKEY_CURRENT_USER\Software\VB and VBA Program Settings`. A sandboxed
//! interpreter must not touch the registry, so the storage is a host
//! service: [`SettingsStore`]. [`MemorySettings`] is the default, an
//! in-memory store that lives as long as the interpreter; an embedder can
//! supply a persistent one through [`Host::settings`](crate::Host::settings).
//!
//! Names are matched case-insensitively (registry semantics). The values are
//! strings. Error numbers are **unverified** beyond the documented
//! "a run-time error occurs" (5 is used).

use crate::host::HostResult;
use std::cell::RefCell;
use std::collections::BTreeMap;
use vba_automation::ErrInfo;

/// Storage for application settings (application -> section -> key -> value).
pub trait SettingsStore {
    /// `SaveSetting`: creates or replaces a key. Empty names are error 5.
    fn save(&self, app: &str, section: &str, key: &str, value: &str) -> HostResult<()>;

    /// `GetSetting`: the value, or `None` when the application, section or
    /// key does not exist (the caller then returns the default).
    fn get(&self, app: &str, section: &str, key: &str) -> HostResult<Option<String>>;

    /// `GetAllSettings`: the `(key, value)` pairs of a section ordered by
    /// key (**unverified**: the registry's enumeration order), or `None` when
    /// the application or section does not exist.
    fn get_all(&self, app: &str, section: &str) -> HostResult<Option<Vec<(String, String)>>>;

    /// `DeleteSetting`: removes a key, a section (without `key`) or the
    /// whole application (without `section`). A missing target is error 5.
    fn delete(&self, app: &str, section: Option<&str>, key: Option<&str>) -> HostResult<()>;
}

type Section = BTreeMap<String, (String, String)>;

/// An in-memory [`SettingsStore`].
#[derive(Debug, Default)]
pub struct MemorySettings {
    // lower-case app -> lower-case section -> lower-case key -> (key, value)
    apps: RefCell<BTreeMap<String, BTreeMap<String, Section>>>,
}

impl MemorySettings {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

fn bad_name() -> ErrInfo {
    ErrInfo::standard(5)
}

impl SettingsStore for MemorySettings {
    fn save(&self, app: &str, section: &str, key: &str, value: &str) -> HostResult<()> {
        if app.is_empty() || section.is_empty() || key.is_empty() {
            return Err(bad_name());
        }
        self.apps
            .borrow_mut()
            .entry(app.to_lowercase())
            .or_default()
            .entry(section.to_lowercase())
            .or_default()
            .insert(key.to_lowercase(), (key.to_owned(), value.to_owned()));
        Ok(())
    }

    fn get(&self, app: &str, section: &str, key: &str) -> HostResult<Option<String>> {
        Ok(self
            .apps
            .borrow()
            .get(&app.to_lowercase())
            .and_then(|a| a.get(&section.to_lowercase()))
            .and_then(|s| s.get(&key.to_lowercase()))
            .map(|(_, v)| v.clone()))
    }

    fn get_all(&self, app: &str, section: &str) -> HostResult<Option<Vec<(String, String)>>> {
        Ok(self
            .apps
            .borrow()
            .get(&app.to_lowercase())
            .and_then(|a| a.get(&section.to_lowercase()))
            .map(|s| s.values().cloned().collect()))
    }

    fn delete(&self, app: &str, section: Option<&str>, key: Option<&str>) -> HostResult<()> {
        let mut apps = self.apps.borrow_mut();
        let app = app.to_lowercase();
        match (section, key) {
            (None, _) => apps.remove(&app).map(|_| ()).ok_or_else(bad_name),
            (Some(section), None) => apps
                .get_mut(&app)
                .and_then(|a| a.remove(&section.to_lowercase()))
                .map(|_| ())
                .ok_or_else(bad_name),
            (Some(section), Some(key)) => apps
                .get_mut(&app)
                .and_then(|a| a.get_mut(&section.to_lowercase()))
                .and_then(|s| s.remove(&key.to_lowercase()))
                .map(|_| ())
                .ok_or_else(bad_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_get_delete_roundtrip_is_case_insensitive() {
        let s = MemorySettings::new();
        s.save("MyApp", "Startup", "Top", "75").unwrap();
        s.save("myapp", "startup", "Left", "50").unwrap();
        assert_eq!(
            s.get("MYAPP", "STARTUP", "top").unwrap().as_deref(),
            Some("75")
        );
        assert_eq!(s.get("MyApp", "Startup", "none").unwrap(), None);
        // Ordered by key, with the key's first spelling.
        assert_eq!(
            s.get_all("MyApp", "Startup").unwrap().unwrap(),
            vec![("Left".into(), "50".into()), ("Top".into(), "75".into())]
        );
        assert_eq!(s.get_all("MyApp", "Nope").unwrap(), None);
        s.delete("MyApp", Some("Startup"), Some("Top")).unwrap();
        assert_eq!(s.get("MyApp", "Startup", "Top").unwrap(), None);
        s.delete("MyApp", Some("Startup"), None).unwrap();
        assert_eq!(s.get_all("MyApp", "Startup").unwrap(), None);
        assert_eq!(
            s.delete("MyApp", Some("Startup"), None).unwrap_err().number,
            5
        );
        s.delete("MyApp", None, None).unwrap();
        assert_eq!(s.delete("MyApp", None, None).unwrap_err().number, 5);
        assert_eq!(s.save("", "s", "k", "v").unwrap_err().number, 5);
    }
}

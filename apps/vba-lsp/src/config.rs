//! Client settings (`vba.*`) to `vba-lsp-core` configuration.
//!
//! ```json
//! { "vba": { "profile": "vba7.1-excel-win64",
//!            "encoding": "utf-8",
//!            "constants": { "DEBUGGING": 1 },
//!            "semantic": true,
//!            "projectName": "VBAProject",
//!            "moduleKinds": { "*.txt": "class", "Sheet1.vba": "document" } } }
//! ```
//!
//! The object under `vba` may also be passed directly (initialization
//! options). Absent settings take their defaults, so removing a setting in
//! the editor restores the default.

use serde_json::Value;
use vba_lsp_core::{Config, ModuleKind, parse_encoding};

/// Builds a configuration from settings and returns warnings for entries
/// that were rejected.
pub fn from_settings(settings: &Value) -> (Config, Vec<String>) {
    let mut cfg = Config::default();
    let mut warnings = Vec::new();
    let section = settings.get("vba").unwrap_or(settings);
    let Some(obj) = section.as_object() else {
        if !section.is_null() {
            warnings.push("`vba` settings must be an object".to_string());
        }
        return (cfg, warnings);
    };
    for (key, value) in obj {
        match key.as_str() {
            "profile" => match value.as_str() {
                Some(id) => {
                    if let Err(e) = cfg.set_profile(id) {
                        warnings.push(e.to_string());
                    }
                }
                None => warnings.push("`vba.profile` must be a string".to_string()),
            },
            "encoding" => match value.as_str().and_then(parse_encoding) {
                Some(e) => cfg.encoding = e,
                None => warnings.push(format!(
                    "`vba.encoding` must be `utf-8` or a Windows code page such as `cp1252`, got {value}"
                )),
            },
            "constants" => match value.as_object() {
                Some(m) => {
                    for (name, v) in m {
                        match v.as_i64() {
                            Some(n) => cfg.constants.push((name.clone(), n)),
                            None => warnings
                                .push(format!("`vba.constants.{name}` must be an integer")),
                        }
                    }
                }
                None => warnings.push("`vba.constants` must be an object".to_string()),
            },
            "semantic" => match value.as_bool() {
                Some(b) => cfg.semantic = b,
                None => warnings.push("`vba.semantic` must be a boolean".to_string()),
            },
            "projectName" => match value.as_str() {
                Some(n) if !n.trim().is_empty() => cfg.project_name = n.to_string(),
                _ => warnings.push("`vba.projectName` must be a non-empty string".to_string()),
            },
            "moduleKinds" => match value.as_object() {
                Some(m) => {
                    for (pattern, kind) in m {
                        match kind.as_str().and_then(module_kind) {
                            Some(k) => cfg.module_kinds.push((pattern.clone(), k)),
                            None => warnings.push(format!(
                                "`vba.moduleKinds.{pattern}` must be `standard`, `class`, \
                                 `document` or `designer`"
                            )),
                        }
                    }
                }
                None => warnings.push("`vba.moduleKinds` must be an object".to_string()),
            },
            other => warnings.push(format!("unknown setting `vba.{other}`")),
        }
    }
    (cfg, warnings)
}

fn module_kind(s: &str) -> Option<ModuleKind> {
    match s.to_ascii_lowercase().as_str() {
        "standard" => Some(ModuleKind::Standard),
        "class" => Some(ModuleKind::Class),
        "document" => Some(ModuleKind::Document),
        "designer" => Some(ModuleKind::Designer),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn full_settings() {
        let (cfg, w) = from_settings(&json!({"vba": {
            "profile": "vba6-excel-win32",
            "encoding": "cp932",
            "constants": {"DEBUGGING": 1}
        }}));
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(cfg.profile.id, "vba6-excel-win32");
        assert_eq!(cfg.constants, [("DEBUGGING".to_string(), 1)]);
        assert_eq!(cfg.encoding, parse_encoding("932").unwrap());
    }

    #[test]
    fn project_settings() {
        let (cfg, w) = from_settings(&json!({"vba": {
            "semantic": false,
            "projectName": "Book",
            "moduleKinds": {"*.txt": "Class", "a.vba": "designer", "bad": "form"}
        }}));
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(!cfg.semantic);
        assert_eq!(cfg.project_name, "Book");
        assert_eq!(
            cfg.module_kinds,
            [
                ("*.txt".to_string(), ModuleKind::Class),
                ("a.vba".to_string(), ModuleKind::Designer)
            ]
        );
        let (cfg, _) = from_settings(&json!({"vba": {"semantic": "yes", "projectName": ""}}));
        assert!(cfg.semantic);
        assert_eq!(cfg.project_name, Config::default().project_name);
    }

    #[test]
    fn section_may_be_passed_directly_and_null_means_defaults() {
        let (cfg, w) = from_settings(&json!({"profile": "vba5-excel-win32"}));
        assert!(w.is_empty());
        assert_eq!(cfg.profile.id, "vba5-excel-win32");
        let (cfg, w) = from_settings(&Value::Null);
        assert!(w.is_empty());
        assert_eq!(cfg.profile.id, Config::default().profile.id);
    }

    #[test]
    fn bad_entries_warn_and_keep_defaults() {
        let (cfg, w) = from_settings(&json!({"vba": {
            "profile": "nope", "encoding": 3, "constants": {"A": "x"}, "extra": true
        }}));
        assert_eq!(w.len(), 4, "{w:?}");
        assert_eq!(cfg.profile.id, Config::default().profile.id);
        assert!(cfg.constants.is_empty());
    }
}

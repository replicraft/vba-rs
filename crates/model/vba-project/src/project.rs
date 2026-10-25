use crate::{Module, Preserved, Reference};

/// Platform a project was created for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysKind {
    /// 16-bit Windows.
    Win16,
    /// 32-bit Windows.
    Win32,
    /// Macintosh.
    Macintosh,
    /// 64-bit Windows.
    Win64,
    /// A value outside the documented set, kept as stored.
    Other(u32),
}

impl SysKind {
    /// Converts the stored numeric value.
    pub fn from_value(v: u32) -> Self {
        match v {
            0 => SysKind::Win16,
            1 => SysKind::Win32,
            2 => SysKind::Macintosh,
            3 => SysKind::Win64,
            other => SysKind::Other(other),
        }
    }

    /// The numeric value used for storage.
    pub fn value(self) -> u32 {
        match self {
            SysKind::Win16 => 0,
            SysKind::Win32 => 1,
            SysKind::Macintosh => 2,
            SysKind::Win64 => 3,
            SysKind::Other(v) => v,
        }
    }
}

/// Version of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VersionInfo {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u16,
}

/// Access restrictions recorded for a project.
///
/// These flags describe what the project declares; the model does not enforce
/// them. Module source is stored unencrypted in a protected project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protection {
    /// The user chose to protect the project.
    pub user_protected: bool,
    /// The host application chose to protect the project.
    pub host_protected: bool,
    /// The VBA editor chose to protect the project.
    pub vbe_protected: bool,
    /// A password (hash or plain text) is recorded.
    pub has_password: bool,
    /// The project is visible in the editor.
    pub visible: bool,
}

impl Default for Protection {
    fn default() -> Self {
        Self {
            user_protected: false,
            host_protected: false,
            vbe_protected: false,
            has_password: false,
            visible: true,
        }
    }
}

impl Protection {
    /// Whether the project is locked for viewing or password protected.
    pub fn is_locked(&self) -> bool {
        self.vbe_protected || self.has_password || self.user_protected
    }
}

/// A host extender: an aggregatable server type library extending the VBA
/// environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostExtender {
    /// Index of the entry.
    pub index: u32,
    /// GUID of the extended type library, as written (with braces).
    pub guid: String,
    /// Host-provided library name (`VBE` for the built-in library).
    pub lib_name: String,
    /// Host-provided creation flags.
    pub creation_flags: u32,
}

/// Saved state of a module editor window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowRecord {
    /// Name of the module the window belongs to.
    pub module: String,
    /// Window coordinates and state exactly as stored, for example
    /// `"23, 23, 911, 280, C"`.
    pub state: String,
}

/// Errors from editing the module list of a [`VbaProject`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModelError {
    /// A module with this name (compared case-insensitively) already exists.
    #[error("a module named {0:?} already exists")]
    DuplicateModule(String),
    /// No module with this name exists.
    #[error("no module named {0:?}")]
    NoSuchModule(String),
}

/// A VBA project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaProject {
    /// Project name.
    pub name: String,
    /// Project description.
    pub description: String,
    /// Windows code page used for single-byte/multi-byte text in the project.
    pub code_page: u16,
    /// Platform the project was created for.
    pub sys_kind: SysKind,
    /// Locale identifier.
    pub lcid: u32,
    /// Locale identifier used for `Invoke` calls.
    pub lcid_invoke: u32,
    /// Compat version, when recorded.
    pub compat_version: Option<u32>,
    /// Path of the Help file, when recorded.
    pub help_file: Option<String>,
    /// Help topic identifier of the project.
    pub help_context: u32,
    /// Flags of the project's type library.
    pub lib_flags: u32,
    /// Project version.
    pub version: VersionInfo,
    /// Conditional compilation constants, for example `abc = 1 : x = 2`.
    pub constants: String,
    /// CLSID of the project's type library, as written (with braces).
    pub project_id: Option<String>,
    /// Declared access restrictions.
    pub protection: Protection,
    /// Host extenders.
    pub host_extenders: Vec<HostExtender>,
    /// Saved editor window states.
    pub workspace: Vec<WindowRecord>,
    /// References to type libraries and other projects.
    pub references: Vec<Reference>,
    /// Modules, in project order.
    pub modules: Vec<Module>,
    /// Data attached by a format layer.
    pub preserved: Preserved,
}

impl VbaProject {
    /// Creates an empty project with default metadata (code page 1252, US
    /// English locale, 32-bit Windows).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            code_page: 1252,
            sys_kind: SysKind::Win32,
            lcid: 0x0409,
            lcid_invoke: 0x0409,
            compat_version: None,
            help_file: None,
            help_context: 0,
            lib_flags: 0,
            version: VersionInfo::default(),
            constants: String::new(),
            project_id: None,
            protection: Protection::default(),
            host_extenders: Vec::new(),
            workspace: Vec::new(),
            references: Vec::new(),
            modules: Vec::new(),
            preserved: Preserved::new(),
        }
    }

    /// Finds a module by name, ignoring ASCII case as VBA does.
    pub fn module(&self, name: &str) -> Option<&Module> {
        self.modules
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// Finds a module by name for modification.
    pub fn module_mut(&mut self, name: &str) -> Option<&mut Module> {
        self.modules
            .iter_mut()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// Appends a module, rejecting a name that is already in use.
    pub fn add_module(&mut self, module: Module) -> Result<(), ModelError> {
        if self.module(&module.name).is_some() {
            return Err(ModelError::DuplicateModule(module.name));
        }
        self.modules.push(module);
        Ok(())
    }

    /// Removes and returns the module with the given name.
    pub fn remove_module(&mut self, name: &str) -> Result<Module, ModelError> {
        let idx = self
            .modules
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| ModelError::NoSuchModule(name.to_owned()))?;
        Ok(self.modules.remove(idx))
    }

    /// Replaces the source text of an existing module and marks it present.
    pub fn set_module_source(
        &mut self,
        name: &str,
        source: impl Into<String>,
    ) -> Result<(), ModelError> {
        let m = self
            .module_mut(name)
            .ok_or_else(|| ModelError::NoSuchModule(name.to_owned()))?;
        m.source = source.into();
        m.has_source = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleKind;

    #[test]
    fn module_lookup_is_case_insensitive() {
        let mut p = VbaProject::new("P");
        p.add_module(Module::new("Mod1", ModuleKind::Standard, "x"))
            .unwrap();
        assert!(p.module("mOD1").is_some());
        assert!(p.module("Mod2").is_none());
    }

    #[test]
    fn duplicate_names_rejected() {
        let mut p = VbaProject::new("P");
        p.add_module(Module::new("A", ModuleKind::Class, ""))
            .unwrap();
        let err = p
            .add_module(Module::new("a", ModuleKind::Standard, ""))
            .unwrap_err();
        assert_eq!(err, ModelError::DuplicateModule("a".into()));
        assert_eq!(p.modules.len(), 1);
    }

    #[test]
    fn remove_and_set_source() {
        let mut p = VbaProject::new("P");
        p.add_module(Module::new("A", ModuleKind::Class, "old"))
            .unwrap();
        p.set_module_source("a", "new").unwrap();
        assert_eq!(p.module("A").unwrap().source, "new");
        assert!(p.set_module_source("zz", "").is_err());
        assert_eq!(p.remove_module("A").unwrap().source, "new");
        assert_eq!(
            p.remove_module("A").unwrap_err(),
            ModelError::NoSuchModule("A".into())
        );
    }

    #[test]
    fn set_source_marks_present() {
        let mut p = VbaProject::new("P");
        let mut m = Module::new("A", ModuleKind::Standard, "");
        m.has_source = false;
        p.add_module(m).unwrap();
        p.set_module_source("A", "Sub X()\r\nEnd Sub\r\n").unwrap();
        assert!(p.module("A").unwrap().has_source);
    }

    #[test]
    fn sys_kind_round_trips() {
        for v in [0, 1, 2, 3, 77] {
            assert_eq!(SysKind::from_value(v).value(), v);
        }
    }

    #[test]
    fn protection_default_is_unlocked_and_visible() {
        let p = Protection::default();
        assert!(!p.is_locked());
        assert!(p.visible);
        assert!(
            Protection {
                has_password: true,
                ..p
            }
            .is_locked()
        );
    }

    #[test]
    fn preserved_data_survives_clone() {
        let mut p = VbaProject::new("P");
        p.preserved.insert("x:y", vec![1, 2, 3]);
        let mut m = Module::new("A", ModuleKind::Standard, "");
        m.preserved.insert("x:z", vec![9]);
        p.add_module(m).unwrap();
        let q = p.clone();
        assert_eq!(q, p);
        assert_eq!(q.preserved.get("x:y"), Some(&[1, 2, 3][..]));
        assert_eq!(q.modules[0].preserved.get("x:z"), Some(&[9][..]));
    }
}

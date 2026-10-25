/// A reference from a project to an Automation type library or another VBA
/// project.
///
/// Library identifiers (`libid` fields) are kept as the text stored in the
/// project, for example `*\G{00020430-0000-0000-C000-000000000046}#2.0#0#...`.
/// The model does not validate their syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    /// A registered Automation type library.
    Registered {
        /// Display name of the reference, if recorded.
        name: Option<String>,
        /// Type library identifier.
        libid: String,
    },
    /// A reference to another VBA project.
    Project {
        /// Display name of the reference, if recorded.
        name: Option<String>,
        /// Project identifier with an absolute path.
        libid_absolute: String,
        /// Project identifier with a path relative to this project.
        libid_relative: String,
        /// Major version of the referenced project.
        major_version: u32,
        /// Minor version of the referenced project.
        minor_version: u16,
    },
    /// A reference to a control's twiddled type library.
    Control {
        /// Display name of the reference, if recorded.
        name: Option<String>,
        /// Identifier of the twiddled type library.
        libid_twiddled: String,
        /// Name of the extended type library, if recorded.
        extended_name: Option<String>,
        /// Identifier of the extended type library.
        libid_extended: String,
        /// GUID of the type library the extended library was generated from,
        /// in registry form (`{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`).
        original_type_lib: String,
        /// Cookie of the extended type library.
        cookie: u32,
        /// Identifier of the Automation type library the twiddled library was
        /// generated from, when the project records one.
        libid_original: Option<String>,
    },
}

impl Reference {
    /// Short human-readable kind: `registered`, `project` or `control`.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Reference::Registered { .. } => "registered",
            Reference::Project { .. } => "project",
            Reference::Control { .. } => "control",
        }
    }

    /// The recorded display name, if any.
    pub fn name(&self) -> Option<&str> {
        match self {
            Reference::Registered { name, .. }
            | Reference::Project { name, .. }
            | Reference::Control { name, .. } => name.as_deref(),
        }
    }

    /// The primary library identifier of the reference.
    pub fn libid(&self) -> &str {
        match self {
            Reference::Registered { libid, .. } => libid,
            Reference::Project { libid_absolute, .. } => libid_absolute,
            Reference::Control { libid_twiddled, .. } => libid_twiddled,
        }
    }
}

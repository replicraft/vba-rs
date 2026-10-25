/// The application hosting the VBA project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostApplication {
    /// Microsoft Excel.
    Excel,
    /// Microsoft Word.
    Word,
    /// Microsoft PowerPoint.
    PowerPoint,
    /// Microsoft Access.
    Access,
    /// Microsoft Outlook.
    Outlook,
}

impl HostApplication {
    /// All hosts.
    pub const ALL: [HostApplication; 5] = [
        HostApplication::Excel,
        HostApplication::Word,
        HostApplication::PowerPoint,
        HostApplication::Access,
        HostApplication::Outlook,
    ];

    /// Lower-case identifier.
    pub const fn id(self) -> &'static str {
        match self {
            HostApplication::Excel => "excel",
            HostApplication::Word => "word",
            HostApplication::PowerPoint => "powerpoint",
            HostApplication::Access => "access",
            HostApplication::Outlook => "outlook",
        }
    }
}

/// Host application and its version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostInfo {
    /// The application.
    pub application: HostApplication,
    /// Application major version as reported by its `Version` property
    /// (for example 16 for Office 2016).
    pub major: u8,
    /// Application minor version.
    pub minor: u8,
    /// Marketing release year, for display (for example 2016).
    pub release_year: u16,
}

impl HostInfo {
    /// Creates host metadata.
    pub const fn new(
        application: HostApplication,
        major: u8,
        minor: u8,
        release_year: u16,
    ) -> Self {
        HostInfo {
            application,
            major,
            minor,
            release_year,
        }
    }
}

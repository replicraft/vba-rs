//! Host identity.

use vba_profile::{Profile, default_profile};

/// Who the host is and which VBA implementation it presents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostMetadata {
    /// Host name, e.g. the value of `Application.Name`.
    pub name: String,
    /// Host version string, e.g. the value of `Application.Version`.
    pub version: String,
    /// The VBA target profile (language generation, host application as seen
    /// by `#If`, operating system, bitness).
    pub profile: Profile,
}

impl HostMetadata {
    /// Creates metadata.
    pub fn new(name: &str, version: &str, profile: Profile) -> Self {
        HostMetadata {
            name: name.to_string(),
            version: version.to_string(),
            profile,
        }
    }

    /// Metadata of the headless host with the default profile.
    pub fn headless() -> Self {
        Self::new(
            "vba-rs headless",
            env!("CARGO_PKG_VERSION"),
            *default_profile(),
        )
    }
}

//! Raw version facts of a project and a cautious profile hint.
//!
//! Nothing here is a conclusion. A VBA project does not record which Office
//! build wrote it; the closest records are the version word in the
//! `_VBA_PROJECT` stream header, the platform in the `dir` stream, and
//! whatever the references happen to name. [`ProjectVersion`] exposes those
//! facts and [`ProjectVersion::profile_hint`] maps them through a tiny,
//! explicit table.

use crate::{ProjectSourceAvailability, Reference, SysKind, VbaProject};

/// Preserved-data key under which format layers keep the original
/// `_VBA_PROJECT` stream (MS-OVBA 2.3.4.1). [`ProjectVersion`] reads the
/// version word from it.
pub const VBA_PROJECT_STREAM_KEY: &str = "ovba:vba-project-stream";

/// The constant every `_VBA_PROJECT` header starts with (Reserved1).
const RESERVED1: u16 = 0x61CC;

/// Raw facts about a library identifier of a reference.
///
/// Parsed from the `*\G{guid}#major.minor#lcid#path#name` form. Fields that
/// are absent or unparseable are `None`; nothing is inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceHint {
    /// Index into [`VbaProject::references`].
    pub reference: usize,
    /// Display name of the reference, if recorded.
    pub name: Option<String>,
    /// The library identifier the facts were parsed from.
    pub libid: String,
    /// Type library GUID with braces, as written.
    pub guid: Option<String>,
    /// Type library version as written (two hexadecimal numbers), as
    /// `(major, minor)`.
    pub type_lib_version: Option<(u32, u32)>,
    /// Locale identifier as written (hexadecimal).
    pub lcid: Option<u32>,
    /// File path of the library as written.
    pub path: Option<String>,
    /// The number in an `Office<N>` path component (for example `16` for
    /// `...\Office16\MSO.DLL`), if the path has one. This is a number found in
    /// text, not a statement about which Office produced the project: the
    /// path describes the machine that last saved the reference.
    pub office_dir: Option<u32>,
}

/// Version-related facts about a project. See the module documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectVersion {
    /// The 16-bit version word of the `_VBA_PROJECT` header (MS-OVBA
    /// 2.3.4.1: the two-byte reserved constant `0x61CC`, then this word).
    /// `None` if the project carries no such stream or the constant is
    /// wrong. `0xFFFF` is the value meaning "no performance cache" and says
    /// nothing about the producer.
    pub version_word: Option<u16>,
    /// PROJECTSYSKIND.
    pub sys_kind: SysKind,
    /// PROJECTCODEPAGE.
    pub code_page: u16,
    /// PROJECTLCID.
    pub lcid: u32,
    /// Facts from the references' library identifiers, in reference order.
    pub reference_hints: Vec<ReferenceHint>,
    /// The project has modules and none has source.
    pub compiled_only: bool,
}

impl ProjectVersion {
    /// Collects the facts from a project.
    pub fn of(project: &VbaProject) -> Self {
        let mut hints = Vec::new();
        for (i, r) in project.references.iter().enumerate() {
            let name = r.name().map(str::to_owned);
            match r {
                Reference::Registered { libid, .. } => {
                    hints.push(parse_libid(i, name, libid));
                }
                Reference::Control {
                    libid_twiddled,
                    libid_original,
                    ..
                } => {
                    hints.push(parse_libid(i, name.clone(), libid_twiddled));
                    if let Some(o) = libid_original {
                        hints.push(parse_libid(i, name, o));
                    }
                }
                Reference::Project { .. } => {}
            }
        }
        Self {
            version_word: project
                .preserved
                .get(VBA_PROJECT_STREAM_KEY)
                .and_then(version_word),
            sys_kind: project.sys_kind,
            code_page: project.code_page,
            lcid: project.lcid,
            reference_hints: hints,
            compiled_only: project.source_availability() == ProjectSourceAvailability::CompiledOnly,
        }
    }

    /// A profile identifier (as listed by `vba-profile`) this project may
    /// belong to, or `None`.
    ///
    /// The table is deliberately tiny and every entry is a **hypothesis**,
    /// not a finding, until confirmed by projects from independently known
    /// Office builds:
    ///
    /// | version word | platform | hint |
    /// |---|---|---|
    /// | `0x0097` | 32-bit Windows | `vba7.0-excel-win32` |
    ///
    /// `0x0097` has been observed in a project that referenced Office 2010
    /// libraries on a 32-bit platform, which suggests VBA 7.0. The host in the
    /// returned profile name is the catalog's representative for that
    /// generation; the project does not say which host application wrote it.
    /// Every other word, platform combination, a missing or invalid header
    /// and the no-cache word `0xFFFF` yield `None`.
    pub fn profile_hint(&self) -> Option<&'static str> {
        match (self.version_word?, self.sys_kind) {
            (0x0097, SysKind::Win32) => Some("vba7.0-excel-win32"),
            _ => None,
        }
    }
}

impl VbaProject {
    /// Version-related facts about this project; see [`ProjectVersion`].
    pub fn project_version(&self) -> ProjectVersion {
        ProjectVersion::of(self)
    }
}

fn version_word(stream: &[u8]) -> Option<u16> {
    let reserved = u16::from_le_bytes([*stream.first()?, *stream.get(1)?]);
    if reserved != RESERVED1 {
        return None;
    }
    Some(u16::from_le_bytes([*stream.get(2)?, *stream.get(3)?]))
}

fn parse_libid(reference: usize, name: Option<String>, libid: &str) -> ReferenceHint {
    let mut hint = ReferenceHint {
        reference,
        name,
        libid: libid.to_owned(),
        guid: None,
        type_lib_version: None,
        lcid: None,
        path: None,
        office_dir: None,
    };
    let Some(open) = libid.find('{') else {
        return hint;
    };
    let Some(close) = libid[open..].find('}').map(|c| c + open) else {
        return hint;
    };
    hint.guid = Some(libid[open..=close].to_owned());
    let mut parts = libid[close + 1..].split('#');
    let _empty = parts.next();
    if let Some((a, b)) = parts.next().and_then(|v| v.split_once('.')) {
        hint.type_lib_version = u32::from_str_radix(a, 16)
            .ok()
            .zip(u32::from_str_radix(b, 16).ok());
    }
    hint.lcid = parts.next().and_then(|v| u32::from_str_radix(v, 16).ok());
    hint.path = parts.next().filter(|p| !p.is_empty()).map(str::to_owned);
    hint.office_dir = hint.path.as_deref().and_then(office_dir);
    hint
}

/// The digits of the first `Office<digits>` path component.
fn office_dir(path: &str) -> Option<u32> {
    let lower = path.to_ascii_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find("office") {
        let begin = from + at;
        let start = begin + 6;
        let digits: String = lower[start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let after = lower[start + digits.len()..].chars().next();
        let before = lower[..begin].chars().next_back();
        if (1..=2).contains(&digits.len())
            && before.is_none_or(|c| matches!(c, '\\' | '/'))
            && after.is_none_or(|c| matches!(c, '\\' | '/'))
        {
            return digits.parse().ok();
        }
        from = start;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(word: u16) -> Vec<u8> {
        let mut v = RESERVED1.to_le_bytes().to_vec();
        v.extend(word.to_le_bytes());
        v.extend([0, 1, 0]);
        v
    }

    fn project(word: Option<Vec<u8>>, sys: SysKind) -> VbaProject {
        let mut p = VbaProject::new("P");
        p.sys_kind = sys;
        if let Some(w) = word {
            p.preserved.insert(VBA_PROJECT_STREAM_KEY, w);
        }
        p
    }

    #[test]
    fn reads_the_word_after_the_reserved_constant() {
        let v = project(Some(header(0x0097)), SysKind::Win32).project_version();
        assert_eq!(v.version_word, Some(0x0097));
        assert_eq!(v.profile_hint(), Some("vba7.0-excel-win32"));
    }

    #[test]
    fn wrong_constant_or_short_stream_has_no_word() {
        let mut bad = header(0x0097);
        bad[0] = 0;
        for s in [bad, vec![0xCC, 0x61, 0x97], vec![]] {
            let v = project(Some(s), SysKind::Win32).project_version();
            assert_eq!(v.version_word, None);
            assert_eq!(v.profile_hint(), None);
        }
        let none = project(None, SysKind::Win32).project_version();
        assert_eq!(none.version_word, None);
    }

    #[test]
    fn unknown_words_and_platforms_give_no_hint() {
        for (w, k) in [
            (0x0097, SysKind::Win64),
            (0x0097, SysKind::Macintosh),
            (0x0098, SysKind::Win32),
            (0xFFFF, SysKind::Win32),
            (0x0000, SysKind::Win32),
        ] {
            let v = project(Some(header(w)), k).project_version();
            assert_eq!(v.version_word, Some(w));
            assert_eq!(v.profile_hint(), None, "{w:#06x} {k:?}");
        }
    }

    #[test]
    fn parses_reference_facts() {
        let mut p = VbaProject::new("P");
        p.references.push(Reference::Registered {
            name: Some("Office".into()),
            libid: "*\\G{2DF8D04C-5BFA-101B-BDE5-00AA0044DE52}#2.5#0#C:\\Program Files\\Common Files\\Microsoft Shared\\OFFICE14\\MSO.DLL#Microsoft Office 14.0 Object Library".into(),
        });
        p.references.push(Reference::Registered {
            name: None,
            libid: "*\\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\\Windows\\stdole2.tlb#OLE Automation".into(),
        });
        p.references.push(Reference::Registered {
            name: None,
            libid: "garbage".into(),
        });
        let h = p.project_version().reference_hints;
        assert_eq!(h.len(), 3);
        assert_eq!(
            h[0].guid.as_deref(),
            Some("{2DF8D04C-5BFA-101B-BDE5-00AA0044DE52}")
        );
        assert_eq!(h[0].type_lib_version, Some((2, 5)));
        assert_eq!(h[0].lcid, Some(0));
        assert_eq!(h[0].office_dir, Some(14));
        assert_eq!(h[1].office_dir, None);
        assert_eq!(h[2].guid, None);
        assert_eq!(h[2].type_lib_version, None);
    }

    #[test]
    fn office_dir_needs_a_whole_component() {
        assert_eq!(
            office_dir("C:\\x\\Microsoft Office\\root\\Office16\\a.dll"),
            Some(16)
        );
        assert_eq!(office_dir("C:\\x\\Microsoft Office\\a.dll"), None);
        assert_eq!(office_dir("C:\\x\\MyOffice12\\a.dll"), None);
        assert_eq!(office_dir("C:\\Office123\\a.dll"), None);
    }

    #[test]
    fn compiled_only_is_reported() {
        let mut p = VbaProject::new("P");
        assert!(!p.project_version().compiled_only);
        let mut m = crate::Module::new("M", crate::ModuleKind::Standard, "");
        m.has_source = false;
        m.preserved
            .insert(crate::PERFORMANCE_CACHE_KEY, vec![1, 2, 3]);
        p.modules.push(m);
        assert!(p.project_version().compiled_only);
    }
}

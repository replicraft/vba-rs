//! The `PROJECT` stream (MS-OVBA §2.3.1): line-oriented text properties, host
//! extenders and the workspace.
//!
//! Parsing is lenient about content it does not model (unknown properties and
//! sections are ignored, never rejected). The original bytes are preserved by
//! the caller; this module reads them and, for the few mirrored properties,
//! patches single lines in place.

use crate::codepage;
use crate::crypt;
use crate::error::Error;
use vba_project::{HostExtender, ModuleKind, Protection, VbaProject, WindowRecord};

/// Properties of the PROJECT stream that the model represents.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ProjectProps {
    pub id: Option<String>,
    /// Module entries in file order: kind and name.
    pub modules: Vec<(ModuleKind, String)>,
    /// First kind listed for each module name (ASCII lower-cased), so a
    /// lookup costs one hash instead of a scan of every line.
    kinds: std::collections::HashMap<String, ModuleKind>,
    pub help_file: Option<String>,
    pub name: Option<String>,
    pub help_context: Option<String>,
    pub description: Option<String>,
    pub protection: Protection,
    pub host_extenders: Vec<HostExtender>,
    pub workspace: Vec<WindowRecord>,
}

impl ProjectProps {
    /// The kind the stream lists for module `name` (case-insensitive).
    pub(crate) fn kind_of(&self, name: &str) -> Option<ModuleKind> {
        self.kinds.get(&name.to_ascii_lowercase()).copied()
    }
}

/// A line of the stream: the content range and the start of the next line.
struct Line {
    start: usize,
    end: usize,
    next: usize,
}

fn split_lines(raw: &[u8]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut pos = 0;
    while pos < raw.len() {
        let end = raw[pos..]
            .iter()
            .position(|&b| b == b'\r' || b == b'\n')
            .map_or(raw.len(), |i| pos + i);
        let mut next = end;
        if next < raw.len() {
            let first = raw[next];
            next += 1;
            // NWLN is CR LF or LF CR; a lone CR or LF is tolerated.
            if next < raw.len() && raw[next] != first && matches!(raw[next], b'\r' | b'\n') {
                next += 1;
            }
        }
        lines.push(Line {
            start: pos,
            end,
            next,
        });
        pos = next;
    }
    lines
}

fn unquote(value: &str) -> String {
    let v = value.trim();
    match v.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner) => inner.replace("\"\"", "\""),
        None => v.to_owned(),
    }
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Parses `&H` followed by hexadecimal digits.
fn parse_hex_int(s: &str) -> Option<u32> {
    let digits = s.trim().strip_prefix("&H")?;
    u32::from_str_radix(digits, 16).ok()
}

#[derive(PartialEq)]
enum Section {
    Properties,
    HostExtenders,
    Workspace,
    Other,
}

/// Parses the PROJECT stream bytes.
pub(crate) fn parse(raw: &[u8], code_page: u16) -> Result<ProjectProps, Error> {
    let mut props = ProjectProps::default();
    let mut section = Section::Properties;
    let (mut cmg, mut dpb, mut gc) = (None, None, None);
    for line in split_lines(raw) {
        let text = codepage::decode(code_page, &raw[line.start..line.end])?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = match trimmed {
                "[Host Extender Info]" => Section::HostExtenders,
                "[Workspace]" => Section::Workspace,
                _ => Section::Other,
            };
            continue;
        }
        let Some((key, value)) = text.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim_start());
        match section {
            Section::Properties => match key {
                "ID" => props.id = Some(unquote(value)),
                "Document" => {
                    let name = value.split('/').next().unwrap_or(value);
                    props
                        .modules
                        .push((ModuleKind::Document, name.trim().to_owned()));
                }
                "Module" => props
                    .modules
                    .push((ModuleKind::Standard, value.trim().to_owned())),
                "Class" => props
                    .modules
                    .push((ModuleKind::Class, value.trim().to_owned())),
                "BaseClass" => props
                    .modules
                    .push((ModuleKind::Designer, value.trim().to_owned())),
                "HelpFile" => props.help_file = Some(unquote(value)),
                "Name" => props.name = Some(unquote(value)),
                "HelpContextID" => props.help_context = Some(unquote(value)),
                "Description" => props.description = Some(unquote(value)),
                "CMG" => cmg = Some(unquote(value)),
                "DPB" => dpb = Some(unquote(value)),
                "GC" => gc = Some(unquote(value)),
                _ => {}
            },
            Section::HostExtenders => {
                if let Some(ext) = parse_host_extender(key, value) {
                    props.host_extenders.push(ext);
                }
            }
            Section::Workspace => props.workspace.push(WindowRecord {
                module: key.to_owned(),
                state: value.trim_end().to_owned(),
            }),
            Section::Other => {}
        }
    }
    for (kind, name) in &props.modules {
        props
            .kinds
            .entry(name.to_ascii_lowercase())
            .or_insert(*kind);
    }
    props.protection = decode_protection(cmg.as_deref(), dpb.as_deref(), gc.as_deref());
    Ok(props)
}

fn parse_host_extender(key: &str, value: &str) -> Option<HostExtender> {
    let index = parse_hex_int(key)?;
    let mut parts = value.trim().split(';');
    let guid = parts.next()?.to_owned();
    let lib_name = parts.next()?.to_owned();
    let creation_flags = parse_hex_int(parts.next()?)?;
    Some(HostExtender {
        index,
        guid,
        lib_name,
        creation_flags,
    })
}

/// Derives the declared protection state from the obfuscated CMG, DPB and GC
/// values (§2.3.1.15 to §2.3.1.17). Values that cannot be decoded are treated
/// conservatively: an undecodable DPB counts as a password being present.
fn decode_protection(cmg: Option<&str>, dpb: Option<&str>, gc: Option<&str>) -> Protection {
    let mut p = Protection::default();
    if let Some(data) = cmg
        .and_then(crypt::hex_decode)
        .and_then(|b| crypt::decrypt(&b))
        && data.len() >= 4
    {
        let flags = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        p.user_protected = flags & 1 != 0;
        p.host_protected = flags & 2 != 0;
        p.vbe_protected = flags & 4 != 0;
    }
    if let Some(dpb) = dpb {
        let data = crypt::hex_decode(dpb).and_then(|b| crypt::decrypt(&b));
        p.has_password = !matches!(data.as_deref(), Some([0]));
    }
    if let Some(data) = gc
        .and_then(crypt::hex_decode)
        .and_then(|b| crypt::decrypt(&b))
        && data.len() == 1
    {
        p.visible = data[0] != 0;
    }
    p
}

/// A single-line replacement for a mirrored property.
pub(crate) struct LineEdit<'a> {
    /// Property name, for example `Name`.
    pub key: &'a str,
    /// New value (unquoted).
    pub value: &'a str,
}

/// Rewrites the value of properties in place, keeping everything else (other
/// lines, line terminators, unknown content) byte for byte.
pub(crate) fn patch(raw: &[u8], code_page: u16, edits: &[LineEdit<'_>]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(raw.len() + 32);
    let mut pending: Vec<&LineEdit<'_>> = edits.iter().collect();
    let mut in_properties = true;
    for line in split_lines(raw) {
        let content = &raw[line.start..line.end];
        let text = codepage::decode(code_page, content)?;
        let trimmed = text.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_properties = false;
        }
        let hit = if in_properties {
            text.split_once('=')
                .and_then(|(k, _)| pending.iter().position(|e| e.key == k.trim()))
        } else {
            None
        };
        match hit {
            Some(i) => {
                let edit = pending.remove(i);
                out.extend(codepage::encode(
                    code_page,
                    &format!("{}={}", edit.key, quote(edit.value)),
                    "PROJECT stream value",
                )?);
            }
            None => out.extend_from_slice(content),
        }
        out.extend_from_slice(&raw[line.end..line.next]);
    }
    if let Some(e) = pending.first() {
        return Err(Error::UnsupportedEdit(format!(
            "the PROJECT stream has no {} property to update",
            e.key
        )));
    }
    Ok(out)
}

/// Generates a PROJECT stream for a project that has none (a project built in
/// memory rather than read from a file).
pub(crate) fn generate(project: &VbaProject) -> Result<Vec<u8>, Error> {
    use std::fmt::Write as _;
    let id = project
        .project_id
        .clone()
        .unwrap_or_else(|| "{00000000-0000-0000-0000-000000000000}".to_owned());
    let mut t = String::new();
    let nl = "\r\n";
    let _ = write!(t, "ID={}{nl}", quote(&id));
    for m in &project.modules {
        match m.kind {
            ModuleKind::Document => {
                let _ = write!(t, "Document={}/&H00000000{nl}", m.name);
            }
            ModuleKind::Standard => {
                let _ = write!(t, "Module={}{nl}", m.name);
            }
            ModuleKind::Class => {
                let _ = write!(t, "Class={}{nl}", m.name);
            }
            // Designer modules, and any kind added later, cannot be written
            // to a new project stream.
            _ => {
                return Err(Error::UnsupportedEdit(
                    "writing designer modules to a new project is not supported".into(),
                ));
            }
        }
    }
    if let Some(h) = &project.help_file {
        let _ = write!(t, "HelpFile={}{nl}", quote(h));
    }
    let _ = write!(t, "Name={}{nl}", quote(&project.name));
    let _ = write!(
        t,
        "HelpContextID={}{nl}",
        quote(&project.help_context.to_string())
    );
    if !project.description.is_empty() {
        let _ = write!(t, "Description={}{nl}", quote(&project.description));
    }
    let _ = write!(t, "VersionCompatible32=\"393222000\"{nl}");
    if project.protection.has_password {
        return Err(Error::UnsupportedEdit(
            "writing a project password is not supported".into(),
        ));
    }
    let p = &project.protection;
    let flags = u32::from(p.user_protected)
        | u32::from(p.host_protected) << 1
        | u32::from(p.vbe_protected) << 2;
    let cmg = crypt::encrypt(&flags.to_le_bytes(), 0x07, &id);
    let dpb = crypt::encrypt(&[0], 0x07, &id);
    let gc = crypt::encrypt(&[if p.visible { 0xFF } else { 0 }], 0x07, &id);
    let _ = write!(t, "CMG=\"{}\"{nl}", crypt::hex_encode(&cmg));
    let _ = write!(t, "DPB=\"{}\"{nl}", crypt::hex_encode(&dpb));
    let _ = write!(t, "GC=\"{}\"{nl}{nl}", crypt::hex_encode(&gc));
    let _ = write!(t, "[Host Extender Info]{nl}");
    for e in &project.host_extenders {
        let _ = write!(
            t,
            "&H{:08X}={};{};&H{:08X}{nl}",
            e.index, e.guid, e.lib_name, e.creation_flags
        );
    }
    if !project.workspace.is_empty() {
        let _ = write!(t, "{nl}[Workspace]{nl}");
        for w in &project.workspace {
            let _ = write!(t, "{}={}{nl}", w.module, w.state);
        }
    }
    codepage::encode(project.code_page, &t, "PROJECT stream")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MS-OVBA §3.1.6 PROJECT stream example.
    pub(crate) const SPEC_PROJECT: &str = "ID=\"{917DED54-440B-4FD1-A5C1-74ACF261E600}\"\r\nDocument=ThisWorkbook/&H00000000\r\nDocument=Sheet1/&H00000000\r\nPackage={AC9F2F90-E877-11CE-9F68-00AA00574A4F}\r\nBaseClass=UserForm1\r\nHelpFile=\"c:\\example path\\example.hlp\"\r\nName=\"VBAProject\"\r\nHelpContextID=\"1\"\r\nDescription=\"Example VBA Project Description\"\r\nVersionCompatible32=\"393222000\"\r\nCMG=\"0705D8E3D8EDDBF1DBF1DBF1DBF1\"\r\nDPB=\"0E0CD1ECDFF4E7F5E7F5E7\"\r\nGC=\"1517CAF1D6F9D7F9D706\"\r\n\r\n[Host Extender Info]\r\n&H00000001={3832D640-CF90-11CF-8E43-00A0C911005A};VBE;&H00000000\r\n\r\n[Workspace]\r\nThisWorkbook=23, 23, 911, 280, \r\nSheet1=69, 69, 724, 317, C\r\nUserForm1=0, 0, 0, 0, C, 46, 46, 701, 294, Z";

    #[test]
    fn parses_spec_example() {
        let p = parse(SPEC_PROJECT.as_bytes(), 1252).unwrap();
        assert_eq!(
            p.id.as_deref(),
            Some("{917DED54-440B-4FD1-A5C1-74ACF261E600}")
        );
        assert_eq!(
            p.modules,
            vec![
                (ModuleKind::Document, "ThisWorkbook".to_owned()),
                (ModuleKind::Document, "Sheet1".to_owned()),
                (ModuleKind::Designer, "UserForm1".to_owned()),
            ]
        );
        assert_eq!(
            p.help_file.as_deref(),
            Some("c:\\example path\\example.hlp")
        );
        assert_eq!(p.name.as_deref(), Some("VBAProject"));
        assert_eq!(p.help_context.as_deref(), Some("1"));
        assert_eq!(
            p.description.as_deref(),
            Some("Example VBA Project Description")
        );
        assert_eq!(p.protection, Protection::default());
        assert_eq!(
            p.host_extenders,
            vec![HostExtender {
                index: 1,
                guid: "{3832D640-CF90-11CF-8E43-00A0C911005A}".into(),
                lib_name: "VBE".into(),
                creation_flags: 0
            }]
        );
        assert_eq!(p.workspace.len(), 3);
        assert_eq!(p.workspace[2].module, "UserForm1");
        assert_eq!(p.workspace[2].state, "0, 0, 0, 0, C, 46, 46, 701, 294, Z");
    }

    #[test]
    fn line_terminators_are_tolerated() {
        let p = parse(
            b"Name=\"A\"\nModule=M\n\rClass=C\rHelpContextID=\"2\"",
            1252,
        )
        .unwrap();
        assert_eq!(p.name.as_deref(), Some("A"));
        assert_eq!(p.modules.len(), 2);
        assert_eq!(p.help_context.as_deref(), Some("2"));
    }

    #[test]
    fn unknown_content_is_ignored() {
        let p = parse(
            b"Bogus=1\r\nthis is not a property\r\n[Strange]\r\nName=\"x\"\r\n",
            1252,
        )
        .unwrap();
        assert_eq!(p.name, None);
    }

    #[test]
    fn quoted_quotes_round_trip() {
        let raw = b"Name=\"A\"\"B\"\r\n";
        assert_eq!(parse(raw, 1252).unwrap().name.as_deref(), Some("A\"B"));
        let patched = patch(
            raw,
            1252,
            &[LineEdit {
                key: "Name",
                value: "C\"D",
            }],
        )
        .unwrap();
        assert_eq!(patched, b"Name=\"C\"\"D\"\r\n");
    }

    #[test]
    fn patch_changes_only_the_named_line() {
        let raw = SPEC_PROJECT.as_bytes();
        let out = patch(
            raw,
            1252,
            &[LineEdit {
                key: "Name",
                value: "Renamed",
            }],
        )
        .unwrap();
        let expected = SPEC_PROJECT.replace("Name=\"VBAProject\"", "Name=\"Renamed\"");
        assert_eq!(out, expected.as_bytes());
        // A key in the workspace section with the same name is not touched.
        let raw = b"Name=\"A\"\r\n[Workspace]\r\nName=1, 2, 3, 4\r\n";
        let out = patch(
            raw,
            1252,
            &[LineEdit {
                key: "Name",
                value: "B",
            }],
        )
        .unwrap();
        assert_eq!(out, b"Name=\"B\"\r\n[Workspace]\r\nName=1, 2, 3, 4\r\n");
    }

    #[test]
    fn patch_reports_missing_property() {
        let err = patch(
            b"Name=\"A\"\r\n",
            1252,
            &[LineEdit {
                key: "Description",
                value: "x",
            }],
        )
        .unwrap_err();
        assert!(matches!(err, Error::UnsupportedEdit(_)));
    }

    #[test]
    fn protection_is_detected() {
        let id = "{00000000-0000-0000-0000-000000000000}";
        let cmg = crypt::hex_encode(&crypt::encrypt(&4u32.to_le_bytes(), 0x07, id));
        let dpb = crypt::hex_encode(&crypt::encrypt(&[1, 2, 3], 0x03, id));
        let gc = crypt::hex_encode(&crypt::encrypt(&[0], 0x01, id));
        let text = format!("ID=\"{id}\"\r\nCMG=\"{cmg}\"\r\nDPB=\"{dpb}\"\r\nGC=\"{gc}\"\r\n");
        let p = parse(text.as_bytes(), 1252).unwrap();
        assert!(p.protection.vbe_protected);
        assert!(!p.protection.user_protected);
        assert!(p.protection.has_password);
        assert!(!p.protection.visible);
        assert!(p.protection.is_locked());
    }
}

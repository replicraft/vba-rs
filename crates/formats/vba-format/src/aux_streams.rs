//! Typed access to the `PROJECTwm` and `PROJECTlk` streams (MS-OVBA §2.3.3 and
//! §2.3.2).
//!
//! Both streams are preserved verbatim by [`read_project`](crate::read_project)
//! and written back unchanged; this module only interprets them. Parsing never
//! affects reading or writing the project, so a malformed stream is an error
//! here and nothing else.

use vba_project::VbaProject;

use crate::codepage;
use crate::dir::guid_to_string;
use crate::error::Error;
use crate::keys;

/// Finds a root-level preserved stream by case-insensitive name.
pub(crate) fn root_stream<'a>(project: &'a VbaProject, name: &str) -> Option<&'a [u8]> {
    project
        .preserved
        .iter_prefixed(keys::STREAM_PREFIX)
        .find(|(path, _)| path.eq_ignore_ascii_case(name))
        .map(|(_, bytes)| bytes)
}

fn malformed(stream: &'static str, detail: &'static str) -> Error {
    Error::AuxStream { stream, detail }
}

/// One NAMEMAP record of `PROJECTwm` (§2.3.3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameMapEntry {
    /// The MBCS module name exactly as stored, without the terminator.
    pub name_mbcs: Vec<u8>,
    /// `name_mbcs` decoded with the project code page.
    pub name: String,
    /// The UTF-16 module name.
    pub name_unicode: String,
}

/// The `PROJECTwm` stream: module names in both encodings, in the order of
/// the `dir` stream's module records (§2.3.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectWm {
    /// One entry per module.
    pub entries: Vec<NameMapEntry>,
}

impl ProjectWm {
    /// Parses the stream. `code_page` decodes the MBCS names.
    pub fn parse(bytes: &[u8], code_page: u16) -> Result<ProjectWm, Error> {
        const S: &str = "PROJECTwm";
        let body = bytes
            .len()
            .checked_sub(2)
            .filter(|&n| bytes[n..] == [0, 0])
            .map(|n| &bytes[..n])
            .ok_or_else(|| malformed(S, "missing the 0x0000 terminator"))?;
        let mut entries = Vec::new();
        let mut rest = body;
        while !rest.is_empty() {
            let nul = rest
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| malformed(S, "unterminated MBCS name"))?;
            if nul == 0 {
                return Err(malformed(S, "empty MBCS name"));
            }
            let mbcs = &rest[..nul];
            rest = &rest[nul + 1..];
            let mut units = Vec::new();
            loop {
                let Some((pair, tail)) = rest.split_first_chunk::<2>() else {
                    return Err(malformed(S, "unterminated Unicode name"));
                };
                rest = tail;
                let unit = u16::from_le_bytes(*pair);
                if unit == 0 {
                    break;
                }
                units.push(unit);
            }
            if units.is_empty() {
                return Err(malformed(S, "empty Unicode name"));
            }
            entries.push(NameMapEntry {
                name_mbcs: mbcs.to_vec(),
                name: codepage::decode(code_page, mbcs)?,
                name_unicode: String::from_utf16_lossy(&units),
            });
        }
        Ok(ProjectWm { entries })
    }

    /// Serializes the stream; `parse` followed by `to_bytes` reproduces the
    /// input for any stream that parses (names that are not valid UTF-16 are
    /// replaced when parsing, so such streams are the one exception).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for e in &self.entries {
            out.extend_from_slice(&e.name_mbcs);
            out.push(0);
            for u in e.name_unicode.encode_utf16() {
                out.extend_from_slice(&u.to_le_bytes());
            }
            out.extend_from_slice(&[0, 0]);
        }
        out.extend_from_slice(&[0, 0]);
        out
    }
}

/// One LICENSEINFO record of `PROJECTlk` (§2.3.2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseInfo {
    /// CLSID of the ActiveX control, as `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`.
    pub class_id: String,
    /// The control's license key, opaque.
    pub license_key: Vec<u8>,
    /// Whether the control may only be created license-aware (stored as a
    /// 32-bit value; any non-zero value is `true`).
    pub license_required: bool,
}

/// The `PROJECTlk` stream: license information of ActiveX controls (§2.3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseKeys {
    /// The records, in stored order.
    pub records: Vec<LicenseInfo>,
    /// Bytes after the last record (empty in conforming streams).
    pub trailing: Vec<u8>,
}

impl LicenseKeys {
    /// Parses the stream. The version must be 1.
    pub fn parse(bytes: &[u8]) -> Result<LicenseKeys, Error> {
        const S: &str = "PROJECTlk";
        let mut r = Cursor {
            data: bytes,
            pos: 0,
        };
        let version = u16::from_le_bytes(r.array::<2>(S)?);
        if version != 1 {
            return Err(malformed(S, "unsupported version (must be 1)"));
        }
        let count = u32::from_le_bytes(r.array::<4>(S)?);
        let mut records = Vec::with_capacity((count as usize).min(bytes.len() / 24));
        for _ in 0..count {
            let class_id = guid_to_string(&r.array::<16>(S)?);
            let size = u32::from_le_bytes(r.array::<4>(S)?) as usize;
            let license_key = r.take(size, S)?.to_vec();
            let required = u32::from_le_bytes(r.array::<4>(S)?);
            records.push(LicenseInfo {
                class_id,
                license_key,
                license_required: required != 0,
            });
        }
        Ok(LicenseKeys {
            records,
            trailing: bytes[r.pos..].to_vec(),
        })
    }

    /// Serializes the stream. `license_required` is written as 0 or 1, so
    /// streams that stored another non-zero value are not reproduced exactly;
    /// the original bytes stay available in the model's preserved data.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut out = vec![1, 0];
        out.extend_from_slice(&(self.records.len() as u32).to_le_bytes());
        for r in &self.records {
            out.extend_from_slice(&crate::dir::guid_from_string(&r.class_id).ok_or_else(|| {
                Error::UnsupportedEdit(format!("invalid class id {:?}", r.class_id))
            })?);
            out.extend_from_slice(&(r.license_key.len() as u32).to_le_bytes());
            out.extend_from_slice(&r.license_key);
            out.extend_from_slice(&u32::from(r.license_required).to_le_bytes());
        }
        out.extend_from_slice(&self.trailing);
        Ok(out)
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize, stream: &'static str) -> Result<&'a [u8], Error> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or_else(|| malformed(stream, "truncated"))?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn array<const N: usize>(&mut self, stream: &'static str) -> Result<[u8; N], Error> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N, stream)?);
        Ok(a)
    }
}

/// The module-name map of a project read from a file, if it has a
/// `PROJECTwm` stream.
pub fn module_name_map(project: &VbaProject) -> Result<Option<ProjectWm>, Error> {
    root_stream(project, "PROJECTwm")
        .map(|b| ProjectWm::parse(b, project.code_page))
        .transpose()
}

/// The ActiveX license information of a project read from a file, if it has
/// a `PROJECTlk` stream.
pub fn license_keys(project: &VbaProject) -> Result<Option<LicenseKeys>, Error> {
    root_stream(project, "PROJECTlk")
        .map(LicenseKeys::parse)
        .transpose()
}

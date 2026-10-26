//! VBA project digital signature streams: structure only, no verification.
//!
//! The container structure parsed here is `DigSigBlob` /
//! `DigSigInfoSerialized` of MS-OSHARED §2.3.2.1 and §2.3.2.2. The names of
//! the streams that carry it (`\u{5}DigitalSignature`, `\u{5}DigitalSignatureEx`
//! and `\u{5}DigitalSignatureAgile`) are *not* defined in MS-OVBA or
//! MS-OSHARED; they follow the names Office writes, and a stream is recognized
//! by name alone.
//!
//! Nothing here checks a signature: the PKCS #7 `SignedData` is returned as
//! opaque bytes, certificates are not parsed or validated, and the content
//! hash of MS-OVBA §2.4.2 is not computed. A present signature therefore says
//! nothing about trust or integrity.

use vba_project::VbaProject;

use crate::error::Error;
use crate::keys;
use crate::options::Limits;
use crate::read::{open_and_list, read_stream};

/// Which signature stream a [`SignatureStream`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureKind {
    /// `\u{5}DigitalSignature`.
    Legacy,
    /// `\u{5}DigitalSignatureEx`.
    Ex,
    /// `\u{5}DigitalSignatureAgile`.
    Agile,
}

impl SignatureKind {
    /// Recognizes a stream name (the last path component, ignoring leading
    /// control characters and ASCII case).
    pub fn from_stream_name(path: &str) -> Option<SignatureKind> {
        let name = path.rsplit('/').next()?;
        let name = name.trim_start_matches(|c: char| c.is_control());
        match name.to_ascii_lowercase().as_str() {
            "digitalsignature" => Some(Self::Legacy),
            "digitalsignatureex" => Some(Self::Ex),
            "digitalsignatureagile" => Some(Self::Agile),
            _ => None,
        }
    }
}

/// The parsed `DigSigBlob` (MS-OSHARED §2.3.2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureBlob {
    /// The VBA digital signature: a DER PKCS #7 `SignedData`, unparsed and
    /// unverified.
    pub signature: Vec<u8>,
    /// The `VBASigSerializedCertStore` of the signing certificate, unparsed.
    pub cert_store: Vec<u8>,
    /// Size in bytes the structure declares for the project name buffer
    /// (the specification requires 0).
    pub project_name_size: u32,
    /// Size in bytes the structure declares for the timestamp URL buffer
    /// (the specification requires 0).
    pub timestamp_url_size: u32,
}

/// A signature stream found in a project or signature part.
#[derive(Debug)]
pub struct SignatureStream {
    /// Path of the stream.
    pub path: String,
    /// Which of the known names it has.
    pub kind: SignatureKind,
    /// Stream length in bytes.
    pub len: usize,
    /// The parsed structure, or why it could not be parsed. The raw bytes
    /// are preserved either way.
    pub blob: Result<SignatureBlob, Error>,
}

fn bad(detail: &'static str) -> Error {
    Error::AuxStream {
        stream: "DigitalSignature",
        detail,
    }
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn region(blob: &[u8], offset: u32, size: u32) -> Result<&[u8], Error> {
    let start = offset as usize;
    let end = start
        .checked_add(size as usize)
        .filter(|&e| e <= blob.len())
        .ok_or_else(|| bad("a buffer lies outside the structure"))?;
    Ok(&blob[start..end])
}

/// Parses the content of a signature stream as a `DigSigBlob`.
pub fn parse_blob(bytes: &[u8]) -> Result<SignatureBlob, Error> {
    const INFO: usize = 8;
    const INFO_LEN: usize = 36;
    let cb = u32_at(bytes, 0).ok_or_else(|| bad("too short"))? as usize;
    let pointer = u32_at(bytes, 4).ok_or_else(|| bad("too short"))?;
    if pointer != 8 {
        return Err(bad("serializedPointer is not 8"));
    }
    if cb < INFO_LEN || cb > bytes.len() - 8 {
        return Err(bad("cb does not fit the stream"));
    }
    let f = |i: usize| u32_at(bytes, INFO + 4 * i).ok_or_else(|| bad("too short"));
    let (cb_sig, sig_off) = (f(0)?, f(1)?);
    let (cb_cert, cert_off) = (f(2)?, f(3)?);
    let (cb_name, _name_off) = (f(4)?, f(5)?);
    let (cb_ts, _ts_off) = (f(7)?, f(8)?);
    Ok(SignatureBlob {
        signature: region(bytes, sig_off, cb_sig)?.to_vec(),
        cert_store: region(bytes, cert_off, cb_cert)?.to_vec(),
        project_name_size: cb_name,
        timestamp_url_size: cb_ts,
    })
}

fn from_streams<'a>(streams: impl Iterator<Item = (&'a str, &'a [u8])>) -> Vec<SignatureStream> {
    streams
        .filter_map(|(path, bytes)| {
            Some(SignatureStream {
                path: path.to_owned(),
                kind: SignatureKind::from_stream_name(path)?,
                len: bytes.len(),
                blob: parse_blob(bytes),
            })
        })
        .collect()
}

/// The signature streams inside a project read from a file (legacy binary
/// documents keep them in the root storage next to the VBA storage).
pub fn signatures(project: &VbaProject) -> Vec<SignatureStream> {
    from_streams(project.preserved.iter_prefixed(keys::STREAM_PREFIX))
}

/// Whether the project carries any signature stream.
pub fn is_signed(project: &VbaProject) -> bool {
    project
        .preserved
        .iter_prefixed(keys::STREAM_PREFIX)
        .any(|(path, _)| SignatureKind::from_stream_name(path).is_some())
}

/// Reads the signature streams of a standalone signature part, such as the
/// `vbaProjectSignature.bin` of an OOXML package (a compound file).
pub fn read_signature_part(bytes: &[u8], limits: &Limits) -> Result<Vec<SignatureStream>, Error> {
    let (mut cf, nodes) = open_and_list(bytes, limits)?;
    let mut found = Vec::new();
    for node in &nodes {
        if node.is_stream && SignatureKind::from_stream_name(&node.path).is_some() {
            found.push((node.path.clone(), read_stream(&mut cf, node, limits)?));
        }
    }
    Ok(from_streams(
        found.iter().map(|(p, b)| (p.as_str(), b.as_slice())),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A DigSigBlob laid out per MS-OSHARED §2.3.2.2 with the given buffers.
    pub(crate) fn blob(signature: &[u8], cert_store: &[u8]) -> Vec<u8> {
        let info_len = 36u32;
        let sig_off = 8 + info_len;
        let cert_off = sig_off + signature.len() as u32;
        let name_off = cert_off + cert_store.len() as u32;
        let ts_off = name_off + 2;
        let mut info = Vec::new();
        for v in [
            signature.len() as u32,
            sig_off,
            cert_store.len() as u32,
            cert_off,
            0,
            name_off,
            0,
            0,
            ts_off,
        ] {
            info.extend_from_slice(&v.to_le_bytes());
        }
        info.extend_from_slice(signature);
        info.extend_from_slice(cert_store);
        info.extend_from_slice(&[0, 0, 0, 0]); // name and URL: one null each
        while info.len() % 4 != 0 {
            info.push(0xAA);
        }
        let mut out = (info.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(&8u32.to_le_bytes());
        out.extend(info);
        out
    }

    #[test]
    fn names_are_recognized() {
        use SignatureKind::*;
        assert_eq!(
            SignatureKind::from_stream_name("\u{5}DigitalSignature"),
            Some(Legacy)
        );
        assert_eq!(
            SignatureKind::from_stream_name("\u{5}digitalsignatureEX"),
            Some(Ex)
        );
        assert_eq!(
            SignatureKind::from_stream_name("x/\u{5}DigitalSignatureAgile"),
            Some(Agile)
        );
        assert_eq!(SignatureKind::from_stream_name("DigitalSignatures"), None);
        assert_eq!(SignatureKind::from_stream_name("PROJECT"), None);
    }

    #[test]
    fn blob_fields_are_extracted() {
        let b = blob(b"\x30\x82pkcs7", b"certstore");
        let parsed = parse_blob(&b).unwrap();
        assert_eq!(parsed.signature, b"\x30\x82pkcs7");
        assert_eq!(parsed.cert_store, b"certstore");
        assert_eq!(
            (parsed.project_name_size, parsed.timestamp_url_size),
            (0, 0)
        );
    }

    #[test]
    fn malformed_blobs_are_errors() {
        let good = blob(b"sig", b"cert");
        assert!(parse_blob(&[]).is_err());
        assert!(parse_blob(&good[..10]).is_err());
        let mut b = good.clone();
        b[4] = 9; // serializedPointer
        assert!(parse_blob(&b).is_err());
        let mut b = good.clone();
        b[8..12].copy_from_slice(&u32::MAX.to_le_bytes()); // cbSignature
        assert!(parse_blob(&b).is_err());
        let mut b = good;
        b[0..4].copy_from_slice(&u32::MAX.to_le_bytes()); // cb
        assert!(parse_blob(&b).is_err());
    }
}

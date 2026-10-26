//! MS-OVBA §2.4.3 Data Encryption.
//!
//! VBA obfuscates the `CMG`, `DPB` and `GC` values of the PROJECT stream with
//! this reversible scheme. It is used here only to *report* a project's
//! protection state; passwords are never recovered or checked.

/// Decrypts an Encrypted Data Structure (§2.4.3.3).
///
/// Returns `None` when the structure is too short, has an unsupported
/// version, or declares more data than it holds.
pub fn decrypt(enc: &[u8]) -> Option<Vec<u8>> {
    let (&seed, rest) = enc.split_first()?;
    let (&version_enc, rest) = rest.split_first()?;
    let (&proj_key_enc, rest) = rest.split_first()?;
    if seed ^ version_enc != 2 {
        return None;
    }
    let proj_key = seed ^ proj_key_enc;
    let mut unenc1 = proj_key;
    let mut enc1 = proj_key_enc;
    let mut enc2 = version_enc;
    let mut step = |byte_enc: u8| {
        let byte = byte_enc ^ enc2.wrapping_add(unenc1);
        enc2 = enc1;
        enc1 = byte_enc;
        unenc1 = byte;
        byte
    };
    let ignored = usize::from((seed & 6) / 2);
    let mut it = rest.iter().copied();
    for _ in 0..ignored {
        step(it.next()?);
    }
    let mut length = 0u32;
    for index in 0..4 {
        length |= u32::from(step(it.next()?)) << (8 * index);
    }
    let data: Vec<u8> = it.map(&mut step).collect();
    let length = usize::try_from(length).ok()?;
    (data.len() >= length).then(|| data[..length].to_vec())
}

/// Encrypts `data` (§2.4.3.2).
///
/// `seed` selects the number of ignored bytes (`(seed & 6) / 2`), which are
/// filled with zeros so the output is deterministic. `project_id` is the
/// project CLSID text used for the project key checksum.
pub fn encrypt(data: &[u8], seed: u8, project_id: &str) -> Vec<u8> {
    encrypt_with_ignored(data, seed, project_id, 0)
}

/// Like [`encrypt`], with the ignored bytes set to `ignored` instead of zero
/// (the specification's examples use `0x07`).
pub fn encrypt_with_ignored(data: &[u8], seed: u8, project_id: &str, ignored: u8) -> Vec<u8> {
    let proj_key = project_id.bytes().fold(0u8, |a, b| a.wrapping_add(b));
    let version_enc = seed ^ 2;
    let proj_key_enc = seed ^ proj_key;
    let mut out = vec![seed, version_enc, proj_key_enc];
    let mut unenc1 = proj_key;
    let mut enc1 = proj_key_enc;
    let mut enc2 = version_enc;
    let mut step = |byte: u8| {
        let byte_enc = byte ^ enc2.wrapping_add(unenc1);
        enc2 = enc1;
        enc1 = byte_enc;
        unenc1 = byte;
        byte_enc
    };
    for _ in 0..(seed & 6) / 2 {
        out.push(step(ignored));
    }
    for b in (data.len() as u32).to_le_bytes() {
        out.push(step(b));
    }
    for &b in data {
        out.push(step(b));
    }
    out
}

/// Decodes a string of hexadecimal digits (either case).
pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}

/// Encodes bytes as upper-case hexadecimal.
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Values from the MS-OVBA §3.1.6 PROJECT stream example.
    const ID: &str = "{917DED54-440B-4FD1-A5C1-74ACF261E600}";

    #[test]
    fn spec_example_values_decrypt() {
        let cmg = decrypt(&hex_decode("0705D8E3D8EDDBF1DBF1DBF1DBF1").unwrap()).unwrap();
        assert_eq!(cmg, [0, 0, 0, 0]);
        let dpb = decrypt(&hex_decode("0E0CD1ECDFF4E7F5E7F5E7").unwrap()).unwrap();
        assert_eq!(dpb, [0]);
        let gc = decrypt(&hex_decode("1517CAF1D6F9D7F9D706").unwrap()).unwrap();
        assert_eq!(gc, [0xFF]);
    }

    /// The three examples of MS-OVBA section 3.1.6 reproduced byte for byte,
    /// with the decrypted fields the specification lists.
    #[test]
    fn spec_examples_reproduce_exactly() {
        // (hex, seed, plaintext, ignored count, ProjKey)
        let cases: [(&str, u8, &[u8], usize); 3] = [
            ("0705D8E3D8EDDBF1DBF1DBF1DBF1", 0x07, &[0, 0, 0, 0], 3),
            ("0E0CD1ECDFF4E7F5E7F5E7", 0x0E, &[0x00], 3),
            ("1517CAF1D6F9D7F9D706", 0x15, &[0xFF], 2),
        ];
        for (hex, seed, data, ignored) in cases {
            let bytes = hex_decode(hex).unwrap();
            assert_eq!(bytes[0], seed);
            assert_eq!(bytes[1], seed ^ 0x02, "Version is 2");
            assert_eq!(bytes[2], seed ^ 0xDF, "ProjKey is 0xDF");
            assert_eq!(usize::from((seed & 6) / 2), ignored);
            assert_eq!(decrypt(&bytes).unwrap(), data);
            assert_eq!(encrypt_with_ignored(data, seed, ID, 0x07), bytes, "{hex}");
        }
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        for seed in 0..=255u8 {
            for data in [&b""[..], &[0u8][..], b"hello world", &[0xFF; 29][..]] {
                let enc = encrypt(data, seed, ID);
                assert_eq!(decrypt(&enc).unwrap(), data, "seed {seed}");
            }
        }
    }

    #[test]
    fn encrypt_matches_spec_example_bytes() {
        // Re-encrypting the example plaintext with the example's seed and
        // project id reproduces the example's structure when the ignored
        // bytes (random in the example) are zero-equivalent in length.
        let enc = encrypt(&[0xFF], 0x15, ID);
        assert_eq!(enc[..3], [0x15, 0x17, 0xCA]);
        assert_eq!(decrypt(&enc).unwrap(), [0xFF]);
    }

    #[test]
    fn malformed_input_is_rejected() {
        assert_eq!(decrypt(&[]), None);
        assert_eq!(decrypt(&[1, 2]), None);
        // Wrong version.
        assert_eq!(decrypt(&[0, 0, 0, 0, 0, 0, 0, 0]), None);
        // Declares one data byte but holds none.
        let mut enc = encrypt(&[7], 0, ID);
        enc.pop();
        assert_eq!(decrypt(&enc), None);
    }

    #[test]
    fn hex_helpers() {
        assert_eq!(hex_decode("0aFf"), Some(vec![0x0A, 0xFF]));
        assert_eq!(hex_decode("abc"), None);
        assert_eq!(hex_decode("zz"), None);
        assert_eq!(hex_encode(&[0x0A, 0xFF]), "0AFF");
    }
}

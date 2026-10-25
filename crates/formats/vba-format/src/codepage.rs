//! Conversion between project text and bytes in a Windows code page.
//!
//! MS-OVBA stores most text as MBCS in the code page named by
//! PROJECTCODEPAGE (§2.3.4.2.1.5). Conversion uses `encoding_rs`; only the
//! code pages it implements are supported, and others are reported as
//! [`Error::UnsupportedCodePage`] rather than guessed.

use crate::error::Error;
use encoding_rs::Encoding;

/// Returns the converter for a Windows code page number.
pub fn encoding_for(code_page: u16) -> Result<&'static Encoding, Error> {
    use encoding_rs::*;
    Ok(match code_page {
        866 => IBM866,
        874 => WINDOWS_874,
        932 => SHIFT_JIS,
        936 => GBK,
        949 => EUC_KR,
        950 => BIG5,
        1250 => WINDOWS_1250,
        1251 => WINDOWS_1251,
        1252 => WINDOWS_1252,
        1253 => WINDOWS_1253,
        1254 => WINDOWS_1254,
        1255 => WINDOWS_1255,
        1256 => WINDOWS_1256,
        1257 => WINDOWS_1257,
        1258 => WINDOWS_1258,
        10000 => MACINTOSH,
        20866 => KOI8_R,
        21866 => KOI8_U,
        28592 => ISO_8859_2,
        28593 => ISO_8859_3,
        28594 => ISO_8859_4,
        28595 => ISO_8859_5,
        28596 => ISO_8859_6,
        28597 => ISO_8859_7,
        28598 => ISO_8859_8,
        28603 => ISO_8859_13,
        28605 => ISO_8859_15,
        51932 => EUC_JP,
        54936 => GB18030,
        65001 => UTF_8,
        other => return Err(Error::UnsupportedCodePage(other)),
    })
}

/// Decodes bytes in `code_page`. Invalid sequences become U+FFFD.
pub fn decode(code_page: u16, bytes: &[u8]) -> Result<String, Error> {
    let enc = encoding_for(code_page)?;
    let (text, _) = enc.decode_without_bom_handling(bytes);
    Ok(text.into_owned())
}

/// Encodes `text` in `code_page`, failing if any character is unmappable.
///
/// `what` names the text for the error message.
pub fn encode(code_page: u16, text: &str, what: &str) -> Result<Vec<u8>, Error> {
    let enc = encoding_for(code_page)?;
    let (bytes, _, unmappable) = enc.encode(text);
    if unmappable {
        return Err(Error::Unencodable {
            what: what.to_owned(),
            code_page,
        });
    }
    Ok(bytes.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_1252_round_trip() {
        let text = "Größe € ä";
        let bytes = encode(1252, text, "t").unwrap();
        assert_eq!(bytes[0], b'G');
        assert!(bytes.contains(&0x80)); // euro sign
        assert_eq!(decode(1252, &bytes).unwrap(), text);
    }

    #[test]
    fn shift_jis_round_trip() {
        let text = "日本語 ABC";
        let bytes = encode(932, text, "t").unwrap();
        assert!(bytes.len() > 7);
        assert_eq!(decode(932, &bytes).unwrap(), text);
    }

    #[test]
    fn unmappable_is_an_error() {
        assert!(matches!(
            encode(1252, "日本", "module"),
            Err(Error::Unencodable {
                code_page: 1252,
                ..
            })
        ));
    }

    #[test]
    fn unknown_code_page_is_an_error() {
        assert!(matches!(
            decode(437, b"x"),
            Err(Error::UnsupportedCodePage(437))
        ));
    }

    #[test]
    fn invalid_bytes_decode_lossily() {
        assert_eq!(decode(65001, &[b'a', 0xFF, b'b']).unwrap(), "a\u{FFFD}b");
    }
}

use crate::{Module, ModuleKind};

/// Why a standalone source file could not be turned into a [`Module`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StandaloneError {
    /// The file extension is not `.bas`, `.cls` or `.frm`.
    #[error("unsupported source file extension in {file_name:?}; expected .bas, .cls or .frm")]
    UnsupportedExtension {
        /// The offending file name.
        file_name: String,
    },
    /// A `.cls` or `.frm` header block was opened but never closed.
    #[error("unterminated header block in {file_name:?}")]
    UnterminatedHeader {
        /// The offending file name.
        file_name: String,
    },
    /// No module name could be determined.
    #[error("cannot determine module name for {file_name:?}")]
    NoName {
        /// The offending file name.
        file_name: String,
    },
}

pub(crate) fn parse(file_name: &str, text: &str) -> Result<Module, StandaloneError> {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let (stem, ext) = base.rsplit_once('.').unwrap_or((base, ""));
    let kind = match ext.to_ascii_lowercase().as_str() {
        "bas" => ModuleKind::Standard,
        "cls" => ModuleKind::Class,
        "frm" => ModuleKind::Designer,
        _ => {
            return Err(StandaloneError::UnsupportedExtension {
                file_name: file_name.to_owned(),
            });
        }
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let header_len = if kind == ModuleKind::Standard {
        0
    } else {
        header_length(text).ok_or_else(|| StandaloneError::UnterminatedHeader {
            file_name: file_name.to_owned(),
        })?
    };
    let (header, source) = text.split_at(header_len);
    let name = vb_name(source)
        .or_else(|| (!stem.is_empty()).then(|| stem.to_owned()))
        .ok_or_else(|| StandaloneError::NoName {
            file_name: file_name.to_owned(),
        })?;
    let mut module = Module::new(name, kind, source);
    if header_len > 0 {
        module.file_header = Some(header.to_owned());
    }
    Ok(module)
}

/// Byte length of the export header: an optional `VERSION` line followed by
/// a balanced `Begin`/`End` block. Zero when the text has no header. `None`
/// when a block is opened but not closed.
fn header_length(text: &str) -> Option<usize> {
    let mut pos = 0;
    let mut depth = 0usize;
    let mut seen_block = false;
    for line in text.split_inclusive('\n') {
        let upper = line.trim().to_ascii_uppercase();
        let opens = upper == "BEGIN" || upper.starts_with("BEGIN ");
        if depth == 0 {
            if !seen_block && upper.starts_with("VERSION ") && !upper.contains('=') {
                pos += line.len();
                continue;
            }
            if opens && !seen_block {
                depth = 1;
                seen_block = true;
                pos += line.len();
                continue;
            }
            return Some(pos);
        }
        pos += line.len();
        if opens {
            depth += 1;
        } else if upper == "END" {
            depth -= 1;
            if depth == 0 {
                return Some(pos);
            }
        }
    }
    if seen_block { None } else { Some(pos) }
}

/// Extracts the name from an `Attribute VB_Name = "..."` line.
fn vb_name(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let rest = strip_prefix_ci(line.trim_start(), "Attribute")?;
        let rest = strip_prefix_ci(rest.trim_start(), "VB_Name")?;
        let rest = rest.trim_start().strip_prefix('=')?.trim();
        let inner = rest.strip_prefix('"')?.strip_suffix('"')?;
        (!inner.is_empty()).then(|| inner.to_owned())
    })
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bas_uses_vb_name() {
        let m = parse(
            "x.bas",
            "Attribute VB_Name = \"Util\"\r\nSub A()\r\nEnd Sub\r\n",
        )
        .unwrap();
        assert_eq!(m.name, "Util");
        assert_eq!(m.kind, ModuleKind::Standard);
        assert!(m.file_header.is_none());
        assert!(m.source.starts_with("Attribute"));
    }

    #[test]
    fn bas_falls_back_to_stem() {
        let m = parse("dir/Mod1.BAS", "Sub A()\r\nEnd Sub\r\n").unwrap();
        assert_eq!(m.name, "Mod1");
    }

    #[test]
    fn cls_header_is_split_and_round_trips() {
        let text = "VERSION 1.0 CLASS\r\nBEGIN\r\n  MultiUse = -1  'True\r\nEND\r\nAttribute VB_Name = \"Thing\"\r\nOption Explicit\r\n";
        let m = parse("Thing.cls", text).unwrap();
        assert_eq!(m.name, "Thing");
        assert_eq!(m.kind, ModuleKind::Class);
        assert_eq!(
            m.file_header.as_deref(),
            Some("VERSION 1.0 CLASS\r\nBEGIN\r\n  MultiUse = -1  'True\r\nEND\r\n")
        );
        assert_eq!(
            m.source,
            "Attribute VB_Name = \"Thing\"\r\nOption Explicit\r\n"
        );
        assert_eq!(m.to_source_file(), text);
    }

    #[test]
    fn frm_nested_blocks() {
        let text = "VERSION 5.00\r\nBegin {C62A69F0-16DC-11CE-9E98-00AA00574A4F} UserForm1 \r\n   Caption = \"x\"\r\n   Begin Frame F\r\n   End\r\nEnd\r\nAttribute VB_Name = \"UserForm1\"\r\n";
        let m = parse("UserForm1.frm", text).unwrap();
        assert_eq!(m.kind, ModuleKind::Designer);
        assert_eq!(m.source, "Attribute VB_Name = \"UserForm1\"\r\n");
        assert_eq!(m.to_source_file(), text);
    }

    #[test]
    fn errors() {
        assert!(matches!(
            parse("a.txt", ""),
            Err(StandaloneError::UnsupportedExtension { .. })
        ));
        assert!(matches!(
            parse("a.cls", "VERSION 1.0 CLASS\r\nBEGIN\r\n"),
            Err(StandaloneError::UnterminatedHeader { .. })
        ));
        assert!(matches!(
            parse(".bas", "Sub A()\r\n"),
            Err(StandaloneError::NoName { .. })
        ));
    }

    #[test]
    fn cls_without_header_is_accepted() {
        let m = parse("A.cls", "Attribute VB_Name = \"A\"\r\n").unwrap();
        assert!(m.file_header.is_none());
    }
}

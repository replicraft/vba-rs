//! The text layout of `Print` lists: used by `Debug.Print` and by
//! `Print #` in the runtime.
//!
//! Rules (**doc**, unverified against a real VBA host):
//! numbers print with a leading space where the sign would be and a trailing
//! space; `Boolean` is `True`/`False`; dates use the short date and time
//! forms of `CStr`; `Null` prints as `Null`; `Empty` prints nothing; a comma
//! advances to the next 14-column print zone; `Tab(n)` moves to column `n`
//! (to the next line when the position is already past it) and `Spc(n)`
//! writes `n` spaces; a list that does not end with `;` or `,` ends the line.

use vba_value::{Variant, convert};

use crate::services::PrintItem;

/// The width of a print zone.
pub const ZONE: usize = 14;

/// Text of one value in a `Print` list.
pub fn print_value(v: &Variant) -> String {
    let text = |v: &Variant| {
        convert::to_vba_string(v)
            .map(|s| s.to_string_lossy())
            .unwrap_or_default()
    };
    match v {
        Variant::Empty => String::new(),
        Variant::Null => "Null".to_owned(),
        Variant::Boolean(b) => if *b { "True" } else { "False" }.to_owned(),
        Variant::String(s) => s.to_string_lossy(),
        Variant::Error(e) => format!("Error {}", e.number()),
        Variant::Date(_) => text(v),
        Variant::Byte(_)
        | Variant::Integer(_)
        | Variant::Long(_)
        | Variant::LongLong(_)
        | Variant::Single(_)
        | Variant::Double(_)
        | Variant::Currency(_)
        | Variant::Decimal(_) => {
            let t = text(v);
            if t.starts_with('-') {
                format!("{t} ")
            } else {
                format!(" {t} ")
            }
        }
        Variant::Object(_) | Variant::UserDefined(_) | Variant::Array(_) => String::new(),
    }
}

/// Formats a `Print` list starting at 0-based column `*col`; returns the
/// text (including the final newline when the list ends the line) and
/// updates `*col`. Lines end with `\n`.
pub fn format_print(items: &[PrintItem], col: &mut usize) -> String {
    format_print_width(items, col, 0)
}

/// Like [`format_print`] with a line width (`Width #n, w`): a value that
/// would cross `width` columns starts a new line first, and one longer than
/// `width` is broken into lines of `width`. `0` means no limit.
pub fn format_print_width(items: &[PrintItem], col: &mut usize, width: usize) -> String {
    let mut out = String::new();
    let mut newline = true;
    for it in items {
        newline = true;
        match it {
            PrintItem::Value(v) => {
                let t = print_value(v);
                let len = t.chars().count();
                if width > 0 && *col > 0 && *col + len > width {
                    out.push('\n');
                    *col = 0;
                }
                if width > 0 && len > width {
                    // MS-VBAL 5.4.5.8: the line ends when it reaches the
                    // maximum length and output goes on at the next line.
                    for ch in t.chars() {
                        if *col >= width {
                            out.push('\n');
                            *col = 0;
                        }
                        out.push(ch);
                        *col += 1;
                    }
                } else {
                    *col += len;
                    out.push_str(&t);
                }
            }
            PrintItem::Semicolon => newline = false,
            PrintItem::Comma => {
                let next = (*col / ZONE + 1) * ZONE;
                out.extend(std::iter::repeat_n(' ', next - *col));
                *col = next;
                newline = false;
            }
            PrintItem::Tab(None) => {
                let next = (*col / ZONE + 1) * ZONE;
                out.extend(std::iter::repeat_n(' ', next - *col));
                *col = next;
            }
            PrintItem::Tab(Some(n)) => {
                let target = (*n).max(1) as usize - 1;
                if target < *col {
                    out.push('\n');
                    *col = 0;
                }
                out.extend(std::iter::repeat_n(' ', target - *col));
                *col = target;
            }
            PrintItem::Spc(n) => {
                let n = (*n).max(0) as usize;
                out.extend(std::iter::repeat_n(' ', n));
                *col += n;
            }
        }
    }
    if newline {
        out.push('\n');
        *col = 0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: Variant) -> PrintItem {
        PrintItem::Value(x)
    }

    #[test]
    fn numbers_get_sign_space_and_trailing_space() {
        let mut c = 0;
        assert_eq!(
            format_print(&[v(Variant::Long(5)), v(Variant::Long(-5))], &mut c),
            " 5 -5 \n"
        );
    }

    #[test]
    fn separators_and_zones() {
        let mut c = 0;
        let items = [
            v(Variant::from_text("a")),
            PrintItem::Comma,
            v(Variant::from_text("b")),
            PrintItem::Semicolon,
        ];
        assert_eq!(format_print(&items, &mut c), "a             b");
        assert_eq!(c, 15);
    }

    #[test]
    fn tab_and_spc() {
        let mut c = 0;
        let items = [
            PrintItem::Tab(Some(5)),
            v(Variant::from_text("x")),
            PrintItem::Spc(2),
            v(Variant::from_text("y")),
        ];
        assert_eq!(format_print(&items, &mut c), "    x  y\n");
    }

    #[test]
    fn a_long_item_is_broken_at_the_width() {
        let mut c = 0;
        let items = [v(Variant::from_text("abcdefghijkl"))];
        assert_eq!(format_print_width(&items, &mut c, 5), "abcde\nfghij\nkl\n");
        // An item that fits on a line of its own moves there whole.
        let mut c = 0;
        let items = [v(Variant::from_text("abcd")), v(Variant::from_text("efgh"))];
        assert_eq!(format_print_width(&items, &mut c, 5), "abcd\nefgh\n");
    }

    #[test]
    fn empty_list_is_a_blank_line() {
        let mut c = 0;
        assert_eq!(format_print(&[], &mut c), "\n");
    }
}

//! Identifier-reference resolution (`pcode-format.md` section 5).

use crate::layout::{Bitness, Layout, VersionFamily};
use crate::project_stream::IdentTable;

/// Built-in names confirmed by observation.
///
/// Evidence kind for every entry: in the reference project (VBA 7.0, x86),
/// every instruction whose name operand has this built-in index sits on a
/// source line that contains the spelling as a token, and no other
/// candidate token is shared by all such lines. Entries observed fewer than
/// twice are noted in `docs/formats/pcode-status.md`. Indices are the value `k`
/// (`word >> 1`) after the VBA7 adjustment of section 5.2 of the format
/// document; no confirmed entry lies in the adjusted range.
///
/// The table is sorted by index. All other built-in indices are unknown and
/// are reported as such, never guessed.
pub const CONFIRMED_BUILTINS: &[(u16, &str)] = &[
    (8, "Append"),
    (12, "b"),
    (33, "Close"),
    (34, "Compare"),
    (41, "CVDate"),
    (44, "Database"),
    (45, "Date"),
    (63, "Dir"),
    (66, "DoEvents"),
    (72, "End"),
    (82, "f"),
    (86, "Format"),
    (88, "FreeFile"),
    (110, "Left"),
    (125, "Mid"),
    (131, "Name"),
    (154, "Randomize"),
    (174, "String"),
    (178, "Text"),
];

/// Looks up a confirmed built-in spelling.
pub fn builtin_name(index: u16) -> Option<&'static str> {
    CONFIRMED_BUILTINS
        .binary_search_by_key(&index, |&(i, _)| i)
        .ok()
        .map(|p| CONFIRMED_BUILTINS[p].1)
}

/// The word `0xFFFF` used as the name operand of the implicit-`Me`
/// `READ_SIMPLE` instruction `[obs]`.
pub const ME_WORD: u16 = 0xFFFF;

/// A resolved identifier reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Name {
    /// The `Me` sentinel word.
    Me,
    /// Word 0: no name (for example `On Error Resume Next`).
    None,
    /// A user identifier from the project identifier table.
    User(String),
    /// A confirmed built-in name.
    Builtin(&'static str),
    /// A built-in index with no confirmed spelling.
    UnknownBuiltin(u16),
    /// A user identifier index outside the table.
    OutOfRange(usize),
}

impl Name {
    /// The spelling when known.
    pub fn spelling(&self) -> Option<&str> {
        match self {
            Name::Me => Some("Me"),
            Name::User(s) => Some(s),
            Name::Builtin(s) => Some(s),
            _ => None,
        }
    }
}

/// Resolves identifier references against a project identifier table.
#[derive(Debug, Clone, Copy)]
pub struct Resolver<'a> {
    idents: &'a IdentTable,
    layout: Layout,
}

impl<'a> Resolver<'a> {
    /// Creates a resolver.
    pub fn new(idents: &'a IdentTable, layout: Layout) -> Self {
        Self { idents, layout }
    }

    /// Maps the user-identifier part `j = k - 0x100` to a table index, or
    /// `None` when the adjustment underflows.
    ///
    /// Section 5.2 of the format document: for VBA7 subtract 4 (x64: 3 more)
    /// and, above 0xBE, one more. Observation of the reference project (VBA
    /// 7.0 x86, version word 0x97) shows only the subtraction of 4; the
    /// extra step is therefore applied only to layouts other than that
    /// verified one, where it is unverified.
    fn user_index(&self, j: usize) -> Option<usize> {
        if self.layout.family != VersionFamily::Vba7 {
            return Some(j);
        }
        let mut i = j.checked_sub(4)?;
        if self.layout.bitness == Bitness::X64 {
            i = i.checked_sub(3)?;
        }
        let verified = self.layout.version == crate::layout::VERIFIED_VERSION
            && self.layout.bitness == Bitness::X86;
        if !verified && i > 0xBE {
            i -= 1;
        }
        Some(i)
    }

    /// Resolves a name word.
    pub fn resolve(&self, word: u16) -> Name {
        if word == ME_WORD {
            return Name::Me;
        }
        let mut k = word >> 1;
        if k >= 0x100 {
            let j = usize::from(k - 0x100);
            return match self.user_index(j) {
                Some(i) => match self.idents.get(i) {
                    Some(s) => Name::User(s),
                    None => Name::OutOfRange(i),
                },
                None => Name::OutOfRange(j),
            };
        }
        if word == 0 {
            return Name::None;
        }
        if self.layout.family == VersionFamily::Vba7 && k >= 0xC3 {
            k -= 1;
        }
        match builtin_name(k) {
            Some(s) => Name::Builtin(s),
            None => Name::UnknownBuiltin(k),
        }
    }
}

/// The type-suffix character selected by the modifier of an instruction
/// whose operand is a name (spec 3.1), and whether the name is bracketed.
pub fn name_suffix(modifier: u8) -> (Option<char>, bool) {
    match modifier {
        2 => (Some('%'), false),
        3 => (Some('&'), false),
        4 => (Some('!'), false),
        5 => (Some('#'), false),
        6 => (Some('@'), false),
        8 => (Some('$'), false),
        32 => (None, true),
        _ => (None, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(n: usize) -> IdentTable {
        IdentTable {
            names: (0..n).map(|i| format!("id{i}").into_bytes()).collect(),
            announced: n,
            dummies: 0,
            end: 0,
            stream_len: 0,
        }
    }

    #[test]
    fn builtin_table_is_sorted_and_unique() {
        assert!(CONFIRMED_BUILTINS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(builtin_name(33), Some("Close"));
        assert_eq!(builtin_name(35), None);
    }

    #[test]
    fn resolves_user_builtin_and_sentinels() {
        let t = table(10);
        let r = Resolver::new(&t, Layout::select(0x97, 1).unwrap());
        // k = 0x100 + 4 + 3 -> id3
        assert_eq!(r.resolve((0x100 + 4 + 3) << 1), Name::User("id3".into()));
        assert_eq!(r.resolve((0x100 + 4 + 99) << 1), Name::OutOfRange(99));
        assert_eq!(r.resolve((0x100 + 2) << 1), Name::OutOfRange(2));
        assert_eq!(r.resolve(33 << 1), Name::Builtin("Close"));
        assert_eq!(r.resolve(35 << 1), Name::UnknownBuiltin(35));
        assert_eq!(r.resolve(0xFFFF), Name::Me);
        assert_eq!(r.resolve(0), Name::None);
    }

    #[test]
    fn suffix_selection() {
        assert_eq!(name_suffix(8), (Some('$'), false));
        assert_eq!(name_suffix(32), (None, true));
        assert_eq!(name_suffix(1), (None, false));
    }
}

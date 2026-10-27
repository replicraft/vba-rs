//! Identifier-reference resolution (`pcode-format.md` section 5).

use crate::layout::Layout;
use crate::project_stream::IdentTable;

/// Built-in names confirmed by observation.
///
/// Evidence kind for every entry: in observed material (VBA 6 and 7, 32-bit
/// and 64-bit), every instruction whose name operand has this built-in index
/// sits on a source line that contains the spelling as a token, and no other
/// identifier-like candidate token is shared by all such lines. Entries
/// observed fewer than twice are noted in `docs/formats/pcode-status.md`.
/// Indices are the value `k` (`word >> 1`) after the VBA7 adjustment of
/// section 5.2 of the format document; no confirmed entry lies in the
/// adjusted range.
///
/// The table is sorted by index. All other built-in indices are unknown and
/// are reported as such, never guessed.
pub const CONFIRMED_BUILTINS: &[(u16, &str)] = &[
    (2, "Abs"),
    (3, "Access"),
    (8, "Append"),
    (9, "Array"),
    (11, "Assert"),
    (12, "b"),
    (13, "Base"),
    (33, "Close"),
    (34, "Compare"),
    (37, "CStr"),
    (38, "CurDir"),
    (41, "CVDate"),
    (44, "Database"),
    (45, "Date"),
    (63, "Dir"),
    (66, "DoEvents"),
    (72, "End"),
    (77, "Error"),
    (81, "Explicit"),
    (82, "f"),
    (86, "Format"),
    (88, "FreeFile"),
    (95, "GoTo"),
    (104, "InStr"),
    (110, "Left"),
    (116, "Line"),
    (118, "Load"),
    (119, "Local"),
    (125, "Mid"),
    (131, "Name"),
    (135, "Nothing"),
    (136, "Null"),
    (137, "Object"),
    (139, "Open"),
    (154, "Randomize"),
    (160, "Rgb"),
    (164, "Select"),
    (172, "Stop"),
    (173, "StrComp"),
    (174, "String"),
    (178, "Text"),
    (182, "Type"),
    (185, "Unload"),
    // 9 uses, all on lines with the token; no other token shared by all
    // of them (the dot and `Me` are separate operands).
    (192, "Width"),
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
    /// `None` when the adjustment underflows (`Layout::ident_index_adjust`).
    fn user_index(&self, j: usize) -> Option<usize> {
        j.checked_sub(self.layout.ident_index_adjust())
    }

    /// Resolves a name word stored in a record (procedure, argument,
    /// variable, type or library name). Records have no `Me`: the result
    /// record of a function (pcode-format.md 6.6) carries the sentinel
    /// `0xFFFE` on x86 and `0xFFFF` on x64, and the second resolves like the
    /// first, to a name outside the identifier table.
    pub fn resolve_symbol(&self, word: u16) -> Name {
        self.resolve(if word == ME_WORD { word - 1 } else { word })
    }

    /// Resolves a name word of an instruction operand.
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
        if self.layout.has_vba7_names() && k >= 0xC3 {
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
    fn x64_user_index_and_record_sentinel() {
        let t = table(400);
        let r = Resolver::new(&t, Layout::select(0xB5, 3).unwrap());
        // 4 + 3 at the first index and far above 0xBE.
        assert_eq!(r.resolve((0x100 + 7) << 1), Name::User("id0".into()));
        assert_eq!(
            r.resolve((0x100 + 7 + 0xC0) << 1),
            Name::User("id192".into())
        );
        let r86 = Resolver::new(&t, Layout::select(0xAF, 1).unwrap());
        assert_eq!(
            r86.resolve((0x100 + 4 + 0xC0) << 1),
            Name::User("id192".into())
        );
        // Records have no `Me`; the result sentinel is a name outside the table.
        assert_eq!(r.resolve(0xFFFF), Name::Me);
        assert!(matches!(r.resolve_symbol(0xFFFF), Name::OutOfRange(_)));
        assert_eq!(r.resolve_symbol(0xFFFE), r.resolve_symbol(0xFFFF));
    }

    #[test]
    fn suffix_selection() {
        assert_eq!(name_suffix(8), (Some('$'), false));
        assert_eq!(name_suffix(32), (None, true));
        assert_eq!(name_suffix(1), (None, false));
    }
}

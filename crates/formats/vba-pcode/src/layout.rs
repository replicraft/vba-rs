//! Layout selection (`pcode-format.md` section 2).

use crate::error::{PcodeError, Result, UnsupportedLayout};

/// Compiler family derived from the version word of the project stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VersionFamily {
    /// Version word below 0x6B.
    Vba5,
    /// Version word 0x6B to 0x96.
    Vba6,
    /// Version word 0x97 and above.
    Vba7,
}

/// Pointer width of the producing build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bitness {
    /// 32-bit Windows layout.
    X86,
    /// 64-bit Windows layout.
    X64,
}

/// How well a layout is backed by test material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verification {
    /// Checked against real material with the source side by side: the
    /// version word and platform kind were observed in compiled projects
    /// whose decoded names, literals, records and line structure agree with
    /// their source ([`VERIFIED_LAYOUTS`]).
    Verified,
    /// Implemented from the specification text only, or structurally the
    /// same as a verified layout but not seen with this version word.
    SpecUnverified,
}

/// The layout parameters that drive every parser in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Layout {
    /// The version word of the project stream.
    pub version: u16,
    /// Compiler family.
    pub family: VersionFamily,
    /// Pointer width.
    pub bitness: Bitness,
}

/// Version word of the layout that was verified first (Office 2010, VBA 7.0,
/// 32-bit).
pub const VERIFIED_VERSION: u16 = 0x0097;

/// Lowest version word seen with the later identifier numbering
/// ([`Layout::ident_index_adjust`]); 0x88 and below use none.
pub const IDENT_ADJUST_FROM: u16 = 0x8E;

/// Lowest version word seen with the extended module header (bit 0x10 set in
/// the word at offset 2, declaration-table length at 0x43 instead of 0x3F).
pub const EXTENDED_HEADER_FROM: u16 = 0xDF;

/// The (family, bitness, version word) combinations that were observed in
/// real projects and verified against their source. The bitness is the
/// `PROJECTSYSKIND` of the project, not a function of the version word: the
/// same version word occurs with both platform kinds (`0xB2` was seen on
/// 32-bit and on 64-bit Windows).
pub const VERIFIED_LAYOUTS: &[(VersionFamily, Bitness, u16)] = &[
    (VersionFamily::Vba6, Bitness::X86, 0x79),
    (VersionFamily::Vba6, Bitness::X86, 0x88),
    (VersionFamily::Vba7, Bitness::X86, 0x97),
    (VersionFamily::Vba7, Bitness::X86, 0xAF),
    (VersionFamily::Vba7, Bitness::X86, 0xB2),
    (VersionFamily::Vba7, Bitness::X64, 0x9A),
    (VersionFamily::Vba7, Bitness::X64, 0xA6),
    (VersionFamily::Vba7, Bitness::X64, 0xB2),
    (VersionFamily::Vba7, Bitness::X64, 0xB5),
];
impl Layout {
    /// Selects the layout from the project-stream version word and the
    /// `PROJECTSYSKIND` value of the directory stream. The platform kind
    /// alone selects the bitness; the version word selects the family.
    ///
    /// # Errors
    ///
    /// * [`PcodeError::CacheNotUsable`] for version `0xFFFF`.
    /// * [`PcodeError::UnsupportedLayout`] for 16-bit Windows, Macintosh
    ///   (big-endian) and unknown platform kinds.
    pub fn select(version: u16, sys_kind: u32) -> Result<Self> {
        if version == 0xFFFF {
            return Err(PcodeError::CacheNotUsable);
        }
        let bitness = match sys_kind {
            1 => Bitness::X86,
            3 => Bitness::X64,
            2 => return Err(PcodeError::UnsupportedLayout(UnsupportedLayout::BigEndian)),
            other => {
                return Err(PcodeError::UnsupportedLayout(
                    UnsupportedLayout::PlatformKind(other),
                ));
            }
        };
        let family = if version < 0x6B {
            VersionFamily::Vba5
        } else if version < 0x97 {
            VersionFamily::Vba6
        } else {
            VersionFamily::Vba7
        };
        if family == VersionFamily::Vba5 && bitness == Bitness::X64 {
            return Err(PcodeError::UnsupportedLayout(UnsupportedLayout::Version(
                version,
            )));
        }
        Ok(Self {
            version,
            family,
            bitness,
        })
    }

    /// Whether this layout has been checked against real material.
    pub fn verification(&self) -> Verification {
        if VERIFIED_LAYOUTS.contains(&(self.family, self.bitness, self.version)) {
            Verification::Verified
        } else {
            Verification::SpecUnverified
        }
    }

    /// Whether identifier references use the numbering of the later
    /// compilers: the user-identifier index is `j - 4` (x86) or `j - 7`
    /// (x64) instead of `j`. `[obs]`: offset 0 for 0x5E, 0x6D, 0x79 and 0x88,
    /// offset 4 from 0x8E on; the boundary lies between 0x89 and 0x8E.
    pub(crate) fn has_vba7_names(&self) -> bool {
        self.version >= IDENT_ADJUST_FROM
    }

    /// Amount subtracted from `k - 0x100` to get the identifier-table index.
    pub(crate) fn ident_index_adjust(&self) -> usize {
        match (self.has_vba7_names(), self.bitness) {
            (false, _) => 0,
            (true, Bitness::X86) => 4,
            (true, Bitness::X64) => 7,
        }
    }

    /// `wideRefs` flag of the project-stream walk (spec 3.1).
    pub(crate) fn wide_refs(&self) -> bool {
        let v = self.version;
        (v >= 0x5B && !matches!(v, 0x60 | 0x62 | 0x63)) || v == 0x4E
    }

    /// `wideNames` flag of the project-stream walk (spec 3.1).
    pub(crate) fn wide_names(&self) -> bool {
        let v = self.version;
        (v >= 0x59 && !matches!(v, 0x60 | 0x62 | 0x63)) || v == 0x4E
    }

    /// `narrowNames` flag of the project-stream walk (spec 3.1).
    pub(crate) fn narrow_names(&self) -> bool {
        self.version <= 0x59 && self.version != 0x4E
    }

    /// Translates a stored opcode number to the canonical numbering.
    ///
    /// Returns `None` when the stored number is not a valid opcode of the
    /// producer. The VBA5 table is implemented from the specification and is
    /// unverified.
    pub fn canonical_opcode(&self, stored: u16) -> Option<u16> {
        let s = stored;
        match (self.family, self.bitness) {
            (VersionFamily::Vba5, _) => match s {
                0..=68 => Some(s),
                69..=71 => Some(s + 1),
                72..=112 => Some(s + 3),
                113..=151 => Some(s + 7),
                152..=165 => Some(s + 8),
                166..=167 => Some(s + 9),
                168..=170 => Some(s + 10),
                171..=252 => Some(s + 11),
                _ => None,
            },
            (_, Bitness::X64) => (s <= 263).then_some(s),
            (_, Bitness::X86) => match s {
                0..=173 => Some(s),
                174..=175 => Some(s + 1),
                176..=178 => Some(s + 2),
                179..=260 => Some(s + 3),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection() {
        let l = Layout::select(0x97, 1).unwrap();
        assert_eq!(l.family, VersionFamily::Vba7);
        assert_eq!(l.bitness, Bitness::X86);
        assert_eq!(l.verification(), Verification::Verified);
        // The platform kind, not the version word, selects the bitness.
        let b2_32 = Layout::select(0xB2, 1).unwrap();
        let b2_64 = Layout::select(0xB2, 3).unwrap();
        assert_eq!(b2_32.bitness, Bitness::X86);
        assert_eq!(b2_64.bitness, Bitness::X64);
        assert_eq!(b2_32.verification(), Verification::Verified);
        assert_eq!(b2_64.verification(), Verification::Verified);
        // A combination never observed stays unverified.
        assert_eq!(
            Layout::select(0xB5, 1).unwrap().verification(),
            Verification::SpecUnverified
        );
        assert_eq!(
            Layout::select(0x97, 3).unwrap().verification(),
            Verification::SpecUnverified
        );
        assert_eq!(
            Layout::select(0x6A, 1).unwrap().verification(),
            Verification::SpecUnverified
        );
        assert_eq!(Layout::select(0x6A, 1).unwrap().family, VersionFamily::Vba5);
        assert_eq!(Layout::select(0x6B, 1).unwrap().family, VersionFamily::Vba6);
        assert_eq!(Layout::select(0xD7, 3).unwrap().bitness, Bitness::X64);
        assert_eq!(Layout::select(0xD7, 1).unwrap().bitness, Bitness::X86);
        assert_eq!(Layout::select(0xFFFF, 1), Err(PcodeError::CacheNotUsable));
        assert!(matches!(
            Layout::select(0x97, 2),
            Err(PcodeError::UnsupportedLayout(UnsupportedLayout::BigEndian))
        ));
        assert!(Layout::select(0x97, 0).is_err());
        assert!(Layout::select(0x40, 3).is_err());
    }

    #[test]
    fn opcode_translation() {
        let x86 = Layout::select(0x97, 1).unwrap();
        assert_eq!(x86.canonical_opcode(173), Some(173));
        assert_eq!(x86.canonical_opcode(174), Some(175));
        assert_eq!(x86.canonical_opcode(176), Some(178));
        assert_eq!(x86.canonical_opcode(205), Some(208));
        assert_eq!(x86.canonical_opcode(260), Some(263));
        assert_eq!(x86.canonical_opcode(261), None);
        let x64 = Layout::select(0xB5, 3).unwrap();
        assert_eq!(x64.canonical_opcode(174), Some(174));
        assert_eq!(x64.canonical_opcode(264), None);
    }
}

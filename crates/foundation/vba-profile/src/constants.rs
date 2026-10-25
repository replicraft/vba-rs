use crate::{Bitness, LanguageGeneration, OperatingSystem, Profile};

/// A predefined conditional-compilation constant (MS-VBAL §4.1 and the
/// Microsoft VBA compiler-constants documentation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CompilationConstant {
    /// `VBA6`: VBA 6 or later.
    Vba6,
    /// `VBA7`: VBA 7 or later.
    Vba7,
    /// `Win16`: 16-bit Windows.
    Win16,
    /// `Win32`: Windows, 32-bit or 64-bit.
    Win32,
    /// `Win64`: 64-bit Windows.
    Win64,
    /// `Mac`: macOS.
    Mac,
}

impl CompilationConstant {
    /// All predefined constants.
    pub const ALL: [CompilationConstant; 6] = [
        CompilationConstant::Vba6,
        CompilationConstant::Vba7,
        CompilationConstant::Win16,
        CompilationConstant::Win32,
        CompilationConstant::Win64,
        CompilationConstant::Mac,
    ];

    /// The canonical spelling.
    pub const fn name(self) -> &'static str {
        match self {
            CompilationConstant::Vba6 => "VBA6",
            CompilationConstant::Vba7 => "VBA7",
            CompilationConstant::Win16 => "Win16",
            CompilationConstant::Win32 => "Win32",
            CompilationConstant::Win64 => "Win64",
            CompilationConstant::Mac => "Mac",
        }
    }

    /// Looks a constant up by name, ignoring ASCII case.
    pub fn from_name(name: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|c| c.name().as_bytes().eq_ignore_ascii_case(name))
    }

    /// Evaluates the constant for `profile`.
    ///
    /// Returns `None` when the constant does not exist in that
    /// implementation (`VBA6` before VBA 6, `VBA7` before VBA 7). A
    /// conditional-compilation expression treats an undefined name as Empty,
    /// which is false. On macOS the `Win*` constants are false; this follows
    /// the Microsoft documentation and has not been verified on a real Mac
    /// host.
    pub fn evaluate(self, profile: &Profile) -> Option<bool> {
        let t = profile.target;
        let windows = t.os == OperatingSystem::Windows;
        match self {
            CompilationConstant::Vba6 => {
                (profile.generation >= LanguageGeneration::Vba6).then_some(true)
            }
            CompilationConstant::Vba7 => {
                (profile.generation >= LanguageGeneration::Vba7_0).then_some(true)
            }
            CompilationConstant::Win16 => Some(windows && t.bitness == Bitness::B16),
            CompilationConstant::Win32 => Some(windows && t.bitness != Bitness::B16),
            CompilationConstant::Win64 => Some(windows && t.bitness == Bitness::B64),
            CompilationConstant::Mac => Some(t.os == OperatingSystem::MacOs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::find_profile;

    fn eval(profile: &str, c: CompilationConstant) -> Option<bool> {
        c.evaluate(find_profile(profile).expect("profile"))
    }

    #[test]
    fn windows_32_vs_64() {
        use CompilationConstant::*;
        assert_eq!(eval("vba7.1-excel-win32", Win32), Some(true));
        assert_eq!(eval("vba7.1-excel-win32", Win64), Some(false));
        assert_eq!(eval("vba7.1-excel-win64", Win32), Some(true));
        assert_eq!(eval("vba7.1-excel-win64", Win64), Some(true));
        assert_eq!(eval("vba7.1-excel-win64", Mac), Some(false));
        assert_eq!(eval("vba7.1-excel-win64", Win16), Some(false));
    }

    #[test]
    fn mac() {
        use CompilationConstant::*;
        assert_eq!(eval("vba7.1-excel-mac64", Mac), Some(true));
        assert_eq!(eval("vba7.1-excel-mac64", Win64), Some(false));
        assert_eq!(eval("vba7.1-excel-mac64", Win32), Some(false));
    }

    #[test]
    fn generation_constants() {
        use CompilationConstant::*;
        assert_eq!(eval("vba5-excel-win32", Vba6), None);
        assert_eq!(eval("vba5-excel-win32", Vba7), None);
        assert_eq!(eval("vba6-excel-win32", Vba6), Some(true));
        assert_eq!(eval("vba6-excel-win32", Vba7), None);
        assert_eq!(eval("vba7.0-excel-win32", Vba7), Some(true));
        assert_eq!(eval("vba7.1-excel-win32", Vba6), Some(true));
    }

    #[test]
    fn name_lookup_ignores_case() {
        assert_eq!(
            CompilationConstant::from_name(b"wIN64"),
            Some(CompilationConstant::Win64)
        );
        assert_eq!(CompilationConstant::from_name(b"Win65"), None);
        assert_eq!(CompilationConstant::from_name(b""), None);
    }
}

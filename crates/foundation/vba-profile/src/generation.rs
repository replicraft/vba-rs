/// How well a profile or generation is understood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Maturity {
    /// Specified and intended to be fully supported.
    Stable,
    /// Research only: behaviour is a best guess and not verified.
    Experimental,
}

/// A language feature whose availability depends on the VBA generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Feature {
    /// The `LongPtr` type, `DefLngPtr` and `CLngPtr` (MS-VBAL §3.3.5.2).
    LongPtr,
    /// The `PtrSafe` attribute of `Declare` statements.
    PtrSafe,
    /// The `LongLong` type, `DefLngLng`, `CLngLng` and the `^` suffix. The
    /// words are reserved from VBA 7; using the type additionally needs a
    /// 64-bit target (MS-VBAL §3.3.2), see [`crate::Profile::supports`].
    LongLong,
    /// The `Implements` statement.
    Implements,
    /// `Event`, `RaiseEvent` and `WithEvents`.
    Events,
    /// The `AddressOf` operator.
    AddressOf,
}

impl Feature {
    /// All features.
    pub const ALL: [Feature; 6] = [
        Feature::LongPtr,
        Feature::PtrSafe,
        Feature::LongLong,
        Feature::Implements,
        Feature::Events,
        Feature::AddressOf,
    ];
}

/// A generation of the VBA language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LanguageGeneration {
    /// Access 95 Access Basic/VBA research profile. Experimental: the
    /// feature set is a conservative guess (nothing added after VBA 3-era
    /// Access Basic is assumed).
    Access95,
    /// VBA 5.0 (Office 97).
    Vba5,
    /// VBA 6.x (Office 2000 to 2007).
    Vba6,
    /// VBA 7.0 (Office 2010).
    Vba7_0,
    /// VBA 7.1 (Office 2013 and later).
    Vba7_1,
}

impl LanguageGeneration {
    /// All generations, oldest first.
    pub const ALL: [LanguageGeneration; 5] = [
        LanguageGeneration::Access95,
        LanguageGeneration::Vba5,
        LanguageGeneration::Vba6,
        LanguageGeneration::Vba7_0,
        LanguageGeneration::Vba7_1,
    ];

    /// Short identifier, for example `vba7.1`.
    pub const fn id(self) -> &'static str {
        match self {
            LanguageGeneration::Access95 => "access95",
            LanguageGeneration::Vba5 => "vba5",
            LanguageGeneration::Vba6 => "vba6",
            LanguageGeneration::Vba7_0 => "vba7.0",
            LanguageGeneration::Vba7_1 => "vba7.1",
        }
    }

    /// Human readable name.
    pub const fn name(self) -> &'static str {
        match self {
            LanguageGeneration::Access95 => "Access 95 (research)",
            LanguageGeneration::Vba5 => "VBA 5.0",
            LanguageGeneration::Vba6 => "VBA 6.x",
            LanguageGeneration::Vba7_0 => "VBA 7.0",
            LanguageGeneration::Vba7_1 => "VBA 7.1",
        }
    }

    /// Major language version (Access 95 reports 0).
    pub const fn major(self) -> u8 {
        match self {
            LanguageGeneration::Access95 => 0,
            LanguageGeneration::Vba5 => 5,
            LanguageGeneration::Vba6 => 6,
            LanguageGeneration::Vba7_0 | LanguageGeneration::Vba7_1 => 7,
        }
    }

    /// Whether this generation is stable or experimental.
    pub const fn maturity(self) -> Maturity {
        match self {
            LanguageGeneration::Access95 => Maturity::Experimental,
            _ => Maturity::Stable,
        }
    }

    /// Whether the language of this generation includes `feature`.
    ///
    /// `LongPtr`, `PtrSafe` and `LongLong` exist from VBA 7.0 on;
    /// `Implements`, events and `AddressOf` from VBA 5.0 on.
    pub const fn supports(self, feature: Feature) -> bool {
        match feature {
            Feature::LongPtr | Feature::PtrSafe | Feature::LongLong => self.major() >= 7,
            Feature::Implements | Feature::Events | Feature::AddressOf => self.major() >= 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gating() {
        use LanguageGeneration::*;
        for f in [Feature::LongPtr, Feature::PtrSafe, Feature::LongLong] {
            assert!(!Vba6.supports(f));
            assert!(!Vba5.supports(f));
            assert!(Vba7_0.supports(f));
            assert!(Vba7_1.supports(f));
        }
        assert!(Vba5.supports(Feature::Implements));
        assert!(!Access95.supports(Feature::Events));
        assert_eq!(Access95.maturity(), Maturity::Experimental);
        assert_eq!(Vba6.maturity(), Maturity::Stable);
    }

    #[test]
    fn ordered_oldest_first() {
        let mut v = LanguageGeneration::ALL;
        v.sort();
        assert_eq!(v, LanguageGeneration::ALL);
    }
}

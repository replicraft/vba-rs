/// Operating system as visible to VBA code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatingSystem {
    /// Microsoft Windows.
    Windows,
    /// Apple macOS (Office for Mac).
    MacOs,
}

/// Pointer width of the VBA host process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Bitness {
    /// 16-bit (Win16).
    B16,
    /// 32-bit.
    B32,
    /// 64-bit.
    B64,
}

impl Bitness {
    /// Width in bits.
    pub const fn bits(self) -> u8 {
        match self {
            Bitness::B16 => 16,
            Bitness::B32 => 32,
            Bitness::B64 => 64,
        }
    }
}

/// The operating system and bitness a profile targets.
///
/// This is independent of the machine running vba-rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target {
    /// Operating system seen by VBA.
    pub os: OperatingSystem,
    /// Pointer width seen by VBA.
    pub bitness: Bitness,
}

impl Target {
    /// 32-bit Windows.
    pub const WIN32: Target = Target {
        os: OperatingSystem::Windows,
        bitness: Bitness::B32,
    };
    /// 64-bit Windows.
    pub const WIN64: Target = Target {
        os: OperatingSystem::Windows,
        bitness: Bitness::B64,
    };
    /// 32-bit macOS Office (Office 2011).
    pub const MAC32: Target = Target {
        os: OperatingSystem::MacOs,
        bitness: Bitness::B32,
    };
    /// 64-bit macOS Office (Office 2016 and later).
    pub const MAC64: Target = Target {
        os: OperatingSystem::MacOs,
        bitness: Bitness::B64,
    };

    /// Size in bytes of a `LongPtr` on this target.
    pub const fn pointer_size(self) -> u8 {
        self.bitness.bits() / 8
    }
}

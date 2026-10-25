use crate::{
    Bitness, CompilationConstant, Feature, HostInfo, LanguageGeneration, Maturity, Target,
};

/// A complete VBA-visible target description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Profile {
    /// Stable identifier, for example `vba7.1-excel-win64`.
    pub id: &'static str,
    /// Language generation.
    pub generation: LanguageGeneration,
    /// Host application and version.
    pub host: HostInfo,
    /// Operating system and bitness seen by VBA.
    pub target: Target,
}

impl Profile {
    /// Creates a profile.
    pub const fn new(
        id: &'static str,
        generation: LanguageGeneration,
        host: HostInfo,
        target: Target,
    ) -> Self {
        Profile {
            id,
            generation,
            host,
            target,
        }
    }

    /// Whether the profile is stable or experimental.
    pub const fn maturity(&self) -> Maturity {
        self.generation.maturity()
    }

    /// Whether `feature` is usable in this profile.
    ///
    /// This is the generation's answer, except that `LongLong` also needs a
    /// 64-bit target: the specification makes `LongLong` literals invalid in
    /// implementations without 64-bit arithmetic (MS-VBAL §3.3.2).
    pub fn supports(&self, feature: Feature) -> bool {
        match feature {
            Feature::LongLong => {
                self.generation.supports(feature) && self.target.bitness == Bitness::B64
            }
            _ => self.generation.supports(feature),
        }
    }

    /// Evaluates a predefined compilation constant by name (ASCII case
    /// insensitive). `None` means the name is not a predefined constant of
    /// this profile.
    pub fn predefined_constant(&self, name: &[u8]) -> Option<bool> {
        CompilationConstant::from_name(name)?.evaluate(self)
    }
}

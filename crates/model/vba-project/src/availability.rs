//! Classification of whether a module's source text is available.
//!
//! In MS-OVBA a module stream is the *performance cache* (`MODULEOFFSET`
//! bytes of implementation-specific compiled state) followed by the
//! *compressed source container*. A compiled-only project (`.mde`, `.accde`,
//! or any project whose source was stripped) keeps the cache but has no
//! compressed container after it.
//!
//! The classification is derived from exactly two facts about a module read
//! from a file:
//!
//! 1. whether a compressed container follows the cache (`has_source`), and
//! 2. whether a performance cache is present (its length is `MODULEOFFSET`).
//!
//! A genuinely empty module still stores a container (a signature byte plus
//! an empty chunk list), so "no container" is distinguishable from "a
//! container that decompresses to nothing". See `docs/compiled-projects.md`
//! for the limits of this heuristic.

use crate::{Module, VbaProject};

/// Key under which format layers store a module's performance cache in
/// [`Module::preserved`].
pub const PERFORMANCE_CACHE_KEY: &str = "ovba:module/performance-cache";

/// Whether a module's source text exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceAvailability {
    /// A source container is present and holds text.
    Present,
    /// A source container is present and decompresses to zero bytes: a
    /// module with source whose text is empty. It can be run (it defines
    /// nothing). If a performance cache is also present, see
    /// [`Module::source_is_ambiguous`].
    Empty,
    /// No source container, but a performance cache is present: the module
    /// is compiled-only. Its code exists only as implementation-specific
    /// compiled state, which this project never executes or decodes.
    CompiledOnly,
    /// No source container and no performance cache. Nothing proves the
    /// module was ever compiled (it may be a stripped module whose cache was
    /// also removed, or a model module created without text). Treated like
    /// [`SourceAvailability::CompiledOnly`] wherever source is required.
    AbsentNoCache,
}

impl SourceAvailability {
    /// Whether source text is lacking (the module cannot be parsed or run).
    pub fn lacks_source(self) -> bool {
        matches!(
            self,
            SourceAvailability::CompiledOnly | SourceAvailability::AbsentNoCache
        )
    }

    /// Stable lower-case label.
    pub fn name(self) -> &'static str {
        match self {
            SourceAvailability::Present => "present",
            SourceAvailability::Empty => "empty source",
            SourceAvailability::CompiledOnly => "compiled-only",
            SourceAvailability::AbsentNoCache => "absent, no cache",
        }
    }
}

/// Whether a project's modules have source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectSourceAvailability {
    /// Every module has source ([`SourceAvailability::Present`] or
    /// [`SourceAvailability::Empty`]). A project with no modules is
    /// vacuously `FullSource`.
    FullSource,
    /// Some modules have source and some lack it.
    Mixed,
    /// There is at least one module and none has source.
    CompiledOnly,
}

impl ProjectSourceAvailability {
    /// Stable lower-case label.
    pub fn name(self) -> &'static str {
        match self {
            ProjectSourceAvailability::FullSource => "full source",
            ProjectSourceAvailability::Mixed => "mixed",
            ProjectSourceAvailability::CompiledOnly => "compiled-only",
        }
    }
}

impl Module {
    /// Length in bytes of the performance cache held in
    /// [`Module::preserved`] (zero when none).
    pub fn performance_cache_len(&self) -> usize {
        self.preserved
            .get(PERFORMANCE_CACHE_KEY)
            .map_or(0, <[u8]>::len)
    }

    /// Classifies the module's source.
    pub fn source_availability(&self) -> SourceAvailability {
        if self.has_source {
            if self.source.is_empty() {
                SourceAvailability::Empty
            } else {
                SourceAvailability::Present
            }
        } else if self.performance_cache_len() > 0 {
            SourceAvailability::CompiledOnly
        } else {
            SourceAvailability::AbsentNoCache
        }
    }

    /// Whether the classification rests on a guess. True for
    /// [`SourceAvailability::AbsentNoCache`] (stripped or never compiled?)
    /// and for [`SourceAvailability::Empty`] with a performance cache (an
    /// empty compiled module, or a stripped module with a placeholder
    /// container?).
    pub fn source_is_ambiguous(&self) -> bool {
        match self.source_availability() {
            SourceAvailability::AbsentNoCache => true,
            SourceAvailability::Empty => self.performance_cache_len() > 0,
            _ => false,
        }
    }
}

impl VbaProject {
    /// Classifies the whole project.
    pub fn source_availability(&self) -> ProjectSourceAvailability {
        let lacking = self.modules_without_source().count();
        if lacking == 0 {
            ProjectSourceAvailability::FullSource
        } else if lacking == self.modules.len() {
            ProjectSourceAvailability::CompiledOnly
        } else {
            ProjectSourceAvailability::Mixed
        }
    }

    /// The modules whose source text is lacking, in project order.
    pub fn modules_without_source(&self) -> impl Iterator<Item = &Module> {
        self.modules
            .iter()
            .filter(|m| m.source_availability().lacks_source())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ModuleKind;

    fn module(has_source: bool, source: &str, cache: usize) -> Module {
        let mut m = Module::new("M", ModuleKind::Standard, source);
        m.has_source = has_source;
        if cache > 0 {
            m.preserved.insert(PERFORMANCE_CACHE_KEY, vec![7; cache]);
        }
        m
    }

    #[test]
    fn module_classes() {
        use SourceAvailability::*;
        let cases = [
            (module(true, "x", 0), Present, false),
            (module(true, "x", 4), Present, false),
            (module(true, "", 0), Empty, false),
            (module(true, "", 4), Empty, true),
            (module(false, "", 4), CompiledOnly, false),
            (module(false, "", 0), AbsentNoCache, true),
        ];
        for (m, class, ambiguous) in cases {
            assert_eq!(m.source_availability(), class);
            assert_eq!(m.source_is_ambiguous(), ambiguous);
            assert_eq!(class.lacks_source(), !m.has_source);
        }
    }

    #[test]
    fn project_classes() {
        let mut p = VbaProject::new("P");
        assert_eq!(
            p.source_availability(),
            ProjectSourceAvailability::FullSource
        );
        p.modules.push(module(true, "x", 0));
        assert_eq!(
            p.source_availability(),
            ProjectSourceAvailability::FullSource
        );
        let mut stripped = module(false, "", 3);
        stripped.name = "S".into();
        p.modules.push(stripped);
        assert_eq!(p.source_availability(), ProjectSourceAvailability::Mixed);
        assert_eq!(p.modules_without_source().count(), 1);
        p.modules.remove(0);
        assert_eq!(
            p.source_availability(),
            ProjectSourceAvailability::CompiledOnly
        );
    }
}

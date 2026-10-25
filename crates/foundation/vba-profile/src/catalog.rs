use crate::{HostApplication::*, HostInfo, LanguageGeneration as G, Profile, Target};

const fn p(id: &'static str, g: G, host: HostInfo, t: Target) -> Profile {
    Profile::new(id, g, host, t)
}

static PROFILES: [Profile; 11] = [
    p(
        "access95-research",
        G::Access95,
        HostInfo::new(Access, 7, 0, 1995),
        Target::WIN32,
    ),
    p(
        "vba5-excel-win32",
        G::Vba5,
        HostInfo::new(Excel, 8, 0, 1997),
        Target::WIN32,
    ),
    p(
        "vba6-excel-win32",
        G::Vba6,
        HostInfo::new(Excel, 11, 0, 2003),
        Target::WIN32,
    ),
    p(
        "vba6-excel-mac32",
        G::Vba6,
        HostInfo::new(Excel, 14, 0, 2011),
        Target::MAC32,
    ),
    p(
        "vba7.0-excel-win32",
        G::Vba7_0,
        HostInfo::new(Excel, 14, 0, 2010),
        Target::WIN32,
    ),
    p(
        "vba7.0-excel-win64",
        G::Vba7_0,
        HostInfo::new(Excel, 14, 0, 2010),
        Target::WIN64,
    ),
    p(
        "vba7.1-excel-win32",
        G::Vba7_1,
        HostInfo::new(Excel, 16, 0, 2016),
        Target::WIN32,
    ),
    p(
        "vba7.1-excel-win64",
        G::Vba7_1,
        HostInfo::new(Excel, 16, 0, 2016),
        Target::WIN64,
    ),
    p(
        "vba7.1-word-win64",
        G::Vba7_1,
        HostInfo::new(Word, 16, 0, 2016),
        Target::WIN64,
    ),
    p(
        "vba7.1-access-win64",
        G::Vba7_1,
        HostInfo::new(Access, 16, 0, 2016),
        Target::WIN64,
    ),
    p(
        "vba7.1-excel-mac64",
        G::Vba7_1,
        HostInfo::new(Excel, 16, 0, 2016),
        Target::MAC64,
    ),
];

/// All built-in profiles, oldest generation first.
pub fn profiles() -> &'static [Profile] {
    &PROFILES
}

/// Finds a built-in profile by its identifier.
pub fn find_profile(id: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.id == id)
}

/// The default profile: VBA 7.1 in 64-bit Excel on Windows.
pub fn default_profile() -> &'static Profile {
    find_profile("vba7.1-excel-win64").expect("default profile exists")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Feature, Maturity};

    #[test]
    fn ids_are_unique_and_listing_is_stable() {
        let ids: Vec<_> = profiles().iter().map(|p| p.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
        assert_eq!(ids[0], "access95-research");
        assert!(
            profiles()
                .windows(2)
                .all(|w| w[0].generation <= w[1].generation)
        );
    }

    #[test]
    fn lookup() {
        assert!(find_profile("nope").is_none());
        assert_eq!(default_profile().id, "vba7.1-excel-win64");
        let experimental: Vec<_> = profiles()
            .iter()
            .filter(|p| p.maturity() == Maturity::Experimental)
            .collect();
        assert_eq!(experimental.len(), 1);
    }

    #[test]
    fn longlong_needs_64_bit() {
        assert!(
            !find_profile("vba7.1-excel-win32")
                .unwrap()
                .supports(Feature::LongLong)
        );
        assert!(
            find_profile("vba7.1-excel-win32")
                .unwrap()
                .supports(Feature::LongPtr)
        );
        assert!(
            find_profile("vba7.1-excel-win64")
                .unwrap()
                .supports(Feature::LongLong)
        );
        assert!(
            !find_profile("vba6-excel-win32")
                .unwrap()
                .supports(Feature::PtrSafe)
        );
    }

    #[test]
    fn pointer_sizes() {
        assert_eq!(
            find_profile("vba7.1-excel-win32")
                .unwrap()
                .target
                .pointer_size(),
            4
        );
        assert_eq!(
            find_profile("vba7.1-excel-win64")
                .unwrap()
                .target
                .pointer_size(),
            8
        );
    }

    /// The answer never depends on the machine running the tests.
    #[test]
    fn host_independent() {
        let p = find_profile("vba7.1-excel-win32").unwrap();
        assert_eq!(p.predefined_constant(b"Win64"), Some(false));
        assert_eq!(p.predefined_constant(b"win32"), Some(true));
        assert_eq!(p.predefined_constant(b"Foo"), None);
    }
}

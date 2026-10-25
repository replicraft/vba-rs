//! Registry, catalog consistency, provider affinity and end-to-end `Declare`
//! tests.

use proptest::prelude::*;
use std::collections::BTreeSet;
use vba_ffi::{
    ArgValue, CallConv, Charset, Codepage, DeclareSpec, EntryCandidate, HandleTable, NativeReturn,
    ParamKind, ParamSpec, Passing, PreparedCall, ProviderId,
};
use vba_profile::{OperatingSystem, Target};
use vba_value::VbaString;
use vba_winapi::catalog::{Status, catalog, lookup, parse};
use vba_winapi::{
    CallContext, Identity, ManualClock, NATIVE_SHIMS, Provider, Registry, WinApiError,
};

fn ctx(target: Target) -> CallContext {
    CallContext::new(
        target,
        Box::new(ManualClock::at(0)),
        Identity::new("u", "C").unwrap(),
    )
}

fn sym(s: &str) -> Vec<EntryCandidate> {
    vec![EntryCandidate::Symbol(s.into())]
}

#[test]
fn catalog_matches_registry_exactly() {
    let registry: BTreeSet<(String, String)> = Registry::with_native_shims()
        .exports()
        .into_iter()
        .collect();
    let implemented: BTreeSet<(String, String)> = catalog()
        .iter()
        .filter(|r| r.status == Status::Implemented)
        .flat_map(|r| r.exports().into_iter().map(|e| (r.dll.clone(), e)))
        .collect();
    let not_implemented: BTreeSet<(String, String)> = catalog()
        .iter()
        .filter(|r| r.status == Status::NotImplemented)
        .flat_map(|r| r.exports().into_iter().map(|e| (r.dll.clone(), e)))
        .collect();
    assert_eq!(
        registry, implemented,
        "registered functions and IMPLEMENTED rows differ"
    );
    assert!(
        registry.is_disjoint(&not_implemented),
        "a registered function is cataloged NOT IMPLEMENTED"
    );
}

#[test]
fn catalog_is_well_formed() {
    let mut seen = BTreeSet::new();
    for r in catalog() {
        for e in r.exports() {
            assert!(
                seen.insert((r.dll.clone(), e.clone())),
                "duplicate row for {}!{e}",
                r.dll
            );
        }
        assert_eq!(r.dll, r.dll.to_ascii_lowercase());
        if r.status == Status::NotImplemented {
            assert_eq!(r.tests, 0, "{}", r.symbol);
        } else {
            assert!(
                r.tests > 0,
                "{} is implemented without reference tests",
                r.symbol
            );
            assert_eq!(r.strategy, vba_winapi::catalog::Strategy::NativeShim);
        }
    }
    assert!(parse("h\nonly,three,fields\n").is_err());
    assert!(lookup("kernel32", "Sleep").is_some());
    assert!(lookup("kernel32", "SleepEx").is_none());
}

#[test]
fn unimplemented_symbols_fail_visibly() {
    let reg = Registry::with_native_shims();
    for (lib, name, why) in [
        ("kernel32", "Beep", "cataloged"),
        ("user32", "MessageBoxA", "cataloged"),
        ("kernel32", "CopyMemory", "cataloged"),
        ("kernel32", "NoSuchFunction", "no registered provider"),
        ("mylib", "Foo", "no registered provider"),
    ] {
        match reg.resolve(OperatingSystem::Windows, lib, &sym(name)) {
            Err(WinApiError::Unsupported { symbol, reason }) => {
                assert!(symbol.ends_with(&format!("!{name}")), "{symbol}");
                assert!(reason.contains(why), "{reason}");
            }
            other => panic!("{lib}!{name}: {other:?}"),
        }
    }
}

#[test]
fn ordinals_and_non_windows_targets_are_unsupported() {
    let reg = Registry::with_native_shims();
    assert!(matches!(
        reg.resolve(
            OperatingSystem::Windows,
            "kernel32",
            &[EntryCandidate::Ordinal(5)]
        ),
        Err(WinApiError::Unsupported { .. })
    ));
    assert!(matches!(
        reg.resolve(OperatingSystem::MacOs, "kernel32", &sym("Sleep")),
        Err(WinApiError::Unsupported { .. })
    ));
}

#[test]
fn library_names_are_normalized_and_symbols_are_case_sensitive() {
    let reg = Registry::with_native_shims();
    for lib in [
        "kernel32",
        "KERNEL32.DLL",
        "C:\\Windows\\System32\\Kernel32.dll",
    ] {
        assert!(
            reg.resolve(OperatingSystem::Windows, lib, &sym("GetTickCount"))
                .is_ok(),
            "{lib}"
        );
    }
    assert!(
        reg.resolve(OperatingSystem::Windows, "kernel32", &sym("gettickcount"))
            .is_err()
    );
}

#[test]
fn duplicate_exports_are_rejected() {
    let mut reg = Registry::with_native_shims();
    assert!(matches!(
        reg.register(Box::new(vba_winapi::NativeShims)),
        Err(WinApiError::DuplicateExport(_))
    ));
}

struct Other;
impl Provider for Other {
    fn id(&self) -> ProviderId {
        ProviderId("other")
    }
    fn exports(&self) -> Vec<(&'static str, &'static str)> {
        vec![("other", "Mint")]
    }
    fn call(
        &self,
        _: &mut CallContext,
        _: &str,
        _: &str,
        _: &[vba_ffi::NativeArg],
    ) -> Result<NativeReturn, WinApiError> {
        Ok(NativeReturn::Void)
    }
}

#[test]
fn handles_minted_by_one_provider_are_rejected_by_another() {
    let mut reg = Registry::with_native_shims();
    reg.register(Box::new(Other)).unwrap();
    let r = reg
        .resolve(OperatingSystem::Windows, "other", &sym("Mint"))
        .unwrap();
    assert_eq!(reg.provider_of(&r), ProviderId("other"));
    let s = reg
        .resolve(OperatingSystem::Windows, "kernel32", &sym("Sleep"))
        .unwrap();
    assert_eq!(reg.provider_of(&s), NATIVE_SHIMS);

    let mut table = HandleTable::new();
    let h = table.mint(reg.provider_of(&r), "thing");
    assert!(table.resolve(reg.provider_of(&r), &h).is_ok());
    assert!(table.resolve(reg.provider_of(&s), &h).is_err());
    assert!(table.resolve_value(NATIVE_SHIMS, h.value()).is_err());
}

fn p(name: &str, kind: ParamKind, passing: Passing) -> ParamSpec {
    ParamSpec {
        name: name.into(),
        kind,
        passing,
    }
}

fn decl(
    target: Target,
    name: &str,
    lib: &str,
    params: Vec<ParamSpec>,
    ret: Option<ParamKind>,
) -> vba_ffi::ValidatedDeclare {
    DeclareSpec {
        target,
        ptr_safe: true,
        name: name.into(),
        library: lib.into(),
        alias: None,
        charset: Charset::Ansi,
        conv: CallConv::Stdcall,
        params,
        ret,
    }
    .validate()
    .unwrap()
}

#[test]
fn declare_to_shim_end_to_end() {
    // Declare Function lstrcpy Lib "kernel32" Alias "lstrcpyA" (ByVal d As String, ByVal s As String) As Long
    for target in [Target::WIN32, Target::WIN64] {
        let mut c = ctx(target);
        let reg = Registry::with_native_shims();
        let mut d = decl(
            target,
            "lstrcpy",
            "kernel32",
            vec![
                p("d", ParamKind::String, Passing::ByVal),
                p("s", ParamKind::String, Passing::ByVal),
            ],
            Some(ParamKind::LongPtr),
        );
        // No explicit alias: the ANSI probe `lstrcpy` -> `lstrcpyA` finds the shim.
        let resolved = reg.resolve_declare(&d).unwrap();
        assert_eq!(resolved.symbol(), "lstrcpyA");
        let text = |s: &str| ArgValue::Str(VbaString::from_units(s.encode_utf16().collect()));
        let buf = ArgValue::Str(VbaString::from_units(vec![u16::from(b' '); 8]));
        let call = PreparedCall::prepare(
            &d,
            &mut c.memory,
            &[buf, text("hello")],
            Codepage::Windows1252,
        )
        .unwrap();
        let ret = reg.call(&mut c, &resolved, call.args()).unwrap();
        let out = call.finish(&d, &mut c.memory, ret).unwrap();
        // The whole buffer is converted back: "hello" NUL then the untouched spaces.
        let ArgValue::Str(s) = &out.args[0] else {
            panic!()
        };
        assert_eq!(s.to_string_lossy(), "hello\0  ");
        assert!(matches!(out.ret, Some(ArgValue::Int(a)) if a > 0xFFFF));
        d = decl(
            target,
            "lstrlen",
            "KERNEL32.DLL",
            vec![p("s", ParamKind::String, Passing::ByVal)],
            Some(ParamKind::Long),
        );
        let resolved = reg.resolve_declare(&d).unwrap();
        let call =
            PreparedCall::prepare(&d, &mut c.memory, &[text("héllo")], Codepage::Windows1252)
                .unwrap();
        let ret = reg.call(&mut c, &resolved, call.args()).unwrap();
        assert_eq!(
            call.finish(&d, &mut c.memory, ret).unwrap().ret,
            Some(ArgValue::Int(5))
        );
        assert_eq!(c.memory.used(), 0, "temporaries released");
    }
}

#[test]
fn explicit_alias_and_mac_and_ordinal_declares() {
    let reg = Registry::with_native_shims();
    let mut spec = DeclareSpec {
        target: Target::WIN64,
        ptr_safe: true,
        name: "Ticks".into(),
        library: "kernel32".into(),
        alias: Some("GetTickCount64".into()),
        charset: Charset::Ansi,
        conv: CallConv::Stdcall,
        params: vec![],
        ret: Some(ParamKind::LongLong),
    };
    let r = reg
        .resolve_declare(&spec.clone().validate().unwrap())
        .unwrap();
    assert_eq!(r.symbol(), "GetTickCount64");
    spec.alias = Some("#12".into());
    assert!(matches!(
        reg.resolve_declare(&spec.clone().validate().unwrap()),
        Err(WinApiError::Unsupported { .. })
    ));
    spec.alias = None;
    spec.target = Target::MAC64;
    assert!(matches!(
        reg.resolve_declare(&spec.validate().unwrap()),
        Err(WinApiError::Unsupported { .. })
    ));
}

#[test]
fn get_tick_count_return_narrows_to_long() {
    let mut c = ctx(Target::WIN32);
    let reg = Registry::with_native_shims();
    let d = decl(
        Target::WIN32,
        "GetTickCount",
        "kernel32",
        vec![],
        Some(ParamKind::Long),
    );
    let r = reg.resolve_declare(&d).unwrap();
    let clock = ManualClock::at(0xFFFF_FFFF);
    c.env = Box::new(clock);
    let call = PreparedCall::prepare(&d, &mut c.memory, &[], Codepage::Windows1252).unwrap();
    let ret = reg.call(&mut c, &r, call.args()).unwrap();
    assert_eq!(
        call.finish(&d, &mut c.memory, ret).unwrap().ret,
        Some(ArgValue::Int(-1))
    );
}

proptest! {
    #[test]
    fn mb_wc_utf8_round_trip(s in "\\PC{0,24}") {
        let reg = Registry::with_native_shims();
        let mut c = ctx(Target::WIN64);
        let call = |c: &mut CallContext, name: &str, args: &[i64]| {
            let r = reg.resolve(OperatingSystem::Windows, "kernel32", &sym(name)).unwrap();
            let a: Vec<_> = args.iter().map(|v| vba_ffi::NativeArg::Int(*v)).collect();
            match reg.call(c, &r, &a).unwrap() { NativeReturn::Int(v) => v, o => panic!("{o:?}") }
        };
        let utf8 = c.memory.alloc_bytes(s.as_bytes()).unwrap() as i64;
        let n = s.len() as i64;
        if n == 0 { return Ok(()); }
        let wlen = call(&mut c, "MultiByteToWideChar", &[65001, 8, utf8, n, 0, 0]);
        let wbuf = c.memory.alloc((wlen as usize * 2).max(2)).unwrap() as i64;
        prop_assert_eq!(call(&mut c, "MultiByteToWideChar", &[65001, 8, utf8, n, wbuf, wlen]), wlen);
        let blen = call(&mut c, "WideCharToMultiByte", &[65001, 0x80, wbuf, wlen, 0, 0, 0, 0]);
        prop_assert_eq!(blen, n);
        let out = c.memory.alloc(blen as usize).unwrap() as i64;
        prop_assert_eq!(call(&mut c, "WideCharToMultiByte", &[65001, 0x80, wbuf, wlen, out, blen, 0, 0]), blen);
        prop_assert_eq!(c.memory.read(out as u64, blen as usize).unwrap(), s.as_bytes());
    }

    #[test]
    fn lstrcpy_lstrlen_ansi_round_trip(bytes in proptest::collection::vec(1u8..=255, 0..40)) {
        let reg = Registry::with_native_shims();
        let mut c = ctx(Target::WIN32);
        let mut with_nul = bytes.clone();
        with_nul.push(0);
        let src = c.memory.alloc_bytes(&with_nul).unwrap() as i64;
        let dst = c.memory.alloc(with_nul.len()).unwrap() as i64;
        let run = |c: &mut CallContext, n: &str, a: &[i64]| {
            let r = reg.resolve(OperatingSystem::Windows, "kernel32", &sym(n)).unwrap();
            let a: Vec<_> = a.iter().map(|v| vba_ffi::NativeArg::Int(*v)).collect();
            reg.call(c, &r, &a).unwrap()
        };
        run(&mut c, "lstrcpyA", &[dst, src]);
        prop_assert_eq!(run(&mut c, "lstrlenA", &[dst]), NativeReturn::Int(bytes.len() as i64));
        prop_assert_eq!(c.memory.read_cstr(dst as u64).unwrap(), bytes);
    }

    #[test]
    fn char_case_is_idempotent_on_ascii(s in "[ -~]{0,30}") {
        let reg = Registry::with_native_shims();
        let mut c = ctx(Target::WIN32);
        let mut v = s.clone().into_bytes();
        v.push(0);
        let p = c.memory.alloc_bytes(&v).unwrap() as i64;
        let r = reg.resolve(OperatingSystem::Windows, "user32", &sym("CharUpperA")).unwrap();
        reg.call(&mut c, &r, &[vba_ffi::NativeArg::Int(p)]).unwrap();
        prop_assert_eq!(c.memory.read_cstr(p as u64).unwrap(), s.to_ascii_uppercase().into_bytes());
    }
}

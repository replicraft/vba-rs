//! The catalog against the generated Win32 function inventory.

use vba_ffi::EntryCandidate;
use vba_profile::OperatingSystem;
use vba_winapi::catalog::catalog;
use vba_winapi::inventory::{Direction, inventory, lookup, lookup_all, lookup_any_dll};
use vba_winapi::{Registry, WinApiError};

/// Catalog entries Microsoft's metadata deliberately does not list: the
/// `Rtl*Memory` functions are real kernel32/ntdll exports that the SDK headers
/// expose as macros, and `CopyMemory`/`FillMemory`/`ZeroMemory` are macros only.
const NOT_IN_METADATA: &[(&str, &str)] = &[
    ("kernel32", "RtlMoveMemory"),
    ("kernel32", "RtlFillMemory"),
    ("kernel32", "RtlZeroMemory"),
    ("kernel32", "CopyMemory"),
    ("kernel32", "FillMemory"),
    ("kernel32", "ZeroMemory"),
];

fn declared_param_count(signature: &str) -> Option<usize> {
    if signature.starts_with("many") {
        return None;
    }
    let (params, _) = signature.split_once("=>")?;
    let params = params.trim();
    Some(if params.is_empty() {
        0
    } else {
        params.split(';').count()
    })
}

#[test]
fn inventory_is_sorted_and_well_formed() {
    let rows = inventory();
    assert!(rows.len() > 10_000, "{}", rows.len());
    for w in rows.windows(2) {
        let k = |e: &vba_winapi::inventory::Entry| (e.dll, e.symbol, e.arch);
        assert!(k(&w[0]) < k(&w[1]), "{:?} !< {:?}", k(&w[0]), k(&w[1]));
    }
    for e in rows {
        assert_eq!(e.dll, e.dll.to_ascii_lowercase());
        assert!(!e.dll.ends_with(".dll"), "{}", e.dll);
        if let Some(c) = e.charset {
            assert!(e.paired);
            assert_eq!(e.symbol, format!("{}{c}", e.base));
        }
    }
}

#[test]
fn lookup_reports_the_metadata_signature() {
    let sleep = lookup("kernel32", "Sleep").expect("Sleep");
    assert_eq!(sleep.param_count(), 1);
    assert_eq!(sleep.ret, "void");
    assert_eq!(sleep.conv, "system");
    let p = sleep.params().next().unwrap();
    assert_eq!(
        (p.name, p.direction, p.ty),
        ("dwMilliseconds", Direction::In, "u32")
    );
    assert_eq!(sleep.signature(), "Sleep(dwMilliseconds: in u32) -> void");
    let w = lookup("kernel32", "lstrcpyW").expect("lstrcpyW");
    assert_eq!((w.charset, w.paired, w.base), (Some('W'), true, "lstrcpy"));
    assert!(lookup("kernel32", "NoSuchFunction").is_none());
    assert!(lookup_all("kernel32", "NoSuchFunction").is_empty());
    assert!(!lookup_any_dll("MessageBoxA").is_empty());
}

#[test]
fn every_cataloged_function_exists_and_agrees_with_the_metadata() {
    for row in catalog() {
        let declared = declared_param_count(&row.signature);
        for export in row.exports() {
            if NOT_IN_METADATA.contains(&(row.dll.as_str(), export.as_str())) {
                continue;
            }
            let e = lookup(&row.dll, &export)
                .unwrap_or_else(|| panic!("{}!{export} is not in the inventory", row.dll));
            if let Some(n) = declared {
                assert_eq!(
                    e.param_count(),
                    n,
                    "{}!{export}: catalog `{}` vs metadata {e}",
                    row.dll,
                    row.signature
                );
            }
            match row.variants.as_str() {
                "-" => assert_eq!(e.charset, None, "{}!{export}", row.dll),
                v => {
                    let suffix = export.chars().last();
                    assert!(v.chars().any(|c| Some(c) == suffix));
                    assert_eq!(e.charset, suffix, "{}!{export}", row.dll);
                    assert!(e.paired, "{}!{export}", row.dll);
                    assert_eq!(e.base, row.symbol, "{}!{export}", row.dll);
                }
            }
        }
    }
}

#[test]
fn implemented_shims_are_all_known_win32_exports() {
    for (dll, symbol) in Registry::with_native_shims().exports() {
        if NOT_IN_METADATA.contains(&(dll.as_str(), symbol.as_str())) {
            continue;
        }
        assert!(lookup(&dll, &symbol).is_some(), "{dll}!{symbol}");
    }
}

#[test]
fn unimplemented_but_documented_functions_say_so() {
    let reg = Registry::with_native_shims();
    for (lib, name) in [
        ("kernel32", "Beep"),
        ("kernel32", "GetModuleFileNameA"),
        ("USER32.DLL", "MessageBoxW"),
    ] {
        let sym = [EntryCandidate::Symbol(name.into())];
        match reg.resolve(OperatingSystem::Windows, lib, &sym) {
            Err(WinApiError::Unsupported { reason, .. }) => {
                assert!(reason.contains("known Win32 export"), "{reason}");
                assert!(reason.contains(&format!("{name}(")), "{reason}");
                assert!(reason.contains(") -> "), "{reason}");
            }
            other => panic!("{lib}!{name}: {other:?}"),
        }
    }
    // A symbol the metadata places in another DLL names that DLL.
    let sym = [EntryCandidate::Symbol("MessageBoxA".into())];
    match reg.resolve(OperatingSystem::Windows, "kernel32", &sym) {
        Err(WinApiError::Unsupported { reason, .. }) => {
            assert!(reason.contains("`user32` (not `kernel32`)"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
    // Truly unknown names stay "no registered provider" without a signature.
    let sym = [EntryCandidate::Symbol("NoSuchFunction".into())];
    match reg.resolve(OperatingSystem::Windows, "kernel32", &sym) {
        Err(WinApiError::Unsupported { reason, .. }) => {
            assert!(!reason.contains("known Win32 export"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
}

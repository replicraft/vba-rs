//! Reads real type libraries from a local Wine installation.
//!
//! These tests are `#[ignore]`d (run with `-- --ignored`) and skip cleanly
//! when the files are absent, so CI passes without them. The libraries are
//! Wine builds, not Microsoft's originals: they validate the reader and the
//! documented layout, not Microsoft's member ordering.
//!
//! Set `VBA_TYPELIB_SAMPLES` to a directory (searched recursively) to use
//! other files.

use std::path::{Path, PathBuf};

use vba_typelib::{
    FuncKind, InvokeKind, TypeDesc, TypeKind, TypeLib, TypeRef, TypelibError, VarKind, load_all,
};

fn roots() -> Vec<PathBuf> {
    if let Some(p) = std::env::var_os("VBA_TYPELIB_SAMPLES") {
        return vec![PathBuf::from(p)];
    }
    let base = "/Applications/Wine Stable.app/Contents/Resources/wine/lib/wine";
    vec![
        Path::new(base).join("x86_64-windows"),
        Path::new(base).join("i386-windows"),
    ]
}

fn candidates() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str())
                && ["tlb", "olb", "dll", "ocx", "exe"].contains(&ext.to_ascii_lowercase().as_str())
            {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    for r in roots() {
        walk(&r, &mut out);
    }
    out
}

fn check_invariants(file: &str, lib: &TypeLib) {
    let ptr: u16 = match lib.sys_kind {
        Some(3) => 8,
        _ => 4,
    };
    assert!(lib.warnings.is_empty(), "{file}: {:?}", lib.warnings);
    assert!(!lib.name.is_empty(), "{file}: library without a name");
    let ntypes = lib.types.len();
    let ok_ref = |r: &TypeRef| match r {
        TypeRef::Local(i) => *i < ntypes,
        TypeRef::Imported(i) => *i < lib.imports.len(),
    };
    fn refs(d: &TypeDesc, out: &mut Vec<TypeRef>) {
        match d {
            TypeDesc::Prim(_) => {}
            TypeDesc::Ptr(i) | TypeDesc::SafeArray(i) => refs(i, out),
            TypeDesc::CArray { elem, .. } => refs(elem, out),
            TypeDesc::User(r) => out.push(*r),
        }
    }
    for t in &lib.types {
        let mut rs = Vec::new();
        t.implements.iter().for_each(|i| rs.push(i.target));
        if let Some(a) = &t.alias {
            refs(a, &mut rs);
        }
        for f in &t.functions {
            refs(&f.ret, &mut rs);
            f.params.iter().for_each(|p| refs(&p.ty, &mut rs));
            assert!(
                !f.name.is_empty(),
                "{file}: {} has an unnamed function",
                t.name
            );
        }
        t.variables.iter().for_each(|v| refs(&v.ty, &mut rs));
        assert!(
            rs.iter().all(ok_ref),
            "{file}: {} has a dangling reference",
            t.name
        );

        // Interfaces: pure-virtual vtable slots lie inside the vtable.
        if t.kind == TypeKind::Interface {
            for f in t
                .functions
                .iter()
                .filter(|f| f.kind == FuncKind::PureVirtual)
            {
                assert_eq!(f.vtable_offset % ptr, 0, "{file}: {}.{}", t.name, f.name);
                assert!(
                    f.vtable_offset + ptr <= t.vtable_size,
                    "{file}: {}.{} slot outside the vtable",
                    t.name,
                    f.name
                );
            }
        }
        // Property get/put accessors of one name share a member id.
        for f in &t.functions {
            if matches!(
                f.invoke,
                InvokeKind::PropertyPut | InvokeKind::PropertyPutRef
            ) && let Some(g) = t
                .functions
                .iter()
                .find(|g| g.name == f.name && g.invoke == InvokeKind::PropertyGet)
            {
                assert_eq!(g.memid, f.memid, "{file}: {}.{}", t.name, f.name);
            }
        }
        // Record fields lie inside the record.
        if t.kind == TypeKind::Record {
            for v in &t.variables {
                assert_eq!(v.kind, VarKind::PerInstance, "{file}: {}", t.name);
                assert!(
                    v.offset.unwrap() <= t.instance_size,
                    "{file}: {}.{}",
                    t.name,
                    v.name
                );
            }
        }
        // Enum members are constants with values.
        if t.kind == TypeKind::Enum {
            for v in &t.variables {
                assert_eq!(v.kind, VarKind::Const, "{file}: {}", t.name);
                assert!(v.value.is_some(), "{file}: {}.{}", t.name, v.name);
            }
        }
    }
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn all_local_libraries_parse_or_report_a_typed_reason() {
    let files = candidates();
    if files.is_empty() {
        eprintln!("no sample libraries found; skipping");
        return;
    }
    let (mut parsed, mut sltg, mut none) = (0, 0, 0);
    for f in files {
        let name = f.display().to_string();
        let data = std::fs::read(&f).unwrap();
        match load_all(&data) {
            Ok(libs) => {
                for (_, lib) in &libs {
                    check_invariants(&name, lib);
                    parsed += 1;
                }
                if data.windows(4).any(|w| w == b"SLTG") {
                    sltg += 1;
                }
            }
            Err(TypelibError::NoTypelibResource) => none += 1,
            Err(e) => panic!("{name}: {e}"),
        }
    }
    eprintln!(
        "parsed {parsed} libraries ({sltg} files with SLTG data), {none} without a TYPELIB resource"
    );
    assert!(parsed > 0);
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn stdole2_has_the_standard_ole_types() {
    let Some(f) = candidates()
        .into_iter()
        .find(|p| p.file_name().is_some_and(|n| n == "stdole2.tlb"))
    else {
        eprintln!("stdole2.tlb not found; skipping");
        return;
    };
    let image = std::fs::read(f).unwrap();
    let libs = load_all(&image).unwrap();
    let lib = &libs[0].1;
    // The resource bytes read as a raw `.tlb` give the same library.
    let res = vba_typelib::pe::typelib_resources(&image).unwrap();
    assert_eq!(&vba_typelib::parse_msft(res[0].data).unwrap(), lib);
    assert_eq!(lib.name, "stdole");
    assert_eq!(lib.guid.to_string(), "00020430-0000-0000-C000-000000000046");
    assert_eq!((lib.version.major, lib.version.minor), (2, 0));
    let (_, font) = lib.type_by_name("IFont").unwrap();
    assert_eq!(
        font.guid.unwrap().to_string(),
        "BEF6E002-A874-101A-8BBA-00AA00300CAB"
    );
    let (_, pic) = lib.type_by_name("Picture").unwrap();
    assert_eq!(pic.kind, TypeKind::Dispatch);
    let ids: Vec<_> = pic
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.memid))
        .collect();
    assert_eq!(
        ids,
        vec![
            ("Handle", 0),
            ("hPal", 2),
            ("Type", 3),
            ("Width", 4),
            ("Height", 5)
        ]
    );
    let (_, fns) = lib.type_by_name("StdFunctions").unwrap();
    assert_eq!(fns.dll_name.as_deref(), Some("oleaut32.dll"));
    let load = &fns.functions[0];
    assert_eq!(load.name, "LoadPicture");
    assert_eq!(load.doc.as_deref(), Some("Loads a picture from a file"));
    assert_eq!(load.optional_count, 1);
    assert!(load.params[1].default.is_some());
}

#[cfg(feature = "sema")]
#[test]
#[ignore = "needs local Wine type libraries"]
fn scripting_runtime_drives_sema() {
    use vba_diagnostics::Severity;
    use vba_profile::find_profile;
    use vba_project::ModuleKind;
    use vba_sema::{Options, SourceModule, analyze_sources};
    use vba_typelib::sema::TypelibProvider;

    let Some(f) = candidates()
        .into_iter()
        .find(|p| p.file_name().is_some_and(|n| n == "scrrun.dll"))
    else {
        eprintln!("scrrun.dll not found; skipping");
        return;
    };
    let lib = vba_typelib::load(&std::fs::read(f).unwrap()).unwrap();
    assert_eq!(lib.name, "Scripting");
    let provider = TypelibProvider::new().with_library(lib);
    let src = "Option Explicit\n\
        Sub T()\n\
        Dim d As Scripting.Dictionary\n\
        Dim n As Long\n\
        Dim b As Boolean\n\
        Set d = CreateObject(\"Scripting.Dictionary\")\n\
        d.Add \"a\", 1\n\
        b = d.Exists(\"a\")\n\
        n = d.Count\n\
        d.RemoveAll\n\
        d.Frobnicate\n\
        End Sub\n";
    let profile = *find_profile("vba7.1-excel-win64").unwrap();
    let a = analyze_sources(
        &[SourceModule {
            name: "M".into(),
            kind: ModuleKind::Standard,
            source: src.as_bytes().to_vec(),
        }],
        &profile,
        &provider,
        &Options::default(),
    );
    let errs: Vec<String> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| {
            let line = a.sources.get(d.file).unwrap().line_col(d.span.start).line + 1;
            format!("{}@{line}", d.code.0)
        })
        .collect();
    assert_eq!(errs, vec!["SEM019@11"], "{:?}", a.diagnostics);
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn stdole32_is_an_sltg_library() {
    let files: Vec<_> = candidates()
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|n| n == "stdole32.tlb"))
        .collect();
    if files.is_empty() {
        eprintln!("stdole32.tlb not found; skipping");
        return;
    }
    for f in files {
        let image = std::fs::read(&f).unwrap();
        let res = vba_typelib::pe::typelib_resources(&image).unwrap();
        assert_eq!(&res[0].data[..4], b"SLTG");
        let lib = vba_typelib::load(&image).unwrap();
        // The raw resource bytes give the same library.
        assert_eq!(vba_typelib::parse_any(res[0].data).unwrap(), lib);
        assert_eq!(lib.name, "stdole");
        assert_eq!(lib.guid.to_string(), "00020430-0000-0000-C000-000000000046");
        assert_eq!((lib.version.major, lib.version.minor), (1, 0));
        assert_eq!(lib.lcid, 0x409);
        assert!(lib.warnings.is_empty(), "{:?}", lib.warnings);
        let names: Vec<_> = lib.types.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "GUID",
                "DISPPARAMS",
                "EXCEPINFO",
                "IUnknown",
                "IDispatch",
                "IEnumVARIANT"
            ]
        );
        let (_, d) = lib.type_by_name("IDispatch").unwrap();
        assert_eq!(
            d.guid.unwrap().to_string(),
            "00020400-0000-0000-C000-000000000046"
        );
        assert_eq!(d.implements[0].target, TypeRef::Local(3));
        let invoke = d.functions.iter().find(|f| f.name == "Invoke").unwrap();
        assert_eq!(invoke.memid, 0x6001_0003);
        assert_eq!(invoke.params.len(), 8);
        assert_eq!(invoke.vtable_offset, 24);
        let (_, g) = lib.type_by_name("GUID").unwrap();
        assert_eq!(g.kind, TypeKind::Record);
        assert_eq!(g.instance_size, 16);
        assert_eq!(lib.describe(&g.variables[3].ty), "UI1[8]");
        let (_, e) = lib.type_by_name("EXCEPINFO").unwrap();
        assert_eq!(e.variables.len(), 9);
    }
}

//! Early-bound VBA against the type libraries shipped with a local Wine
//! installation (`scrrun`, `msxml`, `msado15`, `vbscript`).
//!
//! These tests are `#[ignore]`d (run with `-- --ignored --nocapture`) and
//! skip cleanly when the files are absent. The libraries are Wine builds,
//! not Microsoft's originals; nothing from them is committed. Set
//! `VBA_TYPELIB_SAMPLES` to use another directory.
#![cfg(feature = "sema")]

use std::collections::BTreeMap;
use std::path::PathBuf;

use vba_diagnostics::Severity;
use vba_profile::find_profile;
use vba_project::ModuleKind;
use vba_sema::{Analysis, LibraryType, Options, SourceModule, Type, TypeProvider, analyze_sources};
use vba_typelib::sema::TypelibProvider;
use vba_typelib::{TypeKind, load_all};

fn dir() -> PathBuf {
    std::env::var_os("VBA_TYPELIB_SAMPLES").map_or_else(
        || {
            PathBuf::from(
                "/Applications/Wine Stable.app/Contents/Resources/wine/lib/wine/x86_64-windows",
            )
        },
        PathBuf::from,
    )
}

/// A provider with the given files (every library of each) plus `stdole2`.
fn provider(files: &[&str]) -> Option<TypelibProvider> {
    let mut p = TypelibProvider::new();
    for f in files.iter().chain(&["stdole2.tlb"]) {
        let Ok(data) = std::fs::read(dir().join(f)) else {
            eprintln!("{f} not found; skipping");
            return None;
        };
        for (_, lib) in load_all(&data).unwrap_or_else(|e| panic!("{f}: {e}")) {
            p.add_library(lib);
        }
    }
    Some(p)
}

fn analyse(p: &TypelibProvider, src: &str) -> Analysis {
    let profile = *find_profile("vba7.1-excel-win64").unwrap();
    analyze_sources(
        &[SourceModule {
            name: "M".into(),
            kind: ModuleKind::Standard,
            source: src.as_bytes().to_vec(),
        }],
        &profile,
        p,
        &Options::default(),
    )
}

/// Diagnostics as `CODE@line` (all severities, errors and informational).
fn report(a: &Analysis) -> Vec<(String, u32)> {
    a.diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Hint)
        .map(|d| {
            let line = a.sources.get(d.file).unwrap().line_col(d.span.start).line + 1;
            (d.code.0.to_owned(), line)
        })
        .collect()
}

fn is_identifier(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn early_bound_programs_resolve_against_real_libraries() {
    let Some(p) = provider(&["scrrun.dll", "msado15.dll"]) else {
        return;
    };
    let src = r#"Option Explicit

Sub Files()
    Dim fso As Scripting.FileSystemObject
    Dim f As Scripting.File
    Dim ts As Scripting.TextStream
    Dim d As Scripting.Dictionary
    Dim k As Variant
    Dim n As Long
    Set fso = New Scripting.FileSystemObject
    If fso.FileExists("C:\data.txt") Then
        Set f = fso.GetFile("C:\data.txt")
        Debug.Print f.Name, f.Size, f.DateLastModified, f.Path
        Set ts = f.OpenAsTextStream(ForReading)
        Do While Not ts.AtEndOfStream
            Debug.Print ts.ReadLine
        Loop
        ts.Close
    End If
    Set ts = fso.CreateTextFile("C:\out.txt", True)
    ts.WriteLine "hello"
    ts.Close
    Set ts = fso.OpenTextFile("C:\out.txt", IOMode.ForAppending, True, TristateUseDefault)
    ts.Write "x"
    ts.Close
    n = fso.GetFolder("C:\").Files.Count
    Debug.Print fso.GetExtensionName("a.txt"), fso.BuildPath("a", "b")
    Set d = New Scripting.Dictionary
    d.CompareMode = TextCompare
    d.Add "k", 1
    d("k2") = 2
    For Each k In d.Keys
        Debug.Print k, d(k)
    Next
    Debug.Print d.Count, d.Exists("k")
    d.Remove "k"
    d.RemoveAll
End Sub

Sub Database()
    Dim cn As ADODB.Connection
    Dim rs As ADODB.Recordset
    Dim cmd As ADODB.Command
    Dim fld As ADODB.Field
    Set cn = New ADODB.Connection
    cn.ConnectionString = "Provider=SQLOLEDB;Data Source=x"
    cn.Open
    Set rs = New ADODB.Recordset
    rs.CursorLocation = adUseClient
    rs.Open "SELECT a, b FROM t", cn, adOpenStatic, adLockReadOnly
    Do While Not rs.EOF
        Debug.Print rs.Fields("a").Value, rs("b"), rs!a
        rs.MoveNext
    Loop
    For Each fld In rs.Fields
        Debug.Print fld.Name, fld.Type
    Next
    Debug.Print rs.RecordCount, rs.State = adStateOpen
    rs.Close
    Set cmd = New ADODB.Command
    Set cmd.ActiveConnection = cn
    cmd.CommandText = "DELETE FROM t"
    cmd.CommandType = adCmdText
    cmd.Execute
    cn.Close
End Sub
"#;
    let a = analyse(&p, src);
    let r = report(&a);
    for d in &a.diagnostics {
        eprintln!("{}", vba_diagnostics::render_plain(d, &a.sources));
    }
    // Nothing unresolved: every type and member that exists binds.
    let bad: Vec<_> = r
        .iter()
        .filter(|(c, _)| ["SEM004", "SEM005", "SEM019", "SEM002", "SEM003"].contains(&c.as_str()))
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
    let errors: Vec<_> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn xml_and_regular_expressions_resolve_against_real_libraries() {
    let Some(p) = provider(&["msxml6.dll", "vbscript.dll"]) else {
        return;
    };
    let src = r#"Option Explicit

Sub Xml()
    Dim doc As MSXML2.DOMDocument60
    Dim node As MSXML2.IXMLDOMNode
    Dim list As MSXML2.IXMLDOMNodeList
    Dim n As Long
    Set doc = New MSXML2.DOMDocument60
    doc.async = False
    doc.validateOnParse = False
    If Not doc.loadXML("<root><a id='1'>x</a></root>") Then
        Debug.Print doc.parseError.reason, doc.parseError.line
    End If
    Set node = doc.selectSingleNode("//a")
    Debug.Print node.Text, node.nodeName, node.nodeType = NODE_ELEMENT
    Set list = doc.selectNodes("//a")
    For n = 0 To list.length - 1
        Debug.Print list.Item(n).Text, list(n).Attributes.getNamedItem("id").Text
    Next
    Debug.Print doc.documentElement.childNodes(0).xml
End Sub

Sub Regex()
    Dim re As VBScript_RegExp_55.RegExp
    Dim ms As VBScript_RegExp_55.MatchCollection
    Dim m As VBScript_RegExp_55.Match
    Set re = New VBScript_RegExp_55.RegExp
    re.Pattern = "(\d+)-(\d+)"
    re.Global = True
    re.IgnoreCase = True
    Debug.Print re.Test("10-20")
    Set ms = re.Execute("10-20 30-40")
    For Each m In ms
        Debug.Print m.Value, m.FirstIndex, m.Length, m.SubMatches(0)
    Next
    Debug.Print ms.Count, ms(0).Value, re.Replace("a-b", "$1")
End Sub
"#;
    let a = analyse(&p, src);
    for d in &a.diagnostics {
        eprintln!("{}", vba_diagnostics::render_plain(d, &a.sources));
    }
    let bad: Vec<_> = report(&a)
        .into_iter()
        .filter(|(c, _)| ["SEM002", "SEM003", "SEM004", "SEM005", "SEM019"].contains(&c.as_str()))
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn mistakes_are_found_against_real_libraries() {
    let Some(p) = provider(&["scrrun.dll", "msado15.dll"]) else {
        return;
    };
    let src = r#"Option Explicit
Sub T()
    Dim fso As Scripting.FileSystemObject
    Dim d As Scripting.Dictionary
    Dim f As Scripting.File
    Dim rs As ADODB.Recordset
    Dim m As Scripting.IOMode
    Dim i As Scripting.IDictionary
    fso.NoSuchMethod
    fso.FileExists
    fso.FileExists "a", "b"
    f.Size = 3
    d.Count = 1
    d.Add "k"
    m = ForWriting
    m = 2
    Set rs = New ADODB.Recordset
    Set i = New Scripting.IDictionary
End Sub
"#;
    let a = analyse(&p, src);
    let r: Vec<String> = report(&a)
        .into_iter()
        .filter(|(c, _)| c != "SEM030")
        .map(|(c, l)| format!("{c}@{l}"))
        .collect();
    let want = [
        "SEM019@9",  // no such member
        "SEM008@10", // missing argument
        "SEM008@11", // too many arguments
        "SEM029@12", // read-only property
        "SEM029@13", // read-only property
        "SEM008@14", // missing argument
        "SEM031@18", // IDictionary is an interface
    ];
    assert_eq!(r, want, "{:#?}", a.diagnostics);
}

#[derive(Default, Debug)]
struct Tally {
    types: usize,
    skipped_types: usize,
    members: usize,
    skipped_members: usize,
    clean: usize,
    unverified: usize,
    missing: usize,
    syntax: usize,
    other: BTreeMap<String, usize>,
    sample_bad: Vec<String>,
}

/// Binds every member of every declarable interface/class of the library
/// `name` with a statement `x.Member` and counts the outcome.
fn tally_library(p: &TypelibProvider, name: &str) -> Tally {
    let mut t = Tally::default();
    let lib = p
        .libraries()
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("library {name}"));
    for ti in &lib.types {
        if !matches!(
            ti.kind,
            TypeKind::Interface | TypeKind::Dispatch | TypeKind::CoClass
        ) {
            continue;
        }
        let lt = LibraryType {
            library: lib.name.clone(),
            name: ti.name.clone(),
        };
        if !is_identifier(&ti.name)
            || !matches!(p.resolve_declared(None, &ti.name), Some(Type::Library(_)))
        {
            t.skipped_types += 1;
            continue;
        }
        let Some(members) = p.all_members(&lt) else {
            t.skipped_types += 1;
            continue;
        };
        t.types += 1;
        let mut src = format!(
            "Option Explicit\nSub T()\nDim x As {}.{}\n",
            lib.name, ti.name
        );
        let mut lines = Vec::new();
        for m in &members {
            if !is_identifier(&m.name) {
                t.skipped_members += 1;
                continue;
            }
            // Line numbers: 1 Option, 2 Sub, 3 Dim, members from 4.
            lines.push(m.name.clone());
            src.push_str(&format!("x.{}\n", m.name));
        }
        src.push_str("End Sub\n");
        let a = analyse(p, &src);
        let r = report(&a);
        for (i, name) in lines.iter().enumerate() {
            let line = 4 + i as u32;
            t.members += 1;
            let here: Vec<&str> = r
                .iter()
                .filter(|(_, l)| *l == line)
                .map(|(c, _)| c.as_str())
                .collect();
            let mut bad = false;
            for c in &here {
                match *c {
                    "SEM005" | "SEM004" => {
                        t.unverified += 1;
                        bad = true;
                    }
                    "SEM019" => {
                        t.missing += 1;
                        bad = true;
                    }
                    c if !c.starts_with("SEM") => {
                        t.syntax += 1;
                        bad = true;
                    }
                    // Argument-list findings are expected for a bare call.
                    "SEM008" | "SEM009" | "SEM006" => {}
                    other => *t.other.entry(other.to_owned()).or_default() += 1,
                }
            }
            if bad {
                if t.sample_bad.len() < 12 {
                    t.sample_bad
                        .push(format!("{}.{}.{name}: {here:?}", lib.name, ti.name));
                }
            } else {
                t.clean += 1;
            }
        }
    }
    t
}

#[test]
#[ignore = "needs local Wine type libraries"]
fn how_many_members_bind_cleanly() {
    let sets: [(&str, &[&str], &str); 7] = [
        ("scrrun", &["scrrun.dll"], "Scripting"),
        ("msado15", &["msado15.dll"], "ADODB"),
        ("msxml3", &["msxml3.dll"], "MSXML2"),
        ("msxml6", &["msxml6.dll"], "MSXML2"),
        ("vbscript", &["vbscript.dll"], "VBScript_RegExp_10"),
        ("vbscript", &["vbscript.dll"], "VBScript_RegExp_55"),
        ("vbscript", &["vbscript.dll"], "VBScript_Global"),
    ];
    let mut total = Tally::default();
    for (label, files, lib) in sets {
        let Some(p) = provider(files) else { return };
        let t = tally_library(&p, lib);
        eprintln!("{label} ({lib}): {t:#?}");
        total.types += t.types;
        total.members += t.members;
        total.clean += t.clean;
        total.unverified += t.unverified;
        total.missing += t.missing;
        total.syntax += t.syntax;
        total.skipped_members += t.skipped_members;
        total.skipped_types += t.skipped_types;
    }
    eprintln!(
        "TOTAL: {} types, {} members, {} bind cleanly ({} unverified, {} missing, {} syntax), {} members skipped (not VBA identifiers), {} types skipped",
        total.types,
        total.members,
        total.clean,
        total.unverified,
        total.missing,
        total.syntax,
        total.skipped_members,
        total.skipped_types
    );
    assert!(total.members > 0);
    assert_eq!(total.missing, 0, "members that exist must be found");
}

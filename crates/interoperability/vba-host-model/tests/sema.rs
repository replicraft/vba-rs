//! Description -> early-bound typing and diagnostics.
#![allow(missing_docs)]

mod common;

use common::{diagnose, library};
use vba_sema::{LibGlobal, LibMemberKind, LibraryType, Type, TypeProvider};

/// The statements are the body of `Sub Main`, which starts on line 3.
fn check(lines: &str) -> Vec<String> {
    diagnose(&format!("Sub Main()\n{lines}End Sub\n"))
}

#[test]
fn a_correct_program_is_clean() {
    let d = check(
        "Dim db As Database\n\
         Dim rs As Recordset\n\
         Dim f As Field\n\
         Dim n As Long\n\
         Dim ok As Boolean\n\
         Dim k As RecordsetType\n\
         Set db = Application.CurrentDb\n\
         Set rs = db.OpenRecordset(\"t\", dbOpenSnapshot)\n\
         Set rs = db.OpenRecordset(Kind:=dbOpenTable, Name:=\"t\")\n\
         k = dbOpenDynaset\n\
         Debug.Print rs.Fields(\"a\").Value, rs.Fields!a.Value\n\
         rs.Fields(\"a\") = 5\n\
         n = rs.Lookup(\"pear\", ok)\n\
         ok = rs.FindFirst(\"pear\")\n\
         n = rs.GetRows + rs.GetRows(3) + rs.RecordCount\n\
         Debug.Print rs.Join(\"-\", 1, \"b\", 3.5)\n\
         n = Nz(Null, 0)\n\
         n = DLookup(\"a\", \"t\")\n\
         Debug.Print AppVersion, DbLib.AppVersion, DbLib.dbOpenTable\n\
         For Each f In rs.Fields\n\
         Debug.Print f.Name\n\
         Next\n\
         rs.MoveNext\n",
    );
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn an_argument_count_error_is_precise() {
    // Too few: OpenRecordset needs Name.
    assert_eq!(
        check("Dim db As Database\ndb.OpenRecordset\n"),
        ["SEM008@4"]
    );
    // Too many, for a member that cannot pass them on.
    assert_eq!(
        check("Dim rs As Recordset\nDim ok As Boolean\nok = rs.FindFirst(\"a\", \"b\")\n"),
        ["SEM008@5"]
    );
    // Too many, for a member whose result is a described class without a
    // default member (`Recordset`): nothing can take the surplus argument.
    assert_eq!(
        check("Dim db As Database\nDim rs As Recordset\nSet rs = db.OpenRecordset(\"t\", 1, 2)\n"),
        ["SEM008@5"]
    );
    // A global function with a required parameter missing.
    assert_eq!(
        check("Dim v As Variant\nv = DLookup(\"a\")\n"),
        ["SEM008@4"]
    );
    // Unknown named argument.
    assert_eq!(
        check("Dim db As Database\ndb.OpenRecordset Nme:=\"t\"\n"),
        ["SEM008@4"]
    );
}

#[test]
fn an_argument_type_mismatch_is_reported() {
    assert_eq!(check("Dim db As Database\ndb.Attach 5\n"), ["SEM006@4"]);
    assert_eq!(
        check("Dim rs As Recordset\nDim a As Application\nrs.Fields = a\n"),
        ["SEM029@5"]
    );
}

#[test]
fn unknown_members_and_types_are_reported() {
    assert_eq!(check("Dim rs As Recordset\nrs.Bogus\n"), ["SEM019@4"]);
    assert_eq!(check("Debug.Print Application.Bogus\n"), ["SEM019@3"]);
    assert_eq!(check("Dim rs As Recordsett\n"), ["SEM004@3"]);
    assert_eq!(
        check("Dim r As DbLib.Recordset\nr.MoveNext\n"),
        Vec::<String>::new()
    );
    assert_eq!(check("Dim r As Other.Recordset\n"), ["SEM004@3"]);
}

#[test]
fn byref_parameters_need_the_exact_type() {
    // Found is a ByRef Boolean: an Integer variable cannot be passed.
    assert_eq!(
        check("Dim rs As Recordset\nDim n As Long\nDim i As Integer\nn = rs.Lookup(\"p\", i)\n"),
        ["SEM026@6"]
    );
    // A Boolean variable is fine; an expression is passed by value.
    assert_eq!(
        check(
            "Dim rs As Recordset\nDim n As Long\nDim b As Boolean\nn = rs.Lookup(\"p\", b)\nn = rs.Lookup(\"p\", True)\n"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn read_only_and_write_only_are_enforced() {
    assert_eq!(
        check("Dim rs As Recordset\nrs.RecordCount = 3\n"),
        ["SEM029@4"]
    );
    assert_eq!(
        check("Dim rs As Recordset\nrs.Name = \"x\"\n"),
        Vec::<String>::new()
    );
}

#[test]
fn the_provider_answers_from_the_description() {
    let lib = library();
    let rs = LibraryType {
        library: "DbLib".into(),
        name: "Recordset".into(),
    };
    assert_eq!(
        lib.resolve_type(Some("dblib"), "RECORDSET"),
        Some(rs.clone())
    );
    assert_eq!(lib.resolve_type(Some("Other"), "Recordset"), None);
    assert!(lib.describes_members(&rs));
    let m = lib.member(&rs, "lookup").unwrap();
    assert_eq!(m.kind, LibMemberKind::Method);
    assert_eq!(m.ty, Type::Long);
    assert!(m.params[1].by_ref && !m.params[0].by_ref);
    assert!(lib.member(&rs, "Fields").is_some());
    // Default member of Fields is Item.
    let fields = LibraryType {
        library: "DbLib".into(),
        name: "Fields".into(),
    };
    assert_eq!(lib.default_member(&fields).unwrap().name, "Item");
    // Enum and globals.
    match lib.global("dbopentable") {
        Some(LibGlobal::Constant(v, Type::LibEnum(e))) => {
            assert_eq!(v, vba_value::Variant::Long(1));
            assert_eq!(e.name, "RecordsetType");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        lib.global("Application"),
        Some(LibGlobal::Object(Type::Library(_)))
    ));
    assert!(matches!(lib.global("nz"), Some(LibGlobal::Function(f)) if f.params.len() == 2));
    // Events and creatable.
    let events = lib.events(&rs).unwrap();
    assert_eq!(events[0].name, "Updated");
    assert!(events[0].params[0].by_ref);
    let db = LibraryType {
        library: "DbLib".into(),
        name: "Database".into(),
    };
    assert!(lib.class_info(&db).unwrap().creatable);
    assert!(!lib.class_info(&rs).unwrap().creatable);
}

#[test]
fn documentation_and_catalog_come_from_the_description() {
    let lib = library();
    let db = LibraryType {
        library: "DbLib".into(),
        name: "Database".into(),
    };
    assert_eq!(
        lib.documentation(Some(&db), Some("openrecordset"))
            .as_deref(),
        Some("Opens a recordset on a table.")
    );
    assert_eq!(
        lib.documentation(None, Some("Nz")).as_deref(),
        Some("Replaces Null.")
    );
    assert_eq!(
        lib.documentation(None, Some("dbOpenTable")).as_deref(),
        Some("A table.")
    );
    assert_eq!(
        lib.documentation(Some(&db), Some("Name")),
        None,
        "no text, no answer"
    );
    let names: Vec<String> = lib.catalog().into_iter().map(|e| e.name).collect();
    for n in [
        "Recordset",
        "RecordsetType",
        "AppVersion",
        "dbOpenTable",
        "Nz",
        "Application",
    ] {
        assert!(names.iter().any(|x| x == n), "{n} missing from {names:?}");
    }
}

#[test]
fn hidden_members_stay_out_of_interface_lists() {
    use vba_host_model::{HostClass, HostLibrary, HostMember};
    let lib = HostLibrary::new("L")
        .class(
            HostClass::new("C")
                .member(HostMember::sub("Shown"))
                .member(HostMember::sub("Secret").hidden()),
        )
        .check()
        .unwrap();
    let ty = LibraryType {
        library: "L".into(),
        name: "C".into(),
    };
    let names: Vec<_> = lib
        .interface_members(&ty)
        .unwrap()
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(names, ["Shown"]);
    assert!(lib.member(&ty, "Secret").is_some(), "still bindable");
}

#[test]
fn a_described_class_for_another_described_class_is_a_compile_time_mismatch() {
    // A Database where a Recordset is required: argument, assignment, result.
    assert_eq!(check("Dim db As Database\ndb.Attach db\n"), ["SEM006@4"]);
    assert_eq!(
        check("Dim db As Database\nDim rs As Recordset\nSet rs = db\n"),
        ["SEM006@5"]
    );
    assert_eq!(
        check("Dim rs As Recordset\nSet rs = Application.CurrentDb\n"),
        ["SEM006@4"]
    );
    // The right class, Nothing, Object and Variant are still fine.
    assert!(
        check(
            "Dim db As Database\nDim rs As Recordset\nDim o As Object\nDim v As Variant\n\
             Set db = Application.CurrentDb\nSet rs = db.OpenRecordset(\"t\")\n\
             Set rs = Nothing\nSet o = db\nSet rs = o\nSet v = db\ndb.Attach rs\n"
        )
        .is_empty()
    );
}

#[test]
fn for_each_uses_the_described_element_type() {
    assert_eq!(
        check("Dim rs As Recordset\nDim x As Database\nFor Each x In rs.Fields\nNext\n"),
        ["SEM018@5"]
    );
    assert!(
        check("Dim rs As Recordset\nDim f As Field\nFor Each f In rs.Fields\nDebug.Print f.Name\nNext\n")
            .is_empty()
    );
}

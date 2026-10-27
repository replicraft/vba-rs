//! A described host library (vba-host-model) as the language server's type
//! provider: completion, hover, signature help and diagnostics all come
//! from the one description.
#![allow(missing_docs)]

mod common;

use std::sync::Arc;

use common::{URI, cursor};
use vba_host_model::{
    HostClass, HostConst, HostEnum, HostGlobal, HostLibrary, HostMember, HostParam,
};
use vba_lsp_core::{CompletionKind, Workspace};

fn library() -> HostLibrary {
    HostLibrary::new("DbLib")
        .enumeration(
            HostEnum::new("RecordsetType")
                .doc("How a recordset is opened.")
                .documented("dbOpenTable", 1, "A table.")
                .member("dbOpenDynaset", 2),
        )
        .constant(HostConst::new("AppVersion", "1.0").doc("The application version."))
        .class(
            HostClass::new("Recordset")
                .doc("Rows of a table.")
                .member(HostMember::property("Name", "String").doc("The name of the recordset."))
                .member(HostMember::property("RecordCount", "Long").read_only())
                .member(HostMember::sub("MoveNext").doc("Moves to the next row."))
                .member(HostMember::sub("Secret").hidden())
                .member(
                    HostMember::function("FindFirst", "Boolean")
                        .param(HostParam::new("Criteria", "String").doc("The WHERE clause."))
                        .param(HostParam::new("Limit", "Long").default_value(1))
                        .doc("Finds the first matching row."),
                ),
        )
        .class(
            HostClass::new("Application").member(
                HostMember::function("OpenRecordset", "Recordset")
                    .param(HostParam::new("Name", "String"))
                    .doc("Opens a recordset."),
            ),
        )
        .global(HostGlobal::new("Application", "Application").doc("The running application."))
        .function(
            HostMember::function("Nz", "Variant")
                .param(HostParam::new("Value", "Variant"))
                .param(HostParam::new("ValueIfNull", "Variant").optional())
                .doc("Replaces Null by another value."),
        )
}

fn open(src: &str) -> (Workspace, vba_lsp_core::Position) {
    let (text, pos) = cursor(src);
    let mut ws = Workspace::new();
    let lib = library().check().expect("valid");
    ws.set_type_provider(Some(Arc::new(lib)));
    ws.open(URI, &text, 1);
    (ws, pos)
}

const PRELUDE: &str = "Option Explicit\nSub Run()\n    Dim rs As Recordset\n";

fn labels(src: &str) -> Vec<String> {
    let (ws, pos) = open(src);
    ws.view(URI)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect()
}

#[test]
fn members_complete_with_signatures_and_documentation() {
    let (ws, pos) = open(&format!("{PRELUDE}    rs.|\nEnd Sub\n"));
    let c = ws.view(URI).unwrap().completions(pos);
    let names: Vec<_> = c.items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(names, ["FindFirst", "MoveNext", "Name", "RecordCount"]);
    assert!(!names.contains(&"Secret"), "hidden members are not offered");
    let find = c.items.iter().find(|i| i.label == "FindFirst").unwrap();
    assert_eq!(find.kind, CompletionKind::Procedure);
    assert_eq!(
        find.detail.as_deref(),
        Some(
            "Function Recordset.FindFirst(ByVal Criteria As String, Optional ByVal Limit As Long) As Boolean"
        )
    );
    assert_eq!(
        find.documentation.as_deref(),
        Some("Finds the first matching row.")
    );
    let count = c.items.iter().find(|i| i.label == "RecordCount").unwrap();
    assert_eq!(count.kind, CompletionKind::Property);
    assert_eq!(count.documentation, None);
}

#[test]
fn chained_members_complete_through_return_types() {
    let l = labels("Sub Run()\n    Application.OpenRecordset(\"t\").|\nEnd Sub\n");
    assert!(l.contains(&"MoveNext".to_string()), "{l:?}");
}

#[test]
fn top_level_names_complete() {
    let g = labels(&format!("{PRELUDE}    |\nEnd Sub\n"));
    for want in [
        "Application",
        "Nz",
        "AppVersion",
        "dbOpenTable",
        "dbOpenDynaset",
    ] {
        assert!(g.contains(&want.to_string()), "{want} in {g:?}");
    }
    assert!(
        !g.contains(&"Recordset".to_string()),
        "types are not values: {g:?}"
    );
    let t = labels("Sub Run()\n    Dim x As |\nEnd Sub\n");
    assert!(t.contains(&"Recordset".to_string()), "{t:?}");
    assert!(t.contains(&"RecordsetType".to_string()), "{t:?}");
    assert!(!t.contains(&"Nz".to_string()), "{t:?}");
    // After the library name.
    let q = labels("Sub Run()\n    Dim x As DbLib.|\nEnd Sub\n");
    assert!(q.contains(&"Recordset".to_string()), "{q:?}");
    let (ws, pos) = open("Sub Run()\n    Debug.Print Nz|\nEnd Sub\n");
    let c = ws.view(URI).unwrap().completions(pos);
    let nz = c.items.iter().find(|i| i.label == "Nz").unwrap();
    assert_eq!(nz.kind, CompletionKind::Function);
    assert_eq!(
        nz.detail.as_deref(),
        Some(
            "Function DbLib.Nz(ByVal Value As Variant, Optional ByVal ValueIfNull As Variant) As Variant"
        )
    );
    assert_eq!(
        nz.documentation.as_deref(),
        Some("Replaces Null by another value.")
    );
}

#[test]
fn hover_shows_documentation() {
    let (ws, pos) = open(&format!("{PRELUDE}    rs.Move|Next\nEnd Sub\n"));
    let h = ws.view(URI).unwrap().hover(pos).expect("member hover");
    assert!(
        h.contents.contains("Sub Recordset.MoveNext()"),
        "{}",
        h.contents
    );
    assert!(
        h.contents.contains("Moves to the next row."),
        "{}",
        h.contents
    );

    let (ws, pos) = open(&format!("{PRELUDE}    Debug.Print N|z(Null, 1)\nEnd Sub\n"));
    let h = ws
        .view(URI)
        .unwrap()
        .hover(pos)
        .expect("global function hover");
    assert!(h.contents.contains("Function DbLib.Nz("), "{}", h.contents);
    assert!(
        h.contents.contains("Replaces Null by another value."),
        "{}",
        h.contents
    );

    let (ws, pos) = open(&format!("{PRELUDE}    Debug.Print dbOpen|Table\nEnd Sub\n"));
    let h = ws.view(URI).unwrap().hover(pos).expect("constant hover");
    assert!(
        h.contents
            .contains("Const dbOpenTable = 1 As DbLib.RecordsetType"),
        "{}",
        h.contents
    );
    assert!(h.contents.contains("A table."), "{}", h.contents);

    let (ws, pos) = open("Sub Run()\n    Debug.Print Appl|ication\nEnd Sub\n");
    let h = ws
        .view(URI)
        .unwrap()
        .hover(pos)
        .expect("global object hover");
    assert!(
        h.contents.contains("Application As DbLib.Application"),
        "{}",
        h.contents
    );
    assert!(
        h.contents.contains("The running application."),
        "{}",
        h.contents
    );

    let (ws, pos) = open("Sub Run()\n    Dim rs As Record|set\nEnd Sub\n");
    let h = ws.view(URI).unwrap().hover(pos).expect("type hover");
    assert!(
        h.contents.contains("Class DbLib.Recordset"),
        "{}",
        h.contents
    );
    assert!(h.contents.contains("Rows of a table."), "{}", h.contents);
}

#[test]
fn signature_help_for_members_and_global_functions() {
    let (ws, pos) = open(&format!("{PRELUDE}    rs.FindFirst \"a\", |\nEnd Sub\n"));
    let s = ws.view(URI).unwrap().signature_help(pos).expect("member");
    assert!(
        s.signatures[0]
            .label
            .starts_with("Function Recordset.FindFirst(")
    );
    assert_eq!(s.active_parameter, Some(1));
    let doc = s.signatures[0].documentation.as_deref().unwrap();
    assert!(doc.starts_with("Finds the first matching row."), "{doc}");

    let (ws, pos) = open(&format!("{PRELUDE}    Debug.Print Nz(Null, |)\nEnd Sub\n"));
    let s = ws
        .view(URI)
        .unwrap()
        .signature_help(pos)
        .expect("global function");
    assert_eq!(
        s.signatures[0].label,
        "Function DbLib.Nz(ByVal Value As Variant, Optional ByVal ValueIfNull As Variant) As Variant"
    );
    assert_eq!(s.active_parameter, Some(1));
    assert_eq!(s.signatures[0].parameters.len(), 2);
    assert!(
        s.signatures[0]
            .documentation
            .as_deref()
            .unwrap()
            .contains("Replaces Null by another value.")
    );
}

#[test]
fn diagnostics_follow_the_description() {
    let src = "Option Explicit\nSub Run()\n\
        Dim rs As Recordset\n\
        Dim n As Long\n\
        rs.Bogus\n\
        rs.RecordCount = 1\n\
        n = rs.FindFirst()\n\
        Dim x As Recordsett\n\
        End Sub\n";
    let mut ws = Workspace::new();
    ws.set_type_provider(Some(Arc::new(library().check().unwrap())));
    ws.open(URI, src, 1);
    let mut codes: Vec<String> = ws
        .view(URI)
        .unwrap()
        .diagnostics()
        .into_iter()
        .map(|d| d.code.to_string())
        .collect();
    codes.sort();
    assert_eq!(codes, ["SEM004", "SEM008", "SEM019", "SEM029"]);
}

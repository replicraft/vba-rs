//! A described host library through the facade: the same description types
//! the program (`Engine::analyze`) and runs it (`Engine::run`).
#![allow(missing_docs)]

#[path = "../../../interoperability/vba-host-model/tests/common/mod.rs"]
mod sample;

use vba::{Capability, Engine, Project};

fn engine() -> Engine {
    Engine::new()
        .grant(Capability::ComCreate)
        .host_library(sample::runtime())
}

fn source(body: &str) -> String {
    format!("Option Explicit\nPublic Sub Main()\n{body}End Sub\n")
}

fn run(body: &str) -> String {
    let project = Project::from_source("Module1", &source(body));
    let engine = engine();
    let report = engine.analyze(&project);
    assert!(!report.has_errors(), "{:?}", report.rendered());
    engine
        .run(&project, "Main", vec![])
        .unwrap_or_else(|e| panic!("{e}"))
        .output
}

fn analyze(body: &str) -> Vec<&'static str> {
    engine()
        .analyze(&Project::from_source("Module1", &source(body)))
        .errors()
        .map(|d| d.code.0)
        .collect()
}

#[test]
fn an_early_bound_program_runs_against_the_description() {
    let out = run("Dim db As Database\n\
         Dim rs As Recordset\n\
         Set db = Application.CurrentDb\n\
         Set rs = db.OpenRecordset(\"fruit\", dbOpenSnapshot)\n\
         Debug.Print Application.Name\n\
         Debug.Print db.Name\n\
         Debug.Print rs.Name\n\
         Debug.Print rs.Fields(\"Name\").Value\n\
         Debug.Print rs.Fields(\"Qty\").Value\n\
         Debug.Print rs.Fields.Count\n\
         Debug.Print rs.Fields(0)\n\
         rs.Fields(\"Qty\") = 7\n\
         Debug.Print rs.Fields(\"Qty\").Value\n\
         Debug.Print rs.RecordCount\n\
         Debug.Print rs.GetRows\n\
         Debug.Print rs.GetRows(9)\n\
         Debug.Print rs.FindFirst(\"pear\")\n\
         Debug.Print rs.Join(\"-\", \"a\", 2, \"c\")\n");
    assert_eq!(
        out,
        "TestApp\nmain.db\nfruit\napple\n 3 \n 2 \napple\n 7 \n 2 \n 1 \n 2 \nTrue\na-2-c\n"
    );
}

#[test]
fn named_arguments_defaults_and_byref_work_in_vba() {
    let out = run("Dim rs As Recordset\n\
         Dim found As Boolean\n\
         Dim n As Long\n\
         Set rs = Application.CurrentDb.OpenRecordset(Kind:=dbOpenTable, Name:=\"t\")\n\
         n = rs.Lookup(\"pear\", found)\n\
         Debug.Print n; found\n\
         n = rs.Lookup(\"kiwi\", found)\n\
         Debug.Print n; found\n\
         Debug.Print rs.GetRows(NumRows:=1)\n");
    assert_eq!(out, " 5 True\n 0 False\n 1 \n");
}

#[test]
fn global_functions_run_through_the_description() {
    let out = run("Dim v As Variant\n\
         Debug.Print Nz(Null, \"none\")\n\
         Debug.Print Nz(5)\n\
         v = DLookup(\"Qty\", \"fruit\")\n\
         Debug.Print v\n\
         Debug.Print DLookup(\"x\", \"y\", \"z\")\n\
         Debug.Print Nz(ValueIfNull:=\"named\", Value:=Null)\n");
    assert_eq!(out, "none\n 5 \nQty!\nx!\nnamed\n");
}

#[test]
fn creatable_classes_and_for_each() {
    let out = run("Dim db As Database\n\
         Dim f As Field\n\
         Dim rs As Recordset\n\
         Set db = CreateObject(\"DbLib.Database\")\n\
         Set rs = db.OpenRecordset(\"t\")\n\
         For Each f In rs.Fields\n\
         Debug.Print f.Name & \"=\" & f.Value\n\
         Next\n\
         Dim d2 As New Database\n\
         Debug.Print d2.Name\n");
    assert_eq!(out, "Name=apple\nQty=3\nmain.db\n");
}

#[test]
fn errors_raised_by_the_implementation_reach_err() {
    let out = run("On Error Resume Next\n\
         Dim rs As Recordset\n\
         Set rs = Application.CurrentDb.OpenRecordset(\"t\")\n\
         Debug.Print rs.Fields(\"Nope\").Value\n\
         Debug.Print Err.Number\n\
         Debug.Print Err.Description\n");
    assert_eq!(out, " 3265 \nItem not found in this collection.\n");
}

#[test]
fn late_bound_calls_use_the_same_binding() {
    // `Object` hides the type from the analysis: the call goes by name.
    let out = run("Dim o As Object\n\
         Set o = Application.CurrentDb.OpenRecordset(\"t\")\n\
         Debug.Print o.GetRows(\"2\")\n\
         On Error Resume Next\n\
         Debug.Print o.GetRows(\"many\")\n\
         Debug.Print Err.Number\n\
         Err.Clear\n\
         o.Bogus\n\
         Debug.Print Err.Number\n");
    assert_eq!(out, " 2 \n 13 \n 438 \n");
}

#[test]
fn analysis_reports_what_the_description_rules_out() {
    assert!(analyze("Dim rs As Recordset\nrs.MoveNext\n").is_empty());
    assert_eq!(analyze("Dim rs As Recordset\nrs.Bogus\n"), ["SEM019"]);
    assert_eq!(analyze("Dim rs As Recordsett\n"), ["SEM004"]);
    assert_eq!(
        analyze("Dim db As Database\ndb.OpenRecordset\n"),
        ["SEM008"]
    );
    assert_eq!(
        analyze("Dim rs As Recordset\nDim i As Integer\nDim n As Long\nn = rs.Lookup(\"p\", i)\n"),
        ["SEM026"]
    );
    assert_eq!(
        analyze("Dim rs As Recordset\nrs.RecordCount = 1\n"),
        ["SEM029"]
    );
}

#[test]
fn described_events_reach_withevents_handlers() {
    let mut p = Project::new();
    p.add_module(
        "Watcher",
        vba::ModuleKind::Class,
        "Private WithEvents rs As Recordset\n\
         Public Sub Attach(r As Recordset)\n  Set rs = r\nEnd Sub\n\
         Private Sub rs_Updated(Cancel As Integer)\n  Debug.Print \"updated\"\n  Cancel = -1\nEnd Sub\n",
    )
    .unwrap();
    p.add_module(
        "Module1",
        vba::ModuleKind::Standard,
        "Public Sub Main()\n\
           Dim rs As Recordset\n  Dim w As New Watcher\n\
           Set rs = Application.CurrentDb.OpenRecordset(\"t\")\n\
           w.Attach rs\n  rs.MoveNext\n  Debug.Print rs.EOF\n  rs.MoveNext\n  Debug.Print rs.EOF\n\
         End Sub\n",
    )
    .unwrap();
    let engine = engine();
    assert!(!engine.analyze(&p).has_errors());
    let out = engine.run(&p, "Main", vec![]).unwrap().output;
    // Both moves were cancelled by the handler through the ByRef argument.
    assert_eq!(out, "updated\nFalse\nupdated\nFalse\n");
}

#[test]
fn the_engine_without_the_library_does_not_know_its_types() {
    let project = Project::from_source("Module1", &source("Dim rs As Recordset\n"));
    assert!(Engine::new().analyze(&project).has_errors());
}

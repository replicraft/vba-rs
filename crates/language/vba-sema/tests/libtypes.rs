//! Library enums, records, aliases, coclasses, defaults and read-only
//! properties, against a table-driven provider.

mod common;

use common::{analyze_with, assert_clean, bas, cls, codes, profile64};
use vba_sema::{
    Analysis, LibEvent, LibField, LibMember, LibParam, LibraryType, StaticLibrary, Type,
};
use vba_value::Variant;

fn lt(name: &str) -> LibraryType {
    LibraryType {
        library: "Lib".into(),
        name: name.into(),
    }
}

fn library() -> StaticLibrary {
    let color = Type::LibEnum(lt("Color"));
    StaticLibrary::new("Lib")
        .with_enum("Color", &[("Red", 1), ("Green", 2), ("Blue", 4)])
        .with_alias("Handle", Type::Long)
        .with_alias("Tint", color.clone())
        .with_record(
            "Pt",
            vec![
                LibField {
                    name: "X".into(),
                    ty: Type::Long,
                    offset: Some(0),
                },
                LibField {
                    name: "Y".into(),
                    ty: Type::Long,
                    offset: Some(4),
                },
                LibField {
                    name: "Shade".into(),
                    ty: color.clone(),
                    offset: Some(8),
                },
            ],
        )
        .with_type(
            "Widget",
            vec![
                LibMember::property("Name", Type::String).with_dispid(1),
                LibMember::property("Id", Type::Long)
                    .read_only()
                    .with_dispid(2),
                LibMember::sub(
                    "Open",
                    vec![
                        LibParam::required("Mode", color.clone()).with_default(Variant::Long(1)),
                        LibParam::optional("Note", Type::String),
                        LibParam::required("Where", Type::LibRecord(lt("Pt"))),
                    ],
                )
                .with_dispid(3),
                LibMember {
                    kind: vba_sema::LibMemberKind::PropertyPut,
                    ..LibMember::property("Secret", Type::String).with_dispid(4)
                },
                LibMember::function(
                    "Tag",
                    vec![LibParam::optional("Index", Type::Long)],
                    Type::Variant,
                )
                .default_member()
                .with_dispid(0),
                LibMember::function("Paint", vec![], color).with_dispid(5),
                LibMember::property("Child", Type::Library(lt("Widget")))
                    .read_only()
                    .with_dispid(6),
                LibMember::function(
                    "Spawn",
                    vec![LibParam::optional("Kind", Type::Long)],
                    Type::Library(lt("Plain")),
                )
                .with_dispid(7),
                LibMember::function(
                    "Anything",
                    vec![LibParam::optional("Kind", Type::Long)],
                    Type::Object,
                )
                .with_dispid(8),
            ],
        )
        .with_type(
            "Plain",
            vec![LibMember::property("Label", Type::String).with_dispid(1)],
        )
        .with_creatable("Widget", true)
        .with_type("IWidget", vec![LibMember::property("Name", Type::String)])
        .with_creatable("IWidget", false)
        .with_events(
            "Widget",
            vec![LibEvent {
                name: "Changed".into(),
                params: vec![LibParam::required("Shade", Type::LibEnum(lt("Color")))],
            }],
        )
}

fn run(src: &str) -> Analysis {
    analyze_with(&[bas("M", src)], &profile64(), &library())
}

#[test]
fn enums_are_declarable_and_interchangeable_with_long() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim c As Lib.Color\n\
         Dim d As Color\n\
         Dim l As Long\n\
         c = Red\n\
         c = 2\n\
         l = c\n\
         d = c\n\
         c = l + 1\n\
         Select Case c\n\
         Case Red\n\
         l = 1\n\
         Case Green, Blue\n\
         l = 2\n\
         End Select\n\
         If c = Lib.Blue Then l = 3\n\
         c = Color.Green\n\
         c = Lib.Color.Blue\n\
         End Sub\n");
    assert_clean(&a);
    let main = a.program.find_proc("M", "Main").unwrap();
    assert_eq!(main.locals[0].ty, Type::LibEnum(lt("Color")));
    // The enum's members are kept by name for tools.
    let e = a.program.lib_enum(&lt("Color")).expect("registered");
    assert_eq!(e.members[2], ("Blue".to_owned(), 4));
}

#[test]
fn enum_constants_work_in_const_statements() {
    let a = run("Option Explicit\n\
         Const Fav = Red\n\
         Const Both = Red + Blue\n\
         Const Named = Color.Green\n\
         Sub Main()\n\
         Dim l As Long\n\
         l = Fav + Both + Named\n\
         End Sub\n");
    assert_clean(&a);
}

#[test]
fn aliases_resolve_transitively() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim h As Lib.Handle\n\
         Dim t As Tint\n\
         h = 5\n\
         t = Green\n\
         End Sub\n");
    assert_clean(&a);
    let main = a.program.find_proc("M", "Main").unwrap();
    assert_eq!(main.locals[0].ty, Type::Long);
    assert_eq!(main.locals[1].ty, Type::LibEnum(lt("Color")));
}

#[test]
fn records_have_fields_and_value_semantics() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim p As Lib.Pt\n\
         Dim q As Pt\n\
         p.X = 1\n\
         p.Shade = Blue\n\
         q = p\n\
         With q\n\
         .Y = .X + 1\n\
         End With\n\
         End Sub\n");
    assert_clean(&a);
    let rec = &a.program.lib_records[0];
    assert_eq!(rec.ty, lt("Pt"));
    assert_eq!(rec.fields.len(), 3);
    assert_eq!(rec.offsets, vec![Some(0), Some(4), Some(8)]);
    // The record id follows the project's own UDT ids.
    assert_eq!(rec.id.index(), a.program.udts.len());
    assert_eq!(a.program.udt_name(rec.id), "Pt");
    assert_eq!(a.program.udt_fields(rec.id)[1].name.to_string(), "Y");
}

#[test]
fn record_errors() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim p As Lib.Pt\n\
         p.Z = 1\n\
         p.X = 5\n\
         Dim n As Long\n\
         n = p\n\
         End Sub\n");
    assert_eq!(
        codes(&a),
        vec!["SEM019@4", "SEM006@7"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn coclasses_need_to_be_creatable() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As New Lib.Widget\n\
         Dim v As Lib.Widget\n\
         Set v = New Lib.Widget\n\
         Dim i As New Lib.IWidget\n\
         Set v = New Lib.IWidget\n\
         End Sub\n");
    assert_eq!(
        codes(&a),
        vec!["SEM031@6", "SEM031@7"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn read_only_and_write_only_properties() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim s As String\n\
         Dim n As Long\n\
         n = w.Id\n\
         w.Id = 3\n\
         w.Name = \"ok\"\n\
         w.Secret = \"ok\"\n\
         s = w.Secret\n\
         w.Paint = 1\n\
         End Sub\n");
    assert_eq!(
        codes(&a),
        vec!["SEM029@7", "SEM030@10", "SEM025@11"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn assigned_values_are_converted_to_the_property_type() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         w.Name = 12\n\
         w.Name = \"x\" & 1\n\
         End Sub\n");
    assert_clean(&a);
    let dbg = format!("{:?}", a.program.find_proc("M", "Main").unwrap().body);
    // `12` is stored as the String "12": converted at analysis time.
    assert!(dbg.contains("String"), "{dbg}");
}

#[test]
fn optional_parameters_take_the_declared_default() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim p As Lib.Pt\n\
         w.Open , , p\n\
         w.Open Green, \"n\", p\n\
         w.Open Where:=p\n\
         End Sub\n");
    assert_clean(&a);
    let dbg = format!("{:?}", a.program.find_proc("M", "Main").unwrap().body);
    // First call: Mode is the declared default 1 (a Long constant of the
    // enum), Note stays missing.
    assert!(dbg.contains("Long(1)"), "{dbg}");
    assert!(dbg.contains("Missing"), "{dbg}");
}

#[test]
fn argument_errors_against_the_declaration() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim p As Lib.Pt\n\
         w.Open 1, \"n\"\n\
         w.Open 1, \"n\", p, 4\n\
         w.Open Nope:=1, Where:=p\n\
         w.Open 1, \"n\", 5\n\
         End Sub\n");
    // Missing `Where`, too many arguments, unknown name, and a number where
    // a record is required.
    assert_eq!(
        codes(&a),
        vec!["SEM008@5", "SEM008@6", "SEM008@7", "SEM006@8"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn members_are_early_bound_by_id() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim v As Variant\n\
         Dim o As Object\n\
         w.Name = \"x\"\n\
         v = w.Name\n\
         v = o.Name\n\
         v = w(1)\n\
         End Sub\n");
    assert_clean(&a);
    let body = &a.program.find_proc("M", "Main").unwrap().body;
    let mut ids = Vec::new();
    collect_late(&format!("{body:?}"), &mut ids);
    // Name (put), Name (get), the Object's Name (late), the default member.
    assert_eq!(ids, vec![Some(1), Some(1), None, Some(0)], "{ids:?}");
}

/// The `dispid` of every `BLate` in a debug dump, in order.
fn collect_late(dump: &str, out: &mut Vec<Option<i32>>) {
    for chunk in dump.split("BLate {").skip(1) {
        let tail = chunk.split("dispid: ").nth(1).expect("dispid field");
        let value: String = tail
            .chars()
            .take_while(|c| *c != ',' && *c != '\n' && *c != '}')
            .collect();
        out.push(match value.trim() {
            "None" => None,
            s => Some(
                s.trim_start_matches("Some(")
                    .trim_end_matches(')')
                    .parse()
                    .unwrap(),
            ),
        });
    }
}

#[test]
fn events_of_library_classes() {
    let run_cls = |src: &str| analyze_with(&[cls("C", src)], &profile64(), &library());
    let a = run_cls(
        "Option Explicit\n\
         Private WithEvents w As Lib.Widget\n\
         Private Sub w_Changed(ByVal Shade As Long)\n\
         End Sub\n",
    );
    assert_clean(&a);
    let a = run_cls(
        "Option Explicit\n\
         Private WithEvents w As Lib.Widget\n\
         Private Sub w_Changed(ByVal Shade As Lib.Color)\n\
         End Sub\n\
         Private Sub w_Nope()\n\
         End Sub\n",
    );
    assert_eq!(codes(&a), vec!["SEM017@5"], "{:?}", a.diagnostics);
}

#[test]
fn default_member_of_library_classes() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim v As Variant\n\
         v = w\n\
         v = w(2)\n\
         End Sub\n");
    assert_clean(&a);
}

#[test]
fn surplus_arguments_go_to_the_default_member_of_the_result() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim v As Variant\n\
         Dim n As Long\n\
         v = w.Child(3)\n\
         v = w.Child.Tag(3)\n\
         n = w.Id(2)\n\
         v = w.Name(1)\n\
         End Sub\n");
    // `Child(3)` is `Child.Tag(3)`; a `Long` or a `String` has no default
    // member to take the argument.
    assert_eq!(
        codes(&a),
        vec!["SEM008@8", "SEM008@9"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn surplus_arguments_are_an_error_when_the_result_has_no_default_member() {
    let a = run("Option Explicit\n\
         Sub Main()\n\
         Dim w As Lib.Widget\n\
         Dim p As Lib.Plain\n\
         Dim v As Variant\n\
         Set p = w.Spawn(1)\n\
         Set p = w.Spawn(1, 2)\n\
         w.Spawn 1, 2, 3\n\
         v = w.Anything(1, 2)\n\
         v = w.Child(3)\n\
         End Sub\n");
    // `Plain` is fully described and has no default member; an `Object`
    // result is not known, and `Widget` has a default member.
    assert_eq!(
        codes(&a),
        vec!["SEM008@7", "SEM008@8"],
        "{:?}",
        a.diagnostics
    );
    let msg = a
        .diagnostics
        .iter()
        .find(|d| d.message.contains("wrong number of arguments"))
        .expect("the message names the problem");
    assert!(msg.message.contains("Spawn"), "{}", msg.message);
}

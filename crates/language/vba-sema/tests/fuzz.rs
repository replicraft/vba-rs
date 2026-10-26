//! No-panic and determinism properties on random token soups and on
//! mutated valid programs.

mod common;

use common::*;
use proptest::prelude::*;
use vba_project::ModuleKind;

const TOKENS: &[&str] = &[
    "Dim",
    "As",
    "Long",
    "String",
    "Variant",
    "Object",
    "Integer",
    "Double",
    "Boolean",
    "New",
    "Sub",
    "Function",
    "End",
    "If",
    "Then",
    "Else",
    "ElseIf",
    "For",
    "To",
    "Next",
    "Each",
    "In",
    "Do",
    "Loop",
    "While",
    "Wend",
    "Until",
    "Select",
    "Case",
    "Is",
    "With",
    "Set",
    "Let",
    "Call",
    "GoTo",
    "GoSub",
    "Return",
    "Resume",
    "On",
    "Error",
    "Exit",
    "Property",
    "Get",
    "Class",
    "Private",
    "Public",
    "Static",
    "Const",
    "Type",
    "Enum",
    "Declare",
    "PtrSafe",
    "Lib",
    "Alias",
    "Implements",
    "Event",
    "RaiseEvent",
    "WithEvents",
    "ReDim",
    "Preserve",
    "Erase",
    "Option",
    "Explicit",
    "Me",
    "Nothing",
    "Empty",
    "Null",
    "True",
    "False",
    "And",
    "Or",
    "Not",
    "Mod",
    "ByVal",
    "ByRef",
    "Optional",
    "ParamArray",
    "x",
    "y",
    "i",
    "foo",
    "Foo",
    "bar",
    "Class1",
    "Module1",
    "Collection",
    "Dictionary",
    "Err",
    "Debug",
    "Print",
    "Left$",
    "Len",
    "UBound",
    "1",
    "2",
    "42",
    "0",
    "&H10",
    "3.5",
    "\"s\"",
    "#1/1/2000#",
    "(",
    ")",
    ",",
    ".",
    "!",
    ":",
    "=",
    "+",
    "-",
    "*",
    "/",
    "\\",
    "^",
    "&",
    "<",
    ">",
    "<>",
    ":=",
    ";",
    "lbl:",
    "10",
    "'c",
    "_",
];

fn line() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(TOKENS), 0..9).prop_map(|t| t.join(" "))
}

fn source() -> impl Strategy<Value = String> {
    (prop::collection::vec(line(), 0..14), any::<bool>()).prop_map(|(lines, wrap)| {
        let body = lines.join("\n");
        if wrap {
            format!("Sub Fuzz()\n{body}\nEnd Sub\n")
        } else {
            format!("{body}\n")
        }
    })
}

const VARS: &[&str] = &[
    "i", "l", "s", "v", "o", "a", "c", "d", "u", "q", "Me", "Foo", "Nope",
];
const TEMPLATES: &[&str] = &[
    "{0} = {1}",
    "Set {0} = {1}",
    "{0} = {1} + {2}",
    "{0} = {1} & {2}",
    "{0} = {1}({2})",
    "{0}({1}) = {2}",
    "{0}.{1} = {2}",
    "{0}.{1}({2}) = {3}",
    "{0}!{1} = {2}",
    "Call {0}({1}, {2})",
    "{0} {1}, {2}",
    "{0}.Add {1}",
    "If {0} Then {1} = {2}",
    "If {0} = {1} Then\n{2} = 1\nElseIf {2} Then\n{0} = 2\nElse\n{1} = 3\nEnd If",
    "For {0} = {1} To {2} Step {3}\n{1} = {0}\nNext {0}",
    "For Each {0} In {1}\n{2} = {0}\nNext",
    "Do While {0}\n{1} = {2}\nLoop",
    "Do\n{1} = {2}\nLoop Until {0}",
    "While {0}\nWend",
    "Select Case {0}\nCase {1}\n{2} = 1\nCase {1} To {2}\nCase Is > {2}\nCase Else\nEnd Select",
    "With {0}\n.{1} = {2}\n{3} = .{1}\nEnd With",
    "ReDim {0}({1}, {2})",
    "ReDim Preserve {0}({1} To {2})",
    "Erase {0}",
    "On Error GoTo lbl\nlbl:\nResume Next",
    "On Error Resume Next",
    "GoTo lbl",
    "lbl:",
    "Exit For",
    "Exit Do",
    "Exit Sub",
    "Debug.Print {0}; {1}, {2}",
    "{0} = New {1}",
    "{0} = {1} Is {2}",
    "{0} = TypeOf {1} Is Collection",
    "{0} = Len({1}) + UBound({2})",
    "{0} = Left$({1}, {2})",
    "RaiseEvent {0}({1})",
    "Open {0} For Input As #{1}\nClose #{1}",
    "Print #{0}, {1}; {2}",
    "{0} = -{1} * Not {2}",
    "{0} = AddressOf Fuzz",
    "Static {0} As Long",
    "Const {0} = {1}",
];

fn statement() -> impl Strategy<Value = String> {
    (
        prop::sample::select(TEMPLATES),
        prop::collection::vec(prop::sample::select(VARS), 4),
    )
        .prop_map(|(t, vars)| {
            let mut out = t.to_owned();
            for (i, v) in vars.iter().enumerate() {
                out = out.replace(&format!("{{{i}}}"), v);
            }
            out
        })
}

fn structured() -> impl Strategy<Value = String> {
    prop::collection::vec(statement(), 1..10).prop_map(|stmts| {
        format!(
            "Sub Fuzz()\nDim i As Integer, l As Long, s As String, v As Variant, o As Object\nDim a(1 To 3) As Long, c As New Collection, d As New Scripting.Dictionary\nDim u As Pt, q() As Variant\n{}\nEnd Sub\nType Pt\nx As Long\nEnd Type\nFunction Foo(ByVal n As Long) As Long\nEnd Function\n",
            stmts.join("\n")
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn structured_programs_never_panic(a in structured()) {
        let mods = vec![
            module("Module1", ModuleKind::Standard, &a),
            module("Class1", ModuleKind::Class, "Event E(x As Long)\nPublic Sub Fuzz2()\nEnd Sub\n"),
        ];
        let first = analyze(&mods);
        let second = analyze(&mods);
        prop_assert_eq!(render(&first), render(&second));
    }

    #[test]
    fn token_soup_never_panics(a in source(), b in source(), kind in 0..2usize) {
        let kinds = [ModuleKind::Standard, ModuleKind::Class];
        let mods = vec![
            module("Module1", ModuleKind::Standard, &a),
            module("Class1", kinds[kind], &b),
        ];
        let first = analyze(&mods);
        let second = analyze(&mods);
        prop_assert_eq!(render(&first), render(&second));
    }
}

#[test]
fn mutated_corpus_never_panics() {
    let base = std::fs::read_to_string(format!(
        "{}/tests/fixtures/Geometry.bas",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let lines: Vec<&str> = base.lines().collect();
    for skip in 0..lines.len() {
        // Drop one line at a time.
        let src: String = lines
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != skip)
            .map(|(_, l)| format!("{l}\n"))
            .collect();
        let _ = analyze(&[bas("Geometry", &src)]);
    }
    for cut in (0..base.len()).step_by(37) {
        if base.is_char_boundary(cut) {
            let _ = analyze(&[bas("Geometry", &base[..cut])]);
        }
    }
}

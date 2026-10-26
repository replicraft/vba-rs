//! Properties: random programs never panic anywhere in parse -> sema -> IR,
//! and every program that analyses without errors lowers to IR that the
//! verifier accepts. Dumps are deterministic.

mod common;

use common::*;
use proptest::prelude::*;
use vba_ir::{DumpOptions, dump_program, lower, verify};
use vba_project::ModuleKind;

const TOKENS: &[&str] = &[
    "Dim",
    "As",
    "Long",
    "String",
    "Variant",
    "Object",
    "New",
    "Sub",
    "Function",
    "End",
    "If",
    "Then",
    "Else",
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
    "Call",
    "GoTo",
    "GoSub",
    "Return",
    "Resume",
    "On",
    "Error",
    "Exit",
    "Private",
    "Public",
    "Static",
    "Const",
    "Type",
    "ReDim",
    "Erase",
    "Me",
    "Nothing",
    "True",
    "And",
    "Or",
    "Not",
    "Mod",
    "ByVal",
    "ByRef",
    "Optional",
    "x",
    "y",
    "i",
    "foo",
    "Collection",
    "Err",
    "Debug",
    "Print",
    "Left$",
    "1",
    "2",
    "\"s\"",
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
    "&",
    "<",
    ">",
    "lbl:",
    "10",
];

fn soup() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::collection::vec(prop::sample::select(TOKENS), 0..8).prop_map(|t| t.join(" ")),
        0..12,
    )
    .prop_map(|lines| format!("Sub Fuzz()\n{}\nEnd Sub\n", lines.join("\n")))
}

// Typed placeholders keep a good share of the programs valid.
const NUM: &[&str] = &["i", "l", "1.5", "1", "2", "(i + 1)", "l * 2", "-i"];
const STR: &[&str] = &["s", "\"abc\"", "Left$(s, 1)", "s & \"x\"", "CStr(i)"];
const VAR: &[&str] = &["v", "i", "s", "Empty", "Null"];
const OBJ: &[&str] = &["o", "c", "Nothing", "New Collection"];
const TEMPLATES: &[&str] = &[
    "l = {n} + {n}",
    "s = {s} & {s}",
    "s = {s}",
    "v = {v}",
    "Set o = {o}",
    "Set c = New Collection",
    "i = Len({s})",
    "If {n} > {n} Then {n0} = 1 Else {n0} = 2",
    "If {n} = {n} Then\\n{n0} = 3\\nElseIf {n} Then\\n{n0} = 4\\nElse\\n{n0} = 5\\nEnd If",
    "For i = {n} To {n} Step {n}\\nl = l + i\\nNext i",
    "For Each v In a\\nl = l + 1\\nNext",
    "For Each v In c\\nDebug.Print v\\nNext v",
    "Do While {n} < {n}\\nl = l + 1\\nLoop",
    "Do\\nl = l + 1\\nLoop Until l > 3",
    "While l < 3\\nl = l + 1\\nWend",
    "Select Case {n}\\nCase 1\\nl = 1\\nCase 2 To 4\\nl = 2\\nCase Is > 9\\nl = 3\\nCase Else\\nl = 4\\nEnd Select",
    "With c\\n.Add {v}\\nl = .Count\\nEnd With",
    "With u\\n.x = {n}\\nl = .x\\nEnd With",
    "c.Add {v}, {s}",
    "l = c.Count",
    "v = c.Item(1)",
    "a(1) = {n}",
    "l = a(2)",
    "u.x = {n}",
    "l = u.x",
    "ReDim d(5)",
    "ReDim Preserve d(1 To {n})",
    "Erase d",
    "Debug.Print {v}; {s}, {n}",
    "On Error Resume Next",
    "On Error GoTo h",
    "On Error GoTo 0",
    "o.Name = {v}",
    "v = o.Name",
    "v = v(1)",
    "Helper l, {n}",
    "l = Twice({n})",
    "l = i And l",
    "b = {s} Like \"a*\"",
    "b = o Is c",
    "b = TypeOf o Is Collection",
    "GoSub sub1",
    "On l GoTo lbl",
    "On l GoSub sub1",
    "Open {s} For Output As #1\\nPrint #1, {v}\\nClose #1",
];

fn fill(t: &str, picks: &[usize]) -> String {
    let mut out = t.to_owned();
    let mut k = 0usize;
    let mut next = |pool: &'static [&'static str]| -> &'static str {
        let p = picks[k % picks.len()] % pool.len();
        k += 1;
        pool[p]
    };
    while let Some(pos) = out.find("{n0}") {
        out.replace_range(pos..pos + 4, "l");
    }
    for (tag, pool) in [("{n}", NUM), ("{s}", STR), ("{v}", VAR), ("{o}", OBJ)] {
        while let Some(pos) = out.find(tag) {
            let v = next(pool);
            out.replace_range(pos..pos + tag.len(), v);
        }
    }
    out
}

fn typed_program() -> impl Strategy<Value = String> {
    prop::collection::vec(
        (
            prop::sample::select(TEMPLATES),
            prop::collection::vec(0usize..16, 4),
        ),
        1..10,
    )
    .prop_map(|stmts| {
        let body: Vec<String> = stmts.iter().map(|(t, p)| fill(t, p)).collect();
        format!(
            "Type Pt\\nx As Long\\nEnd Type\\nSub Helper(ByRef a As Long, ByVal b As Long)\\nEnd Sub\\nFunction Twice(ByVal n As Long) As Long\\nTwice = n * 2\\nEnd Function\\nSub Fuzz()\\nDim i As Integer, l As Long, d() As Double, s As String, v As Variant, b As Boolean\\nDim o As Object, c As Collection, u As Pt, a(0 To 3) As Long\\n{}\\nExit Sub\\nh:\\nResume Next\\nsub1:\\nl = l + 1\\nReturn\\nlbl:\\nEnd Sub\\n",
            body.join("\\n")
        )
        .replace("\\n", "\n")
    })
}

fn check(src: &str) -> Result<(), TestCaseError> {
    let mods = vec![module("Module1", ModuleKind::Standard, src)];
    let a = analyze(&mods);
    let has_errors = a
        .diagnostics
        .iter()
        .any(|d| d.severity == vba_diagnostics::Severity::Error);
    match lower(&a) {
        Ok(ir) => {
            prop_assert!(!has_errors);
            if let Err(e) = verify(&ir) {
                let msgs: Vec<String> = e.iter().map(ToString::to_string).collect();
                prop_assert!(
                    false,
                    "verifier rejected lowered IR:\n{}\n{}",
                    msgs.join("\n"),
                    dump_program(&ir, DumpOptions::default())
                );
            }
            let again = lower(&a).expect("lower twice");
            prop_assert_eq!(
                dump_program(&ir, DumpOptions::default()),
                dump_program(&again, DumpOptions::default())
            );
        }
        Err(_) => prop_assert!(has_errors || a.program.modules.iter().any(|m| m.has_syntax_errors)),
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn typed_programs_lower_and_verify(src in typed_program()) {
        check(&src)?;
    }

    #[test]
    fn token_soup_never_panics(src in soup()) {
        check(&src)?;
    }
}

#[test]
fn typed_programs_are_mostly_valid() {
    use proptest::strategy::{Strategy, ValueTree};
    use proptest::test_runner::TestRunner;
    // Guard against a generator that never produces valid programs.
    let mut runner = TestRunner::deterministic();
    let strat = typed_program();
    let (mut ok, total) = (0, 200);
    for _ in 0..total {
        let src = strat.new_tree(&mut runner).unwrap().current();
        let a = analyze(&[module("M", ModuleKind::Standard, &src)]);
        if lower(&a).is_ok() {
            ok += 1;
        } else if std::env::var("SHOW_ERRORS").is_ok() {
            for d in a
                .diagnostics
                .iter()
                .filter(|d| d.severity == vba_diagnostics::Severity::Error)
            {
                eprintln!("{} {}", d.code.0, d.message);
            }
        }
    }
    assert!(
        ok * 10 >= total * 9,
        "only {ok}/{total} generated programs were valid"
    );
}

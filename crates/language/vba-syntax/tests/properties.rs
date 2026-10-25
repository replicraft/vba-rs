//! Property tests: the parser never panics, the tree is lossless and
//! structurally consistent on arbitrary and adversarial input.

use proptest::prelude::*;
use vba_profile::find_profile;
use vba_source::FileId;
use vba_syntax::{ParseOptions, SyntaxElement, SyntaxNode, parse};

const PROFILES: &[&str] = &[
    "access95-research",
    "vba5-excel-win32",
    "vba6-excel-win32",
    "vba7.1-excel-win32",
    "vba7.1-excel-win64",
    "vba7.1-excel-mac64",
];

fn check_node(n: SyntaxNode<'_>, src_len: u32) {
    let span = n.span();
    assert!(span.end <= src_len);
    let mut at = span.start;
    for c in n.children() {
        match c {
            SyntaxElement::Token(t) => {
                assert_eq!(t.span().start, at, "gap before token");
                assert!(!t.span().is_empty());
                at = t.span().end;
                assert_eq!(t.parent(), n);
            }
            SyntaxElement::Node(c) => {
                assert_eq!(c.span().start, at.max(c.span().start));
                if !c.span().is_empty() {
                    assert_eq!(c.span().start, at, "gap before node {c:?}");
                    at = c.span().end;
                }
                assert_eq!(c.parent(), Some(n));
                check_node(c, src_len);
            }
        }
    }
    if span.is_empty() {
        return;
    }
    assert_eq!(at, span.end, "children do not cover {n:?}");
}

fn check(src: &[u8]) {
    for id in PROFILES {
        let profile = find_profile(id).unwrap();
        for opts in [ParseOptions::tolerant(), ParseOptions::strict()] {
            let p = parse(FileId(0), src, profile, &opts);
            assert_eq!(p.tree.text(), src, "not lossless ({id})");
            assert_eq!(p.tree.root().text(), src);
            assert_eq!(p.tree.root().span().len() as usize, src.len());
            check_node(p.tree.root(), src.len() as u32);
            for d in &p.diagnostics {
                assert!(d.span.start <= d.span.end && d.span.end as usize <= src.len());
            }
            // Deterministic.
            let again = parse(FileId(0), src, profile, &opts);
            assert_eq!(again.diagnostics, p.diagnostics);
            assert_eq!(again.tree.debug_dump(), p.tree.debug_dump());
        }
    }
}

const FRAGMENTS: &[&str] = &[
    " ",
    "  ",
    "\t",
    "\n",
    "\r\n",
    "\r",
    " _\n",
    ":",
    ",",
    ";",
    ".",
    "!",
    "#",
    "(",
    ")",
    "=",
    "<>",
    "<=",
    ":=",
    "+",
    "-",
    "*",
    "/",
    "\\",
    "^",
    "&",
    "'",
    "' c\n",
    "Rem x\n",
    "\"s\"",
    "\"",
    "[x y]",
    "[",
    "#1/2/2020#",
    "12",
    "1.5e3",
    "&HFF",
    "&O7",
    "x",
    "foo",
    "Bar%",
    "s$",
    "a(1)",
    "Sub",
    "End Sub",
    "Function",
    "End Function",
    "Property Get",
    "Property",
    "End Property",
    "Dim",
    "As",
    "Integer",
    "Long",
    "String * 10",
    "LongPtr",
    "LongLong",
    "New",
    "Set",
    "Let",
    "Call",
    "If",
    "Then",
    "Else",
    "ElseIf",
    "End If",
    "EndIf",
    "Select Case",
    "Case",
    "Case Else",
    "Is",
    "End Select",
    "For",
    "To",
    "Step",
    "Each",
    "In",
    "Next",
    "Do",
    "Loop",
    "While",
    "Until",
    "Wend",
    "With",
    "End With",
    "GoTo",
    "GoSub",
    "Return",
    "On Error",
    "Resume",
    "Exit",
    "Stop",
    "End",
    "ReDim",
    "Preserve",
    "Erase",
    "Open",
    "Close",
    "Print",
    "Write",
    "Input",
    "Line Input",
    "Get",
    "Put",
    "Seek",
    "Lock",
    "Unlock",
    "Width",
    "Name",
    "Type",
    "End Type",
    "Enum",
    "End Enum",
    "Event",
    "RaiseEvent",
    "Implements",
    "WithEvents",
    "Declare",
    "PtrSafe",
    "Lib",
    "Alias",
    "Option",
    "Explicit",
    "Attribute",
    "VB_Name",
    "Public",
    "Private",
    "Friend",
    "Global",
    "Static",
    "Const",
    "ByVal",
    "ByRef",
    "Optional",
    "ParamArray",
    "And",
    "Or",
    "Not",
    "Xor",
    "Eqv",
    "Imp",
    "Mod",
    "Like",
    "TypeOf",
    "AddressOf",
    "Me",
    "Debug",
    "Nothing",
    "True",
    "False",
    "Empty",
    "Null",
    "#If",
    "#ElseIf",
    "#Else",
    "#End If",
    "#Const",
    "Win64",
    "VBA7",
    "Then\n",
    "VERSION 1.0 CLASS\n",
    "BEGIN\n",
    "END\n",
    "{",
    "}",
    "~",
    "@",
    "?",
    "0",
    "10 ",
];

fn soup() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::sample::select(FRAGMENTS), 0..60).prop_map(|v| v.concat())
}

const SEEDS: &[&str] = &[
    include_str!("fixtures/standard_module.bas"),
    include_str!("fixtures/class_module.cls"),
    include_str!("fixtures/form_codebehind.frm"),
    include_str!("fixtures/api_declares.bas"),
];

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..300)) {
        check(&bytes);
    }

    #[test]
    fn token_soup(s in soup()) {
        check(s.as_bytes());
    }

    #[test]
    fn soup_inside_a_procedure(s in soup()) {
        let src = format!("Sub T()\n{s}\nEnd Sub\n");
        check(src.as_bytes());
    }

    #[test]
    fn mutated_fixtures(seed in 0..SEEDS.len(), a in any::<usize>(), b in any::<usize>(), ins in soup()) {
        let text = SEEDS[seed].as_bytes();
        let start = a % (text.len() + 1);
        let end = start + b % (text.len() - start + 1).min(40);
        let mut m = text[..start].to_vec();
        m.extend_from_slice(ins.as_bytes());
        m.extend_from_slice(&text[end..]);
        check(&m);
    }

    #[test]
    fn truncated_fixtures(seed in 0..SEEDS.len(), a in any::<usize>()) {
        let text = SEEDS[seed].as_bytes();
        check(&text[..a % (text.len() + 1)]);
    }
}

#[test]
fn deep_nesting_does_not_overflow() {
    let n = 400;
    let mut s = String::from("Sub T()\n");
    for _ in 0..n {
        s.push_str("If x Then\n");
    }
    for _ in 0..n {
        s.push_str("End If\n");
    }
    s.push_str("End Sub\n");
    check(s.as_bytes());

    let mut e = String::from("Sub T()\nx = ");
    e.push_str(&"(".repeat(n));
    e.push('1');
    e.push_str(&")".repeat(n));
    e.push_str("\nEnd Sub\n");
    check(e.as_bytes());
}

#[test]
fn hostile_nesting_is_reported_not_overflowed() {
    let n = 50_000;
    let profile = find_profile("vba7.1-excel-win64").unwrap();
    let cases = [
        format!(
            "Sub T()\nx = {}1{}\nEnd Sub\n",
            "(".repeat(n),
            ")".repeat(n)
        ),
        format!("Sub T()\nx = {}1\nEnd Sub\n", "-".repeat(n)),
        format!("Sub T()\nx = {}1\nEnd Sub\n", "Not ".repeat(n)),
        format!(
            "Sub T()\nx = {}1{}\nEnd Sub\n",
            "f(".repeat(n),
            ")".repeat(n)
        ),
        format!("Sub T()\n{}x = 1\nEnd Sub\n", "If a Then ".repeat(n)),
        format!("Sub T()\n{}\nEnd Sub\n", "If a Then\n".repeat(n)),
        format!("Sub T()\n{}\nEnd Sub\n", "For i = 1 To 2\n".repeat(n)),
        format!("Sub T()\n{}\nEnd Sub\n", "With a\n".repeat(n)),
        format!(
            "Sub T()\n{}\nEnd Sub\n",
            "Select Case a\nCase 1\n".repeat(n)
        ),
        format!("{}\n", "#If True Then\n".repeat(n)),
    ];
    for src in &cases {
        let p = parse(
            FileId(0),
            src.as_bytes(),
            profile,
            &ParseOptions::tolerant(),
        );
        assert_eq!(p.tree.text(), src.as_bytes());
        assert!(p.has_errors());
    }
}

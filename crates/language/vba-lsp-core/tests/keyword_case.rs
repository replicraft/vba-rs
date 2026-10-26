//! Formatter keyword casing: canonical spelling, scope, idempotence.
#![allow(missing_docs)]

mod common;

use common::*;
use proptest::prelude::*;
use vba_lsp_core::*;

fn opts() -> FormatOptions {
    FormatOptions {
        keyword_case: true,
        ..FormatOptions::default()
    }
}

fn fmt(src: &str, o: FormatOptions) -> String {
    let ws = open(src);
    apply_edits(src, &ws.view(URI).unwrap().format(o))
}

#[test]
fn canonical_spelling_merges_with_indentation() {
    let src = "option explicit\nsub a()\ndim x as long\nif x then\nx = 1\nelseif x then\nx = 2\nend if\nend sub\n";
    assert_eq!(
        fmt(src, opts()),
        "Option explicit\nSub a()\n    Dim x As Long\n    If x Then\n        x = 1\n    ElseIf x Then\n        x = 2\n    End If\nEnd Sub\n"
    );
}

#[test]
fn members_comments_strings_and_longer_names_are_left_alone() {
    let src = "Sub A()\n    x.date = 1\n    s = \"end if\" ' end if\n    myend = 1\nEnd Sub\n";
    assert_eq!(fmt(src, opts()), src);
}

#[test]
fn off_by_default_and_idempotent() {
    let src = "sub a()\nfor each v in c\nnext v\nend sub\n";
    assert_eq!(
        fmt(src, FormatOptions::default()),
        "sub a()\n    for each v in c\n    next v\nend sub\n"
    );
    let once = fmt(src, opts());
    assert_eq!(fmt(&once, opts()), once);
}

#[test]
fn range_formatting_stays_in_range() {
    let src = "sub a()\ndim x as long\ndim y as long\nend sub\n";
    let ws = open(src);
    let edits = ws
        .view(URI)
        .unwrap()
        .format_range(Range::new(pos(1, 0), pos(2, 0)), opts());
    assert_eq!(
        apply_edits(src, &edits),
        "sub a()\n    Dim x As Long\ndim y as long\nend sub\n"
    );
}

fn doc() -> impl Strategy<Value = String> {
    let words = prop::sample::select(vec![
        "sub", "end", "if", "then", "else", "elseif", "dim", "as", "long", "x", "y", ".", "=", "1",
        "\"s\"", "'c", "(", ")", "For", "Next", "each", "in", "with", "Date", "Rem",
    ]);
    prop::collection::vec(prop::collection::vec(words, 0..6), 0..10).prop_map(|lines| {
        lines
            .into_iter()
            .map(|l| l.join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn only_letter_case_and_indentation_change_and_it_is_idempotent(src in doc()) {
        let once = fmt(&src, opts());
        let strip = |s: &str| -> String {
            s.split_inclusive('\n')
                .map(|l| l.trim_start_matches([' ', '\t']))
                .collect::<String>()
                .to_ascii_lowercase()
        };
        prop_assert_eq!(strip(&once), strip(&src));
        let ws = open(&once);
        prop_assert!(ws.view(URI).unwrap().format(opts()).is_empty());
    }
}

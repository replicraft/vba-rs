//! Formatter budgets and linear behaviour on hostile documents.
#![allow(missing_docs)]

mod common;

use std::time::{Duration, Instant};

use common::*;
use vba_lsp_core::*;

fn kw() -> FormatOptions {
    FormatOptions {
        keyword_case: true,
        ..FormatOptions::default()
    }
}

#[test]
fn a_document_over_the_byte_budget_is_refused_before_any_work() {
    let src = "Sub A()\nx = 1\nEnd Sub\n".repeat(100);
    let ws = open(&src);
    let v = ws.view(URI).unwrap();
    let r = v.try_format(
        FormatOptions::default(),
        &FormatLimits::unlimited().with_max_source_bytes(Some(100)),
    );
    assert_eq!(
        r,
        Err(FormatError::TooLarge {
            limit: 100,
            actual: src.len() as u64
        })
    );
    let r = v.try_format_range(
        Range::new(Position::new(0, 0), Position::new(2, 0)),
        FormatOptions::default(),
        &FormatLimits::unlimited().with_max_source_bytes(Some(100)),
    );
    assert!(matches!(r, Err(FormatError::TooLarge { .. })));
}

#[test]
fn the_budget_equal_to_the_size_is_accepted_and_matches_format() {
    let src = "Sub A()\nx = 1\nEnd Sub\n";
    let ws = open(src);
    let v = ws.view(URI).unwrap();
    let limits = FormatLimits::unlimited().with_max_source_bytes(Some(src.len() as u64));
    assert_eq!(
        v.try_format(FormatOptions::default(), &limits).unwrap(),
        v.format(FormatOptions::default())
    );
}

#[test]
fn a_huge_tab_size_is_refused_instead_of_allocated() {
    let ws = open("Sub A()\nx = 1\nEnd Sub\n");
    let v = ws.view(URI).unwrap();
    let hostile = FormatOptions {
        tab_size: u32::MAX,
        ..FormatOptions::default()
    };
    let r = v.try_format(hostile, &FormatLimits::untrusted());
    assert!(matches!(
        r,
        Err(FormatError::IndentTooWide { limit: 1024, .. })
    ));
    // Tabs do not depend on the tab size.
    let tabs = FormatOptions {
        insert_spaces: false,
        ..hostile
    };
    assert!(v.try_format(tabs, &FormatLimits::untrusted()).is_ok());
    // Ordinary options pass the untrusted budgets.
    assert!(
        v.try_format(FormatOptions::default(), &FormatLimits::untrusted())
            .is_ok()
    );
}

#[test]
fn untrusted_limits_are_finite() {
    let l = FormatLimits::untrusted();
    assert_eq!(l.max_source_bytes, Some(UNTRUSTED_FORMAT_SOURCE_BYTES));
    assert_eq!(l.max_indent_width, Some(UNTRUSTED_FORMAT_INDENT_WIDTH));
}

fn within(what: &str, t: Instant, secs: u64) {
    assert!(
        t.elapsed() < Duration::from_secs(secs),
        "{what} took {:?}",
        t.elapsed()
    );
}

/// A scan back to the nearest code line per comment line is quadratic in a
/// file that is mostly comments.
#[test]
fn a_file_of_comment_lines_formats_in_linear_time() {
    let mut src = String::from("Sub A()\nx = 1\n");
    src.push_str(&"' comment\n".repeat(150_000));
    src.push_str("x = 2\nEnd Sub\n");
    let ws = open(&src);
    let t = Instant::now();
    let edits = ws.view(URI).unwrap().format(FormatOptions::default());
    within("formatting comments", t, 20);
    // The comments take the depth of the body.
    assert_eq!(edits.len(), 150_002);
}

/// Merging keyword edits into indent edits must not scan every edit.
#[test]
fn many_keyword_edits_format_in_linear_time() {
    let mut src = String::from("sub a()\n");
    src.push_str(&"if x then\nx = 1\nend if\n".repeat(30_000));
    src.push_str("end sub\n");
    let ws = open(&src);
    let t = Instant::now();
    let edits = ws.view(URI).unwrap().format(kw());
    within("formatting keywords", t, 20);
    assert!(edits.len() > 30_000);
    let out = apply_edits(&src, &edits);
    assert!(out.starts_with("Sub a()\n    If x Then\n        x = 1\n    End If\n"));
}

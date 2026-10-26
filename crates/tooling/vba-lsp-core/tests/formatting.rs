//! The formatter: exact outputs, idempotence, losslessness.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

pub fn fmt_with(src: &str, opts: FormatOptions) -> String {
    let ws = open(src);
    apply_edits(src, &ws.view(URI).unwrap().format(opts))
}

fn fmt(src: &str) -> String {
    fmt_with(src, FormatOptions::default())
}

/// Removes the leading blanks of every line.
fn strip_indent(s: &str) -> String {
    s.split_inclusive('\n')
        .map(|l| l.trim_start_matches([' ', '\t']))
        .collect()
}

fn check(src: &str, expected: &str) {
    let out = fmt(src);
    assert_eq!(out, expected, "input:\n{src}");
    assert_eq!(fmt(&out), out, "not idempotent:\n{out}");
    assert_eq!(strip_indent(&out), strip_indent(src), "not lossless");
}

#[test]
fn control_flow_is_reindented() {
    check(
        "\
Option Explicit
  Public Sub Run()
x = 1
      If x > 0 Then
  y = 2
          ElseIf x < 0 Then
 y = 3
Else
        y = 4
    End If
    Select Case x
    Case 1
    y = 1
        Case Else
  y = 2
 End Select
        For i = 1 To 3
  ' comment in loop
   y = i
   Next i
    Do While x < 5
x = x + 1
Loop
   With obj
.Foo = 1
        End With
        GoTo done
        done:
  On Error GoTo h
 Exit Sub
h:
    Resume Next
    End Sub
",
        "\
Option Explicit
Public Sub Run()
    x = 1
    If x > 0 Then
        y = 2
    ElseIf x < 0 Then
        y = 3
    Else
        y = 4
    End If
    Select Case x
        Case 1
            y = 1
        Case Else
            y = 2
    End Select
    For i = 1 To 3
        ' comment in loop
        y = i
    Next i
    Do While x < 5
        x = x + 1
    Loop
    With obj
        .Foo = 1
    End With
    GoTo done
done:
    On Error GoTo h
    Exit Sub
h:
    Resume Next
End Sub
",
    );
}

#[test]
fn comments_stay_with_the_block_they_close() {
    check(
        "\
Sub A()
If x Then
y = 1
' trailing comment of the then branch
Else
' leading comment of the else branch
y = 2
' last comment
End If
' before end sub
End Sub
' after
",
        "\
Sub A()
    If x Then
        y = 1
        ' trailing comment of the then branch
    Else
        ' leading comment of the else branch
        y = 2
        ' last comment
    End If
    ' before end sub
End Sub
' after
",
    );
}

#[test]
fn types_enums_and_members() {
    check(
        "Public Enum E\nA\n   B = 2\nEnd Enum\n  Type T\nx As Long\n    y As String\n  End Type\n",
        "Public Enum E\n    A\n    B = 2\nEnd Enum\nType T\n    x As Long\n    y As String\nEnd Type\n",
    );
}

#[test]
fn continuation_lines_and_line_numbers_are_left_alone() {
    check(
        "Sub A()\nx = 1 + _\n        2 + _\n  3\n  y = 2\n10 z = 3\nEnd Sub\n",
        "Sub A()\n    x = 1 + _\n        2 + _\n  3\n    y = 2\n10 z = 3\nEnd Sub\n",
    );
}

#[test]
fn exported_headers_and_inactive_text_are_left_alone() {
    check(
        "\
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = \"C\"
  Sub A()
#If Win16 Then
        x = old
#Else
x = 1
#End If
End Sub
",
        "\
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = \"C\"
Sub A()
#If Win16 Then
        x = old
#Else
    x = 1
#End If
End Sub
",
    );
}

#[test]
fn blank_lines_trailing_blanks_and_token_text_are_untouched() {
    let src = "Sub A()   \n\n   \n  x   =   1   ' c  \n\t\n End Sub\n";
    let out = fmt(src);
    assert_eq!(
        out,
        "Sub A()   \n\n   \n    x   =   1   ' c  \n\t\nEnd Sub\n"
    );
}

#[test]
fn tabs_option_and_tab_size() {
    let src = "Sub A()\nIf x Then\ny = 1\nEnd If\nEnd Sub\n";
    let tabs = fmt_with(
        src,
        FormatOptions {
            tab_size: 4,
            insert_spaces: false,
        },
    );
    assert_eq!(tabs, "Sub A()\n\tIf x Then\n\t\ty = 1\n\tEnd If\nEnd Sub\n");
    let two = fmt_with(
        src,
        FormatOptions {
            tab_size: 2,
            insert_spaces: true,
        },
    );
    assert_eq!(two, "Sub A()\n  If x Then\n    y = 1\n  End If\nEnd Sub\n");
}

#[test]
fn crlf_documents_keep_their_line_endings() {
    let src = "Sub A()\r\nIf x Then\r\ny = 1\r\nEnd If\r\nEnd Sub\r\n";
    assert_eq!(
        fmt(src),
        "Sub A()\r\n    If x Then\r\n        y = 1\r\n    End If\r\nEnd Sub\r\n"
    );
}

#[test]
fn broken_code_is_formatted_best_effort_and_losslessly() {
    let src = "Sub A()\nIf x Then\ny = \n  z = 1\n";
    let out = fmt(src);
    assert_eq!(strip_indent(&out), strip_indent(src));
    assert_eq!(fmt(&out), out);
}

#[test]
fn range_formatting_touches_only_the_range() {
    let src = "Sub A()\nIf x Then\ny = 1\nz = 2\nEnd If\nEnd Sub\n";
    let ws = open(src);
    let edits = ws
        .view(URI)
        .unwrap()
        .format_range(Range::new(pos(2, 0), pos(3, 3)), FormatOptions::default());
    assert_eq!(
        apply_edits(src, &edits),
        "Sub A()\nIf x Then\n        y = 1\n        z = 2\nEnd If\nEnd Sub\n"
    );
    // a range that ends at column 0 excludes that line
    let edits = ws
        .view(URI)
        .unwrap()
        .format_range(Range::new(pos(2, 0), pos(3, 0)), FormatOptions::default());
    assert_eq!(edits.len(), 1);
}

#[test]
fn formatting_the_sample_module_changes_nothing() {
    assert_eq!(fmt(MODULE), MODULE);
}

#[test]
fn empty_and_whitespace_documents() {
    assert_eq!(fmt(""), "");
    assert_eq!(fmt("   "), "   ");
    assert_eq!(fmt("\n\n"), "\n\n");
}

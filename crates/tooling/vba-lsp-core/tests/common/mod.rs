//! Shared helpers for the feature tests.
#![allow(dead_code, missing_docs)]

use vba_lsp_core::{Position, Range, Workspace};

pub const URI: &str = "file:///test.bas";

/// A realistic module used by several features.
pub const MODULE: &str = "\
Option Explicit

Private Const MAX_ITEMS As Long = 10
Public Enum Color
    Red = 1
    Green
End Enum
Private Type Point
    X As Long
    Y As Long
End Type
Private counter As Long

' Adds two numbers.
' Returns the sum.
Public Function Add(ByVal a As Long, Optional b As Long = 2) As Long
    Dim total As Long
start:
    total = a + b
    If total > MAX_ITEMS Then
        counter = counter + 1
        GoTo start
    End If
    Add = total
End Function

Public Sub Run()
    Dim s As String
    s = Left(\"abc\", 1) & CStr(Add(1, 2))
    Debug.Print s
End Sub
";

/// Opens `src` as the document under test.
pub fn open(src: &str) -> Workspace {
    let mut ws = Workspace::new();
    ws.open(URI, src, 1);
    ws
}

/// Splits a `|` cursor marker out of `src`.
pub fn cursor(src: &str) -> (String, Position) {
    let idx = src.find('|').expect("cursor marker");
    let before = &src[..idx];
    let line = before.matches('\n').count() as u32;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let character = before[line_start..].encode_utf16().count() as u32;
    let mut text = String::from(before);
    text.push_str(&src[idx + 1..]);
    (text, Position::new(line, character))
}

/// The text covered by `r` in `src`.
pub fn slice(src: &str, r: Range) -> String {
    let mut ws = Workspace::new();
    ws.open("mem:slice", src, 0);
    let d = ws.document("mem:slice").unwrap();
    let s = d.text_map().span(d.bytes(), r);
    String::from_utf8_lossy(&d.bytes()[s.start as usize..s.end as usize]).into_owned()
}

pub fn pos(l: u32, c: u32) -> Position {
    Position::new(l, c)
}

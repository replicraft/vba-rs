//! `#` after a file statement is the file-number marker, not the start of a
//! date literal; a `#` after a number is a type character.

use vba_lexer::{LexOptions, TokenKind, lex};
use vba_profile::LanguageGeneration;
use vba_source::FileId;

fn kinds(src: &str) -> Vec<(TokenKind, String)> {
    let out = lex(
        FileId(0),
        src.as_bytes(),
        LexOptions::new(LanguageGeneration::ALL[0]),
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    out.tokens
        .iter()
        .filter(|t| !t.kind.is_trivia())
        .map(|t| (t.kind, String::from_utf8_lossy(t.text).into_owned()))
        .collect()
}

fn count(src: &str, k: TokenKind) -> usize {
    kinds(src).iter().filter(|(kind, _)| *kind == k).count()
}

#[test]
fn print_file_number_then_double_literal() {
    for src in [
        "Print #1, 5#",
        "Print #255, 123456789012345#",
        "Write #1, 1#, 2#",
        "Put #1, , 5#",
        "Print #dec, 5#",
        "Print #1, 1#; 2#",
        "Width #1, 80",
        "Reset",
    ] {
        assert_eq!(count(src, TokenKind::DateLiteral), 0, "{src}");
    }
    assert_eq!(count("Print #1, 5#", TokenKind::Hash), 1);
    assert_eq!(count("Print #1, 5#", TokenKind::FloatLiteral), 1);
}

#[test]
fn date_literals_stay_where_an_operand_is_expected() {
    for src in [
        "x = #3/4/2020#",
        "Print #3/4/2020#",
        "Debug.Print #1/2/2003#",
        "Print #1, #3/4/2020#",
        "f(#1/5/2020#)",
    ] {
        assert!(count(src, TokenKind::DateLiteral) >= 1, "{src}");
    }
    assert_eq!(count("Print #1, #3/4/2020#", TokenKind::Hash), 1);
}

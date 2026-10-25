//! Table-driven tests, one table per MS-VBAL lexical production.

use vba_lexer::{Keyword as Kw, LexOptions, TokenKind as K, TypeSuffix, lex};
use vba_profile::LanguageGeneration;
use vba_source::FileId;

fn run(src: &str, opts: LexOptions) -> (Vec<(K, String)>, Vec<&'static str>) {
    let out = lex(FileId(0), src.as_bytes(), opts);
    let joined: Vec<u8> = out
        .tokens
        .iter()
        .flat_map(|t| t.text.iter().copied())
        .collect();
    assert_eq!(joined, src.as_bytes(), "lossless for {src:?}");
    let toks = out
        .tokens
        .iter()
        .map(|t| (t.kind, String::from_utf8_lossy(t.text).into_owned()))
        .collect();
    let codes = out.diagnostics.iter().map(|d| d.code.0).collect();
    (toks, codes)
}

/// Tokens without trivia and newlines, as (kind, text).
fn sig(src: &str) -> Vec<(K, String)> {
    run(src, LexOptions::default())
        .0
        .into_iter()
        .filter(|(k, _)| !k.is_trivia() && *k != K::Newline)
        .collect()
}

fn check(table: &[(&str, &[(K, &str)])]) {
    for (src, expected) in table {
        let got = sig(src);
        let want: Vec<(K, String)> = expected.iter().map(|(k, t)| (*k, t.to_string())).collect();
        assert_eq!(got, want, "input {src:?}");
    }
}

fn all(src: &str) -> Vec<(K, String)> {
    run(src, LexOptions::default()).0
}

// ---- §3.2.1 physical lines ------------------------------------------------

#[test]
fn line_terminators() {
    for (src, n) in [
        ("a\rb", 1),
        ("a\nb", 1),
        ("a\r\nb", 1),
        ("a\r\n\r\nb", 2),
        ("a\n\rb", 2),
        ("\r\r\r", 3),
    ] {
        let count = all(src).iter().filter(|(k, _)| *k == K::Newline).count();
        assert_eq!(count, n, "{src:?}");
    }
    let t = all("a\r\nb");
    assert_eq!(t[1], (K::Newline, "\r\n".to_string()));
}

// ---- §3.2.2 line continuation --------------------------------------------

#[test]
fn line_continuation() {
    let t = all("a _\r\n b");
    assert_eq!(
        t,
        vec![
            (K::Ident, "a".into()),
            (K::LineContinuation, " _\r\n".into()),
            (K::Whitespace, " ".into()),
            (K::Ident, "b".into()),
        ]
    );
    // Needs at least one whitespace before the underscore.
    assert_eq!(all("a_\nb")[0], (K::Ident, "a_".into()));
    assert_eq!(all("_\nb")[0], (K::Unknown, "_".into()));
    // Continuation at end of file, CR and LF forms, tab as WSC.
    assert_eq!(all("a _")[1], (K::LineContinuation, " _".into()));
    assert_eq!(all("a\t_\rb")[1], (K::LineContinuation, "\t_\r".into()));
    assert_eq!(all("a  _\nb")[1], (K::LineContinuation, "  _\n".into()));
    // Text after the underscore means it is not a continuation.
    assert_eq!(all("a _ b")[1], (K::Whitespace, " ".into()));
    assert_eq!(all("a _ b")[2], (K::Unknown, "_".into()));
}

// ---- §3.3.1 comments ------------------------------------------------------

#[test]
fn comments() {
    check(&[
        ("' hello", &[]),
        ("x ' c\r\ny", &[(K::Ident, "x"), (K::Ident, "y")]),
        ("Rem hello", &[]),
        ("REM", &[]),
        ("rem: still comment", &[]),
        (
            "Remark = 1",
            &[(K::Ident, "Remark"), (K::Eq, "="), (K::IntegerLiteral, "1")],
        ),
        (
            "x.Rem",
            &[(K::Ident, "x"), (K::Dot, "."), (K::Ident, "Rem")],
        ),
    ]);
    assert_eq!(all("' a\nb")[0], (K::Comment, "' a".into()));
    assert_eq!(all("Rem a\nb")[0], (K::RemComment, "Rem a".into()));
    // Quotes and keywords inside comments are inert.
    assert_eq!(all("' \"x If")[0].1, "' \"x If");
}

#[test]
fn comment_continuation() {
    // comment-body = *(line-continuation / non-line-termination-character)
    let t = all("' a _\r\nstill comment\r\nx");
    assert_eq!(t[0], (K::Comment, "' a _\r\nstill comment".into()));
    assert_eq!(t[1].0, K::Newline);
    assert_eq!(t[2], (K::Ident, "x".into()));
    // Chain of continuations, and Rem.
    let t = all("Rem a _\n b _\n c\nx");
    assert_eq!(t[0], (K::RemComment, "Rem a _\n b _\n c".into()));
    // No whitespace before the underscore: not a continuation.
    let t = all("' a_\nx");
    assert_eq!(t[0], (K::Comment, "' a_".into()));
    // A lone underscore preceded by the quote is not a continuation either.
    assert_eq!(all("'_\nx")[0], (K::Comment, "'_".into()));
    assert_eq!(all("' _\nx")[0], (K::Comment, "' _\nx".into()));
    // Continuation immediately followed by end of file.
    assert_eq!(all("' a _\n")[0], (K::Comment, "' a _\n".into()));
}

// ---- §3.3.4 strings -------------------------------------------------------

#[test]
fn strings() {
    check(&[
        ("\"\"", &[(K::StringLiteral, "\"\"")]),
        ("\"abc\"", &[(K::StringLiteral, "\"abc\"")]),
        ("\"a\"\"b\"", &[(K::StringLiteral, "\"a\"\"b\"")]),
        ("\"\"\"\"", &[(K::StringLiteral, "\"\"\"\"")]),
        (
            "\"a' b\" c",
            &[(K::StringLiteral, "\"a' b\""), (K::Ident, "c")],
        ),
        (
            "\"x\" & \"y\"",
            &[
                (K::StringLiteral, "\"x\""),
                (K::Amp, "&"),
                (K::StringLiteral, "\"y\""),
            ],
        ),
    ]);
}

#[test]
fn unterminated_string_recovers() {
    let (t, codes) = run("x = \"abc\r\ny = 1", LexOptions::default());
    assert_eq!(codes, vec!["LEX001"]);
    assert!(t.contains(&(K::UnterminatedString, "\"abc".into())));
    assert!(t.contains(&(K::Newline, "\r\n".into())));
    assert!(t.contains(&(K::IntegerLiteral, "1".into())));
    // At end of file.
    let (t, codes) = run("\"abc", LexOptions::default());
    assert_eq!((t[0].0, codes), (K::UnterminatedString, vec!["LEX001"]));
    // The continuation is left to the continuation token.
    let (t, codes) = run("\"abc _\r\ndef\"", LexOptions::default());
    assert_eq!(t[0], (K::UnterminatedString, "\"abc".into()));
    assert_eq!(t[1], (K::LineContinuation, " _\r\n".into()));
    assert_eq!(codes[0], "LEX001");
}

// ---- §3.3.2 numbers -------------------------------------------------------

#[test]
fn integers() {
    check(&[
        ("0", &[(K::IntegerLiteral, "0")]),
        ("123", &[(K::IntegerLiteral, "123")]),
        ("12%", &[(K::IntegerLiteral, "12%")]),
        ("12&", &[(K::IntegerLiteral, "12&")]),
        ("&HFF", &[(K::HexLiteral, "&HFF")]),
        ("&hff&", &[(K::HexLiteral, "&hff&")]),
        ("&H7FFF%", &[(K::HexLiteral, "&H7FFF%")]),
        ("&HdeadBEEF", &[(K::HexLiteral, "&HdeadBEEF")]),
        ("&O17", &[(K::OctalLiteral, "&O17")]),
        ("&o17&", &[(K::OctalLiteral, "&o17&")]),
        ("&17", &[(K::OctalLiteral, "&17")]),
        ("&H", &[(K::Amp, "&"), (K::Ident, "H")]),
        ("&O9", &[(K::Amp, "&"), (K::Ident, "O9")]),
        ("&8", &[(K::Amp, "&"), (K::IntegerLiteral, "8")]),
        (
            "a &H10",
            &[(K::Ident, "a"), (K::Amp, "&"), (K::Ident, "H10")],
        ),
        (
            "a & 1",
            &[(K::Ident, "a"), (K::Amp, "&"), (K::IntegerLiteral, "1")],
        ),
        (
            "x = &H10",
            &[(K::Ident, "x"), (K::Eq, "="), (K::HexLiteral, "&H10")],
        ),
        (
            "1&2",
            &[
                (K::IntegerLiteral, "1"),
                (K::Amp, "&"),
                (K::IntegerLiteral, "2"),
            ],
        ),
    ]);
}

#[test]
fn floats() {
    check(&[
        ("1.5", &[(K::FloatLiteral, "1.5")]),
        ("1.", &[(K::FloatLiteral, "1.")]),
        (".5", &[(K::FloatLiteral, ".5")]),
        ("1e5", &[(K::FloatLiteral, "1e5")]),
        ("1E+5", &[(K::FloatLiteral, "1E+5")]),
        ("1d-5", &[(K::FloatLiteral, "1d-5")]),
        ("1.5D3", &[(K::FloatLiteral, "1.5D3")]),
        ("2.5!", &[(K::FloatLiteral, "2.5!")]),
        ("2.5#", &[(K::FloatLiteral, "2.5#")]),
        ("2.5@", &[(K::FloatLiteral, "2.5@")]),
        ("3!", &[(K::FloatLiteral, "3!")]),
        ("3#", &[(K::FloatLiteral, "3#")]),
        ("3@", &[(K::FloatLiteral, "3@")]),
        ("1e", &[(K::IntegerLiteral, "1"), (K::Ident, "e")]),
        (
            "1e+",
            &[(K::IntegerLiteral, "1"), (K::Ident, "e"), (K::Plus, "+")],
        ),
        (
            "x.5",
            &[(K::Ident, "x"), (K::Dot, "."), (K::IntegerLiteral, "5")],
        ),
    ]);
}

#[test]
fn number_suffix_accessors() {
    let out = lex(
        FileId(0),
        b"1% 1& 1! 1# 1@ x$ y% ,&HF^",
        LexOptions::default(),
    );
    let suffixes: Vec<_> = out
        .tokens
        .iter()
        .filter(|t| !t.is_trivia())
        .map(|t| t.type_suffix())
        .collect();
    use TypeSuffix as S;
    assert_eq!(
        suffixes,
        vec![
            Some(S::Integer),
            Some(S::Long),
            Some(S::Single),
            Some(S::Double),
            Some(S::Currency),
            Some(S::String),
            Some(S::Integer),
            None,
            Some(S::LongLong),
        ]
    );
}

// ---- §3.3.3 dates ---------------------------------------------------------

#[test]
fn dates() {
    for d in [
        "#1/2/2000#",
        "#12/31/1999 11:59:59 PM#",
        "#Jan 1, 2000#",
        "#10:30#",
        "#2000-01-02#",
        "#1 jan 2000 8am#",
        "# 1/2/2000 #",
    ] {
        assert_eq!(sig(d), vec![(K::DateLiteral, d.to_string())], "{d}");
    }
    check(&[(
        "x = #1/2/2000# + 1",
        &[
            (K::Ident, "x"),
            (K::Eq, "="),
            (K::DateLiteral, "#1/2/2000#"),
            (K::Plus, "+"),
            (K::IntegerLiteral, "1"),
        ],
    )]);
}

#[test]
fn bad_dates_are_errors_and_lexing_continues() {
    let (t, codes) = run("x = #1/2/2000\r\ny", LexOptions::default());
    assert_eq!(codes, vec!["LEX002"]);
    assert!(t.contains(&(K::BadDate, "#1/2/2000".into())));
    assert!(t.contains(&(K::Ident, "y".into())));
    let (t, codes) = run("#1/2/3/4#", LexOptions::default());
    assert_eq!((t[0].0, codes), (K::BadDate, vec!["LEX002"]));
}

#[test]
fn file_numbers_are_not_dates() {
    check(&[
        (
            "Print #1, x",
            &[
                (K::Keyword(Kw::Print), "Print"),
                (K::Hash, "#"),
                (K::IntegerLiteral, "1"),
                (K::Comma, ","),
                (K::Ident, "x"),
            ],
        ),
        (
            "Close #1: Close #2",
            &[
                (K::Keyword(Kw::Close), "Close"),
                (K::Hash, "#"),
                (K::IntegerLiteral, "1"),
                (K::Colon, ":"),
                (K::Keyword(Kw::Close), "Close"),
                (K::Hash, "#"),
                (K::IntegerLiteral, "2"),
            ],
        ),
        (
            "Input #n, a",
            &[
                (K::Keyword(Kw::Input), "Input"),
                (K::Hash, "#"),
                (K::Ident, "n"),
                (K::Comma, ","),
                (K::Ident, "a"),
            ],
        ),
        (
            "Open f For Input As #1",
            &[
                (K::Keyword(Kw::Open), "Open"),
                (K::Ident, "f"),
                (K::Keyword(Kw::For), "For"),
                (K::Keyword(Kw::Input), "Input"),
                (K::Keyword(Kw::As), "As"),
                (K::Hash, "#"),
                (K::IntegerLiteral, "1"),
            ],
        ),
    ]);
}

// ---- §3.3.5 identifiers, keywords, suffixes -------------------------------

#[test]
fn identifiers_and_keywords() {
    check(&[
        ("foo_bar1", &[(K::Ident, "foo_bar1")]),
        ("DIM x", &[(K::Keyword(Kw::Dim), "DIM"), (K::Ident, "x")]),
        ("dIm", &[(K::Keyword(Kw::Dim), "dIm")]),
        ("Dimension", &[(K::Ident, "Dimension")]),
        (
            "End If",
            &[(K::Keyword(Kw::End), "End"), (K::Keyword(Kw::If), "If")],
        ),
        ("EndIf", &[(K::Keyword(Kw::EndIf), "EndIf")]),
        ("Nothing", &[(K::Keyword(Kw::Nothing), "Nothing")]),
        ("tRuE", &[(K::Keyword(Kw::True), "tRuE")]),
        ("Mod", &[(K::Keyword(Kw::Mod), "Mod")]),
        (
            "Attribute VB_Name",
            &[
                (K::Keyword(Kw::Attribute), "Attribute"),
                (K::Keyword(Kw::VbAttribute), "VB_Name"),
            ],
        ),
        ("Integer", &[(K::Keyword(Kw::Integer), "Integer")]),
        ("Date", &[(K::Keyword(Kw::Date), "Date")]),
        ("Decimal", &[(K::Keyword(Kw::Decimal), "Decimal")]),
        ("Error", &[(K::Ident, "Error")]),
        ("object", &[(K::Ident, "object")]),
    ]);
}

#[test]
fn type_suffixes_on_names() {
    check(&[
        ("a%", &[(K::Ident, "a%")]),
        ("a&", &[(K::Ident, "a&")]),
        ("a!", &[(K::Ident, "a!")]),
        ("a#", &[(K::Ident, "a#")]),
        ("a@", &[(K::Ident, "a@")]),
        ("a$", &[(K::Ident, "a$")]),
        ("Date$", &[(K::Ident, "Date$")]),
        ("String$", &[(K::Ident, "String$")]),
        ("Input$", &[(K::Ident, "Input$")]),
        // `&` and `!` followed by an operand are operators.
        ("a&b", &[(K::Ident, "a"), (K::Amp, "&"), (K::Ident, "b")]),
        (
            "a&\"s\"",
            &[(K::Ident, "a"), (K::Amp, "&"), (K::StringLiteral, "\"s\"")],
        ),
        (
            "rs!Name",
            &[(K::Ident, "rs"), (K::Bang, "!"), (K::Ident, "Name")],
        ),
        (
            "rs![My Field]",
            &[
                (K::Ident, "rs"),
                (K::Bang, "!"),
                (K::ForeignName, "[My Field]"),
            ],
        ),
        // Keywords never take `#` or `&` (file numbers, concatenation).
        (
            "Print#1",
            &[
                (K::Keyword(Kw::Print), "Print"),
                (K::Hash, "#"),
                (K::IntegerLiteral, "1"),
            ],
        ),
        (
            "x^2",
            &[(K::Ident, "x"), (K::Caret, "^"), (K::IntegerLiteral, "2")],
        ),
    ]);
    // `^` is the LongLong suffix only where LongLong exists.
    assert_eq!(sig("a^ ")[0], (K::Ident, "a^".into()));
    let v6 = LexOptions::new(LanguageGeneration::Vba6);
    assert_eq!(run("a^ ", v6).0[0], (K::Ident, "a".into()));
}

#[test]
fn foreign_names() {
    check(&[
        ("[a b]", &[(K::ForeignName, "[a b]")]),
        ("[x.y]", &[(K::ForeignName, "[x.y]")]),
        (
            "[Sheet1].Range",
            &[
                (K::ForeignName, "[Sheet1]"),
                (K::Dot, "."),
                (K::Ident, "Range"),
            ],
        ),
        ("[If]", &[(K::ForeignName, "[If]")]),
    ]);
    let (t, codes) = run("[abc\r\nx", LexOptions::default());
    assert_eq!(t[0], (K::BadForeignName, "[abc".into()));
    assert_eq!(codes, vec!["LEX004"]);
    let (t, codes) = run("[]", LexOptions::default());
    assert_eq!((t[0].0, codes), (K::BadForeignName, vec!["LEX004"]));
}

// ---- §4 conditional compilation ------------------------------------------

#[test]
fn conditional_compilation() {
    check(&[
        (
            "#If Win64 Then",
            &[
                (K::HashIf, "#If"),
                (K::Ident, "Win64"),
                (K::Keyword(Kw::Then), "Then"),
            ],
        ),
        (
            "#ElseIf VBA7 Then",
            &[
                (K::HashElseIf, "#ElseIf"),
                (K::Ident, "VBA7"),
                (K::Keyword(Kw::Then), "Then"),
            ],
        ),
        ("#Else", &[(K::HashElse, "#Else")]),
        ("#End If", &[(K::HashEndIf, "#End If")]),
        ("#end   if", &[(K::HashEndIf, "#end   if")]),
        ("#EndIf", &[(K::HashEndIf, "#EndIf")]),
        ("# If", &[(K::HashIf, "# If")]),
        (
            "#Const X = 1",
            &[
                (K::HashConst, "#Const"),
                (K::Ident, "X"),
                (K::Eq, "="),
                (K::IntegerLiteral, "1"),
            ],
        ),
        ("#IfX", &[(K::Hash, "#"), (K::Ident, "IfX")]),
        ("#End", &[(K::Hash, "#"), (K::Keyword(Kw::End), "End")]),
    ]);
}

// ---- §3.3.1 special tokens and operators ---------------------------------

#[test]
fn operators_and_punctuation() {
    check(&[
        (
            "+ - * / \\ ^ & = < > <= >= <> := , ; . ! # ( ) : ?",
            &[
                (K::Plus, "+"),
                (K::Minus, "-"),
                (K::Star, "*"),
                (K::Slash, "/"),
                (K::Backslash, "\\"),
                (K::Caret, "^"),
                (K::Amp, "&"),
                (K::Eq, "="),
                (K::Lt, "<"),
                (K::Gt, ">"),
                (K::Le, "<="),
                (K::Ge, ">="),
                (K::Ne, "<>"),
                (K::ColonEq, ":="),
                (K::Comma, ","),
                (K::Semicolon, ";"),
                (K::Dot, "."),
                (K::Bang, "!"),
                (K::Hash, "#"),
                (K::LParen, "("),
                (K::RParen, ")"),
                (K::Colon, ":"),
                (K::Question, "?"),
            ],
        ),
        (
            "f(a:=1)",
            &[
                (K::Ident, "f"),
                (K::LParen, "("),
                (K::Ident, "a"),
                (K::ColonEq, ":="),
                (K::IntegerLiteral, "1"),
                (K::RParen, ")"),
            ],
        ),
        ("a: b", &[(K::Ident, "a"), (K::Colon, ":"), (K::Ident, "b")]),
        ("a<b", &[(K::Ident, "a"), (K::Lt, "<"), (K::Ident, "b")]),
        (
            "Debug.Print x",
            &[
                (K::Keyword(Kw::Debug), "Debug"),
                (K::Dot, "."),
                (K::Keyword(Kw::Print), "Print"),
                (K::Ident, "x"),
            ],
        ),
        (
            "a.b.c",
            &[
                (K::Ident, "a"),
                (K::Dot, "."),
                (K::Ident, "b"),
                (K::Dot, "."),
                (K::Ident, "c"),
            ],
        ),
        ("? 1", &[(K::Question, "?"), (K::IntegerLiteral, "1")]),
    ]);
}

// ---- generation gating ----------------------------------------------------

#[test]
fn keyword_gating_by_generation() {
    let word = |s: &str, g| run(s, LexOptions::new(g)).0[0].0;
    for w in [
        "LongPtr",
        "LongLong",
        "PtrSafe",
        "CLngLng",
        "CLngPtr",
        "DefLngLng",
        "DefLngPtr",
    ] {
        assert!(
            matches!(word(w, LanguageGeneration::Vba7_1), K::Keyword(_)),
            "{w}"
        );
        assert!(
            matches!(word(w, LanguageGeneration::Vba7_0), K::Keyword(_)),
            "{w}"
        );
        assert_eq!(word(w, LanguageGeneration::Vba6), K::Ident, "{w}");
        assert_eq!(word(w, LanguageGeneration::Vba5), K::Ident, "{w}");
    }
    assert!(matches!(
        word("Implements", LanguageGeneration::Vba5),
        K::Keyword(_)
    ));
    assert_eq!(word("Implements", LanguageGeneration::Access95), K::Ident);
    assert!(matches!(
        word("Dim", LanguageGeneration::Access95),
        K::Keyword(_)
    ));
}

// ---- error tolerance and strict mode --------------------------------------

#[test]
fn unknown_bytes() {
    let (t, codes) = run("a ` b", LexOptions::default());
    assert!(t.contains(&(K::Unknown, "`".into())));
    assert_eq!(codes, vec!["LEX003"]);
    let (t, codes) = run("a\u{0}b", LexOptions::default());
    assert_eq!(t[1].0, K::Unknown);
    assert_eq!(codes.len(), 1);
}

#[test]
fn non_ascii_bytes_and_strict_mode() {
    // Windows-1252 "é" (0xE9) inside an identifier; not valid UTF-8.
    let src = b"caf\xE9 = 1";
    let tolerant = lex(FileId(0), src, LexOptions::default());
    assert_eq!(tolerant.tokens[0].kind, K::Ident);
    assert_eq!(tolerant.tokens[0].text, b"caf\xE9");
    assert!(tolerant.diagnostics.is_empty());
    let strict = lex(FileId(0), src, LexOptions::default().strict());
    assert_eq!(strict.diagnostics.len(), 1);
    assert_eq!(strict.diagnostics[0].code.0, "LEX005");
    // Strings and comments may hold any bytes without diagnostics.
    let s = lex(
        FileId(0),
        b"x = \"\xFF\xFE\" ' \x80",
        LexOptions::default().strict(),
    );
    assert!(s.diagnostics.is_empty());
}

#[test]
fn spans_match_text() {
    let src = b"Dim x As Long ' c\r\nx = 1";
    let out = lex(FileId(0), src, LexOptions::default());
    let mut at = 0u32;
    for t in &out.tokens {
        assert_eq!(t.span.start, at);
        assert_eq!(t.span.slice(src).unwrap(), t.text);
        at = t.span.end;
    }
    assert_eq!(at as usize, src.len());
}

#[test]
fn realistic_module() {
    let src = "Attribute VB_Name = \"Module1\"\r\nOption Explicit\r\n\
#If VBA7 Then\r\nPrivate Declare PtrSafe Function GetTickCount Lib \"kernel32\" () As LongPtr\r\n#Else\r\n\
Private Declare Function GetTickCount Lib \"kernel32\" () As Long\r\n#End If\r\n\
Public Sub Main(ByVal n As Long, Optional s$ = \"x\")\r\n    Dim i As Long: i = &HFF& + 1.5E3\r\n    \
Debug.Print \"a\"\"b\"; _\r\n        i ' trailing\r\n    Rem done\r\nEnd Sub\r\n";
    let (t, codes) = run(src, LexOptions::default());
    assert!(codes.is_empty(), "{codes:?}");
    assert!(t.iter().all(|(k, _)| !k.is_error()));
    assert!(t.contains(&(K::Keyword(Kw::PtrSafe), "PtrSafe".into())));
    assert!(t.contains(&(K::HexLiteral, "&HFF&".into())));
    assert!(t.contains(&(K::FloatLiteral, "1.5E3".into())));
    assert!(t.contains(&(K::RemComment, "Rem done".into())));
}

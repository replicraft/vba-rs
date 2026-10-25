//! Property tests: losslessness and robustness on random and adversarial input.

use proptest::prelude::*;
use vba_lexer::{LexOptions, Lexer, TokenKind, lex};
use vba_profile::LanguageGeneration;
use vba_source::FileId;

fn all_options() -> Vec<LexOptions> {
    let mut v = Vec::new();
    for g in LanguageGeneration::ALL {
        v.push(LexOptions::new(g));
        v.push(LexOptions::new(g).strict());
    }
    v
}

/// Core invariants every lex of every input must satisfy.
fn check_invariants(src: &[u8]) {
    for opts in all_options() {
        let out = lex(FileId(0), src, opts);
        // Lossless and contiguous.
        let mut at = 0usize;
        let mut joined = Vec::with_capacity(src.len());
        for t in &out.tokens {
            assert_eq!(t.span.start as usize, at, "gap or overlap");
            assert!(!t.text.is_empty(), "empty token");
            assert_eq!(t.span.len() as usize, t.text.len());
            assert_eq!(&src[t.span.start as usize..t.span.end as usize], t.text);
            at = t.span.end as usize;
            joined.extend_from_slice(t.text);
        }
        assert_eq!(at, src.len());
        assert_eq!(joined, src);
        // Diagnostics point inside the input and error tokens have one.
        for d in &out.diagnostics {
            assert!(d.span.end as usize <= src.len());
            assert!(d.span.start <= d.span.end);
        }
        let error_tokens = out.tokens.iter().filter(|t| t.kind.is_error()).count();
        assert!(out.diagnostics.len() >= error_tokens);
        // Deterministic.
        let again: Vec<_> = Lexer::new(FileId(0), src, opts).collect();
        assert_eq!(again, out.tokens);
        // Newline tokens are exactly CR, LF or CRLF, nothing else.
        for t in out.tokens.iter().filter(|t| t.kind == TokenKind::Newline) {
            assert!(matches!(t.text, b"\r" | b"\n" | b"\r\n"));
        }
    }
}

/// Fragments that exercise every lexical production, glued randomly.
const FRAGMENTS: &[&str] = &[
    " ",
    "  ",
    "\t",
    "\r",
    "\n",
    "\r\n",
    " _\r\n",
    " _\n",
    "_",
    " _",
    "'",
    "' c _\r\n",
    "Rem ",
    "\"",
    "\"\"",
    "\"a\"",
    "[",
    "]",
    "[x y]",
    "#",
    "#1/2/2000#",
    "#1/2",
    "#10:30 PM#",
    "#If ",
    "#Else",
    "#End If",
    "#Const ",
    "&H",
    "&HFF&",
    "&O7",
    "&7",
    "&",
    "%",
    "^",
    "!",
    "@",
    "$",
    "1",
    "12",
    "1.5",
    ".5",
    "1e5",
    "1E+",
    "2D-3",
    "x",
    "foo_1",
    "Dim ",
    "End",
    "If ",
    "Then",
    "Date$",
    "LongPtr",
    "PtrSafe",
    "Attribute",
    "VB_Name",
    "True",
    "Nothing",
    ":",
    ":=",
    ",",
    ";",
    ".",
    "(",
    ")",
    "<",
    ">",
    "<=",
    ">=",
    "<>",
    "=",
    "+",
    "-",
    "*",
    "/",
    "\\",
    "?",
    "\u{e9}",
    "\u{0}",
    "`",
    "{",
    "}",
    "~",
    "|",
    "\x19",
];

fn fragment_soup() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..60)
        .prop_map(|v| v.concat().into_bytes())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn random_bytes_roundtrip(src in prop::collection::vec(any::<u8>(), 0..400)) {
        check_invariants(&src);
    }

    #[test]
    fn mostly_ascii_roundtrip(src in prop::collection::vec(
        prop_oneof![9 => 0x09u8..0x7F, 1 => any::<u8>()], 0..400)) {
        check_invariants(&src);
    }

    #[test]
    fn fragment_soup_roundtrip(src in fragment_soup()) {
        check_invariants(&src);
    }

    /// Lexing a prefix of the input never reads past it and still tiles it.
    #[test]
    fn every_prefix_roundtrips(src in fragment_soup(), cut in 0usize..200) {
        let cut = cut.min(src.len());
        check_invariants(&src[..cut]);
    }

    /// Tokens of whole lines (outside comment/continuation joins) are not
    /// affected by what follows a line terminator that ends a plain line.
    #[test]
    fn appending_a_line_keeps_earlier_tokens(a in "[a-z0-9 =+()\"]{0,30}", b in fragment_soup()) {
        let mut combined = a.clone().into_bytes();
        combined.push(b'\n');
        let head: Vec<_> = lex(FileId(0), &combined, LexOptions::default())
            .tokens
            .iter()
            .map(|t| (t.kind, t.text.to_vec()))
            .collect();
        combined.extend_from_slice(&b);
        let full: Vec<_> = lex(FileId(0), &combined, LexOptions::default())
            .tokens
            .iter()
            .map(|t| (t.kind, t.text.to_vec()))
            .collect();
        prop_assert!(full.starts_with(&head));
    }
}

#[test]
fn adversarial_inputs() {
    let long = |s: &str, n: usize| s.repeat(n).into_bytes();
    let cases: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        vec![0xFF; 100],
        long("\"", 5000),
        long("#", 5000),
        long("# ", 5000),
        long("#1/", 3000),
        long("[", 5000),
        long("' _\r\n", 2000),
        long("Rem _\n", 2000),
        long(" _", 5000),
        long("_\n", 2000),
        long("&", 5000),
        long("&H", 3000),
        long("1.", 3000),
        long("1e", 3000),
        long(".", 5000),
        long("a", 100_000),
        long("a$", 10_000),
        long("\r", 5000),
        long("\r\n", 5000),
        long("\"\"", 5000),
        long("\"a\" ", 3000),
        long("#If #Else #End #", 1000),
        long("Date$ String$ Input$", 1000),
        long("\u{e9}", 5000),
        b"\"".iter().chain(&[0xE9; 50]).copied().collect(),
        // Quadratic-looking date scans must stay fast.
        long("#1 2 3 4 5 6 7 8 9 ", 2000),
        long("#a ", 20000),
    ];
    for c in cases {
        check_invariants(&c);
    }
}

#[test]
fn every_single_byte_and_pair() {
    for a in 0..=255u8 {
        check_invariants(&[a]);
        check_invariants(&[a, b'\n']);
    }
    // All pairs from an interesting alphabet.
    let alphabet = b" \t\r\n_'\"#[]&!%^@$.:=<>0aEHeD1/-,";
    for &a in alphabet {
        for &b in alphabet {
            for &c in alphabet {
                check_invariants(&[a, b, c]);
            }
        }
    }
}

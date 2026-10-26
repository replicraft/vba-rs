//! Expression canonicalisation used to compare the instruction stream of a
//! line with the tokens of the source line (differential reasoning).
//!
//! Both sides are reduced to the same fully parenthesised text: the source
//! side by a precedence parser over the lexer tokens (MS-VBAL 5.6 operator
//! precedence), the cache side by replaying the instructions on an
//! expression stack under the stack semantics being tested.

#![allow(dead_code, missing_docs)]

use vba_lexer::TokenKind;
use vba_pcode::{Listed, Name, Opcode, Operand, Radix};

use super::Tok;

#[derive(Debug, Clone, PartialEq)]
pub enum Ex {
    Lit(String),
    Name(String),
    Un(&'static str, Box<Ex>),
    Bin(&'static str, Box<Ex>, Box<Ex>),
    Call(String, Vec<Ex>),
}

impl Ex {
    pub fn show(&self) -> String {
        match self {
            Ex::Lit(s) | Ex::Name(s) => s.clone(),
            Ex::Un(op, a) => format!("({op} {})", a.show()),
            Ex::Bin(op, a, b) => format!("({} {op} {})", a.show(), b.show()),
            Ex::Call(n, args) => format!(
                "{n}({})",
                args.iter().map(Ex::show).collect::<Vec<_>>().join(", ")
            ),
        }
    }
}

pub fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn strip(t: &[u8]) -> String {
    let s: String = t
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .to_lowercase();
    s.trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches(|c| "%&!#@$".contains(c))
        .to_string()
}

struct P<'a> {
    t: &'a [Tok],
    i: usize,
}

fn binop(t: &Tok) -> Option<(&'static str, u8)> {
    Some(match t.kind {
        TokenKind::Plus => ("+", 9),
        TokenKind::Minus => ("-", 9),
        TokenKind::Star => ("*", 12),
        TokenKind::Slash => ("/", 12),
        TokenKind::Backslash => ("\\", 11),
        TokenKind::Caret => ("^", 14),
        TokenKind::Amp => ("&", 8),
        TokenKind::Eq => ("=", 7),
        TokenKind::Ne => ("<>", 7),
        TokenKind::Lt => ("<", 7),
        TokenKind::Gt => (">", 7),
        TokenKind::Le => ("<=", 7),
        TokenKind::Ge => (">=", 7),
        _ => match strip(&t.text).as_str() {
            "mod" => ("mod", 10),
            "like" => ("like", 7),
            "is" => ("is", 7),
            "and" => ("and", 5),
            "or" => ("or", 4),
            "xor" => ("xor", 3),
            "eqv" => ("eqv", 2),
            "imp" => ("imp", 1),
            _ => return None,
        },
    })
}

impl P<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }

    fn expr(&mut self, min: u8) -> Option<Ex> {
        let mut lhs = self.prefix()?;
        while let Some(tok) = self.peek() {
            let Some((op, bp)) = binop(tok) else { break };
            if bp < min {
                break;
            }
            self.i += 1;
            let rhs = self.expr(bp + 1)?;
            lhs = Ex::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Some(lhs)
    }

    fn prefix(&mut self) -> Option<Ex> {
        let tok = self.peek()?.clone();
        self.i += 1;
        match tok.kind {
            TokenKind::Minus => Some(Ex::Un("neg", Box::new(self.expr(13)?))),
            TokenKind::Plus => self.expr(13),
            TokenKind::LParen => {
                let e = self.expr(0)?;
                if self.peek()?.kind != TokenKind::RParen {
                    return None;
                }
                self.i += 1;
                Some(e)
            }
            TokenKind::StringLiteral => Some(Ex::Lit(format!(
                "\"{}\"",
                String::from_utf8_lossy(&tok.text[1..tok.text.len() - 1]).replace("\"\"", "\"")
            ))),
            TokenKind::IntegerLiteral | TokenKind::HexLiteral | TokenKind::OctalLiteral => {
                let s = String::from_utf8_lossy(&tok.text).to_lowercase();
                let s = s.trim_end_matches(|c| "%&^".contains(c)).to_string();
                let v = if let Some(h) = s.strip_prefix("&h") {
                    i64::from_str_radix(h, 16).ok()?
                } else if let Some(o) = s.strip_prefix("&o") {
                    i64::from_str_radix(o, 8).ok()?
                } else if let Some(o) = s.strip_prefix('&') {
                    i64::from_str_radix(o, 8).ok()?
                } else {
                    s.parse().ok()?
                };
                // `&H8000` style literals are 16-bit signed in the cache.
                let v = if tok.kind == TokenKind::HexLiteral
                    && !tok.text.last().is_some_and(|c| *c == b'&')
                    && (0x8000..=0xFFFF).contains(&v)
                {
                    v - 0x10000
                } else {
                    v
                };
                Some(Ex::Lit(num(v as f64)))
            }
            TokenKind::FloatLiteral => {
                let s = String::from_utf8_lossy(&tok.text).to_lowercase();
                let single = s.ends_with('!');
                let s = s.trim_end_matches(|c| "!#@".contains(c)).replace('d', "e");
                if single {
                    Some(Ex::Lit(num(f64::from(s.parse::<f32>().ok()?))))
                } else {
                    Some(Ex::Lit(num(s.parse::<f64>().ok()?)))
                }
            }
            TokenKind::Ident | TokenKind::Keyword(_) => {
                let n = strip(&tok.text);
                match n.as_str() {
                    "not" => return Some(Ex::Un("not", Box::new(self.expr(6)?))),
                    "true" | "false" | "nothing" | "empty" | "null" => return Some(Ex::Lit(n)),
                    _ => {}
                }
                if binop(&tok).is_some() {
                    return None;
                }
                if self.peek().is_some_and(|t| t.kind == TokenKind::LParen) {
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.peek()?.kind == TokenKind::RParen {
                        self.i += 1;
                        return Some(Ex::Call(n, args));
                    }
                    loop {
                        args.push(self.expr(0)?);
                        let t = self.peek()?;
                        match t.kind {
                            TokenKind::Comma => self.i += 1,
                            TokenKind::RParen => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                    Some(Ex::Call(n, args))
                } else {
                    Some(Ex::Name(n))
                }
            }
            _ => None,
        }
    }
}

/// Parses a whole token slice as one expression.
pub fn parse_source(toks: &[Tok]) -> Option<Ex> {
    // Member access and dictionary access are outside the checked subset.
    if toks.iter().any(|t| {
        matches!(
            t.kind,
            TokenKind::Dot | TokenKind::Bang | TokenKind::ColonEq
        )
    }) {
        return None;
    }
    let mut p = P { t: toks, i: 0 };
    let e = p.expr(0)?;
    (p.i == toks.len()).then_some(e)
}

/// Replays expression instructions on a stack. Returns the stack (one
/// element for a well-formed expression) and the remaining instructions
/// from the first non-expression opcode.
pub fn replay(ins: &[Listed]) -> Option<(Vec<Ex>, usize)> {
    let mut st: Vec<Ex> = Vec::new();
    let mut k = 0;
    while k < ins.len() {
        let i = &ins[k];
        use Opcode::*;
        let bin = |st: &mut Vec<Ex>, op: &'static str| -> Option<()> {
            let b = st.pop()?;
            let a = st.pop()?;
            st.push(Ex::Bin(op, Box::new(a), Box::new(b)));
            Some(())
        };
        let un1 = |st: &mut Vec<Ex>, f: &str| -> Option<()> {
            let a = st.pop()?;
            st.push(Ex::Call(f.to_string(), vec![a]));
            Some(())
        };
        match i.opcode {
            BoolImplication => bin(&mut st, "imp")?,
            BoolEquivalence => bin(&mut st, "eqv")?,
            BoolExclusiveOr => bin(&mut st, "xor")?,
            BoolDisjunction => bin(&mut st, "or")?,
            BoolConjunction => bin(&mut st, "and")?,
            CompareEqual => bin(&mut st, "=")?,
            CompareNotEqual => bin(&mut st, "<>")?,
            CompareLessEqual => bin(&mut st, "<=")?,
            CompareGreaterEqual => bin(&mut st, ">=")?,
            CompareLess => bin(&mut st, "<")?,
            CompareGreater => bin(&mut st, ">")?,
            ComparePattern => bin(&mut st, "like")?,
            CompareObjectIdentity => bin(&mut st, "is")?,
            NumAdd => bin(&mut st, "+")?,
            NumSubtract => bin(&mut st, "-")?,
            NumRemainder => bin(&mut st, "mod")?,
            NumIntDivide => bin(&mut st, "\\")?,
            NumMultiply => bin(&mut st, "*")?,
            NumDivide => bin(&mut st, "/")?,
            NumPower => bin(&mut st, "^")?,
            TextJoin => bin(&mut st, "&")?,
            BoolNegate => {
                let a = st.pop()?;
                st.push(Ex::Un("not", Box::new(a)));
            }
            NumNegate => {
                let a = st.pop()?;
                st.push(Ex::Un("neg", Box::new(a)));
            }
            IntrinsicAbsoluteValue => un1(&mut st, "abs")?,
            IntrinsicTruncate => un1(&mut st, "fix")?,
            IntrinsicFloor => un1(&mut st, "int")?,
            IntrinsicSign => un1(&mut st, "sgn")?,
            IntrinsicLengthChars => un1(&mut st, "len")?,
            IntrinsicLengthBytes => un1(&mut st, "lenb")?,
            ExprParentheses | LineContinuation => {}
            ReadSimple => {
                let Operand::Name { name, .. } = i.operands.first()? else {
                    return None;
                };
                st.push(Ex::Name(name.spelling()?.to_lowercase()));
            }
            ReadSimpleParams => {
                let Operand::Name { name, .. } = i.operands.first()? else {
                    return None;
                };
                let Operand::Count(c) = i.operands.get(1)? else {
                    return None;
                };
                let n = usize::from(*c);
                if st.len() < n {
                    return None;
                }
                let args = st.split_off(st.len() - n);
                st.push(Ex::Call(name.spelling()?.to_lowercase(), args));
            }
            ConstInt16Decimal | ConstInt32Decimal | ConstInt16Hex | ConstInt32Hex
            | ConstInt16Octal | ConstInt32Octal => {
                let Operand::Int { value, radix } = i.operands.first()? else {
                    return None;
                };
                let _ = matches!(radix, Radix::Decimal);
                st.push(Ex::Lit(num(*value as f64)));
            }
            ConstFloat64 => {
                let Operand::Float64(v) = i.operands.first()? else {
                    return None;
                };
                st.push(Ex::Lit(num(*v)));
            }
            ConstFloat32 => {
                let Operand::Float32(v) = i.operands.first()? else {
                    return None;
                };
                st.push(Ex::Lit(num(f64::from(*v))));
            }
            ConstText => {
                let Operand::Text(t) = i.operands.first()? else {
                    return None;
                };
                st.push(Ex::Lit(format!(
                    "\"{}\"",
                    t.iter().map(|&b| char::from(b)).collect::<String>()
                )));
            }
            ConstKeywordValue => {
                let n = match i.modifier {
                    0 => "false",
                    1 => "true",
                    2 => "null",
                    3 => "empty",
                    _ => return None,
                };
                st.push(Ex::Lit(n.into()));
            }
            ConstNullObject => st.push(Ex::Lit("nothing".into())),
            IntrinsicSearchA | IntrinsicSearchB | IntrinsicSearchC => {
                let n = match i.opcode {
                    IntrinsicSearchA => 2,
                    IntrinsicSearchB => 3,
                    _ => 4,
                };
                if st.len() < n {
                    return None;
                }
                let args = st.split_off(st.len() - n);
                st.push(Ex::Call("instr".into(), args));
            }
            IntrinsicLowerBound | IntrinsicUpperBound => {
                let Operand::Count(w) = i.operands.first()? else {
                    return None;
                };
                let n = usize::from(*w) + 1;
                if st.len() < n {
                    return None;
                }
                let args = st.split_off(st.len() - n);
                let f = if i.opcode == IntrinsicLowerBound {
                    "lbound"
                } else {
                    "ubound"
                };
                st.push(Ex::Call(f.into(), args));
            }
            ConvertToType => {
                let f = match i.modifier {
                    2 => "cint",
                    3 => "clng",
                    4 => "csng",
                    5 => "cdbl",
                    6 => "ccur",
                    7 => "cdate",
                    8 => "cstr",
                    11 => "cbool",
                    17 => "cbyte",
                    _ => return None,
                };
                un1(&mut st, f)?;
            }
            _ => break,
        }
        k += 1;
    }
    Some((st, k))
}

pub fn name_of(op: &Operand) -> Option<String> {
    match op {
        Operand::Name { name, .. } => match name {
            Name::User(_) | Name::Builtin(_) | Name::Me => {
                name.spelling().map(|s| s.to_lowercase())
            }
            _ => None,
        },
        _ => None,
    }
}

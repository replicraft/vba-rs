//! Conditional compilation (MS-VBAL §4): directive discovery and evaluation.
//!
//! A pre-pass over the full token stream (trivia included) finds `#If`,
//! `#ElseIf`, `#Else`, `#End If` and `#Const` lines, evaluates conditions
//! with the profile's predefined constants plus `#Const` definitions, and
//! records which token ranges are inactive. The parser then emits directive
//! nodes for *effective* directives (those not nested in an inactive
//! branch) and wraps each inactive range in one `InactiveText` node without
//! parsing it. Directives nested inside inactive text are part of that
//! text; their nesting is still tracked so that `#End If` pairs correctly.

use std::collections::{HashMap, HashSet};

use vba_diagnostics::{Code, Diagnostic, Severity};
use vba_lexer::{Keyword, Token, TokenKind};
use vba_profile::Profile;
use vba_source::{FileId, Span};

use crate::prec::binary_bp;

pub(crate) const CC_UNMATCHED: Code = Code("SYN040");
pub(crate) const CC_MISSING_END: Code = Code("SYN041");
pub(crate) const CC_BAD_EXPR: Code = Code("SYN042");

/// Result of the pre-pass.
#[derive(Debug, Default)]
pub(crate) struct CcInfo {
    /// Effective directives by raw token index of the directive token,
    /// with the evaluated condition (`#If` / `#ElseIf`) when available.
    pub directives: HashMap<usize, Option<bool>>,
    /// Inactive raw token ranges keyed by start index; value is the
    /// exclusive end index.
    pub regions: HashMap<usize, usize>,
    pub diagnostics: Vec<Diagnostic>,
    /// Names defined by `#Const` in this module (lower case, no suffix).
    defined: HashSet<Vec<u8>>,
    /// Spec-strict rules are errors.
    strict_spec: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Empty,
    Int(i64),
    Float(f64),
    Str(Vec<u8>),
}

struct Frame {
    parent_active: bool,
    taken: bool,
    active: bool,
    seen_else: bool,
    start: Span,
}

pub(crate) fn is_directive(k: TokenKind) -> bool {
    matches!(
        k,
        TokenKind::HashIf
            | TokenKind::HashElseIf
            | TokenKind::HashElse
            | TokenKind::HashEndIf
            | TokenKind::HashConst
    )
}

/// Runs the pre-pass. `extra` are additional project-level constants.
pub(crate) fn analyze(
    file: FileId,
    toks: &[Token<'_>],
    profile: &Profile,
    extra: &[(String, i64)],
    strict_spec: bool,
) -> CcInfo {
    let mut info = CcInfo {
        strict_spec,
        ..CcInfo::default()
    };
    let mut consts: HashMap<Vec<u8>, Value> = HashMap::new();
    for (n, v) in extra {
        consts.insert(n.to_ascii_lowercase().into_bytes(), Value::Int(*v));
    }
    let mut stack: Vec<Frame> = Vec::new();
    let mut open_region: Option<usize> = None;
    let mut at_line_start = true;
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i];
        if t.kind == TokenKind::Newline {
            at_line_start = true;
            i += 1;
            continue;
        }
        if t.kind.is_trivia() {
            i += 1;
            continue;
        }
        let was_line_start = at_line_start;
        at_line_start = false;
        if !(was_line_start && is_directive(t.kind)) {
            i += 1;
            continue;
        }
        // Directive line: significant tokens up to the newline.
        let mut end = i + 1;
        while end < toks.len() && toks[end].kind != TokenKind::Newline {
            end += 1;
        }
        let line: Vec<Token<'_>> = toks[i + 1..end]
            .iter()
            .copied()
            .filter(|t| !t.kind.is_trivia())
            .collect();
        let cur_active = stack.last().is_none_or(|f| f.active);
        let mut effective = false;
        let mut value = None;
        match t.kind {
            TokenKind::HashIf => {
                effective = cur_active;
                // 3.4.2: every condition must conform, even in excluded blocks.
                let c = eval_cond(
                    &mut info.diagnostics,
                    file,
                    t.span,
                    &line,
                    &consts,
                    profile,
                    !cur_active && !strict_spec,
                );
                let v = if effective {
                    value = Some(c);
                    c
                } else {
                    false
                };
                stack.push(Frame {
                    parent_active: effective,
                    taken: v,
                    active: effective && v,
                    seen_else: false,
                    start: t.span,
                });
            }
            TokenKind::HashElseIf => match stack.last_mut() {
                None => unmatched(&mut info, file, t.span, "`#ElseIf` without `#If`"),
                Some(f) => {
                    effective = f.parent_active;
                    if f.seen_else {
                        unmatched(&mut info, file, t.span, "`#ElseIf` after `#Else`");
                    }
                    let v = eval_cond(
                        &mut info.diagnostics,
                        file,
                        t.span,
                        &line,
                        &consts,
                        profile,
                        !f.parent_active && !strict_spec,
                    );
                    if f.parent_active && !f.taken {
                        value = Some(v);
                        f.active = v;
                        f.taken = v;
                    } else {
                        f.active = false;
                    }
                }
            },
            TokenKind::HashElse => match stack.last_mut() {
                None => unmatched(&mut info, file, t.span, "`#Else` without `#If`"),
                Some(f) => {
                    effective = f.parent_active;
                    if f.seen_else {
                        unmatched(&mut info, file, t.span, "duplicate `#Else`");
                    }
                    f.seen_else = true;
                    f.active = f.parent_active && !f.taken;
                    f.taken = true;
                }
            },
            TokenKind::HashEndIf => match stack.pop() {
                None => unmatched(&mut info, file, t.span, "`#End If` without `#If`"),
                Some(f) => effective = f.parent_active,
            },
            TokenKind::HashConst => {
                effective = cur_active;
                define_const(
                    &mut info,
                    &mut consts,
                    file,
                    t.span,
                    &line,
                    profile,
                    effective,
                );
            }
            _ => unreachable!(),
        }
        if effective {
            if let Some(start) = open_region.take()
                && start < i
            {
                info.regions.insert(start, i);
            }
            info.directives.insert(i, value);
            let active = stack.last().is_none_or(|f| f.active);
            if !active {
                open_region = Some((end + 1).min(toks.len()));
            }
        }
        i = end;
    }
    if let Some(start) = open_region
        && start < toks.len()
    {
        info.regions.insert(start, toks.len());
    }
    for f in stack {
        info.diagnostics.push(Diagnostic::error(
            CC_MISSING_END,
            file,
            f.start,
            "`#If` without matching `#End If`",
        ));
    }
    info
}

fn eval_cond(
    diags: &mut Vec<Diagnostic>,
    file: FileId,
    at: Span,
    line: &[Token<'_>],
    consts: &HashMap<Vec<u8>, Value>,
    profile: &Profile,
    lenient: bool,
) -> bool {
    // `lenient`: the block is excluded and the rule is a spec-strict one.
    let body = match line.last() {
        Some(l) if l.kind == TokenKind::Keyword(Keyword::Then) => &line[..line.len() - 1],
        _ => {
            diags.push(Diagnostic::error(
                CC_BAD_EXPR,
                file,
                at,
                "expected `Then` at the end of the directive",
            ));
            line
        }
    };
    match eval_expr(body, consts, profile).and_then(|v| as_boolean(&v)) {
        Ok(b) => b,
        Err(m) => {
            let sp = match (body.first(), body.last()) {
                (Some(f), Some(l)) => Span::new(f.span.start, l.span.end),
                _ => at,
            };
            diags.push(if lenient {
                Diagnostic::warning(CC_BAD_EXPR, file, sp, m)
            } else {
                Diagnostic::error(CC_BAD_EXPR, file, sp, m)
            });
            false
        }
    }
}

fn unmatched(info: &mut CcInfo, file: FileId, span: Span, msg: &str) {
    info.diagnostics
        .push(Diagnostic::error(CC_UNMATCHED, file, span, msg));
}

fn define_const(
    info: &mut CcInfo,
    consts: &mut HashMap<Vec<u8>, Value>,
    file: FileId,
    at: Span,
    line: &[Token<'_>],
    profile: &Profile,
    report: bool,
) {
    let strict_spec = info.strict_spec;
    let ok = line.len() >= 3 && line[0].kind == TokenKind::Ident && line[1].kind == TokenKind::Eq;
    if !ok {
        info.diagnostics.push(Diagnostic::error(
            CC_BAD_EXPR,
            file,
            at,
            "expected `#Const name = expression`",
        ));
        return;
    }
    // The type suffix of the name is ignored (3.4.1).
    let name = strip_suffix(line[0].text).to_ascii_lowercase();
    // Names are distinct within a module, including those in excluded blocks.
    if !info.defined.insert(name.clone()) {
        // Real VBA lets a later `#Const` redefine an earlier one.
        let sev = if strict_spec {
            Severity::Error
        } else {
            Severity::Warning
        };
        info.diagnostics.push(Diagnostic::new(
            sev,
            CC_BAD_EXPR,
            file,
            line[0].span,
            "duplicate `#Const` name",
        ));
    }
    match eval_expr(&line[2..], consts, profile) {
        Ok(v) => {
            consts.insert(name, v);
        }
        Err(m) => {
            if report {
                let sp = Span::new(line[2].span.start, line[line.len() - 1].span.end);
                info.diagnostics
                    .push(Diagnostic::error(CC_BAD_EXPR, file, sp, m));
            }
            consts.insert(name, Value::Empty);
        }
    }
}

/// Let-coercion to Boolean (3.4.2): strings coerce only when they spell
/// `True`/`False` or a number.
fn as_boolean(v: &Value) -> Result<bool, String> {
    match v {
        Value::Empty => Ok(false),
        Value::Int(i) => Ok(*i != 0),
        Value::Float(f) => Ok(*f != 0.0),
        Value::Str(s) => {
            let t = String::from_utf8_lossy(s).trim().to_ascii_lowercase();
            match t.as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => t.parse::<f64>().map(|f| f != 0.0).map_err(|_| {
                    "conditional compilation expression is not coercible to Boolean".to_owned()
                }),
            }
        }
    }
}

fn to_num(v: &Value) -> Result<f64, String> {
    match v {
        Value::Empty => Ok(0.0),
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        Value::Str(_) => Err("type mismatch in conditional compilation expression".into()),
    }
}

fn to_int(v: &Value) -> Result<i64, String> {
    match v {
        Value::Empty => Ok(0),
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(f.round_ties_even() as i64),
        Value::Str(_) => Err("type mismatch in conditional compilation expression".into()),
    }
}

fn bool_val(b: bool) -> Value {
    Value::Int(if b { -1 } else { 0 })
}

struct Eval<'a, 'b> {
    toks: &'a [Token<'b>],
    pos: usize,
    consts: &'a HashMap<Vec<u8>, Value>,
    profile: &'a Profile,
}

fn eval_expr(
    toks: &[Token<'_>],
    consts: &HashMap<Vec<u8>, Value>,
    profile: &Profile,
) -> Result<Value, String> {
    if toks.is_empty() {
        return Err("expected a conditional compilation expression".into());
    }
    let mut e = Eval {
        toks,
        pos: 0,
        consts,
        profile,
    };
    let v = e.expr(0)?;
    if e.pos != toks.len() {
        return Err("unexpected token in conditional compilation expression".into());
    }
    Ok(v)
}

impl Eval<'_, '_> {
    fn peek(&self) -> Option<TokenKind> {
        self.toks.get(self.pos).map(|t| t.kind)
    }

    fn expr(&mut self, min: u8) -> Result<Value, String> {
        let mut lhs = self.prefix()?;
        while let Some(k) = self.peek() {
            let Some(bp) = binary_bp(k) else { break };
            if bp < min {
                break;
            }
            self.pos += 1;
            let rhs = self.expr(bp + 1)?;
            lhs = binop(k, lhs, rhs)?;
        }
        Ok(lhs)
    }

    fn prefix(&mut self) -> Result<Value, String> {
        let Some(t) = self.toks.get(self.pos).copied() else {
            return Err("expected an operand".into());
        };
        self.pos += 1;
        match t.kind {
            TokenKind::Minus => {
                let v = self.expr(14)?;
                match v {
                    Value::Float(f) => Ok(Value::Float(-f)),
                    other => Ok(Value::Int(to_int(&other)?.wrapping_neg())),
                }
            }
            TokenKind::Plus => self.expr(14),
            TokenKind::Keyword(Keyword::Not) => {
                let v = self.expr(7)?;
                Ok(Value::Int(!to_int(&v)?))
            }
            TokenKind::LParen => {
                let v = self.expr(0)?;
                if self.peek() != Some(TokenKind::RParen) {
                    return Err("expected `)`".into());
                }
                self.pos += 1;
                Ok(v)
            }
            TokenKind::IntegerLiteral | TokenKind::HexLiteral | TokenKind::OctalLiteral => {
                parse_int(t).map(Value::Int)
            }
            TokenKind::FloatLiteral => parse_float(t.text).map(Value::Float),
            TokenKind::StringLiteral => {
                let inner = &t.text[1..t.text.len() - 1];
                let mut out = Vec::new();
                let mut i = 0;
                while i < inner.len() {
                    out.push(inner[i]);
                    if inner[i] == b'"' {
                        i += 1;
                    }
                    i += 1;
                }
                Ok(Value::Str(out))
            }
            TokenKind::Keyword(Keyword::True) => Ok(Value::Int(-1)),
            TokenKind::Keyword(Keyword::False) => Ok(Value::Int(0)),
            TokenKind::Keyword(Keyword::Empty) | TokenKind::Keyword(Keyword::Null) => {
                Ok(Value::Empty)
            }
            TokenKind::Ident => {
                let name = strip_suffix(t.text).to_ascii_lowercase();
                if let Some(v) = self.consts.get(&name) {
                    return Ok(v.clone());
                }
                // Undefined names, predefined or not, evaluate to Empty
                // (false) when the profile does not define them.
                Ok(match self.profile.predefined_constant(t.text) {
                    Some(b) => bool_val(b),
                    None => Value::Empty,
                })
            }
            _ => Err("unexpected token in conditional compilation expression".into()),
        }
    }
}

fn strip_suffix(text: &[u8]) -> &[u8] {
    match text.last() {
        Some(b'%' | b'&' | b'^' | b'!' | b'#' | b'@' | b'$') => &text[..text.len() - 1],
        _ => text,
    }
}

fn parse_int(t: Token<'_>) -> Result<i64, String> {
    let text = strip_suffix(t.text);
    let s = std::str::from_utf8(text).map_err(|_| "bad number")?;
    let r = match t.kind {
        TokenKind::HexLiteral => i64::from_str_radix(&s[2..], 16),
        TokenKind::OctalLiteral => {
            let d = if s.len() > 1 && s[1..2].eq_ignore_ascii_case("o") {
                &s[2..]
            } else {
                &s[1..]
            };
            i64::from_str_radix(d, 8)
        }
        _ => s.parse::<i64>(),
    };
    r.map_err(|_| "integer literal out of range".to_string())
}

fn parse_float(text: &[u8]) -> Result<f64, String> {
    let s: String = String::from_utf8_lossy(strip_suffix(text))
        .chars()
        .map(|c| if c == 'd' || c == 'D' { 'e' } else { c })
        .collect();
    s.parse::<f64>()
        .map_err(|_| "bad floating point literal".to_string())
}

fn binop(op: TokenKind, a: Value, b: Value) -> Result<Value, String> {
    use TokenKind as T;
    let cmp = |a: &Value, b: &Value| -> Result<std::cmp::Ordering, String> {
        match (a, b) {
            (Value::Str(x), Value::Str(y)) => Ok(x.cmp(y)),
            _ => to_num(a)?
                .partial_cmp(&to_num(b)?)
                .ok_or_else(|| "invalid comparison".to_string()),
        }
    };
    let both_int = |a: &Value, b: &Value| {
        matches!(
            (a, b),
            (Value::Int(_) | Value::Empty, Value::Int(_) | Value::Empty)
        )
    };
    Ok(match op {
        T::Plus | T::Minus | T::Star => {
            if both_int(&a, &b) {
                let (x, y) = (to_int(&a)?, to_int(&b)?);
                Value::Int(match op {
                    T::Plus => x.wrapping_add(y),
                    T::Minus => x.wrapping_sub(y),
                    _ => x.wrapping_mul(y),
                })
            } else {
                let (x, y) = (to_num(&a)?, to_num(&b)?);
                Value::Float(match op {
                    T::Plus => x + y,
                    T::Minus => x - y,
                    _ => x * y,
                })
            }
        }
        T::Slash => {
            let y = to_num(&b)?;
            if y == 0.0 {
                return Err("division by zero".into());
            }
            Value::Float(to_num(&a)? / y)
        }
        T::Backslash => {
            let y = to_int(&b)?;
            if y == 0 {
                return Err("division by zero".into());
            }
            Value::Int(to_int(&a)?.wrapping_div(y))
        }
        T::Keyword(Keyword::Mod) => {
            let y = to_int(&b)?;
            if y == 0 {
                return Err("division by zero".into());
            }
            Value::Int(to_int(&a)?.wrapping_rem(y))
        }
        T::Caret => Value::Float(to_num(&a)?.powf(to_num(&b)?)),
        T::Amp => {
            let s = |v: &Value| match v {
                Value::Str(s) => s.clone(),
                Value::Empty => Vec::new(),
                Value::Int(i) => i.to_string().into_bytes(),
                Value::Float(f) => f.to_string().into_bytes(),
            };
            let mut x = s(&a);
            x.extend(s(&b));
            Value::Str(x)
        }
        T::Eq => bool_val(cmp(&a, &b)?.is_eq()),
        T::Ne => bool_val(cmp(&a, &b)?.is_ne()),
        T::Lt => bool_val(cmp(&a, &b)?.is_lt()),
        T::Gt => bool_val(cmp(&a, &b)?.is_gt()),
        T::Le => bool_val(cmp(&a, &b)?.is_le()),
        T::Ge => bool_val(cmp(&a, &b)?.is_ge()),
        T::Keyword(Keyword::And) => Value::Int(to_int(&a)? & to_int(&b)?),
        T::Keyword(Keyword::Or) => Value::Int(to_int(&a)? | to_int(&b)?),
        T::Keyword(Keyword::Xor) => Value::Int(to_int(&a)? ^ to_int(&b)?),
        T::Keyword(Keyword::Eqv) => Value::Int(!(to_int(&a)? ^ to_int(&b)?)),
        T::Keyword(Keyword::Imp) => Value::Int(!to_int(&a)? | to_int(&b)?),
        _ => return Err("operator not allowed in conditional compilation".into()),
    })
}

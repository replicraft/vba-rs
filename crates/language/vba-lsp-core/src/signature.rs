//! Signature help for in-file procedures and standard-library functions.

use vba_lexer::{Keyword, KeywordClass, TokenKind};
use vba_syntax::SyntaxToken;

use crate::builtin::{std_doc, std_signature};
use crate::scope::SymKind;
use crate::text::Position;
use crate::types::{ParameterInfo, SignatureHelp, SignatureInfo};
use crate::util::{normalize, qualified_name};
use crate::workspace::{View, is_word};

/// A call found around the cursor.
struct CallSite {
    callee: usize,
    /// Index of the argument the cursor is in.
    arg: u32,
    /// Name given with `name:=` for the argument at the cursor.
    named: Option<String>,
}

fn find_call(tokens: &[SyntaxToken<'_>], offset: u32) -> Option<CallSite> {
    let end = tokens.partition_point(|t| t.span().start < offset);
    if end > 0 {
        let last = tokens[end - 1];
        if last.span().end > offset
            && matches!(
                last.kind(),
                TokenKind::Comment
                    | TokenKind::RemComment
                    | TokenKind::StringLiteral
                    | TokenKind::DateLiteral
            )
        {
            return None;
        }
    }
    let mut depth = 0i32;
    let mut commas = 0u32;
    let mut j = end;
    let mut open = None;
    // tokens after the last comma at depth 0 (for `name:=`)
    let mut arg_start = end;
    while j > 0 {
        j -= 1;
        let t = tokens[j];
        match t.kind() {
            TokenKind::RParen => depth += 1,
            TokenKind::LParen => {
                if depth == 0 {
                    open = Some(j);
                    break;
                }
                depth -= 1;
            }
            TokenKind::Comma if depth == 0 => {
                if commas == 0 {
                    arg_start = j + 1;
                }
                commas += 1;
            }
            TokenKind::Newline | TokenKind::Colon => {
                j += 1;
                break;
            }
            _ => {}
        }
    }
    let named_in = |from: usize| -> Option<String> {
        let sig: Vec<&SyntaxToken<'_>> = tokens[from..end]
            .iter()
            .filter(|t| !t.kind().is_trivia())
            .collect();
        match sig.as_slice() {
            [a, b, ..] if is_word(a.kind()) && b.kind() == TokenKind::ColonEq => {
                Some(normalize(a.text()))
            }
            _ => None,
        }
    };
    if let Some(o) = open {
        let prev = (0..o).rev().find(|&k| !tokens[k].kind().is_trivia())?;
        if !is_word(tokens[prev].kind()) {
            return None;
        }
        let from = if commas == 0 { o + 1 } else { arg_start };
        return Some(CallSite {
            callee: prev,
            arg: commas,
            named: named_in(from),
        });
    }
    // Implicit call `Name a, b`: `j` is the first token of the statement.
    let sig: Vec<usize> = (j..end)
        .filter(|&k| !tokens[k].kind().is_trivia())
        .collect();
    let mut c = *sig.first()?;
    let first = tokens[c];
    let callable = match first.kind() {
        TokenKind::Ident => true,
        TokenKind::Keyword(k) => {
            matches!(
                k.class(),
                KeywordClass::ReservedName | KeywordClass::SpecialForm
            ) && k != Keyword::Me
        }
        _ => false,
    };
    if !callable {
        return None;
    }
    let mut p = 1;
    while p + 1 < sig.len()
        && tokens[sig[p]].kind() == TokenKind::Dot
        && is_word(tokens[sig[p + 1]].kind())
    {
        c = sig[p + 1];
        p += 2;
    }
    // Arguments must follow the name, and must not start like an assignment
    // or member access.
    let next = sig.get(p).map(|&k| tokens[k].kind())?;
    if matches!(
        next,
        TokenKind::Eq | TokenKind::Dot | TokenKind::LParen | TokenKind::Comma | TokenKind::Colon
    ) {
        return None;
    }
    let from = if commas == 0 { sig[p] } else { arg_start };
    Some(CallSite {
        callee: c,
        arg: commas,
        named: named_in(from),
    })
}

impl View<'_> {
    /// Signature help for the call around the cursor. Resolves undotted
    /// names to procedures declared in this file and `VBA.`-qualified or
    /// plain names to standard-library functions; members of objects are
    /// not resolved.
    pub fn signature_help(&self, pos: Position) -> Option<SignatureHelp> {
        let offset = self.offset(pos);
        let tokens = self.tokens();
        let site = find_call(&tokens, offset)?;
        let callee = tokens[site.callee];
        let (name, leading_dot) = qualified_name(&tokens, site.callee);
        if leading_dot {
            return None;
        }
        if name.contains('.')
            && let Some(h) =
                self.lib_signature(&tokens, site.callee, site.arg, site.named.as_deref())
        {
            return Some(h);
        }

        let scopes = self.scopes();
        if !name.contains('.')
            && let Some(sym) = scopes.lookup_at(callee.span().start, &normalize(name.as_bytes()))
        {
            let s = &scopes.symbols[sym as usize];
            if !matches!(s.kind, SymKind::Proc(_) | SymKind::Declare | SymKind::Event) {
                return None;
            }
            let mut signatures = Vec::new();
            let mut active = None;
            for sig in &s.sigs {
                let mut label = format!("{} {}(", sig.keyword, s.name);
                let mut params = Vec::new();
                for (i, p) in sig.params.iter().enumerate() {
                    if i > 0 {
                        label.push_str(", ");
                    }
                    let start = label.encode_utf16().count() as u32;
                    label.push_str(&p.text);
                    params.push(ParameterInfo {
                        start,
                        end: label.encode_utf16().count() as u32,
                    });
                }
                label.push(')');
                if let Some(r) = &sig.ret {
                    label.push(' ');
                    label.push_str(r);
                }
                let idx = match &site.named {
                    Some(n) => sig
                        .params
                        .iter()
                        .position(|p| &p.name == n)
                        .map(|i| i as u32),
                    None => Some(site.arg),
                };
                let last_is_array = sig
                    .params
                    .last()
                    .is_some_and(|p| p.text.to_ascii_lowercase().contains("paramarray"));
                let idx = idx.and_then(|i| {
                    if (i as usize) < sig.params.len() {
                        Some(i)
                    } else if last_is_array {
                        Some(sig.params.len() as u32 - 1)
                    } else {
                        None
                    }
                });
                active = active.or(idx);
                signatures.push(SignatureInfo {
                    label,
                    documentation: Some("Declared in this file.".into()),
                    parameters: params,
                });
            }
            return Some(SignatureHelp {
                signatures,
                active_parameter: active,
            });
        }

        let info = vba_stdlib::lookup(&name)?.info;
        let sig = std_signature(info);
        let n = info.params.len();
        let idx = match &site.named {
            Some(nm) => info
                .params
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(nm))
                .map(|i| i as u32),
            None => Some(site.arg),
        };
        let active = idx.and_then(|i| {
            if (i as usize) < n {
                Some(i)
            } else if info.params.last().is_some_and(|p| p.param_array) {
                Some(n as u32 - 1)
            } else {
                None
            }
        });
        Some(SignatureHelp {
            signatures: vec![SignatureInfo {
                label: sig.label,
                documentation: Some(std_doc(info)),
                parameters: sig
                    .params
                    .iter()
                    .map(|&(start, end)| ParameterInfo { start, end })
                    .collect(),
            }],
            active_parameter: active,
        })
    }
}

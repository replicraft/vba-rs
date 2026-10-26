//! Go to definition, find references and rename over the syntactic scope
//! index (see [`crate::scope`] for what that does and does not resolve).

use vba_lexer::{LexOptions, TokenKind, lex};
use vba_source::Span;

use crate::scope::{Scope, SymKind};
use crate::text::{Position, Range};
use crate::types::{Error, TextEdit};
use crate::workspace::{View, is_word, token_index_at};

impl View<'_> {
    fn symbol_at(&self, pos: Position) -> Option<(u32, Span)> {
        let off = self.offset(pos);
        let o = self.scopes().occurrence_at(off)?;
        Some((o.symbol, o.span))
    }

    /// Declaration name ranges of the symbol under the cursor (several for
    /// `Property Get/Let/Set` pairs). Empty when the name does not resolve
    /// to a declaration in this file.
    pub fn definition(&self, pos: Position) -> Vec<Range> {
        let Some((sym, _)) = self.symbol_at(pos) else {
            return Vec::new();
        };
        self.scopes().symbols[sym as usize]
            .decls
            .iter()
            .map(|&s| self.range(s))
            .collect()
    }

    /// All uses of the symbol under the cursor in this file, in source
    /// order. Declarations are included when `include_declaration` is set.
    pub fn references(&self, pos: Position, include_declaration: bool) -> Vec<Range> {
        let Some((sym, _)) = self.symbol_at(pos) else {
            return Vec::new();
        };
        self.scopes()
            .occurrences_of(sym)
            .filter(|o| include_declaration || !o.is_decl)
            .map(|o| self.range(o.span))
            .collect()
    }

    /// Checks that the cursor is on a renameable name and returns its range.
    pub fn prepare_rename(&self, pos: Position) -> Result<Range, Error> {
        if let Some((_, span)) = self.symbol_at(pos) {
            return Ok(self.range(span));
        }
        let tokens = self.tokens();
        let Some(i) = token_index_at(&tokens, self.offset(pos)) else {
            return Err(Error::Invalid("nothing to rename here".into()));
        };
        let t = tokens[i];
        if !is_word(t.kind()) {
            return Err(Error::Invalid("nothing to rename here".into()));
        }
        if matches!(t.kind(), TokenKind::Keyword(_)) {
            return Err(Error::Invalid(
                "reserved words cannot be renamed".to_string(),
            ));
        }
        let prev = tokens[..i]
            .iter()
            .rev()
            .find(|t| !t.kind().is_trivia())
            .map(|t| t.kind());
        if matches!(prev, Some(TokenKind::Dot | TokenKind::Bang)) {
            return Err(Error::Unsupported(
                "renaming members needs type information that arrives with semantic analysis \
                 (vba-sema); only names declared in this file can be renamed"
                    .into(),
            ));
        }
        if vba_stdlib::lookup(&String::from_utf8_lossy(t.text())).is_some() {
            return Err(Error::Unsupported(
                "library functions cannot be renamed".into(),
            ));
        }
        Err(Error::Invalid(
            "this name is not declared in this file".into(),
        ))
    }

    /// Renames the symbol under the cursor: one edit per declaration and
    /// use in this file. Fails when the new name is not a plain identifier
    /// or would clash with, or capture, another name.
    pub fn rename(&self, pos: Position, new_name: &str) -> Result<Vec<TextEdit>, Error> {
        self.prepare_rename(pos)?;
        let (sym, _) = self
            .symbol_at(pos)
            .ok_or_else(|| Error::Invalid("nothing to rename here".into()))?;
        self.check_new_name(new_name)?;
        let idx = self.scopes();
        let target = &idx.symbols[sym as usize];
        let n = new_name.to_ascii_lowercase();
        let is_label = target.kind == SymKind::Label;
        let clash = |what: &str| Error::Invalid(format!("`{new_name}` {what}"));

        for (i, s) in idx.symbols.iter().enumerate() {
            if i as u32 == sym || s.norm != n || (s.kind == SymKind::Label) != is_label {
                continue;
            }
            if s.scope == target.scope {
                return Err(clash("is already declared in the same scope"));
            }
            // Module-level target: a local of the same name in a procedure
            // that uses the target would shadow it. Local target: a module
            // symbol used in the same procedure would be shadowed.
            let conflict = match (target.scope, s.scope) {
                (Scope::Module, Scope::Proc(p)) => idx
                    .occurrences_of(sym)
                    .any(|o| idx.proc_at(o.span.start) == Some(p)),
                (Scope::Proc(p), Scope::Module) => idx
                    .occurrences_of(i as u32)
                    .any(|o| idx.proc_at(o.span.start) == Some(p)),
                _ => false,
            };
            if conflict {
                return Err(clash("would change the meaning of existing references"));
            }
        }
        Ok(idx
            .occurrences_of(sym)
            .map(|o| TextEdit {
                range: self.range(o.span),
                new_text: new_name.to_string(),
            })
            .collect())
    }

    fn check_new_name(&self, name: &str) -> Result<(), Error> {
        let bytes = name.as_bytes();
        let out = lex(
            crate::document::FILE_ID,
            bytes,
            LexOptions::for_profile(self.profile()),
        );
        let ok = bytes.len() <= 255
            && out.tokens.len() == 1
            && out.tokens[0].kind == TokenKind::Ident
            && out.tokens[0].type_suffix().is_none()
            && out.tokens[0].span.end as usize == bytes.len()
            && bytes[0].is_ascii_alphabetic();
        if ok {
            Ok(())
        } else {
            Err(Error::Invalid(format!(
                "`{name}` is not a valid VBA identifier (letters, digits and underscores, \
                 starting with a letter, not a reserved word)"
            )))
        }
    }
}

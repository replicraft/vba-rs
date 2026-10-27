//! Project-wide features on top of the `vba-sema` analysis: definition,
//! references and rename across modules, workspace symbols, and the
//! semantic parts of hover and completion.
//!
//! A definite binding (see [`crate::resolve`]) is answered from the project.
//! A name the project does not know, or a member access on a `Variant` or
//! `Object`, falls back to the syntactic walk of the document, and a
//! late-bound member is never renamed.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use vba_lexer::{Keyword, TokenKind};
use vba_project::ModuleKind;
use vba_sema::{LibMember, LibraryType, ModuleId, Symbol, Type};
use vba_source::Span;
use vba_syntax::{NodeKind, SyntaxToken};

use crate::document::Document;
use crate::navigation::check_new_name;
use crate::project::{Decl, Project};
use crate::resolve::{Binding, Ctx, Ty};
use crate::text::{Position, Range};
use crate::types::{
    CompletionItem, CompletionKind, Error, FileEdit, Hover, Location, SymbolKind, TextEdit,
    WorkspaceSymbol,
};
use crate::util::normalize;
use crate::workspace::{View, Workspace, token_index_at};

/// The project, the module of a document and its tolerant parse.
struct Sem {
    proj: Arc<Project>,
    m: usize,
}

fn word_token(t: &SyntaxToken<'_>) -> bool {
    matches!(
        t.kind(),
        TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
    )
}

impl Workspace {
    fn sem(&self, uri: &str) -> Option<Sem> {
        let proj = self.project()?;
        let m = *proj.by_uri.get(uri)?;
        Some(Sem { proj, m })
    }

    fn range_in(&self, uri: &str, span: Span) -> Range {
        match self.docs().get(uri) {
            Some(d) => d.text_map().range(d.bytes(), span),
            None => Range::new(Position::new(0, 0), Position::new(0, 0)),
        }
    }

    fn location(&self, proj: &Project, m: usize, span: Span) -> Location {
        let uri = proj.mods[m].uri.clone();
        let range = self.range_in(&uri, span);
        Location { uri, range }
    }

    fn decl_locations(&self, proj: &Project, d: &Decl) -> Vec<Location> {
        if let Decl::Module(m) = d {
            return vec![self.location(proj, m.index(), Span::empty(0))];
        }
        proj.locations(d)
            .into_iter()
            .map(|(m, s)| self.location(proj, m, s))
            .collect()
    }

    /// The binding of the word under `pos` and the token's range.
    fn binding_at(&self, uri: &str, pos: Position) -> Result<(Option<Sem>, Binding, bool), Error> {
        let view = self.view(uri)?;
        let Some(sem) = self.sem(uri) else {
            return Ok((None, Binding::Unknown, false));
        };
        let tokens = view.tokens();
        let off = view.offset(pos);
        let Some(i) = token_index_at(&tokens, off) else {
            return Ok((Some(sem), Binding::Unknown, false));
        };
        let t = tokens[i];
        let is_word = word_token(&t);
        let b = Ctx::new(&sem.proj, sem.m).binding(&t);
        Ok((Some(sem), b, is_word))
    }

    /// Every occurrence (declaration or use) of a declaration in the
    /// workspace, in document order.
    fn occurrences(&self, proj: &Project, d: &Decl) -> Vec<(usize, Span)> {
        if matches!(d, Decl::Module(_)) {
            // Uses of a module name only; the declaration is the file.
        }
        let norm = normalize(proj.decl_name(d).as_bytes());
        let only = match d {
            Decl::Local(p, _) => Some(proj.sema.program.proc(*p).module.index()),
            _ => None,
        };
        let mut out = Vec::new();
        for (mi, pm) in proj.mods.iter().enumerate() {
            if only.is_some_and(|o| o != mi) {
                continue;
            }
            let cands: Vec<Span> = pm
                .idents
                .iter()
                .filter(|(_, n)| *n == norm)
                .map(|(s, _)| *s)
                .collect();
            if cands.is_empty() {
                continue;
            }
            let Some(doc) = self.docs().get(&pm.uri) else {
                continue;
            };
            let an = doc.analysis(self.config());
            let tokens = an.tree().root().descendant_tokens();
            let ctx = Ctx::new(proj, mi);
            for s in cands {
                let i = tokens.partition_point(|t| t.span().start < s.start);
                let Some(t) = tokens.get(i).filter(|t| t.span().start == s.start) else {
                    continue;
                };
                if ctx.binding(t) == Binding::Decl(d.clone()) {
                    out.push((mi, s));
                }
            }
        }
        out
    }

    /// Definition locations of the name under the cursor. A name bound by
    /// the project is answered across modules; anything else falls back to
    /// the syntactic walk of the document.
    pub fn definition(&self, uri: &str, pos: Position) -> Result<Vec<Location>, Error> {
        let (sem, b, _) = self.binding_at(uri, pos)?;
        if let (Some(sem), Binding::Decl(d)) = (&sem, &b) {
            let locs = self.decl_locations(&sem.proj, d);
            if !locs.is_empty() {
                return Ok(locs);
            }
        }
        Ok(self
            .view(uri)?
            .definition(pos)
            .into_iter()
            .map(|range| Location {
                uri: uri.to_string(),
                range,
            })
            .collect())
    }

    /// All uses of the name under the cursor in the workspace, ordered by
    /// document URI and position.
    pub fn references(
        &self,
        uri: &str,
        pos: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, Error> {
        let (sem, b, _) = self.binding_at(uri, pos)?;
        if let (Some(sem), Binding::Decl(d)) = (&sem, &b) {
            let decl_spans = sem.proj.locations(d);
            let mut occ = self.occurrences(&sem.proj, d);
            occ.retain(|o| include_declaration || !decl_spans.contains(o));
            let mut out: Vec<Location> = occ
                .into_iter()
                .map(|(m, s)| self.location(&sem.proj, m, s))
                .collect();
            out.sort_by(|a, b| {
                (&a.uri, a.range.start, a.range.end).cmp(&(&b.uri, b.range.start, b.range.end))
            });
            out.dedup();
            return Ok(out);
        }
        Ok(self
            .view(uri)?
            .references(pos, include_declaration)
            .into_iter()
            .map(|range| Location {
                uri: uri.to_string(),
                range,
            })
            .collect())
    }

    /// Checks that the cursor is on a renameable name and returns its range.
    pub fn prepare_rename(&self, uri: &str, pos: Position) -> Result<Range, Error> {
        let (sem, b, _) = self.binding_at(uri, pos)?;
        let view = self.view(uri)?;
        if sem.is_some() {
            match &b {
                Binding::Decl(Decl::Module(_)) => {
                    return Err(Error::Unsupported(
                        "modules cannot be renamed here; change the `Attribute VB_Name` and the \
                         file name"
                            .into(),
                    ));
                }
                Binding::Decl(_) => {
                    let tokens = view.tokens();
                    let i = token_index_at(&tokens, view.offset(pos))
                        .ok_or_else(|| Error::Invalid("nothing to rename here".into()))?;
                    return Ok(view.range(tokens[i].span()));
                }
                Binding::Late => {
                    return Err(Error::Unsupported(
                        "this member is accessed late bound (the receiver is a Variant or \
                         Object), so its declaration is unknown and it is not renamed"
                            .into(),
                    ));
                }
                Binding::Unknown => {}
            }
        }
        view.prepare_rename(pos)
    }

    /// Renames the declaration under the cursor in every document that uses
    /// it. Names that the project does not bind fall back to the syntactic,
    /// single-document rename; late-bound members are refused.
    pub fn rename(&self, uri: &str, pos: Position, new_name: &str) -> Result<Vec<FileEdit>, Error> {
        self.prepare_rename(uri, pos)?;
        let (sem, b, _) = self.binding_at(uri, pos)?;
        let (Some(sem), Binding::Decl(d)) = (sem, b) else {
            let edits = self.view(uri)?.rename(pos, new_name)?;
            if let Some(proj) = self.project()
                && let Some(&m) = proj.by_uri.get(uri)
                && let Some(doc) = self.docs().get(uri)
            {
                let spans: Vec<Span> = edits
                    .iter()
                    .map(|e| doc.text_map().span(doc.bytes(), e.range))
                    .collect();
                self.verify_rename(&proj, &BTreeMap::from([(m, spans)]), new_name)?;
            }
            return Ok(vec![FileEdit {
                uri: uri.to_string(),
                edits,
            }]);
        };
        let proj = &sem.proj;
        check_new_name(&self.config().profile, new_name)?;
        let old = proj.decl_name(&d);
        let old_norm = normalize(old.as_bytes());
        let new_norm = new_name.to_ascii_lowercase();
        let occ = self.occurrences(proj, &d);
        if occ.is_empty() {
            return Err(Error::Invalid("nothing to rename here".into()));
        }
        if old_norm != new_norm {
            self.check_clashes(proj, &d, &occ, &new_norm, new_name)?;
        }
        self.check_unverifiable_uses(proj, &d, &old_norm, &old)?;

        let mut by_mod: BTreeMap<usize, Vec<Span>> = BTreeMap::new();
        for (m, s) in occ {
            by_mod.entry(m).or_default().push(s);
        }
        for spans in by_mod.values_mut() {
            spans.sort_by_key(|s| (s.start, s.end));
            spans.dedup();
        }
        self.verify_rename(proj, &by_mod, new_name)?;
        Ok(by_mod
            .into_iter()
            .map(|(m, spans)| FileEdit {
                uri: proj.mods[m].uri.clone(),
                edits: spans
                    .into_iter()
                    .map(|s| TextEdit {
                        range: self.range_in(&proj.mods[m].uri, s),
                        new_text: new_name.to_string(),
                    })
                    .collect(),
            })
            .collect())
    }

    fn proc_declares(proj: &Project, p: &vba_sema::ProcInfo, nn: &str) -> bool {
        let prog = &proj.sema.program;
        p.locals.iter().any(|l| {
            !matches!(l.kind, vba_sema::LocalKind::Temp) && normalize(l.name.as_bytes()) == nn
        }) || prog
            .globals
            .iter()
            .any(|g| g.static_of == Some(p.id) && normalize(g.name.as_bytes()) == nn)
    }

    /// Refuses a new name that would clash with, shadow or capture another
    /// declaration.
    fn check_clashes(
        &self,
        proj: &Project,
        d: &Decl,
        occ: &[(usize, Span)],
        nn: &str,
        new_name: &str,
    ) -> Result<(), Error> {
        let prog = &proj.sema.program;
        let clash = |what: &str| Err(Error::Invalid(format!("`{new_name}` {what}")));
        let name = vba_ast::Name::new(nn);
        let home = proj.decl_module(d);
        // Parameters, locals and `Static` locals live in one procedure.
        let scoped = match d {
            Decl::Local(p, _) => Some(*p),
            Decl::Global(g) => prog.global(*g).static_of,
            _ => None,
        };
        match d {
            _ if scoped.is_some() => {
                let pi = prog.proc(scoped.expect("checked"));
                if Self::proc_declares(proj, pi, nn) {
                    return clash("is already declared in the same procedure");
                }
                if normalize(pi.name.as_bytes()) == nn {
                    return clash("is the name of the procedure");
                }
                // Capture: another use of the new name in the procedure
                // would start to mean the local.
                let pm = &proj.mods[home.index()];
                let doc = self.docs().get(&pm.uri);
                if let Some(doc) = doc {
                    let an = doc.analysis(self.config());
                    let tokens = an.tree().root().descendant_tokens();
                    for (s, n) in &pm.idents {
                        if n != nn || s.start < pi.span.start || s.start >= pi.span.end {
                            continue;
                        }
                        let i = tokens.partition_point(|t| t.span().start < s.start);
                        let Some(t) = tokens.get(i) else { continue };
                        if t.span().start != s.start || is_member_or_arg_name(t) {
                            continue;
                        }
                        return clash(
                            "would change the meaning of an existing use in this procedure",
                        );
                    }
                }
            }
            Decl::Field(u, _) => {
                if prog.udts[u.index()]
                    .fields
                    .iter()
                    .any(|f| normalize(f.name.as_bytes()) == nn)
                {
                    return clash("is already a field of this type");
                }
            }
            Decl::Module(_) => {}
            _ => {
                if prog.module(home).symbols.contains_key(&name) {
                    return clash("is already declared in the same module");
                }
                if prog.modules.iter().any(|m| m.name == name) {
                    return clash("is the name of a module");
                }
                for (m, s) in occ {
                    let ctx = Ctx::new(proj, *m);
                    if let Some(p) = ctx.proc_at(s.start)
                        && Self::proc_declares(proj, p, nn)
                    {
                        return clash("would be hidden by a local of the same name");
                    }
                    if ModuleId(*m as u32) != home && prog.modules[*m].symbols.contains_key(&name) {
                        return clash("would be captured by a declaration in another module");
                    }
                }
                let global = prog.module(home).kind == ModuleKind::Standard
                    || matches!(d, Decl::Udt(_) | Decl::Enum(_) | Decl::EnumMember(..));
                if global {
                    for (i, o) in prog.modules.iter().enumerate() {
                        if i >= proj.mods.len() || o.id == home {
                            continue;
                        }
                        if let Some((sym, _)) = o.symbols.get(&name)
                            && (o.kind == ModuleKind::Standard
                                || matches!(sym, Symbol::Udt(_) | Symbol::Enum(_)))
                            && proj.symbol_public(sym)
                        {
                            return clash(
                                "is already declared in another module and would be ambiguous",
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Refuses renames whose other uses cannot be found: late-bound
    /// accesses of a class member and named arguments of a parameter.
    fn check_unverifiable_uses(
        &self,
        proj: &Project,
        d: &Decl,
        old_norm: &str,
        old: &str,
    ) -> Result<(), Error> {
        let prog = &proj.sema.program;
        let class_member = matches!(d, Decl::Proc(..) | Decl::Global(_) | Decl::Event(_))
            && prog.module(proj.decl_module(d)).kind != ModuleKind::Standard;
        let param = matches!(d, Decl::Local(p, i)
            if matches!(prog.proc(*p).locals[*i as usize].kind, vba_sema::LocalKind::Param(_)));
        if !class_member && !param {
            return Ok(());
        }
        let mut late = 0usize;
        let mut named = 0usize;
        for (mi, pm) in proj.mods.iter().enumerate() {
            if !pm.idents.iter().any(|(_, n)| n == old_norm) {
                continue;
            }
            let Some(doc) = self.docs().get(&pm.uri) else {
                continue;
            };
            let an = doc.analysis(self.config());
            let tokens = an.tree().root().descendant_tokens();
            let ctx = Ctx::new(proj, mi);
            for (k, t) in tokens.iter().enumerate() {
                if !word_token(t) || normalize(t.text()) != old_norm {
                    continue;
                }
                if class_member && is_member_name(t) && ctx.binding(t) == Binding::Late {
                    late += 1;
                }
                if param && t.parent().kind() == NodeKind::Arg {
                    let next = tokens[k + 1..].iter().find(|x| !x.kind().is_trivia());
                    if next.is_some_and(|x| x.kind() == TokenKind::ColonEq) {
                        named += 1;
                    }
                }
            }
        }
        if late > 0 {
            return Err(Error::Unsupported(format!(
                "`{old}` is also accessed late bound ({late} use(s) on a Variant or Object \
                 receiver) and those uses cannot be renamed safely"
            )));
        }
        if named > 0 {
            return Err(Error::Unsupported(format!(
                "{named} named argument(s) called `{old}` exist in the workspace and cannot be \
                 attributed to this parameter"
            )));
        }
        Ok(())
    }

    /// Applies the edits to copies of the documents and analyses the
    /// result: the rename is refused when it adds a diagnostic of any code.
    fn verify_rename(
        &self,
        proj: &Project,
        by_mod: &BTreeMap<usize, Vec<Span>>,
        new_name: &str,
    ) -> Result<(), Error> {
        let mut docs: Vec<(String, Document)> = Vec::new();
        for (mi, pm) in proj.mods.iter().enumerate() {
            let Some(doc) = self.docs().get(&pm.uri) else {
                continue;
            };
            let mut bytes = doc.bytes().to_vec();
            if let Some(spans) = by_mod.get(&mi) {
                for s in spans.iter().rev() {
                    bytes.splice(s.start as usize..s.end as usize, new_name.bytes());
                }
            }
            docs.push((
                pm.uri.clone(),
                Document::from_bytes(bytes, doc.version(), doc.text_map().encoding()),
            ));
        }
        let refs: Vec<(&str, &Document)> = docs.iter().map(|(u, d)| (u.as_str(), d)).collect();
        let after = Project::build(self.config(), &refs, proj.provider.clone());
        let count = |p: &Project| {
            let mut m: HashMap<(&'static str, usize), usize> = HashMap::new();
            for d in &p.sema.diagnostics {
                let mi = p.module_of_file(d.file).unwrap_or(usize::MAX);
                *m.entry((d.code.0, mi)).or_default() += 1;
            }
            m
        };
        let (before_c, after_c) = (count(proj), count(&after));
        for ((code, mi), n) in &after_c {
            if *n > before_c.get(&(*code, *mi)).copied().unwrap_or(0) {
                let msg = after
                    .sema
                    .diagnostics
                    .iter()
                    .find(|d| d.code.0 == *code && after.module_of_file(d.file) == Some(*mi))
                    .map(|d| d.message.clone())
                    .unwrap_or_default();
                return Err(Error::Invalid(format!(
                    "renaming to `{new_name}` would introduce {code}: {msg}"
                )));
            }
        }
        Ok(())
    }

    /// Declarations of all open modules whose name matches `query`
    /// (case-insensitive; a prefix, a substring or a subsequence), best
    /// matches first, at most `limit`.
    pub fn workspace_symbols(&self, query: &str, limit: usize) -> Vec<WorkspaceSymbol> {
        let Some(proj) = self.project() else {
            return Vec::new();
        };
        let q = query.trim().to_ascii_lowercase();
        let prog = &proj.sema.program;
        let mut found: Vec<(u8, WorkspaceSymbol)> = Vec::new();
        for d in &proj.order {
            let (kind, container) = match d {
                Decl::Local(..) => continue,
                Decl::Global(g) if prog.global(*g).static_of.is_some() => continue,
                Decl::Module(m) => {
                    let k = if prog.module(*m).kind == ModuleKind::Standard {
                        SymbolKind::Module
                    } else {
                        SymbolKind::Class
                    };
                    (k, String::new())
                }
                Decl::Proc(m, l) => {
                    let k = match proj.group(*m, l).first().map(|p| p.kind) {
                        Some(vba_ast::ProcKind::Sub) => SymbolKind::Method,
                        Some(vba_ast::ProcKind::Function) => SymbolKind::Function,
                        _ => SymbolKind::Property,
                    };
                    (k, prog.module(*m).name.to_string_lossy())
                }
                Decl::Declare(_) => (SymbolKind::Function, String::new()),
                Decl::Global(_) => (SymbolKind::Field, String::new()),
                Decl::Const(_) => (SymbolKind::Constant, String::new()),
                Decl::Udt(_) => (SymbolKind::Struct, String::new()),
                Decl::Field(u, _) => (
                    SymbolKind::Field,
                    prog.udts[u.index()].name.to_string_lossy(),
                ),
                Decl::Enum(_) => (SymbolKind::Enum, String::new()),
                Decl::EnumMember(e, _) => (
                    SymbolKind::EnumMember,
                    prog.enums[e.index()].name.to_string_lossy(),
                ),
                Decl::Event(_) => (SymbolKind::Event, String::new()),
            };
            let name = proj.decl_name(d);
            let Some(score) = match_score(&q, &name.to_ascii_lowercase()) else {
                continue;
            };
            let container = if container.is_empty() {
                match d {
                    Decl::Module(_) => String::new(),
                    d => prog.module(proj.decl_module(d)).name.to_string_lossy(),
                }
            } else {
                container
            };
            let locs = self.decl_locations(&proj, d);
            let Some(location) = locs.into_iter().next() else {
                continue;
            };
            found.push((
                score,
                WorkspaceSymbol {
                    name,
                    kind,
                    container,
                    location,
                },
            ));
        }
        found.sort_by(|a, b| {
            (
                a.0,
                a.1.name.to_ascii_lowercase(),
                &a.1.location.uri,
                a.1.location.range.start,
            )
                .cmp(&(
                    b.0,
                    b.1.name.to_ascii_lowercase(),
                    &b.1.location.uri,
                    b.1.location.range.start,
                ))
        });
        found.into_iter().take(limit).map(|(_, s)| s).collect()
    }
}

/// 0 exact, 1 prefix, 2 substring, 3 subsequence.
fn match_score(q: &str, name: &str) -> Option<u8> {
    if q.is_empty() {
        return Some(0);
    }
    if name == q {
        return Some(0);
    }
    if name.starts_with(q) {
        return Some(1);
    }
    if name.contains(q) {
        return Some(2);
    }
    let mut it = name.chars();
    q.chars().all(|c| it.any(|n| n == c)).then_some(3)
}

/// Whether `t` is the member name of a `MemberExpr`.
fn is_member_name(t: &SyntaxToken<'_>) -> bool {
    let p = t.parent();
    p.kind() == NodeKind::MemberExpr
        && p.child_tokens()
            .find(word_token)
            .is_some_and(|f| f.span() == t.span())
}

fn is_member_or_arg_name(t: &SyntaxToken<'_>) -> bool {
    is_member_name(t) || t.parent().kind() == NodeKind::Arg
}

impl View<'_> {
    fn sem(&self) -> Option<Sem> {
        let proj = self.ws.project()?;
        let m = *proj.by_uri.get(&self.uri)?;
        Some(Sem { proj, m })
    }

    /// Signature of a call to a member of a library type: the callee token
    /// is the member name of a member access whose receiver the project
    /// types as a library type.
    pub(crate) fn lib_signature(
        &self,
        tokens: &[SyntaxToken<'_>],
        callee: usize,
        arg: u32,
        named: Option<&str>,
    ) -> Option<crate::types::SignatureHelp> {
        let sem = self.sem()?;
        let ctx = Ctx::new(&sem.proj, sem.m);
        let t = tokens[callee];
        let node = t.parent();
        if node.kind() != NodeKind::MemberExpr {
            return None;
        }
        let name = String::from_utf8_lossy(t.text()).into_owned();
        let (owner, m) = ctx.lib_member(&ctx.receiver_ty(node), &name)?;
        Some(self.member_signature(&sem.proj, &owner, &m, arg, named))
    }

    /// Signature help for the member `m` of `owner`.
    fn member_signature(
        &self,
        proj: &Project,
        owner: &LibraryType,
        m: &LibMember,
        arg: u32,
        named: Option<&str>,
    ) -> crate::types::SignatureHelp {
        let (label, spans) = proj.lib_member_sig(owner, m);
        let n = m.params.len();
        let idx = match named {
            Some(nm) => m
                .params
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(nm))
                .map(|i| i as u32),
            None => Some(arg),
        };
        let active = idx.and_then(|i| {
            if (i as usize) < n {
                Some(i)
            } else if m.params.last().is_some_and(|p| p.param_array) {
                Some(n as u32 - 1)
            } else {
                None
            }
        });
        crate::types::SignatureHelp {
            signatures: vec![crate::types::SignatureInfo {
                label,
                documentation: Some(match proj.lib_doc(Some(owner), Some(&m.name)) {
                    Some(d) => format!("{d}\n\nMember of `{owner}`."),
                    None => format!("Member of `{owner}`."),
                }),
                parameters: spans
                    .into_iter()
                    .map(|(start, end)| crate::types::ParameterInfo { start, end })
                    .collect(),
            }],
            active_parameter: active,
        }
    }

    /// Signature help for a call of a global function of a described
    /// library (`Nz(`).
    pub(crate) fn lib_global_signature(
        &self,
        name: &str,
        arg: u32,
        named: Option<&str>,
    ) -> Option<crate::types::SignatureHelp> {
        let sem = self.sem()?;
        if !name.contains('.') {
            // A method of the module's described host class, called by its
            // bare name.
            let ctx = Ctx::new(&sem.proj, sem.m);
            if let Some((owner, m)) = ctx.host_member(name) {
                return Some(self.member_signature(&sem.proj, &owner, &m, arg, named));
            }
        }
        sem.proj.lib_global_signature(name, arg, named)
    }

    /// The names a described library offers after `Library.`.
    pub(crate) fn lib_qualified_items(&self, library: &str) -> Option<Vec<CompletionItem>> {
        self.sem()?.proj.lib_qualified_items(library)
    }

    /// Type-aware hover for names bound by the project: the declaration
    /// with its declared type, signature or constant value.
    pub(crate) fn semantic_hover(
        &self,
        tokens: &[SyntaxToken<'_>],
        i: usize,
        range: Range,
    ) -> Option<Hover> {
        let sem = self.sem()?;
        let ctx = Ctx::new(&sem.proj, sem.m);
        let t = tokens[i];
        let fence = |code: String, extra: Vec<String>| {
            let mut s = format!("```vba\n{code}\n```");
            for e in extra {
                s.push_str("\n\n");
                s.push_str(&e);
            }
            Some(Hover { contents: s, range })
        };
        if t.kind() == TokenKind::Keyword(Keyword::Me) {
            let kind = sem.proj.mods[sem.m].kind;
            if kind != ModuleKind::Standard {
                return fence(
                    format!("Me As {}", sem.proj.mods[sem.m].name),
                    vec!["The current instance.".to_string()],
                );
            }
            return None;
        }
        match ctx.binding(&t) {
            Binding::Decl(d) => {
                let (code, extra) = sem.proj.describe(&d);
                fence(code, extra)
            }
            Binding::Late => fence(
                format!("(late bound) {}", String::from_utf8_lossy(t.text())),
                vec![
                    "The receiver is a `Variant` or `Object`; the member is resolved at run time."
                        .to_string(),
                ],
            ),
            Binding::Unknown => self.lib_hover(&ctx, &sem.proj, &t, fence),
        }
    }

    /// Hover for a member of a library type described by the embedding
    /// application's type provider.
    fn lib_hover(
        &self,
        ctx: &Ctx<'_>,
        proj: &Project,
        t: &SyntaxToken<'_>,
        fence: impl Fn(String, Vec<String>) -> Option<Hover>,
    ) -> Option<Hover> {
        let node = t.parent();
        let name = String::from_utf8_lossy(t.text()).into_owned();
        if node.kind() == NodeKind::NameExpr
            && let Some((owner, m)) = ctx.host_member(&name)
        {
            return self.member_hover(proj, &owner, &m, fence);
        }
        if node.kind() != NodeKind::MemberExpr {
            return proj.lib_global_hover(&name, node.kind() == NodeKind::TypeRef, fence);
        }
        let (owner, m) = ctx.lib_member(&ctx.receiver_ty(node), &name)?;
        self.member_hover(proj, &owner, &m, fence)
    }

    fn member_hover(
        &self,
        proj: &Project,
        owner: &LibraryType,
        m: &LibMember,
        fence: impl Fn(String, Vec<String>) -> Option<Hover>,
    ) -> Option<Hover> {
        let (label, _) = proj.lib_member_sig(owner, m);
        let mut extra = Vec::new();
        extra.extend(proj.lib_doc(Some(owner), Some(&m.name)));
        extra.push(format!("Member of `{owner}`."));
        if m.is_default {
            extra.push("Default member.".to_string());
        }
        fence(label, extra)
    }

    /// Members of the receiver before the dot at token `dot`, when the
    /// project knows the receiver's type: `Some` (possibly empty) when the
    /// receiver is typed, `None` to let the syntactic rules answer.
    pub(crate) fn semantic_member_items(
        &self,
        tokens: &[SyntaxToken<'_>],
        dot: usize,
    ) -> Option<Vec<CompletionItem>> {
        let sem = self.sem()?;
        let ctx = Ctx::new(&sem.proj, sem.m);
        let node = tokens[dot].parent();
        if node.kind() != NodeKind::MemberExpr {
            return None;
        }
        let recv = ctx.receiver_ty(node);
        let proj = &sem.proj;
        let prog = &proj.sema.program;
        let here = ModuleId(sem.m as u32);
        let mut items = Vec::new();
        let push_symbol =
            |name: &str, sym: &Symbol, m: ModuleId, items: &mut Vec<CompletionItem>| {
                let Some(d) = proj.decl_of_symbol(m, sym) else {
                    return;
                };
                let kind = match sym {
                    Symbol::Proc(p) if prog.proc(*p).has_result => CompletionKind::Procedure,
                    Symbol::Proc(_) | Symbol::Declare(_) => CompletionKind::Procedure,
                    Symbol::Property(_) => CompletionKind::Property,
                    Symbol::Var(_) => CompletionKind::Field,
                    Symbol::Const(_) => CompletionKind::Constant,
                    Symbol::Enum(_) | Symbol::Udt(_) => CompletionKind::Type,
                    Symbol::EnumMember(..) => CompletionKind::EnumMember,
                    Symbol::Event(_) => return,
                };
                let (code, _) = proj.describe(&d);
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind,
                    detail: Some(code.lines().next().unwrap_or_default().to_string()),
                    documentation: None,
                });
            };
        match recv {
            Ty::Known(Type::Class(m)) | Ty::Module(m) => {
                let class = matches!(recv, Ty::Known(_));
                for (n, (sym, _)) in &prog.module(m).symbols {
                    if m != here && !proj.symbol_public(sym) {
                        continue;
                    }
                    if class
                        && matches!(
                            sym,
                            Symbol::Const(_)
                                | Symbol::Enum(_)
                                | Symbol::EnumMember(..)
                                | Symbol::Udt(_)
                                | Symbol::Declare(_)
                        )
                    {
                        continue;
                    }
                    push_symbol(&n.to_string_lossy(), sym, m, &mut items);
                }
                if let Some(lt) = proj.host_classes.get(&m.index()) {
                    items.extend(self.host_class_items(&ctx, lt));
                }
                Some(items)
            }
            Ty::Known(Type::Udt(u)) => {
                for (i, f) in prog.udts[u.index()].fields.iter().enumerate() {
                    let d = Decl::Field(u, i as u32);
                    items.push(CompletionItem {
                        label: f.name.to_string_lossy(),
                        kind: CompletionKind::Field,
                        detail: Some(proj.describe(&d).0),
                        documentation: None,
                    });
                }
                Some(items)
            }
            Ty::Enum(e) => {
                for (i, (n, _)) in prog.enums[e.index()].members.iter().enumerate() {
                    let d = Decl::EnumMember(e, i as u32);
                    items.push(CompletionItem {
                        label: n.to_string_lossy(),
                        kind: CompletionKind::EnumMember,
                        detail: Some(proj.describe(&d).0),
                        documentation: None,
                    });
                }
                Some(items)
            }
            Ty::Project => {
                for (i, m) in prog.modules.iter().enumerate().take(proj.mods.len()) {
                    let _ = i;
                    items.push(CompletionItem {
                        label: m.name.to_string_lossy(),
                        kind: CompletionKind::Module,
                        detail: None,
                        documentation: None,
                    });
                }
                Some(items)
            }
            Ty::Known(Type::Variant | Type::Object | Type::Any) => Some(items),
            Ty::Known(Type::Library(lt)) => {
                for m in ctx.lib_members(&lt) {
                    let kind = match m.kind {
                        vba_sema::LibMemberKind::Method => CompletionKind::Procedure,
                        _ => CompletionKind::Property,
                    };
                    items.push(CompletionItem {
                        label: m.name.clone(),
                        kind,
                        detail: Some(proj.lib_member_sig(&lt, &m).0),
                        documentation: proj.lib_doc(Some(&lt), Some(&m.name)),
                    });
                }
                Some(items)
            }
            _ => None,
        }
    }

    /// The controls, properties and methods of a module's described host
    /// class, as completion items.
    fn host_class_items(&self, ctx: &Ctx<'_>, lt: &LibraryType) -> Vec<CompletionItem> {
        ctx.lib_members(lt)
            .into_iter()
            .map(|m| CompletionItem {
                label: m.name.clone(),
                kind: match m.kind {
                    vba_sema::LibMemberKind::Method => CompletionKind::Procedure,
                    _ => CompletionKind::Property,
                },
                detail: Some(ctx.proj.lib_member_sig(lt, &m).0),
                documentation: ctx.proj.lib_doc(Some(lt), Some(&m.name)),
            })
            .collect()
    }

    /// Names of other modules that are visible without qualification:
    /// public members of standard modules, public types and enums, and
    /// module names (as qualifiers or class types).
    pub(crate) fn semantic_global_items(&self, types_only: bool) -> Vec<CompletionItem> {
        let Some(sem) = self.sem() else {
            return Vec::new();
        };
        let proj = &sem.proj;
        let prog = &proj.sema.program;
        let mut items = proj.lib_catalog_items(types_only);
        if !types_only && let Some(lt) = proj.host_classes.get(&sem.m) {
            items.extend(self.host_class_items(&Ctx::new(proj, sem.m), lt));
        }
        for (i, m) in prog.modules.iter().enumerate().take(proj.mods.len()) {
            if i == sem.m {
                continue;
            }
            let class_like = m.kind != ModuleKind::Standard;
            if !types_only || class_like {
                items.push(CompletionItem {
                    label: m.name.to_string_lossy(),
                    kind: if class_like {
                        CompletionKind::Type
                    } else {
                        CompletionKind::Module
                    },
                    detail: None,
                    documentation: None,
                });
            }
            if m.kind == ModuleKind::Standard && m.options.private_module && !types_only {
                continue;
            }
            for (n, (sym, _)) in &m.symbols {
                let type_like = matches!(sym, Symbol::Udt(_) | Symbol::Enum(_));
                if !proj.symbol_public(sym) {
                    continue;
                }
                if types_only && !type_like {
                    continue;
                }
                if !type_like && class_like {
                    continue;
                }
                if matches!(sym, Symbol::Event(_)) {
                    continue;
                }
                let Some(d) = proj.decl_of_symbol(m.id, sym) else {
                    continue;
                };
                let kind = match sym {
                    Symbol::Proc(_) | Symbol::Property(_) | Symbol::Declare(_) => {
                        CompletionKind::Procedure
                    }
                    Symbol::Var(_) => CompletionKind::Variable,
                    Symbol::Const(_) => CompletionKind::Constant,
                    Symbol::Enum(_) | Symbol::Udt(_) => CompletionKind::Type,
                    Symbol::EnumMember(..) => CompletionKind::EnumMember,
                    Symbol::Event(_) => continue,
                };
                items.push(CompletionItem {
                    label: n.to_string_lossy(),
                    kind,
                    detail: Some(
                        proj.describe(&d)
                            .0
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .to_string(),
                    ),
                    documentation: None,
                });
            }
        }
        items
    }
}

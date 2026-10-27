//! The project view of the workspace: every open document is one module of
//! one `vba-sema` analysis. The analysis is cached by a workspace generation
//! number (any open, edit, close or configuration change bumps it), so the
//! semantic features of a quiet workspace share one run.

use std::collections::HashMap;
use std::sync::Arc;

use vba_ast::Visibility;
use vba_lexer::TokenKind;
use vba_project::ModuleKind;
use vba_sema::{
    Analysis, DeclareId, EnumId, EventId, GlobalId, ModuleId, NoLibraries, ProcId, Symbol, Type,
    TypeProvider, UdtId,
};
use vba_source::{FileId, Span};
use vba_syntax::NodeKind;

use crate::document::{Config, Document};
use crate::text::decode;
use crate::util::{normalize, suffix_len};

/// A type provider shared between the workspace and its project analyses.
pub type SharedProvider = Arc<dyn TypeProvider + Send + Sync>;

/// What a name denotes in the analysed project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Decl {
    /// A procedure; `Property Get/Let/Set` of one name are one declaration.
    Proc(ModuleId, String),
    Declare(DeclareId),
    Global(GlobalId),
    Const(vba_sema::ConstId),
    Udt(UdtId),
    Field(UdtId, u32),
    Enum(EnumId),
    EnumMember(EnumId, u32),
    Event(EventId),
    /// A parameter or local variable (index into the procedure's locals).
    Local(ProcId, u32),
    Module(ModuleId),
}

/// A module of the project (one open document).
pub(crate) struct PMod {
    pub uri: String,
    pub name: String,
    pub kind: ModuleKind,
    /// Name-like tokens (identifiers, foreign names, keywords) in source
    /// order, with the span of the name proper (type suffix excluded) and
    /// the normalised name.
    pub idents: Vec<(Span, String)>,
}

impl PMod {
    /// The name span of the identifier token that starts at `start`.
    pub fn ident_at(&self, start: u32) -> Option<&(Span, String)> {
        let i = self.idents.partition_point(|(s, _)| s.start < start);
        self.idents.get(i).filter(|(s, _)| s.start == start)
    }

    /// The first identifier named `norm` whose span lies in `within` and
    /// starts at or after `from`.
    pub fn find_name(&self, within: Span, from: u32, norm: &str) -> Option<Span> {
        let start = within.start.max(from);
        let i = self.idents.partition_point(|(s, _)| s.start < start);
        self.idents[i..]
            .iter()
            .take_while(|(s, _)| s.start < within.end)
            .find(|(_, n)| n == norm)
            .map(|(s, _)| *s)
    }
}

/// One analysis of the whole workspace.
pub(crate) struct Project {
    pub mods: Vec<PMod>,
    pub by_uri: HashMap<String, usize>,
    pub sema: Analysis,
    /// Declaration name spans per declaration.
    pub decls: HashMap<Decl, Vec<(ModuleId, Span)>>,
    /// Declaration at a name span `(module, start)`.
    pub decl_at: HashMap<(u32, u32), Decl>,
    /// Display order of declarations for workspace symbols.
    pub order: Vec<Decl>,
    pub project_name: String,
    /// The embedding application's view of type libraries and host types.
    pub provider: Option<SharedProvider>,
    /// The described host class of modules that have one (module index).
    pub host_classes: HashMap<usize, vba_sema::LibraryType>,
}

pub(crate) fn mid(i: usize) -> ModuleId {
    ModuleId(i as u32)
}

/// Last path segment of a URI, percent-decoding left alone.
fn file_name(uri: &str) -> &str {
    let u = uri.split(['?', '#']).next().unwrap_or(uri);
    u.rsplit(['/', '\\']).next().unwrap_or(u)
}

fn file_stem(uri: &str) -> String {
    let f = file_name(uri);
    match f.rfind('.') {
        Some(i) if i > 0 => f[..i].to_string(),
        _ => f.to_string(),
    }
}

/// The module kind configured for `uri`, if a pattern matches: the full URI,
/// the file name, or `*.ext`.
fn configured_kind(config: &Config, uri: &str) -> Option<ModuleKind> {
    let f = file_name(uri).to_ascii_lowercase();
    config.module_kinds.iter().find_map(|(pat, kind)| {
        let p = pat.to_ascii_lowercase();
        let hit = p == uri.to_ascii_lowercase()
            || p == f
            || p.strip_prefix('*').is_some_and(|ext| f.ends_with(ext));
        hit.then_some(*kind)
    })
}

/// Module name and kind of a document: configuration, then
/// `Attribute VB_Name` and the export header, then the file extension.
pub(crate) fn infer_module(
    config: &Config,
    uri: &str,
    doc: &Document,
    tree: &vba_syntax::SyntaxTree,
) -> (String, ModuleKind) {
    let mut name = None;
    let mut class_header = false;
    let mut form_header = false;
    for n in tree.root().child_nodes() {
        match n.kind() {
            NodeKind::AttributeStmt if name.is_none() => {
                let toks: Vec<_> = n
                    .descendant_tokens()
                    .into_iter()
                    .filter(|t| !t.kind().is_trivia())
                    .collect();
                if toks.len() >= 4
                    && toks[1].text_eq("VB_Name")
                    && toks[2].kind() == TokenKind::Eq
                    && toks[3].kind() == TokenKind::StringLiteral
                {
                    let raw = toks[3].text();
                    let inner = &raw[1..raw.len().saturating_sub(1).max(1)];
                    let s = decode(inner, doc.text_map().encoding());
                    if !s.is_empty() {
                        name = Some(s);
                    }
                }
            }
            NodeKind::VersionLine => {
                class_header = n.significant_tokens().any(|t| t.text_eq("CLASS"));
            }
            NodeKind::HeaderBlock => form_header = !class_header,
            _ => {}
        }
    }
    let ext = file_name(uri)
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let kind = configured_kind(config, uri).unwrap_or(match ext.as_str() {
        "bas" => ModuleKind::Standard,
        "cls" => ModuleKind::Class,
        "frm" => ModuleKind::Designer,
        _ if class_header => ModuleKind::Class,
        _ if form_header => ModuleKind::Designer,
        _ => ModuleKind::Standard,
    });
    (name.unwrap_or_else(|| file_stem(uri)), kind)
}

fn name_span(start: u32, end: u32, suffix: u32) -> Span {
    Span::new(start, end - suffix)
}

impl Project {
    /// Analyses `docs` (sorted by URI) as one project.
    pub fn build(
        config: &Config,
        docs: &[(&str, &Document)],
        provider: Option<SharedProvider>,
    ) -> Project {
        let mut mods = Vec::new();
        let mut sources = Vec::new();
        let mut by_uri = HashMap::new();
        for (i, (uri, doc)) in docs.iter().enumerate() {
            let an = doc.analysis(config);
            let (name, kind) = infer_module(config, uri, doc, an.tree());
            let mut idents = Vec::new();
            for t in an.tree().root().descendant_tokens() {
                if matches!(
                    t.kind(),
                    TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
                ) {
                    let s = t.span();
                    idents.push((
                        name_span(s.start, s.end, suffix_len(&t)),
                        normalize(t.text()),
                    ));
                }
            }
            by_uri.insert((*uri).to_string(), i);
            sources.push(vba_sema::SourceModule {
                name: name.clone(),
                kind,
                source: doc.bytes().to_vec(),
            });
            mods.push(PMod {
                uri: (*uri).to_string(),
                name,
                kind,
                idents,
            });
        }
        let opts = vba_sema::Options {
            strict_syntax: true,
            project_name: config.project_name.clone(),
            constants: config.constants.clone(),
            module_hosts: config.module_hosts.clone(),
            ..vba_sema::Options::default()
        };
        let lib: &dyn TypeProvider = match provider.as_deref() {
            Some(p) => p,
            None => &NoLibraries,
        };
        let sema = vba_sema::analyze_sources(&sources, &config.profile, lib, &opts);
        let mut p = Project {
            mods,
            by_uri,
            sema,
            decls: HashMap::new(),
            decl_at: HashMap::new(),
            order: Vec::new(),
            project_name: config.project_name.clone(),
            host_classes: HashMap::new(),
            provider,
        };
        p.index_host_classes(&config.module_hosts);
        p.index_decls();
        p
    }

    /// Resolves the configured host classes through the provider, as the
    /// analysis did.
    fn index_host_classes(&mut self, hosts: &[vba_sema::ModuleHost]) {
        let Some(provider) = self.provider.clone() else {
            return;
        };
        for h in hosts {
            let Some(i) = self
                .mods
                .iter()
                .position(|m| m.name.eq_ignore_ascii_case(&h.module))
            else {
                continue;
            };
            let (q, n) = match h.class.rsplit_once('.') {
                Some((q, n)) => (Some(q), n),
                None => (None, h.class.as_str()),
            };
            if let Some(lt) = provider.resolve_type(q, n) {
                self.host_classes.insert(i, lt);
            }
        }
    }

    fn add_decl(&mut self, d: Decl, m: ModuleId, span: Span) {
        let list = self.decls.entry(d.clone()).or_default();
        if list.is_empty() {
            self.order.push(d.clone());
        }
        list.push((m, span));
        self.decl_at.insert((m.0, span.start), d);
    }

    /// The name span of a declaration whose full or name span is `span`.
    fn name_in(&self, m: ModuleId, span: Span, norm: &str) -> Span {
        let pm = &self.mods[m.index()];
        pm.find_name(span, 0, norm)
            .or_else(|| pm.ident_at(span.start).map(|(s, _)| *s))
            .unwrap_or(span)
    }

    fn index_decls(&mut self) {
        let prog = &self.sema.program;
        let mut entries: Vec<(Decl, ModuleId, Span)> = Vec::new();
        let mut mod_decls = Vec::new();
        for (i, md) in prog.modules.iter().enumerate() {
            if self.mods.get(i).is_none() {
                continue;
            }
            mod_decls.push(md.id);
        }
        for p in &prog.procs {
            let key = Decl::Proc(p.module, p.name.to_string_lossy().to_ascii_lowercase());
            let norm = normalize(p.name.as_bytes());
            let s = self.name_in(p.module, p.name_span, &norm);
            entries.push((key, p.module, s));
            for (li, l) in p.locals.iter().enumerate() {
                if !matches!(
                    l.kind,
                    vba_sema::LocalKind::Param(_)
                        | vba_sema::LocalKind::Var
                        | vba_sema::LocalKind::Implicit
                ) {
                    continue;
                }
                let s = self.name_in(p.module, l.span, &normalize(l.name.as_bytes()));
                entries.push((Decl::Local(p.id, li as u32), p.module, s));
            }
        }
        for g in &prog.globals {
            let s = self.name_in(g.module, g.span, &normalize(g.name.as_bytes()));
            entries.push((Decl::Global(g.id), g.module, s));
        }
        for c in &prog.consts {
            let s = self.name_in(c.module, c.span, &normalize(c.name.as_bytes()));
            entries.push((Decl::Const(c.id), c.module, s));
        }
        for u in &prog.udts {
            let s = self.name_in(u.module, u.span, &normalize(u.name.as_bytes()));
            entries.push((Decl::Udt(u.id), u.module, s));
            for (fi, f) in u.fields.iter().enumerate() {
                let fs = self.name_in(u.module, f.span, &normalize(f.name.as_bytes()));
                entries.push((Decl::Field(u.id, fi as u32), u.module, fs));
            }
        }
        for e in &prog.enums {
            let s = self.name_in(e.module, e.span, &normalize(e.name.as_bytes()));
            entries.push((Decl::Enum(e.id), e.module, s));
            let mut from = s.end;
            for (mi, (n, _)) in e.members.iter().enumerate() {
                let norm = normalize(n.as_bytes());
                let pm = &self.mods[e.module.index()];
                if let Some(ms) = pm.find_name(e.span, from, &norm) {
                    from = ms.end;
                    entries.push((Decl::EnumMember(e.id, mi as u32), e.module, ms));
                }
            }
        }
        for d in &prog.declares {
            let s = self.name_in(d.module, d.span, &normalize(d.name.as_bytes()));
            entries.push((Decl::Declare(d.id), d.module, s));
        }
        for ev in &prog.events {
            let s = self.name_in(ev.module, ev.span, &normalize(ev.name.as_bytes()));
            entries.push((Decl::Event(ev.id), ev.module, s));
        }
        for m in mod_decls {
            self.order.push(Decl::Module(m));
            self.decls.entry(Decl::Module(m)).or_default();
        }
        for (d, m, s) in entries {
            self.add_decl(d, m, s);
        }
    }

    /// Module index of a sema file id.
    pub fn module_of_file(&self, f: FileId) -> Option<usize> {
        self.sema
            .program
            .modules
            .iter()
            .position(|m| m.file == f)
            .filter(|&i| i < self.mods.len())
    }

    /// Whether a module-scope symbol is visible outside its module.
    pub fn symbol_public(&self, sym: &Symbol) -> bool {
        let prog = &self.sema.program;
        let vis = |v: Visibility| !matches!(v, Visibility::Private);
        match sym {
            Symbol::Var(g) => vis(prog.global(*g).visibility),
            Symbol::Const(c) => vis(prog.consts[c.index()].visibility),
            Symbol::Proc(p) => vis(prog.proc(*p).visibility),
            Symbol::Property(g) => [g.get, g.let_, g.set]
                .into_iter()
                .flatten()
                .any(|p| vis(prog.proc(p).visibility)),
            Symbol::Declare(d) => vis(prog.declares[d.index()].visibility),
            Symbol::Enum(e) | Symbol::EnumMember(e, _) => vis(prog.enums[e.index()].visibility),
            Symbol::Udt(u) => vis(prog.udts[u.index()].visibility),
            Symbol::Event(_) => true,
        }
    }

    /// The declaration a module-scope symbol stands for.
    pub fn decl_of_symbol(&self, m: ModuleId, sym: &Symbol) -> Option<Decl> {
        let prog = &self.sema.program;
        let proc_key = |p: ProcId| {
            let pi = prog.proc(p);
            Decl::Proc(pi.module, pi.name.to_string_lossy().to_ascii_lowercase())
        };
        let _ = m;
        Some(match sym {
            Symbol::Var(g) => Decl::Global(*g),
            Symbol::Const(c) => Decl::Const(*c),
            Symbol::Proc(p) => proc_key(*p),
            Symbol::Property(g) => proc_key(g.get.or(g.let_).or(g.set)?),
            Symbol::Declare(d) => Decl::Declare(*d),
            Symbol::Enum(e) => Decl::Enum(*e),
            Symbol::EnumMember(e, i) => Decl::EnumMember(*e, *i),
            Symbol::Udt(u) => Decl::Udt(*u),
            Symbol::Event(e) => Decl::Event(*e),
        })
    }

    /// The declared type of a declaration, when it has one.
    pub fn decl_type(&self, d: &Decl) -> Option<Type> {
        let prog = &self.sema.program;
        Some(match d {
            Decl::Global(g) => prog.global(*g).ty.clone(),
            Decl::Const(c) => prog.consts[c.index()].ty.clone(),
            Decl::Field(u, i) => prog.udts[u.index()].fields[*i as usize].ty.clone(),
            Decl::Local(p, i) => prog.proc(*p).locals[*i as usize].ty.clone(),
            Decl::Declare(d) => prog.declares[d.index()].ret.clone(),
            Decl::EnumMember(e, _) => Type::Enum(*e),
            Decl::Proc(m, name) => {
                let procs = self.group(*m, name);
                let p = procs
                    .iter()
                    .find(|p| p.kind == vba_ast::ProcKind::PropertyGet)
                    .or(procs.first())?;
                if p.has_result || p.kind == vba_ast::ProcKind::PropertyGet {
                    p.ret.clone()
                } else {
                    return None;
                }
            }
            _ => return None,
        })
    }

    /// The procedures of a declaration group.
    pub fn group(&self, m: ModuleId, lname: &str) -> Vec<&vba_sema::ProcInfo> {
        self.sema
            .program
            .module(m)
            .procs
            .iter()
            .map(|p| self.sema.program.proc(*p))
            .filter(|p| p.name.to_string_lossy().eq_ignore_ascii_case(lname))
            .collect()
    }

    /// Declaration spans with their document URI.
    pub fn locations(&self, d: &Decl) -> Vec<(usize, Span)> {
        self.decls
            .get(d)
            .map(|v| v.iter().map(|(m, s)| (m.index(), *s)).collect())
            .unwrap_or_default()
    }

    /// Readable type name.
    pub fn type_name(&self, t: &Type) -> String {
        self.sema.program.type_name(t)
    }
}

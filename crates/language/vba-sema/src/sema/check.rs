//! Declaration-level checks: class lifetime procedures, property pairing,
//! `Declare` rules, `WithEvents` and event handlers, `Implements`
//! completeness.

use vba_ast::{Name, ProcKind, Visibility};
use vba_diagnostics::Severity;
use vba_project::ModuleKind;

use crate::codes;
use crate::program::*;
use crate::provider::{LibMember, LibMemberKind};
use crate::types::*;

use super::Sema;

/// The shape of an interface member for completeness checks.
struct Required {
    name: Name,
    kind: Option<ProcKind>,
    params: Vec<Type>,
    ret: Option<Type>,
}

impl Sema<'_> {
    pub(crate) fn check_declarations(&mut self) {
        let n = self.program.modules.len();
        for i in 0..n {
            let m = ModuleId(i as u32);
            self.check_class_hooks(m);
            self.check_properties(m);
            self.check_procedure_rules(m);
            self.check_with_events(m);
        }
        for i in 0..n {
            self.check_implements(ModuleId(i as u32));
        }
        self.check_declares();
    }

    fn check_class_hooks(&mut self, m: ModuleId) {
        if self.program.modules[m.index()].kind == ModuleKind::Standard {
            return;
        }
        let procs = self.program.modules[m.index()].procs.clone();
        for p in procs {
            let pi = &self.program.procs[p.index()];
            let hook = if pi.name.eq_ignore_case("Class_Initialize") {
                0
            } else if pi.name.eq_ignore_case("Class_Terminate") {
                1
            } else {
                continue;
            };
            if pi.kind != ProcKind::Sub || !pi.params.is_empty() {
                self.error(
                    m,
                    codes::CLASS_HOOK,
                    pi.name_span,
                    format!("`{}` must be a `Sub` without parameters", pi.name),
                );
                continue;
            }
            let mi = &mut self.program.modules[m.index()];
            if hook == 0 {
                mi.initialize = Some(p);
            } else {
                mi.terminate = Some(p);
            }
        }
    }

    /// MS-VBAL 5.3.1.1, 5.3.1.5 and 5.3.1.7 declaration rules.
    fn check_procedure_rules(&self, m: ModuleId) {
        for &p in &self.program.modules[m.index()].procs {
            let pi = &self.program.procs[p.index()];
            for prm in &pi.params {
                if prm.by_val && prm.is_array {
                    self.spec_strict(
                        m,
                        codes::PROCEDURE_RULE,
                        prm.span,
                        "an array parameter cannot be passed `ByVal`",
                    );
                }
                if matches!(pi.kind, ProcKind::Function | ProcKind::PropertyGet)
                    && prm.name == pi.name
                {
                    self.spec_strict(
                        m,
                        codes::PROCEDURE_RULE,
                        prm.span,
                        "a parameter cannot have the name of its function",
                    );
                }
            }
        }
        // A property's Get, Let and Set agree on how each shared parameter is passed.
        let groups: Vec<PropGroup> = self.program.modules[m.index()]
            .symbols
            .values()
            .filter_map(|(s, _)| match s {
                Symbol::Property(g) => Some(*g),
                _ => None,
            })
            .collect();
        for g in groups {
            let Some(get) = g.get.map(|p| &self.program.procs[p.index()]) else {
                continue;
            };
            for p in [g.let_, g.set].into_iter().flatten() {
                let pi = &self.program.procs[p.index()];
                if pi.params.len() != get.params.len() + 1 {
                    continue; // reported by `check_properties`
                }
                for (a, b) in get.params.iter().zip(&pi.params) {
                    if a.by_val != b.by_val {
                        self.spec_strict_with_label(
                            m,
                            codes::PROPERTY_PAIRING,
                            b.span,
                            format!(
                                "parameter `{}` is passed differently than in `Property Get {}`",
                                b.name, get.name
                            ),
                            a.span,
                            "declared here",
                        );
                    }
                }
            }
        }
    }

    fn check_properties(&self, m: ModuleId) {
        let groups: Vec<(Name, PropGroup)> = self.program.modules[m.index()]
            .symbols
            .iter()
            .filter_map(|(n, (s, _))| match s {
                Symbol::Property(g) => Some((n.clone(), *g)),
                _ => None,
            })
            .collect();
        let mut groups = groups;
        groups.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, g) in groups {
            let get = g.get.map(|p| &self.program.procs[p.index()]);
            for (p, is_set) in [(g.let_, false), (g.set, true)] {
                let Some(p) = p else { continue };
                let pi = &self.program.procs[p.index()];
                let kw = if is_set {
                    "Property Set"
                } else {
                    "Property Let"
                };
                let Some(value) = pi.params.last() else {
                    self.error(
                        m,
                        codes::PROPERTY_PAIRING,
                        pi.name_span,
                        format!("`{kw} {name}` needs a value parameter"),
                    );
                    continue;
                };
                if value.optional || value.param_array {
                    self.error(
                        m,
                        codes::PARAMETER_RULE,
                        value.span,
                        "the value parameter of a property cannot be `Optional` or `ParamArray`",
                    );
                }
                if is_set && !(value.ty.is_object() || value.ty.is_variant() || value.ty.is_error())
                {
                    self.error(
                        m,
                        codes::PROPERTY_PAIRING,
                        value.span,
                        "the value parameter of `Property Set` must be an object type or `Variant`",
                    );
                }
                let Some(get) = get else { continue };
                if pi.params.len() != get.params.len() + 1 {
                    self.error_with_label(
                        m,
                        codes::PROPERTY_PAIRING,
                        pi.name_span,
                        format!(
                            "`{kw} {name}` must have one more parameter than `Property Get {name}`"
                        ),
                        get.name_span,
                        "Property Get declared here",
                    );
                    continue;
                }
                for (a, b) in get.params.iter().zip(&pi.params) {
                    if a.ty != b.ty && !a.ty.is_error() && !b.ty.is_error() {
                        self.error_with_label(
                            m,
                            codes::PROPERTY_PAIRING,
                            b.span,
                            format!(
                                "parameter `{}` has a different type than in `Property Get {name}`",
                                b.name
                            ),
                            a.span,
                            "declared here",
                        );
                    }
                }
                if value.ty != get.ret && !value.ty.is_error() && !get.ret.is_error() {
                    self.error_with_label(
                        m,
                        codes::PROPERTY_PAIRING,
                        value.span,
                        format!(
                            "the value type {} differs from the `Property Get {name}` type {}",
                            self.program.type_name(&value.ty),
                            self.program.type_name(&get.ret)
                        ),
                        get.name_span,
                        "Property Get declared here",
                    );
                }
            }
            if let (Some(l), Some(s)) = (g.let_, g.set) {
                let _ = (l, s);
            }
        }
    }

    fn check_with_events(&mut self, m: ModuleId) {
        let globals = self.program.modules[m.index()].globals.clone();
        let we: Vec<GlobalId> = globals
            .into_iter()
            .filter(|g| self.program.globals[g.index()].with_events)
            .collect();
        for g in &we {
            let gi = &self.program.globals[g.index()];
            let ok = matches!(&gi.ty, Type::Class(_) | Type::Library(_) | Type::Error);
            if !ok {
                self.error(
                    m,
                    codes::EVENTS,
                    gi.span,
                    format!(
                        "`WithEvents` variable `{}` must have a class type that raises events",
                        gi.name
                    ),
                );
            }
            if gi.auto_new {
                self.error(
                    m,
                    codes::EVENTS,
                    gi.span,
                    "a `WithEvents` variable cannot be declared `As New`",
                );
            }
            // MS-VBAL 5.2.3.1.2: the class must declare at least one event and
            // may not be the containing class.
            if let Type::Class(c) = &gi.ty {
                if *c == m {
                    self.spec_strict(
                        m,
                        codes::EVENTS,
                        gi.span,
                        "a `WithEvents` variable cannot have the type of its own class",
                    );
                } else if self.program.modules[c.index()].events.is_empty() {
                    self.spec_strict(
                        m,
                        codes::EVENTS,
                        gi.span,
                        format!(
                            "class `{}` declares no events",
                            self.program.modules[c.index()].name
                        ),
                    );
                }
            }
        }
        if we.is_empty() {
            return;
        }
        let mut handlers = Vec::new();
        let procs = self.program.modules[m.index()].procs.clone();
        for p in procs {
            let pi = &self.program.procs[p.index()];
            let pname = pi.name.to_string();
            if pi.kind != ProcKind::Sub {
                // MS-VBAL 5.3.1.8: an event handler is a subroutine.
                for g in &we {
                    let gi = &self.program.globals[g.index()];
                    let prefix = format!("{}_", gi.name);
                    if pname.len() > prefix.len()
                        && pname[..prefix.len()].eq_ignore_ascii_case(&prefix)
                        && self.handles_event(gi, &pname[prefix.len()..])
                    {
                        self.spec_strict(
                            m,
                            codes::EVENTS,
                            pi.name_span,
                            format!("the event handler `{}` must be a `Sub`", pi.name),
                        );
                    }
                }
                continue;
            }
            for g in &we {
                let gi = &self.program.globals[g.index()];
                let prefix = format!("{}_", gi.name);
                if pname.len() <= prefix.len()
                    || !pname[..prefix.len()].eq_ignore_ascii_case(&prefix)
                {
                    continue;
                }
                let event_name = &pname[prefix.len()..];
                if let Some(h) = self.match_handler(m, *g, p, event_name) {
                    handlers.push(h);
                }
                break;
            }
        }
        self.program.modules[m.index()].handlers = handlers;
    }

    /// Whether `event_name` names an event of the declared type of `gi`.
    fn handles_event(&self, gi: &GlobalInfo, event_name: &str) -> bool {
        let wanted = Name::new(event_name);
        match &gi.ty {
            Type::Class(c) => self.program.modules[c.index()]
                .events
                .iter()
                .any(|e| self.program.events[e.index()].name == wanted),
            _ => false,
        }
    }

    fn match_handler(
        &self,
        m: ModuleId,
        var: GlobalId,
        proc: ProcId,
        event_name: &str,
    ) -> Option<EventHandler> {
        let gi = &self.program.globals[var.index()];
        let pi = &self.program.procs[proc.index()];
        let wanted = Name::new(event_name);
        match &gi.ty {
            Type::Class(c) => {
                let ev = self.program.modules[c.index()]
                    .events
                    .iter()
                    .map(|e| &self.program.events[e.index()])
                    .find(|e| e.name == wanted);
                let Some(ev) = ev else {
                    self.error(
                        m,
                        codes::EVENTS,
                        pi.name_span,
                        format!(
                            "class `{}` has no event `{event_name}`",
                            self.program.modules[c.index()].name
                        ),
                    );
                    return None;
                };
                self.check_handler_signature(
                    m,
                    pi,
                    &ev.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                );
                // MS-VBAL 5.3.1.8: each handler parameter is passed the way
                // the event's parameter is.
                for (a, b) in pi.params.iter().zip(&ev.params) {
                    if a.by_val != b.by_val {
                        self.spec_strict(
                            m,
                            codes::EVENTS,
                            a.span,
                            format!(
                                "parameter `{}` of the handler is passed differently than in the event `{}`",
                                a.name, ev.name
                            ),
                        );
                    }
                }
                Some(EventHandler {
                    var,
                    event: EventRef::Project(ev.id),
                    proc,
                })
            }
            Type::Library(lt) => match self.provider.events(lt) {
                Some(list) => {
                    let Some(ev) = list
                        .iter()
                        .find(|e| e.name.eq_ignore_ascii_case(event_name))
                    else {
                        self.error(
                            m,
                            codes::EVENTS,
                            pi.name_span,
                            format!("type `{lt}` has no event `{event_name}`"),
                        );
                        return None;
                    };
                    self.check_handler_signature(
                        m,
                        pi,
                        &ev.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
                    );
                    Some(EventHandler {
                        var,
                        event: EventRef::Library {
                            ty: lt.clone(),
                            name: wanted,
                        },
                        proc,
                    })
                }
                None => {
                    self.report(
                        Severity::Info,
                        m,
                        codes::LATE_BOUND_LIBRARY_MEMBER,
                        pi.name_span,
                        format!("events of library type `{lt}` are not described; handler `{}` is not verified", pi.name),
                    );
                    Some(EventHandler {
                        var,
                        event: EventRef::Library {
                            ty: lt.clone(),
                            name: wanted,
                        },
                        proc,
                    })
                }
            },
            _ => None,
        }
    }

    fn check_handler_signature(&self, m: ModuleId, pi: &ProcInfo, event_params: &[Type]) {
        let same = pi.params.len() == event_params.len()
            && pi.params.iter().zip(event_params).all(|(a, b)| {
                a.ty == *b
                        || a.ty.is_error()
                        || b.is_error()
                        // Enums are `Long`s: a handler may declare either.
                        || (matches!(a.ty, Type::Long) && matches!(b, Type::LibEnum(_)))
                        || (matches!(a.ty, Type::LibEnum(_)) && matches!(b, Type::Long))
            });
        if !same {
            self.error(
                m,
                codes::EVENTS,
                pi.name_span,
                format!(
                    "the signature of `{}` does not match the event it handles",
                    pi.name
                ),
            );
        }
    }

    fn required_of_class(&self, c: ModuleId) -> Vec<Required> {
        let cm = &self.program.modules[c.index()];
        let mut out = Vec::new();
        for p in &cm.procs {
            let pi = &self.program.procs[p.index()];
            if pi.visibility != Visibility::Public {
                continue;
            }
            out.push(Required {
                name: pi.name.clone(),
                kind: Some(pi.kind),
                params: pi.params.iter().map(|p| p.ty.clone()).collect(),
                ret: pi.has_result.then(|| pi.ret.clone()),
            });
        }
        for g in &cm.globals {
            let gi = &self.program.globals[g.index()];
            if gi.visibility != Visibility::Public || gi.static_of.is_some() {
                continue;
            }
            out.push(Required {
                name: gi.name.clone(),
                kind: Some(ProcKind::PropertyGet),
                params: Vec::new(),
                ret: Some(gi.ty.clone()),
            });
            out.push(Required {
                name: gi.name.clone(),
                kind: Some(if gi.ty.is_object() {
                    ProcKind::PropertySet
                } else {
                    ProcKind::PropertyLet
                }),
                params: vec![gi.ty.clone()],
                ret: None,
            });
        }
        out
    }

    fn find_impl(&self, m: ModuleId, impl_name: &Name, kind: ProcKind) -> Option<ProcId> {
        match self.program.modules[m.index()].symbols.get(impl_name)?.0 {
            Symbol::Proc(p) => (self.program.procs[p.index()].kind == kind).then_some(p),
            Symbol::Property(g) => match kind {
                ProcKind::PropertyGet => g.get,
                ProcKind::PropertyLet => g.let_,
                ProcKind::PropertySet => g.set,
                _ => None,
            },
            _ => None,
        }
    }

    fn check_implements(&mut self, m: ModuleId) {
        let decls = self.implements_src[m.index()].clone();
        let mut result: Vec<ImplementsInfo> = Vec::new();
        for d in decls {
            let ty = self.resolve_type_ref(m, &d.interface, false);
            if result.iter().any(|r| r.interface == ty) && !ty.is_error() {
                self.error(
                    m,
                    codes::IMPLEMENTS,
                    d.span,
                    "this interface is implemented twice",
                );
                continue;
            }
            match &ty {
                Type::Error => {}
                Type::Class(c) => {
                    if *c == m {
                        self.error(
                            m,
                            codes::IMPLEMENTS,
                            d.span,
                            "a class cannot implement itself",
                        );
                        continue;
                    }
                    let cname = self.program.modules[c.index()].name.clone();
                    // MS-VBAL 5.2.4.2: no implemented-name prefix may begin another,
                    // and an interface member name cannot contain an underscore.
                    let prefix = format!("{cname}_").to_ascii_lowercase();
                    let clash = result.iter().any(|r| {
                        let Type::Class(o) = &r.interface else {
                            return false;
                        };
                        let other = format!("{}_", self.program.modules[o.index()].name)
                            .to_ascii_lowercase();
                        other.starts_with(&prefix) || prefix.starts_with(&other)
                    });
                    if clash {
                        self.spec_strict(
                            m,
                            codes::IMPLEMENTS,
                            d.span,
                            "an implemented interface name prefix cannot begin with another interface's prefix",
                        );
                        if self.strict_spec {
                            continue;
                        }
                    }
                    if let Some(bad) = self
                        .required_of_class(*c)
                        .into_iter()
                        .find(|r| r.name.to_string().contains('_'))
                    {
                        self.spec_strict(
                            m,
                            codes::IMPLEMENTS,
                            d.span,
                            format!(
                                "interface `{cname}` cannot be implemented: its member `{}` contains an underscore",
                                bad.name
                            ),
                        );
                        if self.strict_spec {
                            continue;
                        }
                    }
                    let mut members = Vec::new();
                    for req in self.required_of_class(*c) {
                        let kind = req.kind.unwrap_or(ProcKind::Sub);
                        let impl_name = Name::new(format!("{cname}_{}", req.name));
                        let Some(found) = self.find_impl(m, &impl_name, kind) else {
                            self.error_with_label(
                                m,
                                codes::IMPLEMENTS,
                                d.span,
                                format!(
                                    "`{}` does not implement `{}` of interface `{cname}` (expected `{impl_name}`)",
                                    self.program.modules[m.index()].name,
                                    req.name
                                ),
                                d.interface.span,
                                "interface named here",
                            );
                            continue;
                        };
                        let pi = &self.program.procs[found.index()];
                        let sig_ok = pi.params.len() == req.params.len()
                            && pi.params.iter().zip(&req.params).all(|(a, b)| a.ty == *b)
                            && match &req.ret {
                                Some(r) => pi.ret == *r,
                                None => true,
                            };
                        if !sig_ok {
                            self.error(
                                m,
                                codes::IMPLEMENTS,
                                pi.name_span,
                                format!(
                                    "the signature of `{}` does not match `{}` of interface `{cname}`",
                                    pi.name, req.name
                                ),
                            );
                            continue;
                        }
                        // The interface-side procedure (variables have none).
                        let iface_proc = self.program.modules[c.index()]
                            .procs
                            .iter()
                            .copied()
                            .find(|p| {
                                let q = &self.program.procs[p.index()];
                                q.name == req.name && q.kind == kind
                            });
                        if let Some(ip) = iface_proc {
                            members.push((ip, found));
                        }
                    }
                    result.push(ImplementsInfo {
                        interface: ty.clone(),
                        members,
                        span: d.span,
                    });
                }
                Type::Library(lt) => {
                    match self.provider.interface_members(lt) {
                        Some(list) => {
                            for mem in list {
                                self.check_library_member(m, d.span, lt, &mem);
                            }
                        }
                        None => self.report(
                            Severity::Info,
                            m,
                            codes::IMPLEMENTS,
                            d.span,
                            format!("members of library interface `{lt}` are not described; completeness is not verified"),
                        ),
                    }
                    result.push(ImplementsInfo {
                        interface: ty.clone(),
                        members: Vec::new(),
                        span: d.span,
                    });
                }
                other => {
                    self.error(
                        m,
                        codes::IMPLEMENTS,
                        d.span,
                        format!(
                            "`{}` cannot be implemented: only classes and library interfaces can",
                            self.program.type_name(other)
                        ),
                    );
                }
            }
        }
        self.program.modules[m.index()].implements = result;
    }

    fn check_library_member(
        &self,
        m: ModuleId,
        span: vba_source::Span,
        lt: &LibraryType,
        mem: &LibMember,
    ) {
        let kinds: &[ProcKind] = match mem.kind {
            LibMemberKind::Method if mem.is_sub => &[ProcKind::Sub],
            LibMemberKind::Method => &[ProcKind::Function],
            LibMemberKind::PropertyGet => &[ProcKind::PropertyGet],
            LibMemberKind::PropertyPut => &[ProcKind::PropertyLet],
            LibMemberKind::Property => &[ProcKind::PropertyGet, ProcKind::PropertyLet],
        };
        let impl_name = Name::new(format!("{}_{}", lt.name, mem.name));
        for k in kinds {
            if self.find_impl(m, &impl_name, *k).is_none() {
                self.error(
                    m,
                    codes::IMPLEMENTS,
                    span,
                    format!(
                        "`{}` does not implement `{}` of interface `{lt}` (expected `{impl_name}`)",
                        self.program.modules[m.index()].name,
                        mem.name
                    ),
                );
            }
        }
    }

    fn check_declares(&self) {
        // `PtrSafe` availability and the 64-bit `PtrSafe` requirement are
        // diagnosed by the parser (SYN010) from the same profile.
        for d in &self.program.declares {
            if d.lib.is_empty() {
                self.error(
                    d.module,
                    codes::DECLARE,
                    d.span,
                    "`Declare` needs a non-empty `Lib` name",
                );
            }
            // MS-VBAL 5.2.3.5: an alias `#n` names an ordinal, an integer
            // literal from 0 to 32,767.
            if let Some(rest) = d.alias.as_ref().and_then(|a| a.strip_prefix(b"#")) {
                let ok = !rest.is_empty()
                    && rest.iter().all(u8::is_ascii_digit)
                    && std::str::from_utf8(rest)
                        .ok()
                        .and_then(|t| t.parse::<u32>().ok())
                        .is_some_and(|n| n <= 32767);
                if !ok {
                    self.spec_strict(
                        d.module,
                        codes::DECLARE,
                        d.span,
                        "an ordinal alias must be `#` followed by an integer from 0 to 32767",
                    );
                }
            }
        }
    }
}

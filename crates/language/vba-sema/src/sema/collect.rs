//! Pass 1: allocate identifiers and fill module symbol tables.

use vba_ast::{
    AttributeDecl, DefType, ExprKind, Ident, Item, Literal, OptionKind, ProcKind, VarKeyword,
    Visibility,
};
use vba_profile::Feature;
use vba_project::ModuleKind;
use vba_source::Span;

use crate::codes;
use crate::fold::long_ptr;
use crate::program::*;
use crate::types::*;

use super::{CState, ConstSrc, Sema};

pub(crate) fn def_type_to_type(d: DefType, sema: &Sema<'_>) -> Option<Type> {
    Some(match d {
        DefType::Bool => Type::Boolean,
        DefType::Byte => Type::Byte,
        DefType::Cur => Type::Currency,
        DefType::Date => Type::Date,
        DefType::Dbl => Type::Double,
        DefType::Int => Type::Integer,
        DefType::Lng => Type::Long,
        DefType::LngLng => Type::LongLong,
        DefType::LngPtr => long_ptr(sema.profile()),
        DefType::Obj => Type::Object,
        DefType::Sng => Type::Single,
        DefType::Str => Type::String,
        DefType::Var => Type::Variant,
        DefType::Dec => return None,
    })
}

fn is_true_attr(a: &AttributeDecl) -> bool {
    match a.values.first().map(|e| &e.kind) {
        Some(ExprKind::Literal(Literal::True)) => true,
        Some(ExprKind::Literal(Literal::Integer { value, .. })) => value.is_some_and(|v| v != 0),
        _ => false,
    }
}

impl<'a> Sema<'a> {
    pub(crate) fn collect(&mut self) {
        for i in 0..self.program.modules.len() {
            self.collect_module(ModuleId(i as u32));
        }
    }

    /// Adds a name to a module's symbol table, reporting duplicates.
    fn add_symbol(&mut self, m: ModuleId, ident: &Ident, sym: Symbol, span: Span) -> bool {
        if ident.is_missing() {
            return false;
        }
        if let Some((_, prev)) = self.program.modules[m.index()].symbols.get(&ident.name) {
            let prev = *prev;
            self.error_with_label(
                m,
                codes::DUPLICATE_DECLARATION,
                ident.span,
                format!("`{}` is already declared in this module", ident.name),
                prev,
                "first declared here",
            );
            return false;
        }
        self.program.modules[m.index()]
            .symbols
            .insert(ident.name.clone(), (sym, span));
        true
    }

    fn collect_module(&mut self, m: ModuleId) {
        let asts = self.asts;
        let ast = &asts[m.index()];
        let kind = self.program.modules[m.index()].kind;
        let class_like = kind != ModuleKind::Standard;
        self.implements_src.push(Vec::new());
        // MS-VBAL 4.2: a module name has at most 31 characters.
        if self.program.modules[m.index()]
            .name
            .to_string()
            .chars()
            .count()
            > 31
        {
            self.spec_strict(
                m,
                codes::TYPE_DECLARATION,
                Span::empty(0),
                "a module name has at most 31 characters",
            );
        }

        let mut declared_any = false;
        let mut def_seen = [false; 26];
        let (mut seen_explicit, mut seen_base, mut seen_compare, mut seen_private) =
            (false, false, false, false);

        let mut seen_proc = false;
        for item in &ast.items {
            let decl_span = match item {
                Item::Implements(x) => Some(x.span),
                Item::Declare(x) => Some(x.span),
                Item::Event(x) => Some(x.span),
                Item::Const(x) => Some(x.span),
                Item::Var(x) => Some(x.span),
                Item::Type(x) => Some(x.span),
                Item::Enum(x) => Some(x.span),
                Item::DefType(x) => Some(x.span),
                _ => None,
            };
            if let Some(span) = decl_span
                && seen_proc
            {
                self.error(
                    m,
                    codes::OPTION_PLACEMENT,
                    span,
                    "declarations must precede all procedures (MS-VBAL 5.1)",
                );
            }
            match item {
                Item::Option(o) => {
                    if declared_any {
                        self.error(
                            m,
                            codes::OPTION_PLACEMENT,
                            o.span,
                            "`Option` statements must precede all declarations and procedures",
                        );
                    }
                    let Some(k) = o.kind else { continue };
                    let (seen, what) = match k {
                        OptionKind::Explicit => (&mut seen_explicit, "Option Explicit"),
                        OptionKind::Base(_) => (&mut seen_base, "Option Base"),
                        OptionKind::Compare(_) => (&mut seen_compare, "Option Compare"),
                        OptionKind::PrivateModule => (&mut seen_private, "Option Private Module"),
                    };
                    if *seen {
                        self.error(
                            m,
                            codes::OPTION_PLACEMENT,
                            o.span,
                            format!("duplicate `{what}`"),
                        );
                        continue;
                    }
                    *seen = true;
                    let opts = &mut self.program.modules[m.index()].options;
                    match k {
                        OptionKind::Explicit => opts.explicit = true,
                        OptionKind::Base(b) => opts.base = b,
                        OptionKind::Compare(c) => opts.compare = c,
                        OptionKind::PrivateModule => {
                            opts.private_module = true;
                            if class_like {
                                self.error(
                                    m,
                                    codes::OPTION_PLACEMENT,
                                    o.span,
                                    "`Option Private Module` is only valid in standard modules",
                                );
                            }
                        }
                    }
                }
                Item::Attribute(a) => {
                    // MS-VBAL 4.2: classes in source form are not creatable and are
                    // not global-namespace classes.
                    if class_like
                        && a.object.is_none()
                        && (a.name.name.eq_ignore_case("VB_Creatable")
                            || a.name.name.eq_ignore_case("VB_GlobalNameSpace"))
                        && is_true_attr(a)
                    {
                        self.spec_strict(
                            m,
                            codes::TYPE_DECLARATION,
                            a.span,
                            format!("`{}` must be False for a class module", a.name.name),
                        );
                    }
                    if a.object.is_none()
                        && a.name.name.eq_ignore_case("VB_PredeclaredId")
                        && is_true_attr(a)
                    {
                        self.program.modules[m.index()].predeclared = true;
                    }
                }
                Item::DefType(d) => {
                    let Some(ty) = def_type_to_type(d.ty, self) else {
                        self.error(
                            m,
                            codes::UNSUPPORTED,
                            d.span,
                            "`DefDec` is reserved by the specification and not supported",
                        );
                        continue;
                    };
                    if d.ty == DefType::LngPtr && !self.profile().supports(Feature::LongPtr) {
                        self.error(
                            m,
                            codes::TYPE_DECLARATION,
                            d.span,
                            "`DefLngPtr` is not available in this language profile",
                        );
                    }
                    if d.ty == DefType::LngLng && !self.profile().supports(Feature::LongLong) {
                        self.error(
                            m,
                            codes::TYPE_DECLARATION,
                            d.span,
                            "`DefLngLng` needs a 64-bit VBA7 profile",
                        );
                    }
                    // MS-VBAL 5.2.2: a descending span is the same as the
                    // ascending one, and no letter may be named twice in a module.
                    let mut overlap = false;
                    let opts = &mut self.program.modules[m.index()].options;
                    for &(lo, hi) in &d.ranges {
                        for c in lo.min(hi)..=lo.max(hi) {
                            if c.is_ascii_uppercase() {
                                let i = (c - b'A') as usize;
                                overlap |= def_seen[i];
                                def_seen[i] = true;
                                opts.def_types[i] = ty.clone();
                            }
                        }
                    }
                    if overlap {
                        self.spec_strict(
                            m,
                            codes::TYPE_DECLARATION,
                            d.span,
                            "a letter is named by more than one Def directive range",
                        );
                    }
                }
                Item::Implements(i) => {
                    declared_any = true;
                    if !class_like {
                        self.error(
                            m,
                            codes::IMPLEMENTS,
                            i.span,
                            "`Implements` is only valid in class modules",
                        );
                    } else if !self.profile().supports(Feature::Implements) {
                        self.error(
                            m,
                            codes::IMPLEMENTS,
                            i.span,
                            "`Implements` is not available in this language profile",
                        );
                    } else {
                        self.implements_src[m.index()].push(i);
                    }
                }
                Item::Declare(d) => {
                    declared_any = true;
                    let id = DeclareId(self.program.declares.len() as u32);
                    let vis = match d.visibility {
                        Some(Visibility::Private) => Visibility::Private,
                        _ => Visibility::Public,
                    };
                    if class_like && vis != Visibility::Private {
                        self.error(
                            m,
                            codes::DECLARE,
                            d.span,
                            "`Declare` in a class module must be Private",
                        );
                    }
                    self.program.declares.push(DeclareInfo {
                        id,
                        module: m,
                        name: d.name.name.clone(),
                        visibility: vis,
                        is_function: d.kind == vba_ast::DeclareKind::Function,
                        lib: d.lib.clone(),
                        alias: d.alias.clone(),
                        ptr_safe: d.ptr_safe,
                        params: Vec::new(),
                        ret: Type::Variant,
                        span: d.span,
                    });
                    self.declare_src.push(d);
                    self.program.modules[m.index()].declares.push(id);
                    self.add_symbol(m, &d.name, Symbol::Declare(id), d.name.span);
                }
                Item::Event(e) => {
                    declared_any = true;
                    if !class_like {
                        self.error(
                            m,
                            codes::EVENTS,
                            e.span,
                            "`Event` is only valid in class modules",
                        );
                        continue;
                    }
                    if !self.profile().supports(Feature::Events) {
                        self.error(
                            m,
                            codes::EVENTS,
                            e.span,
                            "`Event` is not available in this language profile",
                        );
                    }
                    if e.name.name.to_string().contains('_') {
                        self.spec_strict(
                            m,
                            codes::EVENTS,
                            e.name.span,
                            "an event name cannot contain an underscore",
                        );
                    }
                    let id = EventId(self.program.events.len() as u32);
                    self.program.events.push(EventInfo {
                        id,
                        module: m,
                        name: e.name.name.clone(),
                        params: Vec::new(),
                        span: e.span,
                    });
                    self.event_src.push(e);
                    self.program.modules[m.index()].events.push(id);
                    self.add_symbol(m, &e.name, Symbol::Event(id), e.name.span);
                }
                Item::Const(c) => {
                    declared_any = true;
                    let vis = match c.visibility {
                        Some(Visibility::Global) => Visibility::Public,
                        Some(v) => v,
                        None => Visibility::Private,
                    };
                    for it in &c.items {
                        let id = ConstId(self.program.consts.len() as u32);
                        self.program.consts.push(ConstInfo {
                            id,
                            module: m,
                            name: it.name.name.clone(),
                            ty: Type::Variant,
                            value: crate::bound::Constant::Value(vba_value::Variant::Empty),
                            visibility: vis,
                            span: it.span,
                        });
                        self.const_src.push(ConstSrc {
                            module: m,
                            item: it,
                        });
                        self.const_state.borrow_mut().push(CState::Pending);
                        self.program.modules[m.index()].consts.push(id);
                        self.add_symbol(m, &it.name, Symbol::Const(id), it.name.span);
                    }
                }
                Item::Var(v) => {
                    declared_any = true;
                    if v.keyword == VarKeyword::Static {
                        self.error(
                            m,
                            codes::TYPE_DECLARATION,
                            v.span,
                            "`Static` is only valid inside procedures",
                        );
                    }
                    if v.keyword == VarKeyword::Global && class_like {
                        self.error(
                            m,
                            codes::TYPE_DECLARATION,
                            v.span,
                            "`Global` is not valid in a class module",
                        );
                    }
                    if v.with_events {
                        if !class_like {
                            self.error(
                                m,
                                codes::EVENTS,
                                v.span,
                                "`WithEvents` is only valid in class modules",
                            );
                        } else if !self.profile().supports(Feature::Events) {
                            self.error(
                                m,
                                codes::EVENTS,
                                v.span,
                                "`WithEvents` is not available in this language profile",
                            );
                        }
                    }
                    let vis = match v.visibility {
                        Some(Visibility::Global) | Some(Visibility::Public) => Visibility::Public,
                        _ => Visibility::Private,
                    };
                    for d in &v.declarators {
                        let id = GlobalId(self.program.globals.len() as u32);
                        self.program.globals.push(GlobalInfo {
                            id,
                            module: m,
                            name: d.name.name.clone(),
                            ty: Type::Variant,
                            visibility: vis,
                            with_events: v.with_events,
                            auto_new: false,
                            static_of: None,
                            fixed_bounds: None,
                            span: d.span,
                        });
                        self.global_src.push(Some(d));
                        self.program.modules[m.index()].globals.push(id);
                        self.add_symbol(m, &d.name, Symbol::Var(id), d.name.span);
                    }
                }
                Item::Type(t) => {
                    declared_any = true;
                    if t.members.is_empty() && t.directives.is_empty() {
                        self.spec_strict(
                            m,
                            codes::TYPE_DECLARATION,
                            t.name.span,
                            "a user-defined type needs at least one member",
                        );
                    }
                    let id = UdtId(self.program.udts.len() as u32);
                    self.program.udts.push(UdtInfo {
                        id,
                        module: m,
                        name: t.name.name.clone(),
                        visibility: vis_pub(t.visibility),
                        fields: Vec::new(),
                        span: t.span,
                    });
                    self.udt_src.push(t);
                    self.program.modules[m.index()].udts.push(id);
                    self.add_symbol(m, &t.name, Symbol::Udt(id), t.name.span);
                }
                Item::Enum(e) => {
                    declared_any = true;
                    if class_like && e.visibility == Some(Visibility::Global) {
                        self.spec_strict(
                            m,
                            codes::TYPE_DECLARATION,
                            e.name.span,
                            "`Global Enum` is not allowed in a class module",
                        );
                    }
                    if e.members.is_empty() {
                        self.spec_strict(
                            m,
                            codes::TYPE_DECLARATION,
                            e.name.span,
                            "an enum needs at least one member",
                        );
                    }
                    let id = EnumId(self.program.enums.len() as u32);
                    self.program.enums.push(EnumInfo {
                        id,
                        module: m,
                        name: e.name.name.clone(),
                        visibility: vis_pub(e.visibility),
                        members: Vec::new(),
                        span: e.span,
                    });
                    self.enum_src.push(e);
                    self.enum_state
                        .borrow_mut()
                        .push(vec![CState::Pending; e.members.len()]);
                    self.program.modules[m.index()].enums.push(id);
                    self.add_symbol(m, &e.name, Symbol::Enum(id), e.name.span);
                    for (idx, mem) in e.members.iter().enumerate() {
                        self.add_symbol(
                            m,
                            &mem.name,
                            Symbol::EnumMember(id, idx as u32),
                            mem.name.span,
                        );
                    }
                }
                Item::Proc(p) => {
                    declared_any = true;
                    seen_proc = true;
                    let id = ProcId(self.program.procs.len() as u32);
                    let vis = match p.visibility {
                        Some(Visibility::Global) | None => Visibility::Public,
                        Some(v) => v,
                    };
                    if p.visibility == Some(Visibility::Global) && class_like {
                        self.spec_strict(
                            m,
                            codes::PROCEDURE_RULE,
                            p.name.span,
                            "`Global` is not allowed on a procedure in a class module",
                        );
                    }
                    if vis == Visibility::Friend && !class_like {
                        self.error(
                            m,
                            codes::TYPE_DECLARATION,
                            p.span,
                            "`Friend` is only valid in class modules",
                        );
                    }
                    let has_result = matches!(p.kind, ProcKind::Function | ProcKind::PropertyGet);
                    self.program.procs.push(ProcInfo {
                        id,
                        module: m,
                        name: p.name.name.clone(),
                        kind: p.kind,
                        visibility: vis,
                        is_static: p.is_static,
                        params: Vec::new(),
                        ret: Type::Variant,
                        has_result,
                        locals: Vec::new(),
                        result_local: None,
                        labels: Vec::new(),
                        body: Vec::new(),
                        span: p.span,
                        name_span: p.name.span,
                    });
                    self.proc_src.push(p);
                    self.program.modules[m.index()].procs.push(id);
                    self.add_proc_symbol(m, id, p);
                    for a in p.attributes() {
                        if a.name.name.eq_ignore_case("VB_UserMemId")
                            && class_like
                            && matches!(
                                a.values.first().map(|e| &e.kind),
                                Some(ExprKind::Unary { op: vba_ast::UnaryOp::Neg, operand })
                                    if matches!(
                                        operand.kind,
                                        ExprKind::Literal(Literal::Integer { value: Some(4), .. })
                                    )
                            )
                        {
                            self.program.modules[m.index()].enumerator_member =
                                Some(p.name.name.clone());
                        }
                        if a.name.name.eq_ignore_case("VB_UserMemId")
                            && matches!(
                                a.values.first().map(|e| &e.kind),
                                Some(ExprKind::Literal(Literal::Integer { value: Some(0), .. }))
                            )
                            && class_like
                        {
                            self.program.modules[m.index()].default_member =
                                Some(p.name.name.clone());
                        }
                    }
                }
                Item::Error(_) => {
                    self.program.modules[m.index()].has_syntax_errors = true;
                }
                Item::Directive(_) | Item::Inactive(_) => {}
            }
        }
        if kind != ModuleKind::Class {
            self.program.modules[m.index()].predeclared = true;
        }
        if kind == ModuleKind::Standard {
            self.program.modules[m.index()].predeclared = false;
        }
    }

    fn add_proc_symbol(&mut self, m: ModuleId, id: ProcId, p: &'a vba_ast::ProcDecl) {
        if p.name.is_missing() {
            return;
        }
        let key = p.name.name.clone();
        let existing = self.program.modules[m.index()].symbols.get(&key).copied();
        let sym = match p.kind {
            ProcKind::Sub | ProcKind::Function => Symbol::Proc(id),
            _ => {
                let mut g = match existing {
                    Some((Symbol::Property(g), _)) => g,
                    _ => PropGroup::default(),
                };
                let slot = match p.kind {
                    ProcKind::PropertyGet => &mut g.get,
                    ProcKind::PropertyLet => &mut g.let_,
                    _ => &mut g.set,
                };
                if slot.is_some() {
                    self.dup_error(m, p, existing.map(|e| e.1));
                    return;
                }
                *slot = Some(id);
                if let Some((Symbol::Property(_), span)) = existing {
                    self.program.modules[m.index()]
                        .symbols
                        .insert(key, (Symbol::Property(g), span));
                    return;
                }
                Symbol::Property(g)
            }
        };
        if let Some((_, prev)) = existing {
            self.dup_error(m, p, Some(prev));
            return;
        }
        self.program.modules[m.index()]
            .symbols
            .insert(key, (sym, p.name.span));
    }

    fn dup_error(&self, m: ModuleId, p: &vba_ast::ProcDecl, prev: Option<Span>) {
        match prev {
            Some(prev) => self.error_with_label(
                m,
                codes::DUPLICATE_DECLARATION,
                p.name.span,
                format!("`{}` is already declared in this module", p.name.name),
                prev,
                "first declared here",
            ),
            None => self.error(
                m,
                codes::DUPLICATE_DECLARATION,
                p.name.span,
                format!("`{}` is already declared in this module", p.name.name),
            ),
        }
    }
}

fn vis_pub(v: Option<Visibility>) -> Visibility {
    match v {
        Some(Visibility::Private) => Visibility::Private,
        _ => Visibility::Public,
    }
}

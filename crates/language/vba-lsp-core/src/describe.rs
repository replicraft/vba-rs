//! Names and hover descriptions of project declarations.

use vba_ast::{ProcKind, Visibility};
use vba_project::ModuleKind;
use vba_sema::bound::Constant;
use vba_sema::{LocalKind, ModuleId, ParamInfo, ProcInfo, Type};
use vba_value::Variant;

use crate::project::{Decl, Project};

fn vis_text(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "Public",
        Visibility::Private => "Private",
        Visibility::Friend => "Friend",
        Visibility::Global => "Global",
    }
}

/// Source-like text of a constant value.
pub(crate) fn constant_text(c: &Constant) -> String {
    match c {
        Constant::DateText(b) | Constant::YearlessDate(b) => {
            format!("#{}#", String::from_utf8_lossy(b))
        }
        Constant::Value(v) => match v {
            Variant::Empty => "Empty".to_string(),
            Variant::Null => "Null".to_string(),
            Variant::Boolean(b) => if *b { "True" } else { "False" }.to_string(),
            Variant::Byte(n) => n.to_string(),
            Variant::Integer(n) => n.to_string(),
            Variant::Long(n) => n.to_string(),
            Variant::LongLong(n) => n.to_string(),
            Variant::Single(n) => n.to_string(),
            Variant::Double(n) => n.to_string(),
            Variant::Currency(n) => n.to_string(),
            Variant::Date(n) => format!("#{n}#"),
            Variant::Decimal(n) => n.to_string(),
            Variant::String(s) => format!("\"{s}\""),
            other => format!("{other:?}"),
        },
    }
}

impl Project {
    /// The declared name of a declaration, as written.
    pub fn decl_name(&self, d: &Decl) -> String {
        let prog = &self.sema.program;
        match d {
            Decl::Proc(m, l) => self
                .group(*m, l)
                .first()
                .map_or_else(|| l.clone(), |p| p.name.to_string_lossy()),
            Decl::Declare(x) => prog.declares[x.index()].name.to_string_lossy(),
            Decl::Global(x) => prog.global(*x).name.to_string_lossy(),
            Decl::Const(x) => prog.consts[x.index()].name.to_string_lossy(),
            Decl::Udt(x) => prog.udts[x.index()].name.to_string_lossy(),
            Decl::Field(u, i) => prog.udts[u.index()].fields[*i as usize]
                .name
                .to_string_lossy(),
            Decl::Enum(x) => prog.enums[x.index()].name.to_string_lossy(),
            Decl::EnumMember(e, i) => prog.enums[e.index()].members[*i as usize]
                .0
                .to_string_lossy(),
            Decl::Event(x) => prog.events[x.index()].name.to_string_lossy(),
            Decl::Local(p, i) => prog.proc(*p).locals[*i as usize].name.to_string_lossy(),
            Decl::Module(m) => prog.module(*m).name.to_string_lossy(),
        }
    }

    /// The module that declares `d`.
    pub fn decl_module(&self, d: &Decl) -> ModuleId {
        let prog = &self.sema.program;
        match d {
            Decl::Proc(m, _) | Decl::Module(m) => *m,
            Decl::Declare(x) => prog.declares[x.index()].module,
            Decl::Global(x) => prog.global(*x).module,
            Decl::Const(x) => prog.consts[x.index()].module,
            Decl::Udt(x) | Decl::Field(x, _) => prog.udts[x.index()].module,
            Decl::Enum(x) | Decl::EnumMember(x, _) => prog.enums[x.index()].module,
            Decl::Event(x) => prog.events[x.index()].module,
            Decl::Local(p, _) => prog.proc(*p).module,
        }
    }

    fn param_text(&self, p: &ParamInfo) -> String {
        let mut s = String::new();
        if p.optional {
            s.push_str("Optional ");
        }
        if p.param_array {
            s.push_str("ParamArray ");
        }
        if p.by_val {
            s.push_str("ByVal ");
        }
        s.push_str(&p.name.to_string_lossy());
        let ty = match (&p.ty, p.is_array) {
            (Type::Array { elem, .. }, true) => {
                s.push_str("()");
                self.type_name(elem)
            }
            (t, _) => self.type_name(t),
        };
        s.push_str(" As ");
        s.push_str(&ty);
        if let Some(c) = &p.default {
            s.push_str(" = ");
            s.push_str(&constant_text(c));
        }
        s
    }

    fn params_text(&self, ps: &[ParamInfo]) -> String {
        ps.iter()
            .map(|p| self.param_text(p))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn proc_text(&self, p: &ProcInfo) -> String {
        let kw = match p.kind {
            ProcKind::Sub => "Sub",
            ProcKind::Function => "Function",
            ProcKind::PropertyGet => "Property Get",
            ProcKind::PropertyLet => "Property Let",
            ProcKind::PropertySet => "Property Set",
        };
        let mut s = format!(
            "{}{} {kw} {}({})",
            vis_text(p.visibility),
            if p.is_static { " Static" } else { "" },
            p.name,
            self.params_text(&p.params)
        );
        if p.has_result || p.kind == ProcKind::PropertyGet {
            s.push_str(" As ");
            s.push_str(&self.type_name(&p.ret));
        }
        s
    }

    /// The declaration text of a procedure group, one line per member.
    pub fn proc_group_text(&self, m: ModuleId, lname: &str) -> Vec<String> {
        self.group(m, lname)
            .iter()
            .map(|p| self.proc_text(p))
            .collect()
    }

    /// A hover description of a declaration: the declaration text plus
    /// explanatory lines.
    pub fn describe(&self, d: &Decl) -> (String, Vec<String>) {
        let prog = &self.sema.program;
        let module_line = |m: ModuleId| {
            let mi = prog.module(m);
            let kind = match mi.kind {
                ModuleKind::Standard => "Module",
                ModuleKind::Class => "Class module",
                ModuleKind::Document => "Document module",
                ModuleKind::Designer => "Designer module",
                _ => "Module",
            };
            format!("{kind} `{}`", mi.name)
        };
        let m = self.decl_module(d);
        match d {
            Decl::Proc(m, l) => (
                self.proc_group_text(*m, l).join("\n"),
                vec![module_line(*m)],
            ),
            Decl::Declare(x) => {
                let di = &prog.declares[x.index()];
                let mut s = format!(
                    "{} Declare {}{} {} Lib \"{}\"",
                    vis_text(di.visibility),
                    if di.ptr_safe { "PtrSafe " } else { "" },
                    if di.is_function { "Function" } else { "Sub" },
                    di.name,
                    String::from_utf8_lossy(&di.lib)
                );
                if let Some(a) = &di.alias {
                    s.push_str(&format!(" Alias \"{}\"", String::from_utf8_lossy(a)));
                }
                s.push_str(&format!(" ({})", self.params_text(&di.params)));
                if di.is_function {
                    s.push_str(&format!(" As {}", self.type_name(&di.ret)));
                }
                (s, vec![module_line(m)])
            }
            Decl::Global(x) => {
                let g = prog.global(*x);
                (
                    format!(
                        "{}{} {} As {}{}",
                        vis_text(g.visibility),
                        if g.with_events { " WithEvents" } else { "" },
                        g.name,
                        if g.auto_new { "New " } else { "" },
                        self.type_name(&g.ty)
                    ),
                    vec![module_line(m)],
                )
            }
            Decl::Const(x) => {
                let c = &prog.consts[x.index()];
                (
                    format!(
                        "{} Const {} As {} = {}",
                        vis_text(c.visibility),
                        c.name,
                        self.type_name(&c.ty),
                        constant_text(&c.value)
                    ),
                    vec![module_line(m)],
                )
            }
            Decl::Local(p, i) => {
                let pi = prog.proc(*p);
                let l = &pi.locals[*i as usize];
                let what = match l.kind {
                    LocalKind::Param(_) => "parameter",
                    _ => "local variable",
                };
                (
                    format!("({what}) {} As {}", l.name, self.type_name(&l.ty)),
                    vec![format!("In `{}`", pi.name)],
                )
            }
            Decl::Udt(x) => {
                let u = &prog.udts[x.index()];
                let mut s = format!("{} Type {}", vis_text(u.visibility), u.name);
                for f in &u.fields {
                    s.push_str(&format!("\n    {} As {}", f.name, self.type_name(&f.ty)));
                }
                s.push_str("\nEnd Type");
                (s, vec![module_line(m)])
            }
            Decl::Field(x, i) => {
                let u = &prog.udts[x.index()];
                let f = &u.fields[*i as usize];
                (
                    format!("(field) {} As {}", f.name, self.type_name(&f.ty)),
                    vec![format!("Member of `{}`", u.name), module_line(m)],
                )
            }
            Decl::Enum(x) => {
                let e = &prog.enums[x.index()];
                let mut s = format!("{} Enum {}", vis_text(e.visibility), e.name);
                for (n, v) in &e.members {
                    s.push_str(&format!("\n    {n} = {v}"));
                }
                s.push_str("\nEnd Enum");
                (s, vec![module_line(m)])
            }
            Decl::EnumMember(x, i) => {
                let e = &prog.enums[x.index()];
                let (n, v) = &e.members[*i as usize];
                (
                    format!("(enum member) {n} = {v}"),
                    vec![format!("Member of `{}`", e.name), module_line(m)],
                )
            }
            Decl::Event(x) => {
                let e = &prog.events[x.index()];
                (
                    format!("Event {}({})", e.name, self.params_text(&e.params)),
                    vec![module_line(m)],
                )
            }
            Decl::Module(x) => {
                let mi = prog.module(*x);
                let kw = if mi.kind == ModuleKind::Standard {
                    "Module"
                } else {
                    "Class"
                };
                (format!("{kw} {}", mi.name), Vec::new())
            }
        }
    }
}

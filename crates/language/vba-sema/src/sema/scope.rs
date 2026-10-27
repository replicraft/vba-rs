//! Module and project scope lookup (MS-VBAL §5.6 simple name
//! resolution, the module/project part).

use vba_ast::{CompareMode, Ident, Name, Visibility};
use vba_profile::Profile;
use vba_project::ModuleKind;

use crate::bound::Constant;
use crate::fold::{ConstLookup, ConstScope};
use crate::program::Symbol;
use crate::provider::LibGlobal;
use crate::types::*;

use super::Sema;

/// The outcome of looking a name up in module and project scope.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Found {
    /// A declared symbol and its defining module.
    Sym(ModuleId, Symbol),
    /// The name of a module.
    Module(ModuleId),
    /// Several modules export the name.
    Ambiguous(Vec<ModuleId>),
    /// Not found at project level.
    None,
}

/// The outcome of looking a type name up.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TypeFound {
    Enum(EnumId),
    Udt(UdtId),
    Class(ModuleId),
    Ambiguous(Vec<ModuleId>),
    None,
}

impl Sema<'_> {
    /// Visibility of a symbol seen from another module.
    fn symbol_visibility(&self, sym: Symbol) -> Option<Visibility> {
        Some(match sym {
            Symbol::Var(g) => self.program.globals[g.index()].visibility,
            Symbol::Const(c) => self.program.consts[c.index()].visibility,
            Symbol::Proc(p) => self.program.procs[p.index()].visibility,
            // The accessors of a property may differ (`Private Get` with a
            // `Public Let`): the name is as visible as its widest accessor.
            Symbol::Property(g) => [g.get, g.let_, g.set]
                .into_iter()
                .flatten()
                .map(|p| self.program.procs[p.index()].visibility)
                .max_by_key(|v| match v {
                    Visibility::Global => 3,
                    Visibility::Public => 2,
                    Visibility::Friend => 1,
                    Visibility::Private => 0,
                })?,
            Symbol::Declare(d) => self.program.declares[d.index()].visibility,
            Symbol::Enum(e) | Symbol::EnumMember(e, _) => self.program.enums[e.index()].visibility,
            Symbol::Udt(u) => self.program.udts[u.index()].visibility,
            Symbol::Event(_) => Visibility::Public,
        })
    }

    /// Whether `sym` of module `owner` is visible as a project-wide global
    /// name: public symbols of standard modules, and public types and enums
    /// of any module.
    pub(crate) fn is_global_symbol(&self, owner: ModuleId, sym: Symbol) -> bool {
        let vis = self.symbol_visibility(sym);
        if !matches!(vis, Some(Visibility::Public) | Some(Visibility::Global)) {
            return false;
        }
        match self.program.modules[owner.index()].kind {
            ModuleKind::Standard => true,
            _ => matches!(
                sym,
                Symbol::Enum(_) | Symbol::EnumMember(..) | Symbol::Udt(_)
            ),
        }
    }

    /// The one public declaration called `name` in the whole project, the
    /// target of a `Project.Name` qualification. `None` when there is none
    /// or when several modules declare it.
    pub(crate) fn project_symbol(&self, name: &Name) -> Option<(ModuleId, Symbol)> {
        let mut hits = self.program.modules.iter().filter_map(|m| {
            let (sym, _) = m.symbols.get(name)?;
            self.is_global_symbol(m.id, *sym).then_some((m.id, *sym))
        });
        let first = hits.next()?;
        hits.next().is_none().then_some(first)
    }

    /// Looks `name` up in the scope of module `cur`: its own symbols, then
    /// the public symbols of the other modules, then module names.
    pub(crate) fn lookup_module_scope(&self, cur: ModuleId, name: &Name) -> Found {
        if let Some((sym, _)) = self.program.modules[cur.index()].symbols.get(name) {
            return Found::Sym(cur, *sym);
        }
        if let Some(found) = self.friend_symbol(cur, name) {
            return found;
        }
        let mut hits: Vec<(ModuleId, Symbol)> = Vec::new();
        for m in &self.program.modules {
            if m.id == cur {
                continue;
            }
            if let Some((sym, _)) = m.symbols.get(name)
                && self.is_global_symbol(m.id, *sym)
            {
                hits.push((m.id, *sym));
            }
        }
        match hits.len() {
            0 => {}
            1 => return Found::Sym(hits[0].0, hits[0].1),
            _ => return Found::Ambiguous(hits.iter().map(|h| h.0).collect()),
        }
        if let Some(m) = self.module_ids.get(name) {
            return Found::Module(*m);
        }
        Found::None
    }

    /// The declaration of the friend target (see `Options::friend`) that
    /// `name` stands for in module `cur`.
    fn friend_symbol(&self, cur: ModuleId, name: &Name) -> Option<Found> {
        let (friend, target) = self.friend?;
        if cur != friend {
            return None;
        }
        let (sym, _) = self.program.modules[target.index()].symbols.get(name)?;
        let usable = match self.program.modules[target.index()].kind {
            ModuleKind::Standard => true,
            _ => matches!(
                sym,
                Symbol::Const(_) | Symbol::Enum(_) | Symbol::EnumMember(..) | Symbol::Udt(_)
            ),
        };
        usable.then_some(Found::Sym(target, *sym))
    }

    /// Looks a name up in the type namespace.
    pub(crate) fn lookup_type_name(&self, cur: ModuleId, name: &Name) -> TypeFound {
        let as_type = |s: Symbol| match s {
            Symbol::Enum(e) => Some(TypeFound::Enum(e)),
            Symbol::Udt(u) => Some(TypeFound::Udt(u)),
            _ => None,
        };
        if let Some((sym, _)) = self.program.modules[cur.index()].symbols.get(name)
            && let Some(t) = as_type(*sym)
        {
            return t;
        }
        if let Some((friend, target)) = self.friend
            && cur == friend
            && let Some((sym, _)) = self.program.modules[target.index()].symbols.get(name)
            && let Some(t) = as_type(*sym)
        {
            return t;
        }
        let mut hits: Vec<(ModuleId, TypeFound)> = Vec::new();
        for m in &self.program.modules {
            if m.id == cur {
                continue;
            }
            if let Some((sym, _)) = m.symbols.get(name)
                && let Some(t) = as_type(*sym)
                && self.is_global_symbol(m.id, *sym)
            {
                hits.push((m.id, t));
            }
        }
        match hits.len() {
            0 => {}
            1 => return hits.remove(0).1,
            _ => return TypeFound::Ambiguous(hits.iter().map(|h| h.0).collect()),
        }
        if let Some(m) = self.module_ids.get(name)
            && self.program.modules[m.index()].is_class_like()
        {
            return TypeFound::Class(*m);
        }
        TypeFound::None
    }
}

/// Constant-evaluation scope of a module.
pub(crate) struct ModuleScope<'s, 'a> {
    pub sema: &'s Sema<'a>,
    pub module: ModuleId,
}

impl ModuleScope<'_, '_> {
    fn eval_member(&self, e: EnumId, i: usize) -> Option<i32> {
        self.sema.check_enum_forward_reference(e, i);
        self.sema.eval_enum_member(e, i)
    }

    fn symbol_const(&self, owner: ModuleId, sym: Symbol) -> ConstLookup {
        if owner != self.module && matches!(sym, Symbol::Const(_) | Symbol::EnumMember(..)) {
            self.sema
                .module_deps
                .borrow_mut()
                .insert((self.module.0, owner.0));
        }
        match sym {
            Symbol::Const(c) => match self.sema.eval_const(c) {
                Some((v, t)) => ConstLookup::Found(v, t),
                None => ConstLookup::Failed,
            },
            Symbol::EnumMember(e, i) => match self.eval_member(e, i as usize) {
                Some(v) => {
                    ConstLookup::Found(Constant::Value(vba_value::Variant::Long(v)), Type::Enum(e))
                }
                None => ConstLookup::Failed,
            },
            Symbol::Var(_) => ConstLookup::NotConstant("a variable".into()),
            Symbol::Proc(_) | Symbol::Property(_) => ConstLookup::NotConstant("a procedure".into()),
            Symbol::Declare(_) => ConstLookup::NotConstant("a Declare".into()),
            Symbol::Enum(_) | Symbol::Udt(_) => ConstLookup::NotConstant("a type".into()),
            Symbol::Event(_) => {
                let _ = owner;
                ConstLookup::NotConstant("an event".into())
            }
        }
    }

    /// `Enum.Member`, `Library.Member` and `Library.Enum.Member` of a
    /// library (`first` is the first identifier of the path).
    fn library_path(&self, first: &Name, rest: &[&Ident]) -> ConstLookup {
        let provider = self.sema.provider;
        let enum_member = |lt: &LibraryType, m: &Name| -> ConstLookup {
            let text = m.to_string_lossy();
            match provider
                .enum_members(lt)
                .and_then(|ms| ms.into_iter().find(|(n, _)| n.eq_ignore_ascii_case(&text)))
            {
                Some((_, v)) => ConstLookup::Found(
                    Constant::Value(vba_value::Variant::Long(v as i32)),
                    Type::LibEnum(lt.clone()),
                ),
                None => ConstLookup::Missing,
            }
        };
        let head = first.to_string_lossy();
        match rest {
            [member] => {
                if let Some(Type::LibEnum(lt)) = provider.resolve_declared(None, &head) {
                    return enum_member(&lt, &member.name);
                }
                self.library(&member.name)
            }
            [en, member] if provider.has_library(&head) => {
                match provider.resolve_declared(Some(&head), &en.name.to_string_lossy()) {
                    Some(Type::LibEnum(lt)) => enum_member(&lt, &member.name),
                    _ => ConstLookup::Missing,
                }
            }
            _ => ConstLookup::Missing,
        }
    }

    fn library(&self, name: &Name) -> ConstLookup {
        match self.sema.provider.global(&name.to_string_lossy()) {
            Some(LibGlobal::Constant(v, t)) => ConstLookup::Found(Constant::Value(v), t),
            Some(_) => ConstLookup::NotConstant("a library object or function".into()),
            None => ConstLookup::Missing,
        }
    }
}

impl ConstScope for ModuleScope<'_, '_> {
    fn lookup(&self, path: &[&Ident]) -> ConstLookup {
        let sema = self.sema;
        match path {
            [] => ConstLookup::Missing,
            [one] => match sema.lookup_module_scope(self.module, &one.name) {
                Found::Sym(owner, sym) => self.symbol_const(owner, sym),
                Found::Module(_) => ConstLookup::NotConstant("a module".into()),
                Found::Ambiguous(_) => ConstLookup::Ambiguous,
                Found::None => self.library(&one.name),
            },
            [first, rest @ ..] => {
                // `Project.Module.X`, `Module.X`, `Enum.Member`, `Library.X`.
                let mut owner = None;
                let mut rest = rest;
                let mut head = &first.name;
                if *head == sema.project_name && !rest.is_empty() {
                    head = &rest[0].name;
                    rest = &rest[1..];
                }
                match sema.lookup_module_scope(self.module, head) {
                    Found::Module(m) => owner = Some(m),
                    Found::Sym(_, Symbol::Enum(e)) => {
                        let enum_info = &sema.program.enums[e.index()];
                        if let [member] = rest
                            && let Some(i) = enum_info
                                .members
                                .iter()
                                .position(|(n, _)| *n == member.name)
                                .or_else(|| sema.enum_member_index(e, &member.name))
                        {
                            return self
                                .symbol_const(enum_info.module, Symbol::EnumMember(e, i as u32));
                        }
                        return ConstLookup::Missing;
                    }
                    Found::Ambiguous(_) => return ConstLookup::Ambiguous,
                    _ => {}
                }
                let Some(owner) = owner else {
                    return self.library_path(&first.name, rest);
                };
                match rest {
                    [name] => match sema.program.modules[owner.index()].symbols.get(&name.name) {
                        Some((sym, _)) if sema.symbol_public(*sym) => {
                            self.symbol_const(owner, *sym)
                        }
                        Some(_) => ConstLookup::NotConstant("a private member".into()),
                        None => ConstLookup::Missing,
                    },
                    [en, member] => match sema.program.modules[owner.index()].symbols.get(&en.name)
                    {
                        Some((Symbol::Enum(e), _)) => {
                            match sema.enum_member_index(*e, &member.name) {
                                Some(i) => {
                                    self.symbol_const(owner, Symbol::EnumMember(*e, i as u32))
                                }
                                None => ConstLookup::Missing,
                            }
                        }
                        _ => ConstLookup::Missing,
                    },
                    _ => ConstLookup::Missing,
                }
            }
        }
    }

    fn profile(&self) -> &Profile {
        self.sema.profile()
    }

    fn compare(&self) -> CompareMode {
        self.sema.program.modules[self.module.index()]
            .options
            .compare
    }
}

impl Sema<'_> {
    /// Whether a symbol is publicly visible outside its module.
    pub(crate) fn symbol_public(&self, sym: Symbol) -> bool {
        matches!(
            self.symbol_visibility(sym),
            Some(Visibility::Public) | Some(Visibility::Global) | Some(Visibility::Friend)
        )
    }

    /// Index of a member of an enum, by name.
    pub(crate) fn enum_member_index(&self, e: EnumId, name: &Name) -> Option<usize> {
        self.enum_src[e.index()]
            .members
            .iter()
            .position(|m| m.name.name == *name)
    }
}

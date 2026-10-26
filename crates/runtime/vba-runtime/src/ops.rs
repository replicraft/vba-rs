//! The operations an embedder (or a re-entrant host object) performs on a
//! running project: calls, instances, events, variables.
//!
//! [`Ops`] borrows the interpreter, so the same code serves a [`Session`]
//! (which owns it) and a [`HostContext`](crate::HostContext) (which is
//! handed the interpreter for the duration of a host call). Neither checks
//! trust: the session does that before it builds an `Ops`, and a host
//! context only exists inside a run of trusted code.

use std::rc::Rc;

use vba_ast::{ProcKind, Visibility};
use vba_automation::Dispatch;
use vba_ir::IrProgram;
use vba_sema::{GlobalId, ModuleId, ProcId, Symbol, Type};
use vba_value::{ObjectRef, Variant};
use vba_vm::{Arg, RunOutput, Vm};

use crate::error::Error;

/// What a call by name is addressed to.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Target {
    /// The project: `Proc` of the standard modules, or `Module.Proc`
    /// (a document or designer module runs on its default instance).
    Project,
    /// One instance of a class, document or designer module.
    Object(ObjectRef),
}

/// Modules without source (`SessionBuilder::allow_partial`): the project
/// name and the modules left out.
pub(crate) type Absent = (String, Vec<String>);

/// The operations, over a borrowed interpreter.
pub struct Ops<'a> {
    pub(crate) vm: &'a mut Vm,
    pub(crate) ir: Rc<IrProgram>,
    pub(crate) absent: Option<&'a Absent>,
}

impl<'a> Ops<'a> {
    /// Operations on `vm`; the project has no modules without source.
    pub fn new(vm: &'a mut Vm) -> Ops<'a> {
        let ir = vm.program_rc();
        Ops {
            vm,
            ir,
            absent: None,
        }
    }

    pub(crate) fn with_absent(vm: &'a mut Vm, absent: Option<&'a Absent>) -> Ops<'a> {
        let ir = vm.program_rc();
        Ops { vm, ir, absent }
    }

    /// The error for a call that needs a module that was left out.
    fn compiled_only(&self) -> Option<Error> {
        self.absent
            .filter(|(_, m)| !m.is_empty())
            .map(|(project, modules)| Error::CompiledOnly {
                project: project.clone(),
                modules: modules.clone(),
            })
    }

    fn find(&self, name: &str) -> Result<ProcId, Error> {
        let p = &self.ir.program;
        // A qualified name that names a module without source is an explicit
        // error, not "no such procedure".
        if let (Some((_, absent)), Some((m, _))) = (self.absent, name.split_once('.'))
            && absent.iter().any(|a| a.eq_ignore_ascii_case(m))
        {
            return Err(self.compiled_only().expect("absent is non-empty"));
        }
        let callable = |id: ProcId| matches!(p.proc(id).kind, ProcKind::Sub | ProcKind::Function);
        let matches: Vec<ProcId> = if let Some((m, n)) = name.split_once('.') {
            let Some(module) = p.module_by_name(m) else {
                return Err(Error::NoSuchProcedure(name.to_owned()));
            };
            module
                .procs
                .iter()
                .copied()
                .filter(|id| callable(*id) && p.proc(*id).name.eq_ignore_case(n))
                .collect()
        } else {
            p.modules
                .iter()
                .filter(|m| !m.is_class_like())
                .flat_map(|m| m.procs.iter().copied())
                .filter(|id| callable(*id) && p.proc(*id).name.eq_ignore_case(name))
                .collect()
        };
        match matches.as_slice() {
            // The procedure may live in a module that was left out.
            [] => Err(self
                .compiled_only()
                .unwrap_or_else(|| Error::NoSuchProcedure(name.to_owned()))),
            [one] => Ok(*one),
            _ => Err(Error::AmbiguousProcedure(name.to_owned())),
        }
    }

    /// See [`Session::call`](crate::Session::call).
    pub fn call(
        &mut self,
        target: &Target,
        name: &str,
        positional: Vec<Arg>,
        named: Vec<(String, Arg)>,
    ) -> Result<RunOutput, Error> {
        let (id, receiver) = match target {
            Target::Object(obj) => (
                resolve_in_object(&self.ir, self.vm, obj, name, true)?,
                Some(*obj),
            ),
            _ => {
                let id = self.find(name)?;
                let pr = self.ir.program.proc(id);
                if matches!(pr.visibility, Visibility::Private) {
                    return Err(Error::NotCallable(name.to_owned(), "it is private"));
                }
                let module = self.ir.program.module(pr.module);
                if module.is_class_like() {
                    if !module.predeclared {
                        return Err(Error::NotCallable(
                            name.to_owned(),
                            "it belongs to a class module without a default instance",
                        ));
                    }
                    (id, Some(self.vm.default_instance_object(pr.module)?))
                } else {
                    (id, None)
                }
            }
        };
        let args = self.vm.arrange_call_args(id, positional, named)?;
        Ok(match receiver {
            Some(obj) => self.vm.run_method(obj, id, args)?,
            None => self.vm.run_procedure(id, args)?,
        })
    }

    fn class_like_module(&self, module: &str) -> Result<ModuleId, Error> {
        if let Some((_, absent)) = self.absent
            && absent.iter().any(|a| a.eq_ignore_ascii_case(module))
        {
            return Err(self.compiled_only().expect("absent is non-empty"));
        }
        let m = self
            .ir
            .program
            .module_by_name(module)
            .ok_or_else(|| Error::NoSuchProcedure(module.to_owned()))?;
        if !m.is_class_like() {
            return Err(Error::NotCallable(
                module.to_owned(),
                "it is a standard module, which has no instances",
            ));
        }
        Ok(m.id)
    }

    /// See [`Session::create_object`](crate::Session::create_object).
    pub fn create_object(
        &mut self,
        module: &str,
        host_base: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        let m = self.class_like_module(module)?;
        Ok(self.vm.create_instance(m, host_base)?)
    }

    /// See [`Session::default_instance`](crate::Session::default_instance).
    pub fn default_instance(
        &mut self,
        module: &str,
        host_base: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        let m = self.class_like_module(module)?;
        if !self.ir.program.module(m).predeclared {
            return Err(Error::NotCallable(
                module.to_owned(),
                "it is a class module without a default instance",
            ));
        }
        Ok(self.vm.default_instance_with(m, host_base)?)
    }

    /// See [`Session::destroy_object`](crate::Session::destroy_object).
    pub fn destroy_object(&mut self, object: &ObjectRef) -> Result<(), Error> {
        Ok(self.vm.destroy_instance(object)?)
    }

    /// See [`Session::has_procedure`](crate::Session::has_procedure).
    pub fn has_procedure(&self, object: &ObjectRef, name: &str) -> bool {
        resolve_in_object(&self.ir, self.vm, object, name, false).is_ok()
    }

    /// See [`Session::raise_event_on`](crate::Session::raise_event_on).
    pub fn raise_event_on(
        &mut self,
        object: &ObjectRef,
        source: &str,
        event: &str,
        positional: Vec<Arg>,
        named: Vec<(String, Arg)>,
    ) -> Result<Option<RunOutput>, Error> {
        let name = format!("{source}_{event}");
        let id = match resolve_in_object(&self.ir, self.vm, object, &name, false) {
            Ok(id) => id,
            Err(Error::NoSuchProcedure(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        let args = self.vm.arrange_call_args(id, positional, named)?;
        Ok(Some(self.vm.run_method(*object, id, args)?))
    }

    /// See [`Session::get_member`](crate::Session::get_member).
    pub fn get_member(&mut self, object: &ObjectRef, name: &str) -> Result<Variant, Error> {
        Ok(self.vm.get_member(object, name)?)
    }

    /// See [`Session::set_member`](crate::Session::set_member).
    pub fn set_member(
        &mut self,
        object: &ObjectRef,
        name: &str,
        value: Variant,
    ) -> Result<(), Error> {
        Ok(self.vm.set_member(object, name, value)?)
    }

    /// See [`Session::release`](crate::Session::release).
    pub fn release(&mut self, value: &Variant) {
        self.vm.release(value);
    }

    // ---- module variables ---------------------------------------------------

    fn global_symbol(&self, name: &str) -> Result<Symbol, Error> {
        let p = &self.ir.program;
        let n = vba_ast::Name::new(name.rsplit('.').next().unwrap_or(name));
        let candidates: Vec<Symbol> = match name.split_once('.') {
            Some((module, _)) => {
                let m = p
                    .module_by_name(module)
                    .ok_or_else(|| Error::NoSuchMember(name.to_owned()))?;
                if m.is_class_like() {
                    return Err(Error::NotAccessible(
                        name.to_owned(),
                        "it belongs to a class-like module; its variables are fields of an instance (use get_member / set_member)",
                    ));
                }
                m.symbols.get(&n).map(|(s, _)| *s).into_iter().collect()
            }
            None => p
                .modules
                .iter()
                .filter(|m| !m.is_class_like())
                .filter_map(|m| m.symbols.get(&n).map(|(s, _)| *s))
                .filter(|s| match s {
                    // An unqualified name only names what is public.
                    Symbol::Var(g) => !matches!(p.global(*g).visibility, Visibility::Private),
                    Symbol::Const(c) => {
                        !matches!(p.consts[c.index()].visibility, Visibility::Private)
                    }
                    _ => false,
                })
                .collect(),
        };
        match candidates.as_slice() {
            [s @ (Symbol::Var(_) | Symbol::Const(_))] => Ok(*s),
            [] | [_] => Err(Error::NoSuchMember(name.to_owned())),
            _ => Err(Error::AmbiguousMember(name.to_owned())),
        }
    }

    fn check_var_access(&self, name: &str, g: GlobalId) -> Result<(), Error> {
        let info = self.ir.program.global(g);
        if matches!(info.visibility, Visibility::Private) {
            return Err(Error::NotAccessible(name.to_owned(), "it is private"));
        }
        if matches!(info.ty, Type::Udt(_) | Type::LibRecord(_)) {
            return Err(Error::NotAccessible(
                name.to_owned(),
                "it is a user-defined type, which has no host representation",
            ));
        }
        Ok(())
    }

    /// See [`Session::get_global`](crate::Session::get_global).
    pub fn get_global(&mut self, name: &str) -> Result<Variant, Error> {
        match self.global_symbol(name)? {
            Symbol::Var(g) => {
                self.check_var_access(name, g)?;
                Ok(self.vm.get_global(g)?)
            }
            Symbol::Const(c) => {
                let info = &self.ir.program.consts[c.index()];
                if matches!(info.visibility, Visibility::Private) {
                    return Err(Error::NotAccessible(name.to_owned(), "it is private"));
                }
                match &info.value {
                    vba_sema::bound::Constant::Value(v) => Ok(v.clone()),
                    vba_sema::bound::Constant::DateText(_) => Err(Error::Unsupported(format!(
                        "the value of constant `{name}`, a date literal, is not evaluated ahead of use"
                    ))),
                }
            }
            _ => Err(Error::NoSuchMember(name.to_owned())),
        }
    }

    /// See [`Session::set_global`](crate::Session::set_global).
    pub fn set_global(&mut self, name: &str, value: Variant) -> Result<(), Error> {
        match self.global_symbol(name)? {
            Symbol::Var(g) => {
                self.check_var_access(name, g)?;
                if self.ir.program.global(g).fixed_bounds.is_some() {
                    return Err(Error::NotAccessible(
                        name.to_owned(),
                        "it is a fixed-size array",
                    ));
                }
                Ok(self.vm.set_global(g, value)?)
            }
            Symbol::Const(_) => Err(Error::NotAccessible(name.to_owned(), "it is a constant")),
            _ => Err(Error::NoSuchMember(name.to_owned())),
        }
    }
}

/// The procedure `name` of the class-like module `obj` is an instance of.
/// `public_only` rejects private procedures (event handlers are usually
/// private). A destroyed instance is VBA error 91.
pub(crate) fn resolve_in_object(
    ir: &IrProgram,
    vm: &Vm,
    obj: &ObjectRef,
    name: &str,
    public_only: bool,
) -> Result<ProcId, Error> {
    if matches!(obj, ObjectRef::Handle(_)) && !vm.is_live_instance(obj) {
        // A destroyed instance, or a handle that never named one.
        return Err(Error::from(vba_vm::VmError::Runtime(Box::new(
            vba_vm::RuntimeError::new(vba_vm::RtErr::standard(91)),
        ))));
    }
    let Some(module) = vm.object_module(obj) else {
        return Err(Error::NotCallable(
            name.to_owned(),
            "the object is not an instance of a project class",
        ));
    };
    let p = &ir.program;
    let id = p
        .module(module)
        .procs
        .iter()
        .copied()
        .find(|id| {
            let pr = p.proc(*id);
            matches!(
                pr.kind,
                ProcKind::Sub | ProcKind::Function | ProcKind::PropertyGet
            ) && pr.name.eq_ignore_case(name)
        })
        .ok_or_else(|| Error::NoSuchProcedure(name.to_owned()))?;
    if public_only && matches!(p.proc(id).visibility, Visibility::Private) {
        return Err(Error::NotCallable(name.to_owned(), "it is private"));
    }
    Ok(id)
}

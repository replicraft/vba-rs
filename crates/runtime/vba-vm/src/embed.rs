//! Calls made by the embedder: argument preparation (`ByRef` slots, named
//! and omitted arguments) and write-back.

use std::rc::Rc;

use vba_automation::Dispatch;
use vba_ir::{InvokeKind, Member};
use vba_sema::{GlobalId, ModuleId, ProcId};
use vba_value::{ObjectRef, Variant};

use crate::calls::BoundArg;
use crate::error::{Fault, RtErr, RuntimeError, VResult, VmError};
use crate::state::{Loc, vm_handle, vm_id};
use crate::vm::{Arg, Vm};

/// Where the final value of a [`Arg::ByRef`] argument is read from.
pub(crate) enum ByRefOut {
    /// A temporary variable the callee was bound to.
    Slot(usize),
    /// The argument was not bound by reference (`ByVal` parameter or
    /// `ParamArray`); the value to report.
    Orig(Variant),
}

impl Vm {
    /// Binds the embedder's arguments: values, omitted arguments, and
    /// `ByRef` variables, which live in temporary variable slots above the
    /// program's module variables until the call has finished.
    pub(crate) fn prepare_args(
        &mut self,
        pid: ProcId,
        args: Vec<Arg>,
    ) -> VResult<(Vec<BoundArg>, Vec<ByRefOut>)> {
        let ir = self.ir.clone();
        let p = ir.program.proc(pid);
        let has_pa = p.params.last().is_some_and(|x| x.param_array);
        let fixed = p.params.len() - usize::from(has_pa);
        let temp_base = self.globals.len();
        let mut bound = Vec::with_capacity(args.len());
        let mut outs = Vec::new();
        for (i, a) in args.into_iter().enumerate() {
            let step: VResult<()> = (|| {
                match a {
                    Arg::Value(v) => bound.push(BoundArg::Val(v)),
                    Arg::Missing => bound.push(BoundArg::Missing),
                    Arg::ByRef(v) => match p.params.get(i).filter(|_| i < fixed) {
                        Some(param) if !param.by_val => {
                            let v = self.coerce_arg(v, &param.ty)?;
                            let v = self.adopt(v)?;
                            let idx = self.globals.len();
                            self.globals.push(v);
                            bound.push(BoundArg::Ref(Loc::Global(idx as u32)));
                            outs.push(ByRefOut::Slot(idx));
                        }
                        _ => {
                            outs.push(ByRefOut::Orig(v.clone()));
                            bound.push(BoundArg::Val(v));
                        }
                    },
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
                Ok(())
            })();
            if let Err(f) = step {
                self.collect_byref(temp_base, outs, false);
                return Err(f);
            }
        }
        Ok((bound, outs))
    }

    /// Takes the final values out of the temporary slots and removes them.
    /// With `keep` false the values are released instead of returned.
    pub(crate) fn collect_byref(
        &mut self,
        temp_base: usize,
        outs: Vec<ByRefOut>,
        keep: bool,
    ) -> Vec<Variant> {
        let mut values = Vec::with_capacity(outs.len());
        for o in outs {
            values.push(match o {
                ByRefOut::Slot(i) if i < self.globals.len() => {
                    std::mem::replace(&mut self.globals[i], Variant::Empty)
                }
                ByRefOut::Slot(_) => Variant::Empty,
                ByRefOut::Orig(v) => v,
            });
        }
        self.globals.truncate(temp_base.min(self.globals.len()));
        if keep {
            values
        } else {
            for v in values {
                let _ = self.release_value(v);
            }
            Vec::new()
        }
    }

    /// Arranges positional and named arguments of an embedder's call into
    /// the parameter order of `proc_`, filling gaps with [`Arg::Missing`].
    ///
    /// Errors, as in VBA: an unknown or `ParamArray` parameter name is
    /// 448, a parameter given twice is 446.
    pub fn arrange_call_args(
        &self,
        proc_: ProcId,
        positional: Vec<Arg>,
        named: Vec<(String, Arg)>,
    ) -> Result<Vec<Arg>, VmError> {
        let p = self.ir.program.proc(proc_);
        let fixed = p.params.len() - usize::from(p.params.last().is_some_and(|x| x.param_array));
        let mut out = positional;
        for (n, a) in named {
            let Some(i) = p
                .params
                .iter()
                .take(fixed)
                .position(|q| q.name.to_string().eq_ignore_ascii_case(&n))
            else {
                return Err(plain_error(448));
            };
            while out.len() <= i {
                out.push(Arg::Missing);
            }
            if !matches!(out[i], Arg::Missing) {
                return Err(plain_error(446));
            }
            out[i] = a;
        }
        Ok(out)
    }
}

/// An unhandled VBA error raised by the embedding layer itself (no frame is
/// running).
pub(crate) fn plain_error(number: i32) -> VmError {
    VmError::Runtime(Box::new(RuntimeError::new(RtErr::standard(number))))
}

// ---- instances ------------------------------------------------------------

impl Vm {
    /// The class-like module called `name` (case-insensitive).
    pub fn find_module(&self, name: &str) -> Option<ModuleId> {
        self.ir.program.module_by_name(name).map(|m| m.id)
    }

    /// Creates a **new** instance of a class, document or designer module:
    /// its own module-level state, `Class_Initialize` run. The caller owns
    /// one reference (give it back with [`Vm::destroy_instance`]).
    ///
    /// `host_base` is bound as the instance's *host base* before any code
    /// runs: members the module does not define (`Me.Caption`, control
    /// names, `Me!Field`, the default member) are dispatched to it, late
    /// bound. The VM takes one reference to the object in the host's table.
    pub fn create_instance(
        &mut self,
        module: ModuleId,
        host_base: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, VmError> {
        if self.ir.classes[module.index()].is_none() {
            return Err(plain_error(429));
        }
        self.last_site = None;
        let base = host_base.map(|rc| self.host.objects().insert(rc));
        match self.new_class_instance_with(module, base) {
            Ok(id) => Ok(ObjectRef::Handle(vm_handle(id))),
            Err(f) => Err(self.finish_fault(f)),
        }
    }

    /// The predeclared default instance of `module`, created on first use.
    /// With a `host_base`: bound before `Class_Initialize` when this call
    /// creates the instance, otherwise bound to the existing instance if it
    /// has none yet (error 457 if it already has one). The reference is
    /// borrowed.
    pub fn default_instance_with(
        &mut self,
        module: ModuleId,
        host_base: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, VmError> {
        self.last_site = None;
        if let Some(id) = self.default_instances.get(&module).copied() {
            let obj = ObjectRef::Handle(vm_handle(id));
            if let Some(h) = host_base {
                self.bind_host_base(&obj, h)?;
            }
            return Ok(obj);
        }
        let base = host_base.map(|rc| self.host.objects().insert(rc));
        match self.new_class_instance_with(module, base) {
            Ok(id) => {
                self.default_instances.insert(module, id);
                Ok(ObjectRef::Handle(vm_handle(id)))
            }
            Err(f) => Err(self.finish_fault(f)),
        }
    }

    /// Delivers the events host objects raised while the VM was not
    /// suspended in a call of the raising object (for example from the
    /// embedder's own code between calls), in the order they were raised.
    pub fn deliver_pending_events(&mut self) -> Result<(), VmError> {
        self.last_site = None;
        self.drain_events().map_err(|f| self.finish_fault(f))
    }

    /// Binds a host base to an existing instance that has none (typically
    /// the default instance of a form, whose `Class_Initialize` has then
    /// already run without it).
    pub fn bind_host_base(
        &mut self,
        object: &ObjectRef,
        host_base: Rc<dyn Dispatch>,
    ) -> Result<(), VmError> {
        let id = self.live_object_id(object)?;
        if self.objects.get(&id).is_some_and(|o| o.host_base.is_some()) {
            return Err(plain_error(457));
        }
        let h = self.host.objects().insert(host_base);
        if let Some(o) = self.objects.get_mut(&id) {
            o.host_base = Some(h);
        }
        Ok(())
    }

    /// Gives back the reference [`Vm::create_instance`] handed out. When it
    /// was the last one, `Class_Terminate` runs now and the object goes
    /// away; if VBA code still references the instance it lives on until
    /// that reference is released.
    pub fn destroy_instance(&mut self, object: &ObjectRef) -> Result<(), VmError> {
        let id = self.live_object_id(object)?;
        if self.default_instances.values().any(|d| *d == id) {
            return Err(plain_error(5));
        }
        self.last_site = None;
        self.release_handle(vm_handle(id))
            .map_err(|f| self.finish_fault(f))
    }

    /// Whether `object` is the default instance of a document, designer or
    /// predeclared class module.
    pub fn is_default_instance(&self, object: &ObjectRef) -> bool {
        match object {
            ObjectRef::Handle(h) => {
                vm_id(*h).is_some_and(|id| self.default_instances.values().any(|d| *d == id))
            }
            ObjectRef::Nothing => false,
        }
    }

    /// Whether the object is a live VM-owned instance.
    pub fn is_live_instance(&self, object: &ObjectRef) -> bool {
        self.live_object_id(object).is_ok()
    }

    /// Reads a module-level variable of a standard module. An object value
    /// carries a reference the caller owns.
    pub fn get_global(&mut self, g: GlobalId) -> Result<Variant, VmError> {
        let v = self
            .globals
            .get(g.index())
            .cloned()
            .ok_or_else(|| VmError::Internal("no such module variable".into()))?;
        self.retain_value(&v);
        Ok(v)
    }

    /// Assigns a module-level variable of a standard module, converting the
    /// value to the declared type as an assignment in VBA does.
    pub fn set_global(&mut self, g: GlobalId, value: Variant) -> Result<(), VmError> {
        self.last_site = None;
        let ty = self.ir.program.global(g).ty.clone();
        let r = self
            .convert_value(value, &ty)
            .and_then(|v| self.store(&Loc::Global(g.0), v));
        r.map_err(|f| self.finish_fault(f))
    }

    /// Reads a public variable, property or parameterless member of an
    /// object by name, as `object.name` does (members the module does not
    /// define go to its host base). An object result carries a reference
    /// the caller owns.
    pub fn get_member(&mut self, object: &ObjectRef, name: &str) -> Result<Variant, VmError> {
        self.member_access(object, name, None)
    }

    /// Assigns a public variable or property of an object by name
    /// (`Let`, or `Set` when the value is an object).
    pub fn set_member(
        &mut self,
        object: &ObjectRef,
        name: &str,
        value: Variant,
    ) -> Result<(), VmError> {
        self.member_access(object, name, Some(value)).map(|_| ())
    }

    fn member_access(
        &mut self,
        object: &ObjectRef,
        name: &str,
        put: Option<Variant>,
    ) -> Result<Variant, VmError> {
        self.live_object_id(object)?;
        let (kind, args) = match put {
            None => (InvokeKind::Get, Vec::new()),
            Some(v) => {
                let kind = if matches!(v, Variant::Object(_)) {
                    InvokeKind::PutRef
                } else {
                    InvokeKind::Put
                };
                (kind, vec![(None, BoundArg::Val(v))])
            }
        };
        self.last_site = None;
        self.stop_reason = None;
        let base = self.frames.len();
        let r = self.invoke_value(
            Variant::Object(*object),
            &Member::Named(name.to_owned()),
            kind,
            args,
            true,
        );
        // A member that is a procedure ran in a nested loop; anything else
        // left no frame behind.
        debug_assert!(self.frames.len() == base);
        r.map_err(|f| self.finish_fault(f))
    }

    /// Releases the object references inside a value the VM handed out
    /// (a function result or a `ByRef` value carries one reference per
    /// object).
    pub fn release(&mut self, value: &Variant) {
        let _ = self.release_value(value.clone());
    }

    fn live_object_id(&self, object: &ObjectRef) -> Result<u64, VmError> {
        let ObjectRef::Handle(h) = object else {
            return Err(plain_error(91));
        };
        match vm_id(*h) {
            Some(id) if self.objects.contains_key(&id) => Ok(id),
            _ => Err(plain_error(91)),
        }
    }

    /// Ends a failed top-level operation: the frames of the failed run are
    /// dropped (unless this is a nested run inside a host call, whose
    /// outer frames must survive).
    pub(crate) fn finish_fault(&mut self, f: Fault) -> VmError {
        if self.nesting == 0 {
            self.frames.clear();
        }
        self.fault_to_error(f)
    }
}

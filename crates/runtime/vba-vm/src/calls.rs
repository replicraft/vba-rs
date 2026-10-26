//! Procedure frames, argument binding, object lifetime and events.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use vba_automation::{Arg as AutoArg, DispError, EventSink};
use vba_ir::{ErrMode, FuncKind, Reg};
use vba_sema::{EventId, EventRef, GlobalId, ModuleId, ProcId, Type};
use vba_value::{Bounds, ObjectHandle, ObjectRef, VarType, Variant, VbaArray, VbaError};

use crate::error::{Fault, VResult};
use crate::state::*;
use crate::vm::Vm;

/// A bound argument of a call.
#[derive(Debug, Clone)]
pub(crate) enum BoundArg {
    Val(Variant),
    Ref(Loc),
    Missing,
}

/// The marker value of an omitted `Variant` argument (`IsMissing`).
pub(crate) fn missing_marker() -> Variant {
    AutoArg::missing().value
}

pub(crate) fn is_missing_marker(v: &Variant) -> bool {
    matches!(v, Variant::Error(e) if e.number() == vba_automation::hresult::DISP_E_PARAMNOTFOUND)
}

impl Vm {
    // ---- frames -----------------------------------------------------------

    fn new_frame(
        &self,
        func: usize,
        proc_: Option<ProcId>,
        module: ModuleId,
        me: Option<u64>,
        slots: Vec<Slot>,
        ret_dst: Option<Reg>,
    ) -> Frame {
        let nregs = self.ir.functions[func].regs.len();
        Frame {
            func,
            proc: proc_,
            module,
            block: 0,
            idx: 0,
            slots,
            regs: (0..nregs).map(|_| RegVal::Empty).collect(),
            me,
            mode: ErrMode::Propagate,
            in_handler: false,
            fail: None,
            gosub: Vec::new(),
            line: 0,
            owned: Vec::new(),
            held: Vec::new(),
            ret_dst,
            cur_stmt: u32::MAX,
        }
    }

    /// Coerces an argument value to the declared parameter type.
    fn coerce_arg(&mut self, v: Variant, ty: &Type) -> VResult<Variant> {
        match ty {
            Type::Variant | Type::Any | Type::Error => Ok(v),
            _ => self.convert_value(v, ty),
        }
    }

    /// Pushes a frame for `pid`, binding `args` to its parameters.
    pub(crate) fn push_proc_frame(
        &mut self,
        pid: ProcId,
        me: Option<u64>,
        args: Vec<BoundArg>,
        ret_dst: Option<Reg>,
    ) -> VResult<()> {
        let ir = self.ir.clone();
        let p = ir.program.proc(pid);
        let has_pa = p.params.last().is_some_and(|x| x.param_array);
        let fixed = p.params.len() - usize::from(has_pa);
        if args.len() > fixed && !has_pa {
            return Err(VbaError::new(450).into());
        }
        self.host.budget().enter_call().map_err(Fault::from)?;
        let built = self.bind_slots(&ir, pid, &args);
        let slots = match built {
            Ok(s) => s,
            Err(e) => {
                self.host.budget().leave_call();
                return Err(e);
            }
        };
        if let Some(id) = me {
            self.retain_handle(vm_handle(id));
        }
        let frame = self.new_frame(pid.index(), Some(pid), p.module, me, slots, ret_dst);
        self.frames.push(frame);
        Ok(())
    }

    fn bind_slots(
        &mut self,
        ir: &vba_ir::IrProgram,
        pid: ProcId,
        args: &[BoundArg],
    ) -> VResult<Vec<Slot>> {
        let p = ir.program.proc(pid);
        let func = &ir.functions[pid.index()];
        let has_pa = p.params.last().is_some_and(|x| x.param_array);
        let fixed = p.params.len() - usize::from(has_pa);
        let mut slots: Vec<Slot> = Vec::with_capacity(func.slots.len());
        for i in 0..fixed {
            let param = &p.params[i];
            let slot = match args.get(i).cloned().unwrap_or(BoundArg::Missing) {
                BoundArg::Val(v) => {
                    let v = self.coerce_arg(v, &param.ty)?;
                    Slot::Val(self.adopt(v)?)
                }
                BoundArg::Ref(loc) => {
                    if param.by_val {
                        let v = self.load(&loc)?;
                        let v = self.coerce_arg(v, &param.ty)?;
                        Slot::Val(self.adopt(v)?)
                    } else {
                        Slot::Bound(loc)
                    }
                }
                BoundArg::Missing => {
                    if !param.optional {
                        return Err(VbaError::new(449).into());
                    }
                    match &param.default {
                        Some(c) => {
                            let v = self.constant(c)?;
                            let v = self.coerce_arg(v, &param.ty)?;
                            Slot::Val(v)
                        }
                        None if param.ty == Type::Variant => Slot::Val(missing_marker()),
                        None => Slot::Val(self.default_for(&param.ty)?),
                    }
                }
            };
            slots.push(slot);
        }
        if has_pa {
            let extras = args.get(fixed..).unwrap_or(&[]);
            let mut vals = Vec::with_capacity(extras.len());
            for a in extras {
                vals.push(match a {
                    BoundArg::Val(v) => v.clone(),
                    BoundArg::Ref(l) => self.load(l)?,
                    BoundArg::Missing => missing_marker(),
                });
            }
            let arr = self.variant_array(vals, 0)?;
            slots.push(Slot::Val(arr));
        }
        for s in func.slots.iter().skip(slots.len()) {
            let v = self.default_for(&s.ty)?;
            slots.push(Slot::Val(v));
        }
        Ok(slots)
    }

    /// A one-dimensional `Variant` array starting at `lower`.
    pub(crate) fn variant_array(&mut self, vals: Vec<Variant>, lower: i32) -> VResult<Variant> {
        if vals.is_empty() {
            return Ok(Variant::Array(Box::new(VbaArray::new_empty(
                VarType::Variant,
                lower,
            ))));
        }
        let mut a = VbaArray::new_dynamic(VarType::Variant);
        a.redim(&[Bounds::new(lower, lower + vals.len() as i32 - 1)], false)?;
        for (i, v) in vals.into_iter().enumerate() {
            let v = self.adopt(v)?;
            a.set(&[lower + i as i32], v)?;
        }
        Ok(Variant::Array(Box::new(a)))
    }

    /// Pops the top frame, releasing everything it owns, and returns the
    /// function result (which carries its own reference).
    pub(crate) fn pop_frame(&mut self) -> VResult<Variant> {
        let mut fr = self.frames.pop().expect("frame");
        self.host.budget().leave_call();
        let ir = self.ir.clone();
        let mut result = Variant::Empty;
        let result_slot = fr
            .proc
            .and_then(|p| ir.program.proc(p).result_local)
            .map(|l| l.index());
        let mut pending: Option<Fault> = None;
        let note = |r: VResult<()>, pending: &mut Option<Fault>| {
            if let Err(f) = r
                && !matches!(f, Fault::Err(_))
                && pending.is_none()
            {
                *pending = Some(f);
            }
        };
        let owned = std::mem::take(&mut fr.owned);
        let held = std::mem::take(&mut fr.held);
        for (i, s) in std::mem::take(&mut fr.slots).into_iter().enumerate() {
            if let Slot::Val(v) = s {
                if Some(i) == result_slot {
                    result = v;
                } else {
                    let r = self.release_value(v);
                    note(r, &mut pending);
                }
            }
        }
        // User-defined-type results are collected like any other value.
        for v in owned.into_iter().chain(held) {
            let r = self.release_value(v);
            note(r, &mut pending);
        }
        if let Some(id) = fr.me {
            let r = self.release_handle(vm_handle(id));
            note(r, &mut pending);
        }
        match pending {
            Some(f) => Err(f),
            None => Ok(result),
        }
    }

    /// Discards frames above `base` without running any code.
    pub(crate) fn abandon_to(&mut self, base: usize) {
        while self.frames.len() > base {
            self.frames.pop();
            self.host.budget().leave_call();
        }
    }

    /// Calls a procedure from Rust and runs it to completion.
    pub(crate) fn call_nested(
        &mut self,
        pid: ProcId,
        me: Option<u64>,
        args: Vec<BoundArg>,
    ) -> VResult<Variant> {
        let base = self.frames.len();
        self.push_proc_frame(pid, me, args, None)?;
        match self.run(base) {
            Ok(v) => Ok(v),
            Err(f) => {
                self.abandon_to(base);
                Err(f)
            }
        }
    }

    /// Runs a module initialiser function.
    pub(crate) fn run_init(&mut self, func: usize, me: Option<u64>) -> VResult<()> {
        let ir = self.ir.clone();
        let FuncKind::ModuleInit(m) = ir.functions[func].kind else {
            return Err(Fault::Internal("not an initialiser".into()));
        };
        let base = self.frames.len();
        self.host.budget().enter_call().map_err(Fault::from)?;
        if let Some(id) = me {
            self.retain_handle(vm_handle(id));
        }
        let frame = self.new_frame(func, None, m, me, Vec::new(), None);
        self.frames.push(frame);
        match self.run(base) {
            Ok(_) => Ok(()),
            Err(f) => {
                self.abandon_to(base);
                Err(f)
            }
        }
    }

    // ---- objects ----------------------------------------------------------

    /// Creates an instance of a class module; the caller owns one reference.
    pub(crate) fn new_class_instance(&mut self, m: ModuleId) -> VResult<u64> {
        let ir = self.ir.clone();
        let Some(info) = ir.classes[m.index()].as_ref() else {
            return Err(VbaError::new(429).into());
        };
        self.charge_memory(64 + 32 * info.fields.len() as u64)?;
        let mut fields = Vec::with_capacity(info.fields.len());
        for g in &info.fields {
            let ty = ir.program.global(*g).ty.clone();
            fields.push(self.default_for(&ty)?);
        }
        let id = self.next_obj;
        self.next_obj += 1;
        self.objects.insert(
            id,
            Obj {
                kind: ObjKind::Class {
                    module: m,
                    fields,
                    ext_conns: Vec::new(),
                },
                refs: 1,
                terminating: false,
                sinks: Vec::new(),
            },
        );
        let init: VResult<()> = (|| {
            if let Some(f) = info.init {
                self.run_init(f.0 as usize, Some(id))?;
            }
            if let Some(p) = info.initialize {
                self.call_nested(p, Some(id), Vec::new())?;
            }
            Ok(())
        })();
        if let Err(e) = init {
            let _ = self.destroy_object(id, false);
            return Err(e);
        }
        Ok(id)
    }

    /// The predeclared default instance of a module, created on first use.
    pub(crate) fn default_instance(&mut self, m: ModuleId) -> VResult<u64> {
        if let Some(id) = self.default_instances.get(&m) {
            return Ok(*id);
        }
        let id = self.new_class_instance(m)?;
        self.default_instances.insert(m, id);
        Ok(id)
    }

    /// Drops one reference to a VM or host object.
    pub(crate) fn release_handle(&mut self, h: ObjectHandle) -> VResult<()> {
        match vm_id(h) {
            Some(id) => self.release_obj(id),
            None => {
                let _ = self.host.objects().release(h);
                Ok(())
            }
        }
    }

    fn release_obj(&mut self, id: u64) -> VResult<()> {
        let destroy = match self.objects.get_mut(&id) {
            Some(o) => {
                o.refs = o.refs.saturating_sub(1);
                if o.refs == 0 && !o.terminating {
                    o.terminating = true;
                    true
                } else {
                    false
                }
            }
            None => false,
        };
        if destroy {
            return self.destroy_object(id, true);
        }
        Ok(())
    }

    /// Terminates (optionally) and removes an object whose count is zero.
    fn destroy_object(&mut self, id: u64, run_terminate: bool) -> VResult<()> {
        let ir = self.ir.clone();
        let mut result: VResult<()> = Ok(());
        if run_terminate
            && let Some(Obj {
                kind: ObjKind::Class { module, .. },
                ..
            }) = self.objects.get(&id)
            && let Some(term) = ir.classes[module.index()]
                .as_ref()
                .and_then(|c| c.terminate)
        {
            result = self.call_nested(term, Some(id), Vec::new()).map(|_| ());
        }
        match self.objects.get(&id) {
            Some(o) if o.refs > 0 => {
                // Resurrected by Class_Terminate.
                if let Some(o) = self.objects.get_mut(&id) {
                    o.terminating = false;
                }
                return result;
            }
            None => return result,
            _ => {}
        }
        let obj = self.objects.remove(&id).expect("present");
        match obj.kind {
            ObjKind::Class {
                module,
                fields,
                ext_conns,
            } => {
                self.free_memory(64 + 32 * fields.len() as u64);
                let info = ir.classes[module.index()].as_ref();
                for c in ext_conns {
                    self.disconnect_external(c);
                }
                for (i, v) in fields.into_iter().enumerate() {
                    let g = info.and_then(|c| c.fields.get(i).copied());
                    if let Some(g) = g
                        && ir.program.global(g).with_events
                    {
                        self.disconnect_events(id, g, &v);
                    }
                    if let Err(f) = self.release_value(v)
                        && !matches!(f, Fault::Err(_))
                        && result.is_ok()
                    {
                        result = Err(f);
                    }
                }
            }
            ObjKind::Collection(items) => {
                for it in items {
                    let _ = self.release_quiet(it.value);
                }
            }
            ObjKind::Err | ObjKind::Debug => {}
        }
        result
    }

    // ---- procedure resolution ----------------------------------------------

    /// Finds the procedure to run for a call with an optional receiver.
    /// Returns the procedure and the `Me` object.
    pub(crate) fn resolve_target(
        &self,
        pid: ProcId,
        receiver: Option<&Variant>,
        frame_me: Option<u64>,
    ) -> VResult<(ProcId, Option<u64>)> {
        let ir = &self.ir;
        let pmod = ir.program.proc(pid).module;
        let class_like = ir.program.module(pmod).is_class_like();
        let recv_id = match receiver {
            Some(Variant::Object(ObjectRef::Nothing)) => return Err(VbaError::new(91).into()),
            Some(Variant::Object(ObjectRef::Handle(h))) => match vm_id(*h) {
                Some(id) => Some(id),
                None => return Err(VbaError::new(438).into()),
            },
            Some(Variant::Empty) => return Err(VbaError::new(91).into()),
            Some(_) => return Err(VbaError::new(424).into()),
            None => {
                if class_like {
                    frame_me
                } else {
                    None
                }
            }
        };
        let Some(id) = recv_id else {
            if class_like {
                return Err(VbaError::new(91).into());
            }
            return Ok((pid, None));
        };
        let module = match self.objects.get(&id).map(|o| &o.kind) {
            Some(ObjKind::Class { module, .. }) => *module,
            Some(_) => return Err(VbaError::new(438).into()),
            None => return Err(VbaError::new(91).into()),
        };
        if module == pmod {
            return Ok((pid, Some(id)));
        }
        // Dispatch through an implemented interface.
        if let Some(info) = ir.classes[module.index()].as_ref() {
            for imp in &info.implements {
                if imp.interface == Type::Class(pmod)
                    && let Some((_, target)) = imp.members.iter().find(|(i, _)| *i == pid)
                {
                    return Ok((*target, Some(id)));
                }
            }
        }
        // A procedure of another class module called on an unrelated object.
        Err(VbaError::new(438).into())
    }

    /// Whether an object is an instance of (or implements) a class module.
    pub(crate) fn instance_of(&self, id: u64, m: ModuleId) -> bool {
        let Some(Obj {
            kind: ObjKind::Class { module, .. },
            ..
        }) = self.objects.get(&id)
        else {
            return false;
        };
        if *module == m {
            return true;
        }
        self.ir.classes[module.index()]
            .as_ref()
            .is_some_and(|c| c.implements.iter().any(|i| i.interface == Type::Class(m)))
    }

    // ---- events -------------------------------------------------------------

    /// Finds the handler of `event` for the `WithEvents` variable `var` in
    /// the class of `sink`.
    fn project_handler(&self, sink: u64, var: GlobalId, event: EventId) -> Option<ProcId> {
        let Some(Obj {
            kind: ObjKind::Class { module, .. },
            ..
        }) = self.objects.get(&sink)
        else {
            return None;
        };
        let info = self.ir.classes[module.index()].as_ref()?;
        info.handlers
            .iter()
            .find(|h| h.var == var && h.event == EventRef::Project(event))
            .map(|h| h.proc)
    }

    /// Delivers an event raised by `source` to its sinks in connection order.
    pub(crate) fn raise_event(
        &mut self,
        source: u64,
        event: EventId,
        args: Vec<BoundArg>,
    ) -> VResult<()> {
        let sinks = match self.objects.get(&source) {
            Some(o) => o.sinks.clone(),
            None => return Ok(()),
        };
        for s in sinks {
            if !self.objects.contains_key(&s.obj) {
                continue;
            }
            if let Some(h) = self.project_handler(s.obj, s.var, event) {
                self.call_nested(h, Some(s.obj), args.clone())?;
            }
        }
        Ok(())
    }

    /// Reconnects the events of a `WithEvents` variable after `Set`.
    pub(crate) fn rewire_events(
        &mut self,
        sink: u64,
        var: GlobalId,
        old: &Variant,
        new: &Variant,
    ) -> VResult<()> {
        self.disconnect_events(sink, var, old);
        let Variant::Object(ObjectRef::Handle(h)) = new else {
            return Ok(());
        };
        match vm_id(*h) {
            Some(src) => {
                if let Some(o) = self.objects.get_mut(&src) {
                    o.sinks.push(Sink { obj: sink, var });
                }
                Ok(())
            }
            None => self.connect_external(sink, var, *h),
        }
    }

    fn disconnect_events(&mut self, sink: u64, var: GlobalId, old: &Variant) {
        let Variant::Object(ObjectRef::Handle(h)) = old else {
            return;
        };
        match vm_id(*h) {
            Some(src) => {
                if let Some(o) = self.objects.get_mut(&src) {
                    o.sinks.retain(|s| !(s.obj == sink && s.var == var));
                }
            }
            None => {
                let conn = match self.objects.get_mut(&sink).map(|o| &mut o.kind) {
                    Some(ObjKind::Class { ext_conns, .. }) => ext_conns
                        .iter()
                        .position(|c| c.var == var)
                        .map(|i| ext_conns.remove(i)),
                    _ => None,
                };
                if let Some(c) = conn {
                    self.disconnect_external(c);
                }
            }
        }
    }

    fn disconnect_external(&mut self, c: ExtConn) {
        if let Ok(obj) = self.host.objects().get(c.source)
            && let Some(src) = obj.event_source()
        {
            let _ = src.unadvise(c.cookie);
        }
    }

    fn connect_external(&mut self, sink: u64, var: GlobalId, source: ObjectHandle) -> VResult<()> {
        let table = self.host.objects();
        let obj = table.get(source)?;
        let src = obj.event_source().ok_or(VbaError::new(459))?;
        let Some(Obj {
            kind: ObjKind::Class { module, .. },
            ..
        }) = self.objects.get(&sink)
        else {
            return Ok(());
        };
        let names: Vec<String> = self.ir.classes[module.index()]
            .as_ref()
            .map(|c| {
                c.handlers
                    .iter()
                    .filter(|h| h.var == var)
                    .filter_map(|h| match &h.event {
                        EventRef::Library { name, .. } => Some(name.to_string().to_lowercase()),
                        EventRef::Project(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let qs = Rc::new(QueueSink {
            queue: self.pending_events.clone(),
            sink,
            var,
            names,
        });
        let cookie = src.advise(qs).map_err(|_| VbaError::new(459))?;
        if let Some(Obj {
            kind: ObjKind::Class { ext_conns, .. },
            ..
        }) = self.objects.get_mut(&sink)
        {
            ext_conns.push(ExtConn {
                var,
                source,
                cookie,
            });
        }
        Ok(())
    }

    /// Delivers events queued by objects outside the VM.
    pub(crate) fn drain_events(&mut self) -> VResult<()> {
        loop {
            let ev = self.pending_events.borrow_mut().pop_front();
            let Some(ev) = ev else { return Ok(()) };
            let Some(Obj {
                kind: ObjKind::Class { module, .. },
                ..
            }) = self.objects.get(&ev.sink)
            else {
                continue;
            };
            let handler = self.ir.classes[module.index()].as_ref().and_then(|c| {
                c.handlers
                    .iter()
                    .find(|h| {
                        h.var == ev.var
                            && matches!(&h.event, EventRef::Library { name, .. }
                                if name.to_string().eq_ignore_ascii_case(&ev.event))
                    })
                    .map(|h| h.proc)
            });
            if let Some(p) = handler {
                let args = ev.args.into_iter().map(BoundArg::Val).collect();
                self.call_nested(p, Some(ev.sink), args)?;
            }
        }
    }

    // ---- shutdown -----------------------------------------------------------

    pub(crate) fn shutdown_inner(&mut self) -> VResult<()> {
        let ir = self.ir.clone();
        let mut first: VResult<()> = Ok(());
        let note = |r: VResult<()>, first: &mut VResult<()>| {
            if let Err(f) = r
                && first.is_ok()
            {
                *first = Err(f);
            }
        };
        for g in &ir.program.globals {
            if ir.program.module(g.module).is_class_like() {
                continue;
            }
            let old = std::mem::replace(&mut self.globals[g.id.index()], Variant::Empty);
            let r = self.release_value(old);
            note(r, &mut first);
        }
        let mut defaults: Vec<(ModuleId, u64)> = self.default_instances.drain().collect();
        defaults.sort();
        for (_, id) in defaults {
            let r = self.release_obj(id);
            note(r, &mut first);
        }
        first
    }

    /// Destroys the temporaries of the top frame (statement boundary).
    pub(crate) fn release_owned(&mut self) -> VResult<()> {
        let owned = match self.frames.last_mut() {
            Some(f) if !f.owned.is_empty() => std::mem::take(&mut f.owned),
            _ => return Ok(()),
        };
        let mut first: VResult<()> = Ok(());
        for v in owned {
            if let Err(f) = self.release_value(v)
                && first.is_ok()
            {
                first = Err(f);
            }
        }
        first
    }

    /// Registers a freshly produced reference as a statement temporary.
    pub(crate) fn own(&mut self, v: &Variant) {
        if Self::needs_release(v)
            && let Some(f) = self.frames.last_mut()
        {
            f.owned.push(v.clone());
        }
    }
}

/// The event sink handed to objects outside the VM: it queues events for
/// delivery at the next safe point.
struct QueueSink {
    queue: Rc<RefCell<VecDeque<PendingEvent>>>,
    sink: u64,
    var: GlobalId,
    names: Vec<String>,
}

impl EventSink for QueueSink {
    fn handles(&self, event: &str) -> bool {
        self.names.iter().any(|n| n.eq_ignore_ascii_case(event))
    }

    fn on_event(
        &self,
        _: &vba_automation::ObjectTable,
        event: &str,
        args: &mut [AutoArg],
    ) -> Result<Variant, DispError> {
        self.queue.borrow_mut().push_back(PendingEvent {
            sink: self.sink,
            var: self.var,
            event: event.to_owned(),
            args: args.iter().map(|a| a.value.clone()).collect(),
        });
        Ok(Variant::Empty)
    }
}

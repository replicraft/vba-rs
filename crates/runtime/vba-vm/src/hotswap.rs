//! Replacing the program of an idle VM (hot edit).
//!
//! Two cases, decided by comparing the declarations of the old and the new
//! program (names, kinds, types and order of modules, procedures, variables,
//! user-defined types, enums, events and `Declare`s; bodies, comments and
//! source positions do not matter):
//!
//! * **Same layout.** Only procedure bodies (or constants' values, comments,
//!   formatting) differ. Every id in the VM's state stays valid, so the new
//!   program is swapped in and *nothing else is touched*: module variables,
//!   objects, default instances, `Err` and open files carry on.
//! * **Changed layout.** Ids may differ, so the state is rebuilt: a
//!   module-level variable of a standard module (and a `Static` local) keeps
//!   its value when the new program declares a variable of the same module,
//!   name and declared type, and the value is plain data (no object of the
//!   VM and no user-defined-type value inside). Every other variable returns
//!   to its initial value (standard-module initialisers run again). All
//!   objects owned by the VM (class instances, default instances, collections
//!   held in variables) and all user-defined-type values are dropped
//!   **without** `Class_Terminate`, exactly like [`Vm::reset`]; references to
//!   host objects they held are released. `Err`, open files, the random
//!   generator and the budget counters are kept.
//!
//! The VM must be idle (no frame on its stack, not suspended in a host call):
//! frames carry procedure indexes and block positions of the old program.

use std::rc::Rc;

use vba_ir::IrProgram;
use vba_sema::Program;
use vba_value::{ObjectRef, Variant};

use crate::error::{VResult, VmError};
use crate::state::*;
use crate::vm::Vm;

/// What a program replacement did to the state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SwapReport {
    /// Whether the declarations differed (the state was rebuilt).
    pub layout_changed: bool,
    /// `Module.Name` of the variables that kept their value (empty when the
    /// layout did not change: everything is kept).
    pub kept: Vec<String>,
    /// `Module.Name` of the variables that returned to their initial value.
    pub reset: Vec<String>,
    /// How many objects owned by the VM were dropped.
    pub dropped_objects: usize,
}

/// Why a program could not be replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SwapError {
    /// A procedure is running (or the VM is suspended in a host call).
    Busy,
    /// Re-initialising the state failed.
    Failed(String),
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::Busy => f.write_str("a procedure is running"),
            SwapError::Failed(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for SwapError {}

impl Vm {
    /// Whether no procedure is running and the VM is not suspended in a host
    /// call.
    pub fn is_idle(&self) -> bool {
        self.frames.is_empty() && self.nesting == 0 && self.run_depth == 0
    }

    /// Replaces the program; see the module documentation for what happens
    /// to the state. `lines` and `proc_lines` are the tables for the new
    /// program (see [`Vm::set_source_lines`], [`Vm::set_procedure_lines`]).
    pub fn replace_program(
        &mut self,
        new: Rc<IrProgram>,
        lines: Vec<Vec<u32>>,
        proc_lines: Vec<(u32, u32)>,
    ) -> Result<SwapReport, SwapError> {
        if !self.is_idle() {
            return Err(SwapError::Busy);
        }
        let _meter = self.host.budget().enter();
        let same = same_layout(&self.ir.program, &new.program);
        let mut report = SwapReport {
            layout_changed: !same,
            ..SwapReport::default()
        };
        if same {
            self.install(new, lines, proc_lines);
            return Ok(report);
        }
        // Carry plain module variables over by (module, name, declared type).
        let old = self.ir.clone();
        let mut carried: Vec<(usize, Variant)> = Vec::new();
        let mut kept_old = vec![false; old.program.globals.len()];
        for g in &old.program.globals {
            if old.program.module(g.module).is_class_like() {
                continue;
            }
            let Some(v) = self.globals.get(g.id.index()) else {
                continue;
            };
            let target = new.program.globals.iter().find(|n| {
                n.name == g.name
                    && n.ty == g.ty
                    && n.fixed_bounds == g.fixed_bounds
                    && n.static_of.is_some() == g.static_of.is_some()
                    && new.program.module(n.module).name == old.program.module(g.module).name
                    && !new.program.module(n.module).is_class_like()
                    && static_name(&new.program, n.static_of)
                        == static_name(&old.program, g.static_of)
            });
            if let Some(n) = target
                && is_plain(v)
            {
                carried.push((n.id.index(), v.clone()));
                kept_old[g.id.index()] = true;
                report.kept.push(global_label(&old.program, g.id.index()));
            } else {
                report.reset.push(global_label(&old.program, g.id.index()));
            }
        }
        // References to host objects held by what is dropped are released.
        let mut host_refs = Vec::new();
        for (i, v) in self.globals.iter().enumerate() {
            if !kept_old.get(i).copied().unwrap_or(false) {
                collect_host_handles(v, &mut host_refs);
            }
        }
        report.dropped_objects = self
            .objects
            .keys()
            .filter(|id| **id != ERR_ID && **id != DEBUG_ID)
            .count();
        for o in self.objects.values() {
            match &o.kind {
                ObjKind::Class { fields, .. } => fields
                    .iter()
                    .for_each(|v| collect_host_handles(v, &mut host_refs)),
                ObjKind::Collection(items) => items
                    .iter()
                    .for_each(|i| collect_host_handles(&i.value, &mut host_refs)),
                _ => {}
            }
            if let Some(h) = o.host_base {
                host_refs.push(h);
            }
        }
        for h in host_refs {
            let _ = self.host.objects().release(h);
        }
        self.install(new, lines, proc_lines);
        self.pending_fatal = None;
        self.last_site = None;
        if let Err(f) = self.rebuild_state(carried) {
            let e: VmError = self.fault_to_error(f);
            return Err(SwapError::Failed(e.to_string()));
        }
        Ok(report)
    }

    fn install(&mut self, new: Rc<IrProgram>, lines: Vec<Vec<u32>>, proc_lines: Vec<(u32, u32)>) {
        self.ir = new;
        self.source_lines = lines;
        self.proc_lines = proc_lines;
        self.reindex();
        if let Some(d) = self.debugger.as_ref() {
            let wanted = d.wanted_breakpoints();
            if !wanted.is_empty() {
                self.set_breakpoints(&wanted);
            }
        }
    }

    /// Wipes objects and rebuilds module variables for the installed
    /// program, then puts `carried` values back.
    fn rebuild_state(&mut self, carried: Vec<(usize, Variant)>) -> VResult<()> {
        self.objects.clear();
        self.udts.clear();
        self.default_instances.clear();
        self.pending_events.borrow_mut().clear();
        self.declare.callbacks.invalidate_all();
        if let Some(ctx) = self.declare.ctx.as_mut() {
            ctx.timers.clear();
        }
        self.init_state()?;
        for (i, v) in carried {
            if let Some(slot) = self.globals.get_mut(i) {
                *slot = v;
            }
        }
        Ok(())
    }
}

fn static_name(p: &Program, s: Option<vba_sema::ProcId>) -> Option<String> {
    s.map(|id| p.proc(id).name.to_string().to_ascii_lowercase())
}

fn global_label(p: &Program, i: usize) -> String {
    let g = &p.globals[i];
    let m = p.module(g.module).name.to_string();
    match g.static_of {
        Some(pr) => format!("{m}.{}.{}", p.proc(pr).name, g.name),
        None => format!("{m}.{}", g.name),
    }
}

/// Plain data: no object owned by the VM and no user-defined-type value
/// anywhere inside (host objects are plain: the host table owns them).
fn is_plain(v: &Variant) -> bool {
    match v {
        Variant::UserDefined(_) => false,
        Variant::Object(ObjectRef::Handle(h)) => vm_id(*h).is_none(),
        Variant::Array(a) => a.iter().all(is_plain),
        _ => true,
    }
}

fn collect_host_handles(v: &Variant, out: &mut Vec<vba_value::ObjectHandle>) {
    match v {
        Variant::Object(ObjectRef::Handle(h)) if vm_id(*h).is_none() => out.push(*h),
        Variant::Array(a) => a.iter().for_each(|x| collect_host_handles(x, out)),
        _ => {}
    }
}

/// Whether two programs declare the same modules, procedures, variables and
/// types in the same order, so that every id means the same thing in both.
pub fn same_layout(a: &Program, b: &Program) -> bool {
    a.modules.len() == b.modules.len()
        && a.procs.len() == b.procs.len()
        && a.globals.len() == b.globals.len()
        && a.consts.len() == b.consts.len()
        && a.udts.len() == b.udts.len()
        && a.enums.len() == b.enums.len()
        && a.events.len() == b.events.len()
        && a.declares.len() == b.declares.len()
        && a.lib_enums == b.lib_enums
        && a.lib_records == b.lib_records
        && a.modules
            .iter()
            .zip(&b.modules)
            .all(|(x, y)| x.name == y.name && x.kind == y.kind && x.predeclared == y.predeclared)
        && a.procs.iter().zip(&b.procs).all(|(x, y)| {
            x.module == y.module
                && x.name == y.name
                && x.kind == y.kind
                && x.is_static == y.is_static
                && x.params.len() == y.params.len()
                && x.params.iter().zip(&y.params).all(|(p, q)| {
                    p.name == q.name
                        && p.ty == q.ty
                        && p.by_val == q.by_val
                        && p.optional == q.optional
                        && p.param_array == q.param_array
                })
        })
        && a.globals.iter().zip(&b.globals).all(|(x, y)| {
            x.module == y.module
                && x.name == y.name
                && x.ty == y.ty
                && x.static_of == y.static_of
                && x.fixed_bounds == y.fixed_bounds
                && x.with_events == y.with_events
                && x.auto_new == y.auto_new
        })
        && a.consts
            .iter()
            .zip(&b.consts)
            .all(|(x, y)| x.module == y.module && x.name == y.name && x.ty == y.ty)
        && a.udts.iter().zip(&b.udts).all(|(x, y)| {
            x.module == y.module
                && x.name == y.name
                && x.fields.len() == y.fields.len()
                && x.fields.iter().zip(&y.fields).all(|(p, q)| {
                    p.name == q.name && p.ty == q.ty && p.fixed_bounds == q.fixed_bounds
                })
        })
        && a.enums
            .iter()
            .zip(&b.enums)
            .all(|(x, y)| x.module == y.module && x.name == y.name && x.members == y.members)
        && a.events.iter().zip(&b.events).all(|(x, y)| {
            x.module == y.module && x.name == y.name && x.params.len() == y.params.len()
        })
        && a.declares.iter().zip(&b.declares).all(|(x, y)| {
            x.module == y.module && x.name == y.name && x.lib == y.lib && x.alias == y.alias
        })
}

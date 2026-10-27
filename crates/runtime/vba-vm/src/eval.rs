//! Evaluating an expression or statement text in a running program (an
//! "immediate window").
//!
//! The text is compiled together with the project: an extra standard module,
//! [`EVAL_MODULE`], is appended **last** and holds one procedure,
//! [`EVAL_PROC`], whose body is the text. A compiler supplied by the
//! embedding layer ([`EvalCompiler`]) builds that program; the VM accepts it
//! only if it is the running program plus that one module (so every module,
//! procedure, variable and type keeps its id), runs the procedure on the live
//! state under the same budgets, capabilities and trust as any other call, and
//! puts the running program back.
//!
//! The names the text may use are the module-level names of the context
//! module (private ones included, for a standard module), the project's
//! public names, and, in a frame, the frame's parameters and locals, passed
//! to the procedure as parameters. A `ByRef` copy of each plain local is
//! written back when the procedure ends, so `x = 5` assigns the local.
//! Objects, user-defined types and `Static` locals are passed by value:
//! their members can be read and called, but `Set o = ...` in the text does
//! not rebind the frame's variable.

use std::rc::Rc;

use vba_ir::IrProgram;
use vba_value::{ObjectRef, Variant};

use crate::error::VmError;
use crate::printfmt::print_value;
use crate::state::{Loc, Slot, vm_handle};
use crate::vm::{Arg, Vm};

/// The name of the module the evaluated text lives in.
pub const EVAL_MODULE: &str = "VbaRsImmediate";
/// The procedure of [`EVAL_MODULE`] that holds the text.
pub const EVAL_PROC: &str = "Evaluate";
/// The name `Me` is passed under: the parameter is called `Me`, which takes
/// the place of the keyword inside the procedure.
pub const EVAL_ME: &str = "Me";

/// What to compile; see [`EvalCompiler`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRequest {
    /// The text: an expression (when `expression`) or statements.
    pub text: String,
    /// `?`-form: the text is an expression whose value is returned.
    pub expression: bool,
    /// The module the text is evaluated in: its module-level names are
    /// visible unqualified, private ones included.
    pub module: Option<String>,
    /// Names of the values passed in, in parameter order (each is a
    /// `Variant`, `ByRef` for the ones the VM writes back).
    pub params: Vec<String>,
    /// Whether a last parameter named [`EVAL_ME`] (an `Object`) follows.
    pub me: bool,
}

/// Builds the program for an evaluation. The result must be the running
/// program with [`EVAL_MODULE`] appended as the last module, holding the one
/// procedure [`EVAL_PROC`] with the parameters of the request.
pub trait EvalCompiler {
    /// Compiles the request, or reports rendered diagnostics.
    fn compile(&self, request: &EvalRequest) -> Result<IrProgram, Vec<String>>;
}

/// What an evaluation produced.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EvalOutput {
    /// The value of a `?` expression; `None` for statements.
    pub value: Option<Variant>,
    /// The value as `Debug.Print` would write it (without the line end);
    /// empty for statements.
    pub text: String,
}

/// Why an evaluation did not produce a value.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EvalError {
    /// No compiler was installed in this VM.
    NoCompiler,
    /// The frame index does not exist.
    NoSuchFrame,
    /// The context module does not exist.
    NoSuchModule(String),
    /// The text is not acceptable as one expression or statement list
    /// (it contains a line break, or it does not compile); the rendered
    /// diagnostics.
    Compile(Vec<String>),
    /// The text compiled and ran, and failed. A VBA error is reported here
    /// without ending the program that is paused; the state of `Err` is as
    /// it was before.
    Run(Box<VmError>),
    /// Writing a changed local back failed (for example `i = 70000` for an
    /// `Integer`); the other locals were written.
    WriteBack(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::NoCompiler => f.write_str("evaluation is not available in this VM"),
            EvalError::NoSuchFrame => f.write_str("no such frame"),
            EvalError::NoSuchModule(m) => write!(f, "no such module: {m}"),
            EvalError::Compile(d) => f.write_str(&d.join("\n")),
            EvalError::Run(e) => write!(f, "{e}"),
            EvalError::WriteBack(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for EvalError {}

impl Vm {
    /// Installs the compiler [`Vm::eval`] uses.
    pub fn set_eval_compiler(&mut self, compiler: Option<Rc<dyn EvalCompiler>>) {
        self.eval_compiler = compiler;
    }

    /// Evaluates `text` in the scope of a module (`None`: only the project's
    /// public names). `?expr` is an expression whose value is returned;
    /// anything else is a list of statements (`x = 1`, `Call P`,
    /// `Debug.Print 3`). The text must be a single line.
    ///
    /// It runs on the live state, under the same trust, capabilities and
    /// budgets as any call, and does not pause for breakpoints. Not usable
    /// while the VM is running unless it is paused in a debugger callback
    /// (use [`DebugContext::eval`](crate::DebugContext::eval) for the
    /// frame's locals).
    pub fn eval(&mut self, module: Option<&str>, text: &str) -> Result<EvalOutput, EvalError> {
        if let Some(m) = module
            && self.ir.program.module_by_name(m).is_none()
        {
            return Err(EvalError::NoSuchModule(m.to_owned()));
        }
        self.eval_common(None, module.map(str::to_owned), text)
    }

    pub(crate) fn eval_in_frame(&mut self, fi: usize, text: &str) -> Result<EvalOutput, EvalError> {
        let module = {
            let f = &self.frames[fi];
            self.ir.program.module(f.module).name.to_string()
        };
        self.eval_common(Some(fi), Some(module), text)
    }

    fn eval_common(
        &mut self,
        frame: Option<usize>,
        module: Option<String>,
        text: &str,
    ) -> Result<EvalOutput, EvalError> {
        let compiler = self.eval_compiler.clone().ok_or(EvalError::NoCompiler)?;
        let trimmed = text.trim();
        let (expression, body) = match trimmed.strip_prefix('?') {
            Some(rest) => (true, rest.trim()),
            None => (false, trimmed),
        };
        if body.is_empty() {
            return Err(EvalError::Compile(vec!["nothing to evaluate".into()]));
        }
        if body.contains(['\n', '\r']) {
            return Err(EvalError::Compile(vec![
                "evaluate one line at a time (no line breaks)".into(),
            ]));
        }
        // The values the frame offers, by name.
        let mut names: Vec<String> = Vec::new();
        let mut args: Vec<Arg> = Vec::new();
        let mut back: Vec<(usize, u32, vba_sema::Type)> = Vec::new();
        let mut me = false;
        if let Some(fi) = frame {
            let ir = self.ir.clone();
            let f = &self.frames[fi];
            if let vba_ir::FuncKind::Proc(p) = ir.functions[f.func].kind {
                let pi = ir.program.proc(p);
                for (i, l) in pi.locals.iter().enumerate() {
                    let name = l.name.to_string();
                    if name.is_empty()
                        || matches!(l.kind, vba_sema::LocalKind::Temp)
                        || names.iter().any(|n| n.eq_ignore_ascii_case(&name))
                        || name.eq_ignore_ascii_case(EVAL_PROC)
                    {
                        continue;
                    }
                    let Some(slot) = f.slots.get(i) else { continue };
                    let value = match slot {
                        Slot::Val(v) => v.clone(),
                        Slot::Bound(loc) => self.load(loc).unwrap_or(Variant::Empty),
                    };
                    let plain = !matches!(
                        value,
                        Variant::Object(_) | Variant::UserDefined(_) | Variant::Error(_)
                    ) && !matches!(l.ty, vba_sema::Type::Udt(_));
                    names.push(name);
                    if plain {
                        back.push((args.len(), i as u32, l.ty.clone()));
                        args.push(Arg::ByRef(value));
                    } else {
                        args.push(Arg::Value(value));
                    }
                }
            }
            if let Some(id) = self.frames[fi].me {
                me = true;
                args.push(Arg::Value(Variant::Object(ObjectRef::Handle(vm_handle(id)))));
            }
        }
        let request = EvalRequest {
            text: body.to_owned(),
            expression,
            module,
            params: names,
            me,
        };
        let program = compiler.compile(&request).map_err(EvalError::Compile)?;
        let program = Rc::new(program);
        let proc_id = self.check_extension(&program)?;

        // Swap the extended program in, run, swap back.
        let old_ir = std::mem::replace(&mut self.ir, program);
        let mut lines = std::mem::take(&mut self.source_lines);
        if proc_id.index() <= lines.len() {
            lines.insert(proc_id.index(), Vec::new());
        }
        self.source_lines = lines;
        self.reindex();
        let saved_err = (self.err.clone(), self.erl);
        if let Some(d) = self.debugger.as_mut() {
            d.suppress += 1;
        }
        let paused = !self.frames.is_empty();
        if paused {
            self.nesting += 1;
        }
        let r = self.run_top(proc_id, None, args);
        if paused {
            self.nesting -= 1;
        }
        if let Some(d) = self.debugger.as_mut() {
            d.suppress -= 1;
        }
        let mut lines = std::mem::take(&mut self.source_lines);
        if proc_id.index() < lines.len() {
            lines.remove(proc_id.index());
        }
        self.source_lines = lines;
        self.ir = old_ir;
        self.reindex();

        match r {
            Err(e) => {
                if matches!(e, VmError::Runtime(_)) {
                    self.err = saved_err.0;
                    self.erl = saved_err.1;
                }
                self.last_site = None;
                Err(EvalError::Run(Box::new(e)))
            }
            Ok(out) => {
                let mut failed: Option<String> = None;
                if let Some(fi) = frame {
                    for (k, slot, ty) in back {
                        let Some(v) = out.byref.get(k).cloned() else {
                            continue;
                        };
                        let loc = Loc::Local {
                            frame: fi,
                            slot,
                        };
                        let r = self.convert_value(v, &ty).and_then(|v| self.store(&loc, v));
                        if let Err(f) = r {
                            let e = self.fault_to_error(f);
                            failed.get_or_insert(e.to_string());
                        }
                    }
                }
                self.last_site = None;
                if let Some(msg) = failed {
                    return Err(EvalError::WriteBack(msg));
                }
                if expression {
                    let text = print_value(&out.value);
                    Ok(EvalOutput {
                        value: Some(out.value),
                        text,
                    })
                } else {
                    Ok(EvalOutput {
                        value: None,
                        text: String::new(),
                    })
                }
            }
        }
    }

    /// Checks that `new` is the running program plus the evaluation module,
    /// and returns the id of the evaluation procedure.
    fn check_extension(&self, new: &IrProgram) -> Result<vba_sema::ProcId, EvalError> {
        let bad = |why: &str| EvalError::Compile(vec![format!("internal: {why}")]);
        let (o, n) = (&self.ir.program, &new.program);
        if n.modules.len() != o.modules.len() + 1
            || n.procs.len() != o.procs.len() + 1
            || n.globals.len() != o.globals.len()
            || n.consts.len() != o.consts.len()
            || n.udts.len() != o.udts.len()
            || n.enums.len() != o.enums.len()
            || n.declares.len() != o.declares.len()
            || n.events.len() != o.events.len()
        {
            return Err(EvalError::Compile(vec![
                "the text must be an expression or statements, not declarations".into(),
            ]));
        }
        let last = n.modules.last().ok_or_else(|| bad("no modules"))?;
        if last.name.to_string() != EVAL_MODULE || last.procs.len() != 1 {
            return Err(bad("the evaluation module is not last"));
        }
        for (a, b) in o.modules.iter().zip(&n.modules) {
            if a.name != b.name || a.kind != b.kind {
                return Err(bad("a module changed"));
            }
        }
        for (a, b) in o.procs.iter().zip(&n.procs) {
            if a.name != b.name || a.module != b.module || a.kind != b.kind {
                return Err(bad("a procedure changed"));
            }
        }
        for (a, b) in o.globals.iter().zip(&n.globals) {
            if a.name != b.name || a.module != b.module || a.ty != b.ty {
                return Err(bad("a variable changed"));
            }
        }
        let id = last.procs[0];
        if id.index() != o.procs.len() || !n.proc(id).name.eq_ignore_case(EVAL_PROC) {
            return Err(bad("the evaluation procedure is not last"));
        }
        if new.functions.len() != self.ir.functions.len() + 1 {
            return Err(bad("function table"));
        }
        Ok(id)
    }
}

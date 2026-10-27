//! Text dump of the IR, for tests and debugging.

use std::fmt::Write;

use vba_ast::CompareMode;
use vba_sema::bound::Constant;
use vba_sema::{GlobalId, LocalId};
use vba_value::Variant;
use vba_value::ops::{ArithOp, CompareOp, LogicOp, Overflow, StringNumberRule};

use crate::ir::*;

/// Dump options.
#[derive(Debug, Clone, Copy, Default)]
pub struct DumpOptions {
    /// Append `; stmt#n @start..end` to every instruction.
    pub spans: bool,
    /// Print the statement table.
    pub stmts: bool,
}

/// Dumps the whole program.
pub fn dump_program(p: &IrProgram, opts: DumpOptions) -> String {
    let mut out = String::new();
    for f in &p.functions {
        out.push_str(&dump_function(p, f, opts));
        out.push('\n');
    }
    out
}

/// Dumps one function.
pub fn dump_function(p: &IrProgram, f: &Function, opts: DumpOptions) -> String {
    let mut pr = Printer {
        p,
        f,
        out: String::new(),
        opts,
    };
    pr.function();
    pr.out
}

struct Printer<'a> {
    p: &'a IrProgram,
    f: &'a Function,
    out: String,
    opts: DumpOptions,
}

fn fmt_variant(v: &Variant) -> String {
    match v {
        Variant::Empty => "Empty".into(),
        Variant::Null => "Null".into(),
        Variant::Boolean(b) => if *b { "True" } else { "False" }.into(),
        Variant::Byte(x) => format!("{x}:Byte"),
        Variant::Integer(x) => format!("{x}%"),
        Variant::Long(x) => format!("{x}&"),
        Variant::LongLong(x) => format!("{x}^"),
        Variant::Single(x) => format!("{x:?}!"),
        Variant::Double(x) => format!("{x:?}#"),
        Variant::Currency(c) => format!("{c}@"),
        Variant::Date(d) => format!("#{:?}#", d.serial()),
        Variant::String(s) => format!("{:?}", s.to_string_lossy()),
        Variant::Object(vba_value::ObjectRef::Nothing) => "Nothing".into(),
        other => format!("{other:?}"),
    }
}

impl Printer<'_> {
    fn ty(&self, t: &vba_sema::Type) -> String {
        self.p.program.type_name(t)
    }

    fn operand(&self, o: &Operand) -> String {
        match o {
            Operand::Reg(r) => format!("r{}", r.0),
            Operand::Const(Constant::Value(v)) => fmt_variant(v),
            Operand::Const(Constant::DateText(t) | Constant::YearlessDate(t)) => {
                format!("#{}#", String::from_utf8_lossy(t))
            }
        }
    }

    fn operands(&self, os: &[Operand]) -> String {
        os.iter()
            .map(|o| self.operand(o))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn slot(&self, s: LocalId) -> String {
        match self.f.slots.get(s.index()) {
            Some(sl) if !sl.name.as_bytes().is_empty() => format!("s{}:{}", s.0, sl.name),
            _ => format!("s{}", s.0),
        }
    }

    fn global(&self, g: GlobalId) -> String {
        let gi = self.p.program.global(g);
        format!(
            "g{}:{}.{}",
            g.0,
            self.p.program.module(gi.module).name,
            gi.name
        )
    }

    fn call_arg(&self, a: &CallArg) -> String {
        match a {
            CallArg::Val(o) => self.operand(o),
            CallArg::Ref(r) => format!("ref r{}", r.0),
            CallArg::Missing => "missing".into(),
        }
    }

    fn late_arg(&self, a: &LateArg) -> String {
        match &a.name {
            Some(n) => format!("{n}:={}", self.call_arg(&a.arg)),
            None => self.call_arg(&a.arg),
        }
    }

    fn mode(m: StoreMode) -> &'static str {
        match m {
            StoreMode::Let => "let",
            StoreMode::Set => "set",
            StoreMode::SetWithEvents => "set.events",
            StoreMode::LSet => "lset",
            StoreMode::RSet => "rset",
        }
    }

    fn overflow(o: Overflow) -> &'static str {
        match o {
            Overflow::Error => "typed",
            Overflow::Promote => "variant",
        }
    }

    fn binop(&self, op: &BinOp) -> String {
        match op {
            BinOp::Arith(a) => match a {
                ArithOp::Add => "add",
                ArithOp::Sub => "sub",
                ArithOp::Mul => "mul",
                ArithOp::Div => "div",
                ArithOp::IntDiv => "idiv",
                ArithOp::Mod => "mod",
                ArithOp::Pow => "pow",
            }
            .into(),
            BinOp::Concat => "concat".into(),
            BinOp::Compare(c, o) => {
                let name = match c {
                    CompareOp::Eq => "eq",
                    CompareOp::Ne => "ne",
                    CompareOp::Lt => "lt",
                    CompareOp::Gt => "gt",
                    CompareOp::Le => "le",
                    CompareOp::Ge => "ge",
                };
                let mode = match o.mode {
                    vba_value::CompareMode::Binary => "bin",
                    vba_value::CompareMode::Text => "text",
                };
                let rule = match o.string_number {
                    StringNumberRule::Coerce => "+coerce",
                    StringNumberRule::VariantOrdering => "",
                };
                format!("{name}[{mode}{rule}]")
            }
            BinOp::Logic(l) => match l {
                LogicOp::And => "and",
                LogicOp::Or => "or",
                LogicOp::Xor => "xor",
                LogicOp::Eqv => "eqv",
                LogicOp::Imp => "imp",
            }
            .into(),
            BinOp::Like(m) => format!(
                "like[{}]",
                match m {
                    CompareMode::Binary => "bin",
                    _ => "text",
                }
            ),
            BinOp::Is => "is".into(),
        }
    }

    fn ref_place(&self, p: &RefPlace) -> String {
        match p {
            RefPlace::Local(s) => self.slot(*s),
            RefPlace::Global(g) => self.global(*g),
            RefPlace::Field { obj, global } => {
                format!("{}.{}", self.operand(obj), self.global(*global))
            }
            RefPlace::Elem { base, indices } => {
                format!("r{}[{}]", base.0, self.operands(indices))
            }
            RefPlace::UdtField { base, udt, index } => {
                let p = &self.p.program;
                format!(
                    "r{}.{}.{}",
                    base.0,
                    p.udt_name(*udt),
                    p.udt_fields(*udt)[*index as usize].name
                )
            }
        }
    }

    fn new_target(&self, t: &NewTarget) -> String {
        match t {
            NewTarget::Class(m) => format!("class {}", self.p.program.module(*m).name),
            NewTarget::Library(l) => format!("lib {l}"),
        }
    }

    fn callee(&self, c: &Callee) -> String {
        match c {
            Callee::Proc(p) => {
                let pi = self.p.program.proc(*p);
                format!("{}.{}", self.p.program.module(pi.module).name, pi.name)
            }
            Callee::Declare(d) => {
                let di = &self.p.program.declares[d.index()];
                format!("declare {}", di.name)
            }
            Callee::Intrinsic { name, dollar } => {
                format!("intrinsic {name}{}", if *dollar { "$" } else { "" })
            }
        }
    }

    fn out_items(&self, items: &[OutItem]) -> String {
        items
            .iter()
            .map(|i| match i {
                OutItem::Value(o) => self.operand(o),
                OutItem::Semicolon => ";".into(),
                OutItem::Comma => ",".into(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn opt_op(&self, o: &Option<Operand>) -> String {
        o.as_ref().map_or("-".into(), |o| self.operand(o))
    }

    fn io(&self, op: &IoOp) -> String {
        match op {
            IoOp::Open {
                path,
                mode,
                access,
                lock,
                file,
                record_len,
            } => format!(
                "io.open {} for {mode:?} access {access:?} lock {lock:?} as {} len {}",
                self.operand(path),
                self.operand(file),
                self.opt_op(record_len)
            ),
            IoOp::Close(fs) if fs.is_empty() => "io.close.all".to_owned(),
            IoOp::Close(fs) => format!("io.close {}", self.operands(fs)),
            IoOp::Print { file, items } => {
                format!(
                    "io.print {} : {}",
                    self.operand(file),
                    self.out_items(items)
                )
            }
            IoOp::Write { file, items } => {
                format!(
                    "io.write {} : {}",
                    self.operand(file),
                    self.out_items(items)
                )
            }
            IoOp::Input { file, targets } => format!(
                "io.input {} -> {}",
                self.operand(file),
                targets
                    .iter()
                    .map(|r| format!("r{}", r.0))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            IoOp::LineInput { file, target } => {
                format!("io.lineinput {} -> r{}", self.operand(file), target.0)
            }
            IoOp::Get {
                file,
                record,
                target,
            } => format!(
                "io.get {} rec {} -> {}",
                self.operand(file),
                self.opt_op(record),
                target.map_or("-".into(), |r| format!("r{}", r.0))
            ),
            IoOp::Put {
                file,
                record,
                value,
            } => format!(
                "io.put {} rec {} <- {}",
                self.operand(file),
                self.opt_op(record),
                self.opt_op(value)
            ),
            IoOp::Seek { file, position } => {
                format!("io.seek {} {}", self.operand(file), self.opt_op(position))
            }
            IoOp::Lock { file, from, to } => format!(
                "io.lock {} {} {}",
                self.operand(file),
                self.opt_op(from),
                self.opt_op(to)
            ),
            IoOp::Unlock { file, from, to } => format!(
                "io.unlock {} {} {}",
                self.operand(file),
                self.opt_op(from),
                self.opt_op(to)
            ),
            IoOp::Width { file, width } => {
                format!("io.width {} {}", self.operand(file), self.opt_op(width))
            }
            IoOp::Name { old, new } => {
                format!("io.name {} -> {}", self.operand(old), self.operand(new))
            }
        }
    }

    fn inst(&self, i: &InstKind) -> String {
        match i {
            InstKind::Copy { dst, src } => format!("r{} = copy {}", dst.0, self.operand(src)),
            InstKind::Unary {
                dst,
                op,
                ty,
                overflow,
                src,
            } => format!(
                "r{} = {}.{}:{} {}",
                dst.0,
                match op {
                    UnOp::Neg => "neg",
                    UnOp::Plus => "plus",
                    UnOp::Not => "not",
                },
                Self::overflow(*overflow),
                self.ty(ty),
                self.operand(src)
            ),
            InstKind::Binary {
                dst,
                op,
                ty,
                overflow,
                lhs,
                rhs,
                guard,
            } => format!(
                "r{} = {}.{}:{} {}, {}{}",
                dst.0,
                self.binop(op),
                Self::overflow(*overflow),
                self.ty(ty),
                self.operand(lhs),
                self.operand(rhs),
                guard.map_or_else(String::new, |g| format!(" guard r{}", g.0))
            ),
            InstKind::Convert { dst, src, to } => {
                format!("r{} = convert:{} {}", dst.0, self.ty(to), self.operand(src))
            }
            InstKind::LoadLocal { dst, slot } => {
                format!("r{} = load.local {}", dst.0, self.slot(*slot))
            }
            InstKind::StoreLocal { slot, src, mode } => format!(
                "store.local {} <- {} [{}]",
                self.slot(*slot),
                self.operand(src),
                Self::mode(*mode)
            ),
            InstKind::LoadGlobal { dst, global } => {
                format!("r{} = load.global {}", dst.0, self.global(*global))
            }
            InstKind::StoreGlobal { global, src, mode } => format!(
                "store.global {} <- {} [{}]",
                self.global(*global),
                self.operand(src),
                Self::mode(*mode)
            ),
            InstKind::LoadField { dst, obj, global } => format!(
                "r{} = load.field {}.{}",
                dst.0,
                self.operand(obj),
                self.global(*global)
            ),
            InstKind::StoreField {
                obj,
                global,
                src,
                mode,
            } => format!(
                "store.field {}.{} <- {} [{}]",
                self.operand(obj),
                self.global(*global),
                self.operand(src),
                Self::mode(*mode)
            ),
            InstKind::LoadMe { dst } => format!("r{} = me", dst.0),
            InstKind::LoadDefaultInstance { dst, module } => format!(
                "r{} = default.instance {}",
                dst.0,
                self.p.program.module(*module).name
            ),
            InstKind::LoadLibGlobal { dst, name } => format!("r{} = lib.global {name}", dst.0),
            InstKind::MakeRef { dst, place } => {
                format!("r{} = ref {}", dst.0, self.ref_place(place))
            }
            InstKind::LoadRef { dst, r } => format!("r{} = load.ref r{}", dst.0, r.0),
            InstKind::StoreRef { r, src, mode } => format!(
                "store.ref r{} <- {} [{}]",
                r.0,
                self.operand(src),
                Self::mode(*mode)
            ),
            InstKind::Index {
                dst,
                array,
                indices,
            } => format!(
                "r{} = index {}[{}]",
                dst.0,
                self.operand(array),
                self.operands(indices)
            ),
            InstKind::UdtField {
                dst,
                base,
                udt,
                index,
            } => {
                let p = &self.p.program;
                format!(
                    "r{} = udt.field {}.{}.{}",
                    dst.0,
                    self.operand(base),
                    p.udt_name(*udt),
                    p.udt_fields(*udt)[*index as usize].name
                )
            }
            InstKind::Call {
                dst,
                callee,
                receiver,
                args,
            } => {
                let d = dst.map_or(String::new(), |d| format!("r{} = ", d.0));
                let on = receiver
                    .as_ref()
                    .map_or(String::new(), |r| format!(" on {}", self.operand(r)));
                format!(
                    "{d}call {}({}){on}",
                    self.callee(callee),
                    args.iter()
                        .map(|a| self.call_arg(a))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            InstKind::Invoke {
                dst,
                receiver,
                member,
                kind,
                args,
                static_receiver,
                dispid,
            } => {
                let d = dst.map_or(String::new(), |d| format!("r{} = ", d.0));
                let m = match member {
                    Member::Named(n) => match dispid {
                        Some(id) => format!(".{n}#{id}"),
                        None => format!(".{n}"),
                    },
                    Member::Default => ".<default>".into(),
                };
                let k = match kind {
                    InvokeKind::Get => "get",
                    InvokeKind::Put => "put",
                    InvokeKind::PutRef => "putref",
                };
                let st = static_receiver
                    .as_ref()
                    .map_or(String::new(), |l| format!(" ; {l}"));
                format!(
                    "{d}invoke.{k} {}{m}({}){st}",
                    self.operand(receiver),
                    args.iter()
                        .map(|a| self.late_arg(a))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            InstKind::New { dst, target } => {
                format!("r{} = new {}", dst.0, self.new_target(target))
            }
            InstKind::EnsureNew { r, target } => {
                format!("ensure.new r{} {}", r.0, self.new_target(target))
            }
            InstKind::AddressOf { dst, func } => match func {
                FuncRef::Proc(p) => {
                    format!("r{} = addressof {}", dst.0, self.callee(&Callee::Proc(*p)))
                }
                FuncRef::Declare(d) => {
                    format!(
                        "r{} = addressof {}",
                        dst.0,
                        self.callee(&Callee::Declare(*d))
                    )
                }
            },
            InstKind::TypeOfIs { dst, src, ty } => format!(
                "r{} = typeof.is {} {}",
                dst.0,
                self.operand(src),
                self.ty(ty)
            ),
            InstKind::ReDim {
                r,
                preserve,
                bounds,
                elem,
            } => format!(
                "redim{} r{} as {} ({})",
                if *preserve { ".preserve" } else { "" },
                r.0,
                self.ty(elem),
                bounds
                    .iter()
                    .map(|(l, u)| match l {
                        Some(l) => format!("{} To {}", self.operand(l), self.operand(u)),
                        None => self.operand(u),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            InstKind::DimFixed { r, bounds, elem } => format!(
                "dim.fixed r{} as {} ({})",
                r.0,
                self.ty(elem),
                bounds
                    .iter()
                    .map(|(l, u)| format!("{l} To {u}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            InstKind::Erase { r } => format!("erase r{}", r.0),
            InstKind::Undefined { dst } => format!("r{} = undefined", dst.0),
            InstKind::IterInit { dst, src } => {
                format!("r{} = iter.init {}", dst.0, self.operand(src))
            }
            InstKind::ForTest {
                dst,
                counter,
                end,
                step,
            } => format!(
                "r{} = for.test {}, {}, {}",
                dst.0,
                self.operand(counter),
                self.operand(end),
                self.operand(step)
            ),
            InstKind::SetErrorMode { mode } => match mode {
                ErrMode::Propagate => "on.error propagate".into(),
                ErrMode::Handler(b) => format!("on.error goto b{}", b.0),
                ErrMode::ResumeNext => "on.error resume.next".into(),
            },
            InstKind::ClearError => "clear.error".into(),
            InstKind::Stop => "stop".into(),
            InstKind::SetLine { line } => format!("set.line {line}"),
            InstKind::RaiseEvent { event, args } => {
                let e = &self.p.program.events[event.index()];
                format!(
                    "raise.event {}.{}({})",
                    self.p.program.module(e.module).name,
                    e.name,
                    args.iter()
                        .map(|a| self.call_arg(a))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            InstKind::DebugPrint { items } => format!("debug.print {}", self.out_items(items)),
            InstKind::Io(op) => self.io(op),
        }
    }

    fn term(&self, t: &Terminator) -> String {
        match t {
            Terminator::Jump(b) => format!("jump b{}", b.0),
            Terminator::Branch {
                cond,
                then_block,
                else_block,
            } => format!(
                "branch {} ? b{} : b{}",
                self.operand(cond),
                then_block.0,
                else_block.0
            ),
            Terminator::Switch {
                selector,
                targets,
                default,
            } => format!(
                "switch {} [{}] default b{}",
                self.operand(selector),
                targets
                    .iter()
                    .map(|b| format!("b{}", b.0))
                    .collect::<Vec<_>>()
                    .join(", "),
                default.0
            ),
            Terminator::GoSub { target, ret } => format!("gosub b{} ret b{}", target.0, ret.0),
            Terminator::ReturnFromGoSub => "return.gosub".into(),
            Terminator::ForEachNext {
                iter,
                item,
                body,
                done,
            } => format!(
                "foreach.next r{} -> r{} ? b{} : b{}",
                iter.0, item.0, body.0, done.0
            ),
            Terminator::Resume(k) => match k {
                ResumeKind::Same => "resume".into(),
                ResumeKind::Next => "resume.next".into(),
                ResumeKind::Block(b) => format!("resume b{}", b.0),
            },
            Terminator::Return => "return".into(),
            Terminator::End => "end".into(),
            Terminator::Stop => "stop".into(),
            Terminator::Unreachable => "unreachable".into(),
        }
    }

    fn function(&mut self) {
        let f = self.f;
        let _ = writeln!(self.out, "func {}", f.name);
        for (i, s) in f.slots.iter().enumerate() {
            let role = match s.role {
                SlotRole::Param(n) => format!("param {n}"),
                SlotRole::Result => "result".into(),
                SlotRole::Local => "local".into(),
                SlotRole::Temp => "temp".into(),
            };
            let _ = writeln!(
                self.out,
                "  slot s{i} {} : {} ({role})",
                s.name,
                self.ty(&s.ty)
            );
        }
        for (bi, b) in f.blocks.iter().enumerate() {
            let _ = writeln!(self.out, "  b{bi}:");
            for inst in &b.insts {
                let text = self.inst(&inst.kind);
                if self.opts.spans {
                    let _ = writeln!(
                        self.out,
                        "    {text}    ; stmt#{} @{}..{}",
                        inst.stmt.0, inst.span.start, inst.span.end
                    );
                } else {
                    let _ = writeln!(self.out, "    {text}");
                }
            }
            let t = self.term(&b.term);
            if self.opts.spans {
                let _ = writeln!(
                    self.out,
                    "    {t}    ; stmt#{} @{}..{}",
                    b.term_stmt.0, b.term_span.start, b.term_span.end
                );
            } else {
                let _ = writeln!(self.out, "    {t}");
            }
        }
        if self.opts.stmts {
            for (i, s) in f.stmts.iter().enumerate() {
                let rn = s
                    .resume_next
                    .map_or("-".into(), |p| format!("b{}:{}", p.block.0, p.index));
                let _ = writeln!(
                    self.out,
                    "  stmt#{i} @{}..{} line {:?} resume.next {rn}",
                    s.span.start, s.span.end, s.line
                );
            }
        }
    }
}

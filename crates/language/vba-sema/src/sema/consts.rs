//! Lazy evaluation of module-level constants and enum members.

use vba_value::Variant;

use crate::bound::Constant;
use crate::codes;
use crate::fold::{constant_to_i32, convert_constant, fold};
use crate::types::*;

use super::{CState, ModuleScope, Sema};

impl Sema<'_> {
    /// Evaluates a module-level constant (memoized, cycle-safe).
    pub(crate) fn eval_const(&self, id: ConstId) -> Option<(Constant, Type)> {
        let state = self.const_state.borrow()[id.index()].clone();
        match state {
            CState::Done(c, t) => return Some((c, t)),
            CState::Failed => return None,
            CState::Running => {
                let src = &self.const_src[id.index()];
                self.error(
                    src.module,
                    codes::CONSTANT_EXPRESSION,
                    src.item.name.span,
                    format!(
                        "constant `{}` is defined in terms of itself",
                        src.item.name.name
                    ),
                );
                self.const_state.borrow_mut()[id.index()] = CState::Failed;
                return None;
            }
            CState::Pending => {}
        }
        self.const_state.borrow_mut()[id.index()] = CState::Running;
        let (module, item) = {
            let s = &self.const_src[id.index()];
            (s.module, s.item)
        };
        let scope = ModuleScope { sema: self, module };
        self.eval_ctx.borrow_mut().push(None);
        let result = fold(&scope, &item.value).map_err(|e| (e.span, e.message));
        self.eval_ctx.borrow_mut().pop();
        let out = match result {
            Err((span, msg)) => {
                self.error(module, codes::CONSTANT_EXPRESSION, span, msg);
                None
            }
            Ok((c, t)) => {
                let declared = self.const_declared_type(module, item);
                match declared {
                    Some(dt) if !dt.is_error() => match convert_constant(&c, &dt) {
                        Ok(c2) => Some((c2, dt)),
                        Err(m) => {
                            self.error(
                                module,
                                codes::CONSTANT_EXPRESSION,
                                item.value.span,
                                format!("constant `{}`: {m}", item.name.name),
                            );
                            None
                        }
                    },
                    Some(_) => None,
                    None => Some((c, t)),
                }
            }
        };
        self.const_state.borrow_mut()[id.index()] = match &out {
            Some((c, t)) => CState::Done(c.clone(), t.clone()),
            None => CState::Failed,
        };
        out
    }

    /// The declared type of a constant (`As T` or a suffix), if any.
    pub(crate) fn const_declared_type(
        &self,
        module: ModuleId,
        item: &vba_ast::ConstItem,
    ) -> Option<Type> {
        let t = match (&item.as_clause, item.name.suffix) {
            (Some(a), _) => self.resolve_as_clause(module, a, false),
            (None, Some(s)) => self.suffix_type(s),
            (None, None) => return None,
        };
        if t.is_object()
            || matches!(
                t,
                Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } | Type::Any | Type::Variant
            )
        {
            if !t.is_error() {
                self.error(
                    module,
                    codes::CONSTANT_EXPRESSION,
                    item.name.span,
                    format!("a constant cannot have type {}", self.program.type_name(&t)),
                );
            }
            return Some(Type::Error);
        }
        Some(t)
    }

    /// MS-VBAL 5.2.3.4: the value of an enum member may not refer to a
    /// member that follows it, in the same enum or in a later enum of the
    /// same module.
    pub(crate) fn check_enum_forward_reference(&self, target: EnumId, idx: usize) {
        let Some(&Some((cur, cidx))) = self.eval_ctx.borrow().last() else {
            return;
        };
        let module = self.program.enums[cur.index()].module;
        let forward = if target == cur {
            idx > cidx
        } else {
            self.program.enums[target.index()].module == module && target.index() > cur.index()
        };
        if forward {
            let decl = self.enum_src[cur.index()];
            self.spec_strict(
                module,
                codes::CONSTANT_EXPRESSION,
                decl.members[cidx].span,
                format!(
                    "enum member `{}` refers to a member declared after it",
                    decl.members[cidx].name.name
                ),
            );
        }
    }

    /// Evaluates the value of an enum member.
    pub(crate) fn eval_enum_member(&self, e: EnumId, idx: usize) -> Option<i32> {
        let state = self.enum_state.borrow()[e.index()][idx].clone();
        let module = self.program.enums[e.index()].module;
        let decl = self.enum_src[e.index()];
        match state {
            CState::Done(Constant::Value(Variant::Long(v)), _) => return Some(v),
            CState::Done(..) | CState::Failed => return None,
            CState::Running => {
                self.error(
                    module,
                    codes::CONSTANT_EXPRESSION,
                    decl.members[idx].name.span,
                    format!(
                        "enum member `{}` is defined in terms of itself",
                        decl.members[idx].name.name
                    ),
                );
                self.enum_state.borrow_mut()[e.index()][idx] = CState::Failed;
                return None;
            }
            CState::Pending => {}
        }
        self.enum_state.borrow_mut()[e.index()][idx] = CState::Running;
        let member = &decl.members[idx];
        let out: Option<i32> = match &member.value {
            Some(expr) => {
                let scope = ModuleScope { sema: self, module };
                self.eval_ctx.borrow_mut().push(Some((e, idx)));
                let folded = fold(&scope, expr);
                self.eval_ctx.borrow_mut().pop();
                match folded {
                    Err(er) => {
                        self.error(module, codes::CONSTANT_EXPRESSION, er.span, er.message);
                        None
                    }
                    Ok((c, _)) => match constant_to_i32(&c) {
                        Ok(v) => Some(v),
                        Err(m) => {
                            self.error(
                                module,
                                codes::TYPE_DECLARATION,
                                expr.span,
                                format!("enum member value must fit a Long: {m}"),
                            );
                            None
                        }
                    },
                }
            }
            None if idx == 0 => Some(0),
            None => self.eval_enum_member(e, idx - 1).and_then(|p| {
                p.checked_add(1).or_else(|| {
                    self.error(
                        module,
                        codes::TYPE_DECLARATION,
                        member.name.span,
                        "enum member value overflows Long",
                    );
                    None
                })
            }),
        };
        self.enum_state.borrow_mut()[e.index()][idx] = match out {
            Some(v) => CState::Done(Constant::Value(Variant::Long(v)), Type::Long),
            None => CState::Failed,
        };
        out
    }

    /// Evaluates every constant and enum member and stores the results in
    /// the program tables.
    pub(crate) fn evaluate_all_constants(&mut self) {
        for i in 0..self.const_count() {
            self.eval_const(ConstId(i as u32));
        }
        for e in 0..self.enum_id_count() {
            for i in 0..self.enum_src[e].members.len() {
                self.eval_enum_member(EnumId(e as u32), i);
            }
        }
        for i in 0..self.const_count() {
            if let CState::Done(c, t) = self.const_state.borrow()[i].clone() {
                let info = &mut self.program.consts[i];
                info.value = c;
                info.ty = t;
            } else {
                self.program.consts[i].ty = Type::Error;
            }
        }
        for e in 0..self.enum_id_count() {
            let mut members = Vec::new();
            for (i, m) in self.enum_src[e].members.iter().enumerate() {
                if let CState::Done(Constant::Value(Variant::Long(v)), _) =
                    self.enum_state.borrow()[e][i].clone()
                {
                    members.push((m.name.name.clone(), v));
                } else {
                    members.push((m.name.name.clone(), 0));
                }
            }
            self.program.enums[e].members = members;
        }
    }
}

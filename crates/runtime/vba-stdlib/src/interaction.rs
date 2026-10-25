//! Pure parts of `VBA.Interaction`: `Choose`, `IIf`, `Switch`, `Environ`
//! (through the host) and the `Tab`/`Spc` print helpers.
//!
//! `Choose`, `IIf` and `Switch` are ordinary functions in VBA: **every**
//! argument is evaluated before the call, so they are registered as eager
//! intrinsics (no short-circuit), which is what makes the VM-side behavior
//! identical to Office. `Array`, `Command` and the UI functions are not
//! implemented here.

use crate::call::{Args, Ctx, text};
use crate::error::StdResult;
use vba_value::{Variant, VbaError, convert};

/// `Choose(index, choice1, choice2, ...)`: `index` is rounded to a whole
/// number; outside `1..=n` the result is `Null`; a `Null` index yields `Null`.
pub fn choose(index: &Variant, choices: &[Variant]) -> Result<Variant, VbaError> {
    if index.is_null() {
        return Ok(Variant::Null);
    }
    let i = convert::to_long(index)?;
    Ok(usize::try_from(i)
        .ok()
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| choices.get(i))
        .cloned()
        .unwrap_or(Variant::Null))
}

/// `IIf(expression, truepart, falsepart)`.
pub fn iif(cond: &Variant, t: &Variant, f: &Variant) -> Result<Variant, VbaError> {
    Ok(if convert::to_boolean(cond)? {
        t.clone()
    } else {
        f.clone()
    })
}

/// `Switch(expr1, value1, expr2, value2, ...)`: the value paired with the
/// first true expression, else `Null`. An odd argument count is error 5.
pub fn switch(pairs: &[Variant]) -> Result<Variant, VbaError> {
    if !pairs.len().is_multiple_of(2) {
        return Err(VbaError::INVALID_PROCEDURE_CALL);
    }
    for p in pairs.chunks(2) {
        if !p[0].is_null() && convert::to_boolean(&p[0])? {
            return Ok(p[1].clone());
        }
    }
    Ok(Variant::Null)
}

/// Item of a `Print` list produced by `Tab` and `Spc`; the I/O layer
/// interprets them against the current output column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintHelper {
    /// `Tab(n)`: advance to column `n`; `Tab` alone advances to the next
    /// print zone.
    Tab(Option<i32>),
    /// `Spc(n)`: `n` spaces.
    Spc(i32),
}

// ---------------------------------------------------------------------------
// Registry adapters.

fn rest(a: &Args<'_>, from: usize) -> Vec<Variant> {
    a.items
        .iter()
        .skip(from)
        .map(|v| v.clone().unwrap_or(Variant::Empty))
        .collect()
}

pub(crate) fn f_choose(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(choose(a.req(0)?, &rest(a, 1))?)
}
pub(crate) fn f_iif(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(iif(a.req(0)?, a.req(1)?, a.req(2)?)?)
}
pub(crate) fn f_switch(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(switch(&rest(a, 0))?)
}
pub(crate) fn f_environ(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let v = a.req(0)?;
    if v.is_null() {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    if v.is_numeric_subtype() && !matches!(v, Variant::String(_)) {
        let n = convert::to_long(v)?;
        if n < 1 {
            return Err(VbaError::INVALID_PROCEDURE_CALL.into());
        }
        let s = ctx.host.environ_index(n as usize).unwrap_or_default();
        return Ok(text(s.as_str()));
    }
    let name = convert::to_vba_string(v)?.to_string_lossy();
    Ok(text(ctx.host.environ(&name).unwrap_or_default().as_str()))
}
pub(crate) fn f_tab(_: &mut Ctx<'_>, _: &Args<'_>) -> StdResult<Variant> {
    Err(crate::error::StdError::VmRequired(
        "Tab is a Print helper; use PrintHelper::Tab",
    ))
}
pub(crate) fn f_spc(_: &mut Ctx<'_>, _: &Args<'_>) -> StdResult<Variant> {
    Err(crate::error::StdError::VmRequired(
        "Spc is a Print helper; use PrintHelper::Spc",
    ))
}

/// Builds the `Tab` print helper: negative column counts are clamped to 1
/// by the I/O layer; `None` means the argument was omitted.
pub fn tab(column: Option<i32>) -> PrintHelper {
    PrintHelper::Tab(column)
}

/// Builds the `Spc` print helper; a negative count prints nothing.
pub fn spc(count: i32) -> PrintHelper {
    PrintHelper::Spc(count.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_switch_iif() {
        let c = [Variant::from_text("a"), Variant::from_text("b")];
        assert_eq!(choose(&Variant::Long(2), &c), Ok(Variant::from_text("b")));
        assert_eq!(
            choose(&Variant::Double(1.5), &c),
            Ok(Variant::from_text("b"))
        );
        assert_eq!(choose(&Variant::Long(0), &c), Ok(Variant::Null));
        assert_eq!(choose(&Variant::Long(3), &c), Ok(Variant::Null));
        assert_eq!(choose(&Variant::Null, &c), Ok(Variant::Null));
        let s = [
            Variant::Boolean(false),
            Variant::Long(1),
            Variant::Boolean(true),
            Variant::Long(2),
        ];
        assert_eq!(switch(&s), Ok(Variant::Long(2)));
        assert_eq!(switch(&s[..3]), Err(VbaError::INVALID_PROCEDURE_CALL));
        assert_eq!(switch(&s[..2]), Ok(Variant::Null));
        assert_eq!(
            iif(
                &Variant::Boolean(true),
                &Variant::Long(1),
                &Variant::Long(2)
            ),
            Ok(Variant::Long(1))
        );
        assert_eq!(
            iif(&Variant::Null, &Variant::Long(1), &Variant::Long(2)),
            Err(VbaError::INVALID_USE_OF_NULL)
        );
    }
}

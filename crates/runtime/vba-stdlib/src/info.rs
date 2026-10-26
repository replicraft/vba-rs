//! `VBA.Information` (MS-VBAL §6.1.2.7 and the Microsoft language reference).
//!
//! `IsMissing` needs to know whether the caller omitted an optional
//! parameter, which only the VM can tell; it is a VM intrinsic and the
//! registry reports it as such. `IsObject` and `TypeName` of object values
//! need the VM's object model and are VM-assisted: the pure helpers here
//! answer for what the `Variant` itself carries.

use crate::call::{Args, Ctx, text};
use crate::error::{StdError, StdResult};
use vba_value::{
    Bounds, ObjectRef, VarType, Variant, VbaArray, VbaError, VbaResult, text as vtext,
};

/// `IsArray`.
pub fn is_array(v: &Variant) -> bool {
    matches!(v, Variant::Array(_))
}

/// `IsDate`: a `Date`, or a string that parses as a date or time. Numeric
/// strings are not dates.
pub fn is_date(v: &Variant) -> bool {
    is_date_in_year(v, None)
}

/// [`is_date`] where text written without a year (`Jan 5`) takes
/// `current_year`.
pub fn is_date_in_year(v: &Variant, current_year: Option<i32>) -> bool {
    match v {
        Variant::Date(_) => true,
        Variant::String(s) => {
            vtext::parse_numeric(s).is_err() && vtext::parse_date_in_year(s, current_year).is_ok()
        }
        _ => false,
    }
}

/// `IsNumeric`: numeric subtypes, `Boolean`, `Empty`, and strings that read
/// as numbers.
pub fn is_numeric(v: &Variant) -> bool {
    match v {
        Variant::Empty | Variant::Boolean(_) => true,
        Variant::String(s) => vtext::parse_numeric(s).is_ok(),
        other => other.is_numeric_subtype(),
    }
}

/// `IsObject` for what a `Variant` can tell: true for object references,
/// including `Nothing`. Default-member resolution is the VM's job.
pub fn is_object(v: &Variant) -> bool {
    matches!(v, Variant::Object(_))
}

fn type_name_of(t: VarType) -> &'static str {
    match t {
        VarType::Empty => "Empty",
        VarType::Null => "Null",
        VarType::Integer => "Integer",
        VarType::Long => "Long",
        VarType::Single => "Single",
        VarType::Double => "Double",
        VarType::Currency => "Currency",
        VarType::Date => "Date",
        VarType::String => "String",
        VarType::Object | VarType::DataObject => "Object",
        VarType::Error => "Error",
        VarType::Boolean => "Boolean",
        VarType::Variant => "Variant",
        VarType::Decimal => "Decimal",
        VarType::Byte => "Byte",
        VarType::LongLong => "LongLong",
        VarType::UserDefinedType => "Unknown",
    }
}

/// `TypeName`. Objects other than `Nothing` and user-defined types need the
/// VM for their class name and yield [`StdError::VmRequired`].
pub fn type_name(v: &Variant) -> StdResult<String> {
    Ok(match v {
        Variant::Object(ObjectRef::Nothing) => "Nothing".into(),
        Variant::Object(_) => return Err(StdError::VmRequired("TypeName of an object")),
        Variant::UserDefined(_) => {
            return Err(StdError::VmRequired("TypeName of a user-defined type"));
        }
        Variant::Array(a) => match a.element_type() {
            VarType::Object | VarType::UserDefinedType => {
                return Err(StdError::VmRequired("TypeName of an array of objects"));
            }
            t => format!("{}()", type_name_of(t)),
        },
        other => type_name_of(other.vartype()).into(),
    })
}

/// `LBound`/`UBound`: `upper` selects which; `dim` is 1-based. A non-array is
/// error 13, an invalid dimension or unallocated array error 9.
pub fn bound(v: &Variant, dim: i32, upper: bool) -> VbaResult<i32> {
    let Variant::Array(a) = v else {
        return Err(VbaError::TYPE_MISMATCH);
    };
    let d = u32::try_from(dim).map_err(|_| VbaError::SUBSCRIPT_OUT_OF_RANGE)?;
    if upper { a.ubound(d) } else { a.lbound(d) }
}

/// `Array(...)`: a zero-based (or `Option Base 1`) `Variant` array. With no
/// arguments the result is an allocated array without elements.
pub fn array_of(items: &[Variant], base: i32) -> VbaResult<Variant> {
    if items.is_empty() {
        return Ok(Variant::Array(Box::new(VbaArray::new_empty(
            VarType::Variant,
            base,
        ))));
    }
    let mut a = VbaArray::new_dynamic(VarType::Variant);
    a.redim(&[Bounds::new(base, base + items.len() as i32 - 1)], false)?;
    for (i, v) in items.iter().enumerate() {
        a.set(&[base + i as i32], v.clone())?;
    }
    Ok(Variant::Array(Box::new(a)))
}

// ---------------------------------------------------------------------------
// Registry adapters.

pub(crate) fn f_is_array(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Boolean(is_array(a.req(0)?)))
}
pub(crate) fn f_is_date(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let year = ctx.host.now().parts().year;
    Ok(Variant::Boolean(is_date_in_year(a.req(0)?, Some(year))))
}
pub(crate) fn f_is_empty(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Boolean(a.req(0)?.is_empty()))
}
pub(crate) fn f_is_error(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Boolean(matches!(a.req(0)?, Variant::Error(_))))
}
pub(crate) fn f_is_null(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Boolean(a.req(0)?.is_null()))
}
pub(crate) fn f_is_numeric(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Boolean(is_numeric(a.req(0)?)))
}
pub(crate) fn f_is_object(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Boolean(is_object(a.req(0)?)))
}
pub(crate) fn f_is_missing(_: &mut Ctx<'_>, _: &Args<'_>) -> StdResult<Variant> {
    Err(StdError::VmRequired(
        "IsMissing needs the caller's omitted-argument state",
    ))
}
pub(crate) fn f_type_name(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(text(type_name(a.req(0)?)?.as_str()))
}
pub(crate) fn f_var_type(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Integer(a.req(0)?.vartype_code() as i16))
}
pub(crate) fn f_lbound(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Long(bound(a.req(0)?, a.long_or(1, 1)?, false)?))
}
pub(crate) fn f_ubound(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Long(bound(a.req(0)?, a.long_or(1, 1)?, true)?))
}
pub(crate) fn f_array(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let items: Vec<Variant> = a
        .items
        .iter()
        .map(|v| v.clone().unwrap_or(Variant::Empty))
        .collect();
    Ok(array_of(&items, ctx.array_base)?)
}

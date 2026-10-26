//! Small pure functions of `VBA.Interaction` and `VBA.Conversion` that need
//! nothing from the host: `Partition`, `RGB`, `QBColor`, `IMEStatus` and the
//! Macintosh-only `MacID` / `MacScript`.
//!
//! Sources: the Microsoft Learn pages of each function (cited per item).
//! Rules the pages do not state are tagged **unverified**.

use crate::call::{Args, Ctx, text};
use crate::error::StdResult;
use vba_value::{Variant, VbaError, convert};

/// Rounds a numeric argument to a whole number, ties to even (the
/// documented treatment of the `Partition` arguments).
fn whole(v: &Variant) -> Result<i64, VbaError> {
    let d = convert::to_double(v)?.round_ties_even();
    if !d.is_finite() || d.abs() >= 9.0e18 {
        return Err(VbaError::OVERFLOW);
    }
    Ok(d as i64)
}

/// `Partition(number, start, stop, interval)` (**doc**:
/// <https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/partition-function>).
///
/// Returns `"lower:upper"` of the range that holds `number`. Below `start`
/// the range is `":start-1"`, above `stop` it is `"stop+1:"`, the last range
/// ends at `stop`, and an `interval` of 1 gives `"number:number"` always.
/// Both sides are right-aligned in a field as wide as `stop` has digits
/// (**unverified**: the page's table and its prose disagree about the exact
/// amount of leading blanks, so only the characters other than blanks are
/// covered by tests). Arguments are rounded half to even; a `Null` argument
/// gives `Null`. `start < 0`, `stop <= start` or `interval < 1` is error 5.
pub fn partition(
    number: &Variant,
    start: &Variant,
    stop: &Variant,
    interval: &Variant,
) -> Result<Variant, VbaError> {
    if [number, start, stop, interval].iter().any(|v| v.is_null()) {
        return Ok(Variant::Null);
    }
    let (n, start, stop, interval) = (
        whole(number)?,
        whole(start)?,
        whole(stop)?,
        whole(interval)?,
    );
    if start < 0 || stop <= start || interval < 1 {
        return Err(VbaError::INVALID_PROCEDURE_CALL);
    }
    let (lower, upper): (Option<i64>, Option<i64>) = if interval == 1 {
        (Some(n), Some(n))
    } else if n < start {
        (None, Some(start - 1))
    } else if n > stop {
        (Some(stop + 1), None)
    } else {
        let lo = start + (n - start) / interval * interval;
        (Some(lo), Some((lo + interval - 1).min(stop)))
    };
    let width = stop.to_string().len();
    let side = |v: Option<i64>| match v {
        Some(v) => format!("{v:>width$}"),
        None => " ".repeat(width),
    };
    Ok(text(format!("{}:{}", side(lower), side(upper)).as_str()))
}

/// `RGB(red, green, blue)` (**doc**:
/// <https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/rgb-function>):
/// `red + 256 * green + 65536 * blue`; an argument above 255 counts as 255.
/// A negative argument is error 5 (**unverified**).
pub fn rgb(r: &Variant, g: &Variant, b: &Variant) -> Result<i32, VbaError> {
    let part = |v: &Variant| -> Result<i32, VbaError> {
        let n = convert::to_long(v)?;
        if n < 0 {
            return Err(VbaError::INVALID_PROCEDURE_CALL);
        }
        Ok(n.min(255))
    };
    Ok(part(r)? | (part(g)? << 8) | (part(b)? << 16))
}

/// The RGB values of `QBColor(0..=15)`. The page names the 16 colors but
/// gives no numbers; these are the standard QuickBasic palette
/// (**unverified** against a real VBA).
const QB_COLORS: [i32; 16] = [
    0x000000, 0x800000, 0x008000, 0x808000, 0x000080, 0x800080, 0x008080, 0xC0C0C0, 0x808080,
    0xFF0000, 0x00FF00, 0xFFFF00, 0x0000FF, 0xFF00FF, 0x00FFFF, 0xFFFFFF,
];

/// `QBColor(color)` (**doc** for the argument range:
/// <https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/qbcolor-function>);
/// outside 0..=15 is error 5.
pub fn qb_color(color: i32) -> Result<i32, VbaError> {
    usize::try_from(color)
        .ok()
        .and_then(|i| QB_COLORS.get(i))
        .copied()
        .ok_or(VbaError::INVALID_PROCEDURE_CALL)
}

pub(crate) fn f_partition(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(partition(a.req(0)?, a.req(1)?, a.req(2)?, a.req(3)?)?)
}

pub(crate) fn f_rgb(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Long(rgb(a.req(0)?, a.req(1)?, a.req(2)?)?))
}

pub(crate) fn f_qb_color(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Long(qb_color(a.long(0)?)?))
}

/// `IMEStatus` (**doc**:
/// <https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/imestatus-function>).
/// The page lists the values of East Asian IMEs; the default is
/// `vbIMEModeNoControl` (0), which is what a host without an IME reports.
pub(crate) fn f_ime_status(_: &mut Ctx<'_>, _: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Integer(0))
}

/// `MacID` is documented as Macintosh-only. Decision: on this (non-Mac)
/// implementation it is error 5, "Invalid procedure call" (**unverified**:
/// the page does not state the Windows behavior).
pub(crate) fn f_mac_id(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    a.string(0)?;
    Err(VbaError::INVALID_PROCEDURE_CALL.into())
}

/// `MacScript` is documented as Macintosh-only; see [`f_mac_id`].
pub(crate) fn f_mac_script(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    a.string(0)?;
    Err(VbaError::INVALID_PROCEDURE_CALL.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(n: i64, start: i64, stop: i64, interval: i64) -> String {
        let v = |x: i64| Variant::Long(x as i32);
        match partition(&v(n), &v(start), &v(stop), &v(interval)).unwrap() {
            Variant::String(s) => s.to_string_lossy().replace(' ', ""),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn partition_documented_table() {
        // The table of the Partition page (blanks removed).
        assert_eq!(p(-1, 0, 99, 5), ":-1");
        assert_eq!(p(0, 0, 99, 5), "0:4");
        assert_eq!(p(99, 0, 99, 5), "95:99");
        assert_eq!(p(100, 0, 99, 5), "100:");
        assert_eq!(p(19, 20, 199, 10), ":19");
        assert_eq!(p(20, 20, 199, 10), "20:29");
        assert_eq!(p(199, 20, 199, 10), "190:199");
        assert_eq!(p(200, 20, 199, 10), "200:");
        assert_eq!(p(99, 100, 1010, 20), ":99");
        assert_eq!(p(100, 100, 1010, 20), "100:119");
        assert_eq!(p(1010, 100, 1010, 20), "1000:1010");
        assert_eq!(p(1011, 100, 1010, 20), "1011:");
        // Interval 1: number:number whatever start and stop are.
        assert_eq!(p(100, 0, 1000, 1), "100:100");
    }

    #[test]
    fn partition_errors_and_null() {
        let v = |x: i32| Variant::Long(x);
        assert_eq!(
            partition(&v(1), &v(-1), &v(10), &v(2)),
            Err(VbaError::INVALID_PROCEDURE_CALL)
        );
        assert_eq!(
            partition(&v(1), &v(5), &v(5), &v(2)),
            Err(VbaError::INVALID_PROCEDURE_CALL)
        );
        assert_eq!(
            partition(&v(1), &v(0), &v(5), &v(0)),
            Err(VbaError::INVALID_PROCEDURE_CALL)
        );
        assert_eq!(
            partition(&Variant::Null, &v(0), &v(5), &v(1)),
            Ok(Variant::Null)
        );
        // Arguments are rounded to even: 2.5 -> 2, 3.5 -> 4.
        assert_eq!(
            partition(
                &Variant::Double(2.5),
                &Variant::Double(0.4),
                &v(9),
                &Variant::Double(2.5)
            ),
            partition(&v(2), &v(0), &v(9), &v(2))
        );
    }

    #[test]
    fn rgb_and_qbcolor() {
        let v = Variant::Integer;
        assert_eq!(rgb(&v(255), &v(0), &v(0)), Ok(255));
        assert_eq!(rgb(&v(0), &v(255), &v(0)), Ok(65280));
        assert_eq!(rgb(&v(0), &v(0), &v(255)), Ok(16_711_680));
        assert_eq!(rgb(&v(1), &v(2), &v(3)), Ok(197_121));
        // Above 255 counts as 255.
        assert_eq!(rgb(&v(300), &v(0), &v(0)), Ok(255));
        assert_eq!(
            rgb(&v(-1), &v(0), &v(0)),
            Err(VbaError::INVALID_PROCEDURE_CALL)
        );
        assert_eq!(qb_color(4), Ok(128));
        assert_eq!(qb_color(15), Ok(0xFF_FFFF));
        assert_eq!(qb_color(16), Err(VbaError::INVALID_PROCEDURE_CALL));
        assert_eq!(qb_color(-1), Err(VbaError::INVALID_PROCEDURE_CALL));
    }
}

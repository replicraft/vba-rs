//! `VBA.DateTime` (MS-VBAL §6.1.2.4 and the Microsoft language reference).
//!
//! Intervals: `yyyy` year, `q` quarter, `m` month, `y` day of year, `d` day,
//! `w` weekday, `ww` week, `h` hour, `n` minute, `s` second. `DateAdd`
//! clamps the day to the month end; `DateDiff` counts calendar boundaries
//! crossed (so `DateDiff("yyyy", #12/31/2020#, #1/1/2021#)` is 1) except `w`,
//! which counts whole 7-day periods. Results outside years 100 to 9999 are
//! error 5.

use crate::call::{Args, Ctx, text};
use crate::error::StdResult;
use crate::host::Locale;
use vba_value::{Date, Variant, VbaError, VbaResult, VbaString, convert, date::days_in_month};

const E5: VbaError = VbaError::INVALID_PROCEDURE_CALL;

/// `vbSunday` .. `vbSaturday` first-day-of-week values: 0 means "use the
/// locale", 1..=7 a weekday.
fn first_day(fdow: i32, loc: &Locale) -> VbaResult<i64> {
    match fdow {
        0 => Ok(i64::from(loc.first_day_of_week)),
        1..=7 => Ok(i64::from(fdow)),
        _ => Err(E5),
    }
}

/// `vbUseSystem` (0), `vbFirstJan1` (1), `vbFirstFourDays` (2),
/// `vbFirstFullWeek` (3).
fn first_week(fwoy: i32, loc: &Locale) -> VbaResult<i64> {
    match fwoy {
        0 => Ok(i64::from(loc.first_week_of_year)),
        1..=3 => Ok(i64::from(fwoy)),
        _ => Err(E5),
    }
}

/// The serial day number (days since 1899-12-30) of the calendar day of `d`.
pub fn serial_day(d: Date) -> i64 {
    let p = d.parts();
    Date::date_serial(i64::from(p.year), i64::from(p.month), i64::from(p.day))
        .map(|x| x.serial() as i64)
        .unwrap_or(0)
}

fn secs_of_day(d: Date) -> i64 {
    let p = d.parts();
    i64::from(p.hour) * 3600 + i64::from(p.minute) * 60 + i64::from(p.second)
}

fn from_day_secs(day: i64, secs: i64) -> VbaResult<Date> {
    let day = day + secs.div_euclid(86_400);
    let secs = secs.rem_euclid(86_400);
    let base = Date::from_serial(day as f64).map_err(|_| E5)?;
    let p = base.parts();
    Date::from_ymd_hms(
        p.year,
        p.month,
        p.day,
        (secs / 3600) as u32,
        (secs % 3600 / 60) as u32,
        (secs % 60) as u32,
    )
}

fn weekday_of_day(day: i64) -> i64 {
    (day.rem_euclid(7) + 6) % 7 + 1
}

/// Weekday of `d` relative to a first day of the week: 1 = `fdow`.
pub(crate) fn rel_weekday(d: Date, fdow: i64) -> i64 {
    (weekday_of_day(serial_day(d)) - fdow + 7) % 7 + 1
}

pub(crate) fn day_of_year(d: Date) -> i64 {
    let p = d.parts();
    let jan1 = Date::date_serial(i64::from(p.year), 1, 1).map_or(0, |x| x.serial() as i64);
    serial_day(d) - jan1 + 1
}

fn first_week_start(year: i64, fdow: i64, fwoy: i64) -> VbaResult<i64> {
    let jan1 = Date::date_serial(year, 1, 1).map_err(|_| E5)?.serial() as i64;
    let offset = (weekday_of_day(jan1) - fdow + 7) % 7;
    let ws = jan1 - offset;
    Ok(match fwoy {
        1 => ws,
        2 => {
            if 7 - offset >= 4 {
                ws
            } else {
                ws + 7
            }
        }
        _ => {
            if offset == 0 {
                ws
            } else {
                ws + 7
            }
        }
    })
}

/// Week of the year of `d` for the given first day of week and first week of
/// year rules.
pub fn week_of_year(d: Date, fdow: i64, fwoy: i64) -> VbaResult<i64> {
    let day = serial_day(d);
    let mut year = i64::from(d.parts().year);
    let mut first = first_week_start(year, fdow, fwoy)?;
    while day < first {
        year -= 1;
        first = first_week_start(year, fdow, fwoy)?;
    }
    if fwoy != 1 && day >= first_week_start(year + 1, fdow, fwoy)? {
        return Ok(1);
    }
    Ok((day - first) / 7 + 1)
}

/// The recognised interval codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    /// `yyyy`
    Year,
    /// `q`
    Quarter,
    /// `m`
    Month,
    /// `y`
    DayOfYear,
    /// `d`
    Day,
    /// `w`
    Weekday,
    /// `ww`
    Week,
    /// `h`
    Hour,
    /// `n`
    Minute,
    /// `s`
    Second,
}

impl Interval {
    /// Parses an interval string (case-insensitive); anything else is error 5.
    pub fn parse(s: &VbaString) -> VbaResult<Interval> {
        let t = s.to_string_lossy().to_ascii_lowercase();
        Ok(match t.as_str() {
            "yyyy" => Interval::Year,
            "q" => Interval::Quarter,
            "m" => Interval::Month,
            "y" => Interval::DayOfYear,
            "d" => Interval::Day,
            "w" => Interval::Weekday,
            "ww" => Interval::Week,
            "h" => Interval::Hour,
            "n" => Interval::Minute,
            "s" => Interval::Second,
            _ => return Err(E5),
        })
    }
}

fn add_months(d: Date, months: i64) -> VbaResult<Date> {
    let p = d.parts();
    let total = i64::from(p.year) * 12 + i64::from(p.month) - 1 + months;
    let (y, m) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
    if !(100..=9999).contains(&y) {
        return Err(E5);
    }
    let day = p.day.min(days_in_month(y, m));
    Date::from_ymd_hms(y as i32, m, day, p.hour, p.minute, p.second)
}

/// `DateAdd(interval, number, date)`; `number` is rounded to a whole number
/// by the caller.
pub fn date_add(interval: Interval, number: i64, d: Date) -> VbaResult<Date> {
    let (day, secs) = (serial_day(d), secs_of_day(d));
    match interval {
        Interval::Year => add_months(d, number.checked_mul(12).ok_or(E5)?),
        Interval::Quarter => add_months(d, number.checked_mul(3).ok_or(E5)?),
        Interval::Month => add_months(d, number),
        Interval::DayOfYear | Interval::Day | Interval::Weekday => {
            from_day_secs(day.checked_add(number).ok_or(E5)?, secs)
        }
        Interval::Week => from_day_secs(
            day.checked_add(number.checked_mul(7).ok_or(E5)?)
                .ok_or(E5)?,
            secs,
        ),
        Interval::Hour => from_day_secs(
            day,
            secs.checked_add(number.checked_mul(3600).ok_or(E5)?)
                .ok_or(E5)?,
        ),
        Interval::Minute => from_day_secs(
            day,
            secs.checked_add(number.checked_mul(60).ok_or(E5)?)
                .ok_or(E5)?,
        ),
        Interval::Second => from_day_secs(day, secs.checked_add(number).ok_or(E5)?),
    }
}

/// `DateDiff(interval, date1, date2, firstdayofweek)`.
pub fn date_diff(interval: Interval, d1: Date, d2: Date, fdow: i64) -> VbaResult<i64> {
    let (p1, p2) = (d1.parts(), d2.parts());
    let (day1, day2) = (serial_day(d1), serial_day(d2));
    let q = |p: vba_value::date::DateParts| i64::from(p.year) * 4 + i64::from(p.month - 1) / 3;
    let week = |day: i64| (day - fdow % 7).div_euclid(7);
    Ok(match interval {
        Interval::Year => i64::from(p2.year) - i64::from(p1.year),
        Interval::Quarter => q(p2) - q(p1),
        Interval::Month => {
            (i64::from(p2.year) * 12 + i64::from(p2.month))
                - (i64::from(p1.year) * 12 + i64::from(p1.month))
        }
        Interval::DayOfYear | Interval::Day => day2 - day1,
        Interval::Weekday => (day2 - day1) / 7,
        Interval::Week => week(day2) - week(day1),
        Interval::Hour => (day2 * 24 + i64::from(p2.hour)) - (day1 * 24 + i64::from(p1.hour)),
        Interval::Minute => {
            (day2 * 1440 + i64::from(p2.hour) * 60 + i64::from(p2.minute))
                - (day1 * 1440 + i64::from(p1.hour) * 60 + i64::from(p1.minute))
        }
        Interval::Second => (day2 * 86_400 + secs_of_day(d2)) - (day1 * 86_400 + secs_of_day(d1)),
    })
}

/// `DatePart(interval, date, firstdayofweek, firstweekofyear)`.
pub fn date_part(interval: Interval, d: Date, fdow: i64, fwoy: i64) -> VbaResult<i64> {
    let p = d.parts();
    Ok(match interval {
        Interval::Year => i64::from(p.year),
        Interval::Quarter => i64::from(p.month - 1) / 3 + 1,
        Interval::Month => i64::from(p.month),
        Interval::DayOfYear => day_of_year(d),
        Interval::Day => i64::from(p.day),
        Interval::Weekday => rel_weekday(d, fdow),
        Interval::Week => week_of_year(d, fdow, fwoy)?,
        Interval::Hour => i64::from(p.hour),
        Interval::Minute => i64::from(p.minute),
        Interval::Second => i64::from(p.second),
    })
}

/// `DateSerial`: two-digit years 0..=29 mean 2000..=2029 and 30..=99 mean
/// 1930..=1999 (the default Windows cutoff); negative years are error 5.
pub fn date_serial(year: i32, month: i32, day: i32) -> VbaResult<Date> {
    let y = match year {
        0..=29 => i64::from(year) + 2000,
        30..=99 => i64::from(year) + 1900,
        100.. => i64::from(year),
        _ => return Err(E5),
    };
    Date::date_serial(y, i64::from(month), i64::from(day))
}

/// `TimeSerial`.
pub fn time_serial(h: i32, m: i32, s: i32) -> VbaResult<Date> {
    let f = Date::time_serial(i64::from(h), i64::from(m), i64::from(s))?;
    Date::from_serial(f)
}

/// `Date$`: `mm-dd-yyyy`.
pub fn date_dollar(d: Date) -> String {
    let p = d.parts();
    format!("{:02}-{:02}-{:04}", p.month, p.day, p.year)
}

/// `Time$`: `HH:mm:ss`.
pub fn time_dollar(d: Date) -> String {
    let p = d.parts();
    format!("{:02}:{:02}:{:02}", p.hour, p.minute, p.second)
}

// ---------------------------------------------------------------------------
// Registry adapters.

fn date_arg(a: &Args<'_>, i: usize) -> StdResult<Date> {
    Ok(convert::to_date(a.req(i)?)?)
}

macro_rules! null_guard {
    ($a:expr, $i:expr) => {
        if $a.is_null($i) {
            return Ok(Variant::Null);
        }
    };
}

pub(crate) fn f_now(ctx: &mut Ctx<'_>, _: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Date(ctx.host.now()))
}
pub(crate) fn f_date(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let d = ctx.host.now();
    let day = Date::from_serial(d.serial().trunc())?;
    if a.dollar {
        return Ok(text(date_dollar(day).as_str()));
    }
    Ok(Variant::Date(day))
}
pub(crate) fn f_time(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let d = ctx.host.now();
    let t = Date::from_serial((d.serial() - d.serial().trunc()).abs())?;
    if a.dollar {
        return Ok(text(time_dollar(t).as_str()));
    }
    Ok(Variant::Date(t))
}
pub(crate) fn f_timer(ctx: &mut Ctx<'_>, _: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Single(ctx.host.timer()))
}
pub(crate) fn f_date_add(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 2);
    let iv = Interval::parse(&a.string(0)?)?;
    let n = convert::to_double(a.req(1)?)?;
    let n = vba_value::Decimal::from_f64(n)?
        .to_i64_round()
        .map_err(|_| E5)?;
    Ok(Variant::Date(date_add(iv, n, date_arg(a, 2)?)?))
}
pub(crate) fn f_date_diff(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 1);
    null_guard!(a, 2);
    let iv = Interval::parse(&a.string(0)?)?;
    let fdow = first_day(a.long_or(3, 1)?, ctx.host.locale())?;
    first_week(a.long_or(4, 1)?, ctx.host.locale())?;
    let r = date_diff(iv, date_arg(a, 1)?, date_arg(a, 2)?, fdow)?;
    Ok(Variant::Long(
        i32::try_from(r).map_err(|_| VbaError::OVERFLOW)?,
    ))
}
pub(crate) fn f_date_part(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 1);
    let iv = Interval::parse(&a.string(0)?)?;
    let fdow = first_day(a.long_or(2, 1)?, ctx.host.locale())?;
    let fwoy = first_week(a.long_or(3, 1)?, ctx.host.locale())?;
    Ok(Variant::Long(
        date_part(iv, date_arg(a, 1)?, fdow, fwoy)? as i32
    ))
}
pub(crate) fn f_date_serial(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let int = |i| -> StdResult<i32> { Ok(i32::from(convert::to_integer(a.req(i)?)?)) };
    Ok(Variant::Date(date_serial(int(0)?, int(1)?, int(2)?)?))
}
pub(crate) fn f_time_serial(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let int = |i| -> StdResult<i32> { Ok(i32::from(convert::to_integer(a.req(i)?)?)) };
    Ok(Variant::Date(time_serial(int(0)?, int(1)?, int(2)?)?))
}
pub(crate) fn f_date_value(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    let d = date_arg(a, 0)?;
    Ok(Variant::Date(Date::from_serial(d.serial().trunc())?))
}
pub(crate) fn f_time_value(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    let d = date_arg(a, 0)?;
    Ok(Variant::Date(Date::from_serial(d.serial().fract().abs())?))
}

macro_rules! part_fn {
    ($name:ident, $field:ident) => {
        pub(crate) fn $name(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
            null_guard!(a, 0);
            Ok(Variant::Integer(date_arg(a, 0)?.parts().$field as i16))
        }
    };
}
part_fn!(f_day, day);
part_fn!(f_month, month);
part_fn!(f_year, year);
part_fn!(f_hour, hour);
part_fn!(f_minute, minute);
part_fn!(f_second, second);

pub(crate) fn f_weekday(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    let fdow = first_day(a.long_or(1, 1)?, ctx.host.locale())?;
    Ok(Variant::Integer(rel_weekday(date_arg(a, 0)?, fdow) as i16))
}
pub(crate) fn f_weekday_name(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let wd = a.long(0)?;
    if !(1..=7).contains(&wd) {
        return Err(E5.into());
    }
    let fdow = first_day(a.long_or(2, 0)?, ctx.host.locale())?;
    let idx = ((i64::from(wd) - 1 + fdow - 1) % 7) as usize;
    let loc = ctx.host.locale();
    let name = if a.bool_or(1, false)? {
        &loc.day_abbreviations[idx]
    } else {
        &loc.day_names[idx]
    };
    Ok(text(name.as_str()))
}
pub(crate) fn f_month_name(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let m = a.long(0)?;
    if !(1..=12).contains(&m) {
        return Err(E5.into());
    }
    let loc = ctx.host.locale();
    let idx = m as usize - 1;
    let name = if a.bool_or(1, false)? {
        &loc.month_abbreviations[idx]
    } else {
        &loc.month_names[idx]
    };
    Ok(text(name.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, dd: u32) -> Date {
        Date::from_ymd_hms(y, m, dd, 0, 0, 0).unwrap()
    }

    #[test]
    fn date_add_documented() {
        // "DateAdd("m", 1, "31-Jan-95")" returns 28-Feb-95.
        assert_eq!(
            date_add(Interval::Month, 1, d(1995, 1, 31)).unwrap(),
            d(1995, 2, 28)
        );
        assert_eq!(
            date_add(Interval::Year, 1, d(2020, 2, 29)).unwrap(),
            d(2021, 2, 28)
        );
        assert_eq!(
            date_add(Interval::Quarter, -1, d(2020, 1, 15)).unwrap(),
            d(2019, 10, 15)
        );
        assert_eq!(
            date_add(Interval::Day, 366, d(2020, 1, 1)).unwrap(),
            d(2021, 1, 1)
        );
        assert_eq!(
            date_add(Interval::Week, 1, d(2020, 1, 1)).unwrap(),
            d(2020, 1, 8)
        );
        let t = Date::from_ymd_hms(2020, 1, 1, 23, 30, 0).unwrap();
        assert_eq!(
            date_add(Interval::Minute, 45, t).unwrap(),
            Date::from_ymd_hms(2020, 1, 2, 0, 15, 0).unwrap()
        );
        assert_eq!(date_add(Interval::Year, 9000, d(2020, 1, 1)), Err(E5));
    }

    #[test]
    fn date_diff_boundaries() {
        let dd = |i, a, b| date_diff(i, a, b, 1).unwrap();
        assert_eq!(dd(Interval::Year, d(2020, 12, 31), d(2021, 1, 1)), 1);
        assert_eq!(dd(Interval::Quarter, d(2020, 3, 31), d(2020, 4, 1)), 1);
        assert_eq!(dd(Interval::Month, d(2020, 1, 31), d(2020, 2, 1)), 1);
        assert_eq!(dd(Interval::Day, d(2020, 1, 1), d(2020, 12, 31)), 365);
        assert_eq!(dd(Interval::Day, d(2021, 1, 1), d(2020, 1, 1)), -366);
        // 2020-01-04 is a Saturday, 2020-01-05 a Sunday: one week boundary.
        assert_eq!(dd(Interval::Week, d(2020, 1, 4), d(2020, 1, 5)), 1);
        assert_eq!(dd(Interval::Weekday, d(2020, 1, 4), d(2020, 1, 5)), 0);
        assert_eq!(dd(Interval::Weekday, d(2020, 1, 6), d(2020, 1, 13)), 1);
        let a = Date::from_ymd_hms(2020, 1, 1, 1, 59, 59).unwrap();
        let b = Date::from_ymd_hms(2020, 1, 1, 2, 0, 0).unwrap();
        assert_eq!(dd(Interval::Hour, a, b), 1);
        assert_eq!(dd(Interval::Second, a, b), 1);
    }

    #[test]
    fn date_part_weeks() {
        let p = |i, x, fd, fw| date_part(i, x, fd, fw).unwrap();
        assert_eq!(p(Interval::Week, d(2021, 1, 1), 1, 1), 1);
        assert_eq!(p(Interval::Week, d(2020, 12, 31), 1, 1), 53);
        // ISO-like: Monday start, first four days; 2021-01-01 is in week 53 of 2020.
        assert_eq!(p(Interval::Week, d(2021, 1, 1), 2, 2), 53);
        assert_eq!(p(Interval::Week, d(2021, 1, 4), 2, 2), 1);
        assert_eq!(p(Interval::Week, d(2020, 12, 31), 2, 2), 53);
        assert_eq!(p(Interval::Week, d(2019, 12, 30), 2, 2), 1);
        assert_eq!(p(Interval::DayOfYear, d(2020, 12, 31), 1, 1), 366);
        assert_eq!(p(Interval::Weekday, d(2020, 1, 5), 1, 1), 1);
        assert_eq!(p(Interval::Weekday, d(2020, 1, 5), 2, 1), 7);
        assert_eq!(p(Interval::Quarter, d(2020, 5, 5), 1, 1), 2);
    }

    #[test]
    fn serials() {
        assert_eq!(date_serial(1999, 12, 31).unwrap(), d(1999, 12, 31));
        assert_eq!(date_serial(29, 1, 1).unwrap(), d(2029, 1, 1));
        assert_eq!(date_serial(30, 1, 1).unwrap(), d(1930, 1, 1));
        assert_eq!(date_serial(2020, 13, 1).unwrap(), d(2021, 1, 1));
        assert_eq!(date_serial(2020, 3, 0).unwrap(), d(2020, 2, 29));
        assert_eq!(date_serial(-1, 1, 1), Err(E5));
    }
}

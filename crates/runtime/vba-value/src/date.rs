//! The VBA `Date` type: an OLE automation date stored as an `f64`
//! (MS-VBAL §2.2).
//!
//! The integer part counts days from the epoch 1899-12-30 (serial 0) in the
//! proleptic Gregorian calendar; the fractional part is the time of day.
//! The valid range is 0100-01-01 00:00:00 (serial -657434) through
//! 9999-12-31 23:59:59 (serial 2958465.99998843).
//!
//! Negative-date quirk: for a negative serial the time of day is the absolute
//! value of the fractional part, so `-1.25` is 1899-12-29 06:00 (not 18:00).
//! This is documented OLE automation behavior.

use std::fmt;
use vba_core::{VbaError, VbaResult};

/// Smallest valid serial (0100-01-01).
pub const MIN_SERIAL: f64 = -657_434.0;
/// Exclusive upper bound of valid serials (10000-01-01).
pub const MAX_SERIAL_EXCLUSIVE: f64 = 2_958_466.0;
/// Days between 1899-12-30 and 1970-01-01.
const EPOCH_OFFSET: i64 = 25_569;
const SECONDS_PER_DAY: i64 = 86_400;

/// A `Date` value (OLE automation date).
#[derive(Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Date(f64);

/// Calendar and clock fields of a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateParts {
    /// Year, 100 to 9999.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u32,
    /// Day of month, 1 to 31.
    pub day: u32,
    /// Hour, 0 to 23.
    pub hour: u32,
    /// Minute, 0 to 59.
    pub minute: u32,
    /// Second, 0 to 59.
    pub second: u32,
}

/// Returns `true` for Gregorian leap years.
pub fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Number of days in the given month of the given year (`month` in 1..=12).
pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
    }
}

/// Days since 1970-01-01 for a civil date (month 1..=12, day 1..=31).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil date for days since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl Date {
    /// Creates a date from a serial; non-finite or out-of-range values are
    /// error 6 (Overflow).
    pub fn from_serial(v: f64) -> VbaResult<Self> {
        if v.is_finite() && (MIN_SERIAL..MAX_SERIAL_EXCLUSIVE).contains(&v) {
            Ok(Self(v))
        } else {
            Err(VbaError::OVERFLOW)
        }
    }

    /// Returns the serial number.
    pub fn serial(self) -> f64 {
        self.0
    }

    /// Builds a date from a valid calendar date and time of day. Invalid
    /// fields (including year outside 100..=9999) are error 5.
    pub fn from_ymd_hms(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> VbaResult<Self> {
        if !(100..=9999).contains(&y)
            || !(1..=12).contains(&mo)
            || d < 1
            || d > days_in_month(i64::from(y), mo)
            || h > 23
            || mi > 59
            || s > 59
        {
            return Err(VbaError::INVALID_PROCEDURE_CALL);
        }
        let days = days_from_civil(i64::from(y), i64::from(mo), i64::from(d)) + EPOCH_OFFSET;
        let secs = i64::from(h) * 3600 + i64::from(mi) * 60 + i64::from(s);
        Ok(Self(compose(days, secs)))
    }

    /// `DateSerial`: months and days outside their normal range roll over
    /// (`DateSerial(2020, 13, 1)` is 2021-01-01). The normalized year must be
    /// 100..=9999, otherwise error 5. Two-digit year mapping is left to the
    /// caller.
    pub fn date_serial(year: i64, month: i64, day: i64) -> VbaResult<Self> {
        let m0 = month - 1;
        let y = year
            .checked_add(m0.div_euclid(12))
            .ok_or(VbaError::INVALID_PROCEDURE_CALL)?;
        let m = m0.rem_euclid(12) + 1;
        if y.abs() > 1_000_000 || day.abs() > 1_000_000_000_000 {
            return Err(VbaError::INVALID_PROCEDURE_CALL);
        }
        let first = days_from_civil(y, m, 1);
        let days = first
            .checked_add(day - 1)
            .ok_or(VbaError::INVALID_PROCEDURE_CALL)?
            + EPOCH_OFFSET;
        let r = Self::from_serial(days as f64).map_err(|_| VbaError::INVALID_PROCEDURE_CALL)?;
        Ok(r)
    }

    /// `TimeSerial` as a fraction of a day, wrapping into `[0, 1)`.
    pub fn time_serial(hour: i64, minute: i64, second: i64) -> VbaResult<f64> {
        let total = hour
            .checked_mul(3600)
            .and_then(|h| minute.checked_mul(60).and_then(|m| h.checked_add(m)))
            .and_then(|hm| hm.checked_add(second))
            .ok_or(VbaError::INVALID_PROCEDURE_CALL)?;
        Ok(total.rem_euclid(SECONDS_PER_DAY) as f64 / SECONDS_PER_DAY as f64)
    }

    /// Splits into calendar and clock fields. The time of day is rounded to
    /// the nearest second; the negative-date quirk is honored.
    pub fn parts(self) -> DateParts {
        let v = self.0;
        let negative = v < 0.0;
        let mut days = v.trunc() as i64;
        let frac = (v - v.trunc()).abs();
        let mut secs = (frac * SECONDS_PER_DAY as f64).round() as i64;
        if secs >= SECONDS_PER_DAY {
            secs -= SECONDS_PER_DAY;
            days += if negative { -1 } else { 1 };
        }
        let (y, m, d) = civil_from_days(days - EPOCH_OFFSET);
        DateParts {
            year: y as i32,
            month: m,
            day: d,
            hour: (secs / 3600) as u32,
            minute: (secs % 3600 / 60) as u32,
            second: (secs % 60) as u32,
        }
    }

    /// `Weekday` with `vbSunday` (1) as the first day: 1 = Sunday .. 7 =
    /// Saturday.
    pub fn weekday(self) -> u32 {
        let days = self.0.trunc() as i64;
        ((days.rem_euclid(7) + 6) % 7 + 1) as u32
    }
}

/// Combines a day number and seconds-of-day into a serial, honoring the
/// negative-date quirk.
fn compose(days: i64, secs: i64) -> f64 {
    let frac = secs as f64 / SECONDS_PER_DAY as f64;
    if days < 0 {
        days as f64 - frac
    } else {
        days as f64 + frac
    }
}

impl fmt::Display for Date {
    /// `CStr` of a date in the en-US format: `m/d/yyyy`, `h:mm:ss AM/PM`, or
    /// both separated by a space. Other locales are not reproduced.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.parts();
        let has_date = self.0.trunc() != 0.0;
        let has_time = p.hour != 0 || p.minute != 0 || p.second != 0;
        let date = format!("{}/{}/{}", p.month, p.day, p.year);
        let (h12, ampm) = match p.hour {
            0 => (12, "AM"),
            1..=11 => (p.hour, "AM"),
            12 => (12, "PM"),
            h => (h - 12, "PM"),
        };
        let time = format!("{}:{:02}:{:02} {}", h12, p.minute, p.second, ampm);
        match (has_date, has_time) {
            (true, true) => write!(f, "{date} {time}"),
            (true, false) => f.write_str(&date),
            (false, true) => f.write_str(&time),
            // CStr(CDate(0)) is "12:00:00 AM".
            (false, false) => f.write_str("12:00:00 AM"),
        }
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Date({} = {self})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(v: f64) -> (i32, u32, u32, u32, u32, u32) {
        let p = Date::from_serial(v).unwrap().parts();
        (p.year, p.month, p.day, p.hour, p.minute, p.second)
    }

    #[test]
    fn epoch_and_known_serials() {
        assert_eq!(parts(0.0), (1899, 12, 30, 0, 0, 0));
        assert_eq!(parts(1.0), (1899, 12, 31, 0, 0, 0));
        assert_eq!(parts(2.0), (1900, 1, 1, 0, 0, 0));
        assert_eq!(parts(36526.0), (2000, 1, 1, 0, 0, 0));
        assert_eq!(parts(43831.5), (2020, 1, 1, 12, 0, 0));
        assert_eq!(parts(2_958_465.0), (9999, 12, 31, 0, 0, 0));
        assert_eq!(parts(-657_434.0), (100, 1, 1, 0, 0, 0));
        assert_eq!(parts(-1.0), (1899, 12, 29, 0, 0, 0));
    }

    #[test]
    fn negative_date_quirk() {
        assert_eq!(parts(-1.25), (1899, 12, 29, 6, 0, 0));
        assert_eq!(parts(-0.5), (1899, 12, 30, 12, 0, 0));
        let d = Date::from_ymd_hms(1899, 12, 29, 6, 0, 0).unwrap();
        assert_eq!(d.serial(), -1.25);
    }

    #[test]
    fn range() {
        assert!(Date::from_serial(-657_434.0).is_ok());
        assert_eq!(Date::from_serial(-657_435.0), Err(VbaError::OVERFLOW));
        assert!(Date::from_serial(2_958_465.999_988_4).is_ok());
        assert_eq!(Date::from_serial(2_958_466.0), Err(VbaError::OVERFLOW));
        assert_eq!(Date::from_serial(f64::NAN), Err(VbaError::OVERFLOW));
        assert_eq!(Date::from_serial(f64::INFINITY), Err(VbaError::OVERFLOW));
    }

    #[test]
    fn from_ymd_validation() {
        assert_eq!(
            Date::from_ymd_hms(2020, 2, 29, 0, 0, 0).unwrap().serial(),
            43890.0
        );
        for (y, m, d) in [
            (2019, 2, 29),
            (2020, 13, 1),
            (2020, 0, 1),
            (99, 1, 1),
            (10000, 1, 1),
            (2020, 4, 31),
        ] {
            assert_eq!(
                Date::from_ymd_hms(y, m, d, 0, 0, 0),
                Err(VbaError::INVALID_PROCEDURE_CALL),
                "{y}-{m}-{d}"
            );
        }
        assert!(Date::from_ymd_hms(2020, 1, 1, 24, 0, 0).is_err());
        assert!(Date::from_ymd_hms(1900, 2, 29, 0, 0, 0).is_err());
    }

    #[test]
    fn date_serial_rolls_over() {
        let s = |y, m, d| Date::date_serial(y, m, d).map(|x| x.serial());
        assert_eq!(s(2020, 13, 1), s(2021, 1, 1));
        assert_eq!(s(2020, 0, 1), s(2019, 12, 1));
        assert_eq!(s(2020, 3, 0), s(2020, 2, 29));
        assert_eq!(s(2020, 1, 32), s(2020, 2, 1));
        assert_eq!(s(2020, 1, 1), Ok(43831.0));
        assert!(s(99, 1, 1).is_err());
        assert!(s(10000, 1, 1).is_err());
        assert!(s(9999, 12, 31).is_ok());
        assert!(s(9999, 12, 32).is_err());
    }

    #[test]
    fn time_serial_wraps() {
        assert_eq!(Date::time_serial(12, 0, 0).unwrap(), 0.5);
        assert_eq!(Date::time_serial(25, 0, 0).unwrap(), 1.0 / 24.0);
        assert_eq!(Date::time_serial(-1, 0, 0).unwrap(), 23.0 / 24.0);
        assert_eq!(Date::time_serial(0, 90, 0).unwrap(), 1.5 / 24.0);
    }

    #[test]
    fn weekday() {
        // 1899-12-30 was a Saturday; 2000-01-01 a Saturday; 2020-01-01 a Wednesday.
        assert_eq!(Date::from_serial(0.0).unwrap().weekday(), 7);
        assert_eq!(Date::from_serial(36526.0).unwrap().weekday(), 7);
        assert_eq!(Date::from_serial(43831.0).unwrap().weekday(), 4);
        assert_eq!(Date::from_serial(1.0).unwrap().weekday(), 1);
        assert_eq!(Date::from_serial(-1.0).unwrap().weekday(), 6);
    }

    #[test]
    fn seconds_round_and_carry() {
        // 23:59:59.9 rounds up into the next day.
        let v = 43831.0 + (86_399.9 / 86_400.0);
        assert_eq!(parts(v), (2020, 1, 2, 0, 0, 0));
        let neg = -2.0 - (86_399.9 / 86_400.0);
        assert_eq!(parts(neg), (1899, 12, 27, 0, 0, 0));
    }

    #[test]
    fn display_en_us() {
        let d = |v| Date::from_serial(v).unwrap().to_string();
        assert_eq!(d(43831.0), "1/1/2020");
        assert_eq!(d(43831.5), "1/1/2020 12:00:00 PM");
        assert_eq!(d(0.25), "6:00:00 AM");
        assert_eq!(d(0.0), "12:00:00 AM");
        assert_eq!(
            d(43831.0 + 13.0 / 24.0 + 5.0 / 1440.0 + 9.0 / 86400.0),
            "1/1/2020 1:05:09 PM"
        );
        assert_eq!(d(43831.0 + 1.0 / 86400.0), "1/1/2020 12:00:01 AM");
    }

    #[test]
    fn roundtrip_every_day_in_sample() {
        for days in (-657_434..2_958_466).step_by(997) {
            let d = Date::from_serial(days as f64).unwrap();
            let p = d.parts();
            let back = Date::from_ymd_hms(p.year, p.month, p.day, 0, 0, 0).unwrap();
            assert_eq!(back.serial(), days as f64);
        }
    }
}

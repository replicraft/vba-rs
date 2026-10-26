//! Date literals (`#...#`), MS-VBAL §3.3.3.
//!
//! This is the compile-time reading of a `DATE` token: it follows the
//! grammar and static semantics of §3.3.3 and is independent of the run-time
//! string conversion of [`crate::text::parse_date`]. An invalid literal is a
//! static error, reported by returning `None`.

use crate::date::{Date, days_in_month};
use crate::string::VbaString;

/// The year (`CY` in §3.3.3) used to *validate* a date literal that names no
/// year. A leap year, so that `#2/29#` is accepted. Real VBA gives such a
/// literal the current year; use [`parse_yearless`] and
/// [`YearlessDate::resolve`] to get the date for a given year.
pub const DEFAULT_LITERAL_YEAR: i32 = 2000;

const FULL_MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const ABBREVIATED_MONTHS: [&str; 11] = [
    "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

fn month_name(word: &str) -> Option<u32> {
    if let Some(i) = FULL_MONTHS.iter().position(|m| *m == word) {
        return Some(i as u32 + 1);
    }
    let i = ABBREVIATED_MONTHS.iter().position(|m| *m == word)?;
    // "may" has no separate abbreviation, so the index skips it from "jun".
    Some(if i >= 4 { i as u32 + 2 } else { i as u32 + 1 })
}

fn is_wsc(c: char) -> bool {
    c == '\t'
        || c == '\u{19}'
        || c == ' '
        || c == '\u{3000}'
        || (c.is_whitespace() && c != '\n' && c != '\r')
}

/// Two-digit years are expanded with the Windows default window (00-29 is
/// 2000-2029, 30-99 is 1930-1999); the specification leaves `Year()` to the
/// implementation.
fn year_of(n: u32, digits: usize) -> i32 {
    if digits <= 2 {
        if n <= 29 {
            2000 + n as i32
        } else {
            1900 + n as i32
        }
    } else {
        n as i32
    }
}

fn legal_month(m: u32) -> bool {
    (1..=12).contains(&m)
}

fn legal_day(m: u32, d: u32, y: i32) -> bool {
    legal_month(m) && d >= 1 && d <= days_in_month(i64::from(y), m)
}

#[derive(Clone, Copy)]
enum Item {
    Num(u32, usize),
    Month(u32),
}

/// Splits a date-value into 2 or 3 items.
fn date_items(s: &str) -> Option<Vec<Item>> {
    let chars: Vec<char> = s.chars().collect();
    let mut items = Vec::new();
    let mut i = 0;
    let mut expect_item = true;
    while i < chars.len() {
        if expect_item {
            let start = i;
            if chars[i].is_ascii_digit() {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                if text.len() > 9 {
                    return None;
                }
                items.push(Item::Num(text.parse().ok()?, text.len()));
            } else if chars[i].is_ascii_alphabetic() {
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let text: String = chars[start..i]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase();
                items.push(Item::Month(month_name(&text)?));
            } else {
                return None;
            }
            expect_item = false;
        } else {
            // date-separator = 1*WSC / (*WSC ("/" / "-" / ",") *WSC)
            let start = i;
            let mut punct = 0;
            while i < chars.len() && (is_wsc(chars[i]) || matches!(chars[i], '/' | '-' | ',')) {
                if matches!(chars[i], '/' | '-' | ',') {
                    punct += 1;
                }
                i += 1;
            }
            if i == start || punct > 1 {
                return None;
            }
            expect_item = true;
        }
    }
    if expect_item || !(2..=3).contains(&items.len()) {
        return None;
    }
    let months = items.iter().filter(|i| matches!(i, Item::Month(_))).count();
    if months > 1 {
        return None;
    }
    Some(items)
}

/// Interprets a date-value; `(year, month, day, yearless)`. `yearless` is set
/// when the text names no year and `DEFAULT_LITERAL_YEAR` stands in for it.
fn date_value(s: &str) -> Option<(i32, u32, u32, bool)> {
    let items = date_items(s)?;
    let cy = DEFAULT_LITERAL_YEAR;
    let r = date_value_items(&items, cy)?;
    let yearless = match items.as_slice() {
        [Item::Num(l, _), Item::Num(m, _)] => {
            (legal_month(*l) && legal_day(*l, *m, cy)) || (legal_month(*m) && legal_day(*m, *l, cy))
        }
        [Item::Month(month), Item::Num(n, _)] | [Item::Num(n, _), Item::Month(month)] => {
            legal_day(*month, *n, cy)
        }
        _ => false,
    };
    Some((r.0, r.1, r.2, yearless))
}

fn date_value_items(items: &[Item], cy: i32) -> Option<(i32, u32, u32)> {
    match items {
        [Item::Num(l, _), Item::Num(m, md)] => {
            let (l, m, md) = (*l, *m, *md);
            if legal_month(l) && legal_day(l, m, cy) {
                Some((cy, l, m))
            } else if legal_month(m) && legal_day(m, l, cy) {
                Some((cy, m, l))
            } else if legal_month(l) {
                Some((year_of(m, md), l, 1))
            } else if legal_month(m) {
                let ld = if let Item::Num(_, d) = items[0] { d } else { 0 };
                Some((year_of(l, ld), m, 1))
            } else {
                None
            }
        }
        [Item::Num(l, ld), Item::Num(m, _), Item::Num(r, rd)] => {
            let (l, m, r) = (*l, *m, *r);
            if legal_month(l) && legal_day(l, m, year_of(r, *rd)) {
                Some((year_of(r, *rd), l, m))
            } else if legal_month(m) && legal_day(m, r, year_of(l, *ld)) {
                Some((year_of(l, *ld), m, r))
            } else if legal_month(m) && legal_day(m, l, year_of(r, *rd)) {
                Some((year_of(r, *rd), m, l))
            } else {
                None
            }
        }
        [a, b] => {
            let (month, n, nd) = match (a, b) {
                (Item::Month(m), Item::Num(n, d)) | (Item::Num(n, d), Item::Month(m)) => {
                    (*m, *n, *d)
                }
                _ => return None,
            };
            if legal_day(month, n, cy) {
                Some((cy, month, n))
            } else {
                Some((year_of(n, nd), month, 1))
            }
        }
        [a, b, c] => {
            let mut month = 0;
            let mut nums = Vec::new();
            for it in [a, b, c] {
                match it {
                    Item::Month(m) => month = *m,
                    Item::Num(n, d) => nums.push((*n, *d)),
                }
            }
            if nums.len() != 2 {
                return None;
            }
            let ((n1, d1), (n2, d2)) = (nums[0], nums[1]);
            if legal_day(month, n1, year_of(n2, d2)) {
                Some((year_of(n2, d2), month, n1))
            } else if legal_day(month, n2, year_of(n1, d1)) {
                Some((year_of(n1, d1), month, n2))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Interprets a time-value; seconds since midnight.
fn time_value(s: &str) -> Option<u32> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let number = |i: &mut usize| -> Option<u32> {
        let start = *i;
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            *i += 1;
        }
        if *i == start || *i - start > 9 {
            return None;
        }
        chars[start..*i].iter().collect::<String>().parse().ok()
    };
    let skip_ws = |i: &mut usize| {
        while *i < chars.len() && is_wsc(chars[*i]) {
            *i += 1;
        }
    };
    let hour = number(&mut i)?;
    let mut minute = 0;
    let mut second = 0;
    let mut have_minute = false;
    // time-separator = *WSC (":" / ".") *WSC
    let mut j = i;
    skip_ws(&mut j);
    if j < chars.len() && (chars[j] == ':' || chars[j] == '.') {
        j += 1;
        skip_ws(&mut j);
        i = j;
        minute = number(&mut i)?;
        have_minute = true;
        let mut k = i;
        skip_ws(&mut k);
        if k < chars.len() && (chars[k] == ':' || chars[k] == '.') {
            k += 1;
            skip_ws(&mut k);
            i = k;
            second = number(&mut i)?;
        }
    }
    // ampm = *WSC ("am" / "pm" / "a" / "p")
    let mut k = i;
    skip_ws(&mut k);
    let rest: String = chars[k..].iter().collect::<String>().to_ascii_lowercase();
    let ampm = match rest.as_str() {
        "" => None,
        "am" | "a" => Some(false),
        "pm" | "p" => Some(true),
        _ => return None,
    };
    if ampm.is_none() && !have_minute {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let hour = match ampm {
        Some(true) if hour <= 11 => hour + 12,
        Some(false) if hour == 12 => 0,
        _ => hour,
    };
    Some(hour * 3600 + minute * 60 + second)
}

/// A parsed date-or-time: the date part (if any, with whether it lacked a
/// year) and the seconds after midnight.
struct Parsed {
    date: Option<(i32, u32, u32, bool)>,
    secs: u32,
}

fn parse_core(s: &VbaString) -> Option<Parsed> {
    let text = s.to_string_lossy();
    let t = text.trim_matches(is_wsc);
    if t.is_empty() {
        return None;
    }
    let (date, secs) = if let Some(d) = date_value(t) {
        (Some(d), 0)
    } else if let Some(secs) = time_value(t) {
        (None, secs)
    } else {
        // date-value 1*WSC time-value
        let mut found = None;
        for (idx, c) in t.char_indices() {
            if is_wsc(c) {
                let (left, right) = (&t[..idx], &t[idx + c.len_utf8()..]);
                if let (Some(d), Some(secs)) = (
                    date_value(left.trim_end_matches(is_wsc)),
                    time_value(right.trim_start_matches(is_wsc)),
                ) {
                    found = Some((Some(d), secs));
                    break;
                }
            }
        }
        found?
    };
    Some(Parsed { date, secs })
}

fn assemble(date: Option<(i32, u32, u32)>, secs: u32) -> Option<Date> {
    let (y, m, d) = date.unwrap_or((1899, 12, 30));
    let base = if date.is_none() {
        Date::from_serial(0.0).ok()?
    } else {
        Date::from_ymd_hms(y, m, d, 0, 0, 0).ok()?
    };
    let serial = base.serial();
    let frac = f64::from(secs) / 86_400.0;
    Date::from_serial(if serial < 0.0 {
        serial - frac
    } else {
        serial + frac
    })
    .ok()
}

/// Parses the text between the `#` delimiters of a date literal.
///
/// Returns `None` when the literal is not a valid `date-or-time`. A literal
/// without a year is read in [`DEFAULT_LITERAL_YEAR`]; use
/// [`parse_yearless`] to tell such literals apart.
pub fn parse_date_literal(s: &VbaString) -> Option<Date> {
    let p = parse_core(s)?;
    assemble(p.date.map(|(y, m, d, _)| (y, m, d)), p.secs)
}

/// A date literal that names no year (`#3/4#`, `#Mar 4 10:00#`): real VBA
/// gives it the year in which the code runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearlessDate {
    /// Month, 1 to 12.
    pub month: u32,
    /// Day of the month.
    pub day: u32,
    /// Seconds after midnight.
    pub secs: u32,
}

impl YearlessDate {
    /// The date in `year`; `None` when the day does not exist in that year
    /// (`#2/29#` in a year that is not a leap year).
    pub fn resolve(self, year: i32) -> Option<Date> {
        if self.day > days_in_month(i64::from(year), self.month) {
            return None;
        }
        assemble(Some((year, self.month, self.day)), self.secs)
    }
}

/// `Some` when `s` is a valid date literal without a year. The day is
/// validated against a leap year, so `#2/29#` is accepted here.
pub fn parse_yearless(s: &VbaString) -> Option<YearlessDate> {
    let p = parse_core(s)?;
    let (_, month, day, yearless) = p.date?;
    yearless.then_some(YearlessDate {
        month,
        day,
        secs: p.secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(s: &str) -> Option<f64> {
        parse_date_literal(&VbaString::from(s)).map(Date::serial)
    }

    #[test]
    fn forms() {
        assert_eq!(lit("1/2/2003"), lit("2003-1-2"));
        assert_eq!(lit("13/2/2003"), lit("2/13/2003"));
        assert_eq!(lit("Mar 2003"), lit("3/1/2003"));
        assert_eq!(lit("12/2003"), lit("12/1/2003"));
        assert_eq!(lit("3 p"), Some(0.625));
        assert_eq!(lit("13:30 PM"), Some(13.5 / 24.0));
        assert_eq!(lit("12 a"), Some(0.0));
        assert_eq!(lit("0:30 pm"), Some(12.5 / 24.0));
        assert_eq!(lit("1.02.03"), Some(3723.0 / 86400.0));
        assert!(lit("1/2/2003 3:04 PM").is_some());
    }

    fn yl(s: &str) -> Option<YearlessDate> {
        parse_yearless(&VbaString::from(s))
    }

    #[test]
    fn yearless_forms() {
        let md = |month, day, secs| Some(YearlessDate { month, day, secs });
        assert_eq!(yl("3/4"), md(3, 4, 0));
        assert_eq!(yl("13/4"), md(4, 13, 0));
        assert_eq!(yl("Mar 4"), md(3, 4, 0));
        assert_eq!(yl("4 March"), md(3, 4, 0));
        assert_eq!(yl("3/4 10:30"), md(3, 4, 37_800));
        for s in ["3/4/2020", "Mar 2020", "12/2003", "10:30", "3 p", "x"] {
            assert_eq!(yl(s), None, "{s}");
        }
    }

    #[test]
    fn yearless_resolves_in_a_year() {
        let d = yl("3/4 1:00").unwrap();
        assert_eq!(
            d.resolve(2025),
            Date::from_ymd_hms(2025, 3, 4, 1, 0, 0).ok()
        );
        let leap = yl("2/29").unwrap();
        assert!(leap.resolve(2024).is_some());
        assert_eq!(leap.resolve(2025), None);
    }

    #[test]
    fn invalid() {
        for s in [
            "2/30/2003",
            "13/13/2003",
            "24:00",
            "1:60",
            "1:00:60",
            "Jan Feb 2003",
            "x",
            "",
        ] {
            assert_eq!(lit(s), None, "{s}");
        }
    }
}

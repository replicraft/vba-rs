//! Host services for everything that is not pure: clock, environment and
//! locale. The library never reads the operating system on its own, so every
//! call is deterministic given a [`Host`].

use std::collections::BTreeMap;
use vba_value::Date;

/// Order of the day, month and year in short dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateOrder {
    /// Month, day, year (en-US).
    Mdy,
    /// Day, month, year.
    Dmy,
    /// Year, month, day.
    Ymd,
}

/// Locale data used by `Format`, `FormatNumber`, `MonthName`, `Weekday`
/// defaults and friends. [`Locale::en_us`] is the default; other locales are
/// built by filling the same fields.
///
/// The named date formats are format strings in the `Format` date language
/// (for example `"dddd, mmmm d, yyyy"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locale {
    /// Decimal separator.
    pub decimal: char,
    /// Thousands (digit grouping) separator.
    pub thousands: char,
    /// List separator.
    pub list_separator: char,
    /// Currency symbol.
    pub currency_symbol: String,
    /// `true` if the currency symbol precedes the number.
    pub currency_prefix: bool,
    /// Digits after the decimal point for currency.
    pub currency_digits: u8,
    /// `true` if negative currency is shown in parentheses.
    pub currency_negative_parens: bool,
    /// Date separator.
    pub date_separator: char,
    /// Time separator.
    pub time_separator: char,
    /// Order of date parts in short dates.
    pub date_order: DateOrder,
    /// AM designator.
    pub am: String,
    /// PM designator.
    pub pm: String,
    /// Full month names, January first.
    pub month_names: [String; 12],
    /// Abbreviated month names, January first.
    pub month_abbreviations: [String; 12],
    /// Full day names, Sunday first.
    pub day_names: [String; 7],
    /// Abbreviated day names, Sunday first.
    pub day_abbreviations: [String; 7],
    /// First day of the week (1 = Sunday .. 7 = Saturday) used for
    /// `vbUseSystemDayOfWeek`.
    pub first_day_of_week: u32,
    /// First week of the year rule (1..=3) used for `vbUseSystem`.
    pub first_week_of_year: u32,
    /// `Format` pattern of `"Short Date"`.
    pub short_date: String,
    /// `Format` pattern of `"Medium Date"`.
    pub medium_date: String,
    /// `Format` pattern of `"Long Date"`.
    pub long_date: String,
    /// `Format` pattern of `"Short Time"`.
    pub short_time: String,
    /// `Format` pattern of `"Medium Time"`.
    pub medium_time: String,
    /// `Format` pattern of `"Long Time"`.
    pub long_time: String,
    /// Windows ANSI code page used by `Asc`, `Chr` and `StrConv`
    /// (`vbUnicode`/`vbFromUnicode`); 1252 for en-US.
    pub ansi_code_page: u32,
}

impl Locale {
    /// English (United States), the default locale.
    pub fn en_us() -> Self {
        let s = |a: [&str; 12]| a.map(String::from);
        let d = |a: [&str; 7]| a.map(String::from);
        Locale {
            decimal: '.',
            thousands: ',',
            list_separator: ',',
            currency_symbol: "$".into(),
            currency_prefix: true,
            currency_digits: 2,
            currency_negative_parens: true,
            date_separator: '/',
            time_separator: ':',
            date_order: DateOrder::Mdy,
            am: "AM".into(),
            pm: "PM".into(),
            month_names: s([
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]),
            month_abbreviations: s([
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]),
            day_names: d([
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ]),
            day_abbreviations: d(["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]),
            first_day_of_week: 1,
            first_week_of_year: 1,
            short_date: "m/d/yyyy".into(),
            medium_date: "dd-mmm-yy".into(),
            long_date: "dddd, mmmm d, yyyy".into(),
            short_time: "hh:mm".into(),
            medium_time: "hh:mm AM/PM".into(),
            long_time: "h:mm:ss AM/PM".into(),
            ansi_code_page: 1252,
        }
    }
}

impl Default for Locale {
    fn default() -> Self {
        Locale::en_us()
    }
}

/// Source of the current local date and time.
pub trait Clock {
    /// The current local date and time as a `Date`.
    fn now(&self) -> Date;

    /// Seconds since midnight (`Timer`), with fractional seconds.
    fn timer(&self) -> f32 {
        let v = self.now().serial();
        ((v - v.trunc()).abs() * 86_400.0) as f32
    }
}

/// Source of environment variables (`Environ`).
pub trait Environment {
    /// Value of the variable `name` (case-insensitive on Windows hosts), or
    /// `None` if it is not set.
    fn environ(&self, name: &str) -> Option<String>;

    /// The `index`-th (1-based) entry as `NAME=value`, or `None` past the end.
    fn environ_index(&self, index: usize) -> Option<String>;
}

/// Everything a standard-library call may need from the outside world.
pub trait Host: Clock + Environment {
    /// The active locale.
    fn locale(&self) -> &Locale;

    /// The text collation used by `Option Compare Text` / `vbTextCompare`.
    /// The default is the documented en-US approximation
    /// ([`vba_value::collation::EnUs`]); a host may supply its own.
    fn collation(&self) -> &dyn vba_value::collation::Collation {
        &vba_value::collation::EN_US
    }
}

/// A fully deterministic host: fixed clock, fixed environment, en-US locale.
/// It is the default host for tests and for embedders that want no ambient
/// dependence on the operating system.
#[derive(Debug, Clone)]
pub struct FixedHost {
    /// The value returned by `Now`.
    pub now: Date,
    /// Environment variables, in `Environ(index)` order (sorted by name).
    pub env: BTreeMap<String, String>,
    /// The locale.
    pub locale: Locale,
}

impl FixedHost {
    /// A host whose clock is `now`, with an empty environment and en-US
    /// locale.
    pub fn new(now: Date) -> Self {
        FixedHost {
            now,
            env: BTreeMap::new(),
            locale: Locale::en_us(),
        }
    }
}

impl Default for FixedHost {
    /// 2000-01-01 00:00:00.
    fn default() -> Self {
        FixedHost::new(Date::from_ymd_hms(2000, 1, 1, 0, 0, 0).expect("valid date"))
    }
}

impl Clock for FixedHost {
    fn now(&self) -> Date {
        self.now
    }
}

impl Environment for FixedHost {
    fn environ(&self, name: &str) -> Option<String> {
        self.env
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    }

    fn environ_index(&self, index: usize) -> Option<String> {
        let (k, v) = self.env.iter().nth(index.checked_sub(1)?)?;
        Some(format!("{k}={v}"))
    }
}

impl Host for FixedHost {
    fn locale(&self) -> &Locale {
        &self.locale
    }
}

//! The function registry: case-insensitive lookup by simple name
//! (`Left`, `left$`) or qualified name (`VBA.Strings.Left`, `Strings.Left`,
//! `VBA.Left`), signature metadata, and a uniform call entry point.
//!
//! Version gating through `vba-profile` is not used: every function here
//! exists in all supported VBA versions except `CLngLng`/`CLngPtr`, which the
//! VM's profile layer is expected to hide for VBA6 programs.

use crate::call::{Args, Ctx};
use crate::error::{StdError, StdResult, WRONG_ARGUMENT_COUNT};
use crate::{
    conversion as cv, datetime as dt, financial as fin, format as fm, info, interaction as ia,
    math, misc, strings as st,
};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::OnceLock;
use vba_value::{VarType, Variant};

/// Signature of an intrinsic implementation.
pub type Imp = fn(&mut Ctx<'_>, &Args<'_>) -> StdResult<Variant>;

/// One parameter of an intrinsic.
#[derive(Debug, Clone, Copy)]
pub struct ParamInfo {
    /// Parameter name as documented.
    pub name: &'static str,
    /// Declared type (`Variant` when the parameter is a Variant).
    pub ty: VarType,
    /// `true` for `Optional`.
    pub optional: bool,
    /// Documented default of an optional parameter, as VBA source text.
    pub default: Option<&'static str>,
    /// `true` for `ByRef` (no intrinsic here modifies its arguments; all are
    /// `ByVal`).
    pub by_ref: bool,
    /// `true` for a trailing `ParamArray`.
    pub param_array: bool,
}

/// How the VM must treat the function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Evaluate all arguments, then call [`FunctionInfo::invoke`]. This is
    /// also the case for `IIf`, `Choose` and `Switch`, which VBA evaluates
    /// eagerly.
    Eager,
    /// The VM implements it (it needs state the library cannot see);
    /// `invoke` returns [`StdError::VmRequired`].
    Vm,
    /// A `Print` list helper (`Tab`, `Spc`): see
    /// [`PrintHelper`](crate::interaction::PrintHelper).
    PrintHelper,
}

/// Metadata and implementation of one intrinsic.
#[derive(Clone, Copy)]
pub struct FunctionInfo {
    /// Simple name without `$`.
    pub name: &'static str,
    /// Defining module, e.g. `"Strings"` (library `VBA`).
    pub module: &'static str,
    /// Parameters in order.
    pub params: &'static [ParamInfo],
    /// Return type; `Variant` when it depends on the arguments.
    pub returns: VarType,
    /// `true` if the `$` form (`Left$`) exists: it returns a plain `String`
    /// and raises error 94 for `Null`.
    pub dollar: bool,
    /// Evaluation kind.
    pub kind: Kind,
    /// The implementation.
    pub imp: Imp,
}

impl std::fmt::Debug for FunctionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionInfo")
            .field("name", &self.name)
            .field("module", &self.module)
            .finish()
    }
}

/// Functions whose cost grows with the length of their text arguments
/// (searching, comparing, converting, formatting); they pay for the scan
/// before it starts. Functions that only copy are paid for by the volume of
/// what they build.
fn scans_input(name: &str) -> bool {
    matches!(
        name,
        "InStr"
            | "InStrB"
            | "InStrRev"
            | "Replace"
            | "Split"
            | "Join"
            | "Filter"
            | "StrComp"
            | "StrConv"
            | "Format"
            | "FormatNumber"
            | "FormatCurrency"
            | "FormatPercent"
            | "FormatDateTime"
    )
}

impl FunctionInfo {
    /// Number of arguments that must be present.
    pub fn min_args(&self) -> usize {
        self.params
            .iter()
            .filter(|p| !p.optional && !p.param_array)
            .count()
    }

    /// Maximum number of arguments, or `None` with a `ParamArray`.
    pub fn max_args(&self) -> Option<usize> {
        if self.params.iter().any(|p| p.param_array) {
            None
        } else {
            Some(self.params.len())
        }
    }

    /// Calls the function. `dollar` selects the `$` form. Wrong argument
    /// counts are error 450; omitted required arguments error 449.
    pub fn invoke(
        &self,
        dollar: bool,
        ctx: &mut Ctx<'_>,
        args: &[Option<Variant>],
    ) -> StdResult<Variant> {
        if args.len() < self.min_args() || self.max_args().is_some_and(|m| args.len() > m) {
            return Err(WRONG_ARGUMENT_COUNT.into());
        }
        if self.kind == Kind::Vm {
            return Err(StdError::VmRequired(self.name));
        }
        if scans_input(self.name) {
            ctx.charge(crate::call::scan_units(args))?;
        }
        (self.imp)(
            ctx,
            &Args {
                items: args,
                dollar: dollar && self.dollar,
            },
        )
    }
}

/// A resolved name.
#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    /// The function.
    pub info: &'static FunctionInfo,
    /// `true` if the name used the `$` suffix.
    pub dollar: bool,
}

const fn p(name: &'static str, ty: VarType) -> ParamInfo {
    ParamInfo {
        name,
        ty,
        optional: false,
        default: None,
        by_ref: false,
        param_array: false,
    }
}
const fn o(name: &'static str, ty: VarType, default: Option<&'static str>) -> ParamInfo {
    ParamInfo {
        name,
        ty,
        optional: true,
        default,
        by_ref: false,
        param_array: false,
    }
}
const fn pa(name: &'static str) -> ParamInfo {
    ParamInfo {
        name,
        ty: VarType::Variant,
        optional: true,
        default: None,
        by_ref: false,
        param_array: true,
    }
}

use VarType as T;

const CMP: ParamInfo = o("Compare", T::Long, Some("vbBinaryCompare"));
const ONE_STR: &[ParamInfo] = &[p("String", T::String)];
const ONE_VAR: &[ParamInfo] = &[p("Number", T::Variant)];
const ONE_DBL: &[ParamInfo] = &[p("Number", T::Double)];
const ONE_DATE: &[ParamInfo] = &[p("Date", T::Variant)];

macro_rules! f {
    ($name:literal, $module:literal, $params:expr, $ret:expr, $dollar:expr, $imp:expr) => {
        FunctionInfo {
            name: $name,
            module: $module,
            params: $params,
            returns: $ret,
            dollar: $dollar,
            kind: Kind::Eager,
            imp: $imp,
        }
    };
    ($name:literal, $module:literal, $params:expr, $ret:expr, $dollar:expr, $imp:expr, $kind:expr) => {
        FunctionInfo {
            name: $name,
            module: $module,
            params: $params,
            returns: $ret,
            dollar: $dollar,
            kind: $kind,
            imp: $imp,
        }
    };
}

static FUNCTIONS: &[FunctionInfo] = &[
    // VBA.Strings
    f!("Asc", "Strings", ONE_STR, T::Integer, false, st::f_asc),
    f!("AscB", "Strings", ONE_STR, T::Integer, false, st::f_asc_b),
    f!("AscW", "Strings", ONE_STR, T::Integer, false, st::f_asc_w),
    f!(
        "Chr",
        "Strings",
        &[p("CharCode", T::Long)],
        T::String,
        true,
        st::f_chr
    ),
    f!(
        "ChrB",
        "Strings",
        &[p("CharCode", T::Long)],
        T::String,
        true,
        st::f_chr_b
    ),
    f!(
        "ChrW",
        "Strings",
        &[p("CharCode", T::Long)],
        T::String,
        true,
        st::f_chr_w
    ),
    f!(
        "Filter",
        "Strings",
        &[
            p("SourceArray", T::Variant),
            p("Match", T::String),
            o("Include", T::Boolean, Some("True")),
            CMP
        ],
        T::Variant,
        false,
        st::f_filter
    ),
    f!(
        "Format",
        "Strings",
        &[
            p("Expression", T::Variant),
            o("Format", T::Variant, None),
            o("FirstDayOfWeek", T::Long, Some("vbSunday")),
            o("FirstWeekOfYear", T::Long, Some("vbFirstJan1"))
        ],
        T::Variant,
        true,
        fm::f_format
    ),
    f!(
        "FormatCurrency",
        "Strings",
        &[
            p("Expression", T::Variant),
            o("NumDigitsAfterDecimal", T::Long, Some("-1")),
            o("IncludeLeadingDigit", T::Long, Some("vbUseDefault")),
            o("UseParensForNegativeNumbers", T::Long, Some("vbUseDefault")),
            o("GroupDigits", T::Long, Some("vbUseDefault"))
        ],
        T::String,
        false,
        fm::f_format_currency
    ),
    f!(
        "FormatDateTime",
        "Strings",
        &[
            p("Date", T::Variant),
            o("NamedFormat", T::Long, Some("vbGeneralDate"))
        ],
        T::String,
        false,
        fm::f_format_date_time
    ),
    f!(
        "FormatNumber",
        "Strings",
        &[
            p("Expression", T::Variant),
            o("NumDigitsAfterDecimal", T::Long, Some("-1")),
            o("IncludeLeadingDigit", T::Long, Some("vbUseDefault")),
            o("UseParensForNegativeNumbers", T::Long, Some("vbUseDefault")),
            o("GroupDigits", T::Long, Some("vbUseDefault"))
        ],
        T::String,
        false,
        fm::f_format_number
    ),
    f!(
        "FormatPercent",
        "Strings",
        &[
            p("Expression", T::Variant),
            o("NumDigitsAfterDecimal", T::Long, Some("-1")),
            o("IncludeLeadingDigit", T::Long, Some("vbUseDefault")),
            o("UseParensForNegativeNumbers", T::Long, Some("vbUseDefault")),
            o("GroupDigits", T::Long, Some("vbUseDefault"))
        ],
        T::String,
        false,
        fm::f_format_percent
    ),
    f!(
        "InStr",
        "Strings",
        &[
            o("Start", T::Variant, Some("1")),
            p("String1", T::Variant),
            p("String2", T::Variant),
            o("Compare", T::Long, Some("vbBinaryCompare"))
        ],
        T::Variant,
        false,
        st::f_instr
    ),
    f!(
        "InStrB",
        "Strings",
        &[
            o("Start", T::Variant, Some("1")),
            p("String1", T::Variant),
            p("String2", T::Variant),
            o("Compare", T::Long, Some("vbBinaryCompare"))
        ],
        T::Variant,
        false,
        st::f_instr_b
    ),
    f!(
        "InStrRev",
        "Strings",
        &[
            p("StringCheck", T::String),
            p("StringMatch", T::String),
            o("Start", T::Long, Some("-1")),
            CMP
        ],
        T::Long,
        false,
        st::f_instr_rev
    ),
    f!(
        "Join",
        "Strings",
        &[
            p("SourceArray", T::Variant),
            o("Delimiter", T::Variant, Some("\" \""))
        ],
        T::String,
        false,
        st::f_join
    ),
    f!(
        "LCase",
        "Strings",
        &[p("String", T::Variant)],
        T::Variant,
        true,
        st::f_lcase
    ),
    f!(
        "Left",
        "Strings",
        &[p("String", T::Variant), p("Length", T::Long)],
        T::Variant,
        true,
        st::f_left
    ),
    f!(
        "LeftB",
        "Strings",
        &[p("String", T::Variant), p("Length", T::Long)],
        T::Variant,
        true,
        st::f_left_b
    ),
    f!(
        "Len",
        "Strings",
        &[p("Expression", T::Variant)],
        T::Variant,
        false,
        st::f_len
    ),
    f!(
        "LenB",
        "Strings",
        &[p("Expression", T::Variant)],
        T::Variant,
        false,
        st::f_len_b
    ),
    f!(
        "LTrim",
        "Strings",
        &[p("String", T::Variant)],
        T::Variant,
        true,
        st::f_ltrim
    ),
    f!(
        "Mid",
        "Strings",
        &[
            p("String", T::Variant),
            p("Start", T::Long),
            o("Length", T::Variant, None)
        ],
        T::Variant,
        true,
        st::f_mid
    ),
    f!(
        "MidB",
        "Strings",
        &[
            p("String", T::Variant),
            p("Start", T::Long),
            o("Length", T::Variant, None)
        ],
        T::Variant,
        true,
        st::f_mid_b
    ),
    f!(
        "Replace",
        "Strings",
        &[
            p("Expression", T::String),
            p("Find", T::String),
            p("Replace", T::String),
            o("Start", T::Long, Some("1")),
            o("Count", T::Long, Some("-1")),
            CMP
        ],
        T::String,
        false,
        st::f_replace
    ),
    f!(
        "Right",
        "Strings",
        &[p("String", T::Variant), p("Length", T::Long)],
        T::Variant,
        true,
        st::f_right
    ),
    f!(
        "RightB",
        "Strings",
        &[p("String", T::Variant), p("Length", T::Long)],
        T::Variant,
        true,
        st::f_right_b
    ),
    f!(
        "RTrim",
        "Strings",
        &[p("String", T::Variant)],
        T::Variant,
        true,
        st::f_rtrim
    ),
    f!(
        "Space",
        "Strings",
        &[p("Number", T::Long)],
        T::String,
        true,
        st::f_space
    ),
    f!(
        "Spc",
        "Strings",
        &[p("Number", T::Integer)],
        T::Variant,
        false,
        ia::f_spc,
        Kind::PrintHelper
    ),
    f!(
        "Split",
        "Strings",
        &[
            p("Expression", T::String),
            o("Delimiter", T::Variant, Some("\" \"")),
            o("Limit", T::Long, Some("-1")),
            CMP
        ],
        T::Variant,
        false,
        st::f_split
    ),
    f!(
        "StrComp",
        "Strings",
        &[p("String1", T::Variant), p("String2", T::Variant), CMP],
        T::Variant,
        false,
        st::f_str_comp
    ),
    f!(
        "StrConv",
        "Strings",
        &[
            p("String", T::Variant),
            p("Conversion", T::Long),
            o("LocaleID", T::Long, None)
        ],
        T::Variant,
        false,
        st::f_str_conv
    ),
    f!(
        "String",
        "Strings",
        &[p("Number", T::Long), p("Character", T::Variant)],
        T::Variant,
        true,
        st::f_string
    ),
    f!(
        "StrReverse",
        "Strings",
        &[p("Expression", T::String)],
        T::String,
        false,
        st::f_str_reverse
    ),
    f!(
        "Tab",
        "Strings",
        &[o("Column", T::Integer, None)],
        T::Variant,
        false,
        ia::f_tab,
        Kind::PrintHelper
    ),
    f!(
        "Trim",
        "Strings",
        &[p("String", T::Variant)],
        T::Variant,
        true,
        st::f_trim
    ),
    f!(
        "UCase",
        "Strings",
        &[p("String", T::Variant)],
        T::Variant,
        true,
        st::f_ucase
    ),
    // VBA.Math
    f!("Abs", "Math", ONE_VAR, T::Variant, false, math::f_abs),
    f!("Atn", "Math", ONE_DBL, T::Double, false, math::f_atn),
    f!("Cos", "Math", ONE_DBL, T::Double, false, math::f_cos),
    f!("Exp", "Math", ONE_DBL, T::Double, false, math::f_exp),
    f!("Log", "Math", ONE_DBL, T::Double, false, math::f_log),
    f!(
        "Randomize",
        "Math",
        &[o("Number", T::Variant, None)],
        T::Variant,
        false,
        math::f_randomize
    ),
    f!(
        "Rnd",
        "Math",
        &[o("Number", T::Variant, Some("1"))],
        T::Single,
        false,
        math::f_rnd
    ),
    f!(
        "Round",
        "Math",
        &[
            p("Number", T::Variant),
            o("NumDigitsAfterDecimal", T::Long, Some("0"))
        ],
        T::Variant,
        false,
        math::f_round
    ),
    f!("Sgn", "Math", ONE_VAR, T::Variant, false, math::f_sgn),
    f!("Sin", "Math", ONE_DBL, T::Double, false, math::f_sin),
    f!("Sqr", "Math", ONE_DBL, T::Double, false, math::f_sqr),
    f!("Tan", "Math", ONE_DBL, T::Double, false, math::f_tan),
    // VBA.DateTime
    f!("Date", "DateTime", &[], T::Date, true, dt::f_date),
    f!(
        "DateAdd",
        "DateTime",
        &[
            p("Interval", T::String),
            p("Number", T::Double),
            p("Date", T::Variant)
        ],
        T::Variant,
        false,
        dt::f_date_add
    ),
    f!(
        "DateDiff",
        "DateTime",
        &[
            p("Interval", T::String),
            p("Date1", T::Variant),
            p("Date2", T::Variant),
            o("FirstDayOfWeek", T::Long, Some("vbSunday")),
            o("FirstWeekOfYear", T::Long, Some("vbFirstJan1"))
        ],
        T::Variant,
        false,
        dt::f_date_diff
    ),
    f!(
        "DatePart",
        "DateTime",
        &[
            p("Interval", T::String),
            p("Date", T::Variant),
            o("FirstDayOfWeek", T::Long, Some("vbSunday")),
            o("FirstWeekOfYear", T::Long, Some("vbFirstJan1"))
        ],
        T::Variant,
        false,
        dt::f_date_part
    ),
    f!(
        "DateSerial",
        "DateTime",
        &[
            p("Year", T::Integer),
            p("Month", T::Integer),
            p("Day", T::Integer)
        ],
        T::Date,
        false,
        dt::f_date_serial
    ),
    f!(
        "DateValue",
        "DateTime",
        &[p("Date", T::Variant)],
        T::Variant,
        false,
        dt::f_date_value
    ),
    f!("Day", "DateTime", ONE_DATE, T::Variant, false, dt::f_day),
    f!("Hour", "DateTime", ONE_DATE, T::Variant, false, dt::f_hour),
    f!(
        "Minute",
        "DateTime",
        ONE_DATE,
        T::Variant,
        false,
        dt::f_minute
    ),
    f!(
        "Month",
        "DateTime",
        ONE_DATE,
        T::Variant,
        false,
        dt::f_month
    ),
    f!(
        "MonthName",
        "DateTime",
        &[
            p("Month", T::Long),
            o("Abbreviate", T::Boolean, Some("False"))
        ],
        T::String,
        false,
        dt::f_month_name
    ),
    f!("Now", "DateTime", &[], T::Date, false, dt::f_now),
    f!(
        "Second",
        "DateTime",
        ONE_DATE,
        T::Variant,
        false,
        dt::f_second
    ),
    f!("Time", "DateTime", &[], T::Date, true, dt::f_time),
    f!("Timer", "DateTime", &[], T::Single, false, dt::f_timer),
    f!(
        "TimeSerial",
        "DateTime",
        &[
            p("Hour", T::Integer),
            p("Minute", T::Integer),
            p("Second", T::Integer)
        ],
        T::Date,
        false,
        dt::f_time_serial
    ),
    f!(
        "TimeValue",
        "DateTime",
        &[p("Time", T::Variant)],
        T::Variant,
        false,
        dt::f_time_value
    ),
    f!(
        "Weekday",
        "DateTime",
        &[
            p("Date", T::Variant),
            o("FirstDayOfWeek", T::Long, Some("vbSunday"))
        ],
        T::Variant,
        false,
        dt::f_weekday
    ),
    f!(
        "WeekdayName",
        "DateTime",
        &[
            p("Weekday", T::Long),
            o("Abbreviate", T::Boolean, Some("False")),
            o("FirstDayOfWeek", T::Long, Some("vbUseSystemDayOfWeek"))
        ],
        T::String,
        false,
        dt::f_weekday_name
    ),
    f!("Year", "DateTime", ONE_DATE, T::Variant, false, dt::f_year),
    // VBA.Information
    f!(
        "IsArray",
        "Information",
        &[p("VarName", T::Variant)],
        T::Boolean,
        false,
        info::f_is_array
    ),
    f!(
        "IsDate",
        "Information",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        info::f_is_date
    ),
    f!(
        "IsEmpty",
        "Information",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        info::f_is_empty
    ),
    f!(
        "IsError",
        "Information",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        info::f_is_error
    ),
    f!(
        "IsMissing",
        "Information",
        &[p("ArgName", T::Variant)],
        T::Boolean,
        false,
        info::f_is_missing,
        Kind::Vm
    ),
    // Emulated identity tokens: opaque, never an address, see the VM.
    f!(
        "ObjPtr",
        "Hidden",
        &[p("Object", T::Object)],
        T::Variant,
        false,
        info::f_identity_token,
        Kind::Vm
    ),
    f!(
        "StrPtr",
        "Hidden",
        &[p("String", T::String)],
        T::Variant,
        false,
        info::f_identity_token,
        Kind::Vm
    ),
    f!(
        "VarPtr",
        "Hidden",
        &[ParamInfo {
            by_ref: true,
            ..p("Variable", T::Variant)
        }],
        T::Variant,
        false,
        info::f_identity_token,
        Kind::Vm
    ),
    f!(
        "IsNull",
        "Information",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        info::f_is_null
    ),
    f!(
        "IsNumeric",
        "Information",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        info::f_is_numeric
    ),
    f!(
        "IsObject",
        "Information",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        info::f_is_object
    ),
    f!(
        "LBound",
        "Information",
        &[
            p("ArrayName", T::Variant),
            o("Dimension", T::Variant, Some("1"))
        ],
        T::Long,
        false,
        info::f_lbound
    ),
    f!(
        "TypeName",
        "Information",
        &[p("VarName", T::Variant)],
        T::String,
        false,
        info::f_type_name
    ),
    f!(
        "UBound",
        "Information",
        &[
            p("ArrayName", T::Variant),
            o("Dimension", T::Variant, Some("1"))
        ],
        T::Long,
        false,
        info::f_ubound
    ),
    f!(
        "VarType",
        "Information",
        &[p("VarName", T::Variant)],
        T::Integer,
        false,
        info::f_var_type
    ),
    // VBA.Conversion
    f!(
        "CBool",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Boolean,
        false,
        cv::f_cbool
    ),
    f!(
        "CByte",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Byte,
        false,
        cv::f_cbyte
    ),
    f!(
        "CCur",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Currency,
        false,
        cv::f_ccur
    ),
    f!(
        "CDate",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Date,
        false,
        cv::f_cdate
    ),
    f!(
        "CDbl",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Double,
        false,
        cv::f_cdbl
    ),
    f!(
        "CDec",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Decimal,
        false,
        cv::f_cdec
    ),
    f!(
        "CInt",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Integer,
        false,
        cv::f_cint
    ),
    f!(
        "CLng",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Long,
        false,
        cv::f_clng
    ),
    f!(
        "CLngLng",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::LongLong,
        false,
        cv::f_clnglng
    ),
    f!(
        "CLngPtr",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Variant,
        false,
        cv::f_clngptr
    ),
    f!(
        "CSng",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Single,
        false,
        cv::f_csng
    ),
    f!(
        "CStr",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::String,
        false,
        cv::f_cstr
    ),
    f!(
        "CVar",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Variant,
        false,
        cv::f_cvar
    ),
    f!(
        "CVDate",
        "Conversion",
        &[p("Expression", T::Variant)],
        T::Date,
        false,
        cv::f_cdate
    ),
    f!(
        "CVErr",
        "Conversion",
        &[p("ErrorNumber", T::Variant)],
        T::Variant,
        false,
        cv::f_cverr
    ),
    f!(
        "Error",
        "Conversion",
        &[o("ErrorNumber", T::Variant, None)],
        T::String,
        true,
        cv::f_error
    ),
    f!("Fix", "Conversion", ONE_VAR, T::Variant, false, math::f_fix),
    f!("Hex", "Conversion", ONE_VAR, T::Variant, true, cv::f_hex),
    f!("Int", "Conversion", ONE_VAR, T::Variant, false, math::f_int),
    f!("Oct", "Conversion", ONE_VAR, T::Variant, true, cv::f_oct),
    f!("Str", "Conversion", ONE_VAR, T::Variant, true, cv::f_str),
    f!(
        "Val",
        "Conversion",
        &[p("String", T::Variant)],
        T::Double,
        false,
        cv::f_val
    ),
    // VBA.Financial
    f!(
        "DDB",
        "Financial",
        &[
            p("Cost", T::Double),
            p("Salvage", T::Double),
            p("Life", T::Double),
            p("Period", T::Double),
            o("Factor", T::Variant, Some("2"))
        ],
        T::Double,
        false,
        fin::f_ddb
    ),
    f!(
        "FV",
        "Financial",
        &[
            p("Rate", T::Double),
            p("NPer", T::Double),
            p("Pmt", T::Double),
            o("PV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod"))
        ],
        T::Double,
        false,
        fin::f_fv
    ),
    f!(
        "IPmt",
        "Financial",
        &[
            p("Rate", T::Double),
            p("Per", T::Double),
            p("NPer", T::Double),
            p("PV", T::Double),
            o("FV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod"))
        ],
        T::Double,
        false,
        fin::f_ipmt
    ),
    f!(
        "IRR",
        "Financial",
        &[
            p("ValueArray", T::Variant),
            o("Guess", T::Variant, Some("0.1"))
        ],
        T::Double,
        false,
        fin::f_irr
    ),
    f!(
        "MIRR",
        "Financial",
        &[
            p("ValueArray", T::Variant),
            p("FinanceRate", T::Double),
            p("ReinvestRate", T::Double)
        ],
        T::Double,
        false,
        fin::f_mirr
    ),
    f!(
        "NPer",
        "Financial",
        &[
            p("Rate", T::Double),
            p("Pmt", T::Double),
            p("PV", T::Double),
            o("FV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod"))
        ],
        T::Double,
        false,
        fin::f_nper
    ),
    f!(
        "NPV",
        "Financial",
        &[p("Rate", T::Double), p("ValueArray", T::Variant)],
        T::Double,
        false,
        fin::f_npv
    ),
    f!(
        "Pmt",
        "Financial",
        &[
            p("Rate", T::Double),
            p("NPer", T::Double),
            p("PV", T::Double),
            o("FV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod"))
        ],
        T::Double,
        false,
        fin::f_pmt
    ),
    f!(
        "PPmt",
        "Financial",
        &[
            p("Rate", T::Double),
            p("Per", T::Double),
            p("NPer", T::Double),
            p("PV", T::Double),
            o("FV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod"))
        ],
        T::Double,
        false,
        fin::f_ppmt
    ),
    f!(
        "PV",
        "Financial",
        &[
            p("Rate", T::Double),
            p("NPer", T::Double),
            p("Pmt", T::Double),
            o("FV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod"))
        ],
        T::Double,
        false,
        fin::f_pv
    ),
    f!(
        "Rate",
        "Financial",
        &[
            p("NPer", T::Double),
            p("Pmt", T::Double),
            p("PV", T::Double),
            o("FV", T::Variant, Some("0")),
            o("Due", T::Variant, Some("DueDate.EndOfPeriod")),
            o("Guess", T::Variant, Some("0.1"))
        ],
        T::Double,
        false,
        fin::f_rate
    ),
    f!(
        "SLN",
        "Financial",
        &[
            p("Cost", T::Double),
            p("Salvage", T::Double),
            p("Life", T::Double)
        ],
        T::Double,
        false,
        fin::f_sln
    ),
    f!(
        "SYD",
        "Financial",
        &[
            p("Cost", T::Double),
            p("Salvage", T::Double),
            p("Life", T::Double),
            p("Period", T::Double)
        ],
        T::Double,
        false,
        fin::f_syd
    ),
    // VBA.Interaction
    f!(
        "Array",
        "Interaction",
        &[pa("ArgList")],
        T::Variant,
        false,
        info::f_array
    ),
    f!(
        "Choose",
        "Interaction",
        &[p("Index", T::Double), pa("Choice")],
        T::Variant,
        false,
        ia::f_choose
    ),
    f!(
        "Environ",
        "Interaction",
        &[p("Expression", T::Variant)],
        T::String,
        true,
        ia::f_environ
    ),
    f!(
        "IIf",
        "Interaction",
        &[
            p("Expression", T::Variant),
            p("TruePart", T::Variant),
            p("FalsePart", T::Variant)
        ],
        T::Variant,
        false,
        ia::f_iif
    ),
    f!(
        "Switch",
        "Interaction",
        &[pa("VarExpr")],
        T::Variant,
        false,
        ia::f_switch
    ),
    // VBA.Information and friends (pure)
    f!(
        "Partition",
        "Information",
        &[
            p("Number", T::Variant),
            p("Start", T::Variant),
            p("Stop", T::Variant),
            p("Interval", T::Variant)
        ],
        T::Variant,
        false,
        misc::f_partition
    ),
    f!(
        "RGB",
        "Information",
        &[
            p("Red", T::Variant),
            p("Green", T::Variant),
            p("Blue", T::Variant)
        ],
        T::Long,
        false,
        misc::f_rgb
    ),
    f!(
        "QBColor",
        "Information",
        &[p("Color", T::Integer)],
        T::Long,
        false,
        misc::f_qb_color
    ),
    f!(
        "IMEStatus",
        "Information",
        &[],
        T::Integer,
        false,
        misc::f_ime_status
    ),
    f!(
        "MacID",
        "Information",
        &[p("Constant", T::String)],
        T::Variant,
        false,
        misc::f_mac_id
    ),
    f!(
        "MacScript",
        "Information",
        &[p("Script", T::String)],
        T::String,
        false,
        misc::f_mac_script
    ),
];

/// Every registered function.
pub fn functions() -> &'static [FunctionInfo] {
    FUNCTIONS
}

/// FNV-1a: the keys are short function names, for which the default
/// hasher's setup cost dominates, and nothing here is attacker-chosen.
#[derive(Default)]
struct Fnv(u64);

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 {
            0xcbf2_9ce4_8422_2325
        } else {
            self.0
        };
        for b in bytes {
            h = (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = h;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

type Index = HashMap<String, usize, BuildHasherDefault<Fnv>>;

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut m = Index::default();
        for (i, f) in FUNCTIONS.iter().enumerate() {
            let n = f.name.to_ascii_lowercase();
            let module = f.module.to_ascii_lowercase();
            m.insert(n.clone(), i);
            m.insert(format!("{module}.{n}"), i);
            m.insert(format!("vba.{n}"), i);
            m.insert(format!("vba.{module}.{n}"), i);
        }
        m
    })
}

/// Looks a name up, ignoring case. Accepts `Left`, `Left$`, `Strings.Left`,
/// `VBA.Left` and `VBA.Strings.Left` (and the `$` forms of each). A `$`
/// suffix on a function without a `$` form does not resolve.
pub fn lookup(name: &str) -> Option<Resolved> {
    // Names are short: lower-case into a stack buffer instead of allocating.
    let name = name.trim();
    let mut buf = [0u8; 48];
    let owned;
    let lower: &str = if name.len() <= buf.len() && name.is_ascii() {
        let b = &mut buf[..name.len()];
        b.copy_from_slice(name.as_bytes());
        b.make_ascii_lowercase();
        std::str::from_utf8(b).ok()?
    } else {
        owned = name.to_ascii_lowercase();
        &owned
    };
    let (base, dollar) = match lower.strip_suffix('$') {
        Some(b) => (b, true),
        None => (lower, false),
    };
    let info = &FUNCTIONS[*index().get(base)?];
    if dollar && !info.dollar {
        return None;
    }
    Some(Resolved { info, dollar })
}

/// Looks `name` up and calls it.
pub fn call(name: &str, ctx: &mut Ctx<'_>, args: &[Option<Variant>]) -> StdResult<Variant> {
    let r = lookup(name).ok_or(StdError::Vba(vba_value::VbaError::new(35)))?;
    r.info.invoke(r.dollar, ctx, args)
}

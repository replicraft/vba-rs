//! `EXCEPINFO`, the `Err` object contents and error conversions.

use crate::hresult::{self, Hresult};
use std::fmt;
use vba_core::VbaError;

/// Result alias for operations that report VBA `Err` state.
pub type AutoResult<T> = Result<T, ErrInfo>;

/// The standard description of an error number, covering the Automation
/// related VBA errors beyond what `vba-core` lists.
pub fn describe(number: i32) -> String {
    let s = match number {
        18 => "User interrupt occurred",
        70 => "Permission denied",
        424 => "Object required",
        429 => "ActiveX component can't create object",
        430 => "Class does not support Automation or does not support expected interface",
        432 => "File name or class name not found during Automation operation",
        445 => "Object doesn't support this action",
        446 => "Object doesn't support named arguments",
        447 => "Object doesn't support current locale setting",
        448 => "Named argument not found",
        449 => "Argument not optional",
        450 => "Wrong number of arguments or invalid property assignment",
        451 => {
            "Property let procedure not defined and property get procedure did not return an object"
        }
        452 => "Invalid ordinal",
        453 => "Specified DLL function not found",
        457 => "This key is already associated with an element of this collection",
        458 => "Variable uses an Automation type not supported in Visual Basic",
        _ => return VbaError::new(number).description().to_string(),
    };
    s.to_string()
}

/// The contents of the VBA `Err` object after a failed call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrInfo {
    /// `Err.Number`.
    pub number: i32,
    /// `Err.Description`.
    pub description: String,
    /// `Err.Source`.
    pub source: String,
    /// `Err.HelpFile`.
    pub help_file: String,
    /// `Err.HelpContext`.
    pub help_context: u32,
}

impl ErrInfo {
    /// An error with the given number and description and no source.
    pub fn new(number: i32, description: impl Into<String>) -> Self {
        ErrInfo {
            number,
            description: description.into(),
            source: String::new(),
            help_file: String::new(),
            help_context: 0,
        }
    }

    /// An error with the standard description of `number`.
    pub fn standard(number: i32) -> Self {
        Self::new(number, describe(number))
    }

    /// Sets the source (builder style).
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// The error number as a [`VbaError`].
    pub fn vba_error(&self) -> VbaError {
        VbaError::new(self.number)
    }

    /// Converts to the `EXCEPINFO` a VBA-implemented server would raise.
    pub fn to_excep_info(&self) -> ExcepInfo {
        ExcepInfo {
            code: 0,
            scode: hresult::number_to_hresult(self.number),
            source: self.source.clone(),
            description: self.description.clone(),
            help_file: self.help_file.clone(),
            help_context: self.help_context,
        }
    }
}

impl From<VbaError> for ErrInfo {
    fn from(e: VbaError) -> Self {
        ErrInfo::standard(e.number())
    }
}

impl From<ErrInfo> for VbaError {
    fn from(e: ErrInfo) -> Self {
        e.vba_error()
    }
}

impl fmt::Display for ErrInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Run-time error '{}': {}", self.number, self.description)
    }
}

impl std::error::Error for ErrInfo {}

/// The Rust form of `EXCEPINFO` (MS-OAUT).
///
/// Either `code` (the legacy 16-bit `wCode`) or `scode` identifies the error;
/// `scode` wins when non-zero. Deferred filling (`pfnDeferredFillIn`) is not
/// modelled: callees fill the structure eagerly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExcepInfo {
    /// Legacy error code (`wCode`); used only when `scode` is zero.
    pub code: u16,
    /// The `HRESULT` (`scode`).
    pub scode: Hresult,
    /// Source of the error, usually `"Project.Class"`.
    pub source: String,
    /// Human readable description.
    pub description: String,
    /// Help file path.
    pub help_file: String,
    /// Help context id.
    pub help_context: u32,
}

/// A failed `IDispatch` call: a bare HRESULT, or `DISP_E_EXCEPTION` with
/// details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispError {
    /// A failure HRESULT with no `EXCEPINFO`.
    Hresult(Hresult),
    /// `DISP_E_EXCEPTION` with its `EXCEPINFO`.
    Exception(ExcepInfo),
}

impl DispError {
    /// `DISP_E_MEMBERNOTFOUND`.
    pub const MEMBER_NOT_FOUND: DispError = DispError::Hresult(hresult::DISP_E_MEMBERNOTFOUND);
    /// `DISP_E_TYPEMISMATCH`.
    pub const TYPE_MISMATCH: DispError = DispError::Hresult(hresult::DISP_E_TYPEMISMATCH);
    /// `DISP_E_BADPARAMCOUNT`.
    pub const BAD_PARAM_COUNT: DispError = DispError::Hresult(hresult::DISP_E_BADPARAMCOUNT);
    /// `DISP_E_PARAMNOTOPTIONAL`.
    pub const PARAM_NOT_OPTIONAL: DispError = DispError::Hresult(hresult::DISP_E_PARAMNOTOPTIONAL);
    /// `DISP_E_PARAMNOTFOUND`.
    pub const PARAM_NOT_FOUND: DispError = DispError::Hresult(hresult::DISP_E_PARAMNOTFOUND);
    /// `DISP_E_UNKNOWNNAME`.
    pub const UNKNOWN_NAME: DispError = DispError::Hresult(hresult::DISP_E_UNKNOWNNAME);

    /// An exception carrying a VBA error number and description.
    pub fn raise(number: i32, description: impl Into<String>, source: impl Into<String>) -> Self {
        DispError::Exception(ExcepInfo {
            scode: hresult::number_to_hresult(number),
            description: description.into(),
            source: source.into(),
            ..ExcepInfo::default()
        })
    }

    /// The HRESULT that `Invoke` would return.
    pub fn hresult(&self) -> Hresult {
        match self {
            DispError::Hresult(h) => *h,
            DispError::Exception(_) => hresult::DISP_E_EXCEPTION,
        }
    }

    /// Converts to the VBA `Err` contents. `default_source` is used when the
    /// callee gave none (VBA uses the object's class name).
    pub fn into_err_info(self, default_source: &str) -> ErrInfo {
        match self {
            DispError::Hresult(h) => ErrInfo {
                number: hresult::hresult_to_number(h),
                description: hresult::hresult_description(h),
                source: default_source.to_string(),
                help_file: String::new(),
                help_context: 0,
            },
            DispError::Exception(e) => {
                let number = if e.scode != 0 {
                    hresult::hresult_to_number(e.scode)
                } else {
                    i32::from(e.code)
                };
                ErrInfo {
                    number,
                    description: if e.description.is_empty() {
                        describe(number)
                    } else {
                        e.description
                    },
                    source: if e.source.is_empty() {
                        default_source.to_string()
                    } else {
                        e.source
                    },
                    help_file: e.help_file,
                    help_context: e.help_context,
                }
            }
        }
    }
}

impl From<ErrInfo> for DispError {
    fn from(e: ErrInfo) -> Self {
        DispError::Exception(e.to_excep_info())
    }
}

impl From<VbaError> for DispError {
    fn from(e: VbaError) -> Self {
        ErrInfo::from(e).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hresult_table() {
        let cases = [
            (hresult::DISP_E_MEMBERNOTFOUND, 438),
            (hresult::DISP_E_TYPEMISMATCH, 13),
            (hresult::DISP_E_PARAMNOTFOUND, 448),
            (hresult::DISP_E_PARAMNOTOPTIONAL, 449),
            (hresult::DISP_E_BADPARAMCOUNT, 450),
            (hresult::DISP_E_OVERFLOW, 6),
            (hresult::DISP_E_DIVBYZERO, 11),
            (hresult::DISP_E_BADINDEX, 9),
            (hresult::E_ACCESSDENIED, 70),
            (0x800A_000D_u32 as i32, 13),
            (hresult::VB_OBJECT_ERROR + 1, hresult::VB_OBJECT_ERROR + 1),
        ];
        for (hr, n) in cases {
            assert_eq!(hresult::hresult_to_number(hr), n, "{hr:#x}");
        }
    }

    #[test]
    fn number_roundtrip() {
        for n in [5, 13, 438, 65535, hresult::VB_OBJECT_ERROR + 513] {
            assert_eq!(hresult::hresult_to_number(hresult::number_to_hresult(n)), n);
        }
    }

    #[test]
    fn excepinfo_to_err() {
        let e = DispError::Exception(ExcepInfo {
            scode: hresult::VB_OBJECT_ERROR + 513,
            description: "custom".into(),
            help_file: "h.hlp".into(),
            help_context: 7,
            ..ExcepInfo::default()
        });
        let i = e.into_err_info("Proj.Cls");
        assert_eq!(i.number, hresult::VB_OBJECT_ERROR + 513);
        assert_eq!(i.description, "custom");
        assert_eq!(i.source, "Proj.Cls");
        assert_eq!(i.help_file, "h.hlp");
        assert_eq!(i.help_context, 7);
    }

    #[test]
    fn wcode_used_when_scode_zero() {
        let e = DispError::Exception(ExcepInfo {
            code: 1004,
            ..ExcepInfo::default()
        });
        assert_eq!(e.into_err_info("x").number, 1004);
    }

    #[test]
    fn bare_hresult() {
        let i = DispError::MEMBER_NOT_FOUND.into_err_info("Cls");
        assert_eq!(i.number, 438);
        assert_eq!(
            i.description,
            "Object doesn't support this property or method"
        );
        assert_eq!(i.source, "Cls");
        assert_eq!(
            DispError::TYPE_MISMATCH.hresult(),
            hresult::DISP_E_TYPEMISMATCH
        );
    }
}

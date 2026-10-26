//! HRESULT constants and their mapping to VBA error numbers.
//!
//! Sources: MS-OAUT (IDispatch return codes) and the Microsoft documentation
//! of VBA trappable errors. The mapping for codes VBA does not document
//! precisely follows commonly observed behaviour and is not verified against
//! a real host; see `docs/runtime/automation.md`.

use crate::error::describe;
use windows_sys::Win32::Foundation as f;

/// An `HRESULT` as a signed 32-bit integer.
pub type Hresult = i32;

macro_rules! hr {
    ($($(#[$m:meta])* $name:ident = $v:expr;)*) => {
        $($(#[$m])* pub const $name: Hresult = $v;)*
    };
}

hr! {
    /// Success.
    S_OK = f::S_OK;
    /// Not implemented.
    E_NOTIMPL = f::E_NOTIMPL;
    /// No such interface.
    E_NOINTERFACE = f::E_NOINTERFACE;
    /// Unspecified failure.
    E_FAIL = f::E_FAIL;
    /// Access denied.
    E_ACCESSDENIED = f::E_ACCESSDENIED;
    /// Out of memory.
    E_OUTOFMEMORY = f::E_OUTOFMEMORY;
    /// Invalid argument.
    E_INVALIDARG = f::E_INVALIDARG;
    /// Unknown interface.
    DISP_E_UNKNOWNINTERFACE = f::DISP_E_UNKNOWNINTERFACE;
    /// Member not found.
    DISP_E_MEMBERNOTFOUND = f::DISP_E_MEMBERNOTFOUND;
    /// Parameter not found.
    DISP_E_PARAMNOTFOUND = f::DISP_E_PARAMNOTFOUND;
    /// Type mismatch.
    DISP_E_TYPEMISMATCH = f::DISP_E_TYPEMISMATCH;
    /// Unknown name.
    DISP_E_UNKNOWNNAME = f::DISP_E_UNKNOWNNAME;
    /// No named arguments.
    DISP_E_NONAMEDARGS = f::DISP_E_NONAMEDARGS;
    /// Bad variable type.
    DISP_E_BADVARTYPE = f::DISP_E_BADVARTYPE;
    /// The callee raised an exception (details in EXCEPINFO).
    DISP_E_EXCEPTION = f::DISP_E_EXCEPTION;
    /// Overflow.
    DISP_E_OVERFLOW = f::DISP_E_OVERFLOW;
    /// Bad index.
    DISP_E_BADINDEX = f::DISP_E_BADINDEX;
    /// Unknown LCID.
    DISP_E_UNKNOWNLCID = f::DISP_E_UNKNOWNLCID;
    /// Array is locked.
    DISP_E_ARRAYISLOCKED = f::DISP_E_ARRAYISLOCKED;
    /// Wrong number of arguments.
    DISP_E_BADPARAMCOUNT = f::DISP_E_BADPARAMCOUNT;
    /// A required parameter was omitted.
    DISP_E_PARAMNOTOPTIONAL = f::DISP_E_PARAMNOTOPTIONAL;
    /// Bad callee.
    DISP_E_BADCALLEE = f::DISP_E_BADCALLEE;
    /// Not a collection.
    DISP_E_NOTACOLLECTION = f::DISP_E_NOTACOLLECTION;
    /// Division by zero.
    DISP_E_DIVBYZERO = f::DISP_E_DIVBYZERO;
    /// Buffer too small.
    DISP_E_BUFFERTOOSMALL = f::DISP_E_BUFFERTOOSMALL;
    /// Type library: type mismatch.
    TYPE_E_TYPEMISMATCH = f::TYPE_E_TYPEMISMATCH;
    /// Type library: out of bounds.
    TYPE_E_OUTOFBOUNDS = f::TYPE_E_OUTOFBOUNDS;
    /// Invalid class string.
    CO_E_CLASSSTRING = f::CO_E_CLASSSTRING;
    /// Class not registered.
    REGDB_E_CLASSNOTREG = f::REGDB_E_CLASSNOTREG;
    /// Server execution failed.
    CO_E_SERVER_EXEC_FAILURE = f::CO_E_SERVER_EXEC_FAILURE;
}

/// `vbObjectError`: base of user-defined error numbers (`&H80040000`).
pub const VB_OBJECT_ERROR: i32 = 0x8004_0000u32 as i32;

/// Facility 10 (VBA) error HRESULTs have this high word: `0x800A`.
const VBA_FACILITY_HIGH: u32 = 0x800A_0000;

/// Maps an HRESULT to the value of `Err.Number`.
///
/// * VBA-style HRESULTs `0x800A0000 | n` give `n` (what `CVErr(n)` and
///   VBA-implemented servers produce).
/// * The documented `DISP_E_*` codes map to the VBA error the runtime raises
///   for them (for example `DISP_E_MEMBERNOTFOUND` is 438).
/// * Anything else is passed through unchanged, which is how VBA reports
///   foreign errors (for example `vbObjectError + n`).
pub fn hresult_to_number(hr: Hresult) -> i32 {
    let u = hr as u32;
    if u & 0xFFFF_0000 == VBA_FACILITY_HIGH {
        return (u & 0xFFFF) as i32;
    }
    match hr {
        DISP_E_UNKNOWNINTERFACE | E_NOINTERFACE => 430,
        DISP_E_MEMBERNOTFOUND
        | DISP_E_UNKNOWNNAME
        | DISP_E_NONAMEDARGS
        | DISP_E_BADCALLEE
        | DISP_E_NOTACOLLECTION => 438,
        DISP_E_PARAMNOTFOUND => 448,
        DISP_E_TYPEMISMATCH | TYPE_E_TYPEMISMATCH => 13,
        DISP_E_BADVARTYPE => 458,
        DISP_E_OVERFLOW => 6,
        DISP_E_BADINDEX | TYPE_E_OUTOFBOUNDS => 9,
        DISP_E_ARRAYISLOCKED => 10,
        DISP_E_BADPARAMCOUNT => 450,
        DISP_E_PARAMNOTOPTIONAL => 449,
        DISP_E_DIVBYZERO => 11,
        E_OUTOFMEMORY => 7,
        E_INVALIDARG => 5,
        E_ACCESSDENIED => 70,
        E_NOTIMPL => 445,
        CO_E_CLASSSTRING | REGDB_E_CLASSNOTREG | CO_E_SERVER_EXEC_FAILURE => 429,
        other => other,
    }
}

/// Maps a VBA error number to the HRESULT a VBA-implemented server would
/// report: `1..=65535` become `0x800A0000 | n`; other numbers (for example
/// `vbObjectError + n`) are used as the HRESULT directly.
pub fn number_to_hresult(number: i32) -> Hresult {
    if (1..=0xFFFF).contains(&number) {
        (VBA_FACILITY_HIGH | number as u32) as i32
    } else {
        number
    }
}

/// The standard description for the HRESULT after mapping to an error number.
pub fn hresult_description(hr: Hresult) -> String {
    describe(hresult_to_number(hr))
}

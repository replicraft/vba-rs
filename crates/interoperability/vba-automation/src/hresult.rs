//! HRESULT constants and their mapping to VBA error numbers.
//!
//! Sources: MS-OAUT (IDispatch return codes) and the Microsoft documentation
//! of VBA trappable errors. The mapping for codes VBA does not document
//! precisely follows commonly observed behaviour and is not verified against
//! a real host; see `docs/automation.md`.

use crate::error::describe;

/// An `HRESULT` as a signed 32-bit integer.
pub type Hresult = i32;

macro_rules! hr {
    ($($(#[$m:meta])* $name:ident = $v:expr;)*) => {
        $($(#[$m])* pub const $name: Hresult = $v as u32 as i32;)*
    };
}

hr! {
    /// Success.
    S_OK = 0x0000_0000u32;
    /// Not implemented.
    E_NOTIMPL = 0x8000_4001u32;
    /// No such interface.
    E_NOINTERFACE = 0x8000_4002u32;
    /// Unspecified failure.
    E_FAIL = 0x8000_4005u32;
    /// Access denied.
    E_ACCESSDENIED = 0x8007_0005u32;
    /// Out of memory.
    E_OUTOFMEMORY = 0x8007_000Eu32;
    /// Invalid argument.
    E_INVALIDARG = 0x8007_0057u32;
    /// Unknown interface.
    DISP_E_UNKNOWNINTERFACE = 0x8002_0001u32;
    /// Member not found.
    DISP_E_MEMBERNOTFOUND = 0x8002_0003u32;
    /// Parameter not found.
    DISP_E_PARAMNOTFOUND = 0x8002_0004u32;
    /// Type mismatch.
    DISP_E_TYPEMISMATCH = 0x8002_0005u32;
    /// Unknown name.
    DISP_E_UNKNOWNNAME = 0x8002_0006u32;
    /// No named arguments.
    DISP_E_NONAMEDARGS = 0x8002_0007u32;
    /// Bad variable type.
    DISP_E_BADVARTYPE = 0x8002_0008u32;
    /// The callee raised an exception (details in EXCEPINFO).
    DISP_E_EXCEPTION = 0x8002_0009u32;
    /// Overflow.
    DISP_E_OVERFLOW = 0x8002_000Au32;
    /// Bad index.
    DISP_E_BADINDEX = 0x8002_000Bu32;
    /// Unknown LCID.
    DISP_E_UNKNOWNLCID = 0x8002_000Cu32;
    /// Array is locked.
    DISP_E_ARRAYISLOCKED = 0x8002_000Du32;
    /// Wrong number of arguments.
    DISP_E_BADPARAMCOUNT = 0x8002_000Eu32;
    /// A required parameter was omitted.
    DISP_E_PARAMNOTOPTIONAL = 0x8002_000Fu32;
    /// Bad callee.
    DISP_E_BADCALLEE = 0x8002_0010u32;
    /// Not a collection.
    DISP_E_NOTACOLLECTION = 0x8002_0011u32;
    /// Division by zero.
    DISP_E_DIVBYZERO = 0x8002_0012u32;
    /// Buffer too small.
    DISP_E_BUFFERTOOSMALL = 0x8002_0013u32;
    /// Type library: type mismatch.
    TYPE_E_TYPEMISMATCH = 0x8002_8CA0u32;
    /// Type library: out of bounds.
    TYPE_E_OUTOFBOUNDS = 0x8002_8CA1u32;
    /// Invalid class string.
    CO_E_CLASSSTRING = 0x8004_01F3u32;
    /// Class not registered.
    REGDB_E_CLASSNOTREG = 0x8004_0154u32;
    /// Server execution failed.
    CO_E_SERVER_EXEC_FAILURE = 0x8008_0005u32;
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

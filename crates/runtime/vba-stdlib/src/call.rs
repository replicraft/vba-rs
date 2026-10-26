//! Calling convention shared by every intrinsic: [`Ctx`] carries the host and
//! the mutable library state, [`Args`] the (possibly omitted) arguments.

use crate::error::{ARGUMENT_NOT_OPTIONAL, StdError, StdResult};
use crate::host::Host;
use crate::math::RndState;
use vba_value::{CompareMode, PointerWidth, Variant, VbaResult, VbaString, convert};

/// Mutable library state that lives as long as the program: the `Rnd`
/// generator.
#[derive(Debug, Clone, Default)]
pub struct State {
    /// Pseudo-random generator state (`Rnd` / `Randomize`).
    pub rnd: RndState,
}

/// Receives the scanning work a built-in does, so that the caller can bound
/// CPU time and not only the number of statements.
pub trait WorkMeter {
    /// Accounts for `units` (one per UTF-16 unit looked at). `false` when the
    /// caller's budget is spent and the function must stop.
    fn charge(&self, units: u64) -> bool;
}

/// Everything an intrinsic may consult besides its arguments.
pub struct Ctx<'a> {
    /// Clock, environment and locale.
    pub host: &'a dyn Host,
    /// Library state (random generator).
    pub state: &'a mut State,
    /// The module's `Option Compare` setting, used when a compare argument is
    /// omitted or `vbUseCompareOption`.
    pub compare: CompareMode,
    /// The module's `Option Base` setting (0 or 1), used by `Array`.
    pub array_base: i32,
    /// Pointer width for `CLngPtr`.
    pub pointer_width: PointerWidth,
    /// The longest string, in UTF-16 units, a function may build. A larger
    /// result is error 7 before it is allocated.
    pub max_string_units: usize,
    /// The most elements an array-returning function may build (`Split`).
    pub max_array_elements: usize,
    /// Where scanning work is accounted; `None` leaves it unmetered.
    pub work: Option<&'a dyn WorkMeter>,
}

/// Default of [`Ctx::max_string_units`] (2 GiB of UTF-16 units).
pub const DEFAULT_MAX_STRING_UNITS: usize = 1 << 30;
/// Default of [`Ctx::max_array_elements`].
pub const DEFAULT_MAX_ARRAY_ELEMENTS: usize = 1 << 28;

impl<'a> Ctx<'a> {
    /// A context with `Option Compare Binary`, `Option Base 0` and 64-bit
    /// pointers.
    pub fn new(host: &'a dyn Host, state: &'a mut State) -> Self {
        Ctx {
            host,
            state,
            compare: CompareMode::Binary,
            array_base: 0,
            pointer_width: PointerWidth::Bits64,
            max_string_units: DEFAULT_MAX_STRING_UNITS,
            max_array_elements: DEFAULT_MAX_ARRAY_ELEMENTS,
            work: None,
        }
    }

    /// Accounts for `units` of scanning work (one per UTF-16 unit examined)
    /// before it is done. Fails with [`StdError::WorkExceeded`] when the
    /// caller's budget cannot pay for it.
    pub fn charge(&self, units: u64) -> StdResult<()> {
        match self.work {
            Some(w) if !w.charge(units) => Err(StdError::WorkExceeded),
            _ => Ok(()),
        }
    }
}

/// Units of work to scan the arguments of a call: the length of every
/// string and the size of every array of strings.
pub fn scan_units(args: &[Option<Variant>]) -> u64 {
    fn units(v: &Variant) -> u64 {
        match v {
            Variant::String(s) => s.len() as u64,
            Variant::Array(a) => a.iter().map(|e| 1 + units(e)).sum(),
            _ => 1,
        }
    }
    args.iter().flatten().map(units).sum()
}

/// The arguments of a call. `None` marks an omitted optional argument.
#[derive(Clone, Copy)]
pub struct Args<'a> {
    /// The positional arguments.
    pub items: &'a [Option<Variant>],
    /// `true` when the function was called through its `$` name (`Left$`):
    /// the result is a plain `String` and `Null` arguments are error 94.
    pub dollar: bool,
}

impl<'a> Args<'a> {
    /// Number of argument slots.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` if there are no argument slots.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The argument at `i`, or `None` if omitted or absent.
    pub fn get(&self, i: usize) -> Option<&'a Variant> {
        self.items.get(i).and_then(|v| v.as_ref())
    }

    /// The required argument at `i` (error 449 if omitted).
    pub fn req(&self, i: usize) -> StdResult<&'a Variant> {
        self.get(i).ok_or_else(|| ARGUMENT_NOT_OPTIONAL.into())
    }

    /// The argument at `i` as a string; `Null` is error 94.
    pub fn string(&self, i: usize) -> StdResult<VbaString> {
        Ok(convert::to_vba_string(self.req(i)?)?)
    }

    /// The argument at `i` as a `Long` (rounded half to even).
    pub fn long(&self, i: usize) -> StdResult<i32> {
        Ok(convert::to_long(self.req(i)?)?)
    }

    /// The argument at `i` as a `Double`.
    pub fn double(&self, i: usize) -> StdResult<f64> {
        Ok(convert::to_double(self.req(i)?)?)
    }

    /// Optional `Long` with a default for an omitted argument.
    pub fn long_or(&self, i: usize, default: i32) -> StdResult<i32> {
        match self.get(i) {
            Some(v) => Ok(convert::to_long(v)?),
            None => Ok(default),
        }
    }

    /// Optional `Double` with a default.
    pub fn double_or(&self, i: usize, default: f64) -> StdResult<f64> {
        match self.get(i) {
            Some(v) => Ok(convert::to_double(v)?),
            None => Ok(default),
        }
    }

    /// Optional `Boolean` with a default.
    pub fn bool_or(&self, i: usize, default: bool) -> StdResult<bool> {
        match self.get(i) {
            Some(v) => Ok(convert::to_boolean(v)?),
            None => Ok(default),
        }
    }

    /// `true` if the argument is present and `Null`.
    pub fn is_null(&self, i: usize) -> bool {
        matches!(self.get(i), Some(Variant::Null))
    }
}

/// Resolves a `Compare` argument (`vbUseCompareOption` = -1, `vbBinaryCompare`
/// = 0, `vbTextCompare` = 1); anything else, including `vbDatabaseCompare`
/// (2, Access only), is error 5.
pub fn compare_arg(value: Option<i32>, default: CompareMode) -> VbaResult<CompareMode> {
    match value {
        None | Some(-1) => Ok(default),
        Some(0) => Ok(CompareMode::Binary),
        Some(1) => Ok(CompareMode::Text),
        Some(_) => Err(vba_value::VbaError::INVALID_PROCEDURE_CALL),
    }
}

/// Wraps text as a `String` variant.
pub fn text(s: impl Into<VbaString>) -> Variant {
    Variant::String(s.into())
}

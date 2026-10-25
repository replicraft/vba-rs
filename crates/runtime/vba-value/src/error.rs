//! VBA run-time error numbers.
//!
//! The numbers and descriptions follow the documented VBA trappable errors.
//! This type is intentionally tiny; richer error handling (`Err` object,
//! sources, line numbers) belongs to the runtime.

use std::fmt;

/// A VBA run-time error identified by its error number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VbaError {
    number: i32,
}

/// Result alias used throughout the crate.
pub type VbaResult<T> = Result<T, VbaError>;

impl VbaError {
    /// Error 5, "Invalid procedure call or argument".
    pub const INVALID_PROCEDURE_CALL: VbaError = VbaError::new(5);
    /// Error 6, "Overflow".
    pub const OVERFLOW: VbaError = VbaError::new(6);
    /// Error 7, "Out of memory".
    pub const OUT_OF_MEMORY: VbaError = VbaError::new(7);
    /// Error 9, "Subscript out of range".
    pub const SUBSCRIPT_OUT_OF_RANGE: VbaError = VbaError::new(9);
    /// Error 10, "This array is fixed or temporarily locked".
    pub const ARRAY_FIXED_OR_LOCKED: VbaError = VbaError::new(10);
    /// Error 11, "Division by zero".
    pub const DIVISION_BY_ZERO: VbaError = VbaError::new(11);
    /// Error 13, "Type mismatch".
    pub const TYPE_MISMATCH: VbaError = VbaError::new(13);
    /// Error 94, "Invalid use of Null".
    pub const INVALID_USE_OF_NULL: VbaError = VbaError::new(94);

    /// Creates an error with the given VBA error number.
    pub const fn new(number: i32) -> Self {
        Self { number }
    }

    /// Returns the VBA error number (the value of `Err.Number`).
    pub const fn number(self) -> i32 {
        self.number
    }

    /// Returns the standard VBA description for this error number.
    pub const fn description(self) -> &'static str {
        match self.number {
            3 => "Return without GoSub",
            5 => "Invalid procedure call or argument",
            6 => "Overflow",
            7 => "Out of memory",
            9 => "Subscript out of range",
            10 => "This array is fixed or temporarily locked",
            11 => "Division by zero",
            13 => "Type mismatch",
            14 => "Out of string space",
            28 => "Out of stack space",
            35 => "Sub or Function not defined",
            48 => "Error in loading DLL",
            51 => "Internal error",
            53 => "File not found",
            91 => "Object variable or With block variable not set",
            92 => "For loop not initialized",
            93 => "Invalid pattern string",
            94 => "Invalid use of Null",
            438 => "Object doesn't support this property or method",
            _ => "Application-defined or object-defined error",
        }
    }
}

impl fmt::Display for VbaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Run-time error '{}': {}",
            self.number,
            self.description()
        )
    }
}

impl std::error::Error for VbaError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_descriptions() {
        assert_eq!(VbaError::OVERFLOW.number(), 6);
        assert_eq!(VbaError::TYPE_MISMATCH.description(), "Type mismatch");
        assert_eq!(
            VbaError::new(9999).description(),
            "Application-defined or object-defined error"
        );
        assert_eq!(
            VbaError::DIVISION_BY_ZERO.to_string(),
            "Run-time error '11': Division by zero"
        );
    }
}

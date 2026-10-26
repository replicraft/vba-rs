//! Error and outcome types.

use std::fmt;

use vba_pcode::{Opcode, PcodeError};

/// Why a procedure was not lowered: the first instruction (or header) that
/// the conservative subset does not cover. Lowering never guesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    /// The opcode at which lowering stopped (the header opcode for
    /// problems with the signature or declarations).
    pub opcode: Opcode,
    /// The procedure name.
    pub procedure: String,
    /// The 0-based line index in the module cache.
    pub line: usize,
    /// A short explanation.
    pub reason: String,
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported {} in `{}` at line {}: {}",
            self.opcode.label(),
            self.procedure,
            self.line,
            self.reason
        )
    }
}

/// What happened to one procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Lowered to IR that passes the verifier.
    Lowered,
    /// Not lowered.
    Unsupported(Unsupported),
}

impl Outcome {
    /// Whether the procedure was lowered.
    pub fn is_lowered(&self) -> bool {
        matches!(self, Outcome::Lowered)
    }
}

/// Failures that affect the whole project rather than one procedure.
#[derive(Debug, thiserror::Error)]
pub enum LowerError {
    /// The cache could not be decoded at all.
    #[error("compiled code: {0}")]
    Pcode(#[from] PcodeError),
    /// The program shell could not be built.
    #[error("program shell: {0}")]
    Shell(String),
    /// The IR of a lowered procedure failed the verifier (an internal error
    /// of this crate; the message names the function).
    #[error("internal: verifier rejected the lowered program: {0}")]
    Verify(String),
}

impl From<LowerError> for vba_core::VbaError {
    /// Any failure to lower is "Internal error" (51) from the language's
    /// point of view.
    fn from(_: LowerError) -> Self {
        vba_core::VbaError::new(51)
    }
}

//! A VM-friendly intermediate representation for VBA.
//!
//! [`lower`] turns the bound model of `vba-sema` into [`IrProgram`]: every
//! procedure becomes a [`Function`], a control-flow graph of basic blocks
//! over typed virtual registers and addressable frame slots. Variant/typed
//! operators carry the `vba-value` operator families and overflow mode,
//! `ByRef` arguments are explicit references, late-bound access is an
//! `Invoke` with `IDispatch` flavours, `On Error` is explicit handler state
//! with statement-granular `Resume` points, and every instruction records
//! its source span and statement.
//!
//! [`dump_program`] produces the text form used by the snapshot tests and
//! [`verify`] checks the IR invariants. See `docs/ir.md`.

mod ir;
mod lower;
mod print;
mod verify;

pub use ir::*;
pub use lower::{LowerError, lower};
pub use print::{DumpOptions, dump_function, dump_program};
pub use verify::{VerifyError, inst_def, inst_uses, normal_successors, verify};

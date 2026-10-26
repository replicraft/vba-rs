//! A deterministic, synchronous interpreter for the `vba-ir` intermediate
//! representation.
//!
//! The VM executes the basic-block IR produced by `vba-ir` over `vba-value`
//! variants, calls the `vba-stdlib` registry for intrinsic functions and the
//! `vba-automation` object model for objects owned by the host. It owns
//! frames, `ByRef` references (to locals, module variables, fields, array
//! elements and user-defined-type fields), class instances with reference
//! counting and `Class_Terminate`, `Implements` dispatch, `WithEvents`, the
//! `Err` object and `On Error` / `Resume` / `GoSub` flow.
//!
//! Everything that reaches outside the interpreter goes through two
//! injected seams: the [`vba_host::Host`] (permissions, limits, objects,
//! UI, `Declare` hooks) and [`Services`] (files, `Shell`, `Command`). The VM
//! has no ambient authority and no async runtime, and given the same host,
//! services and clock it is deterministic. See `docs/runtime/vm.md`.
//!
//! ```
//! use std::rc::Rc;
//! use vba_vm::{BufferSink, Vm, VmConfig, NoServices};
//!
//! # fn demo(ir: vba_ir::IrProgram) {
//! let sink = BufferSink::new();
//! let mut vm = Vm::new(
//!     Rc::new(ir),
//!     Rc::new(vba_host::HeadlessHost::new()),
//!     Box::new(NoServices),
//!     Box::new(vba_stdlib::FixedHost::default()),
//!     Box::new(sink.clone()),
//!     VmConfig::default(),
//! )
//! .unwrap();
//! let main = vm.find_procedure("Main").unwrap();
//! vm.run_procedure(main, vec![]).unwrap();
//! println!("{}", sink.contents());
//! # }
//! ```

mod callbacks;
mod calls;
mod config;
mod declare;
mod embed;
mod error;
mod exec;
mod heap;
mod intrinsics;
mod invoke;
mod printfmt;
mod reentry;
mod services;
mod state;
mod vm;

pub use config::{BufferSink, DebugSink, NullSink, VmConfig};
pub use error::{ErrOrigin, LimitKind, RtErr, RuntimeError, StackFrame, VmError, describe};
pub use printfmt::{ZONE, format_print, format_print_width, print_value};
pub use reentry::{ReentryError, ReentrySlot, vm_error_to_disp};
pub use services::{
    IoReply, IoRequest, NoServices, PrintItem, RecordValue, ServiceError, Services, Shape,
};
pub use vm::{Arg, BindStats, RunOutput, RunStatus, Vm};

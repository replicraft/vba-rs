//! The lower-level crates, one module per workspace crate.
//!
//! Use these when the [`Engine`](crate::Engine) convenience layer is too
//! coarse. They follow the versioning of their own crates, not the facade's
//! stability promise, and are always pinned to the same release as this
//! facade.

pub use vba_ast as ast;
pub use vba_automation as automation;
pub use vba_core as core;
pub use vba_diagnostics as diagnostics;
pub use vba_ffi as ffi;
pub use vba_format as format;
pub use vba_host as host;
pub use vba_ir as ir;
pub use vba_lexer as lexer;
pub use vba_profile as profile;
pub use vba_project as project;
pub use vba_runtime as runtime;
pub use vba_sema as sema;
pub use vba_source as source;
pub use vba_stdlib as stdlib;
pub use vba_syntax as syntax;
pub use vba_value as value;
pub use vba_vm as vm;
#[cfg(feature = "winapi")]
pub use vba_winapi as winapi;
#[cfg(feature = "wine")]
pub use vba_wine as wine;

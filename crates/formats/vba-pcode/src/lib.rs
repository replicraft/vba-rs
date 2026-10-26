//! Decoder and disassembler for the compiled-code ("p-code") cache that
//! precedes the source text in the module streams of a VBA project.
//!
//! Facts come from the repository documents `docs/pcode-format.md`,
//! `docs/pcode-opcodes.md` and `docs/pcode.md`, the public MS-OVBA
//! specification, and direct observation of a test project. The crate takes
//! plain byte slices; reading the container is the caller's job.
//!
//! Only VBA 7.0 on 32-bit Windows is verified against real material. Every
//! other layout is implemented from the specification text and reported as
//! [`Verification::SpecUnverified`].

mod error;
mod layout;
mod project_stream;
mod reader;

pub use error::{PcodeError, Result, UnsupportedLayout};
pub use layout::{Bitness, Layout, VERIFIED_VERSION, Verification, VersionFamily};
pub use project_stream::{IdentTable, ProjectCache, SIGNATURE, parse_identifiers, version_word};

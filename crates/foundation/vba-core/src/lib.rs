//! Types shared across vba-rs crates.
//!
//! This crate has no dependencies and no knowledge of values, syntax, hosts
//! or the operating system.

pub mod error;
pub mod memory;

pub use error::{VbaError, VbaResult};
pub use memory::{Charge, MemoryExceeded, MemoryMeter, MeterScope, Tracked};

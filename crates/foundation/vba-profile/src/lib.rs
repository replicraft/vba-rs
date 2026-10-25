//! The VBA-visible target model.
//!
//! A [`Profile`] describes the VBA implementation a program is written for:
//! language generation, host application, and the operating system and
//! bitness *as seen by VBA code*. Nothing here reads the machine vba-rs runs
//! on, so evaluating `#If Win64` for a 32-bit Windows profile gives the same
//! answer on every host.
//!
//! Sources: MS-VBAL §3.3.5.2 (reserved identifiers) and §4 (conditional
//! compilation); the predefined constants follow the Microsoft VBA
//! conditional-compilation documentation. Behaviour that could not be checked
//! against a real host is called out in the item documentation.

mod catalog;
mod constants;
mod generation;
mod host;
mod platform;
mod profile;

pub use catalog::{default_profile, find_profile, profiles};
pub use constants::CompilationConstant;
pub use generation::{Feature, LanguageGeneration, Maturity};
pub use host::{HostApplication, HostInfo};
pub use platform::{Bitness, OperatingSystem, Target};
pub use profile::Profile;

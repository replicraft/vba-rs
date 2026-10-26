//! Native `Scripting.Dictionary` and `Scripting.FileSystemObject` (with
//! `TextStream`, `File`, `Folder`, `Files`, `SubFolders`) for vba-rs.
//!
//! The objects are ordinary automation objects ([`vba_automation::Dispatch`])
//! living in the host's object table, so late-bound calls, default members,
//! `For Each` and `Err.Source` behave as for any other host object.
//! [`ScriptingHost`] decorates a host so that `CreateObject` returns them;
//! creation needs `Capability::ComCreate` like any ProgID.
//!
//! All file access goes through [`ScriptFs`], whose implementation applies
//! the host policy (`FileRead` / `FileWrite`, with prefix scoping) on a
//! virtual file system; this crate has no path of its own to the real file
//! system. Rules not stated by the Microsoft Learn pages are tagged
//! **unverified** in the module documentation of each object.
//!
//! `New Scripting.Dictionary` (early binding) needs a type library and is not
//! provided; use `CreateObject`.

mod dictionary;
mod entries;
mod enumobj;
mod fs;
mod fso;
mod host;
mod textstream;
mod util;

pub use dictionary::Dictionary;
pub use entries::{Collection, File, Folder};
pub use fs::{EntryInfo, EntryStat, FsResult, ScriptFs};
pub use fso::FileSystemObject;
pub use host::ScriptingHost;
pub use textstream::TextStream;

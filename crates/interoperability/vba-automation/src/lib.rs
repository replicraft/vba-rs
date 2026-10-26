//! The VBA-visible OLE Automation object model, without COM.
//!
//! This crate models what VBA sees of `IDispatch` (MS-OAUT): member lookup by
//! name, `Invoke` with Method/PropertyGet/PropertyPut/PropertyPutRef, named
//! and optional arguments, ByRef write-back, `DISPID_VALUE`,
//! `DISPID_NEWENUM` and `DISPID_EVALUATE`, `EXCEPINFO`-style errors mapped to
//! `Err`, reference-counted object lifetimes, connection points for
//! `WithEvents`, and the call-resolution rules of MS-VBAL §5 (default
//! member chaining, `!` access, Let versus Set).
//!
//! Nothing here touches COM, the operating system or any host application.
//! Objects are values implementing [`Dispatch`] that live in an
//! [`ObjectTable`]; `Variant::Object` holds an [`ObjectHandle`] into it.
//!
//! Ownership convention: arguments passed to an invocation are borrowed;
//! an object reference returned from an invocation carries one reference
//! that the receiver must release (COM's out-parameter rule).
//!
//! ```
//! use vba_automation::{call, testing::MemObject, ObjectTable, Final, Step, CallArgs};
//! use vba_value::{ObjectRef, Variant};
//!
//! let table = ObjectTable::new();
//! let h = table.insert_obj(MemObject::new("Thing").property("Name", Variant::from("x")));
//! let root = Variant::Object(ObjectRef::Handle(h));
//! let mut steps = [Step::member("NAME", CallArgs::none())];
//! let v = call::eval_chain(&table, &root, &mut steps, Final::Get).unwrap();
//! assert_eq!(v, Variant::from("x"));
//! ```

pub mod call;
pub mod dispatch;
pub mod enumerate;
pub mod error;
pub mod events;
pub mod hresult;
pub mod table;
pub mod testing;

pub use call::{CallArgs, Final, Step, invoke_early};
pub use dispatch::{
    Arg, Bound, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ParamInfo, bind, names_equal,
};
pub use enumerate::{ForEach, VariantEnum};
pub use error::{AutoResult, DispError, ErrInfo, ExcepInfo};
pub use events::{ConnectionPoint, Cookie, EventSink, EventSource, WithEventsVar};
pub use table::{FOREIGN_HANDLE_BIT, ObjectTable, Owned};
pub use vba_value::{ObjectHandle, ObjectRef};

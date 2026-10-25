//! The `For Each` protocol over `DISPID_NEWENUM` (`IEnumVARIANT`).

use crate::dispatch::{DispId, DispParams, InvokeFlags};
use crate::error::{AutoResult, DispError, ErrInfo};
use crate::table::{ObjectTable, Owned};
use vba_value::{ObjectRef, Variant};

/// `IEnumVARIANT`. Implemented by the enumerator object returned from
/// `_NewEnum`, exposed through [`crate::Dispatch::enumerator`].
pub trait VariantEnum {
    /// `Next(1)`: the next element, or `None` at the end. An object element
    /// carries one reference owned by the caller.
    fn next(&self, table: &ObjectTable) -> Result<Option<Variant>, DispError>;

    /// `Reset`.
    fn reset(&self) -> Result<(), DispError>;
}

/// An in-progress `For Each` over an object.
pub struct ForEach<'t> {
    table: &'t ObjectTable,
    enumerator: Owned<'t>,
}

impl<'t> ForEach<'t> {
    /// Starts enumerating `collection`.
    ///
    /// `Nothing` is error 91; a non-object is error 438; an object without a
    /// `_NewEnum` member, or whose `_NewEnum` result is not an enumerator, is
    /// error 438 ("Object doesn't support this property or method").
    pub fn start(table: &'t ObjectTable, collection: &Variant) -> AutoResult<Self> {
        let h = match collection {
            Variant::Object(ObjectRef::Handle(h)) => *h,
            Variant::Object(ObjectRef::Nothing) => return Err(ErrInfo::standard(91)),
            _ => return Err(ErrInfo::standard(438)),
        };
        let obj = table.get(h)?;
        let mut dp = DispParams::new();
        let r = obj
            .invoke(
                table,
                DispId::NEWENUM,
                InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
                &mut dp,
            )
            .map_err(|e| e.into_err_info(&obj.class_name()))?;
        let eh = match r {
            Variant::Object(ObjectRef::Handle(eh)) => eh,
            other => {
                table.release_value(&other);
                return Err(ErrInfo::standard(438));
            }
        };
        let enumerator = Owned::adopt(table, eh);
        let eo = table.get(eh)?;
        if eo.enumerator().is_none() {
            return Err(ErrInfo::standard(438));
        }
        Ok(ForEach { table, enumerator })
    }

    /// The next element (object elements carry a reference the caller owns),
    /// or `None` when exhausted.
    pub fn next_item(&mut self) -> AutoResult<Option<Variant>> {
        let eo = self.table.get(self.enumerator.handle())?;
        let en = eo.enumerator().ok_or(ErrInfo::standard(438))?;
        en.next(self.table)
            .map_err(|e| e.into_err_info(&eo.class_name()))
    }
}

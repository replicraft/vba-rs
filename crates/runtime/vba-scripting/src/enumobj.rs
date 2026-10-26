//! The enumerator object returned by `_NewEnum` of the collections here.

use std::cell::Cell;
use vba_automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectTable, VariantEnum,
};
use vba_value::{ObjectRef, Variant};

type Maker = Box<dyn Fn(&ObjectTable, usize) -> Variant>;

/// A snapshot of `len` values produced on demand, served one by one.
pub(crate) struct ItemsEnum {
    len: usize,
    make: Maker,
    pos: Cell<usize>,
}

impl VariantEnum for ItemsEnum {
    fn next(&self, table: &ObjectTable) -> Result<Option<Variant>, DispError> {
        let p = self.pos.get();
        if p >= self.len {
            return Ok(None);
        }
        self.pos.set(p + 1);
        Ok(Some((self.make)(table, p)))
    }

    fn reset(&self) -> Result<(), DispError> {
        self.pos.set(0);
        Ok(())
    }
}

/// The `IEnumVARIANT` object.
pub(crate) struct EnumObject(ItemsEnum);

impl EnumObject {
    /// Inserts an enumerator over `items` into the table and returns the
    /// object variant (one reference owned by the caller). Object elements
    /// get a reference for each element served.
    pub(crate) fn over(table: &ObjectTable, items: Vec<Variant>) -> Variant {
        let len = items.len();
        Self::with(table, len, move |t, i| {
            let v = items[i].clone();
            t.add_ref_value(&v);
            v
        })
    }

    /// An enumerator whose `i`-th element is created by `make`, which
    /// returns it with the reference the receiver will own.
    pub(crate) fn with(
        table: &ObjectTable,
        len: usize,
        make: impl Fn(&ObjectTable, usize) -> Variant + 'static,
    ) -> Variant {
        let e = EnumObject(ItemsEnum {
            len,
            make: Box::new(make),
            pos: Cell::new(0),
        });
        Variant::Object(ObjectRef::Handle(table.insert_obj(e)))
    }
}

impl Dispatch for EnumObject {
    fn class_name(&self) -> String {
        "IEnumVARIANT".into()
    }
    fn member_id(&self, _: &str) -> Option<DispId> {
        None
    }
    fn member_info(&self, _: DispId) -> Option<MemberInfo> {
        None
    }
    fn invoke(
        &self,
        _: &ObjectTable,
        _: DispId,
        _: InvokeFlags,
        _: &mut DispParams,
    ) -> Result<Variant, DispError> {
        Err(DispError::MEMBER_NOT_FOUND)
    }
    fn enumerator(&self) -> Option<&dyn VariantEnum> {
        Some(&self.0)
    }
}

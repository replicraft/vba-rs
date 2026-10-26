//! `Scripting.Dictionary`.
//!
//! Implemented from the Learn pages of the Scripting runtime library
//! (Dictionary object, `Add`, `Exists`, `Items`, `Keys`, `Remove`,
//! `RemoveAll`, `Item`, `Key`, `Count`, `CompareMode`). Rules the pages state:
//!
//! * `Add` with an existing key is an error; `Remove` of a missing key is an
//!   error; changing `CompareMode` of a dictionary that holds data is an
//!   error; `Key` of a missing key is an error.
//! * Reading `Item` of a missing key creates the key with an `Empty` item;
//!   assigning `Item` of a missing key creates the key.
//! * Keys can be anything except an array.
//! * `Items` and `Keys` return zero-based arrays; `For Each` visits the keys.
//!
//! Error numbers are not on the pages and are **unverified**: 457 for a
//! duplicate key (the number of the `Collection` duplicate-key error), 32811
//! (`&H800A802B`, "Element not found") for a missing key, 5 for the other
//! failures. Key equality (**unverified**): numbers of every numeric subtype
//! compare by value, strings compare exactly (`vbBinaryCompare`, the default)
//! or by the en-US text collation (`vbTextCompare`), a number never equals a
//! string, objects compare by identity, `Empty` is the empty string. Items
//! and keys are kept in insertion order (the order `Keys` and `Items`
//! report).

use indexmap::IndexMap;
use std::cell::RefCell;
use vba_automation::{Bound, DispError, InvokeFlags, ObjectHandle, ObjectTable};
use vba_value::collation::{Collation, EN_US};
use vba_value::{Bounds, ObjectRef, VarType, Variant, VbaArray, convert};

use crate::enumobj::EnumObject;
use crate::util::{Def, dispatch_impl, getter, method, property, raise};

/// Error 457: "This key is already associated with an element of this collection".
const KEY_EXISTS: i32 = 457;
/// Error 32811: "Element not found".
const NOT_FOUND: i32 = 32811;

const ITEM: i32 = 0;
const ADD: i32 = 1;
const EXISTS: i32 = 2;
const ITEMS: i32 = 3;
const KEYS: i32 = 4;
const REMOVE: i32 = 5;
const REMOVE_ALL: i32 = 6;
const COUNT: i32 = 7;
const KEY: i32 = 8;
const COMPARE_MODE: i32 = 9;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NormKey {
    Str(Vec<u16>),
    Num(u64),
    Bool(bool),
    Obj(u64),
    Err(i32),
}

struct Entry {
    key: Variant,
    item: Variant,
}

struct State {
    mode: i32,
    map: IndexMap<NormKey, Entry>,
}

/// A `Scripting.Dictionary` object.
pub struct Dictionary {
    state: RefCell<State>,
}

impl Default for Dictionary {
    fn default() -> Self {
        Dictionary::new()
    }
}

impl Dictionary {
    pub(crate) const CLASS: &'static str = "Dictionary";
    pub(crate) const DEFS: &'static [Def] = &[
        property("Item", ITEM, &["Key"], true),
        method("Add", ADD, &["Key", "Item"], 2),
        method("Exists", EXISTS, &["Key"], 1),
        method("Items", ITEMS, &[], 0),
        method("Keys", KEYS, &[], 0),
        method("Remove", REMOVE, &["Key"], 1),
        method("RemoveAll", REMOVE_ALL, &[], 0),
        getter("Count", COUNT, &[]),
        property("Key", KEY, &["Key"], false),
        property("CompareMode", COMPARE_MODE, &[], false),
    ];

    /// An empty dictionary with binary key comparison.
    pub fn new() -> Dictionary {
        Dictionary {
            state: RefCell::new(State {
                mode: 0,
                map: IndexMap::new(),
            }),
        }
    }

    fn text_mode(mode: i32) -> bool {
        mode == 1 || mode > 2
    }

    fn norm(mode: i32, v: &Variant) -> Result<NormKey, DispError> {
        Ok(match v {
            Variant::Array(_) | Variant::UserDefined(_) | Variant::Null => {
                return Err(raise(5, "Invalid procedure call or argument", Self::CLASS));
            }
            Variant::Empty => NormKey::Str(Vec::new()),
            Variant::String(s) => {
                if Self::text_mode(mode) {
                    NormKey::Str(EN_US.fold(s.as_units()))
                } else {
                    NormKey::Str(s.as_units().to_vec())
                }
            }
            Variant::Boolean(b) => NormKey::Bool(*b),
            Variant::Object(ObjectRef::Handle(h)) => NormKey::Obj(h.0),
            Variant::Object(ObjectRef::Nothing) => NormKey::Obj(0),
            Variant::Error(e) => NormKey::Err(e.number()),
            other => {
                let d = convert::to_double(other)?;
                NormKey::Num(if d == 0.0 { 0.0f64 } else { d }.to_bits())
            }
        })
    }

    fn not_found() -> DispError {
        raise(NOT_FOUND, "Element not found", Self::CLASS)
    }

    fn array_of(table: &ObjectTable, values: Vec<Variant>) -> Result<Variant, DispError> {
        let arr = if values.is_empty() {
            VbaArray::new_empty(VarType::Variant, 0)
        } else {
            let upper = i32::try_from(values.len() - 1).map_err(|_| DispError::TYPE_MISMATCH)?;
            let mut a = VbaArray::new_dynamic(VarType::Variant);
            a.redim(&[Bounds::new(0, upper)], false)?;
            for (i, v) in values.into_iter().enumerate() {
                a.set(&[i as i32], v)?;
            }
            a
        };
        let out = Variant::Array(Box::new(arr));
        table.add_ref_value(&out);
        Ok(out)
    }

    fn call(
        &self,
        table: &ObjectTable,
        id: i32,
        flags: InvokeFlags,
        b: &Bound,
    ) -> Result<Variant, DispError> {
        let mut st = self.state.borrow_mut();
        let mode = st.mode;
        let arg = |i: usize| &b.slots[i].value;
        match id {
            ADD => {
                let k = Self::norm(mode, arg(0))?;
                if st.map.contains_key(&k) {
                    return Err(raise(
                        KEY_EXISTS,
                        "This key is already associated with an element of this collection",
                        Self::CLASS,
                    ));
                }
                table.add_ref_value(arg(0));
                table.add_ref_value(arg(1));
                st.map.insert(
                    k,
                    Entry {
                        key: arg(0).clone(),
                        item: arg(1).clone(),
                    },
                );
                Ok(Variant::Empty)
            }
            EXISTS => {
                let k = Self::norm(mode, arg(0))?;
                Ok(Variant::Boolean(st.map.contains_key(&k)))
            }
            ITEM if flags.is_write() => {
                let k = Self::norm(mode, arg(0))?;
                let new = b.value.clone().unwrap_or(Variant::Empty);
                table.add_ref_value(&new);
                match st.map.get_mut(&k) {
                    Some(e) => {
                        let old = std::mem::replace(&mut e.item, new);
                        table.release_value(&old);
                    }
                    None => {
                        table.add_ref_value(arg(0));
                        st.map.insert(
                            k,
                            Entry {
                                key: arg(0).clone(),
                                item: new,
                            },
                        );
                    }
                }
                Ok(Variant::Empty)
            }
            ITEM => {
                let k = Self::norm(mode, arg(0))?;
                if let Some(e) = st.map.get(&k) {
                    let v = e.item.clone();
                    table.add_ref_value(&v);
                    return Ok(v);
                }
                // Documented: reading a missing key creates it, left empty.
                table.add_ref_value(arg(0));
                st.map.insert(
                    k,
                    Entry {
                        key: arg(0).clone(),
                        item: Variant::Empty,
                    },
                );
                Ok(Variant::Empty)
            }
            ITEMS => Self::array_of(table, st.map.values().map(|e| e.item.clone()).collect()),
            KEYS => Self::array_of(table, st.map.values().map(|e| e.key.clone()).collect()),
            REMOVE => {
                let k = Self::norm(mode, arg(0))?;
                match st.map.shift_remove(&k) {
                    Some(e) => {
                        table.release_value(&e.key);
                        table.release_value(&e.item);
                        Ok(Variant::Empty)
                    }
                    None => Err(Self::not_found()),
                }
            }
            REMOVE_ALL => {
                let all: Vec<Entry> = st.map.drain(..).map(|(_, e)| e).collect();
                for e in all {
                    table.release_value(&e.key);
                    table.release_value(&e.item);
                }
                Ok(Variant::Empty)
            }
            COUNT => Ok(Variant::Long(st.map.len() as i32)),
            KEY => {
                if !flags.is_write() {
                    return Err(DispError::MEMBER_NOT_FOUND);
                }
                let old = Self::norm(mode, arg(0))?;
                let newkey = b.value.clone().unwrap_or(Variant::Empty);
                let new = Self::norm(mode, &newkey)?;
                let Some(pos) = st.map.get_index_of(&old) else {
                    return Err(Self::not_found());
                };
                if new != old && st.map.contains_key(&new) {
                    return Err(raise(
                        KEY_EXISTS,
                        "This key is already associated with an element of this collection",
                        Self::CLASS,
                    ));
                }
                table.add_ref_value(&newkey);
                let entries: Vec<(NormKey, Entry)> = st.map.drain(..).collect();
                for (i, (nk, mut e)) in entries.into_iter().enumerate() {
                    if i == pos {
                        table.release_value(&e.key);
                        e.key = newkey.clone();
                        st.map.insert(new.clone(), e);
                    } else {
                        st.map.insert(nk, e);
                    }
                }
                Ok(Variant::Empty)
            }
            COMPARE_MODE => {
                if flags.is_write() {
                    let m = convert::to_long(&b.value.clone().unwrap_or(Variant::Empty))?;
                    if !st.map.is_empty() {
                        return Err(raise(5, "Invalid procedure call or argument", Self::CLASS));
                    }
                    if m < -1 {
                        return Err(raise(5, "Invalid procedure call or argument", Self::CLASS));
                    }
                    st.mode = m;
                    Ok(Variant::Empty)
                } else {
                    Ok(Variant::Long(st.mode))
                }
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }

    fn new_enum(&self, table: &ObjectTable) -> Result<Variant, DispError> {
        let keys: Vec<Variant> = self
            .state
            .borrow()
            .map
            .values()
            .map(|e| e.key.clone())
            .collect();
        Ok(EnumObject::over(table, keys))
    }

    fn on_terminate(&self, _: &ObjectTable) {}

    fn held(&self) -> Vec<ObjectHandle> {
        let mut out = Vec::new();
        for e in self.state.borrow().map.values() {
            for v in [&e.key, &e.item] {
                if let Variant::Object(ObjectRef::Handle(h)) = v {
                    out.push(*h);
                }
            }
        }
        out
    }
}

dispatch_impl!(Dictionary);

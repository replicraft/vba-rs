//! An in-memory automation object for tests and documentation examples.
//!
//! [`MemObject`] is a configurable [`Dispatch`] implementation: properties
//! backed by cells, methods and indexed properties backed by closures, a
//! default member, collections with `_NewEnum`, a connection point and a
//! terminate log. It is not a model of any real library.

use crate::dispatch::{
    Arg, Bound, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ParamInfo, bind, names_equal,
};
use crate::enumerate::VariantEnum;
use crate::error::DispError;
use crate::events::{ConnectionPoint, EventSource};
use crate::table::ObjectTable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use vba_value::{ObjectHandle, ObjectRef, Variant};

/// Arguments of one call as seen by a member closure.
pub struct Call {
    /// Bound parameters, one per declared non-ParamArray parameter.
    pub args: Vec<Arg>,
    /// ParamArray elements.
    pub rest: Vec<Arg>,
    /// The value assigned by a property put.
    pub value: Option<Variant>,
}

/// A member implementation.
pub type Func = Rc<dyn Fn(&ObjectTable, &mut Call) -> Result<Variant, DispError>>;

struct Member {
    name: String,
    id: DispId,
    params: Vec<ParamInfo>,
    method: Option<Func>,
    get: Option<Func>,
    put: Option<Func>,
    put_ref: Option<Func>,
}

impl Member {
    fn supports(&self) -> InvokeFlags {
        let mut f = InvokeFlags::NONE;
        if self.method.is_some() {
            f = f | InvokeFlags::METHOD;
        }
        if self.get.is_some() {
            f = f | InvokeFlags::PROPERTY_GET;
        }
        if self.put.is_some() {
            f = f | InvokeFlags::PROPERTY_PUT;
        }
        if self.put_ref.is_some() {
            f = f | InvokeFlags::PROPERTY_PUT_REF;
        }
        f
    }
}

/// Shared log of terminate events (class names in termination order).
pub type TerminateLog = Rc<RefCell<Vec<String>>>;

/// A configurable in-memory automation object.
pub struct MemObject {
    class: String,
    members: RefCell<Vec<Member>>,
    next_id: Cell<i32>,
    log: Option<TerminateLog>,
    held: RefCell<Vec<ObjectHandle>>,
    points: ConnectionPoint,
    enumerator: Option<ListEnum>,
}

impl MemObject {
    /// A new object of the given class with no members.
    pub fn new(class: &str) -> Self {
        MemObject {
            class: class.to_string(),
            members: RefCell::default(),
            next_id: Cell::new(1),
            log: None,
            held: RefCell::default(),
            points: ConnectionPoint::new(&format!("{class}Events")),
            enumerator: None,
        }
    }

    fn add(self, name: &str, params: Vec<ParamInfo>, f: impl FnOnce(&mut Member)) -> Self {
        let id = DispId(self.next_id.get());
        self.next_id.set(id.0 + 1);
        let mut m = Member {
            name: name.to_string(),
            id,
            params,
            method: None,
            get: None,
            put: None,
            put_ref: None,
        };
        f(&mut m);
        self.members.borrow_mut().push(m);
        self
    }

    /// A read/write property holding `initial`. Both Let and Set are
    /// accepted and store the value (object references are counted).
    pub fn property(self, name: &str, initial: Variant) -> Self {
        let cell = Rc::new(RefCell::new(initial));
        let (g, p) = (cell.clone(), cell);
        self.add(name, vec![], |m| {
            m.get = Some(Rc::new(move |t, _| {
                let v = g.borrow().clone();
                t.add_ref_value(&v);
                Ok(v)
            }));
            let put: Func = Rc::new(move |t, c| {
                let v = c.value.clone().unwrap_or(Variant::Empty);
                t.add_ref_value(&v);
                let old = p.replace(v);
                t.release_value(&old);
                Ok(Variant::Empty)
            });
            m.put = Some(put.clone());
            m.put_ref = Some(put);
        })
    }

    /// A value property: supports Let only (a `Property Let` without `Set`).
    pub fn value_property(self, name: &str, initial: Variant) -> Self {
        let cell = Rc::new(RefCell::new(initial));
        let (g, p) = (cell.clone(), cell);
        self.add(name, vec![], |m| {
            m.get = Some(Rc::new(move |_, _| Ok(g.borrow().clone())));
            m.put = Some(Rc::new(move |_, c| {
                p.replace(c.value.clone().unwrap_or(Variant::Empty));
                Ok(Variant::Empty)
            }));
        })
    }

    /// An object property: supports Set only (a `Property Set`).
    pub fn object_property(self, name: &str) -> Self {
        let cell = Rc::new(RefCell::new(Variant::Object(ObjectRef::Nothing)));
        let (g, p) = (cell.clone(), cell);
        self.add(name, vec![], |m| {
            m.get = Some(Rc::new(move |t, _| {
                let v = g.borrow().clone();
                t.add_ref_value(&v);
                Ok(v)
            }));
            m.put_ref = Some(Rc::new(move |t, c| {
                let v = c.value.clone().unwrap_or(Variant::Empty);
                t.add_ref_value(&v);
                let old = p.replace(v);
                t.release_value(&old);
                Ok(Variant::Empty)
            }));
        })
    }

    /// A read-only property computed by `f`.
    pub fn computed(self, name: &str, f: impl Fn(&ObjectTable) -> Variant + 'static) -> Self {
        self.add(name, vec![], |m| {
            m.get = Some(Rc::new(move |t, _| Ok(f(t))));
        })
    }

    /// A method (also readable as a property get, like most automation
    /// functions) with the given parameters.
    pub fn method(
        self,
        name: &str,
        params: Vec<ParamInfo>,
        f: impl Fn(&ObjectTable, &mut Call) -> Result<Variant, DispError> + 'static,
    ) -> Self {
        self.add(name, params, |m| m.method = Some(Rc::new(f)))
    }

    /// A parameterized property: `get` reads, `put` (if any) assigns
    /// (`Call::value` holds the new value).
    pub fn indexed_property(
        self,
        name: &str,
        params: Vec<ParamInfo>,
        get: impl Fn(&ObjectTable, &mut Call) -> Result<Variant, DispError> + 'static,
        put: Option<Func>,
    ) -> Self {
        self.add(name, params, |m| {
            m.get = Some(Rc::new(get));
            m.put = put;
        })
    }

    /// Makes an existing member the default member (`DISPID_VALUE`).
    ///
    /// # Panics
    /// Panics if the member does not exist (test-only helper).
    pub fn default_member(self, name: &str) -> Self {
        self.with_dispid(name, DispId::VALUE)
    }

    /// Gives an existing member a specific id (for example
    /// `DISPID_EVALUATE`).
    ///
    /// # Panics
    /// Panics if the member does not exist (test-only helper).
    pub fn with_dispid(self, name: &str, id: DispId) -> Self {
        {
            let mut ms = self.members.borrow_mut();
            let m = ms
                .iter_mut()
                .find(|m| names_equal(&m.name, name))
                .expect("member must exist");
            m.id = id;
        }
        self
    }

    /// Turns the object into a collection of `items`: adds `Count`, a default
    /// `Item(Index)` (1-based, error 9 when out of range, or a case-insensitive
    /// key match is not supported) and `_NewEnum`.
    pub fn collection(mut self, items: Vec<Variant>) -> Self {
        let items = Rc::new(items);
        let (c, i) = (items.clone(), items.clone());
        self = self.computed("Count", move |_| Variant::Long(c.len() as i32));
        self = self.method(
            "Item",
            vec![ParamInfo::required("Index")],
            move |t, call| {
                let idx = match &call.args[0].value {
                    Variant::Long(n) => i64::from(*n),
                    Variant::Integer(n) => i64::from(*n),
                    _ => return Err(DispError::TYPE_MISMATCH),
                };
                let v = usize::try_from(idx - 1)
                    .ok()
                    .and_then(|k| i.get(k))
                    .ok_or(DispError::Hresult(crate::hresult::DISP_E_BADINDEX))?
                    .clone();
                t.add_ref_value(&v);
                Ok(v)
            },
        );
        self = self.default_member("Item");
        self.enumerator = Some(ListEnum {
            items,
            pos: Cell::new(0),
        });
        self
    }

    /// Records the class name in `log` when the object terminates.
    pub fn log_terminate(mut self, log: &TerminateLog) -> Self {
        self.log = Some(log.clone());
        self
    }

    /// Declares a reference the object owns (released after terminate).
    pub fn hold(self, h: ObjectHandle) -> Self {
        self.held.borrow_mut().push(h);
        self
    }

    /// The object's connection point, to fire events from tests.
    pub fn events(&self) -> &ConnectionPoint {
        &self.points
    }

    fn find(&self, id: DispId) -> Option<(Option<Func>, InvokeFlags, Vec<ParamInfo>, Member2)> {
        let ms = self.members.borrow();
        let m = ms.iter().find(|m| m.id == id)?;
        Some((
            m.method.clone(),
            m.supports(),
            m.params.clone(),
            Member2 {
                get: m.get.clone(),
                put: m.put.clone(),
                put_ref: m.put_ref.clone(),
            },
        ))
    }
}

struct Member2 {
    get: Option<Func>,
    put: Option<Func>,
    put_ref: Option<Func>,
}

struct ListEnum {
    items: Rc<Vec<Variant>>,
    pos: Cell<usize>,
}

impl VariantEnum for ListEnum {
    fn next(&self, table: &ObjectTable) -> Result<Option<Variant>, DispError> {
        let p = self.pos.get();
        match self.items.get(p) {
            Some(v) => {
                self.pos.set(p + 1);
                table.add_ref_value(v);
                Ok(Some(v.clone()))
            }
            None => Ok(None),
        }
    }

    fn reset(&self) -> Result<(), DispError> {
        self.pos.set(0);
        Ok(())
    }
}

/// The enumerator object returned from `_NewEnum` of a collection.
struct EnumObject(ListEnum);

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

impl Dispatch for MemObject {
    fn class_name(&self) -> String {
        self.class.clone()
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        self.members
            .borrow()
            .iter()
            .find(|m| names_equal(&m.name, name))
            .map(|m| m.id)
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        self.members
            .borrow()
            .iter()
            .find(|m| m.id == id)
            .map(|m| MemberInfo {
                name: m.name.clone(),
                id: m.id,
                supports: m.supports(),
                params: m.params.clone(),
            })
    }

    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        if id == DispId::NEWENUM {
            let e = self
                .enumerator
                .as_ref()
                .ok_or(DispError::MEMBER_NOT_FOUND)?;
            let h = table.insert_obj(EnumObject(ListEnum {
                items: e.items.clone(),
                pos: Cell::new(0),
            }));
            return Ok(Variant::Object(ObjectRef::Handle(h)));
        }
        let (method, _, decl, rest) = self.find(id).ok_or(DispError::MEMBER_NOT_FOUND)?;
        let f = if flags.contains(InvokeFlags::PROPERTY_PUT_REF) {
            rest.put_ref
        } else if flags.contains(InvokeFlags::PROPERTY_PUT) {
            rest.put
        } else if flags.intersects(InvokeFlags::METHOD) && method.is_some() {
            method
        } else if flags.intersects(InvokeFlags::PROPERTY_GET) && rest.get.is_some() {
            rest.get
        } else {
            method
        }
        .ok_or(DispError::MEMBER_NOT_FOUND)?;
        let bound: Bound = bind(&decl, params)?;
        let mut call = Call {
            args: bound.slots.clone(),
            rest: bound.rest.clone(),
            value: bound.value.clone(),
        };
        let r = f(table, &mut call);
        let mut out = bound;
        out.slots = call.args;
        out.rest = call.rest;
        out.write_back(params);
        r
    }

    fn terminate(&self, _: &ObjectTable) {
        if let Some(l) = &self.log {
            l.borrow_mut().push(self.class.clone());
        }
    }

    fn held_references(&self) -> Vec<ObjectHandle> {
        self.held.borrow().clone()
    }

    fn event_source(&self) -> Option<&dyn EventSource> {
        Some(&self.points)
    }
}

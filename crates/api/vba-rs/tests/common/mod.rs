//! A toy "form" host object for the embedding tests: a plain `Dispatch`
//! implementation with a property bag and controls. It stands for the
//! object an embedding application passes as the `Me` of a form module.

#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use vba::automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectHandle, ObjectRef,
    ObjectTable, ParamInfo,
};
use vba::{Arg, CallResult, Error, Instance, ModuleKind, Project, Variant};

/// The state of one toy form, shared with the test.
#[derive(Default)]
pub struct FormState {
    /// Property and control values by lower-case name.
    pub props: RefCell<BTreeMap<String, Variant>>,
    /// Every host member the VBA code used, in order (`get Caption`,
    /// `put Caption`, `call Requery`...).
    pub log: RefCell<Vec<String>>,
}

impl FormState {
    pub fn new(props: &[(&str, Variant)]) -> Rc<FormState> {
        let s = FormState::default();
        for (k, v) in props {
            s.props.borrow_mut().insert(k.to_lowercase(), v.clone());
        }
        Rc::new(s)
    }

    pub fn get(&self, name: &str) -> Variant {
        self.props
            .borrow()
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or(Variant::Empty)
    }

    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

const CONTROLS: DispId = DispId(1);
const REQUERY: DispId = DispId(2);
const FIRST_PROP: i32 = 100;

/// The `Me` of a toy form: unknown names are properties of the bag.
pub struct FormHost {
    pub state: Rc<FormState>,
}

impl FormHost {
    pub fn new(state: &Rc<FormState>) -> Rc<FormHost> {
        Rc::new(FormHost {
            state: state.clone(),
        })
    }

    fn prop_name(&self, id: DispId) -> Option<String> {
        let i = usize::try_from(id.0 - FIRST_PROP).ok()?;
        self.state.props.borrow().keys().nth(i).cloned()
    }
}

fn text(v: &Variant) -> String {
    match v {
        Variant::String(s) => s.to_string_lossy(),
        other => format!("{other:?}"),
    }
}

impl Dispatch for FormHost {
    fn class_name(&self) -> String {
        "Form".into()
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "controls" => Some(CONTROLS),
            "requery" => Some(REQUERY),
            n => self
                .state
                .props
                .borrow()
                .keys()
                .position(|k| k == n)
                .map(|i| DispId(FIRST_PROP + i as i32)),
        }
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let (name, params, supports) = match id {
            CONTROLS => (
                "Controls".to_owned(),
                vec![ParamInfo::required("Name").by_val()],
                InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            ),
            REQUERY => (
                "Requery".to_owned(),
                vec![],
                InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            ),
            DispId::VALUE => (
                "Item".to_owned(),
                vec![ParamInfo::required("Name").by_val()],
                InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
            ),
            other => (
                self.prop_name(other)?,
                vec![],
                InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
            ),
        };
        Some(MemberInfo {
            name,
            id,
            supports,
            params,
        })
    }

    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let st = &self.state;
        match id {
            CONTROLS => {
                let name = text(&params.args[0].value);
                st.log.borrow_mut().push(format!("controls {name}"));
                if !st.props.borrow().contains_key(&name.to_lowercase()) {
                    return Err(DispError::MEMBER_NOT_FOUND);
                }
                let h = table.insert_obj(ControlHost {
                    state: st.clone(),
                    name: name.to_lowercase(),
                });
                Ok(Variant::Object(ObjectRef::Handle(h)))
            }
            REQUERY => {
                st.log.borrow_mut().push("call Requery".into());
                Ok(Variant::Empty)
            }
            id => {
                let name = if id == DispId::VALUE {
                    text(&params.args[0].value).to_lowercase()
                } else {
                    self.prop_name(id).ok_or(DispError::MEMBER_NOT_FOUND)?
                };
                if flags.is_write() {
                    let v = params.put_value().cloned().unwrap_or(Variant::Empty);
                    st.log.borrow_mut().push(format!("put {name}"));
                    st.props.borrow_mut().insert(name, v);
                    Ok(Variant::Empty)
                } else {
                    st.log.borrow_mut().push(format!("get {name}"));
                    st.props
                        .borrow()
                        .get(&name)
                        .cloned()
                        .ok_or(DispError::MEMBER_NOT_FOUND)
                }
            }
        }
    }
}

/// A control of a toy form: `Value` (the default member) reads and writes
/// the property of the same name.
pub struct ControlHost {
    state: Rc<FormState>,
    name: String,
}

impl Dispatch for ControlHost {
    fn class_name(&self) -> String {
        "Control".into()
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        name.eq_ignore_ascii_case("Value").then_some(DispId::VALUE)
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        (id == DispId::VALUE).then(|| MemberInfo {
            name: "Value".into(),
            id,
            supports: InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
            params: vec![],
        })
    }

    fn invoke(
        &self,
        _table: &ObjectTable,
        _id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        if flags.is_write() {
            let v = params.put_value().cloned().unwrap_or(Variant::Empty);
            self.state
                .log
                .borrow_mut()
                .push(format!("control put {}", self.name));
            self.state.props.borrow_mut().insert(self.name.clone(), v);
            Ok(Variant::Empty)
        } else {
            Ok(self.state.get(&self.name))
        }
    }
}

/// Loads a project with the given modules as an instance and trusts it.
pub fn load(modules: &[(&str, ModuleKind, &str)]) -> Instance {
    let mut p = Project::new();
    for (name, kind, src) in modules {
        p.add_module(name, *kind, src).unwrap();
    }
    let mut inst = vba::Engine::new()
        .host_members(["CustomerName", "Dirty", "Caption"])
        .load(&p)
        .unwrap();
    inst.trust();
    inst
}

pub fn vba_number(e: &Error) -> i32 {
    match e {
        Error::Run(vba::RunError::Vba(r) | vba::RunError::Host(r)) => r.number,
        other => panic!("not a VBA error: {other:?}"),
    }
}

pub fn result_text(r: &CallResult) -> String {
    text(&r.value)
}

pub fn args(v: Vec<Variant>) -> Vec<Arg> {
    v.into_iter().map(Arg::Val).collect()
}

pub fn handle(o: ObjectRef) -> ObjectHandle {
    match o {
        ObjectRef::Handle(h) => h,
        ObjectRef::Nothing => panic!("Nothing"),
    }
}

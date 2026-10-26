//! A toy "form" host object for the embedding tests: a plain `Dispatch`
//! implementation with a property bag and controls. It stands for the
//! object an embedding application passes as the `Me` of a form module.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use vba::automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectHandle, ObjectRef,
    ObjectTable, ParamInfo,
};
use vba::{Arg, CallResult, Error, HostContext, Instance, ModuleKind, Project, Variant};

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
const UPDATE: DispId = DispId(3);
const FIRST_PROP: i32 = 100;

/// The `Me` of a toy form: unknown names are properties of the bag.
pub struct FormHost {
    pub state: Rc<FormState>,
    /// The VBA instance this host object is the `Me` of (set after
    /// `create_object`), so `Update` can raise its events.
    pub me: Cell<Option<ObjectRef>>,
}

impl FormHost {
    pub fn new(state: &Rc<FormState>) -> Rc<FormHost> {
        Rc::new(FormHost {
            state: state.clone(),
            me: Cell::new(None),
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

    fn reentrant(&self) -> bool {
        true
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "controls" => Some(CONTROLS),
            "requery" => Some(REQUERY),
            "update" => Some(UPDATE),
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
            UPDATE => (
                "Update".to_owned(),
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
            UPDATE => {
                // Raise BeforeUpdate on the VBA instance, synchronously,
                // and read Cancel when the handler is done.
                let ctx = HostContext::current(table)
                    .ok_or_else(|| DispError::raise(5, "no session context", "FormHost"))?;
                let me = self.me.get().ok_or_else(|| {
                    DispError::raise(91, "the form has no VBA module", "FormHost")
                })?;
                let out = ctx
                    .raise_event_on(
                        me,
                        "Form",
                        "BeforeUpdate",
                        vec![Arg::by_ref(Variant::Integer(0))],
                    )
                    .map_err(DispError::from)?;
                let cancelled = out.byref[0] != Variant::Integer(0);
                st.log.borrow_mut().push(
                    if cancelled {
                        "update cancelled"
                    } else {
                        "update committed"
                    }
                    .into(),
                );
                Ok(Variant::Boolean(!cancelled))
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

// ---- an application-like host: DoCmd and Application ------------------------

/// A form `DoCmd.OpenForm` opened.
pub struct OpenForm {
    pub name: String,
    pub object: ObjectRef,
    pub state: Rc<FormState>,
    pub host: Rc<FormHost>,
}

#[derive(Default)]
pub struct AppState {
    pub forms: RefCell<Vec<OpenForm>>,
    /// What the host saw, in order.
    pub log: RefCell<Vec<String>>,
}

/// `DoCmd`: `OpenForm FormName, [DataMode], [OpenArgs]` creates a second
/// instance of `Form_<FormName>` and raises its `Open` and `Load` handlers
/// synchronously, while the VBA statement that called it is waiting.
pub struct DoCmd(pub Rc<AppState>);

const OPEN_FORM: DispId = DispId(1);

impl Dispatch for DoCmd {
    fn class_name(&self) -> String {
        "DoCmd".into()
    }
    fn reentrant(&self) -> bool {
        true
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        name.eq_ignore_ascii_case("OpenForm").then_some(OPEN_FORM)
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        (id == OPEN_FORM).then(|| MemberInfo {
            name: "OpenForm".into(),
            id,
            supports: InvokeFlags::METHOD,
            params: vec![
                ParamInfo::required("FormName").by_val(),
                ParamInfo::optional("DataMode").by_val(),
                ParamInfo::optional("OpenArgs").by_val(),
            ],
        })
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        _id: DispId,
        _flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let app = &self.0;
        let info = self.member_info(OPEN_FORM).expect("known");
        let bound = vba::automation::bind(&info.params, params)?;
        let name = text(&bound.slots[0].value);
        let open_args = bound.slots[2].value.clone();
        let ctx = HostContext::current(table)
            .ok_or_else(|| DispError::raise(5, "DoCmd needs a running session", "DoCmd"))?;
        app.log.borrow_mut().push(format!("OpenForm {name} begin"));
        let state = FormState::new(&[
            ("CustomerName", Variant::Null),
            ("Caption", Variant::from(name.as_str())),
            (
                "OpenArgs",
                if bound.slots[2].is_missing() {
                    Variant::Null
                } else {
                    open_args
                },
            ),
        ]);
        let host = FormHost::new(&state);
        let object = ctx
            .create_object(&format!("Form_{name}"), Some(host.clone()))
            .map_err(DispError::from)?;
        host.me.set(Some(object));
        app.forms.borrow_mut().push(OpenForm {
            name: name.clone(),
            object,
            state,
            host,
        });
        let open = ctx
            .raise_event_on(
                object,
                "Form",
                "Open",
                vec![Arg::by_ref(Variant::Integer(0))],
            )
            .map_err(DispError::from)?;
        if open.byref[0] != Variant::Integer(0) {
            let form = app.forms.borrow_mut().pop().expect("just pushed");
            ctx.destroy_object(form.object).map_err(DispError::from)?;
            app.log
                .borrow_mut()
                .push(format!("OpenForm {name} cancelled"));
            return Err(DispError::raise(
                2501,
                "The OpenForm action was canceled.",
                "DoCmd",
            ));
        }
        ctx.raise_event_on(object, "Form", "Load", vec![])
            .map_err(DispError::from)?;
        app.log.borrow_mut().push(format!("OpenForm {name} end"));
        Ok(Variant::Empty)
    }
}

/// `Application.Setting(Name)` returns the VBA global `Config.<Name>`;
/// `Application.Run(Proc, [Arg1])` calls a VBA procedure back.
pub struct Application(pub Rc<AppState>);

impl Dispatch for Application {
    fn class_name(&self) -> String {
        "Application".into()
    }
    fn reentrant(&self) -> bool {
        true
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "setting" => Some(DispId(1)),
            "run" => Some(DispId(2)),
            _ => None,
        }
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let (name, params) = match id.0 {
            1 => ("Setting", vec![ParamInfo::required("Name").by_val()]),
            2 => (
                "Run",
                vec![
                    ParamInfo::required("Procedure").by_val(),
                    ParamInfo::optional("Arg1").by_val(),
                ],
            ),
            _ => return None,
        };
        Some(MemberInfo {
            name: name.into(),
            id,
            supports: InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            params,
        })
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        _flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let ctx = HostContext::current(table)
            .ok_or_else(|| DispError::raise(5, "no session context", "Application"))?;
        let info = self.member_info(id).ok_or(DispError::MEMBER_NOT_FOUND)?;
        let bound = vba::automation::bind(&info.params, params)?;
        match id.0 {
            1 => {
                let name = text(&bound.slots[0].value);
                self.0.log.borrow_mut().push(format!("Setting {name}"));
                ctx.get_global(&format!("Config.{name}"))
                    .map_err(DispError::from)
            }
            _ => {
                let proc = text(&bound.slots[0].value);
                let args = if bound.slots[1].is_missing() {
                    vec![]
                } else {
                    vec![Arg::val(bound.slots[1].value.clone())]
                };
                let r = ctx
                    .call(vba::CallTarget::Project, &proc, args)
                    .map_err(DispError::from)?;
                Ok(r.value)
            }
        }
    }
}

/// An engine with `DoCmd` and `Application` globals sharing one `AppState`.
pub fn app_engine(app: &Rc<AppState>) -> vba::Engine {
    let (a, b) = (app.clone(), app.clone());
    vba::Engine::new()
        .dispatch_global("DoCmd", move || DoCmd(a.clone()))
        .dispatch_global("Application", move || Application(b.clone()))
}

pub fn load_app(engine: &vba::Engine, modules: &[(&str, ModuleKind, &str)]) -> Instance {
    let mut p = Project::new();
    for (name, kind, src) in modules {
        p.add_module(name, *kind, src).unwrap();
    }
    let mut inst = engine.load(&p).unwrap();
    inst.trust();
    inst
}

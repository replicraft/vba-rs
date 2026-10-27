//! The JSON form of a description.
//!
//! The form is versioned (`"format": "vba-host-library"`, `"version": 1`)
//! and strict: the reader rejects unknown keys, wrong types and an
//! unsupported version, naming the path of the offending value. It is
//! documented in `docs/architecture/host-model.md`.
//!
//! Reading does not validate the description; call
//! [`HostLibrary::check`](crate::HostLibrary::check) on the result.

use serde_json::{Map, Number, Value, json};

use crate::model::{
    Access, ConstValue, HostClass, HostConst, HostEnum, HostEvent, HostGlobal, HostLibrary,
    HostMember, HostParam, MemberKind, TypeRef,
};

/// The value of the `format` key.
pub const FORMAT_NAME: &str = "vba-host-library";
/// The value of the `version` key this crate writes and reads.
pub const FORMAT_VERSION: u32 = 1;

/// A description that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JsonError {
    /// The text is not JSON.
    #[error("not valid JSON: {0}")]
    Syntax(String),
    /// The JSON does not have the shape of the format.
    #[error("{path}: {message}")]
    Shape {
        /// The location, like `classes[1].members[0].params[2].type`.
        path: String,
        /// What is wrong.
        message: String,
    },
    /// The `version` is not one this crate reads.
    #[error("unsupported format version {0}")]
    Version(u64),
}

fn shape<T>(path: &str, message: impl Into<String>) -> Result<T, JsonError> {
    Err(JsonError::Shape {
        path: path.to_owned(),
        message: message.into(),
    })
}

// ---- writing ---------------------------------------------------------------

fn value_json(v: &ConstValue) -> Value {
    match v {
        ConstValue::Empty => json!({"type": "empty"}),
        ConstValue::Null => json!({"type": "null"}),
        ConstValue::Boolean(b) => json!({"type": "boolean", "value": b}),
        ConstValue::Integer(i) => json!({"type": "integer", "value": i}),
        ConstValue::Long(l) => json!({"type": "long", "value": l}),
        ConstValue::Double(d) => {
            let n = Number::from_f64(*d).map_or(Value::Null, Value::Number);
            json!({"type": "double", "value": n})
        }
        ConstValue::String(s) => json!({"type": "string", "value": s}),
    }
}

fn put_doc(m: &mut Map<String, Value>, doc: &str) {
    if !doc.is_empty() {
        m.insert("doc".into(), doc.into());
    }
}

fn attrs_json(attrs: &[(String, String)]) -> Value {
    Value::Array(
        attrs
            .iter()
            .map(|(n, v)| json!({"name": n, "value": v}))
            .collect(),
    )
}

fn param_json(p: &HostParam) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), p.name.clone().into());
    m.insert("type".into(), p.ty.to_string().into());
    if p.optional {
        m.insert("optional".into(), true.into());
    }
    if let Some(d) = &p.default {
        m.insert("default".into(), value_json(d));
    }
    if p.by_ref {
        m.insert("byRef".into(), true.into());
    }
    if p.param_array {
        m.insert("paramArray".into(), true.into());
    }
    put_doc(&mut m, &p.doc);
    Value::Object(m)
}

fn params_json(params: &[HostParam]) -> Value {
    Value::Array(params.iter().map(param_json).collect())
}

fn member_json(x: &HostMember) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), x.name.clone().into());
    m.insert(
        "kind".into(),
        match x.kind {
            MemberKind::Method => "method",
            MemberKind::Property => "property",
        }
        .into(),
    );
    if let Some(t) = &x.ty {
        m.insert("type".into(), t.to_string().into());
    }
    if x.kind == MemberKind::Property {
        m.insert(
            "access".into(),
            match x.access {
                Access::ReadOnly => "read",
                Access::ReadWrite => "readwrite",
                Access::WriteOnly => "write",
            }
            .into(),
        );
    }
    if !x.params.is_empty() {
        m.insert("params".into(), params_json(&x.params));
    }
    if x.default_member {
        m.insert("default".into(), true.into());
    }
    if x.hidden {
        m.insert("hidden".into(), true.into());
    }
    if let Some(id) = x.dispid {
        m.insert("dispid".into(), id.into());
    }
    put_doc(&mut m, &x.doc);
    if !x.attributes.is_empty() {
        m.insert("attributes".into(), attrs_json(&x.attributes));
    }
    Value::Object(m)
}

fn event_json(e: &HostEvent) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), e.name.clone().into());
    if !e.params.is_empty() {
        m.insert("params".into(), params_json(&e.params));
    }
    put_doc(&mut m, &e.doc);
    Value::Object(m)
}

fn class_json(c: &HostClass) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), c.name.clone().into());
    put_doc(&mut m, &c.doc);
    if c.creatable {
        m.insert("creatable".into(), true.into());
    }
    if let Some(t) = &c.enumerable {
        m.insert("enumerable".into(), t.to_string().into());
    }
    if !c.attributes.is_empty() {
        m.insert("attributes".into(), attrs_json(&c.attributes));
    }
    m.insert(
        "members".into(),
        Value::Array(c.members.iter().map(member_json).collect()),
    );
    if !c.events.is_empty() {
        m.insert(
            "events".into(),
            Value::Array(c.events.iter().map(event_json).collect()),
        );
    }
    Value::Object(m)
}

fn enum_json(e: &HostEnum) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), e.name.clone().into());
    put_doc(&mut m, &e.doc);
    let members = e
        .members
        .iter()
        .map(|(n, v, d)| {
            let mut mm = Map::new();
            mm.insert("name".into(), n.clone().into());
            mm.insert("value".into(), (*v).into());
            put_doc(&mut mm, d);
            Value::Object(mm)
        })
        .collect();
    m.insert("members".into(), Value::Array(members));
    Value::Object(m)
}

fn const_json(c: &HostConst) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), c.name.clone().into());
    m.insert("value".into(), value_json(&c.value));
    put_doc(&mut m, &c.doc);
    Value::Object(m)
}

fn global_json(g: &HostGlobal) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), g.name.clone().into());
    m.insert("type".into(), g.ty.to_string().into());
    put_doc(&mut m, &g.doc);
    Value::Object(m)
}

impl HostLibrary {
    /// The JSON form.
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("format".into(), FORMAT_NAME.into());
        m.insert("version".into(), FORMAT_VERSION.into());
        m.insert("name".into(), self.name.clone().into());
        put_doc(&mut m, &self.doc);
        let list = |items: Vec<Value>| Value::Array(items);
        m.insert(
            "classes".into(),
            list(self.classes.iter().map(class_json).collect()),
        );
        m.insert(
            "enums".into(),
            list(self.enums.iter().map(enum_json).collect()),
        );
        m.insert(
            "constants".into(),
            list(self.consts.iter().map(const_json).collect()),
        );
        m.insert(
            "functions".into(),
            list(self.functions.iter().map(member_json).collect()),
        );
        m.insert(
            "globals".into(),
            list(self.globals.iter().map(global_json).collect()),
        );
        Value::Object(m)
    }

    /// The JSON form as indented text.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(&self.to_json()).unwrap_or_default()
    }

    /// Reads the JSON form from text.
    pub fn from_json_str(text: &str) -> Result<HostLibrary, JsonError> {
        let v: Value = serde_json::from_str(text).map_err(|e| JsonError::Syntax(e.to_string()))?;
        HostLibrary::from_json(&v)
    }

    /// Reads the JSON form. The result is not validated.
    pub fn from_json(v: &Value) -> Result<HostLibrary, JsonError> {
        let mut f = Fields::new(v, "$")?;
        if f.str("format")? != FORMAT_NAME {
            return shape("$.format", format!("expected \"{FORMAT_NAME}\""));
        }
        let version = f.u64("version")?;
        if version != u64::from(FORMAT_VERSION) {
            return Err(JsonError::Version(version));
        }
        let mut lib = HostLibrary::new(f.str("name")?);
        lib.doc = f.opt_str("doc")?.unwrap_or_default();
        for (i, c) in f.array("classes")?.iter().enumerate() {
            lib.classes.push(read_class(c, &format!("$.classes[{i}]"))?);
        }
        for (i, e) in f.array("enums")?.iter().enumerate() {
            lib.enums.push(read_enum(e, &format!("$.enums[{i}]"))?);
        }
        for (i, c) in f.array("constants")?.iter().enumerate() {
            let p = format!("$.constants[{i}]");
            let mut cf = Fields::new(c, &p)?;
            let name = cf.str("name")?;
            let value = read_value(cf.get("value")?, &format!("{p}.value"))?;
            let mut k = HostConst::new(name, value);
            k.doc = cf.opt_str("doc")?.unwrap_or_default();
            cf.finish()?;
            lib.consts.push(k);
        }
        for (i, m) in f.array("functions")?.iter().enumerate() {
            lib.functions
                .push(read_member(m, &format!("$.functions[{i}]"))?);
        }
        for (i, g) in f.array("globals")?.iter().enumerate() {
            let p = format!("$.globals[{i}]");
            let mut gf = Fields::new(g, &p)?;
            let mut x = HostGlobal::new(gf.str("name")?, TypeRef::parse(&gf.str("type")?));
            x.doc = gf.opt_str("doc")?.unwrap_or_default();
            gf.finish()?;
            lib.globals.push(x);
        }
        f.finish()?;
        Ok(lib)
    }
}

// ---- reading ---------------------------------------------------------------

struct Fields<'a> {
    map: &'a Map<String, Value>,
    path: String,
    used: Vec<&'static str>,
}

impl<'a> Fields<'a> {
    fn new(v: &'a Value, path: &str) -> Result<Fields<'a>, JsonError> {
        match v {
            Value::Object(map) => Ok(Fields {
                map,
                path: path.to_owned(),
                used: Vec::new(),
            }),
            _ => shape(path, "expected an object"),
        }
    }

    fn at(&self, key: &str) -> String {
        format!("{}.{key}", self.path)
    }

    fn get(&mut self, key: &'static str) -> Result<&'a Value, JsonError> {
        self.used.push(key);
        match self.map.get(key) {
            Some(v) => Ok(v),
            None => shape(&self.path, format!("missing `{key}`")),
        }
    }

    fn opt(&mut self, key: &'static str) -> Option<&'a Value> {
        self.used.push(key);
        self.map.get(key)
    }

    fn str(&mut self, key: &'static str) -> Result<String, JsonError> {
        match self.get(key)? {
            Value::String(s) => Ok(s.clone()),
            _ => shape(&self.at(key), "expected a string"),
        }
    }

    fn opt_str(&mut self, key: &'static str) -> Result<Option<String>, JsonError> {
        match self.opt(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => shape(&self.at(key), "expected a string"),
        }
    }

    fn u64(&mut self, key: &'static str) -> Result<u64, JsonError> {
        match self.get(key)?.as_u64() {
            Some(n) => Ok(n),
            None => shape(&self.at(key), "expected a non-negative integer"),
        }
    }

    fn flag(&mut self, key: &'static str) -> Result<bool, JsonError> {
        match self.opt(key) {
            None => Ok(false),
            Some(Value::Bool(b)) => Ok(*b),
            Some(_) => shape(&self.at(key), "expected true or false"),
        }
    }

    fn array(&mut self, key: &'static str) -> Result<&'a [Value], JsonError> {
        match self.get(key)? {
            Value::Array(a) => Ok(a),
            _ => shape(&self.at(key), "expected an array"),
        }
    }

    fn opt_array(&mut self, key: &'static str) -> Result<&'a [Value], JsonError> {
        match self.opt(key) {
            None => Ok(&[]),
            Some(Value::Array(a)) => Ok(a),
            Some(_) => shape(&self.at(key), "expected an array"),
        }
    }

    fn finish(self) -> Result<(), JsonError> {
        for k in self.map.keys() {
            if !self.used.contains(&k.as_str()) {
                return shape(&format!("{}.{k}", self.path), "unknown key");
            }
        }
        Ok(())
    }
}

fn read_value(v: &Value, path: &str) -> Result<ConstValue, JsonError> {
    let mut f = Fields::new(v, path)?;
    let ty = f.str("type")?;
    let at = f.at("value");
    let out = match ty.as_str() {
        "empty" => ConstValue::Empty,
        "null" => ConstValue::Null,
        "boolean" => match f.get("value")? {
            Value::Bool(b) => ConstValue::Boolean(*b),
            _ => return shape(&at, "expected true or false"),
        },
        "integer" => match f.get("value")?.as_i64().map(i16::try_from) {
            Some(Ok(n)) => ConstValue::Integer(n),
            _ => return shape(&at, "expected an integer in the range of Integer"),
        },
        "long" => match f.get("value")?.as_i64().map(i32::try_from) {
            Some(Ok(n)) => ConstValue::Long(n),
            _ => return shape(&at, "expected an integer in the range of Long"),
        },
        "double" => match f.get("value")?.as_f64() {
            Some(n) => ConstValue::Double(n),
            None => return shape(&at, "expected a number"),
        },
        "string" => ConstValue::String(f.str("value")?),
        other => {
            return shape(
                &f.at("type"),
                format!(
                    "unknown value type `{other}` (empty, null, boolean, integer, long, double, string)"
                ),
            );
        }
    };
    f.finish()?;
    Ok(out)
}

fn read_attrs(f: &mut Fields<'_>, path: &str) -> Result<Vec<(String, String)>, JsonError> {
    let mut out = Vec::new();
    for (i, a) in f.opt_array("attributes")?.iter().enumerate() {
        let p = format!("{path}.attributes[{i}]");
        let mut af = Fields::new(a, &p)?;
        out.push((af.str("name")?, af.str("value")?));
        af.finish()?;
    }
    Ok(out)
}

fn read_params(f: &mut Fields<'_>, path: &str) -> Result<Vec<HostParam>, JsonError> {
    let mut out = Vec::new();
    for (i, p) in f.opt_array("params")?.iter().enumerate() {
        let pp = format!("{path}.params[{i}]");
        let mut pf = Fields::new(p, &pp)?;
        let mut x = HostParam::new(pf.str("name")?, TypeRef::parse(&pf.str("type")?));
        x.optional = pf.flag("optional")?;
        if let Some(d) = pf.opt("default") {
            x.default = Some(read_value(d, &format!("{pp}.default"))?);
        }
        x.by_ref = pf.flag("byRef")?;
        x.param_array = pf.flag("paramArray")?;
        x.doc = pf.opt_str("doc")?.unwrap_or_default();
        pf.finish()?;
        out.push(x);
    }
    Ok(out)
}

fn read_member(v: &Value, path: &str) -> Result<HostMember, JsonError> {
    let mut f = Fields::new(v, path)?;
    let name = f.str("name")?;
    let kind = match f.str("kind")?.as_str() {
        "method" => MemberKind::Method,
        "property" => MemberKind::Property,
        _ => return shape(&f.at("kind"), "expected \"method\" or \"property\""),
    };
    let ty = f.opt_str("type")?.map(|t| TypeRef::parse(&t));
    let mut m = match (kind, ty) {
        (MemberKind::Method, None) => HostMember::sub(name),
        (MemberKind::Method, Some(t)) => HostMember::function(name, t),
        (MemberKind::Property, Some(t)) => HostMember::property(name, t),
        (MemberKind::Property, None) => {
            // Kept so that validation reports `PropertyWithoutType`.
            let mut m = HostMember::sub(name);
            m.kind = MemberKind::Property;
            m
        }
    };
    if let Some(a) = f.opt_str("access")? {
        m.access = match a.as_str() {
            "read" => Access::ReadOnly,
            "readwrite" => Access::ReadWrite,
            "write" => Access::WriteOnly,
            _ => {
                return shape(
                    &f.at("access"),
                    "expected \"read\", \"readwrite\" or \"write\"",
                );
            }
        };
    }
    m.params = read_params(&mut f, path)?;
    m.default_member = f.flag("default")?;
    m.hidden = f.flag("hidden")?;
    if let Some(id) = f.opt("dispid") {
        match id.as_i64().map(i32::try_from) {
            Some(Ok(n)) => m.dispid = Some(n),
            _ => return shape(&f.at("dispid"), "expected a 32-bit integer"),
        }
    }
    m.doc = f.opt_str("doc")?.unwrap_or_default();
    m.attributes = read_attrs(&mut f, path)?;
    f.finish()?;
    Ok(m)
}

fn read_class(v: &Value, path: &str) -> Result<HostClass, JsonError> {
    let mut f = Fields::new(v, path)?;
    let mut c = HostClass::new(f.str("name")?);
    c.doc = f.opt_str("doc")?.unwrap_or_default();
    c.creatable = f.flag("creatable")?;
    c.enumerable = f.opt_str("enumerable")?.map(|t| TypeRef::parse(&t));
    c.attributes = read_attrs(&mut f, path)?;
    for (i, m) in f.array("members")?.iter().enumerate() {
        c.members
            .push(read_member(m, &format!("{path}.members[{i}]"))?);
    }
    for (i, e) in f.opt_array("events")?.iter().enumerate() {
        let ep = format!("{path}.events[{i}]");
        let mut ef = Fields::new(e, &ep)?;
        let mut ev = HostEvent::new(ef.str("name")?);
        ev.params = read_params(&mut ef, &ep)?;
        ev.doc = ef.opt_str("doc")?.unwrap_or_default();
        ef.finish()?;
        c.events.push(ev);
    }
    f.finish()?;
    Ok(c)
}

fn read_enum(v: &Value, path: &str) -> Result<HostEnum, JsonError> {
    let mut f = Fields::new(v, path)?;
    let mut e = HostEnum::new(f.str("name")?);
    e.doc = f.opt_str("doc")?.unwrap_or_default();
    for (i, m) in f.array("members")?.iter().enumerate() {
        let mp = format!("{path}.members[{i}]");
        let mut mf = Fields::new(m, &mp)?;
        let name = mf.str("name")?;
        let value = match mf.get("value")?.as_i64().map(i32::try_from) {
            Some(Ok(n)) => n,
            _ => return shape(&mf.at("value"), "expected an integer in the range of Long"),
        };
        let doc = mf.opt_str("doc")?.unwrap_or_default();
        mf.finish()?;
        e.members.push((name, value, doc));
    }
    f.finish()?;
    Ok(e)
}

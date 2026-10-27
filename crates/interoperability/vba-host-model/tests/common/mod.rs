//! A small database-like object model, described once and implemented
//! with in-memory data, shared by the tests.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vba_automation::{DispError, ObjectTable};
use vba_host_model::{
    CheckedLibrary, ConstValue, Handlers, HostClass, HostConst, HostEnum, HostEvent, HostGlobal,
    HostLibrary, HostMember, HostObject, HostParam, HostRuntime, TypeRef,
};
use vba_value::{ObjectRef, Variant};

pub const LIB: &str = "DbLib";

pub fn description() -> HostLibrary {
    HostLibrary::new(LIB)
        .doc("A small database object model.")
        .enumeration(
            HostEnum::new("RecordsetType")
                .doc("How a recordset is opened.")
                .documented("dbOpenTable", 1, "A table.")
                .member("dbOpenDynaset", 2)
                .member("dbOpenSnapshot", 4),
        )
        .constant(HostConst::new("AppVersion", "1.0").doc("The application version."))
        .class(
            HostClass::new("Field")
                .doc("One column of the current row.")
                .member(
                    HostMember::property("Name", "String")
                        .read_only()
                        .doc("The name."),
                )
                .member(
                    HostMember::property("Value", "Variant")
                        .default_member()
                        .doc("The value."),
                ),
        )
        .class(
            HostClass::new("Fields")
                .doc("The columns of a recordset.")
                .enumerable("Field")
                .member(
                    HostMember::function("Item", "Field")
                        .param(HostParam::new("Index", "Variant"))
                        .default_member(),
                )
                .member(HostMember::property("Count", "Long").read_only()),
        )
        .class(
            HostClass::new("Recordset")
                .doc("Rows of a table.")
                .member(HostMember::property("Fields", "Fields").read_only())
                .member(HostMember::property("EOF", "Boolean").read_only())
                .member(HostMember::property("RecordCount", "Long").read_only())
                .member(HostMember::property("Name", "String"))
                .member(HostMember::sub("MoveNext").doc("Moves to the next row."))
                .member(HostMember::sub("Close"))
                .member(
                    HostMember::function("FindFirst", "Boolean")
                        .param(HostParam::new("Criteria", "String")),
                )
                .member(
                    HostMember::function("GetRows", "Variant")
                        .param(HostParam::new("NumRows", "Long").default_value(1)),
                )
                .member(
                    HostMember::function("Lookup", "Long")
                        .param(HostParam::new("Key", "String"))
                        .param(HostParam::new("Found", "Boolean").by_ref()),
                )
                .member(
                    HostMember::function("Join", "String")
                        .param(HostParam::new("Separator", "String"))
                        .param(HostParam::new("Parts", "Variant").param_array()),
                )
                .event(
                    HostEvent::new("Updated").param(HostParam::new("Cancel", "Integer").by_ref()),
                ),
        )
        .class(
            HostClass::new("Database")
                .creatable()
                .member(HostMember::property("Name", "String").read_only())
                .member(HostMember::sub("Attach").param(HostParam::new("Source", "Recordset")))
                .member(
                    HostMember::function("OpenRecordset", "Recordset")
                        .param(HostParam::new("Name", "String"))
                        .param(HostParam::new("Kind", "RecordsetType").default_value(2))
                        .doc("Opens a recordset on a table."),
                ),
        )
        .class(
            HostClass::new("Application")
                .member(HostMember::property("Name", "String").read_only())
                .member(HostMember::function("CurrentDb", "Database")),
        )
        .global(HostGlobal::new("Application", "Application").doc("The application."))
        .function(
            HostMember::function("Nz", "Variant")
                .param(HostParam::new("Value", "Variant"))
                .param(HostParam::new("ValueIfNull", "Variant").optional())
                .doc("Replaces Null."),
        )
        .function(
            HostMember::function("DLookup", "Variant")
                .param(HostParam::new("Expr", "String"))
                .param(HostParam::new("Domain", "String"))
                .param(HostParam::new("Criteria", "Variant").optional()),
        )
}

pub fn library() -> CheckedLibrary {
    description().check().expect("the sample is valid")
}

fn handle(table: &ObjectTable, o: HostObject) -> Variant {
    Variant::Object(ObjectRef::Handle(table.insert_obj(o)))
}

pub struct Data {
    pub rows: Vec<(String, i32)>,
    pub pos: Cell<usize>,
    pub name: RefCell<String>,
    pub cells: Vec<(Rc<RefCell<Variant>>, Rc<RefCell<Variant>>)>,
}

pub fn field(lib: &CheckedLibrary, name: &'static str, cell: Rc<RefCell<Variant>>) -> HostObject {
    let c2 = cell.clone();
    HostObject::new(
        lib,
        "Field",
        Handlers::new(cell)
            .get("Name", move |_, _| Ok(Variant::from(name)))
            .get("Value", |s, _| Ok(s.borrow().clone()))
            .let_("Value", |s, cx| {
                *s.borrow_mut() = cx.args.value()?.clone();
                Ok(())
            })
            .set("Value", move |_, cx| {
                *c2.borrow_mut() = cx.args.value()?.clone();
                Ok(())
            }),
    )
    .expect("Field binds")
}

pub fn fields(
    lib: &CheckedLibrary,
    cells: Vec<(&'static str, Rc<RefCell<Variant>>)>,
) -> HostObject {
    let l1 = lib.clone();
    let l2 = lib.clone();
    HostObject::new(
        lib,
        "Fields",
        Handlers::new(cells)
            .get("Count", |s, _| Ok(Variant::Long(s.len() as i32)))
            .method("Item", move |s, cx| {
                let idx = cx.args.get(0).cloned().unwrap_or(Variant::Empty);
                let found = match &idx {
                    Variant::String(_) => {
                        let want = cx.args.string(0)?;
                        s.iter().find(|(n, _)| n.eq_ignore_ascii_case(&want))
                    }
                    other => {
                        let i = vba_value::convert::to_long(other)?;
                        s.get(usize::try_from(i).map_err(|_| DispError::MEMBER_NOT_FOUND)?)
                    }
                };
                match found {
                    Some((n, c)) => Ok(handle(cx.table, field(&l1, n, c.clone()))),
                    None => Err(DispError::raise(
                        3265,
                        "Item not found in this collection.",
                        "DbLib",
                    )),
                }
            })
            .enumerate(move |s, table| {
                s.iter()
                    .map(|(n, c)| handle(table, field(&l2, n, c.clone())))
                    .collect()
            }),
    )
    .expect("Fields binds")
}

pub fn recordset(lib: &CheckedLibrary, data: Data) -> HostObject {
    let l1 = lib.clone();
    HostObject::new(
        lib,
        "Recordset",
        Handlers::new(data)
            .get("Fields", move |s, cx| {
                let cells = s
                    .cells
                    .iter()
                    .zip(["Name", "Qty"])
                    .map(|((_, c), n)| (n, c.clone()))
                    .collect();
                Ok(handle(cx.table, fields(&l1, cells)))
            })
            .get("EOF", |s, _| {
                Ok(Variant::Boolean(s.pos.get() >= s.rows.len()))
            })
            .get("RecordCount", |s, _| Ok(Variant::Long(s.rows.len() as i32)))
            .get("Name", |s, _| Ok(Variant::from(s.name.borrow().as_str())))
            .let_("Name", |s, cx| {
                *s.name.borrow_mut() = text(cx.args.value()?);
                Ok(())
            })
            .method("MoveNext", |s, cx| {
                // Handlers may cancel the move through the ByRef argument.
                let mut args = [vba_automation::Arg::by_ref(0i16)];
                cx.raise_event("Updated", &mut args)?;
                if args[0].value == Variant::Integer(0) {
                    s.pos.set(s.pos.get() + 1);
                }
                Ok(Variant::Empty)
            })
            .method("Close", |_, _| Ok(Variant::Empty))
            .method("FindFirst", |s, cx| {
                let c = cx.args.string(0)?;
                Ok(Variant::Boolean(s.rows.iter().any(|(n, _)| *n == c)))
            })
            .method("GetRows", |s, cx| {
                let n = cx.args.long(0)?;
                Ok(Variant::Long(n.min(s.rows.len() as i32)))
            })
            .method("Lookup", |s, cx| {
                let key = cx.args.string(0)?;
                let hit = s.rows.iter().find(|(n, _)| *n == key);
                cx.args.set(1, hit.is_some());
                Ok(Variant::Long(hit.map_or(0, |(_, q)| *q)))
            })
            .method("Join", |_, cx| {
                let sep = cx.args.string(0)?;
                let parts: Vec<String> = cx.args.rest().map(text).collect();
                Ok(Variant::from(parts.join(&sep).as_str()))
            })
            .reentrant()
            .on_terminate(|_, _| {})
            .holds(|_| Vec::new()),
    )
    .expect("Recordset binds")
}

pub fn open_recordset(lib: &CheckedLibrary, name: &str) -> HostObject {
    let rows = vec![("apple".to_owned(), 3), ("pear".to_owned(), 5)];
    let cells = vec![
        (
            Rc::new(RefCell::new(Variant::from("apple"))),
            Rc::new(RefCell::new(Variant::from("apple"))),
        ),
        (
            Rc::new(RefCell::new(Variant::Long(3))),
            Rc::new(RefCell::new(Variant::Long(3))),
        ),
    ];
    recordset(
        lib,
        Data {
            rows,
            pos: Cell::new(0),
            name: RefCell::new(name.to_owned()),
            cells,
        },
    )
}

pub fn database(lib: &CheckedLibrary) -> HostObject {
    let l = lib.clone();
    HostObject::new(
        lib,
        "Database",
        Handlers::new(())
            .get("Name", |_, _| Ok(Variant::from("main.db")))
            .method("Attach", |_, _| Ok(Variant::Empty))
            .method("OpenRecordset", move |_, cx| {
                let name = cx.args.string(0)?;
                Ok(handle(cx.table, open_recordset(&l, &name)))
            }),
    )
    .expect("Database binds")
}

pub fn application(lib: &CheckedLibrary) -> HostObject {
    let l = lib.clone();
    HostObject::new(
        lib,
        "Application",
        Handlers::new(())
            .get("Name", |_, _| Ok(Variant::from("TestApp")))
            .method("CurrentDb", move |_, cx| Ok(handle(cx.table, database(&l)))),
    )
    .expect("Application binds")
}

pub fn runtime() -> HostRuntime {
    let lib = library();
    let (l1, l2) = (lib.clone(), lib.clone());
    HostRuntime::new(&lib)
        .global("Application", move |_| Ok(application(&l1)))
        .creatable("Database", move |_| Ok(database(&l2)))
        .function("Nz", |cx| {
            let v = cx.args.get(0).cloned().unwrap_or(Variant::Null);
            if matches!(v, Variant::Null) {
                Ok(cx.args.get(1).cloned().unwrap_or_else(|| Variant::from("")))
            } else {
                Ok(v)
            }
        })
        .function("DLookup", |cx| {
            let expr = cx.args.string(0)?;
            Ok(Variant::from(format!("{expr}!").as_str()))
        })
        .build()
        .expect("the sample runtime is complete")
}

fn text(v: &Variant) -> String {
    vba_value::convert::to_vba_string(v)
        .map(|s| s.to_string_lossy())
        .unwrap_or_default()
}

pub fn type_ref(s: &str) -> TypeRef {
    TypeRef::parse(s)
}

pub fn default_long(v: i32) -> ConstValue {
    ConstValue::Long(v)
}

/// The diagnostics (`CODE@line`, errors and warnings only) of one standard
/// module analysed against the sample library.
pub fn diagnose(body: &str) -> Vec<String> {
    use vba_diagnostics::Severity;
    use vba_sema::{Options, SourceModule, analyze_sources};
    let profile = *vba_profile::find_profile("vba7.1-excel-win64").expect("profile");
    let src = format!("Option Explicit\n{body}");
    let modules = [SourceModule {
        name: "Module1".into(),
        kind: vba_project::ModuleKind::Standard,
        source: src.into_bytes(),
    }];
    let a = analyze_sources(&modules, &profile, &library(), &Options::default());
    a.diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error || d.severity == Severity::Warning)
        .map(|d| {
            let f = a.sources.get(d.file).expect("file");
            format!("{}@{}", d.code.0, f.line_col(d.span.start).line + 1)
        })
        .collect()
}

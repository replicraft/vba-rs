//! One description of an embedding application's object model, used for
//! analysis, execution and tooling.
//!
//! ```text
//! cargo run -p vba-rs --example host_library
//! ```
//!
//! The application describes `Database`, `Recordset` and friends once. The
//! same description then
//!
//! 1. types the program (`Dim rs As Recordset`, wrong calls are diagnosed),
//! 2. runs it (the closures below are bound to the described members, so
//!    late-bound and early-bound calls convert arguments identically),
//! 3. produces the JSON a language server reads (`vba.hostLibraries`), and
//! 4. supplies the documentation text that completion and hover show.
//!
//! See `docs/architecture/host-model.md`.

use std::cell::{Cell, RefCell};

use vba::host_model::{
    Handlers, HostClass, HostEnum, HostGlobal, HostLibrary, HostMember, HostObject, HostParam,
    HostRuntime,
};
use vba::{Capability, Engine, Project, Variant};
use vba_sema::{LibraryType, TypeProvider};

/// The description: no behavior, only what the object model looks like.
fn describe() -> HostLibrary {
    HostLibrary::new("Shop")
        .doc("A tiny order database.")
        .enumeration(
            HostEnum::new("Ordering")
                .documented("shopByName", 1, "Sort by customer name.")
                .documented("shopByTotal", 2, "Sort by total."),
        )
        .class(
            HostClass::new("Orders")
                .doc("The open orders.")
                .member(HostMember::property("Count", "Long").read_only())
                .member(
                    HostMember::function("Total", "Currency")
                        .param(HostParam::new("Customer", "String"))
                        .param(HostParam::new("Order", "Ordering").default_value(1))
                        .doc("The sum of the orders of one customer."),
                )
                .member(
                    HostMember::sub("Add")
                        .param(HostParam::new("Customer", "String"))
                        .param(HostParam::new("Amount", "Currency"))
                        .doc("Adds an order."),
                )
                .member(
                    HostMember::function("Find", "Boolean")
                        .param(HostParam::new("Customer", "String"))
                        .param(HostParam::new("Amount", "Currency").by_ref())
                        .doc("Looks an order up; Amount receives its amount."),
                ),
        )
        .global(HostGlobal::new("Orders", "Orders").doc("The open orders."))
        .function(
            HostMember::function("Nz", "Variant")
                .param(HostParam::new("Value", "Variant"))
                .param(HostParam::new("IfNull", "Variant").optional())
                .doc("Replaces Null."),
        )
}

/// The behavior: closures bound to the described members.
fn bind(lib: &vba::host_model::CheckedLibrary) -> HostRuntime {
    struct State {
        orders: RefCell<Vec<(String, f64)>>,
        calls: Cell<u32>,
    }
    let l = lib.clone();
    HostRuntime::new(lib)
        .global("Orders", move |_| {
            let state = State {
                orders: RefCell::new(vec![("ada".into(), 12.5)]),
                calls: Cell::new(0),
            };
            let handlers = Handlers::new(state)
                .get("Count", |s, _| Ok(Variant::Long(s.orders.borrow().len() as i32)))
                .method("Total", |s, cx| {
                    s.calls.set(s.calls.get() + 1);
                    let who = cx.args.string(0)?;
                    let sum: f64 = s
                        .orders
                        .borrow()
                        .iter()
                        .filter(|(c, _)| *c == who)
                        .map(|(_, a)| a)
                        .sum();
                    Ok(Variant::Double(sum))
                })
                .method("Add", |s, cx| {
                    let who = cx.args.string(0)?;
                    let amount = cx.args.double(1)?;
                    s.orders.borrow_mut().push((who, amount));
                    Ok(Variant::Empty)
                })
                .method("Find", |s, cx| {
                    let who = cx.args.string(0)?;
                    let hit = s.orders.borrow().iter().find(|(c, _)| *c == who).map(|o| o.1);
                    if let Some(amount) = hit {
                        cx.args.set(1, Variant::Double(amount));
                    }
                    Ok(Variant::Boolean(hit.is_some()))
                });
            Ok(HostObject::new(&l, "Orders", handlers)?)
        })
        .function("Nz", |cx| {
            let v = cx.args.get(0).cloned().unwrap_or(Variant::Null);
            Ok(match v {
                Variant::Null => cx.args.get(1).cloned().unwrap_or(Variant::from("")),
                other => other,
            })
        })
        .build()
        .expect("every described member has an implementation")
}

fn main() {
    // 1. Validate the description once.
    let lib = describe().check().expect("a valid description");
    let runtime = bind(&lib);
    let engine = Engine::new().host_library(runtime);

    // 2. Analysis: precise diagnostics from the description.
    let wrong = Project::from_source(
        "Module1",
        "Public Sub Main()\n  Orders.Add \"ada\"\n  Orders.Remove 1\n  Dim x As Ordr\nEnd Sub\n",
    );
    println!("diagnostics for a wrong program:");
    for line in engine.analyze(&wrong).rendered() {
        println!("{line}");
    }

    // 3. Running: the same description binds the calls.
    let program = Project::from_source(
        "Module1",
        "Public Sub Main()\n\
         \x20 Dim amount As Currency\n\
         \x20 Orders.Add \"grace\", 7.25\n\
         \x20 Debug.Print Orders.Count, Orders.Total(\"grace\", shopByTotal)\n\
         \x20 If Orders.Find(\"ada\", amount) Then Debug.Print \"ada\"; amount\n\
         \x20 Debug.Print Nz(Null, \"none\")\n\
         End Sub\n",
    );
    let engine = engine.grant(Capability::ComCreate);
    let out = engine.run(&program, "Main", vec![]).expect("runs");
    println!("\noutput of a correct program:\n{}", out.output);

    // 4. Tooling data, from the same description.
    let ty = LibraryType {
        library: "Shop".into(),
        name: "Orders".into(),
    };
    println!("members of Shop.Orders:");
    for m in lib.interface_members(&ty).unwrap_or_default() {
        let doc = lib.documentation(Some(&ty), Some(&m.name)).unwrap_or_default();
        println!("  {} - {doc}", m.name);
    }
    println!("\nthe JSON a language server reads (vba.hostLibraries):");
    let json = describe().to_json_string();
    println!("{} bytes, first lines:", json.len());
    for line in json.lines().take(6) {
        println!("  {line}");
    }
}

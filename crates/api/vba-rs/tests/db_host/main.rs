//! A stand-in "database application" embedding vba-rs through the public
//! facade only: recordsets, forms and reports with controls, `DoCmd`,
//! `TempVars`, domain functions, constants and events. Expected values are
//! reasoned from VBA semantics; behavior that was not compared with a real
//! host is marked unverified. Nothing here claims compatibility with a
//! real product.

#![allow(missing_docs)]

mod collections;
mod data;
mod events;
mod forms;
mod host;
mod more;
mod names;
mod robust;
mod session;

use std::rc::Rc;

use host::*;
use vba::{Error, ModuleKind, Variant};

pub fn orders_db() -> Rc<Db> {
    let db = Db::new();
    db.add_table(
        "Orders",
        &["ID", "Customer", "Qty"],
        vec![
            vec![Variant::Long(1), Variant::from("Ann"), Variant::Long(10)],
            vec![Variant::Long(2), Variant::from("Bob"), Variant::Long(20)],
            vec![Variant::Long(3), Variant::from("Cy"), Variant::Null],
        ],
    );
    db
}

/// Runs `Main` of a standard module and returns its `Debug.Print` text.
pub fn run(db: &Rc<Db>, code: &str) -> String {
    let mut inst = load(db, &[("Module1", ModuleKind::Standard, code)]);
    inst.run("Main", vec![])
        .unwrap_or_else(|e| panic!("{e}"))
        .output
}

pub fn run_err(db: &Rc<Db>, code: &str) -> Error {
    let mut inst = load(db, &[("Module1", ModuleKind::Standard, code)]);
    inst.run("Main", vec![]).unwrap_err()
}

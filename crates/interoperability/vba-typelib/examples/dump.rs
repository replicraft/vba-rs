//! Debug dump of the interfaces of a type library.
//!
//! ```text
//! cargo run -p vba-typelib --example dump -- <file.tlb|file.dll> [--members] [--type NAME]
//! ```
//!
//! Prints every library of the file (all `TYPELIB` resources of a PE image),
//! its types and, with `--members`, their functions, variables and
//! constants. Independent of the `vbars` CLI.

use vba_typelib::{
    ConstValue, FuncFlags, InvokeKind, TypeInfo, TypeKind, TypeLib, VarKind, load_all,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("usage: dump <file> [--members] [--type NAME]");
        std::process::exit(2);
    };
    let members = args.iter().any(|a| a == "--members");
    let only = args
        .iter()
        .position(|a| a == "--type")
        .and_then(|i| args.get(i + 1));
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    match load_all(&data) {
        Ok(libs) => {
            for (id, lib) in &libs {
                print_lib(*id, lib, members, only.map(String::as_str));
            }
        }
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    }
}

fn print_lib(id: u16, lib: &TypeLib, members: bool, only: Option<&str>) {
    println!(
        "library {} {{{}}} v{}.{} lcid={:#x} flags={:?} syskind={:?} (resource {id})",
        lib.name, lib.guid, lib.version.major, lib.version.minor, lib.lcid, lib.flags, lib.sys_kind
    );
    if let Some(d) = &lib.doc {
        println!("  doc: {d}");
    }
    for imp in &lib.imports {
        println!(
            "  imports {} {:?} {}",
            imp.file,
            imp.type_guid,
            imp.kind.map_or("?".to_owned(), |k| format!("{k:?}"))
        );
    }
    for w in &lib.warnings {
        println!("  warning: {w}");
    }
    for t in &lib.types {
        if only.is_some_and(|n| !t.name.eq_ignore_ascii_case(n)) {
            continue;
        }
        print_type(lib, t, members);
    }
}

fn print_type(lib: &TypeLib, t: &TypeInfo, members: bool) {
    let guid = t.guid.map_or(String::new(), |g| format!(" {{{g}}}"));
    println!(
        "{:?} {}{guid} flags={:?} ({} functions, {} variables)",
        t.kind,
        t.name,
        t.flags,
        t.functions.len(),
        t.variables.len()
    );
    if !members {
        return;
    }
    if let Some(a) = &t.alias {
        println!("    = {}", lib.describe(a));
    }
    for i in &t.implements {
        println!(
            "    implements {} ({:?})",
            lib.describe(&vba_typelib::TypeDesc::User(i.target)),
            i.flags
        );
    }
    if let Some(d) = &t.dll_name {
        println!("    dll {d}");
    }
    for f in &t.functions {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                let mut s = format!(
                    "{}{} As {}",
                    if p.flags.contains(vba_typelib::ParamFlags::OPTIONAL) {
                        "Optional "
                    } else {
                        ""
                    },
                    p.name.as_deref().unwrap_or("_"),
                    lib.describe(&p.ty)
                );
                if let Some(c) = &p.default {
                    s.push_str(&format!(" = {:?}", c));
                }
                s
            })
            .collect();
        let inv = match f.invoke {
            InvokeKind::Func => "",
            InvokeKind::PropertyGet => "get ",
            InvokeKind::PropertyPut => "put ",
            InvokeKind::PropertyPutRef => "putref ",
        };
        let hidden = if f.flags.contains(FuncFlags::RESTRICTED) {
            " [restricted]"
        } else {
            ""
        };
        println!(
            "    [id {:#x}] {inv}{}({}) -> {}{hidden}",
            f.memid,
            f.name,
            params.join(", "),
            lib.describe(&f.ret)
        );
    }
    for v in &t.variables {
        let value = match (&v.kind, &v.value) {
            (VarKind::Const, Some(c)) => match &c.value {
                ConstValue::Int(i) => format!(" = {i}"),
                other => format!(" = {other:?}"),
            },
            _ => String::new(),
        };
        println!(
            "    [id {:#x}] {:?} {} As {}{value}",
            v.memid,
            v.kind,
            v.name,
            lib.describe(&v.ty)
        );
    }
    let _ = TypeKind::Enum;
}

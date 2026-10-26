//! Generates `crates/windows/vba-winapi/data/win32-functions.csv` from the
//! `Windows.Win32.winmd` bundled in Microsoft's `windows-default` crate
//! (windows-rs, MIT OR Apache-2.0).
//!
//! Usage: `cargo run -p vba-winapi-gen -- [OUTPUT]` (default: the committed
//! data file, resolved relative to this package). The output is deterministic
//! for a given `windows-default` version; never edit it by hand.
//!
//! Columns (`,` separated, no field contains a comma):
//! `dll,symbol,base,charset,pair,params,ret,conv,arch`
//!
//! * `dll`: lower-case import scope (`kernel32`, `api-ms-win-...`).
//! * `symbol`: the exported name (the PInvoke import name).
//! * `base`/`charset`/`pair`: A/W pairing. `charset` is `A`, `W` or `-`;
//!   `base` is the symbol without that suffix; `pair` is `AW` when both
//!   variants exist in the same DLL, otherwise `-`.
//! * `params`: `|` separated `name:direction:type`; direction is `in`, `out`,
//!   `inout` or `-` (unspecified); a trailing `?` marks an optional parameter.
//! * `ret`: return type; `conv`: `system` (stdcall/WINAPI), `C` (cdecl) or `-`.
//!
//! There is deliberately no `SetLastError` column: the metadata bundled with
//! windows-rs does not carry that information (no function has the
//! `SupportsLastError` PInvoke flag), and a column of zeros would mislead.
//! * `arch`: `all` or `+` joined `x86`, `x64`, `arm64`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use windows_metadata::reader::{File, Index, Item, ParamDirection};
use windows_metadata::{HasAttributes, Type};

const HEADER: &str = "dll,symbol,base,charset,pair,params,ret,conv,arch";

fn ty(t: &Type) -> String {
    match t {
        Type::Void => "void".into(),
        Type::Bool => "bool".into(),
        Type::Char => "char16".into(),
        Type::I8 => "i8".into(),
        Type::U8 => "u8".into(),
        Type::I16 => "i16".into(),
        Type::U16 => "u16".into(),
        Type::I32 => "i32".into(),
        Type::U32 => "u32".into(),
        Type::I64 => "i64".into(),
        Type::U64 => "u64".into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::ISize => "isize".into(),
        Type::USize => "usize".into(),
        Type::String => "string".into(),
        Type::Object => "object".into(),
        Type::ClassName(n) | Type::ValueName(n) => n.name.clone(),
        Type::Array(i) => format!("[{}]", ty(i)),
        Type::Generic(n, _) => n.clone(),
        Type::RefMut(i) => format!("&mut {}", ty(i)),
        Type::RefConst(i) => format!("&{}", ty(i)),
        Type::PtrMut(i, n) => format!("{}mut {}", "*".repeat(*n), ty(i)),
        Type::PtrConst(i, n) => format!("{}const {}", "*".repeat(*n), ty(i)),
        Type::ArrayFixed(i, n) => format!("[{} x {n}]", ty(i)),
    }
}

/// Lower-case library name without a `.dll` extension (the catalog's form).
fn dll_name(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    lower
        .strip_suffix(".dll")
        .map_or(lower.clone(), str::to_string)
}

fn clean(s: &str) -> String {
    s.replace([',', '|', '\n', '\r'], "_")
}

struct Row {
    dll: String,
    symbol: String,
    params: String,
    ret: String,
    conv: &'static str,
    arch: String,
}

fn arch(bits: i32) -> String {
    let mut v = Vec::new();
    for (bit, name) in [(1, "x86"), (2, "x64"), (4, "arm64")] {
        if bits & bit != 0 {
            v.push(name);
        }
    }
    if v.is_empty() || v.len() == 3 {
        "all".into()
    } else {
        v.join("+")
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/windows/vba-winapi/data/win32-functions.csv"
        )
        .to_string()
    });
    let file = File::new(windows_default::WIN32.to_vec()).expect("bundled Win32 metadata");
    let index = Index::new(vec![file]);
    let mut rows = Vec::new();
    for (_, _, item) in index.iter_items() {
        let Item::Fn(def) = item else { continue };
        let Some(map) = def.impl_map() else { continue };
        let sig = def.signature(&[]);
        let pmap = def
            .params_by_sequence(sig.types.len())
            .expect("well-formed parameter sequences");
        let mut params = Vec::new();
        for (i, t) in sig.types.iter().enumerate() {
            let (name, dir, opt) = match pmap.params()[i] {
                Some(p) => (
                    p.name().to_string(),
                    match p.direction() {
                        ParamDirection::Input => "in",
                        ParamDirection::Output => "out",
                        ParamDirection::InputOutput => "inout",
                        ParamDirection::Unspecified => "-",
                    },
                    p.is_optional(),
                ),
                None => (format!("arg{i}"), "-", false),
            };
            params.push(format!(
                "{}:{dir}{}:{}",
                clean(&name),
                if opt { "?" } else { "" },
                clean(&ty(t))
            ));
        }
        rows.push(Row {
            dll: dll_name(map.import_scope().name()),
            symbol: map.import_name().to_string(),
            params: params.join("|"),
            ret: clean(&ty(&sig.return_type)),
            conv: match def.calling_convention() {
                "" => "-",
                c => c,
            },
            arch: arch(def.arches()),
        });
    }
    rows.sort_by(|a, b| (&a.dll, &a.symbol, &a.arch).cmp(&(&b.dll, &b.symbol, &b.arch)));
    rows.dedup_by(|a, b| (&a.dll, &a.symbol, &a.arch) == (&b.dll, &b.symbol, &b.arch));
    let names: BTreeSet<(String, String)> = rows
        .iter()
        .map(|r| (r.dll.clone(), r.symbol.clone()))
        .collect();
    let mut text = String::from(HEADER);
    text.push('\n');
    for r in &rows {
        let (mut base, mut charset, mut pair) = (r.symbol.as_str(), '-', "-");
        for (c, other) in [('A', 'W'), ('W', 'A')] {
            if let Some(stem) = r.symbol.strip_suffix(c)
                && !stem.is_empty()
                && names.contains(&(r.dll.clone(), format!("{stem}{other}")))
            {
                (base, charset, pair) = (stem, c, "AW");
            }
        }
        let _ = writeln!(
            text,
            "{},{},{},{},{},{},{},{},{}",
            r.dll, r.symbol, base, charset, pair, r.params, r.ret, r.conv, r.arch
        );
    }
    std::fs::write(&out, text).expect("write output");
    eprintln!("wrote {} functions to {out}", rows.len());
}

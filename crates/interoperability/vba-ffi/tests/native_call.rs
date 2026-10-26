//! Real native calls through the descriptor path (feature `native-call`, Unix
//! hosts): libc/libm functions are called via `Declare`-style descriptors.
#![cfg(all(unix, feature = "native-call"))]

use vba_ffi::*;
use vba_profile::{Bitness, Target};
use vba_value::VbaString;

#[cfg(target_os = "macos")]
const LIBC: &str = "libSystem.B.dylib";
#[cfg(target_os = "macos")]
const LIBM: &str = "libSystem.B.dylib";
#[cfg(not(target_os = "macos"))]
const LIBC: &str = "libc.so.6";
#[cfg(not(target_os = "macos"))]
const LIBM: &str = "libm.so.6";

fn p(name: &str, kind: ParamKind, passing: Passing) -> ParamSpec {
    ParamSpec {
        name: name.into(),
        kind,
        passing,
    }
}

fn decl_in(
    target: Target,
    lib: &str,
    name: &str,
    params: Vec<ParamSpec>,
    ret: Option<ParamKind>,
) -> ValidatedDeclare {
    DeclareSpec {
        target,
        ptr_safe: true,
        name: name.into(),
        library: lib.into(),
        alias: None,
        charset: Charset::Ansi,
        conv: CallConv::Cdecl,
        params,
        ret,
    }
    .validate()
    .unwrap()
}

fn decl(lib: &str, name: &str, params: Vec<ParamSpec>, ret: Option<ParamKind>) -> ValidatedDeclare {
    decl_in(host_target(), lib, name, params, ret)
}

fn grant(lib: &str) -> NativePermission {
    NativePermission::grant([lib.to_string()])
}

fn run(
    d: &ValidatedDeclare,
    mem: &mut Memory,
    values: &[ArgValue],
) -> Result<CallOutcome, FfiError> {
    let mut prepared = PreparedCall::prepare(d, mem, values, Codepage::Windows1252)?;
    match call_prepared(&grant(&d.spec().library), d, mem, &prepared) {
        Ok(ret) => prepared.finish(d, mem, ret),
        Err(e) => {
            prepared.release(mem);
            Err(e.into())
        }
    }
}

fn s(v: &str) -> ArgValue {
    ArgValue::Str(VbaString::from(v))
}

#[test]
fn strlen_receives_a_real_copy_of_the_string() {
    let d = decl(
        LIBC,
        "strlen",
        vec![p("s", ParamKind::String, Passing::ByVal)],
        Some(ParamKind::LongPtr),
    );
    let mut mem = Memory::new(Bitness::B64);
    let out = run(&d, &mut mem, &[s("hello world")]).unwrap();
    assert_eq!(out.ret, Some(ArgValue::Int(11)));
    assert_eq!(mem.used(), 0, "temporaries are freed");
}

#[test]
fn abs_and_getpid_by_value() {
    let abs = decl(
        LIBC,
        "abs",
        vec![p("n", ParamKind::Long, Passing::ByVal)],
        Some(ParamKind::Long),
    );
    let mut mem = Memory::new(Bitness::B64);
    let out = run(&abs, &mut mem, &[ArgValue::Int(-42)]).unwrap();
    assert_eq!(out.ret, Some(ArgValue::Int(42)));

    let pid = decl(LIBC, "getpid", vec![], Some(ParamKind::Long));
    let out = run(&pid, &mut mem, &[]).unwrap();
    assert_eq!(out.ret, Some(ArgValue::Int(i64::from(std::process::id()))));

    // The value-only entry point works too.
    let r = call_native(&grant(LIBC), &abs, host_target(), &[NativeArg::Int(-7)]).unwrap();
    assert_eq!(r, NativeReturn::Int(7));
}

#[test]
fn floating_point_classes() {
    let sqrt = decl(
        LIBM,
        "sqrt",
        vec![p("x", ParamKind::Double, Passing::ByVal)],
        Some(ParamKind::Double),
    );
    let mut mem = Memory::new(Bitness::B64);
    let out = run(&sqrt, &mut mem, &[ArgValue::Float(2.25)]).unwrap();
    assert_eq!(out.ret, Some(ArgValue::Float(1.5)));

    let fabsf = decl(
        LIBM,
        "fabsf",
        vec![p("x", ParamKind::Single, Passing::ByVal)],
        Some(ParamKind::Single),
    );
    let out = run(&fabsf, &mut mem, &[ArgValue::Float(-0.5)]).unwrap();
    assert_eq!(out.ret, Some(ArgValue::Float(0.5)));
}

#[test]
fn byref_long_is_written_back() {
    // double frexp(double x, int *exp)
    let d = decl(
        LIBM,
        "frexp",
        vec![
            p("x", ParamKind::Double, Passing::ByVal),
            p("e", ParamKind::Long, Passing::ByRef),
        ],
        Some(ParamKind::Double),
    );
    let mut mem = Memory::new(Bitness::B64);
    let out = run(&d, &mut mem, &[ArgValue::Float(8.0), ArgValue::Int(0)]).unwrap();
    assert_eq!(out.ret, Some(ArgValue::Float(0.5)));
    assert_eq!(out.args[1], ArgValue::Int(4));
}

#[test]
fn strcpy_fills_a_byref_buffer_and_a_byval_string() {
    // char *strcpy(char *dst, const char *src)
    let any = decl(
        LIBC,
        "strcpy",
        vec![
            p("dst", ParamKind::Any, Passing::ByRef),
            p("src", ParamKind::String, Passing::ByVal),
        ],
        Some(ParamKind::LongPtr),
    );
    let mut mem = Memory::new(Bitness::B64);
    let out = run(&any, &mut mem, &[ArgValue::Bytes(vec![0xAA; 8]), s("abc")]).unwrap();
    assert_eq!(
        out.args[0],
        ArgValue::Bytes(vec![b'a', b'b', b'c', 0, 0xAA, 0xAA, 0xAA, 0xAA])
    );

    // The classic VBA idiom: a pre-sized ByVal String the callee fills.
    let by_string = decl(
        LIBC,
        "strcpy",
        vec![
            p("dst", ParamKind::String, Passing::ByVal),
            p("src", ParamKind::String, Passing::ByVal),
        ],
        None,
    );
    let out = run(&by_string, &mut mem, &[s("xxxxxxxx"), s("hi")]).unwrap();
    let ArgValue::Str(dst) = &out.args[0] else {
        panic!("string expected")
    };
    assert_eq!(&dst.as_units()[..3], &[u16::from(b'h'), u16::from(b'i'), 0]);
    assert_eq!(dst.as_units().len(), 8);
}

#[test]
fn a_callee_that_rewrites_a_string_cell_is_an_error() {
    // long strtol(const char *s, char **end, int base) rewrites `*end`.
    let d = decl(
        LIBC,
        "strtol",
        vec![
            p("s", ParamKind::String, Passing::ByVal),
            p("end", ParamKind::String, Passing::ByRef),
            p("base", ParamKind::Long, Passing::ByVal),
        ],
        Some(ParamKind::LongPtr),
    );
    let mut mem = Memory::new(Bitness::B64);
    let e = run(&d, &mut mem, &[s("123abc"), s("x"), ArgValue::Int(10)]).unwrap_err();
    assert_eq!(
        e,
        FfiError::Native(NativeCallError::PointerCellChanged { index: 1 })
    );
}

#[test]
fn missing_library_and_symbol_are_explicit() {
    let mut mem = Memory::new(Bitness::B64);
    let d = decl("libvba_rs_no_such_library.so", "abs", vec![], None);
    assert!(matches!(
        run(&d, &mut mem, &[]),
        Err(FfiError::Native(NativeCallError::LibraryNotFound { .. }))
    ));
    let d = decl(LIBC, "vba_rs_no_such_symbol", vec![], None);
    assert!(matches!(
        run(&d, &mut mem, &[]),
        Err(FfiError::Native(NativeCallError::SymbolNotFound { .. }))
    ));
}

#[test]
fn permission_is_required_and_scoped() {
    let d = decl(LIBC, "getpid", vec![], Some(ParamKind::Long));
    let deny = NativePermission::grant(["somethingelse".to_string()]);
    assert!(matches!(
        call_native(&deny, &d, host_target(), &[]),
        Err(NativeCallError::PermissionDenied { .. })
    ));
    let mut mem = Memory::new(Bitness::B64);
    let prepared = PreparedCall::prepare(&d, &mut mem, &[], Codepage::Windows1252).unwrap();
    assert!(matches!(
        call_prepared(&deny, &d, &mut mem, &prepared),
        Err(NativeCallError::PermissionDenied { .. })
    ));
}

#[test]
fn argument_count_and_class_are_checked() {
    let abs = decl(
        LIBC,
        "abs",
        vec![p("n", ParamKind::Long, Passing::ByVal)],
        Some(ParamKind::Long),
    );
    let ok = grant(LIBC);
    assert_eq!(
        call_native(&ok, &abs, host_target(), &[]),
        Err(NativeCallError::ArgumentCount {
            expected: 1,
            got: 0
        })
    );
    assert!(matches!(
        call_native(&ok, &abs, host_target(), &[NativeArg::F64(1.0)]),
        Err(NativeCallError::ArgumentClass { index: 0, .. })
    ));
    assert!(matches!(
        call_native(&ok, &abs, host_target(), &[NativeArg::Int(1 << 40)]),
        Err(NativeCallError::ArgumentClass { index: 0, .. })
    ));
    // A memory-taking declaration must go through call_prepared.
    let strlen = decl(
        LIBC,
        "strlen",
        vec![p("s", ParamKind::String, Passing::ByVal)],
        Some(ParamKind::LongPtr),
    );
    assert!(matches!(
        call_native(&ok, &strlen, host_target(), &[NativeArg::Int(4096)]),
        Err(NativeCallError::Unsupported { .. })
    ));
}

#[test]
fn other_abis_are_refused_not_attempted() {
    let ok = grant(LIBC);
    // Windows x64 is not the Unix host ABI, even at the same pointer width.
    let win = decl_in(Target::WIN64, LIBC, "getpid", vec![], Some(ParamKind::Long));
    let mut mem = Memory::new(Bitness::B64);
    let prepared = PreparedCall::prepare(&win, &mut mem, &[], Codepage::Windows1252).unwrap();
    assert_eq!(
        call_prepared(&ok, &win, &mut mem, &prepared),
        Err(NativeCallError::AbiMismatch {
            declared: Target::WIN64,
            loadable: host_target()
        })
    );
    // A 32-bit declaration cannot run in this 64-bit process.
    if host_target().bitness == Bitness::B64 {
        let mac32 = decl_in(Target::MAC32, LIBC, "getpid", vec![], Some(ParamKind::Long));
        let prepared = PreparedCall::prepare(&mac32, &mut mem, &[], Codepage::Windows1252).unwrap();
        assert!(matches!(
            call_prepared(&ok, &mac32, &mut mem, &prepared),
            Err(NativeCallError::AbiMismatch { .. })
        ));
    }
    // The pre-existing declared-vs-claimed-host check still comes first.
    assert!(matches!(
        call_native(&ok, &win, host_target(), &[]),
        Err(NativeCallError::TargetMismatch { .. })
    ));
}

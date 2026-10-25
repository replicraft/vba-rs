//! Integration tests for the Declare ABI layer.

use proptest::prelude::*;
use vba_ffi::*;
use vba_profile::Target;
use vba_value::VbaString;

fn p(name: &str, kind: ParamKind, passing: Passing) -> ParamSpec {
    ParamSpec {
        name: name.into(),
        kind,
        passing,
    }
}

fn spec(target: Target, params: Vec<ParamSpec>, ret: Option<ParamKind>) -> DeclareSpec {
    DeclareSpec {
        target,
        ptr_safe: true,
        name: "Foo".into(),
        library: "kernel32".into(),
        alias: None,
        charset: Charset::Ansi,
        conv: CallConv::Stdcall,
        params,
        ret,
    }
}

fn point(target: Target) -> UdtLayout {
    UdtLayout::new(
        target,
        vec![
            ("b".into(), UdtFieldType::Byte),
            ("l".into(), UdtFieldType::Long),
            ("d".into(), UdtFieldType::Double),
            ("i".into(), UdtFieldType::Integer),
        ],
    )
    .unwrap()
}

#[test]
fn validation_rules() {
    let t64 = Target::WIN64;
    let t32 = Target::WIN32;
    let mut s = spec(t64, vec![], None);
    s.ptr_safe = false;
    assert!(s.validate().is_err(), "PtrSafe required on 64-bit");
    let mut s = spec(t32, vec![], None);
    s.ptr_safe = false;
    assert!(s.validate().is_ok());
    let s = spec(t32, vec![p("a", ParamKind::LongLong, Passing::ByVal)], None);
    assert!(s.validate().is_err(), "LongLong needs 64-bit");
    let s = spec(t64, vec![p("a", ParamKind::LongLong, Passing::ByVal)], None);
    assert!(s.validate().is_ok());
    let s = spec(
        t32,
        vec![p("a", ParamKind::Udt(point(t32)), Passing::ByVal)],
        None,
    );
    assert!(s.validate().is_err(), "ByVal UDT");
    let s = spec(
        t32,
        vec![p("a", ParamKind::Udt(point(t64)), Passing::ByRef)],
        None,
    );
    assert!(s.validate().is_err(), "UDT of another target");
    let s = spec(t32, vec![p("a", ParamKind::Variant, Passing::ByVal)], None);
    assert!(s.validate().is_err());
    let s = spec(t32, vec![], Some(ParamKind::String));
    assert!(s.validate().is_err(), "non-scalar return");
    let mut s = spec(t32, vec![], None);
    s.alias = Some("#0".into());
    assert!(s.validate().is_err());
}

#[test]
fn entry_candidates() {
    let t32 = Target::WIN32;
    let d = spec(
        t32,
        vec![
            p("a", ParamKind::Double, Passing::ByVal),
            p("b", ParamKind::Long, Passing::ByRef),
        ],
        None,
    )
    .validate()
    .unwrap();
    assert_eq!(d.stdcall_arg_bytes(), 12);
    assert_eq!(
        d.entry_candidates(),
        vec![
            EntryCandidate::Symbol("Foo".into()),
            EntryCandidate::Symbol("FooA".into()),
            EntryCandidate::Symbol("_Foo@12".into())
        ]
    );
    let mut s = spec(Target::WIN64, vec![], None);
    s.charset = Charset::Unicode;
    assert_eq!(
        s.validate().unwrap().entry_candidates(),
        vec![
            EntryCandidate::Symbol("FooW".into()),
            EntryCandidate::Symbol("Foo".into())
        ]
    );
    let mut s = spec(t32, vec![], None);
    s.alias = Some("#17".into());
    assert_eq!(
        s.validate().unwrap().entry_candidates(),
        vec![EntryCandidate::Ordinal(17)]
    );
    let mut s = spec(t32, vec![], None);
    s.alias = Some("GetTickCount".into());
    assert_eq!(
        s.validate().unwrap().entry_candidates(),
        vec![EntryCandidate::Symbol("GetTickCount".into())]
    );
    let s = spec(Target::MAC64, vec![], None);
    assert_eq!(s.validate().unwrap().entry_candidates().len(), 1);
}

#[test]
fn udt_layout_follows_target() {
    let l32 = point(Target::WIN32);
    assert_eq!(l32.offset_of("L"), Some(4));
    assert_eq!(l32.offset_of("d"), Some(8));
    assert_eq!(l32.offset_of("i"), Some(16));
    assert_eq!((l32.size(), l32.align()), (24, 8));
    let ptrs = |t| {
        UdtLayout::new(
            t,
            vec![
                ("a".into(), UdtFieldType::Byte),
                ("p".into(), UdtFieldType::LongPtr),
            ],
        )
        .unwrap()
    };
    assert_eq!(ptrs(Target::WIN32).size(), 8);
    assert_eq!(ptrs(Target::WIN64).size(), 16);
    assert!(UdtLayout::new(Target::WIN32, vec![("x".into(), UdtFieldType::LongLong)]).is_err());
    let s = UdtLayout::new(
        Target::WIN32,
        vec![
            ("s".into(), UdtFieldType::FixedString(3)),
            ("l".into(), UdtFieldType::Long),
        ],
    )
    .unwrap();
    assert_eq!((s.offset_of("l"), s.size()), (Some(4), 8));
}

#[test]
fn memory_bounds_and_pointers() {
    let mut m = Memory::new(vba_profile::Bitness::B32);
    let a = m.alloc(8).unwrap();
    assert!(a > 0xFFFF);
    assert_eq!(m.read(0, 1), Err(MemoryError::NullPointer));
    assert!(matches!(m.read(a, 9), Err(MemoryError::OutOfBounds { .. })));
    assert!(matches!(
        m.read(a + 8, 1),
        Err(MemoryError::Unmapped { .. })
    ));
    m.write(a, &[1, 2, 3, 4]).unwrap();
    m.copy(a + 2, a, 4).unwrap();
    assert_eq!(m.read(a, 6).unwrap(), &[1, 2, 1, 2, 3, 4]);
    assert!(m.copy(a, a, 9).is_err());
    m.free(a).unwrap();
    assert!(m.free(a).is_err());
    let mut small = Memory::with_budget(vba_profile::Bitness::B32, 10);
    assert_eq!(small.alloc(11), Err(MemoryError::OutOfMemory));
}

#[test]
fn handles_are_provider_affine() {
    let a = ProviderId("a");
    let b = ProviderId("b");
    let mut t = HandleTable::new();
    let h = t.mint(a, "file");
    assert_eq!(t.resolve(a, &h), Ok("file"));
    assert_eq!(
        t.resolve(b, &h),
        Err(HandleError::WrongProvider {
            owner: "a",
            caller: "b"
        })
    );
    assert!(t.resolve_value(b, h.value()).is_err());
    assert_eq!(t.resolve_value(a, h.value() + 4), Err(HandleError::Invalid));
    let other = HandleTable::new();
    assert_eq!(other.resolve(a, &h), Err(HandleError::ForeignTable));
    assert!(t.close(b, &h).is_err());
    t.close(a, &h).unwrap();
    assert_eq!(t.resolve(a, &h), Err(HandleError::Invalid));
    let h2 = t.mint(a, "file");
    assert_ne!(h.value(), h2.value(), "tokens are never reused");
}

#[test]
fn last_error_is_per_logical_thread() {
    let mut s = LastErrorStore::new();
    assert_eq!(s.get(LogicalThread(1)), 0);
    s.set(LogicalThread(1), 5);
    s.set(LogicalThread(2), 87);
    assert_eq!((s.get(LogicalThread(1)), s.get(LogicalThread(2))), (5, 87));
}

#[test]
fn native_call_is_gated() {
    let d = spec(Target::WIN32, vec![], Some(ParamKind::Long))
        .validate()
        .unwrap();
    let deny = NativePermission::grant(["user32".to_string()]);
    assert!(matches!(
        call_native(&deny, &d, Target::WIN32, &[]),
        Err(NativeCallError::PermissionDenied { .. })
    ));
    let ok = NativePermission::grant(["KERNEL32".to_string()]);
    assert!(matches!(
        call_native(&ok, &d, Target::WIN64, &[]),
        Err(NativeCallError::TargetMismatch { .. })
    ));
    assert!(matches!(
        call_native(&ok, &d, Target::WIN32, &[]),
        Err(NativeCallError::Unavailable { .. })
    ));
}

#[test]
fn codec_known_values() {
    let u = |s: &str| s.encode_utf16().collect::<Vec<_>>();
    let e = Codepage::Windows1252
        .encode(&u("€é"), Unmappable::Reject, true)
        .unwrap();
    assert_eq!(e.bytes, vec![0x80, 0xE9]);
    assert!(
        Codepage::Ascii
            .encode(&u("é"), Unmappable::Reject, true)
            .is_err()
    );
    let r = Codepage::Ascii
        .encode(&u("aé"), Unmappable::Replace(b'?'), true)
        .unwrap();
    assert_eq!((r.bytes, r.used_default), (vec![b'a', b'?'], true));
    assert_eq!(
        Codepage::Utf8.decode(&[0xE2, 0x82, 0xAC], true).unwrap(),
        vec![0x20AC]
    );
    assert!(Codepage::Utf8.decode(&[0xFF], true).is_err());
    assert_eq!(Codepage::Utf8.decode(&[0xFF], false).unwrap(), vec![0xFFFD]);
    assert_eq!(
        Codepage::Utf8
            .encode(&[0xD800], Unmappable::Reject, false)
            .unwrap()
            .bytes,
        vec![0xEF, 0xBF, 0xBD]
    );
    assert_eq!(
        Codepage::Windows1252.decode(&[0x81], true).unwrap(),
        vec![0x81]
    );
}

#[test]
fn marshal_by_ref_string_and_scalars() {
    let t = Target::WIN32;
    let d = spec(
        t,
        vec![
            p("s", ParamKind::String, Passing::ByVal),
            p("r", ParamKind::String, Passing::ByRef),
            p("n", ParamKind::Long, Passing::ByRef),
            p("v", ParamKind::Integer, Passing::ByVal),
        ],
        Some(ParamKind::Long),
    )
    .validate()
    .unwrap();
    let mut m = Memory::new(t.bitness);
    let vals = vec![
        ArgValue::Str(VbaString::from_slice(&[b'h' as u16, b'i' as u16])),
        ArgValue::Str(VbaString::from_slice(&[b'x' as u16, b'y' as u16])),
        ArgValue::Int(-7),
        ArgValue::Int(300),
    ];
    let call = PreparedCall::prepare(&d, &mut m, &vals, Codepage::Windows1252).unwrap();
    let a = call.args().to_vec();
    // ByVal string: pointer to "hi\0".
    assert_eq!(m.read_cstr(a[0].as_int().unwrap() as u64).unwrap(), b"hi");
    // ByRef string: pointer to a pointer to "xy\0"; the callee overwrites it.
    let cell = a[1].as_int().unwrap() as u64;
    let inner = m.read_u32(cell).unwrap() as u64;
    assert_eq!(m.read_cstr(inner).unwrap(), b"xy");
    m.write(inner, b"QZ").unwrap();
    m.write_u32(a[2].as_int().unwrap() as u64, 99).unwrap();
    assert_eq!(a[3], NativeArg::Int(300));
    let out = call
        .finish(&d, &mut m, NativeReturn::Int(0xFFFF_FFFF))
        .unwrap();
    assert_eq!(
        out.args[1],
        ArgValue::Str(VbaString::from_slice(&[b'Q' as u16, b'Z' as u16]))
    );
    assert_eq!(out.args[2], ArgValue::Int(99));
    assert_eq!(out.args[3], ArgValue::Int(300));
    assert_eq!(out.ret, Some(ArgValue::Int(-1)));
    assert_eq!(m.used(), 0, "temporaries released");
}

#[test]
fn marshal_rejects_mismatches_and_releases() {
    let t = Target::WIN32;
    let d = spec(
        t,
        vec![
            p("s", ParamKind::String, Passing::ByVal),
            p("n", ParamKind::Byte, Passing::ByVal),
        ],
        None,
    )
    .validate()
    .unwrap();
    let mut m = Memory::new(t.bitness);
    let bad = vec![ArgValue::Str(VbaString::new()), ArgValue::Int(256)];
    assert!(matches!(
        PreparedCall::prepare(&d, &mut m, &bad, Codepage::Windows1252),
        Err(FfiError::ArgumentMismatch { index: 1, .. })
    ));
    assert_eq!(m.used(), 0);
    assert!(PreparedCall::prepare(&d, &mut m, &bad[..1], Codepage::Windows1252).is_err());
    let v = spec(t, vec![p("v", ParamKind::Variant, Passing::ByRef)], None)
        .validate()
        .unwrap();
    assert!(matches!(
        PreparedCall::prepare(&v, &mut m, &[ArgValue::Int(1)], Codepage::Windows1252),
        Err(FfiError::Unsupported(_))
    ));
}

proptest! {
    #[test]
    fn ansi_string_buffer_round_trip(s in "[ -~\u{a0}-\u{ff}€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ]{0,40}", by_ref in any::<bool>()) {
        let t = Target::WIN64;
        let passing = if by_ref { Passing::ByRef } else { Passing::ByVal };
        let d = spec(t, vec![p("s", ParamKind::String, passing)], None).validate().unwrap();
        let mut m = Memory::new(t.bitness);
        let v = VbaString::from_units(s.encode_utf16().collect());
        let call = PreparedCall::prepare(&d, &mut m, &[ArgValue::Str(v.clone())], Codepage::Windows1252).unwrap();
        let out = call.finish(&d, &mut m, NativeReturn::Void).unwrap();
        prop_assert_eq!(&out.args[0], &ArgValue::Str(v));
        prop_assert_eq!(m.used(), 0);
    }

    #[test]
    fn wide_buffer_round_trip(units in proptest::collection::vec(1u16..=u16::MAX, 0..40)) {
        let mut m = Memory::new(vba_profile::Bitness::B64);
        let addr = m.alloc_wide(&VbaString::from_units(units.clone())).unwrap();
        prop_assert_eq!(m.read_wstr(addr).unwrap(), units);
    }

    #[test]
    fn codec_round_trips(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        for cp in [Codepage::Latin1, Codepage::Windows1252] {
            let units = cp.decode(&bytes, true).unwrap();
            prop_assert_eq!(cp.encode(&units, Unmappable::Reject, true).unwrap().bytes, bytes.clone());
        }
        if let Ok(units) = Codepage::Utf8.decode(&bytes, true) {
            prop_assert_eq!(Codepage::Utf8.encode(&units, Unmappable::Reject, true).unwrap().bytes, bytes);
        }
    }

    #[test]
    fn udt_round_trip(
        b in any::<u8>(), l in any::<i32>(), d in -1e300f64..1e300, i in any::<i16>(),
        ll in any::<i64>(), s in "[a-z]{4}", arr in proptest::collection::vec(any::<i16>(), 3),
        bits64 in any::<bool>(),
    ) {
        let t = if bits64 { Target::WIN64 } else { Target::WIN32 };
        let inner = point(t);
        let mut fields = vec![
            ("b".to_string(), UdtFieldType::Byte),
            ("l".into(), UdtFieldType::Long),
            ("d".into(), UdtFieldType::Double),
            ("i".into(), UdtFieldType::Integer),
            ("p".into(), UdtFieldType::LongPtr),
            ("s".into(), UdtFieldType::FixedString(4)),
            ("a".into(), UdtFieldType::Array { elem: Box::new(UdtFieldType::Integer), count: 3 }),
            ("n".into(), UdtFieldType::Udt(inner)),
        ];
        if bits64 { fields.push(("q".into(), UdtFieldType::LongLong)); }
        let layout = UdtLayout::new(t, fields).unwrap();
        prop_assert_eq!(layout.size() % layout.align(), 0);
        let nested = vec![FieldValue::Int(b.into()), FieldValue::Int(l.into()), FieldValue::Float(d), FieldValue::Int(i.into())];
        let ptr = if bits64 { ll } else { i64::from(l) };
        let mut vals = vec![
            FieldValue::Int(b.into()), FieldValue::Int(l.into()), FieldValue::Float(d), FieldValue::Int(i.into()),
            FieldValue::Int(ptr),
            FieldValue::Str(VbaString::from_units(s.encode_utf16().collect())),
            FieldValue::Array(arr.iter().map(|x| FieldValue::Int((*x).into())).collect()),
            FieldValue::Struct(nested),
        ];
        if bits64 { vals.push(FieldValue::Int(ll)); }
        let bytes = layout.encode(&vals, Codepage::Windows1252).unwrap();
        prop_assert_eq!(bytes.len(), layout.size());
        prop_assert_eq!(layout.decode(&bytes, Codepage::Windows1252).unwrap(), vals);
    }
}

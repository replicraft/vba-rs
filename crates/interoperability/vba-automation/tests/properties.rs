//! Property tests: name case-insensitivity and ByRef write-back.

use proptest::prelude::*;
use vba_automation::call::invoke_member;
use vba_automation::testing::MemObject;
use vba_automation::{
    Arg, CallArgs, Final, ObjectTable, ParamInfo, hresult, hresult::hresult_to_number,
    hresult::number_to_hresult, names_equal,
};
use vba_value::Variant;

fn recase(s: &str, mask: &[bool]) -> String {
    s.chars()
        .zip(mask.iter().cycle())
        .map(|(c, up)| {
            if *up {
                c.to_uppercase().collect::<String>()
            } else {
                c.to_lowercase().collect::<String>()
            }
        })
        .collect()
}

proptest! {
    #[test]
    fn member_lookup_is_case_insensitive(
        name in "[A-Za-z][A-Za-z0-9_]{0,12}",
        mask in proptest::collection::vec(any::<bool>(), 1..16),
    ) {
        let t = ObjectTable::new();
        let h = t.insert_obj(MemObject::new("C").property(&name, Variant::Long(7)));
        let probe = recase(&name, &mask);
        prop_assert!(names_equal(&name, &probe));
        let v = invoke_member(&t, h, &probe, &mut CallArgs::none(), &Final::Get).unwrap();
        prop_assert_eq!(v, Variant::Long(7));
        // A different name never resolves.
        let other = format!("{name}x");
        let r = invoke_member(&t, h, &other, &mut CallArgs::none(), &Final::Get);
        prop_assert_eq!(r.unwrap_err().number, 438);
    }

    #[test]
    fn named_args_are_case_insensitive(mask in proptest::collection::vec(any::<bool>(), 1..8), a in any::<i32>()) {
        let t = ObjectTable::new();
        let h = t.insert_obj(MemObject::new("C").method(
            "Echo",
            vec![ParamInfo::required("Value")],
            |_, c| Ok(c.args[0].value.clone()),
        ));
        let n = recase("value", &mask);
        let mut args = CallArgs::none().with_named(&n, Arg::val(a));
        let v = invoke_member(&t, h, "echo", &mut args, &Final::Get).unwrap();
        prop_assert_eq!(v, Variant::Long(a));
    }

    /// ByRef arguments see exactly what the callee wrote; by-value ones are
    /// never modified; mixing both keeps positions straight.
    #[test]
    fn byref_write_back(vals in proptest::collection::vec(any::<i32>(), 1..6),
                        refs in proptest::collection::vec(any::<bool>(), 1..6)) {
        let n = vals.len().min(refs.len());
        let params: Vec<ParamInfo> = (0..n).map(|i| ParamInfo::required(&format!("p{i}"))).collect();
        let t = ObjectTable::new();
        let h = t.insert_obj(MemObject::new("C").method("Negate", params, |_, c| {
            for a in &mut c.args {
                if let Variant::Long(v) = a.value {
                    a.value = Variant::Long(v.wrapping_neg());
                }
            }
            Ok(Variant::Empty)
        }));
        let mut args = CallArgs::positional(
            (0..n).map(|i| Arg { value: Variant::Long(vals[i]), by_ref: refs[i] }).collect(),
        );
        invoke_member(&t, h, "Negate", &mut args, &Final::Call).unwrap();
        for i in 0..n {
            let want = if refs[i] { vals[i].wrapping_neg() } else { vals[i] };
            prop_assert_eq!(&args.positional[i].value, &Variant::Long(want));
        }
    }

    #[test]
    fn hresult_number_roundtrip(n in 1i32..=65535) {
        prop_assert_eq!(hresult_to_number(number_to_hresult(n)), n);
    }

    #[test]
    fn vb_object_error_roundtrip(n in 0i32..=0xFFFF) {
        // vbObjectError + 0x154 and + 0x1F3 are the COM class-registration
        // HRESULTs, which VBA reports as 429; that collision is inherent.
        prop_assume!(n != 0x154 && n != 0x1F3);
        let num = hresult::VB_OBJECT_ERROR + n;
        prop_assert_eq!(hresult_to_number(number_to_hresult(num)), num);
    }
}

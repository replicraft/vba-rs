//! Property tests: file round trips, robustness against arbitrary file
//! contents and the path normalizer.

mod common;

use common::*;
use proptest::prelude::*;
use vba_runtime::{Vfs, normalize};

#[derive(Debug, Clone)]
enum Val {
    Long(i32),
    Text(String),
    Bool(bool),
    Dbl(i32),
}

impl Val {
    fn dim(&self, name: &str) -> String {
        let ty = match self {
            Val::Long(_) => "Long",
            Val::Text(_) => "String",
            Val::Bool(_) => "Boolean",
            Val::Dbl(_) => "Double",
        };
        format!("Dim {name} As {ty}\n")
    }

    fn literal(&self) -> String {
        match self {
            Val::Long(n) => format!("{n}&"),
            Val::Text(s) => format!("\"{s}\""),
            Val::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
            Val::Dbl(n) => format!("{}", f64::from(*n) / 4.0),
        }
    }

    fn shown(&self) -> String {
        match self {
            Val::Long(n) => n.to_string(),
            Val::Text(s) => s.clone(),
            Val::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
            Val::Dbl(n) => (f64::from(*n) / 4.0).to_string(),
        }
    }
}

fn val() -> impl Strategy<Value = Val> {
    prop_oneof![
        any::<i32>().prop_map(Val::Long),
        "[a-zA-Z0-9 ,.;:!?-]{0,12}".prop_map(Val::Text),
        any::<bool>().prop_map(Val::Bool),
        (-4_000_000..4_000_000i32).prop_map(Val::Dbl),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 120, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn write_then_input_round_trips(vals in prop::collection::vec(val(), 1..8)) {
        let names: Vec<String> = (0..vals.len()).map(|i| format!("v{i}")).collect();
        let mut src = String::from("Sub Main()\n");
        for (v, n) in vals.iter().zip(&names) {
            src.push_str(&v.dim(n));
        }
        src.push_str("Open \"f.txt\" For Output As #1\n");
        let lits: Vec<String> = vals.iter().map(Val::literal).collect();
        src.push_str(&format!("Write #1, {}\n", lits.join(", ")));
        src.push_str(&format!("Write #1, {}\n", lits.join("; ")));
        src.push_str("Close #1\nOpen \"f.txt\" For Input As #1\n");
        src.push_str(&format!("Input #1, {}\n", names.join(", ")));
        src.push_str(&format!(
            "Debug.Print {}\n",
            names.iter().map(|n| format!("CStr({n})")).collect::<Vec<_>>().join(" & \"|\" & ")
        ));
        src.push_str(&format!("Input #1, {}\n", names.join(", ")));
        src.push_str(&format!(
            "Debug.Print {}\n",
            names.iter().map(|n| format!("CStr({n})")).collect::<Vec<_>>().join(" & \"|\" & ")
        ));
        src.push_str("Debug.Print EOF(1)\nClose #1\nEnd Sub\n");
        let want_line = vals.iter().map(Val::shown).collect::<Vec<_>>().join("|");
        let o = run(&src, SharedFs::default(), all_files());
        prop_assert!(o.result.is_ok(), "{:?}\n{src}", o.result);
        prop_assert_eq!(o.out, format!("{want_line}\n{want_line}\nTrue\n"), "{}", src);
    }

    #[test]
    fn print_then_line_input_round_trips(lines in prop::collection::vec("[a-zA-Z0-9 .,;:!?-]{0,20}", 1..6)) {
        let mut src = String::from("Sub Main()\nDim s As String\nOpen \"p.txt\" For Output As #1\n");
        for l in &lines {
            src.push_str(&format!("Print #1, \"{l}\"\n"));
        }
        src.push_str("Close #1\nOpen \"p.txt\" For Input As #1\nDo While Not EOF(1)\nLine Input #1, s\nDebug.Print s\nLoop\nClose #1\nEnd Sub\n");
        let o = run(&src, SharedFs::default(), all_files());
        prop_assert!(o.result.is_ok(), "{:?}", o.result);
        let want: String = lines.iter().map(|l| format!("{l}\n")).collect();
        prop_assert_eq!(o.out, want);
    }

    #[test]
    fn binary_put_get_round_trips(longs in prop::collection::vec(any::<i32>(), 1..10), dbls in prop::collection::vec(-1.0e9..1.0e9f64, 1..5)) {
        let mut src = String::from("Sub Main()\nDim l As Long, d As Double\nOpen \"b.bin\" For Binary As #1\n");
        for n in &longs {
            src.push_str(&format!("l = {n}&\nPut #1, , l\n"));
        }
        for x in &dbls {
            src.push_str(&format!("d = {x:?}#\nPut #1, , d\n"));
        }
        src.push_str("Seek #1, 1\n");
        for _ in &longs {
            src.push_str("Get #1, , l\nDebug.Print l\n");
        }
        for _ in &dbls {
            src.push_str("Get #1, , d\nDebug.Print d\n");
        }
        src.push_str("Close #1\nEnd Sub\n");
        let o = run(&src, SharedFs::default(), all_files());
        prop_assert!(o.result.is_ok(), "{:?}\n{src}", o.result);
        let mut want = String::new();
        for n in &longs {
            want.push_str(&format!(" {n} \n").replace(" -", "-"));
        }
        for x in &dbls {
            let shown = vba_value::convert::to_vba_string(&vba_value::Variant::Double(*x)).unwrap().to_string_lossy();
            if *x < 0.0 {
                want.push_str(&format!("{shown} \n"));
            } else {
                want.push_str(&format!(" {shown} \n"));
            }
        }
        prop_assert_eq!(o.out, want);
    }

    #[test]
    fn arbitrary_file_contents_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..200)) {
        let fs = SharedFs::default();
        fs.0.borrow_mut().write("/junk.bin", &bytes).unwrap();
        let src = "Sub Main()\nOn Error Resume Next\nDim s As String, v As Variant, n As Long, i As Long\nOpen \"junk.bin\" For Input As #1\nDo While Not EOF(1) And i < 50\nLine Input #1, s\ni = i + 1\nLoop\nClose #1\nOpen \"junk.bin\" For Input As #1\nFor i = 1 To 40\nInput #1, v\nInput #1, n\nInput #1, s\nNext\nClose #1\nOpen \"junk.bin\" For Binary As #1\nFor i = 1 To 40\nGet #1, , n\nGet #1, , v\nGet #1, , s\nNext\nClose #1\nOpen \"junk.bin\" For Random As #1 Len = 16\nFor i = 1 To 40\nGet #1, i, n\nGet #1, i, v\nNext\nClose #1\nDebug.Print \"done\"\nEnd Sub\n";
        let o = run(src, fs, all_files());
        prop_assert!(o.result.is_ok(), "{:?}", o.result);
        prop_assert_eq!(o.out, "done\n");
    }

    #[test]
    fn normalized_paths_are_canonical(cwd in "[/a-z.\\\\:]{0,20}", path in "[/a-zA-Z.\\\\:]{0,30}") {
        let once = normalize(&cwd, &path);
        prop_assert!(!once.contains('\\'));
        prop_assert!(once.starts_with('/') || once.as_bytes().get(1) == Some(&b':'));
        prop_assert!(!once.split('/').any(|s| s == ".." || s == "."), "{once}");
        prop_assert!(!once.contains("//"), "{once}");
        prop_assert_eq!(normalize("/elsewhere", &once), once);
    }
}

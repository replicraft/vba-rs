//! End-to-end tests of the command-line surface: standard input, JSON
//! output, colours, completions, the manual page and exit codes.

use assert_cmd::Command;

fn vbars() -> Command {
    Command::cargo_bin("vbars").unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

const HELLO: &str = "Option Explicit\nPublic Sub Main()\n    Debug.Print \"hi\"\nEnd Sub\n";
const UNDECLARED: &str = "Option Explicit\nSub Main()\n    x = 1\nEnd Sub\n";

#[test]
fn run_reads_the_program_from_standard_input() {
    let out = vbars()
        .args(["run", "-"])
        .write_stdin(HELLO)
        .assert()
        .success();
    assert_eq!(text(&out.get_output().stdout), "hi\n");
}

#[test]
fn run_from_standard_input_reports_runtime_errors_with_exit_code_2() {
    let src = "Sub Main()\n    Err.Raise 5\nEnd Sub\n";
    vbars().args(["run", "-"]).write_stdin(src).assert().code(2);
}

#[test]
fn run_has_a_wall_clock_limit_that_unlimited_and_zero_remove() {
    let spin = "Sub Main()\n    Dim i As Long\n    For i = 1 To 30000\n        DoEvents\n    Next\n    Debug.Print \"end\"\nEnd Sub\n";
    // A short allowance ends a run that has steps left (exit code 4).
    let long = "Sub Main()\n    Do\n    Loop\nEnd Sub\n";
    let out = vbars()
        .args(["run", "-", "--timeout", "1", "--max-steps", "100000000000"])
        .write_stdin(long)
        .assert()
        .code(4);
    let err = text(&out.get_output().stderr);
    assert!(err.contains("allowance") || err.contains("cancel"), "{err}");
    // A program that finishes is unaffected, with or without a limit.
    for extra in [&["--timeout", "0"][..], &["--unlimited"][..], &[][..]] {
        let out = vbars()
            .args(["run", "-"])
            .args(extra)
            .write_stdin(spin)
            .assert()
            .success();
        assert_eq!(text(&out.get_output().stdout), "end\n");
    }
}

#[test]
fn check_reads_standard_input_and_names_it_in_diagnostics() {
    vbars()
        .args(["check", "-"])
        .write_stdin(HELLO)
        .assert()
        .success();
    let out = vbars()
        .args(["check", "-"])
        .write_stdin(UNDECLARED)
        .assert()
        .code(1);
    let stdout = text(&out.get_output().stdout);
    assert!(stdout.contains("<stdin>:3"), "{stdout}");
}

#[test]
fn lex_parse_fmt_check_and_inspect_read_standard_input() {
    for sub in ["lex", "parse", "fmt-check", "inspect"] {
        let out = vbars()
            .args([sub, "-"])
            .write_stdin(HELLO)
            .assert()
            .success();
        assert!(!out.get_output().stdout.is_empty(), "{sub}");
    }
}

#[test]
fn standard_input_that_is_not_text_fails_cleanly() {
    let out = vbars()
        .args(["check", "-"])
        .write_stdin(vec![0x81u8, 0x8d, 0x8f, 0x90, 0x9d, 0xff, 0x00])
        .assert();
    // Windows-1252 decodes any byte, so this is a source error or a
    // diagnostic, never a crash.
    let code = out.get_output().status.code();
    assert!(matches!(code, Some(0 | 1)), "{code:?}");
}

#[test]
fn check_json_has_a_documented_shape() {
    let out = vbars()
        .args(["check", "--json", "-"])
        .write_stdin(UNDECLARED)
        .assert()
        .code(1);
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["errors"], 1);
    let d = &v["diagnostics"][0];
    assert_eq!(d["code"], "SEM002");
    assert_eq!(d["severity"], "error");
    assert_eq!(d["file"], "<stdin>");
    assert_eq!(d["line"], 3);
    assert_eq!(d["column"], 5);
    assert!(d["message"].is_string());
    assert!(d["labels"].is_array() && d["notes"].is_array());
}

#[test]
fn check_json_for_a_clean_file_is_success() {
    let out = vbars()
        .args(["check", "--json", "-"])
        .write_stdin(HELLO)
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["errors"], 0);
    assert_eq!(v["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn parse_json_and_tree_conflict() {
    let out = vbars()
        .args(["parse", "--json", "-"])
        .write_stdin(HELLO)
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert!(v["items"].as_u64().unwrap() > 0);
    assert!(v["nodes"].as_u64().unwrap() > 0);
    vbars()
        .args(["parse", "--json", "--tree", "-"])
        .write_stdin(HELLO)
        .assert()
        .code(2);
}

#[test]
fn parse_json_reports_syntax_errors_with_exit_code_1() {
    let out = vbars()
        .args(["parse", "--json", "-"])
        .write_stdin("Sub Main(\nEnd Sub\n")
        .assert()
        .code(1);
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert!(!v["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn fmt_check_json_lists_findings() {
    let out = vbars()
        .args(["fmt-check", "--json", "-"])
        .write_stdin("Sub Main()   \nEnd Sub\n")
        .assert()
        .code(1);
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["findings"].as_array().unwrap().len(), 1);
    assert_eq!(v["file"], "<stdin>");
}

#[test]
fn profiles_json_marks_exactly_one_default() {
    let out = vbars().args(["profiles", "--json"]).assert().success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    let list = v["profiles"].as_array().unwrap();
    assert!(!list.is_empty());
    assert_eq!(list.iter().filter(|p| p["default"] == true).count(), 1);
}

#[test]
fn color_is_off_when_not_a_terminal_and_on_when_forced() {
    let plain = vbars()
        .args(["check", "-"])
        .write_stdin(UNDECLARED)
        .assert()
        .code(1);
    assert!(!text(&plain.get_output().stdout).contains('\u{1b}'));
    let forced = vbars()
        .args(["check", "--color", "always", "-"])
        .write_stdin(UNDECLARED)
        .assert()
        .code(1);
    assert!(text(&forced.get_output().stdout).contains('\u{1b}'));
    let never = vbars()
        .args(["check", "--color", "never", "-"])
        .write_stdin(UNDECLARED)
        .assert()
        .code(1);
    assert!(!text(&never.get_output().stdout).contains('\u{1b}'));
}

#[test]
fn completions_are_generated_for_each_shell() {
    for (shell, marker) in [
        ("bash", "_vbars"),
        ("zsh", "#compdef vbars"),
        ("fish", "complete -c vbars"),
        ("powershell", "Register-ArgumentCompleter"),
        ("elvish", "edit:completion"),
    ] {
        let out = vbars().args(["completions", shell]).assert().success();
        let s = text(&out.get_output().stdout);
        assert!(s.contains(marker), "{shell}: {s}");
        assert!(s.contains("fmt-check"), "{shell}");
    }
    vbars().args(["completions", "nope"]).assert().code(2);
}

#[test]
fn man_page_is_roff_and_lists_subcommands() {
    let out = vbars().arg("man").assert().success();
    let s = text(&out.get_output().stdout);
    assert!(s.contains(".TH vbars"), "{s}");
    assert!(s.contains("vbars\\-run"), "{s}");
}

#[test]
fn help_documents_exit_codes_and_stdin() {
    let out = vbars().arg("--help").assert().success();
    let s = text(&out.get_output().stdout);
    assert!(s.contains("Exit codes:"), "{s}");
    assert!(s.contains("`-` reads standard input"), "{s}");
    assert!(s.contains("NO_COLOR"), "{s}");
}

#[test]
fn every_subcommand_has_help() {
    for sub in [
        "inspect",
        "lex",
        "parse",
        "check",
        "fmt-check",
        "profiles",
        "run",
        "completions",
        "man",
    ] {
        vbars().args([sub, "--help"]).assert().success();
    }
}

#[test]
fn missing_arguments_and_unknown_subcommands_are_usage_errors() {
    vbars().assert().code(2);
    vbars().arg("frobnicate").assert().code(2);
    vbars().arg("check").assert().code(2);
    vbars().args(["lex", "a.bas", "--bogus"]).assert().code(2);
}

#[test]
fn missing_files_fail_with_exit_code_1_for_every_file_command() {
    for sub in ["inspect", "lex", "parse", "check", "fmt-check", "run"] {
        let out = vbars().args([sub, "/no/such/file.bas"]).assert().code(1);
        let err = text(&out.get_output().stderr);
        assert!(err.starts_with("error: "), "{sub}: {err}");
        assert!(err.contains("/no/such/file.bas"), "{sub}: {err}");
    }
}

#[test]
fn unknown_profiles_fail_with_exit_code_1() {
    for sub in ["lex", "parse", "check", "fmt-check", "run"] {
        let out = vbars()
            .args([sub, "-", "--profile", "nope"])
            .write_stdin(HELLO)
            .assert()
            .code(1);
        assert!(
            text(&out.get_output().stderr).contains("unknown profile"),
            "{sub}"
        );
    }
}

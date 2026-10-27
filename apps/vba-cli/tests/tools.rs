//! End-to-end tests of `vbars lex|parse|check|fmt-check|profiles|run`.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use vba_format::{WriteOptions, write_project};
use vba_project::{Module, ModuleKind, VbaProject};

fn vbars() -> Command {
    Command::cargo_bin("vbars").unwrap()
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p
}

fn stdout(a: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(a.get_output().stdout.clone()).unwrap()
}

fn stderr(a: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(a.get_output().stderr.clone()).unwrap()
}

const HELLO: &str =
    "Option Explicit\r\nPublic Sub Main()\r\n    Debug.Print \"hello\"\r\nEnd Sub\r\n";

#[test]
fn check_reports_diagnostics_and_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let good = write(dir.path(), "Good.bas", HELLO);
    let a = vbars().arg("check").arg(&good).assert().success();
    assert!(stdout(&a).contains("0 error(s)"), "{}", stdout(&a));

    let bad = write(
        dir.path(),
        "Bad.bas",
        "Option Explicit\r\nSub Main()\r\n    x = 1\r\nEnd Sub\r\n",
    );
    let a = vbars().arg("check").arg(&bad).assert().code(1);
    let out = stdout(&a);
    assert!(out.contains("SEM002"), "{out}");
    assert!(out.contains("Bad.bas:3"), "{out}");
}

#[test]
fn check_analyses_several_files_as_one_project() {
    let dir = tempfile::tempdir().unwrap();
    let a = write(
        dir.path(),
        "A.bas",
        "Public Function Twice(x As Long) As Long\r\nTwice = x * 2\r\nEnd Function\r\n",
    );
    let b = write(
        dir.path(),
        "B.bas",
        "Public Sub Main()\r\nDebug.Print Twice(4)\r\nEnd Sub\r\n",
    );
    vbars().arg("check").arg(&a).arg(&b).assert().success();
    // Alone, B does not compile: Twice is unknown.
    vbars().arg("check").arg(&b).assert().code(1);
}

#[test]
fn check_rejects_unknown_profiles_and_files() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(dir.path(), "A.bas", HELLO);
    let a = vbars()
        .arg("check")
        .arg("--profile")
        .arg("nope")
        .arg(&f)
        .assert()
        .code(1);
    assert!(stderr(&a).contains("unknown profile"), "{}", stderr(&a));
    let txt = write(dir.path(), "notes.txt", "hello");
    vbars().arg("check").arg(&txt).assert().code(1);
    vbars()
        .arg("check")
        .arg(dir.path().join("missing.bas"))
        .assert()
        .code(1);
}

#[test]
fn lex_prints_tokens_with_positions() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(dir.path(), "A.bas", HELLO);
    let a = vbars().arg("lex").arg(&f).assert().success();
    let out = stdout(&a);
    assert!(out.contains("1:1 Keyword(Option) \"Option\""), "{out}");
    assert!(out.contains("StringLiteral"), "{out}");
    assert!(!out.contains("Whitespace"), "trivia is hidden by default");
    let a = vbars()
        .arg("lex")
        .arg("--trivia")
        .arg(&f)
        .assert()
        .success();
    assert!(stdout(&a).contains("Whitespace"));
}

#[test]
fn lex_reports_lexical_errors() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(
        dir.path(),
        "A.bas",
        "Sub Main()\r\nx = \"unterminated\r\nEnd Sub\r\n",
    );
    let a = vbars().arg("lex").arg(&f).assert().code(1);
    assert!(stdout(&a).contains("error"), "{}", stdout(&a));
}

#[test]
fn parse_prints_a_summary_the_tree_and_syntax_errors() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(dir.path(), "A.bas", HELLO);
    let a = vbars().arg("parse").arg(&f).assert().success();
    assert!(stdout(&a).contains("item(s)"), "{}", stdout(&a));
    let a = vbars()
        .arg("parse")
        .arg("--tree")
        .arg(&f)
        .assert()
        .success();
    assert!(stdout(&a).contains("Module"), "{}", stdout(&a));
    let bad = write(dir.path(), "B.bas", "Sub Main(\r\n");
    let a = vbars().arg("parse").arg(&bad).assert().code(1);
    assert!(stdout(&a).contains("error"), "{}", stdout(&a));
}

#[test]
fn fmt_check_reports_whitespace_findings() {
    let dir = tempfile::tempdir().unwrap();
    let ok = write(dir.path(), "Ok.bas", HELLO);
    let a = vbars().arg("fmt-check").arg(&ok).assert().success();
    assert!(stdout(&a).contains("ok"));
    let messy = write(
        dir.path(),
        "Messy.bas",
        "Sub Main()  \r\n \tDebug.Print 1\nEnd Sub",
    );
    let a = vbars().arg("fmt-check").arg(&messy).assert().code(1);
    let out = stdout(&a);
    assert!(out.contains("trailing whitespace"), "{out}");
    assert!(out.contains("mixed line endings"), "{out}");
    assert!(out.contains("mixed tabs and spaces"), "{out}");
    assert!(out.contains("does not end with a line break"), "{out}");
}

#[test]
fn profiles_lists_the_default() {
    let a = vbars().arg("profiles").assert().success();
    let out = stdout(&a);
    assert!(out.contains("[default]"), "{out}");
    assert!(out.contains("vba7"), "{out}");
}

#[test]
fn run_prints_debug_output_with_no_grants() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(dir.path(), "A.bas", HELLO);
    let a = vbars().arg("run").arg(&f).assert().success();
    assert_eq!(stdout(&a), "hello\n");
}

#[test]
fn run_selects_the_entry_and_passes_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(
        dir.path(),
        "A.bas",
        "Public Sub Show(n As Long, s As String)\r\nDebug.Print n * 2 & s\r\nEnd Sub\r\nPublic Sub Main()\r\nEnd Sub\r\n",
    );
    let a = vbars()
        .arg("run")
        .arg(&f)
        .args(["--entry", "A.Show", "--arg", "21", "--arg", "x"])
        .assert()
        .success();
    assert_eq!(stdout(&a), "42x\n");
    let a = vbars()
        .arg("run")
        .arg(&f)
        .args(["--entry", "Nope"])
        .assert()
        .code(1);
    assert!(stderr(&a).contains("no procedure named"), "{}", stderr(&a));
}

#[test]
fn run_denies_files_by_default_and_scopes_grants() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(data.join("in.txt"), "inside\r\n").unwrap();
    std::fs::write(dir.path().join("secret.txt"), "outside\r\n").unwrap();
    let base = std::fs::canonicalize(dir.path())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let prog = write(
        dir.path(),
        "Reader.bas",
        &format!(
            "Public Sub Main()\r\nDim s As String\r\nOpen \"{base}/data/in.txt\" For Input As #1\r\nLine Input #1, s\r\nClose #1\r\nDebug.Print s\r\nEnd Sub\r\nPublic Sub Escape()\r\nDim s As String\r\nOpen \"{base}/data/../secret.txt\" For Input As #1\r\nLine Input #1, s\r\nDebug.Print s\r\nEnd Sub\r\n"
        ),
    );
    // No grant: permission denied, exit code 3.
    let a = vbars().arg("run").arg(&prog).assert().code(3);
    assert!(stderr(&a).contains("permission denied"), "{}", stderr(&a));
    // Scoped grant.
    let a = vbars()
        .arg("run")
        .arg(&prog)
        .arg("--grant")
        .arg(format!("file-read={}", data.display()))
        .assert()
        .success();
    assert_eq!(stdout(&a), "inside\n");
    // The grant does not cover a path that leaves the directory.
    vbars()
        .arg("run")
        .arg(&prog)
        .args(["--entry", "Escape"])
        .arg("--grant")
        .arg(format!("file-read={}", data.display()))
        .assert()
        .code(3);
    // Write needs its own grant.
    let writer = write(
        dir.path(),
        "Writer.bas",
        &format!(
            "Public Sub Main()\r\nOpen \"{base}/data/out.txt\" For Output As #1\r\nPrint #1, \"w\"\r\nClose #1\r\nEnd Sub\r\n"
        ),
    );
    vbars()
        .arg("run")
        .arg(&writer)
        .arg("--grant")
        .arg(format!("file-read={}", data.display()))
        .assert()
        .code(3);
    vbars()
        .arg("run")
        .arg(&writer)
        .arg("--grant")
        .arg(format!("file-write={}", data.display()))
        .assert()
        .success();
    assert_eq!(
        std::fs::read_to_string(data.join("out.txt")).unwrap(),
        "w\r\n"
    );
}

#[test]
fn run_maps_failures_to_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let run = |src: &str, extra: &[&str]| -> (i32, String) {
        let f = write(dir.path(), "P.bas", src);
        let out = vbars().arg("run").arg(&f).args(extra).output().unwrap();
        (
            out.status.code().unwrap(),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    let (c, e) = run(
        "Public Sub Main()\r\nErr.Raise 77, \"src\", \"boom\"\r\nEnd Sub\r\n",
        &[],
    );
    assert_eq!(c, 7, "{e}");
    assert!(e.contains("Run-time error '77': boom"), "{e}");
    let (c, e) = run(
        "Public Sub Main()\r\nDo\r\nLoop\r\nEnd Sub\r\n",
        &["--max-steps", "10000"],
    );
    assert_eq!(c, 4, "{e}");
    assert!(e.contains("step limit"), "{e}");
    let (c, _) = run(
        "Public Sub Main()\r\nDim p As Double\r\np = Shell(\"true\")\r\nEnd Sub\r\n",
        &["--grant", "process-spawn"],
    );
    assert_eq!(c, 5, "Shell needs the launcher too");
    let (c, _) = run(
        "Public Sub Main()\r\nDim p As Double\r\np = Shell(\"true\")\r\nEnd Sub\r\n",
        &[],
    );
    assert_eq!(c, 3);
    let (c, e) = run("Public Sub Main()\r\nx = \r\nEnd Sub\r\n", &[]);
    assert_eq!(c, 1, "{e}");
    let (c, e) = run("Public Sub Main()\r\nEnd Sub\r\n", &["--grant", "teleport"]);
    assert_eq!(c, 1);
    assert!(e.contains("unknown capability"), "{e}");
    let (c, _) = run("Public Sub Main()\r\nStop\r\nEnd Sub\r\n", &[]);
    assert_eq!(c, 0, "Stop halts without an error");
}

#[test]
fn run_can_use_an_in_memory_file_system() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(
        dir.path(),
        "M.bas",
        "Public Sub Main()\r\nOpen \"/tmp/x.txt\" For Output As #1\r\nPrint #1, \"mem\"\r\nClose #1\r\nOpen \"/tmp/x.txt\" For Input As #1\r\nDim s As String\r\nLine Input #1, s\r\nDebug.Print s\r\nEnd Sub\r\n",
    );
    let a = vbars()
        .arg("run")
        .arg(&f)
        .arg("--memfs")
        .args(["--grant", "file-read", "--grant", "file-write"])
        .assert()
        .failure();
    // /tmp does not exist in the empty in-memory file system.
    assert_eq!(a.get_output().status.code(), Some(7));
    let f = write(
        dir.path(),
        "M2.bas",
        "Public Sub Main()\r\nOpen \"x.txt\" For Output As #1\r\nPrint #1, \"mem\"\r\nClose #1\r\nOpen \"x.txt\" For Input As #1\r\nDim s As String\r\nLine Input #1, s\r\nDebug.Print s\r\nEnd Sub\r\n",
    );
    let a = vbars()
        .arg("run")
        .arg(&f)
        .arg("--memfs")
        .args(["--grant", "file-read", "--grant", "file-write"])
        .assert()
        .success();
    assert_eq!(stdout(&a), "mem\n");
    assert!(!Path::new("x.txt").exists());
}

#[test]
fn run_loads_a_project_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = VbaProject::new("Demo");
    p.constants = "FLAG = 1".into();
    p.add_module(Module::new(
        "Main",
        ModuleKind::Standard,
        "Public Sub Run()\r\n#If FLAG Then\r\nDebug.Print \"flag\"\r\n#End If\r\nEnd Sub\r\n",
    ))
    .unwrap();
    let bin = dir.path().join("vbaProject.bin");
    std::fs::write(&bin, write_project(&p, &WriteOptions::default()).unwrap()).unwrap();
    let a = vbars()
        .arg("run")
        .arg(&bin)
        .args(["--entry", "Main.Run"])
        .assert()
        .success();
    assert_eq!(stdout(&a), "flag\n");
    let a = vbars().arg("check").arg(&bin).assert().success();
    assert!(stdout(&a).contains("1 module(s)"), "{}", stdout(&a));
}

#[test]
fn diagnostics_in_windows_1252_files_point_at_the_right_columns() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("Latin.bas");
    std::fs::write(
        &p,
        b"Sub Main()\r\n    s = \"caf\xe9\" + \"abc\r\nEnd Sub\r\n",
    )
    .unwrap();
    let a = vbars().arg("parse").arg(&p).assert().code(1);
    let out = stdout(&a);
    assert!(out.contains("Latin.bas:2:"), "{out}");
    assert!(out.contains("caf\u{e9}"), "{out}");
    assert!(out.contains("unterminated string literal"), "{out}");
}

#[test]
fn strict_spec_promotes_spec_strict_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(
        dir.path(),
        "Spec.bas",
        "DefInt A-D\r\nDefLng C-F\r\nPublic Sub Main()\r\n    Debug.Print \"ran\"\r\nEnd Sub\r\n",
    );
    let a = vbars().arg("check").arg(&f).assert().success();
    assert!(stdout(&a).contains("1 warning(s)"), "{}", stdout(&a));
    vbars()
        .arg("check")
        .arg("--strict-spec")
        .arg(&f)
        .assert()
        .code(1);
    let a = vbars().arg("run").arg("--memfs").arg(&f).assert().success();
    assert!(stdout(&a).contains("ran"), "{}", stdout(&a));
    assert!(stderr(&a).contains("warning"), "{}", stderr(&a));
    vbars()
        .arg("run")
        .arg("--memfs")
        .arg("--strict-spec")
        .arg(&f)
        .assert()
        .failure();
}

//! File access is denied by default and scoped by the host policy, for the
//! in-memory and the real file system alike.

mod common;

use common::*;
use vba_host::{Capability, Policy};
use vba_runtime::*;
use vba_vm::BufferSink;

/// Runs a body of statements and returns what it printed; every statement
/// that fails prints its error number through `T`.
fn trap(body: &str, fs: SharedFs, policy: Policy) -> String {
    let src = format!(
        "Sub T(what As String)\nPrint1 what\nEnd Sub\nSub Print1(s As String)\nDebug.Print s & \" \" & Err.Number\nErr.Clear\nEnd Sub\nSub Main()\nOn Error Resume Next\n{body}End Sub\n"
    );
    let o = run(&src, fs, policy);
    assert!(o.result.is_ok(), "{:?}", o.result);
    o.out
}

const PROBE: &str = "Open \"/data/in.txt\" For Input As #1\nT \"read\"\nClose\nOpen \"/data/new.txt\" For Output As #1\nT \"write\"\nClose\nKill \"/data/in.txt\"\nT \"kill\"\nMkDir \"/data/sub\"\nT \"mkdir\"\nDebug.Print FileLen(\"/data/in.txt\")\nT \"filelen\"\nDebug.Print Dir(\"/data/*\")\nT \"dir\"\n";

fn files() -> SharedFs {
    SharedFs::with(&[("/data/in.txt", "hello")])
}

#[test]
fn everything_is_denied_without_grants() {
    let out = trap(PROBE, files(), Policy::deny_all());
    assert_eq!(
        out,
        "read 70\nwrite 70\nkill 70\nmkdir 70\nfilelen 70\ndir 70\n"
    );
}

#[test]
fn read_only_grant_allows_reading_but_not_changing() {
    let fs = files();
    let out = trap(
        PROBE,
        fs.clone(),
        Policy::deny_all().grant(Capability::FileRead),
    );
    assert_eq!(
        out,
        "read 0\nwrite 70\nkill 70\nmkdir 70\n 5 \nfilelen 0\nin.txt\ndir 0\n"
    );
    assert_eq!(fs.text("/data/in.txt").as_deref(), Some("hello"));
    assert!(fs.text("/data/new.txt").is_none());
}

#[test]
fn prefix_grants_scope_access_and_ignore_traversal() {
    let fs = SharedFs::with(&[("/data/in.txt", "ok"), ("/secret/key.txt", "no")]);
    let policy = Policy::deny_all().grant_prefixes(Capability::FileRead, ["/data/"]);
    let body = "Open \"/data/in.txt\" For Input As #1\nT \"inside\"\nClose\nOpen \"/secret/key.txt\" For Input As #1\nT \"outside\"\nClose\nOpen \"/data/../secret/key.txt\" For Input As #1\nT \"dotdot\"\nClose\nOpen \"\\data\\..\\secret\\key.txt\" For Input As #1\nT \"backslash dotdot\"\nClose\nChDir \"/data\"\nOpen \"../secret/key.txt\" For Input As #1\nT \"relative dotdot\"\nClose\nOpen \"in.txt\" For Input As #1\nT \"relative inside\"\nClose\nOpen \"/DATA/in.txt\" For Input As #1\nT \"case\"\nClose\n";
    let out = trap(body, fs, policy);
    assert_eq!(
        out,
        "inside 0\noutside 70\ndotdot 70\nbackslash dotdot 70\nrelative dotdot 70\nrelative inside 0\ncase 0\n"
    );
}

#[test]
fn write_grants_are_independent_of_read_grants() {
    let fs = files();
    let policy = Policy::deny_all().grant_prefixes(Capability::FileWrite, ["/data/"]);
    let body = "Open \"/data/out.txt\" For Output As #1\nT \"write\"\nPrint #1, \"x\"\nClose\nOpen \"/data/out.txt\" For Input As #1\nT \"read back\"\nClose\n";
    assert_eq!(trap(body, fs.clone(), policy), "write 0\nread back 70\n");
    assert_eq!(fs.text("/data/out.txt").as_deref(), Some("x\r\n"));
}

#[test]
fn copying_and_renaming_check_both_ends() {
    let fs = files();
    let policy = Policy::deny_all()
        .grant(Capability::FileRead)
        .grant_prefixes(Capability::FileWrite, ["/data/"]);
    let body = "FileCopy \"/data/in.txt\", \"/data/copy.txt\"\nT \"copy inside\"\nFileCopy \"/data/in.txt\", \"/elsewhere.txt\"\nT \"copy out\"\nName \"/data/copy.txt\" As \"/moved.txt\"\nT \"rename out\"\nName \"/data/copy.txt\" As \"/data/renamed.txt\"\nT \"rename inside\"\n";
    let out = trap(body, fs.clone(), policy);
    assert_eq!(
        out,
        "copy inside 0\ncopy out 70\nrename out 70\nrename inside 0\n"
    );
    assert!(fs.text("/elsewhere.txt").is_none() && fs.text("/moved.txt").is_none());
    assert_eq!(fs.text("/data/renamed.txt").as_deref(), Some("hello"));
}

#[test]
fn denial_is_error_70_with_the_resource_in_the_source() {
    let o = run(
        "Sub Main()\nOn Error Resume Next\nOpen \"/data/in.txt\" For Input As #1\nDebug.Print Err.Description & \"|\" & Err.Source\nEnd Sub\n",
        files(),
        Policy::deny_all(),
    );
    assert!(o.out.starts_with("Permission denied|"), "{}", o.out);
    assert!(o.out.contains("/data/in.txt"), "{}", o.out);
}

#[test]
fn memory_and_size_limits_apply_to_open_files() {
    // A file bigger than the per-file limit is Disk full (61).
    let sink = BufferSink::new();
    let fs = SharedFs::with(&[("big.txt", "0123456789")]);
    let mut s = Session::builder()
        .host(host(all_files()))
        .vfs(fs)
        .debug_sink(sink.clone())
        .build_sources(&[std_module(
            "Public Sub Main()\nOn Error Resume Next\nOpen \"big.txt\" For Input As #1\nDebug.Print Err.Number\nErr.Clear\nOpen \"w.txt\" For Output As #2\nPrint #2, \"0123456789\"\nDebug.Print Err.Number\nEnd Sub\n",
        )])
        .unwrap();
    s.file_system().borrow_mut().set_max_file_bytes(8);
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), " 61 \n 61 \n");
}

#[test]
fn the_real_file_system_is_scoped_by_the_same_policy() {
    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let base_s = base.to_string_lossy().replace('\\', "/");
    std::fs::write(base.join("in.txt"), "real").unwrap();
    std::fs::create_dir(base.join("allowed")).unwrap();
    std::fs::write(base.join("allowed/a.txt"), "inside").unwrap();
    std::fs::write(base.join("secret.txt"), "outside").unwrap();
    let policy = Policy::deny_all()
        .grant_prefixes(Capability::FileRead, [format!("{base_s}/allowed/")])
        .grant_prefixes(Capability::FileWrite, [format!("{base_s}/allowed/")]);
    let src = format!(
        "Public Sub Main()\nOn Error Resume Next\nDim s As String\nOpen \"{base_s}/allowed/a.txt\" For Input As #1\nLine Input #1, s\nClose #1\nDebug.Print s & \" \" & Err.Number\nOpen \"{base_s}/secret.txt\" For Input As #1\nDebug.Print \"secret \" & Err.Number\nErr.Clear\nOpen \"{base_s}/allowed/new.txt\" For Output As #2\nPrint #2, \"written\"\nClose #2\nDebug.Print \"write \" & Err.Number\nEnd Sub\n"
    );
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(host(policy))
        .vfs(RealFs::in_dir(&base))
        .debug_sink(sink.clone())
        .build_sources(&[std_module(&src)])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "inside 0\nsecret 70\nwrite 0\n");
    assert_eq!(
        std::fs::read_to_string(base.join("allowed/new.txt")).unwrap(),
        "written\r\n"
    );
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_cannot_escape_a_scoped_grant() {
    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let base_s = base.to_string_lossy().replace('\\', "/");
    std::fs::create_dir(base.join("allowed")).unwrap();
    std::fs::write(base.join("secret.txt"), "outside").unwrap();
    std::os::unix::fs::symlink(base.join("secret.txt"), base.join("allowed/link.txt")).unwrap();
    std::os::unix::fs::symlink(&base, base.join("allowed/up")).unwrap();
    let policy =
        Policy::deny_all().grant_prefixes(Capability::FileRead, [format!("{base_s}/allowed/")]);
    let src = format!(
        "Public Sub Main()\nOn Error Resume Next\nOpen \"{base_s}/allowed/link.txt\" For Input As #1\nDebug.Print \"file link \" & Err.Number\nErr.Clear\nOpen \"{base_s}/allowed/up/secret.txt\" For Input As #1\nDebug.Print \"dir link \" & Err.Number\nEnd Sub\n"
    );
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(host(policy))
        .vfs(RealFs::in_dir(&base))
        .debug_sink(sink.clone())
        .build_sources(&[std_module(&src)])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "file link 70\ndir link 70\n");
}

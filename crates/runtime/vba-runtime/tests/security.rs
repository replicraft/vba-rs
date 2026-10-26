//! Adversarial file-access cases: path spellings, sibling prefixes, probing
//! without a grant, special files and links on the real file system.

mod common;

use common::*;
use vba_host::{Capability, Policy};
use vba_runtime::*;
use vba_vm::BufferSink;

/// Runs statements with `On Error Resume Next`; `T "label"` prints the label
/// and the error number of the statement before it.
fn trap(body: &str, fs: SharedFs, policy: Policy) -> String {
    let src = format!(
        "Sub T(what As String)\nDebug.Print what & \" \" & Err.Number\nErr.Clear\nEnd Sub\nSub Main()\nOn Error Resume Next\n{body}End Sub\n"
    );
    let o = run(&src, fs, policy);
    assert!(o.result.is_ok(), "{:?}", o.result);
    o.out
}

fn two_dirs() -> SharedFs {
    SharedFs::with(&[("/data/in.txt", "ok"), ("/data-secret/key.txt", "no")])
}

#[test]
fn a_directory_grant_without_a_trailing_slash_does_not_cover_a_sibling() {
    let policy = Policy::deny_all().grant_prefixes(Capability::FileRead, ["/data"]);
    let body = "Open \"/data/in.txt\" For Input As #1\nT \"inside\"\nClose\nOpen \"/data-secret/key.txt\" For Input As #1\nT \"sibling\"\nClose\nOpen \"/data/../data-secret/key.txt\" For Input As #1\nT \"dotdot sibling\"\nClose\n";
    assert_eq!(
        trap(body, two_dirs(), policy),
        "inside 0\nsibling 70\ndotdot sibling 70\n"
    );
}

#[test]
fn hostile_path_spellings_never_reach_outside_the_grant() {
    let fs = SharedFs::with(&[("/data/in.txt", "ok"), ("/secret/key.txt", "no")]);
    let policy = Policy::deny_all().grant_prefixes(Capability::FileRead, ["/data/"]);
    let attempts = [
        "//data/../secret/key.txt",
        "/data/./../secret/key.txt",
        "/data /../secret/key.txt",
        "/data/../../secret/key.txt",
        "\\\\?\\/secret/key.txt",
        "\\\\server\\share\\secret\\key.txt",
        "C:\\secret\\key.txt",
        "c:/secret/key.txt",
        "/data/\\..\\secret\\key.txt",
        "/data/%2e%2e/secret/key.txt",
        "/SECRET/KEY.TXT",
        "/secret/key.txt.",
        "/secret/key.txt ",
        "/data/in.txt/../../secret/key.txt",
    ];
    let mut body = String::new();
    for (i, a) in attempts.iter().enumerate() {
        let lit = a.replace('"', "\"\"");
        body.push_str(&format!(
            "Open \"{lit}\" For Input As #1\nT \"{i}\"\nClose\n"
        ));
    }
    // A NUL byte and a very long name.
    body.push_str(
        "Open \"/data/\" & Chr(0) & \"/../secret/key.txt\" For Input As #1\nT \"nul\"\nClose\n",
    );
    body.push_str("Open \"/data/\" & String$(100000, \"a\") For Input As #1\nT \"long\"\nClose\n");
    let out = trap(&body, fs, policy);
    for line in out.lines() {
        let n: i32 = line.rsplit(' ').next().unwrap().parse().unwrap();
        assert!(
            matches!(n, 52 | 53 | 70 | 75 | 76),
            "a hostile spelling was accepted or misreported: {line}\n{out}"
        );
    }
    assert_eq!(out.lines().count(), attempts.len() + 2);
}

#[test]
fn kill_and_dir_do_not_reveal_files_outside_the_grant() {
    let fs = SharedFs::with(&[("/secret/key.txt", "no")]);
    let body = "Kill \"/secret/key.txt\"\nT \"exact hit\"\nKill \"/secret/nothing.txt\"\nT \"exact miss\"\nKill \"/secret/k*\"\nT \"wild hit\"\nKill \"/secret/z*\"\nT \"wild miss\"\nKill \"/nodir/z*\"\nT \"no dir\"\nDebug.Print Dir(\"/secret/*\")\nT \"dir\"\n";
    let out = trap(body, fs.clone(), Policy::deny_all());
    assert_eq!(
        out,
        "exact hit 70\nexact miss 70\nwild hit 70\nwild miss 70\nno dir 70\ndir 70\n"
    );
    assert!(fs.text("/secret/key.txt").is_some());
}

#[test]
fn read_limited_refuses_a_file_over_the_limit_in_every_file_system() {
    let mut mem = MemFs::new().with_file("/a.txt", b"0123456789");
    assert_eq!(mem.read_limited("/a.txt", 10), Ok(b"0123456789".to_vec()));
    assert_eq!(mem.read_limited("/a.txt", 9), Err(VfsError::TooLarge));
}

#[test]
fn a_record_longer_than_any_file_is_refused_before_it_is_buffered() {
    let fs = SharedFs::with(&[]);
    let sink = BufferSink::new();
    let h = host(all_files());
    let mut s = Session::builder()
        .host(h.clone())
        .vfs(fs)
        .debug_sink(sink.clone())
        .build_sources(&[std_module(
            "Public Sub Main()\nDim b As String * 3\nb = \"abc\"\nOn Error Resume Next\nOpen \"/f\" For Random As #1 Len = 2000000000\nDebug.Print Err.Number\nPut #1, 1, b\nDebug.Print Err.Number\nEnd Sub\n",
        )])
        .unwrap();
    s.file_system().borrow_mut().set_max_file_bytes(1 << 20);
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    // 59 "Bad record length" at Open; the Put then has no open file (52).
    assert_eq!(
        sink.contents().split_whitespace().collect::<Vec<_>>(),
        ["59", "52"]
    );
}

#[cfg(unix)]
mod real {
    use super::*;
    use std::fs;

    fn session(base: &std::path::Path, policy: Policy, src: &str) -> (Session, BufferSink) {
        let sink = BufferSink::new();
        let mut s = Session::builder()
            .host(host(policy))
            .vfs(RealFs::in_dir(base))
            .debug_sink(sink.clone())
            .build_sources(&[std_module(src)])
            .unwrap();
        s.trust();
        (s, sink)
    }

    #[test]
    fn a_dangling_link_cannot_be_written_through_out_of_the_grant() {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        let b = base.to_string_lossy().replace('\\', "/");
        fs::create_dir(base.join("allowed")).unwrap();
        fs::create_dir(base.join("outside")).unwrap();
        let target = base.join("outside/created.txt");
        std::os::unix::fs::symlink(&target, base.join("allowed/dangling.txt")).unwrap();
        // A relative dangling link, one hop away.
        std::os::unix::fs::symlink("../outside/rel.txt", base.join("allowed/rel.txt")).unwrap();
        let policy =
            Policy::deny_all().grant_prefixes(Capability::FileWrite, [format!("{b}/allowed/")]);
        let src = format!(
            "Public Sub Main()\nOn Error Resume Next\nOpen \"{b}/allowed/dangling.txt\" For Output As #1\nDebug.Print \"abs \" & Err.Number\nErr.Clear\nClose\nOpen \"{b}/allowed/rel.txt\" For Output As #1\nDebug.Print \"rel \" & Err.Number\nEnd Sub\n"
        );
        let (mut s, sink) = session(&base, policy, &src);
        s.run_procedure("Main", vec![]).unwrap();
        assert_eq!(sink.contents(), "abs 70\nrel 70\n");
        assert!(!target.exists());
        assert!(!base.join("outside/rel.txt").exists());
    }

    #[test]
    fn devices_are_refused_instead_of_read_to_exhaustion() {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        let b = base.to_string_lossy().replace('\\', "/");
        let src = format!(
            "Public Sub Main()\nOn Error Resume Next\nOpen \"/dev/zero\" For Binary As #1\nDebug.Print \"open \" & (Err.Number <> 0)\nErr.Clear\nFileCopy \"/dev/zero\", \"{b}/zero.bin\"\nDebug.Print \"copy \" & (Err.Number <> 0)\nEnd Sub\n"
        );
        let (mut s, sink) = session(&base, Policy::allow_all(), &src);
        s.run_procedure("Main", vec![]).unwrap();
        assert_eq!(sink.contents(), "open True\ncopy True\n");
        assert!(
            !base.join("zero.bin").exists()
                || fs::metadata(base.join("zero.bin")).unwrap().len() == 0
        );
    }

    #[test]
    fn the_real_file_system_reads_at_most_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        fs::write(base.join("f"), vec![1u8; 100]).unwrap();
        let mut real = RealFs::in_dir(&base);
        let p = format!("{}/f", base.to_string_lossy());
        assert_eq!(real.read_limited(&p, 100).map(|d| d.len()), Ok(100));
        assert_eq!(real.read_limited(&p, 99), Err(VfsError::TooLarge));
        // A device reports no size and never ends; it is not a file.
        assert_eq!(real.read_limited("/dev/zero", 10), Err(VfsError::Denied));
        assert_eq!(real.read("/dev/zero"), Err(VfsError::Denied));
    }

    #[test]
    fn a_large_real_file_is_refused_at_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        let b = base.to_string_lossy().replace('\\', "/");
        fs::write(base.join("big.bin"), vec![7u8; 1 << 20]).unwrap();
        let src = format!(
            "Public Sub Main()\nOn Error Resume Next\nOpen \"{b}/big.bin\" For Binary As #1\nDebug.Print Err.Number\nEnd Sub\n"
        );
        let (mut s, sink) = session(&base, all_files(), &src);
        s.file_system().borrow_mut().set_max_file_bytes(1000);
        s.run_procedure("Main", vec![]).unwrap();
        assert_eq!(sink.contents().trim(), "61");
    }
}

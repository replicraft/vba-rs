//! Several file numbers on one file: shared contents, `Lock` clauses of
//! `Open` and the ranges of `Lock` / `Unlock`. Every program runs against the
//! in-memory file system and against a directory on disk, with the same
//! expected output.

mod common;

use common::*;
use vba_runtime::{RealFs, Session};
use vba_vm::BufferSink;

/// Runs `src` (with `{p}` replaced by the directory the program may use)
/// against both file systems and returns the two outputs.
fn both(src: &str) -> (String, String) {
    let mem = run(&src.replace("{p}", "/w"), SharedFs::default(), all_files()).out;
    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let b = base.to_string_lossy().replace('\\', "/");
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(host(all_files()))
        .vfs(RealFs::in_dir(&base))
        .debug_sink(sink.clone())
        .build_sources(&[std_module(&src.replace("{p}", &b))])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    (mem, sink.contents())
}

fn check(src: &str, expected: &str) {
    let (mem, real) = both(src);
    assert_eq!(mem, expected, "in-memory file system");
    assert_eq!(real, expected, "file system on disk");
}

fn program(body: &str) -> String {
    format!("Public Sub Main()\nOn Error Resume Next\nMkDir \"{{p}}\"\nErr.Clear\n{body}End Sub\n")
}

#[test]
fn binary_channels_share_the_contents() {
    check(
        &program(
            "Dim a As Byte, b As Byte\n\
             Open \"{p}/s.bin\" For Binary As #1\n\
             Open \"{p}/s.bin\" For Binary Access Read As #2\n\
             a = 7\nPut #1, 1, a\n\
             Get #2, 1, b\n\
             Debug.Print CStr(b) & \",\" & LOF(2) & \",\" & Err.Number\n\
             Close\n",
        ),
        "7,1,0\n",
    );
}

#[test]
fn lock_clauses_deny_the_access_they_name() {
    // MS-VBAL 5.4.5.1: a lock the system cannot provide is error 70.
    check(
        &program(
            "Open \"{p}/s.bin\" For Binary Lock Write As #1\n\
             Open \"{p}/s.bin\" For Binary Access Write As #2\n\
             Debug.Print \"w \" & Err.Number: Err.Clear\n\
             Open \"{p}/s.bin\" For Binary Access Read As #3\n\
             Debug.Print \"r \" & Err.Number: Err.Clear\n\
             Close\n\
             Open \"{p}/s.bin\" For Binary Lock Read Write As #1\n\
             Open \"{p}/s.bin\" For Binary Access Read As #2\n\
             Debug.Print \"rw \" & Err.Number: Err.Clear\n\
             Close\n\
             Open \"{p}/s.bin\" For Binary Shared As #1\n\
             Open \"{p}/s.bin\" For Binary As #2\n\
             Debug.Print \"shared \" & Err.Number\n\
             Close\n",
        ),
        "w 70\nr 0\nrw 70\nshared 0\n",
    );
}

#[test]
fn a_sequential_writer_needs_the_file_to_itself() {
    check(
        &program(
            "Open \"{p}/o.txt\" For Output As #1\n\
             Open \"{p}/o.txt\" For Input As #2\n\
             Debug.Print CStr(Err.Number)\n\
             Close\n",
        ),
        "55\n",
    );
}

#[test]
fn record_locks_of_one_channel_block_the_others() {
    check(
        &program(
            "Dim v As Long\n\
             Open \"{p}/r.dat\" For Random As #1 Len = 4\n\
             Open \"{p}/r.dat\" For Random As #2 Len = 4\n\
             v = 1\nPut #1, 1, v\nPut #1, 2, v\nPut #1, 3, v\n\
             Lock #1, 2 To 3\n\
             Get #2, 1, v: Debug.Print \"1:\" & Err.Number: Err.Clear\n\
             Get #2, 2, v: Debug.Print \"2:\" & Err.Number: Err.Clear\n\
             Put #2, 3, v: Debug.Print \"3:\" & Err.Number: Err.Clear\n\
             Get #1, 2, v: Debug.Print \"own:\" & Err.Number: Err.Clear\n\
             Unlock #1, 2 To 3\n\
             Get #2, 2, v: Debug.Print \"free:\" & Err.Number: Err.Clear\n\
             Lock #1\n\
             Get #2, 1, v: Debug.Print \"whole:\" & Err.Number: Err.Clear\n\
             Close #1\n\
             Get #2, 1, v: Debug.Print \"closed:\" & Err.Number\n\
             Close\n",
        ),
        "1:0\n2:70\n3:70\nown:0\nfree:0\nwhole:70\nclosed:0\n",
    );
}

#[test]
fn byte_locks_of_one_channel_block_the_others() {
    check(
        &program(
            "Dim b As Byte\n\
             Open \"{p}/b.bin\" For Binary As #1\n\
             Open \"{p}/b.bin\" For Binary As #2\n\
             b = 1\nPut #1, 1, b\nPut #1, 2, b\nPut #1, 3, b\n\
             Lock #1, 2 To 2\n\
             Get #2, 1, b: Debug.Print \"1:\" & Err.Number: Err.Clear\n\
             Get #2, 2, b: Debug.Print \"2:\" & Err.Number: Err.Clear\n\
             Close\n",
        ),
        "1:0\n2:70\n",
    );
}

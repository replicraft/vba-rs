//! Very long `ElseIf` and `Case` chains are analysed, lowered and run without native recursion
//! proportional to the number of clauses.

use vba::{Engine, ModuleKind, Project};

const SMALL_STACK: usize = 256 * 1024;
const CLAUSES: usize = 100_000;

fn elseif_chain(n: usize) -> String {
    let mut s =
        String::from("Public Function Pick(ByVal x As Long) As Long\nIf x = 0 Then\nPick = 0\n");
    for i in 1..n {
        s.push_str(&format!("ElseIf x = {i} Then\nPick = {i}\n"));
    }
    s.push_str("Else\nPick = -1\nEnd If\nEnd Function\n");
    s.push_str("Public Sub Main()\nDebug.Print Pick(5)\nDebug.Print Pick(-7)\nEnd Sub\n");
    s
}

fn case_chain(n: usize) -> String {
    let mut s = String::from("Public Function Pick(ByVal x As Long) As Long\nSelect Case x\n");
    for i in 0..n {
        s.push_str(&format!("Case {i}\nPick = {i}\n"));
    }
    s.push_str("Case Else\nPick = -1\nEnd Select\nEnd Function\n");
    s.push_str("Public Sub Main()\nDebug.Print Pick(5)\nDebug.Print Pick(-7)\nEnd Sub\n");
    s
}

fn run_on_small_stack(src: String) -> String {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(move || {
            let mut p = Project::new();
            p.add_module("Module1", ModuleKind::Standard, &src).unwrap();
            let mut inst = Engine::new().load(&p).unwrap();
            inst.trust();
            inst.run("Main", vec![]).unwrap().output
        })
        .unwrap()
        .join()
        .expect("the thread must not overflow its stack")
}

#[test]
fn a_very_long_elseif_chain_does_not_recurse_per_clause() {
    assert_eq!(run_on_small_stack(elseif_chain(CLAUSES)), " 5 \n-1 \n");
}

#[test]
fn a_very_long_case_list_does_not_recurse_per_item() {
    let items: Vec<String> = (0..CLAUSES).map(|i| i.to_string()).collect();
    let src = format!(
        "Public Sub Main()\nSelect Case 5\nCase {}\nDebug.Print \"hit\"\nEnd Select\nEnd Sub\n",
        items.join(", ")
    );
    assert_eq!(run_on_small_stack(src), "hit\n");
}

#[test]
fn a_very_long_case_chain_does_not_recurse_per_clause() {
    assert_eq!(run_on_small_stack(case_chain(CLAUSES)), " 5 \n-1 \n");
}

//! `max_memory_bytes` bounds what a program holds: strings, arrays of
//! strings, collection items, user-defined-type fields, and the temporary
//! copies alive at one moment. Memory comes back when values are dropped.

mod common;

use std::rc::Rc;

use common::*;
use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_project::ModuleKind;
use vba_vm::{LimitKind, NoServices, VmError};

fn limited(max_memory: u64) -> ResourceLimits {
    ResourceLimits {
        max_steps: Some(50_000_000),
        max_memory_bytes: Some(max_memory),
        ..ResourceLimits::default()
    }
}

fn hits_the_memory_limit(src: &str, max_memory: u64) {
    let o = run_limited(src, limited(max_memory));
    assert_eq!(
        o.result.as_ref().err(),
        Some(&VmError::Limit(LimitKind::Memory)),
        "output: {:?}",
        o.out
    );
}

#[test]
fn many_large_strings_in_an_array_hit_the_limit() {
    // 60 strings of 2 MB under a 10 MB limit (the shape of 60 x 60 MB under 100 MB).
    hits_the_memory_limit(
        "Sub Main()\nDim a(1 To 60) As String, i As Long\nFor i = 1 To 60\na(i) = String$(1000000, \"x\")\nNext\nDebug.Print \"held\"\nEnd Sub\n",
        10_000_000,
    );
}

#[test]
fn strings_added_to_a_collection_hit_the_limit() {
    hits_the_memory_limit(
        "Sub Main()\nDim c As New Collection, i As Long\nFor i = 1 To 60\nc.Add String$(1000000, \"x\")\nNext\nDebug.Print c.Count\nEnd Sub\n",
        10_000_000,
    );
}

#[test]
fn strings_in_a_variant_array_hit_the_limit() {
    hits_the_memory_limit(
        "Sub Main()\nDim a() As Variant, i As Long\nReDim a(1 To 60)\nFor i = 1 To 60\na(i) = String$(1000000, \"x\")\nNext\nEnd Sub\n",
        10_000_000,
    );
}

#[test]
fn strings_in_user_defined_type_fields_hit_the_limit() {
    hits_the_memory_limit(
        "Private Type T\nName As String\nEnd Type\nSub Main()\nDim a(1 To 60) As T, i As Long\nFor i = 1 To 60\na(i).Name = String$(1000000, \"x\")\nNext\nEnd Sub\n",
        10_000_000,
    );
}

#[test]
fn module_level_strings_hit_the_limit() {
    hits_the_memory_limit(
        "Private g(1 To 60) As String\nSub Main()\nDim i As Long\nFor i = 1 To 60\ng(i) = Space$(1000000)\nNext\nEnd Sub\n",
        10_000_000,
    );
}

#[test]
fn a_growing_string_hits_the_limit_before_the_string_limit() {
    hits_the_memory_limit(
        "Sub Main()\nDim s As String, i As Long\ns = Space$(1000)\nFor i = 1 To 40\ns = s & s\nNext\nEnd Sub\n",
        1_000_000,
    );
}

#[test]
fn many_arguments_cannot_multiply_one_string_inside_one_statement() {
    let args = vec!["s"; 200].join(", ");
    let src = format!(
        "Sub Main()\nDim s As String, a As Variant\ns = Space$(2000000)\na = Array({args})\nEnd Sub\n"
    );
    hits_the_memory_limit(&src, 10_000_000);
}

#[test]
fn a_variant_array_is_charged_for_its_elements() {
    hits_the_memory_limit(
        "Sub Main()\nDim a() As Variant\nReDim a(1 To 3000000)\nEnd Sub\n",
        20_000_000,
    );
}

#[test]
fn the_memory_limit_is_not_trappable() {
    hits_the_memory_limit(
        "Sub Main()\nOn Error Resume Next\nDim a(1 To 60) As String, i As Long\nFor i = 1 To 60\na(i) = String$(1000000, \"x\")\nNext\nDebug.Print \"survived\"\nEnd Sub\n",
        10_000_000,
    );
}

#[test]
fn memory_returns_when_values_are_dropped() {
    // 300 strings of 400 KB are built and dropped one after the other: the
    // peak is a few strings, far below the 5 MB limit.
    let host = Rc::new(HeadlessHost::new().with_limits(limited(5_000_000)));
    let o = run_with(
        &[module(
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim s As String, i As Long, n As Long\nFor i = 1 To 300\ns = String$(200000, \"x\")\nn = n + Len(s)\ns = \"\"\nNext\nDebug.Print n\nEnd Sub\n",
        )],
        "Main",
        vec![],
        host.clone(),
        Box::new(NoServices),
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert_eq!(o.out, " 60000000 \n");
    assert!(
        host.budget().memory_used() < 1_000_000,
        "left behind: {}",
        host.budget().memory_used()
    );
}

#[test]
fn a_released_array_and_collection_return_their_memory() {
    let host = Rc::new(HeadlessHost::new().with_limits(limited(50_000_000)));
    let o = run_with(
        &[module(
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim i As Long\nFor i = 1 To 20\nDim a() As String\nReDim a(1 To 100000)\na(1) = Space$(100000)\nErase a\nDim c As New Collection\nc.Add Space$(200000)\nSet c = Nothing\nNext\nEnd Sub\n",
        )],
        "Main",
        vec![],
        host.clone(),
        Box::new(NoServices),
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert!(
        host.budget().memory_used() < 1_000_000,
        "left behind: {}",
        host.budget().memory_used()
    );
}

#[test]
fn output_kept_by_a_buffer_sink_is_charged() {
    hits_the_memory_limit(
        "Sub Main()\nDim i As Long\nFor i = 1 To 200\nDebug.Print String$(100000, \"x\")\nNext\nEnd Sub\n",
        5_000_000,
    );
}

#[test]
fn saved_settings_are_charged() {
    hits_the_memory_limit(
        "Sub Main()\nDim i As Long\nFor i = 1 To 100000\nSaveSetting \"app\", \"sec\", \"key\" & i, \"value\"\nNext\nEnd Sub\n",
        1_000_000,
    );
}

#[test]
fn small_programs_run_under_a_small_limit() {
    let o = run_limited(
        "Sub Main()\nDim i As Long, s As String\nFor i = 1 To 1000\ns = s & \"x\"\nNext\nDebug.Print Len(s)\nEnd Sub\n",
        limited(100_000),
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert_eq!(o.out, " 1000 \n");
}

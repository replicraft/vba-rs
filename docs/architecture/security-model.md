# Security model

vba-rs runs VBA that comes from an untrusted project file. The model has four parts: imported
code never runs by itself, every effect on the outside world needs an explicit grant, the run is
bounded by limits the program cannot lift, and hostile input files are rejected instead of
crashing the process. This page states what is guaranteed and which test checks it, then what
is not guaranteed. A claim without a test name is not a guarantee.

## Trust gate

* Loading a project compiles it and runs nothing: no `Auto_Open`, no `Workbook_Open`, no
  `Class_Initialize`. Running, raising events and shutting down need the embedder to call
  `trust()` first.
  Tests: `building_a_session_runs_nothing_and_execution_needs_trust` (vba-runtime),
  `nothing_runs_until_trusted`, `class_initialize_does_not_run_before_trust` (vba facade).

## Capabilities

Seven capabilities exist: file read, file write, process spawn, network, COM creation, native
`Declare` and UI automation. The default policy grants none; a grant is unrestricted or limited
to resource prefixes. Denial is error 70 and always happens before the effect.

* Default deny: `everything_is_denied_without_grants` (vba-runtime),
  `deny_all_denies_everything`, `ui_automation_and_clock_are_refused_by_default` (vba-host),
  `declare_needs_native_declare_permission` (vba-vm),
  `grants_are_the_only_way_to_reach_the_file_system` (vba facade).
* The clock, `SendKeys`, `AppActivate` and the registry are not reachable even with a grant
  unless the embedder installs a hook (`ui_automation_and_clock_are_refused_by_default`);
  the clock and `Environ` are injected, not read from the machine
  (`clock_environment_and_command_are_injected`); `Shell` needs a grant and an installed
  launcher (`std_shell_starts_a_real_process_when_installed_and_granted`).
* File grants match whole path segments: `/data` covers `/data` and `/data/x`, not
  `/data-secret` (`file_prefix_grants_match_whole_path_segments`,
  `a_directory_grant_without_a_trailing_slash_does_not_cover_a_sibling`). Grants for other
  capabilities match the text prefix.
* Paths are normalised (`.`, `..`, backslashes, drive letters) before the policy sees them, so
  traversal and alternative spellings do not escape a grant
  (`prefix_grants_scope_access_and_ignore_traversal`,
  `hostile_path_spellings_never_reach_outside_the_grant`). On the real file system the
  policy is checked against the path with symbolic links resolved, including links whose
  target does not exist (`a_symbolic_link_cannot_escape_a_scoped_grant`,
  `a_dangling_link_cannot_be_written_through_out_of_the_grant`,
  `the_real_file_system_is_scoped_by_the_same_policy`). Both ends of `FileCopy` and `Name` are
  checked (`copying_and_renaming_check_both_ends`).
* A denied `Kill` or `Dir` reveals nothing about files outside the grant
  (`kill_and_dir_do_not_reveal_files_outside_the_grant`). Devices, pipes and sockets are refused
  by the real file system (`devices_are_refused_instead_of_read_to_exhaustion`).
* A `Declare` is checked against `library!procedure` as written and as the provider will resolve
  it (`a_scoped_declare_grant_cannot_be_smuggled_into_another_library`). The Wine bridge
  re-checks every call (`provider_scoped_grant_limits_symbols`).
* `Shell` through `StdShell` starts one program with arguments; shell syntax in the command line
  has no effect outside Windows (`shell_syntax_is_not_interpreted`).
* The only `unsafe` code is the `native-call` feature of vba-ffi and the Windows module of
  vba-wine-host; every other crate carries `unsafe_code = "forbid"`. Without the feature no
  library is loaded, and with it a host-issued `NativePermission` is still required
  (`a_callee_that_rewrites_a_string_cell_is_an_error`, `memory_bounds_and_pointers`).

## Limits

`ResourceLimits` bounds steps, memory and call depth; the VM bounds the length of one string and
the size of one array. The defaults of `ResourceLimits` are unlimited steps and memory: an
embedder that runs untrusted code must set them.

* Steps stop loops, trappable or not (`step_limit_stops_runaway_loops_and_is_not_trappable`).
  Cancellation from another thread is observed only at `DoEvents`; a loop without it is stopped
  by the step limit alone
  (`cancellation_is_observed_only_at_do_events_and_the_step_limit_stops_the_rest`).
* Memory is charged for arrays, collections and open files
  (`memory_limit_is_charged_by_arrays`, `collection_growth_is_charged_to_the_memory_limit`,
  `open_files_are_charged_to_the_memory_budget_and_released_on_close`,
  `memory_and_size_limits_apply_to_open_files`).
* Oversized results are refused before they are built: `String$`, `Space$`, `Replace`, `Join`,
  `Split` (`builders_fail_with_error_7_before_building_an_oversized_string`), concatenation
  (`oversized_arrays_and_strings_are_refused`), file reads (`the_real_file_system_reads_at_most_the_limit`)
  and `Random` records (`a_record_longer_than_any_file_is_refused_before_it_is_buffered`).
* Recursion ends in a limit error, not a native stack overflow: procedure calls, events,
  `Class_Initialize`, late-bound calls, releasing long object and collection chains and nested
  user-defined types (`late_bound_recursion_is_bounded_by_the_call_depth`,
  `recursion_through_events_is_bounded`, `recursion_through_class_initialize_is_bounded`,
  `long_ownership_chains_are_released_iteratively`,
  `long_chains_of_collections_are_released_iteratively`,
  `user_defined_types_nested_absurdly_deep_are_a_limit_not_a_stack_overflow`).

## Hostile input

Project files, containers, streams, type libraries, p-code caches and Wine protocol frames are
parsed with size, count, ratio and depth limits checked before allocation; malformed input is an
error, never a panic.

* Containers and archives: `damaged_documents_never_panic`, `truncated_documents_never_panic`,
  `arbitrary_bytes_never_panic`, `limits_are_respected`, `zip_bombs_hit_the_ratio_limit`,
  `decompression_bombs_are_stopped_by_the_ratio_limit` (vba-format).
* Type libraries: `every_truncation_is_an_error_or_a_valid_library`,
  `every_single_byte_corruption_is_survivable`, `huge_counts_are_rejected_cheaply`,
  `type_description_cycle_is_cut_by_the_depth_limit` (vba-typelib).
* P-code: `mutated_valid_cache_never_panics`, `module_parse_never_panics_on_noise` (vba-pcode).
* Wine protocol: `hostile_counts_do_not_allocate_or_panic`, `pseudo_random_garbage_never_panics`.
* Source: `hostile_nesting_is_reported_not_overflowed`, `long_but_ordinary_chains_still_parse`
  (vba-syntax; nesting is limited to 200 and a chain of operators, member accesses or call
  suffixes to 512), `structured_programs_never_panic`, `token_soup_never_panics` (vba-sema).

## Not guaranteed

* **Memory accounting is incomplete.** Strings, `Scripting.Dictionary` items and the contents of
  an in-memory file system are not charged to `max_memory_bytes`. A program can hold gigabytes
  of strings under a small limit (`strings_held_by_a_program_are_not_charged_to_the_memory_limit`). Use an operating-system limit (cgroup, `ulimit`, a separate
  process) for hard containment.
* **Steps are not CPU time.** One step can be a scan of a 32-million-unit string (`Like`, `InStr`,
  `Replace`) or a deep copy of a nested array. The step limit bounds statements, not work.
* **Cancellation** needs `DoEvents` (see above).
* **Scoped grants for process spawn, COM creation and native declares compare text prefixes.**
  A grant `notepad` also covers `notepad-evil.exe`. Grant exact names.
* **File grants are textual and case-insensitive.** On a case-sensitive file system `/data/`
  also admits `/DATA/`. The check and the use are two steps, so another process that swaps a
  link in between can defeat the symbolic-link resolution.
* **Windows paths are untested here:** reserved device names, short names, alternate data
  streams and `\\?\` prefixes are not handled specially.
* `CurDir` returns the host working directory of the real file system; `GetObject(path)` is
  checked against the path as written, not the resolved path.
* Very long `ElseIf`/`Case` chains (hundreds of thousands of clauses) and some quadratic
  compile-time shapes (tens of thousands of labels or user-defined types) cost stack or time.
* The Wine helper executes native code of the library the embedder exports; it is not a sandbox.
* `Session::vm()` gives an embedder the interpreter without the trust gate.

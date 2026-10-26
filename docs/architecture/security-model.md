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
* Scoped grants are matched per capability, never as a text prefix (`scope.rs` in
  vba-host has the table). Files: whole path segments, `/data` covers `/data` and
  `/data/x`, not `/data-secret` (`file_prefix_grants_match_whole_path_segments`,
  `a_directory_grant_without_a_trailing_slash_does_not_cover_a_sibling`). The
  comparison follows the file system the path is written for: a POSIX path is
  compared exactly, case included (`posix_paths_are_case_sensitive`); a drive
  path (`c:/work`) and a network path (`//server/share`) are compared as Windows
  does, with case folded and trailing dots and spaces dropped, and a resource
  never matches when one of its segments is only dots and spaces (a parent
  reference in disguise), names a stream (`a:b`), a device (`NUL`, `COM1`) or a
  short name (`PROGRA~1`), or when it is in the `//?/` or `//./` namespace
  (`windows_names_that_the_system_rewrites_never_match_by_accident`,
  `unc_and_device_namespaces_need_their_own_scope`,
  `windows_spellings_never_reach_outside_a_drive_grant`). Network paths keep
  their server and share when normalised and `..` cannot climb above the share
  (`unc_paths_keep_their_server_and_share`), so a UNC path is never confused with a
  local one. A file system that ignores case (`MemFs`) reports the spelling its
  entries were created with from `Vfs::canonical`, so the exact comparison sees
  one name for one file.
  Processes: the program (first word of the command line, read by the same reader
  the launcher uses) is the scope, the scope plus `.exe`, or below the scope as a
  directory; `notepad` never admits `notepad-x.exe` or a later word
  (`a_process_grant_names_the_program`, `a_program_grant_does_not_admit_other_programs`).
  ProgIDs: exact, or `Namespace.*` / `Namespace.` for whole dot segments below
  the namespace, `*` for all; a bare `Scripting` admits nothing but `Scripting`
  (`a_progid_grant_is_exact_or_a_namespace`). Declares: `library!procedure` or
  `library!*`, with the library normalised like the Win32 provider (directory
  and `.dll` dropped, case folded, judged by the last `!`)
  (`a_declare_grant_is_a_procedure_or_a_whole_library`). UI automation: the
  action (`SendKeys`, `AppActivate`) or `AppActivate:title`
  (`ui_automation_grants_name_actions`, `app_activate_is_checked_as_action_and_title`).
  Network: a host, `host:port` or `*.domain` (`network_grants_name_hosts`). Any
  other spelling matches nothing, and `vbars run --grant` rejects malformed
  scopes (`malformed_scopes_are_reported_by_validation`).
* Paths are normalised (`.`, `..`, backslashes, drive letters) before the policy sees them, so
  traversal and alternative spellings do not escape a grant
  (`prefix_grants_scope_access_and_ignore_traversal`,
  `hostile_path_spellings_never_reach_outside_the_grant`). On the real file system the
  policy is checked against the path with symbolic links resolved, including links whose
  target does not exist (`a_symbolic_link_cannot_escape_a_scoped_grant`,
  `a_dangling_link_cannot_be_written_through_out_of_the_grant`,
  `the_real_file_system_is_scoped_by_the_same_policy`). The path that was checked is
  the path that is used, and `RealFs` refuses a path that no longer resolves to
  itself, so a link swapped in between the check and the use is not followed:
  every operation re-resolves its path (the directory that holds it, for the
  operations that act on a final link without following it)
  (`a_link_swapped_in_after_the_check_is_not_followed`). On Unix the operation
  then walks the path from `/` one directory at a time with `O_NOFOLLOW` and acts
  relative to the directory handle it opened (`openat`, `unlinkat`, `mkdirat`,
  `renameat`; the `rustix` crate, safe API): `Kill`, `Name`, `MkDir`, `RmDir`,
  `FileCopy`, `SetAttr` and the open of a file for `Input`, `Output` and the
  other modes. A parent directory swapped for a link at any point is refused or
  already behind the handle, so the call lands on the directory that was
  verified (`a_parent_swapped_after_the_check_does_not_redirect_an_operation`,
  `an_operation_acts_on_the_directory_that_was_verified`). A file is opened
  without following a final link, so a write through a name swapped for a link
  creates nothing behind it, and no empty file appears at the new place before
  a handle check (`a_write_through_a_swapped_name_creates_nothing_outside`,
  `a_swapped_parent_leaves_no_stray_empty_file`). Both ends of `FileCopy` and
  `Name` are checked (`copying_and_renaming_check_both_ends`).
* `GetObject(path)` is made absolute against the working directory and resolved
  through links before the policy sees it, and the host is handed the resolved
  path; a denial names the capability only
  (`get_object_is_checked_against_the_resolved_path`,
  `get_object_resolves_a_relative_path_against_the_working_directory`).
* `CurDir` and the text of a denial do not reveal a start directory the policy does not
  cover: the program sees `/` instead, and a denial names the capability but not the
  resolved path. A directory it may read, or one it entered with `ChDir`, is shown
  (`curdir_does_not_reveal_the_start_directory_without_a_grant`,
  `curdir_is_reported_where_the_directory_may_be_read`).
* A denied `Kill` or `Dir` reveals nothing about files outside the grant
  (`kill_and_dir_do_not_reveal_files_outside_the_grant`). Devices, pipes and sockets are refused
  by the real file system (`devices_are_refused_instead_of_read_to_exhaustion`).
* A `Declare` is checked against `library!procedure` as written and as the provider will resolve
  it (`a_scoped_declare_grant_cannot_be_smuggled_into_another_library`). The Wine bridge
  re-checks every call (`provider_scoped_grant_limits_symbols`).
* `Shell` through `StdShell` starts one program with arguments, directly on every
  platform (no `sh -c`, no `cmd /C`); shell syntax in the command line has no
  effect (`shell_syntax_is_not_interpreted`).
* The only `unsafe` code is the `native-call` feature of vba-ffi and the Windows module of
  vba-wine-host; every other crate carries `unsafe_code = "forbid"`. Without the feature no
  library is loaded, and with it a host-issued `NativePermission` is still required
  (`a_callee_that_rewrites_a_string_cell_is_an_error`, `memory_bounds_and_pointers`).

## Limits

`ResourceLimits` bounds steps, memory and call depth; the VM bounds the length of one string and
the size of one array. `ResourceLimits::default()` and the lower-level crates are unlimited and
leave the choice to the embedder. The facade's `Engine::new()` and `vbars run` start from
`ResourceLimits::untrusted()`: 100 million steps (about half a minute of ordinary statements),
256 MiB, 1000 nested calls, strings of at most 32 million UTF-16 units and arrays of at most
16.7 million elements; `Engine::unlimited()` and `vbars run --unlimited` opt out
(`engine_defaults_are_finite_and_unlimited_opts_out`).

* **Memory.** One `MemoryMeter` per run is shared by every owner of data a program can grow.
  Strings of 32 UTF-16 units or more and arrays register their capacity with the meter that is
  current on the creating thread and credit it when dropped, wherever that happens; objects,
  user-defined types, collection items, open files, `Scripting.Dictionary` entries, in-memory file
  system entries, `Debug.Print` buffers, saved settings, emulated native memory and handles
  charge explicitly and credit on release. Small strings are covered by the per-element charge of
  the container that holds them. The count is conservative: it includes the temporary copies that
  are alive (the interpreter copies values into registers), so a program needs a margin over its
  logical size. A charge made by code that cannot fail (building a string) cannot be refused, so
  the interpreter compares the meter with the limit before every instruction and ends the run with
  the untrappable memory fault; the overshoot is what one instruction can build (a few strings of
  at most `max_string_units`). Tests: `many_large_strings_in_an_array_hit_the_limit`,
  `strings_added_to_a_collection_hit_the_limit`, `strings_in_a_variant_array_hit_the_limit`,
  `strings_in_user_defined_type_fields_hit_the_limit`, `module_level_strings_hit_the_limit`,
  `a_growing_string_hits_the_limit_before_the_string_limit`,
  `many_arguments_cannot_multiply_one_string_inside_one_statement`,
  `a_variant_array_is_charged_for_its_elements`, `the_memory_limit_is_not_trappable`,
  `output_kept_by_a_buffer_sink_is_charged`, `saved_settings_are_charged`,
  `memory_returns_when_values_are_dropped`, `a_released_array_and_collection_return_their_memory`
  (vba-vm `memory.rs`); `files_kept_in_the_memory_file_system_are_charged`,
  `dictionary_items_are_charged`, `the_memory_file_system_gives_memory_back_when_files_are_deleted`
  (vba-runtime `memory.rs`); `emulated_memory_and_handles_are_charged_to_the_run_meter` (vba-ffi);
  plus the older `memory_limit_is_charged_by_arrays`,
  `collection_growth_is_charged_to_the_memory_limit`,
  `open_files_are_charged_to_the_memory_budget_and_released_on_close`,
  `memory_and_size_limits_apply_to_open_files`,
  `strings_held_by_a_program_are_charged_to_the_memory_limit`.
* **Steps measure work, not only statements.** Every IR instruction is one step. Building data also
  costs steps: every 1024 bytes charged to the meter (plus a fixed cost per allocation) is one
  step, so copying a string or an array, concatenating or building many small arrays is paid for
  in proportion. Built-ins that scan text without building it (`InStr`, `InStrRev`, `Replace`,
  `Split`, `Join`, `Filter`, `StrComp`, `StrConv`, `Format`) charge the length of their text
  arguments through `Ctx::charge` before they start, 64 UTF-16 units to a step, and searching is
  linear-time whatever the pattern; `Like` reports its comparisons while it runs, so a pattern
  designed to backtrack is cut off by the step limit. Tests (vba-vm `work.rs`):
  `a_pathological_like_pattern_runs_into_the_step_limit`,
  `like_costs_steps_in_proportion_to_the_work`, `instr_with_a_long_needle_is_linear_and_charged`,
  `replace_split_and_join_on_large_strings_cost_steps`, `copying_large_strings_costs_steps`,
  `copying_large_arrays_costs_steps`, `small_inputs_cost_next_to_nothing`;
  `linear_search_agrees_with_the_plain_loop` and `a_long_needle_in_a_long_text_is_fast`
  (vba-stdlib).
* Steps stop loops, trappable or not (`step_limit_stops_runaway_loops_and_is_not_trappable`).
* **Cancellation** is polled through `Host::poll_cancel` every 1024 steps, in loops and calls alike
  and not only at `DoEvents`; with no token raised nothing changes
  (`cancellation_is_observed_in_loops_without_do_events`,
  `an_endless_loop_is_cancelled_from_another_thread`). A host that implements cancellation only
  in `idle` is not polled; implement `poll_cancel` too.
* Oversized results are refused before they are built: `String$`, `Space$`, `Replace`, `Join`,
  `Split` (`builders_fail_with_error_7_before_building_an_oversized_string`), concatenation
  (`oversized_arrays_and_strings_are_refused`), file reads (`the_real_file_system_reads_at_most_the_limit`)
  and `Random` records (`a_record_longer_than_any_file_is_refused_before_it_is_buffered`).
* **Native nesting has one budget.** Every nested run of the interpreter (an initializer
  inside an initializer, an event handler, a default-member coercion, a late-bound call
  and a run started from a re-entrant host call) goes through the same entry, which
  counts the nesting (`VmConfig::max_native_depth`, 48) and measures the native stack
  between the outermost run and the current one
  (`VmConfig::max_native_stack_bytes`, 1 MiB); the first limit hit ends the run with the
  call-depth limit error, whatever `max_call_depth` says. The count alone was not enough:
  a level costs about 70 KiB in a debug build and a different amount in each path
  (`initializer_recursion_stays_inside_a_small_stack`,
  `terminator_recursion_stays_inside_a_small_stack`,
  `default_member_recursion_stays_inside_a_small_stack`,
  `late_bound_and_event_recursion_share_the_budget`). `vbars run` compiles and runs on a
  thread with 64 MiB of stack, so the smaller main thread of some systems does not
  matter; an embedder that runs on a smaller thread lowers the budget.
* Recursion ends in a limit error, not a native stack overflow: procedure calls, events,
  `Class_Initialize`, late-bound calls, releasing long object and collection chains and nested
  user-defined types (`late_bound_recursion_is_bounded_by_the_call_depth`,
  `recursion_through_events_is_bounded`, `recursion_through_class_initialize_is_bounded`,
  `long_ownership_chains_are_released_iteratively`,
  `long_chains_of_collections_are_released_iteratively`,
  `user_defined_types_nested_absurdly_deep_are_a_limit_not_a_stack_overflow`, arrays nested in
  arrays `arrays_nested_absurdly_deep_are_a_limit_not_a_stack_overflow`).
* Compile-time cost is linear in the size of a procedure for the shapes that were quadratic:
  tens of thousands of labels (`tens_of_thousands_of_labels_lower_and_verify_in_linear_time`,
  `doubling_the_labels_about_doubles_the_time`) and chains of nested user-defined types
  (`a_chain_of_thousands_of_nested_types_is_analysed_in_linear_time`).

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
* `vbars inspect DIR` bounds the sum of all streams as well as each one
  (`a_directory_of_streams_has_a_total_size_limit`).
* Wine protocol: `hostile_counts_do_not_allocate_or_panic`, `pseudo_random_garbage_never_panics`.
* Source: `hostile_nesting_is_reported_not_overflowed`, `long_but_ordinary_chains_still_parse`
  (vba-syntax; nesting is limited to 200 and a chain of operators, member accesses or call
  suffixes to 512), `structured_programs_never_panic`, `token_soup_never_panics` (vba-sema).
  Long `ElseIf` and `Case` chains and `Case` lists are not recursive in the parser, sema,
  lowering or the editor features: 100000 clauses go through compile and run on a
  256 KiB thread (`a_very_long_elseif_chain_does_not_recurse_per_clause`,
  `a_very_long_case_chain_does_not_recurse_per_clause`,
  `a_very_long_case_list_does_not_recurse_per_item`; vba-lsp-core
  `editor_features_survive_a_very_long_elseif_chain` and `..._case_chain`).

## Not guaranteed

* **Memory is bounded, not exact.** The accounting is conservative and the limit can be overshot by
  what one instruction builds (see above). Allocations that the interpreter does not know about
  (the embedder's own host objects, `Dispatch` implementations, a custom `DebugSink` or `Vfs`) are
  not charged unless they use the meter. Use an operating-system limit (cgroup, `ulimit`, a
  separate process) for hard containment of hostile code.
* **Steps approximate CPU time.** One step is calibrated at roughly a microsecond of worst-case
  work, not guaranteed: the cost of an allocation, a hash lookup or a collation compare varies by
  machine and by string content (`Option Compare Text` folds through a table). A wall-clock
  deadline needs the cancellation token.
* **Check and use are two steps where there are no directory handles.** On Unix the
  mutating operations and file opens act through a verified directory handle (see
  above). What remains: `Dir`, `FileLen`, `FileDateTime`, `GetAttr` and `stat`/`list`
  inside the file system read metadata and names by path after the re-resolution, so
  a parent swapped in that instant can show the program the metadata or the names of
  a directory outside the grant (they change nothing). `Name` onto an existing name
  is refused with an atomic no-replace rename on Linux, Android and macOS, and with
  a check followed by `renameat` elsewhere, where a file created in the instant
  between them in the already verified directory is replaced. The host-side
  `GetObject` hook binds the resolved path again by name, so what the embedding
  application does with it is its own to protect. On other systems (Windows) no
  directory handle is used: every operation is the re-resolution followed by a call
  by name, and another process that swaps a directory for a link or junction in the
  instant between them can still redirect `Kill`, `Name`, `MkDir`, `RmDir`,
  `FileCopy` and `SetAttr`, and a write through a swapped path may create an empty
  file at the new place before the handle is compared. Keep granted directories out
  of the reach of other writers.
* **Windows paths follow the rules above, but the Windows file system was not run here.** The
  matching is tested with Windows spellings on every platform; a short name that exists on
  disk is refused rather than expanded, `\\?\` verbatim paths are never granted by a
  drive or UNC grant, and the start of a process (`CreateProcess` through
  `std::process::Command`) was not run.
* Native recursion outside the interpreter (sema, IR lowering and the formatter for shapes
  not listed above) has no byte budget; the shapes tried are bounded or iterative. Run
  analysis of untrusted source on a thread with a large stack (for example 64 MiB).
* The Wine helper executes native code of the library the embedder exports; it is not a sandbox.
* `Session::vm()` gives an embedder the interpreter without the trust gate.

# vba-vm

`crates/runtime/vba-vm` is a deterministic, synchronous interpreter for the
basic-block IR of `vba-ir`. It depends on `vba-value` (Variants, operators,
conversions), `vba-stdlib` (the function registry), `vba-automation` (objects
owned by the host), `vba-host` (permissions, limits, UI, `Declare` hooks) and
`vba-ffi` / `vba-winapi` (`Declare` marshaling and providers). It has no async
runtime, no threads, no `unsafe`, and no ambient authority: everything that
reaches outside is an injected seam.

```rust
let mut vm = Vm::new(ir, host, services, std_host, debug_sink, VmConfig::default())?;
let main = vm.find_procedure("Main").unwrap();
let out = vm.run_procedure(main, vec![])?;     // RunOutput { value, status }
```

`Vm::new` allocates module variables and fixed-size arrays (the standard-module
initialisers); it never runs a user procedure. `vba-runtime` wraps the VM in a
session with a trust gate (see `docs/runtime/runtime.md`).

## Design

- **Frames, slots, registers.** Each activation has slots (parameters, result,
  locals; a slot is a value or a binding to a caller location) and one register
  per IR register. Registers hold a value, a reference, a `For Each` iterator, a
  `Tab`/`Spc` helper, or a lazy *alias* of an array variable (so indexing a
  large array does not copy it; the array is read when the alias is used).
- **References.** A reference (`Loc`) names a local slot of a frame, a module
  variable, a field of a class instance, an array element, or a field of a
  user-defined-type value. `ByRef` arguments bind the callee slot to the
  caller's `Loc`, so aliasing works for locals, fields, elements and UDT
  fields, and chains of `ByRef` pass-through collapse. A by-value argument to a
  `ByRef` parameter simply becomes the callee's own value.
- **Calls.** Procedure calls (`call` and late-bound calls of class members)
  push a frame on an explicit stack and do not recurse natively; `Return` pops
  it. Arguments are bound positionally with `Optional` defaults, a missing
  marker for omitted `Variant` parameters (`IsMissing`), `ParamArray` arrays and
  named arguments (late bound).
- **Errors.** A trappable VBA error (`Fault::Err`) is routed per frame:
  `On Error GoTo` jumps to the handler (the handler is active until `Resume`,
  `On Error`, or procedure exit; an error inside an active handler propagates to
  the caller), `Resume Next` continues at the statement's resume point,
  `Resume` re-executes the *statement* (from its first instruction, so operands
  are evaluated again), `Resume label` jumps. Frames are unwound with their
  objects released. The `Err` object (Number, Description, Source, HelpFile,
  HelpContext, LastDllError, Clear, Raise) and `Erl` are VM state. Everything
  else that interrupts execution is a distinct, **untrappable** fault: resource
  limits, cancellation, unsupported features, the native bridge, internal
  errors; `End` resets module state, `Stop` and a failed `Debug.Assert` halt.
- **Values.** Variant operations go through `vba_value::ops` with the overflow
  mode the IR chose; objects used as values are replaced by their default
  member; typed results are coerced to the static type. Fixed-length strings
  pad and truncate on assignment and start as NULs. Arrays are values (copied on
  assignment, `ByRef` shares).
- **User-defined types** live in a VM heap behind `Variant::UserDefined`
  handles; every store copies (value semantics). Unreachable values are
  reclaimed by a mark-and-sweep pass at statement boundaries of the outermost
  run (`Vm::live_udts`).
- **Objects.** Class instances, `Collection`, `Err` and `Debug` live in a VM
  heap with reference counts; host objects stay in the host's `ObjectTable`
  (handles are partitioned by a high bit). Statement temporaries (results of
  `New`, calls, host members) are released at the next statement boundary.
  When a count reaches zero `Class_Terminate` runs while the object is still
  addressable (it may resurrect itself), then the object is removed and its
  fields are released in declaration order, depth first, in a loop (long
  ownership chains do not nest the native stack). Reference cycles are not
  collected.
- **Interfaces and events.** Calls through an `Implements` interface dispatch on
  the receiver's class using the mapping in `ClassInfo::implements`. An access
  written against the type of a *library* interface the class implements (a
  late-bound `Invoke` whose `static_receiver` names it) runs `Iface_Member`
  of the class, even when the class has a public member of the same name.
  `WithEvents` assignment (from inside or outside the class) connects the
  owner as a sink; `RaiseEvent` delivers to sinks in connection order with
  `ByRef` arguments aliased, stops at the first handler error, and a destroyed
  sink is disconnected. Events of host objects are advised through
  `vba-automation`'s `EventSource`; the VM's sink queues them and delivers them
  right after the host call that raised them returns (and at `DoEvents`).
- **Late binding.** Members of class instances resolve by name
  (case-insensitive, public only, default member through
  `Attribute x.VB_UserMemId = 0`); host objects go through
  `vba_automation::call` (default-member chaining, `ByRef` write-back, Let/Set
  rules). A non-object receiver is error 424, `Nothing` 91, a missing member 438.
- **Intrinsics.** The `vba-stdlib` registry with a `Ctx` built per call from the
  module's `Option Compare` / `Option Base`, the profile's pointer width, the
  injected `vba_stdlib::Host` (clock, environment, locale) and the VM's `Rnd`
  state. VM-implemented: `IsMissing`, `Erl`, `TypeName` of objects and UDTs,
  `LBound`/`UBound` (without copying), `Tab`/`Spc`, the `Mid` statement,
  `CreateObject`, `GetObject`, `MsgBox`, `InputBox`, `DoEvents`, `Beep` (no
  sound; there is nothing to play it), the `Error n` statement, `Debug.Print`
  (zones, `;`, `Tab`, `Spc`) and `Debug.Assert`. File functions, `Shell` and
  `Command` go to the injected `Services` (see `docs/runtime/runtime.md`).
- **`Declare`.** The statement is validated (`vba-ffi`), the host must grant
  `NativeDeclare` for `library!procedure`, a `vba-winapi` provider is looked up
  and arguments are marshaled (scalars, `String` buffers written back, `As Any`,
  UDTs by layout, arrays of scalars). A symbol without a provider falls back to
  the host's `declare_hook`; if that also reports error 48 the run ends with a
  bridge error. `Err.LastDllError` is read from the provider context after each
  call.
- **Limits.** One step is charged per instruction and terminator; call depth is
  charged per frame; memory is charged for arrays, UDTs, objects and collection
  items and released again (approximately). Strings longer than
  `VmConfig::max_string_units` and arrays larger than `max_array_elements`
  are refused. Nesting of the interpreter on the native stack (`Class_Initialize`
  inside `Class_Initialize`, event handlers raising events, default-member
  coercions) is limited by `max_native_depth` (48) and by the native stack the
  nested runs use, `max_native_stack_bytes` (1 MiB); beyond either the run ends
  with a call-depth limit error.

## Embedder calls

`run_procedure` / `run_method` take `Arg::Value`, `Arg::Missing` and `Arg::ByRef`.
A `ByRef` argument lives in a temporary variable slot above the module variables
for the duration of the call; the parameter is bound to it (after conversion to
its declared type) so the callee writes through, and the final values are returned
in `RunOutput::byref`. `arrange_call_args` places named arguments. `create_instance`,
`destroy_instance`, `default_instance_with`, `get_member`/`set_member`,
`get_global`/`set_global`, `current_error` and `reset` are the operations the
runtime layer exposes. A class-like object may carry a *host base* (a handle in the
host's object table owned by the instance): a late-bound member the module does not
define, including the default member, is dispatched to it. Errors carry the call
stack with module, procedure, `Erl` and source line (`StackFrame`), captured when
the error is raised; the stack of an error the program handles is kept in `Err`'s
state (`Vm::current_error`).

## Re-entrancy

A host object whose `Dispatch::reentrant()` is true makes the VM *suspend itself*
for the duration of the call: the whole `Vm` is moved into a `ReentrySlot` that
the host reaches from its `ObjectTable` (`ObjectTable::link`), and a hollow
placeholder takes its place. `ReentrySlot::with_vm` takes the VM out, runs a nested
operation (`run_top` and the `embed` operations), and puts it back; when the host
call returns the VM moves back. No unsafe code, no copying of state, no extra cost
for objects that do not declare themselves re-entrant.

* Nested runs share the budget (steps, memory, call depth), `Err`, module state and
  the `Debug.Print` sink. `max_reentrancy_depth` (default 4) bounds them; each also
  uses a `max_native_depth` level and counts toward `max_native_stack_bytes`.
* A nested run does not clear the frames of the outer run. An unhandled VBA error is
  returned (`Err`); `End`, `Stop` and the fatal faults (limits, cancellation,
  unsupported feature, internal error) are returned **and** stored in
  `Vm::pending_fatal`; when the host call returns, `invoke_external` re-raises it in
  the outer run, whatever the host did with the error, so `On Error` cannot swallow
  it.
* Events raised through a connection point by a re-entrant object while the VM is
  suspended in its call are delivered immediately (`deliver_event_now`) with
  `ByRef` write-back into the argument list; events from other objects, or raised
  while the VM is not suspended, are queued and delivered after the host call, in
  order (`deliver_pending_events` for the latter case). A queued event is always
  delivered before a newer one.
* Cancellation: `Host::poll_cancel` is polled at the start of every run and every
  1024 steps (basic blocks).

## Implemented

Every statement kind the IR can express: `If`, `Select`, `For`, `For Each`
(arrays, `Collection`, host enumerators), `Do`, `While`, `With`, `GoTo`,
`GoSub`/`Return`, `On ... GoTo/GoSub`, `On Error` in all forms, `Resume` in all
forms, `Exit`, `End`, `Stop`, `ReDim [Preserve]`, `Erase`, `LSet`/`RSet` of
strings, `Mid`, `Set`/`Let`, property calls, `RaiseEvent`, `Debug.Print`, `Open`,
`Close`, `Print #`, `Write #`, `Input #`, `Line Input #`, `Get`, `Put`, `Seek`,
`Lock`, `Unlock`, `Width`, `Name`.

## Callbacks (`AddressOf`)

`AddressOf Proc` (a `Sub` or `Function` of a standard module, checked by `vba-sema`) evaluates to
a `LongPtr` **token**, never to a native address: a 31-bit positive integer that names a slot
of the session's callback table (`vba_ffi::CallbackTable`, see `docs/interop/ffi.md`). The same
procedure always gives the same token within a session; a token is only meaningful to this VM.

* **Who can use it.** Only a shim of `vba-winapi` that takes a procedure pointer
  (`EnumWindows`, `EnumChildWindows`, `SetTimer`, `qsort`). While a `Declare` call runs, the shim
  is given a `CallbackHost` (`callbacks.rs`) that validates the token and runs the procedure; a
  host `declare_hook` or any other function receives the number and nothing else.
* **Validation, before anything runs.** The token must be live (a forged, guessed, shifted or
  multiplied number, or one from before `Vm::reset` or `End`, is error 5, "invalid procedure
  call"), and the procedure must fit the callback signature the shim names: same arity, every
  parameter `ByVal` and of the slot's type (`Long`, or `LongPtr` which is `Long` on a 32-bit and
  `LongLong` on a 64-bit target), and a `Function ... As Long` where the callback returns a
  value. A mismatch is error 49 ("bad DLL calling convention", what real VBA reports when the
  stack ends up unbalanced); nothing of the procedure runs.
* **Running it.** A nested run (`call_nested`) on the same VM, so the callback shares the step,
  memory, call-depth and native-stack budgets of the outer program (`max_native_depth`,
  `max_native_stack_bytes`); recursion through callbacks ends in the same call-depth limit as any
  other recursion. The `Declare` context (emulated memory, handles, last error) is handed to the
  procedure, so it may itself call `Declare`d functions such as `RtlMoveMemory` to read the
  elements `qsort` passes it.
* **Failure.** An error the callback does not handle, a limit, cancellation, `End` or `Stop`
  stops the shim (it only learns `Aborted`) and the `Declare` statement that made the call fails
  with exactly that fault: a VBA error stays trappable by the caller's `On Error`, limits and
  cancellation stay untrappable.
* **Capabilities.** A callback needs the grant of the `Declare` that received the pointer
  (`NativeDeclare`, `library!procedure`, deny by default): without it the `Declare` is error 70
  and the procedure never runs. Timers outlive the call that set them, so the grant is checked
  again when they are delivered.
* **Timers.** `SetTimer` keeps its timers in the `Declare` context. There is no message queue;
  `DoEvents` is the pump: it calls every due `TIMERPROC` once with the manual or injected clock
  (`Sleep` advances it). `Vm::reset` and `End` drop all timers.
* **Windows.** `EnumWindows` and `EnumChildWindows` enumerate the window list of the
  `CallContext` given to `Vm::set_native_bridge`; it is empty unless the embedding application
  adds windows (opaque `HWND` numbers it chooses).

Tests: `crates/runtime/vba-vm/tests/callbacks.rs` (error in a callback, recursion, step limit,
stale tokens after `reset` and `End`, forged tokens, signature mismatches, grants, timers) and the
`host/callback_*` conformance programs.

## Run-time provided functions

`CallByName(object, name, calltype, args...)` dispatches like a late-bound access:
`vbMethod` (1) and `vbGet` (2) are a get/call, `vbLet` (4) a property put (or a public
field store; the value is the last argument), `vbSet` (8) a `PutRef`. It works on class
instances (procedures, properties, public fields; a private member is error 438, an
assumed number), the built-in `Collection`/`Err`, and host objects through the automation
traits. Any other call type is error 5 (assumed). `Reset`, `Input()` and `FileAttr()` are
served by the file system layer (`docs/runtime/runtime.md`); the settings statements,
`Date =`/`Time =`, `SendKeys` and `AppActivate` go through host hooks (`docs/architecture/host.md`).
`AddressOf` yields a session-bound token that only emulated `Declare` shims can call back (see
"Callbacks (`AddressOf`)" below); it never produces a native address.

## Rules not verified against a real VBA host

These follow documentation or a reading of the specification; each is covered
by a test of our own expectation and none was compared with Office.

- `Err` is reset by `Resume Next`, `On Error GoTo -1`, `Err.Clear`, and when a
  procedure is left while its handler is active; `On Error` statements do not
  reset it.
- `On Error` inside an active handler re-arms the frame (that is how cleanup
  with `On Error Resume Next` inside a handler works); an error in an active
  handler without re-arming propagates to the caller.
- `Resume` re-executes from the first instruction of the statement.
- `Resume Next` also skips errors raised by `Return` without `GoSub` (3) and by
  `Resume` without an error (20) and continues with the next statement.
- `For Each` over an instance of a class runs the member marked
  `VB_UserMemId = -4` and enumerates what it returns: a `Collection`'s
  `[_NewEnum]` (the VM models the enumerator as the collection itself, usable
  only with `For Each`) or a host object's. A class without that member is
  error 438.
- `Set x = v` with a `Variant` source checks at run time: a non-object is 13
  into a `Variant` target and 424 into a class or `Object`.
- A `Variant` holding a scalar and followed by an argument list is error 9
  (MS-VBAL 5.6.2.1). A `Variant` holding an object keeps calling the object's
  default member (a documented deviation from the MS-VBAL text, recorded by an
  `XFAIL` conformance program).
- Late-bound calls of class members: a duplicate or unknown named argument and
  an omitted positional value for a required parameter are error 448; a missing
  trailing required argument is 449. The embedder-facing call API reports a
  parameter given twice as 446.
- `v(i)(j) = x` stores in place when `v` holds arrays (any depth); if `v` holds
  an object, `v(i)` is evaluated through the default member first.
- Under `Resume Next` an error in an `If`/`Select`/loop condition continues in
  the first body block (inherited from the IR).
- Unhandled errors, `Stop`, and resource limits are reported to the embedder;
  `Debug.Assert False` halts like `Stop`.
- Print formatting: numbers have a leading space (sign position) and a trailing
  space, zones are 14 columns wide, a `Tab` target already passed starts a new
  line.
- Collection errors: bad index 9, bad key 5, duplicate key 457.
- `Class_Initialize` failing destroys the half-built object without running
  `Class_Terminate`.
- Event sinks do not keep their source alive and vice versa (no reference
  cycles through connection points).
- Error numbers 18 (steps), 7 (memory) and 28 (call depth) of `vba-host` are
  *not* used by the VM: limits are untrappable faults, so a program cannot
  swallow them with `On Error`.
- **Unverified:** `Class_Initialize` / `Class_Terminate` also run for document and
  designer modules, because an embedding application's forms are classes; the
  behavior of document modules in other hosts was not compared.
- A fatal fault discards the frames of the run without running code (objects only
  those frames held are never terminated), unlike an unhandled VBA error, which
  unwinds with releases. `Vm::reset` is the way back to a clean state.

## Not implemented / known gaps

- `AddressOf` is a token for the emulated shims only: no native function can call a VBA procedure (no thunk is generated), and `AddressOf` of a `Declare` is rejected by the analysis.
- `LSet`/`RSet` between user-defined types whose members differ in type (the
  byte image would be needed), `Get`/`Put` of dynamic arrays
  are explicit `Unsupported` errors.
- A VBA object passed to a host is an opaque handle in the host's table; the host
  calls its members through `HostContext` (`call_on`, `get_member`...), not through
  `Dispatch` on the handle.
- The `Declare` bridge has no native backend: only the `vba-winapi` shims and
  host hooks exist; `Variant` arguments, `Object` parameters and variable-length
  strings inside UDTs are refused.
- Reference cycles between objects leak until the VM is dropped.
- `Static` arrays and the `Option Private Module` visibility rules are as
  analysed by `vba-sema`, not enforced again at run time.
- Allocations below the string and array limits are legal and cost real
  memory; set `ResourceLimits::max_memory_bytes` to bound them.

## Error taxonomy

| Fault | Trappable | Meaning |
| --- | --- | --- |
| `Err` (origin Vba / Host / Denied / Bridge) | yes | a VBA run-time error; the origin only classifies an unhandled one |
| `End` | no | ends the program, resets module state |
| `Stop` | no | halts (also failed `Debug.Assert`) |
| `Limit` (steps, memory, call depth, string length, array size) | no | a configured limit |
| `Cancelled` | no | `Host::idle` (`DoEvents`) or `Host::poll_cancel` failed |
| `Unsupported` | no | a feature this implementation lacks |
| `Bridge` | no | the `Declare` bridge cannot make the call |
| `Internal` | no | a broken invariant (a bug) |

## Performance

`tooling/vba-bench` is a plain timing binary (no benchmark dependency: it
needs only a wall clock, the minimum and median of N runs, and the step count,
and a statistics framework would add a dependency tree for no extra signal).
It compiles each program under `tooling/vba-bench/programs/`, runs it
repeatedly in fresh VMs and prints the step count, the minimum and median
wall time, the time per step and a checksum of the output. The step count and
checksum are printed so that an optimisation that changes step accounting or
behaviour is visible next to the timings; a run that is not deterministic
aborts.

```text
cargo run --release -p vba-bench -- [--runs N] [name-filter]
```

Programs: `int_loop` (nested `For` with `Long` arithmetic), `string_build`
(concatenation, `Left`/`Right`/`Mid`), `array_sort` (bubble sort of a fixed
array, fills and sums of a dynamic `Double` array), `collection_calls`
(`Add` with keys, `Item` by key and index, `For Each`, `Remove`),
`class_calls` (method, property and public-field access on a class instance,
early and late bound), `recursion` (`Fib`, `ByRef` calls), `variant_math`
(`Variant` arithmetic over mixed subtypes) and `format_val` (`Format` and
`Val` in a loop).

Reference numbers, minimum of 15 runs, release profile (the workspace default,
`opt-level = 3`, no LTO), one thread, on an Apple M1 Pro. Absolute times are
only comparable on the same machine; the step counts do not depend on it.
"Before" is the interpreter before the optimisation work described below.

| program | steps | before (ms) | after (ms) | speed-up |
| --- | ---: | ---: | ---: | ---: |
| `int_loop` | 4,203,909 | 133.2 | 59.5 | 2.2x |
| `string_build` | 2,344,276 | 113.9 | 91.5 | 1.2x |
| `array_sort` | 2,580,940 | 118.0 | 54.0 | 2.2x |
| `collection_calls` | 216,513 | 15.2 | 12.3 | 1.2x |
| `class_calls` | 464,013 | 24.4 | 14.2 | 1.7x |
| `recursion` | 873,137 | 48.2 | 24.1 | 2.0x |
| `variant_math` | 1,340,013 | 44.8 | 20.8 | 2.2x |
| `format_val` | 182,072 | 42.0 | 36.7 | 1.1x |

The step counts and output checksums are identical before and after. What
changed, all inside the interpreter loop and without `unsafe`:

* Operands that are plain values are used in place (no `Variant` copy and no
  default-member resolution) for arithmetic, comparison, logic, `If`
  conditions and `For` tests; `Long`/`Integer`/`Double` operations have a
  shortcut in `vba_value::ops` that returns `None` to the general path
  whenever overflow, promotion or an error could be involved.
* Storing and releasing plain values no longer allocates (the release list was
  built for every store); storing to an unbound local slot skips the location
  resolution.
* A call reuses the slot and register vectors of the frames it pops, and the
  run-time error is boxed inside `Fault` so every `VResult<Variant>` is about
  half the size.
* Array subscripts (up to three) live inline in a `Loc::Elem`; intrinsic names
  are lower-cased on the stack and resolved with a cheap hash; class members
  and public fields are found through per-module tables.

What was deliberately left alone: string copies. A string of 32 code units or
more charges its allocation to the memory meter, and the meter's volume is
part of the step count (see `vba_host::Budget`), so avoiding a copy of a long
string would change the steps a program is charged. Changing that is a
semantic decision, not an optimisation. Collection lookup by key is a linear
scan, and `Format` is dominated by the digit generation in `vba-stdlib`.

## Tests

`tests/*.rs`: table-driven end-to-end programs (source in, `Debug.Print` text
out) for control flow, procedures and arguments, `On Error` in every form,
arrays, UDTs, strings and Variants, classes, lifetime and `Class_Terminate`
order, `Implements`, events, collections; host integration (permissions,
objects, UI, `Declare`, limits, cancellation, `Stop`/`End`); depth and native
stack safety.

Robustness tests:

* `fuzz.rs`: random single-module programs under tight limits: no panic, no
  internal error, steps within the budget, identical output and result on a
  second run.
* `fuzz_programs.rs`: the same properties for random multi-module programs
  built from many more statement kinds: helper procedures (recursion, `ByRef`,
  `ParamArray`, optional arguments, handlers), classes with `Class_Initialize`
  and `Class_Terminate`, an interface and an implementer, multi-dimensional
  and dynamic arrays, user-defined types, `Variant` arithmetic over mixed
  subtypes and `Null`, collections, late binding and every `On Error` form.
  At least 70 percent of the generated programs must compile (about 80 do),
  and roughly a third run to completion, a tenth stop at a limit and the rest
  end in an unhandled error, so all three outcomes are exercised.
  `VBA_FUZZ_CASES=N` raises the number of cases.
* `hostile_ir.rs`: valid lowered programs are mutated (registers, slots,
  global/procedure/module/udt/event ids, types, operands, whole instructions,
  terminators and block targets, the class table) and `vba_ir::verify` must
  not panic on the mutant; when it accepts the mutant the VM must run it
  without panicking (any `VmError` is acceptable). The VM relies on verified
  IR and is not hardened against IR that skipped `verify`. The survey test
  (`-- --ignored survey`, `SURVEY_N` mutants) lists the distinct panic sites.
  It found one gap: the class table (fields, initialiser, terminator,
  implemented members, event handlers) was not range checked, so a malformed
  table passed `verify` and panicked the VM on an out-of-range index; `verify`
  now checks it, with regression tests written first.
* `determinism.rs`: every program of the conformance corpus (`tests/vba`) runs
  twice in fresh VMs; output, result and step count must be identical and
  nothing may panic or report an internal error. `VBA_STEP_DIGEST=<file>`
  writes one line per program (steps, output checksum, result checksum);
  diffing two digests shows whether a change altered step accounting or
  behaviour anywhere in the corpus. Programs that do not compile in this
  harness (diagnostic tests, programs needing a type library) are skipped.

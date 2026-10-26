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
session with a trust gate (see `docs/runtime.md`).

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
  the receiver's class using the mapping in `ClassInfo::implements`.
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
  `Command` go to the injected `Services` (see `docs/runtime.md`).
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
  coercions) is limited by `max_native_depth` (48); beyond it the run ends with
  a call-depth limit error.

## Implemented

Every statement kind the IR can express: `If`, `Select`, `For`, `For Each`
(arrays, `Collection`, host enumerators), `Do`, `While`, `With`, `GoTo`,
`GoSub`/`Return`, `On ... GoTo/GoSub`, `On Error` in all forms, `Resume` in all
forms, `Exit`, `End`, `Stop`, `ReDim [Preserve]`, `Erase`, `LSet`/`RSet` of
strings, `Mid`, `Set`/`Let`, property calls, `RaiseEvent`, `Debug.Print`, `Open`,
`Close`, `Print #`, `Write #`, `Input #`, `Line Input #`, `Get`, `Put`, `Seek`,
`Lock`, `Unlock`, `Width`, `Name`.

## Run-time provided functions

`CallByName(object, name, calltype, args...)` dispatches like a late-bound access:
`vbMethod` (1) and `vbGet` (2) are a get/call, `vbLet` (4) a property put (or a public
field store; the value is the last argument), `vbSet` (8) a `PutRef`. It works on class
instances (procedures, properties, public fields; a private member is error 438, an
assumed number), the built-in `Collection`/`Err`, and host objects through the automation
traits. Any other call type is error 5 (assumed). `Reset`, `Input()` and `FileAttr()` are
served by the file system layer (`docs/runtime.md`); the settings statements,
`Date =`/`Time =`, `SendKeys` and `AppActivate` go through host hooks (`docs/host.md`).
`AddressOf` stays an explicit `Unsupported` error: producing a callable address would need
a native thunk, and a made-up pointer would be a fake; the conformance program
`language/addressof_without_native` records that.

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

## Not implemented / known gaps

- `Resume Next` after an error raised by `Return` (without `GoSub`) or `Resume`
  (without error) cannot continue: the IR drops the unreachable code after those
  jumps, so such an error propagates as if unhandled.
- `AddressOf` is an explicit `Unsupported` error (callbacks from native code).
- `MidB` statement, `LSet`/`RSet` between user-defined types, `Get`/`Put` of
  dynamic arrays and `Decimal` values are explicit `Unsupported` errors.
- Class members cannot be called back by host objects (a VBA object passed to a
  host is not in the host's table; use events). Custom `For Each` enumerators on
  VBA classes (`_NewEnum`) are not supported (error 438).
- `Implements` of library interfaces is analysed but calls through the library
  interface type are late bound by name and do not reach `Iface_Member`.
- Late-bound assignment to an element of an array nested inside another array (`v(1)(2) = x`); `v(i) = x` on a `Variant` variable that holds an array works (the value is converted to the array's element type).
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
| `Cancelled` | no | the host idle hook failed (`DoEvents`) |
| `Unsupported` | no | a feature this implementation lacks |
| `Bridge` | no | the `Declare` bridge cannot make the call |
| `Internal` | no | a broken invariant (a bug) |

## Tests

`tests/*.rs`: table-driven end-to-end programs (source in, `Debug.Print` text
out) for control flow, procedures and arguments, `On Error` in every form,
arrays, UDTs, strings and Variants, classes, lifetime and `Class_Terminate`
order, `Implements`, events, collections; host integration (permissions,
objects, UI, `Declare`, limits, cancellation, `Stop`/`End`); depth and native
stack safety; and `fuzz.rs`, a property test that runs random generated
programs under tight limits and asserts: no panic, no internal error, steps
within the budget, identical output and result on a second run.

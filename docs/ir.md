# Intermediate representation (`vba-ir`)

`vba-ir` lowers the bound model of `vba-sema` into a basic-block IR that a VM
can execute directly with `vba-value` and `vba-stdlib`, plus a text dumper
and a verifier. It has no dependency on any runtime or automation crate: late
binding, objects and files are *operations* here, implemented by the VM.

```rust
let analysis = vba_sema::analyze_sources(...);
let ir = vba_ir::lower(&analysis)?;          // refuses programs with errors
vba_ir::verify(&ir).expect("invariants");    // checks the invariants below
println!("{}", vba_ir::dump_program(&ir, DumpOptions::default()));
```

## Design and rationale

- **CFG of basic blocks, not a structured tree.** VBA has `GoTo`, `GoSub`,
  `Resume`, `On ... GoTo` and jumps out of `With`/`For`/`Do`; a structured IR
  would need escape constructs for all of them. Blocks make every one of them
  a plain edge, make `Resume Next` expressible as a position, and let a VM use
  a simple `(block, index)` program counter. Structured statements (`If`,
  `Select`, `For`, `For Each`, `Do`, `While`, `With`) are lowered to blocks;
  no source construct needs a special terminator except `For Each`.
- **Slots for variables, registers for temporaries.** Locals, parameters and
  the function result are *slots* (addressable frame cells, `s0:name`).
  Expression temporaries are *virtual registers*, each defined exactly once
  (SSA-like definition without phis, because mutable state lives in slots).
  `ByRef` arguments are explicit: `MakeRef` creates a reference to a slot,
  static variable, field, array element or UDT field and calls pass
  `ref rN`. A by-value argument to a `ByRef` parameter is a temporary the
  callee frame wraps.
- **Typed versus Variant operations.** `Binary`/`Unary` carry the result type
  and an `Overflow` mode (`typed` when both operand types are static, so
  overflow is error 6; `variant` when a Variant is involved, promoting like
  `vba_value::ops`), and the operator families of `vba_value::ops`
  (`ArithOp`, `CompareOp` + `CompareOptions`, `LogicOp`). Comparison
  instructions carry the `Option Compare` mode and whether a string is
  compared with a number (`+coerce`). Implicit conversions are explicit
  `Convert` instructions (`vba_value::convert::convert_to`; to an object type
  it is a checked cast, to `Variant` it boxes). Constants are converted at
  analysis time where that is exact.
- **Late binding mirrors `IDispatch`.** `Invoke` has a member (by name or the
  default member), the flavour `get` (method or property get), `put` or
  `putref`, positional and named arguments (variables are passed `ref`), and
  the receiver's library type when it is statically known. Early-bound library
  members use the same instruction with the verified signature already applied
  (argument order, defaults, `ByRef`).
- **`On Error` is explicit state.** `on.error goto bN | resume.next |
  propagate`, `clear.error` (`GoTo -1`). When an instruction fails the VM
  records the failing position, activates the handler and continues at its
  block. `Resume` re-executes the failing instruction, `Resume Next` continues
  at the `resume_next` point of the failing *statement*, `Resume label` jumps.
  Every instruction and terminator belongs to a statement
  (`Function::stmts`), and each statement records the point where execution
  continues after it. For a compound statement (`If`, `Select`, loops) the
  point is the start of its first body block, which reproduces the documented
  VBA behaviour that an error in a condition under `Resume Next` runs the
  body (**unverified**).
- **`GoSub`/`Return`.** `gosub bT ret bR` pushes `bR` on a per-frame stack;
  `return.gosub` pops it (error 3 when empty). `On n GoSub` goes through one
  trampoline block per target.
- **Loops.** `For` evaluates start, end and step once, converts them to the
  counter type when the counter is typed, tests with `for.test`
  (`step >= 0 ? counter <= end : counter >= end`), and increments with a typed
  or variant `add`. `For Each` is `iter.init` plus the `foreach.next`
  terminator which stores the element in a register and branches. `Exit For`,
  `Exit Do` are jumps to the loop exit; `While ... Wend` is not left by
  `Exit Do`.
- **Objects and lifetime.** `StoreMode::Set` is the reference store: the VM
  retains the new object and releases the old one, and runs `Class_Terminate`
  (`ClassInfo::terminate`) when the count reaches zero; leaving a frame
  releases all object slots (the `return` terminator). `new` allocates, runs
  the class's field initialiser (`ClassInfo::init`) and `Class_Initialize`.
  `As New` variables are `ensure.new` before each use. `Set` of a
  `WithEvents` variable is `set.events` and the VM reconnects the handlers in
  `ClassInfo::handlers`. `raise.event` delivers to the handlers of every
  connected sink. Interface calls name the *interface's* procedure with a
  receiver; the VM dispatches on the receiver's class through
  `ClassInfo::implements`.
- **Declarations.** Fixed-size arrays are allocated by `dim.fixed` in the
  function entry (locals) or in the module initialiser
  (`IrProgram::module_init`; for a standard module it runs once at start, for a
  class on each new instance before `Class_Initialize`). UDT values and
  object-typed slots start from their type's default.
- **Debugging.** `Inst::span` and `Inst::stmt` give source positions;
  `StmtInfo::line` and `set.line N` (executed at numeric labels) support
  `Erl`.
- **Clean-up.** Lowering threads empty jump blocks, merges straight-line
  blocks that have a single predecessor (never an error handler, `GoSub` return
  point or `Resume label` target), drops unreachable blocks and lays blocks out
  in reverse post-order (handler-only flows after the main flow).

## Instruction set (overview)

Values: `copy`, `unary`, `binary`, `convert`, `load/store.local|global|field`,
`me`, `default.instance`, `lib.global`, `ref`, `load.ref`, `store.ref`, `index`,
`udt.field`. Calls: `call` (project procedure, `Declare` external call,
intrinsic), `invoke.get|put|putref`, `new`, `ensure.new`, `addressof`,
`typeof.is`. Arrays: `dim.fixed`, `redim[.preserve]`, `erase`. Control:
`for.test`, `iter.init`, `on.error`, `clear.error`, `set.line`, terminators
`jump`, `branch`, `switch`, `gosub`, `return.gosub`, `foreach.next`, `resume`,
`return`, `end`, `stop`, `unreachable`. Events and I/O: `raise.event`,
`debug.print`, `io.open|close|print|write|input|lineinput|get|put|seek|lock|
unlock|width|name`.

`branch` tests a `Boolean` directly; for a `Variant` condition the `If` rule
applies (Null is false, otherwise `CBool`). `ParamArray` extras are appended
after the fixed arguments; `missing` marks an omitted optional argument whose
default the callee frame supplies (`ParamInfo::default`).

## Verifier

`verify` checks: functions line up with procedures; blocks, statements,
slots, globals, procedures, events and UDT fields exist; resume points lie
inside their blocks; every register is defined exactly once and every use is
dominated by its definition on all normal paths (handler entries start with
no registers; `Resume` edges are not normal edges); call argument counts and
receivers match the callee, a result register exists only for callees that
return a value; branch conditions are `Boolean` or `Variant`; the `For Each`
iterator comes from `iter.init`; slot layout matches the procedure signature.

## Not implemented / known gaps

- No optimisation beyond CFG clean-up and constant conversion (no constant
  propagation, no dead-store elimination, no inlining).
- Object release on scope exit, reference counting, `Terminate` order and
  event delivery are VM semantics described above, not IR instructions.
- Overflow of a `For` counter at the end of its type's range (`For i% = 1 To
  32767`) is an ordinary typed `add` and so raises error 6; real VBA
  behaviour is **unverified**.
- `Select Case` evaluates case expressions lazily in order; whether VBA
  evaluates all of them first is **unverified**.
- `With` on a user-defined type reloads the variable at each member access.
- Passing a property or late-bound result `ByRef` passes a copy.

## Tests

`tests/snapshots.rs` has one `expect-test` snapshot per statement kind (44);
`tests/program.rs` covers the multi-module corpus (class information, module
initialisers, interface calls), statement/resume tables and spans, and the
verifier's negative cases; `tests/fuzz.rs` property-tests that random typed
programs always lower to IR the verifier accepts with a deterministic dump,
and that token soup never panics anywhere from parse to verification.
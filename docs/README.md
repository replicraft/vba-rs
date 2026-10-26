# Documentation index

Each document describes one component: what is implemented, what is not, and how sure we
are. Statements about behavior carry a confidence marker (`spec`, `doc`, `derived`,
`unverified`, or `[H]`/`[M]`/`[L]` in the compiled-code documents).

## Start here

1. **What is implemented.** [Conformance](tooling/conformance.md) is the measured summary
   (generated from `tests/inventory.csv` and the programs under `tests/vba/`); the component
   documents below list gaps per crate.
2. **How to run.** The `vbars` command line is described in the [root README](../README.md);
   run-time behavior and exit codes are in [runtime](runtime/runtime.md), and the grants a
   program needs in [host](architecture/host.md).
3. **How to embed.** [Architecture overview](architecture/overview.md), then the
   [embedding guide](architecture/embedding-guide.md) (the whole flow with compiled examples:
   forms with a host `Me`, `ByRef` events, re-entrancy, errors, limits), the
   [public API](architecture/public-api.md) (the `vba` facade) and [host](architecture/host.md)
   (the contract an embedder implements).

## architecture/

* [overview](architecture/overview.md): crate groups, dependency direction, data flow, the permission model, the standalone-library principle.
* [embedding-guide](architecture/embedding-guide.md): embedding end to end (load, trust, forms, host objects, events with `Cancel`, re-entrancy, errors, limits, threading, transactions, what is not supported).
* [public-api](architecture/public-api.md): the stable `vba` facade for embedders.
* [host](architecture/host.md): the `vba-host` contract, capabilities, policy and limits.
* [security-model](architecture/security-model.md): what is guaranteed (with the tests) and what is not.
* [dependencies](architecture/dependencies.md): every third-party dependency and why it is used.

## language/

* [foundation](language/foundation.md): `vba-core`, `vba-source`, `vba-diagnostics`, `vba-profile`.
* [lexer](language/lexer.md): tokens, code pages, lexical heuristics.
* [syntax](language/syntax.md): the lossless syntax tree, the parser, the AST.
* [sema](language/sema.md): name resolution, types, constant folding, diagnostics.
* [spec-coverage](language/spec-coverage.md): status against MS-VBAL 3.0, by chapter.
* [ir](language/ir.md): the intermediate representation and its verifier.
* [vba-value](language/vba-value.md): `Variant`, `Currency`, `Decimal`, strings, arrays.
* [vba-stdlib](language/vba-stdlib.md): the built-in function library and its verification status.
* [scripting](language/scripting.md): `Scripting.Dictionary` and `Scripting.FileSystemObject`.

## runtime/

* [vm](runtime/vm.md): the interpreter over the IR.
* [runtime](runtime/runtime.md): sessions, trust, the file system layer, error taxonomy.
* [automation](runtime/automation.md): late-bound object traits, HRESULT mapping, lifetimes.
* [compiled-projects](runtime/compiled-projects.md): projects whose source is missing or stale, and what is refused.

## formats/

* [vba-format](formats/vba-format.md): the MS-OVBA reader and writer, preservation, JSON inspect schema.
* [vba-project](formats/vba-project.md): the project model.
* [typelib](formats/typelib.md): the type-library reader.
* [typelib-binding](formats/typelib-binding.md): how library types flow through sema, IR and the VM (early binding).
* [pcode](formats/pcode.md): compiled-code cache, observations and verification plan.
* [pcode-format](formats/pcode-format.md): compiled-code cache structure specification.
* [pcode-opcodes](formats/pcode-opcodes.md): compiled-code opcode reference.
* [pcode-status](formats/pcode-status.md): decoder status and metrics.

## interop/

* [ffi](interop/ffi.md): the `Declare` ABI layer and native calls.
* [winapi](interop/winapi.md): the Win32 catalog and provider framework.
* [wine-bridge](interop/wine-bridge.md): running Windows libraries through Wine.

## tooling/

* [lsp](tooling/lsp.md): the language server core and the `vba-lsp` binary.
* [conformance](tooling/conformance.md): the generated conformance report (do not edit by hand).
* [differential-testing](tooling/differential-testing.md): running the conformance programs in real VBA (the oracle), comparison, provenance upgrade. The Windows side is untested.

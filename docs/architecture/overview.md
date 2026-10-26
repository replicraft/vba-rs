# Architecture overview

vba-rs is a standalone Rust implementation of VBA: a library workspace with a small
command-line tool and a language server on top. It does not depend on, or know about, any
embedding application. Everything an embedding application provides (objects, files,
user interface, clocks) comes in through the host contract.

## Crate groups

| Group | Crates | Role |
| --- | --- | --- |
| `crates/foundation/` | `vba-core`, `vba-source`, `vba-diagnostics`, `vba-profile` | Error numbers, source files and spans, diagnostics, language profiles. No dependencies on other groups. See [foundation](../language/foundation.md). |
| `crates/language/` | `vba-lexer`, `vba-syntax`, `vba-ast`, `vba-sema`, `vba-ir`, `vba-lsp-core` | The front end: tokens, concrete syntax tree, typed AST, semantic analysis, the IR the VM runs, and editor analysis. |
| `crates/runtime/` | `vba-value`, `vba-stdlib`, `vba-vm`, `vba-runtime`, `vba-scripting` | Values, the built-in library, the interpreter, the session that wires a project to a host, and the `Scripting` library objects. |
| `crates/model/`, `crates/formats/` | `vba-project`, `vba-format`, `vba-pcode` | The project model, the `vbaProject.bin` / MS-OVBA reader and writer, and the compiled-code (p-code) decoder. |
| `crates/hosts/` | `vba-host` | The contract an embedder implements, with the permission policy and resource limits. |
| `crates/interoperability/` | `vba-automation`, `vba-ffi`, `vba-typelib` | Late-bound object traits, native-call layout rules, type-library reading. |
| `crates/windows/` | `vba-winapi`, `vba-wine` | Win32 `Declare` shims and the optional Wine bridge. Behind cargo features. |
| `crates/api/` | `vba-rs` | The facade (`vba`) embedders depend on. See [public API](public-api.md). |
| `apps/` | `vba-cli` (`vbars`), `vba-lsp`, `vba-wine-host` | Binaries; they use the libraries and nothing else uses them. |
| `tooling/` | `vba-winapi-gen` | Developer tools that generate checked-in data. Never a library dependency. |

## Dependency direction

* Dependencies point from the outside in: `apps` and `crates/api` depend on the libraries;
  libraries never depend on `apps` or on the facade.
* `foundation` depends on nothing else in the workspace (`vba-diagnostics` uses `vba-source`).
* The front end (`lexer -> syntax -> ast -> sema -> ir`) is a chain: a stage depends on the
  earlier stages, never on a later one. The syntax tree knows no semantics; the IR knows no
  runtime.
* `vba-value` and `vba-stdlib` do not depend on the VM. `vba-vm` executes IR against the host
  contract (`vba-host`) and the automation traits; it does not read files or parse projects.
* Format crates (`vba-format`, `vba-pcode`) are pure functions over bytes; the project model
  (`vba-project`) has no workspace dependencies and format crates depend on it, never the
  reverse.
* Platform-specific code (`vba-winapi`, `vba-wine`, native calls in `vba-ffi`) is isolated
  behind cargo features, all off by default. `unsafe` is forbidden workspace-wide except where
  a crate documents and gates it.
* Third-party dependencies are listed with their roles in [dependencies](dependencies.md).

## Data flow

```text
bytes ──> source ──> tokens ──> CST ──> AST ──> sema ──> IR ──> VM ──> host
(file)   (vba-source) (lexer)  (syntax) (ast)  (sema)   (ir)   (vm)   (vba-host)
```

1. A project arrives as source files, exported streams or a `vbaProject.bin`
   (`vba-format` / `vba-project`). Source text stays as the original bytes plus a declared
   code page.
2. The lexer produces tokens with spans; the parser builds a lossless concrete syntax tree
   with error recovery; lowering produces the AST.
3. Semantic analysis resolves names and types and reports diagnostics. `vba-lsp-core` reuses
   these stages for editor features.
4. Lowering produces the IR, which `verify` checks. The VM interprets the IR, calling the
   standard library for built-ins and the host for everything outside the language.
5. `vba-runtime` assembles a session: it owns the project, the file system view, the
   scripting objects and the host wrapper, and exposes load, trust, run and shutdown.

The compiled-code cache of a project is preserved as opaque bytes by the project model and
can be decoded by `vba-pcode`; it is never executed (see [compiled projects](../runtime/compiled-projects.md)).

## The host and permission model

* Deny by default. A host grants capabilities explicitly (`file-read`, `file-write`,
  `process-spawn`, `network`, `com-create`, `native-declare`, `ui-automation`), optionally
  scoped to a path prefix. The provided `Host` methods check the policy before calling the
  embedder's hook, so a hook cannot be reached without a grant. See [host](host.md).
* Resource limits (steps, memory, call depth) and a cancellation token bound every run.
* Loading never runs user code. A loaded project is untrusted until the embedder calls
  `trust` (or calls the one-shot `run`, which is that decision). Importing a project never
  triggers automatic macros or events.
* Host objects are `Dispatch` implementations registered by name; the default host registers
  no classes.

## The standalone-library principle

vba-rs is useful without any particular application. The workspace has no references to
embedding applications and no application-specific types; an application integrates by
implementing `Host` and `Dispatch` and by passing projects in. Anything that would make the
library depend on an application belongs in that application. Behavior is measured against
the language specification and the Microsoft language reference
(see [conformance](../tooling/conformance.md)), not against any one host.

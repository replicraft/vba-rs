# Real-world source corpus (local test)

The unit and conformance tests use small, hand-written programs. Real projects contain constructs
nobody thought to write a test for, so the front end (lexer, parser, semantic analysis, formatter,
lowering, language-server core) can also be run over a directory of module files you collected
yourself. The files are never part of the repository and the test is skipped when it is not
pointed at a directory.

## Making the input

The test reads `.bas`, `.cls` and `.frm` files. To get them from Office documents and
`vbaProject.bin` files, use the exporter, which goes through the same container and project readers
as the rest of the engine (OOXML packages, binary Office documents, raw project storages):

```text
cargo run -p vba-rs --example export_modules -- <documents> <output directory>
```

`<documents>` is a file or a directory searched recursively. Each document becomes a numbered
subdirectory (one project) holding a file per module that has source, plus `project.txt` (project
name and conditional-compilation line). `manifest.tsv` in the output directory lists
`path<TAB>kind` (`standard`, `class`, `document`, `designer`) so document and form modules are
analysed as what they are. Documents the reader refuses are listed and make the exit code 1; that
is a reader bug worth investigating. The documents are only read, never executed.

Any directory of module files works, with or without the extra files: each subdirectory is one
project, files directly in the root form one project, the kind defaults to the file extension.

## Running

```text
VBA_SOURCE_CORPUS=<directory> cargo test -p vba-rs --test source_corpus -- --nocapture
```

Optional variables: `VBA_SOURCE_CORPUS_VERBOSE=1` prints every semantic diagnostic
(`DIAG code severity project/module:line message`), `VBA_SOURCE_CORPUS_MAX_MS` sets the per-module
time limit of the (unoptimized) test build, default 10000.

## What it checks

Per module, in all cases a failure of the test:

* the syntax tree reproduces the source byte for byte, under every language profile and in strict
  and tolerant mode;
* formatting is idempotent, with and without keyword-case normalization, and never changes the token
  stream (apart from the case of reserved words when that option is on);
* no panic anywhere, and no module takes longer than the limit;
* hover, completion, signature help, definition, symbols, folding, semantic tokens and diagnostics
  of the language-server core do not panic at positions spread over the module.

Per project: the project is analysed with a permissive stand-in for the embedding application's
object model (every type name resolves to a late-bound object, the common application globals and
`xl...`/`wd...`/`mso...`/`fm...` constants exist) and lowered to IR, and the IR must verify. A
project with semantic errors cannot be lowered, so the modules that have errors are left out,
repeatedly, and what remains is lowered and verified.

Semantic diagnostics are counted by code and are not failures: real code has real errors (stale
calls, misspelt names, code that only compiles in the application it was written for). Triage them
with the verbose output: a diagnostic is a bug in the analysis when real VBA accepts the code.
Typical true positives are references to other projects or libraries that are not available, form
controls (their declarations live in binary form data, not in the source), and application-only
names the stand-in does not describe.

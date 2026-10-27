# Contributing

Thank you for helping. Read the [architecture overview](docs/architecture/overview.md) first:
it explains the crate groups and which way dependencies may point.

## Build, test, format, lint

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

The toolchain is pinned by `rust-toolchain.toml` (1.99.0, the workspace `rust-version`);
`rustup` installs it on first use. `Cargo.lock` is committed and CI builds with `--locked`: commit
the lock file with any dependency change.

CI (`.github/workflows/ci.yml`) runs `cargo fmt --all -- --check`, clippy with `-D warnings`,
`cargo doc --workspace --no-deps` with `RUSTDOCFLAGS="-D warnings"`, and `cargo test --workspace`
on Linux and macOS; Windows is build-only. `.github/workflows/audit.yml` runs `cargo deny`
(`deny.toml`: licenses, bans, sources, advisories) and `cargo audit` weekly and when the lock file
changes. Run fmt, clippy, doc and test before you push. Warnings are errors.

Rules the workspace enforces or expects:

* `unsafe_code` is forbidden workspace-wide. A crate that needs it must document and gate it.
* Public items have documentation (`missing_docs` warns).
* Dependencies are declared once in the root `Cargo.toml`, under a heading that names their
  function, and crates use `workspace = true`. Add a dependency only when the standard
  library cannot do the job, and record why in
  [docs/architecture/dependencies.md](docs/architecture/dependencies.md).
* Do not add a data-parallelism crate.
* The libraries do not know about any embedding application. Anything an embedding
  application provides comes in through the `vba-host` contract.
* Do not weaken a security guarantee without updating
  [docs/architecture/security-model.md](docs/architecture/security-model.md) and its tests.

## Commit style

Small, focused commits with conventional prefixes: `feat(scope):`, `fix(scope):`,
`docs:`, `test:`, `refactor(scope):`, `build:`, `chore:`. The scope is the crate or area
without the `vba-` prefix (`lexer`, `vm`, `cli`, ...). Write the subject in the imperative,
and use the body to say why. A behavior change and its test belong in the same commit.

## Conformance tests

The conformance suite is the measure of compatibility. Each program under `tests/vba/<area>/`
is a `.bas` file whose leading comment lines state what it tests and what it must print:

```vb
' INV: func.array
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/array-function
' EXPECT:  0  2 
Public Sub Main()
    Dim v As Variant
    v = Array("a", 2, 3.5)
    Debug.Print LBound(v); UBound(v)
End Sub
```

* `INV` lists the ids of the inventory entries in `tests/inventory.csv` that the program
  exercises. Add a row there for a new construct.
* `SOURCE` says where the expected value comes from: `spec <section>` (MS-VBAL), `doc <url>`,
  `derived <why>` or `unverified <note>`. Prefer the strongest source you can cite, and
  never copy the implementation's output into `EXPECT` without marking it `unverified`.
* `EXPECT` lines are the exact `Debug.Print` output. Other directives (`ERROR`, `GRANT`,
  `MAX_STEPS`, `CLOCK`, `STRICT_SPEC`, `COMMAND`, `INPUTBOX`, `ENTRY`, `XFAIL`) and the `'#MODULE` syntax for
  multi-module programs are listed at the top of
  `tooling/vba-conformance/src/lib.rs`. A program that needs a host feature a restricted
  embedding may not provide declares it with `REQUIRES:` (it is skipped, not failed, when the
  run does not provide it).
* A known deviation is recorded with `XFAIL: wrong|unsupported <why>`; the program must keep
  failing until the behavior is fixed, then the `XFAIL` line is removed.

After adding or changing programs, regenerate the derived inventory columns and the summary:

```sh
VBA_BLESS=1 cargo test -p vba-rs --test conformance
cargo test -p vba-rs --test conformance      # must pass without VBA_BLESS
```

Commit `tests/inventory.csv` and `docs/tooling/conformance.md` together with the programs.
See [docs/tooling/conformance.md](docs/tooling/conformance.md) for the meaning of the
statuses and [docs/tooling/differential-testing.md](docs/tooling/differential-testing.md)
for checking programs against real VBA.

## Command-line tests

Tests of `vbars` live in `apps/vba-cli/tests/` and use `assert_cmd`. Cover the success path
and the failure path (with the exit code) of whatever you add.

## Reporting security problems

Do not open a public issue; see [SECURITY.md](SECURITY.md).

## License

By contributing you agree that your contribution is licensed under Apache-2.0, as the
rest of the project.

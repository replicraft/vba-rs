# Differential testing against real VBA (the oracle)

**The Windows side of this workflow (`tests/oracle/OracleHarness.bas`, `OracleCalibrate.bas`,
`run-oracle.ps1`) is untested.** It was written and reviewed by reading, on a machine without
Windows or Office, and has never been compiled or run. The Rust side (bundle preparation, comparison,
provenance upgrade, reports) is tested with synthetic data. Nothing in the repository is real oracle
data yet: `tests/oracle/ENVIRONMENTS.md` has no rows.

## Purpose

The conformance corpus (`tests/vba/`, see [conformance](conformance.md)) records expected output with
a provenance tag. Most tags are `doc`, `derived` or `unverified`: none of them is a measurement of
what real VBA does. A Windows lab with Microsoft Excel or Access can act as the **oracle**: run the
same programs in real VBA, capture what they print, and then

* upgrade the provenance of programs whose real output equals ours to `oracle`, and
* find real behavioral differences (a mismatch is a bug in the engine, a wrong expectation, or an
  intentional difference to document).

## Workflow

```text
tests/vba/*.bas --prepare--> bundle/ --(lab VM)--> results-<env>.jsonl + environment-<env>.json
                                                         |
                       report / apply  <-----------------+
                          |            |
                          |            +--> ' SOURCE: oracle <env> was: ...   (matching programs)
                          |            +--> tests/oracle/ENVIRONMENTS.md      (append-only)
                          |            +--> tests/oracle/mismatches/<env>.md  (everything else)
                          v
                    triage: fix the engine, fix a wrong expectation, or document an
                    intentional difference
```

1. **Prepare** (`cargo run -p vba-rs --example oracle -- prepare <dir>`): classifies every program as
   capturable or not, rewrites `Debug.Print` in the capturable ones, writes the importable module
   files, a generated entry module and `manifest.json`. The rewriting happens here, in Rust and under
   test, not in PowerShell, so the Windows script has no source-transformation logic.
2. **Lab run** (`tests/oracle/run-oracle.ps1`): see `tests/oracle/README.md` for the step-by-step
   procedure, prerequisites and safety rules. It produces the results file and the environment file.
3. **Report** (`... -- report results.jsonl [environment.json]`): runs every program through our
   engine and prints, per program, one of
   * `MATCH`: same output and same unhandled error (or none) as the oracle,
   * `MISMATCH`: different output, same error outcome,
   * `ERROR-DIFFERS`: the unhandled error number differs (including error versus no error),
   * `NOT-CAPTURABLE`: excluded by the scanner or `oracle-skip.txt`, refused by the driver, or our
     output has characters the ANSI log cannot carry exactly,
   * `ORACLE-FAILED`: the oracle run timed out or failed to start the program,
   * `NO-RESULT`: a capturable program has no line in the results,
   * `STALE`: the program changed after the bundle was prepared,
   with summary counts, a table by area and a table by current provenance, and a line diff
   (`-` oracle, `+` engine) for every difference.
4. **Apply** (`... -- apply results.jsonl environment.json`): for each `MATCH` that is not a recorded
   known deviation (`XFAIL`), rewrites `' SOURCE: <old>` to `' SOURCE: oracle <env id> was: <old>`;
   appends one row to `tests/oracle/ENVIRONMENTS.md` (never rewriting existing rows); writes the
   non-matching programs to `tests/oracle/mismatches/<env id>.md`. It never changes an expectation
   to match the oracle. It refuses an environment whose locale is not `en-US` (or that ran with a
   locale override) unless `--allow-locale` is given.
5. **Triage** each entry of the mismatch report, by hand: fix the engine, correct the expectation (and
   say why in its `SOURCE`), or keep the difference and document it as intentional (an `XFAIL` with a
   reason, or a note in the relevant docs page). Then run
   `VBA_BLESS=1 cargo test -p vba-rs --test conformance` to refresh [conformance](conformance.md).

## What `oracle` provenance means

`' SOURCE: oracle <env id> was: <previous kind> <previous reference>` states: **in that exact
environment, real VBA printed exactly this text and ended with this error outcome when the program
ran in the way the oracle driver runs it.** Nothing more. In particular it does not say that every
VBA version, locale or bitness behaves so.

The environment id is `<host>-<version>.<build>-<locale>-<bitness>`, for example
`excel-16.0.17328-en-US-x64` (illustrative). `ENVIRONMENTS.md` records for every id the host,
version, build, VBA version, bitness, OS, locale name and LCID, capture date, and how many programs
were upgraded or did not match. A test (`oracle_tooling`) fails when a program claims an environment
that is not in the table or drops the `was:` part. Pins:

* **Version and build**: VBA behavior changes between versions and builds. Only the exact build
  is confirmed; results from another build are another environment (and may be applied on top only
  for programs not yet `oracle`; an existing `oracle` tag is not overwritten).
* **Locale**: the documented oracle locale is **English (United States)** with default formats
  (decimal `.`, list `,`, short date `M/d/yyyy`, code page 1252). The driver checks it and refuses to
  run otherwise, or runs with an explicit override that is recorded and not applied by default.
* **Bitness**: recorded (`x64`/`x86`, from the host process). Behavior that depends on it
  (`Win64` constants, `LongPtr`, `Declare`) is largely outside the capturable set.
* The conformance runner accepts `oracle` as a provenance kind (the strongest one in the
  per-entry "weakest provenance" summary). Because one SOURCE line holds one environment, a program
  keeps the first environment that confirmed it; further environments are visible in their
  `mismatches/` reports and `ENVIRONMENTS.md` rows.

## What is and is not capturable

Of the 679 programs in the corpus at the time of writing, **581 are capturable** and **98 are not**:

| Reason | Programs | Examples |
| --- | ---: | --- |
| safety (needs a grant, touches files, registry, processes, COM, native code) | 71 | `GRANT: file-write`, `Shell`, `SaveSetting`, `Open`, `CreateObject`, `Environ` |
| engine-only (an engine-specific outcome or compile-error expectation) | 14 | `ERROR: load` (a real host shows a modal dialog), `limit`, `MAX_STEPS` |
| ui (a dialog or the debugger would open) | 6 | `MsgBox`, `InputBox`, `Stop`, `Beep`, `Load`/`Unload` |
| untranslatable (the rewriter refuses) | 6 | `Debug.Assert`, entry with parameters, entry under `#If`, output from `Class_Terminate` after the entry procedure returned (twice) |
| host (needs a document module or host event) | 1 | `RAISE:` events |

The list with a reason per program is `tests/oracle/oracle-skip.txt`. It is **default deny**: it is
generated from the scanner (`... -- skiplist`), may be extended by hand with category `manual`, and
both the preparer and the driver refuse every id in it. A test fails when the scanner refuses a program
that is not listed.

Capturable does not mean the comparison is complete: output printed to the Immediate window is the
only thing compared (plus the unhandled error number). Things that are not captured: the error
`Source`, the text of descriptions (stored but not compared), screen output, files, and time-dependent
values. Programs that depend on the clock or on randomness without a seed would show up as mismatches
and need triage, not trust.

## The `Debug.Print` shim

The real Immediate window cannot be read programmatically. The preparer therefore rewrites each
`Debug.Print` statement into a **real `Print #255,` statement** that writes to a log file opened by
`OracleHarness.Begin` under the reserved file number 255. `Print #` belongs to the same statement
family and the host applies the formatting itself; the shim does not re-implement any formatting.

Rewrite rules (`rewrite_line` in `crates/api/vba-rs/tests/oracle_support/mod.rs`, unit tested):

* `Debug.Print <args>` becomes `Print #255, <args>`; `;` and `,` separators, `Spc`, `Tab`, trailing
  `;`/`,` stay as written and keep their meaning (a trailing separator keeps the column state for
  the next statement, because the log is one continuous file).
* An argument-less `Debug.Print` becomes `Print #255, ""` (identical result: an empty line, or the end
  of a pending line), which avoids relying on how a bare `Print #255,` parses before `:` or `Else`.
* Only statements at the start of a statement are rewritten: the start of a line, after a line
  number or label, after `:`, after `Then` or `Else` of a single-line `If`.
* Text inside string literals and comments (`'`, `Rem`) is never touched.
* **Refused** (the whole program becomes NOT-CAPTURABLE): `Debug.Assert` or any other use of the
  `Debug` object, `Debug.Print` anywhere else than at a statement start (for example as an argument),
  a `Debug.Print` that starts with a separator, non-ASCII source text, a program that mentions
  `#255`, `Close` or `Reset` (they would hit the log), and modules named `OracleHarness` or
  `OracleEntry`. It prefers refusing to guessing.
* Output expectations: numbers keep the sign column and trailing space (the corpus already expects
  ` 16 `), lines end with CR LF in the log; the comparison normalizes CR LF to LF on both sides.
  Characters our engine prints that the ANSI (code page 1252) log cannot carry exactly make a
  program NOT-CAPTURABLE rather than compared.
* The unhandled-error outcome is recorded by the generated entry module (`On Error GoTo` around the
  call of the entry procedure; `Err.Number` and `Err.Description` stored in a status file). A program
  that uses `End` leaves a log but no status file and is recorded as `ended`.

The central assumption, that `Print #` and `Debug.Print` format identically, is **not verified**.
`tests/oracle/OracleCalibrate.bas` prints the same items both ways so a person can compare the
Immediate window with the log once per host version. Until that is done, treat any
`oracle` upgrade as conditional on it.

Self-check available without Windows: `oracle_tooling` rewrites every capturable program, runs it
in our engine with `Print #255` writing to a virtual file, replays the file and requires the same
output as the plain `Debug.Print` run. That tests that the rewritten text is valid and keeps
separator and column behavior in our engine; it says nothing about real VBA. (It found that our
lexer reads `Print #255, 5#` as a date literal; the check skips such programs, and the lexer
is worth a look.)

## Safety rules

* Run only in a throwaway VM restored from a snapshot, without network access, with no personal data.
* Macros run with full VBA power; "Trust access to the VBA project object model" is enabled for the
  automation and must not be enabled elsewhere.
* Never open untrusted files in that VM. The driver only creates its own scratch workbook/database
  and imports module files from the bundle.
* Default deny: the skip list; the driver refuses to run without `-ConfirmThrowawayVm` and with a
  network connection (unless overridden); the work directory must be an ASCII path.
* Results contain only program output and environment descriptions (machine-specific data such as
  user names or paths are excluded by the skip list, for example `Environ` and `CurDir`). Look
  through the files before sending them anyway.

## Sending results back

Send, from the lab: `results-<env id>.jsonl` and `environment-<env id>.json` (both small text
files), together with the output of the calibration comparison (empty means identical, otherwise
the differing lines), a note of anything unusual (dialogs seen, programs that needed `-Filter`
re-runs), and, if the driver needed edits, the diff. On the development machine run `report`, then
`apply` (see above), review the diff of `tests/vba/` and the new `mismatches/` file, and commit. Do
not edit results by hand. Real results are committed only through `apply` (the environment row and
provenance tags) and the generated mismatch report; the raw files can be attached to the
change that applies them.

## Reference

* Tooling code: `crates/api/vba-rs/tests/oracle_support/mod.rs` (library),
  `crates/api/vba-rs/examples/oracle.rs` (command line), `crates/api/vba-rs/tests/oracle_tooling.rs`
  (tests with synthetic results; no test writes into the committed corpus).
* Commands: `stats`, `skiplist`, `prepare <dir>`, `report <results> [<env>]`,
  `apply <results> <env> [--allow-locale]`.
* Windows side: `tests/oracle/README.md` (procedure, "Assumptions to check on first run").

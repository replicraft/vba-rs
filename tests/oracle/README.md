# Oracle harness (Windows lab)

**Status: the Windows side of this directory is UNTESTED.** `OracleHarness.bas`,
`OracleCalibrate.bas` and `run-oracle.ps1` were written and reviewed by reading only, on a machine
without Windows or Office. They have never been compiled or run. Expect to fix small things on the
first run, and read "Assumptions to check on first run" below before trusting any result. The Rust
side (bundle preparation, comparison, provenance upgrade) is tested; see
`docs/tooling/differential-testing.md` for the whole workflow.

Purpose: run the conformance programs in `tests/vba/` in real VBA (Excel or Access) and record what
they print, so that expectations can be upgraded to `oracle` provenance and real differences found.

## Files

| File | What it is |
| --- | --- |
| `OracleHarness.bas` | VBA module: opens the log (file number 255), writes the status file. |
| `OracleCalibrate.bas` | VBA module: one-time check that `Print #255` formats like `Debug.Print`. |
| `run-oracle.ps1` | PowerShell driver: automates Excel or Access through COM. |
| `oracle-skip.txt` | Default-deny list: program ids excluded from automatic runs, with reasons. |
| `ENVIRONMENTS.md` | Append-only table of environments whose results were applied. |
| `mismatches/<env>.md` | Reports of programs whose real behavior differs from ours. |

## Safety (read first)

* The programs run **real VBA with macros enabled**. Run them only in a **throwaway virtual
  machine** that you can restore from a snapshot, with **no network** (disconnect the adapter), no
  personal data, and no access to shares. The driver refuses to start without `-ConfirmThrowawayVm`
  and refuses when a network connection appears to be available (`-AllowNetwork` overrides that).
* Never open untrusted workbooks or databases in that VM, and never enable "Trust access to the VBA
  project object model" on a machine that is not disposable (it lets any macro rewrite macros).
* Default deny: `oracle-skip.txt` lists every program that is excluded, and the preparer derives it
  from a scanner. Excluded are programs that need a capability grant (file read/write, COM, native
  declare), and any program whose code mentions `Open`, `Close`, `Reset`, `Kill`, `Shell`,
  `SendKeys`, `AppActivate`, `SaveSetting`/`GetSetting`/`DeleteSetting`/`GetAllSettings`, `MkDir`,
  `RmDir`, `ChDir`, `ChDrive`, `FileCopy`, `Name ... As`, `Dir`, `FileLen`, `FileDateTime`,
  `GetAttr`, `SetAttr`, `Lock`, `Environ`, `Command`, `CurDir`, `CreateObject`, `GetObject`,
  `Declare`, `MsgBox`, `InputBox`, `Stop`, `Beep`, `Load`/`Unload`, host objects, and so on. The
  driver additionally re-reads `oracle-skip.txt` from the bundle and never runs a listed id.
  Review the file before every lab session; add ids by hand with category `manual`.
* The driver writes only below its work directory (default: a new folder in the temp directory,
  deleted at the end). The VBA code of the programs that do run never opens files (the scanner
  refuses `Open`), except the harness's own log and status files inside that work directory.

## Prerequisites

1. A Windows VM (Windows 10/11) with Microsoft Excel **or** Access installed and activated, desktop
   versions. Note the exact version (the driver records it).
2. Regional settings **English (United States)** with default formats, for the user that runs the
   driver: Settings > Time & language > Region > Regional format = English (United States), and the
   system locale for non-Unicode programs = English (United States) (code page 1252). Office
   display and proofing language English too: error descriptions come out in that language. The
   driver checks locale name `en-US`, decimal `.`, group `,`, list `,`, short date `M/d/yyyy` and ANSI
   code page 1252 and **refuses to run otherwise**. `-AcceptLocale` runs anyway and marks the
   environment `locale_override`; `apply` then refuses it unless given `--allow-locale`. Do not do
   this to upgrade provenance.
3. **Trust access to the VBA project object model**: File > Options > Trust Center > Trust Center
   Settings > Macro Settings > tick it (Excel and Access both have the setting). Alternatively
   set DWORD `AccessVBOM = 1` under `HKCU\Software\Microsoft\Office\<16.0>\<Excel|Access>\Security`.
   The driver checks this at start and tells you if it is missing.
4. PowerShell 5.1 or later (the one in Windows). Run in a normal (non-elevated) session as the same
   user that owns the Office installation. If script execution is restricted:
   `powershell -ExecutionPolicy Bypass -File .\run-oracle.ps1 ...`.
5. No other Excel/Access instance running (the driver starts its own hidden instance and kills only
   that process on timeout).

## Procedure

On the development machine:

1. `cargo run -p vba-rs --example oracle -- skiplist` (only if you changed programs; commit the file)
2. `cargo run -p vba-rs --example oracle -- prepare out/bundle`
   This rewrites `Debug.Print` statements and writes `manifest.json`, `OracleHarness.bas`,
   `oracle-skip.txt` and one directory per capturable program. Copy the `out/bundle` folder and
   `run-oracle.ps1` to the lab VM (for example in a zip; nothing else crosses over).

In the lab VM:

3. Snapshot the VM. Disconnect the network.
4. **Calibration (once per host version)**: open Excel (or Access), import `OracleCalibrate.bas`
   (Alt+F11, File > Import), and follow the steps in its header comment. The Immediate window text
   and the log file must be identical. If they differ, stop and report which items differ; the shim
   is then not faithful for them.
5. Run the driver:

   ```powershell
   .\run-oracle.ps1 -HostApp excel -Bundle C:\oracle\bundle -OutDir C:\oracle\out -ConfirmThrowawayVm
   ```

   Use `-HostApp access` for Access. Useful switches: `-Filter 'strings/*'` (a subset first is
   wise), `-TimeoutSeconds 60`, `-Resume` (continue an interrupted run), `-WorkDir C:\oracle\work`.
   It prints one line per program. A full run takes a while; each program gets a fresh scratch
   workbook/database, and the host is restarted every 100 programs and after each timeout.
6. Collect `C:\oracle\out\results-<envid>.jsonl` and `environment-<envid>.json`. Run once for Excel
   and once for Access if both are available; they are separate environments.
7. Restore the snapshot.

Back on the development machine:

8. Put the two files somewhere (not in `tests/`) and look at the report:
   `cargo run -p vba-rs --example oracle -- report results-<envid>.jsonl environment-<envid>.json`
9. If the report is sensible, `cargo run -p vba-rs --example oracle -- apply results-<envid>.jsonl environment-<envid>.json`
   rewrites `' SOURCE:` of matching programs to `oracle`, appends the environment to
   `ENVIRONMENTS.md` and writes `mismatches/<envid>.md`. Mismatches are never auto-fixed.
10. `VBA_BLESS=1 cargo test -p vba-rs --test conformance` to regenerate the conformance summary, then
    triage `mismatches/<envid>.md` (see the docs page).

## Result format

`results-<envid>.jsonl`, one JSON object per line:
`id, env, host, version, build, bitness, status, output, error_number, error_description, reason, digest`.

| status | meaning |
| --- | --- |
| `ok` | The entry procedure returned; `output` holds what it printed. |
| `error` | An unhandled run-time error ended the program; `error_number` and `error_description` are set; `output` is what was printed before it. |
| `ended` | Log exists but there is no completion marker: the program used `End`, or ended abnormally. `output` is kept; no error is assumed. |
| `timeout` | No result within the time limit; the host was killed. Output is discarded. |
| `host-failure` | The program did not start (import, compile or COM failure); `error_description` has the message. |
| `not-capturable` | Not run (see `reason`). |

`digest` is a hash of the program text at bundle time; the comparison reports STALE when the corpus
changed afterwards. `environment-<envid>.json` holds host name, version, build, product version,
VBA version, bitness, OS, locale name and LCID, separators, `locale_override`, capture time.

## Assumptions to check on first run

Result of a line-by-line review of `run-oracle.ps1`, `OracleHarness.bas`, `OracleCalibrate.bas` and
the generated `OracleEntry` module. Each item is something that could not be verified without Office.
If the first run misbehaves, start here.

Capture design (VBA):

1. `Print #255, args` produces exactly the text `Debug.Print args` shows in the Immediate window
   (sign column and trailing space of numbers, 14-column zones, `Spc`/`Tab`, Boolean/Date/Null/
   Empty/Error text). `OracleCalibrate.bas` exists to check this; do it first.
2. `Print #255, ""` ends a line exactly as an argument-less `Debug.Print` does; and a `Debug.Print`
   that ends with `;` or `,` leaves the column state for the next statement like `Print #` does.
3. File number 255 is free and openable for output; `FreeFile` in a program never returns it
   (the scanner also rejects programs that mention `#255`, `Close` or `Reset`).
4. `Print #` writes ANSI text (code page 1252 under en-US). Characters outside it become `?` or a
   best-fit letter; the comparison treats programs whose output has such characters as not
   capturable. The driver decodes the log as Windows-1252.
5. Line ends: `Print #` ends lines with CR LF. The comparison normalizes CR LF to LF on both sides, so
   a program that prints `vbCrLf` itself is indistinguishable from one printing a plain line end.
6. The Immediate window is not used at all. An unfinished line (trailing `;`) stays unfinished in
   the log; if your VBA host flushes it differently the last line can differ.
7. If an argument expression raises an error half-way through a `Print #` list, real VBA may or may
   not have written the earlier items before raising; the Immediate window may behave differently
   from a file.
8. A program ends with the final `OracleHarness.Finish`. Output printed after the entry procedure
   returns (for example by `Class_Terminate` when module-level objects are released at project
   teardown) cannot be logged because the log is closed first. The preparer detects such programs
   by asking our engine whether it printed anything during shutdown, and refuses them; if real VBA
   tears down differently that detection can be wrong.
9. Unhandled errors are trapped by `On Error GoTo` in the generated `OracleRun`, so `Err.Number` and
   `Err.Description` are those of the original error. Real unhandled errors would show a dialog;
   `Err.Source` may differ from what a dialog would show.
10. Calling `Module1.Main` directly: if the entry procedure is a `Function`, the value is discarded.
11. Imported class modules get `Attribute VB_PredeclaredId = False` and `VB_Exposed = False`;
    programs relying on a predeclared instance of an ordinary class would differ.

Driver (PowerShell/COM):

12. `Get-Culture` / `CultureInfo.CurrentCulture` reflects the user's regional settings (including
    customised separators) and is what VBA uses. Office's own language settings are not checked;
    keep them English. The `ListSeparator` property is from the user's settings.
13. `New-Object -ComObject Excel.Application` / `Access.Application` start a hidden instance;
    `Excel.Hwnd` and `Access.hWndAccessApp()` return a window handle whose owner process is the host
    process (used for the watchdog and for bitness via `IsWow64Process`).
14. `Workbooks.Add()` creates a workbook whose `VBProject` is reachable when the trust setting is on;
    for Access, `NewCurrentDatabase` then `VBE.VBProjects.Item(1)` is the new database's project.
    (`VBE.ActiveVBProject` may be empty in a hidden Access, hence `Item(1)`.)
15. `VBComponents.Import` accepts `.bas`/`.cls` files with the headers written by the preparer
    (`Attribute VB_Name`, and the `VERSION 1.0 CLASS` block for classes), taking the module name
    from `VB_Name`, with CR LF line ends and ASCII text.
16. `Application.Run "'Book1'!OracleRun", log, status` (Excel) and `Application.Run "OracleRun",
    log, status` (Access) call a public sub in a standard module with two string arguments, with
    macros allowed to run in a workbook/database created by automation. If Office policy blocks
    macros every program will end as `host-failure`.
17. No completion marker but a log file means `ended` (the `End` statement). If `Application.Run`
    behaves differently after `End` (it may raise a COM exception), the message is stored in
    `error_description` but the status is still `ended`.
18. A compile error in an imported program raises a modal VBE dialog that COM cannot dismiss, so it
    shows up as `timeout` (host killed), not as a compile error. Programs expected not to compile
    are excluded.
19. The watchdog (C# `System.Threading.Timer`) kills the host process on expiry; the blocked COM call
    then fails with an RPC error and the driver starts a new host. `DisplayAlerts = False` is set
    for Excel; Access runs hidden.
20. Access modules imported without an `Option Compare` line: whether Access then compares as
    Binary (what the engine assumes) or as Database is not verified. Programs doing string
    comparison can differ between Excel and Access for that reason; compare both hosts.
21. A fresh workbook/database per program isolates module-level state and class instances; user
    module names that clash with host or library names (for example a class called `Range`) would
    break the import for that program (recorded as `host-failure`).
22. `ConvertTo-Json -Compress` output (with `\u0027`-style escapes for some characters) is valid JSON;
    the file is written as UTF-8 without BOM. Strings are the Windows-1252 decoding of the log.
23. `ConvertFrom-Json` on the manifest (about 700 entries) works in Windows PowerShell 5.1 and a
    one-element `reasons`/`files` array is wrapped with `@()` before use.
24. The temp path must be ASCII; otherwise VBA's `Open` could mangle it. The driver checks.
25. Excel `Application.Build` and `Application.Version` identify the build; `Version` is "16.0" for
    2016 through Microsoft 365, so the build number (and the recorded product version) is what
    distinguishes updates.

## Troubleshooting

* Everything is `host-failure`: read `error_description` of the first lines. Typical causes: the
  VBA-project trust setting, macro policy, a locked-down Office, or the import format.
* Everything is `timeout`: a dialog is probably open on the (hidden) instance. Try running with a
  visible instance by temporarily editing `Start-Host` (`Visible = $true`) and watch one program.
* `apply` refuses the environment: locale is not en-US, see above.

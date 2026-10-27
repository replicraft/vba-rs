VBA oracle run: instructions for the lab machine
================================================

What this is: this folder holds conformance programs for a VBA implementation and a script that
runs them in your real Microsoft Excel (or Access) and records what they print. Nothing is sent
anywhere; you send the result file back yourself.

What you need (once):

  * A throwaway Windows 10/11 virtual machine you can restore from a snapshot, with desktop
    Excel (or Access) installed and activated, and no personal data. Take a snapshot first and
    disconnect the network adapter: the programs run real VBA with macros enabled.
  * Region set to English (United States) with the default formats, system locale for
    non-Unicode programs English (United States), Office display language English. The script
    checks this and refuses to run otherwise.
  * In Excel: File > Options > Trust Center > Trust Center Settings > Macro Settings > tick
    "Trust access to the VBA project object model". (Access has the same setting.)
  * Windows PowerShell 5.1 (already part of Windows). Nothing else is installed.

The one command (PowerShell, not elevated, opened in THIS folder, every Excel window closed):

    powershell -ExecutionPolicy Bypass -File .\run-oracle.ps1 -ConfirmThrowawayVm

  * For Access add: -HostApp access (a separate run, a separate result).
  * To try a few programs first add: -Filter 'strings/*'.
  * If it is interrupted, run the same command again with -Resume.

When it finishes it prints a count per status and writes, in the "out" folder next to this file:

    results-<id>.jsonl        what each program printed in real VBA (one JSON object per line)
    environment-<id>.json     host, version, build, bitness, OS, locale
    return-<id>.zip           both files, zipped: this is the file to send back

Then restore the snapshot.

Checklist of what to send back:

  [ ] return-<id>.zip (or the two files inside it, unchanged)
  [ ] Host and edition: Excel or Access, 32- or 64-bit, Microsoft 365 or a perpetual licence
  [ ] Whether the run was complete (no -Filter) or partial
  [ ] Any program that showed a dialog or needed a restart
  [ ] Any change you made to run-oracle.ps1 (send the diff)
  [ ] The result of the one-time calibration, if you did it (tests/oracle/README.md, step 4 of
      the procedure: it compares what Debug.Print and Print # produce for the same items)

Do not edit the result files. If the first run misbehaves, tests/oracle/README.md has a section
"Assumptions to check on first run" and a troubleshooting list.

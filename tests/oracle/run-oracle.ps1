#Requires -Version 5.1
<#
.SYNOPSIS
  Runs the vba-rs conformance programs in real Microsoft Excel or Access and
  records what they print (the oracle run).

.DESCRIPTION
  STATUS: UNTESTED. This script was written on a machine without Windows or
  Office and has never been executed. Read tests/oracle/README.md first,
  especially "Assumptions to check on first run". Run it only inside a
  throwaway virtual machine.

  Input is a bundle made on the development machine with
    cargo run -p vba-rs --example oracle -- prepare <bundle-dir>
  The bundle holds manifest.json, OracleHarness.bas, oracle-skip.txt and one
  directory of ready-to-import module files per capturable program. This script
  does no source rewriting itself.

  For every program it creates a scratch workbook (Excel) or database (Access)
  in a work directory, imports OracleHarness, the program's modules and the
  generated OracleEntry, runs OracleRun (module OracleEntry) with a watchdog timeout,
  reads the log file and the status file, tears the scratch document down, and
  appends one JSON line to results-<envid>.jsonl. It also writes
  environment-<envid>.json (versions, build, bitness, OS, locale, separators).

  Output line fields: id, env, host, version, build, bitness, status, output,
  error_number, error_description, reason, digest.
  status is one of: ok, error, ended, timeout, host-failure, not-capturable.

.PARAMETER HostApp
  excel (default) or access.

.PARAMETER Bundle
  Directory produced by the "prepare" command. Default: the folder this
  script is in (the prepared bundle contains a copy of the script).

.PARAMETER OutDir
  Where results-<envid>.jsonl, environment-<envid>.json and return-<envid>.zip
  are written. Default: an "out" folder next to this script.

.PARAMETER TimeoutSeconds
  Per-program limit for importing and running. On expiry the host process is
  killed (this is also how unexpected modal dialogs are dealt with), the
  program is recorded as timeout and a fresh host is started.

.PARAMETER Filter
  Wildcard on the program id (for example 'strings/*'). Default: all.

.PARAMETER WorkDir
  Scratch directory (must have an ASCII-only path). Default: a new directory
  under the temp folder. It is deleted at the end.

.PARAMETER AcceptLocale
  Run even though the regional settings are not the documented en-US
  settings. The environment is then marked locale_override and "apply" will
  refuse it unless told otherwise. Prefer fixing the regional settings.

.PARAMETER ConfirmThrowawayVm
  Required. States that this machine is a disposable VM, snapshot-restorable,
  without network access to anything that matters.

.PARAMETER AllowNetwork
  Run even though a network connection appears to be available.

.PARAMETER Resume
  Skip program ids that already have a line in the results file.

.PARAMETER RestartEvery
  Restart the host application after this many programs (default 100).

.EXAMPLE
  .\run-oracle.ps1 -ConfirmThrowawayVm

.EXAMPLE
  .\run-oracle.ps1 -HostApp access -Bundle C:\oracle\bundle -OutDir C:\oracle\out -ConfirmThrowawayVm
#>
[CmdletBinding()]
param(
    [ValidateSet('excel', 'access')][string]$HostApp = 'excel',
    [string]$Bundle = $PSScriptRoot,
    [string]$OutDir = (Join-Path $PSScriptRoot 'out'),
    [int]$TimeoutSeconds = 30,
    [string]$Filter = '*',
    [string]$WorkDir = '',
    [switch]$AcceptLocale,
    [switch]$ConfirmThrowawayVm,
    [switch]$AllowNetwork,
    [switch]$Resume,
    [int]$RestartEvery = 100
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'

$DriverVersion = '1'
$Ansi = [System.Text.Encoding]::GetEncoding(1252)
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

# ---------------------------------------------------------------- guards

if ($env:OS -ne 'Windows_NT') { throw 'This script drives Microsoft Office through COM and runs on Windows only.' }
if (-not $ConfirmThrowawayVm) {
    throw 'Refusing to run: pass -ConfirmThrowawayVm to state that this is a disposable VM. The programs run real VBA with macros enabled.'
}
if (-not $AllowNetwork -and [System.Net.NetworkInformation.NetworkInterface]::GetIsNetworkAvailable()) {
    throw 'Refusing to run: a network connection appears to be available. Disconnect the VM, or pass -AllowNetwork if the network is isolated.'
}
$Bundle = (Resolve-Path -LiteralPath $Bundle).Path
$manifestPath = Join-Path $Bundle 'manifest.json'
if (-not (Test-Path -LiteralPath $manifestPath)) { throw "No manifest.json in $Bundle. Make the bundle with the 'prepare' command." }

# ---------------------------------------------------------------- locale

# The documented oracle locale. VBA formats numbers, dates and Booleans with the
# user's regional settings, so a run under any other settings is not comparable.
$expectedLocale = [ordered]@{
    locale_name       = 'en-US'
    decimal_separator = '.'
    group_separator   = ','
    list_separator    = ','
    short_date        = 'M/d/yyyy'
    ansi_codepage     = 1252
}

function Get-LocaleReport {
    $c = [System.Globalization.CultureInfo]::CurrentCulture
    [ordered]@{
        locale_name       = $c.Name
        lcid              = $c.LCID
        decimal_separator = $c.NumberFormat.NumberDecimalSeparator
        group_separator   = $c.NumberFormat.NumberGroupSeparator
        list_separator    = $c.TextInfo.ListSeparator
        short_date        = $c.DateTimeFormat.ShortDatePattern
        ansi_codepage     = $c.TextInfo.ANSICodePage
    }
}

$locale = Get-LocaleReport
$localeProblems = @()
foreach ($k in $expectedLocale.Keys) {
    if ("$($locale[$k])" -ne "$($expectedLocale[$k])") {
        $localeProblems += "$k is '$($locale[$k])', expected '$($expectedLocale[$k])'"
    }
}
$localeOverride = $false
if ($localeProblems.Count -gt 0) {
    if (-not $AcceptLocale) {
        throw ("Refusing to run: the regional settings are not en-US:`n  " + ($localeProblems -join "`n  ") +
            "`nSet Region to English (United States) with the default formats (Settings > Time & language > Region) and sign in again, or pass -AcceptLocale to record the run as a locale override.")
    }
    Write-Warning ('Running with a locale override: ' + ($localeProblems -join '; '))
    $localeOverride = $true
}

# ---------------------------------------------------------- work directory

if ([string]::IsNullOrEmpty($WorkDir)) {
    $WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ('vba-oracle-' + [System.Diagnostics.Process]::GetCurrentProcess().Id)
}
if ($WorkDir -notmatch '^[\x20-\x7E]+$') {
    throw "The work directory path '$WorkDir' contains non-ASCII characters; VBA's Open would mangle it. Pass -WorkDir with an ASCII path."
}
New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutDir = (Resolve-Path -LiteralPath $OutDir).Path

# ------------------------------------------------------------ native helper

# Pure C#, so it works without a PowerShell runspace on the timer thread.
if (-not ('OracleNative' -as [type])) {
Add-Type -TypeDefinition @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading;

public static class OracleNative
{
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);

    [DllImport("kernel32.dll")]
    private static extern bool IsWow64Process(IntPtr process, out bool wow64);

    public static int PidFromHwnd(int hwnd)
    {
        uint pid;
        GetWindowThreadProcessId(new IntPtr(hwnd), out pid);
        return (int)pid;
    }

    // True when the process is a 32-bit process on 64-bit Windows.
    public static bool IsWow64(int pid)
    {
        bool wow;
        using (Process p = Process.GetProcessById(pid))
        {
            IsWow64Process(p.Handle, out wow);
        }
        return wow;
    }
}

public sealed class OracleWatchdog : IDisposable
{
    private readonly Timer timer;
    public volatile bool Fired;

    public OracleWatchdog(int pid, int milliseconds)
    {
        timer = new Timer(delegate(object state)
        {
            Fired = true;
            try { Process.GetProcessById(pid).Kill(); } catch (Exception) { }
        }, null, milliseconds, Timeout.Infinite);
    }

    public void Dispose()
    {
        timer.Dispose();
    }
}
'@
}

# ------------------------------------------------------------- host handling

function Start-Host {
    if ($HostApp -eq 'excel') {
        $app = New-Object -ComObject Excel.Application
        $app.Visible = $false
        $app.DisplayAlerts = $false
        $app.ScreenUpdating = $false
        $hwnd = [int]$app.Hwnd
    }
    else {
        $app = New-Object -ComObject Access.Application
        $app.Visible = $false
        $hwnd = [int]$app.hWndAccessApp()
    }
    $hostPid = [OracleNative]::PidFromHwnd($hwnd)
    if ($hostPid -le 0) { throw 'Could not determine the process id of the host application; the watchdog cannot be armed.' }
    [pscustomobject]@{ App = $app; Pid = $hostPid }
}

function Stop-Host($h) {
    if ($null -eq $h) { return }
    try { $h.App.Quit() } catch { }
    try { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($h.App) } catch { }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
    try {
        $p = Get-Process -Id $h.Pid -ErrorAction SilentlyContinue
        if ($p) {
            if (-not $p.WaitForExit(5000)) { $p.Kill() }
        }
    } catch { }
}

# Creates the scratch document and returns what is needed to use and remove it.
function New-Scratch($h, [string]$dir) {
    if ($HostApp -eq 'excel') {
        $wb = $h.App.Workbooks.Add()
        [pscustomobject]@{ Doc = $wb; Project = $wb.VBProject; RunName = "'" + $wb.Name + "'!OracleRun" }
    }
    else {
        $db = Join-Path $dir 'scratch.accdb'
        $h.App.NewCurrentDatabase($db)
        [pscustomobject]@{ Doc = $db; Project = $h.App.VBE.VBProjects.Item(1); RunName = 'OracleRun' }
    }
}

function Remove-Scratch($h, $scratch) {
    if ($null -eq $scratch) { return }
    try {
        if ($HostApp -eq 'excel') {
            $scratch.Doc.Saved = $true
            $scratch.Doc.Close($false)
        }
        else {
            $h.App.CloseCurrentDatabase()
        }
    } catch { }
}

# ----------------------------------------------------------------- manifest

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$skipIds = @{}
$skipPath = Join-Path $Bundle 'oracle-skip.txt'
if (Test-Path -LiteralPath $skipPath) {
    foreach ($line in (Get-Content -LiteralPath $skipPath -Encoding UTF8)) {
        if ($line.Trim().Length -eq 0 -or $line.StartsWith('#')) { continue }
        $parts = $line.Split("`t")
        $skipIds[$parts[0].Trim()] = $line
    }
}

# --------------------------------------------------------------- environment

$hostObj = Start-Host
$app = $hostObj.App

# Without programmatic access to the VBA project the whole run is impossible.
$probe = $null
try {
    $probeDir = Join-Path $WorkDir 'probe'
    New-Item -ItemType Directory -Force -Path $probeDir | Out-Null
    $probe = New-Scratch $hostObj $probeDir
    [void]$probe.Project.VBComponents.Count
}
catch {
    Remove-Scratch $hostObj $probe
    Stop-Host $hostObj
    throw ("Cannot reach the VBA project object model: $($_.Exception.Message)`n" +
        "Enable File > Options > Trust Center > Trust Center Settings > Macro Settings > 'Trust access to the VBA project object model', or set the registry value AccessVBOM=1 (DWORD) under HKCU\Software\Microsoft\Office\<version>\<Excel|Access>\Security, then run again.")
}

$vbaVersion = ''
try { $vbaVersion = [string]$app.VBE.Version } catch { }
$wow = [OracleNative]::IsWow64($hostObj.Pid)
$osIs64 = [System.Environment]::Is64BitOperatingSystem
$bitness = if ($osIs64 -and -not $wow) { 'x64' } else { 'x86' }
$productVersion = ''
try { $productVersion = (Get-Process -Id $hostObj.Pid).MainModule.FileVersionInfo.ProductVersion } catch { }
$osCaption = ''
try {
    $osInfo = Get-CimInstance Win32_OperatingSystem
    $osCaption = "$($osInfo.Caption) build $($osInfo.BuildNumber)"
} catch { }
$hostVersion = [string]$app.Version
$hostBuild = [string]$app.Build
$hostName = [string]$app.Name

$hostSeparators = [ordered]@{}
if ($HostApp -eq 'excel') {
    try {
        $hostSeparators['excel_decimal_separator'] = [string]$app.International(3)
        $hostSeparators['excel_list_separator'] = [string]$app.International(5)
        $hostSeparators['excel_use_system_separators'] = [bool]$app.UseSystemSeparators
    } catch { }
}

$envId = ("{0}-{1}.{2}-{3}-{4}" -f $HostApp, $hostVersion, $hostBuild, $locale['locale_name'], $bitness) -replace '[^A-Za-z0-9._-]', '_'
$environment = [ordered]@{
    schema           = 1
    env_id           = $envId
    host             = $HostApp
    host_name        = $hostName
    version          = $hostVersion
    build            = $hostBuild
    product_version  = $productVersion
    vba_version      = $vbaVersion
    bitness          = $bitness
    os_version       = [System.Environment]::OSVersion.VersionString
    os_caption       = $osCaption
    locale_name      = $locale['locale_name']
    lcid             = [string]$locale['lcid']
    decimal_separator = $locale['decimal_separator']
    group_separator  = $locale['group_separator']
    list_separator   = $locale['list_separator']
    short_date       = $locale['short_date']
    ansi_codepage    = $locale['ansi_codepage']
    locale_override  = $localeOverride
    captured_at      = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    driver_version   = $DriverVersion
    timeout_seconds  = $TimeoutSeconds
}
foreach ($k in $hostSeparators.Keys) { $environment[$k] = $hostSeparators[$k] }
$envPath = Join-Path $OutDir "environment-$envId.json"
[System.IO.File]::WriteAllText($envPath, ((($environment | ConvertTo-Json -Depth 4)) + "`n"), $Utf8NoBom)
Write-Host "Environment $envId"

Remove-Scratch $hostObj $probe

# ------------------------------------------------------------------- results

$resultsPath = Join-Path $OutDir "results-$envId.jsonl"
$done = @{}
if ($Resume -and (Test-Path -LiteralPath $resultsPath)) {
    foreach ($line in (Get-Content -LiteralPath $resultsPath -Encoding UTF8)) {
        if ($line.Trim().Length -gt 0) { $done[(($line | ConvertFrom-Json).id)] = $true }
    }
}

function Write-Result([string]$id, [string]$digest, [string]$status, $output, $errNumber, $errDescription, $reason) {
    $r = [ordered]@{
        id                = $id
        env               = $envId
        host              = $HostApp
        version           = $hostVersion
        build             = $hostBuild
        bitness           = $bitness
        status            = $status
        output            = $output
        error_number      = $errNumber
        error_description = $errDescription
        reason            = $reason
        digest            = $digest
    }
    $json = $r | ConvertTo-Json -Compress -Depth 3
    [System.IO.File]::AppendAllText($resultsPath, $json + "`n", $Utf8NoBom)
}

function Read-Status([string]$path) {
    $s = @{}
    foreach ($line in [System.IO.File]::ReadAllLines($path, $Ansi)) {
        $i = $line.IndexOf('=')
        if ($i -gt 0) { $s[$line.Substring(0, $i)] = $line.Substring($i + 1) }
    }
    $s
}

# Runs one capturable program. Returns a hashtable describing the outcome and
# whether the host must be restarted.
function Invoke-Program($entry) {
    $work = Join-Path $WorkDir ('p' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Force -Path $work | Out-Null
    $log = Join-Path $work 'log.txt'
    $status = Join-Path $work 'status.txt'
    $scratch = $null
    $watchdog = $null
    $failure = $null
    $fired = $false
    try {
        $watchdog = New-Object OracleWatchdog($hostObj.Pid, ($TimeoutSeconds * 1000))
        try {
            $scratch = New-Scratch $hostObj $work
            [void]$scratch.Project.VBComponents.Import((Join-Path $Bundle 'OracleHarness.bas'))
            $dir = Join-Path $Bundle $entry.dir
            foreach ($f in @($entry.files)) {
                [void]$scratch.Project.VBComponents.Import((Join-Path $dir $f))
            }
            [void]$hostObj.App.Run($scratch.RunName, $log, $status)
        }
        catch {
            $failure = $_.Exception.Message
        }
        finally {
            $fired = $watchdog.Fired
            $watchdog.Dispose()
        }
        if ($fired) {
            return @{ status = 'timeout'; output = $null; number = $null; description = "no result within $TimeoutSeconds s; the host was killed (a modal dialog or an endless loop)"; restart = $true }
        }
        $hasLog = Test-Path -LiteralPath $log
        $hasStatus = Test-Path -LiteralPath $status
        if (-not $hasLog) {
            return @{ status = 'host-failure'; output = $null; number = $null; description = "the program did not start: $failure"; restart = $false }
        }
        Remove-Scratch $hostObj $scratch
        $scratch = $null
        $output = $Ansi.GetString([System.IO.File]::ReadAllBytes($log))
        if (-not $hasStatus) {
            return @{ status = 'ended'; output = $output; number = $null; description = "no completion marker (End statement or abnormal end): $failure"; restart = $false }
        }
        $st = Read-Status $status
        $n = [int64]$st['number']
        if ($n -eq 0) {
            return @{ status = 'ok'; output = $output; number = $null; description = $null; restart = $false }
        }
        return @{ status = 'error'; output = $output; number = $n; description = $st['description']; restart = $false }
    }
    catch {
        return @{ status = 'host-failure'; output = $null; number = $null; description = $_.Exception.Message; restart = $true }
    }
    finally {
        Remove-Scratch $hostObj $scratch
        try { Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue } catch { }
    }
}

# ----------------------------------------------------------------------- main

$count = 0
$statusCounts = @{}
$sinceRestart = 0
try {
    foreach ($entry in @($manifest.programs)) {
        $id = [string]$entry.id
        if ($id -notlike $Filter) { continue }
        if ($done.ContainsKey($id)) { continue }
        $digest = [string]$entry.digest

        if ($skipIds.ContainsKey($id)) {
            Write-Result $id $digest 'not-capturable' $null $null $null 'listed in oracle-skip.txt'
            continue
        }
        if ($entry.status -ne 'capturable') {
            Write-Result $id $digest 'not-capturable' $null $null $null ((@($entry.reasons) -join '; '))
            continue
        }

        if ($sinceRestart -ge $RestartEvery) {
            Stop-Host $hostObj
            $hostObj = Start-Host
            $sinceRestart = 0
        }
        $r = Invoke-Program $entry
        $sinceRestart++
        $count++
        Write-Result $id $digest $r.status $r.output $r.number $r.description $null
        $statusCounts[$r.status] = 1 + [int]$statusCounts[$r.status]
        Write-Host ("{0,-60} {1}" -f $id, $r.status)
        if ($r.restart) {
            Stop-Host $hostObj
            $hostObj = Start-Host
            $sinceRestart = 0
        }
    }
}
finally {
    Stop-Host $hostObj
    try { Remove-Item -LiteralPath $WorkDir -Recurse -Force -ErrorAction SilentlyContinue } catch { }
}

Write-Host "Ran $count programs. Results: $resultsPath"
Write-Host "Environment: $envPath"
foreach ($k in ($statusCounts.Keys | Sort-Object)) { Write-Host ("  {0,-14} {1}" -f $k, $statusCounts[$k]) }

# One file to send back: the results and the environment description, unchanged.
$zipPath = Join-Path $OutDir "return-$envId.zip"
try {
    Compress-Archive -LiteralPath $resultsPath, $envPath -DestinationPath $zipPath -Force
    Write-Host "Send this file back: $zipPath"
}
catch {
    Write-Warning "Could not make the zip ($($_.Exception.Message)); send the two files above instead."
}

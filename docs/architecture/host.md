# vba-host

The contract an embedder implements to run VBA with vba-rs. Crate:
`crates/hosts/vba-host`. Depends on `vba-automation`, `vba-profile`,
`vba-value`, `vba-core`. It knows no host application (no Office, Access or
embedding-application types).

## Implemented

| Area | Item | Notes |
| --- | --- | --- |
| Metadata | `HostMetadata` | Name, version, and a `vba_profile::Profile` (language generation, host application seen by `#If`, OS, bitness). |
| Contract | `Host` trait | Accessors (`metadata`, `policy`, `budget`/`limits`, `objects`, `ui`, `events`, `references`) and hooks (`lookup_global`, `create_object_hook`, `get_object_hook`, `declare_hook`, `idle`). |
| Entry points | provided `Host` methods | `resolve_global`, `create_object`, `get_object`, `call_declare`, `do_events`, `check`. They enforce the policy before any hook, so a hook cannot be reached without a grant. |
| Globals | `lookup_global` | `Application`, intrinsic host globals and objects exported by project references are all resolved by the embedder; names are case-insensitive in the provided hosts. Object results carry a caller-owned reference. |
| References | `ProjectReference` | Name, GUID, version; metadata only. |
| `CreateObject` / `GetObject` | `create_object`, `get_object` | ComCreate for the ProgID/class, Network additionally for a remote server, FileRead for a path; neither argument is error 5. |
| `Declare` | `DeclareRequest`, `call_declare` | Opaque to this crate; requires NativeDeclare for `library!procedure`. |
| Permissions | `Capability`, `Policy` | FileRead, FileWrite, ProcessSpawn, Network, ComCreate, NativeDeclare, UiAutomation. Deny by default. Grants are unrestricted or prefix-scoped (case-insensitive; `..` segments never match a scoped grant). Denial is error 70 "Permission denied", detail in `Err.Source`. |
| Limits | `ResourceLimits`, `Budget` | Plain data: step (fuel), memory and call-depth limits. `Budget` meters them; exhaustion maps to errors 18 (steps), 7 (memory), 28 (call depth). |
| Safe points | `do_events` / `idle` | Charges one step, runs the idle hook (which may cancel by returning an error), returns 0. |
| UI | `UiHooks`, `HeadlessUi` | `MsgBox`/`InputBox` as an optional trait; the headless default fails with error 70 and a description naming the missing UI. It never fabricates an answer. |
| Events | `EventDispatch`, `EventRouter` | Source/event names are opaque strings, case-insensitive; handlers run in registration order; ByRef arguments written back; no handler is not an error. |
| Hosts | `HeadlessHost` | Grants nothing, creates nothing (429), gets nothing (432), no `Declare` (48), UI fails; configurable globals, metadata, policy, limits, references. |
| Hosts | `ScriptedHost` | Configurable: factories per ProgID, named objects for `GetObject`, `Declare` implementations, queued MsgBox/InputBox answers (error 5 when exhausted), idle cancellation (error 18), hook-call log. |

Error choices that have no exact VBA analogue: the headless UI uses 70; an
exhausted step budget uses 18 ("User interrupt occurred"). They are
documented decisions, not claims about real VBA behaviour.

## Settings, clock and UI-automation hooks

* `Host::settings()` returns an optional `SettingsStore` behind `SaveSetting`, `GetSetting`,
  `GetAllSettings` and `DeleteSetting`. Without one the VM uses a `MemorySettings` of its own that
  lives as long as the VM; the registry is never touched. Names are case-insensitive, values are
  strings, `GetAllSettings` returns rows ordered by key (**unverified**), a missing target of
  `DeleteSetting` is error 5 (**unverified** number; the page only says a run-time error occurs).
* `Host::set_clock` (`Date =`, `Time =`) forwards to `set_clock_hook`, whose default refuses with
  error 70. The policy has no capability for the clock; refusing is the default behaviour of the hook.
* `Host::send_keys` / `Host::app_activate` (`SendKeys`, `AppActivate`) check
  `Capability::UiAutomation` and then call `send_keys_hook` / `app_activate_hook`. The default hooks
  have no UI to drive and fail with error 70, so granting the capability alone never sends input.

## Not implemented

* `vba-vm` calls these hooks: `lookup_global` for library globals, `create_object` / `get_object` for
  `CreateObject` / `GetObject`, `check` before every file and process operation, `call_declare`-style
  checks and `declare_hook` for `Declare`, `ui()` for `MsgBox` / `InputBox`, `idle()` at `DoEvents`, and the
  budget for steps, memory and call depth. Limit exhaustion is an untrappable fault in the VM, not
  the trappable errors 18 / 7 / 28 that `LimitExceeded::into_err_info` describes. See `docs/runtime/vm.md`.
* File and process operations themselves are not provided, only the capability checks for them. Path canonicalisation (symlinks, `.`, drive aliases) is left to the embedder; the policy compares text prefixes.
* No network or process hooks beyond the capability names; the UI-automation hooks above only forward to the embedder.
* `MsgBox` help-file and context arguments, `InputBox` position and `DoEvents` form counts are not modelled.
* Threading: hosts are single-threaded (`Cell`/`RefCell`); no `Send`/`Sync`.

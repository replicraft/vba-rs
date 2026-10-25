# vba-automation

COM-free model of OLE Automation as seen from VBA. Crate:
`crates/interoperability/vba-automation`. Depends only on `vba-core` and
`vba-value`. Sources: MS-OAUT (IDispatch, VARIANT, DISPID, DISPPARAMS, invoke
flags, EXCEPINFO, HRESULT) and MS-VBAL §5 call semantics. Everything marked
"implemented" has tests; nothing is claimed beyond them.

## Implemented

| Area | Module | Notes |
| --- | --- | --- |
| `IDispatch` | `dispatch` | `Dispatch` trait: `member_id`, `member_info`, `get_ids_of_names` (member + named params), `invoke`, `terminate`, `held_references`. |
| DISPIDs and flags | `dispatch` | `DISPID_VALUE`, `UNKNOWN`, `PROPERTYPUT`, `NEWENUM`, `EVALUATE`, `CONSTRUCTOR`, `DESTRUCTOR`, `COLLECT`; Method / PropertyGet / PropertyPut / PropertyPutRef. |
| `DISPPARAMS` | `dispatch` | Positional args in source order, named args by `DispId` (= parameter index), assigned value as named `DISPID_PROPERTYPUT`. `bind` matches them to declared parameters (optional, defaults, ParamArray). |
| Case-insensitive names | `dispatch::names_equal` | Unicode lowercase comparison; property-tested. |
| ByRef | `dispatch`, `call` | `Arg.by_ref`; callee changes are written back only for ByRef arguments of ByRef parameters. ByVal parameters and by-value arguments never write back. Property-tested. |
| Errors | `error`, `hresult` | `ExcepInfo`, `DispError`, `ErrInfo` (`Err.Number/Description/Source/HelpFile/HelpContext`). `Err.Source` defaults to the class name. |
| HRESULT mapping | `hresult` | See table below; `0x800A00nn` -> `nn`; other numbers pass through; reverse mapping for VBA-implemented servers. |
| Object table | `table` | Handles never reused; AddRef/Release; `Set` (add-ref new, then release old); `Set x = Nothing`; `Is` identity; stale handle = error 91; RAII `Owned`. |
| Terminate ordering | `table` | At zero: `terminate()` runs while the object is addressable (may resurrect itself), then it is removed, then held references are released in declaration order. |
| Enumeration | `enumerate` | `ForEach` over `DISPID_NEWENUM` returning an object that exposes `VariantEnum`. Errors 91/438. |
| Events | `events` | `EventSource`/`EventSink`, `ConnectionPoint` (advise order delivery, ByRef event args such as `Cancel`), `WithEventsVar` (Set/Nothing advise and unadvise with correct reference counts). |
| Call resolution | `call` | See below. |
| Test object | `testing` | `MemObject`: configurable in-memory `Dispatch` for tests and doc examples only. |

### Call resolution (`call`)

* Name lookup via `get_ids_of_names`; failure is 438; unknown named argument is 448.
* Reads use `METHOD | PROPERTYGET`; `Let` uses `PROPERTYPUT` with the value let-coerced (object -> default member value, repeatedly; `Nothing` is 91); `Set` uses `PROPERTYPUTREF` and requires an object or `Nothing` (else 424).
* Default-member chaining: if `member_info` says the member takes fewer parameters than arguments given, the member is called with its own, the result must be an object (else 450) and the surplus goes to its default member, recursively. Covers `rs.Fields("x")`, `rs("x")`, `rs!x`, `Forms!Orders!Total`.
* `Let` onto a member that has no `Property Let` (per `member_info`) assigns to the default member of the object it returns (`rs!Name = v`); if the member returns no object, 451.
* `a!key` = default member with the string `key`; `[expr]` = `DISPID_EVALUATE`.
* `eval_chain` evaluates a multi-step chain; intermediate references are released; the final object result carries a reference owned by the caller.

### HRESULT to `Err.Number`

| HRESULT | Number |
| --- | --- |
| `DISP_E_MEMBERNOTFOUND`, `DISP_E_UNKNOWNNAME`, `DISP_E_NONAMEDARGS`, `DISP_E_BADCALLEE`, `DISP_E_NOTACOLLECTION` | 438 |
| `DISP_E_PARAMNOTFOUND` | 448 |
| `DISP_E_TYPEMISMATCH`, `TYPE_E_TYPEMISMATCH` | 13 |
| `DISP_E_BADVARTYPE` | 458 |
| `DISP_E_OVERFLOW` | 6 |
| `DISP_E_BADINDEX`, `TYPE_E_OUTOFBOUNDS` | 9 |
| `DISP_E_ARRAYISLOCKED` | 10 |
| `DISP_E_BADPARAMCOUNT` | 450 |
| `DISP_E_PARAMNOTOPTIONAL` | 449 |
| `DISP_E_DIVBYZERO` | 11 |
| `E_OUTOFMEMORY` / `E_INVALIDARG` / `E_ACCESSDENIED` / `E_NOTIMPL` | 7 / 5 / 70 / 445 |
| `E_NOINTERFACE`, `DISP_E_UNKNOWNINTERFACE` | 430 |
| `CO_E_CLASSSTRING`, `REGDB_E_CLASSNOTREG`, `CO_E_SERVER_EXEC_FAILURE` | 429 |
| `0x800A0000` OR `n`; any other value | `n`; unchanged |

## Not implemented / unverified

* The HRESULT table beyond the codes named in the task (438, 13, 448 etc.) follows commonly observed VBA behaviour and has not been checked against a real VBA host.
* No real COM: no `IUnknown`, `ITypeInfo`/type libraries, `IDispatchEx`, apartments or marshalling. `member_info` is the only type information.
* Without `member_info`, argument splitting to a default member, the `Let`-through-default rule and ByVal handling are not applied (all arguments are assumed ByRef).
* Positional arguments are source-ordered, not reversed as on the OLE wire.
* Locale (`lcid`) is not passed; `DISP_E_UNKNOWNLCID` is never raised.
* Reference cycles are not collected; arrays of objects are traversed by `add_ref_value`/`release_value` but not by Rust drops.
* `Property Set` versus `Let` fallback for Variant-typed properties is not attempted: a Let is a PropertyPut and a Set is a PropertyPutRef, exactly as the caller requested.
* Event delivery stops at the first handler error; WithEvents source discovery is one connection point per object.

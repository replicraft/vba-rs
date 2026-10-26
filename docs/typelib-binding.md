# Binding to type libraries

How the types of a type library (`docs/typelib.md`) become static types in
`vba-sema`, early-bound instructions in `vba-ir` and member-id calls in
`vba-vm`, what is supported, what is not, and which diagnostics exist.

## The flow

```text
vba-typelib  --TypelibProvider-->  vba-sema  --BLate.dispid-->  vba-ir  --Invoke.dispid-->  vba-vm
(MSFT model)   (TypeProvider)      (static types,                (Invoke)                    (by id, or by name)
                                    diagnostics)
vba-typelib::automation::InterfaceTable  -->  vba-automation::Dispatch implementers
```

1. **Provider.** `TypelibProvider` implements `vba_sema::TypeProvider` for
   one or more loaded libraries. Besides the older queries (`resolve_type`,
   `member`, `default_member`, `events`, `global`, ...) it answers
   `resolve_declared` (the type an `As` clause names), `enum_members`,
   `record`, `class_info` and `has_library`. A `LibMember` carries its
   `DISPID`; a `LibParam` carries the default the library declares for an
   omitted optional argument.
2. **Sema.** The analyzer turns what the provider says into types and
   checks. Everything that is not a library type behaves as before.
3. **IR.** `Invoke` carries `dispid: Option<i32>`. `Some` means the access was
   resolved against a library type: arguments are in parameter order,
   converted to the declared types, defaults filled in, none named.
4. **VM.** For an object outside the VM the VM calls
   `vba_automation::call::invoke_early` with the id. If the object does not
   accept the id for that member name the VM makes the late-bound call by
   name (the explicit fallback). `Vm::bind_stats()` counts early calls, calls
   by name and fallbacks.

Late binding is unchanged: `Object` and `Variant` receivers, and library
types the provider cannot describe, are called by name with the run-time
rules of `vba-automation` (default-member chaining, named arguments, error
438 for a missing member).

## Types

| Library type | Sema type | Notes |
| --- | --- | --- |
| interface, dispinterface, coclass | `Type::Library` | coclasses expose the members of their default interface |
| enum | `Type::LibEnum` | stored as `Long`; assignable to and from `Long` and other enums like a project `Enum`; members are constants of that type |
| record | `Type::LibRecord` | a value type like `Type ... End Type`: field access, copy on assignment, arrays of records |
| alias | the aliased type | resolved transitively: an alias of `Long` is `Long`, of an enum is that enum, of an interface is that interface |
| union, module | not declarable | `As` names them: `SEM004` |

Enums and records the program mentions are registered in the `Program`:
`lib_enums` (type and members by name, for tools that show enum members) and
`lib_records`. A record gets a synthetic `UdtId` numbered after the project's
own user-defined types (ids from `program.udts.len()` on); read them with
`Program::udt_name` and `Program::udt_fields`, which accept either kind.
`LibRecordInfo` also keeps each field's byte offset, the size and the
alignment from the library. `Program::udts` itself holds project types only.

Constants of library enums and modules are globals typed by their enum.
`Enum.Member` and `Library.Enum.Member` qualify them (`IOMode.ForReading`,
`Scripting.IOMode.ForReading`), `Library.Constant` qualifies a global; they
work in `Const` and `Select Case`.

## Calls

* Arguments are matched to the declared parameters once, at analysis time:
  positional, named, omitted, `ParamArray`. An omitted optional argument takes
  the library's declared default (`[defaultvalue]`), or is missing when there
  is none.
* Each argument is converted to the declared parameter type with VBA's
  conversions (`Convert` nodes; a `Variant` holding `"abc"` for a `Long`
  parameter is error 13 at run time, a constant that cannot convert is
  reported when the statement runs). `ByRef` parameters take variables by
  reference and write back; `[out]` parameters are `ByRef` parameters that
  must be supplied; the `[retval]` parameter is the result and is hidden.
* More arguments than a member declares go to the default member of its
  result when the result is an object or a `Variant`
  (`rs.Fields("Name")`, `rs("Name")`, `rs!Name`). Named arguments disable
  this.
* A scalar result is converted to the declared type at run time.
* Assignment converts the value to the property's declared type.
* `New Lib.Class` works for creatable coclasses (`cancreate`); `CreateObject`
  is unchanged.
* `WithEvents` on a library coclass takes its events from the `[source]`
  interface; a handler's parameters must match, `Long` and a library enum
  being interchangeable.

## Diagnostics

Codes added to `vba-sema` (existing ones keep their meaning):

| Code | Meaning |
| --- | --- |
| `SEM029` | assignment to a read-only library property |
| `SEM030` | read of a write-only library property (naming it as an assignment target is fine) |
| `SEM031` | `New` on a type that cannot be instantiated: an interface, or a coclass without `cancreate` |

Existing codes now also fire for library types: `SEM004` for a type the
library does not have (no longer for enums and aliases), `SEM005` (info) only
for types the provider cannot describe, `SEM006` for a value that cannot
convert to a declared type, `SEM008` for argument-list errors (count, name,
missing), `SEM019` for a member that does not exist, `SEM025` for assigning to
a method, `SEM026` for a `ByRef` argument whose type differs from the
parameter.

A mistake that early binding reports at compile time is a run-time error with
late binding:

| Mistake | Early binding | Late binding |
| --- | --- | --- |
| missing member | `SEM019` | 438 |
| too many arguments | `SEM008` | 450 |
| missing required argument | `SEM008` | 449 |
| unknown named argument | `SEM008` | 448 |
| assignment to a read-only property | `SEM029` | 451 (a `Let` with no `Property Let` is applied to the default member of the result) |
| value that does not convert | 13 at run time | 13 at run time |

## Dispatch tables for hosts

`vba_typelib::automation::InterfaceTable` (feature `automation`, on by
default) is built from a library type with
`TypelibProvider::interface_table(&LibraryType)`. It maps member names
(case-insensitive) to ids and describes each member for `vba-automation`:
supported invoke kinds, parameters (type, `ByRef`, optional, default,
`ParamArray`), result type. A `Dispatch` implementation delegates to it:

```rust
fn member_id(&self, n: &str) -> Option<DispId> { self.table.member_id(n) }
fn member_info(&self, id: DispId) -> Option<MemberInfo> { self.table.member_info(id) }
fn accepts_dispid(&self, id: DispId, n: &str) -> bool { self.table.accepts_dispid(id, n) }
fn get_ids_of_names(&self, n: &[&str]) -> Result<Vec<DispId>, DispError> { self.table.get_ids_of_names(n) }
fn invoke(&self, t: &ObjectTable, id: DispId, f: InvokeFlags, p: &mut DispParams) -> Result<Variant, DispError> {
    let mut b = self.table.bind_args(id, f, p)?;   // matched, defaulted, converted
    // ... run the member on b.slots / b.rest / b.value, update ByRef slots ...
    b.write_back(p);
    self.table.coerce_result(id, result)
}
```

`bind_args` rejects a put on a read-only member and a get on a write-only one
(member not found), reports argument-list errors as `bind` does, converts
every present argument and the assigned value to the declared type
(type mismatch 13; subscript out of range 9 for an array of the wrong rank).

An object that does not use a table needs nothing: `Dispatch::accepts_dispid`
defaults to comparing the name `member_info` reports for the id, so an object
whose ids differ from the library's is called by name, and an object with
the same id for a different member is never invoked by that id. The native
`Scripting` classes answer `accepts_dispid` without allocating.

## Not supported

* Unions and module (DLL) functions as declared types or callable members;
  module functions of a library are global functions called by name.
* Records are not marshalled to hosts: a host that receives a record gets an
  opaque `Variant::UserDefined`. Fixed-size array fields of a record lose
  their bounds.
* Assignment between different library object types is checked at run time
  only (`RuntimeCast`); there is no compile-time `implements` check, so
  `Set rs = New Scripting.Dictionary` for an `ADODB.Recordset` variable is not
  a compile error.
* `Set` versus `Let` on a property is not distinguished by the presence of a
  `propputref` accessor.
* Members whose names are not plain identifiers (`_NewEnum`, `[_Default]`)
  cannot be named in VBA source.
* `TypeOf x Is Interface` compares class names for objects outside the VM.
* WithEvents on an interface or dispinterface type reports that its events are
  not described; only coclasses have `[source]` interfaces.
* Result conversion applies to scalar results; object results are not
  checked against the declared interface at run time.

## Checks against real libraries

`crates/interoperability/vba-typelib/tests/wine_libraries.rs` holds
`#[ignore]`d tests that analyse VBA against the type libraries of a local
Wine installation (`scrrun`, `msxml3`, `msxml6`, `msado15`, `vbscript`); they
skip when the files are absent and nothing from the libraries is committed.
Run them with
`cargo test -p vba-typelib --test wine_libraries -- --ignored --nocapture`.

`how_many_members_bind_cleanly` declares a variable of every interface,
dispinterface and coclass of a library and names each member in a statement;
a member binds cleanly when it produces no `SEM004`, `SEM005`, `SEM019` or
syntax error. Result of the run that accompanied this document:

| Library | Types | Members | Bind cleanly |
| --- | ---: | ---: | ---: |
| Scripting (`scrrun`) | 21 | 237 | 237 |
| ADODB (`msado15`) | 24 | 411 | 411 |
| MSXML2 3.0 (`msxml3`) | 124 | 2248 | 2248 |
| MSXML2 6.0 (`msxml6`) | 85 | 1432 | 1432 |
| VBScript_RegExp_10 | 6 | 22 | 22 |
| VBScript_RegExp_55 | 11 | 41 | 41 |
| VBScript_Global | 2 | 199 | 199 |

36 members (names such as `_NewEnum`) and 9 types (names with a leading
underscore) are not valid VBA identifiers and are left out. The same tests
bind realistic early-bound programs (`Scripting.FileSystemObject`,
`TextStream`, `Dictionary`, `ADODB.Connection`/`Recordset`/`Command`,
`MSXML2.DOMDocument60`, `VBScript_RegExp_55.RegExp`) without any error, and
find the planted mistakes (`SEM008`, `SEM019`, `SEM029`, `SEM031`).

## Performance

`early_binding_is_not_slower_than_late_binding` (an `#[ignore]`d test in
`tests/early_binding.rs`; run it with `--release`) times a loop of one
million host calls with a typed receiver and with an `Object` receiver and
asserts the typed one is not more than 10% slower. Early binding skips the
name lookup (`GetIDsOfNames`) and the `MakeRef` of each variable argument.
Measured on a development machine (best of five, so timings vary by a few
percent between runs, other processes were running), time of early binding
divided by time of late binding:

| One million calls of | early | late | ratio |
| --- | ---: | ---: | ---: |
| `n = d.Count` (Dictionary property) | 0.62 s | 0.86 s | 0.71 to 0.78 |
| `n = d.Exists("a")` (Dictionary method) | 2.2 s | 2.4 s | 0.93 to 1.00 |
| `n = d.Add(i, 1)` (table-driven host object) | 2.1 s | 2.3 s | 0.92 to 0.97 |

Most of the time of the method calls is in the host object and in the
argument handling of `vba-automation`, which both paths share.

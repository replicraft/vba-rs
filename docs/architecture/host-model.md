# Host model: describe an object model once

Crate: `crates/interoperability/vba-host-model`. Facade: `vba::host_model`,
`Engine::host_library`. Language server: the `vba.hostLibraries` setting.

An embedding application (a database application with an Access-like object
model, say) has to tell vba-rs about its objects in four places: how a call
binds at run time (`Dispatch`), what the analysis may assume about types
(`TypeProvider`), what an editor lists (completion, hover, signature help)
and what the documentation says. Written separately these drift. The host
model is one description, a `HostLibrary`, from which all four are derived.

```text
HostLibrary  --check()-->  CheckedLibrary  ---+--> impl TypeProvider        (sema, language server)
 (plain data,               (validated;       |--> HostObject: Dispatch     (run time, built from the same signatures)
  builders, JSON)            signatures        |--> HostFunctions            (global functions: Nz, DLookup)
                             derived once)     +--> documentation, catalog   (hover, completion)
```

## Why a new crate

`vba-automation` is the COM-free model of `IDispatch` and depends only on
`vba-core` and `vba-value`; it must not learn about `vba-sema`.
`vba-typelib` reads Windows type libraries and is the other source of
`TypeProvider`s. The description is a third source that needs both the
automation traits (to bind calls) and the sema trait (to type them), with no
Windows format involved, so it gets its own small crate in the same group.
Dependencies: `vba-automation`, `vba-sema`, `vba-value`, `thiserror`, and
`serde_json` (already a workspace dependency of the language server) for the
JSON form, used through `serde_json::Value` without derive macros so the
format is written out by hand and stays under our control.

## The description

All types are plain data with builders. Nothing is checked while building.

| Type | Holds |
| --- | --- |
| `HostLibrary` | name, doc, `classes`, `enums`, `consts`, `functions`, `globals` |
| `HostClass` | name, doc, `members`, `events`, `creatable`, `enumerable` (the `For Each` item type, which implies a `_NewEnum` member), attributes |
| `HostMember` | name, `kind` (`Method` or `Property`), `ty` (result or property type; none for a `Sub`), `params`, `access` (`ReadOnly`, `ReadWrite`, `WriteOnly`), `default_member`, `hidden`, optional `dispid`, doc, attributes |
| `HostParam` | name, `ty`, `optional`, `default` (`ConstValue`), `by_ref`, `param_array`, doc |
| `HostEvent` | name, params (a `ByRef` parameter such as `Cancel` is written back to the raiser), doc |
| `HostEnum` | name, doc, members (name, value, doc); an enumeration is a type (`As RecordsetType`) and its members are global constants |
| `HostConst` | name, `ConstValue` (`Empty`, `Null`, `Boolean`, `Integer`, `Long`, `Double`, `String`), doc |
| `HostGlobal` | a predeclared object (`Application`): name, class, doc |
| `TypeRef` | an intrinsic type, `Named` (a class or enumeration of the library) or `Array`; `TypeRef::parse("Long()")` reads the VBA spelling, and `&str` converts into it in the builders |

Properties have one entry for `Get`, `Let` and `Set`. `Access::ReadWrite`
means "the assignment that fits the type": `Let` for a value type, `Set` for a
class, both for `Variant`. Parameters default to `ByVal` (a deliberate
difference from a VBA declaration, because host members rarely write back);
`.by_ref()` opts in.

```rust
use vba::host_model::{HostClass, HostLibrary, HostMember, HostParam};

let lib = HostLibrary::new("Shop")
    .class(
        HostClass::new("Orders")
            .doc("The open orders.")
            .member(HostMember::property("Count", "Long").read_only())
            .member(
                HostMember::function("Total", "Currency")
                    .param(HostParam::new("Customer", "String"))
                    .param(HostParam::new("Limit", "Long").default_value(10)),
            ),
    )
    .check()
    .expect("valid");
assert_eq!(lib.class_member("Orders", "Total").unwrap().params.len(), 2);
```

## Validation

`HostLibrary::check()` returns a `CheckedLibrary` or `ModelErrors`: the
complete list of `ModelError`s, not only the first. `problems()` reports
without consuming. The rules:

| Rule | Error |
| --- | --- |
| Names are non-empty VBA identifiers (letters, digits, `_`, starting with a letter, at most 255 characters; non-ASCII letters allowed) | `EmptyName`, `InvalidName` |
| A class or enumeration is not named like an intrinsic type | `ReservedName` |
| Names are unique ignoring case within their scope: types (classes and enumerations together), global names (constants, enumeration members, functions, predeclared objects together), members of a class, events of a class, members of an enumeration, parameters of a member. A class and a predeclared object may share a name (`Application`) | `Duplicate` |
| Every `Named` type is a class or enumeration of the library, wherever it is used (results, parameters, events, `enumerable`, globals) | `DanglingType` |
| `ParamArray` is last and is an optional `ByRef Variant()` without a default | `ParamArrayNotLast`, `ParamArrayShape` |
| No required parameter follows an optional one | `RequiredAfterOptional` |
| A default needs `Optional` and must fit the type | `DefaultNotOptional`, `DefaultTypeMismatch`, `NonFiniteNumber` |
| A property has a type; a global function is a method whose parameters are `ByVal` | `PropertyWithoutType`, `FunctionNotMethod`, `ByRefFunctionParam` |
| One default member per class, and it is not a `Sub` | `MultipleDefaultMembers`, `DefaultMemberIsSub` |
| `DISPID`s are unique per class; 0 belongs to the default member and -4 (`_NewEnum`) is reserved | `DuplicateDispId`, `ReservedDispId` |

Case-insensitivity uses Unicode lower-casing, the same comparison as
`vba_automation::names_equal`. Property tests (`tests/properties.rs`) generate
libraries and check that valid ones round-trip through JSON, that any
case-variant duplicate and any dangling type is reported, and that arbitrary
text never panics the checker or the reader.

## What `check()` derives

For every member the checker computes a `MemberSig` once: the `DISPID`
(explicit, 0 for the default member, otherwise the next unused number from 1
in declaration order), the invoke kinds it answers (`supports`), the typed
parameters (`ParamSig`, with the `Type` sema uses and the `vba-automation`
`ParamInfo` binding uses), the result type and flags. `MemberSig::to_lib_member`
and `MemberSig::to_member_info` are the only two conversions, so the sema view
and the automation view of a member come from one structure.

| Member | Invoke kinds supported |
| --- | --- |
| method | `METHOD`, `PROPERTY_GET` |
| read-only property | `METHOD`, `PROPERTY_GET` |
| read/write property of a value type | those plus `PROPERTY_PUT` |
| read/write property of a class or `Object` | those plus `PROPERTY_PUT_REF` |
| read/write property of `Variant` | those plus `PROPERTY_PUT` and `PROPERTY_PUT_REF` |
| write-only property | the puts only |

## Adapter 1: `TypeProvider`

`CheckedLibrary` implements `vba_sema::TypeProvider`; it is `Send + Sync`
(an `Arc` inside), so the same value serves the analyzer, the engine and a
language server.

| Provider method | Answer |
| --- | --- |
| `resolve_type`, `resolve_declared` | classes and enumerations, optionally qualified (`DbLib.Recordset`); enumerations resolve to `Type::LibEnum` |
| `has_library` | the library name |
| `enum_members` | the enumeration members and values |
| `member`, `default_member` | the member signature as a `LibMember`, with its `DISPID`, so calls on a typed receiver are early bound |
| `describes_members` | true for every class and enumeration (an unknown member is `SEM019`, not a late-bound guess) |
| `interface_members` | the non-hidden members in declaration order (what a language server lists) |
| `events` | the described events, with `ByRef` flags (checks `WithEvents` handlers) |
| `class_info` | `creatable` |
| `global` | constants and enumeration members (`Constant`), global functions (`Function`), predeclared objects (`Object`) |
| `catalog` | every top-level name with its kind (new in this change) |
| `documentation` | the doc text of a type, a member or a global name (new in this change) |

`catalog` and `documentation` are two additions to the `TypeProvider` trait
with "nothing" defaults, so existing providers are unaffected. `Layers<T>`
(also new) consults any number of providers in order.

Two behaviors of the analysis that the description inherits, not changes:

* A class used where another class is required is accepted by the analysis
  (`RuntimeCast`) because `QueryInterface` could make it work; the run-time
  adapter below rejects the wrong class with error 13. Passing a scalar where
  an object is required is `SEM006` at analysis time.
* More arguments than a member declares go to the default member of its
  result when that is an object, as in `rs.Fields("a")`. For a result without
  a default member nothing reports the surplus.

This change also lets sema assign through the default member of a library
type (`rs.Fields("a") = 5` for a `Field` whose default member is `Value`),
when the default member needs no arguments; before, only project classes
supported that.

## Adapter 2: `Dispatch`

`HostObject::new(&library, "Class", implementation)` is a `Dispatch` object
whose metadata is the description:

* `member_id`, `member_info`, `accepts_dispid` and the inherited
  `get_ids_of_names` (named arguments resolve to parameter indices) answer
  from the signatures. `_NewEnum` exists exactly for enumerable classes.
* `invoke` is one function for late-bound calls (by name) and early-bound calls
  (by `DISPID`): it checks the invoke kind (`DISP_E_MEMBERNOTFOUND` otherwise),
  binds with `vba_automation::bind` against the described parameters
  (count and omitted-argument errors, optional defaults, `ParamArray`),
  converts every argument to its declared type with VBA's assignment
  conversions (error 13 when it does not fit; an object argument of a class type
  must be an object of that class or `Nothing`), runs the implementation,
  writes `ByRef` results back, and converts the result to the declared result type.
* Write-back is exact: only parameters declared `ByRef` that the caller
  passed a variable for, and that the implementation assigned (`Args::set`),
  reach the caller. An untouched `ByRef` variable keeps its own value, not the
  converted copy.
* A described event is raised with `Invocation::raise_event` or
  `HostObject::raise_event`; sinks connect through the object's
  `event_source()`.

The implementation is a `HostImpl`:

* a closure `Fn(&mut Invocation) -> Result<Variant, DispError>` that matches
  on `cx.name()` and `cx.kind` (`Call`, `Get`, `Let`, `Set`, `NewEnum`);
* your own type implementing `HostImpl` (also `terminate`, `held_references`,
  `reentrant`);
* `Handlers<S>`: a state value plus one closure per member and kind
  (`.method`, `.get`, `.let_`, `.set`, `.enumerate`). `HostObject::new`
  checks the table against the description, so drift is a `BindError` at
  construction: `Unimplemented` (a described member has no handler),
  `UnknownMember`, `KindNotSupported` (a `Let` of a read-only property),
  `UnknownClass`. For a closure or a hand-written `HostImpl` the check cannot
  be made, but binding, conversion and the invoke-kind check still apply.

Objects a member returns carry one reference for the caller, as everywhere in
`vba-automation`: `cx.table.insert_obj(obj)` returns a handle that does.

## Adapter 3: global functions

`Nz` and `DLookup` are not members of an object. The analysis binds a
`LibGlobal::Function` as a call of a named run-time function, and the VM has
asked the services for names it does not implement itself. This change adds a
host hook for those names: `Host::call_function(name, args)` with a default of
"not mine" (`Ok(None)`), consulted by the VM after the standard library and
before the services; step accounting is untouched because only calls that used
to fail reach it. `HostFunctions::function(name, closure)` implements the
described functions with the same binding and conversion as members; arguments
are by value (so the checker rejects `ByRef` parameters on functions, and
assigning to an argument cannot reach the caller).

## Adapter 4: the facade

`HostRuntime::new(&library)` collects the run-time half: `.global(name, make)`
for each predeclared object, `.creatable(class, make)` for each creatable
class, `.function(name, closure)` for each global function. `.build()` returns
all problems at once (a described global or creatable class without a factory,
an unimplemented function, a name the description does not have).

`Engine::host_library(runtime)` registers it:

* predeclared objects become engine globals (built on first use, shared per
  session);
* creatable classes are registered under `Library.Class` for
  `CreateObject` and `New` (which still need `Capability::ComCreate`);
* global functions answer through `Host::call_function`;
* the description is the type provider of `Engine::analyze` and `Engine::load`,
  layered before the engine's own global names.

The library grants no authority by itself.

## Adapter 5: the language server

`vba-lsp-core` already took a `TypeProvider` (`Workspace::set_type_provider`);
a `CheckedLibrary` can be passed as it is. What the description adds to the
editor features:

| Feature | Data from the description |
| --- | --- |
| Member completion | non-hidden members, signature in `detail`, doc text in the new `CompletionItem::documentation` |
| Top-level completion | global functions, constants, enumeration members and predeclared objects in expressions; classes and enumerations after `As`/`New`/`Implements`; the library's names after `Library.` |
| Hover | member signature and doc; the same for a global function, constant or object; classes and enumerations in type positions |
| Signature help | members (with doc) and global functions, active parameter by position or name |
| Diagnostics | the `SEM###` codes of the analysis |
| Types of globals | `Application.OpenRecordset("t").` completes through the return types |

`vba-lsp` reads `vba.hostLibraries`: a list whose entries are JSON file paths
(the format below) or inline description objects. Each is parsed, validated
and installed ahead of the type libraries (`vba.typeLibraries`) as one
provider; invalid ones are reported with `window/logMessage` (for example
``host library `db.json`: unknown type `Nowhere` ``) and skipped. Files are
read through the same 64 MiB cap as type libraries. Changing the setting
reloads. An embedding application generates the file with
`HostLibrary::to_json_string()`, or passes the JSON object in the
`initializationOptions` when it starts the server itself.

## JSON format, version 1

`HostLibrary::to_json` / `to_json_string` write and `from_json` /
`from_json_str` read the form below. The reader is strict: unknown keys,
wrong types and other versions are errors that name the path
(`$.classes[1].members[0].params[2].type`). Reading does not validate; call
`check()`. The writer emits object keys in alphabetical order (the order of
`serde_json::Map`), omits `doc`, flags that are false and empty lists inside
classes and members, and keeps list order (declaration order matters for
automatic `DISPID`s and for completion).

```json
{
  "format": "vba-host-library",
  "version": 1,
  "name": "Shop",
  "doc": "optional",
  "classes": [{
    "name": "Orders", "doc": "optional", "creatable": true, "enumerable": "Order",
    "attributes": [{"name": "VB_Description", "value": "text"}],
    "members": [{
      "name": "Total", "kind": "method", "type": "Currency",
      "params": [{
        "name": "Limit", "type": "Long", "optional": true,
        "default": {"type": "long", "value": 10},
        "byRef": false, "paramArray": false, "doc": "optional"
      }],
      "default": false, "hidden": false, "dispid": null, "doc": "optional",
      "attributes": []
    }, {
      "name": "Count", "kind": "property", "access": "read", "type": "Long"
    }],
    "events": [{"name": "Changed", "params": [{"name": "Cancel", "type": "Integer", "byRef": true}]}]
  }],
  "enums": [{"name": "Ordering", "members": [{"name": "shopByName", "value": 1, "doc": "optional"}]}],
  "constants": [{"name": "AppVersion", "value": {"type": "string", "value": "1.0"}}],
  "functions": [{"name": "Nz", "kind": "method", "type": "Variant", "params": []}],
  "globals": [{"name": "Orders", "type": "Orders"}]
}
```

* `kind`: `method` (no `type` means `Sub`) or `property` (`type` required,
  `access`: `read`, `readwrite` (default) or `write`).
* Types are spelled as in VBA: `Long`, `String`, `Variant`, `Object`,
  `Long()`, or a class or enumeration name of the library. Intrinsic names
  are case-insensitive and written canonically.
* Values are `{"type": T, "value": V}` with `T` one of `empty`, `null`
  (no `value`), `boolean`, `integer` (16 bit), `long` (32 bit), `double`,
  `string`.
* `attributes` are free-form name/value pairs carried for tools (a help id, a
  description); they never change behavior.

## Tests

| What | Where |
| --- | --- |
| Description to `Dispatch`: metadata, ids, named and omitted arguments, conversion, wrong class, `ByRef`, `ParamArray`, early versus late, default-member chains, `For Each`, events, implementation checks, global functions | `vba-host-model/tests/dispatch.rs` |
| Description to analysis: clean program, argument count, type mismatch, unknown member and type, wrong `ByRef` type, read-only, provider answers, documentation, hidden members | `vba-host-model/tests/sema.rs` |
| Validation: every rule above | `vba-host-model/tests/validate.rs` |
| JSON: round trip, exact form, strict reader | `vba-host-model/tests/json.rs` |
| Properties: JSON round trip, case-variant duplicates, dangling types, `TypeProvider` and `Dispatch` agree on every member, garbage never panics | `vba-host-model/tests/properties.rs` |
| Facade: analyze and run one description end to end, errors in `Err`, `New`, `For Each`, events with `Cancel` | `vba-rs/tests/host_library.rs` |
| Language server core: completion, hover, signature help, diagnostics | `vba-lsp-core/tests/host_library.rs` |
| `vba-lsp` over stdio: file and inline descriptions, invalid ones, reload | `vba-lsp/tests/host_libraries.rs` |
| Worked example | `cargo run -p vba-rs --example host_library` |

## Not covered

* User-defined types (records) and aliases; interfaces to `Implements`
  (a described class is listed as an interface by `interface_members`, so
  `Implements Recordset` would require all of its members).
* Inheritance between described classes: a parameter of class `A` accepts
  only an `A` at run time.
* Global functions cannot write back `ByRef` arguments (they are passed by
  value); named arguments work because the analysis puts them in parameter
  order before the call.
* Collections with `Add`/`Remove` semantics are ordinary classes with an
  `enumerable` item type; `For Each` element types are not used by the
  analysis (the control variable is typed by its own `Dim`).
* Localised names, version gating of members, `Friend` members.
* The description does not enforce that an implementation honours the
  described types of values it stores; results and arguments are converted at
  the call boundary only.
* Importing a description from a type library (`vba-typelib`) or exporting one
  to a type library; the JSON form is the only interchange.

//! Global functions and the run-time half of a library: what an embedding
//! application registers next to the description.

use std::collections::HashMap;
use std::rc::Rc;

use vba_automation::{Arg, DispError, DispParams, InvokeFlags, ObjectTable};
use vba_value::{ObjectRef, Variant};

use crate::check::{CheckedLibrary, key};
use crate::model::TypeRef;
use crate::runtime::{BindError, HostImpl, HostObject, Invocation, run_member};

type Func = Rc<dyn Fn(&mut Invocation<'_>) -> Result<Variant, DispError>>;

/// Creates a value (a predeclared object or a new instance) in a table;
/// the value carries one reference for the receiver.
pub type ValueFactory = Rc<dyn Fn(&ObjectTable) -> Result<Variant, DispError>>;

struct FuncImpl(Func);

impl HostImpl for FuncImpl {
    fn invoke(&self, cx: &mut Invocation<'_>) -> Result<Variant, DispError> {
        (self.0)(cx)
    }
}

/// The implementations of a library's global functions (`Nz`, `DLookup`).
///
/// A call is bound exactly like a member call: omitted optional arguments
/// take their described default, every argument is converted to its
/// declared type and the result is converted to the declared result type.
/// Arguments are passed by value; assigning to one does not reach the
/// caller.
#[derive(Clone)]
pub struct HostFunctions {
    lib: CheckedLibrary,
    imps: HashMap<String, Rc<FuncImpl>>,
}

impl HostFunctions {
    /// No implementations yet.
    pub fn new(lib: &CheckedLibrary) -> HostFunctions {
        HostFunctions {
            lib: lib.clone(),
            imps: HashMap::new(),
        }
    }

    /// Implements the described function `name`.
    pub fn function(
        mut self,
        name: &str,
        f: impl Fn(&mut Invocation<'_>) -> Result<Variant, DispError> + 'static,
    ) -> Result<HostFunctions, BindError> {
        let sig = self
            .lib
            .function(name)
            .ok_or_else(|| BindError::UnknownFunction(name.to_owned()))?;
        self.imps
            .insert(key(&sig.name), Rc::new(FuncImpl(Rc::new(f))));
        Ok(self)
    }

    /// The described functions that have no implementation.
    pub fn missing(&self) -> Vec<String> {
        self.lib
            .functions()
            .iter()
            .filter(|s| !self.imps.contains_key(&key(&s.name)))
            .map(|s| s.name.clone())
            .collect()
    }

    /// Calls the function `name` with one entry per declared parameter
    /// (`None` for an omitted one), as the interpreter passes them.
    /// `Ok(None)` means the library has no such function.
    pub fn call(
        &self,
        table: &ObjectTable,
        name: &str,
        args: &[Option<Variant>],
    ) -> Result<Option<Variant>, DispError> {
        let Some(sig) = self.lib.function(name) else {
            return Ok(None);
        };
        let Some(imp) = self.imps.get(&key(&sig.name)) else {
            return Ok(None);
        };
        let mut params = DispParams::new();
        params.args = args
            .iter()
            .map(|a| a.clone().map_or_else(Arg::missing, Arg::val))
            .collect();
        run_member(
            sig,
            "",
            &**imp,
            None,
            table,
            InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            &mut params,
        )
        .map(Some)
    }
}

/// Builds the object a factory produces.
pub(crate) type ObjectFactory = Rc<dyn Fn(&ObjectTable) -> Result<HostObject, DispError>>;

/// A described library together with everything needed to run it:
/// factories for the predeclared objects and creatable classes, and the
/// global functions. Register it with the facade (`Engine::host_library`)
/// or use the parts directly.
#[derive(Clone)]
pub struct HostRuntime {
    lib: CheckedLibrary,
    globals: Vec<(String, ObjectFactory)>,
    classes: Vec<(String, ObjectFactory)>,
    functions: HostFunctions,
    errors: Vec<BindError>,
}

impl HostRuntime {
    /// A runtime without factories or functions.
    pub fn new(lib: &CheckedLibrary) -> HostRuntime {
        HostRuntime {
            lib: lib.clone(),
            globals: Vec::new(),
            classes: Vec::new(),
            functions: HostFunctions::new(lib),
            errors: Vec::new(),
        }
    }

    /// Provides the predeclared object `name`; built on first use and
    /// shared afterwards within a session.
    #[must_use]
    pub fn global(
        mut self,
        name: &str,
        make: impl Fn(&ObjectTable) -> Result<HostObject, DispError> + 'static,
    ) -> HostRuntime {
        match self
            .lib
            .library()
            .globals
            .iter()
            .find(|g| key(&g.name) == key(name))
        {
            Some(g) if matches!(g.ty, TypeRef::Named(_) | TypeRef::Object) => {
                self.globals.push((g.name.clone(), Rc::new(make)));
            }
            _ => self.errors.push(BindError::UnknownGlobal(name.to_owned())),
        }
        self
    }

    /// Provides instances of the creatable class `class`
    /// (`CreateObject("Library.Class")`, `New Class`).
    #[must_use]
    pub fn creatable(
        mut self,
        class: &str,
        make: impl Fn(&ObjectTable) -> Result<HostObject, DispError> + 'static,
    ) -> HostRuntime {
        match self.lib.class(class) {
            Some(c) => self.classes.push((c.name.clone(), Rc::new(make))),
            None => self.errors.push(BindError::UnknownClass(class.to_owned())),
        }
        self
    }

    /// Implements the described global function `name`.
    #[must_use]
    pub fn function(
        mut self,
        name: &str,
        f: impl Fn(&mut Invocation<'_>) -> Result<Variant, DispError> + 'static,
    ) -> HostRuntime {
        match self.functions.clone().function(name, f) {
            Ok(fs) => self.functions = fs,
            Err(e) => self.errors.push(e),
        }
        self
    }

    /// Checks that everything described can run: every predeclared object
    /// has a factory, every creatable class has a factory, every global
    /// function an implementation, and nothing was registered that the
    /// description does not know. Returns all problems.
    pub fn build(self) -> Result<HostRuntime, Vec<BindError>> {
        let mut errors = self.errors.clone();
        let lib = self.lib.library();
        for g in &lib.globals {
            if !self.globals.iter().any(|(n, _)| key(n) == key(&g.name)) {
                errors.push(BindError::NoFactory(g.name.clone()));
            }
        }
        for c in lib.classes.iter().filter(|c| c.creatable) {
            if !self.classes.iter().any(|(n, _)| key(n) == key(&c.name)) {
                errors.push(BindError::NoFactory(c.name.clone()));
            }
        }
        for f in self.functions.missing() {
            errors.push(BindError::Unimplemented {
                class: String::new(),
                member: f,
                kind: crate::runtime::CallKind::Call,
            });
        }
        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }

    /// The description.
    pub fn library(&self) -> &CheckedLibrary {
        &self.lib
    }

    /// The global functions.
    pub fn functions(&self) -> &HostFunctions {
        &self.functions
    }

    /// The predeclared objects: name and a function that creates the
    /// object in a table, returning a value that carries one reference.
    pub fn globals(&self) -> Vec<(String, ValueFactory)> {
        self.globals
            .iter()
            .map(|(n, f)| (n.clone(), insert_with(f.clone())))
            .collect()
    }

    /// The creatable classes, as [`globals`](HostRuntime::globals) does.
    pub fn classes(&self) -> Vec<(String, ValueFactory)> {
        self.classes
            .iter()
            .map(|(n, f)| (n.clone(), insert_with(f.clone())))
            .collect()
    }
}

fn insert_with(f: ObjectFactory) -> ValueFactory {
    Rc::new(move |table| {
        let o = f(table)?;
        Ok(Variant::Object(ObjectRef::Handle(table.insert_obj(o))))
    })
}

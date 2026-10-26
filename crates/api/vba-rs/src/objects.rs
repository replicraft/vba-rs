//! Instances of class, document and designer modules, and events raised on
//! one of them.

use std::rc::Rc;

use vba_automation::Dispatch;
use vba_value::{ObjectRef, Variant};
use vba_vm::RunStatus;

use crate::call::{Arg, CallResult, CallTarget, split_args};
use crate::{Error, Instance};

/// What raising an event on one instance produced.
///
/// ```
/// use vba::EventOutcome;
/// fn cancelled(o: &EventOutcome) -> bool {
///     o.byref.first().is_some_and(|v| *v != vba::Variant::Integer(0))
/// }
/// # let _ = cancelled;
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EventOutcome {
    /// Whether the instance has a handler for the event. When it has none,
    /// nothing ran and [`byref`](EventOutcome::byref) repeats the values
    /// that were passed in.
    pub handled: bool,
    /// How the handler finished (`Completed` when there is none).
    pub status: RunStatus,
    /// The final values of the [`Arg::ByRef`] arguments, in argument order
    /// (for example `Cancel`).
    pub byref: Vec<Variant>,
    /// The `Debug.Print` text written by the handler.
    pub output: String,
}

/// The values of the `ByRef` arguments, in argument order.
pub(crate) fn by_ref_values(
    positional: &[vba_vm::Arg],
    named: &[(String, vba_vm::Arg)],
) -> Vec<Variant> {
    positional
        .iter()
        .chain(named.iter().map(|(_, a)| a))
        .filter_map(|a| match a {
            vba_vm::Arg::ByRef(v) => Some(v.clone()),
            _ => None,
        })
        .collect()
}

impl Instance {
    /// Creates a **new** instance of a class, document or designer module.
    /// Every instance has its own module-level state and gets its own
    /// `Class_Initialize`.
    ///
    /// `host_me` is bound as the instance's *host base*. Members the
    /// module does not define dispatch to it, late bound: unqualified
    /// control and property names (`Dirty`, `Caption`, `CustomerName`),
    /// `Me.Name`, `Me!Name`, `Me.Controls("Name")` and the default member
    /// (`Me("Name")`). Anything the module defines itself wins.
    ///
    /// The returned reference is owned by the caller; give it back with
    /// [`destroy_object`](Instance::destroy_object), which runs
    /// `Class_Terminate` deterministically when nothing else holds the
    /// instance.
    pub fn create_object(
        &mut self,
        module: &str,
        host_me: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        self.session
            .create_object(module, host_me)
            .map_err(Error::Run)
    }

    /// The default (predeclared) instance of a document, designer or
    /// `VB_PredeclaredId` class module (`Form_Orders`, `Forms!Orders`
    /// style), created on first use. The reference is borrowed and lives as
    /// long as the instance; it cannot be destroyed.
    ///
    /// A `host_me` is bound before `Class_Initialize` when this call creates
    /// the instance, and bound afterwards when the instance exists without
    /// one; an instance that already has a host base is VBA error 457.
    pub fn default_instance(
        &mut self,
        module: &str,
        host_me: Option<Rc<dyn Dispatch>>,
    ) -> Result<ObjectRef, Error> {
        self.session
            .default_instance(module, host_me)
            .map_err(Error::Run)
    }

    /// Gives back the reference [`create_object`](Instance::create_object)
    /// returned. When it was the last one, `Class_Terminate` runs before
    /// this returns; if VBA code stored the instance somewhere it lives on
    /// until that reference is released. Destroying a stale or foreign
    /// object is VBA error 91, a default instance error 5.
    pub fn destroy_object(&mut self, object: ObjectRef) -> Result<(), Error> {
        self.session.destroy_object(&object).map_err(Error::Run)
    }

    /// Calls a public `Sub`, `Function` or `Property Get` of one instance.
    /// Arguments and results behave as in [`Instance::call`].
    pub fn call_on(
        &mut self,
        object: ObjectRef,
        procedure: &str,
        args: Vec<Arg>,
    ) -> Result<CallResult, Error> {
        self.call(CallTarget::Object(object), procedure, args)
    }

    /// Reads a public variable or a parameterless `Property Get` of one
    /// instance by name; a name the module does not define goes to its host
    /// base. An object value carries a reference the caller owns (see
    /// [`release`](Instance::release)).
    pub fn get_member(&mut self, object: ObjectRef, name: &str) -> Result<Variant, Error> {
        self.session.get_member(&object, name).map_err(Error::Run)
    }

    /// Assigns a public variable or `Property Let`/`Set` of one instance by
    /// name (`Set` when the value is an object); a name the module does not
    /// define goes to its host base.
    pub fn set_member(
        &mut self,
        object: ObjectRef,
        name: &str,
        value: Variant,
    ) -> Result<(), Error> {
        self.session
            .set_member(&object, name, value)
            .map_err(Error::Run)
    }

    /// Whether the instance defines a `Sub`, `Function` or `Property Get`
    /// called `name`, public or private: how a host discovers the event
    /// handlers (`Form_Current`, `Command1_Click`) of a form.
    pub fn has_procedure(&self, object: ObjectRef, name: &str) -> bool {
        self.session.has_procedure(&object, name)
    }

    /// Raises an event on **one instance**: runs the handler named
    /// `<source>_<event>` (`Form` + `BeforeUpdate` is `Form_BeforeUpdate`,
    /// `Command1` + `Click` is `Command1_Click`) whatever its visibility.
    /// There is no error when the instance has no such handler;
    /// [`EventOutcome::handled`] says whether one ran.
    ///
    /// `ByRef` arguments are written back to [`EventOutcome::byref`], so
    /// the host can read what the handler left in `Cancel`:
    ///
    /// ```
    /// use vba::{Arg, Engine, Project, Variant, ModuleKind};
    ///
    /// let mut project = Project::new();
    /// project.add_module("Form_Orders", ModuleKind::Document,
    ///     "Private Sub Form_BeforeUpdate(Cancel As Integer)\r\n  Cancel = True\r\nEnd Sub\r\n").unwrap();
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// let form = inst.create_object("Form_Orders", None).unwrap();
    /// let out = inst
    ///     .raise_event_on(form, "Form", "BeforeUpdate", vec![Arg::by_ref(Variant::Integer(0))])
    ///     .unwrap();
    /// assert!(out.handled);
    /// assert_eq!(out.byref, vec![Variant::Integer(-1)]);
    /// ```
    pub fn raise_event_on(
        &mut self,
        object: ObjectRef,
        source: &str,
        event: &str,
        args: Vec<Arg>,
    ) -> Result<EventOutcome, Error> {
        let before = self.sink.contents().len();
        let (positional, named) = split_args(args).map_err(|why| {
            Error::Run(vba_runtime::Error::NotCallable(
                format!("{source}_{event}"),
                why,
            ))
        })?;
        let originals = by_ref_values(&positional, &named);
        let r = self
            .session
            .raise_event_on(&object, source, event, positional, named);
        let output = self.sink.contents()[before..].to_owned();
        Ok(match r.map_err(Error::Run)? {
            Some(out) => EventOutcome {
                handled: true,
                status: out.status,
                byref: out.byref,
                output,
            },
            None => EventOutcome {
                handled: false,
                status: RunStatus::Completed,
                byref: originals,
                output,
            },
        })
    }

    /// Delivers events that host objects raised **outside** a call of
    /// theirs (for example from the embedder's own code between calls) to
    /// the `WithEvents` handlers, in the order they were raised. An event
    /// raised while the session waits in a call of the raising object is
    /// delivered at once, with `ByRef` write-back; see
    /// [`HostContext`](crate::HostContext) and the embedding guide.
    pub fn deliver_pending_events(&mut self) -> Result<(), Error> {
        self.session.deliver_pending_events().map_err(Error::Run)
    }

    /// Releases the object references inside a value a call returned: an
    /// object result, or an object left in a `ByRef` slot, carries one
    /// reference the caller owns.
    pub fn release(&mut self, value: &Variant) {
        self.session.release(value);
    }
}

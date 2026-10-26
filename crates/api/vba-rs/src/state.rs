//! Module-level state and project metadata.

use vba_runtime::ModuleSummary;
use vba_value::Variant;

use crate::{Error, Instance};

impl Instance {
    /// Reads a public module-level variable or constant of a standard
    /// module: `Module.Name`, or just `Name` when exactly one standard
    /// module declares it.
    ///
    /// ```
    /// use vba::{Engine, Project, Variant};
    ///
    /// let project = Project::from_source(
    ///     "Config",
    ///     "Public Const Limit As Long = 10\nPublic Counter As Integer\nPublic Sub Bump()\n  Counter = Counter + 1\nEnd Sub\n",
    /// );
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// inst.run("Bump", vec![]).unwrap();
    /// assert_eq!(inst.get_global("Counter").unwrap(), Variant::Integer(1));
    /// assert_eq!(inst.get_global("Config.Limit").unwrap(), Variant::Long(10));
    /// // Assignment converts to the declared type like VBA does.
    /// inst.set_global("Counter", Variant::Long(41)).unwrap();
    /// assert_eq!(inst.get_global("Counter").unwrap(), Variant::Integer(41));
    /// assert!(inst.set_global("Counter", Variant::Long(70_000)).is_err()); // overflow
    /// ```
    ///
    /// An object value carries a reference the caller owns (see
    /// [`release`](Instance::release)). Errors: [`NoSuchMember`],
    /// [`AmbiguousMember`] and [`NotAccessible`] (private, a user-defined
    /// type, or a variable of a class-like module, which is a field of each
    /// instance: use [`get_member`](Instance::get_member)).
    ///
    /// [`NoSuchMember`]: vba_runtime::Error::NoSuchMember
    /// [`AmbiguousMember`]: vba_runtime::Error::AmbiguousMember
    /// [`NotAccessible`]: vba_runtime::Error::NotAccessible
    pub fn get_global(&mut self, name: &str) -> Result<Variant, Error> {
        self.session.get_global(name).map_err(Error::Run)
    }

    /// Assigns a public module-level variable of a standard module. The
    /// value is converted to the declared type as an assignment in VBA is
    /// (error 6 on overflow, 13 when it cannot be converted); a constant,
    /// a fixed-size array and a user-defined type are refused with
    /// [`NotAccessible`](vba_runtime::Error::NotAccessible).
    pub fn set_global(&mut self, name: &str, value: Variant) -> Result<(), Error> {
        self.session.set_global(name, value).map_err(Error::Run)
    }

    /// Describes every module, procedure and module-level variable of the
    /// project, whatever its visibility. Event handlers such as
    /// `Form_Current` are private, so this is how a host finds them; see
    /// also [`modules_defining`](Instance::modules_defining).
    pub fn modules(&self) -> Vec<ModuleSummary> {
        self.session.modules()
    }

    /// The public surface: [`modules`](Instance::modules) without private
    /// procedures and variables.
    pub fn list_public_members(&self) -> Vec<ModuleSummary> {
        self.session.list_public_members()
    }

    /// The class-like modules (class, document and designer modules) that
    /// define a procedure called `name`, public or private: which forms
    /// define `Form_Current`.
    pub fn modules_defining(&self, name: &str) -> Vec<String> {
        self.session.modules_defining(name)
    }
}

//! Diagnostic codes of semantic analysis.

use vba_diagnostics::Code;

/// A name is declared twice in the same scope.
pub const DUPLICATE_DECLARATION: Code = Code("SEM001");
/// An undeclared name is used under `Option Explicit` (or cannot be implicitly declared).
pub const UNDECLARED_NAME: Code = Code("SEM002");
/// A name resolves to nothing at all.
pub const UNDEFINED_NAME: Code = Code("SEM003");
/// A named type is neither a project type nor provided by a library.
pub const UNRESOLVED_LIBRARY_TYPE: Code = Code("SEM004");
/// A library member could not be verified (informational, bound late).
pub const LATE_BOUND_LIBRARY_MEMBER: Code = Code("SEM005");
/// A value cannot be converted to the required type.
pub const TYPE_MISMATCH: Code = Code("SEM006");
/// `Set`/`Let` misuse (object assigned without `Set`, `Set` on a non-object).
pub const ASSIGNMENT_KIND: Code = Code("SEM007");
/// Wrong argument list (count, unknown or duplicate named argument, missing required).
pub const BAD_ARGUMENTS: Code = Code("SEM008");
/// The expression cannot be called, indexed or used like this.
pub const NOT_CALLABLE: Code = Code("SEM009");
/// A constant expression is required, or constant evaluation failed.
pub const CONSTANT_EXPRESSION: Code = Code("SEM010");
/// `Implements` is incomplete or inconsistent.
pub const IMPLEMENTS: Code = Code("SEM011");
/// `Property Get/Let/Set` procedures disagree.
pub const PROPERTY_PAIRING: Code = Code("SEM012");
/// Label problems: undefined, duplicated or in another procedure.
pub const LABEL: Code = Code("SEM013");
/// `Exit` does not match an enclosing construct.
pub const EXIT_MISUSE: Code = Code("SEM014");
/// A name is defined in several modules and the reference is ambiguous.
pub const AMBIGUOUS_NAME: Code = Code("SEM015");
/// `Option` statements are misplaced or repeated.
pub const OPTION_PLACEMENT: Code = Code("SEM016");
/// `WithEvents`/`Event`/`RaiseEvent` misuse.
pub const EVENTS: Code = Code("SEM017");
/// Invalid `For`/`For Each` control variable.
pub const CONTROL_VARIABLE: Code = Code("SEM018");
/// A member does not exist on a statically known type.
pub const NO_SUCH_MEMBER: Code = Code("SEM019");
/// `Me`, `.member` or `!member` used where no object is available.
pub const NO_OBJECT_CONTEXT: Code = Code("SEM020");
/// Array declaration or use is invalid.
pub const ARRAY_RULE: Code = Code("SEM021");
/// `Declare` statement problems.
pub const DECLARE: Code = Code("SEM022");
/// A class lifetime procedure (`Class_Initialize`/`Class_Terminate`) has a bad signature.
pub const CLASS_HOOK: Code = Code("SEM023");
/// Parameter list rules (`Optional`, `ParamArray`).
pub const PARAMETER_RULE: Code = Code("SEM024");
/// Assignment to something that cannot be assigned.
pub const NOT_ASSIGNABLE: Code = Code("SEM025");
/// `ByRef` argument type differs from the declared parameter type.
pub const BYREF_MISMATCH: Code = Code("SEM026");
/// UDT or Enum declaration problems.
pub const TYPE_DECLARATION: Code = Code("SEM027");
/// Visibility: the name exists but is not accessible from here.
pub const NOT_ACCESSIBLE: Code = Code("SEM028");
/// A construct that this implementation does not analyse yet.
pub const UNSUPPORTED: Code = Code("SEM099");

//! VBA's late-bound call resolution (MS-VBAL §5.6 and §5.4.3 semantics on top
//! of `IDispatch`).
//!
//! Implemented rules:
//!
//! * A member name is resolved with `GetIDsOfNames` (case-insensitive);
//!   failure is error 438. A named argument that does not resolve is error
//!   448.
//! * Reads invoke with `DISPATCH_METHOD | DISPATCH_PROPERTYGET`.
//! * `Let` invokes `PROPERTYPUT` with the value let-coerced (an object value
//!   is replaced by its default member's value, repeatedly); `Set` invokes
//!   `PROPERTYPUTREF` and requires an object (or `Nothing`), else error 424.
//! * **Default member chaining**: when a member's type information says it
//!   takes fewer parameters than arguments were supplied, the member is
//!   called with its own parameters, the result must be an object, and the
//!   surplus arguments go to that object's default member (`DISPID_VALUE`),
//!   recursively. This yields `rs.Fields("Name")`, `rs("Name")`, `rs!Name`
//!   and `Forms!Orders!Total` for both reads and assignments.
//! * `a!key` is the default member of `a` called with the string `"key"`.
//! * `[expr]` is `DISPID_EVALUATE` with the text.
//! * ByRef arguments are written back to the caller's [`Arg`]; by-value
//!   arguments (and arguments of ByVal parameters) never are.
//! * Errors become [`ErrInfo`]; `Err.Source` defaults to the object's class
//!   name.
//! * **Early binding**: [`invoke_early`] calls a member by the id a type
//!   library declares, skipping the name lookup, when the object says it
//!   knows that id for that member ([`Dispatch::accepts_dispid`]); it
//!   otherwise returns `None` so the caller falls back to the name.

use crate::dispatch::{Arg, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, names_equal};
use crate::error::{AutoResult, ErrInfo};
use crate::table::{ObjectTable, Owned};
use vba_core::VbaError;
use vba_value::{ObjectHandle, ObjectRef, Variant};

/// The argument list at a call site.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallArgs {
    /// Positional arguments (omitted ones are [`Arg::missing`]).
    pub positional: Vec<Arg>,
    /// Named arguments, applied after the positionals.
    pub named: Vec<(String, Arg)>,
}

impl CallArgs {
    /// No arguments.
    pub fn none() -> Self {
        Self::default()
    }

    /// Positional arguments.
    pub fn positional(args: Vec<Arg>) -> Self {
        CallArgs {
            positional: args,
            named: Vec::new(),
        }
    }

    /// Positional by-value arguments.
    pub fn values<I: IntoIterator<Item = Variant>>(vals: I) -> Self {
        Self::positional(vals.into_iter().map(Arg::val).collect())
    }

    /// Adds a named argument.
    pub fn with_named(mut self, name: &str, arg: Arg) -> Self {
        self.named.push((name.to_string(), arg));
        self
    }
}

/// How the final step of a chain is used.
#[derive(Debug, Clone, PartialEq)]
pub enum Final {
    /// Read as a value or object (`x = a.b`, `Set x = a.b`): no let-coercion
    /// of the result is done here; see [`let_coerce`].
    Get,
    /// A call statement (`a.b args`); the result may be discarded.
    Call,
    /// `Let a.b = value`.
    Let(Variant),
    /// `Set a.b = value`.
    Set(Variant),
}

/// One step of a member-access chain.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `.Name(args)`.
    Member {
        /// Member name.
        name: String,
        /// Arguments.
        args: CallArgs,
    },
    /// `obj(args)`: the default member.
    Default(CallArgs),
    /// `!Key`.
    Bang(String),
    /// `[expression]`.
    Evaluate(String),
}

impl Step {
    /// A `.Name(args)` step.
    pub fn member(name: &str, args: CallArgs) -> Step {
        Step::Member {
            name: name.to_string(),
            args,
        }
    }

    /// A `!Key` step.
    pub fn bang(key: &str) -> Step {
        Step::Bang(key.to_string())
    }
}

#[derive(Clone, Copy)]
enum Target<'a> {
    Named(&'a str),
    /// A member id known in advance (early binding).
    Id(DispId),
    Default,
    Evaluate,
}

fn err(n: i32, source: &str) -> ErrInfo {
    ErrInfo::standard(n).with_source(source)
}

fn param_by_ref(info: &Option<MemberInfo>, i: usize) -> bool {
    match info {
        Some(info) => match info.params.get(i) {
            Some(p) => p.by_ref,
            None => info.has_param_array(),
        },
        None => true,
    }
}

fn write_back(dst: &mut CallArgs, src: &DispParams, info: &Option<MemberInfo>) {
    for (i, a) in dst.positional.iter_mut().enumerate() {
        if a.by_ref
            && param_by_ref(info, i)
            && let Some(s) = src.args.get(i)
        {
            a.value = s.value.clone();
        }
    }
    let first_named = src
        .named
        .iter()
        .filter(|(id, _)| *id != DispId::PROPERTYPUT);
    for ((_, a), (id, s)) in dst.named.iter_mut().zip(first_named) {
        let by_ref = a.by_ref && usize::try_from(id.0).is_ok_and(|i| param_by_ref(info, i));
        if by_ref {
            a.value = s.value.clone();
        }
    }
}

fn invoke_on(
    table: &ObjectTable,
    h: ObjectHandle,
    target: Target<'_>,
    args: &mut CallArgs,
    fin: &Final,
) -> AutoResult<Variant> {
    let obj = table.get(h)?;
    let id = match target {
        Target::Named(n) => obj
            .get_ids_of_names(&[n])
            .map_err(|_| err(438, &obj.class_name()))?
            .first()
            .copied()
            .ok_or_else(|| err(438, &obj.class_name()))?,
        Target::Id(id) => id,
        Target::Default => DispId::VALUE,
        Target::Evaluate => DispId::EVALUATE,
    };
    invoke_resolved(table, h, &*obj, id, target, args, fin)
}

fn invoke_resolved(
    table: &ObjectTable,
    h: ObjectHandle,
    obj: &dyn Dispatch,
    id: DispId,
    target: Target<'_>,
    args: &mut CallArgs,
    fin: &Final,
) -> AutoResult<Variant> {
    // Only errors need the class name.
    let src = || obj.class_name();
    let info = obj.member_info(id);

    // Default member chaining: surplus arguments go to the result's default.
    if args.named.is_empty()
        && let Some(i) = &info
        && !i.has_param_array()
        && args.positional.len() > i.fixed_param_count()
    {
        let n = i.fixed_param_count();
        let mut head = CallArgs::positional(args.positional[..n].to_vec());
        let r = invoke_on(table, h, target, &mut head, &Final::Get)?;
        args.positional[..n].clone_from_slice(&head.positional);
        let rh = match &r {
            Variant::Object(ObjectRef::Handle(rh)) => *rh,
            other => {
                table.release_value(other);
                return Err(err(450, &src()));
            }
        };
        let _keep = Owned::adopt(table, rh);
        let mut tail = CallArgs::positional(args.positional[n..].to_vec());
        let res = invoke_on(table, rh, Target::Default, &mut tail, fin);
        args.positional[n..].clone_from_slice(&tail.positional);
        return res;
    }

    // Let to a member without a Property Let: assign to the default member
    // of the object it returns (`rs!Name = v`, `rs.Fields("x") = v`).
    if matches!(fin, Final::Let(_))
        && let Some(i) = &info
        && !i.supports.contains(InvokeFlags::PROPERTY_PUT)
        && i.supports
            .intersects(InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET)
    {
        let r = invoke_on(table, h, target, args, &Final::Get)?;
        let rh = match &r {
            Variant::Object(ObjectRef::Handle(rh)) => *rh,
            other => {
                table.release_value(other);
                return Err(err(451, &src()));
            }
        };
        let _keep = Owned::adopt(table, rh);
        return invoke_on(table, rh, Target::Default, &mut CallArgs::none(), fin);
    }

    let mut dp = DispParams {
        args: args.positional.clone(),
        named: Vec::new(),
    };
    if !args.named.is_empty() {
        let member_name = match (target, &info) {
            (Target::Named(n), _) => n.to_string(),
            (_, Some(i)) => i.name.clone(),
            _ => return Err(err(448, &src())),
        };
        let mut names: Vec<&str> = vec![&member_name];
        names.extend(args.named.iter().map(|(n, _)| n.as_str()));
        let ids = obj.get_ids_of_names(&names).map_err(|_| err(448, &src()))?;
        if ids.len() != names.len() {
            return Err(err(448, &src()));
        }
        for ((_, a), id) in args.named.iter().zip(&ids[1..]) {
            dp.named.push((*id, a.clone()));
        }
    }

    let mut temp: Option<Variant> = None;
    let flags = match fin {
        Final::Get | Final::Call => InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
        Final::Let(v) => {
            let cv = let_coerce(table, v)?;
            dp.named.push((DispId::PROPERTYPUT, Arg::val(cv.clone())));
            temp = Some(cv);
            InvokeFlags::PROPERTY_PUT
        }
        Final::Set(v) => {
            if !matches!(v, Variant::Object(_)) {
                return Err(err(424, &src()));
            }
            dp.named.push((DispId::PROPERTYPUT, Arg::val(v.clone())));
            InvokeFlags::PROPERTY_PUT_REF
        }
    };
    let r = obj.invoke(table, id, flags, &mut dp);
    if let Some(t) = temp {
        table.release_value(&t);
    }
    write_back(args, &dp, &info);
    match r {
        Ok(v) => Ok(match fin {
            Final::Let(_) | Final::Set(_) => Variant::Empty,
            _ => v,
        }),
        Err(e) => Err(e.into_err_info(&src())),
    }
}

/// Invokes the named member of `h`. See the module documentation.
pub fn invoke_member(
    table: &ObjectTable,
    h: ObjectHandle,
    name: &str,
    args: &mut CallArgs,
    fin: &Final,
) -> AutoResult<Variant> {
    invoke_on(table, h, Target::Named(name), args, fin)
}

/// Early-bound call: invokes member `name` of `h` by the id `id` that a type
/// library declares for it, without a name lookup.
///
/// Returns `None`, with nothing invoked, when the object does not accept the
/// id for that name ([`Dispatch::accepts_dispid`]) or the handle is stale;
/// the caller then makes the late-bound call by name, which reports the
/// proper error. Otherwise the call behaves exactly like
/// [`invoke_member`], including ByRef write-back and error mapping.
pub fn invoke_early(
    table: &ObjectTable,
    h: ObjectHandle,
    id: DispId,
    name: &str,
    args: &mut CallArgs,
    fin: &Final,
) -> Option<AutoResult<Variant>> {
    let obj = table.get(h).ok()?;
    if !obj.accepts_dispid(id, name) {
        return None;
    }
    Some(invoke_resolved(
        table,
        h,
        &*obj,
        id,
        Target::Id(id),
        args,
        fin,
    ))
}

/// Invokes the default member of `h` (`obj(args)`).
pub fn invoke_default(
    table: &ObjectTable,
    h: ObjectHandle,
    args: &mut CallArgs,
    fin: &Final,
) -> AutoResult<Variant> {
    invoke_on(table, h, Target::Default, args, fin)
}

/// `h!key`: the default member called with the string `key`.
pub fn bang(table: &ObjectTable, h: ObjectHandle, key: &str, fin: &Final) -> AutoResult<Variant> {
    let mut a = CallArgs::values([Variant::from(key)]);
    invoke_on(table, h, Target::Default, &mut a, fin)
}

/// `h[expr]`: `DISPID_EVALUATE`.
pub fn evaluate(table: &ObjectTable, h: ObjectHandle, expr: &str) -> AutoResult<Variant> {
    let mut a = CallArgs::values([Variant::from(expr)]);
    invoke_on(table, h, Target::Evaluate, &mut a, &Final::Get)
}

/// Let-coercion of an object to a value: while the value is an object,
/// replace it with the result of its default member. `Nothing` is error 91; an
/// object without a default member is error 438. Non-objects are returned
/// unchanged. The result never holds an object reference.
pub fn let_coerce(table: &ObjectTable, v: &Variant) -> AutoResult<Variant> {
    let mut cur = v.clone();
    let mut owned = false;
    for _ in 0..32 {
        match &cur {
            Variant::Object(ObjectRef::Nothing) => {
                return Err(ErrInfo::standard(91));
            }
            Variant::Object(ObjectRef::Handle(h)) => {
                let h = *h;
                let r = invoke_on(
                    table,
                    h,
                    Target::Default,
                    &mut CallArgs::none(),
                    &Final::Get,
                );
                if owned {
                    table.release_value(&cur);
                }
                cur = r?;
                owned = true;
            }
            _ => return Ok(cur),
        }
    }
    if owned {
        table.release_value(&cur);
    }
    Err(ErrInfo::from(VbaError::new(28)))
}

struct Temp<'t>(&'t ObjectTable, Variant);
impl Drop for Temp<'_> {
    fn drop(&mut self) {
        self.0.release_value(&self.1);
    }
}

/// Evaluates a chain of steps starting at `root`.
///
/// Every step but the last is a read that must produce an object (`Nothing`
/// is error 91, a non-object is error 424); the last step is used as `fin`
/// says. Intermediate references are released. The returned value carries an
/// object reference the caller owns. ByRef results are written back into the
/// steps' argument lists.
pub fn eval_chain(
    table: &ObjectTable,
    root: &Variant,
    steps: &mut [Step],
    fin: Final,
) -> AutoResult<Variant> {
    if steps.is_empty() {
        return Err(ErrInfo::standard(5));
    }
    table.add_ref_value(root);
    let mut cur = Temp(table, root.clone());
    let n = steps.len();
    for (i, step) in steps.iter_mut().enumerate() {
        let h = match &cur.1 {
            Variant::Object(ObjectRef::Handle(h)) => *h,
            Variant::Object(ObjectRef::Nothing) => return Err(ErrInfo::standard(91)),
            _ => return Err(ErrInfo::standard(424)),
        };
        let f = if i + 1 == n { &fin } else { &Final::Get };
        let r = match step {
            Step::Member { name, args } => invoke_on(table, h, Target::Named(name), args, f),
            Step::Default(args) => invoke_on(table, h, Target::Default, args, f),
            Step::Bang(key) => {
                let mut a = CallArgs::values([Variant::from(key.as_str())]);
                invoke_on(table, h, Target::Default, &mut a, f)
            }
            Step::Evaluate(expr) => {
                let mut a = CallArgs::values([Variant::from(expr.as_str())]);
                invoke_on(table, h, Target::Evaluate, &mut a, f)
            }
        }?;
        if i + 1 == n {
            return Ok(r);
        }
        cur = Temp(table, r);
    }
    unreachable!("steps is non-empty")
}

/// Case-insensitive member-name equality, re-exported for hosts that resolve
/// names themselves.
pub fn same_name(a: &str, b: &str) -> bool {
    names_equal(a, b)
}

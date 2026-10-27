//! Which facade types may cross threads. A change here is a change of the
//! documented contract (see `docs/architecture/public-api.md`).

use vba::{
    Arg, CallLimits, CallResult, CallTarget, CancelToken, Capability, Diagnostic, Error,
    EventOutcome, ModuleKind, Outcome, Policy, Profile, Project, Report, ResourceLimits,
    RunStatus, Variant,
};

fn send_sync<T: Send + Sync>() {}
fn send<T: Send>() {}

#[test]
fn plain_data_is_send_and_sync() {
    send_sync::<Error>();
    send_sync::<Project>();
    send_sync::<Report>();
    send_sync::<Diagnostic>();
    send_sync::<CancelToken>();
    send_sync::<CallLimits>();
    send_sync::<Policy>();
    send_sync::<Capability>();
    send_sync::<ResourceLimits>();
    send_sync::<Profile>();
    send_sync::<ModuleKind>();
    send_sync::<RunStatus>();
}

#[test]
fn values_and_results_are_send() {
    send::<Variant>();
    send::<Arg>();
    send::<CallTarget>();
    send::<Outcome>();
    send::<CallResult>();
    send::<EventOutcome>();
}

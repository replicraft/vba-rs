//! Shared test harness: a deterministic host and call helpers.
#![allow(dead_code)]

use vba_stdlib::{Ctx, FixedHost, State, StdError, StdResult, call_function};
use vba_value::{CompareMode, Date, Variant, VbaError};

pub struct Env {
    pub host: FixedHost,
    pub state: State,
    pub compare: CompareMode,
}

impl Env {
    pub fn new() -> Self {
        let mut host = FixedHost::new(Date::from_ymd_hms(2000, 1, 1, 6, 0, 0).unwrap());
        host.env.insert("HOME".into(), "/home/x".into());
        host.env.insert("PATH".into(), "/bin".into());
        Env {
            host,
            state: State::default(),
            compare: CompareMode::Binary,
        }
    }

    pub fn call(&mut self, name: &str, args: &[Variant]) -> StdResult<Variant> {
        let mut ctx = Ctx::new(&self.host, &mut self.state);
        ctx.compare = self.compare;
        let a: Vec<Option<Variant>> = args.iter().cloned().map(Some).collect();
        call_function(name, &mut ctx, &a)
    }

    /// Calls with explicit omitted arguments (`None`).
    pub fn call_opt(&mut self, name: &str, args: &[Option<Variant>]) -> StdResult<Variant> {
        let mut ctx = Ctx::new(&self.host, &mut self.state);
        ctx.compare = self.compare;
        call_function(name, &mut ctx, args)
    }
}

pub fn s(x: &str) -> Variant {
    Variant::from_text(x)
}

pub fn d(y: i32, m: u32, dd: u32, h: u32, mi: u32, sec: u32) -> Variant {
    Variant::Date(Date::from_ymd_hms(y, m, dd, h, mi, sec).unwrap())
}

pub fn err(n: i32) -> StdError {
    StdError::Vba(VbaError::new(n))
}

/// Builds a Variant array of the given values, lower bound 0.
pub fn arr(items: &[Variant]) -> Variant {
    vba_stdlib::info::array_of(items, 0).unwrap()
}

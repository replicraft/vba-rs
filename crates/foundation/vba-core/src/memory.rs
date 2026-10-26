//! Memory accounting shared by every owner of heap-like data.
//!
//! One [`MemoryMeter`] belongs to one run budget. Owners of data a program
//! can grow (strings, arrays, collections, dictionaries, in-memory files,
//! output buffers) charge their bytes to it when they allocate and credit
//! them when they are dropped.
//!
//! Two ways of charging exist:
//!
//! * **Fallible** ([`MemoryMeter::try_alloc`]): the owner asks first and the
//!   program gets an untrappable limit fault instead of the allocation.
//!   Used where the owner can fail (VM objects, files, dictionary items).
//! * **Tracked** ([`Charge`], [`Tracked`]): values that are created by
//!   functions which cannot fail (a string is built by `concat`, `Mid$`,
//!   `Replace`, ...) register with the meter that is *current on the thread*
//!   (see [`MemoryMeter::enter`]) and credit it again when dropped, wherever
//!   that happens. Such a charge cannot be refused, so the meter may be over
//!   its limit; [`MemoryMeter::exceeded`] reports it and the interpreter
//!   checks it before every instruction. The overshoot is bounded by what
//!   one instruction can build.
//!
//! The accounting is conservative: it counts *capacity* actually allocated,
//! including temporary copies that are alive, and small allocations (under
//! [`TRACK_MIN_BYTES`]) are left to the per-element charge of the container
//! that holds them.

use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Allocations smaller than this many bytes are not tracked individually.
pub const TRACK_MIN_BYTES: usize = 64;

/// What one allocation adds to [`MemoryMeter::allocated_total`] on top of its
/// size: the cost of the allocator calls, which for many small objects is
/// larger than copying their bytes.
pub const ALLOCATION_COST: u64 = 192;

/// The memory budget is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryExceeded;

impl fmt::Display for MemoryExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("memory limit exceeded")
    }
}

impl std::error::Error for MemoryExceeded {}

#[derive(Debug)]
struct Shared {
    used: AtomicU64,
    /// `u64::MAX` for no limit.
    limit: AtomicU64,
    /// Every byte ever charged, never credited: the volume of data the run
    /// has built, a proxy for the CPU time spent copying it.
    allocated: AtomicU64,
}

/// A shared, thread-safe byte counter with an optional limit.
#[derive(Debug, Clone)]
pub struct MemoryMeter(Arc<Shared>);

impl MemoryMeter {
    /// A meter with the given limit (`None` for unlimited).
    pub fn new(limit: Option<u64>) -> Self {
        MemoryMeter(Arc::new(Shared {
            used: AtomicU64::new(0),
            limit: AtomicU64::new(limit.unwrap_or(u64::MAX)),
            allocated: AtomicU64::new(0),
        }))
    }

    /// The limit, if any.
    pub fn limit(&self) -> Option<u64> {
        match self.0.limit.load(Ordering::Relaxed) {
            u64::MAX => None,
            n => Some(n),
        }
    }

    /// Replaces the limit (`None` for unlimited). Bytes already charged stay
    /// charged; a lower limit makes the next allocation fail.
    pub fn set_limit(&self, limit: Option<u64>) {
        self.0
            .limit
            .store(limit.unwrap_or(u64::MAX), Ordering::Relaxed);
    }

    /// Bytes currently charged.
    pub fn used(&self) -> u64 {
        self.0.used.load(Ordering::Relaxed)
    }

    /// The total of all bytes ever charged (never decreases). The budget turns
    /// this volume into steps, so building data costs CPU budget as well as
    /// memory.
    #[inline]
    pub fn allocated_total(&self) -> u64 {
        self.0.allocated.load(Ordering::Relaxed)
    }

    /// Whether more is charged than the limit allows. Tracked allocations
    /// cannot be refused, so this is how an over-spent meter shows.
    #[inline]
    pub fn exceeded(&self) -> bool {
        self.0.used.load(Ordering::Relaxed) > self.0.limit.load(Ordering::Relaxed)
    }

    /// Charges `bytes` if that stays within the limit; nothing is charged
    /// otherwise.
    pub fn try_alloc(&self, bytes: u64) -> Result<(), MemoryExceeded> {
        let limit = self.0.limit.load(Ordering::Relaxed);
        let mut cur = self.0.used.load(Ordering::Relaxed);
        loop {
            let next = cur.saturating_add(bytes);
            if next > limit {
                return Err(MemoryExceeded);
            }
            match self
                .0
                .used
                .compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => {
                    self.0
                        .allocated
                        .fetch_add(bytes + ALLOCATION_COST, Ordering::Relaxed);
                    return Ok(());
                }
                Err(c) => cur = c,
            }
        }
    }

    /// Charges `bytes` without checking the limit.
    pub fn alloc_unchecked(&self, bytes: u64) {
        self.0.used.fetch_add(bytes, Ordering::Relaxed);
        self.0
            .allocated
            .fetch_add(bytes + ALLOCATION_COST, Ordering::Relaxed);
    }

    /// Credits `bytes` (never below zero).
    pub fn free(&self, bytes: u64) {
        let mut cur = self.0.used.load(Ordering::Relaxed);
        loop {
            match self.0.used.compare_exchange_weak(
                cur,
                cur.saturating_sub(bytes),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(c) => cur = c,
            }
        }
    }

    /// Makes this the meter that [`Charge`] and [`Tracked`] values created
    /// on this thread register with, until the guard is dropped.
    pub fn enter(&self) -> MeterScope {
        let prev = CURRENT.with(|c| c.replace(Some(self.clone())));
        MeterScope { prev }
    }

    /// The meter current on this thread, if any.
    pub fn current() -> Option<MemoryMeter> {
        CURRENT.with(|c| c.borrow().clone())
    }

    /// Whether two handles are the same meter.
    pub fn same(&self, other: &MemoryMeter) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

thread_local! {
    static CURRENT: RefCell<Option<MemoryMeter>> = const { RefCell::new(None) };
}

/// Restores the previously current meter when dropped.
#[derive(Debug)]
pub struct MeterScope {
    prev: Option<MemoryMeter>,
}

impl Drop for MeterScope {
    fn drop(&mut self) {
        let prev = self.prev.take();
        CURRENT.with(|c| *c.borrow_mut() = prev);
    }
}

/// A claim on a meter, credited by [`Tracked::release`]. Pointer-sized: the
/// owner supplies the byte count again on release (it derives it from its
/// own capacity), so a string needs no extra field.
#[derive(Debug, Default)]
pub struct Tracked(Option<MemoryMeter>);

impl Tracked {
    /// Registers `bytes` with the current meter if the allocation is large
    /// enough to track and a meter is current.
    #[inline]
    pub fn new(bytes: usize) -> Tracked {
        if bytes < TRACK_MIN_BYTES {
            return Tracked(None);
        }
        Tracked(MemoryMeter::current().inspect(|m| m.alloc_unchecked(bytes as u64)))
    }

    /// Credits `bytes`, the amount passed to [`Tracked::new`].
    #[inline]
    pub fn release(&mut self, bytes: usize) {
        if let Some(m) = self.0.take() {
            m.free(bytes as u64);
        }
    }

    /// Whether a meter holds this claim.
    pub fn is_tracked(&self) -> bool {
        self.0.is_some()
    }
}

/// Like [`Tracked`] but remembers its byte count and can be resized; the
/// owner of a growable buffer keeps one and credits automatically on drop.
#[derive(Debug, Default)]
pub struct Charge {
    meter: Option<MemoryMeter>,
    bytes: u64,
}

impl Charge {
    /// No claim.
    pub const fn none() -> Charge {
        Charge {
            meter: None,
            bytes: 0,
        }
    }

    /// A claim of `bytes` on the current meter (none when no meter is
    /// current).
    pub fn new(bytes: u64) -> Charge {
        let mut c = Charge::none();
        c.set(bytes);
        c
    }

    /// A claim on a given meter, charged without checking the limit.
    pub fn on(meter: &MemoryMeter, bytes: u64) -> Charge {
        meter.alloc_unchecked(bytes);
        Charge {
            meter: Some(meter.clone()),
            bytes,
        }
    }

    /// Changes the claim to `bytes`. A claim that has no meter yet takes
    /// the current one.
    pub fn set(&mut self, bytes: u64) {
        if self.meter.is_none() {
            self.meter = MemoryMeter::current();
        }
        if let Some(m) = &self.meter {
            if bytes > self.bytes {
                m.alloc_unchecked(bytes - self.bytes);
            } else if bytes < self.bytes {
                m.free(self.bytes - bytes);
            }
            self.bytes = bytes;
        }
    }

    /// Like [`Charge::set`] but refuses to grow past the limit; on failure
    /// the claim is unchanged.
    pub fn try_set(&mut self, bytes: u64) -> Result<(), MemoryExceeded> {
        if self.meter.is_none() {
            self.meter = MemoryMeter::current();
        }
        if let Some(m) = &self.meter {
            if bytes > self.bytes {
                m.try_alloc(bytes - self.bytes)?;
            } else {
                m.free(self.bytes - bytes);
            }
            self.bytes = bytes;
        }
        Ok(())
    }

    /// Bytes claimed.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl Clone for Charge {
    /// A clone claims nothing; the owner of the cloned buffer sets it.
    fn clone(&self) -> Self {
        Charge::none()
    }
}

impl PartialEq for Charge {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Drop for Charge {
    fn drop(&mut self) {
        if let Some(m) = self.meter.take() {
            m.free(self.bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_alloc_respects_the_limit_and_free_credits() {
        let m = MemoryMeter::new(Some(100));
        assert!(m.try_alloc(60).is_ok());
        assert_eq!(m.try_alloc(41), Err(MemoryExceeded));
        assert_eq!(m.used(), 60);
        m.free(60);
        assert_eq!(m.used(), 0);
        m.free(5);
        assert_eq!(m.used(), 0, "never below zero");
    }

    #[test]
    fn tracked_claims_follow_the_current_meter_and_survive_the_scope() {
        let m = MemoryMeter::new(Some(1000));
        let mut t;
        {
            let _g = m.enter();
            t = Tracked::new(500);
            assert_eq!(m.used(), 500);
            let small = Tracked::new(10);
            assert!(!small.is_tracked());
        }
        assert_eq!(m.used(), 500);
        assert!(MemoryMeter::current().is_none());
        t.release(500);
        assert_eq!(m.used(), 0);
    }

    #[test]
    fn tracked_charges_can_overshoot_and_show_it() {
        let m = MemoryMeter::new(Some(100));
        let _g = m.enter();
        let c = Charge::new(500);
        assert!(m.exceeded());
        drop(c);
        assert!(!m.exceeded());
    }

    #[test]
    fn the_allocated_total_only_grows() {
        let m = MemoryMeter::new(Some(100));
        m.try_alloc(40).unwrap();
        m.alloc_unchecked(10);
        m.free(50);
        assert_eq!(m.used(), 0);
        let total = m.allocated_total();
        assert_eq!(total, 50 + 2 * ALLOCATION_COST);
        assert!(m.try_alloc(101).is_err());
        assert_eq!(m.allocated_total(), total);
    }

    #[test]
    fn scopes_nest() {
        let a = MemoryMeter::new(None);
        let b = MemoryMeter::new(None);
        let _ga = a.enter();
        {
            let _gb = b.enter();
            assert!(MemoryMeter::current().unwrap().same(&b));
        }
        assert!(MemoryMeter::current().unwrap().same(&a));
    }

    #[test]
    fn charge_resizes() {
        let m = MemoryMeter::new(Some(100));
        let _g = m.enter();
        let mut c = Charge::new(40);
        c.set(80);
        assert_eq!(m.used(), 80);
        assert!(c.try_set(200).is_err());
        assert_eq!(m.used(), 80);
        c.set(10);
        assert_eq!(m.used(), 10);
    }
}

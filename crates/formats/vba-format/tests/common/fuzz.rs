//! Deterministic mutation helpers shared by the hostile-input tests.

use std::time::{Duration, Instant};

/// Deterministic xorshift64* generator.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

/// Values that sit on arithmetic boundaries.
pub const U32_SPECIAL: [u32; 8] = [
    0,
    1,
    0xFFFF_FFFF,
    0xFFFF_FFFE,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF,
    0x1_0000,
];

/// Runs `f` and fails if it takes longer than `budget`.
pub fn within<T>(budget: Duration, what: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    let took = start.elapsed();
    assert!(took <= budget, "{what} took {took:?} (budget {budget:?})");
    out
}

pub fn mutate(rng: &mut Rng, base: &[u8]) -> Vec<u8> {
    let mut b = base.to_vec();
    if b.is_empty() {
        return b;
    }
    match rng.below(9) {
        0 => {
            for _ in 0..1 + rng.below(8) {
                let i = rng.below(b.len());
                b[i] ^= 1 << rng.below(8);
            }
        }
        1 => {
            for _ in 0..1 + rng.below(8) {
                let i = rng.below(b.len());
                b[i] = [0x00, 0xFF, 0x7F, 0x80][rng.below(4)];
            }
        }
        2 => {
            let cut = rng.below(b.len() + 1);
            b.truncate(cut);
        }
        3 => {
            let extra = rng.below(4096);
            for _ in 0..extra {
                b.push(rng.next() as u8);
            }
        }
        4 => {
            // duplicate a chunk
            let s = rng.below(b.len());
            let e = (s + 1 + rng.below(600)).min(b.len());
            let chunk = b[s..e].to_vec();
            let at = rng.below(b.len() + 1);
            b.splice(at..at, chunk);
        }
        5 => {
            // remove a chunk
            let s = rng.below(b.len());
            let e = (s + 1 + rng.below(600)).min(b.len());
            b.drain(s..e);
        }
        6 => {
            // overwrite a u32 with a boundary value
            if b.len() >= 4 {
                let i = rng.below(b.len() - 3);
                let v = U32_SPECIAL[rng.below(U32_SPECIAL.len())];
                b[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        7 => {
            // overwrite a u16
            if b.len() >= 2 {
                let i = rng.below(b.len() - 1);
                let v = [0u16, 1, 0xFFFF, 0xFFFE, 0x8000][rng.below(5)];
                b[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
        }
        _ => {
            // splice random bytes
            let at = rng.below(b.len() + 1);
            let n = rng.below(64);
            let junk: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
            b.splice(at..at, junk);
        }
    }
    b
}

//! Function-pointer tokens for `AddressOf`.
//!
//! A token is what VBA code sees when it evaluates `AddressOf Proc`: a small
//! positive integer that fits a `Long` (so it also fits a 32-bit `LongPtr`).
//! It is **never** a native address and never refers to executable memory.
//! It only names a slot of a [`CallbackTable`], which belongs to one session.
//!
//! Layout (31 bits, always positive):
//!
//! | bits | meaning |
//! | --- | --- |
//! | 30 | marker, always 1, so a token is never 0 and small integers are rejected |
//! | 29..18 | 12-bit check value, a keyed mix of slot, generation and the table secret |
//! | 17..10 | low 8 bits of the table generation |
//! | 9..0 | slot index |
//!
//! Arithmetic on a token (adding, multiplying, negating, flipping a bit)
//! changes a field and fails the check with probability 1 - 2^-12; and even a
//! token that passed by luck can only name a slot the program itself created
//! with `AddressOf`, whose target is then checked against the signature the
//! callee expects. The check value is a defence in depth, not a secret: the
//! default secret is a constant so runs stay reproducible.
//!
//! [`CallbackTable::invalidate_all`] starts a new generation; every token
//! minted before it is [`TokenError::Stale`]. A generation is identified by 8
//! bits, so a token can alias a token of a generation 256 resets later; it can
//! still only select one of the program's own procedures.

use thiserror::Error;

/// How many distinct targets one table can hold (10 index bits).
pub const MAX_CALLBACK_SLOTS: usize = 1 << 10;

/// Why a token was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TokenError {
    /// Not a token: out of range, marker or check value wrong, or the slot
    /// does not exist. This is what arithmetic on a token produces.
    #[error("not a valid callback token")]
    Forged,
    /// A well-formed token of an earlier generation (the session was reset).
    #[error("stale callback token (the session was reset after it was created)")]
    Stale,
    /// The table has no free slot.
    #[error("callback table full ({MAX_CALLBACK_SLOTS} procedures)")]
    Full,
}

/// The tokens of one session, each naming a target of type `T`.
#[derive(Debug, Clone)]
pub struct CallbackTable<T> {
    secret: u32,
    generation: u32,
    slots: Vec<T>,
}

fn mix(secret: u32, index: u32, generation: u32) -> u32 {
    let mut x =
        secret ^ index.wrapping_mul(0x9E37_79B1) ^ (generation & 0xFF).wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    x & 0xFFF
}

impl<T: Copy + PartialEq> CallbackTable<T> {
    /// An empty table. `secret` keys the check value.
    pub fn new(secret: u32) -> Self {
        CallbackTable {
            secret,
            generation: 0,
            slots: Vec::new(),
        }
    }

    fn encode(&self, index: usize) -> i64 {
        let i = index as u32;
        let token = (1u32 << 30)
            | (mix(self.secret, i, self.generation) << 18)
            | ((self.generation & 0xFF) << 10)
            | i;
        i64::from(token)
    }

    /// The token for `target`; asking again for the same target returns the
    /// same token.
    pub fn mint(&mut self, target: T) -> Result<i64, TokenError> {
        if let Some(i) = self.slots.iter().position(|t| *t == target) {
            return Ok(self.encode(i));
        }
        if self.slots.len() >= MAX_CALLBACK_SLOTS {
            return Err(TokenError::Full);
        }
        self.slots.push(target);
        Ok(self.encode(self.slots.len() - 1))
    }

    /// The target `value` names, if it is a token of the current generation.
    pub fn resolve(&self, value: i64) -> Result<T, TokenError> {
        if !(0..=i64::from(i32::MAX)).contains(&value) {
            return Err(TokenError::Forged);
        }
        let v = value as u32;
        if v & (1 << 30) == 0 {
            return Err(TokenError::Forged);
        }
        let index = v & 0x3FF;
        let gen8 = (v >> 10) & 0xFF;
        let check = (v >> 18) & 0xFFF;
        // The check is computed with the generation the token claims.
        if check != mix(self.secret, index, gen8) {
            return Err(TokenError::Forged);
        }
        if gen8 != self.generation & 0xFF {
            return Err(TokenError::Stale);
        }
        self.slots
            .get(index as usize)
            .copied()
            .ok_or(TokenError::Forged)
    }

    /// Forgets every token: the next generation starts with no slots.
    pub fn invalidate_all(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.slots.clear();
    }

    /// Number of live tokens.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether no token is live.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_is_stable_and_resolves() {
        let mut t = CallbackTable::new(7);
        let a = t.mint(10u32).unwrap();
        let b = t.mint(20u32).unwrap();
        assert_ne!(a, b);
        assert_eq!(t.mint(10u32).unwrap(), a);
        assert_eq!(t.resolve(a), Ok(10));
        assert_eq!(t.resolve(b), Ok(20));
        assert!(a > 0 && a <= i64::from(i32::MAX));
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn small_and_negative_integers_are_forged() {
        let mut t = CallbackTable::new(7);
        t.mint(1u32).unwrap();
        for v in [0, 1, -1, 4, 0x1000, i64::MIN, i64::MAX, 1 << 31, -(1 << 30)] {
            assert_eq!(t.resolve(v), Err(TokenError::Forged), "{v}");
        }
    }

    #[test]
    fn arithmetic_on_a_token_is_rejected() {
        let mut t = CallbackTable::new(0x5eed);
        let tok = t.mint(1u32).unwrap();
        let mut accepted = 0;
        for d in 1..4096i64 {
            for v in [tok + d, tok - d, tok ^ d, tok * (d + 1)] {
                if v != tok && t.resolve(v).is_ok() {
                    accepted += 1;
                }
            }
        }
        // Only one slot exists. A value other than the token resolves only on a
        // collision of the 12-bit check, and then names that same slot.
        assert!(accepted < 8, "{accepted}");
    }

    #[test]
    fn tokens_go_stale_after_invalidate() {
        let mut t = CallbackTable::new(3);
        let tok = t.mint(5u32).unwrap();
        t.invalidate_all();
        assert_eq!(t.resolve(tok), Err(TokenError::Stale));
        assert!(t.is_empty());
        let fresh = t.mint(5u32).unwrap();
        assert_ne!(fresh, tok);
        assert_eq!(t.resolve(fresh), Ok(5));
        assert_eq!(t.resolve(tok), Err(TokenError::Stale));
    }

    #[test]
    fn the_table_is_bounded() {
        let mut t = CallbackTable::new(1);
        for i in 0..MAX_CALLBACK_SLOTS as u32 {
            t.mint(i).unwrap();
        }
        assert_eq!(t.mint(u32::MAX), Err(TokenError::Full));
        // Existing targets still mint.
        assert!(t.mint(3).is_ok());
    }
}

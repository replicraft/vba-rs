//! A bounds-checked, VBA-visible address space.
//!
//! VBA code that calls `CopyMemory`, `StrPtr`, `VarPtr` and friends observes
//! integer addresses. This module provides those integers without ever
//! exposing a host pointer: every address indexes a region of this emulated
//! space, and every access must stay inside one region.

use crate::{Codepage, FfiError, Unmappable};
use std::collections::BTreeMap;
use thiserror::Error;
use vba_core::Charge;
use vba_profile::Bitness;
use vba_value::VbaString;

/// Lowest address ever handed out. It is above 0xFFFF so that APIs which
/// distinguish a character value from a pointer by the high word (for example
/// `CharUpperA`) can tell them apart.
const BASE: u64 = 0x0001_0000;
const GUARD: u64 = 16;
const DEFAULT_BUDGET: usize = 256 * 1024 * 1024;

/// Emulated memory faults. Real Windows would raise an access violation or
/// corrupt memory; vba-rs reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MemoryError {
    /// A null pointer was dereferenced.
    #[error("null pointer dereference")]
    NullPointer,
    /// The address is not inside any live region.
    #[error("address {addr:#x} is not mapped")]
    Unmapped {
        /// Faulting address.
        addr: u64,
    },
    /// The access starts in a region but runs past its end.
    #[error("access of {len} bytes at {addr:#x} leaves its region")]
    OutOfBounds {
        /// Start address.
        addr: u64,
        /// Requested length.
        len: usize,
    },
    /// The address space or the allocation budget is exhausted.
    #[error("emulated memory exhausted")]
    OutOfMemory,
    /// Zero-sized allocation.
    #[error("zero-sized allocation")]
    ZeroSize,
    /// `free` was given an address that is not a region base.
    #[error("{addr:#x} is not the base of a live allocation")]
    NotABase {
        /// Offending address.
        addr: u64,
    },
}

/// One live allocation; its bytes are also charged to the run's memory
/// meter (`max_memory_bytes`) and credited when it is freed.
#[derive(Debug)]
struct Region {
    bytes: Vec<u8>,
    _charge: Charge,
}

impl Region {
    fn new(bytes: Vec<u8>) -> Region {
        let charge = Charge::new(bytes.len() as u64 + 64);
        Region {
            bytes,
            _charge: charge,
        }
    }
}

/// The emulated address space of one VBA process.
#[derive(Debug)]
pub struct Memory {
    max_addr: u64,
    next: u64,
    budget: usize,
    used: usize,
    regions: BTreeMap<u64, Region>,
}

impl Memory {
    /// Creates an address space whose pointer width follows `bitness`.
    pub fn new(bitness: Bitness) -> Self {
        Self::with_budget(bitness, DEFAULT_BUDGET)
    }

    /// Creates an address space limited to `budget` live bytes.
    pub fn with_budget(bitness: Bitness, budget: usize) -> Self {
        let max_addr = match bitness {
            Bitness::B64 => 1 << 47,
            _ => u64::from(u32::MAX),
        };
        Memory {
            max_addr,
            next: BASE,
            budget,
            used: 0,
            regions: BTreeMap::new(),
        }
    }

    /// Allocates a zero-filled region and returns its base address.
    pub fn alloc(&mut self, size: usize) -> Result<u64, MemoryError> {
        if size == 0 {
            return Err(MemoryError::ZeroSize);
        }
        if self.used.checked_add(size).is_none_or(|u| u > self.budget) {
            return Err(MemoryError::OutOfMemory);
        }
        let addr = self.next.next_multiple_of(16);
        let end = addr
            .checked_add(size as u64)
            .filter(|e| *e <= self.max_addr)
            .ok_or(MemoryError::OutOfMemory)?;
        self.next = end + GUARD;
        self.used += size;
        self.regions.insert(addr, Region::new(vec![0; size]));
        Ok(addr)
    }

    /// Allocates a region holding `data`.
    pub fn alloc_bytes(&mut self, data: &[u8]) -> Result<u64, MemoryError> {
        let addr = self.alloc(data.len().max(1))?;
        self.write(addr, data)?;
        Ok(addr)
    }

    /// Allocates a NUL-terminated ANSI copy of `s`.
    pub fn alloc_ansi(&mut self, s: &VbaString, cp: Codepage) -> Result<u64, FfiError> {
        let mut bytes = cp.encode(s.as_units(), Unmappable::Reject, true)?.bytes;
        bytes.push(0);
        Ok(self.alloc_bytes(&bytes)?)
    }

    /// Allocates a NUL-terminated UTF-16 copy of `s` (what `StrPtr` exposes).
    pub fn alloc_wide(&mut self, s: &VbaString) -> Result<u64, MemoryError> {
        let mut bytes: Vec<u8> = s.as_units().iter().flat_map(|u| u.to_le_bytes()).collect();
        bytes.extend_from_slice(&[0, 0]);
        self.alloc_bytes(&bytes)
    }

    /// Releases a region by its base address.
    pub fn free(&mut self, addr: u64) -> Result<(), MemoryError> {
        match self.regions.remove(&addr) {
            Some(r) => {
                self.used -= r.bytes.len();
                Ok(())
            }
            None => Err(MemoryError::NotABase { addr }),
        }
    }

    /// Number of live bytes.
    pub fn used(&self) -> usize {
        self.used
    }

    fn locate(&self, addr: u64, len: usize) -> Result<(u64, usize), MemoryError> {
        if addr == 0 {
            return Err(MemoryError::NullPointer);
        }
        let (base, region) = self
            .regions
            .range(..=addr)
            .next_back()
            .ok_or(MemoryError::Unmapped { addr })?;
        let offset = (addr - base) as usize;
        if offset >= region.bytes.len() {
            return Err(MemoryError::Unmapped { addr });
        }
        if len > region.bytes.len() - offset {
            return Err(MemoryError::OutOfBounds { addr, len });
        }
        Ok((*base, offset))
    }

    /// Reads `len` bytes at `addr`.
    pub fn read(&self, addr: u64, len: usize) -> Result<&[u8], MemoryError> {
        let (base, off) = self.locate(addr, len)?;
        Ok(&self.regions[&base].bytes[off..off + len])
    }

    /// Writes `data` at `addr`.
    pub fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), MemoryError> {
        let (base, off) = self.locate(addr, data.len())?;
        let region = self.regions.get_mut(&base).expect("located region exists");
        region.bytes[off..off + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Bytes from `addr` to the end of its region.
    pub fn remaining(&self, addr: u64) -> Result<usize, MemoryError> {
        let (base, off) = self.locate(addr, 0)?;
        Ok(self.regions[&base].bytes.len() - off)
    }

    /// Fills `len` bytes at `addr` with `byte`.
    pub fn fill(&mut self, addr: u64, len: usize, byte: u8) -> Result<(), MemoryError> {
        let (base, off) = self.locate(addr, len)?;
        let region = self.regions.get_mut(&base).expect("located region exists");
        region.bytes[off..off + len].fill(byte);
        Ok(())
    }

    /// Copies `len` bytes with `memmove` semantics (overlap allowed). Both
    /// ranges are validated before anything is written.
    pub fn copy(&mut self, dst: u64, src: u64, len: usize) -> Result<(), MemoryError> {
        let data = self.read(src, len)?.to_vec();
        self.locate(dst, len)?;
        self.write(dst, &data)
    }

    /// Reads the NUL-terminated ANSI string at `addr` (without the NUL).
    pub fn read_cstr(&self, addr: u64) -> Result<Vec<u8>, MemoryError> {
        let rest = self.remaining(addr)?;
        let bytes = self.read(addr, rest)?;
        match bytes.iter().position(|b| *b == 0) {
            Some(n) => Ok(bytes[..n].to_vec()),
            None => Err(MemoryError::OutOfBounds {
                addr,
                len: rest + 1,
            }),
        }
    }

    /// Reads the NUL-terminated UTF-16 string at `addr` (without the NUL).
    pub fn read_wstr(&self, addr: u64) -> Result<Vec<u16>, MemoryError> {
        let rest = self.remaining(addr)?;
        let bytes = self.read(addr, rest)?;
        let mut out = Vec::new();
        for pair in bytes.as_chunks::<2>().0 {
            let u = u16::from_le_bytes([pair[0], pair[1]]);
            if u == 0 {
                return Ok(out);
            }
            out.push(u);
        }
        Err(MemoryError::OutOfBounds {
            addr,
            len: rest + 2,
        })
    }

    /// Reads a little-endian `u32`.
    pub fn read_u32(&self, addr: u64) -> Result<u32, MemoryError> {
        let b = self.read(addr, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Writes a little-endian `u32`.
    pub fn write_u32(&mut self, addr: u64, v: u32) -> Result<(), MemoryError> {
        self.write(addr, &v.to_le_bytes())
    }
}

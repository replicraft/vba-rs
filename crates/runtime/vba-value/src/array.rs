//! VBA arrays: multi-dimensional, arbitrary per-dimension lower bounds, fixed
//! or dynamic (MS-VBAL §5.4.3.x `ReDim`, `Erase`, §6.1.2.x `LBound`/`UBound`).
//!
//! This is the data-structure level only. Storage is column-major (the first
//! subscript varies fastest), as for OLE `SAFEARRAY`, which is why
//! `ReDim Preserve` of the last dimension is a resize of the flat storage.
//! Elements are stored as [`Variant`]s whatever the declared element type;
//! writes are coerced to the element type.

use crate::convert::convert_to;
use crate::variant::{VarType, Variant};
use vba_core::Charge;
use vba_core::{VbaError, VbaResult};

/// Maximum number of dimensions (an OLE `SAFEARRAY` limit of VBA).
pub const MAX_DIMENSIONS: usize = 60;
/// Upper limit on the total element count; larger requests are error 7.
pub const MAX_ELEMENTS: u64 = 1 << 28;

/// Inclusive bounds of one dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Lowest valid subscript.
    pub lower: i32,
    /// Highest valid subscript.
    pub upper: i32,
}

impl Bounds {
    /// Creates bounds.
    pub fn new(lower: i32, upper: i32) -> Self {
        Self { lower, upper }
    }

    /// Number of elements (`upper - lower + 1`), 0 if `upper < lower`.
    pub fn len(&self) -> u64 {
        if self.upper < self.lower {
            0
        } else {
            (i64::from(self.upper) - i64::from(self.lower) + 1) as u64
        }
    }

    /// `true` if the dimension holds no elements.
    pub fn is_empty(&self) -> bool {
        self.upper < self.lower
    }
}

/// A VBA array value.
///
/// The element storage is registered with the memory meter that is current
/// on the creating thread ([`vba_core::MemoryMeter::enter`]) and credited
/// when the array is dropped. Elements that own heap data (strings) are
/// registered by their own types.
#[derive(Debug, PartialEq)]
pub struct VbaArray {
    elem: VarType,
    fixed: bool,
    dims: Vec<Bounds>,
    data: Vec<Variant>,
    charge: Charge,
}

impl Clone for VbaArray {
    fn clone(&self) -> Self {
        Self::assemble(self.elem, self.fixed, self.dims.clone(), self.data.clone())
    }
}

/// Bytes of storage per array element, as charged to the memory meter.
pub const ELEMENT_BYTES: u64 = std::mem::size_of::<Variant>() as u64;

/// Bytes charged per array besides its elements (the header, the bounds and
/// the allocation), so that many small arrays are not free.
pub const ARRAY_OVERHEAD: u64 = 96;

fn total_len(dims: &[Bounds]) -> VbaResult<usize> {
    if dims.is_empty() || dims.len() > MAX_DIMENSIONS {
        return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
    }
    let mut n: u64 = 1;
    for b in dims {
        if b.upper < b.lower {
            return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
        }
        n = n.checked_mul(b.len()).ok_or(VbaError::OUT_OF_MEMORY)?;
        if n > MAX_ELEMENTS {
            return Err(VbaError::OUT_OF_MEMORY);
        }
    }
    Ok(n as usize)
}

impl VbaArray {
    fn assemble(elem: VarType, fixed: bool, dims: Vec<Bounds>, data: Vec<Variant>) -> Self {
        let charge = Charge::new(data.capacity() as u64 * ELEMENT_BYTES + ARRAY_OVERHEAD);
        Self {
            elem,
            fixed,
            dims,
            data,
            charge,
        }
    }

    /// Re-registers the storage after `data` changed its capacity.
    fn sync(&mut self) {
        self.charge
            .set(self.data.capacity() as u64 * ELEMENT_BYTES + ARRAY_OVERHEAD);
    }

    /// Creates a fixed-size array (`Dim a(1 To 3, 0 To 4) As T`). Bounds with
    /// `upper < lower`, zero or more than 60 dimensions are error 9.
    pub fn new_fixed(elem: VarType, dims: &[Bounds]) -> VbaResult<Self> {
        let n = total_len(dims)?;
        Ok(Self::assemble(
            elem,
            true,
            dims.to_vec(),
            vec![elem.default_value(); n],
        ))
    }

    /// Creates an unallocated dynamic array (`Dim a() As T`).
    pub fn new_dynamic(elem: VarType) -> Self {
        Self::assemble(elem, false, Vec::new(), Vec::new())
    }

    /// Creates an allocated array without elements, as returned by `Split("")`
    /// or `Array()`: one dimension whose upper bound is `lower - 1`. `ReDim`
    /// with a user-written upper bound below the lower bound stays error 9;
    /// only library functions produce this shape.
    pub fn new_empty(elem: VarType, lower: i32) -> Self {
        Self::assemble(
            elem,
            false,
            vec![Bounds::new(lower, lower.wrapping_sub(1))],
            Vec::new(),
        )
    }

    /// A one-dimensional dynamic `Byte` array indexed from 0 holding `bytes`
    /// (empty input gives the allocated empty array `0 To -1`).
    pub fn from_bytes(bytes: &[u8]) -> VbaResult<Self> {
        if bytes.is_empty() {
            return Ok(Self::new_empty(VarType::Byte, 0));
        }
        let upper = i32::try_from(bytes.len() - 1).map_err(|_| VbaError::OUT_OF_MEMORY)?;
        let mut a = Self::new_dynamic(VarType::Byte);
        a.redim(&[Bounds::new(0, upper)], false)?;
        for (slot, b) in a.data.iter_mut().zip(bytes) {
            *slot = Variant::Byte(*b);
        }
        Ok(a)
    }

    /// The elements of a `Byte` array in storage order; `None` when the
    /// element type is not `Byte`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        if self.elem != VarType::Byte {
            return None;
        }
        self.data
            .iter()
            .map(|v| match v {
                Variant::Byte(b) => Some(*b),
                _ => None,
            })
            .collect()
    }

    /// The declared element type.
    pub fn element_type(&self) -> VarType {
        self.elem
    }

    /// `true` for a fixed-size array.
    pub fn is_fixed(&self) -> bool {
        self.fixed
    }

    /// `false` for a dynamic array that has not been `ReDim`ed (or was erased).
    pub fn is_allocated(&self) -> bool {
        !self.dims.is_empty()
    }

    /// Number of dimensions (0 when unallocated).
    pub fn dimensions(&self) -> usize {
        self.dims.len()
    }

    /// Total number of elements.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// `true` if there are no elements (unallocated).
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The bounds of all dimensions.
    pub fn bounds(&self) -> &[Bounds] {
        &self.dims
    }

    fn dim(&self, dimension: u32) -> VbaResult<Bounds> {
        if dimension == 0 {
            return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
        }
        self.dims
            .get(dimension as usize - 1)
            .copied()
            .ok_or(VbaError::SUBSCRIPT_OUT_OF_RANGE)
    }

    /// `LBound(a, dimension)` (1-based dimension); error 9 when invalid or
    /// unallocated.
    pub fn lbound(&self, dimension: u32) -> VbaResult<i32> {
        self.dim(dimension).map(|b| b.lower)
    }

    /// `UBound(a, dimension)` (1-based dimension); error 9 when invalid or
    /// unallocated.
    pub fn ubound(&self, dimension: u32) -> VbaResult<i32> {
        self.dim(dimension).map(|b| b.upper)
    }

    fn offset(&self, idx: &[i32]) -> VbaResult<usize> {
        if idx.len() != self.dims.len() || self.dims.is_empty() {
            return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
        }
        let mut off: u64 = 0;
        let mut stride: u64 = 1;
        for (i, b) in idx.iter().zip(&self.dims) {
            if *i < b.lower || *i > b.upper {
                return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
            }
            off += (i64::from(*i) - i64::from(b.lower)) as u64 * stride;
            stride *= b.len();
        }
        Ok(off as usize)
    }

    /// Reads an element; wrong rank or subscript is error 9.
    pub fn get(&self, idx: &[i32]) -> VbaResult<&Variant> {
        Ok(&self.data[self.offset(idx)?])
    }

    /// Writes an element after coercing it to the element type (type
    /// mismatch/overflow errors as for assignment).
    pub fn set(&mut self, idx: &[i32], value: Variant) -> VbaResult<()> {
        let off = self.offset(idx)?;
        let v = if self.elem == VarType::Variant {
            value
        } else {
            convert_to(&value, self.elem)?
        };
        self.data[off] = v;
        Ok(())
    }

    /// `ReDim` / `ReDim Preserve`.
    ///
    /// * A fixed-size array cannot be redimensioned: error 10.
    /// * Without `preserve` all elements are reset to the default value.
    /// * With `preserve` the number of dimensions must be unchanged and only
    ///   the upper bound of the last dimension may change (error 9
    ///   otherwise); existing elements keep their subscripts, new elements
    ///   get the default value. Preserving an unallocated array behaves like a
    ///   plain `ReDim`.
    pub fn redim(&mut self, dims: &[Bounds], preserve: bool) -> VbaResult<()> {
        if self.fixed {
            return Err(VbaError::ARRAY_FIXED_OR_LOCKED);
        }
        let n = total_len(dims)?;
        if preserve && self.is_allocated() {
            if dims.len() != self.dims.len() {
                return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
            }
            let last = dims.len() - 1;
            if dims[..last] != self.dims[..last] || dims[last].lower != self.dims[last].lower {
                return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE);
            }
            if n > self.data.len() {
                self.data.reserve_exact(n - self.data.len());
            } else {
                self.data.truncate(n);
                self.data.shrink_to_fit();
            }
            self.data.resize(n, self.elem.default_value());
        } else {
            self.data = vec![self.elem.default_value(); n];
        }
        self.dims = dims.to_vec();
        self.sync();
        Ok(())
    }

    /// `Erase`: a fixed-size array has all elements reset to their default
    /// value; a dynamic array is deallocated.
    pub fn erase(&mut self) {
        if self.fixed {
            let d = self.elem.default_value();
            for e in &mut self.data {
                *e = d.clone();
            }
        } else {
            self.dims.clear();
            self.data = Vec::new();
            self.sync();
        }
    }

    /// Iterates over the elements in storage (column-major) order.
    pub fn iter(&self) -> impl Iterator<Item = &Variant> {
        self.data.iter()
    }

    /// Iterates mutably over the elements in storage order. Elements are not
    /// coerced: the caller keeps them valid for the element type (a VM uses
    /// this to materialize user-defined-type elements and to release or copy
    /// object references).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Variant> {
        self.data.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(l: i32, u: i32) -> Bounds {
        Bounds::new(l, u)
    }

    #[test]
    fn empty_allocated_array() {
        let mut a = VbaArray::new_empty(VarType::String, 0);
        assert!(a.is_allocated() && a.is_empty());
        assert_eq!((a.lbound(1), a.ubound(1)), (Ok(0), Ok(-1)));
        assert_eq!(a.get(&[0]), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(a.iter().count(), 0);
        a.redim(&[b(0, 1)], true).unwrap();
        assert_eq!(a.len(), 2);
        let mut a = VbaArray::new_empty(VarType::Variant, 1);
        assert_eq!((a.lbound(1), a.ubound(1)), (Ok(1), Ok(0)));
        assert_eq!(
            a.redim(&[b(1, 0)], false),
            Err(VbaError::SUBSCRIPT_OUT_OF_RANGE)
        );
    }

    #[test]
    fn iter_mut_visits_every_element_in_storage_order() {
        let mut a = VbaArray::new_fixed(VarType::Long, &[b(1, 2), b(1, 2)]).unwrap();
        for (i, e) in a.iter_mut().enumerate() {
            *e = Variant::Long(i as i32);
        }
        assert_eq!(a.get(&[2, 1]).unwrap(), &Variant::Long(1));
        assert_eq!(a.get(&[1, 2]).unwrap(), &Variant::Long(2));
    }

    #[test]
    fn fixed_array_basics() {
        let mut a = VbaArray::new_fixed(VarType::Integer, &[b(1, 3), b(0, 1)]).unwrap();
        assert_eq!(a.len(), 6);
        assert_eq!(a.dimensions(), 2);
        assert_eq!((a.lbound(1), a.ubound(1)), (Ok(1), Ok(3)));
        assert_eq!((a.lbound(2), a.ubound(2)), (Ok(0), Ok(1)));
        assert_eq!(a.lbound(3), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(a.lbound(0), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(a.get(&[2, 1]).unwrap(), &Variant::Integer(0));
        a.set(&[2, 1], Variant::Long(7)).unwrap();
        assert_eq!(a.get(&[2, 1]).unwrap(), &Variant::Integer(7));
        assert_eq!(a.get(&[0, 0]), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(a.get(&[1, 2]), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(a.get(&[1]), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(
            a.set(&[1, 0], Variant::Long(40_000)),
            Err(VbaError::OVERFLOW)
        );
        assert_eq!(
            a.set(&[1, 0], Variant::from("x")),
            Err(VbaError::TYPE_MISMATCH)
        );
    }

    #[test]
    fn negative_and_extreme_bounds() {
        let a = VbaArray::new_fixed(VarType::Byte, &[b(-5, -1)]).unwrap();
        assert_eq!(a.len(), 5);
        assert_eq!(a.get(&[-5]).unwrap(), &Variant::Byte(0));
        assert!(a.get(&[0]).is_err());
        assert_eq!(
            VbaArray::new_fixed(VarType::Byte, &[b(i32::MIN, i32::MAX)]),
            Err(VbaError::OUT_OF_MEMORY)
        );
        assert_eq!(
            VbaArray::new_fixed(VarType::Byte, &[b(2, 1)]),
            Err(VbaError::SUBSCRIPT_OUT_OF_RANGE)
        );
        assert_eq!(
            VbaArray::new_fixed(VarType::Byte, &[]),
            Err(VbaError::SUBSCRIPT_OUT_OF_RANGE)
        );
        let too_many = vec![b(0, 0); 61];
        assert!(VbaArray::new_fixed(VarType::Byte, &too_many).is_err());
        assert!(VbaArray::new_fixed(VarType::Byte, &too_many[..60]).is_ok());
    }

    #[test]
    fn variant_array_defaults_to_empty() {
        let a = VbaArray::new_fixed(VarType::Variant, &[b(0, 1)]).unwrap();
        assert_eq!(a.get(&[0]).unwrap(), &Variant::Empty);
        let s = VbaArray::new_fixed(VarType::String, &[b(0, 0)]).unwrap();
        assert_eq!(s.get(&[0]).unwrap(), &Variant::from(""));
    }

    #[test]
    fn dynamic_lifecycle() {
        let mut a = VbaArray::new_dynamic(VarType::Long);
        assert!(!a.is_allocated());
        assert_eq!(a.ubound(1), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        assert_eq!(a.get(&[0]), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        a.redim(&[b(0, 2)], false).unwrap();
        a.set(&[1], Variant::Long(5)).unwrap();
        // Preserve growing the last dimension.
        a.redim(&[b(0, 4)], true).unwrap();
        assert_eq!(a.get(&[1]).unwrap(), &Variant::Long(5));
        assert_eq!(a.get(&[4]).unwrap(), &Variant::Long(0));
        // Preserve shrinking.
        a.redim(&[b(0, 1)], true).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a.get(&[1]).unwrap(), &Variant::Long(5));
        // Without Preserve everything resets.
        a.redim(&[b(0, 1)], false).unwrap();
        assert_eq!(a.get(&[1]).unwrap(), &Variant::Long(0));
        // Erase deallocates.
        a.erase();
        assert!(!a.is_allocated());
        assert_eq!(a.len(), 0);
    }

    #[test]
    fn preserve_keeps_subscripts_in_multi_dimensional_arrays() {
        let mut a = VbaArray::new_dynamic(VarType::Variant);
        a.redim(&[b(1, 2), b(1, 2)], false).unwrap();
        for i in 1..=2 {
            for j in 1..=2 {
                a.set(&[i, j], Variant::Long(i * 10 + j)).unwrap();
            }
        }
        a.redim(&[b(1, 2), b(1, 3)], true).unwrap();
        for i in 1..=2 {
            for j in 1..=2 {
                assert_eq!(a.get(&[i, j]).unwrap(), &Variant::Long(i * 10 + j));
            }
            assert_eq!(a.get(&[i, 3]).unwrap(), &Variant::Empty);
        }
        // Changing a non-last dimension, the lower bound of the last, or the
        // rank with Preserve is error 9.
        for dims in [
            vec![b(1, 3), b(1, 3)],
            vec![b(1, 2), b(0, 3)],
            vec![b(1, 2)],
        ] {
            assert_eq!(a.redim(&dims, true), Err(VbaError::SUBSCRIPT_OUT_OF_RANGE));
        }
        // The failed attempts left the array untouched.
        assert_eq!(a.get(&[2, 2]).unwrap(), &Variant::Long(22));
        // Without Preserve any shape is allowed.
        a.redim(&[b(0, 0)], false).unwrap();
        assert_eq!(a.dimensions(), 1);
    }

    #[test]
    fn fixed_array_cannot_be_redimmed_and_erase_resets() {
        let mut a = VbaArray::new_fixed(VarType::Double, &[b(0, 1)]).unwrap();
        a.set(&[0], Variant::Double(1.5)).unwrap();
        assert_eq!(
            a.redim(&[b(0, 3)], false),
            Err(VbaError::ARRAY_FIXED_OR_LOCKED)
        );
        a.erase();
        assert!(a.is_allocated());
        assert_eq!(a.get(&[0]).unwrap(), &Variant::Double(0.0));
    }

    #[test]
    fn column_major_layout() {
        let mut a = VbaArray::new_fixed(VarType::Long, &[b(0, 1), b(0, 2)]).unwrap();
        let mut n = 0;
        for j in 0..=2 {
            for i in 0..=1 {
                a.set(&[i, j], Variant::Long(n)).unwrap();
                n += 1;
            }
        }
        let flat: Vec<_> = a.iter().cloned().collect();
        assert_eq!(flat, (0..6).map(Variant::Long).collect::<Vec<_>>());
    }
}

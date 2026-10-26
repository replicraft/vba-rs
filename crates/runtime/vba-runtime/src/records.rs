//! Binary layout of `Get` and `Put` records.
//!
//! Scalars are little-endian (`Boolean` is -1/0 in two bytes, `Currency` the
//! scaled `Int64`, `Date` an IEEE double). A `String * n` is `n` ANSI bytes
//! padded with spaces. A variable-length `String` is its bytes in binary
//! mode and a two-byte length followed by its bytes in random mode. A
//! `Variant` is a two-byte `VarType` followed by its data (**doc**: layout of
//! VBA's own files; unverified). User-defined types and fixed arrays are the
//! concatenation of their members.

use vba_ffi::{Codepage, Unmappable};
use vba_value::{Currency, Date, VarType, Variant, VbaError, VbaString, convert};
use vba_vm::{RecordValue, Shape};

use crate::error::FsError;

fn encode_text(s: &VbaString) -> Vec<u8> {
    Codepage::Windows1252
        .encode(s.as_units(), Unmappable::Replace(b'?'), false)
        .map(|e| e.bytes)
        .unwrap_or_default()
}

fn decode_text(b: &[u8]) -> VbaString {
    VbaString::from_units(Codepage::Windows1252.decode(b, false).unwrap_or_default())
}

fn type_mismatch() -> FsError {
    FsError::Vba(VbaError::TYPE_MISMATCH.number())
}

/// Appends the encoding of `value` laid out as `shape`.
pub fn encode(
    shape: &Shape,
    value: &RecordValue,
    random: bool,
    out: &mut Vec<u8>,
) -> Result<(), FsError> {
    match (shape, value) {
        (Shape::Scalar(vt), RecordValue::Scalar(v)) => encode_scalar(*vt, v, out),
        (Shape::VarString { .. }, RecordValue::Scalar(v)) => {
            let s = convert::to_vba_string(v).map_err(|e| FsError::Vba(e.number()))?;
            let bytes = encode_text(&s);
            if random {
                let n = u16::try_from(bytes.len()).map_err(|_| FsError::Vba(59))?;
                out.extend_from_slice(&n.to_le_bytes());
            }
            out.extend_from_slice(&bytes);
            Ok(())
        }
        (Shape::FixedString(n), RecordValue::Scalar(v)) => {
            let s = convert::to_vba_string(v).map_err(|e| FsError::Vba(e.number()))?;
            let mut bytes = encode_text(&s);
            bytes.resize(*n as usize, b' ');
            out.extend_from_slice(&bytes);
            Ok(())
        }
        (Shape::Udt(shapes), RecordValue::Fields(vals)) if shapes.len() == vals.len() => {
            for (s, v) in shapes.iter().zip(vals) {
                encode(s, v, random, out)?;
            }
            Ok(())
        }
        (Shape::Array { elem, count }, RecordValue::Fields(vals)) if vals.len() == *count => {
            for v in vals {
                encode(elem, v, random, out)?;
            }
            Ok(())
        }
        _ => Err(type_mismatch()),
    }
}

fn encode_scalar(vt: VarType, v: &Variant, out: &mut Vec<u8>) -> Result<(), FsError> {
    let c = |t: VarType| convert::convert_to(v, t).map_err(|e| FsError::Vba(e.number()));
    match vt {
        VarType::Byte => {
            if let Variant::Byte(b) = c(VarType::Byte)? {
                out.push(b);
            }
        }
        VarType::Boolean => {
            let b = convert::to_boolean(v).map_err(|e| FsError::Vba(e.number()))?;
            out.extend_from_slice(&(if b { -1i16 } else { 0 }).to_le_bytes());
        }
        VarType::Integer => out.extend_from_slice(
            &convert::to_integer(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .to_le_bytes(),
        ),
        VarType::Long => out.extend_from_slice(
            &convert::to_long(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .to_le_bytes(),
        ),
        VarType::LongLong => out.extend_from_slice(
            &convert::to_longlong(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .to_le_bytes(),
        ),
        VarType::Single => out.extend_from_slice(
            &convert::to_single(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .to_le_bytes(),
        ),
        VarType::Double => out.extend_from_slice(
            &convert::to_double(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .to_le_bytes(),
        ),
        VarType::Currency => out.extend_from_slice(
            &convert::to_currency(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .scaled()
                .to_le_bytes(),
        ),
        VarType::Date => out.extend_from_slice(
            &convert::to_date(v)
                .map_err(|e| FsError::Vba(e.number()))?
                .serial()
                .to_le_bytes(),
        ),
        VarType::Variant | VarType::Empty => {
            let code = v.vartype().code() as u16;
            match v {
                Variant::Empty | Variant::Null => out.extend_from_slice(&code.to_le_bytes()),
                Variant::String(_) => {
                    out.extend_from_slice(&code.to_le_bytes());
                    encode(
                        &Shape::VarString { current_len: 0 },
                        &RecordValue::Scalar(v.clone()),
                        true,
                        out,
                    )?;
                }
                Variant::Array(_) | Variant::Object(_) | Variant::UserDefined(_) => {
                    // MS-VBAL 5.4.5.11: an object or user-defined type has no
                    // descriptor. The layout of an array in a Variant is not
                    // documented.
                    return Err(FsError::Unsupported(
                        "Put of a Variant holding an array, object or user-defined type".into(),
                    ));
                }
                other => {
                    out.extend_from_slice(&code.to_le_bytes());
                    encode_scalar(other.vartype(), other, out)?;
                }
            }
        }
        VarType::Decimal => {
            let d = convert::to_decimal(v).map_err(|e| FsError::Vba(e.number()))?;
            // MS-OAUT 2.2.26: reserved word, scale, sign, high 32 bits and
            // low 64 bits of the 96-bit mantissa.
            let m = d.mantissa();
            out.extend_from_slice(&[0, 0, d.scale(), if d.is_negative() { 0x80 } else { 0 }]);
            out.extend_from_slice(&((m >> 64) as u32).to_le_bytes());
            out.extend_from_slice(&(m as u64).to_le_bytes());
        }
        VarType::Error => {
            // MS-VBAL 5.4.5.11: the error code as an HRESULT.
            let n = match v {
                Variant::Error(e) => e.number(),
                _ => return Err(type_mismatch()),
            };
            out.extend_from_slice(&(0x800A_0000u32 | (n as u32 & 0xFFFF)).to_le_bytes());
        }
        _ => return Err(type_mismatch()),
    }
    Ok(())
}

/// A cursor over the bytes of a record.
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Starts reading at `pos`.
    pub fn new(data: &'a [u8], pos: usize) -> Reader<'a> {
        Reader { data, pos }
    }

    /// The current offset.
    pub fn pos(&self) -> usize {
        self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], FsError> {
        let end = self.pos.checked_add(n).ok_or(FsError::Vba(62))?;
        if end > self.data.len() {
            return Err(FsError::Vba(62));
        }
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }
}

/// Reads a value laid out as `shape`.
pub fn decode(shape: &Shape, r: &mut Reader<'_>, random: bool) -> Result<RecordValue, FsError> {
    Ok(match shape {
        Shape::Scalar(vt) => RecordValue::Scalar(decode_scalar(*vt, r)?),
        Shape::VarString { current_len } => {
            let n = if random {
                usize::from(u16::from_le_bytes(
                    r.take(2)?.try_into().expect("two bytes"),
                ))
            } else {
                *current_len
            };
            RecordValue::Scalar(Variant::String(decode_text(r.take(n)?)))
        }
        Shape::FixedString(n) => {
            RecordValue::Scalar(Variant::String(decode_text(r.take(*n as usize)?)))
        }
        Shape::Udt(shapes) => RecordValue::Fields(
            shapes
                .iter()
                .map(|s| decode(s, r, random))
                .collect::<Result<_, _>>()?,
        ),
        Shape::Array { elem, count } => RecordValue::Fields(
            (0..*count)
                .map(|_| decode(elem, r, random))
                .collect::<Result<_, _>>()?,
        ),
    })
}

fn decode_scalar(vt: VarType, r: &mut Reader<'_>) -> Result<Variant, FsError> {
    let arr =
        |r: &mut Reader<'_>, n: usize| -> Result<Vec<u8>, FsError> { Ok(r.take(n)?.to_vec()) };
    Ok(match vt {
        VarType::Byte => Variant::Byte(r.take(1)?[0]),
        VarType::Boolean => {
            Variant::Boolean(i16::from_le_bytes(arr(r, 2)?.try_into().expect("2")) != 0)
        }
        VarType::Integer => Variant::Integer(i16::from_le_bytes(arr(r, 2)?.try_into().expect("2"))),
        VarType::Long => Variant::Long(i32::from_le_bytes(arr(r, 4)?.try_into().expect("4"))),
        VarType::LongLong => {
            Variant::LongLong(i64::from_le_bytes(arr(r, 8)?.try_into().expect("8")))
        }
        VarType::Single => Variant::Single(f32::from_le_bytes(arr(r, 4)?.try_into().expect("4"))),
        VarType::Double => Variant::Double(f64::from_le_bytes(arr(r, 8)?.try_into().expect("8"))),
        VarType::Currency => Variant::Currency(Currency::from_scaled(i64::from_le_bytes(
            arr(r, 8)?.try_into().expect("8"),
        ))),
        VarType::Date => {
            let f = f64::from_le_bytes(arr(r, 8)?.try_into().expect("8"));
            Variant::Date(Date::from_serial(f).map_err(|e| FsError::Vba(e.number()))?)
        }
        VarType::Variant | VarType::Empty => {
            let code = u16::from_le_bytes(arr(r, 2)?.try_into().expect("2"));
            match VarType::from_code(i32::from(code)) {
                Some(VarType::Empty) => Variant::Empty,
                Some(VarType::Null) => Variant::Null,
                Some(VarType::String) => {
                    match decode(&Shape::VarString { current_len: 0 }, r, true)? {
                        RecordValue::Scalar(v) => v,
                        RecordValue::Fields(_) => return Err(type_mismatch()),
                    }
                }
                Some(
                    t @ (VarType::Byte
                    | VarType::Boolean
                    | VarType::Integer
                    | VarType::Long
                    | VarType::LongLong
                    | VarType::Single
                    | VarType::Double
                    | VarType::Currency
                    | VarType::Date),
                ) => decode_scalar(t, r)?,
                Some(t @ (VarType::Error | VarType::Decimal)) => decode_scalar(t, r)?,
                // Objects and user-defined types are never written by `Put`, and
                // other codes are not Variant tags: the file is not what was written.
                _ => return Err(FsError::Vba(321)),
            }
        }
        VarType::Decimal => {
            let b = r.take(16)?;
            let hi = u32::from_le_bytes(b[4..8].try_into().expect("4"));
            let lo = u64::from_le_bytes(b[8..16].try_into().expect("8"));
            let m = (u128::from(hi) << 64) | u128::from(lo);
            // A scale above 28 is not a Decimal that VBA writes.
            Variant::Decimal(
                vba_value::Decimal::from_parts(m, b[2], b[3] & 0x80 != 0)
                    .map_err(|_| FsError::Vba(321))?,
            )
        }
        VarType::Error => {
            let h = u32::from_le_bytes(arr(r, 4)?.try_into().expect("4"));
            let n = if h >> 16 == 0x800A { h & 0xFFFF } else { h };
            Variant::Error(VbaError::new(n as i32))
        }
        _ => return Err(type_mismatch()),
    })
}

/// The encoded size of a shape that has a fixed size (`None` for strings of
/// unknown length and Variants).
#[cfg(test)]
fn fixed_size(shape: &Shape) -> Option<usize> {
    Some(match shape {
        Shape::Scalar(vt) => match vt {
            VarType::Byte => 1,
            VarType::Boolean | VarType::Integer => 2,
            VarType::Long | VarType::Single => 4,
            VarType::LongLong | VarType::Double | VarType::Currency | VarType::Date => 8,
            _ => return None,
        },
        Shape::FixedString(n) => *n as usize,
        Shape::VarString { .. } => return None,
        Shape::Udt(s) => s.iter().map(fixed_size).sum::<Option<usize>>()?,
        Shape::Array { elem, count } => fixed_size(elem)? * count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(shape: &Shape, value: &RecordValue, random: bool) -> RecordValue {
        let mut buf = Vec::new();
        encode(shape, value, random, &mut buf).unwrap();
        let mut r = Reader::new(&buf, 0);
        let out = decode(shape, &mut r, random).unwrap();
        assert_eq!(r.pos(), buf.len());
        out
    }

    #[test]
    fn scalars_and_composites_round_trip() {
        let shape = Shape::Udt(vec![
            Shape::Scalar(VarType::Long),
            Shape::FixedString(4),
            Shape::Scalar(VarType::Double),
            Shape::Array {
                elem: Box::new(Shape::Scalar(VarType::Integer)),
                count: 2,
            },
        ]);
        let value = RecordValue::Fields(vec![
            RecordValue::Scalar(Variant::Long(-7)),
            RecordValue::Scalar(Variant::from_text("ab")),
            RecordValue::Scalar(Variant::Double(1.5)),
            RecordValue::Fields(vec![
                RecordValue::Scalar(Variant::Integer(1)),
                RecordValue::Scalar(Variant::Integer(2)),
            ]),
        ]);
        let back = roundtrip(&shape, &value, false);
        let RecordValue::Fields(f) = back else {
            panic!()
        };
        assert_eq!(f[0], RecordValue::Scalar(Variant::Long(-7)));
        assert_eq!(f[1], RecordValue::Scalar(Variant::from_text("ab  ")));
        assert_eq!(fixed_size(&shape), Some(4 + 4 + 8 + 4));
    }

    #[test]
    fn variable_strings_have_a_descriptor_only_in_random_mode() {
        let shape = Shape::VarString { current_len: 3 };
        let v = RecordValue::Scalar(Variant::from_text("abc"));
        let mut b = Vec::new();
        encode(&shape, &v, false, &mut b).unwrap();
        assert_eq!(b, b"abc");
        let mut b = Vec::new();
        encode(&shape, &v, true, &mut b).unwrap();
        assert_eq!(b, [3, 0, b'a', b'b', b'c']);
        assert_eq!(roundtrip(&shape, &v, true), v);
    }

    #[test]
    fn variants_carry_their_type() {
        let shape = Shape::Scalar(VarType::Variant);
        for v in [
            Variant::Empty,
            Variant::Null,
            Variant::Integer(5),
            Variant::from_text("hi"),
            Variant::Double(2.5),
        ] {
            let rv = RecordValue::Scalar(v);
            assert_eq!(roundtrip(&shape, &rv, true), rv);
        }
    }

    #[test]
    fn reading_past_the_end_is_error_62() {
        let mut r = Reader::new(&[1, 2], 0);
        assert_eq!(
            decode(&Shape::Scalar(VarType::Long), &mut r, false),
            Err(FsError::Vba(62))
        );
    }
}

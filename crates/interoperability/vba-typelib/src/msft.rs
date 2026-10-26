//! Reader for the MSFT-format type library (see `docs/formats/typelib.md` for the
//! layout, the verification and the open unknowns).

use std::collections::HashMap;

use encoding_rs::WINDOWS_1252;

use crate::error::{Result, TypelibError};
use crate::model::{
    CallConv, Const, ConstValue, FuncFlags, FuncKind, Function, Guid, ImplFlags, ImplType, Import,
    InvokeKind, LibFlags, Param, ParamFlags, TypeDesc, TypeFlags, TypeInfo, TypeKind, TypeLib,
    TypeRef, VarFlags, VarKind, Variable, Version, vt,
};

/// Size limits applied while reading, so hostile input cannot make the
/// reader allocate or loop without bound. All counts are checked against
/// the data actually present as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum number of types.
    pub max_types: usize,
    /// Maximum number of members (functions plus variables) over all types.
    pub max_members: usize,
    /// Maximum number of parameters over all functions.
    pub max_params: usize,
    /// Maximum nesting of a type description.
    pub max_type_depth: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_types: 1 << 17,
            max_members: 1 << 20,
            max_params: 1 << 21,
            max_type_depth: 32,
        }
    }
}

const MAGIC: &[u8; 4] = b"MSFT";
const TYPEINFO_SIZE: usize = 0x64;
const SEGMENTS: usize = 15;
const SEG_TYPEINFO: usize = 0;
const SEG_IMPINFO: usize = 1;
const SEG_IMPFILES: usize = 2;
const SEG_REFTAB: usize = 3;
const SEG_GUID: usize = 5;
const SEG_NAMES: usize = 7;
const SEG_STRINGS: usize = 8;
const SEG_TYPDESC: usize = 9;
const SEG_ARRAYDESC: usize = 10;
const SEG_CUSTDATA: usize = 11;
const NONE: u32 = 0xffff_ffff;

#[derive(Clone, Copy)]
struct Seg {
    off: usize,
    len: usize,
}

struct Reader<'a> {
    d: &'a [u8],
    seg: [Seg; SEGMENTS],
    limits: Limits,
    /// Offsets of the type records inside segment 0, by type index.
    ti_offsets: Vec<u32>,
    by_offset: HashMap<u32, usize>,
    members_seen: usize,
    params_seen: usize,
    warnings: Vec<String>,
}

pub(crate) fn oob(what: &'static str, offset: usize) -> TypelibError {
    TypelibError::OutOfRange { what, offset }
}

pub(crate) fn u16_at(d: &[u8], what: &'static str, o: usize) -> Result<u16> {
    d.get(o..o.wrapping_add(2))
        .filter(|_| o.checked_add(2).is_some())
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or(oob(what, o))
}

pub(crate) fn u32_at(d: &[u8], what: &'static str, o: usize) -> Result<u32> {
    d.get(o..o.wrapping_add(4))
        .filter(|_| o.checked_add(4).is_some())
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(oob(what, o))
}

pub(crate) fn decode(bytes: &[u8]) -> String {
    WINDOWS_1252
        .decode_without_bom_handling(bytes)
        .0
        .into_owned()
}

/// Parses an MSFT type library.
///
/// # Errors
/// [`TypelibError::NotATypelib`] when the magic is wrong,
/// [`TypelibError::UnsupportedFormat`] for SLTG data or unknown versions, and the
/// structural errors when the data is truncated or inconsistent.
pub fn parse(data: &[u8], limits: Limits) -> Result<TypeLib> {
    match data.get(..4) {
        Some(m) if m == MAGIC => {}
        Some(b"SLTG") => {
            return Err(TypelibError::UnsupportedFormat(
                "SLTG data given to the MSFT reader (use sltg::parse or parse_any)",
            ));
        }
        _ => return Err(TypelibError::NotATypelib("missing MSFT signature")),
    }
    let version = u32_at(data, "header", 4)?;
    if version != 0x0001_0002 {
        return Err(TypelibError::UnsupportedFormat(
            "MSFT format version is not 1.2",
        ));
    }
    let posguid = u32_at(data, "header", 8)?;
    let lcid = u32_at(data, "header", 0x0c)?;
    let var_flags = u32_at(data, "header", 0x14)?;
    let lib_version = u32_at(data, "header", 0x18)?;
    let lib_flags = u32_at(data, "header", 0x1c)?;
    let ntypes = u32_at(data, "header", 0x20)? as usize;
    let help_string = u32_at(data, "header", 0x24)?;
    let help_context = u32_at(data, "header", 0x28)?;
    // An optional extra header word is present when bit 0x100 of the
    // variable flags is set (no sample available: unverified).
    let shift = if var_flags & 0x100 != 0 { 4 } else { 0 };
    if ntypes > limits.max_types {
        return Err(TypelibError::LimitExceeded("too many types"));
    }
    let arr_off = 0x54 + shift;
    let arr_len = ntypes
        .checked_mul(4)
        .ok_or(oob("typeinfo offsets", arr_off))?;
    let sd_off = arr_off
        .checked_add(arr_len)
        .ok_or(oob("segment directory", arr_off))?;
    let mut seg = [Seg { off: 0, len: 0 }; SEGMENTS];
    for (i, s) in seg.iter_mut().enumerate() {
        let e = sd_off + i * 16;
        let off = u32_at(data, "segment directory", e)?;
        let len = u32_at(data, "segment directory", e + 4)? as usize;
        if off == NONE {
            if len != 0 {
                return Err(TypelibError::Malformed {
                    what: "segment directory",
                    detail: "absent segment with a length",
                });
            }
            continue;
        }
        let off = off as usize;
        if off.checked_add(len).is_none_or(|end| end > data.len()) {
            return Err(oob("segment", off));
        }
        *s = Seg { off, len };
    }
    let ti_seg = seg[SEG_TYPEINFO];
    if ntypes
        .checked_mul(TYPEINFO_SIZE)
        .is_none_or(|n| n > ti_seg.len)
    {
        return Err(TypelibError::Malformed {
            what: "typeinfo table",
            detail: "shorter than the type count",
        });
    }
    let mut ti_offsets = Vec::with_capacity(ntypes);
    let mut by_offset = HashMap::with_capacity(ntypes);
    for i in 0..ntypes {
        let o = u32_at(data, "typeinfo offsets", arr_off + 4 * i)?;
        if (o as usize)
            .checked_add(TYPEINFO_SIZE)
            .is_none_or(|e| e > ti_seg.len)
        {
            return Err(oob("typeinfo record", o as usize));
        }
        by_offset.entry(o).or_insert(i);
        ti_offsets.push(o);
    }
    let mut r = Reader {
        d: data,
        seg,
        limits,
        ti_offsets,
        by_offset,
        members_seen: 0,
        params_seen: 0,
        warnings: Vec::new(),
    };

    let guid = r.guid(posguid)?.unwrap_or(Guid([0; 16]));
    let name = r.library_name()?;
    let doc = r.opt_string(help_string)?;
    let imports = r.imports()?;
    let mut types = Vec::with_capacity(ntypes);
    for i in 0..ntypes {
        types.push(r.type_info(i, &imports)?);
    }
    Ok(TypeLib {
        name,
        guid,
        lcid,
        version: split_version(lib_version),
        flags: LibFlags::from_bits_retain(lib_flags),
        sys_kind: Some((var_flags & 0xf) as u8).filter(|k| *k <= 3),
        doc,
        help_context,
        imports,
        types,
        warnings: r.warnings,
    })
}

fn split_version(v: u32) -> Version {
    Version {
        major: (v & 0xffff) as u16,
        minor: (v >> 16) as u16,
    }
}

impl Reader<'_> {
    fn in_seg(&self, s: usize, what: &'static str, off: usize, len: usize) -> Result<usize> {
        let seg = self.seg[s];
        match off.checked_add(len) {
            Some(end) if end <= seg.len => Ok(seg.off + off),
            _ => Err(oob(what, off)),
        }
    }

    fn guid(&self, off: u32) -> Result<Option<Guid>> {
        if off == NONE {
            return Ok(None);
        }
        let p = self.in_seg(SEG_GUID, "GUID table", off as usize, 16)?;
        let mut g = [0u8; 16];
        g.copy_from_slice(&self.d[p..p + 16]);
        Ok(Some(Guid(g)))
    }

    /// Name table entry at `off`: returns (hreftype word, name).
    fn name_entry(&self, off: usize) -> Result<(u32, String, usize)> {
        let p = self.in_seg(SEG_NAMES, "name table", off, 12)?;
        let href = u32_at(self.d, "name entry", p)?;
        let lh = u32_at(self.d, "name entry", p + 8)?;
        let len = (lh & 0xff) as usize;
        let b = self.in_seg(SEG_NAMES, "name table", off + 12, len)?;
        let stride = (12 + len + 3) & !3;
        Ok((href, decode(&self.d[b..b + len]), stride))
    }

    fn name(&self, off: u32) -> Result<String> {
        Ok(self.name_entry(off as usize)?.1)
    }

    fn opt_name(&self, off: u32) -> Result<Option<String>> {
        if off == NONE {
            Ok(None)
        } else {
            self.name(off).map(Some)
        }
    }

    /// The library name is the first name entry flagged with hreftype -1.
    fn library_name(&self) -> Result<String> {
        let mut off = 0usize;
        for _ in 0..1024 {
            if off >= self.seg[SEG_NAMES].len {
                break;
            }
            let (href, name, stride) = self.name_entry(off)?;
            if href == NONE {
                return Ok(name);
            }
            off += stride;
        }
        Ok(String::new())
    }

    fn string(&self, off: u32) -> Result<String> {
        let o = off as usize;
        let p = self.in_seg(SEG_STRINGS, "string table", o, 2)?;
        let len = u16_at(self.d, "string", p)? as usize;
        let b = self.in_seg(SEG_STRINGS, "string table", o + 2, len)?;
        Ok(decode(&self.d[b..b + len]))
    }

    fn opt_string(&self, off: u32) -> Result<Option<String>> {
        if off == NONE {
            Ok(None)
        } else {
            self.string(off).map(Some)
        }
    }

    fn imports(&mut self) -> Result<Vec<Import>> {
        let seg = self.seg[SEG_IMPINFO];
        if !seg.len.is_multiple_of(12) {
            return Err(TypelibError::Malformed {
                what: "import table",
                detail: "length is not a multiple of 12",
            });
        }
        let mut out = Vec::new();
        for i in 0..seg.len / 12 {
            let p = seg.off + i * 12;
            let w0 = u32_at(self.d, "import", p)?;
            let file_off = u32_at(self.d, "import", p + 4)?;
            let guid_off = u32_at(self.d, "import", p + 8)?;
            let (library_guid, lcid, version, file) = self.import_file(file_off)?;
            out.push(Import {
                library_guid,
                lcid,
                version,
                file,
                type_guid: self.guid(guid_off)?,
                kind: kind_from((w0 >> 24) as u8 & 0xf),
            });
        }
        Ok(out)
    }

    fn import_file(&self, off: u32) -> Result<(Option<Guid>, u32, Version, String)> {
        let p = self.in_seg(SEG_IMPFILES, "import files", off as usize, 14)?;
        let guid = self.guid(u32_at(self.d, "import file", p)?)?;
        let lcid = u32_at(self.d, "import file", p + 4)?;
        let ver = u32_at(self.d, "import file", p + 8)?;
        let tag = u16_at(self.d, "import file", p + 12)? as usize;
        let len = tag >> 2;
        let b = self.in_seg(SEG_IMPFILES, "import files", off as usize + 14, len)?;
        Ok((guid, lcid, split_version(ver), decode(&self.d[b..b + len])))
    }

    fn href(&self, h: u32, imports: &[Import]) -> Result<TypeRef> {
        if h & 1 == 1 {
            let o = h & !1;
            if o.is_multiple_of(12) && ((o / 12) as usize) < imports.len() {
                return Ok(TypeRef::Imported((o / 12) as usize));
            }
        } else if let Some(&i) = self.by_offset.get(&h) {
            return Ok(TypeRef::Local(i));
        }
        Err(TypelibError::Malformed {
            what: "type reference",
            detail: "does not designate a type",
        })
    }

    fn datatype(&self, v: u32, imports: &[Import], depth: usize) -> Result<TypeDesc> {
        if depth > self.limits.max_type_depth {
            return Err(TypelibError::LimitExceeded("type description too deep"));
        }
        if v & 0x8000_0000 != 0 {
            return Ok(TypeDesc::Prim((v & 0xffff) as u16));
        }
        let p = self.in_seg(SEG_TYPDESC, "type description", v as usize, 8)?;
        let kind = u16_at(self.d, "type description", p)?;
        let data = u32_at(self.d, "type description", p + 4)?;
        Ok(match kind {
            vt::PTR => TypeDesc::Ptr(Box::new(self.datatype(data, imports, depth + 1)?)),
            vt::SAFEARRAY => {
                TypeDesc::SafeArray(Box::new(self.datatype(data, imports, depth + 1)?))
            }
            vt::CARRAY => self.array(data, imports, depth)?,
            vt::USERDEFINED => TypeDesc::User(self.href(data, imports)?),
            other => TypeDesc::Prim(other),
        })
    }

    fn array(&self, off: u32, imports: &[Import], depth: usize) -> Result<TypeDesc> {
        let p = self.in_seg(SEG_ARRAYDESC, "array description", off as usize, 8)?;
        let elem = u32_at(self.d, "array description", p)?;
        let dims = u16_at(self.d, "array description", p + 4)? as usize;
        let b = self.in_seg(
            SEG_ARRAYDESC,
            "array description",
            off as usize + 8,
            dims * 8,
        )?;
        let mut bounds = Vec::with_capacity(dims);
        for i in 0..dims {
            let count = u32_at(self.d, "array bounds", b + i * 8)?;
            let lb = u32_at(self.d, "array bounds", b + i * 8 + 4)? as i32;
            bounds.push((count, lb));
        }
        Ok(TypeDesc::CArray {
            elem: Box::new(self.datatype(elem, imports, depth + 1)?),
            bounds,
        })
    }

    /// A constant or default value: an immediate (bit 31 set) or an offset
    /// into the custom-data segment.
    fn constant(&self, v: u32) -> Result<Const> {
        if v & 0x8000_0000 != 0 {
            let tag = ((v >> 26) & 0x1f) as u16;
            let raw = v & 0x03ff_ffff;
            let signed = ((raw << 6) as i32) >> 6;
            let value = match tag {
                vt::I1 | vt::I2 | vt::I4 | vt::INT | vt::ERROR => ConstValue::Int(signed.into()),
                vt::UI1 | vt::UI2 | vt::UI4 | vt::UINT => ConstValue::UInt(raw.into()),
                vt::BOOL => ConstValue::Bool(raw != 0),
                vt::R4 | vt::R8 => ConstValue::Real(f64::from(signed)),
                vt::DISPATCH | vt::UNKNOWN if raw == 0 => ConstValue::Nothing,
                _ => ConstValue::Unsupported,
            };
            return Ok(Const { vt: tag, value });
        }
        let o = v as usize;
        let p = self.in_seg(SEG_CUSTDATA, "custom data", o, 2)?;
        let tag = u16_at(self.d, "custom data", p)?;
        let at = |n: usize| self.in_seg(SEG_CUSTDATA, "custom data", o + 2, n);
        let value = match tag {
            vt::I1 => ConstValue::Int(i64::from(self.d[at(1)?] as i8)),
            vt::UI1 => ConstValue::UInt(self.d[at(1)?].into()),
            vt::I2 => ConstValue::Int(u16_at(self.d, "custom data", at(2)?)? as i16 as i64),
            vt::UI2 => ConstValue::UInt(u16_at(self.d, "custom data", at(2)?)?.into()),
            vt::BOOL => ConstValue::Bool(u16_at(self.d, "custom data", at(2)?)? != 0),
            vt::I4 | vt::INT | vt::ERROR => {
                ConstValue::Int(u32_at(self.d, "custom data", at(4)?)? as i32 as i64)
            }
            vt::UI4 | vt::UINT => ConstValue::UInt(u32_at(self.d, "custom data", at(4)?)?.into()),
            vt::R4 => ConstValue::Real(f64::from(f32::from_bits(u32_at(
                self.d,
                "custom data",
                at(4)?,
            )?))),
            vt::R8 | vt::DATE => ConstValue::Real(f64::from_bits(self.u64_at(at(8)?)?)),
            vt::CY => ConstValue::Currency(self.u64_at(at(8)?)? as i64),
            vt::I8 => ConstValue::Int(self.u64_at(at(8)?)? as i64),
            vt::UI8 => ConstValue::UInt(self.u64_at(at(8)?)?),
            vt::BSTR => {
                let len = u32_at(self.d, "custom data", at(4)?)? as usize;
                let b = self.in_seg(SEG_CUSTDATA, "custom data", o + 6, len)?;
                ConstValue::Str(decode(&self.d[b..b + len]))
            }
            _ => ConstValue::Unsupported,
        };
        Ok(Const { vt: tag, value })
    }

    fn u64_at(&self, p: usize) -> Result<u64> {
        let lo = u32_at(self.d, "custom data", p)?;
        let hi = u32_at(self.d, "custom data", p + 4)?;
        Ok(u64::from(lo) | (u64::from(hi) << 32))
    }

    fn type_info(&mut self, index: usize, imports: &[Import]) -> Result<TypeInfo> {
        let rec = self.seg[SEG_TYPEINFO].off + self.ti_offsets[index] as usize;
        let w = |k: usize| u32_at(self.d, "typeinfo", rec + 4 * k);
        let w0 = w(0)?;
        let kind = kind_from((w0 & 0xf) as u8).ok_or(TypelibError::Malformed {
            what: "typeinfo",
            detail: "unknown type kind",
        })?;
        let mem_off = w(1)?;
        let counts = w(6)?;
        let (n_funcs, n_vars) = ((counts & 0xffff) as usize, (counts >> 16) as usize);
        let guid = self.guid(w(11)?)?;
        let flags = w(12)?;
        let name = self.name(w(13)?)?;
        let version = split_version(w(14)?);
        let doc = self.opt_string(w(15)?)?;
        let help_context = w(17)?;
        let impl_word = w(19)?;
        let n_impl = (impl_word & 0xffff) as usize;
        let instance_size = w(20)?;
        let dt1 = w(21)?;

        let mut ti = TypeInfo {
            name,
            kind,
            guid,
            flags: TypeFlags::from_bits_retain(flags),
            version,
            doc,
            help_context,
            alignment: ((w0 >> 11) & 0x1f) as u8,
            instance_size,
            vtable_size: (impl_word >> 16) as u16,
            alias: None,
            implements: Vec::new(),
            functions: Vec::new(),
            variables: Vec::new(),
            dll_name: None,
        };
        match kind {
            TypeKind::Alias => ti.alias = Some(self.datatype(dt1, imports, 0)?),
            TypeKind::Module => ti.dll_name = self.opt_string(dt1)?,
            TypeKind::CoClass => ti.implements = self.coclass_impls(dt1, n_impl, imports)?,
            TypeKind::Interface | TypeKind::Dispatch => {
                if n_impl > 1 {
                    self.warnings.push(format!(
                        "type {}: {n_impl} implemented types, only the first is read",
                        ti.name
                    ));
                }
                if n_impl >= 1 && dt1 != NONE {
                    ti.implements.push(ImplType {
                        target: self.href(dt1, imports)?,
                        flags: ImplFlags::empty(),
                    });
                }
            }
            TypeKind::Enum | TypeKind::Record | TypeKind::Union => {}
        }
        if (n_funcs > 0 || n_vars > 0) && mem_off != NONE {
            self.members(&mut ti, mem_off as usize, n_funcs, n_vars, imports)?;
        } else if n_funcs > 0 || n_vars > 0 {
            return Err(TypelibError::Malformed {
                what: "typeinfo",
                detail: "members counted but no member data",
            });
        }
        Ok(ti)
    }

    fn coclass_impls(&self, first: u32, count: usize, imports: &[Import]) -> Result<Vec<ImplType>> {
        let mut out = Vec::new();
        let mut off = first;
        while out.len() < count && off != NONE {
            let p = self.in_seg(SEG_REFTAB, "reference table", off as usize, 16)?;
            let href = u32_at(self.d, "reference", p)?;
            let flags = u32_at(self.d, "reference", p + 4)?;
            let next = u32_at(self.d, "reference", p + 12)?;
            out.push(ImplType {
                target: self.href(href, imports)?,
                flags: ImplFlags::from_bits_retain(flags),
            });
            off = next;
        }
        if out.len() != count {
            return Err(TypelibError::Malformed {
                what: "reference table",
                detail: "implemented type chain shorter than the count",
            });
        }
        Ok(out)
    }

    fn members(
        &mut self,
        ti: &mut TypeInfo,
        mem_off: usize,
        n_funcs: usize,
        n_vars: usize,
        imports: &[Import],
    ) -> Result<()> {
        self.members_seen = self.members_seen.saturating_add(n_funcs + n_vars);
        if self.members_seen > self.limits.max_members {
            return Err(TypelibError::LimitExceeded("too many members"));
        }
        let total = n_funcs + n_vars;
        let rec_len = u32_at(self.d, "member data", mem_off)? as usize;
        let start = mem_off + 4;
        let end = start
            .checked_add(rec_len)
            .filter(|e| *e <= self.d.len())
            .ok_or(oob("member records", mem_off))?;
        // After the records: member ids, then name offsets.
        let ids = end;
        let names = end.checked_add(4 * total).ok_or(oob("member names", end))?;
        let mut q = start;
        for i in 0..n_funcs {
            let (f, next) = self.function(q, end, ids, names, i, imports)?;
            ti.functions.push(f);
            q = next;
        }
        for i in 0..n_vars {
            let k = n_funcs + i;
            let (v, next) = self.variable(q, end, ids, names, k, imports)?;
            ti.variables.push(v);
            q = next;
        }
        if q != end {
            self.warnings.push(format!(
                "type {}: member records end at {q:#x}, expected {end:#x}",
                ti.name
            ));
        }
        Ok(())
    }

    fn member_name_id(&self, ids: usize, names: usize, k: usize) -> Result<(i32, u32)> {
        let id = u32_at(self.d, "member ids", ids + 4 * k)? as i32;
        let n = u32_at(self.d, "member names", names + 4 * k)?;
        Ok((id, n))
    }

    fn function(
        &mut self,
        q: usize,
        end: usize,
        ids: usize,
        names: usize,
        k: usize,
        imports: &[Import],
    ) -> Result<(Function, usize)> {
        let rs = u16_at(self.d, "function record", q)? as usize;
        let next = q
            .checked_add(rs)
            .filter(|n| *n <= end)
            .ok_or(oob("function record", q))?;
        let ret = u32_at(self.d, "function record", q + 4)?;
        let flags = u32_at(self.d, "function record", q + 8)?;
        let vtable_offset = u16_at(self.d, "function record", q + 12)?;
        let fk = u32_at(self.d, "function record", q + 16)?;
        let nargs = u16_at(self.d, "function record", q + 20)? as usize;
        let nopt = u16_at(self.d, "function record", q + 22)? as i16;
        self.params_seen = self.params_seen.saturating_add(nargs);
        if self.params_seen > self.limits.max_params {
            return Err(TypelibError::LimitExceeded("too many parameters"));
        }
        let params_len = nargs * 12;
        let fixed = 24 + params_len;
        if rs < fixed {
            return Err(TypelibError::Malformed {
                what: "function record",
                detail: "record shorter than its parameters",
            });
        }
        let pbase = next - params_len;
        let mut raw_params = Vec::with_capacity(nargs);
        for a in 0..nargs {
            let p = pbase + a * 12;
            raw_params.push((
                u32_at(self.d, "parameter", p)?,
                u32_at(self.d, "parameter", p + 4)?,
                u32_at(self.d, "parameter", p + 8)?,
            ));
        }
        let has_defaults = raw_params
            .iter()
            .any(|p| ParamFlags::from_bits_retain(p.2).contains(ParamFlags::HAS_DEFAULT));
        let extra_words = (rs - fixed) / 4;
        let default_words = if has_defaults { nargs } else { 0 };
        if extra_words < default_words {
            return Err(TypelibError::Malformed {
                what: "function record",
                detail: "default values do not fit the record",
            });
        }
        let help_words = extra_words - default_words;
        let word = |i: usize| u32_at(self.d, "function record", q + 24 + 4 * i);
        let help_context = if help_words >= 1 { word(0)? } else { 0 };
        let doc = if help_words >= 2 {
            self.opt_string(word(1)?)?
        } else {
            None
        };
        let mut params = Vec::with_capacity(nargs);
        for (a, (dt, name_off, pf)) in raw_params.into_iter().enumerate() {
            let pflags = ParamFlags::from_bits_retain(pf);
            let default = if has_defaults && pflags.contains(ParamFlags::HAS_DEFAULT) {
                let v = word(help_words + a)?;
                Some(self.constant(v)?)
            } else {
                None
            };
            params.push(Param {
                name: self.opt_name(name_off)?,
                ty: self.datatype(dt, imports, 0)?,
                flags: pflags,
                default,
            });
        }
        let (memid, name_off) = self.member_name_id(ids, names, k)?;
        let func = Function {
            name: self.name(name_off)?,
            memid,
            kind: match fk & 7 {
                0 => FuncKind::Virtual,
                1 => FuncKind::PureVirtual,
                2 => FuncKind::NonVirtual,
                3 => FuncKind::Static,
                4 => FuncKind::Dispatch,
                _ => {
                    return Err(TypelibError::Malformed {
                        what: "function record",
                        detail: "unknown function kind",
                    });
                }
            },
            invoke: match (fk >> 3) & 0xf {
                1 => InvokeKind::Func,
                2 => InvokeKind::PropertyGet,
                4 => InvokeKind::PropertyPut,
                8 => InvokeKind::PropertyPutRef,
                _ => {
                    return Err(TypelibError::Malformed {
                        what: "function record",
                        detail: "unknown invoke kind",
                    });
                }
            },
            call_conv: match (fk >> 8) & 0xf {
                0 => CallConv::Fastcall,
                1 => CallConv::Cdecl,
                2 => CallConv::Pascal,
                3 => CallConv::MacPascal,
                4 => CallConv::Stdcall,
                5 => CallConv::FpFastcall,
                6 => CallConv::Syscall,
                7 => CallConv::MpwCdecl,
                8 => CallConv::MpwPascal,
                o => CallConv::Other(o as u8),
            },
            flags: FuncFlags::from_bits_retain(flags),
            vtable_offset,
            ret: self.datatype(ret, imports, 0)?,
            params,
            optional_count: nopt,
            doc,
            help_context,
        };
        Ok((func, next))
    }

    fn variable(
        &mut self,
        q: usize,
        end: usize,
        ids: usize,
        names: usize,
        k: usize,
        imports: &[Import],
    ) -> Result<(Variable, usize)> {
        let rs = u16_at(self.d, "variable record", q)? as usize;
        let next = q
            .checked_add(rs)
            .filter(|n| *n <= end)
            .ok_or(oob("variable record", q))?;
        if rs < 20 {
            return Err(TypelibError::Malformed {
                what: "variable record",
                detail: "record shorter than 20 bytes",
            });
        }
        let dt = u32_at(self.d, "variable record", q + 4)?;
        let flags = u32_at(self.d, "variable record", q + 8)?;
        let kind_word = u32_at(self.d, "variable record", q + 12)?;
        let value = u32_at(self.d, "variable record", q + 16)?;
        let kind = match kind_word & 0xffff {
            0 => VarKind::PerInstance,
            1 => VarKind::Static,
            2 => VarKind::Const,
            3 => VarKind::Dispatch,
            _ => {
                return Err(TypelibError::Malformed {
                    what: "variable record",
                    detail: "unknown variable kind",
                });
            }
        };
        let extra = (rs - 20) / 4;
        let word = |i: usize| u32_at(self.d, "variable record", q + 20 + 4 * i);
        let help_context = if extra >= 1 { word(0)? } else { 0 };
        let doc = if extra >= 2 {
            self.opt_string(word(1)?)?
        } else {
            None
        };
        let (memid, name_off) = self.member_name_id(ids, names, k)?;
        let var = Variable {
            name: self.name(name_off)?,
            memid,
            kind,
            ty: self.datatype(dt, imports, 0)?,
            flags: VarFlags::from_bits_retain(flags),
            offset: matches!(kind, VarKind::PerInstance | VarKind::Static).then_some(value),
            value: if kind == VarKind::Const {
                Some(self.constant(value)?)
            } else {
                None
            },
            doc,
            help_context,
        };
        Ok((var, next))
    }
}

fn kind_from(k: u8) -> Option<TypeKind> {
    Some(match k {
        0 => TypeKind::Enum,
        1 => TypeKind::Record,
        2 => TypeKind::Module,
        3 => TypeKind::Interface,
        4 => TypeKind::Dispatch,
        5 => TypeKind::CoClass,
        6 => TypeKind::Alias,
        7 => TypeKind::Union,
        _ => return None,
    })
}

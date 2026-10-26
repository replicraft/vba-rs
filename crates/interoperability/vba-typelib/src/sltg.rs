//! Reader for the SLTG-format type library (see `docs/formats/typelib.md` for
//! the layout, what was verified and the known gaps).
//!
//! SLTG is the older compound layout written by the 16-bit and early 32-bit
//! type library compilers. A file is a small header, a list of block
//! entries, an index of block names, and then the blocks themselves: one per
//! type (`BAAAAAAAAA`, `DAAAAAAAAA`, ...) and a `dir` block that holds the
//! library description, the type list with GUIDs and kinds, and the name
//! table. The result is the same [`TypeLib`] the MSFT reader produces.

use std::collections::HashMap;

use crate::error::{Result, TypelibError};
use crate::model::{
    CallConv, Const, ConstValue, FuncFlags, FuncKind, Function, Guid, ImplFlags, ImplType, Import,
    InvokeKind, LibFlags, Param, ParamFlags, TypeDesc, TypeFlags, TypeInfo, TypeKind, TypeLib,
    TypeRef, VarFlags, VarKind, Variable, Version, vt,
};
use crate::msft::{Limits, decode, oob, u16_at, u32_at};

const MAGIC: &[u8; 4] = b"SLTG";
/// Fixed part of the file header, before the block entries.
const HEADER: usize = 0x24;
const BLOCK_ENTRY: usize = 8;
/// Bytes between the last index name and the first block.
const INDEX_PAD: usize = 9;
const LIB_MAGIC: u16 = 0x51cc;
/// Bytes after the library block, before the type count.
const LIB_GAP: usize = 0x40;
/// Bytes between the type list and the name table header (a small record
/// followed by the 512-byte name hash table).
const DIR_GAP: usize = 0x216;
const TI_MAGIC: u16 = 0x0501;
const TI_HEADER: usize = 0x22;
const TAIL: usize = 0x36;
const REF_MAGIC: u16 = 0x00df;
const REF_NUMBER: usize = 68;
const REF_NAMES: usize = 72 + 7;
const IMPL_MAGIC: u8 = 0x4a;
const IMPL_SIZE: usize = 22;
const FUNC_MAGIC: u8 = 0x4c;
const FUNC_FLAGS_MAGIC: u8 = 0x6c;
const VAR_MAGIC: u8 = 0x0a;
const VAR_FLAGS_MAGIC: u8 = 0x2a;
const NONE16: u16 = 0xffff;
const NONE32: u32 = 0xffff_ffff;
const MAX_DIMS: usize = 64;

/// Parses an SLTG type library.
///
/// # Errors
/// [`TypelibError::NotATypelib`] when the magic is wrong, and the structural
/// errors ([`TypelibError::OutOfRange`], [`TypelibError::Malformed`],
/// [`TypelibError::LimitExceeded`]) when the data is truncated or
/// inconsistent. Malformed input never panics.
pub fn parse(data: &[u8], limits: Limits) -> Result<TypeLib> {
    if data.get(..4) != Some(&MAGIC[..]) {
        return Err(TypelibError::NotATypelib("missing SLTG signature"));
    }
    let n = usize::from(u16_at(data, "header", 4)?);
    let first = u16_at(data, "header", 0x0a)?;
    if n < 2 {
        return Err(TypelibError::Malformed {
            what: "header",
            detail: "fewer than two index names",
        });
    }
    if n > limits.max_types.saturating_add(2) {
        return Err(TypelibError::LimitExceeded("too many types"));
    }
    let mut entries = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let o = HEADER + i * BLOCK_ENTRY;
        entries.push(Entry {
            len: u32_at(data, "block entries", o)? as usize,
            index_string: usize::from(u16_at(data, "block entries", o + 4)?),
            next: u16_at(data, "block entries", o + 6)?,
        });
    }
    let index_off = HEADER + (n - 1) * BLOCK_ENTRY;
    let mut pos = index_off;
    for _ in 0..n {
        pos = cstr(data, "index", pos)?.1;
    }
    let data_start = pos + INDEX_PAD;
    if data_start > data.len() {
        return Err(oob("block data", data_start));
    }

    // Blocks are stored in chain order, starting at `first` (1-based).
    let mut order = Vec::with_capacity(entries.len());
    let mut seen = vec![false; entries.len()];
    let mut cur = first;
    while cur != 0 {
        let i = usize::from(cur) - 1;
        if i >= entries.len() || seen[i] {
            return Err(TypelibError::Malformed {
                what: "block entries",
                detail: "broken block chain",
            });
        }
        seen[i] = true;
        order.push(i);
        cur = entries[i].next;
    }
    if order.len() != entries.len() {
        return Err(TypelibError::Malformed {
            what: "block entries",
            detail: "block chain does not cover every block",
        });
    }
    let mut blocks: Vec<&[u8]> = vec![&[]; entries.len()];
    let mut at = data_start;
    for &i in &order {
        let end = at
            .checked_add(entries[i].len)
            .filter(|&e| e <= data.len())
            .ok_or(oob("block", at))?;
        blocks[i] = &data[at..end];
        at = end;
    }
    let mut by_name: HashMap<&[u8], &[u8]> = HashMap::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        let name = cstr(data, "index", index_off + e.index_string)?.0;
        by_name.entry(name).or_insert(blocks[i]);
    }
    let dir = *by_name.get(&b"dir"[..]).ok_or(TypelibError::Malformed {
        what: "index",
        detail: "no dir block",
    })?;
    let mut rd = Reader {
        limits,
        names: &[],
        ntypes: 0,
        imports: Vec::new(),
        import_keys: HashMap::new(),
        members_seen: 0,
        params_seen: 0,
        warnings: Vec::new(),
    };
    rd.library(dir, &by_name)
}

struct Entry {
    len: usize,
    index_string: usize,
    next: u16,
}

/// One type of the `dir` block's type list.
struct DirType {
    stream: Vec<u8>,
    name_off: u16,
    help_context: u32,
    guid: Guid,
    kind: TypeKind,
}

struct Reader<'a> {
    limits: Limits,
    names: &'a [u8],
    ntypes: usize,
    imports: Vec<Import>,
    import_keys: HashMap<(u16, usize), usize>,
    members_seen: usize,
    params_seen: usize,
    warnings: Vec<String>,
}

/// A bounds-checked cursor over a block.
struct Cur<'a> {
    d: &'a [u8],
    pos: usize,
    what: &'static str,
}

impl<'a> Cur<'a> {
    fn new(d: &'a [u8], what: &'static str) -> Cur<'a> {
        Cur { d, pos: 0, what }
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.d.len())
            .ok_or(oob(self.what, self.pos))?;
        let s = &self.d[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A length-prefixed string; length `0xffff` means none.
    fn pstring(&mut self) -> Result<Option<Vec<u8>>> {
        let len = self.u16()?;
        if len == NONE16 {
            return Ok(None);
        }
        Ok(Some(self.bytes(usize::from(len))?.to_vec()))
    }

    fn guid(&mut self) -> Result<Guid> {
        let b = self.bytes(16)?;
        let mut g = [0; 16];
        g.copy_from_slice(b);
        Ok(Guid(g))
    }
}

/// A NUL-terminated string at `pos`; returns it and the position after the NUL.
fn cstr<'a>(d: &'a [u8], what: &'static str, pos: usize) -> Result<(&'a [u8], usize)> {
    let rest = d.get(pos..).ok_or(oob(what, pos))?;
    let len = rest.iter().position(|&b| b == 0).ok_or(oob(what, pos))?;
    Ok((&rest[..len], pos + len + 1))
}

fn kind_of(v: u16) -> Option<TypeKind> {
    Some(match v {
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

fn help_context(v: u16) -> u32 {
    if v >= 0xfffe { 0 } else { u32::from(v) }
}

/// One type description element: wrappers around a terminal type.
enum Wrap {
    Ptr,
    SafeArray,
    CArray(Vec<(u32, i32)>),
}

impl<'a> Reader<'a> {
    fn name(&self, off: u16) -> Result<Option<String>> {
        if off >= 0xfffe {
            return Ok(None);
        }
        let start = usize::from(off) + 8;
        let rest = self.names.get(start..).ok_or(oob("name table", start))?;
        let len = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or(oob("name table", start))?;
        Ok(Some(decode(&rest[..len])))
    }

    fn name_or_empty(&mut self, off: u16, what: &str) -> Result<String> {
        match self.name(off)? {
            Some(n) => Ok(n),
            None => {
                self.warnings.push(format!("{what} has no name"));
                Ok(String::new())
            }
        }
    }

    fn library(&mut self, dir: &'a [u8], blocks: &HashMap<&[u8], &'a [u8]>) -> Result<TypeLib> {
        let mut c = Cur::new(dir, "library block");
        if c.u16()? != LIB_MAGIC {
            return Err(TypelibError::Malformed {
                what: "library block",
                detail: "bad magic",
            });
        }
        let _res02 = c.u16()?;
        let name_off = c.u16()?;
        let _res06 = c.u16()?;
        let doc = c.pstring()?;
        let _helpfile = c.pstring()?;
        let lib_help_context = c.u32()?;
        let sys_kind = c.u16()?;
        let lcid = c.u16()?;
        let _res12 = c.u32()?;
        let lib_flags = c.u16()?;
        let major = c.u16()?;
        let minor = c.u16()?;
        let guid = c.guid()?;
        c.bytes(LIB_GAP)?;
        let ntypes = usize::from(c.u16()?);
        if ntypes > self.limits.max_types {
            return Err(TypelibError::LimitExceeded("too many types"));
        }
        let mut list = Vec::with_capacity(ntypes);
        for _ in 0..ntypes {
            let stream = c.pstring()?.unwrap_or_default();
            let _other = c.pstring()?;
            let _res1a = c.u16()?;
            let name_off = c.u16()?;
            let more = c.u16()?;
            c.bytes(usize::from(more))?;
            let _res20 = c.u16()?;
            let help_context = c.u32()?;
            let _res26 = c.u16()?;
            let guid = c.guid()?;
            let kind = c.u16()?;
            let kind = kind_of(kind).ok_or(TypelibError::Malformed {
                what: "type list",
                detail: "unknown type kind",
            })?;
            list.push(DirType {
                stream,
                name_off,
                help_context,
                guid,
                kind,
            });
        }
        c.bytes(DIR_GAP)?;
        // The name table is preceded by its size and a zero word.
        let size = usize::from(c.u16()?);
        c.bytes(2)?;
        let table = c.bytes(size.min(dir.len().saturating_sub(c.pos)))?;
        self.names = table;
        self.ntypes = ntypes;

        let lib_name = self.name_or_empty(name_off, "library")?;
        let mut types = Vec::with_capacity(ntypes);
        for dt in &list {
            let blk = blocks
                .get(dt.stream.as_slice())
                .copied()
                .ok_or(TypelibError::Malformed {
                    what: "type list",
                    detail: "type block missing from the index",
                })?;
            types.push(self.type_info(dt, blk)?);
        }
        Ok(TypeLib {
            name: lib_name,
            guid,
            lcid: u32::from(lcid),
            version: Version { major, minor },
            flags: LibFlags::from_bits_truncate(u32::from(lib_flags)),
            sys_kind: u8::try_from(sys_kind).ok().filter(|&k| k <= 3),
            doc: doc.map(|d| decode(&d)),
            help_context: lib_help_context,
            imports: std::mem::take(&mut self.imports),
            types,
            warnings: std::mem::take(&mut self.warnings),
        })
    }

    fn type_info(&mut self, dt: &DirType, blk: &[u8]) -> Result<TypeInfo> {
        if u16_at(blk, "type block", 0)? != TI_MAGIC {
            return Err(TypelibError::Malformed {
                what: "type block",
                detail: "bad magic",
            });
        }
        if blk.len() < TI_HEADER {
            return Err(oob("type block", blk.len()));
        }
        let href_table = u32_at(blk, "type block", 2)?;
        let elem_table = u32_at(blk, "type block", 0x0a)? as usize;
        let version = u32_at(blk, "type block", 0x12)?;
        let flags1 = blk[0x1a];
        let flags2 = blk[0x1b];
        if kind_of(u16::from(blk[0x1d])) != Some(dt.kind) {
            self.warnings.push(format!(
                "type {}: block kind differs from the type list",
                dt.stream.escape_ascii()
            ));
        }
        let name = self.name_or_empty(dt.name_off, "type")?;
        let refs = if href_table == NONE32 {
            Vec::new()
        } else {
            self.refs(blk, href_table as usize)?
        };

        let mut c = Cur::new(blk, "member block");
        c.pos = elem_table;
        let _res00 = c.u16()?;
        let _res02 = c.u16()?;
        let _res04 = c.bytes(1)?;
        let extra = c.u32()? as usize;
        let group = c.bytes(extra)?;
        let tail = c.bytes(TAIL)?;
        let t16 = |o: usize| u16::from_le_bytes([tail[o], tail[o + 1]]);
        let cfuncs = usize::from(t16(0));
        let cvars = usize::from(t16(2));
        let cimpls = usize::from(t16(4));
        let funcs_off = t16(0x08);
        let vars_off = t16(0x0a);
        let impls_off = t16(0x0c);
        let instance_size = u32::from(t16(0x20));
        let alignment = t16(0x22);
        let vtable_size = t16(0x28);

        self.members_seen = self
            .members_seen
            .saturating_add(cfuncs)
            .saturating_add(cvars);
        if self.members_seen > self.limits.max_members {
            return Err(TypelibError::LimitExceeded("too many members"));
        }
        if cfuncs > group.len() / 22 || cvars > group.len() / 0x12 || cimpls > group.len() / 22 {
            return Err(TypelibError::Malformed {
                what: "member block",
                detail: "member count exceeds the data",
            });
        }

        let implements = self.impls(group, cimpls, impls_off, &refs)?;
        let dispatch = dt.kind == TypeKind::Dispatch;
        let functions = self.functions(
            group,
            cfuncs,
            if funcs_off == NONE16 {
                cimpls * IMPL_SIZE
            } else {
                usize::from(funcs_off)
            },
            dispatch,
            &refs,
        )?;
        let variables = self.variables(
            group,
            cvars,
            if vars_off == NONE16 {
                0
            } else {
                usize::from(vars_off)
            },
            dt.kind,
            &refs,
        )?;
        if dt.kind == TypeKind::Alias {
            self.warnings
                .push(format!("alias {name}: the aliased type is not decoded"));
        }
        Ok(TypeInfo {
            name,
            kind: dt.kind,
            guid: Some(dt.guid).filter(|g| g.0 != [0; 16]),
            flags: TypeFlags::from_bits_truncate(
                (u32::from(flags1) >> 3) | (u32::from(flags2) << 5),
            ),
            version: Version {
                major: (version & 0xffff) as u16,
                minor: (version >> 16) as u16,
            },
            doc: None,
            help_context: dt.help_context,
            alignment: u8::try_from(alignment).unwrap_or(0),
            instance_size,
            vtable_size,
            alias: None,
            implements,
            functions,
            variables,
            dll_name: None,
        })
    }

    /// The reference table of a type block: the types its descriptions may
    /// name, in order. Local entries are `*\Rffff*#N`, entries of other
    /// libraries `*\Rxxxx*#N` with `xxxx` the name table offset of the
    /// `*\G{guid}#major.minor#lcid#file#` string describing the library.
    fn refs(&mut self, blk: &[u8], at: usize) -> Result<Vec<TypeRef>> {
        if u16_at(blk, "reference table", at)? != REF_MAGIC {
            return Err(TypelibError::Malformed {
                what: "reference table",
                detail: "bad magic",
            });
        }
        let number = u32_at(blk, "reference table", at + REF_NUMBER)? as usize;
        let count = number >> 3;
        if count > blk.len() / 2 {
            return Err(TypelibError::Malformed {
                what: "reference table",
                detail: "reference count exceeds the data",
            });
        }
        let mut c = Cur::new(blk, "reference table");
        c.pos = at + REF_NAMES + number;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let s = c.pstring()?.unwrap_or_default();
            out.push(self.reference(&decode(&s))?);
        }
        Ok(out)
    }

    fn reference(&mut self, s: &str) -> Result<TypeRef> {
        let bad = TypelibError::Malformed {
            what: "reference table",
            detail: "unreadable reference",
        };
        let rest = s.strip_prefix("*\\R").ok_or(bad.clone())?;
        let (lib, num) = rest.split_once("*#").ok_or(bad.clone())?;
        let lib = u16::from_str_radix(lib, 16).map_err(|_| bad.clone())?;
        let num = usize::from_str_radix(num, 16).map_err(|_| bad)?;
        if lib == NONE16 {
            if num >= self.ntypes {
                return Err(TypelibError::Malformed {
                    what: "reference table",
                    detail: "reference to a type that does not exist",
                });
            }
            return Ok(TypeRef::Local(num));
        }
        if let Some(&i) = self.import_keys.get(&(lib, num)) {
            return Ok(TypeRef::Imported(i));
        }
        let import = self.import(lib);
        self.imports.push(import);
        let i = self.imports.len() - 1;
        self.import_keys.insert((lib, num), i);
        Ok(TypeRef::Imported(i))
    }

    fn import(&mut self, off: u16) -> Import {
        let mut imp = Import {
            library_guid: None,
            lcid: 0,
            version: Version::default(),
            file: String::new(),
            type_guid: None,
            kind: None,
        };
        let start = usize::from(off);
        let text = [start + 8, start]
            .iter()
            .filter_map(|&s| self.names.get(s..))
            .map(|r| {
                let end = r.iter().position(|&b| b == 0).unwrap_or(r.len());
                decode(&r[..end])
            })
            .find(|t| t.starts_with("*\\G{"));
        let Some(text) = text else {
            self.warnings
                .push(format!("import library at {off:#x} is unreadable"));
            return imp;
        };
        let mut parts = text.split('#');
        imp.library_guid = parts
            .next()
            .and_then(|g| g.strip_prefix("*\\G{"))
            .and_then(|g| g.strip_suffix('}'))
            .and_then(parse_guid);
        if let Some((a, b)) = parts.next().and_then(|v| v.split_once('.')) {
            imp.version = Version {
                major: a.parse().unwrap_or(0),
                minor: b.parse().unwrap_or(0),
            };
        }
        imp.lcid = parts
            .next()
            .and_then(|l| u32::from_str_radix(l, 16).ok())
            .unwrap_or(0);
        imp.file = parts.next().unwrap_or("").to_owned();
        imp
    }

    fn impls(
        &mut self,
        g: &[u8],
        count: usize,
        off: u16,
        refs: &[TypeRef],
    ) -> Result<Vec<ImplType>> {
        let mut out = Vec::with_capacity(count);
        let mut at = if off == NONE16 { 0 } else { usize::from(off) };
        for _ in 0..count {
            let rec = g
                .get(at..at + IMPL_SIZE)
                .ok_or(oob("implemented type", at))?;
            if rec[0] != IMPL_MAGIC {
                return Err(TypelibError::Malformed {
                    what: "implemented type",
                    detail: "bad magic",
                });
            }
            let next = u16::from_le_bytes([rec[2], rec[3]]);
            let flags = ImplFlags::from_bits_truncate(u32::from(rec[6]));
            let (desc, _) = type_desc(&rec[18..], 0, refs, self.limits.max_type_depth)?;
            let TypeDesc::User(target) = desc else {
                return Err(TypelibError::Malformed {
                    what: "implemented type",
                    detail: "not a type reference",
                });
            };
            out.push(ImplType { target, flags });
            at = if next == NONE16 {
                at + IMPL_SIZE
            } else {
                usize::from(next)
            };
        }
        Ok(out)
    }

    fn functions(
        &mut self,
        g: &[u8],
        count: usize,
        start: usize,
        dispatch: bool,
        refs: &[TypeRef],
    ) -> Result<Vec<Function>> {
        let mut out = Vec::with_capacity(count);
        let mut at = start;
        for _ in 0..count {
            let (f, next) = self.function(g, at, dispatch, refs)?;
            out.push(f);
            if next == NONE16 {
                break;
            }
            at = usize::from(next);
        }
        if out.len() != count {
            self.warnings
                .push("function chain ends before the declared count".to_owned());
        }
        Ok(out)
    }

    fn function(
        &mut self,
        g: &[u8],
        at: usize,
        dispatch: bool,
        refs: &[TypeRef],
    ) -> Result<(Function, u16)> {
        let magic = *g.get(at).ok_or(oob("function", at))?;
        if magic != FUNC_MAGIC && magic != FUNC_FLAGS_MAGIC {
            return Err(TypelibError::Malformed {
                what: "function",
                detail: "bad magic",
            });
        }
        let inv = *g.get(at + 1).ok_or(oob("function", at + 1))?;
        let next = u16_at(g, "function", at + 2)?;
        let name = u16_at(g, "function", at + 4)?;
        let dispid = u32_at(g, "function", at + 6)? as i32;
        let help_ctx = u16_at(g, "function", at + 10)?;
        let arg_off = u16_at(g, "function", at + 14)?;
        let nacc = *g.get(at + 16).ok_or(oob("function", at + 16))?;
        let ret_cell = *g.get(at + 17).ok_or(oob("function", at + 17))?;
        let ret_word = u16_at(g, "function", at + 18)?;
        let vtbl = u16_at(g, "function", at + 20)?;
        let flags = if magic == FUNC_FLAGS_MAGIC {
            u16_at(g, "function", at + 22)?
        } else {
            0
        };
        let invoke = match inv >> 4 {
            1 => InvokeKind::Func,
            2 => InvokeKind::PropertyGet,
            4 => InvokeKind::PropertyPut,
            8 => InvokeKind::PropertyPutRef,
            _ => {
                self.warnings
                    .push(format!("function at {at:#x}: unknown invoke kind {inv:#x}"));
                InvokeKind::Func
            }
        };
        let nargs = usize::from(nacc >> 3);
        self.params_seen = self.params_seen.saturating_add(nargs);
        if self.params_seen > self.limits.max_params {
            return Err(TypelibError::LimitExceeded("too many parameters"));
        }
        let depth = self.limits.max_type_depth;
        // The return type is inline (one word) when the high bit of the cell
        // byte is set, otherwise the word is an offset into the member data.
        let (ret, _) = if ret_cell & 0x80 != 0 {
            type_desc(&ret_word.to_le_bytes(), 0, refs, depth)?
        } else {
            type_desc(g, usize::from(ret_word), refs, depth)?
        };
        let mut params = Vec::with_capacity(nargs);
        for i in 0..nargs {
            let a = usize::from(arg_off) + 4 * i;
            let raw_name = u16_at(g, "function arguments", a)?;
            let tw = u16_at(g, "function arguments", a + 2)?;
            let inline = raw_name & 1 != 0;
            let (ty, first) = if inline {
                type_desc(&tw.to_le_bytes(), 0, refs, depth)?
            } else {
                type_desc(g, usize::from(tw), refs, depth)?
            };
            params.push(Param {
                name: self.name(raw_name & !1)?,
                ty,
                flags: param_flags(first),
                default: None,
            });
        }
        let f = Function {
            name: self.name_or_empty(name, "function")?,
            memid: dispid,
            kind: if dispatch {
                FuncKind::Dispatch
            } else {
                FuncKind::PureVirtual
            },
            invoke,
            call_conv: CallConv::Stdcall,
            flags: FuncFlags::from_bits_truncate(u32::from(flags)),
            vtable_offset: vtbl,
            ret,
            params,
            optional_count: 0,
            doc: None,
            help_context: help_context(help_ctx),
        };
        Ok((f, next))
    }

    fn variables(
        &mut self,
        g: &[u8],
        count: usize,
        start: usize,
        kind: TypeKind,
        refs: &[TypeRef],
    ) -> Result<Vec<Variable>> {
        let mut out = Vec::with_capacity(count);
        let mut at = start;
        for _ in 0..count {
            let magic = *g.get(at).ok_or(oob("variable", at))?;
            if magic != VAR_MAGIC && magic != VAR_FLAGS_MAGIC {
                return Err(TypelibError::Malformed {
                    what: "variable",
                    detail: "bad magic",
                });
            }
            let flags = *g.get(at + 1).ok_or(oob("variable", at + 1))?;
            let next = u16_at(g, "variable", at + 2)?;
            let name = u16_at(g, "variable", at + 4)?;
            let byte_offs = u16_at(g, "variable", at + 6)?;
            let ty_word = u16_at(g, "variable", at + 8)?;
            let memid = u32_at(g, "variable", at + 10)? as i32;
            let help_ctx = u16_at(g, "variable", at + 14)?;
            let var_flags = if magic == VAR_FLAGS_MAGIC {
                u16_at(g, "variable", at + 18)?
            } else {
                0
            };
            let depth = self.limits.max_type_depth;
            let (ty, _) = if flags & 0x02 != 0 {
                type_desc(&ty_word.to_le_bytes(), 0, refs, depth)?
            } else {
                type_desc(g, usize::from(ty_word), refs, depth)?
            };
            let is_const = flags & 0x40 != 0;
            let (vkind, offset, value) = if is_const {
                let value = if flags & 0x08 != 0 {
                    Const {
                        vt: vt::I4,
                        value: ConstValue::Int(i64::from(byte_offs)),
                    }
                } else {
                    self.warnings.push(format!(
                        "constant at {at:#x} is stored out of line; value not decoded"
                    ));
                    Const {
                        vt: vt::I4,
                        value: ConstValue::Unsupported,
                    }
                };
                (VarKind::Const, None, Some(value))
            } else if kind == TypeKind::Dispatch {
                (VarKind::Dispatch, None, None)
            } else {
                (VarKind::PerInstance, Some(u32::from(byte_offs)), None)
            };
            out.push(Variable {
                name: self.name_or_empty(name, "variable")?,
                memid,
                kind: vkind,
                ty,
                flags: VarFlags::from_bits_truncate(u32::from(var_flags)),
                offset,
                value,
                doc: None,
                help_context: help_context(help_ctx),
            });
            if next == NONE16 {
                break;
            }
            at = usize::from(next);
        }
        if out.len() != count {
            self.warnings
                .push("variable chain ends before the declared count".to_owned());
        }
        Ok(out)
    }
}

/// Parameter flags from the first word of a type description.
fn param_flags(w: u16) -> ParamFlags {
    let mut f = if w & 0xc000 == 0xc000 {
        ParamFlags::empty()
    } else if w & 0x8000 != 0 {
        ParamFlags::IN | ParamFlags::OUT
    } else if w & 0x4000 != 0 {
        ParamFlags::OUT
    } else {
        ParamFlags::IN
    };
    if w & 0x2000 != 0 {
        f |= ParamFlags::LCID;
    }
    if w & 0x80 != 0 {
        f |= ParamFlags::RETVAL;
    }
    f
}

/// Reads a type description made of 16-bit words at byte offset `pos` of
/// `d`. Returns it and the first word (which carries the parameter flags).
///
/// A word's low six bits are the `VT_*` tag; the bits `0xe00` all set add a
/// pointer. `USERDEFINED` is followed by the reference (four times the index
/// into the type block's reference table), `CARRAY` by the offset of a
/// `SAFEARRAY` structure in the member data, `SAFEARRAY` by an unused offset.
fn type_desc(d: &[u8], pos: usize, refs: &[TypeRef], depth: usize) -> Result<(TypeDesc, u16)> {
    let mut wraps = Vec::new();
    let mut p = pos;
    let mut first = None;
    let term = loop {
        if wraps.len() > depth {
            return Err(TypelibError::LimitExceeded(
                "type description nesting too deep",
            ));
        }
        let w = u16_at(d, "type description", p)?;
        first.get_or_insert(w);
        p += 2;
        if w & 0x0e00 == 0x0e00 {
            wraps.push(Wrap::Ptr);
        }
        match w & 0x3f {
            vt::PTR => wraps.push(Wrap::Ptr),
            vt::USERDEFINED => {
                let href = usize::from(u16_at(d, "type description", p)?);
                let r = refs.get(href / 4).ok_or(TypelibError::Malformed {
                    what: "type description",
                    detail: "type reference outside the reference table",
                })?;
                break TypeDesc::User(*r);
            }
            vt::CARRAY => {
                let off = usize::from(u16_at(d, "type description", p)?);
                p += 2;
                wraps.push(Wrap::CArray(safearray_bounds(d, off)?));
            }
            vt::SAFEARRAY => {
                u16_at(d, "type description", p)?;
                p += 2;
                wraps.push(Wrap::SafeArray);
            }
            v => break TypeDesc::Prim(v),
        }
    };
    let mut ty = term;
    for w in wraps.into_iter().rev() {
        ty = match w {
            Wrap::Ptr => TypeDesc::Ptr(Box::new(ty)),
            Wrap::SafeArray => TypeDesc::SafeArray(Box::new(ty)),
            Wrap::CArray(bounds) => TypeDesc::CArray {
                elem: Box::new(ty),
                bounds,
            },
        };
    }
    Ok((ty, first.unwrap_or(0)))
}

/// The bounds of a fixed array: a 32-bit `SAFEARRAY` header (16 bytes) then
/// one `(count, lower bound)` pair per dimension.
fn safearray_bounds(d: &[u8], off: usize) -> Result<Vec<(u32, i32)>> {
    let dims = usize::from(u16_at(d, "array description", off)?);
    if dims == 0 || dims > MAX_DIMS {
        return Err(TypelibError::Malformed {
            what: "array description",
            detail: "unsupported number of dimensions",
        });
    }
    (0..dims)
        .map(|i| {
            let b = off + 16 + 8 * i;
            Ok((
                u32_at(d, "array description", b)?,
                u32_at(d, "array description", b + 4)? as i32,
            ))
        })
        .collect()
}

/// Parses `8-4-4-4-12` hexadecimal GUID text.
fn parse_guid(s: &str) -> Option<Guid> {
    let parts: Vec<&str> = s.split('-').collect();
    let [a, b, c, d, e] = parts.as_slice() else {
        return None;
    };
    if [a.len(), b.len(), c.len(), d.len(), e.len()] != [8, 4, 4, 4, 12] {
        return None;
    }
    let d1 = u32::from_str_radix(a, 16).ok()?;
    let d2 = u16::from_str_radix(b, 16).ok()?;
    let d3 = u16::from_str_radix(c, 16).ok()?;
    let tail = format!("{d}{e}");
    let mut d4 = [0u8; 8];
    for (i, byte) in d4.iter_mut().enumerate() {
        *byte = u8::from_str_radix(tail.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(Guid::from_fields(d1, d2, d3, d4))
}

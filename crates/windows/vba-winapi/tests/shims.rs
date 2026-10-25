//! Table-driven reference tests for every native shim. Each case names the
//! symbol it covers; the number of cases per symbol must equal the `tests`
//! column of `catalog.csv` (checked in `catalog.rs`).

#![allow(missing_docs)]

use vba_ffi::{EntryCandidate, LogicalThread, NativeReturn};
use vba_ffi::{MemoryError, NativeArg};
use vba_profile::{OperatingSystem, Target};
use vba_value::VbaString;
use vba_winapi::catalog::catalog;
use vba_winapi::{CallContext, Identity, ManualClock, Registry, WinApiError, WindowsVersion};

pub struct Fx {
    pub ctx: CallContext,
    pub reg: Registry,
    pub clock: ManualClock,
}

#[allow(clippy::new_without_default)]
impl Fx {
    pub fn new() -> Self {
        let t = Target::WIN32;
        let clock = ManualClock::at(1000);
        let ctx = CallContext::new(
            t,
            Box::new(clock.clone()),
            Identity::new("alice", "BOX1").unwrap(),
        );
        Fx {
            ctx,
            reg: Registry::with_native_shims(),
            clock,
        }
    }

    pub fn try_call(&mut self, sym: &str, args: &[i64]) -> Result<NativeReturn, WinApiError> {
        let dll = catalog()
            .iter()
            .find(|r| r.exports().iter().any(|e| e == sym))
            .unwrap()
            .dll
            .clone();
        let r = self.reg.resolve(
            OperatingSystem::Windows,
            &dll,
            &[EntryCandidate::Symbol(sym.into())],
        )?;
        let a: Vec<NativeArg> = args.iter().map(|v| NativeArg::Int(*v)).collect();
        self.reg.call(&mut self.ctx, &r, &a)
    }

    pub fn int(&mut self, sym: &str, args: &[i64]) -> i64 {
        match self.try_call(sym, args).unwrap() {
            NativeReturn::Int(v) => v,
            other => panic!("{sym} returned {other:?}"),
        }
    }

    pub fn void(&mut self, sym: &str, args: &[i64]) {
        assert_eq!(self.try_call(sym, args).unwrap(), NativeReturn::Void);
    }

    pub fn err(&mut self, sym: &str, args: &[i64]) -> WinApiError {
        self.try_call(sym, args).unwrap_err()
    }

    pub fn last(&self) -> u32 {
        self.ctx.last_error()
    }

    pub fn a(&mut self, s: &str) -> i64 {
        let mut b = s.bytes().collect::<Vec<_>>();
        b.push(0);
        self.ctx.memory.alloc_bytes(&b).unwrap() as i64
    }

    pub fn w(&mut self, s: &str) -> i64 {
        let v = VbaString::from_units(s.encode_utf16().collect());
        self.ctx.memory.alloc_wide(&v).unwrap() as i64
    }

    pub fn raw(&mut self, b: &[u8]) -> i64 {
        self.ctx.memory.alloc_bytes(b).unwrap() as i64
    }

    pub fn buf(&mut self, n: usize) -> i64 {
        self.ctx.memory.alloc(n).unwrap() as i64
    }

    pub fn cstr(&self, p: i64) -> Vec<u8> {
        self.ctx.memory.read_cstr(p as u64).unwrap()
    }

    pub fn wstr(&self, p: i64) -> String {
        String::from_utf16(&self.ctx.memory.read_wstr(p as u64).unwrap()).unwrap()
    }

    pub fn bytes(&self, p: i64, n: usize) -> Vec<u8> {
        self.ctx.memory.read(p as u64, n).unwrap().to_vec()
    }

    pub fn u32_at(&self, p: i64) -> u32 {
        self.ctx.memory.read_u32(p as u64).unwrap()
    }

    pub fn cell(&mut self, v: u32) -> i64 {
        self.raw(&v.to_le_bytes())
    }

    pub fn units(&self, p: i64, n: usize) -> Vec<u16> {
        self.bytes(p, n * 2)
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect()
    }
}

type Case = (&'static str, &'static str, fn(&mut Fx));

const CP_ACP: i64 = 0;
const UTF8: i64 = 65001;
const ASCII: i64 = 20127;
const CP1252: i64 = 1252;

pub const CASES: &[Case] = &[
    // --- time -----------------------------------------------------------
    ("GetTickCount", "returns the injected clock", |f| {
        assert_eq!(f.int("GetTickCount", &[]), 1000)
    }),
    ("GetTickCount", "wraps at 32 bits", |f| {
        f.clock.advance((1 << 32) + 5 - 1000);
        assert_eq!(f.int("GetTickCount", &[]), 5);
    }),
    ("GetTickCount", "advances with Sleep", |f| {
        f.void("Sleep", &[250]);
        assert_eq!(f.int("GetTickCount", &[]), 1250);
    }),
    ("GetTickCount64", "does not wrap", |f| {
        f.clock.advance((1 << 32) + 5 - 1000);
        assert_eq!(f.int("GetTickCount64", &[]), (1 << 32) + 5);
    }),
    ("GetTickCount64", "returns the injected clock", |f| {
        assert_eq!(f.int("GetTickCount64", &[]), 1000)
    }),
    (
        "QueryPerformanceCounter",
        "writes ticks and returns TRUE",
        |f| {
            let p = f.buf(8);
            assert_eq!(f.int("QueryPerformanceCounter", &[p]), 1);
            assert_eq!(
                i64::from_le_bytes(f.bytes(p, 8).try_into().unwrap()),
                1000 * 10_000
            );
        },
    ),
    ("QueryPerformanceCounter", "is monotonic", |f| {
        let p = f.buf(8);
        f.int("QueryPerformanceCounter", &[p]);
        let a = f.bytes(p, 8);
        f.clock.advance(1);
        f.int("QueryPerformanceCounter", &[p]);
        assert!(
            i64::from_le_bytes(f.bytes(p, 8).try_into().unwrap())
                > i64::from_le_bytes(a.try_into().unwrap())
        );
    }),
    (
        "QueryPerformanceCounter",
        "null pointer is a memory fault",
        |f| {
            assert_eq!(
                f.err("QueryPerformanceCounter", &[0]),
                WinApiError::Memory(MemoryError::NullPointer)
            );
        },
    ),
    ("QueryPerformanceFrequency", "writes 10 MHz", |f| {
        let p = f.buf(8);
        assert_eq!(f.int("QueryPerformanceFrequency", &[p]), 1);
        assert_eq!(
            i64::from_le_bytes(f.bytes(p, 8).try_into().unwrap()),
            10_000_000
        );
    }),
    (
        "QueryPerformanceFrequency",
        "short buffer is a memory fault",
        |f| {
            let p = f.buf(4);
            assert!(matches!(
                f.err("QueryPerformanceFrequency", &[p]),
                WinApiError::Memory(MemoryError::OutOfBounds { .. })
            ));
        },
    ),
    ("Sleep", "records the duration", |f| {
        f.void("Sleep", &[40]);
        f.void("Sleep", &[2]);
        assert_eq!(f.clock.slept(), vec![40, 2]);
    }),
    ("Sleep", "zero yields without advancing time", |f| {
        f.void("Sleep", &[0]);
        assert_eq!(f.int("GetTickCount", &[]), 1000);
    }),
    ("Sleep", "INFINITE is refused", |f| {
        assert!(matches!(
            f.err("Sleep", &[-1]),
            WinApiError::Unsupported { .. }
        ));
        assert!(f.clock.slept().is_empty());
    }),
    // --- lstrlen --------------------------------------------------------
    ("lstrlenA", "counts bytes", |f| {
        let p = f.a("abc");
        assert_eq!(f.int("lstrlenA", &[p]), 3);
    }),
    ("lstrlenA", "empty string", |f| {
        let p = f.a("");
        assert_eq!(f.int("lstrlenA", &[p]), 0);
    }),
    ("lstrlenA", "NULL is 0", |f| {
        assert_eq!(f.int("lstrlenA", &[0]), 0)
    }),
    ("lstrlenA", "unterminated buffer is a memory fault", |f| {
        let p = f.raw(b"abc");
        assert!(matches!(f.err("lstrlenA", &[p]), WinApiError::Memory(_)));
    }),
    ("lstrlenW", "counts characters", |f| {
        let p = f.w("abc");
        assert_eq!(f.int("lstrlenW", &[p]), 3);
    }),
    ("lstrlenW", "NULL is 0", |f| {
        assert_eq!(f.int("lstrlenW", &[0]), 0)
    }),
    ("lstrlenW", "surrogate pair is two units", |f| {
        let p = f.w("\u{1F600}");
        assert_eq!(f.int("lstrlenW", &[p]), 2);
    }),
    // --- lstrcpy / lstrcat ----------------------------------------------
    ("lstrcpyA", "copies and returns the destination", |f| {
        let (d, s) = (f.buf(8), f.a("hey"));
        assert_eq!(f.int("lstrcpyA", &[d, s]), d);
        assert_eq!(f.cstr(d), b"hey");
    }),
    ("lstrcpyA", "NULL source returns NULL", |f| {
        let d = f.buf(8);
        assert_eq!(f.int("lstrcpyA", &[d, 0]), 0);
    }),
    ("lstrcpyA", "NULL destination returns NULL", |f| {
        let s = f.a("x");
        assert_eq!(f.int("lstrcpyA", &[0, s]), 0);
    }),
    (
        "lstrcpyA",
        "undersized destination faults without writing",
        |f| {
            let (d, s) = (f.buf(3), f.a("hey"));
            assert!(matches!(f.err("lstrcpyA", &[d, s]), WinApiError::Memory(_)));
            assert_eq!(f.bytes(d, 3), vec![0, 0, 0]);
        },
    ),
    ("lstrcpyW", "copies wide text", |f| {
        let (d, s) = (f.buf(16), f.w("héy"));
        assert_eq!(f.int("lstrcpyW", &[d, s]), d);
        assert_eq!(f.wstr(d), "héy");
    }),
    ("lstrcpyW", "NULL source returns NULL", |f| {
        let d = f.buf(4);
        assert_eq!(f.int("lstrcpyW", &[d, 0]), 0);
    }),
    ("lstrcatA", "appends", |f| {
        let (d, s) = (f.buf(16), f.a("ab"));
        f.int("lstrcpyA", &[d, s]);
        let t = f.a("cd");
        assert_eq!(f.int("lstrcatA", &[d, t]), d);
        assert_eq!(f.cstr(d), b"abcd");
    }),
    ("lstrcatA", "NULL source returns NULL", |f| {
        let d = f.buf(4);
        assert_eq!(f.int("lstrcatA", &[d, 0]), 0);
    }),
    ("lstrcatA", "overflow faults without writing", |f| {
        let d = f.raw(b"ab\0\0");
        let s = f.a("xyz");
        assert!(matches!(f.err("lstrcatA", &[d, s]), WinApiError::Memory(_)));
        assert_eq!(f.bytes(d, 4), b"ab\0\0".to_vec());
    }),
    ("lstrcatW", "appends wide text", |f| {
        let (d, s) = (f.buf(32), f.w("ab"));
        f.int("lstrcpyW", &[d, s]);
        let t = f.w("cd");
        f.int("lstrcatW", &[d, t]);
        assert_eq!(f.wstr(d), "abcd");
    }),
    ("lstrcatW", "NULL destination returns NULL", |f| {
        let s = f.w("x");
        assert_eq!(f.int("lstrcatW", &[0, s]), 0);
    }),
    // --- lstrcmp / lstrcmpi ----------------------------------------------
    ("lstrcmpA", "equal", |f| {
        let (a, b) = (f.a("Abc1"), f.a("Abc1"));
        assert_eq!(f.int("lstrcmpA", &[a, b]), 0);
    }),
    ("lstrcmpA", "letters ignore case before tie-break", |f| {
        let (a, b) = (f.a("a"), f.a("B"));
        assert_eq!(f.int("lstrcmpA", &[a, b]), -1);
        assert_eq!(f.int("lstrcmpA", &[b, a]), 1);
    }),
    ("lstrcmpA", "lowercase sorts before uppercase", |f| {
        let (a, b) = (f.a("a"), f.a("A"));
        assert_eq!(f.int("lstrcmpA", &[a, b]), -1);
        assert_eq!(f.int("lstrcmpA", &[b, a]), 1);
    }),
    ("lstrcmpA", "digits sort before letters", |f| {
        let (a, b) = (f.a("9"), f.a("a"));
        assert_eq!(f.int("lstrcmpA", &[a, b]), -1);
    }),
    ("lstrcmpA", "prefix sorts first", |f| {
        let (a, b) = (f.a("ab"), f.a("abc"));
        assert_eq!(f.int("lstrcmpA", &[a, b]), -1);
        let e = f.a("");
        assert_eq!(f.int("lstrcmpA", &[e, a]), -1);
    }),
    ("lstrcmpA", "non-alphanumeric input is Unsupported", |f| {
        let (a, b) = (f.a("a-b"), f.a("ab"));
        assert!(matches!(
            f.err("lstrcmpA", &[a, b]),
            WinApiError::Unsupported { .. }
        ));
    }),
    ("lstrcmpW", "orders wide strings", |f| {
        let (a, b) = (f.w("abc"), f.w("abd"));
        assert_eq!(f.int("lstrcmpW", &[a, b]), -1);
        assert_eq!(f.int("lstrcmpW", &[b, a]), 1);
    }),
    ("lstrcmpW", "equal", |f| {
        let (a, b) = (f.w("x1"), f.w("x1"));
        assert_eq!(f.int("lstrcmpW", &[a, b]), 0);
    }),
    ("lstrcmpW", "non-ASCII input is Unsupported", |f| {
        let (a, b) = (f.w("é"), f.w("e"));
        assert!(matches!(
            f.err("lstrcmpW", &[a, b]),
            WinApiError::Unsupported { .. }
        ));
    }),
    ("lstrcmpiA", "ignores case", |f| {
        let (a, b) = (f.a("aBc"), f.a("AbC"));
        assert_eq!(f.int("lstrcmpiA", &[a, b]), 0);
    }),
    ("lstrcmpiA", "orders", |f| {
        let (a, b) = (f.a("a"), f.a("B"));
        assert_eq!(f.int("lstrcmpiA", &[a, b]), -1);
        assert_eq!(f.int("lstrcmpiA", &[b, a]), 1);
    }),
    ("lstrcmpiA", "NULL pointer is a memory fault", |f| {
        let a = f.a("a");
        assert_eq!(
            f.err("lstrcmpiA", &[a, 0]),
            WinApiError::Memory(MemoryError::NullPointer)
        );
    }),
    ("lstrcmpiW", "ignores case", |f| {
        let (a, b) = (f.w("HELLO"), f.w("hello"));
        assert_eq!(f.int("lstrcmpiW", &[a, b]), 0);
    }),
    ("lstrcmpiW", "orders", |f| {
        let (a, b) = (f.w("b"), f.w("A"));
        assert_eq!(f.int("lstrcmpiW", &[a, b]), 1);
    }),
    // --- CharUpper / CharLower --------------------------------------------
    ("CharUpperA", "converts a string in place", |f| {
        let p = f.a("abcXYZ1");
        assert_eq!(f.int("CharUpperA", &[p]), p);
        assert_eq!(f.cstr(p), b"ABCXYZ1");
    }),
    ("CharUpperA", "single character value", |f| {
        assert_eq!(f.int("CharUpperA", &[97]), 65)
    }),
    ("CharUpperA", "Windows-1252 y-diaeresis maps to 0x9F", |f| {
        assert_eq!(f.int("CharUpperA", &[0xFF]), 0x9F)
    }),
    ("CharUpperA", "sharp s and micro are unchanged", |f| {
        assert_eq!(f.int("CharUpperA", &[0xDF]), 0xDF);
        assert_eq!(f.int("CharUpperA", &[0xB5]), 0xB5);
    }),
    ("CharUpperA", "Latin-1 letters", |f| {
        let p = f.raw(&[0xE9, 0x9A, 0]);
        f.int("CharUpperA", &[p]);
        assert_eq!(f.bytes(p, 2), vec![0xC9, 0x8A]);
    }),
    ("CharUpperW", "converts a string in place", |f| {
        let p = f.w("straße");
        assert_eq!(f.int("CharUpperW", &[p]), p);
        assert_eq!(f.wstr(p), "STRAßE");
    }),
    ("CharUpperW", "single character value", |f| {
        assert_eq!(f.int("CharUpperW", &[0xE9]), 0xC9)
    }),
    ("CharUpperW", "Greek", |f| {
        assert_eq!(f.int("CharUpperW", &[0x3C3]), 0x3A3)
    }),
    ("CharUpperW", "surrogate pairs are untouched", |f| {
        let p = f.w("\u{10428}a");
        f.int("CharUpperW", &[p]);
        assert_eq!(f.wstr(p), "\u{10428}A");
    }),
    ("CharLowerA", "converts a string in place", |f| {
        let p = f.a("ABCxyz1");
        assert_eq!(f.int("CharLowerA", &[p]), p);
        assert_eq!(f.cstr(p), b"abcxyz1");
    }),
    ("CharLowerA", "single character value", |f| {
        assert_eq!(f.int("CharLowerA", &[0xC4]), 0xE4)
    }),
    ("CharLowerA", "Windows-1252 S-caron", |f| {
        assert_eq!(f.int("CharLowerA", &[0x8A]), 0x9A)
    }),
    ("CharLowerW", "converts a string in place", |f| {
        let p = f.w("ÀBC");
        f.int("CharLowerW", &[p]);
        assert_eq!(f.wstr(p), "àbc");
    }),
    ("CharLowerW", "single character value", |f| {
        assert_eq!(f.int("CharLowerW", &[0x3A3]), 0x3C3)
    }),
    ("CharLowerW", "caseless characters are unchanged", |f| {
        assert_eq!(f.int("CharLowerW", &[0x4E2D]), 0x4E2D)
    }),
    // --- MultiByteToWideChar ------------------------------------------------
    ("MultiByteToWideChar", "UTF-8 size query", |f| {
        let s = f.raw("héllo".as_bytes());
        assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0, s, 6, 0, 0]), 5);
    }),
    ("MultiByteToWideChar", "-1 length includes the NUL", |f| {
        let s = f.raw(b"ab\0");
        let d = f.buf(6);
        assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0, s, -1, d, 3]), 3);
        assert_eq!(f.units(d, 3), vec![0x61, 0x62, 0]);
    }),
    ("MultiByteToWideChar", "explicit length adds no NUL", |f| {
        let s = f.raw(b"abc");
        let d = f.buf(6);
        assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0, s, 2, d, 3]), 2);
        assert_eq!(f.units(d, 3), vec![0x61, 0x62, 0]);
    }),
    (
        "MultiByteToWideChar",
        "small buffer sets ERROR_INSUFFICIENT_BUFFER",
        |f| {
            let s = f.raw(b"abc");
            let d = f.buf(4);
            assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0, s, 3, d, 2]), 0);
            assert_eq!(f.last(), 122);
            assert_eq!(f.bytes(d, 4), vec![0; 4]);
        },
    ),
    (
        "MultiByteToWideChar",
        "invalid flags set ERROR_INVALID_PARAMETER",
        |f| {
            let s = f.raw(b"abc");
            assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0x1, s, 3, 0, 0]), 0);
            assert_eq!(f.last(), 87);
        },
    ),
    ("MultiByteToWideChar", "zero length is invalid", |f| {
        let s = f.raw(b"abc");
        assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0, s, 0, 0, 0]), 0);
        assert_eq!(f.last(), 87);
    }),
    (
        "MultiByteToWideChar",
        "invalid UTF-8 is replaced by U+FFFD",
        |f| {
            let s = f.raw(&[0x61, 0xFF, 0x62]);
            let d = f.buf(6);
            assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 0, s, 3, d, 3]), 3);
            assert_eq!(f.units(d, 3), vec![0x61, 0xFFFD, 0x62]);
        },
    ),
    (
        "MultiByteToWideChar",
        "MB_ERR_INVALID_CHARS fails with 1113",
        |f| {
            let s = f.raw(&[0x61, 0xFF]);
            assert_eq!(f.int("MultiByteToWideChar", &[UTF8, 8, s, 2, 0, 0]), 0);
            assert_eq!(f.last(), 1113);
        },
    ),
    ("MultiByteToWideChar", "Windows-1252 euro and CP_ACP", |f| {
        let s = f.raw(&[0x80, 0x81]);
        let d = f.buf(4);
        assert_eq!(f.int("MultiByteToWideChar", &[CP1252, 0, s, 2, d, 2]), 2);
        assert_eq!(f.units(d, 2), vec![0x20AC, 0x81]);
        assert_eq!(f.int("MultiByteToWideChar", &[CP_ACP, 0, s, 2, 0, 0]), 2);
    }),
    (
        "MultiByteToWideChar",
        "unknown code page is Unsupported",
        |f| {
            let s = f.raw(b"a");
            assert!(matches!(
                f.err("MultiByteToWideChar", &[932, 0, s, 1, 0, 0]),
                WinApiError::Unsupported { .. }
            ));
        },
    ),
    ("MultiByteToWideChar", "US-ASCII high byte", |f| {
        let s = f.raw(&[0x80]);
        assert!(matches!(
            f.err("MultiByteToWideChar", &[ASCII, 0, s, 1, 0, 0]),
            WinApiError::Unsupported { .. }
        ));
        assert_eq!(f.int("MultiByteToWideChar", &[ASCII, 8, s, 1, 0, 0]), 0);
        assert_eq!(f.last(), 1113);
    }),
    ("MultiByteToWideChar", "MB_COMPOSITE is Unsupported", |f| {
        let s = f.raw(b"a");
        assert!(matches!(
            f.err("MultiByteToWideChar", &[CP1252, 2, s, 1, 0, 0]),
            WinApiError::Unsupported { .. }
        ));
    }),
    // --- WideCharToMultiByte -------------------------------------------------
    ("WideCharToMultiByte", "UTF-8 encodes the euro sign", |f| {
        let s = f.w("€");
        let d = f.buf(8);
        assert_eq!(
            f.int("WideCharToMultiByte", &[UTF8, 0, s, 1, d, 8, 0, 0]),
            3
        );
        assert_eq!(f.bytes(d, 3), vec![0xE2, 0x82, 0xAC]);
    }),
    (
        "WideCharToMultiByte",
        "size query with -1 includes the NUL",
        |f| {
            let s = f.w("€a");
            assert_eq!(
                f.int("WideCharToMultiByte", &[UTF8, 0, s, -1, 0, 0, 0, 0]),
                5
            );
        },
    ),
    (
        "WideCharToMultiByte",
        "small buffer sets ERROR_INSUFFICIENT_BUFFER",
        |f| {
            let s = f.w("abc");
            let d = f.buf(8);
            assert_eq!(
                f.int("WideCharToMultiByte", &[UTF8, 0, s, 3, d, 2, 0, 0]),
                0
            );
            assert_eq!(f.last(), 122);
        },
    ),
    (
        "WideCharToMultiByte",
        "Windows-1252 maps the euro to 0x80",
        |f| {
            let s = f.w("€é");
            let d = f.buf(4);
            assert_eq!(
                f.int("WideCharToMultiByte", &[CP1252, 0, s, 2, d, 4, 0, 0]),
                2
            );
            assert_eq!(f.bytes(d, 2), vec![0x80, 0xE9]);
        },
    ),
    (
        "WideCharToMultiByte",
        "best-fit substitution is Unsupported",
        |f| {
            let s = f.w("Δ");
            let d = f.buf(4);
            assert!(matches!(
                f.err("WideCharToMultiByte", &[CP1252, 0, s, 1, d, 4, 0, 0]),
                WinApiError::Unsupported { .. }
            ));
        },
    ),
    (
        "WideCharToMultiByte",
        "WC_NO_BEST_FIT_CHARS uses the default char",
        |f| {
            let s = f.w("aΔ");
            let (d, used) = (f.buf(4), f.buf(4));
            assert_eq!(
                f.int("WideCharToMultiByte", &[CP1252, 0x400, s, 2, d, 4, 0, used]),
                2
            );
            assert_eq!(f.bytes(d, 2), vec![b'a', b'?']);
            assert_eq!(f.u32_at(used), 1);
        },
    ),
    (
        "WideCharToMultiByte",
        "custom default character and unused flag",
        |f| {
            let s = f.w("ab");
            let (d, used, def) = (f.buf(4), f.cell(7), f.raw(b"#"));
            assert_eq!(
                f.int(
                    "WideCharToMultiByte",
                    &[ASCII, 0x400, s, 2, d, 4, def, used]
                ),
                2
            );
            assert_eq!(f.u32_at(used), 0);
            let t = f.w("\u{e9}");
            assert_eq!(
                f.int(
                    "WideCharToMultiByte",
                    &[ASCII, 0x400, t, 1, d, 4, def, used]
                ),
                1
            );
            assert_eq!(f.bytes(d, 1), b"#".to_vec());
        },
    ),
    (
        "WideCharToMultiByte",
        "UTF-8 rejects a default character",
        |f| {
            let (s, def) = (f.w("a"), f.raw(b"?"));
            assert_eq!(
                f.int("WideCharToMultiByte", &[UTF8, 0, s, 1, 0, 0, def, 0]),
                0
            );
            assert_eq!(f.last(), 87);
        },
    ),
    (
        "WideCharToMultiByte",
        "lone surrogate is replaced by U+FFFD in UTF-8",
        |f| {
            let s = f.raw(&[0x00, 0xD8]);
            let d = f.buf(4);
            assert_eq!(
                f.int("WideCharToMultiByte", &[UTF8, 0, s, 1, d, 4, 0, 0]),
                3
            );
            assert_eq!(f.bytes(d, 3), vec![0xEF, 0xBF, 0xBD]);
        },
    ),
    (
        "WideCharToMultiByte",
        "WC_ERR_INVALID_CHARS fails with 1113",
        |f| {
            let s = f.raw(&[0x00, 0xD8]);
            assert_eq!(
                f.int("WideCharToMultiByte", &[UTF8, 0x80, s, 1, 0, 0, 0, 0]),
                0
            );
            assert_eq!(f.last(), 1113);
        },
    ),
    (
        "WideCharToMultiByte",
        "WC_COMPOSITECHECK is Unsupported",
        |f| {
            let s = f.w("a");
            assert!(matches!(
                f.err("WideCharToMultiByte", &[CP1252, 0x200, s, 1, 0, 0, 0, 0]),
                WinApiError::Unsupported { .. }
            ));
        },
    ),
    (
        "WideCharToMultiByte",
        "WC_ERR_INVALID_CHARS is invalid for single-byte pages",
        |f| {
            let s = f.w("a");
            assert_eq!(
                f.int("WideCharToMultiByte", &[CP1252, 0x80, s, 1, 0, 0, 0, 0]),
                0
            );
            assert_eq!(f.last(), 87);
        },
    ),
    // --- last error -----------------------------------------------------------
    ("GetLastError", "starts at zero and is emulated", |f| {
        assert_eq!(f.int("GetLastError", &[]), 0)
    }),
    ("GetLastError", "is per logical thread", |f| {
        f.void("SetLastError", &[5]);
        f.ctx.thread = LogicalThread(2);
        assert_eq!(f.int("GetLastError", &[]), 0);
        f.ctx.thread = LogicalThread(0);
        assert_eq!(f.int("GetLastError", &[]), 5);
    }),
    ("SetLastError", "stores a DWORD", |f| {
        f.void("SetLastError", &[-1]);
        assert_eq!(f.int("GetLastError", &[]), 0xFFFF_FFFF);
    }),
    ("SetLastError", "last write wins", |f| {
        f.void("SetLastError", &[5]);
        f.void("SetLastError", &[0]);
        assert_eq!(f.int("GetLastError", &[]), 0);
    }),
    // --- version -----------------------------------------------------------------
    ("GetVersion", "default is Windows 10 build 19045", |f| {
        assert_eq!(f.int("GetVersion", &[]), (19045 << 16) | 10);
    }),
    ("GetVersion", "comes from the context, not the host", |f| {
        f.ctx.version = WindowsVersion {
            major: 6,
            minor: 1,
            build: 7601,
            ..WindowsVersion::windows10()
        };
        assert_eq!(f.int("GetVersion", &[]), (7601 << 16) | (1 << 8) | 6);
    }),
    ("GetVersionExA", "fills OSVERSIONINFOA", |f| {
        let p = f.buf(148);
        f.ctx.memory.write_u32(p as u64, 148).unwrap();
        assert_eq!(f.int("GetVersionExA", &[p]), 1);
        let v: Vec<u32> = (0..5).map(|i| f.u32_at(p + 4 * i)).collect();
        assert_eq!(v, vec![148, 10, 0, 19045, 2]);
        assert_eq!(f.bytes(p + 20, 128), vec![0; 128]);
    }),
    ("GetVersionExA", "fills OSVERSIONINFOEXA", |f| {
        f.ctx.version.service_pack_major = 3;
        f.ctx.version.product_type = 3;
        let p = f.buf(156);
        f.ctx.memory.write_u32(p as u64, 156).unwrap();
        assert_eq!(f.int("GetVersionExA", &[p]), 1);
        assert_eq!(f.bytes(p + 148, 8), vec![3, 0, 0, 0, 0, 0, 3, 0]);
    }),
    (
        "GetVersionExA",
        "wrong size sets ERROR_INVALID_PARAMETER",
        |f| {
            let p = f.buf(156);
            f.ctx.memory.write_u32(p as u64, 100).unwrap();
            assert_eq!(f.int("GetVersionExA", &[p]), 0);
            assert_eq!(f.last(), 87);
        },
    ),
    ("GetVersionExA", "service pack text", |f| {
        f.ctx.version.csd = "Service Pack 1".into();
        let p = f.buf(148);
        f.ctx.memory.write_u32(p as u64, 148).unwrap();
        f.int("GetVersionExA", &[p]);
        assert_eq!(f.cstr(p + 20), b"Service Pack 1");
    }),
    // --- identity ---------------------------------------------------------------------
    ("GetUserNameA", "copies the injected user name", |f| {
        let (b, n) = (f.buf(16), f.cell(16));
        assert_eq!(f.int("GetUserNameA", &[b, n]), 1);
        assert_eq!(f.cstr(b), b"alice");
        assert_eq!(f.u32_at(n), 6);
    }),
    (
        "GetUserNameA",
        "small buffer reports the needed size",
        |f| {
            let (b, n) = (f.buf(16), f.cell(3));
            assert_eq!(f.int("GetUserNameA", &[b, n]), 0);
            assert_eq!(f.last(), 122);
            assert_eq!(f.u32_at(n), 6);
            assert_eq!(f.bytes(b, 6), vec![0; 6]);
        },
    ),
    ("GetUserNameA", "exact fit succeeds", |f| {
        let (b, n) = (f.buf(6), f.cell(6));
        assert_eq!(f.int("GetUserNameA", &[b, n]), 1);
    }),
    (
        "GetComputerNameA",
        "copies the injected name without the NUL in the count",
        |f| {
            let (b, n) = (f.buf(16), f.cell(16));
            assert_eq!(f.int("GetComputerNameA", &[b, n]), 1);
            assert_eq!(f.cstr(b), b"BOX1");
            assert_eq!(f.u32_at(n), 4);
        },
    ),
    (
        "GetComputerNameA",
        "size equal to the length overflows",
        |f| {
            let (b, n) = (f.buf(16), f.cell(4));
            assert_eq!(f.int("GetComputerNameA", &[b, n]), 0);
            assert_eq!(f.last(), 111);
            assert_eq!(f.u32_at(n), 5);
        },
    ),
    ("GetComputerNameA", "size including the NUL succeeds", |f| {
        let (b, n) = (f.buf(5), f.cell(5));
        assert_eq!(f.int("GetComputerNameA", &[b, n]), 1);
        assert_eq!(f.u32_at(n), 4);
    }),
    // --- memory --------------------------------------------------------------------------
    ("RtlMoveMemory", "copies bytes", |f| {
        let (d, s) = (f.buf(8), f.raw(&[1, 2, 3, 4, 5, 6, 7, 8]));
        f.void("RtlMoveMemory", &[d, s, 4]);
        assert_eq!(f.bytes(d, 5), vec![1, 2, 3, 4, 0]);
    }),
    ("RtlMoveMemory", "handles overlap like memmove", |f| {
        let p = f.raw(&[1, 2, 3, 4, 5, 6, 7, 8]);
        f.void("RtlMoveMemory", &[p + 2, p, 4]);
        assert_eq!(f.bytes(p, 8), vec![1, 2, 1, 2, 3, 4, 7, 8]);
    }),
    (
        "RtlMoveMemory",
        "out-of-range copy faults without writing",
        |f| {
            let (d, s) = (f.buf(8), f.raw(&[9; 8]));
            assert!(matches!(
                f.err("RtlMoveMemory", &[d, s, 9]),
                WinApiError::Memory(_)
            ));
            assert_eq!(f.bytes(d, 8), vec![0; 8]);
        },
    ),
    ("RtlMoveMemory", "zero length never touches memory", |f| {
        f.void("RtlMoveMemory", &[0, 0, 0])
    }),
    ("RtlFillMemory", "fills a range", |f| {
        let p = f.buf(6);
        f.void("RtlFillMemory", &[p + 1, 3, 0xAB]);
        assert_eq!(f.bytes(p, 6), vec![0, 0xAB, 0xAB, 0xAB, 0, 0]);
    }),
    ("RtlFillMemory", "fill value must be a byte", |f| {
        let p = f.buf(2);
        assert!(matches!(
            f.err("RtlFillMemory", &[p, 2, 256]),
            WinApiError::BadArguments { .. }
        ));
    }),
    ("RtlFillMemory", "overrun faults without writing", |f| {
        let p = f.buf(2);
        assert!(matches!(
            f.err("RtlFillMemory", &[p, 3, 1]),
            WinApiError::Memory(_)
        ));
        assert_eq!(f.bytes(p, 2), vec![0, 0]);
    }),
    ("RtlZeroMemory", "zeroes a range", |f| {
        let p = f.raw(&[1, 2, 3, 4]);
        f.void("RtlZeroMemory", &[p, 3]);
        assert_eq!(f.bytes(p, 4), vec![0, 0, 0, 4]);
    }),
    ("RtlZeroMemory", "negative length is rejected", |f| {
        let p = f.buf(2);
        assert!(matches!(
            f.err("RtlZeroMemory", &[p, -1]),
            WinApiError::BadArguments { .. }
        ));
    }),
    ("RtlZeroMemory", "NULL with a length faults", |f| {
        assert_eq!(
            f.err("RtlZeroMemory", &[0, 1]),
            WinApiError::Memory(MemoryError::NullPointer)
        );
    }),
];

#[test]
fn run_all_cases() {
    for (sym, name, case) in CASES {
        let result = std::panic::catch_unwind(|| case(&mut Fx::new()));
        assert!(result.is_ok(), "case failed: {sym}: {name}");
    }
}

#[test]
fn cases_per_symbol_match_the_catalog() {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (sym, _, _) in CASES {
        *counts.entry(sym).or_default() += 1;
    }
    for row in catalog()
        .iter()
        .filter(|r| r.status == vba_winapi::catalog::Status::Implemented)
    {
        let n: usize = row
            .exports()
            .iter()
            .map(|e| counts.remove(e.as_str()).unwrap_or(0))
            .sum();
        assert_eq!(
            n, row.tests,
            "{}!{}: catalog says {} tests, found {n}",
            row.dll, row.symbol, row.tests
        );
    }
    assert!(
        counts.is_empty(),
        "cases for symbols that are not cataloged as implemented: {counts:?}"
    );
}

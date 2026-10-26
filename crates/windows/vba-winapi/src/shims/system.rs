//! System identity and state: GetLastError, SetLastError, GetVersion,
//! GetVersionExA, GetUserNameA (advapi32), GetComputerNameA.
//!
//! Docs: learn.microsoft.com/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror,
//! ...-setlasterror, .../sysinfoapi/nf-sysinfoapi-getversion, ...-getversionexa,
//! .../winbase/nf-winbase-getusernamea, .../sysinfoapi/nf-sysinfoapi-getcomputernamea.
//! All values come from the injected context, never from the real machine.

use super::{Args, Shim, invalid_parameter};
use crate::{CallContext, WinApiError};
use vba_ffi::{Codepage, FieldValue, NativeReturn, UdtFieldType, UdtLayout, Unmappable};
use vba_value::VbaString;

pub(super) const SHIMS: [Shim; 6] = [
    Shim {
        dll: "kernel32",
        symbol: "GetLastError",
        arity: 0,
        run: get_last_error,
    },
    Shim {
        dll: "kernel32",
        symbol: "SetLastError",
        arity: 1,
        run: set_last_error,
    },
    Shim {
        dll: "kernel32",
        symbol: "GetVersion",
        arity: 0,
        run: get_version,
    },
    Shim {
        dll: "kernel32",
        symbol: "GetVersionExA",
        arity: 1,
        run: get_version_ex_a,
    },
    Shim {
        dll: "advapi32",
        symbol: "GetUserNameA",
        arity: 2,
        run: get_user_name_a,
    },
    Shim {
        dll: "kernel32",
        symbol: "GetComputerNameA",
        arity: 2,
        run: get_computer_name_a,
    },
];

use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_INSUFFICIENT_BUFFER};

fn get_last_error(ctx: &mut CallContext, _: &Args) -> Result<NativeReturn, WinApiError> {
    Ok(NativeReturn::Int(i64::from(ctx.last_error())))
}

fn set_last_error(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    ctx.set_last_error(a.u32(0)?);
    Ok(NativeReturn::Void)
}

/// `GetVersion`: low byte major, next byte minor, high word build number
/// (the high bit is clear on the NT platform, so the build is reported).
fn get_version(ctx: &mut CallContext, _: &Args) -> Result<NativeReturn, WinApiError> {
    let v = &ctx.version;
    let mut word = (v.minor & 0xFF) << 8 | (v.major & 0xFF);
    if v.platform_id == 2 {
        word |= (v.build & 0x7FFF) << 16;
    } else {
        word |= 0x8000_0000;
    }
    Ok(NativeReturn::Int(i64::from(word)))
}

/// `OSVERSIONINFOA` (148 bytes) or `OSVERSIONINFOEXA` (156 bytes), laid out by
/// the same UDT rules VBA code uses to declare them.
fn os_version_layout(ctx: &CallContext, ex: bool) -> UdtLayout {
    let mut f = vec![
        ("dwOSVersionInfoSize".to_string(), UdtFieldType::Long),
        ("dwMajorVersion".into(), UdtFieldType::Long),
        ("dwMinorVersion".into(), UdtFieldType::Long),
        ("dwBuildNumber".into(), UdtFieldType::Long),
        ("dwPlatformId".into(), UdtFieldType::Long),
        ("szCSDVersion".into(), UdtFieldType::FixedString(128)),
    ];
    if ex {
        f.extend([
            ("wServicePackMajor".to_string(), UdtFieldType::Integer),
            ("wServicePackMinor".into(), UdtFieldType::Integer),
            ("wSuiteMask".into(), UdtFieldType::Integer),
            ("wProductType".into(), UdtFieldType::Byte),
            ("wReserved".into(), UdtFieldType::Byte),
        ]);
    }
    UdtLayout::new(ctx.target, f).expect("static layout is valid")
}

/// Fails with `ERROR_INVALID_PARAMETER` when `dwOSVersionInfoSize` is neither
/// 148 nor 156.
fn get_version_ex_a(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let p = a.ptr(0)?;
    let size = ctx.memory.read_u32(p)? as usize;
    let ex = match size {
        148 => false,
        156 => true,
        _ => return Ok(invalid_parameter(ctx)),
    };
    let layout = os_version_layout(ctx, ex);
    debug_assert_eq!(layout.size(), size);
    let v = ctx.version.clone();
    let csd = Codepage::Windows1252
        .encode(
            &v.csd.encode_utf16().take(127).collect::<Vec<_>>(),
            Unmappable::Replace(b'?'),
            false,
        )
        .expect("replacement never fails")
        .bytes;
    let mut csd_units: Vec<u16> = csd.iter().map(|b| u16::from(*b)).collect();
    csd_units.resize(128, 0);
    let mut vals = vec![
        FieldValue::Int(size as i64),
        FieldValue::Int(i64::from(v.major)),
        FieldValue::Int(i64::from(v.minor)),
        FieldValue::Int(i64::from(v.build)),
        FieldValue::Int(i64::from(v.platform_id)),
        FieldValue::Str(VbaString::from_units(csd_units)),
    ];
    if ex {
        vals.extend([
            FieldValue::Int(i64::from(v.service_pack_major)),
            FieldValue::Int(i64::from(v.service_pack_minor)),
            FieldValue::Int(i64::from(v.suite_mask)),
            FieldValue::Int(i64::from(v.product_type)),
            FieldValue::Int(0),
        ]);
    }
    let bytes = layout
        .encode(&vals, Codepage::Latin1)
        .map_err(|_| a.bad(0, "version string is not encodable"))?;
    ctx.memory.write(p, &bytes)?;
    Ok(NativeReturn::Int(1))
}

fn ansi_name(name: &str) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    Codepage::Windows1252
        .encode(&units, Unmappable::Replace(b'?'), false)
        .expect("replacement never fails")
        .bytes
}

/// Success: copies the name plus NUL, `*pcbBuffer` = characters incl. NUL.
/// Too small: returns 0, `ERROR_INSUFFICIENT_BUFFER`, `*pcbBuffer` = needed.
fn get_user_name_a(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let (buf, size_ptr) = (a.ptr(0)?, a.ptr(1)?);
    let mut name = ansi_name(ctx.identity.user_name());
    name.push(0);
    let avail = ctx.memory.read_u32(size_ptr)? as usize;
    if avail < name.len() {
        ctx.memory.write_u32(size_ptr, name.len() as u32)?;
        ctx.set_last_error(ERROR_INSUFFICIENT_BUFFER);
        return Ok(NativeReturn::Int(0));
    }
    ctx.memory.write(buf, &name)?;
    ctx.memory.write_u32(size_ptr, name.len() as u32)?;
    Ok(NativeReturn::Int(1))
}

/// Success: `*nSize` = characters copied, excluding the NUL. Too small:
/// returns 0, `ERROR_BUFFER_OVERFLOW`, `*nSize` = needed including the NUL.
fn get_computer_name_a(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let (buf, size_ptr) = (a.ptr(0)?, a.ptr(1)?);
    let mut name = ansi_name(ctx.identity.computer_name());
    let chars = name.len();
    name.push(0);
    let avail = ctx.memory.read_u32(size_ptr)? as usize;
    if avail < name.len() {
        ctx.memory.write_u32(size_ptr, name.len() as u32)?;
        ctx.set_last_error(ERROR_BUFFER_OVERFLOW);
        return Ok(NativeReturn::Int(0));
    }
    ctx.memory.write(buf, &name)?;
    ctx.memory.write_u32(size_ptr, chars as u32)?;
    Ok(NativeReturn::Int(1))
}

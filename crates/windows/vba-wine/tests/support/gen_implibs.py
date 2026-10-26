#!/usr/bin/env python3
"""Write MinGW-style .def files from the export tables of PE DLLs.

usage: gen_implibs.py DLL_DIR OUT_DIR NAME...

For each NAME, reads DLL_DIR/NAME.dll and writes OUT_DIR/NAME.def listing every
named export (forwarders included: the loader resolves them). Feed each file to
`llvm-dlltool -m i386:x86-64 -D NAME.dll -d NAME.def -l NAME.lib` to get an
import library for rust-lld when no Windows SDK or mingw is installed.
"""
import struct
import sys
from pathlib import Path


def exports(data):
    (pe,) = struct.unpack_from("<I", data, 0x3C)
    assert data[pe:pe + 4] == b"PE\0\0", "not a PE file"
    (nsec,) = struct.unpack_from("<H", data, pe + 6)
    (optsz,) = struct.unpack_from("<H", data, pe + 20)
    opt = pe + 24
    (magic,) = struct.unpack_from("<H", data, opt)
    dd = opt + (112 if magic == 0x20B else 96)
    exp_rva, _exp_size = struct.unpack_from("<II", data, dd)
    secs = []
    for i in range(nsec):
        o = opt + optsz + i * 40
        vsz, va, rsz, raw = struct.unpack_from("<IIII", data, o + 8)
        secs.append((va, max(vsz, rsz), raw))

    def off(rva):
        for va, sz, raw in secs:
            if va <= rva < va + sz:
                return raw + rva - va
        raise ValueError(hex(rva))

    if exp_rva == 0:
        return []
    e = off(exp_rva)
    _nfunc, nnames, _aof, aon, _aoo = struct.unpack_from("<IIIII", data, e + 20)
    out = []
    for i in range(nnames):
        (nrva,) = struct.unpack_from("<I", data, off(aon) + 4 * i)
        s = off(nrva)
        out.append(data[s:data.index(b"\0", s)].decode("ascii"))
    return out


def main():
    dll_dir, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    for name in sys.argv[3:]:
        names = exports((dll_dir / f"{name}.dll").read_bytes())
        body = "".join(f"  {n}\n" for n in names)
        (out_dir / f"{name}.def").write_text(f"LIBRARY {name}.dll\nEXPORTS\n{body}")
        print(f"{name}: {len(names)} exports")


main()

#!/bin/sh
# Generate import libraries (*.lib) for Wine's builtin PE DLLs.
# usage: make-implibs.sh WINE_DLL_DIR OUT_DIR
# Needs python3 and llvm-dlltool (set DLLTOOL to override the lookup).
set -eu
dlls=$1
out=$2
here=$(cd "$(dirname "$0")" && pwd)
dlltool=${DLLTOOL:-$(command -v llvm-dlltool || true)}
[ -n "$dlltool" ] || { echo "llvm-dlltool not found (set DLLTOOL)" >&2; exit 1; }
libs="kernel32 ntdll userenv ws2_32 dbghelp msvcrt ucrtbase"
mkdir -p "$out/defs" "$out/lib"
# shellcheck disable=SC2086
python3 -I "$here/gen_implibs.py" "$dlls" "$out/defs" $libs
for n in $libs; do
  "$dlltool" -m i386:x86-64 -D "$n.dll" -d "$out/defs/$n.def" -l "$out/lib/$n.lib"
done

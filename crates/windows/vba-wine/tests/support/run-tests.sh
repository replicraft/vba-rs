#!/bin/sh
# Build vba-wine-host.exe without a Windows SDK and run the Wine-gated tests.
# See "Reproducing the live tests" in docs/interop/wine-bridge.md.
#
# Environment (all optional):
#   REPO        vba-rs checkout to test (default: the checkout containing this script)
#   WINE_BIN    Wine bin dir (default: /Applications/Wine Stable.app/Contents/Resources/wine/bin)
#   WORK        scratch/output dir (default: $REPO/.cache/wine, gitignored)
#   DLLTOOL     path to llvm-dlltool (default: found on PATH or in Homebrew llvm)
#   KEEP_TARGET set to 1 to keep the cargo target dir
set -eu

here=$(cd "$(dirname "$0")" && pwd)
REPO=${REPO:-$(cd "$here/../../../../.." 2>/dev/null && pwd)}
WORK=${WORK:-$REPO/.cache/wine}
WINE_BIN=${WINE_BIN:-/Applications/Wine Stable.app/Contents/Resources/wine/bin}
DLL_DIR=${DLL_DIR:-$(dirname "$WINE_BIN")/lib/wine/x86_64-windows}
TARGET=x86_64-pc-windows-msvc

if [ -z "${DLLTOOL:-}" ]; then
  DLLTOOL=$(command -v llvm-dlltool || true)
  [ -n "$DLLTOOL" ] || DLLTOOL=$(ls /opt/homebrew/opt/llvm*/bin/llvm-dlltool /opt/homebrew/Cellar/llvm*/*/bin/llvm-dlltool 2>/dev/null | head -n 1 || true)
  [ -n "$DLLTOOL" ] || { echo "llvm-dlltool not found; set DLLTOOL" >&2; exit 1; }
fi
export DLLTOOL
for t in "$WINE_BIN/wine" "$WINE_BIN/wineserver" "$DLL_DIR/kernel32.dll"; do
  [ -e "$t" ] || { echo "missing $t" >&2; exit 1; }
done
command -v clang >/dev/null || { echo "clang not found" >&2; exit 1; }
rustup target add "$TARGET" >/dev/null 2>&1 || true

mkdir -p "$WORK"

# 1. Import libraries for Wine's builtin DLLs (once).
[ -f "$WORK/lib/ucrtbase.lib" ] || "$here/make-implibs.sh" "$DLL_DIR" "$WORK"

# 2. Startup shim standing in for the Microsoft C runtime (once).
[ -f "$WORK/startup.obj" ] || clang --target=x86_64-pc-windows-msvc -O1 -c "$here/startup.c" -o "$WORK/startup.obj"

# 3. Wine prefix whose side effects stay inside it (once).
export WINEPREFIX="$WORK/prefix"
export PATH="$WINE_BIN:$PATH"
export WINEDEBUG=-all WINEDLLOVERRIDES="mscoree,mshtml="
if [ ! -f "$WINEPREFIX/system.reg" ]; then
  "$WINE_BIN/wineboot" -i >/dev/null 2>&1 || true
  "$WINE_BIN/wineserver" -w || true
fi
# Replace links to the user's real folders (and /Volumes) by empty directories.
udir="$WINEPREFIX/drive_c/users"
if [ -d "$udir" ]; then
  find "$udir" -type l | while IFS= read -r l; do
    rm -f "$l"
    mkdir -p "$l"
  done
fi
# Drop every drive letter except C: (Z: is recreated by Wine as a link to /).
for d in "$WINEPREFIX/dosdevices"/*; do
  [ -L "$d" ] || continue
  case $(basename "$d") in c:|z:) ;; *) rm -f "$d" ;; esac
done

# 4. Build the helper (test-only recipe) in a throwaway target dir.
rust_lld_dir=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$WORK/target"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="$rust_lld_dir/rust-lld"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C panic=abort -C linker-flavor=lld-link -C link-arg=/nodefaultlib -C link-arg=msvcrt.lib -C link-arg=ucrtbase.lib -C link-arg=/libpath:$WORK/lib -C link-arg=$WORK/startup.obj -C link-arg=/entry:mainCRTStartup -C link-arg=/subsystem:console"
cleanup() { [ "${KEEP_TARGET:-0}" = 1 ] || rm -rf "$WORK/target"; }
trap cleanup EXIT
(cd "$REPO" && cargo build -p vba-wine-host --target "$TARGET")
cp "$WORK/target/$TARGET/debug/vba-wine-host.exe" "$WORK/vba-wine-host.exe"

# 5. Run the live tests against it. The test binary is built for the host, so
# the target-specific flags above do not apply (host builds go to the same dir).
unset CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER
export VBA_WINE="$WINE_BIN/wine"
export VBA_WINE_HELPER="$WORK/vba-wine-host.exe"
export VBA_WINE_PREFIX="$WINEPREFIX"
(cd "$REPO" && cargo test -p vba-wine --test wine_live -- --test-threads=1 "$@")

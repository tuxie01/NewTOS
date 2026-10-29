#!/usr/bin/env bash
#
# build-cflib.sh -- build cflib-21 into a NewTOS/AArch64 cflib.a and link the
# cftest cflib client into a native program (.NCP).
#
# cflib is a dialog/window/form helper library built ON TOP of gemlib's AES/VDI
# bindings and mintlib's libc. Unlike mintlib/gemlib it has NO own trap/asm
# boundary: it is pure C that calls appl_init/form_do/objc_draw/... from gemlib.
# The port is therefore just "compile cflib against the already-ported gemlib.a
# + libc.a and fix only what the AArch64/LP64 compiler rejects" (docs/MEMORY.md
# ADR-019, sdk/gemlib/build-gemlib.sh, sdk/libc/build-libc.sh).
#
# The ONE file that matched a trap/asm grep is fontsel.c: its __GNUC__ branch is
# m68k inline assembly that ONLY marshals the external "xFSL" font-selector call
# to the m68k Pure-C word-push ABI (a `_WORD` arg pushed as a 16-bit word). It
# is NOT a system trap -- the selector entry points are ordinary __CDECL cookie
# function pointers. On AArch64 the plain indirect call (the file's own #else
# branch) is correct, so the overlay gates the asm block off under -D__NEWTOS__.
#
# NewTOS-specific deltas from the pristine m68k build (all in sdk/cflib/overlay/,
# applied over the freshly extracted tree by the preamble below):
#   - fontsel.c : gate the m68k xfsl_* inline-asm macros off under __NEWTOS__ so
#                 the direct-call (#else) branch is used (arch boundary).
#   - inline.rsh: the portab.h guard skipped <portab.h> only for m68k GNU
#                 (__GNUC__ && __mc68000__); extend it to skip under __NEWTOS__
#                 too (our AArch64 GNU/gemlib build gets its types from mt_gem.h
#                 and ships no <portab.h>). Arch-detection fix only.
#   - cookie.c  : patch site SDK05 -- getcookie() builds the cookie tag from the
#                 4-character name with shifts (first character in bits 31..24,
#                 the whole long defined) instead of upstream's char[4]/long
#                 union, which on little-endian LP64 handed Getcookie the name
#                 byte-reversed in the low half of an 8-byte long and found no
#                 cookie (TASK/SDK05; host test sdk/cflib/tests/cookie-host.sh).
#
# WHICH cflib, AND WHY IT IS PINNED BY COMMIT (TASK/PORT03)
#
# cflib carries the version number 0.21.0 since 2004 and has never bumped it --
# the number says nothing about the state of the sources. What we build is
# therefore pinned by COMMIT, the way references/emutos and references/freemint
# are, and vendored as `cflib-21-<commit>.tar.xz`:
#
#   github.com/freemint/cflib, commit ce57817c3f68c7d346d1081768505e81acaee807
#   ("Replace last remaining tho-otto.de references with my server", 2026-07-14),
#   packed with `git archive --prefix=cflib-21/ ce57817 | xz -9`.
#
# It replaces tho-otto.de's source snapshot `download/mint/cflib-21.tar.xz`
# (2024-04-30, sha256 7f0d69f4 3f450df1 f0a4699b da8738f5 d8f7cc31 257e0db5
# 31b8e8ba 4062e349), which is 13 commits older. Four compiled files differ, all
# fixes we need or want: fsnorm.c (make_normalpath keeps the trailing backslash
# -- the PORT03 defect), fsexists.c + fexists.c (path_exists/file_exists probe
# with Fsfirst+DTA instead of Fattrib, and strip a trailing backslash first),
# fontsel.c (a workstation leak on the FONTSELECT path). Everything else that
# moved is CI scripting, the Makefile and README -- none of which this script
# uses; it drives the COBJS manifest in SRCFILES directly.
#
# The dated snapshots on the tho-otto mirror are NOT an alternative: the newest
# SOURCE tarball there is the 2024-04-30 one we just left, and the newer
# /snapshots/cflib/*.tar.bz2 are m68k BINARY dev packages (prebuilt libcflib.a).
# The git repository is the only source carrying the fixes, so a commit pin --
# not a branch, not "latest" -- is what makes the build reproducible.
#
# The linker pulls only the archive members it needs, so cflib.a is compiled
# from the whole COBJS manifest (SRCFILES); a file that fails to compile for
# AArch64/LP64 is skipped and logged, and a *needed* one surfaces as an
# undefined symbol at link time.
#
# Usage:  sdk/cflib/build-cflib.sh [client.c] [output.NCP]
#   client.c   defaults to sdk/cflib/tests/cftest.c
#   output.NCP defaults to <builddir>/cftest.NCP
set -uo pipefail

SDK="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"      # .../sdk
CF="$SDK/cflib"
GEM="$SDK/gemlib"
LIBC="$SDK/libc"
TOP="$CF/cflib-21"                                           # cflib top_srcdir
GEMTOP="$GEM/gemlib-0.44.0"                                  # gemlib top_srcdir
CLIENT="${1:-$CF/tests/cftest.c}"
BUILD="$CF/build"
OUT="${2:-$BUILD/$(basename "${CLIENT%.c}").NCP}"
OBJ="$BUILD/obj"
mkdir -p "$OBJ"

# --- ensure the two libraries cflib is built against are present ------------
# cflib.a is compiled against gemlib's AND mintlib's headers, so both must be
# current before a single cflib object is produced -- that dependency, and what
# "out of date" means for each library, is declared once in sdk/lib-stale.sh.
# `lib_ensure gemlib` walks libc -> gemlib in that order.
. "$SDK/lib-stale.sh"
lib_ensure gemlib || exit 1
MINT="$LIBC/mintlib/include"
[ -d "$MINT" ] || { echo "error: mintlib headers missing ($MINT); run build-libc.sh" >&2; exit 1; }
[ -d "$GEMTOP" ] || { echo "error: gemlib headers missing ($GEMTOP); run build-gemlib.sh" >&2; exit 1; }

# --- ensure the cflib source tree is present (extract tarball + overlay) ------
# The pristine cflib source is vendored as a commit-pinned TARBALL (see the
# header); our NewTOS edits live in sdk/cflib/overlay/ (mirroring the cflib
# layout). The extracted sdk/cflib/cflib-21/ tree is a build artifact
# (gitignored). Set CFLIB_REFRESH=1 to force a clean re-extract.
#
# The tree is ALSO re-extracted when the vendored tarball is newer than it.
# Without that test, changing the pin left every existing working copy building
# the old sources in silence -- the extracted tree is not tracked, so nothing
# else would ever notice (PORT03; sdk/lib-stale.sh watches the tarball for the
# matching archive-level test).
TARBALL="$CF/cflib-21-ce57817.tar.xz"
OVERLAY="$CF/overlay"
if [ "${CFLIB_REFRESH:-0}" = "1" ]; then rm -rf "$TOP"; fi
if [ -d "$TOP" ] && [ "$TARBALL" -nt "$TOP" ]; then
  echo "vendored cflib tarball is newer than $TOP -- re-extracting"
  rm -rf "$TOP"
fi
if [ ! -d "$TOP" ]; then
  [ -f "$TARBALL" ] || { echo "error: pristine tarball $TARBALL missing" >&2; exit 1; }
  echo "extracting pristine cflib from $(basename "$TARBALL")…"
  tmp="$(mktemp -d)"
  tar xf "$TARBALL" -C "$tmp"
  mv "$tmp/cflib-21" "$TOP"
  rmdir "$tmp" 2>/dev/null || true
  # tar restores the ARCHIVE's directory timestamp (for a `git archive` tarball
  # that is the commit date), which would be older than the tarball file itself
  # and make the -nt test above fire on every single run. Stamp the tree with
  # the time it was extracted instead.
  touch "$TOP"
fi
# Always (re-)apply the NewTOS overlay so our retarget is present after any extract.
if [ -d "$OVERLAY" ]; then
  ( cd "$OVERLAY" && find . -type f -print0 | while IFS= read -r -d '' f; do
      mkdir -p "$TOP/$(dirname "$f")"
      cp "$f" "$TOP/$f"
    done )
fi

# --- toolchain ---------------------------------------------------------------
CLANG="${CLANG:-clang}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "error: clang not found" >&2; exit 1; }
RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"   # LLVM tools: PATH / LLVM packages first, pinned Rust sysroot as fallback (SDK01 I, K)
AR="${AR:-$(newtos_tool llvm-ar)}"
LLD="${LLD:-$(newtos_tool ld.lld)}"
READOBJ="$(newtos_tool llvm-readobj)"
OBJDUMP="$(newtos_tool llvm-objdump)"
for t in "$AR" "$LLD"; do
    [ -x "$t" ] || { echo "error: missing LLVM tool: $t" >&2; exit 1; }
done

# --- flags -------------------------------------------------------------------
# cflib's own build uses -I$(top_srcdir) (all its headers live in the top dir)
# and -O2. We add the freestanding/no-libc/-nostdinc set, the ported gemlib and
# mintlib include paths, and -D__NEWTOS__ to select the retargeted overlay paths
# (fontsel.c direct call, inline.rsh no-portab). -Wno-macro-redefined silences
# cflib's intentional TRUE/FALSE/min/max/abs re-defines.
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2
        -fPIE -fno-builtin -nostdinc -D__NEWTOS__
        -Wno-macro-redefined)
INCS=(-I"$TOP" -I"$GEMTOP" -I"$MINT" -isystem "$RES")

echo "clang : $($CLANG --version | head -1)"
echo "ar    : $AR"
echo "lld   : $LLD"
echo

# --- authoritative file list: expand cflib's own COBJS manifest --------------
# SRCFILES defines COBJS (the compiled objects of libcflib.a). Expand it with
# make (SRCFILES has no rules needing the cross compiler).
COBJS_MK="$BUILD/print_cobjs.mk"
COBJS_LIST="$BUILD/cobjs.txt"
{ echo 'empty ='; echo 'include SRCFILES'; echo 'print:'; printf '\t@printf "%%s\\n" $(COBJS)\n'; } > "$COBJS_MK"
( cd "$TOP" && make -f "$COBJS_MK" print 2>/dev/null ) | grep -E '\.c$' | sort -u > "$COBJS_LIST"
n_cobjs=$(wc -l < "$COBJS_LIST" | tr -d ' ')
[ "$n_cobjs" -gt 0 ] || { echo "error: could not expand COBJS from SRCFILES" >&2; exit 1; }
echo "COBJS manifest: $n_cobjs files"

# --- compile cflib into objects (best-effort) --------------------------------
rm -f "$OBJ"/*.o "$OBJ"/*.err
: > "$BUILD/compiled.txt"; : > "$BUILD/failed.txt"
: > "$BUILD/failed-err.txt"   # "<source><TAB><its .err>" -- lib_check_failures
                              # quotes the diagnostic of an UNRECORDED failure
ok=0; fail=0
while IFS= read -r c; do
    [ -n "$c" ] || continue
    src="$TOP/$c"
    [ -e "$src" ] || { echo "$c (missing)" >> "$BUILD/failed.txt"; fail=$((fail+1)); continue; }
    b="${c%.c}"
    o="$OBJ/${b}.o"
    if "$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -c "$src" -o "$o" 2>"$OBJ/${b}.err"; then
        echo "$c" >> "$BUILD/compiled.txt"; ok=$((ok+1))
    else
        echo "$c" >> "$BUILD/failed.txt"; fail=$((fail+1)); rm -f "$o"
        printf '%s\t%s\n' "$c" "$OBJ/${b}.err" >> "$BUILD/failed-err.txt"
    fi
done < "$COBJS_LIST"

# --- the failure list is REPORTED, and measured against the record -----------
# This script already printed its list; what it could not do was tell a KNOWN
# gap from a NEW one (TASK/T02). cflib's record is empty, so any failure at all
# stops the build -- before the archive is written, so a silently incomplete
# cflib.a is never left behind for the next run to reuse.
echo "compiled OK=$ok  skipped=$fail  (of $n_cobjs)"
lib_check_failures "cflib" "$BUILD/failed.txt" "$CF/expected-failures.txt" \
                   "$BUILD/failed-err.txt" || exit 1

# --- archive -----------------------------------------------------------------
rm -f "$CF/cflib.a"
"$AR" cr "$CF/cflib.a" "$OBJ"/*.o || exit 1
echo "archived -> $CF/cflib.a ($(wc -c < "$CF/cflib.a" | tr -d ' ') bytes)"
echo

# --- CRT + client ------------------------------------------------------------
# Reuse the libc-aware crt0 (sets SP from p_hitpa, inits the malloc arena, calls
# main) and the NewTOS TPA linker script.
"$CLANG" "${CFLAGS[@]}" -I"$SDK/include" -I"$MINT" \
    -c "$LIBC/crt0.S" -o "$BUILD/crt0.o" || exit 1
"$CLANG" "${CFLAGS[@]}" "${INCS[@]}" \
    -c "$CLIENT" -o "$BUILD/client.o" 2>"$BUILD/client.err" \
    || { echo "client compile FAILED:"; cat "$BUILD/client.err"; exit 1; }

# --- link (crt0 + client + cflib.a + gemlib.a + libc.a) ----------------------
# cflib -> gemlib -> libc is the dependency order; wrap the three archives in a
# group so any residual back-reference between them still resolves.
newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none --why-extract="$BUILD/why.txt" \
    "$BUILD/crt0.o" "$BUILD/client.o" \
    --start-group "$CF/cflib.a" "$GEM/gemlib.a" "$LIBC/libc.a" --end-group \
    -o "$OUT" 2>"$BUILD/link.err"
lr=$?

if [ $lr -eq 0 ]; then
    echo "LINK OK -> $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
    "$READOBJ" --file-headers "$OUT" | grep -iE 'Type:|Machine:|Entry:'
    echo
    echo "GEM trap sites (svc #2) reachable from the linked cflib program:"
    "$OBJDUMP" -d "$OUT" | grep -iE '<aes>:|<vdi>:|svc[[:space:]]+#0x2' | sed 's/^/    /' | head
else
    echo "LINK FAILED -- residual undefined symbols (blockers):"
    grep -oE "undefined symbol: [^ ]+" "$BUILD/link.err" | awk '{print $3}' | sort -u | sed 's/^/    /'
    echo
    echo "See $BUILD/why.txt for the archive pull chains that reference them."
    exit 1
fi

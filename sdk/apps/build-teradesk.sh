#!/usr/bin/env bash
#
# build-teradesk.sh -- compile TeraDesk (tho-otto/aRTS TeraDesk 4.08, a pure-C
# GEM desktop replacement) into a native NewTOS/AArch64 **LP64** GEM application
# (DESKTOP.NAP, per ADR-018) against the ported mintlib (libc.a) + gemlib
# (gemlib.a). TeraDesk bundles its own dialog/utility libraries
# (library/xdialog + library/utility), so it does NOT need cflib -- unlike QED.
#
# LP64 is the only app ABI: the earlier ILP32 build path (ADR-020) was the
# corruption cause, recompiled GEM apps now run as native 64-bit, and the ILP32
# build scripts were retired on 2026-09-04. This script is a second, independent
# EL0 GEM-app validator of the app-facing AES/VDI/GEMDOS svc path besides QED: if
# TeraDesk renders correctly, AES/VDI is sound; if it breaks like QED did, the
# bug is in the kernel svc path, not in any one app.
#
# PATCH mechanism (same as build-qed.sh, extended to HEADERS): a file under
# sdk/overlays/teradesk/ with the same RELATIVE path (e.g.
# library/utility/nf_ops.c, window.h) replaces the vendored file at compile
# time; TeraDesk's upstream tree (embedded git checkout, freemint/teradesk) is
# never edited. Because quoted #include "..." always searches the including
# file's own directory first, header patches cannot work via -I; instead the
# build compiles from a SHADOW TREE (build/tree/ = symlinks to the vendored
# files, patch symlinks layered on top), so .c and .h patches apply uniformly
# and consistently across all translation units. The eight patch files today
# (each file's header says why): library/utility/nf_ops.c (m68k asm: NatFeats
# probe -> "absent"); environm.c, window.h, library/xdialog/library.h, icon.c,
# rsc_load.c, main.c, file.c (LP64: 32-bit TOS fields/words, struct size and
# alignment). Rule: a problem every LP64 port would meet (library, header,
# flag) is fixed ONCE in the SDK (gemlib/libc overlays, newtos-arch, crt0);
# what only TeraDesk needs is a patch file here, with a header saying why.
# Do not edit the vendored tree.
#
# TeraDesk's own build (Makefile + support/config.am) uses, for a cross build:
#   OPTS   = -Os -fomit-frame-pointer
#   CFLAGS = $(OPTS) -I library/xdialog -I library/utility <warnings> -m68000
#   LIBS   = -lgem
# We replace the m68k flag bundle with our AArch64 freestanding set (matching
# the other build-*.sh), keep -Os, and add -D__NEWTOS__ so the gemlib overlay
# headers select their retargeted (svc, direct-call) paths.
#
#   tree/objs : sdk/apps/teradesk/build/
#   output    : sdk/apps/teradesk/build/DESKTOP.NAP  (ELF64 AArch64)
#
# Usage:  sdk/apps/build-teradesk.sh
set -uo pipefail

SDK="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"      # .../sdk
TERA="$SDK/apps/teradesk"
OVL="$SDK/overlays/teradesk" # our asm-retarget/ABI drop-in replacements for
                             # individual .c/.h files (outside the vendored tree)
GEM="$SDK/gemlib"
LIBC="$SDK/libc"
LIBM="$SDK/libm"
LIBMTOP="$LIBM/openlibm-0.8.8"
GEMTOP="$GEM/gemlib-0.44.0"
BUILD="$TERA/build"
TREE="$BUILD/tree"           # shadow tree: vendored files + overlays, symlinked
OBJ="$BUILD/obj"
OUT="$BUILD/DESKTOP.NAP"
mkdir -p "$OBJ"

# --- ensure the two ported libraries are present ------------------------------
# TeraDesk links gemlib -> libc. The chain and each library's real input set
# live once in sdk/lib-stale.sh (TASK/T03); `lib_ensure gemlib` walks
# libc -> gemlib in that order and does not swallow what they report (TASK/T02).
# libm (OpenLibm, TASK/T07) is a second branch off libc -- gemlib does not
# depend on it -- so it is ensured separately; it is linked so that a port
# using <math.h> resolves without touching this script.
. "$SDK/lib-stale.sh"
lib_ensure gemlib || exit 1
lib_ensure libm   || exit 1
MINT="$LIBC/mintlib/include"
[ -d "$MINT" ]    || { echo "error: mintlib headers missing ($MINT); run build-libc.sh" >&2; exit 1; }
[ -d "$GEMTOP" ]  || { echo "error: gemlib headers missing ($GEMTOP); run build-gemlib.sh" >&2; exit 1; }
[ -d "$LIBMTOP" ] || { echo "error: OpenLibm headers missing ($LIBMTOP); run build-libm.sh" >&2; exit 1; }
[ -d "$TERA" ]   || { echo "error: TeraDesk source missing ($TERA)" >&2; exit 1; }

# --- toolchain ---------------------------------------------------------------
CLANG="${CLANG:-clang}"
command -v "$CLANG" >/dev/null 2>&1 || { echo "error: clang not found" >&2; exit 1; }
RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"   # LLVM tools: PATH / LLVM packages first, pinned Rust sysroot as fallback (SDK01 I, K)
AR="${AR:-$(newtos_tool llvm-ar)}"
LLD="${LLD:-$(newtos_tool ld.lld)}"
READOBJ="$(newtos_tool llvm-readobj)"
NM="$(newtos_tool llvm-nm)"
for t in "$AR" "$LLD"; do
    [ -x "$t" ] || { echo "error: missing LLVM tool: $t" >&2; exit 1; }
done

# --- flags -------------------------------------------------------------------
# Freestanding/no-libc set (shared with the other build-*.sh) + TeraDesk's own
# -Os -fomit-frame-pointer, and -D__NEWTOS__ to reach the retargeted GEM
# headers. -Wno-macro-redefined silences intentional TRUE/FALSE re-defines.
# -D_TOS_ONLY -> _MINT_=0 (library/xdialog/library.h:108): build the Single-TOS
# variant (== the distribution's DESKTOS.PRG). NewTOS is single-tasking until the
# cooperative kernel (Phase-2b, coop-kernel step 4) lands; the multitasking default
# would use shel_write()/AV-protocol/AES-multitasking queries that this OS does not
# provide. Switch back to the multitasking build once step 4 is done.
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -Os
        -fomit-frame-pointer -fPIE -fno-builtin -nostdinc -funsigned-char
        -D__NEWTOS__ -D_TOS_ONLY -Wno-macro-redefined)
# Include order: shadow tree root, its xdialog + utility libs, gemlib top
# (gem.h/portaes.h/portvdi.h/tos.h), mintlib include, the libm adapters
# (math.h -> OpenLibm's openlibm_math.h), clang headers last.
INCS=(-I"$TREE" -I"$TREE/library/xdialog" -I"$TREE/library/utility"
      -I"$GEMTOP" -I"$MINT" -I"$LIBM/include" -I"$LIBMTOP/include" -isystem "$RES")

echo "clang: $($CLANG --version | head -1)"
echo "abi  : LP64 (8-byte long/pointer, ELF64 AArch64)"
echo "libs : gemlib.a + libc.a  (no cflib -- TeraDesk bundles xdialog)"
echo

# --- shadow tree: symlink the pristine sources, then layer the overlays ------
# Compiling from the tree (not from $TERA) makes header overlays effective for
# quoted #include "..." lookups, uniformly for every translation unit.
rm -rf "$TREE"; mkdir -p "$TREE"
( cd "$TERA" && find . -type d ! -path './build*' ! -path './.git*' ) | \
    while IFS= read -r d; do mkdir -p "$TREE/$d"; done
( cd "$TERA" && find . -type f \( -name '*.c' -o -name '*.h' \) ! -path './build*' ) | \
    while IFS= read -r f; do ln -s "$TERA/${f#./}" "$TREE/${f#./}"; done
n_ovl=0
if [ -d "$OVL" ]; then
    while IFS= read -r f; do
        f="${f#./}"
        mkdir -p "$TREE/$(dirname "$f")"
        ln -sf "$OVL/$f" "$TREE/$f"
        echo "  overlay: $f (from sdk/overlays/teradesk/, vendored tree untouched)"
        n_ovl=$((n_ovl+1))
    done < <(cd "$OVL" && find . -type f \( -name '*.c' -o -name '*.h' \))
fi
echo "shadow tree: $TREE  (overlays applied: $n_ovl)"

# --- file list: every TeraDesk .c except library/utility/malloc.c (not in SRCS,
#     libc provides malloc). Faithful to the Makefile SRCS manifest. -----------
CFILES=()
while IFS= read -r c; do CFILES+=("$c"); done \
  < <(cd "$TREE" && find . -name '*.c' ! -path './library/utility/malloc.c' | sed 's,^\./,,' | sort)
n=${#CFILES[@]}
[ "$n" -gt 0 ] || { echo "error: no TeraDesk sources found under $TREE" >&2; exit 1; }
echo "TeraDesk sources: $n .c files"

# --- compile all (from the shadow tree; vendored tree stays pristine) --------
rm -f "$OBJ"/*.o "$OBJ"/*.err
: > "$BUILD/compiled.txt"; : > "$BUILD/failed.txt"
ok=0; fail=0
for c in "${CFILES[@]}"; do
    src="$TREE/$c"
    o="$OBJ/$(echo "$c" | tr '/' '_' | sed 's,\.c$,.o,')"
    e="${o%.o}.err"
    if "$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -c "$src" -o "$o" 2>"$e"; then
        echo "$c" >> "$BUILD/compiled.txt"; ok=$((ok+1))
    else
        echo "$c" >> "$BUILD/failed.txt"; fail=$((fail+1)); rm -f "$o"
    fi
done
echo "compiled OK=$ok  FAILED=$fail  (of $n)"

if [ "$fail" -ne 0 ]; then
    echo
    echo "FAILED files (general LP64 issue -> fix once in the SDK; TeraDesk-specific -> patch file in sdk/overlays/teradesk/):"
    while IFS= read -r f; do
        o="$OBJ/$(echo "$f" | tr '/' '_' | sed 's,\.c$,.o,')"
        echo "  === $f ==="
        sed 's/^/      /' "${o%.o}.err" 2>/dev/null | head -20
    done < "$BUILD/failed.txt"
    echo
    echo "STOP: fix the toolchain (libc/gemlib objects, overlay header) or add an overlay, then re-run."
    exit 1
fi

# --- CRT ---------------------------------------------------------------------
"$CLANG" "${CFLAGS[@]}" -I"$SDK/include" -I"$MINT" \
    -c "$LIBC/crt0.S" -o "$BUILD/crt0.o" || exit 1

# --- link (crt0 + all TeraDesk objects + gemlib.a + libc.a) ------------------
# gemlib -> libc dependency order, libm.a before libc.a (it needs libc's
# memset/_ctype/__assert_fail); wrap in a group so any residual
# back-reference between the archives still resolves.
rm -f "$OUT"
newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none --why-extract="$BUILD/why.txt" \
    "$BUILD/crt0.o" "$OBJ"/*.o \
    --start-group "$GEM/gemlib.a" "$LIBM/libm.a" "$LIBC/libc.a" --end-group \
    -o "$OUT" 2>"$BUILD/link.err"
lr=$?

if [ $lr -eq 0 ]; then
    echo
    echo "LINK OK -> $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes)"
    "$READOBJ" --file-headers "$OUT" | grep -iE 'Class:|DataEncoding:|Type:|Machine:|Entry:|Version'
    echo
    # Count real undefined references. (llvm-nm -u excludes the mandatory ELF
    # null symbol at index 0, which readobj would otherwise report as
    # "Undefined".)
    u=$("$NM" -u "$OUT" 2>/dev/null | grep -c . || true)
    echo "undefined symbols in the final image (goal 0): $u"

    # --- dist staging: everything C:\Apps\TERADESK\ needs, ready for the card ---
    # desktop.rsc goes through the KERNEL rsrc_load -> NRSC/LP64 (mkrsc convert).
    # icons.rsc goes through TeraDesk's OWN loader (rsc_load.c xrsrc_load), which
    # maps the file onto native LP64 structs -> native-RSHDR (mkrsc --rshdr).
    # cicons.rsc (the colour icons, a new-format CICONBLK .rsc) goes through the
    # KERNEL rsrc_load too (TASK/A41): NRSC v13/LP64, i.e. the CICONBLK/CICON
    # records behind a native RSHDR preamble, because TeraDesk's load_icons
    # (icon.c:2750) resolves the icon tree of the AES-loaded file with its OWN
    # xrsrc_gaddr (rsc_load.c:496-541), which parses the memory behind
    # ap_rscmem as a classic RSHDR -- the file image EmuTOS's rs_readit leaves
    # there. Since A41 part 2 the converted image IS such a file (mkrsc.py
    # `convert`, resource.rs NRSC_VER_RSHDR), so cicons.rsc is staged; if it is
    # missing from the card, TeraDesk's designed mono fallback (xrsrc_load
    # icons.rsc) takes over.
    DIST="$BUILD/dist"
    MKRSC="$SDK/../tools/mkrsc.py"
    rm -rf "$DIST"; mkdir -p "$DIST"
    cp "$OUT" "$DIST/DESKTOP.NAP"
    python3 "$MKRSC" convert "$TERA/desktop.rsc" "$DIST/desktop.rsc" || exit 1
    python3 "$MKRSC" convert --rshdr "$TERA/icons.rsc" "$DIST/icons.rsc" || exit 1
    python3 "$MKRSC" convert "$TERA/cicons.rsc" "$DIST/cicons.rsc" || exit 1
    echo "dist -> $DIST (DESKTOP.NAP, desktop.rsc + cicons.rsc NRSC/LP64, icons.rsc native-RSHDR)"
else
    echo
    echo "LINK FAILED -- residual undefined symbols (blockers):"
    grep -oE "undefined symbol: [^ ]+" "$BUILD/link.err" | awk '{print $3}' | sort -u | sed 's/^/    /'
    echo
    echo "See $BUILD/why.txt for the archive pull chains that reference them."
    exit 1
fi

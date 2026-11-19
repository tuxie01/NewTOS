#!/usr/bin/env bash
# build-present.sh <tree> <out dir> -- sdk/apps/present/build-present.sh of <tree>, copied with SDK = main tree (prebuilt
# gemlib/libpng/libz/libm/libc archives, no lib_ensure) and BUILD redirected to <out dir>, so nothing is written to any tree.
set -uo pipefail
T="$1"; BUILD="$2"; mkdir -p "$BUILD"
M=/Volumes/Daten/Development/PiRusTOS; SDK=$M/sdk; HERE=$T/sdk/apps/present; OUT=$BUILD/PRESENT.NAP
MINT="$SDK/libc/mintlib/include"; GEMTOP="$SDK/gemlib/gemlib-0.44.0"; LIBMTOP="$SDK/libm/openlibm-0.8.8"
ZTOP="$SDK/zlib/zlib-1.3.2"; PTOP="$SDK/libpng/libpng-1.6.59"
CLANG=clang; RES="$($CLANG -print-resource-dir)/include"; . "$SDK/toolchain.sh"
LLD="$(newtos_tool ld.lld)"
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -funsigned-char -D__NEWTOS__ -Wno-macro-redefined -Wall -Wextra)
INCS=(-I"$GEMTOP" -I"$ZTOP" -I"$PTOP" -I"$MINT" -I"$SDK/libm/include" -I"$LIBMTOP/include" -isystem "$RES")
rm -f "$BUILD"/*.o "$OUT"
"$CLANG" "${CFLAGS[@]}" "${INCS[@]}" -c "$HERE/present.c" -o "$BUILD/present.o" 2>"$BUILD/present.err" || { cat "$BUILD/present.err"; exit 1; }
echo "compiler warnings: $(grep -c 'warning:' "$BUILD/present.err")"
"$CLANG" -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -I"$SDK/include" -I"$MINT" -c "$SDK/libc/crt0.S" -o "$BUILD/crt0.o" || exit 1
newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none "$BUILD/crt0.o" "$BUILD/present.o" \
  --start-group "$SDK/gemlib/gemlib.a" "$SDK/libpng/libpng.a" "$SDK/zlib/libz.a" "$SDK/libm/libm.a" "$SDK/libc/libc.a" --end-group -o "$OUT" 2>"$BUILD/link.err" || { cat "$BUILD/link.err"; exit 1; }
python3 "$HERE/sample/gen-sample.py" "$BUILD"
shasum -a 256 "$OUT"; wc -c < "$OUT"

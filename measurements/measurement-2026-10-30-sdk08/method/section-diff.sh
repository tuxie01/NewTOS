#!/bin/bash
# section-diff.sh <workdir> -- rebuild every member of the three shipped archives
# with the current host clang and the flags the parity tests use; compare each
# against the shipped member section by section (elfsecdiff.py).
set -u
SDK=$(cd "$(dirname "$0")/../../../sdk" && pwd)
W="$1"; HERE=$(cd "$(dirname "$0")" && pwd)
CLANG=clang
. "$SDK/toolchain.sh"
AR="$(newtos_tool llvm-ar)"
RES="$("$CLANG" -print-resource-dir)/include"
CRT_TOP="$SDK/compiler-rt/compiler-rt-21.1.8.src"
OL_TOP="$SDK/libm/openlibm-0.8.8"
MINT="$SDK/libc/mintlib/include"
CMDS="$SDK/libc/build/compile-commands.txt"
CRT_FLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc
           -std=c11 -fvisibility=hidden -DVISIBILITY_HIDDEN -fomit-frame-pointer -DCOMPILER_RT_HAS_FLOAT16
           -I"$CRT_TOP/lib/builtins" -isystem "$RES")
OL_FLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc
          -std=c99 -fno-strict-aliasing -D__BSD_VISIBLE
          -I"$OL_TOP" -I"$OL_TOP/include" -I"$OL_TOP/aarch64" -I"$OL_TOP/src" -I"$OL_TOP/ld128" -I"$MINT" -isystem "$RES")
mkdir -p "$W"/{ship,same}/{crt,libm,libc}
(cd "$W/ship/crt" && "$AR" x "$SDK/compiler-rt/libclang_rt.builtins.a")
(cd "$W/ship/libm" && "$AR" x "$SDK/libm/libm.a")
(cd "$W/ship/libc" && "$AR" x "$SDK/libc/libc.a")
while IFS= read -r c; do
  o="$(printf '%s' "${c%.c}" | tr '/' '_').o"
  "$CLANG" "${CRT_FLAGS[@]}" -c "$CRT_TOP/$c" -o "$W/same/crt/$o" || echo "COMPILEFAIL crt $c"
done < "$SDK/compiler-rt/build/srcs.txt"
while IFS= read -r c; do
  d="${c%%/*}"; o="${d}__$(basename "${c%.c}").o"
  "$CLANG" "${OL_FLAGS[@]}" -c "$OL_TOP/$c" -o "$W/same/libm/$o" 2>/dev/null || echo "COMPILEFAIL libm $c"
done < "$SDK/libm/build/srcs.txt"
for f in "$W"/ship/libc/*.o; do
  mem=$(basename "$f")
  line=$(grep -F -- "-o $SDK/libc/build/obj/$mem " "$CMDS" | tail -1)
  [ -n "$line" ] || { echo "NOCMD libc $mem"; continue; }
  eval "cmd=($line)"
  for i in "${!cmd[@]}"; do [ "${cmd[$i]}" = "-o" ] && oi=$((i+1)); done
  cmd[$oi]="$W/same/libc/$mem"; "${cmd[@]}" 2>/dev/null || echo "COMPILEFAIL libc $mem"
done
for a in crt libm libc; do
  for f in "$W"/ship/$a/*.o; do
    m=$(basename "$f")
    if [ ! -f "$W/same/$a/$m" ]; then echo "$a $m MISSING-REBUILD"; continue; fi
    if cmp -s "$f" "$W/same/$a/$m"; then echo "$a $m BYTEIDENT"; else echo "$a $m $(python3 -I "$HERE/elfsecdiff.py" "$f" "$W/same/$a/$m")"; fi
  done
done

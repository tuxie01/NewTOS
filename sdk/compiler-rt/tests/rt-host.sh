#!/bin/bash
#
# rt-host.sh -- host test of the compiler-rt builtins archive
# (sdk/compiler-rt/libclang_rt.builtins.a, TASK/SDK01 E) and of the libm
# members that need it. Run by sdk/run-host-tests.sh; exit 0 = pass,
# 77 = skip, else fail.
#
#   1. rt-elf.c links with sdk/bin/newtos-cc -lm (the wrapper's link line:
#      crt0 + libm.a + libc.a + the builtins), 0 undefined symbols; clang
#      lowers `long double` a*b+c and C99 complex `*`/`/` in it to
#      __multf3/__addtf3/__muldc3/__divdc3/__mulsc3; the link map shows the
#      binary128 helpers from libclang_rt.builtins.a and __extenddftf2 still
#      from libc.a (newtos-arch/softfloat.c, first archive wins).
#   2. Rebuild parity: every member of the shipped libclang_rt.builtins.a and
#      libm.a is recompiled here from the same source with the flags of
#      build-compiler-rt.sh / build-libm.sh and must be byte-identical --
#      except for the compiler identification: clang writes its version string
#      into `.comment' and nowhere else, so a member that differs only there
#      (archives built by an earlier clang release, TASK/SDK08) counts as
#      identical once `.comment' is removed from both objects; code, data,
#      relocations and symbols stay byte-compared. The line prints the host
#      clang and the clang the shipped members name. A counter-check flips one
#      byte in the code of a rebuilt member, which must be reported as differing.
#   3. The same rebuild with -ffixed-x18 (macOS clobbers x18; rt-host.c
#      "x18") is linked with rt-elf.c into a program whose data segment sits
#      on a 16 KiB page (newtos.ld + one ALIGN), and rt-host.c runs that
#      AArch64 code on the host against exact and host-libm oracles; the host
#      side links OpenLibm built for the host from the same sources.
#
# Skips when the archives are not built, clang is missing, or the host is not
# AArch64 (the loader runs the target's machine code natively).
set -u

SDK=$(cd "$(dirname "$0")/../.." && pwd)
HERE="$SDK/compiler-rt/tests"
CC=${CC:-cc}
CLANG=${CLANG:-clang}

command -v "$CLANG" >/dev/null 2>&1 || { echo "skip: clang not found"; exit 77; }
case $(uname -m) in arm64|aarch64) ;; *) echo "skip: host is not AArch64"; exit 77 ;; esac
for a in libc/libc.a libm/libm.a compiler-rt/libclang_rt.builtins.a; do
  [ -f "$SDK/$a" ] || { echo "skip: sdk/$a not built"; exit 77; }
done
CRT_TOP="$SDK/compiler-rt/compiler-rt-21.1.8.src"
OL_TOP="$SDK/libm/openlibm-0.8.8"
MINT="$SDK/libc/mintlib/include"
for f in "$SDK/compiler-rt/build/srcs.txt" "$SDK/libm/build/srcs.txt" "$SDK/libm/build/srcs-ld.txt" "$CRT_TOP/lib/builtins" "$OL_TOP/src" "$MINT"; do
  [ -e "$f" ] || { echo "skip: $f missing (run the library's build script)"; exit 77; }
done
. "$SDK/toolchain.sh"   # LLVM tools: PATH / LLVM packages first, pinned Rust sysroot as fallback (SDK01 I, K)
NM="$(newtos_tool llvm-nm)"; AR="$(newtos_tool llvm-ar)"; LLD="$(newtos_tool ld.lld)"
OBJDUMP="$(newtos_tool llvm-objdump)"; OBJCOPY="$(newtos_tool llvm-objcopy)"; READOBJ="$(newtos_tool llvm-readobj)"
for t in "$NM" "$AR" "$LLD" "$OBJDUMP" "$OBJCOPY" "$READOBJ"; do [ -x "$t" ] || { echo "skip: $t missing"; exit 77; }; done
RES="$("$CLANG" -print-resource-dir)/include"

WORK=$(mktemp -d "${TMPDIR:-/tmp}/rt-host.XXXXXX") || exit 1
trap 'rm -rf "$WORK"' EXIT
fail=0

# parity <shipped> <rebuilt>: "same" when byte-identical; "ident" when they
# differ only in the compiler identification -- byte-identical once `.comment'
# (clang's "... clang version ..." string) is removed from both and their
# symbol tables still agree (removing .comment also drops the symbols defined
# in it, AArch64's `$d' mapping symbol); else "differ".
parity() {
  cmp -s "$1" "$2" && { echo same; return; }
  "$OBJCOPY" --remove-section=.comment "$1" "$WORK/pa.o" &&
  "$OBJCOPY" --remove-section=.comment "$2" "$WORK/pb.o" &&
  cmp -s "$WORK/pa.o" "$WORK/pb.o" || { echo differ; return; }
  [ "$("$READOBJ" --symbols "$1" | grep -v '^File: ')" = "$("$READOBJ" --symbols "$2" | grep -v '^File: ')" ] &&
    echo ident || echo differ
}
# flip_code <object>: invert the byte in the middle of its first non-empty
# executable section; prints that section's name (nothing if there is none).
flip_code() {
  local nm off size pos b
  read -r nm off size < <("$READOBJ" --sections "$1" | awk '
    /Section \{/ {x = 0} /^ *Name:/ {nm = $2} /SHF_EXECINSTR/ {x = 1}
    /^ *Offset:/ {off = $2} /^ *Size:/ {if (x && $2 + 0 > 0) {print nm, off, $2; exit}}')
  [ -n "${size:-}" ] || return 1
  pos=$(( off + size / 2 ))
  b=$(od -An -tu1 -j "$pos" -N1 "$1" | tr -d ' ')
  printf "$(printf '\\%03o' $(( b ^ 0xff )))" | dd of="$1" bs=1 seek="$pos" conv=notrunc 2>/dev/null
  echo "$nm"
}

# --- 1. the newtos-cc link ------------------------------------------------------
"$SDK/bin/newtos-cc" -lm -c "$HERE/rt-elf.c" -o "$WORK/rt-elf.o" || { echo "FAIL: rt-elf.c does not compile"; exit 1; }
und=$("$NM" -u "$WORK/rt-elf.o" | awk '{print $NF}' | tr '\n' ' ')
for s in __multf3 __addtf3 __muldc3 __divdc3 __mulsc3; do
  case " $und " in *" $s "*) ;; *) echo "FAIL: clang did not lower to $s (undefined: $und)"; fail=1 ;; esac
done
"$SDK/bin/newtos-cc" -lm -Wl,-Map="$WORK/rt.map" "$WORK/rt-elf.o" -o "$WORK/RT.NCP" ||
  { echo "FAIL: rt-elf does not link with newtos-cc -lm"; exit 1; }
n_und=$("$NM" -u "$WORK/RT.NCP" | grep -c .)
echo "1  newtos-cc -lm: RT.NCP linked ($(wc -c < "$WORK/RT.NCP" | tr -d ' ') bytes), undefined symbols: $n_und"
[ "$n_und" -eq 0 ] || fail=1
grep -q 'libclang_rt\.builtins\.a(lib_builtins_multf3\.o)' "$WORK/rt.map" ||
  { echo "FAIL: __multf3 not from libclang_rt.builtins.a"; fail=1; }
grep -q 'libc\.a(arch__softfloat\.o)' "$WORK/rt.map" ||
  { echo "FAIL: __extenddftf2 not from libc.a (newtos-arch/softfloat.c)"; fail=1; }
if grep -q 'libclang_rt\.builtins\.a(lib_builtins_extenddftf2\.o)' "$WORK/rt.map"; then
  echo "FAIL: libclang_rt.builtins.a(extenddftf2.o) linked next to libc's"; fail=1
fi

# --- 2. rebuild parity + 3. the -ffixed-x18 rebuild -----------------------------
# Flags: sdk/vendor-lib.sh VL_CFLAGS + build-compiler-rt.sh LIB_CFLAGS/LIB_INCS,
# and build-libm.sh CFLAGS/INCS; object names as those scripts give them.
CRT_FLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc
           -std=c11 -fvisibility=hidden -DVISIBILITY_HIDDEN -fomit-frame-pointer -DCOMPILER_RT_HAS_FLOAT16
           -I"$CRT_TOP/lib/builtins" -isystem "$RES")
OL_FLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc
          -std=c99 -fno-strict-aliasing -D__BSD_VISIBLE
          -I"$OL_TOP" -I"$OL_TOP/include" -I"$OL_TOP/aarch64" -I"$OL_TOP/src" -I"$OL_TOP/ld128" -I"$MINT" -isystem "$RES")
mkdir -p "$WORK/ship/crt" "$WORK/ship/libm" "$WORK/same" "$WORK/x18/crt" "$WORK/x18/libm" "$WORK/olh"
(cd "$WORK/ship/crt" && "$AR" x "$SDK/compiler-rt/libclang_rt.builtins.a") || exit 1
(cd "$WORK/ship/libm" && "$AR" x "$SDK/libm/libm.a") || exit 1
nsame=0; nident=0; ndiff=0
while IFS= read -r c; do
  o="$(printf '%s' "${c%.c}" | tr '/' '_').o"
  "$CLANG" "${CRT_FLAGS[@]}" -c "$CRT_TOP/$c" -o "$WORK/same/$o" || { echo "FAIL: $c does not compile"; exit 1; }
  case $(parity "$WORK/ship/crt/$o" "$WORK/same/$o") in
    same)  nsame=$((nsame+1)) ;;
    ident) nident=$((nident+1)) ;;
    *)     ndiff=$((ndiff+1)); echo "FAIL: rebuild of $c differs from the shipped member (not only in .comment)" ;;
  esac
  "$CLANG" "${CRT_FLAGS[@]}" -ffixed-x18 -c "$CRT_TOP/$c" -o "$WORK/x18/crt/$o" || exit 1
done < "$SDK/compiler-rt/build/srcs.txt"
while IFS= read -r c; do
  d="${c%%/*}"; o="${d}__$(basename "${c%.c}").o"
  "$CLANG" "${OL_FLAGS[@]}" -c "$OL_TOP/$c" -o "$WORK/same/$o" 2>/dev/null || { echo "FAIL: $c does not compile"; exit 1; }
  case $(parity "$WORK/ship/libm/$o" "$WORK/same/$o") in
    same)  nsame=$((nsame+1)) ;;
    ident) nident=$((nident+1)) ;;
    *)     ndiff=$((ndiff+1)); echo "FAIL: rebuild of $c differs from the shipped member (not only in .comment)" ;;
  esac
  "$CLANG" "${OL_FLAGS[@]}" -ffixed-x18 -c "$OL_TOP/$c" -o "$WORK/x18/libm/$o" 2>/dev/null || exit 1
  # the `long double' members (build/srcs-ld.txt, TASK/SDK01 J) have no host
  # counterpart: OpenLibm's Darwin configuration does not build them, the
  # host's `long double' is double. sdk/libc/tests/ldbl-host.sh checks them.
  grep -qxF "$c" "$SDK/libm/build/srcs-ld.txt" && continue
  # the host build of the same source: OpenLibm's own Make.inc flags for a
  # clang host (-std=c99 -fno-builtin -fno-strict-aliasing -D__BSD_VISIBLE,
  # its four include dirs), -O2 like the target build
  "$CC" -O2 -std=c99 -fno-builtin -fno-strict-aliasing -D__BSD_VISIBLE -Wno-implicit-function-declaration \
    -I"$OL_TOP" -I"$OL_TOP/include" -I"$OL_TOP/aarch64" -I"$OL_TOP/src" \
    -c "$OL_TOP/$c" -o "$WORK/olh/$o" 2>/dev/null || { echo "FAIL: host build of $c"; exit 1; }
done < "$SDK/libm/build/srcs.txt"
n_ship=$(( $(ls "$WORK/ship/crt" | grep -c '\.o$') + $(ls "$WORK/ship/libm" | grep -c '\.o$') ))
echo "2  rebuild parity: $nsame of $n_ship shipped members byte-identical, $nident identical but for the compiler identification (.comment), $ndiff differ"
[ "$ndiff" -eq 0 ] && [ $((nsame + nident)) -eq "$n_ship" ] || fail=1
echo "   host clang: $("$CLANG" --version | head -1)"
"$READOBJ" -p .comment "$WORK"/ship/crt/*.o "$WORK"/ship/libm/*.o 2>/dev/null |
  sed -n 's/^ *\[ *[0-9a-f]*\] //p' | sort | uniq -c |
  while read -r n s; do echo "   shipped members built by: $s ($n members)"; done
# counter-check: one byte of code changed in a copy of a rebuilt member must
# still be reported as a difference
cp "$WORK/same/lib_builtins_multf3.o" "$WORK/flip.o"
sec=$(flip_code "$WORK/flip.o") || { echo "FAIL: counter-check found no code section in lib_builtins_multf3.o"; fail=1; }
if [ -n "$sec" ]; then
  r=$(parity "$WORK/ship/crt/lib_builtins_multf3.o" "$WORK/flip.o")
  echo "   counter-check: one byte of $sec in lib_builtins_multf3.o inverted -> $r (expected: differ)"
  [ "$r" = differ ] || { echo "FAIL: the parity check misses a changed code byte"; fail=1; }
fi
n18=$("$OBJDUMP" -d "$WORK"/x18/crt/*.o "$WORK"/x18/libm/*.o | grep -E '^ +[0-9a-f]+:' | grep -cE '\b[xw]18\b')
[ "$n18" -eq 0 ] || { echo "FAIL: the -ffixed-x18 rebuild still uses x18 ($n18 instructions)"; fail=1; }
"$AR" cr "$WORK/x18-builtins.a" "$WORK"/x18/crt/*.o && "$AR" cr "$WORK/x18-libm.a" "$WORK"/x18/libm/*.o || exit 1
ar cr "$WORK/olh.a" "$WORK"/olh/*.o 2>/dev/null || "$AR" cr "$WORK/olh.a" "$WORK"/olh/*.o || exit 1

# newtos.ld with the data segment on a 16 KiB page (host page size)
sed 's/^\(    \)\.init_array : {/\1. = ALIGN(0x4000);\
\1.init_array : {/' "$SDK/newtos.ld" > "$WORK/host.ld"
grep -q 'ALIGN(0x4000)' "$WORK/host.ld" || { echo "FAIL: could not derive host.ld from newtos.ld"; exit 1; }
"$SDK/bin/newtos-cc" -c "$SDK/libc/crt0.S" -o "$WORK/crt0.o" || exit 1
# libc.a: the libm members' memset/_ctype/__assert_fail and crt0's runtime;
# none of it runs on the host. Its __extenddftf2 has no x18 (checked below).
newtos_lld -T "$WORK/host.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none \
  "$WORK/crt0.o" "$WORK/rt-elf.o" --start-group "$WORK/x18-libm.a" "$SDK/libc/libc.a" "$WORK/x18-builtins.a" --end-group \
  -o "$WORK/RT-x18.elf" || { echo "FAIL: x18 test program does not link"; exit 1; }
n18s=$("$OBJDUMP" -d "$WORK/RT-x18.elf" --disassemble-symbols=__extenddftf2 | grep -E '^ +[0-9a-f]+:' | grep -cE '\b[xw]18\b')
[ "$n18s" -eq 0 ] || { echo "FAIL: libc's __extenddftf2 uses x18"; fail=1; }
echo "3  -ffixed-x18 rebuild: 0 x18 instructions expected, found $n18; test program linked with host.ld"

"$CC" -O1 -Wall -Wextra -Werror -o "$WORK/rt-host" "$HERE/rt-host.c" "$WORK/olh.a" || { echo "FAIL: rt-host.c does not build"; exit 1; }
"$WORK/rt-host" "$WORK/RT-x18.elf"; rc=$?
[ $rc -eq 77 ] && exit 77
[ $rc -eq 0 ] || fail=1
exit $fail

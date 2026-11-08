#!/bin/bash
# disasm-check.sh <kernel ELF> <out> : the V59 functions of a release kernel (llvm-objdump -d --demangle), per
# function: the instructions that name a V/Q register, the registers, the calls (ADR-036 points 2 and 3).
set -u
ELF=$1; OUT=$2
OD=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-objdump
TMP=$(mktemp -d "${S:?}/dis.XXXXXX"); trap 'rm -rf "$TMP"' EXIT
"$OD" -d --demangle "$ELF" > "$TMP/all.dis"
{
  echo "ELF $ELF sha256 $(shasum -a 256 "$ELF" | cut -d' ' -f1)"
  for s in "<kernel::raster_copy::read_rect>:" "RasterBackend>::write_device_rect>:" "<kernel::raster_expand::sprite>:" \
           "RasterBackend>::draw_sprite>:" "RasterBackend>::read_device_rect>:"; do
    awk -v s="$s" 'index($0,s){f=1;next} f&&/>:$/{f=0} f' "$TMP/all.dis" > "$TMP/one.dis"
    echo "== $s $(wc -l < "$TMP/one.dis" | tr -d ' ') lines"
    echo "   instructions naming V/Q registers: $(grep -E '\b[qv][0-9]+\b' "$TMP/one.dis" | awk -F'\t' '{print $2}' | sort | uniq -c | tr -s ' ' | tr '\n' ',')"
    echo "   registers: $(grep -oE '\b[qv][0-9]+\b' "$TMP/one.dis" | sort -u | tr '\n' ' ')"
    echo "   calls: $(grep -E $'\tbl\t' "$TMP/one.dis" | sed -E 's/.*<(.*)>.*/\1/' | sort -u | tr '\n' ';')"
  done
} > "$OUT"
cat "$OUT"

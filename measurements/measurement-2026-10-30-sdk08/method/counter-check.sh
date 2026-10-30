#!/bin/bash
# counter-check.sh <workdir> <member>... -- the TASK/SDK08 counter-check over
# every section. <workdir> is the one section-diff.sh filled (ship/ = the
# shipped members, same/ = the rebuild); <member> is e.g.
# crt/lib_builtins_multf3.o. For each member and each non-empty section of its
# rebuild: invert the byte in the middle of that section in a copy and run the
# parity() function of sdk/compiler-rt/tests/rt-host.sh (taken from the script
# itself, not copied) against the shipped member. Expected: `ident' for
# .comment, `differ' for every other section; prints one line per section and
# exits 1 on any other result.
set -u
ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
W="$1"; shift
. "$ROOT/sdk/toolchain.sh"
OBJCOPY="$(newtos_tool llvm-objcopy)"; READOBJ="$(newtos_tool llvm-readobj)"
WORK=$(mktemp -d "${TMPDIR:-/tmp}/counter-check.XXXXXX") || exit 1
trap 'rm -rf "$WORK"' EXIT
eval "$(sed -n '/^parity() {/,/^}/p' "$ROOT/sdk/compiler-rt/tests/rt-host.sh")"
type parity >/dev/null || { echo "parity() not found in rt-host.sh"; exit 1; }
bad=0
for m in "$@"; do
  echo "$m: unchanged copy -> $(parity "$W/ship/$m" "$W/same/$m")"
  "$READOBJ" --sections "$W/same/$m" | awk '
    /^ *Name:/ {nm = $2} /^ *Type:/ {ty = $2}
    /^ *Offset:/ {off = $2} /^ *Size:/ {if ($2 + 0 > 0 && ty != "SHT_NOBITS") print nm, off, $2}' |
  while read -r nm off size; do
    cp "$W/same/$m" "$WORK/flip.o"
    pos=$(( off + size / 2 ))
    b=$(od -An -tu1 -j "$pos" -N1 "$WORK/flip.o" | tr -d ' ')
    printf "$(printf '\\%03o' $(( b ^ 0xff )))" | dd of="$WORK/flip.o" bs=1 seek="$pos" conv=notrunc 2>/dev/null
    r=$(parity "$W/ship/$m" "$WORK/flip.o")
    [ "$nm" = .comment ] && want=ident || want=differ
    [ "$r" = "$want" ] && ok=ok || ok=WRONG
    printf '  %-16s byte %6d of %6d inverted -> %-6s (expected %-6s) %s\n' "$nm" "$pos" "$size" "$r" "$want" "$ok"
  done > "$WORK/out"
  cat "$WORK/out"
  grep -q WRONG "$WORK/out" && bad=1
done
exit $bad

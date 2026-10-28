#!/bin/sh
# TOOL04: first launch vs. file size and volume, toolchain-independent.
# One C program, linked with clang (ld ad-hoc linker-signs it, like rustc's
# test binaries), padded with N MiB of random bytes in a __DATA section so each
# build is a new, never-launched binary. Built straight into the directory $2
# (e.g. one on the internal disk, one on /Volumes/Daten), launched twice.
# $1 = label, $2 = output dir, rest = sizes in MiB (0 = no padding).
M=$(dirname "$0"); label=$1; out=$2; shift 2
mkdir -p "$out"
printf 'int main(void){return 0;}\n' > "$out/hello.c"
for n in "$@"; do
  f="$out/hello-$n"
  if [ "$n" = 0 ]; then
    clang -O0 "$out/hello.c" -o "$f"
  else
    head -c $((n * 1048576)) /dev/urandom > "$out/pad-$n.bin"
    clang -O0 "$out/hello.c" -o "$f" -Wl,-sectcreate,__DATA,__pad,"$out/pad-$n.bin"
    rm -f "$out/pad-$n.bin"
  fi
  sz=$(stat -f %z "$f")
  printf '%s\t%s MiB\t%s bytes\t' "$label" "$n" "$sz"
  python3 -I "$M/t.py" first "$f" | tr '\n' '\t'
  python3 -I "$M/t.py" second "$f"
done

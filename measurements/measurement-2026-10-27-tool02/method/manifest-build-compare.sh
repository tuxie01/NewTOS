#!/usr/bin/env bash
# manifest-build-compare.sh -- full tools/make-sd-image.sh builds, old (85055757, before TOOL02) vs new (working tree),
# same image path, same environment (TASK/TOOL02 Abnahme 1). Scratch dir = $1 (images are deleted, manifests kept there).
#   A: default env, size 64: old twice (the variance of an unchanged script), new twice
#   B: SD_FSBENCH_D=<3 files> SD_MXPROBE_AUTO=0, size 128 (D: partition + switch lines): old once, new once
# The old script runs from a scratch root whose tools/ links every other tool to the working tree (ROOT is the script's
# parent dir; the script only reads from ROOT). Compare: raw diff lines (key + last field), then the manifests with the
# five lines that change between any two builds masked (image_sha256, created, the file lines of the three
# /dev/urandom files C:\DATA.BIN C:\KERNEL8.IMG C:\LYNXBOOT.IMG), byte-compared; stdout of old and new compared too.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"; T="${1:?scratch dir}"
rm -rf "$T"; mkdir -p "$T/old/tools" "$T/fsb" "$T/m"; ln -s "$WS/sdk" "$T/old/sdk"; ln -s "$WS/tosdump" "$T/old/tosdump"
for f in "$WS"/tools/*; do b=$(basename "$f"); case "$b" in make-sd-image.sh|sd-manifest.sh|mkcard.sh) ;; *) ln -s "$f" "$T/old/tools/$b" ;; esac; done
(cd "$WS" && git show 85055757:tools/make-sd-image.sh) > "$T/old/tools/make-sd-image.sh"; chmod +x "$T/old/tools/make-sd-image.sh"
for i in 1 2 3; do printf 'fsbench %s\n' $i > "$T/fsb/F$i.DAT"; done
build() { # <old|new> <name> <size> [env...]
  local which=$1 name=$2 size=$3 s; shift 3
  if [ "$which" = old ]; then s="$T/old/tools/make-sd-image.sh"; else s="$WS/tools/make-sd-image.sh"; fi
  env -u SD_FSBENCH_D -u SD_MXPROBE_AUTO "$@" "$s" "$T/sd.img" "$size" > "$T/m/$name.log" 2>&1; echo "$name: rc=$?"
  cp "$T/sd.img.manifest" "$T/m/$name.manifest"; rm -f "$T/sd.img" "$T/sd.img.manifest"; }
mask() { sed -E -e 's/^(image_sha256|created) .*/\1 <varies>/' \
  -e 's/^file [0-9a-f]{64} ([0-9]+ C:\\(DATA\.BIN|KERNEL8\.IMG|LYNXBOOT\.IMG))$/file <random> \1/' "$1"; }
cmpm() { # <a> <b>
  echo "== $1 vs $2: raw differing lines (key, last field):"
  diff "$T/m/$1.manifest" "$T/m/$2.manifest" | grep '^[<>]' | awk '{print "   ", $1, $2, $NF}'
  mask "$T/m/$1.manifest" > "$T/m/$1.masked"; mask "$T/m/$2.manifest" > "$T/m/$2.masked"
  if cmp -s "$T/m/$1.masked" "$T/m/$2.masked"; then echo "   masked: IDENTICAL ($(wc -l < "$T/m/$2.masked" | tr -d ' ') lines)"; else echo "   masked: DIFFER"; fi
  if cmp -s "$T/m/$1.log" "$T/m/$2.log"; then echo "   stdout+stderr: IDENTICAL"; else echo "   stdout+stderr: DIFFER"; fi; }
build old old-A1 64; build old old-A2 64; build new new-A1 64; build new new-A2 64
build old old-B 128 SD_FSBENCH_D="$T/fsb" SD_MXPROBE_AUTO=0; build new new-B 128 SD_FSBENCH_D="$T/fsb" SD_MXPROBE_AUTO=0
cmpm old-A1 old-A2; cmpm old-A1 new-A1; cmpm old-A2 new-A2; cmpm new-A1 new-A2; cmpm old-B new-B
echo "== B switch/optional/D: lines (new-B):"; grep -E '^(switch|optional_missing|metadata D)|D:\\' "$T/m/new-B.manifest" | sed 's/^/    /'

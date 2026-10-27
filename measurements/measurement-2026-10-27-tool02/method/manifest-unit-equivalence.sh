#!/usr/bin/env bash
# manifest-unit-equivalence.sh -- same inputs, old vs new manifest code, byte comparison (TASK/TOOL02 Arbeit 1).
# old = list_card_files() + the assembly block, verbatim from tools/make-sd-image.sh at 85055757 (before TOOL02);
# new = tools/sd-manifest.sh files/write. Inputs: one fixed image file, two directory trees as C: and D: (spaces in a
# path, an empty file, a .fseventsd metadata dir), 0/1/2 optional_missing entries x SD_* switches unset/set;
# `date` is shimmed to a fixed time on both sides. Output to stdout; scratch dir = $1.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"; U="${1:?scratch dir}"
rm -rf "$U"; mkdir -p "$U/shim" "$U/treeC/Apps/My Dir" "$U/treeC/.fseventsd" "$U/treeC/AUTO" "$U/treeD/FSBENCH"; cd "$U"
printf '#!/bin/sh\necho 2026-10-27T00:00:00Z\n' > shim/date; chmod +x shim/date
printf 'a\r\n' > treeC/README.TXT; head -c 3000 /dev/urandom > "treeC/Apps/My Dir/X.NAP"; : > treeC/AUTO/EMPTY
printf 'z' > treeC/.fseventsd/f; head -c 999 /dev/urandom > treeD/FSBENCH/A.DAT; head -c 100000 /dev/urandom > img.bin
(cd "$WS" && git show 85055757:tools/make-sd-image.sh) > head.sh
{ echo 'set -euo pipefail'
  echo 'IMG="$1"; SIZE_MB=64; MANIFEST="${IMG}.manifest"; MANIFEST_FILES="${IMG}.manifest.part"; OPTIONAL_MISSING=()'
  echo 'while [ $# -gt 1 ]; do shift; OPTIONAL_MISSING+=("$1"); done'
  sed -n '/^list_card_files() {/,/^}/p' head.sh
  echo 'list_card_files "$TC" C; list_card_files "$TD" D'
  sed -n '/^# Assemble the manifest/,/^rm -f "\$MANIFEST_FILES"/p' head.sh; } > old.sh
new() { local img=$1; local a; a=(); shift; for m in "$@"; do a+=(--optional-missing "$m"); done
  "$WS/tools/sd-manifest.sh" files "$TC" C "$img"; "$WS/tools/sd-manifest.sh" files "$TD" D "$img"
  "$WS/tools/sd-manifest.sh" write "$img" 64 tools/make-sd-image.sh ${a[@]+"${a[@]}"}; }
export TC="$U/treeC" TD="$U/treeD" PATH="$U/shim:$PATH"
echo "old.sh: list_card_files() + the manifest assembly block of 85055757:tools/make-sd-image.sh ($(wc -l < old.sh | tr -d " ") lines)"
fail=0
for c in 0 1 2; do
  case $c in 0) om=() ;; 1) om=('C:\ZVIEW\SAMPLES\ -- absent (x) -- build it with: y') ;; 2) om=('one -- a' 'two -- b') ;; esac
  for sw in unset set; do
    if [ $sw = set ]; then export SD_ZED=1 SD_ALPHA=x; else unset SD_ZED SD_ALPHA; fi
    cp img.bin X.img; bash old.sh X.img ${om[@]+"${om[@]}"}; mv X.img.manifest old.m
    cp img.bin X.img; new X.img ${om[@]+"${om[@]}"};          mv X.img.manifest new.m
    left=$( (ls X.img.manifest.part X.img.manifest.tmp 2>/dev/null || true) | wc -l | tr -d ' ')
    if cmp -s old.m new.m; then r=IDENTICAL; else r=DIFFER; fail=1; fi
    echo "optional_missing entries=$c SD_* switches=$sw: $r ($(wc -l < new.m | tr -d ' ') lines, sha256 $(shasum -a 256 new.m | cut -c1-16)); leftover part/tmp files: $left"
    rm -f X.img*
  done
done
echo "--- last case, new.m:"; cat new.m
exit $fail

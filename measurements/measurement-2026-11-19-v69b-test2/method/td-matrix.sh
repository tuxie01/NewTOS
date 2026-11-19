#!/usr/bin/env bash
# td-matrix.sh -- scripts x side (wt = patched, main = unpatched) x planes v16 (default boot), v32 (vdiplanes=32)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
for sc in ${SCRIPTS:-open cancel2 ok3}; do for side in wt main; do for p in v16 v32; do
  [ $p = v16 ] && app="serial=on" || app="serial=on vdiplanes=32"
  [ -f "$HERE/../serial/$sc-$side-$p.log" ] && continue
  bash "$HERE/td-open.sh" $side $p "$app" "$S/bin/k-$sc.img" $sc
done; done; done

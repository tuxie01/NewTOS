#!/usr/bin/env bash
# td-extra.sh -- explicit vdiplanes=16 for the open script (wt, main); repeat of cancel2 at the default (label v16b) to
# see whether the one differing frame of the matrix ("opts", menu hover) repeats
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
for side in wt main; do bash "$HERE/td-open.sh" $side vdiplanes16 "serial=on vdiplanes=16" "$S/bin/k-open.img" open; done
for side in wt main; do bash "$HERE/td-open.sh" $side v16b "serial=on" "$S/bin/k-cancel2.img" cancel2; done
echo EXTRADONE

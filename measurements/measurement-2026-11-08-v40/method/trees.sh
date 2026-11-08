#!/bin/bash
# V40: the source trees the kernels and tables build from, in the session scratchpad S.
#   base = git archive of HEAD (ad201a57) without measurements/. The ln -s below lands INSIDE the archived references/
#   directory (references/references): neither tree has the EmuTOS sources; every build succeeded without them.
#   fix  = base plus every file the V40 change touches (the paths named as arguments), copied from the
#          main tree's working copy -- not the whole dirty tree: a parallel worker's uncommitted files stay out.
# usage: S=... trees.sh base | S=... trees.sh fix <path>...
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
T=$1; shift
rm -rf "$S/t/$T"; mkdir -p "$S/t/$T"
git archive HEAD -- ':!measurements' | tar -x -C "$S/t/$T"
ln -s "$PWD/references" "$S/t/$T/references"
for f in "$@"; do mkdir -p "$S/t/$T/$(dirname "$f")"; cp -p "$f" "$S/t/$T/$f"; echo "$T tree: $f"; done

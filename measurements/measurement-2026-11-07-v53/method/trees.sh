#!/bin/bash
# V53: the two source trees in the scratchpad S. base = git archive of HEAD (3380004f then: 74ad8d3e + docs only; product code untouched);
# fix = the same plus every file the V53 change touches, copied from the main tree's working copy (uncommitted
# then). references/ linked into both. (base was made first, for the base builds: method/builds.sh.)
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
for T in base fix; do
  [ -d "$S/t/$T" ] && continue
  mkdir -p "$S/t/$T"; git archive HEAD | tar -x -C "$S/t/$T"; ln -s "$PWD/references" "$S/t/$T/references"
done
git status --porcelain --untracked-files=all | sed 's/^...//' | while read -r f; do
  case $f in measurements/*) continue ;; esac
  mkdir -p "$S/t/fix/$(dirname "$f")"; cp -p "$f" "$S/t/fix/$f"; echo "fix tree: $f"
done

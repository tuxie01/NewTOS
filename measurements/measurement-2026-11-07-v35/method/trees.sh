#!/bin/bash
# V35: the two source trees the GUI runs build from, in the scratchpad S.
#   base = git archive of HEAD (35eeb6b5), fix = the same plus every file the V35 change touches, copied from
#   the main tree's working copy (the change was uncommitted then). references/ linked into both.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
for T in base fix; do rm -rf "$S/t/$T"; mkdir -p "$S/t/$T"; git archive HEAD | tar -x -C "$S/t/$T"; ln -s "$PWD/references" "$S/t/$T/references"; done
git status --porcelain --untracked-files=all | sed 's/^...//' | while read -r f; do
  case $f in measurements/*) continue ;; esac
  mkdir -p "$S/t/fix/$(dirname "$f")"; cp -p "$f" "$S/t/fix/$f"; echo "fix tree: $f"
done

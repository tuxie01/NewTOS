#!/usr/bin/env bash
# lint-both.sh -- Tester a115-test: build+clippy numbers for base 587e0f68 and new (working tree), scratch trees.
set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"
for side in new base; do
  T="$S/trees/lint-$side"; rm -rf "$T"; mkdir -p "$T"
  if [ $side = new ]; then (cd "$WS" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"; else (cd "$WS" && git archive $HEADREV) | tar -x -C "$T"; fi
  for d in references sdk/apps; do for f in $WS/$d/*; do b=${f#$WS/}; [ -e "$T/$b" ] || ln -s "$f" "$T/$b"; done; done
  bash "$HERE/lint-nt.sh" "$T" "$S/lint-$side"
done
echo ALLDONE > "$S/lint-done"

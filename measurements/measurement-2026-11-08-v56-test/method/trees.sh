#!/bin/bash
# V56: the two source trees in the scratchpad S. base = git archive of ad201a57 (HEAD when the ticket started,
# product code = 142134de's); fix = the same plus the V56 files copied from the main tree's working copy.
# measurements/ is left out of both (not needed for a build; the scratch volume had 7.5 GiB free). references/ linked.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; REV=ad201a574ddfb90aadb808da7fb67b085ddd5030
for T in base fix; do
  [ -d "$S/t/$T" ] && continue
  mkdir -p "$S/t/$T"; git archive $REV | tar -x -C "$S/t/$T" --exclude measurements; ln -s "$PWD/references" "$S/t/$T/references"
done
for f in tos-api/src/vdi/mouse.rs tos-api/tests/aes_v56_vector_tokens.rs; do cp -p "$f" "$S/t/fix/$f"; echo "fix tree: $f"; done
# the new host test also goes into the base tree, for the red run (method/hosttest.sh)
cp -p tos-api/tests/aes_v56_vector_tokens.rs "$S/t/base/tos-api/tests/"

#!/bin/sh
# Run AFTER run-hosttest.sh's extraction (it failed there on missing inputs), then rerun `cargo xtask test`.
# usage: prepare-inputs.sh <scratch-dir>; all links read-only into the main checkout, nothing in the main repo is touched.
S="$1"; R=/Volumes/Daten/Development/PiRusTOS
cd "$S/src" || exit 1
# doc_check tests call `git ls-files`: the archive has no .git -> scratch repo (scratch only)
git init -q . && git add -A && git -c user.name=t -c user.email=t@t commit -qm scratch
# references/ is gitignored (only README.md is tracked): link every checkout
for d in "$R"/references/*/; do n=$(basename "$d"); [ -e references/$n ] || ln -s "${d%/}" references/$n; done
# sdk/apps/teradesk is gitignored; rsc_cicon_tests reads sdk/apps/teradesk/cicons.rsc
ln -s "$R/sdk/apps/teradesk" sdk/apps/teradesk
export CARGO_TARGET_DIR="$S/target"
start=$(date +%s)
cargo xtask test > "$S/test3.log" 2>&1
echo "exit=$?"; echo "wall_s=$(( $(date +%s) - start ))  (warm target)"

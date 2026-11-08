#!/bin/sh
# V40: what ran, in order, from the main tree; S = session scratchpad (.../scratchpad/v40). Stages:
#   base  scratch tree = git archive of ad201a57, product code unchanged
#   fix1  base + the V40 files of the first pass; fix = fix1 + tos-api/src/aes/window.rs (rect_outline writer)
S=${S:?set S to the session scratchpad}
set -x
cargo xtask deps abline --down; cargo xtask deps abline --up; cargo xtask deps polyline --up   # -> deps/
S=$S method/trees.sh base
S=$S method/builds.sh base                                              # -> $S/kernel-base.img, warnings
# (tosdump/probes/vdprobe.c: t40 appended, L40A-L40Zb, L41A-L41D)
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e1
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-base $S/kernel-base.img
python3 tosdump/probes/vdprobe-compare.py $S/e1 $S/n-base
python3 method/gen-grabs.py $S/e1                                        # -> GRABS of cases_v40.rs
# red: the new test files copied into the base tree
(cd $S/t/base && cargo test -p tos-api --test vdi_reference v40)
# --- the fix in the main tree; fix1 tree = base + the V40 files
S=$S method/trees.sh fix <the V40 files>
# red without the AES writers: the three set_lstlin(true) lines of draw_line/draw_border/menu_bar removed in $S/t/fix, restored after
(cd $S/t/fix && cargo test -p tos-api --test vdi_reference v40 && cargo test -p tos-api --lib v40)
S=$S method/builds.sh fix                                               # (kept as builds/fix1, kernel-fix1.img)
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-fix1 $S/kernel-fix1.img
S=$S method/tables.sh                                                   # base tables (the fix1 tables, superseded, not kept)
# --- final fix: + tos-api/src/aes/window.rs; fix tree rebuilt from scratch
S=$S method/trees.sh fix <the V40 files, + window.rs>
S=$S method/run-fix.sh          # builds fix, probe n-fix + compares, the five fix tables, method/gui.sh
S=$S method/gui-compare.sh      # -> runs/gui/compare.txt
(cd $S/t/base && cargo clippy -p tos-api --tests); (cd $S/t/fix && cargo clippy -p tos-api --tests)
cargo xtask deps --check-mapping
for g in bc480 bc1080; do for s in 00-t006.png 01-t010.png; do python3 method/pngdiff.py $S/runs/gui/$g-base/shots/$s $S/runs/gui/$g-fix/shots/$s; done; done
# red for the window frame writer: set_lstlin removed from rect_outline in $S/t/fix, cargo test -p tos-api --lib v40, restored
S=$S method/final-check.sh      # fix2: builds, probe n-fix2, clippy --tests, cargo xtask test
# the xtask test of final-check.sh ran a stale lib (see TESTS); the V40 files touched in $S/t/fix, then:
(cd $S/t/fix && cargo xtask test)
S=$S method/collect.sh

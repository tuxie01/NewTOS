#!/bin/sh
# V37: what ran, in order, from the main tree; S = session scratchpad (.../scratchpad/v37). Stages:
#   base  tree at f9525a3d, product code unchanged
#   fix   the V37 change in the working tree (uncommitted then)
S=${S:?set S to the session scratchpad}
set -x
# --- base: warnings (dev build + clippy, both boards, fresh target per board) and the rpi4 release kernel
method/builds.sh base                                          # -> $S/kernel-base.img
# (tosdump/probes/vdprobe.c: t37 appended, T3700-T3741)
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh emutos falcontc $S/e1
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-head $S/kernel-base.img
cmp measurements/measurement-2026-11-07-v35/probe/e1/VDPROBE.BIN(.gz, unpacked) $S/e1/VDPROBE.BIN   # EOF on the V35 file
python3 method/gen-grabs.py $S/e1                               # -> GRABS of tos-api/tests/vdi_reference/cases_v37.rs
# red: the new test on the unchanged product code
cargo test -p tos-api --test vdi_reference v37 -- --include-ignored
# --- fix (tos-api/src/vdi/text.rs v_justified)
cargo test -p tos-api --test vdi_reference v37 -- --include-ignored
method/builds.sh fix                                           # -> $S/kernel-fix.img
VDPROBE_SCRATCH=$S/scr tosdump/probes/run-vdprobe.sh newtos $S/n-fix1 $S/kernel-fix.img
python3 tosdump/probes/vdprobe-compare.py $S/e1 $S/n-fix1        # and e1/n-head, n-head/n-fix1
# ZG002/003/008/009 against V41's scratch candidate: git archive HEAD + the V37 files +
#   measurements/measurement-2026-11-06-v41/method/candidate-scratch.patch, host test only
(cd $S/cand-tree && patch -p1 < .../candidate-scratch.patch && cargo test -p tos-api --test vdi_reference v37 -- --include-ignored)
S=$S sh method/tables.sh                                       # base tree = git archive of f9525a3d
cargo xtask deps gdp_justified --up; cargo xtask deps output_text --up; cargo xtask deps --check-mapping
S=$S method/trees.sh; S=$S method/gui.sh
cargo xtask test

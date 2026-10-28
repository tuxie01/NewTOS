#!/bin/zsh
# TOOL04 profile run: the deps-size warning end to end, in a scratch target dir
# (never the main target). Cold fast tier, then 20 000 empty files into
# debug/deps, then the fast and the full tier again; the warning must appear in
# both tiers after the fill and not before, and the exit code must stay 0.
# Usage: guard-demo.sh <rawdir> <scratch target dir>
set -u
raw=$1; export CARGO_TARGET_DIR=$2
cd /Volumes/Daten/Development/PiRusTOS
rm -rf $CARGO_TARGET_DIR
cargo xtask test --fast > $raw/guard-1-cold.log 2>&1; echo "cold fast rc=$?"
perl -e 'for $i (1..20000){open F,">","$ARGV[0]/debug/deps/guard-filler-$i.o"; close F}' $CARGO_TARGET_DIR
cargo xtask test --fast > $raw/guard-2-filled.log 2>&1; echo "filled fast rc=$?"
cargo xtask test > $raw/guard-3-filled-full.log 2>&1; echo "filled full rc=$?"
rm -rf $CARGO_TARGET_DIR

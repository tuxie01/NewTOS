#!/bin/zsh
# TOOL04 profile run: per step, touch tos-api/src/lib.rs (except step 0), run one
# tier of `cargo xtask test` in the main target, record wall time, rc, the test
# totals and the entry count of target/debug/deps afterwards.
# Usage: relink-loop.sh <rawdir> <label> <tier: full|fast> <touch: 0|1>
set -u
root=/Volumes/Daten/Development/PiRusTOS
raw=$1; label=$2; tier=$3; touchit=$4
cd $root
[[ $touchit == 1 ]] && touch tos-api/src/lib.rs
args=(); [[ $tier == fast ]] && args=(--fast)
t0=$(perl -MTime::HiRes=time -e 'printf "%.1f", time')
cargo xtask test $args > $raw/$label.log 2>&1
rc=$?
t1=$(perl -MTime::HiRes=time -e 'printf "%.1f", time')
d=target/debug/deps
n=$(ls -A $d | wc -l | tr -d ' ')
no=$(ls -A $d | grep -c '\.o$')
nd=$(ls -A $d | grep -c '\.dSYM$')
tot=$(grep -E '^test result:' $raw/$label.log | awk '{p+=$4; f+=$6; i+=$8} END {print p"/"f"/"i}')
wall=$(echo "$t1 - $t0" | bc)
printf "%s\t%s\ttouch=%s\twall=%ss\trc=%s\tp/f/i=%s\tdeps=%s\t.o=%s\t.dSYM=%s\twarn=%s\n" \
  $label $tier $touchit $wall $rc $tot $n $no $nd $(grep -c 'xtask test: .* holds' $raw/$label.log) | tee -a $raw/steps.tsv

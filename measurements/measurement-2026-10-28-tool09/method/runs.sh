#!/bin/bash
# runs.sh <scratch> -- the TOOL09 acceptance runs, each one `cargo xtask tosdump` call
# (tools/xtask/src/tosdump.rs), run from the main tree; results into runs/ beside method/.
# <scratch> is the session scratchpad (CLAUDE_JOB_DIR was unset); every run deletes its
# own scratch subdirectory (card, clone, test partitions) at its end.
set -u
S=$1
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
R=measurements/measurement-2026-10-28-tool09/runs
cd "$REPO"

# 1. bios-r1, main tree at HEAD, kernel with the argument tracer -> input check
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out $R/r1-argtrace-head \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-falcontc.BIN \
    --features bios-arg-trace --scratch "$S"

# 2. bios-r2, the disk round, main tree at HEAD, test image built by the command
cargo xtask tosdump bios tosdump/cases/bios-r2.tsv --disk --out $R/r2-disk-head \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk --scratch "$S"

# 3. bios-r1 at the storage audit's commit c3db3747, measured in a tree other than the
#    main tree: a shared local clone in the scratch (no change to the main repository)
git clone --shared --no-checkout -q "$REPO" "$S/tree-c3db"
git -C "$S/tree-c3db" checkout -q --detach c3db3747
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --tree "$S/tree-c3db" --out $R/r1-c3db-clone \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS \
    --reference tosdump/reference/EMUTOS-1a7484ab-BIOS-falcontc --scratch "$S"

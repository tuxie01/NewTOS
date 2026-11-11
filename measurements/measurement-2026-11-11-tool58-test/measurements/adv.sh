#!/bin/bash
# usage: adv.sh name sedscript
cd /Volumes/Daten/Development/PiRusTOS/target-agents/agent-afeacd4da56123236/clone
export CARGO_TARGET_DIR=$PWD/../clone-target
name=$1; shift
python3 -I - "$@" <<'PY'
import sys
f='TASK/A05.md'; s=open(f).read()
mode=sys.argv[1]
if mode=='batch': s=s.replace('owner batch 20,','owner batch '+sys.argv[2]+',',1)
elif mode=='evid': s=s.replace('was IMPLEMENTED with a placeholder','was IMPLEMENTED with a placeholdeX',1)
elif mode=='to': s=s.replace('to PARKED)','to '+sys.argv[2]+')',1)
open(f,'w').write(s)
PY
git add TASK/A05.md
echo "=== $name: status-check (working tree, after stage)"; cargo -q xtask status-check 2>&1 | grep -E "migrated-unverified|rejected|Migrated lines" | cut -c1-330
echo "=== $name: commit"; git commit -q -m "fix(xtask): adversarial $name (A05)" 2>&1 | grep -vE "^$" | cut -c1-330 | head -8; echo "commit exit=${PIPESTATUS[0]}"; git log --oneline -1

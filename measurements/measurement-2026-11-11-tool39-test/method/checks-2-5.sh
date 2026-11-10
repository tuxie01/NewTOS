#!/bin/bash
# usage: checks-2-5.sh <scratch-dir> <repo>; scratch/copy = git archive HEAD + working-tree overlay of the 8 changed
# files + references symlink to the repo's references/ (so the real oracle.env is never edited).
S=$1; R=$2; C=$S/copy; E=tools/oracle/oracle.env
cd $C; cp $E $S/oracle.env.orig
h(){ shasum -a 256 docs/analysis/deps/* | shasum -a 256; }
echo "== check 2: pinned cflow 1.8"; python3 -I tools/deps/gen-deps.py --check; echo "exit $?"
run(){ echo "== $1"; B=$(h)
 for m in "" "--check"; do python3 -I tools/deps/gen-deps.py $m >$S/o.txt 2>$S/e.txt; echo "mode[$m] exit=$? stdout_lines=$(wc -l<$S/o.txt) stderr_lines=$(wc -l<$S/e.txt) tracebacks=$(grep -c Traceback $S/e.txt)"; cat $S/e.txt; done
 echo "deps unchanged: $([ "$B" = "$(h)" ] && echo yes || echo NO); gen-deps temp dirs: $(ls -d ${TMPDIR:-/tmp}/gen-deps-* 2>/dev/null | wc -l)"; }
sed -i '' 's/^ORACLE_CFLOW_VERSION=.*/ORACLE_CFLOW_VERSION=1.7/' $E; run "check 3: pin 1.7"; cp $S/oracle.env.orig $E
sed -i '' 's#^ORACLE_CFLOW=.*#ORACLE_CFLOW=/opt/homebrew/bin/cflow-missing#' $E; run "check 4: missing path"; cp $S/oracle.env.orig $E
echo "== check 5: manifest"
mkdir -p $S/m/mnt; echo hi > $S/m/mnt/A.TXT; head -c 1048576 /dev/zero > $S/m/img.img
$C/tools/sd-manifest.sh files $S/m/mnt C $S/m/img.img && $C/tools/sd-manifest.sh write $S/m/img.img 1 tool39-tester; grep '^host_' $S/m/img.img.manifest
sed -i '' 's#^ORACLE_CFLOW=.*#ORACLE_CFLOW=/opt/homebrew/bin/cflow-missing#' $E
cp $S/m/img.img $S/m/img2.img; $C/tools/sd-manifest.sh files $S/m/mnt C $S/m/img2.img && $C/tools/sd-manifest.sh write $S/m/img2.img 1 tool39-tester; grep '^host_' $S/m/img2.img.manifest
cp $S/oracle.env.orig $E

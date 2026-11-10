#!/bin/bash
# check 1b: break check_cflow in the scratch copy (early `return path, "mutant"`), run the 5 gen_deps tests there.
S=$1; cd $S/copy
python3 -I - <<'P'
p='tools/deps/gen-deps.py'; s=open(p).read()
a='    want = env.get("ORACLE_CFLOW_VERSION")\n'
open(p,'w').write(s.replace(a,a+'    return path, "mutant"\n'))
P
CARGO_TARGET_DIR=$S/target-copy cargo test -p xtask gen_deps 2>&1 | grep -E "^test |test result"

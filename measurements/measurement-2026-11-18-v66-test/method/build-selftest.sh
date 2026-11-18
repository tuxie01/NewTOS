#!/bin/bash
# build-selftest.sh <tree> <name> -- temporary selftest_v66 wired into lib.rs of the tester's own tree, release kernels objcopy'd to kern/<name>-st-<board>.img
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-ab6a60009402c9c41; T=$1; N=$2; M=$(cd $(dirname $0) && pwd)
cd $T
cp $M/selftest_v66.rs kernel/src/selftest_v66.rs
python3 -I - <<'PY'
p='kernel/src/lib.rs'
s=open(p).read()
if 'selftest_v66' not in s:
    s=s.replace("pub mod selftest_raster;","pub mod selftest_raster;\npub mod selftest_v66;",1)
    k="    launch::set_bootflags_from_shiftbits(shiftbits);"
    assert k in s
    s=s.replace(k,"    selftest_v66::run();\n"+k,1)
    open(p,'w').write(s)
PY
for B in rpi4 qemu-virt; do
  cargo xtask build --board $B --release > $A/checks/st-$N-$B.log 2>&1; echo "selftest build $N $B exit $?"
  E=$T/target/aarch64-unknown-none-softfloat/release/kernel; K=$A/kern/$N-st-${B/qemu-virt/virt}.img
  rust-objcopy -O binary $E $K 2>/dev/null || llvm-objcopy -O binary $E $K
done

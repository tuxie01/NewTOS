# usage: warn.sh <dir>   (builds with a fresh target dir, counts warning lines)
d=$1; cd $d; unset CARGO_TARGET_DIR
rm -rf target; mkdir target
L=../warn-$(basename $d); : > $L.summary
for b in rpi4 qemu-virt; do
  cargo xtask build --board $b > $L-build-$b.log 2>&1; echo "build $b rc=$? warnings=$(grep -c '^warning' $L-build-$b.log)" >> $L.summary
done
for b in rpi4 qemu-virt; do
  cargo clippy -p kernel --no-default-features --features board-$b > $L-clippy-$b.log 2>&1; echo "clippy kernel $b rc=$? warnings=$(grep -c '^warning' $L-clippy-$b.log)" >> $L.summary
done
cargo clippy -p xtask --tests > $L-clippy-xtask.log 2>&1; echo "clippy xtask --tests rc=$? warnings=$(grep -c '^warning' $L-clippy-xtask.log)" >> $L.summary
echo done >> $L.summary

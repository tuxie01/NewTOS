# commands as run (2026-10-30); S = scratch dir under the session scratchpad, never /tmp itself
export AXPROBE_SCRATCH=$S CLAUDE_JOB_DIR=$S
make -C tosdump axprobe-newtos                       # "Nothing to be done": build/AXPROBE.NAP already current
rsync -a --exclude .git --exclude target --exclude measurements --exclude references ./ $S/tree/
rm -rf $S/tree/{Applications,hardware,sd-card-backup-*,graphify-out}   # size only
mkdir -p $S/tree/target/programs/qemu-virt; cp tosdump/build/AXPROBE.NAP $S/tree/target/programs/qemu-virt/
# edit $S/tree/kernel/embed.rs: NEWDESK entry file -> AXPROBE.NAP (method/embed-probe-scratch.diff; sed also hit the
# test list at line 120, harmless)
(cd $S/tree && cargo xtask build --board qemu-virt --release)
llvm-objcopy -O binary $S/tree/target/aarch64-unknown-none-softfloat/release/kernel $S/kernel.img   # rustup stable toolchain's llvm-objcopy
for n in 1 2 3; do tosdump/probes/run-axprobe.sh newtos-virt <this run dir>/runs/newtos-virt-$n $S/kernel.img; done

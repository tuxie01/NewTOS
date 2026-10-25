#!/usr/bin/env bash
# virt.sh <wt> <tag> -- qemu-virt dev boot via cargo xtask run, serial captured, killed after 70 s (no gtimeout on macOS)
A=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/a117-test
cd "$1" && touch kernel/src/lib.rs tos-api/src/lib.rs
cargo xtask build --board qemu-virt > $A/serial/virt-$2-build.log 2>&1 || { echo build failed; exit 1; }
( cargo xtask run --board qemu-virt < /dev/null > $A/serial/virt-$2.log 2>&1 & )
sleep 80
pkill -f "$1/target.*kernel.img" ; sleep 1; pkill -f "$1/target.*kernel.img"; pkill -f "xtask run --board qemu-virt"
echo done $2

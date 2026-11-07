#!/bin/bash
# clippy.sh <tree> <out-prefix> : cargo clippy -p kernel for both boards in <tree> (adapted from the V50
# run's method/clippy.sh); prints the warning totals and any hit in the files V51 touches.
T=$1; O=$2
cd "$T" || exit 1
for b in rpi4 qemu-virt; do
  cargo clippy -p kernel --features board-$b > "$O-$b.txt" 2>&1
  echo "clippy $b rc $?"
  grep -E "generated [0-9]+ warning" "$O-$b.txt"
  grep -E -- "--> kernel/src/(raster_copy|raster_kernel|raster_expand|raster_prof|vdi_backend|lib)\.rs" "$O-$b.txt" || echo "  0 hits in kernel/src/{raster_copy,raster_kernel,raster_expand,raster_prof,vdi_backend,lib}.rs"
done

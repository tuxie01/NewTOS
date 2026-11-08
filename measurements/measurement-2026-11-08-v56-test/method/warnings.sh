#!/bin/sh
# V56: warning counts per crate from the logs of builds.sh (rustc "warning: `<crate>` (lib) generated N warnings").
S=${S:?}
for L in base fix; do for f in build-rpi4 build-qemu-virt clippy-rpi4 clippy-qemu-virt; do
  printf '%-5s %-16s %s\n' $L $f "$(grep -E '^warning: `(kernel|tos-api|tos-types)` \(lib\) generated' $S/builds/$L/$f.log | sed -E 's/^warning: `([^`]*)` \(lib\) generated ([0-9]+) warnings?.*/\1=\2/' | tr '\n' ' ') errors=$(grep -c '^error' $S/builds/$L/$f.log)"
done; done

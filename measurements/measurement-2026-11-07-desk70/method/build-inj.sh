#!/bin/bash
# build-inj.sh <scratch> <actions> <tag> -- fresh release rpi4 kernel with the injector script in the SCRATCH clone; host_env.rs restored and compared afterwards
set -eu
S=$1; A=$2; T=$3; M=$(cd "$(dirname "$0")" && pwd); W=$S/wt; export SOURCE_DATE_EPOCH=1792152000
python3 "$M/mk-injector.py" "$W" "$A"
(cd "$W" && touch kernel/src/main.rs tos-api/src/lib.rs && cargo xtask build --board rpi4 --release > "$S/build-$T.log" 2>&1) || { tail -20 "$S/build-$T.log"; cp "$S/host_env.rs.pristine" "$W/kernel/src/host_env.rs"; exit 1; }
cp "$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/kern/kernel8-$T.img"; cp "$S/host_env.rs.pristine" "$W/kernel/src/host_env.rs"
cmp "$W/kernel/src/host_env.rs" "$S/host_env.rs.pristine"; shasum -a 256 "$S/kern/kernel8-$T.img"

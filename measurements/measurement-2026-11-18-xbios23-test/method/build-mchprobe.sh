#!/usr/bin/env bash
# build-mchprobe.sh <out.NCP> [SET_VDO-value] -- build the ARCH06 probe (method/mchprobe.c) as a
# console program linked like every mintlib client: sdk/libc/crt0.S + mchprobe.o + sdk/libc/libc.a
# (Getcookie = sdk/libc/newtos-arch/getcookie.c, sysinfo/uname = the SDK03 overlay). Flags as
# sdk/overlays/teradesk/tests/build-cookie-probe.sh (TeraDesk's CFLAGS minus -D_TOS_ONLY).
# With a second argument the probe is compiled with -DSET_VDO=<value> (the _VDO injection side).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WS="$(cd "$HERE/../../.." && pwd)"; SDK="$WS/sdk"; LIBC="$SDK/libc"; MINT="$LIBC/mintlib/include"
OUT="${1:?usage: build-mchprobe.sh <out.NCP> [SET_VDO]}"
DEF=(); [ -n "${2:-}" ] && DEF=(-DSET_VDO="$2")
[ -f "$LIBC/libc.a" ] || { echo "error: $LIBC/libc.a missing" >&2; exit 1; }
CLANG="${CLANG:-clang}"; RES="$($CLANG -print-resource-dir)/include"
. "$SDK/toolchain.sh"; LLD="${LLD:-$(newtos_tool ld.lld)}"
CFLAGS=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -Os -fomit-frame-pointer -fPIE
        -fno-builtin -nostdinc -funsigned-char -D__NEWTOS__ -Wno-macro-redefined)
W="$(mktemp -d)"; trap 'rm -rf "$W"' EXIT
"$CLANG" "${CFLAGS[@]}" ${DEF[@]+"${DEF[@]}"} -I"$MINT" -isystem "$RES" -Wall -c "$HERE/mchprobe.c" -o "$W/probe.o"
"$CLANG" "${CFLAGS[@]}" -I"$SDK/include" -I"$MINT" -c "$LIBC/crt0.S" -o "$W/crt0.o"
mkdir -p "$(dirname "$OUT")"
newtos_lld -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none \
  "$W/crt0.o" "$W/probe.o" --start-group "$LIBC/libc.a" --end-group -o "$OUT"
echo "probe ${2:+(SET_VDO=$2) }: $OUT $(shasum -a 256 "$OUT" | cut -c1-64)"
echo "libc.a: $(shasum -a 256 "$LIBC/libc.a" | cut -c1-64)"

#!/bin/sh
# build-emutos.sh -- build one EmuTOS oracle ROM (`make 512 COUNTRY=de`).
#
#   tools/oracle/build-emutos.sh <commit> <yyyy-mm-dd> <tree-dir>
#
# <tree-dir> is a clone of references/emutos checked out at <commit>; it is
# created when missing. references/emutos itself is only read (cloned from),
# never built in or checked out.
#
# Two things the EmuTOS Makefile takes from the host are pinned here, because
# each would otherwise make two builds of the same commit differ:
#
#   * The build date. tools/mkheader.awk asks `date +%Y,%m,%d` and writes it
#     into the OS header (os_date, os_dosdate) and the copyright year; a
#     snapshot tree's version.mk also sets VERSION = `date +%Y%m%d` (1a7484ab:
#     version.mk:40), the version string on the boot screen. A `date` shim in
#     front of PATH answers every formatted query (`date +FORMAT`) for
#     <yyyy-mm-dd>; any other `date` call goes to /bin/date.
#   * The host tools' architecture. tools/bug.c (VERSION_1_4 and 1a7484ab
#     alike) allocates len+3 bytes for "<name>.tr.c" where it needs len+4;
#     macOS arm64's allocator aborts on that one-byte overflow (bios/kprint.c
#     is the first victim). Built as x86_64 under Rosetta, the overflow lands
#     in allocator slack and the translated output is identical to an arm64
#     ASan build's (docs/analysis/ORACLE.md). Only the host tools are affected;
#     the ROM is compiled by $ORACLE_CROSS_MINT with EmuTOS's own flags.
#
# The cross compiler comes from oracle.env as an absolute prefix; /opt/cross-mint
# is not expected on PATH.
set -eu

[ $# -eq 3 ] || { echo "usage: $0 <commit> <yyyy-mm-dd> <tree-dir>" >&2; exit 2; }
COMMIT=$1; DATE=$2; TREE=$3

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
. "$HERE/oracle.env"

case $DATE in
    [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]) ;;
    *) echo "date must be yyyy-mm-dd: $DATE" >&2; exit 2 ;;
esac
[ -x "${ORACLE_CROSS_MINT}gcc" ] || { echo "no cross gcc: ${ORACLE_CROSS_MINT}gcc" >&2; exit 1; }

if [ ! -d "$TREE/.git" ]; then
    git clone -q --no-checkout "$REPO/references/emutos" "$TREE"
fi
git -C "$TREE" checkout -q "$COMMIT"
[ "$(git -C "$TREE" rev-parse HEAD)" = "$(git -C "$REPO/references/emutos" rev-parse "$COMMIT^{commit}")" ] \
    || { echo "checkout of $COMMIT failed in $TREE" >&2; exit 1; }
[ -z "$(git -C "$TREE" status --porcelain)" ] \
    || { echo "$TREE has local changes; refusing to build" >&2; exit 1; }

SHIM=$(mktemp -d)
trap 'rm -rf "$SHIM"' EXIT
cat > "$SHIM/date" <<EOF
#!/bin/sh
case "\$#:\$1" in
    1:+*) exec /bin/date -j -f %Y-%m-%d "$DATE" "\$1" ;;
esac
exec /bin/date "\$@"
EOF
chmod +x "$SHIM/date"

cd "$TREE"
PATH="$SHIM:/usr/bin:/bin:/usr/sbin:/sbin" make clean >/dev/null
PATH="$SHIM:/usr/bin:/bin:/usr/sbin:/sbin" make 512 COUNTRY=de \
    TOOLCHAIN_PREFIX="$ORACLE_CROSS_MINT" \
    BUILD_TOOLS_OPTFLAGS="-O -arch x86_64"

echo "commit  $(git rev-parse HEAD)"
echo "date    $DATE  (OS_DATE $(sed -n 's/^#define OS_DATE //p' obj/header.h), version $(sed -n 's/^const char version\[\] = \(.*\);/\1/p' obj/version.c))"
echo "gcc     $("${ORACLE_CROSS_MINT}gcc" --version | head -1)"
shasum -a 256 etos512de.img
# The cross-check: a rebuild of the pinned commit must reproduce the committed
# binary bit for bit.
if [ "$(git rev-parse HEAD)" = "$ORACLE_EMUTOS_COMMIT" ]; then
    if [ "$(shasum -a 256 etos512de.img | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ]; then
        echo "matches ORACLE_ROM_SHA256 ($ORACLE_ROM)"
    else
        echo "DOES NOT MATCH ORACLE_ROM_SHA256 ($ORACLE_ROM)" >&2
        exit 1
    fi
fi

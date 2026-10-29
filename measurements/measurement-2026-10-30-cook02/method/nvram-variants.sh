#!/bin/sh
# nvram-variants.sh <scratch> <runs-dir> -- COOK02: tosdump/cases/jar-r1.tsv (jar:_AKP, jar:_IDT among
# others) on the pinned oracle, profile tt, with four variants of the [Keyboard] section of
# tools/oracle/hatari.cfg. Hatari's NVRAM initialisation copies nLanguage / nKbdLayout into NVRAM bytes
# 20/21 when they are not -1 (references: Mushasi68k/hatari src/falcon/nvram.c NvRam_Init).
# The variants are scratch copies of hatari.cfg; tools/hatari-oracle.sh is run through a scratch copy that
# differs from the original in two lines only: HERE is the repo's tools/ dir, and ORACLE_HATARI_CFG is
# taken from $COOK02_CFG after oracle.env is read (oracle.env assigns it unconditionally).
set -u
S=$1; R=$2
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
T=tosdump/cases/jar-r1.tsv
mkdir -p "$S/dumpers/b" "$R"
make -s -C tosdump BUILD="$S/dumpers/b" dumpbios-m68k > "$S/make.txt" 2>&1 || { cat "$S/make.txt"; exit 1; }
make -s -C tosdump/host build/tosdiff > "$S/make-tosdiff.txt" 2>&1 || exit 1
echo "DUMPBIOS.PRG $(shasum -a 256 "$S/dumpers/b/DUMPBIOS.PRG" | cut -d' ' -f1)"
sed -e "s|^HERE=.*|HERE=$REPO/tools|" \
    -e 's|^\. "\$HERE/oracle/oracle.env"|. "$HERE/oracle/oracle.env"; ORACLE_HATARI_CFG=$COOK02_CFG|' \
    tools/hatari-oracle.sh > "$S/hatari-oracle-cfg.sh"
chmod +x "$S/hatari-oracle-cfg.sh"
# name:nLanguage:nKbdLayout
for V in base:-1:-1 kbd1:-1:1 lang2:2:-1 lang2kbd2:2:2; do
    N=${V%%:*}; REST=${V#*:}; L=${REST%%:*}; K=${REST#*:}
    sed -e "s|^nKbdLayout = .*|nKbdLayout = $K|" -e "s|^nLanguage = .*|nLanguage = $L|" \
        tools/oracle/hatari.cfg > "$S/hatari-$N.cfg"
    echo "== $N: $(/usr/bin/grep -A3 '^\[Keyboard\]' "$S/hatari-$N.cfg" | tr '\n' ' ')"
    COOK02_CFG="$S/hatari-$N.cfg" "$S/hatari-oracle-cfg.sh" tt "$S/dumpers/b/DUMPBIOS.PRG" "$T" "$S/emu-$N" > "$S/emu-$N.stdout" 2>&1
    echo "$N: exit $?"
    mkdir -p "$R/$N"
    cp "$S/emu-$N/BIOS.BIN" "$S/emu-$N/BIOS.LOG" "$S/emu-$N/profile.txt" "$S/hatari-$N.cfg" "$R/$N/"
    python3 -I tosdump/../measurements/measurement-2026-10-29-tool18/method/decode-jar.py "$R/$N/BIOS.BIN" "$T" > "$R/$N/decoded.txt"
    /usr/bin/grep -E "_AKP|_IDT" "$R/$N/decoded.txt"
done

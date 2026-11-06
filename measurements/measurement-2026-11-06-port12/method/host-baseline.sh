#!/bin/bash
# PORT12 Q1: upstream XGDOS host tests, built on the Mac from a scratch copy.
# Same compile lines as the upstream Makefile target host-test (read first; not run from the clone).
# usage: host-baseline.sh <xgdos clone> <scratch dir> <out dir> [extra cc flags]
set -u
R=$1; S=$2; OUT=$3; shift 3; EXTRA="$*"
mkdir -p "$OUT"
rm -rf "$S"; mkdir -p "$S"; (cd "$R" && tar cf - --exclude=.git .) | tar xf - -C "$S"
cd "$S" || exit 1
HOSTSRC=""; for s in xfix sintab charmap spd raster xstd heap config fontmgr gcache text vdi pdf soft mem meta sysfont; do HOSTSRC="$HOSTSRC src/$s.c"; done
run() { # name assign_mode
  cc -O1 -DXG_HOST -w $EXTRA -o host/test_vdi host/test_vdi.c host/xos_host.c host/hostvdi.c $HOSTSRC > "$OUT/$1-cc-vdi.txt" 2>&1; echo "cc test_vdi rc=$?"
  cc -O1 -DXG_HOST -w $EXTRA -o host/test_dev host/test_dev.c host/xos_host.c host/hostvdi.c $HOSTSRC > "$OUT/$1-cc-dev.txt" 2>&1; echo "cc test_dev rc=$?"
  (cd host; ./test_vdi > "$OUT/$1-vdi.out" 2>&1; echo "test_vdi rc=$?"; ./test_dev > "$OUT/$1-dev.out" 2>&1; echo "test_dev rc=$?")
}
echo "== A: upstream ASSIGN.SYS unchanged, no fonts in hostfs/C/FONTS (the .SPD are gitignored upstream)"
run a-nofonts
find host/hostfs -type f | sort > "$OUT/a-nofonts-files.txt"
echo "== B: fonts from upstream GEMSYS copied to hostfs/C/FONTS, every FONT*.SPD enabled (no +/- lines: enable all)"
cp "$R"/GEMSYS/*.SPD host/hostfs/C/FONTS/
grep -v '^[+-]' host/hostfs/C/ASSIGN.SYS > host/hostfs/C/ASSIGN.NEW; mv host/hostfs/C/ASSIGN.NEW host/hostfs/C/ASSIGN.SYS
rm -f host/hostfs/C/PRINTS/*.PDF host/hostfs/C/*.GEM host/out_*.pgm
run b-fonts
find host/hostfs -type f | sort > "$OUT/b-fonts-files.txt"
mkdir -p "$OUT/b-artifacts"; cp host/out_*.pgm host/hostfs/C/PRINTS/*.PDF host/hostfs/C/*.GEM "$OUT/b-artifacts/" 2>/dev/null
(cd "$OUT/b-artifacts" && shasum -a 256 * > SHA256SUMS)

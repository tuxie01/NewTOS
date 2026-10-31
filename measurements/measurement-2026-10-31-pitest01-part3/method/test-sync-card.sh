#!/usr/bin/env bash
# test-sync-card.sh <scratch-dir> <kernels-dir> <out-dir> -- exercise tools/sync-card.sh against a scratch card image,
# never a real card (TASK/PITEST01 part 3; part 2's test plus cmdline.txt per kernel and the backups).
# The image: MBR, s1 FAT16 "PT3BOOT" = boot partition AND C: (as on the Pi card, where the
# firmware partition is C:), s2 FAT16 "PT3D" = D:. Fake firmware files (random bytes), boot/rpi4/config.txt,
# a cmdline.txt "asid=on", and an "old" card state (old kernel8.img, an old QED.NAP, user files) so that the dry
# run shows new / changed / same / extra. Every sync call carries --cmdline kernela.img method/cmdline-a.txt
# --cmdline kernelb.img method/cmdline-b.txt (+ --allow-boot-config unless the step tests its absence).
# Steps (one log per step in <out-dir>/<step>.txt, exit status in its last line):
#   r1-r5    refusals: wrong label, C: on s2, a boot partition without firmware, config.txt kernel= other file,
#            a list naming a firmware file
#   r6       --cmdline without --allow-boot-config, even with --write: refused, the card unchanged
#   r7       config.txt cmdline=other.txt: refused
#   r8       --active kernelb.img with a --cmdline for kernela.img only: refused
#   r9       --cmdline for a kernel that is not in KERNELS.TXT: refused
#   s1       dry run (nothing written: the card's file list + sha256 before and after are equal)
#   s2       phase A, --write --active kernela.img: copy, remount, verify, manifest; cmdline.txt = cmdline-a.txt;
#            card-before/C/ holds the card's old kernel8.img, cmdline.txt ("asid=on"), config.txt and the
#            replaced QED.NAP, each equal to the card before s2; afterwards no AppleDouble
#            (._*) file on the card except a foreign one planted before (left alone)
#   s3       dry run again: every listed file `same`
#   c1-c4    corruption: one byte of C:\P0REP.NAP flipped in the RAW IMAGE while it is detached (the Mac cannot
#            have it cached); c1 verify must fail, c2 dry run must say `changed`; c3 --write repairs it; c4 verify ok
#   c5       a byte flipped through the mounted file system: verify must fail; then repaired by c6 --write
#   p1       phase B: --active kernelb.img --write: kernel8.img = kernelb.img, cmdline.txt = cmdline-b.txt
#            (serial=on); card-before/C/ holds kernel8.img = kernela.img and cmdline.txt = cmdline-a.txt
#   p2       same run dir again: refused (card.manifest); p3 a run dir with card-before/ only: refused
#   a1       cmdline.txt deleted on the card, back to phase A: cmdline.txt `new`, manifest says before = absent
#   f1       fsck_msdos -n of both partitions after the runs
# The image is detached and deleted at the end (also on failure).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"
S="${1:?scratch dir}"; K="${2:?kernels dir}"; O="${3:?out dir}"
mkdir -p "$S" "$O"; S="$(cd "$S" && pwd)"; O="$(cd "$O" && pwd)"; K="$(cd "$K" && pwd)"
SYNC="$WS/tools/sync-card.sh"; LIST="$HERE/pitest01-card.list"
IMG="$S/card.img"; B=/Volumes/PT3BOOT; D=/Volumes/PT3D
CLA="$HERE/cmdline-a.txt"; CLB="$HERE/cmdline-b.txt"
DEV=
detach() { [ -z "$DEV" ] || hdiutil detach "$DEV" >/dev/null 2>&1 || hdiutil detach -force "$DEV" >/dev/null 2>&1; DEV=; }
trap 'detach; rm -f "$IMG"' EXIT
attach() {
  DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage "$IMG" 2>/dev/null | head -1 | awk '{print $1}')
  for _ in 1 2 3 4 5 6 7 8 9 10; do [ -d "$B" ] && [ -d "$D" ] && return; sleep 1; done
  echo "image did not mount" >&2; exit 1
}
step() {   # <name> <expected rc: 0|nonzero> <command...>
  local n="$1" want="$2" rc; shift 2
  (cd "$WS" && "$@") > "$O/$n.txt" 2>&1; rc=$?
  echo "exit $rc" >> "$O/$n.txt"
  if { [ "$want" = 0 ] && [ "$rc" = 0 ]; } || { [ "$want" = nonzero ] && [ "$rc" != 0 ]; }; then
    echo "$n ok (rc=$rc)"
  else
    echo "$n UNEXPECTED rc=$rc (wanted $want)"; FAILS=$((FAILS + 1))
  fi
}
cardlist() { (cd "$B" && find . \( -name .Spotlight-V100 -o -name .fseventsd \) -prune -o -type f -exec shasum -a 256 {} + | sort -k2) ; }
FAILS=0
CARD0=(--list "$LIST" --boot "$B" PT3BOOT --drive C "$B" PT3BOOT --drive D "$D" PT3D --kernels "$K")
CARD=("${CARD0[@]}" --cmdline kernela.img "$CLA" --cmdline kernelb.img "$CLB" --allow-boot-config)
check() {   # <label> <command...>: one assertion
  local lbl="$1"; shift
  if "$@"; then echo "$lbl ok"; else echo "$lbl FAILED"; FAILS=$((FAILS + 1)); fi
}
shaof() { shasum -a 256 "$1" | awk '{print $1}'; }

# ---- the scratch card --------------------------------------------------------------------------------------------
rm -f "$IMG"; dd if=/dev/zero of="$IMG" bs=1m count=160 status=none
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" 2>/dev/null | head -1 | awk '{print $1}')
diskutil partitionDisk "$DEV" 2 MBR "MS-DOS FAT16" PT3BOOT 96M "MS-DOS FAT16" PT3D R > /dev/null
[ -d "$B" ] && [ -d "$D" ] || { echo "partitions not mounted" >&2; exit 1; }
export COPYFILE_DISABLE=1
mdutil -i off "$B" >/dev/null 2>&1; mdutil -i off "$D" >/dev/null 2>&1
head -c 2253056 /dev/urandom > "$B/start4.elf"; head -c 5399 /dev/urandom > "$B/fixup4.dat"
head -c 52000 /dev/urandom > "$B/bcm2711-rpi-4-b.dtb"
cp -X "$WS/boot/rpi4/config.txt" "$B/config.txt"
printf 'asid=on\n' > "$B/cmdline.txt"
head -c 1400000 /dev/urandom > "$B/kernel8.img"                    # "old" kernel
mkdir -p "$B/APPS/QED" "$B/DOOM"; head -c 500000 /dev/urandom > "$B/APPS/QED/QED.NAP"   # "old" QED, other case
cp -X "$WS/sdk/apps/qed/build/qed.rsc" "$B/APPS/QED/QED.RSC"     # same content, other case
printf '#R 02\r\n' > "$B/NEWDESK.INF"; head -c 4096 /dev/urandom > "$B/DOOM/DOOM1.WAD"
printf 'D: user file\r\n' > "$D/NOTES.TXT"
rm -rf "$B/.fseventsd" "$D/.fseventsd" "$B/.Spotlight-V100" "$D/.Spotlight-V100" 2>/dev/null; sync
# A clean card has no AppleDouble files; then one foreign ._ file the tool must leave alone.
find "$B" "$D" -name '._*' -delete 2>/dev/null; printf 'foreign\r\n' > "$B/._FOREIGN"; sync
testclean() { find "$B" "$D" -name '._*' ! -name ._FOREIGN -delete 2>/dev/null; sync; }   # after the test's own edits
testclean
echo "image $IMG attached at $DEV"

# ---- refusals ------------------------------------------------------------------------------------------------
step r1 nonzero "$SYNC" sync --list "$LIST" --boot "$B" BOOT --drive C "$B" BOOT
step r2 nonzero "$SYNC" sync --list "$LIST" --boot "$B" PT3BOOT --drive C "$D" PT3D
step r3 nonzero "$SYNC" sync --list "$LIST" --boot "$D" PT3D --drive C "$B" PT3BOOT
cp -X "$B/config.txt" "$S/config.keep"; sed -i '' 's/^kernel=kernel8.img/kernel=kernel7.img/' "$B/config.txt"
step r4 nonzero "$SYNC" sync "${CARD[@]}" --active kernela.img
cp -X "$S/config.keep" "$B/config.txt"; cmp -s "$S/config.keep" "$B/config.txt" || { echo "config.txt not restored"; exit 1; }
testclean
printf 'BOOT:start4.elf | %s | none\n' "$LIST" > "$S/bad.list"
step r5 nonzero "$SYNC" sync --list "$S/bad.list" --boot "$B" PT3BOOT --drive C "$B" PT3BOOT
cardlist > "$O/r6-card-before.sha256"
step r6 nonzero "$SYNC" sync "${CARD0[@]}" --active kernela.img --cmdline kernela.img "$CLA" --cmdline kernelb.img "$CLB" \
  --run-dir "$S/run-r6" --write
cardlist > "$O/r6-card-after.sha256"
check "r6 card unchanged" cmp -s "$O/r6-card-before.sha256" "$O/r6-card-after.sha256"
check "r6 names the switch" grep -q 'cmdline.txt: boot configuration, needs --allow-boot-config' "$O/r6.txt"
check "r6 no card-before" test ! -e "$S/run-r6/card-before"
printf 'cmdline=other.txt\n' >> "$B/config.txt"
step r7 nonzero "$SYNC" sync "${CARD[@]}" --active kernela.img
check "r7 names cmdline=" grep -q 'config.txt reads the command line from another file' "$O/r7.txt"
cp -X "$S/config.keep" "$B/config.txt"; cmp -s "$S/config.keep" "$B/config.txt" || { echo "config.txt not restored"; exit 1; }
testclean
step r8 nonzero "$SYNC" sync "${CARD0[@]}" --active kernelb.img --cmdline kernela.img "$CLA" --allow-boot-config
check "r8 names the missing cmdline" grep -q -- '--active kernelb.img has no --cmdline' "$O/r8.txt"
step r9 nonzero "$SYNC" sync "${CARD[@]}" --active kernela.img --cmdline kernelc.img "$CLB"
check "r9 names the unknown kernel" grep -q -- '--cmdline kernelc.img: no such kernel' "$O/r9.txt"

# ---- dry run, write, dry run -------------------------------------------------------------------------------
cardlist > "$O/s1-card-before.sha256"
OLDK=$(shaof "$B/kernel8.img"); OLDCL=$(shaof "$B/cmdline.txt"); OLDCFG=$(shaof "$B/config.txt"); OLDQED=$(shaof "$B/APPS/QED/QED.NAP")
step s1 0 "$SYNC" sync "${CARD[@]}" --active kernela.img
cardlist > "$O/s1-card-after.sha256"
if cmp -s "$O/s1-card-before.sha256" "$O/s1-card-after.sha256"; then echo "s1 card unchanged ok"; else echo "s1 CARD CHANGED"; FAILS=$((FAILS + 1)); fi
check "s1 cmdline.txt changed" grep -q 'changed  C:\\cmdline.txt <- measurements/measurement-2026-10-31-pitest01-part3/method/cmdline-a.txt' "$O/s1.txt"
check "s1 HYP_VIEW.CFG new" grep -q 'new      C:\\Apps\\HYPVIEW\\HYP_VIEW.CFG' "$O/s1.txt"
check "s1 ZVIEW.HYP new" grep -q 'new      C:\\Apps\\ZVIEW\\ZVIEW.HYP' "$O/s1.txt"
step s2 0 "$SYNC" sync "${CARD[@]}" --active kernela.img --run-dir "$S/run-s2" --write
cp "$S/run-s2/card.manifest" "$O/s2-card.manifest"
(cd "$S/run-s2/card-before" && find . -type f -exec shasum -a 256 {} + | sort -k2) > "$O/s2-card-before.sha256"
SB="$S/run-s2/card-before/C"
check "s2 backup kernel8.img = old card kernel" test "$(shaof "$SB/kernel8.img")" = "$OLDK"
check "s2 backup cmdline.txt = old card cmdline (asid=on)" test "$(shaof "$SB/cmdline.txt")" = "$OLDCL"
check "s2 backup config.txt = card config" test "$(shaof "$SB/config.txt")" = "$OLDCFG"
check "s2 backup APPS/QED/QED.NAP = old card QED" test "$(shaof "$SB/APPS/QED/QED.NAP")" = "$OLDQED"
check "s2 card cmdline.txt = cmdline-a.txt" cmp -s "$B/cmdline.txt" "$CLA"
check "s2 card kernel8.img = kernela.img" cmp -s "$B/kernel8.img" "$K/kernela.img"
grep -q 'extra    D:\\NOTES.TXT' "$O/s1.txt" && echo "s1 lists D:\\NOTES.TXT as extra ok" || { echo "s1 NOTES.TXT NOT extra"; FAILS=$((FAILS + 1)); }
for k in '^host_clang ' '^tree_commit [0-9a-f]\{40\} tree_dirty=' '^kernel kernela.img [0-9a-f]\{64\} features=none ' \
         '^kernel kernelb.img [0-9a-f]\{64\} features=crash-nowait,fs-io-trace ' '^kernel_active kernela.img' \
         '^optional_missing none' '^volume C PT3BOOT ' '^volume D PT3D ' '^replaced .* C:\\kernel8.img' '^extra .* D:\\NOTES.TXT' \
         '^cmdline kernela.img measurements/measurement-2026-10-31-pitest01-part3/method/cmdline-a.txt [0-9a-f]\{64\}$' \
         '^cmdline kernelb.img measurements/measurement-2026-10-31-pitest01-part3/method/cmdline-b.txt [0-9a-f]\{64\}$' \
         '^cmdline_active kernela.img -> cmdline.txt$' '^boot_cmdline empty (no options)$' '^boot_cmdline_before asid=on$' \
         '^replaced .* C:\\cmdline.txt' '^saved .* C:\\kernel8.img -> card-before/C/kernel8.img$' \
         '^saved .* C:\\cmdline.txt -> card-before/C/cmdline.txt$' '^saved .* C:\\config.txt -> card-before/C/config.txt$' \
         '^file .* C:\\cmdline.txt$' '^file .* C:\\Apps\\HYPVIEW\\HYP_VIEW.CFG$' '^file .* C:\\Apps\\ZVIEW\\ZVIEW.HYP$'; do
  grep -q "$k" "$O/s2-card.manifest" || { echo "s2 manifest lacks $k"; FAILS=$((FAILS + 1)); }
done
cmp -s "$O/s2-card.manifest" "$B/CARDMAN.TXT" && echo "s2 C:\\CARDMAN.TXT = run-dir card.manifest ok" ||
  { echo "s2 CARDMAN.TXT differs"; FAILS=$((FAILS + 1)); }
dotunder() { find "$B" "$D" \( -name .Spotlight-V100 -o -name .fseventsd \) -prune -o -name '._*' -print 2>/dev/null | sort; }
[ "$(dotunder)" = "$B/._FOREIGN" ] && echo "s2 no AppleDouble left, ._FOREIGN kept ok" ||
  { echo "s2 APPLEDOUBLE: $(dotunder | tr '\n' ' ')"; FAILS=$((FAILS + 1)); }
step s3 0 "$SYNC" sync "${CARD[@]}" --active kernela.img
grep -q 'summary: 0 new, 0 changed' "$O/s3.txt" && echo "s3 all same ok" || { echo "s3 NOT all same"; FAILS=$((FAILS + 1)); }

# ---- corruption in the raw image (detached: no cache can hide it) ------------------------------------------
P0REP="$WS/sdk/examples/build/P0REP.NAP"
detach
OFF=$(python3 -I - "$IMG" "$P0REP" <<'PY'
import sys
img = open(sys.argv[1], 'rb').read(); f = open(sys.argv[2], 'rb').read()
# a 64-byte window of the file that occurs exactly once in the image, at least 4 KiB into the file
for start in range(4096, len(f) - 64, 512):
    w = f[start:start + 64]
    if img.count(w) == 1 and f.count(w) == 1:
        print(img.find(w) + 32); break
PY
)
[ -n "$OFF" ] || { echo "no unique window of P0REP.NAP in the image"; exit 1; }
python3 -I - "$IMG" "$OFF" <<'PY'
import sys
p, off = sys.argv[1], int(sys.argv[2])
with open(p, 'r+b') as fh:
    fh.seek(off); b = fh.read(1); fh.seek(off); fh.write(bytes([b[0] ^ 0xFF]))
PY
echo "flipped image byte $OFF (inside C:\\P0REP.NAP)" | tee "$O/c0-corrupt.txt"
attach
step c1 nonzero "$SYNC" verify "${CARD[@]}" --active kernela.img
step c2 0 "$SYNC" sync "${CARD[@]}" --active kernela.img
grep -q 'changed  C:\\P0REP.NAP' "$O/c2.txt" && echo "c2 shows P0REP changed ok" || { echo "c2 P0REP NOT changed"; FAILS=$((FAILS + 1)); }
step c3 0 "$SYNC" sync "${CARD[@]}" --active kernela.img --run-dir "$S/run-c3" --write
cp "$S/run-c3/card.manifest" "$O/c3-card.manifest"
step c4 0 "$SYNC" verify "${CARD[@]}" --active kernela.img

# ---- corruption through the mounted file system ------------------------------------------------------------
printf 'X' | dd of="$B/AUTO/DCPROBE.NCP" bs=1 seek=100 conv=notrunc status=none; sync; testclean
step c5 nonzero "$SYNC" verify "${CARD[@]}" --active kernela.img
step c6 0 "$SYNC" sync "${CARD[@]}" --active kernela.img --run-dir "$S/run-c6" --write

# ---- phase switch ---------------------------------------------------------------------------------------------
step p1 0 "$SYNC" sync "${CARD[@]}" --active kernelb.img --run-dir "$S/run-p1" --write
cp "$S/run-p1/card.manifest" "$O/p1-card.manifest"
(cd "$S/run-p1/card-before" && find . -type f -exec shasum -a 256 {} + | sort -k2) > "$O/p1-card-before.sha256"
check "p1 card cmdline.txt = cmdline-b.txt (serial=on)" cmp -s "$B/cmdline.txt" "$CLB"
check "p1 card kernel8.img = kernelb.img" cmp -s "$B/kernel8.img" "$K/kernelb.img"
check "p1 backup kernel8.img = kernela.img" cmp -s "$S/run-p1/card-before/C/kernel8.img" "$K/kernela.img"
check "p1 backup cmdline.txt = cmdline-a.txt" cmp -s "$S/run-p1/card-before/C/cmdline.txt" "$CLA"
for k in '^kernel_active kernelb.img' '^cmdline_active kernelb.img -> cmdline.txt$' '^boot_cmdline serial=on$' \
         '^boot_cmdline_before empty (no options)$' '^replaced .* C:\\cmdline.txt' '^replaced .* C:\\kernel8.img'; do
  grep -q "$k" "$O/p1-card.manifest" || { echo "p1 manifest lacks $k"; FAILS=$((FAILS + 1)); }
done
[ "$(dotunder)" = "$B/._FOREIGN" ] && echo "p1 no AppleDouble left, ._FOREIGN kept ok" ||
  { echo "p1 APPLEDOUBLE: $(dotunder | tr '\n' ' ')"; FAILS=$((FAILS + 1)); }
step p2 nonzero "$SYNC" sync "${CARD[@]}" --active kernelb.img --run-dir "$S/run-p1" --write   # run dir reused
mkdir -p "$S/run-p3/card-before"
step p3 nonzero "$SYNC" sync "${CARD[@]}" --active kernelb.img --run-dir "$S/run-p3" --write   # card-before/ exists
check "p3 names card-before" grep -q 'card-before exists' "$O/p3.txt"

# ---- a card without cmdline.txt, back to phase A ---------------------------------------------------------------
rm -f "$B/cmdline.txt"; sync; testclean
step a1 0 "$SYNC" sync "${CARD[@]}" --active kernela.img --run-dir "$S/run-a1" --write
cp "$S/run-a1/card.manifest" "$O/a1-card.manifest"
check "a1 cmdline.txt was new" grep -q 'new      C:\\cmdline.txt' "$O/a1.txt"
check "a1 card cmdline.txt = cmdline-a.txt" cmp -s "$B/cmdline.txt" "$CLA"
check "a1 manifest before = absent" grep -q '^boot_cmdline_before absent$' "$O/a1-card.manifest"
check "a1 no cmdline backup" test ! -e "$S/run-a1/card-before/C/cmdline.txt"
check "a1 backup kernel8.img = kernelb.img" cmp -s "$S/run-a1/card-before/C/kernel8.img" "$K/kernelb.img"
step a2 0 "$SYNC" verify "${CARD[@]}" --active kernela.img
[ "$(dotunder)" = "$B/._FOREIGN" ] && echo "a1 no AppleDouble left, ._FOREIGN kept ok" ||
  { echo "a1 APPLEDOUBLE: $(dotunder | tr '\n' ' ')"; FAILS=$((FAILS + 1)); }

# ---- fsck --------------------------------------------------------------------------------------------------------
diskutil unmount "$B" >/dev/null; diskutil unmount "$D" >/dev/null
{ fsck_msdos -n "/dev/r${DEV#/dev/}s1"; echo "exit $?"; fsck_msdos -n "/dev/r${DEV#/dev/}s2"; echo "exit $?"; } > "$O/f1.txt" 2>&1
grep -c '^exit 0' "$O/f1.txt" | grep -q '^2$' && echo "f1 fsck ok" || { echo "f1 FSCK"; FAILS=$((FAILS + 1)); }
detach
echo "image sha256 after the runs: $(shasum -a 256 "$IMG" | awk '{print $1}')" | tee "$O/image.txt"
echo "FAILS=$FAILS"
[ "$FAILS" = 0 ]

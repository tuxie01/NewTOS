#!/usr/bin/env bash
#
# mkcard.sh -- a fresh MBR + FAT16 QEMU card for one measurement run, with the
# same `<image>.manifest` as tools/make-sd-image.sh (TASK/TOOL02).
#
# Usage:
#   tools/mkcard.sh <image> <NEWDESK.INF> [dest=host-file[@YYYYMMDDhhmm] ...]
#
#     image          the image file to (re)create; `<image>.manifest` is
#                    written next to it
#     NEWDESK.INF    host file copied to C:\NEWDESK.INF (after the pairs, so it
#                    wins over a pair with the same destination)
#     dest=file      host file copied to C:\<dest>; dest is relative to C:\
#                    with `/` as separator, missing folders are created. An
#                    `@YYYYMMDDhhmm` suffix gives this file its own FAT time
#                    stamp instead of MKCARD_STAMP.
#
#   Environment (all optional; a run states the ones it sets in its call):
#     MKCARD_LABEL     FAT volume label, at most 11 characters (default NTCARD)
#     MKCARD_SIZE_MIB  card size in MiB (default 64)
#     MKCARD_DIRS      empty folders to create on C:\, space-separated
#                      (default: none)
#     MKCARD_STAMP     FAT time stamp of every copied file, `touch -t` form
#                      (default 202610171200, the stamp of the per-run copies)
#     MKCARD_TESTDISK  a test image from tools/mktestdisk.sh: the card gets
#                      it byte for byte as partition 1 = C: (the test
#                      partition) and a separate program partition 2 = D:
#                      behind it; the dest=file pairs and MKCARD_DIRS go to
#                      D:, nothing is written to C:. The test image must
#                      already carry C:\NEWDESK.INF equal to <NEWDESK.INF>
#                      (tools/mktestdisk.sh <img> NEWDESK.INF=<file>). Size
#                      default 128 MiB here (QEMU wants a power of two).
#
# This replaces the per-run copies of measurements/*/method/mkcard.sh (same
# positional interface; their label, empty folders and stamp are the MKCARD_*
# variables now). What goes onto the card stays in the run's own call, which
# the run keeps in its method/ directory; the tool holds nothing run-specific.
#
# Every input is mandatory: a missing NEWDESK.INF or host file ends the script
# with a non-zero status before the image is touched. On any later failure the
# half-written image and its manifest are deleted, so a manifest always
# belongs to the image beside it.
#
# Manifest: format v1, written by tools/sd-manifest.sh. This tool reads no
# SD_* switches (those belong to tools/make-sd-image.sh), so its manifest says
# `switch n/a`; it has no optional parts, so `optional_missing none`.
#
# The image is attached with hdiutil as a user-owned raw device and the
# partition is mounted on a private mount point next to the image, never
# under /Volumes/<label>. QEMU images only -- never a real card.
#
# Test-disk mode (MKCARD_TESTDISK; TASK/TOOL07): the card for disk cases on
# NewTOS. The test partition is the oracle's test image (profile disk,
# docs/analysis/ORACLE.md section 2) copied unchanged -- MBR entry 1, its
# hidden sectors and every sector of the partition -- so EmuTOS and NewTOS
# read the same bytes on C:; the program (dumper, table, BIN, LOG) lives on
# D:, never on the test partition. MBR entry 2 is added for D: (type 0x06,
# from the first sector after the test image to the end of the card); a
# test image with more than one partition is refused. D: is FAT16 written by
# mtools' mformat (hermetic as in tools/mktestdisk.sh: no mount, volume serial
# 4E54-4344, every time stamp MKCARD_STAMP unless a file names its own). The
# manifest lists the C: files read back from the test image and the D:
# files; its header names the test image and its sha256. A run compares only
# the test partition before and after (dd it out, fsck_msdos -n).
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
usage() {
  sed -n '/^# Usage:/,/^# This replaces/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//' >&2
  exit 2
}
[ "$#" -ge 2 ] || usage
IMG="$1"; INF="$2"; shift 2
LABEL="${MKCARD_LABEL:-NTCARD}"
TESTDISK="${MKCARD_TESTDISK:-}"
if [ -n "$TESTDISK" ]; then SIZE_MB="${MKCARD_SIZE_MIB:-128}"; else SIZE_MB="${MKCARD_SIZE_MIB:-64}"; fi
DIRS="${MKCARD_DIRS:-}"
STAMP="${MKCARD_STAMP:-202610171200}"
STAMP_RE='[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'

# Check every input before the image is touched; report all problems at once.
bad=0
[ -f "$INF" ] || { echo "mkcard: NEWDESK.INF source $INF not found" >&2; bad=$((bad + 1)); }
for spec in "$@"; do
  case "$spec" in
    *=*) ;;
    *) echo "mkcard: '$spec' is not dest=host-file" >&2; bad=$((bad + 1)); continue ;;
  esac
  d="${spec%%=*}"; s="${spec#*=}"
  case "$s" in *@$STAMP_RE) s="${s%@*}" ;; esac
  case "$d" in
    ''|/*|*..*) echo "mkcard: destination '$d' must be relative to C:\\ without '..'" >&2; bad=$((bad + 1)) ;;
  esac
  [ -f "$s" ] || { echo "mkcard: host file $s (for C:\\$d) not found" >&2; bad=$((bad + 1)); }
done
case "$SIZE_MB" in ''|*[!0-9]*) echo "mkcard: MKCARD_SIZE_MIB '$SIZE_MB' is not a number" >&2; bad=$((bad + 1)) ;; esac
if [ -n "$TESTDISK" ]; then
  if [ ! -f "$TESTDISK" ]; then
    echo "mkcard: MKCARD_TESTDISK $TESTDISK not found" >&2; bad=$((bad + 1))
  else
    for t in mformat mcopy mmd; do
      command -v "$t" >/dev/null || { echo "mkcard: $t (mtools) not found, MKCARD_TESTDISK needs it" >&2; bad=$((bad + 1)); }
    done
  fi
fi
if [ "$bad" -ne 0 ]; then
  echo "mkcard: $bad problem(s); no image written" >&2
  exit 1
fi

# ---- test-disk mode (MKCARD_TESTDISK): see the header ----------------------
if [ -n "$TESTDISK" ]; then
  export MTOOLSRC=/dev/null MTOOLS_NO_VFAT=1 MTOOLS_SKIP_CHECK=1 TZ=UTC
  SOURCE_DATE_EPOCH=$(date -j -u -f %Y%m%d%H%M%S "${STAMP}00" +%s)
  export SOURCE_DATE_EPOCH
  MANIFEST="${IMG}.manifest"
  STAGE="$(cd "$(dirname "$IMG")" && pwd)/stage.$$"
  trap 'rc=$?; rm -rf "$STAGE"; [ "$rc" -eq 0 ] || rm -f "$IMG" "$MANIFEST" "${IMG}.manifest.part"' EXIT
  le32() { od -An -t u4 -j "$2" -N 4 "$1" | tr -d ' '; }
  byte() { od -An -t u1 -j "$2" -N 1 "$1" | tr -d ' '; }
  # The test image: an MBR with entry 1 only (the layout of tools/mktestdisk.sh).
  tbytes=$(stat -f %z "$TESTDISK")
  [ $((tbytes % 512)) -eq 0 ] || { echo "mkcard: test image is not whole sectors" >&2; exit 1; }
  [ "$(byte "$TESTDISK" 510)" = 85 ] && [ "$(byte "$TESTDISK" 511)" = 170 ] ||
    { echo "mkcard: test image has no MBR signature" >&2; exit 1; }
  [ "$(byte "$TESTDISK" 450)" != 0 ] || { echo "mkcard: test image has no partition 1" >&2; exit 1; }
  for e in 1 2 3; do
    [ "$(byte "$TESTDISK" $((446 + 16 * e + 4)))" = 0 ] ||
      { echo "mkcard: test image has a partition $((e + 1)); only one is supported" >&2; exit 1; }
  done
  TSTART=$(le32 "$TESTDISK" 454); TCOUNT=$(le32 "$TESTDISK" 458)
  [ $(( (TSTART + TCOUNT) * 512 )) -le "$tbytes" ] || { echo "mkcard: test partition runs past the test image" >&2; exit 1; }
  # C:\NEWDESK.INF must already be on the test image and equal to <NEWDESK.INF>.
  mkdir -p "$STAGE/C" "$STAGE/D"
  mcopy -s -m -n -i "${TESTDISK}@@$((TSTART * 512))" '::*' "$STAGE/C/" 2>/dev/null || true
  cmp -s "$STAGE/C/NEWDESK.INF" "$INF" ||
    { echo "mkcard: the test image carries no C:\\NEWDESK.INF equal to $INF (tools/mktestdisk.sh <img> NEWDESK.INF=$INF)" >&2; exit 1; }
  CARD=$((SIZE_MB * 2048)); PSTART=$((tbytes / 512)); PCOUNT=$((CARD - PSTART))
  [ "$PCOUNT" -ge 32768 ] || { echo "mkcard: MKCARD_SIZE_MIB $SIZE_MB leaves less than 16 MiB after the test image" >&2; exit 1; }
  echo "mkcard: ${IMG} (${SIZE_MB} MiB, test image ${TESTDISK} as C:, program partition D: LBA ${PSTART}+${PCOUNT}, label ${LABEL})"
  rm -f "$IMG" "$MANIFEST" "${IMG}.manifest.part"
  dd if=/dev/zero of="$IMG" bs=1m count="$SIZE_MB" status=none
  dd if="$TESTDISK" of="$IMG" bs=512 conv=notrunc status=none
  # MBR entry 2: CHS for 16 heads x 63 sectors (as the test image's entry 1).
  chs() { local l=$1 c h s; c=$((l / 1008)); h=$(((l / 63) % 16)); s=$((l % 63 + 1))
          [ "$c" -le 1023 ] || { c=1023; h=254; s=63; }
          printf '\\%03o\\%03o\\%03o' "$h" $((s | ((c >> 8) & 3) << 6)) $((c & 255)); }
  le() { printf '\\%03o\\%03o\\%03o\\%03o' $(($1 & 255)) $((($1 >> 8) & 255)) $((($1 >> 16) & 255)) $((($1 >> 24) & 255)); }
  printf "\\000$(chs "$PSTART")\\006$(chs $((CARD - 1)))$(le "$PSTART")$(le "$PCOUNT")" |
    dd of="$IMG" bs=1 seek=462 conv=notrunc status=none
  # FAT16 on D:, the cluster size the smallest that keeps it below 65525 clusters.
  spc=4; while [ $((PCOUNT / spc)) -ge 65000 ]; do spc=$((spc * 2)); done
  DPART="${IMG}@@$((PSTART * 512))"
  mformat -i "$DPART" -T "$PCOUNT" -h 16 -s 63 -H "$PSTART" -c "$spc" -N 4E544344 -v "$LABEL" ::
  for d in $DIRS; do
    mkdir -p "$STAGE/D/$d"; p=
    IFS=/; for c in $d; do p="${p:+$p/}$c"; mmd -D s -i "$DPART" "::$p" 2>/dev/null || true; done; unset IFS
    echo "  + D:\\$(printf '%s' "$d" | tr '/' '\\')\\ (empty folder)"
  done
  for spec in "$@"; do
    d="${spec%%=*}"; s="${spec#*=}"; ts="$STAMP"
    case "$s" in *@$STAMP_RE) ts="${s##*@}"; s="${s%@*}" ;; esac
    dir="$(dirname "$d")"
    if [ "$dir" != . ]; then
      mkdir -p "$STAGE/D/$dir"; p=
      IFS=/; for c in $dir; do p="${p:+$p/}$c"; mmd -D s -i "$DPART" "::$p" 2>/dev/null || true; done; unset IFS
    fi
    cp -X "$s" "$STAGE/D/$d"; touch -t "$ts" "$STAGE/D/$d"
    mcopy -m -i "$DPART" "$STAGE/D/$d" "::$d"
    echo "  + D:\\$(printf '%s' "$d" | tr '/' '\\') <- $s"
  done
  # The test partition must be the test image's, byte for byte.
  cmp -s <(dd if="$IMG" bs=512 skip="$TSTART" count="$TCOUNT" status=none) \
         <(dd if="$TESTDISK" bs=512 skip="$TSTART" count="$TCOUNT" status=none) ||
    { echo "mkcard: the test partition differs from the test image" >&2; exit 1; }
  "$SCRIPT_DIR/sd-manifest.sh" files "$STAGE/C" C "$IMG"
  "$SCRIPT_DIR/sd-manifest.sh" files "$STAGE/D" D "$IMG"
  "$SCRIPT_DIR/sd-manifest.sh" write "$IMG" "$SIZE_MB" \
    "tools/mkcard.sh, MKCARD_TESTDISK $(shasum -a 256 "$TESTDISK" | awk '{print $1}') $TESTDISK" --switches n/a
  echo "done: ${IMG}"
  echo "manifest: ${MANIFEST} ($(sed -n '/^file /p' "$MANIFEST" | wc -l | tr -d ' ') files; copy it into the measurement run, RUNINFO.TXT cites it)"
  exit 0
fi

MANIFEST="${IMG}.manifest"
MANIFEST_FILES="${IMG}.manifest.part"
MNT="$(cd "$(dirname "$IMG")" && pwd)/mnt.$$"
DEV=
cleanup() {
  [ -d "$MNT" ] && { diskutil unmount "$MNT" >/dev/null 2>&1 || true; rmdir "$MNT" 2>/dev/null || true; }
  [ -z "$DEV" ] || hdiutil detach "$DEV" >/dev/null 2>&1 || true
  DEV=
}
trap 'rc=$?; cleanup; [ "$rc" -eq 0 ] || rm -f "$IMG" "$MANIFEST" "$MANIFEST_FILES"' EXIT

echo "mkcard: ${IMG} (${SIZE_MB} MiB, MBR + FAT16, label ${LABEL})"
rm -f "$IMG" "$MANIFEST" "$MANIFEST_FILES"
dd if=/dev/zero of="$IMG" bs=1m count="$SIZE_MB" status=none
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" | head -1 | awk '{print $1}')
diskutil partitionDisk "$DEV" 1 MBR "MS-DOS FAT16" "$LABEL" 100% >/dev/null
diskutil unmount "${DEV}s1" >/dev/null
mkdir -p "$MNT"; diskutil mount -mountPoint "$MNT" "${DEV}s1" >/dev/null
export COPYFILE_DISABLE=1; mdutil -i off "$MNT" >/dev/null 2>&1 || true

for d in $DIRS; do mkdir -p "$MNT/$d"; echo "  + C:\\$(printf '%s' "$d" | tr '/' '\\')\\ (empty folder)"; done
for spec in "$@"; do
  d="${spec%%=*}"; s="${spec#*=}"; ts="$STAMP"
  case "$s" in *@$STAMP_RE) ts="${s##*@}"; s="${s%@*}" ;; esac
  mkdir -p "$MNT/$(dirname "$d")"; cp -X "$s" "$MNT/$d"; touch -t "$ts" "$MNT/$d"
  echo "  + C:\\$(printf '%s' "$d" | tr '/' '\\') <- $s"
done
cp -X "$INF" "$MNT/NEWDESK.INF"; touch -t "$STAMP" "$MNT/NEWDESK.INF"
echo "  + C:\\NEWDESK.INF <- $INF"

# Strip macOS metadata (AppleDouble files, volume directories) before hashing.
find "$MNT" -name '._*' -delete 2>/dev/null || true; dot_clean -m "$MNT" 2>/dev/null || true
rm -rf "$MNT/.fseventsd" "$MNT/.Spotlight-V100" "$MNT/.Trashes" "$MNT/.TemporaryItems" 2>/dev/null || true
sync

"$SCRIPT_DIR/sd-manifest.sh" files "$MNT" C "$IMG"
diskutil unmount "$MNT" >/dev/null
rmdir "$MNT"
hdiutil detach "$DEV" >/dev/null
DEV=

"$SCRIPT_DIR/sd-manifest.sh" write "$IMG" "$SIZE_MB" tools/mkcard.sh --switches n/a
echo "done: ${IMG}"
echo "manifest: ${MANIFEST} ($(sed -n '/^file /p' "$MANIFEST" | wc -l | tr -d ' ') files; copy it into the measurement run, RUNINFO.TXT cites it)"

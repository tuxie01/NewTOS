#!/usr/bin/env bash
#
# mktestdisk.sh -- the FAT16 test disk of the oracle's disk profile
# (tools/hatari-oracle.sh disk, docs/analysis/ORACLE.md section 2; TASK/TOOL06),
# byte-reproducible: the same arguments give the same image, bit for bit.
#
# Usage:
#   tools/mktestdisk.sh <image> [dest=host-file[@YYYYMMDDhhmm] ...]
#
#     image          the image file to (re)create; `<image>.manifest` is
#                    written next to it (tools/sd-manifest.sh, format v1)
#     dest=file      host file copied to C:\<dest>; dest is relative to C:\
#                    with `/` as separator, 8.3 names only, missing folders
#                    are created. An `@YYYYMMDDhhmm` suffix gives this file
#                    its own FAT time stamp instead of the default.
#
# The image, fixed (nothing here is a switch, so one command names it all):
#
#   size        64 MiB = 131072 sectors of 512 bytes
#   MBR         one primary partition, entry 1, type 0x06 (FAT16, >= 32 MiB),
#               not active, LBA 63 .. 131039 (130977 sectors), CHS for
#               16 heads x 63 sectors (130 cylinders); entries 2-4 empty;
#               no boot code; signature 0x55 0xAA
#   partition   FAT16 written by mtools' mformat: 512 bytes per sector,
#               4 sectors per cluster, 1 reserved sector, 2 FATs of 128
#               sectors, 512 root entries (32 sectors), 32672 clusters,
#               media 0xF8, hidden sectors 63, volume serial 4E54-5444,
#               label NTTESTDISK
#   time stamp  every FAT time stamp (label, folders, files) is
#               2026-10-17 12:00 (UTC), the stamp of tools/mkcard.sh,
#               unless a file names its own
#
# This is the geometry tools/mkcard.sh gets from macOS' diskutil at its
# 64 MiB default (start 63, 130977 sectors, the same BPB: 512, 4, 2048, 32,
# 128, 129, 289, 32672 -- measured by the BIOS storage audit,
# measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT). diskutil
# picks a random volume serial and its own boot code, so its images differ
# between two builds; this tool uses mtools, which takes every value from
# the command line and SOURCE_DATE_EPOCH, and needs no mount.
#
# Requires mtools (mformat, mcopy, mmd); its configuration files are not
# read (MTOOLSRC=/dev/null), long file names are off (MTOOLS_NO_VFAT=1).
# The manifest's `file` lines come from a staging copy of exactly the files
# written; `image_sha256` is the same for the same inputs, only `created`
# changes. QEMU and Hatari images only -- never a real card.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
usage() {
  sed -n '/^# Usage:/,/^# The image, fixed/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//' >&2
  exit 2
}
[ "$#" -ge 1 ] || usage
IMG="$1"; shift

SECTORS=131072          # 64 MiB
PSTART=63               # first sector of the partition
PSIZE=130977            # 130 cylinders x 16 heads x 63 sectors - 63
STAMP=202610171200      # default FAT time stamp, `touch -t` form, UTC
SERIAL=4E545444
LABEL=NTTESTDISK
STAMP_RE='[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'

for t in mformat mcopy mmd; do
  command -v "$t" >/dev/null || { echo "mktestdisk: $t (mtools) not found" >&2; exit 1; }
done

# Check every input before the image is touched; report all problems at once.
bad=0
for spec in "$@"; do
  case "$spec" in
    *=*) ;;
    *) echo "mktestdisk: '$spec' is not dest=host-file" >&2; bad=$((bad + 1)); continue ;;
  esac
  d="${spec%%=*}"; s="${spec#*=}"
  case "$s" in *@$STAMP_RE) s="${s%@*}" ;; esac
  case "$d" in
    ''|/*|*..*) echo "mktestdisk: destination '$d' must be relative to C:\\ without '..'" >&2; bad=$((bad + 1)) ;;
  esac
  [ -f "$s" ] || { echo "mktestdisk: host file $s (for C:\\$d) not found" >&2; bad=$((bad + 1)); }
done
if [ "$bad" -ne 0 ]; then
  echo "mktestdisk: $bad problem(s); no image written" >&2
  exit 1
fi

# Hermetic mtools: no configuration file, 8.3 names, every time stamp UTC
# and, where mtools takes "now", SOURCE_DATE_EPOCH = the default stamp.
export MTOOLSRC=/dev/null MTOOLS_NO_VFAT=1 MTOOLS_SKIP_CHECK=1 TZ=UTC
# Seconds given explicitly: BSD date -f takes every field it does not parse
# from the current clock.
SOURCE_DATE_EPOCH=$(date -j -u -f %Y%m%d%H%M%S "${STAMP}00" +%s)
export SOURCE_DATE_EPOCH

MANIFEST="${IMG}.manifest"
STAGE="$(cd "$(dirname "$IMG")" && pwd)/stage.$$"
trap 'rc=$?; rm -rf "$STAGE"; [ "$rc" -eq 0 ] || rm -f "$IMG" "$MANIFEST" "${IMG}.manifest.part"' EXIT

echo "mktestdisk: ${IMG} (64 MiB, MBR + FAT16 at LBA ${PSTART}, label ${LABEL})"
rm -f "$IMG" "$MANIFEST" "${IMG}.manifest.part"
dd if=/dev/zero of="$IMG" bs=512 count="$SECTORS" status=none

# MBR partition entry 1 at offset 446: status, CHS first (head 1, sector 1,
# cylinder 0), type, CHS last (head 15, sector 63, cylinder 129 = 0x81),
# LBA first (63) and sector count (130977 = 0x0001FFA1), little endian.
printf '\000\001\001\000\006\017\077\201\077\000\000\000\241\377\001\000' |
  dd of="$IMG" bs=1 seek=446 conv=notrunc status=none
printf '\125\252' | dd of="$IMG" bs=1 seek=510 conv=notrunc status=none

PART="${IMG}@@$((PSTART * 512))"
mformat -i "$PART" -T "$PSIZE" -h 16 -s 63 -H "$PSTART" -c 4 -r 32 -L 128 \
  -N "$SERIAL" -v "$LABEL" ::

mkdir -p "$STAGE"
for spec in "$@"; do
  d="${spec%%=*}"; s="${spec#*=}"; ts="$STAMP"
  case "$s" in *@$STAMP_RE) ts="${s##*@}"; s="${s%@*}" ;; esac
  dir="$(dirname "$d")"
  if [ "$dir" != . ]; then
    mkdir -p "$STAGE/$dir"
    # mmd each missing level; an existing one is not an error here.
    p=
    IFS=/; for c in $dir; do p="${p:+$p/}$c"; mmd -D s -i "$PART" "::$p" 2>/dev/null || true; done; unset IFS
  fi
  cp -X "$s" "$STAGE/$d"; touch -t "$ts" "$STAGE/$d"
  mcopy -m -i "$PART" "$STAGE/$d" "::$d"
  echo "  + C:\\$(printf '%s' "$d" | tr '/' '\\') <- $s"
done

"$SCRIPT_DIR/sd-manifest.sh" files "$STAGE" C "$IMG"
"$SCRIPT_DIR/sd-manifest.sh" write "$IMG" 64 tools/mktestdisk.sh --switches n/a
echo "done: ${IMG} sha256 $(shasum -a 256 "$IMG" | awk '{print $1}')"

#!/bin/sh
# mkimage.sh <scratch> -- the test image of the three gemdos-mc tables
# (TASK/TOOL20), byte-reproducible for the same probe builds:
#   tools/mktestdisk.sh <scratch>/test.img
#     NEWDESK.INF   = method/newdesk-d.inf ("#R 02" / "#Z 01 D:\DUMPBIOS.NAP@",
#                     CRLF; NewTOS' autorun, inert on EmuTOS; the bytes of
#                     measurements/measurement-2026-10-28-tool07)
#     PXPROBE.PRG   = make -C tosdump pxprobe-m68k     (the oracle's child)
#     PXPROBE.NAP   = make -C tosdump pxprobe-newtos   (NewTOS' child)
#     DELME.TXT     = method/delme.txt   (gemdos-mc-paths.tsv deletes it)
#     SUB/INNER.TXT = method/inner.txt   (a folder to list and to enter)
# Both sides boot clones of the same image, so both test partitions start
# from the same bytes. Writes <scratch>/test.img (+ .manifest) and
# <scratch>/px/PXPROBE.{PRG,NAP}. Run from the repository root.
set -eu
S=$1
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$S/px"
make -s -C tosdump BUILD="$S/px" pxprobe-m68k pxprobe-newtos > "$S/px/make.txt" 2>&1 \
    || { cat "$S/px/make.txt" >&2; exit 1; }
tools/mktestdisk.sh "$S/test.img" \
    NEWDESK.INF="$HERE/newdesk-d.inf" \
    PXPROBE.PRG="$S/px/PXPROBE.PRG" \
    PXPROBE.NAP="$S/px/PXPROBE.NAP" \
    DELME.TXT="$HERE/delme.txt" \
    SUB/INNER.TXT="$HERE/inner.txt" > "$S/mktestdisk.stdout"
shasum -a 256 "$S/px/PXPROBE.PRG" "$S/px/PXPROBE.NAP" "$S/test.img"

#!/usr/bin/env bash
# mkcard-call.sh <scratch> -- the run's one card: MBR + FAT16, 64 MiB (defaults of
# tools/mkcard.sh), C:\BSTPROBE.NAP + NEWDESK.INF "#Z 01 C:\BSTPROBE.NAP@".
# The same file, APFS-cloned, is the EmuTOS IDE master disk and the NewTOS SD card.
set -euo pipefail
S=$1; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; cd "$HERE/../../.."
tools/mkcard.sh "$S/card.img" "$HERE/z-bstprobe.inf" BSTPROBE.NAP="$S/BSTPROBE.NAP"

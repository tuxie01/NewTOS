#!/bin/bash
# cards-z.sh <scratch> <method dir> -- the QEMU card of the #Z control on the unpatched kernel: #Z 00 C:\D70M0.NCP@ in NEWDESK.INF (tools/mkcard.sh)
S=$1; M=$2
MKCARD_LABEL=DESK70Z /Volumes/Daten/Development/PiRusTOS/tools/mkcard.sh "$S/cz/cardz.img" "$M/NEWDESK-Z.INF" D70M0.NCP="$S/ncp/D70M0.NCP"

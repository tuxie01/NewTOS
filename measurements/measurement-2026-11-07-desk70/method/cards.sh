#!/bin/bash
# cards.sh <scratch> <method dir> -- the one QEMU base card of the NewTOS runs (tools/mkcard.sh, manifest beside it)
S=$1; M=$2
MKCARD_LABEL=DESK70 /Volumes/Daten/Development/PiRusTOS/tools/mkcard.sh "$S/card.img" "$M/NEWDESK.INF" \
  D70M0.NCP="$S/ncp/D70M0.NCP" D70M0.NTP="$S/ncp/D70M0.NTP" D70M1.NCP="$S/ncp/D70M1.NCP" D70M2.NCP="$S/ncp/D70M2.NCP"

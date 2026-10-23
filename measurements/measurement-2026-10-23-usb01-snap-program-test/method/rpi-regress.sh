#!/usr/bin/env bash
# raspi4b regression: APITEST (expect TOTAL 109 0 5) and the plain desktop (expect "desktop up"), head vs base release kernels
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usbsnap-t; R=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-10-23-usb01-snap-program-test; M=$R/method; A=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usb01-snt/api
mkdir -p $S/cards
$M/mkcard.sh $S/cards/api-base.img $M/z-apitest.inf APITEST.NCP=$A/APITEST.NCP PTERMCH.NCP=$A/PTERMCH.NCP DTACHILD.NCP=$A/DTACHILD.NCP >/dev/null
$M/mkcard.sh $S/cards/nd-base.img $M/nd.inf >/dev/null
for w in head base; do
  cp -c $S/cards/api-base.img $S/cards/api.img
  $M/run-rpi.sh $S/bin/kernel8-$w-rpi4.img $S/cards/api.img $R/serial/rpi4-api-$w.log "TOTAL " 240
  rm -f $S/cards/api.img
  cp -c $S/cards/nd-base.img $S/cards/nd.img
  SHOT=$S/shots/rpi4-desktop-$w.png $M/run-rpi.sh $S/bin/kernel8-$w-rpi4.img $S/cards/nd.img $R/serial/rpi4-nd-$w.log "desktop up" 90
  rm -f $S/cards/nd.img
done
rm -f $S/cards/*-base.img

#!/usr/bin/env bash
# The four runs, in this order, one QEMU at a time.
F=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08-fix
$F/method/run-rpi4.sh r1-zview      'C:\ZVIEW\ZVIEW.NAP' 'serial=on memprot=verify'          120  'NEVER_MATCHES_X'
$F/method/run-rpi4.sh r2-ldgloop    'C:\LDGLOOP.NCP'     'serial=on memprot=verify'          1500 "LDGLOOP.NCP' exited"
$F/method/run-rpi4.sh r3-zview-asid 'C:\ZVIEW\ZVIEW.NAP' 'serial=on memprot=verify asid=on'  120  'NEVER_MATCHES_X'
$F/method/run-virt.sh r4-virt-boot 'serial=on memprot=verify' 60

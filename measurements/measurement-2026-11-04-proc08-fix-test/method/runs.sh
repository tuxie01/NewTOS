#!/usr/bin/env bash
# The runs in order, one QEMU at a time. K = kernel directory (head-* = 01eb7709, base-* = 24437ff0, release builds)
K=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/w/k
R=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-04-proc08-fix-test/method
$R/run-rpi4.sh h-boot       $K/head-rpi4.img    - 'serial=on memprot=verify' 60 'NEVER_X'
$R/run-rpi4.sh b-boot       $K/base-rpi4.img    - 'serial=on memprot=verify' 60 'NEVER_X'
$R/run-rpi4.sh h-zview      $K/head-rpi4.img    'C:\ZVIEW\ZVIEW.NAP' 'serial=on memprot=verify' 120 'NEVER_X'
$R/run-rpi4.sh h-ldgloop    $K/head-rpi4.img    'C:\LDGLOOP.NCP' 'serial=on memprot=verify' 1500 "LDGLOOP.NCP' exited"
$R/run-rpi4.sh h-zview-asid $K/head-rpi4.img    'C:\ZVIEW\ZVIEW.NAP' 'serial=on memprot=verify asid=on' 120 'NEVER_X'
$R/run-rpi4.sh h-cw-boot    $K/head-rpi4-cw.img - 'serial=on memprot=verify' 60 'NEVER_X'
$R/run-rpi4.sh b-cw-boot    $K/base-rpi4-cw.img - 'serial=on memprot=verify' 60 'NEVER_X'
$R/run-rpi4.sh h-argvtest   $K/head-rpi4.img    'C:\ARGVTEST.NAP' 'serial=on memprot=verify' 60 'NEVER_X'
$R/run-virt.sh h-virt-boot  $K/head-virt.img 'serial=on memprot=verify' 60
$R/run-virt.sh b-virt-boot  $K/base-virt.img 'serial=on memprot=verify' 60

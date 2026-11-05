#!/bin/sh
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v20t
M=measurements/measurement-2026-11-05-v20-test
export DBPROBE_SCRATCH=$S/db
P=tosdump/probes
set -x
$P/run-dbprobe.sh kernel bec04bab $S/kdb-head.img
$P/run-dbprobe.sh kernel 20d15c8c $S/kdb-base.img
$P/run-dbprobe.sh newtos $M/runs/db-head $S/kdb-head.img
$P/run-dbprobe.sh newtos $M/runs/db-base $S/kdb-base.img
$P/run-dbprobe.sh emutos $M/runs/db-emutos
$P/run-dbprobe.sh emutos $M/runs/db-emutos-2
echo finished

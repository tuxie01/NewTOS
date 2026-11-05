#!/bin/sh
# V19 acceptance: tosdump runs. usage: tosdump-runs.sh <head|base> <dumper> <table> <refname> <outname>
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v19t
M=measurements/measurement-2026-11-05-v19-test
T=$1; D=$2; TAB=$3; REF=$4; OUTN=$5
cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$OUTN --reference tosdump/reference/$REF.BIN --scratch $S/xt-$T
# AES table without a reference (r4: no pinned oracle reference file exists):
#   cargo xtask tosdump aes tosdump/cases/aes-r4-subfunctions.tsv --tree $S/<head|base> --out $M/runs/<name> --scratch $S/xt-<head|base>

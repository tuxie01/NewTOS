#!/bin/bash
R=$1
pair() { n=0; d=0; l=""; for f in $R/$1/shots/*.png; do g=$R/$2/shots/$(basename $f); n=$((n+1)); cmp -s $f $g || { d=$((d+1)); l="$l $(basename $f)"; }; done; echo "$1 vs $2: $n shots, $d differ:$l"; }
pair newtos-slprobe-base newtos-slprobe-head
pair newtos-slprobe-base newtos-slprobe2-base
pair newtos-slprobe-head newtos-slprobe2-head
pair newtos-slprobe2-base newtos-slprobe2-head
pair newtos-slprobe-base newtos-slprobe2-head
pair newtos-slprobe-head newtos-slprobe2-base

#!/bin/bash
export GIT_OPTIONAL_LOCKS=0
R=/Volumes/Daten/Development/PiRusTOS
for t in references/emutos references/emutos-oracle/1a7484ab references/emutos-oracle/v1.4; do
 p=$R/$t; echo "== $t"
 [ -e "$p" ] || { echo missing; continue; }
 echo HEAD $(git -C $p rev-parse HEAD 2>&1)
 git -C $p for-each-ref 2>&1 | shasum -a 256
 g=$(git -C $p rev-parse --git-dir 2>/dev/null); case $g in /*) ;; *) g=$p/$g;; esac
 echo gitdir $g
 stat -f 'index %m' $g/index 2>&1; stat -f 'HEADf %m' $g/HEAD 2>&1; stat -f 'gitdir %m' $g 2>&1
 echo status $(git -C $p status --short --ignored 2>&1 | shasum -a 256)
 echo stash $(git -C $p stash list 2>&1 | wc -l)
done
echo "== main"; echo HEAD $(git -C $R rev-parse HEAD)

#!/bin/bash
# post.sh -- after the table runs: fill-extent table, APITEST x4, fsck after a 32-plane boot
M=$(cd $(dirname $0) && pwd)
bash $M/run-fill.sh > $M/../checks/fill-run.txt 2>&1
bash $M/apitest-runs.sh > $M/../checks/apitest-run.txt 2>&1

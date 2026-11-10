#!/bin/sh
# The one call of this run (TOOL01, compiler drift check). build-emutos.sh and
# oracle.env were taken from commit 9ff4e188c (git show HEAD:...) into a scratch
# copy whose references/ is a symlink to the repo's references/, because the
# working tree's oracle.env carried an uncommitted TOOL39 edit at the time.
# <scratch> is the session scratchpad; the tree is a fresh clone.
cd <scratch>/rom-rebuild
sh tools/oracle/build-emutos.sh 1a7484aba1bfba23e2c0899e243b6d8177fe41df 2026-04-14 <scratch>/rom-rebuild/tree

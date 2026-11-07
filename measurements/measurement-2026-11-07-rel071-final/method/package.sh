#!/bin/sh
# the only call of this run (lead, 2026-11-07); <scratch> = the session scratchpad
KERNEL_FEATURES=crash-walk tools/release/package-release.sh 0.7.1 <scratch>/rel071-b <scratch>/prev07/NewTOS-0.7.zip

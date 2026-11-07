#!/bin/sh
# the only call of this run (lead, 2026-11-07); <scratch> = the session scratchpad
NAMES_ADDED="KARTE/APPS/HYPVIEW/CHANGES.TXT KARTE/APPS/COREMARK/CHANGES.TXT" KERNEL_FEATURES=crash-walk tools/release/package-release.sh 0.7.1 <scratch>/rel071-c <scratch>/prev07/NewTOS-0.7.zip

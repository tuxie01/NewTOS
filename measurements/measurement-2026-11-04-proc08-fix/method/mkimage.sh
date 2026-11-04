#!/usr/bin/env bash
# The base image of this run (one image, per-run APFS clone in run-rpi4.sh):
# tools/make-sd-image.sh as in measurements/measurement-2026-11-02-proc08,
# because it carries zView with its ten codecs and LDG (C:\LDG\EX_LIB.NLD);
# LDGLOOP.NCP (the probe of that run, method/ldgloop.c + method/build.sh
# there, sha256 a7ed7507...) is copied onto each clone by run-rpi4.sh.
F=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08-fix
tools/make-sd-image.sh $F/base.img 64

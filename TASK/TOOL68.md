# TOOL68 — mkcard normal mode and make-sd-image.sh build byte-reproducible images

Status: OPEN (2026-11-12) | Priority: P3

Title: mkcard normal mode and make-sd-image.sh build byte-reproducible images

Type: tool. Owner decision 2026-11-12, follow-up of TASK/TOOL67.md. Not
started now.

## Finding

The TOOL67 developer measured that the unchanged `tools/mkcard.sh` (normal
mode) and `tools/make-sd-image.sh` give a new image sha256 on every run with
the same inputs and stamp: the FAT volume serial, unstamped directory time
stamps and `.fseventsd` residue differ. The `MKCARD_TESTDISK` mode is
already reproducible (equal sha256), since it builds `D:` without a mount.

## Scope

Both scripts build their images without mounting them, as the test-disk
mode already does for `D:`, so that the same inputs and stamp give the same
image bytes.

## Acceptance

- Two runs of each script with the same inputs and stamp give equal image
  sha256.
- Every file sha256 in the manifest equals that of the image built by the
  script before the change.

TOOL67 residual: not yet shown without the macOS grant and not yet run on the second machine; the first card build after a Claude Code update, and the first on the other Mac, are the proof. A mount failure there reopens the question in a new ticket.

# TOOL67 — mkcard.sh and make-sd-image.sh work with an image on the data volume

Status: IMPLEMENTED (2026-11-12) | Priority: P1 | Commit: self | Missing: tester run (both scripts with the image on the data volume, output dir untouched until the final move, file-level comparison with the unchanged scripts, tosdump --scratch on the data volume, forced failure leaves no temp dir, mount or device)

Title: mkcard.sh and make-sd-image.sh work with an image on the data volume

Type: tool. Owner decision 2026-11-12: split out of TOOL53 (point 5) and
fixed now. It blocks every test that needs an SD image (A68 and the seven
tickets queued behind it under PITEST02, the XBIOS21 oracle rerun, A41).
No researcher gate: a host tool, no TOS or hardware spec.

## Finding

- `TASK/TOOL53.md:36-40` (point 5, "Card images on the data volume").
- `measurements/measurement-2026-11-11-a41-retest/RUNINFO.TXT`: "the same
  partition mounts under /private/tmp".
- `measurements/measurement-2026-11-12-a68/RUNINFO.TXT`, step 9: mkcard
  failed, "Volume on disk7s1 failed to mount".
- `TASK/XBIOS21.md:98-100`: the oracle rerun was stopped for the same
  failure.

Since TOOL46 the workers' images lie under `target-agents/` on
`/Volumes/Daten`. The scripts mount the partition on a private mount point
next to the image, and `diskutil` refuses it there ("Volume on diskNs1
failed to mount"). Workers then call `diskutil` by hand and the owner has to
confirm each call.

## Fix (chosen by the owner)

The private mount point moves to a temporary directory on the system disk
(`mktemp -d` under `$TMPDIR`, removed by the existing cleanup trap); the
image stays where the caller put it. No other change to how the card is
built. Check `tools/sync-card.sh` for the same pattern and say in the
receipt whether it is affected.

**Changed by the owner 2026-11-12 (replaces the paragraph above):** the
whole card is built on the system disk, not only the mount point. The
scripts create the image, attach, mount, fill and unmount it in a temporary
directory (`mktemp -d` under `$TMPDIR`), and move the finished image and its
manifest to the caller's output path as the last step. The cleanup trap
removes the temporary directory on success and on any failure. Nothing of
the build touches `/Volumes/Daten` before that move. Clones, worktrees and
cargo target directories stay on the data volume (TOOL46 unchanged). This
applies to both scripts, `tools/make-sd-image.sh` included.

## Acceptance

- `tools/mkcard.sh` and `tools/make-sd-image.sh` each build an image whose
  path lies under `target-agents/agent-<id>/` on `/Volumes/Daten`, with no
  manual `diskutil`/`hdiutil` call by the worker and no permission prompt.
- For the same inputs and stamp, compared with an image built by the
  unchanged script with its image under `$TMPDIR`, the same place as the
  fixed script's (restated by the owner 2026-11-12, twice):
  - `MKCARD_TESTDISK` mode: image sha256 equal;
  - normal `mkcard.sh` mode and `tools/make-sd-image.sh`: every file sha256
    in the manifest equal, and every differing image byte assigned to one of
    the classes volume serial, directory time stamp or `.fseventsd`, with
    offsets in `RUNINFO.TXT`;
  - `tools/make-sd-image.sh` only, a fourth class "random by design",
    limited to the bytes of `DATA.BIN`, `KERNEL8.IMG` and `LYNXBOOT.IMG`
    (the three `head -c … /dev/urandom` lines of the script, lines 202-204
    at 8db332393) and the two deleted directory entries with `tmp…` names;
  - any other byte outside these classes is a FAIL.

  The owner's first criterion (image sha256 and manifest equal in every
  mode) assumed reproducible images, which the unchanged scripts do not
  produce in these two modes (developer receipt, 2026-11-12).
  Byte-reproducible images there: follow-up TASK/TOOL68.md. The owner's
  second wording (comparison card under the session scratchpad, three
  classes) did not foresee the build location (the scratchpad build leaves
  `.Spotlight-V100` residue that shifts the file clusters) or the random
  files. The tester's control run, the unchanged script with its image
  under `$TMPDIR`, is what shows that the fix causes no difference
  (`measurements/measurement-2026-11-12-tool67/RUNINFO.TXT`,
  `measurements/measurement-2026-11-12-tool67/serial/byte-classification.txt`).
- `cargo xtask tosdump` with `--scratch` on the data volume runs one case
  table to the end.
- No mount point or attached device is left after a run, also after a forced
  failure in the middle.
- The receipt states the cause of `diskutil`'s refusal as far as it can be
  shown (mount point location, volume flags); if it cannot, it says so.
- The temporary directory is gone after every run, on success and after a
  forced failure (owner 2026-11-12).

If the temporary mount point does not make the mount work: verdict
`BLOCKED` with what was observed. No workaround beyond this ticket's fix.

Until this ticket is `DONE`, no worker calls `diskutil` or `hdiutil` by
hand; a test that needs a card waits (owner 2026-11-12).

## Flow

Developer on branch `tool67`, tester, land, accept. Then (lead): A68 retest
on QEMU `raspi4b` with an SD image, the seven queued PITEST02 tickets in the
order A69, A70, A81, A83, FS11, FS18, LIBC-BUF, and a note in XBIOS21 and
A41 that their blocked reruns can run.

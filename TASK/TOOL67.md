# TOOL67 — mkcard.sh and make-sd-image.sh work with an image on the data volume

Status: OPEN (2026-11-12) | Priority: P1

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
- For the same inputs and stamp, image sha256 and manifest equal those of an
  image built by the unchanged script under the session scratchpad (the fix
  changes no byte of the card).
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

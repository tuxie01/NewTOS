# TOOL67 — receipt: mkcard.sh and make-sd-image.sh build the card under $TMPDIR

Role: developer
Agent: developer (agent-a515097a38ec40016)

Branch `tool67`, base `c61aa2cba`; first commit `a9a62704d` (mount point only, the fix as
first written), rework commit after the owner's change of the fix (TASK/TOOL67.md "Fix",
"Changed by the owner 2026-11-12", main `d65cf76fb`). `<agent>` below is the agent
directory target-agents/agent-a515097a38ec40016 (gitignored, data volume); the worktree is
`<agent>/wt`.

## Change

- `tools/mkcard.sh`, both modes (normal and `MKCARD_TESTDISK`): after the input checks
  (new: the directory of `<image>` must exist, so the run still fails before any build
  work) the script makes `WORK=$(mktemp -d "${TMPDIR:-/tmp}/mkcard.XXXXXX")` and builds
  the image as `BIMG=$WORK/<basename of image>`: dd, hdiutil attach, partitionDisk, mount
  on `$WORK/mnt`, fill, `sd-manifest.sh files/write` (partial and full manifest beside
  `BIMG`), unmount, detach; the test-disk stage directory is `$WORK/stage`. The last step
  (`finish`): the manifest's `image` line, which `sd-manifest.sh` writes as the build path,
  is rewritten to `<image>` as given (as before the change), then image and manifest move
  with `mv -f` to `<image>` and `<image>.manifest`. The EXIT trap unmounts, detaches and
  `rm -rf`s `$WORK` always, and on a non-zero status deletes `<image>` and
  `<image>.manifest` (as before, where they were deleted at the start). The early
  `rm -f` of the old image is gone: nothing at the caller's path is touched before the move.
- `tools/make-sd-image.sh`: the same scheme (`$TMPDIR/make-sd-image.XXXXXX`, `BIMG`, the
  `image` line rewrite, `mv -f` of image and manifest as the last step, one EXIT trap for
  the whole build: detach, `rm -rf "$WORK"`, on failure delete image-path and its
  manifest). The partition still auto-mounts at `/Volumes/NEWTOS` (unchanged).
  Behaviour change the owner's fix implies: a failure after the detach (MBR check, manifest
  writer) no longer leaves the image "for inspection"; the trap removes the temporary
  directory on any failure.
- `tools/sync-card.sh`: **not affected.** Its only `mktemp -d` is its own work directory
  (`TMPD`, already under `$TMPDIR`); it builds no image and creates no mount point. It works
  on the mount points of a card the caller names (`--boot`, `--drive`) and `remount` mounts
  again on that same mount point, so no path on the data volume is involved.
- `TASK/TOOL67.md` line 3.
- Anchor line numbers only, shifted by the 16 header lines of make-sd-image.sh (the
  pre-commit doc-check rejected them): docs/audits/2026-11-07-desk70-tos-exit.md:130
  (550-553 to 566-569) and docs/sdk/05-abi-pitfalls.md:586 (198 to 214). By hand, the same
  edit `doc-check --fix-anchors` makes; that option does not exist at this branch's base.

## Cause of diskutil's refusal (as far as shown)

Shown: with the unchanged mkcard.sh (c61aa2cba) and the image in `<agent>/out-before/`
(mount point `<agent>/out-before/mnt.<pid>` on /Volumes/Daten) `diskutil mount
-mountPoint` prints "Volume on disk7s1 failed to mount" (rc 1). The same script with the
image in the session scratchpad (mount point on the internal Data volume) mounts. With the
mount point under `$TMPDIR` (/var/folders/.../T/) it mounts. The location of the mount
point decides.

Volume flags of /Volumes/Daten (`mount`, `diskutil info`): `apfs, local, nodev, nosuid,
journaled, noowners`; Owners: Disabled; Protocol USB; Device Location External; not
read-only, not encrypted. The internal /System/Volumes/Data: `apfs, local, journaled,
nobrowse, protect, root data` (ownership on).

Not shown: which flag makes diskarbitrationd refuse. `log show` for diskarbitrationd and
mount_msdos over the run's window returned no entry; isolating the flag would need changing
the volume's flags or manual diskutil calls, both outside this ticket. Suspected (unproven):
ownership disabled (`noowners`).

## Image sha256: the unchanged mkcard.sh (normal mode) is not byte-reproducible

Three runs of the unchanged mkcard.sh with the same inputs and stamp under the session
scratchpad gave three different sha256 (`1cb8283b…`, `e3065ce9…`, `f3b53655…`). Differing
bytes between runs: the FAT volume serial (LBA 63, offsets 39..42, set by `diskutil
partitionDisk` at format time), the directory-entry time fields of `SUB`, `.`, `..` and the
label entry (made by `mkdir`, not stamped), and deleted `.fseventsd` entries with their UUID
data in freed clusters; under /private/tmp Spotlight also leaves `C:\.Spotlight-V100` (a
manifest `metadata` line). So the acceptance's sha256 equality cannot hold for any two runs
of this mode; the comparison is file-level (manifest `file` lines). make-sd-image.sh is not
reproducible either: `C:\DATA.BIN`, `C:\KERNEL8.IMG`, `C:\LYNXBOOT.IMG` come from
`/dev/urandom` by design (also stated in tools/sd-manifest.sh). mkcard.sh's
`MKCARD_TESTDISK` mode is reproducible (mtools, no mount): there sha256 equality holds.

## Retest done by the developer (rework, uncommitted tree = this commit's scripts)

Inputs `<agent>/in/NEWDESK.INF` (`test inf\r\n`), `<agent>/in/HELLO.TXT` (`hello\r\n`); mkcard
call `<img> <agent>/in/NEWDESK.INF HELLO.TXT=<agent>/in/HELLO.TXT SUB/X.TXT=<agent>/in/HELLO.TXT`,
default stamp 202610171200.

1. mkcard.sh, normal mode, image `<agent>/out2/card.img`: rc 0, 3 files, sha256
   `3246bf39add3b983a9f84296c7cfdd15b3d8a2bb9c3107b5e7fcec1be5a1af93` (= its manifest's
   `image_sha256`), manifest line `image out2/card.img` (as given). A watcher (`ls out2` and
   `hdiutil info` image-path every 0.2 s, `<agent>/watch-mkcard.log`) saw `out2` empty for
   the whole build while the attached image was
   `/var/folders/16/.../T//mkcard.Ps3dOF/card.img`. Afterwards no `$TMPDIR/mkcard.*`, no
   attached image of mine. The `file` lines equal those of the first commit's card and of
   the unchanged script's card (3 files, same sha256 and sizes).
2. mkcard.sh, `MKCARD_TESTDISK=<agent>/out3/test.img` (tools/mktestdisk.sh, sha256
   `5d2b76d3…`): unchanged script with the image in the scratchpad and fixed script with
   `<agent>/out3/td.img` give the **same sha256**
   `9d7e8bfd3b0ee011392ed9987ba03de2b51892d41fb05a574de74631b8de2629`; the manifests differ
   only in the `image` path line and `created`.
3. Forced failures: (a) mkcard.sh with a source of mode 000 (cp fails after the mount):
   "Permission denied", rc non-zero, `<image>` and manifest gone (the previous good card at
   that path too, as before), no `$TMPDIR/mkcard.*`, no attached image, no mount.
   (b) mkcard.sh sent SIGTERM while filling: rc 143, same clean state. (c) make-sd-image.sh
   with `SD_FSBENCH_D` holding an unreadable `.DAT` (fails after attach and partitionDisk,
   `/dev/disk8`): rc 1, output dir empty, no `$TMPDIR/make-sd-image.*`, no attached image,
   no NEWTOS mount left.
4. make-sd-image.sh with the image `<agent>/out5/sd.img` (and before `<agent>/out4/`): rc 0,
   sha256 `2ac2747e3f74e317294bb2ccb18f164f6b52ef110a38588c59b59c765d39cd06`, manifest
   `image` line = the path given, 54 files, no temp dir left. Watcher
   (`<agent>/watch-msd.log`): `out4` empty for the whole build (36 samples with the image
   attached from `/var/folders/.../make-sd-image.sMJ4Ed/sd.img`), then `sd.img`, then
   `sd.img.manifest`. Against the unchanged script (c61aa2cba's make-sd-image.sh run from
   this worktree, so the same inputs, image in the scratchpad; the temporary copy was
   removed afterwards): `file` lines equal in path and size, sha256 differs only for the
   three `/dev/urandom` files. 54 files instead of the main tree's 86: this worktree lacks
   some build outputs (12 required ones were copied from the main tree's gitignored
   builds; ZVIEW samples are `optional missing`).
5. `cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out <agent>/tosdump-out --scratch
   <agent>/xt-scratch`: "run ended normally after 19 s: desktop up after DUMPBIOS.NAP, rc=0",
   `tosdiff --validate: exit 0`, "result run complete". The same with `--disk`
   (MKCARD_TESTDISK path): rc 0, DISKUNIT/PROGEXT read back, `fsck_msdos -n` exit 0 before
   and after, "result run complete". Scratch subdir removed, no temp dir, no device left.

Agent directory size at the time of writing: 2.4G (`du -sh`).

## Build / clippy / tests

- No Rust code touched. `cargo xtask build --board qemu-virt`: Finished (kernel 93
  warnings, pre-existing). `cargo xtask build --board rpi4`: OK; `--release` rpi4 built by
  the tosdump runs.
- clippy: no crate touched, delta 0. `cargo xtask test`: not required (no tos-api /
  tos-types / tools/xtask change), not run. `bash -n` of both scripts clean.

## Tester handoff

- From a detached worktree of the branch head, images under the tester's
  `target-agents/agent-<id>/`: steps 1-5 above. Expected: rc 0 with the image on
  /Volumes/Daten and no manual diskutil/hdiutil call; the output directory stays empty
  until the end (watch it, and `hdiutil info` shows the image under `$TMPDIR` while it is
  attached); file-level manifest equality with the unchanged script (c61aa2cba) and sha256
  equality in `MKCARD_TESTDISK` mode; after success and after a forced failure no
  `$TMPDIR/mkcard.*` / `$TMPDIR/make-sd-image.*`, no attached image, no mount.
- make-sd-image.sh in a fresh worktree needs the main tree's gitignored build outputs (its
  required list names them).
- Leave the lead's attached test image (directory target-agents, lead-mounttest) and the real card
  mounted on this host (BOOT, NEWTOS1..5) alone.
- No real-Pi4 part; host tools only.

## Proposed docs/STATUS.md line

2026-11-12 — TOOL67: tools/mkcard.sh and tools/make-sd-image.sh build the whole card in a
`mktemp -d` directory under `$TMPDIR` and move image and manifest to the caller's path as
the last step (diskutil refused a mount point on /Volumes/Daten: external USB APFS,
noowners); the trap removes the directory on success and failure. sync-card.sh not
affected. (<commit>)

## Out-of-scope observations

- The TOOL67 sha256 acceptance needs restating as file-level equality for mkcard.sh's
  normal mode and make-sd-image.sh (see above); it holds as written for `MKCARD_TESTDISK`.
- make-sd-image.sh writes through the fixed path /Volumes/NEWTOS: with a real card whose
  volume is named NEWTOS mounted, it would write onto that card (a real card with BOOT,
  NEWTOS1..NEWTOS5 is mounted on this host now).
- `hdiutil attach -nomount` / `detach` print deprecation warnings on this macOS.

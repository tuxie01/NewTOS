# TOOL66 — receipt (unfinished: tester acceptance of the fixed OEM name in the test disk)

Role: developer
Agent: agent-ae04ff118fe96d5ef

Developer, 2026-11-11. Worktree branch `worktree-agent-ae04ff118fe96d5ef`, base `f38100fc2`; not merged.
Ticket [TOOL66](TOOL66.md), run `measurements/measurement-2026-11-11-tool66/RUNINFO.TXT`.

## Done

- `tools/mktestdisk.sh:121@OEM`: after `mformat` (`tools/mktestdisk.sh:112@mformat`) the 8-byte OEM name `MTOO4049` is
  written at boot sector offset 3 of the partition (image byte `63 * 512 + 3`) with
  `printf '%s' "$OEM" | dd of="$IMG" bs=1 seek=... conv=notrunc`; the comment says why the value is fixed and where it
  comes from (mtools 4.0.49, the version of every committed disk reference; finding F1). The header names it among the
  fixed values.
- Image check (Work 2): with mtools 4.0.50 the image is `d95a60c3f5d76fee767b9dc5cd16dab3b2bc1215e116fe738cbdbaa318a92d37`,
  the sha256 recorded with mtools 4.0.49 (`measurements/measurement-2026-10-29-tool19/test.img.manifest`); before the
  change `923d06a5...`. No other byte depends on the mtools version.
- Tables (Work 3): `bios-r2` 0/51, `bios-r2-phys` 0/12, `bios-r2-x001` 0/8 cases differ; the 7 F1 cases equal, no
  other case changed.

## Tests and build

- `cargo xtask test` (full): exit 0, 2454 passed, 0 failed, 7 ignored. `cargo xtask doc-check` of the edited files:
  0 failures; `cargo xtask status-check --new-only`: 0 rejected.
- No kernel, tos-api or tos-types source changed; the kernel the run boots is built from the unchanged sources.

## Missing

- Tester acceptance through `cargo xtask accept TOOL66`.

## Proposed STATUS line

- 2026-11-11 TOOL66: `tools/mktestdisk.sh` writes the OEM name `MTOO4049` after `mformat`; the test disk is again
  `d95a60c3...` with mtools 4.0.50 and the 7 F1 Rwabs cases of the disk tables equal the oracle.

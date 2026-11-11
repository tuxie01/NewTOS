# TOOL66 — mktestdisk writes a fixed OEM name, independent of the mtools version

Status: OPEN (2026-11-11) | Priority: P2

Type: tool. Owner decision 2026-11-11 (option b of the BIOS full run). Split off from [TOOL15](TOOL15.md) (PARKED:
pin mtools, `--fast-forward` as a profile parameter), which stays parked.

## Facts

- The full BIOS run `measurements/measurement-2026-11-11-bios-full/RUNINFO.TXT` found 7 Rwabs cases (`bios-r2`
  58/73/74/75, `bios-r2-phys` 34/36, `bios-r2-x001` 42) that differ only in bytes 9 and 10 of the test image's boot
  sector: the OEM name `mformat` writes into bytes 3-10, `MTOO4049` when the references were recorded (mtools 4.0.49),
  `MTOO4050` today (mtools 4.0.50 on the host). Every other byte and every return value is equal.
- `tools/mktestdisk.sh` formats the partition with `mformat` (`tools/mktestdisk.sh:112@mformat`) and otherwise pins every
  value it can (no configuration file, fixed serial, label, time stamps).

## Work

1. After `mformat`, `tools/mktestdisk.sh` writes the OEM name the references were recorded with (`MTOO4049`, 8 bytes at
   boot sector offset 3) into the partition's boot sector, so the image no longer depends on the mtools version.
   State in the script comment why the value is fixed and where it comes from.
2. Check that no other byte depends on the mtools version (compare an image built today with the recorded image's
   sha256 `d95a60c3…` in `measurements/measurement-2026-11-11-bios-full/RUNINFO.TXT` / the TOOL07 runs).
3. Re-run the three disk tables with the 7 cases (`bios-r2`, `bios-r2-phys`, `bios-r2-x001`) against their
   references: those cases equal, no other case changes.

## Acceptance

- The 7 cases equal; the test image sha256 equals the one recorded with mtools 4.0.49 (or the remaining difference
  is named and explained).
- `cargo xtask test` green, doc-check of the edited files 0 failures.

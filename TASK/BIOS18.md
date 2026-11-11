# BIOS18 — Partition and block-device layer: GPT, AHDI and extended partitions

Status: OPEN (2026-11-11) | Priority: P3

Type: research then development. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G04 (T02).

## Facts

- 28 `unmapped` rows: 16 `bios/disk.c` (`atari_partition`, `process_gpt`, `map_gpt_to_atari`, `process_dos_mbr`, `disk_rescan`, ...; `docs/analysis/deps/mapping.tsv:434-453`) and 12 `bios/blkdev.c` (`blkdev_boot`, `hd_boot_read`, `next_logical`, ...; `:321-348`). Notes: NewTOS reads MBR partitions only.
- Observable through `Drvmap`, `Getbpb`, `Mediach`, `Rwabs` on a card with another scheme. No case table names GPT or AHDI; `bios-r2-phys.tsv` reads the MBR of the test image.

## Work

1. Step 1 (pre-start check): survey which partition schemes the card images and `tools/mktestdisk.sh` produce and which answers NewTOS gives on another scheme. If no card image uses GPT/AHDI/extended partitions, the lead parks it (Reason: no card uses them; Reopen: such a card image).
2. Otherwise: measure the cases on the oracle, fix or classify the 28 rows as design.

## Acceptance

- Survey result recorded; either a case table with oracle dump plus equal NewTOS result, or the 28 rows classified in `mapping.tsv`.

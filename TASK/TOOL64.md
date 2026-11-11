# TOOL64 — Promote `aes-r4-subfunctions.tsv` to a reference at the pin and compare

Status: OPEN (2026-11-11) | Priority: P3

Type: measurement (needs a Hatari run). From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G10 (T09).

## Facts

- `aes-r4-subfunctions.tsv` (181 cases, CRC 227422B2) is the only case table without a pinned-oracle dump; its dump is EmuTOS 1.4 (`measurements/measurement-2026-09-12-aes-r4/RUNINFO.TXT`) and is not in `tosdump/reference/`. 21 AES API nodes depend on it.
- The table header declares two AES commits between 1.4 and the pin.

## Work

1. Run the table on the pinned oracle (`docs/analysis/ORACLE.md`), store dump and log under `tosdump/reference/`, compare with the 1.4 dump and with NewTOS.

## Acceptance

- the pinned-oracle dump of the table (BIN and LOG, named like the other EMUTOS-1a7484ab references) exists under tosdump/reference; tosdiff result recorded.

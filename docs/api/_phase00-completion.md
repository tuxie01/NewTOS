# Phase 00 — Completion Report

> Phase 00 deliverable D13. Marks the EmuTOS source-analysis phase as
> complete and hands off to PHASE-02. Source: EmuTOS @ `1a7484ab`.

## Summary

Phase 00 extracted the TOS-API specification used by NewTOS directly
from the EmuTOS source code, function by function, per the protocol
in `docs/SOURCE-ANALYSIS-PROTOCOL.md`. The result is a set of spec
files under `docs/api/` that every later phase implements against.
EmuTOS is no longer required reading for implementation phases —
the spec entries are now the authoritative reference.

## Coverage

### GEMDOS (Trap 1) — **100% of NewTOS surface**

| Group                  | Specs | Files                                  |
|------------------------|------:|----------------------------------------|
| Console (0x01..0x13)   | 15    | `gemdos/console/*.md`                   |
| Process                | 4     | `gemdos/process/*.md`                   |
| Memory                 | 5     | `gemdos/memory/*.md`                    |
| Time/Date              | 4     | `gemdos/time/*.md`                      |
| Drive / DTA / Version  | 5     | `gemdos/{drive,dta,version}/*.md`       |
| File                   | 14    | `gemdos/file/*.md`                      |
| Directory              | 5     | `gemdos/directory/*.md`                 |
| Misc (Srealloc 0x15)   | 1     | `gemdos/15-srealloc.md`                 |
| **Total**              | **53**|                                        |

### BIOS (Trap 13) — **100%** (12/12)

`bios/_dispatcher.md` plus `bios/00-getmpb.md` … `bios/0b-kbshift.md`.

### XBIOS (Trap 14) — **100%** of NewTOS dispatch table

- 88 entries total (`LAST_ENTRY=0x57`).
- 51 routed via `xbios_unimpl` (return function number, see D-008);
  documented collectively in `xbios/_triage.md`.
- 37 implemented entries fully specified across 27 individual specs +
  3 group specs (`_floppy-stubs.md`, `_psg-group.md`, `_tt-shifter.md`).

### Structures (D6) — 8 files

`docs/api/structures/`: PD, DTA, BPB, MPB/MD, OFD/DFD/FCB, DND/DMD,
OSHEADER, IOREC.

### Subsystem state (D7) — 3 files

- `docs/api/bios/_state.md` — chardev vectors, `boot_status`.
- `docs/api/gemdos/_state.md` — `run`, SFT, buffer cache, drive
  table, `conterm`, memory pool, OS header.
- `docs/api/xbios/_state.md` — `kbdvecs`, key tables, RS-232 map,
  display shadows, screen-dump hooks, sound, RNG.

### Other deliverables

- D1 source layout map — `docs/emutos-map.md`.
- D2 trap dispatchers — 3 files.
- D10 Line-A overview — `docs/api/_line-a.md` (no implementation:
  ADR-030, no Line-A emulation).
- D11 discrepancy log — `docs/api/_discrepancies.md` (D-001..D-008).
- D12 dependency graph — `docs/api/_dependency-graph.md`.
- Working list — `docs/api/_working-list.md`.

### Deferred to PHASE-00b

- D8 AES specs (lib/aes/* in EmuTOS, ~80 functions). Out of scope
  for the present round; AES implementation is Phase 12.
- D9 VDI specs (vdi/* in EmuTOS, ~30 functions). Same reasoning.
- LINEA_VARS structure detailed extraction — dropped: no LINEA block is
  exported, `__aline` stays NULL by design (ADR-030).

## Key learnings (folded into specs)

1. **`xbios_unimpl` returns the function number, not EINVFN**
   (D-008). All "unimplemented" XBIOS entries silently echo their
   trap number. The Compendium is wrong on this point; real Atari
   TOS hardware behaves as EmuTOS does.
2. **Falcon `Dosound`/`Giaccess` without YM2149 returns 0**, not
   the function number. The PSG group group spec explicitly notes
   the difference.
3. **`Supexec` (XBIOS 0x26)** has no clean AArch64 equivalent —
   m68k supervisor-flip semantics do not map. Working assumption
   for NewTOS: a curated MMIO mapping with TPA visibility, ADR
   pending before PHASE-06.
4. **GEMDOS file/directory ABI is FAT-shaped through and through.**
   `Fsfirst`/`Fsnext` rely on a 22-byte private search-state inside
   the user-visible DTA; programs that re-use the DTA mid-search
   silently corrupt iteration. PHASE-05 will preserve this layout.
5. **Bconmap MAPTAB save-on-switch**: device switching saves the
   *current* low-RAM vectors back to the previous slot before
   loading the new ones. Programs that mutated `bconstat_vec[1]`
   directly therefore preserve their changes — but only across
   `Bconmap` calls.
6. **PHASE-02 unblocks except for Cconin/Cnecin/Cconrs.** Those
   three need `Pterm(-32)` (PHASE-06) for Ctrl-C handling. The
   other 12 console functions can be implemented immediately.

## Phase 00 → Phase 02 hand-off

PHASE-02 (GEMDOS Console I/O) is unblocked for 12 of 15 console
functions. Remaining three depend on Phase 06 (process management).
Implementation can begin immediately against the spec files in
`docs/api/gemdos/console/`.

PHASE-03 (Memory Management) is unblocked entirely; spec is in
`docs/api/gemdos/memory/`.

## Open ADRs (pre-implementation)

| ID            | Topic                                   | Blocks   |
|---------------|-----------------------------------------|----------|
| ADR-Supexec   | AArch64 substitute for `Supexec`         | PHASE-06 |
| ADR-BDOS-Lock | BDOS serialisation model on SMP          | PHASE-05 |
| ADR-Line-A-01 | C-ABI shape for Line-A export            | closed: moot, no export (ADR-030) |
| ADR-Line-A-02 | Mouse-cursor state ownership             | closed: the VDI's (ADR-030) |

## Sign-off

- EmuTOS reference pinned: `references/emutos/` @ commit `1a7484ab`.
- Phase 00 deliverables D1, D2, D3 (all groups), D4, D5, D6, D7, D10,
  D11, D12, D13: **complete**.
- Deferred to PHASE-00b: D8 (AES), D9 (VDI).
- Status: `docs/STATUS.md` updated; `docs/api/_working-list.md`
  closed for Phase 00 functions.

**Phase 00 is closed. PHASE-02 may begin.**

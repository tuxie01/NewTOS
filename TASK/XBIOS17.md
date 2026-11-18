# XBIOS17 — qemu-virt: `Bconout(1)` discards, although reception now works

Status: ACCEPTED (2026-11-11) | Priority: P3 | Ref: docs/COMPATIBILITY.md

Migrated (owner batch 16, 2026-11-11, to ACCEPTED), evidence: commit 639c3aa02 (feat(deps): classify mapping gaps, coverage per layer (TOOL1) [fix-subject], fix/feat commits naming the id: n=1

Line 3 before the migration:
Status: **GESCHLOSSEN** 2026-10-29 (ACCEPTED-Abweichung, Anwender) · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29, Fund aus [XBIOS15](XBIOS15.md); Entscheidung offen, erst Bericht

## Facts

- Since XBIOS15 AUX on qemu-virt reads from the ring of device 0
  (`kernel/src/console.rs@AuxSerial`); `Bconout(1)`/`Bcostat(1)` still go
  through `AuxNoCable` (`tos-api/src/bios/chardev.rs:132@AuxNoCable`:
  `Bconout` discards and reports 1, `Bcostat` -1).
- **Lead's report (2026-10-29), no decision:**
  - The UART is shared: `SerialConsole` is the only send path for
    `kprint!`/`kprintln!`, program console output, panic handler and
    self-tests (`kernel/src/console.rs:106-111@SerialConsole`). `Bconout(1)`
    to the UART would mix with the kernel log.
  - What it could affect: runs that search the serial log for markers
    — `cargo xtask tosdump` (`scan_serial`,
    `tools/xtask/src/tosdump.rs:734@scan_serial`, "desktop up", panic lines),
    the probe scripts under `tosdump/probes/` and measurement runs with
    `-serial file:`. These run on raspi4b; there AUX is "no cable" and
    would stay so. On qemu-virt, `virtrun.py`-style runs read the log by
    regex: a program writing to AUX could fake markers
    or tear lines apart.
  - EmuTOS sends AUX output through `bconout1` to the serial port
    (`references/emutos/bios/serport.c:279-317`); EmuTOS has no kernel log
    on that line.

## Work (after the decision)

0. Localisation (lead) per CLAUDE.md "Testing".
1. Decision by the user, e.g. `Bconout(1)` to the UART (separated
   from the kernel log, e.g. kernel log only with `serial=on`), or
   keep discarding as a documented deviation.

## Acceptance

- After the decision; a measurement case against the oracle (Hatari `--rs232-out`).

## User decision 2026-10-29

- ACCEPTED deviation on qemu-virt: AUX output is discarded, the UART is
  shared with the kernel log, qemu-virt is a development board. Entry in
  `docs/COMPATIBILITY.md`; revisit when a port writes to AUX.
  Ticket closed.

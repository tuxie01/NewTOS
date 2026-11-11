# RST02 — Warmstart EmuTOS-treu: Kern-Neueinstieg mit erhaltenen Systemvariablen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/RST02.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — Rest aus [RST01](RST01.md) (Anwenderentscheidung 2026-10-17)

RST01 macht Ctrl+Alt+Del vorerst zum echten Pi-Neustart (wie kalt). EmuTOS'
`warm_reset` ist `jmp _main` mit gueltigen `memvalid`/`memval2`/`memval3`:
Systemvariablen (`resvalid`/`resvector`, `phystop`, `bootdev` …) bleiben, nur
BSS, die ersten 64 Vektoren und Treiber werden neu aufgesetzt, FIRST_BOOT-
Zweige uebersprungen (`bios/panicasm.S:102-107`, `bios/memory.S:117-152,567-616`,
`bios/bios.c:482,492,798,806`; Spec `docs/api/bios/_reset-warm-cold.md`).

Auftrag: Kern-Neueinstieg ohne Firmware (MMU/Caches/Geraete sauber neu),
Systemvariablen-Erhalt nach der Spec, `resvector`-Aufruf; Test in QEMU und auf
der Pi.

[Gemeinsame Regeln](README.md) gelten.

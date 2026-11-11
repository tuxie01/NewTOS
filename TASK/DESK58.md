# DESK58 — `BLKDEVNUM` im Desktop ist 16 statt 26

Status: DONE (2026-11-11) | Priority: P3 | Commit: 3b8098415 | Receipt: DESK58-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/DESK58-STATUS-DONE.MD exists, commit 3b8098415, TASK/DESK58-STATUS-DONE.MD, commit 3b8098415 (fix(desk): valid_drive checks against BLKDEVNUM 26 like EmuTOS (DESK58), fix/feat commits naming the id: 3b8098415 (n=1), measurements/measurement-2026-10-16-desk58/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-16 ([Quittung](DESK58-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Konformitaet — gefunden in der DESK43-Pruefung, gelistet von U5a (2026-10-14)

`kernel/src/desk/main.rs:1471` setzt `BLKDEVNUM = 16` und zitiert dafuer
`bdosdefs.h`; EmuTOS hat `#define BLKDEVNUM 26` (`include/sysconf.h:43`,
A: … Z:). Der Konstantenabgleich (`tools/xtask/emutos-consts-deviations.txt`)
fuehrt ihn als `defect`; nach der Korrektur muss der Eintrag dort entfallen
(der Hosttest erzwingt das). Vorher pruefen, welche Desktop-Pfade die Grenze
benutzen (`main.rs:1454`) und ob Laufwerke Q:–Z: dadurch sichtbar werden.

[Gemeinsame Regeln](README.md) gelten.

## Abschluss — 2026-10-16

Status: **ERLEDIGT (QEMU), nicht committet.** `valid_drive` (jetzt in `programs/desktop/src/main.rs`) prueft den Bereich gegen
`BLKDEVNUM = 26` (`include/sysconf.h:43`); der `defect`-Eintrag in `tools/xtask/emutos-consts-deviations.txt` ist entfernt.
Test: Desktop-Selbsttest `[desk-drv]` 6/2 → 8/0. Q:–Z: werden **nicht** sichtbar: das BIOS meldet hoechstens A:–P:
(`tos-api/src/bios/drive.rs` `MAX_DRIVES = 16`); 45/45 Dumps byte-gleich zu DESK59. Quittung:
[DESK58-STATUS-DONE.MD](DESK58-STATUS-DONE.MD); Beleg `measurements/measurement-2026-10-16-desk58/RUNINFO.TXT`.

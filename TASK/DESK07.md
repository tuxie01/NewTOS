# DESK07 — File → Shutdown: Zustand pruefen

Status: DONE (2026-11-11) | Priority: P4 | Commit: 5336331a3 | Receipt: DESK07-STATUS-DONE.MD

Migrated (owner batch 20, 2026-11-11, to DONE), evidence: receipt TASK/DESK07-STATUS-DONE.MD exists, commit 5336331a3 (docs(desk): record why Shutdown is greyed, and why Execute EmuCON is n), VERIFIED: 5336331a3 (docs(desk): record why Shutdown is greyed, and why Execute E) added the receipt TASK/DESK07-STATUS-DONE.MD itself, the report of this research or measurement ticket (3 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23)** · Prioritaet: **P4** · Typ: Pruefung / ggf. gegenstandslos

Abhaengigkeiten: —

## Sachverhalt

`Item::Shutdown` ohne Handler. EmuTOS zeigt den Punkt nur, wenn
`can_shutdown()` wahr ist (`bios/bios.c:754-770`; auf dem
Falcon030/TOS4-Profil **FALSE**); `shel_write(SHW_SHUTDOWN)` ist dann ein
beobachtbares No-op mit Rueckgabe 1 (A48). Zu pruefen: zeigt unser Menue
den Punkt ueberhaupt, und ist „kein Handler" damit schon das richtige
Verhalten? Auf der Pi waere ein echtes Herunterfahren (Watchdog/PM_RSTC)
moeglich — eigene Entscheidung, nicht EmuTOS-Konformitaet.

## Arbeit

Quelle pruefen, Menuegatter wie EmuTOS setzen; Entscheidung des Leads, ob
die Pi wirklich herunterfaehrt.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-23) — gegenstandslos, das Verhalten war schon richtig.**

EmuTOS zeigt den Punkt immer und schaltet ihn ueber
`menu_ienable(tree, QUITITEM, can_shutdown())` (`desk/deskmain.c:582`);
`can_shutdown()` ist `TRUE` nur auf FireBee, Amiga, Apple Lisa und bei
vorhandenen Native Features (`bios/bios.c:754-770`) und damit auf dem
Falcon030/TOS4-Profil dieses Projekts **FALSE**. Ein dauerhaft graues
`Shutdown` **ist** EmuTOS' Verhalten fuer unsere Maschine — `QUITITEM`s
Rumpf (`deskmain.c:773-778`) wird dort nie erreicht.

NewTOS zeigt den Punkt (`menu.rs` `Item::Shutdown`) und laesst ihn grau; das
lief bis jetzt ueber den Sammelzweig `_ => false`, also ohne Begruendung an
der Stelle. Geaendert wurde nur das: ein eigener `Item::Shutdown`-Zweig mit
der Quellenangabe, damit die Regel dort steht, wo das Verhalten entsteht.
Kein Verhaltensunterschied, keine Messung noetig — `Execute EmuCON` hat
dabei seinen eigenen Satz bekommen, weil es aus einem **anderen** Grund grau
ist (EmuTOS schaltet `CLIITEM` nie ab; uns fehlt der Handler, [DESK06](DESK06.md)).

**Lead-Entscheidung zum echten Herunterfahren:** zurueckgestellt. Die Pi 4
koennte ueber PM_RSTC/Watchdog wirklich abschalten, aber das waere eine
eigene Erweiterung ohne EmuTOS-Vorbild und ist vor dem Meilenstein 2b
(sauberer EmuTOS-Stand) nicht dran. Wenn sie kommt, kommt sie als eigenes
Ticket mit BCM2711-Datenblattbeleg.

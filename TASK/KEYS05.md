# KEYS05 — Alt+Help zaehlt `dumpflg` nicht hoch

Status: PARKED (2026-11-11) | Priority: P4 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue, unmeasured candidate from reading, not yet shown to affect a program | Reopen: a measurement or a program shows the divergence

Migrated (owner batch 10, 2026-11-11), evidence: TASK/KEYS05.md status line: OFFEN (2026-11-07), no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** (2026-11-07) · Prioritaet: **P4** · Typ: Abweichung, gelesen, nicht gemessen — Nebenbefund aus [V34](V34.md)

## Sachverhalt

EmuTOS: Alt+Help erhoeht `dumpflg` (`references/emutos/bios/ikbd.c:595-597`, "tell VBL to call scrdmp() function"); der VBL ruft `dump_vec` erst bei `dumpflg == 0` und setzt danach -1 (`bios/vectors.S:466-472`). NewTOS reiht Alt+Help als gewoehnliche Taste ein und zaehlt nicht (`kernel/src/ikbd.rs:243@decode_key`). Den VBL-Dump gibt es bei uns nicht, `dump_vec` rufen ist ADR-031; sichtbar waere nur der Wert in `$4EE`.

Dazu aus V34, ebenfalls nicht gemessen: `dumpflg` liegt in NewTOS je Prozess in der Seite 0, in EmuTOS ist es eine globale Zelle; ein Kind sieht einen vom Elternprozess gesetzten Wert nicht.

## Auftrag

Wirkung zuerst: liest ein Kartenprogramm oder ein geplanter Port `$4EE`? Ohne Nutzer parken (Vor-Start-Pruefung). Sonst am Orakel messen (Alt+Help per Injektor, `$4EE` davor und danach) und wie EmuTOS nachbauen.

[Gemeinsame Regeln](README.md) gelten.

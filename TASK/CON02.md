# CON02 — `Cconout`/`Cconws` folgen nicht der Umlenkung von Handle 1

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: a37f0c7d4 | Missing: tester acceptance through cargo xtask accept

Migrated (owner batch 17, 2026-11-11, to IMPLEMENTED), evidence: commit a37f0c7d4 (feat(gemdos): character functions follow p_uft redirection (PROC04 pt ), VERIFIED: a37f0c7d4 (feat(gemdos): character functions follow p_uft redirection () first added the cited measurements/measurement-2026-10-18-proc04-redir, legacy line: ERLEDIGT 2026-10-18 (mit PROC04 Punkt 1, `1a306120`)

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-18 (mit PROC04 Punkt 1, `1a306120`) · Prioritaet: **P3** · Typ: Konformitaet — Nebenfund LIBC-FSTAT (2026-10-17)

NewTOS-`Cconws` schreibt immer auf CON, auch wenn Handle 1 per `Fforce`
umgelenkt ist (`measurements/measurement-2026-10-17-libc-fstat/`). EmuTOS
leitet die Konsolenfunktionen ueber die Standard-Handles des Prozesses:
`run->p_uft[1]` fuer Ausgabe, `run->p_uft[0]` fuer Eingabe
(`bdos/console.c:161,170,281,377,401`). Researcher: genaues Verhalten belegen
(Ziel Geraet vs. Datei, was `HXFORM` bei einem Datei-Handle tut, welche
C-Funktionen betroffen sind), Spec `docs/api/gemdos/console/*` ergaenzen,
dann Developer. Beruehrt FS11 (Geraetetabelle) und CON01.

[Gemeinsame Regeln](README.md) gelten.

## Erledigt (2026-10-18) — mit PROC04 Punkt 1 (`1a306120`)

Siehe [PROC04](PROC04.md); Belege `measurements/measurement-2026-10-18-proc04-redir*/`.

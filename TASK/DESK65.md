# DESK65 — Einstellungen des Desktops sofort speichern (Erweiterung)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 10, 2026-11-11), evidence: TASK/DESK65.md status line: OFFEN (Erweiterung), no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN (Erweiterung)** · Prioritaet: **P3** · Typ: Weiterentwicklung ueber EmuTOS hinaus — Anwenderentscheidung 2026-10-17

## Hintergrund

EmuTOS schreibt Desktop-Einstellungen erst beim naechsten „Desktop
sichern“ in die `.INF` (z. B. `inf_backgrounds` endet ohne Speichern,
`desk/deskinf.c:919`). NewTOS' Hintergrund-Dialog speichert dagegen sofort
(`cnx_put(); app_save(true)`, `programs/desktop/src/inf.rs` `set_background`).

Anwender 2026-10-17 (bei DESK64): *„ich empfand das sofortige speichern gar
nicht so schlimm. Ich denke wir werden das überall einbauen.“* Das
sofortige Speichern im Hintergrund-Dialog **bleibt** daher, als benannte
Abweichung dokumentiert (DESK64).

## Auftrag (spaeter)

1. Alle Desktop-Dialoge auflisten, die Einstellungen aendern (Hintergrund,
   Desktop-Konfiguration, Voreinstellungen, Fenster-/Ansichtsoptionen,
   Anwendung anmelden, …) mit ihrer EmuTOS-Funktion.
2. Festlegen, welche sofort speichern sollen, und das einheitlich bauen
   (`cnx_put` vor `app_save`, wie jeder EmuTOS-`app_save`-Aufruf).
3. Als Erweiterung in `docs/COMPATIBILITY.md` eintragen; „Desktop
   sichern“ bleibt funktionsfaehig.

Einordnung: Weiterentwicklung ueber EmuTOS hinaus — nach dem sauberen
EmuTOS-Basisstand einplanen, sofern der Anwender nichts anderes sagt.

[Gemeinsame Regeln](README.md) gelten.

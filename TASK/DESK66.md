# DESK66 — Desktop-Hintergrund an die AES melden (`set_aes_background`)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 10, 2026-11-11), evidence: TASK/DESK66.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — Nebenfund DESK64, Anwender 2026-10-17: Weg (a)

## Sachverhalt

EmuTOS' `inf_backgrounds` ruft nach OK `set_aes_background(curdesk & 0xff)`
(`desk/deskinf.c:904`). Die Funktion ist AES-intern: die AES malt beim
Programmstart den Desktop-Hintergrund und kennt den neuen Wert sofort.

NewTOS' Desktop ist das EL0-Programm NEWDESK (ADR-026) und hat dafuer keinen
Aufruf. Die AES liest den Hintergrund nur beim eigenen Start aus `#Q`; bis
zum naechsten Neustart malt sie beim Programmstart den alten Hintergrund.
Vermerkt in `programs/desktop/src/inf.rs` (`inf_backgrounds`) und
`docs/api/desktop/deskinf.md`.

## Auftrag

Einen Weg schaffen, auf dem der Desktop den Wert an die AES gibt — kleinste
Schnittstelle, die es braucht. Zuerst pruefen, ob ein vorhandener AES-Aufruf
den Wert bereits traegt (z. B. `wind_set(0, WF_NEWDESK…)`/Shell-Puffer,
EmuTOS' Weg des Werts von `#Q` in die AES); erst wenn keiner passt, eine
neue Schnittstelle als **benannte Abweichung** (Spec in `docs/api/`,
`docs/COMPATIBILITY.md`). Beleg: QEMU-Screendump eines Programmstarts direkt
nach Hintergrundwechsel, vorher alter, nachher neuer Hintergrund.

[Gemeinsame Regeln](README.md) gelten.

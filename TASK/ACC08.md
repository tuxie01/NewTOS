# ACC08 — Accessory mit eigenem Fenster

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/ACC08.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Pruefung / Funktionsluecke — Nebenbefund ACC04

Abhaengigkeiten: [ACC04](ACC04.md) (erledigt)

## Sachverhalt

Der Pruefling `sdk/examples/sysacc.c` zeigt seine Werte mit `form_alert` —
bewusst so entschieden (ACC06), weil es ein einziger AES-Aufruf ist und damit
die groesste Chance hatte, auf der frischen Accessory-Implementierung zu
laufen.

**Ungetestet ist damit der Fall, den ein echtes Accessory braucht:** ein
**eigenes Fenster**. Das treibt den Fenstermanager aus einem **zweiten**
Klienten — `wind_create`/`wind_open`, `WM_REDRAW` an einen anderen Besitzer
als den Desktop, Fenster, die einen Programmstart ueberleben oder eben nicht.

## Arbeit

Eine Fensterfassung von `sysacc.c` bauen (der Alert bleibt als einfacher Fall
stehen), damit fahren, und die Befunde einzeln melden. Es ist ausdruecklich
**erwuenscht**, dass dabei etwas bricht — dafuer ist der Test da.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

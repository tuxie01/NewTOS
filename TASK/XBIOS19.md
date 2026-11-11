# XBIOS19 — `Bioskeys` liefert einen Zeiger auf die Ausgangsstruktur, nicht auf die aktuelle

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: no program calls Bioskeys | Reopen: a program calls Bioskeys

Migrated (owner batch 12, 2026-11-11), evidence: TASK/XBIOS19.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm ruft Bioskeys)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm ruft Bioskeys) · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29 („wenn es in EmuTOS so ist“), Nachlauf aus [XBIOS02](XBIOS02.md)

## Sachverhalt

- EmuTOS: `bioskeys` ist `void`, in `d0` bleibt der Wert von `get_keytbl()` —
  die Adresse der **unveraenderlichen** Ausgangsstruktur des Landes, nicht
  `&current_keytbl` (`references/emutos/bios/ikbd.c:136-140`; Orakel `0x00E06E8C`,
  ROM). Auf dem Orakel ist die Rueckgabe von `Bioskeys()` daher **ungleich** der
  von `Keytbl(-1,-1,-1)`.
- NewTOS seit XBIOS02: die Adresse der eben zurueckgesetzten aktuellen Struktur,
  also **gleich** `Keytbl(-1,-1,-1)` (`docs/api/xbios/18-bioskeys.md`,
  „Return value“).

## Arbeit

1. Eigene, fuer Programme nur lesbare Ausgangsstruktur im Adressraum des
   Aufrufers; `Bioskeys` liefert ihre Adresse. Spec „Return value“ anpassen.
2. Fall beidseitig: `Bioskeys()`-Rueckgabe ungleich `Keytbl(-1,-1,-1)`, und die
   drei Zeiger dahinter zeigen auf die Ausgangstabellen (Sonde K07/K11).

## Abnahme

- Sonde wie oben gleich dem Orakel (Zeigerbeziehung, nicht Zeigerwert);
  `cargo xtask test` gruen; STATUS im selben Commit.

[Gemeinsame Regeln](README.md) gelten.

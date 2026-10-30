# TOOL27 — `rsprobe`-Faelle nach tosdiff

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-30, Folgeticket von [XBIOS06](XBIOS06.md) (Sondenregel, CLAUDE.md "Testing")

## Sachverhalt

- XBIOS06 nimmt die dauerhafte Sonde `rsprobe.c` (neu, unter `tosdump/probes/`) als
  Abnahmefall (MEASURED, beide Seiten). Nach der Sondenregel bekommt jede
  solche Abnahme ein Folgeticket, das den Fall nach tosdiff bringt.
- Hindernis: die Schutzliste der Tabellen verbietet `Rsconf` mit gesetzter
  Baudrate (`tosdump/core/cases.c:4050-4055@CR_RSCONF`).

## Arbeit

1. Schutzliste so erweitern, dass die `rsprobe`-Faelle als Tabellenfaelle
   laufen (Wiederherstellung des Ausgangszustands am Fallende).
2. Feld-Dump von `Iorec(0)` als Ergebnisklasse (`r:extiorec`, TOOL16-Layout).
3. Neue Tabelle (eingefrorene Tabellen bleiben unveraendert), Referenzen vom
   gepinnten Orakel.

## Abnahme

- Neue Tabelle laeuft auf beiden Seiten; Ergebnis gleich der `rsprobe`-Messung.

[Gemeinsame Regeln](README.md) gelten.

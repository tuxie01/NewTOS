# TOOL25 — `Setexc`-Abfrage mit Wert `0x00000000FFFFFFFF` als tosdiff-Fall

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket nach CLAUDE.md „Testing“ (Sonde als Abnahme), aus [BIOS07](BIOS07.md) P3

## Sachverhalt

BIOS07 P3 ist per Sonde `tosdump/probes/sxprobe.c` abgenommen (beidseitig,
`bios-arg-trace`; `measurements/measurement-2026-10-29-bios07/RUNINFO.TXT`).
DUMPBIOS erlaubt bei `Setexc` nur die Abfrage `-1`; der Fall „oberes Wort 0,
unteres `0xFFFFFFFF` ist Abfrage“ ist darum keine Tabellenzeile.

## Arbeit

DUMPBIOS-Waechter fuer `Setexc` so lockern, dass dieser Wert zugelassen ist
(er schreibt nicht), Fall in eine neue Tabelle, Referenzen am Pin.

## Abnahme

- Fall EQUAL auf tt/falcontc; bestehende Tabellen unveraendert.

[Gemeinsame Regeln](README.md) gelten.

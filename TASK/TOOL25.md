# TOOL25 — `Setexc`-Abfrage mit Wert `0x00000000FFFFFFFF` als tosdiff-Fall

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: associated fix BIOS07 accepted via probe | Reopen: the owner wants the probe moved to tosdiff

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL25.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS07) per Sonde abgenommen)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS07) per Sonde abgenommen) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket nach CLAUDE.md „Testing“ (Sonde als Abnahme), aus [BIOS07](BIOS07.md) P3

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

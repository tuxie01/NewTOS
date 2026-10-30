# TOOL26 — DUMPBIOS: Fallart „Speicher im Super-Zustand schreiben“ (Sonde `vwprobe` nach tosdiff)

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket nach CLAUDE.md „Testing“ (Sonde als Abnahme), aus [BIOS16](BIOS16.md)

## Sachverhalt

BIOS16 ist per Sonde `tosdump/probes/vwprobe.c` abgenommen. DUMPBIOS erlaubt
bei `Setexc` nur die Abfrage und kennt keine Fallart, die Speicher im
Super-Zustand schreibt (`tosdump/core/cases.c`).

## Arbeit (Bedarf laut BIOS16-Developer)

- Fallart: eine LONG-Konstante an eine Adresse aus einer Erlaubnisliste
  (`$008`–`$40B`) schreiben, per `Super`/`Supexec`, beidseitig gleich.
- Folgefall `Setexc(n,-1)` EXACT gegen die Konstante.
- Den Wert am Ende des Falls zwingend direkt zurueckschreiben, vor jedem
  `Pterm` oder Plattenaufruf (EmuTOS ruft `etv_term`/`etv_critic` wirklich).
- `$408`-Fall als ACCEPTED markiert.
- T02 (Rueckgabe beim `Pterm` des Kindes) braucht einen Kindprozess-Fall
  (`Pexec` eines Helfers) oder bleibt dokumentierte Abweichung ohne Tabelle.

## Abnahme

- Neue Tabelle, Referenzen am Pin (tt/falcontc), Faelle wie `vwprobe`; Sonde
  danach abgeloest; bestehende Tabellen unveraendert.

[Gemeinsame Regeln](README.md) gelten.

# TOOL35 — tosdiff-Fall fuer `dumpflg` nach `Scrdmp` und `v_hardcopy`

Status: **GEPARKT** 2026-11-07 (zugehoeriger Fix [V34](V34.md) per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket der Sondenregel (CLAUDE.md "Testing") aus V34

## Sachverhalt

V34 (VDI-TEXT-011, 012, `docs/audits/2026-11-06-v18-t2-vdi-text.md`) setzt `dumpflg` (`$4EE`) in `Scrdmp` auf -1 wie EmuTOS `bios/xbios.c:490-494`, und Escape 17 ruft `Scrdmp` wie `vdi/vdi_esc.c:260-263`. Abnahmefall ist die dauerhafte Sonde `tosdump/probes/vdprobe.c` (T18A; Lauf `measurements/measurement-2026-11-07-v34/RUNINFO.TXT`). Ein tosdiff-Fall geht heute nicht: DUMPBIOS kann eine Systemvariable nicht vor und nach einem Aufruf lesen.

## Auftrag

Dumper oder Tabellenformat so erweitern, dass ein Fall eine Systemvariable (WORD an fester Adresse) vor und nach dem Aufruf festhaelt; T18A als Fall in eine Tabelle, beidseitig gegen das gepinnte Orakel. Danach die Sonden-Abnahme von V34 in V34 auf den tosdiff-Fall umschreiben.

[Gemeinsame Regeln](README.md) gelten.

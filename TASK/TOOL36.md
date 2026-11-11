# TOOL36 — tosdiff-Faelle fuer die lokale `PTSIN`-Kopie (TPF–TPH)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: associated fix V35 accepted via probe, tool ticket behind the VDI work (2026-11-07) | Reopen: the owner wants the probe moved to tosdiff

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL36.md status line: GEPARKT 2026-11-07 (zugehoeriger Fix [V35](V35.md) per Sonde abgenommen, Werkzeug-Ticket hinter der VDI-Arbeit

Line 3 before the migration:
Status: **GEPARKT** 2026-11-07 (zugehoeriger Fix [V35](V35.md) per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket der Sondenregel (CLAUDE.md "Testing") aus V35

V35 ist mit der dauerhaften Sonde `tosdump/probes/vdprobe.c` (`t35`: TPF fremder Handle, TPG zweite Workstation, TPH Schreiben in die Kopie) abgenommen, Lauf `measurements/measurement-2026-11-07-v35/RUNINFO.TXT`. DUMPVDI kann heute weder einen fremden Handle noch eine zweite Workstation zwischen Faellen ausdruecken. Auftrag: Tabellenformat oder Dumper so erweitern, dass diese Faelle beidseitig gegen das gepinnte Orakel laufen; danach die Sonden-Abnahme in V35 umschreiben.

[Gemeinsame Regeln](README.md) gelten.

# TOOL34 — tosdiff-Fall mit mehr als 1024 `PTSIN`-Punkten

Status: **GEPARKT** 2026-11-06 (zugehoeriger Fix [V23](V23.md) per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Pflicht-Folgeticket der Sondenregel (CLAUDE.md "Testing") aus V23

## Sachverhalt

V23 (VDI-DRAW-005, `docs/audits/2026-11-05-v18-t1-vdi-draw.md`) kappt `CONTRL[1]`
zentral auf `MAX_VERTICES` = 1024 wie EmuTOS `GSX_ENTRY`
(`references/emutos/vdi/vdi_asm.S:88-116`). Abnahmefall ist die dauerhafte Sonde
`tosdump/probes/vdprobe.c` (`k5`, Faelle K5A–C; Messung
`measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`). Ein tosdiff-Fall geht
heute nicht: der Dumper fasst 256 `PTSIN`-WORDs, also 128 Punkte
(`TD_VDI_PTSIN_N`, `tosdump/core/tosdump.h:123@TD_VDI_PTSIN_N`); die Falltabelle liest die
Spalte `ptsin` in genau diesen Puffer (`tosdump/core/cases.c:356@parse_list`), und der
VDI-Satz speichert `ptsin[256]` nach dem Aufruf mit festem Versatz
(`TD_VDI_OFF_PTSIN`, `TD_VDI_RECORD_SIZE` 1592).

## Auftrag

Den Dumper so erweitern, dass ein Fall mehr als 1024 Punkte uebergeben kann
(z. B. ein eigener, groesserer `PTSIN`-Puffer fuer den Aufruf und eine
Tabellen-Schreibweise fuer wiederholte Punkte), ohne das Satzformat der
eingefrorenen Tabellen und Referenzen zu aendern. Dann eine neue Tabelle mit
den Faellen K5A–C (`v_pmarker` 1030 und 1025, `v_pline` 1030; `CONTRL[1]`
nach dem Aufruf) und je einem Kontrollfall mit genau 1024 Punkten, gemessen
gegen das gepinnte Orakel (`docs/analysis/ORACLE.md`).

## Abnahme

- Eingefrorene Tabellen und Referenzen byte-gleich; neue Tabelle auf beiden
  Seiten, Ergebnis gleich der `vdprobe`-Messung K5A–C.

[Gemeinsame Regeln](README.md) gelten.

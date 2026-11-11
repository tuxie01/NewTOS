# ZVIEW01 — zView browser shows no file or folder names

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt ZVIEW01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 3b9f155f2 (fix(sdk): zView's own text renderer builds its gly), named by body

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P1** (Vorfuehrung) · Typ: Defekt, Ursache unbekannt — Anwender auf der echten Pi 4, 2026-10-07

## Sachverhalt

Kernel HEAD `3895575` (release), `ZVIEW.NAP` sha `9c6497ad…`, Karte nach dem
Aufraeumen vom 2026-10-07. Im Datei-Browser von zView (Baum links, Kacheln
rechts) erscheinen Laufwerke, Ordner und Dateien nur als Symbole; **weder
Datei- noch Ordnernamen werden gezeichnet** (Foto des Anwenders, Baum mit
aufgeklappten Ordnern, Kacheln mit vier Ordnern und zwei Dateien ohne Text).

## Arbeit

Erst die Ursache mit Belegen (welcher VDI/AES-Aufruf zeichnet die Namen in
zView/WinDom — Textausgabe, Schriftwahl, Farbe, Clipping, Schreibmodus — und
was NewTOS dort anders macht als EmuTOS), in QEMU nachstellen, dann Fix
nach EmuTOS.

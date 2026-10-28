# TOOL06 — Orakel-Plattenprofil: EmuTOS liest dieselbe FAT16-Karte wie NewTOS

Status: **ERLEDIGT** 2026-10-28 ([Quittung](TOOL06-STATUS-DONE.MD)) · Prioritaet: **P1** · Typ: Werkzeug (Orakel) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Das Orakel hat kein BIOS-Blockgeraet: `tools/hatari-oracle.sh` haengt immer
  einen GEMDOS-Ordner als C: ein und startet den Dumper von dort
  (`docs/analysis/ORACLE.md` §2). Hatari faengt GEMDOS auf diesem Laufwerk
  selbst ab; ein Plattenfall maesse Hatari, nicht EmuTOS.
- Das Audit „BIOS-Speicher“ hat sich mit einem eigenen Skript beholfen
  (IDE-Master = C:, GEMDOS-Ordner = D: nur fuer das Programm,
  `measurements/measurement-2026-10-28-bios-storage-audit/method/run-emutos.sh`);
  ob das eine Orakel-Aenderung nach ORACLE §7 ist, war nirgends entschieden
  (`docs/audits/2026-10-28-bios-storage.md`, Abschnitt 11, Punkte 1–2).
- Anwenderentscheidung 2026-10-28: ein **zweites, eigenes Orakelprofil** fuer
  Plattenfaelle mit eigenem Kanarienvogel in ORACLE.md, **keine Aenderung** an
  `tt` und `falcontc` (Profile, `tools/oracle/hatari.cfg`, Referenzen,
  Kanarienvogel §6 bleiben bytegleich). `Drvmap` wird danach je Profil exakt
  verglichen (TOOL07).
- Blockiert: TOOL07, jedes weitere Speicher-Audit, DUMPGDOS.

## Arbeit

1. Profil in `tools/hatari-oracle.sh` (Name z. B. `disk`; Maschine nach
   Begruendung, die Wahl steht in ORACLE §2): FAT16-Testabbild als
   IDE-Master, Floppys aus, das Dumper-Programm auf einem getrennten
   Laufwerk, nie auf dem Testabbild. Wie das Programmlaufwerk entsteht
   (GEMDOS-Ordner, zweites IDE-Abbild, …) und welche Bits das in `_drvbits`
   setzt, misst und begruendet der Bericht; Hatari darf das Testabbild nicht
   ueber seine GEMDOS-Emulation erreichen.
2. Eine eigene Hatari-Konfiguration nur fuer dieses Profil, falls die
   Plattenwerte in `tools/oracle/hatari.cfg` gehoeren wuerden; die bestehende
   Datei bleibt unveraendert.
3. Das Testabbild entsteht reproduzierbar aus einem Befehl (Groesse,
   Partitionstabelle, Dateien); sha256 vor und nach jedem Lauf im RUNINFO.
   Gleiches Abbild auf der NewTOS-Seite (TOOL07).
4. ORACLE.md: §2 neue Zeile in der Profiltabelle samt Kommandozeile und
   Laufwerksbelegung; §6 eigener Kanarienvogel fuer das Profil (Fallsatz,
   Dumper-Build, erwartete sha256, ein Befehl mit Scratch-Fallback wie §6).
   Kanarien-Faelle: die nicht schreibenden Faelle der Plattenrunde, sobald
   TOOL07 sie hat; bis dahin `bios-r1.tsv`.
5. `.claude/agents/auditor.md`, Abschnitt „Disk cases“: auf das Profil
   verweisen (Wortlaut im Bericht vorschlagen, der Lead traegt ein).

## Abnahme

- Bestehender Kanarienvogel §6 (`tt`, `falcontc`) weiter OK, unveraenderte
  sha256.
- Neues Profil zweimal hintereinander: bytegleiche BIN/LOG; Testabbild nach
  einem lesenden Lauf unveraendert (sha256).
- Beleg, dass EmuTOS das Testabbild selbst liest (z. B. Hatari-IDE-Trace
  oder geaenderte Daten auf dem Abbild, die der Dumper sieht).
- doc-check der geaenderten Doku 0 Fehler.

# TOOL07 — DUMPBIOS-Plattenrunde: `Rwabs`, `Getbpb`, `Mediach`, `Drvmap` per tosdiff

Status: **OFFEN** · Prioritaet: **P1** · Typ: Werkzeug (Messung) — Anwenderentscheidung 2026-10-28 · haengt an [TOOL06](TOOL06.md)

## Sachverhalt

- DUMPBIOS weist `Rwabs`, `Getbpb` und `Mediach` ab
  (`tosdump/core/cases.c:3077@call_refusals`); das Audit „BIOS-Speicher“
  konnte sie nur per eigener Sonde messen, alle zehn Befunde blieben
  PLAUSIBLE (`docs/audits/2026-10-28-bios-storage.md`, Abschnitte 5 und 11).
- `Drvmap` (Fall 74, Klasse `NONZERO`) besteht fuer jede nichtleere
  Laufwerkskarte (NewTOS `0x4`, Orakel `0x7`). Anwenderentscheidung
  2026-10-28: je Profil **exakt** vergleichen.
- Auf QEMU `raspi4b` gibt es nur eine SD; die Rollenregel „Programm nie auf
  dem Testabbild“ war dort unerfuellbar. Anwenderentscheidung 2026-10-28:
  **getrennte Programmpartition** auf der Karte; der Abbildvergleich
  umfasst nur die Testpartition.
- Die Faelle dienen nach einem Fix als Regressionsfaelle (BIOS01).

## Arbeit

1. Plattenfallsatz (z. B. bios-r2.tsv unter `tosdump/cases/`; Vorschlag:
   `docs/audits/2026-10-28-bios-storage.md` Abschnitt 5) mit eigenem
   Ledger-Abschnitt in `tosdump/cases/COVERAGE-BIOS.md`; die Abweisung in
   `call_refusals` nur fuer diesen Satz bzw. diese Aufrufformen aufheben.
2. Schreibfaelle (BIOS-STOR-001, -004–006): Folgen mit Vorspann und
   Nachspann, die den Ausgangszustand der Testpartition wiederherstellen;
   Vorlage sind die Gruppen W und X der Sonde
   (`measurements/measurement-2026-10-28-bios-storage-audit/method/bstprobe.c`).
   Format- und Dumper-Aenderungen in `docs/TOS-REFERENCE-DUMPER.md`.
3. `Drvmap`: exakter Vergleich je Profil — Referenz des Plattenprofils
   (TOOL06) und NewTOS-Karte mit festgelegter Laufwerksbelegung; die
   Vergleichsregel steht in der Falltabelle, `tosdiff --selfcheck` besteht.
4. NewTOS-Seite: `tools/mkcard.sh` (bzw. ein Schalter dort) baut eine Karte
   mit Testpartition und getrennter Programmpartition; Manifest wie gehabt.
   Der Abbildvergleich vor/nach dem Lauf betrifft nur die Testpartition
   (Zeitstempel normalisiert, `fsck_msdos` auf der herausgeschnittenen
   Partition).
5. Referenzen erst auf dem Orakel (Plattenprofil), neue Dateien
   `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2*`; dann NewTOS in QEMU
   `raspi4b`. Messabweichungen sind Funde, kein Fix.
6. `.claude/agents/auditor.md`: Wortlaut fuer die Programmpartition im
   Bericht vorschlagen (der Lead traegt ein).

## Abnahme

- Orakel: zwei Laeufe bytegleich (ausser Klassen mit begruendeter
  Normalisierung); Testabbild nach dem Lauf wieder wie vorher, sofern die
  Folgen es zuruecksetzen (sha256 belegt).
- NewTOS: Lauf vollstaendig, tosdiff-Ergebnis je Fall im RUNINFO; die
  Sondenbefunde BIOS-STOR-001, -003, -004, -005, -006, -007 haben je einen
  tosdiff-Fall (DIFF oder EQUAL).
- `tosdiff --selfcheck` besteht fuer alle Referenzen; Hosttests gruen
  (`cargo xtask test` und `tosdump/tests`).
- doc-check der geaenderten Doku 0 Fehler.

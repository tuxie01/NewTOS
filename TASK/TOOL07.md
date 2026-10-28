# TOOL07 — DUMPBIOS-Plattenrunde: `Rwabs`, `Getbpb`, `Mediach`, `Drvmap` per tosdiff

Status: **TEILWEISE** (Etappe 1 2026-10-28, siehe unten) · Prioritaet: **P1** · Typ: Werkzeug (Messung) — Anwenderentscheidung 2026-10-28 · haengt an [TOOL06](TOOL06.md)

## Sachverhalt

- DUMPBIOS weist `Rwabs`, `Getbpb` und `Mediach` ab
  (`tosdump/core/cases.c:3097@call_refusals`); das Audit „BIOS-Speicher“
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

## Stand 2026-10-28 (Etappe 1)

- Erledigt: `!disk`-Tabellen in DUMPBIOS (Vorspann nur lesend, Schreiben nur
  als Write-back desselben Sektors), `tosdump/cases/bios-r2.tsv` (51 Faelle),
  Referenz `EMUTOS-1a7484ab-BIOS-r2-disk` (Profil `disk`, zwei Laeufe
  bytegleich, Abbild unveraendert), Karte mit `MKCARD_TESTDISK`
  (Partition 1 = Testabbild bytegleich, Partition 2 = Programme), `Drvmap`
  EXACT: Orakel und NewTOS `0xC`, EQUAL. NewTOS: 10 von 51 DIFF —
  BIOS-STOR-001 (7 Faelle), -003 (1), -007 (2). Beleg
  `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`.
- Offen: BIOS-STOR-004, -005, -006 brauchen Bootsektoren, die vom Abbild
  abweichen (Patch-Schritt im Dumper); nach gescheitertem `Getbpb` geht
  EmuTOS auf `recsiz=0`, Wiederherstellung dann nur ueber `RW_NOTRANSLATE`,
  dessen Einheit ein Geraeteparameter ist (BIOS-STOR-002, BIOS02).
  Optionen des Developers (Entscheidung beim Anwender): (B) Abschlusstabelle
  je Folge mit Vorspann-Art „Byte n setzen“, ohne Wiederherstellung auf dem
  verworfenen Klon — empfohlen; (A) Token fuer die physische Einheit, nach
  BIOS02; (C) vorbereitete Abbilder — fuer 004 ungeeignet; (D) Einheit als
  Dumper-Argument.

## Entscheidungen des Anwenders 2026-10-28 (Rest)

- **Variante B** fuer BIOS-STOR-004..006: Abschlusstabelle je zerstoerender
  Folge. Neue **Runner-Regel, ausdruecklich**: eine zerstoerende Tabelle
  laeuft immer auf einem frischen Plattenklon (Orakel und NewTOS); in der
  Tabelle als zerstoerend markiert, der Runner verweigert sonst.
- Nach jeder zerstoerenden Tabelle: Abbildvergleich beider Seiten; beide
  muessen im **gleichen Endzustand** enden, auch mit dem kaputten Bootsektor
  (Vergleich der Testpartition wie `compare-testpart.sh`, Ergebnis je Tabelle
  im RUNINFO).
- Vorspann-Schritt „Byte n setzen“: Format waehlt der Lead, minimal und in
  der Falltabelle (siehe unten).
- 002 (`RW_NOTRANSLATE`): Fall mit Einheiten-Token, aufgeloest aus der
  Profildefinition (Orakel `disk` 18, NewTOS 26), keine Suchlogik; die
  Dumper-Sperre fuer `RW_NOTRANSLATE` faellt nur fuer das Token.
- Kanarienvogel des Plattenprofils (ORACLE §6) auf die nicht zerstoerenden
  Faelle von `bios-r2.tsv` umstellen, sobald deren Referenz zweimal bitgleich
  reproduziert ist.

### Format „Byte n setzen“ (Lead-Wahl)

Kopfzeile `!disk<TAB>2<TAB>destructive` markiert die Tabelle als
zerstoerend (Runner: nur auf frischem Klon). Darin eine zusaetzliche
Vorspann-Form `+set<TAB><offset>=<byte>[,<offset>=<byte>…]`: aendert den
Puffer des unmittelbar vorangehenden Lese-Vorspanns; der folgende Schreibfall
schreibt diesen Puffer zurueck (die Write-back-Regel gilt dann fuer
„gelesen + gesetzt“). Offset und Wert hexadezimal (`0x…`) oder dezimal.
Ohne `destructive` wird `+set` abgewiesen. Kein weiterer Mechanismus; Format
in `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6.

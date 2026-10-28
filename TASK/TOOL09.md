# TOOL09 — Ein Befehl fuer einen tosdump-Lauf auf NewTOS

Status: **OFFEN** · Prioritaet: **P2** · Typ: Werkzeug (Messablauf) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Der Auditor-Pre-flight verlangt „run a case on NewTOS through
  `cargo xtask`“ (`.claude/agents/auditor.md`, Pre-flight 2). Einen solchen
  Befehl gibt es nicht; jeder Lauf baut sich ein eigenes Skript
  (`measurements/measurement-2026-10-27-dumpbios/method/run-newtos.sh`,
  `measurements/measurement-2026-10-28-bios-storage-audit/method/run-dumpbios-newtos.sh`):
  Dumper bauen, Karte per `tools/mkcard.sh` mit `#Z`-Autostart, QEMU
  `raspi4b` mit `-append serial=on`, auf „desktop up“ warten, BIN/LOG per
  `mcopy` holen, tosdiff gegen die Referenz.
- Anwenderentscheidung 2026-10-28: Pre-flight 2 nennt den tatsaechlichen
  Befehl; fehlt er, wird er gebaut (dieses Ticket).

## Arbeit

1. `cargo xtask` erhaelt einen Unterbefehl, z. B.
   `cargo xtask tosdump <dumper> <table.tsv> --out <dir> [--reference <BIN>…]`:
   Dumper fuer NewTOS bauen (`tosdump/build-dump*-newtos.sh`), Kernel
   `--release` fuer `rpi4`, Karte per `tools/mkcard.sh` in den Scratch
   (Klon, kein Kopieren), Lauf in QEMU `raspi4b` mit `serial=on`, BIN/LOG und
   Manifest in `<dir>`, sha256 des Abbilds nach dem Lauf, optional tosdiff
   gegen die genannten Referenzen.
2. Scratch nur in `$CLAUDE_JOB_DIR/tmp` bzw. einem uebergebenen Verzeichnis,
   nie `/tmp`; Zeitgrenze fuer den Lauf, Abbruch mit klarer Meldung.
3. Was die Audits vom 2026-10-28 an den Einzelskripten bemaengelt haben
   (`docs/audits/2026-10-28-bios-core.md` und
   `docs/audits/2026-10-28-xbios-chardev.md`, je Abschnitt 11): der Befehl
   nimmt den zu messenden Baum als Parameter (gemessen wird oft in einem
   eingefrorenen Arbeitsbaum, berichtet im Hauptbaum) und baut dort mit
   eigenem Target-Verzeichnis, damit kopierte cargo/xtask-Artefakte mit
   absoluten Pfaden nicht den falschen Baum bauen; Kernel-sha256 in der
   Ausgabe. Relative Pfade fuer Dumper und Tabelle funktionieren. Ein Panic-
   oder Absturzbildschirm beendet den Lauf mit eigener Meldung statt mit der
   Zeitgrenze.
4. Varianten: Plattenrunde (`MKCARD_TESTDISK`, Vergleich der Testpartition
   wie `measurements/measurement-2026-10-28-tool07/method/compare-testpart.sh`)
   und Eingabepruefung (`--features bios-arg-trace`, danach
   `tools/biostrace-check.py`, TOOL08).
5. Vorschlag fuer den Wortlaut von Pre-flight 2 im Bericht (der Lead traegt
   ein).

## Abnahme

- `bios-r1.tsv` mit DUMPBIOS reproduziert die 42/46 abweichenden Faelle des
  Laufs `measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT`
  auf demselben Commit (bzw. erklaert jede Abweichung).
- Hosttests gruen (`cargo xtask test`); doc-check der geaenderten Doku 0
  Fehler.

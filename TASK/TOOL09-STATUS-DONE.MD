# TOOL09 — Quittung: `cargo xtask tosdump`

Datum 2026-10-28 · Beleg
[measurements/measurement-2026-10-28-tool09/](../measurements/measurement-2026-10-28-tool09/RUNINFO.TXT)
· Ticket [TOOL09](TOOL09.md)

- `cargo xtask tosdump <dumper> <table.tsv> --out <dir> [--reference <BIN>]…
  [--tree <dir>] [--features a,b] [--disk | --testdisk <img>] [--scratch <dir>]
  [--time-limit <s>]` (`tools/xtask/src/tosdump.rs`): baut Kernel (`rpi4`,
  release) und Dumper im Messbaum, Karte per `tools/mkcard.sh` im Scratch,
  QEMU `raspi4b` mit `serial=on`, BIN/LOG, Manifest, Kernel- und
  Abbild-sha256, ein tosdiff je Referenz. Messgeraete (mkcard, mktestdisk,
  tosdiff, biostrace-check) aus dem Baum des laufenden xtask.
- Abbruch mit eigener Meldung bei Panic, Ausnahme, Absturzbildschirm,
  Desktop ohne Dumper, QEMU-Ende und Zeitgrenze (Vorgabe 480 s). `/tmp`
  als Scratch abgelehnt.
- Abweichung vom Ticket: kein eigenes `CARGO_TARGET_DIR` — xtask,
  `kernel/embed.rs` und `programs/build.sh` schreiben `<tree>/target` fest,
  aeltere Commits wuerden sonst am Link scheitern oder still veraltete
  Programme einbinden. Stattdessen lehnt der Befehl ein aus einem anderen
  Baum kopiertes `target/` ab (dep-info-Pruefung vor und nach dem Bau).
- Abnahme: `bios-r1` auf `c3db3747` in einem Klon ausserhalb des Hauptbaums
  — Kernel, BIN und LOG bytegleich mit dem Speicher-Audit, 42/46; `bios-r1`
  mit `bios-arg-trace` Pruefer PASS; `bios-r2` mit `--disk` 10/51,
  `testpart: identical`; kopiertes `target/` abgelehnt; Zeitgrenze und
  Absturzbildschirm (XCIOFL-Sonde) erkannt.
- `cargo xtask test` 1742/0/1 (16 neue Tests, Lead-Gegenlauf gleich).
  Auditor-Pre-flight 2 und CLAUDE.md „Testing“ nennen den Befehl.

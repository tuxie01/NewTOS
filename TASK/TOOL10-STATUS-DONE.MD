# TOOL10 — Quittung: `kernel8.img` neben dem ELF statt im Wurzelverzeichnis

Datum 2026-10-28 · Beleg
[measurements/measurement-2026-10-28-tool10/](../measurements/measurement-2026-10-28-tool10/RUNINFO.TXT)
· Ticket [TOOL10](TOOL10.md)

- Einziger Schreiber war `produce_kernel8_img` in `tools/xtask/src/main.rs`
  (aus `cmd_build` und `cmd_run_rpi4`); Leser `cmd_run_rpi4` und
  `tosdump::cmd`. Sonst liest nichts in `tools/` die Wurzel-Datei.
- Neuer Pfad `target/aarch64-unknown-none-softfloat/<debug|release>/kernel8.img`,
  letzte Zeile des Baus nennt ihn. `cargo xtask tosdump --tree` nimmt fuer
  aeltere Commits (Wurzel) wie fuer neue (target) nur das Abbild, das der Bau
  gerade geschrieben hat; eine alte Wurzel-Datei wird nie gemessen.
- Messlaeufe bleiben unveraendert: 125 Laeufe bauen und nehmen
  `<tree>/kernel8.img` (gueltig fuer ihre Commits), 3 Dateien nennen den
  absoluten Wurzelpfad; Liste im Lauf.
- Beide Boards, debug und release, bauen; `cargo xtask run --board rpi4`
  bis zum Desktop (ohne Karte und mit Karte); `cargo xtask tosdump` mit
  `bios-r1` vollstaendig, 42/46.
- `cargo xtask test` 1745/0/1 (3 neue Tests, Lead-Gegenlauf gleich); clippy
  xtask unveraendert; doc-check der geaenderten Doku 0 Fehler.
- Offen beim Anwender: die alte Wurzel-`kernel8.img` (gitignored) loeschen?
  Sieben alte Freigabe-Eintraege in `.claude/settings.local.json` nennen
  den Wurzelpfad.

# TOOL04 — Quittung: Testlaufzeit, schnelle und volle Stufe

Datum 2026-10-28 · Commits `6c943310` (Stufen), `75527af8` (Testprofil,
deps-Waechter), `366f2da4` (CLAUDE.md, Rollendateien) · Belege
[measurements/measurement-2026-10-27-tool04/](../measurements/measurement-2026-10-27-tool04/RUNINFO.TXT),
[measurements/measurement-2026-10-28-tool04/](../measurements/measurement-2026-10-28-tool04/RUNINFO.TXT),
[measurements/measurement-2026-10-28-tool04-profile/](../measurements/measurement-2026-10-28-tool04-profile/RUNINFO.TXT)
· Ticket [TOOL04](TOOL04.md)

- Ursache der ~2,5 h waren zwei Host-Ursachen, nicht die Tests (alle 162
  Binaries laufen in 3,8 s, langsamster Test 0,51 s):
  1. Erststart-Pruefung (Gatekeeper/XProtect) jedes neu gelinkten
     Testbinarys, 12–80 s je Binary, 6472 s gesamt. Weg mit der
     Entwicklertools-Freigabe fuer Terminal (Anwender 2026-10-28): 1,6 s.
  2. `target/debug/deps` mit ueber 500 000 Eintraegen; jeder rustc-Start
     liest es (`sample`: `SearchPath::new`). `target/debug` geloescht
     (Anwender), `[profile.test] split-debuginfo = "packed"`: ueber 7
     Neubindungen flach bei 3996 Eintraegen, ~14 s je Lauf nach einer
     tos-api-Aenderung statt ~200 s.
- `cargo xtask test` (voll) und `--fast` (ohne Doctests, nennt das
  Ausgelassene); jeder Test in genau einer Stufe, Unit-Tests pruefen das.
  Beide gruen, 1726/0/1 bzw. 1726/0/0; kein Testname verloren.
- `cargo xtask test` warnt ab 20 000 Eintraegen in `target/debug/deps`.
- CLAUDE.md „Testing“, `developer.md`, `tester.md`: voller Lauf nach jeder
  Aenderung, `--fast` nur fuer Zwischenschritte; Hinweis auf die
  Entwicklertools-Freigabe (nur Terminal; keine naechtlichen launchd-Laeufe,
  bis VS Code/ssh/launchd abgedeckt sind).
- Integrationstests nicht zusammengelegt: 85 von 159 Zielen installieren den
  prozessglobalen Host, 85 nutzen datei-lokale Sperren ueber globalem
  Kernzustand; ein flaches deps-Verzeichnis bringt dasselbe billiger.

Offen fuer die Triage (nicht Teil der Abnahme):
- `DOCTESTS_WHY` in `tools/xtask/src/test_tiers.rs` nennt noch „16 von
  18,7 s“; ohne Ballast kosten die Doctests ~0,2 s, die schnelle Stufe spart
  praktisch nichts mehr.
- Das dev-Profil (xtask selbst) laesst seine `.o` weiter im deps-Verzeichnis; waechst
  nur bei xtask-Aenderungen, der Waechter faengt es.
- Spotlight auf `target/` nicht gemessen; Entwicklertools-Freigabe fuer
  VS Code, ssh, launchd offen.

# TOOL29 — doc-check als git-pre-commit-Hook

Status: **ERLEDIGT** 2026-10-30 ([Quittung](TOOL29-STATUS-DONE.MD)) · Prioritaet: **P2** (klein) · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-30, direkt nach [A126](A126.md)

## Sachverhalt

- `CLAUDE.md` verlangt `cargo xtask doc-check` vor dem Commit. Am 2026-10-30
  ging ein Commit trotz doc-check-Fehler durch, weil die Befehlskette den
  Fehlercode nicht weitergab (`abc6276a`, berichtigt in `ebda3d72`).

## Arbeit

1. Eingechecktes Hook-Skript (pre-commit), das `cargo xtask doc-check` fuer die
   gestagten Doku-Dateien laeuft (mit der Pruefung "Anker auf neuen Zeilen"
   gegen `HEAD`) und den Commit bei einem Fehler ablehnt — unabhaengig davon,
   wie der Commit-Befehl verkettet ist.
2. Installationsschritt in der Setup-Doku bzw. als xtask-Befehl.
3. Laufzeit klein halten (nur gestagte Dateien).

## Abnahme

- Ein Commit mit einem kaputten Zeilenverweis wird abgelehnt, ein sauberer
  Commit geht durch (beides vorgefuehrt, im Ticket festgehalten).
- Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.

## Erweiterung (Anwenderkorrektur 2026-10-30, ueber den Lead)

- Auch ein Commit, der Code stagt, prueft die Doku, die ihn zitiert: ein
  Zitat, das bei `HEAD` aufloest und im Index nicht mehr, lehnt den Commit
  ab; der alte Rueckstand dieser Doku zaehlt nicht. Laufzeit beider
  Varianten (vorgefiltert / voll) gemessen.
- Fall 5 der Abnahme: eine gestagte Code-Aenderung verschiebt Zeilen unter
  einem verankerten Zitat in einer nicht gestagten Doku → abgelehnt.
- Die Sperre von `--no-verify` fuer Agenten setzt der Lead im
  PreToolUse-Hook um (`tools/hooks/no-verify-guard.sh`), nicht dieses Ticket.

## Nachweis

Lauf: `measurements/measurement-2026-10-30-tool29/RUNINFO.TXT`, zweiter Lauf
nach der Lead-Durchsicht (Wegwerf-Klon im Scratch, Hook dort per
`cargo xtask install-hooks`; das Hauptrepo unberuehrt).

- **Umsetzung:** `tools/git-hooks/pre-commit` ruft
  `cargo -q xtask doc-check --staged`. `--staged`
  (`tools/xtask/src/doc_check.rs:1564@check`) liest den Index statt des
  Arbeitsbaums, wo sie sich unterscheiden (`Resolver::use_tree`, auch fuer
  Zitatziele; `git commit -a`/`git commit <pfad>` ueber `GIT_INDEX_FILE`).
  Es zaehlen nur Fehler, die der Commit einfuehrt (Anwender 2026-10-30,
  [SPEC03](SPEC03.md)): in einer gestagten Doku (`is_commit_doc`:
  `CLAUDE.md`, `docs/**/*.md` ohne `docs/archive/` und datierte
  Schnappschuesse, `TASK/*.md`/`*.MD`, `RUNINFO.TXT`) ein Fehler auf einer
  neuen Zeile oder einer unveraenderten, deren Zitat bei `HEAD` aufloest;
  in jeder anderen Doku ein Zitat auf einen geaenderten Pfad, das bei
  `HEAD` aufloest und im Index nicht
  (`tools/xtask/src/doc_check.rs:1674@introduced`). Altlasten werden
  gezaehlt, blockieren nie. Installation: `cargo xtask install-hooks`
  (`tools/xtask/src/git_hooks.rs:67@cmd`).
- **Kaputter Zeilenverweis abgelehnt** (Fall 1, 0,78 s):
  `TASK/TOOL29.md:28: tools/xtask/src/main.rs:99999@main: line 99999 beyond end (1274 lines)`,
  `git commit` exit 1, `HEAD` unveraendert.
- **Verkettet weiterhin abgelehnt** (Fall 3, 0,89 s):
  `cargo -q xtask doc-check TASK/TOOL29.md | tail -1 && git commit …` —
  die Kette laeuft bis `git commit`, der Hook lehnt ab, Kette exit 1.
- **Sauberer Commit geht durch** (Fall 2, 0,78 s), exit 0.
- **Commit ohne Doku** (Fall 4a, `README.md`, 0,08 s, kein cargo-Lauf) und
  **Code ohne Zeilenverschiebung** (Fall 4b, 0,56 s) gehen durch.
- **Code verschiebt Zeilen unter Ankern nicht gestagter Doku** (Fall 5,
  0,57 s): 20 Zeilen in `tos-api/src/bios/chardev.rs`, 63 Zitate gemeldet
  („resolves at HEAD; this commit breaks it“), exit 1.
- **Teil-Staging:** gestagte Kopie kaputt, Arbeitskopie sauber → abgelehnt
  (6a, 0,85 s); umgekehrt → durch (6b, 0,78 s); `git commit -a` mit
  kaputter Arbeitskopie → abgelehnt (6c, 0,77 s).
- **Altlast blockiert nicht** (Fall 7, 0,80 s): `TASK/X03.md` mit zwei
  Fehlern schon bei `HEAD` plus eine saubere neue Zeile → exit 0
  („0 failures, 2 legacy failures ignored“).
- **Neu kaputtes Zitat in derselben Doku** (Fall 8, 0,77 s) → exit 1
  („1 failures, 2 legacy failures ignored“).
- **Varianten fuer Code-Commits:** vorgefiltert 0,56–0,57 s, voll (jedes
  Zitat jeder nicht gestagten Doku) 1,70–1,80 s, gleicher Befund; gewaehlt
  die vorgefilterte.
- **Hosttests:** `cargo xtask test` 1905 / 0 / 1 (vorher 1899 / 0 / 1, +6).

## Anwenderentscheidungen waehrend der Umsetzung (2026-10-30)

- Code-Commits pruefen auch die nicht gestagten Docs, die die geaenderten
  Dateien zitieren; Fall 5 (Code verschiebt Zeilen, zitierendes Doc nicht
  gestagt) wird abgelehnt.
- Der Hook lehnt nur Fehler ab, die der Commit einfuehrt; die Altlast
  ([SPEC03](SPEC03.md)) blockiert nie.
- Agenten duerfen den Hook nicht umgehen: `tools/hooks/no-verify-guard.sh`
  (PreToolUse) sperrt das Umgehungs-Flag, `commit -n`, `-c core.hooksPath=`
  und jeden `core.hooksPath` ausser `tools/git-hooks`; der Anwender kann von
  Hand umgehen.


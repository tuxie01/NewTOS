# TOOL29 — Quittung

Erledigt 2026-10-30. Eingecheckter pre-commit-Hook `tools/git-hooks/pre-commit`
ruft `cargo xtask doc-check --staged`: prueft den gestagten Stand, bei
Code-Commits auch die zitierenden Docs, und lehnt nur Fehler ab, die der
Commit einfuehrt. Einbau: `cargo xtask install-hooks`
(`core.hooksPath=tools/git-hooks`). Nachweis: 11 Faelle im Wegwerf-Klon
(`TASK/TOOL29.md` "Nachweis",
`measurements/measurement-2026-10-30-tool29/RUNINFO.TXT`), ca. 0,6–0,9 s je
Commit. Hosttests 1905/0/1.

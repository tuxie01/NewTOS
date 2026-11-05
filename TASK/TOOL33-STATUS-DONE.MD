# TOOL33 — Quittung

Erledigt 2026-11-05. Jede EmuTOS-`aes/`-Funktion (418) hat eine Zeile in
`docs/analysis/deps/mapping.tsv`: 365 mapped, 9 design, 44 unmapped mit Begruendung.
`MAPPED_DIRS` enthaelt `aes/`; `cargo xtask deps --check-mapping` sauber, Gegenprobe
mit fehlender Zeile rot, `cargo xtask test` 1974/0/1.

Belege: Ticket [TOOL33](TOOL33.md) (Stand Teil 1–4, 47 Abweichungs-Kandidaten, nicht
gemessen), Commits `9ef69b0f`, `f8ada924`, `96af29a5` und der Commit dieser Quittung.
Offen und nicht Teil dieses Tickets: Messung der Kandidaten gegen das Orakel; `desk/`
und `include/` bleiben ohne Pflichtzeilen.

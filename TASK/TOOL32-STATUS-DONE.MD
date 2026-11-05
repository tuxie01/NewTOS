# TOOL32 — Quittung

Erledigt 2026-11-05. Jede EmuTOS-`vdi/`-Funktion (250) hat eine Zeile in
`docs/analysis/deps/mapping.tsv`: 204 mapped, 37 design, 9 unmapped mit Begruendung.
`MAPPED_DIRS` enthaelt `vdi/`; `cargo xtask deps --check-mapping` sauber, Gegenprobe
mit fehlender Zeile rot, `cargo xtask test` 1974/0/1.

Belege: Ticket [TOOL32](TOOL32.md) (Stand Teil 1–3, 26 Abweichungs-Kandidaten, nicht
gemessen), Commits `bcfc4ed2` (Teil 1), `7ad19d3d` (Teil 2) und der Commit dieser Quittung.
Offen und nicht Teil dieses Tickets: Messung der Kandidaten gegen das Orakel.

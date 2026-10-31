# BIOS12 — Quittung

Erledigt 2026-10-31 (Anwender). Die Specs `docs/api/bios/05-setexc.md` und
`docs/api/bios/0b-kbshift.md` sind seit 2026-10-28 berichtigt; die damals
offenen Entscheidungen sind gefallen und umgesetzt:

- `Setexc` D1–D3: entschieden 2026-10-28 (`TASK/BIOS07.md`, Abschnitt
  "Entscheidungen des Anwenders 2026-10-28"), umgesetzt mit
  [BIOS07](BIOS07-STATUS-DONE.MD); Spec `05-setexc.md` "Pi binding" und
  "Decisions".
- `Kbshift` D1: entschieden 2026-10-28 (`TASK/BIOS10.md`, Abschnitt
  "Entscheidung des Anwenders 2026-10-28"), umgesetzt mit
  [BIOS10](BIOS10-STATUS-DONE.MD); Spec `0b-kbshift.md`.

Die Abnahme ("kein Eintrag widerspricht `bios-core-r1` 46–62 oder Sonde
K01–K13") stuetzt sich auf die Messlaeufe von BIOS07 und BIOS10.

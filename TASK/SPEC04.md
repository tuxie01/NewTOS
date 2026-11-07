# SPEC04 — Nacharbeit aus V18 Teil 3: COMPATIBILITY, Mapping, Graph, Spec

Status: **OFFEN** (2026-11-07) · Prioritaet: **P4** · Typ: Researcher (nur Doku) — aus [V18](V18.md) Teil 3 (VDI-WK-005…008, K23)

Aus `docs/audits/2026-11-07-v18-t3-vdi-workstation.md` (§8, Nebenbefunde):
- `docs/COMPATIBILITY.md`: K23 (`v_opnwk` mit Moduswunsch `work_in[0]` = Modus + 2 schaltet nicht, ADR-017 Rev. 2) und `v_opnwk` bei offener physischer Workstation (EmuTOS haengt, NewTOS kehrt zurueck; VDI-WK-005).
- `docs/analysis/deps/mapping.tsv`: Text `open_physical` ("virtual slots cleared" stimmt nicht), `build_vwk_chain` -> `design:ADR-030`.
- Abhaengigkeitsgraph: Kanten `vblqueue`, `mousevec` fehlen, Asm-Label `scrn_clip` faelschlich `GSX_ENTRY` zugeordnet (VDI-WK-007).
- Spec: VBL-Weg in `docs/api/vdi/` (`_mouse.md`), Abschnitt `v_clswk` (VDI-WK-008).
- VDI-WK-006 (PLAUSIBLE): AUTO-Programm, das eine virtuelle Workstation offen laesst — Messfall entwerfen.

[Gemeinsame Regeln](README.md) gelten.

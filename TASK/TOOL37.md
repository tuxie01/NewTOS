# TOOL37 — Abhaengigkeitsgraph: `scrn_clip`, `mousevec`, `vblqueue` richtig erfassen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: tool ticket, blocks no fix acceptance, behind the VDI work (2026-11-08) | Reopen: a fix acceptance is blocked by it

Migrated (owner batch 12, 2026-11-11), evidence: TASK/TOOL37.md status line: GEPARKT 2026-11-08 (Werkzeug-Ticket, blockiert keine Fix-Abnahme, hinter die VDI-Arbeit)

Line 3 before the migration:
Status: **GEPARKT** 2026-11-08 (Werkzeug-Ticket, blockiert keine Fix-Abnahme; hinter die VDI-Arbeit) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — aus [SPEC04](SPEC04.md) (VDI-WK-007)

Drei Generator-Defekte in `tools/deps/gen-deps.py`, mit Zeilen und Behebungsweg beschrieben in [SPEC04](SPEC04.md) "Generator-Defekte": (A) das lokale Asm-Label `scrn_clip` wird `GSX_ENTRY` zugeschlagen, daraus die falsche Kante `mouse_int -> GSX_ENTRY`; (B) `Initmous` speichert `mouse_int` ueber einen Parameter nach `mousevec`, die Installation fehlt; (C) `vblqueue[0] = vb_draw` und der Aufruf in `_int_vbl` fehlen. Folge: `--up`-Mengen fuer Maus- und VBL-Funktionen sind unvollstaendig. Abnahme: die drei Kanten im Graphen, `--check-mapping` sauber, neue mapping-Zeile fuer `scrn_clip`.

[Gemeinsame Regeln](README.md) gelten.

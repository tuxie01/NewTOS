# DESK67 — Reste aus DESK48: Teilneuzeichnen und `fun_rebld` vollstaendig wie EmuTOS

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 10, 2026-11-11), evidence: TASK/DESK67.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Konformitaet / Laufzeit — Rest aus [DESK48](DESK48-STATUS-DONE.MD) (2026-10-17)

Bei DESK48 bekannt und bewusst nicht angefasst:

1. `search_current_window` und `set_window_mask` rufen weiter
   `redraw_desktop()`; EmuTOS zeichnet nur das betroffene Fenster.
2. `act_bsclick` und das Gummiband zeichnen das ganze Fenster statt nur der
   geaenderten Objekte (EmuTOS `act_allchg`/`act_chg`, `desk/deskact.c`).
3. `fun_rebld`/`fun_mark_for_rebld` vergleichen nur das Verzeichnis, nicht das
   ganze `p_spec` (NewTOS haelt die Maske in `p_mask`): ein maskiertes Fenster
   auf demselben Verzeichnis wird mit aufgefrischt.
4. `fun_rebld_marked` laeuft in Slot- statt Z-Ordnung.
5. `do_new_folder` ruft `rebuild_window(cwin)` statt `fun_rebld`.

Je Punkt: EmuTOS-Funktion mit Datei:Zeile lesen, gegenueberstellen,
angleichen; Beleg per Screendump/AES-Spur wie in
`measurements/measurement-2026-10-17-desk48/`.

Nicht Teil davon: die benannte Abweichung „alle gezogenen Desktopsymbole
verarbeiten“ (Anwender 2026-10-17, `docs/COMPATIBILITY.md`).

[Gemeinsame Regeln](README.md) gelten.

# DESK48 — Vier weitere `redraw_desktop()`-Stellen, die EmuTOS nicht hat

Status: DONE (2026-11-11) | Priority: P2 | Commit: 70fb5dc5b | Receipt: DESK48-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/DESK48-STATUS-DONE.MD exists; commit 70fb5dc5b; TASK/DESK48-STATUS-DONE.MD, commit 70fb5dc5b (fix(desktop): replace redraw_desktop() after file ops with EmuTOS repa), fix/feat commits naming the id: - (n=0)

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-17 (QEMU, `f2f127a7`; Pi abgenommen 2026-10-17, `docs/HW-TEST-LOG.md`) — [Quittung](DESK48-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: Konformitaet / Laufzeit — aus [GUI06](GUI06.md), bewusst zurueckgestellt

Abhaengigkeiten: — · ~~nicht vor dem OFAM-Termin anfassen~~ (Termin 9./10. Oktober vorbei)

## Warum zurueckgestellt und nicht erledigt

GUI06 hat den gemessenen Fall behoben: das Ziehen-Kopieren loest kein
Vollbild-Neuzeichnen mehr aus. Vier weitere Stellen rufen es weiterhin, und
**eine davon liegt auf dem Klickpfad jedes Fensterwechsels** — elf Tage vor der
Vorfuehrung der falsche Ort fuer eine Aenderung, die niemand erzwungen hat.

## Die vier

1. **`fun_desk2desk` (`dir.rs:2249`)** behaelt sein `redraw_desktop()`. EmuTOS
   hat dort keines, aber sein Ersatz ist `desk_clear(DESKWH)` in
   `fun_file2any` (`deskfun.c:1187`) — und **NewTOS' `desk_clear`
   (`desk/main.rs:2208`) fehlt die Zeichenhaelfte** von EmuTOS'
   (`desksupp.c:143` → `act_allchg(…, FALSE)` → `do_wredraw` ueber die
   Vereinigung der geaenderten Symbolrechtecke). Ohne die bliebe die
   Symbolauswahl sichtbar stehen. **Die Reihenfolge ist also: erst die
   Zeichenhaelfte von `desk_clear`, dann das `redraw_desktop()` weg** — Ersatz
   vor Entfernung, wie in der ganzen DESK43-Reihe.
2. **`desk_clear_all` (`main.rs:3223`)** traegt denselben Ersatz
   (`win_shwall()` + `do_wredraw(DESKWH, …)`) und laeuft bei **jedem**
   Fensterwechsel. Faellt mit (1) zusammen weg.
3. **`fun_selectall` (`dir.rs:1598`)** ruft `redraw_desktop()`, obwohl sein
   **eigener Doc-Kommentar** „sendet ein einfaches `WM_REDRAW`" sagt
   (`deskfun.c:607-631`). Dasselbe Muster wie das, was GUI06 gefunden hat.
4. **Ein fehlender Rebuild, keine ueberzaehlige Zeichnung:** NewTOS'
   `fun_win2desk`/`fun_win2win` rufen `rebuild_window(handle)` direkt, EmuTOS
   `fun_rebld(p_spec)` — das baut **jedes** Fenster mit diesem Pfad neu auf.
   Bei zwei Fenstern auf demselben Verzeichnis zeichnet NewTOS **eines zu
   wenig**. Ebenfalls fehlt `if (datype == AT_ISFOLD) fun_rebld(destpath);`
   (`deskfun.c:997-998`).

## Abnahme

Je Stelle: EmuTOS' Ersatz gebaut, dann das `redraw_desktop()` entfernt, und ein
Screendump-Vergleich, der zeigt, dass nichts stehenbleibt, was verschwinden
sollte. Fuer (4): zwei Fenster auf demselben Verzeichnis, beide aktualisieren
sich.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

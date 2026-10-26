# Offene Tasks — Desktop 1:1 EmuTOS Rebuild & AES Render-Kampagne

> Stand: 2026-07-22, Branch `desk-rebuild`, HEAD `db6052d`.
> Ersetzt die (getrennten) MCP-Task-Tools — die sind disconnectet, Task-Nummern
> sind nur noch historische Referenzen. Diese Datei ist der Tracker.

Leitprinzip: **1:1 EmuTOS.** Bei jeder Abweichung ist der Default „match it"
(EmuTOS `file:line` zitieren + reproduzieren), nicht „benign, lassen".
Siehe Memory `no-rationalizing-emutos-deviations`, ADR-022 in `docs/MEMORY.md`.

Workflow pro Problem: Developer implementiert → **unabhängiger** Tester verifiziert
(nicht dem Dev-Selbstreport trauen) → Lead committet bei PASS → nächstes. Seriell.
Keine Stubs. Worker nehmen keine HW-/Outward-Actions (Flash/Eject = Lead/User).
Root-Cause mit Evidenz VOR dem Fix.

---

## A. AKTIV / als Nächstes

### A0 — TeraDesk verliert seinen Zustand, wenn ein gestartetes Programm zurückkehrt — gemeldet 2026-09-09, nicht untersucht

**Funktion und Umfang:** der Rückweg aus einem aus TeraDesk gestarteten Programm.
Betrifft `Pexec` (GEMDOS 0x4B) und/oder `shel_write` (AES 121) plus die
Shell-Schleife, die TeraDesk per `#Z` startet.

**Beleg:** Beobachtung des Users auf realer Pi 4, Wortlaut: *„wenn man etwas aus
teradesk heraus startet und man kehrt wieder zurück dann ist es so als ob der
teradesk neu gestartet worden ist, das sollte eigentlich nicht so sein."*
Keine Messung, kein Trace, kein Screendump — **eine Beobachtung, sonst nichts.**

**Was TeraDesks Quelle dazu sagt** (gelesen, nicht vermutet). `startprg.c:583`
entscheidet selbst, welchen Weg es nimmt:

```c
stask = (single && magx) || (_AESnumapps != -1 && !magx);
...
if (stask || background)   /* :632 */
    error = x_exec(0, fname, &cl, buildenv);   /* just Pexec, :660-662 */
else                       /* :682 "AES is multitasking, use shel_write()" */
    error = shel_write(mode, appl_type, wiscr, p, cmdl);   /* :742 */
```

`_AESnumapps` ist `aes_global[1]` = `ap_count` (`sdk/gemlib/gemlib-0.44.0/gem.h:34`).
Unser AES meldet `ap_count 1` (gemessen, `tosdump/reference/NEWTOS-AES-r2.LOG`),
und ohne MagiC ist `magx` falsch — also ist `stask` bei uns **TRUE** und TeraDesk
nimmt den **Pexec-Weg**. Auf diesem Weg bleibt TeraDesk im Speicher und muss nach
der Rückkehr genau dort weitermachen, wo es war: offene Fenster, Verzeichnisse,
Auswahl, Positionen.

**Einordnung: noch ungeklärt.** Zwei Hypothesen, beide ungeprüft, keine bevorzugt:

1. Der Pexec-Rückweg setzt den Elternprozess nicht fort, sondern lädt ihn neu.
   Dann liegt es in `tos-api/src/bdos/process/` (Pexec/Pterm-Eltern-Resume).
2. Die Shell-Schleife, die TeraDesk per `#Z` gestartet hat, reagiert auf die
   Terminierung des **Kindes** so, wie sie auf die Terminierung von TeraDesk
   reagieren müsste, und startet TeraDesk neu. Dann liegt es in der Shell-Schleife,
   nicht im Prozessmodell.

Ein dritter Fall ist erst auszuschließen, bevor überhaupt von einem Fehler geredet
wird: dass TeraDesk seinen Zustand von sich aus verwirft und aus `DESKTOP.INF`
neu aufbaut. Dann sähe „neu gestartet" genauso aus und wäre keine Abweichung.

**Arbeit:**
1. Feststellen, welcher der beiden Startwege tatsächlich genommen wird — `stask`
   ist aus der Quelle hergeleitet, nicht beobachtet. Ein GEMDOS-/AES-Trace über
   den Startvorgang zeigt es: `Pexec` oder `shel_write`.
2. Wenn Pexec: prüfen, ob TeraDesks Basepage nach der Rückkehr dieselbe ist —
   dann lebt der Prozess weiter und der Zustandsverlust ist TeraDesk-intern; ist
   sie eine andere, wurde neu geladen.
3. Referenz dagegenhalten: was ein echtes TOS an dieser Stelle tut, und was
   EmuTOS' `sh_ldapp`/`Pexec`-Paar dem Aufrufer garantiert.
4. Erst danach reparieren, in der Schicht, die der Befund benennt.

**Abnahme:** aus TeraDesk ein Programm starten, das sichtbar etwas tut, beenden,
und prüfen, dass danach dieselben Fenster mit denselben Verzeichnissen, derselben
Auswahl und denselben Positionen stehen wie vorher. Dazu ein Regressionstest auf
der Ebene, die der Befund trifft — bei (1) ein Selftest, der über `Pexec` ein
Kind startet und nach dessen `Pterm` die Basepage-Adresse und einen Speicherwert
des Elternprozesses wiederfindet; bei (2) ein Test der Shell-Schleife, der zeigt,
dass sie auf ein Kind-`Pterm` nicht mit einem Neustart der Anwendung antwortet.

**Bezug:** Memory `start-teradesk-via-real-emutos-mechanism`,
`teradesk-is-the-desktop`, `singletasking-app-owns-full-screen`.


### A1 — #127 Queue-Unification (Bypass raus) — NICHT gestartet, User-greenlit
**Problem:** NewTOS hat zwei Message-Pfade. WM_REDRAW geht über die Queue
(`post_msg` → per-app FIFO), aber **WM_MOVED / WM_TOPPED / WM_SIZED / Menü**
schreiben über `window::intercept` / `emit` **direkt in `pbuff`**
(`tos-api/src/aes/window.rs:1761` / `:1834`) und umgehen die Queue.
→ Nicht 1:1; drop-anfällig, sobald eine Geste mehrere Messages erzeugt.

**EmuTOS-Referenz:** ALLE Inter-Prozess-Messages laufen über **einen** Pfad:
`ap_sendmsg` (`gemwmlib.c:851-863`) → `ap_rdwr(MU_SDMSG)` → `aqueue`/`doq`
→ per-app FIFO `p_queue[QUEUE_SIZE=128]` (`struct.h`), gelesen von `ev_mesag`
→ `ap_rdwr(MU_MESAG)` → dequeue (`gemevlib.c:100-116`). WM_REDRAW-Rect-Union
beim Enqueue (`gemqueue.c:45-56`, hier lokal gelesen). **Kein** Message umgeht die Queue.

**Aufgabe:** Alle AES-generierten Messages über `post_msg` / ap_sendmsg-Stil
routen, den `intercept→pbuff`-Bypass entfernen, sodass `evnt_multi` alles aus
der Queue liefert. Prime-Suspect für die verbleibenden Redraw-Probleme.
**Depends on:** appl_init (erledigt, `1579655` + `5b23601`).

### A2 — Verbleibende Redraw-Probleme diagnostizieren — offen
appl_init hat geholfen („etwas besser"), aber User meldet weiterhin „einiges an
Redraw-Problemen" auf realer HW. **Noch nicht root-caused.** Braucht Präzisierung
vom User: Move? Open? Menü? überlappende Fenster? Maus-Spuren überall/selektiv?
A1 (#127) ist der Hauptverdacht für die Multi-Message-Drop-Klasse, aber nicht
annehmen — erst Symptome einsammeln, dann gegen EmuTOS prüfen. Ggf. deckt A1 einen
Teil ab; der Rest (v. a. A3) ist separat.

### A3 — Maus-Artefakte auf dem Desktop — flagged PLAUSIBLE, nicht untersucht
Cursor-Save-Under vs. direkte Render-Pfade: beim Redraw bleibt der alte
Maus-Cursor als Artefakt stehen. Vom User zweimal gemeldet
(„es gibt Artefakte auf dem Desktop von der Maus"). Separat von A1.
Zu untersuchen: wie EmuTOS den Cursor beim Redraw hidet/restauriert
(`gsx_mcursor` / mouse hide-show um `w_redraw`).

---

## B. Desktop 1:1 Rebuild — restliche Module (#124)

Module 1 (do_wredraw 1:1, `14c28f0`) ist erledigt. Offen:

- **B1 — Modul 2 (Dialoge → App-Pfad):** Desktop-Dialoge über den App-/AES-Pfad
  statt direktem EL1-`objc_draw`; die Modul-2-`redraw_desktop`-Reliance entfernen.
- **B2 — Modul 3 (menu/window/event 1:1):** Menü-/Fenster-/Event-Handling der
  desk/ 1:1 gegen `deskmain.c` / EmuDesk. Überschneidet sich mit A1.
- **B3 — Modul 4 (file/dir/INF/rez 1:1):** File-/Dir-Ops, `.INF`-Handling,
  Resource-Load 1:1; `desksupp.c` `do_aopen`/`do_open` in eine neue
  `docs/api/desktop/desksupp.md` extrahieren.
- **Endgame:** desk/ als ersetzbare **EL0-AES-App** (späte Stufe) — der Grund für
  den ganzen Rebuild (AUTO-Ordner-Desktop darf nicht brechen).

---

## C. Fix B — AES OBJECT/FORM Render (3D-Look), deferred Rest

Groups A/B/C committet (D1/D9, D3/D4/D5, D7). Deferred:
- **C1 — D2:** SMALL 6x6-Font. Deferred **zusammen mit** UI-Scaling (E3), weil
  6x6 bei 1080p Legibility-Frage ist.
- **C2 — D6:** G_USERDEF-Objekte.
- **C3 — D8:** INDIRECT-Objekte (`ob_flags & INDIRECT`).
- Audit-Doc `docs/analysis/aes-object-form-render-audit.md` ist noch **uncommitted**
  (siehe F).

---

## D. QED / cflib App-Pfad

- **D1 — CLIPBRD „cannot create":** QED konnte `CLIPBRD` nicht anlegen obwohl der
  Ordner existiert. Ursache lag im Fattrib-TOS4-vs-TOS1-3-Fork (cflib
  `path_exists`/`fsexists.c:41` erwartet FA_DIR auf Ordnern = TOS4). Fix C
  (Fattrib TOS4-Superset, `8af8c21`) committet. **Auf realer HW re-verifizieren**,
  ob CLIPBRD-Erstellung jetzt durchgeht. Memory `cflib-apps-expect-tos4-semantics`.

---

## E. Geparkt (minor 1:1 Follow-ups + Scaling)

- **E1 — #120:** FL3DOBJ text/box `th` (Feinheit im 3D-Border).
- **E2 — #121:** SHADOWED hittest + Spec.
- **E3 — #122:** Globales UI-Scaling (der „eigentliche Fix" für 1080p-Legibility).
  Vom User bewusst gewählt, dann deferred („jetzt aber nicht notwendig"). Schaltet
  C1 (D2 6x6) frei.
- **E4 — #126:** chkdisabled press-edge / popup.

---

## F. Housekeeping / Working-Tree

**Erledigt (seit `5b23601` committet):**
- `e330d71` Agent-Definitionen (auditor/developer/researcher + neuer tester).
- `65fee47` Fix-B-Audit `aes-object-form-render-audit.md` (gehört zu C).
- `db6052d` `sdk/apps/build-teradesk-ilp32.sh` (2. EL0-GEM-App-Validator).
- `pictures/` (68 MB reale-HW-Fotos) → **gitignored** (`.gitignore`, analog
  `sd-card-backup-*/`), bleiben lokal, nicht ins Repo.

**Offen — vor Merge nach `main` zu erledigen:**
- **F1 — Debug-Tracer revertieren:** `25fe27a` `chore(debug): TEMPORARY EL0 QED
  syscall tracer` ist bewusst als **„revert before merge"** markiert. Betrifft
  `kernel/src/dbg_qedtrace.rs` (+661 Z.), `kernel/src/arch/aarch64/exceptions.rs`
  (+16), `kernel/src/lib.rs` (+5). Vor dem Branch-Merge zurücknehmen (git revert
  oder Commit droppen). Nur QED/TeraDesk-Bring-up-Instrument, kein Produkt-Code.

---

## Erledigt (Referenz, Branch `desk-rebuild`, neueste zuerst)

- `db6052d` TeraDesk ILP32 Build-Script (2. EL0-GEM-App-Validator)
- `25fe27a` TEMPORARY EL0 QED syscall tracer — **F1: revert before merge**
- `e330d71` Worker-Rollen verfeinert + dedizierter tester-Agent
- `65fee47` Fix-B Render-Audit + dieser Tracker
- `5b23601` appl_init füllt aes_global 1:1 (ap_private, ap_3resv)
- `1579655` desk ruft appl_init (Empty-Window Root-Cause)
- `8d77372` per-app FIFO Message-Queue (#125, notwendig aber nicht hinreichend)
- `bb0b6b9` P3: kein Highlight auf DISABLED-Menüitem bei Hover
- `ef9bfa8` P5: WM_REDRAW an uncovered/topped Fenster bei wind_close/WF_TOP
- `14c28f0` Modul 1: do_wredraw 1:1
- `dd44060` Spec-Validierung + ADR-022
- `26a9b7e` P1+P2: Menü save/restore
- `d95ea59` D7 · `3404a18` D3-D5 · `b8c3a9e` D1+D9 (Fix B)
- `8af8c21` Fix C: Fattrib TOS4-Superset
- `ec054a0` Fix A: rsrc_obfix

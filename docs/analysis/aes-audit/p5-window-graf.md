# P5 — Window Manager + Graphics Manager: 1:1-Divergenz-Katalog

**Scope:** `tos-api/src/aes/window.rs` (1938 Z.) + `tos-api/src/aes/graf.rs` (483 Z.) gegen EmuTOS `aes/gemwmlib.c`, `gemwrect.c`, `gemgrlib.c`, `gemgraf.c`, `gemgsxif.c`, Dispatch `gemsuper.c`, Spec `docs/api/aes/window-graphics.md` + D-053. Rein statisch (kein Build/QEMU — 5 parallele Auditoren).

## Teil 1 — Coverage-Tabelle

### WINDOW (`gemwmlib.c` / `gemwrect.c`)

| EmuTOS fn | EmuTOS file:line | NewTOS fn | Status | 1:1? |
|-----------|------------------|-----------|--------|------|
| `wm_create` | gemwmlib.c:1369 | `wind_create` window.rs:1103 | impl | partial (owner @create nicht gesetzt) |
| `wm_open` | gemwmlib.c:1410 | `wind_open` window.rs:1122 | impl | partial (kein already-open→FALSE) |
| `wm_close` | gemwmlib.c:1438 | `wind_close` window.rs:1157 | impl | partial (not-open→TRUE statt FALSE) |
| `wm_delete` | gemwmlib.c:1466 | `wind_delete` window.rs:1189 | impl | partial (schließt offenes Fenster nicht) |
| `wm_get` | gemwmlib.c:1505 | `wind_get` window.rs:1208 | partial | nein (COLOR/DCOLOR/OWNER fehlen, SCREEN Stub, KIND weicht ab) |
| `wm_set` | gemwmlib.c:1623 | `wind_set` window.rs:1321 | partial | nein (COLOR/DCOLOR fehlen) |
| `wm_find` | gemwmlib.c:1767 | `wind_find` window.rs:1455 | impl | ja |
| `wm_update` | gemwmlib.c:1778 | `wind_update` window.rs:1461 | **STUB** | nein (No-op, Modus ignoriert) |
| `wm_calc` | gemwmlib.c:1807 | `wind_calc` window.rs:1466 (`calc`/`borders` 746/725) | impl | partial (non-3D-Dicken vs 3D-Default) |
| `wm_new` | gemwmlib.c:1875 | `wind_new` window.rs:1479 | impl | partial (Locks nicht freigegeben) |
| `w_bldactive` (3D 479 / non-3D 716) | gemwmlib.c:479/716 | `draw_frame` window.rs:819 | impl (reimpl) | reference (non-3D-Geometrie) |
| `w_bldvbar`/`w_bldhbar` | gemwmlib.c:403/441/634/675 | `draw_vslider`/`draw_hslider` window.rs:917/985 | impl (reimpl) | reference |
| `do_walk` | gemwmlib.c:286 | `compose`/`paint_bg_clip` window.rs:453/409 | impl (reimpl) | partial (**kein moff/mon**) |
| `w_drawdesk` | gemwmlib.c:340 | `paint_bg_clip` window.rs:409 | impl | ja (reference) |
| `w_update` (moff 1011 / mon 1047) | gemwmlib.c:1004 | `broadcast_redraw`/`compose` window.rs:703/453 | impl (reimpl) | partial (**kein moff/mon**, RCAP-Cap) |
| `w_redraw` | gemwmlib.c:887 | `send_redraw`/`send_redraw_in` window.rs:628/685 | impl | ja |
| `w_union` | gemwmlib.c:869 | `visible_bound`/`union_grect` window.rs:658/498 | impl | ja |
| `draw_change` | gemwmlib.c:1058 | `redraw_clip`+`send_redraw` (in open/close/set) | impl (reimpl) | partial |
| `newrect`/`mkrect`/`brkrct` | gemwrect.c:165/136/101 | `compute_rlist`/`subtract` window.rs:583/545 | impl (reimpl) | partial (RCAP=16-Cap) |
| `w_owns` | gemwmlib.c:1230 | `rlist_next`/`compute_rlist` window.rs:612/583 | impl | ja |
| `ap_sendmsg` | gemwmlib.c:851 | `send_redraw`/`emit` window.rs:628/1880 | impl | ja |
| WM_* emit (gemctrl, ref) | gemctrl.c:63-315 | `intercept` window.rs:1807 | impl (reimpl) | partial (UNTOPPED/HSLID/VSLID/WA_*PAGE fehlen) |
| `or_start`/`get_orect` | gemwrect.c:40/53 | (Rust-Array `RLIST`) | reimpl | reference |

### GRAF (`gemgrlib.c` / `gemgraf.c` / `gemgsxif.c`)

| EmuTOS fn | EmuTOS file:line | NewTOS fn | Status | 1:1? |
|-----------|------------------|-----------|--------|------|
| `gr_rubbox`/`gr_rubwind` | gemgrlib.c:228/199 | `graf_rubberbox` graf.rs:121 | impl | ja |
| `gr_dragbox` | gemgrlib.c:243 | `graf_dragbox` graf.rs:171 | impl | ja |
| `gr_movebox` | gemgrlib.c:287 | `graf_movebox` graf.rs:248 (`animate` 213) | impl (reimpl) | reference (Anim.) |
| `gr_growbox` | gemgrlib.c:309 | `graf_growbox` graf.rs:261 | impl (reimpl) | reference (kein Center-Zoom) |
| `gr_shrinkbox` | gemgrlib.c:326 | `graf_shrinkbox` graf.rs:270 | impl (reimpl) | reference (Anim.) |
| `gr_watchbox` | gemgrlib.c:337 | `graf_watchbox`/`watchbox_t` graf.rs:280/305 | impl | ja |
| `gr_slidebox` | gemgrlib.c:358 (return 410) | `graf_slidebox` graf.rs:328 | impl | **NEIN — Return-Slot falsch (BLOCKER)** |
| `gr_mouse` | gemgrlib.c:419 | `graf_mouse` graf.rs:420 | partial | nein (M_OFF/ON nicht zählend; M_SAVE/RESTORE/PREV No-op) |
| `gr_mkstate` | gemgrlib.c:458 | `graf_mkstate` graf.rs:476 | impl | ja |
| `graf_handle` (dispatch) | gemsuper.c:287-292 | `graf_handle` graf.rs:405 | impl | ja |
| `gsx_moff`/`gsx_mon` | gemgsxif.c:440/450 | `mouse::hide`/`show` mouse.rs:354/363 | impl | partial (Window-Composite ruft nicht) |
| `gsx_xbox` (XOR) | gemgraf.c:274 | `outline` graf.rs:44 | impl (reimpl) | reference |
| `bb_screen` | gemgraf.c:357 | `copy_rect` (in `drag_move`) window.rs:1684 | impl (reimpl) | reference |
| `gsx_mfset` | gemgsxif.c:407 | `mouse::set_form` mouse.rs:193 | impl | partial (MFORM fg/bg ignoriert) |

---

## Teil 2 — Findings

### AUD-P5-001: `graf_slidebox` — Position im falschen int_out-Slot; Return immer 1
- **Severity:** BLOCKER
- **EmuTOS:** gemgrlib.c:358-413 (return `mul_div_round(...)` :410); Dispatch gemsuper.c:284-285 `ret = gr_slidebox(...)` → `int_out[0]`
- **NewTOS:** graf.rs:398-399 (`pb.set_intout(1, pos); pb.set_ret(1);`)
- **Divergenz:** `gr_slidebox` liefert die Elevator-Position 0..1000 als **Funktions-Rückgabewert = `int_out[0]`**. NewTOS schreibt die Position nach `int_out[1]` und setzt `int_out[0]=1`. Die Standard-AES-Bindung liest `int_out[0]`, also bekommt jede App **konstant 1** (≈0,1 ‰) statt der Drag-Position. Scrollbar-Elevator-Drag liefert immer "fast ganz oben".
- **1:1-Fix:** `pb.set_ret(pos)` (Position nach `int_out[0]`); `set_intout(1, …)` entfällt.
- **Runtime-verify:** yes

### AUD-P5-002: Window-Composite/Redraw-Pfad ohne Cursor-Hide/Show → Maus-Artefakte beim Redraw
- **Severity:** HIGH
- **EmuTOS:** `w_update` klammert die **gesamte** Fenster-Neuzeichnung mit `gsx_moff()` (gemwmlib.c:1011) / `gsx_mon()` (:1047); ebenso `w_redraw_desktop` (:1923/:1942). `gsx_moff`/`gsx_mon` sind der zählende Cursor-Semaphor (gemgsxif.c:440/450).
- **NewTOS:** `compose` window.rs:453, `paint_bg_clip` :409, `draw_frame` :819, `redraw_clip` :486, `redraw_all` :442, `redraw` :1498, `redraw_open` :1445, `redraw_vslider` :969, `redraw_hslider` :1031 — **kein** `cursor_hide`/`cursor_show`. (Cursor_hide-Aufrufe in window.rs nur in `drag_move` :1683/:1692 und `drag_size` :1755/:1759.)
- **Divergenz:** Der Cursor ist ein Save-under-Sprite: `draw()` speichert die Pixel unter dem Sprite (mouse.rs:238), `erase()` stellt sie später wieder her (:264). `wind_open`/`wind_close`/`wind_set(WF_CXYWH/WF_TOP)`/`wind_new`/`redraw_clip` zeichnen via `compose` **über** die gerade-sichtbaren Cursor-Pixel, ohne den Sprite vorher zu verstecken → der Save-under wird stale → beim nächsten Cursor-Move malt `erase()` alten Hintergrund zurück ⇒ Cursor-„Dropping"/rechteckiges Loch. Genau das gemeldete Redraw-Maus-Artefakt. Die Schwester-Manager `menu.rs`, `form.rs`, `graf.rs` und die Drag-Pfade klammern korrekt mit hide/show — nur der Window-Composite tut es nicht (belegt durch die Grep-Asymmetrie).
- **1:1-Fix:** `compose`/`redraw_clip` (und `redraw_open`/`redraw_vslider`/`redraw_hslider`) mit `cursor_hide(backend)` … `cursor_show(backend)` klammern, analog EmuTOS `w_update` moff/mon.
- **Runtime-verify:** yes

### AUD-P5-003: `wind_delete` schließt ein noch offenes Fenster nicht → stehengebliebener Frame
- **Severity:** HIGH
- **EmuTOS:** gemwmlib.c:1466-1485 — `if (pwin->w_flags & VF_ISOPEN) wm_close(w_handle);` (:1473) vor dem Freigeben; `wm_close`→`draw_change` deckt den freiwerdenden Bereich auf und zeichnet Hintergrund + darunterliegende Fenster neu.
- **NewTOS:** window.rs:1189-1198 — setzt nur `in_use=false; open=false; ret=1`, **kein** Close/Uncover-Redraw.
- **Divergenz:** `wind_delete` auf ein offenes Fenster lässt dessen Frame + Inhalt als toten Pixelblock stehen (kein Composite über den freigewordenen Rechteck). EmuTOS deckt auf und schickt WM_REDRAW an aufgedeckte Fenster.
- **1:1-Fix:** Bei `w.open` zuerst die `wind_close`-Logik (Damage-Redraw + `broadcast_redraw`) ausführen, dann Slot freigeben.
- **Runtime-verify:** yes

### AUD-P5-004: Slider-Elevator-Drag + Track-Page-Klicks fehlen → WM_HSLID/WM_VSLID/WA_*PAGE nie emittiert
- **Severity:** HIGH
- **EmuTOS:** Control-Manager erzeugt WM_HSLID(25)/WM_VSLID(26) mit 0..1000-Position beim Elevator-Drag (`gr_slidebox`) und WM_ARROWED mit WA_UPPAGE/DNPAGE/LFPAGE/RTPAGE (0/1/4/5) beim Track-Klick (gemctrl.c:111-139,300-305; Spec-Tabelle window-graphics.md:711-712,729-734).
- **NewTOS:** `gadget_at` window.rs:1554-1614 kennt nur Arrows (`Up`/`Dn`/`Lf`/`Rt`) + Closer/Fuller/Mover/Sizer — **keinen** Slider-Thumb/Track-Treffer; `intercept` window.rs:1836-1876 emittiert nur WA_UPLINE/DNLINE/LFLINE/RTLINE.
- **Divergenz:** Ein Klick/Drag auf den Elevator oder in die Scroll-Rinne fällt auf `Gadget::None` → `intercept` gibt `false` zurück → App bekommt einen rohen Button-Event statt einer Scroll-Nachricht. Scrollen per Thumb-Drag/Paging ist tot; nur die Pfeil-Zeilen-Scrolls funktionieren.
- **1:1-Fix:** `gadget_at` um Elevator-/Track-Zonen erweitern; bei Elevator-Drag `graf_slidebox`-Logik laufen lassen und WM_HSLID/WM_VSLID (msg[4]=0..1000) posten; bei Track WM_ARROWED mit WA_*PAGE.
- **Runtime-verify:** yes

### AUD-P5-005: `wind_update` — No-op-Stub, ignoriert den Modus (BEG/END_UPDATE + BEG/END_MCTRL)
- **Severity:** MEDIUM
- **EmuTOS:** gemwmlib.c:1778-1797 — Modus 0/1 = END/BEG_UPDATE (Window-Tree-Lock `wind_spb`), 2/3 = END/BEG_MCTRL (`fm_own`, Maus-Kontroll-Übergabe an die App).
- **NewTOS:** window.rs:1461-1463 — `pb.set_ret(1)`, sonst nichts; `int_in[0]` (Modus) wird komplett ignoriert.
- **Divergenz:** Verstößt gegen die No-Stub-Regel (hardcodierter Return, Args ignoriert). Beobachtbare Folge: BEG_MCTRL schaltet die AES-Frame-Interception (`intercept`) nicht ab; eine App, die die Maus per `wind_update(BEG_MCTRL)` übernimmt, hat weiterhin die AES-Gadget-Interception aktiv. BEG/END_UPDATE liefert keine Layout-Sperre (im Single-Tasking meist unkontendiert, daher begrenzt sichtbar).
- **1:1-Fix:** Zählenden Update-Semaphor + `ml_ocnt`-MCTRL-Zähler implementieren; MCTRL-Zustand in `intercept` respektieren; `wind_new` gibt die Locks frei.
- **Runtime-verify:** yes

### AUD-P5-006: `graf_mouse` M_OFF/M_ON nicht zählend (Force-Hide/Show statt Semaphor)
- **Severity:** MEDIUM
- **EmuTOS:** gemgrlib.c:433-438 — M_OFF→`gsx_moff()` (`gl_moff++`), M_ON→`gsx_mon()` (`gl_moff--`, Anzeige erst bei 0) — zählender Semaphor (gemgsxif.c:440-455).
- **NewTOS:** graf.rs:424-431 — M_OFF→`mouse::remove` (HIDE_COUNT **=1**, mouse.rs:394), M_ON→`display_at` (HIDE_COUNT **=0**, mouse.rs:385) — beide unbedingt/nicht zählend.
- **Divergenz:** Verschachtelte `M_OFF;M_OFF;M_ON` — EmuTOS bleibt verborgen (Count 1), NewTOS zeigt den Cursor bereits beim ersten M_ON. Verschachtelte Hide/Show-Blöcke fremder Bibliotheken (die auf die Zählung bauen) zeigen den Cursor zu früh → weitere Artefaktquelle.
- **1:1-Fix:** M_OFF→`cursor_hide` (inkrementierend), M_ON→`cursor_show` (dekrementierend, Anzeige bei 0), statt `remove`/`display_at`.
- **Runtime-verify:** yes

### AUD-P5-007: `graf_mouse` M_SAVE/M_RESTORE/M_PREVIOUS unimplementiert (No-op)
- **Severity:** MEDIUM
- **EmuTOS:** gemgrlib.c:439-451 (`CONF_WITH_GRAF_MOUSE_EXTENSION`, Default 1): M_SAVE speichert `gl_mouse` in `rlr->p_mouse`, M_RESTORE/M_PREVIOUS installieren die gespeicherte/vorige Form.
- **NewTOS:** graf.rs:434-443 — dokumentierter No-op (Form bleibt unverändert).
- **Divergenz:** App-Muster „M_SAVE → Hourglass setzen → M_RESTORE" lässt den Cursor als Hourglass stehen statt die alte Form zurückzusetzen. Ehrlicher No-op, aber beobachtbar abweichend; Deferred laut STATUS (braucht Saved-Form-Slot im VDI-Cursor).
- **1:1-Fix:** Saved-Form-Slot + Current-Form-Reader in `vdi/mouse.rs`; die drei Modi wie EmuTOS bedienen.
- **Runtime-verify:** yes

### AUD-P5-008: `wind_close` auf gültiges, nicht-offenes Fenster liefert TRUE statt FALSE
- **Severity:** MEDIUM
- **EmuTOS:** gemwmlib.c:1446-1447 — `if (!(pwin->w_flags & VF_ISOPEN)) return FALSE;`
- **NewTOS:** window.rs:1157-1186 — bei `was_open==false` wird der Redraw-Block übersprungen, aber `pb.set_ret(1)` (:1185) trotzdem gesetzt.
- **Divergenz:** Doppel-Close / Close eines nur erzeugten Fensters gibt 1 statt 0 zurück; eine App, die auf FALSE prüft, verhält sich anders.
- **1:1-Fix:** Bei nicht-offenem (aber gültigem) Fenster `set_ret(0); return`.
- **Runtime-verify:** no

### AUD-P5-009: `wind_open` auf bereits offenes Fenster liefert TRUE + toppt neu statt FALSE
- **Severity:** MEDIUM
- **EmuTOS:** gemwmlib.c:1418-1419 — `if (pwin->w_flags & VF_ISOPEN) return FALSE;` (keine Aktion).
- **NewTOS:** window.rs:1122-1142 — kein Open-Check; setzt `curr`/`open`/`owner`/`z` neu, zeichnet, `set_ret(1)`.
- **Divergenz:** Reopen eines offenen Fensters gibt 1 statt 0 und hebt es unerwartet an die Spitze/verschiebt es.
- **1:1-Fix:** Wenn `w.open` bereits gesetzt: `set_ret(0); return` ohne Nebeneffekt.
- **Runtime-verify:** no

### AUD-P5-010: `WF_COLOR`/`WF_DCOLOR`/`WF_OWNER` in get+set nicht implementiert (FALSE) trotz Default-Build
- **Severity:** MEDIUM
- **EmuTOS:** `wm_get` gemwmlib.c:1566-1590 (`WF_COLOR` 18 / `WF_DCOLOR` 19 unter `CONF_WITH_WINDOW_COLOURS`=1; `WF_OWNER` 20 unter `AES_VERSION>=0x330`, Default 0x340); `wm_set` :1718-1734. `WF_DCOLOR` braucht keinen gültigen Handle (:1632).
- **NewTOS:** `wind_get` window.rs:1293-1296 (default→FALSE), `wind_set` window.rs:1426-1429 (default→FALSE).
- **Divergenz:** Alle drei Felder existieren im Default-Build und liefern in EmuTOS TRUE/Daten; NewTOS gibt FALSE. Eine App, die per `wind_set(WF_COLOR)` Gadget-Farben setzt oder `wind_get(WF_OWNER)` abfragt, sieht Fehlschlag. Ehrlicher FALSE, aber fehlende Default-Features.
- **1:1-Fix:** `w_tcolor`/`w_bcolor`-Arrays + `gl_wtcolor`/`gl_wbcolor` nachrüsten (get: `int_out[2..3]` — D-095!); `WF_OWNER` aus pid/open/z-order.
- **Runtime-verify:** no

### AUD-P5-011: `WM_UNTOPPED` (30) beim Toppen nie emittiert
- **Severity:** MEDIUM
- **EmuTOS:** `perform_untop` schickt WM_UNTOPPED (30) an den bisherigen Top-Besitzer (gemctrl.c:151; Spec-Tabelle window-graphics.md:715).
- **NewTOS:** `intercept` window.rs:1824-1830 (Click-to-top) und `top_window` window.rs:227 emittieren nur WM_TOPPED an das neu-getoppte Fenster; kein WM_UNTOPPED an das alte Top.
- **Divergenz:** Das bisher aktive Fenster erhält keine Deaktivierungs-Nachricht; eine App, die WM_UNTOPPED nutzt (Menü-/Cursor-Umschaltung beim Deaktivieren), reagiert nicht. WM_NEWTOP(29)/WM_ONTOP(31) korrekterweise nicht emittiert (D-053) — keine Beanstandung dort.
- **1:1-Fix:** Beim Wechsel des Top-Fensters WM_UNTOPPED (msg[0]=30, msg[3]=alter Top-Handle) an dessen Owner posten.
- **Runtime-verify:** yes

### AUD-P5-012: `wind_get(WF_SCREEN)` liefert hartkodiert 0/0-Puffer
- **Severity:** MEDIUM
- **EmuTOS:** gemwmlib.c:1563-1564 → `gsx_mret` gibt Menü/Alert-Save-Puffer-Adresse+Länge (`gl_tmp.fd_addr`/`gl_mlen`, gemgsxif.c:125-129).
- **NewTOS:** window.rs:1244-1250 — schreibt `int_out[1..4]=0`, `ret=1`.
- **Divergenz:** Stub-artiger Konstant-Return (0-Adresse/0-Länge). NewTOS' `bb_save` allokiert transient pro-Alert, daher gibt es keinen stabilen Puffer. Dokumentiert/deferred; beobachtbar für Apps, die WF_SCREEN als Scratch-Buffer verwenden (selten). Grenzfall zwischen ehrlichem 0-Return und No-Stub-Verstoß.
- **1:1-Fix:** Persistenten Menü/Alert-Save-Buffer bereitstellen und dessen Adresse+Länge melden, oder als bewusst nicht-unterstützt dokumentieren.
- **Runtime-verify:** no
- **BEWERTUNG KORRIGIERT, TEILWEISE ERLEDIGT 2026-09-12 (A54):** Die frühere
  Einstufung als *bewusste Abweichung* („null/zero-length buffer truthfully,
  `ret=1`", Kommentar in `window.rs`) **ist zurückgenommen**. Sie hielt nicht
  stand: der Wert ist über die dokumentierte API messbar, und er *wurde*
  gemessen — Fälle 331/332/439 von `tosdump/cases/aes-r4-subfunctions.tsv`
  (CRC `227422B2`), EmuTOS 1.4 auf dem TT liefert Adresse `0x01000218`, Länge
  `40960`, `ret=1` (`measurements/measurement-2026-09-12-aes-r4/`). Eine Begründung ist
  keine Unbeobachtbarkeit. Vor allem aber war `ret=1` mit Adresse 0 die eine
  Antwort, die ein Aufrufer *nicht* von „hier ist ein Puffer, er liegt bei 0"
  unterscheiden kann.
  **Erledigt ist die Hälfte, die ohne Architekturentscheidung geht:**
  `wind_get(WF_SCREEN)` antwortet jetzt **FALSE und schreibt nichts** —
  `wm_get`s eigene Redensart für ein Feld, das diese AES nicht beantwortet
  (`default: return FALSE`, `aes/gemwmlib.c:1590-1591`), dieselbe Reparatur wie
  bei AUD-P5-014/`WF_KIND`. **Offen bleibt der Puffer selbst**, und er ist
  keine `wind_get`-Frage: NewTOS hat keinen persistenten Menü-/Alarmpuffer
  (`form.rs` `BbSave` = frischer `Vec` je Alarm, verworfen beim Schließen;
  `menu.rs` `MBACK`/`SMBACK` = zwei Ablagen, je Dropdown geleert und neu
  gewachsen, beim Start Kapazität 0), und es gibt auch keine Adresse, die man
  herausgeben *könnte*: diese Puffer liegen im Kernel-Heap, der `AP_RW_EL1`
  abgebildet ist — **kein EL0-Zugriff**
  (`kernel/src/arch/aarch64/mmu.rs:82,178-179,283-284`) —, und genau darauf zu
  schreiben ist der Zweck des Feldes. Ein echter persistenter AES-Kratzpuffer
  in EL0-erreichbarem Speicher ist eine Speicherpool- und Abbildungsfrage.
  Registriert als *sanctioned deviation* in `docs/COMPATIBILITY.md`; Dauertests
  `tos-api/tests/aes_window_screen_buffer.rs`, Referenzfall 166 der r1-Tabelle
  über `SANCTIONED` in `tos-api/tests/aes_reference/harness.rs`.

### AUD-P5-013: `wind_calc`/Frame verwendet non-3D-Rahmendicken statt 3D/TOS4-Default
- **Severity:** MEDIUM
- **EmuTOS:** Default-Build ist `CONF_WITH_3D_OBJECTS=1` → `wm_calc` gemwmlib.c:1817-1840: Titel-Band `adj_hbox-1` (= gl_hbox+3 = 22), INFO `gl_hbox` (19), V-Bar `adj_wbox-1` (22), H-Bar `adj_hbox-1` (22). Non-3D (:1841-1853): je `gl_hbox-1`/`gl_wbox-1` (18).
- **NewTOS:** `borders` window.rs:725-743 addiert je `GAD-1` = **18** pro Band (= non-3D-Zweig); `GAD=19`.
- **Divergenz:** `wind_calc(WC_WORK)` und damit `WF_WORKXYWH` liefern eine andere Work-Rect-Geometrie als der EmuTOS-Default-Build (Bänder 22/19 vs. konstant 18). EmuTOS reserviert im 3D-Band 2px Bevel-Rand je Seite (ADJ3DSTD) um das 19px-Gadget; NewTOS zeichnet 3D-Bevels auf non-3D-Geometrie → engerer Frame als TOS4, und die icon-column-Logik (gemwmlib.c:1314-1325) passt nicht. Intern konsistent (`calc` treibt Draw und WF_WORKXYWH), daher für NewTOS-eigene Apps unkritisch; für 1:1-TOS4-Optik/-Geometrie eine echte Abweichung. Spec markiert Frame-Geometrie als *reference* — Spannung zur Projektvorgabe „TOS4-3D-Look" ([[aes-dialog-3d-look]], ADR-022 in `docs/MEMORY.md`).
- **1:1-Fix:** Ziel-Build klären (Researcher). Falls 3D-Default: Bänder auf `adj_hbox-1`/`adj_wbox-1` (=gl_hbox+3) für Titel/Bars, `gl_hbox` für INFO umstellen, in `borders` **und** `draw_frame`.
- **Runtime-verify:** yes (Screendump gegen EmuTOS-Frame)

### AUD-P5-014: `wind_get/set(WF_KIND)` liefert kind+TRUE statt FALSE (bewusste Abweichung, D-053)
- **Severity:** LOW
- **EmuTOS:** gemwmlib.c:1591/1736 — WF_KIND(1) fällt in `default: return FALSE` (nicht unterstützt).
- **NewTOS:** window.rs:1258-1262 — `int_out[1]=w.kind`, `ret=1`.
- **Divergenz:** NewTOS implementiert WF_KIND (get). Laut D-053/Open-Question ist das die bevorzugte Richtung (trivial, `w_kind` gespeichert), also **akzeptierte Abweichung** — jedoch: eine App, die auf EmuTOS' FALSE prüft, sieht TRUE. Bewusst dokumentiert.
- **1:1-Fix:** Keiner nötig, falls die Abweichung bestätigt bleibt; sonst FALSE zurückgeben. (`wind_set(WF_KIND)` fällt korrekt auf FALSE.)
- **Runtime-verify:** no
- **ERLEDIGT 2026-08-31:** Abweichung **zurückgenommen**. Der `WF_KIND`-Zweig ist
  aus `wind_get` entfernt; das Feld fällt jetzt wie in EmuTOS in den
  `_`-Default (`ret = 0`, kein geschriebenes Wort). Grund: außerhalb der
  Hardware-Schicht gibt es keine bewussten Abweichungen. Gemessen unter QEMU:
  `wind_get(0, WF_KIND) → ret=0`, `int_out` bleibt vergiftet. Fall 164 der
  AES-Tabelle bleibt damit MATCH (vorher nur zufällig, weil Handle 0 keinen
  Slot hatte).

### AUD-P5-015: `graf_growbox`/`graf_shrinkbox`-Animation ohne Move-to-Center-Zoom
- **Severity:** LOW
- **EmuTOS:** `gr_growbox` gemgrlib.c:309-319: erst `gr_movebox` von Ursprung zum **Zentrum** des Ziel-Rects, dann `gr_2box`-Zoom Zentrum→voll; `gr_shrinkbox` umgekehrt.
- **NewTOS:** graf.rs:261-275 → `animate(small, big)` graf.rs:213 interpoliert linear Ecke-zu-Ecke (8 Schritte).
- **Divergenz:** Rein visuelle Animation (XOR-Tracking ist laut Spec *reference*), Slots korrekt (small `int_in[0..3]`, big `int_in[4..7]`, D-053). Kosmetisch.
- **1:1-Fix:** Optional Move-to-Center-dann-Zoom nachbilden; nicht ABI-relevant.
- **Runtime-verify:** no

### AUD-P5-016: `graf_mouse(USER_DEF)` ignoriert MFORM-fg/bg-Farbwörter
- **Severity:** LOW
- **EmuTOS:** `gsx_mfset` gemgsxif.c:407-420 kopiert die volle MFORM (inkl. `mf_fg`/`mf_bg`) in `intin`.
- **NewTOS:** `set_user_form` graf.rs:456-471 liest nur word(0/1)=Hotspot, mask=word(5..20), data=word(21..36); Farbwörter (2/3/4) werden verworfen; `set_form` erzwingt mask=weiß/data=schwarz (mouse.rs:201-202).
- **Divergenz:** Farbige Custom-Cursor rendern mit Default-Farben. Mask/Data-Offsets (5/21) korrekt. Für monochrome Cursor (Normalfall) unauffällig.
- **1:1-Fix:** word(3)/word(4) als bg/fg-Pen an `set_form` durchreichen.
- **Runtime-verify:** no

### AUD-P5-017: Rectangle-Liste RCAP=16-Cap kann sichtbare Stücke stillschweigend verwerfen
- **Severity:** LOW
- **EmuTOS:** `newrect`/`brkrct` gemwrect.c:101-205 — dynamische ORECT-Freelist (`NUM_ORECT = NUM_WIN·10`), kein harter Cap.
- **NewTOS:** `subtract`/`put` window.rs:536-571 — `put` verwirft Stücke bei `*m >= RCAP` (16); `compute_rlist` :599.
- **Divergenz:** Bei vielen überlappenden Fenstern (bis 8 × je 4 Splits) kann die Stückzahl 16 überschreiten; verworfene Stücke fehlen in `WF_FIRSTXYWH`/`NEXTXYWH` → die App zeichnet einen sichtbaren Bereich nicht neu (stale Content). Edge-Case.
- **1:1-Fix:** RCAP großzügiger dimensionieren (≥ NUM_WIN·…) oder dynamisch; zumindest Überlauf erkennen.
- **Runtime-verify:** yes

### AUD-P5-018: Diverse ungültige Handles liefern TRUE statt FALSE
- **Severity:** LOW
- **EmuTOS:** `wm_get` verlangt gültigen Handle außer WF_TOP/WF_SCREEN/WF_DCOLOR (gemwmlib.c:1517-1524); `wm_set` außer WF_DCOLOR (:1632-1636).
- **NewTOS:** `wind_get` WF_FIRSTXYWH/NEXTXYWH window.rs:1213-1224 ruft `compute_rlist(handle)` ohne Handle-Validierung → leerer Rect + `ret=1`; `wind_set(WF_NEWDESK)` window.rs:1328-1332 vor dem `win()`-Guard → jeder Handle liefert 1.
- **Divergenz:** Ungültiger Handle bei WF_FIRSTXYWH/WF_NEWDESK gibt 1 statt FALSE. Minimal (WF_NEWDESK-Vorabbehandlung ist für DESKWH=0 nötig und korrekt).
- **1:1-Fix:** WF_FIRSTXYWH/NEXTXYWH: bei ungültigem Nicht-Desktop-Handle FALSE; WF_NEWDESK nur für Handle 0 akzeptieren.
- **Runtime-verify:** no

---

## Zusammenfassung

**Counts:** BLOCKER 1 · HIGH 3 · MEDIUM 8 · LOW 6 — gesamt 18 Findings über ~34 auditierte Funktionen/Gadget-Pfade.

**Top-3 Blocker/kritisch:**
1. **AUD-P5-001 (BLOCKER):** `graf_slidebox` gibt die Position im falschen Slot zurück (`int_out[1]` statt `int_out[0]`) → Standard-Bindung liest immer 1; Elevator-Scrollen praktisch defekt.
2. **AUD-P5-002 (HIGH):** Window-Composite/Redraw ohne `gsx_moff`/`gsx_mon`-Klammerung → **Maus-Cursor-Droppings beim Redraw** (deckt sich exakt mit den gemeldeten Artefakten; Mechanismus per Save-under-Sprite belegt).
3. **AUD-P5-003 / AUD-P5-004 (HIGH):** `wind_delete` lässt offene Fenster als toten Frame stehen; Slider-Thumb-Drag/Track-Paging fehlt (WM_HSLID/WM_VSLID/WA_*PAGE nie emittiert).

**Explizite Notiz zu Maus-Cursor-Hide/Show-Divergenzen (Aufgaben-Schwerpunkt):**
- **AUD-P5-002 (Hauptursache):** EmuTOS klammert *jede* Fenster-Neuzeichnung in `w_update` mit `gsx_moff`/`gsx_mon` (gemwmlib.c:1011/1047). NewTOS' `compose`/`redraw_clip`/`draw_frame`/`redraw_open`/`redraw_vslider`/`redraw_hslider` tun das **nicht** — obwohl `menu.rs`, `form.rs`, `graf.rs` und die Drag-Pfade es korrekt tun. Diese Asymmetrie ist die wahrscheinlichste Quelle der Redraw-Artefakte.
- **AUD-P5-006:** `graf_mouse(M_OFF/M_ON)` ist nicht der zählende Semaphor (`remove`/`display_at` forcieren statt `gl_moff++/--`) — verschachtelte Hide/Show-Blöcke zeigen den Cursor zu früh (Zweitquelle für Artefakte).
- **AUD-P5-007:** `graf_mouse(M_SAVE/M_RESTORE/M_PREVIOUS)` No-op → falsche Cursor-Form nach Save/Restore.
- Positiv verifiziert: der VDI-Sprite selbst (`draw`/`erase`/`HIDE_COUNT`) ist zählend und korrekt (mouse.rs:300-343); der Fehler liegt allein in den fehlenden Klammerungen im Window-Composite.

**Was getrieben wurde vs. offen:** Statischer Quelle-gegen-Quelle-Abgleich aller gelisteten Funktionen; Slot-/Return-ABI gegen `gemsuper.c:265-360` verifiziert; die Maus-Artefakt-Mechanik über den Save-under-Sprite (`draw`/`erase` mouse.rs) mechanisch bewiesen; die Hide/Show-Asymmetrie per Grep über alle AES-Manager belegt (CONFIRMED per Code-Reading). **Nicht getrieben** (auftragsgemäß kein Build/QEMU): pixelgenaue Frame-Geometrie (AUD-P5-013), Laufzeit-Reproduktion der Artefakte/Scroll-Nachrichten/Cursor-Zählung — alle mit „Runtime-verify: yes" markiert (Screendump/QMP bzw. echte-Pi4). `CONF_WITH_*`-Gating geprüft: 3D-Objects/Window-Colours/Graf-Mouse-Extension sind Default=1, daher sind AUD-P5-006/007/010/013 echte Default-Build-Abweichungen, keine HW-Absenzen.

**Out-of-scope-Beobachtungen (nicht verfolgt):**
- `graf_mouse` droppt/restauriert die Control-Manager-Maus-Ownership (`gl_ctmown`, gemsuper.c:295-302) nicht — NewTOS hat kein persistentes ctlmgr-Ownership-Modell; vermutlich N/A.
- Spec `window-graphics.md:389-390` behauptet WF_NAME/WF_INFO-Redraw „all open windows"; EmuTOS `wm_set` ruft `w_cpwalk(w_handle, …)` nur für das eine Fenster (gemwmlib.c:1755) — Spec-Ungenauigkeit (Researcher).
- Drop-Shadow (`DROP_SHADOW_SIZE=2`, gemwmlib.c:49) fehlt in NewTOS' Frame — laut Spec reference, nicht verfolgt.

**Cleanup-Bestätigung:** Keine Produkt-Code-Änderung, keine temporäre Verdrahtung, keine Selbsttests eingefügt. Nur dieses Katalog-Artefakt geschrieben; der Arbeitsbaum ist sonst unverändert.

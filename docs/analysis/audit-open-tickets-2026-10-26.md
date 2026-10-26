# Audit-Liste: alle offen dokumentierten Tickets (2026-10-26)

Alle Tickets unter `TASK/`, deren Statuszeile sie als nicht abgeschlossen
fuehrt, gleich ob schon daran gearbeitet wurde (A–C) oder nicht (D–F) (Stand `67f00f02`). Ersetzt fuer den Audit die Listen
`audit-50-tickets`, `audit-50-started-tickets` und `audit-50-worked-tickets`
vom selben Tag.

Auswahlregel:
- „Gearbeitet“ = Quittung `-STATUS-*.MD`, eigener Messlauf unter
  `measurements/`, Status TEILWEISE oder ein Stand-/Abschluss-Abschnitt mit
  erledigten Teilpunkten.
- In A–C nicht aufgenommen: ERLEDIGT (auch mit offener Pi-Abnahme, z. B. A111, A115,
  A117), BEHOBEN und auf der Pi abgenommen (DESK46, PORT05, SCHED01), BEHOBEN
  ohne offenen Rest (ACC10, FS04), GEKLAERT (PORT08), GEGENSTANDSLOS (A31, A51,
  A58), ZURUECKGEZOGEN (DESK23, DESK29), GESCHLOSSEN ohne Umsetzung (GUI01,
  BALL01) und alle offenen Tickets ohne jede Spur von Arbeit.
- „Zuletzt“ = letzter Commit auf Ticket oder Quittung; „Messl.“ = Anzahl
  `measurements/measurement-*-<id>*`.

## A. Umgesetzt, Abnahme offen

| Ticket | Zuletzt | Messl. | Was fehlt | Thema |
|---|---|---|---|---|
| A16 | 2026-10-01 | 0 | HW-Abnahme | wind_close: WS_CURR / WS_PREV / WS_WORK bleiben unaktualisiert |
| A34 | 2026-10-01 | 1 | Abnahme (A33) | `wind_create` schreibt `WS_WORK`, EmuTOS nie |
| A37 | 2026-10-01 | 2 | anwendungsseitige Abnahme | fix_tedinfo_std fehlt: te_txtlen wird nicht neu berechnet |
| A78 | 2026-10-01 | 1 | Tester- und HW-Abnahme | wind_set(WF_CXYWH): kein w_move-Blit, ein WM_REDRAW zu viel, Fan-out in Handle- statt z-Reihenfolge |
| DESK34 | 2026-10-01 | 1 | HW-Abnahme | Ein Unterverzeichnis oeffnet ein neues Fenster statt im selben zu navigieren |
| DESK35 | 2026-10-01 | 1 | HW-Abnahme | Beim Start einer GEM-Anwendung bleiben die Desktop-Fenster stehen und frieren ein |
| DESK42 | 2026-10-01 | 1 | HW-Abnahme | Der `Show info`-Kasten sieht EmuTOS' nicht aehnlich |
| DESK44 | 2026-10-01 | 0 | HW-Gegenprobe | `.NTP` laeuft als GEM, soll aber Konsole mit Parametern sein |
| GUI06 | 2026-10-01 | 1 | Pi-Abnahme | Der Boden der Geste: zwei Vollbild-Neuzeichnen und 400–600 MB Cachepflege |
| HW03 | 2026-10-01 | 1 | drei Pi-Gegenproben | Schreibfehler der Karte erreichen niemanden, und ein Sektor kann still verloren gehen |
| V17 | 2026-10-01 | 1 | Tester-Abnahme | Mauszeiger startet einen Pixel zu weit unten rechts |
| FS09 | 2026-10-07 | 1 | Quittung FERTIG (QEMU), Status steht noch OFFEN; Pi-Abnahme | Block read cache after FreeMiNT block_IO |
| INPUT01 | 2026-10-08 | 1 | Quittung vorhanden, Status steht noch OFFEN; Abnahme | the mouse pointer freezes for 2-3 s while zView starts |
| ZVIEW01 | 2026-10-07 | 1 | Quittung FERTIG (QEMU), Status OFFEN; Pi-Nachweis | zView browser shows no file or folder names |
| ZVIEW02 | 2026-10-07 | 1 | Quittung vorhanden, Status OFFEN; Abnahme | double-click on a folder tile in zView's browser only selects |
| ZVIEW04 | 2026-10-08 | 1 | Quittung vorhanden, Status OFFEN; Abnahme | File > Quit does not quit zView while the browser window is open |

## B. Teilweise umgesetzt

| Ticket | Zuletzt | Messl. | Was fehlt | Thema |
|---|---|---|---|---|
| A118 | 2026-10-26 | 4 | Umbau auf EmuTOS 1:1 fuer abgelehnte Wiederausfuehrung, 5 Testerfaelle | Restliche Nehmer der `wind_update`-Sperre nach EmuTOS |
| USB01 | 2026-10-25 | 14 | F1–F3 erledigt; F4/F6/F7, SET_IDLE, Gamepads | USB-Maus: nur Boot-Protokoll, Steckkombinationen, Diagnosewerkzeug |
| USB02 | 2026-10-24 | 4 | Punkte 10/11 erledigt; Punkte 1–9 | Kleinbefunde aus USB01 F1/F2/F8 |
| ARCH03 | 2026-10-13 | 10 | Stufe 2b, `asid=on` auf echter Pi | Prozess- und Speicherarchitektur nach FreeMiNT: ein gemeinsamer Adressraum mit Schutzrechten |
| FS17 | 2026-10-21 | 0 | Punkt 3 erledigt (FS18); Punkte 1, 2, 4, 5, 6 offen, 7 ungeprueft | Kleinere GEMDOS-Datei-Abweichungen aus PROC04 Punkt 5 |
| PORT01 | 2026-10-01 | 1 | Ursache behoben; Abnahme blockiert durch zweiten Defekt | Eine auf QED gezogene Datei erreicht die Anwendung nicht |
| PORT11 | 2026-10-21 | 0 | Plain-crt0-Absatz in `docs/sdk/03` | SDK-Doku auf den Stand nach E9 bringen (`malloc` ueber `Malloc`, crt0) |
| T08 | 2026-10-21 | 0 | Test fuer den Vektortausch | VDI `vex_*`: Vektortausch in `CONTRL[7..]`/`[9..]` fest 8 Byte an festem Index |
| T10 | 2026-10-21 | 0 | Abnahme alter a78-Lauf | Die duale Injektor-Bewaffnung waehlt seit den Accessories immer das leere Skript |
| ACC06 | 2026-10-01 | 1 | Teil 1 erledigt, Teil 2 nur als Quelltext | Ein kleines Accessory als Pruefling fuer ACC02/ACC03 |
| SDK01 | 2026-10-22 | 1 | Punkt K umgesetzt, Abnahme Linux/Homebrew offen | SDK fuer Fremde benutzbar machen: libc-Luecken und Portierungs-Doku |
| T13 | 2026-10-01 | 0 | `newtos_pause()` fertig; SDK-Beispiele zurueckgestellt | Die Testprogramme sollen ihr Ergebnis stehenlassen |

## C. Nach Arbeit blockiert

| Ticket | Zuletzt | Messl. | Was fehlt | Thema |
|---|---|---|---|---|
| A45 | 2026-09-13 | 0 | abgeschlossen als BLOCKED (wartet auf fehlende Voraussetzung) | appl_tplay und appl_trecord sind Stubs |

Summe A–C: 16 umgesetzt mit offener Abnahme, 12 teilweise umgesetzt, 1 blockiert = 29.


## D. Offen, noch nicht angefangen

| Ticket | Prio | Status | Zuletzt | Thema |
|---|---|---|---|---|
| HW04 | P2 | OFFEN | 2026-10-21 | EMMC: Initialisierung und Uebertragungsform gegen die Spezifikation |
| NET01 | P2 | OFFEN | 2026-10-20 | Netzwerk fuer NewTOS (Ziel Version 1.2): STinG/STiK-kompatibel |
| SND01 | P2 | OFFEN | 2026-10-23 | Ton: Falcon-DMA-Sound als API, HDMI als Ausgang |
| SND02 | P2 | OFFEN | 2026-10-23 | HDMI-Ton: die Quellenlage ist ungeklaert, nicht guenstig |
| USB03 | P2 | OFFEN | 2026-10-25 | Mausrad von Kabelmaeusen, die es nur im Report-Protokoll schicken (B100, GitHub #8) |
| W01 | P2 | OFFEN | 2026-09-13 | RSC-Betrachter als echte GEM-Anwendung |
| A119 | P3 | OFFEN | 2026-10-22 | `wind_set(WF_INFO/WF_NAME)` zeichnet den ganzen Rahmen statt nur des Elements |
| A120 | P3 | OFFEN | 2026-10-22 | Nachrichten-Queue: Reste nach A116 (wartender Leser, Ketten-Queue, veraltete Spec) |
| A121 | P3 | OFFEN | 2026-10-25 | AC_OPEN ohne `perform_untop(gl_wtop)`; veraltete Single-app-Notiz in `_state.md` |
| A122 | P3 | OFFEN | 2026-10-25 | `edit_char`: Tastenwort und Zeichenbereich weichen von EmuTOS ab |
| A123 | P3 | OFFEN | 2026-10-25 | G_FTEXT ohne Fuellung: alte Glyphenpixel bleiben nach Textaenderung stehen |
| A124 | P3 | OFFEN | 2026-10-25 | `sched_a115_ready_list` unterscheidet dlr- von Slot-Reihenfolge nicht; `selftest_sleepwake_switch` verrottet |
| ACC08 | P3 | OFFEN | 2026-09-24 | Accessory mit eigenem Fenster |
| ACC09 | P3 | OFFEN | 2026-09-24 | `CONFIG.NAC`: die `config.txt` aus dem laufenden System einstellen |
| ARCH04 | P3 | OFFEN | 2026-10-19 | CPU-Eigenschaften zur Laufzeit aus `ID_AA64*_EL1` lesen |
| BOOT01 | P3 | OFFEN | 2026-10-21 | Bootschirm: Layout auf hohen Aufloesungen, Ausrichtung, Tippfehler |
| CON03 | P3 | OFFEN | 2026-10-22 | Die Kernel-Konsole ist nicht EmuCON: Befehlssatz und Ort |
| CON05 | P3 | OFFEN | 2026-10-22 | Absturzbericht sendet `ESC f ESC v` auch auf die serielle Schnittstelle |
| DESK49 | P3 | OFFEN | 2026-09-29 | TeraDesks `filetypes`-Liste kennt die vier nativen Endungen nicht |
| DESK51 | P3 | OFFEN | 2026-10-01 | `graf_shrinkbox` fehlt in `w_setpath` und `fun_full_close` |
| DESK52 | P3 | OFFEN | 2026-10-01 | `do_fopen` ohne Laufwerksprobe und ohne `STDEEPPA` |
| DESK53 | P3 | OFFEN | 2026-10-01 | `refresh_window` (ESC) liest nicht neu ein |
| DESK54 | P3 | OFFEN | 2026-10-01 | `do_xyfix` ohne 3D-Korrektur am rechten Rand |
| DESK55 | P3 | OFFEN | 2026-10-01 | `win_sname` zeigt nur das Verzeichnis statt der ganzen `p_spec` |
| DESK61 | P3 | OFFEN | 2026-10-16 | `act_chg` setzt `f_selected` bei unveraendertem Zustand nicht |
| DESK63 | P3 | OFFEN | 2026-10-16 | Desktop-Verknuepfung auf Programm-Verknuepfung: „illegale Operation“ statt Start |
| DESK65 | P3 | OFFEN | 2026-10-17 | Einstellungen des Desktops sofort speichern (Erweiterung) |
| DESK66 | P3 | OFFEN | 2026-10-17 | Desktop-Hintergrund an die AES melden (`set_aes_background`) |
| DESK67 | P3 | OFFEN | 2026-10-17 | Reste aus DESK48: Teilneuzeichnen und `fun_rebld` vollstaendig wie EmuTOS |
| DESK68 | P3 | OFFEN | 2026-10-25 | „Neuer Ordner“-Dialog ist nicht EmuTOS' ADMKDBOX |
| FS12 | P3 | OFFEN | 2026-10-16 | Laufwerksgrenze 16 statt 26 in BIOS und GEMDOS |
| FS15 | P3 | OFFEN | 2026-10-18 | Geraetenamen `CON:`, `AUX:`, `PRN:` bei `Fopen`/`Fcreate` |
| FS16 | P3 | OFFEN | 2026-10-18 | `Fwrite` schreibt den Sektorcache nach jedem Aufruf zurueck |
| HW07 | P3 | OFFEN | 2026-09-28 | Zeichnet die Pi ab Block 3 auf halbem Takt? |
| INPUT02 | P3 | OFFEN | 2026-10-17 | Ctrl+Alt+Del erreicht ein Programm in einer Endlosschleife nicht |
| LIBC-IOCTL | P3 | OFFEN | 2026-10-13 | mintlib-ioctl: TOS-Fallback fuer TIOCGETP/TIOCSETP/TIOCGWINSZ |
| M09 | P3 | OFFEN | 2026-09-13 | Der Dumper-Record hat keinen Platz für Pufferinhalte |
| M10 | P3 | OFFEN | 2026-09-14 | Eine falsche Begruendung in einer Falltabelle ist nicht korrigierbar |
| MEM08 | P3 | OFFEN | 2026-10-13 | `Mfree`/`Mshrink`/`detach` suchen linear: `Pterm` ist O(L×N) |
| MEM10 | P3 | OFFEN | 2026-10-13 | Speicherschutz-Synchronisierung ohne volle Momentaufnahme |
| MOD01 | P3 | OFFEN | 2026-09-23 | Quellenanalyse: FreeMiNTs Modulsystem als Vorlage fuer nachladbare NewTOS-Module |
| NET02 | P3 | OFFEN | 2026-09-23 | `netif`-Schicht: FreeMiNTs Treiberschnittstelle nachbauen |
| NET03 | P3 | OFFEN | 2026-09-23 | `inet4` portieren: ARP, IP, ICMP, UDP, TCP, Routing |
| NET04 | P3 | OFFEN | 2026-09-23 | Socket-API als GEMDOS-Geraet (`mintsock`) |
| NET05 | P3 | OFFEN | 2026-09-23 | HighWire: portieren, erst lokal, dann online |
| PORT04 | P3 | OFFEN | 2026-09-25 | mintlib und gemlib gegen ihren gepflegten Stand halten |
| REF03 | P3 | OFFEN | 2026-09-28 | Drei falsche Spec-Zitate, zwei davon im Produktcode |
| REF05 | P3 | OFFEN | 2026-10-22 | GIC-Quellen beschaffen (ARM IHI 0048B, DDI 0471B) |
| RST02 | P3 | OFFEN | 2026-10-17 | Warmstart EmuTOS-treu: Kern-Neueinstieg mit erhaltenen Systemvariablen |
| RTC01 | P3 | OFFEN | 2026-10-08 | Battery-backed clock: DS3231 on I2C as a BIOS clock driver |
| SDL01 | P3 | OFFEN | 2026-09-28 | Ein SDL-Backend fuer NewTOS: ein Traeger statt fuenf Portierungen |
| T11 | P3 | OFFEN | 2026-09-26 | `sched_pexec_parent_suspend` faellt gelegentlich aus (globaler Zustand) |
| T15 | P3 | OFFEN | 2026-10-01 | Weitere Zeichentests teilen den Mauszeiger-Zustand ohne Riegel |
| T17 | P3 | OFFEN | 2026-10-22 | `vdi_reference` Fall 167 haengt von der Testreihenfolge ab |
| X02 | P3 | OFFEN | 2026-09-21 | Aufloesungswechsel auf der Pi: Track-B-Vorlage fuer ADR-017 rev 3 (feste Modusliste) |
| DESK40 | P4 | OFFEN | 2026-09-25 | Zwei fehlende `valid_drive`-Pruefungen |
| GUI03 | P4 | OFFEN | 2026-09-24 | `CONF_WITH_LOADABLE_CURSORS`: Mauszeiger aus einer Ressourcendatei |
| GUI05 | P4 | OFFEN | 2026-09-24 | `CONF_WITH_ALT_DESKTOP_GRAPHICS`: linksbuendige Dialogtitel mit Linie |
| T09 | P4 | OFFEN | 2026-09-21 | Lader: datenloses Programm bekommt vier Null-Felder; `.init_array` wird nie ausgefuehrt; alte `selftest_emmc`-Bloecke pruefen nichts |

## E. Geparkt, zurueckgestellt, Idee, Zukunft

| Ticket | Prio | Status | Zuletzt | Thema |
|---|---|---|---|---|
| IDEA01 | - | IDEE | 2026-09-24 | Energieverwaltung als Accessory (Zukunft, kein Auftrag) |
| H01 | P2 | GEPARKT | 2026-09-13 | Echtzeituhr DS3231 als Zeitquelle beim Boot |
| H04 | P2 | GEPARKT | 2026-10-23 | Ton: Systemklingel, Tastenklick, `Dosound` — und ein Audioweg auf der Pi 4 |
| HW05 | P2 | ZURUECKGESTELLT | 2026-10-01 | Schreibdurchsatz: 89 KB/s, weil jeder halbe Kilobyte ein eigenes Kommando ist |
| COOK01 | P3 | ZURUECKGESTELLT | 2026-10-20 | `_MCH` = 0x00050000 ist die ARAnyM-Kennung |
| HW06 | P3 | ZURUECKGESTELLT | 2026-09-25 | Zwei Verzeichnisschreibungen kosten je 176 ms, Datenschreibungen 4 ms |
| IO01 | P3 | IDEE | 2026-10-13 | Externe Schnittstellen: AUX (seriell), MIDI, PRN (Drucker) |
| MEM06 | P3 | IDEE | 2026-10-13 | Heap als reservierter virtueller Bereich, Speicher beim ersten Zugriff |
| R01 | P3 | ZURUECKGESTELLT | 2026-10-01 | vro_cpyfm: oversized destination, D-VDI-25 |
| SLB01 | P3 | ZUKUNFT | 2026-10-06 | FreeMiNT shared libraries (SLB) and th-otto's zView |
| ZVIEW03 | P3 | ZUKUNFT | 2026-10-07 | thumbnail cache per directory (future) |
| GUI02 | P4 | ZURUECKGESTELLT | 2026-09-24 | `CONF_WITH_VIEWER_SUPPORT`: Standardbetrachter fuer nicht zugeordnete Dateien |
| HW02 | P4 | ZURUECKGESTELLT | 2026-09-25 | Takt: gemeldeter gegen gemessenen Wert (`GET_CLOCK_RATE_MEASURED`) |

## F. Backlog (Messreihen G-AES/G-VDI u. a.)

| Ticket | Prio | Status | Zuletzt | Thema |
|---|---|---|---|---|
| G-AES-011 | P3 | BACKLOG | 2026-09-13 | appl_read (AES 11): kleinsten Messfall entwerfen |
| G-AES-012 | P3 | BACKLOG | 2026-09-13 | appl_write (AES 12): kleinsten Messfall entwerfen |
| G-AES-013 | P3 | BACKLOG | 2026-09-13 | appl_find (AES 13): kleinsten Messfall entwerfen |
| G-AES-014 | P3 | BACKLOG | 2026-09-13 | appl_tplay (AES 14): kleinsten Messfall entwerfen |
| G-AES-015 | P3 | BACKLOG | 2026-09-13 | appl_trecord (AES 15): kleinsten Messfall entwerfen |
| G-AES-017 | P3 | BACKLOG | 2026-09-13 | appl_yield (AES 17): kleinsten Messfall entwerfen |
| G-AES-019 | P3 | BACKLOG | 2026-09-13 | appl_exit (AES 19): kleinsten Messfall entwerfen |
| G-AES-020 | P3 | BACKLOG | 2026-09-13 | evnt_keybd (AES 20): kleinsten Messfall entwerfen |
| G-AES-021 | P3 | BACKLOG | 2026-09-13 | evnt_button (AES 21): kleinsten Messfall entwerfen |
| G-AES-022 | P3 | BACKLOG | 2026-09-13 | evnt_mouse (AES 22): kleinsten Messfall entwerfen |
| G-AES-023 | P3 | BACKLOG | 2026-09-13 | evnt_mesag (AES 23): kleinsten Messfall entwerfen |
| G-AES-026 | P3 | BACKLOG | 2026-09-13 | evnt_dclick (AES 26): kleinsten Messfall entwerfen |
| G-AES-030 | P3 | BACKLOG | 2026-09-13 | menu_bar (AES 30): kleinsten Messfall entwerfen |
| G-AES-031 | P3 | BACKLOG | 2026-09-13 | menu_icheck (AES 31): kleinsten Messfall entwerfen |
| G-AES-032 | P3 | BACKLOG | 2026-09-13 | menu_ienable (AES 32): kleinsten Messfall entwerfen |
| G-AES-033 | P3 | BACKLOG | 2026-09-13 | menu_tnormal (AES 33): kleinsten Messfall entwerfen |
| G-AES-034 | P3 | BACKLOG | 2026-09-13 | menu_text (AES 34): kleinsten Messfall entwerfen |
| G-AES-035 | P3 | BACKLOG | 2026-09-13 | menu_register (AES 35): kleinsten Messfall entwerfen |
| G-AES-036 | P3 | BACKLOG | 2026-09-13 | menu_popup (AES 36): kleinsten Messfall entwerfen |
| G-AES-037 | P3 | BACKLOG | 2026-09-13 | menu_attach (AES 37): kleinsten Messfall entwerfen |
| G-AES-038 | P3 | BACKLOG | 2026-09-13 | menu_istart (AES 38): kleinsten Messfall entwerfen |
| G-AES-039 | P3 | BACKLOG | 2026-09-13 | menu_settings (AES 39): kleinsten Messfall entwerfen |
| G-AES-040 | P3 | BACKLOG | 2026-09-13 | objc_add (AES 40): kleinsten Messfall entwerfen |
| G-AES-041 | P3 | BACKLOG | 2026-09-13 | objc_delete (AES 41): kleinsten Messfall entwerfen |
| G-AES-042 | P3 | BACKLOG | 2026-09-13 | objc_draw (AES 42): kleinsten Messfall entwerfen |
| G-AES-043 | P3 | BACKLOG | 2026-09-13 | objc_find (AES 43): kleinsten Messfall entwerfen |
| G-AES-044 | P3 | BACKLOG | 2026-09-13 | objc_offset (AES 44): kleinsten Messfall entwerfen |
| G-AES-045 | P3 | BACKLOG | 2026-09-13 | objc_order (AES 45): kleinsten Messfall entwerfen |
| G-AES-046 | P3 | BACKLOG | 2026-09-13 | objc_edit (AES 46): kleinsten Messfall entwerfen |
| G-AES-047 | P3 | BACKLOG | 2026-09-13 | objc_change (AES 47): kleinsten Messfall entwerfen |
| G-AES-048 | P3 | BACKLOG | 2026-09-13 | objc_sysvar (AES 48): kleinsten Messfall entwerfen |
| G-AES-050 | P3 | BACKLOG | 2026-09-13 | form_do (AES 50): kleinsten Messfall entwerfen |
| G-AES-051 | P3 | BACKLOG | 2026-09-13 | form_dial (AES 51): kleinsten Messfall entwerfen |
| G-AES-052 | P3 | BACKLOG | 2026-09-13 | form_alert (AES 52): kleinsten Messfall entwerfen |
| G-AES-053 | P3 | BACKLOG | 2026-09-13 | form_error (AES 53): kleinsten Messfall entwerfen |
| G-AES-054 | P3 | BACKLOG | 2026-09-13 | form_center (AES 54): kleinsten Messfall entwerfen |
| G-AES-055 | P3 | BACKLOG | 2026-09-13 | form_keybd (AES 55): kleinsten Messfall entwerfen |
| G-AES-056 | P3 | BACKLOG | 2026-09-13 | form_button (AES 56): kleinsten Messfall entwerfen |
| G-AES-070 | P3 | BACKLOG | 2026-09-13 | graf_rubbox (AES 70): kleinsten Messfall entwerfen |
| G-AES-071 | P3 | BACKLOG | 2026-09-13 | graf_dragbox (AES 71): kleinsten Messfall entwerfen |
| G-AES-072 | P3 | BACKLOG | 2026-09-13 | graf_mbox (AES 72): kleinsten Messfall entwerfen |
| G-AES-073 | P3 | BACKLOG | 2026-09-13 | graf_growbox (AES 73): kleinsten Messfall entwerfen |
| G-AES-074 | P3 | BACKLOG | 2026-09-13 | graf_shrinkbox (AES 74): kleinsten Messfall entwerfen |
| G-AES-075 | P3 | BACKLOG | 2026-09-13 | graf_watchbox (AES 75): kleinsten Messfall entwerfen |
| G-AES-076 | P3 | BACKLOG | 2026-09-13 | graf_slidebox (AES 76): kleinsten Messfall entwerfen |
| G-AES-080 | P3 | BACKLOG | 2026-09-13 | scrp_read (AES 80): kleinsten Messfall entwerfen |
| G-AES-081 | P3 | BACKLOG | 2026-09-13 | scrp_write (AES 81): kleinsten Messfall entwerfen |
| G-AES-082 | P3 | BACKLOG | 2026-09-13 | scrp_clear (AES 82): kleinsten Messfall entwerfen |
| G-AES-090 | P3 | BACKLOG | 2026-09-13 | fsel_input (AES 90): kleinsten Messfall entwerfen |
| G-AES-091 | P3 | BACKLOG | 2026-09-13 | fsel_exinput (AES 91): kleinsten Messfall entwerfen |
| G-AES-107 | P3 | BACKLOG | 2026-09-13 | wind_update (AES 107): kleinsten Messfall entwerfen |
| G-AES-110 | P3 | BACKLOG | 2026-09-13 | rsrc_load (AES 110): kleinsten Messfall entwerfen |
| G-AES-111 | P3 | BACKLOG | 2026-09-13 | rsrc_free (AES 111): kleinsten Messfall entwerfen |
| G-AES-112 | P3 | BACKLOG | 2026-09-13 | rsrc_gaddr (AES 112): kleinsten Messfall entwerfen |
| G-AES-113 | P3 | BACKLOG | 2026-09-13 | rsrc_saddr (AES 113): kleinsten Messfall entwerfen |
| G-AES-114 | P3 | BACKLOG | 2026-09-13 | rsrc_obfix (AES 114): kleinsten Messfall entwerfen |
| G-AES-120 | P3 | BACKLOG | 2026-09-13 | shel_read (AES 120): kleinsten Messfall entwerfen |
| G-AES-121 | P3 | BACKLOG | 2026-09-13 | shel_write (AES 121): kleinsten Messfall entwerfen |
| G-AES-122 | P3 | BACKLOG | 2026-09-13 | shel_get (AES 122): kleinsten Messfall entwerfen |
| G-AES-123 | P3 | BACKLOG | 2026-09-13 | shel_put (AES 123): kleinsten Messfall entwerfen |
| G-AES-124 | P3 | BACKLOG | 2026-09-13 | shel_find (AES 124): kleinsten Messfall entwerfen |
| G-AES-125 | P3 | BACKLOG | 2026-09-13 | shel_envrn (AES 125): kleinsten Messfall entwerfen |
| G-AES-126 | P3 | BACKLOG | 2026-09-13 | shel_rdef (AES 126): kleinsten Messfall entwerfen |
| G-AES-127 | P3 | BACKLOG | 2026-09-13 | shel_wdef (AES 127): kleinsten Messfall entwerfen |
| G-VDI-001 | P3 | BACKLOG | 2026-09-13 | v_opnwk (VDI 1): kleinsten Messfall entwerfen |
| G-VDI-002 | P3 | BACKLOG | 2026-09-13 | v_clswk (VDI 2): kleinsten Messfall entwerfen |
| G-VDI-014 | P3 | BACKLOG | 2026-09-13 | vs_color (VDI 14): kleinsten Messfall entwerfen |
| G-VDI-028 | P3 | BACKLOG | 2026-09-13 | v_locator (VDI 28): kleinsten Messfall entwerfen |
| G-VDI-031 | P3 | BACKLOG | 2026-09-13 | v_string (VDI 31): kleinsten Messfall entwerfen |
| G-VDI-101 | P3 | BACKLOG | 2026-09-13 | v_clsvwk (VDI 101): kleinsten Messfall entwerfen |
| G-VDI-112 | P3 | BACKLOG | 2026-09-13 | vsf_udpat (VDI 112): kleinsten Messfall entwerfen |
| G-VDI-118 | P3 | BACKLOG | 2026-09-13 | vex_timv (VDI 118): kleinsten Messfall entwerfen |
| G-VDI-119 | P3 | BACKLOG | 2026-09-13 | vst_load_fonts (VDI 119): kleinsten Messfall entwerfen |
| G-VDI-120 | P3 | BACKLOG | 2026-09-13 | vst_unload_fonts (VDI 120): kleinsten Messfall entwerfen |
| G-VDI-124 | P3 | BACKLOG | 2026-09-13 | vq_mouse (VDI 124): kleinsten Messfall entwerfen |
| G-VDI-125 | P3 | BACKLOG | 2026-09-13 | vex_butv (VDI 125): kleinsten Messfall entwerfen |
| G-VDI-126 | P3 | BACKLOG | 2026-09-13 | vex_motv (VDI 126): kleinsten Messfall entwerfen |
| G-VDI-127 | P3 | BACKLOG | 2026-09-13 | vex_curv (VDI 127): kleinsten Messfall entwerfen |
| G-VDI-128 | P3 | BACKLOG | 2026-09-13 | vq_key_s (VDI 128): kleinsten Messfall entwerfen |
| G-VDI-134 | P3 | BACKLOG | 2026-09-13 | vex_wheelv (VDI 134): kleinsten Messfall entwerfen |
| NET06 | P4 | BACKLOG | 2026-09-23 | WLAN (CYW43455 ueber SDIO) |

Summe gesamt: 29 angefangen (A–C) + 59 offen (D) + 13 geparkt/zurueckgestellt/Idee/Zukunft (E) + 81 Backlog (F) = 182 Tickets.

Nicht aufgenommen: Tickets mit Status ERLEDIGT, BEHOBEN, GEKLAERT, GESCHLOSSEN, GEGENSTANDSLOS, ZURUECKGEZOGEN (auch wenn eine Pi-Abnahme noch fehlt, z. B. A111, A115, A117) sowie die Sammeldateien COMPARISON, GAPS-INDEX, HANDOVER, RSC-AUDIT-INDEX.

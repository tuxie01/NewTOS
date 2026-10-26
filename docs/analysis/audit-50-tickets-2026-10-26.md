# Audit-Liste: die 50 zuletzt bearbeiteten Tickets (2026-10-26)

Grundlage fuer einen 50-Ticket-Audit. Auswahl: alle Tickets unter `TASK/`
gleich welchen Status, geordnet nach dem letzten Commit, der das Ticket oder
seine Quittung (`-STATUS-DONE.MD` / `-STATUS-UNFINISHED.MD`) beruehrt hat
(Stand `e7259088`). Status = Statuszeile des Tickets.

Hinweise:
- Das Datum ist die letzte Beruehrung, nicht der Abschluss; ein Nachtrag
  zaehlt mit.
- A118: Umbau auf EmuTOS 1:1 fuer den nach dem Warten abgelehnten
  Fensteraufruf steht noch aus (`TASK/A118.md`, „Offen nach dem Nachlauf“).
- CON04: Ticket steht auf ERLEDIGT; die alte UNFINISHED-Quittung ist
  ueberholt.
- Die Sammelschliessungen `feb76357` und `526ad8ac` (2026-10-17) haben 21
  Tickets formal geschlossen, deren Arbeit oft frueher lag; nur die hier
  gelisteten sind im Umfang.
- Erzeugt mit `git log --name-only -- TASK`, erster Treffer je Ticket-ID.

| # | Datum | Ticket | Status | Thema |
|---|---|---|---|---|
| 1 | 2026-10-26 | A118 | ERLEDIGT (Umbau offen) | `wind_update`-Sperre, restliche Nehmer |
| 2 | 2026-10-25 | USB01 | OFFEN | USB-Maus, Steckkombinationen, Diagnosewerkzeug |
| 3 | 2026-10-25 | USB03 | OFFEN | Mausrad im Report-Protokoll (B100, #8) |
| 4 | 2026-10-25 | A124 | OFFEN | Testluecke A115, `selftest_sleepwake_switch` verrottet |
| 5 | 2026-10-25 | A115 | ERLEDIGT | Scheduler nach `rlr`/`drl` |
| 6 | 2026-10-25 | A111 | ERLEDIGT | Klickzaehlung und Cursor |
| 7 | 2026-10-25 | A122 | OFFEN | `edit_char`: Tastenwort und Zeichenbereich |
| 8 | 2026-10-25 | A123 | OFFEN | G_FTEXT ohne Fuellung, Glyphenreste |
| 9 | 2026-10-25 | DESK68 | OFFEN | „Neuer Ordner“ ist nicht ADMKDBOX |
| 10 | 2026-10-25 | A121 | OFFEN | AC_OPEN ohne `perform_untop(gl_wtop)` |
| 11 | 2026-10-25 | A117 | ERLEDIGT | MN_SELECTED an den Menue-Besitzer |
| 12 | 2026-10-24 | USB02 | OFFEN | Kleinbefunde USB01 (Punkte 10/11 erledigt) |
| 13 | 2026-10-23 | SND01 | OFFEN | Falcon-DMA-Sound als API, HDMI als Ausgang |
| 14 | 2026-10-23 | H04 | GEPARKT | Systemklingel, Tastenklick, `Dosound` |
| 15 | 2026-10-23 | SND02 | OFFEN | HDMI-Ton, Quellenlage |
| 16 | 2026-10-23 | CON04 | ERLEDIGT | `Bconout(2)` wertet VT52 aus |
| 17 | 2026-10-22 | CON05 | OFFEN | Absturzbericht sendet `ESC f ESC v` seriell |
| 18 | 2026-10-22 | T17 | OFFEN | `vdi_reference` Fall 167 reihenfolgeabhaengig |
| 19 | 2026-10-22 | SDK01 | ERLEDIGT | SDK fuer Fremde benutzbar |
| 20 | 2026-10-22 | CON03 | OFFEN | Kernel-Konsole ist nicht EmuCON |
| 21 | 2026-10-22 | A120 | OFFEN | Nachrichten-Queue, Reste nach A116 |
| 22 | 2026-10-22 | A116 | ERLEDIGT | `appl_write` auf volle Queue |
| 23 | 2026-10-22 | LIBC-BUF | ERLEDIGT | erster `printf`: 1-KiB-Puffer |
| 24 | 2026-10-22 | DESK39 | ERLEDIGT | `inf_file_folder` |
| 25 | 2026-10-22 | GUI01 | GESCHLOSSEN | `CONF_WITH_SHOW_FILE` |
| 26 | 2026-10-22 | A112 | ERLEDIGT | `evnt_multi` weckt nicht auf Nachricht/Timer |
| 27 | 2026-10-22 | A119 | OFFEN | `wind_set(WF_INFO/WF_NAME)` zeichnet ganzen Rahmen |
| 28 | 2026-10-22 | ARCH02 | ERLEDIGT | `Pexec`-Eltern verlieren FP/SIMD-Register |
| 29 | 2026-10-22 | DESK48 | ERLEDIGT | vier `redraw_desktop()`-Stellen |
| 30 | 2026-10-22 | GH1 | ERLEDIGT | Popup im System-Info-Accessory |
| 31 | 2026-10-22 | REF04 | ERLEDIGT | Tabelle 4-42 in `emmc-sd.md` und Lizenz |
| 32 | 2026-10-22 | REF05 | OFFEN | GIC-Quellen beschaffen |
| 33 | 2026-10-22 | RST01 | ERLEDIGT | Warm-/Kaltstart, Ctrl+Alt+Del |
| 34 | 2026-10-22 | MEM01 | ERLEDIGT | `Mxalloc(-1)` ST/TT gleich |
| 35 | 2026-10-22 | FS08 | ERLEDIGT | Bilddateien auf der Pi langsam |
| 36 | 2026-10-21 | ACC07 | ERLEDIGT | Accessory-Titel im Menuebaum |
| 37 | 2026-10-21 | BOOT01 | OFFEN | Bootschirm auf hohen Aufloesungen |
| 38 | 2026-10-21 | DESK43 | ERLEDIGT | Desktop im User-Space |
| 39 | 2026-10-21 | DESK45 | ERLEDIGT | Desk-Selftests in keinem Startpfad |
| 40 | 2026-10-21 | FS02 | ERLEDIGT | `Frename` Langnamen-Kette |
| 41 | 2026-10-21 | FS07 | ERLEDIGT | `Fseek` auf Konsolen-Handle |
| 42 | 2026-10-21 | FS17 | OFFEN | GEMDOS-Datei-Abweichungen aus PROC04 |
| 43 | 2026-10-21 | HW04 | OFFEN | EMMC-Initialisierung gegen Spezifikation |
| 44 | 2026-10-21 | MEM03 | ERLEDIGT | Umgebungsblock des Kindes in EL0 |
| 45 | 2026-10-21 | PORT11 | TEILWEISE | SDK-Doku nach E9 |
| 46 | 2026-10-21 | T08 | TEILWEISE | VDI `vex_*` Vektortausch |
| 47 | 2026-10-21 | T10 | TEILWEISE | Injektor waehlt das leere Skript |
| 48 | 2026-10-20 | NET01 | OFFEN | Netzwerk STinG/STiK (Ziel 1.2) |
| 49 | 2026-10-20 | COOK01 | ZURUECKGESTELLT | `_MCH` = ARAnyM-Kennung |
| 50 | 2026-10-19 | ARCH04 | OFFEN | CPU-Eigenschaften aus `ID_AA64*_EL1` |

Summe: 25 erledigt/geschlossen, 20 offen, 3 teilweise, 2 geparkt/zurueckgestellt.

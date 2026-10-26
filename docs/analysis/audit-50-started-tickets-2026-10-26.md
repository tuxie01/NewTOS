# Audit-Liste: die 50 zuletzt angelegten Tickets (2026-10-26)

Ergaenzung zu [audit-50-tickets-2026-10-26.md](audit-50-tickets-2026-10-26.md)
(dort: zuletzt bearbeitet). Auswahl hier: die 50 Tickets unter `TASK/`, deren
`<ID>.md` zuletzt angelegt wurde, gleich welchen Status, neueste zuerst
(Stand `66ba9aa1`). Datum und Commit = Anlage-Commit; Status = Statuszeile
des Tickets heute.

Hinweise:
- A118: Umbau auf EmuTOS 1:1 fuer den nach dem Warten abgelehnten
  Fensteraufruf steht noch aus (`TASK/A118.md`, „Offen nach dem Nachlauf“).
- USB01/USB02: Teilschritte erledigt (USB01 F1–F3, USB02 Punkte 10/11),
  Ticket offen.
- Erzeugt mit `git log --diff-filter=A --name-only -- 'TASK/*.md'`.

| # | Angelegt | Commit | Ticket | Status | Thema |
|---|---|---|---|---|---|
| 1 | 2026-10-25 | `1d1187d3` | USB03 | OFFEN | Mausrad von Kabelmaeusen, die es nur im Report-Protokoll schicken (B100, GitHub #8) |
| 2 | 2026-10-25 | `17cab8b6` | A124 | OFFEN | `sched_a115_ready_list` unterscheidet dlr- von Slot-Reihenfolge nicht; `selftest_sleepwake_switch` verrottet |
| 3 | 2026-10-25 | `ad9b023d` | A122 | OFFEN | `edit_char`: Tastenwort und Zeichenbereich weichen von EmuTOS ab |
| 4 | 2026-10-25 | `ad9b023d` | A123 | OFFEN | G_FTEXT ohne Fuellung: alte Glyphenpixel bleiben nach Textaenderung stehen |
| 5 | 2026-10-25 | `ad9b023d` | DESK68 | OFFEN | „Neuer Ordner“-Dialog ist nicht EmuTOS' ADMKDBOX |
| 6 | 2026-10-25 | `8da6e860` | A121 | OFFEN | AC_OPEN ohne `perform_untop(gl_wtop)`; veraltete Single-app-Notiz in `_state.md` |
| 7 | 2026-10-23 | `03ae70c8` | USB02 | OFFEN | Kleinbefunde aus USB01 F1/F2/F8 |
| 8 | 2026-10-22 | `5b66f1f3` | USB01 | OFFEN | USB-Maus: nur Boot-Protokoll, Steckkombinationen, Diagnosewerkzeug |
| 9 | 2026-10-22 | `f9fdd32c` | CON05 | OFFEN | Absturzbericht sendet `ESC f ESC v` auch auf die serielle Schnittstelle |
| 10 | 2026-10-22 | `49249a2d` | T17 | OFFEN | `vdi_reference` Fall 167 haengt von der Testreihenfolge ab |
| 11 | 2026-10-22 | `1d7a4c0b` | CON04 | ERLEDIGT | `Bconout(2)` wertet keine VT52-Folgen aus |
| 12 | 2026-10-22 | `02fd1b8d` | A120 | OFFEN | Nachrichten-Queue: Reste nach A116 (wartender Leser, Ketten-Queue, veraltete Spec) |
| 13 | 2026-10-22 | `22325b9a` | A119 | OFFEN | `wind_set(WF_INFO/WF_NAME)` zeichnet den ganzen Rahmen statt nur des Elements |
| 14 | 2026-10-22 | `22325b9a` | REF05 | OFFEN | GIC-Quellen beschaffen (ARM IHI 0048B, DDI 0471B) |
| 15 | 2026-10-21 | `d3410813` | A117 | ERLEDIGT | `menu_intercept` stellt MN_SELECTED an die aktuelle Identitaet statt an den Menue-Besitzer |
| 16 | 2026-10-21 | `d3410813` | A118 | ERLEDIGT (Umbau offen) | Restliche Nehmer der `wind_update`-Sperre nach EmuTOS |
| 17 | 2026-10-20 | `7d5a3f08` | SDK01 | ERLEDIGT | SDK fuer Fremde benutzbar machen: libc-Luecken und Portierungs-Doku |
| 18 | 2026-10-19 | `3720d175` | COOK01 | ZURUECKGESTELLT | `_MCH` = 0x00050000 ist die ARAnyM-Kennung |
| 19 | 2026-10-19 | `3084efa9` | GH1 | ERLEDIGT | Popup im System-Info-Accessory: Fenster dahinter zeichnet sich ueber den Dialog |
| 20 | 2026-10-19 | `2b559b3e` | ARCH04 | OFFEN | CPU-Eigenschaften zur Laufzeit aus `ID_AA64*_EL1` lesen |
| 21 | 2026-10-19 | `22884fcf` | FS18 | ERLEDIGT | `Frename` einer offenen Datei: Laenge 0 und verlorene Cluster |
| 22 | 2026-10-19 | `3bba0533` | FS17 | OFFEN | Kleinere GEMDOS-Datei-Abweichungen aus PROC04 Punkt 5 |
| 23 | 2026-10-18 | `aa0443d9` | FS16 | OFFEN | `Fwrite` schreibt den Sektorcache nach jedem Aufruf zurueck |
| 24 | 2026-10-18 | `8fc3719f` | CON03 | OFFEN | Die Kernel-Konsole ist nicht EmuCON: Befehlssatz und Ort |
| 25 | 2026-10-18 | `097b7f99` | FS15 | OFFEN | Geraetenamen `CON:`, `AUX:`, `PRN:` bei `Fopen`/`Fcreate` |
| 26 | 2026-10-17 | `51504c24` | INPUT02 | OFFEN | Ctrl+Alt+Del erreicht ein Programm in einer Endlosschleife nicht |
| 27 | 2026-10-17 | `6d7e7d93` | RST02 | OFFEN | Warmstart EmuTOS-treu: Kern-Neueinstieg mit erhaltenen Systemvariablen |
| 28 | 2026-10-17 | `828f9efa` | A116 | ERLEDIGT | `appl_write` auf volle Queue eines anderen Prozesses: EmuTOS blockiert, NewTOS gibt 0 |
| 29 | 2026-10-17 | `bf200fe3` | A115 | ERLEDIGT | Scheduler waehlt nach Slot-Reihenfolge statt EmuTOS' `rlr`-Liste |
| 30 | 2026-10-17 | `d22bca5a` | A114 | ERLEDIGT | Accessories sehen beim Programmstart kein `AC_CLOSE`? |
| 31 | 2026-10-17 | `d22bca5a` | DESK67 | OFFEN | Reste aus DESK48: Teilneuzeichnen und `fun_rebld` vollstaendig wie EmuTOS |
| 32 | 2026-10-17 | `fe5ba8f9` | A113 | ERLEDIGT | Accessories melden ap_id 0: `appl_write` erreicht kein anderes Programm |
| 33 | 2026-10-17 | `1a920bda` | A112 | ERLEDIGT | `evnt_multi` schlaeft nur auf Eingabe: Nachricht und Timer wecken nicht, wenn ein anderer Prozess laeuft |
| 34 | 2026-10-17 | `526ad8ac` | FS14 | ERLEDIGT | `Frename` ueber Verzeichnisgrenzen auf einen leeren Namen |
| 35 | 2026-10-17 | `f4c88f96` | RST01 | ERLEDIGT | Neustart: Warm- und Kaltstart, Ctrl+Alt+Del |
| 36 | 2026-10-17 | `55ac3578` | X04 | ERLEDIGT | TeraDesk „Reboot“ springt auf Adresse 0 (Absturzbericht statt Neustart) |
| 37 | 2026-10-17 | `1e20f80c` | DESK66 | OFFEN | Desktop-Hintergrund an die AES melden (`set_aes_background`) |
| 38 | 2026-10-17 | `b6d80712` | DESK65 | OFFEN | Einstellungen des Desktops sofort speichern (Erweiterung) |
| 39 | 2026-10-16 | `58430906` | DESK64 | ERLEDIGT | Hintergrund-Dialog: zeigt EmuTOS eine Vorschau von Farbe und Muster? |
| 40 | 2026-10-16 | `ca53d034` | DESK63 | OFFEN | Desktop-Verknuepfung auf Programm-Verknuepfung: „illegale Operation“ statt Start |
| 41 | 2026-10-16 | `d227902f` | DESK62 | ERLEDIGT | Sanduhr bleibt nach Drop einer Datei auf eine Desktop-Verknuepfung |
| 42 | 2026-10-16 | `67878ccd` | CON02 | ERLEDIGT | `Cconout`/`Cconws` folgen nicht der Umlenkung von Handle 1 |
| 43 | 2026-10-16 | `67878ccd` | FS13 | ERLEDIGT | `Fclose` nach `Fforce(n,-1)` liefert -65 |
| 44 | 2026-10-16 | `67878ccd` | LIBC-BUF | ERLEDIGT | erster `printf` auf ungepuffertes stdout legt nur 1 KiB Puffer an |
| 45 | 2026-10-16 | `3b809841` | FS12 | OFFEN | Laufwerksgrenze 16 statt 26 in BIOS und GEMDOS |
| 46 | 2026-10-16 | `d890e322` | DESK61 | OFFEN | `act_chg` setzt `f_selected` bei unveraendertem Zustand nicht |
| 47 | 2026-10-15 | `52c9b447` | DESK60 | ERLEDIGT | Shell meldet „starting DESKTOP.NAP“ nach einem `#Z`-gestarteten Programm |
| 48 | 2026-10-15 | `23e6e7b0` | T16 | ERLEDIGT | `tools/inject-arm.py`: `launch:`-Ausloeser verpasst ein Kind im selben Slot |
| 49 | 2026-10-14 | `acb0ff45` | PROC06 | ERLEDIGT | `Pexec` liefert nach Ctrl-C `-32` vorzeichenerweitert statt 65504 |
| 50 | 2026-10-14 | `a4064115` | DESK59 | ERLEDIGT | `act_chg` prueft `ob_flags & DISABLED` statt `ob_state` |

Summe: 23 erledigt, 26 offen, 1 zurueckgestellt.

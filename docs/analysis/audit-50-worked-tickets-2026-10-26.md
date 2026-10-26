# Audit-Liste: die 50 zuletzt bearbeiteten Tickets mit echter Arbeit (2026-10-26)

Massgebliche Liste fuer den 50-Ticket-Audit. Ersetzt als Auswahl die beiden
Vorlaeufer [audit-50-tickets](audit-50-tickets-2026-10-26.md) (zuletzt
beruehrt, auch unbearbeitete) und
[audit-50-started-tickets](audit-50-started-tickets-2026-10-26.md) (zuletzt
angelegt, auch unbearbeitete).

Auswahlregel (Stand `5af780fc`):
- Aufgenommen: Status ERLEDIGT oder TEILWEISE, oder Ticket offen, aber mit
  Quittung (`-STATUS-*.MD`) oder eigenem Messlauf unter `measurements/`
  (Arbeit begonnen: USB01, USB02).
- Nicht aufgenommen: offene, geparkte, zurueckgestellte Tickets ohne Quittung
  und ohne Messlauf (noch nicht angefangen) sowie GESCHLOSSEN ohne Umsetzung
  (GUI01 Anwenderentscheid, BALL01 Demo wird neu gebaut).
- Reihenfolge: letzter Commit, der das Ticket oder seine Quittung beruehrt hat.
  Bei Nachtraegen liegt die Arbeit frueher (z. B. DESK39 2026-09-25, REF04
  2026-09-29; die Sammelschliessungen `feb76357`/`526ad8ac` am 2026-10-17).
- Messlaeufe = Anzahl Verzeichnisse `measurements/measurement-*-<id>*`.

Hinweise: A118 Umbau auf EmuTOS 1:1 offen (`TASK/A118.md`); USB01 F1–F3 und
USB02 Punkte 10/11 erledigt, Rest offen.

| # | Zuletzt | Ticket | Status | Messlaeufe | Thema |
|---|---|---|---|---|---|
| 1 | 2026-10-26 | A118 | ERLEDIGT (Umbau offen) | 4 | Restliche Nehmer der `wind_update`-Sperre nach EmuTOS |
| 2 | 2026-10-25 | USB01 | OFFEN (angefangen) | 14 | USB-Maus: nur Boot-Protokoll, Steckkombinationen, Diagnosewerkzeug |
| 3 | 2026-10-25 | A115 | ERLEDIGT | 2 | Scheduler waehlt nach Slot-Reihenfolge statt EmuTOS' `rlr`-Liste |
| 4 | 2026-10-25 | A111 | ERLEDIGT | 2 | Click counting and caret drawing still differ from EmuTOS |
| 5 | 2026-10-25 | A117 | ERLEDIGT | 2 | `menu_intercept` stellt MN_SELECTED an die aktuelle Identitaet statt an den Menue-Besitzer |
| 6 | 2026-10-24 | USB02 | OFFEN (angefangen) | 4 | Kleinbefunde aus USB01 F1/F2/F8 |
| 7 | 2026-10-23 | CON04 | ERLEDIGT | 4 | `Bconout(2)` wertet keine VT52-Folgen aus |
| 8 | 2026-10-22 | SDK01 | ERLEDIGT | 1 | SDK fuer Fremde benutzbar machen: libc-Luecken und Portierungs-Doku |
| 9 | 2026-10-22 | A116 | ERLEDIGT | 1 | `appl_write` auf volle Queue eines anderen Prozesses: EmuTOS blockiert, NewTOS gibt 0 |
| 10 | 2026-10-22 | LIBC-BUF | ERLEDIGT | 1 | erster `printf` auf ungepuffertes stdout legt nur 1 KiB Puffer an |
| 11 | 2026-10-22 | DESK39 | ERLEDIGT | 0 | `inf_file_folder` benennt nicht selbst um und zaehlt Ordner nicht |
| 12 | 2026-10-22 | A112 | ERLEDIGT | 4 | `evnt_multi` schlaeft nur auf Eingabe: Nachricht und Timer wecken nicht, wenn ein anderer Prozess laeuft |
| 13 | 2026-10-22 | ARCH02 | ERLEDIGT | 1 | `Pexec`-Eltern verlieren ihre FP/SIMD-Register |
| 14 | 2026-10-22 | DESK48 | ERLEDIGT | 1 | Vier weitere `redraw_desktop()`-Stellen, die EmuTOS nicht hat |
| 15 | 2026-10-22 | GH1 | ERLEDIGT | 4 | Popup im System-Info-Accessory: Fenster dahinter zeichnet sich ueber den Dialog |
| 16 | 2026-10-22 | REF04 | ERLEDIGT | 0 | Die woertliche Tabelle 4-42 in `emmc-sd.md` gegen die Lizenzbedingungen |
| 17 | 2026-10-22 | RST01 | ERLEDIGT | 2 | Neustart: Warm- und Kaltstart, Ctrl+Alt+Del |
| 18 | 2026-10-22 | MEM01 | ERLEDIGT | 0 | `Mxalloc(-1)` meldet fuer ST-RAM und TT-RAM denselben Wert |
| 19 | 2026-10-22 | FS08 | ERLEDIGT | 4 | Reading image files on the real Pi is extremely slow |
| 20 | 2026-10-21 | ACC07 | ERLEDIGT | 0 | Accessory-Titel im Menuebaum einer Anwendung |
| 21 | 2026-10-21 | DESK43 | ERLEDIGT | 17 | Der Desktop muss im User-Space laufen, wie jede andere Anwendung |
| 22 | 2026-10-21 | DESK45 | ERLEDIGT | 0 | Die vier Selbsttests in `kernel/src/desk/` haengen in keinem Startpfad |
| 23 | 2026-10-21 | FS02 | ERLEDIGT | 0 | `Frename` gibt die Langnamen-Kette des Quellnamens nicht frei |
| 24 | 2026-10-21 | FS07 | ERLEDIGT | 0 | `Fseek` auf einem Konsolen-Handle liefert `EIHNDL`, EmuTOS 0 |
| 25 | 2026-10-21 | MEM03 | ERLEDIGT | 0 | Der Umgebungsblock eines Kindes ist in EL0 nicht abgebildet; `p_env` traegt eine physische Adresse |
| 26 | 2026-10-21 | PORT11 | TEILWEISE | 0 | SDK-Doku auf den Stand nach E9 bringen (`malloc` ueber `Malloc`, crt0) |
| 27 | 2026-10-21 | T08 | TEILWEISE | 0 | VDI `vex_*`: Vektortausch in `CONTRL[7..]`/`[9..]` fest 8 Byte an festem Index |
| 28 | 2026-10-21 | T10 | TEILWEISE | 0 | Die duale Injektor-Bewaffnung waehlt seit den Accessories immer das leere Skript |
| 29 | 2026-10-19 | FS14 | ERLEDIGT | 0 | `Frename` ueber Verzeichnisgrenzen auf einen leeren Namen |
| 30 | 2026-10-19 | FS18 | ERLEDIGT | 2 | `Frename` einer offenen Datei: Laenge 0 und verlorene Cluster |
| 31 | 2026-10-19 | MEM07 | ERLEDIGT | 0 | feste Grenze `MAX_SHARED_BLOCKS = 64` |
| 32 | 2026-10-19 | PROC03 | ERLEDIGT | 5 | Desktop/AES als echter EL0-Prozess mit eigenem PD (`p_parent` exakt wie EmuTOS) |
| 33 | 2026-10-19 | PROC04 | ERLEDIGT | 8 | Remaining GEMDOS handle deviations found during PROC02 |
| 34 | 2026-10-18 | CON01 | ERLEDIGT | 2 | Spaltenzaehler (`glbcolumn`) und BDOS-Typeahead fuer die Konsole |
| 35 | 2026-10-18 | CON02 | ERLEDIGT | 0 | `Cconout`/`Cconws` folgen nicht der Umlenkung von Handle 1 |
| 36 | 2026-10-17 | X04 | ERLEDIGT | 1 | TeraDesk „Reboot“ springt auf Adresse 0 (Absturzbericht statt Neustart) |
| 37 | 2026-10-17 | X01 | ERLEDIGT | 2 | Supexec: brauchen wir es, und was kostet es auf AArch64? |
| 38 | 2026-10-17 | M08 | ERLEDIGT | 0 | mkrusttests kennt die Pufferspalte nicht |
| 39 | 2026-10-17 | A113 | ERLEDIGT | 3 | Accessories melden ap_id 0: `appl_write` erreicht kein anderes Programm |
| 40 | 2026-10-17 | A114 | ERLEDIGT | 1 | Accessories sehen beim Programmstart kein `AC_CLOSE`? |
| 41 | 2026-10-17 | A25 | ERLEDIGT | 1 | appl_init: Wiederaufruf erhält Nachrichten und Flags |
| 42 | 2026-10-17 | MEM02 | ERLEDIGT | 2 | `Mxalloc` mit `MX_PREFTTRAM` liefert ST-RAM |
| 43 | 2026-10-17 | PORT06 | ERLEDIGT | 4 | Doom portieren: konform ueber BIOS, AES und VDI |
| 44 | 2026-10-17 | DESK56 | ERLEDIGT | 0 | Zwei Wurfpfade in `dir.rs` verwerfen die Rueckgabe von `do_aopen_drop` |
| 45 | 2026-10-17 | FS05 | ERLEDIGT | 0 | `fwrite` laeuft die Clusterkette je Sektor von vorn |
| 46 | 2026-10-17 | FS06 | ERLEDIGT | 0 | `Frename` auf einen leeren Namen liefert `E_OK` und legt einen leeren Verzeichniseintrag an |
| 47 | 2026-10-17 | FS13 | ERLEDIGT | 0 | `Fclose` nach `Fforce(n,-1)` liefert -65 |
| 48 | 2026-10-17 | MEM09 | ERLEDIGT | 0 | `memprot::on_alloc` ignoriert den Rueckgabewert von `record` |
| 49 | 2026-10-17 | PORT02 | ERLEDIGT | 0 | Welche Anwendungen koennen wir bis zum Termin portieren? |
| 50 | 2026-10-17 | DESK64 | ERLEDIGT | 1 | Hintergrund-Dialog: zeigt EmuTOS eine Vorschau von Farbe und Muster? |

Summe: 45 erledigt, 3 teilweise, 2 offen und angefangen.

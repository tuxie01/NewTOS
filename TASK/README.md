# Messvergleich und kleine Entwickleraufgaben — 2026-09-09

## Ticket states (ADR-037)

Every ticket `TASK/<ID>.md` carries its state on line 3 in the form of ADR-037 (`docs/MEMORY.md`): one of the six
states `OPEN`, `IMPLEMENTED`, `DONE`, `PARKED`, `ACCEPTED`, `OBSOLETE` with the fields that state requires. The
**Status** column of the tables below is generated from line 3 by `cargo xtask report --readme` (`--check` tells
whether it is stale); a legacy ticket, written before ADR-037, shows its raw status text. Do not edit the column by
hand. The tools (TOOL41, `tools/xtask/src/tickets.rs`): `cargo xtask status-check` (the gap check), `cargo xtask
accept` (the tester), `cargo xtask set-status` (the lead), `cargo xtask report`.
A receipt records role and agent in its header, the lines before its first `## ` heading: `Role: developer` or
`Role: tester`, and `Agent: <name>`; `accept` refuses an acceptor who is a recorded developer of the ticket.

## Einstieg

> **Hinweis für den laufenden Audit — 2026-09-10:** Im gemeinsamen Arbeitsbaum liegt bereits der **uncommitted A25-Fix** zu `appl_init`: [pd.rs](../tos-api/src/aes/pd.rs), [neuer Dispatcher-Test](../tos-api/tests/aes_appl_init.rs) und [aktualisierte Spezifikation](../docs/api/aes/application.md). `pd::open` erhält bei bestehender PID Queue und Flags; neue Slots bleiben leer initialisiert. Nicht als fremden Zwischenstand versehentlich zurücksetzen oder durch eine alte Dateikopie überschreiben. Den Fix bitte unabhängig gegen `EmuTOS aes/gemaplib.c:54–63` prüfen; [Vorher-/Nachher-Beleg und Status](A25-STATUS-UNFINISHED.MD) liegen vor. **UNFINISHED bedeutet hier: Hardwareabnahme fehlt, nicht: Implementierung fehlt.** Keine Änderungen an A13/A14 durch diesen Fix.

**135 einzelne Aufgaben (einschließlich A25). Die verbindlichen Einzelstände stehen in den Task- und STATUS-Dateien; die frühere Gesamtzählung war nach den neuen Statusmeldungen veraltet. Dies ist keine Fehleranzahl.** Die offenen Aufgaben umfassen Prüfungen, Hardwareaufnahmen, Messdesign und bedingte Reparaturen. Jede Datei beschreibt einen begrenzten Auftrag, Abhängigkeiten, Quellen und eine überprüfbare Abnahme. Seit 2026-09-10 enthält A25 eine begrenzte Implementierungskorrektur; die übrigen Befunde wurden davon nicht verändert.

- [Messgegenüberstellung](COMPARISON.md): Zahlen, Einordnung und Zuordnung jedes abweichenden Falls.
- [Funktionsabdeckung](COVERAGE.md): alle 79 AES- und 73 benannten VDI-Hauptfunktionen des vorhandenen Inventars; Subopcode-Vollständigkeit wird nicht behauptet.
- [Validierung](evidence/validation.txt), [SHA-256-Manifest](evidence/manifest.json), [maschinenlesbare Tasks](tasks.json).
- Rohvergleiche: [AES](evidence/aes-full.txt), [VDI](evidence/vdi-full.txt), [Raster](evidence/raster-full.txt).

## Verbindliche Reihenfolge — Benutzerentscheidung

Systematisch nach API-Opcode: **AES ab appl_init (10)**, danach appl_read (11) usw.; erst anschließend VDI ab v_opnwk (1). Prioritäten der ursprünglichen Themenliste ändern diese Reihenfolge nicht. Bereits erledigte Aufgaben bleiben erledigt, sie werden nicht wiederholt.

Aktuell: appl_init, A06–A09 und der dabei gefundene Wiederaufruffehler [A25](A25.md). A06/A08/A09 sind geprüft, A07 wartet auf die archivierte Laufkonfiguration; A25 ist implementiert und hostverifiziert, die reale Gegenprobe fehlt. Deshalb Opcode 10 noch nicht vollständig abgeschlossen und Opcode 11 noch nicht begonnen.

Die neu abgelegten Statusdateien haben Vorrang vor der alten Startempfehlung: M01–M04 und A10–A12 sind erledigt. A13/A14 sind analysiert, aber nicht repariert. Neue Messbasis zusätzlich zu run1: run2-pi4-r2wind und run3-pi4-rlist. M04 ist ein fertiges **Konzept**, keine implementierte Kernelidentität. Der FAT-Dateizeitstempel ist nur ein Indiz, keine Compile-Zeit.

## Gemeinsame Arbeitsregeln

- Aktuelle Messbasis: `measurements/measurement-2026-09-09/run1-pi4/`, `run2-pi4-r2wind/` und `run3-pi4-rlist/`. Ältere NewTOS-Dumps sind Historie, kein Beweis eines heutigen Fehlers. Referenzen stammen aus `tosdump/reference/EMUTOS14-*` vom Atari TT.
- Jede Änderung bekommt zuerst ein belegtes Soll aus `docs/api/`, bei Lücken aus der gepinnten EmuTOS-Quelle gemäß `docs/SOURCE-ANALYSIS-PROTOCOL.md`. Unterschiede zur tatsächlich gemessenen EmuTOS-Version separat behandeln.
- 1920×1080 ist das aktuelle Hardwareprofil. Breite, Höhe, Farbtiefe und Fontmetriken bleiben Geräteparameter. Tests dürfen ein zweites Profil benutzen; kein Task verlangt einen neuen Hardwaremodus. 256 logische Farbstifte sind nicht gleichbedeutend mit 8-Bit-Pixelspeicher: der Lauf meldet 16 Planes.
- „Anderer Wert“ bedeutet nicht automatisch „Fehler“. Globals und Handles nie pauschal wegfiltern. Alternative Erwartung jeweils mit Quelle, Geräteprofil und eigener Prüfung begründen. Rohdiff unverändert behalten.
- Bestehende CRC-gebundene Tabellen und BIN/LOG-Dateien nicht überschreiben oder kommentierend ändern. Neue Fälle in neue Tabellen und neue Läufe in eigene Verzeichnisse. Ein Testfall wird durch Tabellen-CRC + Fallnummer identifiziert, nicht nur durch die Nummer.
- Taskumfang auf die benannte Funktion/Unterfunktion begrenzen. Wenn ein gemeinsames Substrat fehlt, einen eigenen kleinen abhängigen Task anlegen; keinen Sammelrefactor. Ein unabhängiger zusätzlicher Defekt bekommt ebenfalls einen eigenen Task.
- Ein Messdesign-Task endet mit einem konkreten Entwurf, nicht mit einem ungetesteten Kernelpatch. Zustandsreset, Rückbau, Referenzverfügbarkeit und blockierende Aufrufe klären, bevor ein Hardwarefall gebaut/ausgeführt wird.
- Für einen bestätigten Defekt: sinnvoller Test vor Fix, minimaler Fix, betroffene bestehende Tests, anschließend erforderliche reale Gegenmessung. Ein grüner Hosttest ersetzt keine noch offene Hardwareabnahme.
- Bestehende Entscheidungen nicht still übergehen: R01 ist aufgrund D-VDI-25 zurückgestellt. Die Erstellung dieser Taskliste ist keine Freigabe, diese Entscheidung umzukehren.
- Jeder bearbeitete Task bekommt zusätzlich ein Quittungsticket `<ID>-STATUS-<ZUSTAND>.MD` — Zustand ist `DONE`, `FAILURE` oder `UNFINISHED`. Darin steht, was tatsächlich getan wurde, welche Artefakte entstanden sind, welche Prüfung lief und was offen bleibt. Ein Ticket entsteht nur für bearbeitete Tasks; ein unberührter Task hat keins.
- Abschluss in der jeweiligen Taskdatei: Status, Commit/Artefakt, tatsächlich ausgeführte Prüfung, Ergebnis und offene Hardwareabnahme. Ein bereits behobener Fall wird mit Messbeleg geschlossen, ohne neuen Patch.

## Konkrete Aufgaben

| ID | Aufgabe | Typ | Priorität | Status | Abhängigkeiten |
|---|---|---|---|---|---|
| [M01](M01.md) | Aktuellen Pi-Fensterlauf archivieren | Hardwaremessung | P1 | ERLEDIGT | — |
| [M02](M02.md) | Rechtecklisten-Referenz auf Atari TT aufnehmen | Hardwaremessung | P1 | ERLEDIGT | — |
| [M03](M03.md) | Rechtecklisten auf aktuellem Pi aufnehmen | Hardwaremessung | P1 | ERLEDIGT | — |
| [M04](M04.md) | Laufmanifest und gemessenes Kernelabbild eindeutig verbinden | Messverfahren | P2 | ERLEDIGT | — |
| [A01](A01.md) | wind_get(0, WF_WXYWH): geräteabhängige Geometrie absichern | Geräteunterschied / Test | P2 | ERLEDIGT (2026-09-19) | — |
| [A02](A02.md) | wind_get(0, WF_CXYWH): geräteabhängige Geometrie absichern | Geräteunterschied / Test | P2 | ERLEDIGT (2026-09-19) | — |
| [A03](A03.md) | wind_get(0, WF_PXYWH): geräteabhängige Geometrie absichern | Geräteunterschied / Test | P2 | ERLEDIGT (2026-09-19) | — |
| [A04](A04.md) | wind_get(0, WF_FXYWH): geräteabhängige Geometrie absichern | Geräteunterschied / Test | P2 | ERLEDIGT (2026-09-19) | — |
| [A05](A05.md) | wind_get WF_OWNER: Versionsabweichung belegen | Versionsprüfung | P2 | ERLEDIGT (2026-09-19) — Versionsunterschied | — |
| [A06](A06.md) | appl_init: ap_id | Global-Feldprüfung | P2 | PARKED | — |
| [A07](A07.md) | appl_init: ap_private | Global-Feldprüfung | P2 | PARKED | — |
| [A08](A08.md) | appl_init: ap_planes | Global-Feldprüfung | P2 | PARKED | — |
| [A09](A09.md) | appl_init: ap_3resv | Global-Feldprüfung | P2 | PARKED | — |
| [A10](A10.md) | wind_create: Kapazitätsgrenze | Nachmessung auswerten | P1 | ERLEDIGT | M01 |
| [A11](A11.md) | wind_get WF_CXYWH: erzeugtes ungeöffnetes Fenster | Nachmessung auswerten | P1 | ERLEDIGT | M01 |
| [A12](A12.md) | wind_get WF_PXYWH: Zustand nach wind_open | Nachmessung auswerten | P1 | ERLEDIGT | M01 |
| [A13](A13.md) | wind_get WF_FIRSTXYWH: erstes sichtbares Rechteck | Messlücke / bedingte Reparatur | P1 | ERLEDIGT | M02, M03 |
| [A14](A14.md) | wind_get WF_NEXTXYWH: Iteratorfortschritt | Messlücke / bedingte Reparatur | P1 | ERLEDIGT | M02, M03 |
| [A15](A15.md) | wind_get: geschriebene Felder am Listenende | Messlücke / bedingte Reparatur | P1 | ERLEDIGT | M02, M03 |
| [A16](A16.md) | wind_close: WS_CURR/WS_PREV/WS_WORK bleiben unaktualisiert | Belegter Defekt / bedingte Reparatur | P1 | PARKED | — |
| [A17](A17.md) | wind_open: PREV-Rechteck im Soll nachtragen, Zeilenbelege richtigstellen | Spec-/Belegkorrektur | P2 | ERLEDIGT (2026-09-19) | — |
| [A18](A18.md) | Rechteckliste: Soll in der Spec nachtragen statt lizenzieren | Spec-Korrektur | P1 | ERLEDIGT (2026-09-19) | A13 |
| [A19](A19.md) | AES-Dispatcher: int_in ohne IN_LEN-Klammer gelesen | Belegter Defekt / ABI | P1 | IMPLEMENTED | — |
| [A20](A20.md) | Menüleistenhöhe konstant 19 statt abgeleitet | Belegter Defekt / Geräteableitung | P2 | ERLEDIGT (2026-09-19) | — |
| [A21](A21.md) | Rechteckliste verwirft Stücke jenseits von 16 | Belegter Defekt / Robustheit | P2 | ERLEDIGT (2026-09-19) | A13, A14 |
| [A22](A22.md) | Messlücke: drei und mehr überlappende Fenster | Messlücke / Messdesign | P1 | PARKED | A13, A14 |
| [A23](A23.md) | Kompensationsanalyse vor dem Fix-Durchgang | Gate / Analyse vor Reparatur | P1 | ERLEDIGT (2026-09-10, siehe A23-STATUS-DONE.MD) | A13, A14, A15 |
| [A24](A24.md) | Messlücke: verschachtelte FIRST/NEXT über zwei Fenster | Messlücke / Messdesign | P1 | ERLEDIGT (2026-09-12) | A14 |
| [A26](A26.md) | Erratum: Kopfkommentar der Rechtecklisten-Tabelle | Erratum / Dokumentation | P2 | PARKED | A15 |
| [A27](A27.md) | Unterschreibender Handler: bewusste Abweichung belegen, Spec richtigstellen | Bewertung / Spec | P2 | PARKED | — |
| [A28](A28.md) | Fehlende Zustandsrücksetzung Desktop → Kindprogramm | Belegter Defekt / Startpfad | P1 | PARKED | A25, A29 |
| [A29](A29.md) | Programmende löscht den Deskriptor des Desktops | Belegter Defekt / Startpfad | P1 | PARKED | — |
| [A30](A30.md) | Messdesign: appl_init-Wiederaufruf ohne Hängerrisiko | Messlücke / Messdesign | P1 | ERLEDIGT (2026-09-19) — Entwurf; Ausfuehrung als Folgeauftr… | A25, A28 |
| [A31](A31.md) | Testlücke: Rechteckliste über save/restore_windows | Testlücke | P2 | OBSOLETE | A14 |
| [A32](A32.md) | Abnahme der Rechtecklisten-Reparatur auf echter Hardware | Hardwareabnahme | P1 | ERLEDIGT (2026-09-19) | A13, A14, A15 |
| [A33](A33.md) | wind_delete setzt die Fensterrechtecke nicht zurück | Belegter Defekt | P2 | PARKED | A16 |
| [V01](V01.md) | vq_extnd(0): Bildschirmgrenzen | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V02](V02.md) | vq_extnd(0): Farbanzahl | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V03](V03.md) | vq_extnd(0): Palettenpräzision | Geräteunterschied / Quellenprüfung | P2 | PARKED | — |
| [V04](V04.md) | vq_extnd(1): Planes | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V05](V05.md) | vq_extnd(1): Video-Lookup-Tabelle | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V06](V06.md) | vq_extnd(1): Clip-Rechteck nach Ein/Aus | Geräteunterschied / Test | P1 | OBSOLETE | — |
| [V07](V07.md) | vsl_color: Bereichsgrenze aus numcolors | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V08](V08.md) | vsm_color: Bereichsgrenze aus numcolors | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V09](V09.md) | vq_color: Bereichsgrenze aus numcolors | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V10](V10.md) | v_get_pixel: Rückgabeformat Palette gegen Truecolor | Geräteunterschied / Test | P1 | OBSOLETE | — |
| [V11](V11.md) | v_get_pixel: Weiß und OVERLAY_BIT isolieren | Diagnose eines bekannten Defekts | P1 | IMPLEMENTED | — |
| [V12](V12.md) | Weiß-/Overlayfehler lokal korrigieren | Bedingte Implementierung | P1 | PARKED | V11 |
| [V13](V13.md) | vq_chcells: Textzellen aus Bildschirm und Font | Geräteunterschied / Test | P2 | OBSOLETE | — |
| [V14](V14.md) | v_opnvwk: ausgegebene Gerätemetriken | Geräteunterschied / Test | P2 | OBSOLETE | V01, V02, V03 |
| [V15](V15.md) | v_opnvwk: Handle-Identität statt Zahlenvergleich | Messauswertung | P2 | OBSOLETE | — |
| [R01](R01.md) | vro_cpyfm: oversized destination, D-VDI-25 | Zurückgestellte Vertragsabweichung | P3 | PARKED | — |
| [R02](R02.md) | vro_cpyfm: Screen → Memory messen | Messlücke / Messdesign | P1 | PARKED | — |
| [R03](R03.md) | vro_cpyfm: Memory → Screen messen | Messlücke / Messdesign | P1 | PARKED | — |
| [R04](R04.md) | vro_cpyfm: überlappende Screen-Kopie messen | Messlücke / Messdesign | P1 | PARKED | — |
| [R05](R05.md) | vrt_cpyfm: monochrome Expansion auf Screen messen | Messlücke / Messdesign | P1 | IMPLEMENTED | — |
| [Q01](Q01.md) | v_gtext: sichtbares einzelnes a messen | Pixel-Messlücke | P1 | OBSOLETE | — |
| [Q02](Q02.md) | vs_clip: Text am Clip-Rand messen | Pixel-Messlücke | P1 | OBSOLETE | Q01 |
| [Q03](Q03.md) | vr_recfl: XOR-Cursor zweimal zeichnen | Pixel-Messlücke | P1 | OBSOLETE | Q01 |
| [Q04](Q04.md) | evnt_multi: ein Tastaturereignis vermessen | Interaktive Messlücke | P1 | OBSOLETE | — |
| [Q05](Q05.md) | graf_mouse: Hide/Show um Textausgabe | Interaktive Pixel-Messlücke | P1 | OBSOLETE | Q01 |
| [Q06](Q06.md) | QED restore_edit: abgebrochene Verzweigung bestimmen | Anwendungsdiagnose → belegter Defekt | P1 | ERLEDIGT 2026-10-17 (Quittung) | — |
| [A25](A25.md) | appl_init: Nachrichten/Flags bei Wiederaufruf | Bestätigter Zustandsfehler | P1 | DONE | — |

## Späterer Backlog: genau eine Funktion pro Messdesign-Task

Diese 80 Aufgaben schließen fehlende Records im aktuellen Paket. Sie sind keine Aussage, dass die Funktion defekt oder überhaupt noch nie anderweitig geprüft wurde. Implizite Initialisierung und frühere Tests sind ausdrücklich nicht verloren, aber hier kein aktueller Einzelmesswert.

| ID | Funktion / Auftrag |
|---|---|
| [G-AES-011](G-AES-011.md) | appl_read (AES 11): kleinsten Messfall entwerfen |
| [G-AES-012](G-AES-012.md) | appl_write (AES 12): kleinsten Messfall entwerfen |
| [G-AES-013](G-AES-013.md) | appl_find (AES 13): kleinsten Messfall entwerfen |
| [G-AES-014](G-AES-014.md) | appl_tplay (AES 14): kleinsten Messfall entwerfen |
| [G-AES-015](G-AES-015.md) | appl_trecord (AES 15): kleinsten Messfall entwerfen |
| [G-AES-017](G-AES-017.md) | appl_yield (AES 17): kleinsten Messfall entwerfen |
| [G-AES-019](G-AES-019.md) | appl_exit (AES 19): kleinsten Messfall entwerfen |
| [G-AES-020](G-AES-020.md) | evnt_keybd (AES 20): kleinsten Messfall entwerfen |
| [G-AES-021](G-AES-021.md) | evnt_button (AES 21): kleinsten Messfall entwerfen |
| [G-AES-022](G-AES-022.md) | evnt_mouse (AES 22): kleinsten Messfall entwerfen |
| [G-AES-023](G-AES-023.md) | evnt_mesag (AES 23): kleinsten Messfall entwerfen |
| [G-AES-026](G-AES-026.md) | evnt_dclick (AES 26): kleinsten Messfall entwerfen |
| [G-AES-030](G-AES-030.md) | menu_bar (AES 30): kleinsten Messfall entwerfen |
| [G-AES-031](G-AES-031.md) | menu_icheck (AES 31): kleinsten Messfall entwerfen |
| [G-AES-032](G-AES-032.md) | menu_ienable (AES 32): kleinsten Messfall entwerfen |
| [G-AES-033](G-AES-033.md) | menu_tnormal (AES 33): kleinsten Messfall entwerfen |
| [G-AES-034](G-AES-034.md) | menu_text (AES 34): kleinsten Messfall entwerfen |
| [G-AES-035](G-AES-035.md) | menu_register (AES 35): kleinsten Messfall entwerfen |
| [G-AES-036](G-AES-036.md) | menu_popup (AES 36): kleinsten Messfall entwerfen |
| [G-AES-037](G-AES-037.md) | menu_attach (AES 37): kleinsten Messfall entwerfen |
| [G-AES-038](G-AES-038.md) | menu_istart (AES 38): kleinsten Messfall entwerfen |
| [G-AES-039](G-AES-039.md) | menu_settings (AES 39): kleinsten Messfall entwerfen |
| [G-AES-040](G-AES-040.md) | objc_add (AES 40): kleinsten Messfall entwerfen |
| [G-AES-041](G-AES-041.md) | objc_delete (AES 41): kleinsten Messfall entwerfen |
| [G-AES-042](G-AES-042.md) | objc_draw (AES 42): kleinsten Messfall entwerfen |
| [G-AES-043](G-AES-043.md) | objc_find (AES 43): kleinsten Messfall entwerfen |
| [G-AES-044](G-AES-044.md) | objc_offset (AES 44): kleinsten Messfall entwerfen |
| [G-AES-045](G-AES-045.md) | objc_order (AES 45): kleinsten Messfall entwerfen |
| [G-AES-046](G-AES-046.md) | objc_edit (AES 46): kleinsten Messfall entwerfen |
| [G-AES-047](G-AES-047.md) | objc_change (AES 47): kleinsten Messfall entwerfen |
| [G-AES-048](G-AES-048.md) | objc_sysvar (AES 48): kleinsten Messfall entwerfen |
| [G-AES-050](G-AES-050.md) | form_do (AES 50): kleinsten Messfall entwerfen |
| [G-AES-051](G-AES-051.md) | form_dial (AES 51): kleinsten Messfall entwerfen |
| [G-AES-052](G-AES-052.md) | form_alert (AES 52): kleinsten Messfall entwerfen |
| [G-AES-053](G-AES-053.md) | form_error (AES 53): kleinsten Messfall entwerfen |
| [G-AES-054](G-AES-054.md) | form_center (AES 54): kleinsten Messfall entwerfen |
| [G-AES-055](G-AES-055.md) | form_keybd (AES 55): kleinsten Messfall entwerfen |
| [G-AES-056](G-AES-056.md) | form_button (AES 56): kleinsten Messfall entwerfen |
| [G-AES-070](G-AES-070.md) | graf_rubbox (AES 70): kleinsten Messfall entwerfen |
| [G-AES-071](G-AES-071.md) | graf_dragbox (AES 71): kleinsten Messfall entwerfen |
| [G-AES-072](G-AES-072.md) | graf_mbox (AES 72): kleinsten Messfall entwerfen |
| [G-AES-073](G-AES-073.md) | graf_growbox (AES 73): kleinsten Messfall entwerfen |
| [G-AES-074](G-AES-074.md) | graf_shrinkbox (AES 74): kleinsten Messfall entwerfen |
| [G-AES-075](G-AES-075.md) | graf_watchbox (AES 75): kleinsten Messfall entwerfen |
| [G-AES-076](G-AES-076.md) | graf_slidebox (AES 76): kleinsten Messfall entwerfen |
| [G-AES-080](G-AES-080.md) | scrp_read (AES 80): kleinsten Messfall entwerfen |
| [G-AES-081](G-AES-081.md) | scrp_write (AES 81): kleinsten Messfall entwerfen |
| [G-AES-082](G-AES-082.md) | scrp_clear (AES 82): kleinsten Messfall entwerfen |
| [G-AES-090](G-AES-090.md) | fsel_input (AES 90): kleinsten Messfall entwerfen |
| [G-AES-091](G-AES-091.md) | fsel_exinput (AES 91): kleinsten Messfall entwerfen |
| [G-AES-107](G-AES-107.md) | wind_update (AES 107): kleinsten Messfall entwerfen |
| [G-AES-110](G-AES-110.md) | rsrc_load (AES 110): kleinsten Messfall entwerfen |
| [G-AES-111](G-AES-111.md) | rsrc_free (AES 111): kleinsten Messfall entwerfen |
| [G-AES-112](G-AES-112.md) | rsrc_gaddr (AES 112): kleinsten Messfall entwerfen |
| [G-AES-113](G-AES-113.md) | rsrc_saddr (AES 113): kleinsten Messfall entwerfen |
| [G-AES-114](G-AES-114.md) | rsrc_obfix (AES 114): kleinsten Messfall entwerfen |
| [G-AES-120](G-AES-120.md) | shel_read (AES 120): kleinsten Messfall entwerfen |
| [G-AES-121](G-AES-121.md) | shel_write (AES 121): kleinsten Messfall entwerfen |
| [G-AES-122](G-AES-122.md) | shel_get (AES 122): kleinsten Messfall entwerfen |
| [G-AES-123](G-AES-123.md) | shel_put (AES 123): kleinsten Messfall entwerfen |
| [G-AES-124](G-AES-124.md) | shel_find (AES 124): kleinsten Messfall entwerfen |
| [G-AES-125](G-AES-125.md) | shel_envrn (AES 125): kleinsten Messfall entwerfen |
| [G-AES-126](G-AES-126.md) | shel_rdef (AES 126): kleinsten Messfall entwerfen |
| [G-AES-127](G-AES-127.md) | shel_wdef (AES 127): kleinsten Messfall entwerfen |
| [G-VDI-001](G-VDI-001.md) | v_opnwk (VDI 1): kleinsten Messfall entwerfen |
| [G-VDI-002](G-VDI-002.md) | v_clswk (VDI 2): kleinsten Messfall entwerfen |
| [G-VDI-014](G-VDI-014.md) | vs_color (VDI 14): kleinsten Messfall entwerfen |
| [G-VDI-028](G-VDI-028.md) | v_locator (VDI 28): kleinsten Messfall entwerfen |
| [G-VDI-031](G-VDI-031.md) | v_string (VDI 31): kleinsten Messfall entwerfen |
| [G-VDI-101](G-VDI-101.md) | v_clsvwk (VDI 101): kleinsten Messfall entwerfen |
| [G-VDI-112](G-VDI-112.md) | vsf_udpat (VDI 112): kleinsten Messfall entwerfen |
| [G-VDI-118](G-VDI-118.md) | vex_timv (VDI 118): kleinsten Messfall entwerfen |
| [G-VDI-119](G-VDI-119.md) | vst_load_fonts (VDI 119): kleinsten Messfall entwerfen |
| [G-VDI-120](G-VDI-120.md) | vst_unload_fonts (VDI 120): kleinsten Messfall entwerfen |
| [G-VDI-124](G-VDI-124.md) | vq_mouse (VDI 124): kleinsten Messfall entwerfen |
| [G-VDI-125](G-VDI-125.md) | vex_butv (VDI 125): kleinsten Messfall entwerfen |
| [G-VDI-126](G-VDI-126.md) | vex_motv (VDI 126): kleinsten Messfall entwerfen |
| [G-VDI-127](G-VDI-127.md) | vex_curv (VDI 127): kleinsten Messfall entwerfen |
| [G-VDI-128](G-VDI-128.md) | vq_key_s (VDI 128): kleinsten Messfall entwerfen |
| [G-VDI-134](G-VDI-134.md) | vex_wheelv (VDI 134): kleinsten Messfall entwerfen |

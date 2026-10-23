# CON04 — `Bconout(2)` wertet keine VT52-Folgen aus

Status: **ERLEDIGT** 2026-10-23 (implementiert 2026-10-22, QEMU raspi4b + Hatari-Falcon-Gegenprobe; Tester-Abnahme PASS `5ca5fa30`; echte Pi abgenommen 2026-10-23, `measurements/measurement-2026-10-23-usb01-con04-pi/RUNINFO.TXT`, [Quittung](CON04-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Abweichung von der Spec / TOS-API-Luecke — Befund CON03-Spec (2026-10-22)

## Befund

`docs/api/bios/03-bconout.md` (Handle 2 = CON mit VT52, Handle 5 = roh ohne
VT52, EmuTOS `bios/chardev.c:104`) verlangt fuer Handle 2 das Auswerten der
VT52-Escape-Folgen. Die Umsetzung `tos-api/src/vdi/escape.rs` `conout`
(`:122-166`) ignoriert `ESC` und gibt den Folgebuchstaben aus („NewTOS' VT-52
console does not parse ESC sequences from the character stream“). Jedes
Textprogramm mit `ESC E` (Bildschirm loeschen), Cursorsetzen (`ESC Y`),
Inversschrift usw. zeigt Zeichensalat. EmuCON braucht `ESC E e D C K l v w b c`
(`docs/api/emucon/`).

## Auftrag

1. Spec der VT52-Konsole aus `references/emutos/bios/vt52.c` (und
   `conout.c`, Line-A-Zustand `docs/api/_line-a.md`) nach
   `SOURCE-ANALYSIS-PROTOCOL.md`: alle Folgen, Zustandsautomat, Farben
   (`ESC b/c`), Wrap (`ESC v/w`), Cursor ein/aus, Speichern/Wiederherstellen,
   Verhalten von Handle 5. Freigabe durch den Anwender.
2. Vollstaendiger Parser nach `vt52.c`, auf dem bestehenden `escape.rs`-Zustand
   (derselbe Cursor wie `v_escape`).
3. Tests: Hosttest je Folge; QEMU-Sonde mit Screendump; Vergleich gegen
   Hatari-Falcon (EmuTOS) mit derselben Ausgabe.

## Entscheidungen (Anwender, 2026-10-22)

Spec: `docs/api/bios/_vt52.md` (`1d48ce72`), §13.

Grundsatz: programm-sichtbares Verhalten 1:1 EmuTOS. Abweichung nur, wo belegt
ist, dass die EmuTOS-Eigenheit ein Artefakt der alten Plattform ist (Quelle
Datei:Zeile + Gegenprobe Hatari) und die moderne Loesung nachweislich besser
ist; jede solche Abweichung in `docs/COMPATIBILITY.md`.

* **D1** `ESC b/c`: Register → Stift → Workstation-Palette; ohne Workstation
  Falcon-Standardpalette. Wie EmuTOS.
* **D2** sichtbarer Cursor als Zellinversion, `ESC e/f`, `disab_cnt`. Wie EmuTOS.
* **D3** Wrap Anfangswert aus; BS nur links, HT klemmt, LF behaelt Spalte,
  VT/FF wie LF. Wie EmuTOS. Kernel-eigene Ausgaben mit nacktem `\n` auf
  `\r\n` umstellen.
* **D4** BEL wertet `conterm&4` aus, Ton entfaellt (kein Audiogeraet).
* **D5** Handle 5 eigener roher Pfad ohne Automat.
* **Q5** Blinkperiode feste 0,5 s ueber den 200-Hz-Zaehler (EmuTOS: 30 VBL =
  0,43-0,6 s je Maschine, also Geraeteparameter). Abweichung dokumentieren.
* **Q3** Randzelle bei Revers: erst Hatari-Falcon-Gegenprobe (`ESC p` +
  `ESC K`/`ESC o` an gerader/ungerader Spalte). Bestaetigt sich die Eigenheit:
  einheitlich mit Hintergrund loeschen (Ursache Bitplane-Paarung
  `conout.c:586-596`), in COMPATIBILITY mit Beleg.
* **Q6** `vs_curaddress` setzt die Position direkt mit Klemmen (EmuTOS-Umweg
  ueber Byte-`ESC Y` erreicht keine Spalte > 223; NewTOS hat bis 240). Die
  Stromfolge `ESC Y` bleibt Byte-kodiert. Abweichung dokumentieren.

Voraussetzung fuer [CON03](CON03.md).

[Gemeinsame Regeln](README.md) gelten.

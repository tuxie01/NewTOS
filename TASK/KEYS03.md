# KEYS03 — `#K` in NEWDESK.INF gehoert EmuDESK: die Tastaturbelegung braucht eine eigene Zeile

Status: **OFFEN** · Prioritaet: **P3** · Typ: Kernel / Desktop-INF — offener Konflikt aus `docs/COMPATIBILITY.md` (Eintrag „The boot keyboard layout comes from …“, gemeldet 2026-10-26)

Abhaengigkeiten: — (Spec-Entscheidung zum neuen Schluessel, siehe *Offene Frage*)

## Sachverhalt

`#K` ist eine Zeile des EmuDESK-INF-Formats: die 33 Menue-Kurztasten, je ein
zweistelliges Hexbyte. EmuTOS liest sie in `app_start`
(`references/emutos/desk/deskapp.c:967-974`, `NUM_SHORTCUTS` = 33,
`references/emutos/desk/deskmain.h:18`) und schreibt sie in `app_save`
zwischen `#Q` und den `#W`-Zeilen als `#K` + 33 × ` %02X` + CRLF
(`references/emutos/desk/deskapp.c:1184-1192`, unter
`CONF_WITH_DESKTOP_CONFIG`, im Referenzbau an: `docs/api/_emutos-config.md:178`).
Unser Desktop tut dasselbe seit DESK05: Lesen
`programs/desktop/src/inf.rs:472-476`, Schreiben `:625-633`.

Der Kernel benutzt **dieselbe** Zeile fuer die Tastaturbelegung beim Booten:
`keymap::init_from_disk` (`kernel/src/keymap.rs:329-349`, gerufen aus
`kernel/src/lib.rs:400`) liest `C:\NEWDESK.INF`, `find_k_line`
(`kernel/src/keymap.rs:351-377`) nimmt das **erste** Wort der **ersten**
`#K`-Zeile und gibt es an `select_builtin` (`kernel/src/keymap.rs:223-232`),
das nur `us`/`US`/`de`/`DE` annimmt. Dokumentiert als NewTOS-Erweiterung in
`docs/COMPATIBILITY.md` (Eintrag „The boot keyboard layout comes from
`C:\KEYBOARD.KBD` or a `#K` line …“), dort auch der Konflikt als offen
gemeldet.

Folgen, aus dem Quelltext gelesen (noch nicht gemessen):

1. **Desktop sichern loescht die Belegungswahl.** *Save Desktop* schreibt die
   Datei neu und nur seine eigene `#K`-Zeile. Ihr erstes Wort ist ein Hexbyte
   (Kurztaste von Platz 0); `select_builtin` lehnt es ab, der naechste Boot
   faellt auf US zurueck.
2. **Eine handgeschriebene `#K de`-Zeile verdirbt die Kurztasten.** Der
   Desktop liest sie als Kurztastenzeile: `scan_2`
   (`programs/desktop/src/inf.rs:366-374`) macht aus `de` das Byte `0xDE` fuer
   Platz 0 und setzt die uebrigen 32 Plaetze auf 0 (Zeilenende), die
   Vorgaben sind damit ueberschrieben.
3. Stehen beide Zeilen in der Datei, nimmt der Kernel die erste, der Desktop
   die letzte (`case 'K'` ueberschreibt bei jeder Zeile).

`C:\KEYBOARD.KBD` ist nicht betroffen (hat Vorrang, `kernel/src/keymap.rs:336-341`).

## Ziel

`#K` bleibt **genau** wie in EmuTOS (Lesen und Schreiben unveraendert, 33
Bytes). Die Belegungswahl zieht auf eine **eigene** Zeile bzw. einen eigenen
Schluessel um, den *Save Desktop* erhaelt (oder der ausserhalb von
NEWDESK.INF liegt — Teil der offenen Frage). `find_k_line` und der
COMPATIBILITY-Eintrag werden entsprechend umgestellt; der Eintrag verliert
den Absatz „Open conflict“.

## Offene Frage (Anwender)

- **Name und Ort des neuen Schluessels.** Eine neue `#`-Zeile in
  NEWDESK.INF (welcher Buchstabe), eine Zeile in einer anderen Datei, oder
  nur noch `C:\KEYBOARD.KBD`? Nicht vom Developer zu entscheiden. Zu
  beachten: EmuDESKs Leser uebergeht einen unbekannten Buchstaben (der
  `switch` in `app_start` hat keinen `default`,
  `references/emutos/desk/deskapp.c:851-976`), aber `app_save` baut die Datei
  nur aus seinen eigenen Zeilen neu (`references/emutos/desk/deskapp.c:1106`
  ff.) — eine neue Zeile in NEWDESK.INF ginge beim Sichern also ebenso
  verloren, es sei denn, NEWDESK schreibt sie mit (eine Abweichung von
  `app_save`, die dann in `docs/COMPATIBILITY.md` gehoert). Eine eigene Datei
  vermeidet beides.
- Uebergang: soll eine vorhandene `#K us`/`#K de`-Zeile einmalig noch als
  Belegung gelesen werden?

## Reproduktion

1. **NewTOS, Vorher:** QEMU-`raspi4b`-Abbild (Klon, nicht Kopie; `serial=on`)
   mit `C:\NEWDESK.INF`, das eine Zeile `#K de` enthaelt. Booten; die aktive
   Belegung lesen: `Ssystem(S_NEWTOS_KEYBOARD)` = 2017
   (`tos-api/src/gemdos/ssystem.rs:89`, `:174`), z. B. ueber SYSINFO
   (SYSINFO01); erwartet `1` = BuiltinGerman (`kernel/src/keymap.rs:207-208`).
2. Im Desktop *Options → Save Desktop* ausloesen (Injektor nach
   `measurements/measurement-2026-09-17-h03/method/`, oder auf der Pi). Die
   geschriebene `NEWDESK.INF` vom Abbild lesen und die `#K`-Zeile festhalten.
3. Neu booten, Schritt 1 wiederholen: erwartet heute `0` = BuiltinUs — die
   Wahl ist verloren.
4. **EmuTOS-Gegenstueck:** gepinntes Orakel (`docs/analysis/ORACLE.md` §1–§2,
   `tools/hatari-oracle.sh`), dieselben Kurztasten (Vorgabe), *Save Desktop*,
   `EMUDESK.INF` vom GEMDOS-Laufwerk C: lesen und die `#K`-Zeile byteweise mit
   Schritt 2 vergleichen. ORACLE.md beschreibt nur Dumperlaeufe ohne Klicks;
   wie der Speichervorgang im Orakel ausgeloest wird (Injektor, Hatari-
   Debugger, Laufwerksvorlage), legt der Tester fest und haelt es im Lauf
   fest.

Lauf nach `measurements/measurement-<datum>-keys03/` (RUNINFO.TXT, Abbild
und Kernel per sha256).

## Abnahme

- Die von *Save Desktop* geschriebene `#K`-Zeile ist **bytegleich** mit der
  des EmuTOS-Orakels bei denselben Kurztasten.
- Die Belegungswahl ueberlebt *Save Desktop*: nach Sichern und Neustart
  liefert `Ssystem(2017)` dieselbe Belegung wie vorher.
- Eine Datei mit beiden Angaben ergibt sowohl die richtige Belegung als auch
  die gespeicherten Kurztasten.
- `docs/COMPATIBILITY.md` beschreibt den neuen Schluessel; Hosttest fuer die
  Zeilensuche des Kernels (heute `find_k_line`).

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Nichts an `#K` selbst aendern; keine
Karte beschreiben (QEMU-Abbild genuegt fuer die Reproduktion).

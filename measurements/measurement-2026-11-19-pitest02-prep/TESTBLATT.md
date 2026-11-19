# Testblatt PITEST02 - die gesammelten Real-Pi-Pruefungen (Stand main afc2bb1c0, 2026-11-19)

Ein Lauf, ein Kernel (`kern1119.img` = `kernel8.img`, Release-Build von `afc2bb1c0`), zwei Startarten:

* **Teil A: Standardstart** (`CMDLINE.TXT` leer, so ist die Karte heute).
* **Teil B: `vdiplanes=32`** (`CMDLINE.TXT` enthaelt `vdiplanes=32`).
* **Teil C: SND03-Hoertest** (anderer Kernel, `kerns03.img`, nur am Mac umschaltbar).

Jeder Schritt nennt: was du tust, was du sehen musst, was du zurueckmeldest, woher der Schritt kommt (Ticket oder Lauf).
Wo eine Quelle den erwarteten Wert nicht nennt, steht dort **nicht festgelegt**: dann bitte nur melden, was du siehst,
das Blatt entscheidet nichts.

Du brauchst: Pi 4 mit HDMI-Monitor (1920x1080), USB-Tastatur und Maus, Kopfhoerer oder Lautsprecher an der 3,5-mm-Klinke
(A3, Teil C), Handy zum Fotografieren; optional eine Tuner-App (A3) und einen USB-Seriell-Adapter an GPIO 14/15,
115200 8N1 (A11, B1b). Fotos: Handy-Foto des Bildschirms genuegt.

**Lautstaerke:** SNDTONE ist ein Sinus bei halber Aussteuerung. Lautsprecher erst leise, Kopfhoerer zuerst in der Hand.

Programme werden per Doppelklick aus dem Dateifenster gestartet. Ein Programm schreibt seine Datei in **seinen eigenen
Ordner** (am 2026-11-12 landete `KPROBE.LOG` so in `C:\PITEST`). Dateien oeffnest du am Pi mit QED (Doppelklick auf die
Datei oder Datei-Menue von QED), oder du holst sie am Ende vom Karten-Mac. Haengt der Pi so, dass keine Taste hilft: Foto,
Strom aus, aufschreiben, was zuletzt zu sehen war, und mit dem naechsten Schritt weitermachen.

## Teil 0: Umschalten zwischen Standardstart und `vdiplanes=32`

Die Karte enthaelt neben `CMDLINE.TXT` die Dateien `CMD16.TXT` (`vdiplanes=16`), `CMD32.TXT` (`vdiplanes=32`),
`CMDSER.TXT` (`serial=on`) und `CMD32S.TXT` (`serial=on vdiplanes=32`). Der Kernel liest `CMDLINE.TXT` ueber den
Mailbox-Kanal der Firmware, also bei jedem Kaltstart neu.

* **Mit dem Mac (sicherer Weg):** Karte in den Leser, dann am Terminal
  `cp -X "/Volumes/BOOT 1/CMD32.TXT" "/Volumes/BOOT 1/CMDLINE.TXT"` (auf 32) und
  `cp -X "/Volumes/BOOT 1/CMD16.TXT" "/Volumes/BOOT 1/CMDLINE.TXT"` (zurueck; `vdiplanes=16` ist gleich dem Standardstart,
  es erscheint nur eine zusaetzliche Bootzeile). Danach auswerfen.
* **Ohne Mac, am Pi (nicht von mir gefahren, kein Real-Pi hier):** Im Desktop QED starten, `C:\CMDLINE.TXT` oeffnen
  (Wurzel von C:), den Text durch `vdiplanes=32` ersetzen (zurueck: durch `vdiplanes=16`), speichern, QED beenden, dann
  **Strom aus und wieder ein** (Kaltstart; ob Strg+Alt+Entf die Firmware neu laufen laesst, habe ich nicht geprueft).
  Warte nach dem Speichern einige Sekunden, bevor du den Strom trennst.

Pruefung, welche Stellung aktiv ist, ohne Seriell: SCRNINFO (A2 / B2) zeigt `16 planes` oder `32 planes`.

**Reihenfolge:** erst Teil A komplett (ohne Umschalten), dann Umschalten auf 32, Teil B, am Ende B12 (zurueck auf
Standard). Teil C nur nach Absprache (Kernelwechsel).

---

## Teil A - Standardstart (`CMDLINE.TXT` leer)

### A1 Start und Desktop (Basis fuer alles Folgende)
Pi einschalten. Erwartet: Begruessungsbild, dann der Desktop (EmuDESK), Maus bewegt sich, Tastatur reagiert.
Foto vom Desktop. Meldung: ob alles wie bisher aussieht.
Quelle: V62 (Standardstart unveraendert, `measurements/measurement-2026-11-18-v62-test/RUNINFO.TXT`), STATUS 2026-11-18
"collected for the larger real-Pi run".

### A2 SCRNINFO: `_SND` im Cookie-Jar auf Silizium, 16 Planes
`C:\PITEST\SCRNINFO.NAP` doppelklicken (laeuft kurz), danach `C:\PITEST\SCRNINFO.TXT` in QED oeffnen.
Erwartet (Referenz aus QEMU mit demselben Kernel: `smoke-ref/SCRNINFO-qemu-default.TXT`, dort 640x480):
* Abschnitt 1, Cookie-Jar: **ein Eintrag mehr als in QEMU** und darin `5F534E44 '_SND'  value 00000014 (20)`
  (QEMU hat 6 Eintraege ohne `_SND`: NTOS, _MCH, _IDT, _AKP, EdDI, _VDO). Reihenfolge der Eintraege nicht festgelegt.
* Abschnitt 3: `screen 1920x1080, 16 planes (vq_extnd(1)[4])`; bei `vq_scrninfo` `work_out[2]` = `16/0010`,
  `work_out[4]` = `-32768/8000`.
Meldung: Foto oder Datei; ob `_SND` im Jar steht; Bildschirmgroesse; Planes.
Quelle: SND05 "Real-Pi run" (Satz "Not seen in this run: ... `_SND` in the jar"), SND05-Testlauf Real-Pi-Punkt 4
(`measurements/measurement-2026-11-17-snd05-test/RUNINFO.TXT`).

### A3 SNDTONE: Ton, Kanaele, Tonhoehe (SND05-Rest)
Wie am 2026-11-18: Konsole, `run sndtone.ncp` im Ordner `C:\PITEST` (TASK/SND05 "Real-Pi run (owner, 2026-11-18)").
Erwartet auf dem Bildschirm (dieselben Werte wie am 2026-11-18): `Locksnd() = 1`, `Soundcmd(9, -1) = 5`,
`Setmode(STEREO16) = 0`, `Devconnect(0, 8, 0, 1, 1) = 1`; in jedem der drei Durchgaenge (beide / LEFT / RIGHT)
`Setbuffer = 0`, `Buffoper(1) = 0`, `Buffoper(-1)` nach dem Durchgang `= 0`, `Buffptr` play offset `= 393360`, record `= 0`;
`Unlocksnd() = 0`; letzte Zeile `SNDTONE: done`. Kein `MISMATCH`, kein `STALLED`.
Ton: sauber, 440 Hz, beide Ohren / nur links / nur rechts, kein Knacken, kein Aussetzen.
**Tonhoehe (neu):** Tuner-App (Handy) waehrend des ersten Durchgangs an den Lautsprecher/Kopfhoerer halten. Soll: 440 Hz
(Kammerton a), Toleranz **nicht festgelegt**; melde die angezeigte Frequenz oder "nicht messbar".
Meldung: Foto der Textseite, Tuner-Wert, ob etwas anders klang als am 2026-11-18.
Quelle: SND05 (Pitch "not measured", "by feeling it fits"), STATUS 2026-11-18 ("pitch" in der Sammelliste).

### A4 `bios-r1` auf dem Pi: DMA-Sound-Faelle 230-235, `VgetSize`, `Getrez`
`C:\PITEST\BIOS\DUMPBIOS.NAP` doppelklicken (in QEMU etwa 19 s, schreibt `BIOS.BIN` und `BIOS.LOG` in
`C:\PITEST\BIOS\`). Erste Silizium-Laeufe dieser Tabelle: **nicht festgelegt**, ob die Tabelle schon einmal auf dem Pi lief;
haengt sie, Foto und Strom aus.
Erwartet in `BIOS.LOG` (QEMU-Referenz `smoke-ref/bios-r1/BIOS.LOG`): `BIOS.TSV: 147 cases, CRC 583C11E0`,
`all 147 cases ran`, `BIOS.BIN: 147 records, 47072 bytes`.
Am Mac auswerten (Lead, nach dem Zurueckkopieren; Referenz zuerst):
`tosdump/host/build/tosdiff tosdump/cases/bios-r1.tsv tosdump/reference/EMUTOS-1a7484ab-BIOS-falcontc.BIN BIOS.BIN`
(und dieselbe Zeile mit `EMUTOS-1a7484ab-BIOS.BIN` fuer die tt-Referenz). Erwartung nach den Quellen:
* Faelle **230-234** (`Soundcmd 0/1/6`, `Buffoper(-1)`, `Sndstatus(0)`) sind **gleich** der falcontc-Referenz, weil jetzt
  ein Backend da ist; in QEMU (ohne Backend) stehen sie als Unterschied, Antwort = Funktionsnummer
  (COMPATIBILITY "XBIOS DMA sound 0x80-0x8D": "expected equal to `falcontc`"; "Not measured yet: no tosdump run on a Pi with
  the backend"). Fall **235** (`Buffptr`) bleibt Unterschied, Layout bewusst anders (ACCEPTED).
* Fall **213** `VgetSize(0x1C)`: **8294400** (1920x1080x4 Byte) statt QEMUs 1228800
  (XBIOS22 receipt, Punkt 6 "Real Pi 4 only: `VgetSize` = 8 294 400 at the default 1920x1080").
* Fall **87** `Getrez()`: `2`, gleich falcontc (XBIOS23). Die Quelle nennt fuer XBIOS23 nichts Pi-spezifisches
  ("Real Pi 4 only: nothing").
* Die uebrigen Unterschiede in QEMU (31 von 147 gegen falcontc: 70 105 128 129 178 179 183 189-194 213 215 218-227 230-235,
  `smoke-ref/bios-r1/tosdiff-EMUTOS-1a7484ab-BIOS-falcontc.txt`) koennen auf echter Hardware anders ausfallen
  (Giaccess, Floprate, Bconmap, NVRAM): erwarteter Wert **nicht festgelegt**, der Lead meldet die Liste.
Meldung: Foto (falls haengt), `BIOS.BIN` und `BIOS.LOG` (vom Mac aus der Karte).
Quelle: SND05 (Sammelliste "bios-r1 230-235"), XBIOS22, XBIOS23.

### A5 SNPROBE mit Backend (SND05-Rest)
`C:\PITEST\SNPROBE.NAP` doppelklicken. Sie spielt stille Puffer (alles 0), schreibt `SNPROBE.LOG` einmal am Ende in
`C:\PITEST\`. Dauer **nicht festgelegt** (Teile B-F messen Frame-Zeiten); kommt nach rund 3 Minuten kein Desktop zurueck:
Foto, Strom aus, melden.
Erwartet: erste Zeilen `A01 Locksnd() ret=00000001 (1)` (nicht `00000080`) und **kein** `B-F skipped: Locksnd answers its
function number`; die Teile B-F laufen. In QEMU ohne Backend endet das Log mit `B-F skipped` (`smoke-ref/snprobe/SNPROBE.LOG`).
Vergleich gegen das Orakel `measurements/measurement-2026-11-12-snd04/oracle/falcontc-run1/SNPROBE.LOG`: das Orakel ist
Hatari's Modell, nicht Silizium (Kopf von `tosdump/probes/snprobe.c`); bekannte, bewusste Abweichungen in
`docs/DEVIATIONS.md` ("XBIOS DMA sound": Frame-End-Interrupt wird nie ausgeloest, Aufnahme `EBADRQ`, `Buffptr` 64-bit).
Ein Soll fuer die Teile B-F auf dem Pi ist **nicht festgelegt**: die Datei wird nur zurueckgeliefert und verglichen.
Meldung: `SNPROBE.LOG`.
Quelle: SND05 (Sammelliste "snprobe"), SND04 (`measurements/measurement-2026-11-12-snd04/RUNINFO.TXT`).

### A6 DOOM im Standardstart (Basis fuer B9)
`C:\Apps\DOOM\DOOM.NAP` doppelklicken. Erwartet: das Shareware-Demo laeuft von selbst, Farben plausibel (gruene Waende,
brauner Boden), keine Streifen, keine Bildfehler. Meldung: Foto, Eindruck der Fluessigkeit (Zahl **nicht festgelegt**;
es gibt kein Soll, nur den Vergleich mit B9). Beenden wie bisher.
Quelle: V66 ("DOOM speed at 32 on silicon", `measurements/measurement-2026-11-18-v66-test/RUNINFO.TXT`).

### A7 Fbench im Standardstart (Basis fuer B10, V65-Kopiergeschwindigkeit)
`C:\Apps\FBENCH\FBENCH.NAP`, kompletter Lauf (115 Tests, je 1 s, wie am 2026-11-08), Ergebnis als Profil speichern
(Name z. B. `P16`, `LATEST.TXT` nicht ueberschreiben, falls moeglich). Zeilen, auf die es ankommt (V65 beruehrt die
Speicherform-Kopien): `Blit scr->scr 64x64`, `Blit mem->scr 64x64`, `Blit scr->mem 64x64`, `Mono blit (vrt_cpyfm)`,
`Blit 64x64, aligned`. Referenz: Lauf vom 2026-11-08 (`C:\Apps\FBENCH\OPTIMIZE.TXT`, in
`measurements/measurement-2026-11-08-v58-v59-pi/card/`): 38823 / 27971 / 24765 / 45055 / 57343 Ops/s. Lauf-zu-Lauf-Streuung
bis etwa 10 % (dieselbe RUNINFO). Der Kernel ist inzwischen ein anderer (V62-V68): eine Abweichung ueber der Streuung
wird gemeldet, nicht bewertet. Soll fuer die Lage **nicht festgelegt**.
Meldung: `LATEST.TXT` und das Profil von der Karte.
Quelle: V65 (ADR-036 Punkt 5, "goes into the collected real-Pi run").

### A8 TeraDesk: Video-Optionen, Mausklick auf Cancel (Standardstart)
`C:\Apps\TERADESK\DESKTOP.NAP` starten (diese Karte hat jetzt die V69-Fassung), Menue Options, "Video options...".
Erwartet: der Dialog oeffnet sich, `Colours: 64K`, alle Beschriftungen heil. **Mit der Maus auf Cancel klicken.**
Erwartet: Dialog schliesst sich. Befund in QEMU: der Injektor-Klick liess ihn offen und aenderte den Colours-Text auf `256`
(auch auf `main`, bei beiden Tiefen; Eigentuemer: kein Ticket, von Hand pruefen). Schliesst er sich nicht: melden, ob der
Text wechselt; dann mit der Undo-Taste schliessen (welche USB-Taste Undo ist, ist **nicht festgelegt**).
Meldung: geschlossen ja/nein, Foto, ob Colours-Text wechselte.
Quelle: STATUS 2026-11-19 ("a mouse click on Cancel ... check by hand"), V69 Teil B
(`measurements/measurement-2026-11-19-v69b-test2/RUNINFO.TXT`, Punkt 7).

### A9 PRESENT im Standardstart
`C:\Apps\PRESENT\PRESENT.NAP` doppelklicken. Mit Rechts-Taste oder Leertaste durch die 4 Folien, mit Esc beenden.
Erwartet: dunkelblaue Folie, Titel "NewTOS", orange Linie, Fuss "NewTOS - PRESENT01 test slides" und `1 / 4`
(Referenz `screendumps/qemu-present-slide1-default.png`, 640x480; auf dem Pi 1920x1080). Folie 2 zeigt `TEST.PNG`,
Folie 3 den Hinweis auf ein fehlendes Bild, Folie 4 umbrochene Zeilen. Esc bringt den Desktop zurueck.
Danach `C:\Apps\PRESENT\PRESENT.LOG` oeffnen (die Beispieldatei sagt `@log`): je Folie Anzahl, laengste Render-,
Bild- und Blit-Zeit. QEMU-Referenz nur fuer 16 Planes, 1920x1080: Blit 15-20 ms (`docs/sdk/14-present.md`, Abschnitt 5);
fuer Silizium **nicht festgelegt**.
Meldung: Foto einer Folie, `PRESENT.LOG`.
Quelle: V69 Teil C (`measurements/measurement-2026-11-19-v69c-test/RUNINFO.TXT`).

### A10 zView im Standardstart
`C:\Apps\ZVIEW\ZVIEW.NAP` starten, eine Datei oeffnen: `C:\Apps\ZVIEW\SAMPLES\T24.BMP`, dann `TRGB.PNG` (neu auf die Karte
gelegt). Erwartet: ein 64x48-Testbild mit Farbbalken, Verlauf und Schachbrett; Fenstertitel `64x48x24` (T24.BMP) bzw.
`64x48x8` (TRGB.PNG). Im Standardstart in 16-Bit-Qualitaet (sanfte Stufen im Verlauf, in QEMU max. Abweichung 7 je Farbe
gegen den 8-Bit-Wert). Meldung: Foto beider Bilder, Farben plausibel ja/nein.
Quelle: V69 Teil B (`measurements/measurement-2026-11-19-v69b-test2/RUNINFO.TXT`, Punkt 5).

### A11 (optional, nur mit USB-Seriell-Adapter) Bootzeile des Sound-Backends
Adapter an GPIO 14/15 (TX/RX/GND), Terminal 115200 8N1. `CMDSER.TXT` nach `CMDLINE.TXT` (Teil 0), kalt starten.
Erwartet im Terminal eine Zeile `[sound] PWM jack backend: clk_pwm measured <Hz> Hz, DMA channel 5, ring 2048 words at
bus 0x<...>, _SND = 0x14`. Der gemessene Takt liegt nach SND03 H1 bei etwa 107 143 019 Hz (FIFO-Zeit; 107 143 064 Hz laut
Firmware); wie nah die Zeile daran liegen muss, ist **nicht festgelegt**. Danach `CMDLINE.TXT` wieder leeren
(Teil 0, `vdiplanes=16`-Variante oder leer).
Meldung: die Zeile. Ohne Adapter: Schritt entfaellt, siehe Teil D.
Quelle: SND05-Testlauf Real-Pi-Punkt 1; SND03 (`TASK/SND03.md`, "Real-Pi run (owner, 2026-11-12)").

---

## Teil B - `vdiplanes=32`

**Umschalten auf 32** (Teil 0) und kalt starten.

### B1 Start, Mailbox-Weg, HDMI
Erwartet: Begruessungsbild, Desktop, Maus bewegt sich; Aussehen und Farben wie in A1 (die Unterschiede zu 16 Planes liegen
bei 8 Bit je Farbkanal statt 5; das Desktop-Gruen unterscheidet sich im Gruen-Kanal nur um 3 Stufen, 170 gegen 173,
V64, nur mit dem Auge pruefbar). Foto. Dass die Einstellung ueber die Mailbox ankam, siehst du in B2 (`32 planes`):
kaeme sie nicht an, stuende dort 16.
Quelle: V62 (`cmdline.txt` path through the VideoCore mailbox on silicon), V63-V66 "HDMI mode and the real framebuffer at 32".

### B1b (optional, mit Adapter) Bootzeile
`CMD32S.TXT` nach `CMDLINE.TXT`, kalt starten. Erwartet im Terminal `[vdi] VDI depth: 32 planes (vdiplanes=32, boot
argument)` (Referenz `serial/qemu-raspi4b-kern1119-vdiplanes32.log`). Quelle: V62.

### B2 SCRNINFO bei 32: Planes und `vq_scrninfo`
Wie A2. Erwartet (Referenz `smoke-ref/SCRNINFO-qemu-v32.TXT`, 640x480): `screen 1920x1080, 32 planes (vq_extnd(1)[4])`;
`vq_scrninfo`: `work_out[2]` = `32/0020`, `work_out[3]` = `256/0100`, `work_out[4]` = `0/0000`; Bits je Farbkanal 8/8/8
(`work_out[8..10]`), Bitnummern Rot 16..23, Gruen 8..15, Blau 0..7; Cookie-Jar mit `_SND`.
Meldung: Datei oder Foto. Quelle: V66 (`vq_scrninfo`), V62.

### B3 Alltagsbetrieb bei 32 Planes: AES, Maus, Tastatur, Cache-Kohaerenz
Fuenf Minuten Desktop wie im Alltag: alle Menues aufklappen und wieder schliessen, ein Dialog (z. B. Desk-Info) und eine
Alarmbox oeffnen und schliessen, ein Dateifenster oeffnen, verschieben, vergroessern, schliessen, Icons anklicken
(auswaehlen/abwaehlen), in der Konsole eine Zeile Text ausgeben. Erwartet: nach jedem Menue/Dialog/Alarm ist der Bildschirm
**ohne Rest** wiederhergestellt (V65: AES-Sicherungspuffer), Fensterrahmen und Icons in den Farben wie bei 16 Planes,
Text lesbar (V67: VT52-Zellschreiber), kein Haengen, kein Neustart, kein Flimmern. Meldung: Fotos von Auffaelligkeiten.
Ein Haenger oder Neustart hier waere der Verdacht auf Cache-Kohaerenz oder den TTBR0-Wedge (QUIRK-001): beides ist
nur mit diesem Lauf auf Silizium pruefbar, ein eigener Test dafuer existiert nicht.
Quelle: V65, V67 (`.../measurement-2026-11-18-v67-test/RUNINFO.TXT`, "USB-HID interactive use (clicking the icons)").

### B4 VDIDRAW bei 32 Planes
`C:\PITEST\VDIDRAW.NAP` doppelklicken. Erwartet: ein Blatt mit neun Zellen (Linien, Strichtypen, Zickzack, dicke Linie,
Pfeil), schwarz auf weiss, wie bei 16 Planes (Referenz `screendumps/qemu-vdidraw-vdiplanes32.png`, 640x480; das Blatt haelt
sich in QEMU rund 60 s). Meldung: Foto. Quelle: V63 (`measurement-2026-11-18-v63-test`, VDIDRAW-Aufnahmen x8 gleich).

### B5 Breite Rasterdumps auf Silizium (Cache-Kohaerenz bei 32 Planes)
`C:\PITEST\RAST32\DUMPRAST.NAP` doppelklicken (Tabelle `RASTER.TSV` = `vdi-raster32.tsv`, 64 Faelle; in QEMU etwa 19 s).
Erwartet in `C:\PITEST\RAST32\RASTER.LOG` (QEMU-Referenz `smoke-ref/rast32/RASTER32.LOG`): `RASTER.TSV: 64 cases, CRC 09A6D228`,
`table declares screen 640x480 at 32 planes; workstation reports 1920x1080 at 32 planes` (in QEMU: 640x480),
`table declares a 32-plane screen: NewTOS contract (ADR-038), no oracle`, `all 64 cases ran`,
`RASTER.BIN: 64 records, 367648 bytes`. Verweigert der Lauf mit "screen mismatch: refusing to run the table", ist das der
Befund (der 16-Plane-Dumper nahm 1920x1080 an; fuer die breite Tabelle **nicht festgelegt**).
Am Mac (Lead): `python3 -I tosdump/host/contract32-check.py tosdump/cases/vdi-raster32.tsv RASTER.BIN` muss mit Exit 0 und
`0 of 64 cases differ from the NewTOS contract` enden (QEMU: `smoke-ref/rast32/contract32-check.txt`).
Meldung: `RASTER.BIN`, `RASTER.LOG`. Quelle: V69 Teil A (`measurements/measurement-2026-11-19-v69-test2/RUNINFO.TXT`,
"silicon timing of the wide dumps"; ADR-038 Punkt 13).

### B6 TeraDesk: Video-Optionen bei 32 (HDMI-Farben, Cancel per Maus)
TeraDesk starten, Options, "Video options...". Erwartet: Dialog oeffnet sich, `Colours: 64K`, keine zerschossenen Texte
(ohne den V69-Patch stuende `4194304K` ueber dem Rahmen; Referenz `.../measurement-2026-11-19-v69b-test2/screendumps/
open-wt-v32-dialog.png` gegen `open-main-v32-dialog.png`). Farben des Dialogs wie bei 16 Planes (HDMI, nach Augenschein).
Dann **mit der Maus auf Cancel klicken** (wie A8) und melden, ob er sich schliesst. Dialog nicht per OK verlassen
(OK loest die Alarmbox "A change of video mode will terminate all running applications." aus; das war in QEMU harmlos, ist
aber hier nicht verlangt).
Meldung: Foto des Dialogs, Cancel ja/nein. Quelle: V69 Teil B, STATUS 2026-11-19.

### B7 zView bei 32: HDMI-Farben
Wie A10. Erwartet: dieselben Bilder, jetzt in 8-Bit-Genauigkeit (in QEMU stimmten 7776 von 7776 Farbwerten exakt mit dem
dekodierten Beispiel ueberein); der Verlauf im Testbild ohne sichtbare Stufen. Titel `64x48x24` / `64x48x8`.
Meldung: Foto beider Bilder, Farben plausibel ja/nein, Vergleich zu A10. Quelle: V69 Teil B, STATUS 2026-11-19.

### B8 PRESENT bei 32: Folien und Zeit der 32-Bit-Expansion
Wie A9. Erwartet: dieselben Folien wie bei 16 Planes (die Farben behalten 16-Bit-Qualitaet, die Folie wird erst in einen
32-Bit-Puffer erweitert; Referenz `screendumps/qemu-present-slide1-vdiplanes32.png`, gleich dem Standardstart-Bild).
`PRESENT.LOG` lesen: die Blit-Zeit enthaelt die Expansion. Vergleich A9 gegen B8; QEMU hat bei 32 keine Zeit gemessen
(V69c RUNINFO: "no timing measured, no claim made"), Soll **nicht festgelegt**.
Meldung: Foto, `PRESENT.LOG`. Quelle: V69 Teil C, STATUS 2026-11-19 ("time of PRESENT's 32-bit expansion").

### B9 DOOM bei 32
Wie A6. Erwartet: das Demo laeuft, Farben plausibel (gruen/braun), keine Streifen; DOOM prueft seine 256 Farben selbst
(`EdDI check: 256 of 256 test colours exact` steht nur auf der seriellen Leitung, am Pi ohne Adapter nicht sichtbar). Meldung: Foto, Fluessigkeit gegenueber A6 (Zahl **nicht festgelegt**).
Quelle: V66 ("DOOM speed at 32 on silicon").

### B10 Fbench bei 32 (V65: Kopiergeschwindigkeit gegen die 16-Bit-Naht)
Wie A7, Profil z. B. `P32`. Dieselben fuenf Zeilen vergleichen: A7 gegen B10 (und gegen den Lauf vom 2026-11-08). Ob Fbench
bei 32 Planes ueberhaupt laeuft oder sich anders meldet, ist **nicht festgelegt** (in QEMU nicht gefahren); meldet es einen
Fehler oder haengt es, ist das der Befund. Es gibt kein Soll, ADR-036 Punkt 5 verlangt eine Messung auf dem Pi.
Meldung: `LATEST.TXT`/Profil. Quelle: V65.

### B11 SNDTONE bei 32 (nur Konsolentext)
Wie A3, aber nur auf den Text achten: Konsolenzeilen bei 32 Planes lesbar, richtige Textfarbe (V67). Ton wie A3.
Meldung: Foto. Quelle: V67 (VT52-Zellschreiber bei 32).

### B12 Zurueck auf Standardstart
`CMDLINE.TXT` wieder leeren (oder `CMD16.TXT` darauf kopieren), kalt starten, SCRNINFO zeigt `16 planes`. Pruefung A1/A2-Kurzform.

---

## Teil C - SND03: Hoertest der Ton-Sonde (Kernelwechsel am Mac)

Der Ticketstand von `TASK/SND03.md`: H1-H4 gemessen, der **Hoertest ist offen** ("keep the hearing test open", kein Geraet mit
Klinke). Am 2026-11-18 war ein Ton an der Klinke hoerbar (SND05, SNDTONE); der Lead entscheidet, ob das den SND03-Hoertest
ersetzt. Wenn nicht: Kernel `kerns03.img` liegt auf der Karte (SND03-Lauf). Sheet und Erwartung stehen unveraendert im
`measurements/measurement-2026-11-12-snd03/TESTBLATT.md`: drei Seiten H4 / H3+H5 / H2+H1+Ton, Toene T1-T4 (T1 440 Hz beide,
T2 440 Hz nur links, T3 880 Hz nur rechts, T4 Mark/Space), je eine Taste weiter. **Abnahme SND03: mindestens T1 hoerbar.**
Kernelwechsel (nur am Mac, Absprache mit dem Lead): `cp -X "/Volumes/BOOT 1/kerns03.img" "/Volumes/BOOT 1/kernel8.img"`, nach
dem Test zurueck mit `cp -X "/Volumes/BOOT 1/kern1119.img" "/Volumes/BOOT 1/kernel8.img"`. Der Wechsel steht **nicht** im
Sync-Befehl, die Karte bekommt ihn nur auf Ansage.
Nicht Teil dieses Laufs: das Update von `docs/hardware/pwm-audio.md` (Dokumentarbeit des Leads).

---

## Teil D - Was sich nicht (voll) vorbereiten liess

| Punkt | Grund | Behelf |
|---|---|---|
| Bootzeile des Backends, `[vdi]`-Zeile | steht nur auf der seriellen Leitung (Konsolenausgabe des Kernels geht nicht auf den Bildschirm) | A11 / B1b mit Adapter; ohne Adapter zeigen A2 und B2 dieselbe Tatsache (`_SND`, Planes) |
| QUIRK-001 (TTBR0-Wedge) | kein eigener Test, er zeigt sich als Haenger | B3 (Dauer-Alltag) als Beobachtung, kein Bestehen-Kriterium |
| Echter Monitor / EDID | kein Programm prueft das | A2/B2 melden die Bildschirmgroesse, A1/B1 das Bild |
| Tonhoehe | Soll und Toleranz in keinem Ticket | A3 per Tuner-App, nur melden |
| DOOM-/Fbench-/PRESENT-Zeiten | kein Soll in V65, V66, V69 | nur Vergleich A gegen B |
| Umschalten am Pi allein | nicht gefahren (kein Real-Pi) | Teil 0, Mac-Weg ist der gesicherte |
| XBIOS23 | die Quelle nennt nichts Pi-spezifisches | nur Fall 87 in A4 |
| V68 | nichts Pi-spezifisch (Antworten ohne Hardwarezugriff, Option nur QEMU) | keiner |
| HighWire | kein gebautes Programm (V69) | keiner |

## Rueckgabe

Fotos (A1, A3, A8, A9, A10, B1, B3, B4, B6, B7, B8, B9, bei Haengern auch diese), die Dateien `SCRNINFO.TXT`,
`SNPROBE.LOG`, `BIOS.BIN`, `BIOS.LOG`, `RASTER.BIN`, `RASTER.LOG`, `PRESENT.LOG`, `LATEST.TXT` (Fbench) vom Mac aus der Karte
(`cp -X`), je Schritt PASS / FAIL / nicht gefahren. Der Lead wertet `BIOS.BIN` (tosdiff) und `RASTER.BIN`
(`contract32-check.py`) am Mac aus.

## Abdeckung der Sammelliste

| Sammelliste (STATUS 2026-11-18/19) | Schritt |
|---|---|
| SND05: bios-r1 230-235 | A4 |
| SND05: snprobe | A5 |
| SND05: Bootzeile des Backends | A11 |
| SND05: `_SND` im Jar | A2, B2 |
| SND05: Tonhoehe | A3 |
| V65 Kopiergeschwindigkeit | A7, B10 |
| `vdiplanes=32`: Mailbox-Weg | B1, B2, B1b |
| `vdiplanes=32`: HDMI | B1, B3, B4, B6, B7, B8, B9 |
| `vdiplanes=32`: Cache-Kohaerenz | B3, B5 |
| XBIOS22 Real-Pi (VgetSize) | A4 (Fall 213) |
| XBIOS23 Real-Pi | A4 (Fall 87), sonst keiner |
| V62-V69 Real-Pi-Punkte | B1-B11 (V62 B1/B2, V63/V64 B1/B4, V65 A7/B10, V66 B2/B9, V67 B3/B11, V68 keiner, V69 B5-B8) |
| HDMI-Farben TeraDesk-Dialog / zView / PRESENT bei 32 | B6 / B7 / B8 |
| Zeit der 32-Bit-Expansion von PRESENT | A9 gegen B8 |
| Mausklick auf Cancel im Video-Dialog | A8, B6 |
| SND03 Real-Pi-Abnahme | Teil C |

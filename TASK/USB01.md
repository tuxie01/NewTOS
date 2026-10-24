# USB01 — USB-Maus: nur Boot-Protokoll, Steckkombinationen, Diagnosewerkzeug

Status: **OFFEN** · Prioritaet: **P1** · Typ: Treiberluecke + offene Defekte — Anwender 2026-10-22 („die Maus ist wichtig“)

## Sachverhalt

* **Ein Protokoll.** `kernel/src/drivers/usb/mod.rs:139-152` dekodiert nur
  HID-Boot-Berichte (`bInterfaceProtocol` 1 = Tastatur, 2 = Maus,
  `hid.rs`). Maeuse, die nur das Report-Protokoll sprechen, mit Report-IDs
  senden (Funkempfaenger, Verbundgeraete), 16-Bit-Deltas haben oder absolute
  Zeiger liefern (Tablets, KVM, QEMU `usb-tablet`), bleiben stumm oder
  werden falsch gelesen. Vergleich: Circle parst den HID-Report-Deskriptor
  (`references/circle/lib/usb/usbmouse.cpp:28-70`), FreeMiNT hat getrennte
  Treiber `mouse`, `mouse-boot`, `tablet` (`references/freemint/sys/usb/src.km/udd/hid/`).
  Anwender: „bei Maeusen gibt es verschiedene Protokolle und ich denke wir
  unterstuetzen nur ein Protokoll“.
* **Steckkombination** (seit 2026-09-04, `docs/STATUS.md` Recent Decisions
  2026-09-09): Maus und Tastatur liefen auf der Pi nur in einer Kombination.
  Damals vier Defekte beim Lesen belegt, keiner repariert: (1) `wait_transfer`
  nimmt das erste Transfer-Event ohne Slot/Endpoint-Pruefung, (2)
  `poll_interrupt` ordnet nur ueber die Slot-ID zu, ohne Completion-Code und
  Endpoint, (3) alles ausser Protokoll 1 wurde Maus — inzwischen teilweise
  behoben (`mod.rs:145-152`), (4) Hub-Port-Reset Einzelschuss 60 ms statt
  FreeMiNTs 5 × 150 ms. Stand heute ungeprueft
  (`docs/analysis/inventory-2026-10-08.md:62,198`).
* **Instrument vorhanden:** `--features usb-diag` schreibt den Bootlog nach
  `C:\USBDIAG.LOG` (`kernel/src/bootlog.rs`), aber nur waehrend des Boots.

## Auftrag

1. **Analyse (Rechercheur, nur lesen):** Treiber gegen Circle und FreeMiNT,
   Ergebnis als Spec-Ergaenzung unter `docs/api/usb/`:
   - welche Protokolle/Geraeteklassen es gibt (Boot, Report mit Deskriptor,
     Report-IDs, Verbundgeraete mit mehreren Interfaces, absolute Zeiger)
     und wie Circle und FreeMiNT sie erkennen und dekodieren;
   - Zustand der Defekte 1–4 im heutigen Code;
   - Hotplug: was passiert beim Umstecken nach dem Boot.
2. **Diagnosewerkzeug** (Anwenderwunsch): ein Programm, das zu jedem
   angeschlossenen Geraet die Informationen ausliest und in eine Datei auf
   `C:` schreibt — Geraete-, Konfigurations-, Interface-, Endpoint- und
   HID-Report-Deskriptor (roh, hex), Port/Hub-Weg, Geschwindigkeit, das
   gewaehlte Protokoll und einige rohe Berichte bei Bewegung/Klick. Der
   Weg (EL0-Programm ueber eine neue Kernel-Schnittstelle oder Ausbau von
   `usb-diag`) wird in Schritt 1 vorgeschlagen und vom Anwender gewaehlt.
   Zweck: der Anwender steckt verschiedene Maeuse an, die Dateien belegen,
   was jede Maus spricht.
3. **Ursache belegen, dann Fix-Vorschlag** (Report-Protokoll-Parser,
   Defekte 1, 2, 4) — vorlegen, nicht direkt bauen (Regel 5).

## Stand 2026-10-23

Schritt 1 erledigt: `docs/api/usb/hid-mouse-protocols.md` (`29c3aa50`), Fix-Liste F0–F9 dort.
Anwender-Freigabe 2026-10-23: **F1** (Befehlsring mit Link-TRB), **F2**
(`bInterfaceClass == 3` vor Protokoll 1/2), **F8** (Slot-Budget/stille Abbrueche
loggen) bauen. Diagnoseweg, F3–F7 noch offen.

F1/F2/F8 umgesetzt (`0a1b80f4`, `7e7fbafa`, `190281c6`, `7c1f621d`), Tester PASS
(`34f73b01`). Echte Pi offen: Steckkombinationen, >15 Befehle am VL805,
`C:\USBDIAG.LOG`. Nebenbefunde: [USB02](USB02.md).

## Stand 2026-10-23 (Diagnosewerkzeug, Schritt 2)

Anwender waehlt Weg (a). Spec-ENTWURF: [`docs/api/usb/usb-snapshot.md`](../docs/api/usb/usb-snapshot.md)
(`Ssystem`-Modus 2022 + Konsolenprogramm `USBSNAP.NCP`, eine Textdatei je Geraet auf `C:\`),
Modus-Zeile als DRAFT in `docs/api/gemdos/154-ssystem.md`. **Nichts gebaut**, wartet auf
Freigabe und die Antworten zu §7 des Entwurfs. Wichtigster Befund: seit F2 werden
Report-only-Geraete gar nicht mehr konfiguriert — der Kern braucht einen Beobachtungs-Pfad,
sonst zeigt das Werkzeug genau die Maeuse nicht, derentwegen es gebaut wird. Aufwand
geschaetzt 7–9 Tage (ohne Hardware-Iterationen).

**Echte Pi 2026-10-23** (`measurements/measurement-2026-10-23-usb01-con04-pi/RUNINFO.TXT`): Apple-Tastatur + ROCCAT-Maus in zwei
Steckpositionen, je 4 Geraete, 13 Befehle, kein Fehler, Eingabe geht; `C:\USBDIAG.LOG`
wird geschrieben. Defekte 1, 2, 4 damit auf der Pi abgenommen (fuer diese Geraete).
Offen: >15 Befehle, Report-only-Maeuse. Paket fuer den externen Tester (usb-diag-Kernel
`86205b81…` + LIESMICH/NOTIZEN) liegt beim Anwender; Diagnosewerkzeug USBSNAP:
Spec `docs/api/usb/usb-snapshot.md` am 2026-10-23 freigegeben (alle Empfehlungen §7,
Ausgabe `C:\USB\`), Umsetzung K1–K12 beauftragt.

## Stand 2026-10-23 (Schritt 2, Kernelteil gebaut)

K1–K9 der Spec `docs/api/usb/usb-snapshot.md` §5 umgesetzt (`cf396382`): `Ssystem(2022)`
liefert Geraeteliste, Rohdeskriptoren, Report-Deskriptoren, Hersteller-/Produktstring und
den Berichtsring je Geraet; MUTE bis 60 s. Report-only-Maeuse werden jetzt beobachtet
(konfiguriert und aufgezeichnet, nicht dekodiert); Interrupt-TRB-Laenge = Max-Packet.
QEMU-Belege `measurements/measurement-2026-10-23-usb01-snap-kernel/RUNINFO.TXT`
(Eingabe wie vorher, EL0-Probe, raspi4b `ERROR`, APITEST 109/0/5). Umsetzungsnotizen fuer
das Programm: Spec §9. **Offen:** Programm `USBSNAP.NCP` (K10), QEMU-Option (K11),
Tester-Abnahme, Pi-Test mit Apple-Tastatur + ROCCAT (Report-/String-Abruf auf dem VL805,
TRB-Laenge 64 der ROCCAT).

## Stand 2026-10-23 (Schritt 2, Programmteil gebaut)

K10/K11 der Spec umgesetzt: `sdk/examples/usbsnap.c` → `USBSNAP.NCP` (`46d37272`, Image-Eintrag in
`tools/make-sd-image.sh`), `cargo xtask run --usb-extra`/`--usb-dev` (`46a027e2`). Kernel-Nachbesserungen
(`53cb16c3`): Schnappschuss-Speicher in `.bss` (Image −225 KB) und — Befund aus T6 — Loslassen gehaltener
Tasten/Knoepfe am MUTE-Beginn (sonst wiederholte sich die Start-RETURN-Taste 30 s lang). Entscheidungen der
Umsetzung: Spec §9.1. QEMU-Belege `measurements/measurement-2026-10-23-usb01-snap-program/RUNINFO.TXT`
(T5, T6 geteilt auf qemu-virt + raspi4b-Wiedergabe, T7, Regression). **Offen:** Tester-Abnahme; echte Pi (T8):
Doppelklick-Start, Klick auf den Desktop waehrend der Aufnahme, Dateien von ROCCAT und Apple-Tastatur.

**USBSNAP (K10/K11) Tester PASS 2026-10-23** (`99c2c33d`): alles QEMU-Erreichbare
gruen, keine Regression; Restluecke nur T8 auf der echten Pi (echte Aufnahme + echte
Dateien in einem Lauf). Kosmetik nach USB02 Punkt 9. Pi-Test wartet auf den Anwender:
Kernel `56e4e78b…` + `USBSNAP.NCP` `e46feb9d…` (aus `fe30bfc1`), danach Paket fuer Ragnar.

[Gemeinsame Regeln](README.md) gelten.

## Stand 2026-10-23 — Ragnars Lauf: Ursache der Funkmaeuse ist F3

`measurements/measurement-2026-10-23-usb01-ragnar/RUNINFO.TXT`: Unifying
(`046d:c52b`), Rii X8 (`1997:2433`) und Cherry DW 9000 (`046a:0115`) bieten die
Boot-Maus auf Interface 1 hinter einer Boot-Tastatur auf Interface 0; die
F2-Regel „ein Interface je Geraet“ ueberspringt sie. **F3 ist damit der
naechste Schritt, nicht F4.** Offen: mit Empfaenger geht die Kabeltastatur
nicht (Verdacht, unbewiesen: gemeinsamer Tastaturzustand `hid.rs:47`);
USBSNAP-Aufnahme mit Empfaenger erbeten. Spec-Abschnitt F3 in
`docs/api/usb/hid-mouse-protocols.md` folgt vor dem Code.

**Nachtrag 2026-10-23:** Spec-Entwurf F3 steht in
`docs/api/usb/hid-mouse-protocols.md` §9 (ENTWURF, Freigabe offen, zehn
Fragen in §9.9). Richtigstellung zum Lauf `measurement-2026-10-23-usb01-ragnar`:
`serial/` enthaelt **10** Logs, nicht 11 wie RUNINFO und Commit `cc0e76aa`
sagen; der Log zu Start 8 (Cherry DW 9000, blaue Buchse) fehlt in Ragnars
Sendung.

## Stand 2026-10-24 — F3 gebaut

Spec `docs/api/usb/hid-mouse-protocols.md` §9 freigegeben (Entscheidungen §9.11) und umgesetzt
(`be506300`): jedes HID-Boot-Interface (Alt 0) eines Geraets ist eine Funktion mit eigenem
Interrupt-Endpoint — die Maus eines Funkempfaengers wird jetzt konfiguriert und gepollt;
Tastatur-/Maustastenzustand je Funktion; `Ssystem(2022)` Layout v2 und `USBSNAP.NCP` v2
(`docs/api/usb/usb-snapshot.md` §10). Hosttests 1630/0/1, QEMU-Regression unveraendert, der
Mehr-Interface-Pfad ist in QEMU nicht nachbildbar (Host-Tests + Wiedergabe):
`measurements/measurement-2026-10-24-usb01-f3/RUNINFO.TXT`. **Offen:** Tester-Abnahme; echte Pi
P1–P5 (§9.8): Pi-Kandidat rpi4-`kernel8.img` `edc9c4d1…` + `USBSNAP.NCP` `522c8e94…` (beide aus
`be506300`), danach Paket fuer Ragnar (Rueckfragen GitHub #3).

## Stand 2026-10-24 — Pi-Lauf F3 + USBSNAP T8 (Anwender)

`measurements/measurement-2026-10-24-usb01-f3-pi/RUNINFO.TXT`: Enumeration
byte-gleich zum Kern vor F3 (P1 ohne Aenderung), USBSNAP v2 zeichnet beide
Geraete mit 0 Fehlern auf und schreibt `C:\USB\05AC020C.TXT` und
`1E7D2CEE.TXT`. **T8 damit erledigt.** Anwender: „nach der Rueckkehr sah alles
wieder normal aus“ (Zeiger da, Desktop reagiert normal). Offen: P2/P3 mit
Funkempfaenger (ragnar76), Tester-Bericht zu F3, USB02 Punkt 10
(ErrorRollOver).

**Tester F3 (2026-10-24): PASS** — `measurements/measurement-2026-10-24-usb01-f3-test/RUNINFO.TXT`
(`a0dec932`). Host 1630/0/1 (+19), keine neuen Warnungen, QEMU ohne Regression
(10 Topologie-Paare pixelgleich), USBSNAP v2 gegen den echten Kern, Ragnars
Deskriptoren per Mutationstest belegt, Zustand je Funktion per temporaerem
In-Kernel-Selbsttest belegt. Eine Abweichung Code gegen Spec (Rueckfall nach
Timeout) als USB02 Punkt 11. **Richtigstellung:** der vom Developer genannte
rpi4-Release-Hash `edc9c4d1…` ist nicht reproduzierbar (der Kern stempelt Git-Rev
und Datum ein; der Developer-Baum hatte einen anderen Stand). Massgeblich fuer
den Pi-Lauf ist `fe967851…` (usb-diag, gebaut vom Lead aus `be506300`).

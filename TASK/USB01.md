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
`86205b81…` + LIESMICH/NOTIZEN) liegt beim Anwender; Diagnosewerkzeug USBSNAP
wartet auf die Antworten zu `docs/api/usb/usb-snapshot.md` §7.

[Gemeinsame Regeln](README.md) gelten.

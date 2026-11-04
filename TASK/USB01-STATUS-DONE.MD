# USB01 — Quittung DONE (2026-11-04)

## Getan

- Schritt 1, Analyse: `docs/api/usb/hid-mouse-protocols.md` (`29c3aa50`),
  Fix-Liste F0–F9.
- F1 Befehlsring mit Link-TRB, F2 Klassenpruefung, F8 stille Abbrueche
  (`0a1b80f4`, `7e7fbafa`, `190281c6`); Tester PASS
  `measurements/measurement-2026-10-23-usb01-test/RUNINFO.TXT`.
- Schritt 2, Diagnosewerkzeug: `Ssystem(2022)` (`cf396382`, `53cb16c3`),
  `USBSNAP.NCP` (`46d37272`), `--usb-extra`/`--usb-dev` (`46a027e2`);
  Spec `docs/api/usb/usb-snapshot.md`.
- F3 mehrere HID-Funktionen je Geraet (`be506300`); Tester PASS
  `measurements/measurement-2026-10-24-usb01-f3-test/RUNINFO.TXT`.

## Abnahme auf echter Hardware

- Eigene Pi 2026-10-23: zwei Steckpositionen, 13 Befehle, Eingabe geht,
  Defekte 1, 2, 4 abgenommen
  (`measurements/measurement-2026-10-23-usb01-con04-pi/RUNINFO.TXT`).
- Eigene Pi 2026-10-24: F3 ohne Aenderung der Enumeration, USBSNAP T8
  (`measurements/measurement-2026-10-24-usb01-f3-pi/RUNINFO.TXT`).
- Ragnar 2026-10-25 mit Paket 2 (`usbtest-2026-10-24`, Kern aus `19ff79f6`):
  vier Funkempfaenger an beiden Buchsen, Zeiger, Tasten, Rad, Kabel- und
  Funktastaturen; 10 Aufnahmen, 0 verloren
  (`measurements/measurement-2026-10-25-usb01-ragnar2/RUNINFO.TXT`). Rad der
  DW 9000 von Ragnar am 2026-10-26 in GitHub #3 bestaetigt.

## Offen, mit eigenem Platz

- Mehr als 15 Befehle auf dem VL805: Testszenario fuer die Tester.
- Report-only-Maeuse und Taste 3: [USB03](USB03.md) (P1).
- AES-Menue haengt / Hintergrund bleibt schwarz (Ragnar, #3): [A127](A127.md).
- F5-Rest, F6, F7, F9: [USB04](USB04.md) (P3, nach 1.0).

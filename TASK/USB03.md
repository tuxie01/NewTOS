# USB03 — Mausrad von Kabelmaeusen, die es nur im Report-Protokoll schicken (B100, GitHub #8)

Status: **OFFEN** · Prioritaet: **P2** · Typ: Treiberluecke — aus Ragnars USB-Test 2 (2026-10-25)

## Befund

`measurements/measurement-2026-10-25-usb01-ragnar2/RUNINFO.TXT`: Die Logitech
B100 (`046d:c077`, low speed) schickt im Boot-Protokoll 3 Bytes (Tasten, X, Y).
Ihr Report-Deskriptor (46 Bytes) erklaert X, Y und Wheel (Usage 0x38) als
3 x 8 Bit; das Rad gibt es nur im Report-Protokoll. Die Funkempfaenger schicken
schon im Boot-Protokoll ein 4. Byte und das Rad laeuft dort. Wosch (#8) meldet
dasselbe fuer die RPi-Maus und mehrere Logitech-Kabelmaeuse.

## Auftrag

Quellen zuerst (USB HID 1.11 Device Class Definition: Boot-Protokoll Anhang B,
SET_PROTOCOL, Report-Deskriptor-Items; HUT 1.5 `references/usb/hut1_5.pdf`),
Spec in `docs/api/usb/` vor Code. Dann: Maus mit Rad im Report-Deskriptor im
Report-Protokoll betreiben und Tasten/X/Y/Rad ueber einen Deskriptor-Leser
dekodieren; Rueckfall auf Boot-Protokoll, wenn der Deskriptor nicht lesbar ist.
Hosttests gegen die echten Deskriptoren und Berichte aus den Messlaeufen.
Abnahme: echte Pi (B100).

[Gemeinsame Regeln](README.md) gelten.

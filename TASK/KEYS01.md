# KEYS01 — Tastenwiederholung fehlt (Doom bleibt bei gehaltener Taste stehen)

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt KEYS01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 20f6b941d (feat(ikbd): held keys repeat like EmuTOS, driven f), named by body

Line 3 before the migration:
Status: **ERLEDIGT — Pi-Abnahme 2026-10-03** · Prioritaet: **P1** (vor dem Termin) · Typ: Kernel / BIOS-Tastatur — aus [PORT06-KEYS-DIAGNOSIS](PORT06-KEYS-DIAGNOSIS.MD), Anwenderentscheidung 2026-10-02

Abhaengigkeiten: —

## Sachverhalt

Die Diagnose (`TASK/PORT06-KEYS-DIAGNOSIS.MD`, b942bc3) hat gezeigt: NewTOS
erzeugt keine Tastenwiederholung. `kernel/src/drivers/usb/hid.rs` meldet nur
Aenderungen (ein Make, ein Break), `kernel/src/ikbd.rs` legt pro Make genau ein
Wort ab, und die 50-Hz-Wiederholung von EmuTOS (`bios/ikbd.c:451-536`, Stopp
`:816-817`) fehlt. `Kbrate` liefert `0x0F02`, aber nichts liest die Werte.
`tos-api/src/bios/tick.rs:40-41` und `docs/api/xbios/23-kbrate.md` begruenden
die Luecke falsch mit „USB-HID macht das selbst“. Doom gibt eine gehaltene
Taste nach 360 ms auf, weil keine Wiederholung kommt.

## Entscheidung (Anwender 2026-10-02)

**Variante (b):** die Wiederholung entsteht im gepollten Eingabe-Pump, nicht
im Timer-Interrupt. Bei jedem Poll erst die USB/IKBD-Eingabe lesen (ein Break
wird gesehen), dann die seit dem letzten Poll per `hz_200` verstrichenen
50-Hz-Schritte gutschreiben. Gleiche Verzoegerungs-/Raten-Semantik wie EmuTOS
(KB_RATE aus `Kbrate`, `conterm` Bit 1, Stopp beim Break der wiederholten
Taste, die zuletzt gedrueckte Taste wiederholt, Modifier-Wechsel wie EmuTOS),
keine Geistertasten nach dem Loslassen. Benannte Abweichung im *Ort*, mit
Begruendung, in Spec und `docs/COMPATIBILITY.md`.

## Abnahme

* Spec korrigiert (23-kbrate, Repeat-Eintrag, tick.rs-Kommentar).
* Hosttests fuer die Zustandsmaschine: Verzoegerung, Rate, Stopp beim Break,
  letzte Taste gewinnt, Kbrate lesen/setzen, conterm aus, lange Luecke.
* QEMU raspi4b mit Injektor: Pfeil hoch 5 s halten → Strom von
  Wiederholungen nach der Verzoegerung, Loslassen → Strom endet; Doom laeuft
  die ganze Haltezeit (Diagnoselauf als vorher/nachher).
* Basislinien gehalten; echte Pi: Doom laeuft bei gehaltener Taste weiter.

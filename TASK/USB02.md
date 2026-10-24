# USB02 — Kleinbefunde aus USB01 F1/F2/F8

Status: **OFFEN** · Prioritaet: **P3** · Typ: Sammelticket — Nebenbefunde Developer/Tester 2026-10-23
(`measurements/measurement-2026-10-23-usb01-f1f2f8/RUNINFO.TXT`, `measurements/measurement-2026-10-23-usb01-test/RUNINFO.TXT`)

1. **Route-String:** Hub-Ports > 15 werden mit `p & 0xf` maskiert; xHCI 1.2
   Tabelle 6-4, Fussnote 106 verlangt 15 ab mehr als 14 Ports. Hubtiefe > 5
   wird nicht geprueft.
2. **`open`:** mehr als 64 Scratchpad-Puffer und mehr als 16 Slots werden still
   gekappt (F8-Regel: loggen).
3. **Hub nur ueber Interface-Klasse** (Geraeteklasse 0, Interface-Klasse 9) wird
   nicht als Hub erkannt, seit F2 abgelehnt und geloggt.
4. **Boot-Schirm** (`kernel/src/initinfo.rs:336`) beschriftet Hubs und Nicht-HID
   als „HID device“ und zeigt hoechstens 4 Geraete.
5. **Nicht-HID-Geraete belegen einen `MAX_HID`-Eintrag.**
6. **`bootlog.rs`:** zwei Warnungen „variable does not need to be mutable“ im
   `usb-diag`-Bau.
7. **QEMU 11.0.2:** QMP `input-send-event` mit `device=<usb-id>` bricht ab
   („Property 'qemu-fixed-text-console.device' not found“) — Messhinweis, kein
   NewTOS-Defekt.

8. **Boot-Schirm mit echten Geraetenamen** (Anwender 2026-10-23, perspektivisch):
   statt `HID device 05ac:1003` den Produktstring des Geraets zeigen, z. B.
   `Apple Keyboard`, plus Hub-Port statt nur Root-Port (heute alles „Port 1“, weil am
   Pi 4 alle USB-2-Geraete hinter dem VL805-internen Hub haengen). Voraussetzung: der
   Stringabruf aus USB01/USBSNAP (K6, `docs/api/usb/usb-snapshot.md`). Zu klaeren:
   UTF-16LE auf den Zeichensatz des Boot-Fonts abbilden, Laenge kappen, Rueckfall
   VID:PID, wenn das Geraet keinen String hat. Zusammen mit Punkt 4 umsetzen.
   **Darstellung (Anwender-Vorbild: macOS Systeminformationen, USB-Baum):** Kette
   als eingerueckter Baum nach Hub-Ebene, nicht als flache Liste; Wurzel je
   Root-Port/Bus, Hubs als Knoten mit ihren Geraeten darunter, je Zeile nur der Name
   (Rolle/VID:PID hoechstens knapp dahinter). Fuer die Pi mit Apple-Tastatur + ROCCAT
   also etwa:
   ```
   USB 2.0 Bus
     VL805 Hub
       Keyboard Hub
         Apple Keyboard
       ROCCAT Kone
   ```
   Daten dafuer liegen nach USB01 K4 vor (`parent`, Route je Geraet).

9. **USBSNAP-Kleinigkeiten** (Tester 2026-10-23, `measurements/measurement-2026-10-23-usb01-snap-program-test/RUNINFO.TXT`):
   (a) `Store::mute_state`: `mute_seen` wird nur von einem Poll zurueckgesetzt, der
   „nicht gemutet“ sieht — MUTE aus und sofort wieder an innerhalb eines Poll-Abstands
   loest keinen zweiten Release aus (nur gelesen, USBSNAP selbst tut das nie);
   (b) wenn alle Dateien uebersprungen werden, druckt das Programm „Done. Wrote:“ vor
   den Skip-Zeilen und nennt nur den zuletzt probierten Namen (`.T99`) — kosmetisch.

Je Punkt: gegen Circle/FreeMiNT und xHCI 1.2 (`references/xhci/`) pruefen, dann beheben.

[Gemeinsame Regeln](README.md) gelten.

## 10. ErrorRollOver-Bericht wird wie ein normaler Bericht ausgewertet (2026-10-24)

Beleg: `measurements/measurement-2026-10-24-usb01-f3-pi/usb/05AC020C.TXT`
seq 117 — die Apple-Tastatur meldet bei drei gehaltenen Tasten
`00 00 01 01 01 01 01 01` (HID-Fehlercode ErrorRollOver, Usage 0x01).
`hid_decode::keyboard_diff` (`kernel/src/drivers/usb/hid_decode.rs:74`)
vergleicht ihn wie jeden Bericht: die gehaltenen Tasten bekommen einen Break,
0x01 hat keinen Scancode, der naechste Bericht macht sie wieder. Folge: ein
falsches Break/Make-Paar (Wiederholung bricht ab oder Zeichen doppelt). Soll
laut HID-Spec und den Referenzen erst belegen (HID 1.11 Usage Tables
„ErrorRollOver“, FreeMiNT `usb_keyboard.c`, Circle), vermutlich: Bericht
verwerfen, Zustand behalten. Klein, betrifft jede Boot-Tastatur.

**Stand 2026-10-24: GESTOPPT, keine Quelle nennt das Host-Verhalten.** HID 1.11
(`references/usb/hid1_11.pdf`) Anh. C S. 62 und F.3 S. 74 schreiben nur die Pflicht
des *Geraets* vor (ErrorRollOver in allen Array-Feldern bei > 6 Tasten oder Ghosting;
Tabelle Anh. C S. 63: „Modifiers still reported“), nicht, was der Host damit tut.
HID Usage Tables liegen nicht unter `references/`. FreeMiNT `usb_keyboard.c`
`kbd_int` (`:324`) und Circle `usbkeyboard.cpp` `ReportHandler` (`:184-249`) pruefen
0x01 nicht und diffen jeden Bericht (= heutiges Verhalten). Nichts implementiert,
`hid-keyboard.md` unveraendert; braucht eine Quelle (z. B. HUT) oder eine
Anwenderentscheidung. Beleg `measurements/measurement-2026-10-24-usb02-10-11/RUNINFO.TXT`.

## 11. F3-Rueckfall auch nach Command-Timeout (2026-10-24, Tester)

`measurements/measurement-2026-10-24-usb01-f3-test/RUNINFO.TXT` Check 7:
`configure_endpoint()` liefert `false` fuer einen Fehler-Completion-Code
**und** fuer einen Command-Timeout; `configure_hid` wiederholt in beiden
Faellen mit Funktion 0. Spec `docs/api/usb/usb-snapshot.md` §10 („Fallback“):
nach einem Timeout **kein** Rueckfall, weil der Befehlsring dann nicht
antwortet. Wirkung: hoechstens ein zweiter Timeout. Code an die Spec
angleichen.

**ERLEDIGT 2026-10-24 (`8c112965`):** `configure_endpoint` liefert
`config_desc::CommandOutcome` (Success/Failed/Timeout); Rueckfall nur bei Failed und
mehr als einer Funktion (`config_desc::retry_with_function0`, Hosttest
`configure_endpoint_fallback_only_after_an_answer`, rot mit der alten Regel). Beleg
`measurements/measurement-2026-10-24-usb02-10-11/RUNINFO.TXT`.

**Tester 2026-10-24: PASS.** Code nach Spec §10 auf allen drei Pfaden (Success / Fehler-CC / Timeout);
Hosttest 1631/0/1 (Basis 1630 rekonstruiert, 5 Worktree-Fehlschlaege = fehlendes `references/`), Mutation mit der
alten Regel rot ("no retry after a command timeout"); 8 Builds rc 0, Warnungen und Clippy unveraendert
(93/64, 121/76); QEMU default und `--usb-extra` Basis = neu ([usb]/[pcie]-Zeilen, Maus, Tastatur, Key-Repeat).
Timeout-/Mehrfunktionspfad per Fehlerinjektion im Scratch-Worktree gefahren (Failed: Retry, Geraet laeuft;
Timeout: kein Retry, Geraet faellt aus). Offen nur: echter VL805-Timeout / echtes Verbundgeraet (Pi).
Beleg `measurements/measurement-2026-10-24-usb02-11-test/RUNINFO.TXT`.

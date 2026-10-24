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

### Quellen zum Host-Verhalten (Recherche 2026-10-24)

Frage: Bericht mit allen Array-Feldern = 0x01 (`00 00 01 01 01 01 01 01`) verwerfen
und Zustand behalten, oder wie normal auswerten; und das Modifier-Byte?
**Ergebnis vorab: Keine normative (USB-IF-)Quelle schreibt das Host-Verhalten vor.**
Mehrere Implementierungen verwerfen den Bericht; die Quellen sind sich beim
Modifier-Byte nicht einig. Trennung unten: Normativ vs. Implementierungspraxis.

#### A. Normativ (USB-IF)

| Quelle | Befund |
|---|---|
| HID 1.11, Anh. C S. 62/63, F.3 S. 74 (`references/usb/hid1_11.pdf`) | nur Geraetepflicht (bereits oben) — **schweigt** zum Host |
| **HID Usage Tables 1.5** (`references/usb/hut1_5.pdf`, `https://usb.org/sites/default/files/hut1_5.pdf`, abgerufen 2026-10-24, sha256 `f4ead217…c07db3`), Kap. 10 Keyboard/Keypad Page (0x07), gedr. S. 89 (PDF-S. 90), Tabelle: `01 Keyboard ErrorRollOver¹`, `02 Keyboard POSTFail¹`, `03 Keyboard ErrorUndefined¹`, Typ Sel, Boot-Spalte „4/101/104“. Fussnote 1, gedr. S. 95: „Reserved for typical keyboard status or keyboard errors. Sent as a member of the keyboard array. Not a physical key.“ | **sagt:** 0x01–0x03 sind keine physischen Tasten, sondern Status/Fehler als Array-Mitglied. **schweigt:** was der Host mit dem Bericht tut, und zum Modifier-Byte. (HUT 1.12 nicht erreichbar: `hut1_12v2.pdf` -> HTTP 404; 1.5 ist die abrufbare, nicht aeltere Fassung.) |

Folgerung (**Schluss, nicht gelesen**): HUT 1.5 belegt nur „0x01 ist kein Tastencode“.
Dass der Host den Bericht verwerfen *muss*, steht nirgends.

#### B. Implementierungspraxis (keine Autoritaet; Dateien am 2026-10-24 von `master`/`main`/`trunk` geholt; Pin = letzter Commit, der die Datei beruehrt)

| Quelle (Datei, Funktion, Pin) | Verhalten bei 0x01 | Modifier-Byte |
|---|---|---|
| **Linux** `drivers/hid/hid-core.c`, `hid_input_fetch_field` `:1631-1637`, `hid_input_array_field` `:1679-1681`, Pin `c5fd5e67c781` — `/* Ignore report if ErrorRollOver */ … field->ignored = true;` bei Array-Feld, dessen Usage `HID_UP_KEYBOARD + 1` ist; in `hid_input_array_field`: `if (field->ignored) return;` (alter Array-Zustand `field->value` bleibt) | **Array-Feld verwerfen, Zustand behalten** | **wird normal verarbeitet**: `ignored` gilt nur fuer das Array-Feld; das Modifier-Feld ist ein Variable-Feld und laeuft in `hid_input_var_field` (`:1649-1664`) wie immer (**gelesen**: `hid_process_report` ruft alle Felder; Variable-Zweig ohne `ignored`-Test) |
| **Linux** `drivers/hid/usbhid/usbkbd.c`, `usb_kbd_irq` `:101-146`, Pin `34b228b739f5` (alter Boot-only-Treiber) | prueft 0x01 nicht; Tasten mit Code `> 3` werden gediffed (`:123`, `:132`). Eine Fehlerrolle-Meldung loest also fuer gehaltene Tasten ein Release aus, der Folgebericht ein Press = **heutiges NewTOS-Verhalten** | Modifier-Bits werden immer gemeldet (`:118-119`) |
| **FreeBSD** `sys/dev/usb/input/ukbd.c`, `ukbd_intr_callback` `:863-866`, Pin `8809ea46f105` — `if (key == KEY_ERROR) { … sc->sc_ndata = sc->sc_odata; goto tr_setup; /* ignore */ }` | **ganzen Bericht verwerfen, Zustand behalten** | **mit verworfen**: Modifier sind im selben `sc_ndata`-Bitmap (Usage 0xE0–0xE7, Schleife nach dem Array) und ndata wird auf odata zurueckgesetzt, `ukbd_interrupt` wird uebersprungen |
| **FreeBSD** `sys/dev/hid/hkbd.c` `:786-792`, Pin `c0e26f7b29dd` — `memcpy(sc_ndata…, sc_odata…); return; /* ignore */` | wie ukbd.c | wie ukbd.c (ndata/ndata0 beide zurueckgesetzt) |
| **NetBSD** `sys/dev/hid/hidkbd.c`, `hidkbd_input` `:510-513`, Pin `1122aef3944e` — `if (ud->keycode[0] == KEY_ERROR) { … return; /* ignore */ }` | **verwerfen**, nur `keycode[0]` geprueft; `sc_odata = *ud` (`:552`) wird nicht erreicht = Zustand bleibt | **mit verworfen**: Rueckkehr steht vor der Modifier-Schleife (`:517`) |
| **U-Boot** `common/usb_kbd.c`, `usb_kbd_service_key` `:305-327` (`old[i] > 3`), `usb_kbd_irq_worker` `:330`, Pin `b075fd4e2d85` | prueft 0x01 nicht; Codes `<= 3` werden uebersprungen, uebrige gediffed (= wie usbkbd.c) | keine Sonderbehandlung |
| **TinyUSB** Beispiel `examples/host/cdc_msc_hid/src/hid_app.c`, `process_kbd_report` `:128-`, Pin `bda7efb1b3d4` | **schweigt** (nur Beispielcode, behandelt jeden Code != 0 als Taste) | — |
| FreeMiNT `usb_keyboard.c` `kbd_int`, Circle `usbkeyboard.cpp` `ReportHandler` | schweigen (siehe Stand oben) | — |
| OpenBSD `sys/dev/usb/ukbd.c` (Pin nicht erhoben) | Datei geholt, kein Treffer fuer ErrorRollOver/0x01; Treiber nutzt den generischen HID-Pfad — **nicht weiter verfolgt** | — |
| Zephyr, Haiku | **nicht geprueft** (nicht abgerufen) | — |

Lesart: Linux-Kern (`hid-core`), FreeBSD (ukbd, hkbd) und NetBSD (hidkbd) verwerfen
den Fehlerbericht und behalten den alten Zustand; die Boot-only-Treiber (Linux
`usbkbd`, U-Boot) und TinyUSB-Beispiel/FreeMiNT/Circle diffen ihn. Beim Modifier-Byte
gehen Linux (verarbeiten) und BSD (mit verwerfen) auseinander — **hier gibt es
keine normative Entscheidung.**

#### C. Entwurf fuer `docs/api/usb/hid-keyboard.md` (Recherche-Stand; freigegeben in geaenderter Form, siehe D)

> Stuetzt sich auf
> HUT 1.5 Kap. 10, Fussnote 1 (0x01–0x03 sind Statuscodes, keine Tasten) und auf
> Linux `hid-core.c` `hid_input_fetch_field`/`hid_input_array_field`, FreeBSD `ukbd.c`
> `ukbd_intr_callback`, NetBSD `hidkbd.c` `hidkbd_input`. Enthaelt ein Array-Feld des
> Boot-Berichts (Byte 2..7) den Code 0x01 (ErrorRollOver), wertet der Host die
> Tasten-Bytes dieses Berichts nicht aus und behaelt die vorige Tastenmenge; es
> entstehen weder Break noch Make. **Offene Entscheidung (keine Quelle einig):**
> Modifier-Byte (Byte 0) dieses Berichts entweder wie bei Linux weiter diffen
> oder wie bei FreeBSD/NetBSD zusammen mit dem Bericht verwerfen. Ob 0x02/0x03 gleich
> behandelt werden, ist nur bei FreeBSD/NetBSD/Linux fuer 0x01 belegt; fuer 0x02/0x03
> liegt keine Quelle vor, die sie wie 0x01 behandelt (Linux: nur `HID_UP_KEYBOARD + 1`).

Offen fuer den Anwender: Modifier-Byte-Variante waehlen; die Boot-Capture
(seq 116-118, 158-160, 164-166) zeigt `00` im Modifier-Byte, entscheidet also nichts.

#### D. Anwenderentscheidung 2026-10-24 (= Freigabe der Spec-Regel)

1. Enthaelt das Tasten-Array (Byte 2..7) 0x01 ErrorRollOver: Tasten-Bytes nicht
   auswerten, vorige Tastenmenge behalten, weder Break noch Make fuer Tasten.
2. Das Modifier-Byte dieses Berichts wird normal ausgewertet (wie Linux; HID 1.11
   Anh. C S. 63 „Modifiers still reported“).
3. Nur 0x01 loest das Verwerfen aus. 0x02 POSTFail und 0x03 ErrorUndefined erzeugen
   nie Make oder Break (HUT: „Not a physical key“); der Rest des Berichts wird normal
   ausgewertet.

Regel steht in `docs/api/usb/hid-keyboard.md`, Abschnitt „Status codes 0x01–0x03
in the key array“.

**ERLEDIGT 2026-10-24 (`795066d9`):** `hid_decode::keyboard_diff` nimmt bei 0x01 im
Array die alte Tastenmenge als neue (Modifier weiter gediffed); `is_key` (> 0x03)
schliesst 0x01-0x03 aus Make/Break aus. Zustand weiter pro Funktion (`hid::PREV`,
Test `error_rollover_is_per_function`). Host-Tests: Capture seq 116-118 (Break m
0x10, Make j 0x0d, `,` 0x36 unberuehrt), seq 158-160 und 164-166, Modifier-Wechsel
im 01-Bericht, 0x02/0x03 im Slot. Rot gegen alten Code: 4 der 5 neuen Tests; der
0x02/0x03-Test war schon vorher gruen (alter Code liefert fuer 1..3 keinen
Scancode, beobachtbar gleich). QEMU qemu-virt: sieben Tasten an EmuCON — Basis tippt
`asdfghasdfgh` (Fehlerbild reproduziert), neu `asdfgh`; Tastatur, Wiederholung, Maus
ohne Regression (beide Topologien). Hostsuite 1631 -> 1636, Warnungen unveraendert.
Nicht abgedeckt: echte Pi mit der Apple-Tastatur; Modifier-Wechsel im Phantombericht
nur im Hosttest; kein Geraet, das 0x02/0x03 sendet. Beleg
`measurements/measurement-2026-10-24-usb02-10/RUNINFO.TXT`.

**Tester 2026-10-24: PASS (`795066d9`, Basis `2019888e`).** Code gegen Spec-Regel 1-3
gelesen und mit Zusatz-Randfaellen im Scratch gefahren (0x01 als erster Bericht, 0x01 in
einem Slot neben echten Tasten, nur Byte 7, Byte 1 ausserhalb des Arrays, Leerbericht nach
Phantom): alle gruen. `cargo xtask test` 1631 -> 1636 (0 failed, 1 ignored, 158 Binaries,
Warnungszeilen 127 = 127). Mutation (alter `keyboard_diff`): 4 der 5 neuen Tests rot,
`post_fail_...` bleibt gruen (kein Unterscheidungswert, nur Dokumentation). Builds dev+release
beide Boards rc 0, rustc 93/64 und clippy 121/76 unveraendert. QEMU qemu-virt, beide
Topologien: Maus, Ctrl+Z, Tastenwiederholung ohne Regression (0 Pixel Differenz, [usb]-Zeilen
identisch, 0 Exceptions); sieben Tasten: Basis `asdfghasdfgh`, neu `asdfgh` (3 von 3 Laeufen,
auch mit `--usb-extra`). Offen: echte Pi mit Apple-Tastatur (HW). Beleg
`measurements/measurement-2026-10-24-usb02-10-test/RUNINFO.TXT`.

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

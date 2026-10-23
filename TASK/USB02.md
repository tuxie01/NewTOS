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

# USB-Maus: Protokolle, Defektstand, Hotplug, Diagnosewerkzeug (USB01 Schritt 1)

> Ergaenzung zu [`hid-mouse.md`](hid-mouse.md) (dort: nur Boot-Maus →
> IKBD-Relativpaket). Dieses Dokument ist **Analyse, kein Code**: es vergleicht
> unseren Treiber mit Circle und FreeMiNT, belegt den Stand der vier Defekte vom
> 2026-09-09, beschreibt das Hotplug-Verhalten und schlaegt den Weg fuer das
> Diagnosewerkzeug und die Reparaturen vor. Ticket: `TASK/USB01.md`.
> Spec-Gruppe: [`_state.md`](_state.md).

**Scope in einem Satz:** Wie praesentieren sich USB-Maeuse, wie erkennen und
dekodieren Circle und FreeMiNT sie, was macht `kernel/src/drivers/usb/` heute
damit, in welchem Zustand sind die Defekte 1–4, was passiert beim Umstecken,
und welcher Weg fuehrt zu einer Diagnosedatei pro Geraet.

## 0. Quellenlage und Kennzeichnung

* **[G]** = gelesen, mit `Datei:Zeile` belegt (diese Sitzung, Stand `main`
  `baf36a98`). **[S]** = Schluss aus dem Gelesenen, nicht selbst beobachtet.
  **[?]** = nicht entscheidbar aus den lokalen Quellen.
* **Lokal in `references/` liegt keine USB-2.0-, HID- oder xHCI-Spezifikation**
  (`references/datasheets/` enthaelt nur das BCM2711-Datenblatt). Darum:
  * HID-Aussagen stuetzen sich auf **HID 1.11** (usb.org, `hid1_11.pdf`), das in
    dieser Sitzung geholt und per `pdftotext` gelesen wurde (Kopie nur im
    Session-Scratchpad `…/scratchpad/usb01/hid1_11.pdf`, **nicht** im Repo).
    Zitiert wird nach Kapitel (§4.2, §4.3, §7.2.4, §7.2.6, §8.1, Anhang B.2).
  * Aussagen zu **USB 2.0 / xHCI** (Ringregeln, Verhalten bei Unterlauf/Overrun,
    Portstatus-Events) stehen auf **Circle- und FreeMiNT-Code** bzw. auf Schluss;
    sie sind unten als [S] oder [?] markiert. Empfehlung: die xHCI-1.2-Spec und
    USB 2.0 nach `references/` legen (gitignored), bevor Hotplug gebaut wird.
* **QEMU-Deskriptoren** (`usb-mouse`, `usb-tablet`) stammen aus QEMU
  `hw/usb/dev-hid.c` (extern per Web geholt, tertiaere Quelle, nicht lokal).
* Alle Zeilennummern unserer Dateien gelten fuer den heutigen Stand.

## 1. Wie sich Maeuse praesentieren

### 1.1 Grundregeln aus HID 1.11

* `bInterfaceSubClass` = 1 → „Boot Interface Subclass“; **nur dann** hat
  `bInterfaceProtocol` (1 Tastatur, 2 Maus) eine Bedeutung, sonst ist es 0
  (HID 1.11 §4.2, §4.3). [G]
* `SET_PROTOCOL` (wValue 0 = Boot, 1 = Report) „is supported by devices in the
  boot subclass“; nach dem Einschalten steht jedes Geraet im **Report**-Protokoll,
  der Host soll es trotzdem immer setzen (§7.2.6). [G]
* Boot-Maus-Bericht: Byte 0 Tasten 1–3 (Bits 0–2), Byte 1 X, Byte 2 Y, „3 to n
  device specific (optional)“ (Anhang B.2). [G]
* Report-Protokoll: das Format steht **nur** im Report-Deskriptor
  (`GET_DESCRIPTOR` Typ 0x22 an das Interface). Kommt in dem Deskriptor irgendwo
  ein Report-ID-Item vor, tragen **alle** Berichte ein fuehrendes ID-Byte
  (§6.2.2.7 / §8.1). Felder sind bitgepackt, nicht byteausgerichtet (§8.1). [G]
* Standard-Idle fuer Maeuse ist „infinity“ (§7.2.4), `SET_IDLE` ist fuer Maeuse
  also nicht noetig. [G]

### 1.2 Katalog

Legende Spalte „NewTOS heute“: **geht** / **stumm** / **falsch** / **[S]** =
nur aus dem Code gelesen, nicht an Hardware beobachtet.

| # | Praesentation | Erkennung (Circle) | Erkennung (FreeMiNT) | NewTOS heute |
|---|---|---|---|---|
| A | **Boot-Maus** (3,1,2), ein Interface, 3–4 Byte | `int3-1-2` → `CUSBMouseDevice` (`usbdevicefactory.cpp:103-106`); parst trotzdem den Report-Deskriptor (`usbmouse.cpp:66-113`) | `mouse-boot`: Protokoll 2, genau 1 Endpoint, Interrupt (`mouse-boot/usb_mouse.c:386-405`); `mouse`: dieselbe Pruefung plus Deskriptor-Parser (`mouse/usb_mouse.c:650-681`) | **geht** — auf echter Pi 4 belegt (Apple `05ac:0304`, Roccat `1e7d:2cee`: `STATUS.md:331`, Log `docs/analysis/usb-port-investigation/run3-fixed-both-in-usb3.log:87-101`). Rad-Byte 3 nur bei `len > 3` (`hid.rs:163-181`). **Mittel-/Seitentasten werden nicht uebersetzt** (`hid.rs:112` nutzt nur Bit 0/1) |
| B | **Boot-Maus, im Report-Protokoll mit 12/16-Bit-Deltas, Report-ID, >8 Byte, Hi-Res** | wie A, aber Circle schaltet die Maus bewusst auf **Report**-Protokoll (`usbhiddevice.cpp:107-121`: Protokoll 2 → `REPORT_PROTOCOL`) und liest Deltas bis 32 Bit, klemmt auf ±127 (`usbmouse.cpp:128-139`) | `mouse`: `SET_PROTOCOL(hid_mode)` — Report, wenn der Parser Tasten+X+Y fand, sonst Boot (`mouse/usb_mouse.c:705-733`); Delta wird auf s8 geklemmt (`:361-370`, `:498-499`) | **geht** (in 8 Bit): wir schicken immer `SET_PROTOCOL(boot)` (`xhci/mod.rs:1450`, `:1353-1355`), ein Boot-Subclass-Geraet muss dann das 3-Byte-Format liefern (HID 1.11 §7.2.6, B.2) [S]. Verloren gehen Hi-Res, Rad-Aufloesung, AC Pan |
| C | **Report-only-Maus** (Subclass 0, Protokoll 0): Format nur im Deskriptor, ggf. mit Report-ID | Circle routet `int3-0-0`/`int3-0-2`/`int3-1-0` **nicht** zur Maus, sondern zu `GetGenericHIDDevice` → Touchscreen bei Usage Page Digitizer, sonst **Gamepad** (`usbdevicefactory.cpp:108-121`, `:237-316`) — d. h. eine solche Maus ist in Circle selbst **nicht** unterstuetzt | `mouse`/`mouse-boot` lehnen sie ab: `bInterfaceProtocol != 2 → -1` (`mouse/usb_mouse.c:655-658`, `mouse-boot/usb_mouse.c:386-389`). Nur `tablet` nimmt jedes HID-Interface (`tablet/usb_tablet.c:860`) | **stumm** [S]: `read_config` liefert Protokoll 0 (`xhci/mod.rs:1303-1304`), `poll` verwirft Protokoll ≠ 1/2 (`mod.rs:145-152`). Zusaetzlich geht `SET_PROTOCOL(boot)` an ein Nicht-Boot-Interface (`xhci/mod.rs:1450`) — nach HID 1.11 §7.2.6 nicht vorgesehen; stalled es, wird das Geraet ganz verworfen (`:1450-1453`, `:1573-1575` setzt `dci` nicht) |
| D | **Verbundgeraet / Funkempfaenger**: mehrere HID-Interfaces (z. B. Tastatur + Maus + Hersteller) | jedes Interface wird eine eigene Funktion, bis `USBDEV_MAX_FUNCTIONS` = 10 (`usbdevice.cpp:393-481`, `usbdevice.h:31`) | `usb_new_device` ruft fuer **jedes** Interface `usb_find_interface_driver` (`usb.c:1031-1034`); Treiber nimmt, was `probe` annimmt | **nur ein Interface** [S]: `read_config` nimmt den **ersten** Interrupt-IN-Endpoint der ganzen Konfiguration und paart ihn mit dem *zuletzt gesehenen* Interface-Deskriptor (`xhci/mod.rs:1293-1325`); DevDma hat nur einen Interrupt-Ring und einen 8-Byte-Puffer je Geraet (`:117-121`). Steht die Tastatur zuerst, bleibt die Maus unkonfiguriert und umgekehrt. Beleg fuer Mehr-Interface-Geraete: Roccat hat (3,1,2)+(3,0,1) (`run3…log:92-97`); Apple-Tastatur (3,1,1)+(3,0,0) (`:144-149`) |
| E | **Kein HID, aber Interrupt-IN-Endpoint mit Protokoll 1/2** (z. B. Kommunikations-/Funk-Schnittstellen) | Name `int<klasse>-…` filtert nach Klasse, kein Treffer | `probe` prueft Klasse HID (bzw. Subclass Boot) | **falsch** [S]: `read_config` prueft weder `bInterfaceClass` noch `bInterfaceSubClass` (Kommentar `xhci/mod.rs:1257-1264`, Code `:1301-1304`), nimmt `bInterfaceProtocol` jeder Klasse; Protokoll 1 wuerde als Tastatur dekodiert (`mod.rs:146`). Welche reale Klasse das trifft, ist **nicht** beobachtet |
| F | **Absoluter Zeiger** (Tablet, KVM, QEMU `usb-tablet`) | Digitizer → Touchscreen; Generic-Desktop-Absolutzeiger → Gamepad-Pfad [S] (`usbdevicefactory.cpp:269-316`); `usbmouse.cpp` unterscheidet nicht relativ/absolut (`:354-356` prueft nur Input-Bits 0/1) | `tablet`: Deskriptor-Parser fuer X/Y/Druck/Tip, **Umrechnung in Relativpakete** (`tablet/usb_tablet.c:615-690`, Anfahren der Ecke 0/0 bei „in range“ `:663`, `send_mouse_packets` `:572-592`); echte Absolutpakete 0xF7 sind dort als „no reaction“ auskommentiert (`:746-790`) | **stumm** [S]: QEMU `usb-tablet` = HID, Subclass 0, Protokoll 0 (QEMU-Quelle, extern) → wie C. Dazu verwirft `ikbd.rs` das Absolutpaket 0xF7 (`ikbd.rs:61`, `:194`); es gibt keinen Weg, absolute Koordinaten an die VDI zu geben |

### 1.3 Anforderungen der Geraete an den Host (Anfragen)

| Anfrage | Circle | FreeMiNT | NewTOS heute |
|---|---|---|---|
| `SET_CONFIGURATION` | vor den Funktionen, `usbdevice.cpp:381` | `usb_new_device`, `usb.c:1004` | in `configure_hid` **nach** Configure Endpoint (`xhci/mod.rs:1442-1449`, Reihenfolge begruendet `:1438-1441`) |
| `GET_DESCRIPTOR` HID-Report (0x22) | ja, Laenge aus HID-Deskriptor `wReportDescriptorLength` (`usbmouse.cpp:66-90`) | ja: Laenge per Konfigurations-Walk (`hid-common/hid.c:56-132`), Abruf `:134-155` | **nie** — es gibt keinen Aufruf; `get_descriptor` kennt nur Geraete-/Konfigurationsabruf (`xhci/mod.rs:1125-1142`) |
| `SET_PROTOCOL` | nur wenn Klasse 3, Subclass 1, Protokoll > 0: Maus → Report (1), Tastatur → Boot (0) (`usbhiddevice.cpp:107-121`) | `mouse`: Report/Boot je nach Parser (`mouse/usb_mouse.c:733`); `mouse-boot`: immer Boot (`mouse-boot/usb_mouse.c:435`; der Report-Zweig ist auskommentiert `:427-434`) | **immer** Boot, unabhaengig von Subclass (`xhci/mod.rs:1450`) |
| `SET_IDLE` | nie (kein Treffer in `lib/usb`) | `mouse` `:731`, `mouse-boot` `:425`, `tablet` `:936`: Dauer 0, Report-ID 0 | nie |
| Interface-Auswahl | alle Interfaces der Konfiguration, Alternate-Settings > 0 ignoriert (`usbdevice.cpp:393-406`) | alle Interfaces, jeder Treiber `probe` (`usb.c:1062-1091`) | erstes Interrupt-IN der Konfiguration (s. D) |
| Mehrere gleiche Geraete | mehrere Funktionen moeglich | **nur eine Maus**: `if (mse_data.pusb_dev) return -1` (`mouse/usb_mouse.c:630-633`, `mouse-boot:361-364`) | bis `MAX_HID` = 8 Slots, Hubs eingerechnet (`xhci/mod.rs:96`) |

### 1.4 Dekodierung des Berichts

| Punkt | Circle | FreeMiNT `mouse` | NewTOS heute |
|---|---|---|---|
| Berichtslaenge | aus Deskriptor berechnet; Bericht mit anderer Laenge wird verworfen (`usbmouse.cpp:117-119`, `:385`) | liest hoechstens 8 Byte (`mouse/usb_mouse.c:420-424`); `actlen < 3` → verworfen (`:437-440`) | TRB-Laenge fest 8 (`xhci/mod.rs:626`), Puffer `intbuf: [u8; 8]` (`:121`); Laenge aus Restgroesse des Events (`:719`). **Berichte >8 Byte sind strukturell nicht vorgesehen**; wie der Controller sie meldet, ist **[?]** (keine xHCI-Spec lokal); ein Fehlercode fuehrt nur zum Neuarmieren (`:722-734`) |
| Report-ID | Item `0x84`; erste ID wird gemerkt, 8 Bit Offset addiert (`usbmouse.cpp:293-313`) | Bericht beginnt mit ID, wird uebersprungen, falsche ID → Wert 0 (`hid-common/hid.c:191-199`) | keine Behandlung |
| Feldbreiten | Bit-Extraktion bis 32 Bit (`usbmouse.cpp:170-226`) | `fill_extract_bits`/`hid_get_data` (`hid.c:160-218`, `:220-232`) | feste 8 Bit (`hid.rs:114-115`) |
| Klemmen | ±127 (`usbmouse.cpp:130-139`) | `clip_to_s8` (−128..127) (`mouse/usb_mouse.c:361-370`) | entfaellt (8 Bit) |
| Tasten | Bits 1–5 → links/rechts/mitte/Seite1/Seite2 (`usbmouse.cpp:143-163`) | Taste 3 → Ziffernblock-Enter, 4/5 → Shift+Pfeil (`mouse/usb_mouse.c:501-526`) | nur Bit 0/1 (`hid.rs:112`) |
| Rad | Wheel-Usage 0x38 (`usbmouse.cpp:342-345`) | Wheel 0x38 und AC Pan (Consumer 0x238) (`hid-common/hid.h:42-44`; `mouse/usb_mouse.c:321-351`), als Pfeiltasten (`:529-576`) | Byte 3 bei `len > 3` als Scancode 0x59/0x5a (`hid.rs:163-181`); **kein** horizontales Rad (`0x5c/0x5d` unbenutzt) |
| Parser-Umfang | nur Items Usage Page/Usage/Collection/Report ID/Size/Count/Input; Usage Min/Max, Push/Pop, Relativ-/Absolut-Bit werden nicht ausgewertet (`usbmouse.cpp:228-386`) | `find_usage` ueber Pfadknoten, Usage Min/Max in Tabelle, `report_count`/`size` als `bit8u` (`hidparse.c:93-332`, `:280-288`, `:355-366`) | kein Parser |

**Folgerung fuer einen eigenen Parser [S]:** beide Referenzen sind bewusst
schmal (Maus: Tasten, X, Y, Rad, ggf. AC Pan). Keine von beiden behandelt
absolute Zeiger als Maus; FreeMiNT behilft sich mit Relativ-Umrechnung.

### 1.5 Weitere Luecken im Enumerationspfad, die Maeuse betreffen

Alle [S] aus dem Code, keine an Hardware beobachtet:

1. **Konfigurationspuffer 256 Byte.** `read_config` kappt `wTotalLength` auf 255
   (`xhci/mod.rs:1244-1247`; Puffer `buf: [u8; 256]` `:120`). Eine groessere
   Konfiguration (viele Interfaces, Funkempfaenger) wird mitten im Deskriptor
   abgeschnitten, ohne Meldung. Ein HID-Report-Deskriptor kann ebenfalls groesser
   sein; es gibt keinen Pfad dafuer.
2. **Alternate Settings** werden nicht beachtet: `bAlternateSetting` wird nicht
   gelesen, der Endpoint wird dem zuletzt gesehenen Interface-Deskriptor
   zugeordnet (`:1301-1304`, `:1306-1325`). Circle ignoriert Alt > 0
   (`usbdevice.cpp:393-406`).
3. **Interrupt-Intervall** fuer SuperSpeed wird mit der FS/LS-Formel kodiert
   (`:1408-1415`, nur `speed == 3` ist High-Speed-Sonderfall). [?] ob das fuer
   SS-HID-Geraete falsch ist — xHCI-Spec nicht lokal.
4. **Mittel-/Seitentasten** gehen verloren (siehe Katalog A), obwohl die
   Empfaengerseite fertig ist (`hid-mouse.md`, „Middle button“).
5. **Befehlsring ohne Link-TRB — harte Grenze von 15 Befehlen** (siehe §3.1;
   betrifft jede Steckkombination, nicht nur Maeuse).

## 2. Stand der vier Defekte vom 2026-09-09

Die Reparaturen liegen **gemeinsam in `4cbd7485` (2026-09-13)**, einem Commit mit
dem Titel „fix(aes): repair the resource load path, …“, der laut Body auch „the
USB/xHCI port investigation, the usb-diag boot-log capture and the QED trace
instrumentation“ mitnimmt (`git show --stat 4cbd7485`: `kernel/src/bootlog.rs`,
`drivers/usb/mod.rs`, `drivers/usb/xhci/mod.rs` +625 Zeilen, `hal/uart.rs`).
Es gibt kein eigenes Ticket und keinen eigenen Eintrag in `STATUS.md`; die Phase-16-Zeile
(`STATUS.md:331`) und der Eintrag vom 2026-09-09 (`:738`) stehen unveraendert auf „offen“.

| # | Defekt (2026-09-09) | Stand heute | Beleg |
|---|---|---|---|
| 1 | `wait_transfer` nimmt das erste Transfer-Event ohne Slot/Endpoint-Pruefung; Interrupt-IN eines schon konfigurierten Geraets wird sofort scharf | **behoben** [G] | `wait_transfer` quittiert nur Events mit passendem Slot **und** Endpoint, fremde werden geparkt statt verworfen (`xhci/mod.rs:1022-1066`, Parken `:753-768`, Abholen `:772-781`); Endpoints werden erst nach der Enumeration des ganzen Baums scharf gemacht (`arm_hid_endpoints` `:1506-1512`, aufgerufen `:1489`) |
| 2 | `poll_interrupt` ordnet nur ueber die Slot-ID zu, ohne Completion-Code und Endpoint | **behoben** [G] | Besitzer = Slot **und** `dci` (`:694-697`), `dci == 0` (Hub, fehlgeschlagene Konfiguration) besitzt nie ein Event (`:672-679`); Completion-Code wird geprueft, `Success`/`ShortPacket` liefern den Bericht (`:707-721`), jeder andere Code wird protokolliert und das Endpoint **neu armiert** (`:722-734`); `queue_interrupt` ignoriert `dci == 0` (`:609-621`) |
| 3 | Alles ausser Protokoll 1 wurde Maus | **teilweise behoben** | `poll` dekodiert nur Protokoll 1 (Tastatur) und 2 (Maus), alles andere wird ignoriert (`mod.rs:145-152`); Hubs tragen Protokoll 9 (`xhci/mod.rs:1567-1568`). **Offen**: das Protokoll stammt weiter aus `bInterfaceProtocol` *jeder* Klasse und jeder Subclass (`xhci/mod.rs:1301-1304`), siehe Katalog E und HID 1.11 §4.3. Zudem wird ein Hub nur erkannt, wenn `bDeviceClass == 9` (`:1567`); ein Hub als Interface eines Verbundgeraets (Geraeteklasse 0) fiele in `configure_hid` [S] |
| 4 | Hub-Port-Reset Einzelschuss 60 ms statt FreeMiNT 5 × 150 ms | **behoben** [G] | `hub_port_reset`: bis zu 5 Versuche, je `SET_FEATURE(PORT_RESET)` + 150 ms + neu lesen, Abbruch bei Verbindungswechsel, `C_PORT_RESET` nur nach Erfolg (`xhci/mod.rs:1831-1904`), 1:1 zu FreeMiNT `hub.c:225-272`. **Nicht** betroffen: der Reset eines **Root**-Ports ist weiter ein Einzelschuss mit Spin-Schleife ohne Wiederholung (`port_reset` `:896-909`) — dafuer gibt es in FreeMiNT keine Entsprechung im Hub-Code, Circle wartet dort 100 ms (`xhcirootport.cpp:75`) |

**Was die Messung belegt und was nicht.** `docs/analysis/usb-port-investigation/`
enthaelt drei Boot-Logs derselben Kette (Root-Port 1 → VIA-Hub `2109:3431` →
Roccat-Maus / Apple-Hub `05ac:1003` → Apple-Tastatur `05ac:020c`). In `run3`
(Build nach der Reparatur) enumerieren **alle** Geraete (`run3…log:61-157`), jeder
Hub-Port war nach dem ersten 150-ms-Reset aktiv (`:80-82`, `:107-109`, `:132-134`).
Die Logs enden mit der Enumeration; **ob danach Maus und Tastatur Eingaben liefern, zeigt
keiner der Laeufe.** Ob der Steckkombinations-Fehler mit `4cbd7485` verschwunden ist,
ist daher **[?]** — dafuer braucht es einen Hardwarelauf in beiden Kombinationen
(der Anwender hat Hardware, ich nicht). Dass (1) die Ursache war, ist ein
konsistenter Schluss [S]: in `run2` (vor der Reparatur) wird die Maus *vor* dem
Tastatur-Hub konfiguriert (`run2…log:100`, Tastatur erst `:151`), und nach
`defer_transfer_event`s Kommentar verlor genau diese Reihenfolge das Event der
Maus (`xhci/mod.rs:1014-1021` verweist auf `run2`).

## 3. Weitere Abhaengigkeiten vom Steckplatz / von der Kombination

### 3.1 Befehlsring: nach 15 Befehlen ist Schluss [S, hoch]

`command()` schreibt in einen 16-Eintraege-Ring und springt nach Index 14 auf 0
zurueck, **ohne** Link-TRB und **ohne** das Cycle-Bit umzuschalten
(`xhci/mod.rs:54`, `:810-814`; der Kommentar gibt es zu: „the ring does not wrap
— a Link TRB / wrap is a later refinement“ `:799-801`). Eintrag 15 bleibt null.
Ein xHCI-Controller wandert am Ende des Segments zu Eintrag 15, sieht dort ein
ungueltiges (Cycle 0) TRB und wartet; der 16. Befehl liegt auf Index 0 und wird
nie geholt → `cmd timeout` nach 200 Mio. Spin-Schleifen (`:820-851`). Circle legt
dagegen einen Link-TRB mit Toggle-Cycle an (`xhciring.cpp:50-55`, `:142-148`).
Die Ringregel selbst ist nicht aus lokaler xHCI-Spec belegt [?], folgt aber
zwingend aus dem Code, der sie umgeht.

Befehlsverbrauch des `run3`-Baums (aus den Logs gezaehlt, [S]): VIA-Hub 3
(Enable Slot, Address Device, Evaluate Context fuer Hub), Roccat-Maus 4
(+ Evaluate fuer EP0 64, `:1162`), Apple-Hub 3, Apple-Tastatur 3 → **13 von 15**.
Ein weiteres Geraet mit 64-Byte-EP0 (4 Befehle) oder zwei weitere
(je 3) laeuft ueber. **Das macht das Ergebnis von der Anzahl und Art der
angeschlossenen Geraete abhaengig**, nicht von der Portnummer. Hotplug
(Re-Enumeration nach dem Boot, braucht 3–4 Befehle) hat nur 2 Befehle Luft.

### 3.2 Slot-Budget `MAX_HID` = 8

Jedes Geraet — Hubs und Nicht-HID eingeschlossen — belegt einen `HidDev`-Eintrag
(`xhci/mod.rs:96`, `:1538-1546`); `n_hid` wird nie zurueckgesetzt. Ist das Budget
erschoepft, brechen die Schleifen **ohne Meldung** ab (`:1469-1471`, `:1529-1531`,
`:1610-1612`). Geraete an hoeheren Hub-Ports bzw. spaeter eingesteckte fallen so
stumm heraus. Der Pi-Baum aus `run3` braucht 4.

### 3.3 Hub-Beschreibung und Geschwindigkeit

* `hub_get_descriptor` fragt immer Typ `0x29` (`:1770`); akzeptiert aber auch
  `0x2A` in der Antwort (`:1777`). Ein SuperSpeed-Hub (Root-Ports 2–5 an der Pi)
  erwartet [?] `0x2A`; **weder Circle** (`DESCRIPTOR_HUB = 0x29`, `usbhub.h:38`)
  **noch FreeMiNT** (kein xHCI) sind dafuer Referenz.
* Die Geschwindigkeit eines Hub-Kindes kommt aus den Portstatus-Bits 9/10
  (`:1639-1645`), SuperSpeed hinter einem Hub ist nicht abgedeckt.
* Root-Ports werden nur beim Boot und nur bei `current_connect_status`
  beruecksichtigt (`:1477-1479`).

## 4. Hotplug

### 4.1 Heute

* **Enumeration passiert genau einmal**, in `usb::init` (`mod.rs:46-71`,
  aufgerufen `lib.rs:430`). Es gibt keinen weiteren Aufrufer von
  `enumerate_all` (Suche im Baum).
* **Ohne Geraet beim Boot kein Controller-Objekt:** `XHCI` wird nur gesetzt,
  wenn `enumerate_all() > 0` (`mod.rs:64-67`); sonst kehrt `poll` sofort zurueck
  (`mod.rs:135-137`). Ein erst spaeter eingestecktes Geraet wird nie gesehen.
* **Port-Status-Change-Events werden verworfen:** `poll_interrupt` (`:688-690`),
  `command` (`:826-830`) und `wait_transfer` (`:1037`) konsumieren sie ohne
  Auswertung. `PORTSC` wird nach dem Boot nie gelesen oder quittiert;
  `scan_ports` ist reine Ausgabe (`:1930-1956`). Ob stehengebliebene Change-Bits
  weitere Events unterdruecken, ist **[?]** (xHCI-Spec nicht lokal).
* **Hubs bekommen kein Status-Change-Interrupt-Endpoint**: `enumerate_hub`
  macht nur `SET_CONFIGURATION` (`:1599`), `dci` bleibt 0 (`:1567-1572`), also
  wird auch nichts armiert (`:1506-1511`).
* **Kein Abbau:** es gibt weder Disable Slot noch Reset/Stop Endpoint noch eine
  Disconnect-Behandlung (Suche in `xhci/mod.rs`/`mod.rs`: kein Treffer);
  `UsbError::NoDevice` existiert nur als Typ (`mod.rs:170-171`). Zieht man ein
  enumeriertes Geraet ab, bleibt sein `HidDev`/Slot stehen. Was der Controller
  dann am Endpoint meldet, ist **[?]**; der Code reagiert auf jeden
  Nicht-Erfolgscode mit Protokollzeile + Neuarmieren (`:722-734`) — das kann
  pro Poll eine Zeile Log erzeugen [S].
* **Folge: Umstecken = Neustart.** Wieder eingesteckt wird ein Geraet nur, wenn
  es zum naechsten Boot angeschlossen ist.

### 4.2 Referenzen

| | Circle | FreeMiNT |
|---|---|---|
| Root-Port | Port-Status-Change-Event → `PortStatusChanged` → `HandlePortStatusChange`: verbunden und kein Geraet → `ReScanDevices`; getrennt → `RemoveDevice` (`xhcirootport.cpp:202-243`) | Root-Hub ereignisgetrieben: `usb_hub_thread` ruft `usb_hub_events` bei Wakeup (`hub.c:767-778`) |
| Hub | Status-Change-Interrupt-Endpoint des Hubs, nur bei Plug-and-Play (`usbstandardhub.cpp:188-189`, `:480-514`); `HandlePortStatusChange` liest den Portstatus, quittiert `C_PORT_CONNECTION` und ruft `ReScanDevices`/`RemoveDeviceAt` (`:528-628`) | **Polling** jede Sekunde (`usb_hub_poll_thread`, `addtimeout(1000L…)`, `hub.c:606-621`); `usb_hub_events` liest jeden Port, bei `C_CONNECTION` → `usb_hub_port_connect_change`: Kind trennen (`usb_disconnect` ruft `disconnect` der Treiber), neu zuruecksetzen, `usb_new_device` (`hub.c:275-330`, `:472-560`, `usb.c:768-803`) |
| Voraussetzung | `bPlugAndPlay` im Konstruktor + Aufruf von `UpdatePlugAndPlay` in der Hauptschleife, nicht aus IRQ (`doc/usb-plug-and-play.txt`, `usbhostcontroller.cpp:148-188`) | TOS-only-Variante: **kein** Hotplug, nur ein Durchlauf beim Start (`usb_hub_init`, `hub.c:784-832`) |
| Treiber-Disconnect | Geraeteobjekte werden zerstoert | `mouse_disconnect` setzt `pusb_dev = NULL` (`mouse/usb_mouse.c:183-195`) |

**Einschraenkung fuer NewTOS [S, belegt durch den Code]:** `usb::poll()` laeuft
auch aus dem 200-Hz-Timer-Interrupt (`host_env.rs:476-485`, `:505-`, INPUT01).
Eine Hotplug-Enumeration mit `delay_ms` (50 ms nach Address Device `:984`, 150 ms
je Reset `:1843`, 100 ms Hub-Power `:1754`) darf dort **nicht** laufen; sie
gehoert in den trapseitigen Pump bzw. in einen eigenen Aufruf ausserhalb des
IRQ-Pfads.

## 5. Diagnosewerkzeug (Ticket Schritt 2) — Vorschlag

**Ziel des Anwenders:** verschiedene Maeuse einstecken, je Geraet eine Datei auf
`C:` mit Geraete-/Konfigurations-/Interface-/Endpoint-/HID-Report-Deskriptor (roh,
hex), Hub-/Port-Weg, Geschwindigkeit, gewaehltem Protokoll und einigen rohen
Berichten bei Bewegung/Klick.

### 5.1 Was beide Wege brauchen (gemeinsamer Kern, heute nicht vorhanden)

1. **Deskriptoren aufbewahren.** Heute werden sie nur gedruckt (`usb-diag`,
   `xhci/mod.rs:1265-1292`) und der Puffer wird bei jedem Transfer ueberschrieben
   (`:1128`); behalten werden nur Klasse, VID, PID (`:1177-1179`).
2. **Report-Deskriptor holen** (Klassenanfrage `GET_DESCRIPTOR` 0x22 an das
   Interface, Laenge aus dem HID-Deskriptor): existiert nicht (§1.3). Dafuer
   groesserer Puffer als 256 Byte.
3. **Rohe Berichte aufzeichnen:** `poll_interrupt` liefert sie (`:680-721`), der
   Aufrufer wirft sie nach der Dekodierung weg (`mod.rs:138-155`). Noetig: ein
   Ring je Geraet (Zeitpunkt, Laenge, Bytes).
4. **Pfad/Geschwindigkeit/Protokoll** liegen in `HidDev` bereits vor (`port`,
   `route`, `speed`, `protocol`, `dci`: `:165-189`).
5. **Holen waehrend der Enumeration** (kein neuer Transfer spaeter): dann laeuft
   alles im ruhigen Boot-Pfad und es entsteht keine Wiedereintritts-Frage mit
   dem Timer-IRQ-Pump (`host_env.rs:476-485`).

### 5.2 (a) EL0-Programm ueber eine neue Kernel-Schnittstelle

* **Existiert in FreeMiNT eine USB-Auskunft zum Uebernehmen?** Nein als
  Systemaufruf: `Ssystem` kennt keinen USB-Modus (`sys/ssystem.c`,
  `sys/mint/ssystem.h`: kein Treffer fuer „usb“). FreeMiNTs Weg ist der
  `_USB`-Cookie, der auf `struct usb_module_api` zeigt — eine Tabelle von
  **Funktionszeigern im selben Adressraum** (`usb_api.h:127-186`, gesetzt
  `init.c:182-199`); das Tool `usbtool` liest damit `usb_device`-Strukturen und
  pollt `usb_hub_events` jede Sekunde (`tools/usbtool/usbtool.c:150-170`,
  `:249-262`). Das laesst sich auf einem EL0-Programm ohne gemeinsamen
  Adressraum nicht uebernehmen; uebernehmbar ist das **Datenmodell**
  (Geraeteliste, Deskriptoren, Hub-Kind-Baum).
* **Passender Weg im Projekt:** NewTOS-eigene `Ssystem`-Modi ab 2000, bereits
  etabliert (SYSINFO01: 2003–2020 Skalare, **2019 als String in den Puffer des
  Aufrufers**, `tos-api/src/gemdos/ssystem.rs:1-30`, `:182-209`; Spec
  `docs/api/gemdos/154-ssystem.md`; Nummernraum 2000–2999 frei, `:17-23`;
  Kernel-Anschluss ueber das `Host`-Trait `tos-api/src/host.rs:330-337`,
  `kernel/src/host_env.rs:406-413`). Ein neuer Modus, z. B. „USB-Schnappschuss
  in Puffer (Geraeteindex, Puffer, Laenge)“, haengt sich genau so ein.
* **Programm:** Konsolenprogramm in `sdk/apps/`, ruft den Modus je Geraet,
  schreibt per `Fopen/Fwrite` `C:\USB\…`, bittet zum Bewegen/Klicken, liest die
  Berichtsringe erneut.
* **Kosten (Schaetzung, nicht gemessen):** gemeinsamer Kern (§5.1) + Host-Trait-
  Methode + Ssystem-Modus + Spec-Zeile + EL0-Programm + Test: **M–L**
  (ca. 2–4 Tage).
* **Was es freischaltet:** Programm laeuft *im laufenden System* beliebig oft;
  Dateien je Geraet (VID/PID im Namen) ohne Neustart — sobald Hotplug existiert,
  auch Mausumstecken ohne Reboot; Berichte *nach* dem Boot (Anwender bewegt die
  Maus, wenn das Programm sagt „jetzt“); spaeter nutzbar als Systeminfo-Werkzeug
  (SYSINFO01-Linie). **Wegwerf:** nur das Programm-Textformat; der Kern bleibt.
* **Fit:** passt zum Projekt (Ssystem-Erweiterung, FreeMiNT-Richtung, Regel 5:
  Spec vor Code); belastet die Hardware-Pi nicht mit Sonderbuild.

### 5.3 (b) `usb-diag`/`bootlog` ausbauen

* **Heute:** Feature `usb-diag` schneidet `kprint!` in 192 KiB `.bss` mit und
  schreibt es per `svc #1` (`Fcreate/Fwrite/Fclose`) nach `C:\USBDIAG.LOG`
  (`bootlog.rs:27-114`, Tee in `hal/uart.rs`), nur **waehrend des Boots**
  (zwei Aufrufe: `lib.rs:438` und nach `initinfo`). Es enthaelt schon alle
  Interface-/Endpoint-Zeilen (`xhci/mod.rs:1265-1292`), aber **weder** rohe
  Deskriptoren (nur Auszuege), **noch** den Report-Deskriptor, **noch** Berichte.
* **Ausbau:** Kern (§5.1) + Hex-Formatierung in der Kernel-Ausgabe + ein
  „Mess-Fenster“ im Boot (Kernel pollt N Sekunden und zeichnet Berichte auf, dann
  `flush`). Der `flush` darf **nur aus normalem Kernelkontext** laufen, nicht aus
  einem Exception-Handler (`bootlog.rs:75-77`) — das schliesst Flush aus dem
  Timer-IRQ-Pump aus; „Berichte nach dem Boot“ geht also nur ueber das
  Boot-Fenster.
* **Kosten (Schaetzung):** Kern + Formatierung + Fenster: **S–M** (ca. 1–2 Tage).
* **Was es freischaltet:** schnellster Weg zu *Daten* (Descriptors + Report-
  Deskriptor je Mausmodell). Datei heisst fest `USBDIAG.LOG`, ein Boot = ein Lauf
  = ein (ueberschriebenes) Log; **je Maus ein Neustart und Umbenennen**; kein
  Hotplug, keine Berichte nach dem Boot-Fenster. Sonderbuild noetig
  (`cargo xtask build --board rpi4 --release --features usb-diag`,
  `kernel/Cargo.toml:96-97`) und Aufspielen auf die Karte.
* **Wegwerf:** das Boot-Fenster und die Formatierung (fuer ein Dauerwerkzeug
  unbrauchbar); der Kern bleibt.

### 5.4 Empfehlung

**(a).** Der aufwendige Teil (§5.1) ist fuer beide gleich; (a) kostet dazu nur
die Schnittstelle und ein kleines Programm, bleibt als Werkzeug stehen und
erfuellt den Anwenderwunsch wortlich (Programm, Datei je Geraet, Berichte bei
Bewegung/Klick). (b) ist nur dann zu bevorzugen, wenn die **allerersten Daten
sofort** gebraucht werden und ein Reboot je Maus akzeptiert wird; dann sollte
man den Kern so bauen, dass (a) ihn unveraendert weiterverwendet.
**Abhaengigkeit:** „Maus umstecken ohne Reboot“ gelingt in *keinem* der beiden
Wege vor §4 / Fix F6; bis dahin heisst der Ablauf: Maus einstecken → neu booten →
Programm laufen lassen.
Testbar ohne Pi: QEMU `qemu-xhci` mit `usb-mouse` laeuft schon
(`tools/xtask/src/main.rs:664-667`); `usb-tablet` und `device_add`/`device_del`
per QMP erlauben Absolutzeiger und Hotplug ohne Hardware [S, nicht probiert].

## 6. Rangliste der Reparaturkandidaten (nichts davon gebaut)

Aufwand ist **meine Schaetzung** (S ≤ 0,5 Tag, M 1–2, L 3+), nicht gemessen.

| Rang | Kandidat | Warum | Aufwand | Abhaengig von |
|---|---|---|---|---|
| 1 | **F0 Diagnose-Kern** (§5.1): Deskriptoren behalten, Report-Deskriptor holen, Berichtsring | ohne Daten ist die Wahl des Parsers geraten; die Anwenderfrage „welche Maus spricht was“ ist nur so zu beantworten | M | — |
| 2 | **F1 Befehlsring mit Link-TRB** (§3.1) | stille, kombinationsabhaengige Grenze (13 von 15 im Pi-Baum), Voraussetzung fuer Hotplug und fuer mehr Geraete | S | — |
| 3 | **F2 Klassenpruefung** `bInterfaceClass == 3` (und Hub per Interface-Klasse) vor Protokoll 1/2 (Katalog E, Defekt 3 Rest) | verhindert Fehlzuordnung fremder Geraete | S | — |
| 4 | **F3 Mehrere Interfaces je Geraet** (Interface-Schleife, je HID-Interface eigener Endpoint/Ring/Puffer, Alt-Settings ignorieren) — **Spec: §9 (freigegeben 2026-10-24)** | Funkempfaenger/Verbundgeraete: sonst haengt es an der Reihenfolge im Deskriptor. Seit Ragnars Lauf (`measurements/measurement-2026-10-23-usb01-ragnar/RUNINFO.TXT`) belegt: alle drei Funkempfaenger bieten die Boot-Maus auf Interface 1 | M–L | F0 hilfreich, F1 |
| 5 | **F4 Report-Protokoll-Parser fuer Maeuse** (Tasten, X, Y, Rad, AC Pan, Report-ID, Bitbreiten; kein `SET_PROTOCOL(boot)` an Nicht-Boot-Interfaces; Zerlegung grosser Deltas in mehrere ±127-Pakete wie FreeMiNT `send_mouse_packets`; Berichte >8 Byte, Puffer >256) | schliesst Katalog C und die Hi-Res-Verluste aus B | L | F0 (Daten), F3 |
| 6 | **F5 Tasten 3–5 und horizontales Rad** (Bit 2..4 → IKBD-Zusatzscancodes `0x37/0x5e/0x5f`, AC Pan → `0x5c/0x5d`) | Empfaengerseite ist fertig (`hid-mouse.md`); fuer A schon mit Boot-Bericht fuer Taste 3 sofort machbar | S (Taste 3), mit F4 den Rest | — / F4 |
| 7 | **F6 Hotplug** (Root-Port-PSC-Events, Hub-Status-Change-Endpoint oder FreeMiNT-artiges Polling alle 1 s, Abbau: Disable Slot / Slot-Freigabe, Neuenumeration nur ausserhalb des Timer-IRQ) | „Umstecken ohne Reboot“; heute gar nicht vorhanden | L | F1; xHCI-Spec lokal ablegen |
| 8 | **F7 Absolute Zeiger** (QEMU-Tablet, KVM): Entscheidung Relativ-Umrechnung wie FreeMiNT `tablet` oder echter Absolutweg zur VDI | `ikbd.rs` verwirft 0xF7 (`:61`); braucht eine Produktentscheidung | M | F4, Anwenderentscheid |
| 9 | **F8 Slot-Budget/Stille-Abbrueche** melden, Budget pruefen (`MAX_HID`) | Diagnose „Geraet fehlt“ ohne Logzeile | S | — |
| 10 | **F9 Root-Port-Reset mit Wiederholung, SuperSpeed-Hub/Intervall** | nur falls Messung es zeigt; keine Referenz lokal | M | Hardware-Daten |

Die Defekte 1, 2, 4 brauchen **keine** weitere Reparatur (§2); offen ist ihre
**Hardware-Abnahme** in beiden Steckkombinationen.

## 7. Offene Fragen an den Anwender

1. **Diagnoseweg:** (a) Ssystem-Modus + EL0-Programm (empfohlen) oder (b)
   `usb-diag` ausbauen? Reicht „je Maus ein Neustart“ fuer den ersten Datensatz?
2. **Hardware-Abnahme der Reparatur `4cbd7485`:** Maus und Tastatur in beiden
   Kombinationen (Maus USB2/Tastatur USB3 und umgekehrt) — liefert *Eingabe*,
   nicht nur Enumeration? Mit `usb-diag`-Build waere das Log ein Beleg.
3. **Hotplug:** muss Umstecken ohne Reboot jetzt gehen (F6, L), oder genuegt
   Neustart je Maus?
4. **Absolute Zeiger:** Relativ-Umrechnung (FreeMiNT-Weg) oder echter Absolutweg?
   Gibt es einen konkreten Anwendungsfall (KVM, Tablet)?
5. **Spezifikationen:** darf ich USB 2.0, HID 1.11 und xHCI 1.2 nach `references/`
   legen (gitignored)? Aussagen zu Ringregeln und Hotplug stuetzen sich sonst
   nur auf Code.
6. **Welche Maeuse haben tatsaechlich versagt?** Ein konkretes Modell mit
   Deskriptor waere der staerkste Beleg fuer Katalog C/D.

## 8. Ausserhalb des Scopes beobachtet

* `docs/STATUS.md:331` und `:738` fuehren die Defekte 1, 2, 4 weiter als offen,
  obwohl der Code (`4cbd7485`) sie behebt; es fehlt ein Statuseintrag.
* `4cbd7485` bundelt die USB-Reparaturen unter einem AES-Titel; die Historie ist
  mit `git log` auf USB allein schwer auffindbar.
* `docs/api/usb/_state.md` fuehrt den HID-Parser als „deferred“; dieses Dokument
  macht ihn zur Voraussetzung (F4).
* `hid-mouse.md` nennt „request the boot protocol so no HID report-descriptor
  parsing is needed“ — das gilt nur fuer Boot-Subclass-Geraete (HID 1.11 §7.2.6).
* Blockierende Timeouts bei stummen Geraeten: 200 Mio. bzw. 50 Mio. Spin-
  Iterationen je Befehl/Transfer (`xhci/mod.rs:820`, `:1036`) — siehe
  `TASK/PORT06-KEYS-DIAGNOSIS.MD:165`.

## 9. F3 — Mehrere Interfaces je Geraet

> **STATUS: APPROVED 2026-10-24 (Anwender), Entscheidungen in §9.11; gebaut 2026-10-24** (§9.12;
> Snapshot-Layout v2: `usb-snapshot.md` §10). Pi-Abnahme (P1–P5) offen. Entwurf 2026-10-23. Stand der Quellen:
> `main` bei `3d6023b4`. Auftrag: `TASK/USB01.md` („Ragnars Lauf: Ursache der Funkmaeuse ist F3“).
> Dieser Abschnitt ist Spec und Analyse, kein Code. Ersetzt die Zeile F3 in §6 und
> ueberholt Katalog D (§1.2), soweit dort der Stand von vor F2 beschrieben ist.

**Scope in einem Satz:** Wie wird aus *einem* USB-Geraet mit mehreren HID-Interfaces
(Funkempfaenger: Tastatur + Maus + Hersteller) ein Satz von *mehreren* gepollten
Eingabe-Endpoints, welche Interfaces werden genommen, was ist je Endpoint an
Ring/Puffer/Zustand noetig, wo liegen die Grenzen, was aendert sich am Snapshot, und
wie verhalten sich *mehrere Tastaturen gleichzeitig* (der von Ragnar gesehene, nicht
erklaerte Ausfall der Kabeltastatur).

### 9.0 Kennzeichnung der Aussagen

* **[G]** gelesen, mit `Datei:Zeile` (Stand `3d6023b4`). **[S]** Schluss aus dem Gelesenen, nicht
  beobachtet. **[H]** Hypothese, nicht belegt. **[?]** aus den lokalen Quellen nicht entscheidbar.
  **[E]** Entwurfsentscheidung dieses Dokuments (Vorschlag, der Anwender darf sie kippen).
* **Lokale Quellen:** `references/usb/hid1_11.pdf` (HID 1.11; zitiert als `hid1_11.pdf §x`, dazu die
  Zeile im `pdftotext -layout`-Auszug, der nur im Scratchpad liegt), `references/xhci/xhci-1.2.txt`
  (zitiert `xhci-1.2.txt:Zeile`), `references/freemint/sys/usb/src.km/`, `references/circle/`.
  **USB 2.0 liegt nicht in `references/`** (`ls references/usb` = nur `hid1_11.pdf`); Aussagen, die
  das USB-2.0-Kernprotokoll bräuchten (z. B. „Endpoint-Adressen sind je Konfiguration eindeutig“),
  sind [S]/[?].
* **Vertrag vs. intern** (`docs/COMPATIBILITY.md`): Vertrag ist nur, was ein TOS-Programm sieht —
  Scancodes und Mauspakete aus dem IKBD-Strom. Alles in §9.2–§9.5 (Funktionstabelle, Ringe, Limits) ist
  **intern** und frei. Vertrag im weiteren Sinn ist die `Ssystem(2022)`-Schnittstelle (§9.7), weil
  `USBSNAP.NCP` sie liest.

### 9.1 Evidenz und Ausgangslage

**Die Geraete (alle [G], `measurements/measurement-2026-10-23-usb01-ragnar/`)**

| Geraet (VID:PID) | Interfaces der Konfiguration (alt 0) | Endpoints (Adresse, max, bInterval) | Log |
|---|---|---|---|
| Logitech Unifying `046d:c52b` (MX Master 2S), Full-Speed, 84 Byte | 0 = (3,1,1) Tastatur · 1 = (3,1,2) **Maus** · 2 = (3,0,0) report-only | 0x81 8 / 8 · 0x82 8 / 2 · 0x83 32 / 2 | `serial/001_…LOG:109-121` |
| Rii X8 `1997:2433`, Full-Speed (bcdUSB 1.10), 84 Byte | 0 = (3,1,1) Tastatur · 1 = (3,1,2) **Maus** · 2 = (3,1,1) **zweite Tastatur** | 0x81 64 / 10 · 0x82 64 / **1** · 0x83 64 / 10 | `serial/005_…LOG:109-121` |
| Cherry DW 9000 `046a:0115`, Full-Speed, 84 Byte | 0 = (3,1,1) Tastatur · 1 = (3,1,2) **Maus** · 2 = (3,0,0) report-only | 0x81 10 / 10 · 0x82 10 / 2 · 0x83 10 / 10 | `serial/007_…LOG:109-121` |
| Cherry KC 6000 `046a:0113` (Kabeltastatur, in **jedem** Start), Low-Speed, 59 Byte | 0 = (3,1,1) Tastatur · 1 = (3,0,0) report-only | 0x81 8 / 10 · 0x82 5 / 10 | z. B. `serial/001_…LOG:141-149` |

Alle vier haengen hinter dem VL805-internen Hub `2109:3431` (Root-Port 1, High-Speed), der Empfaenger in den
„schwarz“-Starts an Hub-Port 3 (Route `0x3`), in den „blau“-Starts an Hub-Port 1 (Route `0x1`,
`002_…LOG:98`, `004:98`, `006:98`), die Kabeltastatur immer an Port 4 (Route `0x4`). Das Ergebnis ist von
der Buchse unabhaengig (Tester-Tabelle), also kein Hub-Port-Effekt. Die Konfigurationslaenge folgt
aus dem Aufbau: 9 + 3 × (9 Interface + 9 HID + 7 Endpoint) = 84 [S, passt zu allen drei Logs].

**Was der Treiber heute daraus macht [G]:** `config_desc::scan` nimmt das *erste* Boot-Interface mit
Interrupt-IN (`kernel/src/drivers/usb/config_desc.rs:290-293`: `None if out.hid.is_some() =>
Some(SkipReason::OneInterfacePerDevice)`), alle weiteren Boot-Interfaces fliegen mit der Logzeile
„second HID boot interface: one interface per device“ heraus (`config_desc.rs:396`; im Log
`serial/001_…LOG:120`, `005_…LOG:120-121`, `007_…LOG:120`). Bei allen drei Empfaengern ist das
Interface 0 eine **Tastatur**; die Maus (Interface 1) wird nie konfiguriert, nie gepollt. Die
Zeile „HID configured: slot 2 … (keyboard)“ steht in allen drei Logs (`:125`), eine Zeile „(mouse)“
fehlt. Folge: Zeiger und Tasten des Funkmaus-Sets sind **in Code und Log erklaert** ([G]; nicht
nur ein Verdacht).

**Gegenprobe (Log-Diff [G]):** `003_logi_b100_schwarz.LOG` (laeuft) und `001_…LOG` (laeuft nicht)
unterscheiden sich ab Zeile 102 nur im Geraet an Slot 2 (B100: Interface (3,1,2), Low-Speed, 34 Byte; Empfaenger:
siehe Tabelle). Slot 3 (Kabeltastatur, Route `0x4`) ist in beiden Logs Zeile fuer Zeile gleich konfiguriert
(`003:126-145`, `001:134-153`). Alle EP0-Transfers Success, 9 bzw. 10 Befehle (der zehnte ist **vermutlich** das
Evaluate Context fuer EP0 des Full-Speed-Empfaengers [S, aus der Differenz zu 9 Befehlen im B100-Lauf]), kein Limit, kein „stray transfer event“ im
Aufzeichnungsfenster (die Captures enden bei „after initinfo“: **kein Eingabeverkehr im Log**).

### 9.2 Begriffe und Sollverhalten

* **Geraet** = ein xHCI-Slot = ein Eintrag `hid[di]` / `DEV[di]` (`xhci/mod.rs:229-307`, `:107-126`).
  Unveraendert; `MAX_HID = 8` (`:98`) bleibt das Geraetebudget (Hubs und Nicht-HID eingerechnet, USB02 #5).
* **Funktion** (neu, [E]) = ein vom Treiber uebernommenes HID-Interface mit *seinem* Interrupt-IN-Endpoint. Der
  Name folgt Circle (`CUSBFunction`, `usbdevice.cpp:410`; Obergrenze `USBDEV_MAX_FUNCTIONS = 10`,
  `usbdevice.h:31`). FreeMiNT sagt „Interface“ und bindet je Interface einen Treiber (`usb.c:1031-1034`,
  `usb_find_interface_driver` `:1062-1091`).

**Soll (Zielverhalten):**

1. Fuer **jedes** Interface, das die Auswahlregeln (§9.3) bestehen, entsteht eine Funktion. Ein Geraet kann 0…`MAX_FUNC_PER_DEV`
   Funktionen haben. Eine Maus-Funktion und eine Tastatur-Funktion desselben Geraets laufen gleichzeitig.
2. **Ein** Configure-Endpoint-Befehl je Geraet legt *alle* Funktions-Endpoints an (§9.4); danach **ein**
   `SET_CONFIGURATION`; danach je Boot-Funktion `SET_PROTOCOL(boot)` mit `wIndex` = deren Interfacenummer.
3. Alle Funktionen werden erst nach dem Durchlauf des ganzen Baums scharf gemacht (heute
   `arm_hid_endpoints`, `xhci/mod.rs:1853`, Begruendung `:1838-1852`) — jede Funktion bekommt ihr eigenes TRB.
4. Jeder fertige Transfer wird einer Funktion zugeordnet (Slot **und** DCI, wie heute fuer das Geraet,
   `xhci/mod.rs:749-751`) und mit dem Zustand *dieser* Funktion dekodiert (§9.6).
5. Die Berichts-Zuordnung zum IKBD-Strom bleibt: Tastaturfunktion → `hid::keyboard_report`, Mausfunktion →
   `hid::mouse_report`; ein Geraet mit beiden speist beide Pfade.

### 9.3 Auswahlregeln fuer Interfaces

| # | Regel | Beleg |
|---|---|---|
| R1 | Nur **Alternate Setting 0** wird beachtet. Spaetere Alt-Settings derselben `bInterfaceNumber` sind Alternativen, keine weiteren Funktionen. | Circle ueberspringt sie: `usbdevice.cpp:395-406` („Alternate setting … ignored“). FreeMiNT zaehlt dieselbe Nummer als `num_altsetting++`: `usb.c:364-377`. **Abweichung heute [G]:** `classify`/Auswahl in `config_desc.rs:211-221,287-293` pruefen `alt` nicht — ein Boot-Interface mit `alt != 0` koennte genommen werden, obwohl `SET_CONFIGURATION` Alt 0 aktiviert. F3 muss `alt == 0` verlangen [E, Korrektur eines latenten Fehlers]. |
| R2 | **Boot-Funktion:** Klasse 3, Subklasse 1, Protokoll 1 (Tastatur) oder 2 (Maus), mit Interrupt-IN-Endpoint (Auswahl wie F2). Dekodiert. | HID 1.11 §4.1–§4.3 (`config_desc.rs:1-27`); `config_desc::is_boot_hid` `:200-205`. |
| R3 | Als Endpoint der Funktion zaehlt der **erste Interrupt-IN-Endpoint des Interface** (wie heute `IfaceRow.ep_addr`, `config_desc.rs` Endpoint-Zweig). Interrupt-OUT und weitere IN-Endpoints werden ignoriert. FreeMiNT verlangt zusaetzlich `bNumEndpoints == 1` (`usb_keyboard.c` `kbd_probe`, „`bNumEndpoints != 1`“); das uebernehmen wir **nicht**, weil F2 es nicht verlangt und kein Log ein Gegenbeispiel zeigt [E]. | `config_desc.rs` Endpoint-Zweig; FreeMiNT `usb_keyboard.c:526-575`. |
| R4 | **Mehrere Funktionen gleicher Art sind erlaubt** (Rii X8: Tastatur auf Interface 0 *und* 2). Jede bekommt eigenen Zustand (§9.6). | `005_…LOG:109-121`. |
| R5 | **Report-only-Interfaces** (Subklasse 0): wie heute nur der Beobachtungs-Pfad (Rolle 3, `usb-snapshot.md` §3.2), und **nur wenn das Geraet keine Boot-Funktion hat**. Das Hersteller-Interface eines Empfaengers (Unifying Interface 2, Endpoint 0x83, max 32) wird in F3 also **nicht** gepollt. Begruendung: der Empfaenger waere sonst mit einem dritten Endpoint belastet, dessen Verkehr (HID++) nichts bewirkt; Ausweitung der Beobachtung auf alle Interfaces ist Frage 3 in §9.9. | `config_desc.rs` `Scan.observe` („Only when `hid` is `None`“); `usb-snapshot.md` §3.2, §7 Entscheidung 3. |
| R6 | **Grenzen:** hoechstens `MAX_FUNC_PER_DEV` Funktionen je Geraet und `MAX_FUNC` insgesamt (§9.5). Das naechste Interface bekommt einen neuen `SkipReason::FunctionLimit` (`config_desc.rs:96-100` Familie; Snapshot-Reason 6, noch frei: `snapshot.rs` Reason-Codes 1–5, 16–22) und die Logzeile „not used: function limit“. `OneInterfacePerDevice` entfaellt (Reason 5 bleibt als „reserviert, nicht mehr vergeben“ [E]). | F8-Regel („kein stilles Auslassen“). |
| R7 | **Endpoint-Kollision:** bildet ein zweites Interface denselben DCI (gleiche Endpoint-Nummer und Richtung) wie eine bereits gewaehlte Funktion, wird es uebersprungen und geloggt. Weil jeder DCI genau einen Endpoint-Context hat (`xhci-1.2.txt:4682-4686`), waere sonst der Kontext doppelt belegt. Dass ein gueltiger Konfigurationsdeskriptor das nicht erzeugt, ist **USB-2.0-Stoff** und hier [?]; die Regel ist reine Absicherung [E]. | xHCI: `DCI = Endpoint Number × 2 + Direction` (`xhci-1.2.txt:4682-4686`). |
| R8 | **Reihenfolge:** Funktionen in Deskriptor-Reihenfolge; Funktion 0 ist die „primaere“ (setzt Geraete-`role`/`reason`, bestimmt das Verhalten bei Fehlern, §9.4). | [E]. |

**Verhalten gegenueber heute:** fuer Geraete mit genau einer Boot-Funktion aendert sich nichts
(identische Kommandos, identische Logzeilen — Abnahme: die Logs aus `usb01-con04-pi`/`usb01-pi-kbdhub`/`usb01-snap-pi`
muessen bis auf neue Funktionszeilen gleich bleiben).

### 9.4 Endpoint, Ring und Puffer je Funktion

**Heute (Ist) [G]:** je *Geraet* `DevDma { device, input, ep0_ring, int_ring, buf, intbuf[64] }`
(`xhci/mod.rs:107-126`) — also genau ein Interrupt-Ring (16 TRBs) und ein Berichtspuffer; `HidDev` fuehrt
`dci`, `protocol`, `trb_len`, `int_enq`, `int_cycle`, `role`, `reason` (`:229-262`); `configure_endpoint`
baut *einen* Endpoint-Context (`:1645-1713`), `queue_interrupt(di)` stellt ein TRB und klingelt `(slot, dci)`
(`:658-689`).

**Soll [E, aus xHCI und Ist abgeleitet]:**

* **Aufteilung.** Ein Geraet behaelt Device-/Input-Context, EP0-Ring und `buf` (Slot-Teil). Neu eine
  Tabelle **`FUNC[MAX_FUNC]`**, je Eintrag: `{ dev: u8 (Index hid[]), iface: u8, ep_addr: u8, dci: u8,
  protocol: u8 (1/2/0), trb_len: u16, int_enq, int_cycle, role, reason }` im Treiberzustand und
  `FuncDma { int_ring: Ring (16 TRBs, 64-Byte-ausgerichtet), intbuf: [u8; 64] }` im `.dma`-Block (`Ring` ist
  `align(64)`, `xhci/mod.rs:66-70`). Rechnung ([S], aus den Strukturgroessen): `DevDma` heute
  1024 + 1056 + 256 + 256 + 256 + 64 = 2912 Byte (auf 64 aufgerundet 2944); ohne `int_ring`/`intbuf`
  2592 (2624). Eine Funktion kostet 256 + 64 = 320 Byte `.dma`.
* **Configure Endpoint.** Ein einziger Befehl je Geraet. Im Input-Context: Add-Context-Flag **A0** (Slot) und
  **A_dci** fuer jede Funktion; `Context Entries` des Slot-Context = **groesster DCI** aller Funktionen (heute
  `set_context_entries(dci)` fuer genau einen, `xhci/mod.rs:1673`). Grundlage: ein Add-Flag mit DCI >
  `Context Entries` macht den Context ungueltig (`xhci-1.2.txt:6070`); Definition des Feldes „Index des
  letzten gueltigen Endpoint-Context“ (`xhci-1.2.txt:22321-22326`). **Hinweis [?]:** `xhci-1.2.txt:4720`
  schreibt fuer den Configure-Endpoint-Befehl „Context Entries = Maximum DCI+1 of configured Endpoint
  Contexts“ — das widerspricht `:22321` dem Wortlaut nach; der heutige Code (= groesster DCI) laeuft auf
  VL805 und QEMU, der Widerspruch ist unaufgeloest und gehoert vor dem Bau geklaert (Frage 6).
  Je Endpoint-Context: Typ Interrupt-IN, Max Packet Size, eigener `TR Dequeue Pointer` = *dessen* `FuncDma.int_ring`,
  DCS 1, CErr 3, Average TRB Length und Max ESIT Payload = Max Packet, Interval nach der heutigen Formel
  (`xhci/mod.rs:1696-1708`; Rii-Maus `bInterval 1` Full-Speed ⇒ Interval 3 = 1 ms). Beispiele: `0x81 → DCI 3`,
  `0x82 → DCI 5`, `0x83 → DCI 7` (`DCI = EpNr·2 + 1`, `xhci/mod.rs:1662`).
* **Befehlsbudget.** F3 braucht **keinen** zusaetzlichen xHCI-Befehl (ein Configure Endpoint je Geraet bleibt),
  nur ein Steuerrequest `SET_PROTOCOL(boot)` mehr je weitere Boot-Funktion. F1 hat die 15er-Grenze
  entfernt; die Zahl 13/15 aus §3.1 gilt nicht mehr.
* **Bandbreite/Ressourcen.** Mehr periodische Endpoints koennen Configure Endpoint mit *Bandwidth Error* oder
  *Resource Error* scheitern lassen; der xHC laesst dabei die Zuteilungen unveraendert
  (`xhci-1.2.txt:5733-5740`, `:5840-5845`). Ob der **VL805** drei Interrupt-Endpoints eines Full-Speed-Geraets
  hinter dem Hub-TT annimmt, ist **[?]** (keine Messung; Test-Pi-Schritt P2 in §9.8).
  **Fallback [E]:** schlaegt der kombinierte Befehl fehl, einmal wiederholen mit *nur Funktion 0* (= F2-Verhalten),
  beide Versuche loggen; die uebrigen Funktionen bekommen Reason `CONFIGURE_ENDPOINT` (18). Der Wiederholungsbefehl
  ist zulaessig, weil ein gescheiterter Befehl nichts veraendert (`xhci-1.2.txt:5738-5740`).
* **Reihenfolge der Steuerrequests [G aus Ist, E fuer Neues].** Configure Endpoint **vor** `SET_CONFIGURATION`
  (`xhci/mod.rs:1743-1754`, xHCI-Reihenfolge `xhci-1.2.txt:4346-4364`), dann `SET_PROTOCOL(boot)` je Boot-Funktion
  in Funktionsreihenfolge (HID 1.11 §7.2.6: gilt je Interface, `wIndex` = Interface; Geraete stehen nach dem
  Einschalten im Report-Protokoll, der Host soll es setzen, `hid1_11.pdf` §7.2.6). **Alles davor** liegt in
  `configure_hid`, **vor** `fetch_extras` (Report-Deskriptoren, Strings; `xhci/mod.rs:1967-1977`), damit ein
  STALL der riskanten Abrufe kein `SET_PROTOCOL` mehr trifft (`usb-snapshot.md` §3.1).
* **Fehlerverhalten bei Funktion k > 0 [E].** Scheitert `SET_PROTOCOL` fuer die primaere Funktion 0 → wie heute
  Geraet verworfen (`xhci/mod.rs:1757-1761`). Scheitert es fuer Funktion k > 0 → *nur diese* Funktion wird
  nicht scharf gemacht (Reason 20), das Geraet bleibt; weitere Steuerrequests an dasselbe Geraet unterbleiben
  (ein STALL setzt EP0 auf Halted, einen Reset-Endpoint-Pfad gibt es nicht — `usb-snapshot.md` §3.1, Zitat xHCI
  §4.8.3 dort) — das beendet auch `fetch_extras` fuer dieses Geraet.
* **Zuordnung von Events.** Das Transfer-Event traegt `Slot ID` und `Endpoint ID` = DCI (`xhci-1.2.txt:1571`).
  `poll_interrupt` sucht heute den Besitzer in `hid[0..n_hid]` mit `(slot, dci)` (`xhci/mod.rs:749-751`) und
  liefert den Geraeteindex; es sucht kuenftig in `FUNC[..n_func]` und liefert den **Funktionsindex**. `queue_interrupt`
  nimmt Funktionsindex; `note_error` zaehlt je Geraet *und* Funktion (§9.7). Ein Event ohne Besitzer bleibt „stray“.
* **Steady State.** Je Funktion hoechstens **ein** ausstehendes TRB (wie heute je Geraet). Also bis zu `n_func + 1`
  ausstehende Events (Funktionen + ein Steuerevent). Das ist die Grenze aus dem Kommentar `xhci/mod.rs:377-385`
  („at most `MAX_HID + 1`“), die dort auf den Event-Ring (16 Eintraege, `EVT_RING_LEN`, `:57`) abgestimmt ist →
  **`MAX_FUNC + 1 ≤ EVT_RING_LEN`**, sonst droht ein voller Event-Ring [S, aus dem Kommentar; xHCI-Ereignisringregel
  nicht nachgelesen → [?]].

### 9.5 Limits

| Grenze | Wert / Vorschlag | Beleg / Begruendung |
|---|---|---|
| Geraete `MAX_HID` | **8, unveraendert** | `xhci/mod.rs:98`. Funktionen verbrauchen keinen `hid[]`-Eintrag. Die Pi-Baeume aus den Logs brauchen 3–4. |
| `MAX_FUNC_PER_DEV` | **4** [E] | Passt zu `snapshot::REPORT_DESC_IFACES = 4` (`snapshot.rs:25`); beobachtetes Maximum 3 Interfaces je Geraet (alle Logs). Circle erlaubt 10 (`usbdevice.h:31`), FreeMiNT 8 Interfaces/Konfiguration (`usb.h:40`). |
| `MAX_FUNC` | **12** [E], harte Obergrenze **15** | 15 = `EVT_RING_LEN − 1` (§9.4 „Steady State“). 12 deckt 8 Geraete × 1 + 4 Zusatzfunktionen; bei Bedarf anheben, dann aber Event-Ring und `PENDING_EVENTS` (`:385`) mitvergroessern. |
| `.dma` je Funktion | 320 Byte → 12 × 320 = 3,75 KiB | §9.4; der `.dma`-Block hat 2 MiB (`[pcie] .dma block 0xe00000..0x1000000` in den Logs). |
| Pollrate | **ein** Bericht je `poll()`-Aufruf | `usb/mod.rs:198` (`if let Some(..) = c.poll_interrupt()`), `snapshot` §2.6 „Abholrate“. Mit 3 Endpoints je Empfaenger konkurrieren mehr Quellen um denselben Durchsatz (Rii-Maus: `bInterval 1`, also bis zu 1000 Berichte/s) [S]; nicht neu, aber F3 verschaerft es. Ob das sichtbar traegt, ist Messfrage (P4). |
| Konfigurationspuffer | 1024 Byte (bereits seit K2) | `snapshot::CONFIG_MAX`; alle bekannten Empfaenger 84 Byte. |
| Interface-Tabelle | 16 Zeilen (bereits) | `config_desc.rs` `MAX_IFACE_ROWS`; Gamepad `045e:028e` zeigt 4 Interfaces, 153 Byte (`8Bit_Do_Joypad.LOG`). |

### 9.6 Mehrere Tastaturen (und Mäuse) gleichzeitig

**Befund A — was der Treiber heute tut [G].**

* Jedes Geraet mit Boot-Tastatur-Interface wird `protocol 1`, jedes wird scharf gemacht
  (`xhci/mod.rs:1967-1969`, `:1853-1859`); `usb::poll` dekodiert jeden Bericht mit `hid::keyboard_report`
  (`usb/mod.rs:221`).
* `hid::keyboard_report` vergleicht den neuen Bericht mit **einem einzigen globalen** Vorzustand
  `static PREV: AtomicU64` (`hid.rs:47`, Lesen/Schreiben `:86-117`): Break fuer jede Taste im alten, aber
  nicht im neuen Bericht, Make fuer jede im neuen, aber nicht im alten (`hid.rs:94-111`); Modifier genauso
  (`:91-92`). `release_all` beim MUTE-Beginn laeuft ueber denselben einen `PREV` (`hid.rs:57-65`). Der Zustand
  gehoert also **nicht** zum Geraet.
* Sind zwei Tastaturen aktiv, ueberschreibt jeder Bericht den Zustand des anderen: der Diff vergleicht
  Tastatur B mit dem *Zustand von Tastatur A*. **Ableitung [S] (reine Rechnung am Code, nicht beobachtet):**
  Kabeltastatur haelt „a“ (Bericht `[a]`), Empfaenger-Tastatur meldet leer (`[]`):
  `[a]` → Make a · `[]` → **Break a** · naechster `[a]` der Kabeltastatur → **Make a** … Taste und Modifier
  „flackern“ so oft, wie Berichte der beiden Geraete einander abwechseln. Halten beide je eine Taste, kippen
  bei jedem Bericht beide (`[a]`→`[b]`: Break a, Make b; dann `[a]`: Break b, Make a …).
* Die Wirkung haengt an der **Berichtskadenz beider Geraete**. HID 1.11: Boot-Tastaturen *sollen* Berichte
  auch ohne Aenderung senden (`hid1_11.pdf` Anh. F.3, Textauszug Z. 3358-3360), der empfohlene
  Standard-Idle-Wert fuer Tastaturen ist **500 ms** (§7.2.4, Auszug Z. 2433). **Unser Treiber sendet nie
  `SET_IDLE`** [G: einziger Klassenrequest ist `SET_PROTOCOL` `xhci/mod.rs:1639-1641`; Suche nach 0x0A:
  kein Treffer]; FreeMiNT sendet `SET_IDLE(0,0)` („report infinite“) vor `SET_PROTOCOL` fuer Tastatur und
  Maus (`usb_keyboard.c:599-602`, `mouse-boot/usb_mouse.c:425`, `mouse/usb_mouse.c:731`). Eine
  Empfaenger-Tastatur ohne Funk-Tastatur im Set kann also — je nach Firmware — alle 500 ms (oder bei jedem
  Poll, 8–10 ms) einen leeren Bericht liefern. Welches davon die Empfaenger tun, ist **[?]**: die Logs
  enden vor dem Eingabeverkehr.
* **`mouse_report` ist zustandslos pro Bericht** (`hid.rs:127-136`): Delta + Tastenbyte. Zwei Maeuse addieren sich
  im IKBD-Strom korrekt (relative Deltas); das **Tastenbyte** des letzten Berichts gewinnt aber
  (`hid.rs:128-131`: `0xF8 | wire` je Bericht, `MOUSE_BUTTONS` nur fuer `release_all` `:50`). Haelt Maus A links
  und bewegt sich Maus B ohne Tasten, kommt ein Paket „keine Taste“ = Loslassen [S].

**Befund B — was die Referenzen sagen.**

| Quelle | Aussage | Beleg |
|---|---|---|
| FreeMiNT | **Eine** Tastatur zur Zeit: `kbd_probe` weist jede weitere mit `-1` ab (Kommentar „Only one keyboard at time“); ebenso eine Maus (`mouse-boot`, `mouse`). Der Zustand `olddata/newdata` ist ein einziges `static kbd_data`. **Erste gewinnt:** haette ein Empfaenger-Tastatur-Interface vor der Kabeltastatur enumeriert, wuerde in FreeMiNT die *Kabeltastatur* abgewiesen. Damit ist FreeMiNT hier **kein Vorbild fuer „beide leben“**, sondern nur fuer die Bindung je Interface (`usb.c:1031-1034`). | `usb_keyboard.c:533-538`, `:126-141`; `mouse-boot/usb_mouse.c:359-364`; `mouse/usb_mouse.c:628-633`. |
| FreeMiNT | Zustandsbehandlung beim Lesen: Laenge < 3 oder > 8 → verwerfen, Bytes hinter `actlen` auf 0. | `usb_keyboard.c:364-373`. |
| Circle | Jede Tastatur ist ein **eigenes Objekt** mit **eigenem** `m_LastReport` und eigener Gerätenummer (Namenspraefix `ukbd`, `usbkeyboard.cpp:32`, Nummernpool `:29,80-82`). | `usbkeyboard.h:79`, `usbkeyboard.cpp:29,32,80-82,216-249`. |
| Circle | „setting the LED status forces some keyboard adapters to work“ — Tastatur-Adapter, die ohne LED-Output-Report schweigen. | `usbkeyboard.cpp:77-78`. Beleg fuer die *Existenz* solcher Adapter, **kein** Beleg fuer die Empfaenger hier. |
| HID 1.11 | Jedes Interface ist ein eigenes HID-Geraet mit eigenem Protokoll-/Idle-Zustand (`wIndex` = Interface in `Set_Protocol`/`Set_Idle`); mehrere Interface-Deskriptoren teilen Endpoint 0 (§4.4); das Beispielgeraet im Anh. E „Tastatur mit integriertem Zeigegeraet“ ist **absichtlich als zwei Interfaces** gebaut, damit es das Boot-Protokoll kann (Auszug Z. 2994-2999, Maus-Interface Z. 3195-3203). Die **Zusammenfuehrung mehrerer Tastaturen beim Host** legt HID nicht fest. | `hid1_11.pdf` §4.4, §7.2.4, §7.2.6, Anh. E. |

**Soll (Entwurf) [E]:**

1. **Zustand je Funktion, nicht global.** `PREV` wird ein Feld der Funktion (`prev: u64` je `FUNC[i]`); `keyboard_report`
   bekommt den Funktionsindex. Das beseitigt den Diff gegen fremden Zustand. `release_all` laeuft ueber alle
   Funktionen. Minimalfix, aendert bei einer Tastatur nichts.
2. **Alle Tastaturen speisen denselben IKBD-Strom** (ein Atari hat eine Tastatur; `ikbd.rs` fuehrt einen
   Modifier-/Wiederholzustand, `ikbd.rs:39-43,93,321`). Bekannte Grenze: haelt Tastatur A „a“ und
   Tastatur B laesst „a“ los, entsteht ein Break trotz gehaltener Taste auf A. **Option** (kostet 128 Byte):
   Zaehler je Scancode, Make nur bei 0→1, Break nur bei 1→0. Empfehlung: erst Punkt 1, den Zaehler nur wenn eine
   Messung (P3) Bedarf zeigt — Frage 4.
3. **Maus-Tasten OR-verknuepfen:** Zustand `buttons` je Mausfunktion; das gesendete Tastenbyte ist das OR aller
   Mausfunktionen (Bits 0/1). Gleiche Argumentation wie Punkt 1. [E]
4. **`SET_IDLE` ist nicht Teil von F3.** Es erklaert moeglicherweise die Berichtskadenz (§9.6 oben) und FreeMiNT
   sendet es, aber: ein `SET_IDLE`, das ein Geraet mit STALL beantwortet, setzt EP0 auf Halted ohne Reset-Pfad
   (`usb-snapshot.md` §3.1). Boot-Tastaturen *muessen* es koennen (`hid1_11.pdf` Anh. F.3, Z. 3358); fuer Funk-Interfaces
   unbekannt [?]. Entscheidung dem Anwender (Frage 5); falls ja: nach allen `SET_PROTOCOL`, vor `fetch_extras`.

**Befund C — der Ausfall der Kabeltastatur mit Empfaenger (Ragnar, Starts 1, 2, 5–8) [Hypothesen].**

* Beobachtung [G]: Starts 1/2/5–8 „Tastatur nein“; Start 3/4 (B100 statt Empfaenger) „ja“
  (`NOTIZEN.TXT`). Kabeltastatur in allen Logs gleich konfiguriert (§9.1, Gegenprobe). **Einziger Unterschied
  der beiden Gruppen im Log:** ein zweites Geraet mit `protocol 1` (Empfaenger-Interface 0, Slot 2) [G].
* **H1 (Mechanismus belegt, Wirkung nicht): gemeinsamer `PREV`.** Der Code erzeugt bei zwei aktiven Tastaturen
  Scancode-Flackern (siehe Befund A). **Was H1 vorhersagt:** falsche/ doppelte Zeichen, Modifier (Shift, Ctrl)
  fallen weg, Wiederholung bricht ab, bei hoher Kadenz beider Geraete (8–10 ms, ohne Idle) ein
  Zeichenhagel — **nicht** zwingend voelliges Schweigen. Ob „nein“ des Testers „gar nichts“ oder „unbrauchbar“
  heisst, ist offen (Frage 1). H1 erklaert damit „nicht tippbar“, aber nicht automatisch „keine Reaktion“ [S].
* **H2 [H]: Fehler-/Neuarmierungsschleife.** Liefert ein Endpoint (z. B. der des Empfaengers) dauernd einen
  Nicht-Erfolgs-Code, protokolliert `poll_interrupt` jeden und armiert sofort neu (`xhci/mod.rs:781-796`) —
  im selben Aufruf, bis zu `PENDING_EVENTS + EVT_RING_LEN` Durchlaeufe, je mit `kprintln`. Das koennte den Pump
  ausbremsen. **Kein Hinweis darauf im Material** (die Logs enden vor dem Verkehr). Pruefbar am Zaehler
  `int_errors`/`last_cc` je Geraet (`snapshot.rs` Entry-Bytes 28-30), den eine `USBSNAP`-Aufnahme liefert.
* **H3 [H]: Begriff.** Der Tester testet in 5–8 evtl. die Funktastatur des Sets statt der Kabeltastatur (die
  Spalte heisst „Tastatur“, die Kopfzeile nennt die Cherry KC 6000). Bei Start 1/2 (reine Maus) ist das
  ausgeschlossen. Frage 1.
* **H4 [H]: Hub-TT/Bandbreite.** Ein Full-Speed- und ein Low-Speed-Interrupt-Endpoint hinter *demselben* High-Speed-Hub.
  In den Arbeitslaeufen waren es zwei Low-Speed-Geraete (B100, Cherry KC 6000), im Apple-Lauf eines je
  (`apple_tastatur.LOG`: Full-Speed-Tastatur + Low-Speed-Maus, beides geht). Kein Hinweis auf ein Problem.
  Nur der Vollstaendigkeit halber.
* **Nicht erklaerbar aus dem Material:** wie H1 allein ein *vollstaendiges* Versagen ergaebe. Entscheidende
  Messung: **eine `USBSNAP`-Aufnahme mit Empfaenger + Kabeltastatur** (der Ring je Geraet zeigt Berichte und
  Zeitstempel *beider* Tastaturen getrennt, `usb/mod.rs:203-208` zeichnet vor dem Dekodieren auf; Kernel mit
  Modus 2022 ist `main`). Sie braucht kein F3.

**Befund D — „Zeiger ja“ bei nie gepollter Maus.** [G] In den Logs 001, 002, 005, 006, 007 existiert **kein** Geraet mit
`protocol 2`: die einzigen Zeilen „HID configured“ lauten zweimal `(keyboard)` (`grep` ueber `serial/*.LOG`: 0 × „(mouse)“,
2 × „(keyboard)“). **Der Log zu Start 8 (Cherry DW 9000, blau) liegt nicht im Ordner** — `serial/` hat 10 Dateien,
`RUNINFO.TXT` nennt 11 [G]. `mouse_report` (`hid.rs:127`) ist nur fuer
`protocol 2` erreichbar (`usb/mod.rs:225`). Also kann **kein** Mauspaket aus dem Empfaenger stammen [S, aus dem Code].
Moegliche Lesarten [H]: (a) „Zeiger“ = der Mauszeiger ist **sichtbar** (nicht „bewegt sich“); ob der Zeiger
ohne Maus gezeichnet wird, ist hier nicht geprueft [?]; (b) der Zeiger bewegte sich ueber eine andere Quelle
(z. B. die Alt+Pfeil-Mausemulation der EmuTOS-BIOS, `references/emutos/bios/ikbd.c:286-322`, die mit funktionierender Tastatur
ginge — widerspricht „Tastatur nein“). Frage 1 klaert es in einem Satz.

### 9.7 Auswirkung auf den Snapshot-Speicher (`usb-snapshot.md`)

Der Snapshot ist heute **ein Eintrag je Geraet** (`Device`, `snapshot.rs:276-330`, `MAX_DEVICES = 8`) mit **einem** Ring
(`ring_iface`/`ring_ep`, `snapshot.rs:302-304`; Eintraege 18/19 des Listeneintrags `usb-snapshot.md` §2.4;
Ring 256 × 80 Byte `snapshot.rs:29,44`). Entscheidung 3 in `usb-snapshot.md` §7 nannte das ausdruecklich als Grenze
(„mehrere Ringe erst mit F3“). Folgen von F3:

| Stelle | Heute | Mit F3 [E, Optionen] |
|---|---|---|
| Ring | 1 je Geraet, nur das gewaehlte Interface | **Option A (empfohlen):** weiter *ein* Ring je Geraet (Speicher bleibt 240 KiB), aber jeder Eintrag bekommt die Funktion: die 4 reservierten Bytes bei +12 des 80-Byte-Eintrags (`usb-snapshot.md` §2.6, „reserviert, 0“) tragen `iface` und `ep_addr`. Berichte der drei Endpoints eines Empfaengers laufen in einer `seq`-Folge (Reihenfolge des Eintreffens — genau das, was H1 braucht). **Option B:** ein Ring je Funktion = `MAX_FUNC` × 20 KiB = 240 KiB zusaetzlich `.bss` (bei 12), dafuer eigene `seq` je Funktion. Kosten-Nutzen: A billiger und zeigt die Interleaving-Reihenfolge, B einfacher zu lesen. |
| `ring_iface` / `ring_ep` (Eintrag 18/19) | das ueberwachte Interface | = Funktion 0 (primaer, R8); `0xFF` bleibt „kein Ring“. Gilt nur noch als „erste Funktion“. |
| `role` (Eintrag 14) | eine Rolle je Geraet (1 Tastatur / 2 Maus / 3 beobachtet …) | = Rolle der primaeren Funktion (Empfaenger: 1). **Reicht nicht**, um „Tastatur + Maus“ zu zeigen: `USBSNAP.NCP` deutet die Berichte nach `role` (`sdk/examples/usbsnap.c:1113-1160`, Zeilen 1129-1153) und wuerde die Mausberichte als Tastaturberichte lesen. Noetig: **Rolle je Interface-Zeile** (Zeilenbyte 15 ist reserviert, `usb-snapshot.md` §2.5) und `n_funcs` in Eintragsbyte 31 (reserviert). |
| Interface-Tabelle `monitored` (Zeilenbyte 14) | genau ein Interface 1 | bereits ein Flag je Zeile (`snapshot.rs:168-190`) — traegt mehrere Funktionen ohne Aenderung. |
| Reason-Codes | 5 = `OneInterfacePerDevice` | 5 entfaellt (reserviert), 6 = `FunctionLimit` (neu). `config_desc::SkipReason` und `snapshot::reason` gleichziehen (`xhci/mod.rs:186` `skip_code`). |
| `int_errors`/`last_cc` (Eintrag 28-30) | je Geraet | bleibt je Geraet (Summe der Funktionen); je Funktion nur mit Option B. |
| `Ssystem(2022)`-Version | 1 | Option A/B aendern die **Bedeutung** reservierter Felder → empfohlen **`VERSION = 2`** (`snapshot.rs:38`; Regel §2.3 Nr. 3: unbekannte Version → `EINVAL` mit Kopf) und `USBSNAP.NCP` im selben Schritt anpassen (Kernel und Programm gehen ohnehin zusammen raus, `usb-snapshot.md` §4.1). Alternative: Version 1 beibehalten, v1-Programme zeigen dann gemischte Berichte falsch beschriftet (kein Absturz). |
| Erste Boot-Anzeige (`device_info`, `xhci/mod.rs:1315`) | ein Eintrag je Geraet, `protocol` = Rolle | zeigt „Keyboard“ fuer den Empfaenger; „Keyboard + Mouse“ ist USB02 #4/#8-Gebiet, **nicht** F3. |

### 9.8 Testplan

**Host (ohne Hardware, `cargo test` in `tools/xtask`; `config_desc.rs`, `snapshot.rs`, `ring.rs` werden dort per
Pfad eingebunden, `tools/xtask/src/main.rs:55-75`).** Bauen, sobald F3 freigegeben ist:

* **H-1 Auswahl.** Synthetische Konfigurationen mit dem Aufbau aus §9.1 (Interface-/HID-/Endpoint-Deskriptoren;
  `wDescriptorLength` der Empfaenger steht **nicht** in den Logs [?] — Fixture setzt einen Platzhalter und prueft
  nur Aufbau und Auswahl): Unifying (Tastatur, Maus, report-only → 2 Funktionen, Interface 2 `NotBootSubclass`),
  Rii X8 (3 Boot-Funktionen: Tastatur, Maus, Tastatur), Cherry DW 9000, Apple-Tastatur + B100 (Regression: je 1 Funktion), Gamepad
  `045e:028e` (0 Funktionen, 4× `NotHid`), Alt-Setting-Fall (R1), Kollisions-Fall (R7), mehr als `MAX_FUNC_PER_DEV`
  (R6), abgeschnittene Konfiguration (`stopped_at`).
* **H-2 Zustand je Funktion.** Den Tasten-Diff als reine Funktion herausziehen (heute in `hid.rs:86-123` mit `ikbd::feed`
  verdrahtet) und mit zwei verschachtelten Berichtsfolgen testen: (i) Kabeltastatur haelt `a`, Empfaenger sendet
  dauernd `[]` → **kein** Break fuer `a` mit Zustand je Funktion; (ii) zwei Tastaturen halten `a` und `b`;
  (iii) Modifier; (iv) `release_all` ueber alle Funktionen. Diese Tests zeigen *mit dem heutigen globalen `PREV` das
  Flackern* (Rot) und mit dem Fix das Gegenteil (Gruen) — damit ist H1 als Mechanismus pruefbar.
* **H-3 Snapshot.** Ring-Eintrag mit `iface`/`ep`-Feld, Zeilenrolle, `n_funcs`, `FunctionLimit`-Reason in der Op-2-Antwort.

**QEMU (`cargo xtask run --board qemu-virt [--usb-extra] [--usb-dev <spec>]`; raspi4b hat keinen USB-Controller,
`tools/xtask/src/main.rs:719-720`).**

* *Was `--usb-extra`/`--usb-dev` emulieren kann* [G aus `tools/xtask/src/main.rs:613-635,172-187`]: Standard `usb-kbd` +
  `usb-mouse`; `--usb-extra` zusaetzlich `usb-tablet` (Root-Port 3) und `usb-hub` (Root-Port 4) mit `usb-mouse` (4.1) und
  `usb-kbd` (4.2); `--usb-dev` haengt **jede** QEMU-`-device`-Spezifikation an. `qemu-system-aarch64 -device help`
  (11.0.2, lokal ausgefuehrt, nur gelesen) kennt an HID-artigen Geraeten `usb-kbd`, `usb-mouse`, `usb-tablet`, `usb-wacom-tablet`,
  `usb-braille` und `usb-host` (Durchreichen eines Host-Geraets).
* *Was damit NICHT geht* [S, QEMU-Quelle ist nicht lokal → die Deskriptoren der Geraete sind [?]]: **kein QEMU-Geraet bildet
  einen Funkempfaenger nach** (mehrere HID-Interfaces in einer Konfiguration); `usb-kbd`/`usb-mouse`/`usb-tablet` sind nach
  bisherigen Logs Ein-Interface-Geraete. Der Mehr-Interface-Pfad von F3 ist in QEMU **nicht durchlaufbar**. `usb-host`
  koennte ein echtes Funkgeraet vom Mac durchreichen; ob das auf macOS geht, ist nicht geprueft [?] und waere ein
  Eingriff in die Entwicklungsmaschine — nicht vorgeschlagen.
* *Was geht:* (1) **Regression** Ein-Funktions-Geraete (Standard, `--usb-extra`) — Eingabe wie vorher
  (`measurements/measurement-2026-10-23-usb01-snap-kernel`-Verfahren). (2) **Zwei Boot-Tastaturen gleichzeitig:**
  `--usb-extra` hat sie bereits (Wurzel-`usb-kbd` + `usb-kbd` am Hub); weitere per `--usb-dev usb-kbd`. Der Tastaturverkehr
  laesst sich aber **nicht gezielt auf eine** Tastatur lenken: QMP `input-send-event` mit `device=<usb-id>` bricht ab
  (QEMU 11.0.2; `measurements/measurement-2026-10-23-usb01-test/RUNINFO.TXT:46`, USB02 #7), Tasten gehen ueber die
  Standardroute. Welche der Tastaturen QEMU damit bedient, ist **[?]**. Das Flackern (H1) ist darum in QEMU nur mit Glueck
  nachweisbar — der Nachweis laeuft ueber H-2. (3) **Budget:** viele `--usb-dev` (Hubs, Mäuse) bis `MAX_HID = 8`
  und darueber → Logzeile „not enumerated — all 8 device entries in use“ (`xhci/mod.rs:1805-1812`).
* *Moeglich, aber ausserhalb dieses Auftrags:* ein kleines eigenes QEMU-Geraet bzw. ein `usb-host`-Aequivalent mit
  zwei HID-Interfaces — **Code ausserhalb des Repos**, Entscheidung beim Anwender (Frage 7).

**Echte Pi (Eigener Lauf + Ragnar).**

* **P0 (vor dem Bau, kein F3 noetig).** Ragnar (oder der Anwender mit eigenem Funkset): `USBSNAP.NCP` mit Empfaenger +
  Kabeltastatur, 30 s tippen und Maus bewegen. Auswerten: Berichte/Zeitstempel/`int_errors` der Slots 2 und 3 getrennt
  (entscheidet H1 gegen H2). Kernel: `main` mit Modus 2022 (nicht der `7c1f621d`-Lauf).
* **P1 Regression** der bekannten Aufbauten (Apple-Tastatur + ROCCAT in den beiden Steckpositionen,
  `usb01-con04-pi`/`usb01-pi-kbdhub`/`usb01-snap-pi`): gleiche Befehlszahl (13), gleiche Logzeilen bis auf neue Funktionszeilen.
* **P2 Empfaenger einzeln** (Unifying/MX Master, Rii X8, Cherry DW 9000), **ohne** Kabeltastatur: Log zeigt je Empfaenger
  zwei bzw. drei „function“-Zeilen, `Configure Endpoint → Success` (sonst Fallback-Zeile); **Zeiger bewegt sich,
  links/rechts gehen** (die Soll-Wirkung von F3), Funktastatur tippt. Befehlszahl = bisher (10) — F3 fuegt keine hinzu.
* **P3 Empfaenger + Kabeltastatur** (Ragnars Aufbau): Kabeltastatur tippt fehlerfrei, auch mit gehaltenen Tasten und Shift;
  gleichzeitig Funktastatur tippen (nur Rii/Cherry DW) — kein Flackern.
* **P4 Last:** Rii-Maus (1 ms) bewegen *und* tippen; Zeiger ruckelt nicht erkennbar mehr als mit der B100.
* **P5 Gamepad** `045e:028e` bleibt „no HID boot keyboard/mouse interface“ (Regression von F2).
* Abnahmekriterium je Lauf: kein „stray transfer event“, kein „cmd timeout“, kein „re-arming“ im Eingabefenster
  (`usb-diag`-Kernel; die Logs muessen das Eingabefenster mitschneiden — der Capture endet heute bei „after initinfo“,
  `bootlog`, deshalb P0/`USBSNAP` fuer Verkehr).

### 9.9 Offene Fragen an den Anwender

1. **Ragnar, bitte konkretisieren:** (a) „Tastatur nein“ in Start 1/2/5–8 — *gar keine* Zeichen, falsche/doppelte Zeichen
   oder nur Sondertasten? Welche Tastatur (Kabel-Cherry oder die Funktastatur des Sets)? (b) „Zeiger ja“ — *bewegte* sich der Zeiger
   mit der Funkmaus, oder war er nur sichtbar? (c) Ging die Kabeltastatur **allein** (ohne Empfaenger) im selben Kernel?
   (d) Moegliche P0-Aufnahme mit dem neuen Kernel + `USBSNAP`.
2. **Freigabe der Auswahlregeln R1–R8** (insb. R5: Report-only-Interfaces eines Geraets mit Boot-Funktion bleiben ungepollt;
   R3: kein `bNumEndpoints == 1`).
3. **Beobachtung ausweiten?** Sollen die report-only-Interfaces eines Empfaengers (Unifying Interface 2) kuenftig *zusaetzlich* beobachtet
   (gepollt, aufgezeichnet) werden? Nutzen: Daten fuer F4/F5; Kosten: ein Endpoint mehr, HID++-Verkehr im Ring, mehr Chancen fuer H2.
   Empfehlung: **nein bis P0/P2 gelaufen sind**.
4. **Mehrere Tastaturen:** reicht „Zustand je Funktion“ (Empfehlung), oder gleich der Zaehler je Scancode (§9.6 Soll 2)?
5. **`SET_IDLE`** (FreeMiNT sendet es, wir nie): einfuehren? Ja/nein, wenn ja nur an Tastatur-Funktionen und nach allen
   `SET_PROTOCOL`, Fehler nicht fatal. Es koennte die Kadenzfrage aus H1 *entschaerfen*, ist aber kein Fix fuer den gemeinsamen `PREV`.
6. **xHCI-Spec-Widerspruch** `Context Entries` (`xhci-1.2.txt:4720` vs `:22321`) vor dem Bau klaeren lassen (Rechercheur
   kann das in einem Folgeauftrag gegen die Tabellen 6-x pruefen) — betrifft nur die mehrfache Belegung.
7. **QEMU-Mehr-Interface-Geraet** (eigenes Geraet oder `usb-host`): lohnt der Aufwand, oder genuegt Host-Test (H-1) + Pi (P2/P3)?
8. **Snapshot:** Option A (ein Ring, Eintrags-Tag) oder B (Ring je Funktion); `VERSION = 2` mit gleichzeitiger Programmanpassung ja/nein.
9. **Limits** `MAX_FUNC_PER_DEV = 4`, `MAX_FUNC = 12` (Obergrenze 15): einverstanden?
10. **Fallback bei Bandwidth/Resource Error** (§9.4): auf Funktion 0 zurueckfallen — oder das Geraet ganz verwerfen (lauter Fehler)?

### 9.10 Ausserhalb des Scopes beobachtet

* `hid::keyboard_report` ignoriert die Berichtslaenge `len` (`usb/mod.rs:210`, `hid.rs:86`): ein Bericht < 8 Byte
  laesst alte Pufferbytes im Diff stehen; FreeMiNT verwirft < 3/> 8 Byte und nullt den Rest (`usb_keyboard.c:364-373`).
* `hid::modifiers` bildet LCtrl und RCtrl auf **einen** Scancode `0x1d` ab (`bits & 0x11`, `hid.rs:70-72`): Loslassen
  der einen Ctrl-Taste bei gehaltener anderen erzeugt einen Break (gilt schon fuer *eine* Tastatur; Alt gleich).
* Das Log fuer das Eingabefenster fehlt in der Diagnoseausgabe (Capture endet bei „after initinfo“): Ragnars Logs koennen
  per Konstruktion keinen Tastaturverkehr zeigen; ein Mess-Fenster ist in §5.3 vorgeschlagen, nie gebaut worden.
* `config_desc.rs`-Kopfkommentar nennt F3 weiterhin „one interface per device (F3)“ und `SkipReason::OneInterfacePerDevice`;
  beides fliegt mit dem Bau (R6).

### 9.11 Entscheidungen (Anwender 2026-10-24)

Der Anwender hat die Empfehlungen zu §9.9 freigegeben („ok du kannst loslegen“):

| Frage | Entscheidung |
|---|---|
| 1 Ragnar | Rueckfragen gehen in GitHub #3; F3 wartet nicht auf die Antwort. |
| 2 R1–R8 | freigegeben wie §9.3 (einschliesslich R3 und R5). |
| 3 Report-only beobachten | nein, erst nach P0/P2 erneut entscheiden. |
| 4 Mehrere Tastaturen | Zustand je Funktion (§9.6 Soll 1); kein Zaehler je Scancode. |
| 5 `SET_IDLE` | nicht in F3; eigenes Ticket, weil ein STALL EP0 ohne Reset-Pfad haelt. |
| 6 `Context Entries` | heutiges Verhalten beibehalten (laeuft auf VL805 und QEMU); Widerspruch nur notieren. |
| 7 QEMU-Mehr-Interface-Geraet | nein; Host-Test H-1 mit den echten Deskriptoren aus Ragnars Logs plus Pi P2/P3. |
| 8 Snapshot | Option A (ein Ring je Geraet, Interface-Tag im Eintrag), `VERSION = 2`, `USBSNAP.NCP` im selben Schritt angepasst. |
| 9 Limits | `MAX_FUNC_PER_DEV = 4`, `MAX_FUNC = 12`. |
| 10 Fallback | bei Bandwidth/Resource Error auf Funktion 0 zurueckfallen, Geraet behalten, Fehler loggen. |
| P0 | F3 wird ohne vorherige USBSNAP-Messung gebaut; die Messung klaert nur die Tastaturfrage. |

### 9.12 Umsetzung (2026-10-24)

Gebaut nach §9.2–§9.7 mit den Entscheidungen §9.11: `config_desc::scan` waehlt alle Boot-Funktionen
(R1–R8, `MAX_FUNC_PER_DEV = 4`), `xhci` fuehrt eine Funktionstabelle (`MAX_FUNC = 12`, je Funktion
Ring + Puffer in `.dma`), ein Configure Endpoint je Geraet mit allen Endpoints (Context Entries =
groesster DCI, Entscheidung 6), Rueckfall auf Funktion 0, `SET_PROTOCOL` je Boot-Funktion,
Event-Zuordnung ueber Slot **und** DCI auf die Funktion; Decoder-Zustand je Funktion
(`hid_decode.rs`, Maustasten ODER-verknuepft, §9.6 Soll 1/3); Snapshot-Layout v2 (Option A).
Einzelheiten, die §9 offen liess (Reason-Codes 7/8 fuer R1/R7, Rueckfall bei jedem
Completion-Code-Fehler, Verhalten der Funktionen nach einem gescheiterten `SET_PROTOCOL` k > 0,
Gesamtbudget), stehen als **[E]** in `usb-snapshot.md` §10. Belege
`measurements/measurement-2026-10-24-usb01-f3/RUNINFO.TXT`. QEMU kann den Mehr-Interface-Pfad
nicht durchlaufen (§9.8); er ist durch Host-Tests H-1 mit Ragnars Deskriptoren und eine
Wiedergabe von `USBSNAP.NCP` mit synthetischen Kernel-Antworten abgedeckt, die Wirkung (Zeiger und
Tastatur eines Funkempfaengers) nur an der echten Pi (P2/P3).

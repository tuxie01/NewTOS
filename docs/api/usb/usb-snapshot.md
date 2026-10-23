# USB-Schnappschuss: `Ssystem`-Modus 2022 und das Konsolenprogramm `USBSNAP` (USB01 Schritt 2, Weg a)

> **STATUS: FREIGEGEBEN 2026-10-23** (Anwender: „ja mach los“, alle Empfehlungen aus §7,
> Entscheidungen dort). **Kernelteil K1–K9 gebaut 2026-10-23** (Umsetzungsnotizen §9);
> **Programm `USBSNAP` (K10) und QEMU-Option (K11) gebaut 2026-10-23** (§9.1), dazu eine
> Kernel-Nachbesserung am MUTE-Beginn (§9.1); offen: Tester-Abnahme, Test an der echten Pi
> (T8). **Layout v2 seit USB01 F3 (2026-10-24, §10):** mehrere Funktionen je Geraet
> (`hid-mouse-protocols.md` §9.7, Option A). Dieses Dokument ist **Spec und
> Analyse, kein Code**. Es ergaenzt [`hid-mouse-protocols.md`](hid-mouse-protocols.md)
> §5.1 (gemeinsamer Kern) und §5.2 (Weg a); der Anwender hat Weg (a) am 2026-10-23
> gewaehlt. Ticket: [`TASK/USB01.md`](../../../TASK/USB01.md). Modus-Zeile:
> [`../gemdos/154-ssystem.md`](../gemdos/154-ssystem.md) (DRAFT-Tabelle).

**Scope in einem Satz:** Welche USB-Daten haelt der Kernel heute, welche muss er
zusaetzlich beim Enumerieren festhalten, wie reicht ein neuer `Ssystem`-Modus sie als
versionierten Little-Endian-Puffer an ein EL0-Programm, wie sieht dieses Programm
(`USBSNAP.NCP`) und seine Ausgabedatei aus, und was kostet es.

Anwenderwunsch (woertlich): *„Eventuell koennen wir ein Testprogramm bauen, was infos
ausliest und in eine datei schreibt“.* Der Anwender testet mit **einer** USB-Maus und
**einer** Tastatur an einer echten Pi 4; danach steckt ein externer Tester (Ragnar)
verschiedene Maeuse an und schickt die Dateien zurueck. Das Programm und die Dateien
muessen fuer einen Nicht-Entwickler benutzbar sein: klare Bildschirmanweisungen,
selbstbeschreibende Dateien, **kein Sonderbuild** des Kernels (kein Feature `usb-diag`).

## 0. Kennzeichnung der Aussagen

* **[G]** gelesen, mit `Datei:Zeile` belegt (diese Sitzung, `main` bei `03ae70c8`).
  **[S]** Schluss aus dem Gelesenen, nicht beobachtet. **[?]** aus den lokalen Quellen
  nicht entscheidbar. **[E]** Entwurfsentscheidung dieses Dokuments (kein Fakt ueber
  fremden Code, sondern ein Vorschlag, den der Anwender kippen darf).
* **Lokal vorhanden** (`references/`, gitignored): `usb/hid1_11.pdf` (HID 1.11),
  `xhci/xhci-1.2.pdf`. **USB 2.0 ist nicht lokal**; Aussagen dazu sind [S]/[?].
  FreeMiNT (`references/freemint/sys/usb/`) dient nur als **Datenmodell**
  (`usb.h:84-100` `struct usb_device_descriptor`, `:187-229` `struct usb_device`
  mit `speed`, `mf`/`prod`/`serial`, `portnr`, `parent`; `usb_api.h:127-186`
  Funktionszeiger-Tabelle, im EL0-Adressraum nicht uebernehmbar).
* **Vertrag vs. intern** (`docs/COMPATIBILITY.md`): Eigene NewTOS-Modi ab 2000 stehen
  nicht in COMPATIBILITY.md, sondern nur in `154-ssystem.md` [G: „Ssystem“ kommt in
  COMPATIBILITY.md nur einmal vor, Zeile 1043, nicht als Modusliste]. **Vertrag** dieses Entwurfs:
  Modusnummer, Aufrufform, Header, Op-Codes, Feldlayouts, Rueckgabewerte (§2).
  **Intern/frei**: Ringgroessen, Pufferlimits (§2.7, per Layout-Version aenderbar),
  Speicherorte, Encoder-Struktur.

## 1. Was der Kernel heute behaelt und wegwirft [G]

| Datum | Heute | Beleg |
|---|---|---|
| Geraetedeskriptor (18 Byte) | gelesen in `DEV[di].buf`, **ueberschrieben** beim naechsten Transfer; behalten nur `class`, `vid`, `pid` | `xhci/mod.rs:1138-1173` (`:1169-1171`), `buf` `:122` |
| Konfigurationsdeskriptor | 9 Byte, dann `wTotalLength` **gekappt auf 255**, nur gedruckt (`usb-diag`) und von `config_desc::scan` ausgewertet, danach ueberschrieben | `:1232-1263`, Kappung `:1242`, `buf: [u8;256]` `:122` |
| HID-Report-Deskriptor | **nie abgerufen** (`get_descriptor` kennt nur Typ 1 und 2) | `:1117-1134`; `hid-mouse-protocols.md` §1.3 |
| HID-Deskriptor (0x21, `wDescriptorLength`) | im Konfigurationsstrom vorhanden, von `scan` **uebersprungen** (`_ => {}`) | `config_desc.rs:135-203` |
| Interface-Auswahl | `scan` waehlt **nur** das erste HID-Boot-Interface (Klasse 3, Subklasse 1, Protokoll 1/2) mit Interrupt-IN; jedes andere Interface liefert eine Logzeile mit `SkipReason` (F2/F8) | `config_desc.rs:66-80,119-129,135`; Log `xhci/mod.rs:1299-1319`; Text `config_desc.rs:207-217` |
| Geraet **ohne** gewaehltes Boot-Interface | `read_config` gibt `None` → `configure_hid` bricht mit `?` ab: **weder Configure Endpoint noch `SET_CONFIGURATION`, kein Endpoint scharf** | `xhci/mod.rs:1328-1334`, `:1441` |
| Rohe Interrupt-Berichte | `poll_interrupt` liefert `[u8;8]` + Laenge, `poll` dekodiert Protokoll 1/2 und wirft sie weg; Protokoll ≠ 1/2 → `_ => {}` | `xhci/mod.rs:658-716`; `mod.rs:139-157` |
| Berichtspuffer / TRB-Laenge | `intbuf: [u8;8]`, TRB-Laenge fest 8, Laenge = `8 - Rest` | `xhci/mod.rs:123`, `:604`, `:697-698` |
| Pfad, Geschwindigkeit, Slot, Rolle | in `HidDev` vorhanden: `slot`, `dci`, `protocol`, `port`, `vid`, `pid`, `class`, `route`, `speed` (PORTSC-Speed-ID 1 voll, 2 niedrig, 3 hoch, 4 super) | `xhci/mod.rs:167-191`; Zuweisung `:1563-1569`; Hub-Kinder `:1671-1679` |
| Eltern-Hub | **nicht gespeichert** (nur Route-String und TT-Slot) | `xhci/mod.rs:1679-1687` |
| Controller ohne Geraet | `XHCI` wird nur gesetzt, wenn `enumerate_all() > 0`; sonst Controller-Objekt verworfen | `mod.rs:65-68` |
| Kein Controller auf `raspi4b` | `find_xhci` → `None` („RC not present“), `usb::init` kehrt zurueck | `pcie_rpi4.rs:457-468`; `mod.rs:58` |

**Befund, der den Entwurf bestimmt [G/S]:** Seit F2 wird ein **Report-only-Geraet**
(HID-Klasse, Subklasse 0, z. B. QEMU `usb-tablet`, Funkempfaenger-Interfaces, jede Maus,
die nur das Report-Protokoll spricht) **gar nicht mehr konfiguriert**: kein
`SET_CONFIGURATION`, kein Interrupt-Endpoint, kein Transfer. Vor F2 war es nach
`hid-mouse-protocols.md` §1.2 Zeile C konfiguriert und stumm [S]; jetzt ist es nicht
einmal das [G]. Ein Diagnosewerkzeug, das nur Daten aus dem
bestehenden Pfad abgreift, sieht genau die Maeuse **nicht**, wegen derer es gebaut wird.
Der Kern braucht deshalb einen **Beobachtungs-Pfad** (§3.2): Interface konfigurieren und
Berichte aufzeichnen, aber nicht dekodieren.

**Zweiter Befund [G + xHCI 1.2]:** Die TRB-Laenge ist fest 8 Byte (`:604`). Sendet ein
Geraet ein Paket, das groesser ist als die restliche TD-Laenge, meldet der xHC
*Babble Detected Error* und setzt den Endpoint auf **Halted** (xHCI 1.2 §4.10.2.4,
Tabelle bei „DPP exceeds Max Packet Size or remaining TD Transfer Size Error“;
§4.8.3: „A Reset Endpoint Command shall be used to clear the Halt condition“). Der
Fehlerzweig in `poll_interrupt` armiert nur neu (`xhci/mod.rs:700-712`), ohne Reset
Endpoint. Eine Report-Protokoll-Maus mit Berichten > 8 Byte und Max-Packet-Size > 8 ist
damit **vermutlich** dauerhaft stumm [S] — und das Werkzeug wuerde ohne Gegenmassnahme
nur „0 Berichte“ zeigen. Gegenmassnahme in §3.3 (TRB-Laenge = Max-Packet-Size, Zaehler
fuer Interruptfehler).

## 2. Der Modus `S_NEWTOS_USBSNAP` = **2022** (DRAFT)

Nummer: 2022 ist die erste unbeanspruchte der Reihe (2000–2021 belegt:
`tos-api/src/gemdos/ssystem.rs:94` bis 2020, `tos-api/src/bios/reset.rs:100` = 2021).
Der Dispatcher faellt fuer unbekannte Modi auf `EINVFN` (`ssystem.rs:124`) — **so
erkennt ein Programm einen zu alten Kernel**. Ein Test pinnt heute „2022 ist kein Modus“
(`tos-api/tests/gemdos_ssystem_sysinfo.rs`, Kopfkommentar Punkt 7) und muss mit der
Umsetzung auf 2023 wandern.

### 2.1 Aufruf

```
LONG Ssystem(2022, void *buf, LONG len)      /* x0=2022, x1=buf (64 Bit), x2=len (i32) */
```

Die Argumentform folgt dem Muster „String in den Puffer des Aufrufers“ von Modus 2019
(`ssystem.rs:182-209`: `arg1` = Puffer als `arg_u64(1)`, `arg2` = Laenge als
`arg_i32(2)`). Anders als 2019 enthaelt der Puffer **am Anfang eine Anfrage** (32 Byte,
§2.2); der Kernel liest sie, schreibt die Antwort in denselben Puffer. Grund: `Ssystem`
hat nur `mode`, `arg1`, `arg2`; Geraeteindex, Op-Code und Folgenummer passen nicht
zusaetzlich hinein. Alternative (3 Modi 2022/2023/2024 mit Indexkodierung in `arg2`) ist
unnoetig verbogen. [E]

* **Keine Berechtigung noetig** (reine Auskunft, wie 2000–2020). Op 4 (MUTE, §2.6)
  veraendert Eingabeverhalten, ist aber zeitbegrenzt (≤ 60 s) und selbstheilend.
* **Zeiger:** `buf` ist eine Aufrufer-Adresse. Zugriff ueber `args.space.copy_from` /
  `copy_to` (`uaccess.rs:364-387`): ein ungueltiger Zeiger **beendet den aufrufenden
  Prozess** mit Crash-Bericht, er wird **kein** Fehlercode (`uaccess.rs` Modulkommentar,
  `FAULTED` `:55`) — wie bei `S_GETCOOKIE`/2019.
* **Byte-Reihenfolge:** alles **little-endian** (native AArch64, NewTOS-eigener Modus;
  die Big-Endian-Konvention der m68k-TOS-Strukturen gilt hier nicht). Feldbreiten fest,
  keine Strukturen mit Compiler-Padding: das Programm liest Bytes mit Offsets.

### 2.2 Anfrage-/Antwortkopf (32 Byte, v1)

| Off | Gr. | Feld | Anfrage | Antwort |
|---:|---|---|---|---|
| 0 | 4 | `magic` | ASCII `"USBS"` (`55 53 42 53`) | gleich |
| 4 | u16 | `version` | gewuenschte Layout-Version, seit F3 **2** (v1: 1; §10) | benutzte Version (2) |
| 6 | u16 | `hdr_len` | ignoriert | **32** (Offset des Rumpfs) |
| 8 | u16 | `op` | 1 LIST, 2 DEVICE, 3 REPORTS, 4 MUTE | gleich |
| 10 | u16 | `flags` | ignoriert | Bit 0 `TRUNCATED`, Bit 1 `MORE` |
| 12 | u32 | `index` | Geraeteindex (Op 2, 3); sonst 0 | gleich |
| 16 | u32 | `arg` | Op 3: erste gewuenschte Folgenummer; Op 4: Dauer in `_hz_200`-Ticks; sonst 0 | gleich |
| 20 | u32 | `total_len` | ignoriert | Bytes der **vollstaendigen** Antwort inkl. Kopf |
| 24 | u32 | `used_len` | ignoriert | Bytes, die der Kernel in den Puffer geschrieben hat, inkl. Kopf |
| 28 | u32 | `now_hz200` | ignoriert | `_hz_200` zum Zeitpunkt des Aufrufs (`tos_api::bios::tick::hz_200()`, `tick.rs:92`) |

`now_hz200` gibt dem Programm eine Uhr ohne eigene Zeitfunktion: GEMDOS kennt
`_hz_200` nur ueber Supervisor-Zugriff, `Tgettime` ist 2-s-aufgeloest. Ein
Aufzeichnungsfenster misst das Programm in Ticks aus dieser Spalte (5 ms je Tick).

### 2.3 Rueckgabewerte und Fehlerfaelle

Rueckgabewerte mit den Konstanten aus `tos-types/src/error.rs` (`ERROR` `:21`, `EINVFN`
`:39`, `EINVAL` `:62`). Pruefreihenfolge wie dargestellt; die erste zutreffende gewinnt.

| # | Bedingung | Rueckgabe | Wirkung |
|---|---|---|---|
| 1 | `buf == 0` oder `len < 32` | `EINVAL` (-25) | nichts geschrieben |
| 2 | `magic` falsch | `EINVAL` | nichts geschrieben |
| 3 | `version != 2` (v1: `!= 1`; §10) | `EINVAL` | Kopf mit `version = 2`, `used_len = 32` geschrieben (Programm erfaehrt die unterstuetzte Version) — die Regel von `newtos_scalar` („ein spaeteres Programm erfaehrt, dass es mit einem aelteren Kernel spricht“, `ssystem.rs:301-304`) |
| 4 | `op` unbekannt | `EINVAL` | wie 3 |
| 5 | kein nutzbarer USB-Controller (Host ohne USB, Board ohne xHCI, Controller nicht gefunden, hostlos im Test) | `ERROR` (-1) | nichts geschrieben; **die Regel der Modi 2000–2020** („Board kann es nicht messen → `ERROR`“, `154-ssystem.md` §Interface) |
| 6 | `index >= n_devices` (Op 2, 3) | `EINVAL` | wie 3 (Kopf mit `used_len = 32`) |
| 7 | Op 4 mit `arg > 12000` oder `index != 0` | `EINVAL` | wie 3 |
| 8 | Antwort passt nicht in `len` | **`E_OK` (0)** | siehe §2.5 |
| 9 | sonst | `E_OK` (0) | Antwort geschrieben, `used_len` ≤ `len` |

* **Kein Controller vs. Controller ohne Geraet:** `raspi4b` (kein PCIe-Root-Complex,
  `pcie_rpi4.rs:465-468`) und jeder Boot ohne gefundenen Controller → `ERROR`.
  Controller gefunden, aber **kein** Geraet gesteckt → `E_OK` mit `n_devices = 0`. Das
  verlangt eine Kernel-Aenderung (§5, K5): heute wird das Controller-Objekt in diesem
  Fall verworfen (`mod.rs:65-68`), beides waere nicht unterscheidbar.
* **Zu alter Kernel:** `EINVFN` (-32) — Folge des Dispatchers, nicht dieses Modus.
* Wahl `E_OK` statt eines „Puffer zu klein“-Codes (§2.5): Praezedenz 2019 (Abschneiden
  ist `E_OK`, `ssystem.rs:182-209`) und kein neuer, in TOS schwach belegter Fehlercode
  (`ERANGE` = -64 heisst im TOS „Seek out of range“, `error.rs:52`). [E]

### 2.4 Op 1 `LIST` — Geraeteliste

Rumpf = Listenkopf (16 Byte) + `n_devices` × `entry_len` Byte. Die Liste enthaelt **jeden
belegten Geraeteslot** (`hid[0..n_hid]`, `xhci/mod.rs:332`), auch Hubs (Rolle 4) und
fehlgeschlagene Enumerationen (Rolle 5); das Programm filtert fuer die Anzeige.

**Listenkopf**

| Off | Gr. | Feld |
|---:|---|---|
| 0 | u32 | `generation` — v1: 1, nach der Enumeration konstant; **reserviert fuer Hotplug (F6)**, damit Clients Neuenumeration erkennen |
| 4 | u16 | `n_devices` |
| 6 | u16 | `entry_len` (v1: **32**; Clients muessen mit `entry_len` weiterschalten, nicht mit 32) |
| 8 | u8 | `root_ports` (`Xhci::root_ports`, `xhci/mod.rs:1224`) |
| 9 | u8 | `controller_state` (v1: 1 = xHCI laeuft) |
| 10 | u16 | `enum_skipped` — Zahl der F8-Ereignisse „Geraet nicht enumeriert“ (Budget `MAX_HID` an den drei Stellen `xhci/mod.rs:1481`, `:1546`, `:1634`; `enable_slot` `:1556`; Root-Port-Reset `:1490`; Address Device `:1587`) |
| 12 | u32 | reserviert, 0 |

**Eintrag (32 Byte)** — auch der Kopf von Op 2

| Off | Gr. | Feld | Quelle |
|---:|---|---|---|
| 0 | u8 | `index` (0-basiert = Position in `hid[]`; Schluessel fuer Op 2/3) | `HidDev`-Index `di` |
| 1 | u8 | `slot` (xHCI-Slot-ID) | `hid[di].slot` `:1564` |
| 2 | u8 | `root_port` (1-basiert) | `:1565` |
| 3 | u8 | `parent` — Index des Hub-Eintrags, `0xFF` = am Root-Port | **neu**, K4 |
| 4 | u32 | `route` — xHCI-Route-String, 4 Bit je Hub-Stufe, Stufe 0 = unterstes Nibble | `:1566`, Aufbau `:1679` |
| 8 | u16 | `vid` | `:1170` |
| 10 | u16 | `pid` | `:1171` |
| 12 | u8 | `dev_class` (`bDeviceClass`) | `:1169` |
| 13 | u8 | `speed`: PORTSC-Speed-ID **1 voll, 2 niedrig, 3 hoch, 4 super** (nicht die Zaehlung von `RootHubStatus`, `mod.rs:240`) | `:1567`; `ep0_max_packet` `:248` |
| 14 | u8 | `role` — siehe unten | **neu** |
| 15 | u8 | `reason` — siehe unten | **neu** |
| 16 | u8 | `flags`: Bit 0 Geraetedeskriptor gueltig, 1 Konfiguration gueltig, 2 Konfiguration gekappt, 3 `SET_CONFIGURATION` gesendet | **neu** |
| 17 | u8 | `n_hid_ifaces` (Interfaces der Klasse 3, alt 0) | aus `scan`, K2 |
| 18 | u8 | `ring_iface` — Interfacenummer, deren Interrupt-Endpoint den Berichtsring fuellt; `0xFF` = kein Ring. **v2:** die der *primaeren* Funktion (Funktion 0); alle Funktionen fuellen denselben Ring (§10) | K3 |
| 19 | u8 | `ring_ep` — Endpoint-Adresse (Bit 7 = IN), 0 = keiner | `HidInfo.ep_addr` `config_desc.rs:49` |
| 20 | u32 | `ring_oldest_seq` | K1 |
| 24 | u32 | `ring_next_seq` | K1 |
| 28 | u16 | `int_errors` — Zahl der Interrupt-Transfers mit Completion-Code ≠ Success/ShortPacket | K1, Fehlerzweig `:700-712` |
| 30 | u8 | `last_cc` — letzter solcher xHCI-Completion-Code (Zahl, xHCI 1.2 §6.4.5 Tabelle 6-90, z. B. 3 Babble, 6 Stall; 0 = keiner) | K1 |
| 31 | u8 | v1: reserviert, 0. **v2:** `n_funcs` — Zahl der Funktionen (Interfaces, deren Endpoint den Ring fuellt; Interface-Zeilen mit Byte 14 = 1), §10 |

**`role`:** 0 = keine (enumeriert, aber kein Eingabegeraet / nicht konfiguriert);
1 = Tastatur (Boot, wird dekodiert → `hid::keyboard_report`); 2 = Maus (Boot, wird
dekodiert → `hid::mouse_report`); 3 = **nur beobachtet** (HID-Interface konfiguriert,
Berichte aufgezeichnet, **nicht dekodiert** — §3.2); 4 = Hub; 5 = Enumeration
fehlgeschlagen (Slot belegt, Geraet unbrauchbar).
Die Rollen 1/2/4 entsprechen `HidDev.protocol` 1/2/9 (`xhci/mod.rs:171`, `:1592`).

**`reason`** (0 = „benutzt“; sonst warum **kein** Treiber das Geraet nimmt, oder warum
nur beobachtet). Codes 1–5 = `config_desc::SkipReason` + 1, **dieselben Gruende wie die
F2/F8-Logzeilen** (`config_desc.rs:66-80`, Texte `:207-217`):

| Code | Bedeutung | Logtext heute |
|---:|---|---|
| 0 | benutzt (Rolle 1/2/4) | — |
| 1 | `NotHid` — Interface-Klasse ≠ 3 | „not a HID interface (class != 3)“ |
| 2 | `NotBootSubclass` — HID, Subklasse ≠ 1 (nur Report-Protokoll). Bei Rolle 3: „beobachtet, nicht dekodiert“ | „HID without boot subclass: report protocol only, no report parser yet“ |
| 3 | `UnknownBootProtocol` — Boot-Subklasse, Protokoll weder 1 noch 2 | „HID boot subclass, protocol neither keyboard nor mouse“ |
| 4 | `NoInterruptIn` | „HID boot interface without an interrupt-IN endpoint“ |
| 5 | v1: `OneInterfacePerDevice`. **v2: reserviert, nicht mehr vergeben** (§10) | v1: „second HID boot interface: one interface per device“ |
| 6 | v2: `FunctionLimit` — Boot-Interface ueber der Funktionsgrenze (je Geraet 4, insgesamt 12; `hid-mouse-protocols.md` §9.3 R6) | „function limit“ |
| 7 | v2: `AlternateSetting` — Boot-Interface in Alternate Setting ≠ 0 (§9.3 R1) | „HID boot interface in an alternate setting (only alternate setting 0 is used)“ |
| 8 | v2: `EndpointCollision` — Boot-Interface auf dem Endpoint (DCI) einer frueheren Funktion (§9.3 R7) | „HID boot interface on an endpoint an earlier function already uses“ |
| 16 | Geraet hat **kein** Interface der Klasse 3 (`xhci/mod.rs:1329-1333`) | „no HID boot keyboard/mouse interface — not an input device“ |
| 17 | Konfigurationsdeskriptor nicht lesbar (`:1234-1236`, `:1253-1259`) | „configuration … read failed“ |
| 18 | Configure Endpoint fehlgeschlagen (`:1447-1449`) | „configure_endpoint → …“ |
| 19 | `SET_CONFIGURATION` fehlgeschlagen (`:1450-1453`) | „set_configuration failed“ |
| 20 | `SET_PROTOCOL(boot)` fehlgeschlagen (`:1454-1457`) | „set_protocol(boot) failed“ |
| 21 | Address Device fehlgeschlagen (`:1587-1589`) | „address_device slot … →“ |
| 22 | Geraetedeskriptor nicht lesbar (`:1144-1162`) | „get device descriptor … failed“ |

Das Geraet-`reason` ist der **erste** Grund in Deskriptor-Reihenfolge, der verhinderte,
dass das Geraet Rolle 1/2 bekam; die vollstaendige Aufschluesselung je Interface steht
in der Interface-Tabelle von Op 2.

### 2.5 Op 2 `DEVICE` — ein Geraet im Detail

Rumpf = **Eintrag** (32 Byte, wie §2.4) + `u16 n_sections` + `u16 reserved` + Abschnitte.
Abschnittskopf (8 Byte): `u16 type`, `u8 iface`, `u8 flags`, `u32 len`; danach `len`
Nutzbytes, aufgefuellt auf ein Vielfaches von 4 (Fuellbytes zaehlen nicht in `len`).
Abschnitt-`flags`: Bit 0 `TRUNCATED` (Limit erreicht), Bit 1 `FAILED` (Abruf
fehlgeschlagen, `len = 0`), Bit 2 `SKIPPED` (nicht versucht, `len = 0`; Gruende §3.1).

| Typ | Inhalt | `iface` | Limit |
|---:|---|---|---|
| 1 | **Geraetedeskriptor roh**, 18 Byte | 0 | — |
| 2 | **Konfigurationsdeskriptor roh**, wie vom Geraet geliefert (Konfiguration Index 0) | 0 | 1024 Byte; darueber `TRUNCATED` — die echte Gesamtlaenge steht in Byte 2–3 des Rohdeskriptors |
| 4 | **Interface-Tabelle**, je 16 Byte (unten), eine Zeile je Interface-Deskriptor in Reihenfolge, auch Alt ≠ 0 | 0 | ≤ 16 Zeilen |
| 3 | **HID-Report-Deskriptor roh**, einer je HID-Interface (alt 0), in Interface-Reihenfolge | Interfacenummer | **2048 Byte** je Interface, **4 Interfaces** je Geraet; Rest `TRUNCATED`/`SKIPPED` |
| 5 | Herstellerstring (ASCII, ohne NUL) | 0 | 31 Zeichen |
| 6 | Produktstring (ASCII, ohne NUL) | 0 | 31 Zeichen |

Reihenfolge im Strom: 1, 2, 4, 3 (je Interface), 5, 6. Fehlende Abschnitte (kein
Konfigurationsdeskriptor lesbar) fehlen einfach; `n_sections` zaehlt vorhandene.
Seriennummer wird **bewusst nicht** erfasst (Datenschutz: die Dateien gehen an Dritte).

**Interface-Tabelle, Zeile (16 Byte)**

| Off | Gr. | Feld |
|---:|---|---|
| 0 | u8 | `bInterfaceNumber` |
| 1 | u8 | `bAlternateSetting` |
| 2 | u8 | `bInterfaceClass` |
| 3 | u8 | `bInterfaceSubClass` |
| 4 | u8 | `bInterfaceProtocol` |
| 5 | u8 | `reason` (Tabelle §2.4; 0 = dieses Interface speist Treiber/Ring) |
| 6 | u8 | Adresse des ersten Interrupt-IN-Endpoints dieses Interfaces, 0 = keiner |
| 7 | u8 | dessen `bInterval` |
| 8 | u16 | dessen `wMaxPacketSize` (roh, inkl. Bits 12:11) |
| 10 | u16 | `wDescriptorLength` des Report-Deskriptors laut HID-Deskriptor (HID 1.11 §6.2.1, Offset 7/2), 0 = kein HID-Deskriptor gefunden |
| 12 | u16 | davon abgelegte Bytes (Abschnitt 3) |
| 14 | u8 | 1 = dieses Interface ist das ueberwachte (`ring_iface`). **v2:** 1 = dieses Interface ist eine Funktion, sein Endpoint fuellt den Ring (mehrere Zeilen moeglich) |
| 15 | u8 | v1: reserviert, 0. **v2:** Rolle der Funktion dieses Interfaces (1 Tastatur, 2 Maus, 3 nur beobachtet; 0 = keine Funktion), §10 |

**Zu kleiner Puffer (alle Ops):** Passt die **vollstaendige** Antwort nicht in `len`,
schreibt der Kernel nur den 32-Byte-Kopf (`used_len = 32`, `total_len` = benoetigte
Groesse, `flags.TRUNCATED`) und gibt `E_OK` zurueck. Das Programm wiederholt den Aufruf
mit `total_len` Byte. Es gibt **keine** Teilantwort fuer Op 1/2: ein abgeschnittenes
Binaerformat ist schlimmer als keines. Obergrenze Op 2 in v1: 32 + 32 + 8 + 18 + 8 +
1024 + 8 + 256 + 4·(8 + 2048) + 2·(8 + 32) ≈ **9,7 KiB**; das Programm legt 16 KiB an
und braucht die Wiederholung nur bei kuenftigen Versionen.

### 2.6 Op 3 `REPORTS` — Berichtsring lesen; Op 4 `MUTE`

**Der Ring** ist je Geraet (nur mit `ring_iface != 0xFF`) ein Umlaufpuffer aus
**256 Eintraegen** zu 80 Byte, gefuellt in `usb::poll()` (§3.3). Jeder Bericht bekommt
eine **Folgenummer** `seq` (u32, je Geraet bei 0 beginnend, monoton). Der Ring wird nie
geloescht und nicht „geleert“: das Programm liest *ab einer Folgenummer*. Ein
Lösch-Op wurde bewusst weggelassen — er verlaeuft gegen einen zweiten Leser (spaetere
Werkzeuge, die Maus-Einstellung) und die Folgenummer macht ihn unnoetig. [E]

Anfrage: `index` = Geraet, `arg` = erste gewuenschte `seq`. Rumpf der Antwort:

| Off | Gr. | Feld |
|---:|---|---|
| 0 | u32 | `oldest_seq` — aeltester noch im Ring liegender Bericht |
| 4 | u32 | `next_seq` — die `seq` des naechsten kommenden Berichts (= Anzahl bisher aufgezeichneter) |
| 8 | u32 | `lost` — wie viele der ab `arg` gewuenschten Berichte schon ueberschrieben sind (`max(0, oldest_seq - arg)`) |
| 12 | u16 | `count` — Zahl der folgenden Eintraege |
| 14 | u16 | `rec_len` (v1: **80**) |

Danach `count` Eintraege ab `max(arg, oldest_seq)` in aufsteigender `seq`:

| Off | Gr. | Feld |
|---:|---|---|
| 0 | u32 | `seq` |
| 4 | u32 | `ts_hz200` — `_hz_200` beim **Abholen durch den Host** (siehe Hinweis) |
| 8 | u16 | `len` — gueltige Bytes in `data`, 0–64 |
| 10 | u8 | `flags`: Bit 0 gekappt (Geraet sendete mehr als 64 Byte), Bit 1 waehrend MUTE abgeholt |
| 11 | u8 | `cc` — xHCI-Completion-Code des Transfers (1 Success, 13 Short Packet; xHCI 1.2 §6.4.5 Tabelle 6-90) |
| 12 | u32 | v1: reserviert, 0. **v2:** +12 u8 `iface` (Interfacenummer der Funktion, von der der Bericht stammt), +13 u8 `ep` (deren Endpoint-Adresse), +14 u16 reserviert, 0 (§10) |
| 16 | u8[64] | `data`; Bytes ab `len` sind 0 |

* `arg >= next_seq` → `count = 0`, kein Fehler (der Standard-Wartezustand).
* Passen nicht alle verfuegbaren Eintraege in `len`, liefert der Kernel **ganze
  Eintraege**, soviele passen, und setzt `flags.MORE`; `total_len` ist die Groesse aller
  verfuegbaren. Passt nicht einmal Kopf + Rumpfkopf (48 Byte): `TRUNCATED`, nur Kopf.
* Geraet ohne Ring: `oldest_seq = next_seq = lost = count = 0`, `E_OK`.
* **Hinweis zur Zeit:** `ts_hz200` ist der Moment, in dem `poll()` den Bericht aus dem
  Event-Ring holt, **nicht** der Moment, in dem das Geraet ihn sendete. Der Takt der
  `poll()`-Aufrufe (Timer-IRQ-Pump 200 Hz plus Trap-seitige Pumps, `host_env.rs:20-60`,
  `:460-540`) bestimmt die Aufloesung. Die Datei sagt das (§4.4).
* **Abholrate:** `poll_interrupt` liefert **einen** Bericht je Aufruf (`mod.rs:139`).
  Ein 1-kHz-Mausmodell (z. B. die Roccat: `bInterval 1`, Log `run3…:94`) wird also nur so
  schnell aufgezeichnet, wie `poll()` laeuft. Das Programm treibt `poll()` mit einer
  `Cconis`-Schleife (§4.2); fehlende Berichte erscheinen als Luecke in der Zeitspalte.
  Ob der Controller Berichte zwischen zwei Pumps akkumuliert, ist **[?]**.

**Op 4 `MUTE`** (empfohlen, **kippbar**, §7 Frage 1): `arg` = Dauer in Ticks (1…12000,
also ≤ 60 s), 0 = aus. Solange gemuted: `poll()` zeichnet auf und re-armiert wie
gewohnt, **dekodiert aber nicht** (`ikbd::feed` entfaellt). Beim **Beginn** eines Mute laesst
der Dekodierer einmal alle noch gehaltenen Tasten und Maustasten los (§9.1: sonst verliert
eine vor dem Mute gedrueckte Taste ihr Loslassen an den Mute und wiederholt sich durch).
Zweck: waehrend der
Aufnahme sollen Mausklicks nicht den Desktop erreichen. Ob liegengebliebene Klicks
nach Programmende am Desktop wirken, ist **[?]** (nicht untersucht; deshalb Test T6;
QEMU-Ergebnis §9.1).
Der Kernel hebt das Muten nach Ablauf der Dauer **selbst** auf (Frist als `_hz_200`-Wert,
wrap-sicher verglichen): ein abgestuerztes Programm kann die Eingabe nicht dauerhaft
sperren. Antwort-Rumpf: `u32 muted_until` (0 = aus), `u32 reserved`.

### 2.7 v1-Grenzen (intern, per Layout-Version aenderbar)

| Grenze | Wert | Warum |
|---|---:|---|
| Geraete | `MAX_HID` = 8 | `xhci/mod.rs:98` |
| Konfigurationsdeskriptor | 1024 Byte | heute 255 (`:1242`); Hub-Empfaenger und Verbundgeraete sind groesser; Roccat 59 Byte (`run3…:90`) |
| Report-Deskriptor | 2048 Byte × max. 4 Interfaces | `wDescriptorLength` ist u16 (HID 1.11 §6.2.1), reale Maus-Deskriptoren sind hundert bis wenige hundert Byte [S] |
| Bericht je Eintrag | 64 Byte | volle Low/Full-Speed-Interrupt-Paketgroesse; HS-Endpoints bis 1024 sind fuer HID selten [S] |
| Ring je Geraet | 256 Eintraege × 80 Byte = 20 KiB | bei 8 Geraeten 160 KiB `.bss` (Praezedenz: `bootlog` 192 KiB, `bootlog.rs:27`) |
| Strings | 31 Zeichen, ASCII | Nicht-ASCII-Codeeinheiten werden `?` |

### 2.8 Nebenlaeufigkeit mit dem 200-Hz-IRQ-Pump [G]

* **Der Ssystem-Handler laeuft im synchronen SVC mit maskierten IRQs** (BDOS/AES/VDI-
  Ausschluss beruht darauf: `input_gate.rs` Modulkommentar, Abschnitt „The interrupt was
  taken from EL0“; `host_env.rs:20-35`). Der Timer-IRQ-Pump laeuft **nur aus EL0** und
  nie waehrend eines Traps (`irq_input_pump`: `if !from_el0 { return }`,
  `host_env.rs:505-513`). Ein Lesen des Rings im Handler kann daher **nicht** mit einem
  IRQ-seitigen Schreiben verschraenken.
* **Der Trap-seitige Pump** (`pump_input`, `host_env.rs:91-`) oeffnet IRQs kurz
  (`daifclr #2`, `:98-110`), aber ein dann eintreffender Timer-IRQ kommt aus EL1 und
  pumpt nie. `PUMP.try_enter()` (`host_env.rs:443`, `input_gate.rs:247`) verhindert
  zusaetzlich jede Doppel-Betretung.
* **Einzige Ausnahme: ein EL0-Upcall mitten in `poll()`.** `usb::poll()` ruft
  `ikbd::feed`, das einen `kbdvec`-Hook im EL0 aufrufen kann; ein Hook koennte
  `Ssystem(2022)` rufen, waehrend `poll()` darunter suspendiert ist. Deshalb **Pflicht-
  Reihenfolge in `poll()`**: *zuerst* den Bericht vollstaendig im Ring ablegen
  (Daten, dann `next_seq` zuletzt erhoehen), *dann* dekodieren. [E, verlangt von K1]
* **Der Schnappschuss-Speicher ist ein eigenes `static`**, getrennt von `XHCI`. Der
  Handler fasst `XHCI` nie an (kein zweiter `&mut` auf das gerade suspendierte Objekt,
  `mod.rs:41`, `:136`); alles, was LIST/DEVICE brauchen, wird bei der Enumeration dorthin
  kopiert (`root_ports`, `generation`, `enum_skipped`, Geraete).
* Enumeration selbst (Deskriptoren, Report-Deskriptor, Strings) laeuft **einmalig im
  Bootpfad** (`lib.rs:430`) vor dem ersten EL0-Programm — keine Wiedereintrittsfrage.

## 3. Wann die Daten entstehen (Kern, `hid-mouse-protocols.md` §5.1)

### 3.1 Ablauf je Geraet in `enumerate_device` (`xhci/mod.rs:1537-1605`)

1. `enable_slot`, `address_device` — unveraendert. Schlaegt etwas fehl: Eintrag mit
   Rolle 5 und `reason` 21/22 (heute bleibt `hid[di]` belegt, `:1562-1570`, aber leer).
2. `get_device_descriptor` (`:1138-1173`): nach dem 18-Byte-Lesen (`:1159`) die Rohbytes
   in den Schnappschuss kopieren (K2).
3. **Hub** (`class == 9`, `:1591`): Rolle 4; Konfiguration wird wie heute nur auf
   `bConfigurationValue` gelesen (`read_config_value` `:1342`) — **kein** Konfigurations-
   Abschnitt. `enumerate_hub` setzt `parent` der Kinder (K4).
4. **Nicht-Hub:** `read_config` liest die Konfiguration mit Limit **1024** (heute 255) in
   einen gemeinsamen Enumerationspuffer (§5, K2) und kopiert sie in den Schnappschuss.
   `config_desc::scan` wird erweitert: je Interface eine Zeile der Interface-Tabelle,
   dazu aus dem HID-Deskriptor (Typ 0x21, direkt nach dem Interface-Deskriptor; HID 1.11
   §7.1: Reihenfolge Konfiguration → Interface → HID-Deskriptor → Endpoint) das
   `wDescriptorLength` des ersten Report-Deskriptor-Eintrags (Typ 0x22).
5. **Wahl:** (i) wie heute das erste Boot-Interface → Rolle 1/2; sonst (ii) das **erste
   Interface der Klasse 3 mit Interrupt-IN** → Rolle 3 (Beobachtung, §3.2); sonst (iii)
   keins → Rolle 0, `reason` 16, **kein** `SET_CONFIGURATION` (Verhalten wie heute).
6. `configure_hid` fuer (i)/(ii): Configure Endpoint → `SET_CONFIGURATION` →
   `SET_PROTOCOL(boot)` **nur bei (i)** (HID 1.11 §7.2.6: nur Geraete der
   Boot-Subklasse unterstuetzen es; `hid-mouse-protocols.md` §1.3). Reihenfolge Configure
   Endpoint vor `SET_CONFIGURATION` bleibt (`:1442-1445`).
7. **Nach erfolgreichem Schritt 6, als letzter Schritt von `enumerate_device`:**
   a. Je HID-Interface (alt 0, höchstens 4) den **Report-Deskriptor** holen:
      `bmRequestType 0x81`, `bRequest 6`, `wValue 0x2200`, `wIndex` = Interfacenummer,
      `wLength = min(wDescriptorLength, 2048)` (HID 1.11 §7.1.1; die Anfrage ist eine
      Interface-Anfrage und braucht den Zustand *Configured* — daher **nach**
      `SET_CONFIGURATION` [S, USB 2.0 nicht lokal]). Im Gegensatz zu `get_descriptor`
      (`:1117`) geht das ueber den gemeinsamen Puffer (`control` nimmt eine beliebige
      Pufferadresse, `:1063`).
   b. Optional (Frage 2): `GET_DESCRIPTOR(String, 0)` fuer die erste LANGID, dann
      Hersteller- und Produktstring (`iManufacturer`/`iProduct` aus Byte 14/15 des
      Geraetedeskriptors). **Best effort**: Fehler ignorieren, Abschnitt faellt weg.
8. Pro fehlgeschlagenem Abruf in 7 eine Logzeile (F8-Regel); `FAILED` im Abschnitt.

**Warum Report-/String-Abruf zuletzt:** Ein STALL auf einem Control-Endpoint setzt den
Endpoint auf *Halted* (xHCI 1.2 §4.8.3: „A STALL detected on any stage … of a Default
Control Endpoint request shall transition the Endpoint Context to the Halted state“;
§4.10.2.1) und es gibt heute keinen Reset-Endpoint-Pfad. Laeuft der riskante Abruf als
Letztes, kann ein STALL keinen folgenden `SET_PROTOCOL` oder Hub-Zugriff mehr treffen;
EP0 des Slots wird danach in v1 nicht mehr benutzt. **Randbedingung fuer F6 (Hotplug):**
dort braucht EP0 einen Reset-Endpoint-Pfad. (`wait_transfer` meldet beim STALL `false`
sobald das Event kommt, `:1014-1058`; bei stummem Geraet bleibt es beim 50-Mio-
Spin-Timeout `:1028`.)

### 3.2 Der Beobachtungs-Pfad (Rolle 3)

Ein Interface, das heute abgelehnt wird (`NotBootSubclass`, `UnknownBootProtocol`),
bekommt Configure Endpoint, `SET_CONFIGURATION` und einen armierten Interrupt-IN-Endpoint,
aber **keinen** `SET_PROTOCOL` und **keine Dekodierung**: `poll()` hat fuer
`protocol ∉ {1,2}` bereits den Zweig `_ => {}` (`mod.rs:152`) und re-armiert danach
(`:156`). `HidDev.protocol` bleibt 0. Damit entsteht ein Ring ohne jede Wirkung auf
IKBD/Cursor. **Verhaltensaenderung fuer alle Builds** (auch ohne Diagnose): bisher
unbeachtete Report-only-Geraete werden konfiguriert und liefern Events. Risiko: ein
exotisches Geraet, dessen Configure Endpoint fehlschlaegt, verbraucht Befehlsring-Budget
(F1 loest die 15er-Grenze) und einen `MAX_HID`-Eintrag (verbraucht ihn heute ohnehin,
`:1562-1570`). Fehlschlag bleibt wie heute nicht-fatal (`configure_hid` → `None`).
[S — fuer echte VL805-Hardware nicht erprobt; Abnahme T7.]

### 3.3 Ring, Zaehler, Berichtspuffer in `poll`

* **Puffer:** `intbuf` 8 → **64 Byte** (`:123`), TRB-Laenge = `clamp(wMaxPacketSize & 0x7FF,
  8, 64)` statt fester 8 (`:604`). Begruendung: ein Paket, das die TD-Restlaenge
  uebersteigt, ist *Babble* und halt den Endpoint (xHCI 1.2 §4.10.2.4); mit TRB-Laenge
  ≥ Max-Packet-Size kann ein Paket nie groesser sein als die TD. Fuer Max-Packet ≤ 8
  (QEMU-Maus: 4, `config_desc.rs` Test) aendert sich nichts (8 bleibt). Die Laenge ist
  `trb_len - Rest` (`:697` bisher mit literal 8). Ist `wMaxPacketSize & 0x7FF > 64`,
  setzt der Eintrag `flags.Bit0` (Bericht evtl. gekappt).
* **`poll_interrupt`** (`:658`) gibt die Berichte als Slice (≤ 64 Byte) + Laenge + `cc`
  zurueck; `hid::keyboard_report`/`mouse_report` bekommen weiter die ersten 8 Byte
  (`hid.rs:69`, `:110`).
* **`poll` (`mod.rs:132`)**: `record(di, bytes, cc, now)` *vor* dem `match` (§2.8);
  `if muted() { skip decode }`; `queue_interrupt`. Im Fehlerzweig (`xhci/mod.rs:700-712`)
  `int_errors += 1; last_cc = code` — **ohne** Ring-Eintrag.
* **Reset Endpoint** nach Halt (z. B. Babble) ist **nicht** Teil dieser Spec; das
  Werkzeug *zeigt* `int_errors`/`last_cc`, damit der Defekt in der Datei sichtbar wird
  (siehe §8).

## 4. Das Programm `USBSNAP`

### 4.1 Form

* **Name:** `USBSNAP.NCP` (8.3). Native NewTOS-Konsolenprogramm (ADR-018), freestanding
  ohne libc wie `sdk/examples/sysinfo.c` (dort `gemdos3`-Wrapper, `napp`/`sapp`-Helfer,
  `Cconws`); gebaut mit `sdk/build.sh sdk/examples/usbsnap.c <ziel>/USBSNAP.NCP`.
  Quelle: **`sdk/examples/usbsnap.c`**. (`hid-mouse-protocols.md` §5.2 nannte
  `sdk/apps/`; dort liegen Ports grosser Fremdprogramme, `sdk/examples/` die
  Kernel-Beweisprogramme wie `sysinfo.c` — dieses hier ist eines.)
* **Start:** per Doppelklick auf `C:\USBSNAP.NCP` im Desktop (so starten `.NCP` heute,
  `docs/sdk/03-first-programs.md` Schritt 1). **Nicht in `\AUTO\`** (laeuft dort vor dem
  AES ohne Rueckmeldung, Anwender-Regel „AUTO ist konsolenonly“). Aufs Image bringt es
  `tools/make-sd-image.sh` (QEMU-Abbild, Zeile 269 als Vorbild fuer `SYSINFO.NCP`); die
  **echte Karte** wird nur auf Ansage des Anwenders beschrieben.
* **Sprache der Bildschirmtexte:** Englisch, alle Texte in einer Tabelle am Programmanfang.
* **Benoetigter Kernel:** einer, der Modus 2022 enthaelt. Das ist der normale
  Release-Kernel — **kein** `usb-diag`-Feature —, aber **nicht der aktuelle**:
  Ragnar braucht `kernel8.img` und `USBSNAP.NCP` zusammen (Frage 5).

### 4.2 Ablauf und Dialog

```
USBSNAP - USB device recorder              NewTOS 0.6.1 2026-10-23 (abcdef0)
Writes one text file per USB device to C:\USB\ - send them to the developer.

Found 3 USB devices:
  1  046d:c077  Mouse        root port 1
  2  04d9:0169  Keyboard     root port 1 via hub 2109:3431 port 2
  3  1e7d:2cee  Mouse        root port 1 via hub 2109:3431 port 1
 (hubs are not listed as devices)

Recording will run for 30 seconds.
  - MOUSE : move it, press EVERY button, turn the wheel.
  - KEYBOARD: press a few keys, also Shift/Ctrl/Alt.
The pointer will NOT move during the recording - that is normal.
Press RETURN to start, or wait 15 seconds.
```
Danach ein Zaehler pro Sekunde, je Geraet eine Zeile (VT52-Cursoradressierung `ESC Y`, die
`sdk/examples/vt52pi.c:23` testet):

```
 Recording...  time left: 22 s
  1  046d:c077  Mouse        reports: 143
  2  04d9:0169  Keyboard     reports: 12
  3  1e7d:2cee  Mouse        reports: 0   <- nothing received yet
```
Ende:
```
Done. Wrote:
  C:\USB\046DC077.TXT   (3.1 KB)
  C:\USB\04D90169.TXT   (1.2 KB)
  C:\USB\1E7D2CEE.TXT   (2.7 KB)
Copy these files from the SD card and send them. Press any key to exit.
```

Ablauf technisch:

1. `Ssystem(2019, buf, 96)` fuer die Versionszeile. `Ssystem(2022)` Op 1 mit 16-KiB-Puffer.
2. `EINVFN` → „This kernel is too old (no USB snapshot call).“ `ERROR` → „No USB controller
   in this system. Nothing to record.“ (Normalfall `raspi4b`.) `n_devices == 0` → „No USB
   device was detected at boot. Plug the device in and RESTART the machine; devices
   plugged in later are not seen yet.“ (Hotplug fehlt bis F6.) Danach „press any key“,
   Ende ohne Datei.
3. Geraete anzeigen, Start abwarten (RETURN oder 15 s; `Cconis` + `Cnecin`, Zeit aus
   `now_hz200`).
4. Je Geraet Op 2 holen. `Ssystem(2022)` Op 4 MUTE mit `30 s + 5 s`.
5. **Aufnahmeschleife** fuer 30 s (6000 Ticks): `Cconis()` (pumpt USB; `host_env.rs:20-60`:
   Bconstat/Cconis pumpen, solange die Tastenwarteschlange leer ist) — **je Durchlauf**;
   ca. alle 50 ms (10 Ticks) Op 3 je Geraet mit `arg = next_seq` des letzten Laufs,
   Eintraege in einen Programmspeicher ablegen; jede Sekunde die Zaehlerzeilen neu.
   Gemutete Tastatur liefert dem Programm keine Tasten; ein Abbruch per Taste gibt es
   darum **nicht** — nur die Zeit. (Ohne Mute: `Cconis`-Treffer mit `Cnecin` verwerfen.)
6. MUTE aus (`arg 0`). Zum Schluss ein letzter Op-3-Durchlauf.
7. Dateien schreiben (§4.3), Dateiliste ausgeben.

Speicher: je Geraet bis **1024** Berichte im Programm (80 Byte → 80 KiB × 8 = 640 KiB
statisches BSS); darueber hinaus werden nur Anzahl, kuerzeste/laengste Laenge und die
Zeit des letzten gezaehlt und im Dateikopf vermerkt („first 1024 of N listed“).

### 4.3 Ausgabedateien

* **Eine Datei je Nicht-Hub-Geraet** (Rollen 0, 1, 2, 3, 5). Hubs erscheinen nur in der
  `Path`-Zeile der Kinder.
* **Pfad:** `C:\USB\VVVVPPPP.TXT` (4 Hex-Ziffern VID + 4 PID, Grossbuchstaben; z. B.
  `C:\USB\046DC077.TXT`). Fehlt `C:\USB`, legt das Programm den Ordner mit `Dcreate`
  an; schlaegt das fehl, Fehlercode anzeigen und abbrechen (Frage 4) — 8.3-sicher, immer genau 8 Zeichen. Unbekannte IDs (Rolle 5, VID/PID
  0): `C:\USB\NOID` + Root-Port (2 Ziffern) + Index (2 Ziffern) `.TXT`, z. B. `NOID0103.TXT`.
* **Kollisionsregel (nie ueberschreiben):** Existenz mit `Fopen(pfad, 0)` pruefen
  (Erfolg → vorhanden, `Fclose`; `EFILNF` -33 → frei). Existiert `VVVVPPPP.TXT`, der
  Reihe nach `.T01`, `.T02`, … `.T99`. Danach: Fehlermeldung, Datei wird uebersprungen.
  Zwei gleiche Maeuse in einem Lauf bekommen so `.TXT` und `.T01`; die Datei nennt
  zusaetzlich `Device index` (`Device 2 of 3`). Anlegen mit `Fcreate(pfad, 0)`
  (`sdk/examples/a112mesg.c:186` als Vorbild); `Fwrite` in Haeppchen; bei negativem
  Rueckgabewert Code + Name (`EACCDN` -36, `EWRITF` -10, …) anzeigen.
* **Unterordner `C:\USB`** statt Wurzel (Anwender-Entscheid Frage 4: raeumt sich besser
  auf, die Wurzel von `C:` traegt schon Kernel und Firmware).

### 4.4 Dateiinhalt (reiner Text, ASCII, CRLF, <= 78 Spalten)

Abschnitte in fester Reihenfolge, jeder mit einer Zeile `=== n. NAME ===`, damit ein
spaeterer Parser (oder ein Mensch) sie findet. Inhalt in Stichworten:

```
=== 0. FILE ===
Format        : NewTOS USBSNAP text report v1
Program       : USBSNAP.NCP v1
Kernel        : NewTOS 0.6.1 2026-10-23 (abcdef0)         <- Ssystem(2019)
Snapshot API  : Ssystem 2022, layout v1, generation 1
Recorded      : 30.0 s, pointer muted: yes, ring 256, reports listed <= 1024
Device        : 2 of 3 (index 1)
Timestamps    : time of collection by the host, NOT the time the device sent
                the report (resolution = how often the kernel polls USB).

=== 1. IDENTITY ===
VID:PID       : 046d:c077
Manufacturer  : "Logitech"        (string descriptor; "-" if the device has none)
Product       : "USB Optical Mouse"
bcdUSB 2.00  class 0  subclass 0  protocol 0  EP0 max 8  configurations 1

=== 2. POSITION ===
Controller    : xHCI, 5 root ports
Path          : root port 1 -> hub 2109:3431 port 1       (route string 0x00000001)
Speed         : low speed (1.5 Mbit/s)  [xHCI PORTSC speed id 2]
xHCI slot     : 3

=== 3. DRIVER DECISION ===
Role          : MOUSE (HID boot protocol, decoded -> IKBD)
Role codes    : none / keyboard / mouse / observed-only / hub / failed
Reason        : -
Interfaces:
  if 0 alt 0  class 3 sub 1 proto 2  int-IN 0x81 max 4 interval 10  USED (mouse)
  if 1 alt 0  class 3 sub 0 proto 1  int-IN 0x82 max 64 interval 1   not used:
      HID without boot subclass: report protocol only, no report parser yet
Interrupt transfer errors: 0

=== 4. RAW DESCRIPTORS (hex, offset: 16 bytes) ===
Device descriptor, 18 bytes
0000: 12 01 00 02 00 00 00 08 6d 04 77 c0 00 72 01 02   |..........w..r..|
...
Configuration descriptor, 34 bytes (wTotalLength 34)
HID report descriptor, interface 0, 52 bytes (declared 52)

=== 5. DECODED DESCRIPTORS ===
Device: bLength 18 bDescriptorType 1 (Device) bcdUSB 0x0200 bDeviceClass 0 ...
Configuration: bNumInterfaces 1 bConfigurationValue 1 bmAttributes 0xa0 bMaxPower 50 (100 mA)
  Interface 0 alt 0: bNumEndpoints 1 class 3 (HID) sub 1 (boot) protocol 2 (mouse)
    HID: bcdHID 1.11 bCountryCode 0 bNumDescriptors 1 report length 52
    Endpoint 0x81: IN, interrupt, wMaxPacketSize 4, bInterval 10
Report descriptor items (interface 0):
  05 01            Usage Page (Generic Desktop)
  09 02            Usage (Mouse)
  a1 01            Collection (Application)
  ...
  Report ID: none      Report size/count summary: 3 x 1 bit buttons, 5 bit padding,
  ... (only what the item stream states; no semantic interpretation beyond items)

=== 6. REPORTS ===
Total 143 (seq 0..142), listed 143, lost 0, interrupt errors 0
   seq   t(s)   len  bytes                         decoded (boot mouse only)
     0   0.405    4  00 03 fe 00                   buttons ---  dx +3  dy -2  wheel 0
     1   0.410    4  01 00 00 00                   buttons L--  dx  0  dy  0
...
=== END ===
```
**Das Beispiel oben ist ein Formatmuster mit frei gewaehlten Werten (VID/PID, Strings,
Hexbytes, Zeiten), keine Messung und kein Geraetebeleg.** Echt ist nur die Struktur.

Regeln fuer den Programmteil:

* **Hexdump:** 16 Byte je Zeile, Offset 4-stellig hex, ASCII-Spalte.
* **Decodierte Deskriptoren:** nur aus den Rohbytes abgeleitet, Namen nach HID 1.11 §6.2.2
  (Short-Item-Format §6.2.2.2: `bSize` 0/1/2/4 Byte, `bType` 0 Main/1 Global/2 Local;
  Main-Tags §6.2.2.4–6.2.2.6 Input/Output/Feature/Collection/End Collection; Global-Tags
  §6.2.2.7 Usage Page, Logical Min/Max, Physical Min/Max, Unit(-Exponent), Report Size,
  **Report ID**, Report Count, Push/Pop; Local-Tags §6.2.2.8 Usage, Usage Min/Max, …).
  Usage-Namen nur fuer Seite 0x01 (Generic Desktop), 0x09 (Button), 0x0C (Consumer), 0x0D
  (Digitizer) in einer kleinen Tabelle; sonst Hexzahl. Lange Items (`0xFE`, §6.2.2.3):
  Laenge uebernehmen, ueberspringen. **Keine** Interpretation von Bitfeldern zu Feldlayouts
  — das ist der F4-Parser, nicht Aufgabe dieses Werkzeugs. Die Datei nennt nur, ob ein
  Report-ID-Item vorkommt (dann tragen alle Berichte ein fuehrendes ID-Byte, HID 1.11
  §6.2.2.7/§8.1).
* **Decodierung der Berichte** nur bei Rolle 2 (Boot-Maus, HID 1.11 Anh. B.2: Byte 0 Tasten
  Bit 0–2, Byte 1 X, Byte 2 Y, Byte 3 Rad nur bei `len > 3`, `hid.rs:163-181`) und
  Rolle 1 (Boot-Tastatur, Anh. B.1: Byte 0 Modifier, Byte 1 reserviert, Byte 2–7 Usage-Codes
  als Zahlen). Rolle 3: nur Hex — ausdruecklich vermerkt: „report-protocol device, not
  decoded by NewTOS yet“.
* **Rolle 0/5:** Abschnitt 6 lautet „not recorded (not an input device / not configured)“.
* Kein Seriennummernstring, keine Wanduhr, keine Dateipfade des Anwenders.

## 5. Kernel-seitige Aenderungen (der Kern aus §5.1) und Aufwand

Zeilen gelten fuer `03ae70c8`.

| # | Aenderung | Anker | Aufwand |
|---|---|---|---|
| K1 | Neues Modul `kernel/src/drivers/usb/snapshot.rs`: statischer Speicher (je Geraet Deskriptoren, Interface-Tabelle, Strings, Ring 256×80, Zaehler), `record`, `note_error`, `muted`, `set_mute`; reiner Encoder (Zwei-Pass: Groesse, dann Ausgabe) fuer Op 1–4; per Pfad in `tools/xtask` fuer Host-Tests eingebunden wie `ring.rs`/`config_desc.rs` (`tools/xtask/src/main.rs:61-68`) | neu; `mod.rs:33-35` | 1,25 d |
| K2 | Enumeration behaelt Daten: Rohbytes des Geraetedeskriptors kopieren; `read_config` mit Limit 1024 in gemeinsamen `.dma`-Puffer (4-KiB-ausgerichtet, damit ein Puffer nie eine 64-KiB-Grenze kreuzt, xHCI 1.2 §6.4.1-Hinweis „Data buffers … shall not span 64KB boundaries“); `config_desc::scan` erweitert um Interface-Tabelle, Klasse-3-Liste, `wDescriptorLength` | `xhci/mod.rs:1159-1171`, `:1232-1263`, `:122`; `config_desc.rs:49`, `:135-203` | 1 d |
| K3 | Beobachtungs-Pfad: Wahl (i)/(ii)/(iii), `SET_PROTOCOL` nur bei Boot, `intbuf` 64, TRB-Laenge = Max-Packet (8…64), Laenge `trb_len - Rest`, Rueckgabe als Slice | `xhci/mod.rs:123`, `:584-614`, `:658-716`, `:1440-1466`; `mod.rs:139-157`; `hid.rs:69,110` | 1 d |
| K4 | `HidDev` um `parent`, Rolle/Reason, Fehlerzaehler erweitern; `enumerate_hub` reicht `di` des Hubs an `enumerate_device` (`:1687`); `enum_skipped` an allen F8-Stellen zaehlen | `xhci/mod.rs:167-211`, `:1481`, `:1546`, `:1556`, `:1587`, `:1634`, `:1687` | 0,5 d |
| K5 | `usb::init`: Controller auch **ohne Geraet** behalten (`XHCI = Some(...)` unabhaengig von `enumerate_all() > 0`), damit „kein Controller“ (`ERROR`) und „keine Geraete“ (`n_devices 0`) unterscheidbar sind. Verhaltensaenderung: `poll()` laeuft dann auch bei leerem Controller (`mod.rs:135-137`) — ein Event-Ring-Check je Pump | `mod.rs:63-68` | 0,25 d |
| K6 | Report-Deskriptor- und String-Abruf als letzter Schritt von `enumerate_device`, STALL-Behandlung wie §3.1 | `xhci/mod.rs:1597-1604`, neu `fetch_extras` | 0,75 d |
| K7 | `usb::poll`: `record` vor Dekodierung, MUTE-Pruefung, Fehlerzaehler | `mod.rs:132-159` | 0,25 d |
| K8 | `Host`-Trait: `usb_snapshot(req, sink)` mit Chunk-Sink (der Ssystem-Handler kopiert per `Space::copy_to`, kein grosser Zwischenpuffer); Kernel-Impl in `host_env.rs`; Hostlos-Default liefert „kein Controller“ | `tos-api/src/host.rs:337`; `kernel/src/host_env.rs:413` | 0,5 d |
| K9 | Ssystem: Konstante `S_NEWTOS_USBSNAP = 2022`, Dispatch, Kopf-Validierung nach §2.3, Abbildung auf `E_OK`/`ERROR`/`EINVAL`; Test `tos-api/tests/gemdos_ssystem_usbsnap.rs` mit Mock-Host; Test `gemdos_ssystem_sysinfo.rs` („2022 ist kein Modus“) auf 2023 verschieben | `tos-api/src/gemdos/ssystem.rs:94`, `:121-124` | 0,75 d |
| K10 | Programm `sdk/examples/usbsnap.c` + Image-Eintrag in `tools/make-sd-image.sh` (QEMU-Abbild) | neu; Vorbild `sysinfo.c`, `make-sd-image.sh:269` | 1,75 d |
| K11 | QEMU-Aufruf mit zusaetzlichen Geraeten (heute fest `usb-kbd` + `usb-mouse`): Option oder manueller Aufruf | `tools/xtask/src/main.rs:674-677` | 0,25 d |
| K12 | Testlaeufe QEMU + Auswertung (T1–T6) | — | 0,75 d |
| | **Summe** | | **≈ 9 d Roh**, siehe unten |

**Ehrliche Schaetzung:** die Zeilen K1–K12 addieren sich auf etwa **9 Arbeitstage**. Das
ist mehr als die 2–4 Tage in `hid-mouse-protocols.md` §5.2. Der Unterschied: jene
Schaetzung kannte den Beobachtungs-Pfad (K3) und den Report-Abruf (K6) noch nicht, die
sich aus dem F2-Befund (§1) ergeben, und fuehrte das Programm nur als „klein“. Realistische
Spanne **7–9 Tage**; die Kuerzungen unten sparen zusammen etwa einen Tag. **Nicht**
enthalten: Iterationen an echter Hardware (jede Controller-Aenderung muss der Anwender
an der Pi pruefen; VL805-Besonderheiten sind in der Historie dieses Treibers die
Hauptkostenquelle). Alle Zahlen sind Schaetzung, nicht gemessen.

Kuerzungsoptionen (jeweils ohne Layoutbruch, die Abschnitte fehlen einfach):
* Strings weglassen (Frage 2): −0,3 d (K6 und Teil K10).
* MUTE weglassen (Frage 1): −0,3 d (K1/K7/K9/K10), dafuer Risiko stoerender Klicks.
* Ring nur fuer ein Interface (so spezifiziert) statt aller (siehe Frage 3).
* Report-Deskriptor-Item-Dekodierung im Programm nur als Hex, ohne Namen: −0,5 d (K10).

## 6. Testplan

Hostseitig und QEMU ohne Hardware, Abnahme am Ende an der Pi durch den Anwender.

| # | Test | Wo | Erwartung |
|---|---|---|---|
| T1 | Encoder-Goldvektoren: LIST/DEVICE/REPORTS/MUTE Byte fuer Byte; Ring Ueberlauf (`lost`), `MORE`, leere Abfrage `arg >= next_seq`; Kopf-Validierung; Wrap von `seq` | Host (`cargo xtask test`-Pfad wie `ring.rs`) | exakte Bytes |
| T2 | `config_desc::scan` mit Verbundgeraeten: Roccat (3,1,2)+(3,0,1) aus `run3…log:90-97` (59 Byte), Report-only-Interface, Konfiguration > 1024, HID-Deskriptor fehlt, `bLength 0` | Host | Interface-Tabelle, `wDescriptorLength`, Wahl (i)/(ii)/(iii) |
| T3 | Ssystem-Randfaelle mit Mock-Host: `len < 32`, `buf 0`, falsche Magic/Version/Op, Index zu gross, kein Controller (`ERROR`), zu kleiner Puffer (`E_OK` + `TRUNCATED`, nur Kopf), Op 4 `arg > 12000`; ungueltiger Zeiger beendet den Prozess (Muster `uacc3a.c`) | `tos-api/tests/` | Rueckgabewerte nach §2.3 |
| T4 | **qemu-virt** mit `qemu-xhci` + `usb-kbd` + `usb-mouse` (heutiges Verhalten, `tools/xtask/src/main.rs:674-677`): LIST zeigt 2 Geraete, Rolle 1/2, `reason 0`; DEVICE: Geraetedeskriptor 18 Byte stimmt mit `[usb] device descriptor`-Logzeile (`xhci/mod.rs:1165`) ueberein, Konfiguration und Report-Deskriptor-Laenge = `wDescriptorLength`; Regression: Tastatur tippt, Cursor bewegt sich wie vorher | QEMU | wie links |
| T5 | **qemu-virt** mit zusaetzlich `usb-tablet` (Report-only, Subklasse 0, Protokoll 0 — nach QEMU `dev-hid.c`, extern/nicht lokal [S]) und `usb-hub` mit Maus dahinter | QEMU (Option K11) | Tablet: Rolle 3, Ring fuellt sich, **kein** Cursor-Effekt; Hub: Kind hat `parent` und `route`; Reihenfolge/Eintraege stimmen |
| T6 | Programm Ende-zu-Ende in QEMU: Dateien `C:\USB\VVVVPPPP.TXT` entstehen (Durchfuehrung §9.1: kein QEMU-Board hat USB **und** Laufwerk), zweiter Lauf → `.T01`; Format nach §4.4 (alle Abschnitte); **MUTE**: Cursor friert waehrend der Aufnahme ein und laeuft danach; Klick waehrend der Aufnahme loest am Desktop **nichts** aus; Programm per Kill waehrend der Aufnahme → Eingabe kehrt nach spaetestens 35 s zurueck. Berichte erzeugen: QMP `input-send-event` — **Vorsicht:** mit `device=<usb-id>` bricht QEMU 11.0.2 ab (USB02 Punkt 7); ohne `device` ist der Weg zum richtigen Geraet **ungeprueft** [?]; Rueckfall: nur ein USB-Eingabegeraet je Lauf | QEMU | Dateien wie spezifiziert |
| T7 | **raspi4b** (kein USB): `Ssystem(2022)` → `ERROR`; Programm druckt „No USB controller…“, beendet ohne Datei, `Ssystem` ohne Abstuerze | `-M raspi4b` | Beleg `pcie_rpi4.rs:465-468` + Lauf |
| T8 | **Echte Pi 4, Anwender:** 1 Maus + 1 Tastatur; Programm laufen lassen; Dateien lesen: stimmen VID/PID/Pfad/Speed mit `lsusb`-Wissen ueberein, Maus-Berichte erscheinen bei Bewegung/Klick, Report-Deskriptor vorhanden; Regression: Desktop/Maus/Tastatur wie vor der Aenderung (Beobachtungs-Pfad aendert nichts am Boot); Datei an den Entwickler zur Sichtung, **dann erst** an Ragnar. Optional: Abgleich mit `C:\USBDIAG.LOG` eines `usb-diag`-Builds | Pi | Abnahme durch den Anwender |

Nach T4–T6 sind die Aussagen §3.2 (Beobachtungs-Pfad ohne Seiteneffekt) und §2.6
(MUTE, Klicks) belegt; bis dahin sind sie **[S]/[?]**.

## 7. Entscheidungen (Anwender 2026-10-23) und die Fragen dazu

Freigegeben mit „ja mach los“ auf die Empfehlungen: **1** MUTE ein; **2** Hersteller-/
Produktstring ja, Seriennummer nein; **3** ein Ring je Geraet, mehrere erst mit F3;
**4** Ausgabe nach `C:\USB\`; **5** der Lead baut Kernel + `USBSNAP.NCP` als Paket, der
Anwender verschickt es; **6** Bildschirmtexte Englisch; **7** Controller ohne Geraet
behalten (K5) ja; **8** Beobachtungs-Pfad fuer alle Builds, ohne Schalter; **9** Hubs nur
als Pfadzeile.


1. **MUTE (Op 4) einschalten?** Vorteil: Klicks waehrend der Aufnahme erreichen den
   Desktop nicht (Desktop-Icons unter dem Zeiger!). Nachteil: der Zeiger bewegt sich
   waehrend der 30 s nicht — ein Tester koennte „Maus tot“ denken; deshalb der
   Zaehler „reports: N“ auf dem Bildschirm. Ohne MUTE: einfacher, aber nicht untersuchtes
   Risiko stoerender Klicks. Empfehlung: **ein**, bis T6 das Gegenteil zeigt.
2. **Hersteller-/Produktstring** erfassen (+0,3 d, zusaetzliche Control-Transfers je Geraet
   beim Booten, Fehler werden ignoriert, Seriennummer **nicht**)? Empfehlung: ja — der
   Produktname ist fuer Ragnars Dateien der wertvollste menschenlesbare Hinweis.
3. **Nur ein Interface je Geraet** zeichnet Berichte auf (das vom Treiber gewaehlte bzw. das
   erste Klasse-3-Interface); alle anderen bekommen Deskriptoren, aber keinen Ring. Der
   Roccat-Fall (Interface 1 = Protokoll 1, Subklasse 0, Endpoint 0x82, Log `run3…:95-97`)
   ist damit **nur halb** zu sehen. Mehrere Ringe waeren der F3-Umbau (M–L) und gehoeren
   nicht in diesen Schritt. Einverstanden?
4. **Ausgabeort** `C:\` (Wurzel) wie angegeben, oder lieber `C:\USB\` (aufgeraeumter, braucht
   `Dcreate` und Existenzpruefung)?
5. **Verteilung an Ragnar:** der Modus ist **neu**, also braucht Ragnar einen neuen
   `kernel8.img` plus `USBSNAP.NCP` — „kein Sonderbuild“ heisst nur „kein
   `usb-diag`-Feature“. Wer baut/verschickt den Satz, und welche Versionszeile gilt
   (die Datei druckt `Ssystem(2019)`)? Ich schreibe keine Karte und lege nichts ab.
6. **Bildschirmsprache** Englisch (Vorschlag) oder Deutsch?
7. **Controller ohne Geraet behalten (K5):** kleine Verhaltensaenderung im Pump-Pfad —
   einverstanden?
8. **Beobachtungs-Pfad fuer alle Builds (§3.2):** konfiguriert kuenftig auch Report-only-
   Geraete, die heute unbeachtet bleiben. Ein Schalter per Bootargument (`usbsnap=off`)
   waere moeglich, kostet aber eine Bootargument-Stelle. Annahme ohne Schalter?
9. **Hubs** schreiben keine eigene Datei (nur Pfadzeile). Genuegt das?

## 8. Ausserhalb des Scopes beobachtet (nicht bearbeitet)

* `154-ssystem.md` nannte „2021 is the next free number“ — falsch (2021 ist RST01,
  `reset.rs:100`); mit der DRAFT-Zeile dort korrigiert, weil die Nummernvergabe davon
  abhaengt.
* **Halted-Endpoint ohne Reset Endpoint:** der Fehlerzweig re-armiert nur
  (`xhci/mod.rs:700-712`); nach Babble/Transaktionsfehler ist der Endpoint *Halted*
  (xHCI 1.2 §4.8.3) und bleibt stumm. Gehoert in USB02 oder F4/F6.
* Feste TRB-Laenge 8 gegen Report-Protokoll-Berichte > 8 Byte: wird durch K3 fuer die
  Aufnahme entschaerft; der **Dekodierpfad** (`[u8;8]`, `hid.rs`) bleibt F4.
* `docs/api/usb/_state.md` Quellenkarte kennt weder `usb-snapshot.md` noch diesen Weg;
  bei Freigabe eine Zeile ergaenzen.
* `docs/COMPATIBILITY.md` fuehrt die NewTOS-eigenen `Ssystem`-Modi nicht; die Spec liegt
  allein in `154-ssystem.md`.

## 9. Umsetzung Kernelteil (K1–K9, 2026-10-23)

Gebaut in `kernel/src/drivers/usb/snapshot.rs` (Speicher, Ring, Encoder; Hosttests ueber
`tools/xtask`), `config_desc.rs` (Interface-Tabelle, `wDescriptorLength`, Beobachtungs-Kandidat),
`xhci/mod.rs` (K2–K6), `usb/mod.rs` (K5, K7), `kernel/src/host_env.rs` + `tos-api/src/host.rs`
(K8, `Host::usb_snapshot`), `tos-api/src/gemdos/ssystem.rs` (K9). Wo §2/§3 eine Einzelheit
offen liessen, gilt (alles **[E]**, beim Bau festgelegt, vom Programm K10 so zu lesen):

* **Rolle/Reason:** Gruende 16–20 (keine nutzbare Schnittstelle, Konfiguration/Configure
  Endpoint/`SET_CONFIGURATION`/`SET_PROTOCOL` gescheitert) → Rolle 0; 21/22 → Rolle 5. Rolle 3
  traegt als Geraete-`reason` den Code des beobachteten Interfaces (2 oder 3). Ein Hub bleibt
  Rolle 4; scheitert seine Konfiguration, steht 17/18/19 im `reason`.
* **Beobachtungs-Kandidat (ii)** nur in Alternate Setting 0 (das nach `SET_CONFIGURATION`
  aktive); Interfaces mit Alt ≠ 0 stehen nur in der Tabelle.
* **Eintrag-`flags` Bit 3:** `SET_CONFIGURATION` gesendet **und** bestaetigt.
* **Abschnitt 1** steht immer da (`FAILED`, wenn der Geraetedeskriptor nicht lesbar war).
  Abschnitte 5/6 fehlen, wenn der Geraetedeskriptor keinen String nennt (Index 0); `FAILED`
  bei gescheitertem Abruf, `SKIPPED`, wenn nicht versucht (Geraet nicht konfiguriert).
  Abschnitt 3 ist `SKIPPED` ohne HID-Deskriptor (`wDescriptorLength` 0) und ab dem fuenften
  HID-Interface.
* **Schritt 7** endet beim ersten gescheiterten EP0-Abruf (ein STALL haelt EP0 an, §3.1);
  alles danach bleibt `SKIPPED` bzw. die Strings `FAILED`. Strings: zuerst String 0 mit 4 Byte
  (erste LANGID), dann je String 64 Byte (Kopf + 31 Codeeinheiten) mit dieser LANGID.
* **Kopf bei Fall 3/4/6/7 (§2.3):** `total_len = used_len = 32`, `flags = 0`, die Anfrage
  (`op`, `index`, `arg`) zurueckgespiegelt.
* **MUTE** wirkt auch, wenn die 8-Byte-Antwort nicht in den Puffer passte (nur Kopf,
  `TRUNCATED`); eine Frist, die genau auf 0 umliefe, wird um einen Tick auf 1 verschoben.
* **`enum_skipped`** zaehlt auch Address-Device-Fehler (Liste §2.4), obwohl der Slot dann als
  Rolle 5 in der Liste steht. Ein Hub, dessen restliche Ports das `MAX_HID`-Budget nicht mehr
  zulaesst, zaehlt einmal.
* Neue Bootlog-Zeilen (alle Builds): je HID-Interface `report descriptor N bytes (declared M)`,
  beim Beobachtungs-Pfad `report-protocol HID interface observed … not decoded`,
  Zusammenfassung `N devices (K keyboard/mouse, O observed only)`.

Belege: `measurements/measurement-2026-10-23-usb01-snap-kernel/RUNINFO.TXT`.

### 9.1 Umsetzung Programm und QEMU-Option (K10, K11, 2026-10-23)

Gebaut: `sdk/examples/usbsnap.c` (→ `USBSNAP.NCP`, Image-Eintrag `tools/make-sd-image.sh`),
`cargo xtask run --usb-extra` / `--usb-dev <spec>` (`tools/xtask/src/main.rs`
`qemu_usb_args`, Doku `docs/sdk/06-image-and-testing.md`). Entscheidungen, wo §4 offen war
oder beim Testen nicht trug (alle **[E]**):

* **Reihenfolge:** DEVICE (Op 2) je Geraet schon **vor** der Startfrage (§4.2 Schritt 4 nannte
  „danach“), damit Hersteller-/Produktstring in der Geraeteliste stehen. Der Ordner `C:\USB\`
  wird ebenfalls **vor** der Aufnahme geprueft/angelegt (`Fattrib` → bei `EFILNF` `Dcreate`):
  scheitert das, steht die Meldung da, bevor der Anwender 30 s Maus bewegt. Existiert
  `C:\USB` als **Datei**, eigene Meldung, kein Schreiben.
* **`NOIDrrii`:** `rr` = Root-Port dezimal, `ii` = API-Index (0-basiert) dezimal; die Datei
  nennt den Index in `Device : n of N (index i)`. Gilt fuer Rolle 5 **oder** VID = PID = 0.
* **Kollision:** `Fopen(…, 0)` ≥ 0 → belegt, `EFILNF` → frei, jeder andere Fehler → Meldung,
  Datei uebersprungen; `.TXT`, dann `.T01` … `.T99`.
* **Startwartezeit / Uhr:** `now_hz200` aus einem LIST-Aufruf mit `len = 32` (nur Kopf); der
  Aufnahmebeginn ist `now` der MUTE-Antwort, gelesen wird ab `ring_next_seq` der LIST kurz davor.
  Waehrend der Aufnahme je Schleife `Cconis`, alle 10 Ticks Op 3 je Geraet mit Ring, jede Sekunde
  die Zaehlerzeilen per `ESC Y`; dazu eine feste Hinweiszeile („Move the mouse …“).
* **Dateiformat:** Zeilen > 78 Spalten bricht der Schreiber am letzten Leerzeichen um
  (Fortsetzung unter dem Wert bzw. 4 Spalten eingerueckt). `ring 256` in Abschnitt 0 ist die
  v1-Grenze aus §2.7 (das Programm akzeptiert nur Layout v1). Tastatur-Berichte (Rolle 1):
  Modifier und Usage-Codes **hex**, nur die von 0 verschiedenen. Logical/Physical Min/Max und
  Unit Exponent werden **vorzeichenbehaftet** gelesen (HID 1.11 Beispiel `15 81` = -127); ist
  der Wert negativ, steht der Rohwert dabei (`25 ff` → „-1, raw 0xff“: manche Geraete meinen
  ihn vorzeichenlos). Namen: Deskriptorfelder nach HID 1.11 Anhang E, Items nach §6.2.2,
  Usage Pages/Usages nach FreeMiNT `hid-common/hid.h` (lokal), xHCI-Codes Tabelle 6-90,
  Speed-IDs Tabelle 7-13; alles andere als Zahl.
* **Kernel-Nachbesserung MUTE-Beginn (T6-Befund):** In QEMU wurde die RETURN-Taste, mit der die
  Aufnahme startet, gedrueckt dekodiert, ihr Loslassen aber vom Mute verschluckt; die
  Tastenwiederholung lief 30 s weiter, die Wiederholungen erfuellten nach der Aufnahme sofort
  „Press any key to exit“ und starteten das Programm erneut, dessen Startfrage ebenfalls sofort
  erfuellt war (A/B im Beleg unten: `serial/virt-norelease.log` ohne, `serial/virt-extra.log`
  mit der Abhilfe). Abhilfe in `usb::poll`:
  `Store::mute_state` meldet den Beginn eines Mute genau einmal, dann `hid::release_all()` —
  leerer Tastaturbericht (Break fuer jede gehaltene Taste und jeden Modifier, beendet die
  Wiederholung) und, falls ein Knopf gedrueckt war, ein Mausbericht ohne Knopf und ohne
  Bewegung. Ausserhalb der Store-Ausleihe (§2.8). Verlaengern eines laufenden Mute ist kein
  neuer Beginn.
* **Speicher:** der Schnappschuss-Speicher liegt in `.bss` (vorher `.data`, +225 KB im Image):
  `Fetch` ist `repr(u8)` mit `NotTried = 0`, unbelegte Slots sind `Device::UNCLAIMED` (alle Bytes
  0); `Store::claim` setzt weiterhin `Device::EMPTY`, gelesen wird nur `devices[..n_devices]`.
* **Test-Naehte:** `usbsnap.c` hat zwei `#ifdef`-Stellen fuer Messbauten (`USBSNAP_TEST_SNAPCALL`:
  `snap_call` kommt vom Wrapper; `USBSNAP_TEST_SINK`: die `out_*`-Dateifunktionen kommen vom
  Wrapper). `sdk/build.sh` setzt keine; die Wrapper liegen im Messordner.
* **Warum Wiedergabe:** `qemu-virt` hat xHCI, aber kein Laufwerk; `raspi4b` hat die Karte, aber
  kein USB. T6 wurde deshalb geteilt: echte Kernel-Antworten auf `qemu-virt` (Dateien auf die
  Konsole), Dateischreiben auf `raspi4b` mit den dort aufgezeichneten Antworten. Der Klick auf
  den **Desktop** waehrend der Aufnahme ist nur an der echten Pi pruefbar (T8); in QEMU belegt
  eine AES-Probe (`graf_mkstate`) Zeiger und Knopf.

Belege: `measurements/measurement-2026-10-23-usb01-snap-program/RUNINFO.TXT`.

## 10. Layout v2: mehrere Funktionen je Geraet (USB01 F3, 2026-10-24)

Grundlage: `hid-mouse-protocols.md` §9.7, Entscheidung 8 in §9.11 (**Option A**: ein Ring je
Geraet, Interface-Tag im Eintrag, `VERSION = 2`, `USBSNAP.NCP` im selben Schritt). Seit F3 kann ein
Geraet mehrere *Funktionen* haben (HID-Interfaces mit eigenem Interrupt-IN-Endpoint, §9.2 dort),
z. B. ein Funkempfaenger Tastatur + Maus. v2 gibt reservierten v1-Feldern eine Bedeutung; ein
v1-Programm wuerde Mausberichte als Tastaturberichte lesen. Darum ist die Layout-Version 2 und
der Kernel lehnt `version = 1` nach §2.3 Nr. 3 ab (`EINVAL`, Kopf mit `version = 2`).

| Stelle | v2 |
|---|---|
| Kopf `version` (§2.2) | **2** |
| Listeneintrag Byte 14 `role` / 15 `reason` (§2.4) | die der **primaeren** Funktion (Funktion 0, Deskriptor-Reihenfolge, §9.3 R8 dort) |
| Listeneintrag Byte 18/19 `ring_iface`/`ring_ep` | Funktion 0; `0xFF` bleibt „kein Ring“ |
| Listeneintrag Byte 28–30 `int_errors`/`last_cc` | weiter je Geraet: Summe ueber alle Funktionen |
| Listeneintrag Byte 31 | `n_funcs`: Funktionen, deren Endpoint den Ring fuellt |
| Interface-Zeile Byte 14 (§2.5) | 1 = Funktion (mehrere Zeilen moeglich) |
| Interface-Zeile Byte 15 | Rolle der Funktion: 1 Tastatur, 2 Maus, 3 beobachtet; 0 keine |
| Ring-Eintrag +12/+13 (§2.6) | `iface`, `ep` der Funktion; +14 u16 reserviert 0. Berichte aller Funktionen eines Geraets stehen in **einer** `seq`-Folge in der Reihenfolge des Abholens |
| Ring-Eintrag `flags` Bit 0 | „gekappt“ je Funktion: deren `wMaxPacketSize & 0x7FF > 64` |
| Reason 5 | reserviert, nicht mehr vergeben |
| Reason 6/7/8 | `FunctionLimit` / `AlternateSetting` / `EndpointCollision` (§2.4) |

Unveraendert: Ringlaenge 256 × 80 Byte je Geraet, alle anderen Felder, Op-Codes, Rueckgabewerte.

**Beim Bau festgelegt [E]** (Einzelheiten, die `hid-mouse-protocols.md` §9 offen liess):

* **Reason 7 und 8** sind neu. §9.3 R1 (Alternate Setting) und R7 (Endpoint-Kollision) verlangen
  „ueberspringen und loggen“, nannten aber keinen Code; ein Interface, das als Funktion in Frage
  kaeme, braucht in der Interface-Zeile einen Grund ≠ 0. Andere Interfaces in Alt ≠ 0 behalten
  ihren Klassengrund (1–3) wie in v1.
* **Fallback** (§9.4, Entscheidung 10): jeder Nicht-Erfolgs-Completion-Code des gemeinsamen
  Configure-Endpoint-Befehls fuehrt zur Wiederholung mit Funktion 0 allein (nicht nur Bandwidth /
  Resource Error); ein Timeout nicht (der Befehlsring antwortet dann nicht). Die uebrigen Zeilen
  bekommen Reason 18.
* **`SET_PROTOCOL` scheitert fuer Funktion k > 0:** k und alle spaeteren Funktionen bleiben
  unscharf (Reason 20) — die spaeteren, weil nach dem Fehler kein Steuerrequest mehr an EP0 geht
  (§9.4 „weitere Steuerrequests unterbleiben“) und eine Boot-Funktion ohne `SET_PROTOCOL(boot)`
  im Report-Protokoll stuende. Auch `fetch_extras` entfaellt dann.
* **Funktionsbudget insgesamt** (`MAX_FUNC` 12): reicht es nicht, bekommen die ueberzaehligen
  Funktionen Reason 6 und die Logzeile „not used: function limit“; bekommt schon Funktion 0
  keinen Platz, hat das Geraet Rolle 0 und Reason 6.
* **Konnten Endpoints angelegt werden**, gehoeren die Funktionen dem Geraet auch dann, wenn
  danach `SET_CONFIGURATION` oder `SET_PROTOCOL` scheitert (der Controller haelt Endpoint-Contexts
  auf ihre Ringe); sie werden nicht scharf gemacht und verbrauchen ihr Budget.
* Logzeilen: je weitere Funktion eine Zeile `HID iface N protocol P → int-IN ep … (function k)`
  und eine `HID configured: slot … (keyboard|mouse)`; nach der Zusammenfassung
  `N functions armed (MAX_FUNC 12)`, nur wenn ein Geraet mehrere Funktionen hat. Fuer Geraete mit
  einer Funktion bleiben alle Zeilen wie vorher.

`USBSNAP.NCP` v2 (`sdk/examples/usbsnap.c`): fragt Layout 2 an, lehnt andere ab („this program
reads v2“); Liste und Zaehler zeigen ein Geraet mit Tastatur- und Mausfunktion als `Kbd+Mouse`
(dafuer holt das Programm die DEVICE-Antworten jetzt **vor** der Geraeteliste); Datei-Format v2:
Abschnitt 3 nennt `n_funcs`, „USED“ je Interface nach dessen Zeilenrolle; Abschnitt 6 hat bei
mehreren Funktionen eine Spalte `if` und dekodiert jeden Bericht nach der Rolle seiner Funktion,
dazu je Funktion die Zahl der gelisteten Berichte.

Belege: `measurements/measurement-2026-10-24-usb01-f3/RUNINFO.TXT`.

# ACC06 — Ein kleines Accessory als Pruefling fuer ACC02/ACC03

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt ACC06-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 16, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by d26a5fe1d (feat(gemdos): Ssystem reports ARM clock and SoC te), named by body

Line 3 before the migration:
Status: **TEIL 1 ERLEDIGT (2026-09-24), TEIL 2 ALS QUELLTEXT** · Prioritaet: **P2** · Typ: Abnahmefall + Kernelbindung

Abhaengigkeiten: [ACC01](ACC01.md) (Quellenanalyse, offen) — der Pruefling
laeuft erst, wenn ACC02/ACC03 stehen; **gebaut wird er vorher**, wie
`curdir.c` vor DESK18 dastand.

## Zweck

Ein Accessory, das im Desk-Menue erscheint und auf Klick **Taktfrequenz und
CPU-Temperatur** zeigt. Es ist der Abnahmefall fuer das Accessory-Geruest:
ohne ihn wird ACC02/ACC03 gegen nichts geprueft.

Lead-Entscheidung zur Anzeigeform (2026-09-24): **ein `form_alert`-Kasten**,
kein eigenes Fenster. Ein einziger AES-Aufruf, kein selbstgebauter
Objektbaum — groesste Chance, dass er auf der frischen
Accessory-Implementierung sofort laeuft. Eine Fensterfassung ist spaeter ein
eigenes Ticket.

```
  [i] NewTOS System
      ARM clock : 1500 MHz
      CPU temp  : 47.2 C
      [  OK  ]
```

## Teil 1 — die zwei Werte muessen aus dem Kernel herauskommen

Heute gibt es sie nirgends: die Mailbox-Infrastruktur steht
(`kernel/src/drivers/bcm2711/mailbox.rs`), aber die Firmware-Tags
`GET_CLOCK_RATE` (`0x0003_0002`, Takt-ID 3 = ARM, Wert in Hz) und
`GET_TEMPERATURE` (`0x0003_0006`, ID 0, Wert in Tausendstel Grad Celsius)
sind nicht implementiert — und ein EL0-Programm kommt an die Mailbox ohnehin
nicht heran.

**Lead-Entscheidung zum Weg (2026-09-24): `Ssystem`, mit NewTOS-eigenen
Unterfunktionen ab 2000.**

* `Ssystem` gibt es schon (GEMDOS 0x154, `tos-api/src/gemdos/ssystem.rs`,
  heute `S_GETCOOKIE`/`S_SETCOOKIE`/`S_DELCOOKIE`), und FreeMiNT-kompatible
  Erweiterungen sind die sanktionierte Richtung des Projekts.
* FreeMiNT belegt 0–29, 100, 900–913, 1000–1100 und `0x54f8`
  (`references/freemint/sys/mint/ssystem.h`) — **ab 2000 ist frei**, also
  kein Kollisionsrisiko. Vorschlag: `S_NEWTOS_CPUCLOCK = 2000`,
  `S_NEWTOS_CPUTEMP = 2001`. Die Nummernwahl gegen die Quelle gegenpruefen,
  bevor sie festgeschrieben wird.
* Die Keksdose taugt **nicht**: eine Temperatur aendert sich, ein Keks ist
  ein Startwert.

Der Weg vom Trap zur Mailbox geht ueber den `Host`-Trait
(`tos-api/src/host.rs`) wie jede andere Hardwareanbindung — `qemu-virt` und
eine hostlose Konfiguration antworten ehrlich mit „gibt es hier nicht" und
der Aufrufer macht `ERROR` daraus, **keine erfundene Zahl**.

## Teil 2 — der Pruefling

Ein Konsolenprogramm (`.NCP`) zuerst, das die zwei `Ssystem`-Aufrufe absetzt
und die Werte druckt: damit ist Teil 1 belegbar, **bevor** es ein Accessory
gibt. Danach das Accessory selbst: `appl_init` → `menu_register` →
Ereignisschleife auf `AC_OPEN`/`AC_CLOSE`, auf `AC_OPEN` der Kasten oben. Ein
Accessory endet nie.

`menu_register` (35) gibt es bereits und traegt Titel + Anwendungs-ID ein
(`tos-api/src/aes/menu.rs:1105`); was fehlt, ist das Eintragen in die
Desk-Klappliste und das Senden von `AC_OPEN` — das ist ACC02/ACC03 und
**nicht** dieses Ticket.

**Offen und von ACC01 zu entscheiden:** die Dateiendung. Klassisch ist
`.ACC`; unsere eigenen Formate heissen `.NAP`/`.NCP`, damit der spaetere
Emulator die m68k-Endungen bekommt. Hier **nicht** vorgreifen.

## Arbeit

Teil 1 vollstaendig, mit Hosttest fuer die Trap-Ebene und einem QEMU-Lauf
des Konsolenprogramms. Teil 2 als Quelltext im SDK, mit der Begruendung im
Kopf, warum er noch nicht laufen kann. Danach ist ACC02/ACC03 pruefbar.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-24)

**Teil 1 ist gebaut und belegt, Teil 2 liegt als Quelltext vor.**
Messung: [`measurements/measurement-2026-09-24-acc06/`](../measurements/measurement-2026-09-24-acc06/).
Kurzfassung: [`ACC06-STATUS-DONE.MD`](ACC06-STATUS-DONE.MD).

### Die Nummernwahl gegen die Quelle geprueft

Das Ticket nannte FreeMiNTs belegte Bereiche als „0–29, 100, 900–913,
1000–1100 und `0x54f8`". Die Quelle sagt etwas Weiteres: `sys/mint/ssystem.h`
(@ `f906416`) reserviert per Kommentar **ganze Bloecke**, nicht nur die
vergebenen Nummern — *„additional informations about the kernel / reserved
900 - 999"* und *„debug section / reserved 1000 - **1999**"*. Und der
Verteiler beantwortet eine Nummer, die im Header gar nicht steht: `case 3000:`
(`sys/ssystem.c:631`), ein `register_trap2`-Testhaken hinter
`#ifdef MINT_STATUS_CVS`, im Quelltext als *„XXX only for testing"* markiert.

Ergebnis fuer uns unveraendert: **2000 ist frei**, und zwar mit einer Luecke
von 2000 bis 2999. `S_NEWTOS_CPUCLOCK = 2000` und `S_NEWTOS_CPUTEMP = 2001`
sind festgeschrieben. Vermerkt ist zugleich, dass ein spaeterer NewTOS-Block
nicht ueber 2999 hinauswachsen darf, ohne 3000 erneut zu pruefen.

### Der Weg, den die zwei Werte nehmen

`Ssystem(0x154)` → `newtos_scalar` (`tos-api/src/gemdos/ssystem.rs`) →
`Host::cpu_clock_hz` / `Host::cpu_temperature_mdeg` (`tos-api/src/host.rs`) →
`kernel/src/host_env.rs` (nur `board-rpi4`) →
`mailbox::get_clock_rate` / `::get_temperature`
(`kernel/src/drivers/bcm2711/mailbox.rs`, Tags `0x0003_0002` und
`0x0003_0006`).

Argumente: `arg1` = Zeiger auf ein 32-Bit-LONG oder 0 fuer den Wert im
Rueckgabe-LONG; `arg2` **reserviert und zurueckgewiesen** (`EINVAL`) statt
stillschweigend ignoriert. Ein Brett ohne Mailbox behaelt den Trait-Standard
`None`, und der Trap macht daraus `ERROR` — **keine erfundene Zahl**, und in
diesem Fall wird auch nichts in den Zeiger geschrieben.

### Was QEMU antwortet

Der Emulator beantwortet **beide** Tags, mit festen Modellzahlen:
**700,0 MHz** und **25,0 C**. Sie stehen so im Protokoll — erfunden ist
nichts, gemessen ist es auch nicht. Der `ERROR`-Pfad war im Emulator also
nicht zu sehen und ist stattdessen am Trap belegt
(`tos-api/tests/gemdos_ssystem_cpuinfo.rs`).

**Nur die echte Pi 4 kann entscheiden**, ob der Treiber wirklich die Firmware
liest: dort duerfen dort nicht dieselben zwei Zahlen stehen, und die
Temperatur muss sich nach einem Lastlauf veraendern. Hardwarepunkt **64** in
`docs/api/desktop/_hw-test-2026-09-17.md`.

### Teil 2

`sdk/examples/sysacc.c` ist geschrieben und uebersetzt sauber, **laeuft aber
nicht**: `menu_register` (35) traegt nur ein, niemand haengt den Eintrag in
die Desk-Klappliste und niemand sendet `AC_OPEN`. Das steht so im Dateikopf.
Ein Geruest wurde nicht darum herum gebaut, die Dateiendung bleibt ACC01
ueberlassen (der Bauhinweis nennt bewusst `.TBD`), und
`tools/make-sd-image.sh` installiert ihn absichtlich nicht.

### Offen geblieben (gehoert in eigene Tickets)

1. **Der Emulator taugt nicht als Bildbeleg fuer den AUTO-Textschirm.** Die
   Textphase ist kuerzer als 0,12 s (zwei Verfahren, RUNINFO §3); ein Bild
   der zwei Zeilen gibt es erst mit dem Accessory.
2. **`GET_CLOCK_RATE` gegen `GET_CLOCK_RATE_MEASURED` (`0x0003_0047`).**
   circle fragt erst den gemessenen Takt und faellt auf den eingestellten
   zurueck (`references/circle/lib/machineinfo.cpp:459-471`). Welcher der
   beiden auf der Pi 4 die ehrlichere Zahl ist, entscheidet ein Lauf auf der
   Hardware — nicht dieses Ticket.

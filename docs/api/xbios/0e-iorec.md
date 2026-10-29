# `Iorec` (XBIOS 0x0E)

> Spec, EmuTOS @ `1a7484ab` (`references/emutos`, HEAD `1a7484aba1bf`, per
> `docs/analysis/ORACLE.md`). Source: `bios/xbios.c:336-348` (`iorec`), slot
> 0x0E of `xbios_vecs[]` (`bios/xbios.c:1304`).
> **[XBIOS01] 2026-10-28:** NewTOS-Vertrag des IOREC-Abbilds ergaenzt (Abschnitt
> "NewTOS-Vertrag"), Status und die falsche `EINVFN`-Aussage korrigiert.
> **[XBIOS01] 2026-10-29:** implementiert nach E1-E6 (Abschnitt "NewTOS
> implementation"); die Abschnitte F1-F3 beschreiben, wo sie "heute" sagen, den
> Stand **vor** dem Fix (2026-10-28) und bleiben als Begruendung stehen.

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, `devno` -> record mapping, `-1` for an invalid device,
> the field layout and ring semantics a program reads/writes. Reference: the
> ACIA/UART drivers behind each record, the concrete addresses.

## Status: **Implementiert 2026-10-29 (XBIOS01, E1-E6)** — Vertrag entschieden 2026-10-28

Seit XBIOS01: `Iorec(0/1/2)` liefert Zeiger auf die Records in der gemeinsamen
IOREC-Seite, jedes andere Geraet `-1`; der IKBD-Record **ist** die
Tastenschlange aller Verbraucher (Abschnitt "NewTOS implementation"). Der
folgende Absatz ist der Befund vor dem Fix.


[XBIOS01] Vorher stand hier "decision deferred" und "returns `EINVFN`". Beides
ist falsch: `Iorec` ist in NewTOS nicht verdrahtet; der Sammelzweig
`other => other as i64` liefert die **Funktionsnummer `0x0E`**, nicht `EINVFN`
(`tos-api/src/xbios/mod.rs:65-138@dispatch`, `:138`). Gemessen: jedes Geraet,
auch `3`, `-1`, `0x7FFF`, `-32768`, liefert `0x0000000E`
(`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b/XCPROBE.LOG`,
Zeilen I01-I07). Die zurueckgestellte Entscheidung ist mit XBIOS-CHARDEV-001
(`docs/audits/2026-10-28-xbios-chardev.md`) und TASK/XBIOS01 faellig.
Der Teil "NewTOS-Vertrag" legt fest, was aus dem Quelltext **folgt**; was eine
Wahl war, hat der Anwender am 2026-10-28 als E1-E6 entschieden (Ende dieses
Dokuments); die Optionen bleiben dort als Begruendung stehen.

---

## EmuTOS source

**Primary location:** `bios/xbios.c:336-348`:
`devno 0` -> `rs232iorecptr` (`EXT_IOREC *`), `1` -> `&ikbdiorec`,
`2` -> `&midiiorec`, sonst `-1` (`bios/xbios.c:338-347`). Der Selektor ist ein
`WORD`, **vorzeichenbehaftet** verglichen: `3`, `-1`, `0x7FFF`, `-32768` ->
`-1` (EmuTOS gemessen I04-I07, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt/XCPROBE.LOG`).

**Installed handler / vector:** keiner — direkter Tabelleneintrag.
**Helpers called:** keine. **Systemvariablen geschrieben:** keine.
**Build configuration:** der Wrapper ist ungated. Die Records existieren
unabhaengig von `CONF_WITH_IKBD_ACIA`/`CONF_WITH_MIDI_ACIA` (sie liegen in
`bios/aciavecs.S`, `.bss`); der Inhalt von Geraet 0 haengt vom Serialtreiber
ab (`CONF_WITH_MFP_RS232`/`CONF_WITH_SCC`/`CONF_WITH_TT_MFP`,
`bios/serport.c:84-114`, `:1003-1018`). Auf dem Oracle-Falcon-Profil zeigt
`Iorec(0)` auf einen anderen Record (SCC) als auf TT (`0x478A` gegen `0x4A54`,
`runs/PROBE-EMUTOS-falcontc` / `-tt`, I01) — die **Adresse** ist nie Vertrag.

### Record-Strukturen (EmuTOS, m68k, big endian)

`IOREC` (`bios/iorec.h:20-27`), **14 Byte**:

| Offset | Feld | Typ |
|---:|---|---|
| 0x00 | `buf` | `UBYTE *` (4) |
| 0x04 | `size` | `WORD` |
| 0x06 | `head` | `volatile WORD` |
| 0x08 | `tail` | `volatile WORD` |
| 0x0A | `low` | `WORD` |
| 0x0C | `high` | `WORD` |

`EXT_IOREC` (`bios/serport.h:57-65`): `in` @0x00 (14), `out` @0x0E (14),
`baudrate` @0x1C, `flowctrl` @0x1D, `ucr` @0x1E, `datamask` @0x1F, `wr5`
@0x20 (je `UBYTE`) -> 33 Byte Felder; `sizeof` auf m68k folgt aus der
2-Byte-Ausrichtung (34, **abgeleitet, nicht gemessen**). Korrektur: `_state.md`
§3 nennt "24 bytes" — das ist weder 33 noch 34 (Beobachtung unten).

Anfangswerte:
- IKBD: Puffer 256 Byte, `low = 64`, `high = 192` (`bios/aciavecs.S:99-100,
  233-239`; kopiert nach `_ikbdiorec` bei Init `:197-201`). MIDI: 128 Byte,
  `low = 32`, `high = 96`. Gemessen I02b/I03b (`runs/PROBE-EMUTOS-tt`).
- Geraet 0: `in` und `out` je 256 Byte, 64/192, `baudrate` = `DEFAULT_BAUDRATE`,
  `flowctrl 0`, `ucr 0x88`, `datamask 0xff`, `wr5 0xEA`
  (`bios/serport.c:86-89`); Puffer-Zeiger werden in `init_serport` gesetzt
  (`:1015-1018`). Gemessen I01b/I01c: `baud=1 flow=0 ucr=88 datamask=ff`
  (`wr5` wurde nicht gelesen).

### Ringsemantik (was ein Programm beobachtet; aus dem Code, Zeilen unten)

- **`head`/`tail` sind Byte-Indizes** in `buf`, Ring bei `size`. Leer iff
  `head == tail` (`bios/ikbd.c:242`, `bios/midi.c:25`, `bios/serport.c:194`).
- **Produzent** (Interrupt): `tail` zuerst weiterschieben (mit Umbruch bei
  `>= size`), voll iff `neuer tail == head` -> Eintrag **verworfen**, sonst
  Eintrag nach `buf[neuer tail]` schreiben und `tail` setzen
  (`bios/ikbd.c:157-173`, `bios/serport.c:331-342`).
- **Konsument** (`Bconin`): `head` zuerst weiterschieben (Umbruch), dann
  `buf[head]` lesen (`bios/ikbd.c:269-273`, `bios/serport.c:171-183`,
  `bios/midi.c:48-53`). Die **Schrittweite ist je Geraet verschieden**:
  IKBD **4 Byte** (`bios/ikbd.c:163,269`), MIDI und RS-232 **1 Byte**
  (`bios/midi.c:48`, `bios/serport.c:150`). Kapazitaet daher IKBD 63 Eintraege
  (256/4 - 1), MIDI 127, RS-232 255.
- **IKBD-Eintrag** = 32-Bit-Wort `shifty<<24 | scancode<<16 | ascii`
  (Union `bios/ikbd.c:427-435`; Push `:909`); `Bconin(2)` blendet Bits 24-31
  aus, wenn `conterm & 8` fehlt (`bios/ikbd.c:279-280`). **Die Tastenwiederholung
  schreibt in denselben Ring** (`bios/ikbd.c:524`).
- **`low`/`high`** werden von EmuTOS nirgends gelesen (Durchsicht `bios/*.c`,
  `*.S`: keine Verwendung von `->low`/`.low` fuer IOREC; die Treffer in
  `bios/dsp.c` sind DSP-Register). Sie sind reine, vom Programm lesbare Daten.
- **`buf`/`size` darf das Programm ersetzen**: der Interrupt liest `buf` bei
  jedem Byte neu ("must use ptr, user may have changed it",
  `bios/aciavecs.S:331`); `ikbd.c:171` benutzt `ikbdiorec.buf` ebenfalls bei
  jedem Push. Ein Terminalprogramm kann so den RS-232-Puffer vergroessern.
- **Der gaengige Flush `io->ibufhd = io->ibuftl`** leert den Ring wirksam:
  `bconstat2` testet nur `head == tail` (`bios/ikbd.c:242`). Dass `Bconstat(2)`
  danach 0 liefert, ist aus dem Code **abgeleitet**; gemessen ist nur, dass das
  Programm weiterlaeuft (XCIOFL: `F02 after write hd=0 tl=0`, TT und Falcon,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-EMUTOS-tt/XCIOFL.LOG`).

---

## Trap interface

- **Trap:** 14, **Function ID:** 0x0E. m68k: `Iorec(devno)` = `xbios_l_w(14, a)`
  (`include/xbiosbind.h:32`): `+2: devno (word)`, `0: 0x000E`; 4 Parameterbytes.
- **Native (AArch64):** ID `0x0E` in `x8`, `devno` in `x0` (16 bit, vorzeichenbehaftet
  zu lesen). Ergebnis in vollem `x0`: Zeiger (EL0-VA, unter 4 GiB) oder `-1`
  (`0xFFFF_FFFF_FFFF_FFFF`; EmuTOS: LONG `-1`, I04 `hi=ffffffff`).

## Inputs / Outputs / Errors

| Eingabe | Typ | Notiz |
|---|---|---|
| `devno` | `WORD` | 0 = RS-232 (`EXT_IOREC`), 1 = IKBD, 2 = MIDI (`bios/xbios.c:338-344`) |

| Ausgabe | Wert |
|---|---|
| Erfolg | Zeiger auf Record (stabil: zwei Aufrufe liefern denselben Zeiger, I08 EmuTOS und NewTOS) |
| `devno` nicht in {0,1,2} | `-1` (`bios/xbios.c:345-346`) — kein `EINVFN`, kein Fehlercode sonst |

Seiteneffekte: keine. Der Falcon-Bereich 3..7 gibt auf EmuTOS ebenfalls `-1`
(kein Unterschied zur Oracle-Messung, I04).

---

## NewTOS-Vertrag [XBIOS01]

Alles Folgende ist **Folgerung aus dem Quelltext** und den Messungen oben, soweit
nicht anders markiert. Die Wahl-Punkte stehen als E1-E6 am Ende.

### F1 — Welches Layout bekommt ein NewTOS-Programm (Frage 1)

**Der Kernel liefert das Layout des SDK-Headers, nicht das m68k-Drahtformat.**
Begruendung: ein nativ uebersetztes Programm greift ueber `_IOREC`
(`sdk/libc/overlay/include/mint/ostruct.h:576-583@_IOREC`: `char *ibuf;
short ibufsiz; volatile short ibufhd; volatile short ibuftl; short ibuflow;
short ibufhi;`) zu; diese Struktur ist unter LP64 **24 Byte** gross (gemessen:
`sizeof=24`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b/XCPROBE.LOG` I01b-I03b, gegen 14 auf
EmuTOS). Das m68k-Format ist erst fuer einen spaeteren m68k-Thunk noetig (nur
Vorbereitung dort, nicht jetzt; Praezedenz: die `Kbdvbase`-Tabelle hat
8-Byte-Slots, `tos-api/src/xbios/kbdvecs.rs:198@kbdvbase`,
`docs/api/xbios/22-kbdvbase.md` Teil D).

LP64-Layout (aus den C-Ausrichtungsregeln **abgeleitet**; gemessen ist nur
`sizeof(_IOREC) == 24` und dass die Sonde `out` bei `io + 1` und die Zusatzbytes
bei `(o + 1)` liest, `measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c`, `iorec_show`):

| Offset | Feld | Breite |
|---:|---|---|
| 0 | `ibuf` | 8 (Zeiger, EL0-VA) |
| 8 | `ibufsiz` | 2 |
| 10 | `ibufhd` | 2 (Byte-Index) |
| 12 | `ibuftl` | 2 (Byte-Index) |
| 14 | `ibuflow` | 2 |
| 16 | `ibufhi` | 2 |
| 18-23 | Fuellbytes | — |

Geraet 0 (`EXT_IOREC`, LP64): `in` @0, `out` @24 (zweiter `_IOREC`),
`baudrate` @48, `flowctrl` @49, `ucr` @50, `datamask` @51, `wr5` @52, `sizeof` 56.
Geraete 1/2: ein einzelner `_IOREC` (24).

Gleichbleibende Vertragsinhalte (unabhaengig von der Zeigerbreite): Ringsemantik
und Schrittweiten (4 Byte IKBD / 1 Byte MIDI, RS-232), Eintragsformat
`shifty<<24|scancode<<16|ascii` als 32-Bit-Ganzzahl in der Ziel-Byteordnung,
Puffergroessen 256/256/128, Wassermarken 64/192, 64/192, 32/96.
**LP64-Fallstrick:** ein Programm, das den Eintrag als `long` liest, liest unter
LP64 8 Byte — es muss `int`/`uint32_t` verwenden (abgeleitet aus C; Programm-seitig,
nicht durch den Kernel loesbar).

**Wo es liegt (und MEM12).** Nicht Seite 0: Seite 0 ist fuer EL0 im
Benutzerzustand nur lesbar, nur im Supervisor-Zustand schreibbar
(`docs/api/_super-state.md`, Abschnitt 0). Genau das ist der gemessene Absturz:
`Bus Error ... page-0 system-variable memory, write`, `far 0x18`, weil der
zurueckgegebene Zeiger `0x0E` in Seite 0 liegt
(`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-NEWTOS-raspi4b/BOOT-SERIAL.txt`).
Damit der Flush im Benutzerzustand **schreibbar** ist, muss `ibufhd`/`ibuftl`
(und `buf`, sofern E4) in einer EL0-lesbaren **und -schreibbaren**, nicht
ausfuehrbaren Seite liegen. Vor XBIOS01 gab es genau eine solche Seite: die
prozessprivate RW-Seite bei `0x7F000` (`tos-api/src/bdos/process/mmu.rs:259@PRIV_PAGE_VA`,
Modul-Kopf `tos-api/src/bdos/process/mmu.rs:21@PRIV_PAGE_VA`), belegt mit der `Kbdvbase`-Tabelle (`0x00-0x5F`,
`tos-api/src/bdos/process/loader.rs:1051@KBDVECS_AREA_SIZE`) und dem
`PARMBLK`-Scratch ab `+0x100` (`loader.rs:1022@PARMBLK_SCRATCH_VA`, 40 Byte).
Frei bleiben ab `+0x128` rund 3,7 KiB; ein Satz Records mit Puffern
(IKBD 24+256, MIDI 24+128, AUX 56+256+256 = 1000 Byte) passt dort hinein
(Rechnung aus den oben zitierten Groessen). Ob es dorthin gehoert, haengt an
E1 (die Seite ist je Prozess; EmuTOS' Ring ist global). **Entschieden (E1):**
eine eigene, gemeinsame Seite bei `0x7D000`
(`tos-api/src/bdos/process/mmu.rs:264@IOREC_PAGE_VA`), siehe "NewTOS
implementation".

### F2 — Welche Geraete, und was steht heute dahinter (Frage 2)

| devno | EmuTOS (Oracle) | NewTOS vor XBIOS01 (2026-10-28) | Ziel-Vertrag (seit XBIOS01 umgesetzt) |
|---|---|---|---|
| 0 | `EXT_IOREC *`, 256/256 + Baud/Flow/UCR | nichts: `0x0E`. Dahinter AUX ohne Kabel, `Bconstat(1)=0`, `Bconout(1)` verwirft, `Bcostat(1)=-1` (`tos-api/src/bios/chardev.rs:115@AuxNoCable`, `:229@DEVICES`) | Record vorhanden; `in` leer und bleibt es, `out` ist immer leer (NewTOS verwirft sofort, der Ring kann nie fuellen) — konsistent zum BIOS (E5) |
| 1 | `&ikbdiorec`, 256, 4-Byte-Eintraege | nichts. Die Tastenschlange war `RX_RING` in `kernel/src/console.rs` (mit XBIOS01 entfernt): 128 Eintraege **`u16`** `scancode<<8\|ascii`, ohne Shift-Byte (Wort `kernel/src/ikbd.rs:316@key_word`) | Record mit 64 4-Byte-Eintraegen (E1/E2 entscheiden, ob er die Schlange **ist**) |
| 2 | `&midiiorec`, 128, 1-Byte | nichts. MIDI ist "absent" (`chardev.rs:229@DEVICES`, Slot 3), `Midiws` verwirft (Audit BIOS-CORE-001) | Record vorhanden, leer, nie gefuellt (E5) |
| sonst | `-1` | `0x0E` | `-1` (gemessen EmuTOS I04-I07) |

`Bconmap` ist in NewTOS ebenfalls Funktionsnummer (`tos-api/src/xbios/system.rs:106@bconmap`
gibt `0x2c`): `Iorec(0)` bleibt ein fester Record; ein Umbiegen durch `Bconmap`
(`bios/serport.c:1003,1110-1119`) gibt es nicht (Abschnitt Bconmap: XBIOS-CHARDEV-005,
ausserhalb dieses Auftrags).

### F3 — Kopplung: Spiegel oder die lebende Schlange? (Frage 3)

EmuTOS: der IOREC **ist** die Schlange. Der IKBD-Interrupt schreibt in ihn
(`bios/ikbd.c:157-173`), `Bconstat(2)`/`Bconin(2)` lesen aus ihm
(`:241-273`), und die Wiederholung schreibt hinein (`:524`). Folglich gilt, was
Programme verwenden — und was ein reiner **Spiegel** nicht erfuellt:

1. **Flush wirkt:** `head = tail` leert die Schlange fuer `Bconstat(2)`,
   `Bconin(2)`, `Cconis`, `Cconin` (alles geht durch `bconstat2`/`bconin2`).
   Ein Spiegel, der nicht zurueckschreibt, laesst die Tasten stehen — das
   Programm, das vor einem "druecke eine Taste" leert, liest sofort die alte
   Taste. (Folgerung; gemessen ist nur "kein Absturz".)
2. **Lesen zeigt die Wahrheit:** ein Programm, das `head != tail` pollt oder
   die Eintraege direkt liest, sieht dieselben Tasten wie `Bconin`.

NewTOS' Schlange `RX_RING` (vor XBIOS01) wurde auch vom AES-Ereignismanager (Wortform
`rx_pop_key`, `kernel/src/host_env.rs`) und den `Fread`-Pfaden verbraucht; ein
Spiegel muesste also bei **jedem** dieser Verbraucher synchronisieren. Dazu die
**Zustellung** (Folgerung aus dem Quelltext, der Kommentar steht an
`kernel/src/host_env.rs:56@fill_from_device`): Tastenbytes erreichen die
Schlange nur, wenn eine Pump laeuft; die Pump rufen die Traps (`Bconstat`,
blockierendes Lesen, AES-Idle) und der 200-Hz-Timer. **Der Timer decodiert nur
Zeigerbewegung; Tasten und Wiederholung bleiben bei der Trap-Pump**
(`kernel/src/host_env.rs:513@pump_devices_irq`, `kernel/src/ikbd.rs:341@poll_repeat`).
Ein Programm, das den IOREC im Benutzerzustand pollt **ohne je einen Trap zu
machen**, sieht also nie eine Taste, egal ob Spiegel oder lebender Ring (E3).
Das ist der Unterschied zu EmuTOS, wo der ACIA-Interrupt asynchron fuellt.

**Eintragsbreite.** `RX_RING` hielt (vor XBIOS01) 16-Bit-Woerter ohne `shifty`; der EmuTOS-
Eintrag ist 32 Bit mit `shifty` aus dem Zeitpunkt des Push (`bios/ikbd.c:900-909`).
NewTOS faltete den **aktuellen** Shift-Zustand erst beim Lesen ein (`shift_fold`
in `tos-api/src/bios/chardev.rs`, mit XBIOS01 entfernt). Ein Eintrag im IOREC muss
das 32-Bit-Wort sein (Vertrag nach `bios/ikbd.c:427-435`); der Shift-Zustand
zum Push-Zeitpunkt ist an der Push-Stelle verfuegbar (`kernel/src/ikbd.rs:292@push_rx`,
`shift_state()` direkt daneben) — **gelesen, nicht gebaut** (Stand 2026-10-28;
umgesetzt in XBIOS01).

**Der Kernel misstraut dem beschreibbaren Record** (Folgerung aus S10 in
`docs/api/_super-state.md`: "the kernel reads page 0 only as input it documents,
validated" — dieselbe Haltung gilt fuer jede EL0-beschreibbare Seite): `head`/
`tail` muessen vor jedem Gebrauch auf `0 <= x < size` und auf die Schrittweite
(4/1) geprueft werden; ungueltig -> Ring als leer behandeln und neu setzen, nie
ausserhalb des eigenen Puffers lesen/schreiben.

### F4 — Was bleibt Referenz (nicht Vertrag)

Adressen der Records (`0x4A54` usw.), die Aufteilung `iorec_init`/`.bss`,
`wr5`, der Zeiger-Umweg ueber `rs232iorecptr`. NewTOS darf sie anders legen,
solange F1-F3 gelten.

## NewTOS implementation [XBIOS01, 2026-10-29]

**Where.** One page, `0x7D000` (`tos-api/src/bdos/process/mmu.rs:264@IOREC_PAGE_VA`),
a kernel static (`:248@IOREC_PAGE`), entered EL0 read-write, never executable in
every process root (`tos-api/src/bdos/process/mmu.rs:830@IOREC_PAGE_IDX`), refused to
`map_shared`, its user VA resolved by `private_page_alias` to the shared page (so a
pointer into it may be passed to a trap). Layout in the page (reference, F4):
device 0 `EXT_IOREC` at `+0x00`, IKBD at `+0x40`, MIDI at `+0x60`, the rings at
`+0x100` (IKBD 256), `+0x200` (MIDI 128), `+0x300`/`+0x400` (RS-232 in/out 256)
(`tos-api/src/xbios/iorec.rs:113@AUX_RECORD`). Records are written on first use
with the oracle values (E5): `ibufsiz` 256/128/256+256, `low`/`high` 64/192,
32/96, 64/192, empty, device 0 `baudrate 1`, `flowctrl 0`, `ucr 0x88`, `datamask
0xff`, `wr5 0xea` (`bios/serport.c:86-89`).

**`Iorec`** (`tos-api/src/xbios/iorec.rs:454@iorec`, wired at
`tos-api/src/xbios/mod.rs:107@IOREC`): signed `WORD` selector
(`:441@record_va`), `0/1/2` -> the record's EL0 address, anything else `-1`.

**E1 — the ring is the queue.** `RX_RING` is gone; every producer and consumer
goes through the IKBD record: push `tos-api/src/xbios/iorec.rs:369@push_key`
(EmuTOS `push_ikbdiorec`: `tail` first, full -> dropped, 63 entries), pop
`:374@pop_key` (`head` first, then the entry), `:384@key_pending` (`head !=
tail`). Kernel side: `kernel/src/console.rs:198@push_rx` (IKBD decoder),
`serial_rx_byte` (qemu-virt), `rx_pop`/`rx_pop_key`/`rx_pop_entry`
(`kernel/src/console.rs:231@rx_pop_entry`) for `Fread`/`Cconrs`, the AES and
`Bconin(2)`. Every operation runs with IRQs masked (the qemu-virt PL011 receive
interrupt is a second producer).

**The kernel distrusts the page.** Before every use the record is checked
(`tos-api/src/xbios/iorec.rs:263@check`): `ibuf`/`ibufsiz` must be the kernel's,
`ibufhd`/`ibuftl` below `ibufsiz` and on the entry step (4 IKBD, 1 MIDI/RS-232).
A replaced buffer (E4 (a)) gets the kernel buffer back, emptied; bad indices
empty the ring. The kernel never reads or writes through `ibuf`. After every
trap all four rings are checked as well (`kernel/src/console.rs:267@check_iorec_after_trap`,
called from `kernel/src/arch/aarch64/exceptions.rs:664@check_iorec_after_trap`), so a
replaced buffer is noticed "at the next trap" even when no key moves. Each
repair is warned about once per process tenancy
(`tos-api/src/xbios/iorec.rs:416@take_warnings`, printed by
`kernel/src/console.rs:275@report_iorec_repairs`, reset at process exit
`tos-api/src/bdos/process.rs:1366@release_slot`):
`[iorec] warning: Iorec(1) IKBD buffer or size replaced by the program ...`.

**E2 — 32-bit entries, shift at push time.** The decoder pushes
`shifty<<24|scancode<<16|ascii` with the shift state of the make
(`kernel/src/ikbd.rs:292@entry_from_word`), the key repeat with the repeat's
shift state (`kernel/src/ikbd.rs:353@entry_from_word`, after `poll` re-converted
the word when the modifiers changed, `bios/ikbd.c:502-508`). `Bconin(2)` keeps
bits 24-31 only with `conterm & 8` (`tos-api/src/bios/chardev.rs:351@pack_console_key`,
`bios/ikbd.c:279-280`); the host reads whole entries
(`tos-api/src/host.rs:121@console_read_entry_blocking`,
`kernel/src/host_env.rs:203@console_read_entry_blocking`). Closes BIOS11. qemu-virt
serial bytes enter as `ascii` with scancode 0 and shift 0, as before (EmuTOS'
`push_ascii_ikbdiorec` looks the scancode up in the key tables,
`bios/ikbd.c:192-231` — not done here, see "Out-of-scope observations").

**E3 (a).** Keys still reach the ring only at a trap-side input pump (F3); a
program that polls `ibuftl` without ever making a trap sees no key.

**E5 (a).** Devices 0 and 2 have their records and stay empty: no producer
writes them, `Bconstat(1)`/MIDI stay as before.

**E6.** The `kbdvec` hook gets the IKBD record in `x1`
(`kernel/src/arch/aarch64/exceptions.rs:851@IKBD_IOREC_VA`, EmuTOS `a0 =
&ikbdiorec`, `bios/aciavecs.S:412`).

**Tests.** Unit tests `tos-api/src/xbios/iorec.rs` (ring order, 63 entries,
wrap, flush, bad indices, replaced buffer, selector, warn-once); integration
`tos-api/tests/xbios01_iorec.rs` (dispatcher `Iorec`, records, mapping,
flush -> `Bconstat(2) == 0`, BIOS11 `0x02300042`, `conterm` mask); SDK layout
`sdk/libc/tests/iorec-layout.c` (`_Static_assert` of `_IOREC` and the device-0
record against F1, in `sdk/run-host-tests.sh`). Measured:
`measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`.

---

## Verification criteria (Abnahme)

- `Iorec(0/1/2)`: Zeiger `!= -1`, stabil, gerade; `ibufsiz` 256/256/128,
  `low`/`high` 64/192, 64/192, 32/96, `head == tail == 0` im Ruhezustand, Geraet
  0: `out`-Ring 256, `baudrate 1`, `flowctrl 0`, `ucr 0x88`, `datamask 0xff`.
  Sonde I01-I08.
- `Iorec(3)`, `(-1)`, `(0x7FFF)`, `(-32768)`: `-1`. Faelle `bios-r1` 101/102,
  `xbios-chardev-r1` 24-27.
- Flush: `io->ibufhd = io->ibuftl` im Benutzerzustand laeuft durch (XCIOFL);
  danach `Bconstat(2) == 0` (**[XBIOS01] gemessen 2026-10-29 mit drei Tasten im
  Ring, `tosdump/cases/xbios-flush-r1.tsv`, beidseitig EQUAL, siehe
  `measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`**; vorher: noch nicht gemessen — eine Sonde "Taste
  einspeisen, flushen, `Bconstat(2)`" fehlt; sie braucht Eingabe-Trace wie die
  X-Sonden, TASK/TOOL08).
- Inhalt durch den Zeiger: erst nach TASK/TOOL16 per tosdiff. **[XBIOS01]**
  `xbios-ptr-r1` Faelle 24-31 EQUAL gegen tt und falcontc.
- **[Anwender 2026-10-28]** Layout: Pruefung zur Uebersetzungszeit
  (`offsetof`/`sizeof` des SDK-`_IOREC` und der Geraet-0-Struktur) gegen die
  Offsets dieser Spec — heute sind die LP64-Offsets nur abgeleitet.
  **[XBIOS01]** `sdk/libc/tests/iorec-layout.c` (`_Static_assert`, Ziel
  aarch64), gruen; die Kernel-Seite haelt dieselben Offsets als Konstanten
  (`tos-api/src/xbios/iorec.rs`, `OFF_*`).
- **[Anwender 2026-10-28]** Flush als gemessener Fall auf beiden Seiten:
  `io->ibufhd = io->ibuftl` schreiben, danach `Bconstat(2) == 0`. Braucht
  „durch den Rueckgabezeiger schreiben“ in TASK/TOOL16. **[XBIOS01]** siehe
  Flush oben.

---

## Dependencies

Requires: `RX_RING` / Pump (Tasten), `Kbshift`-Zustand (`shifty`), die
prozessprivate oder gemeinsame RW-Seite (E1). Used by: Terminalprogramme,
Spiele/Editoren (Tastaturpuffer leeren), MIDI-Programme, der `kbdvec`-Haken
(EmuTOS uebergibt `a0 = &ikbdiorec`, `bios/aciavecs.S:412`; NewTOS gibt `x1 = 0`,
`docs/api/xbios/22-kbdvbase.md` D2a/O2 — E6).

## Related data structures

- `docs/api/xbios/_state.md §3` — Layouts (`IOREC` stimmt; `EXT_IOREC`-Groesse
  "24 bytes" ist falsch, siehe oben).
- `docs/api/structures/iorec.md` — nennt den IKBD-Ring "WORD-sized entries": falsch,
  der Eintrag ist 4 Byte (`bios/ikbd.c:163,269`).

## Cross-references

Atari Compendium / Profibuch: nicht in `references/`, Abgleich ausstehend.

---

## Entscheidungen (Anwender 2026-10-28)

- **E1 = 1:** ein gemeinsamer Ring ersetzt `RX_RING`. Der Ringinhalt ist
  Programmdaten; der Kernel validiert `head`/`tail` (und `buf`/`size`, E4)
  bei **jeder** Nutzung.
- **E2 = 32 Bit:** `shifty<<24 | scancode<<16 | ascii`, Shift zum
  Push-Zeitpunkt; schliesst BIOS11 (TASK/BIOS11.md) mit.
- **E3 = (a) vorerst, dokumentiert:** eine Warnung ist nicht moeglich (es
  kommt kein Kernelaufruf). Erhebung: pollt ein portiertes Programm den Ring
  ohne Trap? Wenn ja → (b), die treuere Variante (EmuTOS fuellt den Ring per
  Interrupt, unabhaengig von Traps).
- **E4 = nach Erhebung:** keine Nutzer, die `buf`/`size` ersetzen → (a)
  kernel-eigener Puffer; bemerkt der Kernel beim naechsten Trap ein
  geaendertes `buf`/`size`, warnt er einmal (nie still) und stellt den
  Standard wieder her. Nutzer vorhanden → (b) nur fuer Puffer in global
  abgebildetem Speicher, bei jeder Nutzung validiert, beim Ende des
  besitzenden Prozesses auf den Standardpuffer zurueckgesetzt.
- **E5 = (a):** Records fuer Geraete 0 und 2 vorhanden und leer, wie das
  Orakel.
- **Erhebung 2026-10-28** (`docs/analysis/iorec-survey-2026-10-28.md`): kein
  portiertes Programm ruft `Iorec`; keines pollt den Ring, keines ersetzt
  `buf`/`size` (nur Quelltext durchsucht, keine Binaerprogramme). Damit nach
  der Regel des Anwenders: **E3 = (a)** bleibt, **E4 = (a)** kernel-eigener
  Puffer, einmalige Warnung und Wiederherstellung des Standards beim
  naechsten Trap nach einer Aenderung von `buf`/`size`.
- **E6 = ja**, entschieden im gemeinsamen Haken-ADR (TASK/BIOS03.md).

### Optionen, wie vorgelegt (Begruendung)

**E1 — Ort und Kopplung der IKBD-Schlange (Kern; bestimmt F1-Ort, F3).**
1. *Lebender gemeinsamer Ring:* der IOREC-Ring **ersetzt** `RX_RING` (alle
   Produzenten/Verbraucher inkl. AES laufen darueber), in einer neuen,
   fuer alle Prozesse RW abgebildeten Seite (die private Seite ist je Prozess,
   EmuTOS' Ring global; eine neue Seitenvergabe unter 2 MiB ist noetig —
   `mmu.rs` hat dort heute nur Stub- und Privatseite). Flush, Direktlesen,
   Wiederholung verhalten sich wie EmuTOS. Kosten: groesster Eingriff
   (Schlange, AES, `Fread`, Selbsttests), Programme koennen sich gegenseitig
   die Schlange verderben (wie auf TOS), Kernel muss jeden Zugriff validieren.
2. *Spiegel je Prozess in der privaten RW-Seite, mit Rueckkopplung:* die
   Kernel-Schlange bleibt; an jedem Eintritt in einen Verbraucher und beim
   Trap-Rueckkehr wird gesynct, ein vom Programm gesetztes `head == tail`
   leert die Kernel-Schlange. Kosten: zwei Wahrheiten, Sync-Zeitpunkte
   muessen lueckenlos sein, gesyncte Schlange ist nur bei Traps aktuell
   (siehe E3); Fehler = gestohlene/duplizierte Tasten.
3. *Nur Lese-Schnappschuss, Schreiben toleriert aber ohne Wirkung:* Seite RW,
   Kernel ueberschreibt Kopf/Schwanz bei jedem Sync. Kein Absturz, Lesen
   plausibel, **aber der Flush wirkt nicht** (Abweichung F3.1 vom Oracle).
   Billigste Variante; widerspricht dem "1:1 EmuTOS"-Ziel am deutlichsten.

**E2 — Eintragsbreite (nur falls E1 = 1 oder 2):** 32-Bit-Wort wie EmuTOS
(`shifty` zum Push-Zeitpunkt; muss an der Push-Stelle `kernel/src/ikbd.rs:287@push_rx`
und fuer die Wiederholung `:334@poll_repeat` mitgefuehrt werden) — oder das
bestehende 16-Bit-Wort behalten (dann ist der Ring fuer Direktleser nicht
EmuTOS-gleich). Empfehlung der Quelle: 32-Bit; die Wahl bleibt beim Anwender.

**E3 — Zustellung ohne Trap:** (a) hinnehmen und dokumentieren: ein Programm,
das den Ring pollt und nie einen Trap macht, haengt (heutiger Zustand der
Pump; reale Programme rufen meist `Bconstat`/AES/Vsync-artig); (b) die
200-Hz-Timer-Pump (`kernel/src/host_env.rs:513@pump_devices_irq`) so erweitern,
dass sie auch Tastenbytes und Wiederholung in den Ring schreibt — das ist ein
Eingriff in das INPUT01-Gate (`crate::input_gate`), dessen Grund die heutige
Zeiger-Beschraenkung ist. Folge (a): einige Direktpoller laufen nie; Folge (b):
Neuaufbau des Input-Gates.

**E4 — Vom Programm ersetztes `buf`/`size`:** (a) Kernel besitzt Puffer und
Groesse; ein ersetztes `buf` wird bei der Validierung zurueckgesetzt
(Abweichung: "Puffer vergroessern" wirkt nicht, Ring bleibt aber konsistent);
(b) EmuTOS-treu: ersetztes `buf`/`size` gilt, der Kernel schreibt ueber den
vom Programm gelieferten Zeiger (mit `uaccess`-Fixup, Fehler -> Eintrag
verwerfen). (b) ist 1:1, (a) vermeidet, dass ein Programm den Kernel in fremden
Speicher schreiben laesst. Betrifft v.a. Geraet 0; fuer 1/2 praktisch ohne Folge.

**E5 — Geraete 0 und 2 bis ein Treiber da ist (TASK/IO01):** (a) Records
vorhanden und **dauerhaft leer**, konsistent zu `Bconstat(1)=0`/MIDI absent
und zu den Oracle-Werten I01-I03 (ein Treiber fuellt sie spaeter); (b) `-1`
fuer 0 und/oder 2 zurueckgeben (ehrlich "nicht vorhanden", aber **weicht vom
Oracle ab**, der Zeiger liefert, und bricht Programme, die den Rueckgabewert
nicht auf `-1` pruefen).

**E6 — `kbdvec`-Hakenargument:** mit `Iorec(1)` kann `x1` des Hakens (D2a,
`docs/api/xbios/22-kbdvbase.md`) auf den IOREC-Zeiger des Prozesses gesetzt
werden (EmuTOS `a0`, `bios/aciavecs.S:412`) statt `0`. Ja/nein; haengt an E1.

---

## Out-of-scope observations

- `docs/api/structures/iorec.md`: "WORD-sized entries" falsch (4-Byte-IKBD-Eintraege).
- `docs/api/xbios/_state.md §3`: `EXT_IOREC` "24 bytes" falsch (33 Byte Felder, 34 auf m68k).
- `Bconin(2)` faltet `shifty` beim Lesen, EmuTOS beim Push (`pack_console_key`);
  schon heute eine Abweichung, unabhaengig von `Iorec`. **Erledigt mit XBIOS01
  (E2, BIOS11).**
- [XBIOS01] qemu-virt: ein serielles Byte kommt als Eintrag mit Scancode 0 und
  Shift 0 in den Ring; EmuTOS' `push_ascii_ikbdiorec` sucht Scancode und Shift
  in den Tastaturtabellen (`bios/ikbd.c:192-231`). Nur qemu-virt
  (`CONF_SERIAL_CONSOLE`-Profil), unveraendert durch XBIOS01.
- [XBIOS01] `Rsconf` schreibt in EmuTOS `baudrate` und `flowctrl` in den
  Geraet-0-Record und liest `Rsconf(-2)` von dort
  (`references/emutos/bios/serport.c:222-240`, SCC `:780-800`); NewTOS fuehrt
  die Werte in eigenen Zellen (`tos-api/src/xbios/system.rs:263@rsconf`), der
  Record bleibt bei `baudrate 1`/`flowctrl 0`. Nach `Rsconf(speed)` liest ein
  Programm in `Iorec(0)` also den alten Wert. Nicht gemessen, ausserhalb dieses
  Auftrags (gehoert zu `Rsconf`, XBIOS-CHARDEV).

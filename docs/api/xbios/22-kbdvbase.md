# `Kbdvbase` (XBIOS 0x22) — der Tastatur-/IKBD-Vektor

> **Status: FREIGEGEBEN (Anwender 2026-10-04)** — D1–D8 wie empfohlen: Tabelle je Prozess in TPA-Seite 0 bei `0x9A0` (D1); AAPCS64, `x0` = Scancode/Paketzeiger, `x1` = 0 bei `kbdvec` bis `Iorec(1)` (D2/D2a); Ring 64 Eintraege je Prozess + Upcall-Kette (D3/D3a); `ikbdsys`-Wechsel-Stopp nachbauen (D4); `drvstat` = 0 (D5); Syscall-Nummer vergibt der Lead beim Bau aus dem freien internen Bereich (D6); Minimum jetzt = `kbdvec` + gueltige Stubs in allen Slots, uebrige Haken Folgeticket, Luecke in STATUS „Blocked“ (D7); Haken verschwindet mit dem Prozess, in COMPATIBILITY.md (D8). Vorherige Kennzeichnung: ENTWURF (KEYS02, Schritt 1 / Regel 5). Bei der Freigabe
> war nichts davon gebaut; **der Kernteil (D7-Minimum) ist seit 2026-10-04 gebaut —
> Teil C** (Syscall-Nummer D6 = `Ssystem` Modus **2002**, O1 geklaert). Teil A (EmuTOS-Referenz) ist aus dem Quelltext belegt; Teil B
> (NewTOS-Abbildung) ist ein **Vorschlag**, jede Stelle mit `ENTSCHEIDUNG D<n>`
> braucht ein Ja/Nein des Anwenders (Liste am Ende).
>
> Quelle: EmuTOS @ `1a7484ab` (`references/emutos`), FreeMiNT
> (`references/freemint`), Atari Compendium als tos.hyp (WebFetch, siehe
> "Querverweise"). Bearbeitet 2026-10-04 durch den Researcher; ersetzt die
> Phase-00-Fassung dieser Datei (Korrekturen: Abschnitt "Korrekturen an der
> Vorfassung").
>
> **Vertrag vs. Interna** (`docs/COMPATIBILITY.md`): Vertrag ist, was ein
> Programm beobachtet: Trap-ID, der Rueckgabezeiger, Reihenfolge der Slots,
> `kbdvec` direkt vor Slot 0, Registerinhalt bei Aufruf, Bit 7 = Break,
> Verkettbarkeit (Slot lesen, sichern, spaeter rufen). Intern (frei) sind ACIA,
> MFP-Interrupt, Paketpuffer-Layout in BSS.

---

# Teil A — EmuTOS-Referenz (belegt)

## A1. Aufruf und Rueckgabe

`Kbdvbase()` = XBIOS 0x22, keine Parameter (`bios/xbios.c:716-719`:
`return (LONG)&kbdvecs;`), Slot 0x22 der Tabelle `xbios_vecs[]`
(`bios/xbios.c:1337`). Ergebnis in D0: Zeiger auf `kbdvecs`. Stabil (Zeiger
aendert sich nie, Struktur liegt in BSS: `bios/aciavecs.S:126-141`).

## A2. Speicherlayout rund um den Zeiger

`bios/aciavecs.S:36-57` (Kommentar) und `:126-141` (Definition). Der
Rueckgabezeiger zeigt auf `midivec`; **davor** liegt `kbdvec`, und mit
`CONF_WITH_EXTENDED_MOUSE` davor `mousexvec`:

```
  kbdvbase-8   mousexvec   (nur #if CONF_WITH_EXTENDED_MOUSE, aciavecs.S:126-128)
  kbdvbase-4   kbdvec      (aciavecs.S:129, "must be right before _kbdvecs")
  kbdvbase+0x00 midivec    MIDI-Eingang               (aciavecs.S:131)
  kbdvbase+0x04 vkbderr    IKBD-Fehler                (:132)
  kbdvbase+0x08 vmiderr    MIDI-Fehler                (:133)
  kbdvbase+0x0C statvec    IKBD-Status                (:134)
  kbdvbase+0x10 mousevec   Maus                       (:135)
  kbdvbase+0x14 clockvec   IKBD-Uhr                   (:136)
  kbdvbase+0x18 joyvec     Joystick                   (:137)
  kbdvbase+0x1C midisys    Haupt-MIDI-Vektor          (:138)
  kbdvbase+0x20 ikbdsys    Haupt-IKBD-Vektor          (:139)
  kbdvbase+0x24 ikbdstate  Byte, Paketstatus          (:140)
  kbdvbase+0x25 kbdlength  Byte, Restbytes im Paket   (:141)
```

Alle Slots sind 4-Byte-Zeiger (`.ds.l`) auf m68k. `struct kbdvecs`
(`include/biosdefs.h:105-116`) enthaelt nur die neun dokumentierten Slots;
`kbdvec`, `mousexvec`, `ikbdstate`, `kbdlength` sind **nicht** in der C-Struktur.

Konfigurationsabhaengigkeit: `mousexvec` nur mit `CONF_WITH_EXTENDED_MOUSE`
(`include/config.h:1496-1497`: Standard 1; laut `docs/api/_emutos-config.md:183,255`
in manchen Maschinenprofilen 0). `midisys` wird nur bei `CONF_WITH_MIDI_ACIA`
gesetzt (`aciavecs.S:180-187`), `ikbdsys` nur bei `CONF_WITH_IKBD_ACIA`
(`:188-195`, Standard 1: `include/config.h:1001-1002`), `clockvec` nur bei
`CONF_WITH_IKBD_CLOCK` (`:169-178`, Standard 1: `include/config.h:1008-1009`); sonst
bleibt der Slot `just_rts` bzw. (midisys/ikbdsys) unbelegt = 0 (BSS). Das
Falcon-Profil (Zielbild, `_emutos-config.md`) hat ACIA, IKBD-Uhr und
erweiterte Maus an.

`ikbdstate` (Compendium: `drvstat`): Quelltext-Kommentar `aciavecs.S:54-57`;
tos.hyp fuehrt `drvstat` als **dokumentiertes** Feld der `KBDVBASE` direkt hinter
`ikbdsys` ("non-zero when the IKBD is in the process of sending a packet",
`xbios_structures.html#KBDVBASE`). EmuTOS-Kommentar: ungleich 0 = ein Paket wird
gerade empfangen (nicht die Anzahl, `aciavecs.S:54-57`).

## A3. Vorgabebelegung (`_init_acia_vecs`, `aciavecs.S:146-223`)

| Slot | Vorgabe | Beleg |
|---|---|---|
| `kbdvec` | `_kbdvec` -> `kbd_int` | `:154-158`, `_kbdvec` `:372-376` |
| `midivec` | `_midivec` (schreibt in MIDI-iorec) | `:160-164`, `:315-332` |
| `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `joyvec` | `just_rts` (`rts`) | `:148,165-168,179`, `just_rts:` `:377` |
| `clockvec` | `_clockvec` (`bios/clock.c:969`) bzw. `just_rts` | `:169-178` |
| `midisys` | `_midisys` (nur MIDI-ACIA) | `:180-187` |
| `ikbdsys` | `_ikbdsys` (nur IKBD-ACIA) | `:188-195` |
| `mousexvec` | `just_rts` (VDI ersetzt sie: `vdi/vdi_mouse.c:611`) | `:150-152` |

`ikbdstate` wird auf 0 gesetzt (`:206-207`). Danach IKBD-Daten-Register leer lesen
(`:210-214`) und der ACIA-Interrupt eingehaengt (`_mfpint(6, _int_acia)`,
`:217-221`). Die Belegung der Slots geschieht in `init_acia_vecs()`
(`bios/bios.c:457-458`), nach `kbd_init()` (`:453-454`).

## A4. Wann wird welcher Vektor mit was gerufen

Kette (alles im ACIA-Interrupt, IPL 6, endet mit `rte`):

```
MFP-Int 6 -> _int_acia (aciavecs.S:249)
  retet d0-d3/a0-a3 (:253-259) ; Kommentar :250-252: es gibt keine Spezifikation,
  welche Register die Vektoren verwenden duerfen; d0-d1/a0-a1 reichten theoretisch
  loop (:262-273):
     [MIDI]  move.l midisys,a0 ; jsr (a0)                     (:263-266)
     [IKBD]  move.l ikbdsys,a0 ; jsr (a0)                     (:267-270)
     solange GPIP Bit 4 == 0 (Interrupt steht noch) wiederholen   (:271-273)
  End-of-Interrupt (:274), Register zurueck (:277-283), rte
```

* `ikbdsys` (Vorgabe `_ikbdsys`, `:382-400`): liest ACIA-Status/-Daten selbst
  (`ikbd_acia_stat/data`), wenn Daten da sind, ruft `ikbdraw` mit **d0 = Datenbyte**
  (`:391`); bei Overrun (Statusbit 5) liest es das Datenbyte nochmal und ruft
  **`vkbderr` mit d0 = Inhalt des ACIA-Datenregisters** (`:393-397`).
  `ikbdsys` bekommt selbst keine Argumente.
* `ikbdraw` (`:407-414`): ist `ikbdstate == 0` und Byte `< 0xF6`, ist es ein
  **Tastaturbyte** -> `a0 = &ikbdiorec`, `d0 =` Byte, Sprung (`rts`-Trick, kein
  `jsr`) in **`kbdvec`**. Die Rueckkehr der `kbdvec`-Routine geht daher direkt
  zum Aufrufer von `ikbdraw` (`_ikbdsys`).
* Bytes `0xF6..0xFF` beginnen ein Paket (`begin_packet` `:416-432`, Tabellen
  `:433-437`); weitere Bytes sammelt `in_packet` (`:439-463`); beim letzten Byte
  `got_packet` (`:465-475`) -> `action_table` (`:476-482`):

| Paket | Aktion | Vektor | Zeiger `a0` (UND als Langwort auf dem Stack) |
|---|---|---|---|
| `F6 a1..a7` | `kbd_status` `:484-487` | `statvec` | `kbdbuf+1` (hinter dem Header, 7 Bytes) |
| `F7 0b xh xl yh yl` | `kbd_abs_mouse` `:488-489` | `mousevec` | `kbdbuf+1` (hinter dem Header) |
| `F8..FB dx dy` | `kbd_rel_mouse` `:490-492` | `mousevec` | `kbdbuf` (**inkl.** Header-Byte) |
| `FC yy MM dd hh mm ss` | `kbd_clock` `:493-496` | `clockvec` | `kbdbuf+1` (6 Bytes) |
| `FD j0 j1` | `kbd_joys` `:513-514` | `joyvec` | `joybuf` = `[j0, j1]` |
| `FE bj` | `kbd_joy0` `:515-517` | `joyvec` | `joybuf` = `[0xFE, bj, (alter 3. Wert)]` |
| `FF bj` | `kbd_joy1` `:518-519` | `joyvec` | `joybuf` = `[0xFF, (alter 2. Wert), bj]` |

  Aufruf: `kbd_jump_vec` (`:527-533`): `move.l a0,-(sp); jsr (a1); addq.l #4,sp;
  clr.b ikbdstate`. Der Zeiger liegt also **in a0 und als C-Argument auf dem
  Stack**; Rueckkehr per `rts`; Rueckgabewert wird nicht ausgewertet. Danach
  wird `ikbdstate` geloescht. **Undokumentiert, "bis mindestens TOS 3.06"**
  (`aciavecs.S:508-510`): beim Eintritt in `joyvec` steht in d0.b das zuletzt
  empfangene Datenbyte. (Dass d0 auch bei den anderen Paketvektoren noch das
  letzte Byte hat, folgt aus dem Codepfad `:465-475` -> `:527`, ist dort aber
  nicht als Zusage kommentiert: **gelesen, nicht Vertrag**.)
* `midisys` (`:295-312`): liest MIDI-ACIA, ruft `midivec` mit **d0 = Datenbyte**
  (`:302-304`), bei Overrun `vmiderr` mit d0 = Datenregister (`:307-310`).
* tos.hyp (Compendium) dazu: Adresse des Pakets bei `clockvec`/`joyvec` in A0
  **und** auf dem Stack, Routinen mit `RTS` beenden, wenn moeglich nicht laenger
  als 1 ms (`xbios_structures.html#KBDVBASE`); vor einer Aenderung eines Vektors
  muss `drvstat == 0` sein, Interrupts sperren und erneut pruefen
  (`xbios_interface.html`, Abschnitt Kbdvbase). *(Beide Stellen wurden ueber
  WebFetch zusammengefasst gelesen, nicht im Wortlaut geprueft.)*

### Der Standard-`kbdvec`

`_kbdvec` (`aciavecs.S:372-376`): `move.w d0,-(sp); jsr _kbd_int; addq.l #2,sp;
rts` -> `void kbd_int(UBYTE scancode)` (`bios/ikbd.c:725`). Das Byte ist ein
**IKBD-Scancode**: Bit 7 gesetzt = Break (`KEY_RELEASED 0x80`, `ikbd.c:48`,
`:777`), sonst Make. `kbd_int` (`ikbd.c:725-910`) macht der Reihe nach:

1. Ctrl-Alt-Del / Ctrl-Alt-RShift-Del -> `warm_reset()`/`cold_reset()` (`:734-747`; exact key-state conditions in `docs/api/bios/_reset-warm-cold.md` §2);
2. Scancodes `0x37,0x5e,0x5f,0x59,0x5a,0x5c,0x5d` (`CONF_WITH_EXTENDED_MOUSE`) ->
   `mousexvec(scancode)` und **Rueckkehr** (`:748-760`);
3. Break (`:777`): Modifier-Bits loeschen (`:779-806`), Alt-Release mit
   Alt-Numpad (`:788-799`), Home/Insert-Break stoppt Wiederholung (`:807-814`),
   anderer Break: `if (scancode_only == kb_last_actual) kb_ticks = 0;` (`:815-817`);
   Mausemulation (`handle_mouse_mode`, `:821`); Rueckkehr — **ein Break landet
   nie in der iorec**;
4. Make: Modifier setzen `shifty` (`:825-870`); Nicht-Modifier: `kb_last_actual =
   scancode` (`:875`), `convert_scancode` (`:887`), `kb_ticks = kb_initial`
   (`:891`), Tastenklick, `kb_last` und `kb_last_ikbdsys = kbdvecs.ikbdsys`
   merken (`:898-903`), und `push_ikbdiorec(kb_last.key)` (`:909`, Format
   `shifty<<24 | scancode<<16 | ascii`, `ikbd.c:428-434`; iorec-Push mit
   Voll-Verwerfen `:157-173`).

## A5. Tastenwiederholung und `ikbdsys`

`do_key_repeat()` (`ikbd.c:451-528`, 50 Hz aus `kb_timerc_int` `:533-536`):

* `:453`: `if (kb_ticks > 0 && kbdvecs.ikbdsys != kb_last_ikbdsys)` -> `kb_ticks = 0`,
  Rueckkehr. Kommentar `:454-470`: Anlass war FreeMiNT, das seinen `ikbdsys`
  einhaengt, waehrend eine Taste gehalten ist. **Nur `ikbdsys` wird verglichen,
  nicht `kbdvec`**: ein Spiel, das `kbdvec` ersetzt, stoppt die Wiederholung
  nicht dadurch.
* Die Wiederholung legt das Wort **direkt in die iorec** (`push_ikbdiorec`,
  `:524`) und ruft **nie `kbdvec`** auf. Ein `kbdvec`-Haken sieht daher
  Make und Break, **keine** Wiederholungen (das Software-Repeat entsteht erst
  *hinter* `kbd_int`).
* Die Wiederholung entsteht nur, wenn `kbd_int` lief (`kb_ticks` wird nur
  dort gesetzt: `:891`, `kb_last*` `:898-903`). Ein Haken, der den Standard-
  `kbdvec` **nicht** ruft, bekommt also: keine iorec-Eintraege, keine
  Wiederholung, keine Aktualisierung von `shifty`/`Kbshift` (`:779-870`), keinen
  Reset-Check (`:732-742`).

## A6. Was ein Programm tun muss (aus dem Code abgeleitet, **nicht** im Quelltext als Anleitung)

*Die folgende Abfolge ist Schlussfolgerung aus A2-A5, kein EmuTOS-Zitat.*

1. Im Supervisor-Modus `p = Kbdvbase()`, `old = ((void**)p)[-1]` (der `kbdvec`-Slot)
   lesen und sichern. (Die API gibt keinen alten Vektor zurueck; der Quelltext
   enthaelt keinen Mechanismus, der das uebernimmt.)
2. Eigene Routine in den Slot schreiben. Die Routine bekommt **d0.b =
   Scancode (Bit 7 = Break), a0 = iorec**.
3. Wer die normale Tastaturbehandlung (BIOS-Warteschlange `Bconin`, AES,
   Shift-Zustand, Wiederholung) behalten will, **verkettet**: am Ende zum
   gesicherten `old` springen mit unveraendertem d0/a0 (der Aufruf kam per
   `rts`-Trick, `old` muss mit `rts` zum selben Aufrufer zurueckkehren). Ohne
   Verkettung sieht der restliche TOS-Tastaturweg diese Tasten nie (A5).
4. Beim Programmende (`Pterm`, besser ueber den `etv_term`-Vektor, `bdos/proc.c:
   606-607`) den alten Wert zurueckschreiben. Muster in EmuTOS selbst:
   `vdimouse_exit` stellt `statvec` aus `old_statvec` wieder her
   (`vdi/vdi_mouse.c:638-639`); `wheel_int` verkettet am Ende per
   `move.l _old_statvec,-(sp); rts` (`vdi/vdi_asm.S:476`). Die Haken rufen
   keine Betriebssystemfunktionen auf (Interrupt-Kontext).
5. Wer **nicht** verkettet, bekommt trotzdem jede Make/Break-Meldung genau
   einmal je Byte; Wiederholungen entstehen nicht (A5).

## A7. Welche TOS-Versionen haben `kbdvec`

* EmuTOS: immer (`aciavecs.S:129`, `ikbdraw` `:407-414`). Der Kommentar nennt es
  "undocumented feature of TOS >= 2.0" (`aciavecs.S:52`).
* FreeMiNT: `has_kbdvec = true` genau wenn `tosvers >= 0x200` oder EmuTOS
  (`references/freemint/sys/keyboard.c:1938-1941`); "TOS < 2.00 doesn't know
  about this vector" (`sys/arch/init_intr.c:94-98`). Fuer solche TOS haengt
  MiNT stattdessen `ikbdsys` ein und ruft den `KBDVEC`-Slot selbst
  (`init_intr.c:79-87`, `sys/arch/acia.S:146-149`).
* **TT TOS 3.06 (0x306): ja, ABGELEITET** aus der Regel `tosvers >= 0x200`; kein
  Quelltext sagt ausdruecklich "3.06 hat `kbdvec`". Belegt ist nur, dass der
  EmuTOS-Autor das Verhalten des `joyvec`-d0-Details "bis mindestens TOS 3.06"
  kennt (`aciavecs.S:508`) — das betrifft `joyvec`, nicht `kbdvec`.
  **Gemessen 2026-10-04 (KEYS02; eingetragen 2026-10-26, schliesst O3): ja.**
  Im ROM `tos306de.rom` liefert XBIOS `$22` (`$E03A96`)
  `move.l #$0000106A,d0 / rts`, und `ikbdraw`
  (`$E032DA`) springt mit `move.l $1066.w,-(sp) / rts` ueber `kbdvec` =
  `Kbdvbase()-4` — derselbe `rts`-Trick wie EmuTOS `aciavecs.S:407-414`. Zur
  Laufzeit (Hatari 2.6.1 TT, Debugger) steht in `$1066` `$00E033EA`, TOS 3.06s
  `kbd_int` (Shift-Zustand). Belege
  `measurements/measurement-2026-10-04-keys02-doom/RUNINFO.TXT:75-92`,
  `measurements/measurement-2026-10-04-keys02-doom/hatari/o3-tos306-debugger.log:20,30,34`; Doom meldet dort
  `Kbdvbase 0000106a, kbdvec was 00e033ea`
  (`measurements/measurement-2026-10-04-keys02-tt/RUNINFO.TXT:20`).
* Ausnahme-Hinweis: FireTOS und CT60-TOS-2.xx rufen `kbdvec` **nicht** aus ihrem
  `ikbdsys` auf (`init_intr.c:104-111`) — dort ist ein `kbdvec`-Haken wirkungslos.
  Auf diesen ROMs ist `ikbdsys` der verlaessliche Weg.
* In tos.hyp wurde `kbdvec` in den abgerufenen Seiten nicht gefunden (nur
  `drvstat`, kein Slot -4): bestaetigt "undokumentiert".

## A8. Verhalten beim Programmende, wenn ein Haken installiert bleibt

* **EmuTOS:** Nichts. `xterm` ruft `etv_term` (`bdos/proc.c:606-607`), setzt
  `run` zurueck und `ixterm` schliesst Dateien, gibt Verzeichnisse frei und
  `free_all_owned` (`bdos/proc.c:110-140`) — keine Stelle fasst `kbdvecs` an. Eine
  Suche ueber `*.c/*.h/*.S` im ganzen Baum findet `kbdvecs`/`mousevec`/
  `statvec`/`joyvec`/`ikbdsys` nur in `bios/aciavecs.S`, `bios/ikbd.c`,
  `bios/mouse.c:117,120`, `bios/xbios.c:718`, `bios/clock.c`, `vdi/vdi_mouse.c`,
  `bios/amiga*.{c,S}`, `bios/lisa.c`, `bios/bios.h`, `include/biosdefs.h`; nie in
  `bdos/`. **Schluss (gefolgert aus dem Fehlen):** der Zeiger bleibt auf den
  nun freigegebenen (und ggf. neu vergebenen) Speicher stehen; der naechste
  Tastendruck springt dorthin. Der Programmierer muss selbst zuruecksetzen
  (daher A6.4).
* **FreeMiNT:** Der Kernel verfolgt Haken einzelner Programme **nicht**; es gibt
  keine Zeile in `sys/` ausser `keyboard.c`, `sys/arch/init_intr.c`, `sys/arch/acia.S`,
  `sys/arch/intr.S`, `dev-mouse.c` und den USB-Treibern, die `kbdvecs` anfasst
  (Suche `kbdvec|ikbdsys|kbdvbase` in `references/freemint/sys`). Was MiNT tut:
  * Es haengt **seinen** `kbdvec_handler` ein, mit XBRA-Kopf davor
    (`sys/arch/intr.S:629-639`; `install_vector`, `init_intr.c:48-57`;
    Eintrag `:101`), und sichert den alten Wert in `old_kbdvec`. Der Handler
    ruft `ikbd_scan(scancode, kbd_iorec)` und kehrt per `rts` zurueck
    (`intr.S:638-661`), **ohne** an `old_kbdvec` weiterzuleiten, ausser fuer
    "verwaiste" Breaks (`chain_oldkeys`, `intr.S:671-683`, Ausloeser
    `keyboard.c:1030-1043`).
  * Ein Programm, das *nach* MiNT seinen Haken einhaengt und `old` (= MiNTs
    Handler) verkettet, funktioniert; das Aushaengen bleibt Sache des Programms.
    (*Gefolgert*, nicht als Zusage im Quelltext.)
  * Beim **Ende von MiNT selbst** wird die ganze Tabelle auf den gesicherten
    Stand zurueckgesetzt (`restore_TOS_vectors`, `init_intr.c:243-264`),
    mit dem Kommentar "BUG: we should restore *all* vectors, not just the ones
    that MiNT caught" (`:238-241`).
  * `/dev/mouse` sichert `mousevec`/`joyvec` beim Oeffnen und stellt sie beim
    Schliessen wieder her (`sys/dev-mouse.c:190-231`) — das ist eine
    Geraete-Schliessen-Aktion, kein Prozessende-Hook fuer fremde Programme.
  * Die USB-Treiber (`udd/hid/mouse*/usb_mouse.c:507-522`,
    `sys/usb/src.km/udd/hid/tablet/usb_tablet.c:1010-1026`) senden Tasten/Maus selbst ueber die
    `KBDVEC`-Struktur, wenn GEMDOS >= 0x19: das ist der FreeMiNT-Praezedenzfall,
    dass ein **nicht-ACIA-Treiber** die Vektoren aufruft (wie es NewTOS' HID-
    Uebersetzer tun wuerde). FreeMiNT-Struktur: `struct kbdvbase` mit `drvstat`
    hinter `ikbdsys` (`sys/mint/struct_kbdvbase.h`).

## A9. Praezedenz: EmuTOS ruft die Vektoren auch aus nicht-ACIA-Quellen

Amiga: `move.l _kbdvecs-4,a0 ; jsr (a0)` ruft `kbdvec` (`bios/amiga2.S:201-202`),
`call_ikbdraw` (`aciavecs.S:556-568`), `call_mousevec` (`:538-554`),
`call_joyvec` (`:570-590`, nur Amiga). Ein Treiber ohne ACIA erzeugt also die
IKBD-Bytes und laeuft durch dieselben Slots — genau das Zielmodell fuer NewTOS.

---

# Teil B — NewTOS-Abbildung (VORSCHLAG, braucht Freigabe)

## B0. Ist-Zustand (gelesen)

* `Kbdvbase` liefert `&KBDVECS`, neun `usize`-Slots, **alle 0**, in Kernel-
  Speicher (`tos-api/src/xbios/system.rs:63-98`), **kein** `kbdvec`-Slot davor.
  Kernel-Adresse, nur EL1 gemappt (`kernel/src/arch/aarch64/mmu.rs:82-83,179`;
  Diagnose `TASK/PORT06-KEYS-DIAGNOSIS.MD` Frage 4).
* Der Dekoder `kernel/src/ikbd.rs:feed/decode_key` ruft keinen Vektor; er legt
  Makes in `console::push_rx` ab, verwirft gewoehnliche Breaks
  (`ikbd.rs:148-190`, Make-Zweig und `_ =>`-Break), Maus geht direkt nach `vdi::feed_relative`
  (`ikbd.rs:125`), `0x37/0x59/...` nur an das kernel-interne `mousexvec`
  (`ikbd.rs:148-163`).
* Die Wiederholung (KEYS01) legt Woerter ebenfalls direkt in `push_rx`
  (`ikbd.rs:222-231`) — **wie EmuTOS** (A5), sie ruft keinen Vektor.
* Eingabe wird **gepollt**: `host_env::pump_input` (`kernel/src/host_env.rs:84-123`)
  -> `usb::poll()` -> `ikbd::feed` -> `poll_repeat`, aus jeder AES-Warteschleife,
  jedem blockierenden Lesen, `Bconstat` bei leerer Schlange (KEYS01-STATUS).
  Kein Interrupt. *(Nachtrag INPUT01, 2026-10-08: zusaetzlich pumpt der
  200-Hz-Timer-Interrupt, wenn er ein Programm an EL0 unterbricht und kein
  Upcall/kein angehaltener AES-Aufruf laeuft — `kernel/src/input_gate.rs`,
  abschaltbar mit `input=aes`. Er dekodiert aber **nur reine Zeigerbewegung**;
  ein Bericht mit Tastenbyte, Knopfwechsel oder Mausrad bleibt zurueckgehalten
  bis zur naechsten Pumpe im Trap-Pfad. Tastenbytes erreichen `route_key` und
  den Haken-Ring also weiter nur aus dem Trap-Pfad; aus dem Interrupt wird nie
  zugestellt.)*
* EL0-Rueckruf: `deliver_upcall` (`kernel/src/arch/aarch64/exceptions.rs:1310-1373`)
  sichert alle 30 GPRs + lr/elr/spsr/sp_el0 im Kernel, setzt `x0..x3` aus
  `UpcallDesc.args`, `elr = handler_va`, `lr = TRAMPOLINE_VA` (0x700, ruft
  `Psigreturn`), SP 16-ausgerichtet + 0x20 Red Zone. Zustellpunkt:
  Rueckkehr eines Syscalls nach EL0 (`complete_call_tail`, `exceptions.rs:
  538-557`; auch `:1041`), **nicht** im Interrupt. Geschachtelt bis
  `MAX_UPCALL_NEST = 8` (`tos-api/src/bdos/process.rs:74`), sonst `Pterm(-998)`.
  `Psigreturn` (`exceptions.rs:1390ff`) stellt den Kontext wieder her (inkl. des
  Syscall-Ergebnisses in `x0`).
* **Bestehender Verbraucher mit demselben Muster:** `vex_wheelv`
  (`tos-api/src/vdi/mouse.rs:421-428, 565-590, 960-973`): EL0-Vektor + Slot des
  Besitzers, Aufruf nur wenn der Besitzer der laufende Prozess ist, Rueckgabe
  des Vektors an die Vorgabe beim Prozessende (`release_wheel_vector`, aus
  `free_slot` `process.rs:991`). Begruendung dort: ein EL0-VA bedeutet nichts
  in einem fremden Adressraum.
* `post_upcall` hat **einen** Wartesteckplatz je Prozess, "last writer wins"
  (`process.rs:850-856`). Fuer Tastenereignisse reicht das nicht (siehe B4).
* Die erste Seite jedes Prozesses (TPA-Beginn, `[0, 0x1000)`) ist
  `UserRwExec` gemappt und enthaelt Basepage + Sysvars + Trampoline
  (`loader.rs:195-201`, `:730`); der Kernel liest/schreibt sie ueber den
  Kernel-Alias (`bios::chardev::current_conterm`, `chardev.rs:75-85`, liest
  `$484` des **laufenden** Prozesses; `KEYTBL_AREA_VA`, `loader.rs:~790`
  belegt `[0x768, 0x9A0)`). Frei bleibt `[0x9A0, 0x1000)`.
* App-ABI: nur noch LP64; ELFCLASS32 wird vom Lader abgewiesen
  (`loader.rs:99-101, 397-412`), der ILP32-Build-Pfad ist stillgelegt
  (`docs/MEMORY.md` ADR-020 Nachtrag 2026-09-04).

## B1. Wo die Tabelle liegt  (ENTSCHEIDUNG D1)

**Vorschlag:** je Prozess in der ersten TPA-Seite, neuer fester Bereich
`KBDVECS_AREA_VA = 0x9A0` (nach dem Keytbl-Bereich, vor `RESERVED_LOW_END`),
vom Lader beim Start mit den Standardwerten beschrieben (wie `populate_sysvars`).
`Kbdvbase()` liefert **den EL0-VA** dieses Prozesses (Ergebnis in `x0`, volle
64 Bit). Layout (LP64, 8-Byte-Slots, Reihenfolge wie EmuTOS):

```
  +0x00 mousexvec   (nur falls kernel-intern EL0-faehig; D3)
  +0x08 kbdvec      <- (void**)Kbdvbase()[-1]
  +0x10 midivec  <- Kbdvbase() zeigt hierher
  +0x18 vkbderr   +0x20 vmiderr   +0x28 statvec  +0x30 mousevec
  +0x38 clockvec  +0x40 joyvec    +0x48 midisys  +0x50 ikbdsys
  +0x58 drvstat (Byte, immer 0, D5)   +0x59 kbdlength (Byte, 0)
```

Begruendung (Schluss, nicht Zitat): Jeder Prozess hat einen eigenen Adressraum;
ein Vektor ist ein EL0-VA und nur dort gueltig (derselbe Grund wie
`vex_wheelv`, `mouse.rs:486-503`). Die Seite gehoert zur TPA und verschwindet mit
ihr — **ein haengender Zeiger ist damit unmoeglich** (strikt besser als EmuTOS,
A8). Der Kernel liest den Slot des laufenden Prozesses wie `current_conterm()`
ueber den Alias (Praezedenz). Vorgabe-Werte siehe B3.
Nachteil/Grenze (muss der Anwender kennen): ein TSR/Tastaturerweiterer, der per
`Ptermres` einen Haken fuer *alle spaeteren* Programme installieren will, geht
nicht — er koennte nirgends laufen, auch nicht mit globaler Tabelle (anderer
Adressraum). Keine Rolle fuer Doom.
Alternative (verworfen): globale Kerneltabelle mit Besitzer-Slot wie beim Rad.
Aber dann liest EL0 eine Kernel-Adresse (Mapping noetig) und Kbdvbase-Chaining
("alten Wert merken") braeuchte einen Kernel-Zustand je Besitzer ohne Gewinn.
ILP32: nicht unterstuetzt (Lader weist ELFCLASS32 ab); der Slot-Typ ist `u64`.

## B2. Aufrufkonvention fuer native aarch64-Programme  (ENTSCHEIDUNG D2)

Die Ersetzung fuer `d0`/`a0`. Alle Haken sind **AAPCS64-Funktionen**
`void h(uint64 a, uint64 b)` mit Rueckkehr per `ret` (Trampolin-`lr`):

| Vektor | x0 (ersetzt d0 / Stack-Arg) | x1 (ersetzt a0) | Quelle EmuTOS |
|---|---|---|---|
| `kbdvec` | Scancode in Bit 0-7, **Bit 7 = Break**, obere Bits 0 | Zeiger auf den IKBD-`IOREC` (D2a, seit XBIOS01) | `ikbdraw` `:407-414` |
| `midivec`, `vkbderr`, `vmiderr` | Byte | 0 | `:302-304, :393-397` |
| `statvec`, `mousevec`, `clockvec`, `joyvec` | Zeiger auf Paket (ersetzt Stack-Argument UND a0), Layout wie A4 | gleicher Zeiger | `kbd_jump_vec` `:527-533` |
| `midisys`, `ikbdsys` | — | — | siehe B6 |

`x2`/`x3` = 0. Aufrufer-gesicherte Register (x19-x28, sp, x29) erhaelt der
Haken nach AAPCS64; der Kernel sichert **alle** Register des unterbrochenen
Kontexts (`deliver_upcall`), der Haken darf also jedes Register veraendern —
strikt mehr Spielraum als `d2-d7/a2-a6` unter EmuTOS (`:250-259`).
D2a: `IOREC`-Zeiger. **[XBIOS01, 2026-10-29, E6]** `x1` = der IKBD-`IOREC`
(dieselbe Adresse wie `Iorec(1)`, `tos-api/src/xbios/iorec.rs:135@IKBD_IOREC_VA`,
gesetzt in `kernel/src/arch/aarch64/exceptions.rs:851@IKBD_IOREC_VA`), wie EmuTOS
`a0 = &ikbdiorec` (`bios/aciavecs.S:412`); die Tastenschlange **ist** seit
XBIOS01 dieser Ring (`docs/api/xbios/0e-iorec.md`). Vorher: `x1 = 0`, weil
`Iorec` nicht verdrahtet und die Schlange `console::push_rx` kein `IOREC` war.
Die Paketpuffer (`statvec` usw.) liegen in der ersten Seite (neuer Bereich
hinter der Tabelle, oder `PARMBLK`-aehnlich) und werden vor dem Aufruf
vom Kernel beschrieben.

**Verkettung** (Slot lesen, merken, spaeter aufrufen) funktioniert nur, wenn die
Vorgabe-Slots **gueltige EL0-Einsprungadressen** enthalten (B3). Verkettung ist
ein Tail-Call `br old` mit unveraendertem x0/x1.

## B3. Standard-Haken im Prozess (Stubs, EmuTOS-`just_rts`-treu)

Beim Laden schreibt der Lader (wie `write_upcall_trampoline`, `loader.rs:1136ff`)
ein kleines Stubfeld (z. B. `[0xA00, 0xA40)`), und die Vorgabe der Slots zeigt
darauf:

* `ret_stub` (ein `ret`) — Vorgabe fuer `vkbderr, vmiderr, statvec, mousevec,
  clockvec, joyvec, midivec, midisys, ikbdsys, mousexvec` (EmuTOS-Vorgabe ist
  `just_rts` fuer die meisten, `aciavecs.S:148-168,179`; fuer `midisys/ikbdsys/
  clockvec/midivec` ist es dort ein echter Treiber, hier gibt es kein ACIA/MIDI;
  `Initmous`/AES-Maus laufen intern ueber `vdi::feed_relative`).
* `kbdvec_default` — `mov x8,#<NR>; svc #1; ret`: ein **neuer interner
  Syscall (ENTSCHEIDUNG D6, Nummer vom Lead)**, der im Kernel die heutige
  `decode_key(byte)` (Modifier, ASCII, `push_rx`, Wiederholungsstart) ausfuehrt.
  Das ist das Gegenstueck zu `_kbdvec -> kbd_int`.

Damit ist "Haken ruft den Standard auf" = `br old` und "Haken ruft ihn nicht
auf" = EmuTOS-treu (A5): die BIOS-Schlange, `Kbshift`, Wiederholung und
Reset-Check bleiben aus (Reset-Check hat `ikbd.rs` ohnehin nicht).

## B4. Wann es laeuft und wie nichts verloren geht  (ENTSCHEIDUNG D3)

* Ort: `ikbd::feed()`, an der Stelle, wo ein Byte `< 0xF6` als Taste erkannt wird
  (jetzt `decode_key`, `ikbd.rs:~144`). Lauf im **gepollten Pump** (KEYS01),
  nicht im Interrupt; Zustellung am naechsten Syscall-Rueckgang nach EL0
  (`complete_call_tail`).
* Schnellpfad: Steht in `kbdvec` des laufenden Prozesses noch `kbdvec_default`
  (Adresse = Konstante), oder gibt es keinen laufenden Prozess (Desktop/Boot,
  `current() == null`, wie `current_conterm`), ruft der Kernel `decode_key`
  **direkt** wie heute -> normale Programme unveraendert (Abnahme).
* Sonst: Byte in einen Kernel-Ring je Prozess-Slot (Vorschlag 64 Eintraege, bei
  Ueberlauf aelteste/neueste verwerfen -> D3a) und **einen** Upcall mit
  `x0 = Byte`. Beim `Psigreturn` des Hakens: ist der Ring nicht leer, naechstes
  Byte zustellen (derselbe Kniff wie die Userdef-Kette, `exceptions.rs:
  1410ff`), sonst Kontext wiederherstellen. Schachtelungstiefe bleibt 1.
  **Warum nicht `post_upcall` direkt:** ein Steckplatz, "last writer wins"
  (`process.rs:850-856`) — ein Make und sein Break im selben Poll wuerden das Make
  verlieren; genau der Fehler, den Doom nicht haben darf.
* Neues Maskenbit `UPCALL_MASK_KBDVEC = 1 << 3` (nach `process.rs:541-553`).
* Der Besitzer-Test: nur wenn der laufende Prozess die Tabelle besitzt — trivial
  erfuellt, da der Kernel die Tabelle des *laufenden* Prozesses liest (kein
  Fremdzugriff wie beim Rad noetig).
* Verzoegerung: Zustellung ist durch die Poll-Frequenz begrenzt (KEYS01-Zusage:
  mindestens einmal je Tick in jeder AES-Warteschleife; Doom ruft
  `evnt_multi`, `sdk/apps/doom/doomgeneric_gem.c:1699`). **Zu pruefen beim Bau:**
  dass der AES-Weg nach `evnt_multi` ebenfalls durch `complete_call_tail` (also
  Upcall-Zustellung) laeuft — hier nicht belegt (`exceptions.rs:538-557` zeigt
  es fuer den gewoehnlichen Trap-Pfad).
* Nicht-Verlass: Ein Haken, der nie zurueckkehrt, haelt den Prozess an (wie auf
  TOS); kein Watchdog vorgeschlagen.

## B5. Pro Prozess oder global / Prozessende  (ENTSCHEIDUNG D1, Folge)

Pro Prozess (B1). Prozessende: Tabelle und Stubs verschwinden mit der TPA
(`free_slot`, `process.rs:944`); zusaetzlich im `free_slot` den Kernel-Ring des
Slots leeren (so wie `release_wheel_vector` dort aufgerufen wird,
`process.rs:991`). Ein ausstehender Upcall wird dort schon verworfen
(`process.rs:971-978`). **Nichts bleibt haengen.** Das ist eine bewusste
Abweichung von EmuTOS (A8: dort haengt der Zeiger) und gehoert in
`docs/COMPATIBILITY.md` "Sanctioned deviations" nach Freigabe. Zusaetzlich:
ein Kind (`Pexec`) startet mit Standardwerten; ein Elternhaken wirkt nicht im
Kind (EmuTOS: globale Tabelle, wirkt).

## B6. Wechselwirkung mit KEYS01-Wiederholung und `ikbdsys`  (ENTSCHEIDUNG D4)

* Wiederholungen rufen `kbdvec` nicht (A5) — NewTOS tut es heute schon so
  (`ikbd.rs:222-231`); **unveraendert lassen**. Ein Spiel mit Haken sieht
  Make/Break genau einmal; laeuft der Standard (Verkettung), kommen
  Wiederholungen in die BIOS-Schlange wie unter EmuTOS.
* `ikbdsys`-Wechsel (`ikbd.c:453, 507, 903`): in der Zeit des ACIA-Interrupts
  ruft EmuTOS `ikbdsys`. NewTOS hat **keinen** ACIA-Interrupt, `ikbdsys` wird
  nie gerufen (jede Eingabe laeuft durch `feed`). Vorschlag: Slot vorhanden,
  Vorgabe `ret_stub`, **wird nicht gerufen**; ein Programm, das `ikbdsys` ersetzt
  und selbst ACIA-Register liest (TOS-1.x-Spiele), geht auf der Pi ohnehin nicht
  (kein ACIA) und erhaelt keine Tasten — als *nicht unterstuetzt* dokumentiert.
  Die Stopp-Regel kann EmuTOS-treu nachgebaut werden: beim Make `ikbdsys` aus der
  Tabelle merken (`KeyRepeat::make`), in `poll_repeat` vergleichen, bei
  Abweichung Wiederholung stoppen (`ikbd.c:453-471`). Aufwand klein
  (`kernel/src/key_repeat.rs` + Lesen des Slots). D4 entscheidet: nachbauen
  (EmuTOS-treu, Standardregel "match it") oder weglassen (KEYS01 hat es bereits
  als "nicht nachgebildet" vermerkt, (cite stale 2026-10-26: _state.md:164, the KEYS01 "not modelled" note in docs/api/xbios/_state.md)).
* BIOS-Schlange: laeuft nur ueber den Standard-`kbdvec` (A4.4/A5). Verkettet das
  Spiel, fuellt der Standard die Schlange (`push_rx`) wie bisher und startet die
  Wiederholung; wer nicht verkettet, bekommt (und liefert) EmuTOS-gemaess nichts
  weiter.

## B7. Umfang  (ENTSCHEIDUNG D7)

**Minimum fuer Doom** (empfohlen als Bauauftrag KEYS02):
1. Tabelle je Prozess + `Kbdvbase` liefert den EL0-VA, **inklusive** `kbdvec`
   bei `-8` Byte und aller neun Slots mit gueltigen Vorgabe-Stubs (B1, B3).
2. `kbdvec`-Zustellung mit Ring und Kettenzustellung (B4), `kbdvec_default`-
   Syscall (D6).
3. Aufraeumen in `free_slot` (B5).
4. Optional im selben Zug: `ikbdsys`-Stopp (D4), weil klein.
Damit: Make **und Break** jeder Taste (auch Pfeile, Mehrfachtasten), Verketten
zur BIOS-Schlange, Programmende sicher.

**Vollumfang (spaeter, eigene Tickets):** `mousevec` (Paket nach A4, Aufruf bei
Maus-Paketen; heutiger Weg `feed_relative` bleibt als Standardpfad -> hier waere
ebenfalls Stub-Vergleich), `joyvec` (HID-Gamepad -> IKBD-Joystickpaket
`FD/FE/FF`, `docs/api` muss zuerst klaeren, was der HID-Treiber liefert),
`statvec`/`clockvec` (setzen die IKBD-Befehlsseite `Ikbdws` voraus, Antwortpakete;
ausserhalb), `midivec`/`midisys`/`vmiderr`/`vkbderr` (keine Hardware).
Diese Slots **existieren** im Minimum mit `ret_stub`, aber **ein dort eingehaengter
Programm-Haken wird nicht gerufen**. Das ist eine dokumentierte, nicht
getarnte Luecke (nach Regel 2 in `STATUS.md` unter "Blocked" eintragen — das
tut der Lead, nicht diese Spec) und kein Verstecken hinter einem
Stub-Rueckgabewert.

## B8. Abnahme (Vorschlag, aus der TASK abgeleitet)

* Test: Haken installiert, `br old`-Verkettung; Make A, Make B, Break A, Break B
  in einem Poll -> vier Zustellungen in dieser Reihenfolge.
* Ohne Verkettung: `Bconstat` bleibt leer, `Kbshift` unveraendert.
* Prozessende mit installiertem Haken -> naechster Prozess/Desktop reagiert normal.
* `Kbdvbase()[-1] == kbdvec`-Slot, `Kbdvbase()` stabil; Slots nie 0.
* Doom (Pi, TT): Laufen + Drehen gleichzeitig, Antippen dreht kurz.

---

# Teil C — Bau (KEYS02 Kernteil, 2026-10-04)

Umfang = D7-Minimum. Teil B gilt mit den folgenden Festlegungen; Abweichungen
von Teil B sind hier ausdruecklich genannt.

## C1. Tabelle, Stubs, `Kbdvbase` (B1/B3)

* Layout wie B1, Bereich `[0x9A0, 0xA00)` der ersten TPA-Seite
  (`loader::KBDVECS_AREA_VA`, Groesse 0x60, 0x5A belegt). `Kbdvbase()` = `0x9B0`
  (EL0-VA von `midivec`), `kbdvec` bei `0x9A8`, `mousexvec` bei `0x9A0`,
  `ikbdsys` bei `0x9F0`, `drvstat`/`kbdlength` bei `0x9F8`/`0x9F9` (beide 0, D5).
* Stubs in `[0xA00, 0xA40)` (`loader::KBDVEC_STUB_AREA_VA`):
  `ret_stub` bei `0xA00` (`ret`), `kbdvec_default` bei `0xA08`:
  `mov x1,x0; mov x0,#2002; mov x2,#0; mov x8,#0x154; svc #1; ret`
  (Kodierungen mit clang/llvm-objdump geprueft).
* Geschrieben vom Lader bei jedem Laden (`xbios::kbdvecs::write_defaults`, aus
  `populate_and_map` neben den Upcall-Trampolinen, vor der Code-Cache-Pflege).
* **Ergaenzung (in B1 nicht geregelt):** ein EL1-Aufrufer ohne Prozess (Desktop,
  Boot, `RUN == null`) bekommt eine Kernel-Tabelle gleichen Layouts mit denselben
  Vorgabewerten. Durch sie wird nie gerufen (ohne Prozess laeuft immer der
  Standardweg); sie beantwortet nur den Zeiger — dieselbe Regel wie `Keytbl`.

## C2. D6 — die Syscall-Nummer

`kbdvec_default` ruft **`Ssystem(S_NEWTOS_KBDVEC = 2002, scancode, 0)`**
(GEMDOS 0x154). Der NewTOS-eigene Bereich ist `Ssystem` 2000+
(`docs/api/gemdos/154-ssystem.md`, belegt 2000/2001); 2002 ist die naechste freie
Nummer. Vertrag: `arg1` = Byte (0..0xFF, sonst `EINVAL`), `arg2` = 0 (sonst
`EINVAL`), Ergebnis `E_OK`, `ERROR` ohne Tastaturdekoder (Host ohne Dekoder).
Die Nummer steht nur im kernel-geschriebenen Stub; kein Programm-Binaerbild
enthaelt sie, ein Umnummerieren braeche nichts.

## C3. Zustellung (B4, D3/D3a)

* `ikbd::feed` → `kbdvecs::route_key(byte)` fuer jedes Byte `< 0xF6`:
  kein laufender Prozess **oder** `kbdvec == 0xA08` → `decode_key` direkt
  (Schnellpfad, unveraendert). Sonst Byte in den Ring des Prozess-Slots
  (64, bei vollem Ring wird das **neueste** verworfen).
* Zustellung (`exceptions.rs::deliver_pending_kbdvec`) an jeder Rueckkehr nach
  EL0 (`complete_call_tail`, Resume nach Scheduler-Wechsel in `apply_switch`,
  Resume-`Psigreturn`): ein Byte je Upcall, `x0` = Scancode, `x1` = 0,
  Ziel = der **aktuelle** `kbdvec`-Wert, Maskenbit `UPCALL_MASK_KBDVEC = 1 << 3`.
  Solange das Bit gesetzt ist, wird kein weiteres Byte zugestellt (Tiefe 1);
  das naechste folgt beim `Psigreturn` des Hakens (Kette). Ein schon
  gepostetes anderes Upcall hat Vorrang. Steht beim Zustellen wieder die
  Vorgabe im Slot, wird das Byte direkt dekodiert.
* **O1 — geklaert, war nicht gegeben, ist gebaut.** Eine blockierende AES-Warte
  (`evnt_*` → `sched::sleep`) und die blockierenden Einzeltasten-Lesungen
  (`Cconin`, `Cnecin`, `Crawcin`, `Bconin(2)`) kehrten nie nach EL0 zurueck,
  solange keine Taste in der BIOS-Schlange lag — die legt bei einem verkettenden
  Haken aber erst der Haken selbst hinein: Verklemmung. Jetzt **weichen sie aus**:
  `sleep` liefert bei wartenden Ring-Bytes (vor dem Blockieren und nach dem
  eigenen Pump) `Yield(switch_restart_for_upcall())`, die Lesungen
  `Host::console_read_key_restartable() == None` + `kbdvecs::request_restart()`.
  Der Trap-Handler bricht den Aufruf wie einen S2-Yield ab (`elr` auf das `svc`
  zurueck, nichts festgeschrieben), stellt den Haken zu, und nach dessen
  `Psigreturn` laeuft das `svc` erneut (`evnt_multi`-Zeitgrenze bleibt ueber
  `syscall_deadline` erhalten). Gemessen in QEMU: `evnt_multi(MU_KEYBD)` **ohne**
  Timer und `Bconin(2)` liefern die vom Haken verkettete Taste; zwei Breaks im
  selben Poll erreichen den Haken beide, in Reihenfolge, das zweite ueber die
  `Psigreturn`-Kette (`measurements/measurement-2026-10-04-keys02/`).
* **Grenzen (dokumentiert, nicht gebaut):** `Cconrs`, konsolen-`Fread` und
  `v_string` (Anfragemodus) lesen weiter ohne Ausweichen — sie haben vor dem
  Ende bereits Wirkung (Echo, Puffer); ein Programm, das bei eingehaengtem und
  verkettendem Haken darin wartet, haengt. Modale AES-Schleifen, die
  `pump_input` selbst rufen (`form_do`, Menue-/Ziehschleifen), stellen erst bei
  ihrer Rueckkehr zu (Ring 64). Ein Haken laeuft als normale AAPCS64-Funktion,
  aber das Upcall sichert die **FP/SIMD-Register** des unterbrochenen Codes
  nicht (gilt fuer alle Upcalls); ein Haken darf sie nicht benutzen.

## C4. Wiederholung und `ikbdsys` (D4)

`KeyRepeat::make` merkt sich den `ikbdsys`-Wert des laufenden Prozesses
(`kbdvecs::current_ikbdsys`, ohne Prozess `0xA00`), `poll` stoppt bei einem
50-Hz-Schritt mit laufender Wiederholung und anderem Wert (vor dem
`conterm`-Test, `ikbd.c:453-472`); bei Neuumsetzung wird der Wert nachgezogen
(`:507`). `ikbdsys` wird nie gerufen. Wiederholungen rufen `kbdvec` nicht
(unveraendert, `ikbd.c:524`) — gemessen: der Haken sah 6 Codes, die AES 7+7
Woerter (Make + Wiederholungen).

## C5. Prozessende (D8)

Tabelle und Stubs liegen in der TPA und verschwinden mit ihr; `free_slot` leert
den Ring (`kbdvecs::release_slot`). Gemessen: ein Programm, das seinen Haken
**nicht** zuruecksetzt, endet, und der Desktop reagiert auf Alt-C normal.
Abweichung von EmuTOS in `docs/COMPATIBILITY.md` (Sanctioned deviations).

## C6. Nicht gerufene Slots (D7-Luecke)

`midivec`, `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `clockvec`, `joyvec`,
`midisys`, `ikbdsys` und `mousexvec` existieren mit `ret_stub`; ein dort
eingehaengter Programm-Haken wird **nicht gerufen** (Maus laeuft weiter
kernelintern ueber `vdi::feed_relative`, `mousexvec` ueber
`bios::vectors::mousexvec`). Eintrag unter `docs/STATUS.md` "Blocked" (Lead).
**[BIOS15] ADR-031:** nicht gerufen bleibt (XBIOS-CHARDEV-003 ACCEPTED), aber
nicht still: das erste Eingabeereignis, das den Slot bei EmuTOS riefe
(`ikbdsys`/`midisys` jedes IKBD-Byte, `mousevec` Mauspaket, `statvec`,
`clockvec`, `joyvec` ihr Paket), schreibt eine Warnung je Haken und Prozess
(`tos-api/src/xbios/kbdvecs.rs:447@check_events`). `midivec`, `vkbderr`,
`vmiderr`: ihr Ereignis (MIDI-Byte, Ueberlauf) tritt nie ein, keine Warnung.

## C7. Belege

Hosttests `tos-api/tests/xbios_kbdvecs.rs` (10), `kernel/src/key_repeat.rs`
(+3 `ikbdsys`), QEMU-raspi4b-Lauf mit `sdk/examples/kbdvec.c`
(`measurements/measurement-2026-10-04-keys02/RUNINFO.TXT`).

---

# Korrekturen an der Vorfassung (22-kbdvbase.md Phase 00) und `_state.md §1`

1. **`kbdvec` fehlte komplett** (Slot bei `-4`; `aciavecs.S:129`); ebenso
   `mousexvec` (`-8`) und die Nachbarbytes `ikbdstate`/`kbdlength`.
2. `statvec` = Paket `0xF6` (stimmte), `mousevec` bekommt **auch `0xF7`**
   (Absolutmaus, `aciavecs.S:488-489`), nicht nur `0xF8..0xFB`; `joyvec` bekommt
   `0xFD..0xFF` (stimmte).
3. "`ikbdstate`/`kbdlength` ... not part of the contract": **falsch fuer
   `ikbdstate`**. tos.hyp fuehrt `drvstat` als dokumentiertes Feld hinter
   `ikbdsys`; Programme sollen vor dem Umhaengen darauf pruefen. Intern frei
   bleibt `kbdlength`/`kbdbuf` (D5).
4. Die Vorfassung nannte als NewTOS-Vorgabe "36-Byte-Layout mit nativen
   Zeigern, nicht befuellt" und "`Initmous` schreibt `mousevec`": der Zustand
   (alle 0) ist unbrauchbar fuer Verkettung (`br 0`). Neuer Vorschlag B3.
5. Die Compendium-Querverweise "not present in references/" sind ueberholt
   (tos.hyp abgerufen, s. u.).
6. `_state.md §1` (Tabelle `kbdvecs`) hat dieselben Luecken (kein `kbdvec`, Paket
   `0xF7`, `drvstat`); beim Freigeben mit anheben.

---

# Querverweise

* Atari Compendium (tos.hyp, abgerufen 2026-10-04 per WebFetch; Seite wurde vom
  Abruf-Werkzeug **zusammengefasst**, nicht im Wortlaut geprueft):
  * https://freemint.github.io/tos.hyp/en/xbios_structures.html (KBDVBASE: Feld-
    liste mit `drvstat`, "address of the packet in A0 and on the stack", RTS,
    <= 1 ms)
  * https://freemint.github.io/tos.hyp/en/xbios_interface.html (Kbdvbase:
    Warnung `drvstat`/Interrupts sperren vor Aenderung)
  * `kbdvec` wird auf beiden Seiten **nicht** erwaehnt.
* EmuTOS: `bios/aciavecs.S`, `bios/ikbd.c`, `bios/xbios.c`, `bios/mouse.c`,
  `bios/amiga2.S`, `vdi/vdi_mouse.c`, `vdi/vdi_asm.S`, `bdos/proc.c`,
  `include/biosdefs.h`, `include/config.h`.
* FreeMiNT: `sys/keyboard.c`, `sys/arch/init_intr.c`, `sys/arch/intr.S`,
  `sys/arch/acia.S`, `sys/dev-mouse.c`, `sys/mint/struct_kbdvbase.h`,
  `sys/usb/src.km/udd/hid/{mouse,mouse-boot,tablet}/*.c`.
* NewTOS-Code (gelesen): siehe B0.

# Offene Punkte (nicht geklaert, bewusst nicht erraten)

* **O1.** Ob der AES-Pfad nach `evnt_multi` eine Upcall-Zustellung am Rueckweg
  hat (B4) — beim Bau pruefen. **Geklaert 2026-10-04 (C3):** am Rueckweg ja, aber
  eine *blockierende* Warte kehrte nicht zurueck; jetzt weicht sie aus.
* **O2.** `IOREC`-Form fuer `x1` (D2a) haengt an der Entscheidung ueber `Iorec`.
  **Geklaert 2026-10-29 (XBIOS01, E6):** `x1` = `Iorec(1)`, LP64-`_IOREC`.
* **O3.** TT TOS 3.06 hat `kbdvec`: nur abgeleitet (A7). Eine Messung am
  Hatari-TT (`hatari-produces-tt-references`) koennte es bestaetigen — hier nicht
  ausgefuehrt.
  **Geklaert 2026-10-04 (KEYS02):** gemessen, ja — ROM und Hatari-TT-Debugger,
  Belege in A7.
* **O4.** Ob `ikbdstate` ueberhaupt je beobachtbar mitten im Paket ist: der
  HID-Uebersetzer fuettert pro Report atomar (gefolgert, nicht geprueft) —
  daher Vorschlag `drvstat == 0` konstant.

# Entscheidungen fuer den Anwender

| # | Frage | Empfehlung |
|---|---|---|
| D1 | Tabelle je Prozess in TPA-Seite 0 (`0x9A0`), `Kbdvbase` liefert EL0-VA; Elternhaken wirkt nicht im Kind; TSR-Haken fuer Fremdprogramme nicht moeglich | ja |
| D2 | Aufrufkonvention: AAPCS64, `x0` = Scancode/Byte bzw. Paketzeiger, `x1` = ersetzt a0; ILP32 nicht unterstuetzt | ja |
| D2a | `x1` bei `kbdvec`: 0 bis `Iorec(1)` existiert | ja |
| D3 | Zustellung ueber Kernel-Ring je Prozess + Upcall-Kette (nicht `post_upcall` pur); Schnellpfad bei unveraendertem Standardhaken | ja |
| D3a | Ring-Groesse / Ueberlaufregel (Vorschlag 64, neueste verwerfen) | 64 |
| D4 | `ikbdsys`-Wechsel-Stopp der Wiederholung nachbauen (EmuTOS-treu) bzw. `ikbdsys` nie rufen | nachbauen, `ikbdsys` nie rufen |
| D5 | `drvstat`-Byte hinter `ikbdsys` als konstantes 0 anbieten | ja |
| D6 | Neuer interner Syscall fuer `kbdvec_default` (Nummer vergibt der Lead) | ja |
| D7 | Umfang: Minimum (kbdvec + sichere Vorgaben + Aufraeumen) jetzt; Maus/Joystick/Status/Uhr/MIDI als Folge-Tickets, im Minimum nur `ret_stub`, dokumentierte Luecke | Minimum |
| D8 | Abweichung "Haken verschwindet mit dem Prozess" in COMPATIBILITY.md "Sanctioned deviations" eintragen | ja |

# Teil D — MEM12: Ort der Tabelle und Empfaenger eines Ereignisses (FREIGEGEBEN 2026-10-27)

Anlass: TASK/MEM12, Anwenderentscheidung 2026-10-27 ("E2: yes; also specify
whose Kbdvbase table receives an input event (owner/foreground)"). Teil B/C
bleiben gueltig, bis auf D1 (Ort) und B4 (Empfaenger), die hier ersetzt werden.

## D1-neu. Ort

Die Tabelle (`mousexvec` … `kbdlength`, Layout B1 unveraendert) zieht aus
Seite 0 in eine **zweite private Seite je Prozess**, EL0 lesen + schreiben,
nicht ausfuehrbar, an fester VA unter 2 MiB, die nie geteilt abgebildet wird
(Seite 0 ist ab MEM12 fuer EL0 nur im Supervisor-Zustand beschreibbar,
`docs/api/_super-state.md`). `Kbdvbase()` liefert die EL0-VA des `midivec`-Slots
dort. Die Vorgabe-Stubs (`just_rts`, `kbdvec_default`) ziehen in die gemeinsame,
fuer EL0 nur ausfuehrbare Stub-Seite; ihre Adressen bleiben fuer alle Prozesse
gleich. Beobachtbar aendert sich nur der Zahlenwert des Zeigers.

## D9. Wessen Tabelle bekommt ein Eingabeereignis

EmuTOS hat **eine** Tabelle; `ikbdraw` springt durch den `kbdvec`, der gerade
eingetragen ist, gleich welches Programm laeuft (`bios/aciavecs.S:405-414`).
Auf einem Single-TOS ist das der Haken des Programms im Vordergrund — oder
eines Accessorys, wenn eines ihn gesetzt hat.

Bis MEM12 (KEYS02 C3) nahm NewTOS die Tabelle des Prozesses, der beim Poll
gerade die CPU hat (`route_key` mit `current()`, Stand `a90bcfb0`; seit D9
`tos-api/src/xbios/kbdvecs.rs:352@route_key`). Pollt der
Pump im Kontext eines Accessorys, sieht DOOMs Haken das Byte nicht.

**Vorschlag:** Empfaenger ist der **innerste gestartete Prozess** — EmuTOS'
`run` ohne Accessories, in NewTOS `process::emutos_run()`
(`tos-api/src/bdos/process.rs:1023@emutos_run`): das Programm im Vordergrund oder
sein laufendes `Pexec`-Kind. Gleich, welcher Prozess beim Poll die CPU hat:

* Steht in dessen `kbdvec` die Vorgabe (oder lebt kein gestarteter Prozess),
  laeuft `kbd_int` direkt (Schnellpfad wie bisher).
* Sonst kommt das Byte in den Ring **dieses** Prozesses und wird an seiner
  naechsten Rueckkehr nach EL0 zugestellt (C3). Schlaeft er in einer Warte, die
  bei wartenden Ring-Bytes ausweicht (C3 "O1"), wird er dafuer geweckt.
* Die Tabelle eines **Accessorys** wird nie befragt (Accessories werden nicht
  gestartet, `emutos_run` nennt sie nie). Abweichung von EmuTOS: dort wirkte
  ein Accessory-Haken global. Kein portiertes Accessory setzt einen.

**D9 freigegeben (Anwender 2026-10-27):** Empfaenger = `emutos_run()`.
Setzt ein Accessory je einen `kbdvec`-Haken (sein Slot haelt nicht mehr die
Vorgabe), schreibt der Kern einmal je Accessory eine Warnung ins Protokoll —
die Abweichung soll nie still zuschlagen. [BIOS15] Seit ADR-031 beim
ersten Tastenbyte, das der Haken bei EmuTOS bekaeme (der gemeinsame
Mechanismus, `check_events`), nicht mehr bei jedem Syscall-Ruecksprung. Der Haken laeuft im
Supervisor-Zustand (`docs/api/_super-state.md` S11, O1 = ja).

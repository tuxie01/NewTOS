# OS-Haken, die Programme einhaengen — Bestand und ADR-Vorschlag (2026-10-28)

**Zweck:** Beleg und Entscheidungsvorlage fuer das gemeinsame ADR „OS hooks
installed by programs“ (Anwenderentscheidung 2026-10-28, `TASK/BIOS03.md`
Arbeit 1–4). Vorbild in Aufbau und Beleggrad: ADR-030 *No Line-A emulation*
(`docs/MEMORY.md`, Abschnitt `## ADR-030`) und die Erhebung dahinter
(`docs/analysis/line-a-readers-2026-10-26.md`). Punkt 5 der Arbeit
(Entscheidung) ist Sache des Anwenders; diese Datei **entscheidet nichts** — wo
der Beleg duenn ist, steht eine Optionstabelle.

**Stand:** EmuTOS-Pin `references/emutos` @ `1a7484ab` (`docs/analysis/ORACLE.md`);
NewTOS `main` @ `be638c5e`. Datierter Schnappschuss.

**Legende.** **[R]** = im Quelltext dieser Sitzung gelesen, mit Zitat.
**[D]** = abgeleitet/geschlossen (Rechnung, Fehlen eines Treffers, Folgerung);
traegt nur, was in der Herleitung steht. **[O]** = offen, „needs deeper
analysis“. EmuTOS-Zitate sind Zeilenzitate (gepinnt); NewTOS-Zitate tragen den
Symbolanker. Schalter-Gating (`CONF_WITH_*`) steht in der Haken-Liste; die
Konfigurationstabelle ist `docs/api/_emutos-config.md`.

---

## 1. Die Haken-Liste (Arbeit 1)

„Haken“ = eine Stelle, ueber die EmuTOS Code ruft, den ein Programm dort
eingehaengt haben kann. Nicht dazu gehoeren `Supexec` (ein Aufruf mit
Code-Argument, kein Haken; Anweisung des Auftrags) und reine Daten-Sysvars.
Zwei Eintrittswege, die fuer NewTOS verschieden sind:

- **Setexc-Weg:** `setexc(num, v)` liest/schreibt das LONG bei `4*num`
  (`references/emutos/bios/bios.c:1070-1080`) — [R].
- **Direkter Speicherzugriff:** Haken, deren Adresse **nicht durch 4 teilbar**
  ist (`0x472`, `0x476`, `0x47E`, `0x46A`, `0x46E`, `0x47A`, `0x426`, `0x42A`,
  `0x456`, `0x4FE`, `0x502`–`0x512`), sind per `Setexc` nicht erreichbar — [D]
  (Adressen aus `references/emutos/tosvars.ld:40-97`, Teilbarkeit gerechnet).
  Programme schreiben sie direkt im Super-Zustand (`force_mediach`, Abschnitt 2).

### 1.1 `Setexc`-adressierbare Systemhaken

| # | Haken | Adresse / `Setexc`-Nr. | Default (Installation) | Aufrufstelle (Arg./Rueckgabe) | Art | Gating |
|---|---|---|---|---|---|---|
| H01 | `etv_timer` | `0x400` / `0x100` | `just_rts` `bios/bios.c:360`; danach GEMDOS `etv_timer = tikfrk` `bdos/time.c:167`; danach VDI `tick_int` ueber `Setexc(0x100, …)` `vdi/vdi_misc.c:131-132` (`tim_chain` = alter Wert, `:88`) | `bios/vectors.S:540-544` im Timer-C-Handler (alle 20 ms): `move.w _timer_ms,-(sp); move.l _etv_timer,a0; jsr (a0)`; Arg = ms-Zaehler auf dem Stapel, Rueckgabe ignoriert | Benachrichtigung, Kette | immer; Timer C selbst ist MFP-Hardware |
| H02 | `etv_critic` | `0x404` / `0x101` | `default_etv_critic` `bios/bios.c:361`, Code `bios/vectors.S:823-828`; AES setzt `criterr_handler` mit `Setexc(0x101)` `aes/gemdosif.S:105-108,125-130,153-156` | `call_etv_critic` `bios/vectors.S:807-821`, gerufen aus `bios/blkdev.c:520-521`, `:628`, `:808` (Fehlercode‖Geraet auf dem Stapel, Rueckgabe in `d0` = Abbruch/Wiederholung, `CRITIC_RETRY_REQUEST` `bios/blkdev.h:36`) | **synchron, mit Rueckgabewert** | immer |
| H03 | `etv_term` | `0x408` / `0x102` | `just_rts` `bios/bios.c:362` | `xterm`: `Setexc(0x102, -1)` lesen und `protect_v` rufen, vor jedem Abbau `bdos/proc.c:603-607`; kein Argument | Benachrichtigung vor Terminierung | immer |
| H04 | `etv_xtra` | `0x40C` / `0x103` | keiner (nur `tosvars.ld:36`) | **kein Aufrufer im Baum** (`/usr/bin/grep -rn etv_xtra`: nur `tosvars.ld`) — [D] | — | — |
| H05 | `bell_hook` | `0x5AC` / `0x16B` [D] | `do_bell` `bios/sound.c:90` | `bell()` `bios/sound.c:251-254` (`protect_v(bell_hook)`), gerufen aus VT52-BEL `bios/vt52.c:216` | Benachrichtigung | Klang selbst `CONF_WITH_YM2149` (`bios/sound.c:243-246`) |
| H06 | `kcl_hook` | `0x5B0` / `0x16C` [D] | `do_keyclick` `bios/sound.c:91` | `_keyclick` `bios/vectors.S:1430-1437` (`d0` = Scancode), gerufen aus `bios/ikbd.c:496`, `:802`, `:846`, `:895`, nur bei `conterm & 1` (`:495`) | Benachrichtigung | — |

### 1.2 Seite-0-Zellen, nur per Direktzugriff

| # | Haken | Adresse | Default | Aufrufstelle | Art | Gating |
|---|---|---|---|---|---|---|
| H07 | `hdv_rw` | `0x476` | `blkdev_rwabs` `bios/blkdev.c:102` | `lrwabs` `bios/bios.c:1047` (`protect_wlwwwl(hdv_rw, …)`); Verbraucher: jeder GEMDOS-Plattenzugriff ueber `Rwabs` (`bdos/fs.h:64`, `:73`) | **synchron, Rueckgabe = Fehlercode** | — |
| H08 | `hdv_bpb` | `0x472` | `blkdev_getbpb` `bios/blkdev.c:101` | `getbpb` `bios/bios.c:1122`; GEMDOS `ckdrv` `bdos/fsdrive.c:96`, Medienwechsel `bdos/bdosmain.c:471` | synchron, Rueckgabe = BPB-Zeiger | — |
| H09 | `hdv_mediach` | `0x47E` | `blkdev_mediach` `bios/blkdev.c:103` | `mediach` `bios/bios.c:1186`; GEMDOS `bdos/fsbuf.c:179` (**je Puffer-Wiederverwendung**), `bdos/fsdrive.c:106-114` (nur Laufwerke aus `drvrem`) | synchron, Rueckgabe 0/1/2 | — |
| H10 | `hdv_init` | `0x46A` | `flop_hdv_init` `bios/blkdev.c:98`, Kommentar `:96-97`: „EmuTOS does not use hdv_init“ | **kein Aufruf ueber den Vektor** (`/usr/bin/grep`: nur Zuweisung `bios/blkdev.c:98`; `blkdev_hdv_init` wird direkt gerufen `:106`) — [D] | — | — |
| H11 | `hdv_boot` | `0x47A` | `blkdev_hdv_boot` `bios/blkdev.c:95` | **kein Aufruf ueber den Vektor**: `blkdev_boot` ruft `blkdev_hdv_boot()` direkt `bios/blkdev.c:252` — [D] | — | — |
| H12 | `swv_vec` | `0x46E` | `just_rts` `bios/bios.c:363`; spaeter `os_header.reseth` `:552` | `detect_monitor_change` `bios/screen.c:1329`, aus dem VBL `bios/vectors.S:351-353` | Benachrichtigung („reset on monitor change“) | `#if CONF_WITH_ATARI_VIDEO` (`bios/screen.c:1230`) |
| H13 | `exec_os` | `0x4FE` | `os_header.os_magic->gm_init` `bios/bios.c:534` | Boot: `Pexec(…)`-Basepage mit `p_tbase = exec_os` `bios/bios.c:856-865` (nach `AUTO`, ersetzt die ROM-Shell) | einmalig beim Boot | — |
| H14 | `dump_vec` + `dumpflg` | `0x502`, `0x4EE` | `just_rts` `bios/chardev.c:84`; `dumpflg = -1` `bios/bios.c:357` | VBL, wenn `dumpflg == 0` `bios/vectors.S:466-472` (Alt-Help setzt es `bios/ikbd.c:596`); `Scrdmp` `bios/xbios.c:490-494` | Benachrichtigung | — |
| H15 | `prt_stat`, `prt_vec`, `aux_stat`, `aux_vec` | `0x506`, `0x50A`, `0x50E`, `0x512` | `just_rts` `bios/chardev.c:78-83` | **kein Aufrufer im Baum** (`/usr/bin/grep -rn` ueber `bios bdos aes vdi desk cli`: nur Zuweisungen und `include/tosvars.h:88-91`) — [D] | — | — |
| H16 | `vblqueue`, `nvbls`, `vbl_list` | `0x456`, `0x454`, `0x4CE` | `vblqueue = vbl_list`, 8 Slots, leer `bios/bios.c:365-373`; VDI belegt Slot 0 mit `vb_draw` `vdi/vdi_mouse.c:602` | VBL-Schleife `bios/vectors.S:436-459` (jeder Eintrag `!= 0` wird gerufen) | Benachrichtigung je 50/60 Hz | — |
| H17 | `resvalid` / `resvector` | `0x426` / `0x42A` | 0 (EmuTOS setzt nichts) | `bios/startup.S:497-518`: bei `resvalid == 0x31415926`, `resvector` gerade und ≠ 0, `jmp` dorthin (Warmstart) | einmalig beim Reset | — |
| H18 | `con_state` | `0x4A8` | `normal_ascii` `bios/vt52.c:244` | `bios/vt52.c:153-159` (VT52-Zustandsautomat) | OS-interner Zustand, kein vorgesehener Haken — [D] | — |
| H19 | `bconstat_vec`, `bconin_vec`, `bcostat_vec`, `bconout_vec` (je 8) | `0x51E`, `0x53E`, `0x55E`, `0x57E` | `bconstat_init` usw. `bios/chardev.c:68-75`; Geraet 1 aus `MAPTAB` `bios/serport.c:998-1001` | `bios/bios.c:932`, `:976`, `:1009`, `:1161` (Geraetetabelle der BIOS-Zeichenfunktionen) | synchron, Rueckgabe | `Bconmap`/`MAPTAB`-Teil gated durch die Bconmap-Konfiguration, siehe `docs/api/xbios/2c-bconmap.md` |

### 1.3 `Kbdvbase`-Tabelle (`struct kbdvecs`, XBIOS 0x22)

Layout `references/emutos/bios/aciavecs.S:126-139`; Default-Belegung
`:146-193` (`_init_acia_vecs`), alle ausser `kbdvec`/`midivec`/`clockvec`/
`midisys`/`ikbdsys` auf `just_rts`.

| # | Slot | Default | Aufrufstelle | Argument | Gating |
|---|---|---|---|---|---|
| K01 | `kbdvec` (vor der Tabelle) | `_kbdvec` → `kbd_int` `:372`, `:154-157` | `ikbdraw`: `move.l kbdvec,-(sp); rts` `:411-414`; VDI/BIOS-Pfad `kbd_int` | `d0` = Scancode (Make/Break), `a0` = `ikbdiorec` | `CONF_WITH_IKBD_ACIA` fuer die Quelle |
| K02 | `midivec` | `_midivec` `:315` | `_midisys` `:302-303` | `d0` = MIDI-Byte | `CONF_WITH_MIDI_ACIA` |
| K03 | `vkbderr` | `just_rts` `:165` | `_ikbdsys` bei Empfaenger-Ueberlauf `:396-397` | `d0` = Byte | `CONF_WITH_IKBD_ACIA` |
| K04 | `vmiderr` | `just_rts` `:166` | `_midisys` bei Ueberlauf `:308-309` | `d0` = Byte | `CONF_WITH_MIDI_ACIA` |
| K05 | `statvec` | `just_rts` `:167`; **VDI belegt es** mit `wheel_int` (Wheel-Pakete) `vdi/vdi_mouse.c:609-612` | `kbd_status` `:484-487`, `kbd_jump_vec` `:527-531` | Zeiger auf das Paket (ohne Kopf) auf dem Stapel | `CONF_WITH_EXTENDED_MOUSE` fuer die VDI-Belegung |
| K06 | `mousevec` | `just_rts` `:168`; **VDI belegt es** mit `mouse_int` ueber `Initmous(1, …)` `vdi/vdi_mouse.c:605`, `bios/mouse.c:117` | `kbd_rel_mouse` `:491`; aus C `call_mousevec` `bios/ikbd.c:319`, `:388`, `:522` | Zeiger auf 3-Byte-Paket; Handler duerfen es **veraendern** und kettieren — [D] (Kommentar `aciavecs.S:35-37`, Rolle der Kette aus `vdi/vdi_mouse.c:605`) | — |
| K07 | `clockvec` | `_clockvec` `bios/clock.c:969` (`CONF_WITH_IKBD_CLOCK`), sonst `just_rts` `:171-177` | `kbd_clock` `:495-496` | Paketzeiger | `CONF_WITH_IKBD_CLOCK` |
| K08 | `joyvec` | `just_rts` `:179` | `kbd_joys`/`joy_next` `:525`, aus C `call_joyvec` | Paketzeiger, `d0` = letztes Byte (undokumentiert, `:505-516`) | — |
| K09 | `midisys` | `_midisys` `:295` | `_int_acia` `:264-265` | — (kein Argument) | `CONF_WITH_MIDI_ACIA` |
| K10 | `ikbdsys` | `_ikbdsys` `:382` | `_int_acia` `:268-269`; ausserdem liest `kbd_int` den Wert zum Abbruch der Tastenwiederholung `bios/ikbd.c:453`, `:507`, `:903` | — | `CONF_WITH_IKBD_ACIA` |
| K11 | `mousexvec` (**vor** `kbdvec`, nicht ueber `Kbdvbase` erreichbar) | `just_rts` `:149-151`; VDI `vdi_mousex_handler` `vdi/vdi_mouse.c:612` | `kbd_int` `bios/ikbd.c:748-760` | Scancode | `CONF_WITH_EXTENDED_MOUSE` |

### 1.4 CPU-/Interrupt-Vektoren ueber `Setexc` (Nr. < 0x100)

| # | Haken | Beleg | Anmerkung |
|---|---|---|---|
| V01 | Trap-Vektoren `#1` (`Setexc(0x21)`), `#2` (`0x22`), `#13`/`#14` | EmuTOS besetzt sie selbst: `bdos/bdosmain.c:320`, `:326`, `aes/gemdosif.S:95-99`, `bios/bios.c:174-175`; Programme (GEMDOS-Patcher, TSRs) ketten sich davor | Aufruf durch den m68k-`TRAP`-Befehl |
| V02 | Autovektoren (VBL `0x70`/Nr. 28, HBL `0x68`), MFP-Vektoren `0x100–0x13C` (Timer A–D, ACIA `0x118`, …), Exception-Vektoren (Bus-/Adressfehler, illegal …) | Tabelle `_init_exc_vec` `bios/vectors.S:103-114`; ACIA-Interrupt `_int_acia` `bios/aciavecs.S:264-269`; MFP-Installation `mfpint` `bios/mfp.c:114-120`, `xbtimer` `:163-169` | Hardware-Interrupts bzw. m68k-Ausnahmen |

### 1.5 XBIOS-Installer (Funktionen, die einen Haken setzen)

| # | Aufruf | Setzt | Beleg |
|---|---|---|---|
| X01 | `Initmous(type, param, vec)` (0x00) | `kbdvecs.mousevec` (K06) | `bios/mouse.c:117-120` |
| X02 | `Mfpint(num, vec)` (0x0D), `Xbtimer(timer, ctrl, data, vec)` (0x1F), `Jdisint`/`Jenabint` (0x1A/0x1B) | MFP-Vektor `0x40+num` | `bios/mfp.c:114-130`, `:163-169` (`CONF_WITH_MFP`) |
| X03 | `Scrdmp` (0x14) | ruft `dump_vec` (H14) | `bios/xbios.c:490-494` |
| X04 | `Bconmap` (0x2C) | Zeiger auf `MAPTAB`, dessen Eintraege Programme ersetzen (H19) | `bios/serport.c:998-1002`, `:1105-1117` |

### 1.6 VDI-Austauschvektoren (benachbart)

Dokumentierte VDI-Calls, nicht Seite 0; sie gehoeren zur selben Frage („Code
eines Programms wird vom OS gerufen“), also hier:

| # | Haken | EmuTOS-Aufrufstelle |
|---|---|---|
| D01 | `vex_butv`, `vex_motv`, `vex_curv` | `mouse_int` (`vdi/vdi_asm.S:283`, `:359`, `:367` laut `tos-api/src/vdi/mouse.rs:937@vex_butv`-Kommentar) |
| D02 | `vex_timv` | `tick_int` `vdi/vdi_misc.c:83-88`, `vdi_vex_timv` `:100-113` |
| D03 | `vex_wheelv` | `vdi/vdi_mouse.c:424-442` (`CONF_WITH_EXTENDED_MOUSE`) |

### 1.7 Bereits bedient — fuer die Vollstaendigkeit der Tabelle

`Supexec` (kein Haken), AES `G_USERDEF`/`ub_code` (`UPCALL_MASK_AES_USERDEF`),
`etv_term`, `kbdvec`, `vex_wheelv`: Abschnitt 3.

---

## 2. Bestand: welche Programme setzen welchen Haken (Arbeit 2)

**Methode.** `/usr/bin/grep -rnIE` ueber die Quellen der Portliste
(`docs/sdk/09-port-material.md` §1: mintlib, gemlib, cflib, libm, LDG, libshare,
Codec-Bibliotheken, WinDom, zView, QED, TeraDesk, CoreMark, Doom, `PRESENT`;
dazu `sdk/apps/{hypview,highwire,gemtest,chaintest,apitest}`, `sdk/overlays/`,
`sdk/examples/`, `sdk/templates/`, `programs/`, `tosdump/`,
`Applications/{doomgeneric,qed,thing-neo-codeSVN-r62,z-tools,ldg-2,
windom-1.21.3,pml-2.03,cflib-git}`), Muster: `hdv_*`, `etv_*`, `swv_vec`,
`exec_os`, `scr_dump`/`dump_vec`, `prt_*`/`aux_*`, `bell_hook`/`kcl_hook`,
`resvalid`/`resvector`, `vblqueue`/`vbl_list`, `Kbdvbase`, `Initmous`, `Mfpint`,
`Jdisint`, `Xbtimer`, `Bconmap`, `mousevec`/`statvec`/`clockvec`/`joyvec`/
`midivec`/`vkbderr`/`vmiderr`/`ikbdsys`/`midisys`/`kbdvec`, `vex_*`, `Setexc`,
`Supexec` sowie die Hex-Adressen `0x400/404/408/426/42a/456/46a/46e/472/476/
47a/47e/4a8/4ce/4fe/502/506/50a/50e/512/5ac/5b0`. Jeder Treffer gelesen;
Namensgleichheiten (Versionsnummern `0x400`, `Setexc(0x5A0/4,-1)` als Cookie-Peek,
`Supexec` fuer Cookie-/Header-Lesen, `conterm` `0x484`) sind unten als „kein
Haken“ eingeordnet. Kein Binaerbefund (anders als in der Line-A-Erhebung):
Haken werden zur Laufzeit gesetzt, `llvm-nm` hilft nicht; die Quellaussage
gilt fuer das, was gebaut wird, ueber die Build-Skripte
(`sdk/apps/build-*.sh`) — [D].

### 2.1 Programme, die einen Haken einhaengen

| Programm | Haken | Was es tut | Datei:Zeile | Gebaut/ausgeliefert |
|---|---|---|---|---|
| **TeraDesk** | `hdv_rw`, `hdv_bpb`, `hdv_mediach` (H07–H09) | `force_mediach()`: im Super-Zustand die drei Vektoren auf `Newrwabs` (liefert `ECHMEDIA` fuer das Laufwerk), `Newgetbpb`, `Newmediach` (liefert `2` fuer das Laufwerk) umbiegen, `Fopen("X:\\X")` erzwingt dadurch Neu-Anmeldung des Laufwerks, danach zurueck. Aufrufer: Shift+ESC in einem Verzeichnisfenster und nach Disk-Operationen | `sdk/apps/teradesk/file.c:925-960`, `:866-884` (`Newmediach`), `:887-915` (`Newrwabs`); Aufrufer `sdk/apps/teradesk/window.c:3284`, `sdk/apps/teradesk/floppy.c:878`; NewTOS-Overlay (LP64: 32-Bit-Zellen) `sdk/overlays/teradesk/file.c:952@force_mediach`. Der `_MiNT`-Zweig (`Dlock`) wird nur mit MiNT-Cookie genommen (`sdk/apps/teradesk/main.c:1792`); NewTOS hat weder Cookie noch `Dlock` (`/usr/bin/grep -i dlock tos-api/src`: keine Treffer) — [D] | ja (`DESKTOP.NAP`) |
| **TeraDesk** | `etv_critic` (H02) | `install_critic()` beim Start eines TOS-Programms (`aptype == 0`): `Setexc(0x101, new_critic)`, `new_critic` gibt den Fehlercode zurueck (kein Alert); beim Ende `remove_critic()` | `sdk/apps/teradesk/startprg.c:104-118`, `:285-290`, `:318` | ja |
| **TeraDesk** | CPU-Vektoren VBL `0x70` (Nr. `0x070`, zweimal) und Timer C `0x114`/Nr. `0x114` | `lobo()` im Herunterfahren: Vektoren auf `loopcpu` (Endlosschleife), um die Maschine „anzuhalten“ (Option „shutdown“ ohne Reset) | `sdk/apps/teradesk/main.c:1722-1735`; Aufruf `Supexec(rv)` `:2118` | ja |
| **TeraDesk** | `resvalid`/`resvector` (H17) | (a) Shutdown-Reset: beide auf 0 schreiben vor dem Sprung zum Reset-Handler; (b) `cookie_reset` wird angehaengt, **nur** wenn kein Cookie-Jar existiert (NewTOS hat einen: `tos-api/src/bdos/process/loader.rs:1242@p_cookies`) | `sdk/apps/teradesk/main.c:59-60`, `sdk/apps/teradesk/main.c:2099-2100`; `sdk/apps/teradesk/library/utility/cookie.c:94-95`, `sdk/apps/teradesk/library/utility/cookie.c:160-163` | ja |
| **Doom** (`sdk/apps/doom`) | `kbdvec` (K01); auf m68k zusaetzlich `etv_term` (`0x408`) | `kbd_install` (Supexec): `Kbdvbase()-8` lesen, Hook schreiben, alten Wert kettieren; `kbd_remove` stellt zurueck; **nur im `__m68k__`-Zweig** wird `etv_term` mitbelegt (zum Aufraeumen bei Abbruch) | `sdk/apps/doom/doomgeneric_gem.c:1871-1886@kbd_install`, `:1891-1902@kbd_remove`, Aufruf `:1923`, `:1939` | ja |
| `KBDVEC.NAP` (Beispiel) | `kbdvec` (K01) | KEYS02-Beweis: Hook, kettet an den alten Wert | `sdk/examples/kbdvec.c:16-17@kbdvec`, `:167` | Messprogramm, nicht in `make-sd-image.sh` |
| `P0KID` (Beispiel) | `kbdvec`-Slot lesen/schreiben/zuruecksetzen | prueft Beschreibbarkeit der privaten Seite | `sdk/examples/p0kid.c:146-150@Kbdvbase` | Messprogramm |
| `APITEST` | `etv_*` (**nur Abfrage**), Schatten-Slot | `Setexc(0x102,-1)` fragt nur ab; schreibt nur einen Schatten-Slot, nie einen lebenden `etv_*` | `sdk/apps/apitest/apitest.c:2228-2252@SETEXC`, `:527-541` | Testprogramm |

### 2.2 Treffer, die **kein** Haken sind

| Fundstelle | Was | Einordnung |
|---|---|---|
| `Setexc(0x5A0/4, -1)` in `sdk/apps/teradesk/library/utility/cookie.c:56`, `sdk/apps/highwire/src/AEI.c:72`, `sdk/apps/highwire/src/romvdi.c:46`, `sdk/apps/highwire/src/inet.c:215`, `sdk/apps/highwire/tools/num_char.c:171`, `Applications/z-tools/zweather/inet.c:23` | Cookie-Jar-Zeiger lesen (`-1` = nur Abfrage) | Daten-Sysvar; von `Setexc`-Leseregime bedient (`tos-api/src/bios/vectors.rs:98@setexc`) |
| `Supexec` in `sdk/apps/hypview/init.c:160` (`getBootDrive`), `sdk/apps/hypview/dl_routs.c:325` und `:334` (`ChangeSysKeyconfig` schreibt `conterm`), `sdk/windom/windom-1.21.3/src/vq_tos.c:91`, `sdk/windom/windom-1.21.3/src/get_cookie.c:58`, `sdk/cflib/cflib-21/cookie.c:76`, `sdk/apps/teradesk/library/utility/nf_ops.c:260`, `sdk/apps/highwire/modules/network.src/iconnect/socklib.c:177` | Supervisor-Lesen/Schreiben von Sysvars | kein Haken (`conterm`, `bootdev`, Cookie-Jar, `_sysbase`) |
| `gl_ap_version >= 0x400` u. ae. | AES-Versionsvergleiche | kein Haken |
| `Applications/z-tools`, `thing-neo`, `ldg-2`, `windom`, `doomgeneric`, `pml`, `cflib-git`: Treffer nur `Setexc(0x5A0/4,-1)` (z-tools) und `etv_critic` als Strukturfeldname in `Applications/cflib-git/magx.h:50`, `sdk/apps/hypview/include/magx.h:19` | MagiC-Strukturdeklaration | keine Verwendung |
| `vex_butv/motv/curv/timv/wheelv` | nur in `gemlib` (Bindungen `sdk/gemlib/gemlib-0.44.0/vex_*.c`) | **kein Programm der Portliste ruft sie** — [D] |
| `vblqueue`, `Initmous`, `Mfpint`, `Xbtimer`, `Jdisint`, `Bconmap`, `mousevec`, `statvec`, `clockvec`, `joyvec`, `midivec`, `exec_os`, `dump_vec`, `swv_vec`, `bell_hook`, `kcl_hook`, `hdv_init`, `hdv_boot` | — | **kein Treffer** in der ganzen Portliste — [D] |
| Vanilla-Conquer-ST (`Applications/`) | m68k-only-Bau, kein NewTOS-Ziel (Line-A-Erhebung Tabelle 1 Nr. 8–12) | nicht gezaehlt; Wiederholung der Suche fand keinen Treffer fuer die obigen Muster |

### 2.3 Zusammenfassung des Bestands (Fakten)

- Programme der Portliste, die einen **nicht bedienten** Haken setzen: genau
  **ein**, TeraDesk (`hdv_*`, `etv_critic`, CPU-Vektoren, `resvector`).
- Programme, die einen **bedienten** Haken setzen: Doom (`kbdvec`), `KBDVEC`,
  `P0KID` (`kbdvec`).
- Fuer `etv_timer`, `mousevec`/`statvec`/`clockvec`/`joyvec`, `vblqueue`,
  `bell_hook`, `kcl_hook`, `exec_os`, `dump_vec`, `swv_vec`, `Initmous`,
  `Mfpint`, `Xbtimer`, `Bconmap`-Tabellen, `vex_butv/motv/curv/timv`: **kein**
  Programm im Bestand. Die Haken stehen auf der Liste, weil EmuTOS sie ruft und
  weil drei Audit-Berichte sie als Abweichung fuehren — nicht, weil ein
  portiertes Programm sie braucht.
- Kandidaten fuer 1.0/2.0 (`TASK/IO01.md`, `TASK/NET01.md`ff., Sound): die
  Portliste fuehrt keine Programmnamen, die man auf Haken pruefen koennte; die
  Aussage „kein Kandidat setzt X“ ist deshalb **nicht** belegbar, nur die fuer
  das, was im Baum liegt — [O].

---

## 3. Was NewTOS heute je Haken tut (Arbeit 3)

**Grundmechanismen** (gelesen):

- **Software-Schatten.** `Setexc(num < 0x120)` liest/schreibt eine globale
  Schattentabelle `VECTORS` (`tos-api/src/bios/vectors.rs:56@VECTORS`,
  `:98@setexc`); der Schatten ist **ein** Satz fuer das ganze System (nicht je
  Prozess), 64-Bit-Eintraege; nur die `etv_*` haben Startwerte
  (`ETV_DEFAULT_JUST_RTS`/`ETV_DEFAULT_CRITIC`, Platzhalter, keine
  Code-Adressen — `tos-api/src/bios/vectors.rs:45@ETV_DEFAULT_JUST_RTS`,
  `:54@ETV_DEFAULT_CRITIC`; Befund BIOS-CORE-005, `docs/audits/2026-10-28-bios-core.md`).
  `Setexc(0x120 ≤ num < 0x200)` liest/schreibt den Prozess-eigenen Page-0-Zelle
  `4*num` ueber den Kernel-Alias; ab `0x200` Rueckgabe 0
  (`tos-api/src/bios/vectors.rs:98@setexc`).
- **Seite 0 ist je Prozess privat, im User-Zustand EL0-schreibgeschuetzt, im
  Super-Zustand beschreibbar; „der Kernel behandelt Seite 0 als Ausgabe; er
  liest nur dokumentierte Eingabe-Sysvars (conterm, memvalid …), validiert“**
  (Anwenderentscheidung 2026-10-27, `docs/api/_super-state.md` §0). ADR-028
  Entscheidung 1: nur Seite 0 ist privat, alle anderen Adressen gemeinsam
  (`docs/MEMORY.md`, `## ADR-028`).
- **Befuellung.** `populate_sysvars` schreibt `resvalid`/`resvector` = 0,
  `dumpflg`, `prt_stat`/`prt_vec` = 0 und weitere Daten-Sysvars
  (`tos-api/src/bdos/process/loader.rs:1124@populate_sysvars`,
  `:1195@resvalid`, `:1263@prt_stat`); **kein** Schreibzugriff auf `0x400–0x408`,
  `0x454/0x456`, `0x46A–0x47E`, `0x4A8`, `0x4CE`, `0x4FE`, `0x502`, `0x50E/0x512`,
  `0x51E–0x57E`, `0x5AC/0x5B0` (Durchsicht von `:1188-1285`) — [D]. Diese Zellen
  lesen 0 und werden nie ausgewertet.
- **Upcall-Primitive** (ADR-029): `deliver_upcall` biegt den Rahmen auf einen
  EL0-Handler um, Rueckkehr ueber `Psigreturn`, Zustellung am Syscall-Ruecksprung
  (`kernel/src/arch/aarch64/exceptions.rs:1780@deliver_upcall`,
  `docs/analysis/coop-kernel/10-upcall-signal-design.md` §7.1); Maskenbits
  `UPCALL_MASK_ETV_TERM`/`_AES_USERDEF`/`_VEX_WHEEL`/`_KBDVEC`
  (`tos-api/src/bdos/process.rs:676@UPCALL_MASK_ETV_TERM`,
  `:682@UPCALL_MASK_AES_USERDEF`, `:688@UPCALL_MASK_VEX_WHEEL`,
  `:698@UPCALL_MASK_KBDVEC`); Disposition-Varianten `Resume`,
  `Terminate`, `UserdefReturn`, `SupexecReturn`, `ResumeSuper`
  (`tos-api/src/bdos/process.rs:1212@post_upcall`-Nachbarschaft, Typ `Disposition` oberhalb von `UPCALL_MASK_ETV_TERM`). Verschachtelungsgrenze
  `MAX_UPCALL_NEST`, bei Ueberlauf `Pterm(-998)` (`exceptions.rs:1780@deliver_upcall`).
  **Kein** IRQ-Zustellweg: der Lower-EL-IRQ-Vektor fuehrt nicht zu einem
  Zustell-Check (Design §7.3 „OFFEN/Vorbedingung“, Stand des Dokuments; der
  heutige Code ist nicht erneut darauf geprueft) — [O].

**Je Haken:**

| Haken | Adresse vorhanden? | gelesen / geschrieben / gerufen | Zitat |
|---|---|---|---|
| H01 `etv_timer` | Schatten-Slot `0x100`, Platzhalter-Startwert | `Setexc` setzt/liest; **nie gerufen** (kein Aufrufer von `vector(ETV_TIMER)`); 200-Hz-Zaehler `bios::tick::advance` ruft keinen Haken | `tos-api/src/bios/vectors.rs:149@vector`, `tos-api/src/bios/tick.rs:78@advance`; Audit BIOS-CORE-002 (gemessen: Orakel `calls=100`, NewTOS `calls=0`, `TASK/BIOS05.md`) |
| H02 `etv_critic` | Schatten-Slot `0x101`, Platzhalter | gesetzt/gelesen, **nie gerufen**; NewTOS hat in `Rwabs` keinen Wiederholungs-/Critic-Pfad | `tos-api/src/bios/mod.rs:148@rwabs` (Doc-Kommentar „retry loop / etv_critic chain … out of scope“); `kernel/src/arch/aarch64/exceptions.rs:1393@etv` (Kommentar darueber: „etv_critic (M1) and etv_timer are later cooperative-kernel steps“) |
| H03 `etv_term` | Schatten-Slot `0x102` | **gerufen**: Pterm/Ptermres rufen einen echten Handler als EL0-Upcall vor dem Abbau (`Disposition::Terminate`); Platzhalter/0 = kein Handler | `kernel/src/arch/aarch64/exceptions.rs:1393@etv`, `:1341@handle_pterm` |
| H04 `etv_xtra` | Schatten-Slot `0x103` (`< NVEC`) | gesetzt/gelesen, nie gerufen (EmuTOS ruft ihn auch nicht) | `tos-api/src/bios/vectors.rs:21@NVEC` |
| H05/H06 `bell_hook`/`kcl_hook` | Seite-0-Zelle (Nr. `0x16B`/`0x16C` ueber `Setexc`-Zellen-Regime) | beschreibbar, **nie gelesen**; NewTOS erzeugt weder Klingel noch Tastenklick | `tos-api/src/bios/vectors.rs:98@setexc` (Zellen-Zweig); `/usr/bin/grep -i "bell_hook\|kcl_hook" kernel tos-api/src`: keine Treffer — [D] |
| H07–H09 `hdv_rw/bpb/mediach` | Zellen **nicht befuellt** (0), im Super-Zustand beschreibbar | geschrieben von TeraDesk, **nie gelesen**. GEMDOS ruft die Blockschicht **direkt** (`bcache::device`), nicht ueber BIOS-Vektoren; die BIOS-Traps `Rwabs`/`Getbpb`/`Mediach` sind feste Funktionen | `tos-api/src/bios/mod.rs:148@rwabs`, `:237@mediach`; `tos-api/src/bios/bpb.rs:284@getbpb`; `tos-api/src/bdos/fs/cache.rs:175@block` (GEMDOS-Puffer); in `tos-api/src/bdos` kein `mediach`/`Getbpb`-Aufruf (`/usr/bin/grep -i mediach tos-api/src/bdos`: kein Treffer) — [D]; Doc `tos-api/src/bios/bcache.rs:11@GEMDOS` („under every access“) |
| H10/H11 `hdv_init`/`hdv_boot` | Zellen nicht befuellt | nicht gelesen (EmuTOS ruft sie auch nicht ueber den Vektor) | — |
| H12 `swv_vec` | Zelle nicht befuellt | nicht gelesen; es gibt keinen Monitorwechsel (`CONF_WITH_ATARI_VIDEO`-Pfad) | — |
| H13 `exec_os` | Zelle nicht befuellt | nicht gelesen; NewTOS startet die Shell ueber seinen eigenen Bootpfad (`OSHEADER.os_magic` = 0, `loader.rs:1124@populate_sysvars`-Kommentar „os_magic … left 0“) | `tos-api/src/bdos/process/loader.rs:1163@os_magic` |
| H14 `dump_vec` | `dumpflg` = −1 geschrieben, `dump_vec`-Zelle nicht befuellt | `Scrdmp` ist dokumentiertes No-op; Alt-Help hat keinen Verbraucher | `tos-api/src/xbios/system.rs:65@scrdmp` (Doc: „Installing a custom dump_vec depends on the deferred BIOS vector dispatch“) |
| H15 `prt_*`/`aux_*` | `prt_stat`/`prt_vec` = **0** (EmuTOS: `just_rts`), `aux_*` nicht befuellt | nie gelesen (EmuTOS auch nicht) | `tos-api/src/bdos/process/loader.rs:1263@prt_stat` |
| H16 `vblqueue` | nicht befuellt (`nvbls` = 0, `vblqueue` = 0) | nie gelesen; es gibt keine VBL-Schleife ueber Programmcode | `/usr/bin/grep vblqueue kernel tos-api/src`: keine Treffer — [D] |
| H17 `resvalid`/`resvector` | `0`/`0` geschrieben | **nie gelesen**; Reset ist Kernel-Neustart | `tos-api/src/bdos/process/loader.rs:1195@resvalid`; `/usr/bin/grep -ri resvector kernel tos-api/src` ausser Kommentar: kein Treffer — [D]; Warmstart-Variante `TASK/RST02.md` |
| H18 `con_state` | nein | — (VT52 ist Kernel-intern) | — |
| H19 Geraetetabellen | nein; `Bconmap` meldet „nicht verfuegbar“ (`0x2c`) | — | `tos-api/src/xbios/system.rs` (`bconmap`, bis XBIOS05; seitdem `tos-api/src/xbios/bconmap.rs`) |
| K01 `kbdvec` | privater Tabellenslot je Prozess, Default = EL0-Stub → `Ssystem(2002)` | **gerufen** (Ring je Prozess, Upcall in Super-Zustand, KEYS02/D7/D9) | `tos-api/src/xbios/kbdvecs.rs:352@route_key`, `kernel/src/arch/aarch64/exceptions.rs:802@deliver_pending_kbdvec`, `kernel/src/ikbd.rs:202@route_key` |
| K02–K04, K07–K10 `midivec` … `ikbdsys` | alle neun Slots vorhanden, **alle** mit demselben `ret`-Stub besetzt | gelesen nur `ikbdsys` (Wiederholungsabbruch, `current_ikbdsys`); sonst nie gerufen; Pakete des IKBD werden verschluckt (Status/Uhr/Joystick) bzw. gehen direkt an die VDI-Maus | `tos-api/src/xbios/kbdvecs.rs:147@write_table`, `:232@current_ikbdsys`; `kernel/src/ikbd.rs:219@mouse_event`; Audit XBIOS-CHARDEV-003 (`TASK/XBIOS03.md`: Orakel `mousevec calls=2`, NewTOS 0) |
| K05/K06 `statvec`/`mousevec` | Slot da, `ret`-Stub | nie gerufen. NewTOS' VDI-Maus sitzt **nicht** an `mousevec`: `ikbd::mouse_event` → `vdi::feed_relative` | `kernel/src/ikbd.rs:219@mouse_event`, `tos-api/src/vdi/mouse.rs:189@feed_relative` |
| K11 `mousexvec` | Slot `Kbdvbase()-16` mit `ret`-Stub (EL0-Spiegel); der **lebende** Vektor ist eine Kernel-interne Zelle fuer die VDI | der EL0-Slot wird nie gerufen | `tos-api/src/bios/vectors.rs:185@MOUSEXVEC` (Modulkommentar), `:197@mousexvec` |
| V01/V02 CPU-/Trap-/MFP-Vektoren | Schatten-Slots `0..0x11F` | gesetzt/gelesen, **nie dispatcht** (AArch64 hat keine m68k-Ausnahmen; Modulkopf) | `tos-api/src/bios/vectors.rs:12@AArch64` (Modulkommentar), `:98@setexc` |
| X01 `Initmous` | nicht verdrahtet | Rueckgabe = Funktionsnummer (Katch-all) | `tos-api/src/xbios/mod.rs:26@Initmous`, `:129@Initmous` |
| X02 `Mfpint`/`Xbtimer`/`Jdisint`/`Jenabint` | nicht verdrahtet | Rueckgabe = Funktionsnummer | `tos-api/src/xbios/mod.rs:137@Initmous` (Kommentar nennt `0x0D/0x1A/0x1B/0x1F`) |
| D01 `vex_butv/motv/curv` | Slot (`USER_BUT` …) | gespeichert/zurueckgegeben, **nie ausgefuehrt**, ohne Log | `tos-api/src/vdi/mouse.rs:937@vex_butv` (Kommentar „stored and returned, never executed“), `:919@swap_vector` |
| D02 `vex_timv` | Slot `TIM_ADDR` | wie D01; Rueckgabe `TICKCAL_MS` = 20 | `tos-api/src/vdi/mouse.rs:992@vex_timv` |
| D03 `vex_wheelv` | Slot + Art (`NONE`/`AES`/`EL0`) | **gerufen** als `post_upcall` mit `UPCALL_MASK_VEX_WHEEL` | `tos-api/src/vdi/mouse.rs:967@vex_wheelv`, `:571@call_user_wheel` |
| — `G_USERDEF` | — | gerufen (synchroner Upcall, `UserdefReturn`) | `kernel/src/arch/aarch64/exceptions.rs:888@deliver_userdef_upcall` |
| — `Supexec` | — | gerufen, Rueckgabewert wird `x0` | `kernel/src/arch/aarch64/exceptions.rs:1879@handle_supexec` |

**Die Luecke in der Disziplin „nie still“ heute:** H01, H02, H04–H09,
K02–K10 (EL0-Spiegel), D01–D02 laufen ohne jede Meldung ins Leere
(`Setexc` meldet Erfolg; `vex_*` gibt den alten Wert zurueck). Eine Warnung gibt es
einzig fuer den `kbdvec`-Haken einer Accessory
(`tos-api/src/xbios/kbdvecs.rs:330@accessory_hook_unwarned`,
`kernel/src/arch/aarch64/exceptions.rs:802@deliver_pending_kbdvec`) — das
Vorbild fuer die Warnregel.

---

## 4. ADR-Vorschlag „OS hooks installed by programs“ (Arbeit 4)

**Kein Beschluss.** Aufbau wie ADR-030: Kontext (Abschnitte 1–3 oben, im ADR
gekuerzt), Entscheidung (Tabelle 4.4), Rationale, Folgen. Die Nummer vergibt der
Lead.

### 4.1 Die Mechanismen, auf denen jede Option aufbaut (gelesen)

| Mechanismus | Was er kann | Vorhanden seit |
|---|---|---|
| **N** — Benachrichtigungs-Upcall (fire-and-forget) | EL0-Handler wird am naechsten Syscall-Ruecksprung des Besitzers gerufen, Rueckgabewert verworfen (`Disposition::Resume`) | `etv_term` (H1), `vex_wheelv`, `kbdvec` |
| **S** — synchroner Callout mit Rueckgabewert | EL0-Funktion wird im Rahmen eines Syscalls gerufen, ihr Ergebnis wird `x0` des Syscalls (`SupexecReturn`/`UserdefReturn`) | `Supexec`, `G_USERDEF` |
| **K** — Kette ueber Default-Stub | Der „alte Vektor“ ist eine EL0-Adresse (`ret`-Stub bzw. Stub, der per `Ssystem` in den Kernel-Default springt), damit `old = Setexc(-1)` / `p[-1]`-Idiom kettieren kann | `kbdvec` (`S_NEWTOS_KBDVEC`) |
| **R** — Ring je Prozess | Ereignisse eines IRQ-Pfads werden fuer den Besitzer eingereiht, Zustellung bei dessen naechstem Syscall-Ruecksprung (kein IRQ-Zustellweg noetig) | `kbdvec` (`xbios::kbdvecs`) |

Gemeinsame Randbedingungen, [R]/[D] wie markiert:

- **Zustellzeitpunkt** ist der Syscall-Ruecksprung (Design §7.1). Ein Haken, der
  50 Mal je Sekunde „gleichmaessig“ rufen muss (`etv_timer`, VBL), wird damit
  nicht isochron, sondern **zusammengefasst** (hoechstens ein Aufruf je
  Syscall-Ruecksprung des Besitzers; kein Nachholen) — [D]. Ein Besitzer ohne
  Syscalls (Spiel in einer Schleife) empfaengt nichts — [D]. Ein Weg ueber
  IRQ-Rueckkehr nach EL0 ist laut Design §7.3 nicht vorhanden — [O].
- **Besitz.** Der Schatten `VECTORS` ist global, die `Kbdvbase`-Tabelle je
  Prozess privat (D8/D9, `docs/api/xbios/22-kbdvbase.md`). Ein globaler Haken
  braucht eine Besitzregel: wessen Adressraum/Rechte, was bei `Pterm` des
  Besitzers, was bei `Ptermres` (Haken ueberlebt, der Prozess-Slot nicht —
  hier fehlt der Kontext fuer den Upcall) — [O].
- **Rechte.** Der Handler laeuft mit den Rechten des Besitzers (ADR-028
  Entscheidung 5); ein Aufruf ueber den Rechte-Raum eines anderen Prozesses
  (Kette A→B bei globalem Schatten, `memprot=strict`) ist nicht
  untersucht — [O].
- **Breite.** Seite-0-Zellen sind 32-Bit-LONG; alle Benutzeradressen liegen unter
  `0x7FFF_F000` (ADR-028 Rationale, `TOS_MD_MAX_END`), die Abbildung 64→32 Bit
  ist verlustfrei (so nutzt es der TeraDesk-Overlay `sdk/overlays/teradesk/file.c:26@force_mediach`-Kopfkommentar).
- **Lesen von Seite 0 durch den Kernel** widerspricht der MEM12-Entscheidung
  („Ausgabe-only“). Soll ein Seite-0-Haken wirken, braucht das eine
  Ausnahme — oder einen anderen Einhaengeweg (`Ssystem`-Untercode, Setexc-Schatten).

### 4.2 Warnregel (Vorschlag)

Der Auftrag des Anwenders: „haengt ein Programm einen Haken ein, den NewTOS nicht
bedient, schreibt NewTOS **einmal** eine Warnung ins Log“. Offen ist „einmal je
was“. Optionen:

| Option | Granularitaet | Fuer | Gegen |
|---|---|---|---|
| **W1 (Vorschlag)** | einmal je **(Haken, Prozess)** | Vorbild `accessory_hook_unwarned` (einmal je Slot-Besitz, zurueckgesetzt bei `release_slot`, `tos-api/src/xbios/kbdvecs.rs:330@accessory_hook_unwarned`); TeraDesk-Neustart eines TOS-Programms (`install_critic` je Start) erzeugt keine Meldungsflut, weil derselbe Prozess; zwei Prozesse mit demselben Haken sind zwei Aussagen | ein Prozess mit vielen Haken meldet mehrfach (je Haken einmal) |
| W2 | einmal je Haken und **Boot** | kleinstes Log | verschluckt den zweiten, anderen Verursacher |
| W3 | bei **jedem** Einhaengen | einfach | Flut (`force_mediach` je Shift+ESC) |

**Erkennungsstellen** (je nach Eintrittsweg; Entwurf, [D]):

| Eintrittsweg | Erkennung |
|---|---|
| `Setexc`-Haken (H01–H06, V01/V02) | direkt im `Setexc`-Aufruf (`vector != -1`, Wert ≠ Startwert) |
| Seite-0-Zellen (H07–H18) | Vergleich der Haken-Zellen mit ihrem Startwert beim **Verlassen des Super-Zustands** (`Super(old)`, `Supexec`-Ruecksprung) und bei `Pterm`; moeglich, weil Seite 0 nur im Super-Zustand beschreibbar ist (`docs/api/_super-state.md` S2). Der Kernel **liest** die Zellen nur zum Vergleich, nutzt den Wert nicht |
| `Kbdvbase`-Slots (K02–K11) | beim ersten Ereignis, das den Haken riefe (wie die Accessory-Warnung), oder beim `Pterm` des Besitzers |
| `Initmous`/`Mfpint`/`Xbtimer`/`Jdisint` | im Aufruf selbst (heute stiller Katch-all, `tos-api/src/xbios/mod.rs:137@Initmous`) |
| `vex_butv/motv/curv/timv` | im Aufruf (neuer Wert ≠ `just_rts`-Token) |

Meldungsform (Vorschlag): `[hook] warning: slot <n> (<programm>) installed
<haken> = <adr>; NewTOS does not call it (<grund>, ADR-0xx)`.

**Hooks, die EmuTOS selbst nicht ruft** (H04, H10, H11, H15) und solche, deren
Ausloeser auf NewTOS nie eintreten kann (H12 Monitorwechsel): kein Warnanlass
fuer W1 noetig, weil Verhalten und Orakel gleich sind (H04/H10/H11/H15) —
Vorschlag, Anwenderfrage Q4.

### 4.3 Sicherheit: EL0-Code im Kernpfad (allgemein)

Gelesen oder aus dem Entwurf (`docs/analysis/coop-kernel/10-upcall-signal-design.md`,
`docs/api/_super-state.md`, ADR-028/029):

1. **Kein EL1-Code eines Programms.** Jeder Haken laeuft als EL0-Upcall mit den
   Rechten seines Besitzers; Super-Zustand gewaehrt nur Seite-0-Schreiben
   (S2). Kein Kernel-Zeiger geht an den Handler; Argumente liegen in `x0..x3`
   bzw. in Besitzer-Speicher — Pakete (Maus) in der privaten Seite des
   Besitzers, vom Kernel nur ueber `uaccess` beschrieben (ADR-028 Entscheidung 5).
2. **Adresspruefung.** Eine Hook-Adresse ist programmkontrolliert; sie muss im
   ausfuehrbaren Bereich des Besitzers liegen, bevor der Kernel den Rahmen darauf
   umbiegt. Referenz: FreeMiNT `CHECK_PROGDEF_ADDR`
   (`docs/analysis/coop-kernel/30-aes-thread-design.md:193`). Ob der heutige
   `deliver_upcall` das vor dem Redirect prueft, ist nicht untersucht — [O];
   der Handler faehrt sonst einen EL0-Abbruch (`kill_crashed_program`,
   `kernel/src/arch/aarch64/exceptions.rs:308@kill_crashed_program`) — [D].
3. **Haengende Handler.** Kooperativer Kern, keine Preemption (ADR-029): ein
   Handler, der nicht zurueckkehrt, haelt das System an (wie ein Handler in
   einer m68k-ISR). Bei **synchronen** Callouts (S) in Kernpfaden mit dem
   BDOS-Giant-Lock (`hdv_*`, `etv_critic`) gilt das zusaetzlich fuer die Sperre.
4. **Wiedereintritt.** Ruft ein Handler selbst den Pfad (z. B. ein `hdv_rw`-Haken
   ruft den „alten“ `Rwabs`), muss der Kernel den Haken fuer den Besitzer
   maskieren (Maskenbit je Haken; Muster `upcall_mask`), sonst Rekursion;
   Verschachtelung ist durch `MAX_UPCALL_NEST` begrenzt (`Pterm(-998)`).
5. **Seite-0-Lesen** (Abschnitt 4.1): Ausnahme von „Ausgabe-only“ muss
   ausdruecklich beschlossen werden, validiert (gerade, im Besitzerbereich).
6. **Fremdzustellung.** Ein Ereignis kann eintreffen, waehrend ein anderer Prozess
   laeuft; Zustellung nur, wenn der Besitzer laeuft (Weckpfad
   `wake_for_upcall`, `tos-api/src/bdos/process/sched.rs:904@wake_for_upcall`).

### 4.4 Tabelle je Haken: Empfehlung, Optionen, Aufwand, Sicherheit, Messfall

Aufwandsklassen sind **Schaetzungen [D]**: **S** ≤ 1 Tag, **M** 2–4 Tage,
**L** > 1 Woche. Empfehlung = Vorschlag des Researchers aus dem Beleg, **keine
Entscheidung**. „Beleg“ nennt, wie dick die Evidenz ist (Abschnitt 2).

Wirkungsklassen: **A** wirkt (heute oder nach Aufwand), **B** wirkt mit Semantik
X, **C** wirkt nicht (Grund + Folge + Warnung).

| Haken | Beleg (genutzt von) | Empfehlung | Optionen / Semantik X | Aufwand | Sicherheit | Messfall |
|---|---|---|---|---|---|---|
| H03 `etv_term` | Doom (nur m68k-Zweig), EmuTOS-Pterm | **A (bleibt)** | wie H1 gebaut; ADR nimmt ihn mit auf (Besitzregel: Haken zeitlich an den Besitzer binden, bei `Ptermres` offen, Q3) | 0 | Upcall mit Rechten des Terminierenden (`exceptions.rs:1393@etv`) | NewTOS-Sonde: Pterm mit Handler, der einen Zaehler setzt; Orakel-Gegenstueck fehlt (Audit: „nicht gemessen“) → in Aufwand aufnehmen |
| K01 `kbdvec` | Doom, `KBDVEC`, `P0KID` | **A (bleibt)** | wie KEYS02 | 0 | wie gebaut; Super-Zustand fuer den Handler (S11) | vorhanden: KEYS02-Messung `sdk/examples/kbdvec.c`; `bios-r1` 136 und `xbios-chardev-r1` 37 EQUAL |
| D03 `vex_wheelv` | kein Programm im Bestand | **A (bleibt)** | wie gebaut | 0 | `post_upcall` mit `UPCALL_MASK_VEX_WHEEL` | `tos-api/tests/vdi_vex_wheelv.rs` |
| G_USERDEF, Supexec | AES-Programme / TeraDesk, HypView, WinDom | **A (bleibt)** | — | 0 | — | vorhanden |
| H07–H09 `hdv_rw/bpb/mediach` | **TeraDesk** (`force_mediach`); RAM-Disks/Plattencaches als Klasse, im Bestand ausser TeraDesk keine | **C + Warnung** (Beleg: ein Programm, kleine beobachtbare Folge) | **(a)** C: nicht bedient, einmal warnen (W1). **(b)** B-Teil: nur die BIOS-Traps `Rwabs`/`Getbpb`/`Mediach` rufen den Haken (Mechanismus S, `SupexecReturn`-artig; Lesen der Zelle aus Seite 0 = Ausnahme). GEMDOS bliebe aussen vor, weil es die Blockschicht direkt ruft — ein Haken, der Laufwerk X uebernimmt (RAM-Disk), wuerde fuer BIOS-Aufrufer, aber nicht fuer `Fopen` wirken: **halb bedient ist schlechter als ehrlich nicht bedient** — [D]. **(c)** A: GEMDOS-Blockzugriffe (`fsio`, `cache`, `bpb`) laufen ueber Hook-faehigen Pfad; synchroner Callout aus tiefem Kernel-Code unter Giant-Lock (Muster AES-Thread-Design `30-aes-thread-design.md`); `hdv_mediach` pro Puffer-Wiederverwendung (`bdos/fsbuf.c:179` im Orakel) heisst einen Callout je Block-Zugriff. **(d)** Ersatz statt Haken: TeraDesk-Overlay ersetzt `force_mediach` durch einen NewTOS-Aufruf (z. B. `Ssystem`-Untercode „Medienwechsel erzwingen“) — Programmaenderung nur als `sdk/overlays/`-Patch, keine Kernel-Haken | (a) S; (b) M–L; (c) L (Giant-Lock-/Reentranz-Entwurf offen); (d) S–M | (a) keine; (b)/(c) synchroner EL0-Code im BIOS-/GEMDOS-Pfad unter Giant-Lock (Abschnitt 4.3 Nr. 3, 4), Seite-0-Lesen | Sonde aus BIOS-STOR-010 (`docs/audits/2026-10-28-bios-storage.md`): Super, `hdv_mediach`-Haken liefert 2, `Mediach(2)`, zurueckstellen — auf beiden Seiten; Folgefall: `Fopen` auf dem Laufwerk loest `Getbpb`-Haken aus (Orakel `bdos/fsbuf.c:179`) |
| H02 `etv_critic` | **TeraDesk** (`install_critic`) | **C + Warnung** | **(a)** C: NewTOS kennt keinen kritischen Fehler im Rwabs-Pfad (kein Wiederholen), kein Aufrufer → Haken kann nichts bewirken. Folge fuer TeraDesk: dessen Handler gibt ohnehin nur den Fehlercode zurueck (= „Abbruch“) — NewTOS gibt den Fehlercode direkt zurueck, beobachtbar gleich — [D]. **(b)** A, sobald `Rwabs` einen Wiederholungs-/Critic-Pfad bekommt (BIOS-STOR-Befunde): synchroner Callout (S) mit Rueckgabewert (Abbruch/Wiederholung) | (a) S; (b) M (nach dem Rwabs-Retry-Pfad) | (b) synchroner Callout unter Giant-Lock bei I/O-Fehler | Auf QEMU nicht erzeugbar (Audit Storage §6); Echt-Pi-Liste |
| H01 `etv_timer` | **kein Programm im Bestand**; Audit BIOS-CORE-002 (S2, gemessen) | **offen — Beleg duenn** | **(a)** C + Warnung (aus Beleg: nichts im Bestand). **(b)** B: Mechanismus N, Zustellung zusammengefasst am Syscall-Ruecksprung des Besitzers, Argument `timer_ms` = 20 in `x0`, **ohne** Nachholen; Messfall T01 erwartet dann `calls` ≠ Orakel `100` (Zusammenfassung) — die Abweichung muss das ADR als Vertrag nennen. **(c)** A: Zustellung aus dem 200-Hz-IRQ nach EL0 (Design §7.3 Vorbedingung, `Lower-EL-IRQ`-Weg fehlt) — exakt `calls=100` nur so | (a) S; (b) M; (c) L | (b) wie `kbdvec`; (c) IRQ-Pfad nach EL0 neu | T01 aus `TASK/BIOS05.md` (`calls=100` ueber 400 Ticks) |
| K06 `mousevec` (+ X01 `Initmous`) | **kein Programm im Bestand**; Audit XBIOS-CHARDEV-003 (S2, gemessen) | **offen — Beleg duenn** | **(a)** C + Warnung. **(b)** B nach Muster `kbdvec` (K/R): Default-Slot = EL0-Stub → `Ssystem(S_NEWTOS_MOUSEVEC)` → `vdi::feed_relative`; Pakete je Prozess in einen Ring, Zustellung als Upcall mit Zeiger auf das 3-Byte-Paket in der privaten Seite. **Kette erhaelt die EmuTOS-Semantik**: ein Programm, das filtert/veraendert und kettet, wirkt; eines, das nicht kettet, trennt die VDI-Maus (wie EmuTOS, wo die VDI an `mousevec` haengt, `vdi/vdi_mouse.c:605`). **(c)** B-schwach: nur Benachrichtigung (Original geht an die VDI, Aenderungen am Paket wirkungslos) — Beschleuniger wirken nicht, Beobachter schon | (a) S; (b) M–L (gleiche Maschinerie wie KEYS02, plus Paketkopie); (c) M | wie `kbdvec`: EL0-Handler mit Besitzerrechten; Paketpuffer nur ueber `uaccess`; Maus-Input-Latenz = naechster Syscall des Besitzers | Eingabesonde X04 (`TASK/XBIOS03.md`, `calls=2`) |
| K05 `statvec`, K07 `clockvec`, K08 `joyvec` | **kein Programm**; W04 (gemessen: EmuTOS `1/1/1`, NewTOS `0/0/0`, Wurzel von XBIOS04 nicht trennbar) | **C + Warnung, bis XBIOS04 geklaert** | Auf NewTOS entsteht kein Statuspaket (XBIOS04: Ursprung des Pakets fehlt), also nichts zu rufen. Nach XBIOS04: wie K06 (b) | S jetzt; M danach | wie K06 | Sonde W04 |
| K02 `midivec`, K04 `vmiderr`, K09 `midisys` | **kein Programm**; kein MIDI-Geraet (`Midiws` verwirft Bytes, `tos-api/src/xbios/system.rs:82@midiws`) | **C + Warnung** | Grund: kein MIDI-ACIA (`CONF_WITH_MIDI_ACIA`-Quelle fehlt); sobald `TASK/IO01.md` einen MIDI-Treiber liefert, B (wie K06) | S | — | keiner (kein Geraet) |
| K03 `vkbderr`, K10 `ikbdsys` | **kein Programm** | **C + Warnung** | `vkbderr`: Ueberlauf tritt auf USB-HID nicht auf; `ikbdsys`: kein ACIA-Interrupt (D4 in `22-kbdvbase.md`); `ikbdsys` wird als Wert gelesen (Wiederholungsabbruch) — Hook, der Rohbytes vorfiltert, wirkt nicht | S | — | Abfrage `Kbdvbase`-Slots (V01–V03 gleich) |
| K11 `mousexvec` | **kein Programm**; nicht ueber `Kbdvbase` erreichbar | **C, keine Warnung** | liegt vor dem Tabellenzeiger; nur durch Adress-Arithmetik erreichbar | 0 | — | — |
| H05/H06 `bell_hook`/`kcl_hook` | **kein Programm**; NewTOS hat keinen Klang | **C + Warnung, bis Klang (1.0) existiert** | spaeter B (Mechanismus N) | S–M | — | spaeter |
| H14 `dump_vec` | **kein Programm** | **C + Warnung** | A fuer `Scrdmp`: synchroner Callout (S, wie `Supexec`) — wenig Aufwand, aber Seite-0-Lesen und kein Drucker | S | Seite-0-Lesen | `Scrdmp` mit Haken |
| H12 `swv_vec`, H13 `exec_os`, H17 `resvalid/resvector` | **kein Programm** ausser TeraDesk-Shutdown, das beide auf 0 setzt | **C + Warnung** | H12: kein Monitorwechsel (`CONF_WITH_ATARI_VIDEO`); H13: eigener Bootpfad (AUTO ist konsolenspezifisch, `docs/MEMORY.md`-Vermerk nicht zitiert, hier nicht belegt) — [O]; H17: nach `TASK/RST02.md` (Warmstart) als B pruefen | S | — | — |
| H16 `vblqueue` | **kein Programm** | **C + Warnung** | A erforderte eine VBL-Schleife ueber Programmcode (Mechanismus N je 50/60 Hz) plus ein schreibbares `vbl_list`-Array im Speicher der Programme | M–L | Schreibzugriff auf OS-Array | — |
| V01 Trap-Vektoren `#1/#2/#13/#14`, V02 Autovektoren/MFP/Ausnahmen | **TeraDesk** (nur `lobo`-Shutdown: VBL `0x70`, Timer C `0x114`) | **C + Warnung** | Grund: AArch64 kennt keinen m68k-`TRAP`/Autovektor; ein Programm, das sich vor `Trap #1/#13/#14` haengt, kann sie nicht abfangen. Gleiche Logik wie ADR-030 Entscheidung 1. `lobo`: Endlosschleife im Super-Zustand statt Reset/Halt — beobachtbar: Rechner haengt in `loopcpu` (Besitzer-Prozess), nicht der gewollte Halt — [D] | S | — | — |
| X02 `Mfpint`/`Xbtimer`/`Jdisint`/`Jenabint` | **kein Programm** | **C + Warnung** | Grund: kein MFP (Nicht-Ziel „kein Hardware-Emulation“, `CLAUDE.md`). Option B: software-emulierte Timer A–D als Mechanismus N — nur bei Bedarf (Spiele) | S (C) / L (B) | — | — |
| D01 `vex_butv/motv/curv`, D02 `vex_timv` | **kein Programm** | **C + Warnung** (aus „still gespeichert“ wird „gespeichert + gemeldet“) | `vex_motv` liefert eine geaenderte Position zurueck (Mechanismus S noetig); `vex_butv` Mechanismus N+Rueckgabe; Begruendung der Luecke (`tos-api/src/vdi/mouse.rs:937@vex_butv`-Kommentar, ARCH03 3a F4) bleibt | S (Warnung) / M–L (B) | — | `tos-api/tests` (nur Rundlauf des Vektors) |
| H10/H11/H15/H04 | kein Aufruf auch in EmuTOS | **A wie EmuTOS (nicht gerufen), keine Warnung** | NewTOS' Seite-0-Werte `prt_stat/prt_vec` = 0 statt `just_rts` weichen vom Orakel ab (Beobachtung unten) | 0 | — | — |
| H18 `con_state`, H19 `bconout_vec`-Tabellen | **kein Programm**; `Bconmap` nicht verfuegbar | **C + Warnung bei Direktschreibzugriff nicht erkennbar** | Tabellen werden nicht publiziert → Programm liest 0 und scheitert laut (Zeiger 0) — [D] | 0 | — | — |

### 4.5 Offene Anwenderentscheidungen

- **Q1 — `hdv_*`:** (a) nicht bedient + Warnung (Bestand: nur TeraDesk,
  Folge klein), (b) halb bedient (BIOS-Traps), (c) voll bedient (grosse
  Arbeit, GEMDOS-Blockschicht), (d) Ersatz-Aufruf plus TeraDesk-Overlay.
  Empfehlung des Researchers: **(a)**, dazu pruefen ob (d) fuer die TeraDesk-
  Funktion „Medienwechsel erzwingen“ nutzbar sein soll. (b) wird abgeraten
  (Abschnitt 4.4).
- **Q2 — Beleg-duenne Haken** (`etv_timer`, `mousevec` und Verwandte): bedienen
  (B) obwohl kein Programm im Bestand sie braucht, oder (C + Warnung) bis ein
  Programm sie belegt? Die Audit-Befunde BIOS-CORE-002 und XBIOS-CHARDEV-003
  bleiben in beiden Faellen DIFF; bei C muessen sie im ADR als bewusste
  Abweichung stehen. Zu `etv_timer`: B liefert hoechstens zusammengefasste
  Aufrufe; exakt wie das Orakel nur mit IRQ-Weg nach EL0.
- **Q3 — Besitz und TSR:** globaler Schatten (heute) mit Besitzer-Aufraeumen bei
  `Pterm`, oder je Prozess (wie `Kbdvbase`)? Und was geschieht mit einem Haken
  eines `Ptermres`-Prozesses (Slot weg, Code resident)? Bleibt offen, bis das
  untersucht ist — [O].
- **Q4 — Warnumfang:** W1/W2/W3 (Abschnitt 4.2); keine Warnung fuer Haken, die
  EmuTOS selbst nicht ruft (H04, H10, H11, H15)?
- **Q5 — Seite-0-Lesen:** darf der Kernel Haken-Zellen aus Seite 0 lesen
  (nur zum Warnen; oder auch zum Rufen) — Ausnahme von der MEM12-Entscheidung?
  Ohne Ausnahme faellt (b)/(c) der `hdv_*`-Optionen weg.
- **Q6 — `prt_stat`/`prt_vec`-Startwert:** 0 (heute) oder ein gueltiger
  Einsprungpunkt wie EmuTOS' `just_rts` — Programme, die den alten Wert fuer
  Ketten lesen, bekommen sonst 0 (Beobachtung unten).

### 4.6 Folgen fuer Tickets (nach der Entscheidung, Lead)

`TASK/BIOS05.md` (`etv_timer`), `TASK/XBIOS03.md` (`mousevec` …), ein neues
`hdv_*`-Ticket (BIOS-STOR-010), die Warn-Infrastruktur als gemeinsames Ticket
(Erkennungsstellen 4.2), und `docs/COMPATIBILITY.md` (Eintrag „OS hooks“ wie
„No Line-A“ bei ADR-030).

---

## 5. Nicht belegt / needs deeper analysis

1. **Zustellung ohne Syscall** (IRQ nach EL0): Design §7.3 beschreibt die
   Vorbedingung; ob sie heute gebaut ist, wurde in dieser Sitzung nicht erneut
   gegen `kernel/src/arch/aarch64/exception_vectors.s` geprueft.
2. **Adresspruefung in `deliver_upcall`:** nicht gelesen, ob der Handler-Eintritt
   auf den ausfuehrbaren Bereich des Besitzers geprueft wird.
3. **Globaler Haken ueber Prozessgrenzen** (Kette A→B, `memprot=strict`,
   `Ptermres`): nicht untersucht.
4. **`exec_os`:** wie NewTOS' Bootpfad die ROM-Shell waehlt, wurde nicht
   verfolgt; die Aussage „nicht gelesen“ gilt fuer die Haken-Zelle, nicht fuer
   den gesamten Bootpfad.
5. **Kandidaten fuer 1.0/2.0:** ohne Programmliste nicht bewertbar (Abschnitt 2.3).
6. **`bell_hook`/`kcl_hook`-Nummern** (`0x16B`/`0x16C`) sind gerechnet
   (`0x5AC/4`, `0x5B0/4`), nicht gemessen.

## 6. Out-of-scope observations (je eine Zeile)

- `prt_stat`/`prt_vec` stehen in NewTOS auf 0, in EmuTOS auf `just_rts`
  (`tos-api/src/bdos/process/loader.rs:1263@prt_stat` gegen `bios/chardev.c:78-83`);
  `aux_stat`/`aux_vec` fehlen — ein eigener, kleiner Beleg-Unterschied, nicht
  verfolgt.
- Das Doc-Kommentar-Kopf von `tos-api/src/xbios/mod.rs:26@Initmous` nennt `Supexec`
  als „nicht verdrahtet“, `handle_supexec` im Kernel bedient es seit MEM12
  (`kernel/src/arch/aarch64/exceptions.rs:1879@handle_supexec`) — veralteter
  Kommentar.
- AES-`G_PROGDEF`/`ub_code` ist der dritte Hook-Typ mit Programmcode im
  Kernpfad (bedient, `UPCALL_MASK_AES_USERDEF`); in diesem ADR nur als
  „bleibt“ gefuehrt, Spec-Pflege liegt bei der AES-Spec.
- `etv_term` ist auf NewTOS ein globaler Schatteneintrag (Audit-Beobachtung
  `docs/audits/2026-10-28-bios-core.md`, „Nicht verfolgt“): ob er einen
  Folgeprozess erreicht, ist Teil von Q3.

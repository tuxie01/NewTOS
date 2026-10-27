# MEM12 — EL0 darf Seite 0 beschreiben

Status: **OFFEN** · Prioritaet: **P1** · Typ: Kernfehler (Schutzgrenze fehlt) — Anwenderentscheidung 2026-10-26

## Sachverhalt

Seite 0 `[0, 0x1000)` ist in jedem Prozess `UserRwExec` abgebildet
(`tos-api/src/bdos/process/mmu.rs:643`), in jedem Speicherschutzmodus
(`tos-api/src/bios/reset.rs:26`). Ein EL0-Programm kann dort also lesen,
schreiben und ausfuehren.

Auf dem Atari loest ein Zugriff auf die unteren Adressen im Usermodus einen
Busfehler aus; die Systemvariablen sind nur im Supervisormodus erreichbar.
Anwenderentscheidung 2026-10-26: **EL0 darf Seite 0 nie beschreiben.** Ob EL0
sie lesen darf, haengt davon ab, ob native Programme dort Systemvariablen
lesen; das klaert dieses Ticket durch Suche und Messung. Liest keines, wird
Seite 0 fuer EL0 ganz ausgeblendet.

Was heute in Seite 0 liegt (`docs/COMPATIBILITY.md`, Abschnitt „One shared user
address space, page 0 per process“): Systemvariablen, OSHEADER mit `os_run`,
Cookie-Jar-Spiegel, Trampoline, `Keytbl`-Bereich, KEYS02-Tabelle, die
Vektortabelle bei `0x9A0`, der Reset-Stub (`RESET_STUB_VA`,
`tos-api/src/bios/reset.rs`) und der `S_NEWTOS_KBDVEC`-Stub. Seite 0 ist je
Prozess eigen.

Bekannte Folge: `__aline` ist per ADR-030 NULL; `__aline->Feld` mit positivem
Offset liest Seite 0 und schreibt dorthin, statt zu fehlen
(`docs/COMPATIBILITY.md`, Abschnitt „No Line-A“).

## Fragen, die das Ticket mit Belegen beantwortet

1. **Lesen:** Welche nativen Programme (`programs/`, `sdk/apps/`, mintlib,
   gemlib, cflib, AUTO-Programme, tosdump) lesen Adressen in `[0, 0x1000)` —
   direkt, ueber `_sysbase`/`Setexc`/`Supexec`-Rueckrufe oder ueber mintlib-
   Makros? `file:line` je Fundstelle; wo die Suche nicht reicht, messen
   (Seite 0 testweise ausblenden, Boot bis Desktop und die Anwendungen der
   Karte starten, Datenabbrueche protokollieren).
2. **Ausfuehren:** Welche Stubs in Seite 0 springt EL0 an (Reset-Stub,
   KBDVEC-Stub, Trampoline)? Brauchen sie Ausfuehrungsrecht in Seite 0, oder
   koennen sie in eine eigene, nur ausfuehrbare Seite umziehen?
3. **Schreiben:** Schreibt heute irgendein natives Programm nach Seite 0
   (z. B. `conterm` nach `Super()`)? Was macht EmuTOS dann im Supervisormodus,
   und was soll NewTOS stattdessen tun (Fehler, Prozessende)? Liest der Kern
   selbst Werte aus der Seite 0 des Prozesses zurueck, die ein Programm
   veraendert haben koennte?

## Befund zu 1–3 (2026-10-27, HEAD `a90bcfb0`)

Methode: Quellsuche mit `/usr/bin/grep -rnI` ueber `sdk/apps`, `sdk/overlays`,
`sdk/examples`, `sdk/libc` (mintlib + `newtos-arch`), gemlib, cflib, WinDom,
LDG, zView, libshare, `programs/`, `tosdump/`, `kernel/`, `tos-api/` (Muster:
Adressen `0x0..0x67F` hinter einem Zeiger-Cast, `Super`/`Supexec`/`Setexc`,
`_sysbase`, `p_cookies`, `_hz_200`, `conterm`, `get_sysvar`/`set_sysvar_*`,
`Kbdvbase`, `Keytbl`, `page0_kva`); jede Fundstelle gelesen. Dazu eine Messung
mit Seite 0 testweise `UserRoExec` (EL0 lesen + ausfuehren, nicht schreiben):
`measurements/measurement-2026-10-27-mem12/RUNINFO.TXT`.

Vorbemerkung: `Super()`/`Supexec()` heben auf NewTOS nichts an; `Super` ist
eine Klammer ohne Wirkung, `Supexec` ein gewoehnlicher Aufruf bei EL0
(`sdk/libc/overlay/include/mint/osbind.h:539@Super`). Jeder "Supervisor-Zugriff"
eines Programms auf die Systemvariablen ist also ein EL0-Zugriff auf Seite 0.

### 1. Lesen — es gibt Leser; Ausblenden geht nicht

| Leser | Stelle | Was |
|---|---|---|
| mintlib `clock()` (jedes Programm mit `clock`/Zeitmessung) | `sdk/libc/mintlib/time/clock.c:15,21@get_sysvar` → `sdk/libc/newtos-arch/getsysvar.c:36@get_sysvar` | `_hz_200` `0x4BA`, direkte Ladung |
| DOOM | `sdk/apps/doom/doomgeneric_gem.c:675@get_sysvar`, `:1858-1866@kbd_probe` | `_hz_200`; `_sysbase` `0x4F2` → OSHEADER `os_version`/`os_dummy` |
| PRESENT | `sdk/apps/present/present.c:934@get_sysvar` | `_hz_200` |
| TeraDesk | `sdk/apps/teradesk/library/utility/gettos.c:40-42@os_version` | `_sysbase` → `os_version` |
| WinDom (zView) | `sdk/windom/overlay/src/vq_tos.c:83@syshead`, `sdk/windom/overlay/src/cookieptr.c:48@__cookieptr` | `_sysbase`, `_p_cookies` `0x5A0` |
| zView | `sdk/zview/z-tools-04805b7/zview/chrono.c:25,45@Super` | `_hz_200` |
| cflib (QED) | `sdk/cflib/cflib-21/cookie.c:60-63@get_cookie_adr` | `_p_cookies` |
| `Keytbl`-Bereich (`0x768`) | TeraDesk `sdk/apps/teradesk/library/xdialog/xdevent.c:76@Keytbl`, cflib `sdk/cflib/cflib-21/nkccinit.c:99-102@Keytbl`, `sdk/cflib/cflib-21/menucre.c:38@Keytbl`, `sdk/cflib/cflib-21/fdfindsc.c:48@Keytbl`, WinDom `sdk/windom/windom-1.21.3/src/keybd2ascii.c:48@Keytbl`, HypView `sdk/overlays/hypview/dl_init.c:245@Keytbl`, DOOM `:1930`, NEWDESK `programs/desktop/src/main.rs:1350@Keytbl` | lesen die Tabellen, die `Keytbl` zurueckgibt |
| Testprogramme | `sdk/examples/cmdline.c:40@run_cell`, `fppexec.c`, `ptermch.c`, `uakid.c`, `mpkid.c`, `e10usr.c`, `e10kid.c`, `crchild.c`; `sdk/apps/apitest/apitest.c:896-937@sysbase` | `_sysbase` → `os_run`; APITEST T03–T06, SX04 |

Gemessen: mit Seite 0 nur lesbar laufen AUTO (`MXPROBE.NCP`), beide
Accessories, NEWDESK, QED, PRESENT, zView, DUMPVDI, VDIDRAW, ARGVTEST an
(ohne Eingabe). **Ergebnis 1: Seite 0 muss fuer EL0 lesbar bleiben**
(mindestens `[0x400, 0x670)` und der `Keytbl`-Bereich).

### 2. Ausfuehren — fuenf Stubs, alle prozessunabhaengig

| Stub | VA | Wer springt hin |
|---|---|---|
| Upcall-Trampolin | `0x700` | Kern setzt `lr` darauf (`kernel/src/arch/aarch64/exceptions.rs:1791@TRAMPOLINE_VA`) |
| AES-`G_USERDEF`-Stub | `0x714` | Kern als Upcall-Handler (`exceptions.rs:849@USERDEF_TRAMPOLINE_VA`); gemessen in TeraDesk (`pc=0x724`) |
| `just_rts` / `kbdvec_default` | `0xA00` / `0xA08` | Werte der `Kbdvbase`-Tabelle; DOOM kettet an den alten `kbdvec` (`sdk/apps/doom/doomgeneric_gem_kbdvec.c:57@kbdvec_old`) |
| Reset-Stub | `0xA40` | OSHEADER `reseth`/`os_res1`; TeraDesk "Reboot" (`sdk/overlays/teradesk/main.c:2120,2142@rv`) |

Der Code aller fuenf ist konstant (`TRAMPOLINE_CODE`, `USERDEF_STUB_CODE`,
`KBDVEC_DEFAULT_CODE`, `RET_STUB_CODE`, `RESET_STUB_CODE`); nichts darin haengt
vom Prozess ab. Sie brauchen Ausfuehrungsrecht, aber nicht in Seite 0: eine
**gemeinsame** Seite, fuer EL0 nur ausfuehrbar (`AP=00`, `UXN=0`, ARM ARM
Stage-1-Rechte), an einer festen VA unter 4 GiB (die OSHEADER-Felder sind 32
Bit) reicht fuer alle Prozesse. Bleiben sie in Seite 0, muss Seite 0
`UserRoExec` sein (Daten ausfuehrbar).

### 3. Schreiben — es gibt Schreiber, davon zwei im Kern selbst

EmuTOS: `$0..$7FF` sind auf dem Atari nur im Supervisor-Modus erreichbar; ein
Programm schreibt sie nach `Super()`/aus `Supexec`, und der Schreibzugriff
gelingt. Im Usermodus loest die Hardware einen Busfehler aus, EmuTOS beendet
das Programm (`any_vec` → `dopanic` → `kill_program`, `bios/vectors.S:152-153`,
`bios/kprint.c:302`, `:587-594`, `bios/panicasm.S:96-99`). NewTOS mit Seite 0
nur lesbar: Berechtigungsfehler bei EL0 → `kill_crashed_program`
(`kernel/src/arch/aarch64/exceptions.rs:1412@kill_crashed_program`):
Absturzbericht, Taste, `Pterm(-1)`, der Kern laeuft weiter (gemessen, s. u.).
Ein Systemaufruf mit Ausgabepuffer in Seite 0 endet ebenso: `uaccess` prueft
mit den Rechten des Aufrufers (`tos-api/src/uaccess.rs:453@user_kptr`).

| Schreiber | Stelle | Was | Folge bei Seite 0 nur lesbar |
|---|---|---|---|
| **Kern-Stub** `G_USERDEF` | `tos-api/src/bdos/process/loader.rs:1447,1452@USERDEF_STUB_CODE` | `str w0, [0x760]` (Rueckgabewort von `ub_code`) | **gemessen**: TeraDesk endet beim ersten Zeichnen, `pc=0x724 far=0x760` |
| **Kern-Ablage** PARMBLK `0x730` | `tos-api/src/aes/object.rs:2876@PARMBLK_SCRATCH_VA` | der Kern schreibt, `ub_code` liest — und darf schreiben: WinDom `xedit` aendert `pb_x`/`pb_w` (`sdk/windom/windom-1.21.3/src/userdef.c:1819-1820@pblk`); in EmuTOS liegt der PARMBLK auf dem beschreibbaren AES-Stapel | WinDom-`XEDIT` endet; in zView kein `XEDIT` gefunden |
| DOOM | `sdk/apps/doom/doomgeneric_gem.c:1884,1896@kbdvec_hook` | haengt den `kbdvec`-Haken in die `Kbdvbase`-Tabelle (`0x9A8`) | DOOM endet beim Start (Tastatur ueber `kbdvec`) |
| `sdk/examples/kbdvec.c` | `:182,211@hook` | dasselbe (KEYS02-Testprogramm) | endet |
| HypView | `sdk/apps/hypview/dl_routs.c:307-312@conterm`, Aufrufer `sdk/apps/hypview/window.c:320,512@KeyboardOnOff` | `conterm` `0x484` bei **jedem** Tastendruck im Fenster: `Ssystem(S_GETBVAL)` liefert auf NewTOS `EINVFN` (`tos-api/src/gemdos/ssystem.rs:130@EINVFN`), daher der `Supexec`-Rueckfall | HypView endet beim ersten Tastendruck |
| TeraDesk "Reboot" (kalt) | `sdk/overlays/teradesk/main.c:2114-2125@memval` | `memvalid`, `memval2`, `resvector`, `resvalid` = 0, dann Sprung in den Reset-Stub | TeraDesk endet statt Neustart (RST01 rueckwaerts) |
| TeraDesk Shift+Esc / nach Formatieren | `sdk/apps/teradesk/file.c:945-969@hdv_rw`, Aufrufer `sdk/apps/teradesk/window.c:3284@force_mediach`, `sdk/apps/teradesk/floppy.c:878@force_mediach` | `hdv_rw`/`hdv_bpb`/`hdv_mediach` `0x472..0x47E` (NewTOS ruft sie ohnehin nie) | TeraDesk endet |

Nicht gebaut oder tot: TeraDesks `install_cookie` (`#if 0`), LDGs TSR-Kern
(`cookiec.c`, nicht in `sdk/ldg/build-ldg.sh`), WinDoms Keksschreiber (Overlay
nimmt `Ssystem(S_SETCOOKIE)`), mintlib `set_sysvar_to_*` (gebaut, kein Aufrufer).

**Liest der Kern zurueck, was ein Programm in Seite 0 geschrieben hat?** Ja:

| Wert | Leser im Kern | Wirkung |
|---|---|---|
| `conterm` `0x484` | `tos-api/src/bios/chardev.rs:317@current_conterm` → Tastenwiederholung `kernel/src/ikbd.rs:336@current_conterm`, `Bconin`-Shiftfaltung `chardev.rs:373@current_conterm`, Glocke `tos-api/src/vdi/escape.rs:693@current_conterm` | Bits 1/2/3 wirken |
| `kbdvec`, `ikbdsys` | `tos-api/src/xbios/kbdvecs.rs:198@read_slot`, `:212@kbdvec_of`, `:221@current_ikbdsys` | Haken-Upcall, Wiederholstopp |
| `memvalid`/`memval2`/`memval3` | `tos-api/src/bios/reset.rs:160@entry_kind` | warm/kalt (auf der Pi warm = kalt) |
| Rueckgabewort `0x760` | `kernel/src/arch/aarch64/exceptions.rs:897@read_userdef_ret_slot` | `G_USERDEF`-Ergebnis |
| beliebige Zelle `< 0x800` | `tos-api/src/bios/vectors.rs:98-133@setexc` | `Setexc(n, v)` liest/schreibt sie ueber den Kern-Alias — der API-Weg, kein EL0-Speicherzugriff |

### Entscheidungen, die vor dem Umbau noetig sind (offen, beim Lead)

Seite 0 nur lesbar ist mit dem heutigen Inhalt nicht zu haben, ohne dass
TeraDesk (gemessen), DOOM, HypView und das KEYS02-Testprogramm enden.
Vorschlag des Developers, je Punkt mit Alternativen:

- **E1 Rechte von Seite 0.** Vorschlag `UserRoNx` (lesen, nicht schreiben,
  nicht ausfuehren); die fuenf Stubs ziehen in **eine gemeinsame, fuer EL0 nur
  ausfuehrbare Seite**. Alternative: `UserRoExec`, Stubs bleiben (kleinster
  Eingriff, Daten bleiben ausfuehrbar).
- **E2 Was beschreibbar bleiben muss, zieht um.** `Kbdvbase`-Tabelle
  (`0x9A0`, TOS-Idiom, DOOM) und PARMBLK (`0x730`) in **eine zweite private
  Seite je Prozess**, `UserRwNx`, an fester VA unter 2 MiB, die nie geteilt
  abgebildet wird (Pi: unter dem Kernbild `0x80000`; virt: unter 1 GiB).
  `alias_of`/`resolve_kva`/`user_kptr` (`tos-api/src/uaccess.rs:482@alias_of`,
  `tos-api/src/bdos/process/usermem.rs:336@page0_kva`) muessen sie wie Seite 0
  behandeln, `map_shared` muss die VA verweigern. Aendert den Ort aus
  `docs/api/xbios/22-kbdvbase.md` D1 ("in page 0"), nicht Layout oder Wirkung.
  Das Rueckgabewort `0x760` entfaellt: der Stub uebergibt `w0` im Register an
  `Psigreturn` (kernintern, ohne Speicher).
- **E3 `conterm` (HypView).** (a) FreeMiNT `Ssystem` `S_GETBVAL`/`S_SETBVAL`
  (+ W/L) fuer den Sysvar-Bereich umsetzen (sanktionierte Erweiterung, braucht
  Spec; HypView nimmt dann diesen Weg und schreibt nie); (b) HypView endet beim
  Tastendruck hinnehmen; (c) Overlay-Patch fuer HypView.
- **E4 TeraDesk "Reboot" und `force_mediach`.** (a) Overlay-Patch: auf NewTOS
  die Schreibzugriffe weglassen (der Kern ruft `hdv_*` nie; warm/kalt ist auf
  der Pi gleich); (b) TeraDesk endet hinnehmen.
- **E5 `Setexc(n < 0x200, v)`** schreibt weiter ueber den Kern in Seite 0
  (TOS-API-Weg, EmuTOS erlaubt ihn). Bestaetigen oder einschraenken.

Nebenbefund: der Absturzbericht nennt einen Schreibversuch auf Seite 0
"Memory violation: hardware memory" (`memprot::region_kind` kennt Seite 0
nicht) — beim Umbau mitbenennen.

## Arbeit

- Ergebnis von 1–3 als Abschnitt in diesem Ticket.
- Seite 0 fuer EL0 nicht beschreibbar machen (mindestens `UserRo`/`UserRx`,
  je nach 2); ohne Leser ganz ausblenden.
- Ein Schreibzugriff von EL0 nach Seite 0 beendet das Programm, der Kern
  laeuft weiter (Test).
- Das Ergebnis in `docs/COMPATIBILITY.md` festhalten (Seite-0-Abschnitt und
  der Hinweis im Abschnitt „No Line-A“).

## Abnahme

- Hosttest bzw. Selftest: EL0-Schreiben nach Seite 0 endet mit Prozessende,
  nicht mit Kernpanik; Lesen verhaelt sich wie unter 1 festgelegt.
- Frischer Bau beider Boards, QEMU-Boot bis Desktop (raspi4b mit
  `serial=on`); die Anwendungen der Karte starten wie vorher.
- `docs/COMPATIBILITY.md` beschreibt den neuen Stand.

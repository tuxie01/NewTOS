# MEM12 — EL0 darf Seite 0 beschreiben

Status: **ERLEDIGT** 2026-10-30 ([Quittung](MEM12-STATUS-DONE.MD); Pi-Lauf des Anwenders 2026-10-29, Kern `58e35ac8`) · Prioritaet: **P1** · Typ: Kernfehler (Schutzgrenze fehlt) — Anwenderentscheidung 2026-10-26

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
(`sdk/libc/overlay/include/mint/osbind.h` (Stand `a90bcfb0`; seit MEM12 wieder Traps)). Jeder "Supervisor-Zugriff"
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
| cflib (QED) | `sdk/cflib/cflib-21/cookie.c:90-92@get_cookie_adr` | `_p_cookies` |
| `Keytbl`-Bereich (`0x768`) | TeraDesk `sdk/apps/teradesk/library/xdialog/xdevent.c:76@Keytbl`, cflib `sdk/cflib/cflib-21/nkccinit.c:99-102@Keytbl`, `sdk/cflib/cflib-21/menucre.c:38@Keytbl`, `sdk/cflib/cflib-21/fdfindsc.c:48@Keytbl`, WinDom `sdk/windom/windom-1.21.3/src/keybd2ascii.c:48@Keytbl`, HypView `sdk/overlays/hypview/dl_init.c:245@Keytbl`, DOOM `:1930`, NEWDESK `programs/desktop/src/main.rs:1350@Keytbl` | lesen die Tabellen, die `Keytbl` zurueckgibt |
| Testprogramme | `sdk/examples/cmdline.c:40@run_cell`, `fppexec.c`, `ptermch.c`, `uakid.c`, `mpkid.c`, `e10usr.c`, `e10kid.c`, `crchild.c`; `sdk/apps/apitest/apitest.c:896-937@sysbase` | `_sysbase` → `os_run`; APITEST T03–T06, SX04 |

Gemessen: mit Seite 0 nur lesbar laufen AUTO (`MXPROBE.NCP`), beide
Accessories, NEWDESK, QED, PRESENT, zView, DUMPVDI, VDIDRAW, ARGVTEST an
(ohne Eingabe). **Ergebnis 1: Seite 0 muss fuer EL0 lesbar bleiben**
(mindestens `[0x400, 0x670)` und der `Keytbl`-Bereich).

### 2. Ausfuehren — fuenf Stubs, alle prozessunabhaengig

| Stub | VA | Wer springt hin |
|---|---|---|
| Upcall-Trampolin | `0x700` | Kern setzt `lr` darauf (`kernel/src/arch/aarch64/exceptions.rs:1833@TRAMPOLINE_VA`) |
| AES-`G_USERDEF`-Stub | `0x714` | Kern als Upcall-Handler (`exceptions.rs:903@USERDEF_TRAMPOLINE_VA`); gemessen in TeraDesk (`pc=0x724`) |
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
(`kernel/src/arch/aarch64/exceptions.rs:1454@kill_crashed_program`):
Absturzbericht, Taste, `Pterm(-1)`, der Kern laeuft weiter (gemessen, s. u.).
Ein Systemaufruf mit Ausgabepuffer in Seite 0 endet ebenso: `uaccess` prueft
mit den Rechten des Aufrufers (`tos-api/src/uaccess.rs:453@user_kptr`).

| Schreiber | Stelle | Was | Folge bei Seite 0 nur lesbar |
|---|---|---|---|
| **Kern-Stub** `G_USERDEF` | `tos-api/src/bdos/process/loader.rs`, `USERDEF_STUB_CODE` (Stand `a90bcfb0`; der Speicherbefehl ist seit MEM12 entfernt) | `str w0, [0x760]` (Rueckgabewort von `ub_code`) | **gemessen**: TeraDesk endet beim ersten Zeichnen, `pc=0x724 far=0x760` |
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
| `conterm` `0x484` | `tos-api/src/bios/chardev.rs:479@current_conterm` → Tastenwiederholung `kernel/src/ikbd.rs:343@current_conterm`, `Bconin`-Shiftfaltung `chardev.rs:514@current_conterm`, Glocke `tos-api/src/vdi/escape.rs:693@current_conterm` | Bits 1/2/3 wirken |
| `kbdvec`, `ikbdsys` | `tos-api/src/xbios/kbdvecs.rs:209@read_slot`, `:212@kbdvec_of`, `:221@current_ikbdsys` | Haken-Upcall, Wiederholstopp |
| `memvalid`/`memval2`/`memval3` | `tos-api/src/bios/reset.rs:160@entry_kind` | warm/kalt (auf der Pi warm = kalt) |
| Rueckgabewort `0x760` | `kernel/src/arch/aarch64/exceptions.rs`, `read_userdef_ret_slot` (Stand `a90bcfb0`; seit MEM12 entfernt) | `G_USERDEF`-Ergebnis |
| beliebige Zelle `< 0x800` | `tos-api/src/bios/vectors.rs:98-133@setexc` | `Setexc(n, v)` liest/schreibt die Zelle fuer `n` in `0x120…0x1FF` ueber den Kern-Alias — der API-Weg, kein EL0-Speicherzugriff; fuer `n < 0x120` (auch `$40C–$47F`) liest/schreibt es den Schatten `VECTORS[]`, nicht die Systemvariable (BIOS-CORE-006, [BIOS07](BIOS07.md); Nachtrag 2026-10-28 aus BIOS13). *Seit BIOS07 (2026-10-29):* jede Zelle `n < 0x200` ist eine Seite-0-Zelle — `n <= 0x103` eine Zelle fuer alle Prozesse mit Kopie in jeder Seite 0, darueber die Systemvariable des aufrufenden Prozesses (`docs/api/bios/05-setexc.md`, D3) |

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
  `tos-api/src/bdos/process/usermem.rs`, `resolve_kva`) muessen sie wie Seite 0
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
  *Nachtrag 2026-10-28 (BIOS13):* gilt nur fuer `n` in `0x120…0x1FF`; fuer
  `n < 0x120` schreibt `Setexc` den Schatten `VECTORS[]` (BIOS-CORE-006,
  `bios-core-r1` 58–62, Sonde S06–S08; [BIOS07](BIOS07.md)).
  *Nachtrag 2026-10-29 (BIOS07):* jetzt fuer alle `n < 0x200` (D3); die
  Vektorzellen `n <= 0x103` sind eine Zelle fuer alle Prozesse, jede Seite 0
  traegt ihre Kopie; der Schreibweg bleibt der Kern-Alias. MEM12-Sonde P0PAR
  danach erneut PASS (`measurements/measurement-2026-10-29-bios07/RUNINFO.TXT`).

Nebenbefund: der Absturzbericht nennt einen Schreibversuch auf Seite 0
"Memory violation: hardware memory" (`memprot::region_kind` kennt Seite 0
nicht) — beim Umbau mitbenennen.

### Anwenderentscheidungen 2026-10-27 (woertlich)

> E1: (a). E2: yes; also specify whose Kbdvbase table receives an input event (owner/foreground). E5: yes.
> E3/E4: no Ssystem and no patches for now. Map the Atari protection model onto page tables instead:
> - page 0: EL0 read-only in user state, read-write while the process is in super state (inside Super()/Supexec), never executable
> - write in user state → bus error, as on Atari
> - HypView conterm and TeraDesk Reboot/Shift+Esc then run unpatched, if they write in super state. Verify per program (source + probe kernel) before building.
> - cold vs warm start: the Reset stub reads memvalid, as on Atari
> - the kernel treats page 0 as output only; it reads only documented input sysvars (conterm, memvalid …), validated
> Ssystem S_GETBVAL/S_SETBVAL: later, as a FreeMiNT extension (ADR-027), oracle = FreeMiNT in Hatari.
> The acceptance run must include the interactive paths (qemu-virt): HypView key, TeraDesk Reboot + Shift+Esc, DOOM keyboard.

Damit gilt: E1 = Seite 0 `UserRoNx`, Stubs in eine eigene, fuer EL0 nur
ausfuehrbare Seite; E2 = zweite private RW-Seite je Prozess (Kbdvbase-Tabelle,
PARMBLK), dazu wird festgelegt, wessen Tabelle ein Eingabeereignis bekommt;
E5 = `Setexc` bleibt; E3/E4 = Supervisor-Zustand je Prozess (Seite 0 dann
EL0-RW), keine Overlay-Patches, kein `Ssystem` jetzt. Vor dem Bau: Spec fuer
`Super`/`Supexec` (Regel 5) und Freigabe.

Spec-Entwurf (2026-10-27, wartet auf Freigabe): `docs/api/_super-state.md`
(Super-Zustand), `docs/api/xbios/22-kbdvbase.md` Teil D (Ort der Tabelle,
Empfaenger D9).

### Super/Supexec in den Programmen (2026-10-27)

Quelle und gebaute Binaerdatei (`llvm-objdump -d`, Stand der Karte):

| Pfad | Quelle: im Super-Zustand? | Binaerdatei heute |
|---|---|---|
| HypView `conterm` | ja: `Supexec(ChangeSysKeyconfig)` (`sdk/apps/hypview/dl_routs.c:325,334@Supexec`), nach `Ssystem(0x0C)` = `EINVFN` | `KeyboardOnOff` @`0x594c`: `svc` Ssystem 12, dann der Callee **eingebettet**: `ldrb/strb [0x484]` @`0x59a8/0x59b4` |
| TeraDesk "Reboot" | ja: `Super(0L)` (`sdk/overlays/teradesk/main.c:2114@Super`), Schreibzugriffe `:2121-2124`, dann `Supexec(rv)` `:2142` | (Pfad nur per Menue; Super-Makro erzeugt keinen Befehl, s. naechste Zeile) |
| TeraDesk Shift+Esc | ja: `Super(0L)` … `Super(stack)` um die `hdv_*`-Zugriffe (`sdk/apps/teradesk/file.c:945-969@hdv_rw`) | `force_mediach` @`0xb3f4`: **kein** Befehl fuer `Super`; 8-Byte-`str` nach `0x476`, `0x472`, `0x47E` @`0xb490/0xb49c/0xb4ac`, `Fopen`/`Fclose` per `svc` dazwischen |
| DOOM `kbdvec` | ja: `Supexec(kbd_install)` (`sdk/apps/doom/doomgeneric_gem.c:1939@kbd_install`) | `kbd_install`/`kbd_probe` ohne eigenes Symbol, eingebettet |

Folgerung: alle vier Schreiber stehen in der Quelle im Supervisor-Klammer; mit
den heutigen SDK-Makros wird daraus gewoehnlicher EL0-Code ohne jeden Trap. Kein
Kern — auch kein Probe-Kern — kann in diesen Binaerdateien einen
Super-Zustand sehen. Erst nach Umstellung der Makros auf Traps und Neubau der
Programme ist "schreibt im Super-Zustand" messbar; die Messung je Pfad gehoert
deshalb hinter den Bau (Probe-Kern zum Abgleich der Fehlerstelle: s. Plan).

Nebenbefund TeraDesk (LP64-Fehler im unveraenderten `file.c`, ausserhalb
MEM12): `hdv_bpb`/`hdv_rw`/`hdv_mediach` sind 8-Byte-Zeiger an 4 Byte
Abstand. `hdv_bpb = …` ueberschreibt die obere Haelfte von `hdv_rw`, die
Rueckpruefung `hdv_rw == Newrwabs` schlaegt fehl, die alten Werte werden nie
zurueckgeschrieben; `hdv_mediach` (8 Byte ab `0x47E`) setzt dabei `conterm`
(`0x484`) in TeraDesks Seite 0 auf 0 (Tastenklick, Wiederholung, Glocke aus).
**Behoben 2026-10-27 (Pruefpunkt 1):** Patchdatei `sdk/overlays/teradesk/file.c`
(die drei Zellen als `uint32_t`, Umwandlung ueber `uintptr_t`); Beleg
`measurements/measurement-2026-10-27-mem12-td-lp64/RUNINFO.TXT` (objdump vorher/
nachher; raspi4b: nach Shift+Esc `conterm` 0 → bleibt 7).
Ebenso behoben (Lead, `0edf53d0`): TeraDesks `_sysbase`-Lesen in `gettos.c`
war 8 Byte breit (nahm `_shell_p` `$4F6` als obere Haelfte mit); Patchdatei
`sdk/overlays/teradesk/library/utility/gettos.c` liest die 32-Bit-Zelle.

### Interaktive Abnahme: qemu-virt geht nicht, raspi4b mit Injektor (Plan)

qemu-virt hat kein `C:`: `cargo xtask run --sd` haengt das Abbild nur fuer
raspi4b an (`tools/xtask/src/main.rs:996@sd`), der Kern hat fuer virt keinen
Blocktreiber (`kernel/src/drivers/qemu_virt/` = PL011 + ramfb), und HypView,
TeraDesk, DOOM sind keine eingebetteten Programme. `.claude/agents/tester.md`
("qemu-virt has input but no `C:`") stimmt also noch. Weg: raspi4b mit dem
Kratz-Injektor `tools/inject-arm.py` (Ausloeser `launch:NAME`/`up:NAME`),
wie RST01 TeraDesk "Reboot" schon gefahren hat
(`measurements/measurement-2026-10-17-rst01/RUNINFO.TXT` Abschnitt 4). Alle vier
Pfade warten in AES-Schleifen (`evnt_*`), in denen der Pump laeuft; keiner
haengt in `Cconin`. Je Pfad eine Karte (#Z) und ein Skript: HypView (+ eine
`.HYP`-Datei, Taste im Fenster), TeraDesk Shift+Esc (Laufwerksfenster oeffnen,
Shift+Esc) und Quit → Reboot (`-no-reboot`: "[reset] … (Cold)" + QEMU-Ende),
DOOM (+ WAD aus `Applications/doom-wad`, Tasten ueber den `kbdvec`-Haken).

### Anwenderentscheidungen 2026-10-27, zweite Runde (woertlich)

> 1. S1–S12 approved. Add measurement cases for S3–S6 against the oracle: Super(1) return values, Super(0)/SuperToUser returns (address class), Supexec passes the callback's return value through.
> 2. O1: yes, the hook runs in super state (as EmuTOS from the interrupt).
> 3. O2: (a), documented deviation.
> 4. D9: yes. If an accessory ever installs a kbdvec hook: log a warning, so the deviation never bites silently.
> 5. Yes, raspi4b with inject-arm.py. Plus one run on the real Pi before MEM12 counts as done: toggling page-0 permissions needs correct TLB maintenance (TLBI + DSB/ISB), and QEMU often hides missing invalidations.
> 6. Yes, probe per program after the rebuild.
> TeraDesk LP64 bug: fix it before enabling MEM12. Once the kernel reads conterm (S10), TeraDesk's zeroed conterm turns off click, repeat and bell system-wide. This is a porting bug in our TeraDesk port, not an app patch for NewTOS.
> Real card: only after the real-Pi acceptance run, with all programs rebuilt in one go (manifest).

Ablauf in drei Pruefpunkten (Lead, 2026-10-27): (1) TeraDesk-LP64-Fehler
`hdv_*` beheben; (2) Messfaelle S3–S6 am Orakel, vor jeder Kernaenderung;
(3) Umbau nach `docs/api/_super-state.md`, Proben je Programm, Abnahmelauf,
Prueffolge fuer die echte Pi.

**Pruefpunkt 2 (2026-10-27): Orakelseite der Faelle S3–S6.** Dumper-Erweiterung
(`docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6, "Super and Supexec in a
BIOS-area table": Trap 1 nur fuer `Super`, Argumente `f:<n>`/`t:0`,
Klammerregel), Tabelle `tosdump/cases/super-r1.tsv`, Referenzen
`tosdump/reference/EMUTOS-1a7484ab-SUPER` und `-SUPER-falcontc`; Beleg
`measurements/measurement-2026-10-27-mem12-super-oracle/RUNINFO.TXT`. EmuTOS:
`Super(1L)` 0 / −1 / 0, `Super(0L)` = altes SSP, `SuperToUser` ohne
definiertes `d0` (IGNORE, Zustand ueber die folgende Abfrage gemessen),
`Supexec` reicht 0x12345678 und −2 durch, `Super(1L)` im Callback = −1. Die
NewTOS-Seite folgt nach Pruefpunkt 3.
Hosttests dieses Pruefpunkts (`cargo xtask test` auf `a7ffd707`): der
Beleglauf verwies auf einen Chat-Bericht, der nicht mehr vorliegt; der
committete Lauf bleibt unveraendert. Nachgemessen 2026-10-28 aus
`git archive` mit eigenem Target: **1706 / 0 / 1**
(`measurements/measurement-2026-10-28-mem12-hosttest/RUNINFO.TXT`; gitignorierte
Eingaben `references/` und `sdk/apps/teradesk` dafuer aus dem heutigen Baum
gelinkt, dort benannt).

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

## Abnahmestand 2026-10-27 (Pruefpunkt 3b, QEMU)

Lauf `measurements/measurement-2026-10-27-mem12-acceptance/RUNINFO.TXT`
(Kern `34530791`, alle Programme in einem Zug neu gebaut, Karten mit Manifest):

- **EL0-Schreiben nach Seite 0 endet mit Prozessende, nicht mit Kernpanik:**
  Probe `sdk/examples/p0par.c` + `sdk/examples/p0kid.c` (raspi4b) — Schreiben
  im User-Zustand, Ausfuehren aus Seite 0, Lesen und Schreiben der Stub-Seite,
  Schreiben direkt nach dem Verlassen des Super-Zustands: je "Bus Error",
  `Pterm(-1)`, `0xFFFF` beim Elternprozess; Lesen, `Super`-Klammer,
  `Supexec`, `Kbdvbase`-Haken, Ende im Super-Zustand: 0; "P0PAR: PASS".
- **Proben je Programm** (neu gebaut, unveraendert gegenueber upstream bis auf
  die LP64-Patchdateien): HypView-Taste → `Supexec(ChangeSysKeyconfig)`,
  `conterm` 7 → 5 → 7 im Super-Zustand; TeraDesk Shift+Esc → `Super`-Klammer
  um `hdv_*`, `conterm` bleibt 7; TeraDesk "Reboot" → `Super(0L)`,
  `memvalid` = 0, `Supexec(Reset-Stub)` → "(Cold) requested"; DOOM →
  `Supexec(kbd_install)`, Haken in der privaten Seite (`Kbdvbase` `0x7F010`),
  F10 + `y` nur ueber den Haken → Ende mit 0.
- **Orakel S3–S6:** `super-r1.tsv` NewTOS gegen EmuTOS: 0 von 9 Faellen
  verschieden. APITEST SX01–SX09 OK (105/0/9).
- **Frischer Bau beider Boards, Boot bis Desktop:** raspi4b (`serial=on`) und
  qemu-virt; die Programme der Karte (NEWDESK, TeraDesk, QED, PRESENT, zView,
  VDIDRAW, HypView, DOOM) starten ohne Absturz.
- **Offen:** Lauf auf der echten Pi (Anwenderentscheidung 2026-10-27,
  Punkt 5), Prueffolge unten. Erst danach gilt MEM12 als erledigt und kommt
  auf die Alltagskarte.

## Prueffolge echte Pi (fuer Anwender/Lead; der Developer fasst keine Hardware an)

Zweck: die TLB-Pflege beim Umschalten von Seite 0 (RO ↔ RW) auf echtem
Silizium; QEMU verdeckt fehlende Invalidierungen oft.

1. **Karte** (eine Testkarte, nicht die Alltagskarte): `kernel8.img` aus
   `cargo xtask build --board rpi4 --release` des committeten Stands;
   `cmdline.txt` mit `serial=on`, falls ein UART-Adapter haengt. Auf `C:`
   aus dem einen Neubau (`measurements/measurement-2026-10-27-mem12-acceptance/method/build-programs.sh`):
   `P0PAR.NAP` + `P0KID.NAP` (Wurzel), TeraDesk (`C:\Apps\TERADESK\`, wie
   `tools/make-sd-image.sh`), HypView mit `HYP_VIEW.CFG` + `README.TXT`
   (`C:\APPS\HYPVIEW\`, Inhalte im Lauf unter method), DOOM + `DOOM1.WAD`
   (`C:\DOOM\`), `DUMPBIOS.NAP` + `tosdump/cases/super-r1.tsv` als `C:\BIOS.TSV`.
   Ein Manifest der Karte (`tools/sd-manifest.sh`) gehoert zum Lauf.
2. **Boot:** Desktop erscheint; seriell `[desk] desktop up`.
3. **P0PAR** starten (Doppelklick): fuenf Absturzberichte nacheinander
   ("Memory violation: page-0 system-variable memory, write …",
   "… execute …", "execute-only stub memory, read/write", wieder "page-0 …
   write"), jeden mit einer Taste beantworten. Ergebnis auf dem Schirm:
   alle Zeilen `OK`, "own state after L: user OK", **"P0PAR: PASS"**. Ein
   `WRONG` bei Fall `S` (Schreiben im Super-Zustand wird abgewiesen) oder bei
   Fall `T` (Schreiben nach dem Verlassen gelingt) waere ein TLB-Fehler.
4. **HypView** starten, mehrmals Pfeil-runter im Fenster: kein Absturz;
   danach Tastenwiederholung (Taste halten) und Tastenklick wie vorher.
5. **TeraDesk** starten, Laufwerk C: oeffnen, mehrmals Shift+Esc: kein
   Absturz, Klick/Wiederholung bleiben an. Dann Strg+Q → "Reboot": die Pi
   startet neu (seriell `[reset] OSHEADER reset entry (Cold) requested`).
6. **DOOM** starten, mit Pfeiltasten laufen, F10 → `y`: zurueck zum Desktop,
   Tastatur des Desktops arbeitet.
7. **DUMPBIOS** (Doppelklick) schreibt `C:\BIOS.BIN`; auf dem Host
   `tosdump/host/build/tosdiff tosdump/cases/super-r1.tsv tosdump/reference/EMUTOS-1a7484ab-SUPER.BIN BIOS.BIN`
   → "0 of 9 cases differ".
8. **USB-Tastatur → `Iorec(1)`-Ring** (aus XBIOS01, Anwender 2026-10-29): Tasten
   auf der USB-Tastatur landen im IKBD-Ring von `Iorec(1)` (`Bconstat(2)` -1,
   `tail` waechst in 4-Byte-Schritten; Sonde aus
   `measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`).
9. **Sonde K16 mit physischer Umschalttaste:** erwartet ein Wert der Klasse
   `0x02300042` (Shift-Bit im oberen Byte gesetzt, Shift zum Zeitpunkt des
   Tastendrucks; QEMU-Wert aus dem XBIOS01-Lauf).
10. Ergebnis mit Seriellprotokoll und Fotos in einem Lauf unter
   `measurements/` und in `docs/HW-TEST-LOG.md`.

## Abschluss (Anwender, 2026-10-30)

- MEM12 ist abgeschlossen mit Verweis auf den Pi-Lauf des Anwenders vom
  2026-10-29 (Release-Kern `58e35ac8`, enthaelt MEM12): Desktop, Programme und
  Tastatur liefen im Alltag; dabei fiel der DOOM-Freeze auf ([DOOM01](DOOM01.md)).
- Die Prueffolge oben ist nicht formal durchlaufen. `P0PAR` (TLB-Pflege beim
  Umschalten von Seite 0) wandert als erster Diagnoseschritt nach
  [DOOM01](DOOM01.md) und in das spaetere Testszenario fuer die Tester.
- Die Schritte 10/11 (AUX-Warten aus BIOS09, RXD0 aus UART01) sind entfernt;
  beide Themen sind geparkt bzw. gehen in das Testszenario.


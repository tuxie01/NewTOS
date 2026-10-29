# Maschinenidentitaet (`_MCH`) — Bestand, Beleg und ADR-Vorschlag (2026-10-28)

**Zweck:** Beleg und Entscheidungsvorlage fuer `TASK/ARCH05.md`, Arbeit 1, 2, 4, 5, 6.
Punkt 3 (ADR-012 ueberarbeiten) ist Sache des Lead nach der Anwenderentscheidung.
Diese Datei **entscheidet nichts**; wo der Beleg duenn ist, steht es so da.

**Stand:** EmuTOS-Pin `references/emutos` @ `1a7484ab` (`docs/analysis/ORACLE.md`);
NewTOS `main` @ `0afde760`. Datierter Schnappschuss.

**Legende.** **[R]** = im Quelltext oder in der Messdatei dieser Sitzung gelesen.
**[D]** = abgeleitet; traegt nur, was in der Herleitung steht. **[O]** = offen,
„needs deeper analysis“. EmuTOS-Zitate sind Zeilenzitate (gepinnt), NewTOS-Zitate
tragen den Symbolanker.

**Ausgangslage (Anwender 2026-10-28).** ADR-012 (`docs/MEMORY.md`, Abschnitt
„Consequences“, Zeilen 386-390) haelt fest: `_MCH` bleibt als NewTOS-Maschinenkennung,
`_VDO`/`_SND`/`_FDC`/`_FPU` entfallen, `NTOS` kommt dazu. **Der Wert steht im ADR nicht.**
`0x0005_0000` ist die vorlaeufige Abweichung davon (`tos-api/src/cookie.rs:221-226@seed_defaults`).

---

## 1. Bestand der Kennungs-Cookies

### 1.1 Was NewTOS meldet [R]

`seed_defaults` setzt genau fuenf Cookies, sonst setzt nichts im Kernel einen
(`/usr/bin/grep` ueber `kernel/src`, `tos-api/src`, `programs`: einziger Aufruf
`kernel/src/lib.rs:362@seed_defaults`; Selbsttest erwartet fuenf,
`kernel/src/selftest_cookie.rs:63@Seeded`):

| Cookie | NewTOS-Wert | Stelle | Orakel `tt` | Orakel `falcontc` | Orakel `disk` |
|---|---|---|---|---|---|
| `NTOS` | `0x00000100` (Version) | `tos-api/src/cookie.rs:222@NTOS`, Wert `kernel/src/lib.rs:362@seed_defaults` | fehlt [D] (EmuTOS kennt ihn nicht, `include/cookie.h:21-39`) | fehlt [D] | fehlt [D] |
| `_MCH` | `0x00050000` | `tos-api/src/cookie.rs:223@MCH` | `0x00020000` (MCH_TT) | `0x00030000` (MCH_FALCON) | nicht gemessen; Falcon-Maschine (`ORACLE.md` §2) -> `0x00030000` [D] |
| `_IDT` | `0x0000112F` | `tos-api/src/cookie.rs:224@IDT` | `0x0000112E` | `0x0000112E` | nicht gemessen |
| `_AKP` | `0x00000000` | `tos-api/src/cookie.rs:225@AKP` | `0x00000100` | `0x00000100` | nicht gemessen |
| `EdDI` | `0x00000001` (Marker) | `tos-api/src/cookie.rs:226@EDDI`, `:58@EDDI_MARKER` | fehlt | fehlt | nicht gemessen |
| `_CPU` | **fehlt** (Absicht) | `docs/HARDWARE-PROFILE.md` §3.7 | `0x0000001E` (30) | `0x0000001E` | nicht gemessen |
| `_VDO` | **fehlt** (ADR-012) | ebd. | `0x00020000` | `0x00030000` | nicht gemessen |
| `_FPU` | **fehlt** (ADR-012) | ebd. | `0x00060000` | `0x00000000` | nicht gemessen |
| `_SND` | **fehlt** (ADR-012) | ebd. | `0x00000003` | `0x0000001F` | nicht gemessen |
| `_FDC` | **fehlt** (ADR-012) | ebd. | `0x01415443` | `0x01415443` | nicht gemessen |
| `_SWI` | fehlt | ebd. | `0x000000BF` | `0x000000BF` | nicht gemessen |
| `_FRB`, `XHDI`, `SCSI` | fehlen | — | vorhanden | vorhanden | nicht gemessen |
| `_FLK` | fehlt (ADR-012 sagte „behalten“, Nachtrag 2026-10-26: nicht gesetzt) | `docs/MEMORY.md` Nachtrag | fehlt | fehlt | nicht gemessen |

Zur Spalte „Orakel“ — **was gemessen ist und was nicht:**

- **Gemessen [R], aber nicht auf dem Pin.** Es gibt **keine** Cookie-Tabelle im Orakel
  und keinen Cookie-Fall in `tosdump/cases/*.tsv` (Suche nach `cookie`/`_MCH`: nur
  der `Setexc(0x168,-1)`-Fall `tosdump/cases/bios-core-r1.tsv:60@Setexc`, der nur
  „Zeiger im RAM“ prueft). Die Zeilen `tt`/`falcontc` stammen aus der EdDI-Sonde
  `SCRNINFO` (Abschnitt „1. COOKIE JAR“):
  `measurements/measurement-2026-10-02-scrninfo/hatari-tt-ttmed/SCRNINFO.TXT` und
  `measurements/measurement-2026-10-02-scrninfo/hatari-falcon-tc/SCRNINFO.TXT`,
  Hatari 2.6.1, **EmuTOS 1.4** (`etos512de.img`, sha256 `5805c0a4...`,
  `measurements/measurement-2026-10-02-scrninfo/RUNINFO.TXT`) — **nicht** die gepinnte ROM
  (`1a7484ab`, sha256 `323c87bb...`). Beide Jars haben je 12 Eintraege in der
  Reihenfolge `_CPU _VDO _FPU _MCH _SWI _SND _FRB _AKP _IDT _FDC XHDI SCSI`.
- **Die Werte folgen aus Code, der im Pin unveraendert ist** [D]:
  `setvalue_mch` (`bios/machine.c:382-405`) liefert ohne ARAnyM-Namen
  (`machine.c:386`) bei `has_videl` `MCH_FALCON`, sonst bei `has_tt_shifter`
  `MCH_TT` (`:388-391`). Gleiche Quelle, anderes Datum: erwartbar gleich auf dem Pin,
  **gemessen ist es dort nicht.** Das ist die Luecke, die Fallvorschlag 5.3 schliesst.
- **Echte TT als Gegenprobe** [R]: `measurements/measurement-2026-10-04-tt-nova/SCRNINFO.TXT`
  (TOS 3.06): `_MCH = 0x00020000`, `_VDO = 0x00020000`, `_CPU = 30`, `_SND = 3`, 15 Eintraege
  inkl. `NOVA`/`USHD`/`VSCR`/`EdDI` — gleicher `_MCH` wie `tt`.
- Die Werte von `_AKP = 0x0100` und `_IDT = 0x112E` sind die der **`COUNTRY=de`-ROM**
  (`ORACLE.md` §1: `make 512 COUNTRY=de`); `_AKP` kommt aus NVRAM, wenn dieses lesbar ist
  (`bios/country.c:140-162`), sonst aus dem Landesdefault. NewTOS meldet `_AKP = 0`
  (US) und `_IDT = 0x112F` (`/`-Trenner). Das sind Unterschiede der Voreinstellung, nicht
  der Maschine; hier nur erfasst (Cookie `_AKP`: `docs/COMPATIBILITY.md` kennt ihn
  bereits als offen).
- **Das Jar der NewTOS-Seite gemessen** [R]:
  `measurements/measurement-2026-10-02-eddi/newtos-qemu-raspi4b/SCRNINFO.TXT`:
  `NTOS _MCH _IDT _AKP EdDI`, in dieser Reihenfolge, Terminator-Wert 16 (Spiegel-Kapazitaet,
  `tos-api/src/bdos/process/loader.rs:951@COOKIE_JAR_SLOTS`).

### 1.2 Was `_MCH` auf einem Atari bedeutet [R]

`references/emutos/include/cookie.h:50-57`: `MCH_NOHARD 0xFFFFFFFF`, `MCH_ST 0`,
`MCH_STE 0x00010000`, `MCH_MSTE 0x00010010`, `MCH_TT 0x00020000`, `MCH_FALCON 0x00030000`,
`MCH_MILAN_C 0x00040000`, **`MCH_ARANYM 0x00050000`**. FreeMiNT kennt nur bis Milan
(`references/freemint/sys/cookie.h:92-100`).

---

## 2. Beleg: Wer liest welchen Cookie, und was geschieht je Wert

Methode: `/usr/bin/grep -rnIaE` ueber `sdk/` (ohne `/build/`), `programs/`, `tosdump/`,
`kernel/`, `tos-api/` und die Quellen unter `Applications/` (`qed`, `doomgeneric`,
`cflib-git`, `thing-neo-codeSVN-r62`, `ldg-2`, `pml-2.03`, `Conquer-VDI`, `windom-1.21.3`);
Muster: `Getcookie|getcookie|get_cookie|find_cookie|S_GETCOOKIE|_p_cookies|0x5a0|_MCH|C__MCH|0x5F4D4348`
und jeder Cookiename. Jeder Treffer gelesen. Nicht durchsucht: das Innere von
`references/freemint` (haengt) — fuer Punkt 4 nur die benannten Dateien. Gebaut/ausgeliefert
nach `docs/analysis/os-hooks-adr-proposal-2026-10-28.md` §2 (Portliste).

### 2.1 Programme, die `_MCH` lesen — und die einzige Verzweigung

| Programm | Cookie | Was es tut | Datei:Zeile | `0x00050000` (heute) | `0xFFFFFFFF` | `0x00030000` | gebaut? |
|---|---|---|---|---|---|---|---|
| **zView** JPEG-Plugin | `_MCH` | `if (mach == 0x50000L)` -> „ARAnyM erkannt“, `dsp_ram = 3`, `return` (TT-RAM fuer DSP). **Gesamter Block nur, wenn `_JPD` existiert** (`jpgdrv`), den NewTOS nicht setzt | `sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:807@mach` (Lesen), `:811@mach` (Vergleich), `:813@dsp_ram` | **Zweig wuerde genommen, wenn `_JPD` je existierte** (`:801@Getcookie`); heute tot | Zweig nicht genommen, dann `_CPU`-Pfad `:818@cpu` | Zweig nicht genommen | ja (zView) |
| **mintlib** `sysinfo(SI_PLATFORM)` / `uname().machine` | `_MCH` | `hi = (_mch & 0xffff0000) >> 16`, `switch (hi)`: 0 ST, 1 STe, 2 TT, 3 Falcon (bzw. FireBee mit `_CF_`), 4 Milan, **5 „aranym“/Provider „aranym“**, `default` „atariclone“ | `sdk/libc/mintlib/unix/sysinfo.c:515@Getcookie`, `:516@hi`, `:520-558@platform` | Plattform „aranym“, Provider „aranym“ [D] | **`case -1` setzt `platform` nur bei ColdFire (`:520-525`), sonst bleibt es `NULL`, danach `strlen(platform)` (`:562@platform_len`) -> Null-Zeiger-Zugriff** [D] | Plattform „falcon“, Provider „atari“ [D] | libc ja (`sdk/libc/build/obj/unix__sysinfo.o` vorhanden); ein **aufrufendes Programm** der Portliste habe ich nicht gefunden (`uname`/`SI_PLATFORM` nur in `sdk/libc/mintlib/unix/uname.c:43@SI_PLATFORM`) — [O] ob je ein Aufrufer existiert |
| mintlib `sysinfo(SI_PLATFORM)`, Familie ausserhalb 0-5 | `_MCH` | `default`: `platforms[_PLATFORM_CLONE]` mit `_PLATFORM_CLONE = 9`; das Feld `platforms[]` hat **neun** Eintraege, weil bei `"cfevb"` ein Komma fehlt (`"cfevb" "atariclone"` verschmolzen) -> Index 9 ausserhalb | `sdk/libc/mintlib/unix/sysinfo.c:461@_PLATFORM_CLONE`, `:463-474@platforms`, `:555-558@default` | n/a | n/a (Familie -1 hat eigenen `case`) | n/a | bei einer **eigenen Familie** (z. B. `0x0006....`) Lesefehler [D] |

**Ergebnis zu „numerischen Vergleichen, die einen eigenen Wert in TT/Falcon-Hardwarepfade
schicken“:** Es gibt **keinen** Treffer der Form `mch >= 2` oder `(mch>>16) >= 3` in
`sdk/apps`, `sdk/overlays`, `programs/`, `sdk/examples`, `tosdump`, `kernel`, `tos-api`
und den durchsuchten `Applications/`-Quellen. Die beiden Treffer sind oben: zView
(`== 0x50000`, tot ohne `_JPD`) und mintlib `sysinfo` (Namenstabelle, kein Hardwarepfad).
Der einzige `>=`-Vergleich auf `_MCH` im ganzen Pin ist EmuTOS' eigener
`util/boot.c:129` (`get_cookie(_MCH) >= MCH_TT`, nur im Bauziel `TARGET_256`).

### 2.2 Programme, die andere Kennungs-Cookies lesen

| Programm | Cookie | Verhalten bei fehlendem / vorhandenem Cookie | Datei:Zeile | Beleg |
|---|---|---|---|---|
| **TeraDesk** (ausgeliefert) | **`_VDO`** | `find_cookie(C__VDO,&vdo)`; `vdo` ist `static long vdo` (0) und `find_cookie` fasst `*value` bei „nicht gefunden“ **nicht** an (`sdk/apps/teradesk/library/utility/cookie.c:39-59`), also `vdohi = 0` = `ST_VIDEO` (`sdk/apps/teradesk/video.c:66`) -> Zweig `st_ste` (Overscan-Suche, `Blitmode(-1)` `:247-248`). Der Kommentar `:157` („wird 0xffffffff ohne Cookie“) stimmt mit dem Code nicht ueberein. Bei `_VDO >> 16 == 3` oder `5` dagegen `VsetMode(-1)`/`VgetMonitor()` (`:172-176`); `5` = „Aranym ?“ (`:71`) | `sdk/apps/teradesk/video.c:158@find_cookie`, `:160@vdohi`, `:172-176@VsetMode`, `:202@ST_VIDEO` | [R] Code; **Wirkung auf NewTOS nicht gemessen** [O] |
| TeraDesk | `_FLK` | `flock = find_cookie(C__FLK, NULL)` — fehlt auf NewTOS (wie auf EmuTOS) | `sdk/apps/teradesk/xfilesys.c:1865@find_cookie` | [R] |
| TeraDesk | `MiNT`, `Gnva`, `nAES`, `MagX`, `CT60` | Systemart | `sdk/apps/teradesk/main.c:1792-1795@find_cookie`, `:1837@ct60` | [R]; alle fehlen |
| TeraDesk | (Ssystem) | `have_ssystem = Ssystem(-1,0,0) == 0`, dann `S_GETCOOKIE` | `sdk/apps/teradesk/main.c:1784@have_ssystem`, `sdk/apps/teradesk/library/utility/cookie.c:39@find_cookie` | [R] |
| QED | `_IDT` | Fehlt -> Default `0x0000112E`; sonst Wert | `sdk/apps/qed/src/global.c:564@getcookie` | [R] |
| NewTOS-Desktop | `_IDT` | `dos_getcookie(COOKIE_IDT)` | `programs/desktop/src/redraw.rs:145@COOKIE_IDT` | [R] |
| Doom | `EdDI` | Vorhandensein genuegt | `sdk/apps/doom/doomgeneric_gem.c:1096@Getcookie` | [R] |
| HighWire | `EdDI`, `STiK` | `Setexc(0x5A0/4,-1)`-Lauf; Struktur `{long id; long val;}` | `sdk/apps/highwire/src/romvdi.c:46@Setexc`, `:70@has_eddi_cookie`; `sdk/apps/highwire/src/inet.c:215@jar` | [R]; Schrittweite auf LP64 siehe Abschnitt 7 |
| cflib | `MiNT`, `MgPC`, `nAES`, `NVDI`, `FSEL`, `xFSL`, `Gnva`, `MagX` | Systemart/Dialoge | `sdk/cflib/cflib-21/appinit.c:51-57@getcookie`, `sdk/cflib/cflib-21/filesel.c:77@getcookie`, `sdk/cflib/cflib-21/fontsel.c:195@getcookie`, `sdk/cflib/cflib-21/userdef.c:692@getcookie`, `sdk/cflib/cflib-21/magxver.c:37@getcookie` | [R]; keine Hardwarekennung |
| WinDOM | `MagX`, `NVDI`, `MiNT`, `FSEL`, `FSMC`, `BGEM`, `BHLP`, `ICFS`, `LTMF`, `HBFS`, `WINX` | AES/Fenster-Zusatz | `sdk/windom/windom-1.21.3/src/vq_magx.c:40@get_cookie`, `sdk/windom/windom-1.21.3/src/fsel.c:73@get_cookie` u. a. | [R]; keine Hardwarekennung |
| mintlib Start | `MiNT` | `__mint` | `sdk/libc/mintlib/mintlib/main.c:95@Getcookie` | [R] |
| mintlib `sysinfo(SI_ARCHITECTURE)` | `_CPU`, `_MCF` | `_CPU` gefunden -> `mc680x0`-Zweig; `_MCF` -> ColdFire; **keiner von beiden** -> Fortsetzung hinter `:420` | `sdk/libc/mintlib/unix/sysinfo.c:370@Getcookie`, `:385@C__MCF` | [R] Eingang; Ausgang bei fehlenden Cookies **nicht gelesen** [O] |
| zView JPEG | `_JPD`, `_CPU` | nur bei `_JPD` | `sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:801@Getcookie`, `:818@cpu` | [R] |
| Thing (nur Quelle in `Applications/`, nicht gebaut [D]) | `_AKP`, `XHDI`, `MgMc`, `SCSI` | — | `Applications/thing-neo-codeSVN-r62/Source/ThingTbx/src/system.c:55`, `Applications/thing-neo-codeSVN-r62/Source/XHDI/xhdi.c:39` | [R] |
| mintlib `_checkcpu` | `_CPU`, `_FPU` | **nur** in m68020+/FPU-Bauten (`#if __mc68020__`, `__HAVE_68881__`); der AArch64-Bau uebersetzt keine `.S`-Dateien | `sdk/libc/mintlib/mintlib/checkcpu.S:25-103`, `sdk/libc/build-libc.sh:344@getcookie` | [R]; nicht wirksam |
| `NTOS` | — | **Kein Programm der durchsuchten Quellen liest `NTOS`.** Treffer nur im Kernel und in Selbsttests | — | [R] (Suche leer ausserhalb `kernel/`, `tos-api/`, `docs/`) |

**Zugriffsweg (Rahmen, nicht Gegenstand):** NewTOS-libc `Getcookie` laeuft ueber
`Ssystem(S_GETCOOKIE)` (`sdk/libc/newtos-arch/getcookie.c:33@Getcookie`);
`Setexc(0x5A0/4,-1)` und `$5A0` laufen ueber den Spiegel (ADR-012, Nachtrag B1).

---

## 3. EmuTOS: Verhalten, das am Maschinentyp haengt

### 3.1 Direkt an `cookie_mch` [R]

| Stelle | Bedingung | Wirkung | NewTOS beobachtbar? |
|---|---|---|---|
| `bios/serport.c:966` | `MCH_FALCON \|\| IS_ARANYM` | `Bconmap(-1)` liefert 7, sonst 6 — **im Orakel gemessen**: `Bconmap(-1)` `tt` = 6, `falcontc` = 7 (Fall 183, `tosdiff` BIOS `tt` gegen `falcontc`) | ja; NewTOS-`Bconmap` ist ein eigener Fall (`tos-api/src/xbios/mod.rs:138@other`, Eintrag `BCONMAP`) |
| `bios/serport.c:974,982` | `MCH_FALCON`/`MCH_MSTE`/ARAnyM, `MCH_TT` | Belegung der `MAPTAB` (SCC-Ports) | Hardware (SCC) |
| `bios/nvram.c:171` | `MCH_TT` | Nur im Arm „alles zuruecksetzen“ (`type == 2`): TT berechnet nur die Pruefsumme, sonst Initialdaten | ja, wenn NVRAM bestuende |
| `bios/clock.c:1174` | `MCH_TT`, nur `EMUTOS_LIVES_IN_RAM` | Basisjahr der NVRAM-Uhr (1970 / 1968) | nein (`EMUTOS_LIVES_IN_RAM` = Bauart) [D] |
| `bios/bios.c:292` | `MCH_ST/STE/MSTE` und 68030 | Blitter abschalten (PAK 68/3) | nein |
| `bios/floppy.c:1639` | `MCH_FALCON` | DMA-Schreibschleife | Hardware (FDC) |
| `bios/acsi.c:127` | `MCH_STE/MSTE` | 16-Bit-ACSI-Zugriff | Hardware |
| `bios/memory2.c:162,204` | `MCH_MSTE` | MonSTer-RAM-Fenster | Hardware |
| `bios/sound.c:65` | `MCH_TT \|\| MCH_MSTE` | LAN-Port-Bit | Hardware |
| `bios/machine.c:807-821` | `MCH_*` | `machine_name()` („Atari TT“, „Atari Falcon“ …), Anzeige der Bootmeldung/About | nein; NewTOS: Baukonstante pro Board (`programs/desktop/src/inf.rs:2614@MACHINE_NAME`) |
| `util/boot.c:129` | `>= MCH_TT` | nur `TARGET_256`-Lader | nein |

### 3.2 An Hardware-Flags, **nicht** am Cookie [R]

Das ist der tragende Befund fuer Optionen und `NVMaccess`:

- `NVMaccess`: `bios/xbios.c:1349-1353` (`CONF_WITH_NVRAM`, sonst `xbios_unimpl`), Laufzeit
  `nvram.c:161-162` (`if (!has_nvram) return 0x2E;`); `has_nvram` kommt aus einer
  Buslesung (`nvram.c:49-54`), **nicht** aus `_MCH`.
- Videl (`VsetMode`, `VgetMonitor`, `VgetSize`, `VgetRGB`, `Vfixmode`): `has_videl`
  (`bios/videl.c:623`, `:666`, `:679` …), im Fehlerfall Funktionsnummer
  (`videl.c:624`: `return 0x58`). TT-Shifter: `has_tt_shifter` (`bios/screen.c:212-214`,
  `egetshift` -> `0x51`). DSP: `has_dsp` (`bios/dsp.c:189-195`). DMA-Ton: `has_dmasound`
  (`has.h:99-106`).
- `_SND`: `HAS_DMASOUND`/`HAS_FALCON_DMASOUND`/`HAS_DSP` (`bios/machine.c:409-447`);
  `_VDO`: `HAS_VIDEL`/`HAS_TT_SHIFTER`/`HAS_STE_SHIFTER` (`:363-379`).
- RTC-Auswahl: `has_nvram`, dann Mega-RTC, ICD-RTC, zuletzt IKBD-Uhr
  (`bios/clock.c:1150-1198`, `:1213-1260`) — wieder Flags, nicht `_MCH`.
- `_AKP`/`_IDT`: **aus dem NVRAM**, falls lesbar (`bios/country.c:140-186`:
  `nvmaccess(0,6,2,..)`, `nvmaccess(0,8,2,..)`), sonst Landesvorgabe. Wer NVRAM
  einfuehrt, aendert damit zwei Cookies (Kopplung NVRAM <-> Cookie-Werte).

### 3.3 Gemessen im Orakel: `tt` gegen `falcontc`, BIOS/XBIOS [R]

`tosdiff tosdump/cases/bios-r1.tsv EMUTOS-1a7484ab-BIOS.BIN EMUTOS-1a7484ab-BIOS-falcontc.BIN`
(147 Faelle) weicht in Faellen ab, die an der Maschine haengen: `Getrez` (4 gegen 2),
`Rsconf`, `Giaccess(0,14)`, `Bconmap(-1)` (6 gegen 7), `EgetShift` und die TT-Palette
(`0x51`-`0x57`: nur `tt`), `VsetMode`/`VgetMonitor`/`VgetSize`/`VgetRGB`/`Vfixmode`
(`0x58`-`0x5F`: nur `falcontc`), `Dsp_*` (`0x67`-`0x7D`: nur `falcontc`), `Soundcmd`, `Buffptr`.
**`NVMaccess` (Faelle 189-194, `bios-r1`) und `xbios-system-r1` 68-76 stehen nicht in der
Differenzliste, also gleich auf `tt` und `falcontc`**: `0x0000` beim Lesen, `0xFFFB` bei
Bereichs-/Operationsfehler (`measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`,
Fall 189 und 194; `TASK/XBIOS10.md`). Beide Orakelprofile haben also NVRAM.

---

## 4. Register-Pruefung fuer den Identitaets-Cookie `NTOS`

Quellen (gelesen): (a) mintlib-Liste `sdk/libc/mintlib/include/mint/cookie.h` — Kopfkommentar:
„generated from cookie.dat (Kelly's MiNT-Setter)“; 708 Zeilen; (b) tos.hyp, Kapitel „Cookie Jar“
(`https://freemint.github.io/tos.hyp/de/bios_cookiejar.html`, Abschnitt 3.7.2.337 `nTOS`,
3.7.2.604 `_MCH`; vom Anwender in `TASK/COOK01.md` genannt, per Abruf gelesen);
(c) FreeMiNT `references/freemint/sys/cookie.h` (nur Tags/MCH-Werte) und
`references/freemint/doc/programmer/cookiejar.txt` (Programmierer-Text, keine Registerliste).
Es gibt **keine Vergabestelle** (`TASK/COOK01.md`; tos.hyp definiert fuer FireBee,
MiSTer, Hatari oder neue Maschinen keinen `_MCH`-Wert).

| Pruefung | Ergebnis |
|---|---|
| `NTOS` = `0x4E544F53` in mintlib `cookie.h` | **nicht enthalten.** Naechster Treffer `C_nTOS 0x6E544F53` „MilanBlaster“ (`sdk/libc/mintlib/include/mint/cookie.h:339@C_nTOS`) |
| `NTOS` in tos.hyp | **nicht enthalten.** `nTOS` (MilanBlaster, Rainer Mannigel, „Nutzt den Trap#14/XBIOS und TTMFP/SCSI DMAC“) ja; Namen mit grossem `N`: N203, NAH3, NBEL, NBLL, NCLN, NERV, NETS, NEXB, NEXK, NFMT, NFnt, NKCC, NLdr, NMGE, NO7P, NOR5, NOVA, NSC1, NTK4, NUL>, NVDI |
| Kollision | **nein.** Cookie-Tags sind 4 Byte, Gross-/Kleinschreibung zaehlt (`0x4E…` gegen `0x6E…`). **Aber:** ein Vergleich ohne Beachtung der Schreibweise haelt `NTOS` und `nTOS` fuer dieselbe Sache — Nähe, keine Kollision |
| Auch nicht `_NEW`, `NTOS` im Pin | EmuTOS vergibt `NTOS` nicht (`include/cookie.h:21-39`) |

Grenzen: tos.hyp Abschnitt 2/2 gelesen ab Zeichen 100000 per Abruf; Nachschlagen
von `NTOS` im ersten Teil (Zeichen 0-100000) lieferte nur `nTOS` — [R] soweit der Abrufer
den Text wiedergibt, kein eigener Volltext-`grep` der Seite. Der Registerzustand ist der
einer Liste von 2026 aus dem Netz, kein Vergabeakt.

**Zur Bedeutung von `0x00050000`.** Quellen in der Umgebung: `references/emutos/include/cookie.h:57`
(`MCH_ARANYM`), `bios/machine.c:386` (nur gesetzt, wenn der NatFeats-Name mit „ARAnyM“ beginnt,
`:509-519`), tos.hyp 3.7.2.604 („0x0005 / 0x0000 ARAnyM >=v0.8.5beta“), zView-Kommentar
(`sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:810@Aranym`), mintlib
`sdk/libc/mintlib/unix/sysinfo.c:551@case`. **Nicht** in FreeMiNTs `sys/cookie.h` (Liste endet
bei `MILAN_C`) und nicht in mintlibs `cookie.h` (kein MCH-Wert dort). Damit ist die Lesart
„0x00050000 = ARAnyM“ **belegt**; die vorlaeufige Wahl in `cookie.rs` ist eine Kollision mit
einem vergebenen Wert, nicht eine freie Nummer.

---

## 5. Vorschlag: Cookie-Jar-Unterschiede als ACCEPTED-Abweichungen je Profil

### 5.1 Stand der Mechanik [R]

`tosdiff` kennt keine Markierung „ACCEPTED“: es gibt nur `accepted` als Ergebnis von
`--validate` (`tosdump/host/tosdiff.c:234@accepted`) und Ausgang 1 bei Differenzen
(`:14@exit`). **ACCEPTED ist eine Audit-Klassifikation, kein Tabellenfeld** (Audit-Berichte
`docs/audits/2026-10-28-*.md` zaehlen „equal / accepted / diff“; ADR-031 nennt
BIOS-CORE-002 „ACCEPTED deviation (proof: this ADR)“, `docs/MEMORY.md:3057`). Ein
ACCEPTED-Fall bleibt in `tosdiff` ein DIFF; die Annahme steht im Beleg.

### 5.2 Wo das stehen soll (Vorschlag)

1. **`docs/COMPATIBILITY.md`, Abschnitt „Sanctioned deviations, one by one“**: ein Eintrag
   „Cookie jar vs. the oracle“ mit Datum und Anwenderentscheidung, **eine Tabelle je Profil**
   (`tt`, `falcontc`, `disk`) mit den Spalten Cookie | Orakel | NewTOS | Status
   (`EQUAL` / `ACCEPTED` / `OPEN`) | Begruendung. Muster: `_MCH`, `_VDO`, `_SND`, `_FPU`, `_CPU`,
   `_FDC`, `_SWI`, `_FRB`, `XHDI`, `SCSI`, `_AKP`, `_IDT`, `NTOS`, `EdDI`.
2. **`docs/api/bios/_cookie-jar.md`, Abschnitt „The Pi cookie set“**: die dortige Tabelle
   (Zeilen 130-150) verweist auf den Eintrag und fuehrt den Wert nach der Entscheidung
   nach (heute `0x00050000`, dort „NewTOS-specific“ — nach Abschnitt 4 sachlich falsch).
3. **Fallkopf der Cookie-Tabelle** (Abschnitt 5.3): ein Satz „Differenzen -> Eintrag in
   `docs/COMPATIBILITY.md`“; die Kommentarspalte jedes Falls nennt den Eintrag, wie es
   `tosdump/cases/bios-r2.tsv` mit dem `NEWDESK.INF`-Hinweis tut.
4. **Audit-Bericht** fuehrt die Faelle als `ACCEPTED` mit Verweis auf 1.

### 5.3 Falltabelle (Vorschlag, nichts ist gemessen)

Es fehlt ein Zeilentyp, der den Cookie-Jar liest — `tosdump/core/cases.c` kennt nur
`trap:function`-Aufrufe (`tosdump/cases/bios-r1.tsv` Kopf). Bausteine sind da:
`td_jar_read` kopiert das Jar in beiden Zielen
(`tosdump/core/scrninfo.h:39@td_jar_read`, m68k-Zweig `tosdump/abi/m68k_scrninfo.c:60@td_jar_read`,
NewTOS-Zweig `tosdump/abi/newtos_scrninfo.c:43@td_jar_read`). **Ein Entwickler-Auftrag**, kein
Researcher-Code:

- Neue Tabelle „jar-r1“ im Format des Kopfes von `tosdump/cases/bios-r1.tsv` mit einem
  Pseudoaufruf `jar:<TAG>` je Zeile (Rueckgabeklasse `EXACT`, `NONZERO`, `ABSENT`).
- Zeilen: `_MCH EXACT`, `_VDO EXACT`, `_SND EXACT`, `_CPU EXACT`, `_FPU EXACT`, `_FDC EXACT`,
  `_SWI EXACT`, `_AKP EXACT`, `_IDT EXACT`, `_FLK ABSENT`, `NTOS ABSENT`, `EdDI ABSENT`,
  `_FRB NONZERO`, `XHDI NONZERO`, `SCSI NONZERO`, `_5MS ABSENT`, `__NF`, plus
  **Zahl der Eintraege** und **Reihenfolge der Tags** (Reihenfolge ist beobachtbar:
  `fill_cookie_jar`, `bios/machine.c:665-796`).
- Referenzen: `tt`, `falcontc`, `disk` auf dem Pin (`tools/hatari-oracle.sh`).

---

## 6. ADR-Optionen fuer `_MCH`

Die Optionen betreffen **nur** den Wert und seine Begleitentscheidungen. Entscheidend ist
Abschnitt 3.2: Die maschinenabhaengigen **XBIOS-Funktionen** schaltet EmuTOS ueber
Hardware-Flags, nicht ueber `_MCH`. Ein NewTOS-`_MCH` entscheidet daher nicht
von selbst ueber `NVMaccess`; er entscheidet, **was ein Programm erwartet**.

### Option A — eigener Wert (Ausgangspunkt ADR-012)

- **A1 Status quo `0x00050000`.** [R] Kollision: ARAnyM (Abschnitt 4). Folgen: mintlib
  meldet „aranym“; zView-Zweig (tot ohne `_JPD`) wuerde greifen; EmuTOS-eigene Pruefungen
  auf `IS_ARANYM` entfielen nur, weil NewTOS nicht EmuTOS ist.
- **A2 `0xFFFFFFFF` (`MCH_NOHARD`).** [R] EmuTOS meldet genau das in einem Bau ohne
  Atari-Hardware (`bios/machine.c:400-401`, `include/cookie.h:50`); tos.hyp: „nicht-ST-kompatible
  Hardware … alle Hardware-Zugriffe haben zu unterbleiben“. Folgen: kein Programm im Baum
  verzweigt darauf (Abschnitt 2.1) — **ausser** mintlib `sysinfo` -> Null-Zeiger-Zugriff
  (`sdk/libc/mintlib/unix/sysinfo.c:520-525`, `:562`) [D]; ein SDK-Eingriff in libc waere noetig (Entwickler,
  allgemeine LP64/Familien-Korrektur, ADR-019-Regel „Upstream unveraendert, Patches
  benannt“ pruefen). `NVMaccess`: kein Zwang. Dass NewTOS dann `0x2E` liefert, ist
  EmuTOS' eigene Antwort ohne NVRAM-Hardware (`nvram.c:161-162`, bzw. `xbios_unimpl`
  mit Funktionsnummer `0x2E`, `bios/xbios.c:1352`) — dieselbe Zahl wie heute.
- **A3 eigene Familie ausserhalb 0-5, z. B. `0x0006....`.** Kein Treffer in Registern
  (Abschnitt 4: Liste endet bei 5 in tos.hyp). Folgen: mintlib `default` liest ausserhalb
  des Feldes (`sdk/libc/mintlib/unix/sysinfo.c:555-558` mit Index 9, Feld hat 9 Eintraege) [D]; sonst wie A2.
  Es gibt keine Vergabestelle, die den Wert schuetzte.

### Option B — Falcon-Kennung `0x00030000`

- Folgen [D]: mintlib „falcon“/„atari“. **Kein** Programm im Baum liest `_MCH` auf diesen
  Wert (Abschnitt 2.1). TeraDesk haengt an `_VDO`, nicht an `_MCH`; solange `_VDO` fehlt
  bleibt es im ST-Zweig (`sdk/apps/teradesk/video.c:160-202`). Setzte man zusaetzlich `_VDO = 0x00030000`,
  riefe TeraDesk `VsetMode(-1)` und `VgetMonitor()` (`sdk/apps/teradesk/video.c:175-176`), die NewTOS **heute
  mit der Funktionsnummer beantwortet** (`tos-api/src/xbios/mod.rs:138@other`, `0x58`/`0x59`).
- `NVMaccess`/RTC/Video/DSP/Ton: Das Falcon-Orakelprofil hat alle (Abschnitt 3.3). Die
  Kennung allein schaltet nichts; sie macht aber jede fehlende Funktion zu einer
  **Luege** gegenueber dem Programm („Falcon“, aber `VsetMode` unbekannt). Gleiches gilt
  fuer die Kopplung `_AKP`/`_IDT` aus NVRAM (Abschnitt 3.2).
- Widerspricht dem Wortlaut von ADR-012 („eigene Kennung“) und dem Nicht-Ziel „kein
  Hardware-Nachbau“ (`CLAUDE.md`), soweit mehr als der Zahlenwert gemeint ist.

### Option C — Falcon-Kennung mit Abweichungsliste

Wie B, aber der Falcon-Bestand wird ausdruecklich als Abweichungsliste gefuehrt
(`docs/COMPATIBILITY.md`, 5.2): Videl-, DSP-, DMA-Ton-, SCC-, FDC-, NVRAM-Funktionen
ACCEPTED, `_VDO`/`_SND`/`_CPU`/`_FPU` je mit Wert oder Abwesenheit festgelegt.
- Folgen [D]: der Cookie-Satz ist dann in sich widerspruechlich (Falcon ohne `_VDO`/`_SND`);
  jeder Beleg muss eine Falcon-Annahme einzeln widerlegen. Vorteil: `tt`/`falcontc`-Faelle
  bleiben vergleichbar, weil das Referenzprofil der Spec das Falcon ist
  (`docs/api/_emutos-config.md`). Mehraufwand: Liste pflegen, `XBIOS10` muss
  NVRAM entweder liefern oder als ACCEPTED fuehren (`XBIOS10`-Abnahme verlangt „EQUAL tt und falcontc“
  oder ACCEPTED).

### Folgen je Option fuer die maschinenabhaengigen Funktionen [R/D]

| Funktion | A1 / A2 / A3 | B | C |
|---|---|---|---|
| `NVMaccess` (0x2E) | beliebig; `0x2E` = EmuTOS-Antwort ohne NVRAM; `XBIOS10` = „kein NVRAM“ mit Eintrag in COMPATIBILITY (Fall 189-194 ACCEPTED) | Programm erwartet NVRAM (Falcon hat es, Orakel `falcontc` auch) -> Druck Richtung Emulation | wie B, aber als Abweichung benennbar |
| RTC (`Gettime`/`Settime`) | unberuehrt: wird ueber die GEMDOS-Uhr beantwortet (`tos-api/src/xbios/mod.rs:116@GETTIME`); EmuTOS' RTC-Wahl haengt an Flags (`clock.c:1150-1198`) | wie links | wie links |
| `_AKP`/`_IDT` | bleiben fest (US/`/`) oder folgen der Tastatur (`COMPATIBILITY.md`, offen) | wie links; EmuTOS-Falcon liest sie aus NVRAM | wie B |
| Video (`0x58`-`0x5F`), `EgetShift` (`0x51`) | Funktionsnummer, ehrlich „nicht vorhanden“ | Funktionsnummer, aber „Falcon“-Kennung -> Widerspruch | ACCEPTED-Liste |
| DSP (`0x67`-`0x7D`), `Soundcmd` | wie oben | wie oben | ACCEPTED-Liste |
| `Bconmap(-1)` | `0x2C` (`xbios/mod.rs` BCONMAP-Arm) — weicht von beiden Profilen ab (6/7) | wie A | wie A |
| Desktop-Maschinenname | Baukonstante (`programs/desktop/src/inf.rs:2597@_MCH`), weil `_MCH` beide Boards gleich meldet | wie A | wie A |

### Empfehlung

**A2 (`_MCH = 0xFFFFFFFF`) mit `NTOS` als einziger NewTOS-Identitaet** — Begruendung, getrennt
nach Beleg und Urteil:

- [R] EmuTOS meldet diesen Wert selbst fuer „keine ST-kompatible Hardware“ (`machine.c:400-401`);
  das ist die Lage von NewTOS (kein Shifter, MFP, Videl, FDC). Damit ist es ein
  **belegter, kein erfundener** Wert; kein vergebener Rechner wird imitiert (A1, B, C).
- [R] Kein Programm im Baum verzweigt auf `_MCH` ausser zView (`== 0x50000`, tot) und
  mintlib `sysinfo` (Namenstabelle) — der Wert selbst ist also fuer die Portliste frei.
- [R]/[D] Der einzige reale Preis von A2 ist die Null-Zeiger-Falle in `sysinfo(SI_PLATFORM)`
  (`uname`): zu schliessen durch eine libc-Korrektur (Entwickler) oder Aufnahme in
  den Bestand; gefunden wurde **kein** aufrufendes Programm [O].
- [D] Die XBIOS10-Frage wird davon **entkoppelt**: weil EmuTOS Funktionen ueber
  Hardware-Flags schaltet, ist „kein NVRAM“ unter A2 konsistent (NewTOS gibt, was
  EmuTOS ohne NVRAM-Hardware gibt). Soll NewTOS NVRAM liefern (z. B. fuer `_AKP`/`_IDT`),
  ist das eine **eigene** Entscheidung in `XBIOS10`, keine Folge der Kennung.
- Urteil, kein Beleg: Fallback bei Widerstand gegen eine libc-Aenderung ist A3 mit
  einer zusaetzlich zu korrigierenden `platforms[]`-Tabelle — gleiche Aenderung, schlechterer
  Wert. B/C empfehle ich nicht: sie nuetzen den Programmen im Baum nichts (keiner
  liest den Wert) und erzeugen die „Luege gegenueber dem Programm“.

**Braucht ADR-012 einen ausdruecklichen Nachtrag? Ja, in jeder Option ausser A1.**
Der ADR nennt keinen Wert (`docs/MEMORY.md:386-390`), und der Nachtrag vom 2026-10-26
fuehrt nur den Satz der Tags auf. Fuer A2/A3/B/C aendert sich ein **programmsichtbarer**
Wert; das gehoert als Nachtrag (Datum, Wert, Begruendung, Quelle Abschnitt 4) in
ADR-012 — „nie still“ (`TASK/ARCH05.md`, Punkt 3). Selbst A1 braucht einen Eintrag, weil
sonst die Kollision (Abschnitt 4) als offene Abweichung stehen bliebe.

**Kopien des Wertes, die mitzuziehen sind** (Begruendung steht an mehreren Stellen):
`tos-api/src/cookie.rs:223@MCH`, `kernel/src/selftest_cookie.rs:65@MCH`,
`kernel/src/initinfo.rs:127@_MCH`, `programs/desktop/src/inf.rs:2597@_MCH`,
`docs/api/bios/_cookie-jar.md:136`/`:148`, `docs/api/desktop/deskmain.md:1340`,
`docs/HARDWARE-PROFILE.md:411`, `TASK/COOK01.md`, `TASK/X03.md`. Das Spiegel-Jar fasst
15 Cookies (`loader.rs:951@COOKIE_JAR_SLOTS`) — jede Option bleibt darunter.

---

## 7. Offene Anwenderentscheidungen und Randbefunde

**Offen (Anwender):**
1. Option A1 / A2 / A3 / B / C (Abschnitt 6) — und damit der Nachtrag zu ADR-012.
2. Bei A2: darf die SDK-libc `sysinfo` korrigiert werden (Null-Pruefung bei `hi == -1`; ggf. Komma in `platforms[]`)?
3. `_CPU`/`_FPU`/`_VDO`/`_SND`: bleiben sie weggelassen? (ADR-012: `_VDO/_SND/_FDC/_FPU` entfallen; `_CPU` laut `HARDWARE-PROFILE.md` §3.7 weggelassen, aber nicht im ADR.) Beleg: TeraDesk liest `_VDO` und landet bei Abwesenheit im ST-Zweig (`sdk/apps/teradesk/video.c:160-202`) — Wirkung ungemessen.
4. Cookie-Falltabelle „jar-r1“ (Abschnitt 5.3) beauftragen und auf dem Pin messen lassen (Hatari-Lauf, kein Researcher-Lauf).

**Nicht gelesen / [O]:** Verhalten von `sysinfo(SI_ARCHITECTURE)` ohne `_CPU`/`_MCF`
(`sysinfo.c` hinter Zeile 420); ob ein Programm `uname`/`SI_PLATFORM` ruft; die Wirkung des
ST-Zweigs in TeraDesk auf NewTOS; die Jar-Werte der `disk`-Profile und alle Werte auf dem
Pin (Abschnitt 1.1).

**Randbefunde ausserhalb des Auftrags (je eine Zeile):**
- HighWire (`sdk/apps/highwire/src/romvdi.c:36-52@getcookie`) und TeraDesks `COOKIE` laufen
  das Jar mit `long`-Paaren; bei LP64 ist die Schrittweite 16 statt 8 Byte
  (`sdk/apps/apitest/apitest.c:1840@SX16` belegt den 8-Byte-Takt; „SX17“ den Fehlgriff) —
  ob HighWires `EdDI`-Suche damit scheitert, ist ungeprueft.
- `docs/api/bios/_cookie-jar.md:136` nennt `0x00050000` „NewTOS-specific“; nach Abschnitt 4
  ist das belegt falsch.
- Die Kommentare `tos-api/src/xbios/mod.rs` (Kopf, „Supexec 0x26 nicht verdrahtet“) sind
  moeglicherweise veraltet gegenueber ADR-031; nicht geprueft.

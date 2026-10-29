# LP64: `long`-Zugriffe auf TOS-Strukturen in allen Ports — Erhebung (SDK04 Arbeit 1, 2026-10-28)

Stand `a9f4d564`. Auftrag: `TASK/SDK04.md` Punkt 1 (nur die Erhebung; Messung ist Punkt 2,
SDK-Typen Punkt 3). Dies ist ein **datierter Schnappschuss**, keine Spezifikation.
Es wurde nichts gebaut und nichts gemessen; wo eine Wirkung steht, ist sie als
**[R]** (aus Quelltext/Messprotokoll gelesen) oder **[D]** (abgeleitet) gekennzeichnet.

## 0. Frage, Verfahren, Umfang

**Frage.** Wo greift ein portiertes Programm auf eine TOS-Struktur mit einem Typ zu,
dessen Breite auf m68k (4-Byte-`long`/Zeiger) und auf NewTOS-LP64 (8 Byte) verschieden
ist, so dass Offsets oder Breiten falsch sind?

**Durchsucht** (`/usr/bin/grep`, jede Fundstelle gelesen): `sdk/apps/`, `sdk/overlays/`,
`sdk/examples/`, `programs/`, `tosdump/`, `sdk/libc` (`sdk/libc/mintlib`, `sdk/libc/overlay`,
`sdk/libc/newtos-arch`, `sdk/libc/tests`), `sdk/gemlib` (+`overlay`), `sdk/cflib` (+`overlay`),
`sdk/windom` (+`overlay`), `sdk/zview` (+`overlay`, `z-tools`), `sdk/ldg` (+`overlay`),
`sdk/libshare`, `sdk/include`, `sdk/templates`, `Applications/` (Thing, Vanilla-Conquer,
nur Stichwortsuche). Muster: `_p_cookies`, `0x5A0`, `get_cookie`/`find_cookie`/
`Getcookie`, `Setexc(0x5A0/4`, `_sysbase`, `0x4F2`, `OSHEADER`, Seite-0-Adressen hinter
`long`/Zeigerkonvertierung (`0x4BA`, `0x472..0x47E`, `0x420..0x43A`, `0x484`, `0x446`),
`sysvars.h`-Makros, `BASEPAGE`/`_base`/`p_*`, `DTA`/`Fgetdta`/`Fsetdta`, `Iorec`,
`Kbdvbase`, `Keytbl`, `Getmpb`/`MD`/`MPB`, `Getbpb`/`BPB`, `Dfree`, `Bconmap`, `XATTR`.

**Nicht durchsucht:** AES-/VDI-Strukturen (`OBJECT`, `PARMBLK`, `MFDB`, `AESGLOBAL` — eigene
Reihe, `docs/sdk/05-abi-pitfalls.md` B9/B10/B16); das Innere der Kern-Rust-Quellen
(`kernel/`, `tos-api/`) ausser dort, wo ein Befund die Kernseite belegen muss;
`references/` (ausser den EmuTOS-Belegen unten).

**Vorwissen, nicht neu erhoben:** `docs/sdk/05-abi-pitfalls.md` B1/B2 und
`measurements/measurement-2026-09-17-diskinfo/08-struct-reach-survey.md` (Stand 2026-09-17)
haben die mintlib-Typedefs schon einmal gesichtet. Neu hier: die Zugriffe der Ports
selbst, alle Seite-0-/Cookie-Leser, und was seitdem dazugekommen ist (DOOM,
HypView, zView, WinDom, LDG, `Kbdvbase`/`Iorec`-Vertrag).

## 1. Drahtbreiten (Soll, aus `docs/api/`, gegen EmuTOS geprueft)

Jedes Feld unten ist auf dem Draht 4 Byte (Zeiger, `LONG`) bzw. 2 Byte (`WORD`).
EmuTOS-Quelle und NewTOS-Schreiber sind mitzitiert.

| Struktur | Draht | Beleg (Spec) | Beleg (EmuTOS) | Beleg (NewTOS-Schreiber) |
|---|---|---|---|---|
| Cookie-Slot | `{ULONG tag; ULONG value}` = **8 Byte**; Ende `tag==0`, `value`=Kapazitaet | `docs/api/bios/_cookie-jar.md:16-20` | `references/emutos/include/cookie.h:16-19` | `tos-api/src/bdos/process/loader.rs:941@COOKIE_JAR_VA`, `:951@COOKIE_JAR_SLOTS` (16 Slots ab `0x5F0`, Zeiger in `0x5A0` via `:1243@COOKIE_JAR_VA`) |
| `_p_cookies` (`0x5A0`), `_sysbase` (`0x4F2`), alle Seite-0-Zellen | `LONG` = 4 Byte, Nachbarzelle 4 Byte dahinter (`ramtop` `0x5A4`, `_shell_p` `0x4F6`) | `docs/api/structures/osheader.md:1-20`; `docs/api/bios/_cookie-jar.md:268-280` | — | `tos-api/src/bios/vectors.rs:131@read_unaligned` (4-Byte-Zelle, `as i32 as i64`) |
| `OSHEADER` | 48 Byte: `os_entry`/`os_version` je WORD @0/@2, danach elf 4-Byte-Zeiger/LONGs, `os_date` @0x18, `os_run` @0x28, `os_dummy` @0x2C | `docs/api/structures/osheader.md:36-56` | `references/emutos/include/biosdefs.h:44-63` | `tos-api/src/bdos/process/loader.rs:938@OSHEADER_VA` (`0x5C0`) |
| `BASEPAGE` | 256 Byte, Zeiger/`LONG` 4 Byte, `p_env` @0x2C, `p_cmdlin` @0x80 | `docs/api/structures/basepage.md:24-47` | `references/emutos/include/bdosdefs.h:66-69` (`struct _pd`) | `tos-api/src/bdos/process/loader.rs` (`populate_basepage`, im Overlay-Kopf von `basepage.h` genannt) |
| `DTA` | 44 Byte: `d_attrib` @0x15, `d_time` @0x16, `d_date` @0x18, `d_length` **LONG @0x1A**, `d_fname` @0x1E | `docs/api/structures/dta.md:23-29` | — | `tos-api/src/bdos/fs/find.rs` (`makbuf`, im Overlay-Kopf von `ostruct.h` genannt) |
| `_DISKINFO` | 4 `LONG` = 16 Byte | `docs/api/gemdos/directory/36-dfree.md` | — | `tos-api/src/bdos/fs/drive.rs` (`dfree`) |
| `IOREC` | m68k: `buf` Zeiger 4, dann fuenf `WORD` = 14 Byte (Spec nennt 16) | `docs/api/structures/iorec.md:18-24` | `references/emutos/bios/iorec.h:20-27` | **Vertrag entschieden 2026-10-28: Kern liefert das LP64-Layout des SDK-Headers (24 Byte)**, `docs/api/xbios/0e-iorec.md:140-150`; `Iorec` selbst ist noch nicht verdrahtet (ebd. `:16-22`) |
| `KBDVECS` | m68k: neun Zeiger + `kbstate` | `docs/api/xbios/22-kbdvbase.md` Teil B1/D | `references/emutos/include/biosdefs.h:105-115` | LP64-Tabelle mit 8-Byte-Slots **per Entscheidung** (`tos-api/src/bdos/process/loader.rs:1030@KBDVECS_AREA_VA`) |
| `KEYTAB` | sieben Zeiger | `docs/api/xbios/10-keytbl.md` | — | `tos-api/src/xbios/keytbl.rs:122@put_ptr` schreibt jedes Feld **in der Breite des Aufrufers** |
| `MD` / `MPB` | `MD` 16 Byte (`m_link`,`m_start` Zeiger; `m_length` LONG; `m_own` Zeiger), `MPB` 12 Byte | `docs/api/structures/md.md:28-45` | `references/emutos/include/bdosdefs.h:108-125` | Kern-`MD` = 24 Byte (weder 16 noch LP64-32), ADR-023; s. `docs/sdk/05-abi-pitfalls.md` B1 |
| `BPB` | neun `UWORD` = 18 Byte, **keine** Zeiger/`LONG` | `docs/api/structures/bpb.md:18-26` | `references/emutos/include/biosdefs.h:79` | `Getbpb` liefert einen 64-Bit-Zeiger im Ergebnisregister (`docs/sdk/05-abi-pitfalls.md` A1) |

Wichtig fuer die Wirkungsableitungen: die Kern-Zeiger-/Cookie-Tabellen liegen **unter
4 GiB und unter 2 GiB** (Jar `0x5F0`, OSHEADER `0x5C0`), die 4-Byte-Werte zero-/sign-
extendieren dort gleich. Eine Wirkung durch Vorzeichenerweiterung ist deshalb nicht
abgeleitet, wo sie nicht ausdruecklich steht.

## 2. Befunde nach Klasse

Legende Spalte „Erreichbar“: **ja** = in einem Bau-Rezept des SDK uebersetzt **und** auf
einem Bau-Pfad aufgerufen; **nein (nicht gebaut)** = Quelle liegt im Baum, kein Rezept;
**tot** = im gebauten Objekt per `#if 0`/Guard ausgeschlossen.
Spalte „Wirkung“: **[R]** gelesen, **[D]** abgeleitet (mit dem Schluss in Klammern).

### 2.1 Cookie-Jar mit `long`-Paaren (7 Funde, 1 erreichbar)

Gemeinsamer Mechanismus, **gemessen [R]** fuer genau diese Schleifenform:
`struct { long a; long b; }` ist auf LP64 16 Byte, der Slot 8 Byte; `p++` ueberspringt
jeden zweiten Slot, und das Wort `p->a` fasst acht Byte = `tag | value << 32`. Der
Abschluss `{0, Kapazitaet}` liest sich als `Kapazitaet << 32 != 0`
(`sdk/apps/apitest/apitest.c:1788@SX16`, Messung
`sdk/apps/apitest/apitest.c:1832@SX17`: `sizeof(ported COOKIE)=16`, Scan nimmt 3
Schritte und endet bei `0x620`, hinter dem Abschluss bei `0x610`,
`docs/sdk/05-abi-pitfalls.md` B2). Das Seed-Jar hat in dieser Reihenfolge
`NTOS`, `_MCH`, `_IDT`, `_AKP`, `EdDI`, Abschluss (`tos-api/src/cookie.rs:221@seed_defaults`,
Anhaengen in `tos-api/src/cookie.rs:104@set`). **[D]** folgt: ein langer-Paar-Leser trifft
nur Slot 0, 2, 4 …; ein Tag wird nur gefunden, wenn er in einem geraden Slot steht
**und** sein Wert 0 ist. `EdDI` steht in Slot 4 mit Wert 1, seine Lesart ist also
`0x0000000145644449`, nicht `0x45644449`.

| ID | Programm | Stelle | Struktur / Feld | Typ | Draht | Wirkung unter LP64 | Erreichbar? | Messvorschlag |
|---|---|---|---|---|---|---|---|---|
| C1 | HighWire | `sdk/apps/highwire/src/romvdi.c:40@id`, Schleife `:48@cookiejar` | Cookie-Slot `id`/`val` | `long`, `long` (16 Byte) | 8 | `getcookie(0x45644449ul)` (`:70@has_eddi_cookie`) **findet `EdDI` nicht** [D: Slot 4, Wert 1]. Folge: die Bedingung `has_fsm_gdos \|\| has_eddi_cookie` (`:329@has_eddi_cookie`) bleibt falsch, HighWire fragt `vq_scrninfo` nicht und rechnet die Pixelfolge selbst (`:336-340` fabriziert `out[]`) [R: Quelltext], ob das sichtbar abweicht: offen | **nein (nicht gebaut)**: `sdk/apps/build-*.sh` kennt kein HighWire, nur `sdk/apps/highwire/Makefile` (m68k); auf keinem Abbild | M1 (Sonde mit der Funktion woertlich), HighWire-Lauf erst nach NET-Reihe |
| C2 | HighWire | `sdk/apps/highwire/src/AEI.c:72@Setexc`, `:79@ptr`, `:90@ptr` | Slot-Tag, `ptr += 2` auf `long*` | `long` | 8 | `_systype()` laeuft Slot 0,2,4 mit 8-Byte-Vergleich; kein Tag trifft; Ergebnis bleibt `SYS_TOS` — **fuer NewTOS zufaellig richtig** (kein `MiNT`/`MagX`-Cookie im Jar) [D] | nein (nicht gebaut) | in M1 mitlaufen lassen |
| C3 | HighWire | `sdk/apps/highwire/src/inet.c:213@cktag`, `:215@Setexc`, `:230@jar` | `struct {long cktag; long ckvalue;}` | `long` | 8 | `init_stik()` findet `STiK` nicht -> `tpl` bleibt `NULL`, kein Netz; `STiK` fehlt auf NewTOS ohnehin [D] | nein (nicht gebaut; nur Zweig `USE_STIK`, `sdk/apps/highwire/src/inet.c:13@USE_STIK`) | — |
| C4 | HighWire (iConnect-Modul) | `sdk/apps/highwire/modules/network.src/iconnect/socklib.c:9@COOKIE`, `:166@jar_addres`, `:183@cookie` | `COOKIE{long id; …}`, **8-Byte-Lesen von `0x5A0`** (`*(long*)0x5a0l`) | `long` | 4 / 8 | Lesen von `0x5A0` nimmt `ramtop` (`0x5A4`) als obere Haelfte mit -> Zeiger `0x3BE0_0000_0000_05F0`-Form statt `0x5F0` (gemessen in `docs/sdk/05-abi-pitfalls.md` B2 fuer dieselbe Zelle) [R]; danach Zugriff auf unzugaengliche Adresse | nein (nicht gebaut; Modul) | — |
| C5 | HighWire (Werkzeug) | `sdk/apps/highwire/tools/num_char.c:171@Setexc` | Slot `COOKJAR` | `long` | 8 | wie C1; Werkzeug zur Tabellenerzeugung auf dem Atari, nicht Teil des Browsers [D] | nein (nicht gebaut) | — |
| **C6** | **TeraDesk** | `sdk/apps/teradesk/library/utility/cookie.c:56@Setexc`, Schleife `:60@cookie`, Wert `:65@cvalue`; Typ `sdk/apps/teradesk/library/xdialog/library.h:205@COOKIE` (`long name; long value;`) | Cookie-Slot | `long`, `long` | 8 | `find_cookie()` **findet nichts, was in einem ungeraden Slot oder mit Wert != 0 steht** [D]; gemessene Schleifenform: apitest `SX17` nennt sie „TeraDesk's find_cookie loop“. Heute ohne sichtbare Wirkung: alle abgefragten Tags fehlen im Seed-Jar ohnehin (s. unten) [D]. **Latent**: ein spaeter eingefuehrtes `_VDO` (ARCH05, offener Punkt 3 in `docs/analysis/machine-identity-adr-proposal-2026-10-28.md` Abschnitt 7) bliebe unsichtbar, TeraDesk bliebe im ST-Zweig (`sdk/apps/teradesk/video.c:158@find_cookie`, `:160@vdohi`) | **ja**: `-D_TOS_ONLY` (`sdk/apps/build-teradesk.sh:102@D_TOS_ONLY`) setzt `_MINT_=0` (`sdk/apps/teradesk/library/xdialog/library.h:109@_MINT_`), damit entfaellt der `Ssystem`-Zweig in `find_cookie` (`#if _MINT_`) und **nur** der `Setexc`-Zweig bleibt; Aufrufer im Bau: `sdk/apps/teradesk/video.c:158@find_cookie` (ueber `get_set_video(0)`, `sdk/overlays/teradesk/main.c:864@get_set_video`), `sdk/overlays/teradesk/main.c:1861@find_cookie` (`C_CT60`), `sdk/apps/teradesk/xfilesys.c:1865@find_cookie` (`_FLK`). Die `MiNT`/`Gnva`/`nAES`/`MagX`-Aufrufe (`sdk/apps/teradesk/main.c:1792@find_cookie`) liegen in `#if _MINT_` (`:1782@_MINT_`) und sind nicht uebersetzt | M1 (TeraDesks `cookie.c` unveraendert in die Sonde), M2 (TeraDesk mit eingeschleustem `_VDO`) |
| C7 | TeraDesk | `sdk/apps/teradesk/library/utility/cookie.c:28@p_cookie` und `install_cookie` | `p_cookie = *(COOKIE**)0x5A0` | Zeiger | 4 | 8-Byte-Zugriff auf `0x5A0` plus C6-Stride [D] | **tot** (`#if 0` im selben File, „currently not used in Teradesk“) | — |

**Warum C6 heute ohne sichtbare Wirkung bleibt [D]:** `C_CT60`, `C__FLK`, `C__VDO` fehlen im
Seed-Jar (`tos-api/src/cookie.rs:221@seed_defaults` setzt nur `NTOS`, `_MCH`, `_IDT`,
`_AKP`, `EdDI`); `C_OVER`/`C_Lace` stehen hinter `_OVSCAN 0`
(`sdk/apps/teradesk/library/xdialog/library.h:135@_OVSCAN`). Dass `find_cookie` bei
Abwesenheit `FALSE` liefert, tut er richtig; die Schleife versagt nur beim **Vorhandensein**.

**Cookie-Wertbreite (Querschnitt) [D]:** `Getcookie()` (`sdk/libc/newtos-arch/getcookie.c:33@Getcookie`)
holt den Wert ueber `Ssystem(S_GETCOOKIE)`; der Kern schreibt vier Byte
(`tos-api/src/gemdos/ssystem.rs:366@put_u32`), `v` ist mit `0` vorbelegt, `*p_value = v`
nimmt acht Byte. Ein Wert mit gesetztem Bit 31 (kuenftig `_MCH = 0xFFFFFFFF`, ARCH05) liest
ein Programm als `0x00000000FFFFFFFF`, nicht als `-1L` wie auf m68k. Die einzige gefundene
Auswertung, `sdk/libc/mintlib/unix/sysinfo.c`, ist davon nicht betroffen: `hi = (_mch & 0xffff0000) >> 16`
in `short` (`sdk/libc/mintlib/unix/sysinfo.c:516@hi`) ergibt hier wie dort `-1`.
Kein weiterer Vergleich auf `-1`/Vorzeichen gefunden; jeder kuenftige numerische
Vergleich (`docs/analysis/machine-identity-adr-proposal-2026-10-28.md`) ist darauf zu pruefen.

### 2.2 Systemvariablen / `_sysbase` / `OSHEADER` (2 offene Funde, beide nicht wirksam)

Die Seite-0-Leser, die im Bau stehen, sind **bereits** auf feste Breiten umgestellt
oder lesen ueber `get_sysvar()` (`sdk/libc/newtos-arch/getsysvar.c:33@newtos_sysvar_t`:
fester `unsigned int`-Typ, Null-Erweiterung). Die Tabelle fuehrt nur die verbleibenden
Zugriffe mit falscher Breite; die bereits behobenen stehen in Abschnitt 3.

| ID | Programm | Stelle | Struktur / Feld | Typ | Draht | Wirkung unter LP64 | Erreichbar? | Messvorschlag |
|---|---|---|---|---|---|---|---|---|
| S1 | TeraDesk | `sdk/apps/teradesk/main.c:2150@t1` und Overlay `sdk/overlays/teradesk/main.c:2174@t1` | `_hz_200` (`0x4BA`) | `long` | 4 | `show_timer()` liest `0x4BA` mit 8 Byte (fasst `0x4BE` mit) [D] | **tot** (`#if 0`, „evaluation during development“) | — |
| S2 | mintlib (alle Programme, die `<mint/sysvars.h>` einbinden) | `sdk/libc/mintlib/include/mint/sysvars.h:83@_hz_200`, `:88@_sysbase`, `:99@_p_cookies`, `:55@phystop` u. a. | Makros wie `((volatile unsigned long *) 0x4baL)`, `((long *) 0x4F2L)`, `((long **) 0x5A0L)` | `unsigned long`/`long`/Zeiger | 4 | **Fallstrick, kein Zugriff**: wer ein Makro direkt dereferenziert, liest 8 Byte. Alle gefundenen Nutzer gehen ueber `get_sysvar()` (`sdk/libc/mintlib/time/clock.c:15@get_sysvar`, `sdk/apps/present/present.c:934@get_sysvar`, `sdk/apps/doom/doomgeneric_gem.c:675@get_sysvar`, `sdk/examples/fsbench.c:431@get_sysvar`) und sind damit richtig; direkte Dereferenzierung: **0 Treffer** [R] | ja (Header ist im Bau), aber ohne fehlerhaften Zugriff | — (Typ-Frage, s. Abschnitt 6) |
| S3 | Programme mit `OSHEADER` aus mintlib | `sdk/libc/mintlib/include/mint/sysvars.h:102@_osheader` (ab `reseth` Zeiger, `os_date` `long`) | `OSHEADER` | Zeiger/`long` | 4 | Nur `os_entry`/`os_version` (@0/@2) liegen vor dem ersten Zeiger. Gelesen wird ausschliesslich `os_version`: TeraDesk `sdk/overlays/teradesk/library/utility/gettos.c:58@os_version`, WinDom `sdk/windom/overlay/src/vq_tos.c:91@sys`. **Richtig nur zufaellig**; jedes weitere Feld (`os_date`, `os_run`, …) ueber das mintlib-Typedef ist ab Offset 4 verschoben (96 statt 48 Byte, `docs/sdk/05-abi-pitfalls.md` B1). Die Ports, die weitere Felder brauchen, nutzen rohe Offsets mit 32-Bit-Lesen (DOOM `sdk/apps/doom/doomgeneric_gem.c:1858@kbd_probe`, `sdk/examples/cmdline.c:39@sysbase`, `sdk/examples/fsbench.c:431@get_sysvar`) | ja, ohne Fehlwirkung | — |

Nebenbefund zum Vertrag (**[R]**, `tos-api/src/bios/vectors.rs:131@read_unaligned`): `Setexc(n, -1)` fuer
Zellen unter `0x800` gibt den 4-Byte-Wert **vorzeichenerweitert** zurueck
(`as i32 as i64`). Fuer `Setexc(0x5A0/4, -1)` (Jar `0x5F0`) harmlos; ein Zeiger ab
`0x8000_0000` in einer solchen Zelle kaeme als negative 64-Bit-Zahl an.

### 2.3 Basepage (1 Fund, nicht gebaut)

Alle gebauten Leser nutzen `BASEPAGE` aus dem Overlay mit festen 32-Bit-Typen und
`_Static_assert` auf Offsets/Groesse (`sdk/libc/overlay/include/mint/basepage.h:51@BASEPAGE`,
Asserts `:57-60@p_hitpa`). Lesezugriffe auf Adressfelder machen die noetige
`(T*)(uintptr_t)`-Umwandlung (`sdk/overlays/teradesk/environm.c:57@p_env`,
`sdk/ldg/overlay/src/devel/ldgload.c:96@p_env`, `sdk/libc/newtos-arch/cmdargs.c:409@p_env`,
`sdk/libc/overlay/mintlib/thread.c:68@p_parent`).

| ID | Programm | Stelle | Struktur / Feld | Typ | Draht | Wirkung | Erreichbar? | Messvorschlag |
|---|---|---|---|---|---|---|---|---|
| B1 | TeraDesk (Test-`main` in `malloc.c`) | `sdk/apps/teradesk/library/utility/malloc.c:575@p_blen` | `p_blen` (32-Bit) an `%08lx` | `int` an `long`-Format | 4 | falsche Variadic-Breite (Ausgabe), nur Diagnose [D] | **tot** (`#ifdef MAIN`, `sdk/apps/teradesk/library/utility/malloc.c:543@MAIN`) | — |

### 2.4 DTA (0 offene Funde)

Alle gefundenen Zugriffe benutzen `_DTA` aus dem Overlay
(`sdk/libc/overlay/include/mint/ostruct.h:92@_DTA`, `__INT32_TYPE__ dta_size`, `packed`,
`_Static_assert` `sizeof==44` `:102@sizeof`): TeraDesk (`sdk/apps/teradesk/xfilesys.c:550@_DTA`,
`:838@dta_size`), cflib (`sdk/cflib/cflib-21/scclear.c:37@DTA`,
`sdk/cflib/cflib-21/rpl_stat.c:150@_DTA`), mintlib `dirent`/`stat` (`opendir.c`,
`readdir.c`, `quickstat.c`, `do_stat.c` — gelesen: alle `_DTA`), QED (`sdk/apps/qed/src/global.h:27@DTA`),
apitest (`sdk/apps/apitest/apitest.c:1069@dta_size`). **Kein** Port deklariert ein eigenes
DTA-Layout. HighWire `sdk/apps/highwire/src/cache.c:926@Fgetdta` nimmt `DTA` der
mintlib-Header (nicht gebaut). `Fsfirst` wird nirgends mit `long`-Feldzugriff auf den
Puffer gelesen. Der Rust-Port liest `d_fname` als die letzten 14 Byte der 44
(`programs/newtos-gem/src/gemdos.rs:350@d_fname`).

### 2.5 Weitere Strukturen (0 offene Funde)

| Struktur | Befund | Beleg |
|---|---|---|
| `_KEYTAB` (`Keytbl`) | **richtig**: der Kern schreibt jedes Zeigerfeld in der Breite des Aufrufers. Nutzer TeraDesk, cflib (`nkcc`), HypView, DOOM lesen `->unshift` | `tos-api/src/xbios/keytbl.rs:122@put_ptr`; `sdk/apps/teradesk/library/xdialog/xdevent.c:76@Keytbl`; `sdk/apps/doom/doomgeneric_gem.c:1930@Keytbl`; `sdk/overlays/hypview/dl_init.c:245@Keytbl`; cflib hat eine eigene 3-Zeiger-Struktur `sdk/cflib/cflib-21/intern.h:78@KEYTAB`, nur Felder 0..2 |
| `_KBDVECS` (`Kbdvbase`) | LP64-Tabelle mit 8-Byte-Slots **ist die Entscheidung**; die Leser sind darauf ausgelegt: DOOM `kbdslot_t = unsigned long` mit Zeigerbreiten-Pruefung `sdk/apps/doom/doomgeneric_gem_kbdvec.h:38@kbdslot_t`, `sdk/examples/kbdvec.c:167@xbios0`, `sdk/examples/p0kid.c:146@Kbdvbase`. Wer die m68k-4-Byte-Tabelle erwartet, bricht; so ein Port ist **nicht** gefunden | `docs/api/xbios/22-kbdvbase.md` Teil B1/C1; `tos-api/src/bdos/process/loader.rs:1030@KBDVECS_AREA_VA` |
| `_IOREC`, `_MAPTAB`, `_BCONMAP`, `_PBDEF`, `META_INFO_*` | LP64-Layout ist entschieden (IOREC) bzw. unverdrahtet; **kein** Port ruft `Iorec`, `Bconmap`, `Prtblk`, `Metainit` | `sdk/libc/overlay/include/mint/ostruct.h:599@_IOREC`; `docs/api/xbios/0e-iorec.md:140-150`; 0 Aufrufer in den durchsuchten Baeumen [R] |
| `MD`/`MPB` (`Getmpb`) | Kern-`MD` = 24 Byte; **nur apitest** liest, mit eigener 24-Byte-Deklaration und Groessenpruefung | `MDX`/`MPBX` in `sdk/apps/apitest/apitest.c` (bis BIOS06) |
| `BPB` (`Getbpb`) | neun `UWORD`, breitenneutral; apitest `BPB18`, fsbench `unsigned short *` | `sdk/apps/apitest/apitest.c:482@BPB18`; `sdk/examples/fsbench.c:636@bpbp` |
| `_DISKINFO` (`Dfree`) | behoben (4x `__UINT32_TYPE__`, packed, asserts); TeraDesk benutzt es (`sdk/apps/teradesk/library/utility/malloc.c:44@_DISKINFO`, `sdk/apps/teradesk/showinfo.c:709@diskinfo`) | `sdk/libc/overlay/include/mint/ostruct.h:47@_DISKINFO`; `:53@sizeof` |
| `XATTR` (`Fxattr`) | `Fxattr` ist auf NewTOS `EINVFN`, die mintlib-Wege fallen auf `Fsfirst` zurueck | `sdk/libc/newtos-arch/do_stat.c:7@Fxattr` |
| `MAGX_COOKIE`, `xFSL`, `LDG_INFOS` (Zeiger auf Zeiger-Strukturen aus dem Cookie-Wert) | in NewTOS-Jar nicht vorhanden -> Zweig nicht erreicht; Wert kommt ueber `Getcookie` null-erweitert und wuerde als Adresse stimmen | `sdk/apps/hypview/dl_ledit.c:143@Getcookie`; `sdk/cflib/cflib-21/magxver.c:37@getcookie`; `sdk/ldg/overlay/src/devel/ldgutil.c:91@Getcookie` |

## 3. Geprueft, kein offener Befund (Ausschluss mit Beleg)

Diese Stellen waren Anfangsverdacht oder Stand der Technik; sie sind **keine** Funde,
und zwar weil der Fix oder die Auslegung im Baum steht.

| Stelle | Warum in Ordnung | Beleg |
|---|---|---|
| TeraDesk `hdv_bpb/hdv_rw/hdv_mediach` (`0x472/0x476/0x47E`) | Overlay: `uint32_t`-Zellen, Umwandlung ueber `uintptr_t` (Messung `measurements/measurement-2026-10-27-mem12-td-lp64/RUNINFO.TXT`) | `sdk/overlays/teradesk/file.c:849@hdv_bpb`, `:851@hdv_mediach`; Original unbereinigt `sdk/apps/teradesk/file.c:824@hdv_bpb` ist **nicht** im Bau (Schattenbaum, `sdk/apps/build-teradesk.sh`) |
| TeraDesk Reboot-Woerter (`0x4F2`,`0x420`,`0x426`,`0x42A`,`0x43A`) | Overlay: `uint32_t` | `sdk/overlays/teradesk/main.c:80@os_start`, `:81@memval`, `:83@resvector` |
| TeraDesk `_sysbase` in `gettos.c` | Overlay liest die 32-Bit-Zelle | `sdk/overlays/teradesk/library/utility/gettos.c:45@_sysbase` |
| WinDom `__cookieptr`, `vq_tos` | Overlay: 4-Byte-Lesen | `sdk/windom/overlay/src/cookieptr.c:48@__cookieptr`, `sdk/windom/overlay/src/vq_tos.c:82@syshead` |
| WinDom `get_cookie` (Schleife ueber `long*`, `cookiejar += 2`) | Zweig `#else` zu `#ifdef __MINTLIB__`; mintlib definiert `__MINTLIB__` (`sdk/libc/mintlib/include/features.h:546@__MINTLIB__`), also wird `Getcookie` genommen | `sdk/windom/windom-1.21.3/src/get_cookie.c:50@__MINTLIB__`, Schleife `:58@cookiejar` |
| cflib `getcookie` | `Getcookie`-Zweig; der `long*`-Schleifenzweig (`:56-108`) steht unter `#else` zu `!defined(__PUREC__)` und ist nicht uebersetzt | `sdk/cflib/cflib-21/cookie.c:49@Getcookie`, `:56@else`; Objekt `sdk/cflib/build/obj/cookie.o` gebaut |
| LDG `ldg_cookie` | Overlay ruft `Getcookie` | `sdk/ldg/overlay/src/devel/ldgutil.c:86@ldg_cookie`; Original-TSR-Kern (`cookie.s`, `cookiec.c`, `mem/*`) nicht in `sdk/ldg/build-ldg.sh` |
| DOOM `_sysbase` -> `os_beg` -> `os_version`, `os_dummy` | feste 32-Bit-Offsets, bewusst nicht mintlibs `OSHEADER` | `sdk/apps/doom/doomgeneric_gem.c:1858@kbd_probe` |
| HypView `conterm`, `_bootdev` | Ein-Byte bzw. `short`-Zelle: breitenneutral | `sdk/apps/hypview/dl_routs.c:307@ChangeSysKeyconfig`; `sdk/apps/hypview/init.c:45@getBootDrive` |
| zView `*(int32 *)0x4ba` | `int32` ist per Kopf auf 32 Bit festgenagelt | `sdk/zview/z-tools-04805b7/zview/chrono.c:25@start_time`; `sdk/zview/include/zview-ints.h:34@int32` |
| Beispielprogramme `_sysbase`/`os_run` | alle `*(volatile unsigned int *)` | z. B. `sdk/examples/cmdline.c:39@sysbase`, `sdk/examples/p0kid.c:68@sysbase` |
| `tosdump` Cookie-Leser | fester Typ `uint32_t` je Feld, Vorbild | `tosdump/core/scrninfo.h:24@td_cookie` |
| Rust-Programme (`programs/`) | typisierte Breiten; Cookie nur ueber `Ssystem` mit `u32`; **0** Seite-0-Zugriffe gefunden | `programs/newtos-gem/src/gemdos.rs:453@dos_getcookie` |

**Nicht uebersetzt und ohne NewTOS-Bezug (Applications/):** Thing
`Applications/thing-neo-codeSVN-r62/Source/Thing/external/getcooki.c` (nicht portiert,
`TASK/PORT02.md` „unbewertet“), Vanilla-Conquer
`…/tiberiandawn/gem/tests/gem00_tt.cpp:235` (m68k-Testprogramm fuer Hatari/TT, kein
NewTOS-Ziel). Beide lesen das Jar mit `long`/`Cookie*` — auf m68k richtig.

## 4. Erreichbarkeit, Kartenmanifest, „keine lokale Quelle“

Prueffrage: gibt es ein Programm auf einem Abbild, dessen Quelle diese Erhebung nicht
gesehen hat? Standardabbild `tools/make-sd-image.sh`, Manifestprobe
`measurements/measurement-2026-10-27-mem12/base.img.manifest` (Format `tools/sd-manifest.sh`,
Zeile `optional_missing none`). Ausfuehrbare Dateien auf dem Abbild und ihr Rezept:

| Auf der Karte | Rezept (lokale Quelle) |
|---|---|
| `\Apps\QED\QED.NAP` | `sdk/apps/build-qed.sh` |
| `\Apps\TERADESK\DESKTOP.NAP` | `sdk/apps/build-teradesk.sh` (+ `sdk/overlays/teradesk/`) |
| `\Apps\PRESENT\PRESENT.NAP` | `sdk/apps/present/build-present.sh` |
| `\ZVIEW\ZVIEW.NAP`, `\ZVIEW\CODECS\*.NLD` | `sdk/zview/build-zview.sh` |
| `\COREMARK\CORE.NCP`, `COREVAL.NCP` | `sdk/apps/build-coremark.sh` |
| `\LDG\EX_LIB.NLD`, `\LDGTEST.NCP` | `sdk/ldg/build-ldg-demo.sh` |
| `\LIBTEST.NCP`, `\CLOCKTST.NCP`, `\SETJMPT.NCP`, `\MXPROBE.NCP`, `\AUTO\MXPROBE.NCP`, `\ARGVTEST.NAP` | `sdk/libc/build-libc.sh` (`sdk/libc/tests/`, `sdk/examples/`) |
| `\MATHTST.NCP` | `sdk/libm/build-libm.sh` |
| `\DUMPVDI.NAP`, `\VDIDRAW.NAP` | `tosdump/` (`make -C tosdump dumpvdi-newtos`, `tosdump/build-vdidraw-newtos.sh`) |
| `\BIG`, `\CMDLINE`, `\CTRLCPAR`, `\DTACHILD`, `\FPPEXEC`, `\HELLO2`, `\PHYSBASE`, `\PTERMCH`, `\SCRFLIP`, `\SUB\CURDIR`, `\SYSINFO`, `\USBSNAP` (`.NCP`/`.NTP`), `\ACC\SYSACC.NAC`, `\SYSACC2.NAC` | `build_example` in `tools/make-sd-image.sh` aus `sdk/examples/*.c` |
| `\HELLO.NCP` | `tools/mk-hello-ncp.py` (erzeugte Bytes, keine C-Quelle; enthaelt laut Suche keine Seite-0-/Cookie-Zugriffe) |

Jedes Standard-Programm ist also einem lokalen Rezept zugeordnet; **die Luecke ist
per Konstruktion leer, mit einer Einschraenkung**: `HELLO.NCP` entsteht aus
Handassemblierung, nicht aus C; ich habe `tools/mk-hello-ncp.py` nur per Stichwortsuche
(`0x4F2`, `0x5A0`, `cookie`: 0 Treffer) und nicht Befehl fuer Befehl gelesen [offen, gering].

Nicht auf dem Standardabbild, aber mit Rezept: DOOM (`sdk/apps/build-doom.sh`,
Karte `doom.img` in `measurements/measurement-2026-10-27-mem12-acceptance/cards/`),
HypView (`sdk/apps/build-hypview.sh`), apitest (`sdk/apps/apitest/build-apitest.sh`),
gemtest/chaintest. **Ohne Rezept: HighWire** (`sdk/apps/highwire/`, nur
`Makefile` fuer m68k), daher C1–C5 „nicht gebaut“. Andere Standardkarten
(`base.img`, `doom.img`, `hv.img`, `zview.img`) tragen kein HighWire.

## 5. Messvorschlaege (Punkt 2, QEMU `raspi4b`; nichts davon ist hier gelaufen)

Hinweis: Lauf mit `serial=on` (Memory „raspi4b runs need serial=on“), Abbild als
Klon des Basisabbilds; Messordner nach `CLAUDE.md` „Measurement / evidence runs“.

- **M1 — Jar-Sonde (deckt C1, C2, C3, C6).** Ein `.NCP` im `\AUTO\`, das
  (a) `S_SETCOOKIE` fuer `Z0Z0` (Wert 0, Slot 5) und `Z1Z1` (Wert 0, Slot 6) setzt,
  (b) die Funktion `getcookie()` aus `sdk/apps/highwire/src/romvdi.c:46@Setexc`
  woertlich und `find_cookie()` aus `sdk/apps/teradesk/library/utility/cookie.c:39@find_cookie`
  **unveraendert uebersetzt** (`-D_TOS_ONLY`, Schluessel wie `sdk/apps/build-teradesk.sh:102@D_TOS_ONLY`)
  fuer `EdDI`, `NTOS`, `_IDT`, `_AKP`, `_MCH`, `Z0Z0`, `Z1Z1` aufruft, daneben `Getcookie()`.
  Vorhersage [D]: `Getcookie` findet alle sieben; beide Langpaar-Schleifen finden
  **keines** von `EdDI/NTOS/_IDT/_AKP/_MCH`, und nur `Z1Z1` (gerader Slot 6, Wert 0).
  Beantwortet die Ticketfrage „findet HighWire `EdDI`?“ ohne HighWire zu bauen; der
  App-Lauf selbst haengt am Netz (NET-Reihe).
- **M2 — TeraDesk mit `_VDO` (C6, Wirkung).** `\AUTO\`-Sonde setzt `_VDO = 0x00030000`
  (Falcon); danach TeraDesk starten und mit dem Aufrufschreiber
  (`tools/svc-seq.py`, Kern-Feature `svc-seq-trace`) pruefen, ob XBIOS
  `VsetMode`/`VgetMonitor` aufgerufen werden (Falcon-Zweig, `sdk/apps/teradesk/video.c:172@VsetMode`).
  Vorhersage [D]: nein, TeraDesk bleibt im ST-Zweig (`vdohi == 0`, `:160@vdohi`), obwohl das
  Jar `_VDO` traegt. Ohne `_VDO` im Jar (heute) gibt es keinen Unterschied — das macht M2 erst
  nach der ARCH05-Entscheidung zu `_VDO` noetig, aber billig.
- **M3 — Setexc-Vorzeichen (Nebenbefund 2.2).** Falls gewuenscht: `Setexc(0x5A0/4,-1)` ist im
  Standardjar unauffaellig; ein Test braucht eine Seite-0-Zelle mit Wert `>= 0x80000000`
  (z. B. `Setexc(0x420/4, 0x80000000)`, dann `Setexc(0x420/4, -1)`); Vorhersage [R aus Code]:
  Ergebnis `0xFFFFFFFF80000000`. Nur zur Dokumentation des Vertrags.

Alle uebrigen Eintraege (S1, S2, S3, B1, C2–C5 als Quelle) brauchen keine eigene
Messung: tot/nicht gebaut/ohne Zugriff. Begruendung steht je Zeile.

## 6. „SDK-Typen fester Breite“ (Eingang fuer Punkt 3)

Welche Struktur braucht einen SDK-Typ fester Breite, wo existiert schon einer:

| Struktur / Zugriff | Braucht SDK-Typ? | Vorhandenes | Luecke |
|---|---|---|---|
| **Cookie-Eintrag** (`{u32 tag; u32 value}`, Stride 8) | **ja, neu** | nur lokal: `tosdump/core/scrninfo.h:24@td_cookie`, apitest `RD_U32` (`sdk/apps/apitest/apitest.c:69@RD_U32`) | Kein SDK-Typ; TeraDesk (`library.h:205@COOKIE`) und HighWire deklarieren `long`-Paare. Vorschlag: `NEWTOS_COOKIE` mit `_Static_assert(sizeof==8)` und `offsetof(value)==4` gegen `docs/api/bios/_cookie-jar.md:16-20`; dazu ein Lesehelfer, der `*(uint32_t*)0x5A0` liest |
| **Seite-0-Zellen** (`_p_cookies`, `_sysbase`, `hdv_*`, `_hz_200`, `memval*`, …) | **ja, neu** | `get_sysvar()` (`sdk/libc/newtos-arch/getsysvar.c:33@newtos_sysvar_t`) liest fest 32 Bit; Overlays je Port mit `uint32_t` (`sdk/overlays/teradesk/file.c:849@hdv_bpb`, `sdk/overlays/teradesk/main.c:80@os_start`) | Die mintlib-Makros in `sdk/libc/mintlib/include/mint/sysvars.h:83@_hz_200` bleiben `long*`; es gibt **keinen** Overlay-Kopf fuer `sysvars.h` unter dem Overlay-Verzeichnis der libc (`ls sdk/libc/overlay/include/mint`: basepage, falcon, metados, mintbind, osbind, ostruct). Vorschlag: Overlay-Kopf mit `volatile uint32_t *`-Makros (und `__attribute__`-Pruefung der Adressen gegen `docs/api/`), damit neuer Code den Fehler nicht nachbaut |
| **`OSHEADER`** (48 Byte, Draht) | **ja, neu** | nur mintlibs `OSHEADER` (96 Byte, ab `reseth` verschoben) | Ein Overlay-Typ mit `__UINT32_TYPE__`-Feldern und `offsetof`-Pruefung gegen `docs/api/structures/osheader.md:36-56` und `tos-api/src/bdos/process/loader.rs:938@OSHEADER_VA`. Heutige Leser (S3) lesen nur `os_version` — richtig nur zufaellig |
| `BASEPAGE` | nein, vorhanden | `sdk/libc/overlay/include/mint/basepage.h:51@BASEPAGE` (+Asserts `:57-60@p_hitpa`) | — |
| `_DTA` | nein, vorhanden | `sdk/libc/overlay/include/mint/ostruct.h:92@_DTA` (+Asserts `:102@sizeof`) | — |
| `_DISKINFO` | nein, vorhanden | `sdk/libc/overlay/include/mint/ostruct.h:47@_DISKINFO` | — |
| `_KEYTAB` | nein | Kern passt sich der Aufruferbreite an (`tos-api/src/xbios/keytbl.rs:122@put_ptr`) | — |
| `_IOREC`, `_KBDVECS`, `_MAPTAB`, `_BCONMAP` | nein (**Vertrag ist LP64-Layout**) | `sdk/libc/overlay/include/mint/ostruct.h:599@_IOREC`, `:609@_KBDVECS` | Nur `_Static_assert` auf die entschiedene Groesse/Offsets (24 Byte `_IOREC`, `docs/api/xbios/0e-iorec.md:140-150`) ergaenzen, damit eine Aenderung des Kerns den Header mitzieht |
| `MD`/`MPB` | ja (Pruefung), niedrige Prioritaet | nur apitest-lokal `MDX` in `sdk/apps/apitest/apitest.c` (bis BIOS06) | Typ + Asserts (Kern-24-Byte) in den SDK-Kopf heben; heute kein Port |
| `BPB` | nein | breitenneutral | optional: Typ mit Offsetpruefung `docs/api/structures/bpb.md:18-26` |
| **Cookie-Wert beim Lesen** | Regel | `Getcookie` null-erweitert (`sdk/libc/newtos-arch/getcookie.c:33@Getcookie`) | Dokumentieren, ggf. `uint32_t`-Variante; nur wichtig bei Bit 31 gesetzt (2.1, Querschnitt) |

Praezedenz fuer die Form: `sdk/libc/overlay/include/mint/ostruct.h:47@_DISKINFO` und
`sdk/libc/overlay/include/mint/basepage.h:51@BASEPAGE` (feste Typen, `packed`,
`_Static_assert` gegen den Kernschreiber, Kommentar „was die Messung zeigte“).

## 7. Zaehlung

| Klasse | Funde | davon im Bau erreichbar | Anmerkung |
|---|---:|---:|---|
| Cookie-Jar mit `long`-Paaren | 7 (C1–C7) | **1 (C6, TeraDesk `find_cookie`)** | C1–C5 HighWire/Werkzeug: nicht gebaut; C7 tot |
| Systemvariablen / `_sysbase` / `OSHEADER` | 3 (S1–S3) | 0 mit Fehlwirkung | S1 tot; S2 Typfalle ohne Zugriff; S3 „richtig durch Zufall“ |
| Basepage | 1 (B1) | 0 | tot (`#ifdef MAIN`) |
| DTA | 0 | — | alles ueber `_DTA`-Overlay |
| Weitere (IOREC/KBDVECS/KEYTAB/MD/BPB/…) | 0 | — | Vertrag oder kein Nutzer |

Bereits behoben und hier **nicht** mitgezaehlt (Abschnitt 3): TeraDesk `hdv_*`, Reboot-Woerter,
`gettos`, WinDom `__cookieptr`/`vq_tos`, DISKINFO, DTA, BASEPAGE, zView-Ganzzahlen.

## 8. Offene Punkte und Nachbarbefunde

- **Offen (Messung):** C6-Wirkung ist abgeleitet, M1 ist die billigste Bestaetigung; M2 erst
  mit der `_VDO`-Entscheidung relevant.
- **Offen (gering):** `tools/mk-hello-ncp.py` nicht Befehl fuer Befehl gelesen (Abschnitt 4).
- **Berichtigung zu `docs/sdk/05-abi-pitfalls.md` B2 (Text):** „Setexc … returns exactly that,
  which is why TeraDesk's live path works“ stimmt nur fuer das **Lesen des Zeigers**; TeraDesks
  `find_cookie` bleibt wegen des 16-Byte-Strides auf der `_TOS_ONLY`-Strecke trotzdem
  funktionslos (C6). Nicht geaendert (Auftrag: nur diese Datei); fuer Punkt 3 vormerken.
- **Ausserhalb des Auftrags:** `docs/api/xbios/0e-iorec.md:140-150` (IOREC-Vertrag) und
  `docs/api/structures/iorec.md:24` („total 16 bytes“ vs. EmuTOS 14 Byte,
  `references/emutos/bios/iorec.h:20-27`) widersprechen sich; nicht untersucht.

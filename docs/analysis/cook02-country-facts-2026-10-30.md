# COOK02 country facts: `_AKP` / `_IDT` (2026-10-30)

Fact report for the user's advisor. No decisions, no recommendations. Every
statement is marked **READ** (a source location read in this session) or
**DERIVED** (reasoned from READ facts, stated as such) or **MEASURED** (a run,
`measurements/measurement-2026-10-30-cook02/RUNINFO.TXT`). Point 1 of the
brief was not handed to this task; the numbering follows the brief (2 to 6).

Two trees matter and must not be confused:

- `references/emutos` is a checkout of the pinned commit 1a7484ab, but its
  **generated** files (`include/i18nconf.h`, `bios/ctables.h`) belong to some
  other build: `references/emutos/include/i18nconf.h:7-9` has
  `CONF_MULTILANG 0` and `CONF_IDT (IDT_12H|IDT_MMDDYY|'/')`, i.e. a
  `COUNTRY=us` unique build. READ.
- `references/emutos-oracle/1a7484ab` is the oracle's own build tree
  (`COUNTRY=de`, `obj/country` = `de de`). Its generated files are the ones
  that describe the oracle ROM. READ (`references/emutos-oracle/1a7484ab/obj/country:1`).

Where this note says "the oracle build" it cites the second tree.

## 2. `_AKP` byte layout

| Fact | Source |
|---|---|
| `detect_akp` builds the cookie as `MAKE_UWORD(country, keyboard)`. READ | `references/emutos/bios/country.c:162` |
| `MAKE_UWORD(hi,lo)` = `((UWORD)(UBYTE)(hi) << 8) \| (UBYTE)(lo)`. READ | `references/emutos/include/portab.h:138` |
| So **bits 15..8 = `country`, bits 7..0 = `keyboard`**; bits 31..16 are 0 (the cookie is `ULONG`, assigned a `UWORD`). READ + DERIVED (the zero high word follows from `MAKE_UWORD` yielding a `UWORD`) | `country.c:162`, `country.c:57` |
| **Low byte = keyboard**: `get_current_country_input()` returns `LOBYTE(cookie_akp)`; `get_kbd_index()` maps that country code through `get_country_record()` to `cr->keyboard`, and `get_keytbl()` indexes `keytables[]` with it. READ | `country.c:96-99`, `country.c:101-106`, `country.c:190-196` |
| **High byte = display (language + charset/fonts)**: `get_current_country_display()` returns `HIBYTE(cookie_akp)`; `get_lang_name()` takes `cr->lang_name` (the translation name), `get_charset_index()` takes `cr->charset` (font set). These exist **only under `#if CONF_MULTILANG`**. READ | `country.c:108-130` |
| In a build with `CONF_MULTILANG 0` the high byte is not read by `country.c` at all for language/charset (`get_fonts` takes charset index 0). READ | `country.c:213-218` |
| The values stored in the two bytes are country codes (`COUNTRY_US 0`, `COUNTRY_DE 1`, `COUNTRY_FR 2`, `COUNTRY_UK 3` ...), not keyboard-table indices. A lookup of an unknown code falls back to the **default country**, not to US. READ | `references/emutos/include/ctrycodes.h:23-39`; `country.c:75-90` |
| `doc/country.txt` describes `_AKP` only indirectly ("country, language, keyboard, charset"); it names no byte layout. READ | `references/emutos/doc/country.txt:17-47` |

**Comment versus code (flagged).** The comment above `detect_akp()`'s call in
`fill_cookie_jar` says "indicates the presence of an Advanced Keyboard
Processor. The high word of this cookie is currently reserved. The low word
indicates the language currently used by TOS for keyboard interpretation and
alerts" (`references/emutos/bios/machine.c:735-738`). The code does not
contradict "high word reserved", but "low word = the language" is a single
flat value there, whereas the code splits the low word into display (high
byte) and keyboard (low byte) (`country.c:162`). The code is the
authority. READ.

**What `docs/api/` and NewTOS say.**

- `docs/api/bios/_cookie-jar.md:56` lists `_AKP` as "keyboard/language (low
  word)" and `:138` seeds `0x00000000` "keyboard/language: US". It does not
  split the two bytes. READ.
- `docs/api/gemdos/154-ssystem.md:647` calls it "(keyboard language)". READ.
- `tos-api/src/cookie.rs:30@AKP` documents `_AKP` as "keyboard / language
  (low word = language id)", and `tos-api/src/i18n.rs:38@current_lang` says
  the language could be derived "from the `_AKP` cookie (low word = language
  id)". Against `country.c:96-113` that wording names neither byte: in EmuTOS
  the language is the **high** byte of the low word, the keyboard the **low**
  byte. DERIVED (comparison of the two READ texts).
- Neither `tos-api/src/cookie.rs:223@seed_defaults` nor
  `kernel/src/keymap.rs:367@init_from_disk` connects the two: `seed_defaults`
  writes the constant 0 (`cookie.rs:227@AKP`), and `keymap.rs` contains no
  reference to the cookie jar (grep for `cookie` in `kernel/src/keymap.rs`
  returns nothing). So a loaded German keymap does not change `_AKP`.
  READ (the two sites) + DERIVED (the absence).

Other consumers of the layout (external): Thing's `ThingTbx` shifts the value
right by 8 and uses the result as the language (`value >>= 8`, 0 en, 1 de,
2 fr ...), i.e. it reads the **high byte** as EmuTOS does (point 5). READ.

## 3. The oracle's `_AKP 0x0100`

Measured by TOOL18: `_AKP 0x00000100`, `_IDT 0x0000112E` on tt, falcontc and
disk (`measurements/measurement-2026-10-29-tool18/RUNINFO.TXT`, table rows 52
and 53). Decoded: high byte 1 = `COUNTRY_DE`, low byte 0 = `COUNTRY_US` (US
keyboard). So yes, **keyboard byte 0 (US) on a `de` ROM was measured**.

### The source path that produces it

1. **The build is a unique-country build with multi-keyboard.** The oracle is
   `make 512 COUNTRY=de` (`tools/oracle/oracle.env`). Target 512 sets
   `UNIQUE = $(COUNTRY)` and passes `MULTIKEYBD='-k'`
   (`references/emutos/Makefile:560-564`); `localise` turns that into a
   `CONF_MULTILANG 0` header (`references/emutos-oracle/1a7484ab/include/i18nconf.h:7`)
   with all keyboard tables compiled in
   (`references/emutos-oracle/1a7484ab/bios/ctables.h:20-72` and
   `references/emutos/tools/localise.c:37-39`). READ.
2. **The OS header carries the country.** With `CONF_MULTILANG 0`, `os_conf`
   is `(OS_COUNTRY << 1) + OS_PAL` (`references/emutos-oracle/1a7484ab/bios/startup.S:98-103`),
   `OS_COUNTRY` is `COUNTRY_DE` (`references/emutos-oracle/1a7484ab/obj/header.h:28`),
   `OS_PAL` is 1 (`header.h:31`). `get_default_country()` therefore returns
   `os_conf >> 1` = 1 (the branch for `os_conf != OS_CONF_MULTILANG`,
   `country.c:60-73`, `ctrycodes.h:60`). READ.
3. **`detect_akp` starts from that**: `country = get_default_country()` = 1,
   `keyboard = country` = 1 (`country.c:142-143`). Without NVRAM the result
   would be `MAKE_UWORD(1,1)` = **0x0101**. DERIVED.
4. **The NVRAM override.** `CONF_WITH_NVRAM` defaults to 1 (`references/emutos/include/config.h:1159-1162`);
   the three `#define ... 0` hits at `references/emutos/include/config.h` lines 289-290, 412-413 and 770-771
   sit in per-target blocks for other machines. READ. Whether the **512
   target** falls in one of those blocks was not read block by block here;
   the measurement (below) shows NVRAM is active in the oracle run. For a
   machine whose NVRAM is readable, `nvmaccess(0, 6, 2, buf)` reads bytes 6
   and 7 (`country.c:150`); on success `keyboard = buf[1]`
   (`country.c:157`), and `country = buf[0]` **only if `CONF_MULTILANG`**
   (`country.c:154-156`). READ. In the oracle build the country byte is
   therefore not overridden and stays 1; the keyboard byte comes from NVRAM
   byte 7. `doc/country.txt:66-70` says the same for "the 512K version": the
   country code is in the OS header, "other parameters are obtained from
   NVRAM". READ.
5. **`nvmaccess` read path.** Returns `0x2E` without NVRAM (`has_nvram` 0),
   `-12` on checksum mismatch, otherwise copies bytes at `start + 14`
   (`NVRAM_START = NVRAM_RTC_SIZE = 14`) (`references/emutos/bios/nvram.c:155-161`,
   `:184-199`, `:27-28`). READ.

### What Hatari's NVRAM contains

Source read in `/Volumes/Daten/Development/Mushasi68k/hatari`
(`git describe`: `v2.6.1-235-g446d9852`, i.e. **after** 2.6.1; the oracle
binary is the 2.6.1 bundle, so the source is the same lineage but not
provably the same build; the MEASURED part below covers the binary).

- Emulated for TT and Falcon: the NVRAM/RTC chip is mapped at
  `0xff8961/0xff8963` in both Hatari src/ioMemTabTT.c lines 154-155 and
  src/ioMemTabFalcon.c lines 350-352. READ.
- Built-in contents: the static `nvram[64]` at Hatari src/falcon/nvram.c lines 101-106.
  Chip index 20 = "Language", 21 = "Keyboard layout", 22 = "Format of
  date/time", 23 = "Separator for date" (header comment, `nvram.c` lines 20-39;
  `NVRAM_LANGUAGE 20`, `NVRAM_KEYBOARDLAYOUT 21` in src/falcon/nvram.h lines 26-27).
  The initialiser puts 0 at 20 and 21, **17 (0x11) at 22 and 46 (0x2E) at 23**.
  EmuTOS user offset = chip index - 14, so EmuTOS' bytes 6/7/8/9 are these
  four. READ + DERIVED (the offset arithmetic).
- A `hatari.nvram` file in Hatari's home overrides the 50 user bytes
  (`NvRam_Load`, Hatari `nvram.c` about lines 113-139, name from `NvRam_Init`). The oracle runs
  with `HOME` = an empty directory per run, so there is no file
  (`docs/analysis/ORACLE.md:32-33`; `tools/hatari-oracle.sh` `HOME="$OUT/home"`). READ.
- Settings that write into NVRAM at start: `NvRam_Init` copies
  `ConfigureParams.Keyboard.nLanguage` to chip index 20 and `nKbdLayout` to
  21 when they are not `TOS_LANG_UNKNOWN` (-1), then recomputes the checksum
  (Hatari `nvram.c` about lines 309-314; src/includes/tos.h line 42 for the -1). The pinned
  `tools/oracle/hatari.cfg` sets `nCountryCode`, `nKbdLayout` and `nLanguage`
  all to -1 (`tools/oracle/hatari.cfg:45@nKbdLayout`, lines 45-47), so the built-in 0/0 stays. READ.
- `nCountryCode` acts elsewhere: `tos.c` overrides the EmuTOS **header**
  country only when `os_conf >> 1 == TOS_LANG_ALL` (i.e. a multilanguage
  ROM) and Hatari has no NVRAM (Hatari src/tos.c lines 1229-1241). For the oracle ROM
  (`os_conf` = 3) the condition is false. READ + DERIVED.
- The vendor-supplied NVRAM does carry a valid checksum (`NvRam_Init` calls
  `NvRam_SetChecksum`, Hatari nvram.c about line 316), so the EmuTOS read succeeds. READ.

### Result

`_AKP` = `MAKE_UWORD(1 (header), 0 (NVRAM byte 7))` = **0x0100**. DERIVED from
the READ chain above.

`_IDT`: see point 4; with `CONF_WITH_NVRAM` the cookie is
`MAKE_UWORD(buf[0], buf[1])` of NVRAM bytes 8, 9 = `0x11, 0x2E` = **0x112E**
(`country.c:167-176`). That value is identical to the `de` table row
(`IDT_24H|IDT_DDMMYY|'.'`), but the source path says it came from Hatari's
NVRAM, not from the table. DERIVED.

### Measurement (this task)

Prediction written before the runs, four variants, tt profile, same ROM,
Hatari, table (`measurements/measurement-2026-10-30-cook02/RUNINFO.TXT`):

| variant | `nKbdLayout` | `nLanguage` | `_AKP` | `_IDT` |
|---|---|---|---|---|
| base (pinned cfg) | -1 | -1 | 0x00000100 | 0x0000112E |
| kbd1 | 1 | -1 | 0x00000101 | 0x0000112E |
| lang2 | -1 | 2 | 0x00000100 | 0x0000112E |
| lang2kbd2 | 2 | 2 | 0x00000102 | 0x0000112E |

MEASURED: the low byte follows Hatari's NVRAM keyboard byte; Hatari's
`nLanguage` (NVRAM byte 6, EmuTOS `buf[0]`) does not reach the cookie in this
ROM; `_IDT` does not change with either. All four predictions hit; the other
20 jar cases are identical. This settles the open point in point 3: the
`0x0100` is "header country DE, keyboard from Hatari's default NVRAM = US",
an emulator-NVRAM default, **not** a keyboard choice made by the `de` ROM.
DERIVED from MEASURED + READ (that a real Atari TT/Falcon with other NVRAM
contents would give another low byte follows from the same code path).

Not tested: a `hatari.nvram` file, a multilanguage ROM, a machine without
NVRAM (`has_nvram` 0). Not measured on real hardware.

## 4. `_IDT`: oracle 0x112E vs NewTOS 0x112F

Bit layout (READ, `references/emutos/include/biosdefs.h:161-173`, and the
comment at `references/emutos/bios/machine.c:745-757`):

| Bits | Meaning |
|---|---|
| 7..0 (`IDT_SMASK 0xFF`) | ASCII code of the date separator |
| 9..8 | field order: `00` MM-DD-YY (`IDT_MMDDYY`), `01` DD-MM-YY (`IDT_DDMMYY` = `IDT_BIT_DM 0x100`), `10` YY-MM-DD (`IDT_YYMMDD` = `IDT_BIT_YM 0x200`), `11` YY-DD-MM |
| 11..10 | not defined in `biosdefs.h`; the `machine.c` comment says bits 8-11 hold the date format value 0..3 |
| 12 (`IDT_TMASK 0x1000`) | `0x0000` 12-hour (`IDT_12H`), `0x1000` 24-hour (`IDT_24H`) |

Decoded:

| | value | high byte | separator | order | clock |
|---|---|---|---|---|---|
| Oracle (EmuTOS `de`, via Hatari NVRAM) | `0x112E` | `0x11` | `0x2E` = `.` | DD-MM-YY (`0x100`) | 24-hour (`0x1000`) |
| NewTOS (`cookie.rs:226@IDT`) | `0x112F` | `0x11` | `0x2F` = `/` | DD-MM-YY | 24-hour |

**The only difference is the separator byte: `.` versus `/`.** DERIVED
(decoding).

How EmuTOS builds it: `detect_idt` takes `MAKE_UWORD(buf[0], buf[1])` from
NVRAM bytes 8 and 9 if `nvmaccess` succeeds, else (no NVRAM or bad checksum)
the `idt` field of the current country's record when `CONF_MULTILANG`, else
the compile-time `CONF_IDT` (`country.c:165-190`). In the oracle build
`CONF_IDT` is `(IDT_24H|IDT_DDMMYY|'.')`
(`references/emutos-oracle/1a7484ab/include/i18nconf.h:9`) and the `de` row is
`IDT_24H|IDT_DDMMYY|'.'` (`references/emutos-oracle/1a7484ab/bios/ctables.h:79`), so the table value is
also 0x112E. READ. A separate value exists in `util/cookie.c`:
`DEFAULT_IDT` = 24 h | `YY-MM-DD` | `/` = 0x122F, used only if the cookie is
missing (`references/emutos/util/cookie.c:16-20`, `:83-91`). READ.

Where NewTOS's 0x112F comes from: `tos-api/src/cookie.rs:223@seed_defaults`,
comment "bring-up policy" (`tos-api/src/cookie.rs:222@seed_defaults`). The docs state the same value as
"`/` separator, DD-MM-YY, 24-hour" (`docs/api/bios/_cookie-jar.md:137`). No
EmuTOS source row carries `/` together with DD-MM-YY and 24 h *and* is the
default for a unique build other than the non-`de` rows (e.g. `fr`:
`IDT_24H|IDT_DDMMYY|'/'`, `references/emutos/localise.ctl:21`). DERIVED
(comparison; not a claim about why 0x112F was chosen).

Effect of the separator on consumers (READ):

- EmuTOS desktop: `fmt_date` takes the separator from `_IDT` only when the
  preference is `DATEFORM_IDT`, and uses `/` otherwise
  (`references/emutos/desk/deskinf.c:124-176`, `desk/deskbind.h` `DATEFORM_SEP`);
  the NewTOS desktop does the same (`programs/desktop/src/redraw.rs:144@refresh_idt`
  feeds the value in).
- QED builds its date format from the order bits only:
  `switch ((unsigned short)_idt & 0xF00)`, `0x000` -> `%m/%d/%Y`,
  `0x100` -> `%d.%m.%Y`, otherwise `%Y-%m-%d`
  (`sdk/apps/qed/src/global.c:319@make_date`). The separator bits are not
  read, so 0x112E and 0x112F give **the same QED output** (`dd.mm.yyyy`).
  DERIVED. QED's own fallback if the cookie is missing is `0x0000112E`
  (`sdk/apps/qed/src/global.c:565@_idt`). READ.

## 5. Survey: who reads `_AKP` / `_IDT`

Search: `/usr/bin/grep -rn` over `programs`, `sdk` (incl. its overlays,
cflib, ldg, libc, windom, zview and apps directories), `Applications`
(source only) and `tosdump`, for the tags `_AKP`, `_IDT`, `C__AKP`, `C__IDT`,
`0x5F414B50`, `0x5F494454`, `COOKIE_AKP/IDT`, and for every
`getcookie` / `Getcookie` / `S_GETCOOKIE` call site. The kernel and `tos-api`
**seed** the tags but never read them (`grep` of `cookie::AKP|IDT` in
`kernel/src tos-api/src tos-types/src` returns only the two definitions,
`tos-api/src/cookie.rs:29@IDT`, `:31@AKP`). READ.

Boot-image coverage: `tools/make-sd-image.sh` `REQUIRED` list
(`tools/make-sd-image.sh:60@REQUIRED`) plus the NEWDESK program the image embeds
(`kernel/Cargo.toml:23@NEWDESK`, `kernel/src/embedded.rs:5@include_bytes`). The standard image
carries these executables that matter here: QED (`QED.NAP`, built by
`sdk/apps/build-qed.sh`), TeraDesk (`DESKTOP.NAP`, `build-teradesk.sh`),
ZVIEW, PRESENT, the tosdump `DUMPVDI.NAP`, the libc test programs, and the
embedded NEWDESK. **Not** in `REQUIRED`: Thing, the LDG installer, windom
applications, HighWire (built by script but not listed). READ for the list;
the "built for NewTOS" column below is that mapping.

| # | Hit | Program | What it does with the value | Built for NewTOS / on the image? |
|---|---|---|---|---|
| 1 | `programs/desktop/src/redraw.rs:144@refresh_idt` (reads `_IDT` through `dos_getcookie`, `programs/newtos-gem/src/gemdos.rs:453@dos_getcookie`) | NEWDESK (the EmuDESK rebuild) | `G.g_idt`; date separator, field order and 12/24 h of the desktop file list when the format preference is "Default" (`programs/desktop/src/model.rs:66-110@IDT_TMASK`, `programs/desktop/src/inf.rs:2719-2724@fmt_date`) | Yes, embedded in the kernel image (`kernel/Cargo.toml:23@NEWDESK`) |
| 2 | `sdk/apps/qed/src/global.c:564@getcookie` (`getcookie("_IDT", &_idt)`; use at `sdk/apps/qed/src/global.c:319@_idt`) | QED | Date string order (`0x000` / `0x100` / else); separator and time bits ignored. Never reads `_AKP`. | Yes: `QED.NAP` is in `REQUIRED` (`tools/make-sd-image.sh:75@qed`). Source under `sdk/apps/qed/src` is the upstream tree (gitignored `/apps/*/`, `sdk/.gitignore:77`); the file is not patched (`sdk/overlays/qed` holds only `memory.c`, `set.c`) |
| 3 | `sdk/overlays/teradesk/tests/cookie-host.c:73-74@_IDT`, `sdk/overlays/teradesk/tests/cookie-probe.c:61-62@_IDT`, `sdk/overlays/teradesk/tests/nohard-host.c:56-71@TAG_IDT` | TeraDesk **tests** | Seed/regression tests for the jar (expect `_IDT 0x112F`, `_AKP 0`); not TeraDesk runtime code. No `_AKP`/`_IDT` reference in the TeraDesk sources themselves (grep of `sdk/apps/teradesk`). | Tests only; TeraDesk (`DESKTOP.NAP`) is on the image but does not read the tags |
| 4 | `sdk/cflib/tests/cookie-host.c:37-38@_IDT`, `sdk/cflib/tests/cookie-host.c:123-124@_IDT`, `sdk/cflib/tests/cookie-probe.c:40-42@_IDT` | cflib tests | Same, test data only. The library `getcookie()` is generic (`sdk/cflib/overlay/cookie.c:59@getcookie`) | Host tests |
| 5 | `sdk/run-host-tests.sh:277@_IDT` | host test script | Comment about the jar's five tags | Host only |
| 6 | `sdk/ldg/ldg-r133/inst/src/install.c:586@ldg_cookie` (`ldg_cookie('_AKP', &l)`; `if ((int)l == 2) lang = LAN_FR`) | LDG installer (`inst`) | Selects French vs. English UI by comparing the **whole** cookie with 2. It uses this only if `has_appl_getinfo()` is false; else `appl_getinfo(3, ...)` | **No**: `sdk/ldg/build-ldg*.sh` do not build the `inst` directory, and no installer is in `REQUIRED` |
| 7 | `Applications/thing-neo-codeSVN-r62/Source/ThingTbx/src/system.c:55,61` (`Getcookie(C__AKP, &value)`, then `value >>= 8`) | Thing toolbox (`sysLanguageCode`) | Language from the **high byte** (0 en, 1 de, 2 fr, 5 ...) used to pick a language code string; used only when `appl_getinfo(3)` is unavailable | **No**: source tree under `Applications/`, referenced by `TASK/PORT02.md` as a port candidate; no build script in `sdk/` and nothing in `REQUIRED` |
| 8 | `sdk/libc/mintlib/include/mint/cookie.h:590,600` (`C__AKP`, `C__IDT`) and `Applications/mintlib-src/mintlib-0.60.1/include/mint/cookie.h` | mintlib | Constants only. No mintlib `.c` file under `sdk/libc` looks up `_AKP`/`_IDT` (`Getcookie` uses there: `_CPU`, `_MCF`, `hade`, `_MCH`, `MiNT`, `__NF`) | n/a |
| 9 | `sdk/libc/mintlib/include/mint/ostruct.h:653`, `ChangeLog:1590` | mintlib | A comment about the `_AKP` cookie and an old changelog line | n/a |
| 10 | `tosdump/cases/jar-r1.tsv:52-53@jar` | tosdump jar cases | Reads the cookies as measurement subjects on EmuTOS and on NewTOS | `DUMPBIOS.NAP` is not in `REQUIRED`; built on demand by `cargo xtask tosdump` |

Programs that read the jar but **not** these two tags (checked, for
completeness): HighWire reads `_FSMC`, `NVDI`, `EdDI`
(`sdk/apps/highwire/src/romvdi.c:70@getcookie`, `sdk/apps/highwire/tools/num_char.c:201@getcookie`, `sdk/apps/highwire/tools/num_char.c:213@getcookie`); HypView
reads `MagX`, `DHST` (`sdk/apps/hypview/dl_ledit.c:143@Getcookie`, `sdk/apps/hypview/dl_dhst.c:45@Getcookie`); QED
also reads `DHST` (`sdk/apps/qed/src/comm.c:62@getcookie`); cflib reads `MiNT`, `MgPC`,
`nAES`, `NVDI`, `FSEL`, `Gnva`, `xFSL`, `MagX`; the zview JPEG plugin reads
`_JPD`, `_MCH`, `_CPU`; the windom `get_cookie` is generic with no tag of its
own. READ.

`Applications/Vanilla-Conquer-Atari-ST` hits are measurement logs
(`GEM00.TXT` and `run-host.log` files), not source that reads the tags. READ.

Also relevant to the "who sees it" question: EmuTOS has no `appl_getinfo`
(`grep -i getinfo` over `references/emutos/aes/*.c *.h` returns nothing), so
the programs in rows 6 and 7 fall to the `_AKP` branch on an EmuTOS-faithful
AES. Whether NewTOS implements `appl_getinfo` was not determined here
(`tos-api/src/aes` was searched for `getinfo` without a hit; not read further).
READ (the grep results), the consequence is DERIVED.

## 6. Facts for a human-readable country setting

### 6.1 How `cmdline.txt` and `config.txt` reach the kernel

- **`cmdline.txt` (boot arguments).** On `board-rpi4` the kernel asks the
  VideoCore for the "Get command line" tag (`0x0005_0001`,
  `kernel/src/drivers/bcm2711/mailbox.rs:523@PROPTAG_GET_COMMAND_LINE`) via
  `kernel/src/drivers/bcm2711/mailbox.rs:556@with_command_line` (the doc comment names circle
  `references/circle/lib/koptions.cpp:52-61` as the source). `kernel/src/lib.rs:1240@boot_switch`
  hands the string to a decider closure; on QEMU `raspi4b` the same tag
  answers `-append`; on QEMU `virt` the string is `/chosen/bootargs` of the
  DTB (a 256-byte buffer). READ (`kernel/src/bootargs.rs:5@Get`, lines 5-8,
  `kernel/src/lib.rs:1236-1257@boot_switch`).
- **Parser**: `kernel/src/bootargs.rs:140@has_token` (whole-token match, split
  on ASCII whitespace, stops at NUL) and `kernel/src/bootargs.rs:113@numeric_setting`
  (first token with the prefix; decimal digits only, at most 10, bounded by
  `max`; returns `Default` / `Given(n)` / `Invalid`). Pure byte logic, no
  allocation. READ.
- **Existing keys** (all constants in `kernel/src/bootargs.rs`): `serial=on`
  (`:22`), `memprot=off|strict|verify` (`:27,32,39`), `asid=on|tiny`
  (`:45,50`), `emmc=compat` (`:57`), `cpuclk=firmware` (`:63`), `input=aes`
  (`:73`), `bcache=<KiB>` (`:80`), `bcachepct=<0..100>` (`:85`),
  `mdlimit=<n>` (`:94`). No key with a string value is parsed today:
  `has_token` matches complete tokens and `numeric_setting` accepts digits
  only. The consumers sit in `kernel/src/lib.rs` (lines 231, 257-281,
  398, 1053-1055, 1139-1141, 1194). READ. That a `country=xx` string
  value would need a new accessor is DERIVED.
- The decider runs at boot while the boot tables still reach the DTB
  (`kernel/src/lib.rs:1236-1240@boot_switch`), i.e. **before** the filesystem is mounted;
  `keymap::init_from_disk()` runs later, once `C:` exists (`kernel/src/lib.rs:409@init_from_disk`,
  `kernel/src/keymap.rs:367@init_from_disk`). `seed_defaults` runs at `kernel/src/lib.rs:362@seed_defaults`.
  The relative order of those three calls in `lib.rs` was read from the line
  numbers only, not from the control flow between them. READ (lines) /
  order is DERIVED from line order.
- **`config.txt`** is read by the Raspberry Pi firmware, not by the kernel.
  Grep for `config.txt` in `kernel/src`, `tos-api/src`, `programs/` finds only
  comments about geometry the firmware already set (`kernel/src/drivers/bcm2711/framebuffer.rs:24@config`,
  lines 278 and 469-471 of the same file; `kernel/src/drivers/mod.rs:19@config`;
  `kernel/src/arch/aarch64/boot.rs:204@config`). **No kernel code reads `config.txt`
  as a file.** READ (grep). The tracked sample is `boot/rpi4/config.txt`
  (firmware keys such as `arm_64bit`, `kernel`, `hdmi_group`, `gpu_mem`);
  the tree has no `cmdline.txt` sample beside it (`ls boot/rpi4` shows only
  `config.txt`). READ. What the firmware does with unknown `config.txt` keys
  was not looked up here (BCM2711/firmware documentation not consulted).

### 6.2 Other per-machine settings files

- `C:\KEYBOARD.KBD`: a native `KBD1` blob (magic, three 128-byte tables, AltGr
  pairs), tried first (`kernel/src/keymap.rs:370@KEYBOARD`, format at
  `kernel/src/keymap.rs:415@KBD_MAGIC`). READ.
- `C:\NEWDESK.INF` line `#K <layout>`: read by `find_k_line`
  (`keymap.rs:386@find_k_line`) and applied by `select_builtin`
  (`keymap.rs:236@select_builtin`), which knows only `us`/`US` and `de`/`DE`
  (`kernel/src/keymap.rs:238@select_builtin`); any other name returns false and leaves US.
  READ.
- Sample layouts in the repository: `keyboards/german.kbd`, `keyboards/us.kbd`.
  READ (`ls keyboards`).
- The language of OS-owned strings is fixed English: `current_lang()` always
  returns `Lang::En` (`tos-api/src/i18n.rs:38@current_lang`). READ.
- `_AKP` and `_IDT` are constants written by `seed_defaults`
  (`tos-api/src/cookie.rs:223@seed_defaults`); neither is derived from the
  keymap, the INF file, or the boot arguments. READ (see point 2).
- The desktop's own date/time preference lives in `NEWDESK.INF` as the `#E`
  byte's `INF_E2_IDTDATE` / `INF_E2_IDTTIME` bits and defers to `_IDT` by
  default (`docs/api/desktop/deskinf.md:675-688`,
  `programs/desktop/src/inf.rs:161-166@INF_E2_IDTDATE`). READ.

### 6.3 What EmuTOS' `countries[]` row would select

The record is `{ country, lang_name, keyboard, charset, idt }`
(`references/emutos/bios/country.c:29-35`). The `de` example, from the oracle
build's generated table (`references/emutos-oracle/1a7484ab/bios/ctables.h:79`)
and the control file (`references/emutos/localise.ctl:20`):

| Field | `de` value | Used for | Cited use |
|---|---|---|---|
| `country` | `COUNTRY_DE` = 1 | the code in the `_AKP` bytes; lookup key | `ctrycodes.h:24`; `country.c:75-90` |
| `lang_name` | `"de"` | translation (NLS) language name | `country.c:116-121` (only `CONF_MULTILANG`) |
| `keyboard` | `KEYB_DE` = index 1 into `keytables[]` (`keyb_de`) | key translation tables | `ctables.h:22`, `:59`; `country.c:101-106`, `:190-196` |
| `charset` | `CHARSET_ST` (`st`) | system font set (6x6/8x8/8x16) | `country.c:123-128`, `:205-222`; in `localise.ctl` the `cz hu pl ro` rows name `l2`, `ru` `ru`, `gr` `gr`, `tr` `tr` |
| `idt` | `IDT_24H|IDT_DDMMYY|'.'` = 0x112E | `_IDT` when no NVRAM and `CONF_MULTILANG` | `country.c:181-184` |

Full control file (country, language, keyboard, font, IDT): `us us us st
12H/MMDDYY//`, `de de de st 24H/DDMMYY/.`, `fr fr fr st /`, `uk us uk st 12H /`,
`es es es`, `it it it`, `se us se`, `sg de sg .`, `fi fi se .`, `no us no /`,
`nl nl nl -`, `cz cs cz l2 .`, `hu hu hu l2 /`, `pl pl pl l2 /`, `ru ru ru ru
/`, `gr gr gr gr /`, `tr tr tr tr .`, `ro ro ro l2 .`, `ca ca es st /`
(`references/emutos/localise.ctl:18-38`). READ. Note the difference between
the table row and the oracle's generated `ctables.h`: the oracle build lists
`CHARSET_ST` for `cz/hu/pl/ru/gr/tr/ro` too (`ctables.h:89-95`), because a
unique-country build keeps only the `de` font set (`ctables.h:9-18`).
READ.

How the pieces would be selected, per the code path (DERIVED where marked):

- A single value `country=xx` would have to be turned into the two `_AKP`
  bytes. EmuTOS has two independent inputs (display byte, keyboard byte), so
  `country=de` naturally corresponds to bytes `(1,1)` = 0x0101 in a build
  where both come from the OS header (`country.c:142-143`, case without
  NVRAM). DERIVED.
- The row picked by the display byte decides language, charset and (without
  NVRAM) the `_IDT`; the row picked by the keyboard byte decides only the
  keyboard table (`country.c:96-99`, `:108-130`, `:181-184`). READ.
- NewTOS today has: keyboard tables for `us` and `de` only
  (`kernel/src/keymap.rs:238@select_builtin`), English strings only
  (`tos-api/src/i18n.rs:38@current_lang`), one font set. So for NewTOS only the keyboard
  and `_IDT` fields of a row have an implementation behind them; `lang_name`
  and `charset` have none. DERIVED from the READ facts.
- EmuTOS selects fonts for `CHARSET_ST` via `font_sets[0]`; the Latin-2,
  Cyrillic, Greek and Turkish sets exist only for those rows
  (`references/emutos/localise.ctl:28-37`). READ.

## Out-of-scope observations

- `tos-api/src/cookie.rs:30@AKP` and `tos-api/src/i18n.rs:38@current_lang`
  say "low word = language id"; against `country.c` that is imprecise (point 2).
  Not edited.
- `docs/api/bios/_cookie-jar.md:56` has the same one-word wording.
  Not edited.
- `docs/api/gemdos/154-ssystem.md:647` says `_AKP` "(keyboard language)".
  Not edited.
- `references/emutos/include/i18nconf.h` and `bios/ctables.h` are
  generated for a different build than the oracle's (see top); a reader who
  cites them for the oracle gets the `us` build.
- Whether the 512 target sits inside a `CONF_WITH_NVRAM 0` block of
  `config.h` (`:289`, `:412`, `:770`) was not read block by block; the
  measurement shows NVRAM active.

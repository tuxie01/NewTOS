# ORACLE — the pinned EmuTOS reference machine

The oracle is the machine every tosdump reference is measured on: one EmuTOS
ROM, built from a pinned source commit, run on one pinned Hatari with one
pinned configuration. This file states what it is, how to rebuild it, and
the calibration that ties it to the `tosdump/reference/EMUTOS14-*` dumps
measured before it existed.

Evidence for everything below: `measurements/measurement-2026-10-26-oracle/RUNINFO.TXT`.

## 1. The pin

| Item | Value |
|---|---|
| EmuTOS commit | `1a7484aba1bfba23e2c0899e243b6d8177fe41df` (2026-04-14, = `references/emutos` HEAD, the tree `docs/api/` is extracted from) |
| Build | `make 512 COUNTRY=de` → `TARGET_512`, `-DWITH_AES=1 -DWITH_CLI=1`, no `localconf.h`; CONF table: `docs/api/_emutos-config.md` |
| Build date (pinned) | 2026-04-14 = the commit date → `OS_DATE 0x04142026`, version string `"20260414"` |
| Toolchain | `/opt/cross-mint/bin/m68k-atari-mint-gcc` — `m68k-atari-mint-gcc (MiNT 20230908) 7.5.0` |
| Host tools | built `-arch x86_64` (Rosetta), see §3 |
| ROM | `tools/oracle/etos512de-1a7484ab.img` (committed, the safeguard); rebuilt into `references/emutos-oracle/1a7484ab/` (gitignored) as the cross-check |
| ROM sha256 | `323c87bbff65096e54a8dc00b474f6924a950ebf25cc9513bd53d495a2d2750c` |
| Hatari | 2.6.1 (macOS bundle, compiled Aug 15 2025), `/Volumes/Daten/hatari-2.6.1_macos/Hatari.app/Contents/MacOS/Hatari`, binary sha256 `953f4e78eea55a8ca12ed6c83de1f045545b8da42a49e00112eaa235dffcd6f6` |
| Hatari config | `tools/oracle/hatari.cfg`, sha256 `c8f4998aa139115d97dbb3d14a07e1cc05e8bc848350ff841ecfb9d038f8ae46`: only result-relevant settings, no paths (§2) |

All of these live in **one** file, `tools/oracle/oracle.env`, read by
`tools/oracle/build-emutos.sh`, `tools/hatari-oracle.sh`, `tools/hatari-qed.sh`,
`tools/emutos-config.sh` and `tosdump/Makefile`. Nothing is taken from `$PATH`; `/opt/cross-mint` is
not on it.

## 2. Machine profiles (`tools/hatari-oracle.sh`)

Common to all profiles: `HOME` = an empty directory (no user config, no user NVRAM —
Hatari starts with a fresh NVRAM), `--configfile tools/oracle/hatari.cfg`,
`SDL_VIDEODRIVER=dummy`, `--sound off --fast-forward on --conout 2`,
GEMDOS drive C: (profile `disk`: D:) = a host directory holding the dumper
and its table, `--auto C:\DUMP*.PRG` (`disk`: `D:\DUMP*.PRG`). The run ends
when the dumper's final LOG is on the drive; Hatari is stopped with SIGKILL (SIGTERM opens the "really quit?"
dialog, `bConfirmQuit = TRUE`).

`tools/oracle/hatari.cfg` holds only settings that can change what EmuTOS
or a dumper sees, no paths: `[System]` (model, CPU 68030 @ 32 MHz, FPU 68882,
cycle-exact, data cache, 32-bit address space, no blitter/DSP/MMU,
`bFastBoot = FALSE`, `VideoTiming = 3`), `[Memory]` ST-RAM 4096 KiB, TT-RAM
16384 KiB, `[ROM] bPatchTos = TRUE`, `[Screen]` monitor and
`bUseExtVdiResolutions = FALSE`, `[Keyboard]` country/layout/language `-1`,
`[HardDisk]` GEMDOS case/conversion/host time, `[Log] bNatFeats = FALSE`,
`[Debugger] nExceptionDebugMask`. The values are those of the user
configuration every Hatari reference up to 2026-10-26 ran with (sha256
`dfc52b8d…0ead`, unchanged since 2026-07-21); values equal to Hatari's
default are still stated. Checked: the configuration Hatari ends up with
from this file differs from the one it ended up with from the full user file
only in paths (all replaced on the command line or unused) and one joystick
key; the control run with it reproduces the earlier control run byte for
byte (§4). `--saveconfig` must never be combined with it: Hatari writes the
whole configuration back into the `--configfile` path.

| Profile | Command-line machine | Screen | References |
|---|---|---|---|
| `tt` | `--machine tt --tos-res ttmed --monitor vga` | 640×480, 4 planes (TT-Medium) | `EMUTOS14-VDI`, `-VDI-r2`, `-VDI-r3`, `-VDI-all`, `-AES`, `-AES-r2wind`, `-AES-r2brlist`, `-AES-r3`, `-AES-m05/m06/m07`, `-RASTER`; pin only: `EMUTOS-1a7484ab-BIOS` |
| `falcontc` | `--machine falcon --monitor vga` + `EMUDESK.INF` `#E 1A E0 00 1C 60` | 640×480, 16-bit truecolor (VIDEL `0x001C`, read by `process_inf1`, `aes/geminit.c`) | `EMUTOS14-VDI-all-falcontc`, `-RASTER-screen-falcontc`; pin only: `EMUTOS-1a7484ab-BIOS-falcontc` |
| `disk` | `--machine falcon --monitor vga --drive-a off --drive-b off` + `--ide-master <clone of the test image> --ide-swap 0=off`, dumper on GEMDOS drive D: | 640×480, EmuTOS' default mode (`VsetMode(-1)` = `0x001A`, no INF) | pin only: `EMUTOS-1a7484ab-BIOS-r2-disk` and the closing tables' `-BIOS-r2-<phys\|x004\|x005\|x006>-disk` (TASK/TOOL07); canary §6 |

The INF is byte-identical to `measurements/measurement-2026-09-21-vdi-qemu/method/EMUDESK-falcon-truecolor.INF`.

### Profile `disk` — EmuTOS on its own block device

For disk cases (`Rwabs`, `Getbpb`, `Mediach`, `Drvmap`, later GEMDOS on a
real FAT): Hatari's GEMDOS drive answers GEMDOS calls itself, so a disk case
on it would measure Hatari. In this profile EmuTOS reads and writes the test
image through its own IDE driver. Evidence for every statement below:
`measurements/measurement-2026-10-28-tool06/RUNINFO.TXT`.

```
HATARI_ORACLE_DISK=<test.img> tools/hatari-oracle.sh disk <DUMP*.PRG> <table.tsv> <out-dir>
```

The script clones the image to `<out-dir>/disk.img` (the caller's file is
never attached; the caller deletes the clone) and writes the sha256 before
and after the run to `<out-dir>/disk.sha256`. The Hatari command is the one
of the other profiles with: `--machine falcon --monitor vga --drive-a off
--drive-b off`, `--ide-master <out-dir>/disk.img --ide-swap 0=off`,
`-d <out-dir>/hd --gemdos-drive D --auto D:\<DUMPER>`.

| Drive | What | Set by | `_drvbits` |
|---|---|---|---|
| A:, B: | absent (`--drive-a off --drive-b off`) | — | — |
| C: | the test image, IDE master, its one FAT16 partition | EmuTOS (`bios/disk.c`, IDE unit) | bit 2 |
| D: | host directory: dumper, table, BIN, LOG | Hatari's GEMDOS emulation | bit 3 |

`Drvmap()` = `0x0000000C`, measured. Controls: without `--ide-master` the
boot screen lists drive D only; with Hatari's default floppies (both drives
on, no disk inserted) `Drvmap()` = `0xF`, and `Giaccess(0,14)` and both
`Floprate` cases change — the other `bios-r1.tsv` answers stay the same.

- **Machine Falcon030**: the Falcon has its IDE port on board, and NewTOS'
  target profile is the Falcon030/TOS4 one. Against the `falcontc` reference
  `EMUTOS-1a7484ab-BIOS-falcontc`, `bios-r1.tsv` differs in four cases only:
  the floppy-related `Giaccess(0,14)`, `Floprate(0,-1)`, `Floprate(0,-4)`
  (the drives are off), and `VsetMode(-1)` (`0x001A`: the desktop reads no
  INF with `#E`).
- **Why the dumper is on a Hatari GEMDOS drive.** Measured alternative: the
  dumper on a second IDE image (`--ide-slave`), no GEMDOS drive at all. EmuTOS
  then has C: and D:, but `--auto` does nothing — Hatari's `--auto` works only
  through its GEMDOS emulation (it overrides the desktop's INF), and the
  dumper never starts. So D: is a GEMDOS drive, and only D:.
- **What Hatari's GEMDOS layer does on C:**: Hatari's trace shows every GEMDOS
  call on C: handed to EmuTOS (`-> to TOS`), except one — the desktop's first
  `Fopen("C:\EMUDESK.INF")`, which Hatari answers from its `--auto` INF
  override (a host file, handle 64), without touching the image. EmuTOS reads
  the image itself: IDE `READ MULTIPLE` of LBA 0 (MBR), 63 (boot sector) and
  320 (root directory, = `Rwabs(0,…,1,257,2)` after `Fsfirst("\AUTO\*.PRG")`).
  No IDE write in a `bios-r1.tsv` run; the image's sha256 is unchanged.
- **`--ide-swap 0=off`**: EmuTOS gets the sectors in file order, as NewTOS
  gets them from the SD card, and its IDE unit stays unswapped
  (`bios/disk.c:867-870` enables its own byte swap only for a swapped MBR).
  Traced: with `off` and with Hatari's default `auto` the boot sector arrives
  as `eb3c 904d …` (file order), with `on` as `3ceb 4d90 …`; all three give
  the same `bios-r1.tsv` answers (EmuTOS swaps back). `off` is stated so that
  Hatari's heuristic cannot change it for another image.
- **No own Hatari configuration.** As for `tt` and `falcontc`, what differs
  between profiles is on the command line; `tools/oracle/hatari.cfg` holds
  what they share and stays unchanged.
- **The physical unit of C: is 18**, EmuTOS' IDE master `2 + 8 × 2`
  (`bios/disk.h:30`). The profile names it in `D:\DISKUNIT.TXT` (`18`), which
  DUMPBIOS reads for the unit token `u:0` of an `Rwabs(RW_NOTRANSLATE)` case
  (`docs/api/bios/04-rwabs.md`, "Decisions"; `docs/TOS-REFERENCE-DUMPER.md`
  section 6, "The closing tables"); NewTOS' runner names 26, the SD card.
- **Destructive tables** (`!disk <drive> destructive`, the closing tables)
  run only in this profile: the script refuses them in any other, clones the
  test image for the run as for every disk table, and refuses an image whose
  partition 1 fails `fsck_msdos -n`
  (`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`).

The test image comes from one command, `tools/mktestdisk.sh <image>
[dest=host-file …]`: 64 MiB, MBR with one FAT16 partition (type `0x06`,
LBA 63, 130977 sectors — the geometry and BPB of `tools/mkcard.sh`'s
default card), volume serial `4E54-5444`, label `NTTESTDISK`, every FAT time
stamp fixed. It is byte-reproducible (mtools with `SOURCE_DATE_EPOCH`, no
mount); `fsck_msdos -n` on the partition is clean. The image of the canary
(no files) has sha256 `d83c543471fd30f29e4704cb9450f70517fe43a4e5e0250e137205e409bfc4a0`
with mtools 4.0.49; mtools writes its version into the boot sector
(`MTOO4049`), so another mtools gives another image — the canary checks it.

Two values are not the oracle's but the run's: `Gettime` (the RTC, which
Hatari starts from the host clock) and the first `Random` (seeded from
`hz_200` at its first call, `bios/xbios.c:421-426`; the tick count at that
point varied between runs). Measured: two runs of `bios-r1.tsv` differed in
`Gettime` alone; with `Gettime` left out, two runs differed in `Random` alone
(BIN only, LOG identical). The disk canary in §6 leaves both out; five runs
without them were byte-identical.

## 3. Rebuilding the ROM

```
tools/oracle/build-emutos.sh 1a7484aba1bfba23e2c0899e243b6d8177fe41df 2026-04-14 references/emutos-oracle/1a7484ab
```

The script clones `references/emutos` (read-only; never built in) into the
tree directory, checks out the commit, refuses a dirty tree, and runs
`make 512 COUNTRY=de` with an absolute `TOOLCHAIN_PREFIX`. Two host inputs
are pinned, each because it otherwise makes two builds of one commit differ:

1. **Build date.** `tools/mkheader.awk` asks `date +%Y,%m,%d` (OS header
   `os_date`/`os_dosdate`, copyright year); a snapshot tree's `version.mk:40`
   sets `VERSION = $(shell date +%Y%m%d)` (the boot-screen version string).
   A `date` shim in front of `PATH` answers every `date +FORMAT` for the
   pinned day. The pinned day is the commit date. No `__DATE__`/`__TIME__`
   in the tree (checked).
2. **Host tool architecture.** `tools/bug.c`, at both commits, allocates
   `len+3` bytes for `<name>.tr.c` and needs `len+4` (`translate()`,
   `bug.c:2538-2540` at `VERSION_1_4`). macOS 27 arm64's allocator aborts on
   the one-byte overflow (first victim `bios/kprint.c`; `-D_FORTIFY_SOURCE=0`
   and `MallocNanoZone=0` do not help). Built `-arch x86_64` the overflow
   lands in allocator slack; `bios/kprint.tr.c` from that build is identical
   (sha256 `3bffd867…5630`) to the one from an arm64 AddressSanitizer build
   that reports the overflow and recovers. Only host tools are affected —
   the ROM's own compile flags are EmuTOS's. ROM level: every one of the
   604 distinct string literals in the eleven `bug`-translated sources
   (`*.tr.c`) is either in both the own 1.4 build and the release ROM as a
   NUL-terminated byte string (502) or in neither (102 — debug output that
   is compiled out, or fragments of joined literals); none is in only one
   (`measurements/measurement-2026-10-26-oracle/method/check-translations.py` of the run). The translations of the
   x86_64-tool build equal the release's.

Reproducible: a second build in a different directory gives the same sha256
(checked for both the pin and the calibration build). For the pinned commit
the script compares the result with `ORACLE_ROM_SHA256` and fails on a
mismatch.

## 4. Calibration — why the EMUTOS14 references still count

The `EMUTOS14-*` references were measured with the **EmuTOS 1.4 release
ROM** `etos512de.img`, sha256 `5805c0a4834f8935c427454785d32c59e796b11ad6802101052b84fb1eb7a745`
(on the real TT, later on Hatari; OS header date 2025-06-07 = tag
`VERSION_1_4` `978e37569bff95841e42675d11fcc6799aad8483`). The pin is 71
EmuTOS commits later and built with a different toolchain. Two variables,
changed one at a time:

| Run | ROM | Measures |
|---|---|---|
| control | release 1.4 (`5805c0a4…`) | dumper (HEAD source, each reference's own build id) + Hatari + pinned config vs. the references |
| calibration | own build of `VERSION_1_4`, date 2025-06-07, sha256 `67bc59a6e182e3399eaff479184d17017e34789cda4c814e08530874dbec34cf` | **toolchain**: same source as the release, our compiler |
| pin | own build of `1a7484ab` (`323c87bb…`) | **source**: the 71 commits, same compiler as calibration |

Each run covers all 14 `EMUTOS14-*` tables (938 cases) on their own profile.

**Control vs. references:** 13/14 BIN+LOG bit-identical; `EMUTOS14-RASTER`
0/59 cases, its bytes differ only in the raster CONTRL's MFDB pointer slots
(the dumper's own buffer addresses — the HEAD dumper is laid out differently
from the original `fdf8` binary). Hatari, the pinned config and the
fresh-NVRAM `HOME` reproduce the references. Repeated with the cleaned
`hatari.cfg`: byte-identical to the first control run in all 14 tables.

**Calibration vs. release (toolchain effect):** 0 cases in all 14 tables.
Bytes differ in exactly one field: `global[12]`, the low word of `ap_3resv`,
0xBE04 → 0xBD5C, in every AES case of all seven AES tables (record offset
38–39 only), and in the LOG line that prints `global`. `ap_3resv` is
`pglobal->ap_3resv = (LONG)&D` (`aes/gemsuper.c:98`): the address of the
AES global area, i.e. the linker's layout, not behaviour. Field class
**address** since this calibration (`tosdump/core/record.c`,
`td_aes_fields`): recorded, never compared by value. VDI and raster:
bit-identical.

**Toolchain changes layout only. Calibration passed.**

## 5. What the pin changes against 1.4 (source effect)

Pin vs. calibration build, all 14 tables:

| Table | Cases | Field(s) | Commit |
|---|---|---|---|
| `EMUTOS14-AES` (aes-r1) | 1/95: case 167 `wind_get 0 WF_OWNER` | `int_out[0]` 0 → 1 | `acb7f3b4` Add partial implementation of WF_OWNER to wind_get (2025-08-23) |
| `EMUTOS14-AES-r2wind` | 2/78: case 244 (created window), 247 (open window), both `WF_OWNER` | `int_out[0]` 0 → 1; `int_out[1..4]` = `p_pid`, `VF_ISOPEN`, `gl_wtop`, `W_TREE[ROOT].ob_head` | `acb7f3b4` (`aes/gemwmlib.c` `wm_get`, `case WF_OWNER`) |
| all 7 AES tables | 0 cases (address class) | `ap_3resv` 0xBD5C → 0xBD60 | layout — not attributable to one commit, by design not compared |
| VDI ×5, RASTER ×2, AES r2brlist/r3/m05/m06/m07 | 0 | — | — |

Every compared difference maps to one commit. **No unmappable difference.**
The other 70 commits (38 of them in `bios/`) change nothing these tables see.

## 6. Canary

The canary proves that the oracle in front of you is the pinned one: same
ROM, Hatari, configuration and dumper output as when the references were
recorded. The auditor pre-flight runs it before every audit
(`.claude/agents/auditor.md`, "Pre-flight"); any other result than two OKs
stops the audit (BLOCKED).

| | |
|---|---|
| Cases | `tosdump/cases/vdi-all.tsv`, 182 cases, CRC `D135ADAF`, sha256 `7ca7de7ce52af54e82bfe0dca8dff310b32f75359177068060172a841a764ada` |
| Dumper | `DUMPVDI.PRG` built from the current tree with build id `19b7` (the id in the references' header) |
| ROM | the pin, `tools/oracle/etos512de-1a7484ab.img` (§1); `tools/hatari-oracle.sh` refuses another sha256 |
| Profiles | `tt` and `falcontc` (§2) |
| Expected `tt` | `tosdump/reference/EMUTOS-1a7484ab-VDI-all.BIN` sha256 `715c734cbac5fdfeb61bb107ea76b807c60e4dd0470b95ae7cc3628d21def8b6`, `.LOG` sha256 `d30e973c0c0646e2aee035b9f3c019c0b18270ad931f68f66d7da802c6945936` |
| Expected `falcontc` | `tosdump/reference/EMUTOS-1a7484ab-VDI-all-falcontc.BIN` sha256 `81b3bdba2004e7f812739d7f389bfca726d4f46b92d26ae610c04f73e9821dab`, `.LOG` sha256 `64dacda7760dab0561630990c69514b4f61d27e123fd47079dff15ce38d32742` |
| Pass | both profiles: `VDI.BIN` and `VDI.LOG` byte-identical to the expected files (`cmp`) |
| Runtime | about 12 s per profile |

Command, from the repository root. `SCRATCH` is `$CLAUDE_JOB_DIR/tmp` or,
when that is empty, the session scratchpad — never `/tmp`, never inside the
repository; the timestamp keeps the directory new, which the script requires:

```sh
sh measurements/measurement-2026-10-28-oracle-canary/method/canary.sh "$SCRATCH/canary-$(date +%Y%m%d-%H%M%S)"
# prints "canary tt: OK" and "canary falcontc: OK"; exit 0 only if both pass
```

It builds the dumper with `make -C tosdump BUILD=<dir>/dumpers/19b7
BUILD_ID=19b7 dumpvdi-m68k`, runs `tools/hatari-oracle.sh <profile>
<dir>/dumpers/19b7/DUMPVDI.PRG tosdump/cases/vdi-all.tsv <dir>/<profile>`
for each profile and compares both files. Last pass: 2026-10-28 on
`d8e9628c` (`measurements/measurement-2026-10-28-oracle-canary/RUNINFO.TXT`).

Why this table: it reproduces byte for byte with a current dumper. It holds
no pointer cells into the dumper's own text (unlike RASTER, §7), and its
output stayed identical while the dumper code grew. If a dumper change ever
breaks the canary, the fix is a re-recording under §7, not a looser compare.

History: the first canary (2026-10-26, Phase 0 step 5,
`measurements/measurement-2026-10-26-oracle/RUNINFO.TXT`) compared against
`EMUTOS14-VDI-all` and `-falcontc`, BIN and LOG bit-identical, with the
user's full Hatari configuration and again with the cleaned `hatari.cfg`.
Those references stay record format 2 (frozen 1.4 evidence), which no
dumper built since format 3 (`d42675e5`) reproduces; the canary therefore
moved to the format-3 pin references above.

### Canary of the disk profile

Proves the same for profile `disk`, plus the test image. An audit with disk
cases runs it in addition to the two canaries above.

| | |
|---|---|
| Cases | `tosdump/cases/bios-r2.tsv`, the disk round's non-destructive table, as it is: 51 cases, CRC `09C71B80`, sha256 `071300e43736d1c3371e65d4a0e2b3f4b5e327dcd4d7147763fe0c04e553bc44` (no time-dependent case; its writes write back what was just read) |
| Dumper | `DUMPBIOS.PRG` built from the current tree with build id `b81d` (the id of the reference) |
| Image | `tools/mktestdisk.sh <dir>/test.img NEWDESK.INF=<inf>`, the INF `#R 02` / `#Z 01 D:\DUMPBIOS.NAP@` (CRLF, sha256 `c779a308dbfae3b61747e3ebcf5d45f7d616bc53e63b83ec4afa3557f02ecf5b`, written by the script), image sha256 `d95a60c3f5d76fee767b9dc5cd16dab3b2bc1215e116fe738cbdbaa318a92d37` |
| Expected | `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk`: `BIOS.BIN` sha256 `072d8cd1bdbc03127d6175d888f51fb2ec100209d96b61f1a4c5f0c684177b1e`, `BIOS.LOG` sha256 `055e586a71f6c3bc32065d08099b14b5e8ee50d1871191083724a52e768a5a76`; the image unchanged by the run |
| Runtime | about 10 s |

```sh
sh measurements/measurement-2026-10-28-tool07b/method/canary-disk.sh "$SCRATCH/canary-disk-$(date +%Y%m%d-%H%M%S)"
# prints "canary disk: OK ..." and "canary disk: test image unchanged ..."; exit 0 only then
```

`SCRATCH` as above. The script checks the table, the INF and the image
against their sha256 before the run, then runs `tools/hatari-oracle.sh disk`,
and deletes the image and its clone. Since 2026-10-28 (TOOL07, user
decision): the reference reproduced bit-identically in its own two runs
(`measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`) and in two runs of
this script with the dumper source of TOOL07's closing tables
(`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`), the last pass.
The destructive closing tables are no canary material: they change the image
by design. History: the first disk canary ran `bios-r1.tsv` without its
`Random` and `Gettime` lines on an image without files
(`measurements/measurement-2026-10-28-tool06/method/canary-disk.sh`,
`measurements/measurement-2026-10-28-tool06/RUNINFO.TXT`); it still passed on
2026-10-28 with the new dumper source.

## 7. Rules from here on

- New references are measured on the pin and go into **new files**;
  `EMUTOS14-*` stay untouched as the 1.4 evidence. The pin run of all 14
  tables is `tosdump/reference/EMUTOS-1a7484ab-*` (same suffixes as the
  `EMUTOS14-*` they re-measure).
- Moving the pin = rerun this calibration (`measurements/measurement-2026-10-26-oracle/method/run-set.sh` +
  `measurements/measurement-2026-10-26-oracle/method/compare-set.sh` of the run) and map every new difference to a commit.
- A change of Hatari, its config, or the toolchain = rerun the control and
  calibration runs first.
- The BIOS + XBIOS references `EMUTOS-1a7484ab-BIOS` (`tt`) and
  `EMUTOS-1a7484ab-BIOS-falcontc` were measured on this pin on 2026-10-27,
  in format 3 from the start, with `DUMPBIOS` build `a8b7` and the table
  `tosdump/cases/bios-r1.tsv` (CRC `583C11E0`, 147 cases); they have no
  `EMUTOS14-*` counterpart. Two `tt` runs are identical except the
  `Gettime` case (class `NONZERO`). Why the second profile: a third of the
  XBIOS answers is the machine's (TT shifter, Videl, DSP, DMA sound,
  palette) — `measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`.
- The disk-round reference `EMUTOS-1a7484ab-BIOS-r2-disk` was measured on
  this pin on 2026-10-28 with profile `disk`, `DUMPBIOS` build `b81d` and
  the table `tosdump/cases/bios-r2.tsv` (CRC `09C71B80`, 51 cases), on the
  test image `tools/mktestdisk.sh <img> NEWDESK.INF=<inf>` (the NEWDESK.INF
  NewTOS needs on C:; EmuTOS does not read it). Two runs byte-identical, the
  image unchanged by both
  (`measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`). The NewTOS
  side is a card from `tools/mkcard.sh` with `MKCARD_TESTDISK` = the same
  image, so both sides read the same bytes on C:.
- The closing tables' references `EMUTOS-1a7484ab-BIOS-r2-phys-disk`,
  `-r2-x004-disk`, `-r2-x005-disk` and `-r2-x006-disk` were measured on this
  pin on 2026-10-28 with profile `disk`, `DUMPBIOS` build `99ad` and the
  tables `tosdump/cases/bios-r2-phys.tsv` (CRC `514944FD`, 12 cases),
  `-x004.tsv` (`9F135A6F`, 10), `-x005.tsv` (`FBB54EFF`, 13), `-x006.tsv`
  (`E4B17B75`, 11), on the test image of `bios-r2.tsv`, each run on its own
  fresh clone. Two runs of each byte-identical, the end states of the two
  runs identical; x004 and x006 leave C:'s boot sector changed by design,
  x005 and phys leave the image unchanged
  (`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`).
- All fourteen VDI, AES and raster `EMUTOS-1a7484ab-*` references are record **format 3**
  (pointer width and word order in the header,
  `docs/TOS-REFERENCE-DUMPER.md` section 3), re-recorded on this pin on
  2026-10-26 with each reference's own build id. The twelve VDI and AES
  payloads are byte-identical to the format 2 recording
  (`measurements/measurement-2026-10-26-ref07-v3/RUNINFO.TXT`). The two
  `EMUTOS-1a7484ab-RASTER*` payloads cannot be: a rebuilt dumper puts its
  MFDB buffers elsewhere, so their pointer cells (address class) move with
  the dumper's text segment (same effect as the control run in §4). They
  were replaced under the user's condition "payload byte-identical except
  header and address cells", checked with the exclusion set derived from the
  field descriptors: 4 address cells per record (236 resp. 248 cells), every
  other payload byte equal
  (`measurements/measurement-2026-10-26-raster-v3/RUNINFO.TXT`). The format 2
  files stay in git history. `tosdiff` refuses to compare a format 3 dump
  with a format 2 one, so a NewTOS dump compared against a format 3
  reference has to be format 3 as well. `EMUTOS14-*` stay format 2.

## 8. Open for Phase 1

GEMDOS/BIOS/XBIOS return many addresses (`Malloc`, `Fgetdta`, `Logbase`,
`Getbpb`, `Iorec`, …). They need the **address** class to be **relational**,
compared that way, **never by value and never ignored**. Since record format 3
the class has its first relational property: every address cell, on each side
separately, must be `< 0x80000000` (a TOS address is a signed `LONG`,
`TASK/MEM11.md`); `tosdiff` reports a violation as its own verdict and a
format 2 dump as "not checkable" (`docs/TOS-REFERENCE-DUMPER.md` section 3,
field classes). A second relational property within one side is the raster
**kept check**: every MFDB pointer cell after the call equals the one before
it (the VDI must not modify the pointers), again its own verdict and "not
checkable" on format 2 (same section). Since 2026-10-27 the BIOS + XBIOS area
exists (`DUMPBIOS`, task T7): a returned pointer is class `PTR_RAM` —
non-null, even and `< 0x80000000` on each side
(`docs/TOS-REFERENCE-DUMPER.md` section 6; first a multiple of 4, which the
reference failed for WORD-aligned m68k structures). A relational rule is only
valid if the oracle passes it: `tosdiff --selfcheck` runs every one on each
pinned reference alone in the host tests, and a failure there is a rule bug
for the Researcher, never a NewTOS finding (same section, "The oracle
self-check"). Still open: the GEMDOS area. tos-api's `aes_reference` harness
checks `ap_3resv` relationally on its own (non-zero iff the reference's is,
`tos-api/tests/aes_reference/harness.rs`).

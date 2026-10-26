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
`tools/oracle/build-emutos.sh`, `tools/hatari-oracle.sh`, `tools/hatari-qed.sh`
and `tosdump/Makefile`. Nothing is taken from `$PATH`; `/opt/cross-mint` is
not on it.

## 2. Machine profiles (`tools/hatari-oracle.sh`)

Common to both: `HOME` = an empty directory (no user config, no user NVRAM —
Hatari starts with a fresh NVRAM), `--configfile tools/oracle/hatari.cfg`,
`SDL_VIDEODRIVER=dummy`, `--sound off --fast-forward on --conout 2`,
GEMDOS drive C: = a host directory holding the dumper and its table,
`--auto C:\DUMP*.PRG`. The run ends when the dumper's final LOG is on the
drive; Hatari is stopped with SIGKILL (SIGTERM opens the "really quit?"
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
| `tt` | `--machine tt --tos-res ttmed --monitor vga` | 640×480, 4 planes (TT-Medium) | `EMUTOS14-VDI`, `-VDI-r2`, `-VDI-r3`, `-VDI-all`, `-AES`, `-AES-r2wind`, `-AES-r2brlist`, `-AES-r3`, `-AES-m05/m06/m07`, `-RASTER` |
| `falcontc` | `--machine falcon --monitor vga` + `EMUDESK.INF` `#E 1A E0 00 1C 60` | 640×480, 16-bit truecolor (VIDEL `0x001C`, read by `process_inf1`, `aes/geminit.c`) | `EMUTOS14-VDI-all-falcontc`, `-RASTER-screen-falcontc` |

The INF is byte-identical to `measurements/measurement-2026-09-21-vdi-qemu/method/EMUDESK-falcon-truecolor.INF`.

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
   (`method/check-translations.py` of the run). The translations of the
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

`tools/hatari-oracle.sh` with its default ROM (the pin, sha256 checked by the
script) on `vdi-all.tsv`: `tt` vs `EMUTOS14-VDI-all` and `falcontc` vs
`EMUTOS14-VDI-all-falcontc` — **BIN and LOG bit-identical** (182 cases each),
with the user's full configuration and again with the cleaned `hatari.cfg`.

## 7. Rules from here on

- New references are measured on the pin and go into **new files**;
  `EMUTOS14-*` stay untouched as the 1.4 evidence. The pin run of all 14
  tables is `tosdump/reference/EMUTOS-1a7484ab-*` (same suffixes as the
  `EMUTOS14-*` they re-measure).
- Moving the pin = rerun this calibration (`method/run-set.sh` +
  `method/compare-set.sh` of the run) and map every new difference to a commit.
- A change of Hatari, its config, or the toolchain = rerun the control and
  calibration runs first.

## 8. Open for Phase 1

GEMDOS/BIOS/XBIOS return many addresses (`Malloc`, `Fgetdta`, `Logbase`,
`Getbpb`, `Iorec`, …). They need the **address** class to become
**relational** — null/non-null, alignment — compared that way, **never by
value and never ignored**. Today `TD_FIELD_ADDRESS` means "recorded, not
compared" (tosdiff); tos-api's `aes_reference` harness already checks
`ap_3resv` relationally (non-zero iff the reference's is,
`tos-api/tests/aes_reference/harness.rs`).

# BIOS Subsystem State

> Phase 00 deliverable D7 (BIOS). The group-level catalog of everything the
> BIOS **makes available to programs**: system variables (with their fixed TOS
> addresses), constants, error codes, and shared data-structure layouts. So it
> is not repeated in every function entry.
> Source: EmuTOS @ `1a7484ab`. Addresses from `tosvars.ld`.
>
> **§1's table below is a curated subset** (the sysvars this function group's
> 12 traps actually touch). For the **complete** `tosvars.ld` coverage
> (`$380`–`$5B4`, every symbol, with its boot-time initialiser and a
> "no active EmuTOS backing" flag for pure address reservations) plus the
> **boot-time population order** (cold- vs. warm-boot sysvar-clear
> behaviour, the OSHEADER/`_sysbase`/cookie-jar init sequence), see
> `docs/api/bios/_boot-init.md` (added 2026-07-23, BB1 pass).

> **Observable vs. internal** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract** (programs observe — keep value/semantics exact): the
> character-device vector tables and their handle mapping, the `hdv_*` disk
> vectors, `drvbits`, `hz_200`, `shifty`, media-change return values, the BPB
> layout, the VT52 console behaviour. **Free to reimplement** (unobservable):
> `boot_status` flags, how tables are stored, `kprintf` routing, the m68k
> physical addresses themselves (native NewTOS reaches these by name/call, not
> by poking low memory — see COMPATIBILITY "System variables — compatible how").

---

## 1. System variables (fixed TOS low-memory addresses)

All addresses are the classic m68k low-memory slots (`tosvars.ld`). For native
AArch64 ports these live in a kernel struct reached via the BIOS calls; the
addresses matter only for the Phase-10 m68k emulator.

| Address | Symbol           | Type            | Meaning | Used by |
|--------:|------------------|-----------------|---------|---------|
| `0x432` | `membot`         | `LONG`          | bottom of free ST-RAM (TPA) | `Getmpb` |
| `0x436` | `memtop`         | `LONG`          | top of free ST-RAM | `Getmpb` |
| `0x42e` | `phystop`        | `LONG`          | top of physical ST-RAM | `bmem_init` |
| `0x43a` | `memval2`        | `LONG`          | validates memcntlr/memconf if `0x237698aa` (unrelated to `phystop`; see correction below) | boot RAM-config check |
| `0x454` | `nvbls`          | `WORD`          | number of VBL routines | VBL |
| `0x456` | `vblqueue`       | `LONG` (ptr)    | pointer to the VBL routine list | VBL |
| `0x46e` | `swv_vec`        | `LONG` (vector) | resolution-change vector | screen |
| `0x472` | `hdv_bpb`        | `LONG` (vector) | Get-BPB routine → `blkdev_getbpb` | `Getbpb` |
| `0x476` | `hdv_rw`         | `LONG` (vector) | read/write routine → `blkdev_rwabs` | `Rwabs` |
| `0x47a` | `hdv_boot`       | `LONG` (vector) | hard-disk boot routine | boot |
| `0x47e` | `hdv_mediach`    | `LONG` (vector) | media-change routine → `blkdev_mediach` | `Mediach` |
| `0x484` | `conterm`        | `UBYTE`         | console-output attribute bits (bit0 keyclick, bit1 key-repeat, bit2 bell, **bit3 = fold `shifty` into `Bconin(2)`'s high byte**) | `Bconin` (handle 2) |
| `0x48e` | `themd`          | `MD` (16 bytes) | the single static memory descriptor `Getmpb` points `mp_mfl` at | `Getmpb` |
| `0x4a2` | `savptr`         | `LONG` (ptr)    | BIOS register save-area stack pointer | trap trampoline |
| `0x4ba` | `hz_200`         | `LONG` (volatile) | 200 Hz free-running system clock counter | timing |
| `0x4c2` | `drvbits`        | `LONG`          | logical-drive presence bitmap | `Drvmap` |
| `0x4ee` | `dumpflg`        | `WORD`          | screen-dump flag | `Scrdmp` |
| `0x51e` | `bconstat_vec[8]`| 8×`LONG` (ptr)  | input-status routines | `Bconstat` |
| `0x53e` | `bconin_vec[8]`  | 8×`LONG` (ptr)  | input routines | `Bconin` |
| `0x55e` | `bcostat_vec[8]` | 8×`LONG` (ptr)  | output-status routines | `Bcostat` |
| `0x57e` | `bconout_vec[8]` | 8×`LONG` (ptr)  | output routines | `Bconout` |
| —       | `shifty`         | `UBYTE`         | keyboard modifier state (via `os_kbshift`) | `Kbshift` |

> **Corrections vs. earlier drafts:** `hdv_rw` is at **`0x476`** (earlier
> `04-rwabs.md` said `0x47E`); `hdv_mediach` is at **`0x47e`** (earlier
> `09-mediach.md` said `0x47A`, which is actually `hdv_boot`). `savptr` is at
> `0x4a2`, `drvbits` at `0x4c2`.
>
> **New correction (this pass, 2026-07):** `phystop` is at **`0x42e`**, not
> `0x43a` as previously listed here. The wrong value came from a stale comment
> in `bios/biosmem.c:49` ("`LONG phystop TOS variable in 0x43a`") — the
> authoritative source is the linker script `tosvars.ld:42`
> (`_phystop = 0x42e`), which the C compiler's `_`-prefix convention ties
> directly to the `phystop` global (`include/tosvars.h:49`). `0x43a` is
> actually `memval2` (`tosvars.ld:45`), an unrelated RAM-config validity
> marker — not read by any function in this group. CLAUDE.md Rule 1 applies
> literally here: the comment lied, the linker script (the actual link-time
> binding) is authoritative. `00-getmpb.md` itself never cited the address
> directly so it needed no fix, only this table.
>
> **Gap filled (this pass):** `conterm` (`0x484`) was missing from this table
> entirely, even though bit 3 changes `Bconin`'s (handle 2) return-value
> contract — see `02-bconin.md` for the corrected encoding. `themd`'s fixed
> address (`0x48e`) was also missing; it is the classic documented "first
> memory descriptor block" sysvar and is exactly the `MD` `Getmpb` points
> `mp_mfl` at (`bios/biosmem.c:144-147`, `tosvars.ld:73`).

The GEM/AES exception hooks live at the very start of the sysvar area
(`tosvars.ld:31-36`) and are installed via `Setexc(num, …)` where `addr =
4*num`:

| Address | Symbol       | `Setexc` num | Meaning |
|--------:|--------------|-------------:|---------|
| `0x400` | `etv_timer`  | `0x100`      | GEM event-timer vector |
| `0x404` | `etv_critic` | `0x101`      | GEM critical-error handler |
| `0x408` | `etv_term`   | `0x102`      | GEM program-termination vector |
| `0x40c` | `etv_xtra`   | `0x103`      | GEM extra vectors (unused) |

---

## 2. `boot_status` flags (`bios/bios.h:84-89`) — internal

```c
#define DEVICES_INITIALIZED 0x01
#define MACHINE_INITIALIZED 0x02
#define DOS_AVAILABLE       0x04
#define LINEA_AVAILABLE     0x08
#define CHARDEV_AVAILABLE   0x10
#define RAM_VALIDATED       0x20
#define TPA_INITIALIZED     0x40
#define DRIVER_LOADED       0x80
```

`CHARDEV_AVAILABLE` is set by `bios/bios.c:430` after `chardev_init()`. Until
then, all four character-device BIOS calls return `0L`. This flag is a NewTOS
*free* item (unobservable to normal programs) but NewTOS needs an equivalent
guard so early `kprintf`/panic output routes to the debug UART, not the console
layer.

---

## 3. Character-device vector tables (`bios/chardev.c:30-45`)

Four 8-entry function-pointer tables at the fixed addresses above. Initial
wiring (`bios/chardev.c`, verified line-by-line):

| Handle | `bconstat_vec` | `bconin_vec` | `bcostat_vec`     | `bconout_vec`   |
|-------:|----------------|--------------|-------------------|-----------------|
| 0      | `char_dummy`   | `bconin0` (stub→0) | `bcostat0`  | `bconout0`      |
| 1      | `bconstat1`    | `bconin1`    | `bcostat1`        | `bconout1`      |
| 2      | `bconstat2`    | `bconin2`    | `bcostat2`        | `bconout2`      |
| 3      | `bconstat3`    | `bconin3`    | **`bcostat4`** \* | `bconout3`      |
| 4      | `char_dummy`   | `char_dummy` | **`bcostat3`** \* | `bconout4`      |
| 5      | `char_dummy`   | `char_dummy` | `char_dummy`      | `bconout5`      |
| 6      | `char_dummy`   | `char_dummy` | `char_dummy`      | `charout_dummy` |
| 7      | `char_dummy`   | `char_dummy` | `char_dummy`      | `charout_dummy` |

\* MIDI (handle 3/`bcostat3`) and IKBD (handle 4/`bcostat4`) **output-status**
slots are swapped on purpose: `/* note that IKBD and MIDI bcostat() are
swapped! */` (`bios/chardev.c:40`). Applications depend on this. The `bconout`
and `bconin`/`bconstat` tables are **not** swapped.

Notes on the default handlers (all Atari-hardware / `CONF_*` gated —
*behaviour reference*, not contract):
- `char_dummy` / `charout_dummy` → `0L` (`bios/chardev.c:50,55`).
- `bconin0` is a **stub** returning 0 (`bios/parport.c:123`, not "reads the
  port").
- `bconstat2`/`bconin2` operate on the IKBD IOREC queue (`bios/ikbd.c:235,250`).
- `bconout2` → `cputc` returns `1L`; `bconout5` → `ascii_out` (no VT52) returns
  `1L`; `bcostat2` returns `-1` (screen always ready).
- `bconout0` returns `0L` on printer-busy/timeout; all other output handlers
  return `1L` on success.

The tables sit at fixed addresses so programs can overwrite a slot to install a
driver — contract for m68k binaries (Phase 10); native ports use a
NewTOS-native registration.

---

## 4. Auxiliary hooks (`bios/chardev.c:78-84`)

```c
aux_stat = just_rts;  aux_vec = just_rts;
prt_stat = just_rts;  prt_vec = just_rts;
dump_vec = just_rts;
```

Five indirect-call slots for old-style printer/serial filters and `Scrdmp`
(XBIOS 0x14). Default `just_rts` (a no-op). Replacing these is the documented
extension mechanism for printer drivers.

---

## 5. `bconmap_root` (gated by `BCONMAP_AVAILABLE`)

Per-handle remap for handles ≥ `BCONMAP_START_HANDLE` (canonically 6). Used by
`Bconmap` (XBIOS 0x2C) to switch which hardware port answers handles 6/7. Each
`MAPTAB` entry holds the four function pointers plus IOREC pointers.
NewTOS scope: deferred; early phases expose only handles 0–5.
**[XBIOS05]** `bconmap_root` exists since XBIOS05 with one entry (device 6,
`docs/api/xbios/2c-bconmap.md`). **[BIOS08]** The BIOS character calls take
handles `>= 6` from it, as EmuTOS does: 6 = entry 0 = the AUX port (handle 1's
entry), 7 and up `0`; the device table holds 0..5
(`docs/api/bios/{01,02,03,08}-*.md`, BIOS08 notes).

---

## 6. Disk vectors and block-device state

- `hdv_bpb` / `hdv_rw` / `hdv_mediach` (addresses in §1) are installed by
  `blkdev_init()` (`bios/blkdev.c:101-103`) to `blkdev_getbpb`/`blkdev_rwabs`/
  `blkdev_mediach`. `Getbpb`/`Mediach` call through them via `protect_w`;
  `Rwabs` via `protect_wlwwwl`. (`Drvmap` does **not** use a vector — it reads
  `drvbits` directly.)
- Per-drive state lives in `blkdev[]` (`BLKDEV`) and per-unit state in
  `units[]` — internal (free to reimplement). Fields a program can observe
  only indirectly: `mediachange` (latched, read by `Mediach`, cleared by
  `Getbpb`), `forcechange` (armed by a logical write to sector 0), and the
  embedded `bpb` returned by `Getbpb`.

---

## 7. Constants a program sees

### Media-change return values (`bios/disk.h:69-71`)
| Value | Constant | Meaning |
|------:|----------|---------|
| 0 | `MEDIANOCHANGE` | definitely not changed |
| 1 | `MEDIAMAYCHANGE` | may have changed |
| 2 | `MEDIACHANGE` | definitely changed |

### `Rwabs` `r_w` flags (`bios/disk.h:53-61`)
| Bit | Mask | Constant | Meaning |
|----:|-----:|----------|---------|
| 0 | 0x01 | `RW_WRITE`/`RW_RW` | write (vs read) |
| 1 | 0x02 | `RW_NOMEDIACH` | skip media-change check |
| 2 | 0x04 | `RW_NORETRIES` | no auto-retry |
| 3 | 0x08 | `RW_NOTRANSLATE` | physical (unit) mode |
| 7 | 0x80 | `RW_NOBYTESWAP` | skip byte-swap (physical only) |

### BPB flags (`include/biosdefs.h:84-85`)
| Bit | Mask | Constant | Meaning |
|----:|-----:|----------|---------|
| 0 | 0x01 | `B_16`   | 16-bit FAT (FAT16) |
| 1 | 0x02 | `B_1FAT` | single FAT |

FAT-type thresholds (`bios/blkdev.h:25-26`): `MAX_FAT12_CLUSTERS = 4084`,
`MAX_FAT16_CLUSTERS = 65524`.

### `Kbshift` modifier bits (`include/biosdefs.h:90-100`)
See `0b-kbshift.md` (bits 0–6: RSHIFT, LSHIFT, CTRL, ALT, CAPS, HOME, INSERT;
`MODE_SCA = 0x0F`).

### BIOS/GEMDOS error codes seen from this group (`include/gemerror.h`)
| Code | Value | Where |
|------|------:|-------|
| `E_OK`   | 0   | `Rwabs` success |
| `ESECNF` | -8  | `Rwabs` bad range / invalid BPB |
| `EWRITF` | -10 | `Rwabs` write fault (driver) |
| `EREADF` | -11 | `Rwabs` read fault (driver) |
| `EWRPRO` | -13 | `Rwabs` write-protect (driver) |
| `E_CHNG` | -14 | `Rwabs` media change |
| `EUNDEV` | -15 | `Rwabs`/`Mediach` unknown device |

(`Getbpb` returns only NULL/`BPB *`, never an error code; the char-device
calls never return error codes, only `0`/`-1`.)

---

## 8. Shared data-structure layouts

- **BPB** (`include/biosdefs.h:67`, 18 bytes, 9×UWORD): returned by `Getbpb`.
  Full layout in `docs/api/structures/bpb.md`.
- **MD** (`include/bdosdefs.h:108`, 16 bytes m68k): `m_link`, `m_start`,
  `m_length`, `m_own`. Written by `Getmpb`. See `docs/api/structures/md.md`.
- **MPB** (`include/bdosdefs.h:120`, "Memory _Partition_ Block", 12 bytes m68k):
  `mp_mfl`, `mp_mal`, `mp_rover` (last "no longer used"). Filled by `Getmpb`
  (which writes only `mp_mfl`/`mp_mal`).
- **OSHEADER** `os_kbshift` (`include/biosdefs.h:59`) aliases `&shifty`.

---

## 9. VT52 console state (`bios/conout.c`, `lineavars`)

The screen handler (handle 2) keeps cursor position (`v_cur_cx`/`v_cur_cy`), cursor cell
offset (`v_cur_of`), VT52 escape-parser state (`con_state` @ `0x4a8`, `save_row` @ `0x4ac`;
corrected CON04 2026-10-22, full spec: [`_vt52.md`](_vt52.md)),
and fg/bg colour (Line-A vars). Applications read these directly; the full
layout is part of the Line-A block (`docs/api/_line-a.md`). Contract:
observable VT52 *behaviour*; the byte layout is preserved for m68k binaries.

---

## 10. `kprintf()` early routing (`bios/kprint.c`)

Before `CHARDEV_AVAILABLE`, `kprintf` routes to the native debug path
(NatFeats / framebuffer / serial); after, to `bconout(2, …)`. NewTOS analogue:
early panic/boot logs via the dedicated PL011 debug UART, switching to the
console only after the NewTOS `chardev_init` equivalent.

---

## Pi / BCM2711 binding

> Re-points the system-variable *storage* from **fixed m68k low-memory
> addresses** (`tosvars.ld`) to **kernel structs reached by call**. The
> observable *values/semantics* are the contract and are unchanged; the
> addresses matter only for the deferred Phase-10 m68k emulator. See
> `docs/HARDWARE-PROFILE.md` §3.8.

### Where the system-variable values actually live on the Pi

No Atari low-memory address is mapped on AArch64 (same decision as the
`$420`/`$432` vars — direct poke is m68k-emulator-only). The documented globals
keep their **values/semantics** but live in kernel state, accessed via the
BIOS/XBIOS calls, not a poke:

| Sysvar (contract) | Pi backing | Access | Source |
|-------------------|------------|--------|--------|
| `drvbits` (`0x4c2`) | mounted-partition bitmap from the SD MBR | `Drvmap` (0x0A) | `tos-api/src/bios/drive.rs` (see `0a-drvmap.md`) |
| `hz_200` (`0x4ba`) | **live 200 Hz counter** — ARM generic timer IRQ (PPI INTID 30) advances it and mirrors it into the running process's `$4BA` | direct read at `$4BA` from EL0 (mintlib `get_sysvar(_hz_200)`); `bios::tick::hz_200()` in-kernel | `kernel/src/drivers/timer.rs`, `tos-api/src/bios/tick.rs` (see `06-tickcal.md`) |
| `etv_timer/critic/term` (`$0400/04/08`) | one cell for every process, `VECTORS[0x100/0x101/0x102]`, carried in every page 0 at `$400/$404/$408` [BIOS07] | `Setexc` (0x05); read in page 0 | `tos-api/src/bios/vectors.rs:90@VECTORS`, `:39@ETV_TIMER` (see `05-setexc.md`) [BIOS13] |
| `shifty` / `os_kbshift` | IKBD-decoder modifier byte from USB-HID | `host().keyboard_shift_state()` (EL1); `Kbshift` trap **not exposed** | `kernel/src/ikbd.rs`, `host_env.rs:231-233` (see `0b-kbshift.md`) |
| `membot`/`memtop`/`phystop` | ST-RAM pool extent from the AArch64 memory map | (allocator internal; `Getmpb` vestigial) | `kernel/src/bios/machine/layout.rs` (see `00-getmpb.md`) |
| `bconstat/in/out/cost_vec[8]` | kernel char-device dispatch, not a poked table | BIOS char calls | `tos-api/src/bios/chardev.rs` (see `_state.md §3`) |
| `hdv_bpb/rw/mediach/boot` | direct kernel calls into the block layer | `Getbpb`/`Rwabs`/`Mediach` | `tos-api/src/bios/` (see block specs) |
| `dumpflg` (`0x4ee`) | no screen-dump path on the Pi (`Scrdmp` unwired, S2) | — | — |

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| Program pokes `4*num` / `$4c2` / `$4ba` directly in low RAM | reached by name/call; no address is exposed to EL0 |
| Overwriting a `bcon*_vec` slot installs a driver | native char-device registration (deferred); table not at a fixed VA |
| `hz_200` free-running at 200 Hz | free-running at 200 Hz, driven by a real periodic interrupt; readable at its classic address `$4BA` in the process's own low memory |

### Gaps vs. the contract (Step-2 candidates)

- ~~**`hz_200` has no incrementing counter**~~ — **closed 2026-09-04**: a
  200 Hz timer interrupt now advances it and publishes it at `$4BA`
  (`06-tickcal.md` §"The 200 Hz tick"). What remains open is the rest of the
  **50 Hz half** of EmuTOS' timer-C sieve (`bios/vectors.S:521`): `dosound`
  and the `etv_timer` dispatch are still not run. Key repeat runs since
  2026-10-03 (KEYS01) — polled in the input pump, not in the interrupt; see
  `docs/api/xbios/_state.md` §"Key-repeat handler".
- **`etv_*` hooks stored but never invoked** (see `05-setexc.md`).
- **`dumpflg` / `Scrdmp` unwired** (Screen S2) — nothing resets the flag because
  the call does not exist.
- **char-device vector tables are not poke-installable** — native driver
  registration is deferred; only the value-level contract is honoured today.

## Cross-references

- `docs/api/bios/_dispatcher.md` — trap-13 trampoline + `savptr` save area.
- `docs/api/bios/00-getmpb.md` … `0b-kbshift.md` — per-function entries.
- `docs/api/structures/bpb.md`, `md.md` — struct layouts.
- `docs/api/_dependency-graph.md` — implementation order.

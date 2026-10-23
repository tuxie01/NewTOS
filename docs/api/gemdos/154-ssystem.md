# `Ssystem` (GEMDOS 0x154 — MiNT extension)

> **FreeMiNT extension spec** — `Ssystem` is **not implemented by EmuTOS**
> (the EmuTOS GEMDOS table tops out at `MAX_FNCALL = 0x57`,
> `bdos/bdosmain.c`). For this function **FreeMiNT is the source of truth**
> (Track A source-hierarchy 2b, `docs/SOURCE-ANALYSIS-PROTOCOL.md`).
> Source: FreeMiNT @ `f906416` —
> `sys/syscall_vectors.c:473` (trap slot `0x154 = sys_s_system`),
> `sys/ssystem.c:57` (`sys_s_system`), `sys/mint/ssystem.h` (mode codes),
> `sys/cookie.c` (`get_cookie`/`set_cookie`/`del_cookie`).
> Adopted per ADR-012 (`docs/MEMORY.md`) as NewTOS's cookie-jar access path;
> data model: [`../bios/_cookie-jar.md`](../bios/_cookie-jar.md).
>
> **Scope of this spec:** the `Ssystem` trap shell + the three **cookie**
> subfunctions (`S_GETCOOKIE`/`S_SETCOOKIE`/`S_DELCOOKIE`) that ADR-012 needs.
> FreeMiNT defines ~40 other modes (`S_OSNAME`, `S_OSVERSION`, `S_GETLVAL`,
> `S_SETLVAL`, `S_SECLEVEL`, …) — catalogued below for completeness but marked
> **deferred**; the dispatcher returns `ENOSYS` for every unimplemented mode so
> the surface stays honest.
>
> **Plus NewTOS's own modes from 2000 upwards** (§"NewTOS-own modes (2000+)"):
> machine facts an EL0 program cannot reach any other way, because the hardware
> that knows them is kernel-only (2000/2001), the kernel entry of the
> default `kbdvec` stub (2002, KEYS02), and the effective system settings a
> system-information tool shows (2003–2020, SYSINFO01 / KEYOWNER), and the
kernel entry of the OSHEADER reset stub (2021, RST01), and the USB snapshot (2022, USB01). These are *not* FreeMiNT modes and are
> marked as ours wherever they appear; 2000+ is a range FreeMiNT leaves free.

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract (for the modes NewTOS implements): trap ID `0x154`, the `(mode,
> arg1, arg2)` ABI, the mode-code numbers, the cookie sub-op semantics
> (return-by-value-or-pointer, tag validation, not-found error). Reference:
> FreeMiNT's `*CJAR` sysvar, its `struct cookie` internals, its m68k
> low-memory `S_*VAL` addresses — NewTOS reimplements the *behaviour*, not
> these internals.

## EmuTOS status

Not applicable — EmuTOS has no `Ssystem`. Do **not** search EmuTOS for it; the
authority here is FreeMiNT (confirmed: `sys_s_system` sits at trap index
`0x154` in `sys/syscall_vectors.c:473`).

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** `0x0154` (340 decimal)
- **Signature (FreeMiNT):** `long _cdecl s_system(int mode, ulong arg1,
  ulong arg2)` (`sys/ssystem.c:56-57`).
- **Stack frame on entry (m68k):**
  ```
  +8: arg2 (LONG)
  +4: arg1 (LONG)
  +2: mode (word)
   0: function_id (word) = 0x0154
  ```
- **Total parameter bytes:** 10 (WORD + LONG + LONG).
- **Native (AArch64) ABI:** ID `0x154` in `x8`; `mode` in `x0`, `arg1` in
  `x1`, `arg2` in `x2`; LONG result **sign-extended** into `x0`. When a mode
  returns *by pointer* (e.g. `S_GETCOOKIE` with `arg2 != 0`), `arg2` is a
  user-space pointer to a `LONG` the kernel writes; the LONG result is the
  status.

  | m68k stack | AArch64 reg | Name   | Type / size |
  |------------|-------------|--------|-------------|
  | +2         | `x0`        | `mode` | `WORD` (16) — dispatch selector |
  | +4         | `x1`        | `arg1` | `LONG` (32) — mode-specific |
  | +8         | `x2`        | `arg2` | `LONG` (32) — mode-specific |

**Dispatch note (NewTOS):** `0x154` is far above the EmuTOS-derived GEMDOS
`MAX_FN = 0x57`, so the static dispatch table cannot hold it. The GEMDOS
dispatcher special-cases `func_id == 0x154` **before** the table lookup (a
small `match` arm that routes to the Ssystem handler). Future MiNT extensions
follow the same "high id, special-cased" pattern. See
`docs/api/gemdos/_dispatcher.md`.

---

## Mode codes (FreeMiNT `sys/mint/ssystem.h`)

### Implemented by NewTOS (cookie subset — in scope)

| Mode          | Value | Meaning                    | Privilege |
|---------------|------:|----------------------------|-----------|
| `S_GETCOOKIE` |     8 | read a cookie              | any       |
| `S_SETCOOKIE` |     9 | add / update a cookie      | root only |
| `S_DELCOOKIE` |    26 | remove a cookie            | root only |

`mode == -1` in FreeMiNT is a "does this call exist" probe. `sys_s_system`
pre-initializes `register long r = E_OK;` (`sys/ssystem.c:66`) before the
`switch (mode)` (`sys/ssystem.c:70`); `case -1:` (`sys/ssystem.c:72-76`) only
does a `TRACE()` and `break`s — it never assigns to `r`. Execution falls
through to `return r;` (`sys/ssystem.c:651-652`) with `r` still `E_OK`.
Confirmed by direct line-by-line re-read (D-072): `Ssystem(-1, arg1, arg2)`
returns `E_OK` (0) **unconditionally**, independent of `arg1`/`arg2`. NewTOS
must honour it so callers can feature-test.

Any other mode → `ENOSYS` (`sys/ssystem.c:642-648`). In NewTOS this maps to
`EINVFN` (`-32`, numerically identical to FreeMiNT's `ENOSYS`) — consistent
with how the GEMDOS dispatcher reports unknown function IDs.

### Defined by FreeMiNT but DEFERRED (out of scope for now)

Full list from `sys/mint/ssystem.h`, so the "unknown mode → error" surface is
deliberate rather than accidental:

| Mode | Value | Group | Notes |
|------|------:|-------|-------|
| `S_OSNAME` | 0 | OS info | returns `'MiNT'` |
| `S_OSXNAME` | 1 | OS info | returns `'Free'` |
| `S_OSVERSION` | 2 | OS info | kernel version |
| `S_OSHEADER` | 3 | OS info | reads the m68k OS header (low-mem, m68k-only) |
| `S_OSBUILDDATE` | 4 | OS info | |
| `S_OSBUILDTIME` | 5 | OS info | |
| `S_OSCOMPILE` | 6 | OS info | CPU the kernel was built for |
| `S_OSFEATURES` | 7 | OS info | memory-protection / VM / crypto bitfield |
| `S_GETLVAL` | 10 | sysvar peek | read LONG at `arg1` (low-mem `$420..$fffc`) |
| `S_GETWVAL` | 11 | sysvar peek | read WORD |
| `S_GETBVAL` | 12 | sysvar peek | read BYTE |
| `S_SETLVAL` | 13 | sysvar poke | write LONG (root); low-mem addresses |
| `S_SETWVAL` | 14 | sysvar poke | write WORD (root) |
| `S_SETBVAL` | 15 | sysvar poke | write BYTE (root) |
| `S_SECLEVEL` | 16 | security | get/set secure mode (root) |
| `S_RUNLEVEL` | 17 | — | reserved / disabled in source |
| `S_TSLICE` | 18 | scheduler | time-slice (root) |
| `S_FASTLOAD` | 19 | loader | force fastload (root) |
| `S_SYNCTIME` | 20 | clock | RTC sync interval (root) |
| `S_BLOCKCACHE` | 21 | FS cache | cache % (root) |
| `S_FLUSHCACHE` | 22 | CPU cache | `cpushi` |
| `S_CTRLCACHE` | 23 | CPU cache | CPU-specific |
| `S_INITIALTPA` | 24 | loader | initial TPA size (root) |
| `S_CTRLALTDEL` | 25 | keyboard | Ctrl-Alt-Del behaviour (root) |
| `S_LOADKBD` | 27 | keyboard | reload keyboard table (root) |
| `S_SETEXC` | 28 | security | gate `Setexc` for non-kernel procs (root) |
| `S_GETBOOTLOG` | 29 | debug | copy boot-log path (root) |
| `S_CLOCKUTC` | 100 | clock | RTC-in-UTC flag |
| `S_TIOCMGET` | 0x54f8 | serial | MiNTLib hack |
| `S_KNAME` | 900 | info | kernel name string |
| `S_CNAME`/`S_CVERSION`/`S_CDEFINES`/`S_COPTIM` | 910-913 | info | compiler strings |
| `S_DEBUGLEVEL`/`S_DEBUGDEVICE`/`S_DEBUGKMTRACE` | 1000/1001/1100 | debug | |

The `S_*VAL` peek/poke modes read/write **m68k low memory** (`arg1` in
`$420..$ffff`, `sys/ssystem.c:189-272`) — meaningless for native AArch64 code
(no low-memory map, same reason the cookie jar dropped `$5A0`). If NewTOS ever
implements them it must be for the m68k emulator only, or re-mapped to the
kernel's sysvar struct. Several `S_OS*` and info modes could later expose a
NewTOS identity, but that is a separate decision.

### NewTOS-own modes (2000+) — **implemented**

| Mode | Value | Group | Notes |
|------|------:|-------|-------|
| `S_NEWTOS_CPUCLOCK` | 2000 | machine | ARM core clock, **Hz** |
| `S_NEWTOS_CPUTEMP` | 2001 | machine | SoC die temperature, **1/1000 °C** |
| `S_NEWTOS_KBDVEC` | 2002 | keyboard | run the default `kbdvec` (EmuTOS `kbd_int`) for one IKBD key byte — KEYS02, see below |
| `S_NEWTOS_CPUMEAS` | 2003 | machine | ARM core clock **measured** by the firmware, Hz — SYSINFO01, see §"System-information modes" |
| `S_NEWTOS_CPUPOLICY` | 2004 | machine | ARM clock policy in effect (firmware / fixed at maximum) |
| `S_NEWTOS_MEMPROT` | 2005 | kernel | memory protection in effect (off / on / strict) |
| `S_NEWTOS_ASID` | 2006 | kernel | address-space IDs in effect (off / on / test) |
| `S_NEWTOS_SDMODE` | 2007 | storage | SD transfer mode in effect (fast / compatibility) |
| `S_NEWTOS_SDBUS` | 2008 | storage | SD data-bus width negotiated with the card, bits |
| `S_NEWTOS_BCACHE` | 2009 | storage | block read cache size, KiB (0 = off) |
| `S_NEWTOS_BCACHEPCT` | 2010 | storage | block cache share one linear read may fill, percent |
| `S_NEWTOS_STRAM` | 2011 | memory | ST-RAM pool size, KiB |
| `S_NEWTOS_TTRAM` | 2012 | memory | TT-RAM pool size, KiB (0 = none) |
| `S_NEWTOS_SCRWIDTH` | 2013 | display | framebuffer width, pixels |
| `S_NEWTOS_SCRHEIGHT` | 2014 | display | framebuffer height, pixels |
| `S_NEWTOS_SCRDEPTH` | 2015 | display | framebuffer depth, bits per pixel |
| `S_NEWTOS_GPUMEM` | 2016 | display | VideoCore (GPU) memory, KiB |
| `S_NEWTOS_KEYBOARD` | 2017 | input | source of the active keyboard layout |
| `S_NEWTOS_SERIAL` | 2018 | console | serial console output on / off |
| `S_NEWTOS_KVERSION` | 2019 | kernel | kernel version + build stamp, **string** into the caller's buffer |
| `S_NEWTOS_INPUTMODE` | 2020 | input | where the keyboard/mouse pump runs (timer pump / `input=aes`) — KEYOWNER |
| `S_NEWTOS_RESET` | 2021 | kernel | restart the machine — the kernel entry of the OSHEADER reset stub (RST01), see below |

### NewTOS-own mode 2022 — the USB snapshot (USB01 step 2)

Approved 2026-10-23, **built 2026-10-23** (kernel part, `S_NEWTOS_USBSNAP` in
`tos-api/src/gemdos/ssystem.rs`, store and encoder in
`kernel/src/drivers/usb/snapshot.rs`). The first free number is now **2023**;
`tos-api/tests/gemdos_ssystem_sysinfo.rs` and `gemdos_ssystem_cpuinfo.rs` pin it.

| Mode | Value | Group | Notes |
|------|------:|-------|-------|
| `S_NEWTOS_USBSNAP` | 2022 | input / USB | (USB01 step 2): snapshot of the enumerated USB devices into a caller buffer — device list with hub/port route, speed, VID/PID, chosen driver role + rejection reason, raw device/configuration/HID-report descriptors, per-device ring of raw interrupt reports, optional timed mute of the input decoder. Versioned little-endian buffer, `Ssystem(2022, buf, len)` with a 32-byte request header at `buf`; `E_OK` / `ERROR` (no USB controller, e.g. `raspi4b`) / `EINVAL`. Full spec: [`../usb/usb-snapshot.md`](../usb/usb-snapshot.md) |

**Why the number block is free.** FreeMiNT's `sys/mint/ssystem.h` (@ `f906416`)
assigns 0–29, 100 and `0x54f8` (21752), and reserves two ranges by comment:
*"additional informations about the kernel / reserved 900 - 999"* and *"debug
section / reserved 1000 - 1999"*. The highest mode its dispatcher answers at
all is `case 3000:` (`sys/ssystem.c:631`), a `register_trap2` test hook behind
`#ifdef MINT_STATUS_CVS` and marked *"XXX only for testing"*. **2000…2999 is
claimed by nobody**, so NewTOS takes it. (Note for a future extension: do not
grow past 2999 without re-checking 3000.)

**Why `Ssystem` and not a cookie.** A cookie is a value deposited once at boot;
a die temperature changes between two reads, and the ARM clock changes with
load and thermal state. The jar cannot express "ask the machine now"
(`docs/api/bios/_cookie-jar.md`), `Ssystem` can — and FreeMiNT-compatible
extension is the sanctioned direction (ADR-012, `docs/MEMORY.md`).

#### `S_NEWTOS_KBDVEC` (2002) — the default `kbdvec` (KEYS02)

```
Ssystem(2002, scancode, 0)
```

The kernel entry of the `kbdvec_default` stub every process finds in its own
IKBD vector table (`docs/api/xbios/22-kbdvbase.md` B3/C2, decision D6). It runs
EmuTOS' default keyboard handler, `_kbdvec` → `kbd_int(scancode)`
(`bios/aciavecs.S:372-376`, `bios/ikbd.c:725-910`), on the kernel's decoder:
modifier state, the key word into the console queue, the key repeat. A program
reaches it by chaining to the old `kbdvec` (`br old`), not by calling `Ssystem`
itself — but calling it directly has exactly the same effect, as calling the
default `kbdvec` does on TOS.

| Arg | Meaning |
|-----|---------|
| `arg1` | the IKBD key byte, `0..0xFF` (bit 7 = break), as `kbdvec` receives it in `x0` |
| `arg2` | **reserved**, must be **0** |

| Return | Condition |
|--------|-----------|
| `E_OK` (0) | the byte was decoded |
| `EINVAL` (-25) | `arg1 > 0xFF` or `arg2 != 0`; nothing ran |
| `ERROR` (-1) | the host has no keyboard decoder (a hostless build) |

Tests: `tos-api/tests/xbios_kbdvecs.rs` (`ssystem_kbdvec_runs_the_default_decoder`).

#### `S_NEWTOS_RESET` (2021) — the OSHEADER reset entry (RST01)

```
Ssystem(2021, 0, 0)
```

**NewTOS-internal**, like 2002: the kernel entry of the **reset stub** every
process finds in its page 0 at `0xA40`, the address OSHEADER `reseth` (+0x04)
and `os_res1` (+0x10) hold. EmuTOS stores `_main` — the full restart — in both
fields (`bios/startup.S:82-83,92-93`); a program that jumps there restarts the
machine (`docs/api/bios/_reset-warm-cold.md` §3). An EL0 program cannot do that
itself, so the stub asks the kernel. A program reaches it by jumping through
the OSHEADER field (TeraDesk's "Reboot", `main.c:2096,2118`), not by number.

Warm or cold follows EmuTOS `memchk` (`bios/memory.S:125-140`): warm while
`memvalid`/`memval2`/`memval3` in the caller's page 0 still hold their magics,
cold otherwise. **Both restart the Pi through the PM watchdog for now** (RST01
decision 2; the warm start that keeps the system variables is RST02); the
kind is logged. Only the data cache is cleaned first; no file buffers are
written (decision 3, as EmuTOS).

| Arg | Meaning |
|-----|---------|
| `arg1` | **reserved**, must be **0** |
| `arg2` | **reserved**, must be **0** |

| Return | Condition |
|--------|-----------|
| — | does not return: the machine restarts (`board-rpi4`) |
| `EINVAL` (-25) | `arg1 != 0` or `arg2 != 0`; nothing happened |
| `ERROR` (-1) | the host has no restart mechanism (`board-qemu-virt`, a hostless build); the stub returns it to its caller |

Code: `tos-api/src/bios/reset.rs`, `kernel/src/reset.rs`. Tests:
`tos-api/tests/bios_reset.rs`.

#### Interface (both modes)

```
Ssystem(2000 | 2001, arg1, arg2)
```

| Arg | Meaning |
|-----|---------|
| `arg1` | pointer to a **32-bit LONG** that receives the value, or **0** to have the value returned in the call's own LONG |
| `arg2` | **reserved** for a future selector (a second clock, a second sensor). Must be **0** today |

| Return | Condition |
|--------|-----------|
| `E_OK` (0) | `arg1 != 0`: the value was written to `*arg1` |
| the value | `arg1 == 0`: the reading itself, in the returned LONG |
| `ERROR` (-1) | this board cannot measure the value; nothing is written |
| `EINVAL` (-25) | `arg2 != 0`; checked **before** the sensor is read, and nothing is written |

`arg1` follows FreeMiNT's own convention for "answer into the caller's
storage" (`S_GETBOOTLOG` 29, `S_KNAME` 900: `arg1` = buffer), narrowed to a
single LONG. The pointer form is the unambiguous one — a returned LONG cannot
distinguish a value of `0xFFFF_FFFF` from `ERROR`. On a Pi 4 neither reading
can reach that far (ARM clock ≤ 2.0 GHz; a die temperature of 4.29 million
degrees is not a physical state), but the contract, not the current board,
decides which form a caller should prefer.

`arg2` is *refused*, not ignored, so a program written against a later, wider
version of this call is told it is talking to an older kernel instead of being
handed the wrong quantity.

#### Where the values come from

Neither reading is a kernel constant. Both go out through the
[`Host`](../../ARCHITECTURE.md) trait (`tos-api/src/host.rs`,
`Host::cpu_clock_hz` / `Host::cpu_temperature_mdeg`) — the same swappable
hardware boundary as `Host::raster`:

| Board | Source | Answer |
|-------|--------|--------|
| `board-rpi4` | BCM2711 VideoCore mailbox property tags `0x0003_0002` (`GET_CLOCK_RATE`, clock id 3 = ARM) and `0x0003_0006` (`GET_TEMPERATURE`, sensor id 0) — `docs/hardware/mailbox-framebuffer.md` §3d | the firmware's reading, live per call |
| `board-qemu-virt` | no VideoCore | trait default `None` → `ERROR` |
| hostless (unit test) | — | trait default `None` → `ERROR` |

**A board that cannot measure a value says so.** There is no fallback constant
anywhere in this path: `None` becomes `ERROR`, and the caller decides what to
print. Observed on QEMU `raspi4b` (2026-09-24): its firmware model *does*
answer both tags, with the fixed values 700 000 000 Hz and 25 000 m°C — those
are the emulator's numbers, faithfully reported, and only a real Pi 4 shows a
clock that moves and a die that warms up.

#### System-information modes (2003–2020) — SYSINFO01, KEYOWNER

The settings and machine facts a system-information tool shows (the SYSACC desk
accessory, `sdk/examples/sysacc.c`, TASK/SYSINFO01). Every mode reports the
**effective** state the kernel runs with — what the boot code actually did with
the boot arguments (`cmdline.txt`, `kernel/src/bootargs.rs`) and what the
hardware actually negotiated — never the text of a configuration file. A
program that wants to show `memprot=strict` learns "strict" only if the kernel
really runs strict.

**Interface of the scalar modes 2003–2018 and 2020** — exactly the one of 2000/2001
(§"Interface (both modes)" below): `arg1` = pointer to a 32-bit LONG that
receives the value, or 0 for the value in the returned LONG; `arg2` reserved,
must be 0 (`EINVAL` before anything is read, nothing written); `ERROR` (-1)
when the value is unknown on this machine (nothing written). No value of these
modes can reach `0xFFFF_FFFF`, but the pointer form remains the recommended
one, for the same reason as there.

| Mode | Value | Answer (`E_OK` / returned LONG) | `ERROR` when | Source of the effective state |
|------|------:|---------------------------------|--------------|-------------------------------|
| `S_NEWTOS_CPUMEAS` | 2003 | the ARM clock the firmware **measured**, Hz — *"the true/actual clock rate … which respects clamping, throttling and clock divider limitations"*, where 2000 answers *"the last requested value"* | the firmware does not answer tag `0x0003_0047` or answers rate 0; no VideoCore (`qemu-virt`, hostless) | `Host::cpu_clock_measured_hz` → mailbox "Get clock rate measured" `0x0003_0047`, clock id 3 (`docs/hardware/mailbox-framebuffer.md` §3f; `references/rpi4-boot/firmware-wiki-mailbox-property-interface.md`), live per call |
| `S_NEWTOS_CPUPOLICY` | 2004 | **0** = the firmware controls the ARM clock (no `cpuclk=max`, or `cpuclk=firmware`); **1** = fixed at the firmware maximum (`cpuclk=max`, "Set clock rate" answered); **2** = `cpuclk=max` asked, but the firmware reported no maximum or did not answer "Set clock rate" — the clock stayed with the firmware | the board has no clock control (`qemu-virt`, hostless) | `Host::cpu_clock_policy`, recorded by the kernel's boot step `raise_arm_clock` (FS08 part 3) |
| `S_NEWTOS_MEMPROT` | 2005 | **0** = off (`memprot=off`); **1** = on (default); **2** = on and strict (`memprot=strict`, FreeMiNT `MPF_STRICT`). `memprot=off memprot=strict` is off: strict has no effect without protection | — (always known once GEMDOS runs) | `tos_api::bdos::process::memprot::{enabled, strict}` (ARCH03 3b/E5) |
| `S_NEWTOS_ASID` | 2006 | **0** = off (default; TLB dropped on every address-space switch); **1** = on (`asid=on`); **2** = test mode (`asid=tiny`: ASIDs on, four numbers per generation — the rollover test) | — | `tos_api::bdos::process::mmu::{asids_enabled, asid_stats}`, the generation limit `ASID_TEST_LIMIT` (ARCH03 stage 4) |
| `S_NEWTOS_SDMODE` | 2007 | **0** = fast mode (multi-block `CMD18`/`CMD25`, 4-bit bus where the card offers it — FS08); **1** = compatibility mode (`emmc=compat`: 1-bit bus, one `CMD17`/`CMD24` per sector) | no SD card initialised, or a board without the SD driver (`qemu-virt`, hostless) | `Host::sd_card_mode` → `kernel::drivers::bcm2711::emmc` (`COMPAT`) |
| `S_NEWTOS_SDBUS` | 2008 | the data-bus width the driver and the card actually use, in bits: **1** or **4**. Fast mode with a card whose SCR lists no 4-bit bus (or whose `ACMD6` failed) answers 1 | as 2007 | `Host::sd_card_mode` → `emmc` (`BUS_4BIT`, set only after `ACMD6` succeeded and the host was switched) |
| `S_NEWTOS_BCACHE` | 2009 | the memory given to the block read cache, **KiB**; **0** = no cache (`bcache=0`, or a board without a block device) | — | `tos_api::bios::bcache` (FS09): the arena handed to `install`, the same number as the boot line `[bcache] … N KiB at …` |
| `S_NEWTOS_BCACHEPCT` | 2010 | the share of the cache one linear read may fill, percent (`bcachepct=`, default 5) | no cache installed | `tos_api::bios::bcache::stats().pct` |
| `S_NEWTOS_STRAM` | 2011 | the ST-RAM pool the BDOS manages, **KiB** (`end − start` as seeded at boot; the memory `Malloc`/`Mxalloc` can hand out, the extent `phystop` closes) | GEMDOS memory not initialised | `tos_api::bdos::mem::init::stram_range` |
| `S_NEWTOS_TTRAM` | 2012 | the TT-RAM (alt-RAM) pool as seeded at boot, **KiB**; **0** = the machine has none (EmuTOS `ramtop == NULL`). A later `Maddalt` region does not count, as it does not move `ramtop` | as 2011 | `tos_api::bdos::mem::init::ttram_range` |
| `S_NEWTOS_SCRWIDTH` | 2013 | width of the hardware framebuffer, pixels | no display up | `Host::screen().width` (the framebuffer the kernel allocated from the VideoCore) |
| `S_NEWTOS_SCRHEIGHT` | 2014 | height, pixels | as 2013 | `Host::screen().height` |
| `S_NEWTOS_SCRDEPTH` | 2015 | bits per pixel of the hardware framebuffer (32 = the XRGB8888 scan-out). This is the framebuffer, not the VDI workstation's colour model | as 2013 | `Host::screen().bpp` |
| `S_NEWTOS_GPUMEM` | 2016 | the memory the firmware gives the VideoCore (`gpu_mem` split), **KiB** | the firmware did not answer "Get VC memory"; no VideoCore | `Host::gpu_memory_bytes` → mailbox "Get VC memory" `0x0001_0006` (§3a), read once at boot with "Get ARM memory" — the split cannot change while the machine runs |
| `S_NEWTOS_KEYBOARD` | 2017 | where the active layout came from: **0** = built-in US (the default, or `#K us`); **1** = built-in German (`#K de` in `NEWDESK.INF`); **2** = loaded from `C:\KEYBOARD.KBD`; **3** = changed since by a program (XBIOS `Keytbl` replaced a table with different contents) | the board has no keyboard translator (hostless) | `Host::keyboard_layout` → `kernel::keymap` |
| `S_NEWTOS_SERIAL` | 2018 | **0** = serial console output off; **1** = on (`serial=on` on `board-rpi4`; always on on `board-qemu-virt`, whose serial log is its only console) | hostless | `Host::serial_console` → `kernel::console::serial_tx_enabled` (SER01) |
| `S_NEWTOS_INPUTMODE` | 2020 | where the keyboard/mouse pump runs (INPUT01): **0** = also from the 200 Hz timer interrupt while a program runs, so the pointer moves while a program is busy (the default); **1** = only in the AES waits and the console reads (`input=aes`, the behaviour before INPUT01). Added by TASK/KEYOWNER after the 2003–2019 block | hostless | `Host::input_mode` → `kernel::input_gate::mode` (decided once at boot from `input=aes`, `kernel/src/bootargs.rs` `INPUT_AES`) |

**`S_NEWTOS_KVERSION` (2019) — the kernel version string.** The string form of
FreeMiNT's `S_KNAME` (900, `sys/ssystem.c:572-576` → `strncpy_f`,
`sys/libkern/strncpy_f.c:47-56`):

```
Ssystem(2019, char *buf, LONG len)
```

| Arg | Meaning |
|-----|---------|
| `arg1` | the caller's buffer |
| `arg2` | its size in bytes (a signed LONG) |

The kernel copies at most `len − 1` characters of the version string and always
terminates with a NUL — `strncpy_f`'s rule; `len <= 0` writes nothing and is
still `E_OK`. The string is the OS name and version followed by the build stamp,
`"NewTOS 0.6.1 2026-10-07 (c50a9c9)"` — the same two texts the boot screen prints
as *Version* and *Built* (`kernel/src/initinfo.rs` `VERSION_STRING`,
`tos_api::build::BUILD_STAMP`). It is shorter than 64 bytes.

| Return | Condition |
|--------|-----------|
| `E_OK` (0) | copied (or `len <= 0`: nothing to copy) |
| `EINVAL` (-25) | `arg1 == 0` with `len > 0` (FreeMiNT would write to address 0; NewTOS refuses) |
| `ERROR` (-1) | the host reports no version (hostless) |

**Not a number from the boot arguments.** Every scalar above is the state the
kernel ended up in: a `bcache=` value that was refused keeps the default and
2009 reports the default; `emmc=` on a board whose card did not initialise is
`ERROR`, not "fast"; `cpuclk=max` that the firmware did not grant is 2, not 1.
Where a fact is not known on a board, the answer is `ERROR` — the same rule as
2000/2001.

**Board table (SYSINFO01):**

| Mode | `board-rpi4` (real Pi 4 / QEMU `raspi4b`) | `board-qemu-virt` | hostless (unit test) |
|------|-------------------------------------------|-------------------|----------------------|
| 2003 | firmware answer (QEMU: whatever its model answers, or `ERROR`) | `ERROR` | `ERROR` |
| 2004 | 0 / 1 / 2 | `ERROR` | `ERROR` |
| 2005, 2006 | value | value | value |
| 2007, 2008 | value with a card, else `ERROR` | `ERROR` | `ERROR` |
| 2009 | KiB | 0 | 0 |
| 2010 | percent, `ERROR` with `bcache=0` | `ERROR` | `ERROR` |
| 2011, 2012 | KiB | KiB | `ERROR` until `bdos::mem::init` |
| 2013–2015 | framebuffer | framebuffer if a display is up | `ERROR` |
| 2016 | firmware answer | `ERROR` | `ERROR` |
| 2017 | 0–3 | 0–3 | `ERROR` |
| 2018 | 0 / 1 | 1 | `ERROR` |
| 2019 | string | string | `ERROR` |
| 2020 | 0 / 1 | 0 / 1 | `ERROR` |

Tests: `tos-api/tests/gemdos_ssystem_sysinfo.rs` (2023 answers `EINVFN`; 2021 is
`S_NEWTOS_RESET`, 2022 the USB snapshot — on a host without a USB stack a
well-formed 2022 request is `ERROR`); the snapshot itself:
`tos-api/tests/gemdos_ssystem_usbsnap.rs` (return values of
[`../usb/usb-snapshot.md`](../usb/usb-snapshot.md) §2.3 against the kernel's
encoder) and the encoder's byte tests in `kernel/src/drivers/usb/snapshot.rs`
(run by `cargo xtask test`).

---

## `S_GETCOOKIE` (mode 8) — `get_cookie(NULL, arg1, (ulong *)arg2)`

Dispatcher: `sys/ssystem.c:277-282`. `arg1` = cookie **tag**; `arg2` = a
pointer to a `LONG` where the value is written (the *return-by-pointer* form),
or `0` to return the value directly in `x0`. **No privilege required.** Three
behaviours (`sys/cookie.c:400-510`):

1. **`arg1 == 0`** (`sys/cookie.c:422-442`) → scan to the terminator and take
   the **NULL-slot value** = the jar's **total slot capacity**. If `arg2 != 0`,
   write it to `*arg2` and return `E_OK`; if `arg2 == 0`, return the capacity
   directly in `x0`.
2. **`arg1` high word == 0** (`(arg1 & 0xffff0000) == 0`, a small number,
   `sys/cookie.c:448-483`) → treat `arg1` as a **1-based slot index**: return
   the **tag** of the Nth cookie (not its value, and **not** a `{tag,value}`
   pair). If `arg2 != 0`, write the tag to `*arg2` and return `E_OK`; else
   return the tag in `x0`. If the index exceeds `cookie_count + 1`, return
   `EINVAL` (`sys/cookie.c:458-462`).
3. **otherwise** (`sys/cookie.c:487-509`) → search the jar for `tag`; on match,
   if `arg2 != 0` write the **value** to `*arg2` and return `E_OK`, else return
   the value in `x0`. If the tag is absent, return `EERROR` (-1)
   (`sys/cookie.c:508-509`).

(Correction vs. earlier draft: slot-index mode returns the **tag only**, not a
`{tag,value}` entry; not-found is `EERROR` (-1), and the over-index error is
`EINVAL`. See D-038.)

## `S_SETCOOKIE` (mode 9) — `set_cookie(NULL, arg1, arg2)`

Dispatcher: `sys/ssystem.c:283-292`. **Root only** — if the caller is not root,
return `EPERM` **without** touching the jar (`sys/ssystem.c:285-289`).
Otherwise `arg1` = tag, `arg2` = value (`sys/cookie.c:516-576`):

- **Validate the tag** (`sys/cookie.c:534-541`): each of the four bytes must be
  non-zero (`tag & 0xff000000`, `& 0x00ff0000`, `& 0x0000ff00`, `& 0x000000ff`
  all non-zero). This keeps the small-number space (high word 0) usable as slot
  indices by `S_GETCOOKIE`. Invalid → `EINVAL`.
- **Update or append** (`sys/cookie.c:545-570`): scan to the terminator,
  counting entries into `n`. If `tag` is already present, overwrite its value →
  `E_OK`. Otherwise, if there is room (after `n++`, `n < terminator.value`,
  where `terminator.value` is the jar's total slot capacity), write
  `{tag, value}` into the old terminator slot and push a fresh
  `{0, capacity}` terminator one slot down → `E_OK`. If the jar is full →
  `ENOMEM` (`sys/cookie.c:574-575`). (FreeMiNT does not grow the jar here; it
  was sized with spare slots at `init_cookies`.)

## `S_DELCOOKIE` (mode 26) — `del_cookie(NULL, arg1)`

Dispatcher: `sys/ssystem.c:293-302`. **Root only** — non-root → `EPERM`
(`sys/ssystem.c:295-299`). Otherwise `arg1` = tag; scan the jar, and on a match
compact the array by copying each following entry (including the terminator)
down one slot → `E_OK` (`sys/cookie.c:591-604`). If the tag is not found →
`EINVAL` (`sys/cookie.c:606-607`).

---

## Error conditions (cookie subset)

FreeMiNT error values (`sys/mint/errno.h`) — note these are the **FreeMiNT**
numbering, which does not always coincide with the TOS/GEMDOS `tos-types::error`
set NewTOS uses (see Open questions):

| Condition | Mode | FreeMiNT code | Value |
|-----------|------|---------------|------:|
| success | any | `E_OK` | 0 |
| `mode == -1` probe | — | `E_OK` | 0 |
| tag not found | `S_GETCOOKIE` (search) | `EERROR` | -1 |
| slot index too big | `S_GETCOOKIE` (index) | `EINVAL` | -25 |
| caller not root | `S_SETCOOKIE`/`S_DELCOOKIE` | `EPERM` | -38 |
| invalid tag (a zero byte) | `S_SETCOOKIE` | `EINVAL` | -25 |
| jar full | `S_SETCOOKIE` | `ENOMEM` | -39 |
| tag not found | `S_DELCOOKIE` | `EINVAL` | -25 |
| unknown mode | any other | `ENOSYS` | -32 |

---

## Verification criteria (cookie subset)

- **Probe:** `Ssystem(-1, 0, 0)` returns `E_OK` (0).
- **Get capacity:** `Ssystem(S_GETCOOKIE, 0, 0)` returns the jar's total slot
  count in `x0`; with a non-zero `arg2`, writes that count to `*arg2` and
  returns `E_OK`.
- **Get present tag by-value:** with an `NTOS` cookie present, `Ssystem(
  S_GETCOOKIE, 'NTOS', &buf)` returns `E_OK` and writes the version value to
  `buf`.
- **Get absent tag:** `Ssystem(S_GETCOOKIE, 'ZZZZ', &buf)` returns `EERROR`
  (-1) (or its NewTOS mapping) and does not fault.
- **Get by slot index:** `Ssystem(S_GETCOOKIE, 1, &buf)` writes the **tag** of
  the first cookie to `buf` and returns `E_OK`; an index past the end returns
  `EINVAL`.
- **Set new (root):** `Ssystem(S_SETCOOKIE, 'TEST', 0x1234)` as root returns
  `E_OK`; a subsequent `S_GETCOOKIE 'TEST'` yields `0x1234`.
- **Set invalid tag:** `Ssystem(S_SETCOOKIE, 0x00414243, v)` (a zero high
  byte) returns `EINVAL`.
- **Set non-root:** the same call from a non-root process returns `EPERM` and
  leaves the jar unchanged.
- **Delete present (root):** `Ssystem(S_DELCOOKIE, 'TEST', 0)` returns `E_OK`;
  a following `S_GETCOOKIE 'TEST'` returns not-found.
- **Delete absent:** `Ssystem(S_DELCOOKIE, 'ZZZZ', 0)` returns `EINVAL`.
- **Unknown mode:** `Ssystem(9999, 0, 0)` returns `ENOSYS`/`EINVFN`.

## Verification criteria (NewTOS-own modes 2000/2001)

Driven at the trap boundary by `tos-api/tests/gemdos_ssystem_cpuinfo.rs`, and
end-to-end on a board by `C:\SYSINFO.NCP` (`sdk/examples/sysinfo.c`, installed
by `tools/make-sd-image.sh`; `SD_SYSINFO_AUTO=1` additionally puts it in
`\AUTO\` for a headless serial run).

- **Value form:** with a host that can measure, `Ssystem(2000, 0, 0)` returns
  the ARM clock in Hz and `Ssystem(2001, 0, 0)` the temperature in 1/1000 °C,
  each in the call's own LONG.
- **Pointer form:** `Ssystem(2000, &buf, 0)` returns `E_OK` and writes the
  same value to `buf`; likewise for 2001. Both forms must agree.
- **Board without the sensor:** both modes return `ERROR` (-1) and write
  **nothing** through `arg1` — a caller that ignores the status must not be
  able to mistake a stale buffer for a reading.
- **Reserved `arg2`:** `Ssystem(2000, 0, 1)` and `Ssystem(2001, 0, 1)` return
  `EINVAL` (-25) and write nothing.
- **Neighbouring numbers stay unimplemented:** 1999, 2003 and 2999 return
  `EINVFN`, so the two modes cannot be reached by accident.
- **The probe is unaffected:** `Ssystem(-1, 0, 0)` still returns `E_OK`.

---

## Edge cases discovered in source

1. **`arg2` overloads value-in-d0 vs value-by-pointer.** For `S_GETCOOKIE`,
   `arg2 == 0` means "return the value in `x0`"; `arg2 != 0` means "write the
   value to `*arg2` and return status". FreeMiNT decides this per call
   (`sys/cookie.c:431-441,472-482,492-502`). NewTOS must support both forms.

2. **Slot-index mode returns the tag, not the value.** By-index
   (`S_GETCOOKIE` with high-word-zero `arg1`) enumerates and returns the
   **tag** of the Nth cookie — useful for walking the jar without knowing tags
   up front. Returning the value would be wrong (D-038).

3. **Tag validation exists to protect slot indexing.** `set_cookie` rejects any
   tag with a zero byte precisely so the low-number tag space stays reserved
   for the by-index `get_cookie` form (`sys/cookie.c:530-541`). NewTOS keeps
   this rule.

4. **Root gate on set/delete, none on get.** Reads are unprivileged; mutations
   require root (`suser(...)`, `sys/ssystem.c:59,285,295`). NewTOS's process
   model must define "root" (or the equivalent capability) before implementing
   the mutating modes.

5. **Not-found errors differ between get and delete.** `S_GETCOOKIE` returns
   `EERROR` (-1) for a missing tag, but `S_DELCOOKIE` returns `EINVAL` (-25) —
   two different codes for "no such cookie". Preserve the distinction (or
   document a deliberate NewTOS unification).

6. **Terminator `value` is the capacity, not a cookie value.** Both `get`
   (capacity query) and `set` (room check) treat the `{0, N}` terminator's `N`
   as the total slot count — the same invariant as the EmuTOS jar
   (`docs/api/bios/_cookie-jar.md`).

---

## Dependencies

### Requires (must be initialized / functional)

- The NewTOS cookie jar (kernel-owned, ADR-012) — seeded at boot with the
  NewTOS cookie set. See `docs/api/bios/_cookie-jar.md`.
- A GEMDOS-dispatcher special-case arm for `0x154`
  (`docs/api/gemdos/_dispatcher.md`).
- A process privilege model (for the root gate on set/delete).
- An EL0-checked user-store path for the `arg2` return-by-pointer writes.

### Used by

- MiNT-aware programs and libraries that read/write cookies without `Supexec`
  + `$5A0` (the replacement path ADR-012 chose).

---

## Related data structures

- `docs/api/bios/_cookie-jar.md` — the cookie-jar data model + NewTOS access
  decision (ADR-012).
- `docs/api/gemdos/_state.md` §9 — OS header / cookie-jar values (contract).
- `docs/api/gemdos/_dispatcher.md` — the high-id special-case dispatch.

---

## Cross-references

### FreeMiNT (authority for this function)

- `sys/syscall_vectors.c:473` — trap `0x154` → `sys_s_system`.
- `sys/ssystem.c:56-653` — the mode dispatch.
- `sys/mint/ssystem.h` — mode-code numbers.
- `sys/cookie.c:400-608` — `get_cookie`/`set_cookie`/`del_cookie`.
- All @ FreeMiNT commit `f906416`.

### Atari Compendium / MiNT programmer docs

- Not present in `references/` as PDFs. Cross-check deferred.
  (`references/freemint/doc/programmer/newcall.html/ssystem.html` is the
  in-tree FreeMiNT programmer doc; it matches the source above.)

### Decision

- ADR-012 (`docs/MEMORY.md`) — kernel-owned jar, function-returns-content
  access.

---

## Configuration / version notes

- `Ssystem` and its mode codes are a FreeMiNT/MiNT feature; EmuTOS has none of
  it. NewTOS implements only the cookie subset initially; the rest return
  `ENOSYS`.
- The FreeMiNT error numbering (`sys/mint/errno.h`) differs from TOS/GEMDOS
  error codes; NewTOS returns the values chosen in the Open-questions
  resolution below, not necessarily the FreeMiNT numbers verbatim.

---

## NewTOS implementation notes

- **Jar storage (ADR-012):** kernel-owned, allocated from the kernel heap,
  pointer in a kernel-internal global (the NewTOS analogue of FreeMiNT's
  `*CJAR` — **not** the Atari `$5A0` sysvar). The jar lives in EL1 high-half
  memory; user programs never see the pointer, only values via this trap.
  FreeMiNT's return-by-pointer `arg2` fits this exactly — the kernel writes one
  `LONG` to the caller's `arg2` buffer with a permission-checked store.
- **Growth:** dynamic allocation means no hard cap; grow the jar (realloc +
  copy + re-seed the `{0, capacity}` terminator) when `set_cookie` would
  overflow, instead of returning `ENOMEM` — a permitted NewTOS improvement over
  FreeMiNT's fixed spare-slot jar.
- **NewTOS cookie set** (seeded at boot, replacing `fill_cookie_jar` /
  `init_cookies`): keep the meaningful tags — `_MCH` (a NewTOS machine id),
  `_IDT` (date/time format), `_FLK` (GEMDOS file-locking present), `_AKP`
  (keyboard language) — and add an **`NTOS`** identity cookie (value = NewTOS
  version) so ported programs can detect the environment. **Drop**
  `_VDO`/`_SND`/`_FDC`/`_FPU`/`_CPU` (Shifter/PSG/floppy/m68k-FPU/m68k-CPU —
  meaningless on AArch64). See `docs/api/bios/_cookie-jar.md`.
- **`arg2` writes** target a user buffer: use the same EL0-permission-checked
  store path as `Cconrs`, not a raw kernel store, so a bad pointer faults the
  caller rather than the kernel.
- **Root gate:** map FreeMiNT's `suser()` to NewTOS's process privilege /
  capability check; return the NewTOS "access denied" code for non-privileged
  set/delete.

---

## Known implementation gaps (`tos-api`, audit 2026-07-09)

A source-analysis audit of `tos-api/src/gemdos/ssystem.rs` and
`tos-api/src/cookie.rs` against this spec found two divergences (D-072,
D-073, `docs/api/_discrepancies.md`). Both are **implementation** bugs — the
spec above was already correct on every point checked — for the Developer to
fix, not a spec correction:

1. **`mode == -1` probe missing.** `ssystem()`'s `match` only handles
   `S_GETCOOKIE`(8)/`S_SETCOOKIE`(9)/`S_DELCOOKIE`(26); every other value,
   including `-1`, falls to the wildcard arm and returns `EINVFN`, so a
   feature-test probe wrongly reports "not implemented". See D-072 for the
   exact fix.
2. **Cookie error codes wrong; `S_GETCOOKIE(0)` returns occupied count, not
   capacity.** `s_getcookie`'s slot-index-too-big and `s_delcookie`'s
   not-found both currently return `ERROR` (-1) instead of `EINVAL` (-25);
   `s_setcookie`'s invalid-tag path returns `EINVFN` (-32) instead of
   `EINVAL` (-25). `cookie::count()` (occupied slots) is used for the
   `tag == 0` capacity query, where FreeMiNT returns the jar's fixed **total
   capacity** instead. See D-073 for the exact fix (including the two new
   `tos_types::error` constants it requires).

## Open questions

1. ~~**Error-code mapping.**~~ **RESOLVED (D-073, 2026-07-09).** For the
   cookie subset in scope here, NewTOS returns FreeMiNT's own MiNT-errno
   values **verbatim**: `E_OK`=0, `EERROR`=-1 (`ERROR` in `tos_types::error`,
   already correct), `EINVAL`=-25 (new constant needed), `EPERM`=-38 (new
   constant needed, for the root gate), `ENOMEM`=-39 (`ENSMEM` in
   `tos_types::error`, already numerically identical). Rationale: `Ssystem`
   is a FreeMiNT-only extension with no classical TOS/GEMDOS equivalent, so
   per `docs/COMPATIBILITY.md` its return values are **contract** toward
   MiNT-aware callers, not a free NewTOS internal — and none of these four
   values collides with an existing TOS `gemerror.h` code (`-25` and `-38`
   are unused slots in the classical table; only `ENOSYS`/`EINVFN` (`-32`)
   and `EBADARG`/`ERANGE` (`-64`) coincidentally alias, and those affect only
   the *deferred* modes, not the cookie subset). The unknown-mode return
   stays `EINVFN` as already recommended above.
2. **Scope confirmation.** Only `S_GETCOOKIE`/`S_SETCOOKIE`/`S_DELCOOKIE` (+
   the `mode == -1` probe) are in NewTOS scope for now; every other mode →
   `ENOSYS`. Confirm no early consumer needs `S_OSVERSION`/`S_OSNAME` (identity)
   or `S_SETEXC`/`S_SECLEVEL` before Phase-08.

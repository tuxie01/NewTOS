# XBIOS Subsystem State

> Phase 00 deliverable D7 (XBIOS). Documents the XBIOS-internal state
> referenced by spec entries in this group. Source: EmuTOS @ `1a7484ab`.
> Cite-never-paste: structures are given as byte-exact layout tables, not
> as pasted C.

XBIOS owns less mutable state than GEMDOS — most of its surface is direct
hardware access plus a handful of OS variables in low memory. This file
describes those variables and the small number of tables XBIOS maintains.

> **Observable vs. internal** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract** (a program observes value/layout — keep exact): the
> `kbdvecs` table (`Kbdvbase`), the `IOREC`/`EXT_IOREC` layouts (`Iorec`),
> the packed palette word (`Setcolor`/`Setpalette`), the resolution codes
> (`Getrez`/`Setscreen`), the `Random` reproducible sequence, the
> `kbrate`/`keytbl` return encodings, the low-memory var *values* (`v_bas_ad`,
> `dumpflg`, `dump_vec`, `colorptr`, `frclock`). **Free to reimplement**: the
> hardware shadows, SR-locking, the ACIA/MFP driver internals, the RTC
> backend, the physical low-memory addresses (meaningless on AArch64 for
> native callers — see COMPATIBILITY §System variables).

---

## 1. `kbdvecs` (`include/biosdefs.h:105`, returned by `Kbdvbase` 0x22)

The keyboard/MIDI packet-handler vector table. `Kbdvbase` returns `&kbdvecs`
(`bios/xbios.c:716-718`). 9 function pointers, **36 bytes** on m68k:

| Offset | Field       | Purpose |
|-------:|-------------|---------|
| 0x00   | `midivec`   | MIDI input byte handler |
| 0x04   | `vkbderr`   | IKBD ACIA overrun / error handler |
| 0x08   | `vmiderr`   | MIDI ACIA overrun / error handler |
| 0x0C   | `statvec`   | IKBD status packet (0xF6) handler |
| 0x10   | `mousevec`  | IKBD mouse packet (0xF7 absolute, 0xF8..0xFB relative; `bios/aciavecs.S:488-492`) handler |
| 0x14   | `clockvec`  | IKBD time-of-day packet (0xFC) handler |
| 0x18   | `joyvec`    | IKBD joystick packet (0xFD..0xFF) handler |
| 0x1C   | `midisys`   | top-level MIDI ACIA ISR |
| 0x20   | `ikbdsys`   | top-level IKBD ACIA ISR |

> **Correction (KEYS02 draft, 2026-10-04):** the table is incomplete. Directly
> *before* offset 0x00 sits the undocumented `kbdvec` slot (TOS >= 2.0,
> `bios/aciavecs.S:129`, called with d0 = scancode incl. bit 7 = break, a0 =
> `ikbdiorec`), before it `mousexvec` (`CONF_WITH_EXTENDED_MOUSE`, `:126-128`);
> after `ikbdsys` come the bytes `ikbdstate` (Compendium: `drvstat`) and
> `kbdlength` (`:140-141`). Full analysis: `22-kbdvbase.md`.

All fields are `PFVOID` (4 bytes on m68k). `Initmous` writes `mousevec`
(`bios/mouse.c:117,120`); applications install their own handlers by
overwriting a field (read-save-chain — no previous-vector return). The
struct lives in BIOS BSS, process-lifetime stable.

> The internal IKBD-packet state machine keeps a few private fields
> (`ikbdstate`, `kbdlength`) adjacent to the table in `bios/aciavecs.S`.
> **Correction (KEYS02):** `ikbdstate` is the Compendium's documented
> `drvstat` and *is* contract surface; only `kbdlength` is internal
> (`22-kbdvbase.md`, Korrekturen 3).

**NewTOS (KEYS02, built 2026-10-04):** one table **per process**, in page 0 of
its TPA at `0x9A0` with **8-byte** slots (LP64): `mousexvec` `0x9A0`, `kbdvec`
`0x9A8`, the nine slots from `0x9B0` (= `Kbdvbase()`), `drvstat`/`kbdlength`
`0x9F8`/`0x9F9` (always 0). Defaults are EL0 stubs (`ret` at `0xA00`;
`kbdvec_default` at `0xA08` → `Ssystem(2002)` → the kernel decoder). Only
`kbdvec` is called — as an EL0 upcall at the next syscall return, one byte per
upcall, `x0` = scancode, `x1` = 0; the other slots are present but not called
(documented gap). Full binding: `22-kbdvbase.md` Teil B/C; code
`tos-api/src/xbios/kbdvecs.rs`.

---

## 2. Keyboard-table state (`bios/ikbd.c`, used by `Bioskeys` 0x18, `Keytbl` 0x10, `Kbrate` 0x23)

### `current_keytbl` (`struct keytbl`, `bios/ikbd.h:57-86`) — **30 bytes**

| Offset | Field      | Type | Meaning |
|-------:|------------|------|---------|
| 0x00   | `norm`     | `const UBYTE *` | 128-byte unshifted scancode→ASCII table |
| 0x04   | `shft`     | `const UBYTE *` | 128-byte shifted table |
| 0x08   | `caps`     | `const UBYTE *` | 128-byte caps-lock table |
| 0x0C   | `altnorm`  | `const UBYTE *` | Alt (variable format) |
| 0x10   | `altshft`  | `const UBYTE *` | Alt+Shift (variable format) |
| 0x14   | `altcaps`  | `const UBYTE *` | Alt+Caps (variable format) |
| 0x18   | `dead`     | `const UBYTE *const *` | dead-key table array |
| 0x1C   | `features` | `WORD` | feature flags (e.g. `DUAL_KEYBOARD = 0x0001`) |

`Keytbl` (0x10) overwrites `norm`/`shft`/`caps` unless the argument is the
sentinel `(UBYTE *)-1`, and returns `&current_keytbl` (`bios/ikbd.c:122-134`).
`Bioskeys` (0x18) resets `current_keytbl = *get_keytbl()` from the compiled
country tables (`bios/ikbd.c:136-140`). The IKBD ACIA ISR reads these tables
when converting scancodes.

> **Correction vs. earlier draft:** the old `_state.md` listed the table as
> only `{unshift, shift, caps}`. The real struct is 30 bytes with six table
> pointers + `dead` + `features`. `Keytbl` returns a pointer to the whole
> struct, and the sentinel is `(UBYTE *)-1`, not `NULL`. See D-042.

### Key-repeat rate (`bios/ikbd.c:425-427`)

```
WORD kb_initial;   /* initial delay, units of 20 ms */
WORD kb_repeat;    /* repeat interval, units of 20 ms */
WORD kb_ticks;     /* countdown timer (private) */
```

`Kbrate(initial, repeat)` (0x23) returns `MAKE_UWORD(kb_initial, kb_repeat)`
(old initial in the **high** byte, old repeat in the **low** byte), then, for
each parameter `>= 0`, stores it masked to `& 0x00ff`; a negative parameter is
a query (`bios/ikbd.c:439-449`). Powerup defaults are `KB_INITIAL = 15`
(≈300 ms) and `KB_REPEAT = 2` (≈40 ms) (`include/sysconf.h:68-69`, set by
`kbd_init`, `bios/ikbd.c:1092-1093`).

> **Correction vs. earlier draft:** the old `_state.md` gave defaults
> `kb_initial = 25` / `kb_repeat = 5` and 50 Hz units. The source uses
> 15 / 2 in 20 ms units. See D-042.

### Key-repeat handler (`bios/ikbd.c:395-536`, `:777-909`)

The two `Kbrate` fields are consumed by a **software** repeat in the BIOS
keyboard layer; the IKBD's own repeat is not used. State
(`bios/ikbd.c:425-437`): `kb_ticks` (countdown, `0` = nothing repeating),
`kb_last` (the key word last pushed: `shifty`, scancode, ASCII) and
`kb_last_actual` (the raw make scancode, before any conversion).

| Event | EmuTOS behaviour | Source |
|-------|------------------|--------|
| Make of a **non-modifier** key | `kb_last_actual = scancode`; the key word is pushed into `ikbdiorec`; `kb_ticks = kb_initial` (the value at make time); `kb_last = {shifty, scancode, ascii}` | `:875,891,898-909` |
| Make of Shift / Ctrl / Alt / CapsLock | updates `shifty` only; a running repeat **continues** | `:825-870` |
| Break of `kb_last_actual` | `kb_ticks = 0` — repeat stops | `:815-817` |
| Break of Home (`0x47`) or Insert (`0x52`) | `kb_ticks = 0` unconditionally (also clears `MODE_HOME`/`MODE_INSERT`) | `:807-813` |
| Break of any other key, or of a modifier | no effect on the repeat (Alt release stops it only during Alt-numpad entry or keyboard mouse emulation) | `:778-818` |
| Make of a *second* key while one repeats | the second key becomes `kb_last`; **only the last-pressed key repeats** — the first stays silent even though it is still held | `:875-903` |
| 50 Hz tick (every 4th 200 Hz timer-C interrupt, `bios/vectors.S:521`, sieve `0x1111` `bios/bios.c:463`) → `do_key_repeat` | (1) if `ikbdsys` changed since `kb_last` was set: `kb_ticks = 0`; (2) if `conterm & 2` is clear: return **without** decrementing (the countdown is frozen, not reset); (3) if `kb_ticks == 0`: return; (4) `--kb_ticks`; if still `> 0`: return; (5) keyclick if `conterm & 1`; (6) if `shifty` differs from `kb_last.shifty`: re-convert `kb_last_actual` with the current `shifty` and update `kb_last`; (7) push `kb_last` into `ikbdiorec` (or repeat the mouse-emulation packet); (8) `kb_ticks = kb_repeat` | `:451-536` |

Consequences that follow from the table and are part of the observable
behaviour: the first repeat comes `kb_initial` ticks (default 15 = 300 ms,
minus the sub-tick phase) after the make, then one every `kb_repeat` ticks
(default 2 = 40 ms), **unbounded**, until the break; `kb_initial = 0` means no
repeat at all, `kb_repeat = 0` means exactly one repeat; a `Kbrate` set takes
effect for `kb_initial` at the next make and for `kb_repeat` at the next
repeat. A full `ikbdiorec` drops the repeated word (`push_ikbdiorec`,
`:157-173`).

#### NewTOS binding — sanctioned deviation: the repeat runs in the polled input pump

On the Pi the keyboard is a USB-HID boot keyboard that reports **changes
only** (FreeMiNT sets its idle rate to "infinite",
`references/freemint/sys/usb/src.km/udd/hid/keyboard/usb_keyboard.c:599`),
and its reports — including every **break** — reach the decoder only when
somebody polls the xHCI ring (`kernel::drivers::usb::poll`). A repeat driven
from the timer interrupt, as EmuTOS does, would keep pushing words while no
one polls and so produce ghost keys after the release (user decision
2026-10-02, variant (b), `TASK/KEYS01.md`). NewTOS therefore runs
`do_key_repeat` **in the input pump**, right after each USB poll: first the
pending reports are decoded (so a break is seen), then the 50 Hz steps that
elapsed since the previous poll are credited. Implementation:
`kernel/src/key_repeat.rs` (state machine), `kernel/src/ikbd.rs`
(`poll_repeat`, called right after `usb::poll` in the one pump body
`pump_devices` of `kernel/src/host_env.rs`). INPUT01 (2026-10-08) added a
second, interrupt-side pump (`pump_devices_irq`, the 200 Hz timer interrupt
while a program runs at EL0; `kernel/src/input_gate.rs`, off with `input=aes`):
it decodes **pointer motion only** — a report holding a key byte is held for
the next trap-side pump — and it does not run the repeat. This binding is
therefore unchanged.

- **Same semantics as the table**: start/stop/last-key-wins/modifier
  re-conversion/`conterm & 2` freeze/`kb_initial` and `kb_repeat` read at the
  same moments. The 50 Hz steps are the multiples of 4 that `_hz_200` crosses,
  so the phase matches EmuTOS' sieve.
- **`conterm`** is read from the running process's own `$484` (each TPA has
  its own copy, `bdos::process::loader::populate_sysvars`); with no process
  running (desktop, boot) the boot value `7` applies (`bios/ikbd.c:1100`).
- **Burst guard — at most one repeat per poll.** EmuTOS would push one word
  per `kb_repeat` ticks into the queue while the key is held, regardless of
  whether anybody reads. With polling NewTOS cannot know *when* within a gap a
  key was released — a break seen at the next poll may have happened right
  after the previous one — so crediting a long gap in full would queue
  repeats for time the key may not have been held, i.e. exactly the ghost keys
  this binding exists to avoid. A poll therefore emits at most one repeat; the
  remaining elapsed steps are discarded and the next repeat is due
  `kb_repeat` ticks after that poll. While the pump runs at least once per
  tick (every AES wait, every blocking read, every `Bconstat` on an empty
  queue) the cadence is EmuTOS' exactly; only a caller that pumps less often
  than every `kb_repeat` ticks sees the repeat rate drop to its own poll rate.
- **`ikbdsys`-change stop (step 1)** — modelled since KEYS02 (D4): the make
  records the running process's `ikbdsys` slot (no process: the default
  `0xA00`), a 50 Hz step with a key repeating and a different value stops the
  repeat, ahead of the `conterm` test; the value is never called
  (`22-kbdvbase.md` C4).
- **Not modelled** (no counterpart on NewTOS): the keyclick (step
  5 — no keyclick on NewTOS at all), the mouse-emulation repeat (the decoder
  has no keyboard mouse emulation), Alt-numpad, and the scancode rewrite of
  `convert_scancode` (the NewTOS decoder does not rewrite scancodes; the
  re-conversion of step 6 recomputes the ASCII through the active keymap).
- **Overflow**: the repeated word goes through `console::push_rx` like any
  make; a full ring drops it, as `push_ikbdiorec` does.

---

## 3. RS-232 / MIDI / IKBD `IOREC` state (`bios/iorec.h`, `bios/serport.h`, used by `Iorec` 0x0E)

### `IOREC` — the bare input ring descriptor (`bios/iorec.h:20-27`) — **14 bytes**

| Offset | Field  | Type | Meaning |
|-------:|--------|------|---------|
| 0x00   | `buf`  | `UBYTE *` | ring buffer base |
| 0x04   | `size` | `WORD` | buffer size in bytes |
| 0x06   | `head` | `volatile WORD` | write index (ISR advances) |
| 0x08   | `tail` | `volatile WORD` | read index (consumer advances) |
| 0x0A   | `low`  | `WORD` | low-water mark (resume flow) |
| 0x0C   | `high` | `WORD` | high-water mark (assert flow) |

`Iorec(1)` (IKBD) and `Iorec(2)` (MIDI) return the address of a bare
`IOREC` (input only). `ikbdiorec` uses a **256-byte** buffer
(`low = 64`, `high = 192`); `midiiorec` uses a **128-byte** buffer
(`low = 32`, `high = 96`) (`bios/aciavecs.S`).

> **Correction vs. earlier draft:** the old doc described a 16-byte IOREC
> "with input+output ringbuf" and an IKBD buffer of 128 bytes. The bare
> `IOREC` is **14 bytes and input-only**; the output ring lives in the
> separate `EXT_IOREC` (below); the IKBD buffer is 256 bytes. See D-043.

### `EXT_IOREC` — the RS-232 record (`bios/serport.h:57-65`) — **33 bytes of fields (34 on m68k)**

`Iorec(0)` returns `rs232iorecptr`, which points at an `EXT_IOREC`:

| Offset | Field      | Type    | Meaning |
|-------:|------------|---------|---------|
| 0x00   | `in`       | `IOREC` (14) | input ring |
| 0x0E   | `out`      | `IOREC` (14) | output ring |
| 0x1C   | `baudrate` | `UBYTE` | current baud code (0..15) |
| 0x1D   | `flowctrl` | `UBYTE` | flow-control mode (0..3) |
| 0x1E   | `ucr`      | `UBYTE` | shadow of MFP/SCC UCR |
| 0x1F   | `datamask` | `UBYTE` | data-bit mask for < 8-bit chars |
| 0x20   | `wr5`      | `UBYTE` | SCC WR5 shadow (SCC only) |

(The two nested 14-byte `IOREC`s make the leading 28 bytes; the 5 trailing
`UBYTE`s end at offset 0x21, so the fields total 33 bytes and, with the
struct's 2-byte alignment on m68k, `sizeof` is 34. [XBIOS07] correction:
earlier text said "24 bytes". The `in`/`out` offsets 0x00/0x0E are contract;
the total is compiler dependent and not asserted by a measurement.)

### `bconmap_root` (`BCONMAP`, `bios/serport.h:115-119`)

| Offset | Field           | Type | Meaning |
|-------:|-----------------|------|---------|
| 0x00   | `maptab`        | `MAPTAB *` | array of per-device vector sets |
| 0x04   | `maptabsize`    | `WORD` | number of `MAPTAB` entries |
| 0x06   | `mapped_device` | `WORD` | currently-mapped device (6/7/8) |

> **Correction vs. earlier draft:** the old `_state.md` and `2c-bconmap.md`
> gave the field order as `{mapped_device, maptabsize, maptab}`. The real
> order is `{maptab, maptabsize, mapped_device}` — `maptab` first. See D-044.

### `MAPTAB` (`bios/serport.h:106-113`) — **24 bytes**

Six vectors per device, in order: `Bconstat`, `Bconin`, `Bcostat`,
`Bconout`, `Rsconf`, `EXT_IOREC *Iorec` (each 4 bytes on m68k).

### `rsconfptr`, `rs232iorecptr` (`bios/serport.c:74-75`)

Pointers to the active RS-232 `Rsconf` handler and its `EXT_IOREC`. Default
targets `rsconf1` / `&iorec1` (`bios/serport.c:1018-1019`), rewired by
`Bconmap` from the active `MAPTAB` entry. `BCONMAP_AVAILABLE =
(CONF_WITH_SCC || CONF_WITH_TT_MFP)` (`bios/serport.h:18`) — true on the
reference build (make 512), so `Bconmap` exists on every machine under it;
only a build without SCC and TT-MFP returns the function number `0x2c`
(see `2c-bconmap.md` and D-045). **[XBIOS05]** NewTOS: the one-entry table
(`maptabsize` 1, device 6) and `Iorec(0)`'s record as its `Iorec`; NewTOS has
no `rsconfptr`/`rs232iorecptr` vectors to rewire — device 1 stays the AUX
port (`2c-bconmap.md`, "NewTOS implementation").

---

## 4. Display State (`bios/screen.c`)

### Screen base + resolution shadow

| Var         | Sysvar addr | Meaning |
|-------------|------------:|---------|
| `v_bas_ad`  | `0x44E`     | logical screen base — `Logbase` returns it; `Setscreen` writes it |
| `sshiftmod` | `0x44C`     | resolution-code shadow — `Getrez` returns it when there is no video hardware; `atari_setrez` updates it |
| `vblsem`    | `0x452`     | VBL-enable semaphore (`Setscreen` clears then restores around a rez change) |
| `colorptr`  | `0x45A`     | pointer to the palette to load at the next VBL (`Setpalette` writes it) |
| `frclock`   | `0x466`     | free-running VBL frame counter (`Vsync` waits for it to change) |

`physbase()` (`bios/screen.c:1042`) reads the Shifter video-base hardware
registers (`0xFFFF8201/8203/820D`); with no Atari video it falls back to
`logbase()`. So on the NewTOS profile `Physbase() == Logbase()` until the
framebuffer driver implements real page flipping.

### Palette shadow (`palette[]`, `bios/screen.c`)

16-entry ST/STe palette shadow. `Setcolor` (0x07) reads/writes
`palette[colorNum & 0x0F]`; masks the returned old value with `0x0777`
(plain ST) or `0x0FFF` (STe/TT/Videl) (`bios/screen.c:1024-1035`). Packed
word format: bits 11-8 red, 7-4 green, 3-0 blue (4 bits/channel).
`Setpalette` (0x06) defers a 16-word load to the next VBL via `colorptr`.

> Note (STe intensity-bit order): real STe/TT hardware rotates the LSB of
> each 4-bit channel to the top for backward ST compatibility. EmuTOS stores
> and masks straight 4-bit nibbles (`0x0FFF`); the hardware nibble rotation
> is a driver detail. NewTOS maps the 4-bit channels to 8-bit VideoCore
> palette entries; the *stored/returned word* is the contract, the mapping
> is free. See `07-setcolor.md`.

### TT shifter shadow (see [`_tt-shifter.md`](_tt-shifter.md))

`TT_SHIFTER` (`0xFFFF8262`, resolution) and `TT_PALETTE_REGS`
(`0xFFFF8400`, 256×UWORD) with `has_tt_shifter`. Out of the 24-function
scope; reference only.

---

## 5. Screen Dump (`bios/xbios.c` + `bios/tosvars`, `Scrdmp` 0x14)

| Var        | Sysvar addr | Meaning |
|------------|------------:|---------|
| `dump_vec` | `0x502`     | screen-dump routine pointer; default `just_rts` (no-op), `bios/chardev.c:84` |
| `dumpflg`  | `0x4EE`     | dump-in-progress flag; `-1` = idle, `bios/bios.c:357` |

`Scrdmp` (`bios/xbios.c:490-494`) calls `dump_vec` via `protect_v`, then
resets `dumpflg = -1`. Pressing Alt-Help does `dumpflg++` (`bios/ikbd.c:596`),
which the VBL notices to trigger a dump. No `CONF_WITH_*` gate. Programs
replace `dump_vec` to redirect output.

> **Correction vs. earlier draft:** the old doc's default `dump_vec` was
> vaguely "dmpfunc"; the source initialises it to the no-op `just_rts`
> (`bios/chardev.c:84`) — EmuTOS ships **no** built-in printer dump routine.
> See D-046.

NewTOS: preserve both vars; a NewTOS default `dump_vec` may write a PNG.

---

## 6. PSG / Sound (`bios/sound.c`, `Dosound` 0x20) — reference (out of 24-function scope)

`sndtable` / `snddelay` drive the `Dosound` script from the VBL when
`CONF_WITH_YM2149` is set. Without it, `Giaccess`/`Dosound` return `0` and
`Ongibit`/`Offgibit` are no-ops (`_psg-group.md`). NewTOS profile: no YM2149.

---

## 7. Random Seed (`bios/xbios.c:419`, `Random` 0x11)

```
static ULONG rseed;   /* LCG state, seeded from hz_200 at first call */
```

On the first call (`rseed == 0`) it is seeded `(hz_200 << 16) + hz_200`
(`bios/xbios.c:423-426`); each call does `rseed = rseed*3141592621 + 1` and
returns `(rseed >> 8) & 0xFFFFFF`. **Contract:** the sequence is reproducible
from a known seed; the return is a 24-bit LONG (bits 24-31 zero).

---

## 8. VBL queue / frame clock (`bios/*`, referenced by `Vsync`/`Setpalette` timing)

| Var         | Sysvar addr | Meaning |
|-------------|------------:|---------|
| `frclock`   | `0x466`     | free-running VBL counter; `Vsync` waits for it to increment |
| `vblqueue`  | `0x456`     | pointer to the VBL handler-slot array |
| `nvbls`     | `0x454`     | number of VBL slots |
| `vblsem`    | `0x452`     | VBL-enable semaphore |

`Vsync` (0x25) reads `frclock` and busy-waits (optionally
`stop_until_interrupt`) until it changes (`bios/screen.c:1205-1228`). The
`colorptr` palette load and `dumpflg` dump also happen from the VBL. XBIOS
specs read this state but do not modify the VBL queue from their surface.

---

## 9. Concurrency

XBIOS is single-threaded under EmuTOS. Hardware register accesses (PSG,
palette, MFP) are bracketed by `set_sr(0x2700)` to lock interrupts. NewTOS
replaces SR-locking with per-device spinlocks; the trap dispatcher is
serialised per-CPU, so the ABI surface still ignores SMP.

---

## Cross-references

- Per-function specs in `docs/api/xbios/`.
- BIOS state — `docs/api/bios/_state.md`.
- GEMDOS state — `docs/api/gemdos/_state.md` (§12 shares the date/time
  globals `Settime`/`Gettime` write).
- Structures — `docs/api/structures/`.

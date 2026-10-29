# `Initmous` (XBIOS 0x00)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/mouse.c:40` (`Initmous`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `type` values, the `struct param` layout, the
> `mousevec` install/clear behaviour. Reference: the raw IKBD command bytes
> and `ikbd_writeb`.

## EmuTOS source

**Primary location:** `bios/mouse.c:40` —
`void Initmous(WORD type, struct param *param, PFVOID newvec)`, slot 0 of
`xbios_vecs[]` (`bios/xbios.c:1286`).

**Installed handler / vector:** installs into `kbdvecs.mousevec`
(`bios/mouse.c:117,120`) — the mouse-packet handler field of the `kbdvecs`
table (see `22-kbdvbase.md`).

**Helpers called:**
- `bios/ikbd.c` — `ikbd_writeb()` / `ikbd_writew()` — send command bytes to
  the IKBD ACIA.

**Headers / type definitions used:**
- `bios/mouse.h:44-54` — `struct param`; `IN_YBOT`(1)/`IN_YTOP`(0).
- `include/biosdefs.h:105` — `kbdvecs`.

**System variables / globals read:** `kbdvecs.mousevec` (indirectly).

**System variables / globals written:**
- `kbdvecs.mousevec` — set to `newvec` on success, or to `just_rts` on
  error/disable (`bios/mouse.c:117,120`).

**Build configuration affecting this function:**
- `CONF_WITH_IKBD_ACIA` — the transport (`ikbd_writeb`). On the NewTOS
  profile the IKBD command bytes are interpreted by the USB-HID translator.

---

## Source extraction summary

`Initmous` configures how the IKBD reports mouse movement, and installs the
caller's mouse-packet handler. The `type` selects the mode: 0 disables the
mouse (IKBD `0x12`); 1 sets relative mode (`0x08`, then `0x0B` + threshold);
2 sets absolute mode (`0x09` + max X/Y, `0x0C` + scale, `0x0E` + initial
X/Y); 4 sets keycode mode (`0x0A` + deltas). Any other `type` is an error.
For types 1/2/4 the command bytes are sent **only if `param != NULL`**.

After the switch, on **success** (`type != 0` and not an error), if `param !=
NULL` it also sends the Y-origin command (`0x0F` for `IN_YBOT`, `0x10` for
`IN_YTOP`) and the button-reaction command (`0x07` + `buttons`), then, if
`newvec != NULL`, installs `kbdvecs.mousevec = newvec`. On **error or
`type == 0`**, it sets `kbdvecs.mousevec = just_rts` — a dummy no-op — so
disabling the mouse (type 0) or a bad call always **drops** any previously
installed mouse handler.

There is no return value; a bad `type` is silent (it only changes the vector
to `just_rts`).

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x00
- **Stack frame on entry (m68k):** `Initmous(type, param, newvec)` =
  `xbios_v_wll(0, a, b, c)` (`include/xbiosbind.h:19`):
  ```
  +8: newvec (LONG, pointer)
  +4: param  (LONG, pointer)
  +2: type   (word)
   0: function_id (word) = 0x0000
  ```
- **Total parameter bytes:** 10 (W + L + L).
- **Native (AArch64) ABI:** ID `0x00` in `x8`; args in `x0..x2`; **`void`** —
  no return value.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `type`   | `WORD` (16) |
  | +4         | `x1`        | `param`  | `struct param *` (ptr) |
  | +8         | `x2`        | `newvec` | `PFVOID` (ptr) |

---

## Inputs

| Position | Name     | Type   | Size | Source location   | Notes |
|----------|----------|--------|------|-------------------|-------|
| arg0     | `type`   | `WORD` | 2    | `bios/mouse.c:45` | 0=disable, 1=relative, 2=absolute, 4=keycode; else error |
| arg1     | `param`  | `struct param *` | 4 | `bios/mouse.c:53` | mode parameters; `NULL` skips the config bytes |
| arg2     | `newvec` | `PFVOID` | 4  | `bios/mouse.c:116`| new mouse-packet handler; `NULL` keeps current (on success) |

### `struct param` (`bios/mouse.h:44-54`) — 12 bytes

| Offset | Field      | Type   | Used for type |
|-------:|------------|--------|---------------|
| 0x00   | `topmode`  | `UBYTE`| 1/2/4 (Y origin: `IN_YBOT`=1/`IN_YTOP`=0) |
| 0x01   | `buttons`  | `UBYTE`| 1/2/4 (button reaction) |
| 0x02   | `xparam`   | `UBYTE`| 1 (threshold) / 2 (scale) / 4 (delta) |
| 0x03   | `yparam`   | `UBYTE`| 1/2/4 |
| 0x04   | `xmax`     | `WORD` | 2 (absolute area) |
| 0x06   | `ymax`     | `WORD` | 2 |
| 0x08   | `xinitial` | `WORD` | 2 |
| 0x0A   | `yinitial` | `WORD` | 2 |

---

## Outputs

### Return value

- **Register:** none (`void`).
- **Type:** `void`.
- **Source location:** `bios/mouse.c:40` (declared `void`).

### Side effects on global state

- `kbdvecs.mousevec` set to `newvec` (success, `newvec != NULL`) or to
  `just_rts` (error / `type == 0`) (`bios/mouse.c:117,120`).

### Side effects on hardware

- Sends the IKBD command sequence for the mode (via `ikbd_writeb`/`ikbd_writew`).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `type` not in {0,1,2,4} | `bios/mouse.c:101-103` | (no code; sets `mousevec = just_rts`) |

There is no error **code** — `Initmous` is `void`. A bad `type` is signalled
only by the fact that no config bytes are sent and `mousevec` becomes
`just_rts`.

**Error paths not in Compendium:** the `type == 0`/error branch **replaces**
`mousevec` with `just_rts` (drops the handler), which the documentation does
not spell out.

---

## Verification criteria

- **Disable drops the handler:** `Initmous(0, NULL, myhandler)` sends IKBD
  `0x12` and sets `mousevec = just_rts` (the handler is **not** installed —
  type 0 always clears it).
- **Relative install:** `Initmous(1, &p, myhandler)` with `p != NULL` sends
  the relative-mode bytes and installs `mousevec = myhandler`.
- **`param == NULL` skips config, still installs:** `Initmous(1, NULL,
  myhandler)` sends no threshold bytes but (success path) installs
  `myhandler`.
- **`newvec == NULL` keeps current:** `Initmous(1, &p, NULL)` leaves
  `mousevec` unchanged (success path).
- **Bad type:** `Initmous(3, &p, myhandler)` sets `mousevec = just_rts` and
  sends no config bytes.

---

## Edge cases discovered in source

1. **`type == 0` (and any error) forces `mousevec = just_rts`.** Disabling the
   mouse or a bad `type` **removes** the current handler, regardless of
   `newvec` (`bios/mouse.c:119-120`). The old spec said the vector is "left
   untouched" — it is not for type 0.
2. **Config bytes are gated on `param != NULL`.** Types 1/2/4 with `param ==
   NULL` send **no** IKBD configuration; the IKBD keeps its previous mode
   (`bios/mouse.c:53,64,95`). The Y-origin/button bytes are likewise
   `param`-gated (`:107-115`).
3. **`newvec` is installed only on the success path.** The `if (newvec !=
   NULL) mousevec = newvec` lives inside the `retval != 0 && type != 0` branch
   (`:116-117`).
4. **Type 3 is not implemented.** It falls to `default` (error) →
   `mousevec = just_rts`. Compendium documents a type-3 ("Y=0 at bottom");
   EmuTOS treats it as an error. See D-040.

---

## Dependencies

### Requires (must be initialized / functional)

- The IKBD transport (`ikbd_writeb`), the `kbdvecs` table.

### Used by

- AES desktop init/exit, GEM apps changing mouse mode, control panel.

---

## Related data structures

- `docs/api/xbios/22-kbdvbase.md` — `kbdvecs` and `mousevec`.
- `docs/api/xbios/_state.md §1` — `kbdvecs`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents types 0/1/2
  and a type 3 that EmuTOS does not implement — see D-040.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `CONF_WITH_IKBD_ACIA` transport on Atari; NewTOS interprets the command
  bytes for the USB-HID translator.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: NewTOS interprets the IKBD command sequence to configure the
USB-HID-mouse translator (relative/absolute/keycode, scale, initial
position). Mouse packets are synthesised in IKBD format and dispatched
through `kbdvecs.mousevec` exactly as on hardware; `type == 0`/error must
still reset `mousevec` to a no-op.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the IKBD ACIA command bytes
> `ikbd_writeb` sends over the 6301 serial link) at the NewTOS profile,
> where the mouse is a **USB-HID boot mouse over xHCI/PCIe** and IKBD mouse
> packets are **synthesised**, not received. The **contract is unchanged** —
> `void`, the `type` selector {0,1,2,4}, the 12-byte `struct param`, and the
> `mousevec` install-on-success / reset-to-`just_rts`-on-disable-or-error
> rule. Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2 (Keyboard /
> mouse → USB-HID) and §3.3.

- **What the mouse actually is.** A USB boot-protocol mouse enumerated on
  the xHCI controller. Its 3-byte report `[buttons, dx, dy]` is turned into
  an IKBD **relative-mouse packet** `[0xF8|wire, dx, dy]` by
  `kernel/src/drivers/usb/hid_decode.rs:139-145` and fed to
  `kernel/src/ikbd.rs::feed`, which swaps the buttons wire→VDI and calls
  `tos_api::vdi::feed_relative` (`ikbd.rs:98-104,116-122`). So the
  *relative* mode (`type == 1`) that EmuTOS arms with `Initmous(1, …)` is
  effectively the always-on default of the USB path.
- **What `Initmous` would configure.** On the Pi the IKBD command bytes have
  no ACIA to reach; the intent (relative/absolute/keycode mode, threshold/
  scale, Y-origin, initial position, button reaction) would parameterise the
  USB→IKBD translator (`hid.rs`) and the cursor accumulator
  (`tos-api/src/vdi/mouse.rs::feed_relative`). Absolute (`type == 2`) and
  keycode (`type == 4`) modes are not modelled by the current relative-only
  synthesiser.
- **Step-2 — not wired.** `Initmous` (XBIOS `0x00`) has **no entry in the
  NewTOS XBIOS dispatcher** (`tos-api/src/xbios/mod.rs` — only `0x22`
  Kbdvbase is present); calling it returns `EINVFN` instead of configuring
  the mouse or installing `mousevec`. And because the USB decoder calls
  `feed_relative` directly rather than dispatching through
  `kbdvecs.mousevec` (which is all-zero, see `22-kbdvbase.md`), the
  "install `newvec`" and "reset to `just_rts`" contract steps are not
  implemented. The mouse *works* (cursor moves) via the direct path; the
  `Initmous` *API contract* does not. Recorded as K-series below.
- **[BIOS15] ADR-031:** `Initmous(1|2|4, param, vec)` with `vec != NULL` —
  the calls that install `mousevec` on EmuTOS (`bios/mouse.c:105-117`) —
  writes one `[hook] warning` per process (`Hook::Mousevec`, the key the
  `Kbdvbase` slot shares); the answer stays the dispatcher's function number
  (`tos-api/src/xbios/mod.rs:169@note_installer`).
- **Y direction.** The USB `+dy = down` convention is passed straight
  through (`kernel/src/drivers/usb/hid_decode.rs:143`); `ikbd.rs`/`feed_relative` add the signed delta so
  `+dy` increases `GCURY` (downward), matching the EmuTOS relative mode with
  `IN_YBOT`. See `docs/api/vdi/input/_mouse-packet.md` and `hid-mouse.md`.
- **Datasheet.** No peripheral register; transport is the VL805 xHCI
  controller — see `docs/HARDWARE-PROFILE.md` §2.

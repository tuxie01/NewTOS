# IKBD relative-mouse packet handler (`mouse_int`) — the VDI mouse input contract

> **Phase-00b extraction (for Phase-13 mouse bring-up).**
> Source: EmuTOS @ `1a7484ab` — `vdi/vdi_asm.S:231` (`_mouse_int`),
> `:187-223` (`scrn_clip`), `bios/ikbd.c:101-103` (packet + button
> constants), `bios/mouse.c` (`Initmous` relative-mode setup),
> `vdi/vdi_mouse.c:565-615` (`vdimouse_init`), `:195-231` (`gloc_key`
> terminators), `:341-352` (`vq_mouse`). Button/`cur_ms_stat` bit meanings:
> `vdi/vdi_mouse.c:183-194` (comment) + `vdi_asm.S:261-300`.
>
> This documents how the 3-byte IKBD relative-mouse packet becomes the VDI
> cursor state (`GCURX`/`GCURY`/`MOUSE_BT`/`cur_ms_stat`) read by `vq_mouse`
> (124), the locator ([`_input.md`](_input.md)) and the AES event manager
> (`evnt_button`/`evnt_mouse`). It is the input source the NewTOS IKBD
> driver must feed; the keyboard decoder already receives these packets on
> UART2 and currently discards them ([[keyboard-via-atari-ikbd]]).
>
> **Contract vs. reference** — see [`../../../COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract** (must match TOS exactly): the 3-byte packet layout, the
> `0xF8` header test, the IKBD→VDI button-bit **swap**, the `MOUSE_BT` and
> `cur_ms_stat` bit meanings, the signed dx/dy accumulate + screen clamp,
> and the button→motion→cursor vector call order. **Reference** (NewTOS
> reimplements freely): that this is an m68k ISR (`_mouse_int`), the
> register usage, and the `Initmous`/`aciavecs` wiring — NewTOS decodes the
> same bytes off its own IKBD/USB-HID path.

## The packet

`SBYTE mouse_packet[3]` (`bios/ikbd.c:110`) — all three bytes are
**signed**:

| Byte | Meaning |
|-----:|---------|
| 0 | header `0xF8` + button bits |
| 1 | relative `dx` (signed, −128..127) |
| 2 | relative `dy` (signed, −128..127) |

Header validation (`vdi_asm.S:244-248`): a packet is a relative-mouse
report iff `(packet[0] & 0xF8) == 0xF8` (i.e. header `0xF8..0xFB`). Anything
else is ignored by `mouse_int`. Constants: `MOUSE_REL_POS_REPORT = 0xF8`
(`bios/ikbd.c:101`).

## Button bits — note the swap

On the **IKBD wire** (`bios/ikbd.c:102-103`) the two low header bits are:

| Bit | Wire meaning | Constant |
|----:|--------------|----------|
| 0 (0x01) | **right** button down | `RIGHT_BUTTON_DOWN` |
| 1 (0x02) | **left** button down | `LEFT_BUTTON_DOWN` |

`mouse_int` **swaps** them into the VDI/GEM convention
(`vdi_asm.S:251-258`: isolate `& 3`, `lsr #1` shifts wire-bit0 (right) into
carry and wire-bit1 (left) down to bit0, then `bset #1` re-adds the carried
right bit at bit1). Result in `MOUSE_BT`:

| Bit | `MOUSE_BT` meaning |
|----:|--------------------|
| 0 (0x01) | **left** button down |
| 1 (0x02) | **right** button down |
| 2-4 | extended buttons 3/4/5 (`CONF_WITH_EXTENDED_MOUSE`, separate status packets — out of scope for bring-up) |

On a button-state change (`vdi_asm.S:268-300`): the previous extended bits
are preserved (`& 0xFFFC`), `MOUSE_BT` is updated, the `user_but` vector is
called, then the **change deltas** are computed — `eor` old vs new, `ror #2`
to place them in bits 6/7 — and stored into `cur_ms_stat` alongside the
live state.

### `cur_ms_stat` bit layout (`vdi_mouse.c:183-194`, corrected — see D-VDI-19)

`cur_ms_stat` (`UBYTE`) is the mouse-status snapshot the locator polls:

| Bit | Meaning |
|----:|---------|
| 0 (0x01) | left button state (0 = up) |
| 1 (0x02) | right button state (0 = up) |
| 2-4 | reserved |
| 5 (0x20) | **motion flag** (1 = moved since last poll) |
| 6 (0x40) | **left-button-changed** flag |
| 7 (0x80) | **right-button-changed** flag |

> **D-VDI-19 — the source's own `gloc_key` docstring comment
> (`vdi_mouse.c:183-194`, "6 - 0x40 Right mouse button status flag; 7 -
> 0x80 Left mouse button status flag") is backwards.** Trust the code:
> the ISR that *writes* `cur_ms_stat` computes `eor.b d0,d1` (delta =
> old-2-bit-state XOR new-2-bit-state, VDI convention bit0=left/bit1=
> right) then `ror.b #2,d1`, which rotates delta-bit0 (left-changed)
> into result-bit6 and delta-bit1 (right-changed) into result-bit7
> (`vdi_asm.S:295-297`; the ColdFire variant `lsl.l #6,d1` at `:291-293`
> shifts the same two bits into the same two positions). The reader,
> `gloc_key`, agrees: `if (cur_ms_stat & 0x40) TERM_CH = 0x20; /* ...
> left key */ else TERM_CH = 0x21; /* ... right key */`
> (`vdi_mouse.c:203-207`) — bit `0x40` (bit 6) drives the **left**
> terminator. Three independent code sites (the 68K ISR, the ColdFire
> ISR, and the C reader) agree with each other and all contradict the
> comment; per CLAUDE.md Rule 1 ("comments lie; code doesn't"), the code
> wins: **bit 6 = left-button-changed, bit 7 = right-button-changed.**

`gloc_key` (locator) consumes bits 6/7: a set change-bit yields terminator
`0x20` (left, bit 6) / `0x21` (right, bit 7), and it clears bits 6/7 with
`& 0x23` (keeping live state bits 0/1 and any reserved) after reporting
(`vdi_mouse.c:203-208`). It consumes bit 5 for motion, clearing it after
returning the point (`:221-228`).

## Position

Motion test (`vdi_asm.S:308-320`): if `dx == 0 && dy == 0`, no motion —
clear `cur_ms_stat` bit 5 and leave the coordinates unchanged. Otherwise
(`vdi_asm.S:322-365`) set bit 5 and:

```
GCURX = clip( GCURX + (WORD)(SBYTE)dx )
GCURY = clip( GCURY + (WORD)(SBYTE)dy )
```

`dx`/`dy` are **sign-extended** and **added directly** — EmuTOS does not
invert Y; the cursor moves by whatever signed delta the packet carries
(positive `dy` ⇒ `GCURY` increases ⇒ downward, given the IKBD relative mode
EmuTOS configures with `Initmous(1, {topmode 0, buttons 0, xparam 1,
yparam 1}, mouse_int)`, `vdi_mouse.c:567-572,605`).

`scrn_clip` (`vdi_asm.S:187-223`) clamps against `_DEV_TAB`:

- `GCURX` to `[0, DEV_TAB[0]]` = `[0, xres]`
- `GCURY` to `[0, DEV_TAB[1]]` = `[0, yres]`

> **`xres`/`yres` are the LAST COORDINATE, not the width and height.**
> `vdi_defs.h:94-95` aliases `xres = DEV_TAB[0]`, `yres = DEV_TAB[1]`, and
> `update_rez_dependent` fills those with `V_REZ_HZ - 1` / `V_REZ_VT - 1`
> (`vdi_control.c:203-204`). Every `xres`/`yres` quoted from EmuTOS on this
> page carries that meaning: at 640x480, `xres` is **639** and `yres` is
> **479**. Reading them as the pixel count is what produced V17 — the cursor
> parked one pixel down and right of centre on every even resolution.

`scrn_clip` is called **twice** — once before and once after `user_mot`, so
a motion vector that repositions the cursor is re-clamped
(`vdi_asm.S:357-362`).

## Vector call order (`vdi_asm.S:271-368`)

Per packet, in order: **`user_but`** (only on a button-state change) →
**`user_mot`** (only on motion, between the two clips) → **`user_cur`**
(only on motion, after saving `GCURX`/`GCURY`). These are the VDI `vex_*`
slots ([`_mouse.md`](_mouse.md)); default `user_cur = mov_cur` schedules the
VBL redraw.

## Initial state (`vdimouse_init`, `vdi_mouse.c:575-600`)

`loc_mode = val_mode = chc_mode = str_mode = 0` (request); `HIDE_CNT = 1`
(cursor initially hidden); `GCURX = xres/2`, `GCURY = yres/2` — i.e.
`(V_REZ_HZ - 1) / 2` and `(V_REZ_VT - 1) / 2`, **319/239** at 640x480 and
not 320/240 (see the box above);
`MOUSE_BT = 0`; `cur_ms_stat = 0`; `user_but = user_mot = just_rts`,
`user_cur = mov_cur`. The default arrow form is installed via
`set_mouse_form(default_mform(), &mouse_cdb)`. Relative mode is armed with
`Initmous(1, &mouse_params, mouse_int)` (`:605`).

## What `vq_mouse` (124) returns (`vdi_mouse.c:341-352`)

Atomic read (interrupts masked): `INTOUT[0] = MOUSE_BT`,
`PTSOUT[0] = GCURX`, `PTSOUT[1] = GCURY`. Already implemented in NewTOS
(`tos-api/src/vdi/mouse.rs`) but currently reads a parked state because
nothing feeds `set_state` yet. Full `vq_mouse` contract:
[`_mouse.md`](_mouse.md#vq_mouse-124--query-position--buttons).

## Verification criteria

- Packet `{0xF9, 0, 0}` (header + wire right-button-down, no motion) →
  `MOUSE_BT` bit 1 (right) set, bit 0 clear; `GCURX`/`GCURY` unchanged;
  `cur_ms_stat` bit 5 clear, bit 7 set (right changed — see D-VDI-19).
- Packet `{0xFA, 5, -3}` (wire left-button-down, motion) → `MOUSE_BT` bit 0
  (left) set; `GCURX += 5`, `GCURY += -3` then clamped; `cur_ms_stat` bit 5
  set, bit 6 set (left changed — see D-VDI-19).
- Packet `{0xFF, 0, 0}` — not a relative-mouse header? `0xFF & 0xF8 = 0xF8`
  → **is** valid (both buttons down, no motion).
- Non-mouse packet (header `& 0xF8 != 0xF8`) → ignored, no state change.
- `dx` driving `GCURX` past `xres` (= width − 1) → clamps to `xres` (no wrap).

## NewTOS implementation notes

- The IKBD driver (`kernel/src/ikbd.rs`) already receives these packets on
  UART2 (`packet_data_len(0xF8..0xFB) = 2`) and discards them. The mouse
  driver decodes header+dx+dy here instead, applies the button swap + signed
  accumulate + screen clamp above, and calls `vdi::mouse::set_state(GCURX,
  GCURY, MOUSE_BT, backend)` — the existing VDI integration point.
- Screen bounds come from the active `ScreenInfo` / raster backend, not a
  hard-coded resolution — and the backend reports `width()`/`height()`, the
  pixel **counts**, so every EmuTOS `xres`/`yres` above becomes
  `width() - 1` / `height() - 1` in NewTOS
  (`tos-api/src/vdi/mouse.rs`, `park_position`).
- `user_but`/`user_mot`/`user_cur` are the VDI `vex_*` slots
  ([`_mouse.md`](_mouse.md)); the AES installs `user_but`/`user_mot` to
  drive `evnt_button`/`evnt_mouse` — that wiring lands with Phase-13 D3.
  Preserve the button→motion→cursor call order and the double `scrn_clip`.
- The `cur_ms_stat` bit-6/7 change flags are what the locator's terminator
  logic reads — reproduce them so `vrq_locator`/`vsm_locator` report the
  documented `0x20`/`0x21` terminators.
- Extended buttons 3-5 + wheel (`CONF_WITH_EXTENDED_MOUSE`) arrive as
  separate IKBD status packets, not in this 3-byte report — deferred.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the IKBD 6301 relative-mouse packet
> arriving over the ACIA) at the NewTOS profile, where the packet is
> **synthesised from a USB-HID boot-mouse report** and fed to the same
> decoder. The **contract is unchanged** — the 3-byte `[0xF8|buttons, dx,
> dy]` layout, the `0xF8` header test, the IKBD-wire→VDI **button swap**, the
> signed dx/dy accumulate + screen clamp, and the `MOUSE_BT` bit meanings.
> Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2 (Keyboard / mouse →
> USB-HID) and §3.3.

- **Where the packet comes from.** A USB boot-protocol mouse on the xHCI
  controller. Its report `[buttons, dx, dy, wheel]` (byte 0 buttons: bit0
  left, bit1 right, bit2 middle; bytes 1/2 signed dx/dy) is polled by
  `kernel/src/drivers/usb/mod.rs::poll` (`:136-143`) and converted by
  `kernel/src/drivers/usb/hid.rs::mouse_report` (`kernel/src/drivers/usb/hid.rs:66-71`, `kernel/src/drivers/usb/hid_decode.rs:139-145`) into the exact
  IKBD relative-mouse packet `[0xF8|wire, dx, dy]`, which it pushes byte by
  byte through `kernel/src/ikbd.rs::feed`.
- **The double button swap (verify the chain).** USB byte-0 is `bit0 = left,
  bit1 = right`. `kernel/src/drivers/usb/hid_decode.rs:140` swaps it
 into **IKBD wire order** `bit0 =
  right, bit1 = left` (`wire = ((usb&1)<<1) | ((usb&2)>>1)`). Then
  `ikbd.rs:98-104` swaps wire→VDI (`gem = ((wire>>1)&1) | ((wire&1)<<1)`) so
  `MOUSE_BT` ends `bit0 = left, bit1 = right`. The two swaps compose to
  identity — the net effect is USB-left → VDI-bit0 — matching the contract
  above and `hid-mouse.md`. The middle button and wheel (USB byte 0 bit 2,
  byte 3) are dropped (no Atari equivalent in the 3-byte packet).
- **Packet assembly + accumulate.** `ikbd.rs::feed` runs the same state
  machine the contract describes: header `0xF8..0xFB` → collect `dx` then
  `dy` (`ikbd.rs:69-107`), then `mouse_event` (`:116-122`) calls
  `tos_api::vdi::feed_relative(dx, dy, buttons, VDI_SCREEN)`
  (`tos-api/src/vdi/mouse.rs:126-134`), which adds the **sign-extended**
  deltas onto `GCURX`/`GCURY` and **clamps** to `[0, width-1] × [0,
  height-1]` from the live `RasterBackend` — the `scrn_clip` equivalent, with
  screen bounds from the framebuffer, not a hard-coded resolution.
- **Y direction.** USB `+dy = down` is passed through unchanged
  (`kernel/src/drivers/usb/hid_decode.rs:143`) and added
 directly to `GCURY`, so `+dy` moves the cursor
  down — the EmuTOS relative-mode / `IN_YBOT` behaviour. No inversion is
  applied.
- **Correction to the doc's premise.** The preamble/notes above say the IKBD
  driver "already receives these packets on **UART2** and currently discards
  them" — that reflects the shelved native-ST-keyboard plan. On the Pi the
  packets come from **USB-HID** (the `ikbd_uart` driver was removed;
  `ikbd.rs:8-12`), and they are **not discarded** — `feed_relative` moves the
  cursor. The synthetic-packet path is verified (the headless directory-window
  test; `hid-mouse.md`).
- **Step-2 gaps against the full contract.** `feed_relative`/`set_state`
  update `GCURX`/`GCURY`/`MOUSE_BT` and redraw the sprite, but they do **not**
  maintain `cur_ms_stat` (motion bit 5, button-change bits 6/7) and do **not**
  call the `user_but`/`user_mot`/`user_cur` exchange vectors in the
  button→motion→cursor order (`tos-api/src/vdi/mouse.rs:105-134`). So the
  locator's `0x20`/`0x21` button terminators and the AES `evnt_button`/
  `evnt_mouse` wiring driven by those vectors are not yet fed by the mouse
  packet. Extended buttons 3-5 + wheel are dropped at `hid.rs`. Recorded as
  K-series in `docs/STEP2-VALIDATION.md`.
- **Datasheet.** No peripheral register; transport is the VL805 xHCI
  controller on the Pi-4 PCIe root complex (§2).

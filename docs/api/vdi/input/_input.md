# VDI logical input devices — locator / valuator / choice / string (Phase 00b D6)

> Source: EmuTOS @ `1a7484ab`. Verified against:
> `vdi/vdi_mouse.c:280` (`vdi_v_locator`), `:195` (`gloc_key`),
> `vdi/vdi_input.c:31` (`vdi_v_choice`), `:79` (`vdi_v_string`), `:118`
> (`vdi_vsin_mode`), `:154` (`vdi_vqin_mode`), `:199` (`gchc_key`), `:215`
> (`gchr_key`); globals `vdi/vdi_defs.h:258` (`loc_mode`/`val_mode`/
> `chc_mode`/`str_mode`), `:272` (`TERM_CH`). Opcode numbers from
> [`../_dispatcher.md`](../_dispatcher.md) jump table 1.
> Calling convention: [`../_dispatcher.md`](../_dispatcher.md).
> `vqin_mode` (115) / `vq_key_s` (128) are inquiries, documented in
> [`../inquiry/_color-pixel-input.md`](../inquiry/_color-pixel-input.md);
> summarised here because `vqin_mode` reads the same mode globals.
>
> **Contract vs. reference** — see [`../../../COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract** (must match TOS exactly): the opcode numbers, the
> request-vs-sample model, the `INTIN`/`PTSIN` inputs, the `INTOUT`/
> `PTSOUT`/`CONTRL[2]`/`CONTRL[4]` outputs and their terminator/count
> encodings, the four mode globals' `0`/`1` meaning, and the dummy nature
> of `v_choice`/`v_valuator`. **Reference** (NewTOS reimplements freely):
> `gloc_key`/`gchr_key` reading `Bconstat(2)`/`Bconin(2)` + `cur_ms_stat`,
> and how the event source is polled — NewTOS drives it from the USB-HID /
> IKBD path.

The VDI defines four **logical input devices**, each with a **REQUEST**
handler (block until a trigger) and a **SAMPLE** handler (poll, return
immediately). The C bindings `vrq_*`/`vsm_*` are *not* separate opcodes:
each device is one opcode, and the current **input mode** (set by
`vsin_mode`, op 33) selects request vs. sample inside the handler.

| Device | Opcode | Fn | Request binding | Sample binding | Mode global |
|--------|:------:|----|-----------------|----------------|-------------|
| locator | 28 | `v_locator` (`vdi_mouse.c:280`) | `vrq_locator` | `vsm_locator` | `loc_mode` |
| valuator | 29 | *(no handler — `vdi_v_nop`)* | `vrq_valuator` | `vsm_valuator` | `val_mode` |
| choice | 30 | `v_choice` (`vdi_input.c:31`) | `vrq_choice` | `vsm_choice` | `chc_mode` |
| string | 31 | `v_string` (`vdi_input.c:79`) | `vrq_string` | `vsm_string` | `str_mode` |
| set mode | 33 | `vsin_mode` (`vdi_input.c:118`) | — | — | (writes all four) |

**Mode-global convention (contract).** Each global holds `mode − 1`:
`0` = **REQUEST**, `1` = **SAMPLE** (`vdi_input.c:122-141`). All four
default to `0` (request) at mouse init (`vdi_mouse.c:575-578`). `vqin_mode`
returns the stored value (`0`/`1`), not the documented `1`/`2` — this is a
known Atari-TOS quirk, see the inquiry doc.

---

## `vsin_mode` (33) — SET INPUT MODE

**Primary location:** `vdi/vdi_input.c:118` (`vdi_vsin_mode`).
**Build config:** none (configuration-independent).

### Parameter-block interface

| Slot | Meaning |
|------|---------|
| `CONTRL[0]` | opcode = 33 |
| `INTIN[0]` | device: 1 = locator, 2 = valuator, 3 = choice, 4 = string |
| `INTIN[1]` | mode: 1 = request, 2 = sample |
| `CONTRL[3]` (`N_INTIN`) | 2 |
| `CONTRL[4]` (`N_INTOUT`) | 1 (dispatcher pre-load) |
| `INTOUT[0]` | echo of `INTIN[1]` (the requested mode, **1 or 2**) |

### Behaviour

Reads `INTIN[1]` into `INTOUT[0]` (echoes the raw 1/2 mode), then stores
`INTIN[1] − 1` into the mode global selected by `INTIN[0]`
(`vdi_input.c:122-141`). A device selector outside `1..4` stores nothing
(the `switch` has no `default`) but still echoes `INTOUT[0]`. No range
check on the mode value: `INTIN[1]` values other than 1/2 are stored
verbatim as `INTIN[1] − 1`.

### Verification criteria

- `vsin_mode(dev=1, mode=2)` → `INTOUT[0] = 2`, `loc_mode = 1` (sample).
- `vsin_mode(dev=4, mode=1)` → `INTOUT[0] = 1`, `str_mode = 0` (request).
- `vsin_mode(dev=5, mode=1)` → `INTOUT[0] = 1`, no mode global changed.

---

## `v_locator` (28) — LOCATOR INPUT (mouse position + terminator)

**Primary location:** `vdi/vdi_mouse.c:280` (`vdi_v_locator`).
**Helpers:** `vdi/vdi_mouse.c:195` `gloc_key()` — poll one locator event;
`vdi/vdi_mouse.c:127` `dis_cur()` / `:157` `hide_cur()` — show/hide the
cursor around a request.
**Globals read:** `loc_mode`; `GCURX`/`GCURY`; `cur_ms_stat`; `TERM_CH`.
**Globals written:** `GCURX`/`GCURY` (from `PTSIN`); `TERM_CH`;
`cur_ms_stat` (button-change bits cleared by `gloc_key`).
**Build config:** none.

### Parameter-block interface

| Slot | Meaning |
|------|---------|
| `CONTRL[0]` | opcode = 28 |
| `CONTRL[1]` (`N_PTSIN`) | 1 (one input point) |
| `PTSIN[0]`, `PTSIN[1]` | initial locator x, y → seed `GCURX`/`GCURY` |
| `CONTRL[2]` (`N_PTSOUT`) | out: 1 if a point is returned, else 0 |
| `CONTRL[4]` (`N_INTOUT`) | out: 1 if a terminator is returned, else 0 |
| `INTOUT[0]` | out: terminating character (`TERM_CH & 0xff`) |
| `PTSOUT[0]`, `PTSOUT[1]` | out: locator x, y |

Jump-table pre-load is `{nptsout=0, nintout=0}` (`vdi_main.c`): the handler
sets `CONTRL[2]`/`CONTRL[4]` itself.

### Behaviour — request (`loc_mode == 0`)

Seed `GCURX`/`GCURY` from `PTSIN[0..1]`, show the cursor (`dis_cur`), then
**loop until `gloc_key()` returns a button/key event** (its bit 0 set)
(`vdi_mouse.c:289-301`). On return: `INTOUT[0] = TERM_CH & 0xff`,
`CONTRL[4] = 1`, `CONTRL[2] = 1`, `PTSOUT[0..1]` = the seeded input point,
then hide the cursor (`hide_cur`).

- **First-call quirk / terminators.** `gloc_key` treats a mouse button
  press *or* release as a trigger, returning `TERM_CH = 0x20` for the left
  button, `0x21` for the right, or the keyboard char (ASCII in bits 0-7,
  scancode in 8-15; `INTOUT[0]` keeps only the low byte). Because press and
  release are separate events, a single click satisfies **two** successive
  `vrq_locator` calls; `0x20`/`0x21` are also indistinguishable from
  pressing space / `!`. (`vdi_mouse.c:203-216`, header comment
  `:240-254`.)
- **Returned point.** `PTSOUT` is the *input* point (`point->x/y`), **not**
  the live `GCURX`/`GCURY` — EmuTOS notes this differs from the Atari docs
  (`vdi_mouse.c:277-278`).

### Behaviour — sample (`loc_mode != 0`)

Seed `GCURX`/`GCURY` from `PTSIN`, call `gloc_key()` **once**
(`vdi_mouse.c:302-313`):

- bit 0 set (button/key) → `CONTRL[4] = 1`, `INTOUT[0] = TERM_CH & 0xff`;
- bit 1 set (motion) → `CONTRL[2] = 1`, `PTSOUT[0..1]` = input point;
- neither → both counts left 0 → the call returns "no event".

`gloc_key` returns `1` (button/key), `2` (motion), or `3` (both); a pending
keyboard char is fetched via `Bconstat(2)`/`Bconin(2)` (**reference** event
source).

### Verification criteria

- Sample, no event pending → `CONTRL[2] = CONTRL[4] = 0` (returns
  immediately, nothing in `INTOUT`/`PTSOUT`).
- Sample, left button change → `CONTRL[4] = 1`, `INTOUT[0] = 0x20`.
- Request, left button press → blocks, then `INTOUT[0] = 0x20`,
  `CONTRL[2] = CONTRL[4] = 1`, `PTSOUT[0..1] = PTSIN[0..1]`.

---

## `v_valuator` (29) — VALUATOR INPUT (not implemented)

**Primary location:** none — opcode 29 maps to `vdi_v_nop` in jump table 1
(`vdi_main.c`; see [`../_dispatcher.md`](../_dispatcher.md)), i.e. it does
nothing. The Atari ST has no analogue valuator device.

`vsin_mode`/`vqin_mode` still track `val_mode` for ABI completeness, but a
`vrq_valuator`/`vsm_valuator` call reaches no handler: the dispatcher's
pre-loaded `{nptsout=0, nintout=0}` stand, and the call returns with empty
output arrays. This is an **honest unimplemented**, not a stub — the device
genuinely does not exist. (Contract: a program calling op 29 observes zero
output counts.)

---

## `v_choice` (30) — CHOICE INPUT (dummy device)

**Primary location:** `vdi/vdi_input.c:31` (`vdi_v_choice`).
**Helper:** `vdi/vdi_input.c:199` `gchc_key()`.
**Globals written:** `TERM_CH` (set to 1 by `gchc_key`).
**Build config:** none.

### Parameter-block interface

| Slot | Meaning |
|------|---------|
| `CONTRL[0]` | opcode = 30 |
| `CONTRL[4]` (`N_INTOUT`) | 1 (dispatcher pre-load) |
| `INTOUT[0]` | out: choice value (`TERM_CH & 0xff`) |

### Behaviour

**This is a dummy function in both Atari TOS and EmuTOS** (the imported DRI
source; the "GEM Programmer's Guide: VDI" marks the choice device as not
required). `vdi_v_choice` ignores `chc_mode` entirely — there is **no
request/sample branch**. It calls `gchc_key()`, which unconditionally sets
`TERM_CH = 1` and returns, then writes `INTOUT[0] = TERM_CH & 0xff = 1`
(`vdi_input.c:31-36,199-203`). So `v_choice` always returns choice value
**1** immediately, whatever the mode or keyboard state.

### Verification criteria

- `v_choice` in any mode → `INTOUT[0] = 1`, `CONTRL[4] = 1` (dispatcher
  pre-load), returns immediately without blocking.

> **Correction vs. earlier revision:** a prior version of this doc
> described `v_choice` as reading function keys and honouring
> request/sample via `gchc_key()`. That is wrong — `gchc_key` is a stub
> returning constant `1`; `v_choice` is a DRI dummy. Logged as D-VDI-10.

---

## `v_string` (31) — STRING INPUT (keyboard)

**Primary location:** `vdi/vdi_input.c:79` (`vdi_v_string`).
**Helper:** `vdi/vdi_input.c:215` `gchr_key()` — fetch one char via
`Bconstat(2)`/`Bconin(2)`; sets `TERM_CH` (scancode in 8-15, ASCII in 0-7).
**Globals written:** `TERM_CH`.
**Build config:** none.

### Parameter-block interface

| Slot | Meaning |
|------|---------|
| `CONTRL[0]` | opcode = 31 |
| `INTIN[0]` | max length `j` (see sign rule below) |
| `INTIN[1]` | echo flag — **ignored** (`vdi_input.c:62,77`) |
| `CONTRL[3]` (`N_INTIN`) | 2 |
| `CONTRL[4]` (`N_INTOUT`) | out: number of characters returned |
| `INTOUT[0..count-1]` | out: the characters (one per WORD) |

The string is the logical keyboard device. The max-length sign selects the
character encoding (`vdi_input.c:83-88`):

- `j ≥ 0` → `mask = 0x00ff`: **ASCII only**, one char per `INTOUT` word,
  high byte 0.
- `j < 0` → `j = |j|`, `mask = 0xffff`: **extended codes**, scancode in the
  high byte, ASCII in the low byte.

### Behaviour — request (`str_mode == 0`)

`TERM_CH = 0`; read chars in a `for` loop until `i` reaches `j` **or** the
last char's low byte is CR (`0x0d`) (`vdi_input.c:89-97`). Each iteration
**busy-waits** `while (gchr_key() == 0);` for a keystroke, then stores
`INTOUT[i] = TERM_CH & mask`. If terminated by CR, `i` is decremented so
**the CR is not stored and not counted** (`:95-96`). `CONTRL[4] = i` = the
character count.

- **Source comment claims an "Atari TOS bug" that a line-by-line trace does
  not confirm — flagged, not silently accepted (STOP, needs empirical
  check).** The doc-comment above `vdi_v_string` (`vdi_input.c:63-65`)
  states: *"Atari TOS bug: when the maximum is specified as negative,
  carriage returns do NOT terminate input; input is only terminated by
  the maximum number of characters being reached."* Tracing the actual
  loop (`vdi_input.c:89-97`) against this claim:
  ```c
  for (i = 0; (i < j) && ((TERM_CH & 0x00ff) != 0x000d); i++) {
      while (gchr_key() == 0);
      INTOUT[i] = TERM_CH = TERM_CH & mask;
  }
  if ((TERM_CH & 0x00ff) == 0x000d)
      --i;
  ```
  The CR test is **hard-coded to `& 0x00ff`** — it is *not* `& mask`. Since
  both `mask` values (`0x00ff` ASCII-only and `0xffff` extended) leave bits
  0-7 unchanged, `TERM_CH & 0x00ff` reads the same ASCII byte regardless of
  which mode is active (`gchr_key`, `vdi_input.c:215-227`, always puts
  ASCII in bits 0-7 and scancode in bits 8-15, independent of `mask`).
  Manually tracing `j=-3` (extended mode) with keystrokes `'A'` then
  `<CR>`: iteration 0 stores `INTOUT[0]=0x1E41` (scancode 0x1E, ASCII
  `'A'`=0x41), loop continues (`0x41 != 0x0d`); iteration 1 stores
  `INTOUT[1]=0x1C0D` (scancode 0x1C, ASCII `0x0d`) and sets `TERM_CH =
  0x1C0D`; the **next** loop-condition check evaluates `TERM_CH & 0x00ff =
  0x0d`, which is **false** → the loop **does** terminate on CR, and the
  post-loop `if` decrements `i` so the CR is excluded from
  `CONTRL[4]`, identically to ASCII mode. This appears to contradict the
  comment's claim for the *current* C reimplementation (plausible
  explanation: real Atari ROM TOS may have compared the *full* `TERM_CH`
  word against `0x000d` without an explicit low-byte mask — which *would*
  produce the described bug in extended mode, since a scancode-populated
  high byte makes the full word never equal `0x000d` — and EmuTOS's
  from-scratch C rewrite, using `& 0x00ff` on both sides, does not
  reproduce that specific ROM behaviour even though its own comment still
  describes it). **Not resolved here** — flagged for the lead/Developer:
  either (a) confirm empirically (build EmuTOS, run `vrq_string` with
  `INTIN[0]<0` and check whether CR terminates), or (b) treat this
  spec bullet as unverified and keep NewTOS's behaviour aligned with
  the **traced code** (CR terminates and is excluded in both modes)
  rather than the comment, per CLAUDE.md Rule 1. Logged as a candidate
  discrepancy, not yet assigned a D-VDI number pending confirmation.

### Behaviour — sample (`str_mode != 0`)

Non-blocking drain: `while (gchr_key() != 0 && i < j) INTOUT[i++] = TERM_CH
& mask;` (`vdi_input.c:100-103`). Returns immediately if no char is
pending. **CR is treated like any other character** — it is stored and does
not terminate (`vdi_input.c:70-74`). `CONTRL[4] = i` = count drained.

### Verification criteria

- Request, `INTIN[0] = 4`, user types `AB<CR>` → `INTOUT = ['A','B']`,
  `CONTRL[4] = 2` (CR dropped, not counted).
- Request, `INTIN[0] = 2`, user types `ABC` → stops at 2, `INTOUT =
  ['A','B']`, `CONTRL[4] = 2`.
- Sample, nothing pending → `CONTRL[4] = 0`, returns immediately.
- Sample, `INTIN[0] = -3`, one key pending → `INTOUT[0]` carries scancode
  in bits 8-15 + ASCII in 0-7.

---

## Dependencies

### Requires (must be functional)

- **Keyboard/console input queue** — `gloc_key`/`gchr_key` read it through
  `Bconstat(2)`/`Bconin(2)` (BIOS device 2 = keyboard). NewTOS supplies
  this from the Phase-06 console + IKBD/USB-HID path.
- **Mouse button/motion state** — `cur_ms_stat`, `GCURX`/`GCURY`,
  `TERM_CH`, maintained by the mouse packet handler
  ([`_mouse-packet.md`](_mouse-packet.md)).

### Used by

- `vrq_*`/`vsm_*` C bindings map onto these opcodes with the mode preset by
  `vsin_mode`.
- `v_dspcur`/`v_rmcur` (escape sub 18/19) reuse the cursor show/hide, not
  the locator (see [`../_escape.md`](../_escape.md)).

---

## Related data structures

- Input-mode / terminator / button constant tables →
  [`_mouse-packet.md`](_mouse-packet.md).
- Shared input globals (`loc_mode` … `TERM_CH`, `GCURX`/`GCURY`) →
  [`../_state.md`](../_state.md#input-and-cursor-state).

---

## Cross-references

- **Atari Compendium / Profibuch:** cross-check of the documented
  `vrq_*`/`vsm_*` ABIs, the first-call locator behaviour, and the
  documented (1/2 vs. 0/1) `vqin_mode` return is **deferred** — no PDFs in
  `references/` (see [`../_discrepancies.md`](../_discrepancies.md)
  preamble).
- **XBIOS:** the locator/button events originate from the IKBD mouse
  configured by `Initmous` ([`../../xbios/00-initmous.md`](../../xbios/00-initmous.md));
  keyboard events from `Kbdvbase` / `Bconin(2)`.

---

## Configuration / version notes

- None of the four handlers is gated by `CONF_WITH_*`. The dummy
  `v_choice`/`v_valuator` and the negative-`j` CR bug are present across all
  EmuTOS build configurations and match Atari TOS.

## NewTOS implementation notes

- All four bind to the **keyboard/mouse input subsystem** (`gloc_key`/
  `gchr_key`/`gchc_key` are EmuTOS's event pumps). Until that subsystem is
  wired for QEMU, REQUEST-mode calls must block on the real event queue (as
  console input already does, Phase 06), not return canned values;
  SAMPLE-mode returns "no event" cleanly (zero output counts).
- `v_choice` (dummy) and `v_valuator` (absent) return their fixed results
  without touching the input queue — reimplement them as such, not as
  faked keyboard reads.
- Char/scancode encoding (`mask` 0xff vs 0xffff) matches the BIOS keyboard
  codes; reuse the Phase-06 keyboard decode. Preserve the negative-`j` CR
  bug for bit-exact `vrq_string` compatibility (or document a deliberate
  deviation).

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the four logical input devices reading
> the IKBD keyboard queue via `Bconstat(2)`/`Bconin(2)` and the mouse
> `cur_ms_stat`) at the NewTOS profile, where the keyboard is a **USB-HID
> keyboard** feeding the console key ring and the locator's mouse state comes
> from the **USB-HID mouse**. The **contract is unchanged** — the opcode
> numbers, request/sample model, the mode globals, and the terminator/count
> encodings. Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2, §3.3.

- **Where the events come from.** `vsin_mode`/`vqin_mode` track the four
  per-device modes (`tos-api/src/vdi/input.rs:23-47`). The keyboard-reading
  devices — `v_string` (31), `v_choice` (30, dummy → key), and `v_locator`
  (28) REQUEST terminator — read one char via
  `host().console_read_byte_blocking()` (cite stale 2026-10-26: tos-api/src/vdi/input.rs:56-58), which on the Pi
  is the console RX ring filled by the **USB-HID keyboard**
  (`kernel/src/host_env.rs:83-147` → `console::rx_pop`). `v_locator`'s
  position comes from `mouse::position()` (cite stale 2026-10-26: tos-api/src/vdi/input.rs:65) = the live
  `GCURX`/`GCURY` moved by the USB-HID mouse.
- **Locator sample vs request.** SAMPLE returns the live cursor position
  immediately; REQUEST blocks for a key and returns it as the terminator
  (cite stale 2026-10-26: tos-api/src/vdi/input.rs:64-73). Because the current mouse path does not maintain
  `cur_ms_stat` button-change bits (see `_mouse-packet.md`), the locator
  does **not** yet produce the `0x20`/`0x21` mouse-button terminators — only
  keyboard terminators. Recorded as K-series.
- **Step-2 — UART leakage into the input queue.** The console RX ring these
  devices read is fed **both** by the USB-HID keyboard and by the PL011 UART0
  (`kernel/src/console.rs:91-107`, polled from `host_env.rs:112,120`). Per
  the console policy (`HARDWARE-PROFILE.md` §3.2) the TOS console/input must
  not depend on the UART; the UART bytes reaching `v_string`/`v_locator` are
  the same policy issue flagged for the console. Recorded as K1.
- **`v_choice` (dummy) / `v_valuator` (absent).** Match the contract:
  `v_choice` returns a fixed value from a blocking key read in REQUEST, 0 in
  SAMPLE (cite stale 2026-10-26: tos-api/src/vdi/input.rs:77-85) — note
 this reads the keyboard rather than
  returning the constant `1` the EmuTOS DRI dummy does (a minor divergence,
  see K-series); `v_valuator` (op 29) is an honest unimplemented (no
  handler).
- **Datasheet.** Keyboard/mouse transport = VL805 xHCI (§2); no peripheral
  register for the logical input devices (pure software over the console
  ring + cursor state).

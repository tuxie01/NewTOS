# `Srealloc` (GEMDOS 0x15)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/umem.c:358` (`srealloc`, inside `#if CONF_WITH_VIDEL`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `< 0` query vs `>= 0` allocate split, the pointer /
> size / `NULL` / `EINVFN` result shapes, the "no VIDEL ⇒ `EINVFN`" rule.
> Reference: the `pmd.mp_mfl` last-free-block reuse, `EXTRA_VRAM_SIZE`, the
> `FREESPACE_KLUDGE`, `video_ram_addr`/`video_ram_size`.

## NewTOS status: **decision deferred**

`Srealloc` reallocates VIDEL (Falcon) screen memory. On non-VIDEL hardware
EmuTOS returns `EINVFN`, so NewTOS (no VIDEL) already matches real TOS by
returning `EINVFN` — the GEMDOS dispatcher's catch-all, a **defined** result,
not a stub. The port-vs-reimplement decision (`docs/COMPATIBILITY.md`) is
deferred until a program actually needs a RAM-backed resizable screen; the RPi4
framebuffer is managed by the VideoCore, not carved from the GEMDOS pool.

---

## EmuTOS source

**Primary location:** `bdos/umem.c:358` — `void *srealloc(long amount)`, wholly
inside `#if CONF_WITH_VIDEL`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x15` =
`srealloc`, `wparms = 2`, **only when `CONF_WITH_VIDEL`** (else `NI`)
(`bdos/bdosmain.c:200-204`).

**Helpers called:**
- `util/memset` / `bzero` — clears the new screen region on a successful
  reallocation (`bdos/umem.c:420`).

**Headers / type definitions used:**
- `include/bdosdefs.h:108-115` — `MD` (the last free-list block it manipulates).
- `include/biosext.h:49` — `EXTRA_VRAM_SIZE = 256UL` (over-allocation slack).
- `include/gemerror.h:40` — `EINVFN = -32`.
- local: `FREESPACE_KLUDGE = 256` (`bdos/umem.c:357`).

**System variables / globals read:**
- `has_videl` (`bios/machine.c:73`, `include/has.h:41`) — VIDEL present?
- `video_ram_addr`, `video_ram_size` (`bios/screen.c`) — current screen buffer.
- `pmd.mp_mfl` — the ST-RAM free list; only the **last** MD is used.

**System variables / globals written (on successful realloc):**
- The last free MD's `m_length` (shrunk/extended), `video_ram_size`,
  `video_ram_addr` (`bdos/umem.c:415-417`).

**Build configuration affecting this function:**
- `CONF_WITH_VIDEL` — **required**; without it the function and its dispatch
  entry do not exist, and trap 0x15 falls through to `EINVFN`.

---

## Source extraction summary

`Srealloc` is an undocumented Falcon-TOS call that resizes the screen buffer by
re-using the ST-RAM free block immediately below the current video RAM. It has
two modes selected by the sign of `amount`: `amount < 0` is a **query** for the
maximum screen size obtainable; `amount >= 0` **reallocates** the screen to that
size and returns a pointer to the new buffer.

If `has_videl` is false it returns `(void *)EINVFN` (the value -32 cast to a
pointer) — this is how non-Falcon systems report "not available". If VIDEL is
present but `video_ram_size == 0` it returns `NULL`.

It finds the **last** MD on `pmd.mp_mfl`. If that free block is exactly
contiguous with the current video RAM (`last->m_start + last->m_length ==
video_ram_addr`), reallocation is possible and `available = last->m_length +
video_ram_size - EXTRA_VRAM_SIZE - FREESPACE_KLUDGE` (the kludge guarantees the
MD always keeps ≥ 256 bytes so it never has to be freed). If not contiguous,
`available = video_ram_size - EXTRA_VRAM_SIZE` (clamped ≥ 0) and reallocation is
**forbidden**.

A query (`amount < 0`) returns `available` as a pointer-shaped LONG. For a real
request: if reallocation is forbidden → `NULL`; else round `amount` up to the
next 256 bytes, add `EXTRA_VRAM_SIZE`, and if it exceeds `available` → `NULL`.
Otherwise it adjusts the last free MD's length, updates
`video_ram_size`/`video_ram_addr`, zeroes the new screen, and returns the new
`video_ram_addr`.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x15
- **Stack frame on entry (m68k):**
  ```
  +2: amount (LONG)
   0: function_id (word) = 0x0015
  ```
- **Total parameter bytes:** 4 (LONG) → `wparms = 2`.
- **Native (AArch64) ABI:** ID `0x15` in `x8`; `amount` in `x0`. Result in
  `x0`: a **pointer** (full 64-bit) on success, a pointer-shaped **size** LONG
  on query, `0L`/`NULL`, or the sign-extended `EINVFN` LONG.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `amount` | `LONG` (32/`long`) |

---

## Inputs

| Position | Name     | Type   | Size | Source location   | Notes |
|----------|----------|--------|------|-------------------|-------|
| arg0     | `amount` | `long` | 4    | `bdos/umem.c:358` | `< 0` → query max size; `>= 0` → requested screen size |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (pointer / size / error)
- **Encoding:**

  | Case | Return |
  |------|--------|
  | No VIDEL (`has_videl == 0`) | `EINVFN` (-32), cast to pointer |
  | `video_ram_size == 0` | `NULL` |
  | Query (`amount < 0`) | `available` bytes (pointer-shaped LONG) |
  | Realloc OK | new `video_ram_addr` pointer |
  | Realloc forbidden (last free block not contiguous with vram) | `NULL` |
  | `amount` (rounded + extra) > `available` | `NULL` |

- **Source location:** `bdos/umem.c:365,368,400,406,412,422`.

### Side effects on global state

- On a successful realloc only: last `pmd.mp_mfl` MD `m_length` adjusted,
  `video_ram_size`/`video_ram_addr` updated, new screen zeroed. All other
  paths change nothing.

### Side effects on hardware

- Indirectly moves/clears the framebuffer contents that the video hardware
  scans out; the actual VIDEL register reprogramming happens elsewhere
  (`Setscreen`/screen code), not in `srealloc`.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| VIDEL absent | `bdos/umem.c:364-365` | `EINVFN` (-32) |
| `video_ram_size == 0` | `bdos/umem.c:367-368` | `NULL` |
| Last free block not contiguous with vram, on a realloc request | `bdos/umem.c:405-406` | `NULL` |
| Requested (rounded) size exceeds `available` | `bdos/umem.c:411-412` | `NULL` |

**Error paths not in Compendium:** `Srealloc` is Atari-undocumented; the only
negative code is `EINVFN` (no VIDEL). Every other failure is `NULL`, not a
negative error — the Falcon ABI quirk.

---

## Verification criteria

- **No VIDEL (NewTOS default):** `Srealloc(-1)` and `Srealloc(320*200)` both
  return `EINVFN` (-32); no state changes. This is the only behaviour NewTOS
  must reproduce until VIDEL support is decided.
- *(VIDEL builds, for completeness — not required on RPi4:)*
  - **Query:** `Srealloc(-1)` returns the max obtainable screen size as a LONG.
  - **Contiguous realloc:** `Srealloc(n)` with a contiguous trailing free block
    and `n + extra <= available` returns a non-NULL, zeroed buffer.
  - **Non-contiguous:** `Srealloc(n)` when the last free block is not adjacent
    to vram returns `NULL`.

---

## Edge cases discovered in source

1. **Only a contiguous trailing free block can grow vram.** If `pmd.mp_mfl`'s
   last MD does not end exactly at `video_ram_addr`, realloc is forbidden and
   `Srealloc(n)` returns `NULL`; a program must `Srealloc(-1)` first to learn
   whether resizing is possible (`bdos/umem.c:388-396`).

2. **`FREESPACE_KLUDGE = 256`.** `available` is reduced so the last free MD
   always keeps ≥ 256 bytes, avoiding the messy case of freeing the MD itself
   (`bdos/umem.c:357,389`).

3. **`EXTRA_VRAM_SIZE = 256` over-allocation.** The actual allocation is 256
   bytes more than requested, matching Atari TOS4 screen-allocation behaviour
   (`include/biosext.h:49`, `bdos/umem.c:410`).

4. **Rounded up to 256 bytes.** `amount = (amount + 255) & ~255` before adding
   the extra (`bdos/umem.c:409`).

5. **New screen is zeroed.** `bzero(video_ram_addr, video_ram_size)` — the
   screen goes black immediately on a successful resize (`bdos/umem.c:420`).

6. **Query returns a pointer-shaped LONG.** `(void *)available` — the caller
   must read it as a number, not dereference it (`bdos/umem.c:400`).

7. **`EINVFN` is returned cast to a pointer.** `(void *)EINVFN`
   (`bdos/umem.c:365`); the dispatcher passes the pointer value back as the LONG
   D0, so callers see -32.

---

## Dependencies

### Requires (must be initialized / functional)

- `has_videl` (boot detection), `video_ram_addr`/`video_ram_size` (screen init).
- `pmd.mp_mfl` — the ST-RAM free list.

### Used by

- Falcon-aware applications setting up a custom screen mode (e.g. demos
  requesting a large linear vram). Pairs with `Setscreen` (XBIOS 5).

---

## Related data structures

- `docs/api/structures/md.md` — `MD`/`MPB` (the free list it edits).
- `docs/api/gemdos/memory/4a-mshrink.md` — the generic memory shrink (related).

---

## Cross-references

### Atari Compendium

- Not present in `references/`, and `Srealloc` is Atari-undocumented anyway.
  Cross-check deferred.

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

### XBIOS

- `docs/api/xbios/05-setscreen.md` — pairs with `Setscreen` in Falcon
  boot flows.

---

## Configuration / version notes

- `CONF_WITH_VIDEL` **required**; NewTOS profile has it **disabled**, so
  `Srealloc` returns `EINVFN` unconditionally — exactly the Falcon-without-VIDEL
  behaviour. Introduced in Falcon TOS.

---

## NewTOS implementation notes

(Empty during Phase 00.)

NewTOS has no VIDEL and no RAM-carved framebuffer; the VideoCore owns the
framebuffer. `Srealloc` returns `EINVFN`, exactly matching Falcon-without-VIDEL.
Programs that detect this fall back to `Setscreen`/`Physbase`/`Logbase` on the
VideoCore framebuffer.

---

## Pi / BCM2711 binding

> Re-points the reference from **VIDEL RAM-carved screen memory** (`has_videl`,
> `video_ram_addr`, the `pmd.mp_mfl` trailing block) to the **VideoCore-owned
> framebuffer** — the screen is not carved from the GEMDOS pool at all. The
> "no VIDEL ⇒ `EINVFN`" contract is unchanged. See `docs/HARDWARE-PROFILE.md`
> §3.1, §3.6.

### No VIDEL, no RAM-backed screen → `EINVFN` is correct — but not wired yet

The BCM2711 framebuffer is a **mailbox-allocated VideoCore buffer**
(`kernel/src/drivers/bcm2711/framebuffer.rs`), not memory carved from the ST-RAM pool, so the
Falcon `Srealloc` mechanism has nothing to resize. The contract answer is
`EINVFN` (-32), exactly the Falcon-without-VIDEL behaviour.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| `has_videl` from bus probe | always false (no VIDEL) |
| screen carved from `pmd.mp_mfl` trailing block | framebuffer owned by VideoCore, outside the GEMDOS pools |
| grow/shrink vram via the last free MD | N/A — `Setscreen`/`Physbase`/`Logbase` re-point shadows, VideoCore owns scan-out |

### Current-code status (contract already met)

- **`Srealloc` (0x15) is not in the GEMDOS dispatch table**, but the GEMDOS
  dispatcher returns **`EINVFN`** for any unregistered slot
  (`tos-api/src/gemdos/mod.rs:128,132` — `None => EINVFN`). Since the contract
  answer on a no-VIDEL machine *is* `EINVFN` (-32), the fall-through **already
  produces the correct value** — unlike the XBIOS side, where an unwired slot
  yields `EINVFN` for a call whose contract wants a *different* value (cf.
  `Blitmode` wanting `0x0000`). No Step-2 fix required; an explicit 0x15 slot
  would only make the intent self-documenting.

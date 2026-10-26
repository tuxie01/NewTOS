# `Mxalloc` (GEMDOS 0x44)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/umem.c:234` (`xmxalloc`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `mode` 0-3 pool-selection semantics *as observed*
> (which pool a block comes from and the preferred/fallback order), the `-1L`
> largest-free query, the pointer / `0L` / LONG result shapes, `mode & 0x03`.
> Reference: `ffit`, the MD/MPB lists, the `pmd`/`pmdalt` split.

## EmuTOS source

**Primary location:** `bdos/umem.c:234` — `void *xmxalloc(long amount, int mode)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x44` =
`xmxalloc`, `wparms = 3` (`bdos/bdosmain.c:264`).

**Helpers called:**
- `bdos/iumem.c:35` — `ffit()` — first-fit search on a pool's `mp_mfl`; on a
  real allocation it also splits the free MD, prepends the allocated MD to
  `mp_mal`, and sets `m_own = run`. For `amount == -1L` it instead returns the
  largest `m_length` (cast to a pointer-shaped value), touching no state.

**Headers / type definitions used:**
- `include/bdosdefs.h:18-22` — `MX_STRAM=0`, `MX_TTRAM=1`, `MX_PREFSTRAM=2`,
  `MX_PREFTTRAM=3`, `MX_MODEMASK=0x03`.
- `include/bdosdefs.h:108-115` — `MD`.
- `bdos/mem.h` — `MALLOC_ALIGN_ALTRAM = 3`; `extern MPB pmd, pmdalt`.

**System variables / globals read:**
- `pmd` (`bdos/umem.c:35`) — ST-RAM MPB.
- `pmdalt` (`bdos/umem.c:37`, `CONF_WITH_ALT_RAM`) — alt-RAM (TT-RAM) MPB.
- `run` — via `ffit`, tags the new MD's `m_own`.

**System variables / globals written:**
- The selected pool's `mp_mfl`/`mp_mal` lists and the split/allocated MDs
  (`bdos/iumem.c:103-134`). Query and failure paths write nothing.

**Build configuration affecting this function:**
- `CONF_WITH_ALT_RAM` — when **0**, `pmdalt` is absent and the `MX_TTRAM` arms
  are `#if`'d out of the switch, so `mode == MX_TTRAM (1)` falls to `default →
  NULL`; `MX_PREFSTRAM`/`MX_PREFTTRAM` have no alt-RAM fallback and effectively
  become ST-RAM-only.

---

## Source extraction summary

`Mxalloc` is the general allocator that `Malloc` wraps. It takes a size and a
`mode` selecting one of two logical pools — ST-RAM (`pmd`) or alternate/TT-RAM
(`pmdalt`) — with two "preferred" modes that fall back to the other pool. The
very first thing it does is `mode &= MX_MODEMASK` (`= 0x03`), keeping only bits
0-1; any higher bits (Atari documents bits 4+ as supervisor / memory-protection
/ globally-accessible flags) are silently discarded — EmuTOS has no MMU-based
protection to honour them.

For `amount == -1L` it performs the largest-free-block **query**: `MX_STRAM`
reports `ffit(-1L,&pmd)`, `MX_TTRAM` reports `ffit(-1L,&pmdalt)`, and both
"preferred" modes report the larger of the two pools' biggest single free
blocks (verified on TOS3 per the source comment). An unknown mode returns NULL.
The query is a pure scan for `max(m_length)` on the free list — it does not
allocate and never reflects total free space, only the biggest single block.

For a normal `amount > 0`, it dispatches to `ffit` on the pool(s) implied by
the mode: `MX_STRAM` → `pmd` only; `MX_TTRAM` → `pmdalt` only; `MX_PREFSTRAM` →
`pmd`, then `pmdalt` if that returned NULL; `MX_PREFTTRAM` → `pmdalt`, then
`pmd`. On success it returns `m->m_start`; on any failure (no fitting block,
`amount <= 0`, unknown mode, or a pool that doesn't exist in this build) it
returns `NULL`. A single allocation always lives entirely in one pool — there
is no split-across-pools path.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x44
- **Stack frame on entry (m68k):**
  ```
  +6: mode   (WORD)
  +2: amount (LONG)
   0: function_id (word) = 0x0044
  ```
- **Total parameter bytes:** 6 (LONG + WORD) → `wparms = 3`.
- **Native (AArch64) ABI:** ID `0x44` in `x8`; `amount` in `x0`, `mode` in
  `x1`. Result in `x0`, LONG: a **pointer** occupies the full 64-bit `x0`; a
  **`0L`/NULL** failure is all-zero; the `-1L` **size query** returns a LONG.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `amount` | `LONG` (32/`long`) |
  | +6         | `x1`        | `mode`   | `WORD` (16) — masked to `& 0x03` |

---

## Inputs

| Position | Name     | Type   | Size | Source location   | Notes |
|----------|----------|--------|------|-------------------|-------|
| arg0     | `amount` | `long` | 4    | `bdos/umem.c:234` | bytes to allocate; `-1L` = largest-free query; `<= 0` (else) → NULL |
| arg1     | `mode`   | `int`  | 2    | `bdos/umem.c:241` | pool selector; masked with `MX_MODEMASK (0x03)` on entry |

### `mode` values (`include/bdosdefs.h:18-22`), after `& MX_MODEMASK (0x03)`

| Value | Constant       | Allocation behaviour |
|------:|----------------|----------------------|
| 0 | `MX_STRAM`     | ST-RAM (`pmd`) only |
| 1 | `MX_TTRAM`     | TT-/alt-RAM (`pmdalt`) only |
| 2 | `MX_PREFSTRAM` | ST-RAM preferred; fall back to alt-RAM |
| 3 | `MX_PREFTTRAM` | alt-RAM preferred; fall back to ST-RAM |

Bits 2-15 are cleared by the mask and cannot influence the call. Some Atari
documentation assigns bits 4+ to supervisor/memory-protection/global flags;
this EmuTOS build ignores them (no MMU protection).

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (pointer, or size, or `0L`)
- **Encoding:**

  | Case | Return |
  |------|--------|
  | `amount > 0`, success | pointer to the region (full 64-bit on native) |
  | `amount > 0`, selected pool(s) have no fitting block | `0L` (NULL) |
  | `amount == -1L` | size of the largest single free block in the selected pool(s) |
  | `amount <= 0` (and ≠ -1L) | `0L` (NULL) |
  | `mode` (after mask) not 0-3, or a needed pool absent in build | `0L` (NULL) |

- **Source location:** `bdos/umem.c:327` (`ret_value = m->m_start`),
  `:247-278` (query), `:324` (NULL).

### Side effects on global state

- Successful allocation: split/consume a free MD in the chosen pool, prepend
  the allocated MD to that pool's `mp_mal`, set `m_own = run`
  (`bdos/iumem.c:103-134`); a split consumes one MD from the OS-memory MD pool.
- Query and all failures: no state change.

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `amount > 0`, no fitting free block in selected pool(s) | `bdos/iumem.c:97-101` | `0L` (NULL) |
| `amount == 0` or `amount < -1L` | `bdos/umem.c:284-286` | `0L` (NULL) |
| `mode == MX_TTRAM` (1) with `CONF_WITH_ALT_RAM == 0` | `bdos/umem.c:315-317` (falls to `default`) | `0L` (NULL) |
| unknown masked `mode` | `bdos/umem.c:274-276,315-317` | `0L` (NULL) |
| MD-pool exhausted during split | `bdos/iumem.c:113-117` | `0L` (NULL) |

**Error paths not in Compendium:** `Mxalloc` never returns a negative GEMDOS
error code; every failure is `0L`. (NewTOS may choose to return `EINVFN` for a
genuinely-unknown mode — see notes — but EmuTOS returns NULL.)

---

## Verification criteria

- **Nominal ST-RAM alloc:** `Mxalloc(4096, MX_STRAM)` with ST-RAM room returns
  a non-NULL pointer inside the ST-RAM pool; `m_own == run`.
- **Nominal TT-RAM alloc:** `Mxalloc(4096, MX_TTRAM)` with alt-RAM present
  returns a pointer inside `pmdalt`.
- **Preferred fallback:** `Mxalloc(N, MX_PREFTTRAM)` where `pmdalt` cannot fit
  `N` but `pmd` can returns an ST-RAM pointer; the symmetric `MX_PREFSTRAM`
  falls back to alt-RAM.
- **Largest-free query (mode 0):** `Mxalloc(-1, MX_STRAM)` returns the largest
  single free ST-RAM block size and changes no state.
- **Largest-free query (preferred):** `Mxalloc(-1, MX_PREFTTRAM)` returns the
  larger of the two pools' biggest free blocks.
- **TT-RAM unavailable:** `Mxalloc(4096, MX_TTRAM)` when the build has no
  alt-RAM returns `0L`; likewise `Mxalloc(-1, MX_TTRAM)`.
- **Mode masking:** `Mxalloc(4096, 0x40 | MX_STRAM)` behaves exactly as
  `MX_STRAM` (the 0x40 bit is dropped).
- **Zero / bad amount:** `Mxalloc(0, m)` and `Mxalloc(-2, m)` return `0L`.

---

## Edge cases discovered in source

1. **`mode &= 0x03` on entry.** Any protection/supervisor bits are discarded
   before dispatch (`bdos/umem.c:241`). A program passing `mode = 0x40`
   ("globally accessible") gets `MX_STRAM` behaviour and no protection.

2. **`amount == -1L` reports one block, not total free.** For `MX_PREFSTRAM`/
   `MX_PREFTTRAM` it is the **larger** of the two pools' biggest single free
   blocks (`bdos/umem.c:257-273`), not the combined free space.

3. **No cross-pool split.** A single allocation is served entirely from one
   pool; the "preferred" modes only retry the *other* pool wholesale, they do
   not split a request across both.

4. **`MX_TTRAM` without alt-RAM → NULL.** With `CONF_WITH_ALT_RAM == 0` the
   `case MX_TTRAM` arm is compiled out, so mode 1 (alloc *and* query) hits
   `default` and returns NULL — not an ST-RAM allocation.

5. **Owner attribution.** `ffit` sets `m_own = run` (`bdos/iumem.c:134`); the
   block is freed automatically at the owner's `Pterm` via `free_all_owned`
   (`bdos/proc.c:92-101,136-139`).

6. **Pointer-shaped query value.** `ffit(-1L,…)` casts the max length to
   `(MD *)maxval` (`bdos/iumem.c:73`) and `xmxalloc` returns it directly; the
   caller must read it as a LONG size, not dereference it.

---

## Dependencies

### Requires (must be initialized / functional)

- `umem_init` (`bdos/umem.c:549`) — seeds `pmd`, `start_stram`/`end_stram`,
  `malloc_align_stram`.
- `pmdalt` seeded by one or more `Maddalt` calls (or `altram_init`) for the
  TT-RAM paths — see [`14-maddalt.md`](14-maddalt.md).
- `ffit`, `xmgetmd` (MD meta-allocator).

### Used by

- `Malloc` (0x48) — supplies the mode from `PF_TTRAMMEM`.
- AES / drivers that want a specific pool (e.g. `MX_STRAM` for DMA-safe ST-RAM,
  `MX_PREFTTRAM` for large buffers).

---

## Related data structures

- `docs/api/structures/md.md` — `MD`/`MPB`, the pool lists, `ffit`.
- `docs/api/gemdos/_state.md` §8 — the `pmd`/`pmdalt` pool model and `MX_*`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. Historically documents
  modes 0-3 and the `-1L` query — consistent with EmuTOS; the higher
  "protection" mode bits it lists are **not** honoured by this build.

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `CONF_WITH_ALT_RAM` (default 1 on the full build): without it, modes 1 and
  the alt-RAM half of 2/3 disappear (mode 1 → NULL, modes 2/3 → ST-RAM only).
- Alt-RAM allocations are always 4-byte aligned (`MALLOC_ALIGN_ALTRAM = 3`);
  ST-RAM follows `malloc_align_stram` (2 or 4 bytes) — see
  [`4a-mshrink.md`](4a-mshrink.md).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-04 (D8): two MPBs always present (ST-RAM over `Pool::Stram`, TT-RAM over
`Pool::Ttram`). `mode &= MX_MODEMASK` then dispatch 0/1/2/3 as above.
`Mxalloc(-1, mode)` is a `largest_free` scan on `mp_mfl` so callers can size
without committing. **Divergence to decide:** whether an unknown masked mode
returns `EINVFN` (cleaner) or `NULL` (EmuTOS) — the earlier draft claimed
`EINVFN`; EmuTOS returns `NULL`, so `EINVFN` would be a *deliberate* documented
divergence, not the extracted contract. **Protection bits** (Atari bits 4+)
are honoured since ARCH03 stage 3b with memory protection on (see "rights per
process" below); with `memprot=off` they are ignored, as by EmuTOS.

### NewTOS: one shared address space (ARCH03 stage 1)

Every user address is its own physical address, and both pools are entered
into the user page tables of every process at boot
(`tos-api/src/bdos/process/mmu.rs`, plan
`docs/analysis/arch03-implementation-plan.md` §3.1/§3.2, decisions E1/E18).
`Mxalloc` therefore returns the pool block's `m_start` itself — to an EL0
program and to a kernel caller alike — exactly as EmuTOS does, and the
address means the same in every process (a block can be handed to a child, an
accessory or another program by pointer). `Mxalloc(-1, mode)` is the largest
free block of the selected pool(s) (`bdos/umem.c`), with no further bound.
Host tests: `tos-api/tests/arch03_shared_space.rs`.

Until stage 1 (ADR-021) an EL0 caller got the block mapped into a per-process
"heap window" at an address ≠ the pool address, with a one-page alignment
slack, a 32-entry block table and a `-1` answer bounded by the remaining
window; all of that is gone. **Protection bits** (`mode` bits 4+): decoded
and enforced since stage 3b (next section), plan Frage 8.


### NewTOS: rights per process (ARCH03 stage 3b)

With memory protection on (the default; boot argument `memprot=off` switches
it off, FreeMiNT `MEM_PROT=NO`, `sys/bootmenu.c:207-219`) the protection
nibble of `mode` is **honoured**, with FreeMiNT's encoding (`sys/dosmem.c:
209-221`, `sys/mint/mem.h:118`): `(mode & 0xF0) >> 4` is `0` = the caller's
`PRGFLAGS` mode, otherwise one more than the `PROT_*` value — `0x10`
`PROT_P` (owner only), `0x20` `PROT_G` (everybody), `0x30` `PROT_S` (owner;
no program else), `0x40` `PROT_PR` (owner read/write, everybody read),
`0x50` `PROT_I` (nobody); above → `PROT_P`. The pool is still selected by
bits 0–1 only. A block a **kernel** caller allocates without a nibble is
`PROT_S` (FreeMiNT `sys/memory.c:150-159`). Bit `0x08` is not evaluated
(FreeMiNT has it disabled, `sys/dosmem.c:226-236`).

**`F_KEEP` (`0x4000`, ARCH03 stage 3c, FreeMiNT `sys/mint/mem.h:104-105`):**
"if the owner of this block terminates, don't free this block. Instead, let
MiNT inherit it" (`sys/dosmem.c:161-165`). The bit marks the block
(FreeMiNT `M_KEEP`, `sys/dosmem.c:131-135`); it is not part of the
protection mode (`:237-242`) and does not select a pool. When the process
holding the block's **last** owner link terminates, the block is not freed
but passes to the system (FreeMiNT attaches it to the root process,
`sys/proc_help.c:242-258`; NewTOS: the kernel owner) with its protection
mode unchanged; from then on **any** process may `Mfree` it by address
(`sys/dosmem.c:365-398`, `49-mfree.md`). While an owner lives, the block
behaves like any other (only an owner frees or shrinks it). The bit acts
with protection on and off — it is region bookkeeping, not protection
(FreeMiNT keeps `M_KEEP` with `MEM_PROT=NO` too). EmuTOS clears it (`mode &=
MX_MODEMASK`, `bdos/umem.c:241`): a sanctioned FreeMiNT extension. FreeMiNT
calls it "a special mode meant for the OS only, and may not remain available
to user processes"; NewTOS accepts it from programs, as FreeMiNT does today.
Host tests: `tos-api/tests/arch03_3c_keep.rs`.

**Observable (sanctioned FreeMiNT deviation, plan E16):** with protection on
both pools hand out whole 4-KiB pages — the length is rounded up to a page
and every block starts on a page (FreeMiNT rounds to its `QUANTUM`,
`sys/mint/mem.h:152`), so `Malloc(-1)` and the MDs `Getmpb` shows are page
multiples. With `memprot=off` the 16-byte grain of stage 3a applies.

Implementation: `tos-api/src/bdos/mem/traps.rs` `mxalloc`,
`tos-api/src/bdos/process/memprot.rs`. Host tests:
`tos-api/tests/arch03_3b_memprot.rs`; QEMU probe `sdk/examples/mprot3b.c`.

---

## Pi / BCM2711 binding

> Re-points the two-pool model from **physically distinct ST-RAM (chip-shared,
> < 14 MiB) and TT-RAM (CPU-only extension board)** to **one uniform LPDDR4
> address space split into two logical, additive pools**. The `mode` 0-3
> pool-selection semantics, the `-1` largest-free query, and the pointer/`0L`
> result shapes are unchanged; only the physical backing is re-pointed here.
> See `docs/HARDWARE-PROFILE.md` §3.6, ADR-010.

### The pools on the Pi — one DRAM, two logical partitions

There is no DMA-zone distinction on AArch64, so the ST/TT split is a **pure API
contract** kept so source-ported TOS programs still get the pool they ask for
(`kernel/src/bios/machine/layout.rs:1-44`). The two MPBs (`pmd` = ST-RAM,
`pmdalt` = TT-RAM) are seeded from the AArch64 memory map at boot
(`tos-api/src/bdos/mem/init.rs`), fed by `kernel/src/lib.rs:452-481`
(`bring_up_bdos_pools`).

| Pool | Size | Placement | Source |
|------|------|-----------|--------|
| **ST-RAM slot** | **16 MiB** window, **14 MiB usable** (top 2 MiB reserved, mirroring the Atari `$E00000-$FFFFFF` I/O window) | low end of DRAM, above the kernel image | `layout.rs:52,57,62,310-313` |
| **TT-RAM (alt-RAM) pool** | **256 MiB** on `qemu-virt` (`DRAM_SIZE - 16 MiB` on real Pi-4) | immediately above the ST-RAM slot | `layout.rs:136-139,315-318` |
| **Total managed** | ~272 MiB (`qemu-virt`) | both first-fit `Malloc` | `layout.rs:280-331` |

**Board addresses** (`layout.rs`):

| Board | DRAM_START | ST-RAM slot | TT-RAM pool | Source |
|-------|-----------:|-------------|-------------|--------|
| `board-rpi4` | `0x0000_0000` | `[kernel_end, 0x00E0_0000)` (14 MiB minus kernel) | `[0x0100_0000, DRAM_END)` | `layout.rs:165,310-318` |
| `board-qemu-virt` | `0x4000_0000` | `[kernel_end, 0x40E0_0000)` | `[0x4100_0000, 0x5100_0000)` (256 MiB) | `layout.rs:133-140,310-318` |

> **Address-width gate.** `MD::m_start`/`m_length` are `Long` (`i32`), so on
> real Pi-4 with ≥ 2 GiB DRAM the pool would overflow the Atari ABI; the seed
> path returns `AddressOutOfRange` until the Phase-05 address-window translation
> lands (`tos-api/src/bdos/mem/init.rs:14-21,226-236`). Today the managed region is capped to fit
> `i32`.

### `Mxalloc` mode → pool on the Pi

`mxalloc` (`tos-api/src/bdos/mem/traps.rs:254-340`) masks `mode & MX_MODEMASK`
and dispatches to `pmd` / `pmdalt` exactly as the contract table: `MX_STRAM`→ST
only, `MX_TTRAM`→TT only, `MX_PREFSTRAM`/`MX_PREFTTRAM`→preferred-then-other.
`amount == -1` returns the `largest_free` of the selected pool(s)
(`traps.rs:260-285`). `find_mpb` (`traps.rs:386-393`) routes an address to `pmd`
if it lies in the cached ST-RAM range, else `pmdalt`.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| ST-RAM = chip-shared DMA memory < 14 MiB | ST-RAM slot = low 14 MiB of LPDDR4 (no DMA meaning, just the address window) |
| TT-RAM = CPU-only extension board | TT-RAM = the DRAM above 16 MiB (same physical RAM, CPU-visible) |
| `MX_STRAM` for DMA-safe buffers | `MX_STRAM` returns low-DRAM addresses (API hint only; no DMA-zone guarantee) |

### Divergences / gaps vs. the contract (Step-2 candidates)

- **Unknown masked mode returns `0L` (NULL) — matches EmuTOS.** `mxalloc`
  returns `0` for a mode outside 0-3 (`traps.rs:283,331`), the extracted
  contract. **The earlier draft's `EINVFN` claim is *not* implemented** and would
  be a deliberate divergence if adopted; the code does the EmuTOS `NULL`. (Recorded
  so Step-2 does not "fix" it toward `EINVFN` by accident — that needs a decision.)
- **Alignment differs.** NewTOS uses a **16-byte** grain for both pools
  (`PoolConfig::AARCH64`) vs. EmuTOS's 2/4-byte ST-RAM and 4-byte alt-RAM.
  Unobservable except for block sizes rounded up more.
- **Protection bits (Atari 4+)** — ignored as by EmuTOS with `memprot=off`;
  honoured with FreeMiNT's encoding since ARCH03 3b (protection on).
- **≥ 2 GiB real-Pi-4 DRAM not fully managed** — capped to the `i32` ABI window
  pending Phase-05 address translation.

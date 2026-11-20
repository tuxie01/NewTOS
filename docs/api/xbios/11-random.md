# `Random` (XBIOS 0x11)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/xbios.c:421` (`random`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the 24-bit LONG result (bits 24-31 zero), the
> deterministic LCG sequence from a given seed, **and the first seed**
> `(hz_200 << 16) + hz_200` taken from the caller-visible `_hz_200` (a program
> that reads `$4BA` around its first `Random()` can predict the first value;
> probe `xsseed.c` X01 measures exactly that, see "Acceptance probe"). Reference
> (free): where `rseed` is stored.

## EmuTOS source

**Primary location:** `bios/xbios.c:421` — `static LONG random(void)`, slot
0x11 of `xbios_vecs[]` (`bios/xbios.c:1307`). Uses the file-static seed
`rseed` (`bios/xbios.c:419`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:** none.

**Headers / type definitions used:**
- `include/tosvars.h` — `hz_200` (the 200 Hz tick counter).

**System variables / globals read:**
- `hz_200` (`0x4BA`) — used to seed `rseed` on the first call.

**System variables / globals written:**
- `rseed` (`bios/xbios.c:419`) — advanced on every call.

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Random` returns a 24-bit pseudo-random number using a linear congruential
generator. At **every** call that finds `rseed == 0` (the first call, and any later call
after the LCG has stepped onto 0) it seeds `rseed = (hz_200 << 16) + hz_200`
in 32-bit unsigned arithmetic (`bios/xbios.c:423-426`; `rseed` is `ULONG`,
`bios/xbios.c:419`; `hz_200` is `volatile LONG`, `include/tosvars.h:53`).
A seed that comes out 0 (only possible for `hz_200 == 0`, because 65537 is odd
and so invertible mod 2^32; inferred) is **not** replaced: the code goes on
with `rseed = 0`, so that call computes `0*3141592621 + 1 = 1` and returns
`(1 >> 8) & 0xFFFFFF = 0` (`bios/xbios.c:427-429`). There is no
"bump zero to one" step in EmuTOS. Each call then does `rseed = rseed * 3141592621 + 1` (a 32-bit LCG,
multiplier ≈ π×10⁹) and returns `(rseed >> 8) & 0xFFFFFF` — the middle 24 bits,
with bits 24-31 always zero.

The sequence is fully deterministic given a seed, which is contract: two runs
that reach the same `rseed` produce the same sequence. The `>> 8` discards the
worst low bits (bit 0 of an LCG alternates every step), but the result is not
cryptographic. Well-behaved code that needs a specific starting sequence sets
`rseed` (historically by patching the BIOS BSS location); NewTOS does not
expose that back door.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x11
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0011
  ```
  No parameters (`Random()` = `xbios_l_v(17)`, `include/xbiosbind.h:35`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x11` in `x8`; no argument registers; the
  LONG result **sign-extended** into `x0` (top byte is 0, so the value is a
  clean 0..0xFFFFFF).

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (24-bit value)
- **Encoding:** `(rseed >> 8) & 0xFFFFFF` — bits 0-23 random, bits 24-31 zero.
- **Source location:** `bios/xbios.c:429`.

### Side effects on global state

- Advances `rseed` (and seeds it from `hz_200` on the first call).

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Random` cannot fail.

---

## Verification criteria

- **Range:** every result satisfies `0 <= r <= 0xFFFFFF` (bits 24-31 zero).
- **Determinism:** from a fixed `rseed`, the sequence of results matches the
  LCG `rseed = rseed*3141592621 + 1; return (rseed>>8)&0xFFFFFF`.
- **Auto-seed:** the first call seeds from `hz_200`, so it is not a constant
  across boots (but is a function of uptime at first call). Contract check: for
  some `h` in `[hz_200 before .. hz_200 after]` the first result equals the LCG
  step of `(h << 16) + h` (probe X01).
- **Zero seed:** `hz_200 == 0` at the first call gives `rseed = 0` then 1; the
  result is 0 and the next result is the LCG step of 1 (cited
  `bios/xbios.c:423-429`; not measured: `hz_200` is never 0 by the time a
  program runs).
- **Advances each call:** two consecutive calls return different values (for
  any non-degenerate seed).

---

## Edge cases discovered in source

1. **Seed from `hz_200`.** `rseed = (hz_200 << 16) + hz_200`
   (`bios/xbios.c:423-426`), re-done whenever `rseed` is 0 at entry; early after boot `hz_200` is small, so the
   effective entropy is low (~16 bits). AES/games often mix `hz_200`
   themselves for this reason.
2. **24-bit result, LCG weaknesses.** The `>> 8` masks the worst low bits, but
   sequential outputs remain linearly correlated; never use for security.
3. **Seed is internal.** `rseed` is a BIOS static; the classic "poke the seed"
   trick relies on a fixed BSS offset. NewTOS keeps the LCG for determinism
   but does not expose the poke; a reseed is a NewTOS extension.

---

## Dependencies

### Requires (must be initialized / functional)

- `hz_200` running (for the seed).

### Used by

- Games, AES timer jitter, screen-saver pattern generation, sample shuffling.

---

## Related data structures

- `docs/api/xbios/_state.md §7` — `rseed`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the 24-bit
  return.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The multiplier `3141592621` and the `>> 8`/`& 0xFFFFFF` shaping match
  classic TOS, so the sequence is bit-identical to real hardware for a given
  seed.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: NewTOS keeps this exact LCG for ABI determinism. Higher-quality
entropy (AArch64 `RNDR`) is exposed via a separate NewTOS-extension slot, not
by changing `Random`.

---

## Pi / BCM2711 binding

> The 24-bit result, the LCG recurrence and the first seed are the EmuTOS
> contract above. `_hz_200` exists on NewTOS (`bios/tick.rs`, see
> [`docs/api/bios/06-tickcal.md`](../bios/06-tickcal.md)); the 60 Hz frame clock
> is **not** a seed source.

### Current code (XBIOS11)

`Random` is dispatched (`tos-api/src/xbios/mod.rs:116@RANDOM`) to `system::random`
(`tos-api/src/xbios/system.rs:24-29@random`), which keeps the state in its own static
and runs one step `random_step(rseed, H)` with `H` = the kernel counter
`bios::tick::hz_200()` (`tos-api/src/xbios/system.rs:297@random_step`). The step seeds
a zero state as `(H << 16) + H` in `u32` wrapping arithmetic, without substituting a
zero seed (`tos-api/src/xbios/system.rs:298-302@random_step`), then applies the EmuTOS
LCG and returns `(s >> 8) & 0xFFFFFF` as a non-negative `i64`
(`tos-api/src/xbios/system.rs:304-305@wrapping_mul`): the four required behaviours below,
as `bios/xbios.c:423-429`. No frame clock is read. Until XBIOS11 the first seed came
from the 60 Hz frame clock and a zero seed was bumped to 1 (X01 `no` on NewTOS with
`hz_200` 3024 and frame clock 908; run
`measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-NEWTOS-raspi4b/XSSEED.LOG`).

Host tests: the unit tests of `random_step` (`tos-api/src/xbios/system.rs:309@tests`: seed
from the counter, including the oracle's measured pair `hz_200` 2573 → `7d9272`, 32-bit
wrap of the seed, zero seed kept, a state stepped onto 0 re-seeded, the 24-bit result)
and `tos-api/tests/xbios11_random_seed.rs` (the first `Random` through the XBIOS
dispatcher after `bios::tick::advance(2573)` gives `7d9272`).

### Required behaviour

1. Seed value: `(H << 16) + H` in `u32` wrapping arithmetic, `H` = the 200 Hz
   counter as a `u32`. No frame clock anywhere in `Random`.
2. No zero-seed substitution: a seed of 0 goes straight into
   `s = s*3141592621 + 1` (result 0, state 1).
3. The `rseed == 0` test and the seeding stay at the top of every call, as in
   EmuTOS (so a state that steps onto 0 is re-seeded from the then-current `H`).
4. Result `(s >> 8) & 0xFFFFFF`, returned as a non-negative `i64`
   (`tos-api/src/xbios/system.rs:304-305@wrapping_mul`).

### Which `_hz_200` is `H`

What a program observes is the `$4BA` cell of its own page 0 (`tos-api/src/bios/tick.rs:108@publish`,
loader seed `tos-api/src/bdos/process/loader.rs:1262@HZ_200_ADDR`). The kernel counter
`bios::tick::hz_200()` (`tos-api/src/bios/tick.rs:92@hz_200`) is the value that cell is
published from (`tos-api/src/bios/tick.rs:78-85@advance`). EmuTOS has one global, so the
source gives no choice; the choice is NewTOS-internal and the contract is the
observable one: **`H` must lie in the window `[cell before the call .. cell
after the call]`** that X01 reads.

- **Kernel counter `tick::hz_200()`:** satisfies the window whenever the
  caller's cell is current. `Random` runs inside the SVC handler with IRQs
  masked (`bios/tick.rs` comment at `publish`, cited above), so the counter cannot
  move during the call. Inferred, not verified: right after a switch to another
  process the new process's cell may lag the counter by up to one tick until
  the next timer interrupt (`publish` writes only the running process's page).
- **Calling process's page-0 cell:** matches what the program literally reads,
  including a lagging cell, and would differ from the kernel counter only in that
  case; it needs `current_page0_kva()` (`tos-api/src/bdos/process.rs:1084@current_page0_kva`) and an unaligned
  read at `0x4BA`. A program that *writes* its own `$4BA` is seen by EmuTOS's
  `Random` but by neither NewTOS option for the kernel counter (only the cell
  option would see it); not measured, no known user.

Chosen (XBIOS11): the kernel counter. No owner question remains: the two
options agree on every state the probe can reach (first program after boot,
cell written by the loader at launch,
`tos-api/src/bdos/process/loader.rs:1262@HZ_200_ADDR`).

### Acceptance probe

`tosdump/probes/xsseed.c`, the probe of the audit run
(`measurements/measurement-2026-10-28-xbios-system-audit/method/xsseed.c`) made
permanent with its program text unchanged (XBIOS11). Built by
`make -C tosdump xsseed-m68k` (`XSSEED.PRG`) and `make -C tosdump xsseed-newtos`
(`XSSEED.NAP`, `tosdump/build-xsseed-newtos.sh`); run on one side by
`tosdump/probes/run-xsseed.sh emutos tt|falcontc <out-dir>` (the pinned oracle,
`--auto`) or `tosdump/probes/run-xsseed.sh newtos <out-dir> <kernel8.img>` (QEMU
raspi4b, `#Z` autorun; a kernel with `--features bios-arg-trace` leaves the
argument trace). It must run as the **first program after boot**, so no earlier
`Random()` (e.g. TOS Protobt) has advanced the state. It reads `$4BA` through
`Supexec`, calls `Random()` once, reads `$4BA` again, and for each `h` in
`[h1..h2]` computes the LCG step of `((u32)h << 16) + h`. It writes
`XSSEED.LOG` with two lines:

- `X01 first Random() matches a seed (hz_200<<16)+hz_200 for hz_200 in the window: yes|no`
- `X01 info hz_200 window=<h1>..<h2> random=<hex6>` (machine value, not compared)

Counts as equal to the oracle: the `X01 first ... : yes` line (oracle: `yes` in
`runs/SEED-EMUTOS-tt`, `-tt-repeat`, `-falcontc` of the run above). The `info`
line differs by design. Companion cases that must stay equal: `xsprobe.c`
R01-R03 and P08 (generator unchanged). The zero seed is not probed.

### Gaps vs. the contract

- None in the seed or the generator since XBIOS11 (first seed from `_hz_200`,
  zero seed kept, re-seed on every zero state).
- **No reseed back door:** `rseed` is an internal static; a higher-quality
  source (AArch64 `RNDR`) would be a separate extension, not a change to `Random`.

---

## Open points

- Inferred: zero seed only for `hz_200 == 0` (modular argument, `bios/xbios.c:424-425`).
- Inferred: lag of the page-0 cell after a process switch; not measured.
- Inferred: a program writing its own `$4BA` is out of any probe; behaviour of the
  two NewTOS options not tested.
- `xsseed.c` is a permanent probe under `tosdump/probes/` (XBIOS11); a tosdiff
  table case would need the first `Random` before any other, so it stays a probe
  until a follow-up ticket finds a table form (CLAUDE.md, Testing, probe rule).
- Out of scope: `docs/api/xbios/_state.md:347-351` already states the EmuTOS seed
  correctly; `docs/COMPATIBILITY.md` has no `Random` entry (lead's call).

# `Getmpb` (BIOS 0x00)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/biosmem.c:134` (`getmpb`).

## NewTOS status: **wired at BIOS 0x00** (read-only report; see NewTOS implementation notes)

Historically called exactly once, by GEMDOS init, to discover the ST-RAM
extent. NewTOS initialises its allocator directly from the AArch64 memory
map, so this call is vestigial. Whether to expose a thin MPB-over-free-list
is a `docs/COMPATIBILITY.md` port-vs-reimplement decision, deferred until a
program needs it. The dispatcher's out-of-range fall-through is *not* the
answer — `getmpb` is a real, in-range handler (`bios_vecs[0]`).

> **[BIOS06] 2026-10-28:** section "NewTOS contract" added (layout,
> content, checks, acceptance) and "Open decisions" (O1-O4). State:
> **contract decided 2026-10-28 (O1 = A, O2 = (a), O3 = (a) until the oracle measurement, O4 confirmed)**;
> **implemented 2026-10-29** (a public descriptor of its own in the kernel,
> V1/V4), `Getmpb(NULL)` measured on the oracle (V3, O3); **O3 decided
> 2026-11-11** (owner): a program's `Getmpb(NULL)`, in user and in super
> state, ends the program as on EmuTOS, where it is a bus error; nothing is
> written; the system keeps running (V3). Sources:
> `references/emutos/bios/biosmem.c:134-151` (read), kernel and SDK header
> (read, cites below), probe G00-G06 and tosdiff case 34 of
> `tosdump/cases/xbios-ptr-r1.tsv` (measured,
> `measurements/measurement-2026-10-29-bios06/RUNINFO.TXT`).

---

## NewTOS contract (NewTOS-Vertrag) [BIOS06]

> **Contract vs. reference** — `docs/COMPATIBILITY.md` "Native vs m68k struct
> variants" (`docs/COMPATIBILITY.md:1922`): fields and meaning are contract,
> the pointer width follows the caller's ABI. Reference (free): address and
> page of the descriptor, `themd` at `$48E`, the split into kernel `MD` and
> descriptor. Read = source/measurement; derived = inference, marked as such.

### V1 — The layout a NewTOS program sees (question 1)

**EmuTOS (m68k, read):** `MD` = `MD *m_link; UBYTE *m_start; LONG m_length;
PD *m_own` (`references/emutos/include/bdosdefs.h:108-115`), 16 bytes, offsets
0/4/8/12; `MPB` = three `MD *` (`include/bdosdefs.h:120-126`), 12 bytes.

**Published by the kernel until BIOS06 (read):** `tos_types::MD` = `m_link` (8, offset 0),
`m_start: Long` (4, offset 8), `m_length: Long` (4, offset 12), `m_own` (8,
offset 16), 24 bytes (`tos-types/src/memory.rs:61@MD`, table in the module
header there; check `:76`). `MPB` = three pointers, 24 bytes (`tos-types/src/memory.rs:101@MPB`,
check `:111`).

**In the SDK header today (read):** `_MD` = `struct _md *md_next; long md_start;
long md_length; BASEPAGE *md_owner` (`sdk/libc/overlay/include/mint/ostruct.h:293@_md`),
`_MPB` = three `_MD *` (`:305@_MPB`). Under LP64 (`long` = 8 bytes) that is
`_MD` 32 bytes (offsets 0/8/16/24) and `_MPB` 24 bytes (0/8/16) — **derived**
from the C alignment rules; measured: `sizeof(_MD)=32`, `sizeof(XMPB)=24`
(`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b/BCPROB2.LOG`,
G00). The header is byte-identical to the upstream text
(`Applications/mintlib-src/mintlib-0.60.1/include/mint/ostruct.h:240-255`,
read) — it is not NewTOS-specific.

**Finding (read + measured):** `MPB` agrees on both sides (24 bytes,
offsets 0/8/16). Only the **MD** differs: the kernel delivered a mixed format
(pointers 8 bytes, address/length 4 bytes — neither m68k nor LP64), the header
expects LP64. The program reads `md_start` = `m_start | m_length << 32` and
`md_length` = padding bytes: measured `md_start = 0x00A00000_00400000`,
`md_length = 0` (G03), raw bytes G05 `00 00 00 00 00 40 00 00 00 A0 00 ...`
are the kernel layout with the right values 0x00400000 / 0x00A00000.
`sdk/apps/apitest/apitest.c` declared the kernel layout itself until BIOS06
(`MDX`/`MPBX`, `sizeof == 24`); since BIOS06 it takes `_MD`/`_MPB` from the
header (`sdk/apps/apitest/apitest.c:2188@_MPB`).

**Which side breaks the contract (inference from COMPATIBILITY:1922):** if
the contract is "fields equal, pointer width follows the ABI", the
**kernel's mixed format** is the deviation: the kernel reused its internal
BDOS `MD` (fields as `Long`) for publication
(until BIOS06 `ThemdPage { md: MD }` in `tos-api/src/bios/getmpb.rs`), without
that layout ever being decided as a public contract. The unchanged upstream
header is then the contract side. That is the inference from the rule; the
choice itself is **O1** below.

**Public layout (O1 = A, user 2026-10-28):**

| Structure | Offset | Field | Width |
|---|---:|---|---:|
| `_MD` | 0 | `md_next` | 8 (pointer, EL0 VA) |
| | 8 | `md_start` | 8 (`long`; under VA = PA a value below 2 GiB) |
| | 16 | `md_length` | 8 (`long`, bytes) |
| | 24 | `md_owner` | 8 (pointer) |
| | | `sizeof` | 32 |
| `_MPB` | 0 | `mp_free` (`mp_mfl`) | 8 |
| | 8 | `mp_used` (`mp_mal`) | 8 |
| | 16 | `mp_rover` | 8 |
| | | `sizeof` | 24 |

The internal BDOS `MD` (24 bytes) stays as it is; it is no program contract
(pools `pmd`/`pmdalt` are never published, `THEMD` is named only in
`getmpb.rs` — read, `grep THEMD` over `tos-api`/`kernel`: no other hit).

**Implementation (BIOS06, 2026-10-29, read):** the kernel publishes a
descriptor of its own, `PublicMd` (`tos-api/src/bios/getmpb.rs:41@PublicMd`,
four 8-byte fields `md_next`/`md_start`/`md_length`/`md_owner`), and computes
the field offsets of the caller's block from `PublicMpb`
(`tos-api/src/bios/getmpb.rs:59@PublicMpb`); `THEMD` holds a
`PublicMd` (`tos-api/src/bios/getmpb.rs:91@ThemdPage`). The internal
`tos_types::MD` is unchanged. Measured: probe G03-G05 `md_start =
0x00400000 = membot`, `md_length = 0x00A00000`, `start+length = memtop`;
tosdiff case 34 equal against tt and falcontc
(`measurements/measurement-2026-10-29-bios06/RUNINFO.TXT`).

### V2 — Content, lifetime, write access (question 2)

**EmuTOS (read):** `getmpb` fills the one static `themd` with
`[membot, memtop)` and sets `mpb->mp_mfl = &themd`, `mpb->mp_mal = NULL`
(`bios/biosmem.c:144-150`). `mp_rover` stays untouched. **Who reads it
later:** GEMDOS `umem_init` calls `Getmpb((long)&pmd)` exactly once and reads
`pmd.mp_mfl->m_start`/`m_length` for `start_stram`/`end_stram`
(`bdos/umem.c:545-556`). From then on `themd` is **live GEMDOS state**:
`pmd.mp_mfl` points at it, first-fit allocation shortens and links it
(`docs/api/structures/md.md`, "Allocation flow", `bdos/iumem.c:35`). A
second `Getmpb` call after start-up rewrites `themd` while
`pmd.mp_mfl` still points at it (derived from `bios/biosmem.c:144-147` and
`umem_init`); probe G06 (second call, same pointer) and G03/G04
(`start+length = _memtop`) are therefore true on EmuTOS **by construction**:
the program reads what its own call has just written.

**NewTOS (read):** GEMDOS owns its pools itself: `bdos::mem::init::init`
seeds `pmd`/`pmdalt` from the memory map
(`tos-api/src/bdos/mem/init.rs:192@STRAM_START`; `docs/api/bios/00-getmpb.md`
"Pi / BCM2711 binding"), GEMDOS never calls `Getmpb`. `getmpb` is a
**report**: `(start, end) = stram_range()`
(`tos-api/src/bios/getmpb.rs:166@stram_range`; `stram_range` returns the values
set once at pool seeding, the only write site
`tos-api/src/bdos/mem/init.rs:192@STRAM_START`), writes into the static `THEMD`
`md_next = NULL`, `md_start = start`, `md_length = end - start`, `md_owner = NULL`
(`tos-api/src/bios/getmpb.rs:128-135@describe`, written at
`tos-api/src/bios/getmpb.rs:174@describe`). The contract follows:

1. **Content = snapshot of the initial extent of the ST-RAM pool**
   (`[membot, memtop)` at boot), **not** the live free list. One
   single MD, `md_next == NULL`, `md_owner == NULL`; `mp_used == NULL`
   (`mp_mal`) always; TT-RAM never included (as EmuTOS,
   `bios/biosmem.c:144-147`: only `membot..memtop`). Whoever looks for the
   free rest asks `Malloc(-1)`/`Mxalloc`. Measured: `md_start = membot`,
   `start+length = memtop` on both sides (G03/G04; NewTOS
   `membot = 0x00400000`, `memtop = 0x00E00000`, G00).
2. **Lifetime/stability:** one static descriptor, the same address on
   every call (G06 measured: `same_free=1`), the values never change after
   boot (derived: the only write site of the source is `init`). It is
   rewritten on **every** call with the same values and is therefore
   harmless; it outlives every program (`'static`). Unlike on EmuTOS a
   second call destroys **nothing** (deviation from EmuTOS, already settled
   in `docs/api/bios/00-getmpb.md` "NewTOS implementation notes": direction
   "harmless").
3. **Writability:** the descriptor is **read-only** for the program
   (FreeMiNT `PROT_PR`, `tos-api/src/bios/getmpb.rs:187@expose`; `UserRoNx`,
   `tos-api/src/bdos/process/usermem.rs:245@flags`). A write is a
   memory fault, as on EmuTOS (`themd` in protected memory). The
   **read in user state** is allowed on NewTOS; on EmuTOS it is
   a bus error (`themd` at `$48E`; `docs/audits/2026-10-28-bios-core.md`
   BIOS-CORE-003 "Messnebenbefund") — a program that reads the MD like a
   TOS program inside `Supexec` works on both sides.
4. **The page:** `THEMD` owns a 4-KiB page of its own
   (`tos-api/src/bios/getmpb.rs:91@ThemdPage`, check `tos-api/src/bios/getmpb.rs:100@ThemdPage`); the publication is
   idempotent per process (`tos-api/src/bdos/process/usermem.rs:204@expose`). If it fails,
   `mp_mfl = NULL` stays (empty free list, `tos-api/src/bios/getmpb.rs:177-189@expose`).
5. **Return value:** none (EmuTOS `void`); the kernel returns `0`.
6. **Caller buffer:** **only** `mp_mfl` (offset 0) and
   `mp_mal` (offset 8) are written, 8 bytes each, in this order
   (`tos-api/src/bios/getmpb.rs:205-211@fill_mpb`); `mp_rover` (offset 16) is not touched
   (as `bios/biosmem.c:149-150`). Measured: G02 `mp_rover_kept=1`,
   `mp_used_null=1` on both sides. The buffer must be writable for at least
   16 bytes, not the full 24.

### V3 — What the kernel checks (question 3, MEM12)

- **EmuTOS (read):** no check; `mpb == NULL` is stored through, at address 0
  (`bios/biosmem.c:149`, no guard).
- **measured (O3, 2026-10-29):** EmuTOS on the oracle, profiles tt and falcontc,
  `Getmpb(0L)` from a user program: the first store
  `mpb->mp_mfl = &themd` (`bios/biosmem.c:149`; in the ROM `getmpb+0x2C`,
  `move.l a1,(a0)` with `a0 = 0`, `a1 = $48E`) raises a **bus error in
  supervisor state** ("Bus error writing at address $0", PC
  `$E027E6`); EmuTOS reports `Panic: Bus Error ... addr=00000000` and waits
  for a key — the program does not return, the system halts.
  `themd` is already rewritten at that point (`:144-147` run
  before). NewTOS before the 2026-11-11 decision: `Getmpb(0L)` returned `0`,
  addresses 0 and 4 unchanged, the program ran on (probe N00-N03,
  `measurements/measurement-2026-10-29-bios06/RUNINFO.TXT`).
- **O3 decided 2026-11-11 (owner): like EmuTOS, no silent success, in
  every state.** A program's NULL MPB is refused **before any store**, in
  user state and in super state alike
  (`tos-api/src/bios/getmpb.rs:162@refuse_user_write`): NULL is treated as
  an address the program cannot write. In super state page 0 is EL0
  read-write (`docs/api/_super-state.md` S2, S8;
  `tos-api/src/bdos/process/mmu.rs:769@set_page0_writable`), so a store
  would succeed there and overwrite `$0`-`$F` of the program's page 0 (the
  page-0 check after the trap would then adopt `0` into the bus- and
  address-error vector cells `$8`/`$C`,
  `tos-api/src/bdos/process/hooks.rs:317@check_page0`); on EmuTOS address 0
  is not writable even in supervisor state (the measured bus error above).
  The refusal records the fault a store to a read-only page reports
  (`tos-api/src/uaccess.rs:890@refuse_user_write`; syndrome EC `0x25`, ISS
  `0x4F` = `WnR` + permission fault level 3, `FAR` = 0, as measured for an
  `sttr` in `docs/analysis/arch03-stage0b-spike.md` §5), the call returns
  `uaccess::FAULTED` (`tos-api/src/uaccess.rs:55@FAULTED`) having written
  nothing, and the syscall tail ends the program with the crash report "Bus
  Error" and `Pterm(-1)`
  (`kernel/src/arch/aarch64/exceptions.rs:775@take_pending_fault`,
  `kernel/src/arch/aarch64/exceptions.rs:1510@uaccess_fault_tail`;
  `docs/api/bios/_exception-crash.md`) — the same path as a store that
  faults. **Deliberate difference:** EmuTOS panics and the system halts;
  NewTOS ends only the calling program and the system keeps running (memory
  protection; `docs/COMPATIBILITY.md` "`Getmpb(NULL)` ends the program, the
  system keeps running"). Unlike EmuTOS, `THEMD` is not rewritten first
  (unobservable: the program does not return).
- **Any other address** is written with the caller's rights
  (`tos-api/src/bios/getmpb.rs:196@put_u64`, `tos-api/src/uaccess.rs:352@put_u64`;
  ARCH03 3a): one the program cannot write (e.g. in page 0, EL0 read-only in
  user state, `tos-api/src/bdos/process/mmu.rs:826@UserRoNx`, MEM12) is a
  permission fault at a fault-recovered `sttr`, the call ends with
  `uaccess::FAULTED` (`tos-api/src/bios/getmpb.rs:209@FAULTED`) and the
  program ends the same way.
- **In-kernel caller (EL1, `Space::Kernel`):** its pointers are dereferenced
  as kernel addresses, so a NULL there is a kernel bug; it is refused before
  any store with `uaccess::FAULTED` and no fault is recorded
  (`tos-api/src/bios/getmpb.rs:161@is_user`). No in-kernel caller passes NULL
  today (read: the only one is the unwired `kernel/src/selftest_bios.rs`
  check, with a stack MPB).
- A failure after the first store leaves `mp_mfl` set and `mp_mal`
  unchanged (derived from the order).
- **read:** page-wise check against MEM12: the kernel reads nothing from
  page 0; `Getmpb` reads no system variable (`stram_range()` is a
  kernel atomic, not `$432/$436`). The system variables `membot`/`memtop`
  in page 0 are an **output** (`docs/api/_super-state.md` section 0:
  "the kernel treats page 0 as output only").
- The alignment of the buffer is not checked (`put_u64` is
  alignment-free, `uaccess.rs:352@put_u64`; derived: the comments on
  `get_u64` say "any alignment").

### V4 — Compile-time check (question 4, as IOREC)

If O1 = A: in `ostruct.h` (SDK overlay), `#if defined(__LP64__)`:
`_Static_assert(sizeof(_MD) == 32)`, `offsetof(_MD, md_next) == 0`,
`md_start == 8`, `md_length == 16`, `md_owner == 24`, `sizeof(_MPB) == 24`,
`offsetof(_MPB, mp_free) == 0`, `mp_used == 8`, `mp_rover == 16`; in the kernel the
same numbers as `const _: () = assert!(offset_of!(..) == ..)` on the
public descriptor (and `size_of`). `MDX` in `sdk/apps/apitest/apitest.c` then goes
(the test takes `_MD` from the header). With O1 = B/C they would be the numbers of the
chosen variant (see below). **State 2026-10-29 (read):** SDK
since SDK04 (`sdk/libc/overlay/include/mint/ostruct.h:312-320@_Static_assert`),
kernel since BIOS06 on `PublicMd`/`PublicMpb`
(`tos-api/src/bios/getmpb.rs:70-79@offset_of`); `apitest` takes `_MD`/`_MPB`
from the header. The check `size_of::<MD>() == 2*usize+8`
(`tos-types/src/memory.rs:76@size_of`) now concerns only the internal BDOS `MD`.

### V5 — Users (survey)

See `docs/analysis/lp64-tos-struct-survey-2026-10-28.md` (row "MD/MPB")
and this ticket's survey: the only reader of `md_start`/`md_length` is
`sdk/apps/apitest/apitest.c:2188@_MPB` (B3, BS03-BS10; until BIOS06 through its
own `MDX`). `sdk/examples/uakid.c`
calls `Getmpb(BAD)` only with an invalid pointer (no read of the MD, not in the
standard image); `tosdump` measures `Getmpb` since TOOL16 only in
`tosdump/cases/xbios-ptr-r1.tsv` (inside a `Super` bracket, as the last
call; `tosdump/core/cases.c:3786@CR_GETMPB`,
`tosdump/cases/COVERAGE-BIOS.md:58@Getmpb`), MPB and MD field by field with the
layout from `docs/api/structures/md.md` ("Dumper layout"). No
ported program under `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`,
`programs/` and the local, gitignored `Applications` tree calls `Getmpb` in its source (binaries:
`sdk/apps/highwire/modules/network.src/socklib/mintlib.lib` contains only the
name, prebuilt m68k stock, gitignored; no evidence of a call). The
bug hits **no** ported program today; it hits every future
memory info program and the probe G00-G06.

### Open decisions (user) [BIOS06]

All four are decided; the rest of this section is the reasoning.

**Decisions (user 2026-10-28):** O1 = **A** (LP64 header layout,
a public descriptor of its own in the kernel); O2 = **(a)** static
initial snapshot; O3 = **(a)** silent success stays until EmuTOS's behaviour
on `mpb == NULL` is measured on the oracle; O4 confirmed (probe
G00–G06 as a permanent test until TOOL16). **O3 decided 2026-11-11
(owner):** like EmuTOS — a program's NULL is refused as an unwritable
store, in user and in super state, and ends the program; nothing is
written; the system keeps running (V3). The options stay below as the
reasoning.

**O1 — Which layout is the contract?**
- **A — LP64 header layout** (32/24 bytes, all fields 8 bytes). The kernel
  publishes a public descriptor of its own (new structure in
  `getmpb.rs`, `THEMD` uses it; the internal BDOS `MD` stays). The
  upstream header stays unchanged (ADR "Upstream unchanged"); `apitest`
  loses `MDX`/`MPBX`. Cost: kernel change in `getmpb.rs` + host test;
  follows the IOREC model F1 ("kernel delivers the layout of the SDK header").
  A later m68k thunk then needs the conversion 32 -> 16 bytes (as for
  OBJECT).
- **B — keep the kernel mixed format (24 bytes), change the overlay header**
  (`md_start`/`md_length` as 4-byte types). Kernel unchanged; instead the
  header departs from the upstream text (patch in `sdk/libc/overlay/`), and
  `md_start` is no longer a pointer. Cost: an exception in COMPATIBILITY
  needed (reasoning as for `AESGLOBAL`: there is a word-index contract there; for
  `MD` there is none — read, `docs/COMPATIBILITY.md:1937-1954` — so the
  argument is missing).
- **C — m68k wire format on every ABI** (16/12 bytes, everything 4 bytes). Requires
  a 12-byte `_MPB` in the header as well and 4-byte pointers in `md_next`/`md_owner`;
  breaks today's `_MPB` (24 bytes, equal on both sides). Cost:
  header patch of both structures, kernel writes 4-byte pointers. Only sensible
  if one wanted programs built against the m68k header to run without
  recompilation (the goal is the m68k thunk, not now).

**O2 — Content:** (a) static initial snapshot (today's code,
equal to EmuTOS in what a program gets through `Getmpb`) — no
change; (b) publish the live free list/allocated list of the BDOS pool.
**Caution:** `docs/COMPATIBILITY.md` claimed (2026-10-28, before `c173cddc`) that a
program finds a TSR's blocks on the allocated list through `Getmpb` —
the code (`mp_mal = 0`, one MD) does not do that (read, `tos-api/src/bios/getmpb.rs:205-211@fill_mpb`).
Either COMPATIBILITY must be corrected (a) or (b) built (a large
task: BDOS MDs are kernel-internal, page rights, consistency under the
giant lock). EmuTOS too shows only the one initial MD (`themd`) after
start-up, not a program's live state (inference
from `bios/biosmem.c:144-147`).

**O3 — `mpb == NULL`:** (a) silent success without a write (until 2026-11-11);
(b) fail the call like a non-writable buffer
(`FAULTED`), which ends the program. EmuTOS: writes to address 0, measured 2026-10-29 a
bus error in supervisor state with `Panic` (V3). Observable only
for faulty programs; (b) is the stricter direction. **Decided
2026-11-11: like EmuTOS** — realised as (b) for a program in every state
(user and super state, owner 2026-11-11 option (a) for the super state);
the halt is not reproduced (V3).

**O4 — Acceptance of the contents:** ticket BIOS06 asks for tosdiff only after
TOOL16; until then probe G00-G06 as a permanent test (both sides, layout
from O1). No decision needed, only confirmation.

---

## EmuTOS source

**Primary location:** `bios/biosmem.c:134` — `void getmpb(MPB *mpb)`
(the exported `getmpb` symbol; `bios/bios.c:895` `bios_0()` is only a
`DBGBIOS` debug wrapper and is compiled out in the default build).

**Helpers called:** none. The function is a straight-line field fill.

**Headers / type definitions used:**
- `include/bdosdefs.h:108` — `MD` (memory descriptor) definition
- `include/bdosdefs.h:120` — `MPB` (memory partition block) definition
- `include/tosvars.h:83` — `extern struct _md themd;`

**System variables / globals read:**
- `membot` (`bios/biosmem.c:145`, TOS var `0x432`) — bottom of TPA (free ST-RAM)
- `memtop` (`bios/biosmem.c:146`, TOS var `0x436`) — top of TPA
- (`end_os`, `0x4fa`, is set to `membot` in `balloc_stram`; `getmpb` reads
  `membot` after all `balloc_stram` calls have settled)

**System variables / globals written:**
- `themd` (`bios/biosmem.c:144-147`) — the single static BIOS `MD`
- `*mpb` (`bios/biosmem.c:149-150`) — caller-supplied struct
- `bmem_allowed = FALSE` (`bios/biosmem.c:137`) **only under `DBG_BALLOC`** —
  the "no more BIOS allocation past this point" guard

**Build configuration affecting this function:**
- `DBG_BALLOC` (default `0`, `bios/biosmem.c:15`) — when set, `getmpb`
  additionally clears `bmem_allowed` so any later `balloc_stram` panics.
  In the default build the body is only the six field stores.

---

## Source extraction summary

`Getmpb` reports the initial free ST-RAM region to GEMDOS as a Memory
Partition Block. It fills a single static memory descriptor `themd`
describing the whole free region `[membot, memtop)`: `m_link = NULL` (only
one descriptor), `m_start = membot`, `m_length = memtop - membot`,
`m_own = NULL` (unowned). It then points the caller's MPB free list at
`themd` (`mp_mfl = &themd`) and empties the allocated list (`mp_mal = NULL`).

Note what it does **not** touch: `mp_rover` (the third MPB field) is left
exactly as the caller supplied it. `getmpb` writes only `mp_mfl` and
`mp_mal`. (`mp_rover` is documented as "roving pointer — no longer used" in
`include/bdosdefs.h:125`.)

`membot`/`memtop` are set by `bmem_init()` (`bios/biosmem.c:51`) at boot:
`membot = end_os` (end of OS static data), `memtop = phystop` (top of
ST-RAM), then adjusted downward/upward by every `balloc_stram()` call during
BIOS init. By the time GEMDOS calls `Getmpb`, they bound the final free
region.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x00
- **Stack frame on entry (m68k):**
  ```
  +2: mpb (LONG, MPB *)
   0: function_id (word) = 0x0000
  ```
- **Total parameter bytes:** 6 (word ID + LONG pointer)
- **Native (AArch64) ABI:** function ID `0x00` in `x8`; the single pointer
  argument in `x0`. `getmpb` is `void` — the dispatcher leaves whatever is in
  `x0` (classically `d0`); programs must read the populated `*mpb`, never the
  return value.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `mpb` | `MPB *` (pointer) |

---

## Inputs

| Position | Name  | Type    | Size | Source location      | Notes |
|----------|-------|---------|------|----------------------|-------|
| arg0     | `mpb` | `MPB *` | 4/8  | `bios/biosmem.c:134` | caller-owned, writable 12-byte (m68k) MPB. **Not validated**: `NULL` is dereferenced (measured 2026-10-29: bus error in supervisor state, EmuTOS panics; NewTOS ends the program, NewTOS contract V3). |

`getmpb` performs no input validation whatsoever.

---

## Outputs

### Return value

- **Register:** D0 (m68k) / `x0` (native)
- **Type:** none — `getmpb` is declared `void`
- **Encoding:** undefined; the result is delivered entirely through `*mpb`.
- **Source location:** `bios/biosmem.c:134` (no `return` statement)

### Side effects on global state

- `themd` rewritten (`bios/biosmem.c:144-147`): `m_link=NULL`,
  `m_start=membot`, `m_length=memtop-membot`, `m_own=NULL`.
- `*mpb` written (`bios/biosmem.c:149-150`): `mp_mfl=&themd`, `mp_mal=NULL`.
  `mp_rover` is **not** written.
- Under `DBG_BALLOC` only: `bmem_allowed=FALSE`.

The MPB after the call:

| Field      | Value                          | Written? |
|------------|--------------------------------|----------|
| `mp_mfl`   | `&themd`                       | yes      |
| `mp_mal`   | `NULL`                         | yes      |
| `mp_rover` | (caller's prior value)         | **no**   |

The MD `themd` after the call:

| Field      | Value               |
|------------|---------------------|
| `m_link`   | `NULL`              |
| `m_start`  | `membot`            |
| `m_length` | `memtop - membot`   |
| `m_own`    | `NULL`              |

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none — the function cannot fail) | `bios/biosmem.c:134` | n/a |

`getmpb` has no error path. A `NULL` or non-writable `mpb` faults; EmuTOS
does not guard it. Measured on the oracle (2026-10-29, tt and falcontc):
`Getmpb(NULL)` from a user program ends in "Panic: Bus Error" at the
`mp_mfl` store (`bios/biosmem.c:149`) and the system halts. NewTOS (owner
decision O3, 2026-11-11; NewTOS contract V3): a program's NULL `mpb` is
refused before any store, in user and in super state, and a non-writable
one faults at the first store; nothing is written, and the program is
ended with the crash report "Bus Error" and `Pterm(-1)`; the system keeps
running (sanctioned deviation, `docs/COMPATIBILITY.md`). An in-kernel
caller's NULL is refused without a store (V3).

**Error paths not in Compendium:** none (Compendium also documents no error).

---

## Verification criteria

- **Nominal:** given a writable `MPB` at `mpb`, after the call
  `mpb->mp_mfl` points at a single `MD` with `m_link == NULL`,
  `m_start == membot`, `m_length == memtop - membot`, `m_own == NULL`, and
  `mpb->mp_mal == NULL`.
- **`mp_rover` untouched:** pre-load `mpb->mp_rover` with a sentinel; after
  the call it is unchanged (`getmpb` writes only `mp_mfl`/`mp_mal`).
- **Error:** no error code is defined. `Getmpb(NULL)` from a program, in
  user or in super state, does not return and writes nothing: the program
  ends with "Bus Error" (EmuTOS: bus error and panic; O3 decided
  2026-11-11, V3). Host tests
  `null_mpb_from_a_program_is_refused_before_any_store_in_any_state` and
  `unwritable_mpb_faults_and_writes_nothing` in
  `tos-api/src/bios/getmpb.rs`.

---

## Edge cases discovered in source

1. **Single static MD, shared across calls.** `themd` is one static `MD`
   (`bios/biosmem.c:144`). A second `Getmpb` call after GEMDOS init rewrites
   `themd` from the *current* `membot`/`memtop` and replaces the free list
   with one descriptor — discarding all of GEMDOS's allocation state. EmuTOS
   does not protect against this; the call is meant to run exactly once.

2. **`mp_rover` deliberately preserved.** Unlike `mp_mfl`/`mp_mal`, the
   third MPB field is never written, so the caller's value survives. (Old
   docs correctly flagged this; the source confirms it.)

3. **ST-RAM only.** `Getmpb` reports `[membot, memtop)` = ST-RAM. Alternate
   (TT-)RAM is added to the BDOS pool separately via `Maddalt`
   (`altram_init`, gated by `CONF_WITH_ALT_RAM`), never through `Getmpb`.

4. **The "two-bank MPB" documentation fiction.** Some Atari docs describe
   `Getmpb` returning ST and TT MPBs. EmuTOS fills exactly the one MPB the
   caller points at; there is no second bank.

---

## Dependencies

### Requires (must be initialized / functional)

- `bmem_init()` (`bios/biosmem.c:51`) — sets `membot`/`memtop`/`end_os`.
- All BIOS `balloc_stram()` allocations complete (final `membot`/`memtop`).
- `themd` (static, in BIOS BSS).

### Used by

- GEMDOS `osinit()` — the one caller, at boot, to seed the free-memory pool.

---

## Related data structures

- `docs/api/structures/md.md` — `MD` memory descriptor (16 bytes m68k)
- `docs/api/structures/basepage.md` — TPA context
- `MPB` (`include/bdosdefs.h:120`): three pointer fields `mp_mfl`, `mp_mal`,
  `mp_rover` (12 bytes on m68k). Note the struct name in source is
  **Memory _Partition_ Block**, and `mp_rover` is "no longer used".

---

## Cross-references

### Atari Compendium

- Not present in `references/` (no Compendium PDF). Cross-check deferred.
  Historically documents populate-MPB-and-return-void.

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

### Atari ST Internals

- Not present in `references/`.

---

## Configuration / version notes

- `DBG_BALLOC` (debug builds only) adds the `bmem_allowed=FALSE` guard.
- Configuration-independent in release builds across all TOS versions.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-03: NewTOS initialises the GEMDOS allocator directly from the AArch64
device-tree / memory map, so `Getmpb` is vestigial. If exposed at all it
should be a read-only report of the ST-RAM-equivalent pool; calling it
post-init must **not** reproduce EmuTOS's "destroy your free list" behaviour.

---

## Pi / BCM2711 binding

> Re-points the free-RAM discovery from the **`membot`/`memtop` low-memory
> globals** (set by `bmem_init` + `balloc_stram` on ST-RAM) to the **AArch64
> memory map** discovered from the device tree at boot. The MPB/MD layout is
> the contract; `Getmpb` itself is vestigial on the Pi. See
> `docs/HARDWARE-PROFILE.md` §3.6, §3.8.

### The allocator is seeded from the memory map, not from `Getmpb`

NewTOS builds its pools directly: `kernel/src/bios/machine/layout.rs::discover`
computes the ST-RAM (`pmd`) and TT-RAM (`pmdalt`) extents from the DTB memory
node (or the board fallback), and `kernel/src/lib.rs:452-481` hands those ranges
to `bdos::mem::init::init`. GEMDOS never calls `Getmpb` to learn the free region
— the classic "one caller, at boot" role is filled by the memory-map path.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| `membot`/`memtop` from `bmem_init` + `balloc_stram` (ST-RAM only) | ST-RAM pool `[kernel_end, DRAM_START + 14 MiB)` from the memory map (`layout.rs:310-313`) |
| `themd` single static MD over `[membot, memtop)` | the `pmd` MPB's seed MD, plus a separate `pmdalt` for TT-RAM (`Getmpb` never reported TT-RAM) |
| GEMDOS `osinit` calls `Getmpb` once | GEMDOS pools seeded from the AArch64 map before any trap runs |

### Current-code gaps (Step-2 candidates)

- **Wired since — correction (2026-09-05).** An earlier revision of this
  section said "`Getmpb` (BIOS 0x00) is not wired in the BIOS dispatcher …
  a call hits the BIOS catch-all". That is no longer true: `mod.rs` dispatches
  `GETMPB => getmpb::getmpb(args)` and `tos-api/src/bios/getmpb.rs` fills the
  caller's MPB as a **read-only** report of the ST-RAM pool extent — it does
  *not* rewrite the live free list, so EmuTOS's "second call destroys your
  allocator state" quirk stays rejected. Measured from EL0 on QEMU `raspi4b`:
  `mp_mal == NULL`, `mp_rover` untouched, `mp_mfl` non-NULL (`APITEST` checks
  BS03..BS05 PASS).
- **`mp_mfl` pointed at a kernel address an EL0 program could not read —
  MEASURED 2026-09-05, CLOSED the same day (ADR-023).** The MD the free-list
  head points to is the kernel static `THEMD` (`getmpb.rs`), at a high-half VA
  (`0xFFFF_0000_0000_0000 + physical`,
  `kernel/src/arch/aarch64/link-rpi4.ld:24`) whose pages are EL1-only. An
  application therefore got a free-list head it could not dereference:
  measured `mp_mfl == 0xFFFF00000026D8B8` (`APITEST` check BS06 FAIL; the MD
  field checks BS07..BS10 unrunnable from EL0).

  **Now:** `THEMD` occupies a page of its own and is published read-only —
  FreeMiNT `PROT_PR` (`references/freemint/sys/mint/mem.h:133`) — via
  `bdos::process::usermem::expose`; `mp_mfl` carries the resulting **user
  address** (ADR-023, same mechanism as `Getbpb`,
  `docs/api/bios/07-getbpb.md`; since ARCH03 stage 1 the descriptor's own
  physical address in the shared user address space, VA = PA). An in-kernel
  (EL1) caller still gets the kernel VA. If the publication fails, `mp_mfl` is
  left NULL — an empty free list, which this contract already defines —
  rather than a pointer the caller would fault on. Measured after the change
  on QEMU `raspi4b`: `mp_mfl = 0x00402000`, `APITEST` BS03..BS10 all PASS
  (`m_link == NULL`, `m_own == NULL`, `m_start == membot $432 == 0x400000`,
  `m_length == memtop-membot == 0xA00000`).
- **The published MD had the kernel's 24-byte BDOS layout — MEASURED
  2026-10-28 (BIOS-CORE-003), CLOSED 2026-10-29 (BIOS06).** An LP64 program
  reading through the SDK's `_MD` got `md_start = 0x00A00000_00400000`,
  `md_length = 0`. Now the kernel publishes its own descriptor in the SDK
  header's layout (NewTOS contract V1, O1 = A); tosdiff case 34 of
  `tosdump/cases/xbios-ptr-r1.tsv` is equal against both oracle profiles
  (`measurements/measurement-2026-10-29-bios06/RUNINFO.TXT`).

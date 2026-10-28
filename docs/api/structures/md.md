# `MD` / `MPB` — Memory Descriptor and Memory Partition Block

> Phase 00 spec D6. Sources: EmuTOS @ `1a7484ab` —
> `include/bdosdefs.h:108-125` (`MD`, `MPB`), `:18-22` (`MX_*`),
> `bdos/umem.c` (user-memory interface: `xmalloc`/`xmfree`/`xsetblk`/
> `xmxalloc`/`xmaddalt`, `find_mpb`, `umem_init`, `set_owner`),
> `bdos/iumem.c` (internal allocator: `ffit`, `freeit`, `shrinkit`),
> `bdos/osmem.c` (MD meta-allocator: `xmgetmd`, `xmfremd`, `xmgetblk`).
>
> **Note (corrected):** there is **no bdos/mem.c** in this tree — only
> `bdos/mem.h`. The allocator body is `bdos/iumem.c`; the trap-level wrappers
> are `bdos/umem.c`. There is no function named `mdalloc`. Earlier revisions
> of this doc cited `bdos/mem.c:xmalloc`/`xmfree`/`mdalloc`; all were wrong.

## Purpose

GEMDOS heap allocator metadata.

- **`MD`** (Memory Descriptor) describes one allocated or free
  contiguous region.
- **`MPB`** (Memory Partition Block) is the head of a free list +
  allocated list pair. EmuTOS uses one MPB per memory pool (ST-RAM,
  optionally TT-RAM).

This is the data structure backing `Malloc`, `Mfree`, `Mxalloc`,
`Mshrink`, and `Pexec`-time TPA carve-out.

## Binary layout — `MD` (16 bytes)

```c
struct _md {
/* 0x00 */ MD    *m_link;     /* next MD in this list, or NULL */
/* 0x04 */ UBYTE *m_start;    /* start address of region       */
/* 0x08 */ LONG   m_length;   /* region size in bytes          */
/* 0x0C */ PD    *m_own;      /* owner's PD, or NULL if free   */
};
```

## Binary layout — `MPB` (12 bytes)

```c
struct _mpb {
/* 0x00 */ MD *mp_mfl;        /* memory free list head       */
/* 0x04 */ MD *mp_mal;        /* memory allocated list head  */
/* 0x08 */ MD *mp_rover;      /* roving pointer (unused)     */
};
```

Comment in the header: *"roving pointer - no longer used"*. EmuTOS
keeps the field for ABI compatibility with code that walks the MPB
externally.

## NewTOS: oeffentliches Layout von `MD`/`MPB` [BIOS06]

> **Entschieden 2026-10-28: O1 = A** (`docs/api/bios/00-getmpb.md`,
> „Entscheidungen“). Noch nicht umgesetzt (TASK/BIOS06.md).

Drei Layouts existieren heute nebeneinander (gelesen):

| Layout | `MD` | `MPB` | Quelle |
|---|---|---|---|
| EmuTOS m68k | 16 Byte (0/4/8/12) | 12 Byte | `references/emutos/include/bdosdefs.h:108-125` |
| Kern (BDOS-intern, von `Getmpb` mitveroeffentlicht) | 24 Byte (Zeiger 8 @0, `m_start` 4 @8, `m_length` 4 @12, Zeiger 8 @16) | 24 Byte | `tos-types/src/memory.rs:61@MD`, `:101@MPB` |
| SDK-Header unter LP64 (`long` = 8) | 32 Byte (0/8/16/24, alle 8) — abgeleitet, gemessen `sizeof=32` | 24 Byte (0/8/16) | `sdk/libc/overlay/include/mint/ostruct.h:293@_md`, `:305@_MPB` |

### Dumper layout (tosdump) [TOOL16]

The layout the reference dumper reads through `Getmpb`'s MPB
(`docs/TOS-REFERENCE-DUMPER.md` section 6, "Structures behind a pointer").
This block is the **source**: a case table carries a copy of each line, and
the host test `layoutcheck` (`tosdump/host/layoutcheck.c`) refuses a copy
that is not this line byte for byte; the loader checks every number against
C on both ABIs.

- `md` -- m68k: EmuTOS' 16-byte `MD` above (`include/bdosdefs.h:108-115`;
  mintlib's m68k `_MD` has the same widths); LP64: the SDK's `_MD`, 32
  bytes, the contract O1 = A (`docs/api/bios/00-getmpb.md` V1). Field names
  are the SDK's: `md_next` = `m_link`, `md_start` = `m_start`,
  `md_length` = `m_length`, `md_owner` = `m_own`. `md_start` is a C `long`
  in the SDK and a pointer in EmuTOS -- four bytes on the m68k either way.
- `mpb` -- `MPB` above / the SDK's `_MPB`, 24 bytes on LP64; `mp_free` =
  `mp_mfl` is followed into `md` (`p>md`), `mp_used` = `mp_mal`,
  `mp_rover`.
- Classes (V2): `md_next`, `md_owner`, `mp_used` are NULL on both sides
  (EXACT); `mp_rover` is left as the caller laid it (EXACT: the dumper's
  poison); `md_start` is an address (PTR_RAM, `membot` differs per machine);
  `md_length` is the machine's free ST-RAM (NONZERO); `mp_free` an address
  (PTR_RAM).

```tosdump-layout
!layout	md	16/32	md_next:p@0/0:EXACT,md_start:l@4/8:PTR_RAM,md_length:l@8/16:NONZERO,md_owner:p@12/24:EXACT
!layout	mpb	12/24	mp_free:p>md@0/0,mp_used:p@4/8:EXACT,mp_rover:p@8/16:EXACT
```

Das Kern-24-Byte-`MD` ist ein **internes** BDOS-Layout. Was ein Programm ueber
`Getmpb` sieht, ist der Vertrag aus `docs/api/bios/00-getmpb.md` (V1);
der Vorschlag dort ist das LP64-Header-Layout (32/24 Byte). Pool-`MD`s
(`pmd`/`pmdalt`, Allokationslisten) werden keinem Programm veroeffentlicht
(abgeleitet: einzige Veroeffentlichung eines `MD` an ein Programm ist
`tos-api/src/bios/getmpb.rs:35@ThemdPage`).

## Allocator semantics

### Allocation strategy

First-fit linear scan of `mp_mfl`. EmuTOS does **not** use a roving
pointer or buddy/best-fit. The free list is kept sorted by ascending
`m_start` so coalescing on free is O(n) in the list length.

### Allocation flow (`bdos/iumem.c:35` `ffit`)

Reached via `Mxalloc`/`Malloc`. The free MD's front is carved for the request:

1. Walk `mp_mfl`; take the first MD with `m_length >= request` (after rounding
   the request up to the pool's alignment grain, `bdos/iumem.c:83-86`).
2. If that MD's length equals the request exactly, unlink it whole
   (`bdos/iumem.c:103-104`).
3. Otherwise carve `request` bytes off the *front*: allocate a **new** MD via
   `xmgetmd` for the *remainder* and leave it on `mp_mfl`
   (`m_start += request`, `m_length -= request`); the original MD is trimmed to
   `request` and becomes the allocated block (`bdos/iumem.c:105-127`).
4. Prepend the allocated MD to `mp_mal` and set `m_own = run`
   (`bdos/iumem.c:132-134`). If the `xmgetmd` in step 3 fails, the whole
   allocation fails (returns NULL) — the free MD is *not* modified.

### Free flow (`bdos/iumem.c:144` `freeit`)

1. Find the MD on `mp_mal` whose `m_start` matches. If none → return (caller
   `xmfree` maps this to `EIMBA`).
2. Unlink from `mp_mal`; walk `mp_mfl` to the ascending-`m_start` position and
   link it in there (`bdos/iumem.c:180-202`).
3. Coalesce with the higher and lower free neighbours if contiguous, freeing the
   absorbed neighbour's MD via `xmfremd` (`bdos/iumem.c:207-221`).

> **Note on ownership.** `Mfree` does **not** check `m_own == current PD` —
> EmuTOS preserves the original Atari quirk that any process can free any block
> by passing its address. Ownership is consulted only by `Pterm`'s
> `free_all_owned` / `Ptermres`'s `reserve_blocks` paths (see below), not by
> interactive `Mfree`. The per-function spec
> `docs/api/gemdos/memory/49-mfree.md` § "Edge cases" is authoritative.
>
> **Note (corrected): `freeit` does NOT clear `m_own`.** It only relinks the MD
> between the lists (`bdos/iumem.c:144-222` — no `m_own` write). The freed MD
> keeps its previous owner pointer. Nothing in the free/alloc path reads
> `m_own` for a free MD (membership is by list position + `m_start`), so this is
> internally harmless — but "`m_own == NULL` ⇔ free" is **not** an invariant of
> this build. Earlier revisions asserted `freeit` sets `m_own = NULL`; it does
> not.

### `Pterm` / `Ptermres` cleanup (`bdos/proc.c:77-101`)

When a process terminates, EmuTOS walks `mp_mal` (both pools) and frees every
MD with `m_own == the PD` via `free_all_owned` → `freeit`
(`bdos/proc.c:92-101,136-139`). `Ptermres` uses `reserve_blocks`, which instead
*detaches* the owned MDs from `mp_mal` (via `xmfremd`) so the memory stays
reserved after the process becomes resident (`bdos/proc.c:77-89,628`). This
owner filter is the only place `m_own` gates a free.

## Multiple pools (`MX_*` modes)

`Mxalloc(size, mode)` selects a pool based on `mode`:

| `mode` | Constant         | Behaviour                         |
|-------:|------------------|-----------------------------------|
| 0      | `MX_STRAM`       | ST-RAM only                       |
| 1      | `MX_TTRAM`       | TT-RAM only (fail if none)        |
| 2      | `MX_PREFSTRAM`   | ST-RAM preferred, fall back TT    |
| 3      | `MX_PREFTTRAM`   | TT-RAM preferred, fall back ST    |

`MX_MODEMASK = 0x03` (`include/bdosdefs.h:22`). Bits 2..15 of `mode` are
reserved; EmuTOS clears them with `mode &= MX_MODEMASK` (`bdos/umem.c:241`)
before dispatching, so caller-supplied protection bits (Atari documents
bits 4..15 as supervisor/global flags) are silently ignored.
NewTOS does the same and records the divergence under
`docs/api/gemdos/memory/44-mxalloc.md`.

Each pool has its own MPB. EmuTOS's globals (`bdos/umem.c:35-38`): `pmd`
(ST-RAM MPB, always present), `pmdalt` (alt-/TT-RAM MPB) plus the
`int has_alt_ram` flag, both compiled in **only under `CONF_WITH_ALT_RAM`**.
(`CONF_WITH_TTRAM` is a separate flag that governs *detecting* TT-RAM at boot
and requires `CONF_WITH_ALT_RAM`; the pool struct itself is gated on
`CONF_WITH_ALT_RAM`.) When `CONF_WITH_ALT_RAM == 0`, `Mxalloc` mode 1
(`MX_TTRAM`) returns NULL and modes 2/3 have no alt-RAM half.

NewTOS: two MPBs from the start (one per pool). See
"NewTOS implementation notes" below.

## MD meta-allocator (`xmgetmd` / `xmfremd`)

MD structs themselves need storage. EmuTOS does **not** allocate
them on the heap they manage — it would be a bootstrap problem.
Instead it carves them out of a fixed-size internal OS-memory pool
(`osmem.c`):

- A static word array `osmem[LENOSM]` where `LENOSM =
  LEN_OSM_BLOCK*NUM_OSM_BLOCKS/sizeof(WORD) = 66*118/2 = 3894` words = **7788
  bytes**, sized for `NUM_OSM_BLOCKS = 118` blocks of `LEN_OSM_BLOCK = 66`
  bytes each (`bdos/osmem.c:36-39`).
- `xmgetblk(WORD i)` / `xmfreblk` allocate fixed-size blocks from
  this pool via a quick-list of LIFO free chains indexed by
  paragraph count (`root[MAXQUICK]`, `MAXQUICK = 5`). In practice `xmgetblk`
  always uses index 4: a 64-byte payload + 1 control word = 66 bytes = one
  `LEN_OSM_BLOCK` (`bdos/osmem.c:304-321`).
- MDs are not allocated one-by-one. They live in `MDBLOCK`s, each
  holding `MDS_PER_BLOCK = 3` MDs plus a `mdb_next` link
  (`bdos/osmem.c:44-55`):

```c
typedef struct {
    MD   md;
    WORD index;        /* slot index 0..2 if used, else -1 */
} MDEXT;

struct _mdb {
    MDBLOCK *mdb_next;
    MDEXT    entry[3];
};
```

`MDBLOCK` size: `4 + 3*(16+2) = 58` bytes; rounded up to a
64-byte `osmem` block.

### `xmgetmd` (`bdos/osmem.c:169`)

1. If `mdbroot` (chain of partially-used MDBLOCKs) is empty,
   `MGET(MDBLOCK)` allocates a fresh one from `osmem` and links it
   in.
2. Walk the entries of the head MDBLOCK; pick the first slot with
   `index < 0`; set `index = i` (slot number 0..2); return
   `&entry[i].md`.
3. If that allocation made the MDBLOCK full (no remaining `index <
   0`), unlink it from `mdbroot`. It will be re-linked on the next
   `xmfremd` that frees one of its MDs.

### `xmfremd` (`bdos/osmem.c:229`)

1. Read the *word following* the MD (`*(WORD *)(md+1)`) — this is
   the slot's `index` field. EmuTOS relies on the
   `MD`/`WORD` adjacency in `MDEXT` to recover the index without a
   back-pointer.
2. From `index` reconstruct the start of `entry[]`, then the
   `MDBLOCK` (one pointer-sized field before).
3. Mark the slot free (`index = -1`). Count remaining free slots:
   - 1 free → add MDBLOCK back to `mdbroot`.
   - 3 free → unlink and return the MDBLOCK to the `osmem` quick
     list via `xmfreblk`.
   - 2 free → already on chain, do nothing.

### Why this matters for NewTOS

The `MDEXT` trick — reading `*(WORD *)(md+1)` to recover the slot
index — is **a layout invariant**, not just an allocator detail.
If the Rust port stores `MD` in a way that puts something other
than `index: i16` immediately after the 16 bytes of MD payload,
`xmfremd` semantics break for any caller that holds an MD pointer
long enough for the meta-allocator to consult it.

NewTOS implementation: store MDs in an `MDExt { md: MD, slot: i16,
_pad: i16 }` `#[repr(C)]` struct grouped 3 per `MDBlock`. The MD
pointer handed to BDOS code is `&mut md`. The 2-byte `slot` field
follows `md` in memory; pad to keep alignment. See PHASE-04 D5.

## Sizing constraints

- `osmem` capacity: `NUM_OSM_BLOCKS = 118` blocks of `LEN_OSM_BLOCK = 66` bytes
  = 7788 bytes (the array is declared in words: `LENOSM = 3894` words). Each
  `MDBLOCK` consumes one 64-byte-payload block and provides 3 MDs, so the
  absolute ceiling is `118 * 3 = 354` MDs — minus whatever `osmem` storage the
  DMD/DND/OFD tables consume (they draw from the same pool via
  `MEMTYPE_DMD`/`DND`/`OFD`, `bdos/mem.h`).
- NewTOS (MEM05, `docs/MEMORY.md` ADR-025, `docs/COMPATIBILITY.md` "No
  fixed memory-descriptor limit"): **no fixed MD count.** `MDBlock`s live
  in 4-KiB pages of 39 blocks = 117 MDs; the first page is static, further
  pages come from the kernel heap on demand
  (`tos-api/src/bdos/mem/mdpool.rs`, `pages.rs`) and are never moved or
  given back. MD pages and the kernel's per-block tables share one budget
  of 512 pages (2 MiB); reaching it, or a full kernel heap, is the
  exhaustion path below. A per-process limit of allocated blocks (default
  8192, boot argument `mdlimit=<n>`, `0` = off) bounds a single program
  (`owner_limit.rs`). We do **not** port the `osmem` quick-list; MDs do not
  share storage with OFD/DND tables. `xmfremd` recovers the slot index in
  O(1) exactly as EmuTOS does.
- **NewTOS exhaustion** (per-process limit reached, page budget reached,
  kernel heap full): `xmgetmd` returns null and the callers take their
  EmuTOS error paths — `Malloc`/`Mxalloc` 0, `Mshrink` `ERROR` (-1),
  `Maddalt` `ENSMEM`, `Pexec` `ENSMEM`; with memory protection on, the room
  for a block's record is secured before the block is allocated, so a
  refused table page also ends in `Malloc` 0 with nothing changed.
- **On real exhaustion, EmuTOS halts, it does not return an error.**
  `xmgetblk()` (`bdos/osmem.c:291-354`) returns `NULL` only for
  `MEMTYPE_MDBLOCK` requests; for `MEMTYPE_DMD`/`DND`/`OFD` it tries
  `free_available_dnds()` once and, if that still yields nothing, prints
  `"System halted!"` and calls `halt()` — a full system freeze, never an
  `ENSMEM` return to the GEMDOS caller. See
  `docs/api/structures/dnd.md` ("Lifetime") and
  `docs/api/_discrepancies.md` D-064 for the fuller writeup and the open
  NewTOS design question (do not replicate the halt without an explicit
  decision).

## Invariants

- Free list (`mp_mfl`) is sorted ascending by `m_start` (`freeit` inserts in
  order, `bdos/iumem.c:191-202`); allocated list (`mp_mal`) is in insertion
  order (prepend, `bdos/iumem.c:132-133`).
- Adjacent regions on the free list are coalesced by `freeit` — after a free,
  `mp_mfl` holds no two MDs `A`, `B` with `A->m_start + A->m_length ==
  B->m_start` (`bdos/iumem.c:207-221`). Caveat: `Maddalt` prepends to `pmdalt`
  without full re-sort, so alt-RAM can be transiently unsorted until the first
  alloc/free cycle self-heals it (see `14-maddalt.md` edge case 5).
- `m_length > 0` for all MDs.
- **`m_own` is NOT a reliable free flag.** `ffit` sets `m_own = run` on
  allocation, but `freeit` does **not** clear it on free (see the corrected
  "Free flow" note). Only `Pterm`/`Ptermres` read `m_own`, and they walk
  `mp_mal` (allocated MDs), where `m_own` *is* meaningful. Do not treat
  "`m_own == NULL`" as "this MD is free". (NewTOS may cleanly clear `m_own` on
  free — a permitted improvement, since it doesn't rely on the stale value.)
- The MD struct itself is allocated separately from the region it describes,
  from the OS-memory pool via `xmgetmd`/`xmfremd` (`bdos/osmem.c`), **not** from
  the heap the MDs manage. There is no `mdalloc` function.

## Cross-references

- **Compendium:** "Memory Allocation" / "MD" — agrees on layout.
- **Profibuch:** "Speicherverwaltung" — agrees.
- **`docs/api/structures/basepage.md`** — `m_own` points to PD.
- **`docs/api/gemdos/memory/48-malloc.md`**, **`44-mxalloc.md`**,
  **`49-mfree.md`**, **`4a-mshrink.md`**, **`14-maddalt.md`** — use MPB/MD.
- **EmuTOS source:** `bdos/iumem.c` (allocator), `bdos/umem.c` (wrappers),
  `bdos/osmem.c` (MD meta-allocator).

## Configuration

- `CONF_WITH_ALT_RAM` compiles in the second pool `pmdalt` + `has_alt_ram`
  (`bdos/umem.c:36-38`). `CONF_WITH_TTRAM` (which requires `CONF_WITH_ALT_RAM`)
  additionally enables *detecting* TT-RAM at boot.
- `CONF_WITH_FRB` (= `CONF_WITH_ALT_RAM` by default, `include/config.h:1504`)
  reserves a 64 KiB ST-RAM buffer for DMA fallback — not part of the regular
  MPB.

## NewTOS implementation notes

Phase 04 plan (specs:
[`docs/api/gemdos/memory/`](../gemdos/memory/)):

- **Two pools from day one.** ST-RAM MPB sits over the `Pool::Stram`
  frame slot from
  [`kernel/src/bios/machine/layout.rs`](../../../kernel/src/bios/machine/layout.rs);
  TT-RAM MPB over `Pool::Ttram`. `MX_PREFSTRAM` / `MX_PREFTTRAM`
  fall through cleanly. The single-pool fallback EmuTOS uses for
  pre-TT machines is irrelevant here — every NewTOS target has
  TT-RAM.
- **MD storage.** `MDBlock` (3 MDs + `mdb_next`) in
  `tos-api/src/bdos/mem/`, in pages of 117 MDs that grow on demand
  within the MEM05 page budget (see "Sizing constraints").
  The `MDEXT { md, slot }` layout invariant is preserved so the
  `xmfremd`-style index recovery from `*(WORD*)(md+1)` works
  byte-for-byte, even though we will write `xmfremd` in Rust.
- **MPB storage.** Two `MPB` statics in `tos-api::bdos::mem`,
  initialised once during BIOS bring-up after
  `bios::machine::frames::init`.
- **Region storage.** `m_start` points into the corresponding pool's
  frame range. The MPB free list is seeded with one MD per pool
  covering the whole pool extent. Subsequent `Mxalloc` calls
  carve from there.
- **Owner field.** `m_own` is `Option<NonNull<BasePage>>` —
  `None` for free MDs. Until Phase 06 introduces real PDs, every
  alloc gets a sentinel `KERNEL_PD` pointer so the
  `Pterm`-cleanup mechanism is structurally correct from the
  start.
- **Allocations beyond ABI width.** TOS `Malloc` returns `LONG`
  (i32). On QEMU the entire memory map fits in 32 bits; on RPi4
  with `>1` GiB DRAM the high TT-RAM region does not. Phase 05
  introduces a low-half address window for user pointers; until
  then RPi4 EC verification is gated.

# `DTA` — Disk Transfer Area

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `include/bdosdefs.h:48-55`. Used by `Fsfirst` / `Fsnext`
> (`bdos/fsdir.c`).

## Purpose

44-byte structure that `Fsfirst`/`Fsnext` populate with directory
entry information. Each process has a "current DTA" pointer
(`PD.p_xdta`); applications change it via `Fsetdta` (GEMDOS 0x1A) and
query it via `Fgetdta` (0x2F).

The first 21 bytes are reserved for the kernel's iteration state
across `Fsnext` calls (search pattern, current cluster, attribute
mask, …). Applications must not modify them between the matching
`Fsfirst` and the final `Fsnext` that reports `EFILNF`.

## Binary layout (44 bytes, fixed by ABI)

```c
typedef struct {
/* 0x00 */ char  d_reserved[21];   /* internal EmuTOS state */
/* 0x15 */ char  d_attrib;         /* attributes of found entry */
/* 0x16 */ UWORD d_time;           /* packed time (FAT format)  */
/* 0x18 */ UWORD d_date;           /* packed date (FAT format)  */
/* 0x1A */ LONG  d_length;         /* file length in bytes      */
/* 0x1E */ char  d_fname[14];      /* NUL-terminated 8.3 name   */
/* 0x2C — end */
} DTA;
```

## Field semantics

### `d_reserved[21]` (0x00–0x14)

Internal EmuTOS layout (cf. `bdos/fsdir.c` — `dnd_pos`, search-spec
copy, attribute mask, time-of-search). **Not portable** between TOS
versions — applications are documented to treat these bytes as
opaque. NewTOS must reproduce *some* per-search state but the
internal field layout is not part of the binary contract.

The Compendium documents only the post-0x14 fields; the 21-byte
reserved region is "for system use".

### `d_attrib` (0x15)

The `FA_*` attribute bits of the **found** entry — not the search
mask. Bits per `bdosdefs.h:36-41`:

```c
#define FA_RO       0x01    /* read-only       */
#define FA_HIDDEN   0x02    /* hidden          */
#define FA_SYSTEM   0x04    /* system          */
#define FA_VOL      0x08    /* volume label    */
#define FA_SUBDIR   0x10    /* directory       */
#define FA_ARCHIVE  0x20    /* archive bit     */
```

### `d_time` / `d_date` (0x16, 0x18) — FAT packed, native byte order

```
d_time bits: hhhhh mmmmmm sssss (s in 2-second units)
d_date bits: yyyyyyy mmmm ddddd (year - 1980)
```

Same *bit* packing as `Tgettime`/`Tgetdate` and FAT directory entries — but the
DTA words are stored in **native (m68k big-endian) byte order**, not the
on-disk little-endian order. `makbuf` byte-swaps them from the FCB via
`swpw`/`swpl` (`bdos/fsdir.c:1851-1862`); likewise `d_length` (0x1A) is native
byte order. NewTOS must reproduce the m68k big-endian words for the AArch64 ABI
(see `_discrepancies.md` D-024).

### `d_length` (0x1A)

File length in bytes (LONG). For directories: 0.

### `d_fname[14]` (0x1E)

Found filename in 8.3 form, NUL-terminated, **uppercase**, with the
dot in place (e.g. `"FOO.TXT\0"`). 14 bytes is enough for "12345678.123\0"
plus one trailing pad byte. Names with no extension lack the dot.

## DTA pointer in the basepage

`PD.p_xdta` (offset 0x20) is the active DTA pointer. **Default value
on process creation: `p_xdta = &p_cmdlin[0]`** — i.e., the basepage's
own command-line buffer (`bdos/proc.c:bxexec`). This is the historic
Atari quirk: programs that call `Fsfirst` without first calling
`Fsetdta` overwrite their own command line.

NewTOS must preserve this default for binary compatibility (m68k
emulator, Phase 10+); native source ports should call `Fsetdta`
explicitly before any `Fsfirst`.

## Lifecycle / invariants

1. `Fsetdta(buf)` writes `p_xdta = buf` and returns nothing.
2. `Fsfirst(spec, attr)` initialises the entire 44-byte DTA and, on
   success, fills bytes 0x15..0x2B with the first match.
3. Subsequent `Fsnext()` mutate `d_reserved[]` and rewrite the
   visible fields with the next match.
4. After `ENMFIL` (-49, no more matches — **not** `EFILNF`, which is
   `Fsfirst`'s own "no match at all" code) the visible fields hold the last
   successful match; the reserved area is in a "search exhausted" state.
5. Calling `Fsfirst` again (with same or different DTA pointer)
   resets the iteration. Calling `Fsnext` *without* a prior matching
   `Fsfirst` on the same DTA returns `ENMFIL` (-49) — the `dt_offset_drive`
   private field is `< 0` (uninitialised), and `xsnext` returns `ENMFIL`
   for every uninitialised / exhausted case (`bdos/fsdir.c:662-663`). (An
   earlier draft here wrongly claimed `EINTRN` on EmuTOS — corrected, see
   `_discrepancies.md` D-019.)

Multiple processes / multiple concurrent searches: each search needs
its own DTA. The `p_xdta` pointer is per-process, but applications
that nest searches must explicitly swap DTAs (or use separate DTA
buffers and manual `Fsetdta` calls).

## Cross-references

- **Compendium:** "DTA" — agrees on visible layout.
- **Profibuch:** "Die DTA" — agrees, same caveat about reserved bytes.
- **`docs/api/structures/basepage.md`** — `p_xdta` field at 0x20.
- **`docs/api/gemdos/dta/1a-fsetdta.md`** — sets the pointer.
- **`docs/api/gemdos/file/4e-fsfirst.md`** — initialises the DTA.
- **EmuTOS source:** `bdos/fsdir.c` — search state encoding in
  `d_reserved`.

## NewTOS implementation notes

(Empty during Phase 00. Reserved-area layout: spec it once we
implement `Fsfirst`/`Fsnext` in the filesystem phase.)

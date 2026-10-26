# `OFD` / `DFD` / `FCB` — Open File Descriptors and on-disk Directory Entry

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `bdos/fs.h:104-200`. Used pervasively by `bdos/fsmain.c`,
> `bdos/fsfat.c`, `bdos/fsdir.c`, `bdos/fsio.c`.

## Three structures, layered

- **`FCB`** — on-disk directory entry (32 bytes, FAT-format).
- **`DFD`** — in-memory copy of the FCB, shared between all OFDs that
  refer to the same file.
- **`OFD`** — per-handle open-file state, embedding a `DFD` for the
  base OFD and a pointer to the base OFD's DFD for shared OFDs.

## `FCB` — File Control Block (32 bytes, on-disk format)

```c
typedef struct {
/* 0x00 */ char    f_name[FNAMELEN];  /* FNAMELEN = 11: 8 name + 3 ext */
/* 0x0B */ UBYTE   f_attrib;
/* 0x0C */ UBYTE   f_fill[10];        /* reserved per FAT spec */
/* 0x16 */ DOSTIME f_td;              /* time/date, little-endian! */
/* 0x1A */ CLNO    f_clust;           /* starting cluster (UWORD) */
/* 0x1C */ LONG    f_fileln;          /* file size in bytes */
/* 0x20 — end */
} FCB;
```

Bit-identical to MS-DOS / FAT12/16 directory entries. The
`f_name[0] == 0xE5` byte means erased (`ERASE_MARKER` constant).
Long filenames (`FA_LFN = 0x0F` attribute pattern) are recognised
and skipped — EmuTOS does not implement LFN read/write.

## `DOSTIME` — packed time/date

```c
typedef struct {
    UWORD time;     /* hhhhh mmmmmm sssss (s/2) */
    UWORD date;     /* yyyyyyy mmmm ddddd (year-1980) */
} DOSTIME;
```

Stored **little-endian on disk** even on m68k. EmuTOS handles the
byte swap explicitly; in memory the WORDs are native-endian.

## `DFD` — Disk File Data (14 bytes)

In-memory shared mirror of the FCB. **One DFD per file**, not per
handle. Multiple OFDs that opened the same file share the DFD via
`OFD.o_dfd`; the DFD is owned by the *base* OFD (the first one to
open the file).

```c
typedef struct {
/* 0x00 */ UWORD   o_flag;     /* see flags below */
/* 0x02 */ WORD    o_usecnt;   /* number of OFDs sharing this DFD */
/* 0x04 */ DOSTIME o_td;       /* mirrors FCB f_td */
/* 0x08 */ CLNO    o_strtcl;   /* mirrors FCB f_clust */
/* 0x0A */ LONG    o_fileln;   /* mirrors FCB f_fileln */
/* 0x0E — end, 14 bytes total */
} DFD;
```

**Correction:** an earlier draft claimed 16 bytes ("2 bytes pad to 16") — no
such padding exists in `bdos/fs.h:116-124`; the struct is exactly 14 bytes
(all fields 2-byte-aligned, m68k needs no trailing pad). The tail three
fields (`o_td`, `o_strtcl`, `o_fileln` = 4+2+4 = 10 bytes) are laid out
identically to the FCB's tail so `ixclose` can `memcpy(&fcb->f_td,
&dfd->o_td, 10)` in one shot (`bdos/fsopnclo.c:538`).

`o_flag` bits:
```c
#define O_DIRTY  1   /* DFD has been modified, FCB needs rewriting */
```

## `OFD` — Open File Descriptor (≤ 64 bytes — architectural limit)

**Size constraint:** "this structure must not exceed 64 bytes in
length" (comment in `bdos/fs.h:138`). Required for compatibility
with the FOLDRnnn.PRG mechanism, which preallocates fixed-size OFD
slabs from the system FCB pool.

```c
struct _ofd {
/* 0x00 */ OFD  *o_link;       /* link in FCB (system) free / used list */
/* 0x04 */ DFD  *o_dfd;        /* points at DFD (own o_disk for base OFD) */
/* 0x08 */ DMD  *o_dmd;        /* drive media block */
/* 0x0C */ DND  *o_dnode;      /* directory containing this file */
/* 0x10 */ OFD  *o_dirfil;     /* OFD for that directory */
/* 0x14 */ LONG  o_dirbyt;     /* byte offset of FCB within parent dir */

/* 0x18 */ LONG  o_bytnum;     /* current byte position within file */
/* 0x1C */ CLNO  o_curcl;      /* current cluster number */
/* 0x1E */ RECNO o_currec;     /* current record (sector) number */
/* 0x22 */ UWORD o_curbyt;     /* offset within current cluster */
/* 0x24 */ OFD  *o_thread;     /* link in chain of OFDs sharing this DFD */
/* 0x28 */ UWORD o_mod;        /* open mode (see below) */

/* 0x2A */ DFD   o_disk;       /* embedded DFD (used only by base OFD) */
/* 0x38 — total 56 bytes; fits in 64-byte budget */
};
```

(Verified: `o_mod` — `UWORD`, 2 bytes — sits at 0x28, ending at 0x2A; the
embedded `o_disk` is the corrected 14-byte `DFD` above, so it runs 0x2A–0x38.
`sizeof(OFD) = 0x38 = 56` bytes, fields matched one-for-one against
`bdos/fs.h:140-157` with no invented padding anywhere in the struct.)

`RECNO` is `ULONG`, `CLNO` is `UWORD` (`bdos/fs.h:88-90`).

### Sharing model

- The first OFD opened for a given file is the **base OFD**.
- `base_ofd->o_dfd = &base_ofd->o_disk` (self-reference).
- Subsequent OFDs for the same file: `ofd->o_dfd = base_ofd->o_dfd`.
- `o_thread` chains all OFDs that share the DFD. Used at flush time
  to propagate `O_DIRTY` and at close time to find the new base
  when the previous base is closed.

### `o_mod` field

```c
#define INH_MODE         0x80   /* inheritance flag (not implemented) */
#define MODE_FSM         0x70   /* file-sharing mode (not implemented) */
#define MODE_FAC         0x07   /* file-access code:                   */
#define RO_MODE              0  /*   read-only                         */
#define WO_MODE              1  /*   write-only                        */
#define RW_MODE              2  /*   read-write                        */
#define VALID_FOPEN_BITS  MODE_FAC
```

Bits 4–6 (file sharing) and bit 7 (inheritance) are accepted but
not enforced — EmuTOS allows multiple OFDs on the same file in any
mode combination.

### `o_dirbyt` — directory entry pointer

Encoded as **byte offset within the parent directory** rather than a
sector + offset. The on-disk FCB lives at:
```
record = o_dirfil->o_strtcl-derived sector + (o_dirbyt / recsiz)
offset = o_dirbyt % recsiz
```

For root directories (which don't have an OFD): `o_dirfil = NULL`,
`o_dirbyt` is the sector*512 + offset within the fixed root dir
area.

## OFD pool sizing

```c
#define OPNFILES   75              /* max open files in system */
#define NUMSTD      6              /* std handles 0..5 */
#define NUMHANDLES (NUMSTD+OPNFILES) /* = 81 */
```

The system file table `sft[OPNFILES]` (in `bdos/fsmain.c`) is the
fixed-size slab. Per-process `PD.p_uft[6]` holds index into `sft[]`.

## File-position invariants

- `o_bytnum` is the **byte offset** read/write would happen at next.
- `o_curcl` is the cluster containing `o_bytnum` (or the last cluster
  written at EOF).
- `o_currec` is the record (sector) within that cluster, expressed
  as an absolute device record number (0 = boot sector).
- `o_curbyt` is `o_bytnum % clsizb`.
- `Fseek` updates all four; `Fread`/`Fwrite` advance them as bytes
  are transferred.

The four position fields are redundant — kept in sync to amortise
the cluster-walk cost across multiple small reads.

## Cross-references

- **Compendium:** the OFD/FCB layout is documented in the file
  system internals chapter.
- **Profibuch:** "FCB" / "OFD" — agrees, including the 64-byte
  budget.
- **`docs/api/structures/dnd.md`** — DND/DMD link in via
  `o_dnode`/`o_dmd`.
- **`docs/api/gemdos/file/3d-fopen.md`** — allocates an OFD.
- **EmuTOS source:** `bdos/fsmain.c:fopen, fclose`,
  `bdos/fsio.c:rwfd`, `bdos/fsfat.c:nextcl`.

## Configuration

- `OPNFILES = 75` — fixed by ABI for binary compatibility.
- `INH_MODE` / `MODE_FSM` accepted but not enforced.
- `FNAMELEN = LEN_ZNODE + LEN_ZEXT = 11`.

## NewTOS implementation notes

(Empty during Phase 00. PHASE-05 implements `Fopen`/`Fread`/`Fwrite`
against this structure.)

# `PD` — Process Descriptor / Basepage

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `include/bdosdefs.h:65-95`. Initialisation in `bdos/proc.c` and
> `bdos/kpgmld.c`.

## Purpose

Per-process control block. Created at process load time; lives at the
**start** of the TPA (transient program area) of the loaded program,
so the basepage pointer doubles as the TPA base. The basepage
*is* the first 0x100 bytes of the process address space.

The currently-running PD is reachable via `os_run` in the OS header
(double-indirect: `*OSHEADER.os_run` → current `PD*`).

## Binary layout (256 bytes total, 6 bytes per row max in groups of 16)

```c
#define NUMSTD     6                    /* standard handles 0..5 */
#define NUMCURDIR  BLKDEVNUM            /* one per drive (26 in this build: A:..Z:) */
#define PDCLSIZE   0x80                 /* command-line buffer */

struct _pd {
/* 0x00 */ UBYTE  *p_lowtpa;            /* TPA start (== &this) */
/* 0x04 */ UBYTE  *p_hitpa;             /* TPA end + 1 */
/* 0x08 */ UBYTE  *p_tbase;             /* text segment base */
/* 0x0C */ LONG    p_tlen;              /* text segment length */
/* 0x10 */ UBYTE  *p_dbase;             /* data segment base */
/* 0x14 */ LONG    p_dlen;              /* data segment length */
/* 0x18 */ UBYTE  *p_bbase;             /* bss segment base */
/* 0x1C */ LONG    p_blen;              /* bss segment length */
/* 0x20 */ DTA    *p_xdta;              /* current DTA pointer */
/* 0x24 */ PD     *p_parent;            /* parent's PD */
/* 0x28 */ ULONG   p_flags;             /* PF_* flags */
/* 0x2C */ char   *p_env;               /* environment string ptr */
/* 0x30 */ SBYTE   p_uft[6];            /* std-handle redirection table */
/* 0x36 */ char    p_lddrv;             /* drive program was loaded from */
/* 0x37 */ UBYTE   p_curdrv;            /* current drive */
/* 0x38 */ LONG    p_1fill[2];          /* reserved */
/* 0x40 */ UBYTE   p_curdir[NUMCURDIR]; /* per-drive current-dir index */
/* 0x40+N */ char  p_2fill[32-NUMCURDIR]; /* pad to 0x60 */
/* 0x60 */ LONG    p_3fill[2];          /* reserved */
/* 0x68 */ LONG    p_dreg[1];           /* saved d0 */
/* 0x6C */ LONG    p_areg[5];           /* saved a3..a7 */
/* 0x80 */ char    p_cmdlin[0x80];      /* command line: len byte + ASCII */
/* 0x100 — end */
};
```

The 256-byte total length is **fixed and binary-compatible** with
Atari TOS. Programs index into the basepage by absolute byte offsets;
the layout cannot be changed.

## Field semantics

### Memory layout (0x00–0x1F)

- `p_lowtpa` always equals the address of the basepage itself.
  `p_hitpa - p_lowtpa` is the total TPA size.
- Text/data/bss segments live above the basepage. Layout produced by
  `kpgmld` (`bdos/kpgmld.c:140-155`):
  ```
  p_lowtpa = basepage
  p_tbase  = basepage + 0x100
  p_dbase  = p_tbase + p_tlen
  p_bbase  = p_dbase + p_dlen
  free TPA = p_bbase + p_blen .. p_hitpa
  ```
- The 0x100-byte gap between basepage end and text base is the
  basepage itself: `p_tbase = p_lowtpa + sizeof(PD)`.

### `p_xdta` (0x20)

Per-process current DTA pointer. Set by `Fsetdta` (GEMDOS 0x1A) and
read by every `Fsfirst`/`Fsnext`. Defaults to `p_cmdlin` (offset
0x80) — Atari quirk: the command line buffer doubles as the initial
DTA. **Writing the DTA without first calling `Fsetdta` will scribble
over the command line.**

### `p_parent` (0x24)

The PD that was `run` when this one was **started** — written only by
`proc_go` (`bdos/proc.c:541`, `p->p_parent = run`); `init_pd_fields`
zeroes it before (`bdos/proc.c:370`). So: an AUTO program and the AES
get `&initial_basepage` (`bios/bios.c:685-695,855-866`,
`bdos/bdosmain.c:337-342`), the AES gets `&initial_basepage` too, the desktop
and every program it or the `#Z` line starts get the **accdesk PD** — the
process `aes_run_rom_program(accdesk_start)` makes with the AES PD as its
parent (`aes/geminit.c:894`, `aes/gemshlib.c:570-581,606,645`) — so their
chain is program → accdesk → AES PD → `initial_basepage` → 0; a program started
by EmuCON has the **EmuCON PD** (`cli/cmdexec.c:52`, `aes/gemshlib.c:614-620`) in
front of accdesk; a desk accessory — loaded with `PE_LOAD`, never `proc_go`ed —
keeps **0** (`util/gemdos.c:29-47`); a `Pexec` child gets `run` — its caller's PD,
except that a call made by an accessory has `run` = the PD in front
([`_run-ownership-curdir.md`](../gemdos/process/_run-ownership-curdir.md) §1.3). `initial_basepage` itself has
`p_parent == 0`. (**Corrected 2026-10-18, TASK/PROC03:** earlier text named
the AES PD as the parent of the desktop and its programs.) mintlib tells an
accessory by `p_parent == 0` (`sdk/libc/mintlib/startup/crt0.S:52-56`). On
`Pterm`/`Pterm0`/`Ptermres`, control returns to `p_parent`'s context.

NewTOS: the PDs the kernel has no process for (`initial_basepage`, the AES PD
and, per the 2026-10-18 decision, the accdesk and EmuCON PDs) are stand-ins in
a published kernel page — see
[`../gemdos/process/_parent-and-environment.md`](../gemdos/process/_parent-and-environment.md)
(TASK/PPARENT, Way A).

### `p_flags` (0x28)

```c
#define PF_FASTLOAD   0x0001  /* skip BSS clear (Atari "fastload" bit) */
#define PF_TTRAMLOAD  0x0002  /* program may be loaded into TT-RAM */
#define PF_TTRAMMEM   0x0004  /* malloc may use TT-RAM */
#define PF_STANDARD   (PF_FASTLOAD|PF_TTRAMLOAD|PF_TTRAMMEM)
```

These flags come from the program-header `PRGFLAGS` field of the
loaded executable.

NewTOS: `PF_TTRAMLOAD` and `PF_TTRAMMEM` map to "may use beyond-32-bit
or normally-non-DMA memory" — the AArch64 equivalent is whether
allocation can come from any RAM page. `PF_FASTLOAD` is honoured
verbatim.

### `p_env` (0x2C)

Pointer to a NUL-terminated chain of `KEY=value\0` strings, with a
double-NUL terminator. Inherited from parent unless `Pexec` was called
with a non-NULL env argument. **Every** process gets one (`alloc_env`
runs for every load and basepage mode, `bdos/proc.c:215,283,413-432`):
an AUTO program a copy of `initial_basepage`'s empty `{0, 0}`
(`bdos/bdosmain.c:81,341`), the AES a copy of `default_env`
`"PATH=\0X:\\\0\0"` (`bios/bios.c:77-80,626-636,862`), the desktop,
its programs and the accessories a copy of the AES's (`ad_envrn`,
`aes/gemstart.S:78-96`, `aes/gemshlib.c:645`, `util/gemdos.c:34`; for the
desktop and the accessories the copy is of the accdesk PD's block, which is a
copy of the AES's, `bdos/proc.c:419-420`). Details:
[`../gemdos/process/_parent-and-environment.md`](../gemdos/process/_parent-and-environment.md).

### `p_uft[6]` (0x30)

Standard-handle redirection table. Indices `0..5` correspond to the
six standard handles:

| Idx | Handle name           | Default device          |
|----:|-----------------------|-------------------------|
| 0   | stdin                 | CON: (BIOS handle 2)    |
| 1   | stdout                | CON:                    |
| 2   | stderr                | CON:                    |
| 3   | stdaux                | AUX: (BIOS handle 1)    |
| 4   | stdprn                | PRN: (BIOS handle 0)    |
| 5   | reserved              | unused                  |

Values: positive → index into the global `sft[]` (system file table)
for redirected files; negative → BIOS-handle indicator
(`-1` = no redirection, BIOS character device default applies). See
`docs/api/gemdos/_dispatcher.md` for the redirection logic in
`osif()`.

### `p_lddrv` (0x36) / `p_curdrv` (0x37)

`p_lddrv`: drive number the program was loaded from (0=A, 1=B, …).
`p_curdrv`: drive number `Dgetdrv` should return for this process.
Both are per-process (`Dsetdrv` updates `p_curdrv`, not a global).

### `p_curdir[]` (0x40)

Per-drive current-directory index into the system DND tree. One byte
per drive. `p_curdir[d]` is an index into `dirtbl[]`; the actual DND
pointer is `dirtbl[p_curdir[d]]`.

**Correction:** `BLKDEVNUM = 26` in this build (`include/sysconf.h:43`,
drives A:..Z: — see `docs/api/_discrepancies.md` D-025), not 16. The array
therefore uses 26 of the 32 reserved bytes; the remaining 6
(`p_2fill[32-NUMCURDIR] = p_2fill[6]`) pad out to keep `p_3fill` at the fixed
offset 0x60 regardless of `BLKDEVNUM`.

### `p_dreg`/`p_areg` (0x68/0x6C)

Used during `Pexec` to save and restore the parent's registers
across a child invocation. Layout: `p_dreg[0]=d0`,
`p_areg[0..4]=a3..a7`. **d1, d2, a0–a2 are not saved here** — they
are scratch in the C ABI, plus the trap stub already saves what it
needs. NewTOS will need an analogous register-save layout in the
m68k user emulator; not relevant for native AArch64 source ports.

### `p_cmdlin[0x80]` (0x80–0xFF)

Command line image. Format:
- `p_cmdlin[0]`: length byte (0..125).
- `p_cmdlin[1..]`: ASCII command tail, **terminated by `\r` (0x0D)**,
  not NUL. The trailing CR is *not* counted in the length.
- Bytes after the CR are zeroed but unspecified to caller.

The length byte cannot exceed 125 because the structure ends at 256
and the CR + slack must fit. `Pexec` truncates longer command lines.

When `Pexec` is given a NULL command tail, `p_cmdlin[0] = 0`,
`p_cmdlin[1] = '\r'`.

Also doubles as the initial DTA buffer (see `p_xdta`).

## Initial state at process entry (m68k convention)

When the freshly loaded program starts (jumped to via the `Pexec`
trampoline in `bdos/proc.c:540`):

| Register | Value                                   |
|----------|-----------------------------------------|
| `a0`     | 0 (no AES context yet)                  |
| `a3`     | basepage address (== `p_tbase` - 0x100) |
| `a4..a7` | undefined (set by program prologue)     |
| `4(sp)`  | basepage address (C-callable convention)|
| `0(sp)`  | return address (back into `gouser`)     |

Compendium documents `4(sp)` as the basepage on entry; this is what C
startup code reads as `argv[0]` substitute.

NewTOS source-ports won't see this convention — they enter with the
AArch64 ABI. The m68k convention is preserved only inside the m68k
emulator (Phase 10+).

## Allocation and lifetime

- Allocated by `Pexec mode 0/3/5` from the parent's free TPA pool
  (`bdos/proc.c:bxexec` → `bdos/mem.c:xmalloc`).
- Freed by `Pterm`/`Pterm0`/`Ptermres`. `Ptermres` keeps a portion
  resident (TSR pattern).
- Memory ownership is tracked via `MD.m_own == &PD` in the memory
  descriptor list.

## Cross-references

- **Compendium:** "Basepage" — agrees on layout.
- **Profibuch:** "Die Basepage" — agrees.
- **`docs/api/structures/md.md`** — MD points back to PD via `m_own`.
- **`docs/api/gemdos/_dispatcher.md`** — `p_uft[]` redirection.
- **`docs/api/structures/dta.md`** — `p_xdta`.
- **EmuTOS source:** `bdos/proc.c:340-410` (basepage init in `bxexec`),
  `bdos/kpgmld.c:120-160` (segment fields populated by loader).

## Configuration

- `NUMCURDIR = BLKDEVNUM = 26` (`include/sysconf.h:43`) — a plain,
  unconditional `#define`; **not** gated by `CONF_WITH_TT_SHIFTER` or any
  other `CONF_WITH_*` (an earlier draft claimed 16-with-that-gate; wrong on
  both the value and the conditional — see `docs/api/_discrepancies.md`
  D-025/D-066).
- `NUMSTD = 6` — fixed by ABI.
- `PDCLSIZE = 0x80` — fixed by ABI.

## NewTOS implementation notes

- The loader (`tos-api/src/bdos/process/loader.rs`, `populate_basepage`)
  reads an ELF, not a PRG header. The six segment fields come from the
  `PT_LOAD` table: `p_tbase`/`p_tlen` = the first executable segment
  (file part), `p_dbase`/`p_dlen` = the first writable non-executable
  segment's file part, `p_bbase`/`p_blen` = that segment's `memsz - filesz`
  tail. `sdk/newtos.ld` emits exactly those two segments back to back, so the
  EmuTOS relations hold to the byte: `p_dbase = p_tbase + p_tlen`,
  `p_bbase = p_dbase + p_dlen` (`bdos/kpgmld.c:131-132`), and
  `p_bbase + p_blen` is the linker's `_end` (TASK/T05; loader host tests
  `basepage_segment_tests`, APITEST T32). `p_tbase` is `basepage + 0x1000`,
  not `basepage + 0x100`: the program is a position-independent image linked
  at `0x1000` that keeps its 4-KiB page offsets when the loader relocates it
  for its TPA block (ARCH03 plan §3.5). An image with a single RWX segment
  (no data) leaves the four data/BSS fields zero.
- **Where the basepage is (ARCH03 stage 1).** Every user address is its own
  physical address in one address space all processes share; the basepage
  is the start of the process's TPA block, a page-aligned BDOS-pool block
  (EmuTOS `p_lowtpa = p`, `bdos/proc.c:372`). That address is the process's
  **PD pointer**: `Pexec(PE_LOAD)` returns it, `p_parent` holds the parent's
  (set when the child is started, EmuTOS `proc_go`, `bdos/proc.c:541`; for
  a program the kernel starts the `initial_basepage` / AES-PD stand-in today,
  TASK/PPARENT — `0` before it; the target after TASK/PROC03 is the accdesk
  stand-in for desktop/`#Z` programs, [`_parent-and-environment.md`](../gemdos/process/_parent-and-environment.md) §4.2), the OSHEADER's `os_run` points at a
  page-0 cell (`0x5B8`) holding it (a desk accessory's cell holds EmuTOS's
  `run` instead, TASK/PROC03 B5, `_run-ownership-curdir.md` §6.6), and the entry gets it in `x0`. All
  address fields (`p_lowtpa`, `p_hitpa`, `p_tbase`, `p_dbase`, `p_bbase`,
  `p_xdta`, `p_env`) are addresses every process can read. Until stage 1 the
  basepage sat at VA 0 of every process (`p_lowtpa = 0`).
- `p_hitpa` = `p_lowtpa` + the length of the largest free block the loader
  took (ARCH03 E9, 2026-10-05: EmuTOS `alloc_tpa`, `bdos/proc.c:460-502`,
  `p_hitpa = p + max` `:374`; until then a fixed 4 MiB). The SDK start-up
  code shrinks the block at once (mintlib `_crtinit`); `p_hitpa` keeps naming
  the original top, as on TOS (`Mshrink` never touches the basepage).

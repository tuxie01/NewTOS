# GEMDOS Subsystem State

> Phase 00 deliverable D7 (GEMDOS). Documents the BDOS-internal state
> referenced by GEMDOS spec entries. Source: EmuTOS @ `1a7484ab`.

This file captures the **shared mutable state** of the BDOS layer.
Per-function specs reference these tables by name; this file is the
single description of their structure, lifecycle, and concurrency
expectations.

> **Observable vs. internal** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> This document is mostly EmuTOS's **internal model**, captured for
> reference — NewTOS keeps the *behaviour*, not these structures.
> **Free to reimplement** (a program cannot observe them): the SFT internals,
> the disk-buffer cache `bufl[]`, `drvtbl[]`/`DMD`, the memory pools
> `pmd`/`pmdalt` layout, the concurrency mechanics. **Contract** (a program
> *does* observe these — keep value/semantics exact): the `PD`/basepage
> fields, the `conterm` bits, the `drvbits` drive map, the per-process DTA
> pointer + the 44-byte DTA the program owns, the OS-header/cookie-jar
> values. When in doubt, apply the audit rubric in COMPATIBILITY.md.

---

## 1. Process descriptor `run` and the basepage/parent chain (`bdos/proc.c:52`)

```c
PD *run;        /* bdos/proc.c:52 — current PD (process descriptor / basepage) */
```

Pointer to the currently-running process. Read by virtually every BDOS call that
needs the calling process's path, DTA, file-handle table, current dirs/drive, or
memory ownership.

**Lifecycle / writers:**
- Initialised at GEMDOS boot to the static `initial_basepage`
  (`bdos/bdosmain.c:339`) — not to a shell or AUTO program; the full timeline
  (AUTO PDs, AES PD, accdesk PD, desktop/EmuCON/program PDs; unchanged while an
  accessory or AES code executes) is in
  [`process/_run-ownership-curdir.md`](./process/_run-ownership-curdir.md) §1.
- `proc_go` (`bdos/proc.c:575`, reached by `Pexec` modes 0/4/6) sets
  `run = child` when launching.
- `xterm`/`x0term`/`xtermres` (`bdos/proc.c:609`) set `run = run->p_parent`
  **before** dismantling the terminating process.

### Parent chain (`PD.p_parent`, basepage offset 0x24)

Each launched process records its launcher in `p_parent`, set by `proc_go`
(`p->p_parent = run`, `bdos/proc.c:541`) at the moment of launch — **not** at
load time. So a `Pexec(3)` (load, don't go) basepage has `p_parent == 0` until it
is actually launched via `Pexec(4/6)`. `Pterm` walks exactly one link
(`run = run->p_parent`) to return control; the exit code is stored into the
**parent's** saved-D0 slot `p_parent->p_dreg[0]` (offset 0x68). The boot process
(`initial_basepage`, `bdos/bdosmain.c:78`) has `p_parent == NULL`; EmuTOS does
not guard a `Pterm` from it (would deref NULL). The AES and the AUTO programs
are its children; the desktop and its programs are children of the **accdesk
PD**, which is the AES PD's child (**corrected 2026-10-18**: this line said "the
AES's"; chains in
[`process/_parent-and-environment.md`](./process/_parent-and-environment.md) §1.1). **Contract:** `p_parent` at offset 0x24 and `p_dreg[0]` at 0x68 are the
observable way a child's exit code reaches its parent's `Pexec`. The full
basepage byte layout is in `docs/api/structures/basepage.md` and
`docs/api/gemdos/process/4b-pexec.md` (PD table).

NewTOS: same `run` pointer + parent link, mapped to the kernel's `Process`
struct; a sentinel init PD catches `Pterm` from the top-level shell instead of
dereferencing a NULL parent. Today `Pterm` returns through the child slot's
`parent_slot`, not through `p_parent`, and `RUN` is `null` in kernel context;
the 2026-10-18 decision (TASK/PROC03) requires EmuTOS's `run`/`p_parent`
semantics — see `process/_run-ownership-curdir.md` §2 and §6.

### Process-owned resources released at termination

`Pterm`/`Pterm0` release **everything** the process owns; `Ptermres` **reserves**
its memory instead of freeing it. Ownership is tracked per resource:
- **Open files** — the six standard handles `p_uft[0..5]` (§2) plus every
  `sft[]` entry with `f_own == run` (§2). Both are closed by `ixterm`
  (`bdos/proc.c:117-123`). Cross-reference §2; not duplicated here.
- **Current directories** — each non-zero `p_curdir[d]` index into `dirtbl[]`
  (§5) is `decr_curdir_usage`'d (`bdos/proc.c:128-132`). Cross-reference §5.
- **Memory** — every MD with `m_own == run` in `pmd`/`pmdalt` (§8), freed by
  `free_all_owned` (`bdos/proc.c:92-101`) or, for `Ptermres`, orphaned by
  `reserve_blocks` (`bdos/proc.c:77-89`). Cross-reference §8 (including the
  `m_own` ownership caveat).

### ETV_TERM — the user/GEM termination vector (`0x102` / `0x408`)

`Pterm`/`Pterm0`/`Ptermres` invoke a user-installed termination handler **before**
teardown. `xterm` fetches it with `Setexc(0x102, -1L)` — exception number
`0x102`, i.e. byte address `4 * 0x102 = 0x408 = _etv_term` (`tosvars.ld:35`,
"GEM program termination vector"; the C global is `etv_term`,
`include/tosvars.h:112`). The `-1` sentinel makes `Setexc` **read without
replacing** (`bios/bios.c:1076`). The handler is called via `protect_v`
(`util/miscasm.S:213`), which preserves `d2`/`a2`. This is the `atexit`/GEM
`etv_term` hook. **Contract:** a program that installs a handler at `Setexc(0x102,
…)` sees it run once, before file/memory teardown, on every `Pterm` variant.
NewTOS maps exception *number* `0x102` to a per-process handler slot (byte address
`0x408` is meaningless on AArch64). See
`docs/api/gemdos/process/4c-pterm.md`.

---

## 2. System File Table (SFT) and per-process handle table

Handles come in two classes, with **two separate tables**:

- **Per-process standard handles 0..5** live in `PD.p_uft[NUMSTD]`
  (`include/bdosdefs.h:84`), a 6-entry array of **signed bytes** (`SBYTE`).
  Each entry is either a **negative BIOS character-device handle** (`H_Console
  = -1`, `H_Aux = -2`, `H_Print = -3`; `bdos/console.h:23-25`) or, once
  redirected by `Fforce`/`Fdup`, the **non-standard handle number** (`>= 6`)
  it now maps to.
- **Global non-standard handles 6..80** are indexed into the **system file
  table** `sft[OPNFILES]` (`FTAB`, `bdos/fs.h:307-312`; declared
  `bdos/fsglob.c`). Each `FTAB` holds `f_ofd` (OFD pointer, or a negative BIOS
  handle), `f_own` (owning PD), and `f_use` (reference count). Handle `h`
  addresses `sft[h - NUMSTD]`.

Sizes (`bdos/fs.h:43-45`, `include/bdosdefs.h:61`):

```
NUMSTD     = 6      /* standard handles 0..5              */
OPNFILES   = 75     /* system-wide open non-standard files */
NUMHANDLES = 81     /* NUMSTD + OPNFILES                   */
```

(Correction vs. earlier draft: `OPNFILES` is **75**, not 32/34; `NUMHANDLES`
is **81**, not 40. The 44-byte-DTA / `p_uft` "points into syshnd" wording was
wrong — `p_uft` holds either a negative BIOS handle or a non-standard handle
number, not an index into a `syshnd[]` array.)

### Default standard-handle wiring (`default_handle[]`, `bdos/console.c:60-68`)

Set at boot and restored by `Fclose` of a standard handle
(`get_default_handle`, `bdos/console.c:132`):

| Handle | `p_uft[h]` default | Device |
|-------:|--------------------|--------|
| 0 (stdin)  | `H_Console` = -1 | `CON:` (BIOS console) |
| 1 (stdout) | `H_Console` = -1 | `CON:` |
| 2 (stdaux) | `H_Aux` = -2     | `AUX:` (serial) |
| 3 (stdprn) | `H_Print` = -3   | `PRN:` (printer) |
| 4          | `H_Console` = -1 | `CON:` (TOS redirects undefined handles) |
| 5          | `H_Console` = -1 | `CON:` |

`getofd(h)` (`bdos/fsmain.c:474`) resolves any handle to an OFD via `syshnd`
(`bdos/fsmain.c:457`): for a standard handle it first follows `p_uft[h]`; a
value `> 0` becomes `sft[value - NUMSTD]`, a value `<= 0` stays a BIOS handle
(no OFD → NULL). Out-of-range handles → NULL.

`Fopen`/`Fcreate` allocate a free `sft[]` slot via `opnfil` (`f_own = run`,
`f_use = 1`); `Fclose` releases it via `xclose`→`sftdel` at `f_use == 0`.
`Fdup` claims a new slot copying the mapping (no source `f_use` bump);
`Fforce` rewires a standard slot and bumps `f_use` for file mappings. See the
per-function specs under `docs/api/gemdos/file/`.

### Handle lifecycle across `Pexec` / `Pterm` (TASK/PROC02)

- **Inheritance** — `init_pd_files` (`bdos/proc.c:387-399`), run for every
  new PD (`Pexec` 0/3 after a successful load, `:350`; 5/7, `:235`): for each
  standard handle of `run`, a value `> 0` is `ixforce(i, h, p)`d into the
  child (so `sft[h-NUMSTD].f_use++`, `bdos/fshand.c:66-74`), every other value
  is copied verbatim. A redirected stdout therefore stays redirected in the
  child, and the file stays open until both have closed it.
- **Termination** — `ixterm` (`bdos/proc.c:110-123`): every `p_uft[i] > 0` of
  the dying PD is `xclose`d, then every `sft[]` entry with `f_own == r`.
- **Re-forcing does not release.** `ixforce` overwrites `p_uft[std]` without
  closing or decrementing what it mapped before (`bdos/fshand.c:46-77`); the
  use count that `Fforce`/inheritance took stays until a `Fclose` of the
  standard handle (`xclose` reverts and closes, `bdos/fsopnclo.c:450-458`) or
  until the owner's `ixterm`.
- **A shared OFD is listed once.** `ixclose(fd, 0)` unlinks the OFD from its
  directory's `d_files` list (`bdos/fsopnclo.c:552-566`); a later `ixclose` of
  the same OFD (the second close of a handle `Fforce` or inheritance shared)
  writes the dirty directory entry, finds no list entry and returns `EINTRN`
  without flushing the buffers. `xclose` decrements `f_use` either way
  (`:492-493`); `sftdel` frees the OFD only when no other `sft[]` entry
  points at it (`sftofdsrch`, `:362-410`).

NewTOS (TASK/PROC02): `sft[]` entries (`f_ofd`, `f_own`, `f_use`) and the
OFDs they point at are separate tables in `tos-api/src/bdos/fs/file.rs`; an
OFD carries a `listed` flag for its `d_files` membership. `f_own` is the
running PD, or — kernel code with no running process — the stand-in PD that
is EmuTOS's `run` there (`standin::kernel_run`, TASK/PPARENT).

**NewTOS deviations from the above — re-validated 2026-10-18 against EmuTOS
(TASK/PROC04); point 3 NOT yet repaired (points 1 and 2 repaired, see
there); the EmuTOS behaviour is the target:**

1. *Character functions ignore `p_uft[]`.* **Repaired 2026-10-18 (TASK/PROC04
   point 1 = TASK/CON02):** `gemdos/mod.rs` `dispatch_inner` runs
   `console::redirect_to_file` before the table (the `osif` file block,
   `_dispatcher.md` §5: raw one-byte reads sign-extended / `EOF_INDICATOR`,
   writes return 0, `Cconrs` byte-wise to CR with the following byte eaten
   and `max` read unsigned — EmuTOS reads it as a signed `char`, named
   deviation 2026-10-18 in `docs/COMPATIBILITY.md` —, `Cconis`/`Cauxis` 0 only at EOF, output
   status -1, `Crawio` excluded); the 15 bodies take their BIOS device from
   `HXFORM(run->p_uft[n])` (`console::std_dev`). `run` without a running
   process is the stand-in PD (`file::std_handle` → `kernel_run_uft`, always
   `default_handle`), so kernel code keeps CON/CON/AUX/PRN. Host test
   `tos-api/tests/proc04_console_redirect.rs`; QEMU evidence
   `measurements/measurement-2026-10-18-proc04-redir/`. Before:
   `gemdos/console.rs` hard-wired
   `DEV_CON`/`DEV_AUX`/`DEV_PRN` (`cconout :88`, `cauxin :118`, `cauxout :132`,
   `cprnout :149`, `crawio :171`, `cconin :228`, `crawcin :253`, `cnecin :267`,
   `cconrs :462`, `cconws :514`, `cconis :559`, `cconos :569`, `cprnos :581`,
   `cauxis :592`, `cauxos :603`) and `gemdos/mod.rs` `dispatch_inner` has no
   pre-dispatch step. EmuTOS: `osif` `bdos/bdosmain.c:498-568` (see
   `_dispatcher.md` §5, which was corrected the same day) **plus** the
   `p_uft`-derived BIOS device in `bdos/console.c` (`HXFORM(run->p_uft[n])`,
   `:161,170,179,188,197,281,333,342,377,401,419,434,447,451,461,535`) — both
   halves are needed: a redirected-to-file handle is answered in `osif`, a
   BIOS handle other than the default (`Fforce(1,-2)` etc.) is answered by
   the `console.c` functions. NewTOS today has neither.
2. *Open-file table.* **Repaired 2026-10-18 (TASK/PROC04 point 4):** NewTOS
   `OPEN_FILES = 75` (`file.rs`) → `NUMHANDLES` 81, handles 6..80; every
   allocator (`Fopen`/`Fcreate` via `open_handle`, `Fdup`) goes through
   `claim_sft` and answers `ENHNDL` on a full table; `Fcreate`'s directory
   entry is written before the claim, as in `ixcreat` (`bdos/fsopnclo.c:
   186-201`). Host test `tos-api/tests/proc04_open_file_table.rs`. Before:
   `OPEN_FILES = 16` → `NUMHANDLES` 22. EmuTOS `OPNFILES = 75` → 81
   (`bdos/fs.h:43-45`, unconditional — no
   `CONF_WITH_*`/`TARGET_*` gate; the comment `bdos/fs.h:34-41` lists TOS
   1.04/2.06/3.06 = 75, TOS 4.04 = 19 "[likely a per-process number]", i.e.
   the Falcon/TOS4 *profile* does not change EmuTOS's own value). The table
   is **system-wide**; there is no per-process handle limit (a process owns
   only the 6 `p_uft[]` slots). Full table → `ENHNDL` (`opnfil`
   `bdos/fsopnclo.c:341-346`, `xdup` `bdos/fshand.c:100-105`).
3. *`Fdelete` of an open file* — **repaired 2026-10-18 (TASK/PROC04 point 5)**:
   `file.rs` `ixdel` as EmuTOS (own OFD `ixclose`d, handles stay valid, later
   `Fclose` `EINTRN`; another process's → `EACCDN`), plus the sanctioned
   protection that nothing through a deleted file's handle reaches the card
   (`docs/COMPATIBILITY.md`; details `file/41-fdelete.md` "PROC04 point 5").
   Before: audit fix "F6" invalidated every handle.

(Point 3 of the PROC04 ticket — `Fread`/`Fwrite` on negative handles — was
already repaired by FS11 on 2026-10-13: `file.rs:393-394,441-442`
`arg_i16`, `resolve` `:231`, `gemdos/console.rs` `device_read`/`device_write`.
Standard handles whose `p_uft`/`f_ofd` is a BIOS device reach AUX/PRN through
the device table, not the console.)

---

## 3. Disk Buffer Cache `bufl[]` (`bdos/fsbuf.c`)

Two singly-linked lists of cached disk sectors, indexed by `BI_FAT = 0` /
`BI_DATA = 1` (`bdos/fs.h:278-279`):

```c
BCB *bufl[2];
/* bufl[BI_FAT]  (index 0) — FAT-sector buffers ONLY (buftype BT_FAT) */
/* bufl[BI_DATA] (index 1) — root-directory AND ordinary dir/file data
                             buffers, COMBINED (buftypes BT_ROOT + BT_DATA);
                             getbcb() maps both to BI_DATA */
```

**Correction:** an earlier draft labelled these "`[0] = FAT/dir buffers, [1]
= data buffers`" — wrong split. `getbcb()`'s own selector is
`phdr = &bufl[buftype==BT_FAT ? BI_FAT : BI_DATA]` (`bdos/fsbuf.c:121`): only
`BT_FAT` gets its own list; **root-directory sectors share the *data* list**
with ordinary file/subdirectory sectors (`getrec()`'s three-way `BT_FAT` /
`BT_ROOT` / `BT_DATA` classification collapses to two physical lists,
`bdos/fsbuf.c:218-222`).

Each `BCB` (`bdos/fs.h:290-299`) holds `b_link`, `b_bufdrv`, `b_bufrec` (lba),
`b_buftyp`, `b_dirty`, `b_dm`, `b_bufr` (pointer to the actual sector buffer).

**Correction:** the cache is **not** "4 buffers per list" — `NUMBUFS = 2`
(`bdos/fsbuf.c:26`), a plain `#define` with no `CONF_WITH_*` gating, so there
are exactly **2 buffers in each of the 2 lists (4 physical sector buffers
system-wide)**, sized `sizeof(BCB) + pun_ptr->max_sect_siz` each
(`bufl_init()`, `bdos/fsbuf.c:56-73`). This is small enough that a single
file write (dirtying a data-cluster buffer) competing with its own directory
entry's flush (a root/dir-cluster buffer sharing the *same* 2-slot `BI_DATA`
list) can evict each other under light concurrent access to more than 2
distinct root/dir/data sectors. `getbcb()` does use MRU-at-head /
LRU-at-tail list-head insertion for eviction order (`bdos/fsbuf.c:130-201`)
— that part of the original description was correct.

Critical invariants:
- `b_dirty == 1` requires writeback before eviction (checked in `getbcb()`,
  `bdos/fsbuf.c:162-163`).
- **Correction:** there is **no** `flush(drive)` or no-arg `flush()` API.
  `void flush(BCB *b)` (`bdos/fsbuf.c:85-105`) always writes back exactly
  **one** buffer (plus its mirror in the second FAT copy, if `b` is a
  `BT_FAT` buffer on a 2-FAT medium). "Flush every dirty buffer for every
  drive" is achieved by **callers looping over both lists themselves** — e.g.
  `ixclose()`'s full-close path (`bdos/fsopnclo.c:576-579`, `for (i=BI_FAT;
  i<=BI_DATA; i++) for (b=bufl[i]; …) if (dirty) flush(b);`) flushes dirty
  buffers for **all** drives on every real file close, not just the closed
  file's drive. There is a *separate*, non-flushing mechanism for
  media-change recovery: `mark_bcbs_invalid(drv)`
  (`bdos/bdosmain.c:409-424`) merely sets `b_bufdrv = -1` for the affected
  drive's buffers — it **discards** any dirty content instead of writing it
  back (the medium is assumed gone). See `docs/api/_discrepancies.md` D-065.

NewTOS: replace with a write-back page cache in the FAT driver;
preserve the "close flushes all drives, media-change discards without
flushing" observable behaviour, not literally a `flush()` API shape.

### Replacement policy and FAT mirroring (`getbcb`, `flush`) — FS08

Extracted 2026-10-07 from EmuTOS @ `1a7484ab` for TASK/FS08 (part 2):

- **Which list.** `getrec(recn, ofd, wrt)` classifies the access by the OFD
  — the drive's "FAT file" OFD (`m_fatofd`) → `BT_FAT`, an OFD without a
  directory node → `BT_ROOT`, everything else → `BT_DATA`
  (`bdos/fsbuf.c:218-222`) — and `getbcb` searches **only** that type's list
  (`bdos/fsbuf.c:121`), matching drive, type and record
  (`bdos/fsbuf.c:132`). Data and directory traffic therefore can never
  displace a FAT sector, and vice versa.
- **Hit.** The buffer moves to the head of its list (`bdos/fsbuf.c:197-199`):
  the list is kept in most-recently-used order.
- **Miss.** The victim is the **last invalid** buffer met during the search
  (`b_bufdrv == -1`, `bdos/fsbuf.c:137-138`, `:146-147`); if every buffer is
  valid, the **tail** of the list — the least recently used one
  (`bdos/fsbuf.c:154-157`). A dirty victim is written back first
  (`:162-163`), then the record is read into it (`:165`) and it becomes the
  list head. A write access (`getrec(…, 1)`) reads the record too and then
  marks the buffer dirty (`bdos/fsbuf.c:231-232`); EmuTOS has no "overwrite
  without reading" path in the cache.
- **FAT mirror.** Only one FAT copy is cached: `m_recoff[BT_FAT] = fatrec`
  (`bdos/fsdrive.c:241`), the **second** FAT on a two-FAT medium. `flush`
  writes the buffer there and, unless `m_1fat`, the same 512 bytes once more
  `m_fsiz` records earlier, into the first FAT (`bdos/fsbuf.c:95-102`). A
  `clfix` (`bdos/fsfat.c:36-93`) therefore dirties exactly one buffer, and
  both copies come out identical at write-back.
- **Number of buffers.** `NUMBUFS = 2` per list (`bdos/fsbuf.c:26`). The
  count is not part of the contract (programs may append BCBs to `bufl[]`,
  which is the API, but rely on no particular count).

NewTOS (FS08, `tos-api/src/bdos/fs/cache.rs`): two separate lists, FAT and
data (directory + root + file data), each with the policy above (MRU order,
last invalid buffer else LRU tail, dirty victim written back first, FAT
buffers cached once and written to every FAT copy). 8 buffers per list instead
of 2 — the ST-RAM economy behind `NUMBUFS = 2` does not apply, and the count
is unobservable. Two named deviations stay: a write access to a *whole* sector
does not read the old contents first (unobservable — all 512 bytes are
replaced), and a failed write-back keeps the buffer (HW03, see the module
comment) instead of `b_bufdrv = -1`.

### 3a. The file transfer engine `xrw` (`bdos/fsio.c`) — FS08

`Fread`, `Fwrite` and the directory reader all move bytes through one engine,
`xrw(wrtflg, ofd, len, ubufr)` (`bdos/fsio.c:208-284`), driven by four OFD
position fields (`docs/api/structures/ofd.md`):

| Field | Meaning |
|---|---|
| `o_bytnum` | byte position in the file |
| `o_curcl` | **current cluster**: the cluster holding the last byte moved (0 = none yet) |
| `o_currec` | first record of `o_curcl`, data-area relative (`cl2rec`, `bdos/fsfat.c:23-26`) |
| `o_curbyt` | bytes of `o_curcl` already behind the position, `0..=clsizb` |

`o_curbyt == 0` and `o_curbyt == clsizb` both mean "`o_curcl` is used up —
the next byte lives in the **next** cluster": `addit` (`bdos/fsio.c:33-47`)
adds a transfer to `o_curbyt` only when it is smaller than a cluster, so after
a run of whole clusters `o_curbyt` stays at the 0 that `nextcl` left
(`bdos/fsfat.c:290`).

**No chain walk per call.** The engine only ever steps **one** FAT link at a
time, from `o_curcl`: `nextcl(ofd, wrtflg)` (`bdos/fsfat.c:247-293`) reads the
FAT entry of `o_curcl` (or takes `o_strtcl` when `o_curcl == 0`), and on a
write past the end of the chain allocates a cluster with `findfree(o_curcl)`
— from the start of the FAT only for the file's *first* cluster
(`findfree16`, `bdos/fsfat.c:215-216`), otherwise scanning forward from the
current cluster with wrap-around (`:223-231`) — links it (`clfix`) and, for
the first cluster, sets `o_strtcl` and `O_DIRTY` (`:269-283`). An end-of-chain
or a cluster number `< 2` ends the transfer (`:285-286`).

**Three parts per call** (`bdos/fsio.c:196-206`):

1. **Head bytes** — the request starts inside a record (`o_curbyt % recsiz !=
   0`): `min(len, recsiz - bytn)` bytes are copied through the buffer cache
   (`getrec`, `:229-245`).
2. **Whole records** — `xrw_recs` (`bdos/fsio.c:84-187`):
   - *header records*: the rest of the current cluster, if the position is
     inside it (`:98-117`);
   - *whole clusters*: one `nextcl` per cluster; consecutive clusters whose
     records are contiguous are **merged into one transfer** until a gap, the
     end of the request, the end of the chain or `CNTMAX = 0x7FFF` records
     (`:22`, `:130-169`);
   - *tail records*: after one more `nextcl`, the leading records of the last
     cluster (`:174-184`).
   Every one of these is a single `usrio` (`bdos/fsio.c:57-74`): **one `Rwabs`
   straight into (or out of) the caller's buffer**, after any cached data
   buffer of the same drive inside the record range has been written back if
   dirty and invalidated (`:61-71`). Whole records never pass through the
   cache.
3. **Tail bytes** — the request ends inside a record: if the position is at a
   cluster boundary (`o_curbyt / recsiz` is 0 or `clsiz`), `nextcl` first;
   then the bytes are copied through the cache (`:261-278`).

A write's head/tail record is read-modify-write through the cache (`getrec(…,
1)` reads the old record). The return value is `o_bytnum - bytpos`, i.e. the
bytes actually moved; reaching the end of the chain (`nextcl` fails, e.g. disk
full on a write) ends the transfer with the partial count (`:255-257`, `:280-283`).

**`Fseek` (`ixlseek`, `bdos/fsio.c:354-426`).** `n == 0` clears all four
fields. Otherwise it walks the chain to the cluster of `n` — **forward from
`o_curcl`** when one is loaded and `n >= o_bytnum` (counting the current
cluster as not yet entered when `o_curbyt` is 0 or `clsizb`, `:378-396`),
otherwise from `o_strtcl` — and stops **one cluster short** when `n` is on a
cluster boundary (`:410-411`), because `xrw` steps (`nextcl`) before it
transfers at a boundary. A chain shorter than the walk is `EINTRN`
(`:413-418`). It then sets `o_curcl`, `o_currec`, `o_bytnum = n`,
`o_curbyt = n % clsizb` (`:420-423`).

**Write-back.** `xrw` itself writes nothing back: dirty head/tail and FAT
buffers stay in the cache until they are evicted (`getbcb`), overlapped by a
later `usrio`, or flushed by `ixclose` (§3, `bdos/fsopnclo.c:576-579`).

NewTOS (FS08, `tos-api/src/bdos/fs/fsio.rs`): the same engine — the four
position fields per OFD, `nextcl` one link at a time, `ixlseek`, the three-part
split with whole records transferred directly between the device and the
caller's buffer (the caller's range checked and reached through its kernel
alias, as `Rwabs` does), contiguous clusters merged up to `CNTMAX`, head/tail
through the data list. `findfree` follows EmuTOS (first cluster from the FAT
start, later clusters forward from the current one). The program loader's
range reads (`FileImage`) use the same engine with a private position. NewTOS
additionally writes the cache back at the end of every `Fwrite` (HW03: that is
how a device error becomes the call's `EWRITF` instead of surfacing at a later,
unrelated call); this is stricter than EmuTOS and loses nothing.

---

## 4. Drive Descriptors and the current-drive model

### Drive count: `BLKDEVNUM = 26` (A:..Z:)

`BLKDEVNUM = 26` (`include/sysconf.h:43`), so drives are **A:..Z:**, not
A:..P:. `drvbits`/`Drvmap()` return a 32-bit bitmap with bits 0..25 usable
(`A:` = bit 0 … `Z:` = bit 25). (Correction vs. earlier draft, which used 16 /
A:..P: throughout — see `_discrepancies.md` D-025.)

### `drvtbl[]`, `drvbits`, `drvsel`, `drvrem`

```c
DMD *drvtbl[BLKDEVNUM];     /* bdos/fsglob.c:28 — one DMD per drive, NULL if not logged in */
LONG drvsel;               /* bdos/fsdrive.c:59 — mask of drives selected since power-up */
```

- **`drvbits`** — the BIOS's `Drvmap()` return value (BIOS layer,
  `bios/blkdev.c:833`, `drvbits`). Bitmap of **present** block devices. Read by
  `xsetdrv`/`xgetdir` via `Drvmap()`.
- **`drvrem`** — BIOS bitmap of **removable** drives (`include/biosext.h:36`);
  read by `ckdrv` to decide whether to run a media-change check.
- **`drvsel`** (BDOS) — drives logged in since power-up; set the first time a
  drive is accessed. Read/written by `ckdrv`.
- **`drvtbl[d]`** — points to a Drive Media Descriptor for drive `d` once it has
  been logged in; created lazily by `getdmd()`/`log_media()` on first reference.

### `DMD` (Drive Media Block, `bdos/fs.h:240-266`)

Holds BPB-derived geometry, FAT cache pointers, and the root-directory DND.
Fields read by the directory/drive group:
`m_numcl` (`CLNO`, total data clusters), `m_recsiz` (`UWORD`, bytes/sector),
`m_clsiz` (`WORD`, sectors/cluster), `m_16` (16-bit-FAT flag). These four (three
plus a computed free count) are exactly what `Dfree` returns — see §11.

Created lazily by `getdmd()` (`bdos/fsdrive.c:140`) on first reference to a drive.

### Current drive: `PD.p_curdrv` (per-process) — there is **no** global `curdrv`

The current default drive is **per-process**, stored in `PD.p_curdrv`
(`UBYTE`, basepage offset **0x37**, `include/bdosdefs.h:86`):

- Read by `xgetdrv` (Dgetdrv 0x19, `bdos/fsmain.c:440`) and by `xgetfree`/
  `xgetdir`/`dcrack` when the drive argument is 0 ("current").
- Written by `xsetdrv` (Dsetdrv 0x0E, `bdos/fsmain.c:423`) — only when the new
  value is in `0..BLKDEVNUM-1`; otherwise unchanged (silent ignore).
- Inherited by children at `Pexec` and thereafter diverges per process.

(Correction vs. earlier draft: there is **no** global `WORD curdrv` variable in
EmuTOS. The current drive lives only in `run->p_curdrv`. See `_discrepancies.md`
D-025.)

**Drive-number conventions differ across this group** (a frequent off-by-one
source): `Dsetdrv`/`Dgetdrv` are **0-based** (`0 = A:`); `Dfree`/`Dgetpath` use
the **`+1`** convention (`0 = current`, `1 = A:`).

### 4a. Drive login and media change (`ckdrv`, `getbcb`, `osif`) — [FS20]

Extracted 2026-10-28 from EmuTOS @ `1a7484ab` for TASK/FS20. Measured with
the probe `tosdump/probes/gcprobe.c` (oracle and NewTOS,
`measurements/measurement-2026-10-28-fs20/RUNINFO.TXT`; the first,
NewTOS-before-the-fix run: `measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT`,
section "GEMDOS consequence").

**When the BDOS calls `Getbpb` [FS20].**

1. **Log-in.** `ckdrv(d, checkrem)` (`bdos/fsdrive.c:77-132`), called by
   every path-resolving call (`dcrack`, `bdos/fsdir.c:1715`), `Dfree`
   (`bdos/fsfat.c:345`) and `Dgetpath` (`bdos/fsdir.c:1237`, with
   `checkrem = FALSE`): a drive whose bit in `drvsel` is clear is logged in —
   `Getbpb(d)`, then `log_media(b, d)` builds the DMD from that BPB
   (`bdos/fsdrive.c:94-104`, `:184-249`) and the bit is set. `Getbpb` NULL →
   `EPTHNF` for a removable drive (`drvrem`), else `EDRIVE`; `log_media`
   failing → `ENSMEM`. A logged-in drive is answered from `drvtbl[d]`; no
   sector is read.
2. **Re-login after a media change**, in `osif` (below), `bdos/bdosmain.c:471`.

There is no other `Getbpb` in `bdos/`.

**When the BDOS learns of a media change [FS20].** Three places, each of
which ends in `longjmp(errbuf, 1)` with `errcode = rwerr = E_CHNG` (-14) and
`errdrv` = the drive:

1. **`ckdrv` on a removable drive** (`bdos/fsdrive.c:106-114`): with
   `checkrem` TRUE and the drive in `drvrem`, `Mediach(d) != 0` (1 or 2, or
   a negative driver error) → `E_CHNG`. A fixed drive (`drvrem` bit clear) is
   never asked here.
2. **A buffer hit in `getbcb`** (`bdos/fsbuf.c:179-191`): before a cached
   record is used, `Mediach(b_bufdrv)`: `1` (may have changed) → the record
   is read again as on a miss (`goto doio`); `2` → `E_CHNG`; `0` or a
   negative value → the buffer is used.
3. **Every device access of the BDOS is a logical `Rwabs` without
   `RW_NOMEDIACH`** — `longjmp_rwabs` (`bdos/fs.h:60-77`, flag 0 or 1): the
   read of a `getbcb` miss (`bdos/fsbuf.c:165`), the write-back of a dirty
   buffer (`flush`, `bdos/fsbuf.c:95,100`; called for an evicted buffer,
   `:162-163`, by `usrio` and by `ixclose`'s loop) and the whole-record file
   transfers (`usrio`, `bdos/fsio.c:73`). A non-zero `Rwabs` return of any
   kind is stored in `rwerr`/`errcode` with `errdrv` = the drive and jumps
   (`bdos/fs.h:64-68,73-77`). For a drive whose logical sector 0 was written
   (`forcechange`), `Rwabs` fails every logical request with `E_CHNG`
   (`docs/api/bios/04-rwabs.md`, order table, steps 5 and 10). `flush` marks
   the buffer invalid **before** its write (`bdos/fsbuf.c:93`), `getbcb` the
   victim before its read (`:164`), so a failed access leaves no stale buffer.

So a call that touches no buffer and no sector of the drive — `Dgetpath`,
`Dsetdrv`, `Fsetdta`, `Dgetdrv` — does **not** notice a pending change on a
fixed drive; the next call that reads or writes one of its records does.

**What the re-login does — `osif`'s media-change block** (`bdos/bdosmain.c:
449-487`; the `setjmp` is re-armed at the top of every call, `:442-449`).
On `errcode == E_CHNG`, in this order:

1. **Open files** — `offree(drvtbl[errdrv])` (`bdos/bdosmain.c:387-405`):
   every `sft[]` entry whose OFD belongs to the drive's DMD is freed (the OFD
   released, `f_ofd = NULL`, `f_own = NULL`, `f_use = 0`). The handle number
   stays in the program's hands but names a free entry: `Fread`/`Fwrite`/
   `Fseek` on it answer `EIHNDL` ("invalid handle: media change, etc",
   `bdos/bdosmain.c:591-592`), `Fclose` answers `EIHNDL` (`getofd` NULL,
   `bdos/fsopnclo.c:481-482`). Data the program wrote that was still in a
   buffer is lost with step 4. Standard handles forced to such an entry
   (`p_uft[h] > 0`) are not reset; they name the free entry.
2. **The DMD** is released and `drvtbl[errdrv] = NULL` (`:459-463`).
3. **Current directories** — `freetree(dmd->m_dtl)` (`bdos/bdosmain.c:
   356-378`): the drive's DND tree is released and every `dirtbl[]` slot that
   named one of its DNDs is emptied (`dnd = NULL`, `use = 0`). The PDs'
   `p_curdir[d]` bytes are not touched; the next `ckdrv` on the drive finds an
   empty slot and gives the running PD the root (`bdos/fsdrive.c:120-131`).
   Observable: after a media change every process's current directory on that
   drive is the root (`process/_run-ownership-curdir.md` §4.5).
4. **Buffers** — `mark_bcbs_invalid(errdrv)` (`bdos/bdosmain.c:411-424`):
   every buffer of the drive, in both lists (FAT and data), is marked invalid
   (`b_bufdrv = -1`). Dirty buffers are **discarded, not written back** (§3).
5. **`Getbpb(errdrv)`** (`:471`), which also clears the drive's
   `forcechange` and `mediachange` (`docs/api/bios/07-getbpb.md`,
   `bios/blkdev.c:589-590`). NULL → the drive's `drvsel` bit is cleared and
   the call returns **`E_CHNG`** (-14) to the program (`:472-476`); the next
   call on the drive logs it in afresh through `ckdrv`.
6. **`log_media(b, errdrv)`** with that BPB (`:478-482`) — the geometry is the
   **new** boot sector's. Failure → `drvsel` bit cleared, **`ENSMEM`**.
7. `rwerr = errdrv = 0` and **`goto restrt`** (`:484-486`): the whole call is
   run again from its function number, with its original arguments, under a
   fresh `setjmp`. Whatever the first run did before the jump stays done
   (except the buffers of the drive, step 4); its return value is dropped.

Any other hard error (`errcode != E_CHNG`) only marks the drive's buffers
invalid and returns the code (`:489-492`); see §3 and the HW03 deviation in
`tos-api/src/bdos/fs/cache.rs:340@write_back` (NewTOS keeps a dirty buffer
whose write-back failed).

**What a media change does not touch.** The DTA of an `Fsfirst`/`Fsnext`
search holds drive, cluster and offset, no DND pointer (`xsnext`,
`bdos/fsdir.c:556-580`): `Fsnext` after a re-login continues on the new
geometry. `p_curdrv` is not changed.

**Measured (gcprobe, oracle).** C: a fixed IDE drive. After a logical
sector-0 write with `RW_NOMEDIACH` (arms `forcechange`), `Mediach(2)` = 2;
the next `Dfree(C:)` (a FAT-buffer access) re-logs C: in, and `Mediach(2)` is
0 after it (A05 → A07). After the boot sector's root-entry count is changed
512 → 256 and written back the same way, `Dfree(C:)` returns the **new**
cluster total 32676 (B04), `Mediach` 0 (B05), and a file created next has its
cluster 3 in partition sector 277, the new geometry's (`datrec` 273).

**NewTOS [FS20].**

- The SD card is non-removable (`docs/api/bios/09-mediach.md`, Pi binding):
  `drvrem` is empty, so place 1 does not exist; `Volume::mount`
  (`tos-api/src/bdos/fs/fat.rs:132@mount`) answers a logged-in drive without
  a device access, as `ckdrv` does for a fixed drive.
- The BDOS reaches the card through the block cache (`bios::bcache`), not
  through `Rwabs`, so the BDOS sector cache asks the BIOS media state itself
  at places 2 and 3 (`tos-api/src/bdos/fs/cache.rs:255@hit_test`,
  `tos-api/src/bdos/fs/cache.rs:270@rwabs_test`): on a hit `Mediach`'s answer
  (`bios::media_change`), before every device access the media tests of a
  logical `Rwabs` without `RW_NOMEDIACH` (`forcechange` armed or
  `Mediach` > 0 → `E_CHNG`). Each buffer remembers its drive (`b_bufdrv`).
- There is no `longjmp`: the access fails, the cache records `E_CHNG` and the
  first `errdrv` of the call (`tos-api/src/bdos/fs/cache.rs:227@signal_media_change`),
  and the call returns through its error paths. While the change is pending
  every further access to that drive fails the same way, so nothing reaches
  the disk after the detection. `gemdos::dispatch` — `osif` — then runs steps
  1-7 (`tos-api/src/gemdos/mod.rs:210@media_change_relog`) and runs the call
  again (`tos-api/src/gemdos/mod.rs:150@dispatch_inner`): `offree` =
  `tos-api/src/bdos/fs/file.rs:1326@free_drive_files`, `freetree`'s
  `dirtbl` half = `tos-api/src/bdos/fs/curdir.rs:332@free_drive` (NewTOS keeps
  no DND tree), the DMD = `tos-api/src/bdos/fs/fat.rs:108@log_out`,
  `mark_bcbs_invalid` = `tos-api/src/bdos/fs/cache.rs:284@invalidate_drive`,
  `Getbpb` + `log_media` = `Volume::mount` (`bpb::geometry` clears
  `forcechange`, `docs/api/bios/07-getbpb.md`). A `Getbpb` refusal returns
  `E_CHNG`; NewTOS has no `log_media` failure of its own (`bpb::parse` already
  refuses what `log_media` would, `fsiz == 0`), so `ENSMEM` does not occur.
- Difference in kind, not in the measured result: EmuTOS abandons the call at
  the first failed access; NewTOS lets it run on through its error paths
  before the restart. Device state cannot differ (every later access to the
  drive fails until the re-login); in-memory effects of those error paths
  are overwritten by the restarted run or freed by steps 1-4.
- **Open [FS20]:** BDOS work that does not enter through `gemdos::dispatch`
  — `Pexec` (the kernel's frame-mutating path) and the kernel's direct file
  readers (`bdos::fs::read_file`: the AES resource loader, `shel_find`, the
  file selector's resource, `NEWDESK.INF`, the keyboard table) — meets the
  pending change and fails, without a re-login; the next GEMDOS call that
  touches the drive re-logs it. In EmuTOS all of these are GEMDOS traps and
  restart. Not decided (report of TASK/FS20).

---

## 5. Current Path per Drive

The current directory is **per-process, per-drive**, stored in
`PD.p_curdir[NUMCURDIR]` (`UBYTE[26]`, basepage offset **0x40**,
`include/bdosdefs.h:89`; `NUMCURDIR = BLKDEVNUM = 26`, `include/sysconf.h:62`).
Each entry is an **index into the global `dirtbl[]`**, not a path string. Index
`0` means "not set yet" (treated as the drive root). (This is the
current-directory state; the DTA pointer is `p_xdta`, §6 — the two are
unrelated. An earlier draft mislabelled the path storage as `p_dta` and used 16
slots; it is 26 — see D-025.)

### `dirtbl[]` — reference-counted DND cache (`bdos/fsdrive.c:54`)

```c
DIRTBL_ENTRY dirtbl[NCURDIR];   /* bdos/fs.h:342-345, NCURDIR = 40 (bdos/fs.h:44) */
/* DIRTBL_ENTRY { DND *dnd; WORD use; } */
```

`dirtbl[]` maps a small integer (the value stored in `p_curdir[]`) to a cached
directory `DND` plus a use count. Slot 0 is reserved. `Dsetpath` (via
`incr_curdir_usage`, `bdos/fsdir.c:1164`) finds an existing slot for the target
DND or allocates a free one and bumps its use count; the old slot is released by
`decr_curdir_usage` (`bdos/fsdir.c:1194`). Multiple processes/drives referencing
the same directory share one slot. When `dirtbl[]` is full,
`incr_curdir_usage` returns `EINTRN`, which `Dsetpath` surfaces to the caller as
`EPTHNF`.

`Dsetpath` (0x3B) selects the drive from an optional `X:` prefix
(`extract_drive_number`, 0..25) or the current drive, then updates
`p_curdir[dlog]`; it does **not** change `p_curdrv`. `Dgetpath` (0x47) reads
`dirtbl[p_curdir[drv]].dnd` and walks the DND parent chain (`dopath`) to build
the path string, *without* drive letter and *with* a leading backslash (root =
empty string). `NCURDIR` (40) is distinct from the drive count (26); NewTOS may
enlarge `dirtbl[]` as a free implementation choice.

---

## 6. DTA Pointer (`PD.p_xdta`)

The Disk Transfer Area is a 44-byte buffer (`DTA`, `include/bdosdefs.h:47-55`)
where `Fsfirst`/`Fsnext` write directory entries. Each PD holds the active DTA
pointer in **`p_xdta`** (`DTA *`, basepage offset **0x20**,
`include/bdosdefs.h:79`). `Fsetdta` (0x1A) sets it (`run->p_xdta = addr`,
`bdos/fsmain.c:405`, no validation); `Fgetdta` (0x2F) returns it
(`bdos/fsmain.c:396`).

**Default:** on process creation `Pexec` sets `p_xdta = (DTA *)p->p_cmdlin`
(`bdos/proc.c:375`) — the basepage command-line buffer at offset **0x80**.
A program that calls `Fsfirst` before any `Fsetdta` therefore overwrites its
own command line (the classic Atari quirk; contract for m68k binaries).

**Layout of the 44-byte DTA** (`bdos/fs.h:319-337` `DTAINFO` private view /
`include/bdosdefs.h:47-55` public view):

| Offset | Public field | Private (`DTAINFO`) | Meaning |
|-------:|--------------|---------------------|---------|
| 0x00-0x0B | `d_reserved[0..11]` | `dt_name[12]` | search pattern (11 FCB chars + attr) |
| 0x0C-0x0F | `d_reserved[12..15]` | `dt_offset_drive` (LONG) | `-1` = uninitialised; else drive id (low 5 bits) + root resume offset |
| 0x10-0x11 | `d_reserved[16..17]` | `dt_cloffset` (UWORD) | sub-dir resume offset within cluster |
| 0x12-0x13 | `d_reserved[18..19]` | `dt_clnum` (UWORD) | sub-dir current cluster |
| 0x14   | `d_reserved[20]` | `dt_attr` (char) | search attribute mask |
| 0x15   | `d_attrib`   | `dt_fattr`  | attribute byte of the found entry |
| 0x16   | `d_time`     | `dt_td.time`| packed time — **native (m68k big-endian)** byte order |
| 0x18   | `d_date`     | `dt_td.date`| packed date — native byte order |
| 0x1A   | `d_length`   | `dt_fileln` | file size, LONG, native byte order |
| 0x1E   | `d_fname[14]`| `dt_fname[14]` | dotted 8.3 name, upper-case, NUL-terminated |

Bytes 0x00-0x14 (21 bytes) are the **private** search state programs must
treat as opaque between the matching `Fsfirst` and the final `Fsnext`. The
time/date/size are byte-swapped from the little-endian on-disk FCB to native
order by `makbuf` (`bdos/fsdir.c:1851-1862`) — NewTOS must reproduce the m68k
big-endian values for the DTA's public fields (contract), while its private
encoding is a free choice as long as `Fsnext` resumes correctly and returns
`ENMFIL` (-49) when exhausted/uninitialised.

---

## 7. `conterm` (system variable, `include/tosvars.h:38`)

```c
extern UBYTE conterm;   /* low byte of OS var _conterm @ 0x484 */
```

`conterm` is a single **UBYTE** system variable at low-memory address `0x484`
(`tosvars.ld:70`, labelled `_conterm`; the linker comment "attribute vector for
console output" is historical and misleading). It is initialised to **`7`** by
`kbd_init` (`bios/ikbd.c:1100`, "keyclick, key repeat & system bell on by
default") — bits 0,1,2 set, bit 3 clear.

**Bit fields — as actually tested in this EmuTOS build (`1a7484ab`):**

| Bit | Mask | Meaning | Read at |
|----:|------|---------|---------|
| 0   | `& 1` | keyboard **key click** sound on keypress | `bios/ikbd.c:495,801,845,894` |
| 1   | `& 2` | keyboard **key repeat** enabled | `bios/ikbd.c:475` |
| 2   | `& 4` | **system bell** on Ctrl-G / VT52 bell | `bios/vt52.c:215` (`do_bell`) |
| 3   | `& 8` | return the **shift-state byte** in the `Bconin` high word (bits 24-31); if clear, `Bconin` masks it out (`value &= 0x00ffffff`) | `bios/ikbd.c:279-280` |
| 4-7 | —    | unused in this build | — |

> **Correction (D-027).** An earlier draft of this section claimed bit 0 =
> "`Cconin` echoes input", bit 1 = "Ctrl-S/Ctrl-Q flow control", bit 2 =
> "Ctrl-C abort", and that `conterm` is "read by `xconin`, `xnecin`, `xconrs`".
> **All of that is wrong for this EmuTOS build.** `conterm` is **not read
> anywhere in `bdos/console.c`** — a full-tree grep finds it only in
> `bios/ikbd.c` and `bios/vt52.c` (the keyboard/screen BIOS layer). In the
> GEMDOS console layer, echo (`Cconin`) and Ctrl-C/Ctrl-S/Ctrl-Q handling
> (`conbrk`) are **unconditional** — no `conterm` bit gates them. The bit table
> above is the classic Atari layout (key click / repeat / bell / shift-status)
> and matches the code.

**Contract vs. reference:** the *bit meanings and the value* are contract (a
program that pokes `0x484` to toggle key click / repeat / bell / shift-status
observes those effects), but the *effects live in the BIOS keyboard/screen
layer*, not in the GEMDOS console calls. NewTOS keeps the four documented bits
with the same semantics.

**Read by:** `bios/ikbd.c` (key click, key repeat, shift-state packing),
`bios/vt52.c` (`do_bell`). **Written by:** `kbd_init` (init to 7), AES/CPX/setup
code and test programs that adjust keyboard behaviour. **Not read by any GEMDOS
console function.**

---

## 8. Memory Pools `pmd` / `pmdalt` and the `Mxalloc` modes (`bdos/umem.c:35-45`)

```c
MPB pmd;                 /* bdos/umem.c:35 — main ST-RAM pool (always present) */
#if CONF_WITH_ALT_RAM
MPB pmdalt;              /* bdos/umem.c:37 — alt-/TT-RAM pool */
int has_alt_ram;         /* bdos/umem.c:38 — TRUE once Maddalt has run */
#endif
ULONG malloc_align_stram;/* bdos/umem.c:40 — 1 (2-byte) or 3 (4-byte, Falcon) */

static UBYTE *start_stram, *end_stram;  /* bdos/umem.c:44-45 — ST-RAM bounds */
```

Each `MPB` (`include/bdosdefs.h:120-125`) has `mp_mfl` (free-list head),
`mp_mal` (allocated-list head) and the vestigial `mp_rover` ("no longer used").
`Malloc`/`Mfree`/`Mxalloc`/`Mshrink` operate on these lists via the `ffit` /
`freeit` / `shrinkit` allocator (`bdos/iumem.c`); `Maddalt` adds a free region
to `pmdalt`. `MD` records themselves come from a separate OS-memory pool
(`bdos/osmem.c`), never from the heap they describe. See
`docs/api/structures/md.md` for the full model.

**Pool bounds / routing.** `umem_init` (`bdos/umem.c:549`) calls `Getmpb` to
fill `pmd`, then records `start_stram = pmd.mp_mfl->m_start` and `end_stram =
start_stram + pmd.mp_mfl->m_length`. `find_mpb(addr)` (`bdos/umem.c:99`) routes
a free/shrink: `addr ∈ [start_stram, end_stram)` → `pmd`; otherwise, if
`has_alt_ram`, `pmdalt` (a trust-the-caller shortcut — real membership is the
subsequent `mp_mal` scan); else NULL → `EIMBA`.

**`Mxalloc` mode constants (`include/bdosdefs.h:18-22`) — CONTRACT:**

| Value | Constant       | Pool selection |
|------:|----------------|----------------|
| 0 | `MX_STRAM`     | `pmd` (ST-RAM) only |
| 1 | `MX_TTRAM`     | `pmdalt` (alt-/TT-RAM) only; NULL if no alt-RAM |
| 2 | `MX_PREFSTRAM` | `pmd` preferred, `pmdalt` fallback |
| 3 | `MX_PREFTTRAM` | `pmdalt` preferred, `pmd` fallback |

`MX_MODEMASK = 0x03` (`:22`) — only bits 0-1 are honoured; higher bits (Atari's
supervisor/protection flags) are masked off (`bdos/umem.c:241`).

**Ownership caveat (contract-relevant behaviour):** `ffit` tags an allocated
MD with `m_own = run`; `freeit` does **not** clear `m_own` on free. So `m_own`
is meaningful only for MDs on `mp_mal`; it is **not** a free flag. `Pterm` /
`Ptermres` read `m_own` (walking `mp_mal`) to reclaim/reserve a process's
blocks. `Mfree`/`Mshrink` themselves do **no** ownership check — any process
can free any block by address.

NewTOS: implement two pools (ST-RAM + alt/TT-RAM) mapped onto the AArch64
device-tree memory layout (Phase 03/04). The ST/alt split is a TOS-API contract
only — AArch64 has no hardware DMA-zone distinction. NewTOS may cleanly clear
`m_own` on free (it does not depend on the stale-`m_own` quirk).

---

## 9. OS Header `os_header` (`bdos/fsmain.c`, `bios/startup.S:77-113`)

The TOS OS header at low memory address `0x4F2` (`_sysbase`)
contains `os_version`, `os_beg`, `os_end`, `os_magic`, `os_date`,
`os_palmode`, `os_dosdate`, plus pointers to drive table, run-time
flags, etc.

Read by the AES/CPX init code. **`os_dosdate`** is the OS-header build date
used as `DEFAULT_DATETIME = MAKE_ULONG(os_header.os_dosdate, 0)`
(`bios/ikbd.h:95`), the fall-back returned by `Gettime()` when no RTC is
present (`bios/clock.c:1319`) — this is the boot-time seed for `current_date`
(§12).

**`Sversion` (0x30) does *not* read this header.** It returns the compile-time
`GEMDOS_VERSION` constant (§12), which is the *GEMDOS* version and is
independent of the OS-header `os_version` (they can differ). Do not conflate
the two. See `docs/api/gemdos/version/30-sversion.md`.

---

## 10. Concurrency Model

EmuTOS is single-threaded (cooperative): only one BDOS call active
at a time. The state above is therefore not protected by locks.

NewTOS: must serialise BDOS entry per-CPU using a per-process or
global BDOS mutex (decision: ADR-pending, before PHASE-05). Disk
buffer cache and SFT need explicit locks because the kernel is
SMP-aware.

---

## 11. `Dfree` info buffer (4 LONGs / 16 bytes)

`Dfree` (0x36, `xgetfree`, `bdos/fsfat.c:337`) fills a caller-supplied 16-byte
buffer with four LONGs, copied from the drive's `DMD` (§4) plus a computed free
count (`bdos/fsfat.c:361-364`):

| Index | Field           | DMD source | Type | Meaning |
|------:|-----------------|------------|------|---------|
| `buf[0]` | free clusters   | computed (`countfree16` / `getrealcl` loop) | LONG | unallocated data clusters |
| `buf[1]` | total clusters  | `m_numcl` (`CLNO`) | LONG | total data clusters on the volume |
| `buf[2]` | bytes/sector    | `m_recsiz` (`UWORD`) | LONG | bytes per logical record (sector) |
| `buf[3]` | sectors/cluster | `m_clsiz` (`WORD`) | LONG | logical records per cluster |

Each field is written as a full 32-bit LONG (native byte order — big-endian for
m68k callers). Derived: bytes/cluster = `buf[2]*buf[3]`; free bytes =
`buf[0]*buf[2]*buf[3]`; total bytes = `buf[1]*buf[2]*buf[3]`. The buffer is
**contract** (fixed layout a program reads); the DMD internals it derives from
are **free** to reimplement. See `docs/api/gemdos/directory/36-dfree.md`.

---

## 12. GEMDOS date/time globals and the version constant (`bdos/time.c`)

```c
UWORD current_time, current_date;   /* bdos/time.c:53 */
```

Two file-local `UWORD`s hold the cached system time/date in **FAT/DOS packed**
form (epoch 1980). They are the shared state behind `Tgetdate`/`Tgettime`
(pure reads) and `Tsetdate`/`Tsettime` (validate then commit via XBIOS
`Settime`). Read/written by:

- **`time_init`** (`bdos/time.c:160`) — at boot, `current_date = HIWORD(
  Gettime())`, `current_time = LOWORD(Gettime())`; also installs `etv_timer =
  tikfrk`. `Gettime()` is **XBIOS 23 (0x17)** → `gettime()` (`bios/clock.c:
  1266`), which reads whatever RTC is present or falls back to
  `DEFAULT_DATETIME` (§9).
- **`tikfrk`** (`bdos/time.c:175`) — the 200 Hz timer handler (via `etv_timer`);
  every 2000 ms it bumps `current_time` and cascades minute/hour/day/month/year
  rollovers into both words. This is the only place they advance on their own.
- **`settime()`** (`bios/clock.c:1218`, reached by XBIOS 22 `Settime` from
  `Tsetdate`/`Tsettime`) — writes both words directly from the packed LONG,
  then persists to the RTC hardware.

**Packed layouts (contract — a program reads these bits):**

*Date WORD:* bits 0-4 day (1-31), bits 5-8 month (1-12), bits 9-15 year-offset
-from-1980 (0-127). *Time WORD:* bits 0-4 double-seconds (`sec/2`, 0-29), bits
5-10 minute (0-59), bits 11-15 hour (0-23). Odd seconds are not representable.
The same bit layout appears in the DTA public `d_date`/`d_time` (§6) and in FAT
directory entries. **Free to reimplement:** the internal storage of the clock,
the tick mechanism, the RTC driver. **Contract:** the packed values the four
`T*` calls read/accept.

**Version constant:** `GEMDOS_VERSION` (`include/config.h:1590`, default
`0x2000`) is a compile-time constant returned verbatim by `Sversion` (0x30,
`xgetver`, `bdos/bdosmain.c:298`). Byte order is **minor in the high byte,
major in the low byte** (`include/config.h:1577-1578`), so `0x2000` = GEMDOS 0.32. It
is **contract** (programs branch on it) but **not** an OS-header field (§9) and
**not** a reliable feature bitmap. NewTOS reports `0x2000`. See
`docs/api/gemdos/version/30-sversion.md`.

**Ssystem-reachable system values (FreeMiNT extension).** `Ssystem` (0x154) is
not an EmuTOS function; in NewTOS its in-scope modes reach the **cookie jar**
(kernel-owned per ADR-012), *not* the low-memory sysvars FreeMiNT's `S_*VAL`
modes touch. The jar's `{tag,value}` array + `{0,capacity}` terminator is the
observable state; see `docs/api/bios/_cookie-jar.md` and
`docs/api/gemdos/154-ssystem.md`. The deferred `S_GETLVAL`/`S_SETLVAL`-style
peek/poke of `$420..$ffff` is m68k-low-memory-only and out of scope for native
AArch64.

NewTOS: `current_date`/`current_time` become a kernel clock (AArch64 generic
timer + RTC abstraction); an atomic `(date<<16)|time` snapshot accessor avoids
a torn read across a midnight tick under preemption. The four `T*` traps wrap
it; the version constant and cookie jar are kernel globals behind their traps.

---

## Cross-references

- BIOS state — `docs/api/bios/_state.md`.
- XBIOS state — `docs/api/xbios/_state.md`.
- Structures (PD, OFD, DND, DMD, MD/MPB, DTA) — `docs/api/structures/`.
- Dependency graph — `docs/api/_dependency-graph.md`.

# TOS Discrepancy Log

> Phase 00 deliverable D11. Every behaviour difference found between
> EmuTOS source (the primary source) and Compendium / Profibuch
> documentation. Append-only.
>
> Format per entry: `D-NNN`, three-digit sequential.
>
> **NewTOS rule:** when EmuTOS and documentation disagree, NewTOS
> follows EmuTOS unless an ADR overrides this for a specific function.

---

## D-001: `Cconout` (GEMDOS 0x02) return value

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/gemdos/console/02-cconout.md`

**EmuTOS:** `bdos/console.c:xconout()` returns `1` (LONG)
unconditionally after calling `tabout()`.

**Compendium:** Variously documented as returning the function code (2)
or "undefined". Not consistent across editions.

**Profibuch:** "kein Rückgabewert" / value is undefined.

**Decision:** NewTOS follows EmuTOS — `Cconout` returns `1`.

---

## D-002: `Crawio` (GEMDOS 0x06) blocking semantics

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/gemdos/console/06-crawio.md`

**EmuTOS:** `bdos/console.c:xrawio()` with `parm == 0xFF` is
**non-blocking**: it calls `constat()` first; if no character is
available it returns `0L` immediately. This contradicts the most common
documentation, which describes `Crawio` as blocking when reading.

**Compendium:** Says input mode "returns the next character entered, or
0 if no character is available" — actually agrees with EmuTOS, but is
often misread as blocking.

**Profibuch:** Describes as non-blocking (consistent).

**Decision:** NewTOS follows EmuTOS — non-blocking. Distinct from
`Crawcin` (0x07), which **is** blocking.

---

## D-003: `Cauxin`/`Cauxout` typeahead behaviour vs. printer

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/gemdos/console/03-cauxin.md`,
`04-cauxout.md`, `05-cprnout.md`.

**EmuTOS:** Maintains a typeahead buffer for AUX (index 1, see
`bdos/console.c:46`) but **none** for PRN. `Cauxin` therefore goes
through `Bconin(1)` directly (no buffer integration) — only `Cconin`
(0x01) and `Cnecin` (0x08) consume the typeahead buffer via
`getch()`/`conbrk()`.

**Compendium / Profibuch:** Generally don't mention the typeahead
buffer. Treat `Cauxin` as a plain `Bconin(AUX)` — consistent with
EmuTOS.

**Decision:** NewTOS follows EmuTOS. AUX typeahead structure exists in
`bdos/console.c` but is allocated only and exercised by no current
GEMDOS function. The buffer is reserved for future use; we will keep
it inert in NewTOS.

---

## D-004: Standard-handle redirection (`p_uft[]`)

**Date:** 2026-05-02
**Affected:** all 0x01–0x0B + 0x10–0x13 console functions.

**EmuTOS:** All console functions consult the per-process
`run->p_uft[]` table (`include/bdosdefs.h:84`, `NUMSTD = 6`) **before**
falling through to the BIOS device. If `p_uft[stdh]` has been
redirected to a file handle (i.e., `> 0`), the BDOS dispatcher
(`osif()` in `bdos/bdosmain.c:498`) intercepts the call and routes I/O
through the filesystem (`xread`/`xwrite`) instead of calling the
xc/xa/... wrappers. The wrappers themselves only run for the
non-redirected (handle ≤ 0) case.

**Compendium / Profibuch:** Describe standard-handle redirection as a
property of `Fforce` (0x46) / GEMDOS handle 0–5. The interception by
the GEMDOS dispatcher is implementation-detail level and not visible
to applications, so no real discrepancy — just a note that the
"console function" specs need to mention this routing.

**Decision:** NewTOS replicates the dispatcher-level redirection
exactly (see `docs/api/gemdos/_dispatcher.md`).

---

## D-005: `Pterm`/`Pterm0` exit-code width

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/gemdos/process/4c-pterm.md`

**EmuTOS:** `xterm()` declares `rc` as `UWORD` and stores it directly
into the parent's `p_dreg[0]` (a `LONG` field) via
`run->p_dreg[0] = rc`. The store zero-extends to 32 bits.

**Compendium / Profibuch:** "Return code is a 16-bit value passed in
the low word of D0; the high word is undefined."

**Behavioural difference:** Code that interprets the parent's `Pexec`
return value as a full `LONG` sees a guaranteed-zero high word under
EmuTOS (and NewTOS), whereas under classic Atari TOS the high word
may contain garbage. Programs that mask with `& 0xFFFF` are
unaffected; programs that don't mask see deterministic behaviour
under EmuTOS — strictly an improvement, but technically a divergence
from "high word undefined".

**Decision:** NewTOS follows EmuTOS — zero-extend the exit code into
the LONG return.

---

## D-006: `Ptermres` ignores `xsetblk` return

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/gemdos/process/31-ptermres.md`

**EmuTOS:** `xtermres()` calls `xsetblk(0, run, blkln)` and
**discards the return value**, then proceeds with `reserve_blocks` +
`xterm` regardless. If `blkln` is invalid (< 256 or > original block
size), the basepage's MD is not actually shrunk; `reserve_blocks`
then orphans the *original* (full) MD, wasting any TPA the program
intended to release.

**Compendium / Profibuch:** Describe `Ptermres` as "shrink TPA to
`blkln` then keep resident", implying that a bad `blkln` returns a
GEMDOS error to the parent.

**Behavioural difference:** EmuTOS silently ignores invalid `blkln`
(no error code, function still does not return). Classic TOS
documentation suggests an error path that does not exist in code.

**Decision:** NewTOS follows EmuTOS — silently ignore. Document the
behaviour but do not introduce an artificial error return.

---

## D-007: `Tsetdate` accepts day 0 / month 0

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/gemdos/time/2b-tsetdate.md`

**EmuTOS:** `xsetdate()` validates only the upper bounds:
`year > 119`, `curmo > 12`, `day > nday[curmo]`. With `curmo == 0`
the lookup table entry `nday[0] == 0`, so `day == 0` passes the
test (`0 > 0` is false). Likewise `(curmo == 1, day == 0)` is
accepted. The function then tells the BIOS to commit the invalid
date.

**Compendium / Profibuch:** "Sets the system date. Returns ERR if
the date is invalid." Day 0 and month 0 are not valid Gregorian
dates.

**Behavioural difference:** EmuTOS silently accepts (year,0,0) and
(year,m,0) where it should reject. The accepted invalid date then
propagates into FAT directory entries via `Fcreate`, producing
on-disk timestamps that mainstream FAT tools may flag.

**Decision:** NewTOS adds explicit `day >= 1 && curmo >= 1` checks.
This is a strict superset of the documented Compendium behaviour;
no program that passes valid dates is affected.

---

## D-008: XBIOS unimplemented-function return value

**Date:** 2026-05-02
**Phase 00 spec:** `docs/api/xbios/_triage.md`

**EmuTOS:** `xbios_do_unimpl(WORD number) { return number; }` —
`bios/xbios.c`. Calling an unimplemented XBIOS function returns
the (positive) function number, not an error code.

**Compendium / Profibuch:** documented as returning `EINVFN` (-32).

**Atari TOS hardware:** matches EmuTOS — returns the function
number. Compendium documentation is incorrect.

**Impact:** Programs that test `if (Xbios(N, ...) < 0)` to detect
"unimplemented" will think the call succeeded. Defensive code uses
side-effect probing (write a known sentinel, see if it changed)
rather than return-value testing for this exact reason.

**Decision:** NewTOS follows EmuTOS — `xbios_unimpl` returns the
function number. Programs assuming the Compendium value would
already be broken on real Atari hardware.

---

## D-009: `Cconws` (GEMDOS 0x09) return value semantics

**Date:** 2026-05-07
**Phase 00 spec:** `docs/api/gemdos/console/09-cconws.md`

**EmuTOS:** `bdos/console.c:454` `xconws(char *p)` is declared `void`.
The dispatcher returns whatever the C compiler left in `D0` after
the void function — undefined ABI scratch. Spec section explicitly
states "the TOS API contract is that Cconws has an undefined return
value."

**Compendium:** documents the return as "the number of characters
output."

**Profibuch:** agrees with Compendium.

**NewTOS implementation:** returns the byte count actually written
(matching Compendium / Profibuch documentation, not the
EmuTOS-source undefined behaviour). Rationale: (a) the count is
genuinely useful to callers and matches what most TOS programs
expect from Compendium documentation; (b) the Phase-02 self-test
asserts the count, which is a cheap end-to-end-check of the trap
path; (c) replacing it with "undefined scratch" would force every
test to find another verification angle.

**Decision:** NewTOS deliberately diverges from EmuTOS source for
this one function. Programs that check the return value see the
count, just as Compendium claims they should.

---

## D-010: `Maddalt` (GEMDOS 0x14) ST-RAM overlap predicate

**Date:** 2026-05-07
**Phase 00 spec:** `docs/api/gemdos/memory/14-maddalt.md`

**EmuTOS:** `bdos/umem.c:455` `xmaddalt` rejects iff
`start < end_stram`, i.e. anything starting below the end of
ST-RAM is refused (`if (start < end_stram) return -1`). This
includes ranges entirely below ST-RAM start.

**NewTOS implementation:** uses the half-open intersection
predicate `[start, end) ∩ [stram_start, stram_end) ≠ ∅` — only
ranges that *actually overlap* ST-RAM get rejected. Ranges
entirely below ST-RAM are accepted.

**Behavioural difference:** on EmuTOS, calling
`Maddalt(0x100000, 0x10000)` when ST-RAM begins at `0x40414000`
returns `-1`. On NewTOS, the same call accepts the region (it
doesn't overlap ST-RAM).

**Practical impact:** none on supported boards. ST-RAM starts at
the kernel-image end (~0x40414000 on QEMU virt, ~0x80000+kernel
size on RPi4); the address space below is reserved for MMIO and
the kernel image itself, so no caller would legitimately pass
`Maddalt` such a region. The divergence is detectable only by
test code that deliberately targets the specific range.

**Decision:** keep the half-open intersection predicate. Strictly
better semantics (only reject genuinely-overlapping additions),
zero impact on real workloads.

---

## D-011: `Cconout` / `Crawio` write-side return values

**Date:** 2026-05-07
**Phase 00 specs:** `docs/api/gemdos/console/02-cconout.md`,
`06-crawio.md`

**EmuTOS:** `Cconout` returns `1` unconditionally (already
captured as D-001 vs Compendium). `Crawio` write-mode
(`parm != 0xFF`) calls `Bconout(1, parm)` and discards the BIOS
return — D0 holds whatever the dispatcher left, effectively `0`.

**Initial NewTOS implementation (PHASE-02 D3):** drifted from the
spec: `Cconout` returned `E_OK = 0` and `Crawio` write returned
the written byte (`parm as Long`). Caught during the PHASE-05
post-EC4 spec audit (2026-05-07) and corrected. **Cconout** now
returns `1` (matching D-001); **Crawio** write now returns `0`
(matching the `06-crawio.md` "Write: always 0" output table).

**Decision:** spec compliance. The drift never broke the
self-tests because neither is checked, but it would have
surfaced as soon as a Compendium-aware program checked
`Cconout`'s return for "1 means success".

---

## D-012: BIOS disk-vector low-memory addresses (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/bios/04-rwabs.md`, `09-mediach.md`,
`07-getbpb.md`, `_state.md`

**EmuTOS:** `tosvars.ld` fixes the disk-vector sysvars at:
`hdv_bpb = 0x472`, `hdv_rw = 0x476`, `hdv_boot = 0x47a`,
`hdv_mediach = 0x47e`. Installed by `blkdev_init()` (`bios/blkdev.c:101-103`).

**Prior NewTOS docs (wrong):** `04-rwabs.md` claimed `hdv_rw = 0x47E`;
`09-mediach.md` claimed `hdv_mediach = 0x47A`. Both were swapped/wrong —
`0x47A` is `hdv_boot`, `0x47E` is `hdv_mediach`, and `hdv_rw` is `0x476`.

**Not a Compendium discrepancy** — an internal spec error. Corrected in the
refined docs and catalogued once in `_state.md §1`.

**Decision:** use the `tosvars.ld` addresses. (For native AArch64 these live
in a kernel struct; the physical address only matters to the Phase-10 m68k
emulator.)

---

## D-013: `Mediach` invalid-drive and error returns (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/bios/09-mediach.md`

**EmuTOS:** `blkdev_mediach()` (`bios/blkdev.c:783`) returns **`EUNDEV` (-15)**
for an out-of-range / invalid drive (`:789-790`), and **passes through the
negative error code** from `flop_mediach`/`disk_mediach` when the probe fails
(`:810-811`). It does not normalise failures to `MEDIAMAYCHANGE`.

**Prior NewTOS doc (wrong):** said an out-of-range drive "returns 0 (pretends
no change)" and that "any underlying I/O failure is collapsed into 'may have
changed' (1)". Both contradict the source.

**Compendium:** documents only the 3-state 0/1/2 return; does not mention
`EUNDEV` or negative pass-through (so this is also a Compendium omission).

**Decision:** follow EmuTOS — return `EUNDEV` for invalid drive and propagate
negative driver errors. Corrected in `09-mediach.md`.

---

## D-014: `Rwabs` flag bits, `numb==0`, and undocumented features

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/bios/04-rwabs.md`

**EmuTOS:** `r_w` flags (`bios/disk.h:53-61`): bit0 `RW_WRITE`, bit1
`RW_NOMEDIACH`, bit2 `RW_NORETRIES`, bit3 `RW_NOTRANSLATE`, bit7
`RW_NOBYTESWAP`. `blkdev_rwabs()` has **no `EBADRQ` path for `numb==0`** (a
zero count returns `E_OK`). It has two behaviours not in any documentation:
(a) `adr==NULL` on a floppy **sets** `blkdev[dev].mediachange = numb` and
returns 0 (FastCopy-III feature, `bios/blkdev.c:419-428`); (b) a logical
**write to sector 0** arms `forcechange`, so the *next* access returns
`E_CHNG` until `Getbpb` clears it (`:452-453`).

**Prior NewTOS doc (wrong / incomplete):** named bit 3 as "don't retry" and
bit 4 as "don't translate" (off by one; missing `RW_NOBYTESWAP`); claimed
`numb==0` returns `EBADRQ` (no such check exists); omitted both undocumented
features.

**Compendium:** documents the standard flags and the `lfirst` extension; does
**not** document the `NULL`-buffer feature or `forcechange`.

**Decision:** follow EmuTOS — correct flag bits, no `EBADRQ` for zero count,
and reproduce both undocumented behaviours for m68k-binary compatibility
(may be NewTOS-native for source ports). Corrected in `04-rwabs.md`.

---

## D-015: `Getbpb` does not flush the GEMDOS FAT cache

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/bios/07-getbpb.md`

**EmuTOS:** `blkdev_getbpb()` (`bios/blkdev.c:569`) **resets the media-change
state** (`mediachange = MEDIANOCHANGE`, `forcechange = FALSE`, `:589-590`) and
re-reads/re-parses the boot sector on every call. It does **not** flush any
GEMDOS in-memory FAT cache — that is a separate GEMDOS concern.

**Prior NewTOS doc (wrong):** listed as an edge case that `Getbpb` "also
flushes any GEMDOS in-memory FAT cache for that drive when it detects media
change." No such action is in the BIOS-layer source.

**Decision:** follow EmuTOS — `Getbpb` resets the BIOS media-change flag and
rebuilds the BPB; the FAT-cache flush claim is removed. Corrected in
`07-getbpb.md`.

---

## D-016: `bconin0` (PRT input) is a stub, and MIDI/IKBD swap is bcostat-only

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/bios/02-bconin.md`, `08-bcostat.md`,
`_state.md`, `_dispatcher.md`

**EmuTOS:** `bconin0` (`bios/parport.c:123`) is `/* not implemented */` and
returns `0` unconditionally. The MIDI/IKBD slot swap exists **only** in the
`bcostat_vec` table (`bcostat_vec[3]=bcostat4`, `[4]=bcostat3`,
`bios/chardev.c:38-41`); the `bconin`/`bconout`/`bconstat` tables are **not**
swapped (handle 3 = MIDI, handle 4 = IKBD there).

**Prior NewTOS docs (wrong):** `02-bconin.md` claimed `bconin0` "reads the
parport status / data lines"; `_dispatcher.md §5` implied the MIDI/IKBD swap
applied to input as well as output.

**Decision:** follow EmuTOS — `bconin0` is a stub returning 0; the swap is
`bcostat`-only. Corrected in the refined docs.

**Note (not a discrepancy):** all four char-device default handlers are
Atari-hardware / `CONF_*` gated (MFP, ACIA, SCC, printer port). NewTOS supplies
equivalent handlers for its own devices; these EmuTOS internals are *behaviour
reference*, not contract.

---

## D-017: `Fopen` valid-mode mask is `0x07`, not `0x77` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/file/3d-fopen.md`

**EmuTOS:** `xopen` masks the mode with `VALID_FOPEN_BITS = MODE_FAC = 0x07`
(`bdos/fs.h:169,173`; `bdos/fsopnclo.c:225`). Only the low three bits (the
access code `RO_MODE`/`WO_MODE`/`RW_MODE`) survive; the file-sharing bits 4-6
(`MODE_FSM`) and the inheritance bit 7 (`INH_MODE`) are **masked away** — this
EmuTOS build does not implement file sharing / record locking.

**Prior NewTOS doc (wrong):** `3d-fopen.md` claimed the mask was `0x77` and
documented a "sharing mode" in bits 4-6 that `Fopen` accepts. Neither is true
of the source.

**Compendium:** documents the sharing/compatibility modes as part of `Fopen`;
those bits are a real TOS/GEMDOS concept but are **not honoured** by EmuTOS
`1a7484ab` (they never reach the OFD via `Fopen`).

**Decision:** follow EmuTOS — `Fopen` honours only bits 0-2. Corrected in
`3d-fopen.md`. If NewTOS later implements sharing it must widen the mask
deliberately and document it.

---

## D-018: `Fopen`/`Fdelete`/`Fattrib` folder & path error codes (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/file/3d-fopen.md`, `41-fdelete.md`,
`43-fattrib.md`

**EmuTOS:**
- `xopen` returns **`EFILNF`** (-33) when `findit` fails — i.e. even when the
  *directory* part of the path is missing (`bdos/fsopnclo.c:248-249`). It never
  returns `EPTHNF` for `Fopen`.
- `xopen`/`xunlink` scan with `FA_NORM`, which excludes `FA_VOL`/`FA_SUBDIR`, so
  a **folder** of that name is "not found" → `EFILNF`, **not** `EACCDN`
  (`bdos/fsopnclo.c:256,604`).

**Prior NewTOS docs (wrong):** `3d-fopen.md` listed `EPTHNF` as a possible
`Fopen` return; `41-fdelete.md` claimed `Fdelete` on a folder returns `EACCDN`.
Both contradict the source (`Fdelete` on a folder → `EFILNF`; `Fopen` on a bad
path → `EFILNF`).

**Compendium:** documents `EPTHNF`/`EFILNF` generically; the exact collapse to
`EFILNF` is EmuTOS behaviour.

**Decision:** follow EmuTOS. Corrected in the three specs. (`Fattrib` and
`Frename` *do* return `EPTHNF` from their own `findit` failures — that part is
unchanged.)

---

## D-019: `Fsnext`/`Fsetdta` error and default facts (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/file/4f-fsnext.md`,
`docs/api/gemdos/dta/1a-fsetdta.md`, `docs/api/structures/dta.md`

**EmuTOS:** `xsnext` returns **`ENMFIL`** (-49) for *every* exhausted /
uninitialised case — no prior `Fsfirst`, failed search, or already-exhausted DTA
(`bdos/fsdir.c:662-663,669-670`). There is no `EINTRN` path in `Fsnext`. The
per-process DTA pointer field is **`p_xdta`** at basepage offset **0x20**
(`include/bdosdefs.h:79`), defaulting to `p_cmdlin` (offset 0x80,
`bdos/proc.c:375`).

**Prior NewTOS docs (wrong):** `dta.md` claimed "`Fsnext` without a prior
`Fsfirst` returns `EINTRN` (-65) on EmuTOS, `ENMFIL` on Atari TOS" — the source
returns `ENMFIL` in all cases. The old DTA docs also referred to the pointer as
`p_dta`/`p_xdta` inconsistently and to a nonexistent `p_dtabuf`.

**Compendium:** documents `ENMFIL` on exhaustion — consistent with EmuTOS; the
claimed `EINTRN` divergence never existed.

**Decision:** follow EmuTOS — `ENMFIL` for all exhausted/uninitialised `Fsnext`.
Corrected in `4f-fsnext.md`, `1a-fsetdta.md`, and the `dta.md` cross-reference
note.

---

## D-020: SFT/handle sizing — `OPNFILES = 75`, `NUMHANDLES = 81` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/file/45-fdup.md`, `3d-fopen.md`,
`docs/api/gemdos/_state.md`

**EmuTOS:** `OPNFILES = 75`, `NUMSTD = 6`, `NUMHANDLES = NUMSTD + OPNFILES = 81`
(`bdos/fs.h:43-45`, `include/bdosdefs.h:61`). `PD.p_uft[NUMSTD]` is an array of
**signed bytes** holding either a negative BIOS handle or a non-standard handle
number; it is **not** an index into a `syshnd[]` array.

**Prior NewTOS docs (wrong):** `45-fdup.md` computed `OPNFILES = NUMHANDLES -
NUMSTD = 40 - 6 = 34`; `3d-fopen.md` and `_state.md` implied 32/40. All were
wrong. `_state.md` also mis-described `p_uft` as pointing "into syshnd".

**Not a Compendium discrepancy** — an internal spec error (Atari TOS 1.04-3.06
also use 75 system-wide open files; TOS 4.04 uses 19 per-process). Corrected in
the three docs; `docs/api/structures/ofd.md` already had the correct 75.

**Decision:** use `OPNFILES = 75`, `NUMHANDLES = 81`.

---

## D-021: `Fcreate` illegal-character set is narrow and create-specific

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/file/3c-fcreate.md`

**EmuTOS:** `contains_illegal_characters` tests against
`ILLEGAL_FNAME_CHARACTERS = " *,:;<=>?[]|"` (`bdos/fsopnclo.c:64`) — space,
`*`, `,`, `:`, `;`, `<`, `=`, `>`, `?`, `[`, `]`, `|`. This is *mostly* the
MS-DOS set **but** Atari additionally allows `+`, and the set does **not**
include `/`, `\`, or `"`. The same test guards `Frename`'s new name.

**Prior NewTOS doc (wrong):** `3c-fcreate.md` listed the rejected set as
`< > : " / \ | ? *` — it wrongly included `"`, `/`, `\` and omitted space, `,`,
`;`, `=`, `[`, `]`.

**Compendium:** describes "invalid filename characters" loosely; the exact set
is EmuTOS's.

**Decision:** follow EmuTOS — reject exactly `" *,:;<=>?[]|"` on create/rename.
Corrected in `3c-fcreate.md` (and referenced by `56-frename.md`).

---

## D-022: `Fcreate` opens the new file `RO_MODE` when `FA_RO` is set (nuance)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/file/3c-fcreate.md`

**EmuTOS:** `ixcreat` calls `opnfil(..., (fcb->f_attrib & FA_RO) ? RO_MODE :
RW_MODE)` (`bdos/fsopnclo.c:201`) — so a file created with `FA_RO` gets an OFD
whose recorded access code is `RO_MODE`. **However**, EmuTOS does not enforce
the access code on `Fwrite` (`bdos/fsio.c:536-538`), so the returned handle is
still writable; the read-only attribute only blocks a *later* `Fopen` for write.

**Prior NewTOS doc (imprecise):** `3c-fcreate.md` said the handle is "opened
read-write, even if FA_RO is set". The observable result (writable handle) is
correct, but the recorded OFD mode is `RO_MODE`, not `RW_MODE` — the handle is
writable because the mode is *unenforced*, not because it is RW.

**Not a Compendium discrepancy** — a precision fix. Corrected in `3c-fcreate.md`
(edge case 2) and cross-referenced from `3d-fopen.md`/`3f-fread.md`/`40-fwrite.md`.

**Decision:** document the observable contract (writable handle) and the exact
mechanism (mode recorded but not enforced).

---

## D-023: `Frename` details — reserved word, folder `..` update, `EINTRN`, open-file corruption

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/file/56-frename.md`

**EmuTOS:** `xrename(int n, char *p1, char *p2)` (`bdos/fsdir.c:895`):
- The `n` argument is a historical reserved word; the binding hard-codes 0
  (`include/bdosbind.h:53`) and the code ignores it.
- For a **cross-directory folder move** it **does** rewrite the moved folder's
  `..` entry to the new parent's start cluster and fixes the folder's attribute
  byte in the new parent (`bdos/fsdir.c:1043-1064`); failures there return
  **`EINTRN`** (-65).
- It does **not** check for open OFDs on the source; renaming an open file
  leaves the OFD pointing at the erased old entry, which corrupts the directory
  on the later `Fclose` (`bdos/fsdir.c` — no OFD check).

**Prior NewTOS doc (wrong/incomplete):** `56-frename.md` claimed the folder `.`
/`..` entries are **not** updated (the source updates `..`), and omitted the
`EINTRN` error path.

**NewTOS decision (deliberate divergence):** NewTOS will **not** reproduce the
open-file rename corruption — it will check the OFD chain and return `EACCDN`
if the source file has an open OFD. This is a strict safety improvement; a
program relying on the buggy behaviour would already be corrupting directories
on real TOS. Documented in `56-frename.md`; the `..`-update and `EINTRN` facts
are corrected to match the source.

**Superseded 2026-10-19 (TASK/FS18):** the `EACCDN` refusal was never built.
Re-reading `xrename` showed the same-directory rename is *not* corrupting —
it overwrites the name in place and the OFD stays valid (`fsdir.c:1076-1083`);
only the cross-directory move leaves the OFD on the erased slot. NewTOS now
renames in place like EmuTOS and, for a file moved across directories while
open, points its OFDs at the new entry instead of refusing — `Frename`'s
result stays EmuTOS's. Sanctioned deviation in `docs/COMPATIBILITY.md`;
spec `56-frename.md` edge case 9.

---

## D-024: DTA public time/date/size are native (m68k big-endian) in the buffer

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/file/4e-fsfirst.md`, `4f-fsnext.md`,
`docs/api/structures/dta.md`

**EmuTOS:** `makbuf` copies the found FCB's time/date/size into the DTA's public
fields using `swpw`/`swpl` (`bdos/fsdir.c:1851-1862`), converting from the
**little-endian** on-disk FCB to **native (m68k big-endian)** order. So a
program reading `DTA.d_time`/`d_date`/`d_length` sees big-endian words on m68k,
matching the packed FAT *bit* layout but in host byte order.

**Prior NewTOS doc (incomplete):** `dta.md` described the fields as "FAT packed"
without stating the byte order, which matters for the AArch64 ABI.

**Not a Compendium discrepancy** — a precision fix relevant to NewTOS's
big-endian-ABI marshaling. Corrected in the two `Fsfirst`/`Fsnext` specs and
`docs/api/gemdos/_state.md` §6.

**Decision:** the DTA's public time/date/size are native byte order (big-endian
for m68k callers) — contract for the marshaling shim.

---

## D-025: Drive count is 26 (A:..Z:), `LEN_ZPATH` is 114, and there is no global `curdrv` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/drive/0e-dsetdrv.md`, `19-dgetdrv.md`,
`docs/api/gemdos/directory/39-dcreate.md`, `3b-dsetpath.md`, `47-dgetpath.md`,
`docs/api/gemdos/_state.md` §4, §5

**EmuTOS:**
- `BLKDEVNUM = 26` (`include/sysconf.h:43`, comment "A: ... Z:"). Drives are
  **A:..Z:**, and `Drvmap()`/`drvbits` use bits 0..25. `xsetdrv` bounds-checks
  `0 <= drv < 26` (`bdos/fsmain.c:422`).
- `LEN_ZPATH = 114` (`include/sysconf.h:51`), so the `Dcreate` total-path cap
  (`LEN_ZPATH-3`) is **111** (`bdos/fsdir.c:226`) and the `Dgetpath` write
  budget (`LEN_ZPATH-3`) is **111** (`bdos/fsdir.c:1245`); callers still
  allocate the full 114-byte buffer.
- The current drive is **per-process** in `PD.p_curdrv` (`UBYTE`, basepage
  offset 0x37, `include/bdosdefs.h:86`). There is **no** global `WORD curdrv`
  variable anywhere in EmuTOS. `PD.p_curdir[NUMCURDIR]` is `UBYTE[26]`
  (`NUMCURDIR = BLKDEVNUM = 26`), each entry an index into `dirtbl[NCURDIR]`
  with `NCURDIR = 40` (`bdos/fs.h:44`).

**Prior NewTOS docs (wrong):** the drive/directory specs and `_state.md` §4/§5
repeatedly used `BLKDEVNUM = 16` / "A:..P:", `LEN_ZPATH = 128` (deriving a
"124-byte cap"), a `p_curdir[16]` array, and a non-existent global
`WORD curdrv` "read by xgetdrv, written by xsetdrv". All contradict the source.

**Not a Compendium discrepancy** — internal spec errors. (The A:..P: figure and
`curdrv` come from original-TOS lore, not this EmuTOS build.) Corrected in all
five specs and `_state.md` §4/§5.

**Decision:** use `BLKDEVNUM = 26`, `LEN_ZPATH = 114` (cap 111), `NCURDIR = 40`,
and the per-process `PD.p_curdrv`/`PD.p_curdir[26]` model.

---

## D-026: `Dfree` returns generic `ERR` (-1) on bad drive, not `EDRIVE`

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/directory/36-dfree.md`

**EmuTOS:** `xgetfree` (`bdos/fsfat.c:337`) calls `ckdrv(drv, TRUE)`; on any
negative return it does `return ERR` (`bdos/fsfat.c:345-346`) — the generic
`ERR = -1` (`include/gemerror.h:20`). `ckdrv` itself distinguishes `EDRIVE`
(-46), `EPTHNF` (-34), and `ENSMEM` (-39) (`bdos/fsdrive.c:89-102`), but
`xgetfree` **discards** the specific code and always returns `-1`.

**Compendium:** documents `EDRIVE` (-46) as the `Dfree` error return.

**Prior NewTOS doc:** already returned `ERR` (-1) in its output table (correct),
but flagged this only as a tentative "candidate D-010" that was never logged;
this entry records it properly. (`Dgetpath`, by contrast, *does* return `EDRIVE`
— `bdos/fsdir.c:1240` — so the two directory-group calls differ on bad-drive
codes.)

**Decision:** follow EmuTOS — `Dfree` returns `ERR` (-1) on any `ckdrv` failure.
Compendium's `EDRIVE` for `Dfree` is a documentation discrepancy. (Whether real
Atari TOS returns `-1` here is not verifiable from `references/` — no
disassembly is present — so that claim is left out; the EmuTOS behaviour is the
contract per Rule 1.)

---

## D-027: `conterm` bits, and the GEMDOS console never reads `conterm` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/_state.md §7`, and all console specs
`docs/api/gemdos/console/*.md` (the "conterm" cross-references).

**EmuTOS:** `conterm` is a single `UBYTE` system variable at `0x484`
(`include/tosvars.h:38`, `tosvars.ld:70` `_conterm`), initialised to **`7`** by
`kbd_init` (`bios/ikbd.c:1100`). A full-tree grep finds it read in **only two
files**, both in the BIOS layer — never in `bdos/console.c`:
- bit 0 (`& 1`) → keyboard **key click** (`bios/ikbd.c:495,801,845,894`);
- bit 1 (`& 2`) → keyboard **key repeat** (`bios/ikbd.c:475`);
- bit 2 (`& 4`) → **system bell** on Ctrl-G / VT52 (`bios/vt52.c:215`);
- bit 3 (`& 8`) → return the **shift-state byte** in the `Bconin` high word,
  else mask it out (`bios/ikbd.c:279-280`).

In the GEMDOS console layer, echo (`xconin`→`conin`→`conout`) and Ctrl-C /
Ctrl-S / Ctrl-Q / Ctrl-X handling (`conbrk`, `bdos/console.c:206-244`) are
**unconditional** — no `conterm` bit gates any of them.

**Prior NewTOS docs (wrong):** `_state.md §7` gave the bit table as bit 0 =
"`Cconin` echoes input", bit 1 = "Ctrl-S/Ctrl-Q flow control", bit 2 = "Ctrl-C
abort", and stated `conterm` is "read by `xconin`, `xnecin`, `xconrs`". None of
that is true of this EmuTOS build; those meanings appear to have been invented
from original-TOS lore. The console specs that referenced a `conterm`-gated echo
were likewise corrected.

**Compendium:** documents the classic four-bit layout (key click / repeat /
bell / shift-status) — which **matches the EmuTOS code**, so the old NewTOS spec
was wrong against *both* sources.

**Decision:** follow EmuTOS — the four bits mean key click / key repeat /
system bell / shift-state packing, and their effects live in the BIOS keyboard
and screen layer, **not** in the GEMDOS console calls. The bit *values and
semantics* remain contract (a program can poke `0x484` and observe them); the
GEMDOS console echo and Ctrl-key handling are unconditional. Corrected in
`_state.md §7` and every console spec.

---

## D-028: `Crawio` (GEMDOS 0x06) is never redirected to a file (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/console/06-crawio.md`

**EmuTOS:** the `funcs[]` entry for `Crawio` has **`stdio_typ = 0`**
(`bdos/bdosmain.c:161`, `{ F(xrawio), 0, 1 }`). The dispatcher's standard-handle
interception is guarded by `if (typ && fn && ((fn<12) || …))`
(`bdos/bdosmain.c:498`); with `typ == 0` the whole block is skipped for function
6. Consequently the `case 6:` arm *inside* that switch (`:505-508`) is
**unreachable** for the normal dispatch of `Crawio`. `Crawio` therefore always
talks to the console BIOS device (`Bconstat`/`getch`/`Bconout`), even when
stdin/stdout have been `Fforce`d to files.

**Prior NewTOS doc (wrong):** `06-crawio.md` listed a "Standard-handle
redirection" edge case claiming the dispatcher intercepts `Crawio` and does a
one-byte file read (returning `EOF_INDICATOR`) or file write. That path is dead
code for function 6 and never runs.

**Compendium:** does not address dispatcher-level redirection; not a Compendium
discrepancy — an internal spec error.

**Decision:** follow EmuTOS — `Crawio` is never redirected; it is genuinely raw
console I/O. Corrected in `06-crawio.md`. (The other console read/write
functions, which *do* set a non-zero `stdio_typ`, are redirected as before.)

---

## D-029: `Cconin`/`Cnecin`/`Cconrs` echo to the stdin device, not stdout (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/console/01-cconin.md`, `0a-cconrs.md`

**EmuTOS:** `xconin` echoes via `conin(h)` where `h = HXFORM(run->p_uft[0])` —
the **stdin** device (`bdos/console.c:388,401-403`). `xconrs` passes
`HXFORM(run->p_uft[0])` to `cgets` for **both** the read and the echo
(`bdos/console.c:535`). Neither uses `p_uft[1]` (stdout) for the echo. So
redirecting *stdout* does not move the echo; only redirecting *stdin* does. Echo
is also unconditional (no `conterm` gate — see D-027).

**Prior NewTOS doc (wrong):** `01-cconin.md` stated the echoed character "is
also written to the device in `run->p_uft[1]`" and reasoned about stdout
redirection. The echo device is `p_uft[0]`, not `p_uft[1]`.

**Not a Compendium discrepancy** — an internal spec error.

**Decision:** follow EmuTOS — the console read functions echo on the stdin
device. Corrected in `01-cconin.md` and `0a-cconrs.md`.

---

## D-030: GEMDOS allocator lives in `bdos/iumem.c`, not bdos/mem.c; there is no `mdalloc` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/structures/md.md`, `docs/api/gemdos/memory/*.md`

**EmuTOS:** the user-memory trap wrappers (`xmalloc`/`xmfree`/`xsetblk`/
`xmxalloc`/`xmaddalt`/`srealloc`, `find_mpb`, `umem_init`, `set_owner`) are in
`bdos/umem.c`; the internal allocator (`ffit`/`freeit`/`shrinkit`) is in
`bdos/iumem.c`; the MD/OS-memory meta-allocator (`xmgetmd`/`xmfremd`/`xmgetblk`/
`xmfreblk`) is in `bdos/osmem.c`. There is **no** bdos/mem.c (only the header
`bdos/mem.h`) and **no** function named `mdalloc`.

**Prior NewTOS doc (wrong):** `md.md` repeatedly cited `bdos/mem.c:xmalloc`,
`bdos/mem.c:xmfree`, and `bdos/mem.c:mdalloc`; the memory specs said
`ffit`/`freeit` live in bdos/mem.c.

**Not a Compendium discrepancy** — internal spec / citation error.

**Decision:** cite `bdos/umem.c` (wrappers), `bdos/iumem.c` (allocator),
`bdos/osmem.c` (MD pool). Corrected across `md.md` and the five memory specs.

---

## D-031: `freeit` does NOT clear `m_own`; `m_own == NULL` is not a "free" flag (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/structures/md.md`, `docs/api/gemdos/memory/49-mfree.md`,
`4a-mshrink.md`, `docs/api/gemdos/_state.md §8`

**EmuTOS:** `freeit` (`bdos/iumem.c:144-222`) only relinks the MD between
`mp_mal` and the address-sorted `mp_mfl` and coalesces neighbours — it **never**
writes `m_own`. `ffit` sets `m_own = run` on allocation (`bdos/iumem.c:134`),
but a freed MD keeps its stale owner pointer. Nothing in the alloc/free path
reads `m_own` for a free MD (membership is by list + `m_start`), so it is
internally harmless; but `m_own == NULL` is **not** an invariant of free MDs.
`shrinkit` likewise puts the freed tail MD on `mp_mal` without setting `m_own`
before `freeit`ing it (`bdos/iumem.c:242-260`). Only `Pterm`/`Ptermres`
(`free_all_owned`/`reserve_blocks`, `bdos/proc.c:77-101`) consult `m_own`, and
they walk `mp_mal` (allocated MDs), where it is meaningful.

**Prior NewTOS docs (wrong):** `md.md` asserted "Free flow … set `m_own =
NULL`" and the invariant "Every MD on `mp_mfl` has `m_own == NULL` (used as the
free flag)"; `4a-mshrink.md` claimed "the new freed-tail MD has `m_own = NULL`".

**Not a Compendium discrepancy** — internal spec error.

**Decision:** document that `freeit` leaves `m_own` stale; the free flag is list
membership, not `m_own`. NewTOS *may* cleanly clear `m_own` on free (a permitted
improvement — it does not rely on the stale value). Corrected in the four docs.

---

## D-032: `Maddalt` first add sets `pmdalt.mp_mal = NULL`, not `mp_rover` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/memory/14-maddalt.md`

**EmuTOS:** on the first successful `xmaddalt`, the one-time init writes
`pmdalt.mp_mfl = md; pmdalt.mp_mal = NULL; has_alt_ram = 1`
(`bdos/umem.c:509-514`); `mp_rover` is never touched. The merge path extends an
adjacent free MD in place and consumes no MD (`bdos/umem.c:472-483`).

**Prior NewTOS doc (wrong):** `14-maddalt.md` carried a pasted C body showing
`pmdalt.mp_rover = md; has_alt_ram = TRUE;` on the first add — a fabricated
line; the source sets `mp_mal = NULL`, not `mp_rover`.

**Not a Compendium discrepancy** — the pasted "source extraction" C in the old
spec did not match the actual `bdos/umem.c`.

**Decision:** follow the actual source (`mp_mal = NULL`). Also removed the
pasted C bodies from all five memory specs per the sharpened cite-never-paste
rule (`_template.md`). Corrected in `14-maddalt.md`.

---

## D-033: `Malloc(-1)` largest-free scope depends on `PF_TTRAMMEM`; `Mxalloc(-1)` is per selected pool(s) (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/memory/48-malloc.md`, `44-mxalloc.md`

**EmuTOS:** `xmalloc` forwards `Malloc(-1)` to `xmxalloc(-1, mode)` where `mode`
is `MX_STRAM` (no `PF_TTRAMMEM`) or `MX_PREFTTRAM` (with it)
(`bdos/umem.c:120-126`). In `xmxalloc`, `amount == -1` returns `ffit(-1,&pmd)`
for `MX_STRAM`, `ffit(-1,&pmdalt)` for `MX_TTRAM`, and the **larger of both
pools'** biggest single free block for the two preferred modes
(`bdos/umem.c:247-278`). It is the largest *single* MD, never the sum of free
space. With `CONF_WITH_ALT_RAM == 0`, `MX_TTRAM` (mode 1) query falls to
`default → NULL`.

**Prior NewTOS docs (imprecise/wrong):** `48-malloc.md` said `Malloc(-1)`
returns "the largest free block across both pools" unconditionally — true only
for `PF_TTRAMMEM` programs; a plain program's `Malloc(-1)` reports ST-RAM only.

**Not a Compendium discrepancy** — a precision fix.

**Decision:** document the `PF_TTRAMMEM`-dependent scope for `Malloc(-1)` and
the per-selected-pool(s) scope for `Mxalloc(-1, mode)`. Corrected in both specs.

---

## D-034: `Ptermres`/`Pterm` process specs cited a nonexistent bdos/mem.c and `NUMCURDIR = 16` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/process/31-ptermres.md`,
`4c-pterm.md`

**EmuTOS:**
- `Ptermres`'s TPA shrink is `xsetblk(0, run, blkln)` defined in **`bdos/umem.c`**
  (`bdos/umem.c:170`) — the same helper `Mshrink`/0x4A uses. There is **no**
  bdos/mem.c in the tree (only the header `bdos/mem.h`), consistent with D-030.
  `xsetblk` returns `EIMBA` (block not in a pool) / `EGSBF` (`blkln` >
  current block) / `E_OK`; `xtermres` **discards** this result (already D-006).
- `Pterm`'s current-directory teardown loops `NUMCURDIR = BLKDEVNUM = 26`
  (`include/bdosdefs.h:62`, `include/sysconf.h:43`), not 16.

**Prior NewTOS docs (wrong):** `31-ptermres.md` located `xsetblk` in
bdos/mem.c; `4c-pterm.md`'s configuration note said `NUMCURDIR = 16`. Both
contradict the source (the `NUMCURDIR = 16` figure is the same original-TOS
lore corrected for the drive group in D-025).

**Not a Compendium discrepancy** — internal spec / citation errors.

**Decision:** cite `bdos/umem.c` for `xsetblk`/`set_owner`; use `NUMCURDIR = 26`.
Both process specs were also migrated from the old "paste-the-C-body" format to
the sharpened full-contract template (cite-never-paste; native-AArch64 ABI table;
verification criteria) to match the other groups. Corrected in the two specs.

---

## D-035: `Pexec` never returns `EGSBF`; the load path collapses magic/reloc errors to `EPLFMT` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/process/4b-pexec.md`

**EmuTOS:** the `xexec` load path (`bdos/proc.c:256-360`, `bdos/kpgmld.c`) yields
exactly: `EINVFN` (unknown mode), `EFILNF` (`ixsfirst` miss), a pass-through of
`xopen`'s error (`EPTHNF`/`ENHNDL`/…), `EPLFMT` (magic ≠ `0x601A`, short header,
or a malformed *present* relocation — `bdos/kpgmld.c:48-63,184-185,265-268`),
`ENSMEM` (no memory for env/TPA, or image too big — `bdos/proc.c:216-300`,
`bdos/kpgmld.c:140-141`), and a pass-through of `Rwabs` I/O errors during load.
It does **not** produce `EGSBF` (-67) — that code originates only in `xsetblk`
(`bdos/umem.c:211`), reached via `Mshrink`/`Ptermres`, never via `Pexec`. It is
also tolerant: a failed read of the relocation-start longword loads the program
**without** relocating rather than failing (`bdos/kpgmld.c:176-178`).

**Prior NewTOS doc (wrong):** `4b-pexec.md` listed `EGSBF` (-67, "bad relocation
table") as a `Pexec` error return. The source uses `EPLFMT` for bad relocation
and never returns `EGSBF` from the exec path.

**Compendium:** historically lists `EGSBF` among `Pexec` errors; that does not
match this EmuTOS load path (so also a Compendium discrepancy). Compendium's tie
of `EGSBF` to `Pexec` appears to come from the shared setblock machinery.

**Decision:** follow EmuTOS — `Pexec` returns `EPLFMT` for bad
magic/format/relocation and never `EGSBF`; relocation-read failure loads
unrelocated. Corrected in `4b-pexec.md`.

---

## D-036: `Pexec`/`init_pd_fields` copies the command tail verbatim + a NUL; it does not write a `\r` or synthesize the length byte (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/process/4b-pexec.md`,
`docs/api/structures/basepage.md` (flagged for correction — out of this pass's
write scope)

**EmuTOS:** `init_pd_fields` (`bdos/proc.c:379-384`) builds `p_cmdlin` by copying
the caller-supplied `tail` string byte-for-byte, stopping at the first NUL or at
`PDCLSIZE (0x80)` bytes, then appending a single **NUL (0x00)** terminator. It
does **not** append a carriage return, and it does **not** itself write or
compute the length byte — the length-prefixed `[len][text…]` framing is whatever
the caller passes in `tail`. A `NULL`/empty `tail` therefore yields
`p_cmdlin[0] = 0` followed by a NUL, **not** `p_cmdlin[1] = '\r'`.

**Prior NewTOS doc (wrong):** `docs/api/structures/basepage.md §p_cmdlin` states
the tail is "terminated by `\r` (0x0D), not NUL", that trailing bytes after the
CR are zeroed, and that a NULL tail gives `p_cmdlin[1] = '\r'`. None of that is
what `init_pd_fields` writes — those are the classic-TOS command-line
*convention* (what a shell typically supplies in `tail`), not what `Pexec`
synthesizes.

**Compendium:** documents the `[length byte][command tail]` convention; the CR
terminator is a DOS/TOS shell convention, not something the OS's basepage builder
adds.

**Decision:** follow EmuTOS — the OS copies `tail` verbatim and NUL-terminates;
the length byte and any CR are the caller's framing. Documented correctly in
`4b-pexec.md` (Inputs → "Command tail format" + Edge case 5). `basepage.md`
§`p_cmdlin` should be corrected in a structures pass to say the OS appends a NUL
(not `\r`) and does not synthesize the length byte; left flagged here since the
structures doc is outside this process-group pass.

---

## D-037: Time/date set goes through **XBIOS** `Settime` (22/0x16), and `Sversion` byte order / version table / OS-header claim (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/gemdos/time/2a-tgetdate.md`, `2b-tsetdate.md`,
`2c-tgettime.md`, `2d-tsettime.md`, `docs/api/gemdos/version/30-sversion.md`,
`docs/api/gemdos/_state.md` §9, §12

**EmuTOS:**
- `xsetdate`/`xsettime` (`bdos/time.c:122,150`) commit via `Settime(...)`,
  which is **XBIOS function 22 (0x16)** — `#define Settime(a) xbios_v_l(22,a)`
  (`include/xbiosbind.h:40`) — reaching `settime()` (`bios/clock.c:1218`).
  `settime()` updates the GEMDOS globals `current_time`/`current_date` directly
  (`bios/clock.c:1221-1222`) **and** persists to the RTC. The boot seed comes
  from `Gettime()` = **XBIOS 23 (0x17)** (`time_init`, `bdos/time.c:160-165`).
- `Sversion`/`xgetver` (`bdos/bdosmain.c:298`) returns the compile-time
  constant `GEMDOS_VERSION` (`include/config.h:1590`, default `0x2000`), byte
  order **minor in the high byte, major in the low byte** (`include/config.h:1577-1578`).
  It does **not** read the OS header. The `config.h` enumeration of version
  values starts at `0x1300` (TOS 1.00/1.02); there is **no** `0x0F00` entry.

**Prior NewTOS docs (wrong):** the time specs said the setters call "BIOS
function 22" / "BIOS RTC" (it is **XBIOS** 22); several were still in the old
paste-the-C-body format without the native-AArch64 ABI table or verification
criteria. `30-sversion.md` carried a version table with a bogus `0x0F00` =
"TOS 1.00" row (not in `config.h`) and `_state.md §9` claimed the OS header is
"read by `Sversion`" — `Sversion` reads the `GEMDOS_VERSION` constant, not the
header.

**Not a Compendium discrepancy** — internal spec / citation errors (the BIOS-vs
-XBIOS mislabel, the invented version row, the OS-header conflation). The
day-0/month-0 acceptance remains tracked separately as **D-007** (unchanged).

**Decision:** cite `include/xbiosbind.h:40` (Settime = XBIOS 22) and
`bios/clock.c` (`settime`/`gettime`) for the RTC path; report `GEMDOS_VERSION =
0x2000` with the minor-high/major-low byte order; drop the `0x0F00` row; state
that `Sversion` is independent of the OS-header `os_version`. All five time/
version specs migrated to the sharpened full-contract template (cite-never
-paste; native-AArch64 ABI table; verification criteria) to match the other
groups. `_state.md` gained §12 (date/time globals + version constant).

---

## D-038: `Ssystem` cookie sub-ops — slot-index returns the tag (not `{tag,value}`), not-found is `EERROR`, and set/delete are root-gated (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/gemdos/154-ssystem.md`
**Source:** FreeMiNT @ `f906416` (EmuTOS has no `Ssystem`; FreeMiNT is the
authority, per `SOURCE-ANALYSIS-PROTOCOL.md` Track A source 2b).

**FreeMiNT:**
- Trap `0x154` → `sys_s_system` (`sys/syscall_vectors.c:473`); signature
  `long s_system(int mode, ulong arg1, ulong arg2)` (`sys/ssystem.c:57`).
- `S_GETCOOKIE` (8) calls `get_cookie(NULL, arg1, (ulong*)arg2)`
  (`sys/ssystem.c:279`). Its by-**index** form (high word of `arg1` == 0)
  returns the **tag** of the Nth cookie, **not** a `{tag,value}` pair
  (`sys/cookie.c:472-482`), and an over-large index returns `EINVAL`
  (`sys/cookie.c:458-462`). A tag search that misses returns `EERROR` (-1)
  (`sys/cookie.c:508-509`). `arg2 == 0` returns the value/tag in d0; `arg2 != 0`
  writes it to `*arg2` and returns `E_OK`. **No privilege** required.
- `S_SETCOOKIE` (9) and `S_DELCOOKIE` (26) are **root only** — non-root returns
  `EPERM` before touching the jar (`sys/ssystem.c:285-289,295-299`).
  `set_cookie` rejects any tag with a zero byte (`EINVAL`,
  `sys/cookie.c:534-541`) and returns `ENOMEM` when the jar is full
  (`sys/cookie.c:574-575`). `del_cookie` returns `EINVAL` when the tag is not
  found (`sys/cookie.c:606-607`) — a *different* not-found code than
  `S_GETCOOKIE`'s `EERROR`.

**Prior NewTOS doc (imprecise):** `154-ssystem.md` said the by-index form
"writes its `{tag,value}` (FreeMiNT copies the entry)" (it returns the **tag**
only), gave the not-found error vaguely as "the not-found error" (it is
`EERROR`/`EINVAL` depending on the op), and omitted the **root gate** + `EPERM`
on set/delete, the `ENOMEM`-when-full path, and the `del_cookie` `EINVAL`.

**Not a Compendium discrepancy** — no Compendium/MiNT PDF is in `references/`
(the in-tree `references/freemint/doc/programmer/newcall.html/ssystem.html` matches the source). This is a
spec-vs-FreeMiNT-source precision fix.

**Decision:** follow the FreeMiNT source. Corrected `154-ssystem.md`: by-index
returns the tag, not-found codes distinguished (`EERROR` for get, `EINVAL` for
delete), root gate + `EPERM` documented, `ENOMEM`/`EINVAL` set-side errors
tabulated. **Open question (not resolved):** the *numeric* error values NewTOS
returns — FreeMiNT's MiNT-errno set (`EERROR=-1`, `EINVAL=-25`, `EPERM=-38`,
`ENOMEM=-39`, `ENOSYS=-32`) collides with TOS codes (`ENOSYS -32 == EINVFN`;
`EBADARG -64 == ERANGE`; TOS has no `EINVAL`), so the mapping needs an ADR (see
`154-ssystem.md` Open questions).

---

## D-039: XBIOS out-of-range / unimplemented returns the **function number**, not `EINVFN` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/xbios/_dispatcher.md`, `_triage.md`,
`_tt-shifter.md`

**EmuTOS:** the shared `biosxbios` trampoline range-checks the function ID and,
when `d0 >= xbios_ent`, branches to `bx_ret_exc` (`bios/vectors.S:1325-1359`)
which `rte`s with **D0 unchanged** — D0 still holds the function number
(`moveq #0,d0; move.w (sp)+,d0`, `:1323-1324`). In-range unimplemented slots
use `xbios_unimpl` → `xbios_do_unimpl(number){return number;}`
(`bios/xbios.c:1251`). **Both** paths return the positive function number.
This is the same fact as D-008, restated for the *out-of-range* path.

**Prior NewTOS docs (wrong):** `_triage.md`'s "Build-profile boundary" and
`_tt-shifter.md`'s "Falcon overlap" said a trap with `ID > LAST_ENTRY (0x57)`
"returns `EINVFN` (-32)". It does not — it returns the function number.
`26-supexec.md` additionally claimed Supexec "returns `EINVFN` today (the
dispatcher catch-all)" — doubly wrong, since Supexec has its own table slot
(0x26) and never reaches the catch-all.

**Compendium:** documents `EINVFN` for unimplemented XBIOS — incorrect against
both the real hardware and EmuTOS (already noted in D-008).

**Decision:** follow EmuTOS — every unimplemented XBIOS call (out-of-range or
padded slot) returns its own function number. Corrected in `_dispatcher.md §3`,
`_triage.md`, `_tt-shifter.md`, `26-supexec.md`.

---

## D-040: `Initmous` type 0 / bad type forces `mousevec = just_rts` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/xbios/00-initmous.md`

**EmuTOS:** `Initmous` (`bios/mouse.c:40-122`) installs the mouse vector only
on the success path: `if (retval != 0 && type != 0) { … if (newvec != NULL)
kbdvecs.mousevec = newvec; } else { kbdvecs.mousevec = just_rts; }`
(`:105-121`). So **`type == 0` (disable) and any invalid `type` set
`mousevec = just_rts`** — the current handler is dropped — regardless of
`newvec`. The mode-config bytes for types 1/2/4 are sent only if `param !=
NULL`, and the button/Y-origin bytes are likewise `param`-gated. EmuTOS does
not implement Compendium's mouse `type == 3`.

**Prior NewTOS doc (incomplete):** `00-initmous.md` said `vec == NULL` "leaves
`kbdvecs.mousevec` untouched" and described type 0 only as "disable mouse",
without noting that type 0 / errors overwrite `mousevec` with `just_rts`.

**Compendium:** documents types 0/1/2 and a type 3 EmuTOS does not implement.

**Decision:** follow EmuTOS — document the `just_rts` install on
disable/error, the `param`-gating, and the unimplemented type 3. Corrected in
`00-initmous.md`.

---

## D-041: `Rsconf` query is `speed == -2`, and the MFP return omits `scr` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/xbios/0f-rsconf.md`

**EmuTOS:** the default handler `rsconf_mfp` (`bios/serport.c:217-249`)
treats **`baud == -2`** as the "return current baud code" query
(`rs232iorecptr->baudrate`, `:222-223`); `baud == -1` merely means "don't
change the baud". Each of `ucr`/`rsr`/`tsr`/`scr` is applied only when `>= 0`
(`:239-246`). The `ULONG` return packs the **old** registers as
`(ucr << 24) | (rsr << 16) | (tsr << 8)` (`:229`) — the low byte (`scr`) is
**not** included. The SCC handler packs a different subset.

**Prior NewTOS doc (wrong):** `0f-rsconf.md` said "`baud == -1` means query
current" (it is `-2`) and gave the return as `(ucr<<24)|(rsr<<16)|(tsr<<8)|scr`
(the `scr` byte is not in the MFP return).

**Compendium:** documents the baud table and flow modes; the `-2` query and the
exact return packing are EmuTOS specifics.

**Decision:** follow EmuTOS — `-2` is the baud query, `-1` is "keep", and the
MFP return is `(ucr<<24)|(rsr<<16)|(tsr<<8)`. Corrected in `0f-rsconf.md`.

---

## D-042: `Kbrate` defaults are 15/2 (not 25/5) and `struct keytbl` is 30 bytes (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/xbios/23-kbrate.md`, `docs/api/xbios/10-keytbl.md`,
`docs/api/xbios/_state.md`

**EmuTOS:**
- `kbrate` (`bios/ikbd.c:439-449`) returns `MAKE_UWORD(kb_initial, kb_repeat)`
  (old initial in the high byte, old repeat in the low byte), stores each
  non-negative parameter masked `& 0x00ff`, and treats a negative parameter as
  a query. Power-up defaults are `KB_INITIAL = 15` and `KB_REPEAT = 2`
  (`include/sysconf.h:68-69`, set by `kbd_init`, `bios/ikbd.c:1092-1093`), in
  ~20 ms units.
- `struct keytbl` (`bios/ikbd.h:57-86`) is **30 bytes** with eight fields:
  `norm`, `shft`, `caps`, `altnorm`, `altshft`, `altcaps`, `dead` (a
  `const UBYTE *const *`), and `features` (WORD). `Keytbl` (`:122-134`) sets
  only `norm`/`shft`/`caps` (sentinel `(UBYTE *)-1` = keep) and returns
  `&current_keytbl`.

**Prior NewTOS docs (wrong):** `_state.md §2` gave defaults `kb_initial = 25`
/ `kb_repeat = 5` in 50 Hz units, and described `current_keytbl` as only
`{unshift, shift, caps}`. Both contradict the source (the 25/5 figure is
original-TOS lore).

**Compendium:** documents the packed `kbrate` return + `-1` query; classic-TOS
default figures are often quoted differently — EmuTOS uses 15/2.

**Decision:** follow EmuTOS — defaults 15/2 in 20 ms units, `MAKE_UWORD`
return, `(UBYTE *)-1` sentinel, 30-byte 8-field `struct keytbl`. Corrected in
`23-kbrate.md`, `10-keytbl.md`, `_state.md §2`.

---

## D-043: `IOREC` is 14 bytes input-only; IKBD buffer is 256 bytes; RS-232 uses `EXT_IOREC` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/xbios/0e-iorec.md`, `docs/api/xbios/_state.md`

**EmuTOS:** the bare `IOREC` (`bios/iorec.h:20-27`) is **14 bytes**:
`UBYTE *buf; WORD size; volatile WORD head; volatile WORD tail; WORD low;
WORD high` — **input only**. `Iorec(1)` (IKBD) and `Iorec(2)` (MIDI) return
bare `IOREC`s; the IKBD buffer is **256 bytes** (`low=64`, `high=192`), MIDI
128 bytes (`bios/aciavecs.S`). `Iorec(0)` returns `rs232iorecptr`, an
**`EXT_IOREC`** (`bios/serport.h:57-65`): an input `IOREC` **plus** an output
`IOREC` plus `baudrate`/`flowctrl`/`ucr`/`datamask`/`wr5`.

**Prior NewTOS docs (wrong):** `0e-iorec.md` and `_state.md §3` described a
single 16-byte `IOREC` "with input + output ringbuffer" and an IKBD buffer of
128 bytes, and treated all three devices as one shape.

**Compendium:** documents the `IOREC` fields; the input-only vs `EXT_IOREC`
split and the exact sizes are EmuTOS specifics.

**Decision:** follow EmuTOS — 14-byte input-only `IOREC` for IKBD/MIDI,
`EXT_IOREC` for RS-232, IKBD buffer 256 bytes. Corrected in `0e-iorec.md`,
`_state.md §3`.

---

## D-044: `BCONMAP` field order is `{maptab, maptabsize, mapped_device}` (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/xbios/2c-bconmap.md`, `docs/api/xbios/_state.md`

**EmuTOS:** `BCONMAP` (`bios/serport.h:115-119`) is
`{ MAPTAB *maptab; WORD maptabsize; WORD mapped_device; }` — **`maptab` at
offset 0**. `MAPTAB` (`:106-113`) is 24 bytes: `Bconstat`, `Bconin`,
`Bcostat`, `Bconout`, `Rsconf`, `EXT_IOREC *Iorec`. `Bconmap(-2)` returns
`&bconmap_root`, so the field order is the layout a program walks.

**Prior NewTOS docs (wrong):** `2c-bconmap.md` and `_state.md §3` gave the
order as `{mapped_device, maptabsize, maptab}` — reversed.

**Not a Compendium discrepancy** — an internal spec / layout error.

**Decision:** use `{maptab, maptabsize, mapped_device}`. Corrected in
`2c-bconmap.md`, `_state.md §3`.

---

## D-045: `Bconmap` is unavailable (returns `0x2c`) on a single-serial build (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/xbios/2c-bconmap.md`

**EmuTOS:** `BCONMAP_AVAILABLE = (CONF_WITH_SCC || CONF_WITH_TT_MFP)`
(`bios/serport.h:18`). When it is false, `bconmap` compiles to a single
`return 0x2c;` (`bios/serport.c:1124`) — the function-number "unimplemented"
signal (D-008). On a single-serial build there is no `MAPTAB` and `Bconmap`
reports "not available".

**Prior NewTOS doc (wrong):** `2c-bconmap.md` said NewTOS "enables
`BCONMAP_AVAILABLE = 1` with a single-entry `MAPTAB`" and that `Bconmap(6)` is
a no-op — presenting a design choice as if it were the source behaviour. The
source default for one serial port is the `0x2c` return.

**Not a Compendium discrepancy** — a spec-vs-source / plan-drift error.

**Decision:** on the single-PL011 NewTOS profile, `Bconmap` returns `0x2c`
(unavailable), matching EmuTOS. Enabling a `MAPTAB` would be a deliberate,
documented NewTOS extension. Corrected in `2c-bconmap.md`.

---

## D-046: `Scrdmp` default `dump_vec` is `just_rts` (no built-in printer dump) (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/xbios/14-scrdmp.md`, `docs/api/xbios/_state.md`

**EmuTOS:** `scrdmp` (`bios/xbios.c:490-494`) calls `dump_vec` via `protect_v`
and resets `dumpflg = -1`. `dump_vec` (`0x502`) is initialised to `just_rts`
(a no-op) at `bios/chardev.c:84`; EmuTOS ships **no** built-in screen-dump
routine. Alt-Help does `dumpflg++` (`bios/ikbd.c:596`), which the VBL notices
to invoke `dump_vec`. `dumpflg` (`0x4EE`) defaults to `-1` (`bios/bios.c:357`).

**Prior NewTOS docs (imprecise):** `14-scrdmp.md` / `_state.md §5` said the
default `dump_vec` is `dmpfunc` that "writes the printer-byte stream" — there
is no such default; it is `just_rts`.

**Not a Compendium discrepancy** — a precision fix.

**Decision:** default `dump_vec = just_rts` (no-op); a NewTOS default dumper
(e.g. PNG) would be an install-time choice. Corrected in `14-scrdmp.md`,
`_state.md §5`.

---

## D-047: AES dispatcher DOES return -1 for an unsupported opcode (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 specs:** `docs/api/aes/_dispatcher.md`, `docs/api/aes/application.md`

**EmuTOS:** `crysbind` (`aes/gemsuper.c:77-420`) has a `default:` arm that sets
`unsupported = TRUE` (`:406-408`); after the switch it runs
`fm_show(ALNOFUNC, &opcode, 1)` (unless `opcode == 0`, which is silently
ignored "since some PRGs make this call") and sets **`ret = -1`**
(`:411-417`). So an out-of-range or not-compiled-in opcode returns `-1`, a
defined value. Applies to `appl_yield` (17) when `CONF_WITH_PCGEM == 0`, the
absent `appl_getinfo` (130) / `appl_search` (18), and every gap in the opcode
table (16, 18, 27–29, 49, 57–69, 83–89, 92–99, 115–119, ≥128).

**Prior NewTOS doc (wrong):** `aes/_dispatcher.md §2` stated "There is **no**
'invalid opcode' error path — an out-of-range opcode falls through the switch
and returns an undefined value (the AES is trusted)." The source has an
explicit `default` → `-1` path with an alert.

**Not a Compendium discrepancy** — an internal spec-vs-source error.

**Decision:** follow EmuTOS — unknown/uncompiled opcodes return `-1` (opcode 0
silently, others with an alert). The `-1` return is contract; the `fm_show`
alert is a reference-level UI behaviour NewTOS need not reproduce. Corrected in
`aes/_dispatcher.md §2` and referenced from `aes/application.md`.

---

## D-048: `appl_init` fills `ap_private` (not `ap_ptree`); `appl_getinfo`/`appl_search` absent; AESGLOBAL layout (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/application.md`

**EmuTOS:** the `APPL_INIT` arm (`aes/gemsuper.c:92-99`) writes exactly:
`AP_VERSION`=`AES_VERSION` (`global[0]`), `AP_COUNT`=1 (`global[1]`),
`AP_ID`=`rlr->p_pid` (`global[2]`), **`AP_PRIVATE`** (`global[3]`) =
`rs_trees[DESKTOP][ROOT].ob_spec`, `ap_planes` (`global[10]`)=`gl_nplanes`, and
`ap_3resv` (`global[11]`) = `&D`. It does **not** write `AP_PTREE` (`global[5]`)
— that is filled later by `rsrc_load`. The `AESGLOBAL` field/offset table is
`aes/gemrslib.h:15-27` + `include/aesdefs.h:147-157`. `appl_getinfo` (130),
`appl_search` (18) and `appl_control` have **no** opcode macro and **no**
dispatch `case` in this build (`include/aesdefs.h:16-24`, `gemsuper.c:88-123`);
they return `-1` (per D-047). `AES_VERSION` is build-dependent
(`include/config.h:1345-1355`) and is **0x0340** in the default build (all four
of `CONF_WITH_3D_OBJECTS`/`MENU_EXTENSION`/`WINDOW_COLOURS`/
`GRAF_MOUSE_EXTENSION` = 1). `CONF_WITH_PCGEM` defaults to 1, so `appl_yield`
(17) is present.

**Prior NewTOS doc (wrong/imprecise):** `aes/application.md` said `appl_init`
sets `ap_private = DESKTOP tree's ROOT ob_spec` **and separately** labelled the
value against `ap_ptree` in the field description; it also did not state that
`ap_planes`/`ap_3resv` are written, nor the exact `AES_VERSION` default, and
listed `appl_getinfo`/`appl_search` as candidate functions rather than
confirming their absence.

**Not a Compendium discrepancy** — a spec precision / completeness fix. (The
`appl_getinfo`/`appl_search` functions are real AES 4.1+/MagiC calls, but they
are not in EmuTOS `1a7484ab`, so cross-check against Compendium is deferred.)

**Decision:** follow EmuTOS — `appl_init` writes `ap_private` (not `ap_ptree`),
plus `ap_planes`/`ap_3resv`; `ap_version` = 0x0340 in the default build;
`appl_getinfo`/`appl_search`/`appl_control` are unimplemented (→ `-1`). If
NewTOS later adds `appl_getinfo` it is a documented extension. Corrected in
`aes/application.md` (with the full `AESGLOBAL` layout table) and
`aes/_state.md`.

---

## D-049: `evnt_multi` slot map, `MU_TOSVALID`/`MU_M3` flag values, and the `int_out[1..6]` return block (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/event.md`

**EmuTOS:** the event-library slot macros (`include/aesdefs.h:178-224`) fix the
`evnt_multi` (25) parameter layout exactly:
`MU_FLAGS = int_in[0]`, button spec `MB_CLICKS = int_in[1]` / `MB_MASK =
int_in[2]` / `MB_STATE = int_in[3]`, rectangle 1 `MMO1_FLAGS..MMO1_HEIGHT =
int_in[4..8]`, rectangle 2 `MMO2_FLAGS..MMO2_HEIGHT = int_in[9..13]`, timer
`MT_LOCOUNT = int_in[14]` / `MT_HICOUNT = int_in[15]`, message buffer
`MME_PBUFF = addr_in[0]`. The shared **return block** is
`EV_MX = int_out[1]`, `EV_MY = int_out[2]`, `EV_MB = int_out[3]`,
`EV_KS = int_out[4]`, `EV_KRET = int_out[5]`, `EV_BRET = int_out[6]`, with
`int_out[0]` = `RET_CODE` = the fired-event `MU_*` mask. The single-event waits
`evnt_button`/`evnt_mouse` fill `int_out[1..4]` through the `&EV_MX` pointer
(`gemsuper.c:130,133`). The flag values are `MU_KEYBD=0x01`, `MU_BUTTON=0x02`,
`MU_M1=0x04`, `MU_M2=0x08`, `MU_MESAG=0x10`, `MU_TIMER=0x20`,
**`MU_TOSVALID=0x003F`**, **`MU_M3=0x0100` (internal only)**; `ev_multi` is
entered as `ev_multi(MU_FLAGS & MU_TOSVALID, …)`, so an application **cannot**
request `MU_M3` (`gemsuper.c:153`, `aesdefs.h:256-263`).

**Prior NewTOS doc (wrong/imprecise):** `aes/event.md`'s `evnt_multi` `int_in`
table was off for the rectangles/timer (it lumped `int_in[1..3]` as generic
"button clicks/mask/state" and mislaid the M2 rect / timer indices), described
`MU_TOSVALID = 0x3F` only as "all of the above" without noting it is the mask
applied to user flags, and **omitted `MU_M3 = 0x0100`**. It also cited
non-existent return-index names (`EV_MOUT`/`EV_BUTTON`/`EV_MMGPBUFF`); the real
names are `EV_MX/EV_MY/EV_MB/EV_KS/EV_KRET/EV_BRET` = `int_out[1..6]`, and the
last slot is the **button** click count (`EV_BRET`), not "mouse click count".

**Not a Compendium discrepancy** — a spec precision / completeness fix (no PDFs
in `references/`, so the Compendium slot cross-check is deferred).

**Decision:** follow EmuTOS — use the exact `int_in[0..15]` / `addr_in[0]` /
`int_out[0..6]` slot map, the `MU_*` values incl. `MU_TOSVALID = 0x3F` (the
user-flag mask) and internal `MU_M3 = 0x0100`. Corrected in `aes/event.md` with
the full slot tables.

---

## D-050: `evnt_keybd` returns the key in `int_out[0]`; `evnt_dclick` queries via `setit == 0` (not `rate == -1`); dclick default index 3 (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/event.md`

**EmuTOS:**
- **`evnt_keybd` (20)** returns the packed key code (scancode high byte, ASCII
  low byte) directly in **`int_out[0]` (`RET_CODE`)** via
  `ret = ev_block(MU_KEYBD, 0)` (`gemsuper.c:127`). This differs from
  `evnt_multi`, where the key is delivered in `EV_KRET = int_out[5]`
  (`gemevlib.c:179`). The two calls use different slots for the same datum.
- **`evnt_dclick` (26)** selects get-vs-set by `EV_DCSETIT = int_in[1]`:
  `setit == 0` **reads** the current speed (ignoring `rate`); `setit != 0`
  **sets** `gl_dcindex = rate` (0..4) (`gemevlib.c:300-309`). It **always**
  returns `gl_dcindex`. Compendium sometimes documents the query as
  `rate = -1`; this EmuTOS build keys the query off `setit`, not a `-1` rate.
- The **double-click rate table** is `gl_dcrates[5] = {450, 330, 275, 220, 165}`
  ms (indices 0..4), and the AES sets the **default to index 3** (220 ms) at
  start-up (`ev_dclick(3, TRUE)`, `geminit.c:708`). The returned value is the
  **index (0..4)**, never the ms figure.

**Prior NewTOS doc (incomplete):** `aes/event.md` summarised `evnt_dclick` as
"get (setit=0) or set" without the default-index-3 fact or the rate table
values, did not state that `evnt_keybd`'s key lands in `int_out[0]` (vs
`evnt_multi`'s `int_out[5]`), and did not flag the `rate == -1` vs `setit == 0`
query-selection difference against Compendium.

**Not a Compendium discrepancy for `evnt_keybd`** (an internal completeness
fix); **is a Compendium discrepancy for `evnt_dclick`'s query selector**
(`setit`-driven, not `rate == -1`), with the Compendium cross-check deferred (no
PDFs in `references/`).

**Decision:** follow EmuTOS — `evnt_keybd` key in `int_out[0]`;
`evnt_dclick` query via `setit == 0`; default speed index 3; rate table
`{450,330,275,220,165}` ms; return value is the index. Documented in
`aes/event.md`.

---

## D-051: `menu_*` slot map (`int_in[0]` index / `int_in[1]` state), `menu_ienable` item-bit-15 = redraw flag, and the full `MN_SELECTED` `msg[3..7]` layout (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/menu.md`

**EmuTOS:**
- **Menu slot map.** Every menu selector puts its **primary index in
  `int_in[0]`** and its **state/flag in `int_in[1]`**: `SHOW_IT`/`ITEM_NUM`/
  `TITLE_NUM`/`MM_PID` = `int_in[0]`; `CHECK_IT`/`ENABLE_IT`/`NORMAL_IT` =
  `int_in[1]` (`include/aesdefs.h:275-283`). The tree is `MM_ITREE = addr_in[0]`,
  the `menu_text` replacement string `MM_PTEXT = addr_in[1]`, the `menu_register`
  title `MM_PSTR = addr_in[0]`.
- **`menu_ienable` item bit 15.** The dispatch is
  `do_chg(tree, ITEM_NUM & 0x7fff, DISABLED, !ENABLE_IT, (ITEM_NUM & 0x8000) != 0,
  FALSE)` (`aes/gemsuper.c:174-176`). `do_chg`'s signature is
  `do_chg(tree, iitem, chgvalue, dochg, dodraw, chkdisabled)`
  (`aes/gemmnlib.c:145-146`), so **bit 15 of the item word is the `dodraw`
  (redraw-now) flag**, not a title/special-item marker; the object index is the
  low 15 bits.
- **Redraw / disabled-guard per call.** `menu_icheck` passes `dodraw=FALSE`
  (never repaints; `gemsuper.c:172`); `menu_tnormal` passes `dodraw=TRUE,
  chkdisabled=TRUE` (repaints, and returns `0` on a disabled title;
  `gemsuper.c:179`, `gemmnlib.c:158-159`); `menu_ienable` passes `chkdisabled=FALSE`.
- **`MN_SELECTED` message layout.** Built by `ct_msgup` →
  `ap_sendmsg(appl_msg, MN_SELECTED, owner, title, item, treehi, treelo, parent)`
  (`aes/gemctrl.c:353-368`, `aes/gemwmlib.c:851-863`): `msg[0]=MN_SELECTED (10)`,
  `msg[1]=sender pid`, `msg[2]=0`, `msg[3]=title obj`, `msg[4]=item obj`, and —
  **with `CONF_WITH_MENU_EXTENSION` (the default build)** — `msg[5]=HIWORD(tree)`,
  `msg[6]=LOWORD(tree)`, `msg[7]=submenu parent`; without the extension `msg[5..7]`
  are 0. A desk-accessory pick sends `AC_OPEN (40)` with the DA id in `msg[4]`
  instead (`gemctrl.c:337-349`).
- **`menu_register` return.** Returns the slot index `0..NUM_ACCS-1` (0–5,
  `NUM_ACCS=6`, `include/sysconf.h:37`), or `-1` on a negative pid / all slots
  used (`aes/gemmnlib.c:789-816`); it stores the caller's string **pointer**, not
  a copy (`:806`).

**Prior NewTOS doc (wrong/incomplete):** `aes/menu.md` placed `ITEM_NUM`/
`TITLE_NUM` at `int_in[1]` and `CHECK_IT`/`ENABLE_IT`/`NORMAL_IT` at `int_in[2]`
(off by one slot); described `menu_ienable`'s `item & 0x8000` as "a
title-bar/special item" flag (it is the redraw flag); did not note that
`menu_icheck` never redraws or that `menu_tnormal` fails on a disabled title;
and gave only `msg[3]=title, msg[4]=item` for `MN_SELECTED`, omitting the
`msg[5..7]` tree-pointer/parent words present in the default build.

**Not a Compendium discrepancy** — a spec precision/completeness fix (no PDFs in
`references/`, so the Compendium slot/`msg[5..7]` cross-check is deferred). The
`menu_ienable` high item bit and the AES-3.3 `msg[5..7]` tree pointer are
documented in Compendium and match this build; verification deferred.

**Decision:** follow EmuTOS — `int_in[0]` = index, `int_in[1]` = state;
`menu_ienable` item bit 15 = redraw flag over a 15-bit index; `menu_icheck`
deferred draw; `menu_tnormal` redraws + guards disabled titles; the full
`MN_SELECTED` `msg[0..7]` layout incl. `msg[5..7]` under
`CONF_WITH_MENU_EXTENSION`; `menu_register` slot `0..5` / `-1`, pointer stored
not copied. Corrected in `aes/menu.md`.

---

## D-052: `objc_*`/`form_*` — no `DRAW3D` state bit, colour-word bit split, `objc_change` slots `int_in[6/7]`, `objc_find` does not skip DISABLED, `form_alert` grammar/return (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/object-form.md`,
`docs/api/structures/{object,tedinfo,iconblk,bitblk}.md`

**EmuTOS:** `aes/gemoblib.c`, `aes/gemobed.c`, `aes/gemfmlib.c`,
`aes/gemfmalt.c`, `aes/gemobjop.c`, `aes/gemgraf.c:612-628`,
`include/obdefs.h`, `include/aesdefs.h:48-65,305-384`, `aes/gemsuper.c:203-263`.

The prior `aes/object-form.md` (a thin outline) was verified against the
source. The following are the substantive corrections (all map to
desktop-rendering / dialog behaviour):

- **No `ob_state` `DRAW3D = 0x80` bit.** `obdefs.h:100-107` defines state bits
  only through `WHITEBAK = 0x40`; 0x80 is not a standard state. 3D drawing is
  driven by the **`ob_flags` FL3D\* bits** (`FL3DIND=0x200`, `FL3DBAK=0x400`,
  `FL3DACT=0x600`, mask `FL3DMASK=0x600`, `obdefs.h:92-98`), plus `SUBMENU=0x800`.
  The task brief's "DRAW3D 0x80" was incorrect and is **not** documented as a
  state bit.
- **Colour-word bit split (`gr_crack`, `gemgraf.c:612-628`):** border colour
  bits 15–12, text colour 11–8, **writing mode = bit 7** (set → `MD_REPLACE`),
  inside pattern = 3 bits 6–4, inside colour bits 3–0. For box types the
  **border thickness is byte 1 of the LONG `ob_spec`** (`gemobjop.c:57`).
- **`objc_change` slots:** new state = `int_in[6]` (`OB_NEWSTATE`), redraw flag
  = `int_in[7]` (`OB_REDRAW`), clip rect = `int_in[2..5]`
  (`aesdefs.h:352-377`). (The old outline gave the argument order but not the
  slot numbers.)
- **`objc_find` does NOT skip `DISABLED`.** It returns the deepest object that
  is `inside()` and not `HIDETREE` (`gemoblib.c:1080`); disabled objects are
  ignored only at the *form* layer (`fm_button`, `gemfmlib.c:224`). The old
  wording "deepest visible **non-disabled** object" was wrong.
- **`form_alert` grammar/return:** `[icon][msg][buttons]`, icon `0`=none /
  `1`=NOTE / `2`=QUEST / `≥3`=STOP; up to 5 msg lines (max 40, TOS cap 32) and
  3 buttons (max 20, TOS cap 10); `||`/`]]` escape to literal `|`/`]`; return
  is the **1-based** pressed-button number (`gemfmalt.c:163,258-270,395`,
  `fm_strbrk`).
- **`objc_sysvar` (48) is present in the default build** (`CONF_WITH_3D_OBJECTS
  = 1`, `include/config.h:1259`), returning the 3D indicator/activator/colour globals
  with defaults from `init_3d` (`gemoblib.c:1327-1344`); `AD3DVALUE` is
  get-only = `ADJ3DSTD` (2).
- **`form_dial` rects** occupy `int_in[1..4]` (small) and `int_in[5..8]`
  (large), four WORDs each (`aesdefs.h:315-322`).
- **`OBJECT`/`TEDINFO`/`ICONBLK`/`BITBLK` byte layouts** documented byte-exact
  in `docs/api/structures/` (24/28/34/14 bytes m68k; native 64-bit-pointer
  variants noted).

**Not a Compendium discrepancy** — a spec precision/completeness fix. No PDFs
in `references/`, so the Compendium cross-check of the slot layouts, the
colour-word split, the alert grammar and the `te_pvalid` codes is **deferred**.

**Decision:** follow EmuTOS — no `DRAW3D` state bit (FL3D\* flags instead);
the `gr_crack` bit split above; `objc_change` new-state/redraw at
`int_in[6]`/`[7]`; `objc_find` returns DISABLED objects (form layer filters);
`form_alert` `[icon][msg][buttons]` grammar with `||`/`]]` escapes and 1-based
return; `objc_sysvar` present with `CONF_WITH_3D_OBJECTS=1`. Corrected in
`aes/object-form.md` and the four `structures/*.md` files.

---

## D-053: `wind_*`/`graf_*` — active-title cue direction, `WF_KIND` unsupported, `WM_NEWTOP`/`WM_ONTOP` never emitted, `WF_*` field numbers, `WM_*`/`WA_*` layouts, `graf_watchbox` return, `graf_growbox` slot (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/window-graphics.md`, `docs/api/aes/_state.md`

**EmuTOS:** `aes/gemwmlib.c`, `aes/gemwrect.c`, `aes/gemgrlib.c`,
`aes/gemctrl.c:63-315`, `aes/gemgraf.c:405-415`, `aes/gemlib.h:38-94`,
`include/aesdefs.h:68-77,89-98,226-253,390-562`, `include/obdefs.h:32-39,136`,
`aes/gemsuper.c:265-360`.

The prior `aes/window-graphics.md` (a compact outline) was verified against the
source. Substantive corrections (all map to real window-redraw behaviour):

- **Active/inactive title cue direction.** The **topped (active)** window's
  `W_NAME` carries the **`IP_2PATT` dither** fill (`TOPPED_COLOR = 0x11a1`); the
  **untopped (inactive)** one is **plain/hollow** (`UNTOPPED_COLOR = 0x1100`)
  (`gemwmlib.c:150-151`). In the non-3D build, CLOSER/FULLER/scroll gadgets are
  built **only** for the topped window. (Confirms — and pins the direction of —
  the task brief's "TOPPED = IP_2PATT dither vs UNTOPPED plain".)
- **`WF_KIND` (1) is NOT supported** by `wm_get`/`wm_set` in this build — both
  fall to `default: return FALSE` (`gemwmlib.c:1591,1736`). Compendium documents
  `WF_KIND`; EmuTOS omits it. (NewTOS open question — trivial to implement since
  `w_kind` is stored.)
- **`WM_NEWTOP` (29) and `WM_ONTOP` (31) are declared but never emitted** by
  `aes/` or `desk/` (grep-confirmed) — TOS4/MultiTOS messages this build does
  not generate. The prior task hypothesis listed them as active; they are not.
- **`WF_*` field numbers** confirmed against source (`aesdefs.h:534-552`):
  `WF_WXYWH` 4, `WF_CXYWH` 5, `WF_PXYWH` 6, `WF_FXYWH` 7, `WF_HSLIDE` 8,
  `WF_VSLIDE` 9, `WF_TOP` 10, `WF_FIRSTXYWH` 11, `WF_NEXTXYWH` 12, `WF_NEWDESK`
  **14** (no field 13), `WF_HSLSIZ` 15, `WF_VSLSIZ` 16, `WF_SCREEN` 17,
  `WF_COLOR` 18, `WF_DCOLOR` 19, `WF_OWNER` 20. The source spells them
  `WF_HSLSIZ`/`WF_VSLSIZ`/`WF_WXYWH`/`WF_CXYWH`/`WF_FXYWH` (Compendium's
  `…SIZE`/`…WORKXYWH`/`…CURRXYWH`/`…FULLXYWH` are the same numbers).
- **`WM_*` message word layouts** (`gemctrl.c:81-315`, `gemwmlib.c:851-904`):
  `WM_SIZED` (27)/`WM_MOVED` (28) carry the **proposed new border rect** in
  `msg[4..7]`; `WM_HSLID` (25)/`WM_VSLID` (26) carry the **new 0..1000 slider
  position** in `msg[4]` and the window's `WS_CURR` `y, w, h` in `msg[5..7]`
  (`:196-197`, `:305`, `:314`; A93, 2026-09-21); `WM_ARROWED` (24) carries a **`WA_*` sub-action** in
  `msg[4]`; `WM_REDRAW` (20) carries the **dirty rect** in `msg[4..7]`;
  `WM_TOPPED`/`UNTOPPED` carry only `msg[3]=wh`; `WM_CLOSED`/`FULLED` carry `msg[3]=wh` and the window's `WS_CURR` in `msg[4..7]` (`:196-197`, `:314`; A77, 2026-09-20). `WA_*` =
  `UPPAGE 0, DNPAGE 1, UPLINE 2, DNLINE 3, LFPAGE 4, RTPAGE 5, LFLINE 6,
  RTLINE 7` (`aesdefs.h:245-253`).
- **`graf_watchbox` returns the `out` flag, not a literal "1 = released
  inside".** The actioned semantics are in `gemctrl.c:218-220` (CLOSER/FULLER
  fire when it returns TRUE — button up inside the gadget) (`gemgrlib.c:337-355`).
- **`graf_growbox`/`graf_shrinkbox` big rect is `int_in[4..7]` (`GR_I5`)**, one
  word after the small rect at `int_in[0..3]` (`gemsuper.c:275-279`).
- **`graf_mouse` form numbers** confirmed: `ARROW 0` … `OUTLN_CROSS 7`,
  `USER_DEF 255`, `M_OFF 256`, `M_ON 257`, plus `M_SAVE 258`/`M_RESTORE 259`/
  `M_PREVIOUS 260` under `CONF_WITH_GRAF_MOUSE_EXTENSION` (`aesdefs.h:423-438`).
- **`wind_calc` border model** (`gemwmlib.c:1807-1868`): base 1 px/side; title
  band adds to top, INFO adds to top, a vertical bar adds to right, a horizontal
  bar adds to bottom; a **SIZER-only** window still reserves a vertical bar band
  (TOS4 rule). 3D build adds `2·ADJ3DSTD` per band via `adj_wbox`/`adj_hbox`.

**Not a Compendium discrepancy** (except `WF_KIND` and `WM_NEWTOP`/`WM_ONTOP`,
which are genuine EmuTOS-vs-Compendium gaps) — mostly a spec
precision/completeness fix. No PDFs in `references/`, so the Compendium
cross-check of the field numbers, message layouts, `WA_*` codes, and
`graf_mouse` forms is **deferred**.

**Decision:** follow EmuTOS — topped title = `IP_2PATT` dither, untopped =
hollow; `WF_KIND` unsupported here (NewTOS may add it, decide vs Compendium);
`WM_NEWTOP`/`WM_ONTOP` recognised but not emitted (MultiTOS, deferred); the
`WF_*`/`WM_*`/`WA_*`/`graf_mouse` tables above; `graf_watchbox` returns the
`out` flag with `gemctrl` inside-release action; `graf_growbox` big rect at
`GR_I5`; the `wind_calc` border model incl. the SIZER-only vertical band.
Corrected in `aes/window-graphics.md`; window/rect/active-window globals added
to `aes/_state.md`.

---

## D-054: `rsrc_*`/`fsel_*`/`scrp_*`/`shel_*` — `R_*` type table 0–16, `NEW_FORMAT_RSC` bit, `rs_obfix` signed-offset + width-80 rule, `rsrc_gaddr` sole `addr_out`, `rsrc_load`→`sh_find`, `fsel_*` button = `int_out[1]`, `scrp_read`/`write` always-1/no-validate, `shel_write` four modes + `isgem`/`isover`, `shel_envrn` slot swap, shell buffer sizes (spec-vs-source)

**Date:** 2026-07-03
**Phase 00 spec:** `docs/api/aes/resource-fileselector.md`,
`docs/api/aes/scrap-shell.md`, `docs/api/aes/_state.md`

**EmuTOS:** `aes/gemrslib.c`, `aes/gemfslib.c`, `aes/gemsclib.c`,
`aes/gemshlib.c`, `aes/gemsuper.c:308-405`, `include/rsdefs.h:16-39`,
`include/aesdefs.h:80-115,456-465,568-621`, `aes/gemrslib.h:15-27`,
`aes/gemlib.h:97,120-129`, `include/sysconf.h:51-74`, `util/optimize.c:157-172`.

The prior `aes/resource-fileselector.md` + `aes/scrap-shell.md` (compact
outlines) were verified against the source. Substantive corrections:

**Resource manager:**
- **`R_*` type table is 0–16, not 0–15.** `R_FRIMG = 16` (free images) exists
  (`aesdefs.h:594`). Codes 8/11/14 alias 2/3/4 (sub-pointer = struct's first
  field). `R_STRING`(5)/`R_IMAGEDATA`(6) return the **dereferenced data**;
  `R_FRSTR`(15)/`R_FRIMG`(16) return the **address of the pointer slot**
  (`gemrslib.c:112-176`).
- **`rsh_vrsn` is a bit field, not `0 = classic`.** `NEW_FORMAT_RSC = 0x0004`
  bit marks colour-icon files whose true length is a LONG at offset `rsh_rssize`
  (`rsdefs.h:39`, `gemrslib.c:764-772`).
- **`rs_obfix` low byte is signed, and width 80 → full screen.** `char × cell` +
  signed low byte (`>128 → −256`); `ob_w` char `== 80` maps to `gl_width`
  (`gemrslib.c:53-83`). Box `ob_spec` (`G_BOX`/`G_IBOX`/`G_BOXCHAR`) is **not**
  relocated (colour word); `G_CICON` `ob_spec` is an index→pointer
  (`gemrslib.c:622-635`).
- **`rsrc_load` calls `sh_find` first** to locate the file via the shell search
  path (`gemrslib.c:836`) — not just the current dir.
- **`rsrc_gaddr` is the only AES call whose `addr_out[0]` the dispatcher copies
  back** (`gemsuper.c:446-447`); it returns `1`/`0` for valid/invalid **type**
  and does **not** range-check the index (`gemrslib.c:715-721`).

**File selector:**
- **`fsel_*` button return is `int_out[1]` (`FS_BUTTON`), not an `addr_out`**
  (`aesdefs.h:465`). `fs_input`'s **`int_out[0]` is 1 whenever the dialog ran**
  (0 only on NULL path / no memory); the OK/Cancel value is `inf_what(tree,
  FSOK)` = **1 (OK) / 0 (Cancel) / −1** (`gemfslib.c:922`, `optimize.c:157-172`).
- **`fsel_exinput` label is `addr_in[2]` (`FS_TITLE`)**, defaulting to the "Item
  Selector" string when NULL (`gemfslib.c:691-692`).
- Path/mask split via `fs_pspec`/`fs_back`; empty mask → `*.*`; empty path →
  `"<curdrive>:\*.*"`; folders sort first (`\007` prefix); editing the mask
  turns OK into a rescan (`gemfslib.c:199-236,524-532,634-638,762-767`).

**Scrap manager:**
- **`scrp_read`/`scrp_write` always return 1 and never validate/create** — bare
  `strcpy`s to/from `D.g_scrap` (`gemsclib.c:35-62`). The old "returns 0 if
  unset" / "creates it if needed" were both wrong.

**Shell manager:**
- **`shel_write` has four modes, not two:** `SHW_NOEXEC 0`, `SHW_EXEC 1`,
  `SHW_SHUTDOWN 4`, `SHW_RESCHNG 5` (`aesdefs.h:617-621`). The middle/third
  `int_in` words are **`isgem`** (GEM vs char mode) and **`isover`** (shutdown
  level / rez family), not `isgr`/`iscr`/"launch mode" (`gemshlib.c:127-196`).
  Always returns `1`. **`SHW_SHUTDOWN` is `#if CONF_WITH_SHUTDOWN` = default 0**
  → compiled out on the generic build (`include/config.h:2028-2034`).
- **`shel_envrn`'s output pointer is `addr_in[0]` (a `char **`) and its search
  string is `addr_in[1]`** — the reverse of the old spec (`aesdefs.h:611-612`,
  `gemsuper.c:396`). Returns a pointer *into* `ad_envrn`, or NULL.
- **`shel_find`'s search order** is app-dir → current-dir → drive-root → AES
  `PATH=` (`gemshlib.c:389-472`); it rewrites the buffer in place and returns
  1/0, saving/restoring the caller's DTA.
- **Buffer sizes are contract:** command tail = `CMDTAILSIZE = 128` bytes
  (`memcpy`, not NUL-bounded); shell buffer = `SIZE_SHELBUF = 4192` bytes
  (`gemlib.h:97,129`, `sysconf.h:74`). `shel_get`/`shel_put`/`shel_read`/
  `shel_write` are **unbounded `memcpy`s** — no length clamp.
- **Scrap/shell/rdef/wdef gating:** `scrp_clear`(82)/`shel_rdef`(126)/
  `shel_wdef`(127) are `#if CONF_WITH_PCGEM` (default 1) → `-1` when off.

**Not (mostly) a Compendium discrepancy** — a spec precision/completeness fix.
No PDFs in `references/`, so the Compendium cross-check of the `R_*` codes, the
`rs_obfix` rule, the `fsel_*` button convention, the `shel_write` modes, the
`shel_envrn` slots, and the buffer sizes is **deferred**.

**Decision:** follow EmuTOS — `R_*` 0–16 with the deref/address-of split;
`NEW_FORMAT_RSC` bit + trailing-length; signed-low-byte + width-80 `rs_obfix`;
box `ob_spec` not relocated; `rsrc_load`→`sh_find`; `rsrc_gaddr` sole `addr_out`,
no index check; `fsel_*` button at `int_out[1]` = 1/0/−1, `int_out[0]` = ran;
`fsel_exinput` label at `addr_in[2]`; `scrp_read`/`write` always-1/no-validate;
`shel_write` four modes with `isgem`/`isover`, `SHW_SHUTDOWN` off by default;
`shel_envrn` output `addr_in[0]`/search `addr_in[1]`; `shel_find` app-dir-first
order; 128-byte tail + 4192-byte shell buffer. Corrected in
`aes/resource-fileselector.md` + `aes/scrap-shell.md`; resource/scrap/shell
globals added to `aes/_state.md` §6c. **This completes the AES manager set and
the full Phase-00 API spec re-derivation.**

---

## D-055: `Protobt`'s "executable" flag is a whole-sector checksum trick, not a BRA.S byte write; `disktype` is 0..6, not 0..3 (spec-vs-source)

**Date:** 2026-07-08 (XBIOS re-validation pass, Task #25)
**Phase 00 spec:** `docs/api/xbios/_floppy-stubs.md`

**EmuTOS:** `protobt()` (`bios/floppy.c:690-737`) never writes offset 0x00/0x01
(`struct bs.bra`, `bios/blkdev.h:44`). The "executable" state is controlled
entirely by the boot sector's **512-byte word checksum**
(`compute_cksum()`, `bios/blkdev.c:69-75`, sums all 256 words): `is_exec =
(compute_cksum(b) == 0x1234)` (`:696`); `execflag < 0` keeps the current
`is_exec`; `execflag == 0`/`> 0` force non-exec/exec. Either way `protobt`
recomputes the **last two bytes of the sector** (`cksum[0..1]`, offset
0x1FE/0x1FF) so the whole-sector checksum becomes exactly `0x1234`
(executable) or, "for hyper-compatibility" with real Falcon/TT TOS (source
comment `:723-729`), exactly `0x1235` (non-executable — not "any value ≠
0x1234" as the generic Atari spec allows). Separately, `disktype` selects one
of **7** template entries (`NUM_PROTOBT_ENTRIES = ARRAY_SIZE(protobt_data) =
7`, `bios/floppy.c:672-682`), not 4; `serialno >= 0x01000000` triggers a
**random** serial via `Random()` (`:702-703`), not a literal stamp.

**Prior NewTOS doc (wrong):** `_floppy-stubs.md`'s `Protobt` section said
"`execflag`: 0 = non-bootable (clears bytes 0..1); ≠0 = bootable (sets `0x60
0x1E` BRA.S …)" — a mechanism EmuTOS does not implement — and "`disktype`:
0..3", omitting both the 7-entry range and the `>= 0x01000000` random-serial
rule.

**Compendium:** the generic BRA.S / "any non-0x1234 checksum is
non-executable" description matches the *classic* Atari ST boot-sector
convention; EmuTOS's `protobt()` implements the checksum-only mechanism (no
BRA.S write) and pins the non-executable checksum to `0x1235` specifically,
matching Falcon/TT TOS rather than the generic spec.

**Decision:** follow EmuTOS — document the checksum-based exec mechanism,
the 7-entry `disktype` range, and the random-serial rule. Corrected in
`_floppy-stubs.md`.

---

## D-056: `TT_SHIFTER_BITMASK` is `0x970f`, not `0x0700` (spec-vs-source)

**Date:** 2026-07-08 (XBIOS re-validation pass, Task #25)
**Phase 00 spec:** `docs/api/xbios/_tt-shifter.md`

**EmuTOS:** `#define TT_SHIFTER_BITMASK 0x970f` (`bios/screen.h:33`). The mask
covers bits 0-3 (colour bank), bits 8-10 (resolution — the subset the
mode-code table documents), bit 12, and bit 15 — not just bits 8-10.
`esetshift`/`egetshift` mask the *whole* register with this value
(`bios/screen.c:194-195,215`).

**Prior NewTOS doc (wrong):** `_tt-shifter.md` stated `TT_SHIFTER_BITMASK =
0x0700` ("only bits 8..10 encode the resolution"), understating the real
mask by omitting bits 0-3/12/15.

**Compendium:** not present in `references/`; cross-check deferred.

**Decision:** follow EmuTOS — correct the mask value; keep the resolution
sub-table (still accurate as a subset of the corrected mask). Corrected in
`_tt-shifter.md`.

---

## D-057: `EsetPalette`/`EgetPalette` are `WORD`-returning in EmuTOS (not `void`); most TT-shifter ABI shapes are not in `xbiosbind.h` (spec-vs-source)

**Date:** 2026-07-08 (XBIOS re-validation pass, Task #25)
**Phase 00 spec:** `docs/api/xbios/_tt-shifter.md`

**EmuTOS:** `bios/screen.c:265` / `:290` declare `WORD esetpalette(...)` /
`WORD egetpalette(...)`, with an explicit comment (`:260-263`) that this is a
**deliberate deviation** from the Atari-documented `void` signature: *"if the
TT shifter is not present, it should return the function number in a WORD,
which is the de facto TOS standard for unimplemented xbios functions."* Both
return `0` on success or the function number (`0x54`/`0x55`) when
`!has_tt_shifter` — the same D-008 pattern as the other six TT-shifter calls.
Separately, `include/xbiosbind.h` only defines binding macros for **two** of
the eight TT-shifter opcodes — `EgetShift() xbios_w_v(81)` (`:59`) and
`EsetColor(a,b) xbios_w_ww(83,a,b)` (`:60`); `EsetShift`/`EsetBank`/
`EsetPalette`/`EgetPalette`/`EsetGray`/`EsetSmear` have **no** macro there.

**Prior NewTOS doc (wrong):** `_tt-shifter.md`'s ABI table listed `xbios_v_wwl`
(`void`) for `EsetPalette`/`EgetPalette` and attributed all eight ABI shapes
to `include/xbiosbind.h`, when only two are actually defined there.

**Compendium:** documents `EsetPalette`/`EgetPalette` as `void`, matching the
*declared* Atari API but not EmuTOS's actual C signature/return value.

**Decision:** follow EmuTOS — document the `WORD` return (D-008 pattern) for
`EsetPalette`/`EgetPalette` and correct the ABI-source attribution.
Corrected in `_tt-shifter.md`.

---

## D-058: The NewTOS XBIOS "profile" mixes two mutually-exclusive EmuTOS build branches for the screen group (spec-internal consistency finding)

**Date:** 2026-07-08 (XBIOS re-validation pass, Task #25)
**Phase 00 specs:** `docs/api/xbios/_dispatcher.md`, `_tt-shifter.md`

**EmuTOS:** `include/config.h:2147-2157` — `#if !CONF_WITH_ATARI_VIDEO` /
`#if CONF_WITH_TT_SHIFTER` / `#error CONF_WITH_TT_SHIFTER requires
CONF_WITH_ATARI_VIDEO.` A real EmuTOS build can **never** have
`CONF_WITH_TT_SHIFTER=1` (the TT-shifter opcode table shape `_dispatcher.md`
/ `_triage.md` select as the "NewTOS target profile") together with
`CONF_WITH_ATARI_VIDEO=0` (the "no Atari video" runtime fallback —
`Physbase()==Logbase()`, `Getrez()` returns the `sshiftmod` shadow, `Setcolor`
fakes `return 0`, … — that `02-physbase.md`/`04-getrez.md`/`05-setscreen.md`/
`06-setpalette.md`/`07-setcolor.md`/`25-vsync.md` extract as the NewTOS
profile behaviour).

**Finding:** this is not a spec-vs-source error in any single function's
extraction — each function's documented behaviour is a real, correctly-cited
EmuTOS `#if` branch — but the two groups of specs silently assume **different,
mutually-exclusive** `CONF_WITH_*` combinations without saying so. A reader
combining "table has real `Eset*`/`Eget*` slots" with "`Physbase` falls back
to `Logbase`" would be describing a `#if` combination EmuTOS itself refuses
to compile.

**Decision:** NewTOS deliberately combines the *table-shape* branch (TT
shifter opcodes present) with the *runtime-behaviour* branch (no Atari video
MMIO) because the real target (BCM2711, no Shifter/TT-Shifter hardware at
all) cannot faithfully run either branch's hardware path — both are already
software shadows on NewTOS. Documented explicitly in `_dispatcher.md §4` and
`_tt-shifter.md`'s header so the combination is understood as a NewTOS
choice, not an EmuTOS-attested configuration.

---

## D-059: `struct _protobt` is 19 bytes, not 17; several stale line citations in the `Protobt` section (spec-vs-source)

**Date:** 2026-07-09 (XBIOS re-validation pass, code-audit follow-up)
**Phase 00 spec:** `docs/api/xbios/_floppy-stubs.md`

**EmuTOS:** `struct _protobt` (`bios/floppy.c:662-674`, `PACKED`) totals
**19 bytes**: `bps[2]+spc+res[2]+fat+dir[2]+sec[2]+media+spf[2]+spt[2]+
sides[2]+hid[2]` = 2+1+2+1+2+2+1+2+2+2+2 = 19. This is consistent with the
BPB field range already stated elsewhere in the same doc — `bps..hid`
occupies offset `0x0B-0x1D` in `struct bs` (`bios/blkdev.h:47-57`), and
`0x1D-0x0B+1 = 19`. Separately, several `bios/floppy.c`/`bios/blkdev.c` line
citations in the `Protobt` section had drifted from the current source
revision: `protobt_data[]` (the 7-entry template array) is
`bios/floppy.c:676-687` (`NUM_PROTOBT_ENTRIES = ARRAY_SIZE(protobt_data)`
defined at `:688`, not embedded in the array's own range); the `protobt()`
function itself is `bios/floppy.c:690-736` (not `-737`, a blank line);
`compute_cksum()` is `bios/blkdev.c:69-79` (not `-75`, which cuts off the
closing brace); the `serialno`-handling `if` block is `:698-704` (not
`:701-706`), with the "reversed as per Atari TOS" comment on `:701` (not
`:704`); the `disktype` copy loop is `:706-713` (not `:708-715`); the media
byte literals are on `:679-685` (not `:674-680`); and the "hyper-
compatibility" comment justifying the `0x1235` non-executable checksum is
`:722-726` (not `:723-729`, which runs into unrelated code).

**Prior NewTOS doc (wrong):** `_floppy-stubs.md` stated "the whole
**17-byte** `_protobt` template" — directly contradicting its own correct
`0x0B-0x1D` (= 19-byte) offset range given two sentences earlier in the same
paragraph — plus the stale line citations listed above.

**Compendium:** not present in `references/`; cross-check deferred.

**Decision:** follow EmuTOS — the struct is 19 bytes; the offset range
(`0x0B-0x1D`) was already correct and needed no change, only the "17-byte"
prose. Corrected the prose and every stale citation in `_floppy-stubs.md`'s
`Protobt` section.

---

## D-060: `Setprt` (XBIOS 0x21) is a pure software config word — NewTOS honours it independent of `CONF_WITH_PRINTER_PORT` (spec-vs-source / NewTOS decision)

**Date:** 2026-07-09 (XBIOS re-validation pass, code-audit follow-up)
**Phase 00 specs:** `docs/api/xbios/21-setprt.md` (new), `_dispatcher.md`,
`_triage.md`

**EmuTOS:** `WORD setprt(WORD config)` (`bios/parport.c:49-57`) is pure
software: `old_config = printer_config; if (config != -1) printer_config =
config; return old_config;`. It touches **no hardware register at all** —
the Centronics/PSG output path lives entirely in the separate `prnout()`
helper (`bios/parport.c:62-105`), which `setprt()` never calls.
`printer_config` (`bios/parport.c:36`) defaults to `0` at `parport_init()`
(`bios/parport.c:118`, comment: *"Setprt() default: output via parallel
port"*). However, in EmuTOS both `setprt()` and `prnout()` sit inside the
*same* `#if CONF_WITH_PRINTER_PORT` block (`bios/parport.c:45-106`), and the
`xbios_vecs[]` slot for 0x21 is `xbios_unimpl` when that config is off
(`bios/xbios.c:1332-1336`, returning the function number per D-008).
`CONF_WITH_PRINTER_PORT` defaults to `0` for non-Atari hardware
(`include/config.h:673,707-708`) and to `1` otherwise
(`include/config.h:987-988`) — a literal EmuTOS build for the
`!CONF_ATARI_HARDWARE` profile (the one NewTOS's target most resembles)
would not compile `setprt()` at all.

**Prior NewTOS doc (wrong/missing):** `_dispatcher.md` and `_triage.md`
grouped 0x21/Setprt with the MFP/NVRAM "printer/NVRAM" `xbios_unimpl`
entries (`xbios/_dispatcher.md:171` pre-fix, `_triage.md:59` pre-fix), i.e.
treated it as absent hardware in the same category as the floppy/PSG stub
groups. No per-function spec existed.

**Decision — NewTOS deviates deliberately from a literal
`CONF_WITH_PRINTER_PORT=0` port:** `Setprt`'s get/set of `printer_config`
has zero hardware dependency; only `prnout` needs the (absent) parallel
port. Per `docs/COMPATIBILITY.md` ("if a program can read it … through a
documented interface, it is contract"), the save/restore idiom (`old =
Setprt(cfg); …; Setprt(old);`) is part of the observable contract. Falling
back to `xbios_unimpl`'s function-number-return pattern (D-008) would make
`Setprt(-1)` return `33` instead of the stored config word, silently
corrupting that idiom — with no compensating benefit, since honouring the
word costs nothing (a single scalar `WORD`, default `0`, no I/O). This is
distinct from the genuinely hardware-dependent stubs (floppy `EUNDEV`,
PSG `0`/no-op) — those fail because the *operation* needs absent silicon;
`Setprt` needs none. NewTOS therefore implements `Setprt` with the same
store/return contract EmuTOS gives it when `CONF_WITH_PRINTER_PORT=1`, while
still having **no** parallel-port hardware — the same "config variable =
honour, real HW = absence contract" split already applied to `Rsconf`'s
baud-index shadow (`docs/api/xbios/0f-rsconf.md`). Documented in the new
`docs/api/xbios/21-setprt.md`; `_dispatcher.md`/`_triage.md` and
`docs/HARDWARE-PROFILE.md` §3.9 updated to move 0x21 out of the
printer-hardware-absence grouping.

---

## D-061: GEMDOS's unimplemented-function convention is `EINVFN`, the **opposite** of BIOS/XBIOS's function-number return — confirmed, not a spec bug (cross-trap verification)

**Date:** 2026-07-09 (GEMDOS Batch 1 re-validation, Task 1)
**Phase 00 spec:** `docs/api/gemdos/_dispatcher.md` §4

**Context:** D-008/D-039 established that BIOS/XBIOS's `xbios_unimpl` returns
the **function number**, not `EINVFN`, for both an out-of-range ID and an
in-range-but-unimplemented slot — the original XBIOS bug was returning
`EINVFN` where the function number was expected. Before auditing GEMDOS
(trap 1) it had to be confirmed from source whether GEMDOS follows the same
convention or a different one — guessing by analogy from the BIOS/XBIOS
finding would risk "fixing" GEMDOS in the wrong direction.

**EmuTOS (verified against `bdos/bdosmain.c` @ `1a7484ab`):** GEMDOS does
**not** follow the BIOS/XBIOS convention. Both of GEMDOS's unimplemented
paths return `EINVFN` (-32), a literal error code, never the function number:
- **Out-of-range** (`fn > MAX_FNCALL`, i.e. `fn > 0x57`): `osif()` returns
  `EINVFN` directly (`bdos/bdosmain.c:444-445`).
- **In-range, unimplemented slot:** every gap in `funcs[]` is built with the
  `NI` macro (`bdos/bdosmain.c:145` `#define NI F(ni)`), which points
  `f->fncall.v` at `static long ni(void) { return EINVFN; }`
  (`bdos/bdosmain.c:307-310`), reached through the ordinary arity-dispatch
  switch (`bdos/bdosmain.c:667-700`).

Both paths in GEMDOS agree with each other (`EINVFN`, always); both paths in
BIOS/XBIOS agree with each other (function number, always) — but the two
traps use **opposite** conventions from each other. This is a genuine,
verified difference between traps, not an inconsistency in either spec.

**Prior NewTOS doc:** `docs/api/gemdos/_dispatcher.md` §4 already documented
`EINVFN` correctly for both paths before this pass — no factual correction
was needed. What was missing was the explicit cross-trap contrast, now added
to §4, so a future auditor does not "fix" GEMDOS toward the BIOS/XBIOS
function-number convention (or vice versa) by analogy instead of re-reading
each trap's source.

**Decision:** NewTOS's GEMDOS dispatcher returns `EINVFN` for any unknown
function ID (out-of-range or in-range-unimplemented), matching EmuTOS;
NewTOS's BIOS/XBIOS dispatcher returns the function number for the same
cases (D-008/D-039). These are two independently-correct, opposite contracts
— do not unify them.

**Additional finding (same pass):** GEMDOS's dispatcher does have a
per-function argument-count convention relevant to a later code audit: each
`funcs[]` entry (`FND`, `bdos/bdosmain.c:120-133`) carries `wparms` — the
parameter size in 16-bit WORDs (a `LONG`/pointer counts as 2) — and `osif()`
dispatches through a `switch (f->wparms)` arity table (0/1/2/3/4/5/7 slots,
`bdos/bdosmain.c:667-700`) that calls the matching `union fncall` member with
that many `short` arguments pulled from `pw[1]…pw[wparms]`. This table was
cross-checked function-by-function against `bdos/bdosmain.c:142-289` and
matches `_dispatcher.md` §6 exactly; no correction needed.

---

## D-062: `Super` (GEMDOS 0x20) is a new spec, and its NewTOS implementation is grouped with ADR-Supexec (spec gap filled / decision grouping)

**Date:** 2026-07-09 (GEMDOS Batch 1, follow-up)
**Phase 00 spec:** `docs/api/gemdos/process/20-super.md` (new)

**Gap found:** unlike every other function EmuTOS implements in the GEMDOS
`funcs[]` table, `Super` (0x20) had no spec file, because it is intercepted
directly in the trap-1 assembly trampoline (`bdos/rwa.S:159`,
`x20_usr`/`x20_sup`/`x20_inq` at `:280-346`) and therefore never shows up in
a "one spec per `funcs[]` entry" inventory sweep (`_dispatcher.md` §6 already
noted it is "not in `funcs[]`", but no per-function spec existed). Flagged
as a STOP-block in the GEMDOS Batch 1 report; the coordinator confirmed
creating the spec now (documenting EmuTOS's behaviour), while leaving the
NewTOS/AArch64 implementation itself undecided.

**EmuTOS (now documented in full in `20-super.md`):** `Super` toggles the
CPU mode relative to the caller's current mode (never "set to supervisor")
- user-to-super always raises the S-bit, super-to-user always clears it,
with `stack` (0 / 1-inquiry / explicit address) selecting only which stack
pointer becomes active in the destination mode. The supervisor-to-user
direction has a genuine, source-verified D0 asymmetry: the user-to-super
direction explicitly sets D0 to the previous stack pointer
(`bdos/rwa.S:298`, commented "set return value"), but the super-to-user
direction never assigns D0 at all (no `move.l ...,d0` in `x20_sup`/`xs_0`/
`xs_usp`/`xs_exit`/`xs_scram`) - a caller returning to user mode gets no
meaningful D0, matching the classic idiom where only the entering call's
return value is ever used.

**Grouping decision:** `Super`'s contract ("flip my own privilege bit,
using this stack, give me a token to flip it back") requires exactly the
in-process EL0-to-EL1 privilege flip that ADR-Supexec (open, referenced in
`docs/api/xbios/26-supexec.md`) already identifies as unresolved for
`Supexec`. AArch64 has no per-instruction supervisor-bit flip a user
program can trigger on itself (no m68k-SR equivalent writable from EL0), so
whichever of the three candidate mechanisms ADR-Supexec picks (EL1
callback / pseudo-supervise at EL0 / kernel-side pattern translation)
determines what `Super` can mean on NewTOS too. Decision: `Super`'s NewTOS
implementation is deferred and tracked together with ADR-Supexec, not as an
independent ADR - do not implement one without revisiting the other. Until
resolved, `Super` should surface as `EINVFN` only because it has no
dispatcher slot yet (honest "not wired"), never as a fabricated
stack-pointer value, matching how `Supexec` is currently deferred.

**Not a Compendium discrepancy** - a spec-completeness gap plus a
decision-tracking grouping, not a factual disagreement with any source.

---

## D-063: `Fdelete`'s "0xE5→0x05 substitution" claim is fabricated — EmuTOS has no such convention (spec-vs-source)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/gemdos/file/41-fdelete.md`

**EmuTOS:** the classic MS-DOS convention of storing a filename that
legitimately starts with byte `0xE5` as `0x05` on disk (to avoid confusing it
with the erased-entry marker) is **not implemented anywhere** in this build.
`ixcreat()`/`builds()` (`bdos/fsopnclo.c:99-208`, `bdos/fsdir.c:737-796`) copy
and uppercase name bytes verbatim; `toupper()` (`util/string.c:171-177`) is a
plain `'a'..'z'` fold with no special-casing of `0xE5`. A full grep of
`bdos/fsdir.c`/`bdos/fsopnclo.c` finds no `0x05` substitution logic at all. A
file whose 8.3-encoded name genuinely starts with `0xE5` is therefore
indistinguishable from an erased entry on the next scan (`match()`,
`bdos/fsdir.c:1818`; `ixgetfcb()`'s end-of-directory test only checks byte
`0x00`, but `scan()`/`ixsnext()` both treat `f_name[0] == ERASE_MARKER` as
"skip/deleted").

**Prior NewTOS doc (wrong):** `41-fdelete.md` edge case 5 claimed "a name
legitimately beginning with the byte value `0xE5` is stored on disk as `0x05`
by the create path to avoid false-erasure — this is a FAT convention
preserved on-media." No such code exists in EmuTOS `1a7484ab`.

**Not a Compendium discrepancy** — an internal spec fabrication, caught the
same way the earlier `scasb` misread was: by re-reading the cited source
lines instead of trusting the existing prose. This is a real, latent EmuTOS
limitation (not a NewTOS bug) — filenames with a raw `0xE5` lead byte are rare
(the byte is outside 7-bit ASCII) but not impossible via 8-bit path input.

**Decision:** corrected `41-fdelete.md` to state plainly that no substitution
happens. NewTOS is **not required** to reproduce the collision (it is a
latent EmuTOS bug, not an observed-by-programs contract) but should decide
explicitly if it ever supports non-ASCII 8.3 names; flagged for awareness,
not for mandatory bug-compat.

---

## D-064: DND/DMD/OFD share EmuTOS's static `osmem` pool with `MD`s; exhaustion **halts the system**, not `ENSMEM` (spec-vs-source)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/structures/dnd.md`

**EmuTOS:** `xmgetblk(WORD memtype)` (`bdos/osmem.c:291-354`) is the sole
allocator for `MDBLOCK`, `DMD`, `DND`, and `OFD` requests, all carved as
fixed 64-byte blocks from one static array `osmem[LENOSM]`
(`NUM_OSM_BLOCKS = 118`, `bdos/osmem.c:36-39` — see also `docs/api/structures/md.md`).
When the free list and the raw pool are both exhausted:
- for `MEMTYPE_MDBLOCK`, `xmgetblk` returns `NULL` (`bdos/osmem.c:327-329`);
- for `MEMTYPE_DMD`/`DND`/`OFD`, it first calls `free_available_dnds()`
  (`bdos/fsdir.c:2053-2074`) to scavenge eligible DNDs tree-wide; **if that
  still doesn't yield a block**, it prints `"Out of internal memory... Use
  FOLDR100.PRG to get more.\nSystem halted!"` and calls `halt()`
  (`bdos/osmem.c:338-342`) — an unrecoverable system freeze, not a GEMDOS
  error return. This is why essentially every DND/OFD/DMD allocation site in
  `bdos/fsdir.c`/`bdos/fsopnclo.c`/`bdos/fsdrive.c` carries a comment like
  "`MGET(OFD)` only returns if it succeeds" — it is **literally true**: those
  three memory types cannot signal allocation failure to their caller.

**Impact on the FS mutation batch:** `Fopen`, `Fcreate`, `Dcreate`,
`Frename` (cross-directory), and any path resolution that must allocate a
fresh `DND` (`makdnd`) or `OFD` (`makofd`/`opnfil`) are all reachable from
this path. Under sustained internal-memory pressure (many simultaneously
open files/directories across many drives), real EmuTOS **halts the whole
machine** rather than returning `ENSMEM` from the syscall.

**Prior NewTOS docs:** `dnd.md` did not mention this at all; it instead
invented a fictitious `NDIRS = 16` "DND pool size" constant (see D-066)
that implied ordinary bounded allocation failure, not a halt.

**Not a Compendium discrepancy** — Compendium does not document internal
allocator exhaustion behaviour at all.

**Open question for the lead / STOP:** NewTOS's kernel allocator for the
Rust ports of DND/OFD/DMD-equivalent structures should almost certainly
**not** replicate a full-system halt on exhaustion — but this requires an
explicit decision (return `ENSMEM` gracefully instead, most likely paired
with the already-planned larger static arenas per PHASE-04 D5's memory
sizing notes in `docs/api/structures/md.md`). Flagging here rather than
silently assuming the "obviously better" behaviour, since it changes an
observable failure mode of the whole FS layer under load and any Phase-05
test suite asserting resource-exhaustion behaviour needs to target the
decided-upon NewTOS behaviour, not EmuTOS's halt.

---

## D-065: Disk buffer cache — 2 buffers per list (not 4), FAT list is FAT-only (not "FAT/dir"), and there is no `flush(drive)`/`flush()` API (spec-vs-source)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/gemdos/_state.md` §3, `docs/api/structures/dnd.md`

**EmuTOS:**
- `NUMBUFS = 2` (`bdos/fsbuf.c:26`, no `CONF_WITH_*` gating) — each of the
  two lists (`bufl[BI_FAT]`, `bufl[BI_DATA]`) holds exactly **2** `BCB`s, not
  4; 4 physical sector buffers exist system-wide.
- `getbcb()`'s list selector (`bdos/fsbuf.c:121`) is
  `phdr = &bufl[buftype==BT_FAT ? BI_FAT : BI_DATA]` — `bufl[BI_FAT]`
  (index 0) holds **only** `BT_FAT` buffers; `bufl[BI_DATA]` (index 1) holds
  **both** `BT_ROOT` (root-directory sectors) **and** `BT_DATA` (ordinary
  file/subdirectory sectors) buffers, combined in one 2-slot list.
- There is **no** `flush(int drv)` or no-argument `flush(void)` function.
  `void flush(BCB *b)` (`bdos/fsbuf.c:85-105`) always writes back exactly one
  buffer (plus its FAT-mirror copy on a 2-FAT medium). "Flush all dirty
  buffers, all drives" is achieved by an explicit loop at the call site —
  e.g. `ixclose()`'s full close (`bdos/fsopnclo.c:576-579`) — not by a
  dedicated flush-everything primitive. A separate, **non-flushing** cleanup
  path, `mark_bcbs_invalid(drv)` (`bdos/bdosmain.c:409-424`), runs on media
  change and **discards** (does not write back) that drive's dirty buffers,
  because the medium is assumed to be physically gone.

**Prior NewTOS doc (wrong):** `_state.md` §3 said "4 buffers per list by
default", mislabelled the lists as "`[0] = FAT/dir buffers, [1] = data
buffers`", and asserted a `flush(d)`/`flush()` two-arity API with "writes all
dirty buffers" semantics in both cases — none of which exists; the no-arg
form doesn't exist and the media-change path specifically *discards without
flushing*.

**Not a Compendium discrepancy** — internal-cache-shape spec errors.

**Decision:** corrected `_state.md` §3 and cross-referenced from `dnd.md`.
NewTOS's actual FAT driver cache size/shape is a free implementation choice
(PHASE-05); the *observable* contract worth preserving is: a real `Fclose`
flushes all dirty state across all drives (crash-consistency behaviour
programs may rely on), while a media-change event discards unwritten data
for the affected drive without an attempt to persist it.

---

## D-066: `docs/api/structures/dnd.md` had stale `BLKDEVNUM = 16` and a fabricated `NDIRS = 16` / "LRU" / "d_scan"/"m_ofl" description (spec-vs-source)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/structures/dnd.md`

**EmuTOS:** `BLKDEVNUM = 26` (`include/sysconf.h:43`) — `dnd.md`'s
"Configuration" section still said 16 even though D-025 (2026-07-03) already
corrected every drive/directory *function* spec; the shared *structures* doc
was missed in that pass. Separately: there is no `NDIRS` constant anywhere in
EmuTOS (grepped, absent) — DND/DMD/OFD allocation is unbounded-by-type,
sharing one static block pool with `MD`s (see D-064). `d_scan`
(`bdos/fs.h:223`) is written in exactly one place (`makdnd`,
`bdos/fsdir.c:1644`, zeroed on recycle) and **read nowhere** — the file's own
1986 changelog says "removed all references to d_scan" (`bdos/fsdir.c:55,60`).
`d_fill` (`bdos/fs.h:209`, `/* attributes? */`) is likewise referenced
nowhere. `DMD.m_ofl` (`bdos/fs.h:258`) is declared but never assigned or read
anywhere in `bdos/*.c` — dead, not "every OFD whose drive == this DMD, used at
Mediach-detection/unmount". Reclamation of DNDs is an eligibility sweep
(`free_available_dnds()`/`process_dnd_tree()`, `bdos/fsdir.c:1957-2074`) or a
same-parent scavenge (`makdnd()`, `bdos/fsdir.c:1610-1651`) — neither tracks
recency, so "LRU order" mischaracterised both mechanisms.

**Prior NewTOS doc (wrong):** all of the above were asserted as fact in
`dnd.md` without a matching source citation that actually supports them; none
survive a direct grep of `bdos/*.c`.

**Not a Compendium discrepancy** — internal spec fabrication /
staleness, same category as D-030/D-032 (pasted-looking prose that didn't
match a re-read of the actual source).

**Decision:** corrected in full in `dnd.md` (BLKDEVNUM, DND/DMD/OFD shared
pool + halt-on-exhaustion cross-reference to D-064, dead `d_scan`/`d_fill`/
`m_ofl` fields, eligibility-sweep-not-LRU reclamation, and the real
media-change cleanup path cross-referenced to D-065).

---

## D-067: `ckdrv`/`dcrack` discard the specific error code — `EDRIVE`/`ENSMEM` never surface from `Fopen`/`Fcreate`/`Fdelete`/`Fattrib`/`Frename`/`Dcreate`/`Ddelete`/`Fsfirst`/`Dsetpath` (cross-cutting, spec-vs-source)

**Date:** 2026-07-09
**Phase 00 specs:** cross-cutting; see `docs/api/gemdos/_state.md` §4/§5 and
every `docs/api/gemdos/{file,directory}/*.md` entry whose path resolution goes
through `findit()`.

**EmuTOS:** `findit()` (`bdos/fsdir.c:1407`) resolves the drive via
`dcrack()` (`bdos/fsdir.c:1695-1739`), which calls `ckdrv(d, TRUE)`
(`bdos/fsdrive.c:77-134`, distinguishes `EDRIVE` -46 for an invalid drive
number, `EPTHNF` -34 for an unmounted removable drive, and `ENSMEM` -39 for
`log_media()` OOM *or* a full `dirtbl[]`) but **`dcrack` discards the specific
code**: `if (ckdrv(d, TRUE) < 0) return NULL;` — any negative `ckdrv` result
collapses to a plain "path not found" signal. `findit()` then returns `NULL`
uniformly, and each caller applies its *own* generic "resolution failed"
error (`EPTHNF` for most; `EFILNF` for `Fopen`/`Fdelete`'s `ixopen`/`ixunlink`,
already noted in D-018). The practical effect: **an invalid drive letter, an
unmounted removable drive, and a `dirtbl[]`/`osmem` exhaustion are all
observationally identical** from any of these callers — none of them can
ever produce `EDRIVE` or `ENSMEM`, regardless of which of the three actually
happened. Only functions that call `ckdrv`/`Drvmap()` **directly**, bypassing
`findit`/`dcrack` — `Dgetpath` (`xgetdir`, returns `EDRIVE` directly,
`bdos/fsdir.c:1237-1240`) and `Dfree` (`xgetfree`, collapses to generic `ERR`,
already logged as D-026) — preserve any distinction at all, and even then not
the full `EDRIVE`/`ENSMEM` split.

**Prior NewTOS docs:** each individual function spec already documents its
*own* generic error return correctly (e.g. `EPTHNF` for `Fcreate`/`Dcreate`,
`EFILNF` for `Fopen`), so no single spec is factually wrong — but the
*reason* (this shared `dcrack`/`ckdrv` collapse) was previously stated only
for `Dsetpath` (`_state.md` §5, the `dirtbl[]`-full case). It was not
connected as a general fact spanning `Fopen`/`Fcreate`/`Fdelete`/`Fattrib`/
`Frename`/`Dcreate`/`Ddelete`/`Fsfirst` as well.

**Decision:** documented here as a cross-cutting fact rather than editing all
nine affected per-function specs individually (time-boxed; flagged in the
researcher report). No spec correction needed *per function* — each already
states the right observable error — but any NewTOS test suite probing
"invalid drive letter" or "directory-cache exhaustion" behaviour for these
nine functions must expect the *generic* path-resolution error, never
`EDRIVE`/`ENSMEM`, and should cite this entry rather than re-deriving it.

---

## D-068: `docs/api/structures/ofd.md`'s `DFD` was documented as 16 bytes with a fabricated 2-byte pad; actual size is 14 bytes (spec-vs-source)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/structures/ofd.md`

**EmuTOS:** `DFD` (`bdos/fs.h:116-124`) is `{ UWORD o_flag; WORD o_usecnt;
DOSTIME o_td; CLNO o_strtcl; long o_fileln; }` = 2+2+4+2+4 = **14 bytes**, with
every field already 2-byte-aligned and nothing declared after `o_fileln` —
there is no padding to 16 bytes anywhere in the source or its usage (the
tail three fields, 10 bytes, are `memcpy`'d as a block against the matching
FCB tail in `ixclose`, `bdos/fsopnclo.c:538`, which only works if there is
**no** gap). With the corrected 14-byte `DFD`, `sizeof(OFD)` (embedding
`DFD o_disk` as its last member) is confirmed at **56 bytes (0x38)** — the
number the doc's own byte-offset table already implied field-by-field; only
the `DFD` sub-figure (16, with an invented "2 bytes pad") was wrong.

**Prior NewTOS doc (wrong):** `ofd.md`'s `DFD` section stated 16 bytes with
"2 bytes pad to 16 in FCB-compatible layout" — no such pad exists.

**Not a Compendium discrepancy** — internal struct-arithmetic error, same
category as D-030/D-032/D-066 (asserted numbers not re-derived from the
actual field list).

**Decision:** corrected `DFD` to 14 bytes in `ofd.md`; `OFD`'s 56-byte total
was already right and is unchanged, now with the arithmetic shown explicitly
so it doesn't drift again.

---

## D-069: `docs/api/structures/dta.md` lifecycle note said `EFILNF` where the source returns `ENMFIL` for `Fsnext` exhaustion (self-consistency fix)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/structures/dta.md`

**EmuTOS:** `xsnext()` (`bdos/fsdir.c:654-676`) returns `ENMFIL` (-49) for
every exhausted/uninitialised case (already established as D-019). `dta.md`'s
own "Lifecycle / invariants" list item 4 said "After `EFILNF` (no more
matches)…", contradicting item 5 two lines below (which correctly says
`ENMFIL`) and contradicting D-019/the `4f-fsnext.md` spec it cross-references.

**Not a Compendium discrepancy** — an internal self-consistency slip (`EFILNF`
is `Fsfirst`'s "no match at all" code, `ENMFIL` is `Fsnext`'s "no more
matches" code; the two were conflated in one sentence).

**Decision:** corrected item 4 in `dta.md` to say `ENMFIL`.

---

## D-070: `Frename`'s cycle-prevention check (`is_subdir`) only catches a one-level self-nesting move, not deeper ones (source-level EmuTOS gap, not a spec-reading error)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/gemdos/file/56-frename.md`

**EmuTOS:** `xrename()`'s cross-directory cycle guard is
`if (is_subdir(s1,dn1,dn2)) return EACCDN;` (`bdos/fsdir.c:1002-1003`).
`is_subdir()`'s own doc comment states its scope precisely: "check if
directory 2 is an **immediate** subdirectory of directory 1"
(`bdos/fsdir.c:844-863`) — it tests only `dn2->d_parent == dn1 && dn2's name
== s1`, i.e. "is the target's parent directory *exactly* the folder being
moved". This catches the one-level case
(`Frename(0,"C:\A","C:\A\NEW")` — moving `A` to nest directly inside
itself) but **does not walk further up the ancestor chain**. A move like
`Frename(0,"C:\A","C:\A\SUB\NEW")`, where `SUB` is a subdirectory of `A`,
passes the check (`dn2` = the DND for `C:\A\SUB` has `d_parent` = `A`'s DND,
not the root `dn1`, so `is_subdir` returns `FALSE`) and **proceeds**: the
directory entry for `A` is relocated to live inside `A\SUB`, one of its own
descendants — a genuine cycle in the on-disk/in-memory directory tree. Any
future recursive tree walker (`Dgetpath`'s `dopath()` already recurses via
`d_parent`; a CHKDSK-style scan would recurse via `d_left`/`d_right`) would
not terminate against such a structure.

**This is a source-level EmuTOS limitation, not a misreading of the spec** —
confirmed by `is_subdir`'s own comment stating "immediate" as its documented
scope, and by tracing the exact DND-parent comparison for a concrete
multi-level example. It is the closest analogue in this codebase to the
"the code IS the bug" class of finding (cf. the historical `scasb` misread,
which was a *documentation* error; this one is a genuine *gap in EmuTOS's
own defensive check*).

**Prior NewTOS doc:** `56-frename.md` described the one-level case
correctly ("prevents cycles… e.g. `a\b -> a\b\c`") but did not note that the
check's scope stops there.

**Decision:** documented as edge case 6a and a new error-table footnote in
`56-frename.md`. **Open question for the lead:** does NewTOS's `Frename`
implementation widen this check to walk the full ancestor chain (a
deliberate safety improvement over EmuTOS, analogous to the D-023
open-file-rename decision), or intentionally reproduce the gap for
m68k-binary bug-compat? Given this batch's explicit focus on the historical
`Frename` corruption class, the researcher's recommendation is to **widen
the check** — but this is a decision for the Developer/lead, not something
to silently implement here.

---

## D-071: `docs/api/structures/basepage.md` also missed the D-025 `BLKDEVNUM` fix (16 → 26)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/structures/basepage.md`

**EmuTOS:** `BLKDEVNUM = 26` (`include/sysconf.h:43`), a plain unconditional
`#define` — not gated by `CONF_WITH_TT_SHIFTER` or any other build flag.
`NUMCURDIR = BLKDEVNUM` (`include/bdosdefs.h:62`), so `PD.p_curdir[26]` +
`p_2fill[6]` fill the fixed 32-byte region at basepage offset 0x40-0x60
(`include/bdosdefs.h:89-90`).

**Prior NewTOS doc (wrong):** `basepage.md` still said `BLKDEVNUM = 16`
("16 in default cfg", "uses 16 of the 32 reserved bytes", and "16 with
default `CONF_WITH_TT_SHIFTER`" — the last also fabricating a conditional
that does not exist in the source) in three places. D-025 (2026-07-03)
corrected every drive/directory *function* spec and `_state.md` §4/§5 for
this same fact, but this particular structures file was missed in that pass
— the same gap D-066 found in `dnd.md`.

**Not a Compendium discrepancy** — internal spec staleness, same root cause
as D-025/D-066 (a shared fact that needed propagating to every file that
restates it, and one file was missed each time).

**Decision:** corrected all three occurrences in `basepage.md`. **Process
note for future passes:** `BLKDEVNUM`/`NUMCURDIR` values are restated in at
least five separate files (`_state.md`, the drive/directory function specs,
`dnd.md`, `basepage.md`); a grep for `BLKDEVNUM = 16\|A:\.\.P:\|NDIRS = 16`
across all of `docs/api/` is a cheap way to catch a future regression or a
missed file in one pass.

---

## D-072: `Ssystem(-1, …)` "does this call exist" probe confirmed `E_OK`; NewTOS `ssystem.rs` has no arm for it (spec correct, implementation gap)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/gemdos/154-ssystem.md`
**Source:** FreeMiNT @ `f906416` (EmuTOS has no `Ssystem`).

**FreeMiNT:** `sys_s_system` pre-initializes `register long r = E_OK;`
(`sys/ssystem.c:66`) before the `switch (mode)` (`sys/ssystem.c:70`).
`case -1:` (`sys/ssystem.c:72-76`) does only a `TRACE()` and `break`s — it
never assigns `r`. Execution falls through to `return r;`
(`sys/ssystem.c:651-652`) with `r` still `0`. So `Ssystem(-1, arg1, arg2)`
returns `E_OK` (0) **unconditionally**, independent of `arg1`/`arg2`; every
*other* unrecognized mode instead falls into `default:` and gets `ENOSYS`
(`sys/ssystem.c:642-648`).

**Spec status:** `154-ssystem.md:82-83` (pre-audit line numbers) already
stated this correctly ("`mode == -1` … returns `E_OK`"), and the
verification criterion "Probe: `Ssystem(-1, 0, 0)` returns `E_OK` (0)"
already tested it. **No spec correction needed** — the lead's audit
suspected the opposite (that `-1` would fall into an unhandled/`EINVFN`
path), so this entry exists to record the line-by-line source confirmation
and to log the actual implementation gap the audit uncovered.

**Implementation gap (not a spec bug):**
`tos-api/src/gemdos/ssystem.rs:25-32` (`ssystem()`) matches only
`S_GETCOOKIE`(8)/`S_SETCOOKIE`(9)/`S_DELCOOKIE`(26) on `args.arg_u16(0)`;
every other value — including `-1` (`0xFFFF` as `u16`, i.e.
`args.arg_i16(0) == -1`) — falls to the wildcard arm and returns `EINVFN`.
A MiNT-aware caller probing for `Ssystem` support before touching the cookie
jar sees "not implemented" and falls back to a `$5A0`/`Supexec` path that
ADR-012 removed — cookie access then fails outright.

**Fix (for the Developer, exact match arm):**
```rust
pub fn ssystem(args: &TrapArgs) -> Long {
    if args.arg_i16(0) == -1 {
        // "does this call exist" probe (sys/ssystem.c:66,70-76): `r` is
        // pre-set to E_OK and case -1 never touches it.
        return E_OK;
    }
    match args.arg_u16(0) {
        S_GETCOOKIE => s_getcookie(args),
        S_SETCOOKIE => s_setcookie(args),
        S_DELCOOKIE => s_delcookie(args),
        _ => EINVFN,
    }
}
```

**Decision:** confirmed against source; the fix is scoped to `ssystem.rs`
only (no `cookie.rs` change, no further spec change beyond the two
clarifying notes added to `154-ssystem.md`).

---

## D-073: `Ssystem` cookie error codes must be FreeMiNT-verbatim (`EINVAL`=-25, not `ERROR`/`EINVFN`); `S_GETCOOKIE(0)` returns jar **capacity**, not occupied count (open-question resolution + implementation gap)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/gemdos/154-ssystem.md`
**Source:** FreeMiNT @ `f906416`, `sys/cookie.c`, `sys/mint/errno.h`.

**FreeMiNT error values** (`sys/mint/errno.h:52,85,241,265,318,322`):
`E_OK`=0, `EERROR`=-1, `EINVAL`=-25, `ENOSYS`=-32, `EPERM`=-38, `ENOMEM`=-39.

**Confirmed per-path codes** (re-derivation of D-038, now cross-checked
against the live `errno.h` values):
- `get_cookie`, tag search miss → `EERROR` (-1) (`sys/cookie.c:508-509`).
- `get_cookie`, slot-index too big → `EINVAL` (-25) (`sys/cookie.c:458-462`).
- `set_cookie`, invalid tag (a zero byte) → `EINVAL` (-25)
  (`sys/cookie.c:538-541`).
- `set_cookie`, jar full → `ENOMEM` (-39) (`sys/cookie.c:574-575`).
- `del_cookie`, tag not found → `EINVAL` (-25) (`sys/cookie.c:606-607`).

**`S_GETCOOKIE` with `arg1 == 0` (capacity query):** `get_cookie` scans to
the terminator (`cjar->tag == 0`) and returns `cjar->value`
(`sys/cookie.c:422-442`). That value is the jar's **fixed total slot
capacity**, set once in `init_cookies()`
(`ncsize / sizeof(struct cookie)`, `sys/cookie.c:322-326`) and **preserved
verbatim** on every successful `set_cookie` append: `set_cookie` copies the
*same* `n = cjar->value` (the capacity) into the new terminator
(`sys/cookie.c:557-566`) — it never decrements a "remaining free slots"
counter, and it never becomes the count of *occupied* slots.

**Spec status:** `154-ssystem.md` was already correct on the capacity
question (`S_GETCOOKIE (mode 8)` §, "the NULL-slot value = the jar's total
slot capacity"; verification criterion "Get capacity"). **Open Question 1
(error-code mapping) is now RESOLVED and its "open question" marking
removed** in `154-ssystem.md`: NewTOS's `Ssystem` cookie subset returns the
FreeMiNT values verbatim (`E_OK`/`EERROR`/`EINVAL`/`EPERM`/`ENOMEM` as
above); no numeric collision exists within this subset against classical
`gemerror.h` codes (`-25` and `-38` are unused slots in the TOS table). This
follows `docs/COMPATIBILITY.md`'s contract/internal split: a MiNT-aware
caller's `Ssystem` return value is observable behaviour and therefore
contract, not a free NewTOS internal.

**Implementation gap (not a spec bug):**
- `tos_types::error` has no `EINVAL`/`EPERM` equivalents (only `ERROR`=-1
  and `ENSMEM`=-39 exist, which numerically match `EERROR`/`ENOMEM`;
  `EINVAL`=-25 and `EPERM`=-38 are missing entirely).
- `s_getcookie`'s slot-index-too-big and `s_delcookie`'s not-found both
  currently return `ERROR` (`-1`) instead of `EINVAL` (`-25`);
  `s_setcookie`'s invalid-tag path returns `EINVFN` (`-32`) instead of
  `EINVAL` (`-25`) (`tos-api/src/gemdos/ssystem.rs:39-86`). Jar-full already
  returns `ENSMEM` (`-39`), which is correct (numerically == FreeMiNT
  `ENOMEM`) — no change needed there. `s_getcookie`'s tag-search-miss path
  already returns `ERROR` (`-1`), which is correct (== FreeMiNT `EERROR`) —
  no change needed there either.
- `cookie::count()` (`tos-api/src/cookie.rs:101-106`, occupied-slot count)
  is used by `s_getcookie`'s `tag == 0` branch
  (`tos-api/src/gemdos/ssystem.rs:44`), where FreeMiNT wants the jar's fixed
  **capacity**, not the occupied count. With the current 4-cookie boot seed
  and `JAR_CAP = 32`, `Ssystem(S_GETCOOKIE, 0, 0)` today returns `4` instead
  of `32` — a MiNT program pre-flighting jar headroom before `S_SETCOOKIE`
  undercounts available slots by a wide margin.

**Fix (for the Developer):**
1. Add to `tos_types::error` (values confirmed above, FreeMiNT
   `sys/mint/errno.h:241,318`):
   ```rust
   pub const EINVAL: Long = -25; // FreeMiNT / MiNT: invalid argument
   pub const EPERM:  Long = -38; // FreeMiNT / MiNT: operation not permitted
   ```
2. In `tos-api/src/cookie.rs`, add a capacity accessor distinct from the
   existing occupied-count `count()`:
   ```rust
   /// Total slot capacity of the jar — what FreeMiNT stores in the
   /// terminator slot (`sys/cookie.c:322-326`) and returns for
   /// `S_GETCOOKIE(tag == 0)`. NOT the occupied count; see `count()`.
   pub fn capacity() -> u32 {
       JAR_CAP as u32
   }
   ```
3. In `tos-api/src/gemdos/ssystem.rs`:
   - `s_getcookie`'s `tag == 0` branch: use `cookie::capacity()` instead of
     `cookie::count()`.
   - `s_getcookie`'s slot-index branch (`tag & 0xFFFF_0000 == 0`): `None`
     (index out of range) → `EINVAL`, not `ERROR`.
   - `s_getcookie`'s tag-search branch (the `else` arm): `None` (tag not
     found) → stays `ERROR` (already correct, matches FreeMiNT `EERROR`).
   - `s_setcookie`: invalid-tag → `EINVAL`, not `EINVFN`. Jar-full stays
     `ENSMEM` (already correct).
   - `s_delcookie`: not-found → `EINVAL`, not `ERROR`.

**Decision:** confirmed against source; Open Question 1 in
`154-ssystem.md` is resolved and its marking removed (see spec edit
referenced above). No change to the root-gate logic (`EPERM` on
`S_SETCOOKIE`/`S_DELCOOKIE` for non-root callers) is part of this entry —
`ssystem.rs`'s current code has no privilege check at all for those two
modes, which is a **separate**, pre-existing gap (FreeMiNT
`sys/ssystem.c:285-289,295-299`) outside the two questions this audit
scoped; flagged for the lead, not fixed here.

---

## D-074: `Tsettime` (0x2D) does not validate the hour field; the spec's own "Pi/BCM2711 binding" section contradicted its correctly-sourced main body (self-contradiction, implementation gap)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/gemdos/time/2d-tsettime.md`
**Source:** EmuTOS @ `1a7484ab`, `bdos/time.c:142-153` (`xsettime`).

**EmuTOS validates all three fields**, not two:
```c
long xsettime(UWORD t)
{
    if (((t & SEC_BM) >= 30)                /* 30 "double-seconds" per minute */
     || ((t & MIN_BM) >= (60 << MIN_SHIFT)) /* 60 minutes per hour */
     || ((t & HRS_BM) >= (24 << HRS_SHIFT)))/* 24 hours per day */
        return ERR;
    ...
```
(`bdos/time.c:144-146`). `HRS_BM = 0xF800`, `HRS_SHIFT = 11`; the third
condition rejects any packed hour field ≥ 24 (values 24-31 are representable
in the 5-bit field but invalid). This third check exists in the source and
was already correctly captured in `2d-tsettime.md`'s main body — the
"Error conditions" table (`Hour field ≥ 24 → ERR (-1)`, citing
`bdos/time.c:146`) and Edge case 1 both stated it correctly.

**Spec self-contradiction found on re-validation:** the same file's trailing
"Pi / BCM2711 binding" section (added in a later pass, not cross-checked
against the main body) asserted the *opposite*: "hour unvalidated per
EmuTOS" / "hour not validated — EmuTOS parity". This is the "spec itself may
be wrong" case the task called out — not a misreading of EmuTOS (the main
body read it right), but a stale/wrong claim introduced in a later section
of the *same document* that was never reconciled against the earlier,
source-verified section. **Corrected** (this pass) — see the edit to
`2d-tsettime.md`.

**Implementation gap (not a spec-reading error):**
`tos-api/src/gemdos/time.rs:174-188` (`tsettime`) matches the *wrong* claim,
not the source:
```rust
pub fn tsettime(args: &TrapArgs) -> Long {
    let t = args.arg_u16(0);
    let hour = (t >> 11) as u32; // not validated (EmuTOS parity)
    let min = ((t >> 5) & 0x3F) as u32;
    let dsec = (t & 0x1F) as u32; // second / 2

    if dsec >= 30 || min >= 60 {
        return ERROR;
    }
    ...
```
There is no `hour >= 24` check, so `Tsettime` currently accepts hour field
values 24-31 (e.g. `Tsettime(31 << 11)` → `E_OK`, silently producing a
wall-clock time one or more days in the future via `compose()`'s hour
arithmetic) where EmuTOS returns `ERR` (-1) and leaves the clock unchanged.
The doc-comment above `tsettime` (`time.rs:170-173`) repeats the same wrong
"EmuTOS parity" claim and needs updating alongside the code fix.

The sibling `xbios_settime` (`time.rs:203-214`, the `Settime`/XBIOS-0x16
path) is **not** affected — `Settime`/`settime()` genuinely performs no
field validation in EmuTOS (`bios/clock.c:1218-1264`), so "hour/min/sec pass
through unvalidated" is correct there. The bug is scoped to the GEMDOS
`Tsettime` entry point only, which EmuTOS's `xsettime()` gates before
calling `Settime`.

**Fix (for the Developer, exact match to `bdos/time.c:144-146`):**
```rust
pub fn tsettime(args: &TrapArgs) -> Long {
    let t = args.arg_u16(0);
    let hour = (t >> 11) as u32;
    let min = ((t >> 5) & 0x3F) as u32;
    let dsec = (t & 0x1F) as u32; // second / 2

    if dsec >= 30 || min >= 60 || hour >= 24 {
        return ERROR;
    }
    ...
```
Update the doc-comment on `tsettime` (`time.rs:170-173`) to drop "the hour
field is not validated, matching EmuTOS" and state the three-field check
instead.

**Decision:** confirmed against source (`bdos/time.c:142-153`); the spec's
main body was already right, only the later "Pi/BCM2711 binding" addendum in
the same file was wrong and has been corrected. This is an implementation
bug in `tos-api/src/gemdos/time.rs`, flagged for the Developer — Researcher
does not edit `.rs` files.

---

## D-075 (OPEN QUESTION, not a fix): XBIOS `Gettime` (0x17) upper-32-bits-of-`x0` convention — spec/global-rule say sign-extend, code zero-extends via `u32 as i64`

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/xbios/17-gettime.md`
**Source:** N/A for the extension question itself — m68k has no 64-bit
registers, so EmuTOS cannot answer this; it is a **NewTOS-only native-ABI
design question** surfaced while cross-checking XBIOS `Settime`/`Gettime`
against the GEMDOS time-group specs (as instructed).

**The inconsistency, three places disagree:**
1. **Global convention** (`tos-api/src/trap.rs:6`): "return value in `x0`
   (sign-extended `i64`)" — stated as the universal rule for all native
   trap returns.
2. **Per-function spec** (`docs/api/xbios/17-gettime.md:68`): "the LONG
   result **sign-extended** into `x0` (a packed 32-bit datetime)" — matches
   the global rule.
3. **Implementation** (`tos-api/src/xbios/mod.rs:94-99`):
   ```rust
   // Clock group: Settime/Gettime read/write the same wall clock as
   // GEMDOS Tgetdate/Tgettime (no separate RTC on NewTOS). Settime
   // returns void → 0; Gettime returns the packed LONG (zero-extended).
   ...
   GETTIME => crate::gemdos::time::xbios_gettime() as i64,
   ```
   `xbios_gettime() -> u32`; Rust's `as i64` cast from an unsigned source
   type always **zero**-extends (regardless of destination signedness), so
   the actual code contradicts both (1) and (2) — and its own inline
   comment asserts the zero-extension is intentional, without updating (1)
   or (2) to match.

**Why it is currently latent, not observed:** EmuTOS `gettime()`
(`bios/clock.c:1266`) returns `MAKE_ULONG(date, time)` — a genuine
combined 32-bit value (date in the high word, time in the low word), whose
bit 31 is the date word's bit 15 = the year-offset field's MSB, set only
when the year offset ≥ 64 (calendar year ≥ 2044). At the current NewTOS
default epoch (booting 1980, or any date before 2044) bit 31 is always 0,
so sign-extension and zero-extension of the upper 32 bits of `x0` produce
an *identical* result — the divergence has no observable effect until the
wall clock is set to (or advances to) 2044 or later.

**Contrast with GEMDOS `Tgetdate`/`Tgettime` (0x2A/0x2C), which are NOT
inconsistent:** those C functions return a `UWORD` implicitly promoted to
`long` (`bdos/time.c:99,132`), so the C-level 32-bit value already has its
upper 16 bits zero and bit 31 is *always* 0 — "zero-extended" and
"sign-extended" are mathematically identical there, and both this file's
main body and the NewTOS binding section say "zero-extended" consistently.
`Gettime`'s packed value has no such guarantee (both halves are meaningful,
combined via `MAKE_ULONG`, not a single small type promoted).

**This is a STOP, not a fix — flagged for the lead:** resolving it requires
an architectural decision on the general x0-upper-bits ABI convention, not
just a source re-read:
- **Option A:** the global rule (`trap.rs:6`) is right; fix
  `tos-api/src/xbios/mod.rs:99` to genuinely sign-extend
  (`(crate::gemdos::time::xbios_gettime() as i32) as i64`) and correct its
  comment. `17-gettime.md` stays as-is.
- **Option B:** "packed bit-pattern LONG, not a signed count/error code"
  values are meant to be a *documented exception* to the global
  sign-extend default (as pointers already are, per
  `docs/api/xbios/_dispatcher.md:203`, "pointer full-64-bit"); then the
  code is already correct, and `17-gettime.md:68` needs correcting from
  "sign-extended" to "zero-extended" (and the general rule in `trap.rs`/
  `_dispatcher.md` should gain an explicit exception clause covering
  packed-bitfield LONGs).

Whether the upper 32 bits of `x0` are even **observable by a caller**
(contract question) — i.e. whether native NewTOS/AArch64 or the future
m68k-emulator marshaling shim ever exposes more than the low 32 bits
(`w0`) to TOS-API-facing code — is itself unresolved in the read
architecture docs and bears on whether this is a contract bug or a
harmless kernel-internal inconsistency. Researcher does not decide ABI
design or edit `.rs`; recorded here for the lead/Developer to resolve.

---

## D-076: `Getrez` NewTOS binding published a mono code (`2`/`ST_HIGH`) for a 16-colour device — corrected to `4`/`TT_MEDIUM` (resolution-policy fix)

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/xbios/04-getrez.md`, `docs/HARDWARE-PROFILE.md` §3.1
**Decision:** ADR-017 (`docs/MEMORY.md`)

**EmuTOS:** `getrez()`/`atari_getrez()` (`bios/screen.c:1076-1084,948-988`)
reads the live Shifter/Videl mode and reports the matching Atari code; the
per-code geometry+plane table is `vmode_table[]` (`bios/screen.c:724-735`):
code `2` (`ST_HIGH`) = `{planes:1, hz_rez:640, vt_rez:400}` (**mono**), code
`4` (`TT_MEDIUM`) = `{planes:4, hz_rez:640, vt_rez:480}` (**16 colours**).

**Prior NewTOS binding (wrong, self-flagged as a shim, now confirmed a
defect):** `docs/api/xbios/04-getrez.md` and `docs/HARDWARE-PROFILE.md` §3.1
both documented (and the code at `tos-api/src/xbios/screen.rs:33,68`
implements) `Getrez() == 2` ("`ST_HIGH`, the safe legacy hint") for a
framebuffer the same codebase's VDI reports as 640×480×16-colour
(`DEV_TAB[0,1]=639,479`, `DEV_TAB[13]=16`, `INQ_TAB[4]=4` planes,
`tos-api/src/vdi/workstation.rs:100-106,143-160`). Two subsystems of the
same running system disagreed about the display's colour depth — `2` claims
monochrome, the VDI tables claim 16 colours/4 planes. This was already
flagged twice as a "Step-2 tension" in both specs but not resolved.

**Not a Compendium/Profibuch discrepancy** — this is a NewTOS-internal
binding defect (the value chosen for the fixed Pi-4 mode contradicted the
device's own reported capabilities), caught during the resolution-policy
review requested by the user.

**Decision:** `Getrez` truthfully reports **`4` (`TT_MEDIUM`)** — the exact
Atari code whose `vmode_table` entry matches NewTOS's live VDI geometry and
colour count. Corrected in `docs/api/xbios/04-getrez.md` (Pi/BCM2711
binding) and `docs/HARDWARE-PROFILE.md` §3.1. See ADR-017 for the full
resolution policy this is part of (a fixed single mode, no runtime switch).
Implementation change (constant/shadow return value) is the Developer's;
Researcher does not edit `.rs`.

---

## D-077: `Setscreen`'s NewTOS `rez` range (`[0,2]`) was narrower than EmuTOS's own sanity bound (`[0,7]`); resolved by the single-mode policy, not by inventing a NewTOS-specific range

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/xbios/05-setscreen.md`
**Decision:** ADR-017 (`docs/MEMORY.md`)

**EmuTOS:** `setscreen()` (`bios/screen.c:1102-1166`) validates `rez` only
against the flat range `MIN_REZ=ST_LOW=0` .. `MAX_REZ=TT_LOW=7`
(`include/biosdefs.h:132-133`; check at `screen.c:1120-1121`) — a generic
sanity bound, **not** a per-hardware capability check. Whether a validated
`rez` is actually achievable on the connected hardware is decided one layer
down in `atari_setrez`/`videl_setrez`/`vsetmode` (`screen.c:990-1013`,
`bios/videl.c:619-659,976-993`), and in every branch examined, an in-range
but hardware-unsupported `rez` is **silently ignored** (current mode kept,
no error) rather than rejected:
- TT-shifter: `atari_setrez` writes the shifter only `if ((rez != 3) &&
  (rez != 5))` (`screen.c:1006-1007`) — codes 3 (Falcon-only) and 5
  (unused) are silently skipped on TT hardware.
- Plain ST/STe: the final `else if (rez < 3)` branch (`screen.c:1010-1012`)
  does nothing for TT-only codes 4-7.
- Videl: `vsetmode()` itself returns the **unchanged** `current_video_mode`
  if the fixed-up mode is invalid for the monitor (`bios/videl.c:636-637`,
  `set_videl_vga(mode) < 0`), and that return value is discarded by the
  (void) caller chain — never surfaced as a `Setscreen` error.

**Prior NewTOS implementation (found narrower than the source, flagged by
the earlier XBIOS audit):** `tos-api/src/xbios/screen.rs:34-35` defines
`MIN_REZ=0, MAX_REZ=2` — a NewTOS-specific range with no basis in the
EmuTOS source's own flat bound, and any `rez` in `{3..7}` (including the
true live mode `TT_MEDIUM=4`!) was **rejected with `-1`** instead of being
handled per the "valid code, unsupported hardware → silent no-op" precedent
above.

**Not a Compendium/Profibuch discrepancy** — a NewTOS-internal range chosen
too narrowly, caught during the resolution-policy review.

**Decision (ADR-017):** widen the `Setscreen` `rez` validation to EmuTOS's
real bound `[0,7]`; `rez == 4` (`TT_MEDIUM`) is accepted (a no-op, already
live); every other in-range code (`0,1,2,3,5,6,7`) is accepted (returns `0`)
but produces **no** state/mode change — mirroring EmuTOS's own "hardware
can't do this rez" no-op precedent rather than inventing new NewTOS
behaviour. Out-of-range (`rez<0 || rez>7`) still returns `-1`. Corrected in
`docs/api/xbios/05-setscreen.md` (Pi/BCM2711 binding, exact match arms
given for the Developer). Implementation change is the Developer's;
Researcher does not edit `.rs`.

---

## D-078: D-076/D-077 reopened — ADR-017's single-fixed-640×480 policy was superseded same-day before it was ever committed

**Date:** 2026-07-09
**Phase 00 spec:** `docs/api/xbios/04-getrez.md`, `docs/api/xbios/05-setscreen.md`
**Decision:** ADR-017 (`docs/MEMORY.md`, revised 2026-07-09)

**Context:** D-076 and D-077 recorded corrections for a draft of ADR-017
("NewTOS presents exactly one VDI resolution, 640×480×16 =
`TT_MEDIUM`") that was drafted and never committed the same day. The user
overrode that direction before it shipped: NewTOS now targets a **native,
geometry-independent** resolution — default 1920×1080, switchable at
runtime to 1280×800 — not a single fixed 640×480 canvas (full decision +
source analysis: `docs/MEMORY.md` ADR-017, revised text).

**This entry does not retract D-076/D-077** (the log is append-only) — their
EmuTOS source analysis (the `atari_setrez`/`videl_setrez`/`vsetmode`
"silently ignore an unsupported `rez`" precedent, and the Falcon
`atari_getrez`/`sshiftmod` disagreement for off-table modes) remains
factually correct and is still cited by the revised ADR-017 and the updated
`04-getrez.md`/`05-setscreen.md`. What changes is the **policy conclusion**
built on top of it:

- D-076's "`Getrez` must return `4` (`TT_MEDIUM`), unconditionally" is
  **reopened**: neither 1920×1080 nor 1280×800 (16 colours) is a
  `vmode_table[]` entry, so `4` is no longer a truthful match for either
  live mode. What `Getrez` should return instead is an **open Lead
  decision** — see `docs/api/xbios/04-getrez.md` (Pi/BCM2711 binding) for
  three sketched (undecided) options.
- D-077's "`Setscreen` range widens to `[0,7]`, only `rez==4` applies, every
  other in-range code is a no-op" is **reopened** in the same way: the
  match arms assumed the single-fixed-mode policy. Whether `Setscreen`
  should drive the real 1920×1080⇄1280×800 switch, and which (if any)
  classic `rez` codes apply, is the same open Lead decision — see
  `docs/api/xbios/05-setscreen.md` (Pi/BCM2711 binding).

**Not a Compendium/Profibuch discrepancy** — a NewTOS-internal policy
reopened by a same-day user course-correction, caught before either draft
value shipped as contract.

**Decision:** no value is chosen by this entry. `docs/api/xbios/
04-getrez.md`/`05-setscreen.md` (Pi/BCM2711 binding sections) are updated to
mark the return value / match arms as open pending the Lead's choice among
the sketched options. Implementation is blocked on that choice; Researcher
does not decide it unilaterally (CLAUDE.md Rule 3: stop on uncertainty).

---

## D-079: VDI dispatcher (trap #2 / `screen()`) — "no-op" convention
re-validated, confirmed; a third distinct convention alongside GEMDOS
`EINVFN` and BIOS/XBIOS's function-number echo

**Date:** 2026-07-09
**Phase 00b spec:** `docs/api/vdi/_dispatcher.md`

**Context:** first task of the VDI leg of the bottom-up sweep (GEMDOS,
BIOS, XBIOS already done). Re-read `vdi/vdi_main.c:121-192`
(`screen()`), `vdi/vdi_control.c:178-184` (`get_vwk_by_handle`),
`vdi/vdi_asm.S` (`_vditrap`/`_GSX_ENTRY`), `aes/gemdosif.S:222-304`
(`_aestrap`/`set_aestrap`), and `bios/bios.c:173` (`VEC_GEM = vditrap`)
against the existing `docs/api/vdi/_dispatcher.md`.

**Finding — confirmed, no source-behaviour correction:** `_dispatcher.md`
already stated correctly that VDI has no error return and that an
unknown/invalid opcode is a silent no-op. This holds for **every**
failure path in `screen()`:
- an out-of-range or empty-slot handle (`get_vwk_by_handle()` returns
  `NULL`) → bare `return;` before the output-count preload, `CONTRL[2]/
  [4]` left at the `0/0` set at entry;
- an opcode outside both jump-table ranges (`[1,40)` ∪ `[100,100+
  JMPTB2_ENTRIES)`) → same bare `return;`, same `0/0`;
- an opcode that *is* in-range but mapped to a "does not exist"/"not
  implemented" slot (4, 10, 27, 29, 34, and 132/133 without
  `CONF_WITH_EXTENDED_MOUSE`) → the jump-table entry `{0,0,vdi_v_nop}`
  runs the output-count preload (itself `0/0`) then calls `vdi_v_nop`
  (`just_rts`), which also does nothing observable.

No path writes a `D0` error code, a function-number echo, or an
`INTOUT`/`PTSOUT` sentinel; a caller cannot distinguish these three cases
from the `VDIPB` alone.

**Contrast (the point of this entry — three traps, three conventions):**
- GEMDOS (trap #1): unknown/unimplemented `fn` → **`EINVFN` (-32)**,
  always (`bdos/bdosmain.c:444-445` out-of-range; `ni()` at
  `bdos/bdosmain.c:307-310` for in-range-unimplemented) — see
  `docs/api/gemdos/_dispatcher.md` §4.
- BIOS (trap #13) / XBIOS (trap #14): unknown/unimplemented ID → returns
  the **function number itself** in `D0`, both paths
  (`bios/vectors.S:1323-1326`; `bios/xbios.c:1251`) — not an error code;
  see `docs/api/xbios/_dispatcher.md` §3 (D-008/D-039) and
  `docs/api/bios/_dispatcher.md` §4.
- VDI (trap #2, selector `0x73`): unknown/unimplemented opcode → **no
  signal whatsoever** — a bare `return;`, no D0 code, no echoed number, no
  output-array marker.

Each convention is internally self-consistent within its own trap; per
the same caution already recorded for the GEMDOS/BIOS-XBIOS pair, none
should be "fixed" toward another by analogy.

**Doc-precision fix (not a behaviour correction):** `_dispatcher.md`'s
trap-chain description previously read "`_vditrap` … passes everything
else through", which could be misread as `_vditrap` itself forwarding
unrecognised calls onward. Re-verified: `_vditrap` never forwards
anywhere (`vdi_asm.S:40-48`, falls through to a bare `rte` for any `d0`
other than `0x73`/`0xffff`). The forwarding is one layer *above* it and
only while the AES is running: `_aestrap` (installed by `set_aestrap()`
over the previously-installed `_vditrap`, `aes/gemdosif.S:295-304`)
recognises its own `0xC8`/`0xC9` selectors and, for anything else,
tail-calls the saved `savetrap2` (= `_vditrap`) via `aes/gemdosif.S:
222-228`. Corrected in `docs/api/vdi/_dispatcher.md`.

**Decision:** no value changes; `docs/api/vdi/_dispatcher.md` is
confirmed accurate at the "no-op" claim and updated only for the
trap-chain-topology precision fix and this three-way-contrast writeup.

---

## D-080: `flip_y` (the VDI trap's `d0` return value) is silently set to `1` by four attribute setters (`vsm_height`, `vst_height`, `vst_point`, and — as a leaked side effect — `vst_font`), undocumented in the Batch-2 attribute specs

**Date:** 2026-07-10
**Phase 00b spec:** `docs/api/vdi/attributes/_line-marker.md`,
`docs/api/vdi/attributes/_text.md`

**Context:** re-validating the VDI attribute-setter specs
(`_line-marker.md`, `_fill.md`, `_text.md`, `_mode-clip-color.md`)
against `vdi/vdi_line.c`, `vdi_marker.c`, `vdi_fill.c`, `vdi_text.c`,
`vdi_control.c`, `vdi_col.c` at `1a7484ab`. `docs/api/vdi/_dispatcher.md`
already documents that the raw `trap #2` VDI call returns `d0 = flip_y`
(0 or 1, "coordinate vs magnitude" flag, `vdi_asm.S:124-125`) as part of
the native trap ABI — but no per-function spec states *which* opcodes
set it.

**EmuTOS:** `flip_y` is a single global (`vdi_main.c:20`), reset to `0`
at the top of every `screen()` call (`vdi_main.c:135`) and left at `0`
unless a handler explicitly sets it. Within the Batch-2 attribute
setters, exactly three do:
- `vdi_vsm_height` (op 19, `vdi_marker.c:49`, inside the function body
  after computing `mark_scale`);
- `setup_width_height()` (`vdi_text.c:678`), the shared helper called by
  both `vdi_vst_height` (op 12) and `vdi_vst_point` (op 107) to write
  their `PTSOUT[0..3]` metrics.

`vdi_vst_font` (op 21, `vdi_text.c:939-985`) does **not** set `flip_y`
directly, but it internally re-drives the current size by calling
`vdi_vst_point`/`vdi_vst_height` (`vdi_text.c:975-978`) through a
redirected `PTSIN`/`PTSOUT` (a local `dummy[4]` buffer, so the caller's
*real* `PTSOUT` — 0 points declared for op 21 — is never touched). That
inner call still runs `setup_width_height()` against the global
`flip_y`, so **`vst_font` leaves `flip_y = 1` on return as a leaked side
effect**, even though its own jump-table entry declares `0` `PTSOUT`
words (`vdi_main.c:53`) and no code path writes the real `PTSOUT`.

Every other Batch-2 setter (`vsl_type/width/color/ends/udsty`,
`vsm_type/color`, `vsf_interior/style/color/perimeter/udpat`,
`vswr_mode`, `vs_clip`, `vs_color`, `vst_color/effects/alignment/
rotation`, `vst_load_fonts`/`vst_unload_fonts`) leaves `flip_y` at the
`0` set on entry to `screen()` — confirmed by grepping every `flip_y`
assignment in `vdi/*.c` (12 total: 2 in this batch's scope —
`vdi_marker.c:49`, `vdi_text.c:678` — plus `vdi_control.c:379`
(`init_wk`, batch 1), `vdi_control.c:631` (`vq_extnd`, batch 1), and
`vdi_text.c:1020,1058,1095,1173` (`vqt_attributes`/`vqt_extent`/
`vqt_width`/`vqt_fontinfo`, batch 4 — out of this task's scope, flagged
for the inquiry-spec owner).

**Prior NewTOS doc:** `_line-marker.md`'s `vsm_height` row and
`_text.md`'s `vst_height`/`vst_point`/`vst_font` rows did not mention
`flip_y` at all, even though `_dispatcher.md` already treats `d0 =
flip_y` as part of the documented native-trap **contract**.

**Decision:** documented in `vdi/attributes/_line-marker.md` (`vsm_height`)
and `vdi/attributes/_text.md` (`vst_height`/`vst_point`/`vst_font`) as an
explicit "sets `flip_y = 1`, so the trap's `d0` return is `1` for this
call" note. **Open question for the lead:** does NewTOS's native ABI
(selector-based `svc #2`, no m68k `d0` register) need an equivalent
per-call "coordinate vs magnitude" signal at all, or is this purely an
artifact of the m68k GEM binding library's PTSOUT-copy convention with
no observable effect through a flat struct-passing ABI? Recommend
treating it as **reference** (EmuTOS mechanism), not **contract**,
unless a concrete NewTOS C-binding consumer is shown to depend on it —
but flagging it here since `_dispatcher.md` currently classifies it as
contract without qualification.

---

## D-081: `st_fl_ptr()` reads past the end of `HATCH1[96]` if `vsf_interior` switches `FIS_PATTERN` → `FIS_HATCH` (or restores a pattern-range `fill_index` under hatch) without an intervening `vsf_style` call — a genuine EmuTOS source-level gap, not a spec-reading error

**Date:** 2026-07-10
**Phase 00b spec:** `docs/api/vdi/attributes/_fill.md`

**EmuTOS:** `vdi_vsf_interior` (`vdi_fill.c:173-181`) validates and
stores `vwk->fill_style` and calls `st_fl_ptr(vwk)` — it **never
touches `vwk->fill_index`**. `vdi_vsf_style` (`vdi_fill.c:186-201`)
validates `INTIN[0]` against a range that depends on the **current**
`fill_style` (`1..24` for `FIS_PATTERN`, else `1..12`) and stores
`fill_index = accepted-1` (0-based, so `0..23` for pattern, `0..11` for
every other style). `st_fl_ptr()` (`vdi_fill.c:338-380`) then picks the
pattern table using `vwk->fill_index` **re-interpreted under the
style current at the time `st_fl_ptr` runs**, with no bounds check
against the range the stored `fill_index` was originally validated for:

```
case FIS_HATCH:
    if (fi < 6) { pm = HAT_0_MSK; pp = &HATCH0[fi*(pm+1)]; }      /* HATCH0[48], safe for fi 0..5 */
    else        { pm = HAT_1_MSK; pp = &HATCH1[(fi-6)*(pm+1)]; }  /* HATCH1[96], safe only for fi 6..11 */
```

If a program does `vsf_interior(FIS_PATTERN)` → `vsf_style(24)` (stores
`fill_index = 23`, valid for `FIS_PATTERN`'s `OEMPAT[128]` table) →
`vsf_interior(FIS_HATCH)` **without an intervening `vsf_style`**,
`fill_style` becomes `FIS_HATCH` but `fill_index` is still `23`.
`st_fl_ptr()` (called by `vsf_interior` itself, `vdi_fill.c:180`) then
takes the `fi >= 6` branch with `fi=23`: `&HATCH1[(23-6)*16]` =
`HATCH1[272]` — **176 words past the end of the 96-word `HATCH1[]`
array** — and `vwk->patptr` is left pointing into whatever static data
follows `HATCH1` in the binary (`HOLLOW`/`SOLID`/other tables), which
the next fill/rectangle-fill/GDP draw call then reads as if it were a
16-word hatch pattern. Any `fill_index` in `12..23` inherited from a
prior `FIS_PATTERN` state triggers this (the safe range for
`FIS_HATCH` is only `fi <= 11`); the reverse direction (hatch→pattern,
`fi` up to `11`) stays in-bounds because pattern's own tables
(`DITHER[32]`+`OEMPAT[128]`) cover `fi` up to `23`. Confirmed by
tracing the array sizes: `DITHRMSK`(3)+1 × 8 dither slots = `DITHER[32]`;
`OEMMSKPAT`(7)+1 × 16 OEM slots = `OEMPAT[128]`, together spanning
`fi` `0..23` (`MAX_FILL_PATTERN`); `HAT_0_MSK`(7)+1 × 6 slots =
`HATCH0[48]`; `HAT_1_MSK`(0xF)+1 × 6 slots = `HATCH1[96]`, together only
`fi` `0..11` (`MAX_FILL_HATCH`) — there is no code path that clamps
`fill_index` back into range when `fill_style` changes without a
`vsf_style` call (unlike the line-style table, which explicitly guards
`line_index < 6` before indexing `LINE_STYLE[]`, falling back to
`ud_ls` otherwise — `vdi_line.c:2389`).

**This is a source-level EmuTOS gap** (an out-of-bounds read triggered
by a legal, undocumented setter call sequence), analogous in kind to
D-070 (`Frename`'s incomplete cycle check) rather than a documentation
misreading.

**Prior NewTOS doc:** `vdi/attributes/_fill.md` documented `st_fl_ptr()`
only as "recomputes the active `patptr`/`patmsk` from `fill_style`/
`fill_index`" without noting that `fill_index` is never reset or
re-validated on a bare style change, or that this can walk off the end
of a pattern table.

**Decision:** documented as a new edge case in `vdi/attributes/_fill.md`
with the exact unsafe sequence and the safe/unsafe `fill_index` ranges
per style. **Open question for the lead / Developer:** NewTOS's own
fill-pattern-table implementation must decide whether to reproduce this
verbatim (bug-for-bug, reading whatever static data trails its own
hatch table — implementation-defined, not useful to copy) or to add a
defensive re-clamp of `fill_index` on every `fill_style` change (a
NewTOS-side hardening, analogous to the D-070 `Frename` recommendation)
— **not** a decision to make silently in the spec.

---

## D-082: `PAT_FLAG` (bit 5, `0x10`) on `vro_cpyfm`/`vrt_cpyfm`/line-A raster copy never applies a pattern — the pattern pointer is unconditionally nulled before the flag is even tested (a genuine EmuTOS source-level gap, not a spec-reading error)

**Date:** 2026-07-10
**Phase 00b spec:** `docs/api/vdi/raster/_raster.md`

**EmuTOS:** `cpy_raster` (`vdi_raster.c:1330-1462`), the shared driver
behind `vdi_vro_cpyfm` (109), `vdi_vrt_cpyfm` (121) and `linea_raster`,
handles the pattern flag like this:

```c
/* check the pattern flag (bit 5) and revert to log op # */
info->p_addr = NULL;        /* get pattern pointer */
if (mode & PAT_FLAG) {
    mode &= ~PAT_FLAG;      /* set bit to 0! */
    setup_pattern(raster, info);   /* fill in pattern related stuff */
}
```
(`vdi_raster.c:1344-1349`). `info->p_addr` is unconditionally set to
`NULL` **before** the `PAT_FLAG` check, and `setup_pattern`
(`vdi_raster.c:790-800`) — called only when the flag is set — fills in
`p_nxpl`/`p_nxln`/`p_mask` but never touches `p_addr`:

```c
static void
setup_pattern (struct raster_t *raster, struct blit_frame *info)
{
    info->p_nxpl = 0;
    if (raster->multifill) {
        info->p_nxpl = 32;
    }
    info->p_nxln = 2;
    info->p_mask = 0xf;
}
```

Both callers of `cpy_raster` that own a `Vwk` pre-load the *intended*
pattern pointer just before the call — `vdi_info.p_addr = vwk->patptr;`
in `vdi_vro_cpyfm` (`:1475`) and `vdi_vrt_cpyfm` (`:1497`) — but this
assignment is immediately discarded by the unconditional null two
function calls later. `linea_raster` (the Line-A wrapper) never sets
`p_addr` at all before calling `cpy_raster`, so it is nulled all the
same. The only place `P_ADDR` is tested is the assembler fast path,
`fast_bit_blt` (`vdi_blit.S:1055-1056`):
```
tst.l   P_ADDR(a6)              // pattern and source ?
jeq     next_plane               // no pattern if pointer is null
```
— since `P_ADDR` is always null when reached through `cpy_raster`, this
branch is always skipped, i.e. the pattern-AND step never executes. The
alternate C blit path, `bit_blt()`/`do_blit()` (`vdi_raster.c:588-773`,
`424-565`, used for `CONF_WITH_BLITTER` and ColdFire), does not
reference `p_addr`/`p_indx`/`p_mask`/`p_nxln`/`p_nxpl` **anywhere** —
pattern support is entirely absent from that path regardless of
`p_addr`.

**Net effect:** setting `PAT_FLAG` (`mode |= 0x10`) on `vro_cpyfm`,
`vrt_cpyfm`, or the Line-A raster-copy primitive is **observably
identical to leaving it clear** in every EmuTOS build configuration —
the flag is read and stripped from `mode` (so the low-4-bit logic op
selection is unaffected), `setup_pattern` runs and computes some
now-unused fields, but no pattern is ever composited into the
destination.

**History (not a recent regression, but a real one):** `git blame`
shows the `p_addr = vwk->patptr` assignment used to live *inside*
`setup_pattern` itself (taking a `Vwk *` directly) before commit
`d7ac3170` ("Added support for 'Copy raster form'", Dec 2013), which
introduced the shared `struct raster_t` (needed because the new
Line-A caller has no `Vwk`) and moved responsibility for `p_addr` out
to the individual callers — but the replacement assignments in
`vdi_vro_cpyfm`/`vdi_vrt_cpyfm` were placed *before* `cpy_raster`'s own
unconditional `p_addr = NULL`, so they never take effect. The bug has
been present, unchanged, in every EmuTOS release since that commit
(confirmed unchanged at `1a7484ab`).

**This is a source-level EmuTOS gap** (dead code / silently
non-functional flag triggered by a legal, documented parameter value),
analogous in kind to D-081 rather than a documentation misreading.

**Prior NewTOS doc:** `docs/api/vdi/raster/_raster.md` stated bit 5
"additionally enables pattern blitting" — implying `vro_cpyfm`/
`vrt_cpyfm` can pattern-fill a copy. That capability does not exist in
EmuTOS as shipped.

**Decision:** corrected `vdi/raster/_raster.md` to document `PAT_FLAG` as a
stripped-but-inert bit (contract: identical output with or without it)
and added an explicit "do not implement pattern-fill for `PAT_FLAG`"
note for the Phase-12 `RasterBackend`. **Open question for the lead:**
none — this is a case where reproducing EmuTOS exactly means
implementing *less* than the opcode name suggests; no design decision
needed, just confirmation that NewTOS should not add pattern-fill
capability to `vro_cpyfm`/`vrt_cpyfm` that the reference never had.

---

## D-083: `v_escape` sub 12 (`v_curtext`/`escfn12`) cannot output character code `0xFF` — `Crawio`'s own read/write sentinel intercepts it (a genuine interaction gap between two independently-correct pieces of source, not a spec-reading error)

**Date:** 2026-07-10
**Phase 00b spec:** `docs/api/vdi/_escape.md`

> **DECISION (user, 2026-07-23): keep drawing 0xFF — deliberate, documented
> deviation from EmuTOS.** NewTOS' `v_curtext` (`tos-api/src/vdi/escape.rs`)
> renders character code `0xFF` directly instead of routing it through the
> GEMDOS `Crawio` path, so — unlike EmuTOS — the one printable glyph at code
> 255 is actually emitted. This is a 1-in-256 edge in a single VDI escape and
> practically irrelevant; the user judged replicating the EmuTOS bug not worth
> it. Recorded as a conscious deviation (not a bug, not an oversight). If strict
> 1:1 is ever wanted, routing this one code through the `Crawio` `0xFF`
> read/write sentinel is a one-line change. Otherwise **no code change.**

**EmuTOS:** `escfn12` (`v_curtext`, VDI opcode 5 sub 12) writes cursor-
addressable alpha text by looping over the caller's `INTIN[]` character
array and sending each one through the local `crawio()` wrapper
(`vdi/vdi_esc.c:196-207`):
```c
static void escfn12(Vwk * vwk)
{
    int cnt;
    WORD *chr;

    cnt = CONTRL[3];            /* get the character count */
    chr = INTIN;                /* address of the character array */

    while (cnt--) {
        crawio(*chr++);         /* raw i/o to standard input/output */
    }
}
```
`crawio()` (`vdi_esc.c:29-32`) is a thin wrapper around the GEMDOS trap:
```c
static LONG crawio(WORD ch)
{
    return Crawio(ch);          /* #define Crawio(w) trap1(0x06, w) */
}
```
GEMDOS function `0x06` is `xrawio` (`bdos/console.c:441-453`):
```c
long xrawio(int parm)
{
    int i;

    if (parm == 0xFF)
    {
        i = HXFORM(run->p_uft[0]);
        return constat(i) ? getch(i) : 0L;
    }

    Bconout(HXFORM(run->p_uft[1]), parm);
    return 0; /* dummy */
}
```
`xrawio` uses the exact WORD value `0x00FF` as a **read-vs-write
sentinel**: `parm == 0xFF` means "read a character from stdin instead of
writing one" (already documented in
[`docs/api/gemdos/console/06-crawio.md`](gemdos/console/06-crawio.md),
edge case 3, which independently confirms the comparison is against the
*full* WORD `0x00FF`, not just the low byte). A `v_curtext`
character-array entry of exactly `INTIN[i] = 0x00FF` (character/glyph
code **255**) therefore never reaches `Bconout`; instead `xrawio` probes
`constat()` and, if a key is pending, consumes it via `getch()` — a
side-effecting **read**, not the intended **write**. The character byte
that should have appeared on screen is silently dropped either way.

**Is `0xFF` a real, reachable glyph, or an edge-of-range curiosity?**
Confirmed reachable: the standard EmuTOS/Atari system font
(`bios/fnt_st_8x8.c:150-151`) declares `first_ade=0, last_ade=255` — the
full byte range 0–255 is defined glyph data, so code 255 is a normal,
renderable character, not an out-of-font sentinel value the font itself
reserves.

**Scope — `v_curtext` only:** the other `v_escape` sub-functions that
write to the console (`v_exit_cur`/`v_enter_cur`/cursor moves/erase/
reverse-video, subs 2–10, 13, 14) use `bios_conws()` →
`Bconout(2, ch)` directly (`vdi_esc.c:39-43`), which has no read/write
duality — every byte value, including `0xFF`, is written unconditionally.
The `0xFF` gap is specific to the one sub-function (12) that was
deliberately routed through raw GEMDOS console I/O instead of the BIOS
path (see `_escape.md` edge case 5 for *why* `v_curtext` uses `Crawio`
at all: unrelated to this gap — GEMDOS `Cconout`/`Crawio` semantics for
tab expansion / column tracking, not a read/write concern).

**Not avoidable by the caller within the documented ABI:** `v_curtext`
passes one character per `WORD` slot in `INTIN[]`; a caller wanting to
display glyph 255 has no alternate encoding available through this VDI
entry point (there is no escaping/doubling convention) — the character
is categorically unwritable via `v_curtext`, for any `v_escape` caller,
in every EmuTOS build (no `CONF_WITH_*` gates either function).

**This is a genuine EmuTOS source-level gap** — not a documentation
misreading — but it differs in *kind* from D-081/D-082: those were
single-function oversights (an unguarded array read; a pattern pointer
nulled before its own flag check). This one is an **emergent
interaction** between two functions that are each individually correct
on their own terms: `Crawio`'s `0xFF` sentinel is a deliberate,
documented (`06-crawio.md`) part of that trap's own contract, and
`escfn12`'s per-character `Crawio` loop is a reasonable, simple
implementation choice — the gap only exists at the seam between them,
and only for the one value where the sentinel and a legitimate character
code coincide.

**Contract-vs-internal open question (Compendium cross-check deferred,
consistent with this spec file's front matter):** whether this is
*contract* (real Atari TOS's `v_curtext` presumably also routes through
`Crawio`, since `Crawio`'s `0xFF` read/write convention is a
long-standing, pre-EmuTOS GEMDOS ABI feature, not an EmuTOS invention —
so real TOS very plausibly shares this exact limitation) or merely an
*EmuTOS implementation detail* cannot be settled from source alone; no
Compendium/Profibuch PDF is present in `references/` to check the
documented `v_curtext` semantics against. Until that cross-check happens,
treat this as EmuTOS-confirmed, presumed-contract (per CLAUDE.md Rule 1,
EmuTOS is the spec until shown otherwise) — **not** a NewTOS-side bug to
silently "fix" by special-casing byte `0xFF` in a NewTOS `v_curtext`
implementation.

**Prior NewTOS doc:** `docs/api/vdi/_escape.md`'s sub-12 section
described only the happy path ("Writes each character via `Crawio()`
… Text appears at the current cursor position") with no mention of the
`0xFF` interaction. Added as a new callout in that section, plus edge
case 6 and this global write-up; local table row `docs/api/vdi/
_discrepancies.md` **D-VDI-22**.

**Decision:** none required right now — documentation-only finding for
this pass. **Open question for the lead:** should NewTOS's own
`v_curtext` implementation (Phase 12) reproduce this exact limitation
(bit-for-bit contract fidelity, pending the Compendium cross-check
above), or should it always write the byte and let a NewTOS-native
`Crawio` keep its own `0xFF` sentinel *only* for the direct `Crawio`
trap, not for `v_curtext`'s internal use of it? This is a genuine
API-layering decision (does NewTOS's `v_curtext` have to call through
its own `Crawio`, or may it write to the console directly and bypass the
sentinel?) that the spec alone cannot resolve — flagged for the
Developer/lead, not decided here.

---

## D-084: AES dispatcher's `-1` unknown-opcode convention — re-confirmed, is the fourth distinct TOS dispatch convention, and is only visible to a caller that declares `OUT_LEN ≥ 1`

**Date:** 2026-07-10 (AES leg of the bottom-up sweep, Task 1)
**Phase 00b spec:** `docs/api/aes/_dispatcher.md` §2, §3a (new)

**Context:** first task of the AES leg of the bottom-up sweep (GEMDOS,
BIOS, XBIOS, VDI already done; see D-061 for the GEMDOS/BIOS-XBIOS
contrast and D-079 for the three-way contrast that added VDI). Re-read
`aes/gemsuper.c:77-466` and `include/aesdefs.h:16-140` line-by-line
against the existing `docs/api/aes/_dispatcher.md` (which already carries
D-047's `-1`-with-alert correction from 2026-07-03).

**EmuTOS (confirmed, no factual correction to D-047's value):**
`crysbind`'s `default:` arm (`gemsuper.c:406-417`) is the only path any
unhandled `WORD opcode` (including negative values, values > 127, and any
compiled-out opcode) can take. It always sets `ret = -1`; unless
`opcode == 0` it also pops up a synchronous on-screen alert
(`fm_show(ALNOFUNC, &opcode, 1)`) — opcode `0` is silently ignored "since
some PRGs make this call" (`gemsuper.c:414`).

**New finding not previously documented:** the `-1` only reaches the
*caller's* `intout[0]` if the caller declared `control[2]` (`OUT_LEN`)
`≥ 1`. `xif` writes the opcode's result into a **local** `int_out[0]`
(`RET_CODE = crysbind(...)`, `gemsuper.c:441`) and only copies it back to
`pcrys_blk->intout` when `OUT_LEN` is non-zero (`gemsuper.c:444-445`); a
call that declares zero output words never sees the `-1` in its own
`intout` array — that array is left exactly as the caller set it up. The
trap's own return register is unaffected either way (`super()` always
returns `0`, `gemsuper.c:465`). This is a corollary of the already-documented
`xif` copy-back mechanics in §2, not a new mechanism, but it had not been
connected to the unknown-opcode path before.

**Four-way cross-trap contrast (extends D-061/D-079 to all four TOS
traps):** GEMDOS returns `EINVFN` (-32, an explicit error code); BIOS/XBIOS
return the echoed function number (not an error code); VDI signals nothing
observable at all (a bare `return;`); **AES returns the fixed literal `-1`
and — uniquely among all four traps — also produces a user-visible
synchronous side effect (the alert box)**, except for the deliberately
silent opcode-0 case. Each convention is internally self-consistent within
its own trap; none should be "fixed" toward another by analogy.

**Prior NewTOS doc:** `docs/api/aes/_dispatcher.md` already had D-047's
`-1`-with-alert correction; it did not have the four-way contrast table nor
the `OUT_LEN ≥ 1` visibility caveat. Both added as new §3a.

**Not a Compendium discrepancy** — an internal completeness addition (no
PDFs in `references/` for cross-check).

**Decision:** follow EmuTOS — unknown/unimplemented AES opcodes return the
literal `-1`, visible only when the caller declares `OUT_LEN ≥ 1`; the
reference alert box is optional for NewTOS to reproduce, the `-1` value and
its `OUT_LEN`-gated visibility are contract. Documented in
`docs/api/aes/_dispatcher.md` §3a.

---

## D-085: Menu-bar tree topology — a title's drop-down box is *not* its child object; it is looked up positionally through a separate hidden container

**Date:** 2026-07-10 (AES leg, Menu Manager re-validation, Batch 2)
**Phase 00b spec:** `docs/api/aes/menu.md` ("The menu bar IS an OBJECT tree")

**Context:** third validation pass of the Menu Manager spec (opcodes 30–39),
after the 2026-07-03 re-derive round (D-051). Read the full implementation of
`menu_sub`, `menu_fixup`, `menu_down`, `mn_do` in `aes/gemmnlib.c` line by
line, not just the already-validated slot-map/return-value tables.

**Previous spec (wrong):** "Under `THEACTIVE` the titles (`G_TITLE`) are
siblings; each title's child is a `G_BOX` drop-down whose children are the
items (`G_STRING`)."

**EmuTOS (confirmed):** `menu_sub` (`aes/gemmnlib.c:71-86`) — the function
that maps a title object index to its drop-down box index — does **not**
read the title object's own `ob_head`. It reads `tree[THESCREEN].ob_tail`
(`THESCREEN`'s *second* child, called `themenus` in the source; `THEBAR` is
the first/`ob_head` child), then walks `themenus`'s children via the
`ob_next` sibling chain, one hop per title position, starting from
`tree[themenus].ob_head`:

```c
static WORD menu_sub(OBJECT *tree, WORD ititle)
{
    themenus = (tree+THESCREEN)->ob_tail;
    imenu = (tree+themenus)->ob_head;
    for (i = ititle-THEACTIVE; i > 1; i--)
        imenu = (tree+imenu)->ob_next;
    return imenu;
}
```

I.e. the drop-down boxes for all titles are siblings of *each other* under a
hidden container object, positionally correlated with title order — **not**
children of their respective title objects. `menu_fixup`
(`gemmnlib.c:89-125`) independently confirms this: it computes the Desk
menu's own drop-down box directly as `dabox = tree[themenus].ob_head`
(`gemmnlib.c:101-102`), matching `menu_sub`'s result for `ititle=THEDESK`
(the first title). `menu_down` (`gemmnlib.c:229-245`) calls `menu_sub` to
locate the box before drawing it — it is never obtained via any parent-child
relationship from the title.

**Why this is contract, not internal:** even though `menu_sub`'s C loop is a
reimplementable helper, the *tree shape it decodes* is not — it is exactly
how any external resource compiler lays out (and any real GEM program's
compiled `.RSC` menu bar contains) a menu-bar `OBJECT` tree. A NewTOS
menu-bar object-tree interpreter built on the (incorrect) "drop-down is the
title's child" assumption would misinterpret every real menu-bar resource:
wrong box drawn on drop-down, wrong item highlighted, wrong `item` object
number in `MN_SELECTED`.

**Not a Compendium discrepancy** — no PDFs in `references/` for cross-check;
this is a pure source-reading correction of the previous NewTOS spec, found
by reading the full function body instead of trusting the earlier
high-level description.

**Decision:** `docs/api/aes/menu.md` corrected — new object-index table
annotations, a dedicated "Correction (D-085)" callout under "The menu bar
IS an OBJECT tree", updated "Related data structures" and edge-case #11,
and a NewTOS-implementation-notes callout that this topology is a real
constraint on any menu-bar resource format NewTOS accepts.

---

## D-086: `menu_popup` — the `submenu_event` helper cited in the previous spec does not exist (stale EmuTOS source comment); plus two undocumented behaviours (blocks on held button, silent OOM)

**Date:** 2026-07-10 (AES leg, Menu Manager re-validation, Batch 2)
**Phase 00b spec:** `docs/api/aes/menu.md` (`menu_popup` (36))

**Context:** same pass as D-085; read `mn_popup`/`handle_popup`/`popup_blit`
(`aes/gemmnext.c:136-356`) in full.

**Previous spec (wrong):** listed `handle_popup`/`submenu_event` as the two
helpers implementing the pop-up tracking loop that "sets `keystate`".

**EmuTOS (confirmed):** `grep -rn submenu_event references/emutos/` matches
exactly **one** location in the whole tree: a comment,
`aes/gemmnext.c:342: mdata->mn_keystate = keystate; /* set by
submenu_event() */`. No function, static or otherwise, named
`submenu_event` exists anywhere in EmuTOS. `keystate` is in fact a
file-static (`gemmnext.c:65`) set directly inside `handle_popup`
(`gemmnext.c:245-299`) at `gemmnext.c:272: keystate = rets[3];` — the
comment is stale/inaccurate EmuTOS source text (per project rule: comments
lie, code doesn't). The previous spec derived a fictitious helper name from
this comment rather than from the code that actually sets the value.

**New, previously-undocumented behaviour found in the same pass:**
- `mn_popup` calls `ev_button(1, 0x00ff, 0x0000, rets)` (`gemmnext.c:314`)
  **before** doing anything else (grabbing background, positioning the box)
  — it blocks until any currently-held mouse button is released before the
  popup is shown.
- If `popup_blit`'s background-save allocation fails
  (`dos_alloc_stram`, `gemmnext.c:157-163`, returns `NULL`), `mn_popup`
  never calls `handle_popup` at all (`gemmnext.c:329-334` skipped) and falls
  straight through to the "no item chosen" return path
  (`mdata->mn_keystate = 0`, `rc = FALSE`) — a caller cannot distinguish "no
  pick" from "out of ST-RAM" from the return value alone.

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading correction/completion of the previous NewTOS
spec.

**Decision:** `docs/api/aes/menu.md` `menu_popup (36)` section corrected
(helper list, behaviour paragraph, new callout, edge-case #13); error table
updated to note the OOM path returns the same `0` as "no pick".

---

## D-087: `menu_attach` also writes the item's `ob_type` upper byte (submenu-table index), not just `ob_flags`/the marker character

**Date:** 2026-07-10 (AES leg, Menu Manager re-validation, Batch 2)
**Phase 00b spec:** `docs/api/aes/menu.md` (`menu_attach` (37))

**Context:** same pass as D-085/D-086; read `attach_submenu`/
`detach_submenu`/`find_submenu_from_obtype` (`aes/gemmnext.c:358-460,
465-530`) in full, not just `mn_attach`'s own three-line dispatch body.

**Previous spec (incomplete):** "Attaching sets the item's `SUBMENU`
(0x0800) `ob_flags` bit and writes a right-arrow marker (0x03) two chars
from the end of the item text."

**EmuTOS (confirmed):** true, but incomplete. `attach_submenu`
(`gemmnext.c:522-527`) also does:

```c
obj->ob_type |= ((submenu - gl_submenu) + 128) << 8;   /* gemmnext.c:524 */
```

— it encodes the item's `SMIB` array index (offset by 128) into the **upper
byte of `ob_type`**, a field otherwise holding the object's display-class
number (`G_STRING`=28 etc., always < 256 for a plain object). This is how
`find_submenu_from_obtype` (`gemmnext.c:361-375`) later recovers which
`SMIB` entry an item's submenu lives in: `n = ((ob_type>>8)&0xff) - 128;`
(`gemmnext.c:369`). `detach_submenu` (`gemmnext.c:438-460`) reverses this
exactly: `obj->ob_type &= 0x00ff;` (`gemmnext.c:449`), clears `SUBMENU`, and
blanks the marker character with a space.

**Why this matters for NewTOS:** classic TOS has no memory protection
between an application and its own `OBJECT` tree — a program can (and
historically, some do) read `ob_type` directly after calling
`menu_attach`/`menu_istart`. A NewTOS reimplementation that only sets
`ob_flags` (as the previous spec said) and manages the `SMIB` mapping some
other way would leave `ob_type`'s upper byte at `0`, which is an observable
difference for any such program.

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading completion of the previous NewTOS spec.

**Decision:** `docs/api/aes/menu.md` corrected: `ob_state`/`ob_flags` bit
table now notes the `ob_type` mutation; `menu_attach (37)` section has a new
correction callout; "Related data structures" documents `SMIB`
(`aes/struct.h:180-186`, `NUM_SMIBS=128`) and the `ob_type` encoding as
(narrowly) contract even though the `SMIB` array/index-allocation scheme
itself remains reference/reimplementable.

---

## D-088: `menu_attach` — undocumented failure paths (OOM, table-full, remove-nothing) and replace-on-reattach semantics

**Date:** 2026-07-10 (AES leg, Menu Manager re-validation, Batch 2)
**Phase 00b spec:** `docs/api/aes/menu.md` (`menu_attach` (37))

**Context:** same pass as D-087; the previous spec's "Error conditions"
table for `menu_attach` had exactly one row (non-`G_STRING` item guard).
Reading all of `attach_submenu` (`gemmnext.c:465-530`) surfaces four more
observable outcomes.

**EmuTOS (confirmed):**
- **`ME_ATTACH` on an item that already has a submenu replaces it**:
  `attach_submenu` detaches the existing submenu first
  (`gemmnext.c:471-477`), then proceeds to attach the new one — it does not
  fail, and does not leak the old `SMIB` slot (`detach_submenu` decrements
  `s_usage`).
- **`ME_REMOVE` on an item with no attached submenu returns `FALSE`**:
  `mn_attach(ME_REMOVE, …)` calls `attach_submenu(tree, item, NULL)`; since
  `query_submenu` is `FALSE` (no submenu to detach) execution falls through
  to `if (!mdata) return FALSE;` (`gemmnext.c:471-481`) — "nothing to
  remove" is indistinguishable from any other failure via the return value.
- **`gl_submenu` array allocation (`dos_alloc_anyram`, on the very first
  `ME_ATTACH` call) failing → `FALSE`** (`gemmnext.c:484-488`).
- **The `SMIB` table (`NUM_SMIBS`=128 entries, `aes/struct.h:53`) being
  full → `FALSE`** (`gemmnext.c:509-512`).
- (Separately, `query_submenu` explicitly tolerates a `NULL` output pointer
  for `ME_INQUIRE` as an EmuTOS-only convenience — "not specified by TOS"
  per its own comment, `gemmnext.c:397-400` — flagged as non-contract.)

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading completion of the previous NewTOS spec's
error-conditions coverage.

**Decision:** `docs/api/aes/menu.md`'s "Error conditions" table gained four
new rows; `menu_attach (37)` section gained a "D-088" addition callout;
edge-case #12 added.

---

## D-089: the `ob_state`/`ob_spec` border-thickness (`th`) sign convention was inverted — `th > 0` is *inside*, `th < 0` is *outside*, not the reverse

**Date:** 2026-07-10 (AES leg, Object/Form Manager re-validation, Batch 3)
**Phase 00b spec:** `docs/api/aes/object-form.md` §0 ("The object tree and
the drawing primitives"), `docs/api/structures/tedinfo.md` (`te_thickness`)

**Context:** third validation pass of the Object/Form spec, specifically
tasked with the same rigour that found D-085 (menu tree topology). Traced
every use of the `th`/`border` value computed by `ob_sst`
(`gemobjop.c:25-73`) through to where it actually determines pixel
geometry, rather than trusting the existing one-line summary.

**Previous spec (wrong):** "Values >128 are treated as negative
(`th -= 256`) — **`th < 0` = inside border, `th > 0` = outside border**."
The same claim, "Negative = inside border, positive = outside", was also
copied into `docs/api/structures/tedinfo.md`'s `te_thickness` row (`th` and
`te_thickness` are the same value — `ob_sst` reads `te_thickness` directly
for TED types, `gemobjop.c:51-52`).

**EmuTOS (confirmed, sign is the opposite):**
1. **`gr_box`** (`gemgraf.c:813-832`), which actually draws the border
   lines: for `th > 0` each iteration calls `gr_inside(&n, th)` with a
   **positive** offset, which *shrinks* the box each pass — so the drawn
   lines are **on or inside** the base rect. For `th < 0`, `gr_inside`
   receives a **negative** offset, which *grows* the box each pass — the
   drawn lines are **on or outside** the base rect.
2. **`just_draw`**'s fill inset (`gemoblib.c:553,663-671`):
   `tmpth = (th < 0) ? 0 : th;` then `gr_inside(&t, tmpth)` before filling
   the interior. An outside (`th < 0`) border does **not** inset the fill —
   the interior occupies the full nominal rect, because the border sits
   outside it. An inside (`th > 0`) border **does** inset the fill by `th`.
3. **`just_draw`**'s state-decoration inset (`gemoblib.c:889-892`):
   `if (th > 0) gr_inside(&t, th); else th = -th;` — only a positive
   (inside) border shrinks the content rect before drawing
   `SHADOWED`/`CHECKED`/`CROSSED`/`DISABLED`/`SELECTED` decorations.
4. **`ob_actxywh`**'s 3D shadow hit-test adjustment (`gemoblib.c:329-334`)
   only grows the AES hit-test rect for a `SHADOWED` object **when
   `border < 0`** — a drop shadow (which visually extends outward) only
   needs a bigger hit box when the object's own border is the
   outward-growing (negative) kind, confirming the same polarity.

All four independent code paths agree: **`th > 0` ⇒ border drawn inside
the object's nominal rect (content shrinks by `th`); `th < 0` ⇒ border
drawn outside the nominal rect (content is the full rect, border is an
add-on)**. `G_BUTTON`'s `th` is always negative (`-1`, further decremented
for `EXIT`/`DEFAULT`, `gemobjop.c:59-65`) — i.e. **every button's border is
drawn outside its nominal rect**, which is also why a `DEFAULT` exit button
(most negative `th`) looks visually larger/more emphasized: its border
rings extend further outward, not inward.

**Why this matters for NewTOS:** a renderer built on the inverted rule
would draw every `G_BOX`/`G_BUTTON`/`G_BOXTEXT`/etc. border on the wrong
side of the object's edge (buttons would appear to shrink instead of grow
their visual footprint), get the interior-fill inset backwards for boxes
with an outside border, and get `SHADOWED`-object hit-test sizing
backwards (`ob_actxywh`'s shadow-rect growth is gated on the sign of
`border`).

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading correction of the previous NewTOS spec, found
by tracing `th` through its actual consumers instead of trusting the
existing summary sentence (the same class of error as D-085 and the
`scasb` misread referenced in CLAUDE.md).

**Decision:** `docs/api/aes/object-form.md` §0 corrected (with a full
citation trail across `gr_box`/`just_draw`/`ob_actxywh`) and a new edge
case #17; `docs/api/structures/tedinfo.md`'s `te_thickness` row corrected
to match.

---

## D-090: `fm_alert`'s pipeline steps (DEFAULT-flag set, centring, screen save, draw, `fm_do`) were misattributed to `fm_build`; the real caller order matters because `fm_build` unconditionally overwrites button `ob_flags`

**Date:** 2026-07-10 (AES leg, Object/Form Manager re-validation, Batch 3)
**Phase 00b spec:** `docs/api/aes/object-form.md` (`form_alert` (52))

**Context:** same pass as D-089; read all of `fm_build`
(`gemfmalt.c:188-316`) and `fm_alert` (`gemfmalt.c:319-396`) line by line
instead of trusting the previous spec's single run-on sentence describing
the alert-building pipeline.

**Previous spec (wrong attribution):** "`fm_build` (`gemfmalt.c:188-316`)
sizes and positions the DIALERT tree …, converts char coords to pixels
(`rs_obfix`), sets the `DEFAULT` flag on button `defbut`, centres the tree
(`ob_center`), saves the screen underneath (`bb_save`), draws it
(`ob_draw`), and runs `fm_do`." — attributing all of these actions to
`fm_build`.

**EmuTOS (confirmed):** `fm_build` (`gemfmalt.c:188-316`) only sizes,
positions, and pixel-converts the DIALERT tree, and adds the
icon/message/button objects as `ROOT`'s children. Everything else is done
by **`fm_alert`** itself, strictly *after* `fm_build` returns
(`gemfmalt.c:319-396`): setting `DEFAULT` on button `defbut`
(`:343-347`), fixing up the icon to `G_IMAGE`/32×32 (`:349-352`), centring
(`ob_center`, `:355`), saving the screen (`bb_save`, `:363`), drawing
(`ob_draw`, `:367`), running `fm_do` (`:383`), and restoring the screen
(`bb_restore`, `:390`).

**Why the order is load-bearing, not just cosmetic:** `fm_build`
**unconditionally assigns** (not ORs) each button object's flags:
`obj->ob_flags = BUTTON_FLAGS;` (`gemfmalt.c:288`, where `BUTTON_FLAGS =
SELECTABLE|EXIT[|FL3DACT]`). If a NewTOS reimplementation, following the
previous spec's (wrong) attribution, applied the `DEFAULT` bit *inside* or
*before* the tree-build step, `fm_build`'s flag assignment would silently
overwrite/discard it. EmuTOS avoids this by construction, applying
`DEFAULT` only after `fm_build` has returned — an implementer must
replicate that ordering, not just the end state.

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading correction of the previous NewTOS spec.

**Decision:** `docs/api/aes/object-form.md`'s `form_alert (52)` section
rewritten to attribute each step to the correct function with line
citations, a "D-090" correction callout explaining the flag-overwrite
hazard, and a new edge case #18.

---

## D-091: `ob_format` implements only two fill directions (`TE_RIGHT` vs. everything else), not three distinct `TE_LEFT`/`TE_CNTR`/`TE_RIGHT` behaviours

**Date:** 2026-07-10 (AES leg, Object/Form Manager re-validation, Batch 3)
**Phase 00b spec:** `docs/api/aes/object-form.md` (`objc_draw` (42), text
family / `just_draw` item 3)

**Context:** same pass as D-089/D-090; read `ob_format`
(`gemoblib.c:361-408`) and `gr_just`/`gr_gtext` (`gemgraf.c:567-605`) in
full to check the previous spec's "`_` slots filled, literals kept,
`TE_LEFT`/`RIGHT`/`CNTR` justification" claim against the actual code.

**Previous spec (imprecise):** described `ob_format` (the routine that
merges `te_ptext` into the `te_ptmplt` template for `G_FTEXT`/`G_FBOXTEXT`)
as implementing "`TE_LEFT`/`RIGHT`/`CNTR` justification".

**EmuTOS (confirmed):** `ob_format` has exactly one conditional:
`if (just == TE_RIGHT)`. That branch anchors the raw text to the *end* of
the template and fills `_` slots **backward**. Every other value of
`just` — **including both `TE_LEFT` and `TE_CNTR`** — falls through to the
same unconditional forward-fill code, filling `_` slots from the *start* of
the template. `ob_format` therefore cannot distinguish `TE_LEFT` from
`TE_CNTR` at all; there is no third, "centred", underscore-fill behaviour
in this function. The whole-string centring/right-justification that
`TE_CNTR`/`TE_RIGHT` are normally associated with (positioning the
*rendered* text within the object's display box) is a separate, later step
performed by `gr_just`/`gr_gtext` (`gemgraf.c:567-605`), which *does*
implement all three values distinctly — and which `just_draw` calls right
after `ob_format`, in the same type-switch (`gemoblib.c:757`).

**Why this matters for NewTOS:** an implementer following the previous
spec literally would try to build a three-way "centred slot-fill" branch
into the template-merge routine that does not exist in EmuTOS, and might
miss that `TE_CNTR` for an *editable* field data-entry point
(`find_pos`/`ob_stfn`, `gemobed.c:158-173,316-320`) is computed against a
left-filled raw string, not a centre-filled one.

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading correction/precision-fix of the previous
NewTOS spec.

**Decision:** `docs/api/aes/object-form.md`'s text-family description in
`objc_draw (42)` corrected; new "D-091" correction callout and edge case
#19 added, distinguishing `ob_format`'s two-way fill from `gr_just`'s
three-way box-positioning.

---

## D-092: `form_dial` has no screen save/restore at all; it is `form_alert` (and the Menu Manager's drop-down) that depends on Screen↔Memory `vro_cpyfm`, currently a no-op gap in NewTOS

**Date:** 2026-07-10 (AES leg, Object/Form Manager re-validation, Batch 3)
**Phase 00b spec:** `docs/api/aes/object-form.md` (`form_dial` (51),
`form_alert` (52), "Dependencies")

**Context:** the task briefing for this validation round asked to document
an interaction between `form_dial`'s `FMD_START`/`FMD_FINISH` phases and
the VDI Memory-MFDB gap flagged in the 2026-07-10 VDI-audit `STATUS.md`
entry (item F3: `vro_cpyfm`/`vrt_cpyfm` are a no-op in NewTOS when either
side is a non-screen MFDB). Reading `fm_dial` (`gemfmlib.c:360-383`) in
full to locate the save/restore call shows this premise does not match the
source.

**EmuTOS (confirmed):** `fm_dial` never calls `bb_save`, `bb_restore`, or
`vro_cpyfm`, directly or indirectly. `FMD_START` is an empty `switch` case
with only a comment ("grab screen sync or some other mutual exclusion
method"); `FMD_GROW`/`FMD_SHRINK` draw an animated **outline** box
(`gr_growbox`/`gr_shrinkbox`, vector lines only); `FMD_FINISH` asks the
**window manager** to redraw the area under `pt` (`w_drawdesk`/`w_update`),
which recomputes desktop/window content rather than restoring a saved
bitmap. `form_dial` therefore has **no** dependency on Memory-MFDB support.

The function that actually does a Screen↔Memory copy is **`fm_alert`**
(opcode 52, not 51): `bb_save`/`bb_restore` (`gemfmalt.c:363,390`, defined
`gemgsxif.c:320-393`) blit between the physical screen and an off-screen
buffer `gl_tmp` (`fd_addr` allocated via `dos_alloc_anyram`,
`gemgsxif.c:108-113`) using `vro_cpyfm(S_ONLY, …)` — this **is** exactly
the Screen↔Memory-MFDB case that `docs/STATUS.md`'s F3 item flags as
currently a no-op in NewTOS's `RasterBackend`. The Menu Manager's
drop-down save/restore (`gemmnlib.c:219,221`, already noted as a
`vro_cpyfm`-class dependency in `docs/api/aes/menu.md`) shares the same
`gl_tmp` buffer and has the identical dependency.

**Consequence:** as implemented today, `form_alert` (52) can draw an alert
box but cannot properly save the background before drawing it nor restore
it afterward — this is a VDI raster-layer gap (already tracked in
`docs/STATUS.md` as deferred item F3), not an AES spec ambiguity.
`form_dial` (51) is unaffected and has no such prerequisite.

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading correction of a premise in this validation
round's task briefing, confirmed against the full `fm_dial`/`fm_alert`
source.

**Decision:** `docs/api/aes/object-form.md` corrected: `form_dial (51)`
gained a callout stating it has no save/restore dependency; `form_alert
(52)` gained the Memory-MFDB blocking note (with citations to
`docs/STATUS.md` F3); the "Dependencies → Requires" VDI bullet rewritten to
attribute the save/restore need to `form_alert` only; new edge case #20.

**Status (2026-09-20, A72): the F3 no-op premise is superseded, not this
entry's EmuTOS source analysis.** Task #51 (Screen↔Memory `vro_cpyfm`)
landed 2026-09-20 as A57/A71 (`docs/STATUS.md` Phase-13 row / Recent
Decisions 2026-09-20): `tos-api/src/vdi/raster.rs` implements all four
`vro_cpyfm` source/destination combinations, and `form_alert`'s
`bb_save`/`bb_restore` (`gemfmalt.c:363,390`) now save/restore the
background into NewTOS's one persistent AES screen buffer
(`tos-api/src/aes/gsxif.rs`). The above **EmuTOS source finding stands
unchanged** (`fm_dial` has no save/restore call, `fm_alert` does, and the
identity of the two functions was the actual point of this entry); only
the **NewTOS-side consequence** ("currently a no-op gap") is stale, and is
corrected in place in `docs/api/aes/object-form.md` (the D-092 blocking
note, edge case #20, and the "Dependencies → Requires" bullet all carry a
dated 2026-09-20/A72 correction rather than being rewritten silently).

---

## D-093: two previously-undocumented `objc_change`/`fm_button` behaviours (`G_IMAGE` deselect does XOR *and* redraw; `fm_button` zeroes the next-object output for ordinary button clicks)

**Date:** 2026-07-10 (AES leg, Object/Form Manager re-validation, Batch 3)
**Phase 00b spec:** `docs/api/aes/object-form.md` (`objc_change` (47),
`form_button` (56))

**Context:** same pass as D-089–D-092; read `ob_change`
(`gemoblib.c:1253-1320`) and `fm_button` (`gemfmlib.c:201-270`) in full,
beyond the previously-validated slot tables and high-level behaviour
summaries.

**EmuTOS (confirmed), two additions:**
- **(a) `ob_change` on a `G_IMAGE` whose `SELECTED` bit toggles does *both*
  an XOR background-fill *and* a full `just_draw`, not one or the other.**
  `xor_is_ok` returns `FALSE` unconditionally for `G_IMAGE`
  (`gemoblib.c:114`). In `ob_change`'s default case
  (`gemoblib.c:1295-1304`): `if (ok || ((obtype==G_IMAGE) &&
  !(new_state&SELECTED))) { bb_fill(MD_XOR, …); redraw = !ok; }` — for a
  `G_IMAGE` being *deselected*, this condition is true via the `G_IMAGE`
  special case, `ok` is `FALSE`, so `redraw` stays `TRUE`: the XOR fill
  runs (undoing the background inversion `just_draw` applies to a
  *selected* `G_IMAGE`, `gemoblib.c:766-770`) **and** `just_draw` runs
  afterward (re-blitting the icon in its normal colour). The previous spec
  described these as alternative paths ("toggle SELECTED via a fast XOR
  fill when possible … and fall back to `just_draw` otherwise"), which is
  correct for every other type but not for `G_IMAGE` deselection, where
  both happen.
- **(b) `fm_button` zeroes its next-object output (`FM_ONXTOB`) for a click
  on an ordinary, non-`EDITABLE` object that doesn't end the dialog.**
  `gemfmlib.c:264-267`: `if (cont && !(flags & EDITABLE)) new_obj = 0;`.
  The previous spec's `form_button (56)` section stated the output is "next
  object" without this qualifier — in practice, clicking a plain button
  that doesn't exit the dialog reports `FM_ONXTOB = 0`, not the clicked
  object's index; only an `EDITABLE`-field click (to move edit focus) or a
  dialog-ending click (where the real index, possibly `|0x8000` for
  `TOUCHEXIT` double-click, is preserved) yields a non-zero `FM_ONXTOB`.

**Not a Compendium discrepancy** — no PDFs in `references/` for
cross-check; a source-reading completion of the previous NewTOS spec's
behaviour coverage for `objc_change`/`form_button`.

**Decision:** `docs/api/aes/object-form.md` gained a "D-093a" callout under
`objc_change (47)`, a "D-093b" callout under `form_button (56)`, and edge
cases #21/#22.

---

## D-094: `wind_set(WF_NAME/WF_INFO/WF_NEWDESK)`'s string/tree pointer travels packed in `int_in[2..3]`, not via `addr_in[0]` — the `WM_IOTITLE` macro is dead code

**Date:** 2026-07-10 (AES leg, Window/Graphics Manager re-validation, Batch 4)
**Phase 00b spec:** `docs/api/aes/window-graphics.md` (`wind_set` (105),
Trap interface, Inputs/Outputs summary, NewTOS implementation notes)

**Context:** third validation round of Window/Graphics Manager, tasked with
the same rigour that found D-085 (menu tree topology) and D-089 (border-sign
inversion). Read the `WIND_SET`/`WIND_GET` dispatch arms (`gemsuper.c:342-347`)
and `wm_set`/`wm_get` (`gemwmlib.c:1505,1623`) in full, beyond the previously
validated slot tables.

**Previous spec (wrong):** stated that for `wind_set`'s `WF_NAME`/`WF_INFO`/
`WF_NEWDESK` fields "the pointer is in `addr_in[0]`" (citing the `WM_IOTITLE
= addr_in[0]` macro at `aesdefs.h:502`), in several places (the AESPB slot
table, the value-slots note, the per-field table, the Inputs/Outputs summary,
and the NewTOS implementation notes' `addr_in[]` bullet).

**EmuTOS (confirmed):** the `WIND_SET` dispatch arm calls
`wm_set(WM_HANDLE, WM_WFIELD, &WM_IX)` — **three** arguments only
(`gemsuper.c:345-347`); `wm_set`'s prototype is `BOOL wm_set(WORD w_handle,
WORD w_field, WORD *pinwds)` (`gemwmlib.c:1623`) — it never receives an
`addr_in` array. `WM_IX` = `int_in[2]`, so `pinwds = &int_in[2]`. Inside,
`case WF_NAME: gl_aname.te_ptext = pwin->w_pname = *(char **)pinwds;` and the
analogous `WF_INFO` case (`gemwmlib.c:1660-1674`) read the string pointer by
reinterpreting `int_in[2]`/`int_in[3]` (two `WORD`s) as one packed `LONG`
pointer. `WF_NEWDESK` does the same for the tree pointer (`*(OBJECT
**)pinwds`) plus `pinwds[2]` (`= int_in[4]`) for the root index
(`gemwmlib.c:1685-1689`). The `AESPB` struct itself has `intin`/`intout`/
`addrin`/`addrout` as four **separate** arrays (`gemsuper.h:13-20`), and the
`xif()` trampoline copies each into its own local buffer (`gemsuper.c:428-448`)
— `int_in[2..3]` and `addr_in[0]` are genuinely different memory. Grepping the
entire EmuTOS tree for `WM_IOTITLE` finds exactly one hit: its own
`#define` at `aesdefs.h:502` — it is never referenced by any dispatch arm,
confirming it is vestigial/dead code, not the actual mechanism.
`wind_get` has no case for `WF_NAME`/`WF_INFO`/`WF_NEWDESK` at all (falls to
`default: return FALSE`), so this only affects `wind_set`.

**Contract impact:** this is a hard ABI detail a real caller (or a compatible
NewTOS binding) must get right — the previous spec directed implementers to
populate `addr_in[0]`, which `wm_set` never reads; the actual, working
mechanism is a packed 32-bit pointer occupying two `int_in` words (the high
word first, matching m68k big-endian LONG layout), exactly like a rect's x/y
occupy two `int_in` words elsewhere in the same ABI.

**Not a Compendium discrepancy** — no PDFs in `references/` for cross-check;
a source-reading correction of a previous spec-writing error (the spec
apparently trusted the `WM_IOTITLE` macro's name/comment instead of tracing
the actual call site — "comments/macro names lie, code doesn't").

**Decision:** `docs/api/aes/window-graphics.md` corrected in six places (the
`wind_set` AESPB slot-table row, the value-slots note, a new "D-094" callout
+ corrected per-field table rows under `wind_set (105)`, the Inputs/Outputs
summary, the NewTOS implementation notes' `addr_in[]` bullet, and a new edge
case #14). NewTOS follows EmuTOS: the string/tree pointer for `wind_set`
`WF_NAME`/`WF_INFO`/`WF_NEWDESK` must be packed into `int_in[2..3]`.

---

## D-095: `wind_get(WF_COLOR/WF_DCOLOR)` writes the two colour words one slot later than previously documented — `int_out[2]`/`int_out[3]`, not `int_out[1]`/`int_out[2]`

**Date:** 2026-07-10 (AES leg, Window/Graphics Manager re-validation, Batch 4)
**Phase 00b spec:** `docs/api/aes/window-graphics.md` (`wind_get` (104),
`WF_*` field numbers)

**Context:** same pass as D-094; traced the exact `poutwds` indices `wm_get`
writes for `WF_COLOR`/`WF_DCOLOR`, rather than trusting the general "rect/
scalar" slot summary.

**Previous spec (wrong):** "`WF_COLOR` (18): `int_out[1]`=topped, `[2]`=
untopped colour for gadget `int_in[0]`" (and the analogous claim for
`WF_DCOLOR`).

**EmuTOS (confirmed):** `wm_get`'s `WF_COLOR` case writes `poutwds[1] =
pwin->w_tcolor[gadget]; poutwds[2] = pwin->w_bcolor[gadget];`
(`gemwmlib.c:1567-1571`; `WF_DCOLOR` analogous at `:1572-1576`) — it never
writes `poutwds[0]`. The dispatch arm calls `wm_get(WM_HANDLE, WM_WFIELD,
&WM_OX, &WM_IX)` with `poutwds = &WM_OX = &int_out[1]` (`gemsuper.c:343`), so
`poutwds[1]` is `int_out[2]` and `poutwds[2]` is `int_out[3]` — one slot
later than the previous spec stated. `int_out[1]` (`poutwds[0]`) is left
untouched; per the source's own doc comment (`gemwmlib.c:1494-1503`), the C
binding stages the gadget-number *input* in that same slot, which is simply
not overwritten on output. Also corrected: the gadget number is read from
`pinwds[0]` = `int_in[2]`, not `int_in[0]` (`int_in[0]` is the window handle)
— the previous spec's "for gadget `int_in[0]`" was off by the same kind of
index slip.

**Not a Compendium discrepancy** — no PDFs in `references/` for cross-check;
a source-reading index correction.

**Decision:** `docs/api/aes/window-graphics.md` corrected: the `WF_*` field
numbers §-footnote, the `wind_get (104)` per-field table rows for
`WF_COLOR`/`WF_DCOLOR`, a new "D-095" callout, and edge case #15.

---

## D-096: `graf_watchbox`'s return-value direction was documented backwards — `out == TRUE` on inside-release, `FALSE` on outside-release (not the reverse)

**Date:** 2026-07-10 (AES leg, Window/Graphics Manager re-validation, Batch 4)
**Phase 00b spec:** `docs/api/aes/window-graphics.md` (`graf_watchbox` (75),
edge case #8)

**Context:** same pass as D-094/D-095; the section already carried a
"Return-value precision" callout from an earlier round that itself stated the
correct direction, but the lead sentence directly above it stated the
opposite — an internal contradiction that a careful re-read of `gr_watchbox`
resolves in favour of the callout, not the lead sentence. This is exactly the
class of load-bearing sign/direction error D-089 (border-thickness sign) was.

**Previous spec (wrong lead sentence):** "`out` is `FALSE` (0) when the
button was released *inside* the object, `TRUE` (1) when outside" — while the
"corrects" callout immediately below it said the opposite (`TRUE` = inside),
without the contradiction ever being resolved.

**EmuTOS (confirmed):** `gr_watchbox` (`gemgrlib.c:337-355`): `out` starts
`FALSE`. Loop iteration 1 draws `instate` (`state = out ? outstate :
instate`, `out` is `FALSE`), *then* flips `out = !out` to `TRUE`, *then* calls
`gr_stilldn(out, …)` which blocks for either a button-state change or a
mouse boundary crossing. In the ordinary single-click case — button released
before the mouse ever leaves the gadget's rect — `gr_stilldn` detects the
button change first and returns `FALSE`; the `do…while` loop condition is
`FALSE` and exits immediately, with `out` still holding the `TRUE` value it
was flipped to just before the wait. Only if the mouse first leaves the rect
(second iteration draws `outstate`, flips `out` back to `FALSE`) and *then*
the button comes up does the function return `FALSE`. This matches
`gemctrl.c:218`'s `if (gr_watchbox(gl_awind, gadget, selected, normal))`,
which fires `WM_CLOSED`/`WM_FULLED` exactly when the call returns non-zero
— i.e. on inside-release. **`TRUE` = released inside; `FALSE` = released
outside.**

**Not a Compendium discrepancy** — no PDFs in `references/` for cross-check;
a source-reading correction of an unresolved internal contradiction left over
from an earlier spec round.

**Decision:** `docs/api/aes/window-graphics.md` corrected: the `graf_watchbox
(75)` section's lead sentence and callout merged into one "D-096" callout
with the traced step-by-step derivation, and edge case #8 rewritten.

---

## D-097: non-3D `wind_calc`'s title-band border test is `kind & (NAME|CLOSER|FULLER)`, not the `TGADGETS` macro (which also includes `MOVER`)

**Date:** 2026-07-10 (AES leg, Window/Graphics Manager re-validation, Batch 4)
**Phase 00b spec:** `docs/api/aes/window-graphics.md` (`wind_calc` (108),
"Window-kind gadget bits")

**Context:** same pass as D-094–D-096; cross-checked the "Non-3D build" border
table against the actual `wm_calc` literal, since the 3D build's row
correctly uses `TGADGETS` and the previous spec generalised that label to the
non-3D row too.

**Previous spec (wrong):** the non-3D `wind_calc` border table stated the top
border grows "if `TGADGETS` (title band)".

**EmuTOS (confirmed):** the non-3D branch of `wm_calc` (`gemwmlib.c:1841-1853`,
inside `#else` for `CONF_WITH_3D_OBJECTS`) tests `if (kind & (NAME|CLOSER|
FULLER)) tb += (gl_hbox - 1);` — explicitly **not** the `TGADGETS` macro
(`NAME|CLOSER|FULLER|MOVER`, `gemwmlib.h:16`). This exactly mirrors
`w_bldactive`'s own non-3D branch, which tests the identical
`kind & (NAME|CLOSER|FULLER)` literal (`gemwmlib.c:750`) and has no
`MOVER`-without-`NAME` special case (unlike the 3D branch's `else if (kind &
MOVER)` at `gemwmlib.c:542-551`). The 3D branch of `wm_calc`
(`gemwmlib.c:1818`) does correctly use `kind & TGADGETS`. Consequence: in a
non-3D build, a `MOVER`-only window (no `NAME`/`CLOSER`/`FULLER`) reserves
**no** title band at all, in either `w_bldactive` or `wind_calc`/`wm_calc`.

**Impact:** low — `CONF_WITH_3D_OBJECTS` defaults to 1 in the reference
build, so this only matters for a non-default build variant. Recorded for
completeness and because the previous spec stated it as fact.

**Not a Compendium discrepancy** — no PDFs in `references/` for cross-check;
a source-reading correction.

**Decision:** `docs/api/aes/window-graphics.md` corrected: the non-3D border
table row, a new "D-097" callout, and the `MOVER`-without-`NAME` edge case
under "Window-kind gadget bits" now scoped explicitly to the 3D-object build.

---

## D-098: the "slider creep" rounding fix (`mul_div` → `mul_div_round`) was never applied to the 3D-object (default) build's `w_bldvbar`/`w_bldhbar` — a genuine, still-present EmuTOS source gap

**Date:** 2026-07-10 (AES leg, Window/Graphics Manager re-validation, Batch 4)
**Phase 00b spec:** `docs/api/aes/window-graphics.md` (`wind_set` (105),
edge case #13)

**Context:** the task briefing for this round flagged a known historical
"H-slider fix" to check against the current `wind_set` spec. Located it via
`git log`: commit `a04ebb02` ("Fix 'slider creep' in AES windows", 2021-07-19,
Roger Burrows) — "If you single click on the vertical or horizontal slider in
a window, it will often move upwards/leftwards even if you haven't moved the
mouse. This is because the translation from slider position to pixel
position and back again does not yield the original result, because the
`mul_div()` function does not round." Checked whether the fix is present, in
full, in the pinned commit's source (not just referenced by name).

**EmuTOS (confirmed):** commit `a04ebb02` replaced `mul_div` with
`mul_div_round` in three places: `gr_slidebox` (`gemgrlib.c:410`, drag →
stored 0..1000 value), the file selector's elevator math
(`gemfslib.c`, out of scope here), and the **non-3D** `w_bldvbar`/`w_bldhbar`
(`gemwmlib.c:667-669,708-710`, stored value → pixel geometry). `git log
--follow` on `aes/gemwmlib.c` shows the `#if CONF_WITH_3D_OBJECTS` variant of
`w_bldvbar`/`w_bldhbar` was introduced *later* by commit `dfa61fe9` ("Add 3D
object support"), which duplicated the *pre-fix* pattern: its
`w_bldvbar`/`w_bldhbar` (`gemwmlib.c:433-434,471-472`, the code path active
in the default `CONF_WITH_3D_OBJECTS=1` build) still call plain `mul_div`,
never `mul_div_round`. Confirmed directly against the pinned commit
`1a7484ab`'s checked-out source (not just the historical diff): the 3D
branch's `size = max(gl_hbox, mul_div(h, pw->w_vslsiz, 1000)); posn =
mul_div(h-size, pw->w_vslide, 1000);` is unrounded. Net effect in the
*default* EmuTOS build: a slider drag is converted to a stored value with
rounding (`gr_slidebox`), but the elevator's redrawn pixel geometry from that
same stored value is computed without rounding (`w_bldvbar`/`w_bldhbar`,
3D variant) — the "slider creep" symptom the 2021 fix targeted can still
occur in the reference build, just one layer removed from where it was
originally observed and fixed.

**Scope:** this sits entirely inside `w_bldactive`/`w_bldvbar`/`w_bldhbar`,
which this document already marks **reference** (NewTOS reimplements the
frame-drawing over its own object manager) — not **contract**. The contract
surface (`WF_HSLIDE`/`WF_VSLIDE`/`WF_HSLSIZ`/`WF_VSLSIZ` values, always
1..1000 or −1) is unaffected. Classed like D-081/D-082: a genuine EmuTOS
source-level gap, not a spec-reading error.

**Not a Compendium discrepancy** — no PDFs in `references/` for cross-check;
a genuine source-level finding from `git log`/source cross-reference, not a
documentation disagreement.

**Decision:** `docs/api/aes/window-graphics.md` gained a "D-098" callout
under `wind_set (105)`'s slider-value-validation paragraph and edge case #13.
NewTOS's own elevator geometry may round consistently rather than reproduce
the residual creep — the choice is free (reference-level), not mandated
either way.

---

## D-099: NewTOS deviation — `appl_init` reports `ap_private = 0`, not `rs_trees[DESKTOP][ROOT].ob_spec`, because NewTOS's AES has no DESKTOP resource tree

**Date:** 2026-07-10 (AES leg, Window/Graphics Manager re-validation, Batch 4
— `ap_private` follow-up from the App/Event-Manager fix)
**Phase 00b spec:** `docs/api/aes/application.md` (`appl_init` (10),
`AESGLOBAL`, Verification criteria)

**Context:** the lead flagged that NewTOS's `appl_init` sets `ap_private = 0`
where EmuTOS sets `rs_trees[DESKTOP][ROOT].ob_spec` (`gemsuper.c:95-96`,
already correctly documented per D-… the `ap_private`/`ap_ptree` correction in
`application.md`). This entry records the *reason* this is an intentional
NewTOS deviation, not an unimplemented EmuTOS behaviour, and downgrades the
matching verification criterion.

**EmuTOS:** `appl_init` writes `AP_PRIVATE` (`global[3]`) with
`rs_trees[DESKTOP][ROOT].ob_spec` — the `ob_spec` of the root object of the
**DESKTOP** resource tree, one of several trees EmuTOS keeps permanently
resident (built into the ROM ".rsc" ensemble, `rs_trees[]`,
`aes/gem_rsc.h`/`obdefs.h`), used by EmuDesk to encode the desktop
background pattern/colour.

**NewTOS (deviation, by design):** NewTOS's AES kernel has **no** DESKTOP
resource tree at all — EmuTOS's `rs_trees`/permanent in-ROM `gem.rsc`
ensemble does not exist on NewTOS. The only resource NewTOS's AES loads is
`GEM.RSC`, and only **lazily**, on first use of the file selector (tree 0 =
`FSELECTR`); there is no tree slot analogous to EmuTOS's `DESKTOP` index.
What EmuTOS encodes in that tree's root `ob_spec` (the desktop background
pattern/colour) lives on NewTOS as `g_patcol` **inside the desktop client
application**, strictly **above** the AES/syscall layer — the AES kernel
(where `appl_init`'s dispatch arm runs) has no reference to it and cannot
reach it. `appl_init` therefore reports `ap_private = 0` on NewTOS: there is
no non-zero value it could honestly report from the syscall layer.

**Decision:** NewTOS deviates from EmuTOS here by necessity of its
architecture (no kernel-side DESKTOP tree), not by oversight. Documented in
`docs/api/aes/application.md`:
- a "NewTOS deviation" callout added directly under the `ap_private`/
  `ap_ptree` correction note (in the `AESGLOBAL` section),
- `appl_init (10)`'s behaviour list annotated at the `AP_PRIVATE` bullet,
- the "`appl_init` private" verification criterion (previously stated as an
  unconditional check) downgraded to explicitly flag it as **not
  satisfiable on NewTOS** (deviation, not an open bug) with a cross-reference
  to this entry,
- a new "NewTOS deviation" bullet added under "NewTOS implementation notes"
  giving the full rationale (no `rs_trees`, `GEM.RSC` lazy-loaded FSELECTR
  only, `g_patcol` lives in the desktop client above the syscall boundary).

**CLOSED 2026-09-01 (desktop-user-space stage S1b) — the deviation is
withdrawn; NewTOS now matches EmuTOS exactly.** The premise above was wrong on
one point: `rs_trees[DESKTOP]` is the **AES's own** compiled-in `gem.rsc`
ensemble (`aes/gem_rsc.c:550-570,574-578`, index 2 per `aes/gem_rsc.h:48`), not
anything the desktop client owns — so nothing about it ever needed the desktop
or a downward reach across the syscall boundary. NewTOS carries that three-object
tree as a static in `tos-api::aes::init`; its ROOT's compiled-in `ob_spec` is the
literal `4419L` = `0x1143`, and AES boot replaces only its low byte from the
`.INF` file's `#Q` line (`set_aes_background`, `geminit.c:635-641`, driven by
`process_inf1`, `:343-392`, `:719-728`). `appl_init` reads it verbatim
(`gemsuper.c:95-96`). A real TT running EmuTOS was measured at `ap_private =
0x1143`, which is now what NewTOS reports with no `#Q` line present.

An intermediate revision (2026-07-22) sourced the value from the
**WF_NEWDESK-registered** desktop tree instead; that was also wrong — such a
tree lands in `gl_newdesk` (`aes/gemwmlib.c:1687`), is read only by
`w_drawdesk`/`w_redraw_desktop`, and never reaches `ap_private`. Both the spec
(`docs/api/aes/application.md`: the `ap_private` callout, the `appl_init`
field list, the verification criterion and the implementation note) and the
implementation were corrected in the same change; the removed helper was
`tos-api::aes::window::desktop_bg_obspec()`.

---

## D-100: `resource-fileselector.md` labelled EmuTOS's classic on-disk `.RSC` format as NewTOS "contract" — it contradicts the already-accepted ADR-013 (native format); the GEM.RSC lazy-load fact D-099 forward-referenced was never actually documented there

**Date:** 2026-07-10 (AES leg, Resource + Fileselector re-validation, third
round — tasked with the same rigour that found D-085 (menu tree topology),
D-089 (border-sign inversion), and D-094 (window-set pointer packing))
**Phase 00b spec:** `docs/api/aes/resource-fileselector.md` (intro
Contract/Reference block, `rsrc_load (110)`, `rsrc_obfix (114)`,
`fsel_input (90)`/`fsel_exinput (91)`, Verification criteria, Configuration
notes, NewTOS implementation notes)

**Context:** every EmuTOS-source claim in the existing spec (the `RSHDR`
byte layout, the `R_*` type-code table 0–16 incl. the 8/11/14 aliasing, the
`rs_obfix`/`fix_chpos` signed-low-byte + width-80 rule, the `rsrc_gaddr`-only
`addr_out[0]` copy-back, the `fsel_*` `int_out[1]` button convention) was
re-verified line-by-line against `aes/gemrslib.c`, `aes/gemfslib.c`,
`aes/gemsuper.c`, `include/aesdefs.h`, `include/rsdefs.h`, and
`aes/gemrslib.h` and found **correct** — the prior "full-contract-refinement"
pass (D-054) already got these right, so this round found no EmuTOS
source-misreading error of the D-085/D-089/D-094 kind. Instead, checking the
spec against NewTOS's *own* accepted architecture (as the task explicitly
asked for the GEM.RSC-lazy-load fact) found a different, still substantial,
problem: the spec's "NewTOS implementation notes" section instructed
"Preserve the `sh_find` lookup, the `-1L`-stays-`-1L` rule, the
box-`ob_spec`-not-relocated rule, and the signed-low-byte + width-80
coordinate rule **exactly**" (old wording) — but ADR-013 (`docs/MEMORY.md`,
accepted **2026-06-21**, i.e. *before* this spec's last full-refinement pass
on **2026-07-03**) already committed NewTOS to a **native** `.RSC` format
that does none of that. The spec's own "Contract" list in the intro
block also stated "the `.RSC` on-disk file format (the `RSHDR` byte
layout...)" and "the `rs_obfix` char→pixel coordinate rule" as contract
outright, with no NewTOS-deviation qualifier — directly contradicting
ADR-013. Separately, `docs/api/aes/application.md`'s D-099 entry
(`gemsuper.c:95-96` `ap_private` deviation, written the same day as this
round) explicitly says "the only resource the AES loads is `GEM.RSC`, loaded
**lazily** on first use of the file selector (`resource-fileselector.md`,
tree 0 = `FSELECTR`)" — forward-referencing this very spec as the place that
documents the lazy-load mechanism. It did not: `resource-fileselector.md`
had **no mention** of GEM.RSC being a second, separately-loaded,
lazily-cached resource distinct from the app's own `rsrc_load`.

**Verified against the current NewTOS implementation** (read-only, no code
changed): `tos-api/src/aes/resource.rs` (`NrscHeader`, magic `"NRSC"`,
little-endian, `relocate_and_register`/`relocate_blob`, `rsrc_load` trying
`C:\GEMSYS\RSC\<name>` then `C:\<name>` — not `sh_find` — and `rsrc_obfix`
being a bare `pb.set_ret(1)` identity) and `tos-api/src/aes/fsel.rs`
(`load_fselectr`: loads `C:\GEMSYS\RSC\GEM.RSC` into `SYS.buf` on first call,
caches the resolved tree root in `SYS.root`, returns the cached pointer on
every later call). Also noted in passing (implementation-status finding, not
a spec-writing error, and **not fixed here** — Researcher role is docs-only
per CLAUDE.md): `rsrc_gaddr`/`rsrc_saddr` currently implement only
`R_TREE`/`R_OBJECT` (types 2–16 fall through to "not found"), and
`rsrc_saddr` (`tos-api/src/aes/resource.rs::rsrc_saddr`) unconditionally
returns `0` regardless of input — a no-op stub in the sense CLAUDE.md Rule 2
forbids. Flagged for the lead/Developer; out of scope for this pass.

**Not a Compendium discrepancy, and not an EmuTOS-source misreading** — the
EmuTOS extraction in §0–§3 of `resource-fileselector.md` is accurate and
remains the right reference for a future classic-`.RSC`-compatible loader
(explicitly anticipated by ADR-013's own "alternatives rejected" section, for
an optional m68k-emulator phase). The bug was the spec's failure to carry the
already-accepted ADR-013 decision into its own Contract/Reference framing and
"NewTOS implementation notes" — the same class of gap D-099 caught for
`ap_private`, just on the resource-loading side, and directly undermining the
sibling spec's forward-reference.

**Decision:** `docs/api/aes/resource-fileselector.md` corrected in seven
places: (1) a new "NewTOS deviation (ADR-013, D-100)" callout in the intro
block; (2) a "NewTOS status" callout under `rsrc_load (110)` describing the
actual `NrscHeader`/fixed-path-search mechanism; (3) one under
`rsrc_obfix (114)` stating it is an identity; (4) one under
`fsel_input (90)`/`fsel_exinput (91)` describing the GEM.RSC lazy-load +
cache mechanism (closing the D-099 forward-reference gap); (5) an
applicability footnote under "Verification criteria" flagging the two
criteria (`rsrc_obfix` coordinate fix, `sh_find`-based not-found) that do not
hold against the current native-format implementation; (6) a correction to
the `CONF_WITH_COLOUR_ICONS` config note's "the RSC format + relocation are
contract" claim; (7) the "NewTOS implementation notes" section's stale
"preserve … exactly" bullet replaced with the accurate native-format status,
plus new bullets for the `R_*` coverage gap, the `rsrc_saddr` stub, and the
`FSELECTR` tree provenance. NewTOS's contract remains: the `rsrc_gaddr`/
`rsrc_saddr` ABI slots, the `R_*` type semantics (once fully implemented),
the `1`/`0` return convention, and the `fsel_*` path/mask-split +
folders-first + `int_out[1]` button contract. The on-disk byte format,
`sh_find`, and the char→pixel `rs_obfix` rule are EmuTOS-reference only,
per ADR-013.

---

## D-101: `shel_write`'s `SHW_RESCHNG` mode carries a hidden `+2` offset on `isgem` for ST/TT-family resolution requests (Falcon requests do not); `SHW_SHUTDOWN`'s `isover` parameter is part of the documented ABI but is never read by the current implementation

**Date:** 2026-07-11 (AES leg, Scrap + Shell re-validation, third and final
round — tasked with the same rigour that found D-085 (menu tree topology),
D-089 (border-sign inversion), D-094 (window-set pointer packing), and D-100
(resource-format contract framing))
**Phase 00b spec:** `docs/api/aes/scrap-shell.md` (`shel_write (121)` mode
table + corrections note, Verification criteria, Edge cases discovered in
source, NewTOS implementation notes)

**Context:** every ABI-slot-level claim in the existing spec (the four
`shel_write` modes, the `isgem`/`isover` naming, the `shel_envrn`
`addr_in[0]`/`[1]` swap, the always-1 returns, the buffer sizes) — all
established by the prior full-contract-refinement pass, D-054 — was
re-verified against `aes/gemsclib.c`/`aes/gemshlib.c` @ `1a7484ab` and found
**correct**; this round found no slot-level misreading. Digging one level
deeper than D-054 (into the *values* the ABI slots carry, not just which slot
they occupy) surfaced a genuine gap: the source comment documenting
`sh_write`'s `doex = 5` (`SHW_RESCHNG`) mode (`aes/gemshlib.c:131-138`) reads:

```
 *      (1) doex = 5
 *          . selects next video resolution
 *              (a) isover = 0
 *                  . indicates ST/TT-style video
 *                      isgem = 2 + value used by Getrez()/Setscreen()
 *              (b) isover = 1
 *                  . indicates Falcon-style video)
 *                      isgem = value used by Setscreen() for video mode
```

i.e. **for `isover = 0` (ST/TT family) the caller must add 2 to the real
Setscreen mode value before passing it as `isgem`; for `isover = 1` (Falcon
family) `isgem` is the raw value, no offset.** `sh_write`'s actual code
(`gemshlib.c:187-192`) does not apply or remove this offset itself — it just
stores `gl_changerez = 1 + isover; gl_nextrez = isgem;` verbatim. The `+2` is
the *caller's* job to add and the *consumer's* job to subtract:
- **Consumer**, `aes/geminit.c:778-793` (the post-`.INF` resolution-apply
  code, shared by both the boot-time `.INF` `#E`-line path and the
  `shel_write`-driven path since both write the same `gl_changerez`/
  `gl_nextrez` globals): `case 1` (ST/TT, `gl_changerez==1`) does
  `Setscreen(-1L, -1L, gl_nextrez-2, 0)` (`:782`) — subtracting the `2` back
  off; `case 2` (Falcon, `gl_changerez==2`) does `Setscreen(0L, 0L,
  FALCON_REZ, gl_nextrez)` (`:788`) — no offset.
- **Caller**, EmuTOS's own resolution-change dialog,
  `desk/deskmain.c:900-914` (`do_optnmenu`/`RESITEM`): for a Falcon mode,
  `shel_write(SHW_RESCHNG, newmode, 1, NULL, NULL)` (`:910`, no offset); for
  an ST/TT mode, `shel_write(SHW_RESCHNG, newres+2, 0, NULL, NULL)` (`:913`,
  **`+2` added explicitly by the caller**) — an independent, real EmuTOS
  call site confirming the comment is not aspirational.
- Corroborating evidence: `aes/geminit.c:343-392` (`process_inf1`, the
  `.INF`-file `#E`-line resolution parser, a *different* producer of the same
  `gl_changerez`/`gl_nextrez` globals) independently applies the same `+2`
  when building an ST/TT `gl_nextrez` (`mode = (mode & 0x00ff) + 2;`, `:380`)
  but not for a Falcon one (`gl_nextrez = mode;`, `:375`) — the same
  convention, reached by a wholly separate code path.

The prior spec's `shel_write` mode table collapsed both branches into one
cell — `isgem` = "Setscreen video-mode value" — losing the family-dependent
`+2`. This matters as more than documentation polish: **a well-formed GEM
application computing `isgem` per the documented convention and an AES
implementation that consumes `gl_nextrez` without the matching `+2`/`-2`
round trip will resolve to the wrong video mode for every ST/TT-family
request** — a real ABI/behavioural mismatch, not merely a missing detail.

Separately, re-reading the `SHW_SHUTDOWN` (`doex = 4`) case
(`gemshlib.c:177-186`) against its own documented `isover` semantics
(`gemshlib.c:140-145`: "isover = 0 abort … / 1 partial … / 2 full") found
that **the C implementation never reads `isover` at all** — it only checks
`can_shutdown()`. So while `isover` is part of the call's documented ABI, it
is currently inert: `isover = 0` ("abort") does not abort anything (if
`can_shutdown()` is true, shutdown is recorded exactly as for `isover = 2`);
"partial" (`1`) and "full" (`2`) are not currently distinguished at all — the
source comment itself concedes this ("currently the same as full"). EmuTOS's
own caller (`desk/deskmain.c:776`, the desktop's `QUITITEM`) always passes
`isover = 2`, consistent with this being effectively a fixed "full or
nothing" call in practice. The old spec's mode table reproduced the 0/1/2
gloss without noting the implementation gap.

Two smaller precision fixes bundled into the same pass: (1) the spec's
Error-conditions note said the launch loop shows a load failure "via
`form_error`" — the actual internal function is `fm_error()`
(`aes/gemfmlib.c:436`); there is no AES opcode named `form_error`. (2) the
`SHEL_WRITE` dispatcher case (`aes/gemsuper.c:383-384`) reveals `sh_write`'s
`CONSOLE_APP` dispatch in `sh_ldapp` is itself gated `#if WITH_CLI`
(`gemshlib.c:614-622`), a Makefile-level flag defaulting to 1
(`Makefile:150`, off only for the memory-constrained "192" target,
`Makefile:534`) — not previously noted; with it off, `shel_write(SHW_EXEC,
…, "EMUCON", …)` still classifies the next app as `CONSOLE_APP` (that
assignment is unconditional in `sh_write`), but the launch loop has no
matching dispatch arm and falls through to the normal-app path, which fails
to find "EMUCON" as a file (`EFILNF`).

**Not a Compendium discrepancy** — no PDFs in `references/`; Compendium
cross-check of the `+2` `SHW_RESCHNG` offset convention and the `SHW_SHUTDOWN`
`isover` semantics remains deferred, as for the rest of `scrap-shell.md`
(see the existing Compendium/Profibuch note in that file, itself deferred
under D-054).

**Decision:** follow EmuTOS exactly — `shel_write(SHW_RESCHNG, isgem, isover,
…)` requires `isgem = 2 + <Setscreen mode>` when `isover = 0` and `isgem =
<Setscreen mode>` (no offset) when `isover = 1`; any NewTOS consumer of
`gl_nextrez`-equivalent state must apply the matching `-2`/`-0` when deriving
the real video mode. `SHW_SHUTDOWN`'s `isover` need not be honoured by NewTOS
beyond the existing `can_shutdown()`-equivalent gate — EmuTOS's own
implementation does not read it either; a NewTOS deviation that adds real
partial/full shutdown levels would not break compatibility (there is no
EmuTOS-observed distinct behaviour to preserve). Corrected in
`docs/api/aes/scrap-shell.md`: the `shel_write (121)` mode-flag table (both
the `SHW_RESCHNG` `isgem` cell and the `SHW_SHUTDOWN` `isover` cell), a new
"third-round re-validation" callout in the intro block, an expanded
"Corrections vs. the previous spec" note, the `SHW_RESCHNG` verification
criterion, four new/renumbered "Edge cases discovered in source" bullets
(the `+2` offset, the dead `SHW_SHUTDOWN.isover`, the `WITH_CLI`-gated
`EMUCON` dispatch, unrecognised `doex` values being silently ignored), the
`form_error`→`fm_error` naming fix in "Error conditions", and two new "NewTOS
implementation notes" bullets. **Scrap + Shell was the last outstanding
batch assigned for this re-validation initiative.** With it done, every
`docs/api/aes/*.md` manager spec has been checked against current EmuTOS
source at least once beyond the original D-054 full-contract-refinement pass
(Menu: D-085 through D-088; Object/Form: D-089 through D-093; Window/
Graphics: D-094 through D-098, plus the D-099 `ap_private` follow-up;
Resource/File selector: D-100; Scrap/Shell: this entry). Application- and
Event-Manager's last dedicated pass remains the 2026-07-03 batch (D-049/
D-050, plus the `appl_init` fix D-099 references) — no source-misreading was
reported against them in this later round.

---

## D-102: `wind_set(WF_NAME/WF_INFO/WF_NEWDESK)`'s pointer is `addr_in[0]` on the NewTOS *native* ABI — D-094's `int_in[2..3]` packing is the 68k form only, not the NewTOS contract

**Date:** 2026-07-12 (AES leg, Window/Graphics code audit — spec-vs-code
cross-check)
**Phase 00b spec:** `docs/api/aes/window-graphics.md` (NewTOS implementation
notes, `addr_in[]`/pointer-width bullet)

**Context:** a Window/Graphics implementation audit compared the spec's
guidance for `wind_set`'s `WF_NAME`/`WF_INFO`/`WF_NEWDESK` pointer against the
actual NewTOS code and found a contradiction: the spec's "NewTOS
implementation notes" bullet (added by D-094) instructed implementers to read
this pointer "packed as a raw LONG across `int_in[2..3]`", but
`tos-api/src/aes/window.rs::wind_set` already reads it from `pb.addrin(0)` —
the same slot every other AES pointer argument uses (`tos-api/src/aes/
menu.rs`, `object.rs`, `appl.rs`). D-094 was correct about EmuTOS/68k
behaviour; the error was carrying that 68k-ABI fact over into the
*NewTOS-native* instruction, where it does not apply.

**EmuTOS (confirmed, same citations as D-094):** `wm_set(WORD w_handle, WORD
w_field, WORD *pinwds)` (`gemwmlib.c:1623`) is dispatched from `WIND_SET` as
`wm_set(WM_HANDLE, WM_WFIELD, &WM_IX)` (`gemsuper.c:345-346`), where `WM_IX =
int_in[2]`; `WF_NAME`/`WF_INFO` read `*(char **)pinwds` and `WF_NEWDESK` reads
`*(OBJECT **)pinwds` (`gemwmlib.c:1660-1689`) — a 32-bit pointer packed across
two 16-bit `int_in` words, because the AESPB's `int_in` array element width is
a `WORD` on the m68k ABI. This is genuinely how the 68k-callable `wind_set()`
binding packs the argument, and D-094 documents it correctly for that world.

**Why it does not carry over to native NewTOS:** per `docs/COMPATIBILITY.md`
("Native vs m68k struct variants" — pointer width follows the caller ABI —
and the audit rubric's caller-type-branching rule), a native AArch64 caller
has a 64-bit pointer that cannot round-trip through two 16-bit words at all.
NewTOS's actual, already-implemented native ABI does not attempt it: every
AES pointer-class argument, including this one, is read from a single
`addr_in[]` slot (`tos-api/src/aes/window.rs:869`, `pb.addrin(0)`; same
pattern throughout `menu.rs`/`object.rs`/`appl.rs`). The `int_in[2..3]`
packing is specific to the 68k parameter-block encoding of `wind_set`, not a
NewTOS contract requirement — the two ABIs disagree here by construction
(word-packed LONG vs. a native pointer slot), which is expected and is why
`docs/COMPATIBILITY.md` says pointer width, not pointer *location*, follows
the caller.

**Not a Compendium discrepancy** — an internal spec-vs-code consistency fix
found during implementation audit, not a source-misreading.

**Decision:** `docs/api/aes/window-graphics.md`'s "NewTOS implementation
notes" bullet (the `addr_in[]`/pointer-widths bullet, previously stating the
pointer "is *not* an `addr_in[]` pointer") corrected to state that on NewTOS
natively this pointer **is** `addr_in[0]`, with a one-line cross-reference to
D-094's `int_in[2..3]` finding labelled explicitly as the 68k form, so the two
facts are not mistaken for one another again. The document's description of
the 68k-callable contract elsewhere (the `wind_set` AESPB slot-table row, the
per-field table, the Inputs/Outputs summary, edge case #14) is unchanged — it
correctly documents the 68k ABI per D-094 and remains accurate for that
world; only the native-implementation guidance was wrong and is now fixed.

---

## D-103: `just_draw`'s `G_BUTTON` hollow-white interior condition is "not `FL3DOBJ`", not "not 3D" — a `FL3DBAK` button stays hollow white too

**Date:** 2026-09-20 (research task for TASK/A68–A70)
**Phase 00b spec:** `docs/api/aes/object-form.md` (`objc_draw` / `just_draw`
per-type rendering, item 2 "Box family" and the "3D handling in `just_draw`"
sub-list, item (c))

**Context:** TASK/A70 claims NewTOS's `object.rs` over-applies the "hollow
white/button interior takes the 3D background colour" rule to every 3D flag
combination, where EmuTOS restricts it to `FL3DOBJ` (indicator/activator)
and leaves a `FL3DBAK` `G_BUTTON` hollow white. The spec's own item 2 (Box
family) description — "`G_BUTTON` forces `bcol = BLACK` and a hollow-white
interior (non-3D)" — was consulted to check this and found to be worded
imprecisely enough to read as supporting the *wrong* premise (that hollow
white applies "when not 3D", implying any 3D flag, including `FL3DBAK`,
would leave that regime).

**EmuTOS (confirmed by full trace):** `G_BUTTON`'s colours are set in the
*second* `obtype` switch of `just_draw`, `case G_BUTTON:`
(`aes/gemoblib.c:639-658`): `bcol = BLACK` (`:642`); `if (flags &
FL3DOBJ)` (`:647`) → `ipat = IP_SOLID`, `icol = actbutcol`/`indbutcol`
(`:649-650`); **else** (`:652-657`) → `ipat = IP_HOLLOW`, `icol = WHITE`.
`FL3DOBJ` (`include/obdefs.h:92`) is bit `0x0200`; `FL3DBAK`
(`obdefs.h:97`) is the *disjoint* bit `0x0400`; `FL3DIND = 0x0200`,
`FL3DACT = 0x0600` (`obdefs.h:95-96`). So `flags & FL3DOBJ` is true only
for `FL3DIND`/`FL3DACT` and **false for `FL3DBAK`** — a `FL3DBAK` button
takes the `else` branch exactly like a non-3D button. After
`FALLTHROUGH` (`:659`) into the shared box-drawing code
(`case G_BOXTEXT: case G_FBOXTEXT:`, `:660-695`), the only remaining
place `icol`/`ipat` could change before `gr_rect(icol, ipat, &t)`
(`:692`) is the `SELECTED`-gated `changecol` block (`:681-690`), which is
orthogonal to the 3D flag and irrelevant when the object is not selected.
There is no separate hollow-white → `backgrcol` rule reachable from the
`G_BUTTON` case; the rule with that shape (`gemoblib.c:582-585`) belongs to
the *first*, TEDINFO-only `obtype` switch (`G_TEXT`/`G_BOXTEXT`/`G_FTEXT`/
`G_FBOXTEXT`), which `G_BUTTON` never enters.

**Decision:** `docs/api/aes/object-form.md` item 2's `G_BUTTON` sentence
corrected to state the condition as "hollow white except under `FL3DOBJ`"
rather than "(non-3D)", with a full trace and the A70 verdict recorded in a
new "3D handling in `just_draw`" sub-list (item (c)) right after item 3.
TASK/A70's premise is **confirmed**: EmuTOS leaves an un-selected `FL3DBAK`
`G_BUTTON` hollow white; only `FL3DOBJ` buttons get the solid
indicator/activator fill. Not a Compendium/Profibuch discrepancy — a
spec-wording imprecision found while extending the spec for TASK/A68–A70,
corrected against the EmuTOS source directly.

---

## D-104: `just_draw`'s trailing `SELECTED` XOR is gated on `!(flags & FL3DOBJ) && !changecol`, not on "non-3D" — a `FL3DBAK` object is XOR-inverted, a `FL3DOBJ` object never is, and a `changecol` object is colour-swapped in the type body instead

**Date:** 2026-09-20 (TASK/A85 audit)
**Phase 00b spec:** `docs/api/aes/object-form.md` (`objc_draw` / `just_draw`
per-type rendering, item 8 "State decorations")

**Context:** Item 8 summarised the last state decoration as "`SELECTED`
(non-3D) → XOR-invert the whole rect". "(non-3D)" is the same imprecision
D-103 removed from item 2: it reads as "any 3D flag leaves this regime",
which is wrong in both directions — a `FL3DBAK` object *is* XOR-inverted,
and a non-3D object is *not* when `changecol` is set. NewTOS's `object.rs`
was written against the loose wording (no trailing XOR at all; a per-type
inline colour swap with `changecol = true` for every non-`FL3DOBJ` object).

**EmuTOS (confirmed by trace and by host test):** `aes/gemoblib.c:932-943`:

```c
if (state & SELECTED) {
    if (!(flags & FL3DOBJ) && !changecol)
        bb_fill(MD_XOR, FIS_SOLID, IP_SOLID, t.g_x, t.g_y, t.g_w, t.g_h);
}
```

`FL3DOBJ` = `0x0200` (`include/obdefs.h:92`) is carried by `FL3DIND`
(`0x0200`) and `FL3DACT` (`0x0600`), not by `FL3DBAK` (`0x0400`,
`obdefs.h:95-97`). `changecol` (`gemoblib.c:504-525`): for `FL3DOBJ` it is
`actcolchange` (`FL3DACT`) or `indcolchange` (`FL3DIND`) — `init_3d`
(`:1327-1343`) sets `indcolchange = 1`, `actcolchange = 0`, both
`objc_sysvar`-settable (`ob_sysvar`, `:1350-1369`); otherwise it is
`!xor_is_ok(obtype, flags, spec)` (`:524`), and `xor_is_ok` (`:107-148`)
is TRUE for `G_IBOX`/`G_STRING`/`G_TITLE`, FALSE for `G_IMAGE`, TRUE at
≤ 16 colours, and at more colours TRUE for the box/text families iff the
spec's interior and text colours are ≤ BLACK; `G_BUTTON`/`G_USERDEF`/icons
hit `default:` → TRUE. A `changecol` object is swapped *in the type body*
instead (`:681-690` interior, `:699-706` text colour + `MD_TRANS`, `:807-808`
button label WHITE); a `FL3DOBJ` object without `changecol` (an activator
by default) gets neither — only the text nudge (`:750-756`, `:833-836`) and
the sunk bevel (`:950-951`). The XOR covers `t` after the positive-`th`
inset at `:889-890` (a `G_TITLE` has `th = 1`, `gemobjop.c:44-46`, so the
dropped-title block is inset by one pixel, as in `ob_change`'s toggle,
`:1301-1302`), and in the 16-bit VDI it is the RGB complement of the
device pixel (`vdi/vdi_line.c:484-495`), not a pen swap.

**Compendium / Profibuch:** describe SELECTED only as "the object is
displayed inverted"; neither mentions `changecol`, the 3D gate or the
`th` inset. Not a documentation conflict — an EmuTOS-internal rule the spec
had compressed into one wrong word.

**Decision:** Item 8's `SELECTED` sentence rewritten to the exact gate with
a per-class table (flag class × `changecol` → type-body swap / trailing
XOR / bevel), the XOR area and colour semantics, and a note that item (c)'s
"a selected `FL3DBAK` button may still take the `:683-687` swap" cannot
happen (`G_BUTTON` → `xor_is_ok` TRUE). NewTOS's deviations from the rule
are recorded there and in `TASK/A85.md`; the pinning host tests are
`tos-api/tests/aes_object_selected_xor.rs` (three cells pass, eleven fail
against the EmuTOS expectation as of this entry).

---

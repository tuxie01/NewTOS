# `Rwabs` (BIOS 0x04)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1045` (`lrwabs`) and the installed handler
> `bios/blkdev.c:406` (`blkdev_rwabs`).

> **BIOS02 revision (2026-10-28) — approved by the user 2026-10-28.** Every change below is
> tagged **[BIOS02]**. Source: re-read of `references/emutos/bios/blkdev.c`
> at the pin `1a7484ab` (`docs/analysis/ORACLE.md`); cross-check: the disk
> round `tosdump/cases/bios-r2.tsv` on the oracle
> (`tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN`) and the NewTOS run
> `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. Case ids are the
> row labels of `bios-r2.tsv` (R01-R20, F03, W01-W11, X12-X16); the dumper's
> own case number is the table's line number. Corrected: sector-0 write
> (BIOS-STOR-001), count 0 (BIOS-STOR-003), Pi binding (BIOS-STOR-008).
> Decisions BIOS-STOR-002 and -007 (user 2026-10-28): "Decisions" at the end
> of the Pi binding.

## EmuTOS source

**Primary location:** `bios/bios.c:1045` — `lrwabs(r_w, adr, numb, first,
drive, lfirst)`. A one-line forwarder: it calls the vector `hdv_rw` through
the `protect_wlwwwl` thunk, passing the six arguments unchanged.

**Handler behind the vector:** `bios/blkdev.c:406` — `blkdev_rwabs()`,
installed as `hdv_rw` by `blkdev_init()` (`bios/blkdev.c:102`).

**Helpers called:**
- `util/miscasm.S:255` — `protect_wlwwwl(func, …)` — d2/a2-preserving thunk
  marshalling the W+L+W+W+W+L argument list.
- `bios/blkdev.c:783` — `blkdev_mediach()` — media-change check inside the
  logical path.
- `bios/floppy.c` — `floppy_rw()` (units < `NUMFLOPPIES`).
- `bios/disk.c` — `disk_rw()` (hard-disk units), `disk_rescan()`.
- `bios/blkdev.c` — `call_etv_critic()` — critical-error handler chain.

**Headers / type definitions used:**
- `bios/disk.h:53-61` — `RW_READ`/`RW_WRITE`/`RW_RW`/`RW_NOMEDIACH`/
  `RW_NORETRIES`/`RW_NOTRANSLATE`/`RW_NOBYTESWAP`
- `bios/disk.h:69-71` — `MEDIANOCHANGE`/`MEDIAMAYCHANGE`/`MEDIACHANGE`
- `include/gemerror.h` — `E_OK`, `EUNDEV`, `ESECNF`, `E_CHNG`
- `include/tosvars.h` — `hdv_rw` (`extern LONG (*hdv_rw)(...)`)

**System variables / globals read/written:**
- `hdv_rw` (TOS var **`0x476`**) — the read/write vector (`bios/blkdev.c:102`)
- `blkdev[dev].mediachange` / `.forcechange` (read + written by the handler)
- `units[unit].status` / `.last_access`, `hz_200` (`bios/blkdev.c:535`)

**Build configuration affecting this function:**
- `RWABS_RETRIES` — automatic retry count (bypassed by `RW_NORETRIES`).
- `CONF_WITH_1FAT_SUPPORT`, floppy/ACSI/IDE/SCSI driver `CONF_*` flags — which
  units exist and how `disk_rw`/`floppy_rw` behave (behaviour reference).

---

## Source extraction summary

`Rwabs` reads or writes whole sectors. The trap-level `lrwabs` just forwards
to whatever driver is hooked at `hdv_rw`; on EmuTOS that is `blkdev_rwabs`.

`blkdev_rwabs` has two modes selected by the `RW_NOTRANSLATE` (bit 3) flag.
In **logical** mode (default) it validates `drive` against `BLKDEVNUM` and
`DEVICE_VALID`, rejects an invalid BPB (`recsiz == 0`, e.g. FAT32) with
`ESECNF`, converts logical sectors to physical using the BPB's
sectors-per-logical ratio, bounds-checks against the partition size, adds the
partition start, and (unless `RW_NOMEDIACH`) calls `blkdev_mediach`. In
**physical** mode it validates `unit` against `UNITSNUM` and the unit's own
size. It then loops, splitting the transfer into ≤ `0x7FFF`-sector chunks and
calling `floppy_rw`/`disk_rw`, retrying up to `RWABS_RETRIES` times (unless
`RW_NORETRIES`), and routing persistent errors through `call_etv_critic`.

Two behaviours are invisible from the signature but are real contract for
compatible software: (1) if `dev < NUMFLOPPIES` **and `adr == NULL`**, the
call does **not** transfer data — it *sets* `blkdev[dev].mediachange = numb`
and returns 0 (used by FastCopy III); (2) a logical **write that starts at
sector 0** sets `forcechange` (`bios/blkdev.c:452-453`) — **before** the range
check and the media-change check, so the write call **itself** normally
fails: without `RW_NOMEDIACH` it returns `E_CHNG` (-14) and transfers nothing
(`blkdev_mediach` sees `forcechange`, `bios/blkdev.c:474-476,802-803`); with
`RW_NOMEDIACH` it transfers and returns 0. From then on **every** logical
`Rwabs` on that drive returns `E_CHNG` — `RW_NOMEDIACH` does not help, the
test is at `bios/blkdev.c:450-451` ahead of that flag — and `Mediach` returns
2, until `Getbpb` on that drive clears the flag (`bios/blkdev.c:590`).
**[BIOS02]** (previous text said only "the next Mediach/Rwabs reports a media
change"; measured W01-W08, X12-X16, below.)

The `recnr == -1` convention selects the LONG sector number `lfirst`;
otherwise `first` is used as an **unsigned** 16-bit sector (`(UWORD)recnr`,
`bios/blkdev.c:430-431`).

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x04
- **Stack frame on entry (m68k):** C parameter order is
  `r_w, adr, numb, first, drive, lfirst`, pushed right-to-left above the ID:
  ```
  +14: lfirst (LONG)
  +12: drive  (word)
  +10: first  (word)
   +8: numb   (word)
   +4: adr    (LONG)
   +2: r_w    (word)
   0: function_id (word) = 0x0004
  ```
- **Total parameter bytes:** 16 (W + L + W + W + W + L)
- **Native (AArch64) ABI:** ID `0x04` in `x8`; args in `x0..x5` in
  declaration order; error code sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `r_w`    | `WORD` (16) |
  | +4         | `x1`        | `adr`    | `UBYTE *` (ptr) |
  | +8         | `x2`        | `numb`   | `WORD` (16) |
  | +10        | `x3`        | `first`  | `WORD` (16) |
  | +12        | `x4`        | `drive`  | `WORD` (16) |
  | +14        | `x5`        | `lfirst` | `LONG` (32) |

---

## Inputs

| Position | Name     | Type    | Size | Source location       | Notes |
|----------|----------|---------|------|-----------------------|-------|
| arg0     | `r_w`    | `WORD`  | 2    | `bios/blkdev.c:406`   | flag bits, see below |
| arg1     | `adr`    | `UBYTE*`| 4    | `bios/blkdev.c:413`   | buffer; **`NULL` triggers the set-mediachange feature** for floppies |
| arg2     | `numb`   | `WORD`  | 2    | `bios/blkdev.c:410`   | sector count (also the mediachange value when `adr==NULL`) |
| arg3     | `first`  | `WORD`  | 2    | `bios/blkdev.c:430`   | 16-bit sector, treated **unsigned**; `-1` ⇒ use `lfirst` |
| arg4     | `drive`  | `WORD`  | 2    | `bios/blkdev.c:409`   | logical drive (0=A:) or physical unit if `RW_NOTRANSLATE` |
| arg5     | `lfirst` | `LONG`  | 4    | `bios/blkdev.c:431`   | 32-bit sector number (used when `first == -1`) |

### `r_w` flag bits (`bios/disk.h:53-61`) — **corrected from prior spec**

| Bit | Mask | Constant         | Meaning                                    |
|----:|-----:|------------------|--------------------------------------------|
| 0   | 0x01 | `RW_WRITE`/`RW_RW` | 0 ⇒ read, 1 ⇒ write                      |
| 1   | 0x02 | `RW_NOMEDIACH`   | suppress the media-change check            |
| 2   | 0x04 | `RW_NORETRIES`   | do not auto-retry (1 attempt)              |
| 3   | 0x08 | `RW_NOTRANSLATE` | physical mode: `drive` is a physical unit  |
| 7   | 0x80 | `RW_NOBYTESWAP`  | skip byte-swap (physical mode only; cleared in logical mode, `bios/blkdev.c:481`) |

Prior spec named bit 3 "don't retry" and bit 4 "don't translate" — both were
wrong; retry is bit 2 and no-translate is bit 3, and it omitted
`RW_NOBYTESWAP`.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (BIOS/GEMDOS error code, 2's-complement negative on error)
- **Encoding:** `E_OK` (0) on success; a negative code otherwise.
- **Source location:** `bios/blkdev.c:541` (the loop's `retval`).

### Side effects on global state

- `blkdev[dev].mediachange = numb` (the `adr == NULL` feature,
  `bios/blkdev.c:426`).
- `blkdev[dev].forcechange = TRUE` after a logical write to sector 0
  (`bios/blkdev.c:452-453`).
- `units[unit].status &= ~UNIT_CHANGED` when the physical-mode media-change is
  consumed (`bios/blkdev.c:500`); `units[unit].last_access = hz_200` on success
  (`bios/blkdev.c:535`).
- On `E_CHNG` for a non-floppy unit, `disk_rescan(unit)` re-scans partitions
  (`bios/blkdev.c:537-539`).

### Side effects on hardware

- Issues real sector reads/writes to the floppy controller / ACSI / IDE /
  SCSI / SD unit.
- On read, invalidates the instruction cache over the buffer
  (`instruction_cache_kludge`, `bios/blkdev.c:531`) to mimic TOS.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Logical: `drive` invalid / not `DEVICE_VALID` | `bios/blkdev.c:440` | `EUNDEV` (-15) |
| Logical: invalid BPB (`recsiz == 0`, e.g. FAT32/ext2) | `bios/blkdev.c:443` | `ESECNF` (-8) |
| Logical: `forcechange` already armed (**also with `RW_NOMEDIACH`**) | `bios/blkdev.c:450-451` | `E_CHNG` (-14) — W03/W04/X14 |
| Logical: `lrecnr < 0` or `lrecnr+lcount > partition size` (physical sectors, **also for `numb == 0`**; skipped when `blkdev[dev].size == 0`) | `bios/blkdev.c:461-463` | `ESECNF` (-8) — R06-R08, R10 |
| Logical: `blkdev_mediach` != no-change (unless `RW_NOMEDIACH`) — this is how the sector-0 write itself fails | `bios/blkdev.c:473-477` | `E_CHNG` (-14) — W01 |
| Physical: `unit` invalid / not present | `bios/blkdev.c:484` | `EUNDEV` (-15) |
| Physical: sector range out of unit bounds | `bios/blkdev.c:488` | `ESECNF` (-8) |
| Physical: `UNIT_CHANGED` set (unless `RW_NOMEDIACH`) | `bios/blkdev.c:498` | `E_CHNG` (-14) |
| Underlying `floppy_rw`/`disk_rw` failure (retries exhausted) | `bios/blkdev.c:515-523` | driver code, e.g. `EREADF`/`EWRITF`/`EWRPRO`/`ESECNF` |

**Order of the checks in the logical path (load-bearing, `bios/blkdev.c`)
[BIOS02]:**

| Step | Line | Test | Result |
|-----:|------|------|--------|
| 1 | 425 | `dev < NUMFLOPPIES && buf == NULL` | set `blkdev[dev].mediachange = cnt`, return 0 |
| 2 | 430-431 | `recnr != -1` | `lrecnr = (UWORD)recnr` |
| 3 | 440-441 | `dev < 0 \|\| dev >= BLKDEVNUM \|\| !DEVICE_VALID` | `EUNDEV` |
| 4 | 443-444 | `bpb.recsiz == 0` | `ESECNF` |
| 5 | 450-451 | `forcechange` | `E_CHNG` |
| 6 | 452-453 | `(rw & RW_WRITE) && lrecnr == 0` (logical sector, before scaling) | arm `forcechange` — **no return** |
| 7 | 456-458 | scale `lcount`, `lrecnr` by `bpb.recsiz >> psshift` | — |
| 8 | 461-463 | range check (see table) | `ESECNF` |
| 9 | 466 | `lrecnr += blkdev[dev].start` | — |
| 10 | 473-477 | `!(rw & RW_NOMEDIACH)` and `blkdev_mediach(dev) != 0` | `E_CHNG` |
| 11 | 481 | `rw &= ~RW_NOBYTESWAP` | — |
| 12 | 510-528 | transfer loop | driver code |

Consequences (read from the order; W01/W02/X12/X13 measure the first two,
the rest is **inference** from the same order, not measured): the arming at
step 6 happens even when step 8 or 10 then fails the same call; a write that
merely *covers* sector 0 but starts later does not arm; a physical
(`RW_NOTRANSLATE`) write never arms (the arming lives in the logical branch).
`forcechange` is cleared only in `blkdev_getbpb` (`bios/blkdev.c:590`), and
there **before** any validity test of the boot sector — so even a `Getbpb`
that returns NULL clears it.

**Error paths not in Compendium:** the `forcechange`-driven `E_CHNG` for
every logical access after a sector-0 write (and for that write itself unless
`RW_NOMEDIACH`), and the whole `adr == NULL` set-mediachange branch (which
returns `0`, not an error). Prior spec listed `EBADRQ` for `numb == 0`;
**the source has no such check** — and `numb == 0` is **not** exempt from the
checks above (it fails `ESECNF` when `lrecnr` is out of range, R10, and
`E_CHNG` when `forcechange` is armed).

---

## Verification criteria

- **Nominal read:** `Rwabs(RW_READ, buf, 1, 0, 2, 0)` on a valid FAT16 C:
  fills `buf` with logical sector 0 of C: and returns `E_OK` (0).
- **Unknown device:** `Rwabs(RW_READ, buf, 1, 0, 15, 0)` on a system with no
  P: returns `EUNDEV` (-15).
- **FAT32 partition:** logical `Rwabs` on a device whose `bpb.recsiz == 0`
  returns `ESECNF` (-8).
- **Sector-0 write forces media change [BIOS02, corrected]:**
  `Rwabs(RW_WRITE, buf, 1, 0, 2, 0)` itself returns `E_CHNG` (-14) and writes
  nothing (oracle W01 = -14; the image stays identical, `testpart: identical`).
  Then `Mediach(2)` = 2 (W02, again W05), and every logical `Rwabs(…, 2, …)`
  returns `E_CHNG` with the buffer untouched — **including with
  `RW_NOMEDIACH`** (W03, W04) — until `Getbpb(2)` (W06); afterwards `Mediach(2)`
  = 0 (W07) and reads work (W08).
- **Sector-0 write with `RW_NOMEDIACH`:** `Rwabs(RW_WRITE|RW_NOMEDIACH, buf,
  1, 0, 2, 0)` writes and returns 0 (X12) **and** arms the same state:
  `Mediach(2)` = 2 (X13), `Rwabs(RW_NOMEDIACH)` read = `E_CHNG` (X14), cleared
  by `Getbpb(2)` (X15, X16 = 0).
- **Another sector does not arm:** a write of the last sector returns 0 and
  leaves `Mediach` = 0 and reads working (W09-W11).
- **Count 0 is range-checked [BIOS02]:** `Rwabs(0, buf, 0, -1, 2,
  PSIZE+10)` returns `ESECNF` (-8) (R10); `Rwabs(0, buf, 0, 0, 2, 0)` returns 0
  (R09).
- **NULL-buffer feature (floppy):** `Rwabs(0, NULL, 2, 0, 0, 0)` sets
  `blkdev[0].mediachange = 2` and returns `0` (no transfer).

---

## Edge cases discovered in source

1. **`adr == NULL` on a floppy sets mediachange and returns.** Undocumented
   FastCopy-III feature (`bios/blkdev.c:419-428`): the `numb` value becomes the
   drive's media-change status; **no I/O happens**. Only for `dev <
   NUMFLOPPIES`.

2. **Logical write starting at sector 0 arms `forcechange`.**
   `bios/blkdev.c:452-453`. **[BIOS02, corrected]** The arming precedes the
   `E_CHNG` test of `blkdev_mediach` (`:473-477`), so the arming call itself
   fails with `E_CHNG` and does **not** write unless `RW_NOMEDIACH` is set
   (W01 vs X12). Every later logical access returns `E_CHNG` (line 450-451)
   until `Getbpb` resets it — `RW_NOMEDIACH` does not bypass it (W04, X14).
   This makes editing the boot sector force a re-login; a tool that rewrites
   the boot sector must pass `RW_NOMEDIACH` or expect the first call to fail
   and repeat it after `Getbpb`.

3. **`first`/`recnr` is unsigned 16-bit unless -1.** `lrecnr = (UWORD)recnr`
   (`bios/blkdev.c:431`) — sectors 32768..65535 work via `first`; `first == -1`
   switches to the 32-bit `lfirst`.

4. **Transfers are split into `0x7FFF`-sector chunks.** `CNTMAX = 0x7FFF`
   (`bios/blkdev.c:404,511`) because the low-level functions take a signed WORD
   count.

5. **`RW_NOBYTESWAP` is forced off in logical mode.** `rw &= ~RW_NOBYTESWAP`
   (`bios/blkdev.c:481`) — byte-swap control only applies to physical access.

6. **No buffer bounds check.** `adr + numb*secsize` is never validated against
   any process region.

7. **`numb == 0` is not an `EBADRQ` error, but it is not exempt either
   [BIOS02, corrected].** There is no `EBADRQ` path. The drive tests
   (`:440-444`), the `forcechange` test (`:450`), the range check
   (`:461-463`) and the media-change test (`:473-477`) all run with
   `lcount == 0`; R10 (`ESECNF`) measures the range check, R09 (0) the
   in-range case. The transfer `do … while (lcount > 0)` (`:510-528`) then
   runs **once** with `scount == 0`, i.e. the driver is called with a zero
   count (`sd_rw` returns 0 for it, `bios/sd.c:163-164`; the IDE oracle
   returns 0, R09). The former text "returns `E_OK`" held only for an
   in-range sector.

8. **Logical vs. physical range checks use different boundary operators
   (gap filled this pass).** The logical-mode bounds check is
   `(lrecnr + lcount) > blkdev[dev].size` (`bios/blkdev.c:461-462`, strict
   `>`), but the physical-mode check is `(lrecnr + lcount) >=
   units[unit].size` (`bios/blkdev.c:488-489`, `>=`). A transfer that ends
   *exactly* at the device's sector count (`lrecnr+lcount == size`) is
   therefore accepted in logical mode but rejected with `ESECNF` in physical
   (`RW_NOTRANSLATE`) mode on the identical underlying unit — a genuine,
   source-confirmed asymmetry, not a documentation gap. Whether this is an
   intentional off-by-one or an EmuTOS bug is not resolved by the source;
   flagged here as-is per the "trust the code" rule rather than "corrected"
   to a single consistent rule.

9. **The `etv_critic` critical-error path, end to end (gap filled this
   pass — explicitly requested for the P4-012/eralert follow-up).** When
   the inner retry loop exhausts (`retval < 0`) *and* the request is
   **logical** (not `RW_NOTRANSLATE`), `blkdev_rwabs` calls
   `call_etv_critic((WORD)retval, dev)` (`bios/blkdev.c:520-521`) and, if
   the result is the sentinel `CRITIC_RETRY_REQUEST` (`0x00010000L`,
   `bios/blkdev.h:36`), the **entire outer retry loop repeats**
   (`bios/blkdev.c:522`) — a disk error can therefore cause `Rwabs` to
   retry indefinitely if the installed handler keeps asking for a retry.
   Physical (`RW_NOTRANSLATE`) requests **never** call `etv_critic`
   (`bios/blkdev.c:520` guard) — a raw physical `Rwabs` failure is final,
   no matter what handler is installed.

   The call chain, fully traced (BIOS → AES), file:line:
   - `call_etv_critic(error, device)` (`bios/vectors.S:807-818`, asm) packs
     the two `WORD` args as the C function-pointer signature
     `LONG (*etv_critic)(WORD err, WORD dev)` expects, sets `D0 = -1` as
     the *default* return (TOS's own convention — "abort" if the handler
     does nothing), then calls through `_etv_critic.w` — the same
     low-memory vector `Setexc(0x101, …)` reads/writes (`0x404`,
     `05-setexc.md`).
   - **Boot default:** `etv_critic = default_etv_critic` is installed at
     `bios_init()` time (`bios/bios.c:361`, before any program runs — see
     `docs/api/bios/_boot-init.md` §3). `default_etv_critic`
     (`bios/vectors.S:823-826`) simply re-extracts the error word passed
     on the stack and returns it unchanged — i.e. with **no** AES/desktop
     running, a disk error from `Rwabs` propagates straight through as the
     original negative error code; `CRITIC_RETRY_REQUEST` is never
     produced by the default handler, so the outer loop never spins.
   - **AES override (the eralert connection, `aes/gemdosif.S`):** the
     desktop installs its own `criterr_handler` via
     `Setexc(0x0101, addr)` (`gemdosif.S:101-108`, `_takeerr`, called
     during desktop startup) and saves the prior (BIOS default) handler
     in `save_etv_critic` for both chaining and restoration
     (`gemdosif.S:117,130,156-157`). `criterr_handler`
     (`gemdosif.S:153-219`) only engages when `_enable_ceh` is set (i.e.
     the desktop main loop has started, `gemdosif.S:154-157` — before
     that it forwards straight to the saved handler, preserving classic
     TOS boot-time behaviour). When engaged, it: reads `err`(`4(sp)`)/
     `drive`(`6(sp)`) off the stack (the same layout `call_etv_critic`
     built); maps the error to an alert-string index via `err_tbl`
     (`gemdosif.S:179-192,424-431` — `entry[-error-1]`, out-of-range or
     `error==0` defaults to entry 0); calls `_eralert(alert_num, drive)`
     (`aes/gemfmlib.c:416-433`), which shows a `form_alert()` box and
     returns `TRUE` if the user picked "Retry" (button 2/3), `FALSE` for
     "Cancel" (button 1); and finally returns either the **original
     error code** (Cancel) or `CRITIC_RETRY_REQUEST` (`0x00010000`, Retry,
     `gemdosif.S:199-203`) — which is exactly the sentinel
     `blkdev_rwabs`'s outer loop tests for.
   - **Net effect:** with the desktop running, a disk I/O failure from a
     *logical* `Rwabs` call pops a classic Atari "Abort/Retry" alert box;
     choosing Retry re-runs the whole transfer from `blkdev_rwabs`'s outer
     loop (fresh `retry_count` attempts before the next `etv_critic` call),
     choosing Cancel returns the original error code to the GEMDOS/caller
     level. Without a desktop (or before `_enable_ceh`), the error simply
     propagates — no alert, no retry, matching plain BIOS-level behaviour.
   - **AES scope note:** the `criterr_handler`/`eralert`/`err_tbl`
     mechanism lives entirely in `aes/gemdosif.S` and `aes/gemfmlib.c`,
     outside this BIOS-trap-13 spec's scope; documented here only to the
     depth needed to explain `Rwabs`'s `CRITIC_RETRY_REQUEST` retry
     contract. A full AES-side spec entry (if needed for the
     GEMDOS/error-alert follow-up work referenced in the task) is a
     separate, AES-scoped extraction.

10. **Negative or huge `numb` [BIOS02, open].** `cnt` is a signed `WORD`
    widened to `LONG lcount` (`:411`); for `cnt < 0` the loop passes a
    negative `scount` to the driver once (`:512-515`, `lcount > 0` is false
    afterwards). Not measured; NewTOS reads `numb` as unsigned
    (`tos-api/src/bios/mod.rs:148@rwabs`). **Needs deeper analysis /
    a case before anyone relies on either behaviour.**

11. **`dev` negative with `buf == NULL` [BIOS02, source observation].** The
    NULL-buffer test is `dev < NUMFLOPPIES` on a signed `WORD`
    (`:425`), so `dev == -1` with `buf == NULL` writes `blkdev[-1].mediachange`
    — outside the array — and returns 0. Not a contract to copy; unmeasured.

---

## Dependencies

### Requires (must be initialized / functional)

- `hdv_rw` populated (`blkdev_init`, `bios/blkdev.c:102`).
- Valid `blkdev[]`/`units[]` tables; a live floppy/disk driver.
- For logical mode: a valid BPB (`Getbpb` populated `blkdev[dev].bpb`).

### Used by

- GEMDOS FAT layer (`Fread`/`Fwrite`, directory walks); `Getbpb`
  (`blkdev_getbpb` calls `blkdev_rwabs` in physical no-translate mode,
  `bios/blkdev.c:625`); disk boot.

---

## Related data structures

- `docs/api/structures/bpb.md` — per-drive BPB used for logical→physical
- `docs/api/bios/_state.md` — `hdv_rw`/`hdv_bpb`/`hdv_mediach` vectors + addrs

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  the W+L+W+W+W signature and the `lfirst` extension; does **not** document
  the `NULL`-buffer feature or `forcechange`.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BLKDEVNUM = 26` logical drives (A:..Z:, `include/sysconf.h:43`) **[BIOS02, corrected from 16]**; `UNITSNUM = NUMFLOPPIES + 8*(MAX_BUS+1)` physical units (`bios/disk.h:22`; 26 in the Falcon-like build with IDE, 34 where `CONF_WITH_SDMMC` is on — default only for Firebee and M548X, `include/config.h:167,645`).
- `RWABS_RETRIES` sets the auto-retry count — **currently defined as `1`**
  (`bios/blkdev.h:34`, `#define RWABS_RETRIES 1 /* on real machine might
  want to increase this */`). Gap-filled finding this pass: since
  `retry_count = (rw & RW_NORETRIES) ? 1 : RWABS_RETRIES` evaluates to `1`
  on **both** branches when `RWABS_RETRIES == 1`, the `RW_NORETRIES` flag
  currently has **no observable effect** in EmuTOS as shipped — every
  logical/physical transfer already gets exactly one attempt (no retry)
  regardless of whether the caller sets bit 2. This is a real, source-
  confirmed fact (not a spec error to "fix"; `RW_NORETRIES` remains part of
  the ABI contract and must still be *accepted*, it simply changes nothing
  observable under the current constant).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-04: `hdv_rw` becomes a Rust trait dispatch to per-unit `BlockDevice`
impls. Sector size is 512 throughout. The `r_w` flag bits map onto a
`BlockOp`/flags enum; the `adr == NULL` and sector-0-`forcechange` behaviours
must be reproduced for m68k-binary compatibility (Phase 10) but may be
NewTOS-native for source ports.

FS09 (2026-10-07, FreeMiNT extension): every request reaches the card through
the block read cache `tos-api/src/bios/bcache.rs`
(`docs/analysis/fs09-block-cache-2026-10-07.md`). Keyed by the absolute LBA,
so logical and physical (`RW_NOTRANSLATE`) requests share one copy; reads may
be answered from RAM, writes go to the card first (write-through) and only a
confirmed write updates the cache (≤ 32 KiB) or drops the range (larger, or
failed). Return values and error codes are unchanged; `RW_NORETRIES` /
`RW_NOMEDIACH` do not affect the cache (§2.8 there).

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the Atari WD1772 FDC / ACSI hard disk
> to the **BCM2711 EMMC2 SD-card controller**. The contract above (ABI, flag
> bits, error codes, the `adr==NULL` / sector-0 behaviours) is unchanged; only
> the backing hardware and the concrete NewTOS values are pinned here. See
> `docs/HARDWARE-PROFILE.md` §3.4.

### Backing hardware + driver

| Atari reference | Pi-4 (contract) | Driver |
|-----------------|-----------------|--------|
| WD1772 FDC / ACSI-SCSI hard disk | **EMMC2 SD-card controller** (Arasan/iproc SDHCI, phys `0xFE34_0000`) | `kernel/src/drivers/bcm2711/emmc.rs` behind `kernel/src/hal/block.rs::BlockDevice`, exposed to `tos-api` as `bios::block::BlockDev` via `kernel/src/host_env.rs:246-267` |

- **Physical base / controller selection:** EMMC2 `0xFE34_0000` (real Pi-4 SD
  slot); QEMU `raspi4b` wires its emulated card to the legacy EMMC
  `0xFE30_0000`. The driver probes EMMC2 first, then EMMC, and binds to
  whichever has a responding card (`emmc.rs:16-18,375-386`). One binary serves
  both. Address map: BCM2711 datasheet §1.2 (p.4-6); EMMC register/bit detail
  per BCM2835 datasheet §5 + SD Host Controller Simplified Spec, as documented
  in `docs/hardware/emmc-sd.md`.
- **Sector size = 512 bytes, fixed** (`hal/block.rs:9` `SECTOR_SIZE=512`;
  `bios/mod.rs:26` `SECTOR=512`). The SD `BLKSIZECNT` block-size field is set
  to 512 on every transfer (`emmc.rs:298,323`). SDSC cards are locked to a
  512-byte block length via `SET_BLOCKLEN` (`emmc.rs:274-276`); SDHC/SDXC are
  fixed at 512 in hardware.

### The read/write path (`emmc.rs`)

- `Rwabs` (`tos-api/src/bios/mod.rs:66-116`) resolves `drive`→partition,
  range-checks, computes the absolute LBA, then calls
  `BlockDev::read_blocks`/`write_blocks`, which forward to the EMMC driver
  (`host_env.rs:256-263`).
- **Read:** PIO single-block `CMD17 READ_SINGLE` per sector — `BLKSIZECNT`,
  `ARG1=addr`, `CMDTM`, wait `CMD_DONE`, wait `READ_RDY`, drain 128×32-bit words
  from `DATA`, wait `DATA_DONE` (`emmc.rs:284-308`, `read_blocks` loops one
  block at a time `emmc.rs:339-347`).
- **Write:** PIO single-block `CMD24 WRITE_SINGLE` per sector, symmetric
  (`emmc.rs:310-331`, loop `emmc.rs:349-357`). **The write path is implemented**
  (unlike the read-only default in the HAL trait, `hal/block.rs:36-38`, which
  the EMMC impl overrides).
- **Addressing:** SDHC/SDXC address by block (`arg=lba`); SDSC by byte
  (`arg=lba*512`) — chosen per card at init (`emmc.rs:293-297,318-322`).
- **No IRQ, no DMA:** the bring-up driver polls `INTERRUPT` (`IRPT_EN=0`,
  `emmc.rs:210`); no DMA-coherency concern.

### Partition-relative → absolute LBA

`Rwabs` in **logical** mode maps `(drive, sector)` to
`part.start_lba + sector` and range-checks `sector + numb <= part.count`
(`tos-api/src/bios/mod.rs:178@RW_NOTRANSLATE`; the check sits after the
`numb == 0` return at `tos-api/src/bios/mod.rs:170@numb` — BIOS-STOR-003).
`part.start_lba`/`part.count` come from the MBR parse in
`bios/drive.rs` (see `0a-drvmap.md`). Drives are numbered A:=0, B:=1, C:=2, …;
the SD partitions are assigned from **C:** upward (`tos-api/src/bios/drive.rs:12-13,69`).

### Flag-bit effects on the Pi

| Bit | Constant | On the EMMC SD today |
|----:|----------|----------------------|
| 0 | `RW_WRITE` | selects `write_blocks` (CMD24) vs `read_blocks` (CMD17) — `bios/mod.rs:97,102-108` |
| 1 | `RW_NOMEDIACH` | **no effect** — `Rwabs` never invokes the media-change check (the SD is non-removable, `Mediach`≡0; see `09-mediach.md`). GAP vs the contract's step 10 (`bios/blkdev.c:473-477`). Observable only together with `forcechange` (BIOS-STOR-001): without that state there is nothing for the flag to bypass. |
| 2 | `RW_NORETRIES` | **no effect** — the EMMC driver does a single PIO attempt; there is no `RWABS_RETRIES` retry loop and no `etv_critic` chain. Unobservable at `RWABS_RETRIES = 1` (audit §6). |
| 3 | `RW_NOTRANSLATE` | **[BIOS02, corrected]** the bit **is** honoured: `r_w & RW_NOTRANSLATE` selects an absolute device sector, range-checked against the whole device with `>=` like EmuTOS (`tos-api/src/bios/mod.rs:178@RW_NOTRANSLATE`). What is **wrong** is the `drive` argument: it is still resolved as a *logical drive letter* (`tos-api/src/bios/mod.rs:160@resolve`) — EmuTOS reads it as a *physical unit number* (`bios/blkdev.c:484-485`). BIOS-STOR-002, decided below (unit 26). |
| 7 | `RW_NOBYTESWAP` | **irrelevant on ARM** — see below. |

- **No byte-swap on ARM.** The Atari byte-swap logic (`RW_NOBYTESWAP`, forced
  off in EmuTOS logical mode) has **no analogue**: FAT/DOS structures are
  little-endian and AArch64 is little-endian, so sectors are transferred
  byte-for-byte (`DATA` port read/written via `to_le_bytes`/`from_le_bytes`,
  `emmc.rs:303-304,327`). The bit is simply ignored.

### Contract behaviours vs. the current code (Step-2 candidates)

- **`adr == NULL` set-mediachange feature:** not implemented — a NULL buffer
  on A:/B: ends in `EUNDEV` (`tos-api/src/bios/mod.rs:160@resolve`), EmuTOS
  returns 0 *before* any drive test (`bios/blkdev.c:425-428`). **Measured
  DIFF, F03/F03b** (EmuTOS 0, NewTOS -15), also on an EmuTOS without floppy
  drives (audit §1, `EMUTOS-falcon-nofloppy`) — so not a device parameter.
  **[BIOS02]** Decided below: match EmuTOS (BIOS-STOR-007).
- **Sector-0 write → `forcechange`:** **[BIOS02] not implemented, measured
  DIFF** (W01-W05, X13, X14: EmuTOS -14/2/-14/-14/2/2/-14, NewTOS
  0/0/0/0/0/0/0). Required state per drive: a `forcechange` flag armed by a
  logical write starting at sector 0 (step 6), tested before everything else
  once the drive and BPB checks have passed (step 5), reported by `Mediach`
  as 2 (`09-mediach.md`), cleared only by `Getbpb` on that drive. A
  consequence if left out: a boot-sector rewrite is not seen as a media
  change, so GEMDOS does not re-log the drive (EmuTOS `ckdrv`/`getbcb` ask
  `Mediach`, audit BIOS-STOR-001 — *closed reasoning, not measured*).
- **`first` unsigned-16 / `first==-1`⇒`lfirst`:** implemented
  (`tos-api/src/bios/mod.rs:170@numb`, the `sector` computation below it);
  R03/R04/R18-R20 equal.
- **`numb == 0` ⇒ `E_OK`, no transfer:** **[BIOS02, wrong]** implemented as an
  early return *before* the range check (`tos-api/src/bios/mod.rs:170@numb`).
  EmuTOS checks the range first: R10 (`Rwabs(0,buf,0,-1,2,PSIZE+10)`) is
  `ESECNF` on EmuTOS, 0 on NewTOS — **measured DIFF**; R09 (in range) equal.
- **`0x7FFF`-sector chunking:** not needed — transfers loop one 512-byte block
  at a time (`emmc.rs:343-346,353-356`); no signed-WORD count limit.
- **Error codes:** invalid/unmounted drive → `EUNDEV`; out-of-range sector →
  `ESECNF`; driver failure → **the driver's own code, passed through**, as in
  EmuTOS `blkdev_rwabs` (`bios/blkdev.c:512-523`). Since HW03 the block
  interface carries a TOS code instead of a `bool`, so the card's refusal
  (R1 error bit) and a broken transfer arrive as `EWRITF`/`EREADF`, a wait
  that ran out as `EDRVNR`, and a block outside the card's address space as
  `ESECNF` (`docs/hardware/emmc-sd.md` §5).
  **[BIOS02]** measured equal for R06-R08, R11-R14 (`ESECNF`, `EUNDEV`).
  The invalid-BPB (`recsiz==0`) `ESECNF` guard is **not** in `Rwabs` here —
  FAT32/exFAT rejection happens in `Getbpb` (see `07-getbpb.md`), and `Rwabs`
  does no BPB lookup, so a logical `Rwabs` on a FAT32 partition transfers raw
  sectors rather than returning `ESECNF`. GAP.
- **Buffer pointer:** `adr` is used raw (`bios/mod.rs:99-108`); EL0 user-pointer
  validation is deferred to the process-aware BIOS path (matches EmuTOS's
  no-bounds-check note).

### Measured cross-check — `bios-r2.tsv` cases per statement [BIOS02]

Oracle = `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN`; NewTOS =
`measurements/measurement-2026-10-28-tool07/RUNINFO.TXT` (tosdiff: 10 of 51
cases differ). Dumper case number = line of the row in `bios-r2.tsv`.

| Statement | Cases (label : dumper no.) | EmuTOS | NewTOS | Verdict |
|-----------|----------------------------|--------|--------|---------|
| logical read, data, `first`/`lfirst`, 16-bit unsigned recno | R01 58-59, R02 60, R03 61, R04 62, R18-R20 76-78 | 0 + data | same | EQUAL |
| last sector reachable; one past / straddle / negative → `ESECNF` | R05 63; R06 64, R07 65, R08 66 | 0; -8 | same | EQUAL |
| count 0 in range → 0 | R09 67 | 0 | 0 | EQUAL |
| **count 0 out of range → `ESECNF`** | R10 68 | -8 | 0 | **DIFF** (BIOS-STOR-003) |
| absent / out-of-range drive → `EUNDEV` | R11-R14 69-72 | -15 | -15 | EQUAL |
| flags `RW_NOMEDIACH`, `RW_NORETRIES`, `RW_NOBYTESWAP` accepted | R15-R17 73-75 | 0 | 0 | EQUAL |
| **NULL buffer on A: → 0** | F03 83, F03b 84 | 0 | -15 | **DIFF** (BIOS-STOR-007) |
| **sector-0 write returns `E_CHNG`, no write** | W01 92 | -14 | 0 | **DIFF** (BIOS-STOR-001) |
| **`Mediach` = 2 after it, latched** | W02 93, W05 96 | 2 | 0 | **DIFF** |
| **logical read `E_CHNG`, also with `RW_NOMEDIACH`, buffer untouched** | W03 94, W04 95 | -14 | 0 + data | **DIFF** |
| `Getbpb` clears the state; `Mediach` 0, reads work again | W06 97, W07 98, W08 99 | BPB, 0, 0 | same | EQUAL (nothing to clear on NewTOS) |
| non-zero sector does not arm | W09 101, W10 102, W11 103 | 0, 0, 0 | same | EQUAL |
| **sector-0 write with `RW_NOMEDIACH`: writes (0) and arms** | X12 105 (0), X13 106, X14 107 | 0, 2, -14 | 0, 0, 0 | X12 EQUAL; **X13, X14 DIFF** |
| `Getbpb` / `Mediach` after it | X15 108, X16 109 | BPB, 0 | same | EQUAL |

**Measured since TOOL07 (rest) by the closing tables** (`tosdump/cases/bios-r2-phys.tsv`,
`bios-r2-x004.tsv`, `bios-r2-x005.tsv`, `bios-r2-x006.tsv`;
`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`): `RW_NOTRANSLATE`
through the unit token `u:0` (a literal unit stays refused by the dumper
guard, `tosdump/core/cases.c:3225@rwabs_value_refusal`), and `recsiz == 0` →
`ESECNF` after a failed `Getbpb` (BIOS-STOR-006, `x006`).
**Not measured (stay [source]-only):** arming before a failing range/mediach check,
negative `numb`, negative `dev` with `buf == NULL`, the physical-mode `>=`
asymmetry (edge case 8: audit probe P03 only), `etv_critic`, the 0x7FFF
chunking.

### Decisions (user 2026-10-28) [BIOS02]

- **BIOS-STOR-002 — physical unit numbering for `RW_NOTRANSLATE`.** EmuTOS:
  `unit = NUMFLOPPIES (2) + 8*bus + device`
  (`bios/disk.h:18,20,30`), buses ACSI 0, SCSI 1, IDE 2, SD/MMC 3
  (`bios/machine.h:39-42`) → ACSI 0 = 2, SCSI 0 = 10, IDE master = 18,
  SD/MMC = 26; `sd_rw` accepts device 0 only (`bios/sd.c:160-161`), so the
  SD/MMC bus offers exactly unit 26. **Rule:** on NewTOS the Pi's SD card is
  physical unit **26**; in physical mode every other unit returns `EUNDEV`.
  Case tables name the unit by a token resolved from the profile definition
  (oracle `disk` profile: 18, NewTOS: 26), never by a search at run time.
- **BIOS-STOR-007 — `Rwabs(…, NULL, n, …, A:/B:)`.** **Rule:** match EmuTOS —
  `dev` 0 or 1 with `adr == NULL` returns 0 before any drive test
  (`bios/blkdev.c:425-428`), although A:/B: are no floppy drives on the Pi.
- **BIOS-STOR-005** (`Getbpb`, `tot16 == tot32 == 0`): follow EmuTOS, see
  `07-getbpb.md`.

# `Rwabs` (BIOS 0x04)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1045` (`lrwabs`) and the installed handler
> `bios/blkdev.c:406` (`blkdev_rwabs`).

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
and returns 0 (used by FastCopy III); (2) a logical **write to sector 0**
sets `forcechange`, so the next `Mediach`/`Rwabs` reports a media change until
`Getbpb` clears it (this is how altering the boot sector forces a re-login).

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
| Logical: `forcechange` still pending | `bios/blkdev.c:450` | `E_CHNG` (-14) |
| Logical: `lrecnr < 0` or `lrecnr+lcount > partition size` | `bios/blkdev.c:461` | `ESECNF` (-8) |
| Logical: `blkdev_mediach` != no-change (unless `RW_NOMEDIACH`) | `bios/blkdev.c:474` | `E_CHNG` (-14) |
| Physical: `unit` invalid / not present | `bios/blkdev.c:484` | `EUNDEV` (-15) |
| Physical: sector range out of unit bounds | `bios/blkdev.c:488` | `ESECNF` (-8) |
| Physical: `UNIT_CHANGED` set (unless `RW_NOMEDIACH`) | `bios/blkdev.c:498` | `E_CHNG` (-14) |
| Underlying `floppy_rw`/`disk_rw` failure (retries exhausted) | `bios/blkdev.c:515-523` | driver code, e.g. `EREADF`/`EWRITF`/`EWRPRO`/`ESECNF` |

**Error paths not in Compendium:** the `forcechange`-driven `E_CHNG` on the
call *following* a sector-0 write, and the whole `adr == NULL` set-mediachange
branch (which returns `0`, not an error). Prior spec listed `EBADRQ` for
`numb == 0`; **the source has no such check** — a zero count simply performs
zero transfers and returns `E_OK`.

---

## Verification criteria

- **Nominal read:** `Rwabs(RW_READ, buf, 1, 0, 2, 0)` on a valid FAT16 C:
  fills `buf` with logical sector 0 of C: and returns `E_OK` (0).
- **Unknown device:** `Rwabs(RW_READ, buf, 1, 0, 15, 0)` on a system with no
  P: returns `EUNDEV` (-15).
- **FAT32 partition:** logical `Rwabs` on a device whose `bpb.recsiz == 0`
  returns `ESECNF` (-8).
- **Sector-0 write forces media change:** after
  `Rwabs(RW_WRITE, buf, 1, 0, 2, 0)`, the next `Mediach(2)` returns
  `MEDIACHANGE` (2) until `Getbpb(2)` is called.
- **NULL-buffer feature (floppy):** `Rwabs(0, NULL, 2, 0, 0, 0)` sets
  `blkdev[0].mediachange = 2` and returns `0` (no transfer).

---

## Edge cases discovered in source

1. **`adr == NULL` on a floppy sets mediachange and returns.** Undocumented
   FastCopy-III feature (`bios/blkdev.c:419-428`): the `numb` value becomes the
   drive's media-change status; **no I/O happens**. Only for `dev <
   NUMFLOPPIES`.

2. **Logical write to sector 0 arms `forcechange`.** `bios/blkdev.c:452-453`.
   The *next* logical access then returns `E_CHNG` (line 450) until `Getbpb`
   resets it. This makes editing the boot sector force a re-login.

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

7. **`numb == 0` is not an error.** Contrary to the prior spec, there is no
   `EBADRQ` path; the loop just does nothing and returns `E_OK`.

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

- `BLKDEVNUM = 16` logical drives (A:..P:); `UNITSNUM` physical units.
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
(`bios/mod.rs:75-95`). `part.start_lba`/`part.count` come from the MBR parse in
`bios/drive.rs` (see `0a-drvmap.md`). Drives are numbered A:=0, B:=1, C:=2, …;
the SD partitions are assigned from **C:** upward (`tos-api/src/bios/drive.rs:12-13,69`).

### Flag-bit effects on the Pi

| Bit | Constant | On the EMMC SD today |
|----:|----------|----------------------|
| 0 | `RW_WRITE` | selects `write_blocks` (CMD24) vs `read_blocks` (CMD17) — `bios/mod.rs:97,102-108` |
| 1 | `RW_NOMEDIACH` | **no effect** — `Rwabs` never invokes the media-change check (the SD is non-removable, `Mediach`≡0; see `09-mediach.md`). GAP vs the contract's "unless `RW_NOMEDIACH`, call `Mediach`". |
| 2 | `RW_NORETRIES` | **no effect** — the EMMC driver does a single PIO attempt; there is no `RWABS_RETRIES` retry loop and no `etv_critic` chain. |
| 3 | `RW_NOTRANSLATE` | **not honoured** — `drive` is always treated as a **logical** drive resolved through the partition table; physical/unit mode is not implemented. A caller setting bit 3 still gets partition-relative addressing (or `EUNDEV` if the letter is unmounted). GAP. |
| 7 | `RW_NOBYTESWAP` | **irrelevant on ARM** — see below. |

- **No byte-swap on ARM.** The Atari byte-swap logic (`RW_NOBYTESWAP`, forced
  off in EmuTOS logical mode) has **no analogue**: FAT/DOS structures are
  little-endian and AArch64 is little-endian, so sectors are transferred
  byte-for-byte (`DATA` port read/written via `to_le_bytes`/`from_le_bytes`,
  `emmc.rs:303-304,327`). The bit is simply ignored.

### Contract behaviours vs. the current code (Step-2 candidates)

- **`adr == NULL` set-mediachange feature:** not implemented — the branch is
  floppy-only in EmuTOS and there is no floppy; a NULL buffer would be
  dereferenced. Harmless on the Pi (no floppy caller) but the branch is absent.
- **Sector-0 write → `forcechange`:** **not implemented.** A logical write to
  sector 0 does not arm any change flag; the next `Mediach` still returns 0.
  With a non-removable card this is cosmetically fine, but the boot-sector
  re-login contract step is missing (see `09-mediach.md`, B-series).
- **`first` unsigned-16 / `first==-1`⇒`lfirst`:** implemented
  (`bios/mod.rs:86-90`).
- **`numb == 0` ⇒ `E_OK`, no transfer:** implemented (`bios/mod.rs:83-84`).
- **`0x7FFF`-sector chunking:** not needed — transfers loop one 512-byte block
  at a time (`emmc.rs:343-346,353-356`); no signed-WORD count limit.
- **Error codes:** invalid/unmounted drive → `EUNDEV`; out-of-range sector →
  `ESECNF`; driver failure → **the driver's own code, passed through**, as in
  EmuTOS `blkdev_rwabs` (`bios/blkdev.c:512-523`). Since HW03 the block
  interface carries a TOS code instead of a `bool`, so the card's refusal
  (R1 error bit) and a broken transfer arrive as `EWRITF`/`EREADF`, a wait
  that ran out as `EDRVNR`, and a block outside the card's address space as
  `ESECNF` (`docs/hardware/emmc-sd.md` §5).
  The invalid-BPB (`recsiz==0`) `ESECNF` guard is **not** in `Rwabs` here —
  FAT32/exFAT rejection happens in `Getbpb` (see `07-getbpb.md`), and `Rwabs`
  does no BPB lookup, so a logical `Rwabs` on a FAT32 partition transfers raw
  sectors rather than returning `ESECNF`. GAP.
- **Buffer pointer:** `adr` is used raw (`bios/mod.rs:99-108`); EL0 user-pointer
  validation is deferred to the process-aware BIOS path (matches EmuTOS's
  no-bounds-check note).

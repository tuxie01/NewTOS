# `Fwrite` (GEMDOS 0x40)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsio.c:528` (`xwrite`) and the helper
> `bdos/fsio.c:554` (`ixwrite`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, "actual bytes written" return, short-write = disk
> full, write-to-read-only allowed, error codes. Reference: `xrw`/`nextcl`
> cluster allocation.

## EmuTOS source

**Primary location:** `bdos/fsio.c:528` — `long xwrite(int h, long len, void *ubufr)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x40` =
`xwrite`, `stdio_typ = 0x82`, `wparms=5` (`bdos/bdosmain.c:260`). As with
`Fread`, the `0x82` flag is **inert**: `0x40` is outside the redirection ranges
(`fn < 12` / `16..19`, `bdos/bdosmain.c:498`), so the **stdio redirection block** never intercepts it. (Correction 2026-10-13:
the `0x82` flag is *not* inert overall — bit 7 sends `Fwrite` into the
handle-resolution block `bdos/bdosmain.c:571-643`, which runs **before**
`xwrite` and answers character-device handles itself. See "Character-device
path" below.)

**Helpers called (file path):**
- `bdos/fsmain.c:474` — `getofd()`.
- `bdos/fsio.c:554` — `ixwrite()` → `bdos/fsio.c:208` — `xrw(1, …)` — the
  unified transfer, allocating clusters with `nextcl` when writing past EOF.
- `bdos/fsfat.c` — `nextcl()` — extend the FAT chain.

**Headers / type definitions used:** `bdos/fs.h` — `OFD`, `DFD`, `O_DIRTY`.

**System variables / globals read:** `run`; OFD position; `dfd->o_fileln`.
**System variables / globals written:** OFD position fields; `dfd->o_fileln`
grows and `dfd->o_flag |= O_DIRTY` when the file extends (`bdos/fsio.c:42-46`).

**Build configuration affecting this function:** none.

---

## Character-device path (ADDED 2026-10-13, FS11; EmuTOS @ `1a7484ab`)

Everything here happens in `osif()` **before** `xwrite`; `xwrite` never sees a
handle that ends on a BIOS character device. `bdos/bdosmain.c` unless noted.

### Dispatch and argument layout

`pw[0]`=fn 0x40, `pw[1]`=handle (`:571-576`), `pw[2..3]`=`len` as one LONG
(`count = *(long *)&pw[2]`, `:624`), `pw[4..5]`=buffer pointer (`:604`, `:626`).
Handle resolution (`:578-593`, `:600-602`) is identical to `Fread`/`Fseek`, see
[`42-fseek.md`](42-fseek.md) and [`3f-fread.md`](3f-fread.md): `numl == 0` →
`EIHNDL` (`:592-593`); `numl < -3` → `EIHNDL` (`:601-602`); `numl > 0` → `xwrite`
(file path). `numl` in `-1..-3` → device path. `HXFORM(h) = 3+h`
(`bdos/console.h:36`): CON (-1) → BIOS 2, AUX (-2) → BIOS 1, PRN (-3) → BIOS 0.

Default std handles (`bdos/console.c:60-67`): 0,1,4,5 → CON; 2 → AUX; 3 → PRN.
`Fwrite` does not check direction: `Fwrite(0,…)` (stdin) writes to the console.
`Fdup` of a device-mapped std handle (`bdos/fshand.c:115-118`) and
`Fforce(std, -1..-3)` (`bdos/fshand.c:53-54`) give handles that take the same
device path; `Fforce(std, file)` takes the file path; `Fclose(std)` reverts to
the default (`bdos/fsopnclo.c:450-455`); `Fclose` of the closed dup'ed slot →
later `Fwrite` → `EIHNDL` (slot `f_ofd = 0`, `bdos/fsopnclo.c:462-470`).

### Device-write algorithm (`:622-640`)

```
count = LONG(pw[2..3]); p = buffer
for (n = 0; n < count; n++)               // count <= 0: loop body never runs
    if (numl == H_Console)  tabout(2, (unsigned char)*p++)      // :630-631
    else if (Bconout(HXFORM(numl), (unsigned char)*p++) == 0)   // :634
             return n;                                          // :635
return count;                                                   // :639
```

- **No HUGE guard** (unlike `Fread`): the full 32-bit `count` is used.
- **`count <= 0`**: nothing written; returns **`count` itself** (so
  `Fwrite(-1, -5, b)` returns **-5**, a negative value that is *not* an error
  code; `Fwrite(-1, 0, b)` returns 0). Read from `:628,:639`.
- **Bytes are written one at a time**, each as `(unsigned char)`; the buffer is
  not NUL-terminated-scanned; embedded NULs are written.

### CON (-1 → BIOS 2): `tabout` (`bdos/console.c:292-302`)

`tabout(2, ch)` returns `void`; `Fwrite` therefore **always returns `count`**
for CON (barring Ctrl-C termination). Per byte:

1. TAB (0x09): `do conout(2,' ') while (glbcolumn[2] & 7)` — at least one
   space, then spaces up to the next multiple of 8 columns (`bdos/console.c:294-299`).
2. any other byte: `conout(2, ch)`.

`conout` (`bdos/console.c:263-274`): first `conbrk(2)` (`bdos/console.c:206-243`), then
`Bconout(2, ch)` (return value ignored), then column update: `ch >= 0x20`
(including 0x7F and 0x80-0xFF) → `glbcolumn[2]++`; CR → 0; BS → `--` (not
clamped; may go negative); everything else (LF, BEL, ESC, NUL, …) unchanged.
No other translation: LF is **not** expanded to CR LF; control characters are
sent raw (the `^X` expansion is `cookdout`, used only by `cgets`).

`conbrk` runs **before every byte**: if `Bconstat(2)` is non-zero it drains
`Bconin(2)`:
- Ctrl-C (0x03) → flush the BDOS typeahead buffer and `terminate()` =
  `xterm(-32)` (`bdos/console.c:101,220-225`): **the process is terminated, the
  `Fwrite` does not return** (inference: `xterm` does not return).
- Ctrl-S (0x13) → output pauses; `Bconin` loop continues until Ctrl-Q (0x11)
  (`bdos/console.c:226-231,242`).
- Ctrl-X (0x18) → flush typeahead; the key itself is then queued.
- any other key → appended to the 64-entry BDOS typeahead buffer
  (`bdos/fs.h:46`); if full, BEL `Bconout(2,7)` and the key is dropped
  (`bdos/console.c:235-240`). Type-ahead keys swallowed here are later delivered by
  `getch` (typeahead first) to `Fread`/`Cconin`.
Column state `glbcolumn[2]` is shared with `Cconout`/`Cconws`/`Cconrs`.

### AUX (-2 → BIOS 1) and PRN (-3 → BIOS 0): raw `Bconout`

Per byte `Bconout(dev, (unsigned char)b)` (BIOS trap 13 fn 3,
`include/biosbind.h:22`) — **no `conbrk` (no Ctrl-C/Ctrl-S), no tab expansion,
no column tracking, no CR/LF translation.** The BIOS return value is tested for
non-zero only:

- return **0** → `Fwrite` returns `n` = number of bytes *before* the failing
  one (that byte is not counted, the remaining bytes are not sent, no retry),
  `:634-635`. The value returned is a short count, never an error code.
- non-zero → continue; after the last byte `Fwrite` returns `count`.

When `Bconout` returns 0 (BIOS side):
- PRN (`bconout0`, `bios/parport.c:144-169`): with `CONF_WITH_PRINTER_PORT`: if
  the previous call did not time out within the last `SHORT_TIMEOUT` = 5 s
  (`:40`), it waits for the port up to `LONG_TIMEOUT` = 30 s (`:41`, `:157-166`;
  a Ctrl-C on the console aborts the wait, `:163-165`) and returns `prnout(c)`
  (1) when ready; on timeout it records `last_timeout` and returns **0**
  (`:168-169`); within 5 s of a timeout it returns 0 immediately. **Without**
  `CONF_WITH_PRINTER_PORT` (default 0, `include/config.h:707-708`; machine
  configs may enable it) `bconout0` always returns 0 → the first byte fails →
  `Fwrite(-3 or 3, n, b)` returns **0**.
- AUX (`bconout1`, `bios/serport.c:289-330`): returns 1 after queuing/sending
  on MFP or ColdFire RS232; blocks while the transmit buffer is full; returns 0
  only in a build with neither (`:325-327`).
- Any device: `bconout` returns 0 while `CHARDEV_AVAILABLE` is unset
  (`bios/bios.c:997-998`, early boot), and for BIOS handle `> 7`
  (`:1008-1010`; not reachable via BDOS handles -1..-3). `Bconmap` remaps only
  BIOS handles `>= 6` (`bios/bios.c:1002-1004`).

### Error table (device path)

| Condition | Cite | Returned |
|---|---|---|
| handle resolves to `0` (free sft slot / `p_uft[h] == 0`) | `:592-593` | `EIHNDL` (-37) |
| handle `<= -4` | `:600-602` | `EIHNDL` (-37) |
| `h >= NUMHANDLES` | EmuTOS reads `sft[]` out of bounds (`:578-579`); contract per `42-fseek.md` | `EIHNDL` |
| CON, any `count > 0` | `:630-631,639` | `count` |
| AUX/PRN, all bytes accepted | `:639` | `count` |
| AUX/PRN, `Bconout` returns 0 at byte index `n` | `:634-635` | `n` (short count) |
| any device, `count <= 0` | `:628,639` | `count` (0, or the negative `count` itself) |

### Cross-checks (device path)

- **FreeMiNT** (not followed): `sys_f_write` hands any terminal to `tty_write`
  (`references/freemint/sys/dosfile.c:300-301`), termios processing, no `tabout`.
- **Atari Compendium / Profibuch:** not in the repository (searched); not
  cross-checked. Equality with ROM TOS (esp. the short-count-on-`Bconout`==0 and
  the negative-`count` echo) is **unverified**.

### Test cases (device path; **EmuTOS reference expectations**)

> These are the EmuTOS results. The NewTOS expectations (decisions of
> 2026-10-13: AUX = port present/no cable (option A), PRN absent; the CON
> rule follows EmuTOS again since TASK/CON01, 2026-10-18) are
> the table "NewTOS host tests" below; where they differ, that table is the one
> the Developer implements.

Setup: default process; keyboard/AUX queues injectable; per-device output
captured; printer-port accept/reject switchable.

| # | Setup | Call | Expected |
|---|---|---|---|
| W1 | default | `Fwrite(-1, 5, "hello")` | 5; CON output `"hello"` |
| W2 | default | `Fwrite(1, 3, "abc")`, `Fwrite(0,…)`, `Fwrite(4,…)`, `Fwrite(5,…)` | 3 each; all to CON |
| W3 | CON at column 0 | `Fwrite(-1, 2, "\tX")` | 2; output = 8 spaces + `X` |
| W4 | CON at column 0 | `Fwrite(-1, 4, "ab\tX")` | 4; output `"ab"` + 6 spaces + `"X"` |
| W5 | CON at column 5 (after `Fwrite("12345")`) | `Fwrite(-1,2,"\tX")` | 2; 3 spaces then `X` |
| W6 | CON at column 8 | `Fwrite(-1,1,"\t")` | 1; 8 spaces (always at least one) |
| W7 | CON | `Fwrite(-1, 3, "a\nb")` | 3; output bytes `61 0A 62` (no CR inserted) |
| W8 | CON | `Fwrite(-1, 3, "a\rb")` then `Fwrite(-1,1,"\t")` | 3, then 1; column reset by CR (tab from col 1 → 7 spaces) |
| W9 | CON | `Fwrite(-1, 0, b)` | 0, no output |
| W10 | CON | `Fwrite(-1, -5, b)` | **-5**, no output |
| W11 | CON | `Fwrite(-1, 3, "a\0b")` | 3; bytes `61 00 62` |
| W12 | keyboard queue `('k')` | `Fwrite(-1, 1, "x")` | 1; `'k'` consumed from the hardware queue into typeahead, delivered by next `Fread(-1,1,…)` |
| W13 | keyboard queue `(^C)` | `Fwrite(-1, 3, "abc")` | process terminated (exit -32) before/at the first byte; no return (observe terminate hook) |
| W14 | keyboard queue `(^S)` then later `^Q` | `Fwrite(-1, 3, "abc")` | output of the byte is held until `^Q`; then returns 3 |
| W15 | AUX ready | `Fwrite(-2, 4, "ATZ\r")` and `Fwrite(2, 4, …)` (separate runs) | 4; AUX output `41 54 5A 0D`; nothing on CON |
| W16 | AUX | `Fwrite(-2, 2, "\tX")` | 2; AUX gets raw `09 58` (no tab expansion) |
| W17 | AUX queue `(^C)` | `Fwrite(-2, 2, "ab")` | 2 (no break handling on AUX) |
| W18 | PRN ready | `Fwrite(-3, 3, "abc")`, `Fwrite(3, 3, …)` | 3; printer gets `61 62 63`, CON nothing |
| W19 | PRN rejects from byte index 2 (Bconout returns 0) | `Fwrite(-3, 5, "abcde")` | 2; printer received `ab` only |
| W20 | PRN rejects immediately | `Fwrite(-3, 5, …)` | 0 |
| W21 | AUX rejects at byte 1 (test hook) | `Fwrite(-2, 3, …)` | 1 |
| W22 | any | `Fwrite(-2, 0, b)`, `Fwrite(-3, -1, b)` | 0, **-1** |
| W23 | any | `Fwrite(-4, 1, b)`, `Fwrite(-32768, 1, b)` | `EIHNDL` |
| W24 | free sft slot `6+k` | `Fwrite(6+k, 1, b)` | `EIHNDL` |
| W25 | `d = Fdup(2)` (AUX) | `Fwrite(d, 2, "ok")`; after `Fclose(d)` again | 2 (to AUX); then `EIHNDL` |
| W26 | `Fforce(1, -3)` | `Fwrite(1, 2, "ok")` | 2; to PRN |
| W27 | `Fforce(1, fh)` (file) | `Fwrite(1, 2, "ok")` | 2; into the file; `Fclose(1)` → `Fwrite(1,…)` to CON again |

### NewTOS host tests (decisions 2026-10-13; IDs the Developer implements)

Device table as in `3f-fread.md` "NewTOS implementation notes": CON real, **AUX
(1) = "port present, no cable"** (`Bconout` accepts+discards, → 1; option A) and **PRN (0) = absent-device entry** (`Bconout` → 0). CON write rule
(TASK/CON01, 2026-10-18, replaces the 2026-10-13 raw/Ctrl-C-only rule): per byte
EmuTOS `tabout` — `conbrk` (Ctrl-C/S/Q/X, BDOS typeahead), `Bconout`,
`glbcolumn`, TAB expanded to the next multiple of 8. **(=)** = same as the
EmuTOS row, **(≠)** = NewTOS value given.

| # | Call / setup | NewTOS expected |
|---|---|---|
| W1 | **(=)** `Fwrite(-1,5,"hello")` | 5; CON `"hello"` |
| W2 | **(=)** `Fwrite(1|0|4|5, 3, "abc")` | 3 each; CON |
| W3 | **(=)** CON at column 0: `Fwrite(-1,2,"\tX")` | 2; 8 spaces + `X` |
| W4 | **(=)** CON at column 0: `Fwrite(-1,4,"ab\tX")` | 4; `"ab"` + 6 spaces + `X` |
| W5 | **(=)** after `Fwrite("12345")`: `Fwrite(-1,2,"\tX")` | 2; 3 spaces + `X` |
| W6 | **(=)** CON at column 8: `Fwrite(-1,1,"\t")` | 1; 8 spaces |
| W7 | **(=)** `Fwrite(-1,3,"a\nb")` | 3; `61 0A 62` (no CR inserted) |
| W8 | **(=)** `Fwrite(-1,3,"a\rb")`, then `Fwrite(-1,1,"\t")` | 3, then 1; bytes `61 0D 62`, then 7 spaces (column 1 after the CR) |
| W9 | **(=)** `Fwrite(-1,0,b)` | 0, no output |
| W10 | **(=)** `Fwrite(-1,-5,b)` | **-5**, no output (returns `count` itself) |
| W11 | **(=)** `Fwrite(-1,3,"a\0b")` | 3; `61 00 62` |
| W12 | **(=)** keyboard queue `('k')`; `Fwrite(-1,1,"x")` | 1; `'k'` moved from the key ring into the BDOS typeahead, returned by the next `Fread(-1,1,…)` |
| W13 | **(=)** keyboard queue `(^C)`; `Fwrite(-1,3,"abc")` | terminate (exit -32) requested before the first byte, no byte written, the key consumed (observe the terminate hook); return value not observable by the program |
| W14 | **(=)** keyboard queue `(^S)`, `^Q` arriving while paused; `Fwrite(-1,3,"abc")` | the first byte is held until `^Q`, then 3; both keys consumed, neither stored |
| W15 | **(=)** AUX default entry (port present, no cable): `Fwrite(-2,4,"ATZ\r")`, `Fwrite(2,4,…)` (separate runs) | **4** each (every `Bconout(1,…)` returns 1); bytes discarded; CON receives nothing |
| W16 | **(=)** `Fwrite(-2,2,"\tX")` | 2 (raw, discarded; no tab expansion) |
| W17 | **(=)** CON queue `(^C)`; `Fwrite(-2,2,"ab")` | 2, and the Ctrl-C is **not** consumed (no break check on non-CON; stays queued) |
| W18 | **(≠)** PRN absent: `Fwrite(-3,3,"abc")`, `Fwrite(3,3,…)` | **0** each; CON nothing |
| W19 | test device on PRN accepting 2 bytes then returning 0 (`Bconout`) | `Fwrite(-3,5,"abcde")` → 2; device received `ab` |
| W20 | test device on PRN returning 0 at once | 0 |
| W21 | test device on AUX returning 0 from byte 1 | `Fwrite(-2,3,…)` → 1 |
| W22 | **(=)** `Fwrite(-2,0,b)`, `Fwrite(-3,-1,b)` | 0, **-1** |
| W23 | **(=)** `Fwrite(-4,1,b)`, `Fwrite(-32768,1,b)` | `EIHNDL` |
| W24 | **(=)** free sft slot `6+k` | `EIHNDL` |
| W25 | **(=)** `d=Fdup(2)` (AUX); `Fwrite(d,2,"ok")`; `Fclose(d)`; `Fwrite(d,…)` | 2 (AUX no-cable entry accepts); then `EIHNDL` |
| W26 | **(≠)** `Fforce(1,-3)`; `Fwrite(1,2,"ok")` | 0 (absent PRN) |
| W27 | **(=)** `Fforce(1,fh)` (file); `Fwrite(1,2,"ok")`; `Fclose(1)`; `Fwrite(1,…)` | 2 into the file; then CON again |
| W28 | **regression guard (hang, FS11 acceptance finding 1).** Default process, **no `Fforce`**: `Fwrite(2,5,"hello")`, then `Fwrite(2,1,"x")`, then `Fwrite(2,0x1000,buf)` | **5**, **1**, **0x1000** — each returns `count` and returns (no retry loop on a 0 result); CON receives nothing. Must complete within the test's step budget |
| WT1 | **single dispatch point.** Replace the AUX entry by a test device that accepts all bytes and captures `Bconout`: `Fwrite(-2,4,"ATZ\r")` | 4; captured `41 54 5A 0D`; CON untouched. The same fixture must also be hit by `Fwrite(2,…)`, `Fdup(2)`'d handle, `Cauxout`, `Bconout(1,…)`; and `Bcostat(1)`/`Cauxos` read the entry's status (one fixture, all paths) |
| WT2 | test AUX entry: `Fwrite(-2,2,"\tX")` | 2; captured raw `09 58` (non-CON: no tab expansion) |
| WT3 | PRN entry replaced: `Fwrite(-3,3,"abc")`, `Fwrite(3,…)`, `Cprnout('a')`, `Bconout(0,…)` | all reach entry 0, none entry 1 |
| WT4 | absent entries: byte `n` is read from the user buffer **before** the device call (EmuTOS evaluates `*p++` as argument, `:634`): `Fwrite(-2,3,p)` with `p` unreadable at byte 0 | `FAULTED` (existing ARCH03 3a convention), not 0 |

Note W18 (and W26): the value 0 here is a **short count**, per the rule
`Bconout`==0 → return bytes written so far (`:634-635`). It is the intended
result of the PRN absent-device profile, not an error code. (AUX is no longer absent: option A, below.)

---

## Source extraction summary

`Fwrite` transfers up to `len` bytes from `ubufr` to the file at handle `h`,
starting at the current position, and returns the number of bytes actually
written. `xwrite` looks up the OFD (invalid → `EIHNDL`) and — unlike `ixread`
— calls `ixwrite` unconditionally, which forwards to `xrw(1, …)`. `xrw` walks
the cluster chain, calling `nextcl` to allocate fresh clusters whenever the
write runs past the currently allocated end. As bytes are written past the old
file length, `addit` grows `dfd->o_fileln` and sets `O_DIRTY`, so the new size
and time are flushed by `Fclose`.

The return value is `o_bytnum - bytpos` (bytes moved). A return **less than
`len`** signals a short write, typically because the disk is full: `xrw` stops
when `nextcl` can no longer allocate a cluster and returns the partial count.
There is no dedicated "disk full" error code from `Fwrite`; the caller must
compare `ret == len`.

There is no explicit `len <= 0` guard (`ixwrite` has none, unlike `ixread`);
`xrw` with `len == 0` simply moves nothing and returns `0`. A source comment
(`bdos/fsio.c:536-538`) documents that EmuTOS stopped rejecting writes to a
read-only-*opened* handle for Atari-TOS compatibility — the `o_mod` access code
is not enforced. Physical write faults (write-protect, media error) come back
through the BIOS `longjmp` path, not as an inline `xwrite` return.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x40
- **Stack frame on entry (m68k):** `Fwrite(handle, count, buf)` =
  `trap1(0x40, handle, count, buf)` (`include/bdosbind.h:41`):
  ```
  +8: buf    (LONG, pointer)
  +4: count  (LONG)
  +2: handle (word)
   0: function_id (word) = 0x0040
  ```
- **Total parameter bytes:** 10 (WORD + LONG + LONG) → `wparms = 5`.
- **Native (AArch64) ABI:** ID `0x40` in `x8`; args in `x0..x2`; LONG result
  (bytes written or negative error) sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name    | Type / size |
  |------------|-------------|---------|-------------|
  | +2         | `x0`        | `h`     | `WORD` (16) |
  | +4         | `x1`        | `len`   | `LONG` (32) |
  | +8         | `x2`        | `ubufr` | `void *` (ptr) |

---

## Inputs

| Position | Name    | Type    | Size | Source location    | Notes |
|----------|---------|---------|------|--------------------|-------|
| arg0     | `h`     | `int`   | 2    | `bdos/fsio.c:528`  | file handle |
| arg1     | `len`   | `long`  | 4    | `bdos/fsio.c:554`  | bytes to write; `<= 0` transfers nothing |
| arg2     | `ubufr` | `void *`| 4    | `bdos/fsio.c:528`  | source buffer; not validated |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** bytes actually written (`0 .. len`) on success; negative error.
- **Source location:** `bdos/fsio.c:281-283` (`xrw` returns `o_bytnum - bytpos`).

### Side effects on global state

- OFD position advances; `dfd->o_fileln` grows and `O_DIRTY` is set on extend
  (`bdos/fsio.c:42-46`). New clusters are linked into the FAT chain.

### Side effects on hardware

- Sector reads (for partial head/tail sectors) and writes via the buffer cache.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Invalid handle (`getofd` NULL) | `bdos/fsio.c:540-543` | `EIHNDL` (-37) |
| Handle `<= -4` / free sft slot (before `xwrite`) | `bdos/bdosmain.c:592-593,601-602` | `EIHNDL` (-37) |
| Character device (-1..-3, std handles by default) | `bdos/bdosmain.c:622-640` | see "Character-device path" (`Bconout`==0 → short count) |
| Disk full (cannot allocate cluster) | `bdos/fsio.c` (`xrw` stops, `nextcl` fails) | **short count**, not an error code |
| Write-protect / media fault | via `longjmp_rwabs` (`bdos/fs.h:60`) | BIOS code (e.g. `EWRPRO` -13, `EWRITF` -10) |

**Error paths not in Compendium:** disk-full is reported as a **short byte
count**, never as a negative code, from `Fwrite` itself.

---

## Verification criteria

- **Nominal:** `Fwrite(h, 100, buf)` on a file with room returns 100 and
  extends the file; a following `Fclose` persists the new length.
- **Extend past EOF:** writing beyond the current end allocates clusters and
  grows `o_fileln`.
- **Short write on full disk:** on a nearly full volume, `Fwrite(h, N, buf)`
  returns `< N`; the caller detects disk-full by `ret != N`.
- **Zero len:** `Fwrite(h, 0, buf)` returns 0, no side effects.
- **Write to read-only-opened handle succeeds** (mode not enforced).
- **Error:** `Fwrite(9999, 1, buf)` returns `EIHNDL` (-37).
- **Character devices:** EmuTOS reference W1..W27; NewTOS host tests W1..W28 + WT1..WT4 (see "NewTOS host tests").

---

## Edge cases discovered in source

0. **Character devices are writable via `Fwrite` (added 2026-10-13).** Handles
   -1/-2/-3 and the standard handles 0..5 (default mapping) never reach
   `xwrite`; see "Character-device path" for `tabout`/`Bconout` routing, the
   short-count rule, and `count <= 0` returning `count` itself.

1. **Disk full is a short count, not an error.** `xrw` returns the bytes it
   managed to write; subsequent writes at the full boundary return 0
   (`bdos/fsio.c`).

2. **No explicit `len <= 0` guard.** Unlike `ixread`, `ixwrite` passes straight
   to `xrw`; `len == 0` moves nothing and yields `0` (`bdos/fsio.c:554-557`).

3. **Read-only-opened handles are writable.** The `o_mod` access code is not
   enforced; documented Atari-TOS compatibility (`bdos/fsio.c:536-538`).

4. **Length update is in-memory until flush.** `o_fileln` and time are written
   to the directory entry only by `Fclose` (or an internal buffer flush) — a
   crash before close loses the size update.

5. **No truncate syscall.** GEMDOS has no "set length" call; shrinking a file
   requires `Fcreate` (which deletes + recreates empty). Writing 0 bytes does
   **not** truncate.

6. **Shared-handle position hazard.** As with `Fread`, handles sharing one OFD
   share `o_bytnum`; concurrent writes must be serialised by the caller.

---

## Dependencies

### Requires (must be initialized / functional)

- A valid OFD; FAT free-cluster state; buffer cache + `Rwabs`.

### Used by

- Every program that writes files; the loader when creating files; AES.

---

## Related data structures

- `docs/api/structures/ofd.md` — OFD/DFD position + length fields, `O_DIRTY`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the
  `(handle, count, buf)` ABI and "returns bytes written / short = disk full".)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None; behaviour independent of build flags.

---

## NewTOS implementation notes

(Empty during Phase 00.)

FS11 (2026-10-13, user decisions; `TASK/FS11.md`, follow-up `TASK/IO01.md`).
Before this change NewTOS `fwrite` (`tos-api/src/bdos/fs/file.rs:1072`,
`console_write`) sent every device handle raw to the console. Decisions 1-3 are
approved by the user (2026-10-13).

**1. One per-BIOS-device dispatch point.** `Fwrite` on `-1..-3` and on standard
handles (also after `Fdup`/`Fforce` to a device) resolves to a BIOS device number
(`HXFORM`, `bdos/console.h:36`: CON -1 → 2, AUX -2 → 1, PRN -3 → 0) and calls that
device-table entry's `Bconout`, the same table `Bconout`/`Cauxout`/`Cprnout`/
`Midiws` use (device table and the absent-device profile are defined in
`3f-fread.md` "NewTOS implementation notes"; not repeated). No GEMDOS special case
for AUX/PRN. Documented, deliberate profile (not a stub): PRN and MIDI entries return 0 from `Bconout` like EmuTOS' `charout_dummy`
(`bios/chardev.c:55-58`; AUX map entry `bios/serport.c:96`; PRN without
`CONF_WITH_PRINTER_PORT` `bios/parport.c:144-172`). Hence, *derived* from the
`osif` algorithm (`:622-640`): `Fwrite(-3|3, n>0, b)` = **0** (PRN: the first byte is refused, nothing is retried). **AUX revision (option A, user decision 2026-10-13, after FS11 acceptance finding 1,
`measurements/measurement-2026-10-13-libc-main-fs11-acceptance/RUNINFO.TXT`):** the AUX entry is "port present, no cable" — `Bconout(1,b)` accepts, discards, returns 1 (EmuTOS `bconout1` with an MFP port, `bios/serport.c:289-330`, returns `1L`), so `Fwrite(-2|2, n>0, b)` = **n**. The earlier 0 made mintlib's write loop on default handle 2 (stderr) retry forever. Full AUX profile: `3f-fread.md` "NewTOS implementation notes" item 1a;
`n <= 0` returns `n` itself (0, or the negative count) on every device. A later
driver (`TASK/IO01.md`) only swaps the entry; short-count semantics are already
in place (tests W19-W21, WT1-WT3).

**2. CON write = EmuTOS `tabout`.** FS11 (2026-10-13) had approved a raw write
with a Ctrl-C-only break here; **revoked by user decision 2026-10-18
(TASK/CON01)**. For `Fwrite` to CON NewTOS runs `tabout(2, byte)` per byte
(`:631`), the same path as `Cconout`/`Cconws` (`docs/api/gemdos/console/
02-cconout.md`, `09-cconws.md`): `conbrk` (Ctrl-C flushes the typeahead and
requests `Pterm(-32)` — the call stops, returning the bytes written before it;
Ctrl-S pauses until Ctrl-Q; Ctrl-X flushes; other keys go into the BDOS
typeahead), the byte to `Bconout`, `glbcolumn[2]` updated, TAB expanded from the
tracked column. Return value: `count` (as EmuTOS, `tabout` is void). Non-CON
devices have no break check at all (as EmuTOS, `:633-635`).

**3. Edge cases follow EmuTOS:** `count <= 0` → returns `count` itself (also
negative, e.g. -5; not an error code); the full 32-bit `count` is used (no HUGE
guard, unlike `Fread`); embedded NULs are written; a byte is read from the user
buffer before the device call (unreadable byte → `FAULTED`, ARCH03 3a, bytes
written so far remain written).

Host tests the Developer implements for this spec: **W1..W28, WT1..WT4** (table
"NewTOS host tests").

PHASE-05: FAT write path with explicit cluster allocation. NewTOS may add an
`Fsync`-style extension later; existing programs rely on `Fclose` for flush.

HW03 (2026-09-27): the hard-error path is wired. EmuTOS reaches it with
`longjmp_rwabs` (`bdos/fs.h:60`), which cannot be reproduced in Rust; instead
the sector cache records the BIOS code of a failed access
(`tos-api/src/bdos/fs/cache.rs`, the reimplementation of EmuTOS' `rwerr` /
`errcode` globals) and `fwrite` returns it. The distinction the table above
draws is therefore live: a **full disk** is still a short byte count, a
**device error** is `EWRITF` (-10) / `EDRVNR` (-2) / `ESECNF` (-8) — whatever
the driver reported. Driven by `tos-api/tests/gemdos_fs_device_error.rs`.
Before HW03, `Fwrite` returned the full byte count for data that never left
the machine (`docs/api/hw/emmc-audit.md` finding 5).

FS08 (2026-10-07): same engine as `Fread` (`_state.md` §3a): whole records are
written in one block request straight from the caller's buffer (any cached
copy in that range written back if dirty and invalidated first, EmuTOS
`usrio`), head/tail records are read-modify-write through the data list, new
clusters come from `nextcl`/`findfree` exactly as in EmuTOS (first cluster from
the FAT start, later ones forward from the current cluster). The end-of-call
write-back of the cache (HW03, above) is kept.

**PROC04 point 3 re-validation (2026-10-18) — the ticket's claim is obsolete.**
TASK/PROC04 point 3 ("`Fread`/`Fwrite` read the handle as u16; a negative BIOS
handle gives `EIHNDL`; AUX/PRN land on the console") described the state before
FS11 (2026-10-13). Re-read against both sides today: EmuTOS
`bdos/bdosmain.c:571-640` (signed `short` handle, `numl` resolution `:578-593`,
`numl < -3` → `EIHNDL` `:601-602`, device path `:604-640`) and NewTOS
`tos-api/src/bdos/fs/file.rs:393-407` / `:440-455` (`args.arg_i16(0)`, `resolve`
`:231-246` — same resolution order and the same `-3` floor) →
`gemdos/console.rs` `device_read :682` / `device_write :739` (`hxform` = `3+h`
`:620`, HUGE-read guard on the high word, `pw[3]==1` → `conin` path, else
`cgets`; `Fwrite`: CON Ctrl-C-only break + raw byte (EmuTOS: `tabout`), other devices `Bconout` with
short count on 0). Verdict: **confirmed implemented**; no repair needed. Only
residual differences, all already documented above: no tab expansion / Ctrl-C-only
break on CON (approved divergences of 2026-10-13 — revoked and aligned with
EmuTOS by TASK/CON01, 2026-10-18), AUX/PRN backends are the
absent/no-cable device-table entries (TASK/IO01), and a handle `>= NUMHANDLES`
answers `EIHNDL` where EmuTOS reads `sft[]` out of bounds (`:578-579`, contract
per `42-fseek.md`). Tests: `tos-api/tests/fs11_device_rw.rs`,
`fs10_fseek_device_handles.rs`.

# YM2149 PSG group — XBIOS 0x1C / 0x1D / 0x1E / 0x20

> Phase 00 spec (reference-level; these four are outside the 24-function
> "own-spec" set). Source: EmuTOS @ `1a7484ab` — `bios/sound.c`.
> Cite-never-paste: no C bodies; constants/register tables as data.
>
> **Reference build vs. NewTOS (SPEC07).** The pinned reference build (`make 512`,
> oracle ROM `COUNTRY=de`) has `CONF_WITH_YM2149` = **1**
> (`references/emutos-oracle/1a7484ab/include/config.h:980-981`, the default; the
> only 0-setting, `:704-705`, sits in the `!CONF_ATARI_HARDWARE` block that starts at `:673`, and `CONF_ATARI_HARDWARE` defaults to 1, `:665-667`; the oracle's `emutos.map:749-752` links
> `_giaccess`, `_ongibit`, `_offgibit`, `_dosound`). So on the oracle `Giaccess`/`Ongibit`/`Offgibit` act on a PSG
> (`bios/sound.c:95-109,116-126,131-141`) and `Dosound` installs the script and returns the old `sndtable`
> (`bios/sound.c:147-156`). The `#else` branch quoted below (`Giaccess`/`Dosound` return `0`,
> `Ongibit`/`Offgibit` do nothing, `bios/sound.c:110-112,157-159`) is **not** what the reference build runs. NewTOS has no
> YM2149 (the BCM2711 board has no such chip; ADR-012 amendment 2026-11-12 in `docs/MEMORY.md`: "no YM2149, `Dosound` and `Giaccess` stay
> plausible-zero no-ops", `SND_PSG` bit not set) and returns the same values as that `#else` branch. This is **not** the `xbios_unimpl`
> function-number pattern; callers see plausible zero defaults. **Difference to the reference build:** against the oracle every
> PSG register read/write and every `Dosound` script differs; no `docs/DEVIATIONS.md` row names it (the ADR-012 amendment is
> the only record; see Open points).

| ID    | Name      | Native ABI (`include/xbiosbind.h`) | Args | NewTOS return |
|------:|-----------|------------------------------------|------|---------------|
| 0x1C  | Giaccess  | `xbios_w_ww(28,…)` | data:WORD, reg:WORD | `0` |
| 0x1D  | Offgibit  | `xbios_v_w(29,…)`  | mask:WORD | void |
| 0x1E  | Ongibit   | `xbios_v_w(30,…)`  | mask:WORD | void |
| 0x20  | Dosound   | `xbios_v_l(32,…)`  | table:UBYTE* | `0` |

The PSG (Yamaha YM2149) on the Atari ST is both a 3-channel sound generator
and a parallel I/O port (Port A: floppy drive/side select, RTS, DTR,
parallel-port strobe; Port B: Centronics data).

## Per-function details

### `Giaccess(data, reg)` (0x1C) — vector `xbios_vecs[28]`

Low 4 bits of `reg` select PSG register 0..15; bit 7
(`GIACCESS_WRITE = 0x80`) means write, else read; `data` low byte is the
value to write. Returns the register's value. With `CONF_WITH_YM2149` the
access is bracketed by `set_sr(0x2700)` (IPL 7) because the VBL also pokes
Port A for drive select. **NewTOS: returns `0`.**

PSG register map (reference):

| Reg | Purpose | Reg | Purpose |
|----:|---------|----:|---------|
| 0/1 | Ch A frequency lo/hi(4) | 8  | Ch A volume |
| 2/3 | Ch B frequency lo/hi(4) | 9  | Ch B volume |
| 4/5 | Ch C frequency lo/hi(4) | 10 | Ch C volume |
| 6   | Noise frequency (5)     | 11/12 | Envelope freq lo/hi |
| 7   | Mixer / I/O direction   | 13 | Envelope shape |
|     |                         | 14 | Port A (drive/side/RTS/DTR/strobe) |
|     |                         | 15 | Port B (Centronics data) |

### `Ongibit(mask)` / `Offgibit(mask)` (0x1D / 0x1E) — `xbios_vecs[29/30]`

Atomic read-modify-write on PSG Port A (register 14): `Ongibit` OR-sets the
bits of `mask`; `Offgibit` AND-masks with `mask` (so pass `~bit` to clear a
bit — `bios/parport.c` calls `offgibit(~GI_STROBE)`). **NewTOS: no-op.**

Port A bit assignments (reference):

| Bit | Function | Bit | Function |
|----:|----------|----:|----------|
| 0 | Side 0/1 select | 4 | DTR (RS-232) |
| 1 | Drive 0 select  | 5 | Parallel-port strobe |
| 2 | Drive 1 select  | 6 | GPO (reserved) |
| 3 | RTS (RS-232)    | 7 | GPO (reserved) |

### `Dosound(table)` (0x20) — vector `xbios_vecs[32]`

With `CONF_WITH_YM2149`: if `(LONG)table >= 0` install `table` as the active
sound script (`sndtable = table`, `snddelay = 0`) and return the previous
`sndtable`; the script is run by the sound IRQ from the VBL. `table == -1`
is the query (returns current `sndtable` without changing it); `table ==
NULL` stops playback. **NewTOS: returns `0`** (no chip; `Dosound(NULL)` is a
consistent no-op). The script opcode format is a `bios/sound.c` reference,
not part of the NewTOS contract.

## Errors

None. No error return in any of the four.

## Edge cases

1. **IPL-7 atomicity** — the `CONF_WITH_YM2149` path locks interrupts around
   Port A access; the VBL also writes Port A. Without hardware NewTOS skips
   the SR manipulation. On SMP AArch64 a real Port A would need a spinlock,
   but there is nothing to access.
2. **Programs that poke Port A directly for drive select / RS-232
   handshake** bypass `Floprd`/`Rsconf`. They will not function on NewTOS,
   just as they would not on a Raspberry Pi — intentional ABI silence.

## Dependencies

- (Atari) PSG MMIO `0xFF8800/0xFF8802`; `sndtable`/`snddelay`/`sndirq`
  (`bios/sound.c`); `set_sr`.

Used by: drive-select, RS-232 handshake, parallel-port strobe, sound
effects, classic games' music.

## Cross-references

- **Compendium / Profibuch:** not present in `references/`; cross-check
  deferred.
- `docs/api/xbios/_dispatcher.md §3` — the `#else`-branch `0` return is
  distinct from the `xbios_unimpl` func-number return.

## Configuration

- `CONF_WITH_YM2149` = 1 in the reference build (`references/emutos-oracle/1a7484ab/include/config.h:980-981`). NewTOS: no chip, the `#else` values are returned (see the top of this file).

## NewTOS implementation notes

(Empty during Phase 00.)

**ABI choice:** EmuTOS-without-YM returns `0` (not the func-number pattern),
so NewTOS returns `0` to avoid breaking programs that probe `Giaccess(0, n)`
for `== 0`. A future NewTOS audio extension (`Dosound` stays a no-op) can
expose PCM playback via the VideoCore audio block at a reserved XBIOS slot.

---

## Open points

- Corrected (SPEC07): "`CONF_WITH_YM2149` off / NewTOS profile takes the `#else` branch" (header and *Configuration*): the reference build has it on; NewTOS merely returns the `#else` values because the Pi has no PSG.
- Cited from the source: the macro default and the linked symbols; inferred: that `make 512` sets no override (`Makefile:561` defines only `-DTARGET_512`; same inference as `_triage.md`).
- For the lead: no `docs/DEVIATIONS.md` row covers `Giaccess`/`Ongibit`/`Offgibit`/`Dosound` against the oracle (only the ADR-012 amendment, `docs/MEMORY.md`, which names the `_SND` bit).
- Code comment for the next developer: `tos-api/src/xbios/system.rs:101@YM2149` ("As on an EmuTOS build without `CONF_WITH_YM2149`, these take the `#else` branch") compares with a build the pin does not use; the values are right, the reason is the missing chip. Similar comparisons: `tos-api/src/vdi/escape.rs:705@bell`, `tos-api/src/gemdos/console.rs:552@cprnout` and `:1089@cprnos` (`CONF_WITH_PRINTER_PORT`) — not changed.

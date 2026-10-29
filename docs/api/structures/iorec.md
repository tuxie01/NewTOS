# `IOREC` — Input Output Record (typeahead ring buffer)

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `bios/iorec.h:18-27`. Returned by XBIOS `Iorec` (function 0x0E).

## Purpose

Generic ring-buffer descriptor used by the IKBD, MIDI, and (when
`BCONMAP_AVAILABLE`) RS-232 / Bconmap input drivers. The `IOREC` is
the application-visible interface to the typeahead queue: programs
can read the queue depth, install a new buffer, or flush by adjusting
`head == tail`.

## Binary layout (14 bytes on m68k)

```c
struct iorec {
/* 0x00 */ UBYTE *buf;    /* pointer to circular buffer       */
/* 0x04 */ WORD   size;   /* total buffer size in bytes       */
/* 0x06 */ volatile WORD head; /* read index (consumer)       */
/* 0x08 */ volatile WORD tail; /* write index (producer, ISR) */
/* 0x0A */ WORD   low;    /* low water mark (handshake on)    */
/* 0x0C */ WORD   high;   /* high water mark (handshake off)  */
/* 0x0E — total 14 bytes (bios/iorec.h:20-27) */
};
```

### Volatile fields

`head` and `tail` are declared `volatile` because the producer is an
interrupt handler (IKBD ACIA receive ISR or MIDI receive ISR) while
the consumer is mainline `Bconin` / GEMDOS code running at task
level. The volatile qualifier prevents the compiler from caching the
indices across the read; on m68k it suffices because aligned WORD
loads/stores are atomic. On AArch64 (NewTOS), additional acquire/
release semantics are required — `volatile` alone is not enough.

### Indices, not byte counts

`head` and `tail` are **byte offsets within `buf`**, not modular
counters. Empty: `head == tail`. Full: the producer advances by the entry size and wraps at `size`
(IKBD: `tail + 4`, `bios/ikbd.c:163-166`; one slot wasted to distinguish full
from empty).

`size` should be a power of 2; the IKBD driver uses `size = 256`
(`ikbd_bufsize`, `bios/aciavecs.S:97`), MIDI 128 (`midi_bufsize`, `:98`).
**[XBIOS07] correction:** earlier text said 128 (`KBBUFSZ`, `bdos/fs.h:46`
is a different, GEMDOS-side constant). IKBD entries are 4 bytes, so the
256-byte ring holds 63 keystrokes.

### `low` / `high` — flow control

Used only on hardware-flow-controlled input devices (RS-232 with
RTS/CTS). When `tail` advances past `high`, the driver de-asserts
RTS to throttle the sender; when `head` consumes back below `low`,
RTS is reasserted. For IKBD/MIDI the fields are set but not used
for flow control — measured on the oracle: IKBD `low`/`high` 64/192, MIDI
32/96 (`tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr.LOG`, cases 25/26;
`measurements/measurement-2026-10-29-tool16/RUNINFO.TXT`); the IKBD has no
flow control and dropped bytes are simply lost. [TOOL16]

## Dumper layout (tosdump) [TOOL16]

The layout the reference dumper reads behind `Iorec`'s returned pointer
(`docs/TOS-REFERENCE-DUMPER.md` section 6, "Structures behind a pointer").
This block is the **source**: a case table carries a copy of each line, and
the host test `layoutcheck` (`tosdump/host/layoutcheck.c`) refuses a copy
that is not this line byte for byte. Each field is
`<name>:<type>@<m68k offset>/<LP64 offset>[:<class>]`; the loader checks
every number against C on both ABIs.

- `iorec` -- m68k: the 14-byte `IOREC` above (`bios/iorec.h:20-27`);
  LP64: the SDK's `_IOREC`, 24 bytes
  (`sdk/libc/overlay/include/mint/ostruct.h:576-583@_IOREC`), the NewTOS
  contract F1 (`docs/api/xbios/0e-iorec.md`). Field names are the SDK's:
  `ibuf` = `buf`, `ibufsiz` = `size`, `ibufhd` = `head`, `ibuftl` = `tail`,
  `ibuflow` = `low`, `ibufhi` = `high`.
- `extiorec` -- device 0: m68k `EXT_IOREC` (`bios/serport.h:57-65`), two
  `IOREC`s and five `UBYTE`s, 33 bytes of fields, 34 with the WORD
  alignment; LP64 56 (F1).
- Classes: `ibuf` is an address (PTR_RAM: never a contract value, F4);
  `wr5` is the SCC shadow register, reference not contract (F4): IGNORE;
  every other field EXACT.

```tosdump-layout
!layout	iorec	14/24	ibuf:p@0/0,ibufsiz:w@4/8,ibufhd:w@6/10,ibuftl:w@8/12,ibuflow:w@10/14,ibufhi:w@12/16
!layout	extiorec	34/56	in:[iorec]@0/0,out:[iorec]@14/24,baudrate:b@28/48,flowctrl:b@29/49,ucr:b@30/50,datamask:b@31/51,wr5:b@32/52:IGNORE
```

## Standard EmuTOS instances

```c
extern IOREC ikbdiorec;     /* IKBD keyboard typeahead    */
extern IOREC midiiorec;     /* MIDI in queue              */
```

Definitions in `bios/aciavecs.S:107,114`. RS-232 IORECs (when
present) live in the per-port `bconmap_root.maptab[]` entries.

`Iorec(devno)` returns `IOREC *` for the requested device:
```
0 → RS-232 (`rs232iorecptr`, an EXT_IOREC; Bconmap-dependent)
1 → IKBD (`&ikbdiorec`)
2 → MIDI (`&midiiorec`)
other → -1
```

(`bios/xbios.c:336-348`.) **[XBIOS07] correction:** earlier text had
0 = MIDI, 1 = AUX, 2 = IKBD and "3/4 reserved"; the source returns `-1` for
every other `devno`.

### IKBD scancode encoding

The IKBD IOREC uses **4-byte entries** (a LONG), not byte or WORD: the
push advances `tail` and `head` by 4 and wraps at `size`
(`bios/ikbd.c:163,269`, `ikbdiorec.tail + 4`). **[XBIOS07] correction:**
earlier text said "WORD-sized entries ... advance by 2 bytes". So `tail - head`
in bytes is four times the queue depth in keystrokes and programs reading
the IOREC advance by 4 bytes per dequeue. Entry content layout: see
`docs/api/xbios/0e-iorec.md` (not re-derived here).

## Typical use patterns

### Query queue depth

```c
IOREC *r = (IOREC *)Iorec(2);   /* IKBD */
WORD depth = r->tail - r->head;
if (depth < 0) depth += r->size;
```

### Flush

```c
r->head = r->tail;     /* drop everything */
```

(must be done with interrupts off in single-CPU IKBD context;
multi-CPU NewTOS will need a lock.)

### Install custom buffer

```c
r->buf  = my_buffer;
r->size = sizeof(my_buffer);
r->head = r->tail = 0;
```

(Must be done with the source ISR disabled.)

## Cross-references

- **Compendium:** "Iorec" / "IOREC" — agrees on layout.
- **Profibuch:** "IOREC" — agrees.
- **`docs/api/xbios/_dispatcher.md`** — XBIOS function `0x0E Iorec`.
- **`docs/api/bios/02-bconin.md`** — IKBD driver dequeues from
  `ikbdiorec`.
- **EmuTOS source:** `bios/iorec.h` (struct), `bios/aciavecs.S`
  (`ikbdiorec` definition + producer ISR), `bios/midi.c`,
  `bios/xbios.c:iorec` (table mapping devno → IOREC *).

## NewTOS implementation notes

**[XBIOS01, 2026-10-29]** `Iorec(0/1/2)` returns records in one page mapped
read-write into every process (`0x7D000`), laid out as the LP64 `_IOREC` of the
"Dumper layout" block above (24 bytes; device 0 56); the IKBD record is the live
console key queue, 4-byte entries, 63 keys. The kernel never trusts the page:
it checks `ibuf`/`ibufsiz`/`ibufhd`/`ibuftl` before every use and repairs and
warns once per process. Details, decisions E1-E6 and cites:
`docs/api/xbios/0e-iorec.md`, "NewTOS implementation"; code
`tos-api/src/xbios/iorec.rs`.

**Concurrency note for PHASE-02 / PHASE-07** (XBIOS01: the kernel side uses
acquire/release 16-bit atomics on `head`/`tail` and masks IRQs around each ring
operation, `tos-api/src/xbios/iorec.rs`): AArch64 requires explicit
memory ordering. The producer (interrupt handler) must use
release-store on `tail`; the consumer must use acquire-load. Plain
`volatile` (as in EmuTOS) is **insufficient** on weakly-ordered
architectures. This becomes an ABI quirk: applications that
hand-write IOREC code expecting m68k volatile semantics will
malfunction under the m68k user emulator unless the emulator inserts
the appropriate barriers.

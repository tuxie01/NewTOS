# `IOREC` — Input Output Record (typeahead ring buffer)

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `bios/iorec.h:18-27`. Returned by XBIOS `Iorec` (function 0x0E).

## Purpose

Generic ring-buffer descriptor used by the IKBD, MIDI, and (when
`BCONMAP_AVAILABLE`) RS-232 / Bconmap input drivers. The `IOREC` is
the application-visible interface to the typeahead queue: programs
can read the queue depth, install a new buffer, or flush by adjusting
`head == tail`.

## Binary layout (16 bytes)

```c
struct iorec {
/* 0x00 */ UBYTE *buf;    /* pointer to circular buffer       */
/* 0x04 */ WORD   size;   /* total buffer size in bytes       */
/* 0x06 */ volatile WORD head; /* read index (consumer)       */
/* 0x08 */ volatile WORD tail; /* write index (producer, ISR) */
/* 0x0A */ WORD   low;    /* low water mark (handshake on)    */
/* 0x0C */ WORD   high;   /* high water mark (handshake off)  */
/* 0x0E — total 16 bytes */
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
counters. Empty: `head == tail`. Full: `(tail + 1) % size == head`
(one slot wasted to distinguish full from empty). Index advance:
`idx = (idx + 1) % size`.

`size` should be a power of 2; the IKBD driver uses `size = 128`
(`KBBUFSZ = 64 WORD` = 128 bytes — comment in `bdos/fs.h:46`).

### `low` / `high` — flow control

Used only on hardware-flow-controlled input devices (RS-232 with
RTS/CTS). When `tail` advances past `high`, the driver de-asserts
RTS to throttle the sender; when `head` consumes back below `low`,
RTS is reasserted. For IKBD/MIDI, `low` and `high` are zero and
ignored — the IKBD has no flow control and dropped bytes are simply
lost.

## Standard EmuTOS instances

```c
extern IOREC ikbdiorec;     /* IKBD keyboard typeahead    */
extern IOREC midiiorec;     /* MIDI in queue              */
```

Definitions in `bios/aciavecs.S:107,114`. RS-232 IORECs (when
present) live in the per-port `bconmap_root.maptab[]` entries.

`Iorec(devno)` returns `IOREC *` for the requested device:
```
0 → MIDI
1 → AUX (Bconmap-dependent)
2 → IKBD (only the **first** WORD of each entry: scancode)
3 → reserved
4 → reserved (MIDI in modern EmuTOS)
```

(Values per Compendium / `bios/xbios.c:iorec`.)

### IKBD scancode encoding

The IKBD IOREC uses **WORD-sized entries**, not byte: each enqueued
event is one WORD with `(make_code << 8) | ascii`. So `tail - head`
in bytes is twice the queue depth in keystrokes. Programs reading
the IORec must advance by 2 bytes per dequeue.

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

(Empty during Phase 00.)

**Concurrency note for PHASE-02 / PHASE-07**: AArch64 requires explicit
memory ordering. The producer (interrupt handler) must use
release-store on `tail`; the consumer must use acquire-load. Plain
`volatile` (as in EmuTOS) is **insufficient** on weakly-ordered
architectures. This becomes an ABI quirk: applications that
hand-write IOREC code expecting m68k volatile semantics will
malfunction under the m68k user emulator unless the emulator inserts
the appropriate barriers.

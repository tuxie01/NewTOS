# `Midiws` (XBIOS 0x0C)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/midi.c:104` (`midiws`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, `void` return, the **count = bytes − 1** (unsigned)
> convention. Reference: `bconout3` and the MIDI ACIA transport.

## EmuTOS source

**Primary location:** `bios/midi.c:104` —
`void midiws(WORD cnt, const UBYTE *ptr)`, slot 0x0C of `xbios_vecs[]`
(`bios/xbios.c:1298`).

**Installed handler / vector:** (none) — direct table entry. Each byte goes
through `bconout3` (the MIDI BIOS output handler, handle 3).

**Helpers called:**
- `bios/midi.c` — `bconout3(3, byte)` — write one byte to the MIDI ACIA
  (`CONF_WITH_MIDI_ACIA`), blocking on TX-ready.

**Headers / type definitions used:** none specific.

**System variables / globals read/written:** none.

**Build configuration affecting this function:**
- `CONF_WITH_MIDI_ACIA` — the transport. Without it (NewTOS profile) the
  bytes go nowhere; `Midiws` is effectively a no-op.

---

## Source extraction summary

`Midiws` writes a byte string to the MIDI port. `cnt` is the number of bytes
to write **minus one**, treated as **unsigned** ("just like Atari TOS"). The
loop sends `*ptr++` via `bconout3(3, …)` and continues while `cnt--`, so it
always sends at least one byte and, for `cnt == 0xFFFF`, sends 65536 bytes.
Each byte blocks on the MIDI ACIA TX-ready flag, so the call is synchronous.
There is no return value.

On the NewTOS profile there is no MIDI ACIA, so `Midiws` transmits nothing;
it is documented here for ABI completeness.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x0C
- **Stack frame on entry (m68k):** `Midiws(cnt, ptr)` = `xbios_v_wl(12, a, b)`
  (`include/xbiosbind.h:30`):
  ```
  +4: ptr (LONG, pointer)
  +2: cnt (word)
   0: function_id (word) = 0x000C
  ```
- **Total parameter bytes:** 6 (W + L).
- **Native (AArch64) ABI:** ID `0x0C` in `x8`; args in `x0..x1`; **`void`** —
  no return value.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `cnt` | `WORD` (16) — **bytes − 1, unsigned** |
  | +4         | `x1`        | `ptr` | `const UBYTE *` (ptr) |

---

## Inputs

| Position | Name  | Type   | Size | Source location   | Notes |
|----------|-------|--------|------|-------------------|-------|
| arg0     | `cnt` | `WORD` | 2    | `bios/midi.c:104` | count of bytes **minus one**, unsigned |
| arg1     | `ptr` | `const UBYTE *` | 4 | `bios/midi.c:106` | first byte to send |

---

## Outputs

### Return value

- **Register:** none (`void`).
- **Type:** `void`.
- **Source location:** `bios/midi.c:104` (declared `void`).

### Side effects on global state

None.

### Side effects on hardware

- Writes `cnt + 1` bytes to the MIDI ACIA (with `CONF_WITH_MIDI_ACIA`),
  blocking per byte.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

No validation; a bogus `cnt` sends more/fewer bytes.

---

## Verification criteria

- **Send N bytes:** `Midiws(N-1, buf)` transmits exactly `N` bytes to the MIDI
  port in order.
- **Single byte:** `Midiws(0, &b)` sends one byte.
- **Unsigned count:** `Midiws(0xFFFF, buf)` attempts 65536 bytes.
- **NewTOS profile (BIOS04, corrected 2026-10-29):** NewTOS has no MIDI
  transmitter; the MIDI entry accepts and discards each byte, so the call
  returns without transmitting (no error). This is **not** an EmuTOS
  configuration: EmuTOS with the MIDI ACIA (the oracle build) transmits, and
  without it `Midiws` never returns (Edge case 2).

---

## Edge cases discovered in source

1. **`cnt` is bytes − 1, unsigned.** The loop sends at least one byte and
   wraps to 65536 for `0xFFFF` (`bios/midi.c:106-109`) — the same off-by-one
   contract as `Ikbdws`.
2. **Routes through `bconout3`, not the ACIA directly.** So it shares the MIDI
   output path: each byte waits `while (!bcostat3())` and is written to the
   ACIA (`bios/midi.c:84-95`). On a build without `CONF_WITH_MIDI_ACIA` the
   bytes are **not** dropped — `bcostat3` returns 0 there (`:78-79`), so the
   first `bconout3` spins for good and `Midiws` never returns (the
   `#else return 0L` at `:92-94` is unreachable). (Corrected 2026-10-29,
   BIOS04; the earlier text said "dropped".)

---

## Dependencies

### Requires (must be initialized / functional)

- The MIDI ACIA output path (`bconout3`).

### Used by

- MIDI sequencers, synth controllers, any program driving MIDI out.

---

## Related data structures

- `docs/api/xbios/0e-iorec.md` — the MIDI input record (`Iorec(2)`).

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `cnt` as
  bytes − 1.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `CONF_WITH_MIDI_ACIA` — 1 in the oracle build (`include/config.h:994-996`).
  NewTOS has no MIDI hardware but answers as the ACIA-present build does
  (BIOS04, user decision 2026-10-29: `Bconout(3)` → 1, `Bcostat(4)` → -1), with
  every byte discarded.

---

## NewTOS implementation notes

(Empty during Phase 00.)

FS11 / IO01 prep (2026-10-13, user decision 1): `Midiws(cnt, ptr)` sends `cnt+1`
bytes (`cnt` unsigned 16 bit) by calling `Bconout(3, byte)` of the **MIDI
device-table entry** once per byte, results ignored, `void` -- exactly the shape
of `midiws()` (`bios/midi.c:104-110`, `bconout3(3, *ptr++)`). Since BIOS04
(2026-10-29) the entry is the MIDI port of the reference machine
(`tos-api/src/bios/chardev.rs:163@MidiPort`: `Bconout` -> 1, byte discarded,
always ready — deliberate profile, not a stub; before, the absent device with
`Bconout` -> 0), so the call returns normally and transmits nothing; the
observable result of `Midiws` (`void`, returns at once) is the same before and
after. A later MIDI driver (`TASK/IO01.md`) replaces only the entry. Each source byte is read from `ptr` before the device
call as in EmuTOS (an unreadable byte is a user-pointer fault under the ARCH03 3a
convention; the current `system::midiws()` takes no arguments and reads nothing --
code to be aligned by the Developer, not changed here).

**Inconsistencies found (semantics decided by the user; item 1 corrected in the
text on 2026-10-29, BIOS04):**
1. *(Resolved: "Behaviour on the Pi", Edge case 2 and the "NewTOS profile"
   criterion are rewritten.)* The "Behaviour on the Pi" paragraph below said the no-op is "consistent with
   EmuTOS on a `CONF_WITH_MIDI_ACIA`-off build". **That is wrong.** Without the
   ACIA, `bconout3` is `while(!bcostat3()) ;` (`bios/midi.c:86-87`) and
   `bcostat3` returns 0 (`:78-79`), so EmuTOS `Midiws` **never returns**
   (the `#else return 0L` at `:92-94` is unreachable). NewTOS' return-immediately
   behaviour corresponds to EmuTOS' `charout_dummy` (`bios/chardev.c:55-58`), not to
   EmuTOS' MIDI code. Same defect in "Edge cases" 2 ("bytes are dropped") and the
   "No-MIDI profile" criterion (they describe the NewTOS result, not EmuTOS').
2. "Current-code gaps R4" below ("`Midiws` is not in the XBIOS dispatcher") is stale:
   `MIDIWS => system::midiws()` is wired (`tos-api/src/xbios/mod.rs:88`,
   STATUS 2026-07-03).

PHASE-07 (optional): a NewTOS MIDI-over-USB or -UART path could give `Midiws`
a real transport; until then it is an ABI no-op. The `bytes-1` unsigned count
is honoured either way.

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the Atari **MIDI ACIA** (6850, `bconout3`
> transport) to the Pi. The contract above (trap ID/ABI, `void` return, the
> **`cnt = bytes − 1`** unsigned convention) is **unchanged**; the transport
> disappears. See `HARDWARE-PROFILE.md` §2, §3.9.

**Peripheral / driver.** There is **no MIDI ACIA on the BCM2711** and no MIDI
DIN hardware on the Pi-4 board (`HARDWARE-PROFILE.md` §2 "Printer / MIDI …
absent", §3.9 "MIDI ACIA absent → `Midiws`/MIDI iorec no-op"). The PL011 UART0
is the *only* serial peripheral NewTOS drives, and it is reserved as the
**debug/AUX** channel — it is **not** repurposed as a MIDI transport. So the
EmuTOS `bconout3` path (BIOS handle 3, MIDI) has **no backing transmitter**:
since BIOS04 BIOS handle 3 is a MIDI port entry that is always ready and
discards what it accepts (`tos-api/src/bios/chardev.rs:163@MidiPort`).

**Behaviour on the Pi** (rewritten 2026-10-29, BIOS04). `Midiws` transmits
nothing: each byte goes to the MIDI entry of the device table
(`tos-api/src/bios/chardev.rs:163@MidiPort`), which accepts it (`Bconout(3)` →
1) and discards it. The call returns normally (`void`), honouring the
`bytes − 1` unsigned count contract (it "consumes" `cnt + 1` bytes without
emitting them). No error is raised. This is **not** EmuTOS' behaviour on a
`CONF_WITH_MIDI_ACIA`-off build — there `Midiws` never returns (Edge case 2);
it is the ACIA-present build's result with the transmission left out, the
same answer the oracle gives a program (`Bconout(3)` 1, `Bcostat(4)` -1).

The related `Iorec(2)` MIDI **input** record exists; no driver fills its ring,
and `Bconstat(3)`/`Bconin(3)` read it as `bconstat3`/`bconin3` do
(`docs/api/bios/02-bconin.md` BIOS04 note, `docs/api/xbios/0e-iorec.md`).

### Current-code gaps → Step-2

- **R4 (GAP):** `Midiws (0x0C)` is **not in the XBIOS dispatcher**
  (`tos-api/src/xbios/mod.rs:33-55`) → returns **`EINVFN`**. The contract is a
  **`void`/no-op** (send nothing, no error), so a caller currently sees a
  spurious `EINVFN` instead of the silent no-op. Fix: wire a handler that
  consumes the args and returns `0` (void), leaving the `bytes − 1` count
  semantics documented but inert.

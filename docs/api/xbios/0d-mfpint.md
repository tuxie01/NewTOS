# `Mfpint` (XBIOS 0x0D) and `Xbtimer` (XBIOS 0x1F)

> Spec for TASK/BIOS15 (ADR-031 "OS hooks installed by programs",
> `docs/MEMORY.md`; return value: user decision 2026-10-29). Source: EmuTOS @
> `1a7484ab`. Verified against `bios/mfp.c:114` (`mfpint`) and
> `bios/mfp.c:163` (`xbtimer`); return values measured on the oracle
> (`measurements/measurement-2026-10-29-bios15/RUNINFO.TXT`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md),
> "OS hooks installed by programs". Contract: trap ID/ABI, the value in `d0`.
> Reference: the MFP itself (no MFP on NewTOS, CLAUDE.md non-goals).

## EmuTOS source

**Primary locations:**
- `references/emutos/bios/mfp.c:114-120` — `void mfpint(WORD num, LONG vector)`:
  `num &= 0x0F; jdisint(num); *(LONG *)((0x40L + num)*4) = vector;
  jenabint(num);`
- `references/emutos/bios/mfp.c:163-169` — `void xbtimer(WORD timer, WORD
  control, WORD data, LONG vector)`: `timer` outside 0–3 returns at once;
  otherwise `setup_timer(MFP_BASE, timer, control, data)` and
  `mfpint(timer_num[timer], vector)`, `timer_num[] = { MFP_TIMERA (13),
  MFP_TIMERB (8), MFP_200HZ (5), MFP_TIMERD (4) }` (`bios/mfp.c:161`,
  `include/biosdefs.h:181-192`).
- Slots `0x0D` and `0x1F` of `xbios_vecs[]` (`bios/xbios.c:1300`, `:1327`),
  `#if CONF_WITH_MFP` (the make-512 profile has it).
- `jenabint` = `enable_mfp_interrupt(MFP_BASE, num)` (`bios/mfp.c:59-72`,
  `:127-130`): for `num` 0–7 `ierb |= mask; imrb |= mask`, for 8–15 the same
  on `iera`/`imra`.

## Trap interface

- **Trap number:** 14 (XBIOS). **Function IDs:** 0x0D, 0x1F.
- m68k stack: `Mfpint` +2 `num` (WORD), +4 `vector` (LONG); `Xbtimer` +2
  `timer`, +4 `control`, +6 `data` (WORDs), +8 `vector` (LONG).
- **Native (AArch64) ABI:** ID in `x8`; `Mfpint`: `num` `x0`, `vector` `x1`;
  `Xbtimer`: `timer` `x0`, `control` `x1`, `data` `x2`, `vector` `x3`
  (mintlib `trap_14_wwl`, `trap_14_wwwwl`,
  `sdk/libc/overlay/include/mint/osbind.h:351@trap_14_wwl`, `:362@trap_14_wwwwl`). Result in `x0`.

## Return value

Both functions are `void`. The value a caller finds in `d0` is what the
compiled code leaves there — read from the pinned build
(`m68k-atari-mint-objdump -d references/emutos-oracle/1a7484ab/obj/mfp.o`):

- The trap glue clears `d0` and loads the function number into it before the
  call (`references/emutos/bios/vectors.S:1323-1324`), so the upper word of
  `d0` is 0 on entry.
- `mfpint` tail-jumps to `jenabint`, whose last instructions are
  `moveb $fffa15,d1; orb d1,d0; moveb d0,$fffa15; rts` (IMRB, interrupts
  0–7; `$fffa13` = IMRA for 8–15) with `d0.w = 1 << (num & 7)` before: `d0` =
  **the interrupt mask register after the call**, zero-extended (bits 8–31
  zero).
- `xbtimer` with `timer` < 0 or > 3 returns after `movew sp@(14),d0`: `d0` =
  **`control` as an unsigned WORD**. With a valid timer it ends in `mfpint`,
  so the mask register again.

Measured with `tosdump/probes/hkprobe.c` on EmuTOS, TT and Falcon030 alike
(the mask registers while a program runs: IMRA = 0x14, IMRB = 0x60):

| Case | Call | EmuTOS `d0` |
|---|---|---|
| M02 | `Mfpint(5, timer C vector)` | `0x00000060` |
| M03 | `Mfpint(0x15, same)` | `0x00000060` |
| M04 | `Mfpint(12, RS-232 receive vector)` | `0x00000014` |
| M05 | `Xbtimer(2, 0x50, 192, timer C vector)` | `0x00000060` |
| M06 | `Xbtimer(4, 0x1234, 0, 0)` | `0x00001234` |
| M07 | `Xbtimer(-1, 0x8001, 0, 0)` | `0x00008001` |

IMRA 0x14 = bits 2 and 4 (interrupts 10 `MFP_TBE` and 12 `MFP_RBF`,
`bios/serport.c:1071-1073`); IMRB 0x60 = bits 5 and 6 (5 = timer C,
`bios/mfp.c:208`; 6 = the ACIA, `bios/aciavecs.S:217-221`).

## Side effects (EmuTOS)

The vector is written to `(0x40 + num) * 4` and the interrupt enabled;
`Xbtimer` also programs the timer. The handler is called by the MFP
interrupt.

## NewTOS implementation

`tos-api/src/xbios/mfp.rs:69@mfpint`, `:79@xbtimer`.

- **Not served** (ADR-031 table, "trap and CPU vectors via `Setexc`,
  `Mfpint`, `Xbtimer`"): there is no MFP, nothing installs or calls the
  handler. The first call per vector and process writes one `[hook] warning`
  line (`Hook::Vector(0x40 + n)`, the key `Setexc(0x40 + n)` shares).
- **Return value = the oracle's.** NewTOS keeps an image of the two mask
  registers seeded with the values measured above (`IMR_AT_RUN` = 0x1460,
  `tos-api/src/xbios/mfp.rs:44@IMR_AT_RUN`), sets the bit of the interrupt a
  call enables, as `jenabint` does, and answers the register byte,
  zero-extended; `Xbtimer` with no timer answers `control` as an unsigned
  WORD. Measured equal to the oracle for M02–M07
  (`measurements/measurement-2026-10-29-bios15/RUNINFO.TXT`).

## Open

- `Jdisint` (0x1A) and `Jenabint` (0x1B) still answer their function number
  (`tos-api/src/xbios/mod.rs` default arm); on EmuTOS they change the mask
  registers and leave a register value in `d0` too. Not part of BIOS15; they
  do not read or change the NewTOS image.
- The TT's second MFP (`tt_mfpint`) has no XBIOS entry.

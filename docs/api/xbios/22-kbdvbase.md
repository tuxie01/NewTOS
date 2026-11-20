# `Kbdvbase` (XBIOS 0x22) — the keyboard/IKBD vector table

> **Status: APPROVED (owner 2026-10-04)** — D1–D8 as recommended: table per process (D1; first in TPA page 0 at `0x9A0`, **moved by MEM12 to the private read-write page `0x7F000`, see Part D "D1-new"**); AAPCS64, `x0` = scancode/packet pointer, `x1` = 0 for `kbdvec` until `Iorec(1)` (D2/D2a); ring of 64 entries per process + upcall chain (D3/D3a); rebuild the `ikbdsys`-change stop (D4); `drvstat` = 0 (D5); the lead assigns the syscall number at build time from the free internal range (D6); minimum now = `kbdvec` + valid stubs in all slots, other hooks a follow-up ticket, gap in STATUS "Blocked" (D7); the hook disappears with the process, recorded in COMPATIBILITY.md (D8). Earlier marking: DRAFT (KEYS02, step 1 / rule 5). At approval
> nothing of it was built; **the core part (D7 minimum) has been built since 2026-10-04 —
> Part C** (syscall number D6 = `Ssystem` mode **2002**, O1 settled). Part A (EmuTOS reference) is cited from the source; Part B
> (NewTOS mapping) is a **proposal**, every place marked `DECISION D<n>`
> needed a yes/no from the owner (list at the end).
>
> Source: EmuTOS @ `1a7484ab` (`references/emutos`), FreeMiNT
> (`references/freemint`), Atari Compendium as tos.hyp (WebFetch, see
> "Cross-references"). Edited 2026-10-04 by the researcher; replaces the
> Phase-00 version of this file (corrections: section "Corrections to the
> earlier version").
>
> **Contract vs. internals** (`docs/COMPATIBILITY.md`): the contract is what a
> program observes: trap ID, the returned pointer, order of the slots,
> `kbdvec` directly before slot 0, register contents at the call, bit 7 = break,
> chainability (read a slot, save it, call it later). Internal (free): ACIA,
> MFP interrupt, layout of the packet buffer in BSS.

---

# Part A — EmuTOS reference (cited)

## A1. Call and return

`Kbdvbase()` = XBIOS 0x22, no parameters (`bios/xbios.c:716-719`:
`return (LONG)&kbdvecs;`), slot 0x22 of the table `xbios_vecs[]`
(`bios/xbios.c:1337`). Result in D0: pointer to `kbdvecs`. Stable (the pointer
never changes, the structure lives in BSS: `bios/aciavecs.S:126-141`).

## A2. Memory layout around the pointer

`bios/aciavecs.S:36-57` (comment) and `:126-141` (definition). The returned
pointer points to `midivec`; **before** it lies `kbdvec`, and with
`CONF_WITH_EXTENDED_MOUSE` before that `mousexvec`:

```
  kbdvbase-8   mousexvec   (only #if CONF_WITH_EXTENDED_MOUSE, aciavecs.S:126-128)
  kbdvbase-4   kbdvec      (aciavecs.S:129, "must be right before _kbdvecs")
  kbdvbase+0x00 midivec    MIDI input                 (aciavecs.S:131)
  kbdvbase+0x04 vkbderr    IKBD error                 (:132)
  kbdvbase+0x08 vmiderr    MIDI error                 (:133)
  kbdvbase+0x0C statvec    IKBD status                (:134)
  kbdvbase+0x10 mousevec   mouse                      (:135)
  kbdvbase+0x14 clockvec   IKBD clock                 (:136)
  kbdvbase+0x18 joyvec     joystick                   (:137)
  kbdvbase+0x1C midisys    main MIDI vector           (:138)
  kbdvbase+0x20 ikbdsys    main IKBD vector           (:139)
  kbdvbase+0x24 ikbdstate  byte, packet status        (:140)
  kbdvbase+0x25 kbdlength  byte, bytes left in packet (:141)
```

All slots are 4-byte pointers (`.ds.l`) on the m68k. `struct kbdvecs`
(`include/biosdefs.h:105-116`) holds only the nine documented slots;
`kbdvec`, `mousexvec`, `ikbdstate`, `kbdlength` are **not** in the C structure.

Configuration dependency: `mousexvec` only with `CONF_WITH_EXTENDED_MOUSE`
(`include/config.h:1496-1497`: default 1; according to `docs/api/_emutos-config.md:183,255`
0 in some machine profiles). `midisys` is set only with `CONF_WITH_MIDI_ACIA`
(`aciavecs.S:180-187`), `ikbdsys` only with `CONF_WITH_IKBD_ACIA`
(`:188-195`, default 1: `include/config.h:1001-1002`), `clockvec` only with
`CONF_WITH_IKBD_CLOCK` (`:169-178`, default 1: `include/config.h:1008-1009`); otherwise
the slot stays `just_rts` or (midisys/ikbdsys) unset = 0 (BSS). The
Falcon profile (target image, `_emutos-config.md`) has the ACIA, the IKBD clock and
the extended mouse on.

`ikbdstate` (Compendium: `drvstat`): source comment `aciavecs.S:54-57`;
tos.hyp lists `drvstat` as a **documented** field of `KBDVBASE` directly behind
`ikbdsys` ("non-zero when the IKBD is in the process of sending a packet",
`xbios_structures.html#KBDVBASE`). EmuTOS comment: non-zero = a packet is
being received right now (not the count, `aciavecs.S:54-57`).

## A3. Default assignment (`_init_acia_vecs`, `aciavecs.S:146-223`)

| Slot | Default | Evidence |
|---|---|---|
| `kbdvec` | `_kbdvec` -> `kbd_int` | `:154-158`, `_kbdvec` `:372-376` |
| `midivec` | `_midivec` (writes into the MIDI iorec) | `:160-164`, `:315-332` |
| `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `joyvec` | `just_rts` (`rts`) | `:148,165-168,179`, `just_rts:` `:377` |
| `clockvec` | `_clockvec` (`bios/clock.c:969`) or `just_rts` | `:169-178` |
| `midisys` | `_midisys` (MIDI ACIA only) | `:180-187` |
| `ikbdsys` | `_ikbdsys` (IKBD ACIA only) | `:188-195` |
| `mousexvec` | `just_rts` (the VDI replaces it: `vdi/vdi_mouse.c:611`) | `:150-152` |

`ikbdstate` is set to 0 (`:206-207`). Then the IKBD data register is read empty
(`:210-214`) and the ACIA interrupt is hooked in (`_mfpint(6, _int_acia)`,
`:217-221`). The slots are assigned in `init_acia_vecs()`
(`bios/bios.c:457-458`), after `kbd_init()` (`:453-454`).

## A4. When which vector is called with what

Chain (all inside the ACIA interrupt, IPL 6, ends with `rte`):

```
MFP int 6 -> _int_acia (aciavecs.S:249)
  saves d0-d3/a0-a3 (:253-259) ; comment :250-252: there is no specification
  which registers the vectors may use; d0-d1/a0-a1 would theoretically suffice
  loop (:262-273):
     [MIDI]  move.l midisys,a0 ; jsr (a0)                     (:263-266)
     [IKBD]  move.l ikbdsys,a0 ; jsr (a0)                     (:267-270)
     repeat while GPIP bit 4 == 0 (interrupt still pending)       (:271-273)
  end of interrupt (:274), restore registers (:277-283), rte
```

* `ikbdsys` (default `_ikbdsys`, `:382-400`): reads ACIA status/data itself
  (`ikbd_acia_stat/data`), when data is there it calls `ikbdraw` with **d0 = data byte**
  (`:391`); on overrun (status bit 5) it reads the data byte again and calls
  **`vkbderr` with d0 = contents of the ACIA data register** (`:393-397`).
  `ikbdsys` itself receives no arguments.
* `ikbdraw` (`:407-414`): if `ikbdstate == 0` and the byte is `< 0xF6`, it is a
  **keyboard byte** -> `a0 = &ikbdiorec`, `d0 =` byte, jump (`rts` trick, no
  `jsr`) into **`kbdvec`**. The return of the `kbdvec` routine therefore goes
  straight to the caller of `ikbdraw` (`_ikbdsys`).
* Bytes `0xF6..0xFF` start a packet (`begin_packet` `:416-432`, tables
  `:433-437`); further bytes are collected by `in_packet` (`:439-463`); at the last byte
  `got_packet` (`:465-475`) -> `action_table` (`:476-482`):

| Packet | Action | Vector | Pointer `a0` (AND as a longword on the stack) |
|---|---|---|---|
| `F6 a1..a7` | `kbd_status` `:484-487` | `statvec` | `kbdbuf+1` (behind the header, 7 bytes) |
| `F7 0b xh xl yh yl` | `kbd_abs_mouse` `:488-489` | `mousevec` | `kbdbuf+1` (behind the header) |
| `F8..FB dx dy` | `kbd_rel_mouse` `:490-492` | `mousevec` | `kbdbuf` (**including** the header byte) |
| `FC yy MM dd hh mm ss` | `kbd_clock` `:493-496` | `clockvec` | `kbdbuf+1` (6 bytes) |
| `FD j0 j1` | `kbd_joys` `:513-514` | `joyvec` | `joybuf` = `[j0, j1]` |
| `FE bj` | `kbd_joy0` `:515-517` | `joyvec` | `joybuf` = `[0xFE, bj, (old 3rd value)]` |
| `FF bj` | `kbd_joy1` `:518-519` | `joyvec` | `joybuf` = `[0xFF, (old 2nd value), bj]` |

  Call: `kbd_jump_vec` (`:527-533`): `move.l a0,-(sp); jsr (a1); addq.l #4,sp;
  clr.b ikbdstate`. So the pointer is **in a0 and as a C argument on the
  stack**; return by `rts`; the return value is not evaluated. Afterwards
  `ikbdstate` is cleared. **Undocumented, "up to at least TOS 3.06"**
  (`aciavecs.S:508-510`): on entry to `joyvec` d0.b holds the last received
  data byte. (That d0 also still holds the last byte for the other packet
  vectors follows from the code path `:465-475` -> `:527`, but is not commented
  there as a promise: **read, not contract**.)
* `midisys` (`:295-312`): reads the MIDI ACIA, calls `midivec` with **d0 = data byte**
  (`:302-304`), on overrun `vmiderr` with d0 = data register (`:307-310`).
* tos.hyp (Compendium) on this: address of the packet at `clockvec`/`joyvec` in A0
  **and** on the stack, end the routines with `RTS`, if possible not longer than
  1 ms (`xbios_structures.html#KBDVBASE`); before changing a vector
  `drvstat == 0` must hold, disable interrupts and check again
  (`xbios_interface.html`, section Kbdvbase). *(Both places were read as a
  WebFetch summary, not checked verbatim.)*

### The default `kbdvec`

`_kbdvec` (`aciavecs.S:372-376`): `move.w d0,-(sp); jsr _kbd_int; addq.l #2,sp;
rts` -> `void kbd_int(UBYTE scancode)` (`bios/ikbd.c:725`). The byte is an
**IKBD scancode**: bit 7 set = break (`KEY_RELEASED 0x80`, `ikbd.c:48`,
`:777`), else make. `kbd_int` (`ikbd.c:725-910`) does, in order:

1. Ctrl-Alt-Del / Ctrl-Alt-RShift-Del -> `warm_reset()`/`cold_reset()` (`:734-747`; exact key-state conditions in `docs/api/bios/_reset-warm-cold.md` §2);
2. scancodes `0x37,0x5e,0x5f,0x59,0x5a,0x5c,0x5d` (`CONF_WITH_EXTENDED_MOUSE`) ->
   `mousexvec(scancode)` and **return** (`:748-760`);
3. break (`:777`): clear modifier bits (`:779-806`), Alt release with
   Alt-numpad (`:788-799`), Home/Insert break stops the repeat (`:807-814`),
   other break: `if (scancode_only == kb_last_actual) kb_ticks = 0;` (`:815-817`);
   mouse emulation (`handle_mouse_mode`, `:821`); return — **a break never
   lands in the iorec**;
4. make: modifiers set `shifty` (`:825-870`); non-modifiers: `kb_last_actual =
   scancode` (`:875`), `convert_scancode` (`:887`), `kb_ticks = kb_initial`
   (`:891`), key click, remember `kb_last` and `kb_last_ikbdsys = kbdvecs.ikbdsys`
   (`:898-903`), and `push_ikbdiorec(kb_last.key)` (`:909`, format
   `shifty<<24 | scancode<<16 | ascii`, `ikbd.c:428-434`; iorec push with
   discard-when-full `:157-173`).

## A5. Key repeat and `ikbdsys`

`do_key_repeat()` (`ikbd.c:451-528`, 50 Hz from `kb_timerc_int` `:533-536`):

* `:453`: `if (kb_ticks > 0 && kbdvecs.ikbdsys != kb_last_ikbdsys)` -> `kb_ticks = 0`,
  return. Comment `:454-470`: the cause was FreeMiNT, which hooks in its `ikbdsys`
  while a key is held. **Only `ikbdsys` is compared,
  not `kbdvec`**: a game that replaces `kbdvec` does not stop the repeat by that.
* The repeat puts the word **directly into the iorec** (`push_ikbdiorec`,
  `:524`) and **never calls `kbdvec`**. A `kbdvec` hook therefore sees
  make and break, **no** repeats (the software repeat arises only
  *behind* `kbd_int`).
* The repeat arises only if `kbd_int` ran (`kb_ticks` is set only
  there: `:891`, `kb_last*` `:898-903`). A hook that does **not** call the default
  `kbdvec` therefore gets: no iorec entries, no repeat, no update of
  `shifty`/`Kbshift` (`:779-870`), no reset check
  (`:732-742`).

## A6. What a program has to do (derived from the code, **not** given as instructions in the source)

*The following sequence is a conclusion from A2-A5, not an EmuTOS quotation.*

1. In supervisor mode read and save `p = Kbdvbase()`, `old = ((void**)p)[-1]` (the `kbdvec` slot).
   (The API returns no old vector; the source contains no mechanism that takes
   this over.)
2. Write your own routine into the slot. The routine gets **d0.b =
   scancode (bit 7 = break), a0 = iorec**.
3. Whoever wants to keep the normal keyboard handling (BIOS queue `Bconin`, AES,
   shift state, repeat) **chains**: at the end jump to the saved `old` with
   d0/a0 unchanged (the call came by the `rts` trick, `old` must return with
   `rts` to the same caller). Without chaining the rest of the TOS keyboard
   path never sees these keys (A5).
4. At program end (`Pterm`, better through the `etv_term` vector, `bdos/proc.c:
   606-607`) write the old value back. Pattern in EmuTOS itself:
   `vdimouse_exit` restores `statvec` from `old_statvec`
   (`vdi/vdi_mouse.c:638-639`); `wheel_int` chains at the end by
   `move.l _old_statvec,-(sp); rts` (`vdi/vdi_asm.S:476`). The hooks call
   no operating system functions (interrupt context).
5. Whoever does **not** chain still gets every make/break message exactly
   once per byte; repeats do not arise (A5).

## A7. Which TOS versions have `kbdvec`

* EmuTOS: always (`aciavecs.S:129`, `ikbdraw` `:407-414`). The comment calls it
  "undocumented feature of TOS >= 2.0" (`aciavecs.S:52`).
* FreeMiNT: `has_kbdvec = true` exactly when `tosvers >= 0x200` or EmuTOS
  (`references/freemint/sys/keyboard.c:1938-1941`); "TOS < 2.00 doesn't know
  about this vector" (`sys/arch/init_intr.c:94-98`). For such TOS versions MiNT hooks
  `ikbdsys` instead and calls the `KBDVEC` slot itself
  (`init_intr.c:79-87`, `sys/arch/acia.S:146-149`).
* **TT TOS 3.06 (0x306): yes, DERIVED** from the rule `tosvers >= 0x200`; no
  source says explicitly "3.06 has `kbdvec`". All that is attested is that the
  EmuTOS author knows the behaviour of the `joyvec` d0 detail "up to at least TOS 3.06"
  (`aciavecs.S:508`) — that concerns `joyvec`, not `kbdvec`.
  **Measured 2026-10-04 (KEYS02; entered 2026-10-26, closes O3): yes.**
  In the ROM `tos306de.rom` XBIOS `$22` (`$E03A96`) returns
  `move.l #$0000106A,d0 / rts`, and `ikbdraw`
  (`$E032DA`) jumps with `move.l $1066.w,-(sp) / rts` through `kbdvec` =
  `Kbdvbase()-4` — the same `rts` trick as EmuTOS `aciavecs.S:407-414`. At
  run time (Hatari 2.6.1 TT, debugger) `$1066` holds `$00E033EA`, TOS 3.06's
  `kbd_int` (shift state). Evidence
  `measurements/measurement-2026-10-04-keys02-doom/RUNINFO.TXT:75-92`,
  `measurements/measurement-2026-10-04-keys02-doom/hatari/o3-tos306-debugger.log:20,30,34`; Doom reports there
  `Kbdvbase 0000106a, kbdvec was 00e033ea`
  (`measurements/measurement-2026-10-04-keys02-tt/RUNINFO.TXT:20`).
* Exception note: FireTOS and CT60-TOS-2.xx do **not** call `kbdvec` from their
  `ikbdsys` (`init_intr.c:104-111`) — there a `kbdvec` hook has no effect.
  On those ROMs `ikbdsys` is the reliable way.
* In tos.hyp `kbdvec` was not found in the pages retrieved (only
  `drvstat`, no slot -4): confirms "undocumented".

## A8. Behaviour at program end when a hook stays installed

* **EmuTOS:** nothing. `xterm` calls `etv_term` (`bdos/proc.c:606-607`), resets
  `run`, and `ixterm` closes files, frees directories and
  `free_all_owned` (`bdos/proc.c:110-140`) — no place touches `kbdvecs`. A
  search over `*.c/*.h/*.S` in the whole tree finds `kbdvecs`/`mousevec`/
  `statvec`/`joyvec`/`ikbdsys` only in `bios/aciavecs.S`, `bios/ikbd.c`,
  `bios/mouse.c:117,120`, `bios/xbios.c:718`, `bios/clock.c`, `vdi/vdi_mouse.c`,
  `bios/amiga*.{c,S}`, `bios/lisa.c`, `bios/bios.h`, `include/biosdefs.h`; never in
  `bdos/`. **Conclusion (inferred from the absence):** the pointer stays on the
  now freed (and possibly reallocated) memory; the next key press
  jumps there. The programmer has to reset it (hence A6.4).
* **FreeMiNT:** the kernel does **not** track the hooks of individual programs; there is
  no line in `sys/` apart from `keyboard.c`, `sys/arch/init_intr.c`, `sys/arch/acia.S`,
  `sys/arch/intr.S`, `dev-mouse.c` and the USB drivers that touches `kbdvecs`
  (search `kbdvec|ikbdsys|kbdvbase` in `references/freemint/sys`). What MiNT does:
  * It hooks **its own** `kbdvec_handler`, with an XBRA header in front
    (`sys/arch/intr.S:629-639`; `install_vector`, `init_intr.c:48-57`;
    entry `:101`), and saves the old value in `old_kbdvec`. The handler
    calls `ikbd_scan(scancode, kbd_iorec)` and returns by `rts`
    (`intr.S:638-661`), **without** forwarding to `old_kbdvec`, except for
    "orphaned" breaks (`chain_oldkeys`, `intr.S:671-683`, trigger
    `keyboard.c:1030-1043`).
  * A program that hooks in *after* MiNT and chains `old` (= MiNT's
    handler) works; unhooking remains the program's business.
    (*Inferred*, not a promise in the source.)
  * At the **end of MiNT itself** the whole table is reset to the saved
    state (`restore_TOS_vectors`, `init_intr.c:243-264`),
    with the comment "BUG: we should restore *all* vectors, not just the ones
    that MiNT caught" (`:238-241`).
  * `/dev/mouse` saves `mousevec`/`joyvec` on open and restores them on
    close (`sys/dev-mouse.c:190-231`) — that is a device-close action, not a
    process-end hook for foreign programs.
  * The USB drivers (`udd/hid/mouse*/usb_mouse.c:507-522`,
    `sys/usb/src.km/udd/hid/tablet/usb_tablet.c:1010-1026`) send keys/mouse themselves through the
    `KBDVEC` structure when GEMDOS >= 0x19: that is the FreeMiNT precedent
    that a **non-ACIA driver** calls the vectors (as NewTOS' HID
    translator would). FreeMiNT structure: `struct kbdvbase` with `drvstat`
    behind `ikbdsys` (`sys/mint/struct_kbdvbase.h`).

## A9. Precedent: EmuTOS calls the vectors from non-ACIA sources too

Amiga: `move.l _kbdvecs-4,a0 ; jsr (a0)` calls `kbdvec` (`bios/amiga2.S:201-202`),
`call_ikbdraw` (`aciavecs.S:556-568`), `call_mousevec` (`:538-554`),
`call_joyvec` (`:570-590`, Amiga only). A driver without an ACIA therefore produces the
IKBD bytes and runs through the same slots — exactly the target model for NewTOS.

---

# Part B — NewTOS mapping (PROPOSAL, needed approval)

## B0. As-is state (read, 2026-10-04 — historical; line numbers of NewTOS code dropped in SPEC07, the code has moved since)

* `Kbdvbase` returned `&KBDVECS`, nine `usize` slots, **all 0**, in kernel
  memory (`tos-api/src/xbios/system.rs`), **no** `kbdvec` slot before it.
  Kernel address, mapped for EL1 only (`kernel/src/arch/aarch64/mmu.rs`;
  diagnosis `TASK/PORT06-KEYS-DIAGNOSIS.MD` question 4).
* The decoder `kernel/src/ikbd.rs:feed/decode_key` calls no vector; it puts
  makes into `console::push_rx`, discards ordinary breaks
  (`ikbd.rs`, make branch and `_ =>` break), the mouse goes directly to `vdi::feed_relative`
  (`ikbd.rs`), `0x37/0x59/...` only to the kernel-internal `mousexvec`
  (`ikbd.rs:148-163`).
* The repeat (KEYS01) also puts words directly into `push_rx`
  (`ikbd.rs`) — **as EmuTOS** (A5), it calls no vector.
* Input is **polled**: `host_env::pump_input` (`kernel/src/host_env.rs`)
  -> `usb::poll()` -> `ikbd::feed` -> `poll_repeat`, from every AES wait loop,
  every blocking read, `Bconstat` on an empty queue (KEYS01-STATUS).
  No interrupt. *(Addendum INPUT01, 2026-10-08: in addition the
  200 Hz timer interrupt pumps when it interrupts a program at EL0 and no
  upcall/no suspended AES call is running — `kernel/src/input_gate.rs`,
  switchable off with `input=aes`. It decodes **only pure pointer movement**,
  though; a report with a key byte, button change or mouse wheel is held back
  until the next pump in the trap path. Key bytes therefore still reach `route_key` and
  the hook ring only from the trap path; nothing is ever delivered from the interrupt.)*
* EL0 callback: `deliver_upcall` (`kernel/src/arch/aarch64/exceptions.rs`)
  saves all 30 GPRs + lr/elr/spsr/sp_el0 in the kernel, sets `x0..x3` from
  `UpcallDesc.args`, `elr = handler_va`, `lr = TRAMPOLINE_VA` (0x700, calls
  `Psigreturn`), SP 16-aligned + 0x20 red zone. Delivery point:
  return of a syscall to EL0 (`complete_call_tail`, `exceptions.rs:
  538-557`; also `:1041`), **not** in the interrupt. Nested up to
  `MAX_UPCALL_NEST = 8` (`tos-api/src/bdos/process.rs`), else `Pterm(-998)`.
  `Psigreturn` (`exceptions.rs:1390ff`) restores the context (including the
  syscall result in `x0`).
* **Existing consumer with the same pattern:** `vex_wheelv`
  (`tos-api/src/vdi/mouse.rs:421-428, 565-590, 960-973`): EL0 vector + slot of the
  owner, called only if the owner is the running process, the vector is returned
  to the default at process end (`release_wheel_vector`, from
  `free_slot` `process.rs`). Reason given there: an EL0 VA means nothing
  in a foreign address space.
* `post_upcall` has **one** wait slot per process, "last writer wins"
  (`process.rs`). That is not enough for key events (see B4).
* The first page of every process (TPA start, `[0, 0x1000)`) is mapped
  `UserRwExec` and holds basepage + sysvars + trampoline
  (`loader.rs`, `:730`); the kernel reads/writes it through the
  kernel alias (`bios::chardev::current_conterm`, `chardev.rs`, reads
  `$484` of the **running** process; `KEYTBL_AREA_VA`, `loader.rs:~790`
  occupies `[0x768, 0x9A0)`). `[0x9A0, 0x1000)` remains free. *(State when
  written; the table has not been in page 0 since MEM12, see Part D.)*
* App ABI: LP64 only now; ELFCLASS32 is rejected by the loader
  (`loader.rs:99-101, 397-412`), the ILP32 build path is shut down
  (`docs/MEMORY.md` ADR-020 addendum 2026-09-04).

## B1. Where the table lives  (DECISION D1)

**Proposal (superseded by Part D "D1-new"):** per process in the first TPA page, a new fixed region
`KBDVECS_AREA_VA = 0x9A0` (after the Keytbl region, before `RESERVED_LOW_END`),
written by the loader at start with the default values (like `populate_sysvars`).
`Kbdvbase()` returns **the EL0 VA** of this process (result in `x0`, full
64 bits). Layout (LP64, 8-byte slots, order as EmuTOS), offsets relative to the region start:

```
  +0x00 mousexvec   (only if kernel-internally EL0-capable; D3)
  +0x08 kbdvec      <- (void**)Kbdvbase()[-1]
  +0x10 midivec  <- Kbdvbase() points here
  +0x18 vkbderr   +0x20 vmiderr   +0x28 statvec  +0x30 mousevec
  +0x38 clockvec  +0x40 joyvec    +0x48 midisys  +0x50 ikbdsys
  +0x58 drvstat (byte, always 0, D5)   +0x59 kbdlength (byte, 0)
```

Reason (conclusion, not quotation): every process has its own address space;
a vector is an EL0 VA and valid only there (the same reason as
`vex_wheelv`, `mouse.rs`). The page belongs to the TPA and disappears with
it — **a dangling pointer is thereby impossible** (strictly better than EmuTOS,
A8). The kernel reads the slot of the running process like `current_conterm()`
through the alias (precedent). Default values see B3.
Drawback/limit (the owner must know it): a TSR/keyboard extender that wants to install a hook
for *all later* programs by `Ptermres` does not work — it could run nowhere, not even with a global table
(other address space). Irrelevant for Doom.
Alternative (rejected): global kernel table with an owner slot as for the wheel.
But then EL0 reads a kernel address (mapping needed) and Kbdvbase chaining
("remember the old value") would need kernel state per owner without gain.
ILP32: not supported (the loader rejects ELFCLASS32); the slot type is `u64`.

## B2. Calling convention for native aarch64 programs  (DECISION D2)

The replacement for `d0`/`a0`. All hooks are **AAPCS64 functions**
`void h(uint64 a, uint64 b)` returning by `ret` (trampoline `lr`):

| Vector | x0 (replaces d0 / stack arg) | x1 (replaces a0) | EmuTOS source |
|---|---|---|---|
| `kbdvec` | scancode in bits 0-7, **bit 7 = break**, upper bits 0 | pointer to the IKBD `IOREC` (D2a, since XBIOS01) | `ikbdraw` `:407-414` |
| `midivec`, `vkbderr`, `vmiderr` | byte | 0 | `:302-304, :393-397` |
| `statvec`, `mousevec`, `clockvec`, `joyvec` | pointer to packet (replaces stack argument AND a0), layout as A4 | same pointer | `kbd_jump_vec` `:527-533` |
| `midisys`, `ikbdsys` | — | — | see B6 |

`x2`/`x3` = 0. Callee-saved registers (x19-x28, sp, x29) the hook preserves
per AAPCS64; the kernel saves **all** registers of the interrupted
context (`deliver_upcall`), so the hook may change any register —
strictly more room than `d2-d7/a2-a6` under EmuTOS (`:250-259`).
D2a: `IOREC` pointer. **[XBIOS01, 2026-10-29, E6]** `x1` = the IKBD `IOREC`
(the same address as `Iorec(1)`, `tos-api/src/xbios/iorec.rs:146@IKBD_IOREC_VA`,
set in `kernel/src/arch/aarch64/exceptions.rs:871@IKBD_IOREC_VA`), as EmuTOS
`a0 = &ikbdiorec` (`bios/aciavecs.S:412`); the key queue **is** this ring since
XBIOS01 (`docs/api/xbios/0e-iorec.md`). Before: `x1 = 0`, because
`Iorec` was not wired and the queue `console::push_rx` was no `IOREC`.
The packet buffers (`statvec` etc.) lie in the first page (a new region
behind the table, or `PARMBLK`-like) and are written by the
kernel before the call.

**Chaining** (read a slot, remember it, call it later) works only if the
default slots hold **valid EL0 entry addresses** (B3). Chaining is
a tail call `br old` with x0/x1 unchanged.

## B3. Default hooks in the process (stubs, faithful to EmuTOS `just_rts`)

At load the loader (like `write_upcall_trampoline`, `loader.rs:1136ff`) writes
a small stub field (e.g. `[0xA00, 0xA40)`), and the default of the slots points
to it *(superseded by D1-new: the stubs sit in the shared stub page)*:

* `ret_stub` (one `ret`) — default for `vkbderr, vmiderr, statvec, mousevec,
  clockvec, joyvec, midivec, midisys, ikbdsys, mousexvec` (the EmuTOS default is
  `just_rts` for most, `aciavecs.S:148-168,179`; for `midisys/ikbdsys/
  clockvec/midivec` it is a real driver there, here there is no ACIA/MIDI;
  `Initmous`/AES mouse run internally through `vdi::feed_relative`).
* `kbdvec_default` — `mov x8,#<NR>; svc #1; ret`: a **new internal
  syscall (DECISION D6, number from the lead)** that executes in the kernel today's
  `decode_key(byte)` (modifiers, ASCII, `push_rx`, repeat start).
  This is the counterpart of `_kbdvec -> kbd_int`.

So "hook calls the default" = `br old` and "hook does not call it" =
faithful to EmuTOS (A5): the BIOS queue, `Kbshift`, repeat and
reset check stay off (`ikbd.rs` has no reset check anyway).

## B4. When it runs and how nothing is lost  (DECISION D3)

* Place: `ikbd::feed()`, where a byte `< 0xF6` is recognised as a key
  (now `decode_key`, `ikbd.rs:~144`). Runs in the **polled pump** (KEYS01),
  not in the interrupt; delivery at the next syscall return to EL0
  (`complete_call_tail`).
* Fast path: if `kbdvec` of the running process still holds `kbdvec_default`
  (address = constant), or there is no running process (desktop/boot,
  `current() == null`, like `current_conterm`), the kernel calls `decode_key`
  **directly** as today -> normal programs unchanged (acceptance).
* Otherwise: byte into a kernel ring per process slot (proposal 64 entries, on
  overflow discard oldest/newest -> D3a) and **one** upcall with
  `x0 = byte`. At the `Psigreturn` of the hook: if the ring is not empty, deliver the next
  byte (the same trick as the userdef chain, `exceptions.rs:
  1410ff`), else restore the context. Nesting depth stays 1.
  **Why not `post_upcall` directly:** one slot, "last writer wins"
  (`process.rs`) — a make and its break in the same poll would lose the
  make; exactly the error Doom must not have.
* New mask bit `UPCALL_MASK_KBDVEC = 1 << 3` (after `process.rs`).
* The owner test: only if the running process owns the table — trivially
  satisfied, since the kernel reads the table of the *running* process (no
  foreign access as for the wheel needed).
* Delay: delivery is limited by the poll frequency (KEYS01 promise:
  at least once per tick in every AES wait loop; Doom calls
  `evnt_multi`, `sdk/apps/doom/doomgeneric_gem.c`). **To check at build:**
  that the AES path after `evnt_multi` also runs through `complete_call_tail` (so
  upcall delivery) — not evidenced here (`exceptions.rs` shows
  it for the ordinary trap path).
* No guarantee: a hook that never returns stops the process (as on
  TOS); no watchdog proposed.

## B5. Per process or global / process end  (DECISION D1, consequence)

Per process (B1). Process end: table and stubs disappear with the TPA
(`free_slot`, `process.rs`); in addition in `free_slot` empty the kernel ring of the
slot (the way `release_wheel_vector` is called there,
`process.rs`). A pending upcall is already discarded there
(`process.rs`). **Nothing stays dangling.** This is a deliberate
deviation from EmuTOS (A8: there the pointer dangles) and belongs in
`docs/COMPATIBILITY.md` "Sanctioned deviations" after approval. In addition:
a child (`Pexec`) starts with default values; a parent's hook does not act in the
child (EmuTOS: global table, acts).

## B6. Interaction with KEYS01 repeat and `ikbdsys`  (DECISION D4)

* Repeats do not call `kbdvec` (A5) — NewTOS already does it this way today
  (`ikbd.rs`); **leave unchanged**. A game with a hook sees
  make/break exactly once; if the default runs (chaining), repeats come into
  the BIOS queue as under EmuTOS.
* `ikbdsys` change (`ikbd.c:453, 507, 903`): during the ACIA interrupt
  EmuTOS calls `ikbdsys`. NewTOS has **no** ACIA interrupt, `ikbdsys` is
  never called (all input runs through `feed`). Proposal: slot present,
  default `ret_stub`, **is not called**; a program that replaces `ikbdsys`
  and reads ACIA registers itself (TOS 1.x games) does not work on the Pi anyway
  (no ACIA) and receives no keys — documented as *not supported*.
  The stop rule can be rebuilt faithfully to EmuTOS: at make remember `ikbdsys` from the
  table (`KeyRepeat::make`), compare in `poll_repeat`, on
  difference stop the repeat (`ikbd.c:453-471`). Small effort
  (`kernel/src/key_repeat.rs` + reading the slot). D4 decides: rebuild
  (faithful to EmuTOS, standard rule "match it") or omit (KEYS01 has already
  noted it as "not modelled", (cite stale 2026-10-26: _state.md:164, the KEYS01 "not modelled" note in docs/api/xbios/_state.md)).
* BIOS queue: runs only through the default `kbdvec` (A4.4/A5). If the
  game chains, the default fills the queue (`push_rx`) as before and starts
  the repeat; whoever does not chain gets (and delivers), per EmuTOS, nothing
  further.

## B7. Scope  (DECISION D7)

**Minimum for Doom** (recommended as build order KEYS02):
1. Table per process + `Kbdvbase` returns the EL0 VA, **including** `kbdvec`
   at `-8` bytes and all nine slots with valid default stubs (B1, B3).
2. `kbdvec` delivery with ring and chain delivery (B4), `kbdvec_default`
   syscall (D6).
3. Clean-up in `free_slot` (B5).
4. Optionally in the same go: `ikbdsys` stop (D4), because it is small.
Thereby: make **and break** of every key (also arrows, multiple keys), chaining
to the BIOS queue, safe program end.

**Full scope (later, own tickets):** `mousevec` (packet per A4, called on
mouse packets; today's path `feed_relative` stays as the default path -> here
a stub comparison would be needed as well), `joyvec` (HID gamepad -> IKBD joystick packet
`FD/FE/FF`, `docs/api` must first clarify what the HID driver delivers),
`statvec`/`clockvec` (presuppose the IKBD command side `Ikbdws`, reply packets;
outside), `midivec`/`midisys`/`vmiderr`/`vkbderr` (no hardware).
These slots **exist** in the minimum with `ret_stub`, but **a program hook hung
there is not called**. That is a documented, not
disguised gap (to be entered per rule 2 in `STATUS.md` under "Blocked" — the
lead does that, not this spec) and no hiding behind a
stub return value.

## B8. Acceptance (proposal, derived from the TASK)

* Test: hook installed, `br old` chaining; make A, make B, break A, break B
  in one poll -> four deliveries in this order.
* Without chaining: `Bconstat` stays empty, `Kbshift` unchanged.
* Process end with an installed hook -> next process/desktop reacts normally.
* `Kbdvbase()[-1] == kbdvec` slot, `Kbdvbase()` stable; slots never 0.
* Doom (Pi, TT): running + turning at the same time, a tap turns briefly.

---

# Part C — Build (KEYS02 core part, 2026-10-04)

Scope = D7 minimum. Part B applies with the following fixings; deviations
from Part B are named here explicitly.

## C1. Table, stubs, `Kbdvbase` (B1/B3)

* **Address as built, 2026-10-04: first TPA page `[0x9A0, 0xA00)`; superseded by MEM12 (Part D, "D1-new"). Current state (read at `main`):** the table starts at
  the private read-write page `KBDVECS_AREA_VA = mmu::PRIV_PAGE_VA = 0x7F000`
  (`tos-api/src/bdos/process/loader.rs:1048@KBDVECS_AREA_VA`,
  `tos-api/src/bdos/process/mmu.rs:259@PRIV_PAGE_VA`), size 0x60 (0x5A used,
  `loader.rs:1051@KBDVECS_AREA_SIZE`). Layout as B1 relative to that start
  (`tos-api/src/xbios/kbdvecs.rs:75@OFF_MOUSEXVEC`..`:85@OFF_KBDLENGTH`): `mousexvec` at `0x7F000`,
  `kbdvec` at `0x7F008`, `Kbdvbase()` = `0x7F010` (EL0 VA of `midivec`,
  `tos-api/src/xbios/kbdvecs.rs:199@kbdvbase`), `ikbdsys` at `0x7F050`,
  `drvstat`/`kbdlength` at `0x7F058`/`0x7F059` (both 0, D5). The value `0x7F010` is
  also measured (`docs/COMPATIBILITY.md`, "The IKBD vector table is per process").
* **Stubs as built: `[0xA00, 0xA40)`; current state: in the shared stub page
  `mmu::STUB_PAGE_VA = 0x7E000`**
  (`tos-api/src/bdos/process/mmu.rs:256@STUB_PAGE_VA`), region
  `KBDVEC_STUB_AREA_VA = STUB_PAGE_VA + 0x40 = 0x7E040`
  (`loader.rs:1056@KBDVEC_STUB_AREA_VA`): `ret_stub` at `0x7E040` (`ret`), `kbdvec_default` at `0x7E048`
  (`tos-api/src/xbios/kbdvecs.rs:90@RET_STUB_VA`, `:92@KBDVEC_DEFAULT_VA`; `USER_VA_BASE = 0`,
  `loader.rs:207@USER_VA_BASE`): `mov x1,x0; mov x0,#2002; mov x2,#0; mov x8,#0x154; svc #1; ret`
  (encodings checked with clang/llvm-objdump).
* Written by the loader at every load (`xbios::kbdvecs::write_defaults`, from
  `populate_and_map` next to the upcall trampolines, before the code-cache maintenance).
* **Addendum (not regulated in B1):** an EL1 caller without a process (desktop,
  boot, `RUN == null`) gets a kernel table of the same layout with the same
  default values. It is never called through it (without a process the default path always runs);
  it only answers the pointer — the same rule as `Keytbl`.

## C2. D6 — the syscall number

`kbdvec_default` calls **`Ssystem(S_NEWTOS_KBDVEC = 2002, scancode, 0)`**
(GEMDOS 0x154). The NewTOS-own range is `Ssystem` 2000+
(`docs/api/gemdos/154-ssystem.md`, 2000/2001 taken); 2002 is the next free
number. Contract: `arg1` = byte (0..0xFF, else `EINVAL`), `arg2` = 0 (else
`EINVAL`), result `E_OK`, `ERROR` without a keyboard decoder (host without decoder).
The number appears only in the kernel-written stub; no program binary
contains it, a renumbering would break nothing.

## C3. Delivery (B4, D3/D3a)

* `ikbd::feed` → `kbdvecs::route_key(byte)` for every byte `< 0xF6`:
  no running process **or** `kbdvec == KBDVEC_DEFAULT_VA` (as built `0xA08`, now `0x7E048`, C1) → `decode_key` directly
  (fast path, unchanged). Otherwise byte into the ring of the process slot
  (64, with a full ring the **newest** is discarded).
* Delivery (`exceptions.rs::deliver_pending_kbdvec`) at every return to
  EL0 (`complete_call_tail`, resume after scheduler switch in `apply_switch`,
  resume `Psigreturn`): one byte per upcall, `x0` = scancode, `x1` = 0,
  target = the **current** `kbdvec` value, mask bit `UPCALL_MASK_KBDVEC = 1 << 3`.
  As long as the bit is set, no further byte is delivered (depth 1);
  the next follows at the `Psigreturn` of the hook (chain). An already
  posted other upcall takes precedence. If the default is in the slot again at delivery,
  the byte is decoded directly.
* **O1 — settled, was not given, is built.** A blocking AES wait
  (`evnt_*` → `sched::sleep`) and the blocking single-key reads
  (`Cconin`, `Cnecin`, `Crawcin`, `Bconin(2)`) never returned to EL0
  as long as no key lay in the BIOS queue — but with a chaining
  hook only the hook itself puts one there: deadlock. Now they **give way**:
  `sleep` returns, with waiting ring bytes (before blocking and after its
  own pump), `Yield(switch_restart_for_upcall())`, the reads
  `Host::console_read_key_restartable() == None` + `kbdvecs::request_restart()`.
  The trap handler aborts the call like an S2 yield (`elr` back to the `svc`,
  nothing committed), delivers the hook, and after its
  `Psigreturn` the `svc` runs again (the `evnt_multi` time limit is kept via
  `syscall_deadline`). Measured in QEMU: `evnt_multi(MU_KEYBD)` **without**
  timer and `Bconin(2)` deliver the key chained by the hook; two breaks in the
  same poll both reach the hook, in order, the second through the
  `Psigreturn` chain (`measurements/measurement-2026-10-04-keys02/`).
  **[XBIOS18] 2026-10-29:** likewise the wait of `Bconin(1)` (also `Cauxin`
  and a one-byte `Fread` on AUX) on qemu-virt, where AUX is a driver
  (XBIOS15): `AuxSerial::bconin_restartable` returns `None` with waiting ring bytes
  (`kernel/src/console.rs:231@aux_read_blocking`), the trap restarts after
  the hook. Before, the hook received its bytes only with the next
  serial byte; EmuTOS calls it from the IKBD interrupt while `bconin1`
  waits (`bios/aciavecs.S:407-414`, `bios/serport.c:202-210`). Measured with
  `tosdump/probes/axprobe.c` against tt and falcontc
  (`measurements/measurement-2026-10-29-xbios18/RUNINFO.TXT`). On a real Pi 4
  `Bconin(1)` did not wait at the time (no cable). **[BIOS09] 2026-10-30:** there
  now likewise — the same driver on both boards, the same loop
  (`tos-api/src/bios/chardev.rs:605@aux_read`); while waiting
  no other process runs, only the hook of the waiting one.
  Pi step with `tosdump/probes/awprobe.c` (MEM12 check sequence).
* **Limits (documented, not built):** `Cconrs`, console `Fread` and
  `v_string` (request mode) still read without giving way — they already have
  an effect before the end (echo, buffer); a program that waits in them with a hook installed
  and chaining hangs. Modal AES loops that call
  `pump_input` themselves (`form_do`, menu/drag loops) deliver only on
  their return (ring 64). A hook runs as an ordinary AAPCS64 function,
  but the upcall does not save the **FP/SIMD registers** of the interrupted code
  (applies to all upcalls); a hook must not use them.

## C4. Repeat and `ikbdsys` (D4)

`KeyRepeat::make` remembers the `ikbdsys` value of the running process
(`kbdvecs::current_ikbdsys`, without a process `RET_STUB_VA`, as built `0xA00`, now `0x7E040`, C1), `poll` stops at a
50 Hz step with a running repeat and a different value (before the
`conterm` test, `ikbd.c:453-472`); on re-mapping the value is updated
(`:507`). `ikbdsys` is never called. Repeats do not call `kbdvec`
(unchanged, `ikbd.c:524`) — measured: the hook saw 6 codes, the AES 7+7
words (make + repeats).

## C5. Process end (D8)

Table and stubs live in the TPA and disappear with it; `free_slot` empties
the ring (`kbdvecs::release_slot`). Measured: a program that does **not**
reset its hook ends, and the desktop reacts normally to Alt-C.
Deviation from EmuTOS in `docs/COMPATIBILITY.md` (Sanctioned deviations).

## C6. Slots not called (D7 gap)

`midivec`, `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `clockvec`, `joyvec`,
`midisys`, `ikbdsys` and `mousexvec` exist with `ret_stub`; a program
hook hung there is **not called** (the mouse continues to run
kernel-internally through `vdi::feed_relative`, `mousexvec` through
`bios::vectors::mousexvec`). Entry under `docs/STATUS.md` "Blocked" (lead).
**[BIOS15] ADR-031:** stays not called (XBIOS-CHARDEV-003 ACCEPTED), but
not silently: the first input event that EmuTOS would call the slot with
(`ikbdsys`/`midisys` every IKBD byte, `mousevec` mouse packet, `statvec`,
`clockvec`, `joyvec` their packet) writes one warning per hook and process
(`tos-api/src/xbios/kbdvecs.rs:447@check_events`). `midivec`, `vkbderr`,
`vmiderr`: their event (MIDI byte, overrun) never occurs, no warning.

## C7. Evidence

Host tests `tos-api/tests/xbios_kbdvecs.rs` (10), `kernel/src/key_repeat.rs`
(+3 `ikbdsys`), QEMU raspi4b run with `sdk/examples/kbdvec.c`
(`measurements/measurement-2026-10-04-keys02/RUNINFO.TXT`).

---

# Corrections to the earlier version (22-kbdvbase.md Phase 00) and `_state.md §1`

1. **`kbdvec` was missing entirely** (slot at `-4`; `aciavecs.S:129`); likewise
   `mousexvec` (`-8`) and the neighbouring bytes `ikbdstate`/`kbdlength`.
2. `statvec` = packet `0xF6` (was right), `mousevec` gets **also `0xF7`**
   (absolute mouse, `aciavecs.S:488-489`), not only `0xF8..0xFB`; `joyvec` gets
   `0xFD..0xFF` (was right).
3. "`ikbdstate`/`kbdlength` ... not part of the contract": **wrong for
   `ikbdstate`**. tos.hyp lists `drvstat` as a documented field behind
   `ikbdsys`; programs are to check it before rehooking. `kbdlength`/`kbdbuf`
   stay internal (D5).
4. The earlier version named as the NewTOS default "36-byte layout with native
   pointers, not filled" and "`Initmous` writes `mousevec`": the state
   (all 0) is useless for chaining (`br 0`). New proposal B3.
5. The Compendium cross-references "not present in references/" are outdated
   (tos.hyp retrieved, see below).
6. `_state.md §1` (table `kbdvecs`) has the same gaps (no `kbdvec`, packet
   `0xF7`, `drvstat`); raise it at approval.

---

# Cross-references

* Atari Compendium (tos.hyp, retrieved 2026-10-04 by WebFetch; the page was
  **summarised** by the retrieval tool, not checked verbatim):
  * https://freemint.github.io/tos.hyp/en/xbios_structures.html (KBDVBASE: field
    list with `drvstat`, "address of the packet in A0 and on the stack", RTS,
    <= 1 ms)
  * https://freemint.github.io/tos.hyp/en/xbios_interface.html (Kbdvbase:
    warning `drvstat`/disable interrupts before a change)
  * `kbdvec` is **not** mentioned on either page.
* EmuTOS: `bios/aciavecs.S`, `bios/ikbd.c`, `bios/xbios.c`, `bios/mouse.c`,
  `bios/amiga2.S`, `vdi/vdi_mouse.c`, `vdi/vdi_asm.S`, `bdos/proc.c`,
  `include/biosdefs.h`, `include/config.h`.
* FreeMiNT: `sys/keyboard.c`, `sys/arch/init_intr.c`, `sys/arch/intr.S`,
  `sys/arch/acia.S`, `sys/dev-mouse.c`, `sys/mint/struct_kbdvbase.h`,
  `sys/usb/src.km/udd/hid/{mouse,mouse-boot,tablet}/*.c`.
* NewTOS code (read): see B0.

# Open points (not settled, deliberately not guessed)

* **O1.** Whether the AES path after `evnt_multi` has an upcall delivery on the return path (B4) —
  check at build. **Settled 2026-10-04 (C3):** yes on the return path, but
  a *blocking* wait did not return; now it gives way.
* **O2.** The `IOREC` form for `x1` (D2a) depends on the decision about `Iorec`.
  **Settled 2026-10-29 (XBIOS01, E6):** `x1` = `Iorec(1)`, LP64 `_IOREC`.
* **O3.** TT TOS 3.06 has `kbdvec`: only derived (A7). A measurement on the
  Hatari TT (`hatari-produces-tt-references`) could confirm it — not carried out here.
  **Settled 2026-10-04 (KEYS02):** measured, yes — ROM and Hatari TT debugger,
  evidence in A7.
* **O4.** Whether `ikbdstate` is ever observable in the middle of a packet at all: the
  HID translator feeds atomically per report (inferred, not checked) —
  hence the proposal `drvstat == 0` constant.

# Decisions for the owner

| # | Question | Recommendation |
|---|---|---|
| D1 | Table per process (first in TPA page 0 at `0x9A0`; since MEM12 in the private page `0x7F000`, Part D), `Kbdvbase` returns the EL0 VA; a parent's hook does not act in the child; TSR hooks for foreign programs not possible | yes |
| D2 | Calling convention: AAPCS64, `x0` = scancode/byte or packet pointer, `x1` = replaces a0; ILP32 not supported | yes |
| D2a | `x1` for `kbdvec`: 0 until `Iorec(1)` exists | yes |
| D3 | Delivery through kernel ring per process + upcall chain (not `post_upcall` alone); fast path with an unchanged default hook | yes |
| D3a | Ring size / overflow rule (proposal 64, discard newest) | 64 |
| D4 | Rebuild the `ikbdsys`-change stop of the repeat (faithful to EmuTOS) or never call `ikbdsys` | rebuild, never call `ikbdsys` |
| D5 | Offer the `drvstat` byte behind `ikbdsys` as constant 0 | yes |
| D6 | New internal syscall for `kbdvec_default` (number assigned by the lead) | yes |
| D7 | Scope: minimum (kbdvec + safe defaults + clean-up) now; mouse/joystick/status/clock/MIDI as follow-up tickets, in the minimum only `ret_stub`, documented gap | minimum |
| D8 | Enter the deviation "hook disappears with the process" in COMPATIBILITY.md "Sanctioned deviations" | yes |

# Part D — MEM12: location of the table and recipient of an event (APPROVED 2026-10-27)

Occasion: TASK/MEM12, owner decision 2026-10-27 ("E2: yes; also specify
whose Kbdvbase table receives an input event (owner/foreground)"). Parts B/C
remain valid, except D1 (location) and B4 (recipient), which are replaced here.

## D1-new. Location

The table (`mousexvec` … `kbdlength`, layout B1 unchanged) moves out of
page 0 into a **second private page per process**, EL0 read + write,
not executable, at a fixed VA below 2 MiB that is never mapped shared
(page 0 is, from MEM12, writable for EL0 only in the supervisor state,
`docs/api/_super-state.md`). `Kbdvbase()` returns the EL0 VA of the `midivec` slot
there. The default stubs (`just_rts`, `kbdvec_default`) move into the shared,
EL0 execute-only stub page; their addresses stay the same for all processes.
Observably only the numeric value of the pointer changes. Values as built:
page `0x7F000`, `Kbdvbase()` = `0x7F010`, stubs in `0x7E000` (C1).

## D9. Whose table receives an input event

EmuTOS has **one** table; `ikbdraw` jumps through the `kbdvec` that is entered right now,
whichever program is running (`bios/aciavecs.S:405-414`).
On a single-TOS this is the hook of the foreground program — or
of an accessory, if one has set it.

Until MEM12 (KEYS02 C3) NewTOS took the table of the process that
has the CPU at the poll (`route_key` with `current()`, state `a90bcfb0`; since D9
`tos-api/src/xbios/kbdvecs.rs:352@route_key`). If the
pump polls in the context of an accessory, DOOM's hook does not see the byte.

**Proposal:** the recipient is the **innermost started process** — EmuTOS'
`run` without accessories, in NewTOS `process::emutos_run()`
(`tos-api/src/bdos/process.rs:1023@emutos_run`): the foreground program or
its running `Pexec` child. Regardless of which process has the CPU at the poll:

* If its `kbdvec` holds the default (or no started process lives),
  `kbd_int` runs directly (fast path as before).
* Otherwise the byte goes into the ring of **this** process and is delivered at its
  next return to EL0 (C3). If it sleeps in a wait that gives way
  on waiting ring bytes (C3 "O1"), it is woken for that.
* The table of an **accessory** is never consulted (accessories are not
  started, `emutos_run` never names them). Deviation from EmuTOS: there
  an accessory hook acted globally. No ported accessory sets one.

**D9 approved (owner 2026-10-27):** recipient = `emutos_run()`.
If an accessory ever sets a `kbdvec` hook (its slot no longer holds the
default), the kernel writes one warning per accessory into the log —
the deviation is never to strike silently. [BIOS15] Since ADR-031 at the
first key byte that the hook would get on EmuTOS (the common
mechanism, `check_events`), no longer at every syscall return. The hook runs in the
supervisor state (`docs/api/_super-state.md` S11, O1 = yes).

---

## Open points

- Corrected (SPEC07): the NewTOS table address `0x9A0` (TPA page 0) to `0x7F000` / `Kbdvbase()` `0x7F010` (header, B1 marker, C1, C3, C4, decision table D1, Part D), and the stub addresses `0xA00`/`0xA08` to `0x7E040`/`0x7E048`; cites in C1. Part B0 and the B1/B3 proposal text are kept as written on 2026-10-04 and marked as superseded.
- Translated (SPEC07): the whole file was German; this is the English version, content unchanged apart from the corrections above. Line numbers of the German version have shifted by a few lines; the dated analyses citing it by line (`docs/analysis/`) are snapshots and not touched.
- Not re-verified in this edit: every other statement of Parts A–C (EmuTOS `file:line` cites kept as written; doc-check resolves the paths and lines, not the content). The NewTOS cites inside B0 describe the state of 2026-10-04 and are not re-read.
- Inferred: that `USER_VA_BASE = 0` makes the stub VAs equal to `STUB_PAGE_VA + 0x40`/`+0x48` (`loader.rs:207@USER_VA_BASE`, `kbdvecs.rs:90@RET_STUB_VA`); not measured.

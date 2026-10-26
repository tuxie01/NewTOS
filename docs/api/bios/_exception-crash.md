# Unhandled CPU exceptions in a program — crash report and `Pterm(-1)`

> ARCH03 stage 0c (2026-10-05). Source: EmuTOS @ `1a7484ab`
> (`references/emutos`), cross-checked against FreeMiNT @ `f906416`
> (`references/freemint`). Not a trap function: this is the behaviour of the
> **default exception vectors** — what the OS does when a program takes a
> CPU exception nobody handles. Plan: `docs/analysis/arch03-implementation-plan.md`
> Frage 10/11, Stufe 0c, E11.

## EmuTOS source

**Primary location:** `bios/vectors.S:225` `any_vec` → `bios/kprint.c:302`
`dopanic()` → `bios/panicasm.S:96` `kill_program`.

**Installed handler / vector:** `bios/vectors.S:103` `_init_exc_vec` (called
from `bios/bios.c:143`) points the 62 CPU vectors 2–63 at
`deflt_vec_table` (`bios/vectors.S:151`), one `bsr.s any_vec` per vector; the
`bsr` return address lets `any_vec` recompute the vector number.

**Helpers called:**
- `bios/vectors.S:225-279` — `any_vec` / `dump_stack`: `move.w #0x2700,sr`,
  saves `D0-D7/A0-A6` to `proc_dregs`/`proc_aregs`, `USP` to `proc_usp`, the
  vector number to `proc_enum`, 16 stack words to `proc_stk`, the magic
  `0x12345678` to `proc_lives`, switches to `panic_stack`, `jmp _dopanic`.
- `bios/kprint.c:302-594` — `dopanic()`: prints the report, waits for a key,
  calls `kill_program`.
- `bios/kprint.c:265-269` — `vkcprintf`: every report line goes to the debug
  log (`kprintf`) **and** the screen (`cprintf` → `bconout2`, `kprint.c:53-60`
  adds a CR before each LF).
- `bios/panicasm.S:96-99` — `kill_program`: `move.w #-1,-(sp)`,
  `move.w #0x4C,-(sp)`, `trap #1` — i.e. `Pterm(-1)`; should it return,
  `warm_reset` (`panicasm.S:105`).
- `bdos/proc.c:601-618` — `xterm(UWORD rc)`: runs `etv_term`
  (`Setexc(0x102,-1)` + `protect_v`), `run = run->p_parent`, `ixterm`
  (resources), `run->p_dreg[0] = rc`, `termuser()`.

**System variables / globals written:** the post-mortem area at `0x380`
(`proc_lives` 0x380, `proc_dregs` 0x384, `proc_aregs` 0x3A4, `proc_enum`
0x3C4, `proc_usp` 0x3C8, `proc_stk` 0x3CC; see `_boot-init.md` §sysvars),
`savptr` reset to `trap_save_area` (`kprint.c:587`).

**Build configuration affecting this function:** `DISPLAY_INSTRUCTION_AT_PC`
and `DISPLAY_STACK` (`kprint.c:37-38`) are `0` — no instruction or stack words
are printed. CPU-specific frame decoding (`mcpu`, `kprint.c:358-490`); the
Falcon030 profile prints the 68030 bus/address-error lines.

---

## Source extraction summary

1. **Which exceptions.** Every CPU exception whose vector still holds the
   default (bus error 2, address error 3, illegal instruction 4, zero divide
   5, CHK 6, TRAPV 7, privilege violation 8, trace 9, Line-A 10, Line-F 11,
   and the rest up to 63). `any_vec` does **not** look at the S bit: an
   exception in supervisor mode — e.g. a bus error while GEMDOS dereferences a
   caller's pointer — takes exactly the same path.
2. **Report** (`kprint.c:312-573`), in order:
   - `cprintf("\033f\033v\n\n")` — hide cursor, line wrap on, two newlines;
   - `Panic: <exc_messages[proc_enum]>` (`kprint.c:283-300`: "Bus Error",
     "Address Error", "Illegal Instruction", "Zero Divide", "CHK
     Instruction", "TRAPV Instruction", "Privilege Violation", "Trace",
     "Line A Emulator", "Line F Emulator"), or
     `Panic: Exception number %d` outside that table (`kprint.c:500`);
   - the CPU frame: 68030 bus/address error `fw= ir= ssr=` and
     `addr= sr= pc=` (`kprint.c:432-435`), other exceptions `sr= pc=`;
   - a blank line, then `D0-3:`, `D4-7:`, `A0-3:`, `A4-7:` (four `%08lx`
     each) and ` USP:`, then a blank line (`kprint.c:527-537`);
   - if a process runs (`run`): `basepage=`, `text= data= bss=`, and
     `Crash at text+%08lx` when the PC lies inside the text segment
     (`kprint.c:565-573`).
3. **Wait** (`kprint.c:575-585`): interrupts on, eat pending keys
   (`while (bconstat2()) bconin2();`), print
   `"\n*** Press any key to continue ***"` (screen only, translated), block
   in `bconin2()`, print `"\n"`.
4. **Terminate** (`kprint.c:587-594`): "we are still running with the PD of
   the failing program, so calling Pterm() now should clean up and return
   control to the caller" — `kill_program` → `Pterm(-1)`. The full `xterm`
   path runs: `etv_term` hook, resource release, the parent resumes from its
   `Pexec` with the exit code. `rc` is a `UWORD` (`xterm(UWORD rc)`), so the
   parent's `Pexec` returns `0x0000FFFF` (NewTOS D-005).
5. **Bad pointer in a system call.** TOS has no `EFAULT`. EmuTOS performs no
   pointer validation; a GEMDOS/BIOS function that dereferences an
   inaccessible caller pointer takes a bus (or address) error in supervisor
   mode, and points 2–4 apply with `run` = the **caller**: the calling
   program is terminated with `Pterm(-1)`, the report shows the supervisor PC
   (no "Crash at text+" line). The call returns nothing to the program.
   FreeMiNT agrees in substance: a kernel bus error raises `SIGBUS` on
   `curproc` (`sys/arch/sig_mach.c:493-502`), whose default action kills it
   with the "MEMORY VIOLATION" alert (`sys/arch/mprot.x:89-150`); its `EFAULT`
   (`sys/mint/errno.h:328`, value −40 = TOS `EIMBA`) is only returned by
   explicit checks (`Mfree`/`Mshrink` of a non-block, `sys/dosmem.c`), never
   for a faulting access.

---

## NewTOS contract (ARCH03 0c)

Observable behaviour that must match:

| Item | Contract |
|---|---|
| Who is terminated | the program that took the exception — for a fault inside a system call, the **calling** program |
| How | `Pterm(-1)` through the ordinary terminate path: `etv_term` hook (if installed), resources freed, parent resumes; the parent's `Pexec` returns `0x0000FFFF` |
| Report | on the console screen and in the debug log, before termination; layout as in §2, adapted below |
| Wait | pending keys are discarded, prompt `*** Press any key to continue ***` (`tos_api::i18n::Sys::PressAnyKey`), then one key |
| Machine | keeps running; the desktop/parent continues and repaints |

### AArch64 → m68k exception mapping (`tos_api::bios::crash::classify`)

| `ESR_EL1.EC` | Condition | Reported as | m68k vector |
|---|---|---|---|
| `0x20/0x21` Instruction Abort, `0x24/0x25` Data Abort | `DFSC/IFSC` ≠ `0x21` (translation, permission, access flag, address size, external) | `Bus Error` | 2 |
| same | `DFSC/IFSC` = `0x21` (alignment fault) | `Address Error` | 3 |
| `0x22` PC alignment, `0x26` SP alignment | — | `Address Error` | 3 |
| `0x00` unknown reason (undefined instruction), `0x0E` illegal execution state | — | `Illegal Instruction` | 4 |
| `0x18` trapped `MSR`/`MRS`/system instruction (cache maintenance, `CTR_EL0`, `DC ZVA`, …), `0x01` trapped `WFI`/`WFE` | user mode may not execute it (`SCTLR_EL1` gates) | `Privilege Violation` | 8 |
| anything else (e.g. `0x3C` `BRK`) | no 68k counterpart | `Exception class 0x<ec>` (the analogue of "Exception number %d") | — |

Sources: ARM ARM DDI 0487 M.d D24-8643 (EC), D24-8672/8673 (IFSC),
D24-8683…8685 (DFSC), D24-8681 / D1-7219 (`FAR` validity, `FnV`); measured on
QEMU in `docs/analysis/arch03-stage0b-spike.md` §5.

### Report layout (adapted, `tos_api::bios::crash::write_report`)

```text
<blank>
<blank>
Panic: Bus Error
esr=92000046
addr=00000000dead0000 sr=00000000 pc=0000000000001234
<blank>
X0-3:   <16 hex> <16 hex> <16 hex> <16 hex>
…       (X4-7 … X24-27)
X28-30: <16 hex> <16 hex> <16 hex>
 USP:   <16 hex>
<blank>
basepage=00000000
text=00001000 data=00002000 bss=00002100
Crash at text+00000234
<blank>
*** Press any key to continue ***
```

Adaptations, each because the m68k item does not exist on AArch64: the
`esr=` line stands where the 68030 prints its frame words (`fw= ir= ssr=`);
`addr=` is printed when `FAR_EL1` is valid for the syndrome; `sr=` is
`SPSR_EL1`; the 31 general registers `X0`–`X30` replace `D0-D7/A0-A6`
(64-bit, four per line, ≤ 80 columns); ` USP:` is `SP_EL0`. For a fault in a
system call the registers, `pc`, `sr` are the kernel's at the faulting
`ldtr`/`sttr` (as EmuTOS shows the supervisor state), ` USP:` the caller's.

### Exceptions taken by the kernel itself

A synchronous exception at EL1 **outside** the fault-safe user access
(`tos_api::uaccess`, below) remains fatal (halt with the `[EXC]` report),
as before 0c. This is a deliberate difference from EmuTOS, where a ROM bug
during a trap also ends in `dopanic` + `Pterm`: here it indicates corrupted
kernel state (plan Frage 11, invariant of Stufe 0c).

### Fault-safe user access (`tos_api::uaccess`)

Kernel reads/writes of caller memory go through leaf routines built on
`ldtr*`/`sttr*` (EL0 permission check, ARM ARM C6.2.245) with a fixup table
keyed by the faulting PC. A same-EL data abort (`EC 0x25`) at a listed
instruction is recorded and resumed at the fixup; the call stops, and the
syscall tail terminates the **EL0** caller per §5 above. An **EL1** caller
reaching such a fault (a kernel path passing a kernel pointer to a
user-pointer API) halts, as before.

Converted in 0c (pilot): `Cconws` (`tos-api/src/gemdos/console.rs`), the
`Pexec` path / command tail / environment readers
(`kernel/src/arch/aarch64/exceptions.rs` `handle_pexec`).

**ARCH03 stage 3a** converted the rest of the inventory
(`docs/analysis/arch03-stage3-access-inventory.md`): every trap carries the
caller's space (`TrapArgs::space`, from `SPSR_EL1.M`; an in-kernel caller —
the desktop's `svc`, the AES fiber — passes kernel addresses and is not
checked), and a syscall argument of an EL0 program is either read/written
with its rights through `uaccess` or checked with its rights and then reached
through its kernel alias (`Space::kptr`: GEMDOS paths, file/DTA/console
buffers, `Dgetpath`/`Dfree`/`Fdatime`, `Cconrs`, `Ssystem`, `Rwabs`, `Getmpb`,
`Setpalette`, `Protobt`, `Keytbl`, the AES and VDI parameter blocks and the
`addr_in` buffers and strings, `AESGLOBAL`, MFDB bitmaps, `objc_edit`'s and
`menu_text`'s target text). A bad pointer in any of them ends the calling
program exactly as above; the probe `sdk/examples/uacc3a.c` / `uakid.c`
drives one case per site class
(`measurements/measurement-2026-10-05-arch03-3a`). Two deliberate cases do
**not** crash: a pointer the AES follows *outside* its owner's trap (another
process's tree, window title, `WF_NEWDESK` background) reads as nothing — no
fault can be charged to a process that is not running — and a page-0 cell
the kernel reaches through the process's own page-0 alias (`Setexc` sysvar
peek/poke, `conterm`) cannot fault at all.

### Memory protection (ARCH03 stage 3b)

With per-process rights on (the default; boot argument `memprot=off`
switches them off), an access a program may not make — another process's
`PROT_P`/`PROT_S` memory, a write to another process's `PROT_PR` memory,
freed pool memory — is an ordinary bus error of the program (`EC 0x24/0x20`,
or `EC 0x25` when the kernel touches the memory with the caller's rights in
a system call) and ends it exactly as above. The report gets **one extra
line** right after the `addr=` line, FreeMiNT's classification of the
address (`report_buserr`, `sys/arch/mprot.x:71-150`: "MEMORY VIOLATION",
type, read/write) plus the MMU's reason:

```text
Memory violation: <type> memory, <read|write|execute> (<translation|permission|access flag> fault, level <0-3>)
```

* `<type>` comes from the protection bookkeeping, not from the MMU — a
  translation fault cannot tell "protected" from "unmapped" (plan §3.3, V0a
  §6): `private` (`PROT_P`), `super` (`PROT_S`), `readable` (`PROT_PR`),
  `free` (free pool memory or `PROT_I`, FreeMiNT `berr_msg[PROT_I]`),
  `hardware` (outside the pools, page 0, or a `PROT_G` block — no
  protection fault possible there; FreeMiNT turns `global` into `hardware`,
  `mprot.x:103-107`).
* `<fault>` is `ESR_EL1.DFSC/IFSC` (`0b0001LL` translation, `0b0010LL`
  access flag, `0b0011LL` permission, LL = level; ARM ARM D24-8672/8673,
  D24-8683…8685). Non-owner `PROT_P/S/I` pages are **invalid**, so they show
  as translation faults; a write to `PROT_PR` memory of another process is
  the permission fault.
* `<access>`: `ESR_EL1.WnR` (D24-8682) for data, `execute` for an
  instruction abort.

The line is only printed with protection on and only for translation,
access-flag and permission faults (FreeMiNT reports nothing with
`no_mem_prot`, `mprot.x:96-97`). With `memprot=off` the report is the
stage-0c/3a report unchanged. Example (a child writing its parent's private
block under `memprot=strict`):

```text
Panic: Bus Error
esr=92000047
addr=0000000001c36000 sr=00000000 pc=…
Memory violation: private memory, write (translation fault, level 3)
```

Source: `tos_api::bios::crash::mem_violation`, `tos_api::bdos::process::
memprot::region_kind`; probe `sdk/examples/mprot3b.c` / `mpkid.c`
(`measurements/measurement-2026-10-05-arch03-3b`).

### Not done / open

- **Post-mortem area `0x380`** (`proc_lives` …) is not written: the m68k
  layout cannot hold 31 64-bit registers, and NewTOS' page 0 is per process.
  Open — no known consumer.
- **Escapes `ESC f`/`ESC v`** before the report are not emitted: the screen
  console does not parse escape sequences (`kernel/src/fbcon.rs`).
- **FreeMiNT "MEMORY VIOLATION" alert** is not shown (E11: EmuTOS report,
  no FreeMiNT alert); since stage 3b the report carries FreeMiNT's
  classification as one text line (section above).
- A program that crashes again inside its own `etv_term` handler re-enters
  `Pterm(-1)` → `etv_term`; EmuTOS would loop, NewTOS stops at the upcall
  nesting limit with `Pterm(-998)` (existing `deliver_upcall` rule).
- **SError** (asynchronous) from EL0 still halts — not a synchronous
  exception, outside 0c.
- Writing to user address 0 does **not** fault in NewTOS today: page 0 (the
  basepage + sysvar page) is mapped `UserRwExec` for the program
  (`loader.rs`), unlike the supervisor-only low memory of an ST.

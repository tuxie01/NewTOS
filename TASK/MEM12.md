# MEM12 — EL0 may write page 0

Status: DONE (2026-11-11) | Priority: P1 | Commit: 60d3c4fdf | Receipt: MEM12-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/MEM12-STATUS-DONE.MD exists, commit 60d3c4fdf, TASK/MEM12-STATUS-DONE.MD, commit 60d3c4fdf (feat(sdk): Super/Supexec are kernel traps again (MEM12)), fix/feat commits naming the id: 60d3c4fdf, 345307915, 0edf53d06 (n=4), measurements/measurement-2026-10-27-mem12/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-30 ([Quittung](MEM12-STATUS-DONE.MD); Pi-Lauf des Anwenders 2026-10-29, Kern `58e35ac8`) · Prioritaet: **P1** · Typ: Kernfehler (Schutzgrenze fehlt) — Anwenderentscheidung 2026-10-26

## Facts

When this ticket was opened (`0bc2d4fb1`), page 0 `[0, 0x1000)` was mapped `UserRwExec` in
every process (`mmu.rs` line 643 at that commit, `t.l3.0[0] = page0_pa | ...`), in every
memory-protection mode (`reset.rs` line 26 at that commit). An EL0 program could therefore
read, write and execute there; today page 0 is `UserRoNx` (`tos-api/src/bdos/process/mmu.rs:826@UserRoNx`).

On the Atari, an access to the low addresses in user mode raises a bus error;
the system variables are reachable only in supervisor mode.
Anwenderentscheidung 2026-10-26: **EL0 darf Seite 0 nie beschreiben.** (translated from German: **EL0 may never write page 0.**) Whether EL0
may read it depends on whether native programs read system variables there;
this ticket settles that by search and measurement. If none does, page 0 is
hidden from EL0 entirely.

What lies in page 0 today (`docs/COMPATIBILITY.md`, section „One shared user
address space, page 0 per process“): system variables, OSHEADER with `os_run`,
cookie-jar mirror, trampoline, `Keytbl` area, KEYS02 table, the
vector table at `0x9A0`, the reset stub (`RESET_STUB_VA`,
`tos-api/src/bios/reset.rs`) and the `S_NEWTOS_KBDVEC` stub. Page 0 is
per process.

Known consequence: `__aline` is NULL per ADR-030; `__aline->field` with a positive
offset reads page 0 and writes there instead of failing
(`docs/COMPATIBILITY.md`, section „No Line-A“).

## Questions the ticket answers with evidence

1. **Read:** Which native programs (`programs/`, `sdk/apps/`, mintlib,
   gemlib, cflib, AUTO programs, tosdump) read addresses in `[0, 0x1000)` —
   directly, via `_sysbase`/`Setexc`/`Supexec` callbacks or via mintlib
   macros? `file:line` for each hit; where the search does not suffice, measure
   (hide page 0 as a test, boot to the desktop and start the card's applications,
   log data aborts).
2. **Execute:** Which stubs in page 0 does EL0 jump to (reset stub,
   KBDVEC stub, trampoline)? Do they need execute permission in page 0, or
   can they move to a page of their own that is execute-only?
3. **Write:** Does any native program write to page 0 today
   (e.g. `conterm` after `Super()`)? What does EmuTOS do then in supervisor mode,
   and what should NewTOS do instead (error, process end)? Does the kernel
   itself read back values from the process's page 0 that a program
   might have changed?

## Finding for 1–3 (2026-10-27, HEAD `a90bcfb0`)

Method: source search with `/usr/bin/grep -rnI` over `sdk/apps`, `sdk/overlays`,
`sdk/examples`, `sdk/libc` (mintlib + `newtos-arch`), gemlib, cflib, WinDom,
LDG, zView, libshare, `programs/`, `tosdump/`, `kernel/`, `tos-api/` (patterns:
addresses `0x0..0x67F` behind a pointer cast, `Super`/`Supexec`/`Setexc`,
`_sysbase`, `p_cookies`, `_hz_200`, `conterm`, `get_sysvar`/`set_sysvar_*`,
`Kbdvbase`, `Keytbl`, `page0_kva`); every hit read. Plus a measurement
with page 0 as a test `UserRoExec` (EL0 read + execute, not write):
`measurements/measurement-2026-10-27-mem12/RUNINFO.TXT`.

Preliminary remark: `Super()`/`Supexec()` raise nothing on NewTOS; `Super` is
a bracket without effect, `Supexec` an ordinary call at EL0
(`sdk/libc/overlay/include/mint/osbind.h` (as of `a90bcfb0`; traps again since MEM12)). Every "supervisor access"
of a program to the system variables is therefore an EL0 access to page 0.

### 1. Read — there are readers; hiding does not work

| Reader | Location | What |
|---|---|---|
| mintlib `clock()` (every program with `clock`/timing) | `sdk/libc/mintlib/time/clock.c:15,21@get_sysvar` → `sdk/libc/newtos-arch/getsysvar.c:36@get_sysvar` | `_hz_200` `0x4BA`, direct load |
| DOOM | `sdk/apps/doom/doomgeneric_gem.c:675@get_sysvar`, `:1858-1866@kbd_probe` | `_hz_200`; `_sysbase` `0x4F2` → OSHEADER `os_version`/`os_dummy` |
| PRESENT | `sdk/apps/present/present.c:934@get_sysvar` | `_hz_200` |
| TeraDesk | `sdk/apps/teradesk/library/utility/gettos.c:40-42@os_version` | `_sysbase` → `os_version` |
| WinDom (zView) | `sdk/windom/overlay/src/vq_tos.c:83@syshead`, `sdk/windom/overlay/src/cookieptr.c:48@__cookieptr` | `_sysbase`, `_p_cookies` `0x5A0` |
| zView | `sdk/zview/z-tools-04805b7/zview/chrono.c:25,45@Super` | `_hz_200` |
| cflib (QED) | `sdk/cflib/cflib-21/cookie.c:90-92@get_cookie_adr` | `_p_cookies` |
| `Keytbl` area (`0x768`) | TeraDesk `sdk/apps/teradesk/library/xdialog/xdevent.c:76@Keytbl`, cflib `sdk/cflib/cflib-21/nkccinit.c:99-102@Keytbl`, `sdk/cflib/cflib-21/menucre.c:38@Keytbl`, `sdk/cflib/cflib-21/fdfindsc.c:48@Keytbl`, WinDom `sdk/windom/windom-1.21.3/src/keybd2ascii.c:48@Keytbl`, HypView `sdk/overlays/hypview/dl_init.c:245@Keytbl`, DOOM `:1930`, NEWDESK `programs/desktop/src/main.rs:1350@Keytbl` | read the tables that `Keytbl` returns |
| Test programs | `sdk/examples/cmdline.c:40@run_cell`, `fppexec.c`, `ptermch.c`, `uakid.c`, `mpkid.c`, `e10usr.c`, `e10kid.c`, `crchild.c`; `sdk/apps/apitest/apitest.c:896-937@sysbase` | `_sysbase` → `os_run`; APITEST T03–T06, SX04 |

Measured: with page 0 readable only, AUTO (`MXPROBE.NCP`), both
accessories, NEWDESK, QED, PRESENT, zView, DUMPVDI, VDIDRAW, ARGVTEST start
(without input). **Result 1: page 0 must stay readable for EL0**
(at least `[0x400, 0x670)` and the `Keytbl` area).

### 2. Execute — five stubs, all process-independent

| Stub | VA | Who jumps there |
|---|---|---|
| Upcall trampoline | `0x700` | the kernel sets `lr` to it (`kernel/src/arch/aarch64/exceptions.rs:1862@TRAMPOLINE_VA`) |
| AES `G_USERDEF` stub | `0x714` | the kernel as upcall handler (`exceptions.rs:923@USERDEF_TRAMPOLINE_VA`); measured in TeraDesk (`pc=0x724`) |
| `just_rts` / `kbdvec_default` | `0xA00` / `0xA08` | values of the `Kbdvbase` table; DOOM chains to the old `kbdvec` (`sdk/apps/doom/doomgeneric_gem_kbdvec.c:57@kbdvec_old`) |
| Reset stub | `0xA40` | OSHEADER `reseth`/`os_res1`; TeraDesk "Reboot" (`sdk/overlays/teradesk/main.c:2120,2142@rv`) |

The code of all five is constant (`TRAMPOLINE_CODE`, `USERDEF_STUB_CODE`,
`KBDVEC_DEFAULT_CODE`, `RET_STUB_CODE`, `RESET_STUB_CODE`); nothing in it depends
on the process. They need execute permission, but not in page 0: one
**shared** page, execute-only for EL0 (`AP=00`, `UXN=0`, ARM ARM
stage-1 permissions), at a fixed VA below 4 GiB (the OSHEADER fields are 32
bits) suffices for all processes. If they stay in page 0, page 0 must be
`UserRoExec` (data executable).

### 3. Write — there are writers, two of them in the kernel itself

EmuTOS: `$0..$7FF` are reachable on the Atari only in supervisor mode; a
program writes them after `Super()`/from `Supexec`, and the write succeeds.
In user mode the hardware raises a bus error, EmuTOS ends
the program (`any_vec` → `dopanic` → `kill_program`, `bios/vectors.S:152-153`,
`bios/kprint.c:302`, `:587-594`, `bios/panicasm.S:96-99`). NewTOS with page 0
read-only: permission fault at EL0 → `kill_crashed_program`
(`kernel/src/arch/aarch64/exceptions.rs:1471@kill_crashed_program`):
crash report, key, `Pterm(-1)`, the kernel keeps running (measured, see below).
A system call with an output buffer in page 0 ends the same way: `uaccess` checks
with the caller's permissions (`tos-api/src/uaccess.rs:453@user_kptr`).

| Writer | Location | What | Consequence with page 0 read-only |
|---|---|---|---|
| **Kernel stub** `G_USERDEF` | `tos-api/src/bdos/process/loader.rs`, `USERDEF_STUB_CODE` (as of `a90bcfb0`; the store instruction has been removed since MEM12) | `str w0, [0x760]` (return word of `ub_code`) | **measured**: TeraDesk ends on the first draw, `pc=0x724 far=0x760` |
| **Kernel scratch** PARMBLK `0x730` | `tos-api/src/aes/object.rs:2876@PARMBLK_SCRATCH_VA` | the kernel writes, `ub_code` reads — and may write: WinDom `xedit` changes `pb_x`/`pb_w` (`sdk/windom/windom-1.21.3/src/userdef.c:1819-1820@pblk`); in EmuTOS the PARMBLK lies on the writable AES stack | WinDom `XEDIT` ends; no `XEDIT` found in zView |
| DOOM | `sdk/apps/doom/doomgeneric_gem.c:1884,1896@kbdvec_hook` | hangs the `kbdvec` hook into the `Kbdvbase` table (`0x9A8`) | DOOM ends at start (keyboard via `kbdvec`) |
| `sdk/examples/kbdvec.c` | `:182,211@hook` | the same (KEYS02 test program) | ends |
| HypView | `sdk/apps/hypview/dl_routs.c:307-312@conterm`, caller `sdk/apps/hypview/window.c:320,512@KeyboardOnOff` | `conterm` `0x484` on **every** keypress in the window: `Ssystem(S_GETBVAL)` returns `EINVFN` on NewTOS (`tos-api/src/gemdos/ssystem.rs:121@EINVFN`), hence the `Supexec` fallback | HypView ends on the first keypress |
| TeraDesk "Reboot" (cold) | `sdk/overlays/teradesk/main.c:2114-2125@memval` | `memvalid`, `memval2`, `resvector`, `resvalid` = 0, then jump into the reset stub | TeraDesk ends instead of restarting (RST01 backwards) |
| TeraDesk Shift+Esc / after formatting | `sdk/apps/teradesk/file.c:945-969@hdv_rw`, callers `sdk/apps/teradesk/window.c:3284@force_mediach`, `sdk/apps/teradesk/floppy.c:878@force_mediach` | `hdv_rw`/`hdv_bpb`/`hdv_mediach` `0x472..0x47E` (NewTOS never calls them anyway) | TeraDesk ends |

Not built or dead: TeraDesk's `install_cookie` (`#if 0`), LDG's TSR core
(`cookiec.c`, not in `sdk/ldg/build-ldg.sh`), WinDom's cookie writer (the overlay
takes `Ssystem(S_SETCOOKIE)`), mintlib `set_sysvar_to_*` (built, no caller).

**Does the kernel read back what a program wrote in page 0?** Yes:

| Value | Reader in the kernel | Effect |
|---|---|---|
| `conterm` `0x484` | `tos-api/src/bios/chardev.rs:479@current_conterm` → key repeat `kernel/src/ikbd.rs:343@current_conterm`, `Bconin` shift folding `chardev.rs:514@current_conterm`, bell `tos-api/src/vdi/escape.rs:693@current_conterm` | bits 1/2/3 take effect |
| `kbdvec`, `ikbdsys` | `tos-api/src/xbios/kbdvecs.rs:209@read_slot`, `:212@kbdvec_of`, `:221@current_ikbdsys` | hook upcall, repeat stop |
| `memvalid`/`memval2`/`memval3` | `tos-api/src/bios/reset.rs:160@entry_kind` | warm/cold (on the Pi warm = cold) |
| return word `0x760` | `kernel/src/arch/aarch64/exceptions.rs`, `read_userdef_ret_slot` (as of `a90bcfb0`; removed since MEM12) | `G_USERDEF` result |
| any cell `< 0x800` | `tos-api/src/bios/vectors.rs:98-133@setexc` | `Setexc(n, v)` reads/writes the cell for `n` in `0x120…0x1FF` through the kernel alias — the API path, no EL0 memory access; for `n < 0x120` (also `$40C–$47F`) it reads/writes the shadow `VECTORS[]`, not the system variable (BIOS-CORE-006, [BIOS07](BIOS07.md); addendum 2026-10-28 from BIOS13). *Since BIOS07 (2026-10-29):* every cell `n < 0x200` is a page-0 cell — `n <= 0x103` one cell for all processes with a copy in every page 0, above it the system variable of the calling process (`docs/api/bios/05-setexc.md`, D3) |

### Decisions needed before the rebuild (open, with the lead)

Page 0 read-only is not achievable with today's contents without
TeraDesk (measured), DOOM, HypView and the KEYS02 test program ending.
The developer's proposal, per point with alternatives:

- **E1 Permissions of page 0.** Proposal `UserRoNx` (read, not write,
  not execute); the five stubs move into **one shared page, execute-only
  for EL0**. Alternative: `UserRoExec`, stubs stay (smallest
  change, data stays executable).
- **E2 What must stay writable moves.** `Kbdvbase` table
  (`0x9A0`, TOS idiom, DOOM) and PARMBLK (`0x730`) into **a second private
  page per process**, `UserRwNx`, at a fixed VA below 2 MiB, never mapped
  shared (Pi: below the kernel image `0x80000`; virt: below 1 GiB).
  `alias_of`/`resolve_kva`/`user_kptr` (`tos-api/src/uaccess.rs:482@alias_of`,
  `tos-api/src/bdos/process/usermem.rs`, `resolve_kva`) must treat it like page 0,
  `map_shared` must refuse the VA. Changes the location in
  `docs/api/xbios/22-kbdvbase.md` D1 ("in page 0"), not layout or effect.
  The return word `0x760` goes away: the stub passes `w0` in a register to
  `Psigreturn` (kernel-internal, without memory).
- **E3 `conterm` (HypView).** (a) Implement FreeMiNT `Ssystem` `S_GETBVAL`/`S_SETBVAL`
  (+ W/L) for the sysvar range (sanctioned extension, needs a
  spec; HypView then takes that path and never writes); (b) accept that HypView ends on
  a keypress; (c) overlay patch for HypView.
- **E4 TeraDesk "Reboot" and `force_mediach`.** (a) Overlay patch: on NewTOS
  omit the writes (the kernel never calls `hdv_*`; warm/cold is the same on
  the Pi); (b) accept that TeraDesk ends.
- **E5 `Setexc(n < 0x200, v)`** keeps writing into page 0 through the kernel
  (TOS API path, EmuTOS allows it). Confirm or restrict.
  *Addendum 2026-10-28 (BIOS13):* applies only to `n` in `0x120…0x1FF`; for
  `n < 0x120` `Setexc` writes the shadow `VECTORS[]` (BIOS-CORE-006,
  `bios-core-r1` 58–62, probe S06–S08; [BIOS07](BIOS07.md)).
  *Addendum 2026-10-29 (BIOS07):* now for all `n < 0x200` (D3); the
  vector cells `n <= 0x103` are one cell for all processes, every page 0
  carries its copy; the write path remains the kernel alias. MEM12 probe P0PAR
  afterwards PASS again (`measurements/measurement-2026-10-29-bios07/RUNINFO.TXT`).

Side finding: the crash report calls a write attempt on page 0
"Memory violation: hardware memory" (`memprot::region_kind` does not know page 0)
— name it as well during the rebuild.

### Owner decisions 2026-10-27 (verbatim)

> E1: (a). E2: yes; also specify whose Kbdvbase table receives an input event (owner/foreground). E5: yes.
> E3/E4: no Ssystem and no patches for now. Map the Atari protection model onto page tables instead:
> - page 0: EL0 read-only in user state, read-write while the process is in super state (inside Super()/Supexec), never executable
> - write in user state → bus error, as on Atari
> - HypView conterm and TeraDesk Reboot/Shift+Esc then run unpatched, if they write in super state. Verify per program (source + probe kernel) before building.
> - cold vs warm start: the Reset stub reads memvalid, as on Atari
> - the kernel treats page 0 as output only; it reads only documented input sysvars (conterm, memvalid …), validated
> Ssystem S_GETBVAL/S_SETBVAL: later, as a FreeMiNT extension (ADR-027), oracle = FreeMiNT in Hatari.
> The acceptance run must include the interactive paths (qemu-virt): HypView key, TeraDesk Reboot + Shift+Esc, DOOM keyboard.

So it holds: E1 = page 0 `UserRoNx`, stubs into a page of their own, execute-only
for EL0; E2 = second private RW page per process (Kbdvbase table,
PARMBLK), plus it is fixed whose table receives an input event;
E5 = `Setexc` stays; E3/E4 = supervisor state per process (page 0 then
EL0-RW), no overlay patches, no `Ssystem` now. Before the build: spec for
`Super`/`Supexec` (rule 5) and approval.

Spec draft (2026-10-27, awaiting approval): `docs/api/_super-state.md`
(super state), `docs/api/xbios/22-kbdvbase.md` part D (location of the table,
recipient D9).

### Super/Supexec in the programs (2026-10-27)

Source and built binary (`llvm-objdump -d`, state of the card):

| Path | Source: in super state? | Binary today |
|---|---|---|
| HypView `conterm` | yes: `Supexec(ChangeSysKeyconfig)` (`sdk/apps/hypview/dl_routs.c:325,334@Supexec`), after `Ssystem(0x0C)` = `EINVFN` | `KeyboardOnOff` @`0x594c`: `svc` Ssystem 12, then the callee **inlined**: `ldrb/strb [0x484]` @`0x59a8/0x59b4` |
| TeraDesk "Reboot" | yes: `Super(0L)` (`sdk/overlays/teradesk/main.c:2114@Super`), writes `:2121-2124`, then `Supexec(rv)` `:2142` | (path only via menu; the Super macro produces no instruction, see next row) |
| TeraDesk Shift+Esc | yes: `Super(0L)` … `Super(stack)` around the `hdv_*` accesses (`sdk/apps/teradesk/file.c:945-969@hdv_rw`) | `force_mediach` @`0xb3f4`: **no** instruction for `Super`; 8-byte `str` to `0x476`, `0x472`, `0x47E` @`0xb490/0xb49c/0xb4ac`, `Fopen`/`Fclose` via `svc` in between |
| DOOM `kbdvec` | yes: `Supexec(kbd_install)` (`sdk/apps/doom/doomgeneric_gem.c:1939@kbd_install`) | `kbd_install`/`kbd_probe` without a symbol of their own, inlined |

Conclusion: all four writers stand in the source inside the supervisor bracket; with
today's SDK macros this becomes ordinary EL0 code without any trap. No
kernel — not even a probe kernel — can see a super state in these binaries.
Only after the macros are switched to traps and the programs rebuilt
is "writes in super state" measurable; the measurement per path therefore belongs
after the build (probe kernel to cross-check the fault location: see plan).

Side finding TeraDesk (LP64 bug in the unmodified `file.c`, outside
MEM12): `hdv_bpb`/`hdv_rw`/`hdv_mediach` are 8-byte pointers at 4-byte
spacing. `hdv_bpb = …` overwrites the upper half of `hdv_rw`, the
re-check `hdv_rw == Newrwabs` fails, the old values are never
written back; `hdv_mediach` (8 bytes from `0x47E`) thereby sets `conterm`
(`0x484`) in TeraDesk's page 0 to 0 (key click, repeat, bell off).
**Fixed 2026-10-27 (checkpoint 1):** patch file `sdk/overlays/teradesk/file.c`
(the three cells as `uint32_t`, conversion via `uintptr_t`); evidence
`measurements/measurement-2026-10-27-mem12-td-lp64/RUNINFO.TXT` (objdump before/
after; raspi4b: after Shift+Esc `conterm` 0 → stays 7).
Likewise fixed (lead, `0edf53d0`): TeraDesk's `_sysbase` read in `gettos.c`
was 8 bytes wide (took `_shell_p` `$4F6` along as the upper half); patch file
`sdk/overlays/teradesk/library/utility/gettos.c` reads the 32-bit cell.

### Interactive acceptance: qemu-virt does not work, raspi4b with injector (plan)

qemu-virt has no `C:`: `cargo xtask run --sd` attaches the image only for
raspi4b (`tools/xtask/src/main.rs:1017@sd`), the kernel has no block driver
for virt (`kernel/src/drivers/qemu_virt/` = PL011 + ramfb), and HypView,
TeraDesk, DOOM are not embedded programs. `.claude/agents/tester.md`
("qemu-virt has input but no `C:`") is therefore still correct. Way: raspi4b with the
scratch injector `tools/inject-arm.py` (triggers `launch:NAME`/`up:NAME`),
as RST01 already ran TeraDesk "Reboot"
(`measurements/measurement-2026-10-17-rst01/RUNINFO.TXT` section 4). All four
paths wait in AES loops (`evnt_*`), in which the pump runs; none
hangs in `Cconin`. One card (#Z) and one script per path: HypView (+ a
`.HYP` file, key in the window), TeraDesk Shift+Esc (open a drive window,
Shift+Esc) and Quit → Reboot (`-no-reboot`: "[reset] … (Cold)" + QEMU exit),
DOOM (+ WAD from `Applications/doom-wad`, keys via the `kbdvec` hook).

### Owner decisions 2026-10-27, second round (verbatim)

> 1. S1–S12 approved. Add measurement cases for S3–S6 against the oracle: Super(1) return values, Super(0)/SuperToUser returns (address class), Supexec passes the callback's return value through.
> 2. O1: yes, the hook runs in super state (as EmuTOS from the interrupt).
> 3. O2: (a), documented deviation.
> 4. D9: yes. If an accessory ever installs a kbdvec hook: log a warning, so the deviation never bites silently.
> 5. Yes, raspi4b with inject-arm.py. Plus one run on the real Pi before MEM12 counts as done: toggling page-0 permissions needs correct TLB maintenance (TLBI + DSB/ISB), and QEMU often hides missing invalidations.
> 6. Yes, probe per program after the rebuild.
> TeraDesk LP64 bug: fix it before enabling MEM12. Once the kernel reads conterm (S10), TeraDesk's zeroed conterm turns off click, repeat and bell system-wide. This is a porting bug in our TeraDesk port, not an app patch for NewTOS.
> Real card: only after the real-Pi acceptance run, with all programs rebuilt in one go (manifest).

Sequence in three checkpoints (lead, 2026-10-27): (1) fix the TeraDesk LP64 bug
`hdv_*`; (2) measurement cases S3–S6 at the oracle, before any kernel change;
(3) rebuild per `docs/api/_super-state.md`, probes per program, acceptance run,
check sequence for the real Pi.

**Checkpoint 2 (2026-10-27): oracle side of cases S3–S6.** Dumper extension
(`docs/TOS-REFERENCE-DUMPER.md` section 6, "Super and Supexec in a
BIOS-area table": trap 1 only for `Super`, arguments `f:<n>`/`t:0`,
bracket rule), table `tosdump/cases/super-r1.tsv`, references
`tosdump/reference/EMUTOS-1a7484ab-SUPER` and `-SUPER-falcontc`; evidence
`measurements/measurement-2026-10-27-mem12-super-oracle/RUNINFO.TXT`. EmuTOS:
`Super(1L)` 0 / −1 / 0, `Super(0L)` = old SSP, `SuperToUser` without a
defined `d0` (IGNORE, state measured via the following query),
`Supexec` passes 0x12345678 and −2 through, `Super(1L)` in the callback = −1. The
NewTOS side follows after checkpoint 3.
Host tests of this checkpoint (`cargo xtask test` on `a7ffd707`): the
evidence run referred to a chat report that no longer exists; the
committed run stays unchanged. Re-measured 2026-10-28 from
`git archive` with its own target: **1706 / 0 / 1**
(`measurements/measurement-2026-10-28-mem12-hosttest/RUNINFO.TXT`; gitignored
inputs `references/` and `sdk/apps/teradesk` linked for it from today's tree,
named there).

## Work

- Result of 1–3 as a section in this ticket.
- Make page 0 non-writable for EL0 (at least `UserRo`/`UserRx`,
  depending on 2); with no readers, hide it entirely.
- A write from EL0 to page 0 ends the program, the kernel
  keeps running (test).
- Record the result in `docs/COMPATIBILITY.md` (page-0 section and
  the note in the section „No Line-A“).

## Acceptance

- Host test or selftest: EL0 write to page 0 ends with process end,
  not with a kernel panic; reading behaves as fixed under 1.
- Fresh build of both boards, QEMU boot to the desktop (raspi4b with
  `serial=on`); the card's applications start as before.
- `docs/COMPATIBILITY.md` describes the new state.

## Acceptance state 2026-10-27 (checkpoint 3b, QEMU)

Run `measurements/measurement-2026-10-27-mem12-acceptance/RUNINFO.TXT`
(kernel `34530791`, all programs rebuilt in one go, cards with manifest):

- **EL0 write to page 0 ends with process end, not with a kernel panic:**
  probe `sdk/examples/p0par.c` + `sdk/examples/p0kid.c` (raspi4b) — write
  in user state, execute from page 0, read and write of the stub page,
  write directly after leaving super state: each "Bus Error",
  `Pterm(-1)`, `0xFFFF` at the parent process; read, `Super` bracket,
  `Supexec`, `Kbdvbase` hook, end in super state: 0; "P0PAR: PASS".
- **Probes per program** (rebuilt, unchanged relative to upstream except
  the LP64 patch files): HypView key → `Supexec(ChangeSysKeyconfig)`,
  `conterm` 7 → 5 → 7 in super state; TeraDesk Shift+Esc → `Super` bracket
  around `hdv_*`, `conterm` stays 7; TeraDesk "Reboot" → `Super(0L)`,
  `memvalid` = 0, `Supexec(reset stub)` → "(Cold) requested"; DOOM →
  `Supexec(kbd_install)`, hook in the private page (`Kbdvbase` `0x7F010`),
  F10 + `y` only via the hook → ends with 0.
- **Oracle S3–S6:** `super-r1.tsv` NewTOS against EmuTOS: 0 of 9 cases
  different. APITEST SX01–SX09 OK (105/0/9).
- **Fresh build of both boards, boot to desktop:** raspi4b (`serial=on`) and
  qemu-virt; the card's programs (NEWDESK, TeraDesk, QED, PRESENT, zView,
  VDIDRAW, HypView, DOOM) start without a crash.
- **Open:** run on the real Pi (owner decision 2026-10-27,
  point 5), check sequence below. Only then does MEM12 count as done and goes
  onto the daily-use card.

## Check sequence real Pi (for owner/lead; the developer does not touch hardware)

Purpose: the TLB maintenance when switching page 0 (RO ↔ RW) on real
silicon; QEMU often hides missing invalidations.

1. **Card** (a test card, not the daily-use card): `kernel8.img` from
   `cargo xtask build --board rpi4 --release` of the committed state;
   `cmdline.txt` with `serial=on` if a UART adapter is attached. On `C:`
   from the one rebuild (`measurements/measurement-2026-10-27-mem12-acceptance/method/build-programs.sh`):
   `P0PAR.NAP` + `P0KID.NAP` (root), TeraDesk (`C:\Apps\TERADESK\`, as in
   `tools/make-sd-image.sh`), HypView with `HYP_VIEW.CFG` + `README.TXT`
   (`C:\APPS\HYPVIEW\`, contents in the run under method), DOOM + `DOOM1.WAD`
   (`C:\DOOM\`), `DUMPBIOS.NAP` + `tosdump/cases/super-r1.tsv` as `C:\BIOS.TSV`.
   A manifest of the card (`tools/sd-manifest.sh`) belongs to the run.
2. **Boot:** desktop appears; serial `[desk] desktop up`.
3. Start **P0PAR** (double-click): five crash reports in succession
   ("Memory violation: page-0 system-variable memory, write …",
   "… execute …", "execute-only stub memory, read/write", again "page-0 …
   write"), answer each with a key. Result on screen:
   all lines `OK`, "own state after L: user OK", **"P0PAR: PASS"**. A
   `WRONG` at case `S` (write in super state is rejected) or at
   case `T` (write after leaving succeeds) would be a TLB error.
4. Start **HypView**, press arrow-down several times in the window: no crash;
   then key repeat (hold a key) and key click as before.
5. Start **TeraDesk**, open drive C:, press Shift+Esc several times: no
   crash, click/repeat stay on. Then Ctrl+Q → "Reboot": the Pi
   restarts (serial `[reset] OSHEADER reset entry (Cold) requested`).
6. Start **DOOM**, walk with the arrow keys, F10 → `y`: back to the desktop,
   the desktop's keyboard works.
7. **DUMPBIOS** (double-click) writes `C:\BIOS.BIN`; on the host
   `tosdump/host/build/tosdiff tosdump/cases/super-r1.tsv tosdump/reference/EMUTOS-1a7484ab-SUPER.BIN BIOS.BIN`
   → "0 of 9 cases differ".
8. **USB keyboard → `Iorec(1)` ring** (from XBIOS01, user 2026-10-29): keys
   on the USB keyboard land in the IKBD ring of `Iorec(1)` (`Bconstat(2)` -1,
   `tail` grows in 4-byte steps; probe from
   `measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`).
9. **Probe K16 with a physical shift key:** expects a value of the class
   `0x02300042` (shift bit in the upper byte set, shift at the time of the
   keypress; QEMU value from the XBIOS01 run).
10. Result with serial log and photos in one run under
   `measurements/` and in `docs/HW-TEST-LOG.md`.

## Closure (owner, 2026-10-30)

- MEM12 is closed with reference to the owner's Pi run of
  2026-10-29 (release kernel `58e35ac8`, contains MEM12): desktop, programs and
  keyboard ran in daily use; the DOOM freeze was noticed there ([DOOM01](DOOM01.md)).
- The check sequence above was not formally run. `P0PAR` (TLB maintenance when
  switching page 0) moves as the first diagnostic step to
  [DOOM01](DOOM01.md) and into the later test scenario for the testers.
- Steps 10/11 (AUX wait from BIOS09, RXD0 from UART01) are removed;
  both topics are parked or go into the test scenario.


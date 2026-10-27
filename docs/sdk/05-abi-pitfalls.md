# 05 — The ABI, and the traps that have bitten ports so far

Part A states the contract a native program compiles against. Part B is the
catalogue of things that went wrong in real ports (QED, TeraDesk, CoreMark,
apitest) — each with the symptom, the cause as found in the sources, and what
to do. Nothing here is theoretical; every entry has a file behind it.

---

## Part A — the contract

### A1. The trap ABI

Verified in the kernel dispatcher (`kernel/src/arch/aarch64/exceptions.rs`)
and the argument carrier (`tos-api/src/trap.rs`):

| Item | Value | Source |
|------|-------|--------|
| GEMDOS | `svc #1` | `trap.rs:22-35`, `TrapClass::from_imm` `:40-48` |
| GEM (VDI + AES) | `svc #2` | same |
| BIOS | `svc #13` | same |
| XBIOS | `svc #14` | same |
| function id | `x8`, low 16 bits significant | `trap.rs:60-61`, `:67-71`; `exceptions.rs:284` (`frame.gpr[8] as u16`) |
| arguments | `x0..x7`, left to right, one register per argument | `trap.rs:57-59`, `exceptions.rs:326-338` |
| WORD arguments | the kernel reads the low 16 bits (`arg_i16`/`arg_u16`), LONGs the low 32 (`arg_i32`), pointers all 64 (`arg_u64`) | `trap.rs:73-102` |
| result | `x0`, a sign-extended `i64` of the TOS LONG; BIOS `Getbpb` returns a full 64-bit pointer | `exceptions.rs:359-362`, `:434` |
| GEM trap | `x0` = selector (`115` VDI, `200` AES), `x1` = parameter block of native pointers (VDI: contrl,intin,ptsin,intout,ptsout; AES: control,global,intin,intout,addrin,addrout) | `exceptions.rs:363-392`, `sdk/gemlib/overlay/_gc_aes.c:9-24`, `_gc_vdi.c:9-24` |
| GEM trap result | VDI: `flip_y` zero-extended in `x0`; AES: `0`; selector `−1`: `0` (no jsr-able dispatcher exists — documented deviation); any other selector (e.g. the `−2` GDOS probe): `x0` **unchanged** | `exceptions.rs:363-431` |
| registers preserved | `x1..x30` are saved on entry and restored before `eret`; only `x0` is rewritten with the result | `kernel/src/arch/aarch64/exception_vectors.s:25-46`, `:51-71` |
| unknown `svc` immediate | `EINVFN` in `x0` | `exceptions.rs:270-277` |
| frame-mutating GEMDOS calls | `Pterm`/`Pterm0`/`Ptermres`/`Pexec`/`Pwait`/`Psigreturn` are dispatched before the table | `exceptions.rs:283-318` |

The clean-room helpers in `sdk/include/newtos.h` and the retargeted mintlib
macros in `sdk/libc/overlay/include/mint/osbind.h:119-160` are direct
transcriptions of this table. `docs/ARCHITECTURE.md:48-54` states the same
for `svc #1/#13/#14` but does not mention `svc #2` (a documentation gap, see
the report).

### A2. The process address space

From `tos-api/src/bdos/process/loader.rs` and `mmu.rs` (ARCH03 stage 1: one
address space shared by every process, every user address = its physical
address, except page 0):

```
page 0 (per process)
VA 0x0000_0400   classic sysvars ... _sysbase@0x4F2, _p_cookies@0x5A0
   0x0000_05B8   os_run cell (this process's PD pointer)
   0x0000_05C0   OSHEADER (48 B)                            loader.rs (OSHEADER_VA)
   0x0000_05F0   cookie-jar mirror, 16 slots × 8 B
   0x0000_0700   upcall trampolines / scratch
   0x0000_0768   Keytbl area (0x238 B), 0x9A0 KEYS02 vector table

shared (VA = PA): the BDOS pools, published kernel structures, the screen
B                BasePage (256 B) -- B = the TPA block = the PD pointer
B + 0x100        reserved; crt0's start-up stack
B + 0x1000       program image (PT_LOAD, relocated for B), then .bss
_end             the stack crt0 keeps (_stksize, 64 KiB default)  heap_init.c
B + m            end of the TPA after crt0's Mshrink(B, m)
p_hitpa          the original TPA top = B + the largest free block (E9)
elsewhere        Malloc/Mxalloc blocks (their pool addresses; libc malloc
                 too), the RSC blob, other processes' TPAs
```

The whole pool is mapped user-read/write/execute; the image span (BSS
included) is zeroed before the image is copied, the rest of the TPA only
without `PF_FASTLOAD` (EmuTOS `kpgmld`). Every user address is below 2 GiB (plan
invariant I2) — which is why the 32-bit fields of the classic TOS structures
can hold a NewTOS pointer (`sdk/gemlib/overlay/gem.h:62-64`, `sdk/libc/overlay/include/mint/basepage.h:16-17`).

The basepage the loader writes is the classic 256-byte layout with 4-byte
fields (`loader.rs` `populate_basepage`; kernel struct
`tos-types/src/basepage.rs`): `p_hitpa = B +` the largest free block (E9),
`p_tbase`/`p_tlen` = the
first executable `PT_LOAD`, `p_dbase`/`p_dlen`/`p_bbase`/`p_blen` = the first
writable non-executable one (file part / BSS tail). Since TASK/T05
`newtos.ld` emits exactly those two segments (`text` R-X, `data` RW- with
`.bss` as its memsz-only tail), so all six fields are filled and satisfy the
EmuTOS relations `p_dbase = p_tbase + p_tlen`, `p_bbase = p_dbase + p_dlen`,
`p_bbase + p_blen = _end` (`bdos/kpgmld.c:131-132`; APITEST T32 checks them
on the machine). Before T05 the single `FLAGS(7)` segment left the four
data/BSS fields **zero** — a program without `.data`/`.bss` (the hand-crafted
`tools/mk-hello-ncp.py` HELLO.NCP) still gets that. `p_xdta` = `&p_cmdlin`,
`p_env` = the environment every process gets as on EmuTOS (an AUTO program
`{0, 0}`, a desktop / `#Z` program and an accessory the AES's
`"PATH=\0X:\\\0"`, a `Pexec` child its caller's or the given block),
`p_parent` = the PD that started it — a readable stand-in PD for the AUTO,
desktop and `#Z` starts, 0 only for an accessory, which is how the start-up
code tells one (TASK/PPARENT,
`docs/api/gemdos/process/_parent-and-environment.md`), `p_cmdlin` = the tail
verbatim (length byte first).

### A3. What a program gets, and from where

| Resource | How | Limit |
|----------|-----|-------|
| stack | crt0 / `__libc_crtinit` keep `_stksize` bytes (mintlib default MINKEEP = 64 KiB) above the image and `Mshrink` the TPA to image + stack (`sdk/crt0.S`, `sdk/libc/newtos-arch/heap_init.c`, mintlib `sdk/libc/mintlib/mintlib/crtinit.c:154-247`); an accessory gets a `Malloc`'ed stack (`_acc_main`) | `_stksize` (mintlib's rules: `-1` keep all, `0` MINKEEP, `1/2/3` quarters, `n` bytes, `-n` bytes + own heap; env `STACKSIZE`) |
| libc `malloc` | mintlib `__sbrk` → GEMDOS `Malloc` in hunks of 8–32 KiB (larger requests one block each) with the default `_stksize` (`_split_mem`, `sbrk.c:54-73`, `sdk/libc/overlay/mintlib/malloc.c:64-95`); with `_stksize < -1`/`-1` the own heap above the image | the pool (`PF_TTRAMMEM` → TT-RAM preferred); every hunk is one BDOS block: at most 8192 blocks per process (`mdlimit=`), no fixed system-wide MD count (MEM05, `docs/COMPATIBILITY.md`) |
| GEMDOS `Malloc`/`Mxalloc` | the pool block's own address, valid in every process (ARCH03 stage 1, VA = PA; `tos-api/src/gemdos/memory.rs`) | the pool itself — no per-process window or block-table limit any more |
| kernel structures returned by pointer | published read-only into the window (ADR-023): `Getbpb`, `Getmpb`; screen memory published read/write, uncached: `Physbase`, `Logbase` | see B3 for what is still not published |
| system variables, OSHEADER, cookie jar | materialised in page 0 of the TPA at the classic addresses (`loader.rs:861` `populate_sysvars`) | read-only in substance: the cookie mirror is rewritten from the kernel jar on every change (`loader.rs:1057-1100`); writes to the mirror do not reach the kernel |

---

## Part B — the pitfalls

Each entry: **Symptom** (what you see), **Cause** (with source), **Do**
(the countermeasure the tree already uses).

### B1. `long` and pointers are 8 bytes; TOS structures are 32-bit on the wire

The single most productive bug class. The kernel writes and reads TOS
structures in their classic 4-byte-LONG layout because that *is* the
contract; a mintlib `struct` declared with `long`/`char *` fields is
therefore the wrong shape on LP64.

| Structure | m68k → LP64 size with pristine mintlib | What broke | Fixed in |
|-----------|-----------------------------------------|-----------|----------|
| `BASEPAGE` | 256 → 376; `p_env` moves from 0x2C to 88 | every field after `p_lowtpa` misread by a program using `_base->…` (fixed as a *latent* trap before a port hit it: commit `ff67aa6`, "Fixes the latent lp64-tos-struct-trap for readers") | `sdk/libc/overlay/include/mint/basepage.h` — every field `__UINT32_TYPE__`/`__INT32_TYPE__`, `_Static_assert` on `p_hitpa@4`, `p_tbase@8`, `p_env@0x2c`, `sizeof == 256` (`:35-64`) |
| `_DTA` | 44 → 48; `dta_size` widens, `dta_name` moves 30 → 34 | every filename returned by `Fsfirst`/`Fsnext` shifted by 4 bytes (QED reads the DTA directly) | `ostruct.h:70-102` — `dta_size` is `__INT32_TYPE__`, struct `packed`, offsets pinned to `makbuf` (`tos-api/src/bdos/fs/find.rs`) |
| `_DISKINFO` | 16 → 32 | TeraDesk's drive info showed garbage: `b_free` read `(b_total << 32) \| b_free`, `b_secsiz`/`b_clsiz` read uninitialised stack (`TASK/T01.md`) | `ostruct.h:22-53` — four `__UINT32_TYPE__`, `packed`, pinned to the kernel writer `tos-api/src/bdos/fs/drive.rs` (`5a5820b`, 2026-09-17) |

**Symptom** in general: fields read as two adjacent 32-bit values fused into
one, or as stack garbage; filenames shifted; a pointer that is
`0x0000_0100_0000_xxxx`-shaped.

**Cause**: `sizeof(long) == 8` and `sizeof(void *) == 8` on
`aarch64-unknown-none-elf`; the kernel writes 4-byte fields
(e.g. `drive.rs` `dfree`: four stores at `p.add(i*4)`).

**Do**: declare the on-wire structure with fixed-width types
(`__UINT32_TYPE__`, `__INT32_TYPE__`), add `__attribute__((packed))` where a
wider field would otherwise introduce padding, and pin **every** offset and
`sizeof` with `_Static_assert` against the kernel's writer — the assert is
the fix that would have caught `_DISKINFO` two months earlier
(`TASK/T01.md`, "Was zu tun ist"). Where the field must hold a real pointer
in C (`(char *)`), the read site needs `(T *)(uintptr_t)` — see the
`environm.c` overlay and B7.

**The rest of the neighbourhood is measured, not fixed.** 20 of the 30
`mint/*.h` typedefs still diverge between m68k and LP64
(`measurements/measurement-2026-09-17-diskinfo/04-struct-sizes-both-toolchains.txt`).
Three of them are read by a built application today
(`measurements/measurement-2026-09-17-diskinfo/08-struct-reach-survey.md`):

| typedef | m68k → LP64 | Reader | State |
|---------|-------------|--------|-------|
| `OSHEADER` | 48 → 96 | TeraDesk `gettos.c` (`_sysbase->os_version`), apitest T04 | correct **by accident**: only `os_entry`/`os_version` at offsets 0/2 precede the first pointer; everything from `reseth` on is shifted |
| `MFDB` | 20 → 24 | TeraDesk, QED via cflib | correct: the kernel decodes the MFDB at the caller's ABI width (`tos-api/src/vdi/raster.rs:109-117`); `fd_addr` is a real pointer and cannot be pinned to 32 bits |
| `_KEYTAB` | 28 → 56 | TeraDesk `xdevent.c`, QED via cflib `nkcc` | correct: `Keytbl` writes each pointer field at the caller's width into the process's own KEYTBL area (`tos-api/src/xbios/keytbl.rs`) |

`_MD`/`_MPB` (behind `Getmpb`), `_KBDVECS`, `_IOREC`, `_MAPTAB`, `_BCONMAP`,
`_PBDEF`, `_DSPBLOCK`, `META_INFO_*`, `HDINFO`, `BBPB`, `SDDB`,
`__FONT`, `MCF_COOKIE`, `_CCONLINE` diverge and are read by nobody today.
`__LINEA` is not a divergence to watch: NewTOS exports no LINEA block and
`__aline` stays NULL by design (ADR-030).
**If your port reads one of them, it is the next `_DISKINFO`.** Note that
`_MD` is special: the kernel's own `MD` is `{ptr 8, long 4, long 4, ptr 8}` =
24 bytes (`tos-types/src/memory.rs`, ADR-023), neither the m68k 16 nor the
LP64 32 — apitest declares its own `MDX`/`MPBX` for it.

### B2. The cookie jar at `$5A0`

The classic idiom `for (c = *(COOKIE **)0x5A0; c->tag; c++)` compiles and
runs — and finds nothing.

**Symptom**: `find_cookie` never finds a cookie; a scan walks past the
terminator into zero fill. Measured by apitest SX16/SX17: `sizeof(ported
COOKIE) = 16`, the stride scan takes 3 steps from `0x5F0` and stops at
`0x620`, past the terminator at `0x610`
(`docs/api/bios/_cookie-jar.md:267-304`).

**Cause**: two independent LP64 effects. (1) An **8-byte read of `$5A0`**
fuses `_p_cookies` with `ramtop`@`$5A4`: measured `0x3BE00000000005F0`. (2) A
jar slot is two 32-bit LONGs = 8 bytes (`loader.rs:1033-1055` writes
`wr_long(tag); wr_long(value)` per slot), but `struct { long name; long
value; }` is **16** bytes on LP64, so `cookie++` reads every second slot and
the `{0, capacity}` terminator reads as `capacity << 32 ≠ 0`.

**Do**: call `Getcookie()` — the SDK provides it over the `Ssystem`
trap (`sdk/libc/newtos-arch/getcookie.c`), and cflib's `cookie.c` already
takes that branch under clang (`TASK/X01.md`, correction 2). If the app
*must* scan the jar itself, read `$5A0` as a **4-byte** value
(`Setexc(0x5A0/4, -1)` returns exactly that, which is why TeraDesk's live
path works) and declare the slot as two `__UINT32_TYPE__`. A
toolchain-level fix (narrowing the ported struct, or an LP64 mirror) is an
open ABI decision, not done (`_cookie-jar.md:302-304`). Writes into the mirror
never reach the kernel jar; use `Ssystem(S_SETCOOKIE)`.

### B3. Kernel addresses vs. user addresses

**Symptom**: a data abort at EL0 with `FAR = 0xFFFF_0000_xxxx_xxxx` right
after a BIOS/XBIOS call that returns a pointer.

**Cause**: the kernel lives in the high half (`0xFFFF_0000_0000_0000 +
phys`); a `&'static` returned as-is is unreachable from EL0. Measured 2026-09-05:
`Getbpb(2) = 0xFFFF00000026D8F4`, `mp_mfl = 0xFFFF00000026D8B8`
(`docs/MEMORY.md:1566-1573`).

**State today** (ADR-023 and its amendments, `docs/MEMORY.md:1546-1846`;
`docs/STATUS.md` Recent Decisions 2026-09-05):

| Call | Returns to EL0 | Since |
|------|----------------|-------|
| `Getbpb` (BIOS 7), `Getmpb` (BIOS 0) | user VA, read-only page (`PROT_PR`) — measured `0x00401024`, `0x00402000` | 2026-09-05 |
| `Physbase` (XBIOS 2), `Logbase` (XBIOS 3) | user VA of the **whole** screen memory (both pages), read/write, **Normal Non-Cacheable** — plain stores land in DRAM, no cache maintenance needed; measured `0x00400000` | 2026-09-05 |
| `Keytbl` (XBIOS 16) | user VA — the tables are copied into the process's page 0 at `0x768` | never affected |
| `Kbdvbase` (XBIOS 34) | **still a kernel VA** (`tos-api/src/xbios/system.rs:131-134`); the kernel does not dispatch through those vectors, so publishing it writable "would be a promise without cover" | open |
| `Iorec` (XBIOS 14) | not implemented (returns the function number) | open |
| `Bconmap` | not a pointer (`0x2C`) | n/a |

**Do**: never dereference a pointer from `Kbdvbase`; range-check any returned
pointer against the process window (`< 32 MiB`) before use — apitest's
`el0_readable()` does exactly that and reports instead of faulting. When
writing to the screen through `Physbase`, the pixel format is the
framebuffer's own **XRGB8888**, not bit-planes (`sdk/examples/physbase.c:24-29`),
and the page size for double buffering has to be probed with
`Setscreen`/`Logbase` (`sdk/examples/scrflip.c:14-25`) — there is no
`Vgetsize` (open question, `docs/STATUS.md` Recent Decisions 2026-09-05, "Offen").

### B4. `Super`, `Supexec`, `SuperToUser`

**Symptom** (before 2026-09-14): `sdk/libc/mintlib/mintlib/setsysvar.c` and `sdk/libc/mintlib/unix/sysinfo.c`
did not compile ("unknown register name 'a0'"); a program bracketing a
low-memory read with `Super(0)`/`Super(ssp)` got `EINVFN`.

**Cause and state**: NewTOS cannot raise an EL0 program to EL1. From
2026-09-14 to MEM12 (`TASK/X01.md`) the bindings therefore answered without
a trap (`Super` an EL0 no-op, `Supexec(f)` a direct call of `f`), because
page 0 was writable from EL0 anyway. Since MEM12 (`60d3c4fd`) both are real
traps again: the kernel keeps a per-process **super state**
(`docs/api/_super-state.md`). In user state page 0 is read-only for EL0 and
a write there ends the program like a bus error; `Super(x)` enters or leaves
the super state, in which page 0 is writable; `Supexec(f)` runs `f` at EL0
as an upcall in the super state and returns its `x0`
(`sdk/libc/overlay/include/mint/osbind.h:470@Super`, `:696@Supexec`; the
EmuTOS derivation is in the comments). The code runs at EL0 either way;
nothing else becomes privileged.

**Do**: rebuild with the current header. A binary built with the old no-op
`Super` still reads its system variables, but its page-0 writes now end it.
The classic bracket idiom works. A real `Supexec` makes TeraDesk's shutdown
path live (`sdk/apps/teradesk/main.c:2118@Supexec`, `Supexec(rv)` with the
endless-loop `lobo()`). And do not expect privilege: an `Supexec`'d function
that pokes MMIO faults.
The pre-MEM12 cast macro drew clang's `-Wunused-value` warning on a
statement `Supexec(f);` (HypView's `init.c`/`dl_routs.c`, TASK/SDK01 H).

### B5. `Fattrib` on directories — TOS 4 / FreeMiNT semantics

**Symptom**: a cflib application (QED) reported "clipboard folder could not
be created" on its **second** start, although the folder existed
(`docs/analysis/q07-qed-klemmbrett-fattrib-trailing-backslash-2026-09-12.md`).

**Cause**: cflib's `path_exists` probes a directory with
`Fattrib(path, 0, 0) >= 0 && (attr & FA_DIR)` and `make_normalpath` appends a
trailing separator first, so the call is `Fattrib("C:\CLIPBRD\", 0, 0)`. On
EmuTOS and TOS 1–3 that is `EFILNF` in both spellings (measured on Hatari:
q07 §3, cases 01/02/17); FreeMiNT answers with the folder's attributes.

**State**: NewTOS adopts the FreeMiNT/TOS4 superset: `Fattrib` GET on a
directory returns the attribute byte with `FA_SUBDIR`, with **and without** a
trailing separator; SET on a directory stays `EACCDN`; `Fsfirst`, `Fopen`,
`Dcreate` on a trailing-separator path are unchanged; `Dcreate` on an
existing folder is `EACCDN` everywhere (`docs/COMPATIBILITY.md:262-282`,
`TASK/Q07-STATUS-DONE.MD`).

**Do**: nothing — but if a port expects the TOS 1–3 answer (`EFILNF` for a
folder) it will take the other branch. The memory note
`cflib-apps-expect-tos4-semantics` records this class: when a ported
FreeMiNT/cflib app misbehaves, check the TOS 1–3 vs TOS 4/FreeMiNT fork first.

### B6. `argc`/`argv`, the command tail, and the environment

**Symptom**: `if (argc > 1) use(argv[1])` faults, or the app never sees the
document it was started with.

**Cause (until 2026-09-25)**: the libc crt0 passed a hardcoded `argc = 1`,
`argv = {"", NULL}`, because mintlib's `crtinit.c` — which parses `p_cmdlin`
in `parseargs()` — is m68k assembly in its other half and excluded
(`sdk/libc/expected-failures.txt:27`). Every libc-linked program therefore saw
no arguments whatever the launcher handed it; a file dropped on QED opened an
empty editor (TASK/PORT01, `measurements/measurement-2026-09-25-port01/`).

**Now**: `sdk/libc/crt0.S` calls `__libc_crtinit()`
(`sdk/libc/newtos-arch/heap_init.c`, mintlib's `_crtinit` for AArch64), which
calls `__libc_parseargs()` (`sdk/libc/newtos-arch/cmdargs.c`, mintlib's
`parseargs()`), and then, since TASK/LIBC-MAIN, mintlib's own `_main`.
`__libc_parseargs()` builds `argc`/`argv` from the tail
with mintlib's own rules — `argv[0]` is `""`, leading blanks skipped, split on
runs of blanks/tabs, MagX single-quoting honoured — and publishes
`__libc_argc`/`__libc_argv`. Host-checked by
`sdk/libc/tests/cmdargs-host.sh` (in `sdk/run-host-tests.sh`).

Still true: the desktop passes an **empty** tail on a plain double-click and
the document path only for an association or drop launch
(`programs/desktop/src/main.rs`). Since TASK/PORT09 (2026-10-06) the start-up also
walks `p_env` into `environ` and honours the Atari `ARGV=` extension
(arguments in the environment when the tail is too short; `argv[0]` is then
the program's path), as mintlib's `parseargs()` does — including its in-place
rewrite of the block (cut at `ARGV=`, `"PATH=\0C:\\\0"` joined to
`"PATH=C:\\"`), see [07](07-mintlib-limits.md) §5. A program that must work with either route can still
read the tail itself from the basepage (`p_cmdlin` at offset 0x80: length
byte, then text — `loader.rs:680-691`) or use `shel_read`
(`sdk/apps/chaintest/chaintest.c:41-47` shows both directions, including a
`shel_write` that passes a tail). For a benchmark that needs parameters, build
one binary per configuration (CoreMark's `CORE.NCP`/`COREVAL.NCP`).

### B7. mintlib's `stdint.h` gives a 32-bit `uintptr_t` on our LP64 build

**Symptom**: a pointer round-tripped through `uintptr_t` loses its upper
half; CoreMark's `check_data_types()` would report a size error.

**Cause**: `sdk/libc/mintlib/include/stdint.h:132-143` defines `intptr_t`/
`uintptr_t` as `int`/`unsigned int` unless `__MSHORT__`; with `-nostdinc -I$MINT
… -isystem $RES` that header shadows clang's (`core_portme.h:182-198`).

**Fixed (TASK/PORT10, 2026-10-02)**: the overlay
`sdk/libc/overlay/include/stdint.h` makes `intptr_t`/`uintptr_t` `long`/
`unsigned long` on LP64 — the type mintlib's own `<unistd.h>` already used for
`intptr_t` (`sdk/libc/mintlib/include/bits/types.h` `__intptr_t`), so the two headers no longer
disagree by include order — and sets `INTPTR_MIN/MAX`, `UINTPTR_MAX`;
`inttypes.h`'s `PRI*PTR`/`SCN*PTR` follow. QED and TeraDesk rebuilt: 0
undefined, two TeraDesk `environm.c` truncation warnings gone, none new.
`PTRDIFF_MIN/MAX` and `SIZE_MAX` (same header) and `SSIZE_MAX` (overlay
`sdk/libc/overlay/include/bits/posix1_lim.h`) stayed 32-bit until TASK/SDK01 A (2026-10-20)
and are the LP64 values since (`sdk/libc/tests/lp64-limits.sh`, all
`_Static_assert`). Code that already spells `__UINTPTR_TYPE__` (CoreMark's
`core_portme.h`) stays correct.

### B8. Resource files: format, alignment, and who loads them

**Symptom** 1: `rsrc_load` fails on a classic Atari `.RSC`.
**Cause**: the kernel loads only the native NRSC format (`tos-api/src/aes/resource.rs:440-460`
checks the `NRSC` magic at `:458`). **Do**: `python3 tools/mkrsc.py convert app.rsc
app.rsc` and ship the converted file beside the `.NAP`.

**Symptom** 2 (2026-09-11): TeraDesk panicked at start in a debug kernel —
`slice::from_raw_parts requires the pointer to be aligned` in `blit_plane`;
in a release kernel the same access ran as undefined behaviour.
**Cause**: the converter interned strings and raster data into one section
without alignment, so an odd-length label pushed the next icon raster to an
odd offset (36 of 76 rasters in `icons.rsc`); every raster consumer reads
16-bit words (`tools/mkrsc.py:87-100`, `docs/STATUS.md` 2026-09-11,
`docs/HW-TEST-LOG.md:129`). **Do**: regenerate with the current `mkrsc.py`
(it pads to `RASTER_ALIGN = 2` and asserts the result); the kernel side now
reads rasters unaligned as well.

**Symptom** 3: an application with its **own** resource loader (TeraDesk
`xrsrc_load` for `icons.rsc`) reads garbage from a kernel-format file.
**Do**: convert those with `mkrsc.py convert --rshdr` (native-RSHDR onto LP64
structs, `build-teradesk.sh:189-205`). A new-format `CICONBLK` file (colour
icons) is for the **kernel's** `rsrc_load` (`mkrsc.py convert`, NRSC v13,
since A41) — `--rshdr` refuses it.

**Symptom** 5 (2026-09-19, A41 part 2): after a successful `rsrc_load` the
application faults with a wild pointer built from the bytes `"NRSC"`.
**Cause**: the application parses the image behind `ap_rscmem`
(`aes_global[7]`) as a classic `RSHDR` — TeraDesk's `xrsrc_gaddr`
(`sdk/apps/teradesk/rsc_load.c:496-541`) does that on the AES-loaded `cicons.rsc` — because
that is what EmuTOS leaves there (`rs_readit`, `gemrslib.c:787`: the loaded
file, `RSHDR` at its head). A blob converted before A41 part 2 (NRSC v1–v12)
has the NRSC header at its first byte instead. **Do**: reconvert with the
current `mkrsc.py convert` — it emits NRSC **v13** (LP64) / **v14** (ILP32),
which begin with a 36-byte native `RSHDR` (18 little-endian `u16`, classic
field order, offsets from the file's first byte; `rsh_vrsn` the source's,
never `& 3 == 3`) padded to 40 bytes, the NRSC header at byte 40. The kernel
reports that first byte as `ap_rscmem`, `rsh_rssize` as `ap_rsclen` and
`ap_rscmem + rsh_trindex` as `ap_ptree`, and checks the preamble against
the NRSC header before relocating (`tos-api/src/aes/resource.rs`
`rshdr_is_consistent`). Every `RSHDR` field is a `u16`, so an image must
stay under `RSC_CAP − 40` bytes (the converter refuses a larger one). A
v1–v12 blob still loads, but an application must not parse its `ap_rscmem`
as an `RSHDR`.

**Symptom** 4 (2026-09-19, A40): a dialog or menu bar converted before A40
keeps the layout of a 640-wide 8×16 screen on every machine — an
80-character-wide root object is 640 px on the Pi 4's 1920×1080.
**Cause**: `mkrsc.py convert` used to do the char→pixel fix itself with
`WCHAR, HCHAR, SCREEN_W = 8, 16, 640` baked in (NRSC v7/v8 blobs hold pixels),
while EmuTOS converts at **load** with the live `gl_wchar`/`gl_hchar`/`gl_width`
(`aes/gemrslib.c:53-99`, `rs_fixit`/`fix_objects`/`rs_obfix`). **Do**: regenerate
with the current `mkrsc.py` — it now emits NRSC **v9** (LP64) / **v10** (ILP32)
with the classic file's packed char-grid words untouched, and the kernel's
`rsrc_load` runs `rs_obfix` over every object at load
(`tos-api/src/aes/resource.rs::fix_objects_coords`). A v7/v8 file still loads,
with its baked-in pixels. `rsrc_obfix` (114) is the same function, so an
inline `rs_object[]` and a loaded `.RSC` agree. Nothing in the app changes;
only the `.rsc` beside the `.NAP` has to be reconverted. Note that
`build-qed.sh` does not produce `qed.rsc` — see
[`04-porting-anatomy.md`](04-porting-anatomy.md) for the manual step.

**Finding trees and free strings yourself.** Some applications (HypView's
`dl_init.c`) do not call `rsrc_gaddr` per object but read the `RSHDR`
behind `ap_rscmem` and index the tables at `rsh_trindex`/`rsh_frstr`
themselves. Read the header pointer through gemlib's `_AESrscmem` (B9), not
`*(RSHDR **)&aes_global[7]`. The in-memory layout of those two tables in a
converted (NRSC) file is not specified on this page; the route that depends
on nothing but the AES contract is `rsrc_gaddr(R_TREE, i, &tree)` per tree
and `rsrc_gaddr(R_FRSTR, 0, &p)`, which returns the **address of** the
free-string pointer table (`docs/api/aes/resource-fileselector.md`), so `p`
can be used as the `char **` table. `sdk/overlays/hypview/dl_init.c` does
this (TASK/SDK01 H).

Also on LP64: `OBJECT` is 32 bytes with a 64-bit `ob_spec` (`docs/COMPATIBILITY.md:232-246`),
and gemlib's `BFOBSPEC` bit-field is declared in **reverse** order on a
little-endian target so `framesize`/`framecol`/… land on the documented bits
(`sdk/gemlib/overlay/mt_gem.h.in:2053-2100`; without it TeraDesk drew 111 red
lines instead of a 2-pixel corner).

### B9. `AESGLOBAL` is 15 WORDs with 4-byte pointer fields

**Symptom**: `_AESrscmem`/`_AESrscfile`/`_AESappglobal` read wild pointers
(e.g. `planes << 48`), or a write to one clobbers the neighbours.

**Cause**: gemlib's pristine accessors read `*(long *)&global[3]`,
`*(OBJECT ***)&global[5]`, `*(void **)&global[7]` — 8 bytes each on LP64,
overlapping inside a 30-byte array whose fields are 4 bytes
(`sdk/gemlib/overlay/gem.h:40-77`).

**Do**: use gemlib's macros — the overlay narrows them to 4-byte reads and
turns the two pointer fields into explicit setters
(`_AESrscfile_set`/`_AESrscmem_set` in `sdk/gemlib/overlay/gem.h:79-125`, `mt_AESrscfile_set` in `mt_gem.h.in:2524-2575`). Never
declare your own `long` view of `global[]`. Related: the AES reports version
`0x0340`, so gemlib/cflib never issue `appl_getinfo` (they gate on
`>= 0x0400`); do not rely on AES-4 queries (`docs/analysis/qed-launch-findings-log.md`, F1).

### B10. Pointer arguments to `wind_set` and friends

**Symptom**: a window title of garbage, or a compile error passing a `char *`
as `short`.

**Cause**: gemlib's `wind_set` takes four `short`s (`sdk/gemlib/gemlib-0.44.0/mt_gem.h:2456`); the
m68k idiom of splitting a 32-bit pointer across two WORDs does not carry an
8-byte pointer.

**Do**: use the pointer-typed variants `wind_set_str` / `wind_set_ptr` /
`wind_set_ptr_int` / `wind_set_grect` (`sdk/gemlib/gemlib-0.44.0/mt_gem.h:2468,2470,2474-2475`).
Under A81 these bindings pass the pointer through `addr_in[0]` — the
*native* NewTOS convention, the same slot every other AES pointer-class
argument uses (`docs/api/aes/window-graphics.md`, "Pointers in
`addr_in[]`") — with the extra WORD (`wind_set_ptr_int`'s `W3`, e.g.
`WF_NEWDESK`'s desktop-root index) in `int_in[2]`, not packed alongside the
pointer (overlay in `sdk/gemlib/overlay/`; not yet landed as of this note —
the pristine `gemlib-0.44.0/a_wind_set_*.c` still use the older packed
form below). If you hand-pack an 8-byte LP64 pointer yourself into
`int_in[2..5]` instead (the kernel still accepts this — the *packed* form,
`control[3] == 0`, `tos-api/src/aes/mod.rs::intin_ptr`), the extra/root word
belongs at `int_in[6]` — the word right *after* the 4-word pointer — not
`int_in[4]`: `int_in[4]` is the pointer's own upper half (bits 32..47) at
that width, and writing a real value there corrupts the pointer.

That corruption is not hypothetical: the pristine `a_wind_set_ptr_int.c`
(`:27-28`) does exactly this — `*(void**)(aes_intin+2) = v; aes_intin[4] =
W3;` — so any non-zero `W3` (e.g. a non-zero `WF_NEWDESK` root) silently
overwrites the pointer's own bits 32..47 (`TASK/A81.md` measures a tree at
`0x0000_0001_0177_E4C0` reassembling as `0x0000_0002_0177_E4C0` for root 2).
`a_wind_set_ptr.c` (`:26-27`, two single-word `= 0` stores zeroing
`int_in[4..5]`, the pointer's own upper half) and `a_wind_set_str.c` (`:26`,
one 8-byte `*(const char**)(aes_intin+4) = 0` store zeroing the wider
`int_in[4..7]`) overlap the same pointer bytes but with a hardcoded `0`, so
they are silently harmless — not because the overlap itself is benign, but
because the value written happens to match what the pointer's upper half
already is for any EL0 address on the current stage-1 boot tables
(identity-mapped only in `0..4 GiB`, `kernel/src/arch/aarch64/el0.rs:16-18`).
All three are fixed by the A81 overlay move to `addr_in[0]` + `int_in[2]`.

### B11. Bit-fields, `int` alignment, and big-endian word tricks

**Symptom**: a config loader reads a 2-byte flag word and gets the wrong
half; a bit-set routine answers 25 where 1 is expected.

**Cause**: `unsigned int x : 1` bit-fields make a 4-byte struct on AArch64
(alignment 4) where m68k gcc made 2 (`sdk/overlays/teradesk/window.h:23-34`);
code that addresses the same bits both byte-wise and as hard-coded 32-bit
big-endian words disagrees with itself on LP64/little-endian
(`sdk/overlays/qed/set.c:7-50`); m68k `ror`/`swap` inline asm does not
assemble (`sdk/libc/overlay/include/bits/byteswap.h:47-57`); bit-fields laid
over an `int` in big-endian order land in the wrong bits — mintlib's
`struct __wait` made `WEXITSTATUS` 0 for every status until the overlay
`sdk/libc/overlay/include/sys/wait.h` declared them in little-endian order
([07 §9](07-mintlib-limits.md#9-syswaith-status-macros-taskport-waith)).

**Do**: base bit-fields on `unsigned short` where the app needs 2 bytes;
write byte-addressed code; use `__builtin_bswap32`. These are legitimate
overlay contents (they are LP64/endian corrections, not app logic changes).

### B12. `long double` is 128-bit, and the compiler runtime is compiler-rt

**Symptom**: undefined symbols `__lttf2`, `__addtf3`, `__muldc3`, … at link
time — with a link line that does not go through `newtos-cc`.

**Cause**: AAPCS64 `long double` is IEEE binary128; the CPU has no quad
arithmetic, so clang emits soft-float libcalls for every `long double`
operation (`__addtf3`, `__multf3`, `__divtf3`, `__lttf2`, `__extendsftf2`,
`__trunctfdf2`, …), and calls `__mulsc3`/`__muldc3`/`__divsc3`/`__divdc3`
for C99 complex `*` and `/`. These live in the compiler runtime.

**Do**: since TASK/SDK01 E the SDK has one: LLVM compiler-rt 21.1.8's
builtins, built by `sdk/compiler-rt/build-compiler-rt.sh` into
`sdk/compiler-rt/libclang_rt.builtins.a`; `newtos-cc` (and so `newtos.mk`)
links it last, always. A hand-written link line adds that archive after the
libraries. Since TASK/SDK01 J `long double` is binary128 end to end:
arithmetic (soft-float, so slow), `printf`/`scanf` with `%Lf %Le %Lg %La`,
`strtold`, `qecvt`/`qfcvt`/`qgcvt`, and the libm's `…l` functions (OpenLibm's
`sdk/libm/openlibm-0.8.8/ld128/` set: `expl`, `logl`, `powl`, `sinl`, `sqrtl`, … and `scalbln`/
`scalblnf`/`scalblnl`) — [07 §4, §11](07-mintlib-limits.md#11-functions-that-do-not-link-measured-per-symbol-tasksdk01-a).
Until then libc was built with `-D__NO_LONG_DOUBLE_MATH` and `printf("%Lf")`
read a `double` where a binary128 had been passed. Plain `printf` needs no
compiler runtime (a hand-written link of a program without `long double`
still links without the archive); `libc.a` needs it only in
`qecvt`/`qfcvt`/`qgcvt(_r)` and `printf_size`. Limits, measured
(`sdk/libc/tests/ldbl-host.sh`): compiler-rt honours the `fesetround()` mode
in `+`/`-` only — `*`, `/` and conversions always round to nearest — and
raises `FE_INEXACT` only there, so OpenLibm's `sqrtl` (which decides the last
bit with `fetestexcept(FE_INEXACT)` after a division) is within 1 ulp but not
always correctly rounded; the other `…l` functions are within 2 ulp (measured
maxima 0.5–1.7). `__extenddftf2` still comes from
`sdk/libc/newtos-arch/softfloat.c` (`libc.a` comes first in the group).

### B13. clang drops cross-translation-unit `alias` attributes

**Symptom**: undefined `labs` although mintlib "defines" it.

**Cause**: `sdk/libc/mintlib/mintlib/labs.c` was `strong_alias(abs, labs)` with `abs` in another
file; clang silently drops an alias to a symbol not defined in the same TU
(`sdk/libc/overlay/stdlib/labs.c:1-19`).

**Do**: give the function a real body in an overlay, as `labs.c` does. Expect
the same for any other `strong_alias`/`weak_alias` across files.

### B14. Console output is not the desktop

**Symptom**: `printf` output vanishes behind the desktop; a `.NAP` that
prints scrolls text over its own window.

**Cause**: a `.NCP` gets the VT52 text screen and the "press any key" pause
(`programs/desktop/src/main.rs:482-497`, `:610`); a `.NAP` runs on the GEM screen with no
console handoff. `AUTO` programs run **before** the AES; their output is
painted over when the desktop comes up (`kernel/src/launch.rs:216-226`,
`tools/make-sd-image.sh:184-187`).

**Do**: choose `.NCP` for text tools; in a `.NAP` write diagnostics to a file
(the `Fopen`/`Fseek`/`Fwrite`/`Fclose`-per-line pattern of
`sdk/apps/gemtest/gemtest.c:59-73` survives a hang) or to the serial console
via `Cconws` while debugging in QEMU.

### B15. A libc `malloc` that suddenly fails

**Symptom**: `malloc` returns `NULL` (`ENOMEM`) although the machine has
hundreds of megabytes.

**Cause** (until ARCH03 E9, 2026-10-05): mintlib's heap was the in-TPA arena
between `_end` and the stack reserve inside a fixed 4-MiB TPA, with no
fall-back to `Malloc`. Since E9 the TPA is the largest free block and the
start-up code runs mintlib's own model — malloc through GEMDOS `Malloc`
(see A3) — so a failing `malloc` now means the pool (or the BDOS's 256 memory
descriptors, one per `Malloc` hunk) is exhausted.

**Do**: check `Malloc(-1)`; for many small long-lived allocations prefer one
large block.

### B16. gemlib bindings that split a `long` with a pointer cast

**Symptom**: `evnt_timer(150)` sleeps for hours; TeraDesk's *File → Quit*
never shows its dialog while the mouse keeps moving (ticket `TASK/H03.md`,
2026-09-17).

**Cause**: gemlib 0.44.0 was written for a 32-bit big-endian `long`. Some
bindings split a `long` argument into the two AES/VDI words by casting its
address: `((unsigned short *)&interval)[0]` is the high word on m68k and the
**low** word on AArch64, so the pair lands in `int_in` swapped
(`sdk/gemlib/gemlib-0.44.0/a_evnt_timer.c:27-31`, pristine). The kernel reads
`int_in[0]` as the low word exactly like EmuTOS (`tos-api/src/aes/event.rs:673`,
`references/emutos/aes/gemsuper.c:142`), so `150` becomes `150 << 16` ms.
Bindings that use arithmetic (`interval`, `interval >> 16`,
`sdk/gemlib/gemlib-0.44.0/a_evnt_multi.c:91-92`) are correct on every width and byte order.

**Do**: fix the binding in `sdk/gemlib/overlay/` with the arithmetic form
(`sdk/gemlib/overlay/a_evnt_timer.c` is the model) and guard it with a host
test that captures the parameter block through a fake `aes()`/`vdi()`
(`sdk/gemlib/tests/evnt_timer_words.c`, built with `-D__NEWTOS__`).

**Resolved 2026-09-21 (T04) — the contract and the header fix.** The rule
(`docs/COMPATIBILITY.md`, "A `LONG` in `int_in`/`int_out`/`ptsin`/
`ptsout`"): a `long` inside a VDI/AES WORD array is **32 bits over two
WORDs, low word first, natively in place** — `int_x[n]` bits 0..15,
`int_x[n+1]` bits 16..31, what EmuTOS's `MAKE_ULONG(int_in[1], int_in[0])`
and `*(LONG *)&int_out[1]` both leave on a little-endian machine — and a
**pointer** packed into such an array occupies the **caller's pointer
width** (`N_PTRINTS` WORDs, A81). Three consequences for an LP64 binding:

* Punning a `long` argument as the intin array (`(short *)&point`,
  `n_vq_ctab.c`, `n_vq_dflt_ctab.c`, `n_vst_setsize32.c`, `n_vst_arbpt32.c`)
  is **correct** on LP64/LE — the bytes at `&point` start with the low word.
  Only the explicit swap of `a_evnt_timer.c` (`i[1]`, `i[0]`) was wrong.
* The `*_intin_long`/`*_intout_long`/`*_ptsin_long`/`*_ptsout_long`
  accessors must be **32-bit** accesses. The pristine `*(long *)(array + n)`
  is an 8-byte access on LP64: its store spilled over the two WORDs after the
  LONG (past exactly sized local arrays in `v_create_ctab`,
  `vst_track_offset`, `v_value2color`, `vst_arbpt32` — at `-O2` the binding's
  own `control[0]` was overwritten and the VDI received opcode 0), its load
  took the two WORDs after the LONG as the upper half. The overlay headers
  `sdk/gemlib/overlay/gem_aesP.h`/`gem_vdiP.h` now define them on
  `gem_wire_long` (`int`, `aligned(2)`, `may_alias`): one `str w`/`ldr w` at
  `&array[n]`, sign-extended on the way out. All 33 + 36 sites are fixed by
  the header; no per-file overlay.
* A pointer packed into intin must be **declared at its width**: overlays
  `n_v_delete_ctab.c`, `n_v_delete_itab.c`, `n_vqt_fontheader.c`,
  `u_vqt_fontheader.c` (`N_PTRINTS`), `a_wind_get.c`, `a_wind_sget.c`
  (`2 + N_PTRINTS`). And a LONG must not be read at pointer width:
  `a_wind_get_ptr.c` reads `WF_SCREEN` (two 32-bit LONGs: address, length)
  as a LONG — the pristine 8-byte read returned `address | length << 32`.

Host guards: `sdk/gemlib/tests/intin_long_words.c` (the four bindings + the
`WF_SCREEN` read, under AddressSanitizer; the pristine headers fail) and
`sdk/gemlib/tests/ptr_word_count.c` (the six count overlays). Still open,
own ticket: the `vex_*` vector exchange in `CONTRL[7..]`/`[9..]`
(`TASK/T08.md`). Note for app authors: cflib's `purec.c` (an `aes_intout`
reader of the same 8-byte shape) is `__PUREC__`-only and not compiled on
NewTOS — cflib's `wind_get` is gemlib's `mt_wind_get`; a `union { long
size; struct { _WORD high, low; } words; }` around `wind_get(0, WF_SCREEN,
…)` (TeraDesk `resource.c:233-241`) gets the right low 32 bits and leaves
the upper 32 bits of the 8-byte `long` uninitialised — compare through a
32-bit type.

### B17. Small contract differences a port may notice

* `Cconws` returns the number of bytes written; EmuTOS's is `void`
  (documented divergence D-009, `sdk/include/newtos.h:158-162`).
* `Sversion()` measured `0x3000` in the run of 2026-09-17
  (`docs/sdk/03-first-programs.md`); `os_version` in the OSHEADER is
  `0x0404` (`docs/STATUS.md`, B1). Which GEMDOS version a TOS-4-expecting app
  should see is recorded as "verify value" in the project memory —
  **unverified** here beyond the measurement.
* `_v_bas_ad` (`$44E`) is **not** populated by `populate_sysvars`; a program
  that reads the screen address from `$44E` instead of `Logbase()` gets 0
  (`docs/STATUS.md` Recent Decisions 2026-09-05, "Nebenbefund").
* `shel_envrn` values point into a kernel static; with an empty environment
  the NULL path is taken (`docs/analysis/qed-launch-findings-log.md`, note
  from #7). Latent until a non-empty environment exists.

### B18. Pointers inside AES messages (`appl_write`, AV protocol, BubbleGEM)

(Added 2026-10-21, TASK/SDK01 H — the HypView port met this and found the
answer only in the WinDom and zView chapters.)

**Symptom**: a message carrying a string or tree pointer (`VA_START`,
`AV_*`/`VA_*`, `MN_SELECTED`'s tree in AES 4, BubbleGEM's `BUBBLEGEM_SHOW`)
arrives as a wild pointer, or a field after the pointer is overwritten.

**Cause**: a GEM message is eight 16-bit WORDs. The classic idiom
`*(char **)&msg[3] = p` stores an 8-byte pointer on LP64, over `msg[3..6]`,
and `*(char **)&msg[6]` reads two WORDs past the end of the buffer; a partner
that uses the classic 32-bit layout (two WORDs) reads something else.

**Do**: carry the pointer as a 32-bit value in two WORDs, high WORD first —
`msg[n] = (short)((unsigned long)p >> 16); msg[n + 1] = (short)(unsigned long)p;`
and `p = (char *)(((unsigned long)(unsigned short)msg[n] << 16) | (unsigned short)msg[n + 1]);`.
That is the convention of the WinDom and zView patches
([12 §3](12-windom.md#3-overlays-10-files-each-with-a-header-comment-upstream-line-change-reason): `evnt_windom.c`, `av_init.c`, `shel_write.c`;
[13](13-zview.md) `av_prot.c`), and it holds every NewTOS user address
because all of them are below 2 GiB ([A2](#a2-the-process-address-space)).
No SDK header provides a helper yet (04 lists this as a candidate for a
shared header). The HypView port leaves its ~25 such sites unpatched — see
`sdk/apps/build-hypview.sh` and TASK/SDK01 H.

### B19. m68k binary file formats read straight into structs

(Added 2026-10-21, TASK/SDK01 H.)

**Symptom**: a document or picture format the program reads with `Fread`
into a `struct` is rejected ("wrong format", negative sizes) or shows
garbage, although the file is fine on an Atari.

**Cause**: the file holds the m68k memory image of that struct: big-endian
WORDs and LONGs, `long` = 4 bytes, and m68k gcc's **2-byte** alignment for
`long` and pointers. On AArch64 the same declaration is little-endian, `long`
is 8 bytes and aligned to 8 — HypView's `INDEX_ENTRY` (`char, char, unsigned
long, …`) has its `seek_offset` at byte 2 on disk and at byte 8 in memory,
and `sizeof(HYP_HEADER)` is 12 on disk, 24 in memory. B1 is the same problem
for kernel structures; here the "writer" is a file.

**Do**: read the record into a byte buffer and decode it field by field
(`(p[0] << 8) | p[1]` …), into the native struct; replace every
`sizeof(struct)` used as a **file** offset or read length by the on-disk size.
Bitmaps from such files are big-endian words too (see the zView codec
patches, [13](13-zview.md)). Examples: `sdk/overlays/hypview/hyp/hyp.c`
(header, index table, extra headers), `sdk/overlays/hypview/hyp/prepare.c`
(`HYP_PICTURE`).

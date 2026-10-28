# MEMORY — NewTOS

> **Project memory: knowledge that must persist across sessions.**
>
> Unlike `STATUS.md` (which tracks "what's happening now"), this file
> records **durable
> project knowledge**: ADRs with their full rationale, hard-won bug
> solutions, rejected alternatives with reasoning, hardware quirks
> discovered during implementation, and any context that would otherwise
> be lost between sessions or human contributors months apart.
>
> **Read this file at the start of every working session.**
> Entries are not rewritten. A correction or a later decision is appended
> inside the entry as a dated amendment or addendum, and the entry's
> status line is kept current. A decision that replaces another one gets
> a new entry; the old entry stays, with its status line pointing to the
> new one. Text that is only history moves to `docs/archive/`.

---

## How to use this file

- **ADRs**: Architectural Decision Records. Numbered, dated, with full
  rationale. An ADR's original text stays as written. What changes later
  is appended inside it as a dated amendment or addendum, and its
  **Status** line always says what holds today — "Accepted", "Accepted,
  amended YYYY-MM-DD", "Superseded by ADR-NNN", … A decision that
  replaces an ADR gets a new ADR that references the old one; the old ADR
  stays here with the status "Superseded by ADR-NNN".
- **Quirks**: Bugs, hardware oddities, surprising spec interpretations
  that bit us during development. One per entry, with reproduction info
  and resolution.
- **Lessons**: Process / workflow / tooling lessons. "We learned the
  hard way that …".
- **Rejected**: Alternatives we considered and rejected, with the reason.
  Prevents re-litigating the same decisions.
- **Commit hashes**: history was rewritten on 2026-10-19
  (`git filter-repo`). A hash in an entry written before that date is an
  old hash; translate it with `tools/oldhash.sh <prefix>` (map:
  `docs/commit-map.txt`). New text cites current hashes.
- **Archive**: `docs/archive/memory-2026-10-26.md` holds text moved out of
  this file (never-written ADR plans, unobserved example quirks, replaced
  status lines). It is history, not evidence.

---

## ADRs (Architectural Decision Records)

ADRs are numbered globally, prefix `ADR-NNN`. They follow the
"context / decision / consequences / alternatives" structure; the
template is at the end of this section.

**Numbering.** The first ADR written is ADR-010 (initial commit
`265a43c1`, 2026-06-16). **Numbers 001 to 009 were never written.** Two
reservation lists for low numbers existed and disagreed: this file's
"Anticipated further ADRs" (001 license, 002 EmuTOS configuration
variant, 003 EmuTOS pin, 004 native PRG format, 005 `Mxalloc` modes,
006 VT52 scope; now in `docs/archive/memory-2026-10-26.md`) and
`docs/ARCHITECTURE.md` "Key ADRs (planned)" (001 PRG format, 002
license, 003 `Mxalloc`, 004 VT52, 005 m68k JIT, 006 USB-HID vs UART,
007 long file names). Neither list is a reservation any more; a new ADR
takes the next free number above the highest one here. Where those
topics ended up: license — GPLv2+ (`LICENSE`, `aec5b730`), no ADR;
EmuTOS pin — `docs/analysis/ORACLE.md` §1; EmuTOS configuration — the
Falcon030/TOS4 profile (`docs/api/_emutos-config.md`,
`docs/analysis/aes-audit/emutos-config-profile.md`); native executable
format — ADR-018 and ADR-028 (position-independent ELF); `Mxalloc`
modes — ADR-021, ADR-023, ADR-027, ADR-028; input — ADR-015; m68k JIT —
REJECTED-003; VT52 scope and long file names — no ADR. An "ADR-Supexec"
is announced (ADR-012, `docs/api/xbios/26-supexec.md`) but has not been
written. **ADR-025** is the MD supply (below); the ARCH03 draft that
also called itself "ADR-025 Proposed"
(`docs/analysis/arch03-implementation-plan.md` §1) is ADR-028.

## ADR-010: Single shared user address space (identity-mapped DRAM pools)

**Date:** 2026-06-16
**Status:** **Superseded by ADR-028** (2026-10-04, implemented with ARCH03;
the VA = PA decision lives on there; this ADR's path — one `TTBR0`,
installed once and never switched, no protection — is replaced by one
`TTBR0` root per address space with per-process rights).
**Phase:** PHASE-06 (process model).

### Context

`Pexec(PE_LOAD)` must return a PD pointer (as `LONG` = i32) that the
parent holds and passes back to `Pexec(PE_GO)`, and that source-ported
programs dereference (the classic shell reads `p_tlen`/`p_dlen`/`p_blen`
to size the TPA before `Mshrink`). The Phase-05/06 model gave each
process its **own** `TTBR0_EL1` with its TPA at `USER_VA_BASE = 0`, so
every basepage VA is `0` — not a unique, parent-dereferenceable, or
LONG-representable pointer. Per-process TTBR0 also forces a TTBR0 swap
on every `Pexec`/`Pterm`, which is what triggers QUIRK-001 (the QEMU
cortex-a72 TCG wedge), and per-process ASIDs (the "proper" fix) wedge
QEMU even harder (QUIRK-001).

Classic TOS has exactly **one** address space: programs are loaded into
a `Malloc`'d TPA and run *at that block's address* (no MMU; phys = VA).
PD pointers are therefore real, shared, dereferenceable pointers.

### Decision

NewTOS uses a **single shared user address space** for all processes,
realised by **identity-mapping the DRAM user/pool region (VA = PA) into
one shared `TTBR0_EL1`** that is installed once and never swapped. Each
process's TPA is a BDOS-pool block; the program runs at `VA = PA` of
that block; `p_lowtpa` / basepage VA = the block address (a DRAM phys
~`0x4000_0000`+, which fits a positive `i32`). The kernel stays in the
high half (`TTBR1_EL1`), untouched.

### Rationale

- **EmuTOS fidelity / source-porting.** This is precisely the TOS
  model: TPA = allocated block, phys = VA, PD pointers are real shared
  pointers. The full `Pexec(PE_LOAD)`→read PD→`Mshrink`→`Pexec(PE_GO)`
  shell idiom works, and ported C programs that dereference their
  basepage / `Malloc` results see consistent, dereferenceable pointers.
- **Unifies `Malloc`.** `Malloc`/`Mxalloc` already return pool DRAM
  addresses (~`0x4000_0000`+); under identity mapping those *are* valid
  user VAs, removing today's inconsistency (pool returns phys, code
  runs at VA 0).
- **Dissolves QUIRK-001.** One shared user TTBR0 that is never swapped
  ⇒ no TTBR0 switch between VA-aliased processes ⇒ no TCG wedge ⇒ the
  interim Device-write barrier and the whole boot-restore/install dance
  are deleted. ASIDs become unnecessary (single ASID 0 is correct).
- **Matches the cooperative model.** Only one process runs at a time
  (cooperative, `Pexec` blocks until child `Pterm`); inter-process MMU
  isolation buys nothing here and TOS programs never expected it.

### Consequences

- Positive: faithful PD/pointer semantics; `Malloc` consistency;
  QUIRK-001 + the barrier + per-process page-table machinery
  (`mmu::init_slot_tables`/`map_page`/`install_ttbr0`, `__USER_TABLES`,
  per-slot `l0_pa`, the `restore_boot_ttbr0` dance) all go away.
- Negative: no MMU isolation between user processes (a buggy process
  can corrupt another's TPA). Acceptable — classic TOS has none either;
  revisit only if/when preemptive multitasking is ever in scope.
- Negative: the loader must allocate the TPA from a BDOS pool (not the
  static `__PROCESS_USER_PAGES` arena) and load at `VA = PA`; `Mshrink`
  becomes meaningful (real TPA sizing) instead of a no-op.
- Neutral: user region is mapped `User-RWX` for now (no W^X);
  text/data/bss permission split is a later refinement.

### Alternatives considered

- **Opaque slot-handle under per-process TTBR0** (PE_LOAD returns
  `basepage_pa`, parent treats it opaquely): keeps isolation and is
  less work, but the parent cannot *dereference* the PD, so the real
  shell idiom and ported programs that read the PD break. Rejected for
  fidelity.
- **Distinct per-slot TPA VA + map child basepage into parent's TTBR0:**
  half-measure that still carries per-process TTBR0 swaps (and thus
  QUIRK-001) and the 64-bit-VA-vs-LONG problem. Rejected.
- **Per-process ASIDs:** wedges QEMU worse (QUIRK-001); does not by
  itself solve the LONG-pointer problem. Rejected. *(2026-10-06: rejected
  as a fix for the address-space model only; since ARCH03 stage 4 they
  exist as an optional switch-cost optimisation, `asid=on`, see QUIRK-001
  update.)*

### Implementation note — Stage-0 prerequisite (discovered 2026-06-16)

A first attempt to install the shared user TTBR0 at boot (Stage 1)
**broke the boot**, surfacing a prerequisite that must land first:

- The kernel currently relies on the **low-half identity mapping**
  that the Stage-1 boot tables leave in `TTBR0_EL1` after `reinstall`.
  Concretely, the kernel heap dereferences raw **physical** addresses
  as pointers — `heap.rs`: `let start = frame.pa() as usize; … start
  as *mut Header` — i.e. it touches DRAM through low-half VAs
  (`VA = PA`), served by those boot tables (AP=00, EL1-only, no PXN).
- Installing the user table in `TTBR0` (DRAM-only, 2-MiB blocks,
  `PXN=1`, and ultimately `AP=01`/EL0) removes that EL1-friendly
  low-half mapping, so the next `frame.pa()` dereference in
  `heap::init` dies. (`AP=00` did not help — the remaining deltas are
  PXN / DRAM-only coverage; either way the kernel must not depend on
  TTBR0 for its own data.) Also observed: `tlbi vmalle1` on the
  install path wedges QEMU (QUIRK-001-adjacent) — the install must be
  `msr`+`isb` only.

**Stage 0 (prerequisite) — DONE (commit `55b4390`).** Audit found the
kernel heap was the *only* low-half-deref site (frames manage PAs as
values, BDOS pools keep MDs in a static high-half arena and treat
`m_start` as values, the ELF loader writes via the static high-half
arena). One-line fix: `heap::init` addresses its arena through
`pa + VA_OFFSET`. Boot stays green.

**Stage 1 — BLOCKED on QEMU (attempted 2026-06-16, reverted).**
With Stage 0 in place, installing the shared user table in `TTBR0` at
boot still breaks the boot — but now the failure is a **hard hang on
the first kernel Normal-memory store after the install** (verified: the
heap header write to high-half `0xffff000041000000` hangs, no
exception). Diagnosis: under ADR-010 a DRAM physical page is **aliased
with mismatched permission attributes** — TTBR1 high-half (EL1 Normal)
*and* TTBR0 low-half (EL0 Normal + PXN). Real ARMv8 permits this
(memory *type* + shareability match; AP/PXN differ), but QEMU 11.0.0's
cortex-a72 TCG wedges on a store to such a doubly-mapped PA — the same
class of TCG fragility as QUIRK-001 (TTBR0 swap) and the ASID attempt.
`tlbi vmalle1` on the install path wedges even earlier.

**Conclusion:** ADR-010 is correct for **real hardware** but cannot be
developed or validated on this QEMU model — three independent TCG walls
(TTBR0 swap = QUIRK-001, ASID-tagged TTBR0, and now kernel/user DRAM
aliasing) all block it. Options: (a) develop/validate ADR-010 on real
RPi4 hardware; (b) keep the working per-process-TTBR0 + barrier model
for QEMU and implement `Pexec` `PE_LOAD`/`PE_GO` with an opaque
slot-handle (the parent does not dereference the PD — acceptable until
a real FS-backed loader + dereferenceable PD land in Phase 09). The
Stage-0 heap improvement is kept regardless.

### References

- EmuTOS `bdos/proc.c` (`alloc_tpa`, `proc_go`), `docs/api/gemdos/process/4b-pexec.md`
- QUIRK-001 (this file); STATUS Recent-Decisions 2026-05-03/04 (superseded)
- ARM ARM D8 (TTBR0/TTBR1 translation regimes), D5 (PAN)

## ADR-011: Per-device VDI physical workstation (multi-screen-ready)

**Date:** 2026-06-16
**Status:** Accepted, **not implemented** (checked 2026-10-26): there is no
screen-device registry — `Host::raster()` returns the one raster backend
(`tos-api/src/host.rs:173`), and the palette is device-global
(`kernel/src/vdi_backend.rs:144-153`).
**Phase:** PHASE-00b (VDI spec) / PHASE-12 (VDI impl) — design intent
locked during spec extraction, before any VDI code exists.

### Context

EmuTOS implements the VDI screen device with **global** state: a single
`phys_work` physical workstation plus Line-A globals that describe *the*
one screen — `v_bas_ad` (screen base), `DEV_TAB`/`xres`/`yres`
(geometry), the palette. Every virtual workstation (`v_opnvwk`) shares
that one screen. The VDI *API*, however, was designed for **multiple
devices**: `v_opnwk(device_id)` opens one physical workstation per
device, `contrl[6]` (the handle) selects it, and `v_opnvwk` opens a
virtual workstation *on* a physical device. The single-screen limit is
purely an implementation choice in EmuTOS, not in the API.

Raspberry Pi 4 has **two HDMI outputs**; QEMU can expose multiple
framebuffer devices (ramfb / virtio-gpu). NewTOS already models the
framebuffer as a per-instance HAL type (`hal::framebuffer::Framebuffer`
is a trait; `RamFb` is a struct — multiple instances are natural).

### Decision

NewTOS implements the VDI **physical workstation as a per-device
struct**, not global Line-A state. Each physical (screen) workstation
owns its own `Framebuffer` instance, geometry/`DEV_TAB`, and palette,
held in a small **screen-device registry** instead of a single
`phys_work` global. Each virtual workstation (`Vwk`) carries a **link
to its physical device** (which screen it draws on); the Line-A
primitives draw into *that device's* framebuffer rather than a global
`v_bas_ad`. `v_opnwk(device_id)` selects the screen device.

### Rationale

- **API stays bit-exact.** The VDI handle model already supports
  multiple physical devices, so no compatibility is sacrificed — only
  EmuTOS's *global screen state* is replaced with *per-device state*.
- **Cheapest now.** Baking the per-device shape into the Phase-12
  design from the start avoids a disruptive globals→per-device refactor
  later. Recording it during spec extraction costs nothing.
- **Real hardware wants it.** The Pi 4's two HDMI ports make multi-
  screen physically reachable, and NewTOS's own software could use it.

### Consequences

- Positive: multi-screen at the VDI level is reachable without an API
  break; the framebuffer HAL already fits (per-instance).
- Negative / scope limit: **AES/desktop** multi-monitor (windows
  spanning screens) is a *much* larger, separate undertaking — classic
  GEM is single-screen. ADR-011 covers the **VDI device layer only**.
- Neutral: classic single-screen TOS programs are unaffected (they use
  one workstation on the default screen device); they neither benefit
  from nor break under the per-device model.

### Alternatives considered

- **Mirror EmuTOS's globals** (single `phys_work` + global Line-A
  vars): simplest, but hardcodes single-screen and would force a
  globals→per-device refactor to ever add a second display. Rejected —
  the per-device shape is nearly free now and expensive later.

### References

- `docs/api/vdi/_dispatcher.md`, `docs/api/vdi/workstation/100-v_opnvwk.md`
- EmuTOS `vdi/vdi_control.c` (`phys_work`, `vwk_ptr[]`, `CUR_WORK`),
  `bios/lineavars.S` (`DEV_TAB`), `tosvars.ld` (`v_bas_ad`, sysvar
  `0x44e`) *(corrected 2026-10-26: was "vdi/lineavars.S", which does
  not exist)*
- `kernel/src/hal/framebuffer.rs` (per-instance framebuffer)

---

## ADR-012: Cookie-jar access mechanism on NewTOS

**Date:** 2026-06-20
**Status:** **Accepted** (2026-06-20); amended 2026-07-24 (B1, below) and
2026-10-26 (Consequences checked against the code, see the note there).
**Phase:** PHASE-08 (cookie jar).
**Amended 2026-07-24 (B1):** the "native apps never touch low memory" premise
in Context §1 below is **superseded**. Native EL0 apps now get a **per-process
`$5A0` `_p_cookies`** pointing at a mirror of the kernel-owned jar, plus
`Setexc($5A0/4,-1)` support, so the classic linear-scan and the cookie idiom
work for 1:1-ported TOS apps (ADR-019) — see `docs/analysis/bios-audit/
b1-lowmem-feasibility.md`, commit `3d1c1fe`. The kernel-owned jar (the Decision
below) **remains the source of truth**; B1 only adds the classic-address
mirror. `Supexec` is no longer needed to reach it — `$5A0` lives in the app's
own EL0 low memory (see also ADR-Supexec: still deferred, but not a blocker for
the cookie jar anymore).

### Context

The TOS cookie jar is a feature registry: an array of `{tag, value}`
entries (terminated by `{0, capacity}`) that programs scan to detect
system features (`_MCH`, `_IDT`, `_FLK`, …). Full data-model extraction:
[`docs/api/bios/_cookie-jar.md`](api/bios/_cookie-jar.md).

There is **no trap** for it. Classic access is: read the system-variable
pointer at **`$5A0`** (`_p_cookies`) and linear-scan; because `$5A0` is
supervisor-only low memory, applications elevate via **`Supexec`**
(XBIOS 0x26). NewTOS blocks this path **twice**:

1. `$5A0` is not mapped — Atari low-memory sysvars are not given AArch64
   addresses (same call as the `$420`/`$432` vars: direct poke is out of
   scope, m68k-emulator-only). **[Superseded 2026-07-24 by B1 — see the Amended
   note above: native EL0 apps now get a per-process `$5A0` + sysvar region.]**
2. `Supexec` is itself deferred (Open-ADR *ADR-Supexec*).

So NewTOS needs its **own** cookie set (the still-meaningful tags + a
NewTOS identity cookie; the Shifter/PSG/FDC/FPU tags are meaningless) and
a **native access mechanism** replacing `Supexec` + `$5A0`.

### Decision

**A kernel-owned cookie jar, read back through a function (value-query),
never by direct memory access.**

- **Storage.** An init routine allocates the jar from the **kernel heap**
  (available since Phase 03) and stores the pointer in a **kernel-internal
  global** (the NewTOS analogue of EmuTOS' `p_cookies`, but *not* the
  `$5A0` sysvar). The jar lives in high-half (TTBR1) kernel memory; its
  address is arbitrary and irrelevant.
  *Implementation note (2026-06-20):* `tos-api` is `#![no_std]` **without
  `alloc`**, so the jar is a **fixed-capacity static array** (32 slots,
  `tos-api/src/cookie.rs`) rather than a heap allocation — this is also
  closer to FreeMiNT's own fixed `dflt_jar[]`. Growing past the cap would
  need the kernel allocator; deferred until a real need appears.
- **Access.** A function returns the **content**, not a pointer:
  `Getcookie(tag) -> (found, value)`. The kernel scans its own jar and
  returns only the value. Because the jar is kernel-heap memory (EL1-only,
  not mapped at EL0), a raw pointer would be unusable to a user program
  anyway — so value-return is the *forced* and cleanest choice. A
  companion `Setcookie(tag, value)` adds/updates an entry (the kernel
  owns and serialises the jar). Optional enumeration (`Getcookies(buf)`
  copy-out) only if a real program needs to list all cookies; deferred
  until then.
- **No direct access.** No `$5A0`, no other fixed sysvar address, no
  `Supexec` — none of the three NewTOS blockers is touched.
- **Porting.** Ported C programs are adapted at port time anyway; a tiny
  shim maps the classic `getcookie()`/`$5A0` idiom onto `Getcookie`.

**Syscall placement (decided 2026-06-20):** **FreeMiNT `Ssystem`** —
GEMDOS `0x154`, `Ssystem(WORD mode, LONG arg1, LONG arg2)`, with modes
`S_GETCOOKIE`(8) / `S_SETCOOKIE`(9) / `S_DELCOOKIE`(26). NewTOS adopts the
MiNT ABI verbatim so the MiNT/GEM software base runs (see
[[freemint-extension-direction]]); FreeMiNT is now a sanctioned source
(`references/freemint` @ `f906416`). ABI + cookie semantics extracted in
[`docs/api/gemdos/154-ssystem.md`](api/gemdos/154-ssystem.md).

### Rationale

The user's framing settled it: the kernel must allocate the jar, so store
the pointer kernel-side and hand back *content* through a function instead
of exposing the address. This matches the project's altitude (TOS **API**
compatibility, not memory-map emulation; CLAUDE.md non-goals), needs no
user-space mapping and no `Supexec`, and is implementable now. Apps being
adapted during porting removes the only downside (loss of literal `$5A0`
bit-compatibility), which a one-line shim covers.

### Consequences

- The data model (`_cookie-jar.md`) is access-agnostic and unchanged;
  this ADR fixes the **access path** (value-query function) + **storage**
  (kernel-heap, kernel-held pointer).
- NewTOS defines its own cookie content: keep `_MCH` (NewTOS machine id),
  `_IDT`, `_FLK`, `_AKP`; **drop** `_VDO`/`_SND`/`_FDC`/`_FPU` (Shifter/
  PSG/floppy/m68k-FPU — meaningless); add an `NTOS`/`_NEW` identity +
  version cookie so ported programs can detect NewTOS.
- `Getcookie`/`Setcookie` are NewTOS extensions; the chosen function id(s)
  must be documented as reserved.

*Amendment 2026-10-26 (checked against the code).* Two of the
consequences above did not come true as written. The seeded set is
`NTOS` (identity + version), `_MCH`, `_IDT`, `_AKP` and `EdDI`
(`tos-api/src/cookie.rs:221-227`): `_FLK` is **not** set, `EdDI` is.
And no NewTOS-own `Getcookie`/`Setcookie` function ids were reserved:
the Syscall placement decision above uses FreeMiNT's `Ssystem` modes
`S_GETCOOKIE`/`S_SETCOOKIE`/`S_DELCOOKIE`, which replaces that bullet.

*Amendment 2026-10-28 (user decision on TASK/ARCH05).* **`_MCH` =
`0xFFFFFFFF`**, EmuTOS' value for "no Atari hardware" (`MCH_NOHARD`,
`references/emutos/bios/machine.c:400-401`); **`NTOS`** is the NewTOS
identifier (no collision in the cookie registry: only `nTOS` exists, case
matters). The provisional `0x00050000` was the registered **ARAnyM** id
(`references/emutos/include/cookie.h:57`) and is withdrawn. `_CPU`, `_FPU`,
`_SND`, `_FDC` stay absent (no such hardware); `_VDO` is decided after
measuring what TeraDesk's ST branch does on NewTOS (TASK/ARCH06.md).
**Note for every port:** depending on signedness, `-1` reads as "below ST"
(signed) or as "above Falcon" (unsigned: `(mch >> 16) >= 2` is true), so
every future port is checked for numeric `_MCH` comparisons. `Getcookie()`
zero-extends cookie values, so for a native LP64 port `0xFFFFFFFF` is a
large positive value and numeric comparisons take the "above Falcon"
hardware path (`docs/sdk/08-porting-checklist.md`). NVRAM does not
follow from `_MCH` (EmuTOS gates it on `has_nvram`): NewTOS has none and
answers `NVMaccess` like EmuTOS without an NVRAM chip (TASK/XBIOS10.md).
Evidence: `docs/analysis/machine-identity-adr-proposal-2026-10-28.md`.

### Alternatives considered

- **Map a `$5A0` (or any fixed) sysvar page so apps read+scan it**
  directly: rejected. The address itself is a non-issue once access is
  mediated, but letting apps *scan* the jar requires it to be readable
  from EL0 — a shared read-only user mapping per process (fiddly under the
  per-process-TTBR0 interim, adjacent to ADR-010) — and still wants
  `Supexec` for the classic idiom. Value-query needs none of that.
- **Mirror EmuTOS verbatim** (`$5A0` + `Supexec`): impossible — both
  prerequisites are absent/deferred on NewTOS.
- **Defer entirely**: rejected; the mechanism is now decided.

### References

- `docs/api/bios/_cookie-jar.md` (data model + access extraction)
- EmuTOS `util/cookie.c`, `bios/machine.c:665`, `cli/cmdutil.c:321`
- Open-ADR *ADR-Supexec*; `$420`-vars decision (STATUS Recent-Decisions
  2026-05-03); ADR-010 (low-memory aliasing on QEMU)

---

## ADR-013: Native `.RSC` resource format (not classic Atari)

**Date:** 2026-06-21
**Status:** **Accepted** (2026-06-21, user decision); the native format
stands. Addenda 2026-07-20 (free-string/free-image tables) and 2026-10-26
(system resources in the kernel image; coordinates and the later format
versions — the "coordinates already in pixels" part of the Decision no
longer holds since NRSC v9).
**Phase:** PHASE-13 D7 (Resource Manager).

### Context

`rsrc_load` must load a `.RSC` resource (object trees, strings, TEDINFOs,
icons) for GEM programs. A classic Atari `.RSC` is **big-endian m68k**,
with a **24-byte 16-bit OBJECT**, 16-bit internal offsets, and char-grid
coordinates fixed up by `rsrc_obfix`. NewTOS uses a **little-endian
64-bit OBJECT (32 bytes, 64-bit `ob_spec`)** and pixel coordinates.

### Decision

NewTOS defines and loads a **native `.RSC` format**: our own 64-bit
OBJECT layout, little-endian, internal pointers stored as **byte offsets
from the blob base**, coordinates already in pixels. Loading is a pure
**offset→pointer relocation** — no endianness swap, no 16→64-bit
expansion. `rsrc_obfix` is an identity (coords are already pixels).

NewTOS-native (AArch64) programs ship native `.RSC` files built by a host
tool; this matches the clean-room, recompile-for-AArch64 model. Loading
classic Atari `.RSC` files (full byte-swap + 24→32-byte expansion) is
**not** done now — it would belong with the optional m68k userland
emulator phase, if ever.

### Consequences

- Loader is small and fast (relocation = add base to offset fields,
  type-aware for `ob_spec`).
- Needs a **host tool** to author native `.RSC` (no ORCS/Interface);
  Batch 2 deliverable.
- Existing Atari `.RSC` files are **not** directly loadable.
- The format header is `NrscHeader` (magic `"NRSC"`); see
  [`tos-api/src/aes/resource.rs`].

### Alternatives rejected

- **Classic Atari `.RSC`**: full big-endian + 16→64-bit conversion on
  load. More code; valuable only for unmodified Atari binaries, which are
  out of scope until the m68k emulator.

### References

- `docs/api/aes/resource-fileselector.md`; EmuTOS `aes/gemrslib.c`,
  `include/rsdefs.h`

### Addendum 2026-07-20 (task #108): free-string / free-image tables

The `NrscHeader` gained a **free-string** (`frstr`) and a **free-image**
(`frimg`) pointer table so `rsrc_gaddr`'s `R_FRSTR`/`R_STRING`/`R_FRIMG`/
`R_IMAGEDATA` selectors resolve (QED fetches its alert table via
`rsrc_gaddr(R_FRSTR, …)`; without it `alertmsg` was NULL and cflib panicked).
Layout: after the ICONBLKs come `[ptr; n_frstr]` then `[ptr; n_frimg]`, each
slot a blob offset (string data for frstr, a BITBLK for frimg — a classic free
image *is* a BITBLK) relocated to a user VA at the app's ABI width (8 B LP64 /
4 B ILP32, matching `ob_spec`). The header grew from 28 to 44 bytes and the
`version` field was bumped: **1 = LP64/28 B, 2 = ILP32/28 B (legacy, no
tables); 3 = LP64/44 B, 4 = ILP32/44 B (with tables)**. The loader is
**backward-compatible**: for `version < 3` it treats the free-string/-image
counts as 0, so pre-existing v1/v2 resources (GEM.RSC, DESK.RSC) load unchanged
with those selectors reporting the honest "not found". Emitted by
`tools/mkrsc.py convert [--ilp32]`; relocated in `relocate_blob_core` /
`relocate_blob_core_ilp32`; registered/served in `relocate_and_register_core` /
`rsrc_gaddr` (`tos-api/src/aes/resource.rs`).

### Addendum 2026-10-26: system resources are compiled into the kernel image

Moved from the STATUS log (decision of 2026-06-20, "Ressourcen-Speichermodell")
and checked against the code of 2026-10-26.

**Decision (2026-06-20):** the system's own resources — the AES and desktop
dialog trees and system strings — are **compiled into the kernel image**, as
EmuTOS links them into its ROM; the GUI then boots without depending on the
file system. **Application `.RSC` files** are loaded from the card through
`rsrc_load` in the native format above. A file override for system resources
(theming, language) was left as a later option.

**State of the code today:**

- AES file selector: EmuTOS' `aes/gem.rsc`, converted to NRSC and embedded with
  `include_bytes!` (`GEM_FSEL_RSC`, `tos-api/src/aes/fsel.rs:85-98`). The
  override exists: a valid `C:\GEMSYS\RSC\GEM.RSC` is used first, the embedded
  blob is the fallback (`load_fselectr`, `tos-api/src/aes/fsel.rs:172-205`).
- AES alert trees and the AES's built-in `DESKTOP` tree are built in code
  (`tos-api/src/aes/form.rs`, `tos-api/src/aes/init.rs`), not loaded.
- Desktop: since ADR-026 the desktop is the EL0 program `NEWDESK`; its dialog
  and menu trees are built in its code (`programs/desktop/src/`), and the
  program itself is embedded in the kernel image (`kernel/src/embedded.rs`,
  `kernel/embed.rs`) — so the desktop's resources are still in the image,
  one level further out.
- Application resources: `rsrc_load` reads the file through
  `bdos::fs::read_file` (`tos-api/src/aes/resource.rs:2192-2233`).

### Addendum 2026-10-26 (b): coordinates are converted at load; format versions after v4

Checked against the code of 2026-10-26.

- **Coordinates.** The Decision's "coordinates already in pixels" and
  "`rsrc_obfix` is an identity" held up to NRSC v8 only. Since **NRSC v9**
  (v9 LP64 / v10 ILP32) an OBJECT carries the classic file's packed
  char-grid words, and the loader turns them into pixels at load with the
  live `gl_wchar`/`gl_hchar`/`gl_width`, as EmuTOS `rs_fixit` →
  `fix_objects` → `rs_obfix` does; `rsrc_obfix` (114) is that same
  `rs_obfix` (`tos-api/src/aes/resource.rs:9-13`, `NRSC_VER_CHARGRID`
  at `:345`). The reason: the converter had baked the pixels of a 640-wide
  8×16 screen into every resource. A v1–v8 blob still loads unconverted.
- **Versions.** The version list in the 2026-07-20 addendum ends at 4; the
  loader knows v1–v14 — sub-structure arrays (v5/v6), a relocatable tree
  index (v7/v8), char-grid coordinates (v9/v10), colour icons (v11/v12)
  and, from **v13** (LP64) / v14 (ILP32), a native `RSHDR` at the head of
  the image, so `ap_rscmem` points at what an EmuTOS application expects
  (`NRSC_VER_RSHDR`, `:380`). The authoritative list is the module
  documentation `tos-api/src/aes/resource.rs:16-79`. The format stays
  native (64-bit OBJECT, little-endian, byte offsets); this addendum
  changes nothing about the Decision's choice of format.

---

## ADR-014: All MMIO through the high-half; TTBR0 holds no low-half identity

**Date:** 2026-06-25
**Status:** Accepted
**Phase:** PHASE-06 / PHASE-14 (cross-cutting, MMU + drivers)

### Context

The first EL0 program launch (AUTO folder / desktop) worked in QEMU but
faulted on real Pi4: Instruction Abort, lower EL, **permission fault,
level 1**, at the entry VA `0x100` (`ESR=0x8200000d`). The Stage-1 boot
identity map covers 0..4 GiB with 1-GiB **UXN** L1 blocks and stayed live in
`TTBR0` because the kernel reached MMIO (UART, eMMC, mailbox, GIC) through
the low-half identity. The Cortex-A72 speculatively (re-)cached a UXN block
for the low-half; `install_ttbr0(slot)` swaps `TTBR0` to the slot tables but
**cannot flush the TLB** — a `tlbi` on the eret-to-EL0 path wedges QEMU
(QUIRK-001) — so the stale UXN block shadowed the slot's L3 page and the EL0
fetch faulted. QEMU flushes its TLB on the `TTBR0` write, masking it.

Two failed fixes confirmed the constraints: a `tlbi` in the launch path
wedged QEMU; emptying `TTBR0` while MMIO was still low-half hung the kernel
at the first UART write.

### Decision

Reach **every** peripheral register through the high-half (TTBR1) Device
mapping (`drivers::mmio_va(pa) = pa + VA_OFFSET`). With MMIO independent of
`TTBR0`, `install_stage2_ttbr1` points `TTBR0` at an all-invalid root
(`EMPTY_L0`) after the high-half switch: no UXN identity block remains to be
cached, and per-process `install_ttbr0` swaps in slot tables cleanly with no
TLBI on the eret path. The pre-MMU boot heartbeat UART stays physical.

### Rationale

- Eliminates the fault at the root rather than fighting QUIRK-001.
- A kernel should not depend on a low-half identity at runtime; high-half
  MMIO is the clean design and also lets the kernel touch MMIO while a
  process owns `TTBR0`.
- QEMU now exercises the **same** path as real HW (high-half MMIO + empty
  TTBR0), so it validates the launch path instead of hiding the bug.

### Consequences

- Also required: freshly-loaded code needs D-cache-clean-to-PoU +
  I-cache-invalidate (ARM ARM B2.4.4) and table writes need a `dsb` before
  the walk (D5.10.1) — real-HW-only, no-ops under QEMU's cacheless model.
- `remap_normal_nc` must map **invalid** blocks too: the VideoCore puts the
  framebuffer in reserved memory above `dram.end`, which Stage-2 leaves
  unmapped; the old "convert only valid Normal blocks" guard skipped it and
  the desktop draw took a translation fault. (8-GiB SKUs with a framebuffer
  above 4 GiB still need a `NUM_L2` coverage bump.)
- **Verified end-to-end on real Pi4 (2026-06-25):** init screen → AUTO
  program loads from SD, runs at EL0, Pterm → desktop with 4 drives, no
  exception.

### References

- QUIRK-001 (this file); commits `e8fa10b2` (high-half MMIO + empty TTBR0),
  `af37a4dc` (framebuffer mapping), `e82cf6d3` (cache/barrier maintenance),
  all 2026-06-25. *(Corrected 2026-10-26: the hashes first written here,
  7231e9d / c91c9e5 / 3f93fde, never existed; the commits' old hashes were
  2fb1af4 / b1d5f7b / a858961.)*

## ADR-015: USB-HID input — xHCI host on Pi4, upper stack clean-room from FreeMiNT

**Date:** 2026-06-25
**Status:** Accepted; amended 2026-10-26 — the "fixed MMIO, no PCIe
enumeration" consequence was wrong, and the UART2 IKBD fallback no longer
exists (see the amendment at the end).
**Phase:** PHASE-16 (USB-HID input)

### Context

The desktop renders on real Pi4 but is not interactive — there is no
keyboard/mouse. The native Atari IKBD plan (a vintage 5 V keyboard on UART2)
proved too fiddly (voltage divider, TX level-shifter, uncertain HD6301
hardware reset). A USB host stack is the "right" answer but is large.

Two reference bodies make it tractable:
- **FreeMiNT's USB stack** (`references/freemint/sys/usb`, a sanctioned
  source) is a **TOS-native, modular** USB stack: a host-controller layer
  (`struct ucdif` + `ucd_register`), a hardware-independent core + hub +
  enumeration (`usb.c`/`hub.c`), class drivers
  (`references/freemint/sys/usb/src.km/udd/`), and — crucially —
  HID keyboard/mouse drivers that **translate USB HID → Atari scancodes and
  inject them into the IKBD vector**
  (`references/freemint/sys/usb/src.km/udd/hid/keyboard/usb_keyboard.c`:
  `Setexc(0x114/4, …)`, `SEND_SCAN`). Only the `ucdif` host-controller layer
  is hardware-specific.
- **Circle** (C++, `rsta2/circle`) + the no_std **`rust-osdev/xhci`** crate
  are the references for the Pi4 host controller itself.

### Decision

Target **xHCI** (the VL805 controller behind the four USB-A ports, MMIO
`0xFE9C0000`), not DWC2. Build the stack as: (1) clean-room reimplement
FreeMiNT's upper stack in Rust (core, hub, enumeration, HID keyboard/mouse,
HID→IKBD injection) per the source-analysis protocol — exactly the EmuTOS
methodology; (2) write one Pi4 xHCI host-controller driver (the `ucdif`
equivalent) on top of `rust-osdev/xhci`, with Circle as the behavioural
reference and the xHCI spec as Track-B; (3) feed HID output into NewTOS's
**existing** IKBD sinks (`ikbd.rs` → console RX + `vdi::feed_relative`
cursor), so the desktop is source-agnostic.

### Rationale

- **xHCI = the real ports.** A keyboard + mouse plug into the normal USB-A
  ports; power stays on USB-C. DWC2 would only drive the USB-C/OTG port
  (also the power input) and need a hub — awkward as a product.
- **QEMU validates the whole path.** `qemu-virt` emulates `qemu-xhci` with
  `-device usb-kbd`/`-device usb-mouse`, so the entire stack (controller +
  core + hub + HID) is developed and tested **interactively in the emulator**
  — the same "QEMU exercises the real path" win as ADR-014. Real Pi4 then
  adds only the VL805 firmware-load (via the existing mailbox driver), which
  QEMU doesn't need. DWC2 has no equivalent qemu-virt dev loop.
- **~70 % already specified.** FreeMiNT supplies the architecture and the
  TOS-native HID→IKBD integration; NewTOS already owns the IKBD-sink side.

### Consequences

- Reuses the existing IKBD decoder's output side untouched; the USB-HID
  driver maps HID→Atari-scancode, then the existing path takes over.
- Real-HW bring-up needs VL805 firmware bootstrap via mailbox (have the
  driver) + xHCI at `0xFE9C0000` (a fixed MMIO the firmware maps; no full
  PCIe enumeration required). *(Wrong — see the amendment below.)*
- The IKBD/UART2 plan is shelved (kept only as a fallback note).
  *(The fallback is gone — see the amendment below.)*

### References

- `references/freemint/sys/usb` (sanctioned source); `rsta2/circle`;
  `rust-osdev/xhci`; xHCI spec; RPi firmware VL805 mailbox interface.
  See ADR-014 (high-half MMIO — the mailbox/MMIO groundwork this builds on)
  and the shelved IKBD path (removed, see the amendment below).

### Amendment 2026-10-26 — PCIe enumeration, and no UART2 fallback

Checked against the code of 2026-10-26. The stack runs on the real Pi 4
(USB-HID keyboard and mouse are the only input there), but two statements
above did not hold:

- The VL805 is a **PCIe endpoint**, not a fixed MMIO block. The kernel
  brings up the BCM2711 PCIe root complex, trains the link, enumerates the
  VL805 (bus 1, slot 0), assigns its xHCI BAR into the outbound window at
  CPU `0x6_0000_0000` and loads its firmware
  (`kernel/src/drivers/usb/pcie_rpi4.rs:1-21`, clean-room from Circle's
  `references/circle/lib/bcmpciehostbridge.cpp`; spec `docs/api/usb/pcie-rpi4.md`). The
  `0xFE9C0000` premise in Decision and Consequences is wrong.
- The UART2 IKBD driver (`ikbd_uart`) was removed (`1ace07a4`); there is no
  IKBD fallback. The IKBD *decoder* (`kernel/src/ikbd.rs`) stays as the
  sink the HID drivers feed.

## ADR-016: Ground-up desktop rebuild — EmuTOS-faithful, in kernel/src/desk/ (path before DESK43 U8b)

**Date:** 2026-07-04
**Status:** **Superseded by ADR-022** (2026-07-22: the desktop is a 1:1
rebuild that goes through the AES trap) **and ADR-026** (2026-10-16: the
desktop is the EL0 program NEWDESK in `programs/desktop/`; the kernel
carries no desktop). Accepted by user decision 2026-07-04.
**Phase:** PHASE-14 (Desktop).

### Context

The hand-rolled desktop (kernel/src/desktop.rs ~2329 lines +
kernel/src/desktop_icons.rs ~442 lines, both since deleted) keeps producing rendering defects:
icon-view 8.3 label overlap on real Pi-4 (`CONFIG.TXFIXUP4.DARERNEL8.IME…`),
view-switch (text↔icon) residue, and a redraw path that repaints by handle
order with a full-screen clip instead of honoring the window's damage-rectangle
list. The root causes sit in the **ad-hoc directory-window subsystem** (its own
`DIRTREE`/`build_dir_*`/`draw_dir_listing`/`redraw_*`/`VIEW_MODE`/`SORT_MODE`),
**not** in the AES/VDI foundation, which is EmuTOS-aligned. An earlier QMP
screendump "passed" only because the test image used short names — the
verification was not representative. A targeted patch was drafted
(`~/.claude/plans/mossy-knitting-scone.md`), but after repeated regressions the
user judged the desktop untrustworthy and chose a full ground-up rebuild.

### Decision

Rebuild the entire desktop **application** from scratch as an EmuTOS-faithful
reimplementation in a **new module kernel/src/desk/** (path before DESK43
U8b), mirroring EmuTOS
`desk/` (`deskmain`/`deskapp`/`deskwin`/`desksupp`/`deskdir`/`deskfun`/
`deskact`/`deskins`/`deskinf`/`deskrez`). `desk::run()` replaces
`desktop::run()` at boot. Keep only the OS layer below it — AES/VDI/GEMDOS in
`tos-api` — which is the compatibility contract, **not** "the desktop". The old
`desktop.rs`/`desktop_icons.rs` are declared **deprecated** and stay in git
until the new module reaches feature parity, then are removed. Extraction of
`references/emutos/desk/` into `docs/api/desktop/` **precedes** implementation
(Rule 5, spec before code).

### Rationale

The bug class is structural — an ad-hoc data model + view builder + a redraw
that ignores the real AES rectangle list. Faithfully re-deriving the EmuTOS
window/file data model (`FNODE`/`WNODE`), grid-metric globals (`win_view`), view
builder (`win_bldview`) and damage-rect redraw (`do_wredraw`) on top of the
already-faithful AES/VDI kills the class at the root instead of patching the
three symptoms currently visible. A **module** (chosen over a separate crate,
user decision) gives a clean home and a clear old/new boundary with minimal
workspace-wiring churn.

### Consequences

- Positive: a clean, EmuTOS-conformant desktop; the label-overlap / redraw bug
  class eliminated at the root; the desktop is spec-backed (`docs/api/desktop/`)
  for the first time.
- Positive: reuse of the verified AES/VDI/GEMDOS layer — no contract risk.
- Negative: substantial reimplementation (~full desktop app); two
  implementations coexist until parity (old deprecated + guarded off).
- Negative: as a module (not a crate) the desktop *can* still reach kernel
  internals; the EmuTOS discipline (go through AES/VDI/BIOS calls, as `desk/`
  uses `aesbind`) must be enforced by **review**, not by the compiler — this is
  the Auditor's standing charge.
- Neutral: supersedes the partial `mossy-knitting-scone` plan.

### Alternatives considered

- **Separate `desk/` crate:** the strongest EmuTOS-faithful boundary
  (compiler-enforced app/OS split) but more workspace wiring and API-surface
  exposure. Rejected by the user in favor of the lighter module; revisit if the
  boundary needs enforcing.
- **Targeted patch (`mossy-knitting-scone`):** fix `draw_text` clip + icon-cell
  metric + redraw only, keep the rest. Rejected: the user judged the desktop
  untrustworthy after repeated regressions; a rebuild removes the whole ad-hoc
  subsystem rather than the three symptoms currently visible.
- **Keep everything, patch in place:** rejected — same reason; leaves the ad-hoc
  model that keeps regressing.

### Orchestration (process)

Executed by a **three-role subagent team** under team-lead (Claude) supervision,
defined in `.claude/agents/`:
- **Researcher** (Sonnet) — extracts `references/emutos/desk/` → `docs/api/desktop/`
  per SOURCE-ANALYSIS-PROTOCOL + defect catalog of the current desktop; **no
  product code**.
- **Developer** (Opus) — writes **all** Rust/AArch64 code in kernel/src/desk/
  (path before DESK43 U8b);
  **no stubs**.
- **Auditor** (Opus) — reviews Developer code against spec + COMPATIBILITY,
  adversarially verifies (representative data), reports findings; **no code**.

Model rule: simple tasks on Sonnet, **code exclusively on Opus**.

### References

- `references/emutos/desk/` (`deskmain.c`/`deskapp.c`/`deskwin.c`/`desksupp.c`/
  `deskdir.c`/`deskinf.c`/`deskrez.c`/`deskfpd.c`/…)
- `~/.claude/plans/mossy-knitting-scone.md` (superseded)
- `docs/api/desktop.md` (to be expanded into `docs/api/desktop/`)
- `docs/COMPATIBILITY.md` (desktop = app; AES/VDI/GEMDOS = contract)
- STATUS Recent-Decisions 2026-07-03 (icon-overlap partial fix — now subsumed)
- `.claude/agents/{researcher,developer,auditor}.md`

## ADR-017: NewTOS resolution policy — native, geometry-independent, resolution from `config.txt` (Revision 2; first version: 1920×1080 switchable to 1280×800)

**Date:** 2026-07-09 (revises a same-day, never-committed draft of this ADR —
see "Superseded draft" below).
**Status:** **Accepted as revised by Revision 2 (2026-07-16):** the display
geometry comes from `config.txt` and is detected at boot (fallback
1920×1080); there is no runtime resolution switch. The 2026-07-09 text
below (two switchable resolutions, Decision points 1, 5 and 7) is history.
Its colour point 2 (16-colour palette kept) is superseded by ADR-024.
Geometry independence (point 3) stands as a rule. (Originally accepted by
user course-correction 2026-07-09, confirmed via team lead.)
**Phase:** cross-cutting (framebuffer driver, VDI workstation, AES globals,
desktop `deskapp`/`deskwin`/`deskrez`, XBIOS screen group).

### Revision 2 (2026-07-16) — config.txt-driven, no in-menu switch

The 2026-07-09 policy kept an *in-menu runtime resolution switch* (deskrez ->
`shel_write(SHW_RESCHNG)` -> reinit) as on classic TOS. On reflection with the
user (2026-07-16) that is **dropped**: on Atari, rez switching mattered for
gaming (ST Low/Med/High; programs demanded a specific mode). On NewTOS/Pi4 the
**firmware `config.txt` owns the HDMI mode** (`boot/rpi4/config.txt`
`hdmi_group`/`hdmi_mode`), and a framebuffer-only in-menu switch to a *different*
geometry would merely be **scaled** by the VideoCore to the fixed HDMI signal —
not native. So:

- **Resolution is driven by `config.txt`.** NewTOS **auto-detects** the display
  geometry from the VideoCore at boot (mailbox *get physical (display)
  width/height*, tag `0x00040003`) and renders natively at whatever `config.txt`
  set. `drivers::BOOT_WIDTH/BOOT_HEIGHT` (1920x1080) becomes the **fallback** —
  used when the query returns 0/none and as the **QEMU/ramfb default** (ramfb has
  no firmware display to query; the guest defines the geometry). Change the panel
  resolution by editing `config.txt`; NewTOS follows with no code change.
- **The in-menu switch (deskrez runtime switch) is not implemented.** `Getrez`
  reports the auto-detected mode honestly; there is **no in-system `Setscreen`
  rez change**. (This narrows Decision point 6 / the still-open Getrez question:
  Getrez only *reports*, it does not *switch*.)
- **Config surface = exactly two files:** `config.txt` (firmware: HDMI/resolution,
  `gpu_mem`, `kernel=`, UART, DTB) and `NEWDESK.INF` on C: (desktop state:
  background, view/sort, drive icons `#M`, installed apps `#G/#P/#F/#Y`, autorun
  `#Z`). No third NewTOS-specific config.
- **Wallpaper** (a full-screen background image, a NewTOS extension) is a separate
  future feature: a `NEWDESK.INF` path + an image loader + the desktop background
  painter (`paint_desktop_bg`). It is sensibly done **after** the truecolor VDI
  step — a photo wallpaper in the current 16-colour palette would dither badly.
  *(2026-10-26: the truecolor step is done (ADR-024); no wallpaper has been
  built, and `paint_desktop_bg` no longer exists.)*

Everything below about *geometry-independence*, the *arena/cache blockers*, and
the native-vs-16-colour split still holds; only the "runtime switchable via menu"
mechanism is replaced by "config.txt-driven + boot-time auto-detect".

### Superseded draft

Earlier the same day this ADR was drafted — and never committed — as
"**single fixed 640×480 mode, truthfully `TT_MEDIUM`**": exactly one
hardcoded VDI resolution forever, `Getrez` corrected from `2` to `4`,
`Setscreen`'s range widened to EmuTOS's flat `[0,7]` bound with every
non-`4` code accepted-but-ignored, and native/1080p geometry filed as an
explicitly deferred future option. The user overrode this direction before
it was committed: NewTOS presents a **native, geometry-independent**
resolution — default **1920×1080**, switchable at runtime to **1280×800** —
not a single fixed 640×480 canvas. The EmuTOS source analysis performed for
the superseded draft (its own "silently ignore an unsupported `rez`"
precedent, `bios/screen.c:990-1013`, `bios/videl.c:619-659,976-993`) is
still factually correct and is kept below where it still bears on the
still-open `Getrez`/`Setscreen` question (see Decision point 6); the
**policy conclusion built on top of it — "there is only one mode, ever" —
is replaced.**

### Context

NewTOS's physical framebuffer is currently a **compile-time-fixed** 640×480,
32bpp truecolor VideoCore mailbox buffer (`kernel/src/drivers/bcm2711/
framebuffer.rs:23-24` `const WIDTH: u32 = 640; const HEIGHT: u32 = 480;`,
mirrored in the QEMU dev-vehicle `kernel/src/drivers/qemu_virt/ramfb.rs:75-76`).
This is an **implementation** limit, not a **hardware** one: the RPi4's own
boot firmware config already forces a 1920×1080@60 Hz HDMI signal
(`boot/rpi4/config.txt:12-16`, `hdmi_group=2`/`hdmi_mode=82` = "DMT
1080p60", per the file's own comment), and the mailbox property tags the
driver already speaks (`set physical/virtual size`, `set depth`, `allocate`,
`get pitch` — `kernel/src/drivers/bcm2711/mailbox.rs`,
`docs/hardware/mailbox-framebuffer.md` §3) are fully parametric; only the
values fed into them are hardcoded today. The superseded draft's central
premise — "the framebuffer is fixed at boot, there is no second mode to
switch into" — conflated *what the code currently asks for* with *what the
hardware and firmware config can do*; this rewrite corrects that premise.

A Researcher code survey (2026-07-09, same session) of `tos-api::vdi`,
`tos-api::aes` and `kernel::desk` found that **most of the VDI/AES stack is
already geometry-independent**: `RasterBackend::width()/height()`
(`kernel/src/vdi_backend.rs:130-136`) and `Host::screen()`
(`kernel/src/host_env.rs:262-273`) both re-derive from the live framebuffer
atomics on every call — nothing caches 640/480 as a value, only
`kernel/src/drivers/bcm2711/framebuffer.rs`'s `WIDTH`/`HEIGHT` **constants**
feed those atomics once at boot. `tos-api/src/vdi/workstation.rs:100-117,
143-160` (`DEV_TAB`/`INQ_TAB` patching), `tos-api/src/aes/{graf,form,fsel,
menu,appl,window}.rs` (screen-rect/centring/planes) all read
`backend.width()/height()` fresh per call; `tos-api/src/aes/window.rs:1007-
1013`'s `640, 480` is an unreachable-in-practice fallback (no backend
installed), not the live path. The full file:line inventory — including the
few genuine gaps found (see Consequences) — was handed to the team lead
alongside this ADR, not duplicated here.

EmuTOS's own resolution-change precedent (Track A, `1a7484ab`) directly
informs the switch-mechanism decision below: a rez change is **never** a
live hot-patch of a running desktop. The desktop's `RESITEM` menu action
(`desk/deskmain.c:900-917`) runs a resolution dialog
(`desk/deskrez.c:change_resolution`/`change_falcon_rez`/`change_tt_rez`),
then calls `shel_write(SHW_RESCHNG, …)` (`aes/gemshlib.c:157-196`), which
only sets the `gl_changerez`/`gl_nextrez` globals and asks the desktop
process to **exit**. The actual `Setscreen()` call happens at the very top
of the AES's cold-start entry point, `gem_main()`
(`aes/geminit.c:753,775-806`), **before** any window/menu/VDI-workstation
state exists — i.e. "switch resolution" == "apply the new mode, then run
the exact same from-scratch init sequence boot uses", not an incremental
re-layout of live state. The choice is persisted across boots by
overloading the `#E` NEWDESK.INF line's mode-word field
(`desk/deskapp.c:1161-1174` writes it via `desk_get_videomode()`;
`aes/geminit.c:343-392` `process_inf1()` reads it back and sets
`gl_changerez`/`gl_nextrez` from it at the next boot).

Separately, EmuTOS's own Falcon `Getrez` has no classic code for a
non-ST-compatible (true-colour/high-res VGA) Videl mode: `atari_getrez()`
(`bios/screen.c:948-971`) reads `vsetmode(-1)`, and for any Videl mode
**without** the `VIDEL_COMPAT` bit it hardcodes `rez = 2` (`ST_HIGH`) —
*not* `FALCON_REZ` (3) — regardless of the mode's real geometry/depth
(`screen.c:968-970`). Meanwhile `vsetmode()` itself
(`bios/videl.c:619-659`) sets the **`sshiftmod`** shadow to `FALCON_REZ`
for exactly the same non-compat modes (`videl.c:648-656`) — so even in
EmuTOS, two internal state variables disagree about "the" rez code the
instant the live mode isn't one of the eight classic Atari codes, and the
XBIOS-visible `Getrez()` picks the degenerate ST-compatible guess, not the
Falcon indicator. Real Falcon programs query the *actual* mode via
`VsetMode(-1)` (a raw Falcon mode word: bpp bits, VGA/RGB, sync, resolution
flags), never by inferring it from `Getrez`. This precedent is why point 6
below leaves NewTOS's `Getrez`/`Setscreen` classic-rez contract an **open
question** rather than guessing a value: 1920×1080 and 1280×800 are exactly
the situation EmuTOS itself has no honest classic code for.

### Decision

1. **Two native, live VDI resolutions**, both delivered through the
   existing VideoCore mailbox path: **1920×1080×32bpp (default)** and
   **1280×800×32bpp (switchable at runtime)**. Both ride the *same* fixed
   1920×1080@60 Hz HDMI wire signal already forced by
   `boot/rpi4/config.txt`; a 1280×800 mailbox "set physical size" request is
   expected to be scaled/letterboxed by the VideoCore firmware onto that
   fixed signal without an HDMI/EDID renegotiation — **this is an
   expectation to verify empirically (Stufe 1), not an assumed fact**, since
   `docs/hardware/mailbox-framebuffer.md` does not yet document VideoCore's
   physical-size-vs-forced-`hdmi_mode` scaling behaviour.
2. **Colour model unchanged for this round.** The VDI keeps presenting the
   existing 16-colour/4-plane palette raster
   (`RasterBackend::num_colors() == 16`, `kernel/src/vdi_backend.rs:21`)
   over the (now larger) 32bpp truecolor physical buffer, exactly as today
   at 640×480. A genuine truecolor VDI mode (a real `DEV_TAB`/`INQ_TAB`
   colour-depth increase, a new pixel-format contract for programs) is
   **explicitly out of scope for this ADR** — a later, separate decision.
3. **Geometry independence is the actual engineering target** and a hard
   rule going forward: no VDI/AES/desktop code may assume 640/480/639/479
   (or any other fixed dimension) — every consumer reads the live geometry
   (`RasterBackend::width()/height()`, `Host::screen()`, or the desktop's
   own `DeskState::g_desk`/`screen_dims()` plumbing). Most of the stack
   already follows this (see Context); the remaining gaps are enumerated in
   Consequences and must close.
4. **Staging rule (non-negotiable):** the physical framebuffer stays at its
   present geometry until geometry independence is verified end-to-end
   (VDI + AES + desktop rendering correctly at a live resolution ≠
   640×480, confirmed by screendump) — only then does the framebuffer
   driver's target geometry change to 1920×1080. Raising the physical
   resolution before that point would render a broken desktop, per the
   user's explicit instruction.
5. **Switch-mechanism skeleton (proposed, mirrors the EmuTOS precedent in
   Context, not yet a final API — Developer's to implement):** a
   resolution change re-runs the same cold-start sequence boot already
   uses, rather than hot-patching live state: re-issue the mailbox request
   with the new geometry → invalidate the desktop's one-time-computed state
   (kernel/src/desk/global.rs's idempotent `DESK` static,
   kernel/src/desk/menu.rs's idempotent `MENU.built` cache, paths before
   DESK43 U8b — both
   currently "compute once at boot, never again") → re-run the equivalent
   of `DeskState::new`/`app_start`/`win_start`/`build_menu` → full redraw.
   NewTOS has no separate AES *process* to restart (the desktop is a kernel
   module, `desk::run()`), so "restart" here means "re-run the one-time
   setup", not a reboot.
6. **`Getrez`/`Setscreen`'s classic-rez mapping is explicitly NOT settled by
   this ADR — flagged as an open Lead decision.** Neither 1920×1080 nor
   1280×800 (16 colours) matches any entry of EmuTOS's `vmode_table[]`
   (`bios/screen.c:724-735`); per the Falcon precedent in Context, EmuTOS
   itself has no honest classic code for an "off-table" mode either. Three
   sketched options (none chosen): (a) a fixed degenerate code regardless of
   which of the two live resolutions is active — mirroring Falcon's
   `Getrez`-returns-`2`-regardless precedent; (b) a resolution-derived code
   that differs between the two modes; (c) a NewTOS-native query function
   (cookie-jar or a new trap) alongside a degenerate classic `Getrez`,
   mirroring Falcon's real `VsetMode(-1)`-is-ground-truth /
   `Getrez`-is-a-hint split. See the Researcher's 2026-07-09 report to the
   lead for the full option writeup.
7. **Persistence + UI entry points (proposed, mirrors EmuTOS):** a resolution
   choice persists across boots via a NEWDESK.INF field analogous to the
   `#E` mode word (`desk/deskapp.c:1161-1174`, `aes/geminit.c:343-392`); a
   desktop menu item analogous to `RESITEM`/`deskrez` offers the two
   resolutions. Whether NewTOS reuses the literal `#E` byte layout (risk:
   collides with real `#E` semantics for the environment/time-format bits
   sharing that line) or adds a NewTOS-private INF line is an **open Lead
   decision**, not decided here — `docs/api/desktop/deskrez.md`
   (previously "out of scope" under the superseded draft) is back in scope
   and needs its own extraction pass before implementation (Rule 5).

### Rationale

- **The Pi-4 hardware trivially supports more than 640×480**: the boot
  firmware already negotiates a 1920×1080@60 Hz HDMI signal
  (`boot/rpi4/config.txt:12-16`) and the mailbox driver already speaks the
  parametric tags needed (`mailbox.rs`, `framebuffer.rs:build_request`);
  nothing here is invented hardware behaviour (CLAUDE.md Rule 6) — it is
  the same documented mailbox interface (`docs/hardware/mailbox-framebuffer.md`
  §3) fed different, still-data, values.
- **"New" means modern, not artificially ST/TT/Falcon-constrained.** NewTOS
  is a clean-room reimplementation, not a binary-compatible emulator
  (`CLAUDE.md` non-goals); a source-ported program queries the VDI for its
  geometry (`v_opnvwk`/`vq_extnd`) rather than assuming a fixed Atari
  screen size, so raising the native canvas is transparent to a
  well-behaved program exactly as it was across real ST/TT/Falcon
  resolutions.
- **Colour truecolor is deliberately not bundled with this change**: it is
  an orthogonal VDI/backend redesign (an actual `DEV_TAB`/`INQ_TAB`
  colour-depth increase, a program-visible pixel-format change) that would
  multiply the risk of this change; isolating the work to geometry keeps
  the blast radius bounded and independently verifiable.
- **Source-backed switch model.** EmuTOS itself never hot-patches a live
  AES for a resolution change — it always applies `Setscreen()` at the very
  top of a fresh `gem_main()` and lets the rest of AES/desktop init run from
  scratch (Context). Mirroring "tear down cached state, re-run the one-time
  setup" rather than inventing a live incremental re-layout is the
  source-faithful choice, not a shortcut.
- **The `Getrez`/`Setscreen` question is left open, not guessed**, per
  CLAUDE.md Rule 3 (STOP on uncertainty): EmuTOS's own Falcon behaviour
  shows that "what classic code does an off-table mode get" has no single
  precedent-mandated answer (its own `Getrez`/`sshiftmod` even disagree with
  each other for the same mode, Context) — inventing a value here would be
  exactly the kind of guess CLAUDE.md forbids.

### Consequences

- Positive: matches modern expectations (an HD desktop) using only the
  mailbox interface the codebase already implements — no invented hardware.
- Positive: the code survey shows most of VDI/AES already reads geometry
  live; the remaining work is bounded and enumerated, not open-ended.
- Negative (correctness blocker, must close before 1080p goes live): the
  desktop's fixed-size object arena
  (`MAX_WIN_ITEMS = 48`, `INITIAL_ITEM_SLOTS = (NUM_WNODES+1)*MAX_WIN_ITEMS`,
  kernel/src/desk/global.rs:43-62, path before DESK43 U8b) is explicitly sized "at 640×480" in its
  own doc comment; a maximized window/desktop at 1920×1080 fits roughly
  17×20 = 340 icon cells with the current fixed cell pitch (`g_wicon=100`,
  `g_hicon=50`, `min_wint=8` — kernel/src/desk/app.rs:44-61), not 48. The
  arena cannot grow after `obj_init` (raw `ob_spec`/`ib_ptext` pointers baked
  into the tree would dangle on a `Vec` regrow, per the existing doc
  comment), so this constant must be recomputed for the worst-case
  resolution *before* 1080p is enabled, not patched reactively.
- Negative: two desktop caching spots need an explicit invalidate/rebuild
  path that does not exist today — kernel/src/desk/global.rs:471-487's
  `DESK` static (`init()` is a no-op on a second call) and
  kernel/src/desk/menu.rs:372-378,483's `MENU.built` flag (paths before
  DESK43 U8b) (the menu-bar
  object tree, sized to screen width/height, is built exactly once).
- Negative/Open: `docs/api/desktop/deskrez.md` re-enters scope (was
  "out of scope" under the superseded draft) and needs its own EmuTOS
  extraction pass (`desk/deskrez.c`) before the Developer implements the
  switch-UI, per Rule 5.
- Negative/Open: the classic-rez (`Getrez`/`Setscreen`) contract question
  is reopened (point 6) — no classic Atari code matches either target
  resolution; this needs a Lead decision, not a Researcher default.
  `docs/api/_discrepancies.md` D-076/D-077 (written for the superseded
  single-mode draft) are **not** retracted (append-only log) but a
  follow-up entry marks them reopened — see D-078.
- Neutral: `Physbase`/`Logbase` behaviour is unaffected by this ADR.
- Deferred: real truecolor VDI remains a documented future step, out of
  scope here — inverse framing of the superseded draft (geometry ships now,
  colour depth later, rather than the other way around).

### Alternatives considered

- **Keep the single fixed 640×480/`TT_MEDIUM` mode** (the just-superseded
  draft): rejected same-day by the user — the Pi4 hardware and firmware
  config trivially support more; "New" should mean modern, not an
  artificial 1980s-Atari-geometry ceiling.
- **Ship geometry independence and real truecolor together:** rejected —
  bundling maximizes scope/risk for one change; colour depth is a separate,
  program-visible contract question better isolated as its own later ADR.
- **Query HDMI/EDID for the monitor's native mode instead of a fixed
  1920×1080 default:** not chosen for this round — `hdmi_mode` is already
  forced in `boot/rpi4/config.txt`, so there is no live EDID negotiation
  today; real EDID-driven mode detection remains a further-future option,
  not a blocker for the two-fixed-resolution switch requested here.
- **A single resolution that just moves from 640×480 to one higher fixed
  value (no runtime switch):** rejected — the user explicitly asked for two
  selectable live resolutions (1920×1080 default, 1280×800 alternate).

### References

- `kernel/src/drivers/bcm2711/framebuffer.rs:23-24,86-143` (fixed
  `WIDTH`/`HEIGHT` consts + the parametric mailbox request they feed)
- `kernel/src/drivers/qemu_virt/ramfb.rs:75-76` (QEMU dev-vehicle mirror)
- `boot/rpi4/config.txt:12-18` (forced DMT 1080p60 HDMI timing)
- `docs/hardware/mailbox-framebuffer.md` §3 (mailbox tag reference —
  needs a Track-B addendum on physical-size-vs-forced-`hdmi_mode` scaling
  and the buffer-release tag before a live re-request is implemented)
- `kernel/src/vdi_backend.rs:130-136`, `kernel/src/host_env.rs:262-273`,
  `tos-api/src/vdi/workstation.rs:100-117,143-160`,
  `tos-api/src/aes/window.rs:1007-1013` (already-dynamic geometry reads)
- kernel/src/desk/global.rs:43-62,471-487, kernel/src/desk/menu.rs:372-378,483,
  kernel/src/desk/app.rs:44-61 (paths before DESK43 U8b; the desktop-side
  gaps: arena sizing, the two one-time caches)
- EmuTOS `1a7484ab`: `aes/gemshlib.c:157-196` (`SHW_RESCHNG`),
  `aes/geminit.c:334-392,753,775-806` (`process_inf1`, `gem_main` cold
  `Setscreen`), `desk/deskmain.c:900-917` (`RESITEM`), `desk/deskrez.c`
  (`change_resolution`/`change_falcon_rez`/`change_tt_rez`),
  `desk/deskapp.c:1083-1174` (`#E` mode-word read/write), `bios/screen.c:
  724-735,948-971` (`vmode_table`, `atari_getrez` Falcon quirk),
  `bios/videl.c:619-659` (`vsetmode`, `sshiftmod = FALCON_REZ`)
- `docs/api/xbios/04-getrez.md`, `docs/api/xbios/05-setscreen.md`,
  `docs/HARDWARE-PROFILE.md` §3.1 (updated alongside this ADR, open points
  marked)
- `docs/api/_discrepancies.md` D-076, D-077 (superseded draft's corrections,
  now reopened — see D-078)
- `docs/api/desktop/deskrez.md`, `docs/api/desktop/_overview.md` (back in
  scope — needs its own extraction pass)
- [[qemu-window-view-command]] (how to visually verify framebuffer output
  in QEMU at a non-default resolution)
- ADR-016 (desktop rebuild — the geometry work is cross-cutting and must be
  sequenced against the in-flight kernel/src/desk/ task list)

## ADR-018: Native executable extension family — `.NAP` / `.NCP` / `.NTP` / `.NAC`

**Date:** 2026-07-16
**Status:** **Accepted** (user decision 2026-07-16, confirmed via team lead);
amended 2026-09-24 (`.NAC`), 2026-10-06 (`.NLD`) and 2026-10-26 (launch path
as built — the Routing, Change surface and Staged plan sections are history).
**Phase:** cross-cutting (desktop launcher, then kernel/src/desk/ — path
before DESK43 U8b, now `programs/desktop/`; `kernel/src/launch.rs`,
program-build tooling, NEWDESK.INF associations)

### Context

NewTOS programs are **native AArch64 ELF** binaries: `pexec::load` reads the
file, parses the ELF, loads it into a process slot, and `kernel::launch::
run_program` drops to EL0, runs it, and resumes when it `Pterm`s. They are
**not** m68k Atari `.PRG`/`.APP`/`.TOS`/`.TTP` binaries. The desktop must be
able to launch a program by double-click; today `open_item`
(kernel/src/desk/main.rs, path before DESK43 U8b) returns `OpenTarget::None` for a plain file (an
explicit no-op).

Reusing the classic Atari suffixes for native ELF apps would conflate two
architectures under one name: once the m68k userland emulator (a later phase)
exists, a `.PRG` could be *either* a native ELF *or* an original m68k binary,
indistinguishable by name. We cannot rename existing Atari binaries — they
*are* named `.PRG`/`.APP`/`.TOS`/`.TTP` — so the m68k side is fixed and the
**native** side takes the new, distinct extensions.

### Decision

Native NewTOS ELF programs use a new **`N_P` family** (middle letter = role);
the classic Atari suffixes are **reserved for m68k** binaries (run via the
future emulator). The file extension is a desktop/shell convention, **not** part
of the TOS syscall ABI, so this does not touch TOS API compatibility.

| Ext | Role | m68k analog | Launcher behavior |
|-----|------|-------------|-------------------|
| `.NAP` | Native **A**pp (GEM) | `.PRG` / `.APP` | `run_program`; the app uses AES and draws its own windows over the desktop |
| `.NCP` | Native **C**onsole program | `.TOS` | `fbcon::activate` (VT52 text screen), `run_program`, restore the desktop on `Pterm` |
| `.NTP` | Native, **T**akes params | `.TTP` (**not** `.GTP` — decided 2026-09-26) | prompt for a command tail (form dialog), pass it as the child's cmdline, then run **on the console arm**; see [DESK44](../TASK/DESK44.md) |
| `.NAC` | Native **A**ccessory | `.ACC` | *not* launched by double-click — found by the boot-time scan, loaded, and left running; see `docs/api/aes/accessories.md` |

**Why exactly three:** NewTOS's AES environment is always up, so the launcher
does *not* need the classic GEM-vs-console environment selection at load time —
the program decides its own UI. The three roles are precisely the cases that
need distinct *launcher* behavior: GUI app (nothing special), console app (text
screen), takes-params (tail dialog). "GEM with params" collapses into `.NTP`
(the tail dialog is identical). More roles can be added later without breaking
the scheme.

### Amendment 2026-09-24 — `.NAC`, and where accessories are searched

The family gained a fourth member when accessory support was planned
(ACC01/ACC06). Two user decisions came with it, both recorded here because a
worker reads this file and not the lead's notes:

1. **`.NAC` is the native accessory extension**, by the same reasoning as the
   other three: `.ACC` stays reserved for m68k so the later emulator can claim
   it. That claim is not theoretical — a classic `.ACC` could not run here
   whatever it were called, because `load_one_acc` hands the file to `pgmld`
   and `gotopgm` jumps straight into m68k machine code
   (`references/emutos/aes/gemasm.S:73-131`).

2. **Accessories are searched in two places, `C:\ACC\` first and the root
   second**, and a file name already loaded from the folder is skipped in the
   root. Dedupe is by **file name**, not by registered menu title: the title
   only exists once the accessory runs, which is long after the decision to
   load it. EmuTOS has exactly one location (`\*.ACC`, `aes/geminit.c:264-284`)
   and therefore no precedence question at all, so this ordering is **ours**,
   not an extraction — the root scan alone stays EmuTOS-compatible, and the
   folder is an addition for tidiness.

   What is **not** ours: `NUM_ACCS = 6` is a single budget covering both loaded
   files and registration slots (`include/sysconf.h:37-40` says so in its own
   comment), and `count_accs` simply stops at six with no error. The two-place
   scan inherits that one budget across both locations; it does **not** get six
   per place.

**Closed 2026-09-25 (ACC02):** `classify` has `Flavor::Accessory` for `.NAC`
and the two-place scan is `kernel/src/accs.rs::count_accs`. A double-click on a
`.NAC` deliberately does **not** run it — `launch_native`'s `Accessory` arm
says so in an alert instead, because a second copy of a running accessory would
take a second `menu_register` slot and enter a second, unreachable message
loop. EmuTOS's desktop has no launch path for an `.ACC` either; it simply
cannot reach the case, while ours can. See `docs/api/aes/accessories.md` §9.

### Amendment 2026-10-06 — `.NLD`, the native LDG library

User decision 2026-10-06 (LDG design `docs/analysis/ldg-after-arch03-2026-10-06.md`
§9 Q3, recorded by the lead; implemented in TASK/LDG-M1):

1. **`.NLD` = native LDG library** (LDG = "GEM Dynamical Libraries"). It is a
   native NewTOS ELF built exactly like a program (PIE, `sdk/libc/crt0.S`,
   `sdk/newtos.ld`; `sdk/ldg/build-ldg-demo.sh`), but it is **not a program a
   user starts**: a client loads it through libldg's `ldg_open` → `Pexec`
   5/3/4, the module's `main()` only hands its descriptor back via
   `OFFSETLDG`, and the client then calls its functions directly. Started
   without a client, an LDG module just says so
   (`sdk/ldg/ldg-r133/demo/libs/ex_lib.c:66-69`, unpacked from
   `sdk/ldg/ldg-svn-r133.tar.xz`: `form_alert` "This program is a LDG
   library").
2. **The m68k counterpart `.LDG` is reserved for the later emulator**, by the
   same reasoning as `.PRG`/`.ACC`: a classic `.LDG` is m68k code that `Pexec`
   cannot run natively, and the emulator must be able to claim the suffix.
3. **The desktop keeps the default `Doc` class for `.NLD`** — no new
   `FileType`/`Flavor`. `classify` (then kernel/src/desk/ftype.rs, now
   `programs/desktop/src/ftype.rs:80`) already maps any
   unknown suffix to `FileType::Doc` (`_ => FileType::Doc`, `:96`), so a double-click
   starts nothing. The kernel does not look at suffixes when loading
   (`Pexec` has no extension check), so nothing kernel-side is needed either.
4. The `N_P` pattern ("middle letter = role") does not fit `.NLD` (`N` + `LD`
   for LDG); the name follows the m68k suffix instead, as `.NAC` follows
   `.ACC`. Clients that search for modules by suffix (zView's `plugins.c`
   compares the last three characters with `"ldg"`) get a per-program patch
   file for `.NLD`; the upstream source stays unchanged (ADR-019 overlay rule,
   reworded 2026-10-20: overlays are app patches, not toolchain fixes).

### Routing

- **Double-click / File→Open** on a file (`open_item`):
  - native ext → build the full `C:` path (window `p_spec` drive+dir + filename)
    → `run_program` (console mode for `.NCP`, tail dialog for `.NTP`) →
    `full_redraw` on return.
  - classic m68k ext (`.PRG`/`.APP`/`.TOS`/`.TTP`/`.GTP`) → an honest
    `form_alert` "m68k programs need the emulator (not yet available)" until the
    m68k userland phase.
  - anything else → document / no-op (associated-app opening is future work).
- **AUTO-folder autorun** (`run_auto_folder`, `launch.rs`): runs native
  **console** programs (`.NCP`) only — it executes at boot BEFORE the desktop and
  AES/`appl_init`, so a GEM app (`.NAP`) must NOT run from `\AUTO\` (it needs AES).
  GEM apps auto-launch instead via the desktop `#Z` NEWDESK.INF autorun (after AES
  init). A `.PRG` in `\AUTO\` (m68k) is unsupported and skipped. *(Revised
  2026-07-20: originally `\AUTO\` also ran `.NAP`; that let a GEM app launch before
  AES — see [[singletasking-app-owns-full-screen]] and the AUTO-console-only rule.)*

### Change surface

*(Plan of 2026-07-16; the kernel/src/desk/ and kernel/src/desktop.rs paths are
from before DESK43 U8b and no longer exist — see the amendment of 2026-10-26
below.)*

- kernel/src/desk/main.rs — `open_item`: native-ext detection +
  `OpenTarget::App{path, flavor}` + path construction + route to `run_program`
  + return `full_redraw` + m68k/error alerts.
- kernel/src/desk/ — an extension-classification helper
  (`NAP`/`NCP`/`NTP`/m68k/document). The dead kernel/src/desktop.rs
  `name_is_program` is a **reference only** (do not resurrect it).
- `kernel/src/launch.rs` — `run_program` extended for a command tail (`.NTP`)
  and console mode (`.NCP`: `fbcon::activate` + restore); `run_auto_folder`
  filter to include native extensions.
- kernel/src/desk/inf.rs — NEWDESK.INF file-type/icon associations gain the
  native family alongside the m68k ones (`inf.rs:566-570`).
- **Program-build tooling — PREREQUISITE.** The old `programs/lineecho` crate +
  generated ELF are **gone**; native programs currently exist only as
  hand-crafted ELF byte arrays in the selftests (`selftest_pexec.rs`) + the
  in-kernel `registry`. To ship a *double-clickable file on C:*, either (a)
  hand-craft a minimal native ELF (a console `.NCP`: `Cconws` + `Pterm`) and
  write its bytes onto the SD image, or (b) re-establish a program-build crate
  (Rust `no_std` → stripped ELF → `make-sd-image.sh`) for a real `.NAP` GEM
  demo. (a) proves the chain now; (b) is needed for a real GEM app.
- `tools/make-sd-image.sh` — place `HELLO.NCP` (and later `HELLO.NAP`) on C:
  (+ optionally in `\AUTO\`).
- Doc/comment reconciliation — `launch.rs` / `bdos/fs` comments that say
  "`.PRG`" for the *native* launch path.

### Staged plan (Task #72)

1. Family + routing + classification helper + `.NAP` wiring (`run_program` on
   double-click) + m68k/error alerts + return redraw.
2. `.NCP` console mode (`fbcon` activate/restore).
3. `.NTP` command-tail dialog.
4. A native demo program on C: — start with a hand-crafted `.NCP` for an
   end-to-end proof; a real `.NAP` GEM demo needs the build-crate prerequisite.

### Related

ADR-013 (native `.RSC` format — the same "native, not m68k-packed" principle),
ADR-016 (the desk rebuild `open_item` lives in), and the AES shell manager
(`shel_write`/`pending_launch`, `tos-api/src/aes/shell.rs` — the AES-faithful
launch path that the kernel-driven `run_program` deliberately bypasses).

### Amendment 2026-10-26 — the launch path as built

Checked against the code of 2026-10-26. The extension family and its roles
(Decision table, both amendments) hold. The **Routing**, **Change surface**
and **Staged plan (Task #72)** sections above describe the plan of
2026-07-16 and are history:

- Since ADR-026 the desktop is the EL0 program NEWDESK and starts a program
  **only** through `shel_write(SHW_EXEC, …)` (`pro_run`,
  `programs/desktop/src/pro.rs`; `programs/desktop/src/main.rs:1068-1073`);
  the start itself is the AES shell loop's (`sh_ldapp`). The desktop no
  longer calls `run_program`, and the bypass named under Related is gone.
- Classification by extension is `classify`
  (`programs/desktop/src/ftype.rs:80-96`), with `.NAC` → accessory and the
  m68k suffixes → `FileType::M68k`.
- The AUTO folder runs `.NCP` only (`kernel/src/launch.rs:369-397`).
- `HELLO.NCP` is placed on C: by `tools/make-sd-image.sh:105`; `\AUTO\` is
  empty by default.

## ADR-019: Native C toolchain — port real Atari C libraries, don't reimplement

**Date:** 2026-07-18
**Status:** **Accepted** (user decision 2026-07-18); amended 2026-10-20
(SDK01: app patches in `sdk/overlays/<program>/` replace the "app is
sacrosanct" rule).
**Phase:** cross-cutting (SDK — `sdk/`, separate from the clean-room OS)

### Context

To run existing Atari C software (target app: **QED**, later **zView**) natively
on NewTOS, we need a C toolchain: clang-aarch64 + a libc + GEM libraries. QED
links **mintlib** (libc) + **gemlib** (GEM/AES/VDI) + **cflib** (dialogs). The
tempting shortcut — hand-write a "minimal libc" — was rejected: we have the full
LGPL sources, and reimplementing them from scratch is both wasteful and a poor
test of whether the toolchain can build real software.

### Decision

**Port, don't reimplement.** Reuse the mintlib/gemlib/cflib source unchanged and
retarget only the **thin arch/OS boundary**; compile the rest with clang. The
SDK/libc is a **separate LGPL artifact** under `sdk/` (like glibc to a kernel),
**not** part of the clean-room OS — the OS stays clean-room; the SDK reuses
ported third-party code.

The port surface, proven small for mintlib (Task #90):
- **The trap boundary** — mintlib's ~56 `trap_1_*`/`trap_13_*`/`trap_14_*` macros
  (`include/mint/{osbind,mintbind,falcon,metados}.h`), retargeted from m68k
  stack-`trap` to NewTOS register-`svc` (args left-to-right x0..x7, fn in x8,
  result x0). One change → all ~200 GEMDOS/BIOS/XBIOS bindings work unchanged.
  gemlib's `crys_if` (AES/VDI) retargets the same way to `svc #2`.
- **crt0** (`sdk/libc/crt0.S`; corrected 2026-10-26, the path first written
  here, sdk/libc/newtos-arch/crt0.S, never existed — sets SP from the BasePage `p_hitpa`,
  inits the malloc arena over the free TPA), **setjmp** (aarch64, when pulled).
- **A few honest LP64/endian fixes** to vendored files: aarch64 `byteswap.h`
  (`__builtin_bswap*`), an `is_longlong` assign-to-constant fix in `vfprintf.c`,
  and lean NewTOS `newtos-arch/{do_fstat,do_stat,ioctl}.c` that fill only
  what stdio needs (cutting the Unix time/uid/passwd tail). The first port's
  `isatty.c` shim (fds 0-2 = tty) was dropped by TASK/LIBC-TTY (2026-10-13): it
  never set `__open_stat[].flags`, so console `read()` took mintlib's RAW path;
  mintlib's own `isatty.c` (`Fcntl` -> `EINVFN`, then the `Fseek` probe) works
  since FS10.
- **Build harness** `sdk/libc/build-libc.sh` replicates mintlib's `buildrules`
  (`-D_LIBC -D_REENTRANT -DHAVE_CONFIG_H -D_GNU_SOURCE -nostdinc` + include
  order) → `libc.a`. `>8`-arg XBIOS macros (Falcon, unused) are guarded with a
  loud `_Static_assert`, not stubbed.

The 99% (printf/vfprintf, malloc, stdio buffering, string, qsort, strtol)
compiles **as-is** on top of the retargeted trap layer.

*(Superseded by the amendment of 2026-10-20 directly below.)*
**The ported app is sacrosanct.** QED (and zView) source is **not modified** —
every build failure is fixed in the toolchain (libc/gemlib/cflib/flags/headers),
never in the app. The one exception: genuine **m68k assembly** (inline `asm`/
`.S`) in the app must be retargeted to aarch64 (the arch boundary). Only after
*every* toolchain avenue is exhausted do we even discuss an app-source change.
This keeps QED an honest test that the toolchain builds real Atari software
unmodified.

> **Amended 2026-10-20 (user, SDK01; `docs/analysis/sdk-inventory-2026-10-20.md`).**
> The rule above did not survive the LP64 pivot (ADR-020 superseded): QED,
> TeraDesk and zView carry per-program patch files for LP64 widths (e.g. QED's
> rewritten `memory.c`), not only m68k asm. Those are **app patches**, not
> toolchain fixes, and the docs must say so. Rule as it stands: the upstream
> sources are kept unchanged as archives; an issue every LP64 port would hit is
> fixed once in the SDK (libc overlays, newtos-arch, headers, crt0); what one
> program needs is a patch file in `sdk/overlays/<program>/` with a header
> comment giving the reason.

### Verified

`sdk/libc/tests/libtest.c` — real mintlib `printf("%d %ld")` / `malloc`+`free` /
a `fopen`/`fwrite`/`fread`/`fclose` file round-trip on `C:` — runs on QEMU
raspi4b (`\AUTO\LIBTEST.NCP`): `-42 305419896` / `heap: heap string` /
`roundtrip OK (37 bytes)`, clean `Pterm`, then the desktop. No libc code was
hand-written. *(No measurement run directory was recorded for this; the
evidence is the test source above.)*

### Related

ADR-018 (the `.NCP`/`.NAP` native-extension family these programs use), the
multi-page TPA loader (Task #89, the prerequisite for real-size programs), and
the deferred native 64-bit ABI (Task #91 — the SDK is LP64 C over 32-bit-TOS
structs today).

## ADR-020: App ABI is ILP32 (32-bit pointers) for Atari-compat programs; LP64 native deferred

**Date:** 2026-07-19
**Status:** **Superseded** by the LP64 app-ABI pivot of 2026-07-21 (no ADR of
its own; STATUS log 2026-07-21 ★★ APP-ABI-PIVOT, now in
`docs/archive/status-log-2026-10-26.md`). The ILP32 **build path** was retired
on 2026-09-04 — see the addendum at the end of this ADR. The **runtime** ABI
switch stays, but nothing sets it any more: the loader refuses ELFCLASS32
images (`LoadError::Ilp32Unsupported`,
`tos-api/src/bdos/process/loader.rs:409-426`, since `4cbd7485`), so the
addendum's "set from the ELF class at load time" no longer holds.
**Phase:** cross-cutting (SDK toolchain + kernel loader/AES/VDI ABI boundary)

### Context

The goal is to run **unmodified** Atari C software (QED, then zView). We first
built the native toolchain LP64 (ADR-019: 64-bit pointers, the aarch64 default),
and it works — QED compiled + ran through the whole ported mintlib/gemlib/cflib
stack. But **old Atari C assumes ILP32** (`sizeof(pointer)==sizeof(long)==4`,
universal on m68k) in its *own* code. Under LP64 that breaks: QED's own bin
allocator (`sdk/apps/qed/src/memory.c`) desyncs on 8-byte pointers, and the
patch burden **grows with app complexity** (zView + FreeType/libpng/zlib/xpdf
would need many). Patching each app abandons "unmodified" and adds divergence.

### Decision

**Target the ILP32 app ABI** — 32-bit pointers — so Atari C's assumptions hold
and it compiles **unmodified**. This is the standard "32-bit apps on a 64-bit
kernel" pattern (WoW64 / Linux compat): the kernel stays LP64 internally; only
the app-facing boundary is 32-bit. Toolchain: **`aarch64-elf-gcc -mabi=ilp32`**
(4-byte pointers, ELF32) **+ `aarch64-elf-ld -m aarch64elf32`** — spike-confirmed
working. GCC is also the *native* compiler for this gcc-written code (clang
frictions like the vfprintf computed-goto vanish).

Payoffs: QED's allocator becomes correct (crash gone, no patch); the TOS structs
(BASEPAGE/DTA) are naturally 4-byte-correct (the [[lp64-tos-struct-trap]] audit,
task #98, vanishes); several LP64/clang overlays revert to pristine mintlib.

**LP64 native stays the deferred future** (task #91): a 64-bit `.NAP` path for
*new* NewTOS software that wants >4 GB — non-urgent (Pi4 RAM + a 4 MiB TPA make
32-bit's 4 GB no constraint). The loader is **dual**: ELF32 = ILP32 compat apps,
ELF64 = LP64-native apps + the embedded selftests. The two coexist.

### Change surface (staged)

1. **Foundation (done, commit `14a1f32`):** `tos-api/src/bdos/process/elf.rs` gains an
   ELF32 AArch64 parse path (class-dispatched `ClassLayout`; ELF64 byte-identical);
   sdk/build-ilp32.sh (deleted 2026-09-04). A minimal ELF32 program loads + runs in QEMU alongside
   ELF64.
2. **mintlib ILP32** — rebuild with the GNU ILP32 toolchain; drop the now-moot
   LP64 overlays (vfprintf is_longlong, labs, basepage/DTA); libtest as ELF32.
3. **gemlib + cflib ILP32** + the kernel **AESPB/VDIPB** param-block readers
   (`tos-api/src/aes|vdi`) read **32-bit** pointers for EL0 (svc #2) callers.
4. **QED ILP32 — UNMODIFIED** → runs (the `memory.c` crash is gone by construction).

### Related

ADR-019 (the LP64 toolchain this pivots — not wasted; the trap retarget, port
structure, and the crt0/loader/AES-rsrc fixes carry over), ADR-018 (the `.NAP`
family gains an ILP32/LP64 distinction by ELF class), and the app-ABI memory
[[native-64bit-abi-future]].

### Addendum 2026-09-04 — ILP32 *build path* retired, *runtime* switch kept

**Decision (user, 2026-09-04):** "den 32bit zweig stillegen weil eh nicht
läuft". This retires the ILP32 **toolchain**, not the kernel's ABI plumbing.

**Why now.** The app ABI pivoted to LP64 on 2026-07-21 (recompiled GEM apps run
as native 64-bit; ILP32 was the *cause* of the QED menu corruption, not the
cure). Since then nothing in the tree consumed an ILP32 artifact, but every SDK
change still had to be carried in **two** parallel build scripts — the 200 Hz
tick work of 2026-09-04 had to add `sdk/libc/newtos-arch/getsysvar.c` to
`build-libc.sh` *and* `build-libc-ilp32.sh`. Double maintenance for a path with
no consumer.

**Removed (tracked, 7 files):** sdk/build-ilp32.sh,
sdk/libc/build-libc-ilp32.sh, sdk/libc/crt0-ilp32.S,
sdk/gemlib/build-gemlib-ilp32.sh, sdk/cflib/build-cflib-ilp32.sh,
sdk/apps/build-qed-ilp32.sh, sdk/apps/build-teradesk-ilp32.sh. The matching
`sdk/.gitignore` entries went with them; the extracted `*-ilp32/` source trees,
build-ilp32/ object dirs and `*-ilp32.a` archives were never tracked (local
build output) and were deleted from the working tree. There is now no way to
produce an ELFCLASS32 `.NCP`/`.NAP` from this repository.

**Deliberately kept, and why.** The **runtime** ABI switch stays untouched:
`Abi::Ilp32` / `Abi::read_ptr` / `Abi::write_ptr` (`tos-api/src/bdos/process.rs`)
and the ~65 `current_abi()` / `Abi::Ilp32` sites across 17 files (mostly
`tos-api/src/aes/{object,window,form,resource,fsel,menu,graf,appl}.rs`, plus
`vdi/`, `bdos/process/`, `xbios/keytbl.rs`, `kernel/src/dbg_qedtrace.rs`), and
with them the ELF32 parse path in `tos-api/src/bdos/process/elf.rs`. Three
reasons:

1. It is the same code that reads **LP64** parameter blocks — `read_ptr` is one
   function with two arms, not an ILP32-only subsystem. This is the code that
   draws TeraDesk and QED today.
2. `Abi::Ilp32` is set from exactly one place — the ELF class at load time
   (`loader.rs`, `img.is_ilp32()`). With no ILP32 toolchain the arm is
   **unreachable in practice**, not wrong. Removing it is a separate,
   mechanical change with its own regression risk.
3. It is the reserved substrate for the **WoW64-style thunk for real m68k
   binaries** (`[[native-64bit-abi-future]]`) — the only remaining use case for
   32-bit pointers, since anything recompilable is now built LP64.

**For whoever removes the runtime switch later:** the load-time setter in
`loader.rs` is the single root; delete that and `Abi` collapses to a constant,
after which the ~65 sites fold mechanically. Two producers outside the kernel
would follow: `tools/mkrsc.py convert --ilp32` (emits the 24-byte-OBJECT NRSC
layout; its consumer is `relocate_blob_ilp32` in `aes/resource.rs`, and the NRSC
header `version` values 2/4/6 mark ILP32 width — see ADR-013), and the tracked
test source `sdk/gemlib/tests/gemtest_verify.c`, whose only builder was
`build-gemlib-ilp32.sh`. Both were left in place on 2026-09-04 as out of scope
for a build-path retirement.

---

## ADR-021: EL0 apps get GEMDOS memory via a per-process mapped window; the pool allocator stays EmuTOS-faithful

**Date:** 2026-07-20
**Status:** **Superseded by ADR-028** (2026-10-04, ARCH03): the per-process
window is gone (one shared address space, VA = PA); the EmuTOS-faithful pool
allocator this ADR kept stays.
**Phase:** cross-cutting (BDOS mem + process MMU)

### Context

Classic TOS is flat/unprotected: a program dereferences any physical address
`Malloc`/`Mxalloc` returns. NewTOS runs apps at **EL0** behind `TTBR0_EL1`; the
ported EmuTOS pool returns physical addresses mapped only in the kernel high
half, so an EL0 app faults touching them (QED `Mxalloc` → `0x0160a000` write
fault; the requirement "what an app is handed must be mapped" is carried on by
ADR-028). TT-RAM-aware apps *all* request
TT-RAM, so this hits every non-trivial app, not just QED. The TPA image loads at
a fixed low VA (non-relocating loader), and `Mshrink(basepage@VA 0)` already
fails `EIMBA` for the same address-mismatch reason.

Three sources were analysed (full extraction in
`docs/analysis/emutos-memory-model.md`): **EmuTOS** (primary — the pool model:
`MD`/`MPB`, two pools `pmd`/`pmdalt`, first-fit `ffit`+coalesce, TPA = the app
owns ~all free RAM and sub-allocates, `m_own` is a reap-tag not access control);
**FreeMiNT** (`references/freemint/`, the sanctioned MiNT-extension source — the
one TOS-family OS that solved memory protection for *unmodified* TOS/GEM apps:
one global identity-mapped ST/TT pool `sys/memory.c:210-211`, per-process page
tables holding only *protection bits* over that same layout
`sys/arch/mprot030.c`, default owner-exclusive `PROT_P` `sys/mint/mem.h:130`, an
explicit `PROT_G` for shared memory); and **NewTOS-current** (a faithful pool
port that returns raw kernel-reachable PAs + a fixed 4 MiB TPA). FreeMiNT's
identity-map relies on load-time-**relocatable** binaries (§1B.4) and so does not
transfer to NewTOS's fixed-load-VA ELF loader — but its *pattern* ("global pool;
allocate then grant the owner exclusive access; reverse on free/`Pterm`")
independently confirms the design below.

### Decision

Keep the ported EmuTOS pool allocator (`bdos/mem/{pool,traps}.rs`) **unchanged
and authoritative**. Add a thin **per-process EL0 memory manager** that, for
SVC-originated (EL0) callers, allocates from the real ST/TT pool, **maps the
block into a per-process EL0 heap window**, returns the **user VA**, and tracks
`user_va ↔ m_start` for `Mfree`/`Mshrink`/`Pterm`. This generalises the task-#99
RSC window (the RSC becomes just another mapped block) and registers the TPA as
its first entry (fixing `Mshrink(basepage)`). EL1/kernel callers keep raw PAs.
`Malloc(-1)` for EL0 reports `min(pool-free, window-free)`; ST/TT preference is
preserved because the block still comes from the requested pool.

**Offset-map now** (compact low-VA window → 32-bit-safe pointers, tiny page
tables); **identity-map with dynamic tables is the v2 target** once the
LP64/PIC-relocatable path (#91) lands — the FreeMiNT precedent is independent
evidence that identity-map is the right end-state once binaries no longer need a
fixed load VA. **`Mxalloc`'s `F_PROTMODE` bits (FreeMiNT's `PROT_P/G/S/PR/I`)
stay deferred**: continue masking them exactly as EmuTOS does
(`mode &= MX_MODEMASK`) until real multitasking needs FreeMiNT-compatible
isolation/sharing — EmuTOS, the primary source, ignores these bits too, so
deferring is spec-faithful, not a gap.

**Multitasking is the acknowledged Kernel end-goal (user, 2026-07-20).** When it
lands, much of FreeMiNT's machinery is what we adopt: the `PROT_*` protection
modes on `Mxalloc`, per-process protection over the shared pool, and `PROT_G`
inter-process shared memory. This ADR is the deliberate v1 stepping-stone toward
that — the block table already records `m_start ↔ user_va`, so extending one
entry to a *list* of `(slot, user_va)` pairs is the natural growth path to shared
memory; per-process `TTBR0` windows generalise to per-process protection tables.
The FreeMiNT analysis is therefore **not** discarded — it is the roadmap for the
multitasking phase, captured in [[freemint-extension-direction]].

### Rationale

Faithful (allocator untouched; contract preserved), uniform (image + heap + RSC
all offset-mapped through one table), and minimal (a boundary shim). NewTOS's
per-process `TTBR0` (same VA → different PA per process) is the ISA-appropriate
generalisation of FreeMiNT's per-process-table-over-one-global-layout mechanism,
not a weaker substitute. Pure identity-map cannot be uniform while the loader
places images at fixed VAs — FreeMiNT shows *why* that is a loader property
(relocatable binaries), not a limitation of this design.

### Consequences

- **Positive:** EL0 apps get dereferenceable `Malloc`/`Mxalloc` memory of the
  right RAM type; `Mshrink(basepage)` works; the RSC window is subsumed; default
  owner-exclusive access matches both EmuTOS's block-ownership model and
  FreeMiNT's `PROT_P` default.
- **Negative:** a per-process block table + window cap; an EL0 `Malloc(-1)`
  bounded by mappable space (documented adaptation); one new EL0-caller signal to
  plumb from the SVC path; **no inter-process shared memory** yet (FreeMiNT's
  `PROT_G` has no NewTOS equivalent — documented forward gap for multitasking).
- **Neutral:** returned pointers no longer equal PAs (fine — EL0 apps do no DMA).

### Alternatives considered

- **(b) Per-process physical arenas** (carve each app a private pool at `Pexec`):
  breaks ST/TT faithfulness, caps memory at load time, wastes resident RAM,
  fragments the address space. Independently corroborated as wrong by FreeMiNT
  *also* using global, not per-process, pools.
- **(c) Pure identity-map now** (VA = PA, FreeMiNT-style): impossible while the
  loader places images at a fixed low VA — needs a relocating/PIC loader first.
  Deferred to v2 (#91), where it becomes the maximally-faithful end-state.
- **Adopt FreeMiNT's full protection model now** (`PROT_*` + per-process
  protection tables over one identity space): premature — it presupposes both
  multitasking and relocatable binaries. Its *pattern* is adopted; its mechanism
  waits for the multitasking phase.

### References

- `docs/analysis/emutos-memory-model.md` (EmuTOS @1a7484ab: `umem.c`,
  `osmem.c`, `iumem.c`, `biosmem.c`, `bdosdefs.h`; FreeMiNT: `sys/memory.c`,
  `sys/arch/mprot030.c`, `sys/mint/mem.h`; NewTOS `bdos/mem/{pool,traps}.rs`).
- ADR-020 (ILP32 pointers bound the window), ADR-010 / QUIRK-001 (no TLBI before
  `eret` → fresh-VA cursor), ADR-013 (`.RSC` — the #99 window this subsumes).
- ADR-028 (the requirement EL0 memory must be mapped, now met by VA = PA);
  notes [[freemint-extension-direction]],
  [[native-64bit-abi-future]] (#91 LP64/identity-map v2).
- Tasks #99 (RSC window — subsumed), #102 (this research — closed).

## ADR-022: Desktop is a 1:1 EmuTOS-faithful rebuild that goes *through* the AES trap (refines ADR-016)

**Date:** 2026-07-22
**Status:** **Accepted** (user decision 2026-07-22). Its endgame — the
desktop as a replaceable EL0 AES application — is fulfilled by ADR-026
(2026-10-16); the 1:1 principle stands, and the code now lives in
`programs/desktop/`.
**Phase:** PHASE-14 (Desktop).

### Context

ADR-016 chartered the kernel/src/desk/ rebuild (path before DESK43 U8b) but its principle was
"reimplement the *behaviour* … do **not** port the C line-by-line … internals
free to redesign; the measuring stick is user-observable behaviour". Under that
latitude the desktop **drifted away from the AES app path**: it grew its own
`do_wredraw` loop, drew dialogs via a direct EL1 `objc_draw` call, and used a
`DESK_PAINTER` background hook — i.e. it does **not** exercise the app-facing
AES round-trip (WM_REDRAW → `wind_update` → the `WF_FIRSTXYWH/WF_NEXTXYWH`
rectangle list → clipped `objc_draw`) the way a real GEM app does.

QED — the first real EL0 GEM client — exposed the cost: its menu dropdowns
leave stale pixels (`menu_sr` save/restore area too small for `th=-1` outward
borders) while the **desktop's own** menu looks clean, because the desktop's
boxes fit `abs_rect` **and** it full-redraws after every pick. A pristine
in-kernel desktop therefore gives **false confidence** — it masks app-path bugs,
cannot be replaced by the user, and offers no reference to debug against. See
`docs/analysis/qed-redraw-rootcause.md`; the bypass itself is gone since
ADR-026.

### Decision

Rebuild kernel/src/desk/ (since DESK43 U8b `programs/desktop/`) as a **1:1 EmuTOS-faithful reimplementation** in Rust:
every EmuTOS `desk/` function maps to a **same-named Rust function that makes the
same `tos-api` (AES/VDI/GEMDOS) calls in the same order and sequence**, and the
desktop goes **fully through the AES trap** (opcode 200) like EmuDesk over
`aesbind`/`gembind` — **no** in-kernel shortcuts, **no** hooks, **no** direct-EL1
`objc_draw` for client rendering. Purely-private data structures
(`WNODE`/`FNODE`/`PNODE`/screen arena) may stay idiomatic Rust — they never cross
an ABI boundary, and the debugging benefit comes from the **call-sequence
correspondence**, not the data representation.

This is **not** a C port of EmuDesk (Rust stays Rust per the tech-stack rule; the
source-correspondence alone gives the "debug against the reference" benefit the
user wants). Two EmuTOS single-binary shortcuts are deliberately **not**
replicated (they only work because EmuTOS links `desk/`+`aes/` into one image and
would re-bypass the very AES we are validating):
- `gsx_trans` (bitmap transform, `aes/gemgraf.c:369`) → use the public VDI
  **`vr_trnfm`** trap.
- `menu_text` (locally `#define`d in `deskmain.c` to a direct `ob_spec` write) →
  use the real AES **`menu_text`** opcode.

Sequence: **validate `docs/api/desktop/` against EmuTOS first** (done 2026-07-22
across 4 chunks — a handful of real spec errors fixed, new `gembind.md`
call-surface + `deskpro.md` created, all docs now 1:1-ready), **then** rebuild
module by module — **redraw path first** (`do_wredraw` over the AES rectangle
list), then dialogs onto the app path, then menu/window/file/INF — each under the
Developer → independent Tester → lead-commit loop.

Endgame (later stage, NOT now): the 1:1 desktop becomes a **replaceable EL0 AES
app** (an EL0 GEM client), enabling an AUTO-folder / alternate desktop. In-kernel
1:1 first (smaller steps); EL0 is a follow-on.

### Rationale

- **One world:** the desktop exercises the same app-facing AES paths as real apps,
  so fixing the app path fixes *both* desktop and apps; no more masking.
- **Debuggability (the user's decisive reason):** a divergence = "our Rust fn vs
  the EmuTOS C fn" side by side, isolating a bug in *our* AES/VDI/GEMDOS, not the
  desktop. The validated spec + gembind call-surface make the correspondence exact.
- **Faithful by construction; replaceable later.**
- The validation found **no AES-server-side gaps** — every opcode the desktop
  needs already exists in `tos-api`; the rebuild mainly adds the missing
  client-side trap **wrappers** (`menu_icheck/ienable`, `objc_add/offset/order`,
  `form_*`, `graf_*`, `fsel_*`, `rsrc_*`, `shel_*`, `appl_*`).

### Consequences

- Positive: one AES world, reference-debuggable, EmuTOS-faithful, replaceable-later.
- Negative: re-work the drifted parts of the substantial existing `desk/`
  (~9200 lines); add the missing trap wrappers; extract the still-undocumented
  `desksupp.c` (`do_aopen`/`do_open`/`i_find`/`build_root_path`/`is_installed`)
  before the launch module.
- Refines ADR-016 specifically: its "behavioural, internals-free, not
  line-by-line" clause is tightened to **tight call-sequence correspondence + full
  trap routing**. The module layout and AES-client discipline of ADR-016 stand.
- Concrete constraints surfaced by validation: resolution change ⇒ **desktop
  reboot** (`shel_write(SHW_RESCHNG)`+exit), not in-place recompute; `ib_xtext=0`
  ⇒ `g_wicon = 12·char_w`; deep copy needs **≥12 concurrent DTA slots**
  (`d_doop` private-DTA-per-level); the redraw path skips `handle==DESKWH` (the
  background is the AES's `WF_NEWDESK` job, not `do_wredraw`).

### Alternatives considered

- **Keep in-kernel desk/ + patch the app path piecemeal via QED:** rejected —
  masks bugs, no reference to debug against, never replaceable.
- **Port EmuDesk C as an EL0 app:** rejected by the user — keep Rust; avoid
  EmuDesk's ROM-integration glue; the debug benefit is achievable via
  source-correspondence without running the C.
- **Global UI scaling first:** deferred (task #122) — right direction, not now.
- **Move the desktop to EL0 now:** deferred — in-kernel 1:1 first; EL0 follow-on.

### References

- `docs/api/desktop/_rebuild-plan.md` (updated), `gembind.md` (new, AES/GEMDOS
  call-surface), `deskpro.md` (new), the 4 validation passes 2026-07-22.
- `docs/analysis/qed-redraw-rootcause.md`; ADR-016 (refined), ADR-017 (resolution).
- Tasks #115 (AES=core), #123 (QED redraw), #124 (this rebuild).
- ADR-026 (the desktop moved to EL0; the bypass this ADR describes is gone);
  note [[verify-actual-call-path-before-diagnosing]].

---

## ADR-023: What the kernel hands out, the caller must be able to reach — publish kernel structures into the process under FreeMiNT's `PROT_*` modes

**Date:** 2026-09-05
**Status:** Accepted; amended 2026-09-05 (twice, below) and 2026-10-26: the
principle and the `PROT_*` vocabulary stand, the mechanism (a per-process
window, `Kind::Shared`) was replaced by ADR-028 — see Amendment 3.
**Phase:** cross-cutting (BIOS/XBIOS pointer returns + process MMU)

### Context

Several TOS calls do not return a *value*, they return a **pointer to a
structure the caller then reads**: BIOS `Getbpb` (the drive's BPB), BIOS
`Getmpb` (the memory-descriptor free list), XBIOS `Physbase`/`Logbase`,
`Keytbl`, `Kbdvbase`, `Iorec`. On a flat, unprotected Atari every such
pointer is dereferenceable by definition. NewTOS runs applications at **EL0**
behind `TTBR0_EL1` and links its kernel high-half at
`0xFFFF_0000_0000_0000 + phys` (`kernel/src/arch/aarch64/link-rpi4.ld:24`),
where the pages are EL1-only. A returned `&'static` therefore satisfies the
letter of the contract ("a pointer to the BPB") and violates its substance:
the caller cannot read it.

This was **measured**, not deduced. `sdk/apps/apitest/apitest.c` (an EL0
`.NCP` console program) got `Getbpb(2) = 0xFFFF00000026D8F4` and
`Getmpb → mp_mfl = 0xFFFF00000026D8B8` on QEMU `raspi4b` *and* on real Pi-4
hardware; the test deliberately range-checked instead of dereferencing, so it
reported the defect rather than faulting the machine. The nine BPB field
checks and the four MD field checks behind those two pointers were
consequently unrunnable. The gap had never surfaced because every previous
caller — the FAT mount, the desktop, the boot self-tests — runs at EL1, where
the kernel VA *is* the readable address.

The situation is the mirror image of ADR-021. There the kernel handed an app
a *pool* address it could not reach and the answer was: map the block into a
per-process EL0 window and return the user VA. Here the kernel hands an app
one of *its own* structures. The mechanism should be the same one, not a
second one.

Sources consulted: EmuTOS (`bios/blkdev.c:569-777` `blkdev_getbpb`,
`bios/biosmem.c:134-144` `getmpb`/`themd`, `bios/ikbd.c:122-134` `keytbl`) for
what the contract is; **FreeMiNT** (`references/freemint/`, the project's
sanctioned MiNT-extension source) for the protection model; NewTOS current
code (`tos-api/src/bdos/process/{usermem,mmu}.rs`, `tos-api/src/bios/`).

### Decision

**Adopt FreeMiNT's protection model — its names and its values — instead of
inventing a NewTOS one** (user decision, 2026-09-05: *"wir müssen das Rad
nicht neu erfinden sondern können das verwenden"*). The constants are taken
verbatim from `references/freemint/sys/mint/mem.h:120-136` into
`tos-types::memory`: the external `F_PROTMODE`/`F_PROT_P`/`F_PROT_G`/
`F_PROT_S`/`F_PROT_PR`/`F_PROT_I`/`F_PROTSHIFT` nibble of an `Mxalloc` mode
word (**correction, ARCH03 stage 3a:** these are the *PRGFLAGS* values; in an
`Mxalloc` mode word the nibble is one higher — `0x10` = `PROT_P`, `0x00` =
"the caller's PRGFLAGS", FreeMiNT `sys/mint/mem.h:118`, `sys/dosmem.c:209-221`;
decoders `tos_types::prot_from_prgflags`/`prot_from_mxalloc`), and the internal `PROT_P`/`PROT_G`/`PROT_S`/`PROT_PR`/`PROT_I` values
FreeMiNT passes to `alloc_region()`, typed as `tos_types::ProtMode`. Later
work can therefore *extend* FreeMiNT's model rather than replace ours.

On that vocabulary, add **one** new primitive, inside the ADR-021 window:
`bdos::process::usermem::expose(slot, kernel_va, len, prot) -> Option<u64>` —
map the physical page(s) of a `'static` kernel object into the calling
process's EL0 window under a `PROT_*` mode and return the **user VA**. It is
idempotent per physical page (a `Getbpb` loop over 16 drives costs one
mapping), and the published block is `Kind::Shared`: the process may read it
but can never `Mfree`/`Mshrink` it.

Assignment for the two measured cases: structures a program only **reads**
get `PROT_PR` (read-only, `UserRoNx`). `Getbpb` and `Getmpb` use it. Each
published structure is padded and aligned to own its page, so the page a
program can read holds exactly the published data and no other kernel datum.
EL1 callers are unchanged: with no running process the kernel VA is returned.

**Explicitly out of scope, and deliberately so: FreeMiNT's full memory
protection** — processes that cannot read one another, enforced `Mxalloc`
`F_PROT*` modes, killing a process on trespass. ADR-021 deferred that and it
stays deferred; this ADR takes only the small half: *what we hand out, the
recipient must be able to reach.* The large half remains its own project, and
it now has our vocabulary already aligned to build on.

### Rationale

- **One mechanism, not two.** `expose` is 60 lines in the module that already
  owns the process's low-half VA space, reuses its bump cursor (fresh VAs ⇒ no
  TLBI ⇒ QUIRK-001 stays avoided), its block table and its teardown.
- **Their names, so their code can follow.** Writing `PROT_PR` where FreeMiNT
  writes `PROT_PR` means the eventual protection work is an extension, not a
  migration. Inventing `NewTosAccess::ReadOnly` would have guaranteed a
  rename later.
- **Page-owning statics.** The MMU's granule is a page; without padding,
  publishing an 18-byte table would also publish ~4 KiB of neighbouring
  kernel BSS. Two `const _: () = assert!(size == 4096)` make that structural
  rather than hopeful.
- **Failure is NULL, not a bad pointer.** If the window cannot take the
  mapping, `Getbpb` returns `0` and `Getmpb` leaves `mp_mfl` NULL — both are
  values the contract already defines. Returning an address the caller would
  fault on is never an answer.

### Consequences

- **Positive:** the `Getbpb`/`Getmpb` contract is now satisfied for EL0, which
  is the only kind of caller a real application is. Measured: `APITEST`
  BB18/BS06 pass and the 14 field checks behind them now run and match the
  boot sector and the `membot`/`memtop` sysvars — 66 PASS / 2 FAIL / 7 SKIP
  became **82 PASS / 0 FAIL / 5 SKIP** (the 5 residual SKIPs are the
  interactive keyboard checks, structurally impossible under QEMU `raspi4b`).
  The `PROT_*` vocabulary is in place for the multitasking phase.
- **Negative:** one 4-KiB page of kernel BSS per published structure, one
  window page per publishing process, and one more thing to remember when a
  syscall returns a pointer. The rest of the class is **not** fixed by this
  ADR — see the survey in `docs/api/bios/07-getbpb.md` and the STATUS entry.
- **Neutral:** a published pointer is no longer the kernel's own address, so
  two callers at different privilege levels legitimately see different
  numbers for the same structure. That is invisible to a conforming program,
  which only ever compares a pointer with the one it was given.

### Alternatives considered

- **Copy the structure into the caller's reserved low memory** (the other
  option `docs/api/bios/07-getbpb.md` had named). This is what `Keytbl`
  already does — it materialises `struct keytbl` plus its six tables at
  `KEYTBL_AREA_VA = 0x768` inside the process's own page 0
  (`tos-api/src/xbios/keytbl.rs:217-234`), which is why `Keytbl` never had
  this defect. Rejected here because a copy cannot be `PROT_PR`: page 0 is the
  program's own read/write TPA page, so the app could scribble on its BPB;
  and the reserved low region is 2.4 KiB of a shared page, too small to grow
  into. The precedent is acknowledged, not extended.
- **Return the physical address** (classic-TOS-style flat pointer). The app
  has no identity mapping; a PA is as unreachable as the kernel VA.
- **Map the whole kernel image read-only into every process.** Fixes every
  case at once and gives every app a full read of kernel memory — a protection
  model going backwards, and it makes `PROT_*` meaningless.
- **Adopt FreeMiNT's full protection model now.** Same answer as ADR-021: it
  presupposes multitasking and relocatable binaries. Its *vocabulary* is
  adopted now precisely so its *mechanism* can be adopted later.

### References

- FreeMiNT `references/freemint/sys/mint/mem.h:120-136` (the constants, taken
  verbatim); `sys/memory.c`, `sys/arch/mprot030.c` (the model this prepares
  for).
- EmuTOS `bios/blkdev.c:569-777` (corrected 2026-10-26 from 589-756; the
  function starts at 569), `bios/biosmem.c:134-144`, `bios/ikbd.c:122-134`.
- Specs `docs/api/bios/07-getbpb.md`, `docs/api/bios/00-getmpb.md` (both had
  the gap recorded as "open, a design decision, not taken here" — now closed).
- Code: `tos-types/src/memory.rs` (`ProtMode`, `F_PROT_*`, `PROT_*`),
  `tos-api/src/bdos/process/usermem.rs` (`expose`, `Kind::Shared`),
  `tos-api/src/bdos/process/mmu.rs` (`PageFlags::UserRoNx`),
  `tos-api/src/bios/bpb.rs`, `tos-api/src/bios/getmpb.rs`.
- Measurement: `sdk/apps/apitest/apitest.c` checks BB17..BB28, BS03..BS10.
- ADR-021 (the window this extends; deferred `F_PROTMODE` — partially lifted
  here for the publish direction only), ADR-010, QUIRK-001.
- ADR-028 (EL0 memory must be mapped — met by VA = PA); notes
  [[freemint-extension-direction]],
  [[framebuffer-outside-st-tt-ram]] (why `Physbase` is the hard remainder).

### Amendment, 2026-09-05 — `Physbase`/`Logbase`, and the cacheability half of a published mapping

The "hard remainder" is closed. XBIOS `Physbase` (0x02) and `Logbase` (0x03)
now publish the framebuffer into the calling process's window and return the
**user VA**; an EL1 caller still gets the kernel VA. Two things had to be
decided that `Getbpb`/`Getmpb` did not raise:

**Mode: `PROT_G`, not `PROT_PR`.** These calls exist so a program can *write*
pixels. A read-only screen pointer would be reachable and still useless.

**Memory type: Normal Non-Cacheable.** `expose` grew a second axis —
`usermem::MemAttr` (`Cacheable` / `Uncached`) — because protection alone does
not decide whether a store *arrives*. The framebuffer's consumer is the
VideoCore display engine, a DMA master that scans DRAM from outside the ARM
caches (`docs/hardware/mailbox-framebuffer.md` §6; the same fact that made
the kernel's own alias first Non-Cacheable and later Write-Back-plus-clean,
`kernel/src/drivers/bcm2711/framebuffer.rs`). The kernel can clean after every
drawn rectangle; an EL0 program cannot, and the TOS API has no
cache-maintenance call to give it — inventing one would push a machine detail
into every direct-drawing program. So the published pages are AttrIdx 2
(Normal Non-Cacheable, MAIR `0x44`): a plain `*p = colour` reaches DRAM, and
the VideoCore reads it. **The decision belongs to the mapping, not to the
program.**

That leaves the kernel's Write-Back alias and the program's Non-Cacheable one
pointing at the same physical pages — **mismatched memory attributes**, where
the architecture only guarantees coherency if software keeps the region out
of the caches. Two measures, both kernel-side:

1. `expose_attr` cleans **and invalidates** the whole published span through
   the kernel alias when it creates an `Uncached` mapping, so no line
   (dirty *or* clean) survives from whatever drew there before.
2. While any process holds such a mapping
   (`usermem::uncached_shared_active`, a slot bitmask cleared on
   `release_slot`), the VDI backend's per-primitive flush becomes
   `dc civac` instead of `dc cvac` (`kernel/src/vdi_backend.rs`). Without
   this, a later *partial* kernel write to a retained line would write the
   whole 64-byte line back and silently undo the program's own stores. The
   cost is paid only while a program is actually drawing into the screen
   itself; the desktop's path is untouched.

Measured (QEMU `raspi4b`, `PHYSBASE.NCP` from `C:\AUTO\`): `Physbase() =
Logbase() = 0x00400000` — the process's window base, previously
`0xFFFF0000…`. The program stores three colour bands with **no cache
maintenance of any kind** and the QMP screendump shows them; a subsequent
`Cconws` renders over them correctly, which is the mixed cached/uncached case.

**Not done here, deliberately:** `Setscreen` (0x05) still only writes the
`PHYSBASE`/`LOGBASE` shadows — verified 2026-09-05 by reading every reader of
those two atomics: the raster backend draws into, and the VideoCore scans out,
the live framebuffer regardless. There is therefore no double buffering. Real
flipping needs a double-height virtual buffer at allocation time, the mailbox
set-virtual-offset tag, and a user-VA → physical translation for a
caller-supplied base — its own work item.

Code: `tos-api/src/xbios/screen.rs` (`fb_base`, `direct_screen_active`),
`tos-api/src/bdos/process/usermem.rs` (`MemAttr`, `expose_attr`,
`uncached_shared_active`, `clear_uncached`),
`tos-api/src/bdos/process/mmu.rs` (`PageFlags::UserRwNxUncached` /
`UserRoNxUncached`, `clean_invalidate_dcache_range`),
`kernel/src/arch/aarch64/mmu.rs` (`clean_invalidate_dcache_range`),
`kernel/src/vdi_backend.rs`, `kernel/src/host_env.rs`,
`tos-api/src/host.rs` (`ScreenInfo::stride_bytes` / `size_bytes`),
`sdk/examples/physbase.c`.

### Amendment 2, 2026-09-05 — `Setscreen` really switches: two screen pages and the VideoCore virtual offset

Amendment 1 closed with *"`Setscreen` (0x05) still only writes the
`PHYSBASE`/`LOGBASE` shadows … there is therefore no double buffering"*.
That is now closed too, and the shadows are gone with it.

**Both bases live where EmuTOS keeps them, not in a shadow.** The logical
base *is* the display's draw pointer — the one the VDI raster backend
renders through, which is what `v_bas_ad` is (`bios/screen.c:1071-1073`,
`logbase`) — and
the physical base is read back **from the hardware**, the way EmuTOS'
`physbase()` reconstructs it from the video-base registers
(`bios/screen.c:1042-1049`, which calls `atari_physbase` at `:917`;
corrected 2026-10-26 from `:1030`). Deleting the two shadows removed a
second source of truth: a program that asks after a `Setscreen` is now told
what the machine is doing, not what it requested.

**The hardware.** The VideoCore framebuffer is allocated with **double
virtual height** (`0x0004_8004` with `h × 2`), so its screen memory holds two
whole screens, and the physical base moves with the **set-virtual-offset**
property tag `0x0004_8009` (`u32 offset_x, u32 offset_y`). Tag id and value
layout are taken from `references/circle`
(`references/circle/include/circle/bcmpropertytags.h:65,282`,
`references/circle/lib/bcmframebuffer.cpp:33,386`, corroborated by
`references/circle/addon/linux/raspberrypi-firmware.h:103`) and written up in
`docs/hardware/mailbox-framebuffer.md` §3b — CLAUDE.md Rule 6, nothing from
memory. The driver believes the virtual height the firmware **echoes** and
the byte size the allocate tag **granted**, not the numbers it asked for,
and falls back to a single-page framebuffer if the double-height allocation
fails: a display that boots is worth more than one that flips.

**A base the display cannot use is not applied — and the read-back says so.**
Only a whole screen page inside the VideoCore's own screen memory can be
scanned out; NewTOS cannot make an arbitrary caller-allocated buffer the
physical screen, and that is a property of the display, not a shortcut. The
answer is neither an invented error code nor a silent lie: the base stays
where it was, exactly as a plain ST's video-base register cannot represent
the low byte of an address and silently drops it (`atari_setphys`,
`bios/screen.c:939-947`), and `Physbase`/`Logbase` — which read the hardware
— tell the caller the truth. EmuTOS has no error return for a bad base, so
inventing one would have been the deviation.

**Three consequences worth remembering.**

1. **`Physbase`/`Logbase` publish the *whole* screen memory** (both pages)
   into the calling process as one `PROT_G` Normal-Non-Cacheable block, not
   just the visible screen. A program double-buffers by computing
   `Logbase() + page_size` and handing that back to `Setscreen`; that
   arithmetic is only valid in the process's own address space if the pages
   are one mapping.
2. **The EL0 heap window had to grow** (`USER_L3_COUNT` 10 → 16, 16 MiB →
   28 MiB). At 1920×1080×4 the published screen memory is 15.8 MiB; with the
   old 16-MiB window a program that took the screen address had ~184 KiB
   left for `Malloc`. The screen mapping is a real user of the window, so
   the window has to be sized for it. Cost: +384 KiB of kernel BSS, against
   1.4 MiB of headroom below `STRAM_OS_RESERVE` (the linker `ASSERT` guards
   it).
3. **A syscall that takes a pointer back needs the inverse of ADR-023.**
   `expose` answers "where in the process can this kernel object be reached";
   `usermem::resolve_kva` answers the other direction, by containment and
   including `Kind::Shared` blocks. `Setscreen` is the first caller and will
   not be the last.

**Measured** (QEMU `raspi4b`, `SCRFLIP.NCP` from `C:\AUTO\`): the firmware
grants `virtual 640×960` for a `physical 640×480` display, 2400 KiB =
2 screen pages; the program finds page 1 purely through the
`Setscreen`/`Logbase` contract at offset `0x12C000`, paints it while
invisible (screendump: console text only), flips it in (screendump: three
full-screen colour bands, nothing copied), and restores both bases so the
boot goes on to the desktop. `Vsync` was measured in the same run: 600 calls
in 9.9 s = 60.6 Hz, so it is a real wait.

**Not done, deliberately:** binding `Vsync` to the firmware's own
wait-for-vsync tag `0x0004_800E` (documented, unused — it would turn a
timer read into a blocking mailbox round trip in the trap path); a second
ramfb page on `board-qemu-virt` (its pixel store is a fixed reservation off
the top of DRAM, so a second page shrinks the ST/TT-RAM pools — a
pool-layout decision, not part of this); and **any way for a program to
learn the screen page size other than probing** — see the open question in
`docs/STATUS.md`.

Code: `kernel/src/hal/framebuffer.rs` (`mem_base`/`pages`/`set_draw_base`/
`scanout_base`/`set_scanout_base`),
`kernel/src/drivers/bcm2711/framebuffer.rs` (`WANTED_PAGES`,
`set_virtual_offset`), `kernel/src/vdi_backend.rs`, `kernel/src/host_env.rs`,
`tos-api/src/host.rs` (`ScreenInfo::{phys_base,mem_base,mem_bytes}`,
`set_screen_logical`/`set_screen_physical`),
`tos-api/src/xbios/screen.rs`, `tos-api/src/bdos/process/usermem.rs`
(`resolve_kva`), `tos-api/src/bdos/process/mmu.rs` (`USER_L3_COUNT`),
`sdk/examples/scrflip.c`, `docs/hardware/mailbox-framebuffer.md` §3b.

### Amendment 3, 2026-10-26 — the mechanism after ARCH03 (ADR-028)

Checked against the code of 2026-10-26. The Decision's principle ("what the
kernel hands out, the caller must be able to reach"), the FreeMiNT `PROT_*`
vocabulary, the page-owning statics, NULL on failure, `MemAttr` and the
uncached publication all stand. The **mechanism** described above and in
the two amendments is gone with ADR-021's window:

- There is no per-process window, no bump cursor, no block table and no
  `Kind::Shared`. Since ADR-028 every user address is its physical address.
  `expose` (`tos-api/src/bdos/process/usermem.rs:180`) enters the page(s)
  into the **shared** user tables at VA = PA under the `PROT_*` mode and
  returns the **physical address**; a publication is the same for every
  process (module note, `usermem.rs:14-24`).
- So the measured `Physbase() = Logbase() = 0x00400000` of Amendment 1 was
  the window base of the day; today the call returns the framebuffer's
  physical address.
- `USER_L3_COUNT` (Amendment 2, consequence 2) no longer exists; there is no
  window to size.
- `MemAttr` (`usermem.rs:70`), the `PROT_G` + Non-Cacheable mapping
  (`usermem.rs:225`, `PageFlags::UserRwNxUncached`), `uncached_shared_active`
  and `resolve_kva` (`usermem.rs:308`) remain as described.

---

## ADR-024: VDI truecolor workstation — Falcon 16-bit RGB565 device pixel over a 32-bpp XRGB8888 physical framebuffer

**Date:** 2026-07-22 (lands, `09c9ddc`; block complete 2026-07-23, `11ca57e`).
**Documented retroactively 2026-09-21 (A101)** — the decision shipped without
a contemporaneous ADR; this entry records it after the fact from the landed
code and the design doc that preceded it.
**Status:** Accepted (implemented and shipped).
**Phase:** cross-cutting (VDI workstation/attributes/raster backend, AES
`appl_init` `ap_planes`).

### Context

NewTOS's committed EmuTOS reference build profile is **Falcon030/TOS4**
(`make 512`, the full `CONF_WITH_*` set — the lead's note
[[emutos-target-profile-falcon-tos4]], which is not in this file), confirmed statically in
`docs/analysis/aes-audit/emutos-config-profile.md`: `CONF_WITH_VDI_16BIT`
defaults to `1` in that profile (config.h:1628-1630) and **requires**
`CONF_WITH_VIDEL=1` — EmuTOS hard-errors the build otherwise
(`include/config.h:2199-2200`,
`docs/analysis/aes-audit/emutos-config-profile.md:80,150`). The profile's
own canonical on/off list states the consequence plainly: "`CONF_WITH_VIDEL`
+ `CONF_WITH_VDI_16BIT` | 1 / 1 | VDI-16-Bit-Truecolor-Modi MÜSSEN real sein"
(`emutos-config-profile.md:314`).

Before this decision NewTOS's VDI backend was a bring-up **16-colour**
palette model: a fixed 16-entry GEM palette
(`kernel/src/vdi_backend.rs`, pre-`09c9ddc`, `DEFAULT_PALETTE: [(u8,u8,u8);
16]`, `NUM_COLORS = 16`) — a legitimate EmuTOS mode (ST/STe 4-plane), but not
the profile NewTOS had committed to.

EmuTOS's truecolor VDI is gated purely on `v_planes > 8`
(`TRUECOLOR_MODE`, `vdi/vdi_defs.h:22`), and the only concrete truecolor
value any real EmuTOS target reports is **16**: `get_videl_bpp`
(`bios/videl.c:320-341`, `SPS_HICOLOR → bits_per_pixel = 16`, reached from
`videl_get_current_mode_info`, `:969-974`). There is **no 24-/32-bit VDI
path in EmuTOS at all** — the truecolor branch is everywhere bound to
`v_planes > 8`, and 16 is the only value it is ever fed
(`docs/analysis/vdi-audit/truecolor-conversion-plan.md` §0/§1.1, Researcher,
2026-07-22 — the design doc this ADR retroactively records the decision
for).

Trigger: a user decision, first recorded in the lead's session memory
(vdi-truecolor-workstation; this ADR is now its record) — "convert VDI
from 16-color palette to Falcon truecolor (EmuTOS `CONF_WITH_VDI_16BIT` =
16-bit RGB565; backend maps to 32-bpp)".

### Decision

NewTOS's VDI presents a **Falcon truecolor workstation**, matching EmuTOS's
`CONF_WITH_VDI_16BIT` semantics exactly:

1. **Device pixel** (the VDI-visible colour unit — what a program's
   memory-MFDB, `v_get_pixel`, and the pen model address) is EmuTOS's 16-bit
   RGB565-like word, `RRRRRGGGGG0BBBBB` (green LSB is `OVERLAY_BIT`,
   `vdi/vdi_defs.h:153-158`), reached through **256 logical pens**
   (`NUM_COLORS = 256`, `kernel/src/vdi_backend.rs:36`) and **16 planes**
   (`PLANES = 16` = `v_planes`, `:43`) — both figures satisfy EmuTOS's own
   formula simultaneously (`DEV_TAB[13] = (v_planes<8)?(1<<v_planes):256`,
   `vdi_control.c:206`; `INQ_TAB[4] = v_planes`, `:210`).
2. **Pens 0..15 keep the classic GEM-16 palette values**
   (`DEFAULT_PALETTE`/`VIDEL_PALETTE1`) so existing desktop/AES rendering
   does not regress; **pens 16..255 seed from EmuTOS's own
   `videl_palette2[240]`** default (`vdi/vdi_col.c:137-199`), copied
   verbatim.
3. **The physical Pi framebuffer stays a 32-bpp XRGB8888 store** — a
   NewTOS-internal implementation detail, not part of the VDI contract
   (`docs/COMPATIBILITY.md`), because the BCM2711 VideoCore mailbox
   interface has no native 16-bit-truecolor allocation mode to target
   directly. Two conversion points bridge the two: `pal555_to_xrgb`
   (draw → screen) and `device_to_rgb565`/`rgb565_to_device` (screen →
   16-bit domain — lossless, because the screen is itself 5-bit-quantised)
   (`kernel/src/vdi_backend.rs:174-182,397-424` at the time). *(2026-10-26:
   `pal555_to_xrgb` no longer exists; the pair is now `word_to_device` /
   `device_to_word` (`kernel/src/vdi_backend.rs:199-218`, which also carry
   the overlay bit) behind `device_to_rgb565` / `rgb565_to_device`
   (`:479-498`).)*
4. **The app-visible memory-MFDB wire format** (`fd_stand=0`,
   `fd_nplanes=16`) is real packed **16 bits/pixel**, not the prior 32-bpp
   chunky model — this is **contract**, not internal: a Falcon-truecolor
   program that builds its own MFDB and pokes RGB565 pixels directly (image
   viewer, sprite editor) observes exactly this byte layout, because
   `vro_cpyfm16`/`vrt_cpyfm16` operate raw on `UWORD*`
   (`vdi/vdi_raster.c:975-1221,1226-1326`).
5. **`ap_planes`** (AES globals) follows `planes()` (= 16), not
   `log2(num_colors())`, matching EmuTOS `gemsuper.c:97` /
   `gemgraf.c:395` / `gemgsxif.c:497`.

### Rationale

- **Matches the committed reference profile, not an arbitrary richer
  palette.** `CONF_WITH_VIDEL` + `CONF_WITH_VDI_16BIT` = 1/1 in NewTOS's
  Falcon030/TOS4 reference build (`emutos-config-profile.md:150,314`);
  EmuTOS itself refuses to build `VDI_16BIT` without `VIDEL`
  (`include/config.h:2199-2200`), so this is one package, not two independent
  choices.
- **EmuTOS has exactly one truecolor VDI shape** (16-bit, `v_planes > 8`);
  there is no design freedom in the device-pixel format once "truecolor" is
  chosen (`vdi_defs.h:22`, `bios/videl.c:320-341`). A 24-/32-bit VDI device
  pixel would be inventing an EmuTOS mode that does not exist — CLAUDE.md
  Rule 1 forbids writing spec/code from memory of "what a modern truecolor
  VDI could do" instead of the source.
- **The contract/internals split is already established practice**
  (`docs/COMPATIBILITY.md`): the BCM2711 VideoCore has no 16-bit truecolor
  allocation mode, and forcing the physical store to 16-bit would trade a
  clean internal detail for no observable benefit — the two conversion
  points are the entire cost of keeping the 32-bpp physical store.
- **3D rendering quality is itself colour-count-gated in EmuTOS**: `init_3d`
  (`aes/gemoblib.c:1327-1343`) selects `LWHITE` (= 8, `obdefs.h:117`) over
  plain `WHITE` when `gl_ws.ws_ncolors > LWHITE` (`gemoblib.c:1336`), i.e.
  AES's own 3D look already assumes headroom above a small fixed palette.
  **Inferred, not directly EmuTOS-cited:** at exactly 16 pens this specific
  branch was already satisfied (16 > 8), so this branch alone was not
  literally broken under the old 16-colour model — the practical argument
  for 256 pens is headroom **beyond** the fixed GEM-16 (colour icons, any
  future extended 3D/desktop colour use), not this one branch; see
  Alternatives considered for the honest version of this point.

### Consequences

- **Positive:** `DEV_TAB[13] = 256`, `INQ_TAB[4] = 16`
  (`tos-api/src/vdi/workstation.rs`) match D-VDI-7
  (`docs/api/vdi/_discrepancies.md`, resolved 2026-09-04) exactly — no
  further VDI-side change needed for that discrepancy.
- **Positive:** pens 16..255 give 240 free pens above the fixed GEM-16 for
  colour icons (`CONF_WITH_COLOUR_ICONS`, default on in the reference
  profile) and any future extended 3D/desktop colour use, without evicting
  or reinterpreting the 16 pens desktop/AES already draw with.
- **Negative:** the app-visible memory-MFDB wire format changed from 32-bpp
  chunky to packed 16-bit RGB565 (TC3b, `3d4ab71`) — a structural,
  **contract-level** change, invalidating any prior assumption of a 32-bpp
  memory-MFDB.
- **Negative:** the palette moved from one backend-global array to a
  per-`Vwk` structure (`VwkExt.req_col`/pseudo-palette, TC2, `abb2280`) —
  more state per open workstation, matching EmuTOS's own per-workstation
  palette (`vdi_col.c:601-614`).
- **Neutral:** the physical Pi framebuffer format (32-bpp XRGB8888) is
  unchanged; only the VDI-visible device-pixel model changed.

### Alternatives considered

- **Keep the 16-colour/4-plane bring-up model.** Rejected: does not match
  the committed Falcon030/TOS4 reference profile (`CONF_WITH_VDI_16BIT=1`),
  and pens 0..15 alone leave no headroom for colour icons or any
  per-application extended palette without contending with the desktop's
  own fixed GEM-16 assignments — that headroom argument is the concrete
  cost, not a broken `init_3d` branch (see Rationale's inferred caveat).
  **Not independently re-verified for this ADR:** whether TeraDesk's actual
  `cicons.rsc` icons — each measured as a single 4-plane/16-colour on-disk
  variant (`docs/api/aes/resource-fileselector.md:905-909`) — would
  visibly clash with the desktop's own colours under the old 16-pen model;
  flagged as the assumption behind this alternative's rejection, not a
  measured regression.
- **A 24-/32-bit VDI device pixel** (bypass EmuTOS's 16-bit model, expose
  the physical 32-bpp store directly to programs). Rejected: EmuTOS has no
  such mode (`vdi_defs.h:22`); this would be inventing TOS API behaviour
  from memory of "what a modern truecolor VDI could do", which CLAUDE.md
  Rule 1 forbids — the EmuTOS source is the specification, and it defines
  exactly one truecolor shape.
- **Ship geometry independence and truecolor together** (as one combined
  ADR-017 change). Rejected at the time (ADR-017 "Alternatives considered":
  "Ship geometry independence and real truecolor together: rejected —
  bundling maximizes scope/risk for one change"): truecolor landed six days
  after geometry independence (`09c9ddc`, 2026-07-22, vs. `c95ad37`,
  2026-07-16) as its own, separately-verified change.

### References

- EmuTOS: `vdi/vdi_defs.h:22` (`TRUECOLOR_MODE`), `:153-158`
  (`VwkExt.palette[256]`, device-pixel bit layout), `vdi/vdi_control.c:
  206,210` (`DEV_TAB[13]`/`INQ_TAB[4]` formulae), `vdi/vdi_col.c:137-199,
  601-614` (`videl_palette2`, `set_color16`), `vdi/vdi_raster.c:975-1221,
  1226-1326` (`vro_cpyfm16`/`vrt_cpyfm16`, raw `UWORD*`), `bios/videl.c:
  320-341` (`get_videl_bpp`), `aes/gemoblib.c:1327-1343` (`init_3d`),
  `include/config.h:1628-1630,2198-2201` (`CONF_WITH_VDI_16BIT` default +
  `CONF_WITH_VIDEL` hard-error).
- NewTOS landing commits (all 2026-07-22/23): `09c9ddc` (TC1, foundation —
  `planes()`/256-palette/`DEV_TAB`/`INQ_TAB`), `abb2280` (TC2, per-`Vwk`
  palette + `vs`/`vq_color16`), `2f5cda2`/`3d4ab71` (TC3a/b, 5-bit draw path
  + 16-bit memory-MFDB format), `516dc9a` (TC4, `vr_trnfm16`), `c7fcff2`
  (TC5, cursor save-under via device pixels), `11ca57e` (TC1-TC5 complete +
  V1 re-green). Code: `kernel/src/vdi_backend.rs:29-90` (`NUM_COLORS`,
  `PLANES`, `VIDEL_PALETTE1`/`2`), `tos-api/src/vdi/workstation.rs:24-90,
  141-245`.
- Design doc: `docs/analysis/vdi-audit/truecolor-conversion-plan.md`
  (Researcher, 2026-07-22).
- Profile: `docs/analysis/aes-audit/emutos-config-profile.md:80,150,314`.
- Resolved discrepancy: `docs/api/vdi/_discrepancies.md` D-VDI-7
  (2026-09-04).
- Spec sections corrected to cite this ADR (A101, 2026-09-21):
  `docs/api/vdi/workstation/102-vq_extnd.md` (placeholder
  `[ADR-0xx truecolor VDI]` replaced), `docs/api/vdi/raster/_raster.md`
  §"Pi/BCM2711 binding" (A86, 2026-09-21, pointed at the then-missing ADR),
  `docs/HARDWARE-PROFILE.md` §3.1/§4.
- ADR-017 (geometry independence — the sibling, deliberately separate
  decision; its "Colour truecolor is deliberately not bundled with this
  change" / "Deferred: real truecolor VDI remains a documented future step"
  is what this ADR fulfils).
- Notes: [[vdi-draw-asm-optimization-later]] (a later, unaffected step).
  The triggering user decision is recorded in this ADR (Context).

---

## ADR-025: The MD supply grows page by page within a page budget; a per-process block limit replaces the fixed pool

**Date:** 2026-10-13
**Status:** Accepted (MEM05 part A, kernel; part B, SDK: mintlib `MAXHUNK` 16 MiB via the overlay `sdk/libc/overlay/mintlib/malloc.c`, `docs/sdk/07-mintlib-limits.md` §10)
**Phase:** cross-cutting (BDOS mem, memory protection, kernel heap)

### Context

Since E9 the libc takes heap from GEMDOS `Malloc` (mintlib `__sbrk`), one
BDOS block per hunk of 8–32 KiB. The MD supply was a static arena of 86
`MDBlock`s = 258 MDs (`tos-api/src/bdos/mem/mdpool.rs`); `malloc(32)` ×
147 605 used every MD in the system and `malloc` returned NULL with ~930 MB
free (`measurements/measurement-2026-10-06-post-e9/RUNINFO.TXT:79-82`).
EmuTOS draws its MDs from a fixed OS pool of 118 blocks, at most 354 MDs,
shared with DND/OFD (`bdos/osmem.c:36-39,62-64,104-109`); TOS 1.0 had
"20 blocks per process" (`doc/tos14fix.txt:822-833`). Analysis
`docs/analysis/mem05-md-pool-2026-10-13.md`, design
`docs/analysis/mem05-design-2026-10-13.md` (approved with E1–E8), follow-up
decision B (`TASK/MEM05.md`, "Nachentscheidung … SNAP im Seitenbudget").

### Decision

1. **The MD supply grows** page by page (39 blocks = 117 MDs per 4-KiB
   page) from the **kernel heap**; the first page is static (boot, host
   tests). Pages never move and are never given back (E8). `xmfremd` is
   O(1) over the slot index, as EmuTOS' (`osmem.c:229-243`).
2. **Every table with one entry per allocated block grows with it** —
   the protection records (`memprot` `RECS`), the per-sync snapshot of all
   blocks (`SNAP`), the `F_KEEP` flags (`region` `KEPT`) — stored densely
   (scan cost = live entries), with room for the next block secured in
   `mxalloc` **before** `ffit`, so a block that exists always has its
   record. The `memprot=verify` scratch (`VSCRATCH`) is a fixed 512
   (blocks per 2-MiB region at page grain), no longer tied to the supply.
3. **One page budget** (`tos_api::bdos::mem::pages::MD_PAGE_BUDGET` = 512
   pages = 2 MiB) covers MD pages, records, snapshot and flags together
   (decision B). Budget reached or heap empty = the old exhaustion path:
   `Malloc`/`Mxalloc` 0, `Mshrink` `ERROR`, `Maddalt` `ENSMEM`,
   `Pexec` `ENSMEM`.
4. **Kernel heap 2 → 8 MiB** (E2), so the 2 MiB of MEM05 cannot starve
   desktop and VDI.
5. **Per-process limit** of allocated blocks (EmuTOS tag `m_own`), default
   8192 (E3), boot argument `mdlimit=<n>` (`0` = off, E4); kernel, TSR and
   inherited `F_KEEP` blocks unlimited; `Malloc(-1)` unchanged (E5).

The pool allocator itself (`ffit`/`freeit`/`shrinkit`, the `MD`/`MPB`
structures, `Getmpb`) stays EmuTOS-faithful (ADR-021); only the MD supplier
and the limit change. ADR-023 (`Getmpb` publishes the static `themd`) is
untouched.

### Rationale

The user's principle: NewTOS runs on hardware with far more memory; internal
limits may follow it. A fixed system-wide count is the wrong bound for a
system with 1 GiB; a per-process bound protects against a runaway program
without limiting the others. Kernel heap (not the BDOS pool) because with
`memprot=off` every pool page is mapped for EL0 — MD chains there would be
writable by any program. The budget is counted over *all* MEM05 pages
because the snapshot alone (72 bytes per live block) is larger than an MD
and its record together; counting only MD pages would have let MEM05 take
~7 MiB of the heap at the worst case.

### Consequences

- Positive: `malloc` of small blocks no longer exhausts the system;
  thousands of blocks per process; Pterm/`Mfree` unchanged in semantics;
  `forget` (every alloc/free) is O(1) while no block is `F_KEEP`.
- Negative: the system-wide bound is now the budget: with `memprot=on`
  (record + snapshot + 1–2 MDs per block) roughly 13 000–17 000 live blocks
  in total; with `memprot=off` (MDs only) ~30 000–60 000. Per process 8192.
- Negative, **known limit**: every sync with protection on rebuilds the
  snapshot of all N live blocks — O(N·64 + N log N) per `Malloc`/`Mfree`
  (`region::further_owners` searches 64 entries per block, then a sort).
  With thousands of blocks this is noticeably slower than with tens.
  Follow-up ticket MEM10 (sync over the dirty range only); not done here.
- Negative: `Pterm` of a process with L blocks is O(L × N_allocated)
  (`detach`/`mfree` walk `mp_mal`); the 8192 default keeps that bounded.
- Neutral: the supply and the tables never shrink after a peak (E8); the
  budget bounds what they keep.
- Neutral: the kernel heap's 8 MiB come off the top of DRAM (the TT-RAM
  pool is 6 MiB smaller).

### Alternatives considered

- **A bigger static arena** (1024 blocks): still a fixed system-wide limit,
  only later; rejected by the user decision.
- **MD pages from the BDOS pool** (S2): visible to EL0 with `memprot=off`,
  recursive (`ffit` needs an MD to get a page for MDs), fragments the TPA.
- **Boot reserve** (S3): workable, more machinery; the heap already exists.
- **Budget for MD pages only** (option A of the stop report): worst case
  ~7 MiB of heap with the snapshot outside the budget.
- **Smaller snapshot / sync without a full snapshot** (options C/D):
  `memprot` rework, ticket MEM10.

### References

- `TASK/MEM05.md` (decisions), `TASK/MEM10.md` (sync cost follow-up).
- Design `docs/analysis/mem05-design-2026-10-13.md`; analysis
  `docs/analysis/mem05-md-pool-2026-10-13.md`.
- `docs/COMPATIBILITY.md` "No fixed memory-descriptor limit; a per-process
  limit instead"; `docs/api/structures/md.md` § "Sizing constraints".
- Code: `tos-api/src/bdos/mem/{pages,mdpool,owner_limit,region,traps}.rs`,
  `tos-api/src/bdos/process/memprot.rs`, `kernel/src/bios/machine/heap.rs`,
  `kernel/src/bios/machine/layout.rs`, `kernel/src/bootargs.rs` (`mdlimit=`).
- Tests: `tos-api/tests/mem05_{owner_limit,mdpool,growth,memprot_tables}.rs`.
- Evidence: `measurements/measurement-2026-10-13-mem05-kernel/RUNINFO.TXT`.
- ADR-021 (pool allocator EmuTOS-faithful), ADR-023 (unaffected).

---

## ADR-026: The desktop is an EL0 program (NEWDESK); the kernel carries no desktop (fulfils ADR-022's endgame)

**Date:** 2026-10-16
**Status:** **Accepted** (user decision 2026-10-16; plan approved 2026-10-14, D1–D6 in `TASK/DESK43.md`).
**Phase:** PHASE-14 (Desktop), cross-cutting (AES shell, kernel boot path).

### Context

ADR-022 rebuilt the desktop 1:1 against EmuDesk, through the AES trap, but
left it running at EL1 inside the kernel and named the move to EL0 as the
"endgame, later". That placement was not design freedom: in EmuTOS the
desktop runs in user mode as the AES process `rlr`, and a launched
application takes over the same descriptor (`aes/gemasm.S:77-79`,
`aes/gemstart.S:126`, `aes/gemshlib.c:604,622-634`). It was a deviation
(DESK43, evidence 2026-09-25). It also cost real defects: the EL1 desktop
started programs past the path real applications use (PORT05) and needed a
window handoff (`save_windows`/`restore_windows`) that EmuTOS' `sh_ldapp`
does not have. Two earlier attempts drifted back to bypasses because the
bypasses "had to keep working" (ADR-016 and ADR-022 Context, `343b3c5`).

### Decision

1. **The desktop is a process that ends.** It returns from its main loop
   with EmuTOS' epilogue (`desk/deskmain.c:2154-2166`), starts programs only
   via `shel_write` (`desk/deskpro.c:36-63`), keeps its state in the AES
   shell buffer, and `sh_ldapp`'s DESKTOP arm restarts it from the top
   (`aes/gemshlib.c:600-612`).
2. **The desktop is the EL0 program NEWDESK.** Source: the library
   `programs/desktop/` (dependencies `newtos-gem` and `tos-types` only, so
   a kernel bypass is a compiler error) with the binary `programs/newdesk/`.
   It is built by `cargo xtask build-programs`, embedded in the kernel image
   as an ELF blob (`kernel/embed.rs`), and started through the program
   registry as an ordinary EL0 process, like EmuTOS'
   `aes_run_rom_program` (`aes/gemshlib.c:570-580,606`). It has no
   privilege QED does not have.
3. **The kernel neither links nor calls the desktop** (DESK43 U10,
   `0a3b9be9`). kernel/src/desk/, the `desk=kernel` boot switch and the
   handoff API are gone. The xtask test
   `programs_tests::the_kernel_does_not_link_the_desktop` guards it, and
   `tools/desk-drift-probe.sh` reads 0.
4. **Failure policy (D6).** If the desktop ends, `wm_new()` runs and the
   desktop restarts with no message, as in EmuTOS. A panic inside NEWDESK
   waits for one key (D6-a). A load failure is not retried in the same
   boot (D6-c). It falls back to the console, EmuCON, with the line "No
   desktop: NEWDESK could not be loaded (GEMDOS error N)" (D6-d). The
   failure is kept in `shell_loop::DESKTOP_LOAD_ERROR`.

Order was part of the decision: the process model (1) came before the move
(2/3), and replacement came before removal. The fallback switch lived for
exactly one hardware round (U9-6, real Pi 4, 2026-10-16) and was deleted in
U10.

### Rationale

- **Conformance:** EmuTOS runs its desktop in user mode as an ordinary AES
  client; EL1 was a measured deviation, not latitude
  ([[no-rationalizing-emutos-deviations]]).
- **One world:** the desktop now exercises exactly the trap path every
  application uses, so a desktop that works validates that path
  (the bypass ADR-022 describes no longer applies).
- **Replaceability:** the desktop is one registry entry; a `#Z` program
  (TeraDesk) uses the same mechanism.
- **Security is explicitly not the reason.** NewTOS is single-user and has no
  network, and the original had no memory protection at all.
- Since ARCH03 the premise of plan v2 §8.3 ("every connection crosses a real
  address-space boundary") no longer holds. The address space is shared and
  identity-mapped, with rights per process. The move therefore cost
  correspondence work, not marshalling.

### Consequences

- Positive: the desktop is EmuTOS-conformant in placement and life cycle,
  bypasses are a compile error, and the kernel boot path is smaller.
- Positive: a broken or missing desktop no longer takes the system down.
  The user lands in EmuCON.
- Negative: every desktop change rebuilds the embedded blob, so the kernel
  build depends on the program build (`build_kernel` → `build_programs`). A
  stale board build can embed the wrong blob
  ([[cargo-xtask-board-target-footgun]]).
- Negative: program start/exit latency rises by the EL0 round-trip (QEMU
  estimate ~41 ms vs 6 ms at EL1, U9 acceptance run 2026-10-15; not yet measured on the Pi).
- Neutral: `KERNEL_DTA` stays, because EmuCON at EL1 and the self-tests use
  it. This corrects the plan v2 draft, which listed it for deletion.
  `pd::DESKTOP`, the kernel context's AES identity, is renamed
  `pd::KERNEL_CONTEXT` (`d11ed66b`).
- Not yet run on real hardware: the EmuCON fallback, the NEWDESK panic
  prompt, `memprot=verify` with the desktop up (`TASK/DESK43.md`,
  "Hardware-Abnahme U9-6").

### Alternatives considered

- **Keep the 1:1 desktop at EL1 (ADR-022 as it stood):** rejected. It is a
  deviation from EmuTOS and masks app-path defects.
- **Port EmuDesk's C as the EL0 program:** rejected (ADR-022). Rust stays,
  and the source correspondence gives the debugging benefit.
- **Keep `desk=kernel` permanently as a fallback:** rejected (D5-b, G6).
  Two desktops would be the third drift's mechanism. EmuCON is the fallback
  instead.
- **Move first, fix the process model afterwards:** rejected. That is the
  order that let the earlier attempts drift.

### References

- `TASK/DESK43.md` (decisions D1–D6, progress table, HW acceptance U3/U4 and
  U9-6); `TASK/PORT05.md`.
- `docs/analysis/desktop-userspace-plan-v2.md` (§8, §10 = the draft of this
  ADR), `docs/analysis/desk43-u5-u10-review-2026-10-14.md`,
  `docs/analysis/desk43-u9-prep-2026-10-14.md`,
  `docs/api/aes/shell-loop.md`.
- Commits `b3b1fc54` (U9-3..5, EL0 default), `0a3b9be9` (U10).
- Evidence: `measurements/measurement-2026-10-15-desk43-acceptance/RUNINFO.TXT`,
  `measurements/measurement-2026-10-16-desk43-u10/RUNINFO.TXT`.
- ADR-016, ADR-022 (refined and fulfilled), ADR-018 (`.NAP`), ADR-023 (what
  the kernel hands out must be reachable).

---

## ADR-027: Kernel memory management takes FreeMiNT as its reference, even where that is visible

**Date:** 2026-10-18
**Status:** **Accepted** (user decision 2026-10-18).
**Phase:** cross-cutting (GEMDOS memory, process memory, protection); refines ARCH03 (ADR-028) and ADR-025.

### Context

Since ARCH03 (ADR-028) the kernel's memory internals already follow FreeMiNT: one
shared, identity-mapped address space with rights per process, several
owners per block, `Mfree`/`Mshrink` only by an owner (`EIMBA` otherwise, E15),
protection modes `PROT_P/G/S/PR/I` (ADR-023, ADR-028).
The open memory tickets (MEM06, MEM07, MEM08, MEM10, ARCH03 stage 2b) are
optimisations of exactly that model. The 2026-10-18 rule "everything
EmuTOS-faithful" (PROC03) raised the question which reference those tickets
use. Building memory management EmuTOS-internal now and replacing it with
FreeMiNT for 2.0 ([[version-roadmap-1-0-2-0]], [[freemint-extension-direction]])
would do the work twice.

### Decision

1. **Kernel memory management uses FreeMiNT as its reference**
   (`references/freemint`): allocation, ownership, sharing, protection,
   process memory, performance work.
2. **This holds even where it is visible to programs in small ways.** The
   user (2026-10-18): *"selbst wenn es kleine sichtbare Änderungen gegenüber
   EmuTOS sind, würde ich aus Effizienzgründen dennoch für FreeMiNT mich
   entscheiden."* Every such visible difference is still **named** in
   `docs/COMPATIBILITY.md` (with the FreeMiNT and EmuTOS source lines), never
   left silent.
3. **Scope is the kernel only.** The AES and the VDI stay EmuTOS-faithful
   (ADR-022/ADR-026); NewTOS has not built them for FreeMiNT.
4. **The rest of the kernel is unchanged by this ADR:** processes, the file
   system, the console and the BIOS stay EmuTOS-faithful (PROC03/PROC04
   decisions of 2026-10-18), unless a later decision extends this one.

### Rationale

- Efficiency: no EmuTOS-internal rebuild that 2.0 would throw away.
- Consistency: the model is already FreeMiNT; references for its tickets must
  be the same source the model came from.
- Applications rarely depend on allocator internals; the contract that
  matters (return values, error codes, `Malloc(-1)`, MD lists) is checked per
  change and any visible difference is documented.

### Consequences

- MEM06/07/08/10 and ARCH03 stage 2b are specified from FreeMiNT
  (`sys/memory.c`, `sys/k_exec.c`, `sys/arch/mprot*.c` …); the spec entry
  names the FreeMiNT lines and, where visible, the EmuTOS behaviour it departs
  from.
- B5 (PROC03, user 2026-10-18, option 2) already follows this line: an
  accessory's memory ownership and protection stay with the accessory
  (FreeMiNT-like), only `*os_run`/`p_parent` follow EmuTOS.
- A program that relies on an EmuTOS allocator quirk may behave differently;
  the named entry in COMPATIBILITY is the place to look.

### Alternatives considered

- **EmuTOS for everything, including memory internals:** rejected — double
  work, and it contradicts the ARCH03 model already in place.
- **FreeMiNT only where invisible:** rejected by the user for efficiency.

### References

- `TASK/ARCH03.md`, ADR-023, ADR-025; `TASK/PROC03.md` (B5);
  `docs/api/gemdos/process/_run-ownership-curdir.md` (D-6..D-9).

---

## ADR-028: One shared user address space (VA = PA) with rights per address space — FreeMiNT's model on AArch64

**Date:** 2026-10-04 (user decision); written here 2026-10-26 from the draft
"ADR-025 Proposed" in `docs/analysis/arch03-implementation-plan.md` §1
(`646c3f46`). The number 025 went to the MD supply (ADR-025 above).
**Status:** **Accepted and implemented** (ARCH03). Pi acceptance 2026-10-05 on
`f2af53ea` (all programs rebuilt, `TASK/ARCH03.md` "Pi-Abnahme 2026-10-05");
tester acceptance on `main` 2026-10-13 on `0b52faf6`, memprot on/off/strict and
`asid=on` (`measurements/measurement-2026-10-13-arch03-acceptance/RUNINFO.TXT`).
Open: stage 2b (FreeMiNT `Pexec` modes 100+), `asid=on` on the real Pi.
**Phase:** cross-cutting (process model, loader, BDOS memory, MMU, AES).
**Supersedes:** ADR-010 (its decision VA = PA is kept; its path — one
`TTBR0`, installed once and never switched, no protection — is replaced) and
ADR-021 (the per-process window; its EmuTOS-faithful pool allocator stays).
**Extends:** ADR-023 (the `PROT_*` vocabulary takes effect). Refined by
ADR-027.

### Context

Until ARCH03 every process had its own EL0 address space: basepage at VA 0,
the image fixed at `0x1000`, `Malloc` blocks mapped into a per-process window
with VA ≠ PA (ADR-021). A pointer was valid only in the process that got it.
`Pexec` 3/4/5/6/7 could only be built as a handle crutch or not at all, LDG
could not work (the module writes into its caller's memory through
`OFFSETLDG`), resident programs lost their mapping, and `p_env` was an unmapped
PA (MEM03). The zView/LDG analysis (`docs/analysis/zview-ldg-2026-10-04.md`)
made this visible. EmuTOS has one address space without protection
(`bdos/proc.c:179-361`); FreeMiNT has one address space with per-process page
tables that differ only in rights (`sys/arch/mprot030.c:18,76-80`), and its
protection can be switched off without changing regions, ownership or `Pexec`.
User, 2026-10-04: *"es so wie bei FreeMiNT bauen"* (`TASK/ARCH03.md` "Anlass").
Analysis `docs/analysis/arch03-freemint-memory-model.md`, plan
`docs/analysis/arch03-implementation-plan.md`, ARM ARM check
`docs/analysis/arch03-stage0a-armarm-verification.md`, QEMU/Pi spike
`docs/analysis/arch03-stage0b-spike.md`.

### Decision

1. **One shared user address space, VA = PA,** for everything a process owns
   or is handed: both BDOS pools, the screen memory, published kernel
   structures (ADR-023). **Only exception:** page 0, `[0, 0x1000)`, stays
   private per process (system variables, OSHEADER, `$5A0`, trampolines,
   `Keytbl` area, the KEYS02 vector table) — `tos-api/src/bdos/process/mmu.rs`
   module note; contract side in `docs/COMPATIBILITY.md`, "One shared user
   address space, page 0 per process".
2. **One `TTBR0` root per address space.** Without protection all roots share
   every table below them except the ones leading to page 0 (FreeMiNT
   `no_mem_prot`); with protection a root gets private leaf tables only where
   its rights differ (copy-on-differ), with FreeMiNT's `PROT_P/G/S/PR/I`
   (`sys/mint/mem.h:120-136`). Boot argument `memprot=off|strict`
   (`kernel/src/lib.rs`). ASIDs are an optional switch-cost optimisation behind
   `asid=on`, default off (stage 4, `930aa81d`).
3. **Programs are position-independent** (`ET_DYN`, only
   `R_AARCH64_RELATIVE`); the kernel loads them into a pool block and relocates
   them there, as GEMDOS does with `.PRG`. `ET_EXEC` is refused with `EPLFMT`
   (`tos-api/src/bdos/process/elf.rs`, `ParseError::NotPositionIndependent`).
4. **Ownership after FreeMiNT** (several owners per block; only an owner
   frees), the free tag stays EmuTOS' `m_own` = PD pointer.
5. **Kernel access to user memory** with the caller's rights (unprivileged
   loads/stores with fault fixup, `tos-api/src/uaccess.rs`) for syscall
   arguments, and through the kernel alias `va + VA_OFFSET` after a check in
   the region table for kernel-initiated access to another process (AES).
   User code runs only with its owner's rights; a foreign `G_USERDEF` runs in
   the current process like EmuTOS with temporary rights
   (`memprot::grant_temp`/`revoke_temp`, FreeMiNT `prot_temp`).
6. **A protection fault ends the program with EmuTOS' crash report**
   (`Pterm(-1)`), not the machine (stage 0c, `9b7a7708`).

### Rationale

- A pointer must mean the same in every process — the common premise of
  `Pexec` 3/4/5/6/7, LDG, SLB, TSR, accessories, AES messages with pointers and
  global `Mxalloc`. A window per process cannot provide that; a shared space
  provides it trivially.
- VA = PA rather than a fixed offset: `Malloc` returns `m_start` again
  (EmuTOS), `Getmpb` MDs hold usable addresses, and every user address stays a
  positive 32-bit `LONG` (`TOS_MD_MAX_END = 0x7FFF_F000`).
- Space first, rights second, as FreeMiNT separates them: each stage could
  ship on its own.
- One root per process instead of ADR-010's single never-switched `TTBR0`:
  page 0 must be private, and the way to per-process rights is already laid.
  ADR-010's "wall 3" (QEMU hang on kernel/user aliases with different rights)
  is not an architecture rule (ARM ARM B2.11) and did not reproduce on QEMU
  11.0.2 or the Pi (stage 0a/0b documents above).

### Consequences

- Real PD pointers (`p_parent`, `Pexec(3)` result), `Pexec` 5/6/7, `Ptermres`,
  `vfork` and mintlib's `system()`, F_KEEP; MEM03, MEM04 and PROC01 closed
  (status lines of `TASK/MEM03.md`, `TASK/MEM04.md`, `TASK/PROC01.md`).
- Every native program had to be rebuilt as PIE; external `ET_EXEC` binaries
  (then `BALL.NAP`) are refused with error 35.
- Since E9 the TPA is EmuTOS' largest free block (`98199e35`), not a fixed
  4 MiB.
- Observable differences are named in `docs/COMPATIBILITY.md` (page 0,
  `p_tbase = p_lowtpa + 0x1000`, page-grain protection, F_KEEP).

### Alternatives considered

- **ADR-010 literally (one `TTBR0`, never switched, no protection):** page 0
  could not be private and rights would need a second rebuild. Rejected.
- **Keep ADR-021's window and translate pointers on hand-over:** impossible for
  pointers the kernel never sees (LDG's `OFFSETLDG` in the environment,
  structures with embedded pointers in messages).
- **Map the same blocks at different VAs per process:** visibility without
  pointer equality. Rejected.
- **A userspace LDG loader:** covers LDG only. Rejected.
- **Rights at once, without the unprotected stage:** two error sources in one
  step. Rejected.
- **Fixed load address per program:** needs global image placement and breaks
  two instances of one program. Rejected.

### References

- `TASK/ARCH03.md` and its stage receipts `TASK/ARCH03-*-STATUS-*.MD`;
  plan `docs/analysis/arch03-implementation-plan.md` (§1 draft, "Stufe 3 wie
  gebaut", "Stufe 4 wie gebaut").
- Commits: 0c `9b7a7708`, 0d `7fb554ec`, stage 1 `c6280342`, stage 2
  `32c6171c`, 3a `d2c68ec8`, 3b `48ba717c`, 3c `f2af53ea`, E9 `98199e35`,
  stage 4 `930aa81d`; runs `measurements/measurement-2026-10-05-arch03-*/`,
  `measurements/measurement-2026-10-06-arch03-4/`,
  `measurements/measurement-2026-10-13-arch03-acceptance/`.
- FreeMiNT `sys/arch/mprot030.c`, `sys/memory.c`, `sys/mint/mem.h`; EmuTOS
  `bdos/proc.c`, `bdos/umem.c`, `bios/kprint.c`.
- ADR-010, ADR-021, ADR-023, ADR-025, ADR-027, QUIRK-001.

---

## ADR-029: The cooperative kernel — EmuTOS' cooperative policy on FreeMiNT-shaped primitives

**Date:** 2026-07-24
**Status:** **Accepted** (Lead + user decision 2026-07-24,
`docs/analysis/coop-kernel/10-upcall-signal-design.md:3-7`; written here
2026-10-26, moved from the STATUS log entry of 2026-07-25). Implemented:
steps 1–3b (below); step 4 grew into the accessory and scheduler work since
(`96f17854` accessories, `e7754b16` EmuTOS ready list).
**Phase:** cross-cutting (process model, AES, scheduler).

### Context

The user asked for an effort estimate: build multitasking FreeMiNT-like right
away, or like EmuTOS (`docs/analysis/coop-kernel/00-effort-comparison.md`,
from `emutos-cooperative-model.md` and `freemint-multitasking-model.md`). The
user's intent was *"kooperativ implementieren, aber so, dass man es ohne
Wegwerf auf FreeMiNT-Niveau hochziehen kann"* (ibid., "Warum EmuTOS-Stil ≠
EmuTOS-treu"). NewTOS' hard primitives were already FreeMiNT-shaped: EL0/EL1
with an MMU, a `TTBR0` per process, a context = `ExceptionFrame`. EmuTOS'
`switchto` is a flat supervisor SP/register swap without an address-space
switch and has no clean place on that base.

### Decision

The question splits into two levels, decided separately:

1. **Mechanism / primitives: FreeMiNT-shaped.** Protected per-process
   contexts, a ready queue with `sleep()`/`wake()`, and an EL0 upcall/signal
   path modelled 1:1 on FreeMiNT `sendsig`/`Psigreturn`.
2. **Policy / behaviour: EmuTOS-cooperative, first.** Yield only when
   blocking, no preemption, the AES event/message model, desktop +
   accessories + one main application that owns the screen.
3. **Preemption, priorities, several applications, SMP: deferred and
   additive** — FreeMiNT's `preempt()` sits on `sleep()`/`wake()`, no rewrite.

### Rationale

- The upcall is unavoidable either way (it delivers to a blocked EL0 AES
  client) and was already due for `etv_term`, `vex`, `G_USERDEF` and `eralert`;
  FreeMiNT's `sendsig` is a complete reference for it.
- "Full FreeMiNT now" (an XaAES-like client/server AES plus preemption) is
  more work now and builds extensions before the faithful base runs cleanly on
  the Pi.
- "Pure EmuTOS style" would either bypass the EL0/EL1 protection or build the
  upcall anyway.

### Consequences

- Step 1 generic EL0 upcall + trampoline + GEMDOS `Psigreturn` (0x11A)
  (`55a2a346`, design `docs/analysis/coop-kernel/10-upcall-signal-design.md`); step 2
  `etv_term` via upcall on `Pterm` (`ad3e8eff`); step 3a sleep/wake substrate
  (`1844e09a`, design `docs/analysis/coop-kernel/20-sleepwake-substrate-design.md`); step 3b
  restartable context switch + deliver-on-resume (`aac71446`); prep cleanups
  `366d3b25`. Live today: `deliver_upcall`/`handle_psigreturn` in
  `kernel/src/arch/aarch64/exceptions.rs`, `tos-api/src/bdos/process/sched.rs`.
- Accessories run cooperatively (`96f17854`,
  `measurements/measurement-2026-09-25-acc02-acc04/`), and the scheduler picks
  by EmuTOS' `rlr`/`drl` ready list (A115, `e7754b16`,
  `measurements/measurement-2026-10-25-a115/RUNINFO.TXT`).
- The memory side of the same primitives became FreeMiNT's model with ADR-028;
  ADR-027 keeps kernel memory on FreeMiNT, AES/VDI on EmuTOS.

### Alternatives considered

- **FreeMiNT preemptive multitasking now:** rejected — more effort now,
  extension before the faithful base.
- **EmuTOS' flat `switchto` 1:1:** rejected — no place for it on EL0/EL1 + MMU.

### References

- `docs/analysis/coop-kernel/` (00, 10, 20, 30, `emutos-cooperative-model.md`,
  `freemint-multitasking-model.md`); FreeMiNT `sys/arch/sig_mach.c`,
  `sys/arch/user_things.S`; EmuTOS `aes/gemdisp.c`, `aes/gemasm.S`.
- ADR-022, ADR-026, ADR-027, ADR-028.

---

## ADR-030: No Line-A emulation

**Date:** 2026-10-26
**Status:** **Accepted** (2026-10-26, user decision). Written the same day
as **Proposed** (`075bdddd`) with three open questions; the user answered Q3
first, then Q1 and Q2 (answers in place below). It writes down the line
Phase 12 already followed (closed without a Line-A export) and settles the
variable block.
**Phase:** cross-cutting (VDI, desktop, SDK; an m68k emulator, if one is ever
built).

### Context

On the ST, the m68k opcodes `$A000`–`$A00F` are unimplemented instructions
that trap through exception vector `$28`; EmuTOS installs `int_linea` there
(`references/emutos/bios/bios.c:176`) and dispatches 16 graphics routines
from a table (`references/emutos/bios/linea.S:230-249`). Op 0 returns the
base of the **LINEA variable block** in `a0`
(`references/emutos/bios/linea.S:156-157`). The block holds screen, font and
mouse state at fixed offsets around that base — for example `MOUSE_BT` at
−596 and `V_REZ_HZ` at −12, base at `line_a_vars`
(`references/emutos/bios/lineavars.S:126`, `:192`, `:266`). Two different
things therefore go by the name "Line-A" and are kept apart here:

- **(a) the opcode trap** — a mechanism of the m68k instruction set;
- **(b) the variable block** — data a program reads once it has the pointer.

Evidence, checked 2026-10-26:

- **Phase 00 D10** (`docs/api/_line-a.md`; line numbers in this bullet and
  the next are those of commit `075bdddd` — the file was rewritten when this
  ADR was accepted) planned a C-callable Line-A
  function group for native ports behind a header `tos-linea.h`
  (`docs/api/_line-a.md:65-76`) and opcode interception in an m68k emulator
  (`:72-74`). It left two ADRs open: ADR-Line-A-01, the C ABI of that export
  (`:111-114`), and ADR-Line-A-02, who owns the mouse cursor (`:115-117`).
  The archived status log records ADR-Line-A-02 as settled in fact by ADR-011,
  with the cursor in `tos-api/src/vdi/mouse.rs`, and ADR-Line-A-01 as still
  open (`docs/archive/status-log-2026-10-26.md:1021-1022`). `docs/STATUS.md`
  *Open ADRs* carries ADR-Line-A-01 as a user decision: Phase 12 closed
  without a Line-A export, and "no Line-A emulation" stood in no ADR.
- **D10 is wrong in two places** (checked at the pin `1a7484ab`): its source
  table names vdi/lineavars.S, vdi/lineainit.c and vdi/lineaasm.S, which do
  not exist — the files are `references/emutos/bios/lineavars.S`,
  `references/emutos/bios/lineainit.c` and `references/emutos/bios/linea.S`;
  and its statement that the AES and the desktop call Line-A
  (`docs/api/_line-a.md:78-92`) does not hold (both corrected 2026-10-26:
  the paths in `cbac4e82`, the AES claim when this ADR was accepted). No file
  under `aes/` or `desk/`
  issues a Line-A opcode; the only one that includes `lineavars.h` is
  `references/emutos/desk/desksupp.c:56`; the AES hides and shows the mouse
  through VDI opcodes (`gsx_moff`/`gsx_mon`,
  `references/emutos/aes/gemgsxif.c:440-455`).
- **EmuDESK reads two variables of the block** directly: `V_REZ_HZ` in
  `do_xyfix` (`references/emutos/desk/desksupp.c:267`, under
  `CONF_WITH_3D_OBJECTS`) and `MOUSE_BT` in `get_key`, the page wait of
  `show_file` (`references/emutos/desk/desksupp.c:515-517`). NEWDESK leaves out
  the `do_xyfix` nudge (`programs/desktop/src/main.rs:2814-2819`) and has no
  `show_file`/`get_key` (no hit in `programs/desktop/src/`).
- **ADR-011** (Accepted, not implemented) replaces EmuTOS' global Line-A
  screen state by a per-device physical workstation — it already assumes the
  block is not where the VDI keeps its state.
- **Code:** `kernel/` and `tos-api/` contain no Line-A trap, vector or
  dispatcher; `linea` occurs only in comments (`/usr/bin/grep`, 2026-10-26).
  `Setexc(10, …)` (vector `$28`) stores into the software vector shadow,
  which nothing dispatches (`tos-api/src/bios/vectors.rs:98-109`). The values
  the block would carry live inside the kernel: `GCURX`/`GCURY`/`MOUSE_BT`
  (`tos-api/src/vdi/mouse.rs:24-26`), the device-global `REQ_COL`
  (`tos-api/src/vdi/req_col.rs:8-13`); `CUR_WORK` is not modelled
  (`tos-api/src/vdi/workstation.rs:372-373`). No LINEA block is published to
  a process — B1's page-0 publication covers the sysvars, OSHEADER and the
  cookie jar (`docs/COMPATIBILITY.md`, "System variables — compatible how").
- **SDK:** mintlib's `linea.c` (the `linea0()`… bindings) is a recorded build
  failure (as of this writing; excluded by this ADR since acceptance), m68k register asm (`sdk/libc/expected-failures.txt:21-22`,
  `docs/sdk/07-mintlib-limits.md:47`); a port that calls a Line-A binding does
  not link. mintlib's alglobal.c is compiled, so `__aline` exists and stays a
  null pointer. `__LINEA` and `__FONT` change layout under LP64 and no port
  reads them today (`docs/sdk/05-abi-pitfalls.md:152-156`).
- **m68k binaries:** the m68k user-mode emulator is out of scope
  (`phases/README.md:34-38`); the requirements keep it as optional FR-4
  (`docs/REQUIREMENTS.md:23-27`); ADR-018 reserves the classic suffixes for it.
- **Open uses of the block in NewTOS documents:** EmuCON reads
  `v_cel_mx`/`v_cel_my` through Line-A; its decision D4 is open, and option
  (c) there is a LINEA-like block in the process
  (`docs/api/emucon/06-mapping-and-decisions.md:137-152`). The `Setscreen`
  work names `v_lin_wr` as one of two possible answers to "how large is a
  screen page" (`docs/archive/status-log-2026-10-26.md:317`).
- `docs/COMPATIBILITY.md` has no Line-A entry (`/usr/bin/grep -i -E
  "line-a|linea"`, 2026-10-26; entry "No Line-A" added on acceptance).

### Decision

1. **(a) No Line-A opcode trap in the native system.** AArch64 has no
   `$Axxx` trap, and NewTOS does not invent a substitute (no `svc` class, no
   undefined-instruction hook) for native programs.
2. **(a) The m68k Line-A trap belongs to an m68k emulator**, if one is ever
   built (FR-4). How that emulator serves `$A000`–`$A00F` is its decision;
   this ADR does not design it.
3. **(c) Native programs reach the primitives and the screen/mouse state
   through the documented calls only:** the VDI (`v_opnvwk`/`vq_extnd`
   `work_out` for the geometry — `DEV_TAB[0]` = `V_REZ_HZ − 1`,
   `references/emutos/vdi/vdi_control.c:203-204`; `vq_mouse` for buttons
   and position, `tos-api/src/vdi/mouse.rs:897-904`; `v_show_c`/`v_hide_c`;
   `vq_chcells`), the AES (`graf_mkstate`, `graf_handle`) and the XBIOS.
   The Line-A state stays kernel-internal behind those calls — the state of
   the code today. This also settles ADR-Line-A-02: the mouse cursor state
   is the VDI's (`tos-api/src/vdi/mouse.rs:24-26`); ADR-011, which the
   archived status log names for it, does not mention the cursor.
4. **(b) No LINEA variable block is exported** (Q1). `__aline`, `__fonts`
   and `__funcs` stay null by design
   (`sdk/libc/mintlib/mintlib/alglobal.c:7`, `:11`, `:16`), so a reader
   fails loud instead of reading wrong values: the named variables of
   `<mint/linea.h>` are negative offsets from `__aline`
   (`sdk/libc/mintlib/include/mint/linea.h:154-315`), which address just
   below 2^64, and the EL0 data abort ends the program
   (`kernel/src/arch/aarch64/exceptions.rs:303-309`). **Limit, found when
   the ADR was accepted:** the `__LINEA` struct fields (`__aline->_VPLANES`
   and the MWC macros, `sdk/libc/mintlib/include/mint/linea.h:485`ff.) are
   positive offsets; null plus such an offset lies in page 0
   (`sizeof(__LINEA)` < 0x1000 with `sdk/bin/newtos-cc`, checked
   2026-10-26), which is mapped user read/write in every process
   (`tos-api/src/bdos/process/mmu.rs:643`). Such a read does not fault, a
   write lands in the process's page 0. No built program does either
   (`docs/analysis/line-a-readers-2026-10-26.md`).
5. **No C-callable function group, no `tos-linea.h`** (Q2). mintlib's
   `linea.c` (`linea0()` … `lineaf()`) is **excluded** from the SDK build by
   this ADR, not a build failure to be repaired.

### Open questions (the user's decision) — answered

- **Q1 — Export a read-only LINEA variable block to programs?** The evidence
  shows readers in the reference (EmuDESK: two fields; EmuCON: two fields)
  but no ported program that reads one. If yes, five things need a spec
  first (Rule 5): which fields; how a native program obtains the base pointer
  without `$A000` (a cookie, an `Ssystem` sub-function or a fixed page-0
  location — each one an interface extension); the layout under LP64 (the
  pointer fields, as with `__LINEA`/`__FONT`); live values (`MOUSE_BT`,
  `GCURX`/`GCURY`) or a snapshot; and how one global block maps onto
  ADR-011's per-device workstation. If no, EmuCON D4 needs one of its other
  options, and `__aline` stays null.
  **Answer 2026-10-26 (user): no.** No native reader exists
  (`docs/analysis/line-a-readers-2026-10-26.md`). `__aline` stays null by
  design and a reader fails loud (Decision 4, with its limit). EmuCON D4
  loses option (c).
- **Q2 — The C-callable function group of D10 (`tos-linea.h`):** drop it, or
  keep it as a later option? Dropped, ADR-Line-A-01 (its C ABI) is moot and
  closes with this ADR; kept, ADR-Line-A-01 stays open.
  **Answer 2026-10-26 (user): dropped.** ADR-Line-A-01 closes with this
  ADR; `tos-linea.h` is removed from the documents (dated snapshots and
  `docs/archive/` keep it as history).
- **Q3 — The two EmuDESK readers — answered 2026-10-26 (user):** NEWDESK
  uses the VDI values. Under ADR-022 (1:1) it restores the `do_xyfix` nudge
  and, if `show_file` is ever built, the mouse-button page wait, reading the
  horizontal resolution and the button state through the VDI (above), not
  through a LINEA block. Independent of Q1. Ticket DESK69.

### Rationale

- The opcode trap is a property of the m68k instruction set. On AArch64
  there is nothing to emulate; a replacement would be a new trap, an
  interface extension no program calls — the only native bindings
  (mintlib's) are m68k asm and do not build.
- The EmuTOS AES and desktop do not use the trap either (above); the VDI
  already provides the primitives and the state.
- The variable block: ported programs could read it, and the reference
  desktop does. While it stayed open (Proposed), the evidence supported
  neither an export nor a rejection. The user decided against an export
  because no native reader exists
  (`docs/analysis/line-a-readers-2026-10-26.md`), and chose a null `__aline`
  so that a future reader fails loud rather than silently.

### Consequences

- Positive: no new trap or `svc` class; the Line-A state stays internal
  and per device (ADR-011).
- Negative: a source port that calls `linea0()` and friends fails at link
  time; one that reads `__aline` gets null and ends with a data abort on the
  named variables — except the `__LINEA` struct fields, which reach page 0
  silently (Decision 4). A port that needs Line-A values has to be changed to
  the VDI calls of Decision 3.
- NEWDESK's two EmuDESK deviations get closed through VDI values (Q3):
  `TASK/DESK69.md`.
- The SDK build change (mintlib `linea.c` excluded instead of recorded as an
  expected failure) is a separate Developer task; the documents already say
  "excluded by ADR-030".
- Done on acceptance (2026-10-26): `docs/api/_line-a.md` corrected (AES
  claim, implementation strategy replaced by this ADR; the source paths in
  `cbac4e82`); `docs/COMPATIBILITY.md` entry "No Line-A"; STATUS row
  ADR-Line-A-01 closed; `tos-linea.h` removed from
  `docs/api/emucon/06-mapping-and-decisions.md`.
- Neutral: m68k binaries are unaffected — there is no emulator.

### Alternatives considered

- **A Line-A substitute trap for native code** (own `svc` immediate or an
  undefined-instruction hook): ports would still need source changes, since
  their bindings are m68k asm; it adds an interface without a caller.
  Rejected.
- **The C-callable function group of D10:** rejected (Q2).
- **A full live LINEA block at a fixed address in every process** (like the
  B1 sysvars): rejected (Q1).
- **EmuTOS' global Line-A state as the VDI's own state:** rejected by
  ADR-011.

### References

- `docs/api/_line-a.md` (Phase 00 D10); `docs/STATUS.md` *Open ADRs*;
  `docs/archive/status-log-2026-10-26.md:1021-1022`;
  `docs/analysis/line-a-readers-2026-10-26.md` (the reader sweep behind Q1/Q2);
  `docs/COMPATIBILITY.md` ("No Line-A"); `TASK/DESK69.md`.
- EmuTOS @ `1a7484ab`: `references/emutos/bios/linea.S`,
  `references/emutos/bios/lineavars.S`, `references/emutos/include/lineavars.h`,
  `references/emutos/bios/bios.c:176`, `references/emutos/desk/desksupp.c:56`,
  `references/emutos/aes/gemgsxif.c:440-455`.
- ADR-011, ADR-018, ADR-022.

---

## ADR-031: OS hooks installed by programs

**Date:** 2026-10-28
**Status:** **Accepted** (2026-10-28, user decision on TASK/BIOS03, Q1-Q6
of the proposal).
**Phase:** cross-cutting (BIOS, XBIOS, VDI, process lifecycle).

### Context

On TOS a program can install code that the OS later calls: `Setexc`
vectors (`etv_*`, trap and CPU vectors), page-0 cells reached only by a
direct write (`hdv_*`, `swv_vec`, `exec_os`, `dump_vec`, `resvector`), the
`Kbdvbase` table, the XBIOS installers (`Initmous`, `Mfpint`, `Xbtimer`) and
the VDI exchange vectors (`vex_*`). NewTOS serves some of them and stores
others without ever calling them. Audit findings: BIOS-STOR-010 (`hdv_*`),
BIOS-CORE-002 (`etv_timer`), XBIOS-CHARDEV-003 (`Kbdvbase` hooks).

Evidence: `docs/analysis/os-hooks-adr-proposal-2026-10-28.md` — the hook
list with EmuTOS install and call sites (section 1), the survey of the
ported programs (section 2), what NewTOS does per hook (section 3). Among the
ported programs only TeraDesk installs hooks NewTOS does not serve
(`hdv_*` in `force_mediach`, `etv_critic`, the VBL/Timer-C vectors in its
shutdown, `resvector`); Doom and two test programs install only `kbdvec`,
which is served.

### Decision

1. **Rule: never silent.** A hook NewTOS does not serve is reported: one
   warning in the log **per hook per process** (the accessory model,
   `accessory_hook_unwarned`, `tos-api/src/xbios/kbdvecs.rs:322@accessory_hook_unwarned`).
   Hooks that EmuTOS itself never calls (`etv_xtra`, `hdv_init`,
   `hdv_boot`, `prt_stat`/`prt_vec`, `aux_stat`/`aux_vec`) get no warning.
2. **Per-hook table:**

| Hook | NewTOS | Reason / semantics | Observable consequence |
|---|---|---|---|
| `kbdvec`, `etv_term`, `vex_wheelv`, `Supexec`, `G_USERDEF` | served | as built (KEYS02, upcalls) | as EmuTOS |
| `hdv_rw`, `hdv_bpb`, `hdv_mediach` — **floppy use** (default floppy driver chain, `force_mediach` for unreliable floppy change detection, `Mediach` "maybe changed", A:/B: specifics) | not served, warning | hardware absence: no floppy/DMA (CLAUDE.md non-goals; FAT12/floppy out of scope) | TeraDesk's `force_mediach` has no effect; NewTOS GEMDOS reads the block layer directly and never calls `hdv_*` |
| `hdv_rw`, `hdv_bpb`, `hdv_mediach` — **non-floppy use** (RAM disks, disk caches) | not served, warning | no ported program needs it today | a RAM disk or cache that hooks `hdv_*` does not take over a drive. **Revisit with full support** (GEMDOS block access through a hookable path) when a ported program needs it. No app patch. |
| `etv_critic` | not served, warning | NewTOS `Rwabs` has no retry/critical-error path | the error code is returned directly (TeraDesk's handler returns it anyway) |
| `etv_timer` | not served, warning | no program in the survey | BIOS-CORE-002 is an **ACCEPTED deviation** (proof: this ADR) |
| `Kbdvbase` `mousevec`, `statvec`, `clockvec`, `joyvec`, `midivec`, `vkbderr`, `vmiderr`, `ikbdsys`, `midisys` | not served, warning | no program in the survey; no MIDI/ACIA | XBIOS-CHARDEV-003 is an **ACCEPTED deviation** (proof: this ADR). The `kbdvec` mechanism (chain with a default stub, upcall) can be reused once a user appears. |
| `bell_hook`, `kcl_hook` | not served, warning | no sound device yet | no bell/click hook |
| `dump_vec`, `swv_vec`, `exec_os`, `resvector`/`resvalid`, `vblqueue` | not served, warning | no program in the survey needs them called | the hook is never called |
| trap and CPU vectors via `Setexc` (< 0x100), `Mfpint`, `Xbtimer`, `Jdisint`, `Jenabint` | not served, warning | AArch64 has no m68k trap/autovector; no MFP (as ADR-030 decision 1) | the vector is stored, nothing dispatches it |
| `vex_butv`, `vex_motv`, `vex_curv`, `vex_timv` | not served, **warning added** | stored but never called (today a silent violation of rule 1) | the vector round-trips, the handler is never called |
| `prt_stat`, `prt_vec` | default = the `just_rts` stub like EmuTOS, not 0 | EmuTOS never calls them | a program that chains to the old value gets a valid entry |

3. **Ownership.** The installing process owns the hook. At `Pterm` without
   unhooking: restore the previous vector **only** if the vector still
   points to the exiting process's hook (top of an XBRA chain); otherwise
   leave it and warn. At `Ptermres` the hook stays and its owner stays
   resident.
4. **Page 0.** The kernel may read page-0 vector cells **for warnings only**,
   never to call or act on them. Detection: when a process leaves the super
   state (page 0 is writable for EL0 only there, TASK/MEM12), compare the
   vector cells with their defaults.
5. **Detection points** for the warning: in `Setexc` itself; page-0 cells on
   leaving the super state; `Kbdvbase` slots on the first event that would
   call them; `Initmous`, `Mfpint`, `Xbtimer` and the `vex_*` calls in the
   call itself.

### Consequences

- BIOS-CORE-002 and XBIOS-CHARDEV-003 become ACCEPTED deviations; their
  tickets (TASK/BIOS05.md, TASK/XBIOS03.md) keep only the warning.
  BIOS-STOR-010 likewise. Implementation of rules 1, 3, 4, 5 and the
  `prt_*` default: TASK/BIOS15.md.
- `docs/COMPATIBILITY.md` gets an "OS hooks" entry next to "No Line-A".
- A hook moves to "served" only by amending this table, with the program
  that needs it as evidence.

## Template for new ADRs

```markdown
## ADR-NNN: Title

**Date:** YYYY-MM-DD
**Status:** Proposed | Accepted | Superseded by ADR-MMM | Deprecated
**Phase:** PHASE-XX (or "cross-cutting")

### Context

What is the situation that requires a decision? What constraints apply?
What sources were consulted (per SOURCE-ANALYSIS-PROTOCOL.md)?

### Decision

The chosen course of action, stated in one or two sentences.

### Rationale

Why this decision, not the alternatives? What did the source analysis
reveal? What does this optimize for, and what does it trade off?

### Consequences

- Positive: …
- Negative: …
- Neutral: …

### Alternatives considered

For each alternative, why it was rejected. **Do not omit alternatives
that seem obviously wrong** — future readers may not see why obvious.

### References

- Spec sections, datasheet pages, EmuTOS source paths (with SHA)
- Related ADRs
- Discussion thread / commit hash if any
```

---

## Quirks Discovered

Format: `QUIRK-NNN`, dated, with severity (low / medium / high / critical).

## QUIRK-001: QEMU cortex-a72 TCG wedges on TTBR0 switch into a VA-aliased, ASID-0 address space

**Date discovered:** 2026-06-16
**Severity:** High (blocks `Pexec`/`Pterm` round-trip under QEMU)
**Phase / context:** PHASE-06 D3/D6 — `Pexec(PE_LOADGO)` parent/child round-trip
**Status:** Worked around; the UART barrier is still in the code
(`kernel/src/arch/aarch64/exceptions.rs`, the QUIRK-001 brackets). The
same-VA cause no longer arises since ARCH03 (ADR-028: programs are
position-independent and run at their own physical address). Per-process
ASIDs are built behind `asid=on` (default off until the Pi run). See the
updates below; "do not re-attempt ASIDs" is superseded.

### Symptom

After a child process calls `Pterm`, the kernel restores the parent's
`TTBR0_EL1` and `eret`s back to the parent at EL0. On QEMU
`aarch64-virt,gic-version=2,cortex-a72` (QEMU 11.0.0) the guest then
makes **no forward progress** — a hard hang, no exception, no further
trap. The parent's next instruction (its own `Pterm` SVC) is never
observed. Deterministic.

### Cause

Parent and child are both loaded by the minimal Phase-06 D3 loader with
their TPA (and `e_entry`) at the **same** user VA (`0x100`), and
`install_ttbr0` writes the L0 phys to `TTBR0_EL1` with **ASID = 0** for
every process. So the parent's and the child's distinct physical code
pages live at the *same* VA under the *same* ASID. QEMU's TCG model
keeps the child's already-translated block for VA `0x118` and reuses it
when the parent re-enters that VA across the `TTBR0` switch — the
`tlbi vmalle1` in `restore_boot_ttbr0` flushes the soft-TLB but does not
get TCG to re-walk before the immediately-following `eret`-to-EL0, so
the model wedges. On real ARMv8 hardware this cannot happen: the L1
I-cache is PIPT, so different physical pages never alias, and the
ordering is immaterial.

### Resolution

**Interim (committed):** emit one line of UART traffic (Device-memory
*writes*) **between** the `restore_boot_ttbr0()` TLBI and the
`install_ttbr0(parent)` in the `Pterm` handler
(`kernel/src/arch/aarch64/exceptions.rs::handle_pterm`). A Device
access forces TCG to end the translation block and re-walk under the
new `TTBR0`. Verified deterministic across 4/4 QEMU runs.
`handle_pexec` already relied on exactly this (its `[pexec] …` prints
sit in the same spot). Empirically: a single Device *read* does **not**
suffice, and pure arithmetic/store "distance" between the TLBI and the
`eret` does **not** suffice — only the write traffic of a console line.

**Per-process ASIDs — ATTEMPTED 2026-06-16, does NOT work on this QEMU.**
The architecturally correct fix is a distinct ASID per process in
`TTBR0_EL1[55:48]` (`TCR_EL1.A1 = 0` already makes TTBR0 supply the
ASID; `AS = 0` gives 8-bit ASIDs, enough for `MAX_PROCESSES = 16`).
On real hardware that would let TCG/TLB distinguish the same-VA address
spaces. **But QEMU 11.0.0's cortex-a72 TCG wedges on the ASID-tagged
`TTBR0_EL1` write itself** — tagging the EC4 install with a non-zero
ASID hangs at the `msr ttbr0_el1` / `isb`, *before* any `eret`, so it
makes things strictly worse than ASID 0. Reverted in full. **Key
insight:** the QUIRK-001 wedge is purely a QEMU artifact (real ARMv8
L1 I-caches are PIPT and never alias across distinct physical pages),
so ASIDs are **not needed to fix the wedge** at all — the interim
Device-write barrier is the correct long-term solution for the QEMU
test target (a harmless MMIO read on real hardware). ASIDs remain
worth adding later purely for real-hardware TLB efficiency, and should
be validated on real RPi4 / a newer QEMU, not against this model.

So: **do not re-attempt ASIDs to "fix" QUIRK-001** — that is solved by
the barrier. Keep the barrier; it is correct on both targets.
*(Superseded by the update of 2026-10-06 below: the ASID wedge is gone on
QEMU 11.0.2, and ASIDs are built behind `asid=on`.)*

**Update 2026-10-06 (ARCH03 stage 4):** the ASID wedge is gone on QEMU
11.0.2 and does not exist on the real Pi 4 (ARCH03 0b, S0b §4 + Pi result).
Per-process ASIDs are now built — **behind the boot argument `asid=on`,
default off** (`asid=tiny` = 3 ASIDs per generation, the rollover test):
`nG` on every user leaf, `TCR_EL1.AS` from `ID_AA64MMFR0_EL1.ASIDBits`,
one ASID per slot tenancy with generations (`tos-api/src/bdos/process/asid.rs`),
switch = tagged `msr ttbr0_el1` without TLBI, narrowing by `TLBI VAE1IS`/
`VAAE1IS`, rollover `VMALLE1IS` with the empty root/ASID 0 installed.
With the flag off the TLBI-on-every-switch path above is unchanged. The
UART bracket prints stay in both modes (still diagnostic output; dropping
them is a separate step after the Pi run). **QEMU caveat:** QEMU does not
keep TLB entries of an inactive ASID observable — a deliberately broken
kernel without the per-VA invalidation still passes the inactive-ASID
probe on QEMU, only the same-ASID case (MPROT `f`) catches it. Retention
across ASIDs is therefore verifiable only on the Pi.
Evidence: `measurements/measurement-2026-10-06-arch03-4/RUNINFO.TXT`.

**Update 2026-10-26 (checked against the code):** the Cause above — parent
and child loaded at the **same** user VA — no longer occurs. Since ARCH03
0d (`7fb554ec`) programs are position-independent and the kernel relocates
each one into its own pool block, and since ARCH03 stage 1 (`c6280342`)
every user address is its physical address (ADR-028), so two processes
never share a VA for different physical code. The barrier is still in
place (`kernel/src/arch/aarch64/exceptions.rs`, the "QUIRK-001 bracket"
comments).

### Why this is hard to rediscover

The failure is a silent hang with no exception, and it *disappears the
moment you add a debug print* near the TTBR0 switch — so naive "add a
print to see where it dies" debugging makes the bug vanish and hides the
real cause. The load-bearing nature of the print is the trap.

### Source

QEMU 11.0.0 TCG behaviour, established by bisecting the hang with
incrementally-placed UART writes (a Device *read* alone did not fix it;
a `kprintln` between TLBI and install did). ASID mechanics per ARM ARM
D8 (TTBR/TCR ASID fields). Cross-checked against `handle_pexec`, which
exhibits and silently works around the same quirk.

## QUIRK-002: `GICD_IGROUPR` reads zero from Non-secure state — the diagnosis "PL011 RX IRQ not delivered on real RPi4" was refuted

**Status:** **Refuted** (2026-09-04). INTID 153 (PL011 UART0 RX) **is**
delivered to the Non-secure kernel on real Pi 4. Evidence: the boot-screen
measurement of 2026-09-04 on real hardware found the IKBD ring filled
before any input call had run — only `console::pl011_rx_drain`, the
INTID-153 handler, could have done that — and the PL011 FIFO drained
(`FR = 0x0091`, `RIS = 0x0020`); see "Correction 2026-09-04" below and the
STATUS log row of 2026-09-04 ("Eine Phantomtaste lag seit jeher …", now in
`docs/archive/status-log-2026-10-26.md`). The kernel was changed on that
basis: `pl011_rx_drain` runs from the GIC IRQ handler and
`serial_rx_byte` drops the byte on rpi4 (`kernel/src/console.rs:217-275`,
`44361630`). A second diagnostic boot the same day (`initinfo-diag` build,
`5583602c`) counted one `0x00` byte in the UART init flush — that result is
recorded only in the lead's notes, not in a measurement run. **What
remains true** is the trap: `GICD_IGROUPR` is RAZ from Non-secure state, so
reading it proves nothing about the interrupt's group.
**Date discovered:** 2026-06-20 (the original diagnosis); refuted 2026-09-04
**Severity:** Low (a diagnostic trap; the original "blocks every peripheral
IRQ" was wrong)
**Phase / context:** real Pi 4 serial console; GIC-400 diagnosis

The sections below are the original entry of 2026-06-20 and its correction,
kept as history. Also stale in them: `console::pl011_rx_poll` (deleted in
`b776ffaa`, old hash `c6e4bfb`) and the UART2 IKBD input (removed in
`1ace07a4`); input on real Pi 4 is USB-HID alone (ADR-015). Still useful:
copying the project's `config.txt` template over the card must keep
`gpu_mem=256` (Resolution, "Correction"), and the GICD (`0xFF841000`) /
PL011 (`0xFE201000`) addresses for a register dump.

Original title (2026-06-20): *GIC-400 SPIs reset to Group 0 (secure) —
peripheral IRQs not delivered to the NS kernel on real RPi4.*

### Symptom

On real RPi4 silicon, the PL011 UART0 RX interrupt (INTID 153) is never
delivered to the core, although the identical path works in QEMU. TX and
polled RX work; only IRQ delivery fails. Diagnosed 2026-06-20 with a
register dump (`gic_diag.rs`, since removed): `GICD_CTLR=1`, `GICC_CTLR=1`,
`ISENABLER[153]=1`, `ITARGETSR=CPU0`, `ICFGR=level`, `PL011 IMSC` RX
unmasked — all correct — but **`GICD_IGROUPR[153] = 0` (Group 0 /
secure)** and `GICC_PMR` reads back `0xF0` (the non-secure view).

### Cause

The GIC-400 implements the GICv2 Security Extensions. **At reset every
interrupt is in Group 0 (secure)** → signalled as **FIQ to secure**, not
as IRQ to the Non-secure kernel. The RPi4 firmware drops the kernel to
**Non-secure EL1**, and **Non-secure software cannot move interrupts to
Group 1** (`GICD_IGROUPR` is RAZ/WI from NS). Only secure boot code (an
armstub) can set Group 1. QEMU's GICv2 model has no effective security
split, so it "just works" there. This is general — it blocks **every**
peripheral IRQ on this firmware (PL011, a future UART2 IKBD IRQ, timer,
…), not just the console.

### Resolution

**Not fixed — and deliberately not pursued.** The serial console is a
**development-only** tool; the production system takes input from the
UART2 IKBD keyboard/mouse (already polled) and outputs to HDMI. Console
RX uses the **UART0 poll workaround** (`console::pl011_rx_poll` in the
blocking read + `pump_input`), which is fully functional. The IRQ buys
almost nothing in the current cooperative single-app model; it only
matters later (preemption, smooth mouse via background FIFO drain).

A self-built `armstub8-gic.bin` (raspberrypi/tools `armstub8.S`
`-DGIC=1 -DBCM2711=1`, which writes `IGROUPR=0xFFFFFFFF`) was tried but
**left IGROUPR=0** — it did not take effect on this firmware (reason
unconfirmed: the firmware may re-init the GIC after the stub, or the
self-built stub's layout mismatches what the firmware expects). Reverted.
**Correction:** a framebuffer + memory-split regression first blamed on
the armstub was actually an unrelated mistake — copying the project's
`config.txt` template over the card dropped `gpu_mem=256`, so the
VideoCore framebuffer went away and the ARM saw ~977 MiB instead of
~768 MiB. Restored `gpu_mem=256` (+ the `hdmi_*` lines) into
`boot/rpi4/config.txt`; the armstub itself broke nothing.

**When revisited** (multitasking / smooth mouse): get the GIC into
Group 1 via secure boot code — a *matching* prebuilt GIC armstub for the
installed firmware, or a firmware update, tested against the framebuffer.
To re-diagnose, dump `IGROUPR`/`ISPENDR`/`IPRIORITYR` at the GICD
(`0xFF841000`) and PL011 `RIS`/`MIS` (`0xFE201000`) while a key sits
undrained in the FIFO.

### Correction 2026-09-04 — "never delivered" is not established

The 2026-09-04 boot-screen measurement on real hardware (no keyboard, no
mouse, no serial adapter attached) found **one key word already queued in
the IKBD ring before the boot screen was drawn**, value `0x0000`, and the
PL011 in the state a completed drain leaves behind: `FR = 0x0091`
(`RXFE` = FIFO empty), `RIS = 0x0020` (no `RXRIS`/`RTRIS`, and no
`FE`/`PE`/`BE`/`OE` — so the received character was a clean `0x00` frame,
not a line break). Two independent readings of that:

* Only two functions can advance that ring: `console::pl011_rx_drain`
  (the INTID-153 IRQ) and `console::push_rx` (the IKBD decoder). The IKBD
  decoder is reachable only through `drivers::usb::poll`, which runs only
  from `Host::pump_input` / the blocking console reads — **none of which
  execute before `initinfo`'s wait loop takes that snapshot**
  (`kernel_main_rust` calls `usb::init` and then `initinfo::run`, whose
  drawing path makes no input call). So the drain ran.
* Had no drain ever run, the received character would still be in the RX
  FIFO (`RXFE` = 0) and `RTRIS` would still be raised — nothing else
  clears either. Both are clear, so the FIFO was drained.

⇒ **INTID 153 was delivered to the kernel at least once on this
machine+firmware.** The `IGROUPR[153] = 0` reading above does not refute
this: `GICD_IGROUPR` is RAZ from Non-secure state, so that read carries no
information — the "Group 0 ⇒ never delivered" step was an inference, not a
measurement, and the original symptom (typed serial bytes not arriving)
has other candidate causes (e.g. RXD0 wiring / the cleared pull on GPIO 15).
Treat "the PL011 RX IRQ is not delivered on real RPi4" as **unproven**
until re-measured directly. The consequence that mattered: `c6e4bfb` left
the IRQ→CON path in place *because* of this claim, and that path delivered
a phantom keystroke into CON on every boot (fixed — see STATUS 2026-09-04,
`console::serial_rx_byte`).

Also stale above: `console::pl011_rx_poll` no longer exists (deleted in
`c6e4bfb`); on real Pi-4 CON is the USB-HID keyboard alone.

### Source

BCM2711 ARM Peripherals §6 (GIC-400); ARM IHI 0048B §1.4 (GICv2 security,
Group 0/1 → FIQ/IRQ); raspberrypi/tools armstubs/armstub8.S `setup_gic`
(not in `references/`).

## QUIRK-003: EL0 entry — an IRQ between `msr elr_el1` and `eret` discards the jump target

**Date discovered:** 2026-09-23 (proven and fixed 2026-09-24, ticket ARCH01)
**Severity:** High (every program start; roughly one attempt in seven)
**Phase / context:** `kernel/src/arch/aarch64/user_entry.rs`,
`enter_el0_with_resume_capture` — the only kernel path that enters EL0
without an exception frame (double-click start, `\AUTO\`, `#Z` autostart,
nine self-tests)
**Status:** Fixed (measured on QEMU only; English since 2026-10-26, the
original German wording is in `docs/archive/memory-2026-10-26.md`)

### Symptom

After `[launch] EL0: slot 0 entry 0x1000` the start occasionally aborts:

```
[EXC] synchronous (non-SVC) exception (Instruction Abort (lower EL)):
      ESR=0x000000008200000e ELR=0xffff0000000c6988 SPSR=0x0000000000000000
[EXC] FAR=0xffff0000000c6988
```

`ELR` = `FAR` = a **kernel address** (high half), symbolised in
`enter_el0_with_resume_capture`; `SPSR` = `0`, i.e. the value we wrote;
ESR IFSC `0x0e` = permission fault, level 2 — EL0 executing a kernel page.
Frequency: **once in seven** runs of the same start script on an identical
card copy (`measurements/measurement-2026-09-23-desk19-desk20/`, RUNINFO §7).

### Cause

The instruction sequence set up the jump target and the target state **with
interrupts unmasked**:

```
msr sp_el0, {sp} ; msr elr_el1, {entry} ; mov {tmp},#0 ; msr spsr_el1,{tmp} ; eret
```

Exception entry writes `SPSR_ELx` from `PSTATE` **and the preferred return
address to `ELR_ELx`** (ARM ARM DDI 0487M.c, D1.4.2 "Exception entry", rule
R_DQXFW). If the 200 Hz system tick (or the PL011 IRQ) lands in this window,
`ELR_EL1` afterwards holds the interrupted **kernel PC**; the handler returns
into the middle of the block, the remaining instructions set
`SPSR_EL1 = 0` (EL0t) — and the `eret` jumps to EL0 at that kernel address.
The window is about four instructions wide and the tick comes every 5 ms:
hence rare, but unavoidable.

### Resolution

`msr daifset, #0xf` before the first of these writes
(`kernel/src/arch/aarch64/user_entry.rs`, `enter_el0_with_resume_capture`).
`MSR (immediate)` ORs the immediate into `PSTATE.{D,A,I,F}` (C6.2.288);
asynchronous exceptions are the only ones that can reach this window — the
instructions in it access no memory and cannot abort synchronously. The
mask **does not survive the `eret`**: an exception return sets `PSTATE` from
`SPSR_EL1` (D1.4.4, rule R_BWCFK; C6.2.157 `ERET`), and the `SPSR_EL1`
written is `0` — so EL0 continues with interrupts unmasked and takes the
pending IRQ at once.

Not affected and not touched: the **trap path**. A `Pexec` from EL0 only
rewrites the exception frame and returns through `EXC_BODY`
(`kernel/src/arch/aarch64/exception_vectors.s`); there `DAIF` has been
masked since exception entry (D1.4.2, rule R_WTXBY sets `{D,A,I,F}` to 1),
and no handler unmasks interrupts before — `gic::enable_irqs` is called
exactly once, at boot.

### Why this is hard to rediscover

The fault is not reproducible on demand and looks like a wrong jump target
or an MMU fault: `entry 0x1000` is logged correctly one line above. The only
unambiguous signature is the **combination** `ELR == FAR == kernel address`
**and** `SPSR == 0`: the SPSR value proves that our write went through, the
kernel address that the `ELR` write next to it was overwritten. Whoever
looks only at `FAR` searches the loader.

The proof went like this: in a **copy** of the tree a busy loop was put
between `msr elr_el1` and `eret`
(`measurements/measurement-2026-09-24-arch01/method/force_race_patch.py`,
~16.7 million iterations ≫ 5 ms). With it the fault hits **every** start,
with exactly the reported signature; `ELR`/`FAR` point at the loop's
`subs`. The same copy with `daifset` in front starts cleanly. Without this
step the fix would have stayed a guess.

### Source

ARM ARM DDI 0487M.c (Arm Architecture Reference Manual for A-profile, EAC,
from Arm's documentation service; not in `references/` at the time):
D1.4.2 "Exception entry" (R_DQXFW, R_WTXBY), D1.4.4 "Exception return"
(R_BWCFK), C6.2.157 `ERET`, C6.2.288 `MSR (immediate)`, C5.2.3 `DAIF`.
*Since 2026-10-04 issue M.d is pinned
(`references/arm-arm/DDI0487M_d_a-profile_architecture_reference_manual.pdf`,
`references/README.md`); checked 2026-10-26: R_DQXFW and R_WTXBY are under
D1.4.2 "Exception entry" and R_BWCFK under D1.4.4 "Exception return" there
too; the C-chapter numbers were not re-checked.* Measurement:
`measurements/measurement-2026-09-24-arch01/` (forced case before/after,
twenty clean start runs), finding `measurements/measurement-2026-09-23-desk19-desk20/`
RUNINFO §7.

### Template for new quirks

```markdown
## QUIRK-NNN: Short descriptive title

**Date discovered:** YYYY-MM-DD
**Severity:** Low | Medium | High | Critical
**Phase / context:** Where it bit us
**Status:** Open | Worked around | Fixed | Documented

### Symptom

What we observed. Specific, reproducible.

### Cause

What was actually happening, after analysis.

### Resolution

How we fixed or worked around it. Code reference if applicable
(commit hash, file:line).

### Why this is hard to rediscover

What made this non-obvious? What would a future contributor (or a fresh
agent session) miss?

### Source

How was the truth ultimately established? Datasheet section?
EmuTOS source comparison? Test program?
```

---

## Lessons Learned

Process and workflow lessons.

### LESSON-001: Source-first prevents drift

**Context:** This file itself, and `docs/SOURCE-ANALYSIS-PROTOCOL.md`.

**Lesson:** AI assistants and humans both default to "I know this" when
working on familiar APIs. The TOS API *seems* familiar to anyone with
Atari background, but plausible-sounding behavior is wrong often enough
that every spec entry must be source-backed. The protocol is annoying
but non-negotiable.

---

### LESSON-002: `rust-lld` does not accept `-nostartfiles`

**Context:** Phase 01, kernel `build.rs`.

**Lesson:** Passing `-nostartfiles` (a GCC/ld driver flag) to `rust-lld`
produces "unknown argument" and breaks the link. The `ENTRY()` directive
in the linker script is sufficient on its own; no equivalent flag is
needed.

---

### LESSON-003: Naked functions are stable since Rust 1.88

**Context:** Phase 01 `boot.rs`.

**Lesson:** `#[unsafe(naked)]` + `core::arch::naked_asm!` works on
stable-style nightly without any `#![feature(naked_functions)]` gate.
Adding the feature gate now produces a deprecation/already-stable
warning.

---

## Rejected Alternatives

Things we considered and chose not to do, with reasoning.

### REJECTED-001: Direct port of EmuTOS C source to AArch64

**Considered:** 2026-05-02 (initial planning)
**Status:** Rejected in favor of clean reimplementation

**Why rejected:** EmuTOS contains 25+ years of accreted m68k assumptions
(endianness, stack frames, register-direct optimisations). A direct port
would carry all of these implicit assumptions forward as latent bugs on
ARM. A clean reimplementation against the extracted TOS spec, with EmuTOS
source as the *primary reference*, surfaces these assumptions instead of
hiding them.

**What would change our mind:** Nothing realistic. This decision is
foundational.

---

### REJECTED-002: Building a project-specific MCP server

**Considered:** 2026-05-02 (early planning)
**Status:** Rejected; revisit only if a concrete need emerges

**Why rejected:** A custom MCP server exposing TOS API specs as
structured tools would be elegant but solves no problem the agent
cannot solve by reading Markdown files directly. Maintenance cost
would exceed convenience benefit.

---

### REJECTED-003: m68k JIT in the m68k-emulator phase

**Considered:** During Phase 10 design — "Phase 10" was the m68k-emulator
phase of the original plan (`phases/_archive/PHASE-10-m68k-emulator.md:94,109`);
today's PHASE-10 is the BIOS screen phase (corrected 2026-10-26).
**Status:** Deferred to a possible later phase

**Why rejected:** Interpreter throughput on modern ARM should easily
exceed 16 MHz 68000 equivalent for TOS-typical workloads (which spend
most time in OS calls, not in interpreted instructions). JIT adds
substantial complexity (code-cache, invalidation, debuggability) that
is not justified until measurements show the interpreter as the
bottleneck.

---

### REJECTED-004: `bindgen` over EmuTOS C headers

**Considered:** 2026-05-02
**Status:** Rejected (also in CLAUDE.md "Tech Stack")

**Why rejected:** `bindgen` would tempt us into linking C-defined types
across the m68k/ARM ABI boundary. NewTOS reimplements TOS cleanly in
Rust; types are defined in `tos-types` against the spec in `docs/api/`,
not lifted from EmuTOS headers.

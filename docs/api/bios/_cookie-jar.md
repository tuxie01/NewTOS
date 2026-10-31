# The Cookie Jar (system feature registry)

> **Phase-00b extraction (added for Phase-08 D5).**
> Source: EmuTOS @ `1a7484ab` — `include/cookie.h` (struct + tags),
> `util/cookie.c` (`cookie_init`/`cookie_add`), `bios/machine.c:665`
> (`fill_cookie_jar`), `bios/bios.c:277-280` (boot wiring),
> `cli/cmdutil.c:321` (`getcookie`, app-side access),
> `util/boot.c:72` (`get_cookie`, supervisor-side access).
>
> The cookie jar is **not a trap** — it is a system data structure that
> programs read through a fixed system variable. This document records
> what it is and, crucially, **how applications access it**, because the
> classic access path does not exist on NewTOS (see the NewTOS section
> and ADR-012).

## Structure

```c
struct cookie { ULONG tag; ULONG value; };   /* include/cookie.h:16 */
```

The jar is a contiguous **array** of `{tag, value}` entries. The last
entry is a **terminator** with `tag == 0` whose `value` field holds the
jar's **total slot capacity** (not a value). Readers scan from the start
until they hit `tag == 0`.

EmuTOS allocates a fixed 20-slot jar (`util/cookie.c:24`):

```c
static struct cookie dflt_jar[20];
void cookie_init(void) {            /* util/cookie.c:26, called bios.c:278 */
    dflt_jar[0].tag   = 0;
    dflt_jar[0].value = 20;         /* {0, capacity} terminator */
    p_cookies = dflt_jar;           /* publish via the $5A0 sysvar */
}
```

`cookie_add(tag, value)` (`util/cookie.c:34`) walks to the terminator,
and if there is room (`count < capacity-1`) writes `{tag, value}` into
the terminator slot and pushes a fresh `{0, capacity}` terminator one
slot down. So the jar grows in place; the capacity word is preserved.

## How it is built (`fill_cookie_jar`, `bios/machine.c:665`)

Called once at boot (`bios/bios.c:279`) **after** `cookie_init`. It runs
hardware detection and `cookie_add`s the results. Tags (`cookie.h`):

| Tag (ASCII) | Hex | Meaning | NewTOS relevance |
|-------------|-----|---------|------------------|
| `_CPU` | `0x5F435055` | m68k CPU type (0/10/20/30/40/60) | n/a (AArch64) — re-purpose or omit |
| `_VDO` | `0x5F56444F` | video shifter revision (ST/STe/TT/Falcon) | **meaningless** (no Shifter) |
| `_FPU` | `0x5F465055` | m68k FPU type | **meaningless** |
| `_MCH` | `0x5F4D4348` | machine type | kept, fixed to `MCH_NOHARD` = `0xFFFFFFFF` ("no Atari hardware", ADR-012 amendment 2026-10-28) |
| `_SND` | `0x5F534E44` | sound feature bitmap (PSG/DMA/DSP) | **meaningless** (no PSG) |
| `_FDC` | `0x5F464443` | floppy controller type | **meaningless** (no FDC) |
| `_AKP` | `0x5F414B50` | keyboard/language (low word) | meaningful (IKBD layout) |
| `_IDT` | `0x5F494454` | date/time display format | meaningful (NewTOS has a clock) |
| `_FLK` | `0x5F464C4B` | file/record locking present | meaningful (GEMDOS FS) |
| `_5MS` | `0x5F354D53` | 200 Hz timer vector (for MiNT) | n/a |
| `_NF`/`XHDI`/`NVDI`/… | — | extension drivers | out of scope |

Most standard cookies describe **Atari hardware NewTOS does not emulate**
(non-goal: no Shifter/PSG/FDC/m68k-FPU). A faithful NewTOS jar therefore
carries only the still-meaningful tags plus **its own** cookie(s) so
ported programs can detect the NewTOS environment and its real features.

## How applications access it — the critical part

There is **no GEMDOS/BIOS/XBIOS function** for the cookie jar in classic
TOS / EmuTOS. Access is always: **read the system-variable pointer at
`$5A0` (`_p_cookies`), then scan the array.** Reading `$5A0` is a
supervisor-only low-memory access, so two patterns exist:

**1. Application context** — must elevate via `Supexec` (XBIOS 0x26): inside
the callback, read the jar pointer from `$5A0`, then scan the array of
`(cookie, value)` LONG pairs and return the value of the entry whose `cookie`
matches the wanted tag; stop at the terminator (`cookie == 0`).
(`cli/cmdutil.c:321`)

**2. Supervisor context** — dereference `$5A0` directly and scan the same
array the same way. (`util/boot.c:72`)

So the access contract an application relies on is:
**`Supexec` + read `$5A0` + linear scan until `tag == 0`.**

## NewTOS implementation notes

> **Superseded in part, 2026-09-14** — point 1 below is no longer true: the
> B1 boot-init pass materialises the classic `$400` area, `$5A0` included, in
> every process's own address space. See the 2026-09-14 status section at the
> end of this file. The section is kept as written because the ADR-012
> decision it records was taken on this premise.

The classic access path is **blocked twice** on NewTOS:

1. **`$5A0` is not mapped.** NewTOS does not map Atari low-memory system
   variables onto AArch64 addresses (same decision as the `$420`/`$432`
   BIOS vars — direct poke is out of scope, m68k-emulator-only). So
   `*(COOKIE**)0x5A0` cannot work for native AArch64 programs.
2. **`Supexec` is deferred.** The AArch64 substitute for `Supexec` is an
   open decision (Open-ADR *ADR-Supexec*, blocks PHASE-06). Without it,
   even the elevation half of the app pattern is unavailable.

Therefore the cookie jar cannot be implemented by mirroring EmuTOS; it
needs (a) **NewTOS's own cookie set** (the meaningful tags + a NewTOS
identity cookie) and (b) **a NewTOS-native access mechanism** that
replaces "`Supexec` + `$5A0`".

**Decided (ADR-012, Accepted 2026-06-20):** the jar is **kernel-owned** —
allocated from the kernel heap, pointer held in a kernel-internal global
(no `$5A0`, no fixed address). Access is by **function returning content,
not a pointer**: `Getcookie(tag) -> (found, value)` (the kernel scans its
own jar; the jar is EL1-only memory so a pointer would be useless to EL0
anyway), with `Setcookie(tag, value)` to add/update. No user mapping, no
`Supexec`. This spec is the access-agnostic data model; the function ABI
+ the NewTOS cookie set are pinned in the implementation spec.

## Pi / BCM2711 binding

> Re-points the cookie set from **Atari-hardware detection**
> (`fill_cookie_jar` probing the Shifter / PSG / FDC / m68k CPU+FPU) to the
> **fixed NewTOS-Pi machine identity**, seeded once at boot. The data model
> (tag/value slots, `valid_tag`, enumeration) is unchanged; the *contents* and
> the *access mechanism* are pinned here. See `docs/HARDWARE-PROFILE.md` §3.7.

### The Pi cookie set (seeded at boot)

`tos_api::cookie::seed_defaults(version)` (`tos-api/src/cookie.rs:223-229@seed_defaults`),
called from `kernel/src/lib.rs:373@seed_defaults` with `version = 0x0000_0100`, seeds a
kernel-owned fixed array (`JAR_CAP = 32`, `tos-api/src/cookie.rs:20@JAR_CAP`) — **no** hardware
probing:

| Cookie | Value | Meaning | Source |
|--------|-------|---------|--------|
| `NTOS` | `0x00000100` | NewTOS identity + version 0x100 (the NewTOS-specific cookie; lets ports detect the environment) | `tos-api/src/cookie.rs:224@NTOS` |
| `_MCH` | **`0xFFFFFFFF`** (ADR-012 amendment 2026-10-28, TASK/ARCH06) | machine id: EmuTOS' `MCH_NOHARD`, "no ST-compatible hardware" (`references/emutos/include/cookie.h:50`, set by `setvalue_mch` without Atari hardware, `references/emutos/bios/machine.c:400-401`). Until ARCH06 it was `0x00050000`, the registered **ARAnyM** id (`references/emutos/include/cookie.h:57`), withdrawn. Read by value (`Ssystem(S_GETCOOKIE, '_MCH', 0)`) it comes back as `-1`, the same number as "not found"; only the pointer form (`E_OK`, `*arg2` = `0xFFFFFFFF`) tells the two apart | `tos-api/src/cookie.rs:250@MCH_NOHARD`, `tos-api/src/cookie.rs:225@MCH_NOHARD` |
| `_IDT` | `0x0000112E` | date/time format: `.` separator, DD-MM-YY, 24-hour — the pinned oracle's value (`tosdump/cases/jar-r1.tsv` case 53). User decision 2026-10-30 (TASK/COOK02.md): NEWDESK takes its date separator from `_IDT` when the date preference is "Standard" (`programs/desktop/src/model.rs:194-201@rendered`, as EmuTOS `references/emutos/desk/deskinf.c:140-145`). Until COOK02 `0x0000112F` (`/`) | `tos-api/src/cookie.rs:226@IDT` |
| `_AKP` | `0x00000000` | keyboard/language: US | `tos-api/src/cookie.rs:227@AKP` |
| `EdDI` | `0x00000001` | the VDI answers `vq_scrninfo`; the value is a **non-callable marker**, not a dispatcher address (E2 of [`../vdi/eddi-vq_scrninfo.md`](../vdi/eddi-vq_scrninfo.md); deviation from TOS/EmuTOS, `docs/COMPATIBILITY.md`) | `cookie.rs` `EDDI_MARKER` (2026-10-02) |

Deliberately **absent** (no matching Pi hardware, `tos-api/src/cookie.rs:216-222@seed_defaults`): `_VDO`
(no Shifter; its decision waits for the TeraDesk measurement of TASK/ARCH06),
`_SND` (no PSG/DMA sound), `_FDC` (no floppy controller), `_FPU` and `_CPU`
(no m68k FPU/CPU; both stay absent per the ADR-012 amendment 2026-10-28). A
program scanning for these correctly finds them missing and falls back.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| `_MCH` = ST/STe/TT/Falcon id from bus/Shifter probe | `_MCH` fixed: `0xFFFFFFFF` = `MCH_NOHARD` (ADR-012 amendment 2026-10-28; the earlier `0x00050000` was the ARAnyM id) |
| `_CPU`/`_FPU` = m68k CPU/FPU type | **omitted** (AArch64) — see decision below |
| `_VDO`/`_SND`/`_FDC` from hardware detect | **omitted** (hardware absent) |
| `_AKP` from the IKBD layout | `_AKP` = `0` (US), matches the loaded keymap default (§3.3) |

### Access mechanism (the critical part) — kernel-owned, no `$5A0`, no `Supexec`

> **Superseded in part, 2026-09-14** — "no `$5A0` mapping" no longer holds;
> see the status section at the end of this file.

The classic `Supexec` + read `$5A0` + linear-scan path is **blocked** on NewTOS
(no `$5A0` mapping; `Supexec` deferred — see the "How applications access it"
section and `docs/api/xbios/26-supexec.md`). Per **ADR-012**, the jar is
kernel-owned and accessed by **value**, through FreeMiNT `Ssystem` cookie
subfunctions (`docs/api/gemdos/154-ssystem.md`) that scan the kernel's own jar
(`cookie::get`/`count`/`slot_tag`, `cookie.rs:95-118`) and return the found
value — a pointer would be useless to EL0 since the jar is EL1-only memory.

### Decisions recommended

- **`_CPU`** — could carry an AArch64 marker so ports can branch on
  architecture. Recommend a **NewTOS-defined non-m68k value** (e.g. a sentinel
  distinct from the m68k `0/10/20/30/40/60` scale) *or* omit it; do **not**
  advertise an m68k CPU number. Decision open (`HARDWARE-PROFILE.md` §3.7 TBV).
- **`_SND`** — recommend **keep omitted** (no PSG/DMA/DSP; §3.9 Dosound is a
  no-op). A `_SND = 0` "no sound features" cookie is defensible but redundant
  with absence. Decision open.

### Gaps vs. the contract (Step-2 candidates)

- **`_CPU` / `_SND` undecided** (above) — the jar advertises 5 cookies (4 before `EdDI`, 2026-10-02); whether
  to add these two is unresolved.
- **`Setcookie` reseat** — `cookie::set` supports add/update within `JAR_CAP`,
  but no terminator-`{0, capacity}` slot is materialised in the **kernel** jar
  (the EmuTOS array-form terminator); enumeration uses `count()`/`slot_tag()`
  instead. *Corrected 2026-09-14:* the array-form terminator **is** materialised
  in the per-process `$5A0` mirror the loader writes (`{0, COOKIE_JAR_SLOTS}`,
  see the 2026-09-14 status section), so a binary scanning for `tag == 0` does
  find one — the earlier "would not find one" applied to the kernel jar, which
  no application ever sees.

## OFFEN (re-flagged 2026-07-23, BB1 boot-init pass)

**ADR-012's "kernel-owned, no `$5A0`, `Ssystem` instead of `Supexec`"
decision is now contested by evidence, not just noted as a limitation.**
`docs/analysis/sysvar-osheader-lowmem-gap.md` (Prüfauftrag Punkt 4) and
`docs/analysis/bios-audit/00-PLAN.md` (BB1) point out that TeraDesk's own
`sdk/apps/teradesk/library/utility/cookie.c:28,56` implements exactly the classic
`Setexc(0x5A0/4, -1)` + `$5A0` + linear-scan pattern this doc describes
above — i.e. a real, 1:1-ported TOS application (ADR-019) depends on the
access path ADR-012 explicitly closed off. This is the same class of
"two-worlds assumption refuted by a ported app" finding as the OSHEADER
correction in `docs/api/structures/osheader.md`. This Researcher pass
does **not** re-decide ADR-012 — that is BB1's job, not a spec-extraction
task — but the decision must be revisited with this evidence before BB1
implements the boot-init layer. See `docs/api/bios/_boot-init.md` §6.

## Status 2026-09-14 — what of ADR-012 still holds, and what does not

The BB1 re-flag above was answered in two steps, both driven by ported
applications rather than by argument.

### What still holds

* **The jar stays kernel-owned.** `tos-api/src/cookie.rs` is the single
  source of truth; nothing an application writes reaches it.
* **`Ssystem` stays the access ABI for native programs**
  (`docs/api/gemdos/154-ssystem.md`). A value-returning call is still the
  right answer for anything compiled *for* NewTOS.
* **There is no `Supexec` in the kernel.** XBIOS 0x26 still returns `38`;
  what changed is the *toolchain* binding (`TASK/X01.md`), because on
  AArch64 the low-memory area an app wants is already in its own address
  space — there is nothing to elevate for.

### What ADR-012 got wrong

* **`$5A0` is mapped after all.** ADR-012's premise "`$5A0` is not mapped,
  so `*(COOKIE**)0x5A0` cannot work" was overtaken by the B1 boot-init pass
  (`docs/analysis/bios-audit/b1-lowmem-feasibility.md` §Option 1): every
  process's TPA now carries the classic `$400` low-memory area, including
  `_p_cookies`@`$5A0` pointing at a per-process **mirror** of the kernel jar
  at VA `0x5F0` (`tos-api/src/bdos/process/loader.rs`, `write_cookie_jar`).
  TeraDesk's `sdk/apps/teradesk/library/utility/cookie.c:56` idiom therefore resolves.
* **The classic path is read-only, and that is now a decision, not an
  accident.** See the two subsections below.

### The mirror is live (fixed 2026-09-14)

Until 2026-09-14 the mirror was written **once**, at load time, so a cookie
added afterwards never reached a running program. `cookie::set` / `cookie::del`
now call `loader::refresh_cookie_mirrors()`, which re-writes the mirror of
**every** process that owns a TPA — not only the caller's: classic TOS has
exactly one jar that all programs see under the same address, so letting two
live programs observe different `$5A0` arrays would be a NewTOS-only artefact
of the per-process low-memory area. The hook sits *inside* `set`/`del` so a
future third mutator cannot forget it.

**Capacity / truncation.** The kernel jar holds 32 cookies
(`cookie::capacity()`); the mirror window `[0x5F0, 0x670)` holds
`COOKIE_JAR_SLOTS` = 16 slots = **15 cookies plus the terminator**. A longer
kernel jar is **truncated**, but the last written slot is always the classic
`{0, COOKIE_JAR_SLOTS}` pair, so an application's linear scan still ends
inside the window. The terminator's value is the *mirror's* capacity, because
EmuTOS' terminator word describes the array the reader is scanning
(`util/cookie.c:33`).

Evidence: host test `loader::cookie_mirror_tests`, kernel self-test
`selftest_sysvars` check (f), `apitest` SX11–SX15
(`measurements/measurement-2026-09-14-cookie/`).

### Still open: the counter-direction (application → kernel)

An application that writes into the mirror (TeraDesk's `install_cookie`,
`sdk/apps/teradesk/library/utility/cookie.c:107-179`, today inside the `#if 0` block at :74)
does **not** reach the kernel jar — and, since 2026-09-14, its mirror write is
now also **overwritten** by the next kernel-side `set`/`del`. Whether to close
this, and how without exposing the kernel jar to an application write, is an
open decision; the options and their costs are recorded with the task report.
Until it is decided: **`$5A0` is a read-only view.**

## OFFEN — the LP64 ABI properties of the classic idioms (measured 2026-09-14)

Neither of these is a malfunction. Both are consequences of a 32-bit-by-
construction data structure being read by an LP64 program, and both will hit
**every** 1:1 port. They are recorded here so nobody re-diagnoses them as bugs.

**1. An 8-byte read of `$5A0` is wrong.** `ramtop` sits directly above at
`$5A4`, so `*(long**)0x5A0` yields `(ramtop << 32) | jar` — measured
`0x3BE00000000005F0` (`apitest` T05). Only the **four**-byte read is correct;
that is what `Setexc(0x5A0/4, -1)` returns, so TeraDesk's live path gets the
right jar pointer (its walk still had point 2 until SDK04 C6, below). (Also noted at the write site,
`tos-api/src/bdos/process/loader.rs`, `$5A4` block.)

**2. A `struct { long name; long value; }` element has the wrong stride.** A
TOS cookie slot is a pair of **32-bit** LONGs — 8 bytes — and that is what the
mirror carries. The same declaration is 8 bytes on m68k but **16** on LP64
(TeraDesk `sdk/apps/teradesk/library/xdialog/library.h:201-205`), so a verbatim `cookie++` walk

  * reads every **second** slot, with a tag and its own value
    fused into one 8-byte word (`jar[0]` reads as `0x000001004E544F53` —
    `NTOS` with its value 0x100 on top), and
  * **does not recognise the terminator**: the `{0, capacity}` pair reads as
    `capacity << 32`, which is not zero. The scan only stops on the zero fill
    *beyond* the jar.

Measured by `apitest` SX16/SX17: `sizeof` = 16, the ported-stride scan takes
3 steps from `0x5F0` and stops at `0x620` — past the terminator at `0x610` —
without finding `NTOS`.

Consequence for TeraDesk: its `find_cookie` (compiled with `-D_TOS_ONLY`, so
`_MINT_ == 0` and the `Ssystem` branch is not compiled in —
`sdk/apps/build-teradesk.sh:102@D_TOS_ONLY`) found a cookie only in an even slot with value 0, and
then returned the next slot as its value (measured 2026-10-29: 3 of 12 tags agreed with `Getcookie`,
the three absent ones; `measurements/measurement-2026-10-29-sdk04-c6/RUNINFO.TXT`). It is driven on
every start (`sdk/overlays/teradesk/main.c:1861@find_cookie`, `CT60`) plus `sdk/apps/teradesk/xfilesys.c:1865` and
`sdk/apps/teradesk/video.c:158`. That stayed invisible because every tag it asks for
(`CT60`/`_FLK`/`_VDO`) is genuinely absent, so
"not found" was the right answer by accident. **Fixed for TeraDesk on 2026-10-29** (TASK/SDK04, C6) by
the patch `sdk/overlays/teradesk/library/utility/cookie.c`, which walks the slot as two 32-bit
LONGs: 12 of 12 agree. For other 1:1 ports the general fix remains
an ABI decision (declare the ported element as a pair of 32-bit LONGs in the
toolchain overlay, or give the mirror an LP64 form), not a kernel defect.

## Cross-references

- ADR-012 (cookie-jar access mechanism) — `docs/MEMORY.md` — **partly
  superseded; see "Status 2026-09-14" above for what still holds**
- Open-ADR *ADR-Supexec*, `$420`-vars decision — STATUS Recent-Decisions 2026-05-03
- `docs/api/bios/_boot-init.md` — boot-time `cookie_init()`/
  `fill_cookie_jar()` sequencing
- Compendium: "Cookie Jar"; Profibuch: "Cookie-Jar" — both document the
  `$5A0` + scan contract; cross-check pending.

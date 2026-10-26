# `OSHEADER` — Operating System Header

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `include/biosdefs.h:44-63`. Located at the start of the ROM/RAM OS
> image; pointer in low-memory `_sysbase` (0x4F2).
>
> **Re-validated 2026-07-23** (BB1 boot-init pass, see
> `docs/analysis/bios-audit/00-PLAN.md` / `docs/api/bios/_boot-init.md`).
> The struct's **actual assembly definition** is `bios/startup.S:73-114`
> (label `_os_header`, the literal first bytes of the TEXT segment); the
> `include/biosdefs.h:44-63` C `typedef` is the type only, not the
> instance. Two corrections applied below: the `os_version` "typically
> claims 0x0306" claim was **wrong** (EmuTOS' standard-profile default is
> `0x206`, not `0x306` — see the `os_version` section); and the
> "NewTOS implementation notes" closing section understated the scope —
> corrected per the TeraDesk boot-crash finding.

## Purpose

Top-level kernel descriptor exposed to applications. Programs read
`OSHEADER` fields to query OS version, locale, the current basepage
pointer, and the magic-cookie chain entry point.

The structure is at a **well-known fixed location**: low-memory long
at `0x4F2` (`_sysbase`) holds `OSHEADER *`. The OSHEADER itself is
typically the very first bytes of the ROM, with `os_entry` doubling
as the m68k reset vector landing pad (`bra.s reseth`).

## Binary layout (44+ bytes)

```c
typedef struct _osheader {
/* 0x00 */ UWORD            os_entry;    /* BRA.S to reset handler  */
/* 0x02 */ UWORD            os_version;  /* TOS version (BCD)       */
/* 0x04 */ PFVOID           reseth;      /* reset handler ptr       */
                                          /* --- following 12 bytes are the default GEM_MUPB --- */
/* 0x08 */ struct _osheader *os_beg;     /* base of OS image        */
/* 0x0C */ UBYTE            *os_end;     /* end of static OS ST-RAM */
/* 0x10 */ PRG_ENTRY        *os_rsvl;    /* reserved (was UI entry) */
/* 0x14 */ GEM_MUPB         *os_magic;   /* GEM MUPB ptr            */
/* 0x18 */ ULONG            os_date;     /* TOS date BCD MMDDYYYY   */
/* 0x1C */ UWORD            os_conf;     /* PAL flag + country code */
/* 0x1E */ UWORD            os_dosdate;  /* TOS date in BDOS format */
/* 0x20 */ WORD             **os_root;   /* ptr to BDOS quick pool  */
/* 0x24 */ UBYTE            *os_kbshift; /* ptr to BIOS kbshift var */
/* 0x28 */ struct _pd       **os_run;    /* ptr to current PD ptr   */
/* 0x2C */ ULONG            os_dummy;    /* 'ETOS' signature        */
/* 0x30 — end */
} OSHEADER;
```

### `GEM_MUPB` (12 bytes, embedded at offset 0x08)

```c
typedef struct {
    ULONG     gm_magic;  /* must be 0x87654321 (GEM_MUPB_MAGIC) */
    UBYTE    *gm_end;    /* end of static OS ST-RAM */
    PRG_ENTRY *gm_init;  /* main UI entry point (AES) */
} GEM_MUPB;
```

Pre-1.04 ROMs contained a `GEM_MUPB` directly inline in the OSHEADER
at offsets 0x08..0x13; from 1.04 onwards the inline three longs were
re-purposed (`os_beg`, `os_end`, `os_rsvl`) and `os_magic` at 0x14
points to a separately-located GEM_MUPB. EmuTOS keeps the modern
layout.

## Field semantics

### `os_entry` (0x00)

Two-byte `BRA.S reseth` opcode. The CPU executes this on cold reset
(via the reset vector at 0x00000000 → 0x00000004 → OSHEADER) which
implicitly fetches `os_entry` and branches into the actual reset
handler. **m68k-specific**; on AArch64 the reset path is utterly
different — NewTOS will populate this field with a meaningful pattern
only when the m68k user emulator runs.

### `os_version` (0x02)

BCD-packed: high byte = major, low byte = minor. Examples:
- `0x0102` → TOS 1.02
- `0x0206` → TOS 2.06
- `0x0404` → TOS 4.04

**Corrected (this pass):** the value is the build-time constant
`TOS_VERSION` (`bios/startup.S:80-81`, `.dc.w TOS_VERSION`). EmuTOS'
**default** in the standard-profile build (`include/config.h:1367-1372`) is
**`0x206`** ("we pretend to be TOS 2.06"), *not* 0x0306 as a prior draft
of this doc claimed. Neither `TARGET_512`, `TARGET_1024` nor `TARGET_PRG`
(NewTOS' reference build family) override it — verified in
`docs/analysis/aes-audit/emutos-config-profile.md` (Teil 1c/1l): none of
the three defines a `TOS_VERSION` block, so all three fall through to the
`0x206` default. `TOS_VERSION` is purely cosmetic in EmuTOS — the same
analysis found **no** `#if TOS_VERSION` branch anywhere in the source
tree; it only feeds this one field.

**NewTOS target value — DECIDED (2026-07-24): `0x0404` (TOS 4.04, Falcon).**
Per the user's target-profile directive (the reference profile is Falcon030 /
TOS 4.04 — memory `emutos-target-profile-falcon-tos4`), NewTOS deliberately
deviates from the EmuTOS-default `0x206` to the Falcon-TOS4 value `0x0404` —
the **same target-profile deviation** as `GEMDOS_VERSION`→`0x3000` (commit
`e162c62`). **Implemented in B1:** `loader.rs` `populate_sysvars` writes
`os_version = 0x0404` into the per-process OSHEADER instance (commit `3d1c1fe`,
`docs/analysis/bios-audit/b1-lowmem-feasibility.md`). Cosmetic in the sense
noted above (feeds only this field, no `#if TOS_VERSION` branch), but real
TOS4 apps read it via `_sysbase->os_version` — this is exactly the access B1
fixed (TeraDesk's boot-time version check). (Supersedes finding S3; the earlier
"OFFEN, do not implement from this doc alone" is resolved by the user directive.)

### `reseth` (0x04)

Reset handler pointer. EmuTOS: `.dc.l _main` (`bios/startup.S:82-83`),
the same target as `os_entry` (`bra.s _main`, `:78-79`) and `os_res1`
(+0x10, `:92-93`). Also used by EmuTOS itself as `swv_vec` after
monitor change (`bios/bios.c:552`, called at `bios/screen.c:1329`).
See `docs/api/bios/_reset-warm-cold.md` §3.

**NewTOS (RST01, 2026-10-17):** the EL0 VA of the page-0 reset stub (`0xA40`),
which calls `Ssystem(S_NEWTOS_RESET)` and ends in the restart
(`tos-api/src/bios/reset.rs`); the same value as `os_rsvl` (+0x10). `os_entry`
(+0x00) stays 0 — see `_reset-warm-cold.md` §6.

### `os_beg` (0x08)

Address of the OSHEADER itself (the OS image base). Programs use this
to locate themselves in memory; bootstraps use it to compute the OS
image size.

### `os_end` (0x0C)

End of statically allocated OS ST-RAM. The TPA begins here (rounded
up). On Atari hardware: end of OS RAM workspace, before any video
RAM. On NewTOS: end of kernel BSS, start of available RAM for user
processes.

### `os_rsvl` (0x10) — EmuTOS' `os_res1`

Reserved in the typedef (`biosdefs.h`; "was UI entry"). **CORRECTED
2026-10-17 (TASK/RST01, X04): the previous text here said "EmuTOS sets
`os_rsvl = 0`". That is wrong.** The assembly definition stores
`os_res1: .dc.l _main` (`bios/startup.S:92-93`), and `reseth` (+0x04) is
`_main` as well (`bios/startup.S:82-83`). `_main` is `bra.w _realmain`
(`bios/startup.S:115-116`), i.e. the full restart path. A program that jumps
to `os_beg+0x10` (TeraDesk's `*((long *) os_start + 4)` on m68k, where
`long` = 4 bytes: `sdk/apps/teradesk/main.c:2096`) therefore **restarts
the machine** on EmuTOS; the AES entry is reached via `os_magic → gm_init`,
not through this field. The startup comment (`bios/startup.S:84-87`, "os_end and
os_res1 below are never used") only says EmuTOS does not read the field
itself. Full restart semantics: `docs/api/bios/_reset-warm-cold.md` §3.
The same three entry points (`os_entry`, `reseth`, `os_res1`) all end in
`_main`.

### `os_magic` (0x14)

Pointer to the active `GEM_MUPB`. Bootstraps walk this to find the
AES entry point at boot. NewTOS will only populate this once
GEM/AES is reimplemented (PHASE-12 and beyond).

### `os_date` (0x18)

TOS build date, BCD `MMDDYYYY`. EmuTOS sets it from the build
timestamp: `tools/mkheader.awk` generates `obj/header.h` with
`#define OS_DATE 0x08292026` for a build made on 2026-08-29. Programs
sometimes use this to detect ROM version when `os_version` collides
between vendors.

**NewTOS (DESK12, 2026-09-23):** the same arrangement, one build script
instead of an awk pass — `tos-api/build.rs` emits the date, and
`tos_api::build::OS_DATE` packs it, which is what
`bdos::process::loader` writes. `SOURCE_DATE_EPOCH` pins it for a
reproducible build. It was a 1980-01-01 placeholder until then.

### `os_conf` (0x1C)

```
bit 0       : 0 = NTSC, 1 = PAL
bits 1..7   : country code (0=USA, 1=DE, 2=FR, ...)
bits 8..15  : reserved
```

Set at build time from `CONF_LANG` / `CONF_PAL`. NewTOS-relevant:
country code drives keyboard layout selection in
`bios/country.c`. PAL/NTSC bit drives the default video mode in
classic Atari hardware; on NewTOS this bit will be ignored once
PHASE-08 sets video mode from the BCM2711 framebuffer config.

### `os_dosdate` (0x1E)

Same date as `os_date` but in FAT-packed format
(see `docs/api/structures/ofd.md` `DOSTIME`): `(year-1980) << 9 |
month << 5 | day`, which `obj/header.h` writes as `23837` for
2026-08-29. NewTOS builds it from the same stamp as `os_date`
(`tos_api::build::OS_DOSDATE`).

### `os_root` (0x20)

Pointer to the BDOS "quick pool" head. Internal to GEMDOS memory
allocator; not generally consumed by applications. EmuTOS exposes it
for compatibility with debuggers and disk caches that walk the pool.

### `os_kbshift` (0x24)

Pointer to the single byte that holds the current keyboard shift
state (the same value `Kbshift` returns). Programs that poll
keyboard modifiers without trapping read this byte directly. The
byte itself lives at `0xE1B` in classic TOS; EmuTOS places it
wherever the BIOS chose, and exposes the address here.

### `os_run` (0x28)

Pointer to the **pointer** to the currently-running PD. So:
```c
PD *current = *(osheader->os_run);
```

The double-indirection is so that the PD-pointer storage location
itself stays fixed while the value (current PD) updates on every
process switch / `Pexec`. Programs read this to obtain their own
basepage when they were not given it via the m68k entry-stack
convention.

`os_run` is a static link-time pointer to the GEMDOS variable `run`
(`bios/startup.S:110-111`, `.dc.l _run`), which lives in the **BDOS**
layer, not BIOS: `run = &initial_basepage;` at GEMDOS boot
(`bdos/bdosmain.c:339`), `run = (PD *)p;` on every `proc_go`, i.e. `Pexec` modes 0/4/6 (`PE_LOAD`/`PE_BASEPAGE*`
leave `run` alone; `bdos/proc.c:575`), `run = run->p_parent;` on `Pterm`
(`bdos/proc.c:609`). Verified this pass (`docs/api/bios/_boot-init.md`
§4).

### `os_dummy` (0x2C)

Four-byte signature. EmuTOS sets it to `'ETOS'` (`0x45544F53`) so
applications can distinguish EmuTOS from Atari TOS. NewTOS will set
it to `'NTOS'` (`0x4E544F53`).

## Position in low memory

```
0x000000  reset vector  → 0x000004
0x000004  initial PC    → OSHEADER + 0x00 (os_entry, BRA.S reseth)
...
0x0004F2  _sysbase      → &OSHEADER  (UBYTE*)
0x0005A0  ...
```

`_sysbase` is the documented mechanism for finding the OSHEADER
without knowing its address. It is a **runtime write**, not a static
value: `sysbase = &os_header;` (`bios/bios.c:358`), executed once during
`bios_init()`'s "misc. variables" block, before `chardev_init()` and long
before any program (Shell/AES/AUTO-folder) runs — see
`docs/api/bios/_boot-init.md` §3/§4 for the full boot-order citation.

## Cross-references

- **Compendium:** "OSHEADER" / "_sysbase" — agrees on layout.
- **Profibuch:** "OS-Header" — agrees, including 1.04 transition.
- **`docs/api/structures/basepage.md`** — `os_run` reaches PD.
- **`docs/api/xbios/_dispatcher.md`** (TBD ref) — XBIOS doesn't read
  OSHEADER but lives in the same image.
- **EmuTOS source:** `bios/startup.S` (low-memory layout),
  `bios/bios.c:os_header` (OSHEADER definition).

## Configuration

- `CONF_LANG` — drives `os_conf` country code.
- `CONF_PAL` — drives `os_conf` bit 0.
- Build date drives `os_date`/`os_dosdate`.

## NewTOS implementation notes

**CORRECTED (this pass, 2026-07-23) — the original note below was
wrong and is exactly the finding that triggered this whole audit
phase.** It read: *"Empty during Phase 00. PHASE-11 / PHASE-12 will
populate the m68k-visible OSHEADER once the user emulator can run
programs that read `_sysbase`."* That framing assumed a "two-worlds"
split — native ports read state through API calls, only m68k-emulated
binaries touch raw low memory — and deferred the OSHEADER instance to
the (much later) m68k emulator phase.

That assumption does not hold. Per
`docs/analysis/sysvar-osheader-lowmem-gap.md` and
`docs/analysis/bios-audit/00-PLAN.md` (BB1): real, 1:1-compiled TOS
source ports (ADR-019) read `_sysbase`/OSHEADER directly, exactly like
an m68k binary would — TeraDesk's `sdk/apps/teradesk/main.c:56`
(`#define os_start (*(long*)0x4F2L)`) is a concrete, reproduced
boot-crash example (Data Abort dereferencing an unpopulated `_sysbase`).
So a real, kernel-populated OSHEADER instance (with the fields/values
in this document) is needed **much earlier** than a Phase-11/12 m68k
emulator — it is required for *any* native EL0 program built from
ported TOS C source that performs the classic `_sysbase`/`os_version`
check. This document (the field layout, offsets, and EmuTOS values) is
the target the BB1 audit implements against; deciding *where* the
struct and the `_sysbase` pointer live in a per-process AArch64 VA
layout is BB1's architectural decision, not this spec-extraction
pass's. See `docs/api/bios/_boot-init.md` for the full boot-time
population sequence this document's fields come from.

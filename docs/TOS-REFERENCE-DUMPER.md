# TOS Reference Dumper — Task Specification

Build a set of programs that capture the **observed** behaviour of Atari TOS 3.06
on real hardware and compare it against NewTOS, so that NewTOS conformance is
verified against measured reality instead of against a reading of the EmuTOS
source.

This document is the single source of truth for these tasks. Do not infer
requirements from other documents unless this file points at them.

---

## 1. Why this exists

Today NewTOS is validated by reading the EmuTOS source. That means the
Researcher, the Developer and the Auditor can all read the same passage the same
wrong way: three roles, one failure mode. A dump taken from a real TT breaks that
loop because it is measured behaviour, not interpreted intent.

It also catches a class of bug that cannot be read out of any source at all:
a routine writing an output field it is not allowed to touch. That is an absence,
and absences are only visible if you poison the arrays first.

**Non-goals.** This is not a test suite for drawing correctness, not a
performance benchmark, and not a GEM application. It captures call-level
input/output behaviour and nothing else.

---

## 2. Hard constraint: two builds, one source

NewTOS is native AArch64 and dispatches its API through `svc`. TOS 3.06 on the TT
is m68k and dispatches through traps. The same source compiles for both, but the
resulting binaries are different objects.

Therefore:

- one shared source tree
- one shared case table format
- one shared record format
- two thin ABI backends, and nothing else may differ

If a difference in results can be explained by the backends rather than by the
API implementations, the whole exercise is worthless. Keep the backends as small
as physically possible: marshal registers, execute, return. No logic.

```
tosdump/
  cases/           # case tables (data, shared, never compiled in)
    vdi.tsv
    aes.tsv
    bios.tsv
    gemdos.tsv
  core/            # portable C: runner, poison, record writer
  abi/
    m68k_trap.c    # TOS 3.06 target
    newtos_svc.c   # NewTOS target
  host/            # comparator, runs on the development machine
  tests/           # host-side tests (see rules in section 8)
```

### 2.1 Endianness and type widths

Because two different builds are involved, the risk is no longer "can it be
built" but "do the two builds mean the same thing". Two hazards, both of which
produce differences that look like real bugs:

**Endianness.** m68k is big-endian; AArch64 normally runs little-endian. The
record format is big-endian. The AArch64 build must therefore byte-swap on write
and the m68k build must not. This is the **only** permitted asymmetry between the
two builds. Implement it as a single `store_be16` / `store_be32` pair in `tosdump/core/`,
selected at compile time, and route every field through it — including reserved
fields and the header. A swap bug here silently fabricates differences or hides
them, and it will be mistaken for an API bug.

*Test for this before trusting any dump:* a host-side test that feeds a known
pattern through the record writer for both configurations and asserts the two
outputs are byte-identical.

**Type widths.** VDI and AES arrays are 16-bit words; return values are 32-bit.
Use `int16_t` and `int32_t` throughout. Never `int`, `short` or `long` — their
widths are not guaranteed to agree between the two toolchains. Do not rely on
struct layout for the record: write fields individually at explicit offsets, so
that padding and alignment rules cannot differ between targets.

---

## 3. Record format

Binary, big-endian, fixed-size records. **No formatting happens on the target.**
Every byte of interpretation happens in the host comparator, where there are
tools and a debugger. The target program must be as stupid as possible; every
line of logic there is a bug you cannot chase.

### File header — 32 bytes

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | magic `TDMP` |
| 4 | 2 | format version (currently 3; 2 is still read, see below) |
| 6 | 2 | area: 1=BIOS, 2=XBIOS, 3=GEMDOS, 4=VDI, 5=AES, 6=VDI-RASTER (the raster record, 3696 bytes, layout `TD_RAST_OFF_*` in `tosdump/core/tosdump.h`). `DUMPBIOS` writes area 1 for its BIOS **and** XBIOS cases, the trap in each record (section 3, the call record); area 2 is written by no program |
| 8 | 2 | record size in bytes |
| 10 | 2 | record count |
| 12 | 4 | build id (git short hash as ASCII, zero padded) |
| 16 | 4 | case table CRC32 |
| 20 | 2 | pointer width of the writing machine in bytes: 4 or 8 (format 3) |
| 22 | 2 | word order of the writing machine: 1 = most significant WORD first (m68k), 2 = least significant WORD first (AArch64) (format 3) |
| 24 | 8 | reserved, must be zero |

The comparator **must refuse** to compare two files whose area, record size,
format version or case table CRC differ. Comparing dumps produced from different
case tables silently produces nonsense.

#### Format versions

| Version | Change | Read today |
|---|---|---|
| 1 | first layout | no — refused |
| 2 | the trap's own return value (`d0`) in the reserved LONG after the opcode (`tosdump/core/tosdump.h`) | yes, without the address check |
| 3 | pointer width and word order in header bytes 20–23; **nothing else** — every record is byte for byte what format 2 wrote | yes |

Format 3 exists for the **address** field class (next section). An address the
target laid into a WORD array sits there in the target's own memory order and
at its own pointer width: EmuTOS on the m68k records `ap_3resv` as
`0x0000 0xBD60` (most significant WORD first), NewTOS on AArch64 as
`0x80B0 0x0011` = `0x001180B0` (least significant WORD first); an AArch64 MFDB
pointer in the raster CONTRL is four WORDs, an m68k one two. Without both facts
the WORDs cannot be put back together, so format 2 cannot say what an address
cell holds. Both values are **measured by the dumper on the machine** —
`sizeof(void *)` and a `memcpy` of a LONG into two WORDs
(`td_native_ptr_width`, `td_native_word_order`, `tosdump/core/record.c`) — with the
same lines on both targets, so the backends stay symmetric (section 8, rule 5).
`tosdiff` refuses a header whose pointer width is not 4 or 8 or whose word
order is not 1 or 2.

- **Format 2 stays readable.** `tosdiff --validate` accepts it, and two
  format 2 dumps still compare by value as before. The address check is not
  run on them — reading their address cells would mean guessing width and
  order — and the report says so explicitly per dump:
  `address check, <dump>: not checkable -- format 2 states no pointer width
  or word order`.
- **Format 3 against format 2 is refused** (exit 2,
  `refusing: <a> is format 3, <b> is format 2 -- ... re-record the older
  one`), either way round. The address check would run on one side only, and
  the format 2 side cannot be made to state what it lacks.

The two BIOS + XBIOS references `tosdump/reference/EMUTOS-1a7484ab-BIOS`
and `-BIOS-falcontc` (2026-10-27) were recorded in format 3 from the start
(`measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`). The fourteen
VDI, AES and raster references `tosdump/reference/EMUTOS-1a7484ab-*`
are format 3 since 2026-10-26, re-recorded on the oracle. The twelve VDI and
AES payloads are byte-identical to their format 2 recording
(`measurements/measurement-2026-10-26-ref07-v3/RUNINFO.TXT`). The two raster
references `EMUTOS-1a7484ab-RASTER*` cannot be: their MFDB pointer cells
(address class, the dumper's own buffer addresses) move with the dumper's code
size. They were replaced under the condition "payload byte-identical except the
header and the address cells", the exclusion set derived from the field
descriptors (4 cells per record), every other byte equal
(`measurements/measurement-2026-10-26-raster-v3/RUNINFO.TXT`). The frozen
`EMUTOS14-*`, `TOS306-*` and the existing `NEWTOS-*` dumps stay format 2.

#### Field classes: value and address

Every field descriptor (`td_*_fields`, `tosdump/core/record.c`) carries a class.

- **value** — compared element by element between the two dumps.
- **address** — an address the target chose itself, which cannot be equal
  across two machines or two builds by construction: `ap_3resv`
  (`global[11..12]`, EmuTOS `pglobal->ap_3resv = (LONG)&D`,
  `aes/gemsuper.c:98`) and the two MFDB pointers packed into the raster CONTRL
  (`contrl[7..14]`, the dumper's own buffers). **Never compared by value.**
  Instead each **cell** is checked on each side for a relational property:

  **every address cell must be `< 0x80000000`.** A TOS address is a signed
  32-bit `LONG`; one at or above `0x80000000` reads as a negative error code to
  a caller that tests `< 0` (`TASK/MEM11.md`). A violation is its own verdict,
  printed under the case as `global[11..12]  ADDRESS <dump> = 0x80000000, not
  < 0x80000000` and counted in a per-dump summary line
  (`address check, <dump>: format 3, pointer N bytes, ...; K address cells
  checked, V not < 0x80000000`); it is **not** a value difference and does not
  count towards "N of M cases differ". `tosdiff` exits 1 when there is a value
  difference, an address violation or a kept violation (below), 0 only when
  there is none of them.

  What a cell is comes from the descriptor (`addr_cell`) and the header: a
  **LONG** cell is two WORDs on every target (`ap_3resv` — the AESGLOBAL keeps
  its 15 WORDs for an LP64 caller, `tos-api/src/aes/appl.rs`); a **pointer**
  cell is the header's pointer width, packed from the field's start as gemlib
  packs it (pointer *n* at word *n* · width / 2). The pointer field is sized
  for eight-byte pointers; on a four-byte target the WORDs after the last cell
  (`contrl[11..14]` on the m68k, which keep the poison) are no address and are
  not checked.

  **The kept check** — a second relational property, again within one side.
  The raster record holds the MFDB pointer slots twice: `contrl_mfdb` after
  the call and `contrl_before_mfdb` as they went in. The VDI reads the two
  MFDB pointers and must not write them, so every address cell of
  `contrl_mfdb` must equal its twin in `contrl_before_mfdb` (cells as above:
  `contrl[7..8]`, `[9..10]` on the m68k, `contrl[7..10]`, `[11..14]` on
  AArch64). Both values come from one run of one dumper, so their equality
  measures the API, not the linker. The pair is named in `tosdiff`
  (`kept_pairs`, `tosdump/host/tosdiff.c`) and resolved against the
  descriptors; a name that does not resolve to two address fields of one
  shape makes `tosdiff` refuse (exit 2) instead of silently skipping the
  check. A violation is printed under the case as
  `contrl_mfdb[0..3]  KEPT <dump> = 0x..., before the call 0x...` and counted
  in a per-dump summary line (`kept check, <dump>: format 3; contrl_mfdb
  against contrl_before_mfdb, K cells checked, C changed by the call`); it is
  not a value difference, and `tosdiff` exits 1 on it. A format 2 dump is
  reported as `kept check, <dump>: not checkable`, like the address check.

### VDI record — 1592 bytes

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | case id (line number in tosdump/cases/vdi.tsv) |
| 2 | 2 | opcode |
| 4 | 4 | reserved, zero |
| 8 | 24 | `contrl[12]` after the call |
| 32 | 256 | `intin[128]` after the call |
| 288 | 512 | `ptsin[256]` after the call |
| 800 | 256 | `intout[128]` after the call |
| 1056 | 512 | `ptsout[256]` after the call |
| 1568 | 24 | `contrl[12]` **before** the call |

Input arrays are dumped **after** the call on purpose: a VDI routine is not
allowed to modify its inputs, and dumping them afterwards is what makes a
violation visible.

### AES record — 120 bytes

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | case id |
| 2 | 2 | opcode |
| 4 | 10 | `contrl[5]` after |
| 14 | 30 | `global[15]` after |
| 44 | 32 | `int_in[16]` after |
| 76 | 14 | `int_out[7]` after |
| 90 | 12 | `addr_in[3]` after — or the buffer token, see section 6 |
| 102 | 4 | `addr_out[1]` after |
| 106 | 14 | reserved, zero |

### BIOS / XBIOS / GEMDOS record — 320 bytes

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | case id |
| 2 | 2 | opcode |
| 4 | 32 | input arguments, 8 longs as passed |
| 36 | 4 | return value (d0) |
| 40 | 256 | capture buffer (see case table `capture` column) |
| 296 | 24 | reserved, zero |

**Extended 2026-10-27 (DUMPBIOS, task T7)** — before the first dump of this
record was taken, so no existing dump changes. The record keeps its 320
bytes and offsets 0–295; six of the 24 reserved bytes now carry two fields
the comparison needs:

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | case id (physical line number in the case table) |
| 2 | 2 | opcode = the function number (`Getrez` = 4) |
| 4 | 32 | the 8 arguments as passed, one LONG each: a WORD sign-extended, a LONG as is, a buffer argument as its token `0xB0FF0000 + n` (never its address, which differs per machine by construction — the AES `addr_in` rule, section 6), a callback argument as `0xB0FC0000 + n`, the `Super` token argument as `0xB0F50000` (both section 6, "Super and Supexec"), the unit token `u:0` as `0xB0F40000` (section 6, "The closing tables"), a token `t:<n>` as `0xB0F50000 + n` and a string argument `s:`/`c:`/`x:` as `0xB0F60000 + n`, `n` its index in the table's string pool (both section 6, "GEMDOS after a media change"), an unused slot `0xDEADDEAD` |
| 36 | 4 | return value, **low** 32 bits of the return register (`d0` / `x0`) |
| 40 | 256 | capture (section 6, `capture` column); what no capture wrote stays `0xDEAD` |
| 296 | 4 | return value, **high** 32 bits of the return register: `x0` bits 63..32 on AArch64; on the m68k, whose `d0` has no high half, the sign extension of the low half — the value C's `(int64_t)(long)` gives on both targets, so the comparator always reads the return as the 64-bit value `ret_hi:ret` |
| 300 | 2 | trap number: 13 BIOS, 14 XBIOS, 1 GEMDOS (in a BIOS-area table only `Super`, section 6); 256 a jar case, which issues no trap (section 6, "The cookie jar") |
| 302 | 18 | reserved, zero |

Why the high half: NewTOS returns a TOS `LONG` in a 64-bit register
(`tos_api::trap`, "return value in `x0` (sign-extended `i64`)"), and an LP64
caller compares the whole register — `if (Bconstat(2) < 0)` on a `long`.
A negative error code returned zero-extended, or a pointer above 4 GiB, is
invisible in the low 32 bits and is exactly what the comparison must see.

Why the trap number: one program, `DUMPBIOS`, measures both traps (section
7), from one table into one dump. The dump's header carries area 1 (`BIOS`)
for both; the trap of each record is in the record. Area 2 (`XBIOS`) stays
defined and is not written by any program.

The field descriptors (`td_call_fields`, `tosdump/core/record.c`) keep every
field of this record in class **value**: `args`, `capture` and `trap` are
compared by value; `ret`, read together with `ret_hi`, is the one field
whose comparison the case's `retclass` refines (section 6) — EXACT is the
value comparison, the other four classes replace it by a property on each
side.

---

## 4. Poisoning

Before every case, fill **all** arrays and the capture buffer with the 16-bit
pattern `0xDEAD`, repeated.

This is what makes the method work. If TOS leaves a field poisoned and NewTOS
writes it, a plain byte comparison of the two dumps shows the difference with no
extra logic. Spec violations of the "wrote a field it must not touch" kind fall
out for free.

Do not skip poisoning for fields you believe are always written. That belief is
exactly what is being tested.

---

## 5. State reset

Before every case, reset the state the call could depend on. For VDI that means
at minimum `v_clrwk` plus re-applying the default attributes.

Without this, a result depends on the preceding cases and the whole dump becomes
invalid the moment somebody inserts a case in the middle. The reset sequence
lives in `tosdump/core/` and is identical on both targets.

**Workstation handling for VDI:** open with `v_opnvwk` on the handle returned by
`graf_handle`. This is what real applications do, and matching application
behaviour is the point. Record the returned handle in the dump but exclude it
from comparison — handles legitimately differ between machines.

---

## 6. Case tables

Tab-separated, one case per line, `#` starts a comment. Loaded at runtime from
the same directory as the executable. **Never compiled in** — the case table must
be provably identical for both runs, and recompiling between runs makes that
unprovable.

### tosdump/cases/vdi.tsv and tosdump/cases/aes.tsv

```
opcode <TAB> sub <TAB> intin values (comma separated) <TAB> ptsin values <TAB> comment
```

Comparison is exact for every byte of these two areas. No exception mechanism —
if you find yourself wanting one, the case is wrong, not the comparator.

The AES form has grown two columns since (the four control counts stated
explicitly) and one optional one; `tosdump/core/cases.h` is the authority for it. The
optional one is the point of the section after next.

### Line kinds: comments, directives, preludes, cases

Added 2026-10-26 from `tosdump/core/cases.h`, where the rules have lived since
2026-08-30 (preludes) and 2026-09-06 (`!reset`); nothing here is new, it was
only missing from this spec.

Every line consumes a line number, because **a case id is the physical
1-based line number** and nothing may renumber the cases (`tosdump/core/cases.h`
header). Four kinds of line:

- **Comment** — first non-blank character `#`, or an empty line.
- **Directive** — first non-blank character `!`. Carries no call. Two exist,
  for AES tables only:

  ```
  !reset   <TAB> none | wind_new
  !columns <TAB> 7 | 8
  ```

  Each must stand before the first case or prelude, at most once, and an
  unknown value is refused, never defaulted (`tosdump/core/cases.h`, "The
  directives"). `!columns 8` adds the `addr_in` column (next section).
  `!reset` names **which** reset the runner issues; the sequence itself is in
  `tosdump/core/` (`td_reset_aes`, `tosdump/core/runner_aes.c`), so both targets run the
  identical one. `wind_new` is opcode 109 with all control counts 0, as gemlib
  lays it (`sdk/gemlib/gemlib-0.44.0/a_wind_new.c:25`; `OUT_LEN 0`, because `xif` copies `OUT_LEN`
  words out of an uninitialised stack buffer, `gemsuper.c:430-432,444-445`).
  It is the AES' own reset: `wm_new` closes and deletes every window with
  handle ≥ 1 and drains the `wind_update` locks (`gemwmlib.c:1875-1893`), and
  EmuTOS issues it itself after every graphics program the shell runs
  (`gemshlib.c:676-677`). The runner issues the declared reset before every
  case and once after the last (`tosdump/core/runner_aes.c`, both `td_reset_aes`
  calls). **The table declares it, not the program,** because the dumper is
  double-clicked on the TT without arguments and always reads `AES.TSV`: the
  file, whose CRC the dump header carries, is the only thing that tells two
  rounds apart. **Absent means `none`,** so the frozen round-1 table
  (`tosdump/cases/aes-r1.tsv`, CRC `33C13C8F`) still measures exactly what its dumps
  recorded. What `wind_new` does not reach (`gl_newdesk`, `gl_aname`/
  `gl_ainfo`, `gl_wtcolor`/`gl_wbcolor`, `D.w_win[0]`, only reset by
  `wm_init`/`wm_start`) stays out of the tables through the guards below.
- **Prelude** — the opcode field starts with `+`. A call issued after the
  reset and before the next case's measured call, and **not recorded**. It is
  what makes an effect measurable rather than an echo: a setter's echo only
  proves the echo; a read-back after a non-default set proves the effect.
  Preludes stack (several may precede one case, at most `TD_MAX_PRELUDES` =
  256 per table; a VDI prelude carries at most `TD_PRELUDE_VALUES` = 16
  `intin` and 16 `ptsin` values, an AES prelude the case's own `int_in`
  limit), and a prelude with no
  case after it is an error (`TD_CASE_ERR_DANGLING`). The refusal and
  parameter guards apply to preludes as to cases. The raster table
  (`vdi-raster.tsv`) refuses `+` lines; the screen table allows a fixed
  whitelist of preludes (`tosdump/core/cases.h`, raster and screen sections).
- **Case** — everything else: one measured, recorded call.

A malformed line of any kind is an error, never a skipped case: a table that
half-loads produces a dump that compares against the wrong cases.

### The `addr_in` column — naming a buffer instead of an address

A dozen AES opcodes do all of their work behind a pointer: the object manager,
the menu manager, the scrap manager, `appl_find`, `rsrc_load`, the shell's path
calls, `wind_get`/`wind_set` with `WF_NAME` or `WF_INFO`. None of them could be
stated here, because a case table is **one file carried to two machines** and an
address is different on each. Until 2026-09-12 the runner simply poisoned the
address array and the tables stayed away from those opcodes.

The way out is the one `tosdump/cases/vdi-raster.tsv` already uses for its two pixel
buffers: **the table names a buffer by NUMBER and the dumper fills in its own
address.** An AES table opts in with a directive, so that every table written
before it keeps its meaning byte for byte:

```
!columns <TAB> 8

opcode <TAB> n_intin <TAB> n_intout <TAB> n_addrin <TAB> n_addrout
       <TAB> intin values <TAB> addr_in <TAB> comment
```

The `addr_in` column is `-`, or a comma separated list of `<where>=<buffer>`:

- `aN=B` — buffer `B` into `addr_in[N]`. Only where the case declares
  `AIN_LEN > N`; the dispatcher copies `min(AIN_LEN, AI_SIZE)` slots onto its
  local buffer (`gemsuper.c:439`) and a plant past it never reaches the handler.
- `iN=B` — buffer `B` as a pointer packed into the `int_in` WORDs at word `N`,
  at the **target's own pointer width**. That is the classic GEM binding
  (gemlib `sdk/gemlib/gemlib-0.44.0-m68k/a_wind_set_str.c:25`) and the only place EmuTOS' `wm_set` looks for a
  `WF_NAME` string (`gemwmlib.c:1661`). **Preludes only**: a packed pointer
  occupies two WORDs on the m68k and four on AArch64, and `int_in` is recorded,
  so on a measured case the record would differ by the target's pointer width
  and not by anything the AES did. `IN_LEN` must cover the wider of the two.

The buffers are the runner's own statics: `TD_AES_BUF_N` of them,
`TD_AES_BUF_BYTES` each, laid **before every call** with eight printable
characters, a NUL and a canary of the same pattern (`td_aes_buf_fill`). A bounded
string and not the raster's word fill, because half of the calls this unlocks
read a C string through the pointer and neither AES bounds-checks one.

**A planted `addr_in` slot records the buffer's TOKEN, not its address** —
`TD_AES_ADDR_TOKEN(n)` = `0xB0FF0000 + n`. The address differs between the two
machines by construction, so recording it would fabricate a difference in every
case that uses one, exactly as a byte-pattern poison would. A slot the call
modified records the modified value instead, which is the difference it should
be. A slot no case planted still records the poison, which is what keeps a table
without the column producing the bytes it always did.

What the column does **not** buy: the 120-byte AES record has no room for a
buffer's CONTENTS, so a call whose only effect is what it wrote into the buffer
is still only measurable through its return code. Recording the buffer would
mean a different record size, and a different record size invalidates every dump
already taken.

### The refusal guard, and where a MEASURED question gets answered

A case table is data, and a typo in it must not hang the machine it is carried
to. So the AES loader refuses a whole table -- named reason, offending line --
when it names an opcode that would block on the operator, end the application,
or reach the dispatcher's default arm (`fm_show(ALNOFUNC)`, a modal alert with
nobody to click it: `gemsuper.c:406-417`). `tosdump/core/cases.c` holds the per-opcode
citations.

Five opcodes do not belong there, and that is the point of this section.
`objc_sysvar` (48) is `#if CONF_WITH_3D_OBJECTS`; `menu_popup`/`attach`/
`istart`/`settings` (36-39) are `#if CONF_WITH_MENU_EXTENSION`. Whether a given
machine has them is not a property of the TABLE — it is a property of the
MACHINE, and EmuTOS' own `AES_VERSION` ladder (`include/config.h:1345-1354`)
reads it off a number the machine hands out: `ap_version`, which `appl_init`
writes into the AESGLOBAL (`gemsuper.c:92`).

Until 2026-09-13 the loader refused all five, reasoning "TOS 3.06 = AES 3.20".
That locked out functions the reference EmuTOS measurably **has** — it reports
`ap_version 0340`, and the ladder makes 0x0340 mean all four `CONF_WITH_*` are
on. So the question is now split at the only line where it can be split:

| asked by | when | what it can answer |
|---|---|---|
| the loader (`td_aes_cases_parse`) | host or machine, before anything runs | is this opcode issuable at all — and which capabilities does the table need (`features`) |
| the machine (`td_aes_cases_version_refusal`) | after `appl_init`, before the first case or prelude | does THIS AES have them |

A machine that does not have them refuses the **whole** table, so a dump has
`count` records or there is no dump — the shape of a dump stays a property of
its table alone, which is what the CRC binding and the comparator assume.
`td_run_aes` asks the same function itself, so a program that runs cases cannot
forget to ask.

The version test is **membership, not `>=`**: 0x0340 is the only rung that
requires `CONF_WITH_3D_OBJECTS`, 0x0330 and 0x0340 the two that require
`CONF_WITH_MENU_EXTENSION`. The implication runs one way — a rung tells you
which flags were on, not the reverse — and a number the ladder never produces
establishes nothing. Refusing there costs a table that could perhaps have run;
issuing there costs a hung machine.

`CONF_WITH_PCGEM` (`appl_yield` 17, `scrp_clear` 82, `shel_rdef` 126,
`shel_wdef` 127) is **not** in the ladder, so `ap_version` cannot settle it
either way and those four stay refused by the loader. `CONF_WITH_WINDOW_COLOURS`
and `CONF_WITH_GRAF_MOUSE_EXTENSION` remove no opcode at all — they gate fields
and modes inside opcodes that exist either way. That is a per-parameter
question, but it is not the parameter guard's either: "does this build have
window colours" is still a measured property of the machine, so measuring those
needs a third shape — a per-case, per-feature refusal on the machine side. Not
built.

`casecheck` runs on a host with no machine, so it does not refuse such a table:
it prints what the table needs, and enforces it only against a version the
caller states (`casecheck --aes --aes-version 0320 TABLE`).

### The parameter guard: what the VALUES decide (added 2026-09-13, M07)

The guard above judges a NUMBER. For a dozen opcodes there is no such judgement
to make, because the same opcode is a poll or a hang, a read of the dumper's own
buffer or a write through `0xDEADDEAD`, depending on one WORD of `int_in` or on
whether the `addr_in` column named a buffer. An opcode-level verdict on those is
wrong in both directions at once — it refuses the harmless arm and it waves the
dangerous one through — so a second question, *"with THESE values?"*, is
answered per line in `aes_line_parameter_refusal` (`tosdump/core/cases.c`), for preludes
exactly as for cases:

| opcode | what the values have to say |
|---|---|
| `wind_get`/`wind_set` (104, 105) | the field, the handle, the geometry; `WF_NAME`/`WF_INFO` re-opened by a plant (M05) |
| `wind_open`/`delete`/`update` (101, 103, 107) | the handle, the message-queue bound, the mouse-control balance |
| `evnt_multi` (25) | `MU_TIMER` with a zero `MT_LOCOUNT`/`MT_HICOUNT` is the one arm that returns before `mwait` (`gemevlib.c:216-220` against `:232-262`); `MU_MESAG` needs a planted `MME_PBUFF` |
| `form_error` (53) | `n > 63` is the one arm that returns before `fm_show` (`gemfmlib.c:440-441`) |
| `form_keybd` (55) | the tree must be planted AND `FM_ICHAR` must not send `find_obj` walking it (`:167-190`, `:135-148`) |
| `objc_delete` (41) | `ROOT` is the one object that returns before the tree is read (`gemoblib.c:1154-1157`) |
| `scrp_read`/`scrp_write` (80, 81) | a `strcpy` each way through `addr_in[0]`, so the slot must be planted (`gemsclib.c:40-45`, `:57-62`) |

**All of it is decided from one line, so it lives in the LOADER** and not in
`td_aes_cases_version_refusal`: the flags, the field, the error number, the
character, the object index and the plants are all columns of the line. Nothing
here needs a measured quantity, so a table can still be checked on the host
before it is carried anywhere. Same seam as the version guard, taken from the
other side.

**The shape rule**, and why it is not tidiness: where the guard reads an
`int_in` WORD, the case must state it AND declare `IN_LEN` past it. `xif`'s
`int_in` is a LOCAL array copied into only as far as `min(IN_LEN, I_SIZE)`
(`gemsuper.c:428-438`), so a WORD outside `IN_LEN` is uninitialised stack, not
the number the table wrote.

**The asymmetry it exists for.** A table is written on a host, tried on NewTOS,
and then carried to a TT with no memory protection. Where the two AESs disagree
about whether a parameter is fatal, trying it on one proves nothing about the
other: `objc_delete(-1)` answers 0 on NewTOS (`tos-api/src/aes/object.rs`) and
reads `tree[-1].ob_next` through the poison on the TT. The guard therefore
refuses the UNION of what either machine would find fatal, never the
intersection — which is also why `form_keybd`'s character test covers both
EmuTOS' whole-WORD compare and NewTOS' byte-wise one.

### tosdump/cases/bios.tsv and tosdump/cases/gemdos.tsv

```
opcode <TAB> args <TAB> retclass <TAB> capture <TAB> comment
```

`retclass` is one of:

- `EXACT` — return value must match byte for byte
- `IGNORE` — not compared (times, tick counts)
- `NONZERO` — must be non-zero on both
- `HANDLE` — must be >= 0 on both, value not compared
- `PTR_RAM` — must be even (WORD aligned), at or above `0x400` and inside RAM on both, value not compared

`capture` is either `-` or `<argindex>:<length>`, meaning: after the call, copy
`length` bytes from the buffer pointed to by argument `argindex` into the record.
This is how `Fread`, `Fsfirst` and `Getcookie` results get compared even though
the pointer itself cannot be.

The lower two areas need this classification because BIOS and GEMDOS return
pointers, handles and timestamps that differ legitimately between a TT and a
Pi 4. VDI and AES do not — they are number arrays at fixed positions, which is
why they get the strict treatment.

### The call table, as built for DUMPBIOS (added 2026-10-27, T7)

The five columns above, made precise. `tosdump/core/cases.h` ("call tables")
is the authority; `tosdump/cases/bios-r1.tsv` the first table, with its
ledger `tosdump/cases/COVERAGE-BIOS.md`.

**The opcode column names the trap too:** `<trap>:<function>`, e.g. `14:0x04`
for `Getrez`. One table holds the BIOS and the XBIOS (section 7), and the same
function number means different calls on the two traps. A BIOS-area table
accepts traps 13 and 14, and trap 1 for function `0x20` (`Super`) only — the
processor-state call, see "Super and Supexec" below; GEMDOS proper waits for
`DUMPGDOS`, except the calls a destructive disk table may make after a media
change ("GEMDOS after a media change" below, TOOL20). The trap lands in the
record (section 3).

**`args`** is `-` or up to eight comma separated arguments, in the order of the
binding, each with its width, because the m68k stack frame is laid by width
(`trap_14_wwl` and friends, `sdk/libc/mintlib/include/mint/osbind.h`) and a
wrong width is a different call:

- `w:<v>` a WORD, `-32768..65535`
- `l:<v>` a LONG, `-2147483648..4294967295`
- `b:<n>` a pointer to the dumper's own buffer `n` — `TD_CALL_BUF_N` = 2
  buffers of `TD_CALL_BUF_BYTES` = 1024 bytes, statics of the runner,
  poisoned with `0xDEAD` before every case. The table names a buffer by
  number and the dumper puts its address in (the AES `addr_in` rule); the
  record carries the token, never the address.

  The poison is laid **in the unit the case's capture reads** (below): the
  bytes `DE AD` for unit B and for a buffer no capture reads, native WORDs
  `0xDEAD` for W, native LONGs `0xDEADDEAD` for L (`poison_buffer`,
  `tosdump/core/runner_call.c`). One fill cannot read the same in every unit
  on two byte orders — the bytes `DE AD` are the WORD `0xADDE` on AArch64 —
  and an element the call left alone must read the same on both machines,
  or the poison becomes a difference. The byte fill is also what a call that
  reads the buffer as bytes must see on both (`Protobt`'s checksum over a
  disk-format sector).

How a backend passes them (section 2, marshalling only): the m68k pushes the
WORDs and LONGs right to left under the function number and issues
`trap #13/#14`; AArch64 loads `x0..x7` left to right — a WORD sign-narrowed,
a LONG sign-extended, a pointer whole — and the function number into `x8`,
exactly as the retargeted mintlib bindings do (`osbind.h`, "NewTOS (AArch64)
trap layer"). Argument slots the case does not use are `0xDEADDEAD` in the
AArch64 registers, the counterpart of whatever lies above an m68k frame.

**`retclass`** is one of the five classes above, optionally followed by the
return value's width: `EXACT.W`, `NONZERO.B` … — no suffix means LONG. The
width is the return type of the EmuTOS handler (`WORD getrez(void)`,
`bios/screen.c:1076`; `UBYTE dsp_hstat(void)`, `bios/dsp.c:1273`), because
`d0` above the returned type is not defined by the m68k calling convention:
the VDI's `d0` precedent (`td_vdi_fields`, "the compared field is the half the
contract defines"). A void handler gets `IGNORE`. What each class checks, on
the return value read as the 64-bit `ret_hi:ret` (section 3) and cut to the
class width, sign-extended:

| Class | The check | A failure prints |
|---|---|---|
| `EXACT` | equal on both sides. At LONG width the whole 64-bit value: a negative LONG that NewTOS returns zero-extended in `x0` is a difference | `ret EXACT <a> = … <b> = …` |
| `IGNORE` | nothing | — |
| `NONZERO` | non-zero, on each side | `ret NONZERO <dump> = 0x…, is zero` |
| `HANDLE` | `>= 0`, on each side | `ret HANDLE <dump> = 0x…, is negative` |
| `PTR_RAM` | on each side: non-zero, `>= 0x400`, `< 0x80000000`, and even | `ret PTR_RAM <dump> = 0x…, <which property>` (`is null`, `below 0x400`, `not < 0x80000000`, `is odd`) |

"Inside RAM" in `PTR_RAM` is checked as far as a dump can state it: as the
address class's relational property (`< 0x80000000`, section 3,
`TASK/MEM11.md`) plus non-null. A dump carries no RAM bounds of its machine;
the property "between the machine's RAM base and its top" would need them
and is not checked. `PTR_RAM` takes no width suffix: a pointer is a LONG.

*Lower bound `0x400`* (`TD_PTR_LOW` beside `TD_ADDR_LIMIT` in
`tosdump/core/record.h`; `TASK/TOOL11.md`, proposal
`docs/analysis/ptr-ram-rule-2026-10-28.md` Option A): EmuTOS's
`sysvars_start` (`references/emutos/tosvars.ld:31`). Below it lie the 68000
exception vectors and the processor-state save area, no structure a BIOS or
XBIOS call returns a pointer to; the smallest `PTR_RAM` value in the pinned
references is `0x30D4` (`Iorec(1)`). The bound catches a function number or a
small constant returned in place of a pointer — `Iorec` returning its own
`0x0E`, `Bconmap(-2)` its `0x2C` — which the rule let through while it only
asked for non-null, even and `< 0x80000000`. NewTOS's answers in its page 0
above the bound (`Setexc(0x168)` `0x5F0`, `Keytbl` `0x768`) stay valid. It
cannot tell that a structure lies behind an address above the bound; only a
capture through the pointer can (`r:`, section 6, "The disk round"). The
checks run in the order null, upper bound, lower bound, odd; the first that
fails is printed. One function holds the rule, `td_ptr_ram_violation`
(`tosdump/core/record.c`), which the comparator and the runner's `r:` read
share.

*Even, not four-byte aligned* (user decision 2026-10-27): the first rule
asked for a multiple of 4, and the reference itself failed it for three of
its eight `PTR_RAM` cases on the TT profile — `Iorec(2)` `0x30E2`, `Keytbl`
`0x40AE`, `Bconmap(-2)` `0x4A76` — and a fourth on `falcontc`, `Iorec(0)`
`0x478A` (`measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`): m68k
structures are WORD aligned. An odd address stays a failure. The self-check
below keeps such a rule from reaching a NewTOS comparison again.

A retclass failure **counts as a case difference** ("N of M cases differ")
and makes `tosdiff` exit 1, like a value difference of `args`, `capture` or
`trap`.

### The oracle self-check

A comparison rule is only valid if the oracle passes it (user decision
2026-10-27). `tosdiff --selfcheck CASES.TSV ORACLE.BIN` applies every
relational rule — the address check, the kept check and the per-side
retclasses (`NONZERO`, `HANDLE`, `PTR_RAM`) — to the oracle reference alone
and exits 1 if one fails. Such a failure is a bug in the rule or the case
table, routed to the Researcher, **never a NewTOS finding**.
`tosdump/tests/test_selfcheck.sh` runs it in `make -C tosdump host-test` over
every pinned `EMUTOS-1a7484ab-*` reference, each against the table whose CRC
it carries; a new reference or a new rule is checked on the spot.

**`capture`** is `-` or `<argindex>:<length>[.<unit>][@<offset>]`:

- `argindex` is 0-based and must name a `b:` argument;
- `unit` is `B` (default), `W` or `L` — the element type the call writes
  there. The buffer is read as elements of that type in the machine's own
  order and each element goes into the record through `td_store_be16` /
  `td_store_be32`, exactly as the raster record stores its pixel words: a
  LONG `Dsp_Available` writes is four bytes in opposite order on the two
  machines, and only the element value means the same on both;
- `length` is in bytes, `1..256`, a multiple of the unit;
- `offset` (default 0) is in bytes, a multiple of the unit, and
  `offset + length <= TD_CALL_BUF_BYTES`. `Protobt` writes its checksum at
  510 (`bios/floppy.c:731-735`), past the 256 bytes the record has room for.

Or `<argindex>:<layout>` / `r:<layout>`: field by field through a layout
("Structures behind a pointer", below).

**No preludes, no directives** — outside a disk table. A `+` or `!` line is
refused, except the `!disk` directive and the Rwabs-read preludes of a disk
table ("The disk round", below), and `!layout`, `!write return` and `+put`
("Structures behind a pointer", below). A call table issues no reset either: the
admission rule of a round-1 table is that every case is a query, or a call
whose effect does not outlive it (ledger), so there is no state to restore.
A round that sets state needs a restoring prelude, and gets it with its own
table form.

**The guard is a whitelist.** A typo in this table can write a disk sector
(`Rwabs`), install a vector (`Setexc`) or hang the machine on input
(`Bconin`). The loader (`td_call_cases_parse`) therefore refuses the whole
table — named reason, offending line — unless every line is a call whose
safety on **both** machines has been established in the ledger:
the trap and function are on the list, the argument widths match the
binding, and the values select the query arm (`Kbshift(-1)`,
`Setcolor(n,-1)`, `Bconin` only on the handles that return at once on both
machines, …). Everything else is "not established" until a later round
establishes it; there is no override.

### Super and Supexec in a BIOS-area table (added 2026-10-27, MEM12)

`Super` (GEMDOS `0x20`) and `Supexec` (XBIOS `0x26`) change or use the
processor state, which no query above does. They get their own table,
`tosdump/cases/super-r1.tsv` (run with `DUMPBIOS SUPER.TSV`; header area 1,
like every BIOS-area dump), so `bios-r1.tsv`, its CRC and its references stay
byte for byte as they are. Why they are measured: the NewTOS super state
(`docs/api/_super-state.md` S3–S6, TASK/MEM12).

**Two more argument kinds**, both standing for something only the dumper has:

- `f:<n>` — the dumper's own callback `n`, `0 .. TD_CALL_CB_N - 1`
  (`TD_CALL_CB_N` = 3), passed as a pointer; the record carries
  `0xB0FC0000 + n`. The callbacks are portable C in `tosdump/core/runner_call.c`,
  the same lines on both machines:

  | n | returns | measures |
  |---|---|---|
  | 0 | `0x12345678` | the callback's LONG comes back (S6) |
  | 1 | `-2` | a negative LONG comes back sign-extended in the 64-bit register (S6, section 3 `ret_hi`) |
  | 2 | what `Super(1L)` answers inside the callback | the callback runs in supervisor / super state: `-1` (S6) |

  Callback 2 makes a GEMDOS call from inside `Supexec`, which EmuTOS forbids
  in general (`bios/xbios.c:767-773`). `Super(1L)` is the exception: the
  TRAP #1 front end answers it before the BDOS is entered
  (`bdos/rwa.S:159-184`, `x20_inq` `:340-346`), so nothing the restriction
  protects is reached.
- `t:0` — the token the last `Super(0L)` case returned, passed as a LONG;
  the record carries `0xB0F50000`.

**The bracket.** `Super(0L)` leaves the processor in supervisor state, which
outlives the case — what a call table does not otherwise allow. It is
admitted as one half of an explicit bracket that the same table closes:

| line | means | rule |
|---|---|---|
| `1:0x20  l:1` | `Super(1L)`, the inquiry | anywhere |
| `1:0x20  l:0` | `Super(0L)`, opens the bracket; the runner keeps the return as the token | refused while a bracket is open |
| `1:0x20  t:0` | `Super(token)` = `SuperToUser`, closes the bracket | refused unless a bracket is open |

Every other `Super` argument is refused: an explicit stack address makes
EmuTOS run on that address (`x20_usr`, `bdos/rwa.S:280-301`). A table that
ends with the bracket open is refused (`TD_CASE_ERR_UNSAFE_STATE`). The cases
between the two halves run in supervisor state on the m68k — the runner's own
code included, on the stack `Super(0L)` left it (the old USP), which is why
the opening and the closing call come from the same call site with the same
frame: EmuTOS's `x20_sup` then finds the USP where it left it
(`bdos/rwa.S:305-336`, "already at old USP").

**Return classes.** `Super(1L)` returns `0` or `-1` (`x20_inq`): `EXACT`.
`Super(0L)` returns the old supervisor stack pointer on EmuTOS and a token on
NewTOS (S4): an address, `NONZERO`. It stays `NONZERO`: the cases were
recorded when `PTR_RAM` still asked for four-byte alignment, which an m68k
stack need not meet. `SuperToUser` returns nothing defined on EmuTOS:
`x20_sup` does not write `d0` (`bdos/rwa.S:305-336`), so the register holds
whatever the caller left there: `IGNORE`; whether it left supervisor state is
measured by the `Super(1L)` case after it. `Supexec` returns the callback's
value: `EXACT`.

**Guard.** `Supexec` is admitted with exactly one argument and only an `f:`
one (signature letter `f`); `Super` with `l:1`, `l:0` or `t:0` under the
bracket rule. Nothing else in GEMDOS is admitted -- outside a destructive disk
table, which admits the calls of "GEMDOS after a media change" (below).

### The disk round (added 2026-10-28, TOOL07)

`Rwabs`, `Getbpb` and `Mediach` reach a BIOS block drive. They are measured
by their own table, `tosdump/cases/bios-r2.tsv` (run with `DUMPBIOS`, header
area 1), on a test image only: the oracle's profile `disk`
(`docs/analysis/ORACLE.md` §2, the test image as IDE master C:, the dumper on
D:) and, on NewTOS, a card from `tools/mkcard.sh` with `MKCARD_TESTDISK` (the
same test image as partition C:, the dumper on a program partition D:). The
table, its CRC and the references of round 1 stay as they are. Ledger:
`tosdump/cases/COVERAGE-BIOS.md`, section "Disk round".

**The directive.** `!disk <TAB> <drive>` before the first case or prelude,
at most once, `<drive>` 2 (C:) … 15 (P:): this table is a disk table and
`<drive>` its test drive. Without it, `Rwabs`, `Getbpb`, `Mediach` and every
`+` line are refused as in round 1 (`call_refusals`,
`tosdump/core/cases.c`); any other `!` line is refused in every call table.

**Preludes.** In a disk table a `+` line is a prelude, and a prelude is one
thing only: an `Rwabs` read (no `RW_WRITE`, no `RW_NOTRANSLATE`) into a
dumper buffer. Its `retclass` and `capture` columns are `-` — it is not
recorded. Preludes stack before the next case (at most
`TD_MAX_CALL_PRELUDES` = 32 per table); one with no case after it is an
error (`TD_CASE_ERR_DANGLING`). The runner poisons the frame and both
buffers **once per case, before its preludes**, so what a prelude read is
what the case finds in the buffer (`td_run_call`,
`tosdump/core/runner_call.c`). **A prelude that does not return 0 stops the
run**: no further call is issued, the LOG names the line and its return,
and no BIN is written — the buffer would hold poison where the sectors
should be, and the write after it would put the poison on the disk.

**The guard for the three calls**, in a disk table only:

| Call | Signature | Admitted |
|---|---|---|
| `Getbpb` (13:7), `Mediach` (13:9) | `w` | any drive but A: and B: (a floppy without a disk ends in the critical-error handler on a real TOS, and the disk profile has no floppy drives) |
| `Rwabs` (13:4) | `wpwwwl`, the AHDI 3.1 long form only | flags from `RW_WRITE` 1, `RW_NOMEDIACH` 2, `RW_NORETRIES` 4, `RW_NOBYTESWAP` 0x80; **`RW_NOTRANSLATE` (8) only with the unit token `u:0`** (signature `wpwwul`, "The closing tables" below) — a literal unit number stays refused, it is the machine's. A `b:` buffer with count 0..2 (two sectors fill one buffer), not on A:/B:; or `l:0` (NULL), the FastCopy arm, only as a read on A:/B: with count 0..2 — EmuTOS returns before any I/O (`bios/blkdev.c:425-428`) |
| `Rwabs` **write** | as above, a measured case | **write-back only**: on the test drive, and the **last** prelude before it read exactly the same buffer, count, `recno`, drive and `lrecno`. A write is never a prelude |

Write-back only means the test image keeps every byte even where a write
goes through: what reaches the disk is what was just read from the same
sectors, or nothing (the stopped run). This is what makes the
forcechange handshake measurable (writing logical sector 0, BIOS-STOR-001)
without a restore step. Writes that change the image — patched boot
sectors — are the closing tables' (below), never an ordinary disk table's.

**Capture through the returned pointer.** The capture column also takes
`r:<length>[.B|.W|.L][@<offset>]`: copy `length` bytes from `offset` of the
address the call **returned** (Getbpb's BPB, `r:18.W` = its nine WORDs),
element by element as for a buffer capture. Only with retclass `PTR_RAM`,
so the address is checked on each side; the runner reads only through a
return that passes `PTR_RAM` itself (the same function the comparator
uses, `td_ptr_ram_violation`) — a NULL, an error code, a function number
or a 64-bit value above 4 GiB leaves the capture poisoned instead of
faulting the dumper (`TASK/TOOL11.md`). The address becomes a C pointer
through the backend's `td_abi_addr` (`tosdump/core/abi.h`): the identity on
both machines; the host tests' stub maps a window of modelled TOS memory
there, since a 64-bit host process has no memory below 4 GiB.

**Drvmap** is `EXACT` in this table: both setups give C: + D:,
`0x0000000C` (user decision 2026-10-28; round 1 keeps `NONZERO`).

### The closing tables (added 2026-10-28, TOOL07)

A `Getbpb` that refuses a boot sector leaves EmuTOS at `recsiz = 0`
(`bios/blkdev.c:601`, `:740`), after which no logical `Rwabs` reaches the
disk (`bios/blkdev.c:443-444`): a sequence that breaks C:'s boot sector
cannot put it back the way it came. User decision 2026-10-28 (variant B):
**one closing table per such sequence**, which ends with the image changed;
the end state itself is measured. Tables, references and the run:
`tosdump/cases/bios-r2-x004.tsv`, `-x005.tsv`, `-x006.tsv` (BIOS-STOR-004,
-005, -006) and `bios-r2-phys.tsv` (BIOS-STOR-002, not destructive), references
`EMUTOS-1a7484ab-BIOS-r2-<x004|x005|x006|phys>-disk`,
`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`. Three additions,
nothing else:

**`!disk <TAB> <drive> <TAB> destructive`** marks a table that changes the
test image. Without the third word the directive is the one above; any
other word is refused (`TD_CASE_ERR_DIRECTIVE`). **Runner rule: a
destructive table runs only on a fresh clone of the test image**, on the
oracle and on NewTOS, and both runners refuse otherwise:

- `tools/hatari-oracle.sh` refuses it in a profile other than `disk`, clones
  `$HATARI_ORACLE_DISK` for the run into the new out-dir (as for every disk
  table), and refuses an image whose partition 1 fails `fsck_msdos -n` (the
  boot sector of a previous destructive run does).
- `cargo xtask tosdump` (`tools/xtask/src/tosdump.rs`) refuses it without
  `--disk`/`--testdisk`; the card is built from the test image for the run and
  booted as an APFS clone; a `--testdisk` image must pass `fsck_msdos -n` on
  its test partition first. The test partition after the run is kept as
  `<out>/testpart-after.img` for the end-state comparison (the caller deletes
  it); a changed partition and a failing `fsck_msdos` after the run are the
  table's purpose there, not a failure.

Both read the mark more broadly than the loader (any whitespace separates),
so a table the loader takes as destructive is never run as an ordinary one.
The dumper cannot tell a fresh image from a used one; it logs
`disk table: …, destructive (a fresh clone of the test image only)`.

**`+set <TAB> <offset>=<byte>[,<offset>=<byte>…] [<TAB> comment]`**, only in
a destructive table: a prelude that changes the buffer of the read prelude
**immediately before it**; it issues no trap. Offset and byte decimal or
`0x` hex; at most eight pairs (`TD_CALL_SET_PAIRS`); every offset inside what
that read read (`count × 512` bytes, so not after a count-0 read); no offset
twice; a byte 0..255. The case after it is the write that writes the buffer
back: the write-back rule then names the read before the `+set` (same
buffer, count, `recno`, drive, `lrecno`) — what reaches the disk is "read +
set". Refused: a `+set` in a table that is not destructive
(`TD_CASE_ERR_UNSAFE_STATE`), one that does not follow a read prelude, a
second `+set`, any prelude after it, and a case after it that is not that
write (`TD_CASE_ERR_UNSAFE_FIELD`). The runner applies it between the
poison and the case, as the reads (`td_run_call`,
`tosdump/core/runner_call.c`).

**`u:0`, the unit token** (BIOS-STOR-002; `docs/api/bios/04-rwabs.md`,
"Decisions"): an `Rwabs` `dev` argument meaning "the physical unit of the test
disk". It is the machine's — EmuTOS' IDE master is `2 + 8 × 2` = 18
(`bios/disk.h:30`), NewTOS' SD card is 26 — so the **run's profile names it**
and nothing is compiled in (the two builds still differ in byte order only,
section 2.1): `tools/hatari-oracle.sh` profile `disk` puts `18` into
`D:\DISKUNIT.TXT`, `cargo xtask tosdump --disk` puts `26` there
(`NEWTOS_DISK_UNIT`). DUMPBIOS reads `DISKUNIT.TXT` from its current directory
only for a table that names `u:0` — one decimal number 0..32767, an optional
line end — and logs `unit token u:0 = physical unit <n> (DISKUNIT.TXT, the
run's profile)`; without it the table is not run and no BIN is written
(`TD_RUN_CALL_ERR_UNIT` in the runner). No search at run time. The
argument travels as a WORD; the record carries `0xB0F40000`, never the
number, so the two sides compare. `RW_NOTRANSLATE` is admitted only with the
token, and then only as a measured read: a `b:` buffer, count 0..2, flags
`RW_NOMEDIACH` and `RW_NORETRIES` besides (no `RW_WRITE`; no `RW_NOBYTESWAP`,
which reaches the driver in physical mode); never in a prelude, never as
another argument.

**End state.** After each destructive table the test partitions of both sides
are compared (`compare-ends.sh` of the run, the method of
`compare-testpart.sh`): both must end in the same state, the broken boot
sector included; the result per table stands in the run's RUNINFO.

### Structures behind a pointer (added 2026-10-29, TOOL16)

`Iorec` returns a pointer to an IOREC, `Getmpb` fills an MPB whose first
field points at an MD: the structure is the result, and its layout is not
the same on the two machines -- an m68k pointer is four bytes, an LP64 one
eight, so every field after it moves. `r:<length>` copies raw bytes and
cannot say that. A call table therefore describes the structure once and
captures it field by field. `tosdump/core/cases.h` ("Layouts") is the
authority; `tosdump/cases/xbios-ptr-r1.tsv` the first table.

**The layout comes from `docs/api/`, not from the dumper.** A structure
spec states it in a fenced block ```` ```tosdump-layout ```` beside its prose
tables (`docs/api/structures/iorec.md`, `docs/api/structures/md.md`). The
case table carries a copy of each line -- the table is the one file carried
to both machines, and its CRC is the dump's identity -- and the host test
`layoutcheck` (`tosdump/host/layoutcheck.c`, run by `make -C tosdump
host-test`) refuses a table whose `!layout` line is not, byte for byte, a
spec line, and a spec block that does not parse. So the single source is
the spec; the table's copy is checked against it on every test run.

```
!layout <TAB> <name> <TAB> <size m68k>/<size LP64> <TAB> <field>,<field>...
field:  <fname>:<type>@<offset m68k>/<offset LP64>[:<class>]
```

| type | C type | bytes m68k / LP64 | slot value |
|---|---|---|---|
| `b` | `UBYTE` | 1 / 1 | zero-extended |
| `w` | `WORD` (`short`) | 2 / 2 | sign-extended |
| `L` | `LONG` (`int32`) | 4 / 4 | sign-extended |
| `l` | C `long` | 4 / 8 | sign-extended |
| `p` | a pointer | 4 / 8 | sign-extended (the `ret_hi` rule, section 3) |
| `p><layout>` | a pointer the runner **follows** into `<layout>` | 4 / 8 | as `p`; `<layout>`'s fields follow it |
| `[<layout>]` | `<layout>` embedded at this offset | its sizes | its fields, under `<fname>.` |
| `b<n>`, `w<n>`, `L<n>` | an array of `n` (1..32) of that type (TOOL20) | `n` × the type's | one slot per element, `<fname>[0]` .. `<fname>[n-1]` |
| `x<n>` | `n` (1..1024) opaque bytes, the system's own part of a structure (TOOL20) | `n` / `n` | none: no slot, never compared, no class |

A size `<m68k>/<LP64>:packed` (TOOL20) declares a structure C lays out with
`__attribute__((packed))` on both ABIs, because the TOS ABI fixes its bytes:
every field is aligned on 1, every field has one offset on both ABIs (the
loader refuses two), the size is the end of the last field. The DTA is one
(`docs/api/structures/dta.md`): its LONG `d_length` lies at 26 on both.

`class` is how `tosdiff` judges the field: `EXACT`, `IGNORE`, `NONZERO`,
`HANDLE`, `PTR_RAM` -- the retclass rules (above) on the field's 64-bit
value; default `PTR_RAM` for `p`, `EXACT` for the rest; none on `[..]`,
whose fields carry their own. The comparison rule thus stands in the table
(and in the spec it is copied from), field by field.

**The loader checks the numbers against C** (`td_layout_parse`,
`tosdump/core/cases.c`): every offset must be the one C gives the fields in
this order on that ABI -- on the m68k everything wider than a byte is WORD
aligned (`bios/iorec.h:20-27`, the 14-byte IOREC), on LP64 every type is
aligned on its own size -- with no gap, and every size must be the end of
the last field rounded to the structure's alignment. A layout may name
only layouts defined before it, so following a pointer cannot loop; the
depth is bounded anyway (`TD_LAYOUT_MAX_HOPS` = 2 pointers from the base).
At most eight layouts per table, 32 slots per capture. `!layout` stands
before the first case or prelude.

**Capture.** `r:<layout>` -- through the returned pointer, retclass
`PTR_RAM` only; `<n>:<layout>` -- the `b:` buffer argument `n` (`Getmpb`
writes its MPB there). The record's 256 capture bytes then hold one
**slot** of 8 bytes per field, in layout order (an embedded layout's fields
in place, a followed pointer's structure right after the pointer): the
field's value read at its width on the machine, in the machine's order,
extended to 64 bits as in the table above, stored big-endian. A field the
two ABIs lay out differently lands in the same slot with the same value on
both. Slots past the layout keep the poison and are compared byte by byte.

**The follow rule** (`td_layout_follow_refusal`, `tosdump/core/record.c`):
the runner reads a structure -- behind the returned pointer or behind a
`p>` field -- only when the pointer passes `PTR_RAM` (non-null,
`< 0x80000000`, `>= 0x400`, even, in that order) **and** the structure's
end (pointer + its size on this machine) lies at or below `0x80000000`.
Otherwise it reads nothing and every slot behind that pointer holds
`TD_LAYOUT_NOREAD(reason)` = `0xB0F3B0F3_000000NN` (`tosdump/core/tosdump.h`;
1 null, 2 not `< 0x80000000`, 3 below `0x400`, 4 odd, 5 the structure runs
past `0x80000000`), a value above 4 GiB that no TOS field holds.
`tosdiff` prints such a slot as `not read (<reason>)`; for a per-side class
it is a violation of its own (`<layout>:<path> NONZERO <dump> = not read,
the pointer is below 0x400`), for `EXACT` a value like any other.

A pointer the rule admits can still point at memory the program may not
read: EmuTOS' `themd` lies at `$48E`, and a user-mode read there is a bus
error on the oracle (`measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`,
probe G). The table reads such a structure inside a `Super` bracket.

**Poison of a buffer read through a layout.** The buffer is laid with the
byte poison, then every field of the layout with the poison **of its type**
at the machine's width: `0xAD`, `0xDEAD`, `0xDEADDEAD`, and for `l`/`p` the
LONG `0xDEADDEAD` sign-extended to the machine's width (the unused-register
rule above). A field the call left alone -- `Getmpb`'s `mp_rover`
(`bios/biosmem.c:149-150`) -- then reads `0xFFFFFFFFDEADDEAD` on both
machines and is `EXACT`.

**Writing through a returned pointer.** Only in a table that says

```
!write <TAB> return
```

(before the first case, once, no other word), one prelude form:

```
+put <TAB> <field>=<field> [<TAB> comment]
```

It stands **immediately after** the case whose `r:<layout>` capture names
the structure, and is the only prelude of the case after it: the runner
issues it after that case's poison and before its call, and copies the
value of one field onto another of the same type -- the keyboard flush
`ibufhd=ibuftl` (XBIOS01). Both fields are top-level, non-pointer fields of
the layout (an embedded one, `in.ibufhd`, counts as top-level; one behind a
followed pointer does not). It writes only when the returned pointer
passes the follow rule; a WORD is stored as one WORD (an IOREC index is
read by an interrupt on a real TOS, `bios/ikbd.c:157-173`). What it did --
written, or not written and why -- is kept per prelude
(`td_run_call_put_result`) and logged by DUMPBIOS **after** the run
(`+put line 29: written`), so no console output falls between two
measurements. The record does not carry it; a read-back case after it
measures the effect. Refused: `+put` without `!write return`
(`TD_CASE_ERR_UNSAFE_STATE`), not right after an `r:<layout>` case, a
second prelude after it, a pointer field, a field behind a pointer, a
field onto itself, two types (`TD_CASE_ERR_UNSAFE_FIELD`), and `+put` with
no case after it (`TD_CASE_ERR_DANGLING`).

**`Getmpb` (13:0)** is admitted with this round (`call_policy`,
`tosdump/core/cases.c`; ledger `tosdump/cases/COVERAGE-BIOS.md`): a `b:`
buffer argument (it writes `mp_mfl` and `mp_mal` through it), **inside a
`Super(0L)` ... `Super(t:0)` bracket, and last** -- after it only another
`Getmpb` and the `Super(t:0)` that closes the bracket. On EmuTOS it
rewrites `themd`, the descriptor GEMDOS built its pool on
(`bios/biosmem.c:143-150`); the audit's probe ran it the same way, last and
the MD read in supervisor state, and the machine went on to write its log
on both profiles. Outside a bracket it is refused as before
(`TD_CASE_ERR_UNSAFE_OPCODE`); after it, any other call
(`TD_CASE_ERR_UNSAFE_STATE`).

The runners need nothing new: `xbios-ptr-r1.tsv` writes no disk and runs
on the oracle's `tt` and `falcontc` profiles and through `cargo xtask
tosdump` like a round-1 table. References `EMUTOS-1a7484ab-XBIOS-ptr` (TT)
and `-XBIOS-ptr-falcontc`, `measurements/measurement-2026-10-29-tool16/RUNINFO.TXT`.

### GEMDOS after a media change (added 2026-10-29, TOOL20)

The probes `tosdump/probes/gcprobe.c` (FS20) and `pxprobe.c` (FS22) measure
GEMDOS calls that meet a media change: a logical write of sector 0 arms
forcechange, the next GEMDOS access meets it, osif logs the drive in again
and restarts the call (`bdos/bdosmain.c:449-487`; `docs/api/gemdos/_state.md`
section 4a). Until `DUMPGDOS` exists, their cases -- and the further FS22
paths a user program reaches -- are `DUMPBIOS` tables:
`tosdump/cases/gemdos-mc-fs20.tsv`, `-fs22.tsv`, `-paths.tsv` (header area 1;
ledger `tosdump/cases/COVERAGE-BIOS.md`, "GEMDOS after a media change"). They
are destructive disk tables (`!disk 2 destructive`), run like the closing
tables on a fresh clone of their test image, and their end states are
compared the same way, with the root directory's time stamps zeroed (each
side stamps a file it creates with its own clock). What came with them, in
a destructive disk table only:

**The GEMDOS calls.** `Dsetdrv` (0x0E), `Dgetdrv` (0x19), `Fsetdta` (0x1A),
`Fgetdta` (0x2F), `Dfree` (0x36), `Dsetpath` (0x3B), `Fcreate` (0x3C),
`Fclose` (0x3E), `Fwrite` (0x40), `Fdelete` (0x41), `Dgetpath` (0x47),
`Pexec` (0x4B), `Fsfirst` (0x4E), `Fsnext` (0x4F), with the binding's widths
(`bdos/bdosmain.c:179-277`; `call_policy` and `gdos_refusal`,
`tosdump/core/cases.c`). Outside a destructive disk table each stays refused
as before; inside one, every rule is decided from the table:

| Rule | What it keeps |
|---|---|
| a path (`s:`, `x:`) names the test drive, `"<drive>:\..."` | nothing outside the test image is created, deleted, searched or executed; `Dsetpath` never moves the dumper's own directory |
| `Fcreate(path, 0)` opens a bracket, `Fclose(t:1)` closes it; `Fwrite(t:1, l:0..1024, b:<n>)` inside it; one file, no `Fdelete` while it is open; a table ending with it open is refused | the handle is the one the table opened |
| `Dsetdrv(w:<test drive>)` opens a bracket, `Dsetdrv(t:2)` closes it -- t:2 is what a `Dgetdrv` case before it returned; a table ending with it open is refused | the dumper writes its dump to its own drive again |
| `Dfree`, `Dgetpath`: a `b:` buffer and the test drive as `<drive>+1` | they write into a dumper buffer |
| `Pexec`: mode 0, `x:` the program, a `c:` tail, env `l:0` | the child is the probe build on the test image |
| `Fsetdta(b:<n>)` makes buffer `n` the DTA; no other line names it; `Fsfirst`/`Fsnext` need it set | the search state stays where the call put it |
| none inside a `Super` bracket, none as a prelude | the calls run in user state, as a program makes them |

**Three argument kinds** (`parse_call_args`, `tosdump/core/cases.c`), kept in
the table's string pool and recorded as `0xB0F60000 + n`, `n` their index in
the pool -- never the address:

- `s:<text>` -- a NUL-terminated string (a path);
- `c:<text>` -- a command tail, stored as TOS passes one: its length in the
  first byte, then the text (`Pexec`);
- `x:<path>` -- a program path without its extension. The program format is
  the machine's -- the oracle's `PRG`, NewTOS's `NAP` -- so the **run's
  profile** names the extension, as it names the disk unit: DUMPBIOS reads
  `PROGEXT.TXT` from its current directory only for a table that names `x:`
  (1..3 characters `A-Z`, `0-9`, an optional line end), logs it, and appends
  `.<ext>`; without it the table is not run and no BIN is written
  (`TD_RUN_CALL_ERR_PROGEXT`). `tools/hatari-oracle.sh` profile `disk` puts
  `PRG` into `D:\PROGEXT.TXT`, `cargo xtask tosdump --disk`/`--testdisk`
  puts `NAP` there (`NEWTOS_PROG_EXT`).

The text is printable ASCII without blanks and commas, 1..63 characters (a
tail 1..124). **Tokens:** `t:<n>` is the value an earlier case returned --
`t:0` the `Super` token (a LONG, MEM12), `t:1` the handle of the open file's
`Fcreate`, `t:2` the drive a `Dgetdrv` outside the `Dsetdrv` bracket returned
(both WORDs, as the binding's argument they stand for); recorded as
`0xB0F50000 + n`.

**The DTA.** `d:<layout>` in the capture column reads the DTA buffer field by
field (`docs/api/structures/dta.md`, layout `dta`): the public fields
`d_attrib`, `d_time`, `d_date`, `d_length` and the 14 bytes of `d_fname`, in
the machine's own byte order; the 21 reserved bytes are `x21` and not
captured. The runner poisons the DTA buffer only before `Fsetdta` and
`Fsfirst` -- `Fsnext` continues the search from the state `Fsfirst` left
there, and a case between them (a `Mediach`, a `Pexec`) must not wipe it; so
a capture after `Pexec` or behind `Fgetdta`'s pointer (`r:dta`) shows what the
last search left.

**A logical `Rwabs` returns `E_CHNG` while a change is pending**
(`bios/blkdev.c:450-451`), and a read prelude that does not return 0 stops the
run (above). A table part whose call may leave the change pending therefore
ends with a call that meets it (`Dfree`) before the next part's prelude.

What stays probe-only: the DTA's reserved bytes across a `Pexec` (each
system's own), the identity `Fgetdta() == &dta` (an address), the input trace
of the probes' BIOS calls (`tools/biostrace-check.py` does not read these
tables), and the accessory loader and the AUTO folder, which run at boot and
which no user program reaches. References
`EMUTOS-1a7484ab-BIOS-mc-<fs20|fs22|paths>-disk`,
`measurements/measurement-2026-10-29-tool20/RUNINFO.TXT`.

### The cookie jar (added 2026-10-29, TOOL18)

The cookie jar is no call: a program finds it through `_p_cookies` (`$5A0`)
and walks it. What it holds -- which tags, which values, how many, in which
order -- is still observable, and the machine-identity decision (ADR-012 and
its amendments, `docs/MEMORY.md`; `docs/COMPATIBILITY.md`) needs it measured
on the pin. A call table therefore takes a second kind of case line, the
**jar case**, in the call column instead of `<trap>:<function>`:

| Line | Records | `retclass` |
|---|---|---|
| `jar:<TAG>` | the value of the first entry with that tag, as a `Getcookie` finds it; or *absent* | `EXACT`, `NONZERO`, `PTR_RAM`, `ABSENT` |
| `jar:*` | the number of entries before the terminator, and their tags in jar order | `EXACT` |

`<TAG>` is exactly four characters `0x21`..`0x7E`, the tag in memory order
(`jar:_MCH` is `0x5F4D4348`). `args` and `capture` are `-`; no width suffix.
The loader (`td_call_cases_parse`, `tosdump/core/cases.h` "Jar cases")
refuses a jar case after a prelude, inside a `Super` bracket and after
`Getmpb`, and `ABSENT` on a trap case.

**No trap is issued**, so a jar case needs no whitelist entry. The runner
reads the jar afresh for every jar case through the backend's `td_jar_read`
(`tosdump/core/scrninfo.h`, the SCRNINFO probe's one copy): on the m68k in
supervisor mode via `Supexec` (`tosdump/abi/m68k_scrninfo.c`), on NewTOS as
a plain load of the per-process page 0 (`tosdump/abi/newtos_scrninfo.c`) --
the address and the walk any program makes. No buffer is touched. Both
`DUMPBIOS` builds link that backend.

**The record** is the call record (section 3), its fields used as
`tosdump/core/tosdump.h` "A JAR CASE" states:

| Field | `jar:<TAG>` | `jar:*` |
|---|---|---|
| `trap` | 256 (`TD_CALL_TRAP_JAR`), outside the 0..255 a trap number has | 256 |
| `opcode` | 0 | 1 |
| `args[0]` | the tag as a LONG; the other slots `0xDEADDEAD` | all `0xDEADDEAD` |
| `ret_hi:ret` | the value **zero-extended** (a cookie value is a 32-bit cell on both machines), or `0xFFFFFFFF_FFFFFFFF` (*absent*) | the count, or *absent* when `_p_cookies` is 0 |
| `capture` | the poison | the tags as big-endian LONGs, at most 64; the rest the poison |

A present value never has a high half, so *absent* cannot be a value -- not
even a cookie `0xFFFFFFFF` (`_MCH` on NewTOS, `MCH_NOHARD`), which records as
`0x00000000_FFFFFFFF`. `tosdiff` tells the two apart by the high half alone.

**How `tosdiff` judges it** (`jar_check`, `tosdump/host/tosdiff.c`):

| Class | The check | A failure prints |
|---|---|---|
| `EXACT` (`jar:<TAG>`) | present on both sides with the same value, or absent on both | `cookie EXACT <a> = 0x00020000   <b> = absent` |
| `NONZERO`, `PTR_RAM` | on each side: present, and the class's property holds on the value (section 6, "The call table") | `cookie PTR_RAM <dump> = not in the jar` / `= 0x00000401, is odd` |
| `ABSENT` | on each side: absent | `cookie ABSENT <dump> = 0x00000100, present` |
| `EXACT` (`jar:*`) | the same count and the same tags in the same order; `jar:*`'s capture is not compared byte-wise | both lists, `12: _CPU _VDO …` |

The case header names the cookie (`case 47  jar _MCH  …`). The self-check
(above) applies `NONZERO`, `PTR_RAM` and `ABSENT` to the oracle alone, so a
class that is not `EXACT` must hold on every profile the table is measured
on. Not measured: the jar's capacity (the terminator's value).

The table is `tosdump/cases/jar-r1.tsv`: `jar:*` and every tag EmuTOS names
(`include/cookie.h:26-44`; the ones `fill_cookie_jar` adds in its order,
`bios/machine.c:665-798`), then NewTOS's `NTOS` and `EdDI`
(`tos-api/src/cookie.rs:223@seed_defaults`). References
`EMUTOS-1a7484ab-BIOS-jar` (TT), `-BIOS-jar-falcontc` and `-BIOS-jar-disk`
(profile `disk`; it touches no disk), `measurements/measurement-2026-10-29-tool18/RUNINFO.TXT`.
A difference against NewTOS is entered in `docs/COMPATIBILITY.md`.

### Input check on NewTOS: the BIOS/XBIOS argument tracer (added 2026-10-28, TOOL08)

A result comparison counts only if the arguments that **arrived** match the
table on both sides (`.claude/agents/auditor.md`, "Inputs before outputs").
The record's `args` field is what the dumper says it passed, not what the
dispatcher received. On NewTOS the received side is the kernel's cargo
feature `bios-arg-trace` (`kernel/src/dbg_biostrace.rs`, off by default and
compiled out without it): one serial-console line per **completed** trap #13
or #14 call,

```
[biostrace] 000042 U0  13:0x04 Rwabs       w:0x0000 p:0x0000000001a5cd20 w:0x0001 w:0xffff w:0x0002 l:0x0001ffa0 -> 0x0000000000000000
```

— running number, caller (`U<slot>` an EL0 process, `K` an EL1 `svc`),
`trap:function`, the slot's name (`(empty)` an `xbios_unimpl` slot, `(beyond)`
past the table), the arguments `x0`, `x1`, … **at the width of the EmuTOS
call form** and the 64-bit `x0` result. The call form of every slot is the
prototype of EmuTOS' debug wrapper in the same table slot
(`bios/bios.c:895-1235`, `bios/xbios.c:49-1236`): `w:0x%04x` a WORD/UWORD
(low 16 bits), `l:0x%08x` a LONG/ULONG (low 32 bits), `p:0x%016x` a pointer
(the whole register). A `w:`/`l:` value carries `=0x<register>` when the
register is not the sign extension of the printed value; registers beyond
the call form are not printed. An EL0 `Supexec` never reaches the
dispatcher (it becomes an upcall) and is recorded before the upcall with
`-> upcall` instead of a result.

The console, not a file: the tracer writes no drive, so the disk round's
test partition stays byte for byte comparable, and it needs no flush point.
A QEMU `raspi4b` run captures it with `-append serial=on -serial file:…`.

**The procedure.** Build the kernel with
`cargo xtask build --board rpi4 --release --features bios-arg-trace`, run the
table as usual (round 1: a `tools/mkcard.sh` card; the disk round: the
`MKCARD_TESTDISK` card), then

```
tools/biostrace-check.py <BOOT-SERIAL.txt> <table.tsv> [--bin <BIOS.BIN>] [--caller U0]
```

The checker orders the table's calls as the runner issues them (per case its
preludes, then the case), finds them as one contiguous run in the trace of
one caller, lists that caller's other BIOS/XBIOS calls, and compares every
argument: `w:`/`l:` by value at their width and with the register
sign-extended (the backend's marshalling, section 6 "The call table"); `l:`
in a pointer slot against the sign-extended LONG; `b:<n>`/`f:<n>` as a
non-null address that is the same for the same `n`; a call-form slot the
table leaves out must hold the poison `0xDEADDEAD` sign-extended; a table
width that is not the call form's is `WIDTH`, a table argument the call form
does not have is `BEYOND-FORM`; `t:0` (the `Super` token) is `UNCHECKED`.
`--bin` ties the run to its dump: record *k* has the *k*-th case's line,
trap and function, and its `ret_hi:ret` equals the traced result. Exit 0 =
every argument matched.

**The oracle side.** Hatari's `--trace bios` prints `Rwabs` with five
arguments, without `lrecno`. A debugger breakpoint on the pinned ROM's
`_lrwabs` (`references/emutos-oracle/1a7484ab/emutos.map`, `0x00e01050`) that
dumps `sp..sp+20` on entry shows all six as `lrwabs` receives them:
`measurements/measurement-2026-10-28-tool08/RUNINFO.TXT`, which has the probe
(a copy of `tools/hatari-oracle.sh`, the script itself is unchanged).

---

## 7. Programs to build

One program per area, sharing `tosdump/core/` and one `tosdump/abi/` backend:

| Program | Area | Notes |
|---|---|---|
| `DUMPVDI` | VDI | build this first |
| `DUMPAES` | AES | after VDI is clean |
| `DUMPBIOS` | BIOS + XBIOS | `retclass` and `capture` since 2026-10-27; table `tosdump/cases/bios-r1.tsv`, ledger `tosdump/cases/COVERAGE-BIOS.md`, references `EMUTOS-1a7484ab-BIOS` (TT) and `-BIOS-falcontc`, first comparison `measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`. Second table `tosdump/cases/super-r1.tsv` (`Super`/`Supexec`, section 6), references `EMUTOS-1a7484ab-SUPER` (TT) and `-SUPER-falcontc`. Disk table `tosdump/cases/bios-r2.tsv` (section 6, "The disk round"), reference `EMUTOS-1a7484ab-BIOS-r2-disk` (profile `disk`), first comparison `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`; the closing tables `bios-r2-phys.tsv`, `-x004.tsv`, `-x005.tsv`, `-x006.tsv` (section 6, "The closing tables"), references `EMUTOS-1a7484ab-BIOS-r2-<phys\|x004\|x005\|x006>-disk`, `measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`. Structures behind a pointer (section 6, "Structures behind a pointer"): `tosdump/cases/xbios-ptr-r1.tsv` (`Iorec`, the flush, `Getmpb`), references `EMUTOS-1a7484ab-XBIOS-ptr` (TT) and `-XBIOS-ptr-falcontc`, `measurements/measurement-2026-10-29-tool16/RUNINFO.TXT`. GEMDOS after a media change (section 6, TOOL20): `tosdump/cases/gemdos-mc-fs20.tsv`, `-fs22.tsv`, `-paths.tsv`, references `EMUTOS-1a7484ab-BIOS-mc-<fs20\|fs22\|paths>-disk`, `measurements/measurement-2026-10-29-tool20/RUNINFO.TXT`. The cookie jar (section 6, TOOL18): `tosdump/cases/jar-r1.tsv`, references `EMUTOS-1a7484ab-BIOS-jar` (TT), `-BIOS-jar-falcontc`, `-BIOS-jar-disk`, `measurements/measurement-2026-10-29-tool18/RUNINFO.TXT` |
| `DUMPGDOS` | GEMDOS | needs `retclass` and `capture`. Until it exists, GEMDOS behaviour that needs a measurement is a **probe**: a C program of `tosdump/probes/` with a text log, built for both sides from one source and run by its own script — `gcprobe.c` (media change, TASK/FS20; `make -C tosdump gcprobe-m68k gcprobe-newtos`, `tosdump/probes/run-gcprobe.sh`) and `pxprobe.c` (`Pexec` and `Fsfirst` after a media change, TASK/FS22; `make -C tosdump pxprobe-m68k pxprobe-newtos`, `tosdump/probes/run-pxprobe.sh`), oracle side through `tools/hatari-oracle.sh` profile `disk`. Since TOOL20 their cases are also `DUMPBIOS` tables (`gemdos-mc-*.tsv`, section 6, "GEMDOS after a media change"); the probes stay for what the tables cannot express (listed there) until their cases move to DUMPGDOS |

Each writes exactly one output file, named `<AREA>.BIN`, into the current
directory. Buffer records in RAM and write once at the end. Do not write
per record.

**Build VDI before AES.** The AES calls the VDI internally, so an AES difference
has ambiguous causes and a VDI difference does not. A clean VDI will make a
significant share of the AES differences disappear on its own.

The host comparator `host/tosdiff` takes two dump files plus the case table and
reports differences per case, per field, with the field named — not as a hex
offset.

---

## 8. Rules for these tasks

These exist because the failure mode in this project is drift, not lack of
ability. Every one of them is an acceptance condition, not a suggestion.

1. **Commit on every green state.** One task, one commit. The maximum damage from
   any failure is then one task.
2. **Tests are written first and must be seen failing.** A test that passes the
   moment you write it verifies nothing. Write it, run it, see red, commit it,
   and only then implement.
3. **The implementing task may not modify tests, case tables or the record
   format.** If a test appears wrong, stop and say so; do not adjust it. Changing
   the test to match the code is the single most common form of drift.
4. **Done means a command exits zero**, and the task states which command. Not
   "implement X correctly" but "`host/tosdiff` reports zero differences for
   opcode 6".
5. **The two ABI backends stay symmetric.** Any logic that appears in one and not
   the other is a bug, regardless of how well justified it looks.
6. **State what could not be verified.** If something was not run, say so
   explicitly rather than reporting success.

---

## 9. Task order

Each task ends with the stated command exiting zero, and with a commit.

**T1 — record format and comparator skeleton**
Implement the header and the three record layouts as host-side code, plus
`tosdiff`. Verify with synthetic dump files generated by the tests, not by any
target program. Include the byte-order test from section 2.1.
*Done:* `make -C host test` exits zero, and includes both a test where a single
poisoned field differs and `tosdiff` names that field, and a test asserting the
big-endian and little-endian host configurations emit byte-identical records.

**T2 — core runner and m68k backend, VDI only**
Case table loader, poison, reset, runner, record writer, `tosdump/abi/m68k_trap.c`.
Cross-compile `DUMPVDI.TOS`.
*Done:* builds without warnings, and a host-side test drives the runner against a
stub backend and produces a well-formed dump file.

**T3 — capture the reference**
Run `DUMPVDI.TOS` on the TT, retrieve `VDI.BIN` via CF.
*Done:* `tosdiff --validate VDI.BIN` accepts the file and the record count
matches the case table.

**T4 — AArch64 backend**
`tosdump/abi/newtos_svc.c`, same core, same case table.
*Done:* `DUMPVDI` runs under NewTOS and produces a well-formed `VDI.BIN`.

**T5 — first comparison**
*Done:* `tosdiff cases/vdi.tsv TOS306-VDI.BIN NEWTOS-VDI.BIN` runs and produces a
per-case report. Differences are expected here — the deliverable is the report,
not a zero.

**T6 — AES** — as T2 to T5 for `DUMPAES`.

**T7 — BIOS, XBIOS, GEMDOS** — adds `retclass` and `capture` to the comparator.

Capture the reference (T3) **before** measuring NewTOS (T4). Doing it the other
way round invites choosing cases that make NewTOS look good.

---

## 10. First case list

Do not start with a full opcode sweep. Take the VDI calls that QED actually
issues while starting up and opening a file — that is the path that already runs,
and it is where the current bugs are.

Obtain the list by logging the trap dispatcher under NewTOS, since the dispatcher
is yours there. Nothing needs to be modified on the TT to get this list.

That log is the input to tosdump/cases/vdi.tsv. Expand from there once the first
comparison is clean.

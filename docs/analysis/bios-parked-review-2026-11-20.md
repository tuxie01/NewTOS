# Review of the nine parked BIOS and XBIOS tickets against EmuTOS (BIOS23)

> Dated analysis, 2026-11-20, researcher, ticket `TASK/BIOS23.md`. Reading only: no ticket edited, no spec corrected
> (wrong spec statements are listed, section 4), no code run except `cargo xtask deps ... --down` and
> `cargo xtask doc-check`. Code state is `main` at `458e76ee3` (this branch fast-forwarded to it). "Read" = in the
> file in front of me this run; "inferred" = reasoned from what I read (all inferred statements are collected in
> section 5). Whether NewTOS equals EmuTOS is the auditor's measurement, not a conclusion here. Classes as in the
> ticket: (a) can be made equal without hardware the Pi lacks, (b) bound to hardware the Pi lacks, (c)
> documentation or tooling only.

## 1. Summary

| Ticket | Prio | Difference today | Class | Fix size |
|---|---|---|---|---|
| BIOS13 | P3 | none left of items 1-7 except two stale cites (below) | (c) | docs only, two cites |
| BIOS14 | P3 | item 3 fixed, item 1 mostly fixed; `rsconf`, `bconmap`, `kbdvbase` edges and `couplings.md` still missing | (c) | tool code (owner decides) + one doc |
| XBIOS06 | P2 | `Rsconf` zero-extended, TSR byte `0x01`, no write into `Iorec(0)`, `-2` query from a shadow | (a) (split, 3 parts) | `system.rs` `rsconf`, `iorec.rs`, one COMPATIBILITY entry |
| XBIOS07 | P3 | points 1-5 fixed; three statements in `docs/api/xbios/` still wrong | (c) | three edits |
| XBIOS11 | P3 | first `Random` seed from `frame_clock`, not `_hz_200`; `_hz_200` is live now | (a) | one function, one file |
| XBIOS16 | P3 | `Jdisint`/`Jenabint` answer the function number; oracle value not measured; no spec file | (a) after a measurement | two functions in `mfp.rs` + two arms in `mod.rs` |
| XBIOS19 | P3 | `Bioskeys` returns the current-table address, EmuTOS the address of the unchanging default struct | (a) | `keytbl.rs` + page-0 layout (`loader.rs`) |
| XBIOS20 | P3 | 7 entries without a table case: 2 (a)/(c), 4 (b), 1 split | split | see row |
| XBIOS24 | P3 | none against EmuTOS; a TeraDesk presentation question | (b) + (c) | owner decision pending |

Proposed order of the (a) tickets: **XBIOS11, XBIOS06, XBIOS16, XBIOS19** (section 3).

## 2. Per ticket

### BIOS13 (P3, parked 2026-10-31) - class (c)

The ticket's seven items (`TASK/BIOS13.md` "Sachverhalt") re-checked at `main`:

| Item | State at main | Evidence read |
|---|---|---|
| 1 `bios-r1.tsv` `Setexc` NONZERO rule | recorded as an erratum; the table is frozen by CRC and `bios-core-r1.tsv` carries `PTR_RAM` | `tosdump/cases/ERRATA.md` (section `bios-r1.tsv`, item 1) |
| 2 `Getmpb` TSR statement in COMPATIBILITY | corrected: "not visible through `Getmpb`: ... `mp_mal = NULL` and a single free MD" | `docs/COMPATIBILITY.md:1016-1017` |
| 3 MEM12 table "any cell < 0x800" | corrected, with the BIOS07 note | `TASK/MEM12.md:139` |
| 4 COVERAGE `Bconin` row | corrected: handle 0 returns at once with 0 (`bios/parport.c:121-125`) | `tosdump/cases/COVERAGE-BIOS.md` (row `0x02 | Bconin`) |
| 5 ledger entries (3 tables, CRCs) | present with `C056A3E0`, `5C3A5F7A`, `3944A677` | `tosdump/cases/COVERAGE-BIOS.md` (ledger rows of the three tables) |
| 6 shifted cites of `bios-r1.tsv` comments | recorded as erratum table | `tosdump/cases/ERRATA.md` (cite-shift table) |
| 7 guard / capture form | became TOOL16, state DONE | `TASK/TOOL16.md:3` |
| Addendum: `vectors.rs` comments, `docs/api/bios/_state.md:292` | the `ETV_DEFAULT_*` constants are gone from `vectors.rs` (grep: no hit); `docs/api/bios/_state.md:297` cites `vectors.rs:90@VECTORS`, `:39@ETV_TIMER` | `tos-api/src/bios/vectors.rs:43@ETV_TIMER`, `docs/api/bios/_state.md:297` |

**What differs from EmuTOS today:** nothing - the ticket records no behavioural difference, only document state. A
difference that no longer exists: all seven items (the ticket's own "Stand 2026-10-28" says the same for 1-4).

**What is left to write (read):** `tosdump/cases/ERRATA.md` (section `bios-r1.tsv`, item 1) still cites the `ETV_DEFAULT_JUST_RTS` and
`ETV_DEFAULT_CRITIC` constants of `tos-api/src/bios/vectors.rs`; those symbols no longer exist (BIOS07 removed them, ticket "Erledigt mit BIOS07"). The
erratum is a statement about the state when it was written; whether to date-mark it or leave it is for the lead
(inferred: doc-check would flag the cite on touching). That is the entire residue.

### BIOS14 (P3, parked 2026-10-31) - class (c), tool code part owner-decided

Re-checked by running `cargo xtask deps <fn> --down` (output read this run) and reading `docs/analysis/deps/mapping.tsv`:

| Ticket point | State at main |
|---|---|
| 1 no edges BIOS entry -> device handlers | **gone for the `bcostat` family**: `deps bcostat --down` lists `bcostat0..4`, `char_dummy` "via bcostat_vec[]" (`bios/bios.c:1161`); added by `e70700d69` "add vector-call edges (TOOL12)", 2026-10-28 |
| 2 `rsconf` -> `rsconf1/B/TT/dummy` | **partly**: `deps rsconf --down` lists only `rsconf1` (via `rsconfptr`, `bios/xbios.c:378`); `rsconfA/B/TT/rsconf_dummy` are absent |
| 2 `bconmap` -> MAPTAB targets, `kbdvbase` -> hook callers | **still missing**: `deps bconmap --down` and `deps kbdvbase --down` give 0 functions besides themselves |
| 3 `bconout3` mapped to `midiws` only | **gone**: `mapping.tsv:722-723` has both `xbios/system.rs@midiws` and `bios/chardev.rs@bconout` |
| 4 `couplings.md` absent | **still absent**: `docs/analysis/deps/` holds `edges.tsv`, `layers.md`, `mapping.tsv`; `.claude/agents/auditor.md:160,232` still point at it |

**Differs from EmuTOS:** none (tooling). **Class (c).** Left: (1) graph tool edges for the table-assigned vectors
(`rsconfptr` targets set in `init_bconmap`, MAPTAB entries, `kbdvbase` hook callers) - code under `tools/xtask`, a
developer ticket on owner decision as the ticket says; (2) write `couplings.md` in `docs/analysis/deps/` or remove the two
references in `auditor.md` (the lead's brief file, not mine). Acceptance cases of the ticket: `deps bcostat --down`
already lists the named targets except "Bconmap-`maptab`"; `deps rsconf --down` does not list four `rsconf*` targets.

### XBIOS06 (P2, parked 2026-10-30) - class (a), split into three parts

**Differences today (re-checked, all still present):**

1. **Return not sign-extended.** `rsconf` ends `old as i64` on a `u32` (`tos-api/src/xbios/system.rs:254-286@rsconf`, last
   statement "zero-extended into x0"); the contract is the sign-extended LONG (`docs/COMPATIBILITY.md:2177-2186`,
   `tos-api/src/trap.rs:6@TrapArgs`). EmuTOS returns `ULONG old` (`references/emutos/bios/serport.c:217-249`), seen by an
   m68k program as D0. Measured by the ticket: oracle `0xFFFFFFFF_88018100`, NewTOS `0x00000000_88010100`.
2. **TSR byte.** Shadow `RS_TSR = 0x01` (`tos-api/src/xbios/system.rs:239@RS_TSR`); oracle TT `0x81` (MFP "buffer empty", ticket measurement).
   Falcon differs again (RSR and TSR bytes, SCC branch `serport.c:775-790`, read in `docs/api/xbios/0f-rsconf.md:195-203`).
3. **`Iorec(0)` record not written / `-2` query answered from the shadow.** `rsconf` never touches the record
   (`tos-api/src/xbios/system.rs:254-286@rsconf`); EmuTOS writes `baudrate` (`serport.c:232`) and `flowctrl` (`:238`) and answers `-2` from
   `iorec->baudrate` (`:222-223`). The record exists in NewTOS with the EmuTOS field offsets
   (`tos-api/src/xbios/iorec.rs:96-107@EXT_IOREC_SIZE`, `tos-api/src/xbios/iorec.rs:580@record_va`).

**Class (a).** Nothing needs a register the Pi lacks: the oracle's answer is a software image of three bytes plus two
record fields. The owner decided (2026-10-30, ticket "Entscheidung") option (a) TT/MFP profile, sign-extension, write only
`baudrate`/`flowctrl`, Falcon difference ACCEPTED with a COMPATIBILITY entry. Part 2 (TSR `0x81`, readback
`written | 0x80`) imitates a 68901 flag in software; if the owner counts that as hardware-bound, only that line moves
to (b) with ADR-031's "no MFP" reason (`docs/MEMORY.md:3173`).

**Size.** `rsconf` in `system.rs` (sign-extend; TSR shadow default and readback rule; `-2` from the record; write
two record bytes), a write helper in `iorec.rs` (kernel-alias write of two bytes into the running process's record,
as the ring writers do), the comments in `tos-api/src/xbios/system.rs:225-253@RS_BAUD`, `docs/api/xbios/0f-rsconf.md` notes (`:326-336`, `:408-420`),
one COMPATIBILITY entry, and one change of the sentence at `docs/COMPATIBILITY.md:2184-2185`. About 2 code files,
3 doc files.

**Spec state:** ready. `0f-rsconf.md` was corrected 2026-10-30 (contract sign-extended, implementation
zero-extended, marked apart, `:108-133`), the decision is recorded; the open point in the file (`:331-335`, "register-byte
profile ... open owner decision") is answered by the ticket's "Entscheidung" and needs only the notes updated at fix time.

**Acceptance and oracle.** The permanent probe `tosdump/probes/rsprobe.c` (+ `build-rsprobe-newtos.sh`, `run-rsprobe.sh`)
exists and ran on tt, falcontc and NewTOS (`measurements/measurement-2026-10-30-xbios06-rsprobe/RUNINFO.TXT`, quoted by
the ticket "Messung Schritt 1"). The pinned oracle can show it on **tt** (Hatari's MFP emulation: TSR readback
`written | 0x80`, `-2` from the record, record writes of `baudrate`/`flowctrl` only). It shows a **difference** on falcontc
by the owner's decision (so the falcontc column stays DIFF, accepted). Table cases of `tosdump/cases/bios-r1.tsv` and
`tosdump/cases/xbios-chardev-r1.tsv` (the `Rsconf(-1 x6)` rows, EXACT, `Rsconf(-1 x6)`) become EQUAL on tt, not on falcontc (ticket "Abnahme"). The table
dumper still refuses set arms (`tosdump/core/cases.c:4050-4055@CR_RSCONF`, TOOL27), so set-arm cases stay probe-only.

### XBIOS07 (P3, parked 2026-10-31) - class (c)

Re-check of the five points and the addenda:

| Point | State at main |
|---|---|
| 1 `0e-iorec.md`, `18-bioskeys.md` "returns EINVFN" | corrected: `0e-iorec.md:28-30`, `18-bioskeys.md:244` say function number |
| 2 `mod.rs` "need NewTOS keyboard infrastructure" | gone: `tos-api/src/xbios/mod.rs:26-31@PHYSBASE` now names Iorec (XBIOS01) and Bioskeys (XBIOS02) as wired |
| 3 COMPATIBILITY "IKBD table in page 0 (`0x9A0`)" | corrected: private page `0x7F000`, `Kbdvbase()` `0x7F010` (`docs/COMPATIBILITY.md:860-872`; code `tos-api/src/bdos/process/loader.rs:1048@KBDVECS_AREA_VA` `= mmu::PRIV_PAGE_VA`, `tos-api/src/bdos/process/mmu.rs:259@PRIV_PAGE_VA` `0x7F000`) |
| 4 `2c-bconmap.md` "single-serial build" | corrected: `BCONMAP_AVAILABLE` true under the pin (`2c-bconmap.md:41-48`) |
| 5 `0f-rsconf.md` "zero-extended" | restated as contract sign-extended / implementation zero-extended (`0f-rsconf.md:108-133`); closes with XBIOS06 |
| addenda `iorec.md` 14 bytes, WORD vs 4-byte IKBD entries, `EXT_IOREC` 33/34 | corrected (`docs/api/structures/iorec.md:24`, `docs/api/xbios/_state.md:223-239`) |

**Differs from EmuTOS:** nothing behavioural. **Left to write - three statements still wrong or stale (read):**

- `docs/api/xbios/22-kbdvbase.md:3,338-346,533-535,708` and `docs/api/xbios/_state.md:61-63` give the NewTOS table address
  as `0x9A0` in TPA page 0; the code puts it at `PRIV_PAGE_VA` `0x7F000` (`tos-api/src/bdos/process/loader.rs:1048@KBDVECS_AREA_VA`). The ticket's own "Offen" list names
  `22-kbdvbase.md`; `docs/api/xbios/_state.md:61` is a second place.
- `docs/api/xbios/_state.md:208-209` calls `head` the "write index (ISR advances)" and `tail` the "read index". EmuTOS
  advances `tail` when it pushes and compares to `head` (`references/emutos/bios/ikbd.c:159-172`), so `tail` is the write
  index; the ticket's "Offen" names this.
- The ticket's open question whether `init_bconmap` alone is hidden by `BCONMAP_AVAILABLE` is answered in
  `2c-bconmap.md:44-48` (read: defined and called under the same gate, `serport.c:944`, `:1056-1059` as cited there; not
  re-read by me).

### XBIOS11 (P3, parked 2026-10-31) - class (a)

**Differs today (re-checked):** `random()` seeds on the first call from `host.frame_clock()` as
`(fc << 16) + fc` (`tos-api/src/xbios/system.rs:15-27@random`); EmuTOS seeds from `hz_200` the same way
(`references/emutos/bios/xbios.c:421-430`). The LCG is identical (read, both). The reason in the spec ("no 200 Hz counter
on the Pi", `docs/api/xbios/11-random.md` Pi binding) **no longer holds**: `_hz_200` is a live 200 Hz counter, kernel-readable as
`bios::tick::hz_200()` (`tos-api/src/bios/tick.rs:92@hz_200`) and mirrored into the running process's `$4BA`
(`docs/api/bios/_state.md:296`, `docs/api/bios/_sysvars-contract.md:142`). A second small difference: NewTOS bumps a zero
seed to 1 (`tos-api/src/xbios/system.rs:20-22@random`); EmuTOS lets `rseed` stay 0 and the first step gives 1 (`references/emutos/bios/xbios.c:427-428`). Both are reachable
only with `hz_200 == 0` at the first call (inferred; `(h<<16)+h = h*65537 mod 2^32` is 0 only for `h = 0`).

**Class (a).** Size: one function (`random`) in one file (`system.rs`), drop the `s = 1` bump; plus `11-random.md` Pi binding
(frame clock -> `_hz_200`) and the doc-comment. No host-trait change (`tick::hz_200()` is already `tos-api`).

**Spec state:** ready - contract and reference are separated correctly in `11-random.md:7-9`; only the Pi-binding section
(`:139-165`) is stale and must change with the fix.

**Acceptance and oracle.** The probe `xsseed.c` (first `Random()` after start, `_hz_200` before and after; EmuTOS `yes` on
tt, tt-repeat, falcontc) exists only in the immutable run `measurements/measurement-2026-10-28-xbios-system-audit/method/xsseed.c`
(ticket cites values `91cc67` from `frame_clock` 908 at `hz_200` 3024). Under CLAUDE.md "Testing" a probe-only finding
needs a permanent copy under `tosdump/probes/` before it serves as acceptance (read from CLAUDE.md; that move is part of the
fix or a TOOL follow-up). The oracle shows it: yes, deterministic relation (the value is computable from the two `_hz_200`
reads, not a fixed number). A table case cannot (the first call must come before any other `Random`).

### XBIOS16 (P3, parked 2026-10-31) - class (a) after a measurement

**Differs today (re-checked):** `Jdisint` (0x1A) and `Jenabint` (0x1B) have no arm and answer the function number via the catch-all
(`tos-api/src/xbios/mod.rs:154@Jdisint`, comment names them); EmuTOS vectors them to `jdisint`/`jenabint` (`references/emutos/bios/xbios.c:1317-1318`,
`#if CONF_WITH_MFP` `:583`, `:611`), which write the MFP mask/enable registers (`bios/mfp.c:39-72`, `:122-130`) and are
`void`, so `d0` is what the compiled code leaves. `Jenabint`: `docs/api/xbios/0d-mfpint.md:50-52` says `mfpint` tail-jumps
to `jenabint`, whose last instructions leave the mask register byte in `d0`; therefore a direct `Jenabint(n)` very likely
answers the same byte (inferred, I could not disassemble `obj/mfp.o`: no m68k objdump on this host, the file is a.out).
`Jdisint`: **not established** (the code is `mask = ~(1<<num)` and stores to `imra`/`iera`/`ipra`/`isra`, `references/emutos/bios/mfp.c:44-56`, but
what the compiler leaves in `d0` needs the disassembly or a measurement).

**Class (a):** the NewTOS side already keeps the IMRA/IMRB image (`tos-api/src/xbios/mfp.rs:44@IMR_AT_RUN`, `= 0x1460`, `enable` `:69-78`); the fix adds a `disable`
and two arms. Size: 2 functions in `tos-api/src/xbios/mfp.rs`, 2 arms in `mod.rs`, host tests, ADR-031 already covers the
"not served, warning" side (`docs/MEMORY.md:3173`). Hardware-bound part: none (the handler is never called, ADR-031).

**Spec state: not ready.** There is no `1a`/`1b` spec file (`ls docs/api/xbios` shows none); `0d-mfpint.md:99-102` lists them
under "Open". A researcher step must first record the oracle `d0` of both (measurement by the tester, or disassembly with
`m68k-atari-mint-objdump` as the BIOS15 spec did) and write the entry.

**Acceptance and oracle:** the permanent probe `tosdump/probes/hkprobe.c` (Part M, `:20-30`) already measures `Mfpint`/`Xbtimer`
returns with the IMRA/IMRB read in a Supexec callback; extending it with `Jdisint(n)`/`Jenabint(n)` (the ticket's work item 1)
shows the oracle value on tt and falcontc. The table dumper refuses both (`tosdump/core/cases.c:3880-3881@Jdisint`), so probe only.

### XBIOS19 (P3, parked 2026-10-31) - class (a)

**Differs today (re-checked):** `bioskeys()` resets the translator and answers the address of the freshly rewritten *current*
struct, the same value `Keytbl(-1,-1,-1)` returns (`tos-api/src/xbios/keytbl.rs:220-243@bioskeys`); EmuTOS leaves
in `d0` the address of the country-default struct returned by `get_keytbl()` (`references/emutos/bios/ikbd.c:136-140`;
default is fixed after boot, `bios/country.c:195`; oracle value `0x00E06E8C` and the `jsr _get_keytbl; moveal` code are cited
from `docs/api/xbios/18-bioskeys.md` "Return value" and the run its XCPROBE.LOG, not re-derived). So on
the oracle `Bioskeys()` differs from `Keytbl(-1,-1,-1)`; on NewTOS it equals it.

**Class (a):** needs a second, read-only copy of the power-up `struct keytbl` plus its three 128-byte tables (and, if
the three pointers behind it must point at the default tables as the ticket demands, those too) in the caller's address
space. The current area is `0x238` bytes at `KEYTBL_AREA_VA = 0x768` with const asserts against `RESERVED_LOW_END`
(`tos-api/src/bdos/process/loader.rs:1032-1038@KEYTBL_AREA_VA`, `tos-api/src/xbios/keytbl.rs:76@KEYTBL_AREA_SIZE`), laid out for one struct and six tables (`tos-api/src/xbios/keytbl.rs:70-76@OFF_NORM`). Size: `keytbl.rs`
(`bioskeys`, `publish`, the default image), `loader.rs` (area size or a second area, the page-0 layout that MEM12 fixed;
this is the risk), the spec "Return value" and "NewTOS implementation" sections, one probe. 2-3 code files.
Whether the contract wants it at all is thin: `18-bioskeys.md` says "a program cannot rely on it (the function is `void`)",
the owner's 2026-10-29 answer was "wenn es in EmuTOS so ist" (ticket header).

**Spec state:** ready for the behaviour (`18-bioskeys.md:62-71` states EmuTOS's value and the NewTOS difference); the
"Return value" section changes with the fix.

**Acceptance and oracle:** the ticket asks for the pointer *relation* (`Bioskeys()` != `Keytbl(-1,-1,-1)`; the three
pointers behind it equal the default tables), not the value `0x00E06E8C`. The oracle shows it: probe lines K07-K11 of
`xcprobe.c` (`measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c:329-341`, read) did it once; that
probe lives only in an immutable run, so a permanent probe under `tosdump/probes/` is needed (CLAUDE.md "Testing"). No
table case covers `Bioskeys` (`grep` of `tosdump/cases/*.tsv`: none).

### XBIOS20 (P3, parked 2026-11-12) - split by entry

The ticket's seven entries. No table case exists for any of them (`grep '^14:0x..' tosdump/cases/*.tsv` for 0x00, 0x06, 0x0C, 0x0D, 0x1D,
0x1E, 0x1F: no hit; `tosdump/cases/COVERAGE-BIOS.md` lists them `excluded`/`later`).

| Entry | Difference from EmuTOS today | Class | Reason / what is left |
|---|---|---|---|
| `Mfpint` (0x0D), `Xbtimer` (0x1F) | **none in the return value**: measured equal on M02-M07 (`docs/api/xbios/0d-mfpint.md:57-73`, `tos-api/src/xbios/mfp.rs:60-103@mfpint`); handler not served (ADR-031) | (c) | oracle values exist in `hkprobe` Part M (permanent probe); the table dumper refuses writes of this kind; what is left is a table case or accepting the probe as the case (tooling, TOOL27-like) |
| `Initmous` (0x00) | EmuTOS writes IKBD command bytes and sets `kbdvecs.mousevec` (`references/emutos/bios/mouse.c:40-121`); NewTOS warns once and answers the function number (`tos-api/src/xbios/mod.rs:167@note_installer`, catch-all) | (b) for the effect: IKBD/mouse hardware, ADR-031 "`mousevec` ... not served" (`docs/MEMORY.md:3170`); the `d0` value after the call is not measured (`retval` is local, `void` at the trap) - **needs a measurement** to say whether it is (a) | no oracle case; `tosdump/cases/COVERAGE-BIOS.md` row `0x00 | Initmous`, `excluded` |
| `Midiws` (0x0C) | EmuTOS sends each byte through `bconout3(3, b)` (`bios/midi.c:104-112`); NewTOS passes it to the BIOS MIDI entry that accepts and drops (`tos-api/src/xbios/system.rs:86-96@midiws`) | (b) MIDI ACIA hardware (ADR-031 "no MIDI/ACIA", `MEMORY.md:3170`; BIOS04 per `mapping.tsv:722-723`) | not a case candidate until MIDI hardware (IO01) |
| `Offgibit` (0x1D), `Ongibit` (0x1E) | EmuTOS read-modify-writes PSG port A (`bios/sound.c:115-141`); the pinned target compiles it in (`CONF_WITH_YM2149` 1, `docs/api/_emutos-config.md:332`, `include/config.h:980-981`); NewTOS is a no-op (`tos-api/src/xbios/system.rs:113-120@offgibit`) | (b) YM2149 (CLAUDE.md non-goal "no ... YM2149 in the kernel") | spec statement wrong, section 4 |
| `Setpalette` (0x06) | EmuTOS stores the pointer in `colorptr` (`$45A`) and the VBL copies it to the palette (`bios/screen.c:1168-1182`, `bios/vectors.S:358-372`); NewTOS copies the 16 words into its palette at once (`tos-api/src/xbios/screen.rs:320-339@setpalette`) and has no `colorptr` (`_sysvars-contract.md:120` class "hardware") | (b) for `colorptr`/VBL timing (no Shifter palette, no VBL, `docs/COMPATIBILITY.md:1410`); the observable result after a `Vsync` (`Setcolor(n,-1)`) is (a)-reachable and unmeasured | oracle can show the post-frame read via `Setcolor`; `tosdump/cases/COVERAGE-BIOS.md` row `0x06 | Setpalette`, `later` needs a restoring prelude |

The six floppy/DMA entries left the ticket by owner decision 2026-11-12 (ticket "Facts"; `docs/COMPATIBILITY.md`, "XBIOS floppy group"
as cited there, not re-read). Verdict for the lead: of seven, **no entry needs a new (a) fix ticket by itself**; `Mfpint`/
`Xbtimer` are (c), four are (b), `Setpalette`/`Initmous` have a small measurable (a) residue (the value after the call).

### XBIOS24 (P3, parked 2026-11-18) - class (b) and (c)

**Differs from EmuTOS:** nothing: EmuTOS has no TeraDesk question; the facts are what NewTOS tells a program that gates on
`_VDO` (`0x00030000`, ADR-012 amendment, `docs/MEMORY.md:448-459`) and `VsetMode(-1)` `0x001C` (a Falcon description of a screen the Pi
sets in `config.txt`, ADR-017 Revision 2). The step-1 analysis landed (`docs/analysis/teradesk-video-pi-mode-2026-11-18.md`, commit
`6651c9bfc`, read, sections 1-4: the dialog has no object for width x height; five options compared) and explains the Colours-256
observation from the source. Since then V69 clamped the dialog's plane count to 16 so it shows 64K at 32 planes
(`sdk/overlays/teradesk/video.c:483@xd_nplanes`, `docs/DEVIATIONS.md:54`); ADR-038 makes 32 planes a boot setting
(`docs/MEMORY.md:3646-3660`).

**Class:** (b) for any option that asks for a mode switch or a real Videl mode (no Videl; `VsetMode(mode != -1)` stays unimplemented,
ADR-017/ADR-012 amendment); (c) for the rest: a patch decision in `sdk/overlays/teradesk/` (option 1) or a documentation
statement. **Not (a):** there is no EmuTOS behaviour to equal and no oracle case (inferred; the analysis compares options, it
does not compare with the oracle).

**Left:** the owner's choice among options 0-5 of the analysis, then developer/tester with the scenario of the XBIOS22 run.
The row stays on the DEVIATIONS line 54 until then.

## 3. Proposed order of the (a) tickets

1. **XBIOS11** - one function, one file, behaviour already measured as a difference, the counter exists (`bios::tick::hz_200`),
   no layout or ADR touched. Needs the probe made permanent.
2. **XBIOS06** (P2, so first by priority if the owner prefers) - decided and measured, probe permanent, largest observable effect
   (negative LONG under LP64, `Iorec(0)` fields); bigger change (system.rs + iorec.rs + COMPATIBILITY entry).
3. **XBIOS16** - after one measurement of `Jdisint`/`Jenabint` on the oracle (`hkprobe` extension) and a spec file; then small, in
   the file XBIOS06 does not touch (`mfp.rs`).
4. **XBIOS19** - last: a page-0 layout change (area size/second area) with the MEM12 reservation asserts as risk, and a contract
   value ("a program cannot rely on it").

Independent of this list: XBIOS07 and BIOS13 are (c) edits to cites/statements; BIOS14 needs a tool decision; XBIOS20 and XBIOS24 need
owner decisions.

## 4. Wrong or stale spec statements found (not corrected here)

- `docs/api/xbios/22-kbdvbase.md:3,338-346,533-535,708` and `docs/api/xbios/_state.md:61-63`: NewTOS table at `0x9A0` in TPA page 0; code:
  private page `0x7F000` (`tos-api/src/bdos/process/loader.rs:1048@KBDVECS_AREA_VA`).
- `docs/api/xbios/_state.md:208-209`: `head` = write index, `tail` = read index; EmuTOS: `tail` is the write index (`bios/ikbd.c:163-172`).
- `docs/api/xbios/11-random.md` Pi binding: "no 200 Hz counter is materialised on the Pi" and "Gaps vs the contract: none functional";
  `_hz_200` is live (`docs/api/bios/_state.md:296`) and the measured first-seed difference exists (XBIOS11).
- `docs/api/xbios/_psg-group.md:5-8` (and `tos-api/src/xbios/system.rs:98-105@YM2149`): NewTOS "takes the EmuTOS `#else` branch ... `CONF_WITH_YM2149` off";
  the pinned target has `CONF_WITH_YM2149` on (`docs/api/_emutos-config.md:332`, `include/config.h:980-981`), so on the oracle `Giaccess`/`Ongibit`/`Offgibit`
  act on a PSG (`bios/sound.c:95-141`) and `Dosound` returns the old table pointer (`:144-156`). It is a difference to document, not an EmuTOS
  `#else` behaviour.
- `tosdump/cases/ERRATA.md` (section `bios-r1.tsv`, item 1): cites the `ETV_DEFAULT_JUST_RTS` and `ETV_DEFAULT_CRITIC` constants of `vectors.rs`, removed by BIOS07.
- `docs/api/xbios/0d-mfpint.md:99-102` "Open": still correct (no `1a`/`1b` spec file), not wrong.

## Open points

- STOP (needs deeper analysis): XBIOS16 `Jdisint` `d0` on the oracle: the code (`bios/mfp.c:39-56`) does not determine what the compiler leaves in
  `d0`; needs the disassembly of `references/emutos-oracle/1a7484ab/obj/mfp.o` (no m68k objdump on this host) or an oracle run.
- STOP: XBIOS20 `Initmous` `d0` on the oracle: not read from source, not measured; decides whether a value-only (a) fix exists.
- Not established: whether the owner wants XBIOS06 part 2 (TSR `0x81` readback) classed as (a) or (b); the ticket decision says "option (a) TT/MFP", I read it as (a).
- Out-of-scope observations: `docs/api/xbios/_psg-group.md` and `Giaccess`/`Dosound` (not in the nine tickets) differ from the oracle for the same
  YM2149 reason; `Dosound` returns the old table pointer on the oracle and `0` on NewTOS.
- A second researcher writes `docs/api/bios/_sysvars-counters-contract.md` (BIOS20); not touched.

### Inferred statements (not read directly)

1. BIOS13: "doc-check would flag the cite on touching" (ERRATA cite of removed symbols).
2. XBIOS06: that the oracle shows the TT-profile TSR behaviour only through Hatari's MFP emulation (ticket measurement says "Hatari-MFP"); part 2 as a software image of a 68901 flag.
3. XBIOS06: sizes ("2 code files, 3 doc files") are my estimate from the cited functions.
4. XBIOS07: that `2c-bconmap.md`'s answer to the `init_bconmap` question is correct (I read the spec text, not `serport.c:944,1056`).
5. XBIOS11: a zero seed arises only at `hz_200 == 0`; both rows "differ only then" derive from arithmetic on the cited code.
6. XBIOS16: `Jenabint` direct call answers the same byte as `Mfpint`'s tail (by the BIOS15 spec's disassembly, not re-read).
7. XBIOS19: the area-size risk of a second default image (I read the constants, not the MEM12 layout table in full).
8. XBIOS20: `Initmous`/`Setpalette` "(a) residue" is a hypothesis until measured; `Midiws`/`Offgibit` `d0` leftovers not examined.
9. XBIOS24: "not (a)": no EmuTOS counterpart behaviour compared.
10. The order in section 3 is a judgement on size and risk, not a measured fact.

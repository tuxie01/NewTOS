# BIOS21 — receipt (developer, 2026-11-20)

Role: developer
Agent: developer-a84a2b94baed1aecd

Branch `bios21` from `main` 13b18e50f, worktree `target-agents/agent-a84a2b94baed1aecd/wt`. Spec:
`docs/api/bios/_sysvars-contract.md` section 4 (the table, its 51 "served" rows) and section 8 (the proposed case
type, the guard, the return class per row); `docs/api/bios/_sysvars-counters-contract.md` section 17 (the decided
values of the cells BIOS20 serves, for the rows section 8 predates); `docs/TOS-REFERENCE-DUMPER.md` (the dumper's
case types and classes, extended). No file under `kernel/`, `tos-api/`, `tos-types/`, `tools/xtask/` or
`docs/api/` changed: `cargo xtask tosdump` takes any table, so xtask needs nothing.

Line cites into code below were kept stable on purpose: the new functions stand at the end of `cases.c` and
`runner_call.c`, the edits above them keep their line count (`cases.c`) or add one line (`runner_call.c`), so no
anchored cite in a doc this commit leaves alone moved (among them `docs/api/bios/_sysvars-contract.md:304`,
`00-getmpb.md:264`, `04-rwabs.md:626`, which this ticket may not edit). doc-check of the 37 docs that cite the
touched files: the same failure list as on `main` (gitignored inputs absent in a worktree), plus none.

## Changes

- `tosdump/core/tosdump.h`: "A PEEK CASE" (`:462@TD_CALL_TRAP_PEEK` = 257, `TD_PEEK_LOW` 0x380, `TD_PEEK_END`
  0x800, `TD_PEEK_MONO_VSYNCS` 2, `:469@TD_RET_MONOTONE` = 6); the trap field comment names 257.
- `tosdump/core/cases.h`: "Peek cases" in the call-table comment; `td_peek_column` and `td_noncall_case_parse`
  declared (`:1116@td_peek_column`).
- `tosdump/core/cases.c`: the loader. `is_peek_column`, `is_write_column` (`poke:`, `write:` refused by name),
  `parse_peek_case` (`:5071@parse_peek_case`: `peek:<addr>:<width>`, the guard `0x380 <= addr`, `addr + width <=
  0x800`, width 1/2/4, WORD/LONG even, args and capture `-`, classes EXACT/IGNORE/NONZERO and at width 4
  PTR_RAM/MONOTONE, no width suffix), `td_peek_column`, `td_noncall_case_parse` (`:5154@td_noncall_case_parse`: jar
  or peek; a write form `TD_CASE_ERR_UNSAFE_OPCODE`, a peek prelude `TD_CASE_ERR_DIRECTIVE`). In
  `td_call_cases_parse` the jar branch takes peek lines too (`:4772@td_peek_column`): the same rules after the
  parse (no prelude before it, not in a `Super` bracket, not after `Getmpb`); `td_retclass_name` knows
  `MONOTONE` (`:2757@TD_RET_MONOTONE`).
- `tosdump/core/runner_call.c`: `td_cb_peek` (`:642@td_cb_peek`, the Supexec callback: one volatile load of the
  width through `td_abi_addr`), `peek_once` (Supexec 14:0x26), `vsync` (14:0x25), `run_peek_case`
  (`:690@run_peek_case`: trap 257, args[0] the address, ret the value zero-extended, ret_hi 0; MONOTONE: two
  Vsyncs, the second read at capture[0..3] big-endian); `td_run_call` sends jar and peek cases through one branch
  (`:509@run_peek_case`).
- `tosdump/host/tosdiff.c`: the case header names the cell (`case 75  peek $4BA:4 …`, `:825@TD_CALL_TRAP_PEEK`);
  `is_peek_monotone`, `monotone_check` (`:1021@monotone_check`: per side, second minus first modulo 2^32 in
  1..0x7FFFFFFF, "did not advance" / "went back"; the capture not compared byte-wise). EXACT/NONZERO/PTR_RAM/IGNORE
  of a peek go through the existing `retclass_check`.
- Host tests: `tosdump/tests/test_peek.c` (new: form, 33 refusals, runner against the stub's window, the stub
  dump), `tosdump/tests/stub_abi.h`/`.c` (Vsync adds 1 to the LONG at `TD_STUB_VSYNC_CELL` = 0x5FC and counts
  `td_stub_vsyncs`), `tosdump/tests/test_tosdiff.sh` ("peek cases": EXACT, NONZERO, MONOTONE stands / goes back /
  wraps / other start on the other side, the self-check), `tosdump/host/Makefile` (`test_peek` in `run-tests`).
- `tosdump/cases/bios-sysvars-r1.tsv` (new, 54 cases, CRC AE3147CF) and its references
  `tosdump/reference/EMUTOS-1a7484ab-BIOS-sysvars.{BIN,LOG}` (tt) and `-BIOS-sysvars-falcontc.{BIN,LOG}`.
- Docs: `docs/TOS-REFERENCE-DUMPER.md` (section 3 trap 257 and the class note; section 6 "System variables: the
  peek case": the type, the guard, the read, the record, `MONOTONE`, the table; section 7 the DUMPBIOS row);
  `tosdump/cases/COVERAGE-BIOS.md` (the ledger row for the peek form).
- Run: `measurements/measurement-2026-11-20-bios21/RUNINFO.TXT` (method, oracle and NewTOS runs, negative control,
  regression of the existing tables).

Classes. Section 8's groups as proposed: fixed magic or zero EXACT (21 rows); address PTR_RAM (12); `ramtop`,
`bootdev`, `drvbits` NONZERO (section 8 offers "NONZERO or IGNORE after the first measurement": NONZERO holds on
both oracle profiles, `bootdev` is 2 on both oracle profiles and on NewTOS but stays NONZERO as section 8 groups
it); `hz_200` MONOTONE. The BIOS20 cells from section 17.2: `_timer_ms` 20, `_vblsem` 1, `_nvbls` 8, `_vblqueue`
`$4CE` EXACT; `_v_bas_ad`, `_hdv_init`, `_hdv_boot`, `_dskbufp`, `_dump_vec`, `_aux_stat`, `_aux_vec` PTR_RAM
(addresses that differ by design); `_frclock`, `_vbclock` MONOTONE; `_themd` as m_link EXACT (0), m_start
PTR_RAM, m_length NONZERO, m_own EXACT (0). MONOTONE asks for an advance, not only "nondecreasing" (section 8's
word): a counter that stands passes "nondecreasing"; the two Vsyncs between the reads wait at least a whole frame,
and every oracle and NewTOS run advanced (+2 frames, +6/+7 ticks).

## Build, clippy, tests (real output, kept in the agent's scratch directory, not committed)

- `cargo xtask build --board qemu-virt` OK, 159 `warning` lines; `cargo xtask build --board rpi4` OK, 159
  (both rebuilt after a touch of `kernel/src/main.rs`, `tos-api/src/lib.rs`). Baseline 159/159 (main, BIOS20 test
  run). Delta 0 (no Rust changed).
- `cargo clippy -p tos-api --all-targets` 118 `warning` lines; `cargo clippy -p kernel --features board-rpi4` 196;
  baseline 118 / 196 (`measurements/measurement-2026-11-20-bios20-test/RUNINFO.TXT`). New 0.
- `cargo xtask test` (full): exit 0, 2599 passed / 0 failed / 7 ignored over 224 results (= main's 2599/0/7).
- `make -C tosdump host-test`: exit 0, 0 FAIL (`measurements/measurement-2026-11-20-bios21/runs/host-test.txt`);
  the self-check passes all 52 pinned references, the two new ones included.
- m68k `DUMPBIOS.PRG` and AArch64 `DUMPBIOS.NAP` build with `-Wall -Wextra -Werror`.
- `cargo xtask status-check --new-only`: see the commit (run before it, 0 rejected).

## Per case (own run; `measurements/measurement-2026-11-20-bios21/runs/per-case.txt` has the three values per case)

All 54 cases EQUAL against tt and against falcontc: tosdiff 0 of 54 on both references. So no DIFF needs a
`docs/DEVIATIONS.md` row or a ticket.

| Case | Cell | Class | tt | falcontc |
|---|---|---|---|---|
| 32 | `_etv_timer` $400:4 | PTR_RAM | EQUAL | EQUAL |
| 33 | `_etv_critic` $404:4 | PTR_RAM | EQUAL | EQUAL |
| 34 | `_etv_term` $408:4 | PTR_RAM | EQUAL | EQUAL |
| 35 | `_etv_xtra` $40C:4 | EXACT | EQUAL | EQUAL |
| 37 | `_memvalid` $420:4 | EXACT | EQUAL | EQUAL |
| 38 | `resvalid` $426:4 | EXACT | EQUAL | EQUAL |
| 39 | `resvector` $42A:4 | EXACT | EQUAL | EQUAL |
| 40 | `_phystop` $42E:4 | PTR_RAM | EQUAL | EQUAL |
| 41 | `_membot` $432:4 | PTR_RAM | EQUAL | EQUAL |
| 42 | `_memtop` $436:4 | PTR_RAM | EQUAL | EQUAL |
| 43 | `_memval2` $43A:4 | EXACT | EQUAL | EQUAL |
| 45 | `_timer_ms` $442:2 | EXACT | EQUAL | EQUAL |
| 46 | `_bootdev` $446:2 | NONZERO | EQUAL | EQUAL |
| 47 | `_palmode` $448:2 | EXACT | EQUAL | EQUAL |
| 48 | `_defshiftmod` $44A:1 | EXACT | EQUAL | EQUAL |
| 49 | `_v_bas_ad` $44E:4 | PTR_RAM | EQUAL | EQUAL |
| 51 | `_vblsem` $452:2 | EXACT | EQUAL | EQUAL |
| 52 | `_nvbls` $454:2 | EXACT | EQUAL | EQUAL |
| 53 | `_vblqueue` $456:4 | EXACT | EQUAL | EQUAL |
| 54 | `_screenpt` $45E:4 | EXACT | EQUAL | EQUAL |
| 56 | `_vbclock` $462:4 | MONOTONE | EQUAL | EQUAL |
| 57 | `_frclock` $466:4 | MONOTONE | EQUAL | EQUAL |
| 59 | `_hdv_init` $46A:4 | PTR_RAM | EQUAL | EQUAL |
| 60 | `_swv_vec` $46E:4 | PTR_RAM | EQUAL | EQUAL |
| 61 | `_hdv_boot` $47A:4 | PTR_RAM | EQUAL | EQUAL |
| 63 | `_cmdload` $482:2 | EXACT | EQUAL | EQUAL |
| 64 | `_conterm` $484:1 | EXACT | EQUAL | EQUAL |
| 65 | `trp14ret` $486:4 | EXACT | EQUAL | EQUAL |
| 66 | `criticret` $48A:4 | EXACT | EQUAL | EQUAL |
| 68 | `_themd.m_link` $48E:4 | EXACT | EQUAL | EQUAL |
| 69 | `_themd.m_start` $492:4 | PTR_RAM | EQUAL | EQUAL |
| 70 | `_themd.m_length` $496:4 | NONZERO | EQUAL | EQUAL |
| 71 | `_themd.m_own` $49A:4 | EXACT | EQUAL | EQUAL |
| 72 | `____md` $49E:4 | EXACT | EQUAL | EQUAL |
| 73 | `sav_context` $4AE:4 | EXACT | EQUAL | EQUAL |
| 75 | `_hz_200` $4BA:4 | MONOTONE | EQUAL | EQUAL |
| 76 | `the_env` $4BE:4 | EXACT | EQUAL | EQUAL |
| 77 | `_drvbits` $4C2:4 | NONZERO | EQUAL | EQUAL |
| 78 | `_dskbufp` $4C6:4 | PTR_RAM | EQUAL | EQUAL |
| 79 | `_autopath` $4CA:4 | EXACT | EQUAL | EQUAL |
| 81 | `_dumpflg` $4EE:2 | EXACT | EQUAL | EQUAL |
| 82 | `_prtabt` $4F0:2 | EXACT | EQUAL | EQUAL |
| 83 | `_sysbase` $4F2:4 | PTR_RAM | EQUAL | EQUAL |
| 84 | `_shell_p` $4F6:4 | EXACT | EQUAL | EQUAL |
| 85 | `_end_os` $4FA:4 | PTR_RAM | EQUAL | EQUAL |
| 87 | `_dump_vec` $502:4 | PTR_RAM | EQUAL | EQUAL |
| 88 | `_prt_stat` $506:4 | PTR_RAM | EQUAL | EQUAL |
| 89 | `_prt_vec` $50A:4 | PTR_RAM | EQUAL | EQUAL |
| 90 | `_aux_stat` $50E:4 | PTR_RAM | EQUAL | EQUAL |
| 91 | `_aux_vec` $512:4 | PTR_RAM | EQUAL | EQUAL |
| 92 | `_memval3` $51A:4 | EXACT | EQUAL | EQUAL |
| 94 | `_p_cookies` $5A0:4 | PTR_RAM | EQUAL | EQUAL |
| 95 | `_ramtop` $5A4:4 | NONZERO | EQUAL | EQUAL |
| 96 | `_ramvalid` $5A8:4 | EXACT | EQUAL | EQUAL |

Documented differences these classes cannot show (by design of a per-side class, already rows of
`docs/DEVIATIONS.md` or ADR-031): `_hdv_init` holds the `ret` stub on NewTOS and `flop_hdv_init` on the oracle
(DEVIATIONS "A program calls `_hdv_boot` … or `_hdv_init` …"); the three `etv_*` cells hold one stub on NewTOS and
three handlers on the oracle (ADR-031, contract table rows `$400`-`$408`); `_themd.m_length` on falcontc is not
`memtop - membot` (DEVIATIONS "A program reads `_themd`"). Not read by the table: `_vbl_list` slot 0 and `_pun_ptr`
(not "served").

Negative control (`measurements/measurement-2026-11-20-bios21/runs/negctl-bios-sysvars-r1/`): the same dumper and table on the kernel of `c2140d9ef` (before
BIOS20): 15 of 54 cases differ on both references, exactly the BIOS20 cells; `$462`/`$466` "0x00000000 then
0x00000000, did not advance".

Existing tables: on NewTOS `bios-r1` 35/147 (tt), 31/147 (falcontc), `bios-core-r1` 2/44, 1/44; the tt reports are
byte-identical (paths stripped) to the BIOS20 test run's. On the oracle with this dumper: `bios-core-r1` records
byte-identical to the frozen references; `bios-r1` 0 cases differ by tosdiff, records differ only in `Random` and
`Gettime` (`ret`), the same two as the HEAD dumper of the same day against the frozen reference (control run): the
clock, not the change. The frozen references and the existing tables are byte-unchanged (`git diff` of
`tosdump/reference/` and `tosdump/cases/` shows only the four new reference files and the new table).

## Tester handoff

- Host: `make -C tosdump host-test` (sections "system variables: peek…" and the comparator's "peek cases"; the
  self-check of 52 references); `cargo xtask test`.
- Oracle: `tools/hatari-oracle.sh tt|falcontc tosdump/build/DUMPBIOS.PRG tosdump/cases/bios-sysvars-r1.tsv <out>`
  (after `make -C tosdump dumpbios-m68k`); expected: BIN byte-identical to `tosdump/reference/
  EMUTOS-1a7484ab-BIOS-sysvars{,-falcontc}.BIN` (Hatari was deterministic in two runs each) and `tosdiff
  --selfcheck` passing. The method scripts are in `measurements/measurement-2026-11-20-bios21/method/`.
- NewTOS raspi4b: `cargo xtask tosdump bios tosdump/cases/bios-sysvars-r1.tsv --out <dir> --reference
  tosdump/reference/EMUTOS-1a7484ab-BIOS-sysvars.BIN --reference …-falcontc.BIN`: expected 0 of 54 differ on both,
  0 `[hook]` lines; `bios-r1` and `bios-core-r1` the same differing cases as before (35/147, 2/44 on tt).
- No screendump, no selftest, no qemu-virt run: the change is in the dumper and the comparator only. Real-Pi4-only:
  nothing new (the same DUMPBIOS on the real Pi would read the same per-process page 0; not claimed).

## Proposed `docs/STATUS.md` line

2026-11-20 — BIOS21: tosdump reads system variables. New case type `peek:<addr>:<width>` (read inside `Supexec`,
`$380`-`$7FF` only, no write form) and class `MONOTONE`; table `tosdump/cases/bios-sysvars-r1.tsv` (54 cases, every
"served" row of `docs/api/bios/_sysvars-contract.md`), references `EMUTOS-1a7484ab-BIOS-sysvars` (tt, falcontc).
NewTOS raspi4b: 0 of 54 differ on both; the kernel before BIOS20 differs in 15. Existing tables and references
unchanged. `measurements/measurement-2026-11-20-bios21/RUNINFO.TXT`. Commit <hash>.

## Out-of-scope observations

- `_phystop` equals `_memtop` on NewTOS (`0x00E00000`); on the oracle `_phystop` lies above `_memtop` (the screen
  sits between). PTR_RAM on both; not a contract question this ticket asks.
- `tosdump/host/casecheck.c` has no mode for call tables (`--aes`, `--raster` only); a table of the call form is
  checked only by the dumper's loader or the host tests.
- Hatari runs of `bios-r1` drift against its frozen reference in `Random` and `Gettime` with the date (IGNORE /
  NONZERO, so tosdiff passes); a byte comparison of a re-run against that reference never holds.

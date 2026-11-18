# V69 — receipt (developer): part A, tosdump at 32 planes

Role: developer
Agent: a00b6a784b430f5d6

Date 2026-11-18. Branch `v69` from `main` @ `06ef14f5b` (worktree `target-agents/agent-a00b6a784b430f5d6/wt`).
Part A of TASK/V69.md only ("Spec gate ... two parts"): the tosdump instruments at the 32-plane setting, built as
`docs/TOS-REFERENCE-DUMPER.md`, section "The 32-plane setting: tosdump at `vdiplanes=32`", states it. Part B (TeraDesk,
PRESENT, HighWire, zView, DOOM, `sdk/overlays/`) is untouched: it waits for the owner. No product code: nothing in
`kernel/`, `tos-api/`, `tos-types/`, `programs/`; no `mapping.tsv` row touched (no mapped function changed). No
frozen reference regenerated or edited; no baseline frozen.

## First step: the existing tables at `vdiplanes=32`, per case (facts)

Method: a run script in the agent directory (gitignored, quoted under "Gate" below) on a detached worktree at the base commit
(`target-agents/agent-a00b6a784b430f5d6/base`, `06ef14f5b`, before any change): `cargo xtask tosdump` of the nine VDI and
raster tables plus `aes-r1` and `bios-r1`, default and `--bootargs vdiplanes=32`; then `tosdiff <table> DEFAULT.BIN
V32.BIN` per table (NewTOS against NewTOS, same commit, only the boot argument differs). Every difference is explained by
the landed spec; none looks like a product defect, so nothing was stopped.

| Table | Differ (default vs 32) | The cases, and the rule that explains them |
|---|---|---|
| `vdi-all.tsv` | 6 of 182 | 34, 250, 252 `vq_extnd 1`: `INTOUT[4]` 16 -> 32 (ADR-038 point 3). 54, 160, 161 `v_get_pixel` of a white pixel: `INTOUT[0]` 0 -> `0x00FF`, `INTOUT[1]` `0xFFFF` at both (V65 section 6, `{0x00FF, 0xFFFF}`) |
| `vdi-v18.tsv` | 11 of 19 | all `v_get_pixel` (105): 29, 32, 43, 59 white `{0,FFFF}` -> `{00FF,FFFF}`; 15, 109, 154, 199 red `{0,F800}` -> `{00FF,0000}`; 18 green `07C0` -> `{0000,FF00}`; 21, 24 blue `001F` -> `{0000,00FF}` -- the pen pixels of V64 P3 in the word shape of V65 section 6 |
| `vdi-v18-t2.tsv` | 5 of 32 | 74, 79, 102, 105 white `{0,FFFF}` -> `{00FF,FFFF}`; 86 pen 0 `{0,FFDF}` -> `{00FF,FFFF}` (pen 0 = `0x00FFFFFF`, P3) |
| `vdi-v18-t3.tsv` | 0 of 2 | -- |
| `vdi-a131-palette.tsv` | 23 of 48 | 8 realised `vq_color` (pens 8..15, cases 35-49 odd): the 5-bit answer at 16 (742, 548, 677) against the request at 32 (733, 533, 667), V64 P5/P7; 15 `v_get_pixel` in a `v_bar` (pens 0, 2..15; pen 1 is 0 at both): the pen pixels of P3, e.g. pen 8 `{00BB,BBBB}`, pen 9 `{0088,8888}`, pen 10 `{00AA,0000}`. `contract32-check.py` on the 32 dump: 0 of 48 differ from the contract. The gate's "16 pixel plus 7 realised" was a reading; measured is 8 + 15 |
| `vdi-raster.tsv` | 24 of 59 | 124-139, 146-150, 157, 164-166: the 16/16 memory-form group (ops 0-15 except 5, the geometry and overlap cases) -- all refused at 32 (O5, V65 section 2): both buffers equal their fill in all 24 (checked word by word), only `buf_a`/`buf_b` fields differ. Cases 129 (`D_ONLY`) and 140 (`PAT_FLAG`) are no-ops at both. Planar lines: 0 differ |
| `vdi-v18-raster.tsv` | 1 of 17 | 43, the 16-plane control: refused at 32, buffers = fill |
| `vdi-v18-t2-raster.tsv` | 16 of 26 | 28-31 `vrt_cpyfm` into 9, 12, 15, 16 planes; 33-38, 40-45 `vro_cpyfm` of 9, 12, 15, 16 planes: all refused (9..31, O5), buffers = fill in all 16 |
| `vdi-raster-screen.tsv` | -- | refused by the dumper, `screen mismatch` (declares 16, workstation 32): exit 1, no dump, as designed (gate "What stays", point 3) |
| `aes-r1.tsv` | 95 of 95 | every case: `global[10]` (`ap_planes`) 16 -> 32 (ADR-038 points 3, 10); nothing else |
| `bios-r1.tsv` | 0 of 147 | -- |

## What changed, by file and function

- `tosdump/core/tosdump.h`: `TD_AREA_RASTER32` = 7; the wide record constants `TD_RAST32_*` (5744 bytes, `BUF_A` 1636,
  `BUF_B` 3684, reserved 5732, 1024-WORD buffers, `TD_RAST32_PLANES`); comment "at the 16-plane setting" (row 11).
- `tosdump/core/cases.h`: section "THE 32-PLANE SETTING: A WIDE TABLE" (the rules); `td_rast_screen.wide`; guard 3 text.
- `tosdump/core/cases.c`: `parse_rast_directive` accepts 1..16 or 32 and sets `wide`; `parse_mfdb` caps planes at 32 and
  bounds `fd_h * fd_wdwidth * 32 <= 1024` in a wide table, unchanged otherwise; the `vr_trnfm` comment (row 4). Written
  line-neutral: every line cite into `cases.c` from `docs/MEMORY.md`, `docs/api/` and the audits stays where it was.
- `tosdump/core/record.h`/`record.c`: `struct td_raster32_arrays` (own type, the narrow frame unchanged),
  `td_poison_raster32`, `td_raster32_record_write`, `td_raster32_fields`, area 7 in `td_area_record_size`,
  `td_area_name` ("VDI-RASTER32"), `td_area_fields`.
- `tosdump/core/runner_rast.c`/`.h`: the frame, record size and area chosen by the table (`td_rast_record_size`,
  `td_rast_area`); the wide readback stores `INTOUT[0]`, `INTOUT[1]` at `2 * (row * 32 + col)`, `+1`.
- `tosdump/core/dumprast.c`: record space for 64 wide records; the label log line; area and size from the table.
- `tosdump/host/tosdiff.c` (area-7 kept pair, the "NewTOS contract" line), `casefile.c` (area-7 columns), `casecheck.c`
  (names a wide table), `Makefile` (the wide stub dump; the checker's selftest).
- `tosdump/host/contract32-check.py` (new): the check of a 32-plane dump against the contract (see "As built").
- `tosdump/cases/vdi-raster32.tsv`, `tosdump/cases/vdi-raster-screen32.tsv` (new): the 32-plane tables, 64 cases each.
- `tools/xtask/src/tosdump.rs`: `vdi_planes`, `table_screen_planes`, `table_is_wide`, `out_stem`, `dump_header`,
  `written_by_newtos`, `wide_reference_refusal`, `ORACLE_AT_32_MARK`, `CONTRACT32_CHECK`; in `cmd`: the refusals, the
  report lines, `RASTER32.BIN`, the INFORMATIONAL mark, the contract check; three unit tests.
- `tosdump/probes/icprobe.c`, `icprobe-compare.py`, `run-icprobe.sh`: the wide variant (A.3).
- Tests: `tosdump/tests/test_record.c` (`test_raster32_record`, the wide field table; area 7 known, 8 unknown),
  `test_cases_rast.c` (`test_wide_rules`, the shipped wide tables), `test_runner_rast.c` (the wide record and readback
  order against the stub), `test_tosdiff.sh` (validate, label, wide vs narrow refused).
- Docs: `docs/TOS-REFERENCE-DUMPER.md` (area 7 in the header table; "As built" closing the section's open points; the
  gate's anchors moved with the code), `docs/DEVIATIONS.md` (the two rows for the raster dumper and icprobe at 32
  removed: the change removes both differences), `docs/COMPATIBILITY.md` (the paragraph after the ADR-038 table: the two
  V69 items removed, the tosdump tables named as the running-system gate), `docs/api/vdi/raster/_raster.md` (anchor-only:
  the two `icprobe.c` line cites of `fd_nplanes`, line 185 -> 227).
- Anchor-only (the code they cite moved): `TASK/TOOL19.md`, `TASK/TOOL39.md`, `TASK/XBIOS17.md`, one
  `tools/xtask/src/tosdump.rs` cite each.

## STOP items read, what was found

- **STOP-block 1** (`tools/xtask/src/tosdump.rs`, read to the end): file names are `<area>.BIN`/`.LOG` from the dumper
  table (the dumper's default argv on the card); `--reference` is copied and handed to `tosdiff`, no matching of its own.
  So a wide dump would have been `RASTER.BIN` and an oracle reference would have reached `tosdiff` (which refuses on area
  and CRC, exit 2 = "tosdiff refused", a failed run). Built: refusal before anything is built, the saved name
  `RASTER32.BIN`, the INFORMATIONAL mark for oracle comparisons of any other table at 32, the contract check.
- **STOP-block 2**: the per-case lists, measured, in the table above.
- `tosdump/core/dumprast.c:128@planes` (gate point 3) read: unchanged; it refuses a 16-plane table at 32 and a wide one
  at 16 (measured: `vdi-raster-screen.tsv` at 32 above).

## Where "NewTOS contract" is written (fixed as the spec proposes)

(a) the first comment line of both wide tables; (b) DUMPRAST's log line; (c) the baseline prefix `NEWTOS32-`, enforced by
`cargo xtask tosdump`; (d) the area name `VDI-RASTER32`; plus `tosdiff`'s line for area 7, the `setting`/`table` lines and
the INFORMATIONAL mark in `tosdump-run.txt`, and the check script's output. A 32-plane dump cannot be compared with an
oracle reference by mistake: for a wide table the tool refuses before building; for any other table at 32 the comparison
is marked informational.

## Gate: the 16-plane bar, and the 32-plane tables

**The run script** (in the agent directory, gitignored; per table, `<tree>` the measured worktree, also the harness):

```
cargo xtask tosdump <vdi|rast|aes|bios> tosdump/cases/<table>.tsv --out runs/<label>/<table> --scratch <agent>/scratch/ts-<label> \
    [--bootargs vdiplanes=16|vdiplanes=32] --reference tosdump/reference/EMUTOS-1a7484ab-<ref>.BIN ...
```

tables and references as the V67 test run (`measurements/measurement-2026-11-18-v67-test/method/run-tables-vdi.sh`) plus
`aes-r1` (`-AES`) and `bios-r1` (`-BIOS`, `-BIOS-falcontc`). Labels: `base-default`, `base-v16`, `base-v32` on the base
worktree at `06ef14f5b` (before the change); `wt-default`, `wt-v16`, `wt-v32i` on this branch's final tree (all runs after
the last code edit; build id `06ef` on both sides). Compared by a script in the agent directory: dump bytes past the
32-byte header whole and with the raster MFDB pointer cells (record offsets 22..37, 52..67) masked, and every
`tosdiff-*.txt` with absolute paths removed.

| Check | Result |
|---|---|
| "X of Y cases differ" lines, default and `vdiplanes=16`, 11 tables, base vs branch | **identical** (both configs, every table) |
| `tosdiff` report texts (incl. `--validate`), default and 16 | **identical**, all 22 table runs, 2-3 reports each |
| dump headers | identical (32 bytes, every run) |
| payload, VDI (5 tables), AES | **byte-identical** at default and 16 |
| payload, raster (4 tables) | byte-identical **except the MFDB pointer cells** (address class, never compared by value): 744 / 708 / 204 / 312 differing bytes, all inside those cells; masked, identical. The cells hold DUMPRAST's own buffer addresses, which move with its code and data size -- the precedent `measurements/measurement-2026-10-26-raster-v3/RUNINFO.TXT`. The literal "same payload bytes" of the HANDOVER cannot hold for a raster dump of a changed DUMPRAST |
| payload, `bios-r1` | default: 3 bytes differ, record 52 = case 112 `Random()` (XBIOS 17, retclass `IGNORE`, `tosdump/cases/bios-r1.tsv` line 112): run-to-run noise -- `base-default` against `base-v16` differs in the same 3 bytes; at 16 identical |
| `vdiplanes=32` runs of the 11 tables (informational) | every count equal to the base's `vdiplanes=32` run; every oracle comparison now carries the `[INFORMATIONAL: run at vdiplanes=32 ...]` mark; `vdi-raster-screen.tsv` refused by the dumper (exit 1) as before. **Missed here, found by the tester (F1, rework below):** the contract-check line of these runs -- `vdi-v18.tsv` reported exit 1, 4 of 19 |
| `vdi-raster32.tsv` at 32 | `run complete`, `RASTER32.BIN` area 7, 64 x 5744 bytes; **contract check exit 0, 0 of 64 cases differ** |
| `vdi-raster-screen32.tsv` at 32 | the same: **0 of 64** |
| `vdi-a131-palette.tsv` at 32 | **contract check 0 of 48** |
| contract check, negative control | a forged copy of the `vdi-raster32` dump (one buffer bit, one `INTOUT` word changed) -> exit 1, "2 of 64 cases differ", both named |
| refusal before build | `rast vdi-raster-screen32.tsv --reference EMUTOS-1a7484ab-RASTER-screen-falcontc.BIN` without `--bootargs`: "2 problem(s), nothing run" (no `vdiplanes=32`; the oracle reference of area 6) |
| icprobe, HypView's resource (`sdk/apps/hypview/build/hyp_view.rsc`, 7 trees), NewTOS | 16-plane `ICPROBE.BIN` of the new probe byte-identical to the base probe's (`cmp`); the log gains `P planes 16, record version 1`. At 32: version 2; `icprobe-compare.py n16 n32`: PENS CONTRACT, 15 I cases MEMBER, 15 M EQUAL, 28 T MEMBER (no pixel needed the projection), **cases 59, differing 0**; two version-2 runs word for word: 0; version 2 before version 1 refused |
| first step (above) | every difference explained by the spec; no product defect |

**Build, clippy, tests.**

- `cargo xtask build --board qemu-virt`: OK; `cargo xtask build --board rpi4`: OK (debug); `--board rpi4 --release` built by
  every tosdump run: OK. No kernel source touched; the kernel's 93 warnings are the base's.
- `cargo clippy -p xtask --all-targets`: warning summary identical to the base worktree's (bin 1, bin test 9): **delta 0**.
- `cargo xtask test` (full): **2579 passed, 0 failed, 7 ignored** (baseline 2576/0/7; +3 the new xtask tests
  `the_vdi_depth_is_read_as_the_kernel_reads_it`, `a_wide_table_is_the_one_that_declares_32_planes`,
  `a_wide_table_is_compared_only_with_a_newtos32_baseline`).
- `make -C tosdump/host test`: pass (the new wide record, parser, runner, tosdiff and checker-selftest cases included).
- `make -C tosdump dumpvdi-m68k dumpaes-m68k dumprast-m68k dumpbios-m68k icprobe-m68k icprobe-newtos dumprast-newtos`
  (`-Werror`): all build. The m68k dumpers were not run (the oracle references stay frozen).
- `cargo xtask doc-check` (all of `docs/`): the failure set equals the base's but for one pre-existing line moving by
  three lines (`docs/COMPATIBILITY.md`, a gitignored `sdk/cflib` path); on the edited files only the worktree's absent
  gitignored `sdk/gemlib`/`sdk/cflib`/`sdk/zview` sources fail, as at the base. `cargo xtask status-check --new-only`:
  0 rejected.

**Gitignored outputs cited** (in the agent directory of `agent-a00b6a784b430f5d6` under `target-agents`, 8.0 G with
both worktrees, deleted when the ticket lands): the `vdi-raster32` run's `RASTER32.BIN`
`7da01ef2db53118e31ded4507171f390195b6c8e291ba953b6b080f56cc4e32e`; the `vdi-raster-screen32` run's `RASTER32.BIN`
`7e0789eb4c15b929fe25a66521c6970ee1fc1e87dc614c58e6eb44d6b66d08af`; the `vdi-a131-palette` run at 32, `VDI.BIN`
`1747701ac7b328ff29976a23f000dc5368d7affde1e43c3bb193ea9ca4a344c9`; icprobe at 16, branch and base probe alike,
`ICPROBE.BIN` `6be8708d45f5d56a0257af6b810a8f2f00d2e81b682698f684c4e7671d56c494`; icprobe at 32, `ICPROBE.BIN`
`6a35063ecb4729ce6bd620e68184532aae1d409e63413347c6ebb151356b8bd7`; release `kernel8.img` of the base `06ef14f5b`
`471be47f8a03bdc57cb67953f6ea04c913237b0346f44d42e6f6b9218440ffd7`. **Corrected after the test:** this receipt first said
the branch's image is the same; it is not -- the kernel embeds the commit id, and the tester's fresh build of
`5ee76a97b` measured `517010ede7ce83d6e2b02ed2aefabe883f7779ae973d758a8fd07e4567d297aa`. My runs booted an image cargo had
not rebuilt since the base commit (no kernel source changed, so the build was "up to date" and kept the base's id; every
tosdump run of mine logs `471be47f...`). The kernel code is the base's in both; only the embedded id differs.

## Rework after the tester's FAIL (2026-11-19, test run of `5ee76a97b`, finding F1)

**The failure.** `cargo xtask tosdump vdi tosdump/cases/vdi-v18.tsv ... --bootargs vdiplanes=32` ended with "contract
check ... exit 1, 4 of 19 cases differ" (cases 29, 32, 43, 59, `v_get_pixel` of a white pixel, recorded `{0x00FF, 0xFFFF}`,
the correct answer at 32). **Cause, verified in the code before the change:** `is_palette_table()` of
`tosdump/host/contract32-check.py` took every VDI table whose cases are only opcodes 26 and 105 for
`vdi-a131-palette.tsv`; `vdi-v18.tsv`'s 19 cases are all 105, so `check_palette()` judged them by the palette table's rule
(a `v_get_pixel` reads the `v_bar` its own `vsf_color` prelude painted; without such a prelude, pen 1, `0x000000`) -- the
tester's reading was right. My gate missed it: the per-table list of the `vdiplanes=32` runs showed the `tosdiff` lines
only, not the contract-check line.

**The fix (one attempt).** The checker decides which tables it judges by markers that cannot match another table by
accident: a raster table by its own `!screen w,h,32` (unchanged), a VDI table only by the CRC32 of the table file, the
identity every dump of it carries (`VDI_CONTRACT_TABLES` = `vdi-a131-palette.tsv`, `7E23929E`); `classify()` and `check()`
carry the rule, `main()` uses them. Every other table at 32 ends with exit 3, "not checked". **Test:** the selftest
(`make -C tosdump/host test`) classifies every shipped table under `tosdump/cases/` (only the two wide tables and
`vdi-a131-palette.tsv` are checked), runs a forged `vdi-v18.tsv` dump whose `v_get_pixel` cases answer `{0x00FF, 0xFFFF}`
through `check()` (expected: not judged), and a forged `vdi-a131-palette.tsv` dump with the same answers (expected: judged and
reported -- the palette rule still bites).

**Measured after the fix** (QEMU raspi4b, `cargo xtask tosdump`, `--bootargs vdiplanes=32`, the branch with the fixed
checker): the eleven 16-plane tables -- every run's contract check ends exit 3 "not checked" (`vdi-v18.tsv` included), no
exit 1 anywhere; every `X of Y cases differ` count equal to the base's `vdiplanes=32` run; `vdi-raster-screen.tsv` refused
by the dumper as before. `vdi-raster32.tsv` 0 of 64, `vdi-raster-screen32.tsv` 0 of 64, `vdi-a131-palette.tsv` 0 of 48
(contract check exit 0). The checker run directly on the earlier `vdiplanes=32` dumps of `vdi-v18`, `vdi-all`,
`vdi-v18-t2`, `vdi-v18-t3`, `vdi-raster`: exit 3; on `vdi-a131-palette`: exit 0. **Negative control:** the forged copy of
the `vdi-raster32` dump (one buffer bit, one `INTOUT` word) still ends exit 1, "2 of 64 cases differ"; the raster path of
the checker is unchanged, so the tester's control (a changed expected pen value is reported) holds.

**(a) The 16-plane `!screen` table at 32.** `docs/TOS-REFERENCE-DUMPER.md` ("As built") now says it: a table that
declares a 16-plane screen (`vdi-raster-screen.tsv`) is refused by the dumper at `vdiplanes=32`, by design, and the run is
reported failed; its twin is `vdi-raster-screen32.tsv`. Reasoning for keeping the `docs/DEVIATIONS.md` row removed:
DEVIATIONS indexes differences between NewTOS and EmuTOS by the trigger a program meets; this refusal is a rule of the
measuring tool (a table's expected words belong to the depth it declares, and the same rule refuses a wide table at 16 and
would refuse a mismatched table on the oracle), not a behaviour of the product, so its place is the tool's document. The
removed row described the state before V69 (no table could run at 32 with a screen); that difference is gone.

**(b)** the `kernel8.img` statement is corrected above (Gate, "Gitignored outputs cited").

**(c)** the MFDB pointer cells (raster payloads not byte-identical): confirmed by the tester and accepted by the lead;
no change.

Build, clippy, tests after the fix: `cargo xtask build --board qemu-virt` and `--board rpi4` OK; `cargo clippy -p xtask
--all-targets` bin 1 / bin test 9 warnings, as the base (delta 0; no Rust file changed by the rework); `cargo xtask test`
2579 passed, 0 failed, 7 ignored; `make -C tosdump/host test` pass (the new selftest cases included); doc-check of the
edited files: only the worktree's absent gitignored `sdk/gemlib` sources, as at the base.

## Tester handoff

- **16-plane bar** (acceptance 1): the nine VDI/raster tables, `aes-r1`, `bios-r1` at the default and `--bootargs
  vdiplanes=16`, branch against `main`, as above. Expected: identical verdict lines and reports; VDI/AES/BIOS payloads
  identical (except `Random()`, case 112 of `bios-r1`, run to run), raster payloads identical but for the MFDB pointer
  cells.
- **No false contract failure**: every existing table at `--bootargs vdiplanes=32` ends its contract check with exit 3
  "not checked", except the three contract tables (exit 0). The failing command of F1 (`vdi-v18.tsv`) included.
- **32-plane tables** (acceptance 1, "labelled NewTOS contract"):
  `cargo xtask tosdump rast tosdump/cases/vdi-raster32.tsv --out <dir> --bootargs vdiplanes=32`, the same for
  `vdi-raster-screen32.tsv`, and `cargo xtask tosdump vdi tosdump/cases/vdi-a131-palette.tsv --out <dir> --bootargs
  vdiplanes=32`. Expected: `RASTER32.BIN` (area 7, 64 x 5744) / `VDI.BIN`; `tosdump-run.txt` with the `setting` and
  `table ... NewTOS contract` lines and `contract check ... exit 0, 0 of 64 (48) cases differ`; `RASTER32.LOG` with
  `table declares a 32-plane screen: NewTOS contract (ADR-038), no oracle`.
- **The tool refuses**: a wide table without `--bootargs vdiplanes=32`, or with an `EMUTOS-` reference, ends with "nothing
  run"; any other table at 32 marks its oracle comparisons INFORMATIONAL.
- **icprobe**: `tosdump/probes/run-icprobe.sh newtos <out> <kernel8.img> <native.rsc> <ntree>` once plain and once with
  `ICPROBE_BOOTARGS=vdiplanes=32`; `icprobe-compare.py <out16> <out32>` expected "differing 0"; the plain run's
  `ICPROBE.BIN` byte-identical to `main`'s probe on the same resource.
- Not real-Pi-only: everything runs in QEMU raspi4b. No baseline is frozen (`tosdump/reference/NEWTOS32-*`): freezing one
  after the contract check passes is a choice for the tester or the lead (the gate says "only after that script passes").

## Proposed `docs/STATUS.md` line

`2026-11-18 — V69 part A: tosdump at 32 planes. A raster table that declares !screen w,h,32 is wide: area 7 VDI-RASTER32
(5744-byte record, 1024-WORD buffers, both v_get_pixel words in the readback); two 32-plane tables (vdi-raster32,
vdi-raster-screen32) and vdi-a131-palette pass the NewTOS-contract check (tosdump/host/contract32-check.py, 0 differing
cases); cargo xtask tosdump refuses an oracle reference for a wide table and marks every other oracle comparison at
vdiplanes=32 informational; icprobe has its 32-plane variant. The 16-plane tables, their record and the frozen references
are unchanged (verdicts and reports identical). Part B (the programs) waits for the owner. (<commit>)`

## Out-of-scope observations

- `docs/DEVIATIONS.md` "VDI at 32 planes", first row, and the test-file list in `docs/COMPATIBILITY.md` (the two stale
  sentences of the V68 test run named in the ticket) were already corrected at the base `06ef14f5b`; nothing left to do.
- `docs/TOS-REFERENCE-DUMPER.md:367` and `:427` cite gitignored `sdk/gemlib` sources a worktree does not link
  (`cargo xtask worktree-setup` links `references/*` and `sdk/apps/*` only): fail in every worktree, pass in the main tree.
- `tosdump/probes/vdprobe.c` and `wkprobe.c` keep their 16-plane `grab()` (as the gate says) and their `wbuf[2048]`.
- `cargo xtask doc-check --fix-anchors` (TASK/TOOL49) is not in this tree's xtask ("unknown argument"); anchors were moved
  by hand from doc-check's "nearest occurrence".

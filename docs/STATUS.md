# STATUS — NewTOS

> Single source of truth for project state. Updated with every milestone.
> Edit only via PR; do not edit in random places. Date everything.

## Current state

### As of 2026-10-26

Branch `main`, latest release tag `v0.7.2` (NewTOS 0.7.2 Alpha, `d35670d6`, published 2026-11-08); before that `v0.7.1` (`6baafc92`, 2026-11-07), `v0.7` (NewTOS 0.7 Alpha, `d7711c42`, published 2026-11-04); before that `v0.6.1` (NewTOS 0.6.1 Alpha, `d5e9424c`,
2026-10-22; kernel accepted on a real Pi 4, `docs/HW-TEST-LOG.md`).
Proposed target still `base-1`
(`docs/analysis/inventory-2026-10-08.md` §3, definition open).
History up to today: [docs/archive/status-log-2026-10-26.md](archive/status-log-2026-10-26.md)
(history, not evidence).

**Desktop:** ADR-022 — the desktop is built **1:1 against EmuTOS**, through the
AES trap, without workarounds. Since DESK43 (done 2026-10-16, U10 `f6f90202`,
Tester PASS `87436d52`, ADR-026) it is the library `programs/desktop/`
(only `newtos-gem` + `tos-types`, a workaround = compiler error) and boots as an
EL0 program NEWDESK; the kernel no longer links a desktop, a load failure
falls back to EmuCON. Rust programs for NewTOS: `programs/`
(runtime `newtos-gem`, build `cargo xtask build-programs`).

**Accepted on a real Pi 4:** boot, desktop, TeraDesk (start and clean
exit), QED from within TeraDesk, EmuCON, **USB mouse including mouse wheel**,
file copying across partition boundaries (FS04), Doom (40–75 fps, `kbdvec`
KEYS02), key repeat, `.NTP` parameter dialog (DESK47), the four
native extensions from TeraDesk (DESK41), window opening (DESK50), zView;
the demo card of 2026-10-08 with memory protection on (ARCH03 up to the
follow-up work after E9). Since 2026-10-22, according to `docs/HW-TEST-LOG.md`, in addition: GH1
and HypView, the release kernel 0.6.1, USB01 F1/F2/F8 and CON04 (VT52 in
`Bconout(2)`).

**ARCH03 is implemented and accepted** (ADR-028, VA = PA; Tester on `main`
2026-10-13, memprot on/off/strict + `asid=on`,
`measurements/measurement-2026-10-13-arch03-acceptance/`). Still open from it according to
[ARCH03](../TASK/ARCH03.md): stage 2b (`Pexec` 100+), `asid=on` on a real Pi.

**Baselines** (to be held at every work step):

| | |
|---|---|
| Host tests (`cargo xtask test`) | **2436 / 0 / 7** (measured 2026-11-11 with TOOL54, Tester, +9 `tickets::migrate`; before that 2427 / 0 / 7 with TOOL52, Tester, +11 `repo_guard`; before that 2416 / 0 / 7 with TOOL50, Tester, +1 `pin_guard_pipe_tests_pass`; before that 2415 / 0 / 7 on `6d7cb3456` with TOOL47, Lead, +7 `test_inputs`, one of them the ignored child of two process tests; before that 2408 / 0 / 6 with TOOL46/TOOL51, Tester, +8 `scratch`, `tickets` and hook pipe tests; before that 2400 on 2026-11-11 with TOOL44, Tester, +4 `worktree_setup`; before that 2396 on 2026-11-11 with TOOL41, Tester, +20 `tickets::tests`; before that 2376 on 2026-11-11 with TOOL42, Developer, +1 `no_verify_guard_pipe_tests_pass`; before that 2375 on 2026-11-11 with TOOL40, Tester, +1 `staged_reports_only_what_the_commit_introduces`; before that 2374 on 2026-11-11 with TOOL39 `b7bd932a`, Tester, +5 `gen_deps_tests`; before that 2369 on 2026-11-08 with V55 `b744febc`, Tester, +6 V55, +1 addendum; before that 2362 on 2026-11-08 with V54 `2869dcfc`, Tester, +4 V54; before that 2358 on 2026-11-08 on the merge of V58/V59 `64bf181e`, Lead, +11 V58, +8 V59; before that 2339 on 2026-11-08 on the merge of V50/V51 `20c23349`, Lead, +11 V50, +10 V51; before that 2318 on 2026-11-08 on `5be660c2` with V56 and V40, Tester, +2 V56, +14 V40; before that 2302 on 2026-11-08 on `c013f4b2` with V53 and V38, Lead, +6 V53, +5 V38; before that 2291 on 2026-11-07 with V37, Developer and Tester, +11, 4 ignored with reason V41; before that 2280 on 2026-11-07 on the merge of V49 `2795e432`, Lead, +7 `raster_kernel_fill`; before that 2273 on 2026-11-07 with V35, Developer and Tester, +12 tests, 1 ignored with reason (TPH, V52); before that 2261 on 2026-11-07 with V34, Developer and Tester, +3 tests; before that 2258 on 2026-11-07 with V31, Developer and Tester, +17 tests; before that 2241 on 2026-11-07 with A141, Developer and Tester, +2 `rsc_recolour_tests`; before that 2239 on 2026-11-06 with HW08 fix, Developer, +22 tests; before that 2217 on 2026-11-06 with FS31, Developer, +17 tests; before that 2200 on 2026-11-06 with HW08 items 1+2, Developer, +5 tests; before that 2195 on 2026-11-06 with V44, Developer, +5 tests; before that 2190 on 2026-11-06 with V33, Developer, +30 tests; before that 2160 on 2026-11-06 with V30, Developer, +40 tests; before that 2120 on 2026-11-06 with V32, Developer, +4 tests; before that 2116 on 2026-11-06 with V29, Developer, +20 tests; before that 2096 on 2026-11-06 with V28, Developer, +30 tests; before that 2066 on 2026-11-06 with V25 according to Recent Decisions; before that 2033 on 2026-11-05 with V20, Developer, +20 tests; before that 2013 on 2026-11-05 with V26, Developer, +9 tests; before that 2004 on 2026-11-05 with V19, Tester; before that 1974 on 2026-11-04 with PROC08 screen report, +6 tests; before that 1929 with PROC08 `crash-walk`, +7 tests; before that 1922 with XBIOS12, +4 tests; before that 1918 with FS21, +8 tests; before that 1910 with MEM11, +3 tests; before that 1907 with FS28, +2 tests; before that 1905 with TOOL29, +6 xtask tests; before that 1899 with A126, +4 tests; before that 1895 with SDK08, no new test; before that 1895 with COOK02 `_IDT`, no new test; before that 1895 with XBIOS04, +1 test; before that 1894 with BIOS09, +1 test; before that 1893 with BIOS17, +1 test; before that 1892 with BIOS16, +4 tests; before that 1888 with BIOS08, +7 tests; before that 1881 with XBIOS05, +9 tests, base according to the Lead 1872; before that 1859 with BIOS07, +9 tests; before that 1850 with FS23, +3 tests; before that 1847 with BIOS04, +4 tests; before that 1843 with XBIOS02, +1 test; before that 1842 with BIOS06, +2 tests; before that 1840 with BIOS01 remainder, +7 tests; before that 1833 with TOOL19, +5 tests; before that 1828 with ARCH06, +4 tests; before that 1824 with XBIOS18, +1 test; before that 1823 with TOOL21, +2 tests; before that 1821 with TOOL17, +5 tests; before that 1816 with XBIOS15, +8 tests; before that 1808 with BIOS15, +18 tests; before that 1790 with FS22, +7 tests; before that 1783 with XBIOS01, +16 tests; before that 1767 with FS20, +7 tests; before that 1760 with BIOS01 S2, +9 tests; before that 1751 with TOOL07 remainder, +3 xtask tests; before that 1748 with TOOL12, +3 deps tests; before that 1745 with TOOL10; TOOL09 +16, TOOL10 +3 xtask tests; earlier 1726 / 0 / 1 on `75527af8`, `--fast` without the one doctest) |
| SDK host tests (`sdk/run-host-tests.sh`) | **38 / 0 / 0** (measured 2026-11-07 with A142, Developer, +2 `inline-rsh-host`, `hypview-prepare-host`; before that 36 on 2026-11-07 with A140, Developer, +1 `hypview-win-host`; before that 35 on 2026-10-30 with SDK08, no new test: rebuild parity of `rt-host`/`ldbl-host` only masks out `.comment`, host clang `clang-2100.4.1.1`; in between 33/2/0 after the clang update; before that 35 with XBIOS05, +1 `bconmap-layout`; before that 34 with SDK07, +1 `td-nohard-host`; before that 33 with SDK04, +3 `sysvars-layout`, `hypview-ref-host`, `zview-color-host`; before that 30 with SDK04 C6, +1 `td-cookie-host`; before that 29 with SDK05, +1 `cookie-host`; before that 28 with SDK03, +1 `sysinfo-host`; before that 27 with XBIOS01, +1 `iorec-layout`) |
| `sh tools/desk-drift-probe.sh` | **0** (unsafe 20; measured 2026-10-26) |
| APITEST (`#Z`, memprot on/off/strict) | **109 / 0 / 5** (last run `measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`) |

Build and Clippy warnings are listed here without a number: they were not
re-measured during this cleanup. They must not rise; each work step
gives its before/after number in its line under *Recent Decisions*.

**Open (as of 2026-10-26, from the status line of the tickets without a DONE receipt):**

- **Open with the user (as of 2026-11-08):** (a) done 2026-11-08: Pi run V58 + V59 passed, both in `main`
  (merges `b2ba4981`, `64bf181e`; `measurements/measurement-2026-11-08-v58-v59-pi/RUNINFO.TXT`). (b) [V55](../TASK/V55.md) decided 2026-11-08: (a), implemented, Tester PASS; Pi test with release kernel
  `b744febc` run 2026-11-08 (`measurements/measurement-2026-11-08-rel072-final/RUNINFO.TXT`), **done 2026-11-10** (receipt).
  (c) [V38](../TASK/V38.md) user 2026-11-08: limit higher, value only after a Pi measurement of the 5x3 endless case
  at 16 (Tester scenario). (d) [V41](../TASK/V41.md) D1–D4 decided 2026-11-08
  (D1 replicate, D2 clip cleanly, D3 tolerance, D4 include [V39](../TASK/V39.md)). (e) Push done 2026-11-08 (`cb33f3fc`). (f) [REL072](../TASK/REL072.md) 0.7.2 published 2026-11-08 (tag `v0.7.2`);
  open: inform the Testers (user), fetch the Fbench run of the Pi test from the card (Lead, card into the Mac).
- **Open with the user (as of 2026-11-07, after release 0.7.1):** (1) ticket for the
  47 AES candidates from [TOOL33](../TASK/TOOL33.md) (measurement on the oracle like
  [V18](../TASK/V18.md)), at the end of the queue: yes or no. (2) Names of the
  Fbench tests with an empty window, from them the comparison ticket (`FBENCH.PRG` in the
  oracle against QEMU per test; target list `docs/analysis/fbench-functions-2026-11-07.md`
  section 6). (3) Inform the Testers (Ragnar, Wolfgang) specifically about 0.7.1?
  (4) Connector test from
  REL071 only reported by the user as tested earlier, no evidence in the repo.
  Next product ticket in the queue: [PORT12](../TASK/PORT12.md) (ADR decision
  with the user, no kernel code before it). WFPROBE does not address logical partitions
  (reads the MBR entry drive minus C); no ticket without the user.

- **Queue (user 2026-10-29), in this order.** Rule: a
  new tool ticket goes behind the product fix it would delay,
  unless the fix cannot be accepted without the tool.
  **Pre-start check per ticket (user 2026-10-30):** product ticket —
  is a program on the card or a port planned in this repo
  affected? Tool ticket — does it block the acceptance of a product fix?
  Otherwise park it, with the reason here.
  **Parked 2026-10-31 (user) after this check:**
  [TOOL13](../TASK/TOOL13.md) (no upcoming fix brings a debug feature;
  rule so far proven by hand), [TOOL14](../TASK/TOOL14.md) (probes run
  without a trace mode), [TOOL15](../TASK/TOOL15.md) (real-time probes
  switch off fast-forward themselves; mtools version catches the sha256 check),
  [TOOL03](../TASK/TOOL03.md) (only needed for DUMPGDOS/FS19), DUMPGDOS (no
  ticket; GEMDOS cases run via probes and `gemdos-mc-*`),
  [FS19](../TASK/FS19.md) (only programs with a DTA in unmapped memory),
  [MEM11](../TASK/MEM11.md) (not reachable today; a host test keeps the
  memory layout below 2 GiB, so that a change fails loudly),
  [TOOL01](../TASK/TOOL01.md) (blocks no acceptance),
  [FS25](../TASK/FS25.md) (question only for the oracle).
  Also parked 2026-10-31 (P3 review):
  [BIOS13](../TASK/BIOS13.md) (pure doc/ledger maintenance; no program, no open fix),
  [BIOS14](../TASK/BIOS14.md) (tool gap; blocks no open fix),
  [XBIOS07](../TASK/XBIOS07.md) (doc correction without a user),
  [XBIOS11](../TASK/XBIOS11.md) (no card program depends on the seed origin (only user is TeraDesk formatting)),
  
  [XBIOS13](../TASK/XBIOS13.md) (pure spec maintenance),
  [XBIOS14](../TASK/XBIOS14.md) (oracle limit; blocks no fix),
  [XBIOS16](../TASK/XBIOS16.md) (no program calls Jdisint/Jenabint/Mfpint),
  [XBIOS19](../TASK/XBIOS19.md) (no program calls Bioskeys),
  [KEYS04](../TASK/KEYS04.md) (no card program polls Kbshift (only apitest)),
  [INPUT03](../TASK/INPUT03.md) (no program needs the mouse-keyboard emulation),
  [PROC07](../TASK/PROC07.md) (undocumented CP/M relic; gemlib sends only opcodes != 0),
  [FS27](../TASK/FS27.md) (only damaged media),
  [SPEC02](../TASK/SPEC02.md) (pure follow-up maintenance; open work does not read those places),
  [SPEC03](../TASK/SPEC03.md) (pure cleanup work; the hook does not block the legacy backlog),
  [TOOL22](../TASK/TOOL22.md) (table maintenance, depends on PROC07),
  [TOOL23](../TASK/TOOL23.md) (debug feature; resume when an AES fix needs the capture),
  [TOOL24](../TASK/TOOL24.md) (debug feature, blocks nothing),
  [TOOL25](../TASK/TOOL25.md) (associated fix (BIOS07) accepted via probe),
  [TOOL26](../TASK/TOOL26.md) (associated fix (BIOS16) accepted via probe),
  [TOOL28](../TASK/TOOL28.md) (manual rule in CLAUDE.md covers it),
  [TOOL30](../TASK/TOOL30.md) (associated fix (A126) accepted via probe),
  [SDK02](../TASK/SDK02.md) (no user of the constants),
  [SDK06](../TASK/SDK06.md) (no program reads a cookie with bit 31 set).
  [FS21](../TASK/FS21.md) (P2, archive bit on `Fclose`; implemented 2026-10-31, commit of this line). Waiting for the user: [BIOS12](../TASK/BIOS12.md) done 2026-10-31 (receipt). [XBIOS12](../TASK/XBIOS12.md) implemented 2026-10-31 (without an RTC the clock starts at the kernel's build date, 00:00:00, like EmuTOS `DEFAULT_DATETIME`, until RTC01; commit of this line). Parked: [TICK01](../TASK/TICK01.md) (until Testers see K6 from DOOM01).
  [PITEST01](../TASK/PITEST01.md) (P2, test card and procedure for the Testers on the real Pi; **done 2026-11-02**, receipt; HypView start page = setting in `HYP_VIEW.CFG`, zView window closing not reproducible by the user, no ticket): part 1 implemented 2026-10-31 (debug feature `crash-nowait`, `P0REP.NAP`, `HZPROBE.NAP`), part 2 implemented 2026-10-31 (card sync `tools/sync-card.sh`, run sheet `docs/analysis/tester-run-sheet-2026-10-31.md`), part 3 implemented 2026-10-31 (one card; backup of `kernel8.img`, `cmdline.txt`, `config.txt` and every replaced file before writing; HypView with `HYP_VIEW.CFG` and `ZVIEW.HYP`; `cmdline.txt` changes with the kernel via `--cmdline`, phase A without, phase B with `serial=on`; list and command `measurements/measurement-2026-10-31-pitest01-part3/RUNINFO.TXT`, PITEST01 "Stand Teil 3"); card filled and Pi runs A, B, C1 run 2026-10-31 (`measurements/measurement-2026-10-31-pitest01-phase-a/RUNINFO.TXT`, `measurements/measurement-2026-10-31-pitest01-phase-b/RUNINFO.TXT`, `measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`); findings: [DOOM01](../TASK/DOOM01.md), [PROC08](../TASK/PROC08.md) (new, P1), zView window closing (without ticket); card state in PITEST01 "Stand nach den Pi-Laeufen".
  [PROC08](../TASK/PROC08.md) (P1, new 2026-10-31, user: create ticket): return from `Pexec(4)` did not find the code page of the parent process (zView on the Pi, once); root-cause research first. **Next ticket (user 2026-11-02)**, before DOOM01. Root-cause research 2026-11-02 without evidence (`docs/analysis/proc08-root-cause-2026-11-02.md`); debug feature `crash-walk` built (`371c50fd`); Pi run 2026-11-04 with `serial=on memprot=verify` without recurrence (`measurements/measurement-2026-11-02-proc08-pi/RUNINFO.TXT`). **Waiting for recurrence** (user 2026-11-04): `crash-walk` now also writes its diagnosis into the crash report on the screen (commit of this line); the card runs this kernel in daily use with an empty `cmdline.txt`, on a crash the user takes a photo. Afterwards go through priorities together (user 2026-11-04), then continue in the queue.
  **[REL07](../TASK/REL07.md) release 0.7** (**published 2026-11-04**: tag `v0.7` on `d7711c42`, GitHub pre-release with `NewTOS-0.7.zip` sha256 `a5191394…`; Pi acceptance of the kernel 2026-11-04; Testers informed (#3, #8), #9 closed, REL07 done; package with `tools/release/package-release.sh`, `measurements/measurement-2026-11-04-rel07-final/RUNINFO.TXT`; step 1 version from one source `797e0078`; user 2026-11-04; after the PROC08 ladder, with `crash-walk`, acceptance on the user's Pi, then inform Ragnar). **PROC08 recurred 2026-11-04** (photo, `measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT`): screen report `model=valid table=valid at=fault` = candidate B; mechanism research `docs/analysis/proc08-candidate-b-2026-11-04.md` (window found, trigger not); **deferred "to be tested"** (user: 60 zView starts with the fix without a crash, before that several in a row); fix implemented 2026-11-04 (`01eb7709`), Tester PASS (`measurements/measurement-2026-11-04-proc08-fix-test/RUNINFO.TXT`), Pi acceptance open; ladder in the screen report implemented 2026-11-04 (commit of this line, `measurements/measurement-2026-11-04-proc08-ladder/RUNINFO.TXT`). **First (user 2026-11-05):** [V18](../TASK/V18.md) (measure the 26 VDI candidates from TOOL32 on the oracle, three parts, then fix tickets per finding; part 1 measured 2026-11-05 `2e22799b`: 9 findings, fix tickets [V19](../TASK/V19.md) (done 2026-11-05, `506643fb`, Tester PASS `measurements/measurement-2026-11-05-v19-test/RUNINFO.TXT`) → [V26](../TASK/V26.md) (side finding of V19, memory path reached into an unchecked row, P2; done 2026-11-05, `1c84f4d2`+`87f1684e`, user decision a: EmuTOS geometry, cut only behind `fd_h`; Tester PASS `measurements/measurement-2026-11-05-v26-test/RUNINFO.TXT`) → [V20](../TASK/V20.md) (`abline`, done 2026-11-05, `bec04bab`; user decision A: wrap-around only in the logical screen; Tester PASS `measurements/measurement-2026-11-05-v20-test/RUNINFO.TXT`, AES/desktop scenes pixel-identical; Pi: XOR frame at the screen edge into the test scenario) → [V21](../TASK/V21.md) (done 2026-11-06, `497773f9`, seed queue like EmuTOS; Tester PASS with overflow case `measurements/measurement-2026-11-06-v21-test/RUNINFO.TXT`; emergency brake open with the user) → [V22](../TASK/V22.md) (done 2026-11-06, `7108bdff`, Tester PASS `measurements/measurement-2026-11-06-v22-test/RUNINFO.TXT`) → [V23](../TASK/V23.md) (done 2026-11-06, `ee67903f`, clamping centrally in `dispatch_in`; Tester PASS `measurements/measurement-2026-11-06-v23-test/RUNINFO.TXT`; follow-up ticket [TOOL34](../TASK/TOOL34.md) parked) → [V24](../TASK/V24.md) (done 2026-11-06, `caf77fab`, user decision B; Tester PASS `measurements/measurement-2026-11-06-v24-test/RUNINFO.TXT`) → [V25](../TASK/V25.md) (done 2026-11-06, `294272e2`, early exit of `curve`; Tester PASS `measurements/measurement-2026-11-06-v25-test/RUNINFO.TXT`; part 1 thereby oracle-identical except for decision C) → V18 part 2 measured 2026-11-06 `eba4122a`: [V28](../TASK/V28.md) (done 2026-11-06, `f1006588`, Tester PASS `measurements/measurement-2026-11-06-v28-test/RUNINFO.TXT`; thereby also completes [V36](../TASK/V36.md)) → [V29](../TASK/V29.md) (done 2026-11-06, `b4e419e5`, Tester PASS `measurements/measurement-2026-11-06-v29-test/RUNINFO.TXT`) → [V30](../TASK/V30.md) (done 2026-11-06, `858e5f68`, Tester PASS `measurements/measurement-2026-11-06-v30-test/RUNINFO.TXT`; AES images byte-identical; Pi: wrapping text over the cache-clean path into the test scenario) → [V33](../TASK/V33.md) (done 2026-11-06, `877a14b9`, Tester PASS `measurements/measurement-2026-11-06-v33-test/RUNINFO.TXT`; follow-up [V44](../TASK/V44.md); before the next card relink QED, TeraDesk, HypView, zView; Pi: text into the test scenario) → [V44](../TASK/V44.md) (done 2026-11-06, `6e96e91c`, Tester PASS `measurements/measurement-2026-11-06-v44-test/RUNINFO.TXT`; Pi: Fbench on the new kernel against `e56021e7`) → [HW08](../TASK/HW08.md) (P1, new 2026-11-06, user: before all further text tickets: saving fails sporadically until restart, at times slow, user hypothesis: card timeout; includes HW05, HW06; items 1+2 done 2026-11-06, `e421bd5d`, `measurements/measurement-2026-11-06-hw08/RUNINFO.TXT`; Pi run open, draft `measurements/measurement-2026-11-06-hw08/PI-RUN.TXT`; fix afterwards, user decision A/B in the ticket; **done 2026-11-07**, receipt; old card dropped, user: defective) → [FS31](../TASK/FS31.md) (done 2026-11-06, `20f87d1d`, Tester PASS `measurements/measurement-2026-11-06-fs31-test/RUNINFO.TXT`; deleting on one drive hit open files on another, included in release 0.7; warning to the Testers open with the user; follow-up [FS32](../TASK/FS32.md), parked) → for release [REL071](../TASK/REL071.md): [A140](../TASK/A140.md) (#22; implemented 2026-11-07, patch `sdk/overlays/hypview/dl_win.c`, `measurements/measurement-2026-11-07-a140/RUNINFO.TXT`; Tester and Pi open), [A141](../TASK/A141.md) (#23; measured 2026-11-07: EmuTOS-identical, cause in the resource, decision A/B1/B2/C with the user), [A142](../TASK/A142.md) (#21; byte sequence in cflib's images, different root, open), package and Pi acceptance (user 2026-11-06; package 0.7.1 built 2026-11-07, `measurements/measurement-2026-11-07-rel071-package/RUNINFO.TXT`, Pi acceptance 2026-11-07 passed `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`; **published 2026-11-07**: tag `v0.7.1` on `6baafc92`, GitHub pre-release, #12/#21/#22/#23 answered and closed) → [PORT12](../TASK/PORT12.md) (XGDOS, new 2026-11-06, user: schedule earlier; stage 1 done 2026-11-06 `measurements/measurement-2026-11-06-port12/RUNINFO.TXT`, ADR decision with the user, no kernel code before it) → [V31](../TASK/V31.md) (done 2026-11-07, `17bcaf56`, Tester PASS `measurements/measurement-2026-11-07-v31-test/RUNINFO.TXT`; ZG012 from V45 oracle-identical along the way) → [V32](../TASK/V32.md) (done 2026-11-06, `937d6361`, Tester PASS `measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT`; follow-up [DESK71](../TASK/DESK71.md)) → [V34](../TASK/V34.md) (done 2026-11-07, `5f299bf8`, Tester PASS `measurements/measurement-2026-11-07-v34-test/RUNINFO.TXT`; follow-up [TOOL35](../TASK/TOOL35.md), parked) → [V35](../TASK/V35.md) (done 2026-11-07, `788a8aa9`, Tester PASS `measurements/measurement-2026-11-07-v35-test/RUNINFO.TXT`; V18 part 2 thereby oracle-identical except for V52) → **inserted (user 2026-11-07):** [V49](../TASK/V49.md) (done 2026-11-07, merge `2795e432`, Tester PASS, Pi Fbench `measurements/measurement-2026-11-07-v49-pi/RUNINFO.TXT`) → [V50](../TASK/V50.md) → [V51](../TASK/V51.md) (both done 2026-11-08, merge `20c23349`, Tester PASS, Pi `measurements/measurement-2026-11-08-v50-v51-pi/RUNINFO.TXT`) (raster kernels in NEON under ADR-036, comparison MagiC/fVDI under ADR-035; second worker in its own worktree alongside the V series) → [V58](../TASK/V58.md) (lines) and [V59](../TASK/V59.md) (mouse pointer), user 2026-11-08: finish the NEON series; both done 2026-11-08, merges `b2ba4981`, `64bf181e`, Tester PASS `3711dd00`, `2d58115d`, Pi `measurements/measurement-2026-11-08-v58-v59-pi/RUNINFO.TXT` → [V37](../TASK/V37.md) (done 2026-11-07, `2bf28d71`, Tester PASS `measurements/measurement-2026-11-07-v37-test/RUNINFO.TXT`) → [V38](../TASK/V38.md) (done 2026-11-08, `e6ffdedc`, Tester PASS; EmuTOS hangs in the 5x3 case on the oracle, NewTOS brakes) → V18 part 3 measured 2026-11-07 (`020bfd16`, `docs/audits/2026-11-07-v18-t3-vdi-workstation.md`): [V53](../TASK/V53.md) (`v_opnwk`/`v_clswk` mouse; done 2026-11-08, `4cc7005c`, Tester PASS; follow-up [V56](../TASK/V56.md) P2, folded vector sentinels, done 2026-11-08 `559614ed`, Tester PASS) → [V54](../TASK/V54.md) (`vsc_form`; done 2026-11-08, `721e0441`, Tester PASS `2869dcfc`; side findings [A147](../TASK/A147.md), [TOOL38](../TASK/TOOL38.md)) → [V55](../TASK/V55.md) (P4, pointer at frame rate; user decision (a) 2026-11-08; implemented `9c72d3ec` + addendum `8ef6cf96`, Tester PASS `8f19602e`, `b744febc`; Pi 2026-11-08 ok; done 2026-11-10) → [SPEC04](../TASK/SPEC04.md) (P4, docs; done 2026-11-08, `e197b9cd`; follow-up [TOOL37](../TASK/TOOL37.md) graph, parked, [V57](../TASK/V57.md) P4, parked) → [V27](../TASK/V27.md) (P4), user decisions A/B/C in V18; part 2/3 afterwards). **Order after the priorities round (user 2026-11-04):** [A127](../TASK/A127.md) (root-cause research 2026-11-04 without reproduction, `docs/analysis/a127-root-cause-2026-11-04.md`; D1 implemented 2026-11-04, `bae52dbd`, Tester PASS `measurements/measurement-2026-11-04-a127-d1-test/RUNINFO.TXT`; consequence: click selection in the menu takes effect after the double-click window, measured on the oracle as EmuTOS-identical 2026-11-05 `measurements/measurement-2026-11-04-a127-oracle/RUNINFO.TXT`; user's Pi 2026-11-05 ok (`measurements/measurement-2026-11-05-a127-d1-card/RUNINFO.TXT`); D2/D3 open until Ragnar's feedback) → [A128](../TASK/A128.md) (file selector: slider like EmuTOS, wheel as an addition; GitHub #9; implemented 2026-11-04, `df2c7501`; Tester PASS 2026-11-04, `measurements/measurement-2026-11-04-a128-test/RUNINFO.TXT`; open: real Pi in the test scenario; follow-up ticket [TOOL31](../TASK/TOOL31.md), P3, parked) → [A129](../TASK/A129.md) (done 2026-11-04: pixels EmuTOS-identical, no defect) → [A132](../TASK/A132.md) (implemented 2026-11-04, `be3c06a9`, Tester PASS; mask in the file selector title) → [A133](../TASK/A133.md) (implemented 2026-11-04, `fd6c7e3f`, Tester PASS together with A132 `measurements/measurement-2026-11-04-a133-test/RUNINFO.TXT`; TEDINFO text in the write mode of the colour word) → [A135](../TASK/A135.md) (new, P3: `G_BOXCHAR` likewise) → [A136](../TASK/A136.md) (new, P3: gaps of the drop-down box like EmuTOS) → [A134](../TASK/A134.md) (new, P3: drive click/error branch of the file selector) → [USB03](../TASK/USB03.md) (parked 2026-11-05 until Ragnar's feedback on #19 and on the USB state; user: partly builds on each other) → [ARCH03](../TASK/ARCH03.md) remainder (proposal 2026-11-05, not confirmed: stage 2b after 1.0, `asid=on` into the test scenario) → [TOOL32](../TASK/TOOL32.md) (VDI mapping in `mapping.tsv`, done 2026-11-05; 26 deviation candidates in the ticket, unmeasured) → [TOOL33](../TASK/TOOL33.md) (AES mapping, done 2026-11-05; 47 candidates in the ticket, unmeasured); then EmuTOS conformance (A119–A125, DESK40/49/51–55/61/63/67–69, FS12, FS15–FS17, INPUT02, RST02) → sound for 1.0 (SND01, SND02, H04). Alongside: the acceptances open only on the Pi (A16, A37, DESK34, DESK35, HW03, R02–R05, SER01, V11, V12, DESK43, BIOS09, FS26 T1–T3, >15 USB commands) as one test scenario for the Testers; go through the „ERLEDIGT“ tickets without a receipt once, in a targeted way. Expansion after 1.0 and tool tickets stay on hold. From the issue review 2026-11-04 to the end: [A130](../TASK/A130.md) (#2 window scroll bar, P2; measured on the oracle 2026-11-05, reproduced, findings 001–004 in `docs/audits/2026-11-05-a130-slider.md`; done 2026-11-05: fix `d9e44143`, Tester PASS `measurements/measurement-2026-11-05-a130-test/RUNINFO.TXT`, user's Pi ok `measurements/measurement-2026-11-05-a130-card/RUNINFO.TXT`), [A131](../TASK/A131.md) (#7 window shadow, P3; measured on the oracle 2026-11-05: shadow light grey instead of the `W_BOX` border colour; user decision 2026-11-05 like EmuTOS, overrides that of 2026-09-10; implemented 2026-11-05 `837e1001`, Tester PASS `measurements/measurement-2026-11-05-a131-test/RUNINFO.TXT`; done 2026-11-05: user's Pi ok `measurements/measurement-2026-11-05-a131-card/RUNINFO.TXT`), [DESK70](../TASK/DESK70.md) (#12 text program output, P3; measured on the oracle 2026-11-07: the EmuTOS desktop does not wait either, desktop stays `7d700cf3`; CoreMark waits itself, always, implemented 2026-11-07 `729bab3b`, second build after user decision `measurements/measurement-2026-11-07-desk70-build2/RUNINFO.TXT`; Tester PASS `measurements/measurement-2026-11-07-desk70-test2/RUNINFO.TXT`, Pi ok 2026-11-07, #12 answered and closed; done). From the V30 side findings to the end (2026-11-06): measured [V39](../TASK/V39.md) (`F_LIGHT`), [V40](../TASK/V40.md) (`LSTLIN`; done 2026-11-08, `5be660c2`, Tester PASS), [V41](../TASK/V41.md) (cause proven 2026-11-06, `de1d12dd`, nine places; no app draws effects in replace mode, fix to the end, user), [V45](../TASK/V45.md) (from V41), [FS33](../TASK/FS33.md) (from HW08, second handle), [FS34](../TASK/FS34.md) (from HW08, rejected data sector), [V42](../TASK/V42.md) (P4, 4096+ characters), [A143](../TASK/A143.md), [A144](../TASK/A144.md); from the Fbench report `docs/analysis/fbench-functions-2026-11-07.md` [V46](../TASK/V46.md), [V47](../TASK/V47.md), [A145](../TASK/A145.md), [V48](../TASK/V48.md) (P4); from V34 [KEYS05](../TASK/KEYS05.md) (P4, Alt+Help `dumpflg`); from V35 parked [V52](../TASK/V52.md) (P4, handlers write into the `PTSIN` copy; until the m68k thunk), collective note m68k emulator with Line-A [EMU01](../TASK/EMU01.md) (IDEE, after 1.0), [TOOL36](../TASK/TOOL36.md) (parked); known deviations by trigger: `docs/DEVIATIONS.md` (new 2026-11-07); only read [V43](../TASK/V43.md) (P4), [A137](../TASK/A137.md), [A138](../TASK/A138.md), [A139](../TASK/A139.md), measure first. From the Pi acceptance 0.7.1 (2026-11-07) parked: [A146](../TASK/A146.md) (question, research: distance between window frame and shadow). From V54 (2026-11-08): [A147](../TASK/A147.md) (`graf_mouse(USER_DEF)` colours and hotspot, read, measure first), [TOOL38](../TASK/TOOL38.md) (parked: wkprobe wait limit; runs acceptable by repetition). From the Pi test V54/V55 (2026-11-08): [A148](../TASK/A148.md) (P4, parked: pattern of the slider track flashes briefly only with very fast wheel scrolling, presumably always so).
  Developer, one at a time:
  the S2 audit tickets [XBIOS02](../TASK/XBIOS02.md) (done 2026-10-29, `7bc047a6`),
  [BIOS04](../TASK/BIOS04.md) (done 2026-10-29, commit of the receipt) →
  [FS23](../TASK/FS23.md) (done 2026-10-29, commit of the receipt), [FS24](../TASK/FS24.md) (ACCEPTED under ADR-033, user 2026-10-30) →
  the remaining audit tickets [BIOS07](../TASK/BIOS07.md) (done 2026-10-29, commit of the receipt),
  BIOS08–BIOS14 ([BIOS10](../TASK/BIOS10.md) done 2026-10-29, commit of the receipt; [BIOS08](../TASK/BIOS08.md) done 2026-10-30, commit of the receipt), XBIOS04–XBIOS14 ([XBIOS05](../TASK/XBIOS05.md) done 2026-10-29, commit of the receipt; BIOS05 and XBIOS03
  are ACCEPTED under ADR-031, XBIOS10 under ARCH05) →
  [FS26](../TASK/FS26.md) (creating a folder ~3 s on the Pi; layer measurement in
  QEMU first, Pi measurement into the test scenario; user 2026-10-31) →
  [MEM11](../TASK/MEM11.md) host test (memory layout < 2 GiB; done 2026-10-31, commit of this line) → [FS29](../TASK/FS29.md) (P3, parked 2026-10-31: needs a `Rwabs` sector-0 tool), [FS30](../TASK/FS30.md) (P3, parked: `..` time, no program reads it),
  [PROC07](../TASK/PROC07.md), [COOK02](../TASK/COOK02.md) (parked; item 3 `_IDT` = oracle implemented 2026-10-30, commit of the receipt),
  [TOOL22](../TASK/TOOL22.md), [XBIOS19](../TASK/XBIOS19.md), [TOOL23](../TASK/TOOL23.md),
  [SPEC02](../TASK/SPEC02.md), [TOOL24](../TASK/TOOL24.md), [FS27](../TASK/FS27.md), [TOOL25](../TASK/TOOL25.md) (P3); [BIOS16](../TASK/BIOS16.md) (done 2026-10-30, `9c4c29c5`) → [BIOS17](../TASK/BIOS17.md) (done 2026-10-30, commit of the receipt) → [BIOS09](../TASK/BIOS09.md) (implemented 2026-10-30, commit of the receipt; open the Pi step in the MEM12 check sequence, item 10) → [XBIOS04](../TASK/XBIOS04.md) (implemented 2026-10-30, commit of the receipt) → [SDK08](../TASK/SDK08.md) (done 2026-10-30, commit of the receipt) → [A126](../TASK/A126.md) (implemented 2026-10-30, commit of this line) → [TOOL29](../TASK/TOOL29.md) (implemented 2026-10-30, commit of this line; hook installation by the Lead) → [FS28](../TASK/FS28.md) (user 2026-10-30; parked until a port needs them, because no program on the card is affected: [XBIOS06](../TASK/XBIOS06.md) fix, [XBIOS08](../TASK/XBIOS08.md), [XBIOS09](../TASK/XBIOS09.md), [XBIOS10](../TASK/XBIOS10.md)) → [FS28](../TASK/FS28.md) (done 2026-10-30, commit of this line), [A126](../TASK/A126.md); [TOOL26](../TASK/TOOL26.md) (P3); [FS28](../TASK/FS28.md), [A126](../TASK/A126.md) (P2), [KEYS04](../TASK/KEYS04.md), [INPUT03](../TASK/INPUT03.md) (P3); [SDK02](../TASK/SDK02.md), [SDK06](../TASK/SDK06.md)
  (P3) when there is an opportunity; [SDK08](../TASK/SDK08.md) (P2, SDK parity after clang update; done 2026-10-30);
  parked until MIDI/IO01: [UART01](../TASK/UART01.md); [INPUT04](../TASK/INPUT04.md) (P3, 0xF6 packet length,
  dormant; before any IKBD interpreter); [TOOL27](../TASK/TOOL27.md) (P3,
  rsprobe to tosdiff, parked); [TOOL28](../TASK/TOOL28.md) (P3, host_clang into RUNINFO); [TOOL30](../TASK/TOOL30.md) (P3, akprobe to tosdiff). Deferred (user 2026-10-29):
  [DOOM01](../TASK/DOOM01.md) (DOOM freeze in the menu, only on the Pi) — deferred again 2026-11-02 (user: PROC08 first); diagnosis state 2026-10-31: freeze only with the accessory X-Shot (waits without a timer for `MU_KEYBD`) and only without `serial=on`; cause in the kernel, open; next step in DOOM01 "Stand 2026-11-02".
  Alongside one Researcher at a time, without product change: — (free).
- **P1:** [MEM12](../TASK/MEM12.md) — done 2026-10-30 (user: Pi run 2026-10-29, kernel `58e35ac8`); `P0PAR` is the first step in [DOOM01](../TASK/DOOM01.md) and in the later test scenario.
  [USB01](../TASK/USB01.md) — done 2026-11-04 (receipt; F1/F2/F3/F8 and
  USBSNAP confirmed on real hardware). Remainder: [USB03](../TASK/USB03.md)
  (report-only mice, wheel of the B100, key 3; P1 since 2026-11-04),
  [A127](../TASK/A127.md) (new: menu entries only take effect after several
  clicks, highlight stays black; Ragnar in #3; cause first), more than
  15 commands on the VL805 into the test scenario, [USB04](../TASK/USB04.md) (P3).
  [ARCH03](../TASK/ARCH03.md) remainder (see above). [A45](../TASK/A45.md) is
  blocked (see *Blocked*).
- **P1, implemented, only Pi or application acceptance open** (according to the status line):
  A16, A37 (application side), DESK34, DESK35, HW03 (three Pi counter-checks),
  R02–R05, SER01, V11, V12, DESK43 (Pi counter-check of the EmuCON fallback on
  defective NEWDESK).
- **P2:** [MEM11](../TASK/MEM11.md) (user addresses only by chance < 2 GiB;
  debug check on the return path active, enforcement parked; since 2026-10-31 the layout below 2 GiB is held by build assertions of both boards and `tos-api/tests/mem11_layout_below_2gib.rs`; tosdiff check waits for tosdump format v3), [SDK01](../TASK/SDK01.md) (A–L done or implemented, G dropped;
  open only the acceptance of K under Linux/Homebrew),
  [SND01](../TASK/SND01.md), [SND02](../TASK/SND02.md),
  [NET01](../TASK/NET01.md) (preliminary research, target 1.2), [HW04](../TASK/HW04.md)
  (partial), [W01](../TASK/W01.md), [TOOL03](../TASK/TOOL03.md) (reproducible measurement images and dumper builds, before the DUMPGDOS image comparison), [TOOL05](../TASK/TOOL05.md) (doc-check also for docs that cite a changed file); parked: [TOOL01](../TASK/TOOL01.md) (`cargo xtask doctor`), [H04](../TASK/H04.md),
  [H01](../TASK/H01.md) (waits for hardware); [HW05](../TASK/HW05.md) (finding done, acceptance open: the same `QED.NAP` copy with a stopwatch as before, into the test scenario; [HW08](../TASK/HW08.md) done 2026-11-07); deferred:
  [T13](../TASK/T13.md).
- **„ERLEDIGT“ without a DONE receipt — count as open until measured
  (phase 5 triage; user 2026-10-26):** A06, A08, A09, A26, A116, ACC07,
  CON01, CON02, DESK39, DESK45, DESK47, FS02, FS07, FS14, FS18, GH1,
  LIBC-BUF, MEM01, MEM03, MEM04, MEM07, PROC01, REF04, SPEC01, V01, V02,
  V04–V10, V13–V15 (36); plus seven with only a partial or collective receipt:
  DESK43 (receipts up to U9-3-5), FS08 (per sub-step), MEM05 (A and B),
  R02–R05 (`R02-R05-STATUS-DONE.MD`). List made by script from the status line
  and the receipt files in `TASK/`, as of 2026-10-26.
- **Scope 1.0 (user 2026-10-23):** FAT16 remains the only file system
  (FAT32/exFAT after 1.0); **sound is part of it** — SND01 (Falcon DMA sound),
  SND02 (HDMI), H04 (bell, key click, `Dosound`), all P2.
- **P3 / P4:** 57 tickets P3 and 4 tickets P4 with status OFFEN, plus 80 P3 and
  1 P4 in BACKLOG, 3 TEILWEISE, 3 IDEE. Among them: ARCH03 stage 2b, SDL01,
  RTC01, REF03, REF05 (GIC sources), REF06 (missing Track B sources), T15,
  DESK49, DESK51–DESK55, DESK61, DESK63, DESK65, DESK66, DESK67, DESK69, FS12
  (26 drives), FS15, FS16, FS17, LIBC-IOCTL, A119–A125, RST02, INPUT02,
  CON03, ARCH04, USB02, [USB04](../TASK/USB04.md), MEM08, MEM10, [KEYS03](../TASK/KEYS03.md),
  [T18](../TASK/T18.md), [FS19](../TASK/FS19.md) (`Fsfirst` order, measure first); deferred COOK01; ideas MEM06,
  IO01.
- **Gap without a ticket — `shel_find` stage 4 (`PATH=`):** of EmuTOS'
  four-stage `findfile` order (`gemshlib.c:406-472`) stage 1
  (app directory), 2 (path as passed) and 3 (root of the current
  drive) are implemented (`tos-api/src/aes/shell.rs:1077-1130@sh_find`). Stage 4 is not
  built (`tos-api/src/aes/shell.rs:1131@PATH`), but no longer blocked: the
  AES environment with `PATH=` exists since PPARENT (`tos-api/src/aes/shell.rs:1043-1045@PPARENT`).
  The existence probe `read_file` is fragile compared with an `Fsfirst` probe
  (misses directory hits, ignores wildcards).
- **Wish without a ticket — multi-bank RAM:** `dtb.rs::parse_memory` reads only the
  first `/memory@…` bank (`kernel/src/arch/aarch64/dtb.rs:29-32@DtbMemory`); on the Pi
  that is 768 MiB TT-RAM, the remaining ~3 GiB stay unused. Not a
  blocker (768 MiB is expressly enough); wanted: all `reg` entries,
  discontiguous TT-RAM, page tables for the high region.

**Serial output, since 2026-10-02 (SER01):** on the Pi by default
**off**; `serial=on` in `cmdline.txt` switches it on. QEMU raspi4b runs
need `-append serial=on`, old measurement scripts the shim `tools/qemu-serial-on`.

**Rule for the SD card, since 2026-09-30:** `tools/make-sd-image.sh` builds the
**QEMU image**, **not** the real card. The physical card is
hand-maintained; **only** what the user expressly names goes onto it.

**QEMU runs:** QEMU sockets relative in the run directory (user request
2026-10-16). Long QEMU series only on mains power (battery sleep distorts
timings); screen dumps at log marks, one QEMU at a time. Scratch rule: CLAUDE.md
(*Placement rules*).

**Way of working:** one task per agent; since 2026-11-07 (user) at most two workers at the same time, with separate tasks and files, aware of each other. Since 2026-11-11 (owner, after TOOL52) tooling tickets run on ONE worker slot only, one after another (TOOL54, TOOL48, TOOL49, TOOL53, TOOL43, TOOL45); the other slot is development, starting with the legacy ticket clean-up (LEGACY01). The Lead does not
build himself; he checks afterwards, commits and writes here. Workers touch
neither card nor Pi and commit only when the Lead demands it.

---

## Phase Status

> Phase plan since 2026-05-03 by the EmuTOS modules (`bios/` / `bdos/` /
> `vdi/` / `aes/` / `desk/`); the old numbering is in
> `phases/_archive/`. An m68k emulator is not a phase of this plan; it
> remains a later option for `.PRG`/`.TOS`/`.TTP` (CLAUDE.md).
> Work is done in tickets today (`TASK/`). The earlier
> long versions of the rows are in the archive (section *Phase Status —
> Langfassung*).

| Phase | Title | Status | As of | Details |
|-------|-------|--------|-------|--------------|
| 00 | EmuTOS Source Analysis & Extraction | completed | 2026-05-02 | `docs/api/_phase00-completion.md` |
| 01 | Boot + Early Console | completed; boots on a real Pi 4 | 2026-09-04 | `phases/PHASE-01-boot-uart.md` |
| 02 | GEMDOS Console (Trap 1) | D1–D5 implemented, console in use on a real Pi; deliberately excluded: `console_peek_key` does not give the device a turn (`kernel/src/host_env.rs:87-89@console_peek_key`) | 2026-09-04 | `phases/PHASE-02-gemdos-console.md` |
| 03 | BIOS HW Layer | completed | 2026-09-04 | `phases/PHASE-03-bios-hw-layer.md` |
| 04 | BDOS Memory Layer (EmuTOS `bdos/umem.c`, `bdos/iumem.c`) | completed | 2026-05-03 | `phases/PHASE-04-bdos-memory.md`, ADR-025, ADR-027 |
| 05 | High-Half Migration & Process ABI | completed; D7 `Supexec` decided with X01 (Super/Supexec as SDK overlay) | 2026-10-17 | `phases/PHASE-05-high-half-migration.md`, [X01](../TASK/X01.md) |
| 06 | BDOS Process | completed; `PE_BASEPAGE`/`GOTHENFREE`/`BASEPAGEFLAGS` with ARCH03 stage 2 (`32c6171c`), scheduler after EmuTOS `rlr`/`drl` with A115 (`e7754b16`) | 2026-10-25 | `phases/PHASE-06-bdos-process.md`, ADR-028, ADR-029 |
| 07 | BDOS Misc (Time) | completed, 200 Hz system tick; key repeat polled (KEYS01); `etv_timer` call and `dosound` without counterpart (`tos-api/src/bios/tick.rs`) | 2026-10-03 | `phases/PHASE-07-bdos-misc.md` |
| 08 | BIOS Block Devices (Trap 13) | completed; device table with FS10/FS11 | 2026-10-13 | `phases/PHASE-08-bios-block-devices.md` |
| 09 | BDOS File System | FAT16 complete; FAT12 writing out of scope, FAT32/exFAT after 1.0 | 2026-10-19 | `phases/PHASE-09-bdos-filesystem.md`, [FS18](../TASK/FS18.md) |
| 10 | BIOS Screen | completed; VT52 evaluation in `Bconout(2)` with CON04 | 2026-10-23 | `phases/PHASE-10-bios-screen.md`, [CON04](../TASK/CON04.md) |
| 11 | XBIOS (Trap 14) | substantially done; plus KEYS02 (`kbdvec` hook, Pi 2026-10-04), X01, X04; remaining vectors under *Blocked* | 2026-10-17 | `phases/PHASE-11-xbios.md` |
| 12 | VDI | complete (~70 opcodes); Pi acceptance R02–R05 open | 2026-09-21 | `phases/PHASE-12-vdi.md` |
| 13 | AES | complete; ongoing EmuTOS alignment in A tickets, latest A118 | 2026-10-26 | `phases/PHASE-13-aes.md` |
| 14 | Desktop | EL0 program NEWDESK, 1:1 after EmuTOS (ADR-022, ADR-026), source `programs/desktop/`; open DESK tickets P3 | 2026-10-16 | `phases/PHASE-14-desktop.md`, [DESK43](../TASK/DESK43.md) |
| 15 | First C Source Port | ongoing: QED, TeraDesk, zView, Doom, HypView; SDK for outsiders (SDK01) | 2026-10-22 | `phases/PHASE-15-first-source-port.md`, `docs/sdk/` |
| 16 | USB-HID Input (xHCI + VL805) | runs on a real Pi 4; USB01 done 2026-11-04; open USB03 (P1), USB02 (P3), USB04 (P3) | 2026-11-04 | `phases/PHASE-16-usb-hid-input.md`, [USB03](../TASK/USB03.md) |

## Open ADRs

| ID            | Topic                                   | Open for |
|---------------|-----------------------------------------|-----------|
| ADR-BDOS-Lock | BDOS serialization model on SMP; no ADR in `docs/MEMORY.md` | multi-core expansion |

## Blocked

- **KEYS02 / D7 gap: IKBD vectors other than `kbdvec` are not called (2026-10-04).** `Kbdvbase()` returns the full table per process (`0x9A0`), all slots contain valid EL0 stubs; a program hook in `midivec`, `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `clockvec`, `joyvec`, `midisys`, `ikbdsys` or `mousexvec` is, however, **not called**. Mouse kernel-internally via `vdi::feed_relative`, extra mouse buttons via `bios::vectors::mousexvec`; `ikbdsys` read only for the repeat stop (D4). Missing: mouse packets to `mousevec` (format A4), joystick to `joyvec` (HID clarification), `statvec`/`clockvec` (need an IKBD interpreter; after [XBIOS04](../TASK/XBIOS04.md) option 1 there is none, `Ikbdws`/`Bconout(4)` only warn), MIDI (no hardware). Spec `docs/api/xbios/22-kbdvbase.md` B7/C6. Further limits: `Cconrs`/console `Fread`/`v_string` do not yield while a hook is installed (program hangs there); upcalls do not save FP/SIMD registers (hook must manage without them).

- **A45: `appl_tplay` and `appl_trecord` cannot be implemented without the AES fork queue
  (2026-09-13).** Both are stubs today in the sense of
  CLAUDE.md rule 2 — hard-coded return value, arguments ignored. Rule 2
  allows exactly two ways for this, implement or honestly reject and
  record here; this is the second. **The missing dependency is the
  fork queue**, not the function itself: EmuTOS does not record at all in `ap_trecd`
  but sets `gl_recd`/`gl_rlen`/`gl_rbuf`
  (`aes/gemaplib.c:209-213`), and the recording happens in `forker()`
  (`aes/gemdisp.c:123-146`); `ap_tplay` feeds the events back via
  `forkq`/`dsptch` and for `MCHNG` additionally swaps the
  VDI cursor and motion vectors (`aes/gemaplib.c:126-166`). NewTOS' AES has
  no fork queue — the same gap that `tos-api/src/aes/mod.rs` documents at `chkkbd`
  for the key path. It belongs to the cooperative kernel work
  (milestone 2b); when it lands, these two functions are its first
  consumers. **No error code invented:** the AES knows no convention
  for this here, and a value no TOS returns would be a new deviation
  in place of the old one — `appl_tplay`'s `1` is EmuTOS' dispatcher default
  (`aes/gemsuper.c:85,109-110`), `appl_trecord`'s `0` is literally true (zero
  recorded events). **No target application needs the event band:**
  a single hit in the whole `sdk/`, the declaration in
  `sdk/gemlib/overlay/mt_gem.h.in`, no call. Ticket `TASK/A45.md`.

## Recent Decisions

| Date       | Decision                                            |
|------------|-----------------------------------------------------|
| 2026-11-11 | **Legacy ticket migration, owner batches 1-3 (`80d9bcf4a`, `01d9e4f81`, `7d470f846`, `cargo xtask migrate`, Lead; triage and drafts `328853b33`, [LEGACY01](../TASK/LEGACY01.md), Researcher; tool [TOOL54](../TASK/TOOL54.md), accepted `b6231ef80`).** Why: owner decision 2026-11-11, the legacy tickets are cleaned up before the inventory files new ones. Batches 1 and 2: the 80 `G-*` measurement-design stubs OBSOLETE (superseded by the inventory); batch 3: 43 tickets DONE whose receipt names the fix commit. Baseline 899 -> 819 -> 739 -> 696 entries; `status-check` 0 rejected after each. Batch 3 closed TOOL04, so the open list above no longer names it, and TOOL01 (PARKED) moves to parked. The other 17 draft batches wait for two owner decisions (IMPLEMENTED tickets with a DONE receipt; unverified commits). |
| 2026-11-11 | **[TOOL54](../TASK/TOOL54.md): `cargo xtask migrate --batch <N>`, the one-time migration of legacy tickets (`af80ca3da` through `cargo xtask land`, Developer; Tester PARTIAL, then PASS: `measurements/measurement-2026-11-11-tool54-test/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool54-fixcheck/RUNINFO.TXT`; IMPLEMENTED, DONE through `cargo xtask accept`; commit of this line, Lead).** Why: ADR-037 addendum (b), owner 2026-11-11: the legacy tickets are cleaned up before the inventory files new ones, each batch decided by the owner ([LEGACY01](../TASK/LEGACY01.md) classified 677 baseline keys into M1-M5/M?). A committed `TASK/migration/batch-<N>.tsv` with the owner's approval line drives it; line 3 is rewritten, the old one kept verbatim as a quote, the baseline shrinks by exactly those tickets; a hook rejection reverts its writes. Receipts stay immutable: old hashes go through `docs/commit-map.txt` (also in `status-check`), orphan receipts through an owner-approved `historical-receipts.tsv`. A migrate-only change does not count as touching a ticket for the language rule (owner). Host tests 2427 -> 2436/0/7; warnings delta 0. |
| 2026-11-11 | **[TOOL52](../TASK/TOOL52.md): writing xtask commands refuse when the working directory is not in the binary's own repository (`5420b02cd` through `cargo xtask land`, Developer; Tester PASS: `measurements/measurement-2026-11-11-tool52-test/RUNINFO.TXT`; IMPLEMENTED, DONE through `cargo xtask accept`; commit of this line, Lead).** Why: the main tree's binary run from a clone committed a wrongly labelled TOOL51 acceptance in the main repo (TOOL51, *Correction*). Refuse, not redirect: the binary carries its own tree's rules. Read-only: `doc-check`, `deps`, `help`; an unmarked new command counts as writing, and a test fails on it. **Cut (owner 2026-11-11):** from now on tooling runs on one slot, sequentially; the other slot is development, starting with LEGACY01. Host tests 2416 -> 2427/0/7; warnings delta 0. |
| 2026-11-11 | **[TOOL50](../TASK/TOOL50.md): the EmuTOS checkouts are locked on the pin at OS and git level; the agent guard keeps agents from undoing the lock; `cargo xtask land` is the only merge path (`64abacbbe` through `cargo xtask land`, Developer; Tester PARTIAL, PARTIAL, then PASS: `measurements/measurement-2026-11-11-tool50-test/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool50-retest/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool50-fixcheck/RUNINFO.TXT`; lock: `measurements/measurement-2026-11-11-tool50-pinlock/RUNINFO.TXT`; IMPLEMENTED, DONE through `cargo xtask accept`; commit of this line, Lead).** Why: upstream EmuTOS has moved past `1a7484ab` (owner 2026-11-11). A guard over every write found ~30 false negatives in its first test, so the owner stopped the pattern race: the trees are read-only with `uchg` and remotes set to `pinned:/no-*` (ORACLE.md §7, with undo for a pin change); every read, `gen-deps --check` and the host tests work on them. `tools/hooks/pin-guard.sh` blocks chmod/chflags/chown and remote-redirecting git config writes on the trees and Write/Edit into them; `build-emutos.sh` refuses a tree dir under `references/`; `git merge --squash` into `main` is blocked in favour of `land`; paths compare physically and case-insensitively (APFS) in one helper for all guards. Other gaps are documented limits, backstops the lock and TOOL43. Host tests 2415 -> 2416/0/7; warnings delta 0. |
| 2026-11-11 | **[TOOL47](../TASK/TOOL47.md): tests that need gitignored inputs skip only under `NEWTOS_CI=1` (`6d7cb3456`, first landing through `cargo xtask land`, Developer; Tester PARTIAL, then PASS: `measurements/measurement-2026-11-11-tool47-test/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool47-fixcheck/RUNINFO.TXT`; IMPLEMENTED, DONE through `cargo xtask accept`; commit of this line, Lead).** Why: prerequisite of the [TOOL43](../TASK/TOOL43.md) workflow. Without the switch a missing input stays a failure (owner 2026-11-11); with it the test prints one SKIPPED line and `cargo xtask test` lists the skips. In a clone without inputs: `NEWTOS_CI=1` 2407/0/7 with 8 skips, unset 8 failures. `resolves_against_the_workspace` was a test defect, fixed; `emutos_resolves_at_the_pin` no longer skips silently (a failure goes to the owner, never fixed in the checkout). The TOOL46 one-time `scratch-clean --ticket TOOL46` freed 0 bytes (its workers predate `target-agents`); the two merged worktrees (10.5 GB) were removed. Host tests 2408 -> 2415/0/7 on main. |
| 2026-11-11 | **[TOOL46](../TASK/TOOL46.md) and [TOOL51](../TASK/TOOL51.md): worker clones and cargo target dirs move off the system disk; `accept` refuses the developer under either spelling of the agent name (squash merge of the TOOL46 worktree branch, commit of this line, Developer; Tester PARTIAL, PARTIAL, then PASS: `measurements/measurement-2026-11-11-tool46-test/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool46-retest/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool46-fixcheck/RUNINFO.TXT`; IMPLEMENTED, DONE through `cargo xtask accept`).** Why: 13.6 GB of TOOL41 test clones filled the system disk (ENOSPC). Each worker gets `target-agents/agent-<id>/` on the data volume (SubagentStart); a stop only records a marker (owner 2026-11-11: a resumed worker or one waiting on background work keeps its clone); `accept` and the new `cargo xtask land` delete the recorded agents' dirs, SessionStart those stopped over 72 h ago and warns above 20 GiB; the scratch guard blocks clones and target dirs in the scratchpad. The retest found that `accept` compared agent names without dropping `agent-`, so a developer could accept their own ticket (from TOOL41): one normalisation now, under TOOL51. This landing is the last manual squash; from now on `cargo xtask land`. Host tests 2400 -> 2408/0/6; warnings delta 0. |
| 2026-11-11 | **[TOOL44](../TASK/TOOL44.md): `cargo xtask worktree-setup` links the main tree's gitignored inputs into a fresh worktree (squash merge of the TOOL44 worktree branch, commit of this line, Developer; Tester PASS, refusal in the main tree shown by host tests: `measurements/measurement-2026-11-11-tool44-test/RUNINFO.TXT`; IMPLEMENTED, DONE through `cargo xtask accept`).** Why: a fresh worktree lacked `references/*` and the `sdk/apps/<app>/` checkouts, so 8 xtask tests failed and workers linked them by hand. It derives the inputs from the ignore rules (no app names), never replaces or deletes, and keeps `git status` clean with anchored entries in the shared `.git/info/exclude` (owner OK 2026-11-11). In a fresh worktree `cargo xtask test` then runs 2400/0/6, the main tree's set plus its 4 tests. |
| 2026-11-11 | **First acceptances through `cargo xtask accept`: [TOOL40](../TASK/TOOL40.md), [TOOL41](../TASK/TOOL41.md), [TOOL42](../TASK/TOOL42.md) DONE (`276750c49`, `bb65728f8`, `c205005ba`, Tester; accept trailer `bfc3c6ba2`, Developer); [TOOL43](../TASK/TOOL43.md) step 1 report `docs/analysis/ci-backstop-2026-11-11.md` (Researcher; commit of this line, Lead).** Why: owner decisions 2026-11-11. The tester re-checked each ticket on `bfc3c6ba2` (host tests 2396/0/6, guard pipe tests 0 FAIL) before accepting; every accept commit passed the live ticket hooks and ends with the `Co-Authored-By` trailer (ADR-037 decision h). TOOL43 stays OPEN: no workflow before the owner chooses between alarm only, protected `main`, or admin bypass (report, *Owner's choices*). |
| 2026-11-11 | **Ticket checks switched on (`git config newtos.ticketChecks true`, owner decision 2026-11-11, ADR-037 point 14; commit of this line, Lead).** Why: the ADR-037 tools ([TOOL41](../TASK/TOOL41.md)) and the guard ([TOOL42](../TASK/TOOL42.md)) are in `main`; from now on `pre-commit` runs `status-check --staged --new-only` and `commit-msg` checks every id a `fix`/`feat` subject names. Before the switch the receipts of [TOOL40](../TASK/TOOL40.md) and TOOL42 got parseable `Role:`/`Agent:` headers (`9ab8bc207`), ADR-037 point 13 now says ids count from the subject only, decision (h) gives `accept` commits the usual `Co-Authored-By` trailer, and [TOOL46](../TASK/TOOL46.md) (P1) moves worker clones off the system disk after 13.6 GB of TOOL41 test clones filled it. The TOOL41/TOOL42 worktrees and branches are removed (fully contained in `b7029ab7b`, `9bc27d82b`). |
| 2026-11-11 | **[TOOL41](../TASK/TOOL41.md): the ADR-037 ticket tools — `status-check`, `accept`, `set-status`, `report` and a `commit-msg` hook (squash merge of the TOOL41 worktree branch, commit of this line, Developer; Tester PARTIAL, then PASS after fixes: `measurements/measurement-2026-11-11-tool41-test/RUNINFO.TXT`, `measurements/measurement-2026-11-11-tool41-retest/RUNINFO.TXT`; IMPLEMENTED until `cargo xtask accept`).** Why: ADR-037 and its addendum (owner 2026-11-10/11) said who may change a ticket state, but nothing read a status line. Line 3 now parses; `status-check` names each gap of point 13 and records today's 899 legacy violations in `tools/xtask/ticket-baseline.tsv`, which only shrinks; `accept` is the tester's one commit and refuses the developer's own agent; `set-status` is the lead's, with mandatory fields; `report` counts and regenerates the `TASK/README.md` status column. The new hook checks are off until the owner sets `git config newtos.ticketChecks true`; off, the hooks behave as after TOOL40 (18 of 18 cases identical). CLAUDE.md and the developer and tester briefs name the commands. Host tests 2376 -> 2396/0/6; warnings delta 0. |

## Notes

The project is intentionally over-documented at the start. The premise:
spec-first development against a fixed, finite, well-documented API
prevents the implementation drift that ruins long-running OS hobby
projects.

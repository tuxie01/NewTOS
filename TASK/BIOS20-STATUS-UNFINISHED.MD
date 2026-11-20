# BIOS20 — receipt (developer, 2026-11-20)

Role: developer
Agent: developer-a7de965fe3f5ad91c

Branch `bios20` from `main` c2140d9ef, worktree `target-agents/agent-a7de965fe3f5ad91c/wt`. Spec: the decided
contract, `docs/api/bios/_sysvars-counters-contract.md` section 17 (the checklist 17.5), with sections 5-13 for the
reasons; `docs/api/_super-state.md` S10; ADR-031 amendment of 2026-11-20 (`docs/MEMORY.md`). The two fixed values:
the `hdv_boot` stub returns 4, the `vblqueue` watcher compares the eight slots.

Line cites into code below were kept stable on purpose: every new block stands at the end of its file and the
edits above it keep their line count, so no anchored cite in a doc this commit leaves alone moved
(`doc-check --staged`: 0 broken; `docs/MEMORY.md` is not edited). `--fix-anchors` (TASK/TOOL49.md) does not exist
yet, so this was the only way not to touch those docs.

Changes:

- `tos-api/src/bdos/process/loader.rs:2238@write_bios20_cells` (new, end of file), called from
  `populate_sysvars` (`:1305@write_bios20_cells`, on the former blank line): `$502`, `$50E`, `$512`, `$46A` = the
  `ret` stub; `$47A` = the `hdv_boot` stub; `$442` = `TICKCAL_MS` (20); `$44E` = `v_bas_ad_value()` (not written
  without a screen); `$452` = 1, `$454` = 8, `$456` = `$4CE` (the eight slots stay 0 from the page-0 fill); `$466`
  and `$462` seeded from `tick::frclock()`; `$48E`-`$49D` = `{0, membot, memtop - membot, 0}` from
  `stram_range()`; `$4C6` = `0x7F400`. `_pun_ptr` stays 0 (Q7). New constants `:2212@DSKBUF_VA` (+ size and the
  asserts: `+0x400 + 0xC00 == PAGE_SIZE`, after the PARMBLK scratch, 8-aligned, below 4 GiB), `:2227@VBL_LIST_ADDR`,
  `:2230@VBL_LIST_SLOTS`. `install_slot` resets the slot's lost-frame count (`:691@reset_frames`); the buffer is
  zeroed at load by the existing `private.fill(0)`.
- `tos-api/src/bios/tick.rs`: `publish` (`:108@publish`) also calls `publish_frames_running`; new section after
  it: `start_frame_clock`, `frclock` (`:200@frclock`, frame clock minus the boot frame, wrapping 32 bit),
  `reset_frames`, `write_frames` (`:225@write_frames`: charge `fc - ACCT_FC` to the slot's lost count when its
  `_vblsem <= 0`, then `ACCT_FC = fc`, `_frclock = fc`, `_vbclock = fc - lost`), `publish_frames_running`
  (`:249@publish_frames_running`), `switch_frames` (`:265@switch_frames`: the leaving process is charged up to
  now, the new one refreshed without charge), `vbl_semaphore_down` (`:288@vbl_semaphore_down`). `lost` is a
  per-slot static array in this file (`VBL_LOST`), `acct_fc` one global word (spec section 6: placement is the
  developer's). All lock-free, no allocation, no I/O: one `CNTPCT` read through `Host::frame_clock`, relaxed
  atomics, unaligned stores into the static page 0.
- `tos-api/src/bdos/process.rs:977@set_current`: calls `refresh_on_switch(slot_for(current()), slot_for(bp))`
  before `RUN` changes (`:1462@refresh_on_switch`, end of file): `_v_bas_ad` of the new process, then
  `tick::switch_frames`. For every process, not only accessories.
- `tos-api/src/xbios/system.rs:33@vsync`: after the spin, `tick::publish_frames_running()` (spec section 5 rule
  3); `Vsync` still compares raw `frame_clock` values. Same line count (early `return 0` without a host).
- `tos-api/src/xbios/screen.rs`: end of file `V_BAS_AD_ADDR`, `v_bas_ad_value` (`:567@v_bas_ad_value`: the user
  address of `ScreenInfo.base`, what `Logbase` answers a process, computed without publishing the screen,
  decision Q4 (a)), `write_v_bas_ad` (`:576@write_v_bas_ad`), `set_logical` (`:590@set_logical`); `setscreen`
  calls `set_logical` instead of `host.set_screen_logical` (`:299@set_logical`).
- `tos-api/src/xbios/kbdvecs.rs`: `write_stubs` also writes the `hdv_boot` stub; end of file
  `HDV_BOOT_STUB_VA` (`:488@HDV_BOOT_STUB_VA` = stub area `+0x20`), `HDV_BOOT_NOT_BOOTABLE` = 4,
  `HDV_BOOT_CODE = [0xD280_0080, 0xD65F_03C0]`, asserts (after `kbdvec_default`, inside the 0x40 area, the MOVZ
  immediate is 4). **Encoding checked:** `printf 'mov x0, #4\nret\n'` through `clang -target
  aarch64-unknown-none-elf -c` and `llvm-objdump -d` gives `d2800080` and `d65f03c0`, as the spec says.
- `tos-api/src/bdos/process/hooks.rs`: `PAGE0_HOOKS` keeps 10 rows: `$456` is replaced by `_vbl_list` slot 0
  (`:285@VBL_SLOT0`), `DumpVec`'s default is the `ret` stub (`:294@DumpVec`); slots 1-7 are `VBL_SLOT_HOOKS`
  (`:796@VBL_SLOT_HOOKS`), `page0_hook_cells` (`:810@page0_hook_cells`) chains both and `check_page0` iterates it.
  `aux_*`, `hdv_init`, `hdv_boot` stay unwatched; the `silent` test now checks all 17 cells. The test module
  carries `#[allow(clippy::items_after_test_module)]` (one line, so the cites into the tests stay valid).
- `tos-api/src/bios/mod.rs:95@TICKCAL_MS`: `pub(crate)`.
- `kernel/src/vbl.rs:123@run_if_due`: after the `LAST_FRAME` swap, `if tick::vbl_semaphore_down() { return; }`
  (the frame stays consumed; no process: no gate). Module doc updated in place.
- `kernel/src/host_env.rs:739@install`: `tick::start_frame_clock()` right after `install_host`.
- Tests: unit `loader.rs` `bios20_static_cells_hold_the_emutos_values`, `the_hdv_boot_stub_returns_4`;
  `the_page_0_hook_defaults_are_what_the_loader_writes` now iterates all 17 cells. New
  `tos-api/tests/bios20_sysvars.rs` (10): counters start at 0 and follow the frame clock; nothing published
  without a process; `_vblsem` 0 / -1 for n frames gives `$466 - $462 == n`, 1 again both advance; switch-in
  refresh, away time not charged, the leaving process charged up to the switch; `Vsync` publishes (`b - a >= 1`,
  and 3 Vsyncs with `_vblsem` 0 leave `$462`); the gate (0, negative, 2, no process); `_v_bas_ad` at switch-in and
  after the process's own `Setscreen(L)`, left alone for `logLoc <= 0`; `dump_vec` watcher quiet at the stub,
  warns on another value and on 0; a routine in slot 7 or slot 0 warns, `$456` not compared.
- Permanent probe: `tosdump/probes/svprobe.c` (cases C01-C14 of spec 17.6), `tosdump/build-svprobe-newtos.sh`,
  Makefile targets `svprobe-m68k`/`svprobe-newtos`, runner `tosdump/probes/run-svprobe.sh emutos tt|falcontc
  <out> | newtos <out> <kernel8.img>` (copy of `run-hkprobe.sh`); `tools/hatari-oracle.sh` knows `SVPROBE.PRG`.
- Docs: `docs/api/bios/_sysvars-contract.md` (13 cells and `_vblqueue` moved to served, counts now served 51,
  hardware 23, missing 5, free 3, n/a 2; `_vbl_list` note; writer list item 8; watcher/reader text; section 5 and
  6 notes); `docs/COMPATIBILITY.md` ("System variables — compatible how": the table pointer with the counts and
  the per-process / `_vblsem` / `_v_bas_ad` / `vblqueue` / `_themd` / `_dskbufp` / stubs / `_pun_ptr`
  differences; OS-hooks paragraph: `dump_vec` against the stub, the `vblqueue` slots); `docs/DEVIATIONS.md` (ten
  rows under "XBIOS, keyboard, system variables"); `docs/analysis/deps/mapping.tsv` (new rows `getmpb`,
  `blkdev_init`, `init_system_timer`, `screen_init_address` -> `populate_sysvars`, `setscreen` -> `write_v_bas_ad`,
  `int_vbl` -> `write_frames` and `vbl_semaphore_down`; notes of `chardev_init`, `bios_init`, `blkdev_hdv_boot`,
  `flop_hdv_init`, `setscreen`, `vsync`, `int_vbl`); `TASK/BIOS20.md` line 3.

Checklist 17.5, row by row: `$442`, `$44E`, `$452`, `$454`, `$456` + slots, `$462`, `$466`, `$46A`, `$47A`,
`$48E`, `$4C6`, `$502`, `$50E`/`$512` all implemented with the writer and host test the row names. Not by a host
test: `$44E == logbase()` itself (for a process `Logbase` publishes the screen uncached and runs `dc civac`, which a
macOS host test cannot execute: the test compares with the address `Logbase` computes, the probe case C02 compares
with `Logbase()`); "a new process starts with `lost = 0`" (the reset is in `install_slot`, which needs a loaded
image: covered by the probe, C07 boot line `frclock_minus_vbclock_zero=1`).

Build / clippy / test (worktree, default target dir `wt/target`; baseline = the same commands at c2140d9ef
before any edit):

- `cargo xtask build --board rpi4`: OK, 159 `warning` lines; baseline 159. `--board qemu-virt`: OK, 159 / 159.
- `cargo clippy -p kernel --features board-rpi4`: 196 / 196; `--features board-qemu-virt`: 196 / 196;
  `cargo clippy -p tos-api --all-targets`: 118 / 118. New = 0.
- `cargo xtask test` (full): exit 0, 2599 passed / 0 failed / 7 ignored over 224 results (XBIOS11 recorded
  2587 / 0 / 7 over 223; +12 = the 2 unit tests and the 10 of the new binary).
- `cargo xtask doc-check --staged`: the 4 staged docs 0 failures (7 legacy on unchanged lines); "2163 docs the
  commit leaves alone, 2773 of their cites name one of the 21 changed paths, 0 broken by this commit".
- `cargo xtask deps --check-mapping --staged`: 0 problems the commit introduces.
- `make -C tosdump svprobe-m68k svprobe-newtos`: both build (`-Wall -Wextra -Werror`), NAP 0 undefined symbols.

Smoke runs (developer, not evidence; outputs in `target-agents/agent-a7de965fe3f5ad91c/run-*`, not committed):
`run-svprobe.sh newtos` with the debug rpi4 `kernel8.img` of this commit's code (sha256
`86f9069d5816ccd504df0222170b390754505a324fd8e4a34765a3c2622d4fb9`), probes SVPROBE.PRG
`428b04513e22c19d2c0e476a4ac2b86f9c19c02370fba8d7b3bd88cf334be126`, SVPROBE.NAP
`5bcce799e9a3813dca2d09f3fdc9ad380ec59cf2434a72ecd4050ffe5a9b37a1`; card manifest `optional_missing none`,
`host_clang Apple clang version 21.0.0 (clang-2100.4.1.1)`. Every NewTOS line as the spec predicts (C01 20, C04 1,
C05 8, C06 `4ce` with 8 empty slots, C10 all equal and calls return, C11 returns 4, C12 classes all 1, C13
read/write 1, C14 0, C02/C03 all 1, C07 boot equal and Vsync `>= 1`, C08 `d462=0`, C09 larger, C09b ratio 333);
no `[hook] warning`, `desktop up`. `run-svprobe.sh emutos tt` and `falcontc`: differences to NewTOS only in the
lines the spec decided (C06 slot 0 set, C10 `hdv_init` not the stub, C11 not called, C14 non-zero, C07 boot
`_frclock != _vbclock`).

## Rework after the tester's PARTIAL (2026-11-20)

The tester's run on 3ae5333b0 (`measurements/measurement-2026-11-20-bios20-test/RUNINFO.TXT`, main 083b9c6d6) found no
code failure and two documentation items. To cite the run, `main` was merged into `bios20` (merge commit
c461f7c17, only the run directory comes in; precedent `Merge branch 'main' into v62`). Code unchanged; one doc commit:

1. `_themd`: the run measured the oracle as a running program reads it (probe C12): tt the same field classes as
   NewTOS (link 0, start `membot`, length `memtop - membot`, owner 0), falcontc a length other than `memtop - membot`.
   Corrected where the "live BDOS descriptor / first allocated block" statement stood: the `docs/DEVIATIONS.md` row,
   the `_themd` bullet of `docs/COMPATIBILITY.md` ("System variables — compatible how"), the `0x48E` row of
   `docs/api/bios/_sysvars-contract.md`; in `docs/api/bios/_sysvars-counters-contract.md` section 11 a "Measured"
   note after the inference and the C12 row of 17.6 (the decision Q5 (b) is unchanged). Not explained: the falcontc
   length.
2. Frame rate: oracle `_frclock` 60.0 Hz on tt, about 50.5 Hz on falcontc (probe C09b, 200 Hz ticks per frame 3.33
   and 3.96), NewTOS 60 Hz by decision C1. New `docs/DEVIATIONS.md` row (trigger: a program counts frames to
   measure time); the per-process bullet of `docs/COMPATIBILITY.md` that speaks of the frame clock now names both
   rates; the C09b row of the counters contract carries the measured values.

`cargo xtask doc-check` of the four edited docs: 6 failures, all "missing" cites into gitignored `sdk/` trees on
unchanged lines (`docs/COMPATIBILITY.md:255,397,482,592,2306`), as the tester's item 10 records for main;
`doc-check --staged`: 0 introduced. No code path was found wrong by the run.

## Tester handoff

- Host: `cargo test -p tos-api --test bios20_sysvars` (10), `cargo test -p tos-api --lib bios20` and `--lib
  hdv_boot` (1 each), the existing `bios15_os_hooks`, `bios16_page0_write`, `bios07_setexc`, the `hooks` and
  `loader` unit tests. Expected: pass.
- Probe on both sides: `tosdump/probes/run-svprobe.sh emutos tt <out>`, `... emutos falcontc <out>`,
  `... newtos <out> <kernel8.img>` (rpi4 kernel of the landed commit). Expected NewTOS log as in the smoke run
  above. Expected differences to the oracle, all decided: C06 slot0/nonnull, C10 `hdv_init eq_prt_stat`, C11 the
  call line, C14, C07 boot line; C09b ratio is 200 / the frame rate (NewTOS 60 Hz: ~333; the smoke saw tt ~333,
  falcontc ~400, i.e. 60 and 50 Hz: the oracle rate the ticket asks to measure).
- Tables on raspi4b via `cargo xtask tosdump`: `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`,
  `xbios-system-r1.tsv`. Expected: no case equal before differs newly (`Setexc` reads of `$000-$40F` are
  unchanged: `write_page0_vectors` is not touched).
- APITEST; boot to the desktop on qemu-virt and raspi4b with the mouse pointer drawn and following the mouse
  (the `vb_draw` gate: every process has `_vblsem` 1, so nothing may change); a screendump after a pointer move.
- On screen only (not in a log): C08 also stops the pointer redraw while `_vblsem` is 0 (Q2 (b)).
- Real-Pi-only: the same probe and the pointer on the real Pi 4 (the frame clock and the 200 Hz tick come from
  the generic timer on both, so no board-specific path; the Pi is the acceptance gate).
- What the oracle cannot show (spec 17.6): the per-process view (C09), the NewTOS rate choice, the stub, ROM and
  buffer addresses, the 5 ms publish lag, the effect of `hdv_boot`.

## Proposed docs/STATUS.md line

- 2026-11-20 — BIOS20: thirteen system variables hold EmuTOS's values: `dump_vec`, `aux_stat`, `aux_vec`,
  `hdv_init` the `ret` stub, `hdv_boot` a stub returning 4, `_timer_ms` 20, `_vblsem` 1, `_nvbls` 8, `_vblqueue`
  eight empty slots (watched), `_frclock`/`_vbclock` on the 60 Hz frame clock from 0 at boot (published by the
  tick, `Vsync` and every switch-in; `_vbclock` stops while `_vblsem <= 0`, which also skips the mouse draw),
  `_v_bas_ad` the `Logbase` address, `_themd` the MD at load, `_dskbufp` a 3072-byte private buffer; `_pun_ptr`
  stays 0 (BIOS18). Permanent probe `tosdump/probes/svprobe.c`. Commit: <landed hash>.

## Out-of-scope observations

- Smoke on the oracle `falcontc` (not evidence): C12 `len_eq_memtop_minus_membot=0` there (tt: 1), so `_themd`
  after boot differs from `memtop - membot` on that profile; the spec's section 11 inference (first allocated
  block) is not what tt showed either (all four classes as at `getmpb` time): a measurement point for the
  tester/researcher, the NewTOS value is the decided one.
- `_hz_200` of a switched-in process is not refreshed at the switch (only the frame counters are, as decided):
  the probe's C09b needed one `Vsync` after the child to read a fresh `$4BA` (comment in `svprobe.c`).
- `doc-check --fix-anchors` (TASK/TOOL49.md, OPEN) would have allowed a plainer layout of the new code.
- `docs/api/bios/_sysvars-contract.md` section 6 item 6 quotes the BIOS19 counts in a proposal text; left as the
  BIOS19 record, the live counts are in section 2 and in `docs/COMPATIBILITY.md`.

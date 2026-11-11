# BIOS06 — receipt (unfinished: tester acceptance of O3, `Getmpb(NULL)` ends the program)

Role: developer
Agent: agent-afaf0403dea94a2cf

Developer, 2026-11-11. Worktree branch `worktree-agent-afaf0403dea94a2cf`, base `78d972f1f`
(= `main` when the second commit was made, so no rebase was needed); not merged. The legacy
receipt [BIOS06-STATUS-DONE.MD](BIOS06-STATUS-DONE.MD) (O1 = A, 2026-10-29) stays untouched;
this receipt covers only the owner decisions on O3 (ticket [BIOS06](BIOS06.md), "Owner decision
2026-11-11 (O3)", "Owner decision 2026-11-11 (O3, super state)", "Implementation (2026-11-11)").
Two commits: `ab6a59867` (NULL special case removed) and the super-state commit after it.

## Done

- `tos-api/src/bios/getmpb.rs:154@getmpb`: the silent NULL success (`if mpb == 0 { return 0; }`)
  is gone. A program's NULL is refused before any store, in user and in super state
  (`tos-api/src/bios/getmpb.rs:162@refuse_user_write`); an in-kernel caller's NULL is refused
  without a fault record (`tos-api/src/bios/getmpb.rs:161@is_user`), since its pointers are
  kernel addresses. Every other MPB is written by `tos-api/src/bios/getmpb.rs:205@fill_mpb`
  (mp_free, then mp_used; the first fault ends the call).
- `tos-api/src/uaccess.rs:890@refuse_user_write` (new): records the fault a store to a read-only
  page reports (`tos-api/src/uaccess.rs:872@refused_write_record`: EC `0x25`, ISS `0x4F`, the
  `sttr` syndrome measured in `docs/analysis/arch03-stage0b-spike.md` §5; `FAR` = the address;
  `pc` the refusing routine, state and registers zero since no exception was taken). Placed after
  the target/host code of the module so no existing line cite into `uaccess.rs` shifts.
- EL0 end of the call, read in the code (not driven in QEMU in this run): the recorded fault is
  taken by `kernel/src/arch/aarch64/exceptions.rs:775@take_pending_fault` in the syscall tail and
  `kernel/src/arch/aarch64/exceptions.rs:1510@uaccess_fault_tail` shows the crash report ("Bus
  Error", `classify` of the syndrome) and ends the program with `Pterm(-1)` — the path a faulting
  `sttr` (`kernel/src/arch/aarch64/exceptions.rs:281@fixup_for`) takes too.
- Spec `docs/api/bios/00-getmpb.md`: BIOS06 block translated to English; V3, O3, header note,
  Inputs, Error conditions and Verification criteria record both decisions; nothing left open.
- `docs/COMPATIBILITY.md`: sanctioned deviation "`Getmpb(NULL)` ends the program, the system
  keeps running" at the end of "Sanctioned deviations", no super-state exception; the
  `COMPATIBILITY.md` cites in `TASK/BIOS19.md` line 10 moved with it (`1880-1907`, `:1882`).
- `docs/analysis/deps/mapping.tsv`: `getmpb` row note updated, new row `getmpb` → `fill_mpb`.
- `TASK/BIOS06.md` translated to English (the `Migrated` line and the quoted old line 3 kept
  verbatim), line 3 `Missing:` updated, both owner decisions and the implementation recorded.

## Tests and build

- Host tests in `tos-api/src/bios/getmpb.rs`:
  `null_mpb_from_a_program_is_refused_before_any_store_in_any_state` (fault result, the
  recorded fault equals `refused_write_record(0)`, `classify` = Bus Error, no store — a store to
  0 would crash the host test), `unwritable_mpb_faults_and_writes_nothing`,
  `writable_mpb_gets_free_then_used_only`, `null_mpb_from_the_kernel_is_refused_before_any_store`
  (no fault recorded); the two older getmpb tests green. The fault-record tests are serialised by
  `FAULT_RECORD_TEST_LOCK` (test-only, `uaccess.rs`), also taken by the existing
  `fault_record_round_trip_and_first_fault_wins`.
- `cargo xtask test` (full, final code): 2454 passed, 0 failed, 7 ignored (sum over 209 result
  lines), rc 0.
- `cargo xtask build --board rpi4 --release` and `--board qemu-virt`: rc 0, warnings kernel 93 /
  tos-api 64 on both, equal to the base. `cargo clippy -p kernel --features board-<b>`: kernel 121 /
  tos-api 73 on both boards, the same warning set as the base. Host
  `cargo clippy -p tos-api --tests`: no warning in `getmpb.rs` or `uaccess.rs`.
- `cargo xtask deps --check-mapping`: all rows resolve.
- doc-check of the edited docs: no failure from this change (remaining ones are paths missing in
  this worktree: the gitignored application sources and the gemlib, mintlib, cflib and zview
  source trees).

## Missing

- Tester acceptance (`cargo xtask accept BIOS06`). Suggested QEMU check: the BIOS06 run's probe
  (`measurements/measurement-2026-10-29-bios06/RUNINFO.TXT`,
  `measurements/measurement-2026-10-29-bios06/method/run-newtos.sh`, GNPROBE
  group N, which calls `Getmpb(0L)` in user state) on this kernel: expected N01 in the log, no
  N02/N03, the crash report "Bus Error" with FAR 0, a key press, then the desktop running on
  (EmuTOS: panic, halt). The super-state case has no probe yet (a `Supexec`-wrapped call would
  show the same report and `$0`-`$F` of page 0 unchanged).

## Scratch

Worktree `/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-afaf0403dea94a2cf`, its
`target/` 5.9 GB (`du -sh`). Gitignored outputs built from the final code:
`target/aarch64-unknown-none-softfloat/release/kernel8.img` sha256
`6bd20446df510f87d3eb7cdd2f5b24c15aa7069eb945c8219053f8ca011475a1`,
`target/aarch64-unknown-none-softfloat/release/kernel` sha256
`68b6cd97974f056cf5389189961138a950d218ec3f5df2b78939f1958973cf62` (rpi4),
`target/aarch64-unknown-none-softfloat/debug/kernel` sha256
`4552ee41f21bd4abf3a79aeced46fddfd468b79638d32850185fc4de4ccda9a1` (qemu-virt).

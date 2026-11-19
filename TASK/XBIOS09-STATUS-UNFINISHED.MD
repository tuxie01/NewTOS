# XBIOS09 — receipt (developer, 2026-11-19)

Role: developer
Agent: developer-aca4aa32023f1bcf5

Branch `xbios09` from `main` 9ffcce079, worktree
`target-agents/agent-aca4aa32023f1bcf5/wt`.

Changes:

- `tos-api/src/xbios/mod.rs:124@GETTIME` (`dispatch`, arm `GETTIME`): returns
  `i64::from(xbios_gettime() as i32)`, the packed LONG sign-extended into `x0`;
  before it was `xbios_gettime() as i64` on a `u32` (zero extension).
- `tos-api/src/xbios/mod.rs:117-119@SETTIME`: the clock-group comment says
  "sign-extended" instead of "zero-extended", kept to its three lines so no
  line cite into the file shifts. `Settime` itself is unchanged
  (TASK/XBIOS08.md, owner option B).
- New host test `tos-api/tests/xbios09_gettime_sign_extension.rs`
  (`gettime_sign_extends_the_packed_long`): through `xbios::dispatch`,
  `Settime(0x8CCF6000)` (2050-06-15 12:00:00) then `Gettime` gives
  `0xFFFFFFFF8CCF6000` (negative); `Settime(0x2B8F6000)` (2001-12-15 12:00:00)
  then `Gettime` gives `0x000000002B8F6000`. The host uptime is frozen, so the
  value reads back exactly; both cases are in one test because the clock is
  one global.
- `docs/api/xbios/17-gettime.md`: the Pi-binding gap line "`x0` extension" is
  closed (cites the arm and the test); the open-points line on the comment is
  updated.
- `docs/COMPATIBILITY.md`, "LONG results in `x0`": `Gettime` removed from the
  list of known zero-extending deviations, a sentence records the fix.

Spec satisfied: `docs/api/xbios/17-gettime.md` "Trap interface" (Native ABI)
and "Return value" / "Width and extension"; `docs/COMPATIBILITY.md` "LONG
results in `x0`: sign-extended". EmuTOS: `LONG gettime(void)`
(`references/emutos/bios/clock.c:1266`), `LONG` signed 32 bit
(`references/emutos/include/portab.h:123`).

Build / clippy / test (worktree, default target dir `wt/target`):

- `cargo xtask build --board rpi4`: OK; `tos-api` (lib) 64 warnings, `kernel`
  (lib) 93 warnings, 159 `warning` lines; HEAD 9ffcce079 same build: 159.
  Delta 0.
- `cargo xtask build --board qemu-virt`: OK; 64 + 93 warnings, 159 lines. Delta 0.
- `cargo clippy -p tos-api --all-targets`: 118 `warning` lines after, 118 at
  HEAD (measured on the same worktree with the two files reverted to HEAD and
  restored); none in `xbios/mod.rs` or the new test. New = 0.
- `cargo xtask test` (full): exit 0, 2580 passed / 0 failed / 7 ignored over
  222 test binaries; the new test passes; `xbios12_default_datetime` (the boot
  clock, `time_init`) passes unchanged.
- `cargo xtask deps --check-mapping`: 1990 rows, all resolve. Row `gettime`
  (`tos-api/src/gemdos/time.rs`, `xbios_gettime`, mapped) still valid; no
  row edited (the function it maps is unchanged; the extension is in the
  dispatch arm).
- `cargo xtask doc-check docs/api/xbios/17-gettime.md`: 0 failures.
  `docs/COMPATIBILITY.md`: 9 failures, all pre-existing cites into unpacked
  SDK sources (`sdk/gemlib/gemlib-0.44.0/…`, `sdk/cflib/cflib-21/…`,
  `sdk/libc/mintlib/…`, `sdk/zview/…`) absent from a worktree, on lines this
  commit does not touch; none introduced.

## Tester handoff

- Host: `cargo test -p tos-api --test xbios09_gettime_sign_extension` (the
  acceptance case named by the ticket; the oracle cannot show bit 31,
  TASK/XBIOS14.md). Expected: pass.
- Retest on QEMU raspi4b (ticket section "Reopened; localization and retest
  set"): `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`,
  `xbios-system-r1.tsv` via `cargo xtask tosdump`; expected: no case that was
  equal before differs newly (the 2026 clock values have bit 31 clear, so
  `Gettime` returns the same bits as before).
- Optional confirming case: probe H01 (`xshi.c`, `Settime(0x8CCF6000)`) of
  `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`
  should now print `hi=ffffffff negative-as-long=yes`.
- Boot clock: the `[clock] no RTC: clock starts at the build date …` line
  unchanged (`time_init` not touched).
- Real-Pi-only: nothing; the change is in the board-independent dispatcher.

## Proposed docs/STATUS.md line

- 2026-11-19 — XBIOS09: XBIOS `Gettime` returns its packed LONG
  sign-extended into `x0` (from 2044 on negative, as `LONG gettime(void)`);
  host test `xbios09_gettime_sign_extension`; `Gettime` leaves the
  COMPATIBILITY "LONG results in `x0`" deviation list. Commit: <landed hash>.

## Out-of-scope observations

- `tos-api/src/gemdos/time.rs:145@tgettime` doc-comment says "zero-extended
  LONG" for `Tgettime`; its WORD value never has bit 31 set, so harmless, left
  untouched.
- `cargo xtask build` fails with `CARGO_TARGET_DIR` set (programs/build.sh
  looks for `libhello_gem.a` under `<worktree>/target/programs/…`); built with
  the default target dir inside the worktree instead.

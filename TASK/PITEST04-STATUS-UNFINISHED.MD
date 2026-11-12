# PITEST04 — receipt (developer): Ssystem mode 2023 S_NEWTOS_SCRPITCH built

Role: developer
Agent: developer (agent-a68eed2954a3b5397)

Base: `main` 4f7cbfd7b, branch `pitest04`, one `feat(gemdos)` commit.

Spec satisfied: `docs/api/gemdos/154-ssystem.md` §"NewTOS-own mode 2023 — the
framebuffer pitch (PITEST04)" and the 2013–2015, 2023 row of the board table.

## Files and functions changed

- `tos-api/src/gemdos/ssystem.rs`: new `pub const S_NEWTOS_SCRPITCH: u16 = 2023`;
  `ssystem` dispatches it through the scalar arm (`newtos_scalar`, so the
  `arg1 == 0` / pointer form and `arg2 != 0` → `EINVAL` rules are the same as
  for 2013–2015); `sysinfo_value` returns `Host::screen()?.stride_bytes`
  as a `Long`. `screen()` returning `None` (no display, hostless) becomes
  `ERROR`. Module doc and comments name the mode.
- Kernel: no change. `kernel/src/host_env.rs` already fills
  `ScreenInfo.stride_bytes` from `vdi_backend::stride_bytes()` (spec cites
  checked against the code, they match).
- `tos-api/tests/gemdos_ssystem_sysinfo.rs`: the stand-in display gets a padded
  pitch (`1984 * 4`, not `1920 * 4`), so a value derived from width and depth
  would fail; 2023 checked in both forms, in the unknown-fact case (`ERROR`,
  nothing written) and in the `arg2 != 0` case; the first-free-number test is
  now `the_block_ends_at_2020_and_2024_is_the_first_free_number` (2024 →
  `EINVFN`).
- `tos-api/tests/gemdos_ssystem_cpuinfo.rs`: the free-number probe moves from
  2023 to 2024.
- `docs/api/gemdos/154-ssystem.md`: state "not built yet" → built, the tests
  line and the open point that said the pins move with the build, and the
  anchors into `ssystem.rs` / the tests re-pointed (doc-check 0 failures). No
  semantic change to the spec.

## Build / clippy / tests

- `cargo xtask build --board rpi4`: OK (159 warnings, none in a touched file;
  same set as before the change — no warning mentions `ssystem`).
- `cargo xtask build --board qemu-virt`: OK (159 warnings, likewise).
- `cargo clippy -p tos-api --all-targets`: 118 warnings at HEAD 4f7cbfd7b, 118
  with the change; none in a touched file. Delta 0.
- `cargo test -p tos-api --test gemdos_ssystem_sysinfo --test gemdos_ssystem_cpuinfo`:
  9/0/0 and 5/0/0.
- Full `cargo xtask test`: exit 0, 2459 passed / 0 failed / 7 ignored over 210 test binaries (sum of the `test result` lines).

## Tester handoff

- Host: the two test files above (run by `cargo xtask test`).
- Boards: no existing selftest, case table or program reads 2023 yet (only
  `tosdump/probes/wfprobe.c` reads 2013/2014). The board check needs an EL0
  call `Ssystem(2023, 0, 0)` (and the pointer form) after the desktop is up,
  e.g. the PITEST03 framebuffer probe or a one-off probe in the tester's run
  method directory. Expected:
  - QEMU `raspi4b` (`board-rpi4`): the mailbox pitch (`pitch / 4 * 4`), at the
    default 640-wide mode 2560; must equal `Ssystem(2013) * 4` only if the
    firmware pads nothing — compare against `Physbase`-side facts, not against
    width.
  - `qemu-virt`: `width * 4` (ramfb stride), i.e. `Ssystem(2013) * 4`.
  - `Ssystem(2023, 0, 1)` → `EINVAL` (-25); `Ssystem(2024, 0, 0)` → `EINVFN` (-32).
- Real-Pi4 only: the value at the card's HDMI mode, recorded in the PITEST03 Pi run.

## Proposed docs/STATUS.md line

2026-11-12 — PITEST04: `Ssystem` mode 2023 `S_NEWTOS_SCRPITCH` returns the
visible framebuffer's bytes per line (`ScreenInfo.stride_bytes`), `ERROR` without
a display; first free NewTOS number now 2024. Commit <hash on landing>.

## Out-of-scope observations

- `docs/analysis/deps/mapping.tsv` has no `Ssystem` row; none added: the
  2000+ modes are NewTOS-own, no EmuTOS function behind them.
- `cargo xtask build` fails in `programs/build.sh` (hello-gem `libhello_gem.a`
  not found) when `CARGO_TARGET_DIR` points outside the worktree; it builds with the default.
- `rustfmt --check` run standalone on `tos-api/src/gemdos/ssystem.rs` reports
  diffs in lines this change does not touch (pre-existing formatting).

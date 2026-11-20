# XBIOS11 — receipt (developer, 2026-11-20)

Role: developer
Agent: developer-acc781cc44394cbbc

Branch `xbios11` from `main` 9e0481fb7 (the commit carries main's `TASK/XBIOS11.md` of 1021a7eee, the
lead's note, as its base), worktree `target-agents/agent-acc781cc44394cbbc/wt`. The work was stopped on
the owner's word before the commit and resumed unchanged.

Changes:

- `tos-api/src/xbios/system.rs:24@random`: `random` keeps its static state and runs one step
  `random_step(state, crate::bios::tick::hz_200())`; the frame-clock seed and the bump of a zero seed
  to 1 are gone. The doc comment states the EmuTOS behaviour (seed on every zero state, no
  substitution, result width). Its block keeps its old 17 lines, so no line cite into the rest of the
  file shifts.
- `tos-api/src/xbios/system.rs:297@random_step` (new, private, at the end of the file before the
  tests): `if s == 0 { s = (hz << 16).wrapping_add(hz) }`, `s = s * 3141592621 + 1`, result
  `(s >> 8) & 0xFFFFFF` as a non-negative `i64` (EmuTOS `references/emutos/bios/xbios.c:421-430`).
- Unit tests `tos-api/src/xbios/system.rs:309@tests` (6): seed from the counter (including the
  oracle's measured pair `hz_200` 2573 -> `7d9272`, run
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-EMUTOS-tt/XSSEED.LOG`), 32-bit wrap of
  the seed, zero seed kept (state 1, result 0), a non-zero state ignores the counter, the one state
  that steps onto 0 is re-seeded from the counter of the next call, 24-bit result over 10000 steps.
- New host test `tos-api/tests/xbios11_random_seed.rs`: `bios::tick::advance(2573)`, then `Random`
  through `xbios::dispatch` gives `7d9272`; the second call steps on without re-seeding.
- Permanent probe (CLAUDE.md, Testing, probe rule): `tosdump/probes/xsseed.c` (program text of the
  audit's `measurements/measurement-2026-10-28-xbios-system-audit/method/xsseed.c` unchanged, only the header comment rewritten), `tosdump/build-xsseed-newtos.sh`
  (the link path of `build-rsprobe-newtos.sh`), Makefile targets `xsseed-m68k` / `xsseed-newtos`,
  runner `tosdump/probes/run-xsseed.sh emutos tt|falcontc <out> | newtos <out> <kernel8.img>`
  (after `run-rsprobe.sh`; end condition the `X01 info` line / the shell's exit line; verdict = the
  `X01 first` line).
- `docs/api/xbios/11-random.md`: "Current code" rewritten for the fixed code, required behaviour 4's
  cite moved to `random_step`, the `H` choice recorded (kernel counter), the acceptance-probe section
  points at the permanent probe and its runner, the two seed gaps and the open point on
  `measurements/`-only removed.
- `docs/analysis/deps/mapping.tsv` row `random` (`tos-api/src/xbios/system.rs`, `random`, mapped): still
  current, not edited (`random_step` has no EmuTOS counterpart of its own). `docs/COMPATIBILITY.md` and
  `docs/DEVIATIONS.md` have no `Random` or seed line: nothing stale, not edited.
- `TASK/XBIOS11.md`: line 3 only.

Spec satisfied: `docs/api/xbios/11-random.md`, "Pi / BCM2711 binding", "Required behaviour" 1-4 and
"Which `_hz_200` is `H`" (kernel counter). No host-trait change; no other function of `system.rs`
changed; no case table or frozen reference touched.

Build / clippy / test (worktree, default target dir `wt/target`; baseline = the same commands at
9e0481fb7 before any edit):

- `cargo xtask build --board rpi4`: OK, 159 `warning` lines; baseline 159. Delta 0.
- `cargo xtask build --board qemu-virt`: OK, 159; baseline 159. Delta 0.
- `cargo clippy -p kernel --features board-rpi4`: 196 `warning` lines, baseline 196;
  `--features board-qemu-virt`: 196 / 196. `cargo clippy -p tos-api --all-targets`: 118 / 118 (the
  only line differences are cargo's "(n duplicates)" attribution). No warning in `xbios/system.rs`
  or the new test. New = 0.
- `cargo test -p tos-api --lib xbios::system`: 6 passed. `cargo test -p tos-api --test
  xbios11_random_seed`: 1 passed.
- `cargo xtask test` (full): exit 0, 2587 passed / 0 failed / 7 ignored over 223 test results
  (XBIOS09 at 9ffcce079 recorded 2580 / 0 / 7 over 222; +7 = the six unit tests and the new test binary).
- `cargo xtask doc-check docs/api/xbios/11-random.md tosdump/probes/xsseed.c`: 0 failures. The other
  docs citing `xbios/system.rs:` (31 files): no failure on a `system.rs` cite; their other failures
  are the old backlog on lines this commit does not touch.
- `make -C tosdump xsseed-m68k xsseed-newtos`: both build (`-Wall -Wextra -Werror`), NAP 0 undefined
  symbols.

Smoke runs (developer, not evidence, outputs in the gitignored
subdirectory smoke of this agent's target-agents directory, not committed): `run-xsseed.sh emutos tt`: `X01 first ... : yes` (window 2572..2572, `dc248c`);
`run-xsseed.sh newtos` with the debug rpi4 `kernel8.img` of this change (sha256
`17684795f323d22af2561023afce093f9be1270a60d5f90561f09fdcd037e9d5`): `X01 first ... : yes` (window
3035..3035, `d1e7d7`), card manifest `optional_missing none`. Probes XSSEED.PRG
`4358c6a0752d80a05bd90124eec7ee2c1e4c5cf5a26dafc02e44ea121fc29c4b`, XSSEED.NAP
`63188df813b876e53536a78a733fe07af61954bbc4a64f6c68f2f53f5b90dfd7`.

## Tester handoff

- Host: `cargo test -p tos-api --lib xbios::system` (6) and `cargo test -p tos-api --test
  xbios11_random_seed` (1). Expected: pass.
- Acceptance probe X01 on both sides: `tosdump/probes/run-xsseed.sh emutos tt <out>` (and `falcontc`)
  and `tosdump/probes/run-xsseed.sh newtos <out> <kernel8.img>` with an rpi4 kernel built with
  `--features bios-arg-trace` (the input trace the probe rule asks for; the runner leaves it in
  `BOOT-SERIAL.txt`). Expected: `X01 first Random() matches a seed (hz_200<<16)+hz_200 for hz_200 in
  the window: yes` on every side; the `X01 info` line differs by design.
- Companion cases: `xsprobe.c` R01-R03 and P08 (only in the audit run, built by
  `measurements/measurement-2026-10-28-xbios-system-audit/method/build-xsprobe.sh` and run by `measurements/measurement-2026-10-28-xbios-system-audit/method/run-emutos.sh` /
  `measurements/measurement-2026-10-28-xbios-system-audit/method/run-newtos.sh`). Expected:
  equal to the oracle as before (the generator is unchanged; P08 checks that `Random`, Protobt's random
  serial and `Random` form one LCG chain, R01-R03 range and chain; their "info" lines are not compared).
- Tables on raspi4b via `cargo xtask tosdump`: `bios-r1.tsv`, `bios-core-r1.tsv`,
  `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv`. Expected: no case equal before differs newly.
- Real-Pi-only: nothing; the counter is the board-independent `bios::tick` (on the Pi its rate comes
  from the timer IRQ, as in QEMU).

## Proposed docs/STATUS.md line

- 2026-11-20 — XBIOS11: XBIOS `Random` seeds from the 200 Hz counter `_hz_200` on every call that
  finds the state 0 and keeps a zero seed, as EmuTOS `random()` (no more 60 Hz frame-clock seed, no
  bump to 1); host tests in `xbios/system.rs` and `xbios11_random_seed`; `xsseed.c` is a permanent
  probe (`tosdump/probes/run-xsseed.sh`). Commit: <landed hash>.

## Out-of-scope observations

- `tosdump/probes/run-xsseed.sh` follows `run-rsprobe.sh`; a tosdiff table form of X01 needs the first
  `Random` before any other call, which a table run cannot guarantee: the follow-up ticket of the
  probe rule is the lead's.
- `cargo xtask doc-check` over the 31 docs citing `xbios/system.rs:` reports 142 old failures (e.g.
  `TASK/PORT06-KEYS-DIAGNOSIS.MD`, `TASK/PORT02.md`), none on a touched line.

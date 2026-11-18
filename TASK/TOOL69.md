# TOOL69 — worktree-setup links every gitignored input the worktree's checks read

Status: OPEN (2026-11-17) | Priority: P3

Title: worktree-setup links every gitignored input the worktree's checks read

Type: tool. Owner decision 2026-11-17, from the side findings of TASK/SND05.md
(developer receipt `TASK/SND05-STATUS-UNFINISHED.MD`, "out-of-scope
observations"; tester run `measurements/measurement-2026-11-17-snd05-test/RUNINFO.TXT`,
"TREES" and "OUT-OF-SCOPE OBSERVATIONS"). Goes behind the product fixes; it
blocks no acceptance (both workers got through by linking by hand).

## Finding

`cargo xtask worktree-setup` (TASK/TOOL44.md) links `references/*` and
`sdk/apps/<app>/` into a fresh worktree, so that `cargo xtask test` runs the
main tree's set. Two other checks a worker runs in its worktree read
gitignored inputs it does not link:

- **`sdk/run-host-tests.sh`.** In a worktree it gives 37 passed / 1 failed /
  0 skipped where the main tree's baseline is 38 / 0 / 0 (`docs/STATUS.md`,
  *Baselines*). The failing case is `rt-host` ("compiler-rt rebuild differs
  from the shipped member"). It fails identically on `main` `a5a4665cb` in a
  worktree, so it is not a change of SND05. The tester linked by hand:
  `sdk/ldg/ldg-r133`, `sdk/cflib/cflib-21`, `sdk/zview/z-tools-*`,
  `sdk/libm/libm.a`, `sdk/compiler-rt/{libclang_rt.builtins.a,build,compiler-rt-*.src}`,
  `sdk/libc/mintlib`, `sdk/libshare/libshare-0.1.1`, `Applications`. Not
  established: whether `rt-host` fails because an input is still missing,
  because a linked build directory carries the main tree's absolute paths, or
  for another reason. The developer's run on the same day counted
  33 / 0 / 5 skipped, so the result also depends on what is linked.
- **`cargo xtask doc-check`.** Cites of gitignored paths (`target-agents/`,
  `Applications/`, the SDK source trees above) count as missing in a
  worktree. The SND05 merge commit needed an empty `target-agents/` in the
  worktree for three cites in `TASK/A41.md`, `TASK/TOOL67.md` and
  `docs/STATUS.md`.

## Scope

1. Establish why `rt-host` fails in a worktree (cause first, with evidence;
   no fix before it is known).
2. `worktree-setup` links, or `--check` lists as missing, every gitignored
   input that `sdk/run-host-tests.sh` and `cargo xtask doc-check` read, so
   both give in a fresh worktree what they give in the main tree at the same
   commit.
3. No link may let a worktree write into the main tree's build outputs; a
   directory a test rebuilds into is copied or built in the worktree, not
   linked. The EmuTOS trees stay untouched (pin lock).

## Acceptance

- In a fresh worktree of `main` after `cargo xtask worktree-setup`:
  `sdk/run-host-tests.sh` gives the main tree's count with 0 failed, and
  `cargo xtask doc-check` on the tracked docs reports no failure that the
  main tree does not report.
- `cargo xtask worktree-setup --check` names every input still missing.
- After the two checks ran in the worktree, `git status` and the sha256 of
  the linked gitignored outputs in the main tree are unchanged.
- `cargo xtask test` green; warnings delta 0.

## Not in this ticket

- `selftest_cookie` (`kernel/src/selftest_cookie.rs`) is wired into no boot
  path; the SND05 tester wired it in temporarily. Noted, no ticket (owner
  2026-11-17).
- `cargo xtask land` did not remove the SND05 developer's scratch because
  the receipt's `Agent:` line was not a bare agent id; removed by hand with
  `cargo xtask scratch-clean`. Noted, no ticket (owner 2026-11-17).
- `cargo xtask build` fails when `CARGO_TARGET_DIR` is set: the final
  `kernel8.img` step looks for the kernel ELF under the default target
  directory ("kernel ELF not found"), and `programs/build.sh` does not follow
  the variable either. Seen by the V63 developer and the V63 tester
  (`measurements/measurement-2026-11-18-v63-test/RUNINFO.TXT`, "OUT-OF-SCOPE
  OBSERVATIONS"), who converted the ELF by hand. Noted, no ticket yet.

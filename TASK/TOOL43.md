# TOOL43 — server-side backstop: checks on every push to main (GitHub Actions)

Status: OPEN (2026-11-11) | Priority: P1

Type: tool (enforcement), research first. Owner decision 2026-11-11: local guards are pattern matches and can never be
complete ([TOOL42](TOOL42.md) documents what it cannot see: `eval`, scripts, `fetch` + `reset`, `branch -f`, `rebase`,
`cherry-pick`); the backstop is a check that runs on the server for every push to `main`. **No workflow file is
committed before the owner's OK on the report of step 1.**

## Work

1. **Report first** (`docs/analysis/ci-backstop-<date>.md`), answering with evidence, not assumption:
   - Which checks can run on a GitHub-hosted **Linux** runner, which need **macOS**, which cannot run there at all:
     `cargo xtask doc-check` (whole tree and `--base <rev>` for the pushed range), `cargo xtask deps --check-mapping`,
     `cargo xtask test` (per test target), later `cargo xtask status-check` ([TOOL41](TOOL41.md)).
   - Inputs each needs that are not in the repository: `references/` (EmuTOS checkout at the pin, FreeMiNT, other
     gitignored trees: `.gitignore`, `references/README.md`), the untracked `sdk/apps/<app>/` checkouts
     (`sdk/.gitignore`), `/opt/cross-mint`, cflow 1.8 (`tools/oracle/oracle.env`), the pinned Rust toolchain
     (`rust-toolchain.toml`), host tools named in CLAUDE.md *Required host tools*. For each: can it be fetched or built
     on the runner reproducibly (pinned commit or sha256), and is it allowed to be (licences, private material)?
   - Which tests fail or skip without those inputs (count per target), and whether a test can tell "input absent" from
     "failure" today.
   - Runtime per job, runner minutes per push, cost for a public repository vs a private one (GitHub's published
     pricing, cited with date), caching options (cargo, references).
   - What a red run does: it cannot reject a push that already landed; options (required checks on a protected
     `main` with pull requests, or a post-push alarm), and what each means for the owner's direct pushes.
2. **After the owner's OK only:** the workflow file(s) under `.github/workflows/`, pinned action versions, and a test
   push that shows a deliberately broken doc cite turning the run red.

## Owner decision (2026-11-11) on the step 1 report

Step 1 report: `docs/analysis/ci-backstop-2026-11-11.md`. Variant **A**: a GitHub Actions workflow on every push to
`main`, Linux runner, alarm only, no branch protection. `tuxie01` is a user account and the repository is public, so
standard runners are free. Checks: doc-check (new errors only, over the pushed range), `deps --check-mapping`,
`status-check`, `cargo xtask test`. The range comes from [TOOL48](TOOL48.md) (`--range`, no `git reset --soft`
workaround); the tests that need gitignored inputs skip only under `NEWTOS_CI=1` ([TOOL47](TOOL47.md)). Notification:
GitHub's default e-mail on a failed run to the pusher, no extra setup. The workflow file is shown to the owner before
it is committed. Step 2 starts after TOOL47 and TOOL48.

## Acceptance

- Step 1: the report, every claim with a source (command output, file, GitHub documentation with date); a
  recommendation with the owner's choices spelled out.
- Step 2 (later, separate OK): a green run on `main` and a red run on a test branch with a known defect.

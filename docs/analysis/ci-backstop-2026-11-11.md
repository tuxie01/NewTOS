# CI backstop on GitHub Actions: report for TOOL43 step 1 (2026-11-11)

Ticket: `TASK/TOOL43.md` (step 1 only: this report). No workflow file exists or is proposed as committed
here. Measured in a fresh `git clone --no-hardlinks` of HEAD `628d5dac84c8ff93766ca0c9950f18e7f42b1b71`, with
`references/`, `sdk/apps/<app>/` and `Applications/` absent (gitignored), on the owner's Mac (macOS 27.2, aarch64,
Python 3.14.8, rust `nightly-2026-04-15` from `rust-toolchain.toml`). Nothing was measured on Linux or on a GitHub
runner: every "Linux" statement below is a reading of the code or an inference and is marked so.

Clone sizes at the end of the run: the clone 1.7 GB (of which `.git` 301 MB), `target/` 3.5 GB. Both deleted
after this report was written (see the last section).

## 1. What was measured (clean clone, no gitignored inputs)

| Command (run from the clone root) | Result | Wall clock |
|---|---|---|
| `cargo xtask test` | exit 101; 2388 passed, 8 failed, 5 ignored; 206 test binaries ran | 2:40 (cold build, 365 s user, 258 % cpu) |
| `cargo xtask doc-check` | 476 failures, 12095 skipped ("reference tree missing"), exit non-zero | 4.0 s |
| `cargo xtask doc-check --base HEAD~3` | the same 476 failures: `--base` only moves the anchor baseline, it does not restrict to the range | 0.2 s |
| `cargo xtask deps --check-mapping` | `docs/analysis/deps/mapping.tsv: 1967 rows, all resolve, every class is valid; every bios/, bdos/, vdi/, aes/ function has a row`, exit 0 | 0.1 s |
| `cargo xtask status-check` | `686 checked, 0 rejected, 12 reported, 899 legacy`, exit 0 (the 12 are `report` lines, not rejections) | 0.6 s |

The same `cargo xtask doc-check` in the main tree (references present): 762 failures, 0 skipped. So the whole-tree
mode is red with and without references: it carries the legacy backlog named in CLAUDE.md (*Commit Discipline*,
"752 failures" on 2026-10-27). **The whole-tree `doc-check` cannot be a gate.** The gate-capable mode is
`--staged` (fails only for what the commit introduces; `tools/xtask/src/doc_check.rs:1578@fn check`,
`:1688@fn introduced`).

Per test group in the clean clone (counted from the `test result:` lines of the run):

| Group | Targets | Passed | Failed | Ignored |
|---|---|---|---|---|
| `tos-api` unit tests | 1 | 214 | 0 | 0 |
| `tos-api` integration tests (`tests/*.rs`) | 203 | 1904 | 0 | 5 |
| `tos-types` unit tests | 1 | 6 | 0 | 0 |
| `xtask` unit tests | 1 | 264 | 8 | 0 |

The 5 ignored are `#[ignore]`d with a reason in the source (e.g. `cases_v35::v35_probe_tph_handler_writes...`,
TASK/V35 open point), not skipped for a missing input. The run ended at the `xtask` target, which cargo runs
last; the doctests of `tos-api`/`tos-types` (`tools/xtask/src/test_tiers.rs:37@DOCTEST_PACKAGES`) were therefore
not reached after the failure. By the comment at `test_tiers.rs:42@DOCTESTS_WHY` they cover "one `ignore`d
block", so no passed test is lost by that, but a CI run should use `--no-fail-fast` to see everything.

### The 8 failures, with the missing input

| Test | Missing input | Evidence |
|---|---|---|
| `rsc_obfix_tests::mkrsc_convert_ilp32_keeps_the_char_grid_coordinates` | `references/emutos/aes/gem.rsc` | `.expect("references/emutos/aes/gem.rsc must exist")`, `tools/xtask/src/rsc_obfix_tests.rs:179@check_mode` |
| `rsc_obfix_tests::mkrsc_convert_lp64_keeps_the_char_grid_coordinates` | same | same line |
| `rsc_obfix_tests::the_committed_gem_fsel_blob_carries_char_grid_coordinates` | same | `tools/xtask/src/rsc_obfix_tests.rs:203@the_committed_gem_fsel_blob_carries_char_grid_coordinates` |
| `rsc_cicon_tests::an_old_format_file_declares_no_colour_icons` | same (`gem.rsc` as mkrsc input) | `tools/xtask/src/rsc_cicon_tests.rs:447@an_old_format_file_declares_no_colour_icons`, `assert!(o.status.success())` at `:450` |
| `rsc_recolour_tests::recolour_palettes_equal_their_sources` | `references/freemint/xaaes/src.km/pal/nvdi.pal` (and `references/emutos/vdi/vdi_col.c`) | `tools/xtask/src/rsc_recolour_tests.rs:209@recolour_palettes_equal_their_sources` (and `emutos_palette()`) |
| `rsc_cicon_tests::teradesk_cicons_rsc_converts_and_fits_rsc_cap` | `sdk/apps/teradesk/cicons.rsc` | python traceback `FileNotFoundError ... sdk/apps/teradesk/cicons.rsc` |
| `rsc_recolour_tests::hypview_rsc_8plane_icons_take_the_nearest_tos_colour` | `sdk/apps/hypview/hyp_view.rsc` | python traceback `FileNotFoundError ... sdk/apps/hypview/hyp_view.rsc` |
| `doc_check::tests::resolves_against_the_workspace` | `references/` (inferred) | `tools/xtask/src/doc_check.rs:2084@Outcome` asserts `Outcome::Fail("missing")` for a nonexistent top-level kernel module path "even without references/"; it failed in the clone. I did not print the value it got and did not analyse why: **needs deeper analysis** (a latent defect of this test when references are absent) |

### Can a test tell "input absent" from "failure"? Today: no, with one exception

Read from the test code, not assumed:

- The seven `rsc_*` tests `.expect(...)` / `unwrap()` / assert on the missing file. A missing input is a panic, the
  same red as a real regression. No skip, no distinct exit.
- One test does distinguish: `tools/xtask/src/doc_check.rs:2519@emutos_resolves_at_the_pin` prints
  `references/emutos or a commit absent, skipped` and returns, i.e. it passes silently when the input is absent.
  Two more doc_check tests accept `Outcome::Skipped` as a pass when the tree is missing (in `doc_check.rs`, near `emutos_resolves_at_the_pin`).
- `doc-check` itself separates the cases: it counts "skipped (reference tree missing)" (`tools/xtask/src/doc_check.rs:1670@skipped`) apart
  from failures. Whole-tree it returns non-zero on the 476 failures regardless.
- Consequence for a CI design: either the runner provides the inputs (below), or the 7 tests need a
  "skip when absent, fail when present and wrong" guard. That is a code change (Developer), not part of this report.
  With inputs present the tests pass today (the main tree run of the owner is the proof only by the owner's
  routine; I did not re-run the xtask tests in the main tree in this task).

## 2. Which check needs what

| Check | Inputs beyond the tracked tree | Fits a GitHub-hosted Linux runner? |
|---|---|---|
| `cargo xtask test`, groups `tos-api`, `tos-types` (2124 tests) | rust `nightly-2026-04-15` + components, 11 crates.io packages (`Cargo.lock`: 11 `registry+https` entries) | Yes by reading: pure host Rust, no `references/`, no darwin API. **Not measured on Linux** |
| `cargo xtask test`, group `xtask` (264 tests, 8 need inputs) | `python3` (the rsc tests run `tools/mkrsc.py` and `recolour-cicons.py`, `PYTHON` env, default `python3`: `rsc_obfix_tests.rs`, `tools/xtask/src/gen_deps_tests.rs:48@python`), `git`, `references/emutos` (4 tests gem.rsc, 1 of them also `vdi_col.c`), `references/freemint` (1 test), `sdk/apps/hypview`, `sdk/apps/teradesk` (1 test each) | Yes if the inputs are provided; `grep -rln "darwin\|macos\|sw_vers\|/opt/homebrew\|hdiutil\|diskutil\|osascript" tools/xtask/src tools/git-hooks tools/hooks` found nothing, so xtask has no darwin-specific code path. `/Volumes/...` occurs only as test string data (`tools/xtask/src/tosdump.rs:1693@foreign_dep_info`). Linux behaviour unmeasured |
| `cargo xtask doc-check` (whole tree) | `references/emutos`, `references/freemint` (cite targets, `doc_check.rs:152@REFERENCE_ROOTS`), `references/emutos-oracle` (`:162@EMUTOS_BUILD_ROOT`), `references/libshare` (`:282`) | Runnable, but red by design (backlog) and 12095 cites skip without the trees. Not a gate |
| `cargo xtask doc-check --staged <docs>` | the docs list as arguments (the pre-commit hook passes them); the index | Gate-capable. Range emulation on a runner is an **inference**: `git reset --soft <old-sha>` after checking out the new sha makes the index equal the pushed range. I confirmed the mechanics for `deps --check-mapping --staged` (`0 problems the commit introduces`) and `status-check --staged --new-only` (`0 checked`) on a 3-commit and a 40-commit range; I could not confirm `doc-check --staged <files>` in the nested clone (`xtask: doc-check --staged: target/agents/tool43/clone/CLAUDE.md: not in the index`: the clone sat inside the main repo's directory, so the path handling saw the outer prefix; a runner checkout is not nested, but this is unverified). Step 2 must prove it |
| `cargo xtask deps --check-mapping` | none (reads committed `docs/analysis/deps/edges.tsv`, `mapping.tsv`, NewTOS files, ADR lines) | Yes, 0.1 s, measured exit 0 without any reference tree. Does **not** need cflow |
| `cargo xtask deps --check-mapping --staged` | the index | Yes (range as above) |
| `cargo xtask status-check` (whole tree) | none; legacy baseline `tools/xtask/ticket-baseline.tsv` | Yes, exit 0 measured. `--staged --new-only` is the commit-introduced mode (`TASK/TOOL41.md`) |
| `tools/deps/gen-deps.py` (regenerates edges.tsv) | GNU cflow 1.8 at `ORACLE_CFLOW=/opt/homebrew/bin/cflow` (`tools/oracle/oracle.env`), EmuTOS at the pin | Not a gate (it regenerates, it does not check); would need `apt install cflow` and the path in `oracle.env` is a Homebrew path. Out of scope for the backstop |
| tosdump / tosdiff / Hatari / oracle ROM runs | m68k cross toolchain `/opt/cross-mint`, Hatari 2.6.1 macOS bundle (`oracle.env`: `ORACLE_HATARI=/Volumes/Daten/hatari-2.6.1_macos/...`), QEMU raspi4b | Cannot run on a stock runner: macOS-bundle path, `/Volumes` paths, GUI emulator. Not proposed |
| `cargo xtask build --board rpi4` (kernel8.img) | aarch64 target + `rust-src`, `llvm-tools-preview` (`rust-toolchain.toml`) | Probably builds on Linux (cross-compile, `.cargo/config.toml`); **not measured**, not requested by the ticket |

Nothing in the ticket's list needs macOS. macOS would be required only for the Hatari oracle runs, and those need
the owner's `/Volumes` bundle, so they are not a backstop candidate.

## 3. Inputs that are not in the repository: fetchable, reproducible, allowed?

| Input | Pin (where) | Fetch on a runner | Licence / allowed |
|---|---|---|---|
| `references/emutos` | `1a7484aba1bfba23e2c0899e243b6d8177fe41df` (`tools/oracle/oracle.env`: `ORACLE_EMUTOS_COMMIT`; the local checkout `git rev-parse HEAD` returns the same sha; `git status` shows only `.DS_Store`) | `https://github.com/emutos/emutos` (`references/README.md`, `git remote -v`). A fetch of one pinned sha is an **inference** (GitHub normally serves reachable commit ids; test in step 2) | GPL-2: `references/emutos/doc/license.txt` starts "GNU GENERAL PUBLIC LICENSE Version 2, June 1991". Public upstream. Tests need only `aes/gem.rsc` and `vdi/vdi_col.c`, both tracked upstream (`git ls-files`) |
| `references/freemint` | `f906416a9387ed68876c738cfc5c30037b294fc0` (local `rev-parse`; README says `f906416`) | `https://github.com/freemint/freemint` (remote). 51 MB checkout locally | `COPYING`, `COPYING.GPL`, `COPYING.LGPL`, `COPYING.MiNT` in the tree: GPL/LGPL mix, public upstream. Tests need only `xaaes/src.km/pal/nvdi.pal` (tracked upstream) |
| `references/emutos-oracle/<commit>` | oracle ROM rebuild; the ROM itself is committed (`tools/oracle/etos512de-1a7484ab.img`, sha256 in `oracle.env`) | Needs the m68k cross toolchain; only `doc-check` for build-product cites uses it (skip when absent, `tools/xtask/src/doc_check.rs:1265@any_reference_missing`) | Not needed for the gates |
| `references/tos4x`, `magicmac`, `fvdi`, `xgdos`, `libshare`, PDFs (`compendium`, `profibuch`, ...) | various, `references/README.md` | **Must not be fetched or published**: `references/README.md` says "copyrighted/external sources ... nothing in this directory is redistributable as part of the NewTOS repo"; tos4x has no licence file seen (README), magicmac is GPL-3. No gate needs them; cites into them skip | Not allowed / not needed |
| `sdk/apps/hypview/` | commit `6f3eeeb4471530aaa088eb5b158b75d0e218456b`, archive sha256 in `sdk/apps/hypview-sources.sha256` (with the `curl` recipe) | Reproducible by sha256 | GPL-2.0-or-later (same file) |
| `sdk/apps/teradesk/` | **none**: `sdk/apps/build-teradesk.sh` names "tho-otto/aRTS TeraDesk 4.08 ... embedded git checkout, freemint/teradesk"; there is no `*-sources.sha256` for it in `sdk/apps/` (only `coremark-sources.md5`, `hypview-sources.sha256`) | **Not reproducible today** (no pinned commit or hash). Needed by 1 test (`teradesk_cicons_rsc_converts_and_fits_rsc_cap`) | Licence not checked here; `sdk/apps/teradesk` is a git checkout of `freemint/teradesk` (`git remote -v`) |
| `/opt/cross-mint` | `oracle.env`: MiNT 20240501 gcc 7.5.0 | Not needed by any gate | n/a |
| cflow 1.8 | `oracle.env` | Not needed by any gate | n/a |
| Rust toolchain | `nightly-2026-04-15` + `rust-src rustfmt clippy llvm-tools-preview`, target `aarch64-unknown-none-softfloat` (`rust-toolchain.toml`) | rustup fetches the pinned date reproducibly (rustup installs from the toml in the repo root) | MIT/Apache |
| crates.io deps | `Cargo.lock` (11 registry entries) | `cargo` with the lockfile | per crate, not checked here |

Alternative to fetching the trees: the tests need exactly four files from `references/` and two from `sdk/apps/`;
the cleaner remedy is a skip-when-absent guard in the tests (section 1), which is Developer work.

## 4. Runtime, minutes, cost, caching

Local wall clock is a **proxy** (Apple Silicon, 258 % cpu), not a runner measurement.

| Job | Local wall clock | Note |
|---|---|---|
| `cargo xtask test` cold (compile + 206 test binaries) | 2:40 | `tools/xtask/src/test_tiers.rs:18@FULL_ONLY`: the integration targets themselves run in seconds (162 binaries in 3.8 s on 2026-10-28, the xtask target 16 s in this run); compile dominates |
| `doc-check --staged`, `deps --check-mapping`, `status-check` | 0.1-1 s each after xtask is built | building xtask is part of the 2:40 |
| xtask cold build only | not separated | |

Expected on a GitHub runner (an estimate, not measured): a cold build on 4 vCPU Linux plausibly 4-10 minutes,
a few minutes with a warm `target/` cache. I have no data for that and will not state a number as fact; step 2
measures the first run.

Pricing and runners (GitHub documentation, read 2026-11-11):

- "About billing for GitHub Actions" (`https://docs.github.com/en/billing/managing-billing-for-your-products/managing-billing-for-github-actions/about-billing-for-github-actions`):
  standard GitHub-hosted runners are free in **public** repositories and do not reduce included usage; larger
  runners are billed even in public repos. Private repos: included compute is $12 (Free), $18 (Pro/Team) per
  month; Linux 2-core x64 $0.006/min, macOS 3-4-core $0.062/min (about 10x Linux), Windows 2-core $0.010/min. The page
  states no multiplier table and mentions no 2026 price change.
- "GitHub-hosted runners" (`https://docs.github.com/en/actions/reference/runners/github-hosted-runners`):
  `ubuntu-latest` is 4 vCPU / 16 GB / 14 GB SSD in a public repo, 2 vCPU / 8 GB in a private one;
  `macos-latest` is 3 vCPU (M1) / 7 GB in both. The page does not state which Ubuntu `ubuntu-latest` is (it links the
  Ubuntu 24.04 image); a workflow should name the image explicitly.
- Scheduled workflows in a public repo are disabled after 60 days without repository activity (events page,
  `https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows`, read
  2026-11-11); a `push` trigger is not affected.

Minutes per push (inference from the table above): one Linux job of roughly 3-10 minutes per push to `main`. In the
public repo that costs nothing in money. If the repo went private: at $0.006/min and $12/month the Free allowance
is about 2000 Linux minutes (the page's figure, divided out), i.e. roughly 200-600 pushes a month at 3-10 min;
macOS would use the same allowance about 10x faster and is not needed.

Caching: the `target/` build (a cold build is the whole 2:40) is the only expensive part. Options from reading, not
tried: cache `~/.cargo/registry` and `target/` keyed on `Cargo.lock` + `rust-toolchain.toml`. References need no
cache: a shallow fetch of four files' worth is small (`references/emutos` 26 MB, `references/freemint` 51 MB as
checked out locally). The cache action's version pin and quota are step-2 questions, not read here.

## 5. What a red run can and cannot do

A push to `main` is already in the remote when the workflow starts (the `push` event fires after the push; the
events page says the workflow runs "when you push a commit"). A red run therefore cannot reject it. Options:

| Option | Mechanism (docs read 2026-11-11, `https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches`) | Consequence for the owner who pushes directly to `main` |
|---|---|---|
| A. Post-push alarm only | Workflow on `push` to `main`: red run, e-mail/notification to whoever GitHub notifies (the events page names notification recipients only for scheduled runs; who is notified for a failed `push` run is **not stated** in the pages I read, check in the repo's notification settings), optional step that opens an issue | No change to the workflow of the owner. Damage is detected after the fact; fixing is a follow-up commit. The backstop for the TOOL42 gaps works only if someone reads the alarm |
| B. Protected `main` + required status checks + pull requests | Branch protection: required status checks must pass before merge; required PRs mean collaborators push only through a PR. Rules do **not** apply to repo admins by default; "Do not allow bypassing the above settings" applies them to admins too. "Bypass lists can only be used in organization-owned repositories" (same page); the origin is `https://github.com/tuxie01/NewTOS.git`, whether `tuxie01` is a user or an organisation is not shown by any command I ran | With bypass off, **no** direct push to `main`, also not by the owner: every change via a branch + PR + green check, merges by squash/rebase. This is the only option that makes "a red run cannot reject a push" untrue. It also blocks force-push (blocked by default, same page). A fix for a broken `main` goes through a PR too. A ticket that changes the workflow itself cannot be merged by a red check, so the owner needs a documented emergency path (temporarily disable the rule in settings) |
| C. As B but admin bypass left on (the default) | Same protection for others; the owner as admin can still push directly, GitHub shows the bypass | The owner's direct pushes keep working and are not stopped by anything: this is exactly the hole TOOL43 is meant to close for the owner's own agents. Combined with A it gives alarm + optional PRs |
| D. Rulesets | The about-rulesets page I read says rulesets apply to "GitHub Team and GitHub Enterprise plans"; it does not say whether public repos on the Free plan can use them, and it does not list required status checks or an "evaluate" mode. **Unverified**: check in the repo's Settings before relying on it | Same effect as B/C if available; "push rulesets" only restrict paths/sizes (page), not test results |

Notes for the choice:

- B with bypass off also stops the agents' direct pushes, because agents push as the owner's credentials. That is the
  intent ("the backstop is a check that runs on the server for every push to `main`") but it changes the daily
  routine: one PR per ticket (CLAUDE.md says one commit per ticket via `git merge --squash`; a PR merged with
  "squash" gives the same shape, but the hooks in `tools/git-hooks` then run on the branch commit, not on the
  merge commit, and the server check is the second look).
- The doc/status checks must run in the PR's range mode (`--staged` emulation by soft reset, or a new
  range mode in xtask: Developer work) because the whole-tree modes are red by backlog (sections 1-2).
- Today a red run is expected for: whole-tree `doc-check` (476/762 failures), and 8 xtask tests unless inputs or
  guards are added. A workflow that just runs `cargo xtask test` as is would be red on its first run.

## 6. Recommendation

Smallest design that closes the TOOL42 gap without red noise (my judgement, for the owner to decide):

1. One Linux job (`ubuntu-24.04`, pinned), triggers `push` to `main` and `pull_request`, with three steps whose
   outcome is green on today's `main`: `cargo xtask test --no-fail-fast` limited to the `tos-api` and `tos-types`
   packages (2124 tests, 0 failures measured), `cargo xtask deps --check-mapping`, and the range-mode
   `doc-check`/`status-check --staged` emulation. This needs no references.
2. The 8 xtask tests that need inputs: either a Developer ticket to give them a skip-when-absent guard (kept red when
   the input is present and wrong), or the job fetches `references/emutos` and `references/freemint` at their pins
   plus hypview by sha256; teradesk has no pin and would first need one. I recommend the guard (no third-party
   trees on a server, nothing to keep pinned) plus a separate "full" run on the owner's Mac as today.
3. Alarm first (option A) on the owner's OK, to prove the job is green and stable for a few days; then decide on
   protection (B or C) with the owner's choice below.

### Owner's choices

1. Gate or alarm: A (alarm after the push), B (protected `main`, PRs, no bypass), or C (protected, owner may bypass)?
   Is the owner willing to give up direct pushes to `main` (B), and who is the emergency path if the check itself is
   broken?
2. Is `tuxie01` a user or an organisation account, and does the Free plan offer required status checks / rulesets for
   this public repo (I could not confirm from the pages; check Settings > Branches / Rules)?
3. The 8 failing xtask tests: guard-when-absent (Developer ticket) or fetch the inputs on the runner? If fetch:
   pin TeraDesk first (no pin exists), and confirm fetching EmuTOS (GPL-2) and FreeMiNT (GPL/LGPL, `COPYING*`) pinned
   shas on a CI runner is acceptable.
4. Range semantics for doc/mapping/status checks: soft-reset emulation of `--staged` (works for `deps` and
   `status-check` as measured; `doc-check` unproven) or a new xtask range mode (Developer ticket)?
5. Public repo assumed: standard runners free. Does the owner keep it public (cost 0) or move private (allowance
   $12/month, Linux $0.006/min)?
6. Notification channel for a red run (e-mail to the owner, issue, other).

## Out-of-scope observations

- `doc_check::tests::resolves_against_the_workspace` (`tools/xtask/src/doc_check.rs:2084@Outcome`) failed without references although its
  comment says it must hold without them; cause not analysed here.
- `docs/audits/2026-10-28-bios-storage.md` and several others carry anchored cites that no longer match (visible
  in the 476-line output); legacy backlog per TASK/SPEC03.
- `sdk/apps/teradesk` has no source pin (no `*-sources.sha256`), unlike hypview and coremark.

## Cleanup

The scratch clone (`target/agents/tool43/clone`, HEAD `628d5dac84c8ff93766ca0c9950f18e7f42b1b71`, 1.7 GB; `.git` 301 MB)
and its target dir (3.5 GB) were removed after this report was written (see the hand-back).

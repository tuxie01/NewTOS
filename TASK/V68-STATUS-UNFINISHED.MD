# V68 — receipt (developer): XBIOS answers at both VDI depths, and `--bootargs` for `cargo xtask tosdump`

Role: developer
Agent: a10822410888fee1b

Date 2026-11-18. Branch `v68` from `main` @ `bad47112a` (worktree `target-agents/agent-a10822410888fee1b/wt`).
The two items "left for the developer" in TASK/V68.md, section "Spec gate and documents … and the lead's reading".
No product code in `kernel/`, `tos-api/src`, `tos-types/`; no `mapping.tsv` row touched (no mapped function changed).

## What changed, by file and function

**Item 1, host test** — new `tos-api/tests/xbios_v68_planes_setting.rs`. A test host with a metadata
`RasterBackend` whose `planes()` the test sets (16 or 32, the pattern of `tos-api/tests/vdi_v62_planes_setting.rs`)
beside a 32-bpp framebuffer that can be taken down (the pattern of `tos-api/tests/xbios22_falcon_video.rs`). Before
the calls, `setting()` asserts that the installed host really reports the chosen depth and framebuffer state. At each
setting the test collects every answer of the group in one struct and compares it with the documented values and with
the other setting:

| call | documented answer | source |
|---|---|---|
| `Getrez()` | 2 | `docs/api/xbios/04-getrez.md`, "At the 32-plane setting"; bios-r1 case 87 |
| `VsetMode(-1)` | `0x001C` | `docs/api/xbios/58-vsetmode.md`, "At the 32-plane setting"; case 211 |
| `VsetMode(0)`, `VsetMode(0x1C)` | `0x58` (function number) | same file |
| `VgetMonitor()` | 2 | `docs/api/xbios/59-vgetmonitor.md`; case 212 |
| `VgetSize(0x1C)`, `VgetSize(0)` | `stride_bytes * height` with a framebuffer, `0x5B` without | `docs/api/xbios/5b-vgetsize.md`, "At the 32-plane setting"; case 213 |
| `VgetRGB(0,16)`, `(3,2)`, `(0,17)` | 0 + the case-214 table; 0 + yellow, blue; -1, buffer untouched | `docs/api/xbios/5e-vgetrgb.md`; case 214 |
| `Vfixmode(0x1C)` | `0x5F` (function number) | case 215 |

Tests: `xbios_video_answers_are_the_documented_ones_at_both_settings`,
`without_a_framebuffer_vgetsize_answers_0x5b_at_both_settings`,
`the_mode_word_is_the_falcon_truecolor_word_at_both_settings`. All pass. No answer depends on the depth, so nothing
contradicts ADR-038 point 9; no product code changed.

**Item 2, `cargo xtask tosdump --bootargs <args>`** — `tools/xtask/src/tosdump.rs`:

- `Opts.bootargs: Vec<String>`; `parse_args` takes `--bootargs <value>`, repeatable, through the new
  `parse_bootargs` (split at ASCII whitespace as the kernel splits its command line; refused: a value with no token,
  and any token that starts with `serial=`, so the option can only add, never drop or override `serial=on`).
- New `SERIAL_ON` (`"serial=on"`), `qemu_append(extra)` (= `serial=on`, then the tokens in order, one space apart;
  with no tokens exactly `serial=on`) and `qemu_args(serial, append, kernel, card)` (the QEMU argument vector, the
  same arguments in the same order as the inline call it replaces in `cmd`).
- `tosdump-run.txt` gains a `bootargs` line in every run: `bootargs  serial=on (the default; no --bootargs)` or
  `bootargs  serial=on vdiplanes=32 (--bootargs vdiplanes=32 after the fixed serial=on: NOT the default boot)`; the
  `qemu` line prints the real append string (unchanged text for a default run); the `command` line already records
  the whole command.
- Module doc (call synopsis, step 4) and `USAGE`; `tools/xtask/src/main.rs` help text for `tosdump`.
- Tests: `without_bootargs_the_qemu_call_is_unchanged` (the argument vector of a default run, argument for argument,
  against today's literal list), `bootargs_are_appended_after_serial_on` (one value, several values with mixed
  whitespace, repeated flag; only the `-append` value of the call changes), `bootargs_cannot_touch_serial_on`
  (`serial=off`, `serial=on`, `serial=`, `serial=onx` refused), `bad_command_lines_are_refused` (flag without value,
  empty and whitespace-only value), `relative_paths_resolve_against_the_callers_cwd` (default is empty).

**Cites the line shift broke** (the pre-commit doc-check names them): anchor line numbers moved to where the symbol
now stands in `TASK/MEM12.md`, `TASK/TOOL19.md`, `TASK/TOOL39.md`, `TASK/TOOL47.md`, `TASK/XBIOS17.md`,
`docs/sdk/06-image-and-testing.md` (one number each). `doc-check --fix-anchors` does not exist yet (TASK/TOOL49.md is
OPEN), so these are hand edits and the files count as touched: the German text of `TASK/TOOL19.md` and
`TASK/XBIOS17.md` is translated to English in this commit (content unchanged; the quoted old line 3 stays verbatim).

No markdown doc describes the tosdump command line (`docs/TOS-REFERENCE-DUMPER.md` names it only for `--disk`; the
tools doc is the module doc, `USAGE` and the xtask help, all updated). The command line in `CLAUDE.md` is the lead's,
proposal below.

## Checks (real output)

- `cargo xtask test` (full): **2576 passed / 0 failed / 7 ignored**, exit 0 (baseline 2570 / 0 / 7; +3 in the new
  host test, +3 new xtask tests).
- `cargo xtask build --board qemu-virt`: exit 0; `cargo xtask build --board rpi4`: exit 0 (kernel warnings 93, no
  kernel code changed).
- clippy: `cargo clippy -p xtask --all-targets` 10 warnings before (HEAD's two files restored from a copy) and 10
  after, none in a changed line; `cargo clippy -p tos-api --test xbios_v68_planes_setting`: no warning in the new
  file (lib warnings unchanged, lib untouched). **Delta 0.**
- `cargo xtask doc-check` on `TASK/V68.md` and this receipt (see the commit), `cargo xtask status-check --new-only`: 0
  rejected.

### Both tosdump calls (scratch in this agent's target-agents directory, not the repo)

From the worktree, `R=tosdump/reference/EMUTOS-1a7484ab`, `A=<main repo>/target-agents/agent-a10822410888fee1b`:

```
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out $A/runs/default  --scratch $A/scratch --reference $R-BIOS.BIN --reference $R-BIOS-falcontc.BIN
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out $A/runs/planes32 --scratch $A/scratch --bootargs vdiplanes=32 --reference $R-BIOS.BIN --reference $R-BIOS-falcontc.BIN
```

- Both exit 0, "run complete"; tosdiff against `BIOS` 35 of 147 differ, against `BIOS-falcontc` 31 of 147 differ, the
  same in both runs (the per-case tosdiff texts of the two runs are identical).
- `tosdump-run.txt`: default `bootargs  serial=on (the default; no --bootargs)` and
  `qemu      raspi4b, -append serial=on, time limit 480 s` (the old text); 32 planes
  `bootargs  serial=on vdiplanes=32 (--bootargs vdiplanes=32 after the fixed serial=on: NOT the default boot)` and
  `qemu      raspi4b, -append serial=on vdiplanes=32, time limit 480 s`.
- The kernel took the argument: the 32-plane run's `BOOT-SERIAL.txt` has
  `[vdi] VDI depth: 32 planes (vdiplanes=32, boot argument)`; the default run's has no such line (as
  `kernel/src/lib.rs:1376@apply_vdi_planes` prints it only when given).
- `tosdiff bios-r1.tsv default/BIOS.BIN planes32/BIOS.BIN`: **0 of 147 cases differ** (the raw files differ in 3
  bytes of one record, a field tosdiff does not compare).
- Commit measured: `bad47112a` plus the then-uncommitted change (`dirty 3 path(s)`), same kernel bytes as the commit
  (no kernel source changed).
- Gitignored outputs (deleted with this agent's directory): worktree `target-agents/agent-a10822410888fee1b/wt`
  6.0 G, runs 1.2 M; `kernel8.img` (release) sha256
  `739752b12b80f187e5b48cda666e671edd5b19b3fc0100bab07705effc322e73` (both runs); BIOS.BIN of the default run sha256
  `29164e3b68ad4987129bb0d85293a431efbb8e0667f4b89de9fd02176e0c69c6`; BIOS.BIN of the 32-plane run sha256
  `ad9b0ec99ca384fa2d6f33a3de3628aede1f62fc672a7638361d8f9dcf530d2f`. These runs are a smoke check of the tool,
  not the acceptance run.

## Tester handoff

- Acceptance 1 ("a case run at `vdiplanes=32` shows the XBIOS answers of point 9"): run `tosdump/cases/bios-r1.tsv`
  (cases 87, 211-215) with the two calls above, now without any patch to the xtask; compare the two NewTOS dumps cell
  by cell (`tosdump/host/build/tosdiff bios-r1.tsv <default>/BIOS.BIN <32>/BIOS.BIN`, expected 0 differ) and each
  against `EMUTOS-1a7484ab-BIOS-falcontc.BIN` (case 213 differs by design, TASK/XBIOS22.md). Check that the 32-plane
  run's `tosdump-run.txt` has the `bootargs … vdiplanes=32` line and its serial log the `[vdi] VDI depth: 32 planes`
  line.
- Host: `cargo test -p tos-api --test xbios_v68_planes_setting`, `cargo test -p xtask tosdump`.
- Default unchanged: a run without `--bootargs` prints `-append serial=on` and the same tosdiff counts as main.
- Nothing is real-Pi4-only: the XBIOS answers read no hardware, the option is QEMU-only.

## Proposed `docs/STATUS.md` line

`2026-11-18 — V68: host test shows Getrez/VsetMode/VgetMonitor/VgetSize/VgetRGB/Vfixmode answer alike at 16 and 32
planes (ADR-038 point 9); cargo xtask tosdump gets --bootargs (appended after the fixed serial=on, recorded in
tosdump-run.txt) so a case run at vdiplanes=32 needs no patched xtask. Commit <v68 landing>.`

Proposed `CLAUDE.md` (Testing, the tosdump bullet): `cargo xtask tosdump <dumper> <table.tsv> --out <run dir>
[--reference <BIN>]… (--tree, --disk, --features bios-arg-trace, --bootargs vdiplanes=32; TOOL09)`.

## Out-of-scope observations

- `tos-api/tests/xbios22_falcon_video.rs` and the new test are not rustfmt-clean under the default config (the
  repository has no rustfmt.toml and does not enforce it); left as the existing tests are.

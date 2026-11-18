# V62 — receipt (developer): boot argument `vdiplanes=<16|32>`, `planes()` a boot-time value, the depth answers

Role: developer
Agent: agent-a18c9d2f24deeef22

Date 2026-11-18. Branch `v62` from `main` @ `e52bb97d9` (worktree `target-agents/agent-a18c9d2f24deeef22/wt`).
Spec: `docs/api/vdi/workstation/100-v_opnvwk.md`, section "32-plane setting and the boot argument `vdiplanes`"
(§1-§4), `docs/api/vdi/workstation/102-vq_extnd.md` and `01-v_opnwk.md` ("32-plane setting"),
`docs/api/aes/application.md` (`ap_planes`); contract ADR-038 points 1-3 (`docs/MEMORY.md`).

**Rework 2026-11-18 (after the lead's slice correction).** The first commit `05cc70921` stopped on the spec's
§2 row for the `tos-api` raster constant `CHUNKY_PLANES`. The lead resolved it as the code stood: the constant stays
16 in V62 at both settings and moves to TASK/V65.md (spec corrected on `main` `9849c83a2`, tickets on `main`
`9969cff27`, "Slice cut corrected" in TASK/V62.md). `main` `9969cff27` is merged into `v62` (no conflict, docs and
tickets only); the code was checked against the corrected §2 (below) and needs no product change.

## Changes

- `kernel/src/bootargs.rs`: `VDIPLANES = b"vdiplanes="` (`kernel/src/bootargs.rs:158@VDIPLANES`),
  `VDI_PLANES_DEFAULT = 16`, `VDI_PLANES_WIDE = 32`, and `vdi_planes(args)`
  (`kernel/src/bootargs.rs:174@vdi_planes`): `numeric_setting(args, VDIPLANES, 32)`, then every `Given(n)` other than
  16 and 32 becomes `Invalid` (spec §1 result table, last row; ADR-038 point 2). Host test
  `vdiplanes_accepts_16_and_32_only` (`kernel/src/bootargs.rs:316@vdiplanes_accepts_16_and_32_only`) covers every
  row of the §1 table (absent, 16, 32, empty, `abc`, `32x`, `-16`, 0/1/4/8/15/24/31/33/64/overflow, first token
  decides, other prefix, NUL).
- `kernel/src/vdi_backend.rs`: `PLANES` is an `AtomicU16` initialised to 16
  (`kernel/src/vdi_backend.rs:44@PLANES`), `set_planes` (`:1125@set_planes`), `RasterBackend::planes` loads it
  (`:465@planes`). `NUM_COLORS` stays 256 (spec §3, `DEV_TAB[13]`).
- `kernel/src/lib.rs`: `kernel_main_rust` reads the argument right after `mdlimit` through `boot_switch`
  (`kernel/src/lib.rs:291@apply_vdi_planes`); `apply_vdi_planes` (`:1376@apply_vdi_planes`) sets the backend and prints
  a line only when the token is present: `[vdi] VDI depth: 32 planes (vdiplanes=32, boot argument)` /
  `[vdi] VDI depth: 16 planes (vdiplanes: INVALID boot argument ignored, default)` (wording of the `mdlimit` line,
  spec §1). Absent: no line, so a default boot prints what it printed before.
- `tos-api/src/vdi/workstation.rs`: the `INQ_TAB[5]` gate of `vq_extnd` is `planes == 16 || planes == 32`
  (`tos-api/src/vdi/workstation.rs:331@planes`), spec §3 / `102-vq_extnd.md` ("0 for every truecolor depth (16 and
  32)"); any other plane count (test doubles only) keeps EmuTOS's rule. `DEV_TAB[13]/[35]/[39]` and `INQ_TAB[4]`
  already follow `backend.planes()` per call (spec §3, last paragraph): no change.
- `tos-api/tests/vdi_v62_planes_setting.rs` (new, 5 tests, metadata backend with a settable plane count):
  `DEV_TAB[13]=256/[35]=1/[39]=0` from `v_opnwk`, `v_opnvwk` and `vq_extnd(0)` at 16 and 32;
  `INQ_TAB[4]` = 16/32, `INQ_TAB[5]` = 0 at both, no other `INQ_TAB` word depends on the depth;
  `INQ_TAB[5]` = 1 for 1/4/8/24 planes (EmuTOS rule kept); `vq_scrninfo` at 32 = the `vq_extnd` mode-1 answer
  (45 words), at 16 the 273-word table; `appl_init` `ap_planes` (`global[10]`) = 16 / 32.
- `docs/analysis/deps/mapping.tsv`: the note of the `update_rez_dependent → vq_extnd` row names the 32-plane case.

Not changed, by the slice boundary: EdDI (`tos-api/src/vdi/eddi.rs:52@DEVICE_PLANES`, gate `:82@DEVICE_PLANES`):
the gate is not widened (TASK/V62.md, spec gate point 1); `DEVICE_PLANES` stays the depth of the one table that
exists (V66 owns the 32-plane table).

## Boot order (spec gate point 3, traced)

`kernel_main_rust` reads `vdiplanes=` at `kernel/src/lib.rs:291@apply_vdi_planes`, before `init_framebuffer`
(`kernel/src/lib.rs:358@init_framebuffer`), `host_env::install` (`:372@install`), `drivers::gic::enable_irqs` (`:417@enable_irqs`) and `fbcon::init` (`:516@fbcon`). Every
`planes()` caller reaches the backend either through the host (`host_env`, which returns it only when
`vdi_backend::available()`, i.e. after `init_framebuffer`) or through `VDI_SCREEN` directly from `fbcon`
(after `fbcon::init`), `vbl.rs`/`ikbd.rs` (interrupt paths, after `enable_irqs`). Nothing runs before
`kernel_main_rust` except the assembly entry. So `planes()` is never answered before the argument is read: no stop.
Smoke-confirmed on qemu-virt: the `[vdi]` line is serial line 7, `[fb] ramfb up` line 27-28.

## Check against the corrected spec (§2 of the 32-plane section, `main` `9849c83a2`)

| §2 place | Corrected spec for V62 | Code on `v62` |
|---|---|---|
| kernel backend `planes()` | 16 or 32 by the setting | `PLANES` atomic, `set_planes` from `vdiplanes=` (`kernel/src/vdi_backend.rs:44@PLANES`, `:465@planes`) |
| `tos-api` raster `CHUNKY_PLANES` | stays 16 at both settings; follows with V65 | unchanged, 16 (`tos-api/src/vdi/raster.rs:85@CHUNKY_PLANES`) |
| EdDI `DEVICE_PLANES` and gate | "see §4": the gate is not widened in V62 (§4, TASK/V62.md spec gate point 1) | unchanged; at 32 `vq_scrninfo` falls through to `INQ_TAB` (test `vq_scrninfo_at_32_planes_falls_through_to_vq_extnd`) |

§3 (values) and §4 (EdDI) are unchanged by the correction; the code satisfies them as in the first commit.
The reason the spec now gives (the stamp doubles the row stride over the AES's 16-bit save buffers) is the one this
receipt's first version reported.

## What a `vdiplanes=32` boot shows until V63-V67 (acceptance point 3)

- Answers: `vq_extnd` `INQ_TAB[4] = 32`, `INQ_TAB[5] = 0`, `DEV_TAB[13] = 256`, `[35] = 1`, `[39] = 0`;
  `appl_init` `ap_planes = 32`. `vq_scrninfo` (`CONTRL[5] == 1`, `INTIN[0] == 2`) answers as `vq_extnd` mode 2,
  i.e. `INQ_TAB` (45 words, `CONTRL[4] = 45`), not the 273-word table (gate not widened, spec §4).
- Drawing: unchanged — the kernel still draws the 16-bit device word through the 16-bit seam (V63 changes it).
  qemu-virt smoke without an SD image: the desktop comes up (`[desk] desktop up (0 drives)`), and the QMP
  screendump at 32 planes is byte-identical to the default boot's (sha256 `8720fc18…6b2c` both, 1920x1080 PPM):
  menu bar, trash and printer icons, pointer. (Screendumps taken with the first build of this change, sha256
  `93a1b6dd…1a42` flat image; the committed code differs only by moved lines and comments. The final image,
  sha256 `012cbe38…21f2`, was re-smoked: no argument / `vdiplanes=32` / `vdiplanes=abc` print no line / the
  32-plane line / the INVALID line at serial line 7, before `[fb] ramfb up` at 27-28, and reach `[desk] desktop up`.)
  Scratch: `target-agents/agent-a18c9d2f24deeef22/scratch/smoke*/` (gitignored, removed with the agent directory).
- Program blits: `vro_cpyfm`/`vrt_cpyfm` with the screen as source or destination are silent no-ops at 32
  (the screen's 32 planes exceed the unchanged cap 16, `tos-api/src/vdi/raster.rs:560@CHUNKY_PLANES`,
  `:821@CHUNKY_PLANES`). The AES's own save/restore and colour-icon blits do not pass that gate
  (`screen_to_mem`/`mem_to_screen` directly) and keep working.

## Build, clippy, tests

- `cargo xtask build --board rpi4`: OK (`kernel` 93 rustc warnings, as before). `cargo xtask build --board
  qemu-virt`: OK (93). Each board rebuilt after `touch kernel/src/lib.rs` ("Compiling kernel" seen); the image
  carries the `vdiplanes` strings.
- Clippy, baseline measured at `e52bb97d9` in this worktree (changed files restored from `HEAD` copies, then put
  back): `cargo clippy -p kernel --features board-rpi4` kernel 121 / tos-api 73 before and after;
  `--features board-qemu-virt` 121 / 73 before and after; no warning on a changed line;
  `cargo clippy -p tos-api --lib --tests`: the new test file has none. **Delta 0.**
- `cargo test -p tos-api --test vdi_v62_planes_setting`: 5 passed.
- Full `cargo xtask test` (worktree with `worktree-setup` links; run twice on `05cc70921`'s code): exit 0, 2507 passed / 0 failed / 7 ignored
  (sum of the `test result:` lines), no SKIPPED; includes `vdi_v62_planes_setting` and
  `kernel_bootargs::tests::vdiplanes_accepts_16_and_32_only`.
- After the merge of `main` `9969cff27` (merge commit `b9bd4be4b`): both boards rebuilt (`touch kernel/src/lib.rs`,
  "Compiling kernel", 93 rustc warnings each, OK); full `cargo xtask test` exit 0, 2507 passed / 0 failed / 7 ignored,
  no SKIPPED. Clippy unchanged (the merge touched no code): delta 0.

## Tester handoff

- Host: `cargo xtask test` (the new `vdi_v62_planes_setting` and `kernel_bootargs::tests::vdiplanes_accepts_16_and_32_only`).
- Default unchanged (acceptance 1): the full XBIOS and VDI case runs (`cargo xtask tosdump …`, tables as in the
  last VDI/XBIOS acceptance runs) without the argument and with `vdiplanes=16`: expected no changed verdict. The
  default boot log has no new line; `vdiplanes=16` adds exactly the `[vdi]` line.
- 32 planes (acceptance 2): boot with `vdiplanes=32` (qemu-virt `-append`, raspi4b `-append`, Pi 4
  `cmdline.txt`): expect the `[vdi] VDI depth: 32 planes …` line, `vq_extnd(1)` `INTOUT[4] = 32`, `[5] = 0`,
  `appl_init` `global[10] = 32` (e.g. a tosdump case of `vq_extnd`/`appl_init` run with the argument; the tosdump
  caps at 16 planes, ADR-038 point 13 (iv), are a later slice). Invalid: `vdiplanes=24` (and `abc`) → the INVALID
  line, 16 planes.
- Real-Pi-4 only: the `cmdline.txt` path through the VideoCore mailbox (`with_command_line`); QEMU raspi4b answers
  the same tag with `-append`.

## Proposed `docs/STATUS.md` line

2026-11-18 — V62 (VDI 32 planes, slice 1): boot argument `vdiplanes=<16|32>` (absent/invalid = 16, invalid with a
boot line); the kernel backend's `planes()` is a boot-time value; `vq_extnd` `INQ_TAB[4]` and `ap_planes` report
32 at the 32 setting, `INQ_TAB[5]` stays 0; EdDI gate and `CHUNKY_PLANES` unchanged (they follow with V66 and V65).
Commit `<hash>` (branch `v62`).

## Out-of-scope observations

- `tos-api/tests/vdi_reference/harness.rs` `MACHINE_EXCLUDED` substitute for `INQ_TAB[5]` is `PLANES != 16`; it only
  runs with `PLANES = 16`, but would need the 32 rule if the harness ever gets a 32-plane backend.
- `docs/api/vdi/workstation/100-v_opnvwk.md` §2 intro says V62 makes two places follow the setting, "backend
  `planes()`, EdDI per §4", while TASK/V62.md ("Slice cut corrected") gives the EdDI constant to V66; §4 itself and
  its open point still say the gate decision is open, which the ticket settled (not widened). Wording only; the code
  matches §4 and the ticket.
- `docs/COMPATIBILITY.md` has no section on the 32-plane setting yet (ADR-038 point 12 asks for one); not in V62's
  scope as cut.

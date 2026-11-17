# SND05 — receipt (developer): XBIOS 0x80-0x8D, DMA ring and PWM jack backend

Role: developer
Agent: developer-snd05 (Opus, agent a12e424f71125ae31; resumes the stopped work of agent a9d05a81becfe1313)

Date 2026-11-17. Branch `snd05` (worktree `target-agents/agent-a12e424f71125ae31/wt`): the work-in-progress
commit `1415fb08c`, the merge of `main` @ `a5a4665cb` (`ee30dca1c`, the lead's authorised merge; the only conflict,
`TASK/SND05.md`, keeps both "State when stopped (2026-11-12)" and "Stopped by the owner (2026-11-12)"), then this
commit. Spec: `docs/api/xbios/_dmasound-group.md` (incl. §9.1-§9.6, new §11), hardware contract
`docs/hardware/pwm-audio.md` §12. Owner decisions: SND01 2026-11-12 (Q-N2 option D, Q-N3, Q-N4a-c, Q-N5, Q-N6) and
2026-11-17 (the five spec gaps, "passt", `TASK/SND05.md` "Owner decision 2026-11-17").

## What changed

Code (from `1415fb08c`, described in `TASK/SND05.md` "State when stopped (2026-11-12)"; unchanged by this commit
except the test file):

- `tos-api/src/xbios/sound.rs` (new): XBIOS 0x80-0x8D, EmuTOS' Falcon branch once a backend is available, the
  function number before; the converter of the 200 Hz tick into the staging ring.
- `tos-api/src/xbios/sound/tests.rs` (new): 25 host tests. **This commit:** three `vec![…]` turned into arrays (the
  three clippy `useless_vec` warnings the file added under `--tests`).
- `tos-api/src/xbios/mod.rs` (dispatch arm 0x80-0x8D, `pub mod sound`), `tos-api/src/cookie.rs` (`SND`,
  `SND_VALUE` = `0x14`, seed comment), `tos-api/src/bdos/process/usermem.rs` (`pool_kva`),
  `tos-api/src/bdos/process/hooks.rs` (`Via::Setinterrupt`), `tos-api/src/bdos/process/pterm.rs` (byte-order reset).
- `kernel/src/drivers/bcm2711/dma.rs` (new), `kernel/src/drivers/bcm2711/pwm_audio.rs` (new),
  `kernel/src/drivers/bcm2711/clock_tags.rs` ("Set clock state" + 2 tests), `mailbox.rs` (`set_clock_state`),
  `mod.rs`, `kernel/src/drivers/gic.rs` (refill in the tick branch), `kernel/src/lib.rs` (init after the timer),
  `kernel/src/selftest_cookie.rs` (`_SND` = `0x14` iff the backend is up, else `EERROR`).

This commit:

- `sdk/examples/sndtone.c` (new): the jack test program (tester handoff below).
- `docs/MEMORY.md`: the ADR-012 amendment 2026-11-12 (text of `TASK/SND05.md`, plus the approved Q-N4a sentence on
  the byte-order probe), after the amendment 2026-10-28.
- `docs/analysis/deps/mapping.tsv`: the 23 `bios/dmasound.c` rows rewritten per the ticket's Localization table
  (16 `mapped` to `tos-api/src/xbios/sound.rs`, 7 `design:hardware:…` with the new reasons); and the 45 ADR line
  cites into `docs/MEMORY.md` (`:824`, `:2158`, `:2749`, `:2826`, `:3044`) moved by +23 (the amendment's lines).
  Note-only on those 45 rows, no class change.
- `docs/COMPATIBILITY.md`: cookie rows 41 and 49; the paragraph "XBIOS for absent Atari hardware" (no `_SND`
  claim, DSP range now inside the table); new section "XBIOS DMA sound 0x80-0x8D" (no-backend answer, Falcon
  branch with a backend, `Locksnd` EQUAL and advisory, recording, DSP, frame-end interrupt, `Buffptr` layout,
  `Soundcmd(9)` extension, rates, latch, `Sndstatus`, what is not applied).
- `docs/DEVIATIONS.md`: new section "XBIOS DMA sound (0x80-0x8D)", 13 rows. The file's German markers
  (*gemessen*, *gelesen*, *offen*, *geparkt*) translated to English (*measured*, *read*, *open*, *parked*), as
  CLAUDE.md requires for a touched file.
- `docs/api/xbios/_dmasound-group.md`: header note, §1.4 (state since SND05), §9.3.1 / §9.4 / §9.6 (owner decision
  2026-11-17), §10.4 resolved, §11 implementation notes (layers, transport, observable rules, the five decided
  rules, not modelled, evidence).
- `docs/api/xbios/_dispatcher.md` §4 and §5: NewTOS `LAST_ENTRY` = `0x8D`.
- `TASK/SND05.md`: line 3 `IMPLEMENTED`, the five spec gaps marked confirmed, section "Owner decision 2026-11-17".

The five spec gaps are documented as **owner decisions 2026-11-17** (the lead's relayed message replaces the
HANDOVER's "provisional" wording); no behaviour changed on them.

## Verified (developer)

- **Merge:** `main` @ `a5a4665cb` into `snd05`, textual conflict only in `TASK/SND05.md`; XBIOS21's
  `tos-api/src/xbios/screen.rs` and cookie changes merged cleanly, the build and the tests below run on the merge.
- **Builds:** `cargo xtask build --board rpi4` and `--board qemu-virt`: OK; rustc warnings `tos-api` 64, `kernel` 93
  on both boards, equal to `main` @ `a5a4665cb` built in a detached worktree (`target-agents/agent-a12e424f71125ae31/base`);
  the warning locations in the touched files are identical (only a pre-existing one at line 9 of the kernel's `lib.rs`). Delta 0.
- **Clippy** (both trees): `cargo clippy -p tos-api --lib` 73 = 73; `cargo clippy -p kernel --features board-rpi4`
  and `--features board-qemu-virt`: kernel 121 = 121, tos-api 73 = 73; `cargo clippy -p tos-api --lib --tests`:
  lib test 81 before the fix → 78 = `main`'s 78 after it. Delta 0.
- **Host tests:** `cargo xtask test` (full): 2486 passed, 0 failed, 7 ignored (`main` baseline 2459 / 0 / 7; +25 `xbios::sound` tests, +2 `clock_tags` tests).
- **SDK host tests** (`sdk/` touched): `sdk/run-host-tests.sh` 33 passed, 0 failed, 5 skipped (after
  `sdk/gemlib/build-gemlib.sh` in the worktree, the runner's prerequisite).
- **`cargo xtask deps --check-mapping`:** "1968 rows, all resolve, every class is valid; every bios/, bdos/, vdi/,
  aes/ function has a row".
- **`cargo xtask status-check --new-only`:** 1 checked, 0 rejected.
- **doc-check** of the edited docs (`docs/MEMORY.md`, `docs/COMPATIBILITY.md`, `docs/DEVIATIONS.md`,
  `docs/api/xbios/_dmasound-group.md`, `docs/api/xbios/_dispatcher.md`, `TASK/SND05.md`): 0 failures. The worktree
  lacks four gitignored trees of the main tree (`sdk/ldg/ldg-r133`, `sdk/cflib/cflib-21`,
  `sdk/zview/z-tools-04805b7`, `Applications/`); with them linked in for the check (not committed) 0 failures,
  without them 5 legacy failures on unchanged lines.
- **doc-check of every undated doc citing a changed code file** (193 files): the failures that cite one of the
  changed files are the same 18 legacy lines as on `main` (stale `usermem.rs`/`hooks.rs` cites in dated-era
  receipts and plans, `TASK/XBIOS13.md` `SETTIME`), none new, none of them on a line this branch changed.
- **QEMU raspi4b smoke boot** (debug, `cargo xtask run --board rpi4 --headless`, 90 s, `serial=on` by xtask): boots
  to the desktop; the log has `[sound] no PWM clock (firmware: set clock state Some(1), no measured rate) — no
  sound backend, _SND absent` and `[desk] desktop up (0 drives)` (no SD image attached). Log in the agent scratch
  directory only, no run directory.
- **Test program build:** `sdk/build.sh sdk/examples/sndtone.c <out>/SNDTONE.NCP`: no warnings, 10 504 bytes,
  sha256 `27854b9a926e1db7acd4ee0c0b979b60e1a35fcddf2e7de5af288d67910b8800` (gitignored output in the agent's scratch directory
  `agent-a12e424f71125ae31/out`, not committed).

## Tester handoff

What exercises the change, and the expected result:

1. **Host:** `cargo xtask test` (the 25 tests `xbios::sound::tests`, the 2 `clock_tags` tests via xtask).
2. **QEMU raspi4b / qemu-virt** (no backend): boot to the desktop; serial line `[sound] no PWM clock … no sound
   backend, _SND absent` (raspi4b; qemu-virt builds no sound driver). `bios-r1` 230-235 answer their function
   numbers as before (ACCEPTED, `docs/COMPATIBILITY.md` "XBIOS DMA sound 0x80-0x8D"). The cookie selftest
   (`kernel/src/selftest_cookie.rs`) expects `Getcookie(_SND)` = `EERROR` there. `SNDTONE.NCP` started from the
   desktop prints the "no sound backend" line and ends with exit code 1.
3. **Real Pi 4 only** (owner, the acceptance gate): boot log line `[sound] PWM jack backend: clk_pwm measured
   <≈107 143 0xx> Hz, DMA channel 5, ring 2048 words at bus 0x…, _SND = 0x14`. Put `SNDTONE.NCP` on the card at
   `C:\SNDTONE.NCP`, headphones or an amplifier on the 3.5 mm jack, double-click it. Expected: 2 s of a 440 Hz tone
   on both channels, 2 s on the **left** channel only, 2 s on the **right** channel only. Screen/serial: every line
   ends `(expected …)` without `MISMATCH`: `Locksnd` 1, `Soundcmd(9,-1)` 5, `Setmode` 0, `Devconnect` 1, per pass
   `Setbuffer` 0, `Buffoper(1)` 0, `Buffoper(-1)` after the pass 0, `Buffptr play offset` 393 360 (= `end`, the
   `falcontc` S4 model), `Buffptr record` 0; finally `Unlocksnd` 0 and `SNDTONE: done`. A `STALLED` line means the
   PWM-paced DMA does not advance (the program then stops playback and exits with 3).
4. **Only the real Pi can show:** the audible tone and its channel assignment, the PWM-paced DMA transfer (no run
   has exercised one; QEMU has no PWM block), the measured clock and the ring refill under load (underruns would be
   audible as clicks), `_SND` = `0x14` in the jar, and `bios-r1` 230-234 equal to `falcontc` with the backend
   (expected, not measured) plus the SND04 probe `tosdump/probes/snprobe.c` on NewTOS with the backend.

## Proposed `docs/STATUS.md` line

`2026-11-17 — SND05 implemented on branch snd05: XBIOS DMA sound 0x80-0x8D (Falcon branch once a sound backend is
up, function number before), PWM jack backend on DMA channel 5 with a 200 Hz-tick converter, `_SND` = 0x14 set by the
driver (ADR-012 amendment 2026-11-12), test program SNDTONE.NCP; QEMU has no backend; the audible tone awaits the
real-Pi run.`

## Out-of-scope observations

- `cargo xtask worktree-setup` does not link `target-agents/`, `sdk/ldg/ldg-r133`, `sdk/cflib/cflib-21`,
  `sdk/zview/z-tools-*` and `Applications/`; doc-check in a worktree then reports their cites as missing (the merge
  commit needed an empty `target-agents/` in the worktree for three cites in `TASK/A41.md`, `TASK/TOOL67.md`,
  `docs/STATUS.md`).
- ADR-031's row for `bell_hook`/`kcl_hook` ("no sound device yet", SND01 "Not done") is unchanged; the ticket allows
  a later commit.
- `docs/api/xbios/_dispatcher.md` §5 note "the 25 in-scope functions … NewTOS TT-shifter profile" still speaks of the
  old profile name.
- Legacy stale cites into `tos-api/src/bdos/process/hooks.rs` / `usermem.rs` in `TASK/BIOS07`, `BIOS15`, `BIOS16`,
  `A83`, `MEM04` receipts (present on `main`).

## Notes for the lead

- The track rule (§11.5, `docs/DEVIATIONS.md`): with `Settracks(play > 0)` the converter still reads the buffer as
  one `L R` track, so a 2-4 track buffer plays its tracks one after the other, pair by pair. Documented as a
  deviation (the "tracks not used" item of the stopped state), behaviour unchanged.

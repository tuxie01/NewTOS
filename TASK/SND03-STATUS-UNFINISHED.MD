# SND03 — receipt (unfinished: owner run on the real Pi, pwm-audio.md update)

Role: developer
Agent: agent-adce16ed4b4b0609f

Developer, 2026-11-12. Worktree branch `worktree-agent-adce16ed4b4b0609f`, base `767688323`; one commit, not landed.

## Done

- Work point 1: kernel debug feature `snd-probe` (`kernel/src/snd_probe.rs`): H4 (System Timer, DMA state, `PACTL_CS`),
  H3 (DMA copy with and without cache maintenance), H5 (the same run, GIC pending bit), H1 (firmware view of clock id
  PWM, CPRMAN word read only, timed FIFO drain; firmware "Set clock state/rate" only when no clock runs), H2 (FIFO
  depth), tones T1-T4 on GPIO 40/41. Off by default.
- Work point 2: H5 measured in QEMU raspi4b 11.1.2, see `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`
  RESULT 1.
- Feature off: kernel bytes equal the base except `Location` line fields (RESULT 2, TOOL08 method; TOOL13 is parked).
  Warning delta 0 on both boards (build 159/159, clippy 121/121). `cargo xtask test` full: 2454 passed, 0 failed, 7 ignored.
- Work point 3 prepared: `measurements/measurement-2026-11-12-snd03/TESTBLATT.md`, `measurements/measurement-2026-11-12-snd03/method/snd03-card.list`.

## Missing

- The owner's run on the Pi (H1-H4 numbers, tone heard); the lead builds `kerns03.img=snd-probe` from the landed commit
  and writes the card.
- `docs/hardware/pwm-audio.md` updated with the measured facts (after the Pi run; the note is German and is translated
  when touched).

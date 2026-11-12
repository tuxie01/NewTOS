# SND03 — Sound: hardware probe on the Pi 4 for the PWM jack and the DMA ring (H1-H5)

Status: OPEN (2026-11-12) | Priority: P2

Type: developer (probe as a debug feature), then a real-Pi run with the owner. Step 0 of [SND01](SND01.md)
(decisions 2026-11-12: DMA path, jack first). Facts not in any pinned source are measured here, not assumed
(CLAUDE.md rule 6).

## Facts

- Probe list H1-H5: [SND01](SND01.md), section "Spec review (researcher, 2026-11-12)", §4.
- `docs/hardware/pwm-audio.md` §5.2: the clock manager for the PWM (`clk_pwm`) has no register in the BCM2711
  datasheet. The owner's cross-reference (iosoft article, SND01 "Cross-reference from the owner") gives what to
  expect, not facts.
- EL0 programs cannot reach MMIO; the probe runs in the kernel.

## Work

1. A kernel debug cargo feature (off by default; with it off the kernel bytes equal the build without the change,
   CLAUDE.md "Testing", checked by the xtask command) that runs at boot and prints to the screen and serial:
   - H1 the effective `clk_pwm`: time the FIFO drain at a known `RNG` against the system counter; first read what
     the firmware left in the clock manager (read only), then, if a write sequence is needed, document its source.
   - H2 FIFO depth in PWM mode (writes to `FIF1` until `FULL1`, channels disabled).
   - H3 a staging buffer written by the CPU and read by a DMA channel into another buffer, with and without cache
     maintenance; result compared word by word.
   - H4 the system-timer compare registers and the DMA channel enable mask the firmware leaves at boot.
   - An audible check: a short tone (square or sine table) on GPIO 40/41 through PWM, so the owner hears the jack.
2. H5 in QEMU raspi4b: whether the modelled DMA does memory-to-memory with `PERMAP = 0` and raises its interrupt at
   the GIC (decides what host or QEMU tests the DMA driver can have).
3. A test sheet for the owner (German, in the run directory) and the run on the real Pi with headphones or a speaker
   at the jack; results recorded as a measurement run.

## Acceptance

- H1-H4 measured on the real Pi with the numbers in a run's RUNINFO; H5 measured in QEMU; the tone heard (owner).
- The feature off: kernel bytes unchanged (xtask check); `cargo xtask test` green; warnings delta 0.
- `docs/hardware/pwm-audio.md` updated with the measured facts (translated to English as it is touched).

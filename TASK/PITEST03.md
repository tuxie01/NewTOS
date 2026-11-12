# PITEST03 — Probe: save the framebuffer to the card from a program

Status: OPEN (2026-11-12) | Priority: P2

Title: Probe: save the framebuffer to the card from a program

## Owner decision 2026-11-12

Part of the Pi queue (umbrella TASK/PITEST02.md). The triage
(`docs/analysis/pi-queue-triage-2026-11-12.md`) classifies nine tickets as
`capture`: the program builds the scene and saves the framebuffer to the
card, a tester judges the image on the Mac. PITEST01 provides no tool that
saves the framebuffer; this ticket builds it.

## Scope

One small program under `sdk/examples/` that reads the visible screen
(`Physbase` and the current mode's size and pixel format) and writes it to
a file on `C:`, plus a host-side step that turns the file into a PNG.

## Acceptance

- On QEMU `raspi4b` the PNG equals the QMP screendump of the same moment,
  or every difference is stated.
- The saved file's path, size and sha256 are in the run's `RUNINFO.TXT`.

Real-Pi part (owner): one run, the saved image next to a photo of the HDMI
picture. It decides whether the nine `capture` tickets of the triage stay
`capture`. This part goes into the `Missing` field as
`pi: saved image matches the HDMI picture`.

## Flow

Researcher spec gate (`Physbase`, screen size and pixel format), developer
on branch `pitest03`, tester, land, accept.

## Spec gate (2026-11-12)

Landed as daba4d67d: `docs/analysis/screen-readout-spec-2026-11-12.md`.
Every value is cited except the bytes per line. Owner 2026-11-12, option
(a): the pitch comes from a new `Ssystem` key, TASK/PITEST04.md. The
developer starts on PITEST03 after PITEST04 is landed.

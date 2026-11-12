# PITEST04 — Ssystem key for the framebuffer's bytes per line

Status: OPEN (2026-11-12) | Priority: P2

Title: Ssystem key for the framebuffer's bytes per line

## Owner decision 2026-11-12

The PITEST03 spec gate (`docs/analysis/screen-readout-spec-2026-11-12.md`)
found that no call a program can make returns the framebuffer's bytes per
line; the pitch exists only in the kernel (`ScreenInfo.stride_bytes`). Of
the two options, the owner chose (a): a new NewTOS-own `Ssystem` key, in
the line of `S_NEWTOS_SCRWIDTH` / `SCRHEIGHT` / `SCRDEPTH` (2013-2015).
PITEST03 depends on this ticket.

## Scope

A NewTOS-own `Ssystem` mode (working name `S_NEWTOS_SCRPITCH`, at the
first free number in `docs/api/gemdos/154-ssystem.md`) that returns the
bytes per line of the visible framebuffer, as the kernel uses it, on both
boards.

## Acceptance

- `docs/api/gemdos/154-ssystem.md` has the row and its text (researcher).
- The mode returns the kernel's pitch on QEMU `raspi4b` and `qemu-virt`;
  the host tests that pin the first free number are updated.
- Real-Pi part: the value at the HDMI mode of the card is recorded in the
  PITEST03 Pi run.

## Flow

Researcher (spec row), developer on branch `pitest04`, tester, land, accept.

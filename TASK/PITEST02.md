# PITEST02 — Pi queue: triage of the tickets that wait for a real-Pi run

Status: OPEN (2026-11-12) | Priority: P2

Title: Pi queue: triage of the tickets that wait for a real-Pi run

## Owner decision 2026-11-12

Pi acceptance becomes one merged session driven by a test program on the
card, not about 44 separate manual checks. This ticket is the triage that
the test program is designed from; it builds nothing.

## Scope

Classify every ticket whose line 3 is `IMPLEMENTED` and whose `Missing`
field asks for a real-Pi or hardware run (about 44) for a merged Pi test
program.

Role: researcher. Inputs: `TASK/*.md` line 3 at `main` HEAD;
`TASK/PITEST01.md`; `docs/api/desktop/_hw-test-checklist.md`;
`docs/HW-TEST-LOG.md`.

## Acceptance

docs/analysis/pi-queue-triage-2026-11-12.md (to be written) with one row
per ticket:

- id;
- what must be observed on the Pi (one sentence, cited from the ticket or
  its receipt);
- kind `auto` | `capture` | `ask` | `none`;
- the scene or API call the program needs;
- for `ask`: the one question the owner answers with yes/no.

Kinds:

- `auto` — the program calls the API and checks the result itself;
- `capture` — the program builds the scene and saves the framebuffer to the
  card, a tester judges the image on the Mac;
- `ask` — only the eye or hand at the Pi can judge (flicker, speed, HDMI
  output, input feel);
- `none` — cannot be driven by a program, with the reason.

Plus:

- the list of tickets whose `Missing` field does not name a Pi run after all;
- the tickets that share one scene;
- what PITEST01 (card, `tools/sync-card.sh`, probes) already provides.

## Stop conditions

- No ticket text is edited; no `Missing` field is rewritten.
- A ticket whose required observation is not stated in its ticket or
  receipt is listed as "not stated", not guessed.

## Umbrella (owner 2026-11-12)

The triage is landed (af74d7700,
`docs/analysis/pi-queue-triage-2026-11-12.md`). PITEST02 stays `OPEN` as
the umbrella for the Pi queue.

Not in the first program version, case to be named by the owner (triage
STOP points, "Open points"):

- A90 — no Pi-specific observation stated in ticket or receipt.
- A118 — "HypView-Serie" is open in the receipts but not defined.
- T04 — which of the 13 TeraDesk menu entries is not stated.

## Real-Pi session of 2026-11-19 (lead)

Owner 2026-11-19: card in the Mac, prepare the kernel and say what to test. Preparation
`measurements/measurement-2026-11-19-pitest02-prep/RUNINFO.TXT` (kernel `kern1119.img` of `afc2bb1c0`, test sheet
`measurements/measurement-2026-11-19-pitest02-prep/TESTBLATT.md`); the card was written with `tools/sync-card.sh --write`
by the lead on the owner's word (`measurements/measurement-2026-11-19-pitest02-card/card.manifest`); evaluation of the
card's result files `measurements/measurement-2026-11-19-pitest02-pi/RUNINFO.TXT`.

Owner's answers: the first boot failed at the bootloader (sector 0 read as garbage) and worked after reseating the
card; the Pi boots with both settings, 16 and 32; nothing unusual noticed; the sheet was not gone through step by step
("wir brauchen nicht die ganzen apps durchgehen").

Shown by files (written at `vdiplanes=32`): sheet steps B2, B5, A5, and A4 with the cases 87, 213, 230-235 as
expected; A7 at 16 planes within the spread. `_SND` is in the jar.

Still open in the collected list, no file and no explicit answer: SCRNINFO and `bios-r1` at the default start, Fbench
at 32 planes, PRESENT's times, the pitch, the two serial boot lines, the mouse click on Cancel in TeraDesk's Video
options dialog, the SND03 hearing test.

New observation, values only, for the owner to decide on a ticket: `bios-r1` cases 31 `Bconstat(1)` and 36
`Bconstat(6)` return -1 on the Pi where the `falcontc` reference and QEMU return 0.

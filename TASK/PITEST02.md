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

# XBIOS24 — TeraDesk's video dialog shows the Pi's real mode ("Pi mode")

Status: OPEN (2026-11-18) | Priority: P3

Title: TeraDesk's video dialog shows the Pi's real mode ("Pi mode")

Type: research first (researcher), then developer. Owner decision 2026-11-18, from the TeraDesk measurement of
TASK/XBIOS22.md (`measurements/measurement-2026-11-18-xbios22-test/RUNINFO.TXT`, section "TERADESK GATE").
Owner: "nur die frage ob man es hinbekommt das dort die richtigen Daten drin stehen" (the question is whether
one can get the right data in there) and, asked whether to file the research: "bitte mach einen Pi-Modus"
(please make a Pi mode). Goes behind TASK/XBIOS23.md.

## Facts

- Since XBIOS22 (`f6bdafa47`) NewTOS publishes `_VDO` = `0x00030000`; TeraDesk takes its Falcon branch and reads
  `VsetMode(-1)` = `0x001C` and `VgetMonitor` = 2.
- Its video dialog then shows "VGA-80-column", Overscan greyed, "Doubleline" enabled, Colours 64K. That is the
  nearest Falcon description of the screen, not the Pi's mode (set in `config.txt`, e.g. 1920x1080 at 32 bpp).
- OK on the unchanged dialog raises TeraDesk's alert "A change of video mode will terminate all running
  applications"; OK there restarts the AES without a mode change (`shel_write(SHW_RESCHNG)`, ADR-017).
- Programs get the real size already through the VDI and through `VgetSize`.

## Not established

- Which ways TeraDesk's video code knows besides the Falcon mode word and the ST/TT resolutions (other `_VDO`
  values, graphics-card or extended-mode paths), and what each makes the dialog show. Source:
  `sdk/apps/teradesk/` (gitignored upstream tree; patches go to `sdk/overlays/`, upstream stays unchanged).
- Whether a description exists under which the dialog shows the Pi's resolution and depth and offers no mode it
  cannot honour, without NewTOS claiming hardware it does not have (ADR-012, ADR-017).
- Why, on the XBIOS22 commit, the first Return on the unchanged dialog left *Colours* at 256 and the alert came
  only with the second Return, while with the arch06 injection on `main` it came with the first (same run,
  section "TERADESK GATE" c).
- What other programs on the card do with the same data (zView, QED): no effect was seen in the XBIOS22 run.

## Work

1. Researcher: read TeraDesk's video code and name its branches with `file:line`; explain the *Colours* 256
   observation from the source; list the options for a "Pi mode" with, per option, what the dialog shows, what
   OK does, what it costs (kernel, patch file, or both), and what it changes for other programs.
2. Owner decides the option.
3. Developer, then tester with the TeraDesk scenario of the XBIOS22 run.

## Acceptance

- Step 1: a dated analysis under `docs/analysis/` with the branches cited and the options compared; the
  *Colours* observation explained or named as not explainable from the source.
- After the fix (wording follows the owner's choice in step 2): TeraDesk's video dialog shows the screen's real
  resolution and depth; OK on the unchanged dialog raises no mode-change alert and restarts nothing.

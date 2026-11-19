# SPEC07 — `docs/api/xbios/_triage.md`: the "disabled" entries of the header against the reference build

Status: OPEN (2026-11-19) | Priority: P3

Type: researcher (documentation). Owner decision 2026-11-19: its own small ticket at the end of the queue.

## Facts

- Side observation of TASK/XBIOS13.md (`eb1d563d7`): the header of `docs/api/xbios/_triage.md` lists
  `CONF_WITH_MFP`, `CONF_WITH_VIDEL`, `CONF_WITH_DSP` and others as disabled, and the rows of `Mfpint`, `Jdisint`,
  `Jenabint` and `Xbtimer` give "`CONF_WITH_MFP` off" as their reason, while
  `docs/api/xbios/_full-function-inventory.md` lists several of these switches as 1 in the reference build.
- XBIOS13 corrected only the `CONF_WITH_NVRAM` statements and left a one-line note in `_triage.md`.
- The generated configuration table marks a switch 1 when it is compiled in for all machines, which is not the
  same as active in the Falcon profile.

## Work

1. For every switch the header of `_triage.md` names: its value in the reference build, from the oracle's build
   tree, with `file:line`.
2. Correct the header and every row whose reason names such a switch; where a function is `unimpl` on NewTOS for
   another reason (hardware level, an ADR), the row says that reason.
3. The same statements in `docs/api/xbios/_dispatcher.md`, if any.

## Acceptance

- Every corrected statement is cited; `_triage.md` no longer contradicts `_full-function-inventory.md`.
- No code change; `cargo xtask doc-check` of the edited docs 0 failures.
- A row whose NewTOS state turns out to be an unlisted difference to EmuTOS is reported to the lead, not changed.

## Added from TASK/BIOS23.md (owner, 2026-11-20)

The documentation rests the review of the parked tickets found
(`docs/analysis/bios-parked-review-2026-11-20.md`), corrected with this ticket:

- TASK/BIOS13.md: `tosdump/cases/ERRATA.md` still cites the `ETV_DEFAULT_*` constants that TASK/BIOS07.md removed.
- TASK/XBIOS07.md: three statements, listed in section 4 of the review (the address in
  `docs/api/xbios/22-kbdvbase.md` and `docs/api/xbios/_state.md`, and head and tail swapped in `_state.md`).
- `docs/api/xbios/_psg-group.md` says `CONF_WITH_YM2149` is off; the pinned target has it on. The same statement in
  a code comment is listed for the developer who next touches that file, not changed here.
- `docs/api/xbios/11-random.md`, Pi binding: says there is no 200 Hz counter; stale since `_hz_200` is live.

After the landing the lead brings BIOS13 and XBIOS07 to the owner for closing.

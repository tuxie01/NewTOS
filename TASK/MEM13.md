# MEM13 — `Srealloc` (GEMDOS 0x15): provide or sanction the missing slot

Status: OPEN (2026-11-11) | Priority: P3

Type: research then development. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G06 (T04).

## Facts

- EmuTOS has the entry (`bdos/bdosmain.c:201`); NewTOS has no slot and answers EINVFN (`docs/analysis/deps/mapping.tsv:242`). A spec exists: `docs/api/gemdos/15-srealloc.md`.
- No ticket title names it. `sdk/` names it only in mintlib headers (`sdk/libc/mintlib/include/mint/mintbind.h`); `programs/` not at all (grep 2026-11-11); `Applications/` was not searched.

## Work

1. Step 1 (survey, decides): grep `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`, `programs/`, `tosdump/`, `Applications/` and map every executable of the standard-image manifest (`tools/make-sd-image.sh`) to its build recipe; name any without a local source.
2. If no program calls it, the lead parks it after an owner decision (Reopen: a program calls `Srealloc`). Otherwise implement per the spec and measure.

## Acceptance

- Survey recorded; either `Srealloc` implemented with an oracle case, or the decision recorded in `docs/COMPATIBILITY.md`.

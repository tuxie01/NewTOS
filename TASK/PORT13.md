# PORT13 — zDesk: can it be built for NewTOS?

Status: OPEN (2026-11-19) | Priority: P3

Title: zDesk: can it be built for NewTOS?

Type: research first (researcher); a port, if any, is a later owner decision. Owner 2026-11-19: "bitte mal mit ins
projekt nehmen" (take it into the project) and, asked what for: "schauen ob wir es für NewTOS bauen können" (see
whether we can build it for NewTOS).

## Facts

- The source lies in `references/z-tools/zdesk/` (gitignored reference tree, 90 files, 804 KiB; sha256 over the
  sorted per-file sha256 list: `dcdbe04e68479eb7985cec9b75a2b44d24b0ca26c6e6f6b435454c6281226479`), copied by the
  lead on 2026-11-19 from the owner's directory, where it was untracked. Its origin (branch or release archive of
  z-tools) is not known.
- zDesk is part of z-tools, like zView. The z-tools state pinned for the zView port (`sdk/zview/`, `04805b7`)
  holds `zview`, `ztask` and `common`, no `zdesk`.
- Its Makefile links `-lgem -lldg`; the tree has its own raster code (a raster
  directory and an MFDB module), a file of bug notes in French, and no licence file of its own.

## Not established

- Licence and origin of this state of zDesk.
- What it needs beyond what the zView port already has: libraries (LDG, windom or others), AES and VDI calls,
  GEMDOS calls for the file operations of a desktop, extended AES features (its notes name MyAES-specific points).
- Whether it compiles under LP64 with the SDK, and what breaks (the zView port's overlay shows the kind of fixes).
- Whether it can run as a program on top of the desktop the way TeraDesk does (ADR-022, ADR-026), single-tasking.

## Work

1. Researcher: read the source and the zView port; answer the points above with `file:line`; try a compile of the
   sources with the SDK in a scratch directory (no product change, no new build recipe in the repository) and
   record what fails and why; name the calls it makes that NewTOS does not have (against `docs/api/` and the code).
2. Report to the owner: buildable as is / buildable with patches (how many, what kind) / blocked by what; cost
   estimate in the terms of the zView port; where the source would live.
3. Owner decides whether and when a port follows (desktop extras are after 1.0 by the roadmap).

## Acceptance

- Step 1: a dated analysis under `docs/analysis/` with the dependencies, the compile attempt's result and the
  missing calls, each cited; licence and origin stated or named as not found.

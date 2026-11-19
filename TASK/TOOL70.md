# TOOL70 — status-check reads the whole *Open* section of `docs/STATUS.md`

Status: OPEN (2026-11-20) | Priority: P3

Type: tool, plus a clean-up of the list by the lead. Owner decision 2026-11-19. Goes behind the product fixes; it
blocks no acceptance.

## Facts

- `cargo xtask status-check` compares the tickets the section *Open* of `docs/STATUS.md` names with line 3 of
  their files (report class `status-md`). Today it reports 7.
- On 2026-11-19 the lead removed a whitespace-only line inside the parked list of that section; status-check then
  reported 86, the 79 new ones for lines further down (for example "lists FS21 as Open, line 3 of TASK/FS21.md is
  DONE"). With the line restored it is 7 again (`1ef7607a2`). So the reader stops at that line and the list below
  it is stale without a report.

## Work

1. The reader covers the whole section, independent of blank or whitespace-only lines; a host test with such a
   line.
2. The lead brings the section in line with the tickets (the reported entries), in the commit that lands the tool
   or directly after it.

## Acceptance

- status-check reports every entry of the section whose state differs from line 3; after the clean-up it reports 0
  for the section.
- `cargo xtask test` green.

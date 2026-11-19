# SPEC08 — System variables: the unserved ones in `DEVIATIONS.md`, the stale section 7 of `_boot-init.md`

Status: OPEN (2026-11-20) | Priority: P3

Type: researcher (documentation). Owner decision 2026-11-19, from the findings of TASK/BIOS19.md.

## Work

1. `docs/DEVIATIONS.md`: one row by trigger for the system variables NewTOS does not serve and TASK/BIOS20.md does
   not take up: the four `bcon*_vec` tables (a program that patches a slot to redirect a device is neither served
   nor warned), `_v_bas_ad`, `_vblsem`, `_nvbls`, `_hdv_init`, `_hdv_boot`, `_themd`, `_dskbufp`, `_pun_ptr`; and
   the readable zeros of finding 4 of `docs/api/bios/_sysvars-contract.md`, section 6.
2. `docs/api/bios/_boot-init.md`, section 7: replaced by a pointer to `docs/api/bios/_sysvars-contract.md` (owner:
   pointer, not a corrected translation; the section is wrong in 8 places, listed in section 6 of the contract
   file). The rest of the file is translated as the touch requires.
3. `docs/COMPATIBILITY.md`, the system-variable section: the paragraph proposed in section 6 of the contract file,
   in the wording the owner approved.
4. `docs/api/xbios/_full-function-inventory.md`: `NVMaccess` type 2 is a reset, not a delete.
5. The class "free" of the contract table stays (owner).

## Acceptance

- Every row cites the contract table and the EmuTOS place; no code change; `cargo xtask doc-check` of the edited
  docs 0 failures.
- The gap of ADR-031 rule 1 (the `bcon*_vec` tables are hooks that are not watched) is reported to the lead as a
  proposal for the owner, the ADR is not edited.

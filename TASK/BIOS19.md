# BIOS19 — System variables: per-variable contract table and the `nvmaccess` spec section

Status: OPEN (2026-11-11) | Priority: P3

Type: research (reading), then docs and measurement. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G17 (T15).

## Facts

- `references/emutos/tosvars.ld` has 83 numeric assignments; 43 are written by none of `populate_sysvars` (`tos-api/src/bdos/process/loader.rs`), `PAGE0_HOOKS` (`tos-api/src/bdos/process/hooks.rs`) and `write_page0_vectors` (`tos-api/src/bios/vectors.rs`), among them `_v_bas_ad`, `_vbclock`, `_frclock`, `_hdv_init`, `_hdv_boot`, `_bconin_vec`, `_nflops`. Only 8 are read on the oracle (`Setexc` cases). Whether other code keeps them current was not searched.
- `docs/COMPATIBILITY.md:1880-1907` promises "compatible in value and semantics" without a per-variable list; `$44E` is named at `:1882`.
- XBIOS `nvmaccess` (0x2e) is mapped and measured but only the inventory names it in `docs/api/xbios/`.

## Work

1. Step 1: read the other writers (timer, VBL, console) for the 43 variables.
2. Write the table (served / hardware / missing), add a `peek` or `Supexec` read case type for the served ones, add the `nvmaccess` spec section.

## Acceptance

- Every variable of `tosvars.ld` classified in a spec table; served ones have oracle read cases.

## Table and NVMaccess section landed; owner decisions (2026-11-19)

The researcher returned GAPS (`0c8941107`, landed as `71d0d925e`): `docs/api/bios/_sysvars-contract.md` (84
assignments: served 37, hardware 25, missing 17, free 3, not variables 2) and `docs/api/xbios/2e-nvmaccess.md`,
read from source, nothing measured, the inferred statements listed as open points in both files.

Owner 2026-11-19, on the lead's six recommendations ("ich gehe mit deinen empfehlungen mit"):

- Land the two files as they are.
- A developer ticket for the return stubs, the frame counters and `_timer_ms`: TASK/BIOS20.md. The other unserved
  variables become rows of `docs/DEVIATIONS.md`: TASK/SPEC08.md.
- Section 7 of `docs/api/bios/_boot-init.md` is replaced by a pointer to the new table (SPEC08).
- The paragraph for `docs/COMPATIBILITY.md` is entered in a wording shown to the owner first (SPEC08).
- The oracle read cases for the served variables get their own ticket after BIOS20: TASK/BIOS21.md.
- The class "free" stays.

This ticket stays OPEN: its acceptance line on the read cases is met by BIOS21.

## Read cases delivered (2026-11-20)

TASK/BIOS21.md is DONE (`3b3a27e9e`): the table `tosdump/cases/bios-sysvars-r1.tsv` has a `peek` case for every
served row (54 cases for 51 rows), frozen references for both oracle profiles, and NewTOS equals both in all of
them (`measurements/measurement-2026-11-20-bios21-test/RUNINFO.TXT`). TASK/BIOS20.md is DONE (`cdfb6a823`). With
that both acceptance lines of this ticket are met; as a ticket without a `fix`/`feat` commit of its own it is
brought to the owner for closing.

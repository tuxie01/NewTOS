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

# `v_clsvwk` — VDI opcode 101 (close virtual workstation)

> Phase 00b spec. Source: EmuTOS @ `1a7484ab` —
> `vdi/vdi_control.c:462` (`vdi_v_clsvwk`); `v_clswk` (opcode 2) at
> `:573`. Calling convention: [`../_dispatcher.md`](../_dispatcher.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode 101; closes the *named* virtual workstation; no
> inputs/outputs; closing the physical handle or an already-closed handle
> is a silent no-op. **Reference:** `Mfree`, `vwk_ptr[]`/`CUR_WORK`/chain
> mechanics — NewTOS reimplements these.

Closes the virtual workstation named by `CONTRL[6]`: frees its `Vwk`,
rebuilds the workstation chain, and repoints `CUR_WORK` at the physical
workstation. Mirror image of [`100-v_opnvwk`](100-v_opnvwk.md).

## EmuTOS source

**Primary location:** `vdi/vdi_control.c:462` — `vdi_v_clsvwk(Vwk *vwk)`
(reached via `jmptb2[1]`, `vdi_main.c:76`; the dispatcher has already
resolved `CONTRL[6]` → `vwk`).

**Helpers called:** `build_vwk_chain()` (`:390`), `Mfree()` (GEMDOS 0x49).
**Globals read/written:** `vwk_ptr[handle]`, `CUR_WORK`, `phys_work`.

## Source extraction summary

`v_clsvwk` takes `handle = vwk->handle`. If it is `VDI_PHYS_HANDLE(1)` it
returns immediately (the physical workstation cannot be closed this way).
If the slot is already NULL it returns (already closed). Otherwise it
clears `vwk_ptr[handle]`, rebuilds the workstation chain, sets
`CUR_WORK = &phys_work` (matching what NVDI does — a valid open
workstation, not necessarily the closed one's precursor), and `Mfree`s
the `Vwk`.

Note the dispatcher only calls the handler if `CONTRL[6]` resolved to a
live `Vwk` (`vdi_main.c:139-143`); a bad handle is a no-op *before*
`v_clsvwk` is even reached.

## Trap interface

| | |
|--|--|
| Opcode | `CONTRL[0] = 101` |
| Handle | `CONTRL[6]` — the workstation to close |
| `# INTIN / PTSIN / INTOUT / PTSOUT` | 0 / 0 / 0 / 0 |

Native ABI: GEM trap `svc #2`, selector 115 in `x0`, `VDIPB*` in `x1`.

## Inputs / Outputs / Errors

No inputs (beyond the handle), no outputs, no return value, no error.
Closing the physical handle, or an already-closed handle, is a silent
no-op.

## Verification criteria

- **Close frees the slot:** after `v_clsvwk` on a valid virtual handle
  `h`, `vwk_ptr[h]` is empty and a subsequent `v_opnvwk` may reuse `h`.
- **`CUR_WORK` stays valid:** after any `v_clsvwk`, `CUR_WORK` points at
  the physical workstation.
- **Physical handle protected:** `v_clsvwk` with `CONTRL[6] = 1` does
  nothing (no free, no crash).

## Edge cases discovered in source

1. **Cannot close the physical workstation** via `v_clsvwk` — handle 1 is
   rejected (`vdi_control.c:468-469`). Use `v_clswk` (opcode 2) for the
   full teardown.
2. **`CUR_WORK` is repointed at `phys_work`, not the precursor** — EmuTOS
   deliberately simplified the Atari-TOS behaviour (it just guarantees a
   valid open workstation) (`:477-488`).
3. **A bad/closed handle is a no-op** — resolved either by the dispatcher
   (bad handle) or by the NULL check inside `v_clsvwk` (already closed).

## Related opcode — `v_clswk` (opcode 2, close physical workstation)

`vdi_v_clswk` (`vdi_control.c:573`, `jmptb1[1]`) is the **full teardown**:
it `Mfree`s **every** open virtual workstation (`vwk_ptr[2..]`), sets
`CUR_WORK = vwk_ptr[VDI_PHYS_HANDLE]`, then calls `timer_exit()`,
`vdimouse_exit()`, and `esc_exit(vwk)` (→ `escfn3`, back to alpha/console
mode). Inputs/outputs: 0/0/0/0.

- **`vdimouse_exit`** (`vdi/vdi_mouse.c:622-645`): `user_but = user_mot =
  user_cur = just_rts` (`user_cur` too, not `mov_cur`) and `user_wheel =
  just_rts` (`:624-628`), then `Initmous(0, 0, 0)` (`:634`), whose type 0
  sets the BIOS `mousevec` to `just_rts` (`bios/mouse.c:47-48,119-121`): no
  packet moves or draws the pointer, and none reaches the AES's
  `far_bcha`/`far_mcha`, until the next `v_opnwk`. The pointer itself
  (position, hide count, form, a visible sprite and its save-under) and
  `mousexvec` (extra buttons) stay. **`timer_exit`** (`vdi/vdi_misc.c:144-150`)
  restores the old `etv_timer` and leaves `tim_addr` as it is.
- **NewTOS:** the AES calls it at every session end (`gsx_wsclose`,
  `aes/gemgsxif.c:191-194`, `aes/geminit.c:749`). `tos-api/src/vdi/workstation.rs`
  `v_clswk` closes every virtual workstation, keeps the physical one (the
  framebuffer stays up), runs `mouse::vdimouse_exit` (packets dropped,
  vectors `just_rts`, the AES's direct `user_but`/`user_mot` calls stop) and
  returns the console to text mode (`esc_exit`). `timer_exit` has nothing to
  undo (no `etv_timer` chain, ADR-031). *Correction (V53, 2026-11-07):* this
  note called `v_clswk` "effectively a no-op" and NewTOS ran no mouse exit —
  the pointer kept moving and drawing after it (wkprobe C1,
  `measurements/measurement-2026-11-07-v18-t3/RUNINFO.TXT`, VDI-WK-002); fixed and
  re-measured in `measurements/measurement-2026-11-07-v53/RUNINFO.TXT`.

## Dependencies

- `Mfree` (GEMDOS 0x49) for the `Vwk`; for `v_clswk` also `timer_exit`,
  `vdimouse_exit`, `esc_exit`.

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check deferred.

## Related data structures

- [`docs/api/structures/vwk.md`](../../structures/vwk.md) — the `Vwk` being freed.
- [`_state.md`](../_state.md) — the `vwk_ptr[]`/`CUR_WORK`/chain model.

## NewTOS implementation notes (Phase 12)

- Free the per-handle `Vwk` and keep `CUR_WORK` valid (→ physical). The
  chain relink is **reference**; NewTOS's handle registry does the
  equivalent.
- `v_clswk` must **not** tear down the framebuffer — keep the physical
  device open, switch the mouse off (`vdimouse_exit`) and return the console
  to text mode.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`Mfree`'d `Vwk` chain, `esc_exit`
> back to alpha mode) at the NewTOS static handle table over the BCM2711
> VideoCore framebuffer. The **contract is unchanged** — opcode 101, closes
> the named virtual workstation, physical handle protected, silent no-op on
> a bad/closed handle. Peripheral + driver: see `docs/HARDWARE-PROFILE.md`
> §2/§3.1.

- **Frees a static slot, not heap.** `v_clsvwk` releases the handle's slot
  in the fixed 16-entry table (`tos-api/src/vdi/workstation.rs:186-189` →
  `vwk::free_handle`, `vwk.rs:238`); there is no `Mfree` because the `Vwk`s
  are static (`vwk.rs:9-20`), not `Mxalloc`'d. Handle 1 (physical) is
  protected by the resolve/`PHYS_HANDLE` logic.
- **Framebuffer stays up.** Closing a virtual workstation never touches the
  VideoCore framebuffer; the physical device remains the shared draw
  target for the remaining workstations. `v_clswk` (opcode 2) likewise must
  not tear down the framebuffer in bring-up (keep the VideoCore buffer
  allocated, return the console to text mode).
- **1080p later.** Lifecycle-only; geometry-independent.

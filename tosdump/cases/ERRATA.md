# Errata to the frozen case tables

The `.tsv` case tables in this directory are bound by CRC to the dumps
recorded from them (`tosdump/reference/`, `measurement-*/`). A change to a
table — even to a `#` comment — invalidates that binding, so a table that
turns out to say something wrong in its comments is **not edited**. The
correction lives here, keyed by table and line, and a future table revision
carries it into its text.

Why here and not in `COVERAGE-AES.md`: that file is the design and
prediction record of the rounds; an erratum is a statement about a frozen
artefact, and it must be found from the artefact, not from the round that
produced it. One file for all tables keeps that lookup in one place.

---

## `aes-r2b-rlist.tsv` (CRC `B4CC4A5A`, sha256 `a81a7a6c…`) — header comment, item 1

**Where:** lines 69–72, the sentence *"g_x and g_y keep whatever the last
rc_copy left, and if the list was empty from the start they keep whatever
the caller's buffer held. The poison is what makes that visible."*

**What is wrong:** the two words at the end of the list do **not** come
from the caller's buffer. They come from EmuTOS's dispatcher: `xif` keeps
a **local, uninitialised** `WORD int_out[O_SIZE]` (`aes/gemsuper.c:432`),
`crysbind` writes `RET_CODE` into it and `wm_get` writes only `g_w`/`g_h`
for the end of the list (`aes/gemwmlib.c:1240`), and `xif` copies
`OUT_LEN` words back **unconditionally** (`gemsuper.c:444-445`). The caller's
poisoned buffer is therefore overwritten in all five words, and what
lands in `int_out[1..2]` is whatever the supervisor stack held under
`int_out` at that moment.

**Evidence (A15, 2026-09-10, run on the TT):** all 15 records of the
table; the end-of-list records return `1, 31, 63, 0, 0` (cases 127, 133)
and `1, 0, 19, 0, 0` (case 204) in the **same** run — two different
value pairs from one poison pattern, which a "kept from the caller"
reading cannot produce — and one value survives a case boundary and a
reset. Table in `TASK/A15-STATUS-UNFINISHED.MD` (rows 127/133/204).

**What follows for reading the dumps:** the poison shows **that** the
caller is overwritten, not what the caller keeps. The recorded values
`31, 63` / `0, 19` are stack contents, not an answer, and must not be
used as expectations; the reference harness states the two words itself
(`tos-api/tests/aes_reference/harness.rs`, the `WF_OWNER`/end-of-list
notes). The dumps remain valid as recorded.

**Spec:** `docs/api/aes/window-graphics.md`, "The rectangle-list
iteration" and "Edge cases discovered in source" (A18, 2026-09-19) state
the two-word rule with `gemwmlib.c:1240`.

---

## `aes-m07-parameter.tsv` (CRC `0F46665B`, sha256 `35ca5bca…`) — header comment, "WHAT THESE CASES WILL DIFFER IN"

**Where:** lines 39–44, *"WHAT THESE CASES WILL DIFFER IN, by construction
and not by defect: int_out[1..4] are the machine's own mouse position,
button byte and shift state at the instant of the call. They are stable
within a machine (both runs are headless and nothing moves the mouse) and
have no reason to agree between a TT and a Pi."*

**What is wrong:** in a headless run nothing moves the pointer, so
`int_out[1..2]` are the VDI's park position, and that is a defined value:
`vdimouse_init` sets `GCURX = xres / 2`, `GCURY = yres / 2`
(`vdi/vdi_mouse.c:582-583`), where `xres`/`yres` are `DEV_TAB[0]`/`[1]` =
the **last coordinate**, `V_REZ_HZ - 1`/`V_REZ_VT - 1`
(`vdi/vdi_defs.h:94-95`, `vdi/vdi_control.c:203-204`) — 319/239 at
640×480. NewTOS halved the pixel count (320/240). The difference the 2026-09-13
runs showed in `int_out[1..2]` (10 records each) was that product defect,
not a device value; it was fixed by V17 (`eabbb38c`).

**Evidence (V17, 2026-09-14):** the same table, unchanged, run on QEMU
`raspi4b` headless at 640×480 against `tosdump/reference/EMUTOS14-AES-m07.BIN`
— `int_out[1]` and `int_out[2]` differ in 10 of 20 records before the fix
and in **0** after it; no `int_out` word differs any more, only the six
`AESGLOBAL` words every table shows
(`measurements/measurement-2026-09-14-v17/RUNINFO.TXT:48,80-90`). Receipt
`TASK/V17-STATUS-DONE.MD`, "Die Gegenmessung".

**What follows for reading the dumps:** in an unattended run
`int_out[1..2]` of this table are an expectation, `((w-1)/2, (h-1)/2)` of
the screen the run had, not noise; `int_out[3..4]` (buttons, shift state)
did not differ either. The dumps remain valid as recorded. Why the comment
itself stays: `TASK/M10.md`.

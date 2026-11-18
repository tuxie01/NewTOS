# XBIOS23 — receipt (developer): `Getrez` answers `ST_HIGH` (2), ADR-017 Revision 3 applied

Role: developer
Agent: developer-xbios23 (Opus, agent a677ee71d8572747e)

Date 2026-11-18. Branch `xbios23` from `main` @ `fa44c05e2` (worktree
`target-agents/agent-a677ee71d8572747e/wt`). Spec: `docs/api/xbios/04-getrez.md` (edge case 2, "Coherence with
`VsetMode(-1)`"); decision: `TASK/XBIOS23.md` "Owner decision 2026-11-12" (A: 2; `VsetMode(-1)` as a query; Revision 3
of ADR-017 from the draft). EmuTOS read: `references/emutos/bios/screen.c:948-987` (`atari_getrez`), `:1076-1085`
(`getrez`); `references/emutos/include/biosdefs.h:126-128` (rez codes), `:145-157` (`VIDEL_*`).

## Changes

- `tos-api/src/xbios/screen.rs`: `getrez` returns `ST_HIGH` (2) instead of `TT_MEDIUM` (4); the constant
  `TT_MEDIUM` is replaced by `ST_HIGH` and `VIDEL_COMPAT` (0x80); a build-time `assert!` ties the value to
  `FALCON_MODE_WORD` (XBIOS22): `ST_HIGH` is EmuTOS's answer only while the word has `VIDEL_COMPAT` clear
  (`screen.c:956-972`), so changing the word to an ST-compatible one fails the build instead of leaving `Getrez`
  incoherent. Module and function doc comments rewritten (ADR-017 Revision 3).
- `kernel/src/selftest_xbios.rs`: the expectation text of the `[xbios]` line reads `rez=2 ST_HIGH`.
- `tos-api/tests/xbios23_getrez.rs` (new, 4 tests): `Getrez` = 2 through the dispatcher and directly; = 2 without a
  framebuffer (no error path); agreement with `VsetMode(-1)` by an independent copy of EmuTOS's Videl mapping
  (`VIDEL_COMPAT` clear → 2); `Setscreen(-1,-1,rez)` for rez 0,1,2,3,4,6,7,9,-1 returns 0 and leaves `Getrez` at 2.
- `docs/MEMORY.md`: ADR-017 gets "Revision 3 (2026-11-12)" (the ticket's draft text, applied: decision, the
  alternatives table, consequences) and a status-line sentence. The "A" row's "Against" cell now notes that the
  `VsetMode(-1)` query (XBIOS22) is answered; the draft still said XBIOS22 excluded `VsetMode`.
- `docs/api/xbios/04-getrez.md`: "NewTOS behavior" and the Pi binding rewritten for Revision 3; the "Under review"
  note and the resolved open point removed; the `Setscreen` round-trip verification criterion removed (as the
  spec's coherence section asked: `rez` is ignored).
- `docs/api/xbios/05-setscreen.md`: the one sentence naming the `Getrez` value (now `ST_HIGH`).
- `docs/COMPATIBILITY.md`, "XBIOS Falcon video": a `Getrez` bullet: case 87 expected EQUAL on `falcontc`, differs
  from `tt` (4): ACCEPTED, NewTOS claims the Falcon.
- Line anchors moved (screen.rs grew by 13-16 lines above the cited symbols): `docs/COMPATIBILITY.md:1914`,
  `docs/HW-TEST-LOG.md:128`, `docs/api/xbios/{06-setpalette,07-setcolor,58-vsetmode,59-vgetmonitor,5b-vgetsize,
  5e-vgetrgb}.md`, `docs/audits/2026-10-28-xbios-system.md:234`, `TASK/XBIOS21.md:14-15` and this ticket's own two
  `getrez` cites (each anchor-only, required by the pre-commit doc-check). `docs/analysis/deps/mapping.tsv`: 38 `design:ADR-0xx@docs/MEMORY.md:<n>` cites moved by +46
  for ADR-029/030/031 and the line 2195 cite (the ADR-017 insertion shifts them). Other cites (dated snapshots
  whose anchors still resolve, unanchored legacy cites) left unchanged.

Spec satisfied: `docs/api/xbios/04-getrez.md`, edge case 2, last row (`VIDEL_COMPAT` clear → 2) applied to the
mode word `0x001C`; "Coherence with `VsetMode(-1)`".

No caller in the tree depends on 4 in a way the ticket does not name: `getrez` is reached only through the
dispatcher (`tos-api/src/xbios/mod.rs:75@GETREZ`); the kernel's own AES (`tos-api/src/aes/gsxif.rs`) does not call it; the
VDI's `v_opnwk` reads no mode request (so `Getrez()+2` = 4 in HighWire / the `xgdos` overlay changes nothing on
NewTOS); TeraDesk replaces `Getrez` by `VsetMode(-1)` behind `_VDO` (`sdk/overlays/teradesk/video.c:173-209@Getrez`).

## Verified (developer)

- **Builds** (`cargo xtask build --board qemu-virt` and `--board rpi4`, debug, each after touching the changed
  files): OK; rustc warnings `tos-api` 64, `kernel` 93 on both boards, equal to the XBIOS22 receipt's baseline;
  none in a changed file. Delta 0.
- **Clippy**: `cargo clippy -p tos-api --lib` 73; `-p kernel --features board-rpi4` and `--features
  board-qemu-virt`: kernel 121, tos-api 73; `-p tos-api --lib --tests`: lib test 78, the new test binary has no
  warning. Equal to the XBIOS22 baseline (73/121/78). Delta 0.
- **Host tests:** `cargo xtask test` (full): 2501 passed, 0 failed, 7 ignored (XBIOS22 receipt 2497 / 0 / 7; +4
  in `xbios23_getrez`).
- **`cargo xtask deps --check-mapping`:** "1968 rows, all resolve, every class is valid; every bios/, bdos/, vdi/,
  aes/ function has a row".
- **doc-check** `cargo xtask doc-check --staged`: 0 failures introduced; the legacy failures on unchanged lines
  (e.g. 5 in `docs/MEMORY.md`) are cites into gitignored trees that `worktree-setup` does not link into a worktree
  (the ldg and mintlib checkouts under `sdk/`, the applications folder).
- Not run: QEMU boot, tosdump (tester's).

## Tester handoff

1. **Host:** `cargo xtask test` (the 4 tests of `tos-api/tests/xbios23_getrez.rs`).
2. **`tosdump/cases/bios-r1.tsv`** on QEMU raspi4b with this branch's kernel against both references (as
   `measurements/measurement-2026-11-18-xbios22-test/`): case 87 `Getrez()` = `0x0002` — EQUAL on `falcontc`
   (was DIFF, 4 vs 2); differs from `tt` (4; was EQUAL): ACCEPTED (`docs/COMPATIBILITY.md` "XBIOS Falcon
   video"). Every other case as in the XBIOS22 test run.
3. **Boot selftest** (`kernel/src/selftest_xbios.rs`, `run`; unwired like the other selftests, wire it in scratch
   only): `[xbios] ... Getrez=2 (expect ..., rez=2 ST_HIGH)`.
4. **TeraDesk** (optional, XBIOS22 `vid-vdo` scenario): unchanged behaviour expected, it reads `Getrez` and then
   overrides it from `VsetMode(-1)` behind `_VDO` (`currez = TT_MEDRES`).
5. **Real Pi 4 only:** nothing; the value is board-independent.

## Proposed `docs/STATUS.md` line

`2026-11-18 — XBIOS23 implemented on branch xbios23: Getrez answers ST_HIGH (2), EmuTOS's code for the
non-ST-compatible truecolor Videl mode 0x001C that VsetMode(-1) reports (bios-r1 case 87 EQUAL on falcontc expected;
tt differs, ACCEPTED); ADR-017 Revision 3 applied in docs/MEMORY.md, replacing Revision 2's TT_MEDIUM (4).`

## Open

- Acceptance "no case equal in the full XBIOS run differs anew" cannot hold as worded: case 87 was EQUAL against
  the `tt` reference (both 4) and now differs there (2 vs 4), a direct consequence of the decided value. The draft
  said "tt stays measured-different, 4"; the measured per-case files show it EQUAL on tt
  (`measurements/measurement-2026-11-12-xbios-full/per-case.tsv:41`). Recorded ACCEPTED in `docs/COMPATIBILITY.md`
  ("XBIOS Falcon video"), like XBIOS22's tt cases; the lead confirms or rewords the criterion.

## Out-of-scope observations

- `docs/HARDWARE-PROFILE.md:65,135` and `docs/INVENTORY.md:499,558` still describe `Getrez` as `TT_MEDIUM` / an
  open question.
- `docs/api/emucon/05-config-strings-hardware.md:100` (German) still says `Getrez` returns `TT_MEDIUM` = 4.
- `docs/api/xbios/58-vsetmode.md:62` still says "the radio set today (Getrez 4 -> rmap, `ARCH06` run)" (history of
  the run, not edited).
- `tosdump/cases/bios-r1.tsv:87@Getrez` comment still reads "(TT-Medium: profile)".
- `docs/api/_discrepancies.md` D-076/D-078 (append-only log) still end at "open Lead decision"; no closing entry
  added.

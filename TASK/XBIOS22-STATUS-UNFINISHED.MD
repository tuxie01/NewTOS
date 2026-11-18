# XBIOS22 — receipt (developer): Falcon video queries for the Pi screen, `_VDO` published

Role: developer
Agent: developer-xbios22 (Opus, agent a368e145e5259e2c6)

Date 2026-11-18. Branch `xbios22` from `main` @ `a2abbfa77` (worktree
`target-agents/agent-a368e145e5259e2c6/wt`). Specs: `docs/api/xbios/58-vsetmode.md`, `59-vgetmonitor.md`,
`5b-vgetsize.md`, `5e-vgetrgb.md`, `07-setcolor.md`; decisions: `TASK/XBIOS22.md` "Owner decision 2026-11-12" (both),
"Spec step 2 (researcher, 2026-11-18)", "Owner decision 2026-11-18". EmuTOS read: `references/emutos/bios/videl.c`
`:42-46` (`videl_dflt_palette`), `:619-659` (`vsetmode`), `:664-670` (`vmontype`), `:724-757` (`vgetsize`),
`:862-909` (`vgetrgb`); `bios/videl.h:27-42` (`FRGB_*`); `bios/machine.c:363-367` (`setvalue_vdo`);
`include/cookie.h:27,67`.

## What changed

Code:

- `tos-api/src/xbios/screen.rs` (block at the end of the file, so no existing anchor into the file moves):
  `FALCON_MODE_WORD` = `0x001C`; `vsetmode` (query form `-1` → `0x001C`, otherwise `None`); `vgetmonitor` → 2
  (`MON_VGA`); `vgetsize` → `Host::screen().size_bytes` as a sign-extended LONG, `None` without a framebuffer;
  `FALCON_PALETTE` (the 16 `videl_dflt_palette` entries as `0x00RRGGBB`) and `vgetrgb` (limit 16, `-1` for
  `index < 0`, `count <= 0`, `index + count > 16`, buffer untouched on error, user-access fault path as
  `setpalette`). Module header names the new functions (same line count).
- `tos-api/src/xbios/mod.rs`: arms `VSETMODE` / `VGETMONITOR` / `VGETSIZE` / `VGETRGB` before the catch-all; a
  `None` from `vsetmode`/`vgetsize` gives the function number. Constants after the hook-installer constants.
  `Vfixmode` (0x5F), 0x5A, 0x5C, 0x5D keep the catch-all (comment updated).
- `tos-api/src/cookie.rs`: `VDO` tag and `VDO_FALCON` = `0x0003_0000` (after `MCH_NOHARD`), `set(VDO, VDO_FALCON)` as
  the last seeded cookie; `seed_defaults` doc comment rewritten in the same seven lines (anchors stay).
- `kernel/src/selftest_cookie.rs`: `Getcookie(_VDO) = 0x00030000` check (19 checks instead of 18), comments on the
  seeded count.
- `tos-api/src/bdos/process/loader.rs`: the `write_cookie_jar` capacity comment ("5 cookies") updated.
- `tos-api/tests/xbios22_falcon_video.rs` (new): 11 host tests (below).

Docs:

- `docs/MEMORY.md`: ADR-012 amendment 2026-11-18 (the text of "Spec step 2" (c), with the owner's 2026-11-18
  measuring condition). The 14 inserted lines shift every later line: the 45 `docs/MEMORY.md:N` cites of
  `docs/analysis/deps/mapping.tsv` past line 447 are moved by 14 so each still lands in its ADR.
- `docs/analysis/deps/mapping.tsv`: `vsetmode`, `vmontype`, `vgetsize`, `vgetrgb` → `mapped` to the new
  `screen.rs` functions; `setvalue_vdo` → `mapped` to `seed_defaults`; the note of the other `bios/videl.c` rows
  says which Videl calls still answer the function number.
- `docs/COMPATIBILITY.md`: new entry "XBIOS Falcon video" (VsetMode/Vfixmode sanctioned deviation, VgetSize case 213
  difference, the tt-profile differences); cookie rows 41 and 45, "seeds six cookies", and the "absent Atari hardware"
  entry no longer says "no `_VDO` cookie".
- `docs/DEVIATIONS.md`: three rows (VsetMode(mode)/Vfixmode, VgetSize(mode), Videl register access after `_VDO`).
- Specs: implementation sections in `58-vsetmode.md`, `5b-vgetsize.md`, `5e-vgetrgb.md`; `59-vgetmonitor.md` open
  point closed; `_dispatcher.md` (Videl slots); `_cookie-jar.md`, `docs/HARDWARE-PROFILE.md`,
  `docs/sdk/08-porting-checklist.md` (the `_VDO` copies the amendment lists).
- Anchor-only updates for lines this commit moved: `docs/api/xbios/00-initmous.md` (`INITMOUS`, `note_installer`),
  `docs/api/xbios/_dmasound-group.md` (`@SND`, `@other`), `TASK/XBIOS22.md` (`@other`).
- `TASK/XBIOS22.md` line 3: `IMPLEMENTED`.

Not changed: `tosdump/cases/jar-r1.tsv` (frozen, case id = line number); `Getrez` (XBIOS23).

## Verified (developer)

- **Builds** (`cargo xtask build --board rpi4` and `--board qemu-virt`, debug): OK; rustc warnings `tos-api` 64,
  `kernel` 93 on both boards, equal to the same worktree at `a2abbfa77` before the change. Delta 0.
- **Clippy**: `cargo clippy -p tos-api --lib` 73 = 73; `cargo clippy -p kernel --features board-rpi4` and
  `--features board-qemu-virt`: kernel 121 = 121, tos-api 73 = 73; `cargo clippy -p tos-api --lib --tests`: lib
  test 78 = 78, the new test binary has no warning. Delta 0.
- **Host tests:** `cargo xtask test` (full): 2497 passed, 0 failed, 7 ignored (main baseline 2486 / 0 / 7; +11 in
  `xbios22_falcon_video`).
- **`cargo xtask deps --check-mapping`:** "1968 rows, all resolve, every class is valid; every bios/, bdos/, vdi/,
  aes/ function has a row".
- **doc-check:** `cargo xtask doc-check --staged`: 0 failures in the staged docs (43 legacy failures on unchanged
  lines: gitignored trees absent from the worktree and `docs/HARDWARE-PROFILE.md`'s old driver paths), and "2718 of
  their cites name one of the 21 changed paths, 0 broken by this commit".
- **`cargo xtask status-check --new-only`:** 1 checked, 0 rejected.
- Not run: QEMU boot, tosdump, TeraDesk (tester's).

Host tests (`tos-api/tests/xbios22_falcon_video.rs`), documented value / documented error per function:

| Function | Value | Error / edge |
|---|---|---|
| `VsetMode` 0x58 | `VsetMode(-1)` = `0x001C` | `mode` 0x1C, 0, 0x88, -2, -32768, 32767 → `0x58`, query unchanged |
| `Vfixmode` 0x5F | — | `Vfixmode(0x1C)`, `(-1)` → `0x5F` |
| `VgetMonitor` 0x59 | 2 | no error path; 2 also without a framebuffer |
| `VgetSize` 0x5B | `stride * height` (test 7744 x 1080) for mode 0x1C, 0, 0x88, 0x1FF, -1 | no framebuffer → `0x5B` |
| `VgetRGB` 0x5E | `(0,16)` = 0 and the 16 longs of case 214; `(3,2)`, `(15,1)` | `-1` for (-1,1), (-32768,16), (0,0), (0,-1), (0,17), (15,2), (16,1), (8,9), (32767,32767), buffer untouched; `Setcolor` does not change the table |
| `_VDO` | `Ssystem(S_GETCOOKIE,'_VDO',&v)` = 0, `v` = `0x00030000`; value form `0x30000` | — |

## Tester handoff

1. **Host:** `cargo xtask test` (the 11 tests above).
2. **`tosdump/cases/bios-r1.tsv` on QEMU raspi4b** with this branch's kernel, against both references (as
   `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`), expected per case:
   - 211 `VsetMode(-1)`: `0x001C` — EQUAL on `falcontc`; differs from `tt` (`0x0058`), ACCEPTED (COMPATIBILITY
     "XBIOS Falcon video": NewTOS claims the Falcon).
   - 212 `VgetMonitor`: `0x0002` — EQUAL on `falcontc`; differs from `tt`, ACCEPTED.
   - 213 `VgetSize(0x1C)`: `1228800` (`0x12C000`, one 640x480 XRGB8888 screen; the raspi4b screen's
     `size_bytes`) — differs from `falcontc` (614400) and `tt` (`0x5B`) by decision, ACCEPTED.
   - 214 `VgetRGB(0,16,b0)`: return 0 and the 64 captured bytes `00FFFFFF 00FF0000 0000FF00 00FFFF00 000000FF
     00FF00FF 0000FFFF 00BBBBBB 00888888 00AA0000 0000AA00 00AAAA00 000000AA 00AA00AA 0000AAAA 00000000` — EQUAL on
     `falcontc` (the `2:64.L` capture is compared element by element as LONGs, `docs/TOS-REFERENCE-DUMPER.md`
     "Capture through the returned pointer"); differs from `tt`, ACCEPTED.
   - 215 `Vfixmode(0x1C)`: `0x005F` unchanged — differs from `falcontc` (`0x1C`) and `tt` (`-32`), ACCEPTED.
   - Every other case: as in the 2026-11-12 run (no new difference).
3. **`tosdump/cases/jar-r1.tsv`:** case 45 `_VDO` = `0x00030000` EQUAL on `falcontc` and disk, differs from `tt`
   (`0x00020000`, ACCEPTED); case 41 `jar:*` = `NTOS _MCH _IDT _AKP EdDI _VDO` (6; 7 with `_SND`), ACCEPTED.
4. **Gate for the `_VDO` value (owner 2026-11-18):** the TeraDesk `vid-vdo` scenario of
   `measurements/measurement-2026-10-29-arch06/method/` (`vid-vdo.spec` / `.script`, `build-trace.sh`, `series.sh`),
   on this branch's kernel and **without** the `MCHVDO.NCP` injection (the cookie is now seeded; the `novdo` card
   fits). Expected (read from `video.c`, not measured): the trace shows `14:0x58 VsetMode w:0xffff -> 0x1c` and
   `14:0x59 VgetMonitor -> 0x2`; Options → Video options shows the VGA labels with VGA-80-column (TT_MEDRES) set,
   Overscan **not** ticked, Doubleline **not** ticked, Colours 64K; OK without a change raises **no** alert and no
   AES restart. A wrong dialog state or an alert = FAIL; the value then goes back to the owner.
5. **Cookie selftest:** `kernel/src/selftest_cookie.rs` is not called from any boot path; the 2026-10-29 run wired it
   with `measurements/measurement-2026-10-29-arch06/method/selftest-wire.py` (scratch only). Wired the same way: `[cookie] === Cookie jar PASS (19/19) ===`,
   with `Getcookie(_VDO) = 0x00030000 (VDO_FALCON): OK`.
6. **Real Pi 4 only:** `VgetSize` = 8 294 400 at the default 1920x1080 (or `stride * height` of the configured
   mode); everything else is board-independent.

## Proposed `docs/STATUS.md` line

`2026-11-18 — XBIOS22 implemented on branch xbios22: VsetMode(-1) = 0x001C, VgetMonitor = 2, VgetSize = bytes of
the current 32-bpp screen (0x5B without a framebuffer), VgetRGB = the 16-entry Falcon table; VsetMode(mode) and
Vfixmode keep the function number (COMPATIBILITY "XBIOS Falcon video"); _VDO = 0x00030000 seeded (ADR-012 amendment
2026-11-18), decided after the tester's TeraDesk vid-vdo run.`

## Open

- Acceptance "No case equal in the full XBIOS run differs anew" cannot hold as worded: cases 211, 212, 213 and 214
  were EQUAL against the `tt` reference (both sides answered the function number) and now differ there, because the
  owner's decisions claim the Falcon. Recorded ACCEPTED in `docs/COMPATIBILITY.md` ("XBIOS Falcon video"), as the
  DMA-sound entry does for its `tt` differences; the lead confirms or rewords the criterion.

## Notes

- `VgetMonitor`, `VsetMode(-1)` and `VgetRGB` answer the same with or without a framebuffer; only `VgetSize` falls
  back to the function number (decision item 4 covers only `VgetSize`). EmuTOS has no such split (`has_videl`).

## Out-of-scope observations

- `docs/api/emucon/05-config-strings-hardware.md:99` (German) still says "`_VDO` fehlt absichtlich"; not edited
  because touching it requires translating the file.
- `docs/api/xbios/_dmasound-group.md:754` still says `seed_defaults` leaves out `_VDO`/`_SND` (stale since SND05 for
  `_SND`, now also for `_VDO`).
- `kernel/src/selftest_cookie.rs` is not wired into any boot path (only the module declaration in `kernel/src/lib.rs`).
- The 185 plain `docs/MEMORY.md:N` line cites outside `mapping.tsv` past line 447 now point 14 lines early (no anchor
  scheme for MEMORY.md; doc-check only checks that the line exists).
- `cargo xtask doc-check --fix-anchors` (TOOL49) does not exist yet; the anchors above were moved by hand.

# XBIOS22 — Falcon video queries: `VgetMonitor`, `VgetSize`, `VgetRGB` answer for the Pi screen

Status: DONE (2026-11-18) | Priority: P2 | Commit: f6bdafa47 | Receipt: XBIOS22-STATUS-DONE.MD

Type: research (spec), then developer. Owner decision 2026-11-12, from the full XBIOS run
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`.

## Owner decision 2026-11-12

"müssen wir anders lösen da wir die config.txt dazu anpassen müssen. Ausser VgetMonitor, VgetSize, VgetRGB brauchen
wir das nicht." (We have to solve this differently, since that would mean adapting config.txt. Apart from VgetMonitor,
VgetSize, VgetRGB we do not need it.)

- `VgetMonitor` (0x59), `VgetSize` (0x5B), `VgetRGB` (0x5E): implement, answering for the Pi's screen.
- `VsetMode` (0x58) and `Vfixmode` (0x5F): not implemented; a mode switch at run time would need `config.txt`
  (ADR-017). They keep answering their function number; a sanctioned-deviation entry in `docs/COMPATIBILITY.md`
  records it (part of this ticket). The other Videl calls (0x5A, 0x5C-0x5D, ...) are not in scope.

## Facts

- `tosdump/cases/bios-r1.tsv` cases 211-215 differ (falcontc; 215 also tt): oracle `VsetMode(-1)` 0x1C,
  `VgetMonitor` 2, `VgetSize(0x1C)` 614400, `VgetRGB(0,16,…)` 0 plus the palette, `Vfixmode(0x1C)` 0x1C (tt -32);
  NewTOS answers the function number and leaves the buffer untouched (catch-all arm,
  `tos-api/src/xbios/mod.rs:162@other`).
- EmuTOS: `references/emutos/bios/videl.c:664` (`vmontype`), `:724` (`vgetsize`), `:862` (`vgetrgb`).
- The screen is a native 32-bpp framebuffer with a Falcon RGB565 VDI device pixel (ADR-024), geometry from
  `config.txt` (ADR-017).

## Open for the owner (from the spec step, before code)

- Which monitor type `VgetMonitor` reports for HDMI (EmuTOS knows mono, colour, VGA, TV), and which mode word
  `VgetSize` accepts (a Falcon mode word cannot express 1920x1080; whether it answers for the current screen only).
- What `VgetRGB` returns in a truecolor mode (EmuTOS: the Videl palette registers), and its relation to the
  `Setcolor` shadow (XBIOS21).
- Whether `VsetMode(-1)` (the query of the current mode, used by Falcon programs to detect the mode) may stay
  unanswered; the owner's decision above excludes `VsetMode` as a whole.

## Work

1. Researcher: spec entries for 0x59, 0x5B, 0x5E from `references/emutos/bios/videl.c` per
   `docs/SOURCE-ANALYSIS-PROTOCOL.md`, with the NewTOS mapping proposed for the points above; owner approval.
2. Developer: implement; COMPATIBILITY entry for `VsetMode`/`Vfixmode`.
3. Host tests per function: the documented return for documented input and the documented error.

## Acceptance

- Cases 212-214 equal where the spec says equal, otherwise listed in the COMPATIBILITY entry; 211 and 215 ACCEPTED
  by the entry. No case equal in the full XBIOS run differs anew, except cases 211-214 against the tt reference,
  which the COMPATIBILITY entry "XBIOS Falcon video" names as accepted (NewTOS claims the Falcon profile).
  (Reworded with the owner's approval 2026-11-18, "Ja ich bin damit einverstanden": the Falcon answers make the
  four tt cases differ by design.)
- `cargo xtask test` green; warnings delta 0; doc-check of the edited docs 0 failures.

## Spec step (researcher, 2026-11-12)

Written (new): `docs/api/xbios/59-vgetmonitor.md`, `docs/api/xbios/5b-vgetsize.md`,
`docs/api/xbios/5e-vgetrgb.md`; cross-reference `docs/api/xbios/07-setcolor.md` ("Do `Setpalette` and
`VgetRGB` ..."). Source: `references/emutos/bios/videl.c:664` (`vmontype`), `:724` (`vgetsize`), `:862`
(`vgetrgb`); measured cases 212-214 (falcontc: 2, 614400 for `0x1C`, return 0 plus the Falcon default
palette; tt: function number).

Facts worth the owner's attention:
- The function-number answer NewTOS gives today is **EmuTOS's own no-Videl answer** (`videl.c:666-667`,
  `:730-731`, `:869-870`), so it is a source-valid "no Videl" claim; the questions are only about claiming Videl.
- `0x58` read as a mode word is `OVERSCAN|VGA|80COL`, 1bpp (a valid-looking mono word); `0x59` as a monitor type is
  out of range 0..3.
- No Falcon mode word can express 1920x1080 (no width field, VGA table tops at 640x480); `VgetSize(0x1C)` is
  614400 by formula (worked in the spec).
- `VgetRGB` limit is 16 entries in every mode except 8bpp (`videl.c:875-876`); the truecolor answer is the
  Falcon default table at `0xFF` full scale, not derived from `Setcolor`.
- `VsetRGB` (0x5D) is outside this ticket: `VgetRGB` is then a constant table.

Owner questions (options with consequences are in the three spec files):
1. `VgetMonitor` for HDMI: VGA 2 (recommended, equals oracle), RGB 1, mono 0, TV 3, or keep the function number.
2. `VgetSize(mode)`: A Falcon-table formula (case 213 equal; ignores the real screen), B current screen only
   (case 213 differs), C hybrid. If B or C: bytes of the 32-bpp `Physbase` screen (8 294 400 at 1920x1080,
   `tos-api/src/host.rs:579@size_bytes`) or of the 16-bit VDI device pixel (4 147 200)?
3. `VgetRGB` in truecolor: A separate Falcon table of 16 entries (recommended, equals case 214), B derived from
   the `Setcolor` shadow, C from the VDI table, D `-1`.
4. `VsetMode(-1)` (query only): leave unanswered (returns `0x58`, reads as a mono mode word; `Getrez` option
   in XBIOS23 interacts), or answer the query with a fixed word (e.g. `0x1C`-class, geometry inexpressible) while
   `VsetMode(mode != -1)` stays out. Also: publish the `_VDO` cookie (ADR-012 amendment, open with TASK/ARCH06)?
   Programs gate the Videl calls on it (`sdk/apps/teradesk/video.c:140-160`).

## Owner decision 2026-11-12

Owner 2026-11-12: "ich stimme allem so zu wie du vorgeschlagen hast" (I agree to everything as you proposed), answering the lead's recommendations.

1. `VgetMonitor` answers 2 (VGA), equal to the oracle.
3. `VgetRGB`: A, a separate Falcon table of 16 entries.
4. `VsetMode(-1)` (query only) answers a fixed word; `VsetMode(mode != -1)` stays out. The exact word is proposed
   in the spec (`docs/api/xbios/`) with its reasoning before the developer builds it; the lead brings it back.

Owner 2026-11-12, second answer: "VgetSize : B" and "_VDO-Cookie ja" (_VDO cookie yes).

2. `VgetSize(mode)`: B, the size of the current screen only (case 213 then differs from the oracle; a
   COMPATIBILITY entry). Which byte count (32-bpp `Physbase` screen, 8 294 400 at 1920x1080, or the 16-bit VDI
   device pixel, 4 147 200) is still asked.
5. `_VDO` is published: an ADR-012 amendment (with [ARCH06](ARCH06.md)); value and text drafted from the spec before
   the developer builds it.

Superseded by the answer above: Still open: question 2 (`VgetSize`: A, B or C, and for B/C which byte count), and whether to publish `_VDO`
(ADR-012 amendment, with [ARCH06](ARCH06.md)). The lead made no recommendation on these; the owner answers them
separately.

## Spec step 2 (researcher, 2026-11-18): the three points the decisions of 2026-11-12 left to the spec

Written: `docs/api/xbios/58-vsetmode.md` (new), `docs/api/xbios/5b-vgetsize.md` (byte-count options),
`docs/api/xbios/59-vgetmonitor.md` and `docs/api/xbios/04-getrez.md` (cross-references). Drafts below; no decision
is taken here, `docs/MEMORY.md` and `docs/COMPATIBILITY.md` are not edited.

### (a) The word `VsetMode(-1)` answers — recommendation `0x001C`

`0x1C` = `VGA|80COL|TRUECOLOR` (bits 4, 3, 2-0 = 4): VERTICAL, COMPAT, OVERSCAN, PAL clear. Reasoning, field by
field with what TeraDesk and EmuTOS conclude, is in `docs/api/xbios/58-vsetmode.md`. Short form:

- It is the oracle's own word (case 211, falcontc): case 211 becomes EQUAL, no COMPATIBILITY row for the query.
- It agrees with the other answers decided 2026-11-12: `VgetMonitor` 2 (VGA bit), `Getrez` 2 and `sshiftmod` 3
  (COMPAT clear, `bios/screen.c:971-972`, `bios/videl.c:648-656`), a 16-bit truecolor VDI (bits 2-0 = 4, ADR-024).
- It is a fixed point of `vfixmode` on a VGA monitor (`bios/videl.c:934-943`).
- Its weakness is the same for every Falcon word: it names 640 columns; 1920x1080 is inexpressible; geometry comes
  from VDI (`58-vsetmode.md`, option table: B `0x11C`, C `0x3C`, D keep `0x58`, E a non-Falcon word, each with its cost).
- `VsetMode(mode != -1)` and `Vfixmode` keep the function number (owner decision); TeraDesk never calls
  `VsetMode` with a mode other than -1 (it uses `shel_write(SHW_RESCHNG)`, `sdk/overlays/teradesk/video.c:407-408@shel_write`).

### (b) The `VgetSize` byte count — recommendation B1, `size_bytes` (32-bpp `Physbase` screen)

Options in `docs/api/xbios/5b-vgetsize.md`: **B1** one screen as `Physbase` shows it, `Host::screen().size_bytes`
(8 294 400 at 1920x1080; 1 228 800 at the QEMU 640x480); **B2** `width*height*2` (4 147 200), the VDI device pixel.
B1: the allocation a program makes for "a screen of this mode" is the memory it writes through `Physbase`
(XRGB8888, `docs/api/xbios/02-physbase.md:231-246`), and a too-large buffer is harmless where a too-small one
overruns. B2 only fits a program that builds a 16-bit memory MFDB itself. No program in the repository calls
`VgetSize` (grep of `sdk/apps`, `sdk/overlays`, `programs`, `sdk/examples`, 2026-11-18; only the libc header
defines it), so the choice is not forced by a user. One open question remains (not decidable from EmuTOS):
the value before the framebuffer is up.

### (c) `_VDO` — recommendation `0x00030000` (`VDO_FALCON`)

**Value.** EmuTOS sets `VDO_FALCON` = `0x00030000` when the Videl is present (`bios/machine.c:363-367`,
`include/cookie.h:67`); falcontc measures it (`docs/COMPATIBILITY.md` row 45: tt `0x00020000`, falcontc
`0x00030000`). Options: **A `0x00030000`** (recommended): the Falcon the profile and the answers claim
(`VsetMode(-1)` `0x1C`, `VgetMonitor` 2, `VgetRGB`); jar case 45 EQUAL on falcontc. **B `0xFFFFFFFF`**
(`VDO_NOHARD`, EmuTOS's non-Atari value, `bios/machine.c:375`): TeraDesk's `vdo >> 16` is `0xFFFF` -> default
branch (`video.c:570-574@default` of the overlay), it never calls `VsetMode`/`VgetMonitor`, which defeats the owner's
decision to answer them. **C `0x00050000`** (TeraDesk's `ARA_VIDEO`, `video.c:94@ARA_VIDEO` of the overlay): the ARAnyM
claim, withdrawn for `_MCH` in ADR-012 (2026-10-28). **D `0x00020000`** (TT): TeraDesk's TT branch never asks
the Falcon calls.

**What TeraDesk did, and what changed since 2026-10-29** (`measurements/measurement-2026-10-29-arch06/RUNINFO.TXT`,
RESULTS 2 and 3):
- Without `_VDO` (then and today) TeraDesk takes the ST branch. On 2026-10-29 that branch crashed on Overscan + OK
  (Bus Error at the ACIA address `0xFFFC00`). Since then SDK07 patched it (overlay `sdk/overlays/teradesk/video.c:24-44@SDK07`,
  `newtos_mch_nohard()`), so the ST branch is safe either way. TeraDesk also did not find a later-set `_VDO` before
  SDK04 C6 (16-byte step); that is fixed (RUNINFO "INPUTS", TeraDesk row).
- With `_VDO = 0x00030000` injected (RESULT 3): `VsetMode(-1)` -> `0x58`, `VgetMonitor` -> `0x59`; TeraDesk read
  them as `falmode = 0x58` and `fmtype = 0x59` (garbage): Overscan ticked, "Doubleline" enabled, and OK without a
  change raised the alert "A change of video mode will terminate all running applications" and an AES restart.
  **That is what changes with the answers decided in 2026-11-12**: with `0x1C` and 2, `falmode & VM_STMODE == 0`,
  `VM_80COL` set -> `currez = TT_MEDRES` (`video.c:202-209@VM_STMODE`), `fmtype == VGA_MON`; the dialog's rebuilt word is
  `0x1C` again (64K colours -> `npc[16] = 4`, `:475,:620-621`; Overscan/Doubleline untouched), so
  `newmode == falmode` and `newrez == currez` and OK raises no alert (`:694`). Derived from the source, not
  measured; the tester repeats the 2026-10-29 Falcon-branch run.
- What still holds: choosing another mode in the dialog and confirming calls `shel_write(SHW_RESCHNG, mode, 1)`
  (`video.c:389-408@fal_mil`); NewTOS records the request and restarts the AES but never switches the mode
  (`tos-api/src/aes/mod.rs:101-125@shel_write`, `tos-api/src/aes/shell.rs:484-492@SHW_RESCHNG`), so TeraDesk offers modes it cannot
  honour and, after the restart, reads `0x1C` again. COMPATIBILITY entry for `VsetMode`/`Vfixmode` should say so.
- The 2026-10-29 owner decision "`_VDO` bleibt weg" (`TASK/ARCH06.md`, last section) is superseded by the owner's
  2026-11-12 "_VDO-Cookie ja"; the ARCH06 "design:hardware:videl" argument no longer holds for the cookie.

**Other in-repo programs.** Only TeraDesk reads `_VDO` (`sdk/overlays/teradesk/video.c:181@find_cookie`, upstream
`sdk/apps/teradesk/video.c:158`); the cookie probe `sdk/overlays/teradesk/tests/cookie-probe.c:69@_VDO` lists it.
No other file in `sdk/`, `programs/`, `kernel/`, `tos-api/`, `tosdump/` reads it (`/usr/bin/grep -rl`, 2026-11-18;
the gitignored local Applications tree is absent in this worktree). Gap closure: the programs on the standard image come from local sources
(not re-mapped in this run; no program was found that asks for `_VDO` outside the one above). EmuTOS's own AES and
desktop do not read `_VDO`; they call `VsetMode(-1)` directly (`aes/gemgsxif.c:184`, `desk/deskrez.c:159`).
Risk not measurable by reading: a future port that trusts `_VDO = Falcon` may then touch Videl registers
(`0xFFFF82xx`); none does today.

**Draft: ADR-012 amendment (not applied to `docs/MEMORY.md`)**

> *Amendment (date of landing, owner decision 2026-11-12, TASK/XBIOS22).* **`_VDO` is published** with the value
> `0x00030000` (`VDO_FALCON`, `references/emutos/include/cookie.h:67`; EmuTOS sets it when the Videl is present,
> `references/emutos/bios/machine.c:363-367`). Reason: NewTOS answers the Falcon video calls that describe the
> screen (`VsetMode(-1)`, `VgetMonitor`, `VgetSize`, `VgetRGB`, TASK/XBIOS22) and is the Falcon030/TOS4 profile
> (ADR-024); a program that gates those calls on `_VDO` (TeraDesk, `video.c`) must reach them. It claims a Falcon
> *video description*, not Falcon hardware: `VsetMode(mode != -1)`, `Vfixmode` and the other Videl calls stay
> unimplemented (ADR-017), `_SND`/`_FDC`/`_CPU`/`_FPU` stay absent, `_MCH` stays `0xFFFFFFFF`. This replaces the
> 2026-10-28 amendment's "`_VDO` is decided after measuring" and the 2026-10-29 "stays absent" decision. Copies of
> the old statement to change in the same commit: `tos-api/src/cookie.rs:216-222@seed_defaults` comment and the
> `seed_defaults` set, `kernel/src/selftest_cookie.rs` (count 18), `docs/api/bios/_cookie-jar.md:51,141,151`,
> `docs/COMPATIBILITY.md:1630,1632` (row 45: falcontc equal, tt still different), `docs/HARDWARE-PROFILE.md:424`,
> `docs/analysis/deps/mapping.tsv` row `setvalue_vdo`, `docs/sdk/08-porting-checklist.md:69` (`vdo >> 16` is 3).

### Is XBIOS23 then complete for one developer to build both tickets together?

Yes, with these items the developer takes from the files, not from the owner: (1) `getrez` -> 2 and the selftest
expecting 4 (`TASK/XBIOS23.md`); (2) rewrite of the "NewTOS behavior (settled — ADR-017 rev 2)" section of
`docs/api/xbios/04-getrez.md` and removal of its round-trip criterion (the coherence note is in that file);
(3) ADR-017 Revision 3 text from `TASK/XBIOS23.md` applied in the implementing commit, where it should also name
the `VsetMode(-1)` word once the owner fixes it; (4) the dispatcher arms for 0x58 (query only), 0x59, 0x5B, 0x5E
next to the catch-all (`tos-api/src/xbios/mod.rs:162@other`), tests per function (documented value, documented
error; the error arm is "no error path" for 0x59 and `VgetSize`, `-1` for `VgetRGB`, see the spec files).
Still open for XBIOS22 before code: the exact `VsetMode` word (a), B1/B2 (b), the `_VDO` value (c), and the
`VgetSize`-before-framebuffer value. Nothing is missing from XBIOS23 itself.

### Open points (spec step 2)

- STOP (owner): `VsetMode(-1)` word (`0x1C` recommended); `VgetSize` byte count (B1 recommended); `_VDO` value
  (`0x00030000` recommended) and the ADR-012 amendment text.
- STOP: `VgetSize` before the framebuffer is up (`Host::screen()` = `None`).
- Inferred: TeraDesk's behaviour with `0x1C`/2/`_VDO` = Falcon (read, not measured); needs the tester's
  Falcon-branch run (the 2026-10-29 `vid-vdo` scenario, `measurements/measurement-2026-10-29-arch06/method/`).
- Inferred: the oracle's `0x1C` came from NVRAM, not the built-in default (`bios/screen.c:491,529`).
- Out of scope: EmuTOS's AES puts `VsetMode(-1)` into `intout[45]` of `v_opnwk` (`aes/gemgsxif.c:184`); NewTOS's
  `v_opnwk` answer there is not checked.

## Owner decision 2026-11-18

Answers to the lead's recommendations on spec step 2, after the lead explained that `VsetMode(-1)` is a query
only (the configuration stays in `config.txt`) and that it goes together with `_VDO`.

1. `VsetMode(-1)` answers `0x001C` ("ah ok dann setze das so um": ah ok, then build it like that). Every
   `VsetMode(mode != -1)` keeps answering the function number.
2. `VgetSize`: the byte count of the 32-bpp `Physbase` screen, option B1 ("ja gibt ja die größe zurück": yes, it
   returns the size).
3. `_VDO`: "mach es so das es am besten passt, man sollte vorher natürlich messen wie teradesk es z.b. erwartet"
   (do it the way that fits best; of course one should measure first what e.g. TeraDesk expects). The lead reads
   this as: build the recommended `0x00030000` (`VDO_FALCON`), and the value counts as decided only after the
   tester measured TeraDesk's Falcon branch with it (the `vid-vdo` scenario of
   `measurements/measurement-2026-10-29-arch06/method/`, repeated on the developer's commit). If TeraDesk shows a
   wrong dialog state or an alert there, the branch does not land and the value comes back to the owner. The
   ADR-012 amendment text of spec step 2 is applied in the implementing commit.
4. `VgetSize` while no framebuffer is up (`Host::screen()` is `None`): the function number `0x5B`, EmuTOS's own
   "no Videl" answer (lead's recommendation, covered by the owner's "setze das so um"; not answered separately).

## Owner decision 2026-11-18, after the tester's TeraDesk measurement

Run `measurements/measurement-2026-11-18-xbios22-test/RUNINFO.TXT` (section "TERADESK GATE"). With `_VDO`
TeraDesk takes its Falcon branch: the video dialog shows VGA-80-column, Overscan greyed, 64K colours. OK on the
unchanged dialog raises TeraDesk's alert "A change of video mode will terminate all running applications";
OK there restarts the AES, TeraDesk comes back and reads `0x001C` again; no crash. `main` without `_VDO` showed
the ST/TT dialog (TT-Medium set), no alert, no restart.

Owner: "ok dann gehen wir den falcon weg" (ok, then we take the Falcon way). `_VDO` = `0x00030000` stands; the
branch landed as `f6bdafa47`.

Open from this, not part of this ticket:

- Owner's question: can the dialog show the right data (the Pi's real mode)? The Falcon mode word cannot name a
  mode beyond its own set; to be answered by research before anything is built.
- Not established: on the commit the first Return on the unchanged dialog left *Colours* at 256 and the alert
  came with the second Return; with the arch06 injection on `main` it came with the first.

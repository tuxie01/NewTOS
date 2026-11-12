# Audit SND04 — XBIOS DMA sound group 0x80-0x8D against the oracle (O1-O5)

Written in English (CLAUDE.md, owner decision 2026-11-10: everything in the repository is English; it ranks above
the agent file's "German prose").

## 1. Head

| | |
|---|---|
| Scope | `TASK/SND04.md`: the 14 XBIOS functions 0x80-0x8D (`Locksnd` … `Buffptr`), cases O1-O5 of `TASK/SND01.md` "Spec review" §4, semantics S1-S7 of `docs/api/xbios/_dmasound-group.md` §9.1.2 |
| Date | 2026-11-12 |
| NewTOS measured | `ef37dab68e0b5e51f5288568045a3c0e14b10e4a`; working tree dirty only with this audit's files (no `kernel/`, `tos-api/`, `tos-types/` change); kernel built in a detached worktree of that commit |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df`, ROM sha256 `323c87bb…2d2750c`, `make 512 COUNTRY=de` |
| Hatari | v2.6.1, `tools/oracle/hatari.cfg`; profiles `falcontc` (Falcon branch) and `tt` (STe/TT branch) |
| Kernel | `kernel8.img` sha256 `764f174ef7e02f2a6fc795aceee634feee543e3f018a38478b6ca5aa2d7621da` (rpi4, release, `bios-arg-trace`) |
| Board | QEMU `raspi4b`, `serial=on` |
| Canary | `canary tt: OK`, `canary falcontc: OK` (`docs/analysis/ORACLE.md` §6), at `767688323`; the commit between it and the runs (`ef37dab68`) is docs-only |
| Run | `measurements/measurement-2026-11-12-snd04/RUNINFO.TXT` |

**The oracle here is EmuTOS driving Hatari 2.6.1's DMA sound model, not silicon.** Hatari has two models (the
Falcon crossbar on `falcontc`, STe/TT DMA sound on `tt`), and they disagree on several timing points (§2,
SND04-003). Hatari runs with `--sound off` (pinned profile): the model runs, only host audio is off.

Instrument: a new permanent probe `tosdump/probes/snprobe.c` (with `tosdump/probes/run-snprobe.sh`,
`tosdump/build-snprobe-newtos.sh`, Makefile targets `snprobe-m68k`/`snprobe-newtos`, and one accepted-probe line in
`tools/hatari-oracle.sh`). Why a probe and not a table: DUMPBIOS' guard admits only the four inquiries of this group
(`tosdump/core/cases.c`, `call_policy`), and O2/O3/O5 are timing.

## 2. Findings

### SND04-001 — S3 — CONFIRMED — the whole group answers its function number where the oracle runs the real code

* **Localization:** LOCALIZED: `tos-api/src/xbios/mod.rs:157@other` (the dispatcher's catch-all arm; NewTOS has no
  function for any of the 14). `cargo xtask deps buffoper --down` is empty (a leaf in the `bios` layer), so nothing
  below needs to pass first. Observed via: `bios-r1.tsv` cases 230-235 and the probe's part A.
* **Cases + evidence:** `tosdump/cases/bios-r1.tsv` cases 230-235, tosdiff
  `measurements/measurement-2026-11-12-snd04/newtos/bios-r1/tosdiff-EMUTOS-1a7484ab-BIOS-falcontc.txt` and
  `…-BIOS.txt`: `Soundcmd(0|1|6,-1)` oracle `0`/`0`/`3` (falcontc), `0` (tt), NewTOS `0x82`; `Buffoper(-1)` `0`
  against `0x88`; `Sndstatus(0)` `0` against `0x8C`; `Buffptr` `0` with the buffer written, against `0x8D` with
  the buffer untouched. Probe: `measurements/measurement-2026-11-12-snd04/newtos/snprobe/SNPROBE.LOG` (41 calls,
  every one answers 0x80…0x8D) against `measurements/measurement-2026-11-12-snd04/oracle/falcontc-run1/SNPROBE.LOG` and `measurements/measurement-2026-11-12-snd04/oracle/tt-run1/SNPROBE.LOG`
  (`Locksnd` `1`, `Devconnect` `1`/`0`, `EBADRQ` arms, …).
* **Input check:** NewTOS `biostrace-check.py` PASS (147 calls, 0 rows differ) for the table; the probe's 41
  `[biostrace]` lines in `measurements/measurement-2026-11-12-snd04/newtos/snprobe/BOOT-SERIAL.txt` show every argument as passed (checked by hand). Oracle:
  Hatari `--trace xbios`, `oracle/xbios-trace/*-xbios-calls.txt` — every argument as passed (Hatari prints
  `Xbtimer` without arguments).
* **EmuTOS:** `references/emutos/bios/dmasound.c:215-803` (all 14 run because `has_dmasound` is set on both
  profiles, `:122-126`). **NewTOS:** `tos-api/src/xbios/mod.rs:157@other`.
* **Spec status:** `docs/api/xbios/_dmasound-group.md` §1.4 calls today's answer "conformant" — true only against
  EmuTOS on a machine without the chip, which neither oracle profile is. No `docs/COMPATIBILITY.md` entry covers
  the group (the XBIOS "absent hardware" entry names NVRAM, TT shifter, DSP only), so the DIFF is not ACCEPTED.
  The owner has decided to build sound (`TASK/SND01.md` decisions 2026-11-12, `TASK/SND05.md`).
* **Why S3, not S2:** `_SND` is absent on NewTOS (`docs/COMPATIBILITY.md` row 49, ACCEPTED), so a conforming sound
  program finds no sound and never calls the group; one that calls anyway gets EmuTOS' own "no chip" signal. The
  wrong result is reachable only off the normal path.
* **Not a stub:** the catch-all arm is the dispatcher's documented answer for every unwired id, not a hardcoded
  success.
* **Retest list:** `cargo xtask deps buffoper --up` = `xbiostrap` (the table edge `bios/xbios.c:1508`); cases
  `bios-r1.tsv` 230-235 and the probe (all parts). Same for the other 13 (one table slot each,
  `bios/xbios.c:1500-1513`).
* **Route:** Developer (`TASK/SND05.md` builds the group); Researcher (either a COMPATIBILITY entry for the time
  without a backend, or none, the DIFF staying open until SND05).

### SND04-002 — S4 — MEASURED — spec: `Setinterrupt` mode 1 ("MFP-i7") named the wrong MFP channel and vector

* **Localization:** spec only (`docs/api/xbios/_dmasound-group.md` §4.4 and §9.1.2 row S6). No NewTOS code yet.
* **Evidence:** probe part E, `measurements/measurement-2026-11-12-snd04/oracle/falcontc-run1/SNPROBE.LOG` and `measurements/measurement-2026-11-12-snd04/oracle/tt-run1/SNPROBE.LOG` lines E12-E19: with
  `Setinterrupt(1,1)` and counting handlers installed by `Mfpint(15, h1)` and `Mfpint(7, h2)`, the channel-15 handler
  ran once per single-shot frame, the channel-7 handler never, on both profiles; repeated in run 2 and the trace run.
* **EmuTOS:** `include/biosdefs.h:195` (`MFP_MONODETECT 15` = GPIP7), `:185` (`MFP_DISK 7` = GPIP5),
  `bios/dmasound.c:477-491`. The spec said "MFP i7 is interrupt 7, vector `0x11C`"; measured: channel 15, vector
  `0x13C`.
* **Spec status:** corrected in the working tree by this audit, marked "(corrected 2026-11-12, measured)" — the
  lead asked for the measured results to go into the spec; the Researcher reviews.
* **Failure scenario:** a NewTOS backend built from the old S6 row would deliver mode-1 frame ends to a handler a
  program installed at `0x13C` never, and would count `Mfpint(7, …)` (the disk interrupt) as the sound vector.
* **Retest list:** probe part E; SND05's own case for `Setinterrupt` mode 1.
* **Route:** Researcher.

### SND04-003 — S4 — MEASURED — spec: S2 "once at the end of each play frame" does not hold for the Falcon model

* **Localization:** spec only (§9.1.2 row S2, §4.4).
* **Evidence:** part E lines E06/E09/E11. `falcontc`: timer A once at tick 0 of a single-shot frame (its start), no
  interrupt at its end; in loop mode at ticks 0, 16, 33, 49 (start of every pass, incl. the first); none with
  `Setinterrupt(0,0)`. `tt`: once at tick 16 (the end) for a single-shot frame; in loop mode at 17, 33, 49 (end of
  each pass) and 58 (when `Buffoper(0)` stopped a frame midway); `Setinterrupt(0,0)` is `EBADRQ` there and the
  interrupt still comes (S1, "always enabled", `bios/dmasound.c:467-475`).
* **Spec status:** row S2 rewritten as "measured, the two Hatari models disagree on the moment"; §9.1.6 gives the
  table and names the `falcontc` column as the target for a NewTOS that reports `_SND` `0x14`.
* **Open:** which of the two is silicon. No Falcon or STe on the bench; Hatari's sources are not in `references/`.
* **Route:** Researcher (decide the target model with the owner, Q-N2/Q-N5).

### SND04-004 — S4 — MEASURED — spec inference "`Sndstatus` after a reset answers 3 (`SS_SCLK`)" not reproduced

* **Evidence:** part F, `falcontc`: `Sndstatus(1)` `0`, `Sndstatus(0)` after it `0`. Hatari keeps `codec_status` at
  0 and does not model the codec toggle (ADDERIN set to `1` before the reset still reads `1`, though
  `bios/dmasound.c:735-750` says the toggle zeroes it).
* **Spec status:** §3/0x8C now says "not reproduced by the oracle; the inference stays unmeasured".
* **Route:** Researcher.

### SND04-005 — S4 — PLAUSIBLE — mapping rows say "no counterpart by design" for a group the owner decided to build

* `docs/analysis/deps/mapping.tsv` rows for `buffoper` … `sndstatus` (`bios/dmasound.c`, 20 rows) carry
  `design:hardware:dma-sound`, "not implemented in NewTOS". Since the owner decisions of 2026-11-12 (`TASK/SND01.md`)
  that is no longer a design choice but a gap; the `--up` retest sets therefore report "complete" while the functions
  are unbuilt. Settled by the Researcher re-classifying the rows (or by SND05 mapping them). Route: Researcher.

### Source conflict `Sndstatus(1)` — resolved on the oracle (not a finding)

`Sndstatus(1)` during a looping frame with `Setinterrupt(0,1)` (part F, `falcontc`): `$FFFF8900/01` `0x04 0x03`
before and after, `Buffoper(-1)` `3`, the pointer keeps advancing (2077 → 3539 bytes in 3 ticks), the attenuation
set to `0x50` before reads `0` after (EmuTOS' own write, `bios/dmasound.c:761`). FreeMiNT's "interrupts disabled,
buffer operation disabled" is not what the oracle does. Recorded in spec §3/0x8C with the caveat that Hatari models
no codec toggle at all, so silicon stays open.

## 3. Open tickets in scope

| Ticket | Result | Case |
|---|---|---|
| `TASK/SND04.md` (this measurement) | O1-O5 recorded on the oracle (both profiles); S3/S4 and `Sndstatus(1)` in the spec; doc-check 0 failures. Acceptance text says "committed references": for a probe the references are the oracle LOGs of the run (`measurements/measurement-2026-11-12-snd04/oracle/`), committed when the lead commits the run. S7 not measured | `tosdump/probes/snprobe.c` |
| `TASK/SND05.md` (backend) | DIFF (not built) | SND04-001 |
| `TASK/SND01.md` (umbrella) | DIFF, same | SND04-001 |

## 4. Coverage matrix

| EmuTOS | NewTOS (mapping) | Layer | Status | Cases |
|---|---|---|---|---|
| `locksnd` 0x80 | none (catch-all `other` arm) | bios | DIFF (SND04-001) | probe A01, A02 |
| `unlocksnd` 0x81 | none | bios | DIFF | A03, A04 |
| `soundcmd` 0x82 | none | bios | DIFF | bios-r1 230-232; A05-A12, A34-A39, F08-F14 |
| `setbuffer` 0x83 | none | bios | DIFF | A33, B03, C01, D01, D03 |
| `setsndmode` 0x84 | none | bios | DIFF | A17, A18, B01 |
| `settracks` 0x85 | none | bios | DIFF | A19, A20 |
| `setmontracks` 0x86 | none | bios | DIFF | A21, A22 |
| `setinterrupt` 0x87 | none | bios | DIFF | A23-A27, E02, E10, E12, E16 |
| `buffoper` 0x88 | none | bios | DIFF | bios-r1 233; A13, A14, A40, B05-B09, C02-C07 |
| `dsptristate` 0x89 | none | bios | DIFF | A28 |
| `gpio` 0x8A | none | bios | DIFF | A29, A30 |
| `devconnect` 0x8B | none | bios | DIFF | A31, A32, F18 |
| `sndstatus` 0x8C | none | bios | DIFF | bios-r1 234; A15, F04, F15, F20 |
| `buffptr` 0x8D | none | bios | DIFF | bios-r1 235; A16, B04, B08, polls |

equal 0, accepted 0, diff 14, unmeasured 0 of 14. Bug density: 1 confirmed finding / 14 functions measured (the
one finding covers all 14 DIFFs).

Arms measured on the oracle only partly (they do not change the NewTOS verdict, every call answers its number):
record (`Setbuffer(1,…)`, `Buffoper(4|8)`), `Gpio(0|2,…)`, `Dsptristate(1,…)`, `Devconnect` with other
sources/destinations/clocks, `Setmode(1|2)`, `Settracks`/`Setmontracks` non-zero, `Soundcmd(1..5, value)`.

## 5. New cases

* `tosdump/probes/snprobe.c` — parts A (O1), B (O2/O3, S3/S4), C (loop), D (S5), E (O5, S2/S6), F (O4); runs on
  `tt` and `falcontc`, and on NewTOS (part A only while the group is absent).
* Proposed table rows for when DUMPBIOS' guard admits the group (a tosdump change, route Researcher/Developer; not
  made here):

```
14:0x80	-	EXACT	-	Locksnd() -- bios/dmasound.c:215
14:0x80	-	EXACT	-	Locksnd() again: SNDLOCKED -- bios/dmasound.c:220
14:0x81	-	EXACT	-	Unlocksnd() -- bios/dmasound.c:230
14:0x81	-	EXACT	-	Unlocksnd() again: SNDNOTLOCK -- bios/dmasound.c:235
14:0x82	w:7,w:-1	EXACT	-	Soundcmd(7,-1) unknown mode -- bios/dmasound.c:302
14:0x84	w:3	EXACT	-	Setmode(3) EBADRQ -- bios/dmasound.c:408
14:0x85	w:4,w:0	EXACT	-	Settracks(4,0) EBADRQ -- bios/dmasound.c:424
14:0x86	w:4	EXACT	-	Setmontracks(4) EBADRQ -- bios/dmasound.c:441
14:0x87	w:2,w:0	EXACT	-	Setinterrupt(2,0) EBADRQ -- bios/dmasound.c:464
14:0x8A	w:3,w:0	EXACT	-	Gpio(3,0) EBADRQ -- bios/dmasound.c:563
14:0x8A	w:1,w:0	EXACT	-	Gpio(1,0) read pins -- bios/dmasound.c:559
14:0x8B	w:4,w:8,w:0,w:0,w:1	EXACT	-	Devconnect invalid src -- bios/dmasound.c:633
14:0x83	w:2,l:0,l:0	EXACT	-	Setbuffer(2,..) EBADRQ -- bios/dmasound.c:374
14:0x88	w:-2	EXACT	-	Buffoper(-2) any negative is the inquiry -- bios/dmasound.c:504
```

## 6. Measured vs open

Measured: O1 (every function's return in the arms above, boot state), O2/S3, O3/S4 (range, granularity, end,
loop, stop, before start, `res1`/`res2`/`record`, torn reads), S5, O4, O5/S2/S6 — on both oracle profiles, two runs
each plus a trace run; NewTOS for all 14 functions with input traces.

Open: S7 (time budget; inference); which Hatari model matches silicon (SND04-003); the codec-toggle effects of
`Sndstatus(1)` on silicon; the record path; the oracle side of the probe has no tosdiff (verdict MEASURED, by
design for timing). The `falcontc` runs are not byte-identical (sub-tick timing), the `tt` runs are.

## 7. Real-Pi4 list

None for this scope: NewTOS has no sound backend yet. The hardware facts for the backend are `TASK/SND03.md`
(H1-H5).

## 8. Acceptable deviations

None claimed. Torn `Buffptr` reads (§3/0x8D (e)) are a property of the oracle's register read, not contract
(`docs/api/xbios/_dmasound-group.md` §8 puts the register image under internals); a NewTOS `Buffptr` that never
tears is not a deviation a program can rely on the other way.

## 9. Out-of-scope observations

* `docs/hardware/pwm-audio.md` §9 still maps `Setbuffer` straight onto `SOURCE_AD` (already recorded by the spec
  review, not chased).
* The probe's text runs in TT-RAM on the oracle (PC `0x0102…`): `tosdump/build/*.PRG` load into TT-RAM by their
  header flags; DMA buffers must come from `Mxalloc(…, 0)` (done here).

## 10. Cleanup

* `AUDIT-TEMP`: none written; `git grep AUDIT-TEMP` outside `docs/`/`.claude/` finds only old evidence under
  `measurements/measurement-2026-10-29-xbios15/` (not mine).
* Touched files, permanent additions for the lead to commit (not temporary wiring): `tosdump/Makefile` (snprobe
  targets and usage lines), `tools/hatari-oracle.sh` (SNPROBE.PRG accepted, header line), new
  `tosdump/probes/snprobe.c`, `tosdump/probes/run-snprobe.sh`, `tosdump/build-snprobe-newtos.sh`,
  `docs/api/xbios/_dmasound-group.md` (measured results, §9.1.6), the run directory, this report. Their diffs
  against the pristine copies are additions only.
* No `kernel/`, `tos-api/`, `TASK/SND03.md` change. Foreign change seen: commit `ef37dab68` (docs, SND01) landed
  during the audit; nothing foreign in the working tree.
* Worktree `target-agents/agent-af3707a99691bdbb0/wt` (1.7G) removed after the run; its kernel's sha256 is in
  RUNINFO.

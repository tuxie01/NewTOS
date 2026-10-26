# `run`, memory ownership and current drive/directory per PD (TASK/PROC03, PROC04 point 2)

> Spec extraction, cross-cutting `Pexec` / `Pterm` / `Malloc` / `Dsetpath`.
> Source: EmuTOS @ `1a7484ab` (`references/emutos`), read 2026-10-18.
> User decision 2026-10-18 (`TASK/PROC03.md`, "Anwenderentscheidung"): **every
> open question is answered "as EmuTOS"**. This file is the EmuTOS fact base for
> that decision; the NewTOS target and the points the Developer must decide
> are in §6. The chain/field half of the same package is
> [`_parent-and-environment.md`](./_parent-and-environment.md).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Labels used below: **(read)** = stated by code at the cited `file:line`;
> **(inferred)** = reasoned from cited code, not read as one statement — the
> Developer must treat it as less load-bearing; **[C]** = contract (a program
> can observe it), **[R]** = reference (EmuTOS internal, NewTOS free).

## 0. Gating

Everything here is in `bdos/` and `aes/` and not `#if`-gated, except:
`CONF_WITH_ALT_RAM` (the `pmdalt` branches, `bdos/proc.c:137-139,629-632`,
`bdos/umem.c:104-107`; `1` in the `make 512` profile,
`docs/api/_emutos-config.md`), `WITH_CLI` (EmuCON, link-time:
`aes/gemshlib.c:614-622`, `bios/bios.c:834-843`; `1` for `make 512`),
`DETECT_NATIVE_FEATURES` (`PE_RELOCATE`, `bdos/proc.c:193-209`; not relevant).

## 1. `run` — every write, and its value over time

### 1.1 The write sites (read)

`run` (`PD *run`, `bdos/proc.c:55`) is assigned in **exactly three places**.
A tree-wide search for assignments and for the asm symbol (`_run`) over
`bdos/ aes/ bios/ util/ cli/ desk/` finds no other (`bdos/rwa.S:188,235,266`
only *read* `_run`):

| Site | Statement | When |
|---|---|---|
| `bdos/bdosmain.c:339` | `run = &initial_basepage;` | `osinit_after_xmaddalt`, GEMDOS init |
| `bdos/proc.c:575` | `run = (PD *)p;` | end of `proc_go`, i.e. `Pexec` modes 0, 4, 6 only (`:358-359`, `:244-248`) |
| `bdos/proc.c:609` | `run = run->p_parent;` | `xterm`, i.e. `Pterm`, `Pterm0`, `Ptermres`, trap #2 `d0.w==0`, Ctrl-C |

`Pexec` modes 3 (`PE_LOAD`), 5 (`PE_BASEPAGE`), 7 (`PE_BASEPAGEFLAGS`) never
change `run` (`bdos/proc.c:210-237`, `:358`).

`*os_run` **is** `run`: the OSHEADER word `os_run` holds the address of the C
variable (`bios/startup.S:110-111`, `.dc.l _run`; `include/biosdefs.h:60`
"pointer to the pointer to current BASEPAGE"). [C] A program reading
`**(PD ***)(sysbase+0x28)` sees `run`, nothing else. The AES keeps its own
notion of the current process (`rlr`, an `AESPD`) and **never touches `run`**
(no `run` token in `aes/`; the cooperative dispatcher `dsptch`/`disp`
(`aes/gemdisp.c`, `aes/gemasm.S:122-`) switches AES contexts only).

### 1.2 `run` over time (read, per start path)

| Moment | `run` | Source |
|---|---|---|
| GEMDOS init, BIOS `Dsetdrv`, `autoexec` `Fsfirst` | `initial_basepage` | `bdos/bdosmain.c:339`; `bios/bios.c:827,713` |
| AUTO program *n* running | its PD (`p_parent` = `initial_basepage`) | `bios/bios.c:693`, `bdos/proc.c:541,575` |
| after an AUTO program's `Pterm`/`Ptermres` | `initial_basepage` | `bdos/proc.c:609` |
| `exec_os`: `Pexec(PE_BASEPAGEFLAGS)` | `initial_basepage` (creation) — owner of the AES block | `bios/bios.c:861-865`, `bdos/proc.c:229` |
| AES PD running: `ui_start`, `gem_main` up to `:894` (reads `EMUDESK.INF`, `count_accs`, `dos_alloc_anyram`, `dos_sdrv(bootdev)` on rez change) | **AES PD** | `aes/gemstart.S:46-119`, `aes/geminit.c:753-894,796-801,810-814` |
| `aes_run_rom_program(accdesk_start)` | creates with `run` = AES PD; `PE_GOTHENFREE` → `run` = **accdesk PD** | `aes/geminit.c:894`, `aes/gemshlib.c:570-581`, `bdos/proc.c:238-248,575` |
| `run_accs_and_desktop` up to and including `sh_main`'s own AES/GEMDOS work: `load_accs`, first `dsptch()`/`wait_for_accs(AP_MESAG)`, `sh_ldapp` housekeeping between programs, `free_accs` | **accdesk PD** | `aes/geminit.c:654-745`, `aes/gemshlib.c:592-690` |
| the ROM desktop (`deskstart`) running | desktop PD (`p_parent` = accdesk) | `aes/gemshlib.c:600-611` |
| EmuCON (`coma_start`) running | EmuCON PD (`p_parent` = accdesk) | `aes/gemshlib.c:614-620`, `cli/cmdasm.S:22-49` |
| a program the desktop/`#Z` started | its PD (`PE_LOADGO`; `p_parent` = accdesk) | `aes/gemshlib.c:645` |
| a program started by `Pexec` from program *X* | the child; after its end `X` | `bdos/proc.c:541,575,609` |
| accessory executing (any moment) | **unchanged** — whatever the line above says | `aes/gemasm.S:83-110` (`gotopgm` is a plain `jmp` to `p_tbase`) |
| AES executing on behalf of an AES call | **unchanged** — the caller's PD (the program, or accdesk between programs) | (read) no write site; (inferred) from `dos_exec` comment `aes/gemstart.S:178-211` ("the AES context will not be switched when entering the child program") |
| `accdesk_start` returns (shutdown / `gl_changerez`) | `Pterm0` → AES PD | `aes/gemstart.S:160-163`, `bdos/proc.c:609` |
| AES ends | `Pterm0` → `initial_basepage` → back in `biosmain` after `Pexec(PE_GO)` | `aes/gemstart.S:126-134`, `bios/bios.c:865-866` |

Variants: `COMMAND.PRG` instead of the AES (`cmdload != 0`,
`bios/bios.c:850-855`: `Pexec(PE_LOADGO)` from `initial_basepage`); the early
console (`BOOTFLAG_EARLY_CLI`, `bios/bios.c:834-843`: a `PE_GOTHENFREE` PD with
`p_tbase = coma_start`, `p_parent` = `initial_basepage`, before `autoexec`).
Neither is the default boot of the target profile.

**`run` is a stack, not "the most recently started process".** `xterm` puts
`run` back to `p_parent`; so after a child ends `run` is its parent again, and
while an accessory executes `run` is the innermost *program* (the desktop,
EmuCON, a program, or accdesk between programs).

### 1.3 What it means for an accessory (read + inferred)

An accessory is **not a GEMDOS process**: it is `PE_LOAD`ed (never `proc_go`ed,
so `p_parent == 0`) and entered by a `jmp` from an AES context
(`aes/gemasm.S:83-110`). Every GEMDOS call it makes executes with
`run` = the PD of whatever program is in front. GEMDOS reads/writes only
`run`'s PD, so (each row: the line that proves it):

| What the accessory's call touches | In EmuTOS it is… | Source |
|---|---|---|
| `Malloc`/`Mxalloc` owner tag | `run` | `bdos/iumem.c:134` |
| `Malloc`'s pool choice (`PF_TTRAMMEM`) | `run->p_flags` | `bdos/umem.c:120` |
| `Fopen`/`Fcreate`/`Fdup` file owner | `run` (closed at *its* `Pterm`) | `bdos/fsopnclo.c:348`, `bdos/fshand.c:107`, `bdos/proc.c:121-124` |
| `Fdelete` of an open file ("only the owner may") | compares `f_own == run` | `bdos/fsopnclo.c:665` |
| `Cconout`, `Cconin`, `Cauxout`, `Cprnout`, `Fread/Fwrite` on handles 0-5 (redirection) | `run->p_uft[]` | `bdos/console.c:161-535`, `bdos/bdosmain.c:500,582`, `bdos/fsmain.c:462` |
| `Fsetdta`/`Fgetdta`, `Fsfirst`/`Fsnext` DTA | `run->p_xdta` | `bdos/fsmain.c:396,405`, `bdos/fsdir.c:528,659,673` |
| `Dsetdrv`/`Dgetdrv`/`Dsetpath`/`Dgetpath`, relative paths | `run->p_curdrv`, `run->p_curdir[]` | `bdos/fsmain.c:423,440`, `bdos/fsdir.c:1124-1145,1235-1243,1712-1731` |
| `Pexec` child's `p_parent`, env (`NULL` → copy), std handles, cwd | `run` (not the accessory) | `bdos/proc.c:419-420,391-409,541` |
| `Pterm`/`Pterm0`/`Ptermres` by an accessory | terminates **`run`** (the front program or accdesk), resumes `run->p_parent` | `bdos/proc.c:604-617` |

This is how EmuTOS behaves; none of it is guarded. **[C]** for every row a
program can observe it (for instance: an accessory's `Dsetpath` moves the front
program's directory; an accessory's `Fsetdta` replaces the front program's DTA;
a file an accessory opens while program P is in front is closed by P's `Pterm`).
Whether real accessories rely on any row other than the `Malloc` ownership row
is **not established** (not searched in this pass) — the Developer needs the
decision in §6 D-9.

EmuTOS contains one explicit mitigation for the `Malloc` row, both in the AES
and documented there: `wait_for_accs(AP_MESAG)` runs all accessories until each
has issued its first `evnt_mesag`/`evnt_multi` *before* `sh_main` starts the
desktop, "causing any memory allocated during initialisation (by e.g.
`v_opnvwk()`) to be owned by the AES, and thus the memory will not be freed by
termination of the desktop/autorun program" (`aes/geminit.c:569-598,736-739`;
the flag is set at `aes/gemsuper.c:137,148`; timeout 500 loops,
`aes/geminit.c:102,609`). The mirror mitigation is `ap_exit`:
`wait_for_accs(AP_ACCLOSE)` blocks until every accessory has *received*
`AC_CLOSE` "before the application exits, freeing its memory which includes any
memory allocated by the DAs during the life of the application"
(`aes/gemaplib.c:249-261`, `:255`; comment `aes/geminit.c:588-598`).
"The AES" in that comment is the **accdesk PD** (`run` during that window,
§1.2): (read) `run` = accdesk from `aes/geminit.c:894` through `:739`.

## 2. `Pterm`'s return path and `p_parent`

### 2.1 The sequence (read)

`xterm(rc)` (`bdos/proc.c:601-618`):

1. `userterm = Setexc(0x102, -1)`; `protect_v(userterm)` — the `etv_term`
   handler runs **with `run` still the dying process** (`:606-607`).
2. `run = run->p_parent;` (`:609`) — **no validation** of the value.
3. `ixterm(p)` for the *dying* PD `p` (`:610`): §3.3 (blocks), §4.5 (dirs).
4. `run->p_dreg[0] = rc;` (`:616`) — into the **new** `run`'s PD, offset 0x68.
5. `termuser()` (`:617`; `bdos/rwa.S:262-267`): `move.l _run,a5; jra ggusr`:
   reloads from `run`'s PD the saved stack pointer `P_A7SAVE` (0x7C) and
   `d0/a3-a6` (`P_REGSAVE` 0x68, `include/asmpd.h:25-29`) and resumes there.

The save area being resumed is the one **written by the last trap #1 entered
while that PD was `run`** (`bdos/rwa.S:188-206,223`: `move.l _run,a6;
movem.l d0/a3-a5,P_REGSAVE(a6); … move.l a5,P_A7SAVE(a6)`). Hence the parent
the child returns into is "the execution context that last made a GEMDOS call
under `run == p_parent`" — normally the process that called `Pexec`, because
that is the call that is pending.

### 2.2 When `p_parent` is not the `Pexec` caller (read + inferred)

| Case | What happens | Label |
|---|---|---|
| `p_parent` unchanged | normal | read |
| An accessory (or the AES code under a program) calls `Pexec` | child's `p_parent` = `run` (the front program/accdesk), **not** the accessory (`bdos/proc.c:541`). The accessory's trap saved *its* context into `run`'s PD (`rwa.S:188-206`), so the child's end resumes the **accessory** — the return works because `run` is the PD that holds the save area | read (`:541`, `rwa.S`), inferred (the conclusion) |
| `p_parent` of a started process overwritten by the process itself (or by anyone with write access to the PD) to a PD `Q` that is currently suspended inside a GEMDOS call | `Pterm` sets `run = Q`, stores `rc` in `Q->p_dreg[0]`, resumes `Q`'s saved context as if *Q's* pending call returned `rc`. Everything between (the real caller `R`, its blocks, files) is **not** cleaned up; `R` is never resumed | inferred from `bdos/proc.c:609,616` + `rwa.S:262-267`; **no EmuTOS code or doc was found that does this on purpose** |
| `p_parent == 0` (`Pterm` while `run` is `initial_basepage`, or a PD whose field was zeroed; an accessory's own PD is never `run`, §1.3) | `run = NULL`; `ixterm(p)` runs; `run->p_dreg[0] = rc` writes through NULL, `termuser` reads `0x7C`; unguarded → crash/undefined | read (`:609,616`); documented in `4c-pterm.md` |
| `p_parent` names something that is not a resumable PD | undefined (jumps through whatever `P_A7SAVE` holds) | inferred |
| A loader sets `p_parent` of a `PE_BASEPAGE`/`PE_LOAD` PD before `PE_GO` | **overwritten**: `proc_go` always writes `p->p_parent = run` (`:541`) | read |

So the *only* ways to change a started process's `p_parent` are code running in
that process (or code holding its PD pointer while it is suspended). [C] The
observable contract is: *after `Pterm` the process named by `p_parent` at that
moment resumes, with `rc & 0xFFFF` zero-extended as its `Pexec` result*
(exit-code details: `4c-pterm.md` "Exit-code matrix"). The SDK has no program
that writes `p_parent` (`TASK/PROC03.md` §4: mintlib `getppid`, `tfork`,
`heap_init`/`crt0` only *read* it).

### 2.3 `Ptermres` (read)

`xtermres(blkln, rc)` (`bdos/proc.c:624-634`): `xsetblk(0, run, blkln)` (result
ignored) — shrink `run`'s own block (the PD address) to `blkln`; then
`reserve_blocks(run, &pmd)` (+`pmdalt`) unlinks **every MD with `m_own == run`**
from the allocated list and `xmfremd`s the descriptors, so those blocks stay
allocated forever (`:77-89`); then `xterm(rc)` (whose `free_all_owned` finds
nothing left). It includes the environment block when `m_own == run`
(`PE_LOADGO`), and every `Malloc` the process made. Same return path as §2.1.

## 3. Memory ownership

### 3.1 Who writes `m_own` (read)

`m_own` is written in two places only (`grep m_own bdos/`): `ffit`
(`bdos/iumem.c:134`, `q->m_own = run`, for **every** allocation made through
`ffit`, i.e. `Malloc`, `Mxalloc`, `alloc_tpa`, `alloc_env`) and `set_owner`
(`bdos/umem.c:587-601`, "change the memory owner based on the block address":
finds the MD on `mp_mal` whose `m_start == addr`). `xmaddalt` writes
`m_own = NULL` on a *new free* MD (`bdos/umem.c:501`); `freeit`/`shrinkit` never
write it (`bdos/iumem.c:144-263`), so the tail MD `shrinkit` creates keeps
whatever the arena handed out and a freed MD keeps its old tag — free-ness is
list membership, never `m_own == NULL`.

`set_owner` callers, all in `xexec` (`bdos/proc.c`):

| Pexec mode | TPA block owner | environment block owner | Source |
|---|---|---|---|
| `PE_BASEPAGE`/`PE_BASEPAGEFLAGS` (5/7) | `run` (the creator) | `run` | `:229-230` |
| `PE_LOAD` (3) | `run` (the loader) | `run` | `:305-307` (`owner = (flag == PE_LOADGO) ? p : run`) |
| `PE_LOADGO` (0) | the new PD `p` | `p` | `:305-307` |
| `PE_GO` (4) | unchanged (still the creator's) | unchanged | `:244-248` (no `set_owner`) |
| `PE_GOTHENFREE` (6) | `p` — the PD becomes its own owner | `p->p_env` → `p` | `:238-243` |

So a process's TPA and environment die with its `Pterm` only for modes 0 and 6;
for 3+4 and 5/7+4 they belong to (and die with) the *creator* — or are freed by
whoever calls `Mfree`. (`Pexec` 6 is what `aes_run_rom_program` uses,
`aes/gemshlib.c:578-580`.)

### 3.2 Per-block-class owner table (read)

| Block | Owner | When freed |
|---|---|---|
| `Malloc`/`Mxalloc` block | `run` at the call | `Mfree`/`Mshrink(0)`, or the `Pterm` of that owner, or never after `Ptermres` |
| AUTO program's TPA+env | the program | its `Pterm` (or reserved by `Ptermres`) |
| AES PD block (`bios/bios.c:861`) | `initial_basepage` (`PE_BASEPAGEFLAGS` with `run` = initial, `PE_GO` keeps it) | never in practice (`initial_basepage` does not terminate) |
| accdesk PD block + its env | the accdesk PD itself (`PE_GOTHENFREE`) | `Pterm0` of accdesk = shutdown/rez change |
| ROM desktop PD, EmuCON PD, + their envs | themselves (`PE_GOTHENFREE`) | their own `Pterm0` |
| program started by the desktop/`#Z` (TPA+env) | the program (`PE_LOADGO`) | its `Pterm` |
| accessory TPA + env (`pgmld`: `Pexec(PE_LOAD)` with `run` = accdesk) | **accdesk PD** (`:305-307`, `util/gemdos.c:34`) | `free_accs`' `dos_free(TPA)` at `aes/geminit.c:296-303` (TPA only), the rest and the env with accdesk's `Pterm0` |
| AES internal allocations (`dos_alloc_anyram`: cursor resource, `D.g_acc`, `rsrc_load`, …) | `run` at the call | `D.g_acc`: explicit `dos_free` at `aes/geminit.c:901-902`; rest: owner's `Pterm` |
| memory an accessory allocates during init (before the first `evnt_mesag`) | **accdesk PD** (`run` then, §1.2) | with accdesk |
| memory an accessory allocates later (e.g. at `AC_OPEN`) | `run` then: the desktop, EmuCON or the front program | **that owner's `Pterm`** — the accessory keeps a dangling pointer unless it frees at `AC_CLOSE` (see the `AP_ACCLOSE` wait above) |

(Inference, labelled: the last two rows follow from §1.2's `run` values and
`bdos/iumem.c:134`; the accdesk-as-owner claim for init-time allocations is
confirmed by the explicit comment at `aes/geminit.c:584-587` ("owned by the AES", `:585-587`).)

### 3.3 `Pterm` — which blocks are freed for whom (read)

`ixterm(r)` (`bdos/proc.c:110-141`), after `run = parent`:
1. `p_uft[0..5]` entries `> 0` → `xclose` (`:115-119`);
2. every `sft[i]` with `f_own == r` → `xclose(i+NUMSTD)` (`:121-124`);
3. every non-zero `p_curdir[i]` → `decr_curdir_usage` (`:126-132`) — §4.5;
4. `free_all_owned(r, &pmd)` and, with `CONF_WITH_ALT_RAM && has_alt_ram`,
   `free_all_owned(r, &pmdalt)` (`:136-140`): `freeit` every MD on `mp_mal`
   whose `m_own == r` (`:92-100`).

Consequences (read): the dying PD `r` frees **blocks tagged `r`**, not blocks
"allocated by r's code": that includes blocks an accessory or the AES allocated
while `r` was `run`, children `r` loaded with mode 3/5/7 and never freed, and
`r`'s own TPA/env when `r` was started with mode 0/6. It does **not** free a
child's TPA started by mode 3+4/5+4 *from `r`'s parent*, nor anything tagged
`initial_basepage`, accdesk, the AES PD.

### 3.4 `Mfree` / `Mshrink`: no ownership check (read — **ticket hypothesis refuted**)

`xmfree` (`bdos/umem.c:135-159`): `find_mpb(addr)` (ST-RAM range, else `pmdalt`
if present, else `EIMBA`), scan `mp_mal` for `m_start == addr`, `EIMBA` if
absent, else `freeit` — **no `run`, no `m_own` read**. `xsetblk`
(`bdos/umem.c:170-229`): same lookup, `n` dummy, rounding, `EGSBF` if growing,
`E_OK` if equal, `len == 0` → `freeit`, else `shrinkit`; again no `m_own`
test, and **neither changes `m_own`** (the kept MD keeps its tag, §3.1).
(`TASK/PROC03.md` listed "Mfree/Mshrink ownership checks" as something to
confirm; there is none. `docs/api/gemdos/memory/49-mfree.md` and
`4a-mshrink.md` already said so for EmuTOS; their *NewTOS* paragraphs describe
the opposite, a decision that is now in conflict — §6 D-8.)

[C] Observable: a program (or accessory) can free or shrink any block by
address, including blocks tagged for another PD and a running process's own
TPA; the next `Pterm` of the tagged owner still `freeit`s by `m_own` and, the
MD no longer being on `mp_mal`, skips it (`bdos/iumem.c:164-175`: an MD not on
the list is ignored with only a debug message). Double free: `EIMBA`.

### 3.5 Accessory TPA by `PE_LOAD` from accdesk (read)

`pgmld` (`util/gemdos.c:29-47`): `Pexec(PE_LOAD, pname, "", NULL)` — `run` =
accdesk (§1.2) so: `p_parent` 0; TPA/env owner accdesk; `alloc_tpa` gave the
**largest free block** and `p_hitpa = p + max` (`bdos/proc.c:220,292,374`);
`env == NULL` → copy of **accdesk's** `p_env` (`:419-420`; content equal to the
AES environment, `_parent-and-environment.md` §3); `p_curdrv`/`p_curdir`/
`p_uft` copied from accdesk at *creation time* (`init_pd_files`,
`:350,387-410`). Then `Mshrink(TPA, tlen+dlen+blen+0x100)` (`:40-44`;
`p_hitpa` is left alone, `Mshrink` never writes the basepage, `bdos/umem.c:
170-229`). The AES process is then created with `pstart(gotopgm, name, TPA)`
(`aes/geminit.c:255`, `aes/gempd.c:110-128`); `gotopgm` jumps to `p_tbase` with
`a0` = the basepage (`aes/gemasm.S:83-110`) — **no `proc_go`, so no `p_parent`,
no `run` change, no `PD`-saved-context set up** (`p_areg`/`p_dreg` stay 0).
Memory the accessory later takes: §3.2.

## 4. Current drive and directory per PD

### 4.1 Fields (read)

`PD.p_curdrv` (`UBYTE`, +0x37) and `PD.p_curdir[NUMCURDIR]` (`UBYTE[26]`, +0x40,
`NUMCURDIR = BLKDEVNUM = 26`, `include/bdosdefs.h:62,86-90`, `include/sysconf.h:43`;
`p_lddrv` +0x36 is declared and **never written** anywhere in EmuTOS (0)).
`p_curdir[d]` is an index into the global `dirtbl[NCURDIR=40]` (`bdos/fs.h:44`);
`0` = "none yet" (slot 0 is never allocated: loops start at 1,
`bdos/fsdir.c:1172,1177`). [C] `p_curdrv` as the byte at PD+0x37; [R] the
`p_curdir` bytes (dirtbl indices — no program can interpret them).

### 4.2 Inheritance (read)

`init_pd_files(p)` (`bdos/proc.c:387-410`), run for **every** PD `xexec`
creates — at creation (`:235` for modes 5/7, `:350` after the image is loaded
for 0/3), **not** at `proc_go` — copies from `run` **at that moment**:
`p_uft` (§ `_state.md`), then for `i < NUMCURDIR`: `p->p_curdir[i] =
run->p_curdir[i]` and `dirtbl[dn].use++` if non-zero (`:401-406`), then
`p->p_curdrv = run->p_curdrv` (`:409`). `init_pd_fields` zeroed everything first
(`:370`), so the *default* of a never-inheriting PD is drive A, no dirs.
(Inferred from `:235`/`:350`: a `PE_BASEPAGE` PD created by process A and
started later by process B inherited A's drive/dirs, not B's.)

`initial_basepage` starts as all-zero BSS, `p_curdrv = bootdev` via
`Dsetdrv(bootdev)` while it is `run` (`bios/bios.c:827`), and its
`p_curdir[bootdev]` is created by the first `ckdrv` on that drive (the
`autoexec` `Fsfirst`, `bios/bios.c:713-728`; `bdos/fsdrive.c:112-132`).

### 4.3 The calls (read)

* `Dgetdrv` (`bdos/fsmain.c:438-441`): `return run->p_curdrv`.
* `Dsetdrv` (`bdos/fsmain.c:420-426`): `if (0 <= drv < BLKDEVNUM) run->p_curdrv =
  drv;` then `return Drvmap()` — no `ckdrv`, no check that the drive exists.
* `Dsetpath` (`xchdir`, `bdos/fsdir.c:1113-1148`): wildcards → `EPTHNF`;
  `dlog` = drive of the path or `run->p_curdrv`; `olddir = run->p_curdir[dlog]`;
  `findit` (→ `EPTHNF`); `incr_curdir_usage(dnd)` (`:1164-1188`: reuse the slot
  whose `dnd` matches, else the first slot with `use == 0`; none → `EINTRN`,
  returned to the caller as `EPTHNF`); `run->p_curdir[dlog] = newdir`; then
  `decr_curdir_usage(olddir)`. It changes **only `p_curdir[dlog]`**, never
  `p_curdrv`; a failure leaves the PD untouched.
* `Dgetpath` (`xgetdir`, `bdos/fsdir.c:1229-1251`): `drv = (drv == 0) ?
  run->p_curdrv : drv-1`; `EDRIVE` (and `*buf = 0`) if the drive is not in
  `Drvmap()` or `ckdrv(drv, FALSE) < 0`; the path is built from
  `dirtbl[run->p_curdir[drv]].dnd`. `ckdrv` has already repaired a missing or
  stale slot to the drive root (`bdos/fsdrive.c:112-132`).
* Relative paths (`dcrack`, `bdos/fsdir.c:1695-1731`): drive = path's or
  `run->p_curdrv`; `[D:]\x` → root, else `dirtbl[run->p_curdir[d]].dnd`.
* `ckdrv` (`bdos/fsdrive.c:77-132`) is what creates `p_curdir[d]` lazily (root
  DND) for `run` whenever drive `d` is first used by that PD.

All of this is in the existing per-function specs
(`../directory/3b-dsetpath.md`, `47-dgetpath.md`, `../drive/0e-dsetdrv.md`,
`19-dgetdrv.md`) — those are correct for EmuTOS; what they lack is who `run`
is (§1) and the NewTOS state (§6).

### 4.4 Who changes whose directory in the AES shell (read)

* `shel_write(doex=1, …)` called by a **program**: `sh_curdrvdir(sh_apdir)`
  reads the *calling program's* current drive and directory (`run` = that
  program) (`aes/gemshlib.c:109-125,175`).
* The shell then runs in the **accdesk** context (`sh_ldapp`, `run` = accdesk):
  `sh_chdef` → `set_drvdir(path)` = `dos_sdrv(drive)` + `dos_chdir(path)`
  (`aes/gemshlib.c:526-554`) — this **writes accdesk's `p_curdrv`/`p_curdir[]`
  permanently**: `NORMAL_APP` ← `sh_apdir` (the starter's directory);
  `CONSOLE_APP` ← `sh_cdir` (`#if WITH_CLI`); `AUTORUN_APP`/`DESKTOP_APP` ←
  `sh_cdir`. The child created next (`PE_LOADGO`, `aes_run_rom_program`)
  inherits accdesk's drive/dirs at creation (§4.2). After the program ends
  accdesk keeps that directory.
* EmuCON's `cd`/`d:` (`Dsetpath`/`Dsetdrv`, `cli/cmdint.c:250-255,567`) modify
  the **EmuCON PD**; programs EmuCON starts inherit from it; the PD is
  freed when EmuCON ends, accdesk's directory is unchanged.
* Resolution change: `dos_sdrv(bootdev)` with `run` = AES PD
  (`aes/geminit.c:796-801`) resets the AES PD's `p_curdrv`; the next accdesk
  inherits it.
* The `.ACC`/INF scan at start (`count_accs`: `Dsetdta`, `dos_sfirst`,
  `aes/geminit.c:264-284`) runs with `run` = AES PD.

[C] The observable result: a program's initial drive/directory is whatever the
shell last set in accdesk (its own directory for programs started through
`shel_write` `NORMAL_APP`), **not** the desktop's; and a child's `Dsetpath` never
changes its parent's directory.

### 4.5 What happens to directory references at `Pterm` (read)

`ixterm` (`bdos/proc.c:126-132`): for every `p_curdir[i] != 0` of the dying PD:
`decr_curdir_usage(h)` (`bdos/fsdir.c:1194-1218`): ignore indices
outside `1..NCURDIR-1`; `use--`, clamp at 0; at `use == 0` the slot is freed
(`dnd = NULL`). The DND (cached directory node) itself is **not** freed; the
dying PD's own `p_curdir[]` bytes are not cleared. There are no other
"directory handles": `Fsfirst/Fsnext` state lives in the caller's DTA buffer,
not in a kernel table. `Ptermres` runs the same `ixterm` (`:633`), so a TSR
gives up its directory references too.

(Inferred, [R]) EmuTOS leaks `dirtbl` use counts for PDs that never `Pterm`:
a `PE_LOAD`ed accessory increments the slot usage of accdesk's directories at
creation (`:401-406`) and is never `ixterm`ed (§3.2: it ends only with the
whole accdesk). NewTOS need not copy a leak; it is invisible to programs
except through exhaustion of `NCURDIR = 40`.

On media change `freetree` (`bdos/bdosmain.c:356-380,456-467`) zeroes every
`dirtbl` slot pointing into the freed tree; the next `ckdrv` resets the PD's
`p_curdir[d]` to the drive root (`bdos/fsdrive.c:121-131`) — [C] after a disk
swap the process's directory on that drive reads as the root.

### 4.6 Accessories (read + inferred)

Because GEMDOS uses `run`'s PD (§1.3), an accessory has **no current
directory of its own**: the accessory PD's `p_curdrv`/`p_curdir[]` were copied
from accdesk at `PE_LOAD` time and are never used afterwards.

## 5. Contract vs. reference

| Item | Class |
|---|---|
| Value of `p_parent` for each start path; the chain depth (§ `_parent-and-environment.md`) | **[C]** (a program can walk it) |
| Which PD `Pterm` resumes (`p_parent` at `Pterm` time) and that `rc & 0xFFFF` is delivered | **[C]** |
| Behaviour when `p_parent` was altered (§2.2 rows 3-5) | **[C]** in the weak sense "unspecified"; EmuTOS' behaviour there is accidental. NewTOS needs a *safe* defined outcome, not EmuTOS' crash |
| `*os_run` == the PD in `run`, per §1.2 including "unchanged while an accessory executes" | **[C]** |
| `m_own` = `run` at allocation; blocks freed at the owner's `Pterm`; `Ptermres` keeps them | **[C]** (observable as "my memory survives/disappears") |
| `Mfree`/`Mshrink` without ownership check | **[C]** (a program can free another's block and succeed) |
| `m_own` bookkeeping details (stale tag after free, tail MD tag) | **[R]** |
| Accessory GEMDOS calls act on `run` (§1.3 table) | **[C]** (programs observe it) |
| `p_curdrv` byte at 0x37; Dgetdrv/Dsetdrv/Dsetpath/Dgetpath semantics per PD and inheritance at creation | **[C]** |
| `p_curdir[]` bytes, `dirtbl`, use counts, the accessory use-count leak | **[R]** |
| `p_dreg`/`p_areg` save-area contents, `P_A7SAVE` | **[R]** (m68k trap-frame; only the m68k emulator needs it) |
| The PD addresses of accdesk/AES/EmuCON/`initial_basepage` | **[R]** |

## 6. NewTOS: state read today and what the 2026-10-18 decision requires

### 6.1 State read at HEAD `3f1a8109` (cited; reference)

| Topic | Today | Where |
|---|---|---|
| `Pterm` returns through the slot, not through `p_parent` | `parent_slot` of the *child* slot; the parent's frame was saved in the child's slot at `Pexec` | `tos-api/src/bdos/process/pterm.rs:246-260`; `kernel/src/arch/aarch64/exceptions.rs:2140-2160` |
| `RUN` is `null` in kernel context (shell loop, EmuCON); owner of kernel allocations = a kernel sentinel | | `pterm.rs:256-262`; `49-mfree.md` "several owners" |
| `os_run` cell = per-process page-0 cell (`0x5B8`) naming that process | so an accessory sees its own PD | `tos-api/src/bdos/process/loader.rs:950-957` |
| accessory loaded `EnvSpec::Inherit` with `p_parent` 0; stand-in env | | `kernel/src/accs.rs:201-216` |
| kernel launches: `p_parent` = `standin::kernel_run()` (initial / AES) | | `tos-api/src/bdos/process/pexec.rs:498-519`; `kernel/src/launch.rs:252` |
| `Mfree`/`Mshrink` by a non-owner → `EIMBA` (FreeMiNT rule E15) | **contradicts EmuTOS** | `../memory/49-mfree.md` §"several owners" |
| `p_curdrv`/`p_curdir` **global**, PD+0x37 reads 0 | | `TASK/PROC03.md` §3 B3, `loader.rs`/`path.rs` as cited there |

### 6.2 What "as EmuTOS" requires (analysis, not a design)

1. A **`run` value that can name a non-EL0 PD** (accdesk, AES PD, EmuCON PD,
   `initial_basepage`) in kernel context, and an `os_run` view that shows `run`
   to *every* process including an accessory (EmuTOS `*os_run` is one global
   word; NewTOS's cell is per-process page 0 — the cell of an accessory must be
   made to name `run` at every switch into it; a program other than the front
   one never executes between switches).
2. `m_own` tags equal `run` at the call (so allocations by AES code in kernel
   context and by an accessory use the §1.2 value), and `Pterm` frees by tag.
3. A saved-context slot keyed by **`run`'s PD** and written by whichever
   context traps (EmuTOS `rwa.S:188-224`), if `Pterm` is to resume `p_parent`
   literally (§2) — NewTOS today keys it by the child (`parent_slot`,
   `parent_frame`). The common case (accessory `Pexec`: `p_parent` = front
   program, but the process to resume = the accessory) cannot be expressed with
   "resume the process named by `p_parent`" unless the saved context is the
   *last trapping context under that PD*.
4. Per-PD `p_curdrv`/`p_curdir` with inheritance at creation and a decrement at
   `Pterm`, **including the stand-ins**: accdesk's and the EmuCON PD's drive and
   directory are *mutable state* (§4.4), so the stand-in PDs cannot stay one
   read-only constant page.
5. Accessory GEMDOS calls executing as `run` (§1.3 table).

### 6.3 Points the Developer must decide (not decided by the source)

| # | Question | Why the source cannot answer it | Options |
|---|---|---|---|
| D-1 | AES-PD `p_tbase` (`exec_os` = the AES entry in ROM) and accdesk's `accdesk_start`, EmuCON's `coma_start`: a non-zero code address | the AES/EmuCON code is kernel code at a kernel VA; `p_tbase` is a 32-bit user-address field (`pexec.rs:510-519` "`p_parent` is a plain 32-bit LONG (user addresses are below 4 GiB)") | (a) a fixed non-zero placeholder, documented as "by absence"; (b) a published read-only stub address in the user map; the source defines the *meaning* (entry of the AES/accdesk/EmuCON code), not a value any program can use |
| D-2 | `p_hitpa` of the stand-in PDs | EmuTOS value is `p + max`, `max` = the largest free block at creation (`bdos/proc.c:220,374`), state-dependent and stale after the block's `Mshrink` to 0x100 (`gemstart.S:59-69,143-152`, `cmdasm.S:26-37`) | (a) `p+0x100` (= the real block length after `Mshrink`, deviates from the field); (b) really allocate the stand-in PDs as pool blocks with `alloc_tpa` + shrink, exactly as EmuTOS, which also makes owner tags real pool memory |
| D-3 | `p_xdta` of AES PD/accdesk | EmuTOS: `&p_cmdlin` initially, then AES-internal `dos_sdta(&D.g_dta)` (`geminit.c:273`) | `&p_cmdlin` of the stand-in (faithful default); internal changes not observable |
| D-4 | AES PD `p_cmdlin` | in EmuTOS it is the `shel_write` tail buffer (`ad_stail` = AES PD+0x80, `gemstart.S:54-57`, `gemshlib.c:170,191`) | write the stored tail into the stand-in's `p_cmdlin` on every `shel_write(doex=1)` — representable, only chain-readers see it |
| D-5 | How `Pterm` resumes `p_parent` (§6.2.3) and what happens for an altered/invalid `p_parent` | EmuTOS: undefined/crash | a defined safe outcome (the process dies like a crash?) — a new decision, not contract |
| D-6 | Owner tag of kernel-context allocations (AES code, EmuCON) | EmuTOS owner = `run` = AES PD / accdesk PD / EmuCON PD at that time | an owner value for each stand-in; interacts with D-2(b) |
| D-7 | `*os_run` for an accessory (§6.2.1) | design | update the per-process cell on every switch into an accessory — **built, §6.6** |
| D-8 | `Mfree`/`Mshrink` owner rule (E15, FreeMiNT) versus EmuTOS (none) | **conflict**: an accessory that allocates under accdesk (init) and frees later under another `run`, or frees what the front program owns, gets `EIMBA` under E15 but succeeds on EmuTOS | relax/replace E15 (the user's "everything EmuTOS" decision of 2026-10-18 covers PROC03 scope; E15 was decided 2026-10-04) — **decided 2026-10-18 (B5 option 2): E15 stays; memory stays the accessory's, §6.6** |
| D-9 | Which §1.3 rows to reproduce for accessories beyond `Malloc` ownership (cwd, DTA, std handles, file owner, `p_flags` pool choice, `Pterm`, `Pexec` parent) | "as EmuTOS" says all; none is guarded, several are footguns | reproduce all (consistent with the decision) or list explicit exceptions — **decided 2026-10-18 (B5 option 2): only `*os_run` and the `Pexec` child's `p_parent` follow `run`, every other row stays with the accessory, §6.6** |
| D-10 | Leak of `dirtbl` use counts for accessory PDs (§4.5) | EmuTOS accident | not reproduced (invisible except exhaustion) |

### 6.4 Developer decisions D-1…D-5 (TASK/PROC03 stage 1 + B4, 2026-10-18)

Built as described in [`_parent-and-environment.md`](./_parent-and-environment.md)
§4.4. D-6…D-10 are **not** decided here (B5 — the user still had to decide the
D-8 conflict; decided 2026-10-18, §6.6); memory ownership is unchanged.

**D-1 — `p_tbase` of the AES, accdesk and EmuCON PDs: 0 (deviation by
absence).** The source defines the field's *meaning* — the entry of the code
the PD's process runs (`exec_os`, `accdesk_start`, `coma_start`, in ROM) — not
a value a program can use. In NewTOS that code is kernel code: it has no
address in any program's map (the kernel image is not EL0-readable, and the
OSHEADER's `os_beg`/`os_end` bound no "ROM" a program could compare with,
`loader.rs` OSHEADER notes). The candidates were (a) a fixed placeholder —
invented, forbidden by the user decision ("Analyse vorlegen statt Werte zu
erfinden") — and (b) the kernel function's physical address — a number no
program can dereference and that changes with every build. No program in the
SDK reads the field of a parent (`TASK/PROC03.md` §4). So 0, documented as
absence, as Way A had it for the AES PD. `p_tlen`/`p_dlen`/`p_blen` are 0 in
EmuTOS too.

**D-2 — `p_hitpa`: `p + 0x100`; the stand-ins stay a kernel page outside the
pools (option (a)).** EmuTOS's `p + max` is the largest free block at creation
and goes stale at once: the code `Mshrink`s its block to the 0x100-byte PD
without touching the field (`aes/gemstart.S:59-69,143-152`,
`cli/cmdasm.S:26-37`); `p + 0x100` is the block as it really is afterwards.
Option (b) — allocating the stand-ins with `alloc_tpa` in the pools — was
rejected because it is the option that would **block** stage 3: there the
stand-ins become memory *owners* (`m_own = run`, D-6), and an owner tag must
never be confusable with a block of a real process. A kernel-page PD address
can never be the address of a pool block (a real process's PD always is one),
it survives every `Pterm`/`free_all_owned`, and the same address can be used
for "free everything accdesk owned" at `leave_accdesk()` (EmuTOS's accdesk
`Pterm0`, `bdos/proc.c:136-140`) before the next session reuses it. With (b)
the PD block would itself be owned by the stand-in, would need its own
publication to every process (pool blocks are per-owner under `memprot`), and
`p + max` would point at memory the next allocations hand to others.

**D-3 — `p_xdta`: `&p_cmdlin` of the stand-in.** EmuTOS's initial value
(`init_pd_fields`, `bdos/proc.c:375`). Its later changes by AES-internal
`dos_sdta(&D.g_dta)` (`aes/geminit.c:273,810`) have no counterpart — NewTOS's
AES keeps its DTAs in kernel buffers — and no program can tell.

**D-4 — AES PD `p_cmdlin` = `ad_stail`.** Representable, so built: every write
of `ad_stail` in `tos-api/src/aes/shell.rs` (`set_launch_info`,
`shel_write(SHW_EXEC)`, `shel_write(SHW_RESCHNG)`, `reinit`) mirrors the
128-byte buffer into the AES stand-in (`standin::set_aes_cmdlin`); before the
AES starts nothing is mirrored and `enter_aes()` gives the PD its empty tail
(`init_pd_fields`). The accdesk and EmuCON PDs keep the empty tail EmuTOS
gives them.

**D-5 — `Pterm` returns through `p_parent`; a changed value is honoured when
it names a resume point on the process's launch chain, otherwise the recorded
parent resumes.** `tos-api/src/bdos/process/pterm.rs` `return_target`, read
before the teardown as `bdos/proc.c:609` reads it before `ixterm`:

| live `p_parent` at `Pterm` | Outcome | EmuTOS |
|---|---|---|
| = the value written at start (`Process::launch_parent`) | the recorded parent resumes (unchanged path) | same |
| the PD of an ancestor slot `n + 1` launches up (suspended in its `Pexec`) | that ancestor's `Pexec` returns `rc & 0xFFFF`; the `n` processes between are **terminated** (files, memory, AES client released) and never resumed | resumes the ancestor's saved context with `rc`; the processes between are never resumed and **leak** (§2.2 row 3) — NewTOS frees instead of leaking |
| the PD that started the chain's root (the stand-in the kernel launcher, or EmuCON, wrote; or an accessory root's own PD) | the root ends too; the launcher / EmuCON / accessory resumes with `rc` (`sh_ldapp` sees the program end with `rc`) | same (`run = accdesk` resumes `sh_ldapp`'s `dos_exec`) |
| anything else: 0, not a PD, a process not on the chain, a stand-in with no resume point here (e.g. the AES PD or `initial_basepage` for a desktop program), the process itself | the recorded parent resumes as if the field were unchanged; the kernel logs one `[pterm]` line | 0: NULL `run`, crash; otherwise writes `rc` into whatever is there and jumps through its `P_A7SAVE` — undefined (§2.2 rows 4-5) |

Why the fallback is "the recorded parent" and not "kill the parent too": the
recorded parent is the only context NewTOS *knows* is waiting for this child
(its frame is stored with the child's slot), it is what every unchanged
program gets, and it is exactly the observable result of EmuTOS when the
field is not changed — the safe outcome §5 asks for. The ancestor case is
reproduced because it is the only changed value with a defined EmuTOS meaning
(a suspended PD's saved context). Accessory `Pexec` children are unchanged by
this stage (their `p_parent` is still the accessory's PD — B5; since §6.6 it is
`run`, the recorded-parent fallback and the return to the accessory unchanged).

### 6.5 PROC04 point 2 as built (2026-10-18)

`p_curdrv`/`p_curdir[]` live in the PD, `run`'s PD is the one read and written
(`tos-api/src/bdos/fs/curdir.rs`, `run_pd`): the running process's basepage, or
with kernel code on the stack the stand-in `standin::kernel_run_kva()` names
(`initial_basepage` → AES PD → accdesk PD, the EmuCON PD while EmuCON runs —
§1.2, finding §6.2.4: the stand-ins' drive and directories are mutable kernel
state, still read-only to programs).

| EmuTOS | NewTOS |
|---|---|
| `Dgetdrv`/`Dsetdrv` on `run->p_curdrv` (`fsmain.c:420-441`) | the byte at PD+0x37 of `run_pd()`; a program that writes its own byte changes its `Dgetdrv` [C] |
| `p_curdir[d]` = `dirtbl[]` index, `NCURDIR = 40`, slot 0 unused, use counts | same bytes, same 40-slot table with use counts; a slot holds drive + canonical path (`\A\B`, no DND cache in NewTOS) [R] |
| `init_pd_files` copies `p_curdir[]` (+1 use each) and `p_curdrv` from `run` (`bdos/proc.c:401-409`) | `loader::init_pd_files` → `curdir::inherit_from_run`, for every PD the loader creates (modes 0/3/5/7, accessories, kernel launches); the stand-ins inherit the same way when they start (`rom_pd`) |
| `Dsetpath`: incr new, store, decr old; full → `EINTRN` → `EPTHNF` (`fsdir.c:1140-1151`) | same order (`curdir::set_curdir`); full table → `EPTHNF` |
| `ixterm` decrements every `p_curdir[i]` (`bdos/proc.c:126-132`) | `pterm::terminate_inner` → `curdir::release_pd` after the files, before the memory; accdesk's/EmuCON's end (`leave_accdesk`/`leave_emucon`) releases theirs |
| root DND gets a slot lazily in `ckdrv` | the root is `p_curdir[d] == 0`, no slot (same directory, one slot saved) [R] |
| accessory PD's uses leak (D-10) | released in `free_accs` (and on an accessory's failed `Mshrink`) — not reproduced |
| a byte naming any index is followed | a byte that names no live slot of its drive reads as the root, is copied as 0 and never decremented; on a full table the uses are recounted from every live PD (process slots + stand-ins) before `Dsetpath` gives up, so PDs gone without `ixterm` (unstarted `PE_LOAD` children freed by `Mfree`, scribbled bytes) cannot exhaust it [R] |
| stale `dirtbl` slot after a media change → root via `ckdrv` | a stored path that no longer resolves → root (unchanged) |

Kernel-context consequences, all EmuTOS-faithful: the AUTO programs inherit
`initial_basepage` (`p_curdrv = bootdev`, all roots), so the global reset the
launcher ran before every AUTO program (DESK28) is gone; `sh_chdef` →
`set_drvdir` moves accdesk's directory, which the next program inherits
(`NORMAL_APP` from `sh_apdir` = the `shel_write` caller's directory,
`sh_curdrvdir`); EmuCON's `cd`/`d:` move the EmuCON PD and its children inherit
it; `dos_sdrv(bootdev)` on a restart (`drive::set_boot_drive`) writes the AES
PD, which the next accdesk inherits.

**Accessories (B5 decided 2026-10-18, §6.6):** the PD the current-drive and
directory calls of an accessory use is the accessory's own (copied from accdesk
at load), not the PD in front as §1.3 says for EmuTOS — part of the named
deviation of option 2 (`docs/COMPATIBILITY.md`).

Unchanged deviations, not part of this step: `BLKDEVNUM`/`NUMCURDIR` are 16 in
NewTOS (`tos-types::NUMCURDIR`, `drive.rs`), 26 in EmuTOS — `Dsetdrv(16..25)` is
ignored instead of stored; stored paths are limited to 63 bytes (`Dsetpath`
answers `EPTHNF` beyond), EmuTOS's `Dgetpath` budget is 111.

### 6.6 B5 as built — option 2 (user decision 2026-10-18)

User decision (`TASK/PROC03.md`, "Anwenderentscheidung B5"): while a desk
accessory executes, what a program can read **as** `run` follows EmuTOS —
`*os_run` and the `p_parent` of a process the accessory `Pexec`s; memory
ownership, protection, the current drive and directory, the DTA, the standard
handles and the open files stay with the accessory itself (named deviation,
`docs/COMPATIBILITY.md` "Sanctioned deviations"; ADR-027). E15 is untouched.
That answers D-7 (built), D-8 (E15 stays) and D-9 (only the two rows below).
D-6 (owner of kernel-context allocations) and D-10 are not affected.

**`run` as an accessory sees it** — `tos-api/src/bdos/process.rs`
`emutos_run`. EmuTOS's `run` is a stack (`proc_go` pushes, `bdos/proc.c:575`;
`xterm` pops, `:609`) that accessories never touch (§1.2). NewTOS's `RUN` is
the process with the CPU, an accessory included, so the value is derived: the
live process with the highest start number (`Process::start_seq`, set where
`p_parent` is written at the start — `pexec::write_parent`, i.e. for every
`Pexec` 0/4/6 child and every kernel launch — and cleared when its `Pterm`
begins, before anyone else runs). No started process alive → the stand-in
`standin::kernel_run()` (accdesk between programs, EmuCON while EmuCON runs).
For processes ending in the reverse order of their starts, which a
synchronous `Pexec` gives, this is EmuTOS's `run`; if a program ends while a
child an accessory started still lives, NewTOS names the child and then the
stand-in, never the freed PD (EmuTOS: `run = child->p_parent` = the dead
program).

| EmuTOS (`run` unchanged while an accessory executes) | NewTOS |
|---|---|
| `*os_run` is `run` (`bios/startup.S:110-111`, one global word) | the accessory's page-0 cell (`loader::OS_RUN_CELL_VA`) is written with `emutos_run()` in `process::set_current` whenever the accessory becomes the running slot — a scheduler switch, a kernel-park return, the `Pterm` return from its child. `run` changes only by `Pexec`/`Pterm`, each of which ends in `set_current` again, so the cell is current whenever the accessory runs. A program's cell still names itself (unchanged) [C] |
| `Pexec` from an accessory: `p_parent = run` (`bdos/proc.c:541`) | `pexec::write_parent` → `process::run_for_caller`: an accessory caller gives `emutos_run()`; `launch_parent` records that value [C] |
| the child's `Pterm` resumes the accessory, whose trap saved its context into `run`'s PD (`bdos/rwa.S:188-206`) | the child's `parent_slot` is the accessory and its frame is the accessory's `Pexec` frame (`handle_pexec`, unchanged); the live `p_parent` equals `launch_parent`, so `pterm::return_target` takes the normal path; `RUN` is the accessory again [C] |
| a changed `p_parent` in that child (EmuTOS: undefined, §2.2) | D-5 unchanged, and every value ends at the accessory: its PD is the one ancestor slot on the child's launch chain (honoured: the accessory resumes), anything else is not a resume point (the recorded parent, the accessory, resumes) |
| rights the child inherits | the accessory's (`memprot::on_launch` with the real caller) — E5/strict unchanged |
| every other §1.3 row (`Malloc` owner and pool, files, `Fdelete` owner, handles, DTA, drive/directory, the child's environment/handles/directories, an accessory's own `Pterm`) | the accessory's — named deviation |

Measured: `measurements/measurement-2026-10-19-proc03-b5/` (QEMU raspi4b,
memprot on/strict/off): an accessory reads the accdesk stand-in during its
initialisation, the `#Z` program's PD while that program waits in
`evnt_mesag`, NEWDESK's PD (memprot=off: `p_parent` = accdesk) once only the
desktop runs; each of its `Pexec` children reads `p_parent` = that value and
returns `0x42` to the accessory; a block it took while the `#Z` program was in
front survives that program's `Pterm` and `Mfree` answers 0. Host test
`tos-api/tests/proc03_b5_accessory_run.rs`.

## Related

- [`_parent-and-environment.md`](./_parent-and-environment.md) — the chains and PD field contents.
- [`4c-pterm.md`](./4c-pterm.md), [`31-ptermres.md`](./31-ptermres.md), [`4b-pexec.md`](./4b-pexec.md).
- [`../memory/48-malloc.md`](../memory/48-malloc.md), [`49-mfree.md`](../memory/49-mfree.md), [`4a-mshrink.md`](../memory/4a-mshrink.md).
- [`../directory/3b-dsetpath.md`](../directory/3b-dsetpath.md), [`47-dgetpath.md`](../directory/47-dgetpath.md), [`../drive/0e-dsetdrv.md`](../drive/0e-dsetdrv.md), [`19-dgetdrv.md`](../drive/19-dgetdrv.md).
- [`../../aes/accessories.md`](../../aes/accessories.md).

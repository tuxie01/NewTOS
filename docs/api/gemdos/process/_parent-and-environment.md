# `p_parent` and the environment of every process (TASK/PPARENT, corrected by TASK/PROC03)

> Spec extraction, cross-cutting `Pexec` / basepage / AES. Source: EmuTOS @
> `1a7484ab` (`references/emutos`).
> **Re-validated 2026-10-18 (TASK/PROC03).** The first version of this file
> (TASK/PPARENT, 2026-10-06, "Way A") said the AES PD is the parent of the
> desktop and of every `#Z`/desktop-started program. **That was wrong.** The
> parent is a *third* PD, the **accdesk PD**, and a program started from the
> EmuCON has a *fourth*, the **EmuCON PD**. Corrections are marked
> **CORRECTION**. User decision 2026-10-18: NewTOS follows EmuTOS here
> ("alles EmuTOS-treu", `TASK/PROC03.md`). The companion file for `run`,
> memory ownership and the per-PD current directory is
> [`_run-ownership-curdir.md`](./_run-ownership-curdir.md).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** the value of `p_parent` (0x24) for every way a process is
> started, i.e. the *whole chain* a program can walk (depth and the
> readable content of each PD on it); the content of the block `p_env`
> (0x2C) names; that the PD `p_parent` names is a readable basepage.
> mintlib tells a desk accessory by `p_parent == 0` (`sdk/libc/mintlib/startup/crt0.S:52-56`),
> `getppid` falls back to `p_parent>>8` (`sdk/libc/mintlib/unix/getppid.c:22`).
> **Reference:** where the stand-in PDs live, how they are published, the
> saved-register area (`p_dreg`/`p_areg`), the `p_curdir[]` index bytes.
> Labels: **(read)** = stated by the cited code; **(inferred)** = reasoned
> from cited code.

## 1. EmuTOS: where `p_parent` comes from

`p_parent` is written in exactly one place, `proc_go` (`bdos/proc.c:541`):

```c
p->p_parent = run;
```

`init_pd_fields` zeroes the whole PD first (`bdos/proc.c:370`,
`bzero(p, sizeof(PD))`), so a PD that never passes `proc_go` keeps
`p_parent == 0`. `proc_go` runs for `PE_LOADGO`, `PE_GO` and `PE_GOTHENFREE`
(`bdos/proc.c:238-248,358-359`); `PE_LOAD`, `PE_BASEPAGE` and
`PE_BASEPAGEFLAGS` never call it, and `proc_go` **overwrites** any value a
loader put there beforehand.

`run` at each start (the writes to `run`, and its value over time, are in
[`_run-ownership-curdir.md`](./_run-ownership-curdir.md) §1):

| Start | `run` when it is started → `p_parent` | Source |
|---|---|---|
| GEMDOS initialisation | `run = &initial_basepage` (a static `PD`, `bdos/bdosmain.c:78`) | `bdos/bdosmain.c:337-342` |
| AUTO program | `Pexec(PE_LOADGO, "\AUTO\…", "", NULL)` from the BIOS: `&initial_basepage` | `bios/bios.c:685-695` |
| The AES (`exec_os` = `ui_start`) | `Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", default_env)` + `Pexec(PE_GO)` from the BIOS: `&initial_basepage` | `bios/bios.c:855-866`, `bios/endrom.c:31` |
| **accdesk PD** (accessories + desktop shell, one PD per AES start) | `gem_main` → `aes_run_rom_program(accdesk_start)`: `Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", NULL)` + `dos_exec(PE_GOTHENFREE)` with `run` = the **AES PD**: `p_parent` = AES PD | `aes/geminit.c:894`, `aes/gemshlib.c:570-581` |
| **CORRECTION** — the ROM desktop (`deskstart`) | `sh_ldapp` runs in the accdesk process: `aes_run_rom_program(deskstart)` with `run` = **accdesk PD** | `aes/gemshlib.c:600-611`; `accdesk_start` → `run_accs_and_desktop` → `sh_main`, `aes/gemstart.S:139-160`, `aes/geminit.c:739` |
| **CORRECTION** — EmuCON (`CONSOLE_APP`, `WITH_CLI`) | `aes_run_rom_program(coma_start)` from `sh_ldapp`: `run` = **accdesk PD** | `aes/gemshlib.c:614-620` |
| **CORRECTION** — a program the desktop asked for / the `#Z` autorun program | `sh_ldapp` → `dos_exec(PE_LOADGO, D.s_cmd, ad_stail, ad_envrn)` with `run` = **accdesk PD** (the desktop has already terminated; between programs `run` is accdesk) | `aes/gemshlib.c:645`, `bdos/proc.c:541` |
| Desk accessory | `pgmld` → `Pexec(PE_LOAD, pname, "", NULL)` with `run` = accdesk PD, then `pstart` — never `proc_go`: **0** | `util/gemdos.c:29-47`, `aes/geminit.c:236-257`, `bdos/proc.c:370` |
| Child of a program (`Pexec` 0/4/6) | `run` — **the caller's PD for a program; for a call made by an accessory or by AES code `run` is the PD in front, not the accessory** (`_run-ownership-curdir.md` §1.3) | `bdos/proc.c:541` |
| Program started by EmuCON (`Pexec(0, path, cmdline, NULL)`) | `run` = **EmuCON PD** | `cli/cmdexec.c:52` |
| Early console (`BOOTFLAG_EARLY_CLI`, `WITH_CLI`) / `COMMAND.PRG` (`cmdload`) | `&initial_basepage` (not the default boot) | `bios/bios.c:834-843,850-855` |

**Why the accdesk PD exists** (read): `run_accs_and_desktop` "runs under a
separate process which terminates on shutdown or resolution change … This
ensures that all memory allocated to or by desk accessories is automatically
freed on resolution change" (`aes/geminit.c:646-652`). `accdesk_start` first
`Mshrink`s its own block and switches to supervisor mode
(`aes/gemstart.S:139-158`), calls `run_accs_and_desktop` and ends with
`Pterm0` (`:160-163`); `gem_main` loops back to `aes_restart` and a fresh
accdesk PD whenever `gl_changerez` is set (`aes/gemstart.S:121-124`).

`dos_exec` is a plain `trap #1` `Pexec` (`aes/gemstart.S:169-243`); the AES
does not switch GEMDOS's `run`.

### 1.1 The `p_parent` chain per start path (read; chain depth = the number of
PDs from the program to the 0 terminator, counted including the program)

| Start | Chain (child → … → 0) | Depth |
|---|---|---|
| AUTO program | AUTO → `initial_basepage` → 0 | 3 |
| the AES PD | AES → `initial_basepage` → 0 | 3 |
| accdesk PD | accdesk → AES → `initial_basepage` → 0 | 4 |
| ROM desktop, `#Z`/desktop-started program, `shel_write` program | X → **accdesk → AES → `initial_basepage` → 0** | 5 |
| EmuCON PD | EmuCON → accdesk → AES → `initial_basepage` → 0 | 5 |
| program started by EmuCON | X → EmuCON → accdesk → AES → `initial_basepage` → 0 | 6 |
| `Pexec` child of any program X (not an accessory) | child → X → (chain of X) | |
| `Pexec` child called from an accessory while program P is in front | child → **P** → (chain of P) | |
| accessory | 0 | 1 |

The NewTOS ticket draft's table (`TASK/PROC03.md` §6) matches this except for
the accessory-`Pexec` row, where it said "caller" and the source says `run`.

## 2. EmuTOS: what the four PDs contain

Kind column: **S** fixed by source; **D** dynamic (depends on memory/boot/drive
state or on later use); **I** m68k save area (`include/asmpd.h:25-29`),
internal. "NewTOS" column: how far it can be represented without an AES TPA —
**yes**, **partly** (value defined by the source but not reproducible
literally), **no**.

### 2.1 `initial_basepage` (`bdos/bdosmain.c:78`, static, BSS)

| Field | Value | Kind | NewTOS | Source |
|---|---|---|---|---|
| `p_lowtpa`, `p_hitpa`, `p_tbase`, `p_tlen`, `p_dbase`, `p_dlen`, `p_bbase`, `p_blen` | 0 | S | yes | BSS |
| `p_xdta` | 0 until the BIOS `Fsetdta(&dta)` (`bios/bios.c:713,728`), then a pointer into the BIOS stack frame that no longer exists | D | partly (0) | `bdos/fsmain.c:405` |
| `p_parent` | 0 (root) | S | yes | BSS |
| `p_flags` | `PF_STANDARD` (7) | S | yes | `bdos/bdosmain.c:340` |
| `p_env` | `&double_nul` = `{0, 0}` | S | yes | `bdos/bdosmain.c:81,341` |
| `p_uft[0..5]` | `-1,-1,-2,-3,-1,-1` (`default_handle`) | S | yes | `bdos/console.c:60-68,107-112`, `bdos/console.h:23-25`, `stdhdl_init` `bdos/bdosmain.c:347` |
| `p_lddrv` | 0 (declared, never written in EmuTOS) | S | yes | `include/bdosdefs.h:85` (grep: no writer) |
| `p_curdrv` | `bootdev` (`Dsetdrv(bootdev)` while it is `run`) | S | yes | `bios/bios.c:827` |
| `p_curdir[]` | root slot of the boot drive (created by the BIOS' `Fsfirst("\AUTO\*.PRG")`, `ckdrv`), others 0 unless used | D, reference | partly | `bdos/fsdrive.c:112-132`, `bios/bios.c:713-728` |
| `p_cmdlin`, `p_dreg`, `p_areg` | 0 (BSS); the register area is overwritten by every trap #1 while it is `run` | I | no (not needed) | `bdos/rwa.S:188-206` |

### 2.2 The AES PD (`bios/bios.c:861-865`)

Created by `Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", default_env)` (mode 7,
`bdos/proc.c:212-236`), then `p_tbase = exec_os`, `p_tlen = p_dlen = p_blen =
0`, then `Pexec(PE_GO)` (`proc_go`).

| Field | Value | Kind | NewTOS | Source |
|---|---|---|---|---|
| `p_lowtpa` | the PD itself | S | yes | `bdos/proc.c:373` |
| `p_hitpa` | `p + max`, `max` = the largest free block `alloc_tpa` took at that moment (pool and size depend on RAM state) — **left unchanged** by the `Mshrink` to 0x100 that `ui_start` does at once | D | partly — see `_run-ownership-curdir.md` §6.3 D-2 | `bdos/proc.c:220,374,460-502`, `aes/gemstart.S:59-69`, `bdos/umem.c:170-229` (no basepage write) |
| `p_tbase` | `exec_os` = `ui_start`, the AES entry in the ROM image | S (meaning) | no (D-1) | `bios/bios.c:534,863`, `bios/endrom.c:31` |
| `p_tlen`, `p_dlen`, `p_blen` | 0 | S | yes | `bios/bios.c:864` |
| `p_dbase`, `p_bbase` | 0 | S | yes | `bdos/proc.c:370` |
| `p_xdta` | `&p_cmdlin` at creation; later the AES' own `dos_sdta(&D.g_dta)` (while `run` = AES PD, `aes/geminit.c:273,810`) | D | partly (D-3) | `bdos/proc.c:375` |
| `p_parent` | `&initial_basepage` | S | yes | `bdos/proc.c:541` |
| `p_flags` | `PF_STANDARD` (7) | S | yes | `bdos/proc.c:234` |
| `p_env` | a 12-byte copy of `default_env`; **this block is `ad_envrn`** (`save_ad_envrn` is restored on every AES restart) | S | yes | `bdos/proc.c:215,376,413-432`, `aes/gemstart.S:78-97` |
| `p_uft` | copied from `initial_basepage`: the six default handles | S | yes | `bdos/proc.c:391-398` |
| `p_lddrv` | 0 | S | yes | — |
| `p_curdrv`, `p_curdir[]` | inherited from `initial_basepage` at creation; afterwards changed by the AES running as this PD (rez change `dos_sdrv(bootdev)`, `aes/geminit.c:796-801`) | D | partly (the byte yes, `p_curdir[]` reference) | `bdos/proc.c:401-409` |
| `p_cmdlin` | `""` at creation; **afterwards the `shel_write` command-tail buffer**: `ad_stail` = this PD + 0x80 (`aes/gemstart.S:54-57`), written by `sh_write` (`aes/gemshlib.c:170,191`) and read by `sh_read` (`:105`) | D | yes, if the AES writes the tail into the stand-in (D-4) | as given |
| `p_dreg`, `p_areg` | the AES' pending `dos_exec` context (it is `run` when it calls `Pexec`) | I | no | `bdos/rwa.S:188-206` |
| block | owner `initial_basepage` (`set_owner(p, run)`, `PE_GO` keeps it); block shrunk to 0x100 by `ui_start` | reference | — | `bdos/proc.c:229-230`, `aes/gemstart.S:59-69` |

### 2.3 The accdesk PD (`aes/gemshlib.c:570-581`) — **new in this spec**

`pd = Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", NULL); pd->p_tbase = entry;
dos_exec(PE_GOTHENFREE, NULL, pd, NULL)`, with `run` = AES PD, `entry` =
`accdesk_start`. `aes_run_rom_program` does **not** zero `p_tlen/p_dlen/p_blen`
explicitly (the BIOS variant does) — they are 0 because `init_pd_fields`
`bzero`ed the PD.

| Field | Value | Kind | NewTOS | Source |
|---|---|---|---|---|
| `p_lowtpa` | the PD itself | S | yes | `bdos/proc.c:373` |
| `p_hitpa` | `p + max` (largest free block at creation); the block is `Mshrink`ed to 0x100 by `accdesk_start`, the field is not | D | partly (D-2) | `bdos/proc.c:374`, `aes/gemstart.S:143-152` |
| `p_tbase` | `accdesk_start` (`aes/gemstart.S:139`) | S (meaning) | no (D-1) | `aes/gemshlib.c:575-576` |
| `p_tlen`, `p_dlen`, `p_blen`, `p_dbase`, `p_bbase` | 0 | S | yes | `bdos/proc.c:370` |
| `p_xdta` | `&p_cmdlin`; the AES' `dos_sdta` calls while `run` = accdesk change it | D | partly (D-3) | `bdos/proc.c:375` |
| `p_parent` | the AES PD | S | yes | `bdos/proc.c:541` |
| `p_flags` | `PF_STANDARD` (7) | S | yes | `bdos/proc.c:234`, `aes/gemshlib.c:575` |
| `p_env` | a copy of the **AES PD's** `p_env` (`env == NULL` → `run->p_env`), 12 bytes | S | yes | `bdos/proc.c:419-420,421-429` |
| `p_uft` | copied from the AES PD: the six default handles | S | yes | `bdos/proc.c:391-398` |
| `p_curdrv`, `p_curdir[]` | copied from the AES PD at creation, then **rewritten by the shell on every launch** (`set_drvdir`, `run` = accdesk) | D | the byte yes; `p_curdir[]` reference — **mutable state** | `bdos/proc.c:401-409`, `aes/gemshlib.c:526-554` |
| `p_cmdlin` | `""` | S | yes | `bdos/proc.c:378-384` |
| `p_dreg`, `p_areg` | the pending context of the AES `dos_exec`s it makes | I | no | `bdos/rwa.S:188-206` |
| block | owner **itself** (`PE_GOTHENFREE` `set_owner(p,p)`, also the env); shrunk to 0x100 | reference | — | `bdos/proc.c:238-243` |
| lifetime | from `gem_main:894` to the `Pterm0` after `run_accs_and_desktop` — shutdown or resolution change; **a new PD per AES start** | S | — | `aes/gemstart.S:121-124,160-163` |

While it is `run` (between programs, during accessory initialisation) the AES
executes GEMDOS calls as this PD (`_run-ownership-curdir.md` §1.2).

### 2.4 The EmuCON PD (`WITH_CLI`; `aes/gemshlib.c:614-620`) — **new in this spec**

Same creation as 2.3 with `entry = coma_start`, `run` = accdesk PD. Differences:

| Field | Value | Kind | NewTOS | Source |
|---|---|---|---|---|
| `p_tbase` | `coma_start` (`cli/cmdasm.S:22`) | S (meaning) | no (D-1) | `aes/gemshlib.c:618`, `bios/endrom.c:33` |
| `p_parent` | accdesk PD | S | yes | `bdos/proc.c:541` |
| `p_env` | a copy of accdesk's `p_env` (= the AES environment); `coma_start` stores it in `_environment` and EmuCON only reads it | S | yes | `bdos/proc.c:419-420`, `cli/cmdasm.S:40-44`, `cli/cmdmain.c:223-245` |
| `p_curdrv`, `p_curdir[]` | accdesk's, which `sh_chdef` has just set from `sh_cdir` (`set_drvdir`, `aes/gemshlib.c:545-548`); then EmuCON's own `cd`/`d:` change **this PD's** (`cli/cmdint.c:250-255,567`) | D | **mutable state** | `bdos/proc.c:401-409` |
| `p_hitpa` | `p + max`; block shrunk to 0x100 by `coma_start` | D | partly (D-2) | `cli/cmdasm.S:26-37` |
| block | owner itself (`PE_GOTHENFREE`); freed when EmuCON ends (`Pterm0`, `cli/cmdasm.S:48-49`) | reference | — | `bdos/proc.c:238-243` |
| lifetime | one PD per console start; `run` = EmuCON PD while EmuCON executes and while it `Pexec`s programs | S | — | `aes/gemshlib.c:614-622` |

**The ROM desktop's PD** (`deskstart`) is built the same way (parent accdesk,
`p_tbase = deskstart`, env copy of accdesk's). NewTOS's desktop is the EL0
process NEWDESK (ADR-026), a real PD; its parent must be the accdesk
stand-in, its environment a copy of the accdesk stand-in's.

### 2.5 `default_env` (`bios/bios.c:77-80,626-636`)

```c
#define ENV_SIZE 12        /* sufficient for standard PATH=^X:\^^ (^=nul byte) */
#define DEF_PATH "A:\\"
strcpy(default_env, PATH_ENV);                 /* "PATH=" (bdosdefs.h:128) */
p = default_env + sizeof(PATH_ENV);            /* past the NUL of "PATH=" */
strcpy(p, DEF_PATH); *p += bootdev;            /* "X:\" */
p += sizeof(DEF_PATH); *p = '\0';              /* the double NUL */
```

Bytes (bootdev = 2): `50 41 54 48 3D 00 43 3A 5C 00 00 00` —
`"PATH=" NUL "C:\" NUL NUL` plus the twelfth (static, zero) byte: the Atari TOS
`PATH=` quirk with the value as a separate string after the `=`'s NUL
(`sh_find` skips that NUL, `aes/gemshlib.c:448-455`). `envsize` counts 11
bytes (`bdos/proc.c:150-159`), `alloc_env` copies `(11 + 1) & ~1 = 12`
(`bdos/proc.c:421-429`).

## 3. EmuTOS: the environment every process gets

`alloc_env(flags, env)` (`bdos/proc.c:413-432`) runs for every load and
basepage mode: `env == NULL` → a copy of `run->p_env`, otherwise a copy of
`env`, size `(envsize + 1) & ~1`, in ST-RAM or (with `PF_TTRAMLOAD`) TT-RAM
preferred.

| Start | `env` argument / `run` | environment content |
|---|---|---|
| AUTO program | `NULL`, `run` = `initial_basepage` | `{0, 0}` (2 bytes) |
| The AES | `default_env` | `default_env` (12 bytes) |
| accdesk PD | `NULL`, `run` = AES PD | copy of the AES PD's `p_env` (= `ad_envrn`) |
| ROM desktop, EmuCON | `NULL`, `run` = **accdesk PD** (CORRECTION: was "AES PD") | copy of accdesk's `p_env` (same bytes) |
| Desktop / `#Z` program | `ad_envrn` passed explicitly (`run` = accdesk) | copy of `ad_envrn` (12 bytes) |
| Desk accessory | `NULL`, `run` = **accdesk PD** (CORRECTION: was "AES PD") | copy of accdesk's `p_env` (12 bytes) |
| Program started by EmuCON | `NULL`, `run` = EmuCON PD | copy of EmuCON's `p_env` |
| Child of a program | `NULL` → the caller's (the `run`'s) `p_env`, else the given block | as given |

The bytes are identical for every row that copies the AES environment: nothing
in the AES or in EmuCON writes the environment (`ad_envrn` is only read,
`aes/gemshlib.c:338-347`; EmuCON only reads, `cli/cmdmain.c:223-245`).

## 4. NewTOS mapping

### 4.1 Way A as built 2026-10-06 (reference; superseded by §4.4)

NewTOS has no AES process and no `initial_basepage`: the AES and the shell run
in the kernel and `RUN` is `null` while kernel code is on the stack. Way A
(`tos-api/src/bdos/process/standin.rs`) gives two stand-in PDs in one kernel
page, `initial_basepage` (+0x000), the AES PD (+0x100), `double_nul` (+0x200)
and the AES environment (+0x210, `ENV_SIZE` = 12 bytes), published read-only
at VA = PA to every process (`usermem::expose`, `PROT_PR`, ADR-023):
a kernel page, so no `Pterm`/`free_all_owned` can reach it. Content as §2.1-2.2
with: `p_curdrv` = `bootdev`, AES PD `p_hitpa = p_lowtpa + 0x100`, `p_tbase = 0`
(deviations by absence). `standin::kernel_run()` is `initial_basepage` until
`enter_aes()` and the AES PD from then on; `pexec::set_parent` writes the
caller's PD, or `kernel_run()` when `current()` is `null`
(`tos-api/src/bdos/process/pexec.rs:498-519`); the kernel launcher does the same
before the first instruction (`kernel/src/launch.rs:252`); accessories keep
the loader's 0 (`kernel/src/accs.rs:201-216`). `EnvSpec::Inherit` with no
running process copies the `kernel_run()` stand-in's environment.

### 4.2 Required by the 2026-10-18 decision (target contract)

* The chain of §1.1 for every start path: **desktop, `#Z`, `shel_write` and
  NEWDESK programs have `p_parent` = accdesk stand-in**; the accdesk stand-in's
  `p_parent` = AES stand-in; AES → `initial_basepage` → 0. Two more
  stand-ins: **accdesk** (always, from `enter_aes()`), **EmuCON PD** (one per
  console start; the in-kernel EmuCON is this PD: it is `kernel_run()` while it
  executes and the parent of what it `Pexec`s).
* `kernel_run()` from `enter_aes()` on is the accdesk stand-in between programs
  and while the shell does AES work (EmuTOS `run`, §1.2); the AES stand-in is
  `run` only during AES initialisation (`aes/geminit.c:753-894`) and after
  accdesk ends.
* Fields per §2 with the **partly/no** entries decided by the Developer
  (`_run-ownership-curdir.md` §6.3 D-1…D-4, D-6). The stand-ins hold **mutable**
  state (drive/directory, `p_cmdlin` tail, `p_xdta`): the read-only constant
  page of Way A cannot remain the only representation (kernel-writable, EL0
  read-only is enough for chain readers).
* Environments per §3: accdesk's and EmuCON's = copies of the AES environment
  (12 bytes each, own blocks readable by EL0).
* `shel_envrn` still searches the **AES PD's** `p_env` (`ad_envrn`,
  `aes/gemstart.S:78-97`, `aes/gemshlib.c:338-347`), gemlib's `mt_shel_envrn`
  steps over the NUL after `PATH=` (`sdk/gemlib/gemlib-0.44.0/a_shel_envrn.c:40-50`).

### 4.3 What a program sees (observable)

| Start | `p_parent` | `p_env` content |
|---|---|---|
| AUTO program | `initial_basepage` stand-in | `00 00` |
| Desktop launch, `#Z` program (TeraDesk, QED, APITEST, …), NEWDESK | **accdesk** stand-in | `PATH=\0X:\\\0\0` (12 bytes, X = boot drive) |
| Desk accessory | 0 | `PATH=\0X:\\\0\0` |
| `Pexec` child of a program | the caller's PD (accessory case: the PD in front, `_run-ownership-curdir.md` §1.3) | the caller's `p_env` copy, or the given block |
| `Pexec` from EmuCON | **EmuCON** stand-in | the EmuCON stand-in's environment (= AES copy) |

Chain read from a `#Z` program: accdesk → AES → `initial_basepage` → 0.

### 4.4 As built (TASK/PROC03 stage 1 + B4, 2026-10-18)

`tos-api/src/bdos/process/standin.rs` — the one stand-in page (kernel-writable,
published read-only to every process at VA = PA, `PROT_PR`) now holds four
PDs; the Way-A offsets are unchanged:

| Page offset | Content |
|---|---|
| +0x000 | `initial_basepage` (§2.1) |
| +0x100 | AES PD (§2.2) |
| +0x200 / +0x210 | `double_nul` / AES environment = `ad_envrn` (12 bytes) |
| +0x300 | **accdesk PD** (§2.3), rebuilt by `enter_accdesk()` at the start of every AES session |
| +0x400 | **EmuCON PD** (§2.4), rebuilt by `enter_emucon()` at every console start |
| +0x500 / +0x510 | accdesk / EmuCON environment (copies of their parent's, 12 bytes) |

`kernel_run()` (EmuTOS `run` while kernel code is on the stack) follows §1.2 of
the companion file: `initial_basepage` → `enter_aes()` (`kernel/src/lib.rs`,
after the AUTO folder) → AES PD → `enter_accdesk()` at the top of each pass of
the AES session loop (= `aes_run_rom_program(accdesk_start)`, before
`gsx_init`/`load_accs`) → accdesk PD → `leave_accdesk()` after `gsx_wsclose`
(= `accdesk_start`'s `Pterm0`) → AES PD. `enter_emucon()`/`leave_emucon()`
bracket the in-kernel EmuCON (`kernel/src/shell.rs`, `run_console`,
`run_console_without_desktop`); the EmuCON PD's parent is whatever `run` was
(the accdesk PD in the shell loop's CONSOLE arm and its fallbacks).

Fields: §2.3/§2.4 with the Developer decisions of
[`_run-ownership-curdir.md`](./_run-ownership-curdir.md) §6.4 — `p_tbase = 0`
(D-1), `p_hitpa = p + 0x100` (D-2), `p_xdta = &p_cmdlin` (D-3); the AES PD's
`p_cmdlin` mirrors `ad_stail` (D-4, every write in `tos-api/src/aes/shell.rs`).
`p_uft`, `p_curdir[]` and `p_curdrv` are copied from the creating stand-in
(`init_pd_files`); since PROC04 point 2 the stand-ins' drive and directories are
mutable kernel state, `run`'s for the current-drive/directory calls in kernel
context ([`_run-ownership-curdir.md`](./_run-ownership-curdir.md) §6.5).

Resulting chains (measured, `measurements/measurement-2026-10-18-proc03-s1/`,
QEMU raspi4b, memprot on/off/strict): AUTO depth 3; `#Z` and `shel_write`
programs depth 5 (X → accdesk → AES → `initial_basepage` → 0); a program
started by EmuCON depth 6; accessory 0; a `Pexec` child → its caller. NEWDESK
takes the same `launch()` path as the `#Z` program (`set_kernel_parent` →
accdesk); it was not probed (embedded binary).

Not changed in this stage (B5, `_run-ownership-curdir.md` §6.3 D-6…D-9, user
decision pending then): `RUN`/`*os_run` are still `null`/the per-process cell in
kernel context and during an accessory, memory owner tags of kernel-context
allocations are the kernel sentinel, an accessory's `Pexec` child gets the
accessory's PD as `p_parent` (EmuTOS: `run`, §1.1). The accessory environment
is now the accdesk copy (same bytes as before).

**B5, option 2 (user decision 2026-10-18; as built,
[`_run-ownership-curdir.md`](./_run-ownership-curdir.md) §6.6):** a process a
desk accessory `Pexec`s gets `p_parent` = EmuTOS's `run` — the program in
front (or its running `Pexec` child), NEWDESK when only the desktop runs, the
accdesk stand-in between programs — and its `Pterm` resumes the accessory;
the accessory's `*os_run` names the same PD. Its environment, handles,
directories and rights are still copied from the accessory (named deviation,
`docs/COMPATIBILITY.md`). Chain read from such a child while a `#Z` program X
is in front: X → accdesk → AES → `initial_basepage` → 0 (§1.1 row "`Pexec`
child called from an accessory"). `RUN` in kernel context and the owner tags
of kernel-context allocations are unchanged.

## Edge cases

1. A program reads its own `p_parent` chain: it must not fault in any
   protection mode (every stand-in is published readable); expected depth in
   §1.1.
2. A program writes into a stand-in: EmuTOS has no protection (a write into the
   AES PD would corrupt it). NewTOS: read-only for EL0, the write faults.
3. `PE_LOAD` + `PE_GO` from a kernel caller: `p_parent` is set at `PE_GO`
   (`proc_go`), not at load.
4. **CORRECTION.** A `Pexec` child called from an accessory: EmuTOS has no
   GEMDOS process for an accessory; `run` is the PD in front (the desktop's,
   EmuCON's, a program's, or accdesk's between programs), **not "the AES PD"**
   as this file said; the child's `p_parent` is that PD
   (`bdos/proc.c:541`, `_run-ownership-curdir.md` §2.2). NewTOS writes `run`
   too since TASK/PROC03 B5 (§4.4; until then the accessory's own PD).
5. mintlib's accessory test (`sdk/libc/mintlib/startup/crt0.S:46-62`): `a0->p_parent == 0`;
   `sdk/libc/newtos-arch/heap_init.c` and `sdk/crt0.S` use that test.

## Related

- [`_run-ownership-curdir.md`](./_run-ownership-curdir.md) — `run`, ownership, cwd per PD.
- [`4b-pexec.md`](./4b-pexec.md) — NewTOS notes, "Kernel-launched programs".
- [`../../structures/basepage.md`](../../structures/basepage.md) — `p_parent`, `p_env`.
- [`../../aes/scrap-shell.md`](../../aes/scrap-shell.md) — `shel_envrn`.
- [`../../aes/accessories.md`](../../aes/accessories.md) — accessory start.

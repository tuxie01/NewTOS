# AES Scrap Manager (`scrp_*` 80–82) + Shell Manager (`shel_*` 120–127)

> **PHASE-00b D8, Batch 7 (final, full-contract refinement).**
> Source: EmuTOS @ `1a7484ab`.
> Primary: `aes/gemsclib.c` (`sc_read`/`sc_write`/`sc_clear`),
> `aes/gemshlib.c` (`sh_read`/`sh_write`/`sh_get`/`sh_put`/`sh_find`/
> `sh_envrn`/`sh_rdef`/`sh_wdef`, the shell launch loop `sh_main`/`sh_ldapp`).
> Dispatch arms `aes/gemsuper.c:308-319,380-405`. Slot macros + `shel_write`
> mode flags `include/aesdefs.h:456,600-621`; opcode numbers
> `include/aesdefs.h:80-82,108-115`. Buffer sizes `aes/gemlib.h:97,120-129`,
> `include/sysconf.h:51-74`. `PATH_ENV` `include/bdosdefs.h:128`.
> Cross-checked for the `SHW_RESCHNG`/`SHW_SHUTDOWN` value semantics (D-101)
> against `aes/geminit.c:343-392` (`process_inf1`, a second producer of
> `gl_changerez`/`gl_nextrez`) and `aes/geminit.c:753,775-793` (`gem_main`'s
> resolution-apply/consumer code) and `desk/deskmain.c:772-776,900-914` (real
> `shel_write` call sites); `WITH_CLI` build flag `Makefile:150,534`.
>
> Call convention + AESPB layout: [`_dispatcher.md`](./_dispatcher.md).
> Shared AES globals (the scrap dir `D.g_scrap`, the shell buffer
> `D.g_shelbuf`, the command/tail `D.s_cmd`/`ad_stail`, the environment
> `ad_envrn`): [`_state.md`](./_state.md) §6c.
>
> **Third-round re-validation (2026-07-11).** Re-checked line-by-line against
> `aes/gemsclib.c`, `aes/gemshlib.c` @ `1a7484ab` (unchanged from the D-054
> pass). The ABI slot layout (D-054's `shel_write` four-mode table, the
> `shel_envrn` `addr_in[0]`/`[1]` swap) re-verified **correct**. Found one
> substantial *value*-semantics gap the slot-level D-054 pass did not probe:
> **`SHW_RESCHNG`'s `isgem` (`int_in[1]`) is not a plain Setscreen mode
> number** — ST/TT-family requests (`isover=0`) require a caller-added `+2`
> offset that the consumer (`geminit.c`) subtracts back off; Falcon-family
> requests (`isover=1`) use the raw value. Also found that `SHW_SHUTDOWN`'s
> `isover` parameter, while part of the documented ABI, is never read by the
> current `sh_write` implementation. See D-101 in
> `docs/api/_discrepancies.md`.
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** the opcode numbers (80–82, 120–127); each call's
> `int_in`/`addr_in` slot layout (note **`shel_write` takes three int_in words
> `doex`/`isgem`/`isover`** and two addr_in strings; **`shel_envrn`'s search
> string is `addr_in[1]`, not `[0]`**); the **`shel_write` mode-flag values**
> (`SHW_NOEXEC 0`, `SHW_EXEC 1`, `SHW_SHUTDOWN 4`, `SHW_RESCHNG 5`) and their
> `isgem`/`isover` meanings; the **file-based clipboard model** (`scrp_*`
> manages a directory *path* string, not the data); the **`shel_find` search
> order** (app dir → current dir → drive root → AES `PATH=`) and its in-place
> rewrite + `1`/`0` return; the `shel_envrn` "pointer after the match, or NULL"
> result; the `shel_read` command+`CMDTAILSIZE(128)`-byte tail; the
> `shel_get`/`shel_put` `SIZE_SHELBUF(4192)`-byte buffer. **Reference (NewTOS
> reimplements the *how* freely):** the `SHELL sh` struct's internal shape,
> `sh_main`'s desktop/console/app launch loop *control flow*,
> `sh_chgrf`/`sh_tographic`/`sh_toalpha` alpha↔graphic switching *mechanics*,
> `aes_run_rom_program`'s nested-`Pexec` trick, the `D.g_scrap`/`D.g_shelbuf`/
> `D.s_cmd`/`ad_stail` storage, and the `dos_sfirst`/`dos_snext` scan internals.
>
> **Correction (2026-09-01, see
> [`shell-loop.md`](./shell-loop.md)).** The line above previously read "the
> `SHELL sh` struct, `sh_main`'s … launch loop, … `aes_run_rom_program`, …
> these are EmuTOS's cooperative-shell internals NewTOS rebuilds … freely" —
> unqualified, and too broad. **The launch loop's *implementation shape* is
> reference; the *process-identity numbering it produces is not*.**
> `sh_ldapp` (`gemshlib.c:592-689`) deliberately reuses **one** persistent
> AESPD (slot 0, "dos appl") for the desktop and for every application it
> launches, renaming it in place via `p_nameit`/`p_setappdir` — never
> allocating a second descriptor. The resulting values (`ap_id == 0`,
> `ap_private == 0x1143` by default) are measured facts about a real
> machine (a genuine TT), not implementation choices, and are **contract**:
> see `shell-loop.md` §4–§5, §11. Any NewTOS reimplementation of this loop —
> however it is coded — must reproduce those numbers.

---

## 1. Scrap Manager (`scrp_*`) — the file-based clipboard

GEM's clipboard is **file-based**: applications agree on a **scrap directory**
(e.g. `C:\CLIPBRD\`) and exchange data as `SCRAP.*` files (`SCRAP.TXT`,
`SCRAP.IMG`, …) written/read there via GEMDOS. The Scrap Manager itself manages
**only the directory-path string** `D.g_scrap` — it never touches the data. To
copy, an app `scrp_read`s the path and writes `SCRAP.TXT` there; to paste, it
`scrp_read`s the path and reads the file back.

### `scrp_read` (80) — `sc_read`

**Source:** `gemsuper.c:309-311` → `aes/gemsclib.c:40-45` (`sc_read`).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`SC_PATH`) | out buffer that receives the current scrap-directory path |
| `int_out[0]` | **always `1` (`TRUE`)** |

`strcpy(pscrap, D.g_scrap)` — copies the stored scrap path into the caller's
buffer and returns `TRUE`. **For Atari-TOS compatibility it no longer checks
whether a path is set** (`gemsclib.c:35-37`): if none was ever `scrp_write`n,
`D.g_scrap` is empty and an empty string is returned — but the call still
returns `1`.

> **Correction vs. the previous spec.** `scrp_read` **always returns 1**, not
> "1, or 0 if unset" (`gemsclib.c:44`). See D-054.

### `scrp_write` (81) — `sc_write`

**Source:** `gemsuper.c:312-314` → `aes/gemsclib.c:57-62` (`sc_write`).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`SC_PATH`) | in: the new scrap-directory path |
| `int_out[0]` | **always `1` (`TRUE`)** |

`strcpy(D.g_scrap, pscrap)` — stores the caller's path as the scrap directory.
**It does not validate or create the directory** (again, for Atari-TOS
compatibility, `gemsclib.c:52-54`); creating the directory (if needed) is the
application's job.

> **Correction vs. the previous spec.** `scrp_write` does **not** "create it if
> needed" — it is a bare `strcpy` (`gemsclib.c:59`). See D-054.

### `scrp_clear` (82) — `sc_clear` `#if CONF_WITH_PCGEM`

**Source:** `gemsuper.c:316-317` → `aes/gemsclib.c:74-107` (`sc_clear`).
**PC-GEM extension**, gated by `CONF_WITH_PCGEM` (default **1**,
`include/config.h:1319-1321`); with it off the opcode falls to the dispatcher `default`
→ `-1` ([`_dispatcher.md`](./_dispatcher.md) §unsupported-opcode).

| Slot | Meaning |
|------|---------|
| (no `int_in`/`addr_in`) | operates on the stored `D.g_scrap` |
| `int_out[0]` | `1` (`TRUE`) on success, `0` (`FALSE`) if `D.g_scrap` is empty |

Deletes the `SCRAP.*` files in the scrap directory: appends the mask `SCRAP.*`
to `D.g_scrap`, swaps in the AES DTA (`D.g_dta`, saving/restoring the caller's),
loops `dos_sfirst(FA_SUBDIR)`/`dos_snext` deleting each match with `dos_delete`,
then trims the path back to just the directory (`gemsclib.c:81-106`). Returns
`FALSE` immediately if no scrap path is set.

---

## 2. Shell Manager (`shel_*`) — process launch / desktop

The interface between the **desktop / shell** and launched applications: how the
next program to run is chosen, its command line + environment passed, and shared
state (the `shel_get`/`shel_put` buffer) carried across launches. In EmuTOS the
shell state lives in a file-local `SHELL sh` struct and the `THEGLO` globals
`D.s_cmd` (the fully-qualified next-program name), `ad_stail` (its command tail),
`D.g_shelbuf` (the shell buffer), and `ad_envrn` (the DOS environment); the
launch loop `sh_main`/`sh_ldapp` (`gemshlib.c:592-731`) `Pexec`s each program.

### `shel_read` (120) — `sh_read`

**Source:** `gemsuper.c:380-381` → `aes/gemshlib.c:102-106` (`sh_read`).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`SH_PCMD`) | out buffer: the command that launched **this** app (fully-qualified name) |
| `addr_in[1]` (`SH_PTAIL`) | out buffer: this app's command tail (**`CMDTAILSIZE = 128` bytes**, `memcpy`) |
| `int_out[0]` | `1` (`sh_read` is `void`; `crysbind` `ret` stays `TRUE`) |

`strcpy(pcmd, D.s_cmd)` + `memcpy(ptail, ad_stail, 128)` — hands back the program
name and the 128-byte command tail the shell set when it launched this
application. The tail is the basepage-style tail (`p_cmdlin` equivalent).

**One buffer, not two (H05, 2026-09-18).** `D.s_cmd`/`ad_stail` are the very
cells `shel_write(SHW_EXEC)` writes (`strcpy(D.s_cmd, pcmd)`,
`memcpy(ad_stail, ptail, CMDTAILSIZE)`, `gemshlib.c:169-170`) and that
`sh_ldapp` fills before it runs a program (`:645`). So after an application
has itself called `shel_write(SHW_EXEC, cmd, tail)`, a `shel_read` returns
**that** `cmd`/`tail` — not the command that launched the application —
until the shell overwrites it again. TeraDesk's Single-TOS chain protocol
depends on exactly this: `shel_write(SHW_EXEC, child)` → `Pexec(child)` →
`shel_read` → `strcmp(cmd, own name)` decides whether a chain was requested
(`sdk/apps/teradesk/startprg.c:344-355`). An implementation
 that answers `shel_read` from a
separate "launch command" buffer breaks the round trip
(`TASK/H05-STATUS-DONE.MD`).

### `shel_write` (121) — `sh_write`

**Source:** `gemsuper.c:383-384` → `aes/gemshlib.c:157-196` (`sh_write`).
The AES parameters are `sh_write(doex, isgem, isover, pcmd, ptail)` — note the
signature spells the middle two `isgem`/`isover`, not the `isgr`/`iscr` of older
docs.

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`SH_DOEX`) | **mode** (`SHW_*`, table below) |
| `int_in[1]` (`SH_ISGR` → `isgem`) | see per-mode meaning |
| `int_in[2]` (`SH_ISCR` → `isover`) | see per-mode meaning — **read for every mode** (`gemsuper.c:384` hands all three words to `sh_write`), **consumed only by `SHW_RESCHNG`** (`gl_changerez = 1 + isover`, `gemshlib.c:188`); `SHW_EXEC` ignores it (`:149`), `SHW_SHUTDOWN` documents but never reads it (`:140-145`, `:177-186`) |
| `addr_in[0]` (`SH_PCMD`) | next program name (`SHW_EXEC`) |
| `addr_in[1]` (`SH_PTAIL`) | next command tail (`SHW_EXEC`; **128 bytes** copied) |
| `int_out[0]` | **always `1` (`TRUE`)** |

**`shel_write` mode flags (`doex`, `aesdefs.h:617-621`)** — the multi-purpose
selector (`gemshlib.c:127-196`):

| `doex` | Name | Effect | `isgem` (`int_in[1]`) | `isover` (`int_in[2]`) |
|------:|------|--------|-----------------------|------------------------|
| 0 | `SHW_NOEXEC`  | **exit to desktop** — run the default desktop (`EMUDESK`) next | ignored (forced GEM) | ignored |
| 1 | `SHW_EXEC`    | **run `pcmd` next** (after this app terminates) | `0` = TOS/char mode (TTP), `1` = GEM/graphic (PRG/APP) | ignored (should be 1) |
| 4 | `SHW_SHUTDOWN`† | **shut the system down** (if `can_shutdown()`) | ignored | accepted but **not read** by `sh_write` — see below |
| 5 | `SHW_RESCHNG` | **change screen resolution** — sets `gl_changerez`/`gl_nextrez`, clears `s_cmd`/tail | Setscreen video-mode value — **with a family-dependent offset, see below** | `0` = ST/TT rez family (→ `gl_changerez = 1`), `1` = Falcon rez family (→ `gl_changerez = 2`) |

† `SHW_SHUTDOWN` (4) is `#if CONF_WITH_SHUTDOWN` — **default 0** for the standard
RPi/AArch64-style build (`include/config.h:2028-2034`), so the `case SHW_SHUTDOWN` arm is
**compiled out by default**; the opcode still returns `1` but does nothing (the
`switch` has no matching `case` → falls through). `SHW_NOEXEC`/`SHW_EXEC`/
`SHW_RESCHNG` are unconditional. **The `CONSOLE_APP` launch path (the internal
classification `shel_write(SHW_EXEC, …, "EMUCON", …)` triggers) is `#if
WITH_CLI`-gated** (a Makefile-level build flag, default **1**, `Makefile:150`;
only the memory-constrained "192" target turns it off, `Makefile:534`) — with
it off, `shel_write(SHW_EXEC, …, "EMUCON", …)` still records `sh_nextapp =
CONSOLE_APP` (`sh_write` itself is unconditional), but `sh_ldapp`'s
`CONSOLE_APP` arm is compiled out (`gemshlib.c:614-622`), so the launch loop
falls through to the "normal application" path and tries `sh_find("EMUCON")`
as an ordinary program name — which fails (`EFILNF`) since no such file
exists.

> **`SHW_RESCHNG`'s `isgem` value is *not* a plain Setscreen mode number — it
> carries a `+2` offset for the ST/TT family.** Per the `sh_write` source
> comment (`gemshlib.c:131-138`): for `isover = 0` (ST/TT-style video),
> `isgem = 2 + <the value Getrez()/Setscreen() would use>`; for `isover = 1`
> (Falcon-style video), `isgem` **is** the raw Setscreen mode value, no offset.
> `sh_write` itself does not apply or remove this offset — it just stores
> `gl_nextrez = isgem` verbatim (`gemshlib.c:188-189`). The offset is added by
> the **caller** and later subtracted by the **consumer**: EmuTOS's own
> resolution-change dialog (`desk/deskmain.c:900-913`, `do_optnmenu`/`RESITEM`)
> calls `shel_write(SHW_RESCHNG, newmode, 1, …)` for a Falcon mode (no offset)
> but `shel_write(SHW_RESCHNG, newres+2, 0, …)` for an ST/TT mode (`+2` added
> by the caller); the consumer, `geminit.c`'s post-.INF resolution-apply code
> (`:775-793`), does `Setscreen(-1L, -1L, gl_nextrez-2, 0)` for the ST/TT case
> (`gl_changerez == 1`, `:782`) — subtracting the `2` back off — versus
> `Setscreen(0L, 0L, FALCON_REZ, gl_nextrez)` for the Falcon case
> (`gl_changerez == 2`, `:788`), passing `gl_nextrez` straight through. A
> NewTOS implementation of `shel_write(SHW_RESCHNG, …)` (should it ever
> support ST/TT-style multi-resolution switching) must preserve this `+2`/`-2`
> round trip exactly, or a well-formed caller's `isover = 0` request resolves
> to the wrong video mode. See D-101.
>
> **`SHW_SHUTDOWN`'s `isover` parameter is part of the documented ABI
> (`gemshlib.c:140-145`: "isover = 0 abort … / 1 partial … / 2 full") but the
> `case SHW_SHUTDOWN` implementation never reads it** (`gemshlib.c:177-186`):
> it only checks `can_shutdown()`.  Any `isover` value — including `0`
> ("abort") — behaves identically: if `can_shutdown()` is true, the shutdown
> request is recorded (unconditionally full); if false, it is silently
> dropped. "Partial" (`1`) is not currently distinguishable from "full" (`2`)
> at all — the source comment concedes this itself ("partial shutdown
> (currently the same as full)"). EmuTOS's own caller (`desk/deskmain.c:776`,
> the desktop's `QUITITEM`) always passes `isover = 2`. See D-101.

**Behaviour.** `SHW_NOEXEC`: set `D.s_cmd = "EMUDESK"`, mark the next app as the
desktop, GEM-mode. `SHW_EXEC`: copy `pcmd` into `D.s_cmd`, `memcpy` the 128-byte
tail into `ad_stail`, record GEM-vs-char from `isgem`, classify the next app
(the console name `"EMUCON"` → console, else a normal app), and for a normal app
snapshot the current drive+dir (`sh_apdir`) to restore after it exits. `isover`
is handed in but unused by this arm ("should currently always be 1",
`gemshlib.c:149`). *Cross-reference, not source:* the TOS 4.0 release notes
(`references/freemint/doc/programmer/atari/tos4rn.stg`, "Sh_wiscr") give
`sh_wiscr = 1` under `sh_wdoex = 1` the meaning "set up ARGV-style parameter
passing in the environment", advertised through `appl_getinfo(10)`'s
`ap_gout4`; EmuTOS implements neither `appl_getinfo` nor the ARGV build, so
the word has no effect here. **NewTOS** (A61): `shel_write` reads the word
through the `min(IN_LEN, I_SIZE)` clamp like the other two and retains it
(`shell::sh_isover`, a test hook) without acting on it; a caller declaring
only two words (`IN_LEN = 2`) hands over none and the manager reads 0.
`SHW_RESCHNG`: stash the target resolution, clear the command/tail. Returns
`TRUE` in every case. The actual launch happens later in the shell loop
(`sh_ldapp`) when the current app terminates.

> **Corrections vs. the previous spec.** (1) The old file listed only
> `doex 0/1` and called the flags `isgr`/`iscr` with `iscr` = "launch mode". The
> real modes are **`SHW_NOEXEC 0`, `SHW_EXEC 1`, `SHW_SHUTDOWN 4`,
> `SHW_RESCHNG 5`** (`aesdefs.h:617-621`); the second word is **`isgem`**
> (GEM/char) and the third **`isover`** (shutdown level / rez family)
> (`gemshlib.c:127-196`). (2) `shel_write` **always returns `TRUE`**. See D-054.
> (3) **Third-round correction (D-101):** the mode-flag table's `SHW_RESCHNG`
> row previously described `isgem` as a flat "Setscreen video-mode value" —
> it omitted that ST/TT-family requests (`isover = 0`) require the caller to
> add `+2` to the real Setscreen mode value (Falcon requests, `isover = 1`, do
> not); and the `SHW_SHUTDOWN` row's `isover` column reproduced the source
> comment's 0/1/2 abort/partial/full semantics without noting that the actual
> `case SHW_SHUTDOWN` code never reads `isover` at all. See D-101 for the full
> derivation (`gemshlib.c`, `geminit.c`, `desk/deskmain.c` cross-checked).

### `shel_get` (122) / `shel_put` (123) — `sh_get` / `sh_put`

**Source:** `gemsuper.c:386-390` → `aes/gemshlib.c:203-216` (`sh_get`/`sh_put`).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`SH_LEN`) | number of bytes to copy |
| `addr_in[0]` (`SH_PBUFFER` / `SH_PDATA`) | out (get) / in (put) buffer |
| `int_out[0]` | `1` (both are `void`; `crysbind` `ret` stays `TRUE`) |

`shel_get`: `memcpy(pbuffer, D.g_shelbuf, len)`. `shel_put`:
`memcpy(D.g_shelbuf, pdata, len)`. A raw byte buffer shared across launches —
the **desktop** uses it to carry its saved context (the `NEWDESK.INF`/
`EMUDESK.INF`-style desktop settings) across the app-exit → desktop-relaunch
boundary. The buffer is **`SIZE_SHELBUF = 4192` bytes** (`sysconf.h:74`, "same as
TOS 1.04→"); **neither call bounds `len`** — the caller must not exceed 4192.

> **Correction vs. the previous spec.** The shell buffer is a fixed **4192-byte**
> blob (`SIZE_SHELBUF`), not an "environment-ish" small blob, and there is **no
> length check** in `sh_get`/`sh_put` (`gemshlib.c:205,215`). See D-054.

### `shel_find` (124) — `sh_find`

**Source:** `gemsuper.c:392-393` → `aes/gemshlib.c:474-484` (`sh_find`),
`findfile` (`:406-472`). Uses the AES DTA (`D.g_dta`, saved/restored) and
`dos_sfirst(FA_RO|FA_HIDDEN|FA_SYSTEM)`.

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`SH_PATH`) | **in/out** filespec buffer, rewritten with the resolved path if found |
| `int_out[0]` | `1` if found (buffer rewritten), `0` if not found |

**The search order (contract, `gemshlib.c:389-472`)** — matches TOS 2/3/4:

1. **Application directory** (`rlr->p_appdir`): prepend it to the filename,
   `dos_sfirst`; if found, rewrite the buffer with the fully-qualified name.
2. **Current directory**: `dos_sfirst` on the spec as given; if found, leave the
   buffer unchanged.
3. **Root of the current drive**: prepend `\`; if found, rewrite with the `\`
   prefix.
4. **Each path in the AES `PATH=`** env var (found via `sh_envrn`, split on
   `;`/`,`): try each; if found, rewrite fully-qualified.
5. Not found in any → return `0`.

Wildcards in the spec are tolerated (it uses `Fsfirst`). `shel_find` is what
`rsrc_load` and program launch use to locate a file by name. It saves/restores
the caller's DTA around the search.

### `shel_envrn` (125) — `sh_envrn`

**Source:** `gemsuper.c:395-396` → `aes/gemshlib.c:327-348` (`sh_envrn`).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`SH_PATH`, a `char **`) | out: `*value` set to the char **after** the matched string, or NULL |
| `addr_in[1]` (`SH_SRCH`) | in: the search string (e.g. `"PATH="`) |
| `int_out[0]` | `1` (`sh_envrn` is `void`; `crysbind` `ret` stays `TRUE`) |

Scans the DOS environment `ad_envrn` (a run of NUL-terminated `NAME=VALUE`
strings, ended by a double-NUL) for one starting with the search string; on a
match sets `*ppath` to the first character **after** the search string (the
value), else NULL (`gemshlib.c:338-347`). The result is delivered through the
caller's `char **` in `addr_in[0]` — **not** through `addr_out`.

> **Correction vs. the previous spec.** The old file put the search string at
> `addr_in[0]` and the output at `&value`. The source is: **output pointer =
> `addr_in[0]` (`SH_PATH`, a `char **`)**, **search string = `addr_in[1]`
> (`SH_SRCH`)** (`aesdefs.h:611-612`, `gemsuper.c:396`). See D-054.

**NewTOS (ARCH03 3b, TASK/PPARENT):** `ad_envrn` is the AES PD's `p_env`
(EmuTOS `aes/gemstart.S:78-96`), so since PPARENT the environment searched is
the AES-PD stand-in's: `default_env` = `"PATH=\0X:\\\0\0"` (12 bytes, X =
the boot drive, `bios/bios.c:77-80,626-636`) in the stand-in page
(`tos-api/src/bdos/process/standin.rs`,
[`../gemdos/process/_parent-and-environment.md`](../gemdos/process/_parent-and-environment.md)).
That page is published read-only to every process (`usermem::expose`,
`PROT_PR`, ADR-023) and for an EL0 caller `*value` is the **published** user
address, which the program can read — EmuTOS hands out a pointer into
`ad_envrn`, readable on its flat machine. An in-kernel caller gets the kernel
address. With EmuTOS's environment `shel_envrn(&p, "PATH=")` points at the NUL
after `"PATH="` (the value is the next string, `"X:\"` — the TOS quirk
`sh_find` skips, `aes/gemshlib.c:448-455`; gemlib's `mt_shel_envrn` steps
over it for `"PATH="` itself, `sdk/gemlib/gemlib-0.44.0/a_shel_envrn.c:40-50`,
so a gemlib program sees `"X:\"`). Before PPARENT the AES
environment was a built-in `"PATH=C:\;"` with no EmuTOS source. (Before 3b
an EL0 program got the kernel address and crashed on reading it; probe
`measurements/measurement-2026-10-05-arch03-3b/method/tests/envrn.c`.)

### `shel_rdef` (126) / `shel_wdef` (127) — `sh_rdef` / `sh_wdef` `#if CONF_WITH_PCGEM`

**Source:** `gemsuper.c:399-404` → `aes/gemshlib.c:491-510`. **PC-GEM
extension**, gated by `CONF_WITH_PCGEM` (default **1**); off → `-1`.

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`SH_LPCMD`) | the default-startup command name (`sh.sh_desk`) |
| `addr_in[1]` (`SH_LPDIR`) | the default-startup directory (`sh.sh_cdir`) |
| `int_out[0]` | `1` (both are `void`; `crysbind` `ret` stays `TRUE`) |

`shel_rdef` reads, `shel_wdef` sets the shell's **default startup application +
directory** — the program the shell runs by default (used by EmuDesk to set the
console's working directory, `gemshlib.c:545-548`). Plain `strcpy` in each
direction into/out of the `SHELL sh` struct.

---

## Trap interface (parameter-block form)

All calls enter via the GEM trap (`svc #2`) with **selector 200 in `x0`** and an
**AESPB pointer in `x1`** — see [`_dispatcher.md`](./_dispatcher.md).

| Op | Function | `int_in` | `addr_in` | `int_out[0]` |
|---:|----------|----------|-----------|--------------|
| 80 | `scrp_read`  | — | `[0]`=path (out) | `1` |
| 81 | `scrp_write` | — | `[0]`=path (in) | `1` |
| 82 | `scrp_clear` | — | — | `1`/`0` *(PC-GEM)* |
| 120 | `shel_read`  | — | `[0]`=cmd (out), `[1]`=tail (out) | `1` |
| 121 | `shel_write` | `[0]`=doex, `[1]`=isgem, `[2]`=isover | `[0]`=cmd, `[1]`=tail | `1` |
| 122 | `shel_get`   | `[0]`=len | `[0]`=buffer (out) | `1` |
| 123 | `shel_put`   | `[0]`=len | `[0]`=data (in) | `1` |
| 124 | `shel_find`  | — | `[0]`=path (io) | `1`/`0` |
| 125 | `shel_envrn` | — | `[0]`=&value (out), `[1]`=search (in) | `1` |
| 126 | `shel_rdef`  | — | `[0]`=cmd (out), `[1]`=dir (out) | `1` *(PC-GEM)* |
| 127 | `shel_wdef`  | — | `[0]`=cmd (in), `[1]`=dir (in) | `1` *(PC-GEM)* |

---

## Inputs / Outputs summary

Inputs arrive in the AESPB arrays; there are **no `addr_out` results** in either
manager. `scrp_read`/`shel_read`/`shel_get`/`shel_rdef` write through their
`addr_in` buffers; `shel_find` rewrites `addr_in[0]` in place; `shel_envrn`
writes through the `char **` in `addr_in[0]`. Returns are `int_out[0]` only:
`scrp_read`/`scrp_write`/`shel_write` always `1`; `shel_read`/`shel_get`/
`shel_put`/`shel_envrn`/`shel_rdef`/`shel_wdef` are `void` (`1`); `scrp_clear`
and `shel_find` return `1`/`0`.

---

## Error conditions

No TOS `E*` codes surface at the `scrp_*`/`shel_*` boundary — the returns are
`1`/`0` booleans:

| Condition | Source | Return |
|-----------|--------|--------|
| `scrp_clear` with no scrap path set | `gemsclib.c:81-82` | `0` (`FALSE`) |
| `shel_find` file not found in any search location | `gemshlib.c:449-471` | `0` |
| `scrp_read`/`scrp_write`/`shel_write` | `gemsclib.c:44,61`, `gemshlib.c:195` | always `1` |
| any of 82/126/127 when `CONF_WITH_PCGEM == 0` | `gemsuper.c` default arm | `-1` (unsupported opcode) |

(Internally the launch loop `sh_ldapp` maps a load failure to `EFILNF` and
`sh_main` shows it via the internal helper `fm_error()` (`aes/gemfmlib.c:436`
— not an AES opcode; no such call as "`form_error`" exists), but that is not
returned to the `shel_write` caller — `shel_write` returned `1` when it merely
*recorded* the request, `gemshlib.c:681-688,722-723`.)

---

## Verification criteria

- **`scrp_write` then `scrp_read` round-trips:** `scrp_write("C:\CLIP\")`
  returns `1`; a following `scrp_read` copies `"C:\CLIP\"` into the buffer and
  returns `1` (`gemsclib.c:42,59`).
- **`scrp_read` before any write:** returns `1` with an **empty** string (not
  `0`) (`gemsclib.c:44`).
- **`shel_write(SHW_EXEC, isgem=1, isover, "FOO.PRG", tail)`** returns `1` and
  records `D.s_cmd = "FOO.PRG"` + the 128-byte tail as the next app, GEM-mode
  (`gemshlib.c:168-176`).
- **`shel_write(SHW_NOEXEC, …)`** returns `1` and selects the desktop
  (`D.s_cmd = "EMUDESK"`) as the next app (`gemshlib.c:162-167`).
- **`shel_write(SHW_RESCHNG, isgem=mode, isover=0)`** sets `gl_changerez = 1`
  and `gl_nextrez = mode`, clears `D.s_cmd`/tail, returns `1`
  (`gemshlib.c:187-192`) — **`mode` here must already be the target Setscreen
  value `+2`** (ST/TT family); the consumer subtracts `2` back off
  (`geminit.c:782`). For `isover=1` (Falcon family), `mode` is the raw
  Setscreen value, no offset (`geminit.c:788`). See D-101.
- **`shel_read`** fills `addr_in[0]` with `D.s_cmd` — the launching command, or
  whatever the last `shel_write(SHW_EXEC)` stored there — and `addr_in[1]`
  with exactly 128 tail bytes (`gemshlib.c:104-105`, `:169-170`).
- **`shel_write(SHW_EXEC, c, t)` then `shel_read(b1, b2)`** yields `c` in `b1`
  and `t` in `b2` — the same buffer both ways (`gemshlib.c:104,169`).
- **`shel_get`/`shel_put` round-trip:** `shel_put(data, n)` then
  `shel_get(buf, n)` yields the same `n` bytes for any `n ≤ 4192`
  (`gemshlib.c:205,215`).
- **`shel_find` app-dir hit:** with the file in the app directory,
  `shel_find("PROG.RSC")` returns `1` and rewrites the buffer to the
  fully-qualified path; a nonexistent name returns `0` and the buffer is
  unchanged in step (2) but may be partially set — verify by return value
  (`gemshlib.c:416-471`).
- **`shel_envrn` hit/miss:** `shel_envrn(&val, "PATH=")` with a `PATH=`
  environment entry sets `*val` to the character after `"PATH="`; with no such
  entry `*val = NULL` (`gemshlib.c:333-347`).
- **PC-GEM gating:** with `CONF_WITH_PCGEM = 0`, opcodes 82/126/127 return `-1`
  (`gemsuper.c` default arm).

---

## Edge cases discovered in source

1. **`scrp_read`/`scrp_write` always return 1 and never validate.** For Atari-TOS
   compatibility they are bare `strcpy`s to/from `D.g_scrap`; no directory
   check, no creation (`gemsclib.c:35-37,52-54`). An app must create the scrap
   directory itself.
2. **`shel_write` has four modes, not two.** `SHW_NOEXEC 0`, `SHW_EXEC 1`,
   `SHW_SHUTDOWN 4`, `SHW_RESCHNG 5` (`aesdefs.h:617-621`). The middle/third
   words are `isgem` (GEM vs char mode) and `isover` (shutdown level / rez
   family), not a generic "launch mode".
3. **`SHW_SHUTDOWN` is compiled out by default.** `#if CONF_WITH_SHUTDOWN`
   defaults to **0** for the generic build (`include/config.h:2028-2034`); the mode-4
   arm is absent, so `shel_write(4, …)` returns `1` but does nothing on a
   standard NewTOS-style target. A NewTOS build must decide whether to implement
   power-off (open question).
4. **`SHW_SHUTDOWN`'s `isover` parameter is dead in the current implementation
   (D-101).** It is part of the documented ABI (0/1/2 = abort/partial/full,
   `gemshlib.c:140-145`), but the `case SHW_SHUTDOWN` code never reads it
   (`gemshlib.c:177-186`) — the shutdown request is recorded unconditionally
   (gated only by `can_shutdown()`) regardless of the value passed; "partial"
   and "full" are not currently distinguished at all (source comment,
   `gemshlib.c:144`: "currently the same as full"). EmuTOS's own caller
   (`desk/deskmain.c:776`) always passes `isover = 2`.
5. **`SHW_RESCHNG`'s `isgem` carries a hidden `+2` offset for ST/TT-family
   requests (D-101).** `isover = 0` → the caller must pass `isgem = 2 +
   <Setscreen mode value>`; `isover = 1` (Falcon) → `isgem` is the raw
   Setscreen value. `sh_write` stores `gl_nextrez = isgem` verbatim
   (`gemshlib.c:188-189`); the `+2` is added by the caller and subtracted by
   the consumer (`geminit.c:782` vs. `:788`). Confirmed by EmuTOS's own
   resolution-change dialog (`desk/deskmain.c:900-913`).
6. **`shel_write(SHW_EXEC)` copies a fixed 128-byte tail** (`CMDTAILSIZE`,
   `gemshlib.c:170`) — the tail is a fixed-size buffer, not NUL-bounded copy;
   `shel_read` mirrors it (128-byte `memcpy`).
7. **The next-app launch is deferred to the shell loop.** `shel_write` only
   *records* the request in the `SHELL sh` struct; the real `Pexec` happens later
   in `sh_ldapp` when the current app calls `appl_exit`/`Pterm` and the loop in
   `sh_main` runs the recorded program (`gemshlib.c:692-731`). `shel_write` thus
   returns `1` even for a program that later fails to load.
8. **The console name is magic, and its dispatch is `#if WITH_CLI`-gated.**
   `shel_write(SHW_EXEC, …, "EMUCON", …)` classifies the next app as the
   console (CLI) rather than a normal program (`gemshlib.c:172`,
   `DEF_CONSOLE = "EMUCON"`) — this classification itself is unconditional.
   But `sh_ldapp`'s `CONSOLE_APP` dispatch arm is `#if WITH_CLI`
   (`gemshlib.c:614-622`), a Makefile-level build flag defaulting to **1**
   (`Makefile:150`; off only for the memory-constrained "192" target,
   `Makefile:534`). With `WITH_CLI` off, the classification still happens but
   the launch falls through to the "normal app" path, which tries
   `sh_find("EMUCON")` and fails (`EFILNF`).
9. **Unrecognised `doex` values are silently accepted and ignored.** The
   `switch(doex)` in `sh_write` has no `default` case; a `doex` outside
   `{0, 1, 4, 5}` matches no `case`, so `sh_write` does nothing but still
   `return TRUE` (`gemshlib.c:161-196`).
10. **`shel_get`/`shel_put` are unbounded 4192-byte `memcpy`s.** No length clamp
    — a `len > SIZE_SHELBUF` overruns `D.g_shelbuf` (`gemshlib.c:205,215`). NewTOS
    should clamp to 4192 (a safety choice that does not change well-formed
    behaviour).
11. **`shel_find`'s search order is app-dir-first.** Step 1 is the *application*
    directory (`rlr->p_appdir`), not the current directory — this is how a program
    finds resources shipped next to it even when the current directory differs
    (`gemshlib.c:416-427`). It saves/restores the caller's DTA.
12. **`shel_envrn`'s output goes through `addr_in[0]` (a `char **`)** and its
    search string is `addr_in[1]` — the two are the reverse of what the old spec
    implied (`aesdefs.h:611-612`).
13. **`shel_envrn` returns a pointer *into* the environment**, not a copy — the
    value string lives in `ad_envrn` and is only valid while the environment is
    (`gemshlib.c:342`).
14. **`scrp_clear` uses and restores the AES DTA.** It swaps in `D.g_dta` for the
    `Fsfirst`/`Fsnext`/`Fdelete` scan and restores the caller's DTA afterward
    (`gemsclib.c:90-104`) — so it does not disturb a caller mid-directory-scan.

---

## The AES↔shell launch loop (reference for the mechanism; see `shell-loop.md`
## for the process-identity contract)

`shel_write(SHW_NOEXEC)` is how an application **quits to the desktop**: the AES
records "run the desktop next", and when the app calls `appl_exit`/`Pterm`, the
shell loop `sh_main` (`gemshlib.c:692-731`) runs whatever `shel_write` selected —
the desktop (`sh_ldapp` → `aes_run_rom_program(deskstart)`), the console
(`coma_start`), or a normal app (`dos_exec(PE_LOADGO, D.s_cmd, ad_stail,
ad_envrn)`). After a normal app exits, the loop defaults back to the desktop.
The loop's *internal shape* — the `SHELL` struct, the alpha↔graphic mode switch
mechanics (`sh_chgrf`/`sh_tographic`/`sh_toalpha`), `aes_run_rom_program`'s
nested-`Pexec` trick, the `wind_new`/DTA cleanup call sites — is **reference**:
NewTOS reimplements it on its Phase-06 process model + `Pexec`/`Pterm`.

**Contract** is broader than just `shel_write`'s recorded state: it also
includes what `sh_ldapp` does to process identity on every launch —
**[`docs/api/aes/shell-loop.md`](./shell-loop.md)** is the full extraction
(AESPD numbering, `p_nameit`/`p_setappdir` applied to the one persistent
"dos appl" descriptor, the `ap_private`/`gem.rsc` chain, the `#Q`/`#E`/`#Z`
`EMUDESK.INF` processing, the `app_save`/`app_start` state-handover buffer).
`shel_write` records the next program + tail + mode into `D.s_cmd`/`ad_stail`;
`shel_read` returns those same cells (this app's launch command + tail until
the app overwrites them with its own `shel_write`); the desktop is re-run on app exit; **and** the
launched app's `appl_init()` observably returns `ap_id == 0` and
`ap_private == 0x1143` (default) — see `shell-loop.md` for the citations.

---

## Dependencies

### Requires (must be initialized / functional)

- **The process model + `Pexec`/`Pterm`** (Phase 06 / GEMDOS) — `shel_write`
  selects the next image and the shell loop `Pexec`s it; `shel_read` reads the
  launch command tail the loader populated.
- **GEMDOS file/directory I/O** — `shel_find` uses `Fsfirst` (`dos_sfirst`);
  `scrp_clear` uses `Fsfirst`/`Fsnext`/`Fdelete`; `shel_find`/`scrp_clear` use
  the AES DTA.
- **The DOS environment** `ad_envrn` — `shel_envrn` and `shel_find`'s `PATH=`
  step read it (populated at AES start-up / by the loader).
- **The desktop / CLI images** (`EMUDESK`/`EMUCON`) for the launch loop — the
  shell loop's targets. In NewTOS these are the native desktop and shell.

### Used by

- **`rsrc_load`** ([`resource-fileselector.md`](./resource-fileselector.md))
  calls `shel_find` to locate the `.RSC` file.
- **The desktop** (`desk/`) is the canonical `shel_write` client: it loops
  `shel_write(SHW_EXEC, …)` to launch apps and re-runs itself on their exit
  (`SHW_NOEXEC`). Implementing `shel_*` is the precondition for a real desktop.
- **Every GEM application** uses `shel_write(SHW_NOEXEC)` to quit to the desktop
  and (optionally) `shel_read`/`shel_envrn`/`scrp_*` for cut/paste + startup.

---

## Related data structures

- **Scrap directory string** — `D.g_scrap` (`LEN_ZPATH = 114` bytes,
  `gemlib.h:120`), a plain path string. The **file-based clipboard convention**
  (`SCRAP.*` files in that directory) is the contract; the storage is reference.
- **`shel_write` mode-flag table** — `SHW_NOEXEC 0`/`SHW_EXEC 1`/`SHW_SHUTDOWN 4`/
  `SHW_RESCHNG 5` (`aesdefs.h:617-621`) with the `isgem`/`isover` meanings above.
  Contract.
- **Command + tail** — `D.s_cmd` (`MAXPATHLEN` fully-qualified name) + `ad_stail`
  (`CMDTAILSIZE = 128` bytes) (`gemlib.h:97,122`). `shel_read`/`shel_write`
  transfer these; the 128-byte tail size is contract.
- **Shell buffer** — `D.g_shelbuf` (`SIZE_SHELBUF = 4192` bytes,
  `gemlib.h:129`, `sysconf.h:74`). `shel_get`/`shel_put` transfer it; the
  4192-byte size is contract.
- **DOS environment** — `ad_envrn` (double-NUL-terminated `NAME=VALUE` run).
  `shel_envrn` parses it; the format is the standard DOS/TOS environment.

---

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). Cross-check of the `scrp_*`/`shel_*`
  opcode numbers, the `shel_write` mode flags + `isgem`/`isover` meanings, the
  `shel_find` search order, the `shel_envrn` `addr_in[0]`/`[1]` slot assignment,
  and the `CMDTAILSIZE`/`SIZE_SHELBUF` sizes against Compendium/Profibuch is
  **deferred** until the PDFs are available. EmuTOS `1a7484ab` is authoritative
  per CLAUDE.md Rule 1. (Compendium documents `shel_write` modes 0/1 and the
  PC-GEM 4/5 extensions; this build matches — verification deferred.) See D-054.

### AES managers

- [`resource-fileselector.md`](./resource-fileselector.md) — `rsrc_load` calls
  `shel_find`; the file selector shares the GEMDOS file/directory model.
- [`application.md`](./application.md) — `appl_exit`/`Pterm` bracket the app life
  the shell loop launches; `AP_OPEN` cleanup on a missing `appl_exit`
  (`gemshlib.c:650-657`).
- [`_dispatcher.md`](./_dispatcher.md) — selector-200 call model; the PC-GEM
  gating of 82/126/127 and the unsupported-opcode `-1` path.
- [`_state.md`](./_state.md) §6c — `D.g_scrap`, `D.g_shelbuf`, `D.s_cmd`/
  `ad_stail`, `ad_envrn`.

### GEMDOS

- `Pexec`/`Pterm` (the launch loop), `Fsfirst`/`Fsnext`/`Fdelete` (`shel_find`/
  `scrp_clear`), the DTA, the basepage command tail (`p_cmdlin` ≈ `ad_stail`),
  the environment block (`ad_envrn`).

---

## Configuration / version notes

- **`CONF_WITH_PCGEM`** (default **1**, `include/config.h:1319-1321`) gates the PC-GEM
  functions `scrp_clear` (82), `shel_rdef` (126), `shel_wdef` (127). With it off,
  those three opcodes return `-1`. `scrp_read`/`scrp_write` and
  `shel_read`/`shel_write`/`shel_get`/`shel_put`/`shel_find`/`shel_envrn` are
  unconditional.
- **`CONF_WITH_SHUTDOWN`** (default **0** for the generic build,
  `include/config.h:2028-2034`) gates the `SHW_SHUTDOWN` (4) arm of `shel_write`. Off →
  the mode is a silent no-op (returns `1`). On (native-feature/Amiga/Lisa builds)
  → it powers off via `can_shutdown()`/`shutdown()`.
- `scrp_read`/`scrp_write`, `shel_read`/`shel_write`/`shel_get`/`shel_put`/
  `shel_find`/`shel_envrn` (80/81, 120–125) are stable across all EmuTOS builds
  and TOS versions (the original TOS 1.x scrap/shell set); 82/126/127 (PC-GEM)
  and the 4/5 `shel_write` modes are later additions.

---

## AES extraction complete

**This file completes the AES manager set and the full API spec
re-derivation.** All 11 AES managers (~80 functions) are specified across
Batches 0–7: [`_dispatcher.md`](./_dispatcher.md), [`_state.md`](./_state.md),
[`application.md`](./application.md), [`event.md`](./event.md),
[`menu.md`](./menu.md), [`object-form.md`](./object-form.md),
[`window-graphics.md`](./window-graphics.md),
[`resource-fileselector.md`](./resource-fileselector.md), and this file. With the
AES done, the GEMDOS / BIOS / XBIOS / VDI / AES specification extraction (Phase
00) is complete. Implementation follows in the later phases, gated on the VDI.

---

## NewTOS implementation notes

- **Shell Manager is the bridge to the NewTOS process model** (Phase 06 +
  GEMDOS `Pexec`/`Pterm`, implemented): `shel_write` selects the next image, the
  shell loop `Pexec`s it. This formalises the run-a-program logic already
  prototyped in [kernel/src/shell.rs](../../../kernel/src/shell.rs) with the
  desktop in the loop. Preserve the four `shel_write` modes, the `isgem`/`isover`
  semantics, and the app-dir-first `shel_find` order exactly; the launch loop
  itself (mode switching, `aes_run_rom_program`) is NewTOS's own.
- `shel_find` reuses GEMDOS path search; `shel_read` reuses the `Pexec`
  command tail (basepage `p_cmdlin` ≈ `ad_stail`) the loader populated.
- **The desktop** (`EMUDESK`-equivalent) is the canonical `shel_write` client;
  implementing `shel_*` is the precondition for running the NewTOS desktop.
- **Scrap Manager is trivial given the file system** (Phase 09): a stored path
  string (`scrp_read`/`scrp_write`) plus `SCRAP.*` file ops via GEMDOS
  (`scrp_clear`). Low effort — do it early for cut/paste demos. Keep the
  always-return-1 / no-validation contract of `scrp_read`/`scrp_write`.
- **`shel_get`/`shel_put`** back the shell buffer with a kernel-held 4192-byte
  blob first, persisting it to disk (the desktop settings file) later. **Clamp
  `len` to 4192** — an ADR-worthy safety choice over EmuTOS's unbounded `memcpy`
  that does not change observable behaviour for well-formed callers.
- **`SHW_SHUTDOWN`** power-off is an open question: EmuTOS compiles it out for
  generic builds; NewTOS may implement a real RPi4 power-off later (out of the
  compatibility contract, since the mode is optional). If implemented, `isover`
  need not be honoured beyond gating (EmuTOS's own implementation does not
  read it either, D-101) — a NewTOS deviation that *adds* real partial/full
  distinction would still satisfy the contract (no well-formed EmuTOS-observed
  behaviour to preserve there).
- **`SHW_RESCHNG`'s *hardware* half** (an actual `Setscreen`/`Vsetmode`
  video-mode switch) is out of scope (no ST/TT-vs-Falcon dual video hardware
  on RPi4/AArch64 — the whole `Setscreen`-family distinction is
  EmuTOS-hardware reference, ADR-017 rev 2, `kernel/src/drivers/mod.rs:17-33`).
  **If a future NewTOS resolution-switch facility reuses opcode 121/mode 5
  for compatibility, the `+2` ST/TT offset convention on `isgem` (D-101) is
  contract**, not internal — a real GEM application computing `isgem` per
  the documented Getrez()/Setscreen() convention expects it honoured
  symmetrically by whatever consumes `gl_nextrez`.
- **`SHW_RESCHNG`'s *protocol* half is in scope and implemented (A48/A-5 +
  A106).** `sh_write`'s `SHW_RESCHNG` arm (`gemshlib.c:187-191`) sets
  `gl_changerez`/`gl_nextrez` **unconditionally** — it does *not* call
  `check_moderez` (that validation belongs only to the boot-time
  `.INF`-parse producer, `process_inf1`, `geminit.c:343-392`; the "avoid a
  no-op restart" filter for the interactive case lives in the *caller*,
  EmuTOS's own resolution dialog, `desk/deskrez.c:213-215`, not in
  `sh_write`). The consumer side (`sh_main`'s loop, `gemshlib.c:727`, plus
  `gem_main`/`gemstart.S`'s restart, `geminit.c:753-899`,
  `gemstart.S:86-124`) closes the whole AES session — every running
  program, not just the video mode — and restarts it fresh.
  **NewTOS:** `shel_write(SHW_RESCHNG, isgem, isover)` records the request
  exactly as `sh_write` does — `gl_changerez = 1 + isover`, `gl_nextrez =
  isgem`, unconditionally, then clears `s_cmd`/`s_tail` and returns 1
  (`tos-api/src/aes/shell.rs`, accessors `gl_changerez()`/`gl_nextrez()`;
  A48 tier 1, 2026-09-21). Since A106 (2026-09-22) the shell loop carries
  the `!gl_changerez` term (`tos_api::aes::shell::sh_main_continue`), so a
  program that terminates with the request pending ends the session instead
  of handing over to the next program, and the kernel's session loop
  (`kernel/src/lib.rs`, NewTOS's `aes_restart`) re-initialises the whole
  AES (`tos_api::aes::reinit`, `kernel::desk::reset`, `dos_sdrv(bootdev)`)
  and runs `process_inf2` + `sh_main` again — so the `.INF`'s `#Z` program
  or the built-in desktop starts afresh, exactly as after a cold boot.
  The **physical** mode switch stays out of scope (`Setscreen`/`Vsetmode`,
  `geminit.c:778-793`; one fixed mode, ADR-017 rev 2, D-101): the request
  is recorded and reported, never honoured as a geometry change. Design in
  [`docs/analysis/a48-shw-reschng-design-2026-09-21.md`](../../analysis/a48-shw-reschng-design-2026-09-21.md);
  see [`shell-loop.md`](./shell-loop.md) §7 for the loop and the restart.
- Pointers in `addr_in[]` (paths, buffers, the environment search string, the
  `char **` for `shel_envrn`) are **user (application) pointers** — apply the
  [`_dispatcher.md`](./_dispatcher.md) user/kernel boundary handling; the
  in/out buffers (`scrp_read`, `shel_read`/`get`, `shel_find`) must be flushed
  back to user memory.

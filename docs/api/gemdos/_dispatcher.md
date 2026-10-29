# GEMDOS Trap-1 Dispatcher

> **Phase 00 deliverable D2 (Trap 1).**
> Source: EmuTOS @ `1a7484ab`.
>
> The GEMDOS dispatcher is two-stage: an assembly trampoline
> (`bdos/rwa.S:_enter`) saves caller state into the basepage, then calls
> the C function `osif()` (`bdos/bdosmain.c:498`) which performs
> table-driven dispatch.

---

## 1. Vector installation

`bdos/bdosmain.c:339-352` (`osinit_before_xmaddalt()`):

```c
Setexc(0x21, (long)enter);                 /* take over TRAP #1 */
old_trap2 = (PFVOID) Setexc(0x22, bdos_trap2);
```

- Trap 1 vector (CPU exception number 33 → byte address 0x84) →
  `_enter` in `rwa.S`.
- Trap 2 vector (number 34) → `_bdos_trap2` to intercept `Pterm0()` via
  the undocumented "trap-2 with d0=0" CP/M-68K compatibility path.
  Other trap-2 calls fall through to the previous handler.

Citation: `bdos/rwa.S:159` (`_enter:` label) and `bdos/rwa.S:145`
(`_bdos_trap2:` label).

---

## 2. Stack frame on entry

The m68k CPU pushes onto the supervisor stack:

```
+6 (long-frame only): format/vector word
+2: PC of caller (return address, LONG)
 0: SR of caller (WORD)
```

For a 68000 the format-word is absent (4-byte exception frame).
For 68010+ EmuTOS uses `_longframe` to detect the longer frame
(`bios/vectors.S:1287`-onward — but this affects BIOS/XBIOS; trap 1
takes a different path).

**Above the exception frame**, on the caller's stack (USP if user
mode, SSP if supervisor):

```
+(2*N): arg N (word, may be the high half of a long)
…
+2: arg 1 (word)
 0: function ID (word)
```

The dispatcher reads function ID from `pw[0]` and successive args from
`pw[1]`, `pw[2]`, … as 16-bit words. LONG/pointer arguments occupy two
consecutive slots and are reassembled by `*(long*)&pw[i]` or
`*((char**)&pw[i])` casts.

---

## 3. `_enter` trampoline (`bdos/rwa.S:159`)

Responsibilities:

1. Patch the supervisor exception frame for buggy 68000 stack
   pushes (`fix_trap` at top of `_enter`).
2. Detect undocumented function 0x20 ("toggle/inquire processor/stack
   state", aka `Super`):
   - If invoked from user mode and the caller's USP top word is 0x20,
     branch to `x20_usr` and bypass the C dispatcher.
   - Same for supervisor-mode entry (`x20_sup`).
   This is the only function not handled by `osif()` and therefore not
   present in `funcs[]`.
3. For all other calls:
   - Save D0 + A3-A6 into `P_REGSAVE(a6)` of `_run` (current basepage).
   - Save the caller's stack pointer into `P_A7SAVE(a6)`.
   - Switch to the BDOS internal stack (`fstrt`, 4000 bytes BSS at
     `bdos/rwa.S:fstrt`).
   - Build a pointer to the caller's argument area in A0.
   - Call `osif(a0)` (`callos:` at `bdos/rwa.S:206`).
4. Fall through to `_gouser` to restore caller state and `rte`.

Stack-pointer convention preserved across the call so that programs
can call into BDOS from either mode.

---

## 4. `osif()` C dispatcher (`bdos/bdosmain.c:431`)

```c
long osif(short *pw);
long osif(short *pw)
{
    int fn = pw[0];
    if (fn > MAX_FNCALL)               // 0x57 in this build
        return EINVFN;                 // -32

    if (setjmp(errbuf)) { ... }        // hard-error recovery

    f = &funcs[fn];                    // look up FND record
    typ = f->stdio_typ;

    /* (1) standard-handle redirection: see Step 5 below */

    /* (2) Fopen/Fcreate special-name interception (CON: AUX: PRN:) */

    /* (3) function-by-arity dispatch: */
    switch (f->wparms) {
        case 0: rc = f->fncall.v(); break;
        case 1: rc = f->fncall.w(pw[1]); break;
        case 2: rc = f->fncall.ww(pw[1], pw[2]); break;
        case 3: rc = f->fncall.www(pw[1], pw[2], pw[3]); break;
        case 4: rc = f->fncall.wwww(pw[1]…pw[4]); break;
        case 5: rc = f->fncall.wwwww(pw[1]…pw[5]); break;
        case 7: rc = f->fncall.wwwwwww(pw[1]…pw[7]); break;
        default: rc = EINTRN;
    }
    return rc;
}
```

`MAX_FNCALL` is `ARRAY_SIZE(funcs) - 1` → highest entry in this build is
`0x57` (`xgsdtof`), giving a table size of 88.

Unimplemented IDs map to `ni()` (`bdos/bdosmain.c:316`) which returns
`EINVFN` (-32).

> **Contrast with BIOS/XBIOS — verify before "fixing" (2026-07-09
> re-validation, GEMDOS Batch 1 Task 1).** BIOS/XBIOS's `xbios_unimpl`
> returns the **function number itself**, not `EINVFN` — for both an
> out-of-range ID and an in-range-but-unimplemented table slot (D-008,
> D-039). **GEMDOS does the opposite in both cases**, confirmed directly
> against this source revision:
> - **Out-of-range** (`fn > MAX_FNCALL`, `bdos/bdosmain.c:444-445`): `return
>   EINVFN;` — a literal `-32`, not `fn`.
> - **In-range, unimplemented slot** (every `funcs[]` entry built with the
>   `NI` macro, e.g. `bdos/bdosmain.c:176-177,181,206-208,213-217,...,279-284`):
>   `f->fncall.v` points at `ni()` (`bdos/bdosmain.c:307-310`), which is
>   `static long ni(void) { return EINVFN; }` — again `-32`, not `fn`.
>
> Both dispatchers are internally *self-consistent* (out-of-range and
> in-range-unimplemented agree with each other within each trap), but the
> **two traps disagree with each other**: GEMDOS ⇒ `EINVFN` always; BIOS/XBIOS
> ⇒ function number always. This is a real per-trap difference in EmuTOS, not
> a spec inconsistency to reconcile. **No correction needed here** — this
> file already documented `EINVFN` correctly; this note exists so a future
> audit does not "fix" GEMDOS toward the BIOS/XBIOS convention (or vice
> versa) by analogy instead of by re-reading each trap's source.

---

## 5. Standard-handle (`p_uft[]`) interception

(Re-validated against EmuTOS source 2026-10-18, TASK/PROC04; the earlier
pseudo-code listed `case 6` as live and glossed the byte/EOF details.)

`osif(short *pw)` (`bdos/bdosmain.c:431`; `pw` is `short *`, so every handle
and every WORD argument is **signed 16-bit**) has **two** handle-aware
blocks, both keyed on `funcs[fn].stdio_typ`:

**Block A — character functions redirected to a file**
(`bdos/bdosmain.c:498-568`). Guard: `typ && fn && ((fn < 12) || (16 <= fn <= 19))`.
`typ & 0x7f` is the *standard handle*: `0x80`→0 (stdin), `0x81`→1 (stdout),
`0x82`→2 (aux), `0x83`→3 (prn) (`funcs[]` `:150-197`):

| fn | function | `stdio_typ` | std handle | block A active? |
|---:|----------|------------:|-----------:|-----------------|
| 0x01 | Cconin   | 0x80 | 0 | yes |
| 0x02 | Cconout  | 0x81 | 1 | yes |
| 0x03 | Cauxin   | 0x82 | 2 | yes |
| 0x04 | Cauxout  | 0x82 | 2 | yes |
| 0x05 | Cprnout  | 0x83 | 3 | yes |
| 0x06 | Crawio   | **0**| — | **no** (`:161`; the `case 6`/`rawout` arm below is unreachable for fn 6, only `rawout` is reached by fn 2/4/5) |
| 0x07 | Crawcin  | 0x80 | 0 | yes |
| 0x08 | Cnecin   | 0x80 | 0 | yes |
| 0x09 | Cconws   | 0x81 | 1 | yes |
| 0x0A | Cconrs   | 0x80 | 0 | yes |
| 0x0B | Cconis   | 0x80 | 0 | yes |
| 0x10 | Cconos   | 0x81 | 1 | yes |
| 0x11 | Cprnos   | 0x83 | 3 | yes |
| 0x12 | Cauxis   | 0x82 | 2 | yes |
| 0x13 | Cauxos   | 0x82 | 2 | yes |

```c
h = run->p_uft[typ & 0x7f];                 /* :500 */
if (h > 0) {                                /* :501  only a file handle (>= 6) */
    switch (fn) {
    case 1: case 3: case 7: case 8:         /* read ONE byte      :509-519 */
        if (xread(h, 1L, &ctmp) != 1L) return EOF_INDICATOR;   /* 0x0000ff1a, :69 */
        return ctmp;                        /* ctmp is plain `char` -> SIGN-extended */
    case 2: case 4: case 5:                 /* :521-528 */
    rawout: xwrite(h, 1L, ((char*)&pw[1]) + 1);   /* low byte of the WORD arg */
        return 0;                           /* dummy: NOT the Cconout "1" */
    case 9:  xwrite(h, strlen(pb2), pb2); return 0;            /* :530-533, raw */
    case 10: /* Cconrs: read up to max bytes one by one; stop at 0x0d and EAT the
                following byte (the LF); store count in buf[1]; return 0   :535-553 */
    case 11: case 18: if (eof(h)) return 0L; FALLTHROUGH;      /* :555-558 */
    case 16: case 17: case 19: return -1L;                     /* :561-564 */
    }
}
typ = 0;                                    /* :568 not redirected: normal dispatch */
```

Notes (cited = read in source; *inferred* = reasoning from it):

- Only `p_uft[x] > 0` triggers block A. A **negative** `p_uft` (BIOS handle,
  default or `Fforce(std, -n)`) is **not** touched here — the function body in
  `bdos/console.c` then uses `HXFORM(run->p_uft[x])` = `3 + p_uft[x]` as the
  BIOS device (`bdos/console.c:161,170,179,188,197,281,333,342,377,401,419,434,
  447,451,461,535`; `console.h:36`). Both mechanisms are needed for a faithful
  redirect.
- `Crawio` (fn 6, `stdio_typ 0`) is never answered by block A. With
  `p_uft[0]`/`p_uft[1]` forced to a *file* handle `h > 0`, `xrawio`
  (`bdos/console.c:441-453`) still runs and calls `getch`/`Bconout` with BIOS
  device `3 + h` — no file I/O. (*inferred*: result is BIOS-table dependent
  for `3+h >= 6`, `bios/bios.c:991-1011` Bconmap branch; `constat` returns 0
  for `h` outside 1..2, `bdos/console.c:147-148`.)
- EOF/error on a redirected read returns `EOF_INDICATOR` (`0x0000ff1a`), the
  comment says "same way as MiNT" (`:514-516`). A successful read returns the
  byte **sign-extended** (`char ctmp`, `:433,519`; the EmuTOS `Makefile` has
  no `-funsigned-char` — *inferred* from C semantics: bytes `>= 0x80` come
  back as `0xFFFFFFxx`).
- Write status: `Cconis`/`Cauxis` on a file return `0` only at EOF
  (`eof(h)` is non-zero for EOF *and* for error, `fsio.c:288-300`), else `-1`.
- Block A does no tab expansion, no `conbrk`, no Ctrl-C handling, no
  line editing — it is raw byte I/O on the file.
- `case 10` reads `max = *pb2++` into an `int` from a plain (signed) `char`
  and loops `while (max--)` (`:537-539`): a `max` of 128..255 is negative and
  only CR or EOF stops the loop, past the caller's record. **NewTOS reads `max`
  unsigned (0..255) and stops there** — named deviation, user decision
  2026-10-18 (TASK/PROC04, `docs/COMPATIBILITY.md` "Sanctioned deviations");
  identical for `max` <= 127.

**Block B — handle-based file functions** (`bdos/bdosmain.c:571-643`;
`typ & 0x80`: Fread 0x82 handle `pw[1]`, Fwrite 0x82 `pw[1]`, Fseek 0x81
`pw[3]`). Resolution of the signed handle, `Fread`/`Fwrite` on a character
device and the `-4`/`0` error cases are specified in `file/42-fseek.md`,
`file/3f-fread.md`, `file/40-fwrite.md` and are not repeated here.

Consequence: **applications that redirect stdin/stdout via `Fforce` see
`Cconin`/`Cconout`/`Cconws`/`Cconrs`/… read from / write to the file** —
EmuTOS contract. NewTOS does this since 2026-10-18 (TASK/PROC04 point 1,
`_state.md` §2 "NewTOS deviations" item 1: `console::redirect_to_file`
before the function table, `console::std_dev` = `HXFORM(run->p_uft[n])` in
the bodies).

---

## 6. Function table (`funcs[]`, `bdos/bdosmain.c:142-289`)

Each entry is:

```c
typedef struct {
    union {
        long (*v)(void);
        long (*w)(short);
        long (*ww)(short,short);
        ...
    } fncall;
    UBYTE stdio_typ;   /* 0, 0x80, 0x81, 0x82, or 0x83 */
    UBYTE wparms;      /* number of WORD-sized parameter slots */
} FND;
```

`stdio_typ` bits:
- `0x80` = function consumes a standard handle.
- low bits: `0` = stdprn, `1` = stdout, `2` = stdaux, `3` = stdprn (written
  as `0x80|0,0x80|1,…`). Concretely: 0x80 stdin, 0x81 stdout, 0x82 stdaux,
  0x83 stdprn.
- `0` = no standard-handle interception.

`wparms` is in WORDs; LONG/ptr count as 2 WORDs.

The full table (only entries that resolve to functions, with build
configuration `CONF_WITH_TT` = 1 baseline assumed; flagged as cfg where
relevant):

| ID    | EmuTOS func   | TOS name    | typ  | wparms | Notes                       |
|------:|---------------|-------------|------|-------:|-----------------------------|
| 0x00  | `x0term`      | Pterm0      | 0    |   0    |                             |
| 0x01  | `xconin`      | Cconin      | 0x80 |   0    |                             |
| 0x02  | `xconout`     | Cconout     | 0x81 |   1    |                             |
| 0x03  | `xauxin`      | Cauxin      | 0x82 |   0    |                             |
| 0x04  | `xauxout`     | Cauxout     | 0x82 |   1    |                             |
| 0x05  | `xprtout`     | Cprnout     | 0x83 |   1    |                             |
| 0x06  | `xrawio`      | Crawio      | 0    |   1    |                             |
| 0x07  | `xrawcin`     | Crawcin     | 0x80 |   0    |                             |
| 0x08  | `xnecin`      | Cnecin      | 0x80 |   0    |                             |
| 0x09  | `xconws`      | Cconws      | 0x81 |   2    | (long ptr)                  |
| 0x0A  | `xconrs`      | Cconrs      | 0x80 |   2    | (long ptr)                  |
| 0x0B  | `xconstat`    | Cconis      | 0x80 |   0    |                             |
| 0x0E  | `xsetdrv`     | Dsetdrv     | 0    |   1    |                             |
| 0x10  | `xconostat`   | Cconos      | 0x81 |   0    |                             |
| 0x11  | `xprtostat`   | Cprnos      | 0x83 |   0    |                             |
| 0x12  | `xauxistat`   | Cauxis      | 0x82 |   0    |                             |
| 0x13  | `xauxostat`   | Cauxos      | 0x82 |   0    |                             |
| 0x14  | `xmaddalt`    | Maddalt     | 0    |   4    | cfg `CONF_WITH_ALT_RAM`     |
| 0x15  | `srealloc`    | Srealloc    | 0    |   2    | cfg `CONF_WITH_VIDEL`       |
| 0x19  | `xgetdrv`     | Dgetdrv     | 0    |   0    |                             |
| 0x1A  | `xsetdta`     | Fsetdta     | 0    |   2    |                             |
| 0x2A  | `xgetdate`    | Tgetdate    | 0    |   0    |                             |
| 0x2B  | `xsetdate`    | Tsetdate    | 0    |   1    |                             |
| 0x2C  | `xgettime`    | Tgettime    | 0    |   0    |                             |
| 0x2D  | `xsettime`    | Tsettime    | 0    |   1    |                             |
| 0x2F  | `xgetdta`     | Fgetdta     | 0    |   0    |                             |
| 0x30  | `xgetver`     | Sversion    | 0    |   0    |                             |
| 0x31  | `xtermres`    | Ptermres    | 0    |   3    |                             |
| 0x36  | `xgetfree`    | Dfree       | 0    |   3    |                             |
| 0x39  | `xmkdir`      | Dcreate     | 0    |   2    |                             |
| 0x3A  | `xrmdir`      | Ddelete     | 0    |   2    |                             |
| 0x3B  | `xchdir`      | Dsetpath    | 0    |   2    |                             |
| 0x3C  | `xcreat`      | Fcreate     | 0    |   3    |                             |
| 0x3D  | `xopen`       | Fopen       | 0    |   3    |                             |
| 0x3E  | `xclose`      | Fclose      | 0    |   1    |                             |
| 0x3F  | `xread`       | Fread       | 0x82 |   5    | (typ flag for redirection)  |
| 0x40  | `xwrite`      | Fwrite      | 0x82 |   5    |                             |
| 0x41  | `xunlink`     | Fdelete     | 0    |   2    |                             |
| 0x42  | `xlseek`      | Fseek       | 0x81 |   4    |                             |
| 0x43  | `xchmod`      | Fattrib     | 0    |   4    |                             |
| 0x44  | `xmxalloc`    | Mxalloc     | 0    |   3    |                             |
| 0x45  | `xdup`        | Fdup        | 0    |   1    |                             |
| 0x46  | `xforce`      | Fforce      | 0    |   2    |                             |
| 0x47  | `xgetdir`     | Dgetpath    | 0    |   3    |                             |
| 0x48  | `xmalloc`     | Malloc      | 0    |   2    |                             |
| 0x49  | `xmfree`      | Mfree       | 0    |   2    |                             |
| 0x4A  | `xsetblk`     | Mshrink     | 0    |   5    |                             |
| 0x4B  | `xexec`       | Pexec       | 0    |   7    |                             |
| 0x4C  | `xterm`       | Pterm       | 0    |   1    |                             |
| 0x4E  | `xsfirst`     | Fsfirst     | 0    |   3    |                             |
| 0x4F  | `xsnext`      | Fsnext      | 0    |   0    |                             |
| 0x56  | `xrename`     | Frename     | 0    |   5    |                             |
| 0x57  | `xgsdtof`     | Fdatime     | 0    |   4    |                             |

All other entries (0x0C, 0x0D, 0x0F, 0x16-0x18, 0x1B-0x29, 0x2E, 0x32-0x35,
0x37, 0x38, 0x48-0x4D minus 0x48/0x49/0x4A, 0x4D, 0x50-0x55) → `ni` →
`EINVFN`.

`Super` (0x20) is **not** in `funcs[]` because it's intercepted by
`_enter` itself. Full contract spec:
[`process/20-super.md`](process/20-super.md).

---

## 7. Special-name interception (`Fopen`/`Fcreate`)

Before dispatching `xopen` (0x3D) or `xcreat` (0x3C), `osif()` checks
the path against a small table (`bdos/bdosmain.c:96-104`):

```c
{ "CON:", 0x0000ffffL },  { "con:", 0x0000ffffL },
{ "AUX:", 0x0000fffeL },  { "aux:", 0x0000fffeL },
{ "PRN:", 0x0000fffdL },  { "prn:", 0x0000fffdL },
```

If matched, `osif()` returns the synthetic device handle directly
without calling `xopen`/`xcreat`. Mixed-case names (`Con:`) are not
recognised.

---

## 8. Hard-error recovery (`setjmp`/`longjmp`)

`osif()` runs the function under `setjmp(errbuf)`. Any code path may
`longjmp(errbuf, errcode)` from within `_iorec` / FAT code. On longjmp:

- If `errcode == E_CHNG` (media change), the dispatcher rebuilds the
  drive's BPB + frees DND tree and OFDs, then `goto restrt` to retry
  the original function. (`bdos/bdosmain.c:454-489`).
- Otherwise, the BCBs for `errdrv` are invalidated and the error is
  returned to caller as if it were the function's return value.

For console functions this longjmp path is unreachable; only file/disk
operations use it.

**[FS20]** Where the BDOS meets a media change, what the re-login does to open
files, current directories and buffers, and how NewTOS reproduces the restart
without `longjmp`: `_state.md` §4a.
**[FS22]** NewTOS runs the same restart for the kernel's own file-system
paths (`Pexec`'s load, the kernel's file reads and directory listings, the
shell's GEMDOS calls) and bounds it: at most three restarts per call, then
`E_CHNG` (a sanctioned deviation, `docs/COMPATIBILITY.md`): `_state.md` §4a.

---

## 9. Return path

`osif()` returns a LONG. `_enter` falls through to `_gouser`
(`bdos/rwa.S:240`) which:
1. Stores the returned LONG in `P_D0SAVE(a6)`.
2. Restores caller's stack pointer + saved registers.
3. Restores SR and PC from the saved exception frame.
4. Executes `rte` (or the ColdFire-specific moral equivalent).

The caller resumes with D0 = return value; D1, D2, A0, A1, A2 are not
preserved (consistent with documented TOS calling convention).

---

## 10. Source-location summary

| Concern                        | File:line                    |
|--------------------------------|------------------------------|
| Trap-1 vector install          | `bdos/bdosmain.c:339`        |
| Asm trampoline                 | `bdos/rwa.S:159` (`_enter`)  |
| C dispatcher                   | `bdos/bdosmain.c:431`        |
| Function table                 | `bdos/bdosmain.c:142-289`    |
| FND struct                     | `bdos/bdosmain.c:117-129`    |
| Stdhandle redirection          | `bdos/bdosmain.c:498-575`    |
| Special name interception      | `bdos/bdosmain.c:646-660`    |
| `Super`/`x20` short-circuit    | `bdos/rwa.S:162-184` (detect), `:280-346` (`x20_usr`/`x20_sup`/`x20_inq`); full spec `process/20-super.md` |
| Hard-error recovery            | `bdos/bdosmain.c:454-489`    |
| Return / register restore      | `bdos/rwa.S:240` (`_gouser`) |

---

## NewTOS ABI mapping notes

(Empty during Phase 00. Filled in during Phase 02.)

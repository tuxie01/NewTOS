# `Fread` (GEMDOS 0x3F)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsio.c:440` (`xread`) and the helper
> `bdos/fsio.c:498` (`ixread`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, "actual bytes read" return, EOF-truncation,
> read-of-write-only allowed, error codes. Reference: `xrw`/buffer-cache
> cluster walk.

## EmuTOS source

**Primary location:** `bdos/fsio.c:440` — `long xread(int h, long len, void *ubufr)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x3F` =
`xread`, `stdio_typ = 0x82`, `wparms=5` (`bdos/bdosmain.c:259`). The `0x82`
flag is **inert** for `Fread`: the dispatcher's standard-handle redirection
only fires for `fn < 12` or `16 <= fn <= 19` (`bdos/bdosmain.c:498`), and
`0x3F` is outside both ranges — so `Fread` is never intercepted and reaches
`xread` directly **for file handles**. (Correction 2026-10-13: the `0x82` flag
is *not* inert overall — bit 7 sends `Fread` into the handle-resolution block
`bdos/bdosmain.c:571-643`, which runs **before** `xread` and answers
character-device handles itself. See "Character-device path" below.)

**Helpers called (file path):**
- `bdos/fsmain.c:474` — `getofd()` — handle → OFD.
- `bdos/fsio.c:498` — `ixread()` — EOF-clamp `len`, then call `xrw(0, …)`.
- `bdos/fsio.c:208` — `xrw()` — unified read/write cluster walk through the
  buffer cache; `bdos/fsio.c:57` — `usrio()` → `longjmp_rwabs` (`Rwabs`).

**Headers / type definitions used:** `bdos/fs.h` — `OFD`, `DFD`, `DMD`.

**System variables / globals read:** `run` (via `getofd`); `p->o_bytnum`,
`p->o_dfd->o_fileln`.
**System variables / globals written:** the OFD position fields (`o_bytnum`,
`o_curcl`, `o_currec`, `o_curbyt`) advance as bytes are copied.

**Build configuration affecting this function:** none.

---

## Character-device path (ADDED 2026-10-13, FS11; EmuTOS @ `1a7484ab`)

Everything in this section happens in `osif()` **before** `xread` is reached;
`xread`/`ixread` never see a handle that ends on a BIOS character device.
Line numbers: `bdos/bdosmain.c` unless noted. `console.c` = `bdos/console.c`.

### Dispatch and argument layout

`osif(short *pw)` (`:431`) sees the raw word frame: `pw[0]`=fn 0x3F,
`pw[1]`=handle (`h = pw[1]` because `typ != 0x81`, `:571-576`), `pw[2]`=high word
and `pw[3]`=low word of `len` (a LONG), `pw[4..5]`=buffer pointer (`pb = &pw[4]`,
`:604`). `pw` is `short *`, so `pw[3]` is a **signed** 16-bit value.

Handle resolution (`:578-593`) is the one documented in
[`42-fseek.md`](42-fseek.md) §"Character-device and standard handles" and is not
repeated: `h >= 6` → `sft[h-6].f_ofd`; `0 <= h < 6` → `p_uft[h]` (`> 0` → the sft
entry's `f_ofd`, else the value itself); `h < 0` → `h` itself; result `0` →
**`EIHNDL`** (`:592-593`); `numl < -3` → **`EIHNDL`** (`:601-602`); `numl > 0`
(real OFD) → leaves the block and runs `xread` (file path above). `numl` in
`-1..-3` enters the device path below. BDOS handle → BIOS handle is
`HXFORM(h) = 3+h` (`bdos/console.h:36`): CON (-1) → BIOS 2, AUX (-2) → BIOS 1,
PRN (-3) → BIOS 0.

Default standard handles (`bdos/console.c:60-67`, set by `stdhdl_init`,
`bdos/console.c:107-112`): 0 → CON, 1 → CON, 2 → AUX, 3 → PRN, 4 → CON, 5 → CON.
`Fread` does **not** check direction: `Fread(1,…)` (stdout) reads the keyboard
and `Fread(3,…)` reads from PRN. A `Fdup` of a standard handle that maps to a
device stores the negative BIOS handle in the new sft entry (`bdos/fshand.c:115-118`),
so `Fread` on that dup'ed handle (>= 6) takes the same device path; after
`Fclose` of it the slot is freed (`f_ofd = 0`, `bdos/fsopnclo.c:462-470`) →
`EIHNDL`. `Fforce(std, -1..-3)` stores the negative handle as-is
(`bdos/fshand.c:53-54`) → device path; `Fforce(std, file)` → file path.
`Fclose(std)` reverts to the default (`bdos/fsopnclo.c:450-455`).

### Device-read algorithm (`:608-620`)

For `numl` in `-1..-3`, with `dev = HXFORM(numl)` (`:615`, `:619`):

1. **`pw[2] != 0`** ("HUGE read", `:610-611`) → return **0**. Nothing is read,
   nothing is echoed. Because only the high word is tested, this also catches
   every **negative** `len`.
2. **`pw[3] == 1`** (`:613-617`) → `*buf = conin(dev)`; return **1**
   (`conin` = `bdos/console.c:384-390`: `getch(dev)` then **echo** via
   `conout(dev, (unsigned char)ch)`). The stored byte is the low byte of the
   `Bconin` long (char assignment, `:615`); scan code is dropped. The key is
   stored whatever it is: CR is stored as 0x0D (no line editing), Ctrl-C (0x03)
   is **not** treated as terminate here (`conin` has no such check; contrast
   `xconin`, `bdos/console.c:396-408`), and the call returns 1 immediately — no
   waiting for Return.
3. otherwise → `return cgets(dev, pw[3], buf)` (`:619`; `bdos/console.c:540-581`).
   `maxlen` is `pw[3]` sign-extended to `int`. Consequences (read from code,
   `bdos/console.c:546`: the loop is `for (retlen = 0; retlen < maxlen;)`):
   - `len == 0` → `maxlen == 0` → loop never entered → returns **0**, no echo.
   - `len` in `32768 .. 65535` (high word 0) → `pw[3]` negative → `maxlen < 0`
     → returns **0**, no echo. (So a device read can never exceed 32767 bytes,
     and larger requests silently yield 0.)
   - `len` in `2 .. 32767` → line read, below.

### `cgets` line editor (`bdos/console.c:540-581`)

Let `stcol = glbcolumn[dev]` (`:545`) = the console column the BDOS output
tracker last left on that device (see column tracking below). Loop, one
`getch(dev)` per iteration (`:548`; `getch` = BDOS typeahead buffer first for
dev 1 and 2 only, else `Bconin(dev)`, `bdos/console.c:351-366`); the key's **low
byte** is `ch` (`char ch`, `:542`):

| Key (`ch`) | Cite | Effect |
|---|---|---|
| CR 0x0D **or** LF 0x0A | `:550-553` | `conout(dev, CR)` (echo **CR only, no LF**), `return retlen`. The terminator is **not stored**, no NUL is appended. |
| BS 0x08 **or** DEL 0x7F | `:554-557` | `backsp`: drop last char if any, then erase it visually (`BS,' ',BS` per column, `bdos/console.c:494-524`, column recomputed over the buffer with tab → next multiple of 8, other control chars = 2 columns). On an empty buffer nothing is erased (never below `stcol`). |
| Ctrl-C 0x03 | `:558-559` | `terminate()` = `xterm(-32)` (`bdos/console.c:101`): process terminated with exit code -32; **`Fread` does not return** (inference: `xterm` does not return; no `break` follows in the source). |
| Ctrl-X 0x18 | `:560-565` | erase the whole line visually (`backsp` until `retlen == 0`); nothing echoed besides the erase. |
| Ctrl-U 0x15 | `:566-570` | echo `#`, `newline(dev, stcol)` (CR, LF, then `stcol` spaces, `bdos/console.c:480-491`), `retlen = 0` (buffer discarded). |
| Ctrl-R 0x12 | `:571-576` | echo `#`, `newline`, then redisplay the buffer with `cookdout`; `retlen` unchanged. |
| anything else (incl. Ctrl-S/Ctrl-Q arriving through `getch`, ESC, TAB, bytes >= 0x80, NUL) | `:577-579` | stored as `buf[retlen++] = ch`, echoed with `cookdout` (`bdos/console.c:312-325`): TAB → `tabout` (spaces to the next multiple of 8), other `< 0x20` → `^` + (ch\|0x40), everything else raw. |

- **Termination by length:** when `retlen` reaches `maxlen` the loop ends at the
  top (`:546`) and `retlen` (= `maxlen`) is returned **without** waiting for or
  consuming CR/LF and without echoing a CR; the terminator then stays in the
  input and ends the next read as an empty line.
- **Return value:** number of characters stored, `0 .. maxlen`; CR/LF excluded.
  An empty line returns **0** (indistinguishable from `len == 0`/HUGE).
  There is no EOF condition and no negative return from `cgets` itself.
- **CR+LF pair:** only one terminator is consumed (`:550-553`); an LF following
  a CR is left in the input.
- **Typeahead / break handling during echo:** every echo goes through
  `conout` (`bdos/console.c:263-274`) which first runs `conbrk(dev)` (`bdos/console.c:206-243`):
  if `Bconstat(dev)` is non-zero it drains `Bconin(dev)`: Ctrl-C → flush
  typeahead + `terminate()`; Ctrl-S → pause (loops reading until Ctrl-Q);
  Ctrl-Q → resume; Ctrl-X → flush typeahead (and also queued); anything else is
  appended to the 64-entry typeahead buffer (`bdos/fs.h:46`; full → BEL
  `Bconout(dev,7)`, char dropped). Hence Ctrl-S/Ctrl-Q typed *while a previous
  character is being echoed* are swallowed, but arriving at the blocking
  `getch` they are stored like any other char (timing-dependent; read from the
  code, not a design guarantee).
- **Column tracking** (`conout`, `bdos/console.c:266-272`): `>= 0x20` → column+1;
  CR → 0; BS → column-1 (not clamped); other controls unchanged. Only output
  through `conout`/`tabout` (Cconout, Cconws, Fwrite to CON, echo) updates it;
  raw `Bconout`/`Crawio`/`Cauxout`/`Cprnout` do not.

### Per device

| Handle → BIOS dev | `Fread(h, 1, buf)` | `Fread(h, n>=2, buf)` |
|---|---|---|
| CON (-1) → 2 (also std 0,1,4,5 default) | blocking `Bconin(2)` (or typeahead), echoes the raw key to the console, stores low byte, returns 1 | `cgets` on CON as above (blocking, editing, echo, ends at CR/LF or `n`) |
| AUX (-2) → 1 (std 2 default) | blocking `Bconin(1)` (serial iorec, `bios/serport.c:260-263`), **echoes the byte back out the AUX port**, stores, returns 1 | `cgets` on AUX: same editing/echo logic, echo goes to AUX; typeahead buffer `[1]` (`bdos/console.c:44,116`); CR/LF from the line terminates |
| PRN (-3) → 0 (std 3 default) | `Bconin(0)` = `bconin0` returns 0 immediately ("not implemented", `bios/parport.c:123-127`): stores **0x00**, then echo `Bconout(0,0)` sends a NUL to the printer (`conbrk(0)`: `Bconstat(0)` = `char_dummy` = 0, `bios/chardev.c:30-31,54-58`, so `buffer[0] == NULL` is never dereferenced); returns 1 | `cgets` on PRN (inference by tracing the code, not run): every `getch` returns 0 → default branch stores 0x00 and `cookdout` sends `^`,`@` to the printer per byte → returns **n** NUL bytes, never blocks on input, sends 2n echo bytes to the printer (each `Bconout` may block until the 30 s printer timeout when `CONF_WITH_PRINTER_PORT` and no printer, `bios/parport.c:40-41,150-168`; ignored return) |

BIOS-level gating: `bconin`/`bconout` return 0 if `CHARDEV_AVAILABLE` is not yet
set (`bios/bios.c:920-921`, `964-965`, `997-998`) — early boot only.
`Bconmap` (`BCONMAP_AVAILABLE = CONF_WITH_SCC || CONF_WITH_TT_MFP`,
`bios/serport.h:18`) only remaps BIOS handles `>= 6` (`bios/bios.c:922-928`),
so BIOS 0..2 always use `bconin_vec[]` (`bios/chardev.c:34-36`).

### Error table (device path)

| Condition | Cite | Returned |
|---|---|---|
| handle resolves to `0` (free sft slot, `p_uft[h] == 0`) | `:592-593` | `EIHNDL` (-37) |
| handle `<= -4` (and any `numl < -3`) | `:600-602` | `EIHNDL` (-37) |
| `h >= NUMHANDLES` | `:578-579` reads `sft[]` out of bounds in EmuTOS; contract per `42-fseek.md` | `EIHNDL` |
| device handle, HUGE (`pw[2] != 0`, incl. negative `len`) | `:610-611` | `0` |
| device handle, `len` 0 or 32768..65535 | `bdos/console.c:546` | `0` |
| any device read | — | never a negative code other than `EIHNDL`; no EOF/short-count signalling |

### Cross-checks (device path)

- **FreeMiNT** (`references/freemint`, not followed): `sys_f_read` sends any
  terminal handle to `tty_read` (`sys/dosfile.c:277-278`) with termios
  semantics; no HUGE guard, no `cgets` line editor. Not examined further.
- **Atari Compendium / Profibuch:** not in the repository (`find` over the repo
  for `*compendium*`/`*profibuch*` → none); no cross-check possible. Whether
  TOS 1.x-4.x ROM behaves identically (e.g. the PRN read) is **unverified**; the
  `ARCH03`/tosdump reference run could confirm.

### Test cases (device path; **EmuTOS reference expectations**)

> These are the EmuTOS results. The NewTOS expectations (decisions of
> 2026-10-13, AUX = port present/no cable (option A), absent PRN entry; the
> `edit_line` echo follows EmuTOS `cgets` again since TASK/CON01, 2026-10-18) are the
> table "NewTOS host tests" at the end of this section; where the two
> differ, the NewTOS table is the one the Developer implements.

Setup: default process (std handles per `bdos/console.c:60-67`); keyboard queue and
AUX queue injectable; per-device output captured. `Q(c…)` = bytes queued.

| # | Setup | Call | Expected |
|---|---|---|---|
| R1 | `Q('A')` on CON | `Fread(-1, 1, b)` | returns 1, `b[0]='A'`, CON output `"A"` (echo) |
| R2 | `Q(CR)` | `Fread(-1, 1, b)` | 1, `b[0]=0x0D`, echo `CR` only |
| R3 | `Q('h','i',CR)` | `Fread(-1, 80, b)` | 2, `b="hi"`, no CR/LF/NUL stored; echo `"hi\r"` (no LF) |
| R4 | `Q('h','i',LF)` | `Fread(-1, 80, b)` | 2, `b="hi"`, echo `"hi\r"` |
| R5 | `Q(CR)` | `Fread(-1, 80, b)` | 0 (empty line), echo `"\r"` |
| R6 | `Q('a','b','c','d')` | `Fread(-1, 3, b)` | 3, `b="abc"`; `'d'` still queued; no CR echoed |
| R7 | `Q('a','b',BS,'c',CR)` | `Fread(-1,80,b)` | 2, `b="ac"`; echo has `BS,' ',BS` for the erase |
| R8 | `Q('a',DEL,DEL,'b',CR)` | `Fread(-1,80,b)` | 1, `b="b"` (second DEL on empty buffer erases nothing) |
| R9 | `Q('a','b',^X,'c',CR)` | `Fread(-1,80,b)` | 1, `b="c"` |
| R10 | `Q('a','b',^U,'c',CR)` | `Fread(-1,80,b)` | 1, `b="c"`, echo contains `#`,CR,LF,`stcol` spaces |
| R11 | `Q('a',TAB,'b',CR)` | `Fread(-1,80,b)` | 3, `b="a\tb"`; echo `a` + 7 spaces (to column 8) + `b` + CR |
| R12 | `Q(ESC,CR)` | `Fread(-1,80,b)` | 1, `b[0]=0x1B`, echo `"^["` + CR |
| R13 | any | `Fread(-1, 0, b)` | 0, nothing consumed/echoed |
| R14 | any | `Fread(-1, 0x10000, b)`, `Fread(-1, -1, b)`, `Fread(-1, 0x7FFFFFFF, b)` | 0 each (HUGE) |
| R15 | any | `Fread(-1, 0x8000, b)`, `Fread(-1, 0xFFFF, b)` | 0 each (negative low word → `maxlen<0`) |
| R16 | `Q('x',CR)` | `Fread(0,80,b)`, then `Q` same + `Fread(1,80,b)`, `Fread(4,…)`, `Fread(5,…)` | each 1, `"x"` (all CON) |
| R17 | AUX queue `('o','k',CR)` | `Fread(-2, 80, b)` and `Fread(2, 80, b)` (separate runs) | 2, `"ok"`; echo `"ok\r"` appears on **AUX** output, not CON |
| R18 | AUX queue `('z')` | `Fread(-2, 1, b)` | 1, `b[0]='z'`, echo `'z'` on AUX |
| R19 | printer present | `Fread(-3, 1, b)` / `Fread(3, 1, b)` | 1, `b[0]=0`; printer receives NUL |
| R20 | printer present | `Fread(-3, 4, b)` | 4, `b = 00 00 00 00`, printer receives `"^@^@^@^@"` (derived by tracing; mark as inferred in the test) |
| R21 | any | `Fread(-4, 1, b)`, `Fread(-32768, 1, b)` | `EIHNDL` |
| R22 | free sft slot `6+k` | `Fread(6+k, 1, b)` | `EIHNDL` |
| R23 | `d = Fdup(0)` (CON) | `Fread(d, 1, b)` with `Q('q')` | 1, `'q'`; after `Fclose(d)` → `EIHNDL` |
| R24 | `Fforce(0, -2)` | `Fread(0, 80, b)` | reads from AUX (as R17) |
| R25 | `Fforce(0, fh)` on file "AB" | `Fread(0, 10, b)` | 2, file path (no echo) |
| R26 | after R25, `Fclose(0)` | `Fread(0,1,b)` | CON read again (reverted) |
| R27 | `Q(^C, …)` | `Fread(-1,80,b)` | process terminated (exit -32), call does not return (host test: observe the terminate hook) |
| R28 | `Q('a')` queued after the key being echoed… (optional) | typeahead preserved across calls: `Q('a','b',CR)`; `Fread(-1,1,b)` then `Fread(-1,80,b)` | 1 (`'a'`), then 1 (`"b"`) |

### NewTOS host tests (decisions 2026-10-13; IDs the Developer implements)

Setup as above, but with the NewTOS device table (see "NewTOS implementation
notes" below): CON = real console, **AUX (BIOS 1) = the "port present, no cable" entry** (`Bconin` → 0 without blocking, `Bconstat` → 0, `Bconout` accepts+discards → 1, `Bcostat` → -1; item 1a below) and **PRN (BIOS 0) = the
documented absent-device entry** (`Bconin` → 0, `Bconout` → 0,
`Bconstat` → 0, `Bcostat` → 0). "Echo" = bytes handed to `Bconout(dev, …)` of
the device the read is on. Rows marked **(=)** are unchanged from the EmuTOS
table; **(≠)** differ and give the NewTOS value. (Until TASK/CON01, 2026-10-18,
R9-R12 were "buffer/return only" / raw-echo rows because `edit_line` had no
`glbcolumn`/`cookdout`; they are EmuTOS rows now.)

| # | Call / setup | NewTOS expected |
|---|---|---|
| R1 | **(=)** `Q('A')`; `Fread(-1,1,b)` | 1, `b[0]='A'`, CON echo `"A"` |
| R2 | **(=)** `Q(CR)`; `Fread(-1,1,b)` | 1, `b[0]=0x0D`, echo `CR` only (len 1 = `conin`: CR is stored, no line editing) |
| R3 | **(=)** `Q('h','i',CR)`; `Fread(-1,80,b)` | 2, `b="hi"` (no CR/LF/NUL stored), echo `"hi\r"` |
| R4 | **(=)** `Q('h','i',LF)`; `Fread(-1,80,b)` | 2, `b="hi"`, echo `"hi\r"` |
| R5 | **(=)** `Q(CR)`; `Fread(-1,80,b)` | 0, echo `"\r"` |
| R6 | **(=)** `Q('a','b','c','d')`; `Fread(-1,3,b)` | 3, `b="abc"`, `'d'` still queued (moved by the echo's `conbrk` into the BDOS typeahead; the next read returns it), no CR echoed |
| R7 | **(=)** `Q('a','b',BS,'c',CR)`; `Fread(-1,80,b)` | 2, `b="ac"`, echo contains `BS SP BS` for the erase |
| R8 | **(=)** `Q('a',DEL,DEL,'b',CR)`; `Fread(-1,80,b)` | 1, `b="b"` |
| R9 | **(=)** `Q('a','b',^X,'c',CR)` | 1, `b="c"`; echo erases both characters with `BS SP BS` |
| R10 | **(=)** `Q('a','b',^U,'c',CR)` | 1, `b="c"`, echo contains `#`,CR,LF,`stcol` spaces |
| R11 | **(=)** `Q('a',TAB,'b',CR)`; `Fread(-1,80,b)` | 3, `b="a\tb"`; echo `a` + 7 spaces + `b` + CR |
| R12 | **(=)** `Q(ESC,CR)`; `Fread(-1,80,b)` | 1, `b[0]=0x1B`, echo `"^["` + CR |
| R13 | **(=)** `Fread(-1,0,b)` | 0, nothing consumed/echoed |
| R14 | **(=)** `Fread(-1,0x10000,b)`, `Fread(-1,-1,b)`, `Fread(-1,0x7FFFFFFF,b)` | 0 each (high word set) |
| R15 | **(=)** `Fread(-1,0x8000,b)`, `Fread(-1,0xFFFF,b)` | 0 each |
| R16 | **(=)** `Fread(0|1|4|5, 80, b)` with `Q('x',CR)` | each 1, `"x"` (CON) |
| R17 | **(≠)** AUX default entry (no cable; `Bconin`=0 non-blocking): `Fread(-2,80,b)` and `Fread(2,80,b)` | **80**, `b` = 80 × `0x00`; does not block; no byte read from CON; echo to AUX is accepted (`Bconout(1)`=1) and discarded, so nothing observable on any device |
| R18 | **(≠)** `Fread(-2,1,b)` | 1, `b[0]=0x00` (`conin` stores low byte of `Bconin(1)`=0) |
| R19 | **(≠)** `Fread(-3,1,b)` / `Fread(3,1,b)` | 1, `b[0]=0x00`; nothing sent anywhere |
| R20 | **(≠)** `Fread(-3,4,b)` | 4, `b` = `00 00 00 00` (derived by tracing `cgets` over a `Bconin`=0 device, `bdos/console.c:548,577-579`; mark inferred in the test) |
| R21 | **(=)** `Fread(-4,1,b)`, `Fread(-32768,1,b)` | `EIHNDL` |
| R22 | **(=)** free sft slot `6+k` | `EIHNDL` |
| R23 | **(=)** `d=Fdup(0)`; `Q('q')`; `Fread(d,1,b)`; `Fclose(d)`; `Fread(d,1,b)` | 1 `'q'`; then `EIHNDL` |
| R24 | **(≠)** `Fforce(0,-2)`; `Fread(0,80,b)` | 80 NULs (AUX no-cable entry, as R17) |
| R25 | **(=)** `Fforce(0,fh)` on file "AB"; `Fread(0,10,b)` | 2, file path, no echo |
| R26 | **(=)** after R25 `Fclose(0)`; `Fread(0,1,b)` | CON read again |
| R27 | **(=)** `Q(^C)`; `Fread(-1,80,b)` | process terminated (exit -32), no return (observe the terminate hook) |
| R28 | **(=)** `Q('a','b',CR)`; `Fread(-1,1,b)` then `Fread(-1,80,b)` | 1 (`'a'`), then 1 (`"b"`) |
| R29 | `Fread(-2,2,b)`; `Fread(2,2,b)` (std handle 2 = AUX by default) | 2 and 2, NULs each (echo to AUX accepted, discarded) |
| R30 | `Fread(-2,0x7FFF,b)`; `Fread(-3,0x7FFF,b)` | 0x7FFF each, all NUL, returns (no blocking, no timeout) |
| R31 | `Fread(-3,0x8000,b)`, `Fread(-2,0x10000,b)`, `Fread(-2,0,b)` | 0 each (same guards as CON; the device is not touched) |
| R32 | `Fread(-3,-1,b)` | 0 (high word set) |
| RT1 | **single dispatch point.** Replace the AUX table entry by a test device whose `Bconin` queue is `('o','k',CR)` and which captures `Bconout`; `Fread(-2,80,b)` | 2, `b="ok"`, captured `"ok\r"`; CON untouched. Same entry must be hit by: `Fread(2,80,b)`, `Cauxin()`, `Bconin(1)`, `Bconstat(1)`, `Cauxis()` (and `Fwrite`/`Cauxout`/`Bconout(1,…)` in `40-fwrite.md` WT1) — one fixture, all paths |
| RT2 | same fixture, `Fread(-2,1,b)` with queue `('z')` | 1, `'z'`, captured echo `"z"` |
| RT3 | PRN entry replaced the same way | `Fread(-3,…)` / `Fread(3,…)` / `Bconin(0)` hit the PRN entry (index 0), never the AUX entry |

**mintlib tests (tester, after implementation):** `getchar()` / `fgets()` on
`stdin` (which reaches `Fread(0, 1|n, …)` on CON) must be run by the tester —
`Fread` with `len == 1` now echoes and stores CR (`R2`), and `len >= 2` is a
line read (`R3`..`R8`), which changes what mintlib's line-buffered `stdin`
observes. Not run by the Researcher.

---

## Source extraction summary

`Fread` transfers up to `len` bytes from the file at handle `h` into `ubufr`,
starting at the file's current position, and returns the number of bytes
actually read. `xread` looks up the OFD (invalid handle → `EIHNDL`). `ixread`
clamps `len` to `o_fileln - o_bytnum` (the bytes remaining before EOF); if that
is `<= 0` it returns `0L` immediately. Otherwise it calls `xrw(0, …)`, which
walks the FAT cluster chain, moving whole logical sectors through the buffer
cache where possible and byte-copying the head/tail fragments, and returns the
byte count transferred.

The number returned may be less than `len` without being an error — that is how
callers detect EOF (`ret < len`, or `ret == 0` at/after EOF). EmuTOS does not
distinguish "reached EOF" from "read zero bytes" — both surface as `0`.

A source comment (`bdos/fsio.c:503-505`) records that EmuTOS used to reject
reads from a write-only handle but stopped, for Atari-TOS compatibility: the
`o_mod` access code is **not** consulted by `Fread`. Disk errors during the
transfer are raised via the BIOS `longjmp` path (see the dispatcher spec),
not returned inline by `xread`.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x3F
- **Stack frame on entry (m68k):** `Fread(handle, count, buf)` =
  `trap1(0x3f, handle, count, buf)` (`include/bdosbind.h:40`):
  ```
  +8: buf    (LONG, pointer)
  +4: count  (LONG)
  +2: handle (word)
   0: function_id (word) = 0x003F
  ```
- **Total parameter bytes:** 10 (WORD + LONG + LONG) → `wparms = 5`.
- **Native (AArch64) ABI:** ID `0x3F` in `x8`; args in `x0..x2`; LONG result
  (bytes read or negative error) sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name    | Type / size |
  |------------|-------------|---------|-------------|
  | +2         | `x0`        | `h`     | `WORD` (16) |
  | +4         | `x1`        | `len`   | `LONG` (32) |
  | +8         | `x2`        | `ubufr` | `void *` (ptr) |

---

## Inputs

| Position | Name    | Type    | Size | Source location    | Notes |
|----------|---------|---------|------|--------------------|-------|
| arg0     | `h`     | `int`   | 2    | `bdos/fsio.c:440`  | file handle |
| arg1     | `len`   | `long`  | 4    | `bdos/fsio.c:498`  | max bytes; clamped to bytes-to-EOF; `<= 0` ⇒ 0 |
| arg2     | `ubufr` | `void *`| 4    | `bdos/fsio.c:440`  | destination buffer; no alignment or bounds check |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** bytes actually read (`0 .. len`) on success; negative error.
- **Source location:** `bdos/fsio.c:281-283` (`xrw` returns `o_bytnum - bytpos`).

### Side effects on global state

- Advances the OFD position fields (`bdos/fsio.c:33-47` `addit`).

### Side effects on hardware

- Sector reads via the buffer cache / `Rwabs` for any cluster not already cached.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Invalid handle (`getofd` NULL) | `bdos/fsio.c:445-449` | `EIHNDL` (-37) |
| Handle `<= -4` / free sft slot (before `xread`) | `bdos/bdosmain.c:592-593,601-602` | `EIHNDL` (-37) |
| Character device (-1..-3, std handles by default) | `bdos/bdosmain.c:608-620` | see "Character-device path" (no error codes; HUGE → 0) |
| Underlying disk read failure | via `longjmp_rwabs` (`bdos/fs.h:60`) | BIOS code (e.g. `EREADF` -11) |

**Error paths not in Compendium:** none new; note `len <= 0` is **not** an
error (returns `0`), and reading at/after EOF returns `0`, not an error.

---

## Verification criteria

- **Nominal:** on a 100-byte file at position 0, `Fread(h, 100, buf)` returns
  100 and fills `buf`.
- **Partial at EOF:** at position 90 of that file, `Fread(h, 100, buf)` returns
  10 and leaves the position at EOF.
- **At EOF:** a further `Fread(h, 100, buf)` returns 0.
- **Zero/negative len:** `Fread(h, 0, buf)` and `Fread(h, -5, buf)` return 0
  with no transfer.
- **Read of write-only handle succeeds** (mode not enforced).
- **Error:** `Fread(9999, 1, buf)` (no such open handle) returns `EIHNDL` (-37).
- **Character devices:** EmuTOS reference R1..R28; NewTOS host tests R1..R32 + RT1..RT3 (see "NewTOS host tests").

---

## Edge cases discovered in source

1. **EOF truncation is silent.** `len` is clamped to `o_fileln - o_bytnum`;
   there is no short-read error (`bdos/fsio.c:507-513`).

2. **`len <= 0` returns 0.** The `if (len > 0)` gate means zero and negative
   requests transfer nothing (`bdos/fsio.c:510-513`).

3. **Write-only handles are readable.** The `o_mod` access code is not checked;
   a comment documents the deliberate Atari-TOS compatibility choice
   (`bdos/fsio.c:503-505`).

4. **BIOS char devices ARE readable via `Fread` (CORRECTED 2026-10-13).** The
   earlier text here ("`getofd` returns NULL → `EIHNDL`, no `Fread(-1)`
   bridge") was wrong: `osif` answers character-device handles in
   `bdos/bdosmain.c:571-643` before `xread` runs; see "Character-device path".
   Only handles `<= -4`, free sft slots and `p_uft == 0` give `EIHNDL`.

5. **No buffer validation.** `ubufr` is trusted; no alignment or region check
   (`bdos/fsio.c` uses `memcpy` into it).

6. **Shared-handle position hazard.** Two handles onto the same OFD (via
   `Fdup`/`Fforce`) share `o_bytnum`; interleaved reads corrupt each other's
   position. Single-tasking makes this the caller's responsibility.

---

## Dependencies

### Requires (must be initialized / functional)

- A valid OFD (from `Fopen`/`Fcreate`); the FAT cluster chain; buffer cache.

### Used by

- Every program that reads files; the loader; AES resource loading.

---

## Related data structures

- `docs/api/structures/ofd.md` — OFD position fields, DFD `o_fileln`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the
  `(handle, count, buf)` ABI and "returns bytes read".)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None; behaviour independent of build flags.

---

## NewTOS implementation notes

(Empty during Phase 00.)

FS11 (2026-10-13, user decisions; ticket `TASK/FS11.md`, follow-up `TASK/IO01.md`).
Before this change NewTOS `fread` (`tos-api/src/bdos/fs/file.rs:1052`,
`console_read`) mapped every device handle to a raw blocking `len`-byte console
read; everything below replaces that. Decisions 1-3 are **approved by the user**
(2026-10-13); items marked *derived* are consequences read from the EmuTOS code,
not separate decisions.

**1. One per-BIOS-device dispatch point (deliberate profile, not a stub).**
All character-device paths — `Fread`/`Fwrite` on `-1..-3`, on standard handles
(also `Fdup`/`Fforce`d to devices), `Cconin`/`Cauxin`/`Cprnout`/… and `Bconin`/
`Bconout`/`Bconstat`/`Bcostat` per BIOS device, and `Midiws` — go through ONE
device table indexed by BIOS device number (PRN 0, AUX 1, CON 2, MIDI 3, IKBD 4,
raw-CON 5; handles 6 and up are the `Bconmap` table's, BIOS08: 6 = AUX's entry,
7 and up none), exactly as EmuTOS routes through its `bconin_vec[]` /
`bconout_vec[]` / `bconstat_vec[]` / `bcostat_vec[]` (`bios/bios.c:932,976,1009,1161`).
GEMDOS contains **no special case for AUX/PRN**: `Fread` calls `Bconin`/`Bconstat`
of the device (`conin`, `getch`, `conbrk`, `cgets` of `console.c`), `Fwrite` calls
`Bconout`. A later UART/MIDI/printer driver (`TASK/IO01.md`) replaces only the
table entry.

**GEMDOS reaches only devices 0..2.** The table has six entries, but a GEMDOS
handle resolves only to CON, AUX or PRN: `osif` accepts the device handles
`-1..-3` and answers `EIHNDL` below `-3`, and `HXFORM` maps them to BIOS
devices 2, 1, 0 (`references/emutos/bdos/bdosmain.c:589-639`,
`references/emutos/bdos/console.h:23-25,36`; the standard handles 4/5 default
to CON, `references/emutos/bdos/console.c:60-67`). MIDI (3), IKBD (4), 5 and 6..
are reached only through the BIOS calls (and MIDI through `Midiws`); what is
said about them below describes the BIOS table, not a GEMDOS path — no `Fread`
or `Fwrite` ever reaches them.

**MIDI (3) and IKBD (4) since BIOS04 (2026-10-29, user decision "like the
oracle"; supersedes the MIDI/IKBD part of the next paragraph):** both answer as
the reference machine's ACIAs do — `Bconout` → `1` (byte discarded), `Bcostat`
→ `-1` (both swapped slots); `Bconstat(4)`/`Bconin(4)` → `0` (`char_dummy`);
`Bconstat(3)`/`Bconin(3)` read the MIDI ring of `Iorec(2)`, and `Bconin(3)`
**waits** while it is empty, as EmuTOS' `bconin3`
(`docs/api/bios/{01,02,03,08}-*.md`, BIOS04 notes;
`tos-api/src/bios/chardev.rs:175@MidiPort`,
`tos-api/src/bios/chardev.rs:248@IkbdPort`). Before, both were absent devices
and a program waiting `while (!Bcostat(4));` hung for good (finding
BIOS-CORE-001, `docs/audits/2026-10-28-bios-core.md`).

Current entries (the documented *absent-device* profile): PRN (0) (**handles
6/7 left this profile with BIOS08: 6 is AUX's entry, 7 and up answer `0` past
the one-entry `Bconmap` table, `docs/api/bios/{01,02,03,08}-*.md`**; **AUX (1) is no longer absent, see item 1a; MIDI (3) and IKBD (4) are not
absent since BIOS04, above**): `Bconin` → 0 **without blocking**, `Bconstat` → 0,
`Bcostat` → 0, `Bconout` → 0 (byte discarded). Source of the values — what
EmuTOS does for the same situation:
- AUX: **superseded by item 1a** (was the Bconmap *dummy* map entry `{char_dummy, char_dummy, char_dummy,
  charout_dummy, …}`, `bios/serport.c:96`, `bios/chardev.c:49-58`; its `Bconout(1)`=0 hung mintlib's write loop on stderr).
- PRN: `bconin0` returns 0 (`bios/parport.c:123-127`); `Bconstat(0)` is
  `char_dummy` (`bios/chardev.c:31`); without `CONF_WITH_PRINTER_PORT`
  `bcostat0` returns 0 (`parport.c:139-140`) and `bconout0` returns 0
  (`parport.c:144-172`, whole body gated).
- MIDI: **superseded by BIOS04** (was `charout_dummy`-style results, which are
  not what EmuTOS' own MIDI routines do with or without an ACIA — see
  "Inconsistencies" in `docs/api/xbios/0c-midiws.md`).
- Device 4 (IKBD): **superseded by BIOS04**. Device 5 (raw-CON) keeps its
  existing entry (`docs/api/bios/{01,02,03,08}-*.md`).
Caveat (*cited*): EmuTOS built **without** any RS-232 backend does *not* behave
like the AUX dummy (`bcostat1` returns -1 and `bconin1` blocks, `serport.c:271-286`,
`bconin_iorec` loops `serport.c:202-205`); the AUX profile matches only the
`BCONMAP_AVAILABLE` dummy entry. Contract is what the entry returns, not that EmuTOS
configuration.

**1a. AUX profile = "port present, no cable" (option A, user decision 2026-10-13; supersedes the AUX dummy-entry rule above).**
Trigger: FS11 acceptance finding 1 (`measurements/measurement-2026-10-13-libc-main-fs11-acceptance/RUNINFO.TXT`): with `Bconout(1)`=0, `Fwrite(2,…)`=0 and mintlib's write loop on default stderr retried forever. On a real ST the RS-232 port always exists and accepts bytes with no cable. Entry values:

| BIOS call on AUX (1) | NewTOS value | EmuTOS evidence |
|---|---|---|
| `Bconout(1,b)` | accepts, discards, returns **1** | `bconout1` returns `1L`, `bios/serport.c:323` (MFP, interrupt path `:306-323`; debug-print variant `:305`; coldfire `:301` also 1) |
| `Bcostat(1)` | **-1** (ready) always | `bcostat1` returns `-1L` unless the out-ring is full (`bios/serport.c:271-283`, `:282`); NewTOS discards at once so the ring never fills |
| `Bconstat(1)` | **0** (unchanged) | EmuTOS `bconstat1` → `bconstat_iorec` returns 0 on empty iorec (`serport.c:255-258`, `:191-200`): same value |
| `Bconin(1)` | **0, non-blocking** (unchanged) | **Documented divergence from a real port:** EmuTOS `bconin1` blocks forever on an empty iorec (`serport.c:260-263`, loop `:202-205`); NewTOS stays non-blocking so programs do not hang |

**[XBIOS15] 2026-10-29 — qemu-virt:** AUX there is a serial driver over device 0's
input ring (`docs/api/bios/02-bconin.md`, XBIOS15 note): `Bconstat(1)` is `-1` while a
received byte waits, `Bconin(1)` waits for a byte as EmuTOS' `bconin1`, so
`Fread` on AUX reads received bytes and waits for the rest (R17 holds on real Pi 4
only). `Bconout(1)`/`Bcostat(1)` are the values in this table on both boards.

No-handshake argument (*cited*): `bconout1` waits only on `bcostat1` (`serport.c:296`), which tests ring space, not CTS; `iorec_init` sets `FLOW_CTRL_NONE` (`serport.c:89`, `serport.h:46`); `flowctrl` is never read by any tx/rx path (only assigned `serport.c:238,793`; `serport.h:61` comment "TODO, flow control"). So EmuTOS software does not block on CTS with no cable. (*Inferred*, not read: whether the MFP hardware gates its transmitter on CTS is hardware behaviour outside the software; the contract here is the returned value.) Not modelled: ring back-pressure, `rsconf`/baud/UCR state; a later UART driver (`TASK/IO01.md`) replaces the whole entry. PRN (0) stays absent, unchanged; MIDI (3) is not absent since BIOS04 (item 1).

**Resulting values (derived from the table above + `osif`/`console.c`):**
- `Fread(-2|2, n, b)` / `Fread(-3|3, n, b)`:
  `len==1` → `conin`: `Bconin`=0 → `b[0]=0x00`, echo `Bconout`(discarded), returns **1**.
  `2 <= len <= 32767` → `cgets`: every `getch` yields 0 → `default` branch stores
  `0x00` (no terminator ever arrives, never blocks) → returns **len**, buffer =
  `len` × `0x00`. NewTOS' `edit_line` gives the identical buffer/return.
  `len==0`, `len` 32768..65535, HUGE → **0** (unchanged guards).
  Consequence worth knowing: a program that loops `Fread(2,…)` until a short count
  never gets one on AUX/PRN — it receives an endless stream of NULs (same on EmuTOS
  PRN, `bdos/console.c:548,577-579`).
- `Fread` never returns an error for an absent device; there is no "device absent"
  status.

**2. `Fwrite` on CON** is specified in `40-fwrite.md` (EmuTOS `tabout` since
TASK/CON01, 2026-10-18); the `Fread` echo uses the same `conout` (`conbrk` +
`glbcolumn`).

**3. Edge cases follow EmuTOS** (cited above): HUGE (`pw[2]!=0`) → 0; `len`
32768..65535 → 0; `len==0` → 0; `len==1` → `conin` semantics (echo, CR stored, no
line editing, Ctrl-C stored as data); `len>=2` on CON → `cgets` semantics via
`edit_line` (`tos-api/src/gemdos/console.rs`, the `Cconrs` editor). Since
TASK/CON01 (2026-10-18) the editor is EmuTOS `cgets` including the echo
(`glbcolumn`, `cookdout` `^X` control echo, tab expansion, `#` + `newline(h,
stcol)` for Ctrl-U/Ctrl-R) and `getch` reads the BDOS typeahead first; the
earlier "buffer/return exact, echo differs" divergence is revoked. `edit_line`
runs over the device table (`getch`/`conout` of `dev`), not hard-wired to the
console.

Host tests the Developer implements for this spec: **R1..R32, RT1..RT3** (table
"NewTOS host tests"). Pointer faults while storing to the user buffer follow the
existing `console_read` convention (`crate::uaccess::FAULTED`, ARCH03 3a).

PHASE-05: backed by the FAT driver. On AArch64 the `[ubufr, ubufr+len)` range
is validated once at the trap boundary; the mode-not-enforced behaviour is
contract for source compatibility.

HW03 (2026-09-27): a read that stops because the **device** failed returns the
BIOS code (`EREADF` -11, or `EDRVNR` / `ESECNF` as the driver reported) rather
than a short count; end-of-file and a chain that ends early stay short counts.
Same mechanism as `40-fwrite.md`; driven by
`tos-api/tests/gemdos_fs_device_error.rs`.

FS08 (2026-10-07): the transfer follows EmuTOS `xrw` as extracted in
[`_state.md` §3a](../_state.md): the OFD keeps `o_curcl`/`o_currec`/`o_curbyt`,
so a sequential `Fread` steps the FAT chain one link per new cluster instead
of walking it from the file start on every call; whole records go in one block
request straight into the caller's buffer (contiguous clusters merged up to
`CNTMAX` records), only the partial head/tail records pass through the data
buffer list. Unobservable through the API (same bytes, same return values);
measured effect in `measurements/measurement-2026-10-07-fat-curcl/`.

**PROC04 point 3 re-validation (2026-10-18) — the ticket's claim is obsolete.**
TASK/PROC04 point 3 ("`Fread`/`Fwrite` read the handle as u16; a negative BIOS
handle gives `EIHNDL`; AUX/PRN land on the console") described the state before
FS11 (2026-10-13). Re-read against both sides today: EmuTOS
`bdos/bdosmain.c:571-640` (signed `short` handle, `numl` resolution `:578-593`,
`numl < -3` → `EIHNDL` `:601-602`, device path `:604-640`) and NewTOS
`tos-api/src/bdos/fs/file.rs:393-407` / `:440-455` (`args.arg_i16(0)`, `resolve`
`:231-246` — same resolution order and the same `-3` floor) →
`gemdos/console.rs` `device_read :682` / `device_write :739` (`hxform` = `3+h`
`:620`, HUGE-read guard on the high word, `pw[3]==1` → `conin` path, else
`cgets`; `Fwrite`: CON Ctrl-C-only break + raw byte (EmuTOS: `tabout`), other devices `Bconout` with
short count on 0). Verdict: **confirmed implemented**; no repair needed. Only
residual differences, all already documented above: no tab expansion / Ctrl-C-only
break on CON (approved divergences of 2026-10-13 — revoked and aligned with
EmuTOS by TASK/CON01, 2026-10-18), AUX/PRN backends are the
absent/no-cable device-table entries (TASK/IO01), and a handle `>= NUMHANDLES`
answers `EIHNDL` where EmuTOS reads `sft[]` out of bounds (`:578-579`, contract
per `42-fseek.md`). Tests: `tos-api/tests/fs11_device_rw.rs`,
`fs10_fseek_device_handles.rs`.

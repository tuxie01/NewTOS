# COMPATIBILITY — the contract NewTOS keeps, and what it is free to change

> The measuring stick for every spec, plan and implementation in NewTOS.
> Read alongside [`REQUIREMENTS.md`](REQUIREMENTS.md) (FR-1/FR-3),
> [`ARCHITECTURE.md`](ARCHITECTURE.md) (ABI Rules) and `CLAUDE.md` (Rule 1).
> When a doc or a piece of code is unsure "must this match EmuTOS exactly,
> or may we do it better?", this file decides.

## The goal, in one sentence

NewTOS is a **syscall- and variable-compatible reimplementation** of the
TOS API — **not a 1:1 port** of EmuTOS. EmuTOS is the *behaviour reference*;
the implementation is a clean, modern AArch64 design.

## Why this distinction matters

TOS and EmuTOS were written under 1980s constraints — kilobytes of RAM, a
slow 8 MHz 68000, ROM size limits. Many of their internal choices are
*workarounds for scarcity*, not part of what a program observes. On a
Raspberry Pi 4 those constraints are gone, so NewTOS can implement the same
*observable contract* with a cleaner, faster, safer design. Copying the
1980s internals would import their limitations for no benefit.

## Principle: clean first, optimise later

1. **First**: a correct, clean implementation of the compatibility contract
   (below). Verified against the EmuTOS behaviour and the documented ABI.
2. **Then** (and only then): optimisations and modern extensions, kept
   clearly separate so they never compromise the contract. See
   [Deferred optimisations](#deferred-optimisations-phase-2).

A program built for TOS must not be able to tell — through any documented
interface — that it is running on NewTOS rather than TOS.

## The compatibility contract — the invariant surface

These **must be exact**. They are what a program can observe, so they are
the contract:

| Surface | What is fixed |
|---|---|
| **Trap entry** | trap numbers / `svc` immediates (1 GEMDOS, 13 BIOS, 14 XBIOS; GEM `svc #2` with `x0` selector **115 = VDI**, **200 = AES**, parameter-block pointer in `x1`), function IDs |
| **Parameter ABI** | argument count, order, sizes, and register/stack placement per the two ABIs (m68k stack; AArch64 PCS, ID in `x8`) |
| **Return / error codes** | return values and the negative TOS error codes (`E_OK`, `EFILNF=-33`, …), bit-for-bit |
| **Data structures programs pass** | field layout + semantics of every struct crossing the boundary — DTA, FCB, BPB, MD, PD/basepage, OBJECT, TEDINFO, ICONBLK, BITBLK, AESPB, MFDB, … (offsets a program reads/writes are fixed; see [pointer width](#native-vs-m68k-struct-variants)) |
| **System variables** | the documented globals' *values and semantics* (`_hz_200`, `_bootdev`, `_v_bas_ad`, drive map, cookie jar, …) — see [System variables](#system-variables--compatible-how) |
| **Observable behaviour** | what a documented call *does*: side effects, ordering a program can detect, error conditions, defaults |

## Free implementation — what NewTOS may do differently / better

These are **not** the contract. NewTOS chooses them for cleanliness and
performance; a conforming program cannot observe them:

- Internal data structures, indexes, tables, caches.
- Algorithms (sorting, allocation strategy, FAT walking, scheduling policy).
- Memory layout, alignment, where things physically live (high-half kernel,
  per-process page tables, allocator design).
- Concurrency mechanics not exposed by the API.
- The EmuTOS-specific *scarcity workarounds* (packed bitfields to save ROM,
  re-using buffers, m68k-isms) — reimplemented straightforwardly.
- Anything behind a call whose *result* is unchanged.

> Rule of thumb: if a program can read it, time it, or depend on it through
> a documented interface, it is **contract**. Otherwise it is **free**.

### Sanctioned deviations, one by one

Not everything outside the contract is invisible. Where NewTOS deliberately
shows or does something EmuTOS does not, it is listed here — named, dated and
attributed, so that nobody later mistakes it for drift. The standing rule is
still *match EmuTOS*; an entry here means a human decided otherwise and said so.

**The boot screen reports the ST-RAM pool, not `phystop`** — user decision,
2026-09-04 (*"10Mb ist vollkommen OK"*). EmuTOS' `initinfo` prints
`stramsize = (long)phystop`, the machine's ST-RAM size
(`references/emutos/bios/initinfo.c:277`); on NewTOS that would be **14 MB**.
We print the **10 MiB pool** instead — what a program can actually be given.
Both are constants since `29a4581`; they differ by the 4 MiB the OS occupies
below `membot` and the `.dma` block above `memtop`. No program can observe
this: it is a screen line, not an interface — `Sysconf`, `Malloc(-1)` and the
memory-descriptor chain are untouched and stay EmuTOS-faithful.


**The AES's keyboard poll does not read through `v_string`** — sanctioned
2026-09-06, and it is the *remainder* of a divergence that was open here from
2026-09-04 to 2026-09-06. What was open: EmuTOS' AES reads a keystroke through
`gsx_char()`, which issues `vsin_mode(4, 2)` before every `v_string`
(`references/emutos/aes/gemgsxif.c:459-463`), and `chkkbd()` calls it from the
dispatcher's scheduler on every pass (`aes/gemdisp.c:157-177`), so on any
EmuTOS machine `vqin_mode(4)` reports **1** (SAMPLE) from the first AES tick
onward — measured twice on the reference TT
(`tosdump/reference/EMUTOS14-VDI-r2.BIN` record 16 / case 51,
`EMUTOS14-VDI-all.BIN` record 40 / case 77; devices 1, 2 and 3 read 0 in
records 6, 38, 39). NewTOS reported 0.

**That half is fixed.** `dsptch()` now calls `chkkbd()`, which performs the
same `vsin_mode(4, 2)` write into the same VDI-global state
(`tos-api/src/aes/mod.rs`, `tos-api/src/vdi/input.rs`); `v_opnwk` performs the
`vdimouse_init` reset that goes with it (`vdi/vdi_control.c:563`,
`vdi/vdi_mouse.c:575-578`). Checked against the dump itself by
`tos-api/tests/aes_reference.rs`'s
`aes_dispatch_leaves_the_string_device_in_sample_mode`.

**What stays deliberately different:** the *read*. EmuTOS' `chkkbd` pulls the
key out of the VDI with `v_string` and forks it into the mouse owner's
per-process AES key queue (`forkq(kchange, …)`, `aes/gemdisp.c:172-176`).
NewTOS' AES has no such queue — `evnt_keybd`, the form manager and the file
selector take the key straight from the IKBD ring — which is the same delivery
difference already recorded for the button side in
`tos-api/src/aes/input.rs`. Observable consequence: inside a running GEM
application the AES has *not* already consumed pending keystrokes, so a
`Bconin(2)` from that application still sees them. Closing it means giving the
AES a per-process key queue and is a scheduler change, not an input-mode one.
The mode half — the only part any measured case reaches — now matches.


**GEM trap selector −1 answers `0`, not a dispatcher address** — sanctioned
2026-09-12 with the V16 repair, and recorded here because it was previously
nowhere (audit finding F-2,
`docs/analysis/vdi-r2-improvement-traced-2026-09-12.md`). EmuTOS' `_vditrap`
treats `d0 = -1` (`cmp.w #0xffff,d0`) as *"give me the jsr-able address of the
VDI dispatcher"* and answers `move.l #_GSX_ENTRY,d0`
(`references/emutos/vdi/vdi_asm.S:40,51-52`); the extracted spec carries it
(`docs/api/vdi/_dispatcher.md`, "`d0` special values"). NewTOS has no such
entry point: the VDI is reachable **only** through `svc #2`, and the dispatcher
is EL1 kernel code that an EL0 caller can neither branch to nor read, so there
is no address that would answer the question truthfully. We return `0` — "no
jsr-able address" — which is the value a conforming caller tests for before
using the result, and the value NewTOS has always returned here
(`kernel/src/arch/aarch64/exceptions.rs`, arm `GEM_SEL_ADDR_QUERY`). It is
spelled out as its own trap arm so it cannot silently inherit the neighbouring
"leave `x0` alone" rule, which would hand back `-1` — a value that *looks* like
an address. Handing out a real kernel code address instead would be both
useless (EL0 cannot execute it) and a kernel-address leak. Covered by
`kernel/src/selftest_gemtrap.rs`.

**Not** a sanctioned deviation, and repaired in the same change: selector `-2`
(and every other unknown selector). There EmuTOS leaves `d0` untouched, and so
does NewTOS now — `vq_gdos()` reports *no* GDOS and `vq_vgdos()` returns `-2`,
as on Atari TOS.

**Latent width mismatch, no repair yet (A53, V16 follow-up):** EmuTOS compares
the selector as a **16-bit word** (`cmp.w #0x73,d0` / `cmp.w #0xffff,d0`,
`references/emutos/vdi/vdi_asm.S:33-52` — only the low-order WORD of `d0`
matters). NewTOS's arm (`kernel/src/arch/aarch64/exceptions.rs`, the
`TrapClass::Gem` `match args.regs[0]`) compares the **full 64-bit** `x0`.
Harmless today: no caller in the tree sets bits above the low word
(`grep -rn 'svc[[:space:]]*#2'` over `kernel tos-api sdk tosdump tools` turns
up none). It becomes relevant the day a real m68k binary is run through a
thunk that has to map its 32-bit `d0` into `x0` — if that mapping ever leaves
stale bits above bit 31 (e.g. a zero/sign-extension bug, or `x0` reused
across calls without being fully cleared), a selector NewTOS should recognise
(`115`, `200`, `-1`, `-2`) would instead fall through to the "leave `x0`
unchanged" arm here. No ticket — nothing to fix before the m68k thunk exists —
tracked as a comment at this entry per `TASK/A53.md` (V16 Nachtrag).


**`wind_get(WF_SCREEN)` answers FALSE when there is no menu/alert buffer;
EmuTOS answers TRUE with address 0** — sanctioned 2026-09-12 with the A54
repair, **narrowed 2026-09-20 with A57 to the buffer-less fallback**. EmuTOS'
field 17 hands out the AES' menu/alert screen-save buffer: `gsx_malloc` takes
it once when the AES goes graphic (`references/emutos/aes/gemgsxif.c:108-114`,
from `sh_tographic`, `aes/gemshlib.c:236`), `gsx_mret` reports its address in
`poutwds[0..1]` and its length in `poutwds[2..3]` (`:125-129`), and `wm_get`
returns TRUE (`aes/gemwmlib.c:1563-1565`). Measured on the reference TT
running the 1.4 ROM: address `0x01000218`, length `40960`
(`measurements/measurement-2026-09-12-aes-r4/`, cases 331, 332 and 439 of
`tosdump/cases/aes-r4-subfunctions.tsv`).

Since A57 NewTOS has that buffer (`tos-api/src/aes/gsxif.rs`: one
kernel-owned block of the BDOS pool, taken by the same `Mxalloc(…,
MX_PREFTTRAM)` from `sh_tographic`, used by the alert and the drop-down, handed
to an EL0 process read-write through its own address space) and answers
`WF_SCREEN` TRUE with the two LONGs, as EmuTOS does. What remains of this
entry is the one state in which `gsx_malloc` has nothing to report — the pool
could not supply the block at boot (`ENSMEM`). EmuTOS then leaves
`gl_tmp.fd_addr == NULL` and `gsx_mret` reports it: **TRUE, address 0**, a
pointer the program will scribble through. NewTOS answers **FALSE and writes
nothing** — `wm_get`'s own idiom for a field it cannot serve (`default: return
FALSE`, `aes/gemwmlib.c:1590-1591`), the same repair AUD-P5-014 got for
`WF_KIND`; a conforming caller tests the return code before using the
address, so FALSE is the one answer that cannot be misread as "here is a
buffer, at 0". The alert and the drop-down keep working in that state through
a per-save kernel-heap copy (the pre-A57 path), which no program can observe.
Covered by `tos-api/tests/aes_window_screen_buffer_absent.rs`; the round-1
reference case 166 compares directly again (the `SANCTIONED` list in
`tos-api/tests/aes_reference/harness.rs` is empty).


**The AES menu/alert screen buffer is one screen in device pixels, not
`MENU_BUFFER_SIZE` character cells** — sanctioned 2026-09-20 with A57
(design: `docs/analysis/a57-aes-screen-buffer-design-2026-09-20.md`). EmuTOS
sizes the buffer `gl_mlen = MENU_BUFFER_SIZE * gl_wchar * gl_hchar *
gl_nplanes / 8` with `MENU_BUFFER_SIZE = 640` cells ("somewhat arbitrary",
`aes/gemgsxif.c:44-73,98-104`) and, when a rectangle does not fit, clips the
saved **height** — "adjust height to fit buffer: this will leave droppings!"
(`:334-343`): the bottom of the box is never saved and never put back. At
NewTOS' profile (8×16 cell, 16 planes) the formula gives 163 840 bytes; the
largest `form_alert` the builder allows (three 20-character buttons,
`gemfmalt.c:173,231-239`) needs 172 800 and would overflow it. NewTOS takes
`stride16(W) * H * 2` bytes — `bb_save_words(W, H) * 4`, the whole screen:
614 400 at 640×480, 4 147 200 at 1920×1080 — from the same pool with the same
`Mxalloc`. Both users clamp their rectangle to the screen (`fm_alert`
`rc_intersect(&gl_rscreen, …)`, `gemfmalt.c:358`; the drop-down box lies on
it), so the overflow path is structurally unreachable; the guard stays as a
counted defence (`gsxif::clipped_saves`). **Observable:** the length
`wind_get(WF_SCREEN)` reports is larger than on a TOS/EmuTOS of the same
resolution (614 400 against 163 840 at 640×480/16 bit; 40 960 on the
four-plane TT). A program that takes the length as an upper bound for its own
saves (TeraDesk `sdk/apps/teradesk/resource.c:305-319`, which deletes thirteen menu entries when
its largest drop-down does not fit) gets more room, never less. Two more
things are internal and unobservable: the block is taken once at
`sh_tographic` and — unlike `gsx_mfree` in `sh_toalpha`
(`aes/gemshlib.c:258`) — never freed, so its address is stable across an alpha
interlude where EmuTOS' may move. (Until ARCH03 stage 1 the EL0 view of the
block was a per-process window VA; since stage 1 it is the block's own
address, the same for every process, as on EmuTOS.) Pop-ups and nested submenus are unchanged: a fresh ST-RAM block per
opening, as `popup_blit` does (`aes/gemmnext.c:136-215`).


**An AES handler that writes fewer than `OUT_LEN` words delivers 0 in the
unwritten words; EmuTOS delivers the uninitialised dispatcher stack there** —
sanctioned 2026-09-19 with A27, after the A15 measurement made the difference
visible. EmuTOS' `xif` builds every result in a **local, uninitialised**
`WORD int_out[O_SIZE]` (`references/emutos/aes/gemsuper.c:432`) and copies
`OUT_LEN` words of it back **unconditionally** (`:444-445`), whatever the
handler filled in. A handler that writes fewer words — `wind_get(WF_COLOR)`
writes `int_out[2..3]` and never `[1]` or `[4]` (`aes/gemwmlib.c:1569-1570`),
the slider fields and `WF_TOP` write `[1]` only (`:1542-1554`), the end of a
rectangle list writes `[3..4]` only (`:1240`), a refused field writes nothing
but the return code (`:1521-1523`, `:1591-1592`), `objc_sysvar`'s set mode
writes nothing but the return code (`aes/gemoblib.c:1355-1390`), an unknown
opcode writes `-1` and nothing else (`gemsuper.c:406-417`) — therefore hands
the caller whatever the supervisor stack held under those words: in practice
the previous AES call's answer. Measured on the reference TT: `1, 31, 63, 0, 0`
and `1, 0, 19, 0, 0` for two end-of-list `WF_NEXTXYWH` calls in one run
(`TASK/A15-STATUS-UNFINISHED.MD`, cases 127/133/204 of
`tosdump/cases/aes-r2b-rlist.tsv`; erratum in `tosdump/cases/ERRATA.md`), and
`graf_handle` with `OUT_LEN 7` answering `…, 0, 1` in `int_out[5..6]`
(case 283 of `aes-r4-subfunctions.tsv`). NewTOS zeroes the same local array
(`tos-api/src/aes/mod.rs`, `dispatch`) and copies back the same
`min(OUT_LEN, O_SIZE)` words, so the unwritten words read **0**. The full
per-opcode inventory — which handlers under-write, on which path, against
which binding — is `docs/analysis/aes-underwriting-handlers-2026-09-19.md`.

Three reasons, and they point the same way:

1. **Applications: no effect.** On real TOS the value is indeterminate; a
   program that reads an unwritten `int_out` word is already broken there and
   depends on its own call history. Conforming programs do not read them
   (QED gates `kreturn` on `event & MU_KEYBD`, `sdk/apps/qed/src/aktion.c:148`).
   And 0 is what the client binding itself *intends* for such a word: gemlib
   pre-zeroes `aes_intout[3..4]` before `wind_get` and `aes_intout[5]` before
   `graf_xhandle` "for AES that doesn't support this extension"
   (`sdk/gemlib/gemlib-0.44.0/a_wind_get.c:355-358`, `sdk/gemlib/gemlib-0.44.0/a_graf_xhandle.c:48-49`)
   — a pre-zeroing that EmuTOS' unconditional copy-back defeats and NewTOS
   honours.
2. **Measurement: real effect, and it must be read as noise.** Every
   under-writing call whose case declares more `OUT_LEN` than the handler
   writes diffs against the TT reference in exactly those words — the class
   the r4 assessment files as "S3-a, no soll"
   (`docs/analysis/aes-r4-subfunktionen-einordnung-2026-09-12.md` §4, 23
   cases). A comparison harness must state those words itself rather than
   hold them against the recorded stack, as `tos-api/tests/aes_reference/harness.rs`
   already does for `WF_OWNER`. Whoever does not know this reports a defect
   where there is none — which is why the deviation is named here.
3. **Protection boundary: an argument for 0, not merely a tolerance of it.**
   Between the handler and the application NewTOS has an EL0/EL1 boundary
   classic TOS did not have. Copying an uninitialised kernel-stack word to an
   application was harmless on a machine without protection; here it would be
   a kernel-stack leak into EL0 on every under-writing call. Zeroing the local
   array is the boundary being kept, and the same rule the dispatcher applies
   to the inputs (`int_in`/`addr_in` past `IN_LEN`/`AIN_LEN`, A19).

What is **not** changed by this: the copy-back boundary itself. Exactly
`OUT_LEN` words are written, none past it, none at all for `OUT_LEN 0` — the
caller's poison stays outside the declaration on both machines (measured on
every case of every table). Within `OUT_LEN` the caller **never** keeps its
own word, on either machine; a spec sentence claiming "unchanged" for such a
word was wrong for both and has been corrected (`window-graphics.md`, D-095
and the end-of-list note). Covered by
`tos-api/tests/aes_intout_underwrite.rs` (`wind_get(0, WF_COLOR)`,
`graf_handle` at `OUT_LEN 7`, `wind_get(0, WF_HSLIDE)` at `OUT_LEN 5`,
`objc_sysvar` set mode at `OUT_LEN 3`, with a poisoned caller array).


**`rsrc_gaddr`/`rsrc_saddr` refuse an index past the table; EmuTOS hands out
a pointer past it** — sanctioned 2026-09-20 with A50 (user decision, option
(a): match the return rule, keep the bound). EmuTOS' `get_addr`
(`references/emutos/aes/gemrslib.c:112-176`) checks the **type** only:
every defined `R_*` arm computes `rs_hdr + offset + size * index` with no
look at the header's counts (`get_sub`, `:102-106`), and `rs_gaddr` answers
`ret = (*rsaddr != (void *)-1L)` (`:720`) — TRUE for anything that is not
the `default:` arm's `-1L` (`:170-172`). `rs_saddr` stores through the same
address (`:733-738`). NewTOS resolves in a descriptor that knows each
table's count and its target/kernel-alias pair, and answers **not found**
where the computed address would leave the table. Written out, the three
cases where the two machines differ:

1. **Index at or past the table, or a table the resource does not carry**
   (`R_OBJECT` index 1000 on a 3-object resource, r4 case 626; `R_BITBLK`
   on a resource without BITBLKs). EmuTOS: `int_out[0] = 1`,
   `addr_out[0]` = a pointer past the array, and `rsrc_saddr` writes there
   and answers TRUE. NewTOS: `addr_out[0] = 0`, `int_out[0] = 0`;
   `rsrc_saddr` answers FALSE and writes nothing.
2. **Nothing loaded** — no blob the caller's `AESGLOBAL` names (the A39
   contract, `docs/api/aes/resource-fileselector.md` §1 callout item 4).
   EmuTOS re-binds to `ap_rscmem` (`rs_sglobe`, `:673-679`) and parses
   whatever header stands there — with `ap_rscmem == NULL` that is the m68k
   vector table, and every defined type still answers `1` with an address
   computed from it (r4 cases 598–614, 616–621, 626, 627: all 25 measured
   with `rs_hdr == NULL` on both machines, `int_out[0] = 1` on the TT).
   NewTOS: `0`/`0` for every defined type, FALSE from `rsrc_saddr`, no header
   parsed. The one answer that does **not** depend on the header — the
   unknown type's `-1L` — is given on both machines whether or not anything
   is loaded (cases 623–625 are word-for-word since A50).
3. **`rsrc_saddr` through a NULL slot** (`R_STRING`/`R_IMAGEDATA` on a NULL
   free-string / free-image entry). `get_addr` returns the slot's *content*
   (`:159-162`), i.e. 0; `rsrc_gaddr` reports it as found (`0 != -1`) — that
   half NewTOS matches since A50 — but `rs_saddr` then stores `rsaddr` **at
   address 0** and answers TRUE (`:735-738`). NewTOS answers FALSE and
   stores nothing.

Why the bound stays: between the AES and the application NewTOS has the
EL0/EL1 boundary classic TOS did not have. The resource lives in a pool
block of the process (since ARCH03 stage 1 at its own address, VA = PA) and
the AES reaches it through a kernel alias
(`tos-api/src/aes/resource.rs`, `Slot`); an address past the table names the
next section of the blob or, past its end, memory the blob does not own on
either side, and address 0 is mapped on neither, so `rsrc_saddr`'s store
would corrupt another table, fault at EL1, or scribble across the boundary —
and `rsrc_gaddr`'s pointer would be one the application may not be able to
dereference either. A conforming program never reaches any of the three: the
resource's own counts bound its indices, it loads before it asks, and it
re-points a free string through `R_FRSTR`, not `R_STRING`. The r4
measurement's 25 D-4 cases are all case 2 and stay a recorded difference by
this entry; they must be read as such, not as a defect.

What is **not** deviated, repaired by A50 in the same change: the return
rule. `int_out[0]` follows `ret = (addr != -1)`, so a NULL free-string /
free-image slot answers `1` with `addr_out[0] = 0` (it used to answer `0`),
and an unknown type answers `addr_out[0] = -1` at the caller's pointer width
(`0xFFFFFFFF` ILP32 — the TT's value — all ones LP64) with `0` (it used to
answer `0`/`0`; D-5 of `docs/analysis/aes-r4-subfunktionen-einordnung-2026-09-12.md`).
Covered by `tos-api/tests/aes_rsrc_gaddr_edges.rs` (unknown type with and
without a load, NULL slot, index past the table, absent table, nothing
loaded, `rsrc_saddr` on each).

**Resource-block ownership (A76, 2026-09-20).** Not a deviation, a contract
point with no prior entry: the blob `rsrc_load` returns is an ordinary
process-owned `Mxalloc` block (EmuTOS: `dos_alloc_anyram` =
`Mxalloc(_, MX_PREFTTRAM)`, `aes/gemrslib.c:775`, `util/gemdos.c:117-120`),
not a region `Mfree` must refuse — a program's direct `Mfree(ap_rscmem)`
succeeds on both machines, and only a *subsequent* `rsrc_free` then observes
the double free (`int_out[0] = 0`). NewTOS's `Kind::Rsc` refusal in
`usermem::free` (→ `EIMBA`) is accordingly a **defect**, not a sanctioned
choice; see `docs/api/gemdos/memory/49-mfree.md` § "AES resource block" and
`docs/api/aes/resource-fileselector.md` § `rsrc_free` / A39 callout item 8.


**`wind_set`'s `WF_NAME`/`WF_INFO`/`WF_NEWDESK` pointer accepts a native
`addr_in[0]` form in addition to EmuTOS's packed one; `WF_NEWDESK`'s root
index travels at a position that scales with the pointer's width, not a
fixed slot** — sanctioned 2026-09-20 with A81. EmuTOS's `wm_set` knows
exactly one convention: the tree/string pointer is packed as a raw LONG
across `int_in[2..3]` (`pinwds = &int_in[2]`, `references/emutos/aes/
gemwmlib.c:1623`), read back via `*(char **)pinwds`/`*(OBJECT **)pinwds`;
for `WF_NEWDESK` the desktop root index follows immediately at `pinwds[2]`
= `int_in[4]` (`gemwmlib.c:1685-1689`). Correct for the 68k ABI, where the
AESPB's `int_in` array holds 16-bit words and a pointer is always exactly
two of them.

NewTOS's `int_in` array holds the same 16-bit words for an LP64 caller, so
an 8-byte pointer does not fit in two words either — it needs four. Two
conventions coexist, discriminated by `control[3]` (`AIN_LEN`):

1. **Native** (`control[3] >= 1`): the pointer travels in `addr_in[0]`, the
   same slot every other AES pointer-class argument uses
   (`tos-api/src/aes/window.rs::wf_str_ptr`); `WF_NEWDESK`'s root travels in
   `int_in[2]`. This is what the native desktop sends
   (`kernel/src/desk/mod.rs::wind_set_newdesk`) and what gemlib's
   pointer-typed bindings (`wind_set_str`/`_ptr`/`_ptr_int`) are being moved
   to (overlay in `sdk/gemlib/overlay/`, a Developer task, not yet landed as
   of this note).
2. **Packed** (`control[3] == 0`): the pointer is packed at the caller's own
   width starting at `int_in[2]` — `int_in[2..3]` (2 words) for ILP32,
   `int_in[2..5]` (4 words) for LP64 (`tos-api/src/aes/mod.rs::intin_ptr`).
   `WF_NEWDESK`'s root sits in the word right after the packed pointer,
   `int_in[2 + ptr_words]` — `int_in[4]` for ILP32 (bit-exact with EmuTOS's
   `pinwds[2]`), `int_in[6]` for LP64 (`I_SIZE` is 16, in range).

Before A81, NewTOS's `set_newdesk` read the root at a fixed `int_in[4]` for
every caller width. For an LP64 caller that slot lies *inside* the packed
pointer's own bytes — bits 32..47 of the 8-byte value spanning
`int_in[2..5]` — and gemlib's `a_wind_set_ptr_int.c` writes the root exactly
there (`*(void**)(aes_intin+2) = v; aes_intin[4] = W3;`,
`sdk/gemlib/gemlib-0.44.0/a_wind_set_ptr_int.c:27-28`), so a non-zero root
silently overwrote the pointer's upper half instead of landing at its own
index. Measured: a tree at `0x0000_0001_0177_E4C0` packed with root 2
reassembles as `0x0000_0002_0177_E4C0` (`TASK/A81.md`). It went unnoticed
because every packed-form caller measured so far passes root 0 (TeraDesk,
`sdk/apps/teradesk/icon.c:2511`), and because EL0 addresses are currently
identity-mapped only in `0..4 GiB` on the stage-1 boot tables
(`kernel/src/arch/aarch64/el0.rs:16-18`), so the pointer's own upper half is
already 0 and a root of 0 leaves it unchanged either way.

Not a deviation from EmuTOS's field semantics: a packed-form caller at its
own pointer width, with the root at the position generalised above, gets
exactly the contract D-094 (`docs/api/aes/window-graphics.md`) documents for
EmuTOS — the ILP32 case is bit-identical to EmuTOS's fixed slot; the LP64
case has no EmuTOS equivalent to diverge from (68k TOS has no 8-byte
pointer). The native `addr_in[0]` convention is additive, not a replacement:
`wf_str_ptr` still accepts the packed form when `control[3] == 0`.

**Status as of this note:** this entry states the decided contract
(`TASK/A81.md`); the kernel fix (`tos-api/src/aes/window.rs::set_newdesk`
reading the width-scaled root position instead of a fixed `int_in[4]`) and
the gemlib overlay move to the native form are Developer work, open at the
time of writing — see `TASK/A81.md` for the current status.


**A `LONG` in `int_in`/`int_out`/`ptsin`/`ptsout` is 32 bits over two
WORDs, low word first — natively in place on a little-endian machine; a
pointer packed into one of those WORD arrays occupies the caller's own
pointer width (T04, 2026-09-21).** Not a deviation, a contract point with
no prior entry: one rule, not two, for every value wider than a WORD that
travels inside the AES/VDI WORD arrays. It is what EmuTOS does and what
NewTOS's readers and writers do (audit `docs/analysis/t04-lp64-intin-long-
contract-2026-09-21.md`; pinned by `tos-api/tests/aes_t04_intin_long_contract.rs`).

*The rule.* A value whose EmuTOS type at the access site is `LONG`/`ULONG`
(32 bits, `references/emutos/include/portab.h:119,123`) occupies **two** WORDs,
`int_x[n]` = bits 0..15, `int_x[n+1]` = bits 16..31, at every caller width.
EmuTOS spells that in two ways that coincide on its big-endian 68k and
coincide again, for the *other* reason, on little-endian AArch64: the
explicit form `MAKE_ULONG(int_in[1], int_in[0])` — `int_in[0]` is the
**low** word (`evnt_timer`, `aes/gemsuper.c:143`, `T_LOCOUNT`/`T_HICOUNT`
in `include/aesdefs.h:190-191`, `MAKE_ULONG(hi, lo)` itself `include/portab.h:139`;
`evnt_multi`, `:150`, `:220-221`) — and
the native form `*(LONG *)poutwds` at `&int_out[1]` and `&int_out[3]`
(`wind_get(WF_SCREEN)`, `aes/gemwmlib.c:1564`, `gemgsxif.c:125-129`). On a
little-endian machine a native 32-bit store at `&int_x[n]` leaves exactly
`[low, high]`, so the two forms are one contract: NewTOS reads
`evnt_timer`/`evnt_multi` as `int_in[n] | int_in[n+1] << 16`
(`tos-api/src/aes/event.rs`) and writes `WF_SCREEN` with one unaligned
32-bit store per LONG (`tos-api/src/aes/mod.rs::set_intout_long`, A57).
`WF_SCREEN` **stays LONG-class** — EmuTOS types it `LONG *`, the buffer's
user VA is under 4 GiB — it is not widened to a pointer for an LP64 caller
(that would move the length out of `int_out[3..4]` and break every
`mt_wind_get` reader, TeraDesk among them). The VDI has no LONG in
`intin`/`intout` today (EmuTOS neither); the rule covers `ptsin`/`ptsout`
for the NVDI extensions gemlib binds.

A value whose EmuTOS type at the access site is a **pointer** (`char **`,
`OBJECT **`: `wind_set`'s packed `WF_NAME`/`WF_INFO`/`WF_NEWDESK` form at
`int_in[2]` — `wm_set(…, &WM_IX)` with `WM_IX = int_in[2]`, `aes/gemsuper.c:346`,
`include/aesdefs.h:504`, read as `*(char **)pinwds`/`*(OBJECT **)pinwds`,
`aes/gemwmlib.c:1661,1687`; the MFDB pointers in
`CONTRL[7..]`, `gsx2.h:32-34`) occupies the caller's own pointer width —
`N_PTRINTS = sizeof(void *) / sizeof(short)` WORDs, four for LP64, two for
ILP32 and the 68k — and the declared count (`IN_LEN`, `N_INTIN`) counts
those WORDs (A81 above; `tos-api/src/aes/mod.rs::intin_ptr`/`ptr_words`,
`tos-api/src/vdi/mod.rs::contrl_ptr`). Where the opcode has no EmuTOS source
(NVDI 200-series, `x_*`/`fnts_*`/`pdlg_*` AES extensions), the type
criterion is gemlib's own accessor kind: `*_long` → LONG, `*_ptr` → pointer.

*Two consequences for the LP64 binding side.* (1) A binding's LONG accessor
**must be a 32-bit access**: gemlib's pristine `*(long *)(array + n)` is an
8-byte access on LP64 — its store spills the upper half over the two WORDs
after the LONG (past the end of an exactly sized local array in
`v_create_ctab`, `vst_track_offset`, `v_value2color`, `vst_arbpt32`, where
at `-O2` it overwrote the binding's own `control[0]` — the VDI received
opcode 0 — or the caller's frame), and its load takes the two WORDs after
the LONG as the upper half. `sdk/gemlib/overlay/gem_aesP.h`/`gem_vdiP.h`
define the accessors on a 32-bit, 2-byte-aligned, `may_alias` type
(`gem_wire_long`): one `str w`/`ldr w` at `&array[n]`, sign-extended on the
way out like a 68k `long`. (2) A native **8-byte read of a 32-bit store is
wrong**: at `&int_out[1]` after `wind_get(WF_SCREEN)` it yields
`address | length << 32`, not the address (measured, `aes_t04_intin_long_
contract.rs`). `sdk/gemlib/overlay/a_wind_get_ptr.c` reads `WF_SCREEN` as
the LONG it is; the pristine `mt_wind_get` (four WORD copies) was never
affected. cflib's `purec.c` reader of the same shape (`sdk/cflib/cflib-21/purec.c:166,171`) is
`__PUREC__`-only and not compiled into NewTOS's `cflib.a` (`purec.o` has no
symbols; cflib's `wind_get` *is* gemlib's `mt_wind_get`). Host guards:
`sdk/gemlib/tests/intin_long_words.c` (the four bindings, the `WF_SCREEN`
read, AddressSanitizer for the array bounds; pristine headers fail),
`sdk/gemlib/tests/ptr_word_count.c` (the six pointer-count overlays).

What this entry does **not** decide: the `vex_*` vector exchange in
`CONTRL[7..]`/`[9..]` (`tos-api/src/vdi/mouse.rs`, fixed 8 bytes at a fixed
index; EmuTOS two 32-bit slots, `vdi/vdi_mouse.c:372-373`) lies outside the
`intin`/`intout` arrays and has its own ticket, `TASK/T08.md`. TeraDesk's
`union { long size; struct { _WORD high, low; } words; }` around
`wind_get(0, WF_SCREEN, …, &buffer.words.high, &buffer.words.low)`
(`sdk/apps/teradesk/resource.c:233-241,307,315`) receives the correct low
32 bits under this contract; the upper 32 bits of its 8-byte `long` are the
application's own uninitialised stack, an app-side LP64 matter noted for
Pi observation in `TASK/T04-STATUS-DONE.MD`, not an overlay.


**Kernel-side AES message posts into a full queue are dropped; EmuTOS blocks
the poster** — narrowed 2026-10-22 with A116 from the A75 entry of
2026-09-20 ("`appl_write` on a full application-message queue returns 0"),
which is withdrawn: `appl_write` now blocks like EmuTOS's. `aqueue` parks a
write that does not fit (`qpb_cnt > QUEUE_SIZE - p_qindex`,
`references/emutos/aes/gemqueue.c:96-97,120-124`; `QUEUE_SIZE = 128`,
`aes/struct.h:56`) on the target's `p_qnq` until a read of that queue
services it (`:108-118`); NewTOS reproduces that for `appl_write`, including
the self-targeted case (`tos-api/src/aes/appl.rs` `appl_write`,
`tos-api/src/aes/pd.rs` `park_write`/`after_read`; contract in
`docs/api/aes/application.md` "Blocking behaviour (A75)"). What remains:

1. **The AES's own posters drop instead of blocking.** In EmuTOS every
   internal message goes through `ap_sendmsg` → `ap_rdwr(MU_SDMSG)`
   (`aes/gemctrl.c:84,125`, `aes/gemwmlib.c:904`, `aes/geminput.c:419`,
   `aes/gemmnlib.c:747`) and blocks the poster exactly as `appl_write` does —
   the Screen Manager posting `WM_TOPPED`, a window call posting `WM_REDRAW`,
   `mn_cleanup` posting `AC_CLOSE`. NewTOS's posters (`pd::post_msg`) run
   **inline** inside whatever AES call caused them — `wind_*`, the inline
   Control Manager in the application's own `evnt_multi`, `appl_exit` — and
   that call has already committed state (a window moved, a menu tracked) by
   the time it posts, so it cannot be aborted and re-executed (the S2
   restartable block needs a call that has committed nothing). A post into a
   full queue is therefore refused (`MsgQueue::enqueue` → `false`) and the
   message is lost; the posting call completes. `WM_REDRAW`/`WM_ARROWED`/
   slider posts still coalesce into a queued one first (`doq`,
   `gemqueue.c:44-72`), which keeps the common case below capacity. Reaching
   it needs an application that leaves eight messages unread while the AES
   posts to it; EmuTOS would stall the poster (often the Screen Manager, i.e.
   the whole GUI) there. Revisit when the AES's posters run in a context that
   can park (a Screen Manager process of its own, H02).
2. **A writer parked on an identity that ends is released with 1, its message
   discarded** (`pd::release` → `release_parked_writes`). NewTOS-only:
   EmuTOS's descriptors are never released — its accessories never end
   (`docs/api/aes/accessories.md` §4 point 6) and the chain's `D.g_int[0]`
   outlives every program (`aes/gempd.c:32-35`) — so a parked writer there
   always has a reader left. A NewTOS identity (a desk accessory that ends,
   A113; a chain slot that is released) takes its queue with it, and without
   this the writer would wait forever. The `1` is the value `aqueue` gives
   every serviced write (`gemqueue.c:117`); the message is discarded with the
   rest of the dead identity's queue.

`SCRENMGR` (pid 1) has no queue in NewTOS: a write to it returns 1 at once and
never blocks (EmuTOS's `ctlmgr` reads its queue on every pass,
`aes/gemctrl.c:445-448,480-483`, so a writer there also gets 1 after at most
one turn of the Screen Manager). What `ctlmgr` does with the message is not
reproduced (`tos-api/src/aes/appl.rs` `appl_write`).

**An application does not run while a drop-down menu is open** — sanctioned
2026-09-23 with A91 (found as a side observation of A73). EmuTOS's Control
Manager is a *process*: `gl_mowner = ctl_pd = iprocess("SCRENMGR", ctlmgr)`
(`references/emutos/aes/geminit.c:888`), and `ctlmgr` runs its own endless
`w_setactive(); ev_multi(…); wm_update(BEG_UPDATE); …` loop
(`aes/gemctrl.c:427-486`), so the menu track (`mn_do`'s state machine,
`aes/gemmnlib.c:530-690`) blocks `ctl_pd` and **not** the application: an
application waiting in `evnt_multi` keeps being dispatched while the drop-down
is on the screen and its `MU_TIMER` keeps firing (`tchange`,
`aes/gemflag.c:33-64`). NewTOS has no AES scheduler and no `ctl_pd` process;
the Control Manager runs **inline** in the application's own `evnt_multi` trap
(`tos-api/src/aes/event.rs` `menu_intercept` / `window::intercept` →
`tos-api/src/aes/menu.rs` `track`), so that call does not return until the
track ends — no `MU_TIMER`, no other arm. Since A73 the track also persists in
`OUTSIDE_STATE` (pointer between bar and box, `gemmnlib.c:415-443,559-585`),
which makes the stall arbitrarily long rather than momentary. Nothing about
the messages, their contents or their order changes; what an application can
observe is a timer-driven animation or clock standing still while a menu is
open. **Resolution path:** the cooperative kernel (milestone 2b,
`docs/analysis/aes-audit/00-MASTER.md`) turns the Screen Manager into a real
AES process, and the deviation ends by construction, with no separate fix.
`TASK/ACC03.md` — accessory process model, which already names A91 as a
dependency — is the ticket that reaches the decision first: an accessory is a
second `AESPD` parked in `evnt_multi` (`aes/geminit.c:654-745`), so ACC03 has
to settle whether the inline Control Manager can stay at all. Recorded in full
in `docs/api/aes/event.md` § "NewTOS deviation — the application does not run
while a drop-down is open".


**The VDI speaks EdDI: `EdDI` cookie and `vq_scrninfo`** — user decision,
2026-10-02 (E1–E9 of `docs/api/vdi/eddi-vq_scrninfo.md` §7, approved as
recommended; E3 = `0x02`). EmuTOS has no EdDI at all (spec §0: no hit for
`EdDI`/`scrninfo` in `references/emutos/`); NewTOS adds it so ported programs
can learn the 16-bit device word and fill device-format MFDBs themselves
(Doom, PORT06 "Weg 2"; zView, HighWire already ask). What differs from
EmuTOS, each observable:

1. **The jar has an `EdDI` cookie** (`0x45644449`), seeded at boot
   (`tos-api/src/cookie.rs` `seed_defaults`).
2. **Its value is a marker, not a dispatcher** (E2, option b):
   `0x00000001` — non-zero, odd. On TOS the value is the address of an m68k
   function dispatcher (opcode in `d0`, Pure C convention); that convention
   has no AArch64 mapping and the jar stores 32-bit values. The in-tree
   consumers only test the cookie's presence (zView `sdk/zview/z-tools-04805b7/zview/zvdi/vdi.c:28`,
   HighWire `sdk/apps/highwire/src/romvdi.c:70`); the probe `tosdump/core/scrninfo.c` does not call
   a zero or odd value. **A program that calls the dispatcher faults** —
   accepted until a callable trampoline exists (E2 option a, later). There is
   therefore no dispatcher opcode 0 and no reported EdDI version.
3. **VDI 102 reads `CONTRL[5]`** (EmuTOS ignores it, `vdi_main.c:77`):
   `CONTRL[5] == 1` **and** `INTIN[0] == 2` answer the 273-word
   `vq_scrninfo` table (E7). A program issuing that exact call without
   checking for the cookie got `INQ_TAB` on EmuTOS and gets the EdDI table
   on NewTOS (E8); every other 102 call is unchanged.
4. **The answer is the Falcon HiColor one**: format 2, software CLUT, 16
   planes, 32768 colours, 5/5/5 + 1 genlock bit, red bits 11..15, green
   6..10, blue 0..4, genlock 5, `[14] = 0x02`, `[5..7] = 0` (no accessible
   screen line width or address, E5). It equals word for word what the VDI
   Enhancer 1.00 (a real EdDI) reports on EmuTOS Falcon truecolor, except
   `[14]` (Enhancer: EdDI 1.00, no `[14]`, leaves 0) and the output counts
   (Enhancer leaves `CONTRL[2]/[4]` untouched; NewTOS writes 0 / 272) —
   `measurements/measurement-2026-10-02-eddi/`. The genlock bit is reported
   but programs are to leave it 0 (E9).

Implementation `tos-api/src/vdi/eddi.rs`; tests
`tos-api/tests/vdi_eddi_scrninfo.rs`. `v_opnbm`/`v_clsbm`/`v_open_bm`/
`v_resize_bm` are **not** provided (E1).

**Key auto-repeat runs in the polled input pump, not in the timer
interrupt** — user decision 2026-10-02 (variant (b) of
`TASK/PORT06-KEYS-DIAGNOSIS.MD`), landed as KEYS01 2026-10-03. EmuTOS
generates the repeat in the BIOS: `do_key_repeat` runs on every 4th timer-C
interrupt (`references/emutos/bios/ikbd.c:451-536`, `bios/vectors.S:521`) and
re-pushes the last key until its break (`bios/ikbd.c:815-817`). NewTOS keeps
those semantics — delay and rate from `Kbrate`'s `kb_initial`/`kb_repeat`,
`conterm` bit 1, only the last-pressed key repeats, Home/Insert breaks stop it,
re-conversion when the shift state changes — but runs the step in the input
pump right after each USB poll (`kernel/src/ikbd.rs` `poll_repeat`,
`kernel/src/key_repeat.rs`), crediting the 50 Hz steps `_hz_200` crossed since
the previous poll. **Reason:** the Pi's USB-HID boot keyboard reports changes
only, and its break reaches the decoder only when somebody polls; an
interrupt-driven repeat would go on queueing words while nobody polls and
deliver ghost keys after the release. **Observable difference:** one poll
emits **at most one** repeat (burst guard) — a program that pumps input less
often than every `kb_repeat` ticks sees the repeat rate drop to its poll rate,
and a long gap without any poll leaves no backlog, where EmuTOS would have
queued one word per `kb_repeat` ticks. Every key reader pumps (AES waits,
blocking reads, `Bconstat` on an empty queue), so in practice the cadence is
EmuTOS'. Spec: `docs/api/xbios/_state.md` §"Key-repeat handler",
`docs/api/xbios/23-kbrate.md` §"Pi / BCM2711 binding"; host tests in
`kernel/src/key_repeat.rs` (run via `tools/xtask`) and
`tos-api/tests/xbios_kbrate_repeat_source.rs`.

**The boot keyboard layout comes from `C:\KEYBOARD.KBD` or a `#K` line in
`C:\NEWDESK.INF` — a NewTOS extension** — user request 2026-07-02 (*"beides"*,
both mechanisms), `23ba4c38`; recorded here 2026-10-26, moved from the STATUS
log. EmuTOS picks the layout from its ROM's default country, overridden by the
NVRAM keyboard byte (`detect_akp`, `references/emutos/bios/country.c:133-163`,
the low byte of `_AKP` selects the table, `:95-106`); it has no layout file
(`KEYTBL.TBL` is listed as not supported, `bios/ikbd.c:20-23`). NewTOS has no
NVRAM. At boot, after the card is mounted, `keymap::init_from_disk`
(`kernel/src/keymap.rs:329-349`, called from `kernel/src/lib.rs:400`) loads
`C:\KEYBOARD.KBD` if it is a valid native layout file (magic `KBD1`, three
128-byte tables `norm`/`shft`/`caps` plus three AltGr pair groups, `load_kbd`,
`:381-441`; built from any EmuTOS `keyb_*.h` by `tools/mkkbd.py`); otherwise a
`#K us` or `#K de` line in `C:\NEWDESK.INF` picks one of the two built-in
tables, which are EmuTOS' `bios/keyb_us.h`/`keyb_de.h` verbatim; otherwise US.
Translation itself follows EmuTOS' `struct keytbl` and `ascii_cnv`, and XBIOS
`Keytbl` works as on EmuTOS. **Observable differences:** (1) the two files
above are read; (2) `_AKP` stays `0` (US) whichever layout is active
(`tos-api/src/cookie.rs:225`), where on EmuTOS its low byte names the active
keyboard. **Open conflict, reported 2026-10-26, not sanctioned:** EmuDESK's own
`NEWDESK.INF` already has a `#K` line — the 33 menu-shortcut bytes
(`references/emutos/desk/deskapp.c:967-974`) — and NewTOS' desktop reads and
writes it the same way since DESK05 (`09c940b2`;
`programs/desktop/src/inf.rs:472-476`, `:625-633`). `find_k_line`
(`keymap.rs:351-377`) takes the first `#K` line's first token: a desktop-saved
file yields a hex byte there and no layout, the desktop parses a hand-written
`#K de` as shortcut bytes, and *Save Desktop* writes only its own `#K` line.
`C:\KEYBOARD.KBD` is not affected.

**The IKBD vector table is per process, and a `kbdvec` hook ends with its
process** — user decision 2026-10-04 (D1/D8 of `docs/api/xbios/22-kbdvbase.md`),
landed as KEYS02. EmuTOS has **one** global `kbdvecs` table in BIOS BSS
(`references/emutos/bios/aciavecs.S:126-141`); `Kbdvbase` returns its address
(`bios/xbios.c:716-719`), and nothing in the BDOS touches it at process exit
(`bdos/proc.c:110-140,601-618`) — a hook a program forgot to restore stays
installed and the next key jumps into freed memory (spec A8). NewTOS keeps the
table in each process's own **private read-write page** (`0x7F000`, since
MEM12; `Kbdvbase()` returns `0x7F010`, measured:
`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b/XCPROBE.LOG`
V01; `tos-api/src/bdos/process/mmu.rs:259@PRIV_PAGE_VA`) and returns that
process's EL0 VA. **Reason:** a
hook is code of its process (the same rule as `vex_wheelv`); the table goes
away with the process, so a dangling hook cannot exist, and `free_slot` drops
any bytes still queued for it. (Since ARCH03 stage 1 every other user address
is shared by all processes; the table stays per process until after stage 3,
plan decision E4.)
**Observable differences:** (1) a hook left installed at `Pterm` simply stops
being called — strictly safer than EmuTOS; (2) a `Pexec` child starts with the
default table, a parent's hook does not see the child's keys (EmuTOS: global,
it would); (3) a TSR that installs a hook for later programs (`Ptermres`) cannot
work; (4) the hook is not called from an interrupt but as an EL0 upcall at the
running process's next return from a syscall, one byte per upcall in arrival
order; blocking key waits (`evnt_*`, `Cconin`/`Cnecin`/`Crawcin`/`Bconin(2)`)
and, on qemu-virt, the AUX wait (`Bconin(1)`/`Cauxin`, XBIOS18) give way and are
re-executed after the hook ran. Spec:
`docs/api/xbios/22-kbdvbase.md` Teil B/C; tests
`tos-api/tests/xbios_kbdvecs.rs`; measurement
`measurements/measurement-2026-10-04-keys02/`.

**One shared user address space, page 0 per process**
(ARCH03 stage 1, 2026-10-05; plan `docs/analysis/arch03-implementation-plan.md`,
decisions E1/E3 as recommended; E9 since 2026-10-05 EmuTOS' largest free
block, see below). Like EmuTOS — and FreeMiNT with memory
protection off — every user address means the same in every process: it is
its own physical address (both pools, published kernel structures, the
screen), `Malloc` returns `m_start`, `Pexec(PE_LOAD)` returns a real PD
pointer, `p_parent`/`p_env` are readable by anybody, and — with memory
protection off — no process is protected from another (stage 3b below). **Observable differences:**
(1) page 0 `[0, 0x1000)` is per process — the system variables, the OSHEADER
and its `os_run` cell (`0x5B8`, holding the process's own PD — a desk
accessory's holds EmuTOS's `run`, TASK/PROC03 B5 below), the cookie-jar
mirror and the `Keytbl` area — so a pointer *into page 0* means something
different in another process (on EmuTOS the area is global but
supervisor-only); page 0 is EL0 read-only outside the super state (next
paragraph); (2) `p_tbase` is `p_lowtpa + 0x1000`,
not `+ 0x100` (a position-independent image keeps its link-time page
offsets), and the TPA starts on a page: without memory protection the
less-than-a-page slack in front of the largest free block stays in the pool.
**The TPA is EmuTOS' largest free block** (ARCH03 E9, lead decision
2026-10-05, replacing the fixed 4 MiB of stages 1–3c): `alloc_tpa`
(`bdos/proc.c:460-502`) with its pool choice and bits 31-28 tie-break,
`p_hitpa = p + max`; the SDK start-up code shrinks it like mintlib's
`_crtinit` and the accessory loader like EmuTOS `pgmld`
(`util/gemdos.c:29-47`), so `Malloc(-1)` right after start reports the rest of
the pool, as on TOS.

**Page 0 is read-only outside the super state; `Super`/`Supexec` switch it**
(TASK/MEM12, user decisions 2026-10-26/27; spec `docs/api/_super-state.md`).
On the Atari, `$0..$7FF` is reachable in supervisor mode only; a program
writes a system variable after `Super()` or from a `Supexec` callback, and a
user-mode access is a bus error. NewTOS maps that onto the page tables: a
process has a **super state**, off at load, which `Super(0L)`/`Supexec` turn
on (`tos-api/src/bdos/process/superstate.rs:66@super_call`,
`kernel/src/arch/aarch64/exceptions.rs:1846@handle_super`,
`:1857@handle_supexec`). In super state page 0 is EL0 read-write, in user
state EL0 read-only, never executable
(`tos-api/src/bdos/process/mmu.rs:769@set_page0_writable`, with the TLB
maintenance ARM ARM `RGPPYH` requires); a write in user state ends the
program with "Bus Error" and the report line "page-0 system-variable memory"
(`kernel/src/arch/aarch64/exceptions.rs:1454@kill_crashed_program`). The
super state grants **nothing else**: the code still runs at EL0, memory
protection is unchanged. The EL0 stubs (upcall trampoline, `G_USERDEF`
callout, `kbdvec` stubs, OSHEADER reset stub) moved to one shared stub page
at `0x7E000`, EL0 execute-only; what a program may write — the `Kbdvbase`
table, the `G_USERDEF` `PARMBLK` — to its private read-write page at
`0x7F000` (`tos-api/src/bdos/process/mmu.rs:256@STUB_PAGE_VA`). The `kbdvec`
hook runs in super state, as EmuTOS calls it from the IKBD interrupt in
supervisor mode. The kernel reads page 0 only as documented input:
`conterm` (bits 0–3) and the `memvalid` magics of the reset entry.
**Observable differences:** (1) a user-mode **read** of page 0 does not fault
(on the Atari it does) — native programs read `_hz_200`, `_sysbase` and
`_p_cookies` directly; (2) `Super(stack)` does not switch the stack, an
explicit `stack` is ignored, and `Super(0L)` returns the caller's stack
pointer as its token (S4/S5); (3) a `Supexec` callback may make system calls;
(4) a desk accessory's `kbdvec` hook is never called — keys go to the
innermost started program's table (`tos-api/src/xbios/kbdvecs.rs:372@hook_owner`;
EmuTOS's one table is global), and the kernel logs a warning when an
accessory installs one; (5) a program built before MEM12 with the SDK's former
EL0 `Super`/`Supexec` macros keeps its reads, but its page-0 writes fault
until it is rebuilt. Tests `tos-api/tests/mem12_super_state.rs`,
`tos-api/tests/xbios_kbdvecs.rs`; oracle cases `tosdump/cases/super-r1.tsv`.

**`p_parent` and the environment as on EmuTOS, through two stand-in PDs**
(TASK/PPARENT, user decision 2026-10-06 "Way A"). EmuTOS's `run` while the
BIOS runs the AUTO folder is `initial_basepage`, while the AES initialises the
AES PD, and from then on the **accdesk PD** (the process `gem_main` makes with
the AES PD as parent, `aes/geminit.c:894`, `aes/gemshlib.c:570-581`) between
programs — the AES PD is *not* the parent of the desktop or of `#Z`/desktop
programs (`bdos/bdosmain.c:337-342`, `bios/bios.c:685-695,855-866`,
`aes/gemshlib.c:600-611,645`; **corrected 2026-10-18, TASK/PROC03**: this
paragraph named the AES PD). NewTOS runs the AES and the desktop in the kernel,
so `initial_basepage` and the AES PD exist as read-only stand-ins in a
published kernel page with EmuTOS's field contents, and so (since TASK/PROC03
stage 1, 2026-10-18) do the accdesk PD (one per AES session) and the EmuCON PD
(one per console start; `docs/api/gemdos/process/_parent-and-environment.md`
§2.3-2.4, §4.4). An AUTO program's
`p_parent` is the `initial_basepage` stand-in, a desktop / `#Z` / `shel_write`
program's the accdesk stand-in (whose parent is the AES PD), a program started
from EmuCON the EmuCON stand-in, an accessory's 0 (never `proc_go`ed,
`bdos/proc.c:370,541`);
every process gets an environment (`alloc_env`, `bdos/proc.c:413-432`; the
AES's is `default_env` `"PATH=\0X:\\\0"`, `bios/bios.c:626-636`), and
`shel_envrn` searches the AES-PD stand-in's. **Observable differences:** the
stand-ins are read-only to programs, never the running process (a program's
`*os_run` names itself; only a desk accessory reads a stand-in there — the
accdesk PD between programs, TASK/PROC03 B5 below), the AES, accdesk and EmuCON PDs have
`p_hitpa = p_lowtpa + 0x100` and `p_tbase = 0` (no TPA, no code in the user
address space; decisions D-1/D-2 in
`docs/api/gemdos/process/_run-ownership-curdir.md` §6.4). `Pterm` returns
through `p_parent` (`bdos/proc.c:609`); a value changed to something that is
not an ancestor on the launch chain returns to the recorded parent instead of
EmuTOS's crash (D-5).
Spec: `docs/api/gemdos/process/_parent-and-environment.md`.

**`PRGFLAGS` travel in an ELF note** (ARCH03 stage 2a, plan E8b, user
decision 2026-10-05). A NewTOS program is an ELF file, not a `0x601A` PRG, so
there is no `h01_flags` header word; the flags are a note `"NewTOS"`/type 1/one
`u32` under a `PT_NOTE` header, written by the SDK crt0 (default
`PF_STANDARD`) and changed with `cargo xtask prgflags` instead of
`PRGFLAGS.PRG`. What EmuTOS does with the flags is kept: they become `p_flags`,
`PF_TTRAMLOAD` chooses the pool of TPA and environment with `alloc_tpa`'s
ST/TT tie-break by the bits 31-28 quantum (`bdos/proc.c:283-313,413-502`), and
`PF_FASTLOAD` limits the clear to the image (`bdos/kpgmld.c:212-221`; since
E9). **Observable difference:** the format — a tool that patches the PRG
header finds no such header. Spec: `docs/api/gemdos/process/4b-pexec.md`
(NewTOS notes); tests `tos-api/tests/loader_prgflags.rs`,
`tos-api/tests/arch03_e9_tpa.rs`.

**Only an owner frees; a block can have several owners; resident memory stays
on the allocated list** (ARCH03 stage 2c/2d, plan E15 and §3.4/§3.8 as
recommended, FreeMiNT's model). EmuTOS frees a block for whoever names it
(`bdos/umem.c:135-159`), knows one owner per block (`m_own`) and drops a
TSR's MDs from both lists (`reserve_blocks`, `bdos/proc.c:75-89`). NewTOS keeps
`m_own` as the first owner and adds FreeMiNT's links (`sys/memory.c:512-660`):
a `Pexec` 4/6 child becomes an owner of its basepage and environment
(`sys/k_exec.c:518-520`), a mode-4 child shares its caller's memory space
(`FORK_SHAREVM`, plan E6); `Mfree` drops the caller's ownership and frees with
the last owner; `Mfree`/`Mshrink` by a non-owner and `Pexec` 4/6 of a PD the
caller does not own answer `EIMBA` (FreeMiNT's `EFAULT`, the same −40);
`Ptermres` hands every link to a TSR holder, so resident memory stays
allocated and owned. **Observable differences:** (1) freeing or shrinking
another process's block — or a TSR's — fails with `EIMBA` (EmuTOS: succeeds;
for a TSR's block EmuTOS answers `EIMBA` too); (2) a block shared with a
running child goes back to the pool only after both let go; (3) a TSR's
blocks stay on NewTOS's internal allocated list, owned by a sentinel
(EmuTOS: on no list). This is not visible through `Getmpb`: on both systems
it returns `mp_mal = NULL` and a single free MD (probe G02/G03,
`measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`), so no
list can be walked. Specs: `docs/api/gemdos/memory/
49-mfree.md`, `4a-mshrink.md`, `docs/api/gemdos/process/4b-pexec.md`,
`31-ptermres.md`; tests `tos-api/tests/arch03_region_owners.rs`,
`arch03_pexec_567_ptermres.rs`.

**Memory protection per process (FreeMiNT `PROT_P/G/S/PR/I`); page grain**
(ARCH03 stage 3b, plan §3.2–§3.4, decisions E5/E10/E16 as recommended).
EmuTOS has no memory protection; FreeMiNT's model is adopted
(`sys/arch/mprot030.c`, ADR-023): every pool block has a protection mode
(`Mxalloc` nibble with FreeMiNT's +1 encoding, else the program's `PRGFLAGS`;
the kernel's own blocks `PROT_S`), and each process reaches a block with
owner rights or non-owner rights. Default on; the boot argument
`memprot=off` gives exactly stage 3a (FreeMiNT `MEM_PROT=NO`),
`memprot=strict` FreeMiNT's `MPF_STRICT`. **Observable differences:**
(1) a program touching another process's `PROT_P`/`PROT_S` memory, writing
another's `PROT_PR` memory, or touching memory after `Mfree` ends with the
crash report (+ a `Memory violation:` line) — EmuTOS lets it; (2) a
`Pexec` 0/6 child keeps access to what its parent's space owned at its start
(not strict), a `Pexec` 4 child shares its caller's space, resident
(`Ptermres`) private memory becomes `PROT_G` — FreeMiNT's rules; (3) the
pools hand out whole 4-KiB pages: lengths are rounded up to a page and
every block starts on a page (FreeMiNT `QUANTUM`, `sys/mint/mem.h:152`), so
`Malloc(-1)`, `Getmpb` and `Mshrink`'s effective size show page multiples
(16 bytes with `memprot=off`). Specs: `docs/api/gemdos/memory/44-mxalloc.md`
(rights per process), `48-malloc.md`, `49-mfree.md`, `4a-mshrink.md`,
`docs/api/bios/_exception-crash.md` (memory protection); tests
`tos-api/tests/arch03_3b_memprot.rs`, probe `sdk/examples/mprot3b.c`.
Stage 3c: `memprot=strict` concerns `Pexec` 0/6 children only — a `Pexec` 4
child shares its caller's space in every mode (E6); a `G_USERDEF` routine of
another process runs in the current one with the owner's rights for the
callout (E10 (a), EmuTOS behaviour, `aes/gemoblib.c:411-426`, in every mode);
a process that cannot get a private page table keeps non-owner rights there
and is reported once on the serial line (`[memprot] page-table arena
exhausted …`).

**`Mxalloc` `F_KEEP` (`0x4000`)** (ARCH03 stage 3c). EmuTOS clears the bit
(`bdos/umem.c:241`); NewTOS honours it as FreeMiNT does
(`sys/dosmem.c:131-135,161-165,365-398`, `sys/proc_help.c:242-258`): the
block survives its last owner's termination, passes to the system and can
then be freed by any process. In every protection mode. Spec
`docs/api/gemdos/memory/44-mxalloc.md`, `49-mfree.md`; tests
`tos-api/tests/arch03_3c_keep.rs`.

**No fixed memory-descriptor limit; a per-process limit instead** — user
decision, 2026-10-13 (MEM05; analysis `docs/analysis/mem05-md-pool-2026-10-13.md`,
design `docs/analysis/mem05-design-2026-10-13.md`, follow-up decision B in
`TASK/MEM05.md`; `docs/MEMORY.md` ADR-025). EmuTOS draws its memory
descriptors from a fixed OS pool of 118 blocks, at most 354 MDs for the whole
system, shared with directory and file structures (`bdos/osmem.c:36-39,
62-64,104-109`); Atari TOS 1.0 allowed 20 `Malloc` blocks per process and
1.04 replaced that by the shared pool (`doc/tos14fix.txt:822-833`). NewTOS
has **no fixed MD count**: the MD supply grows page by page on demand from
kernel memory, and a **per-process** limit — default 8192 allocated blocks,
boot argument `mdlimit=<n>`, `0` = off — takes its place. The MD pages and
the kernel's per-block tables (protection records, the block snapshot of the
protection sync, `F_KEEP` flags) share one budget of 512 pages (2 MiB of
kernel memory); that budget is the remaining system-wide bound — roughly
13 000–17 000 live blocks in total with memory protection on, 30 000–60 000
with `memprot=off`. When a process reaches its limit, or the budget is
reached, `Malloc`/`Mxalloc` return 0, `Mshrink` returns `ERROR`, `Maddalt`
and `Pexec` return `ENSMEM`, exactly as EmuTOS does on exhaustion of its
pool; `Malloc(-1)` still reports the largest free block, as on EmuTOS. The
kernel, memory left resident by `Ptermres` and `F_KEEP` blocks the system
inherited are not limited per process (they count against the budget).
**Observable differences:** (1) a program can hold far more blocks than any
TOS (thousands, not 20 or ~350); (2) the number of blocks `Mxalloc(1, …)`
can take until it fails is no longer a system constant: it is the process's
remaining allowance, or what the budget or free memory allows; (3) with
memory protection on, each allocation and free costs time proportional to
the number of live blocks in the system (the protection sync rebuilds its
block snapshot; known limit, ticket MEM10). The MD structure, the
`mp_mfl`/`mp_mal` chains and `Getmpb` are unchanged. (4) The SDK half
(MEM05 part B): mintlib programs built with the NewTOS SDK ask the OS for heap
in hunks that keep doubling up to 16 MiB instead of stopping at 32 KiB
(`sdk/libc/overlay/mintlib/malloc.c`, `docs/sdk/07-mintlib-limits.md` §10),
so a large heap needs tens of blocks, not thousands; a program calling
`_mallocChunkSize()` is unaffected.

**No DND tree, no OS memory pool** — user decision, 2026-10-29 (TOOL17
group C, `docs/MEMORY.md` ADR-033). EmuTOS caches directory nodes per drive
(`references/emutos/bdos/fsdir.c`) and draws them, file structures and MDs
from its fixed OS pool (`references/emutos/bdos/osmem.c:92-100`). NewTOS names
a directory by its path and keeps the current directory as a path string per
drive and process (`tos-api/src/bdos/fs/curdir.rs`); BDOS-internal blocks
are ordinary kernel allocations. **Observable differences:** OS-pool
exhaustion and the folder limit that follows from it ("too many folders"
after deep trees) do not occur; behaviour that depends on a directory node's
identity rather than its path may differ and is measured case by case
before it is accepted (e.g. TASK/FS24).

**Dragging several desktop icons processes all of them** — user decision,
2026-10-17 (DESK48, option b). EmuTOS' `fun_file2any` calls
`desk_clear(DESKWH)` inside the `win_isel` loop of
`fun_desk2desk`/`fun_desk2win` (`references/emutos/desk/deskfun.c:1187`), so
after the first icon nothing is selected any more and only the **first** of
several dragged desktop icons is copied/moved. NewTOS keeps processing every
selected icon and deselects once after the loop
(`programs/desktop/src/dir.rs`). Desktop-internal behaviour, no interface: no
program can observe it.

**`Cconrs` from a redirected stdin stops at `max` read unsigned** — user
decision, 2026-10-18 (TASK/PROC04 point 1, "Anwenderentscheidungen zu Punkt 1").
With stdin forced to a file, EmuTOS' `osif` answers `Cconrs` itself and reads
the limit as `max = *pb2++` — a plain, signed `char` into an `int` — then loops
`while (max--)` (`references/emutos/bdos/bdosmain.c:535-553`, `:537-539`). A
`max` of 128..255 is negative there, so only a CR or the end of the file stops
the loop, and the bytes run past the caller's record. NewTOS reads `max` as
the unsigned byte 0..255 and stops after that many bytes
(`tos-api/src/gemdos/console.rs`, `redirect_to_file`, the `0x0A` arm). For
`max` <= 127 both are identical; above that NewTOS never writes beyond the
record. Spec `docs/api/gemdos/_dispatcher.md` §5,
`docs/api/gemdos/console/0a-cconrs.md`; test
`tos-api/tests/proc04_console_redirect.rs` `a_cconrs_max_is_unsigned`.

**A file deleted while open no longer reaches the card through its
handles** — user decision, 2026-10-18 (TASK/PROC04 point 5, "Anwenderentscheidung
Punkt 5", variant b). `Fdelete`, `Fcreate` over an existing file and `Ddelete`
run EmuTOS' `ixdel` (`references/emutos/bdos/fsopnclo.c:652-699`) unchanged in
everything a program can ask for: a file open in **another** process is refused
with `EACCDN` (`:667-668`, without rolling back closes already done, `:660-669`);
a file open by the **caller** is `ixclose`d (`:665-666`, `:511-581`), deleted,
and the call returns `E_OK`, while the caller's handle and every `Fdup`/`Fforce`
alias stay valid and a later `Fclose` returns `EINTRN` and frees the entry
(`:552-566`, `:484-496`). EmuTOS then keeps working on the OFD's remembered
clusters: `Fread`/`Fwrite` go to a chain the FAT lists as free (another file may
own it by then), and a dirty OFD's `Fclose` rewrites the directory slot,
whatever entry holds it by then (`:530-549`). NewTOS marks every OFD on the
deleted entry and lets nothing done through it touch the card
(`tos-api/src/bdos/fs/file.rs`, `ixdel`, `Ofd::deleted`). **Observable
differences**, all only on a handle whose file was deleted while open:
(1) `Fread` returns 0 (EmuTOS: the bytes of the freed clusters) and does not
move the position; (2) `Fwrite` of one or more bytes returns `EACCDN` (EmuTOS:
the byte count, written into the freed clusters); zero bytes still return 0;
(3) `Fseek` returns the target position or `ERANGE` from the OFD alone, exactly
as `ixlseek` bounds it (`references/emutos/bdos/fsio.c:360-361`) — EmuTOS may
answer `EINTRN` instead, because it walks the freed chain (a freed entry reads 0
and the walk ends on the media word); (4) `Fdatime` reads and sets the time and
date the entry had at the delete, kept with the OFD — as EmuTOS' `o_td`
(`references/emutos/bdos/fsdir.c:684-707`) — but a set is never written to the
card; (5) `Fclose` returns `EINTRN` like EmuTOS but writes no directory entry
and flushes no buffers. Redirected character I/O over such a handle follows (1)
and (2). Unchanged: `Fdelete`/`Fcreate`/`Ddelete` results, `eof` (Cconis on a
redirected stdin) from the OFD's position and length. One addition on the delete
itself: an OFD the owner check does not see (a handle shared through
`Fforce`/`Fdup` after its first close) has its pending length/start cluster
written back before the chain is freed, so no cluster is left allocated. Spec
`docs/api/gemdos/file/41-fdelete.md` edge case 3 and "PROC04 point 5"; tests
`tos-api/tests/proc04_fdelete_open.rs`.

**A file moved to another directory while open stays reachable through its
handles** — TASK/FS18, 2026-10-19, from the user-approved FS18 assignment ("a
cross-directory move keeps any open OFD consistent with where the entry now
is"), confirmed by the user 2026-10-19 (same line as `Fdelete` variant (b):
visible behaviour as EmuTOS, without the data loss). `Frename` itself is EmuTOS' `xrename`
(`references/emutos/bdos/fsdir.c:895-1101`): in the **same** directory it only
overwrites the name in place (`:1076-1083`), so an open file's OFD still names
the live entry and nothing differs. **Across** directories EmuTOS makes a new
entry (`ixcreat`, `:1014`), erases the old one (`:1021-1027`) and leaves every
OFD open on the file pointing at the erased slot (`o_dirfil`/`o_dirbyt`): the
`ixclose` of a handle written after the move stores its start cluster and length
in the erased slot (`references/emutos/bdos/fsopnclo.c:530-549`), while the
moved entry keeps the length (and, for a file that was empty, the start
cluster 0) it had at the move — the data written afterwards is lost and its
clusters leak. NewTOS points such OFDs at the new entry
(`tos-api/src/bdos/fs/file.rs`, `move_ofds`). **Observable difference**, only
for a file moved across directories while open and then written, or its
`Fdatime` set, through such a handle: the moved entry gets the length, start
cluster and time/date the handle leaves (EmuTOS: those of the move, the rest
lost). `Frename`'s results, and everything for a closed file or a
same-directory rename, are unchanged. Spec
`docs/api/gemdos/file/56-frename.md` edge case 9 and "NewTOS implementation
notes"; tests `tos-api/tests/fs18_frename.rs`
(`cross_dir_rename_of_open_file_follows_the_entry`).

**A sector whose write-back fails stays in the cache, dirty** — TASK/HW03,
2026-09-27 (`f3a468fb`), from the ticket's acceptance rule *"Kein Sektor darf
bei fehlgeschlagenem Rueckschreiben aus dem Cache verschwinden"*
(`TASK/HW03.md`, "Abnahme"); recorded here 2026-10-26, moved from the STATUS
log. EmuTOS' `flush` sets `b_bufdrv = -1` before the write ("invalidate in case
of error", `references/emutos/bdos/fsbuf.c:93`), and when the failed write
`longjmp`s back into `osif`, the error branch invalidates every buffer of the
drive (`mark_bcbs_invalid(errdrv)`, `bdos/bdosmain.c:490`). The buffer cannot
travel with the jump, so the only up-to-date copy of the sector is dropped — a
consequence of the jump, not an integrity choice. NewTOS does not jump:
`write_back` keeps the slot's data, `valid` and `dirty` flags on failure, and
eviction does not hand such a slot out (`tos-api/src/bdos/fs/cache.rs:225-243`).
The error still reaches the caller as on EmuTOS (`Fwrite` → `EWRITF`, `Fread` →
`EREADF`, `Rwabs` → the driver's code). Automatic retries are bounded:
`WRITEBACK_ATTEMPTS = 3` per slot, a policy value, not a specification one
(`cache.rs:90-102`; EmuTOS retries once, `RWABS_RETRIES = 1`,
`bios/blkdev.h:34`); after that the slot stays dirty and every later `flush`
reports the stored error without touching the device, until the sector is
written again. **Observable difference**, only after a device write error: a
later `cache::flush()` (the file and directory calls that write, e.g.
`tos-api/src/bdos/fs/file.rs`, `dirops.rs`) can still land the sector, where
EmuTOS would read the old one back from the card, and the clean buffers of that
drive survive. Tests `tos-api/tests/gemdos_fs_device_error.rs`;
run `measurements/measurement-2026-09-27-hw03/`.

**A media change restarts a call at most three times** — user decision,
2026-10-29 (TASK/FS22, work item 3: *"Wiederholungsgrenze: EmuTOS startet
unbegrenzt neu. NewTOS begrenzt (z. B. 3), danach `E_CHNG` … (laut statt
Haenger)"*). EmuTOS' `osif` logs a drive in again after a media change and
restarts the call with `goto restrt` (`references/emutos/bdos/bdosmain.c:
455-486`) — every time, with no count: a call that meets a change on every run
never returns. NewTOS runs the same re-login and restart
(`tos-api/src/gemdos/mod.rs:201@restartable`, used by the GEMDOS dispatcher
and by the kernel's own file-system paths, `docs/api/gemdos/_state.md` §4a),
but restarts one call at most `RESTART_LIMIT` = 3 times
(`tos-api/src/gemdos/mod.rs:170@RESTART_LIMIT`, a policy value like
`WRITEBACK_ATTEMPTS` above); a run that still meets a change after that gets
the re-login and then `E_CHNG` (-14), the code EmuTOS returns when the
re-login's `Getbpb` fails (`:471-476`). **Observable difference**, only where
EmuTOS hangs: a single change needs one restart (the re-login's `Getbpb`
disarms `forcechange`, and no BDOS path issues an `Rwabs` that could arm it
again), so the bound is reached only when the medium changes again during
every restarted run, or a driver keeps answering "changed" after `Getbpb`.
There EmuTOS loops for ever; NewTOS reports the change. Test
`tos-api/tests/fs22_media_change_kernel_paths.rs`
(`the_restart_gives_up_with_e_chng_after_restart_limit_restarts`); run
`measurements/measurement-2026-10-29-fs22/RUNINFO.TXT`.

**A desk accessory's GEMDOS calls act on the accessory; only `*os_run` and
the `p_parent` of its children follow `run`** — user decision, 2026-10-18
(TASK/PROC03 B5, option 2, "Anwenderentscheidung B5"; kernel memory follows
FreeMiNT, `docs/MEMORY.md` ADR-027). EmuTOS never starts an accessory as a
GEMDOS process — it is `PE_LOAD`ed and entered by a plain `jmp` from an AES
context (`references/emutos/aes/gemasm.S:83-110`) — so every GEMDOS call it
makes executes with `run` unchanged: the innermost started process, i.e. the
program in front, that program's running `Pexec` child, or the accdesk PD
between programs (`docs/api/gemdos/process/_run-ownership-curdir.md` §1.2,
§1.3). Everything GEMDOS keeps in the PD is then `run`'s: the `Malloc` owner
(`bdos/iumem.c:134`, `m_own = run`) and pool choice (`bdos/umem.c:120`), the
owner of files it opens (`bdos/fsopnclo.c:348`, `bdos/fshand.c:107`; closed
at `run`'s `Pterm`, `bdos/proc.c:121-124`) and the `Fdelete` owner test
(`bdos/fsopnclo.c:665`), the standard handles (`bdos/console.c:161-535`), the
DTA (`bdos/fsmain.c:396,405`), the current drive and directory
(`bdos/fsmain.c:420-441`, `bdos/fsdir.c:1124-1145`), what a child inherits
(environment, handles, directories, `bdos/proc.c:391-409,419-420`), and an
accessory's own `Pterm` ends `run` (`bdos/proc.c:604-617`). NewTOS builds the
two parts that only *name* `run`: `*os_run` read by an accessory is EmuTOS's
`run` (its page-0 cell is rewritten every time it gets the CPU,
`tos-api/src/bdos/process.rs` `set_current`/`emutos_run`; the accdesk
stand-in between programs, the EmuCON stand-in while EmuCON runs), and a
process it `Pexec`s gets `p_parent = run` (`bdos/proc.c:541`) while that
child's `Pterm` still resumes the accessory. Everything else stays with the
accessory itself: memory ownership and pool choice (its own `p_flags`),
protection (its rights, and those its children inherit, E5), the current
drive and directory, the DTA, the standard handles, file ownership, the
environment, handles and directories its children copy, and its own `Pterm`
ends the accessory. E15 (only an owner frees) is unchanged. **Observable
differences:** (1) memory an accessory allocates while program P is in front
is the accessory's and survives P's `Pterm` (EmuTOS: freed with P — the
pitfall `wait_for_accs(AP_MESAG)` and `ap_exit`'s `AP_ACCLOSE` wait mitigate,
`aes/geminit.c:569-598`, `aes/gemaplib.c:251-261`); an accessory's `Mfree` of
a block P owns answers `EIMBA` (E15; EmuTOS: freed); (2) an accessory's
`Dsetdrv`/`Dsetpath`/`Fsetdta`/`Fforce`/`Fopen`/`Fcreate` change the
accessory's state, not P's, and a file it opens is closed only by its own end;
(3) its children inherit the accessory's environment, handles, directories
and rights, so with memory protection on such a child cannot read the PD its
`p_parent` names unless that PD's block is readable to it (P's TPA mode;
see the memory-protection entry above); (4) an accessory's
`Pterm` ends the accessory, never P; (5) if P ends while a child the
accessory started still lives, `*os_run` names that child and then the
stand-in, never P's freed PD (EmuTOS: `run` names the dead P after the
child's end). Spec `docs/api/gemdos/process/_run-ownership-curdir.md` §6.6,
`_parent-and-environment.md` §4.4; test
`tos-api/tests/proc03_b5_accessory_run.rs`; QEMU probe
`measurements/measurement-2026-10-19-proc03-b5/`.

**Without a screen, or without a loadable desktop, the shell's desktop arm
runs the console** — the one named deviation of the AES shell loop (`sh_main`/
`sh_ldapp`, `47b29a96`, 2026-09-03), extended by DESK43 D6-c/D6-d (user
decision 2026-10-14, `TASK/DESK43.md` "U9-Vorbereitung entschieden"); recorded
here 2026-10-26, moved from the STATUS log. EmuTOS decides at **build** time
which user interface a machine has: `exec_os` is the AES entry `#if WITH_AES`,
EmuCON `#elif WITH_CLI`, otherwise `#error You must provide a main UI`
(`references/emutos/bios/endrom.c:28-35`), because every Atari has a screen.
On NewTOS a board may come up without a raster backend, so the same choice is
made at **run** time, in one place: `sh_ldapp`'s `DESKTOP_APP` arm
(`kernel/src/shell_loop.rs:427-454`, module note `:23-31`). `have_graphics()`
(`:321-325`) is EmuTOS' `WITH_AES` as a runtime fact; `go_graphic` and
`sh_chgrf` clamp the graphics switch to it (`:300-319`, `:400-414`), and with
no screen the arm reports it on the serial line and runs the console arm
(EmuCON, `endrom.c:33` `coma_start`). D6-c/D6-d: a desktop program (`NEWDESK`)
that fails to **load** is not retried this boot and the console runs instead
(`:455-466`, `:487-501`) — a ROM desktop cannot fail to load, so EmuTOS has no
counterpart. **Observable difference:** none on a machine with a screen and a
loadable desktop; there every arm follows `gemshlib.c:592-689`, and a missing
`#Z` program or an `.INF` without `#Z` leads to the built-in desktop as on
EmuTOS. Spec `docs/api/aes/shell-loop.md`.

**Double-clicking a file no application claims shows nothing; there is no
built-in file viewer** — user decision, 2026-09-24, confirmed 2026-10-18
(*"es macht Sinn, eine App als Betrachter zu haben, damit ist man etwas
flexibler"*). The reference build has `CONF_WITH_SHOW_FILE` = 1
(`references/emutos/include/config.h:1838-1844`, `docs/api/_emutos-config.md`):
EmuDESK then shows such a file in its own text view. NewTOS does not build that
view. A viewer is an ordinary application with an extension assignment
(e.g. `.TXT` → QED), the same mechanism as for any other document, and
replaceable. **Observable difference:** without an assignment the double-click
does nothing. EmuCON's `more` is a separate question ([CON03](../TASK/CON03.md)).
Ticket [GUI01](../TASK/GUI01.md).

**The VT52 console (`Bconout(2)`) — four named deviations** — user decisions
2026-10-22 (`TASK/CON04.md` "Entscheidungen", spec `docs/api/bios/_vt52.md`
§13). Everything else of the console is EmuTOS' (`bios/vt52.c`,
`bios/conout.c`): every sequence, the swallowed unknown escapes, the
clamping, the cursor as an inverted cell with `disab_cnt`, wrap off at boot,
non-destructive BS, clamping HT, LF/VT/FF without CR, Handle 5 without the
automaton.

* **Q3 — erasing under reverse video uses the background throughout.**
  EmuTOS' `erase_to_eol_impl`/`erase_from_bol_impl` clear the border cell
  with `ascii_out(' ')` because `blank_out` needs an even start and odd end
  column on interleaved bit-planes (`bios/vt52.c:471-500,752-775`,
  `bios/conout.c:586-596`); under `ESC p` that cell is painted in the
  *foreground*. Checked on the reference (pinned EmuTOS on Hatari Falcon
  truecolor, `measurements/measurement-2026-10-22-con04/`, probe rows 2–7):
  with reverse video on, `ESC K` at an odd column and at the last column and
  `ESC o` at an even column (and column 0) leave one cell in the foreground,
  `ESC K` at an even and `ESC o` at an odd column do not. The pairing is an
  artefact of the bit-plane layout, not a feature; NewTOS erases the whole
  run — `ESC K`, `ESC o`, `ESC J`, `ESC d` — with the background register.
  **Observable difference:** that one cell, only with `ESC p` active.
* **Q5 — the cursor blink runs on the 200 Hz counter.** EmuTOS counts
  `v_period` (default 30) in VBLs (`blink()` per VBL, `bios/vectors.S:355`),
  so the default lasts 0.43–0.6 s depending on the monitor's frame rate — a
  device parameter. NewTOS has no VBL; it counts one `v_period` unit as
  1/60 s on `_hz_200` (EmuTOS' own comment on the default, "0.5 second blink
  rate (@ 60Hz vblank)", `vt52.c:933`), so the default is exactly 0.5 s on
  every board. `Cursconf(4)`/`Cursconf(5)` set and return `v_period` as in
  EmuTOS. **Observable difference:** the blink period in real time.
* **Q6 — `vs_curaddress` sets the position directly.** EmuTOS sends
  `ESC Y` with `0x20 + INTIN - 1` cut to a byte (`vdi/vdi_esc.c:177-182`),
  so no column beyond 223 is reachable and out-of-range values wrap. NewTOS
  has up to 240 columns (1920 pixels); `vs_curaddress` calls the console's
  `move_cursor` with `INTIN - 1`, which clamps to the screen. The `ESC Y`
  stream sequence itself stays byte-coded as in EmuTOS. **Observable
  difference:** columns 224..239, and clamping instead of wrap-around for
  values outside the screen; also, a half-received escape sequence in the
  console is not disturbed by `vs_curaddress`.
* **D4 — BEL makes no sound.** `do_bell` evaluates `conterm & 4` as EmuTOS
  does (`vt52.c:213-218`); EmuTOS' `bell()` calls `bell_hook`, whose default
  sounds only with a YM2149 (`bios/sound.c:251-261`). NewTOS has no audio
  device and no `bell_hook` system variable, so nothing sounds — the same
  result as an EmuTOS build without `CONF_WITH_YM2149`. **Observable
  difference:** no bell, and a program cannot hook the bell through
  `bell_hook` ($5AC).

**OS hooks installed by programs: most are not served, never silently** —
user decision 2026-10-28, ADR-031 (`docs/MEMORY.md`); built by TASK/BIOS15
(`tos-api/src/bdos/process/hooks.rs`). On TOS a program can
hang code into OS vectors that the OS later calls (`etv_*`, `hdv_*`,
`Kbdvbase`, `vex_*`, trap/CPU vectors, …;
`docs/analysis/os-hooks-adr-proposal-2026-10-28.md`). NewTOS serves
`kbdvec`, `etv_term` (installed through `Setexc(0x102)`), `vex_wheelv`,
`Supexec` and `G_USERDEF`; every other hook is stored (by `Initmous`,
`Mfpint`, `Xbtimer` not even that) but never called. `Mfpint` and `Xbtimer`
answer what EmuTOS leaves in `d0` — the MFP interrupt mask register after the
call, or `Xbtimer`'s `control` for a timer outside 0–3 — from an image of the
two mask registers as the oracle has them while a program runs (IMRA 0x14,
IMRB 0x60; user decision 2026-10-29, `docs/api/xbios/0d-mfpint.md`);
`Initmous` still answers its function number. The first installation per hook and process writes one line
`[hook] warning: slot <n> (PD …) installed <hook> = <value> via <where>;
NewTOS never calls it: <reason> (ADR-031)` to the serial log. Where it is
seen: in `Setexc` (`etv_timer`, `etv_critic`, vectors `< 0x100`); in page 0
(`hdv_rw`/`hdv_bpb`/`hdv_mediach`, `swv_vec`, `exec_os`, `resvector`,
`dump_vec`, `vblqueue`, `bell_hook`/`kcl_hook`, and `etv_*` or CPU vector
cells written directly), read only and compared with the loader's value when
the process leaves the super state, **after every trap it makes in super
state** (TeraDesk's `force_mediach` hooks `hdv_*` around one `Fopen` and
restores them before it leaves), when it ends in super state, and after a
`Setexc` into page 0; for the `Kbdvbase` slots `mousevec`, `statvec`,
`clockvec`, `joyvec`, `midisys`, `ikbdsys` — and a desk accessory's
`kbdvec` — at the first input event that would call them on EmuTOS; for
`Initmous` (types 1, 2, 4 with a vector), `Mfpint`, `Xbtimer` and
`vex_butv`/`vex_motv`/`vex_curv`/`vex_timv` in the call. No warning for hooks
EmuTOS itself never calls (`etv_xtra`, `hdv_init`, `hdv_boot`,
`prt_stat`/`prt_vec`, `aux_stat`/`aux_vec`), nor for `midivec`, `vkbderr`,
`vmiderr`, whose events (MIDI bytes, receiver overruns) never occur.

**Owner of a hook in a shared cell** (ADR-031 rule 3). The `Setexc` vectors
up to `etv_xtra` and `vex_butv`/`motv`/`curv`/`timv` are one cell for all
processes; the process that wrote it owns it. When it ends without
unhooking, the cell gets its previous value back if it still holds the
process's hook (top of the XBRA chain); if another hook has been chained on
top, the cell keeps it and the log gets `[hook] warning: … ended without
unhooking …`. After `Ptermres` the hook stays, as the code it points at
stays resident. EmuTOS gives nothing back: a vector a terminated program
left points into freed memory there.

**`prt_stat`/`prt_vec`** (`$506`/`$50A`) hold a `ret` stub, EmuTOS' `just_rts`
(`references/emutos/bios/chardev.c:82-83`), not 0; `aux_stat`/`aux_vec`
are still 0.

**Observable difference:** a RAM disk or disk cache hooked into `hdv_*` does
not take over a drive (NewTOS GEMDOS uses the block layer directly);
TeraDesk's `force_mediach` has no effect; `etv_timer` and the `Kbdvbase`
mouse/status/clock/joystick hooks are never called (accepted deviations
BIOS-CORE-002, XBIOS-CHARDEV-003); a hook a terminated program left in a
shared cell is gone when nothing was chained on top of it.

**`Iorec`: the key queue is the IKBD `IOREC`, in a page every process may
write; three named limits** — user decisions E1-E6 of 2026-10-28, TASK/XBIOS01
(`docs/api/xbios/0e-iorec.md`, "Entscheidungen" and "NewTOS implementation").
As on EmuTOS the record `Iorec(1)` returns **is** the console key queue: a
program flushes the keyboard with `io->ibufhd = io->ibuftl`, entries are the
32-bit `shifty<<24|scancode<<16|ascii` with the shift state of the key press.
The records are laid out as the SDK's LP64 `_IOREC` (24 bytes, device 0 56), not
the 14-byte m68k form (native vs m68k struct variants, below).
**Observable differences:** (1) keys reach the ring only at a trap-side input
pump — a program that polls `ibuftl` and never makes a trap sees no key (E3 (a);
EmuTOS fills it from the ACIA interrupt); (2) a buffer or size a program writes
into a record is not adopted: at its next use, at the latest after the next
trap, the kernel restores its own buffer, empties the ring and logs one
`[iorec] warning` per process (E4 (a); EmuTOS adopts it,
`bios/aciavecs.S:331`); indices outside the ring empty it likewise; (3) the
records of devices 0 (RS-232) and 2 (MIDI) exist with the oracle's values;
MIDI stays empty — no driver fills it (E5 (a)). Device 0's input ring is
filled from the serial line on qemu-virt and read by AUX (`Bconstat(1)`,
`Bconin(1)`), as EmuTOS' `push_serial_iorec` (TASK/XBIOS15, measured equal
to the oracle); on real Pi 4 serial input is dropped and the ring stays
empty (UART0 is the debug console there). Its output ring stays empty on both
boards: AUX output is discarded at once. The survey behind E3/E4 found no
ported program that calls `Iorec` (`docs/analysis/iorec-survey-2026-10-28.md`).

**qemu-virt: AUX output is discarded** — user decision 2026-10-29
(TASK/XBIOS17.md). Since XBIOS15, AUX *input* on qemu-virt reads device 0's
IOREC ring, but `Bconout(1)`/`Bcostat(1)` still go to the no-cable driver
(`tos-api/src/bios/chardev.rs:120@AuxNoCable`): output is dropped. The UART is
shared with the kernel log (`kernel/src/console.rs:106-111@SerialConsole`), and
qemu-virt is a development board only; raspi4b keeps AUX as "no cable".
**Reopen** when a port writes to AUX.

**`Bconmap` MAPTAB function pointers are not callable from programs** —
user decision 2026-10-28 (TASK/XBIOS05.md). EmuTOS calls the six pointers of
a `MAPTAB` entry itself for handles 6 and up
(`references/emutos/bios/bios.c:928`, `:972`, `:1005`, `:1157`; layout
`references/emutos/bios/serport.h:102-117`). No ported program reads
`maptab` or calls through it (`docs/analysis/abi-assumptions-check-2026-10-28.md`
§5). On NewTOS the table is published with the EmuTOS layout, but a call
through one of its pointers from EL0 fails loud rather than silently.
**Requirement to revisit** when a ported program (e.g. a terminal program)
calls through it.

**`_MCH` is `0xFFFFFFFF` ("no Atari hardware")** — ADR-012 amendment
2026-10-28. The value is EmuTOS' `MCH_NOHARD`
(`references/emutos/include/cookie.h:50`, assigned at
`references/emutos/bios/machine.c:401` when `CONF_ATARI_HARDWARE` is 0); the
pinned make-512 oracle never reports it, so every `_MCH` cookie case is an
**ACCEPTED deviation** per profile (measured values: "Cookie jar vs. the
oracle" below, TASK/TOOL18.md).

**Cookie jar vs. the oracle** — measured 2026-10-29 on the pinned ROM,
table `tosdump/cases/jar-r1.tsv`, references `EMUTOS-1a7484ab-BIOS-jar` (tt),
`-BIOS-jar-falcontc`, `-BIOS-jar-disk`
(`measurements/measurement-2026-10-29-tool18/RUNINFO.TXT`). NewTOS seeds five
cookies (`tos-api/src/cookie.rs:223@seed_defaults`); the oracle's jar has twelve.
falcontc and disk are byte-identical, so their columns are one.

| Case | Cookie | Oracle tt | Oracle falcontc = disk | NewTOS | Status | Reason |
|---|---|---|---|---|---|---|
| 41 | `jar:*` | 12: `_CPU _VDO _FPU _MCH _SWI _SND _FRB _AKP _IDT _FDC XHDI SCSI` | same | 5: `NTOS _MCH _IDT _AKP EdDI` | ACCEPTED | follows from the per-cookie rows; re-measure when the seeded set changes |
| 44 | `_CPU` | `0x0000001E` | `0x0000001E` | absent | ACCEPTED | ADR-012 amendment 2026-10-28: no m68k CPU |
| 45 | `_VDO` | `0x00020000` | `0x00030000` | absent | ACCEPTED | user decision 2026-10-29 (TASK/ARCH06): no Shifter/VIDEL, `_VDO` stays absent; TeraDesk's ST-branch overscan crash is a port bug (TASK/SDK07.md) |
| 46 | `_FPU` | `0x00060000` | `0x00000000` | absent | ACCEPTED | ADR-012: no m68k FPU |
| 47 | `_MCH` | `0x00020000` | `0x00030000` | `0xFFFFFFFF` | ACCEPTED | ADR-012 amendment 2026-10-28: `MCH_NOHARD` |
| 48 | `_SWI` | `0x000000BF` | `0x000000BF` | absent | ACCEPTED | user decision 2026-10-29: no DIP switches (missing hardware; EmuTOS adds it only with them, `bios/machine.c:702-706`) |
| 49 | `_SND` | `0x00000003` | `0x0000001F` | absent | ACCEPTED | ADR-012: no PSG / DMA sound |
| 50 | `_FRB` | `0x00013350` | `0x00013350` | absent | ACCEPTED | user decision 2026-10-29: no ACSI/SCSI DMA, so no bounce buffer (`bios/machine.c:478`) |
| 51 | `_FLK` | absent | absent | absent | EQUAL | ADR-012 amendment 2026-10-26: not set |
| 52 | `_AKP` | `0x00000100` | `0x00000100` | `0x00000000` | OPEN | oracle = `COUNTRY=de` ROM; NewTOS keeps US (keyboard-layout entry above, difference 2) |
| 53 | `_IDT` | `0x0000112E` | `0x0000112E` | `0x0000112F` | OPEN | oracle = `COUNTRY=de` (`.`); NewTOS `/`, bring-up policy |
| 54 | `_FDC` | `0x01415443` | `0x01415443` | absent | ACCEPTED | ADR-012: no floppy controller |
| 55 | `__NF` | absent | absent | absent | EQUAL | — |
| 56 | `XHDI` | `0x00E0F702` | `0x00E0F702` | absent | ACCEPTED | ADR-032: no XHDI, no SCSI driver API |
| 57 | `SCSI` | `0x000050B6` | `0x000050B6` | absent | ACCEPTED | ADR-032 |
| 58, 61-64 | `_5MS _CF_ _MCF _NET NVDI` | absent | absent | absent | EQUAL | — |
| 67 | `NTOS` | absent | absent | `0x00000100` | ACCEPTED | ADR-012 (+ amendment 2026-10-28): NewTOS identity |
| 68 | `EdDI` | absent | absent | `0x00000001` | ACCEPTED | "The VDI speaks EdDI" (this file): marker, spec E2 |

**No Line-A: no opcode trap, no LINEA variable block** — user decision
2026-10-26, ADR-030 (`docs/MEMORY.md`). On TOS the m68k opcodes
`$A000`–`$A00F` trap through vector `$28` into 16 graphics routines, and op 0
hands out the LINEA variable block (`references/emutos/bios/bios.c:176`,
`references/emutos/bios/linea.S:156-163`, `:230-249`; spec
`docs/api/_line-a.md`). NewTOS has neither:

* **No opcode trap.** Native AArch64 programs have no `$Axxx` instruction,
  and NewTOS adds no substitute; `Setexc(10, …)` only stores into the vector
  shadow, which nothing dispatches (`tos-api/src/bios/vectors.rs:98-109`).
  m68k binaries, and with them the Line-A trap, belong to the later m68k
  emulator (FR-4).
* **No LINEA variable block.** Screen geometry, mouse position and buttons
  come from the documented calls: `vq_extnd`/`v_opnvwk` `work_out`,
  `vq_mouse`, `v_show_c`/`v_hide_c`, `vq_chcells`, `graf_mkstate`,
  `graf_handle`.
* **`__aline`, `__fonts`, `__funcs` are null by design**
  (`sdk/libc/mintlib/mintlib/alglobal.c:7`, `:11`, `:16`), so a reader fails
  loud: the named variables of `<mint/linea.h>` (`V_REZ_HZ`, `MOUSE_BT`,
  `V_CEL_MX`, …) lie at negative offsets from `__aline`
  (`sdk/libc/mintlib/include/mint/linea.h:154-315`) and end the program with
  an EL0 data abort (`kernel/src/arch/aarch64/exceptions.rs:303-309`). Not
  so the `__LINEA` struct fields (`__aline->_VPLANES`, the MWC macros,
  `sdk/libc/mintlib/include/mint/linea.h:485`ff.): null plus their positive
  offset lies in page 0, which is EL0 **read-only** outside the super state
  (TASK/MEM12, `tos-api/src/bdos/process/mmu.rs:789@UserRoNx`): such a read
  does not fault and returns system-variable bytes, a write ends the program
  with a bus error — unless it runs in super state, inside `Super()`/`Supexec`.
* **mintlib's `linea.c` is excluded from the SDK build by ADR-030**; there is
  no `linea0()` … `lineaf()`, and a port that calls them does not link.

**Observable difference:** a Line-A opcode, a Line-A binding or a LINEA
variable read has no NewTOS counterpart; a port uses the VDI/AES calls above
instead. No ported program is affected
(`docs/analysis/line-a-readers-2026-10-26.md`). EmuDESK's two readers are
rebuilt in NEWDESK through VDI values ([DESK69](../TASK/DESK69.md)).

## EmuTOS is the reference, not the template

CLAUDE.md Rule 1 says EmuTOS source *is* the spec. Through this charter that
means: EmuTOS defines **what a call must do** (the behaviour to match), not
**how NewTOS must build it**. We extract the *contract* from EmuTOS into
`docs/api/`; we then implement it cleanly. We do **not** port EmuTOS C, copy
its internal structures, or mirror its control flow where the result is
unobservable. ("Port `desk/`" wording in older plans means "reimplement the
`desk/` *behaviour*", not "translate the C".)

## System variables — compatible how

TOS exposes global variables at fixed low-memory addresses (`$420`, `$44E`,
`$4BA`, the cookie-jar pointer at `$5A0`, …). NewTOS keeps them
**compatible in value and semantics**; how they are *reached* depends on the
kind of caller:

- **Freshly-written native AArch64 source** reads them through the header
  accessors / BIOS calls (e.g. `Getbpb`, `Drvmap`, cookie-jar lookup,
  `Ssystem`). Same names, same values, same semantics.
- **1:1-compiled TOS ports** (ADR-019) contain the *classic* raw idiom
  (`*(LONG*)0x4F2`, the `$5A0` cookie-scan, `Setexc($5A0/4,-1)`). **B1
  (2026-07-24, commit `3d1c1fe`) populates the classic low-memory sysvars +
  OSHEADER + `$5A0` per-process at their real addresses in each app's EL0 VA**,
  so these raw accesses work — see `docs/analysis/bios-audit/b1-lowmem-feasibility.md`.
  Since MEM12 (2026-10-27) a raw **write** works as on TOS only inside
  `Super()`/`Supexec` (the super state, "Page 0 is read-only outside the
  super state" above); reads work always.
- **m68k binaries** (Phase 10 emulator) get the variables at their real
  addresses too: the emulator maps the low-memory region, so direct pokes work.

**Corrected 2026-07-24 (B1):** an earlier version of this section claimed
native code never uses raw low-memory addresses ("not via a raw `*(LONG*)0x4BA`")
and framed a strict two-world split (native → API only, m68k → low memory).
ADR-019 (1:1-compiled ports) + the TeraDesk `$4F2` boot crash refuted that:
native EL0 ports *do* hit the classic addresses, so NewTOS now provides them
per-process (B1). The kernel-struct-behind-BIOS-calls path (Phase-03 R3) remains
the source of truth and the idiom for freshly-written source.

## LONG results in `x0`: sign-extended

A TOS LONG result arrives in `x0` as a **sign-extended `i64`**
(`tos-api/src/trap.rs:6@TrapArgs`; the one normal write of `x0` is
`kernel/src/arch/aarch64/exceptions.rs:758@result`); pointers are full 64
bit and lie below 2 GiB. This is the m68k `D0` read as a signed LONG. Known
deviations, each with its fix ticket (user 2026-10-28;
`docs/analysis/abi-assumptions-check-2026-10-28.md` §1): `Rsconf`
zero-extends (TASK/XBIOS06.md), `Gettime` zero-extends (TASK/XBIOS09.md),
`Setexc(0x100…0x102, -1)` returns 64-bit placeholders instead of handler
addresses (TASK/BIOS07.md), the pointer form of `Getcookie()` zero-extends
in the SDK (TASK/SDK06.md). The `Pexec` child exit code is zero-extended on
purpose, as in EmuTOS (`references/emutos/bdos/proc.c:601`, `:616`).

## Native vs m68k struct variants

A struct's *fields and semantics* are contract; its *pointer width* follows
the ABI of the caller. NewTOS structs therefore exist in two faithful
layouts:

- **Native AArch64**: 64-bit pointers (e.g. `OBJECT` is 32 bytes with a
  64-bit `ob_spec`; `TEDINFO`/`ICONBLK`/`BITBLK` use 64-bit pointers). C
  source recompiled for NewTOS sees these via the NewTOS headers.
- **m68k**: the original 24/32-bit layout, seen by emulated binaries; the
  marshaling shim (Phase 10) translates between the two.

Both expose the **same fields with the same meaning**. The width difference
is an ABI detail the header set hides from source.

**The named exception: `AESGLOBAL` stays m68k-narrow on every ABI** — decision
of 2026-09-13 (`d8471c90`), recorded here 2026-10-26, moved from the STATUS log.
It keeps the layout of `references/emutos/aes/gemrslib.h:15-27`: 30 bytes /
15 WORDs, every `LONG` and pointer field **4 bytes**, also for an LP64 caller.
The reason is that this one structure is addressed two ways at once — by byte
offset *and* by WORD index (`AP_VERSION`..`AP_4RESV`,
`include/aesdefs.h:147-157`; every GEM program declares `short aes_global[]`).
Eight-byte pointers would move `ap_rsclen` off `global[9]` and `ap_planes` off
`global[10]`, keeping one half of the contract by breaking the other. `appl_init`
therefore writes `ap_private` as 4 bytes at byte 6 and leaves `ap_ptree`
(bytes 10..13) alone (`tos-api/src/aes/appl.rs`, module note "ABI note" and
`write`), and the SDK's gemlib overlay reads `_AESappglobal`/`_AESrscfile`/
`_AESrscmem` as 4-byte fields at offsets 6/10/14, with `_AESrscfile_set`/
`_AESrscmem_set` instead of assignable lvalues (`sdk/gemlib/overlay/gem.h`).
A 4-byte pointer suffices because every user address is below 2 GiB (VA = PA,
`docs/MEMORY.md` ADR-028). `OBJECT` has no index contract and keeps following
the caller's width. Test `tos-api/tests/aes_global_layout.rs` checks every byte
after `appl_init`; run `measurements/measurement-2026-09-13-aesglobal/`.

## The audit rubric — how to cross-check a spec or plan

For every `docs/api/*.md` entry, phase plan, or piece of code, check:

1. **Contract captured?** Are the fixed items (IDs, ABI, codes, struct
   layout, observable behaviour, variable values) stated correctly and
   completely?
2. **No internals smuggled in as contract?** Does it present an EmuTOS
   *implementation detail* (internal struct, algorithm, buffer trick) as if
   a program depended on it? → mark it **free**, don't bind ourselves to it.
3. **Observable vs internal separated?** Is it clear which parts must match
   and which are NewTOS's choice?
4. **Port-drift?** Does the wording say "port/translate EmuTOS" where it
   should say "reimplement the behaviour / contract"?
5. **Both worlds?** Where relevant, does it hold for native *and* m68k
   callers without the API branching on caller type (ARCHITECTURE ABI rule)?

Findings feed back into the spec (`docs/api/`) and the phase plans; ADRs go
in [`MEMORY.md`](MEMORY.md).

## Sanctioned FreeMiNT/TOS4 extensions

NewTOS is a **superset** of strict TOS 1-3: where a later reference (TOS 4 /
FreeMiNT) lifted a restriction and real apps depend on it, NewTOS adopts the
superset behaviour — additively, never breaking a conforming TOS 1-3 program.
Each such divergence is recorded here with its reason and reference.

- **`Fattrib` (GEMDOS 0x43) on directories.** GET (`wrt == 0`) on a folder
  returns the folder's attribute byte with `FA_SUBDIR` (0x10) set, instead of
  TOS 1-3's `EFILNF`. Reason: cflib `path_exists` (used by QED's clipboard init
  and every FreeMiNT/cflib app) probes a directory with
  `Fattrib(path, 0, 0) >= 0 && (attr & FA_DIR)`. Matches FreeMiNT `sys_f_attrib`
  (`references/freemint/sys/dosdir.c:998-1002`). SET on a folder stays rejected
  (`EACCDN`) — stricter than FreeMiNT — so a directory entry is never mutated
  (the #38 corruption guard). Spec: `docs/api/gemdos/file/43-fattrib.md`.

  **Both spellings, with and without a trailing separator** (2026-09-12, Q07).
  cflib's `make_normalpath` appends a separator to every path before it probes
  (`sdk/cflib/cflib-21/fsnorm.c:85-92`), so the caller this extension exists for
  always asks `Fattrib("C:\CLIPBRD\", 0, 0)`. Until 2026-09-12 that spelling
  still answered `EFILNF` and the stated purpose of the deviation was not met.
  FreeMiNT answers it with the folder's attributes (`relpath2cookie` peels the
  empty component, `sys/filesys.c:986-1010`; `fatfs_lookup` resolves the empty
  name as `"."`, `sys/fatfs.c:5753-5764`). The lift is confined to `Fattrib`
  GET and to paths that name a **directory**: a trailing separator on a file
  keeps `EPTHNF` (FreeMiNT's ENOTDIR is the same `-34`), on a missing name keeps
  `EPTHNF`, and the bare drive root `"C:\"` keeps `EFILNF` — FreeMiNT would
  synthesize `FA_DIR` there, NewTOS deliberately does not, and that remains an
  open question rather than a decided one. `Fsfirst`, `Fopen` and `Dcreate` on a
  path ending in a separator are **unchanged** and stay EmuTOS-faithful
  (measured identical on EmuTOS 1.4 and Atari TOS 3.06).

## Deferred optimisations (Phase 2)

Optimisations and modern extensions are **not built until the contract is
clean and verified**, and are kept here so they never blur the contract.
This section is a holding area for ideas — nothing in it changes observable
behaviour for a conforming TOS program.

- _(to be filled in — the user's optimisation ideas go here, each tagged
  with which free-implementation area it touches and confirmed not to alter
  the contract.)_

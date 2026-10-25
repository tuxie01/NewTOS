# AES Resource Manager (`rsrc_*` 110–114) + File Selector (`fsel_*` 90–91)

> **PHASE-00b D8, Batch 6 (full-contract refinement).**
> Source: EmuTOS @ `1a7484ab`.
> Primary: `aes/gemrslib.c` (`rs_load`/`rs_free`/`rs_gaddr`/`rs_saddr`/
> `rs_obfix`, the `.RSC` load + relocation `fix_trindex`/`fix_objects`/
> `fix_tedinfo_std`/`fix_nptrs`/`fix_long`, the coordinate fix
> `rs_obfix`/`fix_chpos`), `aes/gemfslib.c` (`fs_input` = both `fsel_input`
> and `fsel_exinput`, the FSELECTR dialog). Dispatch arms
> `aes/gemsuper.c:308-326,362-377`. RSC header `include/rsdefs.h:16-36`;
> `R_*` type codes + slot macros `include/aesdefs.h:456-465,568-594`;
> `AESGLOBAL` resource fields `aes/gemrslib.h:15-27`.
>
> Call convention + AESPB layout: [`_dispatcher.md`](./_dispatcher.md).
> Shared AES globals (the loaded-resource `ap_rscmem`/`ap_ptree`, the shell
> path used by `rs_load`→`sh_find`): [`_state.md`](./_state.md) §6c.
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** the opcode numbers (110–114, 90–91); each call's
> `int_in`/`addr_in`/`int_out`/`addr_out` slot layout (note `rsrc_gaddr` is the
> **only** AES call whose `addr_out[0]` the dispatcher copies back, and
> `fsel_*`'s button return is `int_out[1]`, **not** an `addr_out`); the
> **`.RSC` on-disk file format** (the `RSHDR` byte layout, the offset-table +
> count fields, the `NEW_FORMAT_RSC` bit); the offset→pointer **relocation
> rule** (every internal reference is a byte offset from the file base fixed to
> an absolute address, `-1L` left unrelocated); the **`rs_obfix` char→pixel
> coordinate rule** (hi byte = char count × cell size, lo byte = signed pixel
> offset, `w == 80` → full screen width); the **`R_*` type→array map** used by
> `rsrc_gaddr`/`rsrc_saddr`; `rsrc_gaddr`'s `1`/`0` found/not-found return; the
> `fsel_*` in-out path/name buffers and the `path`/`mask` split rule; the
> `fsel_*` button return (1 = OK, 0 = Cancel). **Reference (NewTOS reimplements
> the *how* freely):** the `rs_hdr`/`rs_global` file-local statics, the CICON
> fix-up/expansion (`fix_cicons`/`transform_all_cicons`), the FSELECTR resource
> tree drawing, `fs_active`'s shell-sort + `ad_fsnames`/`g_fslist` buffers,
> `fs_nscroll`'s `bb_screen` blit-scroll, and the `dos_alloc_anyram` work-area
> layout — these are the EmuTOS internals NewTOS rebuilds on its own file I/O,
> object manager, and rasteriser.
>
> **NewTOS deviation (ADR-013, D-100).** Everything in §0 below is EmuTOS's
> **classic Atari on-disk `.RSC` format** — accurately extracted, and it is
> what a byte-compatible loader (a future m68k-emulator compat phase) would
> need. **NewTOS's actual `rsrc_load` does not read this format at all.**
> Per ADR-013 (`docs/MEMORY.md`), NewTOS defines its own **native** `.RSC`
> (`NrscHeader`, magic `"NRSC"`, little-endian, 64-bit `ob_spec`, offsets
> from blob base — `tos-api/src/aes/resource.rs`); `rsrc_load` resolves its
> filename through `sh_find` exactly as EmuTOS does (see the `rsrc_load (110)`
> callout), and — from NRSC v9/v10 on (A40, 2026-09-19) — the OBJECT
> coordinates are the classic file's **packed char-grid words**, converted at
> **load** by the `rs_obfix` rule with the live `gl_wchar`/`gl_hchar`/
> `gl_width`, exactly as `rs_fixit` → `fix_objects` does; `rsrc_obfix` (114)
> is that same `rs_obfix`. The `rsrc_gaddr`/`rsrc_saddr` ABI slots, the `R_*`
> type semantics, the 1/0 return convention **and the char→pixel rule** are
> contract; the on-disk byte layout and `sh_find` are EmuTOS-reference only
> for NewTOS's current native-format loader. Full accounting: "NewTOS
> implementation notes" below and `docs/api/_discrepancies.md` D-100.

---

## 0. The `.RSC` file format (`RSHDR`, `rsdefs.h:16-36`)

A resource file is a self-contained blob: a fixed 36-byte header of **byte
offsets + counts** followed by the object arrays, TEDINFOs, ICONBLKs, BITBLKs,
the free-string / free-image pointer tables, the string/image data, and the
tree index. Every internal reference stored on disk is a **byte offset from the
file base**; on load these are converted to absolute pointers. m68k = big-endian
on disk (the header WORDs are big-endian).

| Off | Field | Type | Meaning |
|----:|-------|------|---------|
| 0x00 | `rsh_vrsn`    | UWORD | format version; **bit 2 (`NEW_FORMAT_RSC = 0x0004`)** set → new-format file with CICONBLKs + a trailing extension array |
| 0x02 | `rsh_object`  | UWORD | offset: OBJECT array |
| 0x04 | `rsh_tedinfo` | UWORD | offset: TEDINFO array |
| 0x06 | `rsh_iconblk` | UWORD | offset: ICONBLK array |
| 0x08 | `rsh_bitblk`  | UWORD | offset: BITBLK array |
| 0x0A | `rsh_frstr`   | UWORD | offset: **free-string** pointer array (LONG offsets) |
| 0x0C | `rsh_string`  | UWORD | offset: string data |
| 0x0E | `rsh_imdata`  | UWORD | offset: image data |
| 0x10 | `rsh_frimg`   | UWORD | offset: **free-image** pointer array (LONG offsets) |
| 0x12 | `rsh_trindex` | UWORD | offset: tree-index (one LONG offset per tree) |
| 0x14 | `rsh_nobs`    | WORD  | count: objects |
| 0x16 | `rsh_ntree`   | WORD  | count: trees |
| 0x18 | `rsh_nted`    | WORD  | count: TEDINFOs |
| 0x1A | `rsh_nib`     | WORD  | count: ICONBLKs |
| 0x1C | `rsh_nbb`     | WORD  | count: BITBLKs |
| 0x1E | `rsh_nstring` | WORD  | count: strings (free strings) |
| 0x20 | `rsh_nimages` | WORD  | count: images (free images) |
| 0x22 | `rsh_rssize`  | UWORD | total classic-format file size in bytes |

> **`NEW_FORMAT_RSC` bit.** `rsh_vrsn & 0x0004` marks a *new-format* file with
> colour icons (`rsdefs.h:39`). For such files `rsh_rssize` is only the classic
> size; the **true file length** is read as a LONG at file offset `rsh_rssize`
> (`gemrslib.c:766-772`), and a trailing extension array (true length, CICON
> table offset, …) follows. The previous spec called `rsh_vrsn == 0` "classic";
> it is a **bit test**, not an equality.

### The relocation (what `rs_load` fixes up)

`rs_readit`/`rs_fixit` (`gemrslib.c:751-820`) turn the loaded blob into a live
tree by walking each count and converting offsets to absolute pointers via
`fix_long` (`add rs_hdr base`, leaving `-1L` untouched, `gemrslib.c:576-588`):

1. **`fix_trindex`** (`:591-603`) — point `rs_global->ap_ptree` at the tree-index
   array and relocate each of `rsh_ntree` tree offsets. This is what builds the
   `ap_ptree[]` table `rsrc_gaddr(R_TREE)` and the rest of the AES read.
2. **CICONs** (`fix_cicons`, `#if CONF_WITH_COLOUR_ICONS`) — fix up the CICONBLK
   pointer table and expand/transform each colour icon to the device format
   (reference; see §CICON note).
3. **`fix_tedinfo_std`** (`:655-669`) — for each of `rsh_nted` TEDINFOs relocate
   `te_ptext`/`te_ptmplt`/`te_pvalid` and recompute `te_txtlen`/`te_tmplen`
   from the relocated strings.
4. **`fix_nptrs`** for ICONBLK `ib_pmask`/`ib_pdata`/`ib_ptext` (`rsh_nib` each),
   BITBLK `bi_pdata` (`rsh_nbb`), free strings (`rsh_nstring`) and free images
   (`rsh_nimages`) (`:799-805`).
5. **`fix_objects`** (`rs_fixit`, `:606-637`) — for each of `rsh_nobs` objects:
   apply `rs_obfix` (coordinate fix, below) and relocate `ob_spec` **unless the
   type is `G_BOX`/`G_IBOX`/`G_BOXCHAR`** (whose `ob_spec` is a colour word, not
   a pointer); `G_CICON` `ob_spec` is turned into a CICONBLK-table index→pointer.

> **Object fix-up is deferred past the header read.** `rs_readit` reads and
> relocates everything *except* object coordinates/`ob_spec`; `rs_fixit` does
> those afterwards. This split exists because the char→pixel conversion needs an
> open VDI workstation (`gl_wchar`/`gl_hchar`), which is not up when the GEM
> resource itself is loaded at boot (`gemrslib.c:745-750,811-820`). For an
> app-level `rsrc_load` both run back-to-back (`rs_load`, `:826-850`).

### `R_*` resource type codes (`aesdefs.h:578-594`)

`rsrc_gaddr`/`rsrc_saddr` select **which array and indexing scheme** via the
type code. `get_addr` (`gemrslib.c:112-176`) resolves each:

| Code | Name | `rsrc_gaddr` returns |
|-----:|------|----------------------|
| 0 | `R_TREE`      | `ap_ptree[index]` — the index-th tree's OBJECT base |
| 1 | `R_OBJECT`    | `&object[index]` (base `rsh_object`) |
| 2 | `R_TEDINFO`   | `&tedinfo[index]` (base `rsh_tedinfo`) |
| 3 | `R_ICONBLK`   | `&iconblk[index]` (base `rsh_iconblk`) |
| 4 | `R_BITBLK`    | `&bitblk[index]` (base `rsh_bitblk`) |
| 5 | `R_STRING`    | the free-string **data** pointer `frstr[index]` (dereferenced) |
| 6 | `R_IMAGEDATA` | the free-image **data** pointer `frimg[index]` (dereferenced) |
| 7 | `R_OBSPEC`    | `&object[index].ob_spec` |
| 8 | `R_TEPTEXT`   | `&tedinfo[index]` (== `R_TEDINFO`; `te_ptext` is field 0) |
| 9 | `R_TEPTMPLT`  | `&tedinfo[index].te_ptmplt` |
| 10 | `R_TEPVALID` | `&tedinfo[index].te_pvalid` |
| 11 | `R_IBPMASK`  | `&iconblk[index]` (== `R_ICONBLK`; `ib_pmask` is field 0) |
| 12 | `R_IBPDATA`  | `&iconblk[index].ib_pdata` |
| 13 | `R_IBPTEXT`  | `&iconblk[index].ib_ptext` |
| 14 | `R_BIPDATA`  | `&bitblk[index]` (== `R_BITBLK`; `bi_pdata` is field 0) |
| 15 | `R_FRSTR`    | `&frstr[index]` — address **of** the free-string pointer slot |
| 16 | `R_FRIMG`    | `&frimg[index]` — address **of** the free-image pointer slot |
| other | — | `(void *)-1L` (bad type; `rsrc_gaddr` then returns 0) |

> **Correction vs. the previous spec.** (1) The type table runs **0–16**, not
> 0–15; `R_FRIMG = 16` (free images) exists (`aesdefs.h:594`). (2) `R_STRING`
> (5)/`R_IMAGEDATA` (6) return the **dereferenced data** pointer, whereas
> `R_FRSTR` (15)/`R_FRIMG` (16) return the **address of the table slot** — the
> pair `get`/`set` a free string. (3) Codes 8/11/14 alias 2/3/4 because the
> sub-pointer is the struct's first field (`gemrslib.c:129-142`). See D-054.

---

## 1. `.RSC` load / free

### `rsrc_load` (110) — `rs_load`

**Source:** `gemsuper.c:363-365` → `aes/gemrslib.c:826-850` (`rs_load`).
Helpers: `sh_find` (locate the file via the shell search path, §Dependencies),
`dos_open`/`dos_read`/`dos_lseek`/`dos_close` (GEMDOS file I/O),
`dos_alloc_anyram` (allocate the in-memory blob), `rs_readit`+`rs_fixit`
(read + relocate, §0). **Globals written:** `ap_rscmem` (the loaded blob base),
`ap_rsclen` (its length), `ap_ptree` (the relocated tree table) — all in the
caller's `AESGLOBAL` (`gemrslib.c:786-788,597`).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`RS_PFNAME`) | pointer to the resource filename (NUL-terminated) |
| `int_out[0]` | `1` if loaded + relocated, `0` on any failure |

**Behaviour.** Copy the filename to a scratch buffer and **run `sh_find()`** on
it (`gemrslib.c:835-837`) so the resource is located via the same directory
search as an executable (app dir → current dir → root → AES `PATH=`, see
`shel_find` in [`scrap-shell.md`](./scrap-shell.md)); return `0` if not found.
Open read-only, read the 36-byte header, allocate `rsh_rssize` (or the true
length for a new-format file) bytes of any RAM, read the whole file in, store
`ap_rscmem`/`ap_rsclen`, then relocate every offset to a pointer and fix the
coordinates (§0). Returns `0` (and frees nothing) on a short read or alloc
failure; `1` on success. One `AESGLOBAL` **names** one resource at a time — a
second `rsrc_load` overwrites `ap_rscmem`/`ap_rsclen`/`ap_ptree` without
freeing the first (`rs_readit`, `:786-788`; nothing in `rs_load` frees). The
first blob stays the application's GEMDOS memory: it can be reached again by
restoring the three words (TeraDesk's `load_icons` does so, see the A39 callout
below), it is freed by `rsrc_free` while named, and otherwise dies with the
process (`bdos/proc.c:136`, `free_all_owned`).

> **NewTOS status (deviation, ADR-013/D-100).** The **lookup** matches EmuTOS
> since 2026-09-11: `rsrc_load` (`tos-api/src/aes/resource.rs::rsrc_load`)
> copies `addr_in[0]` into a `MAXPATHLEN` scratch and runs
> `aes::shell::sh_find` on it — `findfile`'s stages (1) application directory
> + name portion, (2) the spec **unchanged**, (3) `\<name>` on the **current**
> drive; stage (4), the AES `PATH=`, is not implemented — the AES
> environment it would read exists since TASK/PPARENT (EmuTOS `default_env`,
> [`../gemdos/process/_parent-and-environment.md`](../gemdos/process/_parent-and-environment.md)),
> the stage itself is open (see `sh_find` in
> [`scrap-shell.md`](./scrap-shell.md)). The previous fixed
> `C:\GEMSYS\RSC\<name>` / `C:\<name>` search — which could not resolve a
> qualified path at all — is gone; `C:\GEMSYS\RSC\` had no EmuTOS counterpart
> and no caller.
>
> What still deviates is the **byte level**: the file is read into a static
> 64 KiB buffer (`crate::bdos::fs::read_file`) and — if the header magic is
> `"NRSC"`, at the front (NRSC v1–v12) or at byte 40 behind the native
> `RSHDR` preamble (v13+, below) — the **native** offset table is relocated
> in place (`relocate_and_register`). There is no big-endian header and no
> on-disk `NEW_FORMAT_RSC`/CICON extension array (the colour icons travel as
> native records, §5.9). The `int_out[0] = 1`/`0` success contract is
> preserved.
>
> **The loaded image begins with a native `RSHDR` since 2026-09-19 (A41
> part 2, NRSC v13 LP64 / v14 ILP32).** EmuTOS publishes the loaded *file*
> as `ap_rscmem` (`rs_readit`, `gemrslib.c:787`, `rs_global->ap_rscmem =
> rs_hdr`), classic `RSHDR` at its head, and applications parse that header
> themselves — TeraDesk's `xrsrc_gaddr` (`sdk/apps/teradesk/rsc_load.c:
> 496-541`) reads `aes_global[7]`, tests `IS_XRSC_HEADER` (`rsh_vrsn & 3 ==
> 3` → the `RSXHDR` with `unsigned long` fields, else the 18 × `u16`
> `RSHDR`) and takes `rsh_trindex`/`rsh_object`/`rsh_tedinfo`/`rsh_iconblk`/
> `rsh_bitblk`/`rsh_frstr`/`rsh_frimg` as byte offsets from `ap_rscmem`,
> bounded by the seven counts (§5.11 item 5). That is a contract over the
> **memory image behind `ap_rscmem`**, beyond ADR-013's freedom over the
> on-disk bytes, and NewTOS honours it the way EmuTOS does — the image *is*
> the file: `tools/mkrsc.py convert` emits the 36-byte native `RSHDR` (the
> 18 LE `u16` of §0's table in §0's order, every offset counted from the
> file's first byte; `rsh_vrsn` the classic file's — 0, or 4 =
> `NEW_FORMAT_RSC` for one with colour icons, as EmuTOS leaves it in memory;
> `rsh_rssize` the whole image; `rsh_string`/`rsh_imdata` the shared
> string+raster section), padded to 40 bytes, **in front of** the NRSC
> header, whose offsets count from the file's first byte too (one reference
> point, `resource.rs` `RSHDR_PAD`, `NRSC_VER_RSHDR`). The loader finds the
> magic at 40, refuses a v13+ header at the front or an older one behind a
> preamble, and checks the preamble against the NRSC header before it
> relocates anything (`rshdr_is_consistent`: the seven offsets, the seven
> counts, `rsh_rssize = total_size`, `rsh_vrsn & 3 != 3`) — `rs_readit`'s
> FALSE on any disagreement. `ap_rscmem` = the image's first byte = the
> `RSHDR`, `ap_rsclen` = `rsh_rssize`, `ap_ptree` = `ap_rscmem +
> rsh_trindex`, and slot *i* there is `rsrc_gaddr(R_TREE, i)`. Every `u16`
> field bounds the image below 64 KiB, so the converter refuses an image
> over `RSC_CAP − 40` bytes. A v1–v12 blob keeps loading, but publishes an
> `ap_rscmem` an application must not parse as an `RSHDR` — hence every
> shipped resource (`desktop.rsc`, `cicons.rsc`, `qed.rsc`, the embedded
> `gem_fsel.nrsc`) is reconverted. Tests: `resource.rs` `rshdr_tests` (the
> `xrsrc_gaddr` reading, every branch, against `rsrc_gaddr`; the refusals),
> `tools/xtask/src/rsc_cicon_tests.rs` (`check_rshdr`).
>
> **The coordinate fix runs at load, with the live metrics, since 2026-09-19
> (A40).** From NRSC v9 (LP64) / v10 (ILP32) on the blob carries every
> object's `ob_x`/`ob_y`/`ob_width`/`ob_height` as the classic file's packed
> char-grid word, and the loader applies `rs_obfix` to each of its
> `n_objects` objects (`fix_objects_coords`) — step 5 of §0, `rs_fixit` →
> `fix_objects` → `rs_obfix(obj, 0)` (`gemrslib.c:816-820`, `:606-637`) —
> with `gl_wchar`/`gl_hchar` from the VDI's default face and `gl_width` from
> the raster (`aes::graf::gl_wchar/gl_hchar/gl_width`, `gemgraf.c:393-405`).
> So an 80-character root is `gl_width` wide on the screen the file is
> loaded on — 640 on a 640×480 workstation, 1920 on the Pi 4's 1920×1080 —
> and every other word is `chars × cell + offset` at the live cell. The
> same pass runs for the embedded file-selector resource
> (`relocate_blob`), EmuTOS's `gem_rsc_fixit` (`aes/gem_rsc.c:636-641`,
> `geminit.c:711`). A v1–v8 blob holds **pixels** — its converter did this
> arithmetic itself, with `8 × 16 / 640` baked in (audit B10) — and is
> loaded without conversion, so nothing is converted twice. At 8×16/640 the
> loaded state of a v9 blob is byte-identical to the v7 blob of the same
> source (measured on `desktop.rsc`, `qed.rsc`, `gem_fsel.nrsc`: 0 differing
> bytes). Test: `tos-api/tests/aes_rsrc_obfix_at_load.rs`; tool side
> `tools/xtask/src/rsc_obfix_tests.rs`.
>
> **The relocation walk matches §0 steps 3-5 since 2026-09-13 (A36).** The
> TEDINFO / ICONBLK / BITBLK pointers are relocated **array-wise**, over the
> `n_ted`/`n_ib`/`n_bb` counts of the native header — the counterpart of
> `rsh_nted`/`rsh_nib`/`rsh_nbb` — so a record no OBJECT references (a free
> image, a spare TEDINFO) is a pointer afterwards too, and `te_txtlen`/
> `te_tmplen` are recomputed for it as well. The earlier walk followed
> `ob_spec` and reached only the referenced records. A native header older than
> v5 declares no such arrays; such a file loads only if it holds no
> text/icon/image object, and is refused otherwise (`rs_readit`'s FALSE), since
> its records could not be relocated by count.
>
> **Reported back through the caller's `AESGLOBAL` since 2026-09-14 (A35).**
> `rsrc_load` now writes `ap_rscmem` (the blob base), `ap_rsclen` (its declared
> size) and `ap_ptree` (the relocated tree index), the way `rs_readit`
> (`gemrslib.c:787-788`) and `fix_trindex` (`:595-597`) do. Three things about
> that are NewTOS's, and are contract:
>
> 1. **The tree index is a relocated pointer table from NRSC v7/v8 on.** Until
>    then NewTOS's native format held root OBJECT *indices* in fixed 4-byte
>    slots, so `ap_ptree` had no array to name. v7 (LP64) / v8 (ILP32) keep the
>    68-byte header and make `off_trindex` a table of root-OBJECT **byte
>    offsets**, relocated in place like the free-string/free-image tables —
>    which is exactly EmuTOS's `rsh_trindex`. `ap_ptree[i]` and
>    `rsrc_gaddr(R_TREE, i)` are therefore the same address, EmuTOS's own
>    binding in `get_addr` (`gemrslib.c:124`). A v1–v6 blob still loads and
>    still answers `rsrc_gaddr(R_TREE, i)`; it simply publishes no `ap_ptree`.
> 2. **The slot width is the caller's, the field width is TOS's.** A slot of
>    that table is a native `OBJECT *` — 8 bytes for an LP64 caller, 4 for
>    ILP32 — because that is how `_AESrscfile[i]` is dereferenced
>    (`sdk/gemlib/overlay/gem.h`). The *field* `ap_ptree` (byte 10) and
>    `ap_rscmem` (byte 14) stay **4 bytes on every ABI**, because `AESGLOBAL`
>    is addressed by WORD index as well as by byte offset
>    ([`application.md`](./application.md) §`AESGLOBAL`). The loader relocates
>    the table at the **caller's** ABI width, not at the file's declared one,
>    exactly as it already does for frstr/frimg.
> 3. **An address those four bytes cannot carry is not carried at all.** The
>    fields are written only when the blob's base fits 32 bits. That holds by
>    construction for every real caller — an EL0 app's blob is a BDOS-pool
>    block, below 2 GiB (ARCH03 stage 1: VA = PA, plan invariant I2; before
>    that a per-process window below 32 MiB, where TeraDesk's 68 rsrc calls
>    were measured at `0x4000ac … 0x409381`). It does **not** hold for the
>    in-kernel path, which relocates in place in a kernel high-half static: the
>    low 32 bits of such a VA name a physical address that resolves to nothing
>    at EL1 (`TTBR0_EL1` maps nothing without a process). There the three stay
>    **0** — EmuTOS's "nothing loaded" reading (`rs_free`, `gemrslib.c:685`) —
>    rather than carry a truncated pointer. The load itself still succeeds and
>    `rsrc_gaddr` still answers at full width; NewTOS's own descriptor is a
>    kernel static, not the `AESGLOBAL`. The three land together for the same
>    reason a length beside a null base would state two contradictory things.
>
> **The resource state is per AES process and bound to the caller's
> `AESGLOBAL`, as `rs_sglobe` binds it (contract, decided 2026-09-19 — A39).**
> In EmuTOS the three words *are* the resource descriptor: `rs_sglobe`
> re-reads `ap_rscmem` out of the caller's `AESGLOBAL` on every `rs_free`,
> `rs_gaddr`, `rs_saddr` and `rs_fixit` (`gemrslib.c:675-679`; calls `:690`,
> `:717`, `:732`, `:818`), `get_addr(R_TREE)` reads `ap_ptree[i]` out of it
> (`:124`), and the `AESPD` holds no resource field at all
> (`aes/struct.h:119-155`). Two AES processes with separate `global[]` arrays
> therefore never interfere — EmuTOS has that case itself in its desk
> accessories, each an `AESPD` of its own with its own `global[]`
> (`aes/geminit.c:812-814`, `gempd.c:110-131`), and `crysbind` receives
> `pglobal` per call from the AESPB (`gemsuper.c:441`). The contract NewTOS
> implements:
>
> 1. **Keyed by the calling AES process** (its pid, as `appl_init` reports it —
>    `current_pid`, GEMDOS slot + 1). A child that `rsrc_load`s and
>    `rsrc_free`s touches only its own state; the parent's `rsrc_gaddr`
>    answers exactly as before (the H05 defect: `TASK/A39.md`).
> 2. **`rsrc_load` writes** `ap_ptree`/`ap_rscmem`/`ap_rsclen` (4-byte user
>    VAs, `gemrslib.c:786-788`, `:597`) and does **not** free an earlier load
>    of the same process (`rs_load` never does); several blobs may be loaded
>    per process at once, the `AESGLOBAL` names one of them.
> 3. **`rsrc_free`/`rsrc_gaddr`/`rsrc_saddr` act on the blob the caller's
>    `ap_rscmem` names** (`:677-678`, `:699`, `:718`, `:733`). An application
>    that rewrites `ap_rscmem` is followed — TeraDesk does exactly that:
>    `load_icons` saves the `desktop.rsc` words, may `rsrc_load("cicons.rsc")`
>    through the AES, loads `icons.rsc` with its own `xrsrc_load` (which
>    stores its own base into `aes_global[5]`/`[7]`, `rsc_load.c:414-431`),
>    and then **restores** the `desktop.rsc` words so later
>    `rsrc_gaddr(R_STRING, …)` resolve there again (`sdk/apps/teradesk/icon.c:
>    2707-2708, 2742-2743, 2762-2763`; its comment: *"alert_abort will call
>    rsrc_gaddr(), need to reset globals here to point to desktop.rsc"*). The
>    earlier claim that no known caller does this was wrong.
> 4. **The selector is only honoured for blobs the AES loaded for this
>    process.** NewTOS matches the 4-byte `ap_rscmem` against the bases of the
>    caller's own loaded blobs (all inside its process window below
>    `USER_TPA_MAX_VA`, so exact in 32 bits); a value naming none of them — 0,
>    another process's blob, a foreign or application-built address, a null
>    `global` in the AESPB — reads as **"nothing loaded"**: `rsrc_free` → 0
>    (`:702-704`), `rsrc_gaddr` → `addr_out[0] = 0`/`int_out[0] = 0` for
>    every defined type (an unknown type still answers `-1`/`0`, §2 — that
>    arm reads no header on either machine), `rsrc_saddr` → 0. EmuTOS would
>    parse whatever header stands at that
>    address (`:100-104`); XaAES follows the pointer too and then range-checks
>    the *result* against the length in that very header
>    (`xaaes/src.km/xa_rsrc.c:1701-1705`, `:1839-1855`). NewTOS moves that
>    check to the selector, because the kernel must not parse a header it did
>    not load. The residual difference — an application pointing the AES at a
>    blob it built itself — has no caller in the tree (TeraDesk queries its
>    private blob only through its own `xrsrc_gaddr`, `icon.c:2750`).
> 5. **`R_TREE`** is answered from the tree table of the selected blob, which
>    from NRSC v7/v8 on is literally the `ap_ptree[i]` array (§ item 1 above);
>    an application repointing `ap_ptree` alone to a private table is served
>    by EmuTOS (`:124`) and not by NewTOS — no known caller.
> 6. **`rsrc_free` frees only the selected blob** and leaves the caller's three
>    words as they are (`rs_free` writes none of them). **`appl_exit` does not
>    touch the resource** (`ap_exit`, `aes/gemaplib.c:251-261`). At **process
>    end** the blob dies with the process's memory (`Mxalloc` stamps the
>    owner, `bdos/iumem.c:134`; `xterm` → `ixterm` → `free_all_owned`,
>    `bdos/proc.c:601-619`, `:136`, `:92-100`) and its descriptor entries go
>    with it — the dying process's only; every other process keeps its state.
>    XaAES frees a client's whole resource list already at `appl_exit`
>    (`xa_appl.c:895` → `exit_client` `:745-750`) and rewrites the `global`
>    words on `rsrc_free` (`xa_rsrc.c:1347-1362`); neither is adopted (EmuTOS
>    is the target profile; both are observable only to a program that calls
>    `rsrc_gaddr` after `appl_exit`/`rsrc_free`).
> 7. `rsrc_load` needs no prior `appl_init` (`crysbind` checks nothing,
>    `gemsuper.c:363-365`).
> 8. **`Mfree(ap_rscmem)` ahead of `rsrc_free` is legal and NewTOS must let it
>    succeed (A76, 2026-09-20).** The blob `rsrc_load` returns is an ordinary
>    process-owned `Mxalloc` block like any other — EmuTOS's own loader gets
>    it through `dos_alloc_anyram` = `Mxalloc(_, MX_PREFTTRAM)`
>    (`util/gemdos.c:117-120`; `aes/gemrslib.c:775`) and its own `dos_free` is
>    a plain `Mfree` (`util/gemdos.c:130-133`); nothing marks the block as
>    AES-owned or otherwise protected in the pool. An application calling
>    `Mfree(ap_rscmem)` directly therefore gets `E_OK`, exactly as for any
>    other block it owns (`docs/api/gemdos/memory/49-mfree.md`). A *later*
>    `rsrc_free` on the same (now-freed) blob re-reads the stale `ap_rscmem`
>    (item 6 above: nothing clears it) and its own `dos_free` hits the
>    already-unlinked block → `EIMBA` → `rsrc_free` returns `0`
>    (`gemrslib.c:685-708`; full derivation:
>    `docs/api/gemdos/memory/49-mfree.md` § "AES resource block"). **NewTOS
>    target contract:** `Mfree` on the mapped `app_va` of a `Kind::Rsc` block
>    must return `E_OK` and release its pool block exactly as for a
>    `Malloc`ed block, not `EIMBA` — the current refusal (`usermem::free`
>    skips `Kind::Rsc`, `tos-api/src/bdos/process/usermem.rs:605-624`) is a
>    **defect** (`TASK/A76.md`, measured `Mfree(0x427000) -> -40` on
>    TeraDesk's `free_icons`), not a sanctioned deviation. Once fixed, the
>    resource entry for that blob must learn the block is gone (so a later
>    `rsrc_free` answers `0` without touching freed pool memory, and process
>    exit does not free the same pool key twice) — matching EmuTOS's
>    *observable* `rsrc_free → 0`, not its hazardous header re-walk over
>    freed memory (see the `Mfree` spec § "AES resource block"). The
>    colour-icon `col_data`/`sel_data` buffers are separate `Mxalloc` blocks
>    (`transform_all_cicons`, `gemrslib.c:428,438`) and stay allocated until
>    `rsrc_free` or process end either way — a bare `Mfree(ap_rscmem)` never
>    reaches them.
>
> Where XaAES is the model — the **per-client list** of loaded resources
> (`struct xa_client::resources`, `xa_types.h:2222`; `list_resource`,
> `xa_rsrc.c:895-900`) — it stands in for what EmuTOS gets for free from the
> application's own memory; the reference comparison is
> `docs/analysis/rsrc-per-app-state-2026-09-19.md`.
>
> **NewTOS status: built, both stages** (2026-09-19). Stage 1 — the
> per-process list, items 1, 2, 6, 7 — landed in `d7ab92d`; stage 2 — the
> selector read back out of the caller's `AESGLOBAL`, items 3–5 — after
> **A60** (`fc1bf02`) put TeraDesk's save/restore of the words at the TOS
> bytes on LP64 (before it, `load_icons` left `0x407000` at byte 14, a value
> naming no blob: `TASK/A60-STATUS-DONE.MD` § 1–3). `resource.rs`:
> `selected` reads the 4-byte `ap_rscmem` and matches it against the names
> of the caller's own entries; `report_load` and the entry's name come from
> one rule (`published_name`), so the word the caller received is the word
> that selects. One realisation detail is NewTOS's, for the path EmuTOS does
> not have: a load the four bytes cannot name (the in-place kernel path, a
> pre-v7 blob — A35 item 3 above, where nothing is written) carries the name
> 0 it left the caller with, and the zero selector reaches the process's
> latest such load and nothing else; a named blob is never reached by 0, an
> unnamed one never by a non-zero value. That keeps A35's "the load itself
> still succeeds and `rsrc_gaddr` still answers at full width" true without
> ever resolving a value the kernel did not publish; on a running system no
> load is unnamed. Tests: `resource.rs` `mod selector_tests` (restore →
> earlier blob; foreign/hybrid/zero selector → nothing, nothing freed;
> another pid's blob → nothing; `rsrc_free` frees the named blob only; null
> `global`; the unnamed case) and `tests/aes_rsrc_per_process.rs` (the
> per-process half).
>
> **There is no EL1 caller of `rsrc_load`** (measured 2026-09-13, re-checked
> 2026-09-19, unchanged).
> The only way in is crysbind opcode 110 (`tos-api/src/aes/mod.rs`) through
> `dispatch` → `thread::call` → the `svc #2` handler. The EL1 `svc #2` emitters
> are `kernel/src/{desk/mod.rs, selftest_objc.rs, selftest_aes.rs,
> selftest_gemtrap.rs}`; only `selftest_objc` issues rsrc opcodes at all, and
> of its three rsrc demos the two that issue opcode 110 (`run_rsrc_file`,
> `run_rsrc_gem`) have no caller, while `run_rsrc` — reachable only from the
> callerless `aes_demo::showcase` — registers a hand-built **v1** blob via
> `relocate_and_register` and never issues 110. That v1 blob is the live
> backward-compatibility case for the old tree-index form.
>
> **The counter-case is EmuTOS's own desktop**, which does call `rsrc_load`
> (`desk/deskapp.c:554`, `load_icon_file`) and reads the field back (`:571`,
> `hdr = (RSHDR *)(AP_1RESV)`, `include/aesdefs.h:153`). NewTOS's `desk/` has
> no such path and is not going to get one: ADR-022 is **rescinded** (user
> decision 2026-09-02); TeraDesk is the desktop. A second, independent
> condition would bind first in that case: **`desk/` keeps no persistent
> `AESGLOBAL`** — `Aes::new()` (`programs/desktop/src/lib.rs`) is a stack local in
> 26 wrappers, so EmuTOS's sequence `rsrc_load(); hdr = AP_1RESV;` — which
> relies on the module static `GLOBAL UWORD global[G_SIZE]`
> (`desk/gembind.c:66`) — could not be executed there at all.
>
> Tests: `tos-api/src/aes/resource.rs` `mod tests` (the representable branch,
> driven at the EL0 access/target split) and
> `tos-api/tests/aes_rsrc_global_fields.rs` (the refusal branch + the
> `ap_ptree[i] == rsrc_gaddr(R_TREE, i)` binding through the real dispatcher).

### `rsrc_free` (111) — `rs_free`

**Source:** `gemsuper.c:366-367` → `aes/gemrslib.c:685-708` (`rs_free`).

| Slot | Meaning |
|------|---------|
| (no `int_in`/`addr_in`) | operates on the caller's `AESGLOBAL` (`global`) |
| `int_out[0]` | `1` on success, `0` if nothing was loaded or a free failed |

Frees the CICON buffers (if any) and the resource blob `ap_rscmem`
(`dos_free`). Returns `0` if `ap_rscmem` was NULL (nothing loaded) or if any
`dos_free` failed (`gemrslib.c:691-705`). It does **not** clear `ap_ptree` — a
program must not `rsrc_gaddr` after `rsrc_free`. It acts on the **caller's**
`AESGLOBAL` only (`rs_sglobe(pglobal)`, `:690`): the blob that array names, and
no other process's load (A39 callout under `rsrc_load`).

**A stray `Mfree(ap_rscmem)` ahead of `rsrc_free`** is a legal, ordinary free
in EmuTOS — the blob is just another `Mxalloc` block — and the subsequent
`rsrc_free` then returns `0` on the resulting double free; see the A39
callout item 8 above and `docs/api/gemdos/memory/49-mfree.md` § "AES
resource block" (A76, 2026-09-20) for the full derivation and the NewTOS
target contract.

---

## 2. `rsrc_gaddr` / `rsrc_saddr` — get/set a resource address

### `rsrc_gaddr` (112) — `rs_gaddr`

**Source:** `gemsuper.c:369-371` → `aes/gemrslib.c:715-721` (`rs_gaddr`),
`get_addr` (`:112-176`). **This is the only AES opcode whose `addr_out[0]` the
dispatcher copies back to the caller** ([`_dispatcher.md`](./_dispatcher.md) §2,
`gemsuper.c:446-447`).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`RS_TYPE`) | `R_*` type code (0–16, §0) |
| `int_in[1]` (`RS_INDEX`) | index within that type's array |
| `addr_out[0]` (`RS_OUTADDR`) | the resolved address (`get_addr` result) |
| `int_out[0]` | `1` if found, **`0` if the type was invalid** (result was `-1L`) |

Resolves `(type, index)` to an absolute address per the `R_*` table (§0) and
writes it to `addr_out[0]`. The return is `(*rsaddr != -1L)` — `1` for any valid
type, `0` only for an out-of-range type code (`gemrslib.c:720`). **Note it does
not range-check `index`** — a too-large index returns `1` with a bogus pointer
(TOS behaviour; the resource's counts bound valid indices).

Two consequences of the `-1` test, both measured on the reference TT
(`measurements/measurement-2026-09-12-aes-r4/`, `tosdump/cases/aes-r4-subfunctions.tsv`):

* **An unknown type answers `addr_out[0] = -1`, not 0.** `get_addr`'s
  `default:` arm returns `(void *)-1L` (`:170-172`) without touching
  `rs_hdr`; `rs_gaddr` stores it (`:719`) and the dispatcher copies it to
  `addrout[0]` (`gemsuper.c:446-447`). Cases 623 (type 17), 624 (99), 625
  (−1 ⇒ 65535 as `UWORD`): `addr_out[0] = 0xFFFFFFFF`, `int_out[0] = 0`,
  with nothing loaded.
* **A NULL slot is "found".** `R_STRING`/`R_IMAGEDATA` return the *content*
  of the free-string / free-image slot (`:159-162`); a NULL there is `0`,
  and `0 != -1`, so `int_out[0] = 1` with `addr_out[0] = 0`.

> **NewTOS status (A50, 2026-09-20 — user decision, option (a)).** The
> return follows `rs_gaddr`'s `-1` rule: a NULL slot answers `1`/`0`, an
> unknown type answers `-1` at the caller's pointer width (`0xFFFFFFFF`
> ILP32, all ones LP64) with `0` — with or without a loaded resource, as in
> EmuTOS. **Kept, as a named deviation** (`docs/COMPATIBILITY.md`,
> "`rsrc_gaddr`/`rsrc_saddr` refuse an index past the table"): an index at
> or past the table's declared count, or a selector whose table the
> resource does not carry, answers `addr_out[0] = 0`, `int_out[0] = 0`
> where EmuTOS computes a pointer past the table and answers `1`. Nothing
> loaded (no blob the caller's `AESGLOBAL` names) is `0`/`0` for every
> defined type by the A39 contract (§1 callout item 4). Pinned by
> `tos-api/tests/aes_rsrc_gaddr_edges.rs`.

### `rsrc_saddr` (113) — `rs_saddr`

**Source:** `gemsuper.c:372-373` → `aes/gemrslib.c:728-742` (`rs_saddr`).

| Slot | Meaning |
|------|---------|
| `int_in[0]` (`RS_TYPE`) | `R_*` type code |
| `int_in[1]` (`RS_INDEX`) | index |
| `addr_in[0]` (`RS_INADDR`) | the address to store |
| `int_out[0]` | `TRUE(1)` on success, `FALSE(0)` if the type was invalid |

Stores `addr_in[0]` **into the resource's pointer slot** at `(type, index)`.
Only meaningful for the *pointer* types (`R_STRING`/`R_IMAGEDATA` via
`R_FRSTR`/`R_FRIMG`, the TEDINFO/ICONBLK sub-pointers, `R_OBSPEC`): `get_addr`
returns the address **of** the slot and `rs_saddr` writes through it
(`gemrslib.c:734-737`). Lets an application point a resource object at its own
runtime string/image/spec. Returns `FALSE` for a bad type
(`get_addr` == `-1L`). For any other `(type, index)` it stores through
whatever `get_addr` computed and returns `TRUE` — through a pointer past the
table for a too-large index, and through address **0** for `R_STRING`/
`R_IMAGEDATA` on a NULL slot.

> **NewTOS status (A50, 2026-09-20).** Unknown type → `FALSE`, nothing
> stored, as `rs_saddr`. Named deviation (`docs/COMPATIBILITY.md`): an index
> at or past the table, an absent table, nothing loaded, and a slot that
> resolved to NULL all answer `FALSE` with nothing stored — the kernel has no
> pointer past the table and no writable address 0 to store through.

---

## 3. `rsrc_obfix` (114) — char→pixel coordinate fix

**Source:** `gemsuper.c:375-377` → `aes/gemrslib.c:89-99` (`rs_obfix`),
`fix_chpos` (`:53-83`). Reads `gl_wchar`/`gl_hchar` (the system-font cell size
from `graf_handle`, see [`window-graphics.md`](./window-graphics.md)) and
`gl_width` (screen width).

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`RS_TREE`) | the OBJECT tree base |
| `int_in[0]` (`RS_OBJ`) | the object index to fix |
| `int_out[0]` | `1` (`rs_obfix` is `void`; `crysbind` `ret` stays `TRUE`) |

**The coordinate rule (contract).** Each of the object's four packed WORDs
`ob_x`, `ob_y`, `ob_w`, `ob_h` encodes an unsigned **character count in the low
byte** and a **signed pixel offset in the high byte** (`fix_chpos`,
`gemrslib.c:53-83`: `coffset = HIBYTE(cpos); cpos = LOBYTE(cpos);`):

| Field | offset | conversion |
|-------|-------:|------------|
| `ob_x` | 0 | `char × gl_wchar` |
| `ob_y` | 1 | `char × gl_hchar` |
| `ob_w` | 2 | `char × gl_wchar`, **except `char == 80` → `gl_width`** (full-screen width) |
| `ob_h` | 3 | `char × gl_hchar` |

where `char = LOBYTE(word)`, then **add the high-byte offset as signed**
(`coffset = HIBYTE(word); coffset > 128 ? coffset-256 : coffset`). So `ob_x =
0x0405` = char 5, offset +4 → `5·gl_wchar + 4` px. Get this exactly right —
every dialog's layout, and `rsrc_load`'s `fix_objects`, depend on it.
`rs_obfix` fixes **one** object; `fix_objects` applies it to all `rsh_nobs`
objects during load.

> **Byte order (corrected 2026-07-21).** An earlier revision of this table had
> the two bytes **reversed** ("character count in the high byte and a signed
> pixel offset in the low byte"). That is wrong: EmuTOS `fix_chpos`
> (`gemrslib.c:58-60`) reads `coffset = HIBYTE(cpos)` (the offset) and
> `cpos = LOBYTE(cpos)` (the char count, multiplied by the cell). The
> **low byte is the char count**, the **high byte is the signed pixel offset**.
> This also matches the raw QED AKTION values seen in the trace (`0x0401` =
> char 1 + offset 4, `0x2000` = char 0 + offset 32). The signed-offset and
> `width == 80 → gl_width` facts (`gemrslib.c:71-74,81`) are unchanged.

> **NewTOS status (implemented 2026-07-21, D-101; load-time fix 2026-09-19,
> A40).** `rsrc_obfix` (`tos-api/src/aes/resource.rs::rsrc_obfix`) performs
> the real char→pixel conversion above, in place, on the object the app
> names, through `resource::rs_obfix` — the **same function** the loader
> applies to every object of a char-grid (NRSC v9/v10) resource at
> `rsrc_load`, with the same live `gl_wchar`/`gl_hchar`/`gl_width`
> (`aes::graf`). An application's inline `rs_object[]` and a loaded `.RSC`
> therefore yield the same pixels for the same packed word, as in EmuTOS,
> where `rs_obfix` is the one function behind both (`gemrslib.c:89-99`,
> `:618`, `gemsuper.c:376`). A well-behaved GEM app does not re-`rsrc_obfix`
> a loaded object (EmuTOS converts it once at load, `gemrslib.c:606-637,
> 815-819`), and, like EmuTOS, NewTOS does not guard against one that does.
> Coordinates/tree are read at the caller's ABI width via `ObjTree`. Until
> A40 the converter (`tools/mkrsc.py`) did the arithmetic with `8 × 16 / 640`
> baked in and the loader converted nothing (audit B10).

---

## 4. File Selector (`fsel_*` 90–91)

Both opcodes call the same worker `fs_input(pipath, pisel, pbutton, pilabel)`;
they differ only in whether a custom title label is passed
(`gemsuper.c:322-326`).

### `fsel_input` (90) / `fsel_exinput` (91) — `fs_input`

**Source:** `gemsuper.c:322-326` → `aes/gemfslib.c:609-926` (`fs_input`).
Helpers: `fs_start` (build/centre the FSELECTR dialog, `:59-173`), `fs_active`
(read + sort the directory, `:268-337`), `fs_newdir`/`fs_format`/`fs_nscroll`
(fill + scroll the list), `fm_dial`(`FMD_START`/`FMD_FINISH`) + `fm_do` (the
modal loop), `set_mask`/`fs_pspec`/`fs_back` (path/mask split), `wildcmp` (TOS
wildcard match), `select_drive`, `inf_what` (decode the exit button),
`dos_sfirst`/`dos_snext` (GEMDOS directory read). **Globals read:** `drvbits`
(the BIOS drive bitmap → which drive buttons are enabled), `rs_trees[FSELECTR]`
(the resource dialog tree). Allocates a work area with `dos_alloc_anyram`.

| Slot | Meaning |
|------|---------|
| `addr_in[0]` (`FS_IPATH`) | **in/out** path buffer: directory + wildcard mask (e.g. `C:\FOO\*.TXT`) |
| `addr_in[1]` (`FS_ISEL`) | **in/out** filename buffer: the selected name (no path) |
| `addr_in[2]` (`FS_TITLE`) | **`fsel_exinput` only** — custom dialog label; **`fsel_input` passes NULL** |
| `int_out[1]` (`FS_BUTTON`) | **1 = OK, 0 = Cancel** (`inf_what` result; see below) |
| `int_out[0]` | `1` if the dialog ran, **`0`** if `path` was NULL or the work-area alloc failed |

> **The modal loop owns the screen and the mouse (A118, 2026-10-25).**
> `fm_do` opens with `fm_own(TRUE)` and closes with `fm_own(FALSE)`
> (`aes/gemfmlib.c:287,349`; `fs_input` calls it at `gemfslib.c:718`, and
> brackets the slider drag with its own pair, `:790-792`): the window-tree
> lock — a second process's selector waits while another holds it — and,
> on the outermost edge, the mouse and keyboard for the caller (`ct_chgown(
> rlr, &gl_rscreen)`, `:93`) and the menu bar parked. A press in a desk
> accessory's selector therefore completes no other process's parked
> `MU_BUTTON`. NewTOS brackets its loop once with `fm_own` and waits at the
> opcode's entry, before the dialog is drawn (named in
> [`window-graphics.md`](./window-graphics.md) "The AES's own takers (A118)").
> Receipts: `tos-api/tests/aes_a118_lock_takers.rs` (`fsel_input_*`).

**The path/name split (contract).** On entry `pipath` holds a full spec
`drive:\dir\mask`. `fs_input` splits it: `set_mask`/`fs_pspec`/`fs_back` walk
back to the last `\` (or the `:` drive separator) to separate the **directory
prefix** from the **wildcard mask** (`gemfslib.c:199-236,524-532`). An empty
mask defaults to `*.*` (`set_all_files`, `:530`). An empty `pipath` defaults to
`"A:\*.*"` with the current drive letter substituted (`:634-638`). The mask
drives the directory filter (`wildcmp`, `:300`); subdirectories always show
(and sort first, `\007` prefix, `:241-254`). On exit `pipath` is rewritten with
the final directory+mask (`locstr`) and `pisel` with the chosen filename
(unformatted, dot restored, `:914-916`).

**The exit button (contract).** `*pbutton = inf_what(tree, FSOK)`
(`gemfslib.c:922`, `util/optimize.c:157-172`): **1** if the OK button was
selected, **0** if Cancel (the object after OK), **−1** otherwise; the routine
also deselects that button. This is written to `int_out[1]`. So a caller reads
`int_out[0]` (did the dialog run) then `int_out[1]` (OK vs Cancel).

> **Corrections vs. the previous spec.** (1) The button return is **`int_out[1]`
> (`FS_BUTTON`)**, *not* an `addr_out` and not written "via the same buffers"
> (`aesdefs.h:465`). (2) `fs_input`'s **`int_out[0]` return is always 1 when the
> dialog ran** (0 only on NULL path / no memory) — the OK/Cancel outcome is
> `int_out[1]`, decoded by `inf_what`, and can be **−1** if neither button ended
> the loop. (3) `fsel_exinput`'s label is `addr_in[2]` (`FS_TITLE`), defaulting
> to the "Item Selector" string when NULL (`gemfslib.c:691-692`). See D-054.

> **NewTOS status (GEM.RSC lazy-load, D-100).** NewTOS has no in-ROM
> `rs_trees[FSELECTR]` (see the `ap_private`/D-099 note: NewTOS's AES kernel
> carries no built-in resource ensemble at all). Instead, `fsel_input`/
> `fsel_exinput` (`tos-api/src/aes/fsel.rs::load_fselectr`) load
> `C:\GEMSYS\RSC\GEM.RSC` (the EmuTOS `gem.rsc`, converted to native format by
> `tools/mkrsc.py convert`) into a **second, dedicated native-`.RSC` buffer**
> that coexists with whatever the calling application loaded via `rsrc_load`
> — the two do not share `ap_rscmem`. The load happens **lazily, on the
> first `fsel_input`/`fsel_exinput` call**, and the resolved tree root is
> cached (`SYS.root`) so later calls skip the file read. This is the concrete
> mechanism behind D-099's "the only resource NewTOS's AES loads is `GEM.RSC`,
> and only lazily, on first use of the file selector".

---

## 5. Colour icons (CICON) — load path (TASK/A41)

> **Scope.** This section extracts EmuTOS's `G_CICON` **load** path in full —
> the new-format `.RSC` on-disk layout, `fix_cicons`'s fix-up chain, and the
> `best_match` device-depth rule — so NewTOS can implement it. There is no
> separate opcode: CICON fix-up is part of `rsrc_load` (110)'s existing
> relocation walk (§0 step 2). The **draw** arm (`just_draw`/`gr_gicon`) is
> documented in [`object-form.md`](./object-form.md) (the `G_CICON` callout
> after its `just_draw` walkthrough), which this section cross-references
> rather than duplicates. Gated by `CONF_WITH_COLOUR_ICONS` (default **1**,
> `include/config.h:1266-1267`) — on in the Falcon030/TOS4 `make 512` target
> profile. Source: `aes/gemrslib.c` (`fix_cicons` `:549-573`, `get_ciconblkptr`
> `:484-508`, `fixup_all_ciconblks`/`fixup_colour_icons` `:185-256`,
> `best_match` `:267-282`, `expand_cicondata` `:284-343`, `transform_cicon`
> `:348-396`, `transform_all_cicons` `:403-477`, `free_cicon_buffers`
> `:515-536`, `fix_objects` G_CICON arm `:623-626`, `rs_readit`/`rs_free`
> call sites `:764-772,795-797,691-697`); `include/obdefs.h:203-217` (`CICON`/
> `CICONBLK`); `include/rsdefs.h:38-40` (`NEW_FORMAT_RSC`).
>
> **Naming correction vs. the task brief.** The device-depth selection
> function is named **`best_match`** in EmuTOS (`gemrslib.c:267`); there is
> no `select_cicon` symbol anywhere in `references/emutos` (checked,
> `grep -rn select_cicon references/emutos/` → 0 hits). This section uses the
> source name throughout.

### 5.1 New-format detection + the extension array

Already summarised in §0's `NEW_FORMAT_RSC` callout; `get_ciconblkptr`
(`gemrslib.c:484-508`) is the function that reads it:

```c
static CICONBLK **get_ciconblkptr(RSHDR *hdr)
{
    LONG *extarray;
    LONG cptr_offset;

    if ((hdr->rsh_vrsn & NEW_FORMAT_RSC) == 0)
        return NULL;

    /*
     * locate extension array, which has the following format (all longs):
     *  extarray[0]     true length of RSC file
     *  extarray[1]     offset of CICON table (-1L => none present)
     *  extarray[2]...  other extensions
     *  extarray[n]     0L indicates end of array
     */
    extarray = (LONG *)((char *)hdr + hdr->rsh_rssize);

    cptr_offset = extarray[1];
    if ((cptr_offset == 0L) || (cptr_offset == -1L))
        return NULL;

    return (CICONBLK **)((char *)hdr + cptr_offset);
}
```

EmuTOS reads **only** `extarray[0]` (already used by `rs_readit` for the true
file length, §0) and `extarray[1]` (the CICON table offset). `extarray[2]…`
("other extensions") is never read anywhere in `references/emutos` (grepped);
its meaning is **not derivable from this source** — flagged, not guessed.

**Measured this session** (`sdk/apps/teradesk/cicons.rsc`, 63 788 B, a real
DRI-compiled new-format resource — Python struct-unpack, not assumed):
`rsh_vrsn = 4` (`NEW_FORMAT_RSC` set), `rsh_rssize = 976` (classic size),
extension array at offset 976 = `(63788, 992, 61740, 0)`. So `extarray[0] =
63788` = the file's true length (matches `ls -la`), `extarray[1] = 992` = the
CICON pointer-table offset, `extarray[2] = 61740` is present but — per the
grep above — not interpreted by EmuTOS at all, and `extarray[3] = 0` is the
terminator. `61740`'s meaning is out of this session's scope; it is not
needed to implement `fix_cicons`, since EmuTOS itself ignores it.

### 5.2 The CICONBLK pointer table (on disk: a counted, sentinel-terminated array)

`fix_cicons` (`gemrslib.c:549-573`):

```c
static void fix_cicons(void)
{
    RSHDR *hdr = rs_hdr;
    CICONBLK **ciconblkptr, **p;
    CICON *cicondata;
    LONG num_ciconblks;

    ciconblkptr = get_ciconblkptr(hdr);
    if (!ciconblkptr)
        return;

    for (num_ciconblks = 0, p = ciconblkptr; *p != (CICONBLK *)-1L; p++)
        num_ciconblks++;

    cicondata = (CICON *)(p+1);        /* right after the sentinel */

    fixup_all_ciconblks(num_ciconblks, ciconblkptr, cicondata);
    transform_all_cicons(num_ciconblks, ciconblkptr);
}
```

**On disk**, at `hdr + cptr_offset`, the table is `num_ciconblks` placeholder
`LONG`s followed by a `-1L` sentinel — the *count* is not stored as a number
anywhere, it is derived by counting up to the sentinel. **Measured** (cicons.rsc,
offset 992): 38 `LONG`s of value `0`, then one `LONG` of `-1` (39 entries,
156 bytes) → `num_ciconblks = 38`. `fixup_all_ciconblks` (below) then
**overwrites** every table slot with the real address of the matching
CICONBLK; the on-disk `0` placeholders carry no information EmuTOS reads.

### 5.3 On-disk CICONBLK record (before fix-up)

`fixup_all_ciconblks`/`fixup_colour_icons` (`gemrslib.c:226-256, 185-217`)
walk the CICON data area (`cicondata`, right after the pointer-table
sentinel) as a sequence of variable-length records. **Derived from the code**
(the struct only gets its final in-memory shape after this pass; what
follows is the **pre-fix-up disk layout**, byte-for-byte, then independently
**confirmed against `cicons.rsc`'s real bytes** this session):

| Offset | Field | Size | On-disk content (pre-fix-up) |
|---:|---|---:|---|
| 0x00 | `monoblk.ib_pmask` | 4 | placeholder, overwritten — never read |
| 0x04 | `monoblk.ib_pdata` | 4 | placeholder, overwritten — never read |
| 0x08 | `monoblk.ib_ptext` | 4 | placeholder, overwritten — never read |
| 0x0C | `monoblk.ib_char` | 2 | **real** (icon overlay char + fg/bg colour) |
| 0x0E | `monoblk.ib_xchar` | 2 | real |
| 0x10 | `monoblk.ib_ychar` | 2 | real |
| 0x12 | `monoblk.ib_xicon` | 2 | real |
| 0x14 | `monoblk.ib_yicon` | 2 | real |
| 0x16 | `monoblk.ib_wicon` | 2 | **real, load-bearing** — mono icon width in px |
| 0x18 | `monoblk.ib_hicon` | 2 | **real, load-bearing** — mono icon height in px |
| 0x1A | `monoblk.ib_xtext` | 2 | real |
| 0x1C | `monoblk.ib_ytext` | 2 | real |
| 0x1E | `monoblk.ib_wtext` | 2 | real |
| 0x20 | `monoblk.ib_htext` | 2 | real |
| 0x22 | `mainlist` | 4 | **`num_cicons`** (count of `CICON`s for this block), later overwritten with the `CICON` chain head pointer (or `NULL`) |

Header = 38 bytes (34-byte `ICONBLK` + 4-byte `mainlist`/count). Immediately
following: `mono_words = (ib_wicon/16) × ib_hicon` **`WORD`s** of mono icon
data (`ib_pdata`), then `mono_words` `WORD`s of mono mask (`ib_pmask`), then a
**fixed** 12 bytes (6 `WORD`s) of icon-text buffer (`ib_ptext`) — then, if
`num_cicons > 0`, the `CICON` chain (§5.4); else the next `CICONBLK` starts
immediately (`gemrslib.c:236-254`).

**Measured** (`cicons.rsc` CICONBLK[0] @ file offset 1148, right after the
39-entry pointer table): `ib_wicon = ib_hicon = 32` → `mono_words = 64`;
`mainlist` (pre-fix-up) `= 1` (one `CICON` follows). All 38 CICONBLKs in this
file share `ib_wicon = ib_hicon = 32`, `mainlist = 1`, and are spaced exactly
1608 bytes apart (`2756 − 1148 = 4364 − 2756 = …`, checked for the first
three) — a uniform 32×32, single-variant icon set.

### 5.4 On-disk CICON record (per colour variant)

`CICON` (`include/obdefs.h:203-211`) and its fix-up (`fixup_colour_icons`,
`gemrslib.c:185-217`):

```c
static CICONBLK *fixup_colour_icons(LONG num_cicons, LONG mono_words, CICON *start)
{
    CICON *p = start;
    WORD *q;
    LONG i;

    for (i = 0; i < num_cicons; i++)
    {
        q = (WORD *)(p+1);                  /* point to start of data area */
        p->col_data = q;
        q += mono_words * p->num_planes;
        p->col_mask = q;
        q += mono_words;                    /* mask is one plane */
        if (p->sel_data)
        {
            p->sel_data = q;
            q += mono_words * p->num_planes;
            p->sel_mask = q;
            q += mono_words;                /* mask is one plane */
        }
        if (p->next_res == (CICON *)1L)     /* more CICONs follow */
            p->next_res = (CICON *)q;
        else p->next_res = NULL;
        p = (CICON *)q;
    }

    return (CICONBLK *)p;
}
```

| Offset | Field | Size | On-disk content (pre-fix-up) |
|---:|---|---:|---|
| 0x00 | `num_planes` | 2 | **real** — the plane depth of this colour variant |
| 0x02 | `col_data` | 4 | placeholder, overwritten |
| 0x06 | `col_mask` | 4 | placeholder, overwritten |
| 0x0A | `sel_data` | 4 | **a boolean flag**: nonzero ⇒ a SELECTED-state variant follows; tested with `if (p->sel_data)` (`:204`) *before* it is overwritten with the real pointer |
| 0x0E | `sel_mask` | 4 | placeholder, overwritten only if `sel_data` was set |
| 0x12 | `next_res` | 4 | **`(CICON *)1L`** ⇒ another `CICON` (a different plane depth) follows immediately after this one's data; any other value (the real files use `0`) ⇒ this is the **last** `CICON` of the chain |

Header = 22 bytes. Data area immediately following: `col_data` =
`mono_words × num_planes` `WORD`s (one plane per `WORD`-array, **standard**
— i.e. device-independent — planar form, `transform_cicon` sets
`fd_stand = TRUE` on it, `:351`), `col_mask` = `mono_words` `WORD`s (one
plane); if `sel_data` was flagged, `sel_data`/`sel_mask` follow in the same
shapes. `next_res == 1L` chains directly into the next `CICON` header at
this point — used for a resource that ships more than one plane-depth
variant per icon (e.g. a 1-plane **and** a 4-plane version); `cicons.rsc` has
none of that (every block's `next_res` is `0`).

**Measured** (CICON[0] of CICONBLK[0], file offset 1454 = `1148 + 38(header)
+ 64×2(mono data) + 64×2(mono mask) + 12(text)`): `num_planes = 4`,
`sel_data (flag) = 1`, `next_res = 0`. Checked identically for CICONBLK[1]
and [2]. So: **every icon in `cicons.rsc` is a single 4-plane (16-colour)
variant with a dedicated SELECTED-state image**, no multi-depth chain.

### 5.5 `fix_objects` — the `G_CICON` `ob_spec` (already noted in §0 edge case 6, detailed here)

```c
case G_CICON:
    if (ciconblkptr)
        obj->ob_spec = (LONG)ciconblkptr[obj->ob_spec];
    break;
```
(`gemrslib.c:623-626`). On disk, `ob_spec` of a `G_CICON` object is a plain
**0-based index** into the CICONBLK pointer table (§5.2) — not a byte offset
like every other relocatable `ob_spec` (§0 step 5). `fix_objects` runs
`rs_obfix(obj, 0)` (the char-grid coordinate fix, §3) **before** the type
switch, unconditionally for every object (`:617-618`) — a `G_CICON`
object's `ob_x`/`ob_y`/`ob_width`/`ob_height` go through the **same**
char→pixel rule as any other object type; there is no CICON-specific
coordinate handling.

**Measured** (`cicons.rsc`, 39 objects): object 0 is `G_BOX` (the root,
`ob_spec` a literal colour word, skipped by the box rule); objects 1–38 are
all `ob_type = G_CICON` with `ob_spec = 0, 1, 2, … 37` — exactly the CICONBLK
table index, one per icon, in table order. Object 1's packed `ob_width` /
`ob_height` words are `0x4800`/`0x2800` — **low byte (char count) = 0** in
both, so by §3's rule (`cpos = 0 × cell; cpos += signed high byte`) they
decode to exactly **72 × 40** px **regardless of the live font cell**
(high bytes `0x48 = 72`, `0x28 = 40`, both `< 128` ⇒ positive). This equals
CICONBLK[0]'s own `ib_wtext = 72` and `ib_hicon(32) + ib_htext(8) = 40`: the
*object's* bounding box is the icon-plus-label footprint used for desktop
grid layout, not the bare icon rectangle. (This cross-check is this
session's own arithmetic from two independently-read fields, not an EmuTOS
statement — labelled as **inference**, included because it is a useful
sanity check for whoever implements the coordinate/`ob_spec` split.)

### 5.6 `transform_all_cicons` — device-depth selection + expansion + transform

```c
static void transform_all_cicons(LONG num_cicons, CICONBLK **ciconblkptr)
{
    CICONBLK *ciconblk;
    CICON *cicon;
    WORD *colbuf, *selbuf, *expandbuf, *src;
    LONG data_size, n;
    BOOL expand;
    WORD i, w, h;

    for (i = 0; i < num_cicons; i++)
    {
        ciconblk = ciconblkptr[i];
        cicon = best_match(ciconblk);   /* find a suitable CICON */
        ciconblk->mainlist = cicon;
        if (!cicon)                     /* nothing suitable ... */
            continue;
        w = ciconblk->monoblk.ib_wicon;
        h = ciconblk->monoblk.ib_hicon;
        data_size = muls(w/8*gl_nplanes,h);
        expand = (cicon->num_planes != gl_nplanes); /* boolean */

        expandbuf = NULL;
        if (expand)
        {
            expandbuf = dos_alloc_anyram(data_size);
            if (!expandbuf)
            {
                ciconblk->mainlist = NULL;  /* no colour for this icon */
                continue;
            }
        }

        n = cicon->sel_data ? 2*data_size : data_size;
        colbuf = dos_alloc_anyram(n);
        if (!colbuf)
        {
            if (expandbuf)
                dos_free(expandbuf);
            ciconblk->mainlist = NULL;      /* no colour for this icon */
            continue;
        }

        src = cicon->col_data;
        if (expand)
        {
            expand_cicondata(src, expandbuf, cicon->col_mask, w, h, cicon->num_planes, gl_nplanes);
            src = expandbuf;
        }
        transform_cicon(src, colbuf, w, h, gl_nplanes);
        cicon->col_data = colbuf;

        if (cicon->sel_data)
        {
            selbuf = colbuf + data_size/sizeof(WORD);
            src = cicon->sel_data;
            if (expand)
            {
                expand_cicondata(src, expandbuf, cicon->sel_mask, w, h, cicon->num_planes, gl_nplanes);
                src = expandbuf;
            }
            transform_cicon(src, selbuf, w, h, gl_nplanes);
            cicon->sel_data = selbuf;
        }

        cicon->num_planes = gl_nplanes;     /* neatness only */
        cicon->next_res = NULL;

        if (expandbuf)
            dos_free(expandbuf);
    }
}
```
(`gemrslib.c:403-477`.) Per CICONBLK:

1. **`best_match`** (§5.7) picks the `CICON` variant; `ciconblk->mainlist` is
   set to it (or `NULL` — the same field that held the on-disk `num_cicons`
   count now holds the chosen pointer or nothing, §5.3). **`NULL` here is
   exactly the trigger for the draw-time mono fallback** (`object-form.md`
   §`G_CICON` callout): no colour art is available at the live plane depth.
2. **Alloc-failure is per-icon, not fatal to the load.** A failed
   `dos_alloc_anyram` for the expand buffer or the colour buffer sets
   `ciconblk->mainlist = NULL` and `continue`s to the next icon — the whole
   `rsrc_load` still succeeds, that one icon just falls back to mono. This
   is the same shape as `rs_readit`'s own short-read refusal (fail the *part*
   that cannot be satisfied, not silently corrupt memory) but at
   icon granularity, not file granularity.
3. **`expand_cicondata`** (`:284-343`, only when `cicon->num_planes !=
   gl_nplanes`) turns an S-plane source into a D-plane (D > S) destination:
   copy the S source planes verbatim into the first S destination planes;
   AND all S source planes together into one extra plane; copy that ANDed
   plane into every remaining destination plane; then AND the **mask**
   plane into every destination plane. (EmuTOS's own comment: "we use the
   same algorithm as Atari TOS".) **`best_match` never returns a variant
   with MORE planes than the device — the code path for `num_planes >
   gl_nplanes` (down-sampling) does not exist**, only expansion.
4. **`transform_cicon`** (§5.7's Vertragsfrage) converts the (possibly
   expanded) standard-form data to device form via `vrn_trnfm` →
   `vr_trnfm` (VDI opcode 110, [`raster/_raster.md`](../vdi/raster/_raster.md)).
5. `sel_data` is transformed the same way into the **second half** of the
   same `colbuf` allocation (`selbuf = colbuf + data_size/sizeof(WORD)`) —
   one allocation serves both images, which is why `rs_free`/
   `free_cicon_buffers` (§5.8) only ever frees `cicon->col_data`.
6. `cicon->num_planes = gl_nplanes` afterwards is cosmetic ("neatness
   only", the comment's own words) — the now-device-form data has already
   been expanded/transformed; nothing re-reads this field for a decision.

### 5.7 `best_match` — the device-depth selection rule (contract)

```c
static CICON *best_match(CICONBLK *start)
{
    CICON *p, *found = NULL;

    for (p = start->mainlist; p; p = p->next_res)
    {
        if (p->num_planes > gl_nplanes)     /* too many planes */
            continue;
        if (p->num_planes == gl_nplanes)    /* exact match */
            return p;
        if (!found || (p->num_planes > found->num_planes))
            found = p;                      /* best so far */
    }

    return found;
}
```
(`gemrslib.c:267-282`, comment `:258-265`: *"1. the CICON with the same
number of planes as the current resolution; 2. of those CICONS with fewer
planes than the current resolution, the one with the most number of
planes"*.) In prose: an **exact** `num_planes == gl_nplanes` match wins
immediately; otherwise the **highest** `num_planes` that is still `≤
gl_nplanes` is kept; a variant with **more** planes than the device is never
selected (never down-sampled); if no variant has `num_planes ≤ gl_nplanes`
at all, `best_match` returns `NULL` and the icon draws mono (§5.6 point 1).
This is the loader-time decision the draw arm (`object-form.md`) reads back
via `((CICONBLK *)spec)->mainlist` — the choice is made **once, at
`rsrc_load`**, not per draw call.

> **Contract question — which device-plane count does NewTOS report, and
> what does that mean for a truecolor CICON path?** `best_match` is
> compared against **`gl_nplanes`**, which is `gsx_nplanes()` = VDI
> `vq_extnd(1)`'s plane count (`aes/gemgraf.c:395`, `aes/gemgsxif.c:497-501`).
>
> **Today**, NewTOS's VDI backend reports a **4-plane, 16-colour CLUT**
> model — `fd_nplanes`/`v_planes` = **4**, not 16
> (`docs/api/vdi/raster/_raster.md:530`, `NUM_COLORS=16` in
> `kernel/src/vdi_backend.rs:21`; explicitly "NewTOS bring-up is palette
> (≤ 8 planes); the truecolor paths are deferred",
> `docs/api/vdi/raster/_raster.md:463-464`). Against `cicons.rsc`'s actual
> content — every CICONBLK offers exactly **one** 4-plane variant (§5.4,
> §5.6) — `best_match` would return an **exact match** for every icon:
> `expand_cicondata` is never invoked, and `transform_cicon`'s
> `CONF_WITH_VDI_16BIT`/`planes > 8` branch (below) never triggers. A first
> implementation against today's backend needs the **palette-model** path
> only.
>
> **Correction (A86, 2026-09-21):** this "Today" paragraph is stale — it
> was already superseded within this same document by §5.9's 2026-09-19
> (TASK/A41) implementation-status note, which says so explicitly: *"§5.7's
> 'today 4 planes' is wrong for the live backend"* (below,
> `kernel/src/vdi_backend.rs:41` `PLANES = 16`, `NUM_COLORS = 256`). The
> live backend is Falcon truecolor, so `best_match(16)` takes every
> 4-plane `cicons.rsc` variant as the best *lower* match, and
> `expand_cicondata` + `transform_cicon`'s `planes > 8` branch — described
> as deferred above — are the actual live path, implemented per §5.9. This
> correction is added here only to stop the paragraph above from being
> read on its own; §5.9 remains the authoritative, current account and is
> not repeated.
>
> **`transform_cicon`'s truecolor branch** (`gemrslib.c:359-395`, gated
> `#if CONF_WITH_VDI_16BIT`, active when `planes > 8`) reverses the bit
> order of the top byte of each transformed pixel `WORD` into an 8-bit
> index `n` (0–255) and looks up `phys_work.ext->palette[n]` — i.e. even in
> Falcon truecolor mode, EmuTOS's CICON pixels are still an **8-bit
> logical-pen code**, resolved through the VDI's 256-entry logical palette
> (`vdi_col.c:613`, `ext->palette[palnum] = (r<<11)|(g<<6)|b`), not a
> direct RGB value. This only matters for a machine where `gl_nplanes = 16`
> (the Falcon's one and only truecolor mode,
> `docs/analysis/vdi-audit/truecolor-conversion-plan.md:15-21`) — NewTOS's
> VDI does not report that today (previous paragraph), and the truecolor
> pivot itself is a distinct, **not-yet-implemented** decision (same
> document; 0 mentions of "cicon" in it, grepped). That plan's own §0
> recommendation is that NewTOS's **future** truecolor memory-MFDB wire
> format be genuine packed 16-bit RGB565 pixels, explicitly **not**
> EmuTOS's 256-entry logical-palette indirection
> (`truecolor-conversion-plan.md:35-40`) — which suggests EmuTOS's exact
> reversed-bit/`palette[n]` mechanism is *not* the right target for a
> future NewTOS truecolor CICON path either, but **no source read this
> session says what should replace it**. This is an open question for
> whichever task schedules the truecolor pivot, not resolved here — flagged,
> not guessed.

### 5.8 Freeing — `rs_free`/`free_cicon_buffers`

```c
static WORD free_cicon_buffers(RSHDR *hdr)
{
    CICONBLK **ciconblkptr, **p;
    CICON *cicon;
    WORD rc = 0;

    ciconblkptr = get_ciconblkptr(hdr);
    if (!ciconblkptr)
        return 0;

    for (p = ciconblkptr; *p != (CICONBLK *)-1L; p++)
    {
        cicon = (*p)->mainlist;
        if (cicon)
            if (dos_free(cicon->col_data))
                rc = -1;
    }

    return rc;
}
```
(`gemrslib.c:515-536`, called from `rs_free` under `#if
CONF_WITH_COLOUR_ICONS`, `:694-697`.) Walks the **same** pointer table,
freeing `cicon->col_data` for every block whose `mainlist` is non-`NULL`
(mono-fallback blocks have nothing allocated, §5.6 point 1). Because
`sel_data` lives in the second half of the **same** allocation (§5.6 point
5), one `dos_free(col_data)` releases both. This frees only the
`transform_all_cicons`-allocated **device-form** buffers; the original
file-resident standard-form `col_data`/`col_mask`/`sel_data`/`sel_mask`
(§5.4) die with the whole resource blob (`rs_hdr`) on the same `rs_free`
call, or with process end (§1's `rsrc_free`/process-death text).

### 5.9 NRSC format proposal — v11/v12 (design proposal, NOT an EmuTOS extraction)

> **Implementation status (2026-09-19, TASK/A41 Developer run) — read before
> the proposal below.** Converter, loader and drawer are implemented
> (`tools/mkrsc.py` `read_cicon_table`/`convert`; `tos-api/src/aes/
> resource.rs` `NRSC_VER_CICON`, `cicon_tables_are_sound`, `fix_cicons`,
> `best_match`, `transform_cicon`, `cicon_pixel`, `CiconDevice`;
> `tos-api/src/aes/object.rs` `Cicon`, `draw_icon`'s colour arm;
> `tos-api/src/vdi/raster.rs` `blit_device_mem`), host-tested
> (`tools/xtask/src/rsc_cicon_tests.rs`, `resource.rs` `cicon_tests`,
> `tos-api/tests/aes_cicon_draw.rs`). Where this section was a proposal, the
> following was **decided differently, with evidence**:
>
> * **The native CICONBLK/CICON records are the application's C structs**
>   (`CICONBLK { ICONBLK monoblk; CICON *mainlist; }`, `CICON { WORD
>   num_planes; WORD *col_data, *col_mask, *sel_data, *sel_mask; CICON
>   *next_res; }`, `obdefs.h:203-217` = gemlib `mt_gem.h`; LP64 56 + 48 B,
>   ILP32 40 + 24 B), **not** the `NrscCiconBlk`/`NrscCiconVariant` tables
>   proposed below: TeraDesk copies a CICONBLK **by value** (`sdk/apps/
>   teradesk/icon.c:1640,1675` `h = malloc(sizeof(CICONBLK)); *h =
>   *icons[n].ob_spec.ciconblk`, again `:2231`) and reads `monoblk` through
>   `ob_spec.ciconblk` (`dir.c:2090-2102`, `icon.c:803,1073,1227,1522`), so
>   the bytes behind a `G_CICON`'s `ob_spec` must *be* that struct, and the
>   drawer follows `mainlist` through the app's copy. The pointer fields
>   hold blob offsets (0 = absent / end of chain); `mainlist`/`next_res`
>   link the variants; the header (84 B, v11 LP64 / v12 ILP32) carries
>   `off_cib`/`n_cib` (CICONBLK array) and `off_cic`/`n_cic` (CICON array).
>   A `G_CICON`'s `ob_spec` is the byte offset of its CICONBLK, relocated by
>   the generic rule as proposed.
> * **Measured, not estimated:** the native `cicons.rsc` is **63 610 B**
>   (LP64; ILP32 61 770 B) against `RSC_CAP` 65 536 B — margin 1 926 B. Two
>   of §5.3/§5.4's "all 38" claims (checked there for the first three
>   blocks only) do not hold for the whole file: block 3 is **48×32**
>   (`mono_words = 96`), and blocks **20 and 31 have no SELECTED image**
>   (`sel_data` flag 0) — the `dither_mask` darkening path is therefore live
>   for two of TeraDesk's icons.
> * **§5.7's "today 4 planes" is wrong for the live backend:** `kernel/src/
>   vdi_backend.rs:41` `PLANES = 16` (Falcon truecolor, `NUM_COLORS = 256`),
>   and `appl_init` reports it as `ap_planes` (`aes/appl.rs`
>   `screen_planes`). `best_match(16)` therefore takes every 4-plane
>   variant as the *best lower* one, `expand_cicondata` (4 → 16 planes)
>   and `transform_cicon`'s `planes > 8` palette branch are the live path —
>   implemented per pixel in `cicon_pixel`. `phys_work.ext->palette[n]` is
>   indexed by **hardware register** (`set_color16` writes
>   `palette[MAP_COL[pen]]`, `vdi_col.c:607-613`); NewTOS folds `MAP_COL`
>   to the identity (`vdi/vwk.rs`), so the loader resolves `n` through
>   EmuTOS's `REV_MAP_COL` (`init_colors`, `:736-747`; `CiconDevice::
>   rev_map_col`) — same RGB word, e.g. index 15 → register 255 → pen 1.
> * **One device-form buffer per resource**, not per icon
>   (`transform_all_cicons` `:436`): the EL0 path maps every allocation as
>   a window block and the per-process table has `MAX_USER_BLOCKS` = 32
>   entries (`bdos/process/usermem.rs`) — fewer than TeraDesk's 38 icons.
>   The per-icon alloc-failure fallback (§5.6 point 2) is therefore per
>   resource. Freed by `rsrc_free` (`free_cicon_buffers`' counterpart) and
>   at process end.
> * **The acceptance's blocker (§5.10 item 3's QEMU run, §5.11 item 5) —
>   found and resolved 2026-09-19:** `rsrc_load("cicons.rsc")` = 1 and
>   `rsrc_gaddr(R_TREE, 0)` = 1 on QEMU raspi4b, but TeraDesk then resolved
>   the icon tree with its **own** `xrsrc_gaddr` (`rsc_load.c:496-541`,
>   `icon.c:2750`), which reads `aes_global[7]` (`ap_rscmem`) and parses the
>   memory there as a classic `RSHDR` (`rsh_trindex` at byte 18) — what
>   EmuTOS leaves there is the file image with the RSHDR at its head
>   (`rs_readit`, `rs_global->ap_rscmem = rs_hdr`, `gemrslib.c:787`); NewTOS
>   left the NRSC header, and TeraDesk faulted (Data Abort, FAR = the bytes
>   `"NRSC"` + version read as a pointer). Resolved by option (a) of §5.11
>   item 5: NRSC **v13/v14** put a native `RSHDR` in front of the blob (the
>   `rsrc_load` callout of §1), so the loaded image is the file as in
>   EmuTOS. `cicons.rsc` is 63 650 B (LP64) with the preamble.

> Everything below this line is a **NewTOS-native design proposal** for the
> Developer's review, per ADR-013 (the on-disk `.RSC` byte format is
> NewTOS's own choice, not a byte-compatibility target — same status as the
> rest of NRSC, `resource-fileselector.md` intro). What **is** contract is
> §5.6/§5.7's algorithm (best-match, expand, transform, alloc-failure
> fallback) and §5.4's SELECTED (`sel_data`/`sel_mask`) semantics — the
> table layout below is one reasonable way to carry the data those
> algorithms need; a different layout is equally conformant as long as the
> loader-visible behaviour holds.

**Today, both converters abort on a new-format file**:
`tools/mkrsc.py:262-263` (`convert`, the kernel-loadable NRSC path) and
`tools/mkrsc.py:516-517` (`convert_rshdr`, TeraDesk's own `xrsrc_load`
path) both `sys.exit("new-format (CICONBLK) .rsc not supported")` on
`vrsn & 0x0004`. `resource.rs`'s relocation core already has the `G_CICON`
special-case (`G_CICON => {}`, `relocate_blob_core` `:1268`,
`relocate_blob_core_ilp32` `:1113`) but nothing populates it, because
`mkrsc.py` never emits a nonzero `G_CICON` `ob_spec` (`nspec = 0  # …
G_CICON: unsupported, drawn empty`, `mkrsc.py:406`).

**Size estimate (this session's arithmetic from the real file, not a built
converter — labelled ESTIMATE).** The raw pixel/mask data for `cicons.rsc`'s
38 icons is a hard floor, independent of any table-layout choice: per icon,
mono data+mask = `2 × mono_words(64) × 2 B = 256 B`, plus one 4-plane
variant **with** a SELECTED image = `col_data(512 B) + col_mask(128 B) +
sel_data(512 B) + sel_mask(128 B) = 1280 B` (both figures measured from
§5.3/§5.4's real numbers) → `38 × (256 + 1280) = 58 368 B` of raster data
alone. Adding a rough table-overhead estimate — 76-byte header (v11, see
below), 39 × 32 B object array (1248 B), an 8-byte tree index, a 38-entry
`NrscCiconBlk` table (~1520 B at 40 B/entry) and a 38-entry
`NrscCiconVariant` table (~760 B at 20 B/entry) ≈ 3612 B — the estimated
total is **≈ 62 000 B**, against the current [`RSC_CAP`] of **65 536 B**
(`resource.rs:1481`) — a thin **≈ 3.5 KB (~5 %) margin**. **Contrary to
A38's framing** ("farbige Icons brauchen mehr Platz",
`TASK/A38-STATUS-DONE.MD:78-80`), a straight conversion of TeraDesk's actual
`cicons.rsc` **probably fits** inside the current cap — but this is an
estimate, not a measurement of a built file, and the margin is thin enough
that the Developer should re-measure once the converter exists rather than
treat this as settled (§5.11 item 2).

**Proposed header extension** (v11 = LP64, v12 = ILP32; same pattern as
v5/v6 adding the TEDINFO/ICONBLK/BITBLK bases, `mkrsc.py:220-223`): two more
`u32` fields, `off_ciconblk`/`n_ciconblk`, after the existing `off_bb`/`n_bb`
— a 76-byte header.

**Proposed `NrscCiconBlk` record** (fixed stride — unlike EmuTOS's
placeholder-and-sentinel on-disk chain, §5.2/§5.3, which exists only because
m68k in-place pointer-patching needs a compact, self-describing layout; the
converter is free to emit a flat, directly-indexable table instead):
`off_mono_data`/`off_mono_mask` (`u32` each, like `BitBlk::bi_pdata`), the 11
`ICONBLK` `WORD` fields (`ib_char`…`ib_htext`, unchanged meaning), `n_variants`
(`u32`), `off_variants` (`u32`) — a self-contained count instead of
EmuTOS's counted-by-chain-walk `mainlist`/`next_res` scheme (§5.2/§5.4),
because the converter already knows the count when it writes the file.

**Proposed `NrscCiconVariant` record**: `num_planes` (`u16`, 2 bytes pad),
`off_col_data`/`off_col_mask`/`off_sel_data`/`off_sel_mask` (`u32` each, `0`
= absent) — a flat array of `n_variants` such records per `NrscCiconBlk`,
replacing the `next_res` chain with plain indexing.

**`ob_spec` for a `G_CICON` object**: bake in the **direct blob byte offset**
of the object's `NrscCiconBlk` record (not an index into a runtime-resolved
pointer table, §5.5) — the loader then relocates it with the **same**
generic `ob_spec += target_base` rule used for every other non-BOX,
non-CICON type today (`relocate_blob_core`'s `ty => { … }` arm). The
existing `G_CICON => {}` special case is **deleted**, not filled in — no
observable behaviour depends on `ob_spec`'s on-disk representation (an
application only ever sees the *relocated* value through `rsrc_gaddr`
`R_OBSPEC`, which returns the address of the slot, not its content's
format).

**Where the plane-selection/expand/transform work should run.** Recommend
doing it in the **loader**, at `rsrc_load` time, exactly mirroring
`fix_cicons`/`transform_all_cicons` (§5.6) — not pre-baked into the
converter, even though today's plane count (4, §5.7) is a compile-time
constant for one kernel build and so *could* be pre-selected offline. This
mirrors the A40 correction of the analogous **B10** mistake (baking a
device-dependent conversion — the char→pixel coordinate arithmetic — into
the offline tool froze every resource to one screen profile,
`docs/analysis/audit-applinit-rsrc-load-2026-09-11.md`): storing the
**standard-form**, all-variants-present data in the file and resolving the
device depth at load time costs little and does not repeat that mistake if
the plane count ever becomes a runtime/per-boot property later (the
truecolor pivot, §5.7). This is a **reasoned recommendation**, not a strict
requirement today, since `gl_nplanes` does not currently vary within one
kernel build (`docs/api/vdi/raster/_raster.md:625-626`, "independent of
resolution").

**`rsrc_free` needs a place to record the transformed buffers.** Mirroring
§5.8, the loader must track which `dos_alloc_anyram`-equivalent allocations
`transform_all_cicons`-equivalent work made, so `rsrc_free` can release them.
NewTOS's current resource descriptor is a single kernel-wide static
(`RSC`/`USER_RSC_RAW`, §1's A39 callout) that the per-process work of A39
(not yet implemented) already needs to extend — this is an **additional**
dependency of A41 on that same extension, not a new one; flagged, not solved
here.

### 5.10 Build list for the Developer, with acceptance criteria

1. **Converter (`tools/mkrsc.py`)**: implement the `NEW_FORMAT_RSC` branch
   (§5.9) instead of `sys.exit`; size-assert the output against `RSC_CAP`
   (fail loudly — like the existing `assert len(blob) == total` — not
   silently truncate).
   - *Host test*: a small, **hand-built** two-icon classic new-format
     `.rsc` fixture (deterministic bytes, not `cicons.rsc` — a minimal
     fixture makes the expected native bytes checkable by hand) → converted
     → assert every field of `NrscCiconBlk`/`NrscCiconVariant` against
     hand-computed expected values.
   - *Acceptance*: `tools/mkrsc.py convert sdk/apps/teradesk/cicons.rsc`
     succeeds and reports the real `total_size` (compare against §5.9's
     ≈ 62 000 B estimate; if it exceeds `RSC_CAP`, **stop and bring the
     `RSC_CAP` question back to the user** — §5.11 item 2 — rather than
     silently raising the cap).
2. **Loader (`tos-api/src/aes/resource.rs`)**: extend `header_is_sound` to
   bounds-check `off_ciconblk`/`n_ciconblk` (fixed-stride, like the existing
   `fits()` closure) **and** every variant's four data offsets against
   `total_size` (variable per icon — needs its own walk, akin to
   `record_fits` for TEDINFO/ICONBLK/BITBLK today, not the single `fits()`
   call); replace the `G_CICON => {}` relocation arm with the generic
   offset relocation (§5.9); implement the `fix_cicons`-equivalent
   (`best_match`/`expand_cicondata`/`transform_cicon`, §5.6/§5.7) called
   from where `fix_objects_coords` is called today; implement the
   per-icon alloc-failure fallback exactly as §5.6 point 2 (continue, don't
   abort the load).
   - *Host test*: a converted small NRSC with 1-2 `NrscCiconBlk`s, 2
     plane-depth variants each, so `best_match`'s three branches (exact,
     best-lower, none) are each exercised by varying a mock device plane
     count; verify the resolved `CICON`/mono-fallback per case.
   - *Host test*: an artificially truncated CICON variant/table (mirroring
     the A38 short-blob pattern, `tests/aes_rsrc_short_blob.rs`) is
     **rejected**, not partially relocated.
3. **Drawer (`tos-api/src/aes/object.rs`, the `G_ICON | G_CICON` arm +
   `draw_icon`/a new `draw_cicon`)**: read the resolved `CICON` (post-load
   `mainlist`) when present, per `object-form.md`'s `G_CICON` callout
   (data/mask/sel selection, `gr_colourblit`-equivalent multi-plane blit,
   the `dither_mask` SELECTED-darken fallback when no `sel_data`); mono
   fallback (`cicon == NULL`) stays exactly today's code path — no
   regression for icons without colour variants.
   - *Host test*: a synthetic single-plane-depth CICON drawn against a mock
     `RasterBackend`, pixel-compared against hand-computed expected output.
   - *QEMU*: TeraDesk with `cicons.rsc` installed (`sdk/apps/
     build-teradesk.sh` stages it, `tools/make-sd-image.sh` copies it —
     since A41 part 2), `#Z`-autorun headless, screendump comparing colour
     icons against the mono baseline (done 2026-09-19,
     `TASK/A41-STATUS-DONE.MD`).

### 5.11 Open decisions for the user (not decided here)

1. **A39/A60 ordering.** TeraDesk's `load_icons` (`sdk/apps/teradesk/
   icon.c:2705-2766`) tries `rsrc_load("cicons.rsc")` through the real AES
   **before** falling back to its own private `xrsrc_load("icons.rsc")`, and
   restores `desktop.rsc`'s globals afterward (`:2762-2763`, error path
   `:2742-2743`) — relying on the AES's per-process `AESGLOBAL` selector
   following `ap_rscmem` the way EmuTOS's `rs_sglobe` does (§1's A39
   callout). NewTOS's resource descriptor is today one kernel-wide static
   (A39 "not yet implemented"); shipping `cicons.rsc` without A39 stage 1
   (per-process keying) risks one AES process's colour-icon load disturbing
   another process's `desktop.rsc` state, and A39 stage 2 (the `ap_rscmem`
   read-back selector) additionally needs **A60** fixed on LP64 before
   TeraDesk's own save/restore even writes the fields the kernel would read
   (`docs/analysis/rsrc-per-app-state-2026-09-19.md` §3, §4.3). **A41
   therefore needs A39 stage 2 (and hence A60) or a deliberate, documented
   stand-in — which one is the user's decision, not this spec's.**
2. **`RSC_CAP`.** §5.9's estimate suggests the current 64 KiB cap likely
   still covers `cicons.rsc` 1:1, with a thin, unverified ≈ 5 % margin.
   Whether to raise/dynamise it now regardless (A38's open question,
   `TASK/A38-STATUS-DONE.MD:167-170`) or wait for the real converter's
   number is the user's call.
3. **NRSC v11/v12 layout.** §5.9 is one reasonable design, not the only
   one — flagged for explicit sign-off since it is new format surface,
   unlike the rest of this spec, which only extracts existing EmuTOS
   behaviour.
4. **Truecolor CICON pixel path.** §5.7's contract question — EmuTOS's
   `phys_work.ext->palette[]` indirection is probably not what a future
   NewTOS truecolor path should do, but no source read this session says
   what should replace it. Needs its own task once/if the VDI truecolor
   pivot is scheduled. *(2026-09-19: implemented as EmuTOS does it, through
   the folded palette — see the §5.9 status callout; the pixel path is the
   live one today, not a future question.)*
5. **`ap_rscmem` must address an RSHDR-headed image (found 2026-09-19,
   blocks the acceptance).** EmuTOS's `rs_readit` publishes the loaded
   **file image** — classic `RSHDR` at its head — as `ap_rscmem`
   (`gemrslib.c:787`), and applications parse that header themselves:
   TeraDesk's `xrsrc_gaddr` (`sdk/apps/teradesk/rsc_load.c:496-541`, tho-
   otto's generic `rsc_load.c`) reads `aes_global[7]`, tests
   `IS_XRSC_HEADER` (`rsh_vrsn & 3 == 3` → the `RSXHDR` with `unsigned
   long` fields, else the 18-`u16` `RSHDR`), and takes
   `rsh_trindex`/`rsh_object`/… as offsets from that address. `load_icons`
   uses it on the AES-loaded `cicons.rsc` (`icon.c:2750`), so the NRSC
   image at `ap_rscmem` — `"NRSC"` magic, `u32` fields — is misread and the
   desktop faults. Two ways to honour the contract, both needing the user's
   sign-off (new in-memory-image surface, beyond ADR-013's on-disk
   freedom): **(a)** the converter emits a 36-byte native `RSHDR` (LE
   `u16`, classic field order, offsets from the file base — the
   representation `--rshdr` already writes and TeraDesk's parser already
   reads) padded to 40 B in front of the NRSC header (NRSC v13/v14; the
   loader finds the magic at 40); the loaded image then *is* the file, as
   in EmuTOS; or **(b)** `load_into_user` synthesizes that RSHDR into 40
   bytes in front of the blob at load time and publishes the shim as
   `ap_rscmem` (EL0 path only; the scratch experiment did this). Either
   way `rsh_rssize` and every offset are `u16`, so the image must stay
   below 64 KiB — `RSC_CAP` already says so (65 536; the last 40 bytes of
   that range would overflow and must be refused).
   *Decided 2026-09-19 by the user: **(a)**, implemented as NRSC v13/v14
   (`tools/mkrsc.py convert`, `resource.rs` `NRSC_VER_RSHDR`/`RSHDR_PAD`/
   `rshdr_is_consistent`; the `rsrc_load` callout of §1 has the contract).
   `cicons.rsc` is staged into `dist/` by `sdk/apps/build-teradesk.sh` and
   copied by `tools/make-sd-image.sh`; TeraDesk shows the colour icons on
   QEMU raspi4b (`TASK/A41-STATUS-DONE.MD`).*

---

## Trap interface (parameter-block form)

All calls enter via the GEM trap (`svc #2`) with **selector 200 in `x0`** and an
**AESPB pointer in `x1`** — see [`_dispatcher.md`](./_dispatcher.md).

| Op | Function | `int_in` | `addr_in` | `int_out` / `addr_out` |
|---:|----------|----------|-----------|------------------------|
| 90 | `fsel_input`   | — | `[0]`=path (io), `[1]`=name (io) | `[0]`=ran(1/0), `int_out[1]`=button(1/0) |
| 91 | `fsel_exinput` | — | `[0]`=path (io), `[1]`=name (io), `[2]`=label | `[0]`=ran(1/0), `int_out[1]`=button(1/0) |
| 110 | `rsrc_load`  | — | `[0]`=filename | `[0]`=1/0 |
| 111 | `rsrc_free`  | — | — | `[0]`=1/0 |
| 112 | `rsrc_gaddr` | `[0]`=type, `[1]`=index | — | `[0]`=1/0, **`addr_out[0]`=address** |
| 113 | `rsrc_saddr` | `[0]`=type, `[1]`=index | `[0]`=address | `[0]`=1/0 |
| 114 | `rsrc_obfix` | `[0]`=object | `[0]`=tree | `[0]`=1 (void) |

---

## Inputs / Outputs summary

Inputs arrive in the AESPB arrays. `rsrc_gaddr` is the sole AES call that returns
through **`addr_out[0]`**; everything else returns via `int_out[0]` (`1`/`0`) or
writes through a caller pointer (`rsrc_saddr` into the resource, `fsel_*` into
`pipath`/`pisel`, coordinates into the tree for `rsrc_obfix`). `fsel_*`'s
OK/Cancel is a *second* `int_out` slot, `int_out[1]`. **`rsrc_load`** writes the
caller's `AESGLOBAL` (`ap_rscmem`/`ap_rsclen`/`ap_ptree`, `gemrslib.c:787-788`,
`:595-597`); `rsrc_free`/`rsrc_gaddr`/`rsrc_saddr` only *read* those words back
(`rs_sglobe`, `:675-679`) and none of them clears them. The tree array
`ap_ptree` names lives inside the loaded blob, not in the caller's memory.

---

## Error conditions

The resource/file-selector managers return small boolean/index values, not TOS
`E*` codes:

| Condition | Source | Return |
|-----------|--------|--------|
| `rsrc_load` file not found via `sh_find` | `gemrslib.c:836-837` | `int_out[0] = 0` |
| `rsrc_load` open / short read / alloc failure | `gemrslib.c:758-783,839-841` | `int_out[0] = 0` |
| `rsrc_free` with nothing loaded, or `dos_free` failed | `gemrslib.c:699-705` | `int_out[0] = 0` |
| `rsrc_gaddr` with an invalid `R_*` type (17, 99, −1 …), loaded or not | `gemrslib.c:171-172,719-720`; r4 cases 623–625 | `addr_out[0] = -1` (caller's pointer width), `int_out[0] = 0` |
| `rsrc_saddr` with an invalid `R_*` type | `gemrslib.c:171-172,733-741` | `FALSE`, nothing stored |
| `rsrc_gaddr(R_STRING/R_IMAGEDATA)` on a NULL free-string / free-image slot | `gemrslib.c:159-162,720` | **not an error**: `addr_out[0] = 0`, `int_out[0] = 1` |
| `rsrc_saddr(R_STRING/R_IMAGEDATA)` on a NULL slot | `gemrslib.c:733-738` | EmuTOS: stores through address 0, `TRUE`. **NewTOS: `FALSE`, nothing stored** (named deviation, `docs/COMPATIBILITY.md`) |
| `rsrc_gaddr`/`rsrc_saddr` with an index at or past the table's count, or a table the resource lacks | `gemrslib.c:102-106` (no check) | EmuTOS: `1`/`TRUE` with a pointer past the table. **NewTOS: `addr_out[0] = 0`, `int_out[0] = 0` / `FALSE`, nothing stored** (named deviation, `docs/COMPATIBILITY.md`) |
| `rsrc_gaddr`/`rsrc_saddr` with nothing loaded (no blob the caller's `AESGLOBAL` names), defined type | `gemrslib.c:100-104` (EmuTOS parses the header behind `ap_rscmem`) | **NewTOS: `addr_out[0] = 0`, `int_out[0] = 0` / `FALSE`** (A39 contract, §1 callout item 4; r4 cases 598–621, 626, 627 measured with `rs_hdr == NULL` on both machines) |
| `fsel_input` with a NULL path pointer | `gemfslib.c:630-631` | `int_out[0] = 0` |
| `fsel_input` work-area alloc failure (`< NM_NAMES` slots) | `gemfslib.c:655-662` | `int_out[0] = 0` (after `ALFSMEM` alert) |
| `fsel_input` Cancel pressed | `gemfslib.c:922`, `optimize.c:168` | `int_out[0] = 1`, `int_out[1] = 0` |

A directory-read error inside the selector (`fs_active`) is shown to the user via
`form_error` and retried once; `EFILNF`/`ENMFIL` (empty/no-match dir) are treated
as success — an empty list, not an error (`gemfslib.c:330-336,733-754`).

---

## Verification criteria

- **`rsrc_gaddr(R_TREE, i)` returns the tree base:** after a successful
  `rsrc_load`, `rsrc_gaddr(type=0, index=i)` sets `addr_out[0] = ap_ptree[i]`
  and returns `int_out[0] = 1`; an out-of-range **type** (e.g. 17) returns
  `int_out[0] = 0` with `addr_out[0] = -1L` (`gemrslib.c:123-124,171-172,720`).
- **`rsrc_gaddr(R_OBJECT, i)` vs `R_OBSPEC`:** `type=1` returns
  `&object[i]`; `type=7` returns `&object[i].ob_spec` — the same object, a
  different field (`gemrslib.c:125-127,144-146`).
- **`rsrc_obfix` coordinate fix:** an object with `ob_x = 0x0405` (low byte =
  char 5, high byte = offset +4) becomes `5·gl_wchar + 4`; `ob_w = 0x0050`
  (char 80) becomes `gl_width` (full-screen); a high byte `0xFF` subtracts 1 px
  (`gemrslib.c:64-82`).
- **`rsrc_load` relocation:** after load, `ap_ptree[0]` and every object's
  relocated `ob_spec` are absolute addresses within `[ap_rscmem,
  ap_rscmem+ap_rsclen)`; a field stored as `-1L` on disk stays `-1L`
  (`gemrslib.c:581-585`).
- **`rsrc_load` not found → 0:** loading a filename `sh_find` cannot locate
  returns `int_out[0] = 0` and leaves `ap_rscmem` unchanged (`gemrslib.c:836`).
- **`rsrc_free` idempotency guard:** `rsrc_free` with `ap_rscmem == NULL`
  returns `0` (`gemrslib.c:702-704`).
- **`fsel_input` OK path:** given `pipath = "C:\*.TXT"`, on OK the call returns
  `int_out[0] = 1`, `int_out[1] = 1`, `pipath` = the final directory+mask, and
  `pisel` = the picked filename; on Cancel `int_out[1] = 0`
  (`gemfslib.c:914-922`).
- **`fsel_input` empty path default:** `pipath = ""` is rewritten to
  `"<curdrive>:\*.*"` before the dialog opens (`gemfslib.c:634-638`).
- **`fsel_input` NULL path:** `addr_in[0] == NULL` returns `int_out[0] = 0`
  without opening the dialog (`gemfslib.c:630-631`).
- **`fsel_exinput` label:** `fsel_exinput` shows the `addr_in[2]` string as the
  dialog title; `fsel_input` shows the default "Item Selector"
  (`gemfslib.c:691-692`).

> **Applicability note (D-100, updated D-101).** The criteria above verify the
> **EmuTOS source** (the primary specification, per CLAUDE.md Rule 1). The
> `rsrc_obfix` coordinate-fix criterion now **holds against NewTOS** (D-101):
> `rsrc_obfix` performs the real char→pixel conversion on the named inline
> object. The `sh_find`-based "not found" mechanism in the `rsrc_load`
> criterion now holds too (2026-09-11): `rsrc_load` runs the real `sh_find`,
> so the `int_out[0] = 0` outcome comes from the same four-stage search — bar
> stage (4), the AES `PATH=`, which is not implemented yet. The
> `rsrc_load` `ob_fix` criterion holds since A40 (2026-09-19): a loaded
> NRSC v9/v10 resource is converted at load with the live metrics by the
> same `rs_obfix` that `rsrc_obfix` runs; an earlier `mkrsc.py` pre-converted
> to pixels and the loader converted nothing. See the "NewTOS status"
> callouts under `rsrc_load (110)`/`rsrc_obfix (114)`.

---

## Edge cases discovered in source

1. **`rsrc_load` searches the shell path.** It calls `sh_find` first
   (`gemrslib.c:836`), so a bare `"MYAPP.RSC"` is resolved against the app dir,
   current dir, drive root, and the AES `PATH=` — not just the current
   directory. A program relies on this to ship an `.RSC` next to its executable.
2. **`R_*` type table is 0–16, and the "sub-pointer" codes alias their parent.**
   `R_TEPTEXT`(8)=`R_TEDINFO`(2), `R_IBPMASK`(11)=`R_ICONBLK`(3),
   `R_BIPDATA`(14)=`R_BITBLK`(4) — the sub-pointer is the struct's first field
   (`gemrslib.c:129-142`). `R_FRIMG`(16) was missing from the old table.
3. **`R_STRING`/`R_IMAGEDATA` dereference; `R_FRSTR`/`R_FRIMG` don't.** Codes 5/6
   return the pointed-to data; codes 15/16 return the address of the pointer
   slot (so `rsrc_saddr` can re-point a free string) (`gemrslib.c:159-170`).
4. **`rsrc_gaddr`/`rsrc_saddr` do not range-check the index.** Only the *type* is
   validated; a too-large index yields a pointer past the array
   (`gemrslib.c:102-106`) — the resource header's counts are the real bound.
5. **`rs_obfix` low byte is signed and width 80 is special.** `char == 80` in
   `ob_w`/`ob_x`-width position maps to `gl_width` (full screen); the low-byte
   offset >128 is treated as negative (`gemrslib.c:71-82`).
6. **Object relocation skips box `ob_spec`.** `G_BOX`/`G_IBOX`/`G_BOXCHAR` keep
   `ob_spec` as a colour word (not relocated); `G_CICON` `ob_spec` is an index
   turned into a CICONBLK pointer (`gemrslib.c:622-635`).
7. **New-format (CICON) files store the true length past `rsh_rssize`.** For
   `rsh_vrsn & NEW_FORMAT_RSC`, the real file size is a LONG at offset
   `rsh_rssize`, and an extension array (true length, CICON table offset) trails
   the classic data (`gemrslib.c:484-508,764-772`).
8. **Only one resource per application.** `ap_rscmem` holds a single blob;
   a second `rsrc_load` without `rsrc_free` leaks the first (`gemrslib.c:775`).
9. **File-selector directory read is deferred + retried once.** A bad path
   triggers `form_error` and one retry (reset to the previous or the drive
   root); empty/no-match directories are *not* errors (`gemfslib.c:330-336,
   730-754`).
10. **Folders sort first in the list.** Each entry is prefixed with `\007`
    (folder) or space (file) so the shell-sort puts directories at the top
    (`gemfslib.c:241-254,313-326`).
11. **`fsel_input`'s "OK" is refused after editing the mask.** If the user edits
    the path/mask field and presses OK, TOS (and EmuTOS) treat it as a *rescan*,
    not an exit — the button is deselected and the new mask is applied
    (`gemfslib.c:762-767`).

---

## Dependencies

### Requires (must be initialized / functional)

- **GEMDOS file I/O** — `rsrc_load` uses `Fopen`/`Fread`/`Fseek`/`Fclose` +
  `Malloc` (`dos_open`/`dos_read`/`dos_lseek`/`dos_close`/`dos_alloc_anyram`);
  `fsel_input` uses `Fsfirst`/`Fsnext` (`dos_sfirst`/`dos_snext`) and `Malloc`.
  All are implemented (Phase 08/09).
- **The shell path search** (`sh_find`, [`scrap-shell.md`](./scrap-shell.md)) —
  `rsrc_load` resolves the filename through it.
- **`graf_handle`** ([`window-graphics.md`](./window-graphics.md)) — `rs_obfix`
  needs `gl_wchar`/`gl_hchar`/`gl_width`, so the VDI workstation must be open.
- **The Object Manager + Form Manager** ([`object-form.md`](./object-form.md)) —
  `fsel_input` runs `fm_do` over the FSELECTR OBJECT tree and draws it with
  `objc_draw`; `fm_dial(FMD_START/FMD_FINISH)` reserves/redraws the screen.
- **The resource `gem.rsc`** — the FSELECTR dialog tree (drive buttons, the
  scrollable file box, OK/Cancel) is a built-in resource.

### Used by

- **Every GEM application** loads its UI via `rsrc_load` + `rsrc_gaddr`, and
  runs its Open/Save dialogs via `fsel_input`/`fsel_exinput`.
- **The NewTOS desktop** loads `DESK.RSC`/the GEM resource this way.
- `form_alert`/menus/windows all draw the OBJECT trees that `rsrc_load` produces.

---

## Related data structures

- **`RSHDR`** — the `.RSC` file header (§0, `rsdefs.h:16-36`). The byte layout
  and the offset/count semantics are **contract**; the on-disk offsets are
  big-endian (m68k).
- **`AESGLOBAL` resource fields** (`gemrslib.h:15-27`): `ap_ptree`
  (`OBJECT **`, the tree table), `ap_rscmem` (`RSHDR *`, the loaded blob),
  `ap_rsclen` (`UWORD`, its length). Filled by `rsrc_load`, read by the AES.
  NewTOS fills them too (see the `rsrc_load (110)` callout) but does **not**
  read them back — its descriptor is a kernel static, so the three words are a
  report to the application and never the AES's own state.
- **`OBJECT` / `TEDINFO` / `ICONBLK` / `BITBLK`** — the arrays the resource
  contains and `rsrc_gaddr` returns (see
  [`../structures/object.md`](../structures/object.md),
  [`tedinfo`](../structures/tedinfo.md),
  [`iconblk`](../structures/iconblk.md), [`bitblk`](../structures/bitblk.md)).
  `rs_obfix` fixes their coordinates; the relocation fixes their pointers.
- **FSELECTR dialog tree** (`gem.rsc`) — reference; the observable contract is
  the path/name split + the 1/0 button return, not the resource's object indices.

---

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). Cross-check of the `rsrc_*`/`fsel_*`
  opcode numbers, the `RSHDR` layout, the `R_*` type codes (0–16), the
  `rs_obfix` char→pixel rule, the `rsrc_gaddr` `addr_out` return, and the
  `fsel_*` `int_out[1]` button convention against Compendium/Profibuch is
  **deferred** until the PDFs are available. EmuTOS `1a7484ab` is authoritative
  per CLAUDE.md Rule 1. See D-054.

### AES managers

- [`scrap-shell.md`](./scrap-shell.md) — `rsrc_load` calls `shel_find`; the path
  search order lives there.
- [`object-form.md`](./object-form.md) — `rsrc_load` produces the OBJECT trees
  the object/form managers draw; `fsel_input` runs `form_do` over the FSELECTR
  tree.
- [`window-graphics.md`](./window-graphics.md) — `graf_handle` provides the cell
  sizes `rs_obfix` needs.
- [`_dispatcher.md`](./_dispatcher.md) — selector-200 call model; the
  `rsrc_gaddr`-only `addr_out[0]` copy-back.
- [`_state.md`](./_state.md) §6c — the loaded-resource `AESGLOBAL` fields.

### GEMDOS

- `Fopen`/`Fread`/`Fseek`/`Fclose`/`Malloc` (`rsrc_load`), `Fsfirst`/`Fsnext`
  (`fsel_input`) — the file/directory calls both use.

---

## Configuration / version notes

- **`CONF_WITH_COLOUR_ICONS`** (default **1**, `config.h:1267`): enables the
  `G_CICON`/CICONBLK fix-up + expansion + device-transform in `rsrc_load`
  (`fix_cicons`/`transform_all_cicons`) and the new-format (`NEW_FORMAT_RSC`)
  true-length read. With it off, only classic-format resources are handled and
  `G_CICON` objects are not fixed up. Full extraction of this path (on-disk
  format, load algorithm, `best_match` selection rule): §5 above (TASK/A41).
  The on-disk byte format + the internal fix-up-chain *shape* stay
  **reference** (NewTOS draws colour icons with its own rasteriser and, per
  §5.9, is free to choose its own on-disk layout); the `best_match` selection
  **rule**, the SELECTED `sel_data`/`sel_mask` semantics, and the mono
  fallback when no suitable variant exists are **contract** — the same
  contract/reference split §3 already applies to `rs_obfix`. **Note (D-100):**
  unlike the earlier
  wording here, the classic on-disk `RSHDR` byte format + its offset-table
  relocation are themselves **reference** for NewTOS too (ADR-013's native
  `.RSC`) — only the relocation's *shape* (offsets → pointers, box-`ob_spec`
  skipped) and the `rsrc_gaddr`/`rsrc_saddr`/`fsel_*` ABI are contract; see
  the intro deviation note.
- **`CONF_WITH_3D_OBJECTS`** (default **1**): `fs_start` sets FL3D\* flags and
  repositions the FSELECTR objects for the 3D look; otherwise it uses the flat
  layout (`gemfslib.c:59-172`). Reference (dialog appearance), not contract.
- `rsrc_load`/`rsrc_free`/`rsrc_gaddr`/`rsrc_saddr`/`rsrc_obfix` (110–114) and
  `fsel_input`/`fsel_exinput` (90–91) are **unconditional** across all EmuTOS
  builds and TOS versions.

---

## NewTOS implementation notes

- **(D-100, ADR-013 already decided — supersedes the older guidance below.)**
  NewTOS does **not** load EmuTOS's classic big-endian on-disk `.RSC`. It
  defines and loads its own **native** format (`NrscHeader`, magic `"NRSC"`,
  little-endian, 64-bit `ob_spec`, packed char-grid coordinates from v9/v10
  on — `tos-api/src/aes/resource.rs`), built from a classic `.rsc` by the
  host tool `tools/mkrsc.py convert`. Consequently: `rsrc_load` does **not**
  read the classic on-disk layout — it *does* call `sh_find` to locate the
  file (2026-09-11), and it *does* run the char→pixel `rs_obfix` over every
  object at load with the live `gl_wchar`/`gl_hchar`/`gl_width` (A40,
  2026-09-19; `rsrc_obfix` is the same function — see the callouts under
  `rsrc_load (110)` and `rsrc_obfix (114)` above). What **is** preserved
  exactly: the offset→pointer relocation *shape* (byte offsets from blob
  base → absolute pointers, type-aware for `ob_spec` per object type), the
  box-`ob_spec`-not-relocated rule, the char→pixel rule, and — as contract —
  the `rsrc_gaddr`/`rsrc_saddr` ABI slots, the `R_*` type semantics, and the
  `1`/`0` return convention. A classic-Atari-`.RSC` loader (the `RSHDR` byte
  layout, `-1L` sentinel, big-endian swap, `sh_find` lookup — all accurately
  extracted in §0–§3 above; the `sh_find` lookup and `rs_obfix` are
  implemented) remains a candidate for a future
  m68k-emulator compatibility phase, per ADR-013's own "alternatives
  rejected" note; it is not implemented today.
- **`R_*` type coverage (D-054, task #108).** `rsrc_gaddr`
  (`tos-api/src/aes/resource.rs`) now implements `R_TREE` (0), `R_OBJECT` (1)
  and the **free-string / free-image family** — `R_STRING` (5), `R_IMAGEDATA`
  (6), `R_FRSTR` (15), `R_FRIMG` (16), exactly per `get_addr` (§0): `R_FRSTR`/
  `R_FRIMG` return the *address of* the pointer slot (`&frstr[i]`/`&frimg[i]`),
  `R_STRING`/`R_IMAGEDATA` return the *dereferenced* data pointer
  (`frstr[i]`/`frimg[i]`), all bounds-checked against the resource's counts.
  This unblocked QED, whose `note()`/alert path fetches its free-string table
  via `rsrc_gaddr(R_FRSTR, NOWINDOW, &alertmsg)` (verified in QEMU: `R_FRSTR`
  now returns a real user VA and QED reaches its editor + menu bar, where the
  NULL table previously triggered a cflib `set_string()` panic). To carry the
  tables the native `.RSC` format grew a **free-string** and a **free-image**
  pointer table and a bumped version (v3 = LP64/44-byte header, v4 =
  ILP32/44-byte header); the loader stays **backward-compatible** — v1/v2
  files (GEM.RSC/DESK.RSC, no tables) load unchanged with those selectors
  reporting the honest "not found". Still-unimplemented type codes
  (`R_TEDINFO`…`R_BIPDATA`, `R_OBSPEC`, 2–4/7–14) remain the documented gap
  (fall through to `addr_out[0] = 0`, `int_out[0] = 0`); QED does not use them.
  `rsrc_saddr` writes the caller's pointer into the resolved slot at the
  blob's pointer width and returns 1, or 0 for an unknown type / an index
  past the table (`rs_saddr`, A50, `tos-api/src/aes/resource.rs`
  `rsrc_saddr`; the earlier "no-op stub" remark here described the state
  before A50 — corrected 2026-09-20).
- **`rsrc_gaddr` is the only `addr_out` writer.** NewTOS's dispatcher must copy
  `addr_out[0]` back for opcode 112 and no other (matching EmuTOS `xif`); this
  is unaffected by the native-format change and confirmed against
  `tos-api/src/aes/mod.rs`'s opcode 90–91/110–114 routing.
- `fsel_input`/`fsel_exinput` load the **real EmuTOS `FSELECTR` tree**
  (converted `GEM.RSC`, see the GEM.RSC-lazy-load callout under
  `fsel_input (90)` above), not a NewTOS-authored dialog — so the FSELECTR
  object indices (`FSDIRECT`, `FSSELECT`, `F1NAME`…, `FSOK`/`FSCANCEL`, the
  drive-letter grid) are the ones `aes/gem.rsc` defines, hard-coded in
  `tos-api/src/aes/fsel.rs`. The **path/mask split, the wildcard filter, the
  folders-first sort, and the 1/0 (`int_out[1]`) button return** are
  contract; the dialog's resource tree and its rendering are EmuTOS-derived
  but reference-level (a NewTOS-authored `FSELECTR` tree would be equally
  conformant as long as the objects `fsel_input` needs — path field,
  selection field, name-list slots, OK/Cancel, drive grid — exist and this
  contract behaviour holds).
- Pointers in `addr_in[]` (the filename, the path/name buffers, the store
  address) are **user (application) pointers** — apply the
  [`_dispatcher.md`](./_dispatcher.md) user/kernel boundary handling; `fsel_*`
  writes back into `pipath`/`pisel`, so those must be flushed to user memory.
- The `.RSC` loader + `rsrc_gaddr` are what let recompiled GEM apps (and the
  file selector's own `GEM.RSC`) run against a real resource tree → high
  value for compatibility, even though the on-disk byte format itself is
  NewTOS-native rather than the classic Atari one.

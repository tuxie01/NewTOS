# Desktop (Phase 14) — EmuTOS `desk/` source extraction

> **Phase-00 extraction (done late: 2026-06-22).** Source: EmuTOS @
> `1a7484ab`, the `desk/` directory. Per
> [`SOURCE-ANALYSIS-PROTOCOL.md`](../SOURCE-ANALYSIS-PROTOCOL.md): every
> claim below cites a `file:line`. This was written **after** an initial
> Batch-1 draft that guessed the structure from general GEM knowledge —
> the [reconciliation](#reconciliation-with-the-batch-1-draft) section
> records what the guess got wrong.
>
> The desktop is a GEM **application** built on the AES (Phase 13), not a
> set of trap functions — so this entry documents its *structure* (object
> trees, menu, drive icons, main loop) rather than a per-function ABI.

## EmuTOS sources read

- `desk/deskmain.c` — `deskmain()` main loop; `hndl_menu` (`:1012`),
  `do_deskmenu` (`:612`), `do_filemenu` (`:654`), `do_viewmenu` (`:786`),
  `do_optnmenu` (`:836`); `hndl_button` (`:947`); `men_update` (`:464`).
- `desk/deskapp.c` — `app_blddesk()` (the desktop OBJECT tree builder),
  `set_background` (`:`, the desktop pattern), the built-in
  `desk_inf_data1`/`desk_inf_data2` (`:171`, `:181`) and the `#M`
  drive-icon generation loop (`:714-728`), `g_patcol` defaults (`:824-829`).
- `desk/desktop.def` — the `ADMENU` menu object names (titles + items).
- `desk/desktop.rsc` — the displayed menu strings.

## Main loop (`deskmain`)

`deskmain()` (`deskmain.c`) sets up the screen, calls `menu_bar` to install
the desktop menu, `app_blddesk()` to build the desktop, then loops on
`evnt_multi` (keyboard + button + message). A returned `MN_SELECTED`
message routes to `hndl_menu(title, item)` (`deskmain.c:1012`); a button
event to `hndl_button` (`:947`); `WM_*` window messages to the window
handlers. After every menu action `menu_tnormal(…, title, 1)` un-highlights
the title (`deskmain.c:1035`).

## Menu (`ADMENU` tree)

The menu is the resource tree **`ADMENU`** with four titles
(`deskmain.c:1012` `hndl_menu` switch):

| Title (`#define`) | Text (`desktop.rsc`) | Dispatcher |
|---|---|---|
| `DESKMENU` | ` Desk ` | `do_deskmenu` (`deskmain.c:612`) |
| `FILEMENU` | ` File ` | `do_filemenu` (`:654`) |
| `VIEWMENU` | ` View ` | `do_viewmenu` (`:786`) |
| `OPTNMENU` | ` Options ` | `do_optnmenu` (`:836`) |

Item object names (from `desktop.def`, dispatched in the `do_*menu`
functions). Texts where read from `desktop.rsc`:

- **Desk**: `ABOUITEM` (“Desktop info...”), then desk-accessory slots
  registered via `menu_register`.
- **File**: `OPENITEM` (Open), `SHOWITEM` (Show Info), `SRCHITEM`,
  `NFOLITEM` (New Folder), `CLOSITEM` (Close), `CLSWITEM` (Close Window),
  `BTOPITEM`, `SLCTITEM` (Select all), `MASKITEM` (Set Mask), `DELTITEM`
  (Delete), `FORMITEM` (Format), `CLIITEM`, `QUITITEM` (Quit).
  - **`EJCTITEM` (Eject) is excluded:** `deskmain.c:744` gates it behind
    `#if CONF_WITH_EJECT`, whose general/Atari default is `0`
    (`include/config.h:2041`); it calls `flop_eject` → `lisa_flop_eject`
    (`bios/floppy.c:647`, `bios/lisa.c:554`) — i.e. it exists only for the
    **Apple Lisa** EmuTOS port (auto-eject drives), not Atari hardware. The
    Phase-00 draft listed it by mistake; the NewTOS File menu omits it.
- **View**: `ICONITEM` (Show as Icons), `TEXTITEM` (Show as Text),
  `NAMEITEM`/`TYPEITEM`/`SIZEITEM`/`DATEITEM`/`NSRTITEM` (sort order),
  `FITITEM`.
- **Options**: `IICNITEM`/`IAPPITEM`/`IDSKITEM` (Install …),
  `RICNITEM` (Remove desktop icon), `PREFITEM` (Preferences),
  `CONFITEM` (Desktop configuration, `#if CONF_WITH_DESKTOP_CONFIG` — kept),
  `RESITEM` (resolution), `READITEM`, `SAVEITEM` (Save desktop),
  `BLITITEM`/`CACHITEM`. `BACKGRND` (Set background) is
  `#if CONF_WITH_BACKGROUNDS` — kept.
  - **`BLITITEM` (Blitter) and `CACHITEM` (Cache) are excluded:** gated by
    `#if CONF_WITH_BLITTER` / `#if CONF_WITH_CACHE_CONTROL` — Atari STE/TT/
    Falcon blitter-chip and 68030+ CPU-cache control, hardware with no
    AArch64 equivalent in NewTOS. (Project-config decision, like
    `CONF_WITH_TT_SHIFTER`; the software features Set background / Desktop
    configuration are kept.)

`men_update` (`deskmain.c:464`) enables/disables items per context
(`menu_ienable`); e.g. `QUITITEM` is gated on `can_shutdown()` (`:582`).

## Desktop object tree + drive icons (`app_blddesk`)

`app_blddesk()` (`deskapp.c`) rebuilds the desktop:

1. `obj_wfree(DROOT, …)` frees the root's children and sizes it to the
   screen.
2. `set_background()` sets the **`DROOT` `ob_spec`** to a
   pattern+colour word — `G.g_patcol[n].desktop | BORDER_TEXT_COLOURS`,
   `n` chosen by plane count (`deskapp.c set_background`). Defaults from
   the built-in INF `#Q 41 40 43 40 43 40` (`deskapp.c:175`): for colour
   (`n=2`) `g_patcol[2].desktop = 0x43`. So the desktop background is a
   **GEM fill pattern + colour**, drawn by the AES as the `DROOT`
   `G_IBOX` — **not** a hand-rolled black/white dither.
3. For each `ANODE` flagged `AF_ISDESK`, allocate a child object
   (`obj_ialloc`) at its grid spot `(a_xspot, a_yspot)` sized
   `(g_wicon, g_hicon)`, set `ob_type = G_ICON`, and `ob_spec` to a copy
   of the disk `ICONBLK` from `G.g_iblist[icon]`. The icon is centred
   (`ib_xicon`), `ib_ptext` = the drive's name string, and **the drive
   letter is OR-ed into `ib_char`** (`pic->ib_char |= pa->a_letter`).

   The `ICONBLK` bitmaps live in `desk/icon.rsc` — 14 32×32 1bpp icons
   (`[0]` hard disk, `[1]` floppy disk, then folder/trash/printer/…), each
   a `data` + `mask` pair; the drive letter sits at the `(ib_xchar,
   ib_ychar)` cell. NewTOS extracts the hard-disk and floppy bitmaps into
   [`programs/desktop/src/icons.rs`](../../programs/desktop/src/icons.rs) and
   blits them (mask = opaque silhouette → body white / data black;
   transparent elsewhere). **Done in Batch 2b.**

The drives themselves are generated in the built-in INF `#M` loop
(`deskapp.c:714-728`):

- `drivemask = dos_sdrv(dos_gdrv())` — the present-drive bitmap.
- Each present drive `i` gets grid position
  `drive_x = icon_index % xcnt`, `drive_y = icon_index / xcnt` — i.e. they
  fill **left-to-right, top-to-bottom from the TOP-LEFT** (`:719-720`).
- `icon_type = (i > 1) ? IG_HARD : IG_FLOPPY` — drives A/B are floppies,
  C and up are **hard disks** (`:721`).
- `drive_letter = 'A' + i`, label text `STDISK` (“Disk”) (`:722-725`).

## Reconciliation with the Batch-1 draft

The initial kernel/src/desktop.rs (legacy, removed in ee84dd42) Batch-1 was guessed; the extraction
shows these **errors** to fix:

1. **Drive icons belong TOP-LEFT in a grid** (`deskapp.c:719`), not on the
   right edge.
2. **A drive icon is a `G_ICON`/`ICONBLK` hard-disk graphic** with the
   letter in `ib_char` and the name as `ib_ptext` — not a procedural box.
   (C/D are `IG_HARD`.)
3. **The desktop background is a GEM pattern+colour `ob_spec`** on the
   `DROOT` `G_IBOX` (`g_patcol`, default `0x43`), not a 50 % black/white
   dither.
4. **Menu titles carry surrounding spaces** (` Desk `/` File `/` View `/
   ` Options `) and the menus have the real item sets above (Batch 2).

These are corrected in Batch 1b before continuing.

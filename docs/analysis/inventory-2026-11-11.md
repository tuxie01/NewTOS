# Inventory 2026-11-11: frozen snapshot of what is implemented, measured and missing

Dated analysis for TASK/INV01.md. **Historical once written**; the living view is `cargo xtask report --coverage`, which does not exist yet (section 6 names its queries). Written read-only from the repository: HEAD `6e2d6b13a` (2026-11-11), EmuTOS graph at the pin `1a7484ab` (`docs/analysis/deps/edges.tsv`, 27137 lines), no run of QEMU or Hatari, no ticket filed, `references/emutos*` untouched.

## 0. How to read this document

### 0.1 Scope and units

- **One row = one graph node** (a function name, merged by name, as `layers.md` merges them). The node set is every name that occurs in `edges.tsv`: **2076 nodes** = 2011 nodes with a `def` row (2014 `def` rows; `bios_conws`, `do_bell`, `do_clip` are one node each, `layers.md` "Names merged into one node") + 65 `ext` nodes (no `def` row: `util/`, `cli/`, libc, `just_rts`, `trap1`).
- `layers.md` counts 2068 nodes. The 8 missing nodes all have a `def` row and no edge: `ikbdraw` (`bios/aciavecs.S:407`, called by `jsr ikbdraw` at `bios/aciavecs.S:391`, which the graph does not resolve) and 7 `include/` inlines (`bios_v_ww`, `xbios_v_w`, `xbios_v_wl`, `xbios_v_wwwl`, `xbios_v_wwwwww`, `xbios_v_llw`, `xbios_w_llwwwww`). They are in this inventory (every `def` row once); the discrepancy is an out-of-scope observation (section 8).
- **Graph function set vs TOS API function set.** The graph holds every function of `bios/`, `bdos/`, `vdi/`, `aes/`, `desk/`; the API is a small subset. The API entry points are read from the graph itself: the targets of the five `table` dispatchers of `layers.md` ("Dispatch tables"): `biostrap` (BIOS, 12 slots), `xbiostrap` (XBIOS, 142 slots), `osif` (GEMDOS, 88 slots), `crysbind` (AES, 87 `table` edges on 78 opcodes: an edge is a call inside a `case` arm of `aes/gemsuper.c`, so helpers called there are API-group nodes too: `gsx_sclip`, `combine_cms`, `fpdnm`, `inline_strcpy`, `do_chg`), `screen` (VDI, 74 slots), plus GEMDOS `Super` (0x20), which `bdos/bdosmain.c:221` leaves as `NI` and `enter` handles before the table (`bdos/rwa.S:172-184`). Several slots may share one target node (`ni` for 35 GEMDOS slots, `xbios_unimpl` for 39 XBIOS slots, `just_rts` for the VDI no-ops, `do_chg` for three `menu_*` calls), so a node row lists all its slots; slot-level counts are in 1.3.
- **Exactly one group per node**, first match wins: (1) API entry (a `table` edge target, or `enter`); (2) Line-A (`int_linea`, `linea_0`, `linea_init`, `set_screen_shift`, every `linea_*`, `get_pix`, `put_pix`, `lineA2Attrib`: the nodes of the Line-A table `linea_0` jumps through, `bios/linea.S:232-246`, and their direct helpers); (3) cookie-jar code (name contains `cookie`); (4) the directory of the `def` row (`bios/`, `bdos/`, `aes/`, `vdi/`, `desk/`, `include/`), or `ext`. "XBIOS internals" cannot be separated from BIOS internals: both live in `bios/` and the graph has no per-trap ownership of a helper, so `bios/ internals` is one group.
- **Out of the API scope, counted but not gap candidates:** `desk/` (325 rows; the desktop is an EL0 program, ADR-026, `docs/MEMORY.md:2433`), `include/` inline trap bindings (58) and `ext` (58 non-cookie). `mapping.tsv` has no rows for `desk/` and `include/` by construction (its header: "EmuTOS functions of layers without rows here ... count as unmapped"), so `cargo xtask deps --coverage` shows them 100% unmapped; this inventory reports them as `u` in the mapping column for consistency with that tool and as a single scope gap (G16), not 383 gaps.
- **Cookies and system variables are not graph nodes** (data, not functions). They get their own sections (3 and 4) with their own units: 22 cookie cases of `jar-r1.tsv`, 83 system-variable assignments of `references/emutos/tosvars.ld`. Counts there are not added to the node counts.

### 0.2 Columns and levels (all derived by script from the repository; nothing is from memory)

| Column | Source | Values |
|---|---|---|
| fn / def | `edges.tsv` `def` row | EmuTOS function, `file:line` of its first definition |
| slot / API | table edge `slot` of `edges.tsv`; API name from the AES `case` label (`aes/gemsuper.c`), the `vdi_` handler name, the inventory table of `docs/api/<layer>/_full-function-inventory.md`, else the handler | trap slot (hex = function number or opcode) |
| NewTOS | `docs/analysis/deps/mapping.tsv`; paths shortened: the prefix `ta` stands for the directory `tos-api/src`, `k` for `kernel/src` (written `ta/bdos/fs/file.rs::fopen`); `+n` = n more counterparts | `file::fn` of the first `mapped` row, `design:<evidence>`, `UNMAPPED`, `ext` (no mapping row possible) |
| c | class of the node: `unmapped` if any definition is unmapped, else `mapped` if any is, else `design` (the rule of `mapping.tsv`'s header, applied per node) | `m`, `d`, `u`, `e` |
| spec (API rows) | `docs/api/` | `F` = a per-function file whose number matches the slot; `H` = a heading of a non-meta spec file names the function (or its slot number in parentheses); `T` = a table row of a non-meta spec file of that layer names it in backticks; `I` = only `_full-function-inventory.md` of the layer names it (listed, no spec); `-` = nowhere. A cite follows for F/H/T. **Presence only: whether the text is correct against the source is not judged here.** |
| spec (internal rows) | same | `m` = the name occurs in a non-meta file of `docs/api/` (weak: a mention, not a spec); `i` = only in an inventory/dispatcher/working-list file; `-` = nowhere |
| oracle (API rows) | `tosdump/cases/*.tsv` (a case is a non-comment, non-`!`, non-`+` line; `+` lines are preludes, "issued before the measured call, never recorded", header of `aes-m06-sysvar.tsv`) joined to `tosdump/reference/EMUTOS-1a7484ab-*.BIN` by the table CRC32 (each table's CRC32 equals the `CRC` of exactly one reference `.LOG`; 39 of the 40 tables match a reference LOG, `aes-r4-subfunctions.tsv` matches none; for the 39 the case count equals the LOG count) | `O n: table*cases` = n recorded cases on the pinned oracle; `R14 n` = only on the EmuTOS 1.4 release (see 1.4); `none`; `(prelude only)` |
| R (internal rows) | graph reading, **not a measurement** | `R` = reachable over `direct`, `table`, `asm`, `layer`, `vector` edges from at least one API node that has an `O` case. `-` = no measured entry reaches it (it may still run at boot, from an interrupt, or through a callback/data edge) |
| dev | `docs/DEVIATIONS.md` rows (trigger column) and the bold title of each paragraph of `docs/COMPATIBILITY.md` "Sanctioned deviations, one by one" (lines 66-1852) | `D<n>` = `docs/DEVIATIONS.md:n`, `C<n>` = `docs/COMPATIBILITY.md:n`; a word match of the function or API name, so it **understates** (a deviation named only in a paragraph body is not found) |
| tickets | `TASK/<ID>.md` line 1 (title) names the function or API name as a word, state from line 3; OBSOLETE tickets are not shown (the `G-*` tickets were made OBSOLETE, "superseded by inventory-2026-11-11") | `ID/O` OPEN, `/I` IMPLEMENTED, `/P` PARKED, `/D` DONE, `/A` ACCEPTED; live (O, I, P) listed first, three shown, `+n` more. A title match is "a ticket that names it", not proof it covers the gap |

### 0.3 What this snapshot cannot say (marked unknown, not guessed)

- **Whether NewTOS equals the oracle on a measured case.** Equality needs a NewTOS run compared by `tosdiff`; no run was made here. "Measured" below means: a recorded case on the pinned oracle exists for this entry, i.e. the comparison *can* be made. Findings of past comparisons live in `tosdump/reference/FINDINGS-*.md` and the tickets; they are not re-derived.
- **Probe measurements** (`tosdump/probes/*.c`, `measurements/**`) are not attributed to functions; a function measured only by a probe shows `none` here. 552 run directories exist under `measurements/`; reading them per function is a separate piece of work.
- **Spec correctness** (the text against the EmuTOS source) is not re-validated; only presence.
- **Ticket coverage** is by title match (above); a ticket that treats a function under another name is not found.
- **Gating.** The graph is the `make 512 COUNTRY=de` build (`-DTARGET_512`, `edges.tsv` header): functions behind `#if CONF_WITH_*` that are off in that build are not nodes. Where a row depends on a gate it is named in the gap (G07: `CONF_WITH_LOADABLE_CURSORS`, `include/config.h:1290-1291`, default 1).

## 1. Counts

### 1.1 Nodes by group (every graph node in exactly one row; the rows are in section 2)

| Group | nodes | mapped | design | unmapped | ext | named in a deviation | named by a ticket title (non-OBSOLETE) | of those live (O/I/P) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| BIOS API entries (trap #13) | 12 | 12 | 0 | 0 | 0 | 2 | 10 | 5 |
| XBIOS API entries (trap #14) | 104 | 41 | 63 | 0 | 0 | 4 | 17 | 15 |
| GEMDOS API entries (trap #1) | 55 | 54 | 0 | 1 | 0 | 9 | 25 | 22 |
| bdos/ internals | 118 | 100 | 2 | 16 | 0 | 0 | 0 | 0 |
| AES API entries (trap #2, d0=0xc8) | 78 | 75 | 0 | 3 | 0 | 6 | 33 | 27 |
| aes/ internals | 341 | 290 | 9 | 42 | 0 | 4 | 18 | 14 |
| VDI API entries (trap #2, d0=0x73) | 68 | 67 | 0 | 0 | 1 | 18 | 17 | 7 |
| vdi/ internals | 167 | 139 | 27 | 1 | 0 | 0 | 2 | 0 |
| Line-A | 18 | 3 | 14 | 1 | 0 | 0 | 0 | 0 |
| Cookie-jar code | 10 | 1 | 2 | 1 | 6 | 0 | 0 | 0 |
| bios/ internals (BIOS/XBIOS helpers, drivers, boot) | 664 | 133 | 407 | 124 | 0 | 0 | 3 | 1 |
| desk/ (EmuDESK program) | 325 | 0 | 0 | 325 | 0 | 3 | 42 | 31 |
| include/ inline trap bindings | 58 | 0 | 0 | 58 | 0 | 0 | 0 | 0 |
| ext (util/, cli/, libc, no def in the node dirs) | 58 | 0 | 0 | 0 | 58 | 0 | 1 | 1 |
| **total** | **2076** | **915** | **524** | **572** | **65** | **46** | **168** | **123** |

`mapped` + `design` + `unmapped` + `ext` = nodes in every row. `design` rows are no gap: a decision (ADR, hardware, source) stands in `mapping.tsv`; their evidence classes are listed by `cargo xtask deps --coverage`.

### 1.2 Roll-up per layer (the seven layers of TASK/INV01.md; sums of the groups above)

| Layer | groups (nodes) | nodes | mapped | design | unmapped | ext |
|---|---|---:|---:|---:|---:|---:|
| BIOS + XBIOS (directory `bios/`: 12 BIOS entries, 104 XBIOS entries, internals) | 12 + 104 + 664 | 780 | 186 | 470 | 124 | 0 |
| GEMDOS / BDOS (`bdos/`) | 55 + 118 | 173 | 154 | 2 | 17 | 0 |
| AES (`aes/`) | 78 + 341 | 419 | 365 | 9 | 45 | 0 |
| VDI (`vdi/`) | 68 + 167 | 235 | 206 | 27 | 1 | 1 |
| Line-A | 18 | 18 | 3 | 14 | 1 | 0 |
| Cookie-jar code (graph part of "cookies/sysvars"; the data part is sections 3 and 4) | 10 | 10 | 1 | 2 | 1 | 6 |
| out of API scope: `desk/`, `include/`, `ext` | 325 + 58 + 58 | 441 | 0 | 0 | 383 | 58 |
| **total** | | **2076** | **915** | **524** | **572** | **65** |

Cross-check against `cargo xtask deps --coverage` (per definition, not per node; run 2026-11-11): aes 418 defs (365 mapped, 9 design, 44 unmapped), bdos 173 (154, 2, 17), bios 789 (188, 475, 126), desk 325 (0, 0, 325), include 59, vdi 250 (211, 38, 1), total 2014 defs: 918 mapped, 524 design, 572 unmapped. The per-node figures above differ because a node is one row however many definitions it has (three names have two definitions) and because the Line-A and cookie regrouping moves rows between directories; they are recomputed from the same two files.

### 1.3 API entries (rows of the five API groups) and API slots

| API group | nodes | real entries (not an empty slot) | mapped | spec F/H/T | oracle `O` | `R14` only | none | `O` for some slots only |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| BIOS API entries (trap #13) | 12 | 12 | 12 | 12 | 12 | 0 | 0 | 0 |
| XBIOS API entries (trap #14) | 104 | 103 | 40 | 61 | 47 | 0 | 56 | 0 |
| GEMDOS API entries (trap #1) | 55 | 54 | 53 | 54 | 15 | 0 | 39 | 0 |
| AES API entries (trap #2, d0=0xc8) | 78 | 78 | 75 | 78 | 17 | 21 | 40 | 0 |
| VDI API entries (trap #2, d0=0x73) | 68 | 68 | 67 | 68 | 54 | 0 | 14 | 0 |

Slot level (a slot = one distinct function number or opcode with a function behind it; spec level = the best level among the nodes at that slot; AES: 87 `table` edges fall on 78 distinct opcodes, because a `case` arm may call several functions, e.g. `gsx_sclip` before `ob_draw`; opcode 77 `graf_handle` is inline in `crysbind`, `aes/gemsuper.c:287`, has no node and therefore no row, but is in the AES case tables as opcode 77):

| layer | slots with a function | empty slots (`ni` / `xbios_unimpl`) | spec F | H | T | I | `O` | `R14` only | none |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| BIOS 0x0-0xb | 12 | 0 | 12 | 0 | 0 | 0 | 12 | 0 | 0 |
| XBIOS 0x0-0x8d | 103 | 39 | 26 | 26 | 9 | 42 | 47 | 0 | 56 |
| GEMDOS 0x0-0x57 | 54 | 35 | 54 | 0 | 0 | 0 | 15 | 0 | 39 |
| AES (opcodes) | 78 | 0 | 0 | 78 | 0 | 0 | 16 | 19 | 43 |
| VDI (opcodes) | 74 | 0 | 5 | 58 | 11 | 0 | 60 | 0 | 14 |

GEMDOS: the table `funcs` has 88 slots, 53 with a function and 35 `ni` (empty); `Super` (0x20) is one of the 35 `ni` slots and is answered by `enter` before the table, so the slot-level rows count it as a 54th slot with a function (`enter`) and keep 35 empty slots.

### 1.4 Case tables and the oracle references (`tosdump/cases/`, `tosdump/reference/`)

40 tables: 1832 recorded trap/opcode cases (non-prelude lines) plus the 22 cookie cases of `jar-r1`. Reference = `tosdump/reference/EMUTOS-1a7484ab-<name>.BIN` + `.LOG` (pinned oracle, Hatari, `docs/analysis/ORACLE.md`); a table is bound to a dump by its CRC32 (`zlib.crc32` of the file equals the `CRC` of the `.LOG`; computed for all 40, 39 match). The `falcontc` and `disk` variants of a reference are other profiles of the same table (`docs/analysis/ORACLE.md` section 2).

| table | cases | oracle reference (name after `EMUTOS-1a7484ab-`) |
|---|---:|---|
| aes-m05-addrin | 5 | AES-m05 |
| aes-m06-sysvar | 19 | AES-m06 |
| aes-m07-parameter | 20 | AES-m07 |
| aes-r1 | 95 | AES |
| aes-r2-wind | 78 | AES-r2wind |
| aes-r2b-rlist | 15 | AES-r2brlist |
| aes-r3-wind-state | 39 | AES-r3 |
| aes-r4-subfunctions | 181 | **none at the pin**; EmuTOS 1.4 release dump in `measurements/measurement-2026-09-12-aes-r4/emutos14-tt-hatari/EMUTOS14-AES-r4.BIN` (run 2026-09-12 on ROM `5805c0a4...`, `RUNINFO.TXT`); not under `tosdump/reference/` |
| bios-core-r1 | 44 | BIOS-core |
| bios-r1 | 147 | BIOS |
| bios-r2 | 51 | BIOS-r2-disk |
| bios-r2-phys | 12 | BIOS-r2-phys-disk |
| bios-r2-x001 | 8 | BIOS-r2-x001-disk |
| bios-r2-x004 | 10 | BIOS-r2-x004-disk |
| bios-r2-x005 | 13 | BIOS-r2-x005-disk |
| bios-r2-x006 | 11 | BIOS-r2-x006-disk |
| gemdos-mc-fs20 | 17 | BIOS-mc-fs20-disk |
| gemdos-mc-fs22 | 13 | BIOS-mc-fs22-disk |
| gemdos-mc-fs24 | 84 | BIOS-mc-fs24-disk |
| gemdos-mc-paths | 49 | BIOS-mc-paths-disk |
| super-r1 | 9 | SUPER |
| vdi-a131-palette | 48 | VDI-a131pal |
| vdi-all | 182 | VDI-all |
| vdi-r1 | 24 | VDI |
| vdi-r2 | 120 | VDI-r2 |
| vdi-r3 | 38 | VDI-r3 |
| vdi-raster | 59 | RASTER |
| vdi-raster-screen | 62 | RASTER-screen-falcontc |
| vdi-v18 | 19 | VDI-v18-falcontc |
| vdi-v18-raster | 17 | RASTER-v18-falcontc |
| vdi-v18-t2 | 32 | VDI-v18t2-falcontc |
| vdi-v18-t2-raster | 26 | RASTER-v18t2-falcontc |
| vdi-v18-t3 | 2 | VDI-v18t3-falcontc |
| xbios-bconmap-r1 | 1 | XBIOS-bconmap |
| xbios-chardev-r1 | 23 | XBIOS-chardev |
| xbios-flush-r1 | 207 | XBIOS-flush |
| xbios-ptr-r1 | 9 | XBIOS-ptr |
| xbios-system-r1 | 43 | XBIOS-system |

`aes-r4-subfunctions.tsv` is the only table without a pinned-oracle dump: the 21 AES API nodes (19 opcodes) that only it covers are `R14`. `ORACLE.md` section 4 states why EMUTOS14 references still count (calibration: 14 tables, 938 cases byte-identical under the toolchain change); `aes-r4` is not among those 14 tables (its own header declares the two AES commits between 1.4 and the pin, `RUNINFO.TXT` of the run), so its status is: measured against 1.4, not re-measured at the pin.

### 1.5 Internals reached by a measured API entry (graph reading)

| group | internals | `R` (reachable from an `O` entry) | `-` |
|---|---:|---:|---:|
| bdos/ internals | 118 | 107 | 11 |
| aes/ internals | 341 | 156 | 185 |
| vdi/ internals | 167 | 153 | 14 |
| bios/ internals (BIOS/XBIOS helpers, drivers, boot) | 664 | 332 | 332 |
| Line-A | 18 | 5 | 13 |
| Cookie-jar code | 10 | 1 | 9 |

145 API nodes have an `O` case and are the seeds; the closure reaches 1029 nodes. `-` rows of `design` class (hardware drivers, boot) are expected to be unreached; the figure that matters for a gap is the `-` of **mapped** rows: see G13.

## 2. Rows: every graph function, once

Legend in 0.2. Rows are sorted by slot (API groups) or by name. The XBIOS, GEMDOS and AES slot lists are hex. Section order: API groups, internals per layer, Line-A, cookie code, `bios/` internals, then the out-of-scope groups.

### 2.1 BIOS API entries (trap #13, `bios/bios.c:1245-1262` `bios_vecs`) - 12 rows

| slot | API | fn | def | NewTOS | c | spec | oracle | dev | tickets |
|---|---|---|---|---|---|---|---|---|---|
| 0x0 | Getmpb | `getmpb` | bios/biosmem.c:134 | `ta/bios/getmpb.rs::getmpb` +1 | m | F `docs/api/bios/00-getmpb.md` | O 1: xbios-ptr-r1*1 | - | BIOS06/I |
| 0x1 | Bconstat | `bconstat` | bios/bios.c:914 | `ta/bios/chardev.rs::bconstat` +2 | m | F `docs/api/bios/01-bconstat.md` | O 22: bios-core-r1*7,bios-r1*9,xbios-flush-r1*5 +1 tbl | - | - |
| 0x2 | Bconin | `bconin` | bios/bios.c:958 | `ta/bios/chardev.rs::bconin` +2 | m | F `docs/api/bios/02-bconin.md` | O 8: bios-core-r1*4,bios-r1*4 | - | BIOS09/I BIOS11/I BIOS08/D |
| 0x3 | Bconout | `bconout` | bios/bios.c:991 | `ta/bios/chardev.rs::bconout` +2 | m | F `docs/api/bios/03-bconout.md` | O 13: bios-core-r1*9,bios-r1*4 | C1365 | BIOS04/D BIOS08/D CON04/D +1 |
| 0x4 | Rwabs | `lrwabs` | bios/bios.c:1045 | `ta/bios/mod.rs::rwabs` | m | F `docs/api/bios/04-rwabs.md` | O 82: bios-r2*31,bios-r2-phys*12,bios-r2-x001*3 +7 tbl | - | TOOL07/D |
| 0x5 | Setexc | `setexc` | bios/bios.c:1070 | `ta/bios/vectors.rs::setexc` +4 | m | F `docs/api/bios/05-setexc.md` | O 22: bios-core-r1*17,bios-r1*5 | D27 C1457 | TOOL25/P BIOS05/A BIOS07/D +1 |
| 0x6 | Tickcal | `tickcal` | bios/bios.c:1096 | `ta/bios/mod.rs::dispatch` | m | F `docs/api/bios/06-tickcal.md` | O 1: bios-r1*1 | - | - |
| 0x7 | Getbpb | `getbpb` | bios/bios.c:1120 | `ta/bios/bpb.rs::getbpb` | m | F `docs/api/bios/07-getbpb.md` | O 21: bios-r2*8,bios-r2-x001*2,bios-r2-x004*3 +3 tbl | - | TOOL07/D |
| 0x8 | Bcostat | `bcostat` | bios/bios.c:1143 | `ta/bios/chardev.rs::bcostat` +2 | m | F `docs/api/bios/08-bcostat.md` | O 16: bios-core-r1*7,bios-r1*9 | - | BIOS04/D BIOS08/D |
| 0x9 | Mediach | `mediach` | bios/bios.c:1184 | `ta/bios/mod.rs::mediach` | m | F `docs/api/bios/09-mediach.md` | O 70: bios-r2*11,bios-r2-x001*3,bios-r2-x004*2 +6 tbl | - | TOOL07/D |
| 0xa | Drvmap | `drvmap` | bios/bios.c:1206 | `ta/bios/mod.rs::drvmap` | m | F `docs/api/bios/0a-drvmap.md` | O 2: bios-r1*1,bios-r2*1 | - | DESK11/P TOOL07/D |
| 0xb | Kbshift | `kbshift` | bios/ikbd.c:145 | `ta/bios/mod.rs::kbshift` +2 | m | F `docs/api/bios/0b-kbshift.md` | O 1: bios-r1*1 | - | KEYS04/P BIOS10/D BIOS12/D |

### 2.2 XBIOS API entries (trap #14, `bios/xbios.c:1285-1515` `xbios_vecs`, 142 slots; `LAST_ENTRY = 0x8d` with `CONF_WITH_DMASOUND`, `bios/xbios.c:1273-1283`) - 104 rows

| slot | API | fn | def | NewTOS | c | spec | oracle | dev | tickets |
|---|---|---|---|---|---|---|---|---|---|
| 0x0 | Initmous | `Initmous` | bios/mouse.c:40 | `ta/xbios/mod.rs::note_installer` | m | F `docs/api/xbios/00-initmous.md` | none | - | - |
| 0x1,0x24,0x27,0x28,0x2d,0x2f,0x30,0x31,0x32,0x33,0x34,0x35,0x36,0x37,0x38,0x39,0x3a,0x3b,0x3c,0x3d,0x3e,0x3f,0x41,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x49,0x4a,0x4b,0x4c,0x4d,0x4e,0x4f,0x5c,0xb | xbios_unimpl | `xbios_unimpl` | bios/vectors.S:1366 | `ta/xbios/mod.rs::dispatch` | m | n/a | n/a | - | - |
| 0x2 | Physbase | `physbase` | bios/screen.c:1042 | `ta/xbios/screen.rs::physbase` | m | F `docs/api/xbios/02-physbase.md` | O 1: bios-r1*1 | - | - |
| 0x3 | Logbase | `logbase` | bios/screen.c:1071 | `ta/xbios/screen.rs::logbase` | m | F `docs/api/xbios/03-logbase.md` | O 1: bios-r1*1 | - | - |
| 0x4 | Getrez | `getrez` | bios/screen.c:1076 | `ta/xbios/screen.rs::getrez` | m | F `docs/api/xbios/04-getrez.md` | O 1: bios-r1*1 | - | - |
| 0x5 | Setscreen | `setscreen` | bios/screen.c:1102 | `ta/xbios/screen.rs::setscreen` | m | F `docs/api/xbios/05-setscreen.md` | O 1: bios-r1*1 | - | - |
| 0x6 | Setpalette | `setpalette` | bios/screen.c:1168 | `ta/xbios/screen.rs::setpalette` | m | F `docs/api/xbios/06-setpalette.md` | none | - | - |
| 0x7 | Setcolor | `setcolor` | bios/screen.c:1192 | `ta/xbios/screen.rs::setcolor` | m | F `docs/api/xbios/07-setcolor.md` | O 3: bios-r1*3 | - | - |
| 0x8 | Floprd | `floprd` | bios/floppy.c:742 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:57` | none | - | - |
| 0x9 | Flopwr | `flopwr` | bios/floppy.c:749 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:57` | none | - | - |
| 0xa | Flopfmt | `flopfmt` | bios/floppy.c:898 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:63` | none | - | - |
| 0xc | Midiws | `midiws` | bios/midi.c:104 | `ta/xbios/system.rs::midiws` | m | F `docs/api/xbios/0c-midiws.md` | none | - | - |
| 0xd | Mfpint | `mfpint` | bios/mfp.c:114 | `ta/xbios/mfp.rs::mfpint` | m | F `docs/api/xbios/0d-mfpint.md` | none | - | - |
| 0xe | Iorec | `iorec` | bios/xbios.c:336 | `ta/xbios/iorec.rs::iorec` +1 | m | F `docs/api/xbios/0e-iorec.md` | O 19: bios-r1*5,xbios-chardev-r1*7,xbios-flush-r1*2 +1 tbl | C1524 | XBIOS01/D |
| 0xf | Rsconf | `rsconf` | bios/xbios.c:376 | `ta/xbios/system.rs::rsconf` | m | F `docs/api/xbios/0f-rsconf.md` | O 4: bios-r1*2,xbios-chardev-r1*2 | - | BIOS14/P XBIOS06/P |
| 0x10 | Keytbl | `keytbl` | bios/ikbd.c:122 | `ta/xbios/keytbl.rs::keytbl` | m | F `docs/api/xbios/10-keytbl.md` | O 2: bios-r1*1,xbios-chardev-r1*1 | - | XBIOS02/D |
| 0x11 | Random | `random` | bios/xbios.c:421 | `ta/xbios/system.rs::random` | m | F `docs/api/xbios/11-random.md` | O 1: bios-r1*1 | - | XBIOS11/P |
| 0x12 | Protobt | `protobt` | bios/floppy.c:690 | `ta/xbios/floppy.rs::protobt` | m | H `docs/api/xbios/_floppy-stubs.md:69` | O 36: bios-r1*5,xbios-system-r1*31 | - | - |
| 0x13 | Flopver | `flopver` | bios/floppy.c:797 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:57` | none | - | - |
| 0x14 | Scrdmp | `scrdmp` | bios/xbios.c:490 | `ta/xbios/system.rs::scrdmp` | m | F `docs/api/xbios/14-scrdmp.md` | none | D26 | TOOL35/P V34/D |
| 0x15 | Cursconf | `cursconf` | bios/vt52.c:893 | `ta/xbios/screen.rs::cursconf` +1 | m | F `docs/api/xbios/15-cursconf.md` | O 4: bios-r1*1,xbios-system-r1*3 | - | - |
| 0x16 | Settime | `settime` | bios/clock.c:1218 | `ta/gemdos/time.rs::xbios_settime` | m | F `docs/api/xbios/16-settime.md` | none | - | XBIOS08/P XBIOS13/P XBIOS14/P |
| 0x17 | Gettime | `gettime` | bios/clock.c:1266 | `ta/gemdos/time.rs::xbios_gettime` | m | F `docs/api/xbios/17-gettime.md` | O 1: bios-r1*1 | - | XBIOS09/P XBIOS13/P XBIOS14/P |
| 0x18 | Bioskeys | `bioskeys` | bios/ikbd.c:136 | `ta/xbios/keytbl.rs::bioskeys` +1 | m | F `docs/api/xbios/18-bioskeys.md` | none | - | XBIOS19/P XBIOS02/D |
| 0x19 | Ikbdws | `ikbdws` | bios/ikbd.c:948 | `ta/xbios/system.rs::ikbdws` +1 | m | F `docs/api/xbios/19-ikbdws.md` | none | - | XBIOS04/I |
| 0x1a | Jdisint | `jdisint` | bios/mfp.c:122 | design:ADR-031@docs/MEMORY.md:3044 | d | I | none | - | XBIOS16/P |
| 0x1b | Jenabint | `jenabint` | bios/mfp.c:127 | design:ADR-031@docs/MEMORY.md:3044 | d | I | none | - | XBIOS16/P |
| 0x1c | Giaccess | `giaccess` | bios/sound.c:94 | `ta/xbios/system.rs::giaccess` | m | H `docs/api/xbios/_psg-group.md:25` | O 2: bios-r1*2 | - | - |
| 0x1d | Offgibit | `offgibit` | bios/sound.c:130 | `ta/xbios/system.rs::offgibit` | m | H `docs/api/xbios/_psg-group.md:45` | none | - | - |
| 0x1e | Ongibit | `ongibit` | bios/sound.c:115 | `ta/xbios/system.rs::ongibit` | m | H `docs/api/xbios/_psg-group.md:45` | none | - | - |
| 0x1f | Xbtimer | `xbtimer` | bios/mfp.c:163 | `ta/xbios/mfp.rs::xbtimer` | m | H `docs/api/xbios/0d-mfpint.md:1` | none | - | - |
| 0x20 | Dosound | `dosound` | bios/sound.c:145 | `ta/xbios/system.rs::dosound` | m | H `docs/api/xbios/_psg-group.md:60` | O 1: bios-r1*1 | - | H04/P |
| 0x21 | Setprt | `setprt` | bios/parport.c:49 | `ta/xbios/system.rs::setprt` | m | F `docs/api/xbios/21-setprt.md` | O 1: bios-r1*1 | - | - |
| 0x22 | Kbdvbase | `kbdvbase` | bios/xbios.c:716 | `ta/xbios/kbdvecs.rs::kbdvbase` | m | F `docs/api/xbios/22-kbdvbase.md` | O 2: bios-r1*1,xbios-chardev-r1*1 | - | BIOS14/P KEYS02/P XBIOS03/A |
| 0x23 | Kbrate | `kbrate` | bios/ikbd.c:439 | `ta/xbios/system.rs::kbrate` | m | F `docs/api/xbios/23-kbrate.md` | O 2: bios-r1*1,xbios-chardev-r1*1 | - | - |
| 0x25 | Vsync | `vsync` | bios/screen.c:1205 | `ta/xbios/system.rs::vsync` | m | F `docs/api/xbios/25-vsync.md` | O 201: bios-r1*1,xbios-flush-r1*200 | - | - |
| 0x26 | Supexec | `supexec` | bios/vectors.S:1376 | `k/arch/aarch64/exceptions.rs::handle_supexec` | m | F `docs/api/xbios/26-supexec.md` | O 3: super-r1*3 | C896 | DESK13/I DESK27/D X01/D |
| 0x29 | Floprate | `floprate` | bios/floppy.c:1039 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:57` | O 3: bios-r1*3 | - | - |
| 0x2a | DMAread | `DMAread` | bios/disk.c:1241 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:122` | none | - | - |
| 0x2b | DMAwrite | `DMAwrite` | bios/disk.c:1254 | `ta/xbios/mod.rs::dispatch` | m | H `docs/api/xbios/_floppy-stubs.md:122` | none | - | - |
| 0x2c | Bconmap | `bconmap` | bios/serport.c:1082 | `ta/xbios/bconmap.rs::bconmap` +1 | m | F `docs/api/xbios/2c-bconmap.md` | O 16: bios-r1*4,xbios-bconmap-r1*1,xbios-chardev-r1*11 | C1558 C1576 | BIOS09/I BIOS14/P BIOS08/D +1 |
| 0x2e | NVMaccess | `nvmaccess` | bios/nvram.c:155 | `ta/xbios/mod.rs::dispatch` | m | I | O 15: bios-r1*6,xbios-system-r1*9 | - | XBIOS10/P |
| 0x40 | Blitmode | `blitmode` | bios/xbios.c:886 | `ta/xbios/system.rs::blitmode` | m | F `docs/api/xbios/40-blitmode.md` | O 1: bios-r1*1 | - | - |
| 0x50 | EsetShift | `esetshift` | bios/screen.c:180 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:30` | none | - | - |
| 0x51 | EgetShift | `egetshift` | bios/screen.c:210 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:31` | O 1: bios-r1*1 | - | - |
| 0x52 | EsetBank | `esetbank` | bios/screen.c:222 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:32` | O 1: bios-r1*1 | - | - |
| 0x53 | EsetColor | `esetcolor` | bios/screen.c:241 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:33` | O 3: bios-r1*3 | - | - |
| 0x54 | EsetPalette | `esetpalette` | bios/screen.c:265 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:34` | none | - | - |
| 0x55 | EgetPalette | `egetpalette` | bios/screen.c:290 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:35` | O 2: bios-r1*2 | - | - |
| 0x56 | EsetGray | `esetgray` | bios/screen.c:313 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:36` | O 1: bios-r1*1 | - | - |
| 0x57 | EsetSmear | `esetsmear` | bios/screen.c:334 | design:hardware:video-shifter | d | T `docs/api/xbios/_tt-shifter.md:37` | O 1: bios-r1*1 | - | - |
| 0x58 | VsetMode | `vsetmode` | bios/videl.c:619 | design:hardware:videl | d | I | O 1: bios-r1*1 | - | - |
| 0x59 | VgetMonitor | `vmontype` | bios/videl.c:664 | design:hardware:videl | d | I | O 1: bios-r1*1 | - | - |
| 0x5a | VsetSync | `vsetsync` | bios/videl.c:675 | design:hardware:videl | d | I | none | - | - |
| 0x5b | VgetSize | `vgetsize` | bios/videl.c:724 | design:hardware:videl | d | I | O 1: bios-r1*1 | - | - |
| 0x5d | VsetRGB | `vsetrgb` | bios/videl.c:806 | design:hardware:videl | d | I | none | - | - |
| 0x5e | VgetRGB | `vgetrgb` | bios/videl.c:862 | design:hardware:videl | d | I | O 1: bios-r1*1 | - | - |
| 0x5f | Vfixmode | `vfixmode` | bios/videl.c:917 | design:hardware:videl | d | I | O 1: bios-r1*1 | - | - |
| 0x60 | dsp_doblock | `dsp_doblock` | bios/dsp.c:222 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x61 | dsp_blkhandshake | `dsp_blkhandshake` | bios/dsp.c:259 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x62 | dsp_blkunpacked | `dsp_blkunpacked` | bios/dsp.c:296 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x63 | dsp_instream | `dsp_instream` | bios/dsp.c:326 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x64 | dsp_outstream | `dsp_outstream` | bios/dsp.c:350 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x65 | dsp_iostream | `dsp_iostream` | bios/dsp.c:425 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x66 | dsp_removeinterrupts | `dsp_removeinterrupts` | bios/dsp.c:494 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x67 | dsp_getwordsize | `dsp_getwordsize` | bios/dsp.c:508 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x68 | dsp_lock | `dsp_lock` | bios/dsp.c:599 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x69 | dsp_unlock | `dsp_unlock` | bios/dsp.c:611 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x6a | dsp_available | `dsp_available` | bios/dsp.c:619 | design:hardware:dsp56001 | d | I | O 2: bios-r1*2 | - | - |
| 0x6b | dsp_reserve | `dsp_reserve` | bios/dsp.c:628 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x6c | dsp_loadprog | `dsp_loadprog` | bios/dsp.c:650 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x6d | dsp_execprog | `dsp_execprog` | bios/dsp.c:670 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x6e | dsp_execboot | `dsp_execboot` | bios/dsp.c:693 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x6f | dsp_lodtobinary | `dsp_lodtobinary` | bios/dsp.c:922 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x70 | dsp_triggerhc | `dsp_triggerhc` | bios/dsp.c:954 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x71 | dsp_requestuniqueability | `dsp_requestuniqueability` | bios/dsp.c:965 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x72 | dsp_getprogability | `dsp_getprogability` | bios/dsp.c:976 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x73 | dsp_flushsubroutines | `dsp_flushsubroutines` | bios/dsp.c:987 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x74 | dsp_loadsubroutine | `dsp_loadsubroutine` | bios/dsp.c:1083 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x75 | dsp_inqsubrability | `dsp_inqsubrability` | bios/dsp.c:1146 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x76 | dsp_runsubroutine | `dsp_runsubroutine` | bios/dsp.c:1166 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x77 | dsp_hf0 | `dsp_hf0` | bios/dsp.c:1200 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x78 | dsp_hf1 | `dsp_hf1` | bios/dsp.c:1222 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x79 | dsp_hf2 | `dsp_hf2` | bios/dsp.c:1247 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x7a | dsp_hf3 | `dsp_hf3` | bios/dsp.c:1257 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x7b | dsp_blkwords | `dsp_blkwords` | bios/dsp.c:522 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x7c | dsp_blkbytes | `dsp_blkbytes` | bios/dsp.c:558 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x7d | dsp_hstat | `dsp_hstat` | bios/dsp.c:1273 | design:hardware:dsp56001 | d | I | O 1: bios-r1*1 | - | - |
| 0x7e | dsp_setvectors | `dsp_setvectors` | bios/dsp.c:1286 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x7f | dsp_multblocks | `dsp_multblocks` | bios/dsp.c:1338 | design:hardware:dsp56001 | d | I | none | - | - |
| 0x80 | locksnd | `locksnd` | bios/dmasound.c:215 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:183` | none | - | - |
| 0x81 | unlocksnd | `unlocksnd` | bios/dmasound.c:230 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:183` | none | - | - |
| 0x82 | soundcmd | `soundcmd` | bios/dmasound.c:342 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:197` | O 3: bios-r1*3 | - | - |
| 0x83 | setbuffer | `setbuffer` | bios/dmasound.c:369 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:234` | none | - | - |
| 0x84 | setsndmode | `setsndmode` | bios/dmasound.c:401 | design:hardware:dma-sound | d | T `docs/api/xbios/_dmasound-group.md:151` | none | - | - |
| 0x85 | settracks | `settracks` | bios/dmasound.c:424 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:254` | none | - | - |
| 0x86 | setmontracks | `setmontracks` | bios/dmasound.c:441 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:254` | none | - | - |
| 0x87 | setinterrupt | `setinterrupt` | bios/dmasound.c:457 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:261` | none | - | - |
| 0x88 | buffoper | `buffoper` | bios/dmasound.c:499 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:280` | O 1: bios-r1*1 | - | - |
| 0x89 | dsptristate | `dsptristate` | bios/dmasound.c:521 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:300` | none | - | - |
| 0x8a | gpio | `gpio` | bios/dmasound.c:545 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:306` | none | - | UART01/P |
| 0x8b | devconnect | `devconnect` | bios/dmasound.c:721 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:314` | none | - | - |
| 0x8c | sndstatus | `sndstatus` | bios/dmasound.c:751 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:345` | O 1: bios-r1*1 | - | - |
| 0x8d | buffptr | `buffptr` | bios/dmasound.c:772 | design:hardware:dma-sound | d | H `docs/api/xbios/_dmasound-group.md:364` | O 1: bios-r1*1 | - | - |

### 2.3 GEMDOS API entries (trap #1, `bdos/bdosmain.c:142-290` `funcs`, plus `Super` through `enter`) - 55 rows

| slot | API | fn | def | NewTOS | c | spec | oracle | dev | tickets |
|---|---|---|---|---|---|---|---|---|---|
| 0x0 | Pterm0 | `x0term` | bdos/proc.c:588 | `ta/bdos/process/pterm.rs::terminate_zero` | m | F `docs/api/gemdos/process/00-pterm0.md` | none | - | PROC07/P |
| 0x1 | Cconin | `xconin` | bdos/console.c:396 | `ta/gemdos/console.rs::cconin` | m | F `docs/api/gemdos/console/01-cconin.md` | none | - | - |
| 0x2 | Cconout | `xconout` | bdos/console.c:279 | `ta/gemdos/console.rs::cconout` | m | F `docs/api/gemdos/console/02-cconout.md` | none | - | CON02/I |
| 0x3 | Cauxin | `xauxin` | bdos/console.c:432 | `ta/gemdos/console.rs::cauxin` | m | F `docs/api/gemdos/console/03-cauxin.md` | none | - | - |
| 0x4 | Cauxout | `xauxout` | bdos/console.c:331 | `ta/gemdos/console.rs::cauxout` | m | F `docs/api/gemdos/console/04-cauxout.md` | none | - | - |
| 0x5 | Cprnout | `xprtout` | bdos/console.c:340 | `ta/gemdos/console.rs::cprnout` | m | F `docs/api/gemdos/console/05-cprnout.md` | none | - | - |
| 0x6 | Crawio | `xrawio` | bdos/console.c:441 | `ta/gemdos/console.rs::crawio` +1 | m | F `docs/api/gemdos/console/06-crawio.md` | none | - | - |
| 0x7 | Crawcin | `xrawcin` | bdos/console.c:375 | `ta/gemdos/console.rs::crawcin` | m | F `docs/api/gemdos/console/07-crawcin.md` | none | - | - |
| 0x8 | Cnecin | `xnecin` | bdos/console.c:414 | `ta/gemdos/console.rs::cnecin` | m | F `docs/api/gemdos/console/08-cnecin.md` | none | - | - |
| 0x9 | Cconws | `xconws` | bdos/console.c:459 | `ta/gemdos/console.rs::cconws` | m | F `docs/api/gemdos/console/09-cconws.md` | none | - | CON02/I |
| 0xa | Cconrs | `xconrs` | bdos/console.c:533 | `ta/gemdos/console.rs::cconrs` | m | F `docs/api/gemdos/console/0a-cconrs.md` | none | C1116 | - |
| 0xb | Cconis | `xconstat` | bdos/console.c:159 | `ta/gemdos/console.rs::cconis` | m | F `docs/api/gemdos/console/0b-cconis.md` | none | - | - |
| 0x16,0x17,0x18,0x1b,0x1c,0x1d,0x1e,0x1f,0x20,0x21,0x22,0x23,0x24,0x25,0x26,0x27,0x28,0x29,0x2e,0x32,0x33,0x34,0x35,0x37,0x38,0x4d,0x50,0x51,0x52,0x53,0x54,0x55,0xc,0xd,0xf | ni | `ni` | bdos/bdosmain.c:307 | `ta/gemdos/mod.rs::dispatch_inner` | m | n/a | n/a | - | - |
| 0xe | Dsetdrv | `xsetdrv` | bdos/fsmain.c:420 | `ta/bdos/fs/drive.rs::dsetdrv` | m | F `docs/api/gemdos/drive/0e-dsetdrv.md` | O 2: gemdos-mc-paths*2 | - | DESK11/P |
| 0x10 | Cconos | `xconostat` | bdos/console.c:168 | `ta/gemdos/console.rs::cconos` | m | F `docs/api/gemdos/console/10-cconos.md` | none | - | - |
| 0x11 | Cprnos | `xprtostat` | bdos/console.c:177 | `ta/gemdos/console.rs::cprnos` | m | F `docs/api/gemdos/console/11-cprnos.md` | none | - | - |
| 0x12 | Cauxis | `xauxistat` | bdos/console.c:186 | `ta/gemdos/console.rs::cauxis` | m | F `docs/api/gemdos/console/12-cauxis.md` | none | - | - |
| 0x13 | Cauxos | `xauxostat` | bdos/console.c:195 | `ta/gemdos/console.rs::cauxos` | m | F `docs/api/gemdos/console/13-cauxos.md` | none | - | - |
| 0x14 | Maddalt | `xmaddalt` | bdos/umem.c:455 | `ta/gemdos/memory.rs::maddalt` +1 | m | F `docs/api/gemdos/memory/14-maddalt.md` | none | - | - |
| 0x15 | Srealloc | `srealloc` | bdos/umem.c:358 | UNMAPPED | u | F `docs/api/gemdos/15-srealloc.md` | none | - | - |
| 0x19 | Dgetdrv | `xgetdrv` | bdos/fsmain.c:438 | `ta/bdos/fs/drive.rs::dgetdrv` | m | F `docs/api/gemdos/drive/19-dgetdrv.md` | O 3: gemdos-mc-paths*3 | - | DESK11/P |
| 0x1a | Fsetdta | `xsetdta` | bdos/fsmain.c:403 | `ta/bdos/fs/find.rs::fsetdta` | m | F `docs/api/gemdos/dta/1a-fsetdta.md` | O 3: gemdos-mc-fs22*1,gemdos-mc-fs24*1,gemdos-mc-paths*1 | - | - |
| 0x20 | Super | `enter` | bdos/rwa.S:159 | `ta/gemdos/mod.rs::dispatch` | m | F `docs/api/gemdos/process/20-super.md` | O 8: super-r1*6,xbios-ptr-r1*2 | C896 | TOOL26/P |
| 0x2a | Tgetdate | `xgetdate` | bdos/time.c:99 | `ta/gemdos/time.rs::tgetdate` | m | F `docs/api/gemdos/time/2a-tgetdate.md` | none | - | - |
| 0x2b | Tsetdate | `xsetdate` | bdos/time.c:108 | `ta/gemdos/time.rs::tsetdate` | m | F `docs/api/gemdos/time/2b-tsetdate.md` | none | - | - |
| 0x2c | Tgettime | `xgettime` | bdos/time.c:132 | `ta/gemdos/time.rs::tgettime` | m | F `docs/api/gemdos/time/2c-tgettime.md` | none | - | - |
| 0x2d | Tsettime | `xsettime` | bdos/time.c:142 | `ta/gemdos/time.rs::tsettime` | m | F `docs/api/gemdos/time/2d-tsettime.md` | none | - | - |
| 0x2f | Fgetdta | `xgetdta` | bdos/fsmain.c:394 | `ta/bdos/fs/find.rs::fgetdta` | m | F `docs/api/gemdos/dta/2f-fgetdta.md` | O 1: gemdos-mc-fs22*1 | - | - |
| 0x30 | Sversion | `xgetver` | bdos/bdosmain.c:298 | `ta/gemdos/version.rs::sversion` | m | F `docs/api/gemdos/version/30-sversion.md` | none | - | - |
| 0x31 | Ptermres | `xtermres` | bdos/proc.c:624 | `ta/bdos/process/pterm.rs::terminate_resident` | m | F `docs/api/gemdos/process/31-ptermres.md` | none | - | - |
| 0x36 | Dfree | `xgetfree` | bdos/fsfat.c:337 | `ta/bdos/fs/drive.rs::dfree` | m | F `docs/api/gemdos/directory/36-dfree.md` | O 13: gemdos-mc-fs20*3,gemdos-mc-fs24*7,gemdos-mc-paths*3 | - | - |
| 0x39 | Dcreate | `xmkdir` | bdos/fsdir.c:201 | `ta/bdos/fs/dirops.rs::dcreate` | m | F `docs/api/gemdos/directory/39-dcreate.md` | none | - | - |
| 0x3a | Ddelete | `xrmdir` | bdos/fsdir.c:297 | `ta/bdos/fs/dirops.rs::ddelete` | m | F `docs/api/gemdos/directory/3a-ddelete.md` | none | D56 | FS01/P |
| 0x3b | Dsetpath | `xchdir` | bdos/fsdir.c:1113 | `ta/bdos/fs/dirops.rs::dsetpath` +1 | m | F `docs/api/gemdos/directory/3b-dsetpath.md` | O 7: gemdos-mc-fs24*5,gemdos-mc-paths*2 | C1088 | FS24/A FS28/D |
| 0x3c | Fcreate | `xcreat` | bdos/fsopnclo.c:81 | `ta/bdos/fs/file.rs::fcreate` | m | F `docs/api/gemdos/file/3c-fcreate.md` | O 2: gemdos-mc-fs20*1,gemdos-mc-fs24*1 | D56 D57 | FS15/P |
| 0x3d | Fopen | `xopen` | bdos/fsopnclo.c:221 | `ta/bdos/fs/file.rs::fopen` | m | F `docs/api/gemdos/file/3d-fopen.md` | none | D52 | FS15/P |
| 0x3e | Fclose | `xclose` | bdos/fsopnclo.c:426 | `ta/bdos/fs/file.rs::xclose` +1 | m | F `docs/api/gemdos/file/3e-fclose.md` | O 2: gemdos-mc-fs20*1,gemdos-mc-fs24*1 | D53 | FS13/P FS21/D |
| 0x3f | Fread | `xread` | bdos/fsio.c:440 | `ta/bdos/fs/file.rs::xread` +1 | m | F `docs/api/gemdos/file/3f-fread.md` | none | - | FS11/I |
| 0x40 | Fwrite | `xwrite` | bdos/fsio.c:528 | `ta/bdos/fs/file.rs::xwrite` +1 | m | F `docs/api/gemdos/file/40-fwrite.md` | O 11: gemdos-mc-fs20*1,gemdos-mc-fs24*10 | - | FS05/P FS11/I FS16/P |
| 0x41 | Fdelete | `xunlink` | bdos/fsopnclo.c:623 | `ta/bdos/fs/file.rs::fdelete` | m | F `docs/api/gemdos/file/41-fdelete.md` | O 1: gemdos-mc-paths*1 | D56 | FS01/P FS25/P |
| 0x42 | Fseek | `xlseek` | bdos/fsio.c:321 | `ta/bdos/fs/file.rs::fseek` | m | F `docs/api/gemdos/file/42-fseek.md` | none | - | FS10/D |
| 0x43 | Fattrib | `xchmod` | bdos/fsdir.c:402 | `ta/bdos/fs/file.rs::fattrib` | m | F `docs/api/gemdos/file/43-fattrib.md` | none | - | - |
| 0x44 | Mxalloc | `xmxalloc` | bdos/umem.c:234 | `ta/gemdos/memory.rs::mxalloc` +1 | m | F `docs/api/gemdos/memory/44-mxalloc.md` | none | C1033 | MEM01/P MEM02/D |
| 0x45 | Fdup | `xdup` | bdos/fshand.c:93 | `ta/bdos/fs/file.rs::fdup` | m | F `docs/api/gemdos/file/45-fdup.md` | none | - | - |
| 0x46 | Fforce | `xforce` | bdos/fshand.c:32 | `ta/bdos/fs/file.rs::fforce` | m | F `docs/api/gemdos/file/46-fforce.md` | none | - | FS13/P PROC02/D |
| 0x47 | Dgetpath | `xgetdir` | bdos/fsdir.c:1229 | `ta/bdos/fs/dirops.rs::dgetpath` | m | F `docs/api/gemdos/directory/47-dgetpath.md` | O 8: gemdos-mc-fs24*5,gemdos-mc-paths*3 | - | FS28/D |
| 0x48 | Malloc | `xmalloc` | bdos/umem.c:116 | `ta/gemdos/memory.rs::malloc` +1 | m | F `docs/api/gemdos/memory/48-malloc.md` | none | - | MEM05/P PORT11/P |
| 0x49 | Mfree | `xmfree` | bdos/umem.c:135 | `ta/gemdos/memory.rs::mfree` +1 | m | F `docs/api/gemdos/memory/49-mfree.md` | none | - | A76/P MEM08/P |
| 0x4a | Mshrink | `xsetblk` | bdos/umem.c:170 | `ta/gemdos/memory.rs::mshrink` +1 | m | F `docs/api/gemdos/memory/4a-mshrink.md` | none | - | MEM08/P |
| 0x4b | Pexec | `xexec` | bdos/proc.c:179 | `ta/bdos/process/pexec.rs::dispatch` +2 | m | F `docs/api/gemdos/process/4b-pexec.md` | O 1: gemdos-mc-fs22*1 | - | A107/P PROC01/P PROC06/I +2 |
| 0x4c | Pterm | `xterm` | bdos/proc.c:601 | `ta/bdos/process/pterm.rs::terminate` +1 | m | F `docs/api/gemdos/process/4c-pterm.md` | none | - | MEM08/P |
| 0x4e | Fsfirst | `xsfirst` | bdos/fsdir.c:523 | `ta/bdos/fs/find.rs::fsfirst` | m | F `docs/api/gemdos/file/4e-fsfirst.md` | O 10: gemdos-mc-fs22*2,gemdos-mc-fs24*5,gemdos-mc-paths*3 | - | FS19/P |
| 0x4f | Fsnext | `xsnext` | bdos/fsdir.c:654 | `ta/bdos/fs/find.rs::fsnext` | m | F `docs/api/gemdos/file/4f-fsnext.md` | O 30: gemdos-mc-fs22*2,gemdos-mc-fs24*19,gemdos-mc-paths*9 | - | - |
| 0x56 | Frename | `xrename` | bdos/fsdir.c:895 | `ta/bdos/fs/file.rs::frename` | m | F `docs/api/gemdos/file/56-frename.md` | none | - | FS02/P FS06/P FS14/P +1 |
| 0x57 | Fdatime | `xgsdtof` | bdos/fsdir.c:684 | `ta/bdos/fs/file.rs::fdatime` | m | F `docs/api/gemdos/file/57-fdatime.md` | none | - | - |

### 2.4 bdos/ internals - 118 rows

| fn | def | NewTOS | c | spec | R | dev | tickets |
|---|---|---|---|---|---|---|---|
| `addit` | bdos/fsio.c:33 | `ta/bdos/fs/fsio.rs::addit` | m | m | R | - | - |
| `alloc_env` | bdos/proc.c:413 | `ta/bdos/process/loader.rs::alloc_env_for` +1 | m | m | R | - | - |
| `alloc_tpa` | bdos/proc.c:460 | `ta/bdos/process/arena.rs::alloc_tpa` | m | m | R | - | - |
| `backsp` | bdos/console.c:494 | `ta/gemdos/console.rs::backsp` | m | m | R | - | - |
| `bdos_trap2` | bdos/rwa.S:145 | UNMAPPED | u | m | - | - | - |
| `bufl_init` | bdos/fsbuf.c:56 | `ta/bdos/fs/cache.rs::initial_order` | m | i | - | - | - |
| `buflush` | bdos/console.c:252 | `ta/gemdos/console.rs::flush` | m | - | R | - | - |
| `builds` | bdos/fsdir.c:737 | `ta/bdos/fs/find.rs::fcb_pattern` +1 | m | m | R | - | - |
| `cgets` | bdos/console.c:540 | `ta/gemdos/console.rs::edit_line` +1 | m | m | R | - | - |
| `ckdrv` | bdos/fsdrive.c:77 | `ta/bdos/fs/fat.rs::mount` | m | m | R | - | - |
| `cl2rec` | bdos/fsfat.c:23 | `ta/bdos/fs/fat.rs::cl2rec` | m | m | R | - | - |
| `clfix` | bdos/fsfat.c:36 | `ta/bdos/fs/fat.rs::set_fat_entry` | m | m | R | - | - |
| `conbrk` | bdos/console.c:206 | `ta/gemdos/console.rs::conbrk` | m | m | R | - | - |
| `conin` | bdos/console.c:384 | `ta/gemdos/console.rs::cconin` | m | m | R | - | - |
| `conout` | bdos/console.c:263 | `ta/gemdos/console.rs::conout` | m | m | R | - | - |
| `constat` | bdos/console.c:143 | `ta/gemdos/console.rs::constat` | m | m | R | - | - |
| `contains_illegal_characters` | bdos/fsopnclo.c:708 | `ta/bdos/fs/path.rs::contains_illegal_characters` | m | m | R | - | - |
| `contains_wildcard_characters` | bdos/fsdir.c:505 | `ta/bdos/fs/dirops.rs::dsetpath` | m | m | R | - | - |
| `cookdout` | bdos/console.c:312 | `ta/gemdos/console.rs::io_cookdout` | m | m | R | - | - |
| `countfree16` | bdos/fsfat.c:299 | `ta/bdos/fs/fat.rs::count_free_clusters` | m | m | R | - | - |
| `create_chain` | bdos/fsbuf.c:29 | `ta/bdos/fs/cache.rs::initial_order` | m | - | - | - | - |
| `dcrack` | bdos/fsdir.c:1695 | `ta/bdos/fs/path.rs::split_drive` | m | m | R | - | - |
| `decr_curdir_usage` | bdos/fsdir.c:1194 | `ta/bdos/fs/curdir.rs::decr` | m | m | R | - | - |
| `dirinit` | bdos/fsdir.c:1257 | `ta/bdos/fs/dir.rs::grow_dir` | m | m | R | - | - |
| `dopath` | bdos/fsdir.c:1365 | UNMAPPED | u | m | R | - | - |
| `envsize` | bdos/proc.c:150 | `ta/bdos/process/standin.rs::env_image` | m | m | R | - | - |
| `eof` | bdos/fsio.c:294 | `ta/bdos/fs/file.rs::eof` | m | m | R | - | - |
| `ffit` | bdos/iumem.c:35 | `ta/bdos/mem/pool.rs::ffit` | m | m | R | - | - |
| `find_mpb` | bdos/umem.c:99 | `ta/bdos/mem/traps.rs::find_mpb` | m | m | R | - | - |
| `findfree` | bdos/fsfat.c:208 | `ta/bdos/fs/fat.rs::findfree` | m | m | R | - | - |
| `findfree16` | bdos/fsfat.c:174 | `ta/bdos/fs/fat.rs::findfree16` | m | i | R | - | - |
| `findit` | bdos/fsdir.c:1407 | `ta/bdos/fs/path.rs::findit` +1 | m | m | R | - | - |
| `flush` | bdos/fsbuf.c:85 | `ta/bdos/fs/cache.rs::write_back` | m | m | R | - | - |
| `free_all_owned` | bdos/proc.c:92 | `ta/bdos/mem/free_all_owned.rs::free_all_owned` | m | m | R | - | - |
| `free_available_dnds` | bdos/fsdir.c:2053 | UNMAPPED | u | m | R | - | - |
| `freednd` | bdos/fsdir.c:1900 | UNMAPPED | u | m | R | - | - |
| `freeit` | bdos/iumem.c:144 | `ta/bdos/mem/pool.rs::freeit` | m | m | R | - | - |
| `freetree` | bdos/bdosmain.c:356 | `ta/bdos/fs/curdir.rs::free_drive` | m | m | R | - | - |
| `get_default_handle` | bdos/console.c:132 | UNMAPPED | u | m | R | - | - |
| `getbcb` | bdos/fsbuf.c:114 | `ta/bdos/fs/cache.rs::get` +4 | m | m | R | - | - |
| `getch` | bdos/console.c:351 | `ta/gemdos/console.rs::getch` +1 | m | m | R | - | - |
| `getclnum` | bdos/fsfat.c:160 | `ta/bdos/fs/fat.rs::fat_entry` | m | m | R | - | - |
| `getdmd` | bdos/fsdrive.c:140 | `ta/bdos/fs/fat.rs::log_in` | m | m | R | - | - |
| `getdnd` | bdos/fsdir.c:1869 | UNMAPPED | u | m | R | - | - |
| `getofd` | bdos/fsmain.c:474 | `ta/bdos/fs/file.rs::getofd` | m | m | R | - | - |
| `getosm` | bdos/osmem.c:100 | UNMAPPED | u | - | R | - | - |
| `getpath` | bdos/fsdir.c:1761 | `ta/bdos/fs/path.rs::resolve_parent` | m | - | R | - | - |
| `getrealcl` | bdos/fsfat.c:107 | `ta/bdos/fs/fat.rs::fat_entry` | m | m | R | - | - |
| `getrec` | bdos/fsbuf.c:209 | `ta/bdos/fs/cache.rs::with_sector` +2 | m | m | R | - | - |
| `gouser` | bdos/rwa.S:234 | design:hardware:m68k-cpu | d | m | R | - | - |
| `incr_curdir_usage` | bdos/fsdir.c:1164 | `ta/bdos/fs/curdir.rs::incr` | m | m | R | - | - |
| `init_pd_fields` | bdos/proc.c:364 | `ta/bdos/process/loader.rs::populate_basepage` +1 | m | m | R | - | - |
| `init_pd_files` | bdos/proc.c:387 | `ta/bdos/process/loader.rs::init_pd_files` +1 | m | m | R | - | - |
| `is_subdir` | bdos/fsdir.c:853 | `ta/bdos/fs/file.rs::is_self_or_descendant` | m | m | R | - | - |
| `ixclose` | bdos/fsopnclo.c:511 | `ta/bdos/fs/file.rs::ixclose` +2 | m | m | R | - | - |
| `ixcreat` | bdos/fsopnclo.c:99 | `ta/bdos/fs/file.rs::fcreate` +1 | m | m | R | - | - |
| `ixdel` | bdos/fsopnclo.c:652 | `ta/bdos/fs/file.rs::ixdel` | m | m | R | - | - |
| `ixforce` | bdos/fshand.c:46 | `ta/bdos/fs/file.rs::ixforce` | m | m | R | - | - |
| `ixgetfcb` | bdos/fsio.c:462 | `ta/bdos/fs/dir.rs::next` | m | m | R | - | - |
| `ixlseek` | bdos/fsio.c:354 | `ta/bdos/fs/fsio.rs::ixlseek` +1 | m | m | R | - | - |
| `ixopen` | bdos/fsopnclo.c:239 | `ta/bdos/fs/file.rs::fopen` | m | m | R | - | - |
| `ixread` | bdos/fsio.c:498 | `ta/bdos/fs/file.rs::ixread` | m | m | R | - | - |
| `ixsfirst` | bdos/fsdir.c:452 | `ta/bdos/fs/find.rs::fsfirst` | m | m | R | - | - |
| `ixsnext` | bdos/fsdir.c:545 | `ta/bdos/fs/find.rs::scan` | m | m | R | - | - |
| `ixterm` | bdos/proc.c:110 | `ta/bdos/fs/file.rs::close_process_files` +1 | m | m | R | - | - |
| `ixunlink` | bdos/fsopnclo.c:588 | `ta/bdos/fs/file.rs::fdelete` | m | m | R | - | - |
| `ixwrite` | bdos/fsio.c:554 | `ta/bdos/fs/file.rs::ixwrite` | m | m | R | - | - |
| `kpgm_relocate` | bdos/kpgmld.c:279 | `ta/bdos/process/reloc.rs::relocate` | m | - | R | - | - |
| `kpgmhdrld` | bdos/kpgmld.c:40 | `ta/bdos/process/elf.rs::parse` | m | m | R | - | - |
| `kpgmld` | bdos/kpgmld.c:79 | `ta/bdos/process/loader.rs::load_from_source` +1 | m | m | R | - | - |
| `log2ul` | bdos/fsdrive.c:164 | UNMAPPED | u | - | R | - | - |
| `log_media` | bdos/fsdrive.c:184 | `ta/bdos/fs/fat.rs::log_in` | m | m | R | - | - |
| `makbuf` | bdos/fsdir.c:1851 | `ta/bdos/fs/find.rs::makbuf` | m | m | R | - | - |
| `makdnd` | bdos/fsdir.c:1610 | UNMAPPED | u | m | R | - | - |
| `makofd` | bdos/fsdir.c:1920 | UNMAPPED | u | m | R | - | - |
| `makopn` | bdos/fsopnclo.c:271 | `ta/bdos/fs/file.rs::open_handle` | m | m | R | - | - |
| `mark_bcbs_invalid` | bdos/bdosmain.c:411 | `ta/bdos/fs/cache.rs::invalidate_drive` | m | m | R | - | - |
| `match` | bdos/fsdir.c:1802 | `ta/bdos/fs/find.rs::pattern_match` | m | m | R | - | - |
| `namlen` | bdos/fsdir.c:178 | UNMAPPED | u | m | R | - | - |
| `newline` | bdos/console.c:480 | `ta/gemdos/console.rs::newline` | m | m | R | - | - |
| `nextcl` | bdos/fsfat.c:247 | `ta/bdos/fs/fsio.rs::nextcl` | m | m | R | - | - |
| `offree` | bdos/bdosmain.c:387 | `ta/bdos/fs/file.rs::free_drive_files` | m | m | R | - | - |
| `opnfil` | bdos/fsopnclo.c:335 | `ta/bdos/fs/file.rs::claim_sft` | m | m | R | - | - |
| `osif` | bdos/bdosmain.c:431 | `ta/gemdos/mod.rs::dispatch` +8 | m | m | R | - | - |
| `osinit_after_xmaddalt` | bdos/bdosmain.c:336 | `ta/bdos/process/standin.rs::init` | m | m | - | - | - |
| `osinit_before_xmaddalt` | bdos/bdosmain.c:317 | `ta/bdos/mem/init.rs::init` | m | m | - | - | - |
| `osmem_init` | bdos/osmem.c:390 | UNMAPPED | u | - | - | - | - |
| `packit` | bdos/fsdir.c:1297 | `ta/bdos/fs/find.rs::name_83` | m | m | R | - | - |
| `pgfix01` | bdos/kpgmld.c:243 | `ta/bdos/process/reloc.rs::relocate` | m | m | R | - | - |
| `pgmld01` | bdos/kpgmld.c:107 | `ta/bdos/process/loader.rs::populate_and_map` | m | m | R | - | - |
| `proc_go` | bdos/proc.c:536 | `ta/bdos/process/pexec.rs::go` +3 | m | m | R | - | - |
| `process_dnd_tree` | bdos/fsdir.c:1957 | UNMAPPED | u | m | R | - | - |
| `prt_line` | bdos/console.c:468 | `ta/gemdos/console.rs::cconws` | m | m | R | - | - |
| `reserve_blocks` | bdos/proc.c:77 | `ta/bdos/mem/free_all_owned.rs::reserve_all_owned` | m | m | R | - | - |
| `scan` | bdos/fsdir.c:1525 | `ta/bdos/fs/find.rs::scan` +2 | m | m | R | - | - |
| `set_owner` | bdos/umem.c:587 | `ta/bdos/mem/traps.rs::set_owner` | m | m | R | - | - |
| `sftdel` | bdos/fsopnclo.c:380 | `ta/bdos/fs/file.rs::sftdel` | m | m | R | - | - |
| `sftofdsrch` | bdos/fsopnclo.c:362 | `ta/bdos/fs/file.rs::sftdel` | m | m | R | - | - |
| `shrinkit` | bdos/iumem.c:228 | `ta/bdos/mem/pool.rs::shrinkit` | m | m | R | - | - |
| `snipdnd` | bdos/fsdir.c:1886 | UNMAPPED | u | - | R | - | - |
| `stdhdl_init` | bdos/console.c:107 | `ta/gemdos/console.rs::stdhdl_init_console` | m | m | - | - | - |
| `syshnd` | bdos/fsmain.c:457 | `ta/bdos/fs/file.rs::getofd` | m | i | R | - | - |
| `tabout` | bdos/console.c:292 | `ta/gemdos/console.rs::tabout` | m | m | R | - | - |
| `termuser` | bdos/rwa.S:265 | design:hardware:m68k-cpu | d | m | R | - | - |
| `tikfrk` | bdos/time.c:175 | `ta/gemdos/time.rs::now_secs` | m | m | - | - | - |
| `time_init` | bdos/time.c:160 | `ta/gemdos/time.rs::time_init` | m | m | - | - | - |
| `total_alt_ram` | bdos/umem.c:520 | `ta/bdos/mem/stats.rs::pool_stats` +1 | m | - | - | - | - |
| `umem_init` | bdos/umem.c:549 | `ta/bdos/mem/init.rs::init` | m | m | - | - | - |
| `unlink_mdblock` | bdos/osmem.c:123 | `ta/bdos/mem/mdpool.rs::unlink_mdblock` | m | - | R | - | - |
| `unpackit` | bdos/fsdir.c:1334 | `ta/bdos/fs/dir.rs::fcb_name` | m | m | R | - | - |
| `update_fcb` | bdos/fsdir.c:871 | `ta/bdos/fs/file.rs::flush_ofd` | m | m | R | - | - |
| `usrio` | bdos/fsio.c:57 | `ta/bdos/fs/fsio.rs::usrio` +3 | m | m | R | - | - |
| `xmfreblk` | bdos/osmem.c:360 | UNMAPPED | u | m | R | - | - |
| `xmfremd` | bdos/osmem.c:229 | `ta/bdos/mem/mdpool.rs::xmfremd_global` | m | m | R | - | - |
| `xmgetblk` | bdos/osmem.c:291 | UNMAPPED | u | m | R | - | - |
| `xmgetmd` | bdos/osmem.c:169 | `ta/bdos/mem/mdpool.rs::xmgetmd_global` | m | m | R | - | - |
| `xrw` | bdos/fsio.c:208 | `ta/bdos/fs/fsio.rs::xrw` | m | m | R | - | - |
| `xrw_recs` | bdos/fsio.c:84 | `ta/bdos/fs/fsio.rs::xrw_recs` | m | i | R | - | - |

### 2.5 AES API entries (trap #2, `aes/gemsuper.c` `crysbind`; the slot is the opcode) - 78 rows

| slot | API | fn | def | NewTOS | c | spec | oracle | dev | tickets |
|---|---|---|---|---|---|---|---|---|---|
| 0xa | appl_init | `ap_init` | aes/gemaplib.c:54 | `ta/aes/appl.rs::appl_init` | m | H `docs/api/aes/application.md:389` | R14 1: aes-r4-subfunctions | - | A06/P A07/P A08/P +3 |
| 0xb,0xc | appl_read/appl_write | `ap_rdwr` | aes/gemaplib.c:69 | `ta/aes/appl.rs::appl_read` +1 | m | H `docs/api/aes/application.md:476` | none | - | A113/I A116/I A75/I |
| 0xb,0xc | appl_read/appl_write | `fpdnm` | aes/gempd.c:38 | `ta/aes/pd.rs::find_by_name` | m | H `docs/api/aes/application.md:476` | none | - | A113/I A116/I A75/I |
| 0xd | appl_find | `ap_find` | aes/gemaplib.c:95 | `ta/aes/appl.rs::appl_find` | m | H `docs/api/aes/application.md:747` | O 2: aes-m05-addrin*2 | - | - |
| 0xe | appl_tplay | `ap_tplay` | aes/gemaplib.c:119 | UNMAPPED | u | H `docs/api/aes/application.md:871` | R14 1: aes-r4-subfunctions | - | A45/P |
| 0xf | appl_trecord | `ap_trecd` | aes/gemaplib.c:189 | UNMAPPED | u | H `docs/api/aes/application.md:871` | none | - | A45/P |
| 0x11 | appl_yield | `dsptch` | aes/gemasm.S:129 | `ta/aes/mod.rs::dsptch` +1 | m | H `docs/api/aes/application.md:772` | none | - | - |
| 0x13 | appl_exit | `ap_exit` | aes/gemaplib.c:251 | `ta/aes/appl.rs::appl_exit` | m | H `docs/api/aes/application.md:792` | none | - | SCHED01/P ACC10/D |
| 0x14 | evnt_keybd | `ev_block` | aes/gemevlib.c:64 | `ta/bdos/process/sched.rs::sleep` | m | H `docs/api/aes/event.md:739` | none | - | - |
| 0x15 | evnt_button | `ev_button` | aes/gemevlib.c:84 | `ta/aes/event.rs::evnt_button` | m | H `docs/api/aes/event.md:759` | none | - | - |
| 0x16 | evnt_mouse | `ev_mouse` | aes/gemevlib.c:123 | `ta/aes/event.rs::evnt_mouse` | m | H `docs/api/aes/event.md:788` | none | - | A90/I |
| 0x17 | evnt_mesag | `ev_mesag` | aes/gemevlib.c:100 | `ta/aes/event.rs::evnt_mesag` | m | H `docs/api/aes/event.md:823` | none | - | - |
| 0x18 | evnt_timer | `ev_timer` | aes/gemevlib.c:134 | `ta/aes/event.rs::wait_ms` | m | H `docs/api/aes/event.md:855` | O 2: aes-r1*2 | - | - |
| 0x19 | evnt_multi | `combine_cms` | aes/gemevlib.h:34 | `ta/aes/input.rs::abutton` | m | H `docs/api/aes/event.md:96` | O 12: aes-m07-parameter*12 | - | A92/I ACC03/P A112/D |
| 0x19 | evnt_multi | `ev_multi` | aes/gemevlib.c:152 | `ta/aes/input.rs::mn_button_arm` +1 | m | H `docs/api/aes/event.md:96` | O 12: aes-m07-parameter*12 | - | A92/I ACC03/P A112/D |
| 0x1a | evnt_dclick | `ev_dclick` | aes/gemevlib.c:300 | `ta/aes/event.rs::ev_dclick` | m | H `docs/api/aes/event.md:877` | R14 4: aes-r4-subfunctions | - | - |
| 0x1e | menu_bar | `mn_bar` | aes/gemmnlib.c:703 | `ta/aes/menu.rs::menu_bar` +1 | m | H `docs/api/aes/menu.md:199` | R14 1: aes-r4-subfunctions | - | - |
| 0x1f,0x20,0x21 | menu_icheck,menu_ienable,menu_tnormal | `do_chg` | aes/gemmnlib.c:145 | `ta/aes/menu.rs::do_chg` | m | H `docs/api/aes/menu.md:119` | none | - | - |
| 0x22 | menu_text | `inline_strcpy` | include/string.h:38 | UNMAPPED | u | H `docs/api/aes/menu.md:348` | none | - | - |
| 0x23 | menu_register | `mn_register` | aes/gemmnlib.c:789 | `ta/aes/menu.rs::menu_register` | m | H `docs/api/aes/menu.md:372` | R14 2: aes-r4-subfunctions | - | - |
| 0x24 | menu_popup | `mn_popup` | aes/gemmnext.c:304 | `ta/aes/menu.rs::mn_popup` | m | H `docs/api/aes/menu.md:407` | none | - | A88/D |
| 0x25 | menu_attach | `mn_attach` | aes/gemmnext.c:535 | `ta/aes/menu.rs::menu_attach` | m | H `docs/api/aes/menu.md:498` | none | - | - |
| 0x26 | menu_istart | `mn_istart` | aes/gemmnext.c:562 | `ta/aes/menu.rs::menu_istart` | m | H `docs/api/aes/menu.md:566` | none | - | - |
| 0x27 | menu_settings | `mn_settings` | aes/gemmnext.c:596 | `ta/aes/menu.rs::menu_settings` | m | H `docs/api/aes/menu.md:586` | none | - | A43/D |
| 0x28 | objc_add | `ob_add` | aes/gemoblib.c:1121 | `ta/aes/object.rs::ob_add` | m | H `docs/api/aes/object-form.md:132` | R14 3: aes-r4-subfunctions | - | A55/D |
| 0x29 | objc_delete | `ob_delete` | aes/gemoblib.c:1147 | `ta/aes/object.rs::ob_delete` | m | H `docs/api/aes/object-form.md:159` | R14 2: aes-r4-subfunctions | - | - |
| 0x2a,0x2e,0x2f,0x37,0x38 | form_button,form_keybd,objc_change,objc_draw,objc_edit | `gsx_sclip` | aes/gemgraf.c:91 | `ta/aes/object.rs::clip_from_intin` | m | H `docs/api/aes/object-form.md:175` | R14 8: aes-r4-subfunctions | - | A138/P A99/I A46/D +1 |
| 0x2a | objc_draw | `ob_draw` | aes/gemoblib.c:959 | `ta/aes/object.rs::render_tree_t` +1 | m | H `docs/api/aes/object-form.md:175` | none | - | A138/P A46/D A65/D |
| 0x2b | objc_find | `ob_find` | aes/gemoblib.c:1034 | `ta/aes/object.rs::find_hit` +4 | m | H `docs/api/aes/object-form.md:587` | none | - | - |
| 0x2c | objc_offset | `ob_offset` | aes/gemoblib.c:285 | `ta/aes/object.rs::abs_offset` | m | H `docs/api/aes/object-form.md:623` | none | - | - |
| 0x2d | objc_order | `ob_order` | aes/gemoblib.c:1201 | `ta/aes/object.rs::ob_order` | m | H `docs/api/aes/object-form.md:639` | R14 3: aes-r4-subfunctions | - | A47/D |
| 0x2e | objc_edit | `ob_edit` | aes/gemobed.c:335 | `ta/aes/object.rs::edit_field_t` +2 | m | H `docs/api/aes/object-form.md:686` | R14 8: aes-r4-subfunctions | - | - |
| 0x2f | objc_change | `ob_change` | aes/gemoblib.c:1253 | `ta/aes/object.rs::change_clip_t` +4 | m | H `docs/api/aes/object-form.md:763` | none | - | A99/I A65/D |
| 0x30 | objc_sysvar | `ob_sysvar` | aes/gemoblib.c:1350 | `ta/aes/object.rs::objc_sysvar` | m | H `docs/api/aes/object-form.md:813` | O 19: aes-m06-sysvar*19 | - | - |
| 0x32 | form_do | `fm_do` | aes/gemfmlib.c:278 | `ta/aes/form.rs::run_form` +1 | m | H `docs/api/aes/object-form.md:867` | none | - | - |
| 0x33 | form_dial | `fm_dial` | aes/gemfmlib.c:360 | `ta/aes/form.rs::fm_dial` +2 | m | H `docs/api/aes/object-form.md:910` | R14 5: aes-r4-subfunctions | - | DESK08/I |
| 0x34 | form_alert | `fm_alert` | aes/gemfmalt.c:319 | `ta/aes/form.rs::form_alert` +3 | m | H `docs/api/aes/object-form.md:947` | none | - | - |
| 0x35 | form_error | `fm_error` | aes/gemfmlib.c:436 | `ta/aes/form.rs::fm_error` +1 | m | H `docs/api/aes/object-form.md:1043` | O 8: aes-m07-parameter*8 | - | - |
| 0x36 | form_center | `ob_center` | aes/gemobed.c:67 | `ta/aes/form.rs::ob_center` | m | H `docs/api/aes/object-form.md:1066` | none | - | DESK08/I |
| 0x37 | form_keybd | `fm_keybd` | aes/gemfmlib.c:163 | `ta/aes/form.rs::fm_keybd` +1 | m | H `docs/api/aes/object-form.md:1083` | none | - | - |
| 0x38 | form_button | `fm_button` | aes/gemfmlib.c:201 | `ta/aes/form.rs::fm_button` +3 | m | H `docs/api/aes/object-form.md:1104` | none | - | - |
| 0x46 | graf_rubbox | `gr_rubbox` | aes/gemgrlib.c:228 | `ta/aes/graf.rs::graf_rubberbox` | m | H `docs/api/aes/window-graphics.md:1516` | none | - | - |
| 0x47 | graf_dragbox | `gr_dragbox` | aes/gemgrlib.c:243 | `ta/aes/window.rs::gr_dragbox` +2 | m | H `docs/api/aes/window-graphics.md:1535` | none | - | - |
| 0x48 | graf_mbox | `gr_movebox` | aes/gemgrlib.c:287 | `ta/aes/graf.rs::graf_movebox` | m | H `docs/api/aes/window-graphics.md:1542` | R14 1: aes-r4-subfunctions | - | - |
| 0x49 | graf_growbox | `gr_growbox` | aes/gemgrlib.c:309 | `ta/aes/graf.rs::graf_growbox` | m | H `docs/api/aes/window-graphics.md:1548` | R14 1: aes-r4-subfunctions | - | - |
| 0x4a | graf_shrinkbox | `gr_shrinkbox` | aes/gemgrlib.c:326 | `ta/aes/graf.rs::graf_shrinkbox` | m | H `docs/api/aes/window-graphics.md:1548` | R14 1: aes-r4-subfunctions | - | DESK51/P |
| 0x4b | graf_watchbox | `gr_watchbox` | aes/gemgrlib.c:337 | `ta/aes/graf.rs::graf_watchbox` +2 | m | H `docs/api/aes/window-graphics.md:1555` | none | - | - |
| 0x4c | graf_slidebox | `gr_slidebox` | aes/gemgrlib.c:358 | `ta/aes/graf.rs::gr_slidebox` +4 | m | H `docs/api/aes/window-graphics.md:1586` | none | - | - |
| 0x4e | graf_mouse | `ct_mouse` | aes/gemctrl.c:400 | `ta/aes/input.rs::ct_mouse` | m | H `docs/api/aes/window-graphics.md:1681` | R14 17: aes-r4-subfunctions | D35 D64 | A147/P A44/D |
| 0x4e | graf_mouse | `gr_mouse` | aes/gemgrlib.c:419 | `ta/aes/graf.rs::graf_mouse` +1 | m | H `docs/api/aes/window-graphics.md:1681` | R14 17: aes-r4-subfunctions | D35 D64 | A147/P A44/D |
| 0x4f | graf_mkstate | `gr_mkstate` | aes/gemgrlib.c:458 | `ta/aes/graf.rs::graf_mkstate` | m | H `docs/api/aes/window-graphics.md:1717` | O 1: aes-r1*1 | - | - |
| 0x50 | scrp_read | `sc_read` | aes/gemsclib.c:40 | `ta/aes/shell.rs::scrp_read` | m | H `docs/api/aes/scrap-shell.md:80` | none | - | - |
| 0x51 | scrp_write | `sc_write` | aes/gemsclib.c:57 | `ta/aes/shell.rs::scrp_write` | m | H `docs/api/aes/scrap-shell.md:98` | none | - | - |
| 0x52 | scrp_clear | `sc_clear` | aes/gemsclib.c:74 | `ta/aes/shell.rs::scrp_clear` | m | H `docs/api/aes/scrap-shell.md:115` | none | - | - |
| 0x5a,0x5b | fsel_exinput,fsel_input | `fs_input` | aes/gemfslib.c:609 | `ta/aes/fsel.rs::modal` +5 | m | H `docs/api/aes/resource-fileselector.md:647` | none | - | - |
| 0x64 | wind_create | `wm_create` | aes/gemwmlib.c:1369 | `ta/aes/window.rs::wind_create` | m | H `docs/api/aes/window-graphics.md:109` | O 11: aes-r2-wind*11 | - | A10/P A34/P |
| 0x65 | wind_open | `wm_open` | aes/gemwmlib.c:1410 | `ta/aes/window.rs::wind_open` | m | H `docs/api/aes/window-graphics.md:306` | O 5: aes-r2-wind*5 | - | A12/P A94/I A104/D +1 |
| 0x66 | wind_close | `wm_close` | aes/gemwmlib.c:1438 | `ta/aes/window.rs::close_window` +1 | m | H `docs/api/aes/window-graphics.md:409` | O 4: aes-r2-wind*4 | - | A16/P |
| 0x67 | wind_delete | `wm_delete` | aes/gemwmlib.c:1466 | `ta/aes/window.rs::wm_delete` +1 | m | H `docs/api/aes/window-graphics.md:423` | O 5: aes-r2-wind*5 | - | A33/P |
| 0x68 | wind_get | `wm_get` | aes/gemwmlib.c:1505 | `ta/aes/window.rs::wind_get` +1 | m | H `docs/api/aes/window-graphics.md:159` | O 108: aes-m05-addrin*2,aes-r1*22,aes-r2-wind*31 +2 tbl | C158 | A05/P A11/P A12/P +9 |
| 0x69 | wind_set | `wm_set` | aes/gemwmlib.c:1623 | `ta/aes/window.rs::wm_set` +1 | m | H `docs/api/aes/window-graphics.md:159` | O 8: aes-r2-wind*8 | C361 | A119/P A78/I A79/P +6 |
| 0x6a | wind_find | `wm_find` | aes/gemwmlib.c:1767 | `ta/aes/window.rs::wm_find` +2 | m | H `docs/api/aes/window-graphics.md:1053` | O 10: aes-r1*5,aes-r2-wind*5 | - | - |
| 0x6b | wind_update | `wm_update` | aes/gemwmlib.c:1778 | `ta/aes/window.rs::wind_update` | m | H `docs/api/aes/window-graphics.md:1065` | O 7: aes-r2-wind*7 | - | A118/I A110/D |
| 0x6c | wind_calc | `wm_calc` | aes/gemwmlib.c:1807 | `ta/aes/window.rs::calc` +2 | m | H `docs/api/aes/window-graphics.md:1338` | O 61: aes-r1*60,aes-r3-wind-state*1 | - | - |
| 0x6d | wind_new | `wm_new` | aes/gemwmlib.c:1875 | `ta/aes/window.rs::wm_new` +1 | m | H `docs/api/aes/window-graphics.md:1434` | O 2: aes-r2-wind*2 | - | A96/I |
| 0x6e | rsrc_load | `rs_load` | aes/gemrslib.c:826 | `ta/aes/resource.rs::rsrc_load` | m | H `docs/api/aes/resource-fileselector.md:96` | none | - | A35/I |
| 0x6f | rsrc_free | `rs_free` | aes/gemrslib.c:685 | `ta/aes/resource.rs::rsrc_free` | m | H `docs/api/aes/resource-fileselector.md:478` | R14 2: aes-r4-subfunctions | - | - |
| 0x70 | rsrc_gaddr | `rs_gaddr` | aes/gemrslib.c:715 | `ta/aes/resource.rs::rsrc_gaddr` | m | H `docs/api/aes/resource-fileselector.md:503` | R14 28: aes-r4-subfunctions | C287 | A50/D |
| 0x71 | rsrc_saddr | `rs_saddr` | aes/gemrslib.c:728 | `ta/aes/resource.rs::rsrc_saddr` | m | H `docs/api/aes/resource-fileselector.md:503` | R14 2: aes-r4-subfunctions | C287 | A50/D |
| 0x72 | rsrc_obfix | `rs_obfix` | aes/gemrslib.c:89 | `ta/aes/resource.rs::rs_obfix` | m | H `docs/api/aes/resource-fileselector.md:581` | none | - | - |
| 0x78 | shel_read | `sh_read` | aes/gemshlib.c:102 | `ta/aes/shell.rs::shel_read` | m | H `docs/api/aes/scrap-shell.md:145` | none | - | - |
| 0x79 | shel_write | `sh_write` | aes/gemshlib.c:157 | `ta/aes/shell.rs::shel_write` | m | H `docs/api/aes/scrap-shell.md:173` | none | - | A48/P A61/D |
| 0x7a | shel_get | `sh_get` | aes/gemshlib.c:203 | `ta/aes/shell.rs::shel_get` | m | H `docs/api/aes/shell-loop.md:922` | R14 2: aes-r4-subfunctions | - | - |
| 0x7b | shel_put | `sh_put` | aes/gemshlib.c:213 | `ta/aes/shell.rs::shel_put` | m | H `docs/api/aes/shell-loop.md:922` | R14 1: aes-r4-subfunctions | - | - |
| 0x7c | shel_find | `sh_find` | aes/gemshlib.c:474 | `ta/aes/shell.rs::sh_find` | m | H `docs/api/aes/scrap-shell.md:301` | none | - | - |
| 0x7d | shel_envrn | `sh_envrn` | aes/gemshlib.c:327 | `ta/aes/shell.rs::shel_envrn` | m | H `docs/api/aes/scrap-shell.md:328` | none | - | - |
| 0x7e | shel_rdef | `sh_rdef` | aes/gemshlib.c:491 | `ta/aes/shell.rs::shel_rdef` | m | H `docs/api/aes/scrap-shell.md:368` | none | - | - |
| 0x7f | shel_wdef | `sh_wdef` | aes/gemshlib.c:504 | `ta/aes/shell.rs::sh_wdef` | m | H `docs/api/aes/scrap-shell.md:368` | none | - | - |

### 2.6 aes/ internals - 341 rows

| fn | def | NewTOS | c | spec | R | dev | tickets |
|---|---|---|---|---|---|---|---|
| `abutton` | aes/geminput.c:545 | `ta/aes/input.rs::abutton` | m | m | R | - | - |
| `acancel` | aes/gemasync.c:217 | `ta/aes/input.rs::mn_button_cancel` +1 | m | m | R | - | - |
| `accdesk_start` | aes/gemstart.S:139 | `ta/bdos/process/standin.rs::enter_accdesk` | m | m | - | - | - |
| `add_3d_effect` | aes/gemoblib.c:220 | `ta/aes/object.rs::add_3d_effect` | m | m | R | - | - |
| `adelay` | aes/geminput.c:496 | `ta/bdos/process/sched.rs::arm_or_get_deadline` | m | m | R | - | - |
| `aes_run_rom_program` | aes/gemshlib.c:570 | `k/shell_loop.rs::sh_ldapp` | m | m | - | - | - |
| `aes_wheel` | aes/gemdosif.S:353 | `ta/aes/mod.rs::aes_wheel` | m | m | - | - | - |
| `aestrap` | aes/gemdosif.S:222 | design:hardware:m68k-cpu | d | m | - | - | - |
| `aestrap_intercepted` | aes/gemgsxif.c:259 | UNMAPPED | u | - | - | - | - |
| `akbin` | aes/geminput.c:486 | `ta/aes/event.rs::evnt_keybd` | m | m | R | - | - |
| `all_run` | aes/geminit.c:554 | `ta/aes/acc.rs::all_run` | m | m | - | - | - |
| `amouse` | aes/geminput.c:568 | `ta/aes/event.rs::evnt_mouse` | m | m | R | - | - |
| `amutex` | aes/gemflag.c:89 | `ta/aes/window.rs::update_block` | m | m | R | - | - |
| `ap_sendmsg` | aes/gemwmlib.c:851 | `ta/aes/window.rs::send_redraw` | m | m | R | - | - |
| `apret` | aes/gemasync.c:182 | `ta/aes/input.rs::bwait_take` | m | m | R | - | - |
| `aqueue` | aes/gemqueue.c:85 | `ta/aes/pd.rs::post_msg` +2 | m | m | R | - | - |
| `attach_submenu` | aes/gemmnext.c:465 | `ta/aes/menu.rs::attach_submenu` +1 | m | m | - | - | - |
| `azombie` | aes/gemasync.c:64 | `ta/bdos/process/sched.rs::wake` | m | m | R | - | - |
| `b_click` | aes/geminput.c:114 | `ta/aes/input.rs::b_click` | m | m | - | - | - |
| `b_delay` | aes/geminput.c:155 | `ta/aes/input.rs::b_delay` | m | m | - | - | - |
| `bb_fill` | aes/gemgraf.c:434 | `ta/aes/object.rs::gr_rect` +1 | m | m | R | - | - |
| `bb_restore` | aes/gemgsxif.c:390 | `ta/aes/gsxif.rs::restore` | m | m | R | - | - |
| `bb_save` | aes/gemgsxif.c:383 | `ta/aes/gsxif.rs::bb_save` | m | m | R | - | A66/I |
| `bb_screen` | aes/gemgraf.c:357 | `ta/aes/window.rs::bb_screen` | m | m | R | - | - |
| `bb_set` | aes/gemgsxif.c:320 | `ta/aes/gsxif.rs::bb_save` +1 | m | m | R | - | - |
| `bchange` | aes/geminput.c:328 | `ta/aes/input.rs::bchange` | m | m | - | - | - |
| `best_match` | aes/gemrslib.c:267 | `ta/aes/resource.rs::best_match` | m | m | - | - | - |
| `brkrct` | aes/gemwrect.c:101 | `ta/aes/window.rs::subtract` | m | m | - | - | - |
| `build_menuid_lookup` | aes/gemmnlib.c:766 | `ta/aes/menu.rs::build_menuid_lookup` | m | m | - | - | - |
| `call_usercode` | aes/gemoblib.c:411 | `ta/aes/object.rs::ob_user` | m | m | R | - | - |
| `centre_title` | aes/gemfslib.c:179 | `ta/aes/fsel.rs::centre_title` | m | - | - | - | - |
| `ch_height` | aes/gemgraf.c:468 | `ta/aes/object.rs::cell_h` | m | m | R | - | - |
| `ch_width` | aes/gemgraf.c:456 | `ta/aes/object.rs::cell_w` | m | m | R | - | - |
| `check` | aes/gemobed.c:252 | `ta/aes/object.rs::check` | m | m | R | - | - |
| `chk_ctrl` | aes/geminput.c:87 | `ta/aes/window.rs::chk_ctrl` | m | m | - | - | - |
| `chkkbd` | aes/gemdisp.c:158 | `ta/aes/mod.rs::chkkbd` | m | m | R | - | - |
| `chkown` | aes/geminput.c:316 | `ta/aes/input.rs::bchange` | m | m | - | - | - |
| `clamp_height` | aes/gemmnext.c:83 | `ta/aes/menu.rs::clamp_height` | m | m | - | - | - |
| `clamp_ypos` | aes/gemmnext.c:115 | `ta/aes/menu.rs::clamp_ypos` | m | m | - | - | A88/D |
| `count_accs` | aes/geminit.c:264 | `k/accs.rs::count_accs` | m | m | - | - | ACC02/P |
| `crysbind` | aes/gemsuper.c:77 | `ta/aes/mod.rs::crysbind` | m | m | - | - | - |
| `ct_chgown` | aes/gemctrl.c:392 | `ta/aes/input.rs::ct_chgown` | m | m | R | - | - |
| `ct_msgup` | aes/gemctrl.c:81 | `ta/aes/input.rs::ct_msgup_hold` | m | m | - | - | - |
| `ctlmgr` | aes/gemctrl.c:427 | `ta/aes/window.rs::intercept` | m | m | - | - | - |
| `curfld` | aes/gemobed.c:194 | `ta/aes/object.rs::curfld` | m | m | R | - | - |
| `default_mform` | aes/geminit.c:174 | `ta/vdi/mouse.rs::set_preset` | m | m | R | - | - |
| `detach_submenu` | aes/gemmnext.c:438 | `ta/aes/menu.rs::submenu_unmark` | m | m | - | - | - |
| `disable_interrupts` | aes/gemdosif.S:61 | design:hardware:m68k-cpu | d | - | R | - | - |
| `disp` | aes/gemdisp.c:221 | `ta/bdos/process/sched.rs::dsptch` +1 | m | m | R | - | - |
| `disp_act` | aes/gemdisp.c:74 | `ta/bdos/process/sched.rs::ready` | m | m | R | - | - |
| `display_submenu` | aes/gemmnext.c:625 | `ta/aes/menu.rs::open_submenu` | m | m | - | - | - |
| `dither_mask` | aes/gemgraf.c:680 | `ta/aes/object.rs::draw_icon` | m | m | R | - | - |
| `do_walk` | aes/gemwmlib.c:286 | `ta/aes/window.rs::walk_rects` +1 | m | m | R | - | - |
| `doq` | aes/gemqueue.c:31 | `ta/aes/pd.rs::enqueue` +1 | m | m | R | - | - |
| `dos_exec` | aes/gemstart.S:169 | `k/launch.rs::run_program_with_tail_rc` | m | m | - | - | - |
| `downorup` | aes/geminput.c:348 | `ta/aes/input.rs::downorup` | m | m | R | - | - |
| `dq` | aes/geminput.c:256 | UNMAPPED | u | - | R | - | - |
| `draw_3d_outline` | aes/gemoblib.c:153 | `ta/aes/object.rs::draw_3d_outline` | m | m | R | - | - |
| `draw_change` | aes/gemwmlib.c:1058 | `ta/aes/window.rs::draw_change_open` +3 | m | m | R | - | A104/D |
| `drawrat` | aes/gemdosif.S:383 | UNMAPPED | u | - | - | - | - |
| `enable_interrupts` | aes/gemdosif.S:76 | design:hardware:m68k-cpu | d | m | R | - | - |
| `eralert` | aes/gemfmlib.c:416 | UNMAPPED | u | m | - | - | - |
| `ev_init` | aes/geminit.c:198 | UNMAPPED | u | - | - | - | - |
| `ev_rets` | aes/gemevlib.c:42 | `ta/aes/event.rs::fill_rets` +1 | m | m | R | - | - |
| `everyobj` | aes/gemobjop.c:76 | `ta/aes/object.rs::draw_tree` | m | m | R | - | - |
| `evinsert` | aes/gemasync.c:81 | UNMAPPED | u | m | R | - | - |
| `evremove` | aes/geminput.c:279 | `ta/bdos/process/sched.rs::wake` | m | m | R | - | - |
| `expand_cicondata` | aes/gemrslib.c:296 | `ta/aes/resource.rs::cicon_pixel` | m | m | - | - | - |
| `FAKE_EVB` | aes/aesvars.h:27 | UNMAPPED | u | - | R | - | - |
| `far_bcha` | aes/gemdosif.S:306 | `ta/aes/mod.rs::user_but` | m | m | - | - | - |
| `far_mcha` | aes/gemdosif.S:327 | `ta/aes/mod.rs::user_mot` | m | m | - | - | - |
| `find_obj` | aes/gemfmlib.c:114 | `ta/aes/form.rs::find_obj` | m | m | R | - | - |
| `find_pos` | aes/gemobed.c:158 | `ta/aes/object.rs::find_pos` | m | m | R | - | - |
| `find_submenu_from_obtype` | aes/gemmnext.c:361 | `ta/aes/menu.rs::smib_find_parent` | m | m | - | - | - |
| `find_submenu_from_tree` | aes/gemmnext.c:380 | `ta/aes/menu.rs::smib_find_sub` | m | - | - | - | - |
| `findfile` | aes/gemshlib.c:406 | `ta/aes/shell.rs::sh_find` | m | m | - | - | - |
| `fix_chpos` | aes/gemrslib.c:53 | `ta/aes/resource.rs::fix_chpos` | m | m | R | - | - |
| `fix_cicons` | aes/gemrslib.c:549 | `ta/aes/resource.rs::fix_cicons` | m | m | - | - | - |
| `fix_long` | aes/gemrslib.c:576 | `ta/aes/resource.rs::relocate_blob_core` | m | m | - | - | - |
| `fix_nptrs` | aes/gemrslib.c:640 | `ta/aes/resource.rs::fix_nptrs64` | m | m | - | - | - |
| `fix_objects` | aes/gemrslib.c:606 | `ta/aes/resource.rs::fix_objects_coords` +1 | m | m | - | - | - |
| `fix_ptr` | aes/gemrslib.c:649 | `ta/aes/resource.rs::fix_nptrs64` | m | - | - | - | - |
| `fix_tedinfo_std` | aes/gemrslib.c:655 | `ta/aes/resource.rs::fix_tedinfo_std` | m | m | - | - | A37/I |
| `fix_trindex` | aes/gemrslib.c:591 | `ta/aes/resource.rs::relocate_blob_core` | m | m | - | - | - |
| `fixup_all_ciconblks` | aes/gemrslib.c:226 | `ta/aes/resource.rs::relocate_blob_core` | m | m | - | - | - |
| `fixup_colour_icons` | aes/gemrslib.c:185 | `ta/aes/resource.rs::relocate_blob_core` | m | m | - | - | - |
| `fm_build` | aes/gemfmalt.c:188 | `ta/aes/form.rs::do_alert` +1 | m | m | R | - | - |
| `fm_inifld` | aes/gemfmlib.c:153 | `ta/aes/form.rs::fm_inifld` | m | m | R | - | - |
| `fm_own` | aes/gemfmlib.c:82 | `ta/aes/window.rs::fm_own` +2 | m | m | R | - | - |
| `fm_parse` | aes/gemfmalt.c:157 | `ta/aes/form.rs::do_alert` | m | m | R | - | - |
| `fm_show` | aes/gemfmlib.c:389 | `ta/aes/form.rs::fm_error` | m | m | R | - | - |
| `fm_strbrk` | aes/gemfmalt.c:78 | `ta/aes/form.rs::split_copy` +1 | m | m | R | - | - |
| `forker` | aes/gemdisp.c:101 | `ta/aes/mod.rs::dsptch` | m | m | R | - | - |
| `forkq` | aes/gemdisp.c:52 | UNMAPPED | u | m | R | - | - |
| `fq` | aes/geminput.c:272 | UNMAPPED | u | - | R | - | - |
| `free_accs` | aes/geminit.c:296 | `k/accs.rs::free_accs` | m | m | - | - | - |
| `free_cicon_buffers` | aes/gemrslib.c:515 | `ta/aes/resource.rs::rsrc_free` | m | m | - | - | - |
| `fs_1scroll` | aes/gemfslib.c:345 | `ta/aes/fsel.rs::nscroll_top` | m | m | - | - | - |
| `fs_active` | aes/gemfslib.c:268 | `ta/aes/fsel.rs::relist` +1 | m | m | - | - | - |
| `fs_add` | aes/gemfslib.c:249 | `ta/aes/fsel.rs::relist` | m | - | - | - | - |
| `fs_back` | aes/gemfslib.c:199 | `ta/aes/fsel.rs::fs_pspec` | m | m | - | - | - |
| `fs_comp` | aes/gemfslib.c:243 | `ta/aes/fsel.rs::entry_cmp` | m | - | - | - | - |
| `fs_format` | aes/gemfslib.c:363 | `ta/aes/fsel.rs::fs_format` +2 | m | m | - | - | - |
| `fs_newdir` | aes/gemfslib.c:487 | `ta/aes/fsel.rs::relist` | m | m | - | - | - |
| `fs_nscroll` | aes/gemfslib.c:424 | `ta/aes/fsel.rs::scroll` +1 | m | m | - | - | - |
| `fs_pspec` | aes/gemfslib.c:229 | `ta/aes/fsel.rs::fs_pspec` | m | m | - | - | - |
| `fs_sel` | aes/gemfslib.c:413 | `ta/aes/fsel.rs::modal` | m | m | - | - | - |
| `fs_start` | aes/gemfslib.c:59 | `ta/aes/fsel.rs::fs_start` | m | m | - | - | - |
| `gem_main` | aes/geminit.c:753 | `k/lib.rs::kernel_main_rust` +2 | m | m | - | - | - |
| `gem_rsc_fixit` | aes/gem_rsc.c:636 | `ta/aes/init.rs::gem_rsc` | m | m | - | - | - |
| `gem_rsc_init` | aes/gem_rsc.c:622 | `ta/aes/init.rs::gem_rsc` | m | m | - | - | - |
| `get_addr` | aes/gemrslib.c:112 | `ta/aes/resource.rs::get_addr` | m | m | - | - | - |
| `get_ciconblkptr` | aes/gemrslib.c:484 | `ta/aes/resource.rs::fix_cicons` | m | m | - | - | - |
| `get_ctrl` | aes/geminput.c:200 | `ta/aes/window.rs::chk_ctrl` | m | m | R | - | - |
| `get_drive` | aes/gemfslib.c:590 | `ta/aes/fsel.rs::set_drive_states` | m | - | - | - | - |
| `get_mown` | aes/geminput.c:209 | `ta/aes/input.rs::get_mown` | m | m | R | - | - |
| `get_orect` | aes/gemwrect.c:53 | `ta/aes/window.rs::put` | m | m | R | - | - |
| `get_par` | aes/gemobjop.c:145 | `ta/aes/object.rs::parent_of` | m | m | R | - | - |
| `get_prev` | aes/gemoblib.c:987 | `ta/aes/object.rs::get_prev` | m | m | R | - | - |
| `get_pwin` | aes/gemwmlib.c:1391 | `ta/aes/window.rs::win` | m | m | R | - | - |
| `get_sub` | aes/gemrslib.c:102 | `ta/aes/resource.rs::get_addr` | m | - | - | - | - |
| `getpd` | aes/gempd.c:60 | `ta/aes/appl.rs::reported_pid` | m | m | - | - | - |
| `giveerr` | aes/gemdosif.S:116 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | - |
| `gotopgm` | aes/gemasm.S:83 | `ta/bdos/process/sched.rs::switch_to` | m | m | - | - | - |
| `gr_2box` | aes/gemgrlib.c:271 | `ta/aes/graf.rs::animate` | m | m | - | - | - |
| `gr_box` | aes/gemgraf.c:813 | `ta/aes/object.rs::draw_signed_border` +2 | m | m | R | - | - |
| `gr_clamp` | aes/gemgrlib.c:83 | `ta/aes/graf.rs::gr_clamp` | m | m | - | - | - |
| `gr_colourblit` | aes/gemgraf.c:642 | `ta/aes/object.rs::draw_icon` | m | m | R | - | - |
| `gr_crack` | aes/gemgraf.c:612 | `ta/aes/object.rs::gr_crack` | m | m | R | - | - |
| `gr_draw` | aes/gemgrlib.c:155 | `ta/aes/window.rs::gr_draw` | m | - | - | - | - |
| `gr_gblt` | aes/gemgraf.c:631 | `ta/aes/object.rs::blit_plane_kernel` +1 | m | m | R | - | - |
| `gr_gicon` | aes/gemgraf.c:710 | `ta/aes/object.rs::draw_icon` | m | m | R | - | - |
| `gr_gtext` | aes/gemgraf.c:595 | `ta/aes/object.rs::gr_gtext` | m | m | R | - | - |
| `gr_inside` | aes/gemgraf.c:535 | `ta/aes/object.rs::ob_actxywh_t` | m | m | R | - | - |
| `gr_just` | aes/gemgraf.c:567 | `ta/aes/object.rs::gr_gtext` +1 | m | m | R | - | - |
| `gr_rect` | aes/gemgraf.c:547 | `ta/aes/object.rs::gr_rect` | m | m | R | - | - |
| `gr_rubwind` | aes/gemgrlib.c:199 | `ta/aes/window.rs::gr_rubwind` +1 | m | m | - | - | - |
| `gr_scale` | aes/gemgrlib.c:91 | `ta/aes/graf.rs::stepcalc` | m | m | - | - | - |
| `gr_setup` | aes/gemgrlib.c:76 | `ta/aes/graf.rs::gsx_xbox` | m | m | - | - | - |
| `gr_stepcalc` | aes/gemgrlib.c:114 | `ta/aes/graf.rs::stepcalc` | m | m | - | - | - |
| `gr_stilldn` | aes/gemgrlib.c:47 | `ta/aes/window.rs::gr_stilldn` +1 | m | m | R | - | - |
| `gr_wait` | aes/gemgrlib.c:169 | `ta/aes/window.rs::gr_rubwind` +1 | m | m | - | - | - |
| `gr_xor` | aes/gemgrlib.c:127 | `ta/aes/graf.rs::animate` | m | - | - | - | - |
| `gsx2` | aes/gsx2.c:20 | `ta/aes/gsxif.rs::trap` | m | m | R | - | - |
| `gsx_0code` | aes/gemgsxif.c:144 | `ta/aes/gsxif.rs::ncode` | m | m | R | - | - |
| `gsx_1code` | aes/gemgsxif.c:151 | `ta/aes/gsxif.rs::code1` | m | m | R | - | - |
| `gsx_attr` | aes/gemgraf.c:201 | `ta/aes/object.rs::gsx_tblt` | m | m | R | - | - |
| `gsx_blt` | aes/gemgraf.c:328 | `ta/aes/object.rs::blit_plane_kernel` +1 | m | m | R | - | - |
| `gsx_box` | aes/gemgraf.c:264 | `ta/aes/object.rs::draw_border` | m | m | R | - | - |
| `gsx_bxpts` | aes/gemgraf.c:246 | `ta/aes/graf.rs::gsx_bxpts` | m | - | R | - | - |
| `gsx_char` | aes/gemgsxif.c:459 | `ta/aes/mod.rs::chkkbd` | m | m | R | - | - |
| `gsx_chkclip` | aes/gemgraf.c:121 | UNMAPPED | u | - | R | - | - |
| `gsx_cline` | aes/gemgraf.c:188 | `ta/vdi/backend.rs::draw_line` | m | m | R | - | - |
| `gsx_fix` | aes/gemgsxif.c:532 | UNMAPPED | u | - | R | - | - |
| `gsx_fix_screen` | aes/gemgsxif.c:518 | UNMAPPED | u | - | R | - | - |
| `gsx_gclip` | aes/gemgraf.c:111 | UNMAPPED | u | m | R | - | - |
| `gsx_graphic` | aes/gemgsxif.c:298 | `ta/aes/gsxif.rs::gsx_graphic` | m | m | - | - | - |
| `gsx_init` | aes/gemgsxif.c:267 | `ta/aes/gsxif.rs::gsx_init` | m | m | - | - | - |
| `gsx_kstate` | aes/gemgsxif.c:432 | `ta/aes/gsxif.rs::gsx_kstate` +1 | m | m | R | - | - |
| `gsx_malloc` | aes/gemgsxif.c:108 | `ta/aes/gsxif.rs::gsx_malloc` | m | m | - | - | - |
| `gsx_mcalc` | aes/gemgsxif.c:98 | `ta/aes/gsxif.rs::gsx_mcalc` | m | m | - | - | - |
| `gsx_mfree` | aes/gemgsxif.c:118 | UNMAPPED | u | - | - | - | - |
| `gsx_mfset` | aes/gemgsxif.c:407 | `ta/aes/graf.rs::set_user_form` +1 | m | m | R | - | - |
| `gsx_moff` | aes/gemgsxif.c:440 | `ta/vdi/mouse.rs::hide` | m | m | R | - | - |
| `gsx_mon` | aes/gemgsxif.c:450 | `ta/vdi/mouse.rs::show` | m | m | R | - | - |
| `gsx_mret` | aes/gemgsxif.c:125 | `ta/aes/gsxif.rs::gsx_mret` | m | m | R | - | - |
| `gsx_mxmy` | aes/gemgsxif.c:424 | UNMAPPED | u | m | - | - | - |
| `gsx_ncode` | aes/gemgsxif.c:133 | `ta/aes/gsxif.rs::ncode` | m | - | R | - | - |
| `gsx_nplanes` | aes/gemgsxif.c:497 | `ta/vdi/backend.rs::planes` | m | m | - | - | - |
| `gsx_pline` | aes/gemgraf.c:170 | UNMAPPED | u | m | - | - | - |
| `gsx_resetmb` | aes/gemgsxif.c:238 | UNMAPPED | u | - | - | - | - |
| `gsx_setmb` | aes/gemgsxif.c:219 | `ta/aes/mod.rs::user_but` +1 | m | m | - | - | - |
| `gsx_setmousexy` | aes/gemgsxif.c:484 | UNMAPPED | u | i | - | - | - |
| `gsx_start` | aes/gemgraf.c:383 | `ta/aes/gsxif.rs::gsx_init` +2 | m | m | - | - | - |
| `gsx_tblt` | aes/gemgraf.c:500 | `ta/aes/object.rs::gsx_tblt` +1 | m | m | R | - | - |
| `gsx_tcalc` | aes/gemgraf.c:480 | `ta/aes/object.rs::gr_gtext` | m | m | R | - | - |
| `gsx_textsize` | aes/gemgsxif.c:505 | `ta/aes/graf.rs::gl_wchar` +1 | m | m | - | - | - |
| `gsx_tick` | aes/gemgsxif.c:397 | `ta/aes/input.rs::now_tick` | m | m | - | - | - |
| `gsx_trans` | aes/gemgraf.c:369 | UNMAPPED | u | m | - | - | - |
| `gsx_wsclear` | aes/gemgsxif.c:198 | UNMAPPED | u | m | - | - | - |
| `gsx_wsclose` | aes/gemgsxif.c:191 | `ta/aes/gsxif.rs::gsx_wsclose` | m | m | - | - | - |
| `gsx_wsopen` | aes/gemgsxif.c:169 | `ta/aes/gsxif.rs::gsx_init` | m | m | - | - | - |
| `gsx_xbox` | aes/gemgraf.c:274 | `ta/aes/graf.rs::gsx_xbox` | m | m | - | - | - |
| `gsx_xcbox` | aes/gemgraf.c:285 | UNMAPPED | u | - | - | - | - |
| `gsx_xline` | aes/gemgraf.c:140 | `ta/aes/graf.rs::gsx_xline` | m | m | - | - | - |
| `handle_arrow_msg` | aes/gemctrl.c:111 | `ta/aes/window.rs::arrow_begin` +1 | m | m | - | - | - |
| `handle_popup` | aes/gemmnext.c:245 | `ta/aes/menu.rs::popup_shown` | m | m | - | - | - |
| `hctl_msg` | aes/gemctrl.c:380 | UNMAPPED | u | m | - | - | - |
| `hctl_rect` | aes/gemctrl.c:318 | `ta/aes/event.rs::menu_intercept` | m | m | - | - | - |
| `hctl_window` | aes/gemctrl.c:158 | `ta/aes/window.rs::hctl_window` +2 | m | m | - | - | - |
| `iasync` | aes/gemasync.c:130 | UNMAPPED | u | m | R | - | - |
| `in_mrect` | aes/geminput.c:76 | `ta/aes/event.rs::rect_event` | m | m | R | - | - |
| `init_3d` | aes/gemoblib.c:1327 | `ta/aes/object.rs::init_3d` | m | m | - | - | - |
| `init_p0_stkptr` | aes/geminit.c:161 | UNMAPPED | u | - | - | - | - |
| `inorout` | aes/geminput.c:427 | `ta/aes/event.rs::rect_event` | m | - | R | - | - |
| `ins_char` | aes/gemobed.c:117 | `ta/aes/fsel.rs::insert_char` +1 | m | - | R | - | - |
| `insert_process` | aes/gempd.c:134 | `ta/bdos/process/sched.rs::ready` | m | m | R | - | - |
| `instr` | aes/gemobed.c:228 | `ta/aes/object.rs::instr` | m | - | R | - | - |
| `iprocess` | aes/geminit.c:215 | UNMAPPED | u | m | - | - | - |
| `is_dashes` | aes/gemoblib.c:455 | `ta/aes/object.rs::is_dashes` | m | m | R | - | - |
| `item_changed` | aes/gemmnlib.c:178 | `ta/aes/menu.rs::track_owned` | m | m | - | - | - |
| `just_draw` | aes/gemoblib.c:471 | `ta/aes/object.rs::just_draw` +8 | m | m | R | - | - |
| `kchange` | aes/geminput.c:288 | UNMAPPED | u | m | - | - | - |
| `load_accs` | aes/geminit.c:309 | `k/accs.rs::load_accs` | m | m | - | - | - |
| `load_mouse_cursors` | aes/geminit.c:471 | UNMAPPED | u | - | - | - | - |
| `load_one_acc` | aes/geminit.c:236 | `k/accs.rs::load_one_acc` | m | m | - | - | ACC02/P |
| `mchange` | aes/geminput.c:360 | `ta/aes/input.rs::mchange` | m | m | - | - | - |
| `menu_down` | aes/gemmnlib.c:229 | `ta/aes/menu.rs::open_box` | m | m | - | - | - |
| `menu_fixup` | aes/gemmnlib.c:89 | `ta/aes/menu.rs::menu_fixup` | m | m | - | - | - |
| `menu_select` | aes/gemmnlib.c:192 | `ta/aes/menu.rs::rehilite` | m | m | - | - | - |
| `menu_sr` | aes/gemmnlib.c:208 | `ta/aes/menu.rs::sr_rect` +1 | m | m | - | - | - |
| `menu_sub` | aes/gemmnlib.c:71 | `ta/aes/menu.rs::menu_sub` | m | m | - | - | - |
| `mkpiece` | aes/gemwrect.c:64 | `ta/aes/window.rs::subtract` | m | m | - | - | - |
| `mkrect` | aes/gemwrect.c:136 | `ta/aes/window.rs::border_pieces` +1 | m | m | - | - | - |
| `mn_cleanup` | aes/gemmnlib.c:740 | `ta/aes/menu.rs::mn_cleanup` | m | m | - | - | - |
| `mn_do` | aes/gemmnlib.c:249 | `ta/aes/menu.rs::track` +1 | m | m | - | - | - |
| `mn_getownid` | aes/gemmnlib.c:838 | `ta/aes/menu.rs::mn_getownid` | m | m | - | - | - |
| `mn_init` | aes/gemmnlib.c:821 | `ta/aes/menu.rs::reinit` | m | m | - | - | - |
| `mnext_init` | aes/gemmnext.c:103 | `ta/aes/menu.rs::mnext_init` | m | m | - | - | - |
| `mwait` | aes/gemasync.c:117 | `ta/aes/input.rs::mn_button_take` +1 | m | m | R | - | - |
| `mwait_act` | aes/gemdisp.c:82 | `ta/bdos/process/sched.rs::sleep` | m | m | R | - | - |
| `newrect` | aes/gemwrect.c:165 | `ta/aes/window.rs::border_pieces` +1 | m | m | R | - | - |
| `nq` | aes/geminput.c:241 | UNMAPPED | u | - | - | - | - |
| `ob_actxywh` | aes/gemoblib.c:313 | `ta/aes/object.rs::ob_actxywh_t` | m | m | R | - | - |
| `ob_delit` | aes/gemobed.c:323 | `ta/aes/object.rs::delete_at` | m | - | R | - | - |
| `ob_format` | aes/gemoblib.c:361 | `ta/aes/object.rs::ob_format` | m | m | R | - | - |
| `ob_fs` | aes/gemoblib.c:1427 | `ta/aes/form.rs::ob_fs` | m | - | R | - | - |
| `ob_getsp` | aes/gemobed.c:55 | `ta/aes/object.rs::tedinfo_read_abi` | m | m | R | - | - |
| `ob_relxywh` | aes/gemoblib.c:302 | `ta/aes/object.rs::x` +1 | m | m | R | - | - |
| `ob_setxywh` | aes/gemoblib.c:349 | `ta/aes/object.rs::set_x` +1 | m | - | R | - | - |
| `ob_sst` | aes/gemobjop.c:25 | `ta/aes/object.rs::sst_th` | m | m | R | - | A68/I |
| `ob_stfn` | aes/gemobed.c:316 | `ta/aes/object.rs::edit_field_t` | m | m | R | - | - |
| `ob_user` | aes/gemoblib.c:433 | `ta/aes/object.rs::ob_user` | m | m | R | - | - |
| `or_start` | aes/gemwrect.c:40 | UNMAPPED | u | m | - | - | - |
| `p_nameit` | aes/gempd.c:77 | `ta/aes/pd.rs::p_nameit` | m | m | - | - | - |
| `p_setappdir` | aes/gempd.c:90 | `ta/aes/resource.rs::set_app_dir` | m | m | - | - | - |
| `path_changed` | aes/gemfslib.c:572 | `ta/aes/fsel.rs::path_differs` | m | - | - | - | - |
| `pd_index` | aes/gempd.c:32 | `ta/aes/pd.rs::by_index` | m | m | R | - | - |
| `perform_untop` | aes/gemctrl.c:146 | `ta/aes/window.rs::post_untopped` | m | m | - | - | A121/P |
| `popup_blit` | aes/gemmnext.c:136 | `ta/aes/gsxif.rs::popup_blit_save` | m | m | - | - | - |
| `post_button` | aes/geminput.c:463 | `ta/aes/input.rs::post_button` | m | m | R | - | - |
| `post_keybd` | aes/geminput.c:299 | UNMAPPED | u | m | - | - | - |
| `post_mouse` | aes/geminput.c:446 | `ta/aes/event.rs::evnt_mouse` +1 | m | m | R | - | - |
| `process_inf1` | aes/geminit.c:343 | `ta/aes/init.rs::process_inf1` | m | m | - | - | - |
| `process_inf2` | aes/geminit.c:407 | `ta/aes/init.rs::process_inf2` | m | m | - | - | - |
| `psetup` | aes/gemasm.S:48 | `ta/bdos/process/sched.rs::admit` | m | m | - | - | - |
| `pstart` | aes/gempd.c:110 | `ta/aes/acc.rs::register_process` | m | m | - | - | - |
| `pxl_rect` | aes/gemobed.c:176 | `ta/aes/object.rs::pxl_rect` | m | m | R | - | - |
| `query_submenu` | aes/gemmnext.c:401 | `ta/aes/menu.rs::query_submenu` | m | m | - | - | - |
| `ratexit` | aes/gemgsxif.c:212 | `k/shell_loop.rs::sh_toalpha` | m | m | - | - | - |
| `ratinit` | aes/gemgsxif.c:205 | `k/shell_loop.rs::ratinit` | m | m | R | - | - |
| `readfile` | aes/geminit.c:323 | `ta/aes/init.rs::read_infbuf` | m | m | - | - | - |
| `rect_change` | aes/gemmnlib.c:131 | UNMAPPED | u | - | - | - | - |
| `retake` | aes/gemdosif.S:94 | design:hardware:m68k-cpu | d | m | - | - | - |
| `rs_fixit` | aes/gemrslib.c:816 | `ta/aes/resource.rs::fix_objects_coords` | m | m | - | - | - |
| `rs_readit` | aes/gemrslib.c:751 | `ta/aes/resource.rs::relocate_and_register_core` +1 | m | m | - | - | - |
| `rs_sglobe` | aes/gemrslib.c:675 | `ta/aes/resource.rs::selected` | m | m | - | - | - |
| `rs_str` | aes/gemrslib.c:854 | `ta/aes/resource.rs::blob_frstr` | m | m | R | - | - |
| `run_accs_and_desktop` | aes/geminit.c:654 | `k/lib.rs::kernel_main_rust` +1 | m | m | - | - | - |
| `scan_to_end` | aes/gemobed.c:141 | `ta/aes/object.rs::scan_to_end` | m | m | R | - | - |
| `schedule` | aes/gemdisp.c:181 | `ta/bdos/process/sched.rs::pick_head` | m | m | R | - | - |
| `screen_rez` | aes/gemgsxif.c:159 | `ta/aes/gsxif.rs::gsx_init` | m | - | - | - | - |
| `select_drive` | aes/gemfslib.c:540 | `ta/aes/fsel.rs::set_drive_states` | m | m | - | - | - |
| `set_aes_background` | aes/geminit.c:635 | `ta/aes/init.rs::set_aes_background` | m | m | - | - | DESK66/P |
| `set_aestrap` | aes/gemdosif.S:295 | design:hardware:m68k-cpu | d | m | - | - | - |
| `set_ctrl` | aes/geminput.c:189 | `ta/aes/window.rs::chk_ctrl` | m | m | R | - | - |
| `set_default_desktop` | aes/gemshlib.c:583 | `ta/aes/shell.rs::set_default_desktop` | m | m | - | - | - |
| `set_drvdir` | aes/gemshlib.c:526 | `ta/aes/shell.rs::set_drvdir` | m | m | - | - | - |
| `set_mask` | aes/gemfslib.c:524 | `ta/aes/fsel.rs::set_mask_in` +1 | m | m | - | - | - |
| `set_mouse_to_arrow` | aes/geminit.c:183 | `ta/vdi/mouse.rs::set_preset` | m | - | R | - | - |
| `set_mouse_to_hourglass` | aes/geminit.c:192 | `ta/vdi/mouse.rs::set_preset` | m | - | - | - | - |
| `set_mown` | aes/geminput.c:220 | `ta/aes/input.rs::set_mown` | m | m | R | - | A102/I |
| `setup_moblk` | aes/gemmnext.c:220 | UNMAPPED | u | - | - | - | - |
| `setup_mouse_cursors` | aes/geminit.c:513 | UNMAPPED | u | m | - | - | - |
| `sh_chdef` | aes/gemshlib.c:537 | `ta/aes/shell.rs::sh_chdef` | m | m | - | - | - |
| `sh_chgrf` | aes/gemshlib.c:513 | `k/shell_loop.rs::sh_chgrf` | m | m | - | - | - |
| `sh_curdrvdir` | aes/gemshlib.c:109 | `ta/aes/shell.rs::sh_curdrvdir` | m | m | - | - | - |
| `sh_draw` | aes/gemshlib.c:269 | `ta/aes/shell.rs::sh_draw` | m | m | - | - | - |
| `sh_ldapp` | aes/gemshlib.c:592 | `k/shell_loop.rs::sh_ldapp` | m | m | - | - | - |
| `sh_main` | aes/gemshlib.c:692 | `k/shell_loop.rs::sh_main` +1 | m | m | - | - | A106/I |
| `sh_name` | aes/gemshlib.c:305 | `ta/aes/shell.rs::sh_name` | m | m | - | - | - |
| `sh_path` | aes/gemshlib.c:356 | UNMAPPED | u | - | - | - | - |
| `sh_show` | aes/gemshlib.c:292 | `ta/aes/shell.rs::sh_show` | m | m | - | - | - |
| `sh_toalpha` | aes/gemshlib.c:246 | `k/shell_loop.rs::sh_toalpha` | m | m | - | - | - |
| `sh_tographic` | aes/gemshlib.c:223 | `k/shell_loop.rs::sh_tographic` | m | m | - | - | - |
| `signal` | aes/gemasync.c:35 | `ta/bdos/process/sched.rs::push_drl` | m | m | R | - | - |
| `size_theglo` | aes/geminit.c:149 | UNMAPPED | u | - | - | - | - |
| `super` | aes/gemsuper.c:454 | `ta/aes/mod.rs::dispatch_in` +1 | m | m | - | - | - |
| `switchto` | aes/gemasm.S:169 | design:ADR-029@docs/MEMORY.md:2749 | d | m | R | - | - |
| `tak_flag` | aes/gemflag.c:71 | `ta/aes/window.rs::tak_flag` | m | m | R | - | - |
| `takeerr` | aes/gemdosif.S:125 | design:ADR-031@docs/MEMORY.md:3044 | d | - | - | - | - |
| `takeoff` | aes/gemasync.c:96 | UNMAPPED | u | m | R | - | - |
| `tchange` | aes/gemflag.c:32 | `ta/bdos/process/sched.rs::wake_timers` | m | m | - | - | - |
| `tikcod` | aes/gemdosif.S:389 | `ta/aes/input.rs::b_delay_poll` +1 | m | m | - | - | - |
| `transform_all_cicons` | aes/gemrslib.c:403 | `ta/aes/resource.rs::fix_cicons` | m | m | - | - | - |
| `transform_cicon` | aes/gemrslib.c:348 | `ta/aes/resource.rs::transform_cicon` | m | m | - | - | - |
| `ui_start` | aes/gemstart.S:46 | `ta/bdos/process/standin.rs::enter_aes` | m | m | - | - | - |
| `undisplay_submenu` | aes/gemmnext.c:683 | `ta/aes/menu.rs::close_submenu` | m | - | - | - | - |
| `unset_aestrap` | aes/gemdosif.S:287 | design:hardware:m68k-cpu | d | m | - | - | - |
| `unsync` | aes/gemflag.c:105 | `ta/aes/window.rs::unsync` | m | m | R | - | - |
| `v_opnwk` | aes/gemgsxif.c:552 | `ta/vdi/workstation.rs::v_opnwk` | m | m | - | D33 D36 D37 D38 +2 | V53/D |
| `v_pline` | aes/gemgsxif.c:573 | `ta/vdi/backend.rs::draw_line` | m | m | R | - | V40/D |
| `vex_wheelv` | aes/gemgsxif.c:663 | `ta/vdi/mouse.rs::install_aes_wheel` | m | m | - | - | - |
| `vr_recfl` | aes/gemgsxif.c:604 | `ta/vdi/backend.rs::fill_rect` | m | m | R | D22 | - |
| `vrn_trnfm` | aes/gemgsxif.c:640 | UNMAPPED | u | m | - | - | - |
| `vro_cpyfm` | aes/gemgsxif.c:613 | `ta/vdi/raster.rs::bb_save` +2 | m | m | R | - | R01/P R02/P R03/P +2 |
| `vrt_cpyfm` | aes/gemgsxif.c:625 | `ta/vdi/backend.rs::blit_mono` | m | m | R | - | R05/I V28/D V50/D |
| `vs_clip` | aes/gemgsxif.c:581 | UNMAPPED | u | m | R | D22 D32 | - |
| `vsl_width` | aes/gemgsxif.c:653 | `ta/aes/gsxif.rs::gsx_init` | m | m | - | - | - |
| `vst_height` | aes/gemgsxif.c:590 | `ta/aes/gsxif.rs::vst_height` | m | m | R | D31 | V45/P |
| `w_adjust` | aes/gemwmlib.c:271 | `ta/aes/window.rs::vbar_box` +4 | m | m | R | - | - |
| `w_bldactive` | aes/gemwmlib.c:479 | `ta/aes/window.rs::draw_frame` +3 | m | m | R | - | - |
| `w_bldhbar` | aes/gemwmlib.c:441 | `ta/aes/window.rs::hbar_box` +6 | m | m | R | - | - |
| `w_bldvbar` | aes/gemwmlib.c:403 | `ta/aes/window.rs::vbar_box` +6 | m | m | R | - | - |
| `w_cpwalk` | aes/gemwmlib.c:363 | `ta/aes/window.rs::draw_frame_pieces` +3 | m | m | R | - | - |
| `w_drawdesk` | aes/gemwmlib.c:340 | `ta/aes/window.rs::paint_bg_clip` +2 | m | m | R | - | - |
| `w_getsize` | aes/gemwmlib.c:254 | `ta/aes/window.rs::ws_true` +1 | m | m | R | - | - |
| `w_getxptr` | aes/gemwmlib.c:231 | `ta/aes/window.rs::win` | m | m | R | - | - |
| `w_move` | aes/gemwmlib.c:938 | `ta/aes/window.rs::w_move` +1 | m | m | R | - | A78/I |
| `w_mvfix` | aes/gemwmlib.c:915 | `ta/aes/window.rs::w_mvfix` | m | m | R | - | - |
| `w_nilit` | aes/gemwmlib.c:172 | UNMAPPED | u | m | R | - | - |
| `w_obadd` | aes/gemwmlib.c:186 | UNMAPPED | u | m | R | - | - |
| `w_owns` | aes/gemwmlib.c:1230 | `ta/aes/window.rs::rlist_owns` | m | m | R | - | - |
| `w_redraw` | aes/gemwmlib.c:887 | `ta/aes/window.rs::send_redraw_in` | m | m | R | - | - |
| `w_redraw_desktop` | aes/gemwmlib.c:1902 | UNMAPPED | u | m | - | - | - |
| `w_setactive` | aes/gemwmlib.c:321 | `ta/aes/window.rs::chk_ctrl` | m | m | R | - | - |
| `w_setcolor` | aes/gemwmlib.c:382 | `ta/aes/window.rs::gadget_ink` +1 | m | m | R | - | - |
| `w_setsize` | aes/gemwmlib.c:265 | `ta/aes/window.rs::set_curr` | m | m | R | - | - |
| `w_setup` | aes/gemwmlib.c:205 | `ta/aes/window.rs::wind_create` +1 | m | m | R | - | - |
| `w_top` | aes/gemwmlib.c:315 | `ta/aes/window.rs::get_top` | m | m | R | - | - |
| `w_union` | aes/gemwmlib.c:869 | `ta/aes/window.rs::w_union` | m | - | R | - | - |
| `w_update` | aes/gemwmlib.c:1004 | `ta/aes/window.rs::w_update` +1 | m | m | R | - | - |
| `wait_for_accs` | aes/geminit.c:599 | `ta/aes/acc.rs::wait_for_accs` | m | m | - | - | ACC03/P ACC10/D |
| `wheel_change` | aes/geminput.c:395 | `ta/aes/input.rs::wheel_change` +1 | m | m | - | - | - |
| `wm_init` | aes/gemwmlib.c:1247 | `ta/aes/window.rs::wm_init` +2 | m | m | - | - | - |
| `wm_mktop` | aes/gemwmlib.c:1605 | `ta/aes/window.rs::wm_mktop` | m | m | R | - | - |
| `wm_start` | aes/gemwmlib.c:1332 | `ta/aes/window.rs::reinit` | m | m | - | - | - |
| `xif` | aes/gemsuper.c:428 | `ta/aes/mod.rs::dispatch_in` | m | m | - | - | - |
| `xlate_obj_array` | aes/gemrslib.c:865 | UNMAPPED | u | m | - | - | - |
| `xor_color` | aes/gemoblib.c:93 | `ta/aes/object.rs::xor_color` | m | m | R | - | - |
| `xor_is_ok` | aes/gemoblib.c:107 | `ta/aes/object.rs::xor_is_ok` | m | m | R | - | - |

### 2.7 VDI API entries (trap #2, `vdi/vdi_main.c:32-111` `jmptb1`, `jmptb2`; the slot is the opcode) - 68 rows

| slot | API | fn | def | NewTOS | c | spec | oracle | dev | tickets |
|---|---|---|---|---|---|---|---|---|---|
| 0x1 | v_opnwk | `vdi_v_opnwk` | vdi/vdi_control.c:496 | `ta/vdi/workstation.rs::v_opnwk` +1 | m | F `docs/api/vdi/workstation/01-v_opnwk.md` | O 1: vdi-v18-t3*1 | D33 D36 D37 D38 +2 | V53/D |
| 0x2 | v_clswk | `vdi_v_clswk` | vdi/vdi_control.c:573 | `ta/vdi/workstation.rs::v_clswk` +1 | m | H `docs/api/vdi/workstation/101-v_clsvwk.md:76` | O 1: vdi-v18-t3*1 | C1829 | V53/D |
| 0x3 | v_clrwk | `vdi_v_clrwk` | vdi/vdi_control.c:599 | `ta/vdi/workstation.rs::v_clrwk` | m | F `docs/api/vdi/workstation/03-v_clrwk.md` | O 2: vdi-all*1,vdi-r2*1 | - | - |
| 0x1b,0x1d,0x22,0x4,0xa,0x84,0x85 | just_rts,v_curup,v_eeol | `just_rts` | - | ext | e | H `docs/api/xbios/22-kbdvbase.md:405` | O 14: vdi-all*7,vdi-r2*7 | - | V56/D |
| 0x5 | v_escape | `vdi_v_escape` | vdi/vdi_esc.c:334 | `ta/vdi/escape.rs::v_escape` | m | H `docs/api/bios/_vt52.md:393` | O 33: vdi-all*16,vdi-r2*16,vdi-v18-t2*1 | - | - |
| 0x6 | v_pline | `vdi_v_pline` | vdi/vdi_line.c:1000 | `ta/vdi/primitives.rs::v_pline` | m | H `docs/api/vdi/output/_lines-markers.md:37` | O 6: vdi-all*3,vdi-r1*2,vdi-r2*1 | - | V40/D |
| 0x7 | v_pmarker | `vdi_v_pmarker` | vdi/vdi_marker.c:86 | `ta/vdi/primitives.rs::v_pmarker` | m | H `docs/api/vdi/output/_lines-markers.md:229` | O 2: vdi-all*1,vdi-r2*1 | D32 | V46/P |
| 0x8 | v_gtext | `vdi_v_gtext` | vdi/vdi_text.c:546 | `ta/vdi/text.rs::v_gtext` | m | H `docs/api/vdi/output/_text.md:1` | O 2: vdi-all*1,vdi-r2*1 | - | - |
| 0x9 | v_fillarea | `vdi_v_fillarea` | vdi/vdi_fill.c:617 | `ta/vdi/primitives.rs::v_fillarea` | m | H `docs/api/vdi/output/_fill.md:38` | O 4: vdi-all*2,vdi-r1*1,vdi-r2*1 | C713 | V22/D |
| 0xb | v_gdp | `vdi_v_gdp` | vdi/vdi_gdp.c:426 | `ta/vdi/gdp.rs::v_gdp` | m | H `docs/api/vdi/output/_gdp.md:1` | O 20: vdi-all*10,vdi-r2*9,vdi-r3*1 | - | - |
| 0xc | vst_height | `vdi_vst_height` | vdi/vdi_text.c:681 | `ta/vdi/attributes.rs::vst_height` +1 | m | T `docs/api/vdi/inquiry/_attributes.md:111` | O 7: vdi-all*3,vdi-r2*3,vdi-v18-t2*1 | D31 | V45/P |
| 0xd | vst_rotation | `vdi_vst_rotation` | vdi/vdi_text.c:924 | `ta/vdi/attributes.rs::vst_rotation` | m | H `docs/api/vdi/attributes/_text.md:72` | O 10: vdi-all*5,vdi-r1*2,vdi-r2*3 | - | - |
| 0xe | vs_color | `vdi_vs_color` | vdi/vdi_col.c:656 | `ta/vdi/attributes.rs::vs_color` | m | H `docs/api/vdi/attributes/_mode-clip-color.md:80` | none | - | - |
| 0xf | vsl_type | `vdi_vsl_type` | vdi/vdi_line.c:90 | `ta/vdi/attributes.rs::vsl_type` | m | H `docs/api/vdi/attributes/_line-marker.md:57` | O 10: vdi-all*5,vdi-r1*2,vdi-r2*3 | - | - |
| 0x10 | vsl_width | `vdi_vsl_width` | vdi/vdi_line.c:104 | `ta/vdi/attributes.rs::vsl_width` | m | T `docs/api/vdi/inquiry/_attributes.md:50` | O 12: vdi-all*5,vdi-r2*5,vdi-v18-t2*2 | - | - |
| 0x11 | vsl_color | `vdi_vsl_color` | vdi/vdi_line.c:143 | `ta/vdi/attributes.rs::vsl_color` | m | T `docs/api/vdi/inquiry/_attributes.md:48` | O 8: vdi-all*4,vdi-r1*2,vdi-r2*2 | - | - |
| 0x12 | vsm_type | `vdi_vsm_type` | vdi/vdi_marker.c:57 | `ta/vdi/attributes.rs::vsm_type` | m | H `docs/api/vdi/attributes/_line-marker.md:107` | O 8: vdi-all*4,vdi-r2*4 | - | - |
| 0x13 | vsm_height | `vdi_vsm_height` | vdi/vdi_marker.c:30 | `ta/vdi/attributes.rs::vsm_height` | m | H `docs/api/vdi/attributes/_line-marker.md:121` | O 7: vdi-all*3,vdi-r2*3,vdi-v18-t2*1 | - | - |
| 0x14 | vsm_color | `vdi_vsm_color` | vdi/vdi_marker.c:72 | `ta/vdi/attributes.rs::vsm_color` | m | T `docs/api/vdi/inquiry/_attributes.md:63` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x15 | vst_font | `vdi_vst_font` | vdi/vdi_text.c:939 | `ta/vdi/attributes.rs::vst_font` | m | H `docs/api/vdi/attributes/_text.md:94` | O 8: vdi-all*3,vdi-r2*3,vdi-v18-t2*2 | - | - |
| 0x16 | vst_color | `vdi_vst_color` | vdi/vdi_text.c:988 | `ta/vdi/attributes.rs::vst_color` | m | T `docs/api/vdi/inquiry/_attributes.md:106` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x17 | vsf_interior | `vdi_vsf_interior` | vdi/vdi_fill.c:173 | `ta/vdi/attributes.rs::vsf_interior` | m | H `docs/api/vdi/attributes/_fill.md:41` | O 12: vdi-all*6,vdi-r1*2,vdi-r2*4 | - | - |
| 0x18 | vsf_style | `vdi_vsf_style` | vdi/vdi_fill.c:186 | `ta/vdi/attributes.rs::vsf_style` | m | H `docs/api/vdi/attributes/_fill.md:51` | O 8: vdi-all*4,vdi-r1*1,vdi-r2*3 | - | - |
| 0x19 | vsf_color | `vdi_vsf_color` | vdi/vdi_fill.c:206 | `ta/vdi/attributes.rs::vsf_color` | m | T `docs/api/vdi/inquiry/_attributes.md:85` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x1a | vq_color | `vdi_vq_color` | vdi/vdi_col.c:845 | `ta/vdi/inquiry.rs::vq_color` | m | H `docs/api/vdi/inquiry/_color-pixel-input.md:33` | O 42: vdi-a131-palette*32,vdi-all*5,vdi-r2*5 | - | - |
| 0x1c | v_locator | `vdi_v_locator` | vdi/vdi_mouse.c:280 | `ta/vdi/input.rs::v_locator` | m | H `docs/api/vdi/input/_input.md:80` | none | - | - |
| 0x1e | v_choice | `vdi_v_choice` | vdi/vdi_input.c:31 | `ta/vdi/input.rs::v_choice` | m | H `docs/api/vdi/input/_input.md:164` | O 2: vdi-all*1,vdi-r2*1 | - | - |
| 0x1f | v_string | `vdi_v_string` | vdi/vdi_input.c:79 | `ta/vdi/input.rs::v_string` | m | H `docs/api/vdi/input/_input.md:201` | none | C84 | - |
| 0x20 | vswr_mode | `vdi_vswr_mode` | vdi/vdi_control.c:256 | `ta/vdi/attributes.rs::vswr_mode` | m | H `docs/api/vdi/attributes/_mode-clip-color.md:34` | O 10: vdi-all*5,vdi-r1*2,vdi-r2*3 | - | - |
| 0x21 | vsin_mode | `vdi_vsin_mode` | vdi/vdi_input.c:118 | `ta/vdi/input.rs::vsin_mode` | m | H `docs/api/vdi/input/_input.md:47` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x23 | vql_attributes | `vdi_vql_attributes` | vdi/vdi_line.c:156 | `ta/vdi/inquiry.rs::vql_attributes` | m | H `docs/api/vdi/inquiry/_attributes.md:41` | O 12: vdi-all*6,vdi-r1*1,vdi-r3*5 | - | - |
| 0x24 | vqm_attributes | `vdi_vqm_attributes` | vdi/vdi_marker.c:169 | `ta/vdi/inquiry.rs::vqm_attributes` | m | H `docs/api/vdi/inquiry/_attributes.md:56` | O 8: vdi-all*4,vdi-r1*1,vdi-r3*3 | - | - |
| 0x25 | vqf_attributes | `vdi_vqf_attributes` | vdi/vdi_fill.c:321 | `ta/vdi/inquiry.rs::vqf_attributes` | m | H `docs/api/vdi/inquiry/_attributes.md:78` | O 10: vdi-all*5,vdi-r1*1,vdi-r3*4 | - | - |
| 0x26 | vqt_attributes | `vdi_vqt_attributes` | vdi/vdi_text.c:999 | `ta/vdi/inquiry.rs::vqt_attributes` | m | H `docs/api/vdi/inquiry/_attributes.md:99` | O 20: vdi-all*8,vdi-r1*1,vdi-r3*7 +1 tbl | - | - |
| 0x27 | vst_alignment | `vdi_vst_alignment` | vdi/vdi_text.c:887 | `ta/vdi/attributes.rs::vst_alignment` | m | H `docs/api/vdi/attributes/_text.md:65` | O 8: vdi-all*4,vdi-r1*1,vdi-r2*3 | - | - |
| 0x64 | v_opnvwk | `vdi_v_opnvwk` | vdi/vdi_control.c:407 | `ta/vdi/workstation.rs::v_opnvwk` +1 | m | F `docs/api/vdi/workstation/100-v_opnvwk.md` | O 2: vdi-all*1,vdi-r3*1 | - | A101/D |
| 0x65 | v_clsvwk | `vdi_v_clsvwk` | vdi/vdi_control.c:462 | `ta/vdi/workstation.rs::v_clsvwk` +1 | m | F `docs/api/vdi/workstation/101-v_clsvwk.md` | none | D37 | - |
| 0x66 | vq_extnd | `vdi_vq_extnd` | vdi/vdi_control.c:626 | `ta/vdi/workstation.rs::vq_extnd` | m | F `docs/api/vdi/workstation/102-vq_extnd.md` | O 8: vdi-all*4,vdi-r1*2,vdi-r3*2 | - | V03/P |
| 0x67 | v_contourfill | `vdi_v_contourfill` | vdi/vdi_fill.c:1084 | `ta/vdi/contour.rs::v_contourfill` | m | H `docs/api/vdi/output/_fill.md:172` | O 4: vdi-all*2,vdi-r2*1,vdi-r3*1 | D39 C1768 | V21/D V38/D |
| 0x68 | vsf_perimeter | `vdi_vsf_perimeter` | vdi/vdi_fill.c:219 | `ta/vdi/attributes.rs::vsf_perimeter` | m | T `docs/api/vdi/inquiry/_attributes.md:88` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x69 | v_get_pixel | `vdi_v_get_pixel` | vdi/vdi_fill.c:1095 | `ta/vdi/inquiry.rs::v_get_pixel` | m | H `docs/api/vdi/inquiry/_color-pixel-input.md:74` | O 59: vdi-a131-palette*16,vdi-all*5,vdi-r1*1 +4 tbl | C688 | V11/I V24/D |
| 0x6a | vst_effects | `vdi_vst_effects` | vdi/vdi_text.c:881 | `ta/vdi/attributes.rs::vst_effects` | m | H `docs/api/vdi/attributes/_text.md:49` | O 6: vdi-all*3,vdi-r1*1,vdi-r2*2 | D28 | A139/P |
| 0x6b | vst_point | `vdi_vst_point` | vdi/vdi_text.c:820 | `ta/vdi/attributes.rs::vst_point` +1 | m | T `docs/api/vdi/inquiry/_attributes.md:111` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x6c | vsl_ends | `vdi_vsl_ends` | vdi/vdi_line.c:128 | `ta/vdi/attributes.rs::vsl_ends` | m | H `docs/api/vdi/attributes/_line-marker.md:72` | O 8: vdi-all*4,vdi-r2*4 | - | - |
| 0x6d | vro_cpyfm | `vdi_vro_cpyfm` | vdi/vdi_raster.c:1471 | `ta/vdi/raster.rs::vro_cpyfm` | m | H `docs/api/vdi/raster/_raster.md:237` | O 109: vdi-raster*34,vdi-raster-screen*45,vdi-v18-raster*17 +1 tbl | - | R01/P R02/P R03/P +2 |
| 0x6e | vr_trnfm | `vdi_vr_trnfm` | vdi/vdi_raster.c:269 | `ta/vdi/raster.rs::vr_trnfm` | m | H `docs/api/vdi/raster/_raster.md:421` | O 17: vdi-raster*17 | - | - |
| 0x6f | vsc_form | `vdi_vsc_form` | vdi/vdi_mouse.c:513 | `ta/vdi/mouse.rs::vsc_form` | m | H `docs/api/vdi/input/_mouse.md:51` | O 2: vdi-all*1,vdi-r3*1 | - | V54/D |
| 0x70 | vsf_udpat | `vdi_vsf_udpat` | vdi/vdi_fill.c:149 | `ta/vdi/attributes.rs::vsf_udpat` | m | H `docs/api/vdi/attributes/_fill.md:61` | none | - | - |
| 0x71 | vsl_udsty | `vdi_vsl_udsty` | vdi/vdi_line.c:81 | `ta/vdi/attributes.rs::vsl_udsty` | m | T `docs/api/vdi/attributes/_line-marker.md:43` | O 4: vdi-all*2,vdi-r2*2 | - | - |
| 0x72 | vr_recfl | `vdi_vr_recfl` | vdi/vdi_fill.c:293 | `ta/vdi/primitives.rs::vr_recfl` | m | H `docs/api/vdi/output/_fill.md:127` | O 2: vdi-all*1,vdi-r2*1 | D22 | - |
| 0x73 | vqin_mode | `vdi_vqin_mode` | vdi/vdi_input.c:154 | `ta/vdi/input.rs::vqin_mode` | m | H `docs/api/vdi/inquiry/_color-pixel-input.md:121` | O 12: vdi-all*6,vdi-r1*1,vdi-r2*3 +1 tbl | - | - |
| 0x74 | vqt_extent | `vdi_vqt_extent` | vdi/vdi_text.c:1024 | `ta/vdi/inquiry.rs::vqt_extent` | m | H `docs/api/vdi/inquiry/_text.md:36` | O 8: vdi-all*3,vdi-r2*2,vdi-r3*1 +1 tbl | - | - |
| 0x75 | vqt_width | `vdi_vqt_width` | vdi/vdi_text.c:1062 | `ta/vdi/inquiry.rs::vqt_width` +2 | m | H `docs/api/vdi/inquiry/_text.md:63` | O 10: vdi-all*4,vdi-r2*2,vdi-r3*2 +1 tbl | - | - |
| 0x76 | vex_timv | `vdi_vex_timv` | vdi/vdi_misc.c:100 | `ta/vdi/mouse.rs::vex_timv` | m | H `docs/api/vdi/input/_mouse.md:226` | none | D33 | - |
| 0x77 | vst_load_fonts | `vdi_vst_load_fonts` | vdi/vdi_text.c:1292 | `ta/vdi/attributes.rs::vst_load_fonts` +3 | m | H `docs/api/vdi/attributes/_text.md:103` | none | C1721 | V33/D |
| 0x78 | vst_unload_fonts | `vdi_vst_unload_fonts` | vdi/vdi_text.c:1353 | `ta/vdi/attributes.rs::vst_unload_fonts` | m | H `docs/api/vdi/attributes/_text.md:103` | none | - | V33/D |
| 0x79 | vrt_cpyfm | `vdi_vrt_cpyfm` | vdi/vdi_raster.c:1493 | `ta/vdi/raster.rs::vrt_cpyfm` | m | H `docs/api/vdi/raster/_raster.md:342` | O 38: vdi-raster*8,vdi-raster-screen*17,vdi-v18-t2-raster*13 | - | R05/I V28/D V50/D |
| 0x7a | v_show_c | `vdi_v_show_c` | vdi/vdi_mouse.c:321 | `ta/vdi/mouse.rs::v_show_c` | m | H `docs/api/vdi/input/_mouse.md:138` | O 4: vdi-all*2,vdi-r3*2 | D64 | - |
| 0x7b | v_hide_c | `vdi_v_hide_c` | vdi/vdi_mouse.c:331 | `ta/vdi/mouse.rs::v_hide_c` | m | H `docs/api/vdi/input/_mouse.md:138` | O 2: vdi-all*1,vdi-r3*1 | D64 | - |
| 0x7c | vq_mouse | `vdi_vq_mouse` | vdi/vdi_mouse.c:341 | `ta/vdi/mouse.rs::vq_mouse` | m | H `docs/api/vdi/input/_mouse.md:192` | none | - | - |
| 0x7d | vex_butv | `vdi_vex_butv` | vdi/vdi_mouse.c:370 | `ta/vdi/mouse.rs::vex_butv` | m | H `docs/api/vdi/input/_mouse.md:226` | none | D33 | - |
| 0x7e | vex_motv | `vdi_vex_motv` | vdi/vdi_mouse.c:391 | `ta/vdi/mouse.rs::vex_motv` | m | T `docs/api/vdi/input/_mouse.md:41` | none | D33 | - |
| 0x7f | vex_curv | `vdi_vex_curv` | vdi/vdi_mouse.c:414 | `ta/vdi/mouse.rs::vex_curv` | m | T `docs/api/vdi/input/_mouse.md:42` | none | - | - |
| 0x80 | vq_key_s | `vdi_vq_key_s` | vdi/vdi_input.c:110 | `ta/vdi/inquiry.rs::vq_key_s` | m | H `docs/api/vdi/inquiry/_color-pixel-input.md:156` | none | - | - |
| 0x81 | vs_clip | `vdi_vs_clip` | vdi/vdi_control.c:235 | `ta/vdi/attributes.rs::vs_clip` | m | H `docs/api/vdi/attributes/_mode-clip-color.md:64` | O 4: vdi-all*2,vdi-r2*2 | D22 D32 | - |
| 0x82 | vqt_name | `vdi_vqt_name` | vdi/vdi_text.c:1100 | `ta/vdi/inquiry.rs::vqt_name` | m | H `docs/api/vdi/inquiry/_text.md:79` | O 8: vdi-all*4,vdi-r2*4 | - | - |
| 0x83 | vqt_fontinfo | `vdi_vqt_fontinfo` | vdi/vdi_text.c:1137 | `ta/vdi/inquiry.rs::vqt_fontinfo` | m | H `docs/api/vdi/inquiry/_text.md:108` | O 11: vdi-all*4,vdi-r2*1,vdi-r3*3 +1 tbl | - | - |
| 0x86 | vex_wheelv | `vdi_vex_wheelv` | vdi/vdi_mouse.c:438 | `ta/vdi/mouse.rs::vex_wheelv` | m | H `docs/api/vdi/input/_mouse.md:226` | none | - | - |

### 2.8 vdi/ internals - 167 rows

| fn | def | NewTOS | c | spec | R | dev | tickets |
|---|---|---|---|---|---|---|---|
| `abline` | vdi/vdi_line.c:2266 | `k/vdi_backend.rs::abline` +1 | m | m | R | - | - |
| `act_siz` | vdi/vdi_text.c:743 | `ta/vdi/text.rs::act_siz` | m | m | R | - | - |
| `adjust_mono_values` | vdi/vdi_col.c:491 | design:hardware:video-shifter | d | - | R | - | - |
| `adjust_tt_colnum` | vdi/vdi_col.c:279 | design:hardware:video-shifter | d | m | R | - | - |
| `arb_corner` | vdi/vdi_misc.c:30 | `ta/vdi/attributes.rs::vs_clip` | m | m | R | - | - |
| `arb_line` | vdi/vdi_misc.c:54 | `ta/vdi/gdp.rs::rbox` | m | m | R | - | - |
| `arrow` | vdi/vdi_line.c:1597 | `ta/vdi/primitives.rs::arrow` | m | m | R | - | - |
| `bit_blt` | vdi/vdi_raster.c:588 | `ta/vdi/raster.rs::planar_mem_to_mem` +1 | m | m | R | - | - |
| `bub_sort` | vdi/vdi_fill.c:394 | `ta/vdi/primitives.rs::fill_polygon` | m | - | R | - | - |
| `build_vwk_chain` | vdi/vdi_control.c:390 | design:ADR-030@docs/MEMORY.md:2826 | d | m | R | - | - |
| `calc_height` | vdi/vdi_text.c:160 | `ta/vdi/text.rs::calc_height` | m | m | R | - | - |
| `calc_width` | vdi/vdi_text.c:176 | `ta/vdi/text.rs::calc_width` | m | m | R | - | - |
| `call_user_but` | vdi/vdi_asm.S:540 | `ta/aes/mod.rs::user_but` | m | - | - | - | - |
| `call_user_wheel` | vdi/vdi_asm.S:548 | `ta/vdi/mouse.rs::call_user_wheel` | m | m | - | - | - |
| `char_resize` | vdi/vdi_textblit.c:1257 | `ta/vdi/text.rs::resize` | m | m | R | - | - |
| `check_clip` | vdi/vdi_textblit.c:128 | `k/vdi_backend.rs::draw_glyph_sized` +2 | m | m | R | - | - |
| `cir_dda` | vdi/vdi_line.c:1240 | `ta/vdi/primitives.rs::cir_dda` | m | - | R | - | - |
| `clc_arc` | vdi/vdi_gdp.c:178 | `ta/vdi/gdp.rs::curve` | m | m | R | - | - |
| `clc_dda` | vdi/vdi_text.c:1374 | `ta/vdi/attributes.rs::clc_dda` | m | m | R | - | - |
| `clc_flit` | vdi/vdi_fill.c:434 | `ta/vdi/primitives.rs::fill_polygon` | m | m | R | - | - |
| `clc_nsteps` | vdi/vdi_gdp.c:347 | `ta/vdi/gdp.rs::clc_nsteps` | m | m | R | - | - |
| `clc_pts` | vdi/vdi_gdp.c:126 | `ta/vdi/gdp.rs::clc_pts` | m | m | R | - | - |
| `clip_code` | vdi/vdi_line.c:1034 | `ta/vdi/line.rs::clip_code` | m | m | R | - | - |
| `clip_line` | vdi/vdi_line.c:1057 | `ta/vdi/line.rs::clip_line` +1 | m | m | R | - | - |
| `clipbox` | vdi/vdi_fill.c:252 | `k/vdi_backend.rs::fill_pattern` | m | m | R | - | - |
| `contourfill` | vdi/vdi_fill.c:948 | `ta/vdi/contour.rs::v_contourfill` +2 | m | m | R | - | - |
| `cpy_raster` | vdi/vdi_raster.c:1331 | `ta/vdi/raster.rs::vro_cpyfm` +2 | m | m | R | - | - |
| `crawio` | vdi/vdi_esc.c:29 | `ta/gemdos/console.rs::rawio` | m | m | R | - | - |
| `crunch_queue` | vdi/vdi_fill.c:875 | `ta/vdi/contour.rs::crunch_queue` | m | m | R | - | - |
| `cur_display` | vdi/vdi_mouse.c:877 | `ta/vdi/mouse.rs::draw_at` | m | m | R | - | - |
| `cur_display16` | vdi/vdi_mouse.c:696 | `ta/vdi/mouse.rs::draw` | m | m | R | - | - |
| `cur_display_clip` | vdi/vdi_mouse.c:779 | design:ADR-024@docs/MEMORY.md:2158 | d | - | R | - | - |
| `cur_replace` | vdi/vdi_mouse.c:1066 | `ta/vdi/mouse.rs::erase` | m | m | R | - | - |
| `cur_replace16` | vdi/vdi_mouse.c:1032 | `ta/vdi/mouse.rs::erase` | m | m | R | - | - |
| `direct_screen_blit` | vdi/vdi_textblit.c:849 | `k/vdi_backend.rs::draw_glyph_sized` | m | - | R | - | - |
| `direct_screen_blit16` | vdi/vdi_textblit.c:751 | `k/vdi_backend.rs::draw_glyph_sized` | m | - | R | - | - |
| `dis_cur` | vdi/vdi_mouse.c:127 | `ta/vdi/mouse.rs::dis_cur` | m | m | R | - | - |
| `do_circ` | vdi/vdi_line.c:1322 | `ta/vdi/primitives.rs::do_circ` | m | - | R | - | - |
| `do_clip` | vdi/vdi_raster.c:809 | `ta/vdi/raster.rs::mem_to_screen` +4 | m | m | R | - | - |
| `dont_clip` | vdi/vdi_raster.c:883 | `ta/vdi/raster.rs::rect_from_ptsin` | m | m | R | - | - |
| `draw_arrow` | vdi/vdi_line.c:1503 | `ta/vdi/primitives.rs::draw_arrow` | m | i | R | - | - |
| `draw_line` | vdi/vdi_line.c:1801 | design:ADR-024@docs/MEMORY.md:2158 | d | m | R | - | - |
| `draw_line16` | vdi/vdi_line.c:1623 | `ta/vdi/line.rs::draw_line16` | m | m | R | - | - |
| `draw_rect` | vdi/vdi_line.c:834 | `k/vdi_backend.rs::fill_pattern` | m | m | R | - | - |
| `draw_rect_common` | vdi/vdi_line.c:792 | `k/vdi_backend.rs::fill_pattern` | m | m | R | - | - |
| `draw_rect_setup` | vdi/vdi_line.c:176 | design:ADR-024@docs/MEMORY.md:2158 | d | - | R | - | - |
| `end_pts` | vdi/vdi_fill.c:836 | `ta/vdi/contour.rs::end_pts` | m | m | R | - | - |
| `end_pts16` | vdi/vdi_fill.c:746 | `ta/vdi/contour.rs::end_pts` | m | m | R | - | - |
| `esc_exit` | vdi/vdi_esc.c:365 | `ta/vdi/escape.rs::esc_exit` | m | m | R | - | - |
| `esc_init` | vdi/vdi_esc.c:356 | `ta/vdi/escape.rs::esc_init` | m | m | R | - | - |
| `escfn0` | vdi/vdi_esc.c:49 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn1` | vdi/vdi_esc.c:62 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn10` | vdi/vdi_esc.c:156 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn11` | vdi/vdi_esc.c:172 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn12` | vdi/vdi_esc.c:196 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn13` | vdi/vdi_esc.c:213 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn14` | vdi/vdi_esc.c:222 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn15` | vdi/vdi_esc.c:231 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn16` | vdi/vdi_esc.c:248 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn17` | vdi/vdi_esc.c:260 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn18` | vdi/vdi_esc.c:274 | `ta/vdi/mouse.rs::force_show` | m | m | R | - | - |
| `escfn19` | vdi/vdi_esc.c:284 | `ta/vdi/mouse.rs::v_hide_c` | m | m | R | - | - |
| `escfn2` | vdi/vdi_esc.c:81 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn3` | vdi/vdi_esc.c:93 | `ta/vdi/escape.rs::v_escape` | m | m | R | - | - |
| `escfn4` | vdi/vdi_esc.c:102 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn5` | vdi/vdi_esc.c:111 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn6` | vdi/vdi_esc.c:120 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn7` | vdi/vdi_esc.c:129 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn8` | vdi/vdi_esc.c:138 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `escfn9` | vdi/vdi_esc.c:147 | `ta/vdi/escape.rs::v_escape` | m | - | R | - | - |
| `fast_bit_blt` | vdi/vdi_blit.S:794 | `ta/vdi/raster.rs::planar_mem_to_mem` +2 | m | m | R | - | - |
| `fivebits2vdi` | vdi/vdi_col.c:804 | `ta/vdi/vwk.rs::fivebits2vdi` | m | - | R | - | - |
| `gchc_key` | vdi/vdi_input.c:199 | `ta/vdi/input.rs::v_choice` | m | m | R | - | - |
| `gchr_key` | vdi/vdi_input.c:215 | `ta/vdi/input.rs::try_read_key_word` | m | m | R | - | - |
| `gdp_curve` | vdi/vdi_gdp.c:371 | `ta/vdi/gdp.rs::curve` | m | m | R | - | - |
| `gdp_justified` | vdi/vdi_text.c:1177 | `ta/vdi/text.rs::v_justified` | m | m | R | - | - |
| `gdp_rbox` | vdi/vdi_gdp.c:229 | `ta/vdi/gdp.rs::rbox` | m | m | R | - | - |
| `get_color` | vdi/vdi_fill.c:637 | design:ADR-024@docs/MEMORY.md:2158 | d | - | R | - | - |
| `get_seed` | vdi/vdi_fill.c:888 | `ta/vdi/contour.rs::get_seed` | m | m | R | - | - |
| `get_start_addr` | vdi/vdi_misc.c:166 | design:ADR-024@docs/MEMORY.md:2158 | d | m | R | - | - |
| `get_start_addr16` | vdi/vdi_inline.h:14 | `k/vdi_backend.rs::get_device_pixel` +4 | m | m | R | - | - |
| `get_tt_hyper_mono` | vdi/vdi_col.c:372 | design:hardware:video-shifter | d | - | R | - | - |
| `get_vwk_by_handle` | vdi/vdi_control.c:178 | `ta/vdi/vwk.rs::resolve` | m | i | - | - | - |
| `gloc_key` | vdi/vdi_mouse.c:195 | `ta/vdi/input.rs::gloc_key` +2 | m | m | - | - | - |
| `gshift_s` | vdi/vdi_input.c:185 | `ta/vdi/inquiry.rs::vq_key_s` | m | m | R | - | - |
| `GSX_ENTRY` | vdi/vdi_asm.S:64 | `ta/vdi/mod.rs::dispatch_in` | m | m | - | - | V35/D |
| `hide_cur` | vdi/vdi_mouse.c:157 | `ta/vdi/mouse.rs::v_hide_c` | m | m | R | - | - |
| `horzline` | vdi/vdi_line.c:846 | `ta/vdi/primitives.rs::horzspan` | m | - | R | - | - |
| `hwblit_raster` | vdi/vdi_raster.c:366 | design:hardware:blitter | d | - | R | - | - |
| `hwblit_rect_common` | vdi/vdi_line.c:340 | design:hardware:blitter | d | - | R | - | - |
| `hwblit_rect_nonstd` | vdi/vdi_line.c:284 | design:hardware:blitter | d | - | R | - | - |
| `hwblit_vertical_line` | vdi/vdi_line.c:194 | design:hardware:blitter | d | - | R | - | - |
| `Icos` | vdi/vdi_gdp.c:113 | `ta/vdi/trig.rs::icos` | m | - | R | - | - |
| `init_colors` | vdi/vdi_col.c:708 | `k/vdi_backend.rs::ensure_palette_init` +2 | m | m | R | - | - |
| `init_wk` | vdi/vdi_control.c:269 | `ta/vdi/workstation.rs::init_wk` | m | m | R | - | - |
| `Isin` | vdi/vdi_gdp.c:92 | `ta/vdi/trig.rs::isin` | m | - | R | - | - |
| `make_header` | vdi/vdi_text.c:770 | `ta/vdi/text.rs::scaled_header` | m | m | R | - | - |
| `merge_byte` | vdi/vdi_textblit.c:239 | `ta/vdi/text.rs::build_cell` | m | - | R | - | - |
| `mouse_int` | vdi/vdi_asm.S:231 | `ta/vdi/mouse.rs::feed_relative` +1 | m | m | - | - | - |
| `mov_cur` | vdi/vdi_asm.S:496 | `ta/vdi/mouse.rs::mov_cur` | m | m | - | - | V56/D |
| `no_abort` | vdi/vdi_fill.c:1076 | design:ADR-030@docs/MEMORY.md:2826 | d | m | R | - | - |
| `normal_blit` | vdi/vdi_tblit.S:243 | `k/vdi_backend.rs::draw_glyph_sized` | m | - | R | - | - |
| `ok_for_direct_blit` | vdi/vdi_text.c:227 | UNMAPPED | u | m | R | - | - |
| `outline` | vdi/vdi_textblit.c:266 | `ta/vdi/text.rs::build_cell` | m | m | R | - | - |
| `output_text` | vdi/vdi_text.c:288 | `ta/vdi/text.rs::v_gtext` +8 | m | m | R | - | - |
| `perp_off` | vdi/vdi_line.c:1169 | `ta/vdi/primitives.rs::perp_off` | m | - | R | - | - |
| `pixelread` | vdi/vdi_fill.c:671 | `ta/vdi/contour.rs::pixelread` | m | m | R | - | - |
| `polygon` | vdi/vdi_fill.c:560 | `ta/vdi/primitives.rs::fill_polygon` +2 | m | m | R | - | - |
| `polyline` | vdi/vdi_line.c:1101 | `ta/vdi/primitives.rs::polyline` +1 | m | m | R | - | - |
| `pre_blit` | vdi/vdi_textblit.c:352 | `ta/vdi/text.rs::build_cell` | m | - | R | - | - |
| `quad_xform` | vdi/vdi_line.c:1133 | `ta/vdi/primitives.rs::quad_xform` | m | - | R | - | - |
| `query_tt_color` | vdi/vdi_col.c:403 | design:hardware:video-shifter | d | - | R | - | - |
| `r_fa_attr` | vdi/vdi_line.c:1376 | `ta/vdi/primitives.rs::wideline` | m | - | R | - | - |
| `rotate` | vdi/vdi_textblit.c:442 | `ta/vdi/text.rs::rotate_cell` | m | - | R | - | - |
| `s_fa_attr` | vdi/vdi_line.c:1355 | `ta/vdi/primitives.rs::wideline` | m | m | R | - | - |
| `scale` | vdi/vdi_textblit.c:672 | `ta/vdi/text.rs::scale_glyph` | m | m | R | - | - |
| `scaledown` | vdi/vdi_textblit.c:628 | `ta/vdi/text.rs::scale_row_down` | m | m | R | - | - |
| `scaleup` | vdi/vdi_textblit.c:577 | `ta/vdi/text.rs::scale_row_up` | m | m | R | - | - |
| `screen` | vdi/vdi_main.c:121 | `ta/vdi/mod.rs::screen` | m | m | - | - | - |
| `screen_blit` | vdi/vdi_textblit.c:1191 | `k/vdi_backend.rs::draw_glyph_sized` | m | - | R | - | - |
| `screen_blit16` | vdi/vdi_textblit.c:967 | `k/vdi_backend.rs::draw_glyph_sized` +1 | m | m | R | - | - |
| `search_to_left` | vdi/vdi_fill.c:798 | `ta/vdi/contour.rs::end_pts` | m | - | R | - | - |
| `search_to_left16` | vdi/vdi_fill.c:722 | `ta/vdi/contour.rs::end_pts` | m | - | R | - | - |
| `search_to_right` | vdi/vdi_fill.c:773 | `ta/vdi/contour.rs::end_pts` | m | - | R | - | - |
| `search_to_right16` | vdi/vdi_fill.c:698 | `ta/vdi/contour.rs::end_pts` | m | - | R | - | - |
| `set_color` | vdi/vdi_col.c:526 | `k/vdi_backend.rs::set_color` | m | m | R | - | - |
| `set_color16` | vdi/vdi_col.c:601 | `ta/vdi/vwk.rs::set_color16` | m | m | R | - | - |
| `set_LN_MASK` | vdi/vdi_line.c:2384 | `ta/vdi/primitives.rs::line_mask` | m | m | R | - | - |
| `set_mouse_form` | vdi/vdi_mouse.c:450 | `ta/vdi/mouse.rs::vsc_form` | m | m | R | - | - |
| `set_tt_color` | vdi/vdi_col.c:304 | design:hardware:video-shifter | d | - | R | - | - |
| `setup_info` | vdi/vdi_raster.c:909 | `ta/vdi/raster.rs::effective_planes` +6 | m | m | R | - | - |
| `setup_pattern` | vdi/vdi_raster.c:791 | design:source:docs/api/vdi/raster/_raster.md | d | m | R | - | - |
| `setup_width_height` | vdi/vdi_text.c:668 | `ta/vdi/attributes.rs::setup_width_height` | m | m | R | - | - |
| `shift_and_update` | vdi/vdi_textblit.c:560 | `ta/vdi/text.rs::scale_row_up` +1 | m | - | R | - | - |
| `st_fl_ptr` | vdi/vdi_fill.c:339 | `ta/vdi/fill_patterns.rs::select` | m | m | R | - | - |
| `swblit_rect_common` | vdi/vdi_line.c:575 | design:ADR-024@docs/MEMORY.md:2158 | d | - | R | - | - |
| `swblit_rect_common16` | vdi/vdi_line.c:450 | `k/vdi_backend.rs::fill_pattern` +1 | m | m | R | - | - |
| `swblit_vertical_line16` | vdi/vdi_line.c:2084 | `ta/vdi/line.rs::vertical_line16` | m | m | R | - | - |
| `text_blt` | vdi/vdi_textblit.c:1281 | `ta/vdi/text.rs::make_glyph` +7 | m | m | R | - | - |
| `text_init` | vdi/vdi_text.c:592 | `ta/vdi/workstation.rs::font_ring_fold` +1 | m | m | R | - | - |
| `text_init2` | vdi/vdi_text.c:573 | `ta/vdi/vwk.rs::alloc_handle` +1 | m | m | R | - | - |
| `tick_int` | vdi/vdi_misc.c:78 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | - |
| `timer_exit` | vdi/vdi_misc.c:144 | design:ADR-031@docs/MEMORY.md:3044 | d | m | R | - | - |
| `timer_init` | vdi/vdi_misc.c:121 | `ta/vdi/mouse.rs::timer_init` | m | m | R | - | - |
| `trnsfont` | vdi/vdi_text.c:562 | `ta/vdi/font.rs::trnsfont` | m | m | R | - | - |
| `update_rez_dependent` | vdi/vdi_control.c:193 | `ta/vdi/workstation.rs::init_wk` +1 | m | m | R | - | - |
| `validate_color_index` | vdi/vdi_control.c:224 | `ta/vdi/vwk.rs::validate_color_index` | m | m | R | - | - |
| `vb_draw` | vdi/vdi_mouse.c:668 | `ta/vdi/mouse.rs::vb_draw` | m | m | - | - | - |
| `vdi2fivebits` | vdi/vdi_col.c:588 | `ta/vdi/vwk.rs::vdi2fivebits` | m | - | R | - | - |
| `vdi2st` | vdi/vdi_col.c:204 | design:hardware:video-shifter | d | - | R | - | - |
| `vdi2ste` | vdi/vdi_col.c:225 | design:hardware:video-shifter | d | i | R | - | - |
| `vdi2tt` | vdi/vdi_col.c:254 | design:hardware:video-shifter | d | - | R | - | - |
| `vdi2videl` | vdi/vdi_col.c:457 | design:hardware:videl | d | - | R | - | - |
| `vdi_mousex_handler` | vdi/vdi_mouse.c:525 | `ta/vdi/mouse.rs::mousex_handler` | m | m | - | - | - |
| `vdimouse_exit` | vdi/vdi_mouse.c:622 | `ta/vdi/mouse.rs::vdimouse_exit` | m | m | R | - | - |
| `vdimouse_init` | vdi/vdi_mouse.c:565 | `ta/vdi/mouse.rs::vdimouse_init` +1 | m | m | R | - | - |
| `vditrap` | vdi/vdi_asm.S:34 | `k/arch/aarch64/exceptions.rs::sync_handler` | m | i | - | - | - |
| `vertical_line` | vdi/vdi_line.c:2153 | design:ADR-024@docs/MEMORY.md:2158 | d | - | R | - | - |
| `videl2vdi` | vdi/vdi_col.c:464 | design:hardware:videl | d | m | R | - | - |
| `vq_color16` | vdi/vdi_col.c:813 | `ta/vdi/inquiry.rs::vq_color` | m | m | R | - | - |
| `vr_trnfm16` | vdi/vdi_raster.c:190 | `ta/vdi/raster.rs::vr_trnfm16` | m | m | R | - | - |
| `vro_cpyfm16` | vdi/vdi_raster.c:975 | `ta/vdi/raster.rs::screen_to_mem` +9 | m | m | R | - | - |
| `vrt_cpyfm16` | vdi/vdi_raster.c:1226 | `ta/vdi/raster.rs::mono_to_mem` +3 | m | m | R | - | - |
| `vs_color16` | vdi/vdi_col.c:620 | `ta/vdi/attributes.rs::vs_color` | m | m | - | - | - |
| `Vwk2Attrib` | vdi/vdi_line.c:817 | `ta/vdi/gdp.rs::fill_of` | m | - | R | - | - |
| `wheel_int` | vdi/vdi_asm.S:391 | design:hardware:ikbd-acia | d | m | - | - | - |
| `wideline` | vdi/vdi_line.c:1389 | `ta/vdi/primitives.rs::wideline` | m | m | R | - | - |

### 2.9 Line-A (no NewTOS Line-A, ADR-030, `docs/MEMORY.md:2826`) - 18 rows

| fn | def | NewTOS | c | spec | R | dev | tickets |
|---|---|---|---|---|---|---|---|
| `get_pix` | vdi/vdi_fill.c:1129 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `int_linea` | bios/linea.S:51 | design:ADR-030@docs/MEMORY.md:2826 | d | m | - | - | - |
| `lineA2Attrib` | vdi/vdi_line.c:884 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_0` | bios/linea.S:156 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_blit` | vdi/vdi_raster.c:1521 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_color` | vdi/vdi_line.c:857 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_fill` | vdi/vdi_line.c:975 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_hide_mouse` | vdi/vdi_mouse.c:1133 | `ta/vdi/mouse.rs::v_hide_c` | m | m | R | - | - |
| `linea_hline` | vdi/vdi_line.c:932 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_init` | bios/lineainit.c:50 | design:ADR-030@docs/MEMORY.md:2826 | d | m | R | - | - |
| `linea_line` | vdi/vdi_line.c:2367 | design:ADR-030@docs/MEMORY.md:2826 | d | m | - | - | - |
| `linea_polygon` | vdi/vdi_line.c:951 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_raster` | vdi/vdi_raster.c:1508 | design:ADR-030@docs/MEMORY.md:2826 | d | i | - | - | - |
| `linea_rect` | vdi/vdi_line.c:903 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `linea_show_mouse` | vdi/vdi_mouse.c:1125 | `ta/vdi/mouse.rs::v_show_c` | m | m | R | - | - |
| `linea_transform_mouse` | vdi/vdi_mouse.c:1138 | `ta/vdi/mouse.rs::vsc_form` | m | m | R | - | - |
| `put_pix` | vdi/vdi_fill.c:1158 | design:ADR-030@docs/MEMORY.md:2826 | d | - | - | - | - |
| `set_screen_shift` | bios/lineainit.c:42 | UNMAPPED | u | - | R | - | - |

### 2.10 Cookie-jar code (`util/cookie.c` is `ext`: not a node directory) - 10 rows

| fn | def | NewTOS | c | spec | R | dev | tickets |
|---|---|---|---|---|---|---|---|
| `add_cookie_frb` | bios/machine.c:458 | design:hardware:acsi | d | - | - | - | - |
| `cookie_add` | - | ext | e | m | - | - | - |
| `cookie_get` | - | ext | e | - | - | - | - |
| `cookie_init` | - | ext | e | m | - | - | - |
| `fill_cookie_jar` | bios/machine.c:665 | `ta/cookie.rs::seed_defaults` | m | m | - | - | - |
| `get_frb_cookie` | - | ext | e | - | R | - | - |
| `get_idt_cookie` | - | ext | e | m | - | - | - |
| `natfeat_cookie` | bios/natfeat.S:73 | design:hardware:natfeats | d | - | - | - | - |
| `nvdi_cookie_present` | - | ext | e | - | - | - | - |
| `XHNewCookie` | bios/xhdi.c:81 | UNMAPPED | u | - | - | - | - |

### 2.11 bios/ internals (BIOS and XBIOS helpers, drivers, boot) - 664 rows

| fn | def | NewTOS | c | spec | R | dev | tickets |
|---|---|---|---|---|---|---|---|
| `acsi_begin` | bios/acsi.c:361 | design:hardware:acsi | d | - | R | - | - |
| `acsi_capacity` | bios/acsi.c:297 | design:hardware:acsi | d | - | R | - | - |
| `acsi_end` | bios/acsi.c:373 | design:hardware:acsi | d | - | R | - | - |
| `acsi_init` | bios/acsi.c:113 | design:hardware:acsi | d | - | - | - | - |
| `acsi_inquiry` | bios/acsi.c:340 | design:hardware:acsi | d | - | R | - | - |
| `acsi_ioctl` | bios/acsi.c:197 | design:hardware:acsi | d | - | R | - | - |
| `acsi_request_sense` | bios/acsi.c:268 | design:hardware:acsi | d | - | R | - | - |
| `acsi_rw` | bios/acsi.c:134 | design:hardware:acsi | d | - | R | - | - |
| `acsi_testunit` | bios/acsi.c:323 | design:hardware:acsi | d | - | R | - | - |
| `add_partition` | bios/blkdev.c:354 | UNMAPPED | u | m | R | - | - |
| `altram_init` | bios/memory2.c:138 | `ta/bdos/mem/init.rs::init` | m | m | - | - | - |
| `and_icr_reg` | bios/scsi.c:495 | design:hardware:scsi | d | - | R | - | - |
| `and_mode_reg` | bios/scsi.c:519 | design:hardware:scsi | d | - | R | - | - |
| `aranym_machine_detect` | bios/machine.c:511 | design:hardware:natfeats | d | - | - | - | - |
| `ascii_cr` | bios/vt52.c:821 | `ta/vdi/escape.rs::normal_ascii` | m | m | R | - | - |
| `ascii_lf` | bios/vt52.c:831 | `ta/vdi/escape.rs::ascii_lf` | m | m | R | - | - |
| `ascii_out` | bios/conout.c:482 | `ta/vdi/escape.rs::ascii_out` | m | m | R | - | - |
| `ata_identify` | bios/ide.c:1501 | design:hardware:ide | d | - | R | - | - |
| `ata_request_sense` | bios/ide.c:2001 | design:hardware:ide | d | - | - | - | - |
| `atapi_identify` | bios/ide.c:1595 | design:hardware:ide | d | - | R | - | - |
| `atapi_request_sense` | bios/ide.c:2045 | design:hardware:ide | d | - | - | - | - |
| `atapi_rw_start` | bios/ide.c:1859 | design:hardware:ide | d | - | - | - | - |
| `atapi_send_cdb` | bios/ide.c:1874 | design:hardware:ide | d | - | - | - | - |
| `atari_get_current_mode_info` | bios/screen.c:799 | design:hardware:video-shifter | d | - | R | - | - |
| `atari_getrez` | bios/screen.c:948 | design:hardware:video-shifter | d | m | R | - | - |
| `atari_partition` | bios/disk.c:847 | UNMAPPED | u | - | R | - | - |
| `atari_physbase` | bios/screen.c:917 | design:hardware:video-shifter | d | m | R | - | - |
| `atari_setcolor` | bios/screen.c:1015 | design:hardware:video-shifter | d | m | R | - | - |
| `atari_setphys` | bios/screen.c:939 | design:hardware:video-shifter | d | m | R | - | - |
| `atari_setrez` | bios/screen.c:990 | design:ADR-017@docs/MEMORY.md:824 | d | m | R | - | - |
| `autoexec` | bios/bios.c:697 | `k/launch.rs::run_auto_folder` | m | m | - | - | - |
| `balloc_stram` | bios/biosmem.c:93 | UNMAPPED | u | m | - | - | - |
| `bcd2int` | bios/clock.c:45 | design:hardware:rtc | d | - | R | - | - |
| `bconin0` | bios/parport.c:123 | design:hardware:parallel-port | d | m | R | - | - |
| `bconin1` | bios/serport.c:260 | `k/console.rs::bconin` +1 | m | m | R | - | - |
| `bconin2` | bios/ikbd.c:250 | `ta/bios/chardev.rs::bconin` +2 | m | m | R | - | - |
| `bconin3` | bios/midi.c:35 | `ta/bios/chardev.rs::midi_read` +1 | m | m | R | - | - |
| `bconin_iorec` | bios/serport.c:202 | `k/console.rs::aux_read_blocking` +1 | m | m | R | - | - |
| `bconinA` | bios/serport.c:504 | design:hardware:scc | d | - | - | - | - |
| `bconinB` | bios/serport.c:556 | `k/console.rs::bconin` +1 | m | - | - | - | - |
| `bconinTT` | bios/serport.c:405 | design:hardware:tt-mfp | d | - | - | - | - |
| `bconout0` | bios/parport.c:144 | design:hardware:parallel-port | d | m | R | - | - |
| `bconout1` | bios/serport.c:289 | `k/console.rs::bconout` +1 | m | m | R | - | - |
| `bconout2` | bios/chardev.c:91 | `ta/bios/chardev.rs::bconout` +1 | m | m | R | - | - |
| `bconout3` | bios/midi.c:84 | `ta/xbios/system.rs::midiws` +1 | m | m | R | - | - |
| `bconout4` | bios/ikbd.c:933 | `ta/bios/chardev.rs::bconout` +1 | m | m | R | - | - |
| `bconout5` | bios/chardev.c:98 | `ta/vdi/escape.rs::rawout` | m | m | R | - | - |
| `bconoutA` | bios/serport.c:517 | design:hardware:scc | d | - | - | - | - |
| `bconoutB` | bios/serport.c:585 | `k/console.rs::bconout` +1 | m | - | - | - | - |
| `bconoutTT` | bios/serport.c:418 | design:hardware:tt-mfp | d | m | - | - | - |
| `bconstat1` | bios/serport.c:255 | `k/console.rs::bconstat` | m | m | R | - | - |
| `bconstat2` | bios/ikbd.c:235 | `ta/bios/chardev.rs::bconstat` +1 | m | m | R | - | - |
| `bconstat3` | bios/midi.c:23 | `ta/bios/chardev.rs::bconstat` | m | m | R | - | - |
| `bconstat_iorec` | bios/serport.c:191 | `ta/xbios/iorec.rs::serial_pending` | m | m | R | - | - |
| `bconstatA` | bios/serport.c:499 | design:hardware:scc | d | - | - | - | - |
| `bconstatB` | bios/serport.c:551 | `k/console.rs::bconstat` | m | - | - | - | - |
| `bconstatTT` | bios/serport.c:400 | design:hardware:tt-mfp | d | - | - | - | - |
| `bcostat0` | bios/parport.c:129 | design:hardware:parallel-port | d | m | R | - | - |
| `bcostat1` | bios/serport.c:271 | `k/console.rs::bcostat` +1 | m | m | R | - | - |
| `bcostat2` | bios/chardev.c:112 | `ta/bios/chardev.rs::bcostat` | m | m | R | - | - |
| `bcostat3` | bios/midi.c:66 | `ta/bios/chardev.rs::bcostat` | m | m | R | - | - |
| `bcostat4` | bios/ikbd.c:916 | `ta/bios/chardev.rs::bcostat` | m | m | R | - | - |
| `bcostatA` | bios/serport.c:509 | design:hardware:scc | d | - | - | - | - |
| `bcostatB` | bios/serport.c:566 | `k/console.rs::bcostat` +1 | m | - | - | - | - |
| `bcostatTT` | bios/serport.c:410 | design:hardware:tt-mfp | d | - | - | - | - |
| `bell` | bios/sound.c:251 | `ta/vdi/escape.rs::bell` | m | m | R | - | - |
| `bios_init` | bios/bios.c:212 | UNMAPPED | u | m | - | - | - |
| `biosmain` | bios/bios.c:781 | UNMAPPED | u | m | - | - | - |
| `biostrap` | bios/vectors.S:1268 | `ta/bios/mod.rs::dispatch` | m | - | - | - | - |
| `blank_out` | bios/conout.c:598 | `ta/vdi/escape.rs::blank_out` | m | m | R | - | - |
| `blank_out16` | bios/conout.c:542 | `ta/vdi/escape.rs::blank_out` | m | m | R | - | - |
| `blink` | bios/vt52.c:849 | `ta/vdi/escape.rs::blink` | m | m | - | - | - |
| `blkdev_avail` | bios/blkdev.c:846 | UNMAPPED | u | m | - | - | - |
| `blkdev_boot` | bios/blkdev.c:223 | UNMAPPED | u | m | - | - | - |
| `blkdev_drvmap` | bios/blkdev.c:833 | `ta/bios/mod.rs::drvmap` | m | m | R | - | - |
| `blkdev_getbpb` | bios/blkdev.c:569 | `ta/bios/bpb.rs::getbpb` +5 | m | m | R | - | - |
| `blkdev_hdv_boot` | bios/blkdev.c:258 | UNMAPPED | u | - | - | - | - |
| `blkdev_hdv_init` | bios/blkdev.c:171 | `ta/bios/mod.rs::mount` | m | - | - | - | - |
| `blkdev_init` | bios/blkdev.c:88 | UNMAPPED | u | m | - | - | - |
| `blkdev_mediach` | bios/blkdev.c:783 | `ta/bios/mod.rs::media_change` | m | m | R | - | - |
| `blkdev_rwabs` | bios/blkdev.c:406 | `ta/bios/mod.rs::rwabs` +5 | m | m | R | - | - |
| `bmem_init` | bios/biosmem.c:51 | UNMAPPED | u | m | - | - | - |
| `bootcheck` | bios/blkdev.c:273 | UNMAPPED | u | - | - | - | - |
| `bootstrap` | bios/bios.c:584 | design:hardware:natfeats | d | m | - | - | - |
| `build_rw_command` | bios/scsi.c:1402 | design:hardware:scsi | d | - | R | - | - |
| `bus_init` | bios/blkdev.c:201 | UNMAPPED | u | - | - | - | - |
| `byteswap` | bios/disk.c:571 | design:hardware:ide | d | - | R | - | - |
| `cache_exists` | bios/processor.S:578 | design:hardware:m68k-cpu | d | m | - | - | - |
| `calc_vram_size` | bios/screen.c:744 | UNMAPPED | u | - | - | - | - |
| `calculate_repeat` | bios/acsi.c:435 | design:hardware:acsi | d | - | R | - | - |
| `calibrate_delay` | bios/delay.c:99 | UNMAPPED | u | - | - | - | - |
| `call_etv_critic` | bios/vectors.S:807 | design:ADR-031@docs/MEMORY.md:3044 | d | m | R | - | - |
| `call_mousevec` | bios/aciavecs.S:538 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | - |
| `can_shutdown` | bios/bios.c:754 | UNMAPPED | u | m | - | - | - |
| `cell_addr` | bios/conout.c:89 | UNMAPPED | u | - | R | - | - |
| `cell_xfer` | bios/conout.c:202 | `ta/vdi/escape.rs::cell_xfer` | m | m | R | - | - |
| `cell_xfer16` | bios/conout.c:137 | `ta/vdi/escape.rs::cell_xfer` | m | m | R | - | - |
| `char_addr` | bios/conout.c:54 | UNMAPPED | u | m | R | - | - |
| `char_dummy` | bios/chardev.c:50 | `ta/bios/chardev.rs::bconstat` | m | m | R | - | - |
| `chardev_init` | bios/chardev.c:63 | `ta/bdos/process/loader.rs::populate_sysvars` | m | m | - | - | - |
| `charout_dummy` | bios/chardev.c:55 | `ta/bios/chardev.rs::bconout` | m | m | R | - | - |
| `check_busses` | bios/machine.c:523 | design:hardware:m68k-cpu | d | - | - | - | - |
| `check_for_no_partitions` | bios/disk.c:516 | UNMAPPED | u | - | R | - | - |
| `check_for_vga` | bios/nova.c:190 | design:hardware:nova | d | - | - | - | - |
| `check_interface_magic` | bios/ide.c:428 | design:hardware:ide | d | - | - | - | - |
| `check_lun_and_zeroes` | bios/ide.c:1651 | design:hardware:ide | d | - | - | - | - |
| `check_moderez` | bios/screen.c:407 | design:ADR-017@docs/MEMORY.md:824 | d | m | - | - | - |
| `check_read_byte` | bios/vectors.S:1389 | design:hardware:m68k-cpu | d | m | - | - | - |
| `cleanup_scsi` | bios/scsi.c:806 | design:hardware:scsi | d | - | R | - | - |
| `cleanup_tt_dma` | bios/scsi.c:779 | design:hardware:scsi | d | - | R | - | - |
| `clear_and_home` | bios/vt52.c:338 | `ta/vdi/escape.rs::clear_and_home` | m | m | R | - | - |
| `clear_multiple_mode` | bios/ide.c:1458 | design:hardware:ide | d | - | R | - | - |
| `clock_init` | bios/clock.c:1138 | design:hardware:rtc | d | m | - | - | - |
| `clockvec` | bios/clock.c:969 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | XBIOS03/A |
| `cold_reset` | bios/panicasm.S:114 | `ta/host.rs::system_restart` +1 | m | m | - | - | - |
| `compute_cksum` | bios/blkdev.c:69 | `ta/xbios/floppy.rs::compute_cksum` | m | m | R | - | - |
| `compute_sum` | bios/nvram.c:98 | design:hardware:nvram | d | - | R | - | - |
| `convert2ste` | bios/videl.c:763 | design:hardware:videl | d | - | R | - | - |
| `convert_drive_and_side` | bios/floppy.c:1425 | design:hardware:floppy | d | - | R | - | - |
| `convert_lod` | bios/dsp.c:879 | design:hardware:dsp56001 | d | - | - | - | - |
| `convert_scancode` | bios/ikbd.c:558 | UNMAPPED | u | m | - | - | - |
| `count_vbls` | bios/nova.c:668 | design:hardware:nova | d | - | - | - | - |
| `cprint_asctime` | bios/initinfo.c:173 | UNMAPPED | u | - | - | - | - |
| `cprint_devices` | bios/initinfo.c:220 | UNMAPPED | u | - | - | - | - |
| `cprintf` | bios/kprint.c:67 | UNMAPPED | u | m | R | - | - |
| `cprintf_bytesize` | bios/initinfo.c:245 | `k/initinfo.rs::size_str` | m | - | - | - | - |
| `cprintf_outc` | bios/kprint.c:53 | UNMAPPED | u | - | - | - | - |
| `cputc` | bios/vt52.c:142 | `ta/vdi/escape.rs::cputc` | m | m | R | - | - |
| `cursor_down` | bios/vt52.c:383 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `cursor_down_impl` | bios/vt52.c:373 | UNMAPPED | u | m | R | - | - |
| `cursor_home` | bios/vt52.c:433 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `cursor_left` | bios/vt52.c:420 | `ta/vdi/escape.rs::cursor_left` | m | m | R | - | - |
| `cursor_left_impl` | bios/vt52.c:410 | `ta/vdi/escape.rs::cursor_left` | m | m | R | - | - |
| `cursor_off` | bios/vt52.c:664 | `ta/vdi/escape.rs::cursor_off` | m | m | R | - | - |
| `cursor_on` | bios/vt52.c:630 | `ta/vdi/escape.rs::cursor_on` | m | m | R | - | - |
| `cursor_on_cnt` | bios/vt52.c:649 | `ta/vdi/escape.rs::cursor_on_cnt` | m | m | R | - | - |
| `cursor_right` | bios/vt52.c:396 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `cursor_up` | bios/vt52.c:359 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `decode_error` | bios/floppy.c:1566 | design:hardware:floppy | d | - | R | - | - |
| `decode_scsi_status` | bios/scsi.c:1309 | design:hardware:scsi | d | - | R | - | - |
| `decode_sense` | bios/scsi.c:1449 | design:hardware:scsi | d | - | R | - | - |
| `default_etv_critic` | bios/vectors.S:823 | design:ADR-031@docs/MEMORY.md:3044 | d | m | R | - | - |
| `delete_line` | bios/vt52.c:577 | `ta/vdi/escape.rs::delete_line` | m | m | R | - | - |
| `deselect` | bios/floppy.c:1447 | design:hardware:floppy | d | m | - | - | - |
| `detect_32bit_address_bus` | bios/memory.S:643 | design:hardware:m68k-cpu | d | - | - | - | - |
| `detect_acsi` | bios/acsi.c:103 | design:hardware:acsi | d | - | - | - | - |
| `detect_akp` | bios/country.c:140 | UNMAPPED | u | m | - | - | - |
| `detect_blitter` | bios/machine.c:303 | design:hardware:blitter | d | - | - | - | - |
| `detect_cpu` | bios/processor.S:63 | design:hardware:m68k-cpu | d | - | - | - | - |
| `detect_dip_switches` | bios/machine.c:330 | design:hardware:dip-switches | d | - | - | - | - |
| `detect_dmasound` | bios/dmasound.c:128 | design:hardware:dma-sound | d | m | - | - | - |
| `detect_dsp` | bios/dsp.c:187 | design:hardware:dsp56001 | d | - | - | - | - |
| `detect_fpu` | bios/processor.S:161 | design:hardware:m68k-cpu | d | - | - | - | - |
| `detect_icdrtc` | bios/clock.c:149 | design:hardware:rtc | d | - | - | - | - |
| `detect_ide` | bios/ide.c:550 | design:hardware:ide | d | - | - | - | - |
| `detect_idt` | bios/country.c:165 | `ta/cookie.rs::seed_defaults` | m | - | - | - | - |
| `detect_mach` | bios/nova.c:265 | design:hardware:nova | d | - | - | - | - |
| `detect_magnum` | bios/machine.c:259 | design:hardware:magnum-ram | d | - | - | - | - |
| `detect_megartc` | bios/clock.c:641 | design:hardware:rtc | d | - | - | - | - |
| `detect_modectl` | bios/machine.c:87 | design:hardware:video-shifter | d | - | - | - | - |
| `detect_monitor_change` | bios/screen.c:1238 | design:hardware:video-shifter | d | - | - | - | - |
| `detect_monster` | bios/machine.c:227 | design:hardware:monster | d | - | - | - | - |
| `detect_monster_rtc` | bios/clock.c:512 | design:hardware:rtc | d | - | - | - | - |
| `detect_native_features` | bios/natfeat.S:39 | design:hardware:natfeats | d | - | - | - | - |
| `detect_nova` | bios/nova.c:213 | design:hardware:nova | d | - | - | - | - |
| `detect_nvram` | bios/nvram.c:49 | design:hardware:nvram | d | - | - | - | - |
| `detect_scc` | bios/machine.c:189 | design:hardware:scc | d | - | - | - | - |
| `detect_scsi` | bios/scsi.c:249 | design:hardware:scsi | d | - | - | - | - |
| `detect_tt_mfp` | bios/machine.c:171 | design:hardware:tt-mfp | d | - | - | - | - |
| `detect_ttram_size` | bios/memory2.c:36 | design:hardware:tt-ram | d | - | - | - | - |
| `detect_video` | bios/machine.c:101 | design:hardware:video-shifter | d | i | - | - | - |
| `detect_vme` | bios/machine.c:204 | design:hardware:vme | d | i | - | - | - |
| `determine_regc0` | bios/videl.c:473 | design:hardware:videl | d | - | R | - | - |
| `determine_vctl` | bios/videl.c:434 | design:hardware:videl | d | - | R | - | - |
| `determine_width` | bios/videl.c:417 | design:hardware:videl | d | - | R | - | - |
| `devconnect_falcon` | bios/dmasound.c:590 | design:hardware:dma-sound | d | m | - | - | - |
| `devconnect_ste` | bios/dmasound.c:691 | design:hardware:dma-sound | d | m | - | - | - |
| `disable_mfp_interrupt` | bios/mfp.c:39 | design:hardware:mfp68901 | d | - | - | - | - |
| `disk_get_capacity` | bios/disk.c:1112 | UNMAPPED | u | - | R | - | - |
| `disk_init_all` | bios/disk.c:304 | UNMAPPED | u | m | - | - | - |
| `disk_init_one` | bios/disk.c:181 | UNMAPPED | u | - | R | - | - |
| `disk_inquire` | bios/disk.c:1099 | UNMAPPED | u | - | - | - | - |
| `disk_mediach` | bios/disk.c:373 | UNMAPPED | u | m | R | - | - |
| `disk_rescan` | bios/disk.c:434 | UNMAPPED | u | m | R | - | - |
| `disk_rw` | bios/disk.c:1177 | UNMAPPED | u | m | R | - | - |
| `disk_try_dmaboot` | bios/disk.c:151 | design:hardware:acsi | d | m | - | - | - |
| `disk_valid_major` | bios/disk.c:1088 | UNMAPPED | u | - | - | - | - |
| `display_inverse` | bios/initinfo.c:134 | `k/initinfo.rs::display_inverse` | m | - | - | - | - |
| `display_message` | bios/initinfo.c:123 | UNMAPPED | u | - | - | - | - |
| `display_startup_msg` | bios/initinfo.c:475 | UNMAPPED | u | - | - | - | - |
| `dma_send_byte` | bios/acsi.c:606 | design:hardware:acsi | d | - | R | - | - |
| `dmaboot` | bios/disk.c:123 | design:hardware:acsi | d | - | - | - | - |
| `dmasound_init` | bios/dmasound.c:200 | design:hardware:dma-sound | d | m | - | - | - |
| `do_acsi_rw` | bios/acsi.c:392 | design:hardware:acsi | d | - | R | - | - |
| `do_backspace` | bios/vt52.c:224 | `ta/vdi/escape.rs::normal_ascii` | m | m | R | - | - |
| `do_bell` | bios/sound.c:256 | `ta/vdi/escape.rs::bell` +1 | m | m | R | - | - |
| `do_cnt_esce` | bios/vt52.c:612 | `ta/vdi/escape.rs::do_cnt_esce` | m | m | R | - | - |
| `do_key_repeat` | bios/ikbd.c:451 | `k/ikbd.rs::poll_repeat` | m | m | - | - | - |
| `do_keyclick` | bios/sound.c:263 | design:hardware:ym2149 | d | m | - | - | - |
| `do_scsi_rw` | bios/scsi.c:1334 | design:hardware:scsi | d | - | R | - | - |
| `do_tab` | bios/vt52.c:233 | `ta/vdi/escape.rs::normal_ascii` | m | m | R | - | - |
| `dopanic` | bios/kprint.c:302 | `ta/bios/crash.rs::write_report` +1 | m | m | R | - | - |
| `dsp_init` | bios/dsp.c:193 | design:hardware:dsp56001 | d | - | - | - | - |
| `dsp_inout_asm` | bios/dsp2.S:26 | design:hardware:dsp56001 | d | - | - | - | - |
| `dsp_inout_handler` | bios/dsp.c:374 | design:hardware:dsp56001 | d | - | - | - | - |
| `dsp_io_asm` | bios/dsp2.S:35 | design:hardware:dsp56001 | d | - | - | - | - |
| `dsp_io_handler` | bios/dsp.c:457 | design:hardware:dsp56001 | d | - | - | - | - |
| `dsp_sv_asm` | bios/dsp2.S:44 | design:hardware:dsp56001 | d | - | - | - | - |
| `dsp_sv_handler` | bios/dsp.c:1314 | design:hardware:dsp56001 | d | - | - | - | - |
| `dummy_seek` | bios/floppy.c:1469 | design:hardware:floppy | d | - | R | - | - |
| `enable_mfp_interrupt` | bios/mfp.c:59 | design:hardware:mfp68901 | d | m | - | - | - |
| `erase_from_bol` | bios/vt52.c:784 | `ta/vdi/escape.rs::erase_from_bol` | m | m | R | - | - |
| `erase_from_bol_impl` | bios/vt52.c:752 | `ta/vdi/escape.rs::erase_from_bol` | m | m | R | - | - |
| `erase_from_home` | bios/vt52.c:592 | `ta/vdi/escape.rs::erase_from_home` | m | m | R | - | - |
| `erase_line` | bios/vt52.c:733 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `erase_to_eol` | bios/vt52.c:506 | `ta/vdi/escape.rs::erase_to_eol` | m | m | R | - | - |
| `erase_to_eol_impl` | bios/vt52.c:471 | `ta/vdi/escape.rs::erase_to_eol` | m | m | R | - | - |
| `erase_to_eos` | bios/vt52.c:451 | `ta/vdi/escape.rs::erase_to_eos` | m | m | R | - | - |
| `esc_ch1` | bios/vt52.c:242 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `extract_date` | bios/clock.c:74 | design:hardware:rtc | d | m | R | - | - |
| `extract_time` | bios/clock.c:84 | design:hardware:rtc | d | m | R | - | - |
| `falcon_dmasound_init` | bios/dmasound.c:182 | design:hardware:dma-sound | d | m | - | - | - |
| `fdc_start_dma_read` | bios/floppy.c:1617 | design:hardware:floppy | d | - | R | - | - |
| `fdc_start_dma_write` | bios/floppy.c:1626 | design:hardware:floppy | d | - | R | - | - |
| `filesize` | bios/dsp.c:903 | design:hardware:dsp56001 | d | - | - | - | - |
| `fixup_ste_palette` | bios/screen.c:76 | design:hardware:video-shifter | d | - | - | - | - |
| `flop_add_drive` | bios/floppy.c:355 | design:hardware:floppy | d | m | - | - | - |
| `flop_boot_read` | bios/floppy.c:350 | design:hardware:floppy | d | - | - | - | - |
| `flop_checksum` | bios/floppy.c:442 | design:hardware:floppy | d | m | R | - | - |
| `flop_detect_drive` | bios/floppy.c:390 | design:hardware:floppy | d | - | - | - | - |
| `flop_hdv_init` | bios/floppy.c:305 | design:hardware:floppy | d | m | - | - | - |
| `flop_init` | bios/floppy.c:290 | design:hardware:floppy | d | - | - | - | - |
| `flop_mediach` | bios/floppy.c:454 | design:hardware:floppy | d | m | R | - | - |
| `flopcmd` | bios/floppy.c:1532 | design:hardware:floppy | d | - | R | - | - |
| `flopio` | bios/floppy.c:1069 | design:hardware:floppy | d | - | R | - | - |
| `flopio_ver` | bios/floppy.c:1194 | design:hardware:floppy | d | - | R | - | - |
| `floplock` | bios/floppy.c:1287 | design:hardware:floppy | d | - | R | - | - |
| `floppy_rw` | bios/floppy.c:555 | design:hardware:floppy | d | m | R | - | - |
| `flopunlk` | bios/floppy.c:1310 | design:hardware:floppy | d | - | R | - | - |
| `flopvbl` | bios/floppy.c:1337 | design:hardware:floppy | d | - | - | - | - |
| `flopwtrack` | bios/floppy.c:1217 | design:hardware:floppy | d | - | - | - | - |
| `flush_data_cache` | bios/processor.S:448 | UNMAPPED | u | - | R | - | - |
| `font_init` | bios/font.c:34 | UNMAPPED | u | - | - | - | - |
| `font_set_default` | bios/font.c:72 | `ta/vdi/font.rs::font_set_default` +1 | m | m | R | - | - |
| `fscsi_and` | bios/scsi.c:443 | design:hardware:scsi | d | - | R | - | - |
| `fscsi_get` | bios/scsi.c:437 | design:hardware:scsi | d | - | R | - | - |
| `fscsi_or` | bios/scsi.c:453 | design:hardware:scsi | d | - | R | - | - |
| `fscsi_put` | bios/scsi.c:431 | design:hardware:scsi | d | - | R | - | - |
| `get_bg_col` | bios/vt52.c:309 | `ta/vdi/escape.rs::cputc` | m | m | R | - | - |
| `get_bus_info` | bios/scsidriv.c:214 | UNMAPPED | u | - | - | - | - |
| `get_bus_maxlen` | bios/scsidriv.c:172 | UNMAPPED | u | - | - | - | - |
| `get_bus_status_reg` | bios/scsi.c:543 | design:hardware:scsi | d | - | R | - | - |
| `get_column` | bios/vt52.c:271 | `ta/vdi/escape.rs::cputc` | m | m | R | - | - |
| `get_country_record` | bios/country.c:75 | `k/keymap.rs::select_builtin` | m | m | - | - | - |
| `get_current_country_input` | bios/country.c:96 | UNMAPPED | u | m | - | - | - |
| `get_data_reg` | bios/scsi.c:463 | design:hardware:scsi | d | - | R | - | - |
| `get_default_country` | bios/country.c:60 | UNMAPPED | u | - | - | - | - |
| `get_default_palmode` | bios/screen.c:465 | design:hardware:video-shifter | d | - | - | - | - |
| `get_disk_size` | bios/ide.c:1683 | design:hardware:ide | d | - | - | - | - |
| `get_dma_status` | bios/floppy.c:1587 | design:hardware:floppy | d | - | R | - | - |
| `get_dma_status_reg` | bios/scsi.c:551 | design:hardware:scsi | d | - | R | - | - |
| `get_fdc_reg` | bios/floppy.c:1597 | design:hardware:floppy | d | - | R | - | - |
| `get_fg_col` | bios/vt52.c:292 | `ta/vdi/escape.rs::cputc` | m | m | R | - | - |
| `get_fonts` | bios/country.c:208 | UNMAPPED | u | - | - | - | - |
| `get_handle_entry` | bios/scsidriv.c:253 | UNMAPPED | u | - | - | - | - |
| `get_icr_reg` | bios/scsi.c:479 | design:hardware:scsi | d | - | R | - | - |
| `get_idxreg` | bios/nova.c:151 | design:hardware:nova | d | - | - | - | - |
| `get_iorecbuf` | bios/serport.c:171 | `ta/xbios/iorec.rs::pop_serial` +1 | m | m | R | - | - |
| `get_kbd_index` | bios/country.c:101 | `k/keymap.rs::init_from_disk` | m | m | - | - | - |
| `get_keytbl` | bios/country.c:195 | `k/keymap.rs::restore_default` | m | m | - | - | - |
| `get_monitor_type` | bios/screen.c:703 | design:hardware:video-shifter | d | i | R | - | - |
| `get_next_phase` | bios/scsi.c:570 | design:hardware:scsi | d | - | R | - | - |
| `get_novamembase` | bios/nova.c:805 | design:hardware:nova | d | - | R | - | - |
| `get_nvram_rtc` | bios/nvram.c:59 | design:hardware:nvram | d | - | R | - | - |
| `get_palette` | bios/screen.c:831 | `ta/vdi/workstation.rs::palette_precision` | m | m | R | - | - |
| `get_pixel_size` | bios/screen.c:895 | UNMAPPED | u | m | R | - | - |
| `get_row` | bios/vt52.c:261 | `ta/vdi/escape.rs::cputc` | m | m | R | - | - |
| `get_shift` | bios/blkdev.c:550 | UNMAPPED | u | - | R | - | - |
| `get_start_count` | bios/ide.c:845 | design:hardware:ide | d | - | - | - | - |
| `get_std_pixel_size` | bios/screen.c:872 | `ta/vdi/workstation.rs::get_std_pixel_size` | m | m | R | - | - |
| `get_sum` | bios/nvram.c:117 | design:hardware:nvram | d | - | R | - | - |
| `get_videl_bpp` | bios/videl.c:320 | design:hardware:videl | d | - | R | - | - |
| `get_videl_height` | bios/videl.c:357 | design:hardware:videl | d | - | R | - | - |
| `get_videl_mode` | bios/videl.c:1049 | design:hardware:videl | d | - | - | - | - |
| `get_videl_width` | bios/videl.c:352 | design:hardware:videl | d | - | R | - | - |
| `get_xhdi_nfid` | bios/natfeats.c:87 | design:hardware:natfeats | d | - | R | - | - |
| `getbpb_allowed` | bios/blkdev.c:329 | UNMAPPED | u | - | R | - | - |
| `getiword` | bios/blkdev.c:59 | UNMAPPED | u | m | R | - | - |
| `guess_machine_name` | bios/machine.c:800 | `k/initinfo.rs::machine_name` | m | - | - | - | - |
| `halt` | bios/panicasm.S:79 | `k/arch/aarch64/mod.rs::halt` | m | m | R | - | - |
| `handle_ata_device` | bios/ide.c:1693 | design:hardware:ide | d | - | - | - | - |
| `handle_atapi_device` | bios/ide.c:1908 | design:hardware:ide | d | - | - | - | - |
| `handle_command` | bios/scsi.c:1040 | design:hardware:scsi | d | - | R | - | - |
| `handle_DATA` | bios/dsp.c:815 | design:hardware:dsp56001 | d | - | - | - | - |
| `handle_data_in` | bios/scsi.c:1009 | design:hardware:scsi | d | - | R | - | - |
| `handle_data_out` | bios/scsi.c:982 | design:hardware:scsi | d | - | R | - | - |
| `handle_mouse_mode` | bios/ikbd.c:295 | UNMAPPED | u | m | - | - | - |
| `handle_msg_in` | bios/scsi.c:1128 | design:hardware:scsi | d | - | R | - | - |
| `handle_msg_out` | bios/scsi.c:1095 | design:hardware:scsi | d | - | R | - | - |
| `handle_status` | bios/scsi.c:1056 | design:hardware:scsi | d | - | R | - | - |
| `handshake` | bios/scsi.c:839 | design:hardware:scsi | d | m | R | - | - |
| `has_natfeats` | bios/natfeats.c:54 | design:hardware:natfeats | d | - | R | - | - |
| `has_nf_shutdown` | bios/natfeats.c:103 | design:hardware:natfeats | d | i | - | - | - |
| `hd_boot_read` | bios/blkdev.c:305 | UNMAPPED | u | - | - | - | - |
| `hdc_start_dma` | bios/acsi.c:622 | design:hardware:acsi | d | - | R | - | - |
| `hex` | bios/dsp.c:745 | design:hardware:dsp56001 | d | m | - | - | - |
| `i2c_read` | bios/clock.c:430 | design:hardware:rtc | d | - | R | - | - |
| `i2c_start` | bios/clock.c:389 | design:hardware:rtc | d | - | R | - | - |
| `i2c_stop` | bios/clock.c:400 | design:hardware:rtc | d | - | R | - | - |
| `i2c_write` | bios/clock.c:411 | design:hardware:rtc | d | - | R | - | - |
| `icd_clock_begin` | bios/clock.c:166 | design:hardware:rtc | d | - | R | - | - |
| `icd_clock_end` | bios/clock.c:182 | design:hardware:rtc | d | - | R | - | - |
| `icdgetdate` | bios/clock.c:320 | design:hardware:rtc | d | - | R | - | - |
| `icdgetdt` | bios/clock.c:349 | design:hardware:rtc | d | m | R | - | - |
| `icdgetregs` | bios/clock.c:193 | design:hardware:rtc | d | - | R | - | - |
| `icdgettime` | bios/clock.c:274 | design:hardware:rtc | d | - | R | - | - |
| `icdsetdate` | bios/clock.c:292 | design:hardware:rtc | d | - | R | - | - |
| `icdsetdt` | bios/clock.c:358 | design:hardware:rtc | d | m | R | - | - |
| `icdsetregs` | bios/clock.c:225 | design:hardware:rtc | d | - | R | - | - |
| `icdsettime` | bios/clock.c:254 | design:hardware:rtc | d | - | R | - | - |
| `ide_decode_type` | bios/ide.c:712 | design:hardware:ide | d | - | - | - | - |
| `ide_detect_devices` | bios/ide.c:730 | design:hardware:ide | d | - | - | - | - |
| `ide_device_type` | bios/ide.c:658 | design:hardware:ide | d | - | R | - | - |
| `ide_get_data` | bios/ide.c:999 | design:hardware:ide | d | - | R | - | - |
| `ide_get_data_32` | bios/ide.c:964 | design:hardware:ide | d | - | R | - | - |
| `ide_init` | bios/ide.c:605 | design:hardware:ide | d | - | - | - | - |
| `ide_interface_exists` | bios/ide.c:487 | design:hardware:ide | d | - | - | - | - |
| `ide_interface_is_ghost` | bios/ide.c:442 | design:hardware:ide | d | - | - | - | - |
| `ide_ioctl` | bios/ide.c:1526 | design:hardware:ide | d | - | R | - | - |
| `ide_nodata` | bios/ide.c:942 | design:hardware:ide | d | - | - | - | - |
| `ide_put_data` | bios/ide.c:1183 | design:hardware:ide | d | - | R | - | - |
| `ide_read` | bios/ide.c:1085 | design:hardware:ide | d | - | R | - | - |
| `ide_request_sense` | bios/ide.c:2067 | design:hardware:ide | d | - | - | - | - |
| `ide_reset` | bios/ide.c:695 | design:hardware:ide | d | - | - | - | - |
| `ide_rw` | bios/ide.c:1343 | design:hardware:ide | d | - | R | - | - |
| `ide_rw_start` | bios/ide.c:881 | design:hardware:ide | d | - | R | - | - |
| `ide_select_device` | bios/ide.c:827 | design:hardware:ide | d | - | R | - | - |
| `ide_write` | bios/ide.c:1259 | design:hardware:ide | d | - | R | - | - |
| `idogetdate` | bios/clock.c:1000 | design:hardware:rtc | d | - | R | - | - |
| `idogettime` | bios/clock.c:1019 | design:hardware:rtc | d | - | R | - | - |
| `idosetdate` | bios/clock.c:1043 | design:hardware:rtc | d | - | R | - | - |
| `idosettime` | bios/clock.c:1030 | design:hardware:rtc | d | - | R | - | - |
| `igetdt` | bios/clock.c:1060 | design:hardware:rtc | d | m | R | - | - |
| `igetregs` | bios/clock.c:980 | design:hardware:rtc | d | - | R | - | - |
| `ikbd_readb` | bios/ikbd.c:981 | design:hardware:ikbd-acia | d | m | - | - | - |
| `ikbd_reset` | bios/ikbd.c:1032 | design:hardware:ikbd-acia | d | m | - | - | - |
| `ikbd_writeb` | bios/ikbd.c:957 | design:hardware:ikbd-acia | d | m | R | - | - |
| `ikbd_writew` | bios/ikbd.c:972 | design:hardware:ikbd-acia | d | m | R | - | - |
| `ikbdraw` | bios/aciavecs.S:407 | `ta/xbios/kbdvecs.rs::route_key` | m | m | - | - | - |
| `ikbdsys` | bios/aciavecs.S:382 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | KEYS02/P |
| `incr_tail` | bios/serport.c:146 | `ta/xbios/iorec.rs::push_byte` | m | - | R | - | - |
| `init_acia_vecs` | bios/aciavecs.S:146 | `ta/xbios/kbdvecs.rs::write_table` | m | m | - | - | - |
| `init_bconmap` | bios/serport.c:956 | `k/console.rs::install_aux` +1 | m | m | - | - | - |
| `init_data_in` | bios/scsi.c:697 | design:hardware:scsi | d | - | R | - | - |
| `init_data_out` | bios/scsi.c:676 | design:hardware:scsi | d | - | R | - | - |
| `init_default_environment` | bios/bios.c:626 | `ta/bdos/process/standin.rs::init` | m | m | - | - | - |
| `init_delay` | bios/delay.c:55 | UNMAPPED | u | - | - | - | - |
| `init_et4000` | bios/nova.c:435 | design:hardware:nova | d | - | - | - | - |
| `init_exc_vec` | bios/vectors.S:103 | design:hardware:m68k-cpu | d | - | - | - | - |
| `init_mach32` | bios/nova.c:306 | design:hardware:nova | d | - | - | - | - |
| `init_mach64` | bios/nova.c:387 | design:hardware:nova | d | - | - | - | - |
| `init_nova` | bios/nova.c:741 | design:hardware:nova | d | - | - | - | - |
| `init_nova_resolution` | bios/nova.c:587 | design:hardware:nova | d | - | - | - | - |
| `init_serport` | bios/serport.c:1011 | `ta/xbios/iorec.rs::init` | m | m | - | - | - |
| `init_system_timer` | bios/mfp.c:193 | design:hardware:mfp68901 | d | m | - | - | - |
| `init_system_vars` | bios/nova.c:720 | design:hardware:nova | d | - | - | - | - |
| `init_user_vec` | bios/vectors.S:125 | design:hardware:m68k-cpu | d | m | - | - | - |
| `init_XHDI_drvmap` | bios/xhdi.c:52 | UNMAPPED | u | - | - | - | - |
| `initialise_falcon_palette` | bios/videl.c:998 | design:hardware:videl | d | - | R | - | - |
| `initialise_palette_registers` | bios/screen.c:432 | design:hardware:video-shifter | d | - | - | - | - |
| `initialise_ste_palette` | bios/screen.c:64 | design:hardware:video-shifter | d | - | - | - | - |
| `initialise_tt_palette` | bios/screen.c:370 | design:hardware:video-shifter | d | - | - | - | - |
| `initinfo` | bios/initinfo.c:267 | `k/initinfo.rs::run` | m | m | - | - | - |
| `insert_line` | bios/vt52.c:562 | `ta/vdi/escape.rs::insert_line` | m | m | R | - | - |
| `instruction_cache_kludge` | bios/processor.S:380 | design:hardware:m68k-cpu | d | m | R | - | - |
| `int2bcd` | bios/clock.c:40 | design:hardware:rtc | d | - | R | - | - |
| `int_acia` | bios/aciavecs.S:249 | `ta/xbios/kbdvecs.rs::check_events` | m | - | - | - | - |
| `int_hbl` | bios/vectors.S:288 | design:hardware:video-shifter | d | - | - | - | - |
| `int_illegal` | bios/vectors.S:837 | design:hardware:m68k-cpu | d | - | - | - | - |
| `int_priv` | bios/vectors.S:859 | design:hardware:m68k-cpu | d | - | - | - | - |
| `int_timerc` | bios/vectors.S:502 | `ta/bios/tick.rs::advance` | m | - | - | - | - |
| `int_unimpint` | bios/vectors.S:954 | design:hardware:m68k-cpu | d | - | - | - | - |
| `int_vbl` | bios/vectors.S:329 | `k/vbl.rs::run_if_due` | m | - | - | - | - |
| `internal_inquire` | bios/disk.c:1023 | UNMAPPED | u | - | R | - | - |
| `invalidate_data_cache` | bios/processor.S:479 | UNMAPPED | u | - | R | - | - |
| `invalidate_instruction_cache` | bios/processor.S:401 | UNMAPPED | u | m | R | - | - |
| `invert_cell` | bios/conout.c:397 | UNMAPPED | u | - | R | - | - |
| `inword` | bios/dsp.c:765 | design:hardware:dsp56001 | d | - | - | - | - |
| `is_nfStdErr` | bios/natfeats.c:72 | design:hardware:natfeats | d | - | R | - | - |
| `is_text_pointer` | bios/bios.c:1272 | UNMAPPED | u | - | - | - | - |
| `iseol` | bios/dsp.c:785 | design:hardware:dsp56001 | d | - | - | - | - |
| `isetdt` | bios/clock.c:1067 | design:hardware:rtc | d | m | R | - | - |
| `isetregs` | bios/clock.c:994 | design:hardware:rtc | d | - | R | - | - |
| `just_rte` | bios/vectors.S:1358 | design:hardware:m68k-cpu | d | - | - | - | - |
| `kb_timerc_int` | bios/ikbd.c:533 | UNMAPPED | u | m | - | - | - |
| `kbd_init` | bios/ikbd.c:1062 | `k/key_repeat.rs::new` +1 | m | m | - | - | - |
| `kbd_int` | bios/ikbd.c:725 | `k/ikbd.rs::kbd_int` +3 | m | m | - | - | - |
| `kbdvec` | bios/aciavecs.S:372 | `k/ikbd.rs::kbd_int` | m | m | - | - | - |
| `kcprintf` | bios/kprint.c:271 | UNMAPPED | u | - | R | - | - |
| `keyclick` | bios/vectors.S:1430 | design:hardware:ym2149 | d | m | - | - | - |
| `kill_program` | bios/panicasm.S:96 | `k/arch/aarch64/exceptions.rs::kill_program` | m | m | R | - | - |
| `kprintf` | bios/kprint.c:253 | UNMAPPED | u | m | R | - | - |
| `kprintf_outc_natfeat` | bios/kprint.c:126 | design:hardware:natfeats | d | - | - | - | - |
| `line_wrap_off` | bios/vt52.c:809 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `line_wrap_on` | bios/vt52.c:797 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `lmc1992_init` | bios/dmasound.c:167 | design:hardware:dma-sound | d | m | - | - | - |
| `load_sub` | bios/dsp.c:1002 | design:hardware:dsp56001 | d | - | - | - | - |
| `lookup_videl_mode` | bios/videl.c:384 | design:hardware:videl | d | - | R | - | - |
| `mach_dac_setrs2rs3` | bios/nova.c:342 | design:hardware:nova | d | - | - | - | - |
| `machine_detect` | bios/machine.c:547 | design:source:docs/HARDWARE-PROFILE.md | d | m | - | - | - |
| `machine_init` | bios/machine.c:624 | design:hardware:mfp68901 | d | m | - | - | - |
| `machine_name` | bios/machine.c:826 | `k/initinfo.rs::machine_name` | m | - | - | - | - |
| `main` | bios/startup.S:115 | `ta/bios/reset.rs::entry_kind` | m | m | R | - | - |
| `map_gpt_to_atari` | bios/disk.c:636 | UNMAPPED | u | - | R | - | - |
| `mdogetdate` | bios/clock.c:788 | design:hardware:rtc | d | - | R | - | - |
| `mdogettime` | bios/clock.c:757 | design:hardware:rtc | d | - | R | - | - |
| `mdosetdate` | bios/clock.c:771 | design:hardware:rtc | d | - | R | - | - |
| `mdosettime` | bios/clock.c:740 | design:hardware:rtc | d | - | R | - | - |
| `memdone` | bios/startup.S:528 | design:hardware:m68k-cpu | d | - | - | - | - |
| `meminit` | bios/memory.S:63 | `k/bios/machine/layout.rs::discover` | m | m | - | - | - |
| `mfp_init` | bios/mfp.c:103 | design:hardware:mfp68901 | d | m | - | - | - |
| `mfp_rs232_rx_interrupt` | bios/vectors.S:585 | `k/drivers/gic.rs::rust_irq_handler` | m | - | - | - | - |
| `mfp_rs232_rx_interrupt_handler` | bios/serport.c:350 | `k/console.rs::pl011_rx_drain` | m | m | - | - | - |
| `mfp_rs232_tx_interrupt` | bios/vectors.S:602 | UNMAPPED | u | - | - | - | - |
| `mfp_rs232_tx_interrupt_handler` | bios/serport.c:367 | UNMAPPED | u | - | - | - | - |
| `mfp_tt_rx_interrupt` | bios/vectors.S:629 | design:hardware:tt-mfp | d | - | - | - | - |
| `mfp_tt_rx_interrupt_handler` | bios/serport.c:449 | design:hardware:tt-mfp | d | - | - | - | - |
| `mfp_tt_tx_interrupt` | bios/vectors.S:646 | design:hardware:tt-mfp | d | - | - | - | - |
| `mfp_tt_tx_interrupt_handler` | bios/serport.c:468 | design:hardware:tt-mfp | d | - | - | - | - |
| `mgetdt` | bios/clock.c:808 | design:hardware:rtc | d | m | R | - | - |
| `mgetregs` | bios/clock.c:679 | design:hardware:rtc | d | - | R | - | - |
| `midi_init` | bios/midi.c:117 | design:hardware:midi-acia | d | m | - | - | - |
| `midisys` | bios/aciavecs.S:295 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | - |
| `midivec` | bios/aciavecs.S:315 | design:ADR-031@docs/MEMORY.md:3044 | d | m | - | - | - |
| `mmu_is_emulated` | bios/natfeats.c:154 | design:hardware:natfeats | d | - | - | - | - |
| `monstergetdt` | bios/clock.c:474 | design:hardware:rtc | d | m | R | - | - |
| `monstersetdt` | bios/clock.c:490 | design:hardware:rtc | d | m | R | - | - |
| `move_block` | bios/dsp.c:1020 | design:hardware:dsp56001 | d | - | - | - | - |
| `move_cursor` | bios/conout.c:416 | `ta/vdi/escape.rs::move_cursor` | m | m | R | - | - |
| `msetdt` | bios/clock.c:817 | design:hardware:rtc | d | m | R | - | - |
| `msetregs` | bios/clock.c:711 | design:hardware:rtc | d | - | R | - | - |
| `natfeat_init` | bios/natfeats.c:32 | design:hardware:natfeats | d | - | - | - | - |
| `natfeats_inquire` | bios/disk.c:1010 | design:hardware:natfeats | d | - | R | - | - |
| `neg_cell` | bios/conout.c:297 | `ta/vdi/escape.rs::neg_cell` | m | m | R | - | - |
| `next_cell` | bios/conout.c:345 | UNMAPPED | u | m | R | - | - |
| `next_logical` | bios/blkdev.c:315 | UNMAPPED | u | m | R | - | - |
| `nf_bootstrap` | bios/natfeats.c:109 | design:hardware:natfeats | d | m | - | - | - |
| `nf_getbootdrive` | bios/natfeats.c:122 | design:hardware:natfeats | d | - | - | - | - |
| `nf_getbootstrap_args` | bios/natfeats.c:135 | design:hardware:natfeats | d | - | - | - | - |
| `nf_setlinea` | bios/natfeats.c:169 | design:hardware:natfeats | d | - | - | - | - |
| `nf_shutdown` | bios/natfeats.c:93 | design:hardware:natfeats | d | - | - | - | - |
| `nfGetFullName` | bios/natfeats.c:59 | design:hardware:natfeats | d | - | - | - | - |
| `nfStdErr` | bios/natfeats.c:77 | design:hardware:natfeats | d | - | - | - | - |
| `ngetdt` | bios/clock.c:912 | design:hardware:rtc | d | m | R | - | - |
| `nop` | bios/vt52.c:204 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `normal_ascii` | bios/vt52.c:166 | `ta/vdi/escape.rs::normal_ascii` | m | m | R | - | - |
| `nsetdt` | bios/clock.c:927 | design:hardware:rtc | d | m | R | - | - |
| `nvram_getregs` | bios/clock.c:866 | design:hardware:rtc | d | - | R | - | - |
| `nvram_setregs` | bios/clock.c:893 | design:hardware:rtc | d | - | R | - | - |
| `OK_id` | bios/disk.c:480 | UNMAPPED | u | - | R | - | - |
| `or_icr_reg` | bios/scsi.c:503 | design:hardware:scsi | d | - | R | - | - |
| `or_mode_reg` | bios/scsi.c:527 | design:hardware:scsi | d | - | R | - | - |
| `os_header` | bios/startup.S:77 | UNMAPPED | u | m | R | - | - |
| `osxhbootdelay` | bios/startup.S:313 | UNMAPPED | u | - | - | - | - |
| `outword` | bios/dsp.c:776 | design:hardware:dsp56001 | d | - | - | - | - |
| `pair_end` | bios/initinfo.c:164 | UNMAPPED | u | - | - | - | - |
| `pair_start` | bios/initinfo.c:153 | UNMAPPED | u | - | - | - | - |
| `panic` | bios/panicasm.S:43 | UNMAPPED | u | m | R | - | - |
| `parport_init` | bios/parport.c:108 | design:hardware:parallel-port | d | m | - | - | - |
| `print_art` | bios/initinfo.c:82 | UNMAPPED | u | - | - | - | - |
| `print_spaces` | bios/initinfo.c:61 | UNMAPPED | u | - | - | - | - |
| `prnout` | bios/parport.c:62 | design:hardware:parallel-port | d | m | R | - | - |
| `process_dos_mbr` | bios/disk.c:734 | UNMAPPED | u | - | R | - | - |
| `process_gpt` | bios/disk.c:658 | UNMAPPED | u | - | R | - | - |
| `processor_init` | bios/processor.S:250 | design:hardware:m68k-cpu | d | m | - | - | - |
| `prog_ics2595` | bios/nova.c:368 | design:hardware:nova | d | - | - | - | - |
| `pun_info_setup` | bios/blkdev.c:112 | `ta/bios/bpb.rs::scan` | m | m | - | - | - |
| `push_ikbdiorec` | bios/ikbd.c:157 | `ta/xbios/iorec.rs::push_key` +1 | m | m | - | - | - |
| `push_serial_iorec` | bios/serport.c:331 | `k/console.rs::serial_rx_byte` +1 | m | m | - | - | - |
| `put_data_reg` | bios/scsi.c:471 | design:hardware:scsi | d | - | R | - | - |
| `put_icr_reg` | bios/scsi.c:487 | design:hardware:scsi | d | - | R | - | - |
| `put_iorecbuf` | bios/serport.c:158 | UNMAPPED | u | - | R | - | - |
| `put_mode_reg` | bios/scsi.c:511 | design:hardware:scsi | d | - | R | - | - |
| `put_select_enable_reg` | bios/scsi.c:559 | design:hardware:scsi | d | - | R | - | - |
| `put_tcr_reg` | bios/scsi.c:535 | design:hardware:scsi | d | - | R | - | - |
| `ramdac_68860_config` | bios/nova.c:481 | design:hardware:nova | d | - | - | - | - |
| `ramdac_ch8398_config` | bios/nova.c:509 | design:hardware:nova | d | - | - | - | - |
| `ramdac_ch8398_unlock` | bios/nova.c:493 | design:hardware:nova | d | - | - | - | - |
| `ramdac_hicolor_off` | bios/nova.c:462 | design:hardware:nova | d | - | - | - | - |
| `read_ds1307` | bios/clock.c:457 | design:hardware:rtc | d | - | R | - | - |
| `realmain` | bios/startup.S:335 | design:hardware:m68k-cpu | d | - | R | - | - |
| `receive_byte` | bios/scsi.c:873 | design:hardware:scsi | d | - | R | - | - |
| `reset_bus` | bios/scsi.c:890 | design:hardware:scsi | d | - | R | - | - |
| `reset_mfp_regs` | bios/mfp.c:25 | design:hardware:mfp68901 | d | - | - | - | - |
| `restore` | bios/floppy.c:1494 | design:hardware:floppy | d | m | R | - | - |
| `restore_cursor_pos` | bios/vt52.c:712 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `reverse_linefeed` | bios/vt52.c:545 | `ta/vdi/escape.rs::reverse_linefeed` | m | m | R | - | - |
| `reverse_video_off` | bios/vt52.c:532 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `reverse_video_on` | bios/vt52.c:519 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `rez_changeable` | bios/screen.c:681 | design:ADR-017@docs/MEMORY.md:824 | d | m | - | - | - |
| `rsconf1` | bios/serport.c:386 | `ta/xbios/system.rs::rsconf` | m | m | R | - | - |
| `rsconf_dummy` | bios/serport.c:945 | UNMAPPED | u | m | - | - | - |
| `rsconf_mfp` | bios/serport.c:217 | design:hardware:mfp68901 | d | m | R | - | - |
| `rsconf_scc` | bios/serport.c:775 | design:hardware:scc | d | m | - | - | - |
| `rsconfA` | bios/serport.c:865 | design:hardware:scc | d | m | - | - | - |
| `rsconfB` | bios/serport.c:872 | UNMAPPED | u | m | - | - | - |
| `rsconfTT` | bios/serport.c:488 | design:hardware:tt-mfp | d | m | - | - | - |
| `run_auto_program` | bios/bios.c:685 | UNMAPPED | u | m | - | - | - |
| `run_calibration` | bios/delayasm.S:30 | UNMAPPED | u | - | - | - | - |
| `run_cartridge_applications` | bios/startup.S:545 | design:hardware:cartridge-port | d | - | R | - | - |
| `save_cursor_pos` | bios/vt52.c:694 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `scc_es_interrupt_handler` | bios/serport.c:734 | design:hardware:scc | d | - | - | - | - |
| `scc_init` | bios/serport.c:905 | design:hardware:scc | d | - | - | - | - |
| `scc_rx_interrupt_handler` | bios/serport.c:647 | `k/console.rs::pl011_rx_drain` +2 | m | m | - | - | - |
| `scc_tx_interrupt_handler` | bios/serport.c:687 | UNMAPPED | u | - | - | - | - |
| `scca_es_interrupt` | bios/vectors.S:717 | design:hardware:scc | d | - | - | - | - |
| `scca_rx_interrupt` | bios/vectors.S:677 | design:hardware:scc | d | - | - | - | - |
| `scca_tx_interrupt` | bios/vectors.S:697 | design:hardware:scc | d | - | - | - | - |
| `sccb_es_interrupt` | bios/vectors.S:777 | design:hardware:scc | d | - | - | - | - |
| `sccb_rx_interrupt` | bios/vectors.S:737 | `k/drivers/gic.rs::rust_irq_handler` | m | - | - | - | - |
| `sccb_tx_interrupt` | bios/vectors.S:757 | UNMAPPED | u | - | - | - | - |
| `screen_get_current_mode_info` | bios/screen.c:811 | UNMAPPED | u | - | R | - | - |
| `screen_init_address` | bios/screen.c:636 | UNMAPPED | u | m | - | - | - |
| `screen_init_mode` | bios/screen.c:487 | design:hardware:video-shifter | d | m | - | - | - |
| `scroll_down` | bios/conout.c:733 | `ta/vdi/escape.rs::scroll_down` | m | m | R | - | - |
| `scroll_up` | bios/conout.c:706 | `ta/vdi/escape.rs::scroll_up` | m | m | R | - | - |
| `scsi_arbitrate` | bios/scsi.c:919 | design:hardware:scsi | d | - | R | - | - |
| `scsi_capacity` | bios/scsi.c:1355 | design:hardware:scsi | d | - | R | - | - |
| `scsi_dispatcher` | bios/scsi.c:1180 | design:hardware:scsi | d | - | R | - | - |
| `scsi_init` | bios/scsi.c:274 | design:hardware:scsi | d | - | - | - | - |
| `scsi_inquiry` | bios/scsi.c:1375 | design:hardware:scsi | d | - | R | - | - |
| `scsi_ioctl` | bios/scsi.c:316 | design:hardware:scsi | d | - | R | - | - |
| `scsi_request_sense` | bios/scsi.c:1290 | design:hardware:scsi | d | - | R | - | - |
| `scsi_rw` | bios/scsi.c:359 | design:hardware:scsi | d | - | R | - | - |
| `scsi_select` | bios/scsi.c:963 | design:hardware:scsi | d | - | R | - | - |
| `scsidriv_CheckDev` | bios/scsidriv.c:498 | UNMAPPED | u | - | - | - | - |
| `scsidriv_Close` | bios/scsidriv.c:592 | UNMAPPED | u | - | - | - | - |
| `scsidriv_Error` | bios/scsidriv.c:622 | UNMAPPED | u | - | - | - | - |
| `scsidriv_In` | bios/scsidriv.c:415 | UNMAPPED | u | - | - | - | - |
| `scsidriv_init` | bios/scsidriv.c:117 | UNMAPPED | u | - | - | - | - |
| `scsidriv_InOut` | bios/scsidriv.c:270 | UNMAPPED | u | - | - | - | - |
| `scsidriv_InquireBus` | bios/scsidriv.c:465 | UNMAPPED | u | - | - | - | - |
| `scsidriv_InquireSCSI` | bios/scsidriv.c:431 | UNMAPPED | u | - | - | - | - |
| `scsidriv_Open` | bios/scsidriv.c:561 | UNMAPPED | u | - | - | - | - |
| `scsidriv_Out` | bios/scsidriv.c:423 | UNMAPPED | u | - | - | - | - |
| `scsidriv_RescanBus` | bios/scsidriv.c:514 | UNMAPPED | u | - | - | - | - |
| `select` | bios/floppy.c:1458 | design:hardware:floppy | d | m | R | - | - |
| `send_byte` | bios/scsi.c:858 | design:hardware:scsi | d | - | R | - | - |
| `send_command` | bios/acsi.c:478 | design:hardware:acsi | d | - | R | - | - |
| `send_ide_command` | bios/ide.c:1980 | design:hardware:ide | d | - | - | - | - |
| `send_scsi_command` | bios/scsi.c:1223 | design:hardware:scsi | d | - | R | - | - |
| `send_uservect` | bios/dsp.c:1053 | design:hardware:dsp56001 | d | - | - | - | - |
| `set_bg` | bios/vt52.c:329 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `set_cache` | bios/processor.S:586 | design:hardware:m68k-cpu | d | - | - | - | - |
| `set_chs_mode` | bios/ide.c:1404 | design:hardware:ide | d | - | - | - | - |
| `set_command_head` | bios/ide.c:872 | design:hardware:ide | d | - | R | - | - |
| `set_cylinder` | bios/ide.c:863 | design:hardware:ide | d | - | R | - | - |
| `set_density` | bios/floppy.c:1264 | design:hardware:floppy | d | - | R | - | - |
| `set_dma_addr` | bios/dma.c:13 | design:hardware:atari-dma | d | - | R | - | - |
| `set_fdc_reg` | bios/floppy.c:1604 | design:hardware:floppy | d | - | R | - | - |
| `set_fg` | bios/vt52.c:323 | `ta/vdi/escape.rs::esc_ch1` | m | m | R | - | - |
| `set_idxreg` | bios/nova.c:159 | design:hardware:nova | d | - | - | - | - |
| `set_input_phase` | bios/scsi.c:594 | design:hardware:scsi | d | - | R | - | - |
| `set_interface_magic` | bios/ide.c:412 | design:hardware:ide | d | - | - | - | - |
| `set_lba48_mode` | bios/ide.c:1438 | design:hardware:ide | d | - | - | - | - |
| `set_line` | bios/initinfo.c:106 | UNMAPPED | u | - | - | - | - |
| `set_mach_idxreg` | bios/nova.c:527 | design:hardware:nova | d | - | - | - | - |
| `set_margin` | bios/initinfo.c:75 | UNMAPPED | u | - | - | - | - |
| `set_multiple_atcreg` | bios/nova.c:177 | design:hardware:nova | d | - | - | - | - |
| `set_multiple_idxreg` | bios/nova.c:167 | design:hardware:nova | d | - | - | - | - |
| `set_multiple_mode` | bios/ide.c:1477 | design:hardware:ide | d | - | - | - | - |
| `set_nvram_rtc` | bios/nvram.c:80 | design:hardware:nvram | d | - | R | - | - |
| `set_output_phase` | bios/scsi.c:575 | design:hardware:scsi | d | - | R | - | - |
| `set_packet_size` | bios/ide.c:1625 | design:hardware:ide | d | - | - | - | - |
| `set_palette_entries` | bios/nova.c:562 | design:hardware:nova | d | - | - | - | - |
| `set_psg_porta` | bios/floppy.c:1405 | design:hardware:floppy | d | - | R | - | - |
| `set_rez_hacked` | bios/screen.c:668 | UNMAPPED | u | m | - | - | - |
| `set_start_count` | bios/ide.c:854 | design:hardware:ide | d | - | R | - | - |
| `set_sum` | bios/nvram.c:134 | design:hardware:nvram | d | - | R | - | - |
| `set_track` | bios/floppy.c:1508 | design:hardware:floppy | d | - | R | - | - |
| `set_videl_vga` | bios/videl.c:511 | design:hardware:videl | d | i | R | - | - |
| `setphys` | bios/screen.c:1058 | `ta/host.rs::set_screen_physical` | m | m | R | - | - |
| `setup_68030_pmmu` | bios/pmmu030.c:150 | design:hardware:m68k-cpu | d | - | - | - | - |
| `setup_falcon_dma` | bios/scsi.c:640 | design:hardware:scsi | d | - | R | - | - |
| `setup_timer` | bios/mfp.c:133 | design:hardware:mfp68901 | d | m | R | - | - |
| `setup_tt_dma` | bios/scsi.c:613 | design:hardware:scsi | d | - | R | - | - |
| `setvalue_fdc` | bios/machine.c:489 | design:hardware:floppy | d | - | - | - | - |
| `setvalue_mch` | bios/machine.c:382 | `ta/cookie.rs::seed_defaults` | m | m | - | - | - |
| `setvalue_snd` | bios/machine.c:409 | design:ADR-012@docs/MEMORY.md:295 | d | m | - | - | - |
| `setvalue_swi` | bios/machine.c:353 | design:hardware:dip-switches | d | - | - | - | - |
| `setvalue_vdo` | bios/machine.c:363 | design:ADR-012@docs/MEMORY.md:401 | d | i | - | - | - |
| `shifter_check_moderez` | bios/screen.c:86 | design:ADR-017@docs/MEMORY.md:824 | d | - | - | - | - |
| `shifter_get_current_mode_info` | bios/screen.c:787 | design:hardware:video-shifter | d | - | R | - | - |
| `shifter_get_monitor_type` | bios/screen.c:120 | design:hardware:video-shifter | d | - | R | - | - |
| `shifter_rez_changeable` | bios/screen.c:110 | design:ADR-017@docs/MEMORY.md:824 | d | - | - | - | - |
| `shutdown` | bios/bios.c:738 | UNMAPPED | u | m | - | - | DESK07/D |
| `skipspaces` | bios/dsp.c:806 | design:hardware:dsp56001 | d | - | - | - | - |
| `skiptoeol` | bios/dsp.c:794 | design:hardware:dsp56001 | d | - | - | - | - |
| `snd_init` | bios/sound.c:69 | design:hardware:ym2149 | d | m | - | - | - |
| `sndcmd_falcon` | bios/dmasound.c:246 | design:hardware:dma-sound | d | m | R | - | - |
| `sndcmd_ste` | bios/dmasound.c:308 | design:hardware:dma-sound | d | m | R | - | - |
| `sndirq` | bios/sound.c:163 | design:hardware:ym2149 | d | m | - | - | - |
| `switch_density` | bios/floppy.c:1275 | design:hardware:floppy | d | - | R | - | - |
| `test_video_memory` | bios/nova.c:704 | design:hardware:nova | d | - | - | - | - |
| `timeout_gpip` | bios/mfp.c:172 | design:hardware:mfp68901 | d | - | R | - | - |
| `toggle_dmaout` | bios/scsi.c:630 | design:hardware:scsi | d | - | R | - | - |
| `tt_mfp_init` | bios/mfp.c:80 | design:hardware:tt-mfp | d | - | - | - | - |
| `tt_mfpint` | bios/mfp.c:88 | design:hardware:tt-mfp | d | m | - | - | - |
| `ttram_detect` | bios/memory2.c:91 | design:hardware:tt-ram | d | - | - | - | - |
| `ultrasatan_get_clock` | bios/acsi.c:648 | design:hardware:acsi | d | - | R | - | - |
| `ultrasatan_get_running_firmware` | bios/acsi.c:633 | design:hardware:acsi | d | - | R | - | - |
| `ultrasatan_getdt` | bios/clock.c:1078 | design:hardware:rtc | d | m | R | - | - |
| `ultrasatan_set_clock` | bios/acsi.c:663 | design:hardware:acsi | d | - | R | - | - |
| `ultrasatan_setdt` | bios/clock.c:1105 | design:hardware:rtc | d | m | R | - | - |
| `unit_is_byteswapped` | bios/disk.c:582 | design:hardware:ide | d | - | R | - | - |
| `unlock_et4000` | bios/nova.c:427 | design:hardware:nova | d | - | - | - | - |
| `update_dspvect` | bios/dsp.c:1040 | design:hardware:dsp56001 | d | - | - | - | - |
| `use_dma` | bios/scsi.c:655 | design:hardware:scsi | d | - | R | - | - |
| `use_ste_palette` | bios/videl.c:778 | design:hardware:videl | d | - | R | - | - |
| `valid_bus` | bios/scsidriv.c:142 | UNMAPPED | u | - | - | - | - |
| `valid_bus_and_device` | bios/scsidriv.c:156 | UNMAPPED | u | - | - | - | - |
| `VALID_PARTITION` | bios/disk.c:467 | UNMAPPED | u | - | R | - | - |
| `vcprintf` | bios/kprint.c:62 | UNMAPPED | u | - | R | - | - |
| `vecs_init` | bios/bios.c:118 | `ta/bios/vectors.rs::write_page0_vectors` | m | m | - | - | - |
| `videl_check_moderez` | bios/videl.c:957 | design:ADR-017@docs/MEMORY.md:824 | d | - | - | - | - |
| `videl_get_current_mode_info` | bios/videl.c:969 | design:hardware:videl | d | - | R | - | - |
| `videl_setrez` | bios/videl.c:976 | design:ADR-017@docs/MEMORY.md:824 | d | i | R | - | - |
| `vkcprintf` | bios/kprint.c:265 | UNMAPPED | u | m | R | - | - |
| `vkprintf` | bios/kprint.c:156 | UNMAPPED | u | - | R | - | - |
| `vt52_init` | bios/vt52.c:921 | `ta/vdi/escape.rs::init` +1 | m | m | R | - | - |
| `wait_dma_complete` | bios/scsi.c:716 | design:hardware:scsi | d | - | R | - | - |
| `wait_for_not_BSY` | bios/ide.c:794 | design:hardware:ide | d | - | R | - | - |
| `wait_for_not_BSY_and_DRDY` | bios/ide.c:681 | design:hardware:ide | d | - | - | - | - |
| `wait_for_not_BSY_not_DRQ` | bios/ide.c:810 | design:hardware:ide | d | - | R | - | - |
| `wait_phase_change` | bios/scsi.c:749 | design:hardware:scsi | d | - | R | - | - |
| `wait_req` | bios/scsi.c:759 | design:hardware:scsi | d | - | R | - | - |
| `warm_reset` | bios/panicasm.S:105 | `ta/host.rs::system_restart` +1 | m | m | R | - | - |
| `write_ds1307` | bios/clock.c:448 | design:hardware:rtc | d | - | R | - | - |
| `write_microwire` | bios/dmasound.c:152 | design:hardware:dma-sound | d | - | R | - | - |
| `write_scc` | bios/serport.c:626 | design:hardware:scc | d | - | - | - | - |
| `write_scc_reg0` | bios/serport.c:637 | design:hardware:scc | d | - | - | - | - |
| `xbios_do_unimpl` | bios/xbios.c:1251 | `ta/xbios/mod.rs::dispatch` | m | i | - | - | - |
| `xbiostrap` | bios/vectors.S:1275 | `ta/xbios/mod.rs::dispatch` | m | - | - | - | - |
| `xhdi_handler` | bios/xhdi.c:460 | UNMAPPED | u | - | - | - | - |
| `xhdi_vec` | bios/natfeat.S:96 | design:hardware:natfeats | d | - | - | - | - |
| `XHDOSLimits` | bios/xhdi.c:248 | UNMAPPED | u | - | - | - | - |
| `XHDriverSpecial` | bios/xhdi.c:221 | UNMAPPED | u | - | - | - | - |
| `XHDrvMap` | bios/xhdi.c:71 | UNMAPPED | u | - | - | - | - |
| `XHEject` | bios/xhdi.c:183 | UNMAPPED | u | - | - | - | - |
| `XHGetCapacity` | bios/xhdi.c:408 | UNMAPPED | u | - | - | - | - |
| `XHGetVersion` | bios/xhdi.c:58 | UNMAPPED | u | - | - | - | - |
| `XHInqDev` | bios/xhdi.c:140 | UNMAPPED | u | - | - | - | - |
| `XHInqDev2` | bios/xhdi.c:91 | UNMAPPED | u | - | - | - | - |
| `XHInqDriver` | bios/xhdi.c:192 | UNMAPPED | u | - | - | - | - |
| `XHInqTarget` | bios/xhdi.c:385 | UNMAPPED | u | - | - | - | - |
| `XHInqTarget2` | bios/xhdi.c:362 | UNMAPPED | u | - | - | - | - |
| `XHLastAccess` | bios/xhdi.c:342 | UNMAPPED | u | - | - | - | - |
| `XHLock` | bios/xhdi.c:165 | UNMAPPED | u | - | - | - | - |
| `XHMediumChanged` | bios/xhdi.c:230 | UNMAPPED | u | - | - | - | - |
| `XHMiNTInfo` | bios/xhdi.c:239 | UNMAPPED | u | - | - | - | - |
| `XHReaccess` | bios/xhdi.c:351 | UNMAPPED | u | - | - | - | - |
| `XHReadWrite` | bios/xhdi.c:430 | UNMAPPED | u | - | - | - | - |
| `XHReserve` | bios/xhdi.c:156 | UNMAPPED | u | - | - | - | - |
| `XHStop` | bios/xhdi.c:174 | UNMAPPED | u | - | - | - | - |

### 2.12 desk/ (EmuDESK; out of API scope, ADR-026) - 325 rows

| fn | def | spec | R | tickets |
|---|---|---|---|---|
| `act_allchg` | desk/deskact.c:480 | m | - | - |
| `act_bdown` | desk/deskact.c:609 | m | - | - |
| `act_bsclick` | desk/deskact.c:566 | m | - | - |
| `act_chg` | desk/deskact.c:419 | m | - | DESK61/P DESK59/D |
| `act_chkobj` | desk/deskact.c:374 | m | - | - |
| `add_fname` | desk/deskdir.c:177 | m | - | - |
| `add_one_level` | desk/deskfun.c:206 | m | - | - |
| `add_path` | desk/deskdir.c:146 | m | - | - |
| `adjust_3d_positions` | desk/deskmain.c:1647 | m | - | - |
| `adjust_menu` | desk/deskmain.c:1511 | m | - | A108/I |
| `align_objects` | desk/deskmain.c:1680 | m | - | - |
| `align_title` | desk/deskmain.c:1731 | m | - | - |
| `allocate_window_anode` | desk/deskins.c:1028 | m | - | - |
| `app_afind_by_id` | desk/deskapp.c:1410 | m | - | - |
| `app_afind_by_name` | desk/deskapp.c:1437 | m | - | - |
| `app_afind_viewer` | desk/deskapp.c:1473 | m | - | - |
| `app_alloc` | desk/deskapp.c:207 | m | - | - |
| `app_blddesk` | desk/deskapp.c:1337 | m | - | - |
| `app_free` | desk/deskapp.c:228 | m | - | - |
| `app_parse` | desk/deskapp.c:295 | m | - | - |
| `app_rdicon` | desk/deskapp.c:662 | m | - | - |
| `app_read_inf` | desk/deskapp.c:1492 | m | - | DESK14/I |
| `app_revit` | desk/deskapp.c:1019 | m | - | - |
| `app_save` | desk/deskapp.c:1106 | m | - | - |
| `app_start` | desk/deskapp.c:781 | m | - | - |
| `app_tran` | desk/deskapp.c:439 | m | - | - |
| `appl_exit` | desk/gembind.c:123 | m | - | SCHED01/P ACC10/D |
| `appl_init` | desk/gembind.c:111 | m | - | A06/P A07/P A08/P +3 |
| `atow` | desk/deskins.c:381 | m | - | - |
| `bios_conin` | desk/desksupp.c:482 | - | - | - |
| `bios_conis` | desk/desksupp.c:490 | - | - | - |
| `bios_conout` | desk/desksupp.c:675 | - | - | - |
| `bios_conws` | desk/desksupp.c:683 | m | R | - |
| `bios_prnout` | desk/desksupp.c:498 | - | - | - |
| `blank_line` | desk/desksupp.c:692 | m | - | - |
| `build_inf` | desk/deskapp.c:698 | m | - | - |
| `build_selected_path` | desk/deskfun.c:903 | m | - | - |
| `change_falcon_rez` | desk/deskrez.c:152 | m | - | - |
| `change_resolution` | desk/deskrez.c:316 | m | - | - |
| `change_st_rez` | desk/deskrez.c:92 | m | - | - |
| `change_tt_rez` | desk/deskrez.c:107 | m | - | - |
| `check_alt_letter_key` | desk/deskmain.c:1136 | m | - | - |
| `check_arrow_key` | desk/deskmain.c:1067 | m | - | - |
| `check_function_key` | desk/deskmain.c:1117 | m | - | - |
| `clear_all_autorun` | desk/deskins.c:345 | m | - | - |
| `clear_screen` | desk/desksupp.c:701 | - | - | - |
| `close_desktop_windows` | desk/deskmain.c:1921 | m | - | - |
| `cnx_get` | desk/deskmain.c:1430 | m | - | DESK09/I |
| `cnx_put` | desk/deskmain.c:1371 | m | - | DESK09/I |
| `convert` | desk/deskfun.c:264 | m | - | - |
| `copy_menu_items` | desk/deskmain.c:1755 | m | - | - |
| `count_ffs` | desk/deskinf.c:476 | m | - | - |
| `d_dofcopy` | desk/deskdir.c:394 | m | - | - |
| `d_dofdel` | desk/deskdir.c:237 | m | - | - |
| `d_dofileren` | desk/deskdir.c:785 | m | - | DESK31/D |
| `d_dofoldel` | desk/deskdir.c:269 | m | - | - |
| `d_dofoldren` | desk/deskdir.c:831 | m | - | - |
| `d_doop` | desk/deskdir.c:528 | m | - | - |
| `del_fname` | desk/deskdir.c:214 | m | - | - |
| `delete_ffd` | desk/deskfun.c:1349 | m | - | - |
| `deselect_all` | desk/desksupp.c:110 | m | - | - |
| `desk_all` | desk/deskmain.c:438 | m | - | - |
| `desk_busy_off` | desk/desksupp.c:1849 | m | - | - |
| `desk_busy_on` | desk/desksupp.c:1840 | m | - | - |
| `desk_clear` | desk/desksupp.c:122 | m | - | - |
| `desk_clear_all` | desk/desksupp.c:160 | m | - | - |
| `desk_get_videomode` | desk/deskapp.c:1083 | m | - | - |
| `desk_rs_init` | desk/desk_rsc.c:2568 | m | - | - |
| `desk_verify` | desk/desksupp.c:175 | m | - | DESK31/D |
| `desk_xlate_fix` | desk/deskmain.c:1864 | m | - | - |
| `deskmain` | desk/deskmain.c:1938 | m | - | - |
| `deskstart` | desk/deskstart.S:26 | m | - | - |
| `desktop_set_cache` | desk/deskmain.c:428 | m | - | - |
| `desktop_str_addr` | desk/deskrsrc.c:33 | m | - | - |
| `detect_features` | desk/deskmain.c:394 | m | - | - |
| `determine_default_drive` | desk/desksupp.c:1529 | - | - | - |
| `dir_op` | desk/deskdir.c:871 | m | - | - |
| `do_aopen` | desk/desksupp.c:843 | m | - | - |
| `do_deskmenu` | desk/deskmain.c:612 | m | - | - |
| `do_diropen` | desk/desksupp.c:422 | m | - | DESK50/P |
| `do_dopen` | desk/desksupp.c:1041 | m | - | - |
| `do_filemenu` | desk/deskmain.c:654 | m | - | - |
| `do_fopen` | desk/desksupp.c:1089 | m | - | DESK52/P |
| `do_format` | desk/desksupp.c:1561 | - | - | - |
| `do_info` | desk/desksupp.c:1288 | m | - | - |
| `do_namecon` | desk/deskdir.c:93 | m | - | - |
| `do_open` | desk/desksupp.c:1191 | m | - | - |
| `do_optnmenu` | desk/deskmain.c:836 | m | - | - |
| `do_viewmenu` | desk/deskmain.c:786 | m | - | - |
| `do_wfull` | desk/desksupp.c:329 | m | - | - |
| `do_wopen` | desk/desksupp.c:294 | m | - | DESK50/P |
| `do_wredraw` | desk/desksupp.c:202 | m | - | - |
| `do_xyfix` | desk/desksupp.c:262 | m | - | DESK54/P |
| `dr_code` | desk/deskinf.c:331 | m | - | - |
| `dr_fnode` | desk/deskinf.c:310 | m | - | - |
| `draw_dial` | desk/deskdir.c:79 | m | - | - |
| `draw_fld` | desk/deskdir.c:131 | m | - | - |
| `end_dialog` | desk/deskinf.c:363 | m | - | - |
| `evnt_button` | desk/gembind.c:188 | m | - | - |
| `evnt_dclick` | desk/gembind.c:282 | m | - | - |
| `evnt_multi` | desk/gembind.c:239 | m | - | A92/I ACC03/P A112/D |
| `evnt_timer` | desk/gembind.c:231 | m | - | - |
| `file_exists` | desk/desksupp.c:354 | m | - | - |
| `fl_free` | desk/deskfpd.c:45 | - | - | - |
| `fmt_date` | desk/deskinf.c:124 | m | - | - |
| `fmt_size` | desk/deskinf.c:190 | m | - | - |
| `fmt_time` | desk/deskinf.c:74 | m | - | - |
| `fnode_is_selected` | desk/deskfun.h:50 | m | - | - |
| `form_alert` | desk/gembind.c:476 | m | - | - |
| `form_center` | desk/gembind.c:497 | m | - | DESK08/I |
| `form_dial` | desk/gembind.c:460 | m | - | DESK08/I |
| `form_do` | desk/gembind.c:452 | m | - | - |
| `form_error` | desk/gembind.c:489 | m | - | - |
| `format_floppy` | desk/desksupp.c:1436 | - | - | - |
| `format_fnode` | desk/deskinf.c:220 | m | - | - |
| `fsel_exinput` | desk/gembind.c:702 | m | - | - |
| `fun_alert` | desk/deskfun.c:75 | m | - | - |
| `fun_alert_merge` | desk/deskfun.c:88 | m | - | - |
| `fun_close` | desk/deskfun.c:862 | m | - | - |
| `fun_del` | desk/deskfun.c:1376 | m | - | - |
| `fun_desk2desk` | desk/deskfun.c:1260 | m | - | - |
| `fun_desk2win` | desk/deskfun.c:1198 | m | - | - |
| `fun_drag` | desk/deskfun.c:1302 | m | - | - |
| `fun_file2any` | desk/deskfun.c:1135 | m | - | - |
| `fun_file2desk` | desk/deskfun.c:1004 | m | - | - |
| `fun_file2win` | desk/deskfun.c:1094 | m | - | - |
| `fun_full_close` | desk/deskfun.c:828 | m | - | DESK51/P |
| `fun_mark_for_rebld` | desk/deskfun.c:119 | m | - | - |
| `fun_mask` | desk/deskfun.c:639 | m | - | - |
| `fun_mkdir` | desk/deskfun.c:680 | m | - | - |
| `fun_msg` | desk/deskfun.c:101 | m | - | - |
| `fun_op` | desk/deskfun.c:782 | m | - | - |
| `fun_print` | desk/deskfun.c:746 | m | - | - |
| `fun_rebld` | desk/deskfun.c:173 | m | - | DESK67/P DESK26/D |
| `fun_rebld_marked` | desk/deskfun.c:150 | m | - | DESK31/D |
| `fun_search` | desk/deskfun.c:560 | m | - | - |
| `fun_selectall` | desk/deskfun.c:607 | m | - | - |
| `fun_win2desk` | desk/deskfun.c:1117 | m | - | - |
| `fun_win2win` | desk/deskfun.c:942 | m | - | DESK19/D |
| `gem` | desk/gembind.c:72 | m | - | - |
| `gem_if` | desk/gembind.c:100 | - | - | - |
| `gem_if0` | desk/gembind.c:89 | m | - | - |
| `get_all_funkeys` | desk/deskinf.c:929 | m | - | - |
| `get_funkey` | desk/deskins.c:404 | m | - | - |
| `get_iconblk_ptr` | desk/desksupp.c:251 | m | - | - |
| `get_key` | desk/desksupp.c:509 | m | - | - |
| `get_new_name` | desk/deskdir.c:700 | m | - | DESK24/D |
| `get_xywh` | desk/desksupp.c:238 | m | - | - |
| `gr_accobs` | desk/deskact.c:87 | m | - | - |
| `gr_bwait` | desk/deskact.c:210 | m | - | - |
| `gr_drgplns` | desk/deskact.c:251 | m | - | - |
| `gr_extent` | desk/deskact.c:169 | m | - | - |
| `gr_isdown` | desk/deskact.c:66 | m | - | - |
| `gr_obalign` | desk/deskact.c:236 | m | - | - |
| `gr_obfind` | desk/deskact.c:50 | m | - | - |
| `gr_plns` | desk/deskact.c:195 | m | - | - |
| `graf_growbox_grect` | desk/gembind.c:589 | m | - | - |
| `graf_handle` | desk/gembind.c:639 | m | - | - |
| `graf_mkstate` | desk/gembind.c:658 | m | - | - |
| `graf_mouse` | desk/gembind.c:650 | m | - | A147/P A44/D |
| `graf_rubbox` | desk/gembind.c:541 | m | - | - |
| `graf_shrinkbox_grect` | desk/gembind.c:603 | m | - | - |
| `grid_free` | desk/deskins.c:118 | m | - | - |
| `hndl_button` | desk/deskmain.c:947 | m | - | - |
| `hndl_kbd` | desk/deskmain.c:1201 | m | - | - |
| `hndl_menu` | desk/deskmain.c:1012 | m | - | - |
| `hndl_msg` | desk/deskmain.c:1250 | m | - | - |
| `i_find` | desk/desksupp.c:1733 | m | - | - |
| `illegal_op_msg` | desk/deskdir.c:220 | m | - | - |
| `inf_backgrounds` | desk/deskinf.c:825 | m | - | - |
| `inf_conf` | desk/deskinf.c:1096 | m | - | - |
| `inf_disk` | desk/deskinf.c:650 | m | - | - |
| `inf_dttm` | desk/deskinf.c:460 | m | - | - |
| `inf_fifosz` | desk/deskinf.c:440 | m | - | - |
| `inf_file_folder` | desk/deskinf.c:493 | m | - | DESK39/I |
| `inf_numset` | desk/deskinf.c:410 | m | - | - |
| `inf_pref` | desk/deskinf.c:721 | m | - | - |
| `inf_show` | desk/deskinf.c:396 | m | - | - |
| `inf_which` | desk/deskinf.c:705 | m | - | - |
| `init_conf_funkeys` | desk/deskinf.c:979 | m | - | - |
| `init_conf_shortcuts` | desk/deskinf.c:1002 | m | - | - |
| `init_start` | desk/desksupp.c:1372 | - | - | - |
| `initialise_anodes` | desk/deskapp.c:611 | m | - | - |
| `ins_app` | desk/deskins.c:721 | m | - | - |
| `ins_app_dialog` | desk/deskins.c:455 | m | - | - |
| `ins_devices` | desk/deskins.c:290 | m | - | - |
| `ins_icon` | desk/deskins.c:1234 | m | - | - |
| `ins_one_app` | desk/deskins.c:611 | m | - | - |
| `ins_posdisk` | desk/deskins.c:197 | m | - | - |
| `ins_shortcut` | desk/deskins.c:1341 | m | - | - |
| `insert_icon` | desk/deskins.c:762 | m | - | - |
| `install_desktop_icon` | desk/deskins.c:781 | m | - | - |
| `install_drive` | desk/deskins.c:230 | m | - | - |
| `install_shortcuts` | desk/deskmain.c:1825 | m | - | - |
| `install_window_icon` | desk/deskins.c:1061 | m | - | - |
| `is_executable` | desk/deskins.c:989 | m | - | - |
| `is_installed` | desk/deskins.c:71 | m | - | - |
| `is_viewer` | desk/deskins.c:87 | m | - | - |
| `item_exists` | desk/deskdir.c:189 | m | - | - |
| `kbd_arrow` | desk/deskmain.c:1043 | m | - | - |
| `load_icon_file` | desk/deskapp.c:537 | m | - | - |
| `lookup_ascii_shortcut` | desk/deskmain.c:1169 | m | - | - |
| `malloc_fail_alert` | desk/desksupp.c:1831 | m | - | - |
| `mark_matching_fnodes` | desk/deskfun.c:357 | m | - | - |
| `men_list` | desk/deskmain.c:453 | m | - | - |
| `men_update` | desk/deskmain.c:464 | m | - | DESK17/D |
| `menu_bar` | desk/gembind.c:293 | m | - | - |
| `menu_icheck` | desk/gembind.c:301 | m | - | - |
| `menu_ienable` | desk/gembind.c:310 | m | - | - |
| `menu_tnormal` | desk/gembind.c:319 | m | - | - |
| `move_drvicon` | desk/deskact.c:126 | m | - | - |
| `must_alloc_buf` | desk/deskapp.c:761 | m | - | - |
| `nomem_alert` | desk/deskapp.c:195 | m | - | - |
| `obj_get_obid` | desk/deskobj.c:249 | m | - | - |
| `obj_ialloc` | desk/deskobj.c:220 | m | - | - |
| `obj_init` | desk/deskobj.c:113 | m | - | - |
| `obj_walloc` | desk/deskobj.c:156 | m | - | - |
| `obj_wfree` | desk/deskobj.c:187 | m | - | - |
| `objc_add` | desk/gembind.c:352 | m | - | A55/D |
| `objc_draw` | desk/gembind.c:371 | m | - | A138/P A46/D A65/D |
| `objc_find` | desk/gembind.c:385 | m | - | - |
| `objc_offset` | desk/gembind.c:396 | m | - | - |
| `objc_order` | desk/gembind.c:407 | m | - | A47/D |
| `opn_appl` | desk/deskinf.c:1253 | m | - | DESK47/I |
| `output_fname` | desk/deskdir.c:299 | m | - | - |
| `output_path` | desk/deskdir.c:734 | m | - | DESK24/D |
| `play_sound` | desk/deskmain.c:375 | m | - | - |
| `pn_active` | desk/deskfpd.c:227 | m | - | - |
| `pn_clear` | desk/deskfpd.c:304 | m | - | - |
| `pn_close` | desk/deskfpd.c:59 | m | - | - |
| `pn_comp` | desk/deskfpd.c:138 | m | - | - |
| `pn_count` | desk/deskfpd.c:335 | m | - | - |
| `pn_fcomp` | desk/deskfpd.c:99 | m | - | - |
| `pn_open` | desk/deskfpd.c:69 | m | - | - |
| `pn_selected` | desk/deskfpd.c:319 | m | - | - |
| `pn_sort` | desk/deskfpd.c:154 | m | - | - |
| `print_buf` | desk/desksupp.c:584 | - | - | - |
| `print_file` | desk/desksupp.c:617 | m | - | - |
| `printer_alert` | desk/desksupp.c:1181 | m | - | - |
| `pro_exec` | desk/deskpro.c:35 | m | - | - |
| `pro_run` | desk/deskpro.c:52 | m | - | - |
| `process_funkey` | desk/deskmain.c:1087 | m | - | DESK33/D |
| `read_inf_file` | desk/deskapp.c:680 | m | - | - |
| `rebuild_window` | desk/deskfun.c:135 | m | - | - |
| `recover_shortcuts` | desk/deskinf.c:1066 | m | - | - |
| `refresh_drive` | desk/desksupp.c:1700 | m | - | - |
| `refresh_window` | desk/desksupp.c:1670 | m | - | DESK53/P |
| `remove_all_viewers` | desk/deskins.c:359 | m | - | - |
| `remove_drive` | desk/deskins.c:263 | m | - | - |
| `remove_locate_shortcut` | desk/desksupp.c:380 | m | - | - |
| `remove_one_level` | desk/deskfun.c:236 | m | - | - |
| `retry_format` | desk/desksupp.c:1423 | - | - | - |
| `rmv_icon` | desk/deskins.c:1317 | m | - | - |
| `rsrc_free` | desk/gembind.c:845 | m | - | - |
| `rsrc_load` | desk/gembind.c:838 | m | - | A35/I |
| `rsrc_obfix` | desk/gembind.c:877 | m | - | - |
| `save_shortcut` | desk/deskinf.c:1028 | m | - | - |
| `save_to_disk` | desk/deskapp.c:1040 | m | - | - |
| `scan_str` | desk/deskapp.c:258 | m | - | - |
| `scroll_conf_funkeys` | desk/deskinf.c:957 | m | - | - |
| `search_display` | desk/deskfun.c:391 | m | - | - |
| `search_icon` | desk/deskfun.c:512 | m | - | - |
| `search_prompt` | desk/deskfun.c:319 | m | - | - |
| `search_recursive` | desk/deskfun.c:441 | m | - | - |
| `set_background` | desk/deskapp.c:1306 | m | - | - |
| `set_default_path` | desk/desksupp.c:1785 | m | - | DESK18/D |
| `set_icon` | desk/deskins.c:1012 | m | - | - |
| `set_tedinfo_name` | desk/deskinf.c:427 | m | - | - |
| `setup_iconblks` | desk/deskapp.c:454 | m | - | - |
| `setup_mono_icons` | desk/deskapp.c:565 | m | - | - |
| `shel_get` | desk/gembind.c:918 | m | - | - |
| `shel_put` | desk/gembind.c:926 | m | - | - |
| `shel_wdef` | desk/gembind.c:963 | m | - | - |
| `shel_write` | desk/gembind.c:907 | m | - | A48/P A61/D |
| `show_buf` | desk/desksupp.c:713 | - | - | - |
| `show_file` | desk/desksupp.c:771 | - | - | - |
| `snap_icon` | desk/deskins.c:146 | m | - | - |
| `sob_malloc` | desk/deskobj.c:51 | m | - | - |
| `source_is_parent` | desk/deskdir.c:850 | m | - | - |
| `start_dialog` | desk/deskinf.c:349 | m | - | - |
| `sub_path` | desk/deskdir.c:157 | m | - | - |
| `trash_alert` | desk/desksupp.c:1169 | m | - | - |
| `update_modified_windows` | desk/deskdir.c:512 | m | - | - |
| `user_abort` | desk/deskdir.c:60 | m | - | - |
| `user_input` | desk/desksupp.c:540 | m | - | - |
| `user_quit` | desk/desksupp.c:569 | - | - | - |
| `valid_drive` | desk/desksupp.c:1810 | m | - | DESK40/P |
| `w_setpath` | desk/deskfun.c:809 | m | - | DESK51/P |
| `wants_to_delete_files` | desk/deskfun.c:1433 | m | - | - |
| `win_alloc` | desk/deskwin.c:156 | m | - | - |
| `win_arrow` | desk/deskwin.c:716 | m | - | - |
| `win_bdall` | desk/deskwin.c:786 | m | - | - |
| `win_bldview` | desk/deskwin.c:392 | m | - | - |
| `win_blt` | desk/deskwin.c:553 | m | - | - |
| `win_count` | desk/deskwin.c:280 | m | - | - |
| `win_delta` | desk/deskwin.c:528 | m | - | - |
| `win_dispfile` | desk/deskwin.c:645 | m | - | - |
| `win_find` | desk/deskwin.c:198 | m | - | - |
| `win_free` | desk/deskwin.c:141 | m | - | - |
| `win_icalc` | desk/deskwin.c:380 | m | - | - |
| `win_isel` | desk/deskwin.c:825 | m | - | - |
| `win_ocalc` | desk/deskwin.c:293 | m | - | - |
| `win_onbottom` | desk/deskwin.c:247 | m | - | - |
| `win_ontop` | desk/deskwin.c:265 | m | - | - |
| `win_shwall` | desk/deskwin.c:805 | m | - | - |
| `win_sinfo` | desk/deskwin.c:867 | m | - | - |
| `win_slide` | desk/deskwin.c:682 | m | - | - |
| `win_sname` | desk/deskwin.c:846 | m | - | DESK55/P |
| `win_srtall` | desk/deskwin.c:768 | m | - | - |
| `win_start` | desk/deskwin.c:105 | m | - | - |
| `win_top` | desk/deskwin.c:218 | m | - | - |
| `win_view` | desk/deskwin.c:63 | m | - | - |
| `wind_calc_grect` | desk/gembind.c:810 | m | - | - |
| `wind_close` | desk/gembind.c:739 | m | - | A16/P |
| `wind_create_grect` | desk/gembind.c:717 | m | - | - |
| `wind_delete` | desk/gembind.c:746 | m | - | A33/P |
| `wind_find` | desk/gembind.c:795 | m | - | - |
| `wind_get` | desk/gembind.c:753 | m | - | A05/P A11/P A12/P +9 |
| `wind_get_grect` | desk/gembind.c:766 | m | - | - |
| `wind_new` | desk/gembind.c:828 | m | - | - |
| `wind_open_grect` | desk/gembind.c:728 | m | - | - |
| `wind_set` | desk/gembind.c:772 | m | - | A119/P A78/I A79/P +6 |
| `wind_set_grect` | desk/gembind.c:789 | m | - | - |
| `wind_update` | desk/gembind.c:803 | m | - | A118/I A110/D |
| `write_boot` | desk/desksupp.c:1361 | - | - | - |

### 2.13 include/ inline trap bindings (out of API scope) - 58 rows

| fn | def | spec | R | tickets |
|---|---|---|---|---|
| `bios_l_v` | include/biosbind.h:79 | m | R | - |
| `bios_l_w` | include/biosbind.h:94 | - | R | - |
| `bios_l_wl` | include/biosbind.h:127 | - | R | - |
| `bios_l_wlwwwl` | include/biosbind.h:145 | - | R | - |
| `bios_l_ww` | include/biosbind.h:110 | - | R | - |
| `bios_v_l` | include/biosbind.h:34 | - | - | - |
| `bios_v_ww` | include/biosbind.h:47 | - | - | - |
| `bios_w_w` | include/biosbind.h:63 | - | R | - |
| `divu` | include/intmath.h:110 | - | R | - |
| `dos_chdir` | include/gemdos.h:95 | m | - | - |
| `dos_chmod` | include/gemdos.h:120 | m | - | - |
| `dos_close` | include/gemdos.h:75 | m | - | - |
| `dos_conis` | include/gemdos.h:40 | m | - | - |
| `dos_conws` | include/gemdos.h:35 | m | - | - |
| `dos_create` | include/gemdos.h:110 | m | - | - |
| `dos_delete` | include/gemdos.h:125 | m | - | - |
| `dos_gdir` | include/gemdos.h:100 | i | - | - |
| `dos_gdrv` | include/gemdos.h:45 | m | - | - |
| `dos_gdta` | include/gemdos.h:55 | - | - | - |
| `dos_lseek` | include/gemdos.h:90 | m | - | - |
| `dos_mkdir` | include/gemdos.h:115 | m | - | - |
| `dos_open` | include/gemdos.h:70 | m | - | - |
| `dos_rawcin` | include/gemdos.h:30 | m | - | - |
| `dos_read` | include/gemdos.h:80 | m | - | - |
| `dos_rename` | include/gemdos.h:130 | m | - | - |
| `dos_rmdir` | include/gemdos.h:135 | m | - | - |
| `dos_sdrv` | include/gemdos.h:105 | m | - | - |
| `dos_sdta` | include/gemdos.h:50 | m | - | - |
| `dos_sfirst` | include/gemdos.h:60 | m | - | - |
| `dos_snext` | include/gemdos.h:65 | m | - | - |
| `dos_write` | include/gemdos.h:85 | m | - | - |
| `inline_strcat` | include/string.h:55 | - | - | - |
| `mul_div` | include/intmath.h:51 | m | R | - |
| `muls` | include/intmath.h:91 | m | R | - |
| `r_get` | include/obdefs.h:258 | m | R | - |
| `r_set` | include/obdefs.h:267 | m | R | - |
| `rc_copy` | include/rectfunc.h:23 | m | R | - |
| `swpcopyw` | include/asm.h:115 | m | R | - |
| `umul_shift` | include/intmath.h:73 | - | R | - |
| `xbios_l_l` | include/xbiosbind.h:354 | m | - | - |
| `xbios_l_lll` | include/xbiosbind.h:370 | m | - | - |
| `xbios_l_v` | include/xbiosbind.h:323 | m | R | - |
| `xbios_l_w` | include/xbiosbind.h:338 | m | - | - |
| `xbios_v_l` | include/xbiosbind.h:180 | m | R | - |
| `xbios_v_llw` | include/xbiosbind.h:193 | - | - | - |
| `xbios_v_llww` | include/xbiosbind.h:209 | m | R | - |
| `xbios_v_v` | include/xbiosbind.h:74 | m | R | - |
| `xbios_v_w` | include/xbiosbind.h:86 | m | - | - |
| `xbios_v_wl` | include/xbiosbind.h:99 | m | - | - |
| `xbios_v_wll` | include/xbiosbind.h:113 | m | R | - |
| `xbios_v_wwl` | include/xbiosbind.h:129 | i | R | - |
| `xbios_v_wwwl` | include/xbiosbind.h:145 | - | - | - |
| `xbios_v_wwwwww` | include/xbiosbind.h:162 | m | - | - |
| `xbios_w_llwwwww` | include/xbiosbind.h:273 | m | - | - |
| `xbios_w_llwwwwwlw` | include/xbiosbind.h:297 | m | - | - |
| `xbios_w_v` | include/xbiosbind.h:225 | m | R | - |
| `xbios_w_w` | include/xbiosbind.h:240 | m | R | - |
| `xbios_w_ww` | include/xbiosbind.h:256 | m | R | - |

### 2.14 ext: util/, cli/, libc or nowhere (out of API scope) - 58 rows

| fn | def | spec | R | tickets |
|---|---|---|---|---|
| `build_root_path` | - | m | - | - |
| `bzero_nobuiltin` | - | - | R | - |
| `coma_start` | - | m | - | - |
| `doprintf` | - | - | R | - |
| `dos_alloc_anyram` | - | m | R | - |
| `dos_alloc_stram` | - | m | - | - |
| `dos_avail_altram` | - | m | - | - |
| `dos_avail_anyram` | - | m | - | - |
| `dos_avail_stram` | - | m | - | - |
| `dos_free` | - | m | R | - |
| `dos_label` | - | m | - | - |
| `dos_load_file` | - | m | - | - |
| `dos_setdt` | - | m | - | - |
| `dos_shrink` | - | - | - | - |
| `dos_space` | - | m | - | - |
| `expand_string` | - | m | R | - |
| `extract_drive_number` | - | m | R | - |
| `filename_start` | - | m | - | - |
| `fmt_str` | - | m | - | - |
| `get_floppy_type` | - | - | - | - |
| `inf_gindex` | - | m | - | - |
| `inf_sget` | - | m | - | - |
| `inf_sset` | - | m | - | - |
| `inf_what` | - | m | - | - |
| `inside` | - | m | R | - |
| `Isqrt` | - | - | R | - |
| `longjmp` | - | m | R | T07/P |
| `memcmp` | - | - | R | - |
| `memcpy` | - | m | R | - |
| `memmove` | - | m | R | - |
| `memset` | - | m | R | - |
| `mul_div_round` | - | m | R | - |
| `pgmld` | - | m | - | - |
| `protect_v` | - | m | R | - |
| `protect_w` | - | m | R | - |
| `protect_wlwwwl` | - | m | R | - |
| `protect_ww` | - | m | R | - |
| `rc_constrain` | - | m | - | - |
| `rc_equal` | - | m | R | - |
| `rc_intersect` | - | m | R | - |
| `rc_union` | - | m | R | - |
| `scan_2` | - | m | - | - |
| `scasb` | - | m | - | - |
| `set_all_files` | - | m | - | - |
| `setjmp` | - | m | R | - |
| `sprintf` | - | m | R | - |
| `stop_until_interrupt` | - | m | R | - |
| `strcmp` | - | m | R | - |
| `strlcpy` | - | m | R | - |
| `strlen` | - | m | R | - |
| `strlencpy` | - | - | R | - |
| `strncasecmp` | - | m | R | - |
| `strncmp` | - | m | R | - |
| `toupper` | - | m | R | - |
| `trap1` | - | m | R | - |
| `trap1_pexec` | - | m | - | - |
| `unfmt_str` | - | m | - | - |
| `wildcmp` | - | m | - | - |

## 3. Cookies (data; the jar code is in 2.10)

Unit: the 22 cases of `jar-r1.tsv` (one per tag, plus `jar:*` = count and order). All 22 are measured on the pinned oracle (references `BIOS-jar`, `BIOS-jar-falcontc`, `BIOS-jar-disk`; CRC `2D0D269D`) and on NewTOS; the comparison is tabulated in `docs/COMPATIBILITY.md:1621-1647` (run `measurements/measurement-2026-10-29-tool18/RUNINFO.TXT`, named there). Status is that table's, not re-derived: **8 EQUAL, 14 ACCEPTED** (a recorded decision per row). Spec: `docs/api/bios/_cookie-jar.md`. The oracle's jar carries 12 cookies, NewTOS seeds 5 (`NTOS _MCH _IDT _AKP EdDI`, `docs/COMPATIBILITY.md:1630`).

| case | oracle expectation (`retclass` in the table) | status (`docs/COMPATIBILITY.md` line) | basis named in that row |
|---|---|---|---|
| `jar:*` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1630`) | follows from the per-cookie rows |
| `jar:_CPU` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1631`) | ADR-012 amendment 2026-10-28: no m68k CPU |
| `jar:_VDO` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1632`) | user decision 2026-10-29 (ARCH06) |
| `jar:_FPU` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1633`) | ADR-012: no m68k FPU |
| `jar:_MCH` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1634`) | ADR-012 amendment 2026-10-28 |
| `jar:_SWI` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1635`) | user decision 2026-10-29 |
| `jar:_SND` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1636`) | ADR-012: no PSG / DMA sound |
| `jar:_FRB` | PTR_RAM | ACCEPTED (`docs/COMPATIBILITY.md:1637`) | user decision 2026-10-29 |
| `jar:_FLK` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1638`) | ADR-012 amendment 2026-10-26 |
| `jar:_AKP` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1639`) | user decision 2026-10-30 (COOK02) |
| `jar:_IDT` | EXACT | EQUAL (`docs/COMPATIBILITY.md:1640`) | user decision 2026-10-30 (COOK02) |
| `jar:_FDC` | EXACT | ACCEPTED (`docs/COMPATIBILITY.md:1641`) | ADR-012: no floppy controller |
| `jar:__NF` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1642`) | - |
| `jar:XHDI` | NONZERO | ACCEPTED (`docs/COMPATIBILITY.md:1643`) | ADR-032 |
| `jar:SCSI` | PTR_RAM | ACCEPTED (`docs/COMPATIBILITY.md:1644`) | ADR-032 |
| `jar:_5MS` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1645`) | - |
| `jar:_CF_` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1645`) | - |
| `jar:_MCF` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1645`) | - |
| `jar:_NET` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1645`) | - |
| `jar:NVDI` | ABSENT | EQUAL (`docs/COMPATIBILITY.md:1645`) | - |
| `jar:NTOS` | ABSENT | ACCEPTED (`docs/COMPATIBILITY.md:1646`) | ADR-012 amendment 2026-10-28: NewTOS identity |
| `jar:EdDI` | ABSENT | ACCEPTED (`docs/COMPATIBILITY.md:1647`) | The VDI speaks EdDI (spec E2) |

Access paths: only `Setexc($5A0/4,-1)` (`13:5` case `Setexc(0x168,-1)` `_p_cookies` of `bios-core-r1.tsv`) is measured on the oracle; the `Ssystem` subset and the raw `$5A0` read have no case; a program that adds a cookie (`cookie_add`, `util/cookie.c:34`) has no case. Unknown whether NewTOS passes the cookie-insertion path of a program: no table covers it.

## 4. System variables (data)

Unit: the 83 assignments of `references/emutos/tosvars.ld` with a numeric address (`sysvars_start` and the alias lines `memvalid = _memvalid` excluded). **NewTOS counterpart** = whether the loader writes the variable into a process' page 0 at load: `P` = an explicit write of `populate_sysvars` in `tos-api/src/bdos/process/loader.rs` (read: its `wr_long`/`wr_word`/`tpa[..]` calls and the two named constants `HZ_200_ADDR`, `DUMPFLG_ADDR`), `H` = a cell of `PAGE0_HOOKS` (`tos-api/src/bdos/process/hooks.rs`), `V` = inside the shared vector cells `$000-$40F` that `write_page0_vectors` (`tos-api/src/bios/vectors.rs`) copies (`NSHARED` = 0x104 cells). `-` means **none of these three writes it**; whether another code path (timer, VBL, console) keeps it current was **not searched: unknown**. The page is zero-filled before `populate_sysvars` (`loader.rs`, `page0.fill(0)`), so a `-` variable reads 0 unless another writer exists. **oracle** = a `Setexc(n,-1)` case reads the cell at `4*n` on the pinned oracle (`13:5` lines of the tables; no other table reads low memory). **spec** = number of non-meta `docs/api/` files naming the variable. **dev** = `docs/DEVIATIONS.md` / `docs/COMPATIBILITY.md` title match.

Note: the crash variables `_proc_*` (0x380-0x3cc) lie inside the vector-cell range `$000-$40F`; `V` there only says that NewTOS writes the shared vector values at those addresses, not that it supports crash state (`docs/api/bios/_exception-crash.md` is the spec).

Counts: 83 variables; 40 written at process load by at least one of P/H/V (22 by `P`, 10 cells in `H`, rest `V` only); 43 never written by these three; 8 read on the oracle by a `Setexc` case; 81 named in a spec file.

| addr | variable | `tosvars.ld` line | NewTOS | oracle | spec | dev |
|---|---|---:|---|---|---:|---|
| 0x380 | `_proc_lives` | 23 | V | - | 2 | - |
| 0x384 | `_proc_dregs` | 24 | V | - | 2 | - |
| 0x3a4 | `_proc_aregs` | 25 | V | - | 2 | - |
| 0x3c4 | `_proc_enum` | 26 | V | - | 2 | - |
| 0x3c8 | `_proc_usp` | 27 | V | - | 2 | - |
| 0x3cc | `_proc_stk` | 28 | V | - | 2 | - |
| 0x400 | `_etv_timer` | 33 | V | Setexc | 5 | - |
| 0x404 | `_etv_critic` | 34 | V | Setexc | 4 | - |
| 0x408 | `_etv_term` | 35 | V | Setexc | 9 | - |
| 0x40c | `_etv_xtra` | 36 | V | Setexc | 2 | - |
| 0x420 | `_memvalid` | 37 | P | Setexc | 5 | - |
| 0x424 | `memctrl` | 39 | - | - | 2 | - |
| 0x426 | `resvalid` | 40 | P | - | 2 | - |
| 0x42a | `resvector` | 41 | PH | - | 3 | - |
| 0x42e | `_phystop` | 42 | P | - | 4 | C73 |
| 0x432 | `_membot` | 43 | P | - | 3 | - |
| 0x436 | `_memtop` | 44 | P | - | 2 | - |
| 0x43a | `_memval2` | 45 | P | - | 4 | - |
| 0x43e | `_flock` | 47 | - | - | 1 | - |
| 0x440 | `_seekrate` | 48 | - | - | 1 | - |
| 0x442 | `_timer_ms` | 49 | - | - | 2 | - |
| 0x444 | `_fverify` | 50 | - | - | 1 | - |
| 0x446 | `_bootdev` | 51 | P | - | 7 | - |
| 0x448 | `_palmode` | 52 | - | - | 1 | - |
| 0x44a | `_defshiftmod` | 53 | - | - | 1 | - |
| 0x44c | `_sshiftmod` | 54 | - | - | 3 | - |
| 0x44e | `_v_bas_ad` | 55 | - | - | 12 | - |
| 0x452 | `_vblsem` | 56 | - | - | 2 | - |
| 0x454 | `_nvbls` | 57 | - | - | 1 | - |
| 0x456 | `_vblqueue` | 58 | H | - | 3 | - |
| 0x45a | `_colorptr` | 59 | - | - | 2 | - |
| 0x45e | `_screenpt` | 60 | - | - | 1 | - |
| 0x462 | `_vbclock` | 61 | - | - | 1 | - |
| 0x466 | `_frclock` | 62 | - | - | 2 | - |
| 0x46a | `_hdv_init` | 63 | - | - | 1 | - |
| 0x46e | `_swv_vec` | 64 | PH | - | 4 | - |
| 0x472 | `_hdv_bpb` | 65 | H | - | 4 | - |
| 0x476 | `_hdv_rw` | 66 | H | - | 4 | - |
| 0x47a | `_hdv_boot` | 67 | - | - | 1 | - |
| 0x47e | `_hdv_mediach` | 68 | H | - | 4 | - |
| 0x482 | `_cmdload` | 69 | P | - | 4 | - |
| 0x484 | `_conterm` | 70 | P | - | 13 | - |
| 0x486 | `trp14ret` | 71 | - | - | 1 | - |
| 0x48a | `criticret` | 72 | - | - | 1 | - |
| 0x48e | `_themd` | 73 | - | - | 2 | - |
| 0x49e | `____md` | 74 | - | - | 0 | - |
| 0x4a2 | `_savptr` | 75 | - | - | 2 | - |
| 0x4a6 | `_nflops` | 76 | - | - | 1 | - |
| 0x4a8 | `_con_state` | 77 | - | - | 2 | - |
| 0x4ac | `_save_row` | 78 | - | - | 2 | - |
| 0x4ae | `sav_context` | 79 | - | - | 1 | - |
| 0x4b2 | `_bufl` | 80 | - | - | 3 | - |
| 0x4ba | `_hz_200` | 81 | P | - | 10 | - |
| 0x4be | `the_env` | 82 | - | - | 1 | - |
| 0x4c2 | `_drvbits` | 83 | P | - | 7 | - |
| 0x4c6 | `_dskbufp` | 84 | - | - | 2 | - |
| 0x4ca | `_autopath` | 85 | - | - | 1 | - |
| 0x4ce | `_vbl_list` | 86 | - | - | 1 | - |
| 0x4ee | `_dumpflg` | 87 | P | - | 3 | D46 |
| 0x4f0 | `_prtabt` | 88 | - | - | 1 | - |
| 0x4f2 | `_sysbase` | 89 | P | - | 3 | - |
| 0x4f6 | `_shell_p` | 90 | - | - | 1 | - |
| 0x4fa | `_end_os` | 91 | P | - | 2 | - |
| 0x4fe | `_exec_os` | 92 | H | - | 6 | - |
| 0x502 | `_dump_vec` | 93 | H | - | 4 | - |
| 0x506 | `_prt_stat` | 94 | P | - | 1 | C1487 |
| 0x50a | `_prt_vec` | 95 | P | - | 1 | C1487 |
| 0x50e | `_aux_stat` | 96 | - | - | 1 | - |
| 0x512 | `_aux_vec` | 97 | - | - | 1 | - |
| 0x516 | `_pun_ptr` | 98 | - | - | 2 | - |
| 0x51a | `_memval3` | 99 | P | - | 4 | - |
| 0x51e | `_bconstat_vec` | 101 | - | - | 4 | - |
| 0x53e | `_bconin_vec` | 102 | - | - | 5 | - |
| 0x55e | `_bcostat_vec` | 103 | - | - | 4 | - |
| 0x57e | `_bconout_vec` | 104 | - | - | 4 | - |
| 0x59e | `_longframe` | 105 | - | - | 1 | - |
| 0x5a0 | `_p_cookies` | 106 | P | Setexc | 3 | - |
| 0x5a4 | `_ramtop` | 107 | P | Setexc | 5 | - |
| 0x5a8 | `_ramvalid` | 108 | P | Setexc | 3 | - |
| 0x5ac | `_bell_hook` | 109 | H | - | 3 | - |
| 0x5b0 | `_kcl_hook` | 110 | H | - | 2 | - |
| 0x6fc | `_warm_magic` | 119 | - | - | 1 | - |
| 0x700 | `_pmmutree` | 128 | - | - | 0 | - |

## 5. Gap list

One row per gap. Types: **NC** no counterpart, **NS** no spec, **NM** never measured on the oracle, **DT** deviation without a live ticket, **CL** classification or documentation defect (the repository contradicts itself). "Live" = OPEN, IMPLEMENTED or PARKED. A gap already covered by a live ticket gets no new ticket (column "existing"); the proposed tickets are in 5.2 and are **not filed**. Evidence is `file:line` or table/case.

### 5.1 Gaps

| ID | layer | type | rows | evidence | existing live ticket | proposal |
|---|---|---|---:|---|---|---|
| G01 | BIOS, cookies | CL/NC | 38 | All 22 `bios/xhdi.c` rows (`docs/analysis/deps/mapping.tsv:1085-1106`, incl. `XHNewCookie`) and all 16 `bios/scsidriv.c` rows (`:894-909`) are `unmapped`; ADR-032 point 1 says the 38 functions "are `design:ADR-032` in `docs/analysis/deps/mapping.tsv`" (`docs/MEMORY.md:3156`). The `XHDI`/`SCSI` cookies are ACCEPTED (`docs/COMPATIBILITY.md:1643-1644`). | TOOL22/P (titled "TOOL17-Gruppen B, C, F einordnen"; Reopen: PROC07 is taken up) | none new; owner may lift TOOL22 |
| G02 | GEMDOS/BDOS | CL/NC | 12 | 8 DND rows of `bdos/fsdir.c` (`makdnd getdnd snipdnd freednd free_available_dnds process_dnd_tree makofd dopath`, `mapping.tsv:99-120`) and 4 `bdos/osmem.c` rows (`getosm osmem_init xmgetblk xmfreblk`, `:200-205`) are `unmapped`; ADR-033 point 3 names exactly these `design:ADR-033` (`docs/MEMORY.md:3196`). `docs/COMPATIBILITY.md:1076` records the decision. | TOOL22/P | none new; owner may lift TOOL22 |
| G03 | BIOS/XBIOS | NC/CL | 59 | 59 `unmapped` rows of `bios/` that no ADR covers and that are not the partition layer: `bios.c` 6, `initinfo.c` 10, `kprint.c` 7, `screen.c` 5, `conout.c` 4, `country.c` 4, `ikbd.c` 3, `processor.S` 3, `serport.c` 5, `delay.c` 2, `biosmem.c` 2, `startup.S` 2, `vectors.S` 2, `delayasm.S`, `panicasm.S`, `font.c`, `vt52.c` 1 each. Every row's note already says "no NewTOS counterpart" and why (`mapping.tsv:295-1106`, filter `bios/` + `unmapped`); none carries a `design:` evidence. | none (TOOL22 covers groups B, C, F only) | T01 |
| G04 | BIOS/XBIOS | NC | 28 | Partition and block-device layer: 16 `bios/disk.c` rows (`atari_partition process_gpt map_gpt_to_atari process_dos_mbr disk_rescan ...`, `mapping.tsv:434-453`) and 12 `bios/blkdev.c` rows (`blkdev_boot hd_boot_read next_logical ...`, `:321-348`), notes: "no NewTOS counterpart function found (NewTOS reads MBR partitions in bios/drive...)". Observable through `Drvmap`, `Getbpb`, `Mediach`, `Rwabs` on a card with GPT/AHDI/extended partitions. No case table names GPT or AHDI (grep of `tosdump/cases/*.tsv`); the partition cases read the MBR of the test image (`bios-r2-phys.tsv`, case P02). | none | T02 |
| G05 | AES, GEMDOS/BDOS, VDI, Line-A | NC/CL | 49 | 42 `aes/` rows (EVB/queue/process 14, `gsx_*` form/clip/workstation 14, window lists 7, misc 7), 4 `bdos/` rows (`bdos_trap2 get_default_handle log2ul namlen`), `ok_for_direct_blit` (`vdi/vdi_text.c`), `set_screen_shift` (`bios/lineainit.c`) and the API helper `inline_strcpy` (`include/string.h`, opcode 34 `menu_text`): `unmapped` in `mapping.tsv:1156-2000` (aes), notes say "no counterpart" with reasons, several "no ADR" (`aestrap_intercepted`, `xlate_obj_array`, `rect_change`). ADR-029 (`docs/MEMORY.md:2749`) is the one cooperative-kernel decision; no ADR names the EVB list or the key queue. | PROC07/P covers only `bdos_trap2`; A136/P and A109/P name `rect_change`; GUI03/P names the loadable cursors; I18N01/P the translation | T03 |
| G06 | GEMDOS | NC | 1 | `Srealloc` (0x15): table entry `bdos/bdosmain.c:201`; NewTOS has no slot, `mapping.tsv:242` ("EINVFN"); a spec exists (`docs/api/gemdos/15-srealloc.md`); no ticket title names it; `sdk/` has it only in mintlib headers (`sdk/libc/mintlib/include/mint/mintbind.h`), `programs/` not at all (grep 2026-11-11, `Applications/` not searched). | none | T04 |
| G07 | AES | NC | 4 | `sh_path` (the `PATH=` walk, step 4 of `shel_find`, `aes/gemshlib.c:356-396`; NewTOS stops after step 3, `mapping.tsv:1853`); `load_mouse_cursors`/`setup_mouse_cursors` (`CONF_WITH_LOADABLE_CURSORS`, default 1, `include/config.h:1290-1291`, file `A:\EMUCURS.RSC`); `xlate_obj_array` (gettext of OBJECT arrays; the oracle ROM is `COUNTRY=de`). `shel_find` is called by `sdk/apps/teradesk/xfilesys.c` and `sdk/apps/qed/src/rsc.c`. | GUI03/P (cursors), I18N01/P (translation), none for `sh_path` | T05 (`sh_path` only) |
| G08 | AES | NC | 2 | `ap_tplay`, `ap_trecd`: appl_tplay returns 1, appl_trecord returns 0 (`mapping.tsv` notes), `docs/DEVIATIONS.md` has no row for them. | A45/P (blocked by the AES fork queue) | none new |
| G09 | GEMDOS | NM | 39 | 39 of 54 GEMDOS entries have no case in any table: console/aux/prn 15 (`Cconin Cconout Cauxin Cauxout Cprnout Crawio Crawcin Cnecin Cconws Cconrs Cconis Cconos Cprnos Cauxis Cauxos`), file 8 (`Fopen Fread Fseek Fattrib Fdup Fforce Fdatime Frename`), directory 2 (`Dcreate Ddelete`), time 4, memory 5 (`Malloc Mfree Mxalloc Mshrink Maddalt`), process 3 (`Pterm0 Pterm Ptermres`), `Sversion`, `Srealloc` (rows in 2.3 with oracle `none`). The 15 measured are in `gemdos-mc-fs20/22/24`, `gemdos-mc-paths`, `super-r1`. The planned GEMDOS dumper does not exist (`tosdump/cases/COVERAGE-BIOS.md`: "go to `DUMPGDOS` once that exists"; TOOL03/P). | FS tickets name single calls (e.g. FS15/P `Fopen`, FS11/I `Fread`); TOOL03/P (DUMPGDOS) | T06 (console, 15), T07 (23: file, directory, time, memory, process, version) |
| G10 | AES | NM | 61 | 40 of 78 AES API nodes have no case at all (`appl_exit appl_write evnt_* menu_* objc_draw/find/offset/change form_* graf_dragbox/rubberbox/slidebox/watchbox fsel_* rsrc_load/obfix scrp_* shel_*`, rows in 2.5), 21 more only in `aes-r4-subfunctions.tsv`, whose dump is EmuTOS 1.4, not the pin (`measurements/measurement-2026-09-12-aes-r4/RUNINFO.TXT`; no CRC match in `tosdump/reference/`). The same run's `NOT-RUN.TXT` names the blockers: ALERT (modal alert hangs), GUARD, PTR (`addr_in` needs a fixture). Consequence: 151 of 290 mapped `aes/` internals are not reachable from any `O` entry (1.5). | A-tickets name single calls (parked); no ticket for the table work | T08 (the 40), T09 (promote `aes-r4` to the pinned oracle) |
| G11 | VDI | NM | 14 | `v_clsvwk` (101), `v_locator` (28), `v_string` (31), `vex_timv/butv/motv/curv/wheelv`, `vq_key_s` (128), `vq_mouse` (124), `vs_color` (14), `vsf_udpat` (112), `vst_load_fonts`/`vst_unload_fonts` (119/120): oracle `none` in 2.7. `tosdump/cases/COVERAGE.md` ledger: `v_clsvwk`, `v_locator`, `v_string` excluded, others "later". | none (V33 DONE covers fonts, not measurement) | T10 |
| G12 | XBIOS | NM | 17 | 17 mapped XBIOS entries without a case: `Bioskeys Ikbdws Initmous Mfpint Midiws Offgibit Ongibit Scrdmp Setpalette Settime Xbtimer Floprd Flopwr Flopfmt Flopver DMAread DMAwrite` (2.2). Live tickets exist for 4 (`bioskeys` XBIOS19/P, `ikbdws` XBIOS04/I, `scrdmp` TOOL35/P, `settime` XBIOS08/P) so 13 have none. | XBIOS04/I, XBIOS19/P, TOOL35/P, XBIOS08/P | T11 (the 13) |
| G13 | all layers | NM | - | Internals that are `mapped` and not reachable from any `O` entry: `aes/` 151 of 290, `bios/` 44 of 133, `vdi/` 12 of 139, `bdos/` 9 of 100, cookie code 1 of 1 (1.5, column `R`). A graph reading, not a measurement; no row of its own: it follows from G09-G12. | - | none (closes with T06-T11) |
| G14 | VDI, AES, GEMDOS | DT | 4 | `docs/DEVIATIONS.md` rows whose status says "no ticket": line 25 (AES drawing bypasses the VDI trap; side finding V35, DONE), line 33 (`vex_butv/motv/timv` before a `v_opnwk`; V53, DONE), line 34 (pointer frozen during one long system call; V55, DONE), line 57 (`Fcreate` with a name longer than 8.3, refused, `PORT12`). Each row is still listed; the ticket that found it is closed or unrelated. Row 53 (`Fclose` write-back failure) names "user decision open" under HW08 (DONE). | none live | T12 |
| G15 | all | CL | - | `docs/DEVIATIONS.md:15` says the list is not complete; it has 28 rows against 90 bold-led paragraphs (69 after dropping continuation labels such as "Observable difference") in `docs/COMPATIBILITY.md:66-1852`. Two deviations stated in `docs/MEASUREMENT-COVERAGE.md` are in neither file: `v_get_pixel` after `v_clrwk` (`0xFFDF` vs `0xFFFF`, parked 2026-09-06) and `D-VDI-25` (`vro_cpyfm` oversized destination; spec `docs/api/vdi/_discrepancies.md:44`, mentioned by `TASK/V19.md`). | none | T13 |
| G16 | all | CL | 441 | 441 nodes (`desk/` 325, `include/` 58, `ext` 58) have no `mapping.tsv` rows by construction; `cargo xtask deps --coverage` therefore shows `desk` 100% and `include` 100% unmapped. 31 `desk/` rows are named by live tickets (DESK*, section 2.12). The desktop is an EL0 program (ADR-026, `docs/MEMORY.md:2433`). | DESK* tickets for individual functions | T14 |
| G17 | all | NS/CL | - | Spec: XBIOS `nvmaccess` (0x2e) is mapped and `O` but only the inventory names it (2.2). System variables: 83 in `tosvars.ld`; 43 are written by none of `populate_sysvars`, `PAGE0_HOOKS`, `write_page0_vectors` among them `_v_bas_ad`, `_vbclock`, `_frclock`, `_hdv_init`, `_hdv_boot`, `_bconin_vec`, `_nflops`; only 8 are read on the oracle (`Setexc` cases). Whether another path keeps them current is unknown (not searched). `docs/COMPATIBILITY.md:1862-1889` promises "compatible in value and semantics" without a per-variable list. | none | T15 |
| G18 | tooling | CL | - | `cargo xtask report --coverage` does not exist (`cargo xtask report` prints ticket states and gap groups only; `tools/deps/README.md` lists `deps --coverage` for the mapping only). Nothing recomputes sections 1-4. | none live | T16 |
| G19 | docs | CL | - | `docs/INVENTORY.md` (2026-09-04) and `docs/MEASUREMENT-COVERAGE.md` (computed 2026-09-06) are hand-kept and contradict the repository in at least 8 places (section 7); `TASK/COVERAGE.md` is a third hand-kept AES coverage table. | none | T17 |

Row counts: the `unmapped` rows of the API layers, Line-A and cookie code are 189 and split without overlap into G01 38 + G02 12 + G03 59 + G04 28 + G05 49 + G06 1 + G08 2 = 189 (`include/` and `desk/` unmapped rows are G16). G07 names 4 rows that are inside G05 (`sh_path`, `load_mouse_cursors`, `setup_mouse_cursors`, `xlate_obj_array`) and is shown with 4 rows, not added again.

Gap counts: 19 gap rows (G01-G19). 4 need no new ticket: G01, G02 and G08 are covered by live tickets, G13 closes with T06-T11. 15 lead to the 17 proposed tickets.

### 5.2 Proposed tickets (not filed; 17 in total, below the owner's threshold of about 20)

The pre-start check proposal follows the ADR-037 vocabulary: OPEN, or PARKED with `Reason` and `Reopen`. A PARKED proposal is a proposal for the owner's pre-start check, not a decision; every "survey" check is the grep over `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`, `programs/`, `tosdump/`, `Applications/` and the manifest of the standard image (`tools/make-sd-image.sh`) that the instructions of INV01 prescribe: if every executable of the manifest maps to a local build recipe, the "no local source" gap is empty by construction; otherwise the executable is named.

| Ticket | title | layer | gaps | priority | pre-start check proposal |
|---|---|---|---|---|---|
| T01 | `mapping.tsv`: give the 59 unmapped `bios/` helper rows a `design:` evidence or a gap decision | BIOS/XBIOS (doc) | G03 | P3 | PARKED. Reason: table maintenance, same as TOOL22 (owner pre-start check 2026-10-31). Reopen: TOOL22 is taken up (do both in one pass). |
| T02 | Partition and block-device layer: measure GPT/AHDI/extended partitions against the oracle, or classify `bios/disk.c` + `bios/blkdev.c` (28 rows) as design | BIOS | G04 | P3 | OPEN with a check: survey which partition schemes the card images and `tools/mktestdisk.sh` produce and which `Drvmap`/`Getbpb` answers a card with another scheme gives; if no card image uses GPT/AHDI, PARKED. Reason: no card uses them. Reopen: a card image with GPT/AHDI/extended partitions. |
| T03 | `mapping.tsv`: classify 49 unmapped `aes/`, `bdos/`, `vdi/`, Line-A rows (EVB list, key queue, FDB/clip helpers, window lists, stragglers) | AES, GEMDOS, VDI (doc) | G05 | P3 | PARKED. Reason: table maintenance, same as TOOL22. Reopen: TOOL22 is taken up. Needs an owner decision per cluster (ADR vs gap): EVB/queue rows against ADR-029. |
| T04 | `Srealloc` (GEMDOS 0x15): provide or sanction the missing slot | GEMDOS | G06 | P3 | PARKED after the survey. Reason (to confirm by the survey): no program of `sdk/` or `programs/` calls it, only mintlib headers name it. Reopen: a program calls `Srealloc`. |
| T05 | `shel_find`: the `PATH=` step (`sh_path`) | AES | G07 | P3 | OPEN with a check: do `sdk/apps/teradesk/xfilesys.c` and `sdk/apps/qed/src/rsc.c` depend on a PATH hit? Measure `shel_find` against the oracle first (opcode 124 has no case). |
| T06 | GEMDOS console, aux and printer calls: oracle case table | GEMDOS | G09 | P3 | OPEN with a check: which of the 15 can run unattended (`Cconin`, `Cnecin`, `Crawcin` block on input: scripted keystrokes in Hatari or excluded with the reason, as `tosdump/cases/COVERAGE-BIOS.md` does). |
| T07 | GEMDOS file, directory, time, memory, process, version calls: oracle case tables (23) | GEMDOS | G09 | P2 | OPEN. Check: DUMPBIOS can issue `1:0xNN` calls (as `gemdos-mc-*.tsv` do), so no new dumper is needed; Hatari run required, listed as the first step. |
| T08 | AES tables for trees, menus, forms, resources, shell, scrap, events: fixture column in the case format, then the 40 entries | AES | G10 | P2 | OPEN. Check: the fixture mechanism ("PTR" in `NOT-RUN.TXT` of the aes-r4 run) is a dumper change; the ticket starts with that decision. |
| T09 | Promote `aes-r4-subfunctions.tsv` to a reference at the pin (Hatari run with the oracle ROM) and compare | AES | G10 | P3 | OPEN. Check: the run needs Hatari and the pinned ROM (`docs/analysis/ORACLE.md`); not done here. |
| T10 | VDI: the 14 entries without a case (`vs_color`, `vsf_udpat`, fonts, `vq_*`, `vex_*`, `v_clsvwk`) | VDI | G11 | P3 | PARKED. Reason (to confirm in the check): `tosdump/cases/COVERAGE.md` lists the blocking ones as excluded and no differing program is recorded; Reopen: a program or port shows a difference on one of the 14. The three "safe to add" entries named there (`vsf_udpat`, `vst_load_fonts`, `vst_unload_fonts`) can be OPEN. |
| T11 | XBIOS: oracle cases for the 13 mapped entries without a ticket | XBIOS | G12 | P3 | PARKED. Reason: XBIOS16 and XBIOS19 were parked with "no program calls" and XBIOS08-XBIOS10 with "until a port needs it"; Reopen: a program calls one of them. The floppy and DMA entries (6) answer an error code and are cheap to add. |
| T12 | Decide the four `DEVIATIONS.md` rows with no live ticket: sanction in `docs/COMPATIBILITY.md` or ticket | VDI, AES, GEMDOS | G14 | P3 | OPEN (decision for the owner; no code). Rows 25, 33, 34, 57. |
| T13 | `docs/DEVIATIONS.md`: index the sanctioned deviations of `docs/COMPATIBILITY.md` (about 69 entries) and the two from `MEASUREMENT-COVERAGE.md` | docs | G15 | P3 | OPEN (doc only, mechanical; fits the tooling of T16). |
| T14 | `mapping.tsv` / `deps --coverage` scope: `desk/`, `include/`, `ext` (441 nodes) | tooling | G16 | P3 | OPEN. Check: the owner decides the class for `desk/` (design:ADR-026?), the inline trap bindings of `include/` and `ext`; otherwise the coverage tool reports 100% unmapped for them forever. |
| T15 | System variables: per-variable contract table (served / hardware / missing) and the `nvmaccess` spec section; measure by `Supexec` read cases | BIOS, docs | G17 | P3 | OPEN. Check: the 43 variables not written by the three loader paths; first step is a read of the other writers (timer, VBL, console), which this inventory did not do. |
| T16 | `cargo xtask report --coverage`: compute sections 1-4 of this snapshot | tooling | G18 | P2 | OPEN (tooling slot, INV01 item 4); queries in section 6. |
| T17 | Retire `docs/INVENTORY.md`, `docs/MEASUREMENT-COVERAGE.md`, `TASK/COVERAGE.md` as hand-kept coverage tables | docs | G19 | P4 | PARKED. Reason: they are history until T16 exists. Reopen: `report --coverage` lands. |

Count: 17 proposed tickets; 6 are proposals for PARKED (T01, T03, T04, T10, T11, T17), 11 for OPEN (T02 may turn PARKED after its survey). Priority P2: T07, T08, T16; P3: T01-T06 and T09-T15 (13); P4: T17.

### 5.3 Top 10 gaps by impact (the lead's report repeats this list)

1. **G09, GEMDOS 39 of 54 entries never measured** (T06, T07): GEMDOS, which every program uses, has 15 entries with oracle cases, all of them through the media-change, `Super` and path tables.
2. **G10, AES 40 entries never measured, 21 only against EmuTOS 1.4** (T08, T09): includes `form_do`, `form_alert`, `menu_bar`, `objc_draw`, `rsrc_load`, `shel_write`; 151 of 290 mapped `aes/` internals are unreached by any measured entry.
3. **G17, 43 of 83 system variables written by none of the three loader paths** (T15): `_v_bas_ad` ($44E) is one of the addresses the contract names (`docs/COMPATIBILITY.md:1864`); `_vbclock`, `_frclock`, `_nflops`, `_bconin_vec` are in the same set; whether another writer exists is unknown.
4. **G04, partition layer: 28 rows without counterpart, no GPT/AHDI case** (T02): decides which cards boot.
5. **G14, 4 deviations persist with no live ticket** (T12), among them `Fcreate` refusing names longer than 8.3 where EmuTOS truncates (`docs/DEVIATIONS.md:57`), visible to any program that passes a long name.
6. **G18, no `report --coverage`** (T16): without it every number here goes stale; this document is already a snapshot.
7. **G11, VDI 14 entries never measured** (T10), among them `vs_color`, `vsf_udpat`, `vq_mouse`.
8. **G07, `shel_find` stops after step 3** (T05): TeraDesk and qed call `shel_find`.
9. **G03, 59 BIOS helper rows without evidence class** (T01): not a behaviour gap, but it keeps `deps --coverage` at 16% unmapped for `bios/`.
10. **G12, 13 XBIOS entries without case or ticket** (T11), mostly parked-by-habit calls (`Initmous`, `Setpalette`, `Midiws`, `Mfpint`).

## 6. Queries for `cargo xtask report --coverage`

What a tool must compute so it replaces this snapshot. Every query below was run once by script for this document, on the inputs named; the tool must reproduce sections 1-4 byte for byte on the same inputs (then it is checked against this file as its first test).

1. **Node set.** Every name in `docs/analysis/deps/edges.tsv` columns 1 and 2 (def rows, edge ends): 2076. Rows per node: `def` sites, `layer` of the `def` (`ext` without a `def`). Check `sum(group counts) == |nodes|` and `every def row belongs to exactly one row` (that is what makes section 1 add up).
2. **Group of a node** (first match): API = target of a `table` edge from `osif`, `biostrap`, `xbiostrap`, `screen`, `crysbind`, plus `enter` for GEMDOS 0x20 (an `asm` edge `enter -> termuser` at `bdos/rwa.S:172/184`; today a name rule, because the `x20_*` labels are no nodes); Line-A = the `asm` targets of `linea_0` plus `int_linea`, `linea_init`, `set_screen_shift` and the direct helpers (today a name rule: `linea_*`, `get_pix`, `put_pix`, `lineA2Attrib`); cookie code = name contains `cookie`; else the directory. The tool should read the Line-A and cookie sets from the graph (the `linea_0` edges) and fail on a node it cannot place.
3. **Mapping class per node.** Join `mapping.tsv` per definition (fn + file), then per node: unmapped if any definition is, else mapped if any is, else design; `ext` = no `def`. Output the counts of section 1.1 and the `design` evidence histogram (that part exists: `cargo xtask deps --coverage`).
4. **Spec presence per API slot.** For slot `s` of layer `L`: `F` if `docs/api/<L>/<hex or decimal of s>-*.md` exists (hex for BIOS/XBIOS/GEMDOS, decimal for VDI); else `H` if a heading of a non-meta `docs/api` file names the function or `(<s>)`; else `T` if a table row of a non-meta file of that layer names it in backticks; else `I` if only `_full-function-inventory.md` names it. Meta files: `_full-function-inventory.md`, `_dispatcher.md`, `_state.md`, `_triage.md`, `_working-list.md`, `_dependency-graph.md`, `_discrepancies.md`, `_emutos-config.md`, `_phase00-completion.md`, `_template.md`. **Better:** a `Covers:` line in each spec file (e.g. `Covers: GEMDOS 0x3d`) that the tool reads; the rules above are a stop-gap. The API name per slot needs a machine-readable source (today: AES `case` labels in `aes/gemsuper.c`, VDI handler names, the inventory tables; BIOS/XBIOS/GEMDOS names only in the inventories).
5. **Measured per API slot.** Parse every `tosdump/cases/*.tsv`: skip empty lines, `#` comments, `!` directives; `+` lines are preludes (not cases); first column is `1:`/`13:`/`14:` + function (`int(x, 0)`) for DUMPBIOS tables, an opcode for `aes-*` and `vdi-*` tables, `jar:<TAG>` for the jar table. Bind each table to its dump: CRC32 of the file bytes equals the `CRC` printed in the second line of a `tosdump/reference/EMUTOS-1a7484ab-*.LOG` (`<N> cases, CRC <hex>`); `O` if one exists, else `R14`/`none`. The `R14` link (`aes-r4`) has no machine-readable form today: it needs an index file listing table, dump path, ROM sha256 (the tool must not guess it from `measurements/` names). Also check `N cases` of the LOG equals the non-prelude line count (it does for all 39 bound tables).
6. **Reach.** Closure over `direct`, `table`, `asm`, `layer`, `vector` edges (the default of `cargo xtask deps`) from every API node with an `O` slot; report per group the `mapped` internals not in the closure. This is a graph reading, never to be presented as a measurement (it already exists as `cargo xtask deps <fn> --up`; the tool needs it as a set operation).
7. **Deviations.** Parse the trigger column of `docs/DEVIATIONS.md` and the bold title of each paragraph of `docs/COMPATIBILITY.md` lines 66-1852 and match function/API names as words. Today free text: add a `Functions` column to `DEVIATIONS.md` and a `Functions:` line per sanctioned deviation, otherwise the match understates (section 0.2). Output: rows named in a deviation; deviation rows whose status has no live ticket (`no ticket` in the Status column, or only DONE/OBSOLETE tickets).
8. **Tickets.** `TASK/<ID>.md` line 1 (title) and line 3 (ADR-037 state; `cargo xtask report` already parses line 3). Match titles by word as in 0.2; better a `Covers:` header line naming functions, so a ticket can declare what it closes. Per row: live tickets (OPEN/IMPLEMENTED/PARKED) first; OBSOLETE ignored. Gap rule: a gap row is **covered** only by a live ticket; DONE/ACCEPTED tickets do not cover an open gap.
9. **Gap rows** (what section 5 lists by hand): `unmapped` rows of the API layers, Line-A and cookie code; API rows whose spec level is `I` or `-` and whose class is `mapped`; API rows of class `mapped` with oracle `none`/`R14`; deviation rows without a live ticket; mapping rows whose ADR cite contradicts the class (an ADR that says `design:ADR-NNN` while the row is `unmapped`: G01, G02).
10. **Cookies.** Cases of `jar-r1.tsv`; status from a machine-readable form of the table `docs/COMPATIBILITY.md:1630-1647` (today prose with a markdown table).
11. **System variables.** Parse `references/emutos/tosvars.ld` (assignments with a hex address); NewTOS writers from the explicit calls of `populate_sysvars`, `PAGE0_HOOKS` and `write_page0_vectors` (today code; a table in code that both the loader and the tool read would remove the regex); oracle reads from `13:5` cases with `l:-1` (address = 4 * the WORD). A future `peek` case type would measure the other variables.
12. **Drift guard.** `report --coverage --check docs/analysis/inventory-2026-11-11.md` prints the first differing count; the snapshot then is a test fixture, not a document.

## 7. Differences against `docs/INVENTORY.md` and `docs/MEASUREMENT-COVERAGE.md`

Both are inputs to compare, not sources. Unit differences first: `docs/INVENTORY.md` counts API functions (GEMDOS 54, BIOS 12, XBIOS about 103, VDI 67, AES 79), this document counts graph nodes and, separately, slots; the slot counts agree where the unit agrees (GEMDOS 54 slots with a function = 53 table slots + `Super`; BIOS 12; XBIOS 103; AES 79 `case` labels = 78 opcodes with a node + `graf_handle` inline; VDI 74 slots = 67 + 7 no-op slots served by `just_rts`: `jmptb1` 0x4, 0xa, 0x1b, 0x1d, 0x22 and `jmptb2` 0x84, 0x85, `vdi/vdi_main.c:32-111`).

### 7.1 What they claim that the repository does not support (now)

| # | claim | where | what the repository shows now |
|---|---|---|---|
| 1 | BIOS: "0 von 12" entries with a running proof | `docs/INVENTORY.md:1108` | all 12 BIOS entries have recorded oracle cases (2.1; tables `bios-r1`, `bios-core-r1`, `bios-r2*`, references `BIOS`, `BIOS-core`, `BIOS-r2-*-disk`). |
| 2 | XBIOS `Supexec` returns 38 and never runs the callback | `docs/INVENTORY.md:536` | `docs/analysis/deps/mapping.tsv:1004` maps `supexec` to `kernel/src/arch/aarch64/exceptions.rs::handle_supexec` ("the callback runs in the super state ... since MEM12 S6"); `super-r1.tsv` has three `Supexec` cases (`14:0x26`) with oracle reference `SUPER`. |
| 3 | `Super` (0x20) missing, no table entry, answers `EINVFN` | `docs/INVENTORY.md:390` | `enter` is mapped (`mapping.tsv`, row `enter`), the spec exists (`docs/api/gemdos/process/20-super.md`), 9 oracle cases (`super-r1.tsv`, reference `SUPER`), and `kernel/src/arch/aarch64/exceptions.rs` handles `Super` in the trap handler (comment "Super (0x20) - the super state (MEM12", `kernel/src/arch/aarch64/exceptions.rs:424@handle_super`). |
| 4 | `Pexec` modes 5-7 unsupported (`UnsupportedMode`) | `docs/INVENTORY.md:384` | `tos-api/src/bdos/process/pexec.rs` documents `PE_BASEPAGE` (5), `PE_GOTHENFREE` (6), `PE_BASEPAGEFLAGS` (7) as implemented (ARCH03 stage 2) and defines their constants; no case table has a `Pexec` mode 5-7 case (by code, not measured). |
| 5 | `Iorec` and `Bioskeys` are missing | `docs/INVENTORY.md:538-539` | `mapping.tsv` rows `iorec` (to `ta/xbios/iorec.rs::iorec`) and `bioskeys` (to `ta/xbios/keytbl.rs::bioskeys`) are `mapped`; `Iorec` has oracle cases (`xbios-flush-r1`), `Bioskeys` none (G12). `Initmous` is as claimed: "the call itself is not implemented" (`mapping.tsv` note). |
| 6 | 19 of 67 VDI opcodes have no reference case, the raster group among them | `docs/INVENTORY.md:1092` | 60 of 74 VDI slots have oracle cases (2.7, 1.3); the raster group (109, 110, 121) is measured (`vdi-raster*.tsv`, references `RASTER*`); 14 slots have none (G11). |
| 7 | "70 of 243" wired API functions with a running proof | `docs/INVENTORY.md:1104` | not comparable (different unit and a different proof class: host tests); on the oracle axis the counts are in 1.3: 12 + 47 + 15 + 16 (+ `graf_handle`) + 60 slots with `O`. |
| 8 | VDI 65 wired / 47 measured / 18 never; AES 79 / 6 / 73 | `docs/MEASUREMENT-COVERAGE.md:14-15` | VDI: 74 slots, 60 `O`, 14 none; of the doc's 18, four are now measured (109, 110, 121 and `v_clswk` 2, case 22 of `vdi-v18-t3.tsv`); the other 14 are the 14 of G11. AES: 78 opcodes with a node, `O` 16 (+ opcode 77), `R14` 19, none 43 (1.3). |
| 9 | AES round 2 "written and built but **not yet run**" | `docs/MEASUREMENT-COVERAGE.md:17-19` | `tosdump/reference/EMUTOS-1a7484ab-AES-r2wind.BIN`/`.LOG` exist (78 cases, CRC `AFB11C9F` = `aes-r2-wind.tsv`); the seven window opcodes are `O` (2.5: `wind_create/open/close/delete/set/update/new`). |
| 10 | measured "on a real Atari TT" | `docs/MEASUREMENT-COVERAGE.md:3` | the references are Hatari runs on the pinned ROM (`docs/analysis/ORACLE.md`); the TT-era `EMUTOS14-*` dumps remain as calibrated history (ORACLE.md section 4). |
| 11 | `v_get_pixel` after `v_clrwk` (0xFFDF vs 0xFFFF) and `D-VDI-25` are known deviations | `docs/MEASUREMENT-COVERAGE.md` ("known real deviations" 2, 3) | neither is a row of `docs/DEVIATIONS.md` or a paragraph title of `docs/COMPATIBILITY.md` (G15). |

### 7.2 What the repository supports that they lack

- The oracle binding by CRC (every table but `aes-r4` is bound to a pinned dump) and the pin itself; neither document names a reference per case.
- A per-function class for all 2076 graph nodes (915 mapped, 524 design with evidence, 572 unmapped, 65 ext), against 79 + 67 + 12 + 54 + 103 hand-counted API entries; helper functions, drivers and boot code are invisible in the hand-kept documents.
- Cookies (22 cases, 8 EQUAL / 14 ACCEPTED, `docs/COMPATIBILITY.md:1621-1647`) and system variables (83, 40 with a loader writer): neither document has them.
- Ticket states under ADR-037 (680 tickets: 18 OPEN, 73 IMPLEMENTED, 183 DONE, 297 PARKED, 4 ACCEPTED, 105 OBSOLETE, `cargo xtask report`), against the `Open` / `blocked` text of the two documents. 297 PARKED tickets carry an owner reason and a Reopen condition; most of the gaps in section 5 are known to the owner as parked work.
- Four more hand-kept ledgers exist and were not re-derived: `tosdump/cases/COVERAGE.md` (VDI), `COVERAGE-AES.md`, `COVERAGE-BIOS.md` and `COVERAGE-GEMDOS.md` (a trace ledger of NewTOS-side TeraDesk runs, `TERADESK-GDTRACE-r1/r2`, no oracle counterpart), plus `TASK/COVERAGE.md`. The ledgers' statuses ("round 1", "later", "excluded") are consistent with sections 1.3 and G09-G12 where read; the GEMDOS trace instrument records live runs and is not counted as a measurement here.

## 8. Unknowns, limits, out-of-scope observations

### 8.1 Not determined (needs a run, a reading outside this scope, or an owner decision)

- Whether NewTOS equals the oracle on any recorded case (needs a NewTOS run and `tosdiff`; none was made). Counts here say "an oracle case exists".
- Whether `aes-r4-subfunctions.tsv` (181 cases) gives the same dump at the pin as on EmuTOS 1.4 (needs a Hatari run, T09).
- Probe and `measurements/` results per function (552 run directories; not attributed).
- Whether another code path keeps the 43 unwritten system variables current (not searched, T15).
- Which executables of the standard image use any unmapped function: the manifest mapping to build recipes (`tools/make-sd-image.sh`) was not done; the program greps cited in G06 and G07 are partial (`Applications/` not searched). The PARKED proposals depend on it.
- Spec correctness against the source, and the NewTOS side of every `mapped` row (read from `mapping.tsv`, not re-read).
- Whether the nodes reached by `R` actually run in a measured case (a graph reading).

### 8.2 Out-of-scope observations (one line each, nothing acted on)

- `layers.md` "Counts" says 2068 nodes and its per-layer lists omit 8 nodes that have a `def` row in `edges.tsv` (7 `include/` inlines and `ikbdraw`); `deps --coverage` counts 2014 definitions and 1952 "defined in the node dirs": three numbers for one graph.
- The graph has no edge into `ikbdraw` although `bios/aciavecs.S:391` calls it (`jsr ikbdraw`): an unresolved assembler call.
- ADR-032 point 1 states a mapping change (`design:ADR-032` for 38 functions) that `mapping.tsv` does not contain (G01).
- `docs/DEVIATIONS.md` and many tickets mix German words ("gemessen", "gelesen", "offen", "geparkt") into an English repository (the language rule of CLAUDE.md).
- `TASK/COVERAGE.md` and `GAPS-INDEX-2026-09-12.md` are in `TASK/` but have no ADR-037 line 3 (not tickets; `cargo xtask report` counts 680 files).
- `tosdump/reference/` holds no dump for `aes-r4` although the run exists under `measurements/measurement-2026-09-12-aes-r4/` (the table is not in the reference set that `tosdiff` would find).


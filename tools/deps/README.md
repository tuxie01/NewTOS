# tools/deps — the EmuTOS dependency graph (Track B)

The call and data dependencies of EmuTOS `bios/`, `bdos/`, `vdi/`, `aes/` and
`desk/` at the oracle pin (`docs/analysis/ORACLE.md` §1), and their mapping
onto NewTOS. Generated, queried, never drawn by hand.

| File | What | Made by |
|---|---|---|
| `docs/analysis/deps/edges.tsv` | nodes (`def` rows) and edges | `tools/deps/gen-deps.py` |
| `docs/analysis/deps/layers.md` | counts, layer matrix, tables, traps, vector slots, everything not resolved | `tools/deps/gen-deps.py` |
| `docs/analysis/deps/mapping.tsv` | EmuTOS function → NewTOS function (n:m) and its class, `bios/` + `bdos/` | by hand, checked |

```
tools/deps/gen-deps.py            # rewrite edges.tsv and layers.md
tools/deps/gen-deps.py --check    # regenerate in a temp dir, compare byte for byte
cargo xtask deps <fn> --down|--up [--kinds k,..|+k,..|all] [--depth N] [--layers l,..]
cargo xtask deps --check-mapping  # what the mapping test checks, as a report
cargo xtask deps --coverage       # mapped/design/unmapped per layer
```

`--down` lists what `<fn>` needs to be right first, `--up` who is affected
when `<fn>` is wrong, each function with the edge it was reached over and its
NewTOS counterpart from `mapping.tsv`, then every `table` and `layer` edge
inside the set (the trap entries and vector calls crossed). Default kinds
are `direct,table,asm,layer,vector`; `callback` and `data` are opt-in
(`--kinds +data`): both connect nearly everything — the trap handlers are
installed by boot code, and `run` is read by hundreds of functions.
Each function also carries its class (below), and the report ends with the
coverage of the set and its verdict (see "Coverage and the ratchet").

## Inputs and reproducibility

Everything comes from `tools/oracle/oracle.env`. The generator checks that
`references/emutos-oracle/<pin[:8]>/` is at the pinned commit, clean, and
holds the built `etos512de.img` with `ORACLE_ROM_SHA256`; it only reads that
tree. The compile command of every source is taken from `make -n` of the
recipe of `tools/oracle/build-emutos.sh` (`make 512 COUNTRY=de`), run in a
throw-away clone: `make -n` still remakes `makefile.dep` and its generated
prerequisites, so it never runs in the build tree. The generator stops if the
script no longer contains that recipe line. All sources compile with one flag
set (`-m68000 -mshort -O2 … -Iinclude -DWITH_AES=1 -DWITH_CLI=1
-DTARGET_512`), recorded in both output headers together with the commit, the
ROM sha256, the cross compiler and cflow versions. The addresses of the
system variables come from `tosvars.ld`, preprocessed by the recipe's own
`-P -x c` command for `emutos.ld` (which includes it).

The eleven `bug`-translated sources (`*.tr.c`) are analysed as their
untranslated `.c`, so sites cite the source the spec cites; the generator
proves on every run that both give the same token stream once string
literals are blanked.

Intermediate files (the clone, preprocessed sources, gimple dumps) live in a
directory under `$TMPDIR` that is removed on exit.

Outputs are sorted and carry no time stamp and no absolute path. Two runs on
the same inputs are byte-identical; `--check` verifies that against the
committed files.

## Nodes

A node is a function name. Names are merged: two static functions of the same
name, or a static inline function of a header compiled into several files, are
one node with one `def` row per definition (`layers.md` lists them). `def`
rows carry the layer (the directory of the definition; `include` for header
inlines). A name without a `def` row is `ext`: defined in `util/`, `cli/`,
libgcc, or nowhere. `util/` is not a node directory; its assembler files are
read only to find trap bindings (`trap1`, `trap1_pexec`).

## Edge kinds — how each is derived, what it misses

Two sources per C file: **cflow** over the preprocessed file (GNU cflow 1.8,
one run over all node directories, `-A -A -d 2`), and the cross compiler's
**gimple dump** (`-fdump-tree-gimple-lineno`, same flags): one statement per
line, exact scoping, a source line on every statement, taken before any
inlining.

### `direct` — a call

cflow lists caller → callee. gimple decides what it is: a call (`direct`), or
the function only used as a value (moved to `callback`). Sites are the first
call line from gimple.

Misses and marks:
- `detail cflow-only`: cflow lists it, gimple has no call — calls gcc folds
  while gimplifying (`memcpy`/`strlen` of constant size), a `goto` label cflow
  takes for a function (`xrw -> eof`). Kept; listed in `layers.md`.
- `detail gimple-only`: gimple has the call, cflow does not list it — cflow
  lists `biosmain` and `xterm` (both declared `NORETURN`) without callees, and
  gives the static inline functions of headers (`dos_*` in `include/gemdos.h`)
  no callee list at all. Kept; listed in `layers.md`.
- Calls through function pointers are not edges. cflow reports `fp(...)` through
  a global function-pointer variable as a call of the variable; those pseudo
  calls are dropped (listed in `layers.md`); the read of the variable is a
  `data` fact.
- gcc's own vararg IL (`VA_ARG`, `__builtin_va_*`) is no call.

### `table` — a dispatch-table slot

The five dispatch tables of the user decision, configured in `TABLES` of the
generator: `funcs[]` (`bdos/bdosmain.c`, trap #1), `bios_vecs[]` and
`xbios_vecs[]` (`bios/bios.c`, `bios/xbios.c`, traps #13/#14), `jmptb1[]` and
`jmptb2[]` (`vdi/vdi_main.c`, trap #2). The initializer is read from the
preprocessed source (so `#if CONF_*` slots are as built), one entry per
top-level element; the slot is `base + index`. The dispatcher is every function
that reads the table (gimple) or names it (assembler): `osif`, `biostrap`,
`xbiostrap`, `screen`. The base is read from the C dispatcher's index
expression (`funcs[fn]` → 0, `jmptb1[opcode - 1]` → 1); for the two BIOS
tables, indexed in assembler (`bios/vectors.S` `biosxbios`: function number ×
4 + table base), it is stated as 0 in `TABLES` — the one number not
machine-read.

The AES dispatch `crysbind` (`aes/gemsuper.c`) is a `switch` on its
parameter `opcode`, configured in `TRAP2` of the generator: every function
called in a case — reached over the gimple control flow (labels, gotos,
fall-through into the next case) without leaving the source lines of the
switch statement — is a slot `crysbind[case]` (layers.md, "Dispatch tables").
Those calls stay `direct` edges as well.

Misses:
- Dispatch that is neither a table nor that switch: the VDI escape
  sub-functions (`esctbl[]`, `vdi/vdi_esc.c`) and the VT52 tables are `vector`
  edges; the Line-A table (`bios/linea.S` `linea_vecs`) stays `asm` edges of
  the region it sits in, `linea_0`.
- A case that calls no function (`GRAF_HANDLE` only copies globals) is a slot
  without an edge; a trap call to it is listed in `layers.md`.
- GEMDOS 0x20 `Super` is answered by `enter` before the table is consulted; the
  table says `funcs[0x20]` = `ni`, and so does the graph.

### `callback` — an address handed on

`from` takes the address of `to` (gimple: the function used as a value — a
vector install, a handler passed to a driver), or `from` reads a variable
whose initializer holds `to` (C initializers from the preprocessed source,
`.dc.l` lists under a data label in assembler; detail `via <variable>`).

Misses: the edge says "`from` hands `to` on", not who finally calls it.
Initializers that nothing in the node directories reads give no edge
(`layers.md` lists them, e.g. `scsidriv_root_init`).

### `asm` — assembler

Each `.S` file is preprocessed like the build does, then cut into regions: a
region starts at a code label that is `.globl` or carries the C prefix `_`
(`_aestrap` is local but is the trap #2 vector) and runs to the next one;
other labels belong to the region they sit in. Edges: a reference to a known
function (`jsr`, `jbsr`, `jra`, `lea`, `move.l #_f`, `.dc.l`), a branch to a
label of another region (detail `<mnemonic> <label>`), and the fall-through
into the next region when the last instruction is not `rts`/`rte`/`jra`/`jmp`
(detail `fallthrough`).

Misses: computed jumps (`jsr (a0)`, the table calls themselves — those are the
`table` kind); assembler reads and writes of C globals (no `data` edges from
assembler); a jump into another region's local label makes the whole region
the target.

### `layer` — a trap from one layer into another

A **binding** is a function that executes `trap #N`: a C function with inline
asm (`bios_l_w`, `xbios_w_v`, `gem`, `gsx2`, …), or an assembler region
(`trap1`, `trap1_pexec` in `util/miscasm.S`). The opcode is the last word
pushed before the trap: for inline asm the operand `%K` is followed back to
the function parameter it names (so `bios_l_w(int op, …)` takes it from
argument 1); in assembler a pushed immediate (`move.w #0x4b,-(sp)`, `clr.w
-(sp)`) is a fixed opcode, no push at all passes the caller's arguments
through (`trap1`). Every gimple call site of a binding whose opcode argument
is a constant gives an edge caller → the function in that slot of the trap's
table (detail `trap #N 0xOP table[0xOP] via binding`); assembler trap sites
give owner → slot function directly. The trap → table association in `TABLES`
is checked on every run: a handler installed on that trap vector (gimple
`*(PFVOID*)0xb4 = biostrap`, a `Setexc` call resolved to the BIOS slot
`setexc`, assembler `move.l #_aestrap,0x88`) must reach the table's
dispatcher over `direct`/`asm` edges, or the generator stops.

**Trap #2** (`TRAP2` in the generator). AES and VDI pass the opcode in a
parameter block: d0 selects the class (`gem` loads 200 = AES, `gsx2` 0x73 =
VDI — read from the binding; an installed trap #2 handler must compare d0
with that value and reach the class's dispatch, or the generator stops), d1
points to the block, whose control array holds the opcode in element 0
(stated: the AES/VDI parameter-block ABI, `OP_CODE` = `control[0]`; checked:
the binding's file stores `&control` / `&contrl` into a block field). The
generator follows the opcode in gimple, statement by statement: a constant
stored into element 0 (`contrl[0] = 8`, or through a pointer from `&control`),
a parameter stored there (`gsx_ncode(code, …)`, with the operations applied:
`gem_if0` stores `(ctrlcode >> 9) + 10`), and functions that pass a parameter
on (`gem_if`, `gsx_0code`, `gsx_1code`: "carriers", listed in `layers.md`).
Every call site where the opcode becomes a constant gives an edge caller →
the function(s) in the slot: `jmptb1`/`jmptb2` for VDI, the `crysbind` case
for AES; detail `trap #2 0xOP <dispatch>[0xOP] via <function called>`
(`appl_init → ap_init`, `trap #2 0xa crysbind[0xa] via gem_if`).

Misses:
- trap #2 control flow is not followed inside a function: values stored on
  two branches both count (`gsx_attr` stores 0 = FALSE when no attribute
  changes; that opcode has no slot and is listed in `layers.md`). A call of a
  function that may issue a trap #2 itself ends the values pending in the
  caller (what it leaves in element 0 is not followed).
- A non-constant opcode is listed in `layers.md`, not guessed.
- Vector installs through a computed address (`move.l #_aestrap,(a0)`) are not
  seen; the absolute form next to it is.

### `vector` — a call through a variable holding a function address

A **slot** is a variable (`hdv_rw`), a field (`kbdvecs.mousevec`), or any
element of an array (`bconstat_vec[]`; indices are not told apart); a
`static` variable carries its file (`cntl_tab@bios/vt52.c[]`). An edge runs
from the function that calls through the slot to every function stored into
it, detail `via <slot>`, site = the call. `layers.md` lists each slot with
its callers, handlers and (for a system variable) address.

Stores (handlers), all in the node dirs:
- gimple: an assignment of a function address to the slot (`hdv_rw =
  blkdev_rwabs`), or of the value of another slot (`bconstat_vec[i] =
  bconstat_init[i]`: the handlers of `bconstat_init[]` flow on);
- C initialisers: every function in an array's initialiser (`V[]`), the
  function a scalar is initialised with;
- `Setexc(n, f)` and `*(PFVOID*)addr = f` with `4 * n` / `addr` ≥ 0x400: the
  system variable at that address (`Setexc(0x100, tick_int)` → `etv_timer`;
  names and addresses from `tosvars.ld`); below 0x400 is an exception vector,
  entered by the CPU (traps: `layer` edges), not called through;
- assembler, with the address registers tracked inside a region and reset at
  every label: `move.l #_f,<slot>` and `move.l aN,<slot>` with `aN` from
  `lea _f,aN`; `<slot>` is a symbol (`_hdv_rw`, `_etv_timer.w`, a local data
  label), a system-variable address, or `label-_base(aN)` with `aN` from `lea
  _base,aN`; `.dc.l` lists under a data label.

Calls through a slot:
- gimple: a call through a temporary or local whose value was read from the
  slot (`con_state.1_2 = con_state; con_state.1_2 (_4);`), or returned by
  `Setexc(n, …)` (the vector before the call: `xterm` calls `etv_term`; the
  VDI timer saves the old `etv_timer` in `tim_chain`, so the handlers of
  `etv_timer` flow into `tim_chain` — the handler it installs itself
  included, the order not followed);
- gimple: the slot's value passed to a function that calls its argument: the
  assembler `protect_v`/`protect_w`/`protect_ww`/`protect_wlwwwl`
  (`CALLS_ARGUMENT` of the generator, stated from their prototypes in
  `include/asm.h`, checked to be regions with a computed `jsr`), and C
  functions whose gimple calls through one of their parameters (read);
  `lrwabs` → `blkdev_rwabs` via `hdv_rw` this way;
- assembler: `jsr (aN)` / `jmp (aN)` where `aN` was loaded from a slot
  (`move.l _etv_timer.w,a0`) in the same region since its last label.

A C field of an assembler structure is the local label of that name in the
file defining the structure: `kbdvecs.mousevec` (C) and `mousevec:` under
`_kbdvecs:` (`bios/aciavecs.S`) are one slot. That is a name match, valid
because the KBDVECS layout is declared field by field there.

Misses (each listed in `layers.md`):
- a slot called through but never stored into with a function address in
  the node dirs (`next_handler` of XHDI, set from a cookie);
- stores of a value that is no function address: what a program installs at
  run time (`vex_butv` → `user_but`), a value from a pointer (`maptabptr->
  Bconstat` into `bconstat_vec[]`), a register computed in assembler; their
  handlers are unknown;
- C calls through a value not read from a variable: the `funcs[]`/`jmptb`
  dispatch (`table` edges of `osif`/`screen`), NatFeats through
  `natfeat_cookie`, a pointer into a structure (`bconmap_root.maptab[i]`);
- assembler computed calls whose register was not loaded from a slot since
  the last label: table dispatch (`xbiostrap`, `int_linea`), loops over a
  list (`vblqueue` in `int_vbl`), the blitter's jump threads, return jumps;
- `Setexc` from assembler with the handler in a register (`takeerr` installs
  `criterr_handler` into `etv_critic` through `call_err`, `aes/gemdosif.S`).

### `data` — shared globals, writer → reader

From gimple, per function and global variable: a **write** is an assignment
to the variable (or an element or field of it), a write through a pointer
whose value came from the variable (flow-insensitive: a temporary or local
assigned from it anywhere in the function), or its address passed in a call
(`setjmp(&errbuf)`); a **read** is every other use. An edge runs from every
writer to every other reader; detail is the variable, `*name` when the writer
only writes through a pointer from it. A `static` variable is one variable per
file. Direction for queries: a reader depends on its writers, so `--down`
follows `data` edges backwards and `--up` forwards.

Misses and over-reach:
- Writes through a pointer parameter are attributed to the callee's own
  pointer origin only, not to the caller's variable.
- Assembler reads and writes, and accesses through absolute addresses (the
  `VEC_*` and `0x…` macros) are not seen; system variables declared as
  `extern` symbols (`hz_200`, Line-A variables) are.
- Flow-insensitive pointer origins over-approximate.

## The mapping (`mapping.tsv`)

One row per EmuTOS function × NewTOS function: `emutos_fn emutos_file
newtos_file newtos_fn class note`. Every row is a reading of both sources —
the NewTOS doc comment that cites the EmuTOS function, or the code — not a
name match. A NewTOS function may sit anywhere under `tos-api/src/` or
`kernel/src/` (the VT52 console is `tos-api/src/vdi/escape.rs`, XBIOS
`tos-api/src/xbios/`).

### Classes (TOOL17)

| `class` | newtos columns | meaning |
|---|---|---|
| `mapped` | file and `fn` | the counterpart |
| `design:<evidence>` | `-` `-` | no counterpart on purpose; the note says why |
| `unmapped` | `-` `-` | no counterpart yet, or none found; the note says what is known |

`<evidence>` is one of:

- `ADR-NNN` — an ADR of `docs/MEMORY.md` that decides it (ADR-017 no
  runtime resolution change, ADR-030 no Line-A, ADR-031 hooks not served,
  ADR-012 cookies of absent hardware);
- `hardware:<device>` — the function drives, probes or sets up Atari
  hardware the Pi does not have (`scsi`, `floppy`, `dsp56001`,
  `video-shifter`, `m68k-cpu`, …; lowercase, `a-z 0-9 -`; CLAUDE.md
  non-goals: no hardware emulation). A TOS API entry (a trap-table slot)
  is never `hardware:` by itself: the call exists even where the device
  does not, so a slot without counterpart is `unmapped` until an ADR
  decides it; so are software helpers that serve only such a slot (the DSP
  LOD loader);
- `source:<path>` — another named reason, stated in the repo file
  `<path>`.

Everything without evidence is `unmapped`, including "no NewTOS function
found" and counterparts that exist in NewTOS but have no row yet (the AUX
serial path, the boot sequence). `cargo xtask deps --coverage` lists the
design rows by evidence.

### Coverage and the ratchet

A definition (`fn` + file, one `def` row) takes the class of its rows:
`unmapped` if any row is, else `mapped` if any row is, else `design`. A
definition without a row is `unmapped`: the layers the mapping does not cover
yet (`vdi`, `aes`, `desk`, `include`) count as unmapped, and so does `ext`
(no `def` row) inside a query's set. `--coverage` prints the three counts
and shares per layer.

A `--up`/`--down` report prints the same table for its set, then the
**touched layers** — the layers of `<fn>`, or the list given with `--layers`
— and every unmapped function of those layers in the set. The set is
**complete** only if that list is empty. Ratchet (CLAUDE.md "Testing"):
every fix adds the rows of the functions it touches; a retest set that is
not complete names its unmapped functions in the ticket.

### Checks

`cargo xtask test` (`deps::tests::mapping_rows_exist_on_both_sides`) checks:
the EmuTOS function is a `def` row of `emutos_file`; the NewTOS file exists
and defines `fn <newtos_fn>`; a `-` row has a reason; every `bios/` and
`bdos/` function of `edges.tsv` has a row; the class parses, `mapped` names a
NewTOS function and `design`/`unmapped` do not, an `ADR-NNN` is a
`## ADR-NNN:` heading of `docs/MEMORY.md`, a `source:` path exists.
Regenerating `edges.tsv` after a pin move therefore turns the mapping red
where it no longer fits — fix the rows by reading, `cargo xtask deps
--check-mapping` lists them.

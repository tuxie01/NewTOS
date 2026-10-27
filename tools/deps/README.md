# tools/deps — the EmuTOS dependency graph (Track B)

The call and data dependencies of EmuTOS `bios/`, `bdos/`, `vdi/`, `aes/` and
`desk/` at the oracle pin (`docs/analysis/ORACLE.md` §1), and their mapping
onto NewTOS. Generated, queried, never drawn by hand.

| File | What | Made by |
|---|---|---|
| `docs/analysis/deps/edges.tsv` | nodes (`def` rows) and edges | `tools/deps/gen-deps.py` |
| `docs/analysis/deps/layers.md` | counts, layer matrix, tables, traps, everything not resolved | `tools/deps/gen-deps.py` |
| `docs/analysis/deps/mapping.tsv` | EmuTOS function → NewTOS function (n:m), `bios/` + `bdos/` | by hand, checked |

```
tools/deps/gen-deps.py            # rewrite edges.tsv and layers.md
tools/deps/gen-deps.py --check    # regenerate in a temp dir, compare byte for byte
cargo xtask deps <fn> --down|--up [--kinds k,..|+k,..|all] [--depth N]
cargo xtask deps --check-mapping  # what the mapping test checks, as a report
```

`--down` lists what `<fn>` needs to be right first, `--up` who is affected
when `<fn>` is wrong, each function with the edge it was reached over and its
NewTOS counterpart from `mapping.tsv`, then every `table` and `layer` edge
inside the set (the trap entries crossed). Default kinds are
`direct,table,asm,layer`; `callback` and `data` are opt-in (`--kinds +data`):
both connect nearly everything — the trap handlers are installed by boot
code, and `run` is read by hundreds of functions.

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
ROM sha256, the cross compiler and cflow versions.

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

Misses:
- Dispatch that is not a table: the AES `crysbind` is a `switch` (its cases are
  plain `direct` edges, without the opcode), the VDI escape sub-functions, the
  Line-A table (`bios/linea.S` `linea_vecs`, a local label: its `.dc.l` entries
  are `asm` edges of the region it sits in, `linea_0`).
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

Misses:
- trap #2: AES and VDI pass the opcode in a parameter block, not on the stack.
  `gem` and `gsx2` get one edge to every installed trap #2 handler (`aestrap`,
  `bdos_trap2`, `vditrap`), detail `not resolved`; the per-opcode step is the
  `crysbind` switch / the VDI `jmptb` table below them.
- A non-constant opcode is listed in `layers.md`, not guessed (none at the pin).
- Vector installs through a computed address (`move.l #_aestrap,(a0)`) are not
  seen; the absolute form next to it is.

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
newtos_file newtos_fn note`. `-` `-` plus a note says there is no NewTOS
counterpart and why. Every row is a reading of both sources — the NewTOS doc
comment that cites the EmuTOS function, or the code — not a name match. A
NewTOS function may sit anywhere under `tos-api/src/` or `kernel/src/`
(the VT52 console is `tos-api/src/vdi/escape.rs`, XBIOS `tos-api/src/xbios/`).

`cargo xtask test` (`deps::tests::mapping_rows_exist_on_both_sides`) checks:
the EmuTOS function is a `def` row of `emutos_file`; the NewTOS file exists
and defines `fn <newtos_fn>`; a `-` row has a reason; every `bios/` and
`bdos/` function of `edges.tsv` has a row. Regenerating `edges.tsv` after a
pin move therefore turns the mapping red where it no longer fits — fix the
rows by reading, `cargo xtask deps --check-mapping` lists them.

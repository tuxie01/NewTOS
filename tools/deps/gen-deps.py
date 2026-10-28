#!/usr/bin/env python3
r"""gen-deps.py -- the EmuTOS dependency graph at the oracle pin (Track B).

usage:
  tools/deps/gen-deps.py           write docs/analysis/deps/edges.tsv and layers.md
  tools/deps/gen-deps.py --check   regenerate into a temporary directory and
                                   compare byte for byte with the committed
                                   files (exit 1 on any difference)

Inputs, all named by tools/oracle/oracle.env (never by $PATH):
  * the pinned EmuTOS build tree references/emutos-oracle/<pin[:8]>/, built by
    tools/oracle/build-emutos.sh (`make 512 COUNTRY=de`); checked: HEAD is the
    pin, the tree is clean, etos512de.img has ORACLE_ROM_SHA256. It is only
    read: every compiler run writes to a temporary directory.
  * the compile command of every source, taken from `make -n` of that recipe
    in a throw-away clone (never in the build tree: `make -n` still remakes
    makefile.dep and its generated prerequisites).
  * the cross compiler ORACLE_CROSS_MINT (gcc 7.5.0): `-E` for preprocessing,
    `-fdump-tree-gimple-lineno` for the per-statement facts below.
  * GNU cflow 1.8 (/opt/homebrew/bin/cflow) over the preprocessed sources,
    one run over bios+bdos+vdi+aes+desk.
  * tosvars.ld (the system-variable addresses), preprocessed by the recipe's
    own `-P -x c` command for the linker script.

How each edge kind is derived and what it misses: tools/deps/README.md.
Outputs are sorted and carry no time stamp or absolute path: two runs on the
same inputs give byte-identical files.
"""

import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ORACLE_ENV = REPO / "tools/oracle/oracle.env"
BUILD_SCRIPT = REPO / "tools/oracle/build-emutos.sh"
OUT_DIR = REPO / "docs/analysis/deps"
CFLOW = "/opt/homebrew/bin/cflow"
GENERATOR = "tools/deps/gen-deps.py"

# The directories whose functions are graph nodes (user decision 2026-10-26).
DIRS = ("bios", "bdos", "vdi", "aes", "desk")
# Read only for trap bindings (trap1, trap1_pexec live in util/miscasm.S);
# their functions are no graph nodes.
BINDING_ONLY_DIRS = ("util",)

# The make invocation of tools/oracle/build-emutos.sh. The generator refuses
# to run if the script no longer contains this line, so a changed recipe
# cannot silently diverge from the graph.
MAKE_RECIPE_LINE = "make 512 COUNTRY=de"
MAKE_ARGS = ["512", "COUNTRY=de"]

# Dispatch tables (user decision: funcs[] in bdosmain.c, the BIOS/XBIOS/VDI
# tables). name -> (defining file, trap number, index base or None).
# None: the base is read from the C dispatcher's index expression
# (`funcs[fn]` -> 0, `jmptb1[opcode - 1]` -> 1) and must exist.
# The two BIOS tables are indexed in assembler (bios/vectors.S `biosxbios`:
# the function number from the stack, times 4, added to the table base), base
# 0; that number is not machine-read and is stated here.
TABLES = {
    "funcs": ("bdos/bdosmain.c", 1, None),
    "bios_vecs": ("bios/bios.c", 13, 0),
    "xbios_vecs": ("bios/xbios.c", 14, 0),
    "jmptb1": ("vdi/vdi_main.c", 2, None),
    "jmptb2": ("vdi/vdi_main.c", 2, None),
}
TRAP_NAMES = {1: "GEMDOS", 2: "AES/VDI", 13: "BIOS", 14: "XBIOS"}
KINDS = ("direct", "table", "callback", "asm", "layer", "vector", "data")

# Trap #2 (TOOL12): AES and VDI pass the opcode in a parameter block, not on
# the stack. binding -> (class, opcode array, dispatch). Machine-read and
# checked on every run: the d0 value the binding loads (AES 200, VDI 0x73),
# an installed trap #2 handler that compares d0 with it and reaches the
# dispatch, and a store of `&<opcode array>` into a parameter-block field in
# the binding's file. Stated, not read: element 0 of the array is the opcode
# (the AES/VDI parameter-block ABI: AES `OP_CODE` = control[0],
# include/aesdefs.h; VDI `opcode = contrl[0]` in screen(), vdi/vdi_main.c).
# The AES dispatch is a `switch` on the named parameter of the named function;
# the VDI dispatch is the two tables of TABLES.
TRAP2 = {
    "gem": ("AES", "control", ("switch", "aes/gemsuper.c", "crysbind", "opcode")),
    "gsx2": ("VDI", "contrl", ("tables", ("jmptb1", "jmptb2"))),
}

# Assembler functions that call the function pointer passed as an argument
# (TOOL12, `vector` edges): name -> argument index. Stated from their
# prototypes (`LONG protect_v(LONG (*func)(void))`, include/asm.h); checked
# on every run to be an assembler region that does a computed `jsr`.
CALLS_ARGUMENT = {"protect_v": 0, "protect_w": 0, "protect_ww": 0, "protect_wlwwwl": 0}

# The linker script fragment naming the TOS system variables (fixed
# addresses 0x380-0x800); preprocessed as the build does (`-P -x c` of
# emutos.ld, which includes it).
SYSVARS_LD = "tosvars.ld"

C_KEYWORDS = set("""
auto break case char const continue default do double else enum extern float
for goto if inline int long register restrict return short signed sizeof
static struct switch typedef union unsigned void volatile while _Bool
__inline__ __inline __attribute__ __asm__ __asm asm __volatile__ __const__
__extension__ __restrict__ __restrict __signed__ __typeof__ typeof __label__
__builtin_va_list MEM
""".split())


def die(msg):
    sys.stderr.write(f"gen-deps: {msg}\n")
    sys.exit(1)


def run(cmd, cwd=None, env=None, check=True):
    p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE,
                       stderr=subprocess.PIPE, text=True, errors="replace")
    if check and p.returncode != 0:
        die(f"`{' '.join(map(str, cmd))}` failed ({p.returncode}):\n{p.stderr[-4000:]}")
    return p


# --------------------------------------------------------------------------
# A. the pin

def read_env():
    env = {}
    for line in ORACLE_ENV.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        k, _, v = line.partition("=")
        env[k] = v
    for k in ("ORACLE_CROSS_MINT", "ORACLE_EMUTOS_COMMIT", "ORACLE_ROM_SHA256"):
        if k not in env:
            die(f"{ORACLE_ENV} lacks {k}")
    return env


def verify_tree(env):
    commit = env["ORACLE_EMUTOS_COMMIT"]
    tree = REPO / "references/emutos-oracle" / commit[:8]
    if not (tree / ".git").exists():
        die(f"no pinned build tree {tree} (tools/oracle/build-emutos.sh)")
    head = run(["git", "-C", str(tree), "rev-parse", "HEAD"]).stdout.strip()
    if head != commit:
        die(f"{tree} is at {head}, the pin is {commit}")
    if run(["git", "-C", str(tree), "status", "--porcelain"]).stdout.strip():
        die(f"{tree} has local changes")
    rom = tree / "etos512de.img"
    if not rom.exists():
        die(f"{rom} missing: the tree is not built")
    sha = run(["shasum", "-a", "256", str(rom)]).stdout.split()[0]
    if sha != env["ORACLE_ROM_SHA256"]:
        die(f"{rom} sha256 {sha} != ORACLE_ROM_SHA256")
    if MAKE_RECIPE_LINE not in BUILD_SCRIPT.read_text():
        die(f"{BUILD_SCRIPT} no longer runs `{MAKE_RECIPE_LINE}`; update MAKE_ARGS")
    return tree


# --------------------------------------------------------------------------
# B. compile commands from `make -n`

def compile_commands(env, tree, tmp):
    """{source: [flags]} for every `-c` line of `make -n` of the recipe."""
    clone = tmp / "clone"
    run(["git", "clone", "-q", "--no-checkout", str(tree), str(clone)])
    run(["git", "-C", str(clone), "checkout", "-q", env["ORACLE_EMUTOS_COMMIT"]])
    cc = env["ORACLE_CROSS_MINT"] + "gcc"
    menv = dict(os.environ)
    menv["PATH"] = "/usr/bin:/bin:/usr/sbin:/sbin"
    p = run(["make", "-n", *MAKE_ARGS, f"TOOLCHAIN_PREFIX={env['ORACLE_CROSS_MINT']}",
             "BUILD_TOOLS_OPTFLAGS=-O -arch x86_64"], cwd=clone, env=menv)
    cmds = {}
    ldcmd = None
    for line in p.stdout.splitlines():
        words = line.split()
        if words and words[0] == cc and "-E" in words and "emutos.ld" in words \
                and "-o" in words:
            # the linker script's preprocessing; its output goes to stdout here
            j = words.index("-o")
            ldcmd = words[:j] + words[j + 2:]
            continue
        if not words or words[0] != cc or "-c" not in words:
            continue
        i = words.index("-c")
        src = words[i + 1]
        flags = words[1:i] + [w for w in words[i + 2:]]
        if "-o" in flags:
            j = flags.index("-o")
            del flags[j:j + 2]
        cmds[src] = flags
    if not cmds:
        die("`make -n` printed no compile command")
    if ldcmd is None:
        die("`make -n` printed no preprocessing command for emutos.ld")
    return cc, cmds, ldcmd


SYSVAR_LINE = re.compile(r"^\s*(_?[A-Za-z]\w*)\s*=\s*(0x[0-9a-fA-F]+|\d+)\s*;")


def sysvars(ldcmd, tree, tmp):
    """{C name: address} of the system variables: the `_name = 0x...;`
    assignments of tosvars.ld, as the preprocessed emutos.ld holds them."""
    text = run(ldcmd, cwd=tree, env=cc_env(tmp)).stdout
    if SYSVARS_LD not in (tree / "emutos.ld").read_text():
        die(f"emutos.ld no longer includes {SYSVARS_LD}")
    out = {}
    for line in text.splitlines():
        m = SYSVAR_LINE.match(line)
        if m and m.group(1).startswith("_"):
            out[m.group(1)[1:]] = int(m.group(2), 0)
    if "hdv_rw" not in out or "etv_timer" not in out:
        die(f"no system variables read from {SYSVARS_LD}")
    return out


def select_sources(cmds, tree):
    """(graph sources, binding-only sources): {path-as-analysed: flags}.

    A `bug`-translated source (`x.tr.c`) is analysed as its untranslated
    `x.c`; check_translated() proves the two differ only in string literals.
    """
    graph, binding, translated = {}, {}, []
    for src, flags in cmds.items():
        d = src.split("/", 1)[0]
        path = src
        if src.endswith(".tr.c"):
            path = src[:-5] + ".c"
            translated.append((src, path, flags))
        if not (tree / path).exists():
            die(f"source {path} missing in {tree}")
        if d in DIRS:
            graph[path] = flags
        elif d in BINDING_ONLY_DIRS and path.endswith(".S"):
            binding[path] = flags
    return graph, binding, translated


# --------------------------------------------------------------------------
# C. tokenizer (preprocessed C and gimple)

TOKEN_RE = re.compile(r"""
  (?P<str>"(?:[^"\\]|\\.)*")
| (?P<chr>'(?:[^'\\\n]|\\.)*')
| (?P<id>[A-Za-z_]\w*(?:\.[0-9]+(?:_[0-9]+)?)?)
| (?P<num>\.?[0-9](?:[eEpP][+-]|[0-9A-Za-z_.])*)
| (?P<op>->|\+\+|--|<<=|>>=|<<|>>|<=|>=|==|!=|&&|\|\||[-+*/%&|^]=|\.\.\.|[^\s])
""", re.X | re.S)


def tokens(text):
    return [m.group(0) for m in TOKEN_RE.finditer(text)]


def is_ident(t):
    return bool(t) and (t[0].isalpha() or t[0] == "_")


# --------------------------------------------------------------------------
# D. preprocessed C: file-scope declarations

LINEMARK = re.compile(r'^# (\d+) "([^"]*)"')


def c_tokens_with_lines(text):
    """[(token, file, line)] of a preprocessed C file."""
    out = []
    cur_file, cur_line = "?", 1
    for raw in text.split("\n"):
        m = LINEMARK.match(raw)
        if m:
            cur_line, cur_file = int(m.group(1)), m.group(2)
            continue
        if raw.startswith("#"):
            cur_line += 1
            continue
        for t in tokens(raw):
            out.append((t, cur_file, cur_line))
        cur_line += 1
    return out


def match_close(toks, i):
    """Index of the bracket closing toks[i] ((, [ or {)."""
    pairs = {"(": ")", "[": "]", "{": "}"}
    open_t = toks[i][0]
    close_t = pairs[open_t]
    depth = 0
    for j in range(i, len(toks)):
        t = toks[j][0]
        if t == open_t:
            depth += 1
        elif t == close_t:
            depth -= 1
            if depth == 0:
                return j
    return len(toks) - 1


def split_top(toks, sep=","):
    parts, cur, depth = [], [], 0
    for tk in toks:
        t = tk[0]
        if t in "([{":
            depth += 1
        elif t in ")]}":
            depth -= 1
        if t == sep and depth == 0:
            parts.append(cur)
            cur = []
        else:
            cur.append(tk)
    parts.append(cur)
    return parts


def strip_groups(toks, words):
    """Drop `word ( ... )` groups (attributes, asm labels)."""
    out, i = [], 0
    while i < len(toks):
        if toks[i][0] in words and i + 1 < len(toks) and toks[i + 1][0] == "(":
            i = match_close(toks, i + 1) + 1
            continue
        out.append(toks[i])
        i += 1
    return out


class CFile:
    def __init__(self, path):
        self.path = path
        self.fndefs = {}         # name -> (file, line)
        self.protos = set()
        self.vars = {}           # name -> (static?, file, line)
        self.typedefs = set()
        self.inits = {}          # var -> initializer tokens [(tok, file, line)]
        self.bodies = {}         # fn name -> body tokens (text only)
        self.body_toks = {}      # fn name -> body tokens [(tok, file, line)]
        self.arrays = set()      # variables declared as arrays


def parse_c(path, text):
    cf = CFile(path)
    toks = c_tokens_with_lines(text)
    stmt, depth = [], 0
    for tk in toks:
        t = tk[0]
        stmt.append(tk)
        if t == "{":
            depth += 1
        elif t == "}":
            depth -= 1
            if depth == 0:
                if fn_def(cf, stmt):
                    stmt = []
        elif t == ";" and depth == 0:
            decl(cf, stmt[:-1])
            stmt = []
    return cf


def fn_def(cf, stmt):
    """stmt ends with the `}` of a depth-0 brace group; a function body?"""
    # first depth-0 `{`
    depth, first = 0, None
    for k, tk in enumerate(stmt):
        if tk[0] in "([":
            depth += 1
        elif tk[0] in ")]":
            depth -= 1
        elif tk[0] == "{" and depth == 0:
            first = k
            break
    if first is None:
        return False
    head = strip_groups(stmt[:first], {"__attribute__", "__asm__", "__asm"})
    if any(tk[0] == "=" for tk in head) or not head or head[-1][0] != ")":
        return False
    if head[0][0] == "typedef":
        return False
    # find the `(` matching the last `)`
    depth = 0
    for k in range(len(head) - 1, -1, -1):
        if head[k][0] == ")":
            depth += 1
        elif head[k][0] == "(":
            depth -= 1
            if depth == 0:
                break
    if k == 0 or not is_ident(head[k - 1][0]) or head[k - 1][0] in C_KEYWORDS:
        return False
    name = head[k - 1]
    cf.fndefs.setdefault(name[0], (name[1], name[2]))
    cf.bodies[name[0]] = [tk[0] for tk in stmt[first:]]
    cf.body_toks[name[0]] = stmt[first:]
    return True


def decl(cf, stmt):
    stmt = [tk for tk in stmt if tk[0] != "__extension__"]
    if not stmt:
        return
    if stmt[0][0] == "typedef":
        for part in split_top(stmt[1:]):
            name = declarator_name(part)
            if name:
                cf.typedefs.add(name[0])
        return
    static = any(tk[0] == "static" for tk in stmt[:6])
    for part in split_top(stmt):
        dtoks, init = part, None
        for k, tk in enumerate(part):
            if tk[0] == "=":
                # `=` at depth 0 of this part
                d = 0
                for tk2 in part[:k]:
                    if tk2[0] in "([{":
                        d += 1
                    elif tk2[0] in ")]}":
                        d -= 1
                if d == 0:
                    dtoks, init = part[:k], part[k + 1:]
                    break
        dtoks = strip_groups(dtoks, {"__attribute__", "__asm__", "__asm"})
        kind_name = declarator_kind(dtoks)
        if kind_name is None:
            continue
        kind, name = kind_name
        if kind == "proto":
            cf.protos.add(name[0])
            continue
        prev = cf.vars.get(name[0])
        site = (name[1], name[2])
        if prev is None or (init is not None):
            cf.vars[name[0]] = (static or (prev[0] if prev else False), *site)
        if init is not None:
            cf.inits[name[0]] = init
        if any(tk[0] == "[" for tk in drop_tags_and_bodies(dtoks)):
            cf.arrays.add(name[0])


def drop_tags_and_bodies(toks):
    out, i = [], 0
    while i < len(toks):
        t = toks[i][0]
        if t in ("struct", "union", "enum"):
            i += 1
            if i < len(toks) and is_ident(toks[i][0]):
                i += 1
            if i < len(toks) and toks[i][0] == "{":
                i = match_close(toks, i) + 1
            continue
        if t == "{":
            i = match_close(toks, i) + 1
            continue
        out.append(toks[i])
        i += 1
    return out


def declarator_kind(dtoks):
    """('var'|'proto', name-token) or None."""
    d = drop_tags_and_bodies(dtoks)
    while d and d[-1][0] == "]":
        # drop trailing [..]
        depth = 0
        for k in range(len(d) - 1, -1, -1):
            if d[k][0] == "]":
                depth += 1
            elif d[k][0] == "[":
                depth -= 1
                if depth == 0:
                    break
        d = d[:k]
    if not d:
        return None
    if d[-1][0] == ")":
        depth = 0
        for k in range(len(d) - 1, -1, -1):
            if d[k][0] == ")":
                depth += 1
            elif d[k][0] == "(":
                depth -= 1
                if depth == 0:
                    break
        if k > 0 and is_ident(d[k - 1][0]) and d[k - 1][0] not in C_KEYWORDS:
            return ("proto", d[k - 1])
        if k > 0 and d[k - 1][0] == ")":
            inner = [tk for tk in d[:k] if is_ident(tk[0]) and tk[0] not in C_KEYWORDS]
            return ("var", inner[-1]) if inner else None
        return None
    ids = [tk for tk in d if is_ident(tk[0]) and tk[0] not in C_KEYWORDS]
    if len(ids) < 1:
        return None
    # a lone type name (`int;`, `WORD;`) declares nothing; with one identifier
    # it is the declarator only if a type keyword precedes it
    if len(ids) == 1 and not any(tk[0] in C_KEYWORDS for tk in d):
        return None
    return ("var", ids[-1])


def declarator_name(part):
    k = declarator_kind(strip_groups(part, {"__attribute__"}))
    return k[1] if k else None


# --------------------------------------------------------------------------
# E. preprocessed assembler

ASM_LABEL = re.compile(r"^\s*([A-Za-z_.$][\w.$]*):(.*)$")
ASM_GLOBL = re.compile(r"^\s*\.glob(?:a)?l\s+([\w.$]+)")
ASM_SYM = re.compile(r"(?<![\w.$])(_[A-Za-z]\w*)")
ASM_TRAP = re.compile(r"^\s*trap\s+#(\d+)")
ASM_PUSH = re.compile(r"^\s*(move|clr|pea|movem)(?:\.([bwl]))?\s+(.*?)-\((?:sp|a7)\)\s*$")
ASM_VEC = re.compile(r"^\s*move\.l\s+#_?([A-Za-z]\w*)\s*,\s*(0x[0-9a-fA-F]+|[0-9]+)(?:\.[wl])?\s*$")
ASM_CMP_D0 = re.compile(r"^\s*cmpi?\.w\s+#(\S+?)\s*,\s*d0\s*$")
NO_FALLTHROUGH = {"rts", "rte", "rtr", "jra", "bra", "jmp", "jbra", "illegal"}


def c_name(sym):
    return sym[1:] if sym.startswith("_") else sym


class AsmFile:
    def __init__(self, path):
        self.path = path
        self.globals = {}        # C-visible name -> (file, line)
        self.refs = []           # (owner, symbol, mnemonic, line)
        self.locals_ref = []     # (owner, label, mnemonic, line)
        self.label_owner = {}    # label -> owner
        self.traps = []          # (owner, trap, opcode|None|'pass', line)
        self.vectors = []        # (vector number, handler, line)
        self.fallthrough = []    # (owner, next owner, line)
        self.data_labels = {}    # variable label in .data/.bss -> (file, line)
        self.local_data = set()  # data labels without the C prefix `_`
        self.inits = defaultdict(list)   # data label -> [(symbol, line)]
        self.events = []         # ("label", label, line) | ("insn", owner, mnem, operands, line)
        self.d0_cmps = []        # (owner, value, line): `cmp.w #value,d0`


def parse_asm(path, text):
    """Regions, references, traps and vector installs of one preprocessed .S.

    A region starts at a code label that is `.globl` or carries the C prefix
    `_` (EmuTOS names C-visible symbols `_name`; `_aestrap` is local but is
    installed as the trap #2 vector) and runs to the next such label. Other
    labels belong to the region they sit in. Labels in .data/.bss are
    variables: a `.dc.l` list under one is that variable's initializer.
    """
    af = AsmFile(path)
    lines = []
    cur_file, cur_line = path, 1
    globls = set()
    for raw in text.split("\n"):
        m = LINEMARK.match(raw)
        if m:
            cur_line, cur_file = int(m.group(1)), m.group(2)
            continue
        if raw.startswith("#"):
            cur_line += 1
            continue
        code = raw.split("//", 1)[0].rstrip()
        lines.append((code, cur_file, cur_line))
        g = ASM_GLOBL.match(code)
        if g:
            globls.add(g.group(1))
        cur_line += 1
    owner = None
    in_text = True
    data_label = None
    last_mnem = None
    pushes_since_label = []
    pending_push = None   # last push before a trap
    for code, f, ln in lines:
        if f != path:
            continue
        m = ASM_LABEL.match(code)
        while m:
            label, code = m.group(1), m.group(2)
            if not in_text:
                data_label = c_name(label)
                af.data_labels.setdefault(data_label, (f, ln))
                if not label.startswith("_"):
                    af.local_data.add(label)
            elif label in globls or label.startswith("_"):
                new_owner = c_name(label)
                if owner is not None and last_mnem is not None and last_mnem not in NO_FALLTHROUGH:
                    af.fallthrough.append((owner, new_owner, ln))
                owner = new_owner
                af.globals.setdefault(owner, (f, ln))
                last_mnem = None
                pushes_since_label = []
                pending_push = None
            if in_text:
                af.label_owner[label] = owner
                af.events.append(("label", label, ln))
            m = ASM_LABEL.match(code)
        s = code.strip()
        if not s:
            continue
        if s.startswith("."):
            sec = re.match(r"\.(text|data|bss)\b|\.section\s+\.?(\w+)", s)
            if sec:
                name = sec.group(1) or sec.group(2)
                in_text = name == "text"
                last_mnem = None
                data_label = None
            elif re.match(r"\.(dc\.l|long)\b", s):
                for sym in ASM_SYM.findall(s):
                    if in_text and owner is not None:
                        af.refs.append((owner, c_name(sym), "dc.l", ln))
                    elif not in_text and data_label is not None:
                        af.inits[data_label].append((c_name(sym), ln))
            continue
        if not in_text:
            continue
        mnem = s.split()[0].lower()
        base_mnem = mnem.split(".")[0]
        last_mnem = base_mnem
        if owner is None:
            continue
        af.events.append(("insn", owner, mnem, s[len(mnem):].strip(), ln))
        c = ASM_CMP_D0.match(s)
        if c:
            try:
                af.d0_cmps.append((owner, int(c.group(1), 0), ln))
            except ValueError:
                pass
        t = ASM_TRAP.match(s)
        if t:
            trap = int(t.group(1))
            if pending_push is not None:
                op = pending_push
            elif not pushes_since_label:
                op = "pass"
            else:
                op = None
            af.traps.append((owner, trap, op, ln))
            pending_push = None
            continue
        p = ASM_PUSH.match(s)
        if p:
            kind, size, src = p.group(1), p.group(2), p.group(3).rstrip(", ")
            val = None
            if kind == "clr" and size == "w":
                val = 0
            elif kind == "move" and size == "w" and src.startswith("#"):
                try:
                    val = int(src[1:], 0)
                except ValueError:
                    val = None
            pending_push = val
            pushes_since_label.append(s)
        v = ASM_VEC.match(s)
        if v:
            addr = int(v.group(2), 0)
            if addr % 4 == 0 and addr < 0x400:
                af.vectors.append((addr // 4, v.group(1), ln))
        operands = s[len(mnem):]
        for sym in ASM_SYM.findall(operands):
            af.refs.append((owner, c_name(sym), mnem, ln))
        for word in re.findall(r"(?<![\w.$#])([A-Za-z][\w.$]*)", operands):
            af.locals_ref.append((owner, word, mnem, ln))
    return af


# --------------------------------------------------------------------------
# F. gimple

GLINE = re.compile(r"^(\s*)((?:\[[^\[\]\s]+:\d+:\d+\] )*)(.*)$", re.S)
GANNOT = re.compile(r"\[([^\[\]\s]+):(\d+):\d+\] ")
GHEAD = re.compile(r"^([A-Za-z_]\w*) \((.*)\)$")
UNPREFIXED = []   # gimple statements without a source line (reported)
TEMP_RE = re.compile(r"^(?:_\d+|D\.\d+|[A-Za-z_]\w*\.\d+(?:_\d+)?)$")


class GFn:
    def __init__(self, name, path):
        self.name = name
        self.path = path
        self.params = []
        self.calls = []          # (callee, [arg token lists], site)
        self.fnrefs = []         # (fn name, site)
        self.reads = []          # (var, site)
        self.writes = []         # (var, through?, site)
        self.asms = []           # (template, operand exprs, site)
        self.fn_temps = {}       # temp -> fn name
        self.deref_writes = []   # (base temp/local, site)
        self.assign_src = defaultdict(set)   # local/temp -> idents in RHS
        self.seq = []            # every statement in order: (site|None, body, locals)
        self.decls = []          # declaration lines (`register ... __asm__ (*d0)`)


def split_args(toks):
    """Argument token lists of a call whose `(` is toks[0]."""
    depth, cur, args = 0, [], []
    for t in toks[1:]:
        if t in "([":
            depth += 1
        elif t in ")]":
            if depth == 0:
                break
            depth -= 1
        if t == "," and depth == 0:
            args.append(cur)
            cur = []
        else:
            cur.append(t)
    if cur:
        args.append(cur)
    return args


def open_string(line):
    """Does `line` end inside a string literal (escapes honoured)?"""
    inside, esc = False, False
    for ch in line:
        if esc:
            esc = False
        elif ch == "\\" and inside:
            esc = True
        elif ch == '"':
            inside = not inside
    return inside


def parse_gimple(path, text, fnset, varset, typedefs):
    fns = []
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        m = GHEAD.match(lines[i])
        if not m or i + 1 >= len(lines) or not GLINE.match(lines[i + 1]).group(3) == "{":
            i += 1
            continue
        g = GFn(m.group(1), path)
        # parameters split at top-level commas (a function-pointer parameter
        # `void (*T) (A, B) routine` holds commas of its own)
        for p in (split_args(tokens("(" + m.group(2) + ")")) if m.group(2) else []):
            ids = [t for t in p if is_ident(t)]
            if ids:
                g.params.append(ids[-1])
        i += 2
        scopes = [set(g.params)]
        stmts = []
        while i < len(lines) and lines[i] != "}":
            line = lines[i]
            # join a statement whose string literal spans lines (asm)
            while open_string(line) and i + 1 < len(lines):
                i += 1
                line += "\n" + lines[i]
            i += 1
            lm = GLINE.match(line)
            pre, rest = lm.group(2), lm.group(3).strip()
            if rest == "{":
                scopes.append(set())
                continue
            if rest == "}":
                if len(scopes) > 1:
                    scopes.pop()
                continue
            if not pre and rest.endswith(";") and not rest.startswith(("goto", "return")):
                d = re.sub(r"\s*=.*$", "", rest[:-1], flags=re.S)
                d = re.sub(r"__asm__\s*\(.*\)", "", d)
                d = re.sub(r"\[[^\]]*\]", "", d)
                ids = re.findall(r"[A-Za-z_]\w*(?:\.\d+)?", d)
                if len(ids) >= 2 and not d.lstrip().startswith(("*", "MEM")) and \
                        ("(" not in d or "(*" in d):
                    scopes[-1].add(ids[-1])
                    g.decls.append(rest)
                else:
                    UNPREFIXED.append(f"{path}: {rest[:80]}")
                continue
            if not pre:
                # labels and gotos gcc adds without a source line
                g.seq.append((None, rest, frozenset().union(*scopes)))
                continue
            site = GANNOT.match(pre)
            site = (site.group(1), int(site.group(2)))
            body = GANNOT.sub("", rest)
            stmts.append((body, site, frozenset().union(*scopes)))
            g.seq.append((site, body, frozenset().union(*scopes)))
        i += 1
        analyse_stmts(g, stmts, fnset, varset, typedefs)
        fns.append(g)
    return fns


def analyse_stmts(g, stmts, fnset, varset, typedefs):
    def is_local(t, locs):
        return t in locs or TEMP_RE.match(t) is not None

    for body, site, locs in stmts:
        toks = tokens(body)
        if not toks:
            continue
        if toks[0] == "__asm__":
            asm_stmt(g, toks, site, locs, varset)
            continue
        # assignment?
        eq, depth = None, 0
        for k, t in enumerate(toks):
            if t in "([":
                depth += 1
            elif t in ")]":
                depth -= 1
            elif t == "=" and depth == 0:
                eq = k
                break
        lhs = toks[:eq] if eq is not None else []
        rhs = toks[eq + 1:] if eq is not None else toks
        is_call = any(is_ident(t) and k + 1 < len(rhs) and rhs[k + 1] == "("
                      and t not in C_KEYWORDS and t not in typedefs and not is_local(t, locs)
                      for k, t in enumerate(rhs))
        base_idx = None
        if lhs:
            base, deref, base_idx = lhs_base(lhs)
            if base is not None:
                if base in varset and not is_local(base, locs):
                    g.writes.append((base, deref, site))
                elif is_local(base, locs):
                    if deref:
                        g.deref_writes.append((base, site))
                    else:
                        for t in rhs:
                            if is_ident(t):
                                g.assign_src[base].add(t)
                        fn_val = [t for t in rhs if t in fnset and not is_local(t, locs)]
                        if fn_val and not is_call:
                            g.fn_temps[base] = fn_val[-1]
            scan_refs(g, lhs, site, locs, fnset, varset, typedefs, base_idx, is_call)
        scan_refs(g, rhs, site, locs, fnset, varset, typedefs, None, is_call)


def lhs_base(lhs):
    """(base identifier, written through a pointer?, its index) of a gimple LHS."""
    k, deref = 0, False
    if lhs[0] == "*":
        deref, k = True, 1
    if lhs[k:k + 2] == ["MEM", "["]:
        deref = True
        k += 2
        if k < len(lhs) and lhs[k] == "(":
            depth = 0
            for j in range(k, len(lhs)):
                if lhs[j] == "(":
                    depth += 1
                elif lhs[j] == ")":
                    depth -= 1
                    if depth == 0:
                        break
            k = j + 1
    if k < len(lhs) and lhs[k] == "&":
        k += 1
    if k >= len(lhs) or not is_ident(lhs[k]):
        return None, deref, None
    nxt = lhs[k + 1] if k + 1 < len(lhs) else None
    if nxt == "->":
        deref = True
    return lhs[k], deref, k


def scan_refs(g, toks, site, locs, fnset, varset, typedefs, skip_idx, in_call):
    n = len(toks)
    for k, t in enumerate(toks):
        if k == skip_idx:
            continue
        if not is_ident(t) or t in C_KEYWORDS or t in typedefs:
            continue
        prev = toks[k - 1] if k else None
        nxt = toks[k + 1] if k + 1 < n else None
        if prev in (".", "->", "struct", "union", "enum", "goto"):
            continue
        if t in locs or TEMP_RE.match(t):
            continue
        if nxt == "(":
            # gcc's own vararg IL (`VA_ARG`, `__builtin_va_*`) is no call
            if t != "VA_ARG" and not t.startswith("__builtin_va_"):
                g.calls.append((t, split_args(toks[k + 1:]), site))
            continue
        if t in varset:
            if prev == "&" and in_call:
                g.writes.append((t, False, site))
            g.reads.append((t, site))
        elif t in fnset:
            g.fnrefs.append((t, site))


def asm_stmt(g, toks, site, locs, varset):
    # __asm__ [__volatile__] ( "tmpl" : outs : ins : clobbers )
    k = toks.index("(")
    tmpl = toks[k + 1]
    sections, cur, depth = [[]], [], 0
    for t in toks[k + 2:]:
        if t == "(":
            depth += 1
        elif t == ")":
            if depth == 0:
                break
            depth -= 1
        if t == ":" and depth == 0:
            sections[-1].append(cur)
            sections.append([])
            cur = []
        elif t == "," and depth == 0:
            sections[-1].append(cur)
            cur = []
        else:
            cur.append(t)
    sections[-1].append(cur)
    sections = sections[1:]
    outs = [op for op in (sections[0] if len(sections) > 0 else []) if op]
    ins = [op for op in (sections[1] if len(sections) > 1 else []) if op]
    operands = []
    for op in outs + ins:
        expr = [t for t in op if not t.startswith('"')]
        operands.append(expr)
    for op in outs:
        for t in op:
            if t in varset and t not in locs:
                g.writes.append((t, False, site))
    for op in ins:
        for t in op:
            if t in varset and t not in locs:
                g.reads.append((t, site))
    g.asms.append((bytes(tmpl[1:-1], "utf-8").decode("unicode_escape"), operands, site))


def c_binding(g):
    """(trap, argument index or None, fixed opcode or None) of a C function
    whose inline asm issues `trap #N`, else None."""
    for tmpl, operands, _site in g.asms:
        lines = [ln.strip() for ln in tmpl.split("\n")]
        for idx, ln in enumerate(lines):
            m = re.match(r"trap\s+#(\d+)", ln)
            if not m:
                continue
            trap = int(m.group(1))
            push = None
            for prev in reversed(lines[:idx]):
                p = re.match(r"(move|clr)\.([bwl])\s+(.*?),?\s*-\(sp\)$", prev)
                if p:
                    push = p
                    break
            if push is None:
                return (trap, None, None)
            if push.group(1) == "clr":
                return (trap, None, 0)
            src = push.group(3).rstrip(",")
            if src.startswith("#"):
                return (trap, None, int(src[1:], 0))
            om = re.match(r"%(\d+)$", src)
            if om and int(om.group(1)) < len(operands):
                expr = operands[int(om.group(1))]
                if len(expr) == 1 and expr[0] in g.params:
                    return (trap, g.params.index(expr[0]), None)
                if len(expr) == 1 and re.match(r"^-?\d+$", expr[0]):
                    return (trap, None, int(expr[0]))
            return (trap, None, None)
    return None


def parse_int(arg):
    if len(arg) == 1 and re.match(r"^-?(0x[0-9a-fA-F]+|\d+)B?$", arg[0]):
        return int(arg[0].rstrip("B"), 0)
    return None


# --------------------------------------------------------------------------
# F2. statement walks over gimple: trap #2 opcodes, the AES switch

BINOPS = {">>": lambda a, b: a >> b, "<<": lambda a, b: a << b,
          "+": lambda a, b: a + b, "-": lambda a, b: a - b,
          "&": lambda a, b: a & b, "|": lambda a, b: a | b}
GOTO_RE = re.compile(r"goto (<[^<>]+>|[A-Za-z_]\w*);")
LABEL_RE = re.compile(r"^(<[^<>]+>|[A-Za-z_]\w*):$")


def split_assign(body):
    """(lhs tokens, rhs tokens) of a gimple statement; lhs is [] if none."""
    toks = tokens(body)
    if toks and toks[-1] == ";":
        toks = toks[:-1]
    depth = 0
    for k, t in enumerate(toks):
        if t in "([":
            depth += 1
        elif t in ")]":
            depth -= 1
        elif t == "=" and depth == 0:
            return toks[:k], toks[k + 1:]
    return [], toks


def strip_cast(toks):
    """`(type) x` -> `x` (gimple prints a conversion as a leading cast)."""
    while len(toks) >= 2 and toks[0] == "(":
        depth = 0
        for k, t in enumerate(toks):
            if t == "(":
                depth += 1
            elif t == ")":
                depth -= 1
                if depth == 0:
                    break
        if k == len(toks) - 1:
            return toks
        toks = toks[k + 1:]
    return toks


def call_of(rhs):
    """(callee, [argument token lists]) if the right-hand side is a call."""
    if len(rhs) >= 2 and is_ident(rhs[0]) and rhs[1] == "(" and rhs[0] not in C_KEYWORDS:
        return rhs[0], split_args(rhs[1:])
    return None


def apply_op(v, op, n):
    if v[0] == "k":
        return ("k", BINOPS[op](v[1], n))
    return ("p", v[1], v[2] + ((op, n),))


def opcode_facts(g, arr, arr_ok, binding, setters, carriers, may_issue):
    """What `g` stores into element 0 of the opcode array `arr` and which
    trap #2 calls it issues, walking its statements in order.

    A value is ("k", n) (a constant), ("p", i, ops) (parameter i with the
    operations `ops` applied) or None (not a constant). Branches are not
    followed: the pending values are those element 0 may hold now -- a store
    after a trap call replaces them, a store after a store adds to them (the
    two arms of an `if`). A call of a function that may issue a trap #2
    itself (`may_issue`) ends the pending values: what it leaves in element
    0 is not followed. Returns (pending values at the end, issues: [(called
    function, value, site)]); an issue is a call of the binding (with every
    pending value) or of a carrier. setters: function name -> the values a
    function that never issues leaves pending (`gem_if0`); carriers: function
    name -> the parameter-valued opcodes it issues."""
    env = {}
    stores, issues = [], []
    issued = [False]

    def store(vals):
        if issued[0]:
            stores.clear()
        stores.extend(vals)
        issued[0] = False

    def val(toks, locs):
        toks = strip_cast(toks)
        if len(toks) == 1:
            t = toks[0]
            n = parse_int([t])
            if n is not None:
                return ("k", n)
            if t in env:
                return env[t]
            if t in g.params:
                return ("p", g.params.index(t), ())
            return None
        if len(toks) == 2 and toks[0] == "&" and toks[1] == arr and arr_ok and arr not in locs:
            return ("ptr", 0)
        if len(toks) == 3 and toks[1] in BINOPS:
            a, n = val([toks[0]], locs), parse_int([toks[2]])
            if a is None or n is None:
                return None
            if a[0] == "ptr":
                return ("ptr", a[1] + n) if toks[1] == "+" else None
            return apply_op(a, toks[1], n)
        return None

    def subst(pe, args, locs):
        if pe is None or pe[0] == "k":
            return pe
        if pe[1] >= len(args):
            return None
        a = val(args[pe[1]], locs)
        if a is None or a[0] not in ("k", "p"):
            return None
        for op, n in pe[2]:
            a = apply_op(a, op, n)
        return a

    for site, body, locs in g.seq:
        if site is None:
            continue
        lhs, rhs = split_assign(body)
        call = call_of(rhs)
        if call:
            callee, args = call
            if callee == binding:
                for v in (stores or [None]):
                    issues.append((binding, v, site))
                issued[0] = True
            elif callee in may_issue:
                issues += [(callee, subst(pe, args, locs), site)
                           for pe in sorted(carriers.get(callee, ()), key=repr)]
                stores.clear()
                issued[0] = True
            elif callee in setters:
                store([subst(pe, args, locs) for pe in sorted(setters[callee], key=repr)])
            if len(lhs) == 1:
                env.pop(lhs[0], None)
            continue
        if not lhs:
            continue
        if arr_ok and lhs == [arr, "[", "0", "]"] and arr not in locs:
            store([val(rhs, locs)])
        elif len(lhs) == 2 and lhs[0] == "*" and env.get(lhs[1]) == ("ptr", 0):
            store([val(rhs, locs)])
        elif len(lhs) == 1 and (lhs[0] in locs or TEMP_RE.match(lhs[0])):
            v = val(rhs, locs)
            if v is None:
                env.pop(lhs[0], None)
            else:
                env[lhs[0]] = v
    return stores, issues


def trap2_issues(gfns, binding, arr, arr_key_of):
    """({function name: [(called function, value, site)]}, carriers) for one
    trap #2 binding, after propagating setters and carriers to a fixpoint.
    `may_issue`: every function whose gimple calls reach the binding."""
    callers = defaultdict(set)
    for g in gfns:
        for c, _a, _s in g.calls:
            callers[c].add(g.name)
    may_issue, todo = set(), [binding]
    while todo:
        x = todo.pop()
        for c in callers.get(x, ()):
            if c not in may_issue:
                may_issue.add(c)
                todo.append(c)
    want = arr_key_of(None)
    setters, carriers = {}, {}
    for _round in range(64):
        facts = defaultdict(list)
        new_s, new_c = defaultdict(set), defaultdict(set)
        for g in gfns:
            ok = arr_key_of(g.path) == want
            st, iss = opcode_facts(g, arr, ok, binding, setters, carriers, may_issue)
            facts[g.name] += iss
            if st and g.name not in may_issue:
                new_s[g.name] |= set(st)
            for _via, v, _s in iss:
                if v is not None and v[0] == "p":
                    new_c[g.name].add(v)
        if dict(new_s) == setters and dict(new_c) == carriers:
            return facts, carriers
        setters, carriers = dict(new_s), dict(new_c)
    die(f"trap #2 via {binding}: opcode propagation does not settle")


def switch_slots(g, cf, param, fnset, typedefs):
    """{case value: {(fn, site)}}: the functions called in each case of the
    switch on parameter `param` of `g`, reached over the gimple control flow
    (labels, gotos, fall-through) without leaving the source lines of the
    switch statement."""
    seq = g.seq
    labels = {}
    for i, (_s, body, _l) in enumerate(seq):
        m = LABEL_RE.match(body)
        if m:
            labels[m.group(1)] = i
    copies = {param}
    sws = []
    for i, (site, body, _l) in enumerate(seq):
        if body.startswith("switch ("):
            m = re.match(r"switch \(([\w.]+)\)", body)
            if m and m.group(1) in copies:
                sws.append(i)
            continue
        lhs, rhs = split_assign(body)
        if len(lhs) == 1 and len(strip_cast(rhs)) == 1 and strip_cast(rhs)[0] in copies:
            copies.add(lhs[0])
    if len(sws) != 1:
        die(f"{g.name}: {len(sws)} switch statements on `{param}`, want 1")
    ssite, sbody, _l = seq[sws[0]]
    btoks = cf.body_toks.get(g.name, [])
    k = next((k for k, tk in enumerate(btoks)
              if tk[0] == "switch" and (tk[1], tk[2]) == ssite), None)
    if k is None or btoks[k + 1][0] != "(":
        die(f"{g.name}: the switch at {ssite[0]}:{ssite[1]} is not in the source")
    brace = match_close(btoks, k + 1) + 1
    if btoks[brace][0] != "{":
        die(f"{g.name}: the switch at {ssite[0]}:{ssite[1]} has no braced body")
    sfile, lo, hi = ssite[0], ssite[1], btoks[match_close(btoks, brace)][2]

    def calls_from(start):
        seen, todo, found = set(), [start], set()
        while todo:
            i = todo.pop()
            if i in seen or i >= len(seq):
                continue
            seen.add(i)
            site, body, locs = seq[i]
            if site is not None and not (site[0] == sfile and lo <= site[1] <= hi):
                continue
            if body.startswith(("goto ", "if (")):
                todo += [labels[t] for t in GOTO_RE.findall(body) if t in labels]
                continue
            if body.startswith("switch ("):
                todo += [labels[t] for t in re.findall(r": (<[^<>]+>)", body) if t in labels]
                continue
            if body.startswith("return"):
                continue
            toks = tokens(body)
            for j, t in enumerate(toks[:-1]):
                if toks[j + 1] == "(" and t in fnset and t not in locs and \
                        t not in C_KEYWORDS and t not in typedefs and not TEMP_RE.match(t):
                    found.add((t, site))
            todo.append(i + 1)
        return found

    out = defaultdict(set)
    for a, b, lab in re.findall(r"case (-?\d+)(?: \.\.\. (-?\d+))?: (<[^<>]+>)", sbody):
        if lab not in labels:
            die(f"{g.name}: case label {lab} not found")
        found = calls_from(labels[lab])
        for v in range(int(a), int(b or a) + 1):
            out[v] |= found
    return out


def binding_d0(g):
    """The constant a trap #2 binding loads into d0 before the trap: an
    immediate in its asm template, or a `register ... __asm__ (d0)` input
    operand assigned a constant."""
    regs = {}
    for d in g.decls:
        m = re.match(r"register .*?\b(\w+) __asm__ \(\*?(\w+)\);$", d)
        if m:
            regs[m.group(1)] = m.group(2)
    for tmpl, operands, _site in g.asms:
        lines = [ln.strip() for ln in tmpl.split("\n")]
        for idx, ln in enumerate(lines):
            if not re.match(r"trap\s+#2$", ln):
                continue
            for prev in reversed(lines[:idx]):
                m = re.match(r"(?:moveq|move\.[wl])\s+#(\S+?),\s*d0$", prev)
                if m:
                    return int(m.group(1), 0)
            for expr in operands:
                if len(expr) == 1 and regs.get(expr[0]) == "d0":
                    for _site, body, _l in g.seq:
                        lhs, rhs = split_assign(body)
                        if lhs == expr and parse_int(rhs) is not None:
                            return parse_int(rhs)
    return None


# --------------------------------------------------------------------------
# F3. vector slots: installs of function addresses, calls through them

AREG = re.compile(r"^a[0-7]$")
ASM_PLAIN_SYM = re.compile(r"^(_?[A-Za-z]\w*)(?:\.[wl])?$")


def split_operands(ops):
    out, cur, depth = [], "", 0
    for ch in ops:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def areg_list(op):
    """Address registers a register operand names (`a0`, `d0-d1/a0-a1`)."""
    out = set()
    if not re.match(r"^[ad][0-7](?:-[ad][0-7])?(?:/[ad][0-7](?:-[ad][0-7])?)*$", op):
        return out
    for part in op.split("/"):
        a, _, b = part.partition("-")
        b = b or a
        if b[0] == "a":
            lo = int(a[1]) if a[0] == "a" else 0
            out |= {f"a{i}" for i in range(lo, int(b[1]) + 1)}
    return out


def vector_scan(gfns, cfiles, afiles, fnset, varset, var_key, sysvar_addr, abs_installs,
                setexc_read):
    """Function addresses stored into variables ("slots") and the calls
    through them (tools/deps/README.md, kind `vector`).

    A slot is a variable (`hdv_rw`), a field (`kbdvecs.mousevec`), or any
    element of an array (`bconstat_vec[]`). Returns (installs: slot ->
    {(fn, site)}, copies: slot -> {(slot, site)}, calls: [(caller, slot,
    site, through)], notes). setexc_read(callee, args): the address whose
    value a `Setexc(n, ...)` call returns (the vector before the call), else
    None."""
    sysname = {a: n for n, a in sysvar_addr.items()}
    installs = defaultdict(set)
    copies = defaultdict(set)
    calls = []
    notes = defaultdict(list)
    computed = defaultdict(set)     # slot -> {"fn at site"}: a stored value not traced

    cvisible = {}   # C-visible asm data label -> its AsmFile
    for af in sorted(afiles.values(), key=lambda a: a.path):
        for lab in af.data_labels:
            if lab not in af.local_data:
                cvisible.setdefault(lab, af)

    def field_slot(base, field):
        # an asm structure (`_kbdvecs:` followed by `mousevec: .ds.l 1`): the
        # field is the local label of that name in the defining file
        af = cvisible.get(base)
        if af is not None and field in af.local_data:
            return field
        return f"{base}.{field}"

    def c_slot(toks, locs, path):
        toks = strip_cast(toks)
        if not toks or not is_ident(toks[0]) or toks[0] in locs or TEMP_RE.match(toks[0]):
            return None
        v = toks[0]
        if v not in varset and v not in sysvar_addr:
            return None
        key, rest = var_key(v, path), toks[1:]
        if rest[:1] == ["["]:
            depth = 0
            for j, t in enumerate(rest):
                if t == "[":
                    depth += 1
                elif t == "]":
                    depth -= 1
                    if depth == 0:
                        break
            key, rest = key + "[]", rest[j + 1:]
        if len(rest) == 2 and rest[0] == "." and is_ident(rest[1]):
            key = field_slot(v, rest[1]) if key == v else f"{key}.{rest[1]}"
            rest = []
        return key if not rest else None

    def is_fn(t, locs):
        return t in fnset and t not in locs and t not in varset

    # assembler regions that call through a register (CALLS_ARGUMENT check)
    jsr_owners = set()
    for af in afiles.values():
        for ev in af.events:
            if ev[0] == "insn" and ev[2].split(".")[0] in ("jsr", "jmp") and \
                    re.match(r"^\(a[0-7]\)$", ev[3]):
                jsr_owners.add(ev[1])
    calls_arg = {}
    for name, idx in sorted(CALLS_ARGUMENT.items()):
        if name not in jsr_owners:
            die(f"{name} (CALLS_ARGUMENT) is no assembler region with a computed jsr")
        calls_arg[name] = idx
    # C functions that call through one of their own parameters
    for g in gfns:
        env = {}
        for site, body, locs in g.seq:
            if site is None:
                continue
            lhs, rhs = split_assign(body)
            call = call_of(rhs)
            if call:
                c = call[0]
                src = g.params.index(c) if c in g.params else env.get(c)
                if isinstance(src, int) and c in locs | set(env):
                    calls_arg.setdefault(g.name, src)
                continue
            if len(lhs) == 1 and (lhs[0] in locs or TEMP_RE.match(lhs[0])):
                r = strip_cast(rhs)
                if len(r) == 1 and r[0] in g.params:
                    env[lhs[0]] = g.params.index(r[0])
                elif len(r) == 1 and r[0] in env:
                    env[lhs[0]] = env[r[0]]
                else:
                    env.pop(lhs[0], None)

    # ---- C: stores and calls, statement by statement
    for g in gfns:
        env = {}    # temp/local -> ("slot", key) | ("fn", name) | ("param", i)
        for site, body, locs in g.seq:
            if site is None:
                continue
            lhs, rhs = split_assign(body)
            call = call_of(rhs)
            if call:
                c, args = call
                if c in g.params:
                    pass                            # calls_arg above
                elif c in locs or TEMP_RE.match(c):
                    ev = env.get(c)
                    if ev and ev[0] == "slot":
                        calls.append((g.name, ev[1], site, None))
                    elif not (ev and ev[0] == "param"):
                        notes["untraced_calls"].append(
                            f"{g.name}: call through `{c}` at {site[0]}:{site[1]}")
                elif c in calls_arg and calls_arg[c] < len(args):
                    a = strip_cast(args[calls_arg[c]])
                    if len(a) == 2 and a[0] == "&":
                        a = a[1:]
                    ev = None
                    if len(a) == 1 and a[0] in env:
                        ev = env[a[0]]
                    elif len(a) == 1 and is_fn(a[0], locs):
                        ev = ("fn", a[0])
                    else:
                        s = c_slot(a, locs, g.path)
                        ev = ("slot", s) if s else None
                    if ev and ev[0] == "slot":
                        calls.append((g.name, ev[1], site, c))
                    elif not (ev and ev[0] in ("fn", "param")):
                        notes["untraced_calls"].append(
                            f"{g.name}: argument {calls_arg[c] + 1} of {c} at {site[0]}:{site[1]}")
                if len(lhs) == 1:
                    a = setexc_read(c, args)
                    if a is not None and a >= 0x400:
                        env[lhs[0]] = ("slot", sysname.get(a, f"{a:#x}"))
                    else:
                        env.pop(lhs[0], None)
                continue
            if not lhs:
                continue
            k = c_slot(lhs, locs, g.path)
            if k is not None:
                r = strip_cast(rhs)
                if len(r) == 2 and r[0] == "&":
                    r = r[1:]
                ev = None
                if len(r) == 1 and r[0] in env:
                    ev = env[r[0]]
                elif len(r) == 1 and is_fn(r[0], locs):
                    ev = ("fn", r[0])
                elif r:
                    s = c_slot(r, locs, g.path)
                    ev = ("slot", s) if s else None
                if ev and ev[0] == "fn":
                    installs[k].add((ev[1], site))
                elif ev and ev[0] == "slot":
                    copies[k].add((ev[1], site))
                else:
                    computed[k].add(f"{g.name} at {site[0]}:{site[1]}")
                continue
            if len(lhs) == 1 and (lhs[0] in locs or TEMP_RE.match(lhs[0])):
                r = strip_cast(rhs)
                ev = None
                if len(r) == 2 and r[0] == "&" and is_fn(r[1], locs):
                    ev = ("fn", r[1])
                elif len(r) == 1 and r[0] in env:
                    ev = env[r[0]]
                elif len(r) == 1 and is_fn(r[0], locs):
                    ev = ("fn", r[0])
                elif len(r) == 1 and r[0] in g.params:
                    ev = ("param", g.params.index(r[0]))
                else:
                    s = c_slot(r, locs, g.path)
                    ev = ("slot", s) if s else None
                if ev is None:
                    env.pop(lhs[0], None)
                else:
                    env[lhs[0]] = ev

    # ---- C initialisers: an array's elements, a scalar's value
    for path in sorted(cfiles):
        cf = cfiles[path]
        for v, init in sorted(cf.inits.items()):
            fns = sorted({(tk[0], tk[1], tk[2]) for tk in init if tk[0] in fnset})
            if not fns:
                continue
            key = var_key(v, path) + ("[]" if v in cf.arrays else "")
            for f, file, ln in fns:
                installs[key].add((f, (file, ln)))

    # ---- absolute installs (`*(PFVOID*)0x400 = f`, Setexc(0x100, f)) at a
    # system-variable address; exception vectors below 0x400 are entered by
    # the CPU (traps: `layer` edges), not called through
    for vec in sorted(abs_installs):
        addr = vec * 4
        if addr >= 0x400:
            key = sysname.get(addr, f"{addr:#x}")
            for f, site in abs_installs[vec]:
                installs[key].add((f, site))

    # ---- assembler: register-tracked loads, stores and computed calls
    def name_slot(n, af):
        if n in af.local_data:
            return n
        if n in fnset:
            return None
        if n in varset or n in sysvar_addr or n in cvisible:
            return n
        return None

    def asm_slot(op, regs, af):
        m = ASM_PLAIN_SYM.match(op)
        if m:
            sym = m.group(1)
            if sym.startswith("_"):
                n = c_name(sym)
                return None if n in af.local_data else name_slot(n, af)
            return sym if sym in af.local_data else None
        m = re.match(r"^(0x[0-9a-fA-F]+|\d+)(?:\.[wl])?$", op)
        if m:
            a = int(m.group(1), 0)
            return sysname.get(a) if a >= 0x400 else None
        m = re.match(r"^(_?[A-Za-z]\w*)-(_?[A-Za-z]\w*)\((a[0-7])\)$", op)
        if m and regs.get(m.group(3)) == ("addr", c_name(m.group(2))):
            return asm_slot(m.group(1), {}, af)
        m = re.match(r"^\((a[0-7])\)$", op)
        if m and regs.get(m.group(1), ("",))[0] == "addr":
            return name_slot(regs[m.group(1)][1], af)
        return None

    for af in sorted(afiles.values(), key=lambda a: a.path):
        in_nodes = af.path.split("/")[0] in DIRS
        if not in_nodes:
            continue
        for lab, syms in sorted(af.inits.items()):
            for sym, ln in syms:
                if sym in fnset:
                    for key in (lab, lab + "[]"):
                        installs[key].add((sym, (af.path, ln)))
        regs = {}
        for ev in af.events:
            if ev[0] == "label":
                regs = {}    # control may arrive from elsewhere
                continue
            _k, owner, mnem, ops, ln = ev
            base, _, size = mnem.partition(".")
            opl = split_operands(ops)
            site = (af.path, ln)
            if base in ("jsr", "jmp") and len(opl) == 1 and re.match(r"^\(a[0-7]\)$", opl[0]):
                r = regs.get(opl[0][1:3])
                if r and r[0] == "val":
                    calls.append((owner, r[1], site, None))
                elif r and r[0] == "addr" and r[1] in fnset:
                    pass        # a direct call: the `asm` edge of the lea
                elif owner not in calls_arg:
                    notes["asm_computed"].append(f"{owner}: {mnem} {ops} at {af.path}:{ln}")
                regs.pop("a0", None)
                regs.pop("a1", None)
                continue
            if base == "lea" and len(opl) == 2 and AREG.match(opl[1]):
                m = ASM_PLAIN_SYM.match(opl[0])
                if m:
                    regs[opl[1]] = ("addr", c_name(m.group(1)))
                else:
                    regs.pop(opl[1], None)
                continue
            if base in ("move", "movea") and len(opl) == 2 and size == "l":
                src, dst = opl
                for grp in re.findall(r"\((a[0-7])\)\+|-\((a[0-7])\)", src):
                    for r in grp:
                        regs.pop(r, None)
                if AREG.match(dst):
                    m = ASM_PLAIN_SYM.match(src[1:]) if src.startswith("#") else None
                    s = None if src.startswith("#") else asm_slot(src, regs, af)
                    if m:
                        regs[dst] = ("addr", c_name(m.group(1)))
                    elif s:
                        regs[dst] = ("val", s)
                    else:
                        regs.pop(dst, None)
                    continue
                k = asm_slot(dst, regs, af)
                if k:
                    f = None
                    if src.startswith("#") and ASM_PLAIN_SYM.match(src[1:]):
                        f = c_name(ASM_PLAIN_SYM.match(src[1:]).group(1))
                    elif AREG.match(src) and regs.get(src, ("",))[0] == "addr":
                        f = regs[src][1]
                    if f in fnset:
                        installs[k].add((f, site))
                    else:
                        computed[k].add(f"{owner} at {af.path}:{ln}")
                    continue
            for op in opl:
                for grp in re.findall(r"\((a[0-7])\)\+|-\((a[0-7])\)", op):
                    for r in grp:
                        regs.pop(r, None)
            if opl:
                for r in areg_list(opl[-1]):
                    regs.pop(r, None)

    called = {s for _c, s, _site, _t in calls}
    for k in sorted(called):
        for x in sorted(computed.get(k, ())):
            notes["computed_installs"].append(f"{k}: {x}")
    notes["calls_arg"] = sorted(f"{n} (argument {i + 1})" for n, i in calls_arg.items())
    return installs, copies, calls, notes


# --------------------------------------------------------------------------
# G. cflow

CFLOW_LINE = re.compile(r"^( *)([A-Za-z_]\w*)\(\)")


def cflow_edges(ppdir, files):
    p = run([CFLOW, "--no-preprocess", "-A", "-A", "--omit-arguments", "-d", "2",
             *files], cwd=ppdir)
    edges = set()
    cur = None
    for line in p.stdout.splitlines():
        m = CFLOW_LINE.match(line)
        if not m:
            continue
        if m.group(1) == "":
            cur = m.group(2)        # a root: one defined function
        elif cur is not None:
            edges.add((cur, m.group(2)))
    return edges


# --------------------------------------------------------------------------
# H. the run

def layer_of(path):
    d = path.split("/", 1)[0]
    return d if d in DIRS else ("include" if d == "include" else d)


def check_translated(cc, tree, translated, tmp):
    """`x.tr.c` and `x.c` must give the same token stream once string
    literals are blanked (tools/bug only translates strings)."""
    bad = []
    for tr, c, flags in translated:
        a = run([cc, *flags, "-E", "-P", tr], cwd=tree, env=cc_env(tmp)).stdout
        b = run([cc, *flags, "-E", "-P", c], cwd=tree, env=cc_env(tmp)).stdout
        blank = lambda s: [("\"\"" if t.startswith('"') else t) for t in tokens(s)]
        ta, tb = blank(a), blank(b)
        # adjacent literals may be joined differently; compare without them
        ta = [t for k, t in enumerate(ta) if not (t == '""' and k and ta[k - 1] == '""')]
        tb = [t for k, t in enumerate(tb) if not (t == '""' and k and tb[k - 1] == '""')]
        if ta != tb:
            bad.append(tr)
    if bad:
        die(f"translated sources differ beyond string literals: {bad}")


def cc_env(tmp):
    e = dict(os.environ)
    e["TMPDIR"] = str(tmp)
    return e


def generate(outdir):
    env = read_env()
    tree = verify_tree(env)
    with tempfile.TemporaryDirectory(prefix="gen-deps-") as td:
        tmp = Path(td)
        cc, cmds, ldcmd = compile_commands(env, tree, tmp)
        sysvar_addr = sysvars(ldcmd, tree, tmp)
        graph_srcs, binding_srcs, translated = select_sources(cmds, tree)
        flagsets = {tuple(f) for f in graph_srcs.values()}
        if len(flagsets) != 1:
            die(f"sources compile with {len(flagsets)} different flag sets")
        flags = list(next(iter(flagsets)))
        check_translated(cc, tree, translated, tmp)
        ccver = run([cc, "--version"]).stdout.splitlines()[0]
        cflowver = run([CFLOW, "--version"]).stdout.splitlines()[0]

        pp = tmp / "pp"
        gdir = tmp / "gimple"

        def prep(path):
            out = pp / path
            out.parent.mkdir(parents=True, exist_ok=True)
            text = run([cc, *graph_srcs.get(path, binding_srcs.get(path)), "-E", path],
                       cwd=tree, env=cc_env(tmp)).stdout
            out.write_text(text)
            gim = None
            if path.endswith(".c"):
                gd = gdir / path.replace("/", "__")
                gd.mkdir(parents=True, exist_ok=True)
                run([cc, *graph_srcs[path], "-S", "-o", "/dev/null",
                     "-fdump-tree-gimple-lineno", "-dumpdir", str(gd) + "/", path],
                    cwd=tree, env=cc_env(tmp))
                dumps = sorted(gd.glob("*.gimple"))
                if len(dumps) > 1:
                    die(f"several gimple dumps for {path}")
                # a file without function bodies (data only) gets no dump
                gim = dumps[0].read_text(errors="replace") if dumps else ""
            return path, text, gim

        allsrc = sorted(graph_srcs) + sorted(binding_srcs)
        with ThreadPoolExecutor(max_workers=8) as ex:
            results = list(ex.map(prep, allsrc))

        cfiles, afiles, gtexts = {}, {}, {}
        for path, text, gim in results:
            if path.endswith(".c"):
                cfiles[path] = parse_c(path, text)
                gtexts[path] = gim
            else:
                afiles[path] = parse_asm(path, text)

        cflow_files = sorted(p for p in graph_srcs if p.endswith(".c"))
        cf_edges = cflow_edges(pp, cflow_files)

        return build(env, tree, flags, cc, ccver, cflowver, cfiles, afiles, gtexts,
                     cf_edges, binding_srcs, sysvar_addr, outdir)


def build(env, tree, flags, cc, ccver, cflowver, cfiles, afiles, gtexts,
          cf_edges, binding_srcs, sysvar_addr, outdir):
    graph_asm = {p: a for p, a in afiles.items() if p.split("/")[0] in DIRS}

    # ---- nodes
    defs = defaultdict(set)           # fn -> {(file, line)}
    for cf in cfiles.values():
        for name, (f, ln) in cf.fndefs.items():
            defs[name].add((f, ln))
    for af in graph_asm.values():
        for name, (f, ln) in af.globals.items():
            defs[name].add((f, ln))
    protos = set()
    typedefs = set()
    for cf in cfiles.values():
        protos |= cf.protos
        typedefs |= cf.typedefs
    for af in afiles.values():
        protos |= set(af.globals)
    fnset = set(defs) | protos
    varset = set()
    static_vars = defaultdict(set)    # var -> {file} where it is static
    var_site = {}
    for af in afiles.values():
        varset |= {v for v in af.data_labels if v not in fnset}
    for path, cf in cfiles.items():
        for v, (static, f, ln) in cf.vars.items():
            if v in fnset:
                continue
            varset.add(v)
            if static:
                static_vars[v].add(path)
            var_site.setdefault(v, (f, ln))

    def var_key(v, path):
        return f"{v}@{path}" if path in static_vars.get(v, ()) else v

    # ---- gimple
    gfns = []
    for path in sorted(gtexts):
        cf = cfiles[path]
        gfns += parse_gimple(path, gtexts[path], fnset, varset, typedefs | cf.typedefs)
    by_name = defaultdict(list)
    for g in gfns:
        by_name[g.name].append(g)

    edges = {}    # (from, to, kind, detail) -> site

    def add(frm, to, kind, detail, site):
        key = (frm, to, kind, detail)
        s = f"{site[0]}:{site[1]}" if site else "-"
        if key not in edges or site_key(s) < site_key(edges[key]):
            edges[key] = s

    notes = defaultdict(list)

    # ---- bindings
    bindings = {}     # name -> (trap, argidx, fixed)
    for g in gfns:
        b = c_binding(g)
        if b:
            bindings.setdefault(g.name, b)
    for af in afiles.values():
        per_owner = defaultdict(list)
        for owner, trap, op, ln in af.traps:
            per_owner[owner].append((trap, op))
        for owner, lst in per_owner.items():
            if len(lst) == 1 and lst[0][1] == "pass":
                bindings.setdefault(owner, (lst[0][0], 0, None))
            elif len(lst) == 1 and isinstance(lst[0][1], int) and owner in protos and \
                    af.path.split("/")[0] in BINDING_ONLY_DIRS:
                bindings.setdefault(owner, (lst[0][0], None, lst[0][1]))

    # ---- tables
    table_entries = {}  # name -> [(slot, fn, (file, line))]
    table_base = {}
    table_dispatchers = defaultdict(set)
    for tname, (tfile, trap, base) in TABLES.items():
        cf = cfiles.get(tfile)
        if cf is None or tname not in cf.inits:
            die(f"dispatch table {tname} not found in {tfile}")
        init = cf.inits[tname]
        if init[0][0] != "{":
            die(f"{tname}: initializer is no brace list")
        entries = split_top(init[1:match_close(init, 0)])
        if entries and not entries[-1]:
            entries = entries[:-1]
        # dispatchers: C functions reading the table, asm regions naming it
        for g in gfns:
            if any(v == tname for v, _ in g.reads) and g.path == tfile:
                table_dispatchers[tname].add(g.name)
        for af in graph_asm.values():
            for owner, sym, mnem, ln in af.refs:
                if sym == tname:
                    table_dispatchers[tname].add(owner)
        if not table_dispatchers[tname]:
            die(f"no dispatcher reads {tname}")
        if base is None:
            bases = set()
            for d in table_dispatchers[tname]:
                body = cf.bodies.get(d, [])
                for k, t in enumerate(body):
                    # `tname[ident]` or `tname[ident - N]`; `tname[0]` (sizeof) is no index
                    if t == tname and k + 2 < len(body) and body[k + 1] == "[" and \
                            is_ident(body[k + 2]):
                        if k + 3 < len(body) and body[k + 3] == "]":
                            bases.add(0)
                        elif k + 5 < len(body) and body[k + 3] == "-" and body[k + 5] == "]":
                            bases.add(int(body[k + 4], 0))
            if len(bases) != 1:
                die(f"{tname}: index base not readable from its dispatcher ({bases})")
            base = bases.pop()
        table_base[tname] = base
        lst = []
        for idx, ent in enumerate(entries):
            fns = [tk for tk in ent if tk[0] in fnset and is_ident(tk[0])]
            for tk in fns:
                lst.append((base + idx, tk[0], (tk[1], tk[2])))
        table_entries[tname] = lst
        for d in sorted(table_dispatchers[tname]):
            for slot, fn, site in lst:
                add(d, fn, "table", f"{tname}[{slot:#x}]", site)

    trap_table = defaultdict(list)
    for tname, (tfile, trap, _b) in TABLES.items():
        trap_table[trap].append(tname)

    def resolve(trap, op):
        out = []
        for tname in trap_table.get(trap, ()):
            for slot, fn, _s in table_entries[tname]:
                if slot == op:
                    out.append((tname, fn))
        return out

    # ---- cflow direct edges, classified by gimple
    g_calls = defaultdict(set)
    g_call_site = {}
    g_refs = defaultdict(set)
    g_ref_site = {}
    for g in gfns:
        for callee, _args, site in g.calls:
            g_calls[g.name].add(callee)
            k = (g.name, callee)
            if k not in g_call_site or site < g_call_site[k]:
                g_call_site[k] = site
        for fn, site in g.fnrefs:
            g_refs[g.name].add(fn)
            k = (g.name, fn)
            if k not in g_ref_site or site < g_ref_site[k]:
                g_ref_site[k] = site
    fnptr_vars = set()
    for frm, to in sorted(cf_edges):
        if to in varset and to not in fnset:
            # cflow lists `fp(...)` through a function-pointer variable as a
            # call of `fp`; gimple shows a read of the variable (a data fact)
            fnptr_vars.add(f"{frm} -> {to}")
            continue
        called = to in g_calls.get(frm, ())
        reffed = to in g_refs.get(frm, ())
        if called:
            add(frm, to, "direct", "-", g_call_site[(frm, to)])
        if reffed:
            add(frm, to, "callback", "address", g_ref_site[(frm, to)])
        if not called and not reffed:
            site = min(defs.get(frm, {("?", 0)}))
            add(frm, to, "direct", "cflow-only", site)
            notes["cflow_only"].append(f"{frm} -> {to}")
    cf_set = set(cf_edges)
    gimple_only = set()
    for g in gfns:
        for callee, _a, _s in g.calls:
            if callee in varset and callee not in fnset:
                continue
            if (g.name, callee) not in cf_set:
                add(g.name, callee, "direct", "gimple-only", g_call_site[(g.name, callee)])
                gimple_only.add(f"{g.name} -> {callee}")
        for fn, _s in g.fnrefs:
            if (g.name, fn) not in cf_set:
                add(g.name, fn, "callback", "address", g_ref_site[(g.name, fn)])
    notes["gimple_only"] = sorted(gimple_only)
    notes["fnptr_vars"] = sorted(fnptr_vars)

    # ---- asm edges
    for af in sorted(graph_asm.values(), key=lambda a: a.path):
        for owner, sym, mnem, ln in af.refs:
            if sym in TABLES:
                continue
            if sym in fnset and sym != owner:
                add(owner, sym, "asm", mnem, (af.path, ln))
        for owner, label, mnem, ln in af.locals_ref:
            lo = af.label_owner.get(label)
            if lo is not None and lo != owner and label not in af.globals:
                add(owner, lo, "asm", f"{mnem} {label}", (af.path, ln))
        for owner, nxt, ln in af.fallthrough:
            add(owner, nxt, "asm", "fallthrough", (af.path, ln))

    # ---- vectors (trap handlers)
    vectors = defaultdict(set)    # vector number -> {(handler, site)}
    # `_12 = 136B; *_12 = vditrap;`: a function stored at a vector address
    for path in sorted(gtexts):
        for m in re.finditer(r"\[([^\[\]\s]+):(\d+):\d+\] (_\d+) = (\d+)B;\n"
                             r"\s*(?:\[[^\[\]\s]+:\d+:\d+\] )+\*\3 = "
                             r"(?:\[[^\[\]\s]+:\d+:\d+\] )?([A-Za-z_]\w*);",
                             gtexts[path]):
            addr, fn = int(m.group(4)), m.group(5)
            if fn in fnset and addr % 4 == 0 and addr < 0x400:
                vectors[addr // 4].add((fn, (m.group(1), int(m.group(2)))))
    for af in afiles.values():
        for vec, fn, ln in af.vectors:
            if fn in fnset:
                vectors[vec].add((fn, (af.path, ln)))

    # ---- layer edges from C call sites
    unresolved = []
    for g in gfns:
        for callee, args, site in g.calls:
            b = bindings.get(callee)
            if not b:
                continue
            trap, argidx, fixed = b
            if trap == 2:
                continue    # added below, once per binding
            op = fixed if fixed is not None else (
                parse_int(args[argidx]) if argidx is not None and argidx < len(args) else None)
            if op is None:
                unresolved.append(f"{g.name} -> {callee} (trap #{trap}, opcode not a constant) "
                                  f"at {site[0]}:{site[1]}")
                continue
            targets = resolve(trap, op)
            if not targets:
                unresolved.append(f"{g.name} -> {callee} (trap #{trap} {op:#x}: no table slot) "
                                  f"at {site[0]}:{site[1]}")
                continue
            for tname, fn in targets:
                add(g.name, fn, "layer", f"trap #{trap} {op:#x} {tname}[{op:#x}] via {callee}", site)
            # Setexc(vector, handler): a vector install
            if any(fn == "setexc" for _t, fn in targets) and len(args) >= 3:
                vec = parse_int(args[1])
                h = args[2]
                hfn = None
                if len(h) == 1 and h[0] in g.fn_temps:
                    hfn = g.fn_temps[h[0]]
                elif len(h) == 1 and h[0] in fnset:
                    hfn = h[0]
                if vec is not None and hfn:
                    vectors[vec].add((hfn, site))
    # asm trap sites
    for af in sorted(afiles.values(), key=lambda a: a.path):
        if af.path.split("/")[0] not in DIRS:
            continue
        for owner, trap, op, ln in af.traps:
            if owner in bindings:
                continue
            if trap == 2:
                continue
            if not isinstance(op, int):
                unresolved.append(f"{owner} (trap #{trap}, opcode not an immediate push) "
                                  f"at {af.path}:{ln}")
                continue
            targets = resolve(trap, op)
            if not targets:
                unresolved.append(f"{owner} (trap #{trap} {op:#x}: no table slot) at {af.path}:{ln}")
            for tname, fn in targets:
                add(owner, fn, "layer", f"trap #{trap} {op:#x} {tname}[{op:#x}]", (af.path, ln))
    # fixed-opcode bindings (asm, e.g. trap1_pexec): binding -> target
    for name, (trap, argidx, fixed) in sorted(bindings.items()):
        if trap != 2 and fixed is not None:
            for tname, fn in resolve(trap, fixed):
                callers = [g.name for g in gfns if any(c == name for c, _a, _s in g.calls)]
                for c in sorted(set(callers)):
                    site = g_call_site[(c, name)]
                    add(c, fn, "layer", f"trap #{trap} {fixed:#x} {tname}[{fixed:#x}] via {name}", site)
    # trap #2: the opcode travels in a parameter block (TRAP2); it is followed
    # from the constant at the call site into element 0 of the opcode array
    trap2 = []          # (binding, class, d0, array, dispatchers, carriers, slots)
    switch_entries = {}  # AES dispatch switch -> [(case, fn, site)]
    if sorted(n for n, b in bindings.items() if b[0] == 2) != sorted(TRAP2):
        die(f"trap #2 bindings {sorted(n for n, b in bindings.items() if b[0] == 2)}"
            f" != TRAP2 {sorted(TRAP2)}")
    for bname, (cls, arr, disp) in sorted(TRAP2.items()):
        bgs = [g for g in gfns if g.name == bname]
        if len(bgs) != 1:
            die(f"trap #2 binding {bname}: {len(bgs)} definitions")
        bg = bgs[0]
        d0 = binding_d0(bg)
        if d0 is None:
            die(f"trap #2 binding {bname}: no constant d0")
        if not any(re.match(rf"^[\w.]+\.\w+ = &{arr};$", body)
                   for g in gfns if g.path == bg.path for _s, body, _l in g.seq):
            die(f"trap #2 binding {bname}: no parameter-block field holds &{arr} in {bg.path}")
        if disp[0] == "switch":
            _k, sfile, sfn, sparam = disp
            sgs = [g for g in gfns if g.name == sfn and g.path == sfile]
            if len(sgs) != 1:
                die(f"trap #2 {cls}: dispatch {sfn} not found in {sfile}")
            cfs = cfiles[sfile]
            cases = switch_slots(sgs[0], cfs, sparam, fnset, typedefs | cfs.typedefs)
            entries = sorted((op, fn, site) for op, s in cases.items() for fn, site in s)
            switch_entries[sfn] = entries
            for op, fn, site in entries:
                add(sfn, fn, "table", f"{sfn}[{op:#x}]", site)
            dispatchers = {sfn}
            slots = defaultdict(list)
            for op, fn, _s in entries:
                slots[op].append((sfn, fn))
        else:
            dispatchers = set().union(*(table_dispatchers[t] for t in disp[1]))
            slots = defaultdict(list)
            for t in disp[1]:
                for op, fn, _s in table_entries[t]:
                    slots[op].append((t, fn))
        bpath = bg.path
        facts, carriers = trap2_issues(gfns, bname, arr,
                                       lambda p, arr=arr, bpath=bpath: var_key(arr, p or bpath))
        for caller in sorted(facts):
            for via, v, site in facts[caller]:
                if v is None:
                    unresolved.append(f"{caller} -> {via} (trap #2 {cls}, opcode not a constant) "
                                      f"at {site[0]}:{site[1]}")
                elif v[0] == "k":
                    if not slots.get(v[1]):
                        unresolved.append(f"{caller} -> {via} (trap #2 {cls} {v[1]:#x}: "
                                          f"no function in that slot) at {site[0]}:{site[1]}")
                    for tname, fn in slots.get(v[1], ()):
                        add(caller, fn, "layer",
                            f"trap #2 {v[1]:#x} {tname}[{v[1]:#x}] via {via}", site)
        trap2.append((bname, cls, d0, arr, sorted(dispatchers), carriers, len(slots)))

    # ---- callback edges through initialised function-pointer variables
    unreferenced_inits = []
    for path in sorted(cfiles):
        cf = cfiles[path]
        for v, init in sorted(cf.inits.items()):
            if v in TABLES:
                continue
            fns = sorted({(tk[0], tk[1], tk[2]) for tk in init if tk[0] in fnset})
            if not fns:
                continue
            key = var_key(v, path)
            refs = set()
            for g in gfns:
                if any(var_key(r, g.path) == key for r, _ in g.reads) or \
                   any(var_key(w, g.path) == key for w, _t, _s in g.writes):
                    refs.add(g.name)
            for af in graph_asm.values():
                for owner, sym, mnem, ln in af.refs:
                    if sym == v and key == v:
                        refs.add(owner)
            if not refs:
                unreferenced_inits.append(f"{v} ({path}): {', '.join(sorted({f[0] for f in fns}))}")
            for r in sorted(refs):
                for fn, f, ln in fns:
                    add(r, fn, "callback", f"via {v}", (f, ln))

    for af in sorted(graph_asm.values(), key=lambda a: a.path):
        for v, syms in sorted(af.inits.items()):
            fns = sorted({(sym, ln) for sym, ln in syms if sym in fnset})
            if not fns:
                continue
            refs = set()
            for g in gfns:
                if any(r == v for r, _ in g.reads) or any(w == v for w, _t, _s in g.writes):
                    refs.add(g.name)
            for af2 in graph_asm.values():
                for owner, sym, mnem, ln in af2.refs:
                    if sym == v:
                        refs.add(owner)
            if not refs:
                unreferenced_inits.append(f"{v} ({af.path}): {', '.join(sorted({f for f, _ in fns}))}")
            for r in sorted(refs):
                for fn, ln in fns:
                    add(r, fn, "callback", f"via {v}", (af.path, ln))

    # ---- data edges (writer -> reader)
    writers = defaultdict(dict)    # key -> {fn: (through?, site)}
    readers = defaultdict(set)
    for g in gfns:
        # write through pointers derived from globals (flow-insensitive)
        origin = defaultdict(set)
        changed = True
        srcs = g.assign_src
        while changed:
            changed = False
            for x, ids in srcs.items():
                new = set()
                for t in ids:
                    if t in varset:
                        new.add(t)
                    elif t in srcs:
                        new |= origin[t]
                if not new <= origin[x]:
                    origin[x] |= new
                    changed = True
        for v, through, site in g.writes:
            k = var_key(v, g.path)
            prev = writers[k].get(g.name)
            if prev is None or (prev[0] and not through) or site < prev[1]:
                writers[k][g.name] = (through, site)
        for b, site in g.deref_writes:
            for v in origin.get(b, ()):
                k = var_key(v, g.path)
                if g.name not in writers[k]:
                    writers[k][g.name] = (True, site)
        for v, _s in g.reads:
            readers[var_key(v, g.path)].add(g.name)
    for k in sorted(writers):
        name = k.split("@")[0]
        for w, (through, site) in sorted(writers[k].items()):
            for r in sorted(readers.get(k, ())):
                if r != w:
                    add(w, r, "data", ("*" if through else "") + name, site)

    # ---- trap -> table verification
    adj = defaultdict(set)
    for (frm, to, kind, _d) in edges:
        if kind in ("direct", "asm"):
            adj[frm].add(to)
    trap_checks = []
    for tname, (tfile, trap, _b) in sorted(TABLES.items()):
        if trap == 2:
            handlers = sorted({h for h, _s in vectors.get(0x22, ())})
        else:
            handlers = sorted({h for h, _s in vectors.get(0x20 + trap, ())})
        reach = set()
        todo = list(handlers)
        while todo:
            x = todo.pop()
            if x in reach:
                continue
            reach.add(x)
            todo += adj.get(x, ())
        ok = bool(table_dispatchers[tname] & reach)
        trap_checks.append((tname, trap, handlers, sorted(table_dispatchers[tname]), ok))
        if not ok:
            die(f"trap #{trap}: no installed handler ({handlers}) reaches the dispatcher "
                f"of {tname} ({sorted(table_dispatchers[tname])})")
    # trap #2 per class: an installed handler compares d0 with the binding's
    # value and reaches the class's dispatch
    trap2_checks = []
    handlers2 = {h for h, _s in vectors.get(0x22, ())}
    cmps = defaultdict(set)
    for af in afiles.values():
        for owner, val, _ln in af.d0_cmps:
            cmps[val].add(owner)
    for bname, cls, d0, arr, disp, _c, _n in trap2:
        hs = sorted(handlers2 & cmps.get(d0, set()))
        reach = set()
        todo = list(hs)
        while todo:
            x = todo.pop()
            if x in reach:
                continue
            reach.add(x)
            todo += adj.get(x, ())
        if not hs or not set(disp) & reach:
            die(f"trap #2 {cls}: no installed handler comparing d0 with {d0:#x} ({hs}) "
                f"reaches {disp}")
        trap2_checks.append((cls, d0, hs, disp))

    # ---- vector edges: calls through a variable to the functions stored in it
    def setexc_read(callee, args):
        b = bindings.get(callee)
        if not b or len(args) < 3:
            return None
        trap, argidx, fixed = b
        op = fixed if fixed is not None else (
            parse_int(args[argidx]) if argidx is not None and argidx < len(args) else None)
        if op is None or not any(fn == "setexc" for _t, fn in resolve(trap, op)):
            return None
        vec = parse_int(["".join(args[1])])
        return vec * 4 if vec is not None else None

    v_installs, v_copies, v_calls, v_notes = vector_scan(
        gfns, cfiles, afiles, fnset, varset, var_key, sysvar_addr, vectors, setexc_read)

    def v_handlers(slot):
        out, seen, todo = set(), set(), [slot]
        while todo:
            s = todo.pop()
            if s in seen:
                continue
            seen.add(s)
            out |= {f for f, _site in v_installs.get(s, ())}
            todo += [s2 for s2, _site in v_copies.get(s, ())]
        return out

    v_slots = defaultdict(lambda: [set(), set()])   # slot -> [callers, handlers]
    no_handler = []
    for caller, slot, site, through in v_calls:
        hs = v_handlers(slot)
        v_slots[slot][0].add(caller)
        v_slots[slot][1] |= hs
        if not hs:
            no_handler.append(f"{caller} -> {slot} at {site[0]}:{site[1]}"
                              + (f" (through {through})" if through else ""))
        for f in sorted(hs):
            add(caller, f, "vector", f"via {slot}", site)

    # ---- def rows (one per definition; merged names have several)
    def_rows = sorted((name, "-", "def", layer_of(f), f"{f}:{ln}")
                      for name in defs for f, ln in defs[name])

    meta = dict(env=env, tree=tree, flags=flags, cc=cc, ccver=ccver, cflowver=cflowver,
                binding_srcs=sorted(binding_srcs), notes=notes,
                unresolved=sorted(set(unresolved)), unreferenced_inits=unreferenced_inits,
                bindings=bindings, table_entries=table_entries, table_base=table_base,
                table_dispatchers=table_dispatchers, vectors=vectors,
                trap_checks=trap_checks, defs=defs, cfiles=cfiles, graph_asm=graph_asm,
                unprefixed=sorted(set(UNPREFIXED)), trap2=trap2, trap2_checks=trap2_checks,
                switch_entries=switch_entries, v_slots=v_slots, v_installs=v_installs,
                v_copies=v_copies, no_handler=sorted(set(no_handler)), v_notes=v_notes,
                sysvar_addr=sysvar_addr)
    write_outputs(outdir, edges, def_rows, meta)


def site_key(s):
    if s == "-":
        return ("~", 0)
    f, _, ln = s.rpartition(":")
    return (f, int(ln) if ln.isdigit() else 0)


# --------------------------------------------------------------------------
# I. outputs

def header_lines(meta, what):
    env = meta["env"]
    return [
        f"{what} -- EmuTOS dependency graph at the oracle pin. GENERATED by {GENERATOR};",
        "never edit by hand: rerun the generator (check: " + GENERATOR + " --check).",
        f"emutos-commit: {env['ORACLE_EMUTOS_COMMIT']}",
        f"rom-sha256: {env['ORACLE_ROM_SHA256']} (etos512de.img, {MAKE_RECIPE_LINE},"
        " tools/oracle/build-emutos.sh, docs/analysis/ORACLE.md)",
        f"build-tree: references/emutos-oracle/{env['ORACLE_EMUTOS_COMMIT'][:8]}",
        f"cc: {env['ORACLE_CROSS_MINT']}gcc ({meta['ccver']})",
        "cflags (make -n of the recipe): " + " ".join(meta["flags"]),
        f"cflow: {CFLOW} ({meta['cflowver']})",
        f"node dirs: {' '.join(DIRS)}; read for trap bindings only: "
        + " ".join(meta["binding_srcs"]),
    ]


def write_outputs(outdir, edges, def_rows, meta):
    outdir.mkdir(parents=True, exist_ok=True)
    rows = sorted([(k, v) for k, v in edges.items()] +
                  [((n, t, k, d), site) for n, t, k, d, site in def_rows],
                  key=lambda kv: (kv[0][0], kv[0][1], kv[0][2], kv[0][3], kv[1]))
    lines = ["# " + l for l in header_lines(meta, "edges.tsv")]
    lines += [
        "# columns: from<TAB>to<TAB>kind<TAB>detail<TAB>site",
        "# kind def: a node row (to = '-', detail = layer, site = definition);"
        " all other kinds are edges.",
        "# kinds: " + " ".join(KINDS) + " (tools/deps/README.md); data edges run writer -> reader.",
        "from\tto\tkind\tdetail\tsite",
    ]
    for (frm, to, kind, detail), site in rows:
        lines.append(f"{frm}\t{to}\t{kind}\t{detail}\t{site}")
    (outdir / "edges.tsv").write_text("\n".join(lines) + "\n")
    (outdir / "layers.md").write_text(layers_md(edges, def_rows, meta))


def layers_md(edges, def_rows, meta):
    defs = meta["defs"]
    out = ["# EmuTOS dependency graph: layers", "",
           "<!-- GENERATED by tools/deps/gen-deps.py; never edit by hand. -->", "",
           "## Provenance", "", "```"]
    out += header_lines(meta, "layers.md")
    out += ["```", "",
            "Edges: `docs/analysis/deps/edges.tsv`. Derivation per edge kind and its",
            "known gaps: `tools/deps/README.md`. Query: `cargo xtask deps <fn> --down|--up`.",
            ""]

    def layer(fn):
        ls = sorted({layer_of(f) for f, _ in defs.get(fn, ())})
        return ",".join(ls) if ls else "ext"

    nodes = set()
    for (frm, to, kind, _d) in edges:
        nodes.add(frm)
        if to != "-":
            nodes.add(to)
    counts = defaultdict(int)
    for (frm, to, kind, _d) in edges:
        counts[kind] += 1
    out += ["## Counts", "", "| item | count |", "|---|---|",
            f"| nodes (functions, merged by name) | {len(nodes)} |",
            f"| nodes defined in the node dirs | {sum(1 for n in defs if any(layer_of(f) in DIRS for f, _ in defs[n]))} |"]
    for k in KINDS:
        out.append(f"| {k} edges | {counts[k]} |")
    out.append(f"| def rows | {len(def_rows)} |")
    out.append("")

    # layer matrix
    out += ["## Edges between layers", "",
            "Rows: layer of `from`; columns: layer of `to`; all edge kinds except `data`."
            " `include` = static inline functions of `include/*.h`; `ext` = defined outside"
            " the node dirs (util/, cli/, libgcc) or nowhere.", ""]
    lays = sorted({layer(n) for n in nodes})
    mat = defaultdict(int)
    for (frm, to, kind, _d) in edges:
        if kind in ("def", "data"):
            continue
        mat[(layer(frm), layer(to))] += 1
    out.append("| from \\ to | " + " | ".join(lays) + " |")
    out.append("|---|" + "---|" * len(lays))
    for a in lays:
        out.append(f"| {a} | " + " | ".join(str(mat[(a, b)]) for b in lays) + " |")
    out.append("")

    # merged names
    merged = sorted(n for n in defs if len(defs[n]) > 1)
    out += ["## Names merged into one node", "",
            "Defined more than once (static functions of the same name, or static inline"
            " functions of a header compiled into several files). One node each; every"
            " definition is a `def` row.", ""]
    for n in merged:
        sites = ", ".join(f"{f}:{ln}" for f, ln in sorted(defs[n]))
        out.append(f"- {n}: {sites}")
    out.append("")

    # tables
    out += ["## Dispatch tables (`table` edges)", "",
            "| table | file | trap | index base | dispatcher | slots with a function |",
            "|---|---|---|---|---|---|"]
    for tname, (tfile, trap, _b) in TABLES.items():
        out.append(f"| {tname} | `{tfile}` | #{trap} ({TRAP_NAMES[trap]}) | "
                   f"{meta['table_base'][tname]} | "
                   f"{', '.join(sorted(meta['table_dispatchers'][tname]))} | "
                   f"{len(meta['table_entries'][tname])} |")
    for bname, cls, d0, arr, disp, _c, _n in meta["trap2"]:
        spec = TRAP2[bname][2]
        if spec[0] == "switch":
            n = len(meta["switch_entries"][spec[2]])
            out.append(f"| {spec[2]} (switch on `{spec[3]}`) | `{spec[1]}` | #2 ({cls}) | "
                       f"case value | {spec[2]} | {n} |")
    out.append("")
    out += ["Trap handlers found (vector installs) and the check that each table's",
            "dispatcher is reachable from them over `direct`/`asm` edges:", "",
            "| table | trap | installed handlers | dispatcher reached |", "|---|---|---|---|"]
    for tname, trap, handlers, disp, ok in meta["trap_checks"]:
        out.append(f"| {tname} | #{trap} | {', '.join(handlers) or '-'} | "
                   f"{'yes' if ok else 'NO'} |")
    out.append("")
    out += ["Trap #2 serves two classes, told apart by d0. For each, the d0 value its",
            "binding loads, the installed handler that compares d0 with it, and the",
            "dispatch that handler reaches over `direct`/`asm` edges:", "",
            "| class | d0 | handler comparing d0 | dispatch reached |", "|---|---|---|---|"]
    for cls, d0, hs, disp in meta["trap2_checks"]:
        out.append(f"| {cls} | {d0:#x} | {', '.join(hs)} | {', '.join(disp)} |")
    out.append("")
    out += ["Vector installs found:", ""]
    for vec in sorted(meta["vectors"]):
        for h, (f, ln) in sorted(meta["vectors"][vec]):
            out.append(f"- vector {vec:#x} (address {vec * 4:#x}): {h} at {f}:{ln}")
    out.append("")

    # bindings
    out += ["## Trap bindings (`layer` edges)", "",
            "| binding | trap | opcode |", "|---|---|---|"]
    for name, (trap, argidx, fixed) in sorted(meta["bindings"].items()):
        if fixed is not None:
            op = f"fixed {fixed:#x}"
        elif argidx is not None:
            op = f"argument {argidx + 1}"
        else:
            op = "parameter block"
            for bname, cls, d0, arr, _d, _c, _n in meta["trap2"]:
                if bname == name:
                    op = f"parameter block: {arr}[0] ({cls}, d0 = {d0:#x})"
        out.append(f"| {name} | #{trap} | {op} |")
    out.append("")

    def pexpr(v):
        e = f"argument {v[1] + 1}"
        for o, n in v[2]:
            e = f"({e} {o} {n})"
        return e[1:-1] if e.startswith("(") else e

    out += ["Trap #2 opcode carriers: functions that pass one of their parameters on"
            " as the opcode (a caller's constant argument is the opcode after the"
            " shown operations; the `layer` edge starts at that caller):", "",
            "| function | class | opcode |", "|---|---|---|"]
    for bname, cls, d0, arr, _d, carriers, _n in meta["trap2"]:
        for fn in sorted(carriers):
            for v in sorted(carriers[fn], key=repr):
                out.append(f"| {fn} | {cls} | {pexpr(v)} |")
    out.append("")

    # vectors
    vs = meta["v_slots"]
    out += ["## Vector slots (`vector` edges)", "",
            "Variables, fields or array elements holding function addresses that a",
            "function calls through (`via <slot>`; `[]` = any element). Handlers are the",
            "functions stored into the slot (assignment, initialiser, `Setexc`, assembler",
            "store), or into a slot copied into it. A system variable carries its",
            f"address from `{SYSVARS_LD}`.", "",
            "| slot | address | callers | handlers |", "|---|---|---|---|"]
    for slot in sorted(vs):
        callers, hs = vs[slot]
        addr = meta["sysvar_addr"].get(slot)
        out.append(f"| {slot} | {f'{addr:#x}' if addr is not None else '-'} | "
                   f"{', '.join(sorted(callers))} | {', '.join(sorted(hs)) or '-'} |")
    out.append("")
    out += ["Functions that call through an argument (the slot passed in is the one"
            " called; assembler ones stated in `CALLS_ARGUMENT` of the generator):", ""]
    out += [f"- {x}" for x in meta["v_notes"]["calls_arg"]] or ["- none"]
    out.append("")

    out += ["## Not resolved", ""]
    out += [f"Trap call sites whose opcode is not a constant or whose slot holds no"
            f" function ({len(meta['unresolved'])}):", ""]
    out += [f"- {u}" for u in meta["unresolved"]] or ["- none"]
    out.append("")
    nh = meta["no_handler"]
    out += [f"Calls through a vector slot no function of the node dirs is stored into"
            f" ({len(nh)}; no `vector` edge: the handler is installed at run time,"
            " by another program, or from a value not traced):", ""]
    out += [f"- {x}" for x in nh] or ["- none"]
    out.append("")
    ci = meta["v_notes"]["computed_installs"]
    out += [f"Stores into a called vector slot of a value that is no function address"
            f" ({len(ci)}; their handlers are unknown):", ""]
    out += [f"- {x}" for x in ci] or ["- none"]
    out.append("")
    uc = sorted(set(meta["v_notes"]["untraced_calls"]))
    out += [f"C calls through a value not read from a variable ({len(uc)}; a table"
            " dispatch among them is a `table` edge of its dispatcher):", ""]
    out += [f"- {x}" for x in uc] or ["- none"]
    out.append("")
    ac = meta["v_notes"]["asm_computed"]
    out += [f"Assembler computed calls and jumps whose register is not loaded from a"
            f" variable in the same region since its last label ({len(ac)}):", ""]
    out += [f"- {x}" for x in ac] or ["- none"]
    out.append("")
    out += ["Initialised variables holding function addresses that no function or"
            f" assembler region of the node dirs references ({len(meta['unreferenced_inits'])};"
            " their functions get no `callback` edge):", ""]
    out += [f"- {u}" for u in meta["unreferenced_inits"]] or ["- none"]
    out.append("")
    co = sorted(meta["notes"]["cflow_only"])
    out += [f"cflow references the gimple dump does not show ({len(co)}; kept as"
            " `direct` with detail `cflow-only`: calls compiled out under the pin's"
            " configuration, or functions gcc never gimplified because nothing uses them):", ""]
    out += [f"- {x}" for x in co] or ["- none"]
    out.append("")
    fv = meta["notes"]["fnptr_vars"]
    out += [f"cflow \"calls\" through function-pointer variables ({len(fv)}; dropped: the"
            " variable is a global, its read is a `data` fact, its target is unknown):", ""]
    out += [f"- {x}" for x in fv] or ["- none"]
    out.append("")
    go = sorted(meta["notes"]["gimple_only"])
    out += [f"Calls in the gimple dump that cflow does not list ({len(go)}; kept as `direct`"
            " with detail `gimple-only`):", ""]
    out += [f"- {x}" for x in go] or ["- none"]
    out.append("")

    # nodes per layer
    out += ["## Nodes per layer", ""]
    per = defaultdict(list)
    for n in nodes:
        per[layer(n)].append(n)
    for lay in sorted(per):
        out += [f"### {lay} ({len(per[lay])})", "", ", ".join(sorted(per[lay])), ""]
    return "\n".join(out).rstrip("\n") + "\n"


def main():
    args = sys.argv[1:]
    if args not in ([], ["--check"]):
        sys.stderr.write(__doc__)
        sys.exit(2)
    if not args:
        generate(OUT_DIR)
        print(f"wrote {OUT_DIR.relative_to(REPO)}/edges.tsv, layers.md")
        return
    with tempfile.TemporaryDirectory(prefix="gen-deps-check-") as td:
        out = Path(td)
        generate(out)
        diff = []
        for name in ("edges.tsv", "layers.md"):
            a = (OUT_DIR / name).read_bytes() if (OUT_DIR / name).exists() else b""
            b = (out / name).read_bytes()
            if a != b:
                diff.append(name)
        if diff:
            print(f"gen-deps --check: differs: {', '.join(diff)}")
            sys.exit(1)
        print("gen-deps --check: edges.tsv, layers.md byte-identical")


if __name__ == "__main__":
    main()

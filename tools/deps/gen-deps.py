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
KINDS = ("direct", "table", "callback", "asm", "layer", "data")

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
    for line in p.stdout.splitlines():
        words = line.split()
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
    return cc, cmds


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
        self.inits = defaultdict(list)   # data label -> [(symbol, line)]


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
        for p in (m.group(2).split(", ") if m.group(2) else []):
            ids = re.findall(r"[A-Za-z_]\w*", p)
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
                else:
                    UNPREFIXED.append(f"{path}: {rest[:80]}")
                continue
            if not pre:
                continue
            site = GANNOT.match(pre)
            site = (site.group(1), int(site.group(2)))
            body = GANNOT.sub("", rest)
            stmts.append((body, site, frozenset().union(*scopes)))
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
        cc, cmds = compile_commands(env, tree, tmp)
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
                     cf_edges, binding_srcs, outdir)


def build(env, tree, flags, cc, ccver, cflowver, cfiles, afiles, gtexts,
          cf_edges, binding_srcs, outdir):
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
    # trap #2: the opcode travels in a parameter block; edge to every handler
    trap2_handlers = sorted(vectors.get(0x22, ()))
    for name, (trap, argidx, fixed) in sorted(bindings.items()):
        if trap != 2:
            continue
        bsite = min(defs.get(name, {("?", 0)}))
        for h, _s in trap2_handlers:
            add(name, h, "layer", "trap #2 (opcode in parameter block, not resolved)", bsite)

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
        if not ok and trap != 2:
            die(f"trap #{trap}: no installed handler ({handlers}) reaches the dispatcher "
                f"of {tname} ({sorted(table_dispatchers[tname])})")

    # ---- def rows (one per definition; merged names have several)
    def_rows = sorted((name, "-", "def", layer_of(f), f"{f}:{ln}")
                      for name in defs for f, ln in defs[name])

    meta = dict(env=env, tree=tree, flags=flags, cc=cc, ccver=ccver, cflowver=cflowver,
                binding_srcs=sorted(binding_srcs), notes=notes,
                unresolved=sorted(set(unresolved)), unreferenced_inits=unreferenced_inits,
                bindings=bindings, table_entries=table_entries, table_base=table_base,
                table_dispatchers=table_dispatchers, vectors=vectors,
                trap_checks=trap_checks, defs=defs, cfiles=cfiles, graph_asm=graph_asm,
                unprefixed=sorted(set(UNPREFIXED)))
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
    out.append("")
    out += ["Trap handlers found (vector installs) and the check that each table's",
            "dispatcher is reachable from them over `direct`/`asm` edges:", "",
            "| table | trap | installed handlers | dispatcher reached |", "|---|---|---|---|"]
    for tname, trap, handlers, disp, ok in meta["trap_checks"]:
        out.append(f"| {tname} | #{trap} | {', '.join(handlers) or '-'} | "
                   f"{'yes' if ok else 'NO'} |")
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
            op = "parameter block (not resolved)"
        out.append(f"| {name} | #{trap} | {op} |")
    out.append("")

    out += ["## Not resolved", ""]
    out += [f"Trap call sites whose opcode is not a constant ({len(meta['unresolved'])}):", ""]
    out += [f"- {u}" for u in meta["unresolved"]] or ["- none"]
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

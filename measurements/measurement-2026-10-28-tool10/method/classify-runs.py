import re, sys, os, collections
root, listf = sys.argv[1], sys.argv[2]
REPO = '/Volumes/Daten/Development/PiRusTOS'
files = [l.strip() for l in open(listf) if l.strip()]
A, B = [], []
pick = re.compile(r'(?:(?:\$\{?[A-Za-z_]\w*\}?|<[a-z]+>|'+re.escape(REPO)+r')(?:/[\w$.{}-]*)*/|(?<![\w/$.<{}-]))kernel8\.img')
for f in files:
    p = os.path.join(root, f)
    try: lines = open(p, errors='replace').read().splitlines()
    except Exception: continue
    builds = any(re.search(r'xtask\s+build\b', l) and not l.lstrip().startswith('#') for l in lines)
    literal = [i+1 for i,l in enumerate(lines) if REPO+'/kernel8.img' in l]
    uses = [i+1 for i,l in enumerate(lines)
            if pick.search(l) and not l.lstrip().startswith('#')
            and not re.search(r'<kernel8\.img>|kernel8\.img\s*(sha|=\s*[0-9a-f]{16})', l)]
    if builds and uses:
        A.append((f, uses[:6]))
    elif literal:
        B.append((f, literal[:6]))
print('# A: builds with `cargo xtask build` and then picks up <tree>/kernel8.img (%d files)' % len(A))
for f,u in A: print('A', f, 'lines', ','.join(map(str,u)))
print('# B: names the main tree root image %s/kernel8.img literally, no build (%d files)' % (REPO, len(B)))
for f,u in B: print('B', f, 'lines', ','.join(map(str,u)))

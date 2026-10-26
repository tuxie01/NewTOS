#!/usr/bin/env python3
"""check-translations.py <tree> <rom-a> <rom-b>

Every C string literal in the `bug`-translated sources (<tree>/**/*.tr.c) is
looked up in both ROMs as its exact bytes followed by NUL. Answers: does the
ROM built with the x86_64 host `bug` carry the same translated strings as the
release ROM?  Adjacent literals ("a" "b") are joined, as the compiler does.
"""
import pathlib
import re
import sys

LIT = re.compile(rb'"((?:[^"\\\n]|\\.)*)"')
JOIN = re.compile(rb'"\s*\n?\s*"')


def unescape(b):
    out = bytearray()
    i = 0
    while i < len(b):
        c = b[i]
        if c != 0x5C:
            out.append(c)
            i += 1
            continue
        n = b[i + 1:i + 2]
        simple = {b'n': 10, b't': 9, b'r': 13, b'a': 7, b'b': 8, b'f': 12,
                  b'v': 11, b'\\': 92, b'"': 34, b"'": 39, b'?': 63}
        if n in simple:
            out.append(simple[n])
            i += 2
        elif n == b'x':
            m = re.match(rb'[0-9A-Fa-f]+', b[i + 2:])
            out.append(int(m.group(0), 16) & 0xFF)
            i += 2 + len(m.group(0))
        elif n.isdigit():
            m = re.match(rb'[0-7]{1,3}', b[i + 1:])
            out.append(int(m.group(0), 8) & 0xFF)
            i += 1 + len(m.group(0))
        else:
            raise ValueError(b[i:i + 4])
    return bytes(out)


def literals(path):
    src = path.read_bytes()
    # drop comments and preprocessor lines so their quotes do not count
    src = re.sub(rb'/\*.*?\*/', b' ', src, flags=re.S)
    src = re.sub(rb'^\s*#.*$', b'', src, flags=re.M)
    src = JOIN.sub(b'', src)
    return [unescape(m.group(1)) for m in LIT.finditer(src)]


def main():
    tree, rom_a, rom_b = sys.argv[1:4]
    a = pathlib.Path(rom_a).read_bytes()
    b = pathlib.Path(rom_b).read_bytes()
    files = sorted(pathlib.Path(tree).rglob('*.tr.c'))
    total = only_a = only_b = neither = 0
    for f in files:
        lits = {s for s in literals(f) if len(s) >= 2}
        for s in sorted(lits):
            total += 1
            in_a = (s + b'\0') in a
            in_b = (s + b'\0') in b
            if in_a and not in_b:
                only_a += 1
                print(f'ONLY-A {f.relative_to(tree)}: {s!r}')
            elif in_b and not in_a:
                only_b += 1
                print(f'ONLY-B {f.relative_to(tree)}: {s!r}')
            elif not in_a and not in_b:
                neither += 1
    print(f'{len(files)} files, {total} distinct literals (len >= 2): '
          f'{total - only_a - only_b - neither} in both, '
          f'{only_a} only in A, {only_b} only in B, {neither} in neither')


if __name__ == '__main__':
    main()

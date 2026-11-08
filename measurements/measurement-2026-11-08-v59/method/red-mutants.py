#!/usr/bin/env python3
"""V59 red check: apply one deliberately wrong edit at a time to the kernel
pieces, the trait defaults or mouse.rs, run the two V59 host tests (plus the
V51 copy test for the register check), record red/green, restore the file
byte for byte.

    red-mutants.py <worktree> <out.txt>

Each mutant is (name, file, old, new); `old` must occur exactly once. The
original bytes are kept in memory and written back after every run, also on
an exception, and compared with the files at the end. The unmutated tree must
be green before the first mutant and after the last.
"""
import subprocess
import sys

W = sys.argv[1]
OUT = sys.argv[2]

EXP = "kernel/src/raster_expand.rs"
CPY = "kernel/src/raster_copy.rs"
BAK = "tos-api/src/vdi/backend.rs"
MOU = "tos-api/src/vdi/mouse.rs"

MUTANTS = [
    ("sprite: mask painted in fg", EXP,
     "rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 }",
     "rows(mask), &Expand { mode: WriteMode::Transparent, fg, bg: 0 }"),
    ("sprite: data pass before mask pass", EXP,
     "        expand(surface, area, rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 });\n"
     "        expand(surface, area, rows(data), &Expand { mode: WriteMode::Transparent, fg, bg: 0 });\n",
     "        expand(surface, area, rows(data), &Expand { mode: WriteMode::Transparent, fg, bg: 0 });\n"
     "        expand(surface, area, rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 });\n"),
    ("sprite: start bit +1", EXP, "        bit: c0,\n        unit: Unit::Word,", "        bit: c0 + 1,\n        unit: Unit::Word,"),
    ("sprite: first row ignored (r0)", EXP, "base: words[r0..].as_ptr().cast::<u8>(),", "base: words[0..].as_ptr().cast::<u8>(),"),
    ("copy_rows: 1-pixel tail dropped", CPY, 'chunk!(fwd, copy, "{{v0.s}}[0]", "", r1, "4"),', '"",'),
    ("copy_rows: 8-pixel tail dropped", CPY, 'chunk!(fwd, copy, "{{v0.4s, v1.4s}}", "", r2, "32"),', '"",'),
    ("copy_rows: the two row pitches swapped", CPY,
     '"add {src}, {src}, {sstep}\\n",\n            "add {dst}, {dst}, {dstep}\\n",',
     '"add {src}, {src}, {dstep}\\n",\n            "add {dst}, {dst}, {sstep}\\n",'),
    ("copy_rows: q2/q3 not saved", CPY,
     '            "stp q2, q3, [{save}, #32]",\n            xfer_loop!(),',
     '            xfer_loop!(),'),
    ("read_rect: right part not zero-filled", CPY, "                row[c1..].fill(0);\n", ""),
    ("read_rect/write_rect: on-screen span one too wide", CPY,
     "let hi = (scr as i64 - p).clamp(lo, n);", "let hi = (scr as i64 - p + 1).clamp(lo, n);"),
    ("write_rect: source column offset dropped", CPY,
     "            src.as_ptr().add(r0 * w + c0),", "            src.as_ptr().add(r0 * w),"),
    ("trait default draw_sprite: mask over data", BAK,
     "                } else if mask[r] & bit != 0 {\n                    self.fill_rect(xi as i16, yi as i16, 1, 1, bg, clip);",
     "                }\n                if mask[r] & bit != 0 {\n                    self.fill_rect(xi as i16, yi as i16, 1, 1, bg, clip);"),
    ("trait default write_device_rect: transposed", BAK,
     "self.put_device_pixel(xi as i16, yi as i16, src[r * w + c]);",
     "self.put_device_pixel(xi as i16, yi as i16, src[c * w + r]);"),
    ("mouse draw: pens swapped", MOU,
     "backend.draw_sprite(ox, oy, &mask, &data, fg, bg);", "backend.draw_sprite(ox, oy, &mask, &data, bg, fg);"),
    ("mouse draw: save-under one column right", MOU,
     "backend.read_device_rect(ox, oy, 16, 16, store);", "backend.read_device_rect(ox + 1, oy, 16, 16, store);"),
    ("mouse erase: flush one column short", MOU,
     "backend.flush_rect(ox, oy, ox + 15, oy + 15);", "backend.flush_rect(ox, oy, ox + 14, oy + 15);"),
]

TESTS = ["raster_kernel_cell", "vdi_mouse_v59_cell", "raster_kernel_copy"]


def run_tests():
    args = ["cargo", "test", "-p", "tos-api"]
    for t in TESTS:
        args += ["--test", t]
    p = subprocess.run(args, cwd=W, capture_output=True, text=True)
    out = p.stdout + p.stderr
    failed = [l.strip() for l in out.splitlines() if l.startswith("test ") and ("FAILED" in l)]
    sig = [l.strip() for l in out.splitlines() if "signal:" in l or "SIGBUS" in l or "SIGSEGV" in l]
    errors = [l.strip() for l in out.splitlines() if l.startswith("error[") or l.startswith("error:")]
    return p.returncode, failed, sig, errors


def main():
    files = sorted({m[1] for m in MUTANTS})
    orig = {f: open(f"{W}/{f}", "rb").read() for f in files}
    lines = []
    rc, failed, sig, errors = run_tests()
    lines.append(f"unmutated: rc {rc} failed {failed} {sig} {errors[:2]}")
    assert rc == 0, "the unmutated tree must be green"
    reds = 0
    try:
        for name, f, old, new in MUTANTS:
            src = orig[f].decode()
            assert src.count(old) == 1, f"mutant '{name}': anchor occurs {src.count(old)} times"
            open(f"{W}/{f}", "wb").write(src.replace(old, new).encode())
            try:
                rc, failed, sig, errors = run_tests()
            finally:
                open(f"{W}/{f}", "wb").write(orig[f])
            red = rc != 0
            reds += red
            what = "; ".join(failed + sig + errors[:2]) or "(no failing test line)"
            lines.append(f"{'RED  ' if red else 'GREEN'} {name} [{f}]: rc {rc}: {what}")
            print(lines[-1], flush=True)
    finally:
        for f in files:
            open(f"{W}/{f}", "wb").write(orig[f])
    for f in files:
        assert open(f"{W}/{f}", "rb").read() == orig[f], f"{f} not restored"
    rc, failed, sig, errors = run_tests()
    lines.append(f"restored: rc {rc} failed {failed}")
    lines.append(f"{reds} of {len(MUTANTS)} mutants red")
    open(OUT, "w").write("\n".join(lines) + "\n")
    print(lines[-1])


main()

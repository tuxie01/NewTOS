#!/usr/bin/env python3
# Tester mutants for V58: one textual edit in a scratch archive tree, run the V58 tests, restore byte for byte.
import subprocess, sys, shutil, os
T = sys.argv[1]  # scratch tree (git archive of 1c1c71a3)
LINE = T + "/tos-api/src/vdi/line.rs"
RL = T + "/kernel/src/raster_line.rs"
RK = T + "/kernel/src/raster_kernel.rs"
# (name, file, old, new, occurrence index (0-based) of old)
M = [
 ("span phase +1", LINE, "let phase = ((ax as i64 + (lo - first)) & 15) as u32;", "let phase = ((ax as i64 + (lo - first) + 1) & 15) as u32;", 0),
 ("vertical stride sign", LINE, "let step = yinc * s;", "let step = -yinc * s;", 0),
 ("last=false end pixel kept (runs)", LINE, "if mode == WriteMode::Xor && !last {", "if mode == WriteMode::Xor && !last && false {", 1),
 ("run-on column not wrapped", LINE, "if *col == s {", "if *col == s + 1_000_000 {", 0),
 ("span box one short", LINE, "((lo % s) as u32, r0 as u32, (hi % s) as u32, r1 as u32)", "((lo % s) as u32, r0 as u32, ((hi % s) as u32).saturating_sub(1), r1 as u32)", 0),
 ("fill_span phase +1", RK, "phase: phase as usize & 15 }", "phase: (phase as usize + 1) & 15 }", 0),
 ("column dash phase +1", RL, "let mut bit = 0x8000u16 >> (phase & 15);", "let mut bit = 0x8000u16 >> ((phase + 1) & 15);", 0),
 ("vertical cut phase +1", LINE, "(klo & 15) as u32, mask);", "((klo + 1) & 15) as u32, mask);", 0),
 ("diagonal fast box y", LINE, "y1.min(y2) as u32, x2 as u32", "y1 as u32, x2 as u32", 0),
 ("column lost on upward step", RL, "p = p.wrapping_offset(step);", "p = p.wrapping_offset(step.abs());", 0),
 ("Xor keeps no complement", RL, "p.write_volatile(!p.read_volatile())", "p.write_volatile(p.read_volatile())", 0),
]
def run():
    a = subprocess.run(["cargo","test","-p","tos-api","--lib","vdi::line"],cwd=T,capture_output=True,text=True)
    b = subprocess.run(["cargo","test","-p","tos-api","--test","raster_kernel_line","--test","raster_kernel_fill"],cwd=T,capture_output=True,text=True)
    return a.returncode, b.returncode, (a.stdout+a.stderr+b.stdout+b.stderr)
rc = run(); print("baseline", rc[0], rc[1], flush=True)
for name, f, old, new, n in M:
    s = open(f).read(); parts = s.split(old)
    if len(parts) <= n+1: print("NOAPPLY", name); continue
    shutil.copy(f, f + ".orig")
    m = old.join(parts[:n+1]) + new + old.join(parts[n+1:])
    open(f, "w").write(m); os.utime(f)
    try:
        r = run()
        failed = [l for l in r[2].splitlines() if l.startswith("test ") and l.endswith("FAILED")]
        sig = [l for l in r[2].splitlines() if "SIGBUS" in l or "SIGSEGV" in l or "signal" in l][:1]
        print(("RED  " if (r[0] or r[1]) else "GREEN(!)"), name, "lib rc", r[0], "int rc", r[1], len(failed), "failed:", [x[5:-10] for x in failed][:3], sig, flush=True)
    finally:
        shutil.move(f + ".orig", f); os.utime(f)  # new mtime, or cargo keeps the mutant build
rc = run(); print("restored", rc[0], rc[1])

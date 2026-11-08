import subprocess, sys, shutil, os
S="/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/v59t"
T=S+"/t/mut"; P=S+"/t/chg"  # pristine = chg archive tree (same content)
M=[
("M1 sprite: data pass before mask pass","kernel/src/raster_expand.rs",
 "        expand(surface, area, rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 });\n        expand(surface, area, rows(data), &Expand { mode: WriteMode::Transparent, fg, bg: 0 });",
 "        expand(surface, area, rows(data), &Expand { mode: WriteMode::Transparent, fg, bg: 0 });\n        expand(surface, area, rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 });"),
("M2 read_rect: target column offset c0 dropped (wrong at left edge)","kernel/src/raster_copy.rs",
 "out.as_mut_ptr().add(r0 * w + c0),","out.as_mut_ptr().add(r0 * w),"),
("M3 copy_rows: q2/q3 not restored","kernel/src/raster_copy.rs",
 "            \"ldp q2, q3, [{save}, #32]\",\n            simd_off!(),\n            src = inout(reg) src => _,\n            dst = inout(reg) dst => _,\n            sstep",
 "            simd_off!(),\n            src = inout(reg) src => _,\n            dst = inout(reg) dst => _,\n            sstep"),
("M4 erase: flush over ox+16 (cell too wide)","tos-api/src/vdi/mouse.rs",
 "backend.flush_rect(ox, oy, ox + 15, oy + 15);","backend.flush_rect(ox, oy, ox + 16, oy + 16);"),
("M5 draw: save after paint","tos-api/src/vdi/mouse.rs",
 "    backend.read_device_rect(ox, oy, 16, 16, store);\n    backend.draw_sprite(ox, oy, &mask, &data, fg, bg);",
 "    backend.draw_sprite(ox, oy, &mask, &data, fg, bg);\n    backend.read_device_rect(ox, oy, 16, 16, store);"),
("M6 on_screen: right/bottom span one too wide","kernel/src/raster_copy.rs",
 "let hi = (scr as i64 - p).clamp(lo, n);","let hi = (scr as i64 - p + 1).clamp(lo, n);"),
("M7 default draw_sprite: bit shifted by one","tos-api/src/vdi/backend.rs",
 "                let bit = 0x8000u16 >> c;\n                if data[r] & bit != 0 {\n                    self.fill_rect(xi as i16, yi as i16, 1, 1, fg, clip);",
 "                let bit = 0x4000u16 >> c;\n                if data[r] & bit != 0 {\n                    self.fill_rect(xi as i16, yi as i16, 1, 1, fg, clip);"),
("M8 default read_device_rect: row stride h instead of w","tos-api/src/vdi/backend.rs",
 "out[r * w + c] = if xi > i16::MAX as i32","out[c * h + r] = if xi > i16::MAX as i32"),
("M9 sprite: mask painted in fg colour","kernel/src/raster_expand.rs",
 "rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 }","rows(mask), &Expand { mode: WriteMode::Transparent, fg, bg: 0 }"),
("M10 copy_rows: 8-pixel remainder block dropped","kernel/src/raster_copy.rs",
 "            chunk!(fwd, copy, \"{{v0.4s, v1.4s}}\", \"\", r2, \"32\"),\n            \"5:\\n\",\n            \"tbz {n}, #2, 6f\\n\",\n            chunk!(fwd, copy, \"{{v0.4s}}\", \"\", r1, \"16\"),",
 "            \"5:\\n\",\n            \"tbz {n}, #2, 6f\\n\",\n            chunk!(fwd, copy, \"{{v0.4s}}\", \"\", r1, \"16\"),"),
]
only=sys.argv[1:]
out=open(S+"/mutants.txt","a")
for name,f,a,b in M:
    if only and name.split()[0] not in only: continue
    src=open(P+"/"+f).read()
    if src.count(a)!=1:
        print(name,"PATTERN COUNT",src.count(a)); out.write(name+" PATTERN-COUNT "+str(src.count(a))+"\n"); continue
    open(T+"/"+f,"w").write(src.replace(a,b))
    r=subprocess.run("cd %s && cargo test -p tos-api --test raster_kernel_cell --test vdi_mouse_v59_cell --test raster_kernel_copy 2>&1"%T,shell=True,capture_output=True,text=True)
    txt=r.stdout
    res=[l for l in txt.splitlines() if l.startswith("test result") or "FAILED" in l or "SIGBUS" in l or "signal" in l or "error" in l.lower()[:20]]
    red = r.returncode!=0
    line="%s: %s (rc %d) | %s"%(name,"RED" if red else "GREEN(!)",r.returncode," ; ".join(res[:6]))
    print(line); out.write(line+"\n"); out.flush()
    open(T+"/"+f,"w").write(src)   # restore

#!/usr/bin/env python3
"""probe-patch.py <scratch-tree> -- TASK/A142: scratch-only serial probes at the four places a cflib raster can reach the screen
(vrt_cpyfm, vro_cpyfm, vr_trnfm, the G_IMAGE blit of objc_draw). Prints the caller's MFDB and the first raster words as the kernel reads them."""
import pathlib, sys
t = pathlib.Path(sys.argv[1])
p = t / "tos-api/src/vdi/raster.rs"; s = p.read_text()
head = '''
extern "Rust" { pub fn a142_probe(args: core::fmt::Arguments); }
fn a142_words(addr: u64) -> [u16; 8] {
    let mut w = [0u16; 8];
    for (i, x) in w.iter_mut().enumerate() { *x = unsafe { (addr as *const u16).add(i).read_unaligned() }; }
    w
}
'''
s = s + head
# vrt_cpyfm: after the source bind
a = "    if !src.bind(pb, 0, 2 * words as u64, false) {\n        return;\n    }\n"
assert s.count(a) >= 1
user = "    let a142_u = src.addr;\n"
s = s.replace(a, user + a + '    if src.w <= 32 && src.h <= 32 { unsafe { a142_probe(format_args!("[a142] vrt_cpyfm src=0x{:x} {}x{} planes={} stand={} mode={} col={},{} words={:04x?}", a142_u, src.w, src.h, src.nplanes, src.stand, raw, pb.intin(1), pb.intin(2), a142_words(src.addr))); } }\n', 1)
# vro_cpyfm: at entry, the caller's pointer (words only after the bind, so print the pointer here)
a = "    let raw_mode = pb.intin(0);\n"
assert s.count(a) == 1
s = s.replace(a, a + '    if !src.is_screen() && src.w <= 32 && src.h <= 32 { unsafe { a142_probe(format_args!("[a142] vro_cpyfm src=0x{:x} {}x{} planes={} stand={} mode={}", src.addr, src.w, src.h, src.nplanes, src.stand, raw_mode)); } }\n')
# vr_trnfm: planar path
a = "    if !src.bind(pb, 0, form, inplace) || !dst.bind(pb, 0, form, true) {\n        return;\n    }\n"
assert s.count(a) == 1
s = s.replace(a, "    let a142_u = (src.addr, dst.addr);\n" + a + '    if src.w <= 32 && src.h <= 32 { unsafe { a142_probe(format_args!("[a142] vr_trnfm planar src=0x{:x} dst=0x{:x} {}x{} planes={} stand={} words={:04x?}", a142_u.0, a142_u.1, src.w, src.h, src.nplanes, src.stand, a142_words(src.addr))); } }\n')
a = "        if !src.bind(pb, 0, total, false) || !dst.bind(pb, 0, total, true) {\n"
assert s.count(a) == 1
s = s.replace(a, '        if src.w <= 32 && src.h <= 32 { unsafe { a142_probe(format_args!("[a142] vr_trnfm truecolor src=0x{:x} dst=0x{:x} {}x{} planes={} stand={}", src.addr, dst.addr, src.w, src.h, src.nplanes, src.stand)); } }\n' + a)
p.write_text(s)
p = t / "tos-api/src/aes/object.rs"; s = p.read_text()
a = "fn blit_image(t: ObjTree, backend: &dyn RasterBackend, bi: &BitBlk, x: i16, y: i16, col: Pel, clip: ClipRect) {\n"
assert s.count(a) == 1
s = s.replace(a, a + '''    if let Some(kva) = t.ua_span(bi.bi_pdata, 16) {
        let mut w = [0u16; 8];
        for (i, x) in w.iter_mut().enumerate() { *x = unsafe { (kva as *const u16).add(i).read_unaligned() }; }
        unsafe { a142_probe(format_args!("[a142] objc_draw G_IMAGE bi_pdata=0x{:x} wb={} hl={} at {},{} words={:04x?}", bi.bi_pdata, bi.bi_wb, bi.bi_hl, x, y, w)); }
    }
''')
s += 'extern "Rust" { fn a142_probe(args: core::fmt::Arguments); }\n'
p.write_text(s)
p = t / "kernel/src/lib.rs"; s = p.read_text()
s += '\n#[no_mangle]\nfn a142_probe(args: core::fmt::Arguments) { kprintln!("{}", args); }\n'
p.write_text(s)

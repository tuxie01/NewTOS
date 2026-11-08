//! TASK/V59 — the VDI mouse pointer's save, paint and restore in one backend
//! call per 16x16 cell (`RasterBackend::read_device_rect` / `draw_sprite` /
//! `write_device_rect`) leave exactly the pixels the per-pixel path left.
//!
//! Two layers:
//!
//! 1. **The cell against the code it replaced.** `old_draw` / `old_erase`
//!    below are `tos-api/src/vdi/mouse.rs` `draw` / `erase` before V59,
//!    verbatim but for the statics turned into arguments: per cell pixel a
//!    `get_device_pixel` and a 1x1 `fill_rect` with the whole screen as clip,
//!    on erase 256 `put_device_pixel` and one `flush_rect`. They run on one
//!    store, the V59 sequence (`read_device_rect` + `draw_sprite`, then
//!    `write_device_rect` + `flush_rect`) on two others: through the trait's
//!    default methods (`Legacy`) and through the kernel's NEON pieces, glued
//!    as `kernel/src/vdi_backend.rs` glues them (`Fast`: `raster_copy::
//!    read_rect`/`write_rect`, `raster_expand::sprite` behind the cell's
//!    `ClipRect::clamp_to`). Store, save-under and the flushed rectangles
//!    must agree after the paint and after the restore, for every cell
//!    position from 17 pixels off each edge to past it, every preset form and
//!    random ones, random and pen colours (the hotspot is already in the
//!    position the cell gets).
//! 2. **The pointer state machine on top.** The same script of public mouse
//!    calls — `display_at` at edge positions and hotspots, nested
//!    `cursor_hide`/`cursor_show`, `v_hide_c`-style removal, form changes
//!    while shown (`set_form`, `set_preset`, `save_form`/`restore_form`,
//!    `previous_form`), packets through `feed_relative` into every screen
//!    clamp — runs once on `Legacy` and once on `Fast`, each on the same
//!    noise ground; the screen after every step and the flush log must be the
//!    same. Both runs start hidden and end hidden (no save-under carried from
//!    one backend to the other). The `display_at` steps are also checked
//!    against `old_draw` of that preset at `(x - xhot, y - yhot)` on the
//!    ground, each removal against the ground and its flush against the
//!    cell — so `mouse.rs` itself is held to the old path, not only the two
//!    backends to each other.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

#[allow(dead_code)]
#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_copy.rs"]
mod raster_copy;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_expand.rs"]
mod raster_expand;

use std::cell::UnsafeCell;
use std::sync::Mutex;

use raster_kernel::Surface;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::mouse::{self, PRESET_FORMS};

/// xorshift64*, deterministic.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
    fn u16(&mut self) -> u16 {
        (self.next() >> 48) as u16
    }
}

/// A plain screen store with the kernel backend's scalar semantics for the
/// methods the pointer uses: `fill_rect` clipped to the clip and the screen,
/// `get_device_pixel` `0` off the screen, `put_device_pixel` dropped off it,
/// a PEL resolved through a fixed table. `flush_rect` is logged.
struct Store {
    w: i32,
    h: i32,
    stride: usize,
    px: UnsafeCell<Vec<u32>>,
    flushes: UnsafeCell<Vec<(i16, i16, i16, i16)>>,
}
// SAFETY: every test using a `Store` runs under `LOCK`, one thread at a time.
unsafe impl Sync for Store {}

impl Store {
    fn new(w: i32, h: i32, stride: usize, init: &[u32]) -> Store {
        Store {
            w,
            h,
            stride,
            px: UnsafeCell::new(init.to_vec()),
            flushes: UnsafeCell::new(Vec::new()),
        }
    }
    fn pixels(&self) -> Vec<u32> {
        // SAFETY: single-threaded (`LOCK`), no borrow outstanding.
        unsafe { (*self.px.get()).clone() }
    }
    fn flush_log(&self) -> Vec<(i16, i16, i16, i16)> {
        // SAFETY: as above.
        unsafe { (*self.flushes.get()).clone() }
    }
    fn surface(&self) -> Surface {
        // SAFETY: as above; the pointer is used only inside one call.
        Surface { base: unsafe { (*self.px.get()).as_mut_ptr() }, stride_px: self.stride }
    }
    fn on(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }
    fn at(&self, x: i32, y: i32) -> *mut u32 {
        // SAFETY: callers check `on` first.
        unsafe { (*self.px.get()).as_mut_ptr().add(y as usize * self.stride + x as usize) }
    }
}

/// PEL -> device pixel (any injective-enough table will do).
fn pel_rgb(pel: Pel) -> u32 {
    (pel as u32).wrapping_mul(0x0101_0101) ^ 0x0012_3456
}

/// The methods shared by both backends; the pointer never calls the rest.
macro_rules! common_methods {
    () => {
        fn width(&self) -> u16 {
            self.0.w as u16
        }
        fn height(&self) -> u16 {
            self.0.h as u16
        }
        fn num_colors(&self) -> u16 {
            256
        }
        fn planes(&self) -> u16 {
            16
        }
        fn set_color(&self, _: u16, _: u16, _: u16, _: u16) {
            panic!("not used by the pointer");
        }
        fn query_color(&self, _: u16) -> (u16, u16, u16) {
            panic!("not used by the pointer");
        }
        fn set_active_palette(&self, _: &[u16; 256]) {
            panic!("not used by the pointer");
        }
        fn pel_to_device16(&self, _: Pel) -> u16 {
            panic!("not used by the pointer");
        }
        fn device_to_rgb565(&self, _: u32) -> u16 {
            panic!("not used by the pointer");
        }
        fn rgb565_to_device(&self, _: u16) -> u32 {
            panic!("not used by the pointer");
        }
        fn read_pixel(&self, _: i16, _: i16) -> Pel {
            panic!("not used by the pointer");
        }
        fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
            let rect = ClipRect {
                xmn: x.max(clip.xmn),
                ymn: y.max(clip.ymn),
                xmx: ((x as i32 + w as i32 - 1) as i16).min(clip.xmx),
                ymx: ((y as i32 + h as i32 - 1) as i16).min(clip.ymx),
            };
            let Some((x0, y0, x1, y1)) = rect.clamp_to(self.0.w as u16, self.0.h as u16) else { return };
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    // SAFETY: clamped onto the screen.
                    unsafe { self.0.at(xx, yy).write(pel_rgb(pel)) };
                }
            }
        }
        fn abline(&self, _: i16, _: i16, _: i16, _: i16, _: Pel, _: WriteMode, _: ClipRect, _: u16, _: bool) -> u16 {
            panic!("not used by the pointer");
        }
        fn fill_pattern(&self, _: i16, _: i16, _: i16, _: i16, _: PatternFill, _: ClipRect) {
            panic!("not used by the pointer");
        }
        fn draw_glyph_sized(&self, _: i16, _: i16, _: &[u8], _: i16, _: i16, _: Pel, _: WriteMode, _: ClipRect) {
            panic!("not used by the pointer");
        }
        fn blit_mono(&self, _: Rect, _: MonoSrc, _: Pel, _: Pel, _: WriteMode, _: ClipRect) {
            panic!("not used by the pointer");
        }
        fn copy_rect(&self, _: Rect, _: i16, _: i16, _: u8, _: ClipRect) {
            panic!("not used by the pointer");
        }
        fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
            let (x, y) = (x as i32, y as i32);
            if !self.0.on(x, y) {
                return 0;
            }
            // SAFETY: on the screen.
            unsafe { self.0.at(x, y).read() }
        }
        fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
            let (x, y) = (x as i32, y as i32);
            if self.0.on(x, y) {
                // SAFETY: on the screen.
                unsafe { self.0.at(x, y).write(argb) };
            }
        }
        fn flush_rect(&self, x0: i16, y0: i16, x1: i16, y1: i16) {
            // SAFETY: single-threaded (`LOCK`).
            unsafe { (*self.0.flushes.get()).push((x0, y0, x1, y1)) };
        }
    };
}

/// The trait's default cell methods: the per-pixel definitions.
struct Legacy(Store);
impl RasterBackend for Legacy {
    common_methods!();
}

/// The kernel's cell methods, glued as `VdiScreen` glues them.
struct Fast(Store);
impl RasterBackend for Fast {
    common_methods!();

    fn read_device_rect(&self, x: i16, y: i16, w: u16, h: u16, out: &mut [u32]) {
        // SAFETY: the store holds the screen; `out` is not the store.
        unsafe {
            raster_copy::read_rect(self.0.surface(), self.0.w, self.0.h, x as i32, y as i32, w as usize, h as usize, out)
        };
    }
    fn write_device_rect(&self, x: i16, y: i16, w: u16, h: u16, src: &[u32]) {
        // SAFETY: as above.
        unsafe {
            raster_copy::write_rect(self.0.surface(), self.0.w, self.0.h, x as i32, y as i32, w as usize, h as usize, src)
        };
    }
    fn draw_sprite(&self, x: i16, y: i16, mask: &[u16; 16], data: &[u16; 16], fg: Pel, bg: Pel) {
        let cell = ClipRect {
            xmn: x,
            ymn: y,
            xmx: (x as i32 + 15).min(i16::MAX as i32) as i16,
            ymx: (y as i32 + 15).min(i16::MAX as i32) as i16,
        };
        let Some(bx) = cell.clamp_to(self.0.w as u16, self.0.h as u16) else { return };
        // SAFETY: the clamped box lies on the screen and in the cell.
        unsafe { raster_expand::sprite(self.0.surface(), x as i32, y as i32, mask, data, bx, pel_rgb(fg), pel_rgb(bg)) };
    }
}

/// `mouse.rs` `draw` before V59 (the statics as arguments).
#[allow(clippy::too_many_arguments)]
fn old_draw(backend: &dyn RasterBackend, ox: i16, oy: i16, mask: &[u16; 16], data: &[u16; 16], fg: Pel, bg: Pel, store: &mut [u32; 256]) {
    let clip = ClipRect::full(backend.width(), backend.height());
    for dy in 0..16i16 {
        let mrow = mask[dy as usize];
        let drow = data[dy as usize];
        for dx in 0..16i16 {
            let (px, py) = (ox + dx, oy + dy);
            store[(dy * 16 + dx) as usize] = backend.get_device_pixel(px, py);
            let bit = 0x8000u16 >> dx;
            if drow & bit != 0 {
                backend.fill_rect(px, py, 1, 1, fg, clip);
            } else if mrow & bit != 0 {
                backend.fill_rect(px, py, 1, 1, bg, clip);
            }
        }
    }
}

/// `mouse.rs` `erase` before V59 (the statics as arguments).
fn old_erase(backend: &dyn RasterBackend, ox: i16, oy: i16, store: &[u32; 256]) {
    for dy in 0..16i16 {
        for dx in 0..16i16 {
            backend.put_device_pixel(ox + dx, oy + dy, store[(dy * 16 + dx) as usize]);
        }
    }
    backend.flush_rect(ox, oy, ox + 15, oy + 15);
}

/// `mouse.rs` `draw` / `erase` since V59, on the trait.
#[allow(clippy::too_many_arguments)]
fn new_draw(backend: &dyn RasterBackend, ox: i16, oy: i16, mask: &[u16; 16], data: &[u16; 16], fg: Pel, bg: Pel, store: &mut [u32; 256]) {
    backend.read_device_rect(ox, oy, 16, 16, store);
    backend.draw_sprite(ox, oy, mask, data, fg, bg);
}
fn new_erase(backend: &dyn RasterBackend, ox: i16, oy: i16, store: &[u32; 256]) {
    backend.write_device_rect(ox, oy, 16, 16, store);
    backend.flush_rect(ox, oy, ox + 15, oy + 15);
}

static LOCK: Mutex<()> = Mutex::new(());

fn edge_positions(n: i32) -> Vec<i16> {
    let mut v: Vec<i32> = (-17..=2).collect();
    v.push(n / 2);
    v.extend(n - 17..=n + 1);
    v.sort();
    v.dedup();
    v.into_iter().map(|p| p as i16).collect()
}

#[test]
fn the_cell_paint_and_restore_equal_the_per_pixel_path() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut rng = Rng(0x5635_3900_0000_0101);
    let mut forms: Vec<([u16; 16], [u16; 16])> = PRESET_FORMS.iter().map(|f| (f.2, f.3)).collect();
    forms.push(([0xFFFF; 16], [0xFFFF; 16]));
    forms.push(([0; 16], [0; 16]));
    for _ in 0..3 {
        forms.push((core::array::from_fn(|_| rng.u16()), core::array::from_fn(|_| rng.u16())));
    }
    let mut n = 0usize;
    for (w, h, stride) in [(48i32, 36i32, 48usize), (41, 29, 44), (16, 16, 16)] {
        for oy in edge_positions(h) {
            for ox in edge_positions(w) {
                let i = n % forms.len();
                let (mask, data) = forms[i];
                let (fg, bg): (Pel, Pel) = if i.is_multiple_of(2) { (1, 0) } else { (rng.u32() as u8, rng.u32() as u8) };
                let init: Vec<u32> = (0..(h as usize - 1) * stride + w as usize).map(|_| rng.u32()).collect();
                let old = Legacy(Store::new(w, h, stride, &init));
                let leg = Legacy(Store::new(w, h, stride, &init));
                let fast = Fast(Store::new(w, h, stride, &init));
                let (mut s_old, mut s_leg, mut s_fast) = ([7u32; 256], [8u32; 256], [9u32; 256]);
                old_draw(&old, ox, oy, &mask, &data, fg, bg, &mut s_old);
                new_draw(&leg, ox, oy, &mask, &data, fg, bg, &mut s_leg);
                new_draw(&fast, ox, oy, &mask, &data, fg, bg, &mut s_fast);
                let what = format!("screen {w}x{h}/{stride} cell ({ox},{oy}) form {i} pens {fg}/{bg}");
                assert_eq!(s_leg, s_old, "save-under, defaults: {what}");
                assert_eq!(s_fast, s_old, "save-under, kernel: {what}");
                assert_eq!(leg.0.pixels(), old.0.pixels(), "painted, defaults: {what}");
                assert_eq!(fast.0.pixels(), old.0.pixels(), "painted, kernel: {what}");
                old_erase(&old, ox, oy, &s_old);
                new_erase(&leg, ox, oy, &s_leg);
                new_erase(&fast, ox, oy, &s_fast);
                assert_eq!(old.0.pixels(), init, "restored, old path: {what}");
                assert_eq!(leg.0.pixels(), init, "restored, defaults: {what}");
                assert_eq!(fast.0.pixels(), init, "restored, kernel: {what}");
                assert_eq!(leg.0.flush_log(), old.0.flush_log(), "flushes, defaults: {what}");
                assert_eq!(fast.0.flush_log(), old.0.flush_log(), "flushes, kernel: {what}");
                n += 1;
            }
        }
    }
    assert!(n > 1500, "cases run: {n}");
}

/// The pointer script: public mouse calls, the screen after each.
fn script(b: &dyn RasterBackend, flushes: &dyn Fn() -> Vec<(i16, i16, i16, i16)>, rng: &mut Rng) -> Vec<Vec<u32>> {
    let snap = |b: &dyn RasterBackend| -> Vec<u32> {
        let mut v = Vec::new();
        for y in 0..b.height() as i16 {
            for x in 0..b.width() as i16 {
                v.push(b.get_device_pixel(x, y));
            }
        }
        v
    };
    let mut shots = Vec::new();
    // A fixed start: hidden, the arrow both as current and previous form.
    mouse::remove(b);
    mouse::set_preset(0, b);
    mouse::set_preset(0, b);
    let (w, h) = (b.width() as i32, b.height() as i32);
    // Every hotspot of the presets at the edges: display, then remove. Each
    // picture also against the old per-pixel paint of that preset at
    // `(x - xhot, y - yhot)` on the ground, pens 1 on 0 (`set_form`'s).
    let ground = snap(b);
    for n in 0..8i16 {
        mouse::set_preset(n, b);
        let (hx, hy, mask, data) = PRESET_FORMS[n as usize];
        for (x, y) in [(0, 0), (-3, 5), (w - 1, h - 1), (w - 4, 2), (3, h - 2), (w / 2, h / 2), (-15, -15), (w + 14, h + 14)] {
            mouse::display_at(x as i16, y as i16, b);
            let shot = snap(b);
            let model = Legacy(Store::new(w, h, w as usize, &ground));
            old_draw(&model, x as i16 - hx, y as i16 - hy, &mask, &data, 1, 0, &mut [0; 256]);
            assert!(shot == snap(&model), "display_at({x},{y}) with preset {n} != the old paint");
            shots.push(shot);
            mouse::remove(b);
            let shot = snap(b);
            assert!(shot == ground, "remove after display_at({x},{y}) with preset {n} left pixels");
            let (ox, oy) = (x as i16 - hx, y as i16 - hy);
            assert_eq!(flushes().last(), Some(&(ox, oy, ox + 15, oy + 15)), "the erase flushes its cell");
            shots.push(shot);
        }
    }
    // Nested hide/show around a shown pointer.
    mouse::display_at(10, 10, b);
    for depth in 1..=4 {
        for _ in 0..depth {
            mouse::hide(b);
            shots.push(snap(b));
        }
        for _ in 0..depth {
            mouse::show(b);
            shots.push(snap(b));
        }
    }
    // An extra show at depth 0 is clamped (no second draw over the first).
    mouse::show(b);
    shots.push(snap(b));
    // Form changes while shown: user forms, save/restore, previous.
    for _ in 0..6 {
        let mask: [u16; 16] = core::array::from_fn(|_| rng.u16());
        let data: [u16; 16] = core::array::from_fn(|_| rng.u16());
        mouse::set_form((rng.u32() % 20) as i16, (rng.u32() % 20) as i16, &mask, &data, b);
        shots.push(snap(b));
    }
    mouse::save_form();
    mouse::set_preset(2, b);
    shots.push(snap(b));
    mouse::restore_form(b);
    shots.push(snap(b));
    mouse::previous_form(b);
    shots.push(snap(b));
    // Packets: into every clamp and back, with the pointer shown.
    mouse::set_preset(0, b);
    for (dx, dy) in [(-4000, -4000), (5, 3), (4000, 0), (0, 4000), (-7, -1), (-4000, 0), (3, -4000), (1, 1)] {
        tos_api::vdi::feed_relative(dx, dy, 0, b);
        shots.push(snap(b));
    }
    // Hidden during packets, shown again where the packets left it.
    mouse::hide(b);
    tos_api::vdi::feed_relative(9, 4, 0, b);
    shots.push(snap(b));
    mouse::show(b);
    shots.push(snap(b));
    mouse::show_reset(b);
    shots.push(snap(b));
    // End hidden: the save-under goes back, nothing is left for the next run.
    mouse::remove(b);
    shots.push(snap(b));
    shots
}

#[test]
fn the_pointer_state_machine_paints_the_same_on_both_paths() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for (w, h, stride) in [(64i32, 48i32, 64usize), (57, 33, 61)] {
        let mut ground = Rng(0x5635_3900_0000_0200 + w as u64);
        let init: Vec<u32> = (0..(h as usize - 1) * stride + w as usize).map(|_| ground.u32()).collect();
        let leg = Legacy(Store::new(w, h, stride, &init));
        let fast = Fast(Store::new(w, h, stride, &init));
        let a = script(&leg, &|| leg.0.flush_log(), &mut Rng(0x5635_3900_0000_0300));
        let b = script(&fast, &|| fast.0.flush_log(), &mut Rng(0x5635_3900_0000_0300));
        assert_eq!(a.len(), b.len());
        for (k, (x, y)) in a.iter().zip(&b).enumerate() {
            if x != y {
                let i = x.iter().zip(y).position(|(p, q)| p != q).unwrap();
                panic!("screen {w}x{h}: step {k} differs first at ({}, {})", i % w as usize, i / w as usize);
            }
        }
        assert_eq!(leg.0.pixels(), init, "the script ends with the ground restored");
        assert_eq!(fast.0.pixels(), init, "the script ends with the ground restored");
        assert_eq!(leg.0.flush_log(), fast.0.flush_log(), "flushes");
        // The script drew something at all (a pointer on the ground).
        assert!(a.iter().any(|s| *s != a[1]), "the script painted a pointer");
    }
}

//! A tiny in-kernel VDI client: builds a VDIPB and issues `svc #2`.
//!
//! Used by the boot-time [`crate::initinfo`] screen to draw through the
//! real VDI dispatcher exactly as a user program would. Each call
//! rebuilds the 5-pointer parameter block from the (stack-stable) array
//! addresses.

/// Memory Form Definition Block (native ABI: 64-bit `addr` then WORDs).
#[repr(C)]
pub struct Mfdb {
    /// Pixel data address (0 = screen).
    pub addr: u64,
    /// Width in pixels.
    pub w: i16,
    /// Height in pixels.
    pub h: i16,
    /// Width in words.
    pub wdwidth: i16,
    /// 0 = device form, 1 = standard form.
    pub stand: i16,
    /// Plane count.
    pub nplanes: i16,
    /// Reserved.
    pub r1: i16,
    /// Reserved.
    pub r2: i16,
    /// Reserved.
    pub r3: i16,
}

/// VDI parameter-block arrays + the open workstation handle.
pub struct Vdi {
    contrl: [i16; 16],
    intin: [i16; 64],
    ptsin: [i16; 32],
    intout: [i16; 45],
    ptsout: [i16; 12],
    handle: i16,
}

impl Default for Vdi {
    fn default() -> Self {
        Self::new()
    }
}

impl Vdi {
    /// A fresh client (no workstation open yet).
    pub fn new() -> Self {
        Self {
            contrl: [0; 16],
            intin: [0; 64],
            ptsin: [0; 32],
            intout: [0; 45],
            ptsout: [0; 12],
            handle: 0,
        }
    }

    /// Issue the current parameter block as a VDI call (`svc #2`).
    fn call(&mut self) {
        let block: [u64; 5] = [
            self.contrl.as_mut_ptr() as u64,
            self.intin.as_mut_ptr() as u64,
            self.ptsin.as_mut_ptr() as u64,
            self.intout.as_mut_ptr() as u64,
            self.ptsout.as_mut_ptr() as u64,
        ];
        let pb = block.as_ptr() as u64;
        // SAFETY: GEM trap (svc #2) with selector 115 (VDI) in x0 and the
        // parameter block in x1. `x0` comes back as the VDI's `flip_y`
        // (`tos_api::vdi::FLIP_Y_MAGNITUDES`); dropped here, as gemlib's
        // `vdi()` binding drops it too.
        unsafe {
            core::arch::asm!(
                "svc #2",
                inout("x0") 115u64 => _,
                in("x1") pb,
                options(nostack),
            );
        }
    }

    /// Open a virtual workstation; returns false if no handle/backend.
    pub fn open(&mut self) -> bool {
        self.contrl.fill(0);
        self.contrl[0] = 100; // v_opnvwk
        self.contrl[3] = 11;
        self.intin[..11].copy_from_slice(&[1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 2]);
        self.call();
        self.handle = self.contrl[6];
        self.handle != 0
    }

    /// Close the workstation.
    pub fn close(&mut self) {
        self.contrl.fill(0);
        self.contrl[0] = 101; // v_clsvwk
        self.contrl[6] = self.handle;
        self.call();
    }

    /// Clear the screen (v_clrwk) to the background colour.
    pub fn clear(&mut self) {
        self.contrl.fill(0);
        self.contrl[0] = 3;
        self.contrl[6] = self.handle;
        self.call();
    }

    /// Set an attribute opcode taking a single `INTIN[0]` value.
    fn set1(&mut self, opcode: i16, value: i16) {
        self.contrl.fill(0);
        self.contrl[0] = opcode;
        self.contrl[3] = 1;
        self.contrl[6] = self.handle;
        self.intin[0] = value;
        self.call();
    }

    /// Set the writing (raster) mode (vswr_mode, opcode 32): `1` replace,
    /// `2` transparent, `3` XOR, `4` reverse-transparent
    /// (`tos_api::vdi::attributes::vswr_mode`).
    ///
    /// Replace is the workstation default and paints a glyph's *clear* pixels
    /// with pen 0, so text over a filled panel needs transparent mode to keep
    /// the panel visible behind the glyph cells.
    pub fn set_write_mode(&mut self, mode: i16) {
        self.set1(32, mode);
    }

    /// Set the fill colour (vsf_color).
    pub fn set_fill_color(&mut self, color: i16) {
        self.set1(25, color);
    }
    /// Set the fill interior style (vsf_interior: 0 hollow … 4 user).
    pub fn set_fill_style(&mut self, style: i16) {
        self.set1(23, style);
    }
    /// Set the fill pattern/hatch index (vsf_style).
    pub fn set_fill_index(&mut self, index: i16) {
        self.set1(24, index);
    }

    /// Convenience: solid fill in `color`.
    pub fn set_fill(&mut self, color: i16) {
        self.set_fill_color(color);
        self.set_fill_style(1); // solid
    }

    /// Fill the rectangle `(x1,y1)-(x2,y2)` (vr_recfl).
    pub fn fill_rect(&mut self, x1: i16, y1: i16, x2: i16, y2: i16) {
        self.contrl.fill(0);
        self.contrl[0] = 114;
        self.contrl[1] = 2;
        self.contrl[6] = self.handle;
        self.ptsin[..4].copy_from_slice(&[x1, y1, x2, y2]);
        self.call();
    }

    /// Set the polyline colour (vsl_color).
    pub fn set_line(&mut self, color: i16) {
        self.set1(17, color);
    }

    /// Draw a line `(x0,y0)-(x1,y1)` (v_pline).
    pub fn line(&mut self, x0: i16, y0: i16, x1: i16, y1: i16) {
        self.contrl.fill(0);
        self.contrl[0] = 6;
        self.contrl[1] = 2;
        self.contrl[6] = self.handle;
        self.ptsin[..4].copy_from_slice(&[x0, y0, x1, y1]);
        self.call();
    }

    /// Set the text colour (vst_color).
    pub fn set_text_color(&mut self, color: i16) {
        self.set1(22, color);
    }

    /// Set text alignment (vst_alignment): horizontal 0..2, vertical 0..5.
    pub fn set_text_align(&mut self, h: i16, v: i16) {
        self.contrl.fill(0);
        self.contrl[0] = 39;
        self.contrl[3] = 2;
        self.contrl[6] = self.handle;
        self.intin[0] = h;
        self.intin[1] = v;
        self.call();
    }

    /// Draw an ASCII string at `(x, y)` (v_gtext).
    pub fn gtext(&mut self, x: i16, y: i16, text: &[u8]) {
        let n = text.len().min(self.intin.len());
        self.contrl.fill(0);
        self.contrl[0] = 8;
        self.contrl[1] = 1; // one PTSIN pair: the handler reads the VDI's copy (V35)
        self.contrl[3] = n as i16;
        self.contrl[6] = self.handle;
        self.ptsin[0] = x;
        self.ptsin[1] = y;
        for (i, &b) in text.iter().take(n).enumerate() {
            self.intin[i] = b as i16;
        }
        self.call();
    }

    /// Pack a native 64-bit value into four CONTRL WORDs at `idx`.
    fn set_contrl_u64(&mut self, idx: usize, val: u64) {
        let b = val.to_le_bytes();
        for i in 0..4 {
            self.contrl[idx + i] = i16::from_le_bytes([b[i * 2], b[i * 2 + 1]]);
        }
    }

    /// Blit the `sw×sh` mono source at `src_mfdb` to the screen at
    /// `(dx, dy)` in colour `fg`/`bg` (vrt_cpyfm `MD_*` mode:
    /// 1 = replace, 2 = transparent, 3 = XOR, 4 = erase).
    #[allow(clippy::too_many_arguments)]
    pub fn blit_mono(&mut self, src_mfdb: u64, sw: i16, sh: i16, dx: i16, dy: i16, fg: i16, bg: i16, mode: i16) {
        self.contrl.fill(0);
        self.contrl[0] = 121;
        self.contrl[1] = 4;
        self.contrl[3] = 3;
        self.contrl[6] = self.handle;
        self.set_contrl_u64(7, src_mfdb);
        self.set_contrl_u64(11, 0);
        self.ptsin[..8].copy_from_slice(&[0, 0, sw - 1, sh - 1, dx, dy, dx + sw - 1, dy + sh - 1]);
        self.intin[..3].copy_from_slice(&[mode, fg, bg]);
        self.call();
    }

    /// Copy a screen rectangle to `(dx, dy)` with one of the 16 BitBlt
    /// logic ops `op` (vro_cpyfm; 3 = replace/S, 6 = XOR/S^D, 7 = OR/S|D).
    #[allow(clippy::too_many_arguments)]
    pub fn copy_screen(&mut self, sx: i16, sy: i16, sw: i16, sh: i16, dx: i16, dy: i16, op: i16) {
        self.contrl.fill(0);
        self.contrl[0] = 109;
        self.contrl[1] = 4;
        self.contrl[3] = 1;
        self.contrl[6] = self.handle;
        self.set_contrl_u64(7, 0);
        self.set_contrl_u64(11, 0);
        self.ptsin[..8].copy_from_slice(&[sx, sy, sx + sw - 1, sy + sh - 1, dx, dy, dx + sw - 1, dy + sh - 1]);
        self.intin[0] = op;
        self.call();
    }
}

//! The machine's one **alpha (text) screen** — the VT52 console behind
//! `Bconout(2, …)`, the raw console behind `Bconout(5, …)` — and the VDI
//! `v_escape` (opcode 5) sub-functions that drive it.
//!
//! Specs: `docs/api/bios/_vt52.md` (the console: state, escape automaton,
//! every sequence, cursor, Handle 2 vs 5, decisions D1–D5/Q3/Q5/Q6),
//! `docs/api/vdi/_escape.md` (`v_escape`), `docs/api/xbios/15-cursconf.md`.
//! EmuTOS: `bios/vt52.c`, `bios/conout.c`, `vdi/vdi_esc.c`.
//!
//! **One state, as in EmuTOS.** Every alpha escape of EmuTOS' VDI is a write
//! to the single BIOS VT52 console (`bios_conws` = `while (*s)
//! Bconout(2, *s++)`, `vdi_esc.c:39-43`; `v_curtext` → `Crawio`), so the
//! cursor, the colours, the reverse/wrap flags and the screen the escapes
//! touch are the ones console output touches. NewTOS keeps that single state
//! here — the cell rendering lives in the VDI crate — and the kernel's
//! console adapter (`kernel::fbcon`, the `Bconout(2)` device) writes into it
//! through [`conout`]; `Bconout(5)` reaches [`rawout`] directly
//! (`bios::chardev`).
//!
//! **Rendering** follows EmuTOS' Falcon 16-bit path (`TRUECOLOR_MODE`,
//! `conout.c:137-180,297-331,542-575`): a cell is eight 16-bit screen words
//! per glyph row, written and read through the backend's row interface;
//! the cursor is the cell's bitwise complement (`neg_cell`). Colours are
//! hardware registers (`v_col_fg`/`v_col_bg`, 0..15) resolved through the
//! physical workstation's pseudo-palette, or `falcon_default_palette` while
//! no workstation has been opened (decision D1).
//!
//! **Deliberate deviations** (`docs/COMPATIBILITY.md`, CON04): the
//! reverse-video border cell of `ESC K`/`ESC o`/`ESC d`/`ESC J` (Q3), the
//! blink clock (Q5), `vs_curaddress` without the byte detour (Q6), and BEL
//! without a sound (D4).

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use super::backend::{ClipRect, RasterBackend, Rect};
use super::{font, mouse, vwk, Vdipb};

// ── v_stat_0 (EmuTOS `bios/conout.h:17-30`) ─────────────────────────────

/// Cursor flashes (1) / is steady (0).
const M_CFLASH: u8 = 0x01;
/// Blink phase: the cursor cell is currently inverted.
const M_CSTATE: u8 = 0x02;
/// Cursor visible; also the semaphore against the blink interrupt.
const M_CVIS: u8 = 0x04;
/// End of line: wrap (1) / overwrite the last cell (0).
const M_CEOL: u8 = 0x08;
/// Reverse video on.
const M_REVID: u8 = 0x10;
/// A position saved by `ESC j` is present.
const M_SVPOS: u8 = 0x20;

/// `conterm` bit 2: the console bell is enabled (`vt52.c:216`).
const CONTERM_BELL: u8 = 0x04;

/// The escape character.
const ESC: u8 = 0x1b;

/// EmuTOS `falcon_default_palette` (`bios/conout.c:31-34`): the 16-bit
/// screen word of hardware register `n`, used while no workstation has been
/// opened (`conout.c:169-170`, `blank_out16` `:557`).
const FALCON_DEFAULT_PALETTE: [u16; 16] = [
    0xffdf, 0xf800, 0x07c0, 0xffc0, 0x001f, 0xf81f, 0x07df, 0xbdd7, //
    0x8c51, 0xa800, 0x0540, 0xad40, 0x0015, 0xa815, 0x0555, 0x0000,
];

/// EmuTOS `REV_MAP_COL` for registers 0..15 on a 256-pen device
/// (`vdi/vdi_col.c:28-29,736-747`: `MAP_COL_ROM` with `MAP_COL[1] = 255`
/// and `MAP_COL[255] = 15`, inverted). EmuTOS' pseudo-palette is indexed by
/// hardware register; NewTOS folds `MAP_COL` and indexes it by pen
/// (`vwk::VwkExt`), so the word EmuTOS reads at `palette[r]`
/// (`conout.c:162-166`) is at `palette[REV_MAP_COL[r]]` here.
const REV_MAP_COL: [u8; 16] = [0, 2, 3, 6, 4, 7, 5, 8, 9, 10, 11, 14, 12, 15, 13, 255];

/// Blink ticks of the 200 Hz counter per `v_period` unit. EmuTOS counts
/// `v_period` in VBLs (`blink()` once per VBL, `vectors.S:355`) and calls the
/// default 30 "0.5 second blink rate (@ 60Hz vblank)" (`vt52.c:933`); NewTOS
/// has no VBL and fixes the unit at 1/60 s on the 200 Hz counter, so the
/// default is exactly 0.5 s on every board (decision Q5).
const TICKS_PER_PERIOD_NUM: u32 = 10;
const TICKS_PER_PERIOD_DEN: u32 = 3;

/// The automaton's state — EmuTOS `con_state` (sysvar `0x4a8`), a function
/// pointer there (`vt52.c:142-160`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ConState {
    /// `normal_ascii` (`vt52.c:166-201`).
    Normal,
    /// `esc_ch1` (`:242-255`).
    Esc,
    /// `get_row` (`:260-265`).
    Row,
    /// `get_column` (`:270-286`).
    Column,
    /// `get_fg_col` (`:292-303`).
    FgCol,
    /// `get_bg_col` (`:309-320`).
    BgCol,
}

/// The console state (`docs/api/bios/_vt52.md` §1).
struct Alpha {
    state: ConState,
    /// `save_row` (sysvar `0x4ac`).
    save_row: i16,
    /// `v_cur_cx`, `v_cur_cy`.
    cx: i16,
    cy: i16,
    /// `v_cel_mx`, `v_cel_my` — from the backend geometry at every entry.
    mx: i16,
    my: i16,
    /// `v_stat_0`.
    stat: u8,
    /// `disab_cnt` (a `WORD`).
    disab_cnt: i16,
    /// `sav_cur_x`, `sav_cur_y`.
    sav_x: i16,
    sav_y: i16,
    /// `v_col_fg`, `v_col_bg` — hardware registers, `ch & 0x0f`.
    fg: u8,
    bg: u8,
    /// `v_period` (`Cursconf` 4/5).
    period: u8,
    /// `_hz_200` value at which the blink timer runs out (EmuTOS `v_cur_tim`
    /// counted down per VBL).
    blink_due: u32,
}

impl Alpha {
    /// The state `vt52_init` leaves (`vt52.c:921-953`) — also the power-up
    /// value, so nothing is lost if output arrives before [`init`].
    const INIT: Alpha = Alpha {
        state: ConState::Normal,
        save_row: 0,
        cx: 0,
        cy: 0,
        mx: 0,
        my: 0,
        stat: M_CFLASH,
        disab_cnt: 1,
        sav_x: 0,
        sav_y: 0,
        fg: 15,
        bg: 0,
        period: 30,
        blink_due: 0,
    };
}

struct AlphaCell(UnsafeCell<Alpha>);
// SAFETY: access is serialised by `BUSY` (see `Guard`).
unsafe impl Sync for AlphaCell {}

static ALPHA: AlphaCell = AlphaCell(UnsafeCell::new(Alpha::INIT));

/// Held while any console operation runs. The blink, which the kernel calls
/// from the timer interrupt, only *tries* to take it and skips the tick when
/// the console is busy — the role of EmuTOS' `M_CVIS` semaphore
/// (`conout.h:21-25`).
static BUSY: AtomicBool = AtomicBool::new(false);

/// Exclusive access to the console state for one operation, with the
/// backend (`None` = no screen: the state still changes, nothing is drawn).
struct Con<'a> {
    st: &'static mut Alpha,
    b: Option<&'a dyn RasterBackend>,
}

/// `font_set_default`'s console font for the screen `b` (`bios/font.c:72-88`:
/// `(V_REZ_VT < 400) ? &fon8x8 : &fon8x16`): its `max_cell_width` ×
/// `form_height` is the character cell (`v_cel_ht`), its rows the glyphs.
fn con_font(b: &dyn RasterBackend) -> &'static font::SysFont {
    font::font(font::font_set_default(b.height() as i16))
}

/// The console cell `(width, height)` in pixels on the screen `b`.
fn cell(b: &dyn RasterBackend) -> (i16, i16) {
    let f = con_font(b);
    (f.max_cell_width, f.form_height)
}

impl Drop for Con<'_> {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}

/// Take the console. Single CPU: a caller never finds it held except from a
/// second host thread (host tests), so this waits.
fn lock(b: Option<&dyn RasterBackend>) -> Con<'_> {
    while BUSY
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    con(b)
}

/// Take the console if it is free (interrupt context).
fn try_lock(b: Option<&dyn RasterBackend>) -> Option<Con<'_>> {
    BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .ok()
        .map(|_| con(b))
}

fn con(b: Option<&dyn RasterBackend>) -> Con<'_> {
    // SAFETY: `BUSY` is held; nobody else touches `ALPHA` until `Con` drops.
    let st = unsafe { &mut *ALPHA.0.get() };
    if let Some(b) = b {
        // `font_set_default` (`bios/font.c:72-88`): last column/row of the
        // cell grid of the console font for this screen height.
        let (cw, ch) = cell(b);
        st.mx = (b.width() as i16 / cw - 1).max(0);
        st.my = (b.height() as i16 / ch - 1).max(0);
        st.cx = st.cx.min(st.mx);
        st.cy = st.cy.min(st.my);
    }
    Con { st, b }
}

fn hz_200() -> u32 {
    crate::bios::tick::hz_200()
}

impl Con<'_> {
    // ── colours and cells ────────────────────────────────────────────────

    /// The 16-bit screen word of hardware register `reg` (D1): the physical
    /// workstation's pseudo-palette once it has been opened
    /// (`phys_work.ext->palette[reg]`, `conout.c:162-166`), else
    /// `falcon_default_palette` (`:169-170`).
    fn word(&self, reg: u8) -> u16 {
        let reg = reg & 0x0f;
        // SAFETY: single trap path, as for the rest of the VDI state.
        let phys_open = unsafe { vwk::resolve(vwk::PHYS_HANDLE) }.is_some();
        if phys_open {
            // SAFETY: as above.
            if let Some(ext) = unsafe { vwk::ext(vwk::PHYS_HANDLE) } {
                return ext.palette[REV_MAP_COL[reg as usize] as usize];
            }
        }
        FALCON_DEFAULT_PALETTE[reg as usize]
    }

    fn flush_cells(&self, x0: i16, y0: i16, x1: i16, y1: i16) {
        if let Some(b) = self.b {
            let (cw, ch) = cell(b);
            b.flush_rect(x0 * cw, y0 * ch, (x1 + 1) * cw - 1, (y1 + 1) * ch - 1);
        }
    }

    /// `cell_xfer` (`conout.c:137-180,202-279`): the glyph of `ch` in the
    /// foreground on the background, swapped under reverse video
    /// (`:150-156,221-228`); the whole cell is replaced.
    fn cell_xfer(&self, x: i16, y: i16, ch: u8) {
        let Some(b) = self.b else { return };
        let (mut fg, mut bg) = (self.word(self.st.fg), self.word(self.st.bg));
        if self.st.stat & M_REVID != 0 {
            core::mem::swap(&mut fg, &mut bg);
        }
        let glyph = font::glyph_rows(con_font(b).id, ch as u16);
        let (cw, chh) = cell(b);
        let (px, py) = (x * cw, y * chh);
        for (row, &bits) in glyph.iter().enumerate() {
            let mut line = [bg; 8];
            for (i, w) in line.iter_mut().enumerate() {
                if bits & (0x80 >> i) != 0 {
                    *w = fg;
                }
            }
            b.write_row_rgb565(px, py + row as i16, &line);
        }
        self.flush_cells(x, y, x, y);
    }

    /// `neg_cell` (`conout.c:297-331`): complement every screen word of the
    /// cell — the cursor. A second call restores the cell.
    fn neg_cell(&self, x: i16, y: i16) {
        let Some(b) = self.b else { return };
        let (cw, ch) = cell(b);
        let (px, py) = (x * cw, y * ch);
        for row in 0..ch {
            let mut line = [0u16; 8];
            b.read_row_rgb565(px, py + row, &mut line);
            for w in line.iter_mut() {
                *w = !*w;
            }
            b.write_row_rgb565(px, py + row, &line);
        }
        self.flush_cells(x, y, x, y);
    }

    /// `blank_out` (`conout.c:598-684`, `blank_out16` `:542-575`): fill the
    /// cell rectangle `(x0,y0)..=(x1,y1)` with the background register —
    /// never the reverse-video foreground (`:611`).
    fn blank_out(&self, x0: i16, y0: i16, x1: i16, y1: i16) {
        let Some(b) = self.b else { return };
        if x0 > x1 || y0 > y1 {
            return;
        }
        let word = self.word(self.st.bg);
        let line = [word; 64];
        let (cw, ch) = cell(b);
        let (px0, px1) = (x0 * cw, (x1 + 1) * cw);
        for py in (y0 * ch)..((y1 + 1) * ch) {
            let mut x = px0;
            while x < px1 {
                let n = ((px1 - x) as usize).min(line.len());
                b.write_row_rgb565(x, py, &line[..n]);
                x += n as i16;
            }
        }
        self.flush_cells(x0, y0, x1, y1);
    }

    /// `scroll_up(top)` (`conout.c:706-725`): rows `top+1..=my` move up one
    /// row, the bottom row is blanked. The cursor is not moved.
    fn scroll_up(&self, top: i16) {
        let (mx, my) = (self.st.mx, self.st.my);
        if let Some(b) = self.b {
            if top < my {
                let (cw, ch) = cell(b);
                let src = Rect { x: 0, y: (top + 1) * ch, w: (mx + 1) * cw, h: (my - top) * ch };
                b.copy_rect(src, 0, top * ch, 3, ClipRect::full(b.width(), b.height()));
            }
        }
        self.blank_out(0, my, mx, my);
    }

    /// `scroll_down(start)` (`conout.c:733-752`): rows `start..my-1` move
    /// down one row (the bottom row falls out), row `start` is blanked.
    fn scroll_down(&self, start: i16) {
        let (mx, my) = (self.st.mx, self.st.my);
        if let Some(b) = self.b {
            if start < my {
                let (cw, ch) = cell(b);
                let src = Rect { x: 0, y: start * ch, w: (mx + 1) * cw, h: (my - start) * ch };
                b.copy_rect(src, 0, (start + 1) * ch, 3, ClipRect::full(b.width(), b.height()));
            }
        }
        self.blank_out(0, start, mx, start);
    }

    // ── cursor (`conout.c:416-470`, `vt52.c:612-688,849-873`) ─────────────

    fn reload_blink(&mut self) {
        let period = if self.st.period == 0 { 256 } else { u32::from(self.st.period) };
        self.st.blink_due = hz_200().wrapping_add(period * TICKS_PER_PERIOD_NUM / TICKS_PER_PERIOD_DEN);
    }

    /// `move_cursor(x, y)` (`conout.c:416-470`): clamp, move, and carry a
    /// visible cursor along.
    fn move_cursor(&mut self, x: i16, y: i16) {
        let x = x.clamp(0, self.st.mx);
        let y = y.clamp(0, self.st.my);
        let (ox, oy) = (self.st.cx, self.st.cy);
        self.st.cx = x;
        self.st.cy = y;
        if self.st.stat & M_CVIS == 0 {
            return;
        }
        if self.st.stat & M_CFLASH != 0 && self.st.stat & M_CSTATE == 0 {
            // Blinking and currently off: show it at the new place.
            self.neg_cell(x, y);
            self.st.stat |= M_CSTATE;
            self.reload_blink();
            return;
        }
        self.neg_cell(ox, oy);
        self.neg_cell(x, y);
        self.reload_blink();
    }

    /// `do_cnt_esce` (`vt52.c:612-624`).
    fn do_cnt_esce(&mut self) {
        self.neg_cell(self.st.cx, self.st.cy);
        self.st.stat |= M_CVIS;
        if self.st.stat & M_CFLASH != 0 {
            self.st.stat |= M_CSTATE;
            self.reload_blink();
        }
    }

    /// `cursor_on` (`vt52.c:630-643`) — `ESC e`, `Cursconf(1)`: forced.
    fn cursor_on(&mut self) {
        if self.st.disab_cnt == 0 {
            return;
        }
        self.st.disab_cnt = 0;
        self.do_cnt_esce();
    }

    /// `cursor_on_cnt` (`vt52.c:649-658`) — counted.
    fn cursor_on_cnt(&mut self) {
        if self.st.disab_cnt == 0 {
            return;
        }
        self.st.disab_cnt = self.st.disab_cnt.wrapping_sub(1);
        if self.st.disab_cnt == 0 {
            self.do_cnt_esce();
        }
    }

    /// `cursor_off` (`vt52.c:664-688`) — `ESC f`, `Cursconf(0)`, and the
    /// bracket of every internal operation.
    fn cursor_off(&mut self) {
        self.st.disab_cnt = self.st.disab_cnt.wrapping_add(1);
        if self.st.stat & M_CVIS == 0 {
            return;
        }
        self.st.stat &= !M_CVIS;
        if self.st.stat & M_CFLASH == 0 {
            self.neg_cell(self.st.cx, self.st.cy);
        } else if self.st.stat & M_CSTATE != 0 {
            self.st.stat &= !M_CSTATE;
            self.neg_cell(self.st.cx, self.st.cy);
        }
    }

    /// `blink` (`vt52.c:849-873`), run against the 200 Hz counter (Q5).
    fn blink(&mut self) {
        if self.st.stat & M_CVIS == 0 || self.st.stat & M_CFLASH == 0 {
            return;
        }
        if (hz_200().wrapping_sub(self.st.blink_due) as i32) < 0 {
            return;
        }
        self.reload_blink();
        self.st.stat ^= M_CSTATE;
        self.neg_cell(self.st.cx, self.st.cy);
    }

    // ── output ───────────────────────────────────────────────────────────

    /// `ascii_out` (`conout.c:482-532`): the glyph at the cursor, then
    /// advance — overwrite the last cell without `M_CEOL`, else CR+LF with a
    /// scroll at the bottom. The 8×16 system font covers 0..255
    /// (`fnt_st_8x16.c:278-279`), so every byte has a glyph.
    fn ascii_out(&mut self, ch: u8) {
        let visible = self.st.stat & M_CVIS != 0;
        if visible {
            self.st.stat &= !M_CVIS;
        }
        self.cell_xfer(self.st.cx, self.st.cy, ch);
        if self.st.cx == self.st.mx {
            if self.st.stat & M_CEOL != 0 {
                self.st.cx = 0;
                if self.st.cy < self.st.my {
                    self.st.cy += 1;
                } else {
                    self.scroll_up(0);
                }
            }
        } else {
            self.st.cx += 1;
        }
        if visible {
            self.neg_cell(self.st.cx, self.st.cy);
            self.st.stat |= M_CSTATE | M_CVIS;
            if self.st.stat & M_CFLASH != 0 {
                self.reload_blink();
            }
        }
    }

    /// `cputc` (`vt52.c:142-160`): one byte through the automaton.
    fn cputc(&mut self, ch: u8) {
        match self.st.state {
            ConState::Normal => self.normal_ascii(ch),
            ConState::Esc => self.esc_ch1(ch),
            ConState::Row => {
                // `get_row`: any byte, no check (`:263`).
                self.st.save_row = i16::from(ch) - 32;
                self.st.state = ConState::Column;
            }
            ConState::Column => {
                // `get_column` (`:278-285`); `move_cursor` clamps.
                let row = self.st.save_row;
                self.move_cursor(i16::from(ch) - 32, row);
                self.st.state = ConState::Normal;
            }
            ConState::FgCol => {
                self.st.fg = ch & 0x0f; // `:301`
                self.st.state = ConState::Normal;
            }
            ConState::BgCol => {
                self.st.bg = ch & 0x0f; // `:318`
                self.st.state = ConState::Normal;
            }
        }
    }

    /// `normal_ascii` (`vt52.c:166-201`) with `cntl_tab` (`:128-136`).
    fn normal_ascii(&mut self, ch: u8) {
        match ch {
            0x20..=0xff => self.ascii_out(ch),
            ESC => self.st.state = ConState::Esc,
            0x07 => do_bell(),
            0x08 => self.cursor_left(), // `do_backspace` = `cursor_left_impl`
            0x09 => {
                // `do_tab` (`:233-236`): next multiple of 8, clamped.
                let x = (self.st.cx & !7) + 8;
                let y = self.st.cy;
                self.move_cursor(x, y);
            }
            0x0a..=0x0c => self.ascii_lf(), // LF, VT, FF
            0x0d => {
                let y = self.st.cy;
                self.move_cursor(0, y); // `ascii_cr` (`:821-825`)
            }
            _ => {} // 0..6, 14..26, 28..31: discarded (`:200`)
        }
    }

    /// `esc_ch1` (`vt52.c:242-255`): back to `normal_ascii` first; an
    /// unknown byte is swallowed.
    fn esc_ch1(&mut self, ch: u8) {
        self.st.state = ConState::Normal;
        match ch {
            // `cursor_up`/`cursor_down`/`cursor_right` (`:359-404`): one
            // cell, nothing at the edge (no scroll, no wrap).
            b'A' if self.st.cy != 0 => {
                let (x, y) = (self.st.cx, self.st.cy - 1);
                self.move_cursor(x, y);
            }
            b'B' if self.st.cy != self.st.my => {
                let (x, y) = (self.st.cx, self.st.cy + 1);
                self.move_cursor(x, y);
            }
            b'C' if self.st.cx != self.st.mx => {
                let (x, y) = (self.st.cx + 1, self.st.cy);
                self.move_cursor(x, y);
            }
            b'D' => self.cursor_left(),
            b'E' => self.clear_and_home(),
            b'H' => self.move_cursor(0, 0),
            b'I' => self.reverse_linefeed(),
            b'J' => self.erase_to_eos(),
            b'K' => self.erase_to_eol(),
            b'L' => self.insert_line(),
            b'M' => self.delete_line(),
            b'Y' => self.st.state = ConState::Row,
            b'b' => self.st.state = ConState::FgCol,
            b'c' => self.st.state = ConState::BgCol,
            b'd' => self.erase_from_home(),
            b'e' => self.cursor_on(),
            b'f' => self.cursor_off(),
            b'j' => {
                // `save_cursor_pos` (`:694-706`)
                self.st.stat |= M_SVPOS;
                self.st.sav_x = self.st.cx;
                self.st.sav_y = self.st.cy;
            }
            b'k' => {
                // `restore_cursor_pos` (`:712-725`)
                let (x, y) = if self.st.stat & M_SVPOS != 0 {
                    (self.st.sav_x, self.st.sav_y)
                } else {
                    (0, 0)
                };
                self.move_cursor(x, y);
                self.st.stat &= !M_SVPOS;
            }
            b'l' => {
                // `erase_line` (`:733-743`)
                self.cursor_off();
                let (mx, y) = (self.st.mx, self.st.cy);
                self.blank_out(0, y, mx, y);
                self.move_cursor(0, y);
                self.cursor_on_cnt();
            }
            b'o' => self.erase_from_bol(),
            b'p' => self.st.stat |= M_REVID,
            b'q' => self.st.stat &= !M_REVID,
            b'v' => self.st.stat |= M_CEOL,
            b'w' => self.st.stat &= !M_CEOL,
            // `nop` table entries (F, G, g, h, i, m, n, r..u), A/B/C at the
            // edge, and every byte outside 'A'..'M' / 'b'..'w': no effect.
            _ => {}
        }
    }

    /// `cursor_left_impl` (`vt52.c:410-414`) — `ESC D` and BS.
    fn cursor_left(&mut self) {
        if self.st.cx != 0 {
            let (x, y) = (self.st.cx - 1, self.st.cy);
            self.move_cursor(x, y);
        }
    }

    /// `ascii_lf` (`vt52.c:831-841`): down one row, the column stays; at the
    /// bottom the screen scrolls.
    fn ascii_lf(&mut self) {
        if self.st.cy != self.st.my {
            let (x, y) = (self.st.cx, self.st.cy + 1);
            self.move_cursor(x, y);
        } else {
            self.cursor_off();
            self.scroll_up(0);
            self.cursor_on_cnt();
        }
    }

    /// `clear_and_home` (`vt52.c:338-353`) — `ESC E`.
    fn clear_and_home(&mut self) {
        self.cursor_off();
        self.move_cursor(0, 0);
        let (mx, my) = (self.st.mx, self.st.my);
        self.blank_out(0, 0, mx, my);
        self.cursor_on_cnt();
    }

    /// `reverse_linefeed` (`vt52.c:545-556`) — `ESC I`.
    fn reverse_linefeed(&mut self) {
        if self.st.cy != 0 {
            let (x, y) = (self.st.cx, self.st.cy - 1);
            self.move_cursor(x, y);
        } else {
            let savex = self.st.cx;
            self.insert_line();
            self.move_cursor(savex, 0);
        }
    }

    /// `insert_line` (`vt52.c:562-571`) — `ESC L`.
    fn insert_line(&mut self) {
        self.cursor_off();
        self.scroll_down(self.st.cy);
        let y = self.st.cy;
        self.move_cursor(0, y);
        self.cursor_on_cnt();
    }

    /// `delete_line` (`vt52.c:577-586`) — `ESC M`.
    fn delete_line(&mut self) {
        self.cursor_off();
        self.scroll_up(self.st.cy);
        let y = self.st.cy;
        self.move_cursor(0, y);
        self.cursor_on_cnt();
    }

    /// `erase_to_eol_impl` (`vt52.c:471-500`) — `ESC K`, and the first half
    /// of `ESC J`. EmuTOS clears the cell at an odd column (or the last one)
    /// with `ascii_out(' ')`, which paints it in the *foreground* under
    /// reverse video, because `blank_out` needs an even start column on
    /// interleaved bit-planes (`conout.c:586-596`). NewTOS blanks the whole
    /// run with the background (decision Q3, `docs/COMPATIBILITY.md`).
    fn erase_to_eol(&mut self) {
        self.cursor_off();
        let (x, y, mx) = (self.st.cx, self.st.cy, self.st.mx);
        self.blank_out(x, y, mx, y);
        self.move_cursor(x, y);
        self.cursor_on_cnt();
    }

    /// `erase_to_eos` (`vt52.c:451-465`) — `ESC J`.
    fn erase_to_eos(&mut self) {
        self.erase_to_eol();
        if self.st.cy != self.st.my {
            let (mx, my, y) = (self.st.mx, self.st.my, self.st.cy);
            self.blank_out(0, y + 1, mx, my);
        }
    }

    /// `erase_from_bol_impl` (`vt52.c:752-775`) — `ESC o`, and the first
    /// half of `ESC d`: column 0 through the cursor cell, inclusive. Same
    /// border-cell decision as [`Self::erase_to_eol`] (Q3).
    fn erase_from_bol(&mut self) {
        self.cursor_off();
        let (x, y) = (self.st.cx, self.st.cy);
        self.blank_out(0, y, x, y);
        self.move_cursor(x, y);
        self.cursor_on_cnt();
    }

    /// `erase_from_home` (`vt52.c:592-606`) — `ESC d`.
    fn erase_from_home(&mut self) {
        self.erase_from_bol();
        if self.st.cy != 0 {
            let (mx, y) = (self.st.mx, self.st.cy);
            self.blank_out(0, 0, mx, y - 1);
        }
    }

    fn puts(&mut self, s: &[u8]) {
        for &c in s {
            self.cputc(c);
        }
    }
}

/// `do_bell` (`vt52.c:213-218`): `if (conterm & 4) bell()`. The byte is
/// consumed and draws nothing either way.
fn do_bell() {
    if crate::bios::chardev::current_conterm() & CONTERM_BELL != 0 {
        bell();
    }
}

/// EmuTOS `bell()` (`bios/sound.c:251-254`) calls `bell_hook`, whose default
/// `do_bell` plays the bell sound only with a YM2149 (`:256-261`). NewTOS
/// has no audio device and no `bell_hook` system variable, so there is
/// nothing to sound — the documented absence of the device (decision D4,
/// `docs/COMPATIBILITY.md`), the same result as EmuTOS without
/// `CONF_WITH_YM2149`.
fn bell() {}

// ── public entry points ────────────────────────────────────────────────

/// `vt52_init` (`vt52.c:921-953`): reset the console — cursor home and
/// hidden (`disab_cnt = 1`), blinking, no wrap, normal video, no saved
/// position, foreground register 15 (the ≥ 4-plane and truecolor case,
/// `:938-948`), background 0, period 30 — and clear the screen. The kernel
/// calls it once the framebuffer is up, where EmuTOS' `bios_init` does
/// (`bios/bios.c:341`).
pub fn init(backend: &dyn RasterBackend) {
    let mut c = lock(Some(backend));
    let (mx, my) = (c.st.mx, c.st.my);
    *c.st = Alpha::INIT;
    c.st.mx = mx;
    c.st.my = my;
    c.reload_blink();
    c.clear_and_home();
}

/// `Bconout(2, ch)` — `cputc(LOBYTE(ch))` (`bios/chardev.c:91-95`,
/// `vt52.c:142-160`): one byte through the VT52 automaton. The kernel's
/// `Bconout(2)` device (`kernel::fbcon`) is a thin adapter onto this.
pub fn conout(backend: &dyn RasterBackend, ch: u8) {
    lock(Some(backend)).cputc(ch);
}

/// `Bconout(5, ch)` — `ascii_out(LOBYTE(ch))` (`bios/chardev.c:97-106`):
/// the glyph of every byte, no automaton, and the automaton's state (a
/// half-received `ESC` sequence of Handle 2) is left alone (decision D5).
pub fn rawout(backend: &dyn RasterBackend, ch: u8) {
    lock(Some(backend)).ascii_out(ch);
}

/// `ESC E` from outside the byte stream — clear the screen and home the
/// cursor (`clear_and_home`, `vt52.c:338-353`). The kernel's own console
/// (`cls`, the EmuCON stand-in) uses it.
pub fn clear_home(backend: &dyn RasterBackend) {
    lock(Some(backend)).clear_and_home();
}

/// One blink step — EmuTOS runs `blink()` in every VBL
/// (`bios/vectors.S:355`); the kernel calls this from its 200 Hz timer
/// interrupt. Skipped while a console operation is in progress.
pub fn blink(backend: &dyn RasterBackend) {
    if let Some(mut c) = try_lock(Some(backend)) {
        c.blink();
    }
}

/// `Cursconf(function, operand)` (`vt52.c:893-914`): 0 `cursor_off`,
/// 1 `cursor_on`, 2 clear `M_CFLASH`, 3 set `M_CFLASH`, 4 `v_period =
/// LOBYTE(operand)`, 5 return `v_period`; every other value returns 0.
///
/// EmuTOS' `blink()` flashes only with `M_CFLASH` **set** (`:856`,
/// `conout.h:17`), so subfunction 2 makes the cursor steady and 3 makes it
/// blink — the code's behaviour, whatever its comments say.
pub fn cursconf(backend: Option<&dyn RasterBackend>, function: i16, operand: i16) -> i16 {
    let mut c = lock(backend);
    match function {
        0 => c.cursor_off(),
        1 => c.cursor_on(),
        2 => c.st.stat &= !M_CFLASH,
        3 => c.st.stat |= M_CFLASH,
        4 => c.st.period = operand as u8,
        5 => return i16::from(c.st.period),
        _ => {}
    }
    0
}

/// `esc_init` (`vdi_esc.c:356-359`) = `escfn2`, run by `v_opnwk`
/// (`vdi_control.c:564`): home, hide the cursor, clear.
pub fn esc_init(backend: &dyn RasterBackend) {
    lock(Some(backend)).puts(b"\x1bH\x1bf\x1bE");
}

/// `esc_exit` (`vdi_esc.c:365-368`) = `escfn3`, run by `v_clswk`
/// (`vdi_control.c:589`): clear, show the cursor, CR.
pub fn esc_exit(backend: &dyn RasterBackend) {
    lock(Some(backend)).puts(b"\x1bE\x1be\r");
}

/// Opcode 5 — `v_escape`: dispatch on `CONTRL[5]`.
///
/// The alpha sub-functions write their VT52 strings into the console
/// automaton (`bios_conws`, `vdi_esc.c:39-43`), so a half-received sequence
/// interacts with them exactly as on EmuTOS — which is why `v_exit_cur`
/// starts with `ESC H` (`vdi_esc.c:74-84`).
pub fn v_escape(pb: &Vdipb, backend: &dyn RasterBackend) {
    match pb.contrl(5) {
        1 => {
            // vq_chcells (`escfn1`, `:63-68`): rows, columns.
            let c = lock(Some(backend));
            pb.set_intout(0, c.st.my + 1);
            pb.set_intout(1, c.st.mx + 1);
            pb.set_contrl(4, 2);
        }
        2 => lock(Some(backend)).puts(b"\x1bH\x1bf\x1bE"), // v_exit_cur (`:83`)
        3 => lock(Some(backend)).puts(b"\x1bE\x1be\r"),    // v_enter_cur (`:95`)
        4 => lock(Some(backend)).puts(b"\x1bA"),           // v_curup
        5 => lock(Some(backend)).puts(b"\x1bB"),           // v_curdown
        6 => lock(Some(backend)).puts(b"\x1bC"),           // v_curright
        7 => lock(Some(backend)).puts(b"\x1bD"),           // v_curleft
        8 => lock(Some(backend)).puts(b"\x1bH"),           // v_curhome
        9 => lock(Some(backend)).puts(b"\x1bJ"),           // v_eeos
        10 => lock(Some(backend)).puts(b"\x1bK"),          // v_eeol
        11 => {
            // vs_curaddress: INTIN[0] = row, INTIN[1] = column, 1-based. EmuTOS
            // sends `ESC Y` with the values cut to bytes (`:177-182`), which
            // cannot reach a column beyond 223; NewTOS sets the position
            // directly through `move_cursor`'s clamp (decision Q6).
            let row = (i32::from(pb.intin(0)) - 1).clamp(i16::MIN.into(), i16::MAX.into()) as i16;
            let col = (i32::from(pb.intin(1)) - 1).clamp(i16::MIN.into(), i16::MAX.into()) as i16;
            lock(Some(backend)).move_cursor(col, row);
        }
        12 => {
            // v_curtext (`escfn12`, `:196-207`): every character through
            // `Crawio` — GEMDOS raw output to stdout's device, i.e. the VT52
            // automaton when stdout is the console, including `Crawio`'s
            // `0x00FF` read sentinel (`docs/api/vdi/_escape.md`, D-083).
            let n = pb.contrl(3).max(0) as usize;
            for i in 0..n {
                crate::gemdos::console::rawio(pb.intin(i) as u16);
            }
        }
        13 => lock(Some(backend)).puts(b"\x1bp"), // v_rvon
        14 => lock(Some(backend)).puts(b"\x1bq"), // v_rvoff
        15 => {
            // vq_curaddress (`escfn15`, `:231-236`): row, column (1-based).
            let c = lock(Some(backend));
            pb.set_intout(0, c.st.cy + 1);
            pb.set_intout(1, c.st.cx + 1);
            pb.set_contrl(4, 2);
        }
        16 => {
            // vq_tabstatus: a mouse/tablet device IS available — NewTOS has a
            // USB-HID mouse. EmuTOS escfn16 unconditionally returns 1
            // (`vdi_esc.c:248-252`); the function name is deceptive.
            pb.set_intout(0, 1);
            pb.set_contrl(4, 1);
        }
        // v_hardcopy: EmuTOS escfn17 is `Scrdmp()` (`vdi_esc.c:260-263`); the
        // XBIOS call re-arms `dumpflg` and prints nothing (no printer, its
        // `dump_vec` is not called: ADR-031).
        17 => {
            crate::xbios::system::scrdmp();
        }
        // v_dspcur: EmuTOS escfn18 ignores the PTSIN coordinates and force-shows
        // the cursor at its current position (`vdi_esc.c:274-278`). It says
        // "show regardless" by storing `INTIN[0] = 0` into the *caller's*
        // input array and delegating to `vdi_v_show_c`; the write is visible
        // to the caller and the TT reference records it, so it is reproduced.
        18 => {
            pb.set_intin(0, 0);
            mouse::force_show(backend);
        }
        // v_rmcur: EmuTOS escfn19 = vdi_v_hide_c → increments the hide count
        // (not an absolute hide), so it balances against a matching show.
        19 => mouse::v_hide_c(backend),
        _ => {} // > 19 → no-op
    }
}

/// Test support: the console's cursor position, `(column, row)`.
#[doc(hidden)]
pub fn cursor_cell() -> (i16, i16) {
    let c = lock(None);
    (c.st.cx, c.st.cy)
}

//! Harness for the VDI **raster** reference-conformance tests.
//!
//! These tests replay measured TOS behaviour against `tos_api::vdi`'s three
//! raster opcodes — 109 `vro_cpyfm`, 110 `vr_trnfm`, 121 `vrt_cpyfm`. The
//! reference is a dump taken by `tosdump/` on the oracle — **EmuTOS 1a7484ab
//! on Hatari's Atari TT** (`docs/analysis/ORACLE.md`; the real TT under
//! EmuTOS 1.4 is `EMUTOS14-RASTER.BIN`),
//! `tosdump/reference/EMUTOS-1a7484ab-RASTER.BIN`, produced by
//! `DUMPRAST` from `tosdump/cases/vdi-raster.tsv` (CRC `702EEA7D`, 59 cases) —
//! and `docs/api/vdi/raster/_raster.md` was extracted from that same EmuTOS
//! source, so a difference here normally has exactly one reading: NewTOS is
//! wrong. The two exceptions are named in [`MACHINE_EXCLUDED`] and
//! [`DEVIATIONS`]; nothing is skipped, and no case is `#[ignore]`d.
//!
//! **What these tests cover that the VDI ones cannot.** A raster call's result
//! is *memory*, not a number in `INTOUT`. So the record carries, on top of the
//! five parameter arrays, the two MFDBs as they stood after the call
//! (`vr_trnfm` overwrites the destination's `fd_stand`) and both pixel buffers
//! whole. This harness therefore compares **pixels** — which the
//! `vdi_reference` harness explicitly does not
//! (`tos-api/tests/vdi_reference/harness.rs`). Every `vr_trnfm` plane
//! permutation at 1/2/4/8 planes and all sixteen `vro_cpyfm` logic ops at 16
//! planes are checked word for word against what the TT actually produced.
//!
//! **Why the pixels are comparable at all.** Only because the case table was
//! built for it (`tosdump/cases/vdi-raster.tsv`, sections A–D). Standard form
//! is machine-independent by definition, so `vr_trnfm` compares at any depth
//! ≤ 8; and at 16 planes EmuTOS' own `vro_cpyfm16` is the identical chunky
//! `UWORD` loop NewTOS runs (`vdi_raster.c:975-1221`). Device form *below*
//! nine planes is not comparable — the TT interleaves planes, NewTOS stores
//! one RGB565 word per pixel — so no case in the table asks for it.
//!
//! **The screen as an operand (2026-09-21).** `tosdump/cases/vdi-raster.tsv`
//! never names the screen, so for it the backend's geometry is not
//! load-bearing. `tosdump/cases/vdi-raster-screen.tsv` does — its reference
//! is EmuTOS 1a7484ab on a Falcon030 in VIDEL truecolor (Hatari, 640×480, 16
//! planes, `tosdump/reference/EMUTOS-1a7484ab-RASTER-screen-falcontc.BIN`), the
//! device model the `vdi_reference` backend double reproduces: 640×480, 16
//! planes, the Falcon 16-bit word with `OVERLAY_BIT` round-tripping through
//! the store. A form with `buffer == 2` is the screen (an MFDB whose
//! `fd_addr` is NULL and whose other fields are 0); `preludes` are issued
//! after the reset, before the measured call and never compared (they put a
//! known picture on the screen); `read` is a screen rectangle read back with
//! `v_get_pixel` after the call into buffer B at `row * 32 + col`, exactly as
//! `tosdump/core/runner_rast.c` does it, so the record compares it like a
//! memory destination. This file reuses the `vdi_reference` harness's
//! backend, host, workstation and reset verbatim rather than keeping a
//! second copy of them that could drift.
//!
//! What a screen case proves on the host, and what it cannot: `vro_cpyfm`
//! screen↔memory and `vrt_cpyfm` into memory run in `tos_api::vdi::raster`
//! and are exercised for real; screen→screen and mono→screen end in the
//! backend's `copy_rect` / `blit_mono`, which here is the double's (op and
//! mode honoured, `vdi_reference/harness.rs`), not the kernel's
//! (`kernel/src/vdi_backend.rs`). For those two arms the QEMU measurement
//! against the same reference is the evidence
//! (`measurements/measurement-2026-09-21-raster-screen/`).
//!
//! **Serialisation.** The VDI keeps device-global state (handle table, `Vwk`
//! slots, palette), and cargo runs tests on parallel threads, so every case
//! takes the same [`VDI_LOCK`](vdi::VDI_LOCK) the `vdi_reference` harness uses
//! — it is the same VDI.

#![allow(dead_code)]

#[path = "../vdi_reference/harness.rs"]
pub mod vdi;

use vdi::{workstation, Pb as VdiPb, Workstation, ABI, POISON, VDI_LOCK};

// ── the raster parameter block ───────────────────────────────────────────

/// Fifteen, not twelve: an AArch64 caller packs the *second* MFDB pointer at
/// `CONTRL[11..14]` (`gemlib gem_vdiP.h:177,204`, `VDI_CNTRLMAX = 15`,
/// `mt_gem.h:3508`). A twelve-WORD `CONTRL` cannot hold the LP64 form at all.
pub const CONTRL_N: usize = 15;
pub const INTIN_N: usize = 128;
pub const PTSIN_N: usize = 256;
pub const INTOUT_N: usize = 128;
pub const PTSOUT_N: usize = 256;

/// `CONTRL[6]`, the workstation handle — see [`MACHINE_EXCLUDED`].
const CONTRL_HANDLE: usize = 6;
/// `CONTRL[7..14]`, the two packed MFDB pointers — see [`MACHINE_EXCLUDED`].
const CONTRL_PTR_FIRST: usize = 7;
const CONTRL_PTR_N: usize = 8;

/// The two pixel buffers, in WORDs (`TD_RAST_BUF_WORDS`, `tosdump.h`). Large
/// enough for a 32×16 form at 16 planes under either machine's reading of it.
pub const BUF_WORDS: usize = 512;

/// The eight WORD fields of an MFDB after `fd_addr`: `fd_w`, `fd_h`,
/// `fd_wdwidth`, `fd_stand`, `fd_nplanes`, `fd_r1..3`
/// (EmuTOS `include/gsxdefs.h:65-76`).
pub const MFDB_N: usize = 8;

/// The MFDB as the caller's own compiler lays it out — which **is** the ABI
/// the VDI reads, because the VDI reads the caller's struct. LP64: an 8-byte
/// `fd_addr` followed by eight WORDs at offsets 8..22, exactly what
/// `tos_api::vdi::raster::Mfdb::from_ptr` parses for [`ABI`]. This is a
/// parameter, not a file, so it is declared as a struct rather than written
/// out at explicit offsets (`tosdump/core/runner_rast.c`).
#[repr(C)]
#[derive(Default)]
struct Mfdb {
    addr: u64,
    w: i16,
    h: i16,
    wdwidth: i16,
    stand: i16,
    nplanes: i16,
    r1: i16,
    r2: i16,
    r3: i16,
}

impl Mfdb {
    /// The eight WORDs after `fd_addr`, read back **after** the call:
    /// `vr_trnfm` overwrites `fd_stand` (`vdi_raster.c:303-314`), so the MFDB
    /// is an output as well as an input, and `fd_r1..3` must come back zero.
    fn words(&self) -> [i16; MFDB_N] {
        [self.w, self.h, self.wdwidth, self.stand, self.nplanes, self.r1, self.r2, self.r3]
    }
}

/// The two pixel buffers, laid out as one struct with A first — the shape
/// `tosdump/core/runner_rast.c`'s `struct td_raster_arrays` has. NewTOS'
/// `mem_to_mem` decides the copy direction from `src.addr == dst.addr` alone
/// (`tos-api/src/vdi/raster.rs`), so nothing here depends on the *order* of
/// the two; keeping it anyway costs nothing and removes a variable from a
/// comparison whose whole subject is memory.
#[repr(C)]
struct Buffers {
    a: [i16; BUF_WORDS],
    b: [i16; BUF_WORDS],
}

/// The raster parameter block: the five VDI arrays at raster width, the two
/// MFDBs and the two buffers they address.
struct Rpb {
    contrl: Box<[i16; CONTRL_N]>,
    intin: Box<[i16; INTIN_N]>,
    ptsin: Box<[i16; PTSIN_N]>,
    intout: Box<[i16; INTOUT_N]>,
    ptsout: Box<[i16; PTSOUT_N]>,
    bufs: Box<Buffers>,
    src: Box<Mfdb>,
    dst: Box<Mfdb>,
    block: Box<[u64; 5]>,
}

impl Rpb {
    fn new() -> Rpb {
        let mut pb = Rpb {
            contrl: Box::new([0; CONTRL_N]),
            intin: Box::new([0; INTIN_N]),
            ptsin: Box::new([0; PTSIN_N]),
            intout: Box::new([0; INTOUT_N]),
            ptsout: Box::new([0; PTSOUT_N]),
            bufs: Box::new(Buffers { a: [0; BUF_WORDS], b: [0; BUF_WORDS] }),
            src: Box::new(Mfdb::default()),
            dst: Box::new(Mfdb::default()),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    /// Poison every parameter array — but **not** the two pixel buffers.
    /// Those are the operand of the call, and a copy out of an all-`0xDEAD`
    /// source would prove nothing about where the bits went; they are laid
    /// with [`fill_word`] instead (`tosdump/core/record.h`, `td_poison_raster`).
    fn poison(&mut self) {
        self.contrl.fill(POISON);
        self.intin.fill(POISON);
        self.ptsin.fill(POISON);
        self.intout.fill(POISON);
        self.ptsout.fill(POISON);
    }

    /// Pack a pointer into `CONTRL` the way every GEM binding does: pointer
    /// `n` at `contrl[7 + n * N_PTRINTS]`, `N_PTRINTS = sizeof(void*) /
    /// sizeof(WORD)` = 4 under LP64 (`gemlib gem_vdiP.h:177,204`). The slot is
    /// only WORD-aligned, hence the unaligned store — which is also how
    /// `Vdipb::contrl_ptr` reads it back.
    fn set_mfdb_ptr(&mut self, n: usize, p: u64) {
        let slot = self.contrl.as_mut_ptr();
        // SAFETY: `7 + n*4 + 4 <= 15` for n ∈ {0,1}; the array is 15 WORDs.
        unsafe { (slot.add(CONTRL_PTR_FIRST + n * 4) as *mut u64).write_unaligned(p) };
    }

    /// Issue the call. The return value is the trap's `d0` (`flip_y`,
    /// `vdi/vdi_asm.S:124-125`), which the record carries at [`OFF_RET`] + 2.
    fn call(&mut self) -> i16 {
        // SAFETY: `block` holds five live pointers to arrays owned by `self`,
        // each at least as long as a raster opcode requires.
        unsafe { tos_api::vdi::dispatch(self.block.as_ptr() as u64, ABI) }
    }
}

// ── the buffer fill ──────────────────────────────────────────────────────

/// The pattern both machines laid their buffers with before every case
/// (`tosdump/core/record.c`, `td_rast_fill_word`). A pure function of
/// `(seed, index)` in fixed-width arithmetic, so a 68030 and an AArch64 write
/// the identical words without the case table having to spell 512 of them
/// out; two different seeds give a source and a destination that no logic op
/// can confuse.
pub fn fill_word(seed: u16, index: u16) -> u16 {
    let mut x = (index as u32).wrapping_mul(1103515245).wrapping_add(12345);
    x ^= (seed as u32).wrapping_mul(2654435761);
    x ^= x >> 15;
    x = x.wrapping_mul(2246822519);
    x ^= x >> 13;
    (x & 0xFFFF) as u16
}

fn fill_buffer(buf: &mut [i16; BUF_WORDS], seed: u16) {
    for (i, w) in buf.iter_mut().enumerate() {
        *w = fill_word(seed, i as u16) as i16;
    }
}

// ── the reference record ─────────────────────────────────────────────────

pub const HDR_SIZE: usize = 32;
pub const REC_SIZE: usize = 3696;
const OFF_CASEID: usize = 0;
const OFF_OPCODE: usize = 2;
const OFF_RET: usize = 4;
const OFF_CONTRL: usize = 8;
const OFF_CONTRL_BEFORE: usize = 38;
const OFF_INTIN: usize = 68;
const OFF_PTSIN: usize = 324;
const OFF_INTOUT: usize = 836;
const OFF_PTSOUT: usize = 1092;
const OFF_SRC_MFDB: usize = 1604;
const OFF_DST_MFDB: usize = 1620;
const OFF_BUF_A: usize = 1636;
const OFF_BUF_B: usize = 2660;

fn be16(b: &[u8], off: usize) -> i16 {
    (((b[off] as u16) << 8) | b[off + 1] as u16) as i16
}

// ── the case ─────────────────────────────────────────────────────────────

/// One MFDB as the case table states it: a **buffer**, never an address.
/// That is what lets both forms name the *same* buffer, which is how the
/// in-place `vr_trnfm` cases are stated — and an address could not be equal
/// on two machines anyway.
pub struct Form {
    /// 0 = buffer A, 1 = buffer B, 2 = the screen (`fd_addr` NULL).
    pub buffer: i16,
    pub w: i16,
    pub h: i16,
    pub wdwidth: i16,
    pub stand: i16,
    pub nplanes: i16,
}

impl Form {
    /// Row stride in pixels of a NewTOS device-format memory MFDB
    /// (`fd_wdwidth` 16-pixel words), used by the [`DEVIATIONS`] substitute.
    fn stride_px(&self) -> usize {
        (self.wdwidth.max(0) as usize) * 16
    }
}

/// The screen form: `fd_addr` NULL, every other field 0 — what
/// `tosdump/core/runner_rast.c` hands over for `S`.
pub const SCREEN: i16 = 2;

/// A prelude of a screen case (`tosdump/core/cases.h`, THE SCREEN): one
/// call issued after the reset and before the measured call, never
/// compared. 109/121 carry two forms; 25 `vsf_color`, 114 `vr_recfl` and
/// 129 `vs_clip` carry none.
pub struct Prelude {
    pub opcode: i16,
    pub intin: &'static [i16],
    pub ptsin: &'static [i16],
    pub src: Option<Form>,
    pub dst: Option<Form>,
}

/// The readback rectangle of a screen case: read with `v_get_pixel` after
/// the call, `INTOUT[1]` into buffer B at `row * 32 + col`.
pub struct Read {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

/// Width of the readback layout in buffer B (`TD_RAST_READ_W`).
pub const READ_W: usize = 32;

/// One case, as `tosdump/host/mkrusttests` emits it. There is no `sub`:
/// none of the three raster opcodes is an escape.
pub struct Case {
    pub id: u16,
    pub rec: usize,
    pub opcode: i16,
    pub intin: &'static [i16],
    pub ptsin: &'static [i16],
    pub src: Form,
    pub dst: Form,
    pub seed_a: u16,
    pub seed_b: u16,
    pub preludes: &'static [Prelude],
    pub read: Option<Read>,
    pub comment: &'static str,
}

struct Diff {
    field: &'static str,
    index: usize,
    got: i16,
    want: i16,
    /// True when `want` is not the reference's number but a substitute — the
    /// message has to say so, or it claims the dump decided something it
    /// cannot.
    substitute: bool,
}

fn push_diff(out: &mut Vec<Diff>, field: &'static str, index: usize, got: i16, want: i16) {
    if got != want {
        out.push(Diff { field, index, got, want, substitute: false });
    }
}

fn push_subst(out: &mut Vec<Diff>, field: &'static str, index: usize, got: i16, want: i16) {
    if got != want {
        out.push(Diff { field, index, got, want, substitute: true });
    }
}

// ── fields the reference cannot decide, and what is checked instead ──────

/// A field excluded from the byte comparison because its value is a property
/// of the **machine** rather than of the raster op. Every entry is checked
/// instead by [`substitute`], which asserts the part of the claim that *is*
/// machine-independent — so the rule is tested even though the number cannot
/// be. Nothing is skipped silently.
///
/// * **`contrl[6]` and `contrl_before[6]` — the workstation handle.** The TT
///   ran `DUMPRAST` as the only GEM application and `v_opnvwk` gave it handle
///   **2** (`tosdump/reference/EMUTOS14-RASTER.LOG`); the same program on the
///   Pi 4 got **3**, because TeraDesk was already holding 2
///   (`NEWTOS-RASTER.LOG`). The number is a property of who else has a
///   workstation open, not of the VDI. What *is* contract — that the caller's
///   handle goes in and comes back out unchanged in `CONTRL[6]` — is checked
///   by [`substitute`] against the handle this harness's own `v_opnvwk`
///   returned, and the reference's handle is required to be a real one (> 0).
/// * **`contrl[7..14]` and `contrl_before[7..14]` — the two packed MFDB
///   pointers.** They hold the address of one of the dumper's own buffers at
///   the target's pointer width: the m68k packs both at `CONTRL[7..8]` and
///   `[9..10]` and leaves `[11..14]` poisoned, AArch64 packs them at
///   `[7..10]` and `[11..14]`. They cannot be equal across two machines by
///   construction, which is why the C comparator records them with
///   `compare = 0` as well (`tosdump/core/record.c`, `td_raster_fields`).
///   [`substitute`] checks the machine-independent claim instead: a raster op
///   may not modify `CONTRL`'s pointer slots, so all eight must come back
///   exactly as they went in.
pub const MACHINE_EXCLUDED: &[Excluded] = &[
    Excluded { field: "contrl", first: CONTRL_HANDLE, count: 1 },
    Excluded { field: "contrl_before", first: CONTRL_HANDLE, count: 1 },
    Excluded { field: "contrl", first: CONTRL_PTR_FIRST, count: CONTRL_PTR_N },
    Excluded { field: "contrl_before", first: CONTRL_PTR_FIRST, count: CONTRL_PTR_N },
];

/// A run of record elements excluded from the byte comparison, named by the
/// field name [`cmp_field`] uses. This list is the single source of truth:
/// [`cmp_field`] skips exactly what it holds, and [`substitute`] must cover
/// exactly the same ground.
pub struct Excluded {
    pub field: &'static str,
    pub first: usize,
    pub count: usize,
}

fn is_machine_excluded(field: &str, index: usize) -> bool {
    MACHINE_EXCLUDED
        .iter()
        .any(|e| e.field == field && (e.first..e.first + e.count).contains(&index))
}

/// One field of one case where **NewTOS knowingly deviates from EmuTOS and
/// the lead has not yet decided what the contract should be**. This is not a
/// skip: the field is compared against an exact expectation derived from the
/// deviation as `docs/api/vdi/_discrepancies.md` documents it, so a change of
/// behaviour still fails the test — it just fails against the documented
/// deviation instead of against the dump.
pub struct Deviation {
    /// Case id = the physical line number in `tosdump/cases/vdi-raster.tsv`.
    pub case: u16,
    pub field: &'static str,
    /// The discrepancy-log row that owns the open question.
    pub tracked_as: &'static str,
    pub why: &'static str,
}

/// The one such field in the raster group.
///
/// **Case 157 — `D-VDI-25`, `vro_cpyfm` and `PTSIN[6..7]`.** The copy *size*
/// always comes from the source rectangle (`dont_clip` sets `b_wd`/`b_ht`
/// from `s_x/ymax − s_x/ymin`, `vdi_raster.c:889-891`), but `dont_clip` also
/// copies `PTSIN[6]`/`PTSIN[7]` **literally** into `info->d_xmax`/`d_ymax`
/// (`:897-898`), and the overlap-safe *backwards* path starts from those
/// (`vro_cpyfm16`, `:995-999`). Which of the two directions is taken is
/// decided by comparing the two computed **start addresses** (`src < dst`,
/// `:993`).
///
/// Case 157 states a source rectangle of `(0,0)-(7,7)`, a destination anchor
/// of `(0,0)` and a deliberately oversized `PTSIN[6..7] = (31,15)`. Measured:
/// the TT takes the backwards path and lands the 8×8 block at rows 8..15 /
/// columns 24..31; NewTOS anchors at the destination upper-left and lands it
/// at rows 0..7 / columns 0..7. Every **well-formed** caller
/// (`d_xmax = d_xmin + b_wd − 1`) is unaffected — the two anchors coincide —
/// which is why the other 25 section-C cases match bit-exactly.
///
/// It is **not fixed here** because reproducing it faithfully needs EmuTOS'
/// *address* comparison between two forms, and for two distinct forms that
/// answer depends on where the caller's buffers happen to sit in memory; on
/// NewTOS it could only ever be matched by accident for one allocation. The
/// honest options — reproduce the anchoring and the address comparison,
/// reproduce the anchoring but pick the direction by coordinates, or document
/// the deviation — need a decision from the lead, which has not been taken.
/// Until then this entry pins NewTOS' *current* contract exactly:
/// [`expected_buf_b`] rebuilds buffer B from the case's own fill seed plus
/// the destination-upper-left anchor rule, and any change to where the block
/// lands fails the test.
pub const DEVIATIONS: &[Deviation] = &[Deviation {
    case: 157,
    field: "buf_b",
    tracked_as: "D-VDI-25",
    why: "EmuTOS anchors the backwards vro_cpyfm at PTSIN[6..7] and picks the \
          direction by comparing two form addresses (vdi_raster.c:897-898, \
          :993, :995-999); NewTOS anchors at the destination upper-left. \
          Open, undecided -- docs/api/vdi/_discrepancies.md D-VDI-25",
}];

fn deviation(case: &Case, field: &str) -> Option<&'static Deviation> {
    DEVIATIONS.iter().find(|d| d.case == case.id && d.field == field)
}

/// Buffer B as NewTOS' **destination-upper-left anchor rule** produces it:
/// the fill, with the source rectangle replace-copied to the destination
/// anchor. Stated from the case table's own numbers — the two forms'
/// geometry, the two seeds and `PTSIN` — and not by calling into the code
/// under test, so it is an independent statement of where the block lands.
///
/// The corner sort is the one the VDI itself applies before it copies
/// (`arb_corner`, `vdi_raster.c:1336-1337`; `rect_from_ptsin`,
/// `tos-api/src/vdi/raster.rs`): both rectangles are normalised to
/// `(min, max)`, the extent comes from the **source** and the anchor is the
/// destination's upper-left corner. Only `INTIN[0] == 3` (`S_ONLY`, replace)
/// is expressible here; the one deviating case uses it, and an entry in
/// [`DEVIATIONS`] for any other logic op would have to say so.
fn expected_buf_b(case: &Case) -> [i16; BUF_WORDS] {
    assert_eq!(
        case.intin.first().copied(),
        Some(3),
        "case {}: the D-VDI-25 substitute only states the S_ONLY (replace) rule",
        case.id
    );
    assert_eq!(case.src.buffer, 0, "case {}: substitute expects source = buffer A", case.id);
    assert_eq!(case.dst.buffer, 1, "case {}: substitute expects dest = buffer B", case.id);

    let mut out = [0i16; BUF_WORDS];
    for (i, w) in out.iter_mut().enumerate() {
        *w = fill_word(case.seed_b, i as u16) as i16;
    }

    let p = case.ptsin;
    let (sx, sx2) = (p[0].min(p[2]), p[0].max(p[2]));
    let (sy, sy2) = (p[1].min(p[3]), p[1].max(p[3]));
    let (dx, dy) = (p[4].min(p[6]), p[5].min(p[7]));
    let (w, h) = (sx2 - sx + 1, sy2 - sy + 1);
    let (ss, ds) = (case.src.stride_px(), case.dst.stride_px());

    for r in 0..h {
        for c in 0..w {
            let (scol, srow) = (sx + c, sy + r);
            let (dcol, drow) = (dx + c, dy + r);
            // Outside either form's fd_w × fd_h extent the pixel is not moved
            // (`Mfdb::get_mem`/`put_mem` bound every access).
            if scol >= case.src.w || srow >= case.src.h {
                continue;
            }
            if dcol >= case.dst.w || drow >= case.dst.h {
                continue;
            }
            let si = srow as usize * ss + scol as usize;
            let di = drow as usize * ds + dcol as usize;
            out[di] = fill_word(case.seed_a, si as u16) as i16;
        }
    }
    out
}

/// What is checked *in place of* every excluded field. Failures land in the
/// same diff list as the byte comparisons, so an excluded field is still a
/// way for a case to fail — it is just no longer the reference's number that
/// decides.
fn substitute(out: &mut Vec<Diff>, case: &Case, pb: &Rpb, before: &[i16], rec: &[u8], wk: &Workstation) {
    // The workstation handle: the reference's must be a real handle, and ours
    // must survive the call unchanged.
    let ref_handle = be16(rec, OFF_CONTRL + CONTRL_HANDLE * 2);
    push_subst(out, "reference handle is a real handle", 0, i16::from(ref_handle > 0), 1);
    push_subst(out, "contrl[6] == our v_opnvwk handle", CONTRL_HANDLE, pb.contrl[CONTRL_HANDLE], wk.handle);
    push_subst(
        out,
        "contrl_before[6] == our v_opnvwk handle",
        CONTRL_HANDLE,
        before[CONTRL_HANDLE],
        wk.handle,
    );

    // The packed MFDB pointers: a raster op may not modify them.
    for (i, (&got, &want)) in
        pb.contrl.iter().zip(before.iter()).enumerate().skip(CONTRL_PTR_FIRST)
    {
        push_subst(out, "contrl[7..14] unchanged by the call", i, got, want);
    }

    // The named deviation, checked against what NewTOS' documented anchoring
    // must produce rather than against the dump.
    if let Some(d) = deviation(case, "buf_b") {
        let want = expected_buf_b(case);
        for (i, &g) in pb.bufs.b.iter().enumerate() {
            push_subst(out, "buf_b (D-VDI-25 substitute)", i, g, want[i]);
        }
        // A deviation that stopped reproducing must be removed, not left
        // standing: if the reference now agrees with the expectation, the
        // exception is hiding a byte comparison that would pass.
        let agrees = (0..BUF_WORDS).all(|i| be16(rec, OFF_BUF_B + i * 2) == want[i]);
        assert!(
            !agrees,
            "case {}: {} no longer reproduces -- the reference agrees with NewTOS' \
             anchoring. Remove the DEVIATIONS entry and let the byte comparison stand.",
            case.id,
            d.tracked_as
        );
    }
}

fn cmp_field(
    out: &mut Vec<Diff>,
    field: &'static str,
    got: &[i16],
    rec: &[u8],
    off: usize,
    count: usize,
) {
    for (i, &g) in got.iter().enumerate().take(count) {
        if is_machine_excluded(field, i) {
            continue; // checked by `substitute` instead
        }
        push_diff(out, field, i, g, be16(rec, off + i * 2));
    }
}

/// The MFDB a form describes: over buffer A or B, or the screen (`fd_addr`
/// NULL, every other field 0 — `runner_rast.c`, `fill_mfdb`).
fn mfdb_of(f: &Form, a_addr: u64, b_addr: u64) -> Mfdb {
    if f.buffer == SCREEN {
        return Mfdb::default();
    }
    Mfdb {
        addr: if f.buffer == 1 { b_addr } else { a_addr },
        w: f.w,
        h: f.h,
        wdwidth: f.wdwidth,
        stand: f.stand,
        nplanes: f.nplanes,
        ..Mfdb::default()
    }
}

/// Issue one prelude through its own parameter block (`runner_rast.c`,
/// `run_prelude`): the whitelist of `tosdump/core/cases.h` is the loader's
/// guard, so nothing is re-checked here.
fn run_prelude(p: &Prelude, a_addr: u64, b_addr: u64, wk: &Workstation) {
    let mut pb = Rpb::new();
    pb.contrl.fill(0);
    pb.intin[..p.intin.len()].copy_from_slice(p.intin);
    pb.ptsin[..p.ptsin.len()].copy_from_slice(p.ptsin);
    pb.contrl[0] = p.opcode;
    pb.contrl[1] = (p.ptsin.len() / 2) as i16;
    pb.contrl[3] = p.intin.len() as i16;
    pb.contrl[5] = 0;
    pb.contrl[6] = wk.handle;
    if let (Some(s), Some(d)) = (&p.src, &p.dst) {
        *pb.src = mfdb_of(s, a_addr, b_addr);
        *pb.dst = mfdb_of(d, a_addr, b_addr);
        let (src_ptr, dst_ptr) = (&*pb.src as *const Mfdb as u64, &*pb.dst as *const Mfdb as u64);
        pb.set_mfdb_ptr(0, src_ptr);
        pb.set_mfdb_ptr(1, dst_ptr);
    }
    pb.call();
}

/// Replay one case and compare every array, both MFDBs and both pixel buffers
/// against the reference.
///
/// The input arrays are compared too: a raster routine may modify neither,
/// and the dump captured them *after* the call precisely so that a violation
/// shows up. So are the MFDBs — `vr_trnfm` is *required* to overwrite the
/// destination's `fd_stand`, and `fd_r1..3` are required to stay zero.
pub fn check(dump: &[u8], case: &Case) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();

    // The reset the dumper runs before every case, through its own twelve-WORD
    // parameter block — exactly as `tosdump/core/runner_rast.c` step 1 calls
    // `td_reset_vdi`, which owns arrays separate from the raster runner's.
    let mut vdi_pb = VdiPb::new();
    vdi::reset_vdi(&mut vdi_pb, wk);

    let mut pb = Rpb::new();

    // 2. lay the two pixel buffers. Not poison: they are the operand.
    fill_buffer(&mut pb.bufs.a, case.seed_a);
    fill_buffer(&mut pb.bufs.b, case.seed_b);

    // 3. the two MFDBs, over the buffers the case named (or the screen).
    let (a_addr, b_addr) = (pb.bufs.a.as_ptr() as u64, pb.bufs.b.as_ptr() as u64);
    *pb.src = mfdb_of(&case.src, a_addr, b_addr);
    *pb.dst = mfdb_of(&case.dst, a_addr, b_addr);

    // 3b. the case's preludes, after the reset and the buffer fill — a
    //     raster prelude may copy a laid buffer to the screen — through
    //     their own parameter block, exactly as `runner_rast.c` step 3b.
    for p in case.preludes {
        run_prelude(p, a_addr, b_addr, wk);
    }

    // 4. poison every parameter array; 5. lay the case values over it.
    pb.poison();
    pb.intin[..case.intin.len()].copy_from_slice(case.intin);
    pb.ptsin[..case.ptsin.len()].copy_from_slice(case.ptsin);

    // 6. contrl: only the input slots. contrl[2] and contrl[4] are outputs and
    //    stay poisoned, so a VDI that leaves them alone stays distinguishable
    //    from one that writes 0.
    pb.contrl[0] = case.opcode;
    pb.contrl[1] = (case.ptsin.len() / 2) as i16;
    pb.contrl[3] = case.intin.len() as i16;
    pb.contrl[5] = 0;
    pb.contrl[6] = wk.handle;
    let (src_ptr, dst_ptr) = (&*pb.src as *const Mfdb as u64, &*pb.dst as *const Mfdb as u64);
    pb.set_mfdb_ptr(0, src_ptr);
    pb.set_mfdb_ptr(1, dst_ptr);

    // 7. snapshot contrl as it stands going in.
    let before: Vec<i16> = pb.contrl.to_vec();

    let rec = &dump[HDR_SIZE + case.rec * REC_SIZE..][..REC_SIZE];
    assert_eq!(
        be16(rec, OFF_CASEID) as u16,
        case.id,
        "record {} is case {}, expected case {} -- dump and generated tests disagree",
        case.rec,
        be16(rec, OFF_CASEID),
        case.id
    );
    assert_eq!(
        be16(rec, OFF_OPCODE),
        case.opcode,
        "case {}: record opcode {} vs case opcode {}",
        case.id,
        be16(rec, OFF_OPCODE),
        case.opcode
    );

    // 8. the call under measurement.
    let flip_y = pb.call();

    // 8b. the readback: the screen rectangle into buffer B through
    //     `v_get_pixel`, one call per pixel, on a separate parameter block
    //     so the recorded arrays stay the measured call's.
    if let Some(r) = &case.read {
        let mut rd = Rpb::new();
        for row in 0..r.h {
            for col in 0..r.w {
                rd.poison();
                rd.contrl[0] = 105;
                rd.contrl[1] = 1;
                rd.contrl[3] = 0;
                rd.contrl[5] = 0;
                rd.contrl[6] = wk.handle;
                rd.ptsin[0] = r.x + col;
                rd.ptsin[1] = r.y + row;
                rd.call();
                pb.bufs.b[row as usize * READ_W + col as usize] = rd.intout[1];
            }
        }
    }

    let mut diffs = Vec::new();
    // The trap's `d0` = `flip_y` (`vdi/vdi_asm.S:124-125`), compared as the
    // WORD the contract defines rather than the whole LONG — the same rule
    // `tosdump/host` uses (`core/record.c`, field `d0` at `OFF_RET + 2`).
    cmp_field(&mut diffs, "d0", &[flip_y], rec, OFF_RET + 2, 1);
    cmp_field(&mut diffs, "contrl", &pb.contrl[..], rec, OFF_CONTRL, CONTRL_N);
    cmp_field(&mut diffs, "contrl_before", &before, rec, OFF_CONTRL_BEFORE, CONTRL_N);
    cmp_field(&mut diffs, "intin", &pb.intin[..], rec, OFF_INTIN, INTIN_N);
    cmp_field(&mut diffs, "ptsin", &pb.ptsin[..], rec, OFF_PTSIN, PTSIN_N);
    cmp_field(&mut diffs, "intout", &pb.intout[..], rec, OFF_INTOUT, INTOUT_N);
    cmp_field(&mut diffs, "ptsout", &pb.ptsout[..], rec, OFF_PTSOUT, PTSOUT_N);
    cmp_field(&mut diffs, "src_mfdb", &pb.src.words(), rec, OFF_SRC_MFDB, MFDB_N);
    cmp_field(&mut diffs, "dst_mfdb", &pb.dst.words(), rec, OFF_DST_MFDB, MFDB_N);
    cmp_field(&mut diffs, "buf_a", &pb.bufs.a[..], rec, OFF_BUF_A, BUF_WORDS);
    if deviation(case, "buf_b").is_none() {
        cmp_field(&mut diffs, "buf_b", &pb.bufs.b[..], rec, OFF_BUF_B, BUF_WORDS);
    }
    substitute(&mut diffs, case, &pb, &before, rec, wk);

    if !diffs.is_empty() {
        let mut msg = format!(
            "case {} (opcode {}) -- {}\n  {} field(s) differ from the EmuTOS 1.4 \
             reference:\n",
            case.id,
            case.opcode,
            case.comment,
            diffs.len()
        );
        for d in DEVIATIONS.iter().filter(|d| d.case == case.id) {
            msg += &format!(
                "  note: `{}` is compared against {}, an OPEN deviation, not against \
                 the dump:\n        {}\n",
                d.field, d.tracked_as, d.why
            );
        }
        for d in diffs.iter().take(24) {
            let source = if d.substitute { "expected" } else { "reference" };
            msg += &format!(
                "    {}[{}]  NewTOS = 0x{:04X} ({})   {} = 0x{:04X} ({})\n",
                d.field, d.index, d.got as u16, d.got, source, d.want as u16, d.want
            );
        }
        if diffs.len() > 24 {
            msg += &format!("    ... and {} more\n", diffs.len() - 24);
        }
        panic!("{}", msg);
    }
}

// ── not from the dump: input validation ──────────────────────────────────

/// `vr_trnfm` with a negative `fd_h` (or a zero/negative plane count): EmuTOS
/// computes the plane size as a signed `LONG` (`(LONG)fd_h * fd_wdwidth`,
/// `vdi_raster.c:299-302`), forces the destination's `fd_stand` and then
/// runs loops whose counts are not positive — nothing is copied. NewTOS used
/// to widen `fd_h` to `usize` first, an overflow panic in the dev profile
/// (ARCH03 3a observation, fixed in 3b).
#[test]
fn vr_trnfm_with_a_negative_height_copies_nothing_but_sets_fd_stand() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    for (h, nplanes) in [(-1i16, 4i16), (-300, 1), (16, 0), (16, -2)] {
        let mut pb = Rpb::new();
        fill_buffer(&mut pb.bufs.a, 0x1111);
        fill_buffer(&mut pb.bufs.b, 0x2222);
        let b_before = pb.bufs.b;
        let (a_addr, b_addr) = (pb.bufs.a.as_ptr() as u64, pb.bufs.b.as_ptr() as u64);
        *pb.src = Mfdb { addr: a_addr, w: 16, h, wdwidth: 1, stand: 1, nplanes, ..Mfdb::default() };
        *pb.dst = Mfdb { addr: b_addr, w: 16, h, wdwidth: 1, stand: 1, nplanes, ..Mfdb::default() };
        pb.poison();
        pb.contrl[0] = 110;
        pb.contrl[1] = 0;
        pb.contrl[3] = 0;
        pb.contrl[5] = 0;
        pb.contrl[6] = wk.handle;
        let (s, d) = (&*pb.src as *const Mfdb as u64, &*pb.dst as *const Mfdb as u64);
        pb.set_mfdb_ptr(0, s);
        pb.set_mfdb_ptr(1, d);
        pb.call();
        assert_eq!(pb.dst.stand, 0, "h={h} planes={nplanes}: fd_stand forced to device form");
        assert_eq!(pb.bufs.b, b_before, "h={h} planes={nplanes}: nothing copied");
    }
}

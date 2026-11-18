//! Opcode 10 contract, through the LP64 AES dispatcher.
//! EmuTOS gemsuper.c:92-98 / gemaplib.c:54-63; docs/api/aes/application.md.
//! This backend supplies metadata only: any rendering is a test failure.
#![allow(clippy::too_many_arguments, unused_variables)]

use std::sync::atomic::{AtomicU16, Ordering};
use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

struct Profile {
    width: AtomicU16,
    height: AtomicU16,
    planes: AtomicU16,
    colors: AtomicU16,
}
static PROFILE: Profile = Profile {
    width: AtomicU16::new(640),
    height: AtomicU16::new(480),
    planes: AtomicU16::new(4),
    colors: AtomicU16::new(16),
};
impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        self.width.load(Ordering::Relaxed)
    }
    fn height(&self) -> u16 {
        self.height.load(Ordering::Relaxed)
    }
    fn planes(&self) -> u16 {
        self.planes.load(Ordering::Relaxed)
    }
    fn num_colors(&self) -> u16 {
        self.colors.load(Ordering::Relaxed)
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {
        panic!("unexpected palette write")
    }
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        panic!("unexpected palette read")
    }
    fn set_active_palette(&self, p: &[u32; 256]) {
        panic!("unexpected palette switch")
    }
    fn pel_to_device16(&self, p: Pel) -> u16 {
        panic!("unexpected pixel conversion")
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        panic!("unexpected pixel conversion")
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        panic!("unexpected pixel conversion")
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        panic!("unexpected pixel read")
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, p: Pel, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn abline(
        &self,
        x: i16,
        y: i16,
        xx: i16,
        yy: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        panic!("unexpected drawing")
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, p: PatternFill, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        g: &[u8],
        w: i16,
        h: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
    ) {
        panic!("unexpected drawing")
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        panic!("unexpected pixel read")
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {
        panic!("unexpected drawing")
    }
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {
        panic!("unexpected flush")
    }
}
struct TestHost;
impl Host for TestHost {
    fn console_write_byte(&self, b: u8) {
        panic!("unexpected console output")
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&PROFILE)
    }
}
static HOST: TestHost = TestHost;
static HOST_REF: &dyn Host = &HOST;
const POISON: i16 = 0x5a3c;

struct Call {
    // Exactly 15 global words, surrounded by canaries at both ends.
    global: [i16; 17],
    input: [i16; 16],
    output: [i16; 7],
    addrin: [u64; 3],
    addrout: [u64; 1],
}
impl Call {
    fn new() -> Self {
        Self {
            global: [POISON; 17],
            input: [POISON; 16],
            output: [POISON; 7],
            addrin: [0; 3],
            addrout: [0; 1],
        }
    }
    fn invoke(&mut self, op: i16, ni: i16, no: i16, na: i16) -> i16 {
        let mut control = [op, ni, no, na, 0];
        let before = control;
        let raw = [
            control.as_mut_ptr() as u64,
            self.global[1..].as_mut_ptr() as u64,
            self.input.as_mut_ptr() as u64,
            self.output.as_mut_ptr() as u64,
            self.addrin.as_mut_ptr() as u64,
            self.addrout.as_mut_ptr() as u64,
        ];
        // SAFETY: all six arrays are live, LP64, and cover the declared lengths.
        assert!(unsafe { tos_api::aes::dispatch(raw.as_ptr() as u64, Abi::Lp64) }.is_none());
        assert_eq!(control, before, "AES control is input-only");
        self.output[0]
    }
}

// One test owns all AES statics; no parallel test can change the profile.
#[test]
fn appl_init_preserves_caller_memory_and_pending_messages_on_both_profiles() {
    tos_api::host::install_host(&HOST_REF);
    let mut identity = None;
    let mut private_address = None;
    for (width, height, planes, colors, expected_private) in [
        (640, 480, 4, 16, 0x1143u64),
        (1920, 1080, 16, 256, 0x1143u64),
    ] {
        PROFILE.width.store(width, Ordering::Relaxed);
        PROFILE.height.store(height, Ordering::Relaxed);
        PROFILE.planes.store(planes, Ordering::Relaxed);
        PROFILE.colors.store(colors, Ordering::Relaxed);
        // No INF was loaded: both profiles use the built-in 0x1143 root.
        // The INF-to-root chain has its own aes::init unit test.
        let mut call = Call::new();
        let pid = call.invoke(10, 0, 1, 0);
        assert!(pid >= 0);
        assert_eq!(
            *identity.get_or_insert(pid),
            pid,
            "repeated init returns the same id"
        );
        let g = &call.global[1..16];
        assert_eq!(&g[..3], &[0x0340, 1, pid]);
        // ap_private is the m68k LONG: four bytes at WORD 3, WORD-aligned.
        // The AESGLOBAL keeps the TOS widths on every ABI (see the ABI note
        // in tos-api/src/aes/appl.rs); byte-for-byte coverage of the whole
        // struct lives in tests/aes_global_layout.rs.
        let private = unsafe { g.as_ptr().add(3).cast::<u32>().read_unaligned() };
        assert_eq!(u64::from(private), expected_private);
        assert_eq!(g[10], planes as i16, "planes is not log2(num_colors)");
        let address = unsafe { g.as_ptr().add(11).cast::<u32>().read_unaligned() };
        assert_ne!(address, 0);
        assert_eq!(*private_address.get_or_insert(address), address);
        // ap_private is WORDs 3..4. The resource fields ap_ptree (5..6),
        // ap_rscmem (7..8) and ap_rsclen (9), and the reserved WORDs 13..14,
        // are NOT outputs of this call (gemsuper.c:92-98).
        for index in [5, 6, 7, 8, 9, 13, 14] {
            assert_eq!(g[index], POISON, "global[{index}]");
        }
        assert_eq!(call.global[0], POISON);
        assert_eq!(
            call.global[16], POISON,
            "write past the 15-WORD global array"
        );
        assert_eq!(call.input, [POISON; 16]);
        assert_eq!(call.output[1..], [POISON; 6]);
        assert_eq!(call.addrin, [0; 3]);
        assert_eq!(call.addrout, [0; 1]);

        // Queue a standard 16-byte message to ourselves. ap_init only ORs
        // AP_OPEN in EmuTOS; calling it again must not discard this message.
        let message: [i16; 8] = [0x4711, pid, 0, 1, 2, 3, 4, 5];
        call.input[..2].copy_from_slice(&[pid, 16]);
        call.addrin[0] = message.as_ptr() as u64;
        assert_eq!(call.invoke(12, 2, 1, 1), 1);
        assert_eq!(call.invoke(10, 0, 1, 0), pid);
        let mut received = [POISON; 8];
        call.addrin[0] = received.as_mut_ptr() as u64;
        assert_eq!(
            call.invoke(11, 2, 1, 1),
            1,
            "appl_init discarded a pending message"
        );
        assert_eq!(received, message);

        // OUT_LEN=0 still initialises global but must not copy RET_CODE.
        call.output.fill(POISON);
        call.global[1] = POISON;
        call.invoke(10, 0, 0, 0);
        assert_eq!(call.global[1], 0x0340);
        assert_eq!(call.output, [POISON; 7]);
    }
}

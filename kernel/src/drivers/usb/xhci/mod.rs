//! xHCI host controller (PHASE-16 D1). Enough of the xHCI init sequence to
//! bring the controller up and read root-hub port connect status. Built on
//! `rust-osdev/xhci` (register layer). See `docs/api/usb/xhci-bringup.md`.
//!
//! DMA structures are cacheable static BSS — correct in QEMU (no caches).
//! The real Pi-4 (D7) must make them Normal-NC / cache-coherent (ADR-014).

#![allow(dead_code)]

use core::num::NonZeroUsize;

use xhci::accessor::Mapper;
use xhci::context::{self, InputHandler};
use xhci::ring::trb::{command, event, transfer};
use xhci::Registers;

use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::aarch64::mmu::va_to_pa;
use crate::drivers::mmio_va;
use crate::kprintln;

mod ring;

/// PCIe DMA bus offset. The controller reaches system RAM through the host
/// bridge's inbound window, whose PCIe-bus base is **not** 0 on the real Pi-4
/// (the `dma-ranges` `pcie_addr`, e.g. `0x4_0000_0000`). So a controller-
/// visible DMA address is the CPU physical address **plus** this offset; 0 on
/// QEMU (identity). Set by the board's controller discovery before `open`.
static DMA_BUS_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Set the PCIe DMA bus offset (the inbound-window `pcie_addr`).
pub fn set_dma_bus_offset(off: u64) {
    DMA_BUS_OFFSET.store(off, Ordering::Relaxed);
}

/// CPU virtual address of a DMA structure → the address the controller must
/// issue to reach it: physical address + the PCIe bus offset.
#[inline]
fn dma_addr(va: u64) -> u64 {
    va_to_pa(va) + DMA_BUS_OFFSET.load(Ordering::Relaxed)
}

/// Maps a peripheral physical address to its high-half VA — the same
/// `drivers::mmio_va` mapping all NewTOS MMIO uses (ADR-014).
#[derive(Clone)]
struct HighHalf;
impl Mapper for HighHalf {
    unsafe fn map(&mut self, phys: usize, _bytes: usize) -> NonZeroUsize {
        // SAFETY: the xHCI MMIO is mapped high-half by Stage-2.
        NonZeroUsize::new(mmio_va(phys)).expect("non-zero MMIO VA")
    }
    fn unmap(&mut self, _virt: usize, _bytes: usize) {}
}

const CMD_RING_LEN: usize = 16;
const EVT_RING_LEN: usize = 16;
const MAX_SLOTS: usize = 16;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Trb([u32; 4]);

// The command ring, event ring, DCBAA and ERST each need **64-byte**
// alignment (xHCI spec). Wrap each so its offset inside the page-aligned
// block is 64-byte aligned (the compiler pads to the field's alignment).
#[repr(C, align(64))]
struct Dcbaa([u64; MAX_SLOTS + 1]);
#[repr(C, align(64))]
struct Ring([Trb; CMD_RING_LEN]);
#[repr(C, align(64))]
struct Erst([u64; 2]);

/// All the controller-level DMA structures, one page-aligned block.
#[repr(C, align(4096))]
struct XhciDma {
    /// Device Context Base Address Array (index 0 = scratchpad array ptr).
    dcbaa: Dcbaa,
    cmd_ring: Ring,
    evt_ring: Ring,
    /// One Event Ring Segment Table entry: base (u64) + (size | rsvd<<32).
    erst: Erst,
}

#[link_section = ".dma"]
static mut DMA: XhciDma = XhciDma {
    dcbaa: Dcbaa([0; MAX_SLOTS + 1]),
    cmd_ring: Ring([Trb([0; 4]); CMD_RING_LEN]),
    evt_ring: Ring([Trb([0; 4]); EVT_RING_LEN]),
    erst: Erst([0; 2]),
};

/// How many HID devices we support concurrently (keyboard + mouse).
// Max device slots we track. Counts *every* enumerated device, not just HID:
// on the real Pi-4 the keyboard/mouse sit behind the VL805 internal USB2 hub
// (and possibly a second hub in the keyboard), so a slot is spent on each hub
// in the path plus each HID endpoint.
pub const MAX_HID: usize = 8;

// Per-device DMA. The Input and Device contexts are 32-byte contexts (CSZ=0;
// checked in `open`): the Input is control + slot + 31 endpoint contexts
// (33 × 8 u32), the Device is slot + 31 endpoints (32 × 8 u32). Laid out as
// `[u32]` so we can view the region as `context::Input32Byte` (a `#[repr(C)]`
// struct of the same u32s). One per device — the controller reads each
// device's contexts continuously, so they cannot be shared.
#[repr(C, align(64))]
struct DevDma {
    // The Device Context MUST be 64-byte aligned: xHCI §6.2.1, and the DCBAA
    // entry that points at it has bits 5:0 RsvdZ (a 64-byte-aligned base). It
    // is placed FIRST so — with `DevDma` itself `align(64)` — it always lands
    // on a 64-byte boundary regardless of the sizes of the other fields.
    // (Previously `device` followed `input` at offset 0x420 = 32-byte aligned;
    // the VL805 then wrote it to the masked 64-byte base 0x…400, so we read it
    // 0x20 too high and saw the EP0 context in the slot region. See
    // docs/STATUS.md "Blocked: VL805 hub configure".)
    device: [u32; 32 * 8],
    // The Input Context needs 16-byte alignment (§6.2.5 / the command TRB's
    // Input Context Pointer). At offset 0x400 after `device` that holds.
    input: [u32; 33 * 8],
    ep0_ring: Ring,
    int_ring: Ring,
    buf: [u8; 256],
    /// The interrupt-IN report buffer. 64 bytes = the largest TRB Transfer
    /// Length `queue_interrupt` asks for (`INT_TRB_MAX`).
    intbuf: [u8; INT_TRB_MAX],
}

// Enforce the alignment contract at compile time so a future field reorder or
// size change cannot silently re-break the Device / Input context alignment.
const _: () = assert!(
    core::mem::offset_of!(DevDma, device) % 64 == 0,
    "xHCI Device Context must be 64-byte aligned (§6.2.1 / DCBAA RsvdZ[5:0])"
);
const _: () = assert!(
    core::mem::offset_of!(DevDma, input) % 16 == 0,
    "xHCI Input Context must be 16-byte aligned (§6.2.5)"
);

const DEV_INIT: DevDma = DevDma {
    device: [0; 32 * 8],
    input: [0; 33 * 8],
    ep0_ring: Ring([Trb([0; 4]); CMD_RING_LEN]),
    int_ring: Ring([Trb([0; 4]); CMD_RING_LEN]),
    buf: [0; 256],
    intbuf: [0; INT_TRB_MAX],
};

#[link_section = ".dma"]
static mut DEV: [DevDma; MAX_HID] = [DEV_INIT; MAX_HID];

/// Smallest and largest TRB Transfer Length of an interrupt-IN transfer
/// (`docs/api/usb/usb-snapshot.md` §3.3): the endpoint's max packet size,
/// clamped to 8…64. A packet larger than what is left of the TD is Babble and
/// halts the endpoint (xHCI 1.2 §4.10.2.4); a TRB of at least one max packet
/// can never be overrun by one packet. 8 keeps the old length for every
/// endpoint whose max packet is ≤ 8 (QEMU's mouse: 4, the boot keyboard: 8).
const INT_TRB_MIN: u16 = 8;
const INT_TRB_MAX: usize = 64;

/// The enumeration buffer: the configuration descriptor (up to
/// `snapshot::CONFIG_MAX`) and the HID report descriptors (up to
/// `snapshot::REPORT_DESC_MAX`) are read here, then copied into the snapshot.
/// One page, page-aligned, so a data stage never crosses a 64-KiB boundary
/// (xHCI 1.2 §6.4.1 note). Shared by all devices: enumeration is sequential.
#[repr(C, align(4096))]
struct EnumBuf([u8; 4096]);
#[link_section = ".dma"]
static mut ENUM_BUF: EnumBuf = EnumBuf([0; 4096]);
/// The enumeration buffer as the last transfer left it (read side). Single
/// CPU, enumeration is sequential; the reference must be dropped before the
/// next `control_in_enum`.
fn enum_buf() -> &'static [u8; 4096] {
    // SAFETY: single-CPU boot path; no `&mut` to ENUM_BUF is alive while a
    // caller reads (`control_in_enum` holds its own only during the transfer).
    unsafe { &(*core::ptr::addr_of!(ENUM_BUF)).0 }
}
const _: () = assert!(
    super::snapshot::CONFIG_MAX <= 4096 && super::snapshot::REPORT_DESC_MAX <= 4096,
    "a kept descriptor must fit the enumeration buffer"
);

use super::snapshot::{self, dev_flag, reason, role, Fetch};

/// `config_desc::SkipReason` → the snapshot's reason code (§2.4: 1–5 =
/// `SkipReason` + 1, the same reasons as the log lines).
fn skip_code(r: config_desc::SkipReason) -> u8 {
    use config_desc::SkipReason as S;
    match r {
        S::NotHid => reason::NOT_HID,
        S::NotBootSubclass => reason::NOT_BOOT_SUBCLASS,
        S::UnknownBootProtocol => reason::UNKNOWN_BOOT_PROTOCOL,
        S::NoInterruptIn => reason::NO_INTERRUPT_IN,
        S::OneInterfacePerDevice => reason::ONE_INTERFACE_PER_DEVICE,
    }
}

/// Run `f` on device `di`'s snapshot entry (if it has one).
fn snap(di: usize, f: impl FnOnce(&mut snapshot::Device)) {
    super::with_store(|s| {
        if let Some(d) = s.device_mut(di) {
            f(d);
        }
    });
}

/// Count one device that was not enumerated (an F8 event, §2.4 `enum_skipped`).
fn snap_skipped() {
    super::with_store(|s| s.enum_skipped = s.enum_skipped.saturating_add(1));
}

// Scratchpad buffers: many controllers (the real Pi-4 VL805 does; QEMU does
// not) require software to provide N page-sized scratch buffers, advertised in
// HCSPARAMS2. DCBAA[0] points at an array of their addresses. If we leave
// DCBAA[0] = 0 when the controller wants scratchpads, it DMA-faults (HSE) on
// its first command. Both the array and the buffers are page-aligned.
const MAX_SCRATCH: usize = 64;
const SCRATCH_PAGE: usize = 4096;
#[repr(C, align(4096))]
struct ScratchArray([u64; MAX_SCRATCH]);
#[repr(C, align(4096))]
struct ScratchBufs([[u8; SCRATCH_PAGE]; MAX_SCRATCH]);
#[link_section = ".dma"]
static mut SCRATCH_ARRAY: ScratchArray = ScratchArray([0; MAX_SCRATCH]);
#[link_section = ".dma"]
static mut SCRATCH_BUFS: ScratchBufs = ScratchBufs([[0; SCRATCH_PAGE]; MAX_SCRATCH]);

/// Per-device runtime state (slot, the interrupt endpoint, ring producers).
#[derive(Clone, Copy)]
struct HidDev {
    slot: u8,
    dci: u8,
    /// 1 = keyboard, 2 = mouse, 9 = hub (not polled), 0 = unused.
    protocol: u8,
    /// Root-hub port (1-based) the device's path starts at + descriptor IDs.
    port: u8,
    vid: u16,
    pid: u16,
    /// `bDeviceClass` from the device descriptor (9 = hub).
    class: u8,
    /// xHCI route string: the downstream-port path through the hub tiers
    /// (0 for a device on a root port).
    route: u32,
    /// PORTSC speed id (1 full, 2 low, 3 high, 4 super).
    speed: u8,
    /// Transaction-Translator hub for a full/low-speed device behind a
    /// high-speed hub: the hub's slot id + the downstream port (0 = none).
    tt_hub_slot: u8,
    tt_port: u8,
    ep0_enq: usize,
    ep0_cycle: bool,
    int_enq: usize,
    int_cycle: bool,
    /// Index of the hub this device hangs off, `0xFF` = a root port.
    parent: u8,
    /// Snapshot role and reason (`snapshot::role`, `snapshot::reason`).
    role: u8,
    reason: u8,
    /// TRB Transfer Length of the interrupt-IN transfer (8…64, §3.3).
    trb_len: u16,
}

impl HidDev {
    const fn new() -> Self {
        Self {
            slot: 0,
            dci: 0,
            protocol: 0,
            port: 0,
            vid: 0,
            pid: 0,
            class: 0,
            route: 0,
            speed: 0,
            tt_hub_slot: 0,
            tt_port: 0,
            ep0_enq: 0,
            ep0_cycle: true,
            int_enq: 0,
            int_cycle: true,
            parent: 0xFF,
            role: role::NONE,
            reason: reason::USED,
            trb_len: INT_TRB_MIN,
        }
    }
}

/// A discovered USB device, for display (the init-screen USB tree).
#[derive(Clone, Copy, Default)]
pub struct DeviceInfo {
    pub port: u8,
    /// 1 = keyboard, 2 = mouse.
    pub protocol: u8,
    pub vid: u16,
    pub pid: u16,
}

pub use super::config_desc::HidInfo;
use super::config_desc;

/// The fields of a USB hub class descriptor (`0x29`, USB 2.0 §11.23.2.1) the
/// hub driver acts on.
#[derive(Clone, Copy)]
struct HubDescriptor {
    /// `bNbrPorts`: number of downstream facing ports.
    nbr_ports: u8,
    /// `bPwrOn2PwrGood`: power-on to power-good time, in **2 ms units**.
    pwr_on_2_pwr_good: u8,
}

impl HubDescriptor {
    /// Power-on to power-good in milliseconds. The descriptor field counts 2 ms
    /// units (USB 2.0 §11.23.2.1); FreeMiNT applies the same `* 2`
    /// (`sys/usb/src.km/hub.c:428` and `hub.c:463`).
    fn power_on_to_power_good_ms(&self) -> u32 {
        self.pwr_on_2_pwr_good as u32 * 2
    }
}

/// Default EP0 max packet size for a port speed (PORTSC speed id).
fn ep0_max_packet(speed: u8) -> u16 {
    match speed {
        4 => 512, // SuperSpeed
        3 => 64,  // High
        _ => 8,   // Full / Low
    }
}

/// Push a TRB onto a producer ring — the Command Ring and every Transfer
/// Ring: stamp the producer cycle bit, and on reaching the last slot write a
/// Link TRB back to the ring base (Toggle Cycle) and flip the producer cycle
/// state, so the ring is circular (xHCI 1.2 §4.9.2, §4.9.3, §4.11.5.1; the
/// index/cycle logic and its tests are in [`ring`]). Returns the index the TRB
/// was written to.
fn ring_push(ring: &mut [Trb], enq: &mut usize, cycle: &mut bool, data: [u32; 4]) -> usize {
    let len = ring.len();
    let base = dma_addr(ring.as_ptr() as u64);
    ring::push(enq, cycle, len, base, data, |i, t| ring[i] = Trb(t))
}

// ── DMA ordering barriers (real Pi-4: PCIe DMA is not cache-coherent) ─────
//
// The `.dma` block is mapped **Device-nGnRnE** (see `usb::map_dma_coherent`),
// so the controller and the CPU already share one un-cached, strongly-ordered
// view of RAM — no cache clean/invalidate is needed (and `dc cvac`/`dc ivac`
// on Device memory would be meaningless). What remains is *ordering*: a full
// barrier so our DMA-struct writes complete before we ring a doorbell, and so
// we observe the controller's write-backs before we read them. `dsb sy` also
// orders against any Normal memory the arguments alias. QEMU is coherent, so
// these compile out there.
#[inline(always)]
fn dcache_clean(_va: u64, _len: usize) {
    #[cfg(feature = "board-rpi4")]
    // SAFETY: a plain ordering barrier; no memory is dereferenced.
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
}
#[inline(always)]
fn dcache_inval(_va: u64, _len: usize) {
    #[cfg(feature = "board-rpi4")]
    // SAFETY: a plain ordering barrier; no memory is dereferenced.
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
}

// The `[u32; 33*8]` DMA buffer we cast to `Input32Byte` must be exactly its
// size, or the crate's slot/endpoint setters would land at the wrong stride
// (e.g. a 64-byte layout writing Slot@[16]/EP0@[32] instead of [8]/[16]).
// Verified at compile time: Input32Byte = control(32B) + slot(32B) +
// 31×endpoint(32B) = 1056 B = 33*8 u32. If this ever fails to build, the crate
// changed its 32-byte context stride and every `input`/`device` offset is off.
const _: () = assert!(core::mem::size_of::<context::Input32Byte>() == 33 * 8 * 4);

/// How many Transfer Events can be parked while the endpoint that owns them
/// has not collected them yet (see [`Xhci::defer_transfer_event`]).
///
/// The bound is structural, not a guess: every device slot has at most **one**
/// transfer outstanding at a time — a control transfer is issued and waited
/// for synchronously, and an interrupt endpoint carries a single armed TRB that
/// only the delivery of its own completion re-arms. So at most `MAX_HID + 1`
/// acknowledgements can be in flight. Sized to the event ring, comfortably
/// above that.
const PENDING_EVENTS: usize = EVT_RING_LEN;

pub struct Xhci {
    regs: Registers<HighHalf>,
    ports: u8,
    /// Command-ring producer index + cycle state.
    cmd_enq: usize,
    cmd_cycle: bool,
    /// Command TRBs issued since `open` (the enumeration summary and the
    /// `cmd timeout` line report it).
    cmd_count: u32,
    /// Event-ring consumer index + cycle state.
    evt_deq: usize,
    evt_cycle: bool,
    /// MMIO VA of the ERDP register's high 32 bits, re-written explicitly each
    /// event because the BCM2711 PCIe mangles the crate's 64-bit ERDP store.
    erdp_hi: u64,
    /// Per-device state (EP0/interrupt ring producers, slot, endpoint).
    hid: [HidDev; MAX_HID],
    /// Number of configured HID devices.
    n_hid: usize,
    /// Transfer Events consumed while waiting for a *different* transfer's
    /// acknowledgement, held for the endpoint that owns them. FIFO in
    /// `pending[..n_pending]`.
    pending: [event::TransferEvent; PENDING_EVENTS],
    n_pending: usize,
}

impl Xhci {
    /// Bring the controller at `mmio_base` (physical) up: reset → max slots
    /// → DCBAA → command ring → event ring → run.
    ///
    /// # Safety
    ///
    /// `mmio_base` must be the controller's MMIO base; single-CPU boot;
    /// `DMA` is exclusively ours.
    pub unsafe fn open(mmio_base: u64) -> Self {
        // SAFETY: caller contract — valid MMIO base, mapped high-half.
        let mut regs = unsafe { Registers::new(mmio_base as usize, HighHalf) };

        // Wait for ready, then reset.
        while regs.operational.usbsts.read_volatile().controller_not_ready() {
            core::hint::spin_loop();
        }
        regs.operational.usbcmd.update_volatile(|c| {
            c.set_host_controller_reset();
        });
        while regs.operational.usbcmd.read_volatile().host_controller_reset() {
            core::hint::spin_loop();
        }
        while regs.operational.usbsts.read_volatile().controller_not_ready() {
            core::hint::spin_loop();
        }

        let hcs1 = regs.capability.hcsparams1.read_volatile();
        let max_slots = hcs1.number_of_device_slots();
        let max_ports = hcs1.number_of_ports();
        let slots = max_slots.min(MAX_SLOTS as u8);
        regs.operational.config.update_volatile(|c| {
            c.set_max_device_slots_enabled(slots);
        });

        // SAFETY: single-CPU boot; DMA is BSS we own exclusively.
        let dma = unsafe { &mut *(&raw mut DMA) };

        let dcbaa_pa = dma_addr((&raw const dma.dcbaa) as u64);
        regs.operational.dcbaap.update_volatile(|d| d.set(dcbaa_pa));

        let cmd_pa = dma_addr((&raw const dma.cmd_ring) as u64);
        regs.operational.crcr.update_volatile(|c| {
            c.set_command_ring_pointer(cmd_pa);
            // Ring Cycle State = 1: the controller's consumer cycle state must
            // match our producer cycle (our first command TRB has cycle = 1).
            // The crate's pointer setter leaves RCS at 0; QEMU tolerates the
            // mismatch, the real VL805 then never processes the command.
            c.set_ring_cycle_state();
        });

        // Event ring: ERST[0] = { segment base, segment size }.
        let evt_pa = dma_addr((&raw const dma.evt_ring) as u64);
        dma.erst.0[0] = evt_pa;
        dma.erst.0[1] = EVT_RING_LEN as u64;
        let erst_pa = dma_addr((&raw const dma.erst) as u64);
        {
            let mut intr = regs.interrupter_register_set.interrupter_mut(0);
            intr.erstsz.update_volatile(|s| s.set(1));
            intr.erdp.update_volatile(|d| d.set_event_ring_dequeue_pointer(evt_pa));
            intr.erstba.update_volatile(|b| b.set(erst_pa));
        }

        // The BCM2711 PCIe MMIO mishandles a single 64-bit store to the 64-bit
        // xHCI pointer registers: the high 32 bits come back as the low value
        // (verified by readback). So after the crate's 64-bit writes set the
        // (correct) low halves, re-write each register's HIGH 32 bits with an
        // explicit 32-bit store. Harmless on QEMU (addresses < 4 GiB → HI = 0).
        // Offsets (xHCI spec): operational = cap + CAPLENGTH; runtime = cap +
        // RTSOFF; interrupter 0 = runtime + 0x20. CRCR 0x18, DCBAAP 0x30 in the
        // operational set; ERSTBA +0x10, ERDP +0x18 in the interrupter set.
        let erdp_hi = unsafe {
            let cap = mmio_va(mmio_base as usize) as u64;
            let caplength = core::ptr::read_volatile(cap as *const u8) as u64;
            let op = cap + caplength;
            let rtsoff = (core::ptr::read_volatile((cap + 0x18) as *const u32) & !0x1f) as u64;
            let intr0 = cap + rtsoff + 0x20;
            let wr_hi = |reg_lo: u64, addr: u64| {
                core::ptr::write_volatile((reg_lo + 4) as *mut u32, (addr >> 32) as u32);
            };
            wr_hi(op + 0x30, dcbaa_pa); // DCBAAP
            wr_hi(op + 0x18, cmd_pa); // CRCR
            wr_hi(intr0 + 0x10, erst_pa); // ERSTBA
            wr_hi(intr0 + 0x18, evt_pa); // ERDP
            intr0 + 0x1c // ERDP-high register VA — poll_event re-writes it per event
        };

        // Scratchpad buffers (HCSPARAMS2): the controller may require N page
        // buffers for its own use, reached through DCBAA[0] → a scratchpad
        // array of their addresses. The real Pi-4 VL805 needs them (else it
        // HSE-DMA-faults on the first command); QEMU reports 0.
        let n_scratch =
            regs.capability.hcsparams2.read_volatile().max_scratchpad_buffers() as usize;
        if n_scratch > 0 {
            let n = n_scratch.min(MAX_SCRATCH);
            // SAFETY: single-CPU boot; the scratch statics are ours.
            let arr = unsafe { &mut *(&raw mut SCRATCH_ARRAY) };
            for i in 0..n {
                let buf = unsafe { (&raw const (*(&raw const SCRATCH_BUFS)).0[i]) as u64 };
                arr.0[i] = dma_addr(buf);
            }
            dma.dcbaa.0[0] = dma_addr((&raw const SCRATCH_ARRAY) as u64);
            dcache_clean(
                core::ptr::addr_of!(SCRATCH_ARRAY) as u64,
                core::mem::size_of::<ScratchArray>(),
            );
        }
        kprintln!("[usb] scratchpad buffers needed: {}", n_scratch);
        kprintln!(
            "[usb] dma addrs: dcbaap={:#x} crcr={:#x} erstba={:#x} dcbaa0={:#x}",
            dcbaa_pa,
            cmd_pa,
            erst_pa,
            dma.dcbaa.0[0]
        );
        // AC64 (64-bit addressing capability), CSZ (context size: 0 => 32-byte,
        // which is what our contexts are), and the DCBAAP readback. The latter
        // documents the BCM2711 64-bit-MMIO-read quirk (the high half mirrors
        // the low), which is why we write 64-bit controller registers as two
        // 32-bit halves.
        let cap1 = regs.capability.hccparams1.read_volatile();
        kprintln!(
            "[usb] reg readback: ac64={} csz={} dcbaap_hw={:#x}",
            cap1.addressing_capability() as u8,
            cap1.context_size() as u8,
            regs.operational.dcbaap.read_volatile().get(),
        );

        // Run.
        // Make the DCBAA, command ring and ERST visible to a non-coherent
        // controller (real Pi-4) before it starts; no-op on QEMU.
        dcache_clean(core::ptr::addr_of!(DMA) as u64, core::mem::size_of::<XhciDma>());
        dcache_clean(
            core::ptr::addr_of!(DEV) as u64,
            core::mem::size_of::<[DevDma; MAX_HID]>(),
        );
        regs.operational.usbcmd.update_volatile(|c| {
            c.set_run_stop();
        });
        while regs.operational.usbsts.read_volatile().hc_halted() {
            core::hint::spin_loop();
        }

        kprintln!("[usb] xHCI up: {} slots enabled, {} root ports", slots, max_ports);

        // Diagnostic build only: walk the xHCI Extended Capabilities and print
        // every Supported Protocol capability (id 2). This is the ONLY thing
        // that says which root-port numbers are USB 2.0 and which are USB 3.0
        // — a SuperSpeed-capable connector presents *two* port numbers, one per
        // protocol, and a low/full-speed HID always lands on the USB 2.0 one.
        // It also carries the Protocol Speed ID table: `port_speed()` returns a
        // PSI, and the default meaning (1 full, 2 low, 3 high, 4 super) that
        // `ep0_max_packet` assumes only holds while PSIC is 0.
        #[cfg(feature = "usb-diag")]
        // SAFETY: the capability list lies inside the controller's MMIO, which
        // is mapped high-half; reads are volatile and side-effect free.
        unsafe {
            // HCCPARAMS1 is at capability offset 0x10; its bits 31:16 are the
            // xECP pointer in 32-bit words from the MMIO base. Read raw rather
            // than through the crate so the walk does not depend on an
            // accessor name.
            let cap_va = mmio_va(mmio_base as usize) as *const u32;
            let xecp = (core::ptr::read_volatile(cap_va.add(4)) >> 16) as usize;
            kprintln!("[usb] xECP at dword {:#x} from base", xecp);
            let mut off = xecp;
            let mut guard = 0;
            while off != 0 && guard < 64 {
                guard += 1;
                let p = mmio_va(mmio_base as usize + off * 4) as *const u32;
                let dw0 = core::ptr::read_volatile(p);
                let id = dw0 & 0xff;
                let next = ((dw0 >> 8) & 0xff) as usize;
                if id == 2 {
                    let name = core::ptr::read_volatile(p.add(1));
                    let dw2 = core::ptr::read_volatile(p.add(2));
                    let port_off = (dw2 & 0xff) as u8;
                    let port_cnt = ((dw2 >> 8) & 0xff) as u8;
                    let psic = ((dw2 >> 28) & 0xf) as u8;
                    kprintln!(
                        "[usb] proto '{}{}{}{}' {}.{} ports {}..{} psic={}",
                        (name & 0xff) as u8 as char,
                        ((name >> 8) & 0xff) as u8 as char,
                        ((name >> 16) & 0xff) as u8 as char,
                        ((name >> 24) & 0xff) as u8 as char,
                        (dw0 >> 24) & 0xff,
                        (dw0 >> 16) & 0xff,
                        port_off,
                        port_off as u16 + port_cnt as u16 - 1,
                        psic
                    );
                    for k in 0..psic as usize {
                        let psi = core::ptr::read_volatile(p.add(4 + k));
                        kprintln!(
                            "[usb]   PSI {}: value={} exp={} mantissa={} full={:#010x}",
                            k,
                            psi & 0xf,
                            (psi >> 4) & 0x3,
                            psi >> 16,
                            psi
                        );
                    }
                } else {
                    kprintln!("[usb] xECP id {} (skipped)", id);
                }
                if next == 0 {
                    break;
                }
                off += next;
            }
        }

        // Power on every root port (PORTSC.PP, bit 9). The real Pi-4 VL805 does
        // not power its ports by default — a device gets no power (LEDs dark,
        // never reports a connect) until software sets PP; QEMU powers them
        // implicitly (HCCPARAMS.PPC tells us which, but setting PP is harmless
        // when ports are always-powered). Then wait for power to stabilise
        // (USB2 PWRON2PWRGOOD ≈ 20 ms) before the connect scan / enumeration.
        let ppc = regs.capability.hccparams1.read_volatile().port_power_control();
        for i in 0..max_ports as usize {
            regs.port_register_set.update_volatile_at(i, |p| {
                p.portsc.set_port_power();
            });
        }
        kprintln!("[usb] root ports powered (PPC={})", ppc);
        crate::arch::aarch64::delay_ms(20);

        Xhci {
            regs,
            ports: max_ports,
            cmd_enq: 0,
            cmd_cycle: true,
            cmd_count: 0,
            evt_deq: 0,
            evt_cycle: true,
            erdp_hi,
            hid: [HidDev::new(); MAX_HID],
            n_hid: 0,
            pending: [event::TransferEvent::new(); PENDING_EVENTS],
            n_pending: 0,
        }
    }

    /// Queue an interrupt-IN transfer (one report, up to the endpoint's max
    /// packet size clamped to 8…64 bytes) on the device's interrupt endpoint
    /// and ring its doorbell. The completion arrives later
    /// as a Transfer Event (drained by [`Self::poll_interrupt`]).
    pub fn queue_interrupt(&mut self, di: usize) {
        let slot = self.hid[di].slot;
        let dci = self.hid[di].dci;
        if dci == 0 {
            // No interrupt endpoint was ever configured for this device — a
            // hub, or a HID whose Configure Endpoint failed. Doorbell target 0
            // is the Command doorbell of slot 0 and is reserved on a device
            // slot (xHCI §5.6), so ringing it would be a malformed doorbell
            // write, not a queued transfer.
            kprintln!(
                "[usb] queue_interrupt: device {} (slot {}) has no interrupt endpoint",
                di,
                slot
            );
            return;
        }
        // SAFETY: single-CPU; DEV is ours.
        let buf_pa = unsafe { dma_addr((&raw const (*(&raw const DEV))[di].intbuf) as u64) };
        let mut n = transfer::Normal::new();
        n.set_data_buffer_pointer(buf_pa)
            .set_trb_transfer_length(self.hid[di].trb_len as u32)
            .set_interrupt_on_completion();
        let data = transfer::Allowed::Normal(n).into_raw();
        // SAFETY: single-CPU; DEV is ours.
        let ring = unsafe { &mut (*(&raw mut DEV))[di].int_ring.0 };
        ring_push(ring, &mut self.hid[di].int_enq, &mut self.hid[di].int_cycle, data);
        self.dma_clean();
        self.regs.doorbell.update_volatile_at(slot as usize, |db| {
            db.set_doorbell_target(dci);
        });
    }

    /// Number of configured HID devices, and a device's protocol.
    pub fn n_hid(&self) -> usize {
        self.n_hid
    }
    pub fn protocol(&self, di: usize) -> u8 {
        self.hid[di].protocol
    }

    /// Write back the whole DMA region (rings, DCBAA, contexts, buffers) so a
    /// non-coherent controller (real Pi-4) reads our latest writes. Called
    /// before every doorbell; a no-op on coherent QEMU.
    #[inline]
    fn dma_clean(&self) {
        dcache_clean(core::ptr::addr_of!(DMA) as u64, core::mem::size_of::<XhciDma>());
        dcache_clean(
            core::ptr::addr_of!(DEV) as u64,
            core::mem::size_of::<[DevDma; MAX_HID]>(),
        );
    }

    /// Deliver one completed HID interrupt transfer: the device index, a copy
    /// of its report buffer, **how many of those bytes the device actually
    /// sent**, and the completion code (1 Success, 13 Short Packet — xHCI 1.2
    /// Table 6-90). `None` when nothing is waiting.
    ///
    /// The length comes from the Transfer Event's *TRB Transfer Length* field,
    /// which for an IN endpoint holds the **residual** — the number of bytes
    /// of the TRB's length (8…64, `HidDev::trb_len`) that were *not*
    /// transferred (xHCI 1.2 §4.11.5.1 / Table 6-39, "the residual number of
    /// bytes not transferred"). With an 8-byte TRB a 3-byte
    /// boot mouse therefore reports `ShortPacket` with residual 5 → length 3,
    /// and a wheel mouse residual 4 → length 4. Without it the report buffer
    /// gives no way to tell a byte the device sent from one it never wrote
    /// (the buffer is not cleared between transfers), which is exactly the
    /// distinction the wheel byte needs.
    ///
    /// An event is handed to a device only when it names that device's slot
    /// **and** the interrupt-endpoint DCI the device was configured with.
    /// `dci == 0` means no interrupt endpoint was ever configured (a hub, or a
    /// device whose `configure_hid` failed) — such a device can never own a
    /// Transfer Event. Matching on the slot id alone attributed a control
    /// transfer's completion on a hub's EP0 to the hub, whose report buffer was
    /// then fed to the IKBD stream and whose (non-existent) endpoint was then
    /// "re-armed" with doorbell target 0.
    pub fn poll_interrupt(&mut self) -> Option<(usize, [u8; INT_TRB_MAX], usize, u8)> {
        // Bounded: `poll` runs from the cooperative input pump, so drain at
        // most the parked events plus one pass over the ring per call.
        for _ in 0..(PENDING_EVENTS + EVT_RING_LEN) {
            let te = match self.pop_pending() {
                Some(te) => te,
                None => match self.poll_event() {
                    Some(event::Allowed::TransferEvent(te)) => te,
                    // Port Status Change / late command completions have no
                    // waiter here; keep draining.
                    Some(_) => continue,
                    None => return None,
                },
            };
            let (slot, ep) = (te.slot_id(), te.endpoint_id());
            let owner = (0..self.n_hid).find(|&i| {
                self.hid[i].dci != 0 && self.hid[i].slot == slot && self.hid[i].dci == ep
            });
            let Some(di) = owner else {
                kprintln!(
                    "[usb] stray transfer event: slot {} ep {} code {:?}",
                    slot,
                    ep,
                    te.completion_code()
                );
                continue;
            };
            match te.completion_code() {
                // A report shorter than the TRB length we ask for completes
                // as ShortPacket — the normal case for a 3-byte mouse report,
                // not an error.
                Ok(code @ (event::CompletionCode::Success | event::CompletionCode::ShortPacket)) => {
                    let trb_len = self.hid[di].trb_len as usize;
                    // SAFETY: single-CPU; DEV is ours.
                    let p = unsafe { (&raw const (*(&raw const DEV))[di].intbuf) as u64 };
                    dcache_inval(p, trb_len); // the controller DMA'd the HID report
                    // `trb_len` = the TRB Transfer Length `queue_interrupt`
                    // asked for; the event reports what is left over. A
                    // controller that reports more residual than requested
                    // would underflow, so saturate.
                    let len = trb_len.saturating_sub(te.trb_transfer_length() as usize);
                    return Some((di, unsafe { (*(&raw const DEV))[di].intbuf }, len, code as u8));
                }
                code => {
                    // Counted for the snapshot (`int_errors`, `last_cc`, §3.3);
                    // no ring entry. A Halted endpoint is not reset here
                    // (usb-snapshot.md §8): the counter makes it visible.
                    let cc = match code {
                        Ok(c) => c as u8,
                        Err(raw) => raw,
                    };
                    super::with_store(|s| s.note_error(di, cc));
                    // The transfer is finished either way, so the endpoint has
                    // nothing armed any more — and only a *delivered* report
                    // takes `poll` back to `queue_interrupt`. Re-arm here, or
                    // the device falls silent after its first failed transfer.
                    kprintln!(
                        "[usb] interrupt transfer slot {} ep {} → {:?}; re-arming",
                        slot,
                        ep,
                        code
                    );
                    self.queue_interrupt(di);
                }
            }
        }
        None
    }

    /// Park a Transfer Event that acknowledges a transfer other than the one
    /// being waited for, so that its owner can still collect it.
    ///
    /// Why software, and not "leave it on the ring": the xHCI event ring is
    /// consumer-driven — reading a TRB means advancing the dequeue pointer
    /// (ERDP) past it, after which the controller may reuse the slot, so an
    /// event cannot be put back. Not consuming it is no better: the event we
    /// are waiting for sits *behind* it on the same ring, so leaving it in
    /// place would deadlock the wait. Dropping it is the worst of the three —
    /// a device has exactly one interrupt transfer armed at a time and only the
    /// delivery of its completion re-arms the next one (`poll` →
    /// `queue_interrupt`), so a discarded Transfer Event silences that device
    /// permanently. Hence: consume it, hold it here, hand it to its owner.
    fn defer_transfer_event(&mut self, te: event::TransferEvent) {
        if self.n_pending < PENDING_EVENTS {
            self.pending[self.n_pending] = te;
            self.n_pending += 1;
            return;
        }
        // Unreachable while the "at most one transfer outstanding per slot"
        // invariant holds (see `PENDING_EVENTS`). Say so loudly rather than
        // lose an acknowledgement in silence.
        kprintln!(
            "[usb] deferred-event queue full: lost slot {} ep {} — more transfers \
             outstanding than the driver tracks",
            te.slot_id(),
            te.endpoint_id()
        );
    }

    /// Take the parked Transfer Event that acknowledges `slot`/`ep`, if one is
    /// waiting.
    fn take_pending(&mut self, slot: u8, ep: u8) -> Option<event::TransferEvent> {
        let idx = (0..self.n_pending)
            .find(|&i| self.pending[i].slot_id() == slot && self.pending[i].endpoint_id() == ep)?;
        let te = self.pending[idx];
        for k in idx..self.n_pending - 1 {
            self.pending[k] = self.pending[k + 1];
        }
        self.n_pending -= 1;
        Some(te)
    }

    /// Take the oldest parked Transfer Event, whoever owns it.
    fn pop_pending(&mut self) -> Option<event::TransferEvent> {
        if self.n_pending == 0 {
            return None;
        }
        let te = self.pending[0];
        for k in 0..self.n_pending - 1 {
            self.pending[k] = self.pending[k + 1];
        }
        self.n_pending -= 1;
        Some(te)
    }

    /// Enqueue a command TRB, ring the command doorbell, and wait for its
    /// Command Completion event. Returns the completion (slot id + code).
    ///
    /// The Command Ring is a single 16-TRB segment closed by a Link TRB with
    /// Toggle Cycle, exactly like the transfer rings ([`ring_push`]; xHCI 1.2
    /// §4.9.3: command-ring management is "identical to Transfer Rings", the
    /// producer "toggling the Cycle bit each pass through the Command Ring").
    /// Before USB01 F1 the producer jumped from index 14 back to 0 with
    /// neither a Link TRB nor a cycle toggle; the controller stopped at the
    /// empty slot 15 and command #16 always timed out
    /// (`measurements/measurement-2026-10-23-usb01-f1f2f8/`).
    fn command(&mut self, data: [u32; 4]) -> Option<event::CommandCompletion> {
        // SAFETY: single-CPU; DMA is ours.
        let dma = unsafe { &mut *(&raw mut DMA) };
        self.cmd_count += 1;
        let cmd_index =
            ring_push(&mut dma.cmd_ring.0, &mut self.cmd_enq, &mut self.cmd_cycle, data);
        // Ring doorbell 0 (the command ring).
        self.dma_clean();
        self.regs.doorbell.update_volatile_at(0, |d| {
            d.set_doorbell_target(0);
        });
        for _ in 0..200_000_000u64 {
            // The event ring is shared by every slot, endpoint and the command
            // ring. Only a Command Completion answers us; a Transfer Event that
            // lands here acknowledges someone else's transfer and is parked for
            // its owner (never dropped — see `defer_transfer_event`). Port
            // Status Change events have no waiter and are consumed.
            match self.poll_event() {
                Some(event::Allowed::CommandCompletion(cc)) => return Some(cc),
                Some(event::Allowed::TransferEvent(te)) => self.defer_transfer_event(te),
                _ => {}
            }
            core::hint::spin_loop();
        }
        // Timed out — dump state to localise the DMA round-trip failure. evt0
        // tells us whether the controller wrote *any* completion event (DMA
        // write reached RAM) or nothing (it never read our command TRB).
        let sts = self.regs.operational.usbsts.read_volatile();
        // SAFETY: single-CPU; DMA is ours.
        let dma = unsafe { &*(&raw const DMA) };
        dcache_inval((&raw const dma.evt_ring.0[0]) as u64, 64);
        let e = dma.evt_ring.0[0].0;
        kprintln!(
            "[usb] cmd timeout: command #{} at ring index {}: hch={} hse={} hce={} cnr={} \
             evt0={:08x} {:08x} {:08x} {:08x}",
            self.cmd_count,
            cmd_index,
            sts.hc_halted(),
            sts.host_system_error(),
            sts.host_controller_error(),
            sts.controller_not_ready(),
            e[0],
            e[1],
            e[2],
            e[3]
        );
        None
    }

    /// Consume one event from the event ring if its cycle bit is current,
    /// advancing the dequeue pointer + ERDP. `None` if no new event.
    fn poll_event(&mut self) -> Option<event::Allowed> {
        // SAFETY: single-CPU; DMA is ours.
        let dma = unsafe { &mut *(&raw mut DMA) };
        // Drop the stale cached copy of this event TRB so we see the
        // controller's write (real Pi-4; no-op on QEMU). One cache line.
        dcache_inval((&raw const dma.evt_ring.0[self.evt_deq]) as u64, 16);
        let data = dma.evt_ring.0[self.evt_deq].0;
        if ((data[3] & 1) != 0) != self.evt_cycle {
            return None; // not yet written by the controller
        }
        let ev = event::Allowed::try_from(data).ok();
        self.evt_deq += 1;
        if self.evt_deq >= EVT_RING_LEN {
            self.evt_deq = 0;
            self.evt_cycle = !self.evt_cycle;
        }
        let erdp = dma_addr((&raw const dma.evt_ring.0[self.evt_deq]) as u64);
        self.regs
            .interrupter_register_set
            .interrupter_mut(0)
            .erdp
            .update_volatile(|d| {
                d.set_event_ring_dequeue_pointer(erdp);
            });
        // The crate's 64-bit ERDP store loses its high half on the BCM2711 (see
        // open); re-write it so the controller keeps a valid dequeue pointer and
        // keeps delivering events. Harmless on QEMU (erdp < 4 GiB → high = 0).
        unsafe { core::ptr::write_volatile(self.erdp_hi as *mut u32, (erdp >> 32) as u32) };
        ev
    }

    /// Enable Slot command → a device slot ID.
    ///
    /// `None` when the command timed out **or** completed with anything but
    /// Success — in particular No Slots Available once the controller's
    /// enabled slots (`MAX_SLOTS` cap in `open`) are used up. That error
    /// carries Slot ID 0 (xHCI 1.2 §4.6.3: "If a Device Slot is not available,
    /// the Slot ID field shall be cleared to '0'"), which was claimed as a
    /// device slot when the completion code went unchecked.
    pub fn enable_slot(&mut self) -> Option<u8> {
        let data = command::Allowed::EnableSlot(command::EnableSlot::new()).into_raw();
        let cc = self.command(data)?;
        match cc.completion_code() {
            Ok(event::CompletionCode::Success) => Some(cc.slot_id()),
            code => {
                kprintln!("[usb] enable slot → {:?}; device not enumerated", code);
                None
            }
        }
    }

    /// Reset a root-hub port (1-based) and wait for it to enable.
    fn port_reset(&mut self, port: u8) -> bool {
        let i = (port - 1) as usize;
        self.regs.port_register_set.update_volatile_at(i, |p| {
            p.portsc.set_port_reset();
        });
        for _ in 0..20_000_000u64 {
            let p = self.regs.port_register_set.read_volatile_at(i).portsc;
            if !p.port_reset() && p.port_enabled_disabled() {
                return true;
            }
            core::hint::spin_loop();
        }
        false
    }

    fn port_speed(&self, port: u8) -> u8 {
        self.regs
            .port_register_set
            .read_volatile_at((port - 1) as usize)
            .portsc
            .port_speed()
    }

    /// Address a device on `port` (speed `speed`) to `slot`: build the Input
    /// Context (slot + EP0), point DCBAA at the Device Context, and issue the
    /// Address Device command. Returns true on Success.
    fn address_device(&mut self, di: usize, port: u8, speed: u8) -> bool {
        let slot = self.hid[di].slot;
        // SAFETY: single-CPU; DEV/DMA are ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        dev.input.iter_mut().for_each(|x| *x = 0);
        dev.device.iter_mut().for_each(|x| *x = 0);

        let ep0_pa = dma_addr((&raw const dev.ep0_ring) as u64); // 64-byte aligned

        // View the zeroed input region as a 32-byte Input Context and fill it.
        // SAFETY: `Input32Byte` is `#[repr(C)]` over the same u32s as `input`.
        let input: &mut context::Input32Byte =
            unsafe { &mut *(dev.input.as_mut_ptr().cast()) };
        input.control_mut().set_add_context_flag(0); // slot
        input.control_mut().set_add_context_flag(1); // EP0
        let (route, tt_slot, tt_port) =
            (self.hid[di].route, self.hid[di].tt_hub_slot, self.hid[di].tt_port);
        {
            let s = input.device_mut().slot_mut();
            s.set_context_entries(1);
            s.set_root_hub_port_number(port);
            s.set_speed(speed);
            s.set_route_string(route);
            // A full/low-speed device behind a high-speed hub reaches the bus
            // through that hub's Transaction Translator.
            if tt_slot != 0 {
                s.set_parent_hub_slot_id(tt_slot);
                s.set_parent_port_number(tt_port);
            }
        }
        {
            let e = input.device_mut().endpoint_mut(1);
            e.set_endpoint_type(context::EndpointType::Control);
            e.set_max_packet_size(ep0_max_packet(speed));
            e.set_tr_dequeue_pointer(ep0_pa);
            e.set_dequeue_cycle_state(); // DCS = 1, matches the ring's PCS
            e.set_error_count(3);
            // Average TRB Length must be non-zero (circle sets 8); an EP0 with
            // ATL=0 is a malformed Endpoint Context that Address Device can
            // "Success" while leaving the slot in a bad state for later commands.
            e.set_average_trb_length(8);
        }

        // DCBAA[slot] → the (controller-written) Device Context. The Device
        // Context is 64-byte aligned (see `DevDma`); a misaligned base makes the
        // VL805 write it to the masked 64-byte address, so the slot state reads
        // back wrong and later Configure/Evaluate Context fail.
        let dma = unsafe { &mut *(&raw mut DMA) };
        dma.dcbaa.0[slot as usize] = dma_addr((&raw const dev.device) as u64);

        let input_pa = dma_addr((&raw const dev.input) as u64);
        let mut cmd = command::AddressDevice::new();
        cmd.set_input_context_pointer(input_pa);
        cmd.set_slot_id(slot);
        // BSR (Block Set Address Request) = 0 so the controller performs
        // SET_ADDRESS and moves the slot Default → Addressed.
        cmd.clear_block_set_address_request();
        let data = command::Allowed::AddressDevice(cmd).into_raw();
        match self.command(data) {
            Some(cc) if matches!(cc.completion_code(), Ok(event::CompletionCode::Success)) => {
                // The VL805 needs settling time after Address Device before the
                // slot is usable for later commands — circle does MsDelay(50).
                crate::arch::aarch64::delay_ms(50);
                true
            }
            Some(cc) => {
                kprintln!("[usb] address_device slot {} → {:?}", slot, cc.completion_code());
                false
            }
            None => {
                kprintln!("[usb] address_device slot {}: no completion", slot);
                false
            }
        }
    }

    /// Push a TRB onto the EP0 transfer ring.
    fn ep0_push(&mut self, di: usize, data: [u32; 4]) {
        // SAFETY: single-CPU; DEV is ours.
        let ring = unsafe { &mut (*(&raw mut DEV))[di].ep0_ring.0 };
        ring_push(ring, &mut self.hid[di].ep0_enq, &mut self.hid[di].ep0_cycle, data);
    }

    /// Wait for the Transfer Event that acknowledges the transfer just queued
    /// on `slot`/`ep` (`ep` is the endpoint's DCI — 1 for the default control
    /// endpoint). True when the controller completed it with Success.
    ///
    /// The event ring is shared by every slot and endpoint, so an event counts
    /// as *our* acknowledgement only when it names this slot **and** this
    /// endpoint. An event for anyone else is parked
    /// ([`Self::defer_transfer_event`]) and stays available to its owner.
    ///
    /// It must not be taken as our completion: a HID report that landed
    /// mid-enumeration then decided a control transfer's result (a 3-byte boot
    /// report completes as `ShortPacket`, so the control transfer "failed"),
    /// and every following acknowledgement was one transfer behind until the
    /// next `command()` wait loop silently swallowed the surplus event. And it
    /// must not be dropped: that leaves the device that owns it with no
    /// outstanding transfer and nothing left to re-arm it — the mouse in
    /// `docs/analysis/usb-port-investigation/run2-broken-both-in-usb3.log`.
    fn wait_transfer(&mut self, slot: u8, ep: u8) -> bool {
        // An event for this endpoint may already have been parked by an earlier
        // wait; by definition it is ours.
        if let Some(te) = self.take_pending(slot, ep) {
            #[cfg(feature = "usb-diag")]
            kprintln!(
                "[usb]   event: slot {} ep {} code {:?} len {} (was deferred)",
                te.slot_id(),
                te.endpoint_id(),
                te.completion_code(),
                te.trb_transfer_length()
            );
            return matches!(te.completion_code(), Ok(event::CompletionCode::Success));
        }
        for _ in 0..50_000_000u64 {
            if let Some(event::Allowed::TransferEvent(te)) = self.poll_event() {
                if te.slot_id() == slot && te.endpoint_id() == ep {
                    #[cfg(feature = "usb-diag")]
                    kprintln!(
                        "[usb]   event: slot {} ep {} code {:?} len {}",
                        te.slot_id(),
                        te.endpoint_id(),
                        te.completion_code(),
                        te.trb_transfer_length()
                    );
                    return matches!(te.completion_code(), Ok(event::CompletionCode::Success));
                }
                #[cfg(feature = "usb-diag")]
                kprintln!(
                    "[usb]   event: slot {} ep {} code {:?} len {} (deferred, waiting for \
                     slot {} ep {})",
                    te.slot_id(),
                    te.endpoint_id(),
                    te.completion_code(),
                    te.trb_transfer_length(),
                    slot,
                    ep
                );
                self.defer_transfer_event(te);
            }
            core::hint::spin_loop();
        }
        kprintln!("[usb] transfer timeout: slot {} ep {}", slot, ep);
        false
    }

    /// A control transfer on EP0. `setup` = the 8-byte SETUP packet; a data
    /// stage of `len` bytes at `buf_pa` follows when `len > 0` (direction
    /// `dir_in`); `len == 0` = no data stage.
    fn control(&mut self, di: usize, setup: [u8; 8], buf_pa: u64, len: u16, dir_in: bool) -> bool {
        let mut s = transfer::SetupStage::new();
        s.set_request_type(setup[0])
            .set_request(setup[1])
            .set_value(u16::from_le_bytes([setup[2], setup[3]]))
            .set_index(u16::from_le_bytes([setup[4], setup[5]]))
            .set_length(len)
            .set_transfer_type(if len == 0 {
                transfer::TransferType::No
            } else if dir_in {
                transfer::TransferType::In
            } else {
                transfer::TransferType::Out
            });
        self.ep0_push(di, transfer::Allowed::SetupStage(s).into_raw());
        if len > 0 {
            let mut d = transfer::DataStage::new();
            d.set_data_buffer_pointer(buf_pa)
                .set_trb_transfer_length(len as u32)
                .set_direction(if dir_in {
                    transfer::Direction::In
                } else {
                    transfer::Direction::Out
                });
            self.ep0_push(di, transfer::Allowed::DataStage(d).into_raw());
        }
        let mut st = transfer::StatusStage::new();
        // Status stage runs opposite the data stage. `set_direction()` is a
        // flag = IN; the default (no call) = OUT. So set IN unless this is an
        // IN data transfer (whose status stage is OUT).
        if !(len > 0 && dir_in) {
            st.set_direction();
        }
        st.set_interrupt_on_completion();
        self.ep0_push(di, transfer::Allowed::StatusStage(st).into_raw());

        let slot = self.hid[di].slot;
        self.dma_clean();
        self.regs.doorbell.update_volatile_at(slot as usize, |db| {
            db.set_doorbell_target(1);
        });
        // EP0's DCI is 1 — the Endpoint ID a Transfer Event reports for it.
        let ok = self.wait_transfer(slot, 1);
        // The controller DMA'd the IN data into our buffer — drop the stale
        // cached copy so the caller reads it (real Pi-4; no-op on QEMU).
        if dir_in && len > 0 {
            // SAFETY: single-CPU; DEV is ours.
            let dev = unsafe { &*(&raw const DEV) };
            dcache_inval((&raw const dev[di].buf) as u64, dev[di].buf.len());
        }
        ok
    }

    /// GET_DESCRIPTOR into `DEV[di].buf` (data there on success).
    fn get_descriptor(&mut self, di: usize, desc_type: u8, index: u8, len: u16) -> bool {
        // SAFETY: single-CPU; DEV is ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        dev.buf.iter_mut().for_each(|b| *b = 0);
        let buf_pa = dma_addr((&raw const dev.buf) as u64);
        let value = ((desc_type as u16) << 8) | index as u16;
        let setup = [
            0x80,
            6,
            value as u8,
            (value >> 8) as u8,
            0,
            0,
            len as u8,
            (len >> 8) as u8,
        ];
        self.control(di, setup, buf_pa, len, true)
    }

    /// Read + log the 18-byte device descriptor, correcting EP0's max packet
    /// size first (two-phase enumeration).
    pub fn get_device_descriptor(&mut self, di: usize) -> bool {
        // Phase 1: read the first 8 bytes to learn bMaxPacketSize0. We addressed
        // the device with EP0 max packet 8 (the safe minimum); a full-speed
        // device whose real EP0 is 16/32/64 would babble on a direct 18-byte
        // read (it packs the reply into one > 8-byte packet). 8 bytes fit one
        // packet for any EP0 >= 8, so this read always succeeds.
        if !self.get_descriptor(di, 1, 0, 8) {
            kprintln!("[usb] get device descriptor (first 8) failed");
            snap(di, |d| d.device_desc_state = Fetch::Failed);
            return false;
        }
        // SAFETY: single-CPU; DEV is ours. [7] = bMaxPacketSize0.
        let bmax0 = unsafe { (*(&raw const DEV))[di].buf[7] } as u16;
        if bmax0 > ep0_max_packet(self.hid[di].speed) {
            // The real EP0 is larger than our guess — correct it (xHCI §4.6.7
            // lists EP0 Max Packet Size as an Evaluate Context field) so the
            // full read below does not babble.
            if !self.update_ep0_max_packet(di, bmax0) {
                snap(di, |d| d.device_desc_state = Fetch::Failed);
                return false;
            }
        }
        // Phase 2: read the full 18-byte descriptor with the corrected EP0.
        if !self.get_descriptor(di, 1, 0, 18) {
            kprintln!("[usb] get device descriptor failed");
            snap(di, |d| d.device_desc_state = Fetch::Failed);
            return false;
        }
        // SAFETY: single-CPU; DEV is ours.
        let b = unsafe { &(*(&raw const DEV))[di].buf };
        kprintln!(
            "[usb] device descriptor: bLength={} type={} bcdUSB={:02x}{:02x} class={} maxpkt0={} VID={:02x}{:02x} PID={:02x}{:02x}",
            b[0], b[1], b[3], b[2], b[4], b[7], b[9], b[8], b[11], b[10]
        );
        self.hid[di].class = b[4];
        self.hid[di].vid = u16::from_le_bytes([b[8], b[9]]);
        self.hid[di].pid = u16::from_le_bytes([b[10], b[11]]);
        // The snapshot keeps the raw bytes (usb-snapshot.md §3.1 step 2, K2):
        // `buf` is overwritten by the next transfer.
        let mut raw = [0u8; 18];
        raw.copy_from_slice(&b[..18]);
        snap(di, |d| {
            d.device_desc = raw;
            d.device_desc_state = Fetch::Ok;
            d.flags |= dev_flag::DEVICE_DESC;
            d.dev_class = raw[4];
            d.vid = u16::from_le_bytes([raw[8], raw[9]]);
            d.pid = u16::from_le_bytes([raw[10], raw[11]]);
            d.manufacturer.present = raw[14] != 0;
            d.product.present = raw[15] != 0;
        });
        true
    }

    /// Update EP0's Max Packet Size via an Evaluate Context command, after the
    /// first 8 descriptor bytes revealed a `bMaxPacketSize0` larger than the
    /// value we addressed with. xHCI §4.6.7 lists the default control endpoint's
    /// Max Packet Size as an evaluable field. The input is seeded from the live
    /// (now correctly aligned) device context so the slot + EP0 base fields are
    /// preserved; only A1 (EP0) is flagged, so the controller re-reads just the
    /// EP0 Max Packet Size.
    fn update_ep0_max_packet(&mut self, di: usize, max_packet: u16) -> bool {
        let slot = self.hid[di].slot;
        // SAFETY: single-CPU; DEV is ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        dev.input.iter_mut().for_each(|x| *x = 0);
        dcache_inval((&raw const dev.device) as u64, core::mem::size_of_val(&dev.device));
        let n = dev.device.len();
        for k in 0..n {
            dev.input[8 + k] = dev.device[k];
        }
        // SAFETY: `Input32Byte` is `#[repr(C)]` over the same u32s as `input`.
        let input: &mut context::Input32Byte =
            unsafe { &mut *(dev.input.as_mut_ptr().cast()) };
        input.control_mut().set_add_context_flag(1); // A1: EP0 (dci 1)
        input.device_mut().endpoint_mut(1).set_max_packet_size(max_packet);
        let input_pa = dma_addr((&raw const dev.input) as u64);
        let mut cmd = command::EvaluateContext::new();
        cmd.set_input_context_pointer(input_pa);
        cmd.set_slot_id(slot);
        let data = command::Allowed::EvaluateContext(cmd).into_raw();
        match self.command(data) {
            Some(cc) if matches!(cc.completion_code(), Ok(event::CompletionCode::Success)) => true,
            Some(cc) => {
                kprintln!("[usb] update_ep0_max_packet → {:?}", cc.completion_code());
                false
            }
            None => false,
        }
    }

    /// Discovered-device info for display (the init-screen USB tree).
    pub fn device_info(&self, di: usize) -> DeviceInfo {
        let d = &self.hid[di];
        DeviceInfo {
            port: d.port,
            protocol: d.protocol,
            vid: d.vid,
            pid: d.pid,
        }
    }

    /// Number of root-hub ports the controller exposes (for the tree header).
    pub fn root_ports(&self) -> u8 {
        self.ports
    }

    /// Read the configuration descriptor and choose the interface the driver
    /// takes: the first HID boot keyboard/mouse interface (class 3, subclass
    /// 1, protocol 1/2) with an interrupt-IN endpoint → `(info, None)`; else
    /// the first HID interface in alternate setting 0 with an interrupt-IN
    /// endpoint, to record but not decode (`usb-snapshot.md` §3.1 step 5 (ii))
    /// → `(info, Some(reason code))`. `None` if the device has neither — it is
    /// then not an input device; each `None` path logs why and sets the
    /// device's snapshot reason.
    ///
    /// The configuration is read whole up to `snapshot::CONFIG_MAX` bytes into
    /// the enumeration buffer and kept in the snapshot with its interface
    /// table (K2).
    pub fn read_config(&mut self, di: usize) -> Option<(HidInfo, Option<u8>)> {
        // Read the 9-byte config header first for wTotalLength.
        if !self.get_descriptor(di, 2, 0, 9) {
            kprintln!("[usb] slot {}: configuration header read failed", self.hid[di].slot);
            self.hid[di].reason = reason::CONFIG_READ;
            return None;
        }
        // SAFETY: single-CPU; DEV is ours.
        let (total, w_total) = {
            let b = unsafe { &(*(&raw const DEV))[di].buf };
            let w_total = u16::from_le_bytes([b[2], b[3]]);
            (w_total.min(snapshot::CONFIG_MAX as u16), w_total)
        };
        if w_total > total {
            kprintln!(
                "[usb] slot {}: configuration is {} bytes, read only the first {} \
                 (descriptor buffer limit); descriptors past that are not seen",
                self.hid[di].slot,
                w_total,
                total
            );
        }
        if !self.control_in_enum(di, [0x80, 6, 0, 2, 0, 0, total as u8, (total >> 8) as u8], total)
        {
            kprintln!(
                "[usb] slot {}: configuration descriptor read ({} bytes) failed",
                self.hid[di].slot,
                total
            );
            self.hid[di].reason = reason::CONFIG_READ;
            return None;
        }
        let b = &enum_buf()[..total as usize];
        // Diagnostic build only: the whole configuration, before we pick from
        // it — every descriptor, so a composite device's other interfaces and
        // class-specific descriptors are visible too.
        #[cfg(feature = "usb-diag")]
        {
            kprintln!("[usb] config descriptor, {} bytes, all descriptors:", total);
            let mut j = 0usize;
            while j + 2 <= b.len() {
                let dl = b[j] as usize;
                let dt = b[j + 1];
                if dl == 0 || j + dl > b.len() {
                    break;
                }
                match dt {
                    4 if dl >= 9 => kprintln!(
                        "[usb]   iface {} alt {} eps {} class {} subclass {} protocol {}",
                        b[j + 2], b[j + 3], b[j + 4], b[j + 5], b[j + 6], b[j + 7]
                    ),
                    5 if dl >= 7 => kprintln!(
                        "[usb]   endpoint {:#04x} attr {:#04x} max {} interval {}",
                        b[j + 2],
                        b[j + 3],
                        u16::from_le_bytes([b[j + 4], b[j + 5]]),
                        b[j + 6]
                    ),
                    33 => kprintln!("[usb]   HID descriptor, {} bytes", dl),
                    _ => kprintln!("[usb]   descriptor type {}, {} bytes", dt, dl),
                }
                j += dl;
            }
        }
        // USB01 F2: only a HID (class 3) boot-subclass (1) keyboard/mouse
        // interface is an input device (HID 1.11 §4.1–§4.3); see
        // `config_desc`. Every interface not taken gets one log line (F8).
        let slot = self.hid[di].slot;
        let scan = config_desc::scan(b, |sk| {
            kprintln!(
                "[usb] slot {} iface {} alt {} (class {} subclass {} protocol {}) not used: {}",
                slot,
                sk.number,
                sk.alt,
                sk.class,
                sk.subclass,
                sk.protocol,
                config_desc::reason_text(sk.reason)
            );
        });
        if let Some(at) = scan.stopped_at {
            kprintln!(
                "[usb] slot {}: configuration descriptor malformed or cut at byte {} of {}; \
                 rest not parsed",
                slot,
                at,
                total
            );
        }
        // Keep the raw configuration and its interface table (§2.5 types 2/4).
        snap(di, |d| {
            d.config[..b.len()].copy_from_slice(b);
            d.config_len = total;
            d.flags |= dev_flag::CONFIG;
            if w_total > total {
                d.flags |= dev_flag::CONFIG_CAPPED;
            }
            d.n_hid_ifaces = scan.n_hid_ifaces;
            d.ifaces_cut = scan.rows_cut || scan.n_rows > snapshot::IFACE_ROWS;
            let n = scan.n_rows.min(snapshot::IFACE_ROWS);
            for (o, r) in d.ifaces.iter_mut().zip(&scan.rows[..n]) {
                *o = snapshot::IfaceRow {
                    number: r.number,
                    alt: r.alt,
                    class: r.class,
                    subclass: r.subclass,
                    protocol: r.protocol,
                    reason: r.skip.map_or(reason::USED, skip_code),
                    ep_addr: r.ep_addr,
                    interval: r.interval,
                    max_packet: r.max_packet,
                    report_len: r.report_len,
                    monitored: false,
                };
            }
            d.n_ifaces = n as u8;
        });
        match (scan.hid, scan.observe) {
            (Some(h), _) => {
                kprintln!(
                    "[usb] HID iface {} protocol {} → int-IN ep {:#04x} max {} interval {}",
                    h.iface, h.protocol, h.ep_addr, h.ep_max, h.interval
                );
                Some((h, None))
            }
            (None, Some(o)) => {
                let h = o.info;
                kprintln!(
                    "[usb] slot {} iface {}: report-protocol HID interface observed \
                     (reports recorded, not decoded) → int-IN ep {:#04x} max {} interval {}",
                    slot, h.iface, h.ep_addr, h.ep_max, h.interval
                );
                Some((h, Some(skip_code(o.reason))))
            }
            (None, None) => {
                kprintln!(
                    "[usb] slot {}: no HID boot keyboard/mouse interface — not an input device",
                    slot
                );
                self.hid[di].reason = reason::NO_HID_INTERFACE;
                None
            }
        }
    }

    /// A control IN transfer whose data stage lands in the enumeration buffer
    /// ([`ENUM_BUF`], zeroed first). True on Success.
    fn control_in_enum(&mut self, di: usize, setup: [u8; 8], len: u16) -> bool {
        // SAFETY: single-CPU; ENUM_BUF is ours, no other reference is alive.
        let buf = unsafe { &mut (*core::ptr::addr_of_mut!(ENUM_BUF)).0 };
        let len = len.min(buf.len() as u16);
        buf[..len as usize].iter_mut().for_each(|b| *b = 0);
        let va = buf.as_ptr() as u64;
        let ok = self.control(di, setup, dma_addr(va), len, true);
        dcache_inval(va, len as usize);
        ok
    }

    /// The last step of `enumerate_device` for a configured device
    /// (`usb-snapshot.md` §3.1 step 7, K6): fetch the HID report descriptors
    /// (first four HID interfaces in alternate setting 0) and the manufacturer
    /// and product strings into the snapshot. No serial number (§2.5).
    ///
    /// Last on purpose: a STALL halts EP0 (xHCI 1.2 §4.8.3) and there is no
    /// Reset Endpoint path, so a later transfer on this EP0 would only time
    /// out. EP0 is not used again after this in v1; within this step, the
    /// first failure ends it — everything after it is left `SKIPPED`.
    fn fetch_extras(&mut self, di: usize) {
        let slot = self.hid[di].slot;
        // (interface number, declared length) of the HID alt-0 rows that get a
        // report descriptor slot.
        let mut wanted = [(0u8, 0u16); snapshot::REPORT_DESC_IFACES];
        let mut n_wanted = 0;
        let mut strings = (0u8, 0u8);
        super::with_store(|s| {
            if let Some(d) = s.device_mut(di) {
                for r in d.ifaces[..d.n_ifaces as usize].iter().filter(|r| r.class == 3 && r.alt == 0) {
                    if n_wanted == wanted.len() {
                        break;
                    }
                    wanted[n_wanted] = (r.number, r.report_len);
                    n_wanted += 1;
                }
                if d.device_desc_state == Fetch::Ok {
                    strings = (d.device_desc[14], d.device_desc[15]);
                }
            }
        });

        // a. Report descriptors: GET_DESCRIPTOR(Report) to the interface
        //    (HID 1.11 §7.1.1: bmRequestType 0x81, wValue 0x2200, wIndex =
        //    interface number, wLength = the HID descriptor's length).
        for (k, &(iface, declared)) in wanted[..n_wanted].iter().enumerate() {
            if declared == 0 {
                continue; // no HID descriptor: nothing to ask for (SKIPPED)
            }
            let want = declared.min(snapshot::REPORT_DESC_MAX as u16);
            let setup = [0x81, 6, 0, 0x22, iface, 0, want as u8, (want >> 8) as u8];
            if !self.control_in_enum(di, setup, want) {
                kprintln!(
                    "[usb] slot {} iface {}: report descriptor ({} bytes) read failed; \
                     no further descriptor requests to this device",
                    slot,
                    iface,
                    want
                );
                snap(di, |d| d.reports[k].state = Fetch::Failed);
                return;
            }
            let bytes = &enum_buf()[..want as usize];
            snap(di, |d| {
                let rd = &mut d.reports[k];
                rd.bytes[..bytes.len()].copy_from_slice(bytes);
                rd.len = want;
                rd.state = if declared > want { Fetch::Truncated } else { Fetch::Ok };
            });
            kprintln!(
                "[usb] slot {} iface {}: report descriptor {} bytes (declared {})",
                slot,
                iface,
                want,
                declared
            );
        }

        // b. Manufacturer / product strings, best effort: the first LANGID of
        //    string descriptor 0, then each string in that language
        //    (GET_DESCRIPTOR(String), wIndex = LANGID). 64 bytes = header +
        //    31 UTF-16 code units, everything the snapshot keeps.
        let (i_mfr, i_prod) = strings;
        if i_mfr == 0 && i_prod == 0 {
            return;
        }
        let fail_strings = |which: &str| {
            kprintln!(
                "[usb] slot {}: string descriptor ({}) read failed; strings not kept",
                slot,
                which
            );
            snap(di, |d| {
                for t in [&mut d.manufacturer, &mut d.product] {
                    if t.present && t.state == Fetch::NotTried {
                        t.state = Fetch::Failed;
                    }
                }
            });
        };
        if !self.control_in_enum(di, [0x80, 6, 0, 3, 0, 0, 4, 0], 4) {
            fail_strings("language list");
            return;
        }
        let l = enum_buf();
        if l[0] < 4 || l[1] != 3 {
            fail_strings("no language id");
            return;
        }
        let lang = [l[2], l[3]];
        for (index, is_mfr) in [(i_mfr, true), (i_prod, false)] {
            if index == 0 {
                continue;
            }
            let setup = [0x80, 6, index, 3, lang[0], lang[1], 64, 0];
            if !self.control_in_enum(di, setup, 64) {
                fail_strings(if is_mfr { "manufacturer" } else { "product" });
                return;
            }
            let raw = &enum_buf()[..64];
            snap(di, |d| {
                let t = if is_mfr { &mut d.manufacturer } else { &mut d.product };
                if raw[1] == 3 {
                    t.set_from_descriptor(raw);
                } else {
                    t.state = Fetch::Failed;
                }
            });
        }
    }

    /// Read the configuration descriptor and return its `bConfigurationValue`
    /// only — the minimal bring-up a hub needs. Unlike [`read_config`] it does
    /// not parse (or log) a HID interface/endpoint, so a hub's status-change
    /// interrupt endpoint is not mislabelled as a HID device.
    fn read_config_value(&mut self, di: usize) -> Option<u8> {
        // The 9-byte config header carries bConfigurationValue at offset 5.
        if !self.get_descriptor(di, 2, 0, 9) {
            return None;
        }
        // SAFETY: single-CPU; DEV is ours.
        Some(unsafe { (*(&raw const DEV))[di].buf[5] })
    }

    /// SET_CONFIGURATION (no-data control OUT).
    fn set_configuration(&mut self, di: usize, value: u8) -> bool {
        self.control(di, [0x00, 9, value, 0, 0, 0, 0, 0], 0, 0, false)
    }

    /// HID SET_PROTOCOL → boot protocol (class control OUT to the interface).
    fn set_protocol_boot(&mut self, di: usize, iface: u8) -> bool {
        self.control(di, [0x21, 0x0B, 0, 0, iface, 0, 0, 0], 0, 0, false)
    }

    /// Configure Endpoint: add the HID interrupt-IN endpoint to the device.
    /// Returns its DCI (Device Context Index) on success, 0 on failure.
    fn configure_endpoint(&mut self, di: usize, _port: u8, speed: u8, hid: &HidInfo) -> u8 {
        let slot = self.hid[di].slot;
        // SAFETY: single-CPU; DEV is ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        dev.input.iter_mut().for_each(|x| *x = 0);
        // Seed the Input Context from the whole live Device Context so the
        // controller-managed slot fields (route, speed, TT, max exit latency,
        // interrupter — set at Address Device) are preserved rather than
        // zeroed. QEMU requires A0 (the slot context) on Configure Endpoint or
        // it returns TRB Error; the real VL805 rejects a *rebuilt-from-scratch*
        // slot context with Context State Error. Copying + A0 satisfies both.
        dcache_inval((&raw const dev.device) as u64, core::mem::size_of_val(&dev.device));
        let n = dev.device.len();
        for k in 0..n {
            dev.input[8 + k] = dev.device[k];
        }
        let ep_num = hid.ep_addr & 0x0F;
        let dci = ep_num * 2 + if hid.ep_addr & 0x80 != 0 { 1 } else { 0 };
        let int_pa = dma_addr((&raw const dev.int_ring) as u64);

        // SAFETY: `Input32Byte` is `#[repr(C)]` over the same u32s as `input`.
        let input: &mut context::Input32Byte =
            unsafe { &mut *(dev.input.as_mut_ptr().cast()) };
        input.control_mut().set_add_context_flag(0); // A0: slot (kept from copy)
        input.control_mut().set_add_context_flag(dci as usize); // A_dci: new EP
        {
            // Only extend Context Entries to cover the new endpoint; the rest of
            // the slot context stays as the controller wrote it.
            input.device_mut().slot_mut().set_context_entries(dci);
        }
        {
            let e = input.device_mut().endpoint_mut(dci as usize);
            e.set_endpoint_type(context::EndpointType::InterruptIn);
            e.set_max_packet_size(hid.ep_max);
            e.set_tr_dequeue_pointer(int_pa);
            e.set_dequeue_cycle_state();
            e.set_error_count(3);
            // Interrupt-endpoint fields the VL805 validates — leaving them 0
            // returned Parameter Error:
            //   * Average TRB Length must be > 0 (xHCI §6.2.3.4). circle uses
            //     the max packet size for interrupt endpoints.
            //   * Max ESIT Payload = max packet size for a single-burst FS/LS/HS
            //     interrupt endpoint.
            //   * Interval must be legal for the speed (§6.2.3.6, Table 6-12):
            //     a full/low-speed interrupt endpoint uses 3..=10 (period
            //     2^Interval × 125 µs); Interval 0 is illegal there. High-speed
            //     interrupt encodes bInterval (125-µs units) as bInterval − 1.
            e.set_average_trb_length(hid.ep_max);
            e.set_max_endpoint_service_time_interval_payload_low(hid.ep_max);
            let interval = if speed == 3 {
                hid.interval.saturating_sub(1).min(15)
            } else {
                let bi = hid.interval.max(1) as u32; // bInterval in frames (ms)
                let log2 = 31 - bi.leading_zeros(); // floor(log2(bInterval))
                ((3 + log2) as u8).clamp(3, 10)
            };
            e.set_interval(interval);
        }

        let input_pa = dma_addr((&raw const dev.input) as u64);
        let mut cmd = command::ConfigureEndpoint::new();
        cmd.set_input_context_pointer(input_pa);
        cmd.set_slot_id(slot);
        let data = command::Allowed::ConfigureEndpoint(cmd).into_raw();
        match self.command(data) {
            Some(cc) if matches!(cc.completion_code(), Ok(event::CompletionCode::Success)) => dci,
            Some(cc) => {
                kprintln!("[usb] configure_endpoint → {:?}", cc.completion_code());
                0
            }
            None => 0,
        }
    }

    /// Bring a HID device online: read its config, add its interrupt
    /// endpoint, SET_CONFIGURATION, and — for a boot interface only —
    /// SET_PROTOCOL(boot). Returns `(dci, protocol)` ready for interrupt
    /// polling; `protocol` is 1/2 for a boot keyboard/mouse (decoded) and 0
    /// for an observed report-protocol interface (recorded only,
    /// `usb-snapshot.md` §3.2). Sets the device's snapshot role/reason.
    pub fn configure_hid(&mut self, di: usize, port: u8, speed: u8) -> Option<(u8, u8)> {
        let (hid, observed) = self.read_config(di)?;
        let boot = observed.is_none();
        // The chosen row of the interface table: its reason becomes the
        // device's failure reason while configuring is not done, 0 after.
        let set_row_reason = |code: u8| {
            snap(di, |d| {
                let n = d.n_ifaces as usize;
                if let Some(r) = d.ifaces[..n].iter_mut().find(|r| r.number == hid.iface && r.alt == 0) {
                    r.reason = code;
                    r.monitored = code == reason::USED;
                }
            })
        };
        // xHCI enumeration order (spec 4.3.5): the Configure Endpoint *command*
        // (controller side) precedes the SET_CONFIGURATION *request* (device
        // side). QEMU tolerates the reverse; the real VL805 returns Context
        // State Error if SET_CONFIGURATION ran first.
        let dci = self.configure_endpoint(di, port, speed, &hid);
        if dci == 0 {
            self.hid[di].reason = reason::CONFIGURE_ENDPOINT;
            set_row_reason(reason::CONFIGURE_ENDPOINT);
            return None;
        }
        if !self.set_configuration(di, hid.config_value) {
            kprintln!("[usb] set_configuration failed");
            self.hid[di].reason = reason::SET_CONFIGURATION;
            set_row_reason(reason::SET_CONFIGURATION);
            return None;
        }
        snap(di, |d| d.flags |= dev_flag::SET_CONFIGURATION);
        // HID 1.11 §7.2.6: only boot-subclass devices support SET_PROTOCOL.
        if boot && !self.set_protocol_boot(di, hid.iface) {
            kprintln!("[usb] set_protocol(boot) failed");
            self.hid[di].reason = reason::SET_PROTOCOL;
            set_row_reason(reason::SET_PROTOCOL);
            return None;
        }
        kprintln!(
            "[usb] HID configured: slot {} ep {:#04x} dci {} ({})",
            self.hid[di].slot,
            hid.ep_addr,
            dci,
            match (boot, hid.protocol) {
                (true, 1) => "keyboard",
                (true, _) => "mouse",
                (false, _) => "observed only, not decoded",
            }
        );
        let max = hid.ep_max & 0x7FF;
        self.hid[di].trb_len = max.clamp(INT_TRB_MIN, INT_TRB_MAX as u16);
        let (role, why) = match observed {
            None if hid.protocol == 1 => (role::KEYBOARD, reason::USED),
            None => (role::MOUSE, reason::USED),
            Some(code) => (role::OBSERVED, code),
        };
        self.hid[di].role = role;
        self.hid[di].reason = why;
        set_row_reason(reason::USED);
        snap(di, |d| {
            d.ring_iface = hid.iface;
            d.ring_ep = hid.ep_addr;
            d.ring_capped = max as usize > snapshot::REPORT_MAX;
        });
        Some((dci, if boot { hid.protocol } else { 0 }))
    }

    /// Enumerate every connected root port, recursing through any hubs, and
    /// configure each HID boot device (keyboard + mouse) for interrupt polling.
    /// Returns the number of device slots claimed (hubs included).
    pub fn enumerate_all(&mut self) -> usize {
        for port in 1..=self.ports {
            let p = self
                .regs
                .port_register_set
                .read_volatile_at((port - 1) as usize)
                .portsc;
            if !p.current_connect_status() {
                continue;
            }
            if self.n_hid >= MAX_HID {
                kprintln!(
                    "[usb] root port {}: device not enumerated — all {} device entries \
                     (MAX_HID, hubs included) in use",
                    port,
                    MAX_HID
                );
                snap_skipped();
                continue;
            }
            if !self.port_reset(port) {
                kprintln!("[usb] port {}: reset failed", port);
                snap_skipped();
                continue;
            }
            let speed = self.port_speed(port);
            // A device on a root port: route string 0, no parent TT, no hub.
            self.enumerate_device(port, 0, speed, 0, 0, 0, 0xFF);
        }
        // Only now that the whole tree has been walked.
        self.arm_hid_endpoints();
        let armed = |i: &usize| self.hid[*i].dci != 0;
        let n_input = (0..self.n_hid).filter(armed).filter(|&i| self.hid[i].protocol != 0).count();
        let n_observed = (0..self.n_hid).filter(armed).filter(|&i| self.hid[i].protocol == 0).count();
        kprintln!(
            "[usb] enumeration done: {} devices ({} keyboard/mouse, {} observed only), {} \
             commands issued",
            self.n_hid,
            n_input,
            n_observed,
            self.cmd_count
        );
        self.n_hid
    }

    /// Arm the interrupt endpoint of every configured HID device, once.
    ///
    /// Deliberately *after* the device tree has been walked. While enumeration
    /// runs, every transfer on the ring must belong to the device being
    /// enumerated: a HID armed earlier can report at any moment, and its
    /// completion then arrives in the middle of another device's control
    /// transfers. FreeMiNT has that property by construction — its class driver
    /// never leaves an interrupt transfer outstanding. `kbd_probe`
    /// (`sys/usb/src.km/udd/hid/keyboard/usb_keyboard.c:527`) only records the
    /// pipe and creates a poll thread; the transfer itself is issued
    /// synchronously from `kbd_int` (`usb_keyboard.c:357`, `usb_bulk_msg`) once
    /// that thread is scheduled, i.e. after `usb_new_device` has returned and
    /// the tree is built.
    fn arm_hid_endpoints(&mut self) {
        for di in 0..self.n_hid {
            if self.hid[di].dci != 0 {
                self.queue_interrupt(di);
            }
        }
    }

    /// Bring one device online: Enable Slot → Address Device → read its device
    /// descriptor, then either configure it as a HID boot device or, if it is a
    /// hub (class 9), recurse into its downstream ports. `root_port` is the
    /// root-hub port the path starts at; `route` is the xHCI route string;
    /// `tt_slot`/`tt_port` name the Transaction-Translator hub (0 = direct);
    /// `depth` is the device's hub tier (0 = on a root port); `parent` is the
    /// `hid[]` index of the hub it hangs off (`0xFF` = a root port).
    ///
    /// Every claimed slot gets a snapshot entry (`usb-snapshot.md` §2.4),
    /// published with its role and reason when this call is done.
    #[allow(clippy::too_many_arguments)]
    fn enumerate_device(
        &mut self,
        root_port: u8,
        route: u32,
        speed: u8,
        tt_slot: u8,
        tt_port: u8,
        depth: u8,
        parent: u8,
    ) {
        if self.n_hid >= MAX_HID {
            kprintln!(
                "[usb] device at root port {} route {:#x} not enumerated — all {} device \
                 entries (MAX_HID, hubs included) in use",
                root_port,
                route,
                MAX_HID
            );
            snap_skipped();
            return;
        }
        let Some(slot) = self.enable_slot() else {
            kprintln!("[usb] enable slot failed (port {} route {:#x})", root_port, route);
            snap_skipped();
            return;
        };
        // Claim the slot now (the device — a hub especially — persists past this
        // call); the transfer methods read its addressing from self.hid[di].
        let di = self.n_hid;
        self.hid[di] = HidDev::new();
        self.hid[di].slot = slot;
        self.hid[di].port = root_port;
        self.hid[di].route = route;
        self.hid[di].speed = speed;
        self.hid[di].tt_hub_slot = tt_slot;
        self.hid[di].tt_port = tt_port;
        self.hid[di].parent = parent;
        self.n_hid += 1;
        super::with_store(|s| {
            if let Some(d) = s.claim(di) {
                d.slot = slot;
                d.root_port = root_port;
                d.parent = parent;
                d.route = route;
                d.speed = speed;
            }
        });
        self.enumerate_claimed(di, root_port, speed, depth);
        let (r, why) = (self.hid[di].role, self.hid[di].reason);
        snap(di, |d| {
            d.role = r;
            d.reason = why;
        });
    }

    /// The part of [`Self::enumerate_device`] after the slot is claimed:
    /// address, describe, then configure (HID) or descend (hub). Sets
    /// `hid[di].role` / `.reason` for the snapshot.
    fn enumerate_claimed(&mut self, di: usize, root_port: u8, speed: u8, depth: u8) {
        let slot = self.hid[di].slot;
        let route = self.hid[di].route;
        #[cfg(feature = "usb-diag")]
        let (tt_slot, tt_port) = (self.hid[di].tt_hub_slot, self.hid[di].tt_port);
        kprintln!(
            "[usb] device slot {} (root port {} route {:#x} speed {})",
            slot, root_port, route, speed
        );
        // Diagnostic build: the rest of the addressing identity. `tt` names the
        // hub whose Transaction Translator a low/full-speed device reaches the
        // bus through — 0 means "none was set", which is correct only for a
        // device that is not behind a high-speed hub. `depth` is the hub tier,
        // and `di` is the index this device occupies in `hid[]`, i.e. the
        // enumeration order — and the order `arm_hid_endpoints` walks once the
        // tree is complete.
        #[cfg(feature = "usb-diag")]
        kprintln!(
            "[usb]   di {} tt_slot {} tt_port {} depth {} (enumeration order)",
            di, tt_slot, tt_port, depth
        );
        if !self.address_device(di, root_port, speed) {
            // F8: counted as "not enumerated"; the slot stays claimed (§3.1
            // step 1: role 5).
            snap_skipped();
            self.hid[di].role = role::FAILED;
            self.hid[di].reason = reason::ADDRESS_DEVICE;
            return;
        }
        let described = self.get_device_descriptor(di);
        if self.hid[di].class == 9 {
            self.hid[di].protocol = 9; // hub: tracked, never polled as HID
            self.hid[di].role = role::HUB;
            // DIAGNOSTIC re-enable (device-context 64-byte alignment test): if
            // the aligned output device context now reads a valid Addressed slot
            // context, configure_hub should succeed. See docs/STATUS.md.
            self.enumerate_hub(di, root_port, speed, depth);
        } else if let Some((dci, protocol)) = self.configure_hid(di, root_port, speed) {
            self.hid[di].dci = dci;
            self.hid[di].protocol = protocol;
            // The endpoint is configured but NOT armed here: `enumerate_all`
            // arms every HID once the whole tree is walked
            // (`arm_hid_endpoints`), so that no device can report while another
            // device's control transfers are still on the ring.
            //
            // The risky requests come last (§3.1 step 7): a STALL here halts
            // EP0, which nothing after this point uses.
            self.fetch_extras(di);
        }
        if !described && self.hid[di].role == role::NONE {
            // Without a device descriptor nothing could be configured: the
            // device is unusable (§2.4 role 5, reason 22).
            self.hid[di].role = role::FAILED;
            self.hid[di].reason = reason::DEVICE_DESCRIPTOR;
        }
    }

    /// Drive a hub: configure it as a hub, power its downstream ports, and
    /// enumerate the device on each connected port (recursively).
    fn enumerate_hub(&mut self, di: usize, root_port: u8, speed: u8, depth: u8) {
        let Some(config_value) = self.read_config_value(di) else {
            kprintln!("[usb] hub: read config failed");
            self.hid[di].reason = reason::CONFIG_READ;
            return;
        };
        let Some(desc) = self.hub_get_descriptor(di) else {
            kprintln!("[usb] hub: get descriptor failed");
            return;
        };
        let nbr = desc.nbr_ports;
        // Configure Endpoint (mark slot as a hub) before SET_CONFIGURATION.
        if !self.configure_hub(di, speed, nbr) {
            self.hid[di].reason = reason::CONFIGURE_ENDPOINT;
            return;
        }
        if !self.set_configuration(di, config_value) {
            kprintln!("[usb] hub: set_configuration failed");
            self.hid[di].reason = reason::SET_CONFIGURATION;
            return;
        }
        snap(di, |d| d.flags |= dev_flag::SET_CONFIGURATION);
        kprintln!("[usb] hub slot {}: {} downstream ports", self.hid[di].slot, nbr);
        self.hub_power_on(di, nbr, desc.power_on_to_power_good_ms());
        let hub_slot = self.hid[di].slot;
        let parent_route = self.hid[di].route;
        let (parent_tt_slot, parent_tt_port) =
            (self.hid[di].tt_hub_slot, self.hid[di].tt_port);
        for p in 1..=nbr {
            if self.n_hid >= MAX_HID {
                kprintln!(
                    "[usb] hub slot {}: ports {}..{} not scanned — all {} device entries \
                     (MAX_HID, hubs included) in use",
                    hub_slot,
                    p,
                    nbr,
                    MAX_HID
                );
                snap_skipped();
                break;
            }
            // A failed status read is not "nothing connected": FreeMiNT
            // skips the port on `usb_get_port_status(...) < 0` (`hub.c:486`).
            let Some(st) = self.hub_get_port_status(di, p) else {
                kprintln!("[usb] hub port {}: get_port_status failed", p);
                continue;
            };
            let connected = st & 0x1 != 0;
            kprintln!("[usb] hub port {}: status={:#06x} connected={}", p, st & 0xffff, connected);
            if !connected {
                continue;
            }
            // Acknowledge the connection change before resetting, as the
            // reference does (`hub.c:280`). FreeMiNT ignores the return value;
            // we report a failure instead of discarding it, because a hub that
            // will not clear C_PORT_CONNECTION makes the reset loop below abort
            // on its own change check.
            if !self.hub_clear_port_feature(di, p, 16) {
                kprintln!("[usb] hub port {}: CLEAR_FEATURE(C_PORT_CONNECTION) failed", p);
            }
            // `hub_port_reset` owns the "did it enable" decision and clears
            // C_PORT_RESET itself on success; `None` = "cannot reset port"
            // (`hub.c:294`), so leave the port alone.
            let Some(st2) = self.hub_port_reset(di, p) else {
                continue;
            };
            // PORT_LOW_SPEED (bit 9) / PORT_HIGH_SPEED (bit 10); else full.
            let dev_speed = if st2 & (1 << 9) != 0 {
                2
            } else if st2 & (1 << 10) != 0 {
                3
            } else {
                1
            };
            // Append this downstream port to the route string at our tier.
            let route = parent_route | ((p as u32 & 0xf) << (4 * depth));
            // A full/low-speed device behind this high-speed hub uses *our* TT;
            // otherwise inherit whatever TT (if any) we sit behind.
            let (tt_slot, tt_port) = if dev_speed < 3 && speed == 3 {
                (hub_slot, p)
            } else {
                (parent_tt_slot, parent_tt_port)
            };
            self.enumerate_device(root_port, route, dev_speed, tt_slot, tt_port, depth + 1, di as u8);
        }
    }

    /// Mark a hub's device slot as a Hub via an **Evaluate Context** command
    /// with a **from-scratch** Input Slot Context (A0 only). This mirrors
    /// circle's `CXHCIUSBDevice::EnableHubFunction`
    /// (`references/circle/lib/usb/xhciusbdevice.cpp`), the working reference
    /// for this exact VL805: it sets `Slot.Hub`, `Slot.NumberOfPorts` and (for
    /// a high-speed hub) `Slot.TTT` and issues **Evaluate Context**, not
    /// Configure Endpoint. On this controller Configure Endpoint runs a strict
    /// Context-State precondition against the *output* device context, which the
    /// VL805 never writes coherently (the RAM slot context reads garbage —
    /// slot state 0 — even after a successful Address Device, the
    /// XHCI_EP_CTX_BROKEN_DCS quirk), so it always returned Context State Error.
    /// Evaluate Context does not run that check, so it succeeds. The VL805
    /// applies the Hub / Number-of-Ports / TT fields on Evaluate Context even
    /// though xHCI §4.6.7 nominally restricts it to Max Exit Latency /
    /// Interrupter Target — circle relies on exactly this behaviour.
    ///
    /// The RAM device context is garbage, so — unlike circle, which memcpys it
    /// as the input base — we rebuild the slot context from known-good values
    /// (route 0, the hub's speed + root port, context_entries = 1 for EP0). The
    /// controller keeps EP0 (dci 1) internally, so A0 alone (no endpoint
    /// contexts) is enough.
    fn configure_hub(&mut self, di: usize, speed: u8, nbr_ports: u8) -> bool {
        let slot = self.hid[di].slot;
        let port = self.hid[di].port;
        // SAFETY: single-CPU; DEV is ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        dev.input.iter_mut().for_each(|x| *x = 0);
        // SAFETY: `Input32Byte` is `#[repr(C)]` over the same u32s as `input`.
        let input: &mut context::Input32Byte =
            unsafe { &mut *(dev.input.as_mut_ptr().cast()) };
        input.control_mut().set_add_context_flag(0); // A0: slot context
        {
            let s = input.device_mut().slot_mut();
            s.set_context_entries(1);
            s.set_root_hub_port_number(port);
            s.set_speed(speed);
            s.set_hub();
            s.set_number_of_ports(nbr_ports);
            // A high-speed hub provides the Transaction Translator. Single-TT,
            // think time 0 (8 FS bit times) is the safe default.
            if speed == 3 {
                s.set_tt_think_time(0);
            }
        }
        let input_pa = dma_addr((&raw const dev.input) as u64);
        let mut cmd = command::EvaluateContext::new();
        cmd.set_input_context_pointer(input_pa);
        cmd.set_slot_id(slot);
        let data = command::Allowed::EvaluateContext(cmd).into_raw();
        match self.command(data) {
            Some(cc) if matches!(cc.completion_code(), Ok(event::CompletionCode::Success)) => true,
            Some(cc) => {
                kprintln!("[usb] configure_hub → {:?}", cc.completion_code());
                false
            }
            None => {
                kprintln!("[usb] configure_hub: no completion (timeout)");
                false
            }
        }
    }

    /// Power the hub's downstream ports and wait for the power to be good.
    ///
    /// 1:1 with FreeMiNT's `usb_hub_power_on` (`sys/usb/src.km/hub.c:139`):
    /// `SET_FEATURE(PORT_POWER)` on every port with `pgood_ms` after each, then
    /// `MAX(pgood_ms, 100)` ms for the power to stabilise. The delay is the
    /// hub's own `bPwrOn2PwrGood` (× 2 ms — `hub.c:463`), not a fixed number:
    /// with the previous hardcoded 100 ms a hub that states a longer power-good
    /// time had its ports read while they were still coming up.
    ///
    /// FreeMiNT's `usb_hub_configure` reaches this through
    /// `usb_hub_power_cycle` (`hub.c:463`), which first switches every port
    /// **off** and waits `2 × MAX(pgood, 100)` ms. We use the `power_on` half
    /// (the same function the reference calls at `hub.c:552`/`:582`): our hub
    /// bring-up runs once at boot on ports the hub already powers by default,
    /// and cutting their power would re-enumerate devices underneath us.
    fn hub_power_on(&mut self, di: usize, nbr_ports: u8, pgood_ms: u32) {
        #[cfg(feature = "usb-diag")]
        kprintln!(
            "[usb] hub slot {}: bPwrOn2PwrGood → {} ms power-good delay per port",
            self.hid[di].slot,
            pgood_ms
        );
        for p in 1..=nbr_ports {
            if !self.hub_set_port_feature(di, p, 8) {
                // FreeMiNT ignores the return value here and only logs the
                // resulting device status (`hub.c:147`). A port that refuses
                // PORT_POWER simply never reports a connection, so name it
                // instead of discarding the acknowledgement.
                kprintln!("[usb] hub port {}: SET_FEATURE(PORT_POWER) failed", p);
            }
            crate::arch::aarch64::delay_ms(pgood_ms as u64);
        }
        // "Wait at least 100 msec for power to become stable" (`hub.c:152`).
        crate::arch::aarch64::delay_ms(pgood_ms.max(100) as u64);
    }

    /// Hub class GET_DESCRIPTOR (type 0x29). Returns the fields the hub driver
    /// acts on: `bNbrPorts` and `bPwrOn2PwrGood`.
    fn hub_get_descriptor(&mut self, di: usize) -> Option<HubDescriptor> {
        // SAFETY: single-CPU; DEV is ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        // Zero the header first: the controller writes only as many bytes as
        // the device returns, so bytes left by the previous transfer must not
        // be able to stand in for a descriptor field.
        dev.buf[..8].iter_mut().for_each(|b| *b = 0);
        let buf_pa = dma_addr((&raw const dev.buf) as u64);
        // bmRequestType=0xA0 (in, class, device), bRequest=6, wValue=0x2900.
        // 8 bytes cover the fixed part through bHubContrCurrent; the variable
        // DeviceRemovable / PortPwrCtrlMask tail is not used here.
        if !self.control(di, [0xA0, 6, 0x00, 0x29, 0, 0, 0, 0], buf_pa, 8, true) {
            return None;
        }
        // SAFETY: single-CPU; DEV is ours. [0] bDescLength, [1] bDescriptorType,
        // [2] bNbrPorts, [5] bPwrOn2PwrGood (USB 2.0 §11.23.2.1; the SuperSpeed
        // hub descriptor 0x2A carries both fields at the same offsets).
        let b = unsafe { &(*(&raw const DEV))[di].buf };
        if (b[1] != 0x29 && b[1] != 0x2A) || b[0] < 8 {
            kprintln!(
                "[usb] hub descriptor malformed: bLength={} bDescriptorType={:#04x}",
                b[0],
                b[1]
            );
            return None;
        }
        Some(HubDescriptor {
            nbr_ports: b[2],
            pwr_on_2_pwr_good: b[5],
        })
    }

    /// Hub class SET_FEATURE on a downstream port (PORT_POWER=8, PORT_RESET=4).
    fn hub_set_port_feature(&mut self, di: usize, port: u8, feature: u8) -> bool {
        // bmRequestType=0x23 (out, class, other), bRequest=3 (SET_FEATURE).
        self.control(di, [0x23, 3, feature, 0, port, 0, 0, 0], 0, 0, false)
    }

    /// Hub class CLEAR_FEATURE on a downstream port (C_PORT_CONNECTION=16, …).
    fn hub_clear_port_feature(&mut self, di: usize, port: u8, feature: u8) -> bool {
        // bmRequestType=0x23 (out, class, other), bRequest=1 (CLEAR_FEATURE).
        self.control(di, [0x23, 1, feature, 0, port, 0, 0, 0], 0, 0, false)
    }

    /// Reset one downstream hub port; returns its status word once the port is
    /// enabled, or `None` when the port must be given up on.
    ///
    /// 1:1 with FreeMiNT's `hub_port_reset` (`sys/usb/src.km/hub.c:225`):
    /// up to `MAX_TRIES` = 5 attempts (`hub.c:201`), each
    /// `SET_FEATURE(PORT_RESET)` + `mdelay(150)` (`hub.c:233`) followed by a
    /// fresh `GET_STATUS`; a failed status read aborts (`hub.c:238`); a
    /// connection that changed or went away aborts (`hub.c:255`); the loop
    /// stops as soon as `USB_PORT_STAT_ENABLE` is set (`hub.c:259`); and
    /// `C_PORT_RESET` is cleared only after that success (`hub.c:269`).
    /// `docs/api/usb/hub.md` records the same shape ("poll GET_STATUS until
    /// reset completes + the port is enabled").
    ///
    /// This replaces both the old one-shot 60 ms wait — which dropped any port
    /// that needed longer — and the `usb-diag`-only 800 ms polling variant that
    /// existed to measure how long ports really need. That measurement has been
    /// taken (both hardware runs enabled at the first 10 ms sample), and the
    /// reference budget now applies to **both** builds: a deliberate change for
    /// the normal build, whose worst case per *connected* port rises from 60 ms
    /// to 5 × 150 ms. Empty ports are never reset, so an idle socket costs
    /// nothing. The `usb-diag` build adds only prints, no different timing.
    ///
    /// One deliberate deviation: the reference returns 0/-1 and its caller
    /// derives the device speed from the port status read *before* the reset
    /// (`hub.c:306`). We hand back the **post-reset** status word instead,
    /// because that is where the low/high-speed bits are valid — a hub only
    /// knows the attached device's speed after the reset handshake (USB 2.0
    /// §11.24.2.7.1) — and `enumerate_hub` needs the speed for Address Device.
    fn hub_port_reset(&mut self, di: usize, p: u8) -> Option<u32> {
        /// `MAX_TRIES` (`hub.c:201`).
        const MAX_TRIES: u32 = 5;
        /// `mdelay(150)` per attempt (`hub.c:234`).
        const TRY_MS: u32 = 150;
        for try_no in 1..=MAX_TRIES {
            if !self.hub_set_port_feature(di, p, 4) {
                // PORT_RESET. FreeMiNT ignores this return and lets the status
                // read below decide (`hub.c:233`); report it rather than
                // discard the acknowledgement.
                kprintln!("[usb] hub port {}: SET_FEATURE(PORT_RESET) failed", p);
            }
            crate::arch::aarch64::delay_ms(TRY_MS as u64);
            let Some(st) = self.hub_get_port_status(di, p) else {
                kprintln!("[usb] hub port {}: get_port_status failed during reset", p);
                return None;
            };
            #[cfg(feature = "usb-diag")]
            kprintln!(
                "[usb]   hub port {}: t={} ms status={:#06x} change={:#06x}",
                p,
                try_no * TRY_MS,
                st & 0xffff,
                st >> 16
            );
            // C_PORT_CONNECTION set again, or the device gone: the port changed
            // under us, so this reset is void (`hub.c:255`).
            if st & (1 << 16) != 0 || st & 0x1 == 0 {
                kprintln!(
                    "[usb] hub port {}: connection changed during reset (status={:#06x} \
                     change={:#06x})",
                    p,
                    st & 0xffff,
                    st >> 16
                );
                return None;
            }
            if st & 0x2 != 0 {
                // USB_PORT_STAT_ENABLE: the reset took. Acknowledge the reset
                // change only now, as the reference does (`hub.c:269`).
                if !self.hub_clear_port_feature(di, p, 20) {
                    kprintln!("[usb] hub port {}: CLEAR_FEATURE(C_PORT_RESET) failed", p);
                }
                if try_no > 1 {
                    kprintln!(
                        "[usb] hub port {}: enabled only on reset {} of {} ({} ms)",
                        p,
                        try_no,
                        MAX_TRIES,
                        try_no * TRY_MS
                    );
                }
                #[cfg(feature = "usb-diag")]
                kprintln!(
                    "[usb] hub port {}: ENABLED after {} ms (reset {} of {}, FreeMiNT \
                     hub.c:225 allows {}x{} ms)",
                    p,
                    try_no * TRY_MS,
                    try_no,
                    MAX_TRIES,
                    MAX_TRIES,
                    TRY_MS
                );
                return Some(st);
            }
        }
        kprintln!(
            "[usb] hub port {}: not enabled after {} resets of {} ms — giving up",
            p,
            MAX_TRIES,
            TRY_MS
        );
        None
    }

    /// Hub class GET_STATUS on a downstream port. Returns
    /// `wPortStatus | wPortChange << 16`, or `None` if the control transfer
    /// itself failed.
    ///
    /// The reference distinguishes those two cases — `usb_get_port_status(…) <
    /// 0` aborts a reset (`hub.c:238`) and skips a port (`hub.c:486`) — and an
    /// all-zero status word is otherwise indistinguishable from "nothing
    /// connected".
    fn hub_get_port_status(&mut self, di: usize, port: u8) -> Option<u32> {
        // SAFETY: single-CPU; DEV is ours.
        let dev = unsafe { &mut (*(&raw mut DEV))[di] };
        // Zero first: only the bytes the device actually returns are written.
        dev.buf[..4].iter_mut().for_each(|b| *b = 0);
        let buf_pa = dma_addr((&raw const dev.buf) as u64);
        // bmRequestType=0xA3 (in, class, other), bRequest=0 (GET_STATUS).
        if !self.control(di, [0xA3, 0, 0, 0, port, 0, 0, 0], buf_pa, 4, true) {
            return None;
        }
        // SAFETY: single-CPU; DEV is ours.
        let b = unsafe { &(*(&raw const DEV))[di].buf };
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Log each root-hub port's connect status (D1 verification target).
    pub fn scan_ports(&mut self) {
        for i in 0..self.ports as usize {
            let p = self.regs.port_register_set.read_volatile_at(i).portsc;
            // Diagnostic build: the fields the one-line summary leaves out.
            // `pls` (port link state) separates "nothing plugged in" from
            // "device present but the link never came up", and the change bits
            // say whether the controller saw an event we then ignored.
            #[cfg(feature = "usb-diag")]
            kprintln!(
                "[usb] port {} detail: pls={} pr={} csc={} pec={} prc={}",
                i + 1,
                p.port_link_state(),
                p.port_reset() as u8,
                p.connect_status_change() as u8,
                p.port_enabled_disabled_change() as u8,
                p.port_reset_change() as u8
            );
            kprintln!(
                "[usb] port {}: ccs={} ped={} pp={} speed={}",
                i + 1,
                p.current_connect_status() as u8,
                p.port_enabled_disabled() as u8,
                p.port_power() as u8,
                p.port_speed()
            );
        }
    }
}

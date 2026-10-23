//! USB host stack — keyboard + mouse input (PHASE-16).
//!
//! **Layer:** a kernel driver subsystem, the same layer as `bcm2711::emmc`
//! / `pl011` — *not* the BIOS/TOS-API. It feeds the existing IKBD decoder
//! ([`crate::ikbd::feed`]) → console RX ring + VDI cursor, which the BIOS
//! keyboard (`Bconin`) and the VDI mouse consume unchanged. (USB is to the
//! keyboard what `emmc` is to the disk.)
//!
//! **Runtime:** polled via `usb::poll()` from [`crate::host_env`]'s input
//! pump — `pump_input`, the blocking console reads and (INPUT01, unless
//! `input=aes`) the 200 Hz timer interrupt while a program runs at EL0
//! (`crate::input_gate`). No USB interrupt of its own; never entered twice
//! (the pump's one-at-a-time guard).
//!
//! **Build-up (QEMU-first):** D1 `xhci` + `pcie` (detect a device) → D2
//! `core` (enumerate) → D3 `hub` → D4/D5 `hid` (keyboard/mouse → `ikbd::feed`)
//! → D6 interactive desktop in QEMU → D7 real Pi-4 (VL805 firmware @
//! `0xFE9C0000`).
//!
//! Decision + scope: `docs/MEMORY.md` ADR-015. Spec extracted from FreeMiNT:
//! `docs/api/usb/` (this module mirrors its architecture, not its C ABI).

// Scaffold: the types + the host-controller contract are in place; the
// concrete modules (xhci, pcie, core, hub, hid) land per PHASE-16 D1..D7.
#![allow(dead_code)]

// Controller discovery is board-specific: QEMU virt = PCIe ECAM; the real
// Pi-4 (D7) = fixed MMIO + VL805 firmware.
#[cfg(feature = "board-qemu-virt")]
mod pcie;
#[cfg(feature = "board-rpi4")]
mod pcie_rpi4;
pub mod config_desc;
pub mod hid;
pub mod snapshot;
pub mod xhci;

const _: () = assert!(
    snapshot::MAX_DEVICES == xhci::MAX_HID,
    "the snapshot keeps one entry per driver device slot (usb-snapshot.md §2.7)"
);

/// The USB snapshot (`Ssystem(S_NEWTOS_USBSNAP)`, `docs/api/usb/usb-snapshot.md`):
/// filled once by the boot enumeration, the report rings by [`poll`]. Its own
/// static, apart from [`XHCI`], so the system call never touches the
/// controller object (§2.8: `poll` may be suspended under an EL0 upcall when
/// a `kbdvec` hook issues the call).
static mut SNAPSHOT: snapshot::Store = snapshot::Store::new();

/// The snapshot store.
///
/// # Safety
///
/// Single CPU. Callers must not hold the reference across anything that can
/// re-enter USB code (in `poll`: across the decoder, `ikbd::feed`, which may
/// call an EL0 `kbdvec` hook); the system call and `poll` never run at the
/// same time otherwise (§2.8).
unsafe fn store() -> &'static mut snapshot::Store {
    // SAFETY: caller contract — no overlapping reference is alive.
    unsafe { &mut *core::ptr::addr_of_mut!(SNAPSHOT) }
}

/// Run `f` on the snapshot store (the enumeration fills it, `xhci` counts
/// failed transfers). Single CPU; `f` must not re-enter USB code.
pub(crate) fn with_store<R>(f: impl FnOnce(&mut snapshot::Store) -> R) -> R {
    // SAFETY: the reference lives only for `f`, which does not re-enter.
    f(unsafe { store() })
}

/// Answer one `Ssystem(S_NEWTOS_USBSNAP)` request from the snapshot
/// (`host_env`'s `Host::usb_snapshot`). Runs in the synchronous SVC with IRQs
/// masked; never touches the controller.
pub fn snapshot_answer(
    req: snapshot::Request,
    sink: &mut dyn FnMut(&[u8]) -> bool,
) -> snapshot::Answer {
    with_store(|s| s.answer(req, sink))
}

// The live controller, kept for `poll`. It holds all per-device state
// (slots, endpoints, ring producers). Single CPU, cooperative scheduling —
// set once at boot, driven from `pump_input`.
#[cfg(any(feature = "board-qemu-virt", feature = "board-rpi4"))]
static mut XHCI: Option<xhci::Xhci> = None;

/// Bring up the USB host stack: find the controller (via the DTB-described
/// PCIe ECAM), start it, enumerate + configure the HID devices (keyboard +
/// mouse) and keep it live for `poll`. QEMU-virt for D1–D6; the real Pi-4
/// path (VL805 firmware + `0xFE9C0000`) is PHASE-16 D7.
pub fn init(dtb_phys: usize) {
    #[cfg(any(feature = "board-qemu-virt", feature = "board-rpi4"))]
    {
        // Board-specific controller discovery; the controller, enumeration and
        // HID code below are shared. QEMU = PCIe ECAM walk; real Pi-4 =
        // brcmstb PCIe RC bring-up + VL805 firmware (PHASE-16 D7).
        #[cfg(feature = "board-qemu-virt")]
        let base = unsafe { pcie::find_xhci(dtb_phys) };
        #[cfg(feature = "board-rpi4")]
        let base = pcie_rpi4::find_xhci(dtb_phys);

        let Some(base) = base else { return };
        // Real Pi-4 PCIe DMA is not cache-coherent: map the dedicated DMA
        // block Device-nGnRnE before the controller touches it.
        #[cfg(feature = "board-rpi4")]
        map_dma_coherent();
        let mut controller = unsafe { xhci::Xhci::open(base) };
        controller.scan_ports();
        controller.enumerate_all();
        // Kept even with no device attached (usb-snapshot.md §5 K5): the
        // snapshot can then tell "no controller" (`ERROR`) from "no device"
        // (`n_devices = 0`). `poll` then checks an empty event ring per pump.
        let root_ports = controller.root_ports();
        with_store(|s| {
            s.present = true;
            s.root_ports = root_ports;
            s.generation = 1;
        });
        // SAFETY: single-CPU boot; stored once.
        unsafe { XHCI = Some(controller) };
    }
    #[cfg(not(any(feature = "board-qemu-virt", feature = "board-rpi4")))]
    let _ = dtb_phys;
}

/// Remap the dedicated `.dma` linker block (a 2-MiB-aligned region holding all
/// xHCI DMA structures) as **Device-nGnRnE**, so the non-coherent Pi-4 PCIe
/// controller (VL805) and the CPU agree on memory. Normal-NC + manual cache
/// maintenance was not enough: Normal-NC still permits CPU gathering,
/// reordering and speculative reads, which corrupted our read-back of the
/// controller's device-context write-back (the EP0 context surfaced at the
/// slot offset, so Address Device "succeeded" but left the slot un-Addressed).
/// Device-nGnRnE forbids all of that — this is exactly how circle maps its
/// xHCI shared region. Its own block contains no kernel code/tables, so the
/// break-before-make remap is safe (unlike a remap of a shared block — see
/// ADR-014). Cleans+invalidates the bss-cleared zeros to RAM first, so no
/// stale cacheable alias survives the switch to Device memory.
#[cfg(feature = "board-rpi4")]
fn map_dma_coherent() {
    extern "C" {
        static __dma_start: u8;
        static __dma_end: u8;
    }
    let start = core::ptr::addr_of!(__dma_start) as u64;
    let end = core::ptr::addr_of!(__dma_end) as u64;
    // SAFETY: boot CPU; `.dma` is a dedicated block, safe to break-before-make.
    unsafe {
        let mut p = start & !63;
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
        while p < end {
            core::arch::asm!("dc civac, {x}", x = in(reg) p, options(nostack, preserves_flags));
            p += 64;
        }
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
        crate::arch::aarch64::mmu::remap_device_ngnrne(start, end - start);
    }
}

pub use xhci::DeviceInfo;

/// Snapshot the discovered USB devices into `buf` for display (the init-screen
/// USB tree). Returns `(root_port_count, n_devices)`; `(0, 0)` if no
/// controller enumerated. Safe to call after `init`.
pub fn tree(buf: &mut [DeviceInfo]) -> (u8, usize) {
    #[cfg(any(feature = "board-qemu-virt", feature = "board-rpi4"))]
    {
        // SAFETY: single-CPU; XHCI is set once at boot and read-only here.
        if let Some(c) = unsafe { (*core::ptr::addr_of!(XHCI)).as_ref() } {
            let n = c.n_hid().min(buf.len());
            for (di, slot) in buf.iter_mut().enumerate().take(n) {
                *slot = c.device_info(di);
            }
            return (c.root_ports(), n);
        }
    }
    let _ = buf;
    (0, 0)
}

/// Pump USB HID input: drain a completed interrupt transfer (a HID report),
/// translate it into the IKBD byte stream (`ikbd::feed`), and re-queue the
/// next transfer for that device. Called from `host_env`'s input pump
/// (`pump_devices`). A no-op until a HID device is enumerated.
pub fn poll() {
    #[cfg(any(feature = "board-qemu-virt", feature = "board-rpi4"))]
    {
        // SAFETY: single-CPU cooperative; XHCI is set once at boot.
        let Some(c) = (unsafe { (*core::ptr::addr_of_mut!(XHCI)).as_mut() }) else {
            return;
        };
        if let Some((di, report, len, cc)) = c.poll_interrupt() {
            // Record first, decode second (usb-snapshot.md §2.8, §3.3): the
            // decoder may run an EL0 `kbdvec` hook, which may read the ring
            // through `Ssystem(2022)` while this call is suspended under it —
            // it must find the report complete, and the store not borrowed.
            let now = tos_api::bios::tick::hz_200();
            let muted = with_store(|s| {
                let muted = s.muted(now);
                s.record(di, &report[..len], cc, now, muted);
                muted
            });
            let mut boot = [0u8; 8];
            boot.copy_from_slice(&report[..8]);
            // `bInterfaceProtocol` of the HID boot interface: 1 = keyboard,
            // 2 = mouse (USB HID 1.11 §4.3). Anything else has no boot report
            // layout we could decode — a hub is 9, a device whose HID
            // configuration never completed or that is only observed (role 3,
            // a report-protocol interface) is 0 — and used to be handed to
            // `mouse_report`, which turned whatever bytes were in the buffer
            // into cursor movement and button presses in the IKBD stream.
            // While the snapshot's MUTE runs nothing is decoded (§2.6).
            match c.protocol(di) {
                _ if muted => {}
                1 => hid::keyboard_report(&boot),
                // `len` — how many bytes the device really sent — matters only
                // for the mouse: the boot report's fourth byte (the wheel) is
                // optional, and a three-byte mouse never writes it.
                2 => hid::mouse_report(&boot, len),
                _ => {}
            }
            // Re-arm regardless: the completed transfer left the endpoint with
            // nothing outstanding, and only this call queues the next one.
            c.queue_interrupt(di);
        }
    }
}

/// USB transfer / stack error. Mirrors the failure modes the FreeMiNT core
/// distinguishes (timeout, stall, NAK …) plus our controller-discovery cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbError {
    /// No host controller found (PCIe ECAM miss on QEMU / no xHCI on Pi-4).
    NoController,
    /// Transfer timed out waiting on the event ring.
    Timeout,
    /// Endpoint returned STALL (protocol error / unsupported request).
    Stall,
    /// Device not responding / disconnected mid-transfer.
    NoDevice,
    /// Device sent more data than the buffer holds (babble / overrun).
    Overrun,
    /// A descriptor was malformed or shorter than required.
    BadDescriptor,
    /// The operation is not supported (e.g. non-boot HID before the parser).
    Unsupported,
}

/// USB endpoint transfer type (USB spec ch. 9; the low 2 bits of FreeMiNT's
/// pipe encoding, `docs/api/usb/host-controller-interface.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
}

/// A USB pipe: the endpoint a transfer targets. Carries the device address
/// so a transfer needs only the pipe (mirrors FreeMiNT's packed pipe).
#[derive(Debug, Clone, Copy)]
pub struct Pipe {
    /// Device address (0 during the first enumeration step, then assigned).
    pub dev_addr: u8,
    /// Endpoint number (0 = the default control endpoint).
    pub endpoint: u8,
    /// `true` = IN (device→host), `false` = OUT (host→device).
    pub dir_in: bool,
    pub ep_type: EndpointType,
    /// Endpoint max packet size (`bMaxPacketSize`).
    pub max_packet: u16,
}

/// The 8-byte USB SETUP packet (USB spec §9.3) carried by a control transfer.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SetupPacket {
    pub request_type: u8,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub length: u16,
}

/// Root-hub operations a controller answers directly (the FreeMiNT root-hub
/// `ioctl`s). Ports are 0-based.
#[derive(Debug, Clone, Copy)]
pub enum RootHubOp {
    /// Number of downstream ports the controller exposes.
    PortCount,
    /// Read a port's status.
    PortStatus(u8),
    /// Reset a port (and enable it).
    PortReset(u8),
    /// Power a port on/off.
    PortPower(u8, bool),
}

/// Per-port status from a [`RootHubOp`].
#[derive(Debug, Clone, Copy, Default)]
pub struct RootHubStatus {
    /// For `PortCount`: the port count. Otherwise unused.
    pub port_count: u8,
    pub connected: bool,
    pub enabled: bool,
    /// Connection-status-change bit (a device was just (un)plugged).
    pub connect_changed: bool,
    /// `0` low, `1` full, `2` high, `3` super speed.
    pub speed: u8,
}

/// A USB host controller. The Pi-4 **xHCI** driver (PHASE-16 D1) implements
/// this; the USB core / hub / HID layers consume it. Contract extracted from
/// FreeMiNT's `struct ucdif` — see
/// `docs/api/usb/host-controller-interface.md`. All transfers replace the
/// `SUBMIT_*_MSG` ioctls 1:1; `root_hub` replaces the root-hub ioctls.
pub trait HostController {
    /// Bring the controller up (reset, rings, slots, start). On real Pi-4
    /// the caller loads the VL805 firmware via the mailbox first.
    fn open(&mut self) -> Result<(), UsbError>;

    /// Shut the controller down.
    fn close(&mut self);

    /// Control transfer (enumeration). Returns the number of data bytes
    /// transferred.
    fn control(
        &mut self,
        pipe: Pipe,
        setup: &SetupPacket,
        data: &mut [u8],
    ) -> Result<usize, UsbError>;

    /// Interrupt transfer (HID keyboard/mouse reports).
    fn interrupt(&mut self, pipe: Pipe, data: &mut [u8]) -> Result<usize, UsbError>;

    /// Bulk transfer (mass storage later; unused by HID).
    fn bulk(&mut self, pipe: Pipe, data: &mut [u8]) -> Result<usize, UsbError>;

    /// Root-hub port management.
    fn root_hub(&mut self, op: RootHubOp) -> Result<RootHubStatus, UsbError>;
}

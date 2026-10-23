//! USB snapshot: what the kernel keeps about every enumerated USB device and
//! the encoder behind `Ssystem(S_NEWTOS_USBSNAP)` (2022) — USB01 step 2, K1.
//!
//! Spec: `docs/api/usb/usb-snapshot.md` §2 (the wire format: list entry,
//! device sections, report ring, mute), §2.8 (concurrency), §3.3 (the ring).
//!
//! Pure data and byte encoding with no register, DMA or kernel dependency, so
//! the host test suite (`tools/xtask`, which includes this file by path) runs
//! its tests, and the `tos-api` trap-boundary test drives the real encoder.
//! The kernel owns the one [`Store`] (`drivers::usb`), fills it while it
//! enumerates (`xhci`) and from `usb::poll` ([`Store::record`]), and answers
//! the system call through [`Store::answer`].
//!
//! Everything on the wire is little-endian with fixed field widths; the
//! layouts are written byte by byte here, never through a `repr(C)` struct.

/// Devices the store holds — the driver's `MAX_HID` (`xhci/mod.rs`; the
/// kernel asserts the two agree).
pub const MAX_DEVICES: usize = 8;
/// Bytes of the raw configuration descriptor kept (§2.5 type 2, §2.7).
pub const CONFIG_MAX: usize = 1024;
/// Bytes of one raw HID report descriptor kept (§2.5 type 3).
pub const REPORT_DESC_MAX: usize = 2048;
/// HID interfaces per device whose report descriptor is kept (§2.5 type 3).
pub const REPORT_DESC_IFACES: usize = 4;
/// Rows of the interface table (§2.5 type 4).
pub const IFACE_ROWS: usize = 16;
/// Entries of the per-device report ring (§2.6).
pub const RING_LEN: usize = 256;
/// Report bytes one ring entry holds (§2.6, §2.7).
pub const REPORT_MAX: usize = 64;
/// Characters of a kept string descriptor (§2.5 types 5/6).
pub const STRING_MAX: usize = 31;
/// The longest mute a caller may ask for, in `_hz_200` ticks (60 s, §2.3 #7).
pub const MUTE_MAX_TICKS: u32 = 12_000;

/// Layout version v1 (§2.2).
pub const VERSION: u16 = 1;
/// Request/response header size (§2.2).
pub const HEADER_LEN: usize = 32;
/// Size of one list entry (§2.4).
pub const ENTRY_LEN: usize = 32;
/// Size of one ring entry on the wire (§2.6).
pub const REC_LEN: usize = 80;
const LIST_HEAD_LEN: usize = 16;
const REPORTS_HEAD_LEN: usize = 16;
const SECTION_HEAD_LEN: usize = 8;
const ROW_LEN: usize = 16;

/// Op 1: device list (§2.4).
pub const OP_LIST: u16 = 1;
/// Op 2: one device in detail (§2.5).
pub const OP_DEVICE: u16 = 2;
/// Op 3: read a device's report ring (§2.6).
pub const OP_REPORTS: u16 = 3;
/// Op 4: mute the input decoder for a while (§2.6).
pub const OP_MUTE: u16 = 4;

/// Device roles (§2.4 `role`).
pub mod role {
    /// Enumerated, but not an input device / not configured.
    pub const NONE: u8 = 0;
    /// HID boot keyboard, decoded.
    pub const KEYBOARD: u8 = 1;
    /// HID boot mouse, decoded.
    pub const MOUSE: u8 = 2;
    /// HID interface configured and recorded, not decoded (§3.2).
    pub const OBSERVED: u8 = 3;
    /// Hub.
    pub const HUB: u8 = 4;
    /// Enumeration failed: the slot is taken, the device unusable.
    pub const FAILED: u8 = 5;
}

/// Reason codes (§2.4 `reason`). 1–5 are `config_desc::SkipReason` + 1.
pub mod reason {
    /// Used (roles 1/2/4), or: this interface feeds the driver/ring.
    pub const USED: u8 = 0;
    /// `bInterfaceClass` != 3.
    pub const NOT_HID: u8 = 1;
    /// HID without the boot subclass (report protocol only).
    pub const NOT_BOOT_SUBCLASS: u8 = 2;
    /// Boot subclass, protocol neither keyboard nor mouse.
    pub const UNKNOWN_BOOT_PROTOCOL: u8 = 3;
    /// HID boot interface without an interrupt-IN endpoint.
    pub const NO_INTERRUPT_IN: u8 = 4;
    /// Second HID boot interface: one interface per device.
    pub const ONE_INTERFACE_PER_DEVICE: u8 = 5;
    /// The device has no interface the driver can take.
    pub const NO_HID_INTERFACE: u8 = 16;
    /// The configuration descriptor could not be read.
    pub const CONFIG_READ: u8 = 17;
    /// Configure Endpoint failed.
    pub const CONFIGURE_ENDPOINT: u8 = 18;
    /// `SET_CONFIGURATION` failed.
    pub const SET_CONFIGURATION: u8 = 19;
    /// `SET_PROTOCOL(boot)` failed.
    pub const SET_PROTOCOL: u8 = 20;
    /// Address Device failed.
    pub const ADDRESS_DEVICE: u8 = 21;
    /// The device descriptor could not be read.
    pub const DEVICE_DESCRIPTOR: u8 = 22;
}

/// List-entry `flags` bits (§2.4 offset 16).
pub mod dev_flag {
    /// The device descriptor is valid.
    pub const DEVICE_DESC: u8 = 1 << 0;
    /// The configuration descriptor is valid.
    pub const CONFIG: u8 = 1 << 1;
    /// The configuration was longer than [`super::CONFIG_MAX`] and is cut.
    pub const CONFIG_CAPPED: u8 = 1 << 2;
    /// `SET_CONFIGURATION` was sent and acknowledged.
    pub const SET_CONFIGURATION: u8 = 1 << 3;
}

/// Response-header `flags` bits (§2.2).
pub const FLAG_TRUNCATED: u16 = 1 << 0;
/// More ring entries are available than were returned (§2.6).
pub const FLAG_MORE: u16 = 1 << 1;

const SEC_DEVICE: u16 = 1;
const SEC_CONFIG: u16 = 2;
const SEC_REPORT: u16 = 3;
const SEC_IFACES: u16 = 4;
const SEC_MANUFACTURER: u16 = 5;
const SEC_PRODUCT: u16 = 6;
const SECF_TRUNCATED: u8 = 1 << 0;
const SECF_FAILED: u8 = 1 << 1;
const SECF_SKIPPED: u8 = 1 << 2;

/// Ring-entry `flags` (§2.6): the device may have sent more than 64 bytes.
pub const REC_CAPPED: u8 = 1 << 0;
/// Ring-entry `flags`: collected while the decoder was muted.
pub const REC_MUTED: u8 = 1 << 1;

/// What became of one descriptor fetch.
///
/// `repr(u8)` with `NotTried = 0`: an all-zero [`Device`] (the unclaimed
/// slots of [`Store::new`]) must be all-zero bytes so the kernel's store
/// lands in `.bss`, not in the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Fetch {
    /// Not tried (§2.5 `SKIPPED`).
    NotTried = 0,
    /// Read completely.
    Ok,
    /// Read, but longer than what is kept (§2.5 `TRUNCATED`).
    Truncated,
    /// Tried and failed (§2.5 `FAILED`).
    Failed,
}

impl Fetch {
    fn section_flags(self) -> u8 {
        match self {
            Fetch::NotTried => SECF_SKIPPED,
            Fetch::Ok => 0,
            Fetch::Truncated => SECF_TRUNCATED,
            Fetch::Failed => SECF_FAILED,
        }
    }
}

/// One row of the interface table (§2.5 type 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IfaceRow {
    /// `bInterfaceNumber`.
    pub number: u8,
    /// `bAlternateSetting`.
    pub alt: u8,
    /// `bInterfaceClass`.
    pub class: u8,
    /// `bInterfaceSubClass`.
    pub subclass: u8,
    /// `bInterfaceProtocol`.
    pub protocol: u8,
    /// Reason code (§2.4); 0 = this interface feeds the driver/ring.
    pub reason: u8,
    /// First interrupt-IN endpoint, 0 = none.
    pub ep_addr: u8,
    /// Its `bInterval`.
    pub interval: u8,
    /// Its `wMaxPacketSize`, raw.
    pub max_packet: u16,
    /// `wDescriptorLength` of the report descriptor (HID descriptor), 0 = none.
    pub report_len: u16,
    /// This interface fills the ring.
    pub monitored: bool,
}

impl IfaceRow {
    /// An all-zero row.
    pub const EMPTY: IfaceRow = IfaceRow {
        number: 0,
        alt: 0,
        class: 0,
        subclass: 0,
        protocol: 0,
        reason: 0,
        ep_addr: 0,
        interval: 0,
        max_packet: 0,
        report_len: 0,
        monitored: false,
    };

    fn is_hid_alt0(&self) -> bool {
        self.class == 3 && self.alt == 0
    }
}

/// One kept HID report descriptor (§2.5 type 3).
pub struct ReportDesc {
    /// Outcome of the fetch.
    pub state: Fetch,
    /// Bytes kept in `bytes`.
    pub len: u16,
    /// The raw descriptor.
    pub bytes: [u8; REPORT_DESC_MAX],
}

impl ReportDesc {
    const EMPTY: ReportDesc =
        ReportDesc { state: Fetch::NotTried, len: 0, bytes: [0; REPORT_DESC_MAX] };
}

/// One kept string descriptor, converted to ASCII (§2.5 types 5/6, §2.7).
#[derive(Clone, Copy)]
pub struct Text {
    /// The device descriptor names a string (`iManufacturer`/`iProduct` != 0).
    pub present: bool,
    /// Outcome of the fetch.
    pub state: Fetch,
    /// Characters kept.
    pub len: u8,
    /// ASCII; a non-ASCII code unit is `?`.
    pub bytes: [u8; STRING_MAX],
}

impl Text {
    const EMPTY: Text = Text { present: false, state: Fetch::NotTried, len: 0, bytes: [0; STRING_MAX] };

    /// Fill from a raw string descriptor (`bLength`, type 3, UTF-16LE code
    /// units). `raw` = the bytes received.
    pub fn set_from_descriptor(&mut self, raw: &[u8]) {
        let blen = raw.first().copied().unwrap_or(0) as usize;
        let end = blen.min(raw.len());
        let units = end.saturating_sub(2) / 2;
        let n = units.min(STRING_MAX);
        for k in 0..n {
            let u = u16::from_le_bytes([raw[2 + 2 * k], raw[3 + 2 * k]]);
            self.bytes[k] = if (0x20..0x7f).contains(&u) { u as u8 } else { b'?' };
        }
        self.len = n as u8;
        self.state = if units > STRING_MAX || blen > raw.len() { Fetch::Truncated } else { Fetch::Ok };
    }
}

/// One recorded interrupt report (§2.6).
#[derive(Clone, Copy)]
struct Record {
    ts: u32,
    len: u8,
    flags: u8,
    cc: u8,
    data: [u8; REPORT_MAX],
}

impl Record {
    const EMPTY: Record = Record { ts: 0, len: 0, flags: 0, cc: 0, data: [0; REPORT_MAX] };
}

/// Everything kept about one device slot (`hid[di]` of the driver).
pub struct Device {
    /// xHCI slot id.
    pub slot: u8,
    /// Root port, 1-based.
    pub root_port: u8,
    /// Index of the hub entry it hangs off, `0xFF` = a root port.
    pub parent: u8,
    /// xHCI route string.
    pub route: u32,
    /// `idVendor`.
    pub vid: u16,
    /// `idProduct`.
    pub pid: u16,
    /// `bDeviceClass`.
    pub dev_class: u8,
    /// PORTSC speed id (1 full, 2 low, 3 high, 4 super).
    pub speed: u8,
    /// [`role`].
    pub role: u8,
    /// [`reason`].
    pub reason: u8,
    /// [`dev_flag`] bits.
    pub flags: u8,
    /// HID interfaces (class 3, alternate setting 0).
    pub n_hid_ifaces: u8,
    /// Interface whose endpoint fills the ring, `0xFF` = no ring.
    pub ring_iface: u8,
    /// That endpoint's address, 0 = none.
    pub ring_ep: u8,
    /// That endpoint's max packet size exceeds [`REPORT_MAX`].
    pub ring_capped: bool,
    /// Outcome of the device-descriptor read.
    pub device_desc_state: Fetch,
    /// The raw device descriptor.
    pub device_desc: [u8; 18],
    /// Bytes of `config` kept (0 = no configuration section).
    pub config_len: u16,
    /// The raw configuration descriptor.
    pub config: [u8; CONFIG_MAX],
    /// The interface table, `ifaces[..n_ifaces]`.
    pub ifaces: [IfaceRow; IFACE_ROWS],
    /// Rows filled.
    pub n_ifaces: u8,
    /// More interface descriptors than [`IFACE_ROWS`].
    pub ifaces_cut: bool,
    /// Report descriptors of the first [`REPORT_DESC_IFACES`] HID interfaces
    /// (alternate setting 0, table order).
    pub reports: [ReportDesc; REPORT_DESC_IFACES],
    /// Manufacturer string.
    pub manufacturer: Text,
    /// Product string.
    pub product: Text,
    ring: [Record; RING_LEN],
    next_seq: u32,
    filled: u16,
    int_errors: u16,
    last_cc: u8,
}

impl Device {
    const EMPTY: Device = Device {
        slot: 0,
        root_port: 0,
        parent: 0xFF,
        route: 0,
        vid: 0,
        pid: 0,
        dev_class: 0,
        speed: 0,
        role: role::NONE,
        reason: reason::USED,
        flags: 0,
        n_hid_ifaces: 0,
        ring_iface: 0xFF,
        ring_ep: 0,
        ring_capped: false,
        device_desc_state: Fetch::NotTried,
        device_desc: [0; 18],
        config_len: 0,
        config: [0; CONFIG_MAX],
        ifaces: [IfaceRow::EMPTY; IFACE_ROWS],
        n_ifaces: 0,
        ifaces_cut: false,
        reports: [ReportDesc::EMPTY; REPORT_DESC_IFACES],
        manufacturer: Text::EMPTY,
        product: Text::EMPTY,
        ring: [Record::EMPTY; RING_LEN],
        next_seq: 0,
        filled: 0,
        int_errors: 0,
        last_cc: 0,
    };

    /// A slot no device has claimed yet: every byte zero (the `0xFF`
    /// "none" markers of [`Device::EMPTY`] are set by [`Store::claim`]).
    /// Only `devices[..n_devices]` is ever read, and `claim` resets a slot
    /// to [`Device::EMPTY`] before it counts, so an unclaimed slot is never
    /// observed. Being all-zero keeps the 240-KiB store in `.bss`.
    const UNCLAIMED: Device = Device { parent: 0, ring_iface: 0, ..Device::EMPTY };

    fn oldest_seq(&self) -> u32 {
        self.next_seq.wrapping_sub(self.filled as u32)
    }

    fn has_ring(&self) -> bool {
        self.ring_iface != 0xFF
    }

    /// The table row of the `k`-th HID interface in alternate setting 0.
    fn hid_row_slot(&self, row: usize) -> Option<usize> {
        let k = self.ifaces[..row].iter().filter(|r| r.is_hid_alt0()).count();
        (self.ifaces[row].is_hid_alt0() && k < REPORT_DESC_IFACES).then_some(k)
    }

    /// The 32-byte list entry (§2.4), also the head of op 2.
    fn entry(&self, index: u8) -> [u8; ENTRY_LEN] {
        let mut e = [0u8; ENTRY_LEN];
        e[0] = index;
        e[1] = self.slot;
        e[2] = self.root_port;
        e[3] = self.parent;
        e[4..8].copy_from_slice(&self.route.to_le_bytes());
        e[8..10].copy_from_slice(&self.vid.to_le_bytes());
        e[10..12].copy_from_slice(&self.pid.to_le_bytes());
        e[12] = self.dev_class;
        e[13] = self.speed;
        e[14] = self.role;
        e[15] = self.reason;
        e[16] = self.flags;
        e[17] = self.n_hid_ifaces;
        e[18] = self.ring_iface;
        e[19] = self.ring_ep;
        let (oldest, next) = if self.has_ring() { (self.oldest_seq(), self.next_seq) } else { (0, 0) };
        e[20..24].copy_from_slice(&oldest.to_le_bytes());
        e[24..28].copy_from_slice(&next.to_le_bytes());
        e[28..30].copy_from_slice(&self.int_errors.to_le_bytes());
        e[30] = self.last_cc;
        e
    }

    /// Call `f(type, iface, flags, payload)` for every op-2 section, in the
    /// stream order 1, 2, 4, 3…, 5, 6 (§2.5).
    fn sections(&self, f: &mut SectionFn<'_>) {
        match self.device_desc_state {
            Fetch::Ok | Fetch::Truncated => f(SEC_DEVICE, 0, 0, &self.device_desc),
            s => f(SEC_DEVICE, 0, s.section_flags(), &[]),
        }
        if self.flags & dev_flag::CONFIG == 0 {
            // No configuration readable (or a hub): the configuration, the
            // interface table and the report descriptors simply are not there.
        } else {
            let cflags = if self.flags & dev_flag::CONFIG_CAPPED != 0 { SECF_TRUNCATED } else { 0 };
            f(SEC_CONFIG, 0, cflags, &self.config[..self.config_len as usize]);
            let mut table = [0u8; ROW_LEN * IFACE_ROWS];
            let n = self.n_ifaces as usize;
            for (r, row) in self.ifaces[..n].iter().enumerate() {
                let o = &mut table[r * ROW_LEN..(r + 1) * ROW_LEN];
                o[0] = row.number;
                o[1] = row.alt;
                o[2] = row.class;
                o[3] = row.subclass;
                o[4] = row.protocol;
                o[5] = row.reason;
                o[6] = row.ep_addr;
                o[7] = row.interval;
                o[8..10].copy_from_slice(&row.max_packet.to_le_bytes());
                o[10..12].copy_from_slice(&row.report_len.to_le_bytes());
                let stored = self.hid_row_slot(r).map_or(0, |k| self.reports[k].len);
                o[12..14].copy_from_slice(&stored.to_le_bytes());
                o[14] = row.monitored as u8;
            }
            let tflags = if self.ifaces_cut { SECF_TRUNCATED } else { 0 };
            f(SEC_IFACES, 0, tflags, &table[..n * ROW_LEN]);
            for (r, row) in self.ifaces[..n].iter().enumerate() {
                if !row.is_hid_alt0() {
                    continue;
                }
                match self.hid_row_slot(r) {
                    Some(k) => {
                        let rd = &self.reports[k];
                        let payload = match rd.state {
                            Fetch::Ok | Fetch::Truncated => &rd.bytes[..rd.len as usize],
                            _ => &[][..],
                        };
                        f(SEC_REPORT, row.number, rd.state.section_flags(), payload);
                    }
                    None => f(SEC_REPORT, row.number, SECF_SKIPPED, &[]),
                }
            }
        }
        for (ty, t) in [(SEC_MANUFACTURER, &self.manufacturer), (SEC_PRODUCT, &self.product)] {
            if !t.present {
                continue;
            }
            match t.state {
                Fetch::Ok | Fetch::Truncated => f(ty, 0, t.state.section_flags(), &t.bytes[..t.len as usize]),
                s => f(ty, 0, s.section_flags(), &[]),
            }
        }
    }
}

/// The kernel's USB snapshot: controller facts plus one [`Device`] per slot.
pub struct Store {
    /// A USB controller was found and brought up.
    pub present: bool,
    /// Root ports of the controller.
    pub root_ports: u8,
    /// Enumeration generation (v1: 1 after the boot enumeration).
    pub generation: u32,
    /// Devices that were not enumerated (F8 events).
    pub enum_skipped: u16,
    /// Slots in use, `devices[..n_devices]`.
    pub n_devices: u8,
    /// The devices, indexed like the driver's `hid[]`.
    pub devices: [Device; MAX_DEVICES],
    mute_active: bool,
    mute_until: u32,
    /// The decoder has seen the current mute and released the held input
    /// ([`Store::mute_state`]).
    mute_seen: bool,
}

/// One `Ssystem(2022)` request after the header checks of §2.3 #1–#4.
#[derive(Clone, Copy, Debug)]
pub struct Request {
    /// [`OP_LIST`] … [`OP_MUTE`].
    pub op: u16,
    /// Device index (ops 2, 3; must be 0 for op 4).
    pub index: u32,
    /// Op 3: first wanted sequence number; op 4: ticks.
    pub arg: u32,
    /// `_hz_200` now.
    pub now: u32,
    /// Bytes the caller's buffer holds behind the 32-byte header.
    pub room: u32,
}

/// How a request was answered (the trap maps it onto §2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// No usable controller (§2.3 #5): `ERROR`, nothing written.
    NoController,
    /// Index or argument refused (§2.3 #6/#7): `EINVAL`, header only.
    Invalid,
    /// Answered: `total` body bytes in the complete answer, `written` of them
    /// handed to the sink.
    Done {
        /// Size of the complete body.
        total: u32,
        /// Bytes passed to the sink.
        written: u32,
        /// Nothing but the header (§2.5 "zu kleiner Puffer").
        truncated: bool,
        /// Op 3 returned fewer entries than available.
        more: bool,
    },
    /// The sink refused a chunk (the caller's buffer faulted).
    Faulted,
}

/// The visitor [`Device::sections`] calls: `(type, iface, flags, payload)`.
type SectionFn<'a> = dyn FnMut(u16, u8, u8, &[u8]) + 'a;

/// Byte sink of the two-pass encoder: counting, or forwarding to the caller.
trait Emit {
    fn put(&mut self, b: &[u8]);
}

struct Count(u32);
impl Emit for Count {
    fn put(&mut self, b: &[u8]) {
        self.0 += b.len() as u32;
    }
}

struct Forward<'a> {
    sink: &'a mut dyn FnMut(&[u8]) -> bool,
    n: u32,
    ok: bool,
}
impl Emit for Forward<'_> {
    fn put(&mut self, b: &[u8]) {
        if self.ok && !b.is_empty() {
            self.ok = (self.sink)(b);
            self.n += b.len() as u32;
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    /// An empty store: no controller, no devices.
    pub const fn new() -> Self {
        Store {
            present: false,
            root_ports: 0,
            generation: 0,
            enum_skipped: 0,
            n_devices: 0,
            devices: [Device::UNCLAIMED; MAX_DEVICES],
            mute_active: false,
            mute_until: 0,
            mute_seen: false,
        }
    }

    /// Start the entry of device slot `di` afresh (the driver claims `hid[di]`).
    pub fn claim(&mut self, di: usize) -> Option<&mut Device> {
        let d = self.devices.get_mut(di)?;
        *d = Device::EMPTY;
        self.n_devices = self.n_devices.max(di as u8 + 1);
        Some(d)
    }

    /// The entry of device slot `di`.
    pub fn device_mut(&mut self, di: usize) -> Option<&mut Device> {
        if di < self.n_devices as usize {
            self.devices.get_mut(di)
        } else {
            None
        }
    }

    /// Append one received report to `di`'s ring (§2.6, §3.3). `data` beyond
    /// [`REPORT_MAX`] bytes is cut; `cc` = the transfer's completion code.
    /// The entry is complete before `next_seq` moves (§2.8: a reader that
    /// runs in an upcall from the decoder sees whole entries only).
    pub fn record(&mut self, di: usize, data: &[u8], cc: u8, now: u32, muted: bool) {
        let Some(d) = self.device_mut(di) else { return };
        if !d.has_ring() {
            return;
        }
        let n = data.len().min(REPORT_MAX);
        let slot = &mut d.ring[d.next_seq as usize % RING_LEN];
        slot.ts = now;
        slot.len = n as u8;
        slot.cc = cc;
        slot.flags = (if d.ring_capped || data.len() > REPORT_MAX { REC_CAPPED } else { 0 })
            | (if muted { REC_MUTED } else { 0 });
        slot.data = [0; REPORT_MAX];
        slot.data[..n].copy_from_slice(&data[..n]);
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::Release);
        d.next_seq = d.next_seq.wrapping_add(1);
        if (d.filled as usize) < RING_LEN {
            d.filled += 1;
        }
    }

    /// Count one failed interrupt transfer of `di` (completion code `cc`).
    pub fn note_error(&mut self, di: usize, cc: u8) {
        if let Some(d) = self.device_mut(di) {
            d.int_errors = d.int_errors.saturating_add(1);
            d.last_cc = cc;
        }
    }

    /// Whether the input decoder is muted at `now`. A mute whose deadline
    /// has passed ends here (wrap-safe), so a crashed caller cannot keep the
    /// input off (§2.6).
    pub fn muted(&mut self, now: u32) -> bool {
        if self.mute_active && (self.mute_until.wrapping_sub(now) as i32) <= 0 {
            self.mute_active = false;
        }
        self.mute_active
    }

    /// The decoder's view of the mute at `now`: `(muted, entered)`, where
    /// `entered` is true exactly once per mute — on the first call that sees
    /// it. `usb::poll` then releases every key and button the decoder still
    /// holds (`hid::release_all`): a key pressed before the mute (the RETURN
    /// that starts USBSNAP's recording) would otherwise lose its break to the
    /// mute and auto-repeat for the whole mute and after it (usb-snapshot.md
    /// §9.1, found by T6). Extending a running mute is not a new mute.
    pub fn mute_state(&mut self, now: u32) -> (bool, bool) {
        let muted = self.muted(now);
        let entered = muted && !self.mute_seen;
        self.mute_seen = muted;
        (muted, entered)
    }

    /// Mute for `ticks` from `now` (0 = off). Returns the deadline, 0 = off.
    /// A deadline that wraps to exactly 0 is moved one tick later so that 0
    /// keeps meaning "off" on the wire.
    pub fn set_mute(&mut self, now: u32, ticks: u32) -> u32 {
        if ticks == 0 {
            self.mute_active = false;
            return 0;
        }
        let until = match now.wrapping_add(ticks) {
            0 => 1,
            t => t,
        };
        self.mute_active = true;
        self.mute_until = until;
        until
    }

    /// Answer one request (§2.3 #5–#9, §2.4–§2.6). Body bytes go to `sink`
    /// in order, in chunks; the caller writes the header.
    pub fn answer(&mut self, req: Request, sink: &mut dyn FnMut(&[u8]) -> bool) -> Answer {
        if !self.present {
            return Answer::NoController;
        }
        let n = self.n_devices as u32;
        match req.op {
            OP_LIST => self.whole(req.room, sink, |s, e| s.list_body(e)),
            OP_DEVICE if req.index < n => {
                let di = req.index as usize;
                self.whole(req.room, sink, |s, e| s.device_body(di, e))
            }
            OP_REPORTS if req.index < n => self.reports(req, sink),
            OP_MUTE if req.arg <= MUTE_MAX_TICKS && req.index == 0 => {
                let until = self.set_mute(req.now, req.arg);
                let mut body = [0u8; 8];
                body[..4].copy_from_slice(&until.to_le_bytes());
                self.whole(req.room, sink, |_, e| e.put(&body))
            }
            _ => Answer::Invalid,
        }
    }

    /// An all-or-nothing answer (ops 1, 2, 4).
    fn whole(
        &self,
        room: u32,
        sink: &mut dyn FnMut(&[u8]) -> bool,
        body: impl Fn(&Self, &mut dyn Emit),
    ) -> Answer {
        let mut c = Count(0);
        body(self, &mut c);
        let total = c.0;
        if total > room {
            return Answer::Done { total, written: 0, truncated: true, more: false };
        }
        let mut f = Forward { sink, n: 0, ok: true };
        body(self, &mut f);
        if !f.ok {
            return Answer::Faulted;
        }
        Answer::Done { total, written: f.n, truncated: false, more: false }
    }

    fn list_body(&self, e: &mut dyn Emit) {
        let mut h = [0u8; LIST_HEAD_LEN];
        h[0..4].copy_from_slice(&self.generation.to_le_bytes());
        h[4..6].copy_from_slice(&(self.n_devices as u16).to_le_bytes());
        h[6..8].copy_from_slice(&(ENTRY_LEN as u16).to_le_bytes());
        h[8] = self.root_ports;
        h[9] = 1; // controller_state: xHCI running
        h[10..12].copy_from_slice(&self.enum_skipped.to_le_bytes());
        e.put(&h);
        for (i, d) in self.devices[..self.n_devices as usize].iter().enumerate() {
            e.put(&d.entry(i as u8));
        }
    }

    fn device_body(&self, di: usize, e: &mut dyn Emit) {
        let d = &self.devices[di];
        e.put(&d.entry(di as u8));
        let mut n_sections = 0u16;
        d.sections(&mut |_, _, _, _| n_sections += 1);
        let mut h = [0u8; 4];
        h[0..2].copy_from_slice(&n_sections.to_le_bytes());
        e.put(&h);
        d.sections(&mut |ty, iface, flags, payload| {
            let mut s = [0u8; SECTION_HEAD_LEN];
            s[0..2].copy_from_slice(&ty.to_le_bytes());
            s[2] = iface;
            s[3] = flags;
            s[4..8].copy_from_slice(&(payload.len() as u32).to_le_bytes());
            e.put(&s);
            e.put(payload);
            let pad = (4 - payload.len() % 4) % 4;
            e.put(&[0u8; 3][..pad]);
        });
    }

    /// Op 3: whole entries, as many as fit (§2.6).
    fn reports(&self, req: Request, sink: &mut dyn FnMut(&[u8]) -> bool) -> Answer {
        let d = &self.devices[req.index as usize];
        let (oldest, next) = if d.has_ring() { (d.oldest_seq(), d.next_seq) } else { (0, 0) };
        let arg = req.arg;
        // Wrap-safe "a < b" on sequence numbers.
        let before = |a: u32, b: u32| (a.wrapping_sub(b) as i32) < 0;
        let lost = if d.has_ring() && before(arg, oldest) { oldest.wrapping_sub(arg) } else { 0 };
        let start = if lost > 0 { oldest } else { arg };
        let avail = if d.has_ring() && before(start, next) { next.wrapping_sub(start) } else { 0 };
        let total = REPORTS_HEAD_LEN as u32 + avail * REC_LEN as u32;
        if req.room < REPORTS_HEAD_LEN as u32 {
            return Answer::Done { total, written: 0, truncated: true, more: false };
        }
        let fit = avail.min((req.room - REPORTS_HEAD_LEN as u32) / REC_LEN as u32);
        let mut f = Forward { sink, n: 0, ok: true };
        let mut h = [0u8; REPORTS_HEAD_LEN];
        h[0..4].copy_from_slice(&oldest.to_le_bytes());
        h[4..8].copy_from_slice(&next.to_le_bytes());
        h[8..12].copy_from_slice(&lost.to_le_bytes());
        h[12..14].copy_from_slice(&(fit as u16).to_le_bytes());
        h[14..16].copy_from_slice(&(REC_LEN as u16).to_le_bytes());
        f.put(&h);
        for k in 0..fit {
            let seq = start.wrapping_add(k);
            let r = &d.ring[seq as usize % RING_LEN];
            let mut w = [0u8; REC_LEN];
            w[0..4].copy_from_slice(&seq.to_le_bytes());
            w[4..8].copy_from_slice(&r.ts.to_le_bytes());
            w[8..10].copy_from_slice(&(r.len as u16).to_le_bytes());
            w[10] = r.flags;
            w[11] = r.cc;
            w[16..16 + REPORT_MAX].copy_from_slice(&r.data);
            f.put(&w);
        }
        if !f.ok {
            return Answer::Faulted;
        }
        Answer::Done { total, written: f.n, truncated: false, more: fit < avail }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(s: &mut Store, op: u16, index: u32, arg: u32, now: u32, room: u32) -> (Answer, Vec<u8>) {
        let mut out = Vec::new();
        let a = s.answer(Request { op, index, arg, now, room }, &mut |b| {
            out.extend_from_slice(b);
            true
        });
        (a, out)
    }

    fn u16le(b: &[u8], o: usize) -> u16 {
        u16::from_le_bytes([b[o], b[o + 1]])
    }
    fn u32le(b: &[u8], o: usize) -> u32 {
        u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
    }

    /// The QEMU picture: a keyboard and a mouse on root ports 1/2, plus a hub
    /// with a report-only device behind it.
    fn store() -> Box<Store> {
        let mut s = Box::new(Store::new());
        s.present = true;
        s.root_ports = 4;
        s.generation = 1;
        s.enum_skipped = 1;
        {
            let d = s.claim(0).unwrap();
            d.slot = 1;
            d.root_port = 1;
            d.vid = 0x0627;
            d.pid = 0x0001;
            d.speed = 3;
            d.role = role::KEYBOARD;
            d.flags = dev_flag::DEVICE_DESC | dev_flag::CONFIG | dev_flag::SET_CONFIGURATION;
            d.n_hid_ifaces = 1;
            d.ring_iface = 0;
            d.ring_ep = 0x81;
            d.device_desc_state = Fetch::Ok;
            d.device_desc = [18, 1, 0, 2, 0, 0, 0, 64, 0x27, 0x06, 1, 0, 0, 0, 1, 2, 0, 1];
            let cfg = [9u8, 2, 34, 0, 1, 1, 0, 0xa0, 50, 9, 4, 0, 0, 1, 3, 1, 1, 0];
            d.config[..cfg.len()].copy_from_slice(&cfg);
            d.config_len = cfg.len() as u16;
            d.ifaces[0] = IfaceRow {
                number: 0,
                class: 3,
                subclass: 1,
                protocol: 1,
                ep_addr: 0x81,
                interval: 7,
                max_packet: 8,
                report_len: 63,
                monitored: true,
                ..IfaceRow::EMPTY
            };
            d.n_ifaces = 1;
            d.reports[0].state = Fetch::Ok;
            d.reports[0].len = 5;
            d.reports[0].bytes[..5].copy_from_slice(&[0x05, 0x01, 0x09, 0x06, 0xa1]);
            d.manufacturer.present = true;
            d.manufacturer.set_from_descriptor(&[8, 3, b'Q', 0, b'E', 0, 0xE9, 0]);
            d.product.present = true;
            d.product.state = Fetch::Failed;
        }
        {
            let d = s.claim(1).unwrap();
            d.slot = 2;
            d.root_port = 2;
            d.role = role::HUB;
            d.dev_class = 9;
            d.flags = dev_flag::DEVICE_DESC;
            d.device_desc_state = Fetch::Ok;
        }
        {
            let d = s.claim(2).unwrap();
            d.slot = 3;
            d.root_port = 2;
            d.parent = 1;
            d.route = 0x3;
            d.role = role::OBSERVED;
            d.reason = reason::NOT_BOOT_SUBCLASS;
            d.ring_iface = 1;
            d.ring_ep = 0x82;
            d.ring_capped = true;
        }
        s
    }

    #[test]
    fn no_controller_answers_nothing() {
        let mut s = Store::new();
        let (a, out) = ask(&mut s, OP_LIST, 0, 0, 0, 4096);
        assert_eq!(a, Answer::NoController);
        assert!(out.is_empty());
    }

    #[test]
    fn controller_without_devices_lists_zero() {
        let mut s = Box::new(Store::new());
        s.present = true;
        s.root_ports = 5;
        s.generation = 1;
        let (a, out) = ask(&mut s, OP_LIST, 0, 0, 0, 16);
        assert_eq!(a, Answer::Done { total: 16, written: 16, truncated: false, more: false });
        assert_eq!(out, [1, 0, 0, 0, 0, 0, 32, 0, 5, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(ask(&mut s, OP_DEVICE, 0, 0, 0, 4096).0, Answer::Invalid);
    }

    /// T1: LIST byte for byte.
    #[test]
    fn list_golden() {
        let mut s = store();
        s.record(0, &[1, 2, 3], 13, 100, false);
        s.note_error(2, 3);
        let (a, out) = ask(&mut s, OP_LIST, 0, 0, 0, 4096);
        assert_eq!(a, Answer::Done { total: 16 + 96, written: 112, truncated: false, more: false });
        assert_eq!(&out[..16], &[1, 0, 0, 0, 3, 0, 32, 0, 4, 1, 1, 0, 0, 0, 0, 0]);
        #[rustfmt::skip]
        let e0 = [0u8, 1, 1, 0xFF, 0, 0, 0, 0, 0x27, 0x06, 1, 0, 0, 3, 1, 0,
                  0x0B, 1, 0, 0x81, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(&out[16..48], &e0);
        #[rustfmt::skip]
        let e1 = [1u8, 2, 2, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 4, 0,
                  1, 0, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(&out[48..80], &e1);
        #[rustfmt::skip]
        let e2 = [2u8, 3, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 2,
                  0, 0, 1, 0x82, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 3, 0];
        assert_eq!(&out[80..112], &e2);
    }

    /// T1: DEVICE byte for byte — entry, sections 1, 2, 4, 3, 5, 6 with
    /// padding; a hub has only its device descriptor.
    #[test]
    fn device_golden() {
        let mut s = store();
        let (a, out) = ask(&mut s, OP_DEVICE, 0, 0, 0, 4096);
        let Answer::Done { total, written, truncated: false, more: false } = a else { panic!("{a:?}") };
        assert_eq!(total, written);
        assert_eq!(out.len() as u32, total);
        assert_eq!(out[0], 0);
        assert_eq!(u16le(&out, 32), 6, "n_sections");
        assert_eq!(u16le(&out, 34), 0);
        let mut o = 36;
        let mut seen = Vec::new();
        while o < out.len() {
            let (ty, iface, flags, len) = (u16le(&out, o), out[o + 2], out[o + 3], u32le(&out, o + 4));
            let payload = &out[o + 8..o + 8 + len as usize];
            seen.push((ty, iface, flags, len));
            match ty {
                1 => assert_eq!(payload[8..10], [0x27, 0x06]),
                2 => assert_eq!(payload[..2], [9, 2]),
                4 => {
                    #[rustfmt::skip]
                    let row = [0u8, 0, 3, 1, 1, 0, 0x81, 7, 8, 0, 63, 0, 5, 0, 1, 0];
                    assert_eq!(payload, &row);
                }
                3 => assert_eq!(payload, &[0x05, 0x01, 0x09, 0x06, 0xa1]),
                5 => assert_eq!(payload, b"QE?"),
                _ => {}
            }
            let padded = (len as usize).div_ceil(4) * 4;
            assert!(out[o + 8 + len as usize..o + 8 + padded].iter().all(|&b| b == 0));
            o += 8 + padded;
        }
        assert_eq!(o, out.len());
        assert_eq!(seen, [(1, 0, 0, 18), (2, 0, 0, 18), (4, 0, 0, 16), (3, 0, 0, 5), (5, 0, 0, 3), (6, 0, 2, 0)]);

        let (_, hub) = ask(&mut s, OP_DEVICE, 1, 0, 0, 4096);
        assert_eq!(hub.len(), 32 + 4 + 8 + 20);
        assert_eq!(u16le(&hub, 32), 1);
        assert_eq!((u16le(&hub, 36), u32le(&hub, 40)), (1, 18));
    }

    /// A HID interface beyond the fourth gets a SKIPPED report section; an
    /// interface table cut at 16 rows says TRUNCATED; a capped configuration
    /// says TRUNCATED.
    #[test]
    fn report_sections_beyond_four_are_skipped() {
        let mut s = store();
        let d = s.device_mut(0).unwrap();
        d.flags |= dev_flag::CONFIG_CAPPED;
        d.ifaces_cut = true;
        for k in 0..6u8 {
            d.ifaces[k as usize] = IfaceRow { number: k, class: 3, ..IfaceRow::EMPTY };
        }
        d.n_ifaces = 6;
        d.reports[1].state = Fetch::Failed;
        d.reports[2].state = Fetch::Truncated;
        d.reports[2].len = 2;
        let (_, out) = ask(&mut s, OP_DEVICE, 0, 0, 0, 8192);
        let mut o = 36;
        let mut secs = Vec::new();
        while o < out.len() {
            let len = u32le(&out, o + 4) as usize;
            secs.push((u16le(&out, o), out[o + 2], out[o + 3], len));
            o += 8 + len.div_ceil(4) * 4;
        }
        assert_eq!(secs[1], (2, 0, SECF_TRUNCATED, 18));
        assert_eq!(secs[2], (4, 0, SECF_TRUNCATED, 6 * 16));
        assert_eq!(
            &secs[3..9],
            &[
                (3, 0, 0, 5),
                (3, 1, SECF_FAILED, 0),
                (3, 2, SECF_TRUNCATED, 2),
                (3, 3, SECF_SKIPPED, 0),
                (3, 4, SECF_SKIPPED, 0),
                (3, 5, SECF_SKIPPED, 0),
            ]
        );
    }

    /// §2.5: too small a buffer → header only, TRUNCATED, total = the size.
    #[test]
    fn too_small_buffer_is_header_only() {
        let mut s = store();
        let (a, out) = ask(&mut s, OP_LIST, 0, 0, 0, 111);
        assert_eq!(a, Answer::Done { total: 112, written: 0, truncated: true, more: false });
        assert!(out.is_empty());
        let (a, _) = ask(&mut s, OP_MUTE, 0, 0, 0, 7);
        assert_eq!(a, Answer::Done { total: 8, written: 0, truncated: true, more: false });
    }

    #[test]
    fn bad_index_and_mute_argument_are_invalid() {
        let mut s = store();
        assert_eq!(ask(&mut s, OP_DEVICE, 3, 0, 0, 4096).0, Answer::Invalid);
        assert_eq!(ask(&mut s, OP_REPORTS, 3, 0, 0, 4096).0, Answer::Invalid);
        assert_eq!(ask(&mut s, OP_MUTE, 0, 12_001, 0, 4096).0, Answer::Invalid);
        assert_eq!(ask(&mut s, OP_MUTE, 1, 10, 0, 4096).0, Answer::Invalid);
        assert_eq!(ask(&mut s, 5, 0, 0, 0, 4096).0, Answer::Invalid);
        assert!(!s.muted(0), "a refused mute does not mute");
    }

    /// T1: REPORTS — empty query, entries, overflow (`lost`), MORE.
    #[test]
    fn reports_ring() {
        let mut s = store();
        let (a, out) = ask(&mut s, OP_REPORTS, 0, 0, 0, 4096);
        assert_eq!(a, Answer::Done { total: 16, written: 16, truncated: false, more: false });
        assert_eq!(out, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 0]);

        s.record(0, &[0xAA, 0xBB], 13, 500, false);
        s.record(0, &[1; 70], 1, 501, true);
        let (_, out) = ask(&mut s, OP_REPORTS, 0, 0, 0, 4096);
        assert_eq!(out.len(), 16 + 160);
        assert_eq!((u32le(&out, 0), u32le(&out, 4), u32le(&out, 8), u16le(&out, 12)), (0, 2, 0, 2));
        #[rustfmt::skip]
        let r0 = [0u8, 0, 0, 0, 0xF4, 1, 0, 0, 2, 0, 0, 13, 0, 0, 0, 0, 0xAA, 0xBB];
        assert_eq!(&out[16..34], &r0);
        assert!(out[34..96].iter().all(|&b| b == 0));
        assert_eq!(u32le(&out, 96), 1);
        assert_eq!((u16le(&out, 104), out[106], out[107]), (64, REC_CAPPED | REC_MUTED, 1));

        // arg >= next_seq: the waiting state.
        let (a, out) = ask(&mut s, OP_REPORTS, 0, 2, 0, 4096);
        assert_eq!(a, Answer::Done { total: 16, written: 16, truncated: false, more: false });
        assert_eq!(u16le(&out, 12), 0);

        // Overflow: 300 more → oldest = 302 - 256 = 46; asking from 10 loses 36.
        for i in 0..300u32 {
            s.record(0, &[i as u8], 13, 1000 + i, false);
        }
        let (a, out) = ask(&mut s, OP_REPORTS, 0, 10, 0, 16 + 3 * 80 + 79);
        assert_eq!(a, Answer::Done { total: 16 + 256 * 80, written: 16 + 3 * 80, truncated: false, more: true });
        assert_eq!((u32le(&out, 0), u32le(&out, 4), u32le(&out, 8), u16le(&out, 12)), (46, 302, 36, 3));
        assert_eq!(u32le(&out, 16), 46);
        assert_eq!(out[16 + 16], 44, "seq 46 is the 45th of the 300 (value i = 44)");

        // Not even the body head fits: header only.
        assert_eq!(
            ask(&mut s, OP_REPORTS, 0, 0, 0, 15).0,
            Answer::Done { total: 16 + 256 * 80, written: 0, truncated: true, more: false }
        );
        // A device without a ring: zeros.
        let (_, out) = ask(&mut s, OP_REPORTS, 1, 0, 0, 4096);
        assert_eq!(out, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 0]);
        s.record(1, &[1], 1, 0, false);
        assert_eq!(ask(&mut s, OP_REPORTS, 1, 0, 0, 4096).1.len(), 16, "no ring, nothing recorded");
    }

    /// T1: `seq` wraps past 2^32 without losing order or entries.
    #[test]
    fn sequence_numbers_wrap() {
        let mut s = store();
        s.devices[0].next_seq = u32::MAX - 1;
        for i in 0..4u8 {
            s.record(0, &[i], 13, 0, false);
        }
        let (a, out) = ask(&mut s, OP_REPORTS, 0, u32::MAX - 1, 0, 4096);
        assert_eq!(a, Answer::Done { total: 16 + 4 * 80, written: 16 + 4 * 80, truncated: false, more: false });
        assert_eq!((u32le(&out, 0), u32le(&out, 4), u32le(&out, 8)), (u32::MAX - 1, 2, 0));
        let seqs: Vec<u32> = (0..4).map(|k| u32le(&out, 16 + 80 * k)).collect();
        assert_eq!(seqs, [u32::MAX - 1, u32::MAX, 0, 1]);
        let data: Vec<u8> = (0..4).map(|k| out[16 + 80 * k + 16]).collect();
        assert_eq!(data, [0, 1, 2, 3]);
        let (_, out) = ask(&mut s, OP_REPORTS, 0, 1, 0, 4096);
        assert_eq!((u16le(&out, 12), u32le(&out, 16)), (1, 1));
    }

    /// T1: MUTE — on, answer, self-expiry (wrap-safe), off.
    #[test]
    fn mute_expires_by_itself() {
        let mut s = store();
        let (a, out) = ask(&mut s, OP_MUTE, 0, 7000, u32::MAX - 10, 8);
        assert_eq!(a, Answer::Done { total: 8, written: 8, truncated: false, more: false });
        let until = (u32::MAX - 10).wrapping_add(7000);
        assert_eq!(out, [until.to_le_bytes().as_slice(), &[0; 4]].concat());
        assert!(s.muted(u32::MAX));
        assert!(s.muted(until.wrapping_sub(1)));
        assert!(!s.muted(until), "deadline reached");
        assert!(!s.muted(until.wrapping_sub(1)), "stays off");

        ask(&mut s, OP_MUTE, 0, 100, 0, 8);
        assert!(s.muted(50));
        let (_, out) = ask(&mut s, OP_MUTE, 0, 0, 60, 8);
        assert_eq!(out, [0; 8]);
        assert!(!s.muted(61));
        assert_eq!(s.set_mute(u32::MAX, 1), 1, "a deadline of 0 moves one tick");
    }

    /// The decoder sees each mute's start once (`mute_state`), also across a
    /// prolonged mute, and again for the next mute after one ended.
    #[test]
    fn mute_start_is_seen_once() {
        let mut s = store();
        assert_eq!(s.mute_state(10), (false, false));
        s.set_mute(10, 100);
        assert_eq!(s.mute_state(11), (true, true));
        assert_eq!(s.mute_state(12), (true, false));
        s.set_mute(50, 100);
        assert_eq!(s.mute_state(60), (true, false), "a prolonged mute is not a new one");
        assert_eq!(s.mute_state(150), (false, false), "expired");
        s.set_mute(200, 5);
        assert_eq!(s.mute_state(201), (true, true));
        s.set_mute(202, 0);
        assert_eq!(s.mute_state(203), (false, false));
        s.set_mute(204, 5);
        assert_eq!(s.mute_state(205), (true, true), "off then on again is a new mute");
    }

    #[test]
    fn a_refusing_sink_is_a_fault() {
        let mut s = store();
        let a = s.answer(Request { op: OP_LIST, index: 0, arg: 0, now: 0, room: 4096 }, &mut |_| false);
        assert_eq!(a, Answer::Faulted);
    }

    #[test]
    fn strings_become_ascii_and_cap_at_31() {
        let mut t = Text::EMPTY;
        let mut raw = vec![2 + 2 * 40, 3];
        for k in 0..40u8 {
            raw.extend_from_slice(&[b'a' + k % 26, 0]);
        }
        t.set_from_descriptor(&raw[..64]);
        assert_eq!((t.len, t.state), (31, Fetch::Truncated));
        t.set_from_descriptor(&[6, 3, b'A', 0, 0x00, 0x30]);
        assert_eq!((t.len, t.state, &t.bytes[..2]), (2, Fetch::Ok, &b"A?"[..]));
    }
}

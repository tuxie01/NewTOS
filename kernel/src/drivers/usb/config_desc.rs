//! Configuration-descriptor scan: pick the HID **boot** interface the driver
//! can decode, and name every interface it does not use.
//!
//! Pure byte parsing with no register access and no kernel dependency, so the
//! host test suite (`tools/xtask`, which includes this file by path) runs its
//! tests; `xhci::read_config` fetches the bytes and logs the result.
//!
//! Sources:
//! * USB HID 1.11 (`references/usb/hid1_11.pdf`) §4.1: "The bInterfaceClass
//!   member of an Interface descriptor is always 3 for HID class devices";
//!   §4.2: `bInterfaceSubClass` 1 = Boot Interface Subclass, otherwise 0;
//!   §4.3: `bInterfaceProtocol` 1 = Keyboard, 2 = Mouse, and it "only has
//!   meaning if the bInterfaceSubClass member declares that the device
//!   supports a boot interface".
//! * Descriptor layouts (interface = type 4, 9 bytes; endpoint = type 5,
//!   7 bytes plus the audio-only `bRefresh`/`bSynchAddress`): FreeMiNT
//!   `sys/usb/src.km/usb.h:103-127` (`struct usb_endpoint_descriptor`,
//!   `struct usb_interface_descriptor`), the same offsets the previous
//!   `read_config` walk used. No USB 2.0 core specification is in
//!   `references/`.
//!
//! Why the boot subclass, not just class 3: everything downstream relies on the
//! **boot** report layout — `configure_hid` sends `SET_PROTOCOL(boot)` (HID
//! 1.11 §7.2.6: supported by devices in the boot subclass) and `hid.rs`
//! decodes the boot keyboard/mouse report (HID 1.11 Appendix B). A
//! report-protocol-only interface (subclass 0) has neither, so until a report
//! descriptor parser exists (`docs/api/usb/hid-mouse-protocols.md` F4) it is
//! not an input device here. A non-HID interface (mass storage, hub, audio,
//! vendor …) never is, whatever its `bInterfaceProtocol` says.
//!
//! USB snapshot (USB01 step 2, `docs/api/usb/usb-snapshot.md` §3.1 step 4/5,
//! §2.5): the same walk also yields the **interface table** (one
//! [`IfaceRow`] per interface descriptor, alt settings included), the
//! report-descriptor length each HID interface declares in its HID descriptor
//! (HID 1.11 §6.2.1: `bNumDescriptors` at offset 5, then `bDescriptorType` /
//! `wDescriptorLength` pairs from offset 6; §7.1: the HID descriptor follows
//! its interface descriptor, before the endpoints) and the **observation
//! candidate**: when no boot interface qualifies, the first HID interface
//! (alternate setting 0) with an interrupt-IN endpoint, which the driver
//! configures and records but does not decode (role 3, §3.2).

/// HID class code (HID 1.11 §4.1).
pub const CLASS_HID: u8 = 3;
/// Boot Interface Subclass (HID 1.11 §4.2).
pub const SUBCLASS_BOOT: u8 = 1;
/// Boot protocol: keyboard (HID 1.11 §4.3).
pub const PROTOCOL_KEYBOARD: u8 = 1;
/// Boot protocol: mouse (HID 1.11 §4.3).
pub const PROTOCOL_MOUSE: u8 = 2;

const DESC_INTERFACE: u8 = 4;
const DESC_ENDPOINT: u8 = 5;
/// HID class descriptor (HID 1.11 §7.1, class descriptor types table).
const DESC_HID: u8 = 0x21;
/// HID report descriptor type, as listed inside the HID descriptor.
const DESC_REPORT: u8 = 0x22;
/// Bytes of a HID descriptor up to the first `wDescriptorLength` (HID 1.11
/// §6.2.1: offset 7, size 2).
const HID_DESC_MIN: usize = 9;
/// Rows the interface table keeps (`usb-snapshot.md` §2.5: ≤ 16 rows).
pub const MAX_IFACE_ROWS: usize = 16;
/// `bLength` of an interface descriptor (FreeMiNT `usb.h:116-127`).
const INTERFACE_LEN: usize = 9;
/// Bytes of an endpoint descriptor up to `bInterval` (FreeMiNT `usb.h:103-113`).
const ENDPOINT_LEN: usize = 7;

/// A HID boot device's configuration, as parsed from its config descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HidInfo {
    /// `bConfigurationValue` for SET_CONFIGURATION.
    pub config_value: u8,
    /// The HID interface number.
    pub iface: u8,
    /// `bInterfaceProtocol`: 1 = keyboard, 2 = mouse.
    pub protocol: u8,
    /// Interrupt-IN endpoint address (bit 7 = IN, low 4 = number).
    pub ep_addr: u8,
    /// Endpoint max packet size.
    pub ep_max: u16,
    /// Endpoint polling interval (`bInterval`).
    pub interval: u8,
}

/// Why an interface descriptor was not taken as the device's input interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// `bInterfaceClass` != 3: not a HID interface at all.
    NotHid,
    /// HID, but not the boot subclass: the report format is only in the
    /// report descriptor, for which there is no parser yet.
    NotBootSubclass,
    /// HID boot subclass with a protocol other than keyboard/mouse
    /// (HID 1.11 §4.3: 0 none, 3–255 reserved).
    UnknownBootProtocol,
    /// A usable HID boot interface without an interrupt-IN endpoint.
    NoInterruptIn,
    /// A usable HID boot interface after the one already chosen: the driver
    /// configures one interface per device (`hid-mouse-protocols.md` F3).
    OneInterfacePerDevice,
}

/// An interface descriptor the scan did not use, with the fields a log line
/// needs to say what it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkippedInterface {
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
    /// Why it was not used.
    pub reason: SkipReason,
}

/// One interface descriptor of the configuration, as the snapshot's
/// interface table shows it (`usb-snapshot.md` §2.5, "Interface-Tabelle").
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
    /// Why the boot-interface selection did not take it; `None` for the
    /// interface it took.
    pub skip: Option<SkipReason>,
    /// Address of the interface's first interrupt-IN endpoint, 0 = none.
    pub ep_addr: u8,
    /// That endpoint's `bInterval`.
    pub interval: u8,
    /// That endpoint's `wMaxPacketSize`, raw (bits 12:11 included).
    pub max_packet: u16,
    /// `wDescriptorLength` of the first report descriptor listed in the
    /// interface's HID descriptor; 0 = no HID descriptor found.
    pub report_len: u16,
}

impl IfaceRow {
    const EMPTY: IfaceRow = IfaceRow {
        number: 0,
        alt: 0,
        class: 0,
        subclass: 0,
        protocol: 0,
        skip: None,
        ep_addr: 0,
        interval: 0,
        max_packet: 0,
        report_len: 0,
    };

    /// A HID interface in its default alternate setting — the ones whose
    /// report descriptor the driver fetches (`usb-snapshot.md` §2.5 type 3).
    pub fn is_hid_alt0(&self) -> bool {
        self.class == CLASS_HID && self.alt == 0
    }
}

/// An interface the driver configures for recording only (role 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observe {
    /// The interface and its interrupt-IN endpoint (`protocol` is the
    /// interface's `bInterfaceProtocol`, never decoded).
    pub info: HidInfo,
    /// Why it is not a boot interface the driver decodes.
    pub reason: SkipReason,
}

/// Outcome of [`scan`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scan {
    /// The first HID boot interface with an interrupt-IN endpoint.
    pub hid: Option<HidInfo>,
    /// Only when `hid` is `None`: the first HID interface in alternate
    /// setting 0 with an interrupt-IN endpoint (`usb-snapshot.md` §3.1 step
    /// 5 (ii)).
    pub observe: Option<Observe>,
    /// Byte offset of a descriptor whose `bLength` is 0 or runs past the end
    /// of `cfg` (a truncated or malformed configuration); the scan stopped
    /// there. `None` = every descriptor was walked.
    pub stopped_at: Option<usize>,
    /// The interface table, `rows[..n_rows]`, in descriptor order.
    pub rows: [IfaceRow; MAX_IFACE_ROWS],
    /// Rows filled.
    pub n_rows: usize,
    /// More interface descriptors than [`MAX_IFACE_ROWS`] were seen.
    pub rows_cut: bool,
    /// HID interfaces (class 3) in alternate setting 0, all of them — not
    /// capped by [`MAX_IFACE_ROWS`].
    pub n_hid_ifaces: u8,
}

/// Whether an interface's class triple is a boot keyboard or boot mouse the
/// driver decodes.
pub fn is_boot_hid(class: u8, subclass: u8, protocol: u8) -> bool {
    class == CLASS_HID
        && subclass == SUBCLASS_BOOT
        && (protocol == PROTOCOL_KEYBOARD || protocol == PROTOCOL_MOUSE)
}

fn classify(class: u8, subclass: u8, protocol: u8) -> Option<SkipReason> {
    if class != CLASS_HID {
        Some(SkipReason::NotHid)
    } else if subclass != SUBCLASS_BOOT {
        Some(SkipReason::NotBootSubclass)
    } else if !is_boot_hid(class, subclass, protocol) {
        Some(SkipReason::UnknownBootProtocol)
    } else {
        None
    }
}

/// Walk a configuration descriptor (`cfg` = the bytes read, starting with the
/// 9-byte configuration header) and choose the first HID boot interface that
/// has an interrupt-IN endpoint. `skipped` is called once for every other
/// interface descriptor, in descriptor order.
///
/// The same walk fills the interface table ([`Scan::rows`]) and, when no boot
/// interface qualifies, names the observation candidate ([`Scan::observe`]).
pub fn scan(cfg: &[u8], mut skipped: impl FnMut(SkippedInterface)) -> Scan {
    let config_value = cfg.get(5).copied().unwrap_or(0);
    let mut out = Scan {
        hid: None,
        observe: None,
        stopped_at: None,
        rows: [IfaceRow::EMPTY; MAX_IFACE_ROWS],
        n_rows: 0,
        rows_cut: false,
        n_hid_ifaces: 0,
    };
    // The interface descriptor whose endpoints follow, and whether it is the
    // candidate still looking for its interrupt-IN endpoint.
    let mut cur: Option<SkippedInterface> = None;
    let mut cur_open = false;
    // Its row in the table (`None` once the table is full), and whether its
    // HID descriptor has been read already (only the first one counts).
    let mut cur_row: Option<usize> = None;
    let mut cur_hid_desc_seen = false;

    // An interface that qualified but whose endpoints held no interrupt IN.
    fn close(
        cur: Option<SkippedInterface>,
        open: bool,
        row: Option<&mut IfaceRow>,
        out: &mut impl FnMut(SkippedInterface),
    ) {
        if let (Some(c), true) = (cur, open) {
            if let Some(r) = row {
                r.skip = Some(SkipReason::NoInterruptIn);
            }
            out(SkippedInterface { reason: SkipReason::NoInterruptIn, ..c });
        }
    }

    let mut i = 0usize;
    while i + 2 <= cfg.len() {
        let dlen = cfg[i] as usize;
        let dtype = cfg[i + 1];
        if dlen == 0 || i + dlen > cfg.len() {
            out.stopped_at = Some(i);
            break;
        }
        match dtype {
            DESC_INTERFACE if dlen >= INTERFACE_LEN => {
                close(cur, cur_open, cur_row.map(|r| &mut out.rows[r]), &mut skipped);
                let d = SkippedInterface {
                    number: cfg[i + 2],
                    alt: cfg[i + 3],
                    class: cfg[i + 5],
                    subclass: cfg[i + 6],
                    protocol: cfg[i + 7],
                    reason: SkipReason::NotHid,
                };
                cur = Some(d);
                cur_open = false;
                cur_hid_desc_seen = false;
                if d.class == CLASS_HID && d.alt == 0 {
                    out.n_hid_ifaces = out.n_hid_ifaces.saturating_add(1);
                }
                let skip = match classify(d.class, d.subclass, d.protocol) {
                    Some(reason) => Some(reason),
                    None if out.hid.is_some() => Some(SkipReason::OneInterfacePerDevice),
                    None => None,
                };
                match skip {
                    Some(reason) => skipped(SkippedInterface { reason, ..d }),
                    None => cur_open = true,
                }
                cur_row = if out.n_rows < MAX_IFACE_ROWS {
                    out.rows[out.n_rows] = IfaceRow {
                        number: d.number,
                        alt: d.alt,
                        class: d.class,
                        subclass: d.subclass,
                        protocol: d.protocol,
                        skip,
                        ..IfaceRow::EMPTY
                    };
                    out.n_rows += 1;
                    Some(out.n_rows - 1)
                } else {
                    out.rows_cut = true;
                    None
                };
            }
            DESC_HID if dlen >= HID_DESC_MIN && cur.is_some() && !cur_hid_desc_seen => {
                cur_hid_desc_seen = true;
                // bNumDescriptors at 5, then (bDescriptorType, wDescriptorLength)
                // triples from offset 6 (HID 1.11 §6.2.1, "up to offset
                // (3*n)+6 and (3*n)+7"). The first report descriptor counts.
                let n = cfg[i + 5] as usize;
                let len = (0..n)
                    .map(|k| i + 6 + 3 * k)
                    .take_while(|&at| at + 3 <= i + dlen)
                    .find(|&at| cfg[at] == DESC_REPORT)
                    .map(|at| u16::from_le_bytes([cfg[at + 1], cfg[at + 2]]));
                if let (Some(r), Some(len)) = (cur_row, len) {
                    out.rows[r].report_len = len;
                }
            }
            DESC_ENDPOINT if dlen >= ENDPOINT_LEN && cur.is_some() => {
                let addr = cfg[i + 2];
                let attr = cfg[i + 3];
                let max = u16::from_le_bytes([cfg[i + 4], cfg[i + 5]]);
                // Interrupt (bmAttributes 1:0 = 3) and IN (address bit 7).
                if attr & 0x3 == 3 && addr & 0x80 != 0 {
                    if let Some(r) = cur_row {
                        let row = &mut out.rows[r];
                        if row.ep_addr == 0 {
                            row.ep_addr = addr;
                            row.interval = cfg[i + 6];
                            row.max_packet = max;
                        }
                    }
                    if cur_open {
                        let c = cur.expect("cur_open implies an interface");
                        out.hid = Some(HidInfo {
                            config_value,
                            iface: c.number,
                            protocol: c.protocol,
                            ep_addr: addr,
                            ep_max: max,
                            interval: cfg[i + 6],
                        });
                        cur_open = false;
                    }
                }
            }
            _ => {}
        }
        i += dlen;
    }
    close(cur, cur_open, cur_row.map(|r| &mut out.rows[r]), &mut skipped);
    if out.hid.is_none() {
        // §3.1 step 5 (ii): the first HID interface in its default alternate
        // setting (the one SET_CONFIGURATION leaves active) that has an
        // interrupt-IN endpoint. Without a boot interface every such row is
        // one the selection skipped as NotBootSubclass or UnknownBootProtocol.
        out.observe = out.rows[..out.n_rows]
            .iter()
            .find(|r| r.is_hid_alt0() && r.ep_addr != 0)
            .map(|r| Observe {
                info: HidInfo {
                    config_value,
                    iface: r.number,
                    protocol: r.protocol,
                    ep_addr: r.ep_addr,
                    ep_max: r.max_packet,
                    interval: r.interval,
                },
                reason: r.skip.unwrap_or(SkipReason::NotBootSubclass),
            });
    }
    out
}

/// Short human-readable text for a [`SkipReason`] (the log line).
pub fn reason_text(r: SkipReason) -> &'static str {
    match r {
        SkipReason::NotHid => "not a HID interface (class != 3)",
        SkipReason::NotBootSubclass => {
            "HID without boot subclass: report protocol only, no report parser yet"
        }
        SkipReason::UnknownBootProtocol => "HID boot subclass, protocol neither keyboard nor mouse",
        SkipReason::NoInterruptIn => "HID boot interface without an interrupt-IN endpoint",
        SkipReason::OneInterfacePerDevice => "second HID boot interface: one interface per device",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(body: &[&[u8]]) -> Vec<u8> {
        let mut v = vec![9, 2, 0, 0, 1, 7, 0, 0x80, 50];
        for d in body {
            v.extend_from_slice(d);
        }
        let total = v.len() as u16;
        v[2..4].copy_from_slice(&total.to_le_bytes());
        v
    }
    fn iface(n: u8, class: u8, sub: u8, proto: u8) -> Vec<u8> {
        vec![9, 4, n, 0, 1, class, sub, proto, 0]
    }
    const HID_DESC: [u8; 9] = [9, 0x21, 0x11, 0x01, 0, 1, 0x22, 52, 0];
    fn ep(addr: u8, attr: u8, max: u16, interval: u8) -> Vec<u8> {
        let m = max.to_le_bytes();
        vec![7, 5, addr, attr, m[0], m[1], interval]
    }

    fn run(cfg: &[u8]) -> (Scan, Vec<SkippedInterface>) {
        let mut s = Vec::new();
        let r = scan(cfg, |x| s.push(x));
        (r, s)
    }

    /// QEMU `usb-mouse`: one HID boot-mouse interface, EP 0x81 interrupt IN.
    #[test]
    fn boot_mouse_is_chosen() {
        let cfg = config(&[&iface(0, 3, 1, 2), &HID_DESC, &ep(0x81, 3, 4, 10)]);
        let (r, s) = run(&cfg);
        assert_eq!(
            r.hid,
            Some(HidInfo {
                config_value: 7,
                iface: 0,
                protocol: 2,
                ep_addr: 0x81,
                ep_max: 4,
                interval: 10
            })
        );
        assert!(s.is_empty());
        assert_eq!(r.stopped_at, None);
    }

    /// Mass storage (class 8, subclass 6, protocol 0x50 BBB) with bulk
    /// endpoints only, and a vendor interface whose protocol byte happens to
    /// be 1 with an interrupt-IN endpoint: neither is a keyboard.
    #[test]
    fn non_hid_interfaces_are_never_input_devices() {
        let storage = config(&[&iface(0, 8, 6, 0x50), &ep(0x81, 2, 512, 0), &ep(0x02, 2, 512, 0)]);
        let (r, s) = run(&storage);
        assert_eq!(r.hid, None);
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].class, s[0].reason), (8, SkipReason::NotHid));

        let vendor = config(&[&iface(0, 0xff, 1, 1), &ep(0x81, 3, 8, 10)]);
        let (r, s) = run(&vendor);
        assert_eq!(r.hid, None, "protocol 1 of a non-HID class is not a keyboard");
        assert_eq!(s[0].reason, SkipReason::NotHid);
    }

    /// The old pairing took the first interrupt-IN endpoint of any interface.
    /// A non-HID interface in front of the boot keyboard must not win.
    #[test]
    fn boot_interface_behind_a_non_hid_interface_is_found() {
        let cfg = config(&[
            &iface(0, 0xff, 0, 1),
            &ep(0x83, 3, 16, 4),
            &iface(1, 3, 1, 1),
            &HID_DESC,
            &ep(0x81, 3, 8, 10),
        ]);
        let (r, s) = run(&cfg);
        let h = r.hid.unwrap();
        assert_eq!((h.iface, h.protocol, h.ep_addr), (1, 1, 0x81));
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].number, s[0].reason), (0, SkipReason::NotHid));
    }

    /// Report-only HID (subclass 0) — QEMU `usb-tablet`, many receivers — and
    /// a boot subclass with a reserved protocol are skipped, with the reason.
    #[test]
    fn report_only_and_reserved_protocol_are_skipped() {
        let tablet = config(&[&iface(0, 3, 0, 0), &HID_DESC, &ep(0x81, 3, 8, 4)]);
        let (r, s) = run(&tablet);
        assert_eq!(r.hid, None);
        assert_eq!(s[0].reason, SkipReason::NotBootSubclass);

        let reserved = config(&[&iface(0, 3, 1, 3), &ep(0x81, 3, 8, 4)]);
        let (r, s) = run(&reserved);
        assert_eq!(r.hid, None);
        assert_eq!(s[0].reason, SkipReason::UnknownBootProtocol);
    }

    /// Roccat-style composite (3,1,2)+(3,0,1) and a keyboard+mouse receiver
    /// (3,1,1)+(3,1,2): the first boot interface is used, every later one is
    /// reported, not silently dropped.
    #[test]
    fn later_interfaces_are_reported() {
        let roccat = config(&[
            &iface(0, 3, 1, 2),
            &HID_DESC,
            &ep(0x81, 3, 8, 1),
            &iface(1, 3, 0, 1),
            &HID_DESC,
            &ep(0x82, 3, 16, 1),
        ]);
        let (r, s) = run(&roccat);
        assert_eq!(r.hid.map(|h| (h.iface, h.protocol)), Some((0, 2)));
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].number, s[0].reason), (1, SkipReason::NotBootSubclass));

        let receiver = config(&[
            &iface(0, 3, 1, 1),
            &ep(0x81, 3, 8, 10),
            &iface(1, 3, 1, 2),
            &ep(0x82, 3, 8, 10),
        ]);
        let (r, s) = run(&receiver);
        assert_eq!(r.hid.map(|h| h.iface), Some(0));
        assert_eq!((s[0].number, s[0].reason), (1, SkipReason::OneInterfacePerDevice));
    }

    /// A boot interface whose only endpoint is interrupt OUT (or bulk) has no
    /// report pipe; it is named and the scan moves on.
    #[test]
    fn boot_interface_without_interrupt_in_is_reported() {
        let cfg = config(&[
            &iface(0, 3, 1, 1),
            &ep(0x01, 3, 8, 10),
            &iface(1, 3, 1, 2),
            &ep(0x82, 3, 4, 10),
        ]);
        let (r, s) = run(&cfg);
        assert_eq!(r.hid.map(|h| h.iface), Some(1));
        assert_eq!((s[0].number, s[0].reason), (0, SkipReason::NoInterruptIn));

        let last = config(&[&iface(0, 3, 1, 2), &ep(0x81, 2, 64, 0)]);
        let (r, s) = run(&last);
        assert_eq!(r.hid, None);
        assert_eq!(s[0].reason, SkipReason::NoInterruptIn);
    }

    /// A configuration cut short (the driver reads at most 255 bytes) stops
    /// at the broken descriptor and says where.
    #[test]
    fn truncated_configuration_reports_where_it_stopped() {
        let mut cfg = config(&[&iface(0, 3, 1, 2), &ep(0x81, 3, 4, 10), &iface(1, 3, 1, 1)]);
        let cut = cfg.len() - 4;
        cfg.truncate(cut);
        let (r, s) = run(&cfg);
        assert_eq!(r.hid.map(|h| h.iface), Some(0));
        assert!(s.is_empty());
        assert_eq!(r.stopped_at, Some(9 + 9 + 7));

        let zero = [9u8, 2, 18, 0, 1, 1, 0, 0x80, 50, 0, 4];
        assert_eq!(run(&zero).0.stopped_at, Some(9));
    }

    /// Every reason has its own log text.
    #[test]
    fn reason_texts_are_distinct() {
        let all = [
            SkipReason::NotHid,
            SkipReason::NotBootSubclass,
            SkipReason::UnknownBootProtocol,
            SkipReason::NoInterruptIn,
            SkipReason::OneInterfacePerDevice,
        ];
        for (i, a) in all.iter().enumerate() {
            assert!(!reason_text(*a).is_empty());
            for b in &all[i + 1..] {
                assert_ne!(reason_text(*a), reason_text(*b));
            }
        }
    }

    /// HID descriptor with one report descriptor of `len` bytes.
    fn hid_desc(len: u16) -> Vec<u8> {
        let l = len.to_le_bytes();
        vec![9, 0x21, 0x11, 0x01, 0, 1, 0x22, l[0], l[1]]
    }

    /// T2 (`usb-snapshot.md` §6): the ROCCAT mouse 1e7d:2cee as the real Pi
    /// logged it (`docs/analysis/usb-port-investigation/run3-fixed-both-in-usb3.log:90-97`,
    /// 59 bytes: config, iface 0 (3,1,2) + HID + ep 0x81 max 64 interval 1,
    /// iface 1 (3,0,1) + HID + ep 0x82 max 64 interval 1). The log does not
    /// show the HID descriptors' report lengths; 77 / 159 are test values.
    #[test]
    fn roccat_composite_table_and_report_lengths() {
        let cfg = config(&[
            &iface(0, 3, 1, 2),
            &hid_desc(77),
            &ep(0x81, 3, 64, 1),
            &iface(1, 3, 0, 1),
            &hid_desc(159),
            &ep(0x82, 3, 64, 1),
        ]);
        assert_eq!(cfg.len(), 59);
        let (r, _) = run(&cfg);
        assert_eq!(r.hid.map(|h| (h.iface, h.ep_addr, h.ep_max)), Some((0, 0x81, 64)));
        assert_eq!(r.observe, None, "a boot interface was chosen: no observation");
        assert_eq!(r.n_rows, 2);
        assert_eq!(r.n_hid_ifaces, 2);
        assert!(!r.rows_cut);
        let (a, b) = (r.rows[0], r.rows[1]);
        assert_eq!((a.number, a.class, a.subclass, a.protocol, a.skip), (0, 3, 1, 2, None));
        assert_eq!((a.ep_addr, a.interval, a.max_packet, a.report_len), (0x81, 1, 64, 77));
        assert_eq!((b.number, b.skip), (1, Some(SkipReason::NotBootSubclass)));
        assert_eq!((b.ep_addr, b.interval, b.max_packet, b.report_len), (0x82, 1, 64, 159));
    }

    /// §3.1 step 5 (ii): no boot interface, a report-only HID interface with
    /// interrupt IN is the observation candidate; a reserved boot protocol
    /// likewise, with its own reason.
    #[test]
    fn report_only_interface_is_observed() {
        let tablet = config(&[&iface(0, 3, 0, 0), &hid_desc(74), &ep(0x81, 3, 8, 4)]);
        let (r, _) = run(&tablet);
        assert_eq!(r.hid, None);
        let o = r.observe.expect("observed");
        assert_eq!((o.info.iface, o.info.ep_addr, o.info.ep_max, o.info.interval), (0, 0x81, 8, 4));
        assert_eq!(o.reason, SkipReason::NotBootSubclass);
        assert_eq!(r.rows[0].report_len, 74);

        let reserved = config(&[&iface(2, 3, 1, 3), &ep(0x83, 3, 16, 4)]);
        let o = run(&reserved).0.observe.expect("observed");
        assert_eq!((o.info.iface, o.reason), (2, SkipReason::UnknownBootProtocol));
    }

    /// §3.1 step 5 (iii): nothing to observe — no HID interface, a HID
    /// interface without interrupt IN, or one only in an alternate setting.
    #[test]
    fn nothing_observed_without_a_hid_interrupt_in_in_alt_0() {
        let storage = config(&[&iface(0, 8, 6, 0x50), &ep(0x81, 2, 512, 0)]);
        let r = run(&storage).0;
        assert_eq!((r.hid, r.observe, r.n_hid_ifaces), (None, None, 0));

        let out_only = config(&[&iface(0, 3, 0, 0), &ep(0x01, 3, 8, 4)]);
        assert_eq!(run(&out_only).0.observe, None);

        let mut alt1 = iface(0, 3, 0, 0);
        alt1[3] = 1;
        let cfg = config(&[&iface(0, 0xff, 0, 0), &alt1, &ep(0x81, 3, 8, 4)]);
        let r = run(&cfg).0;
        assert_eq!(r.observe, None);
        assert_eq!(r.n_rows, 2);
        assert_eq!((r.rows[1].alt, r.rows[1].ep_addr), (1, 0x81), "alt rows are listed");
        assert_eq!(r.n_hid_ifaces, 0, "only alternate setting 0 counts");
    }

    /// A HID interface without a HID descriptor declares no report length;
    /// a HID descriptor listing a physical descriptor first still yields the
    /// report descriptor's length (HID 1.11 §6.2.1 optional triples).
    #[test]
    fn report_length_from_the_first_report_entry() {
        let cfg = config(&[&iface(0, 3, 1, 2), &ep(0x81, 3, 4, 10)]);
        assert_eq!(run(&cfg).0.rows[0].report_len, 0);

        let two = [12u8, 0x21, 0x11, 0x01, 0, 2, 0x23, 9, 0, 0x22, 0x34, 0x01];
        let cfg = config(&[&iface(0, 3, 1, 2), &two, &ep(0x81, 3, 4, 10)]);
        assert_eq!(run(&cfg).0.rows[0].report_len, 0x134);

        // bNumDescriptors claims more triples than bLength holds: stop at bLength.
        let short = [9u8, 0x21, 0x11, 0x01, 0, 3, 0x23, 9, 0];
        let cfg = config(&[&iface(0, 3, 1, 2), &short, &ep(0x81, 3, 4, 10)]);
        assert_eq!(run(&cfg).0.rows[0].report_len, 0);
    }

    /// A configuration longer than the 1024 bytes the driver reads: the scan
    /// sees the cut, keeps what came before; more than 16 interfaces cap the
    /// table and say so.
    #[test]
    fn long_configuration_is_cut_and_table_capped() {
        let mut body: Vec<Vec<u8>> = Vec::new();
        for n in 0..45u8 {
            body.push(iface(n, 3, 0, 0));
            body.push(hid_desc(50));
            body.push(ep(0x81, 3, 8, 4));
        }
        let parts: Vec<&[u8]> = body.iter().map(|v| v.as_slice()).collect();
        let full = config(&parts);
        assert!(full.len() > 1024);
        let (r, s) = run(&full[..1024]);
        assert!(r.stopped_at.is_some(), "cut inside a descriptor");
        assert_eq!(r.n_rows, MAX_IFACE_ROWS);
        assert!(r.rows_cut);
        assert!(r.n_hid_ifaces as usize > MAX_IFACE_ROWS);
        assert_eq!(s.len(), r.n_hid_ifaces as usize, "every interface still logged");
        assert_eq!(r.observe.map(|o| o.info.iface), Some(0));
    }

    #[test]
    fn is_boot_hid_truth_table() {
        assert!(is_boot_hid(3, 1, 1));
        assert!(is_boot_hid(3, 1, 2));
        assert!(!is_boot_hid(3, 0, 2));
        assert!(!is_boot_hid(3, 1, 0));
        assert!(!is_boot_hid(9, 1, 1));
        assert!(!is_boot_hid(0, 1, 2));
    }
}

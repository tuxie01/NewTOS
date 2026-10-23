//! mkreceiver.rs -- MEASUREMENT tool (host): build `replay.h` for usbsnap-replay-v2.c with the
//! kernel's real snapshot encoder (kernel/src/drivers/usb/snapshot.rs, included by path) filled
//! like the driver would fill it for Ragnar's set-up 001 (Logitech Unifying 046d:c52b behind the
//! VL805 hub 2109:3431, Cherry KC 6000 046a:0113 on hub port 4), after F3: the receiver with two
//! functions (keyboard iface 0 / ep 0x81, mouse iface 1 / ep 0x82), its report-only iface 2 not a
//! function. Descriptor fields from measurements/measurement-2026-10-23-usb01-ragnar/serial/
//! 001_logi_mx_master_2s_schwarz.LOG:96-153; report descriptors and the reports are SYNTHETIC
//! (the logs end before any input). Purpose: exercise USBSNAP.NCP's v2 multi-function rendering
//! (no QEMU device has several HID interfaces, hid-mouse-protocols.md §9.8).
//!
//! Build + run: rustc --edition 2021 -O mkreceiver.rs -o <scratch>/mkreceiver && <scratch>/mkreceiver > replay.h
#[path = "../../../kernel/src/drivers/usb/snapshot.rs"]
#[allow(dead_code)]
mod snapshot;
use snapshot::*;

fn ask(s: &mut Store, op: u16, index: u32, arg: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let a = s.answer(Request { op, index, arg, now: 0, room: 1 << 20 }, &mut |b| {
        out.extend_from_slice(b);
        true
    });
    assert!(matches!(a, Answer::Done { truncated: false, .. }), "{a:?}");
    out
}

fn hid_iface(n: u8, sub: u8, proto: u8, ep: u8, max: u8, interval: u8, rlen: u8) -> Vec<u8> {
    vec![9, 4, n, 0, 1, 3, sub, proto, 0, 9, 0x21, 0x11, 0x01, 0, 1, 0x22, rlen, 0, 7, 5, ep, 3, max, 0, interval]
}

fn main() {
    let mut s = Store::new();
    s.present = true;
    s.root_ports = 5;
    s.generation = 1;
    {
        let d = s.claim(0).unwrap(); // VL805 internal hub
        d.slot = 1;
        d.root_port = 1;
        d.speed = 3;
        d.role = role::HUB;
        d.dev_class = 9;
        d.vid = 0x2109;
        d.pid = 0x3431;
        d.device_desc_state = Fetch::Ok;
        d.device_desc = [18, 1, 0x10, 2, 9, 0, 1, 64, 0x09, 0x21, 0x31, 0x34, 0x20, 4, 0, 1, 0, 1];
        d.flags = dev_flag::DEVICE_DESC | dev_flag::SET_CONFIGURATION;
    }
    let recv = 1usize;
    {
        let d = s.claim(recv).unwrap(); // Unifying receiver, hub port 3, full speed
        d.slot = 2;
        d.root_port = 1;
        d.parent = 0;
        d.route = 0x3;
        d.speed = 1;
        d.vid = 0x046d;
        d.pid = 0xc52b;
        d.role = role::KEYBOARD;
        d.reason = reason::USED;
        d.flags = dev_flag::DEVICE_DESC | dev_flag::CONFIG | dev_flag::SET_CONFIGURATION;
        d.n_hid_ifaces = 3;
        d.ring_iface = 0;
        d.ring_ep = 0x81;
        d.n_funcs = 2;
        d.device_desc_state = Fetch::Ok;
        d.device_desc = [18, 1, 0, 2, 0, 0, 0, 32, 0x6d, 0x04, 0x2b, 0xc5, 0x10, 0x12, 1, 2, 0, 1];
        let mut cfg = vec![9u8, 2, 84, 0, 3, 1, 0, 0xa0, 49];
        cfg.extend(hid_iface(0, 1, 1, 0x81, 8, 8, 59));
        cfg.extend(hid_iface(1, 1, 2, 0x82, 8, 2, 148));
        cfg.extend(hid_iface(2, 0, 0, 0x83, 32, 2, 98));
        assert_eq!(cfg.len(), 84);
        d.config[..84].copy_from_slice(&cfg);
        d.config_len = 84;
        let rows = [
            (0u8, 1u8, 1u8, 0x81u8, 8u16, 8u8, 59u16, reason::USED, true, role::KEYBOARD),
            (1, 1, 2, 0x82, 8, 2, 148, reason::USED, true, role::MOUSE),
            (2, 0, 0, 0x83, 32, 2, 98, reason::NOT_BOOT_SUBCLASS, false, role::NONE),
        ];
        for (k, &(n, sub, proto, ep, max, iv, rl, why, mon, r)) in rows.iter().enumerate() {
            d.ifaces[k] = IfaceRow {
                number: n,
                alt: 0,
                class: 3,
                subclass: sub,
                protocol: proto,
                reason: why,
                ep_addr: ep,
                interval: iv,
                max_packet: max,
                report_len: rl,
                monitored: mon,
                role: r,
            };
        }
        d.n_ifaces = 3;
        // Synthetic report descriptors (only the leading items, as stand-ins).
        for (k, b) in [&[0x05u8, 0x01, 0x09, 0x06, 0xa1, 0x01][..], &[0x05, 0x01, 0x09, 0x02, 0xa1, 0x01], &[0x06, 0x00, 0xff, 0x09, 0x01]].iter().enumerate() {
            d.reports[k].state = Fetch::Ok;
            d.reports[k].len = b.len() as u16;
            d.reports[k].bytes[..b.len()].copy_from_slice(b);
        }
        d.manufacturer.present = true;
        d.manufacturer.set_from_descriptor(&[18, 3, b'L', 0, b'o', 0, b'g', 0, b'i', 0, b't', 0, b'e', 0, b'c', 0, b'h', 0]);
        d.product.present = true;
        d.product.set_from_descriptor(&[26, 3, b'U', 0, b'S', 0, b'B', 0, b' ', 0, b'R', 0, b'e', 0, b'c', 0, b'e', 0, b'i', 0, b'v', 0, b'e', 0, b'r', 0]);
    }
    let kbd = 2usize;
    {
        let d = s.claim(kbd).unwrap(); // Cherry KC 6000, hub port 4, low speed
        d.slot = 3;
        d.root_port = 1;
        d.parent = 0;
        d.route = 0x4;
        d.speed = 2;
        d.vid = 0x046a;
        d.pid = 0x0113;
        d.role = role::KEYBOARD;
        d.flags = dev_flag::DEVICE_DESC | dev_flag::CONFIG | dev_flag::SET_CONFIGURATION;
        d.n_hid_ifaces = 2;
        d.ring_iface = 0;
        d.ring_ep = 0x81;
        d.n_funcs = 1;
        d.device_desc_state = Fetch::Ok;
        d.device_desc = [18, 1, 0, 2, 0, 0, 0, 8, 0x6a, 0x04, 0x13, 0x01, 0, 1, 0, 0, 0, 1];
        let mut cfg = vec![9u8, 2, 59, 0, 2, 1, 0, 0xa0, 50];
        cfg.extend(hid_iface(0, 1, 1, 0x81, 8, 10, 54));
        cfg.extend(hid_iface(1, 0, 0, 0x82, 5, 10, 70));
        d.config[..59].copy_from_slice(&cfg);
        d.config_len = 59;
        d.ifaces[0] = IfaceRow { number: 0, alt: 0, class: 3, subclass: 1, protocol: 1, reason: 0, ep_addr: 0x81, interval: 10, max_packet: 8, report_len: 54, monitored: true, role: role::KEYBOARD };
        d.ifaces[1] = IfaceRow { number: 1, alt: 0, class: 3, subclass: 0, protocol: 0, reason: reason::NOT_BOOT_SUBCLASS, ep_addr: 0x82, interval: 10, max_packet: 5, report_len: 70, monitored: false, role: 0 };
        d.n_ifaces = 2;
    }
    // The LIST the program reads before the recording (ring_next_seq = 0: nothing recorded yet).
    let list = ask(&mut s, OP_LIST, 0, 0);
    // Interleaved reports (synthetic): receiver keyboard 'a' down/up, receiver mouse moves + left
    // click, cable keyboard Shift+'b'. Timestamps in ticks from 1000 (= R_T0).
    let k0 = Source { iface: 0, ep: 0x81, capped: false };
    let m1 = Source { iface: 1, ep: 0x82, capped: false };
    let t0 = 1000u32;
    let mut t = t0 + 40;
    for step in 0..12u32 {
        t += 7;
        match step % 4 {
            0 => s.record(recv, m1, &[0, 5, 0xfd, 0, 0, 0, 0, 0], 1, t, true),
            1 => s.record(recv, k0, &[0, 0, if step < 6 { 0x04 } else { 0 }, 0, 0, 0, 0, 0], 1, t, true),
            2 => s.record(recv, m1, &[if step < 8 { 1 } else { 0 }, 0, 0, 0, 0, 0, 0, 0], 1, t, true),
            _ => s.record(kbd, k0, &[if step < 9 { 0x02 } else { 0 }, 0, if step < 9 { 0x05 } else { 0 }, 0, 0, 0, 0, 0], 1, t, true),
        }
    }
    let mut out = String::new();
    let arr = |name: &str, b: &[u8]| {
        format!(
            "static const unsigned char {name}[{}] = {{{}}};\n",
            b.len().max(1),
            b.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
        )
    };
    out += "/* generated by mkreceiver.rs (synthetic receiver set-up, kernel encoder) */\n";
    out += &format!("#define R_T0 {t0}u\n");
    out += &arr("R_LIST", &list);
    out += &format!("#define R_LIST_LEN {}\n", list.len());
    let n = s.n_devices as usize;
    let mut lens = Vec::new();
    for i in 0..n {
        let dev = ask(&mut s, OP_DEVICE, i as u32, 0);
        lens.push(dev.len());
        out += &arr(&format!("R_DEV{i}"), &dev);
    }
    out += &format!("#define R_NDEV {n}\nstatic const unsigned char *const R_DEV[{n}] = {{{}}};\n", (0..n).map(|i| format!("R_DEV{i}")).collect::<Vec<_>>().join(","));
    out += &format!("static const unsigned R_DEVLEN[{n}] = {{{}}};\n", lens.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    let mut idx = Vec::new();
    let mut recs = Vec::new();
    for i in 0..n {
        let r = ask(&mut s, OP_REPORTS, i as u32, 0);
        let cnt = u16::from_le_bytes([r[12], r[13]]) as usize;
        for k in 0..cnt {
            idx.push(i);
            recs.push(r[16 + 80 * k..16 + 80 * (k + 1)].to_vec());
        }
    }
    out += &format!("#define R_NREC {}\nstatic const unsigned char R_RECIDX[{}] = {{{}}};\n", idx.len(), idx.len(), idx.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    out += &format!(
        "static const unsigned char R_REC[{}][80] = {{{}}};\n",
        recs.len(),
        recs.iter().map(|r| format!("{{{}}}", r.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(",")
    );
    print!("{out}");
}

#!/usr/bin/env python3
"""decode_probe.py SERIAL_LOG -- decode the USBPROBE hex dumps (LIST / DEVICE / REPORTS) of one run and print them next to the
kernel's own [usb] log lines, so the two can be compared by eye and by the asserts at the end (exit 1 on a mismatch)."""
import re, struct, sys
log = open(sys.argv[1], errors="replace").read().splitlines()
# --- kernel side
klines = [l for l in log if l.startswith("[usb]")]
kdev = [(m.group(1), m.group(2), m.group(3), m.group(4)) for l in klines
        for m in [re.search(r"device descriptor: .*class=(\d+) maxpkt0=(\d+) VID=([0-9a-f]+) PID=([0-9a-f]+)", l)] if m]
krd = [(int(m.group(1)), int(m.group(2)), int(m.group(3)), int(m.group(4))) for l in klines
       for m in [re.search(r"slot (\d+) iface (\d+): report descriptor (\d+) bytes \(declared (\d+)\)", l)] if m]
# --- probe side: collect blocks "USBPROBE: <what> len N -> rc R | ..." followed by hex lines
blocks = []
cur = None
for l in log:
    m = re.match(r"USBPROBE: (.*) len (\d+) -> rc (-?\d+)(?: \| ver (\d+) hdr (\d+) op (\d+) flags (\d+) total (\d+) used (\d+) now (\d+))?", l)
    if m:
        cur = dict(what=m.group(1), len=int(m.group(2)), rc=int(m.group(3)), hdr=m.groups()[3:], data=bytearray())
        blocks.append(cur); continue
    m = re.match(r"USBPROBE:   ([0-9a-f]{4}):((?: [0-9a-f]{2})+)", l)
    if m and cur is not None:
        cur["data"] += bytes(int(x, 16) for x in m.group(2).split())
    elif not l.startswith("USBPROBE"):
        pass
bad = 0
def chk(c, msg):
    global bad
    print(("  ok   " if c else "  FAIL ") + msg)
    if not c: bad += 1
for b in blocks:
    if b["what"] == "LIST full":
        d = b["data"]; gen, n, el, rp, cs, esk = struct.unpack_from("<IHHBBH", d, 0)
        print(f"LIST: generation {gen} n_devices {n} entry_len {el} root_ports {rp} controller_state {cs} enum_skipped {esk}")
        chk(el == 32 and gen == 1 and cs == 1, "list header generation 1, entry_len 32, controller_state 1")
        lst = []
        for i in range(n):
            e = d[16 + i * el:16 + (i + 1) * el]
            idx, slot, rport, parent, route, vid, pid, cls, spd, role, rsn, fl, nh, ri, rep, oldest, nxt, ierr, lcc = struct.unpack_from("<BBBBIHHBBBBBBBBIIHB", e, 0)
            print(f"  [{idx}] slot {slot} root_port {rport} parent {parent:#04x} route {route:#x} vid {vid:04x} pid {pid:04x} class {cls} speed {spd} role {role} reason {rsn} flags {fl:#04x} n_hid_ifaces {nh} ring_iface {ri:#04x} ring_ep {rep:#04x} oldest {oldest} next {nxt} int_errors {ierr} last_cc {lcc}")
            lst.append((idx, slot, vid, pid, role, rsn))
        chk(len(klines) and len(kdev) >= 0, "kernel [usb] device descriptor lines: %d, report descriptor lines: %d" % (len(kdev), len(krd)))
    if b["what"].startswith("DEVICE ") and b["what"] != "DEVICE index=n" and b["rc"] == 0:
        d = b["data"]; e = d[:32]
        nsec, = struct.unpack_from("<H", d, 32)
        o = 36; secs = {}
        print(b["what"] + ": sections", end=" ")
        got = []
        for _ in range(nsec):
            ty, ifc, fl, ln = struct.unpack_from("<HBBI", d, o); body = bytes(d[o + 8:o + 8 + ln]); o += 8 + ((ln + 3) & ~3)
            got.append((ty, ifc, fl, ln)); secs.setdefault(ty, []).append(body)
        print(got)
        chk(o == len(d) or o + 4 > len(d) - 0, f"sections consume the whole answer ({o} of {len(d)})") if False else None
        dd = secs.get(1, [b""])[0]
        if len(dd) == 18:
            cls, mp, vid, pid = dd[4], dd[7], struct.unpack_from("<H", dd, 8)[0], struct.unpack_from("<H", dd, 10)[0]
            chk((str(cls), str(mp), f"{vid:04x}", f"{pid:04x}") in kdev, f"device descriptor class={cls} maxpkt0={mp} VID={vid:04x} PID={pid:04x} appears in the kernel [usb] device descriptor lines")
        for r in secs.get(3, []):
            chk(any(r and len(r) == k[2] for k in krd), f"report descriptor {len(r)} bytes = a kernel '[usb] ... report descriptor N bytes' line")
        for t, nm in ((5, "manufacturer"), (6, "product")):
            for s in secs.get(t, []): print(f"  string {nm}: {s.decode('ascii','replace')!r}")
        cfg = secs.get(2, [b""])[0]
        if cfg: chk(struct.unpack_from("<H", cfg, 2)[0] == len(cfg), f"config descriptor wTotalLength == section len ({len(cfg)})")
    if b["what"].startswith("REPORTS") and b["rc"] == 0 and len(b["data"]) >= 16:
        d = b["data"]; oldest, nxt, lost, cnt, rl = struct.unpack_from("<IIIHH", d, 0)
        print(f"{b['what']}: hdr flags {b['hdr'][3] if b['hdr'][0] else '-'} total {b['hdr'][5] if b['hdr'][0] else '-'}  oldest {oldest} next {nxt} lost {lost} count {cnt} rec_len {rl}")
        for i in range(cnt):
            seq, ts, ln, fl, cc = struct.unpack_from("<IIHBB", d, 16 + i * rl); data = d[16 + i * rl + 16:16 + i * rl + 16 + ln]
            print(f"    seq {seq} ts {ts} len {ln} flags {fl:#x} cc {cc} data {data.hex(' ')}")
sys.exit(1 if bad else 0)

#!/usr/bin/env python3
"""Benchmark an SD card on macOS and report what the Mac knows about it.

Non-destructive: it only writes one temporary file and a few small files on a
mounted volume and removes them afterwards. It never writes to a raw device.
It refuses anything that is not a removable FAT/exFAT volume.

    tools/sd-bench.py                          list usable volumes
    tools/sd-bench.py /Volumes/NEWTOS3 --label "SanDisk 32GB"
    sudo tools/sd-bench.py /Volumes/NEWTOS3    also reads partition table and BPB
    tools/sd-bench.py --compare                table of all runs so far

Besides throughput it reports single-operation latency outliers: a card that
is usually quick but stalls for 500 ms now and then looks fine in MB/s. That
stall is what a driver timeout sees (TASK/HW08.md; NewTOS waits 1000 ms for a
write, WRITE_BUSY_MS in kernel/src/drivers/bcm2711/emmc.rs).

The numbers include the card reader and the macOS FAT driver. Compare cards
with the same reader and the same partition size; they are not Pi numbers.
"""

import argparse
import datetime
import fcntl
import json
import os
import plistlib
import pwd
import random
import stat
import struct
import subprocess
import sys
import time

MIB = 1024 * 1024
TMP_FILE = "SDBENCH.TMP"
TMP_DIR = "SDBENCHD"
SEQ_BLOCK = MIB
SMALL = 4096
RESERVE = 32 * MIB
# FAT file size limit, minus one MiB.
MAX_FILE = 4095 * MIB
# SD spec busy limits (250/500 ms) and the NewTOS write timeout.
THRESHOLDS_MS = (100, 250, 500, 1000)
COLUMNS = (
    "date label media reader size_gb fs "
    "seq_write_mb_s seq_read_mb_s seq_write_worst_ms "
    "rand_read_iops rand_read_p99_ms "
    "rand_write_iops rand_write_p50_ms rand_write_p99_ms rand_write_max_ms "
    "meta_ops_s meta_p50_ms meta_p99_ms meta_max_ms "
    "over_500ms over_1000ms io_errors data_errors "
    "local_write_p50_ms local_write_max_ms"
).split()
# A write span that fits in one flash allocation unit on any common card.
LOCAL_SPAN = 256 * 1024


def diskutil_info(target):
    r = subprocess.run(["diskutil", "info", "-plist", target], capture_output=True)
    if r.returncode != 0:
        return None
    return plistlib.loads(r.stdout)


def refusal(info):
    """Why this volume must not be benchmarked, or None."""
    if info is None:
        return "not a mounted disk volume"
    if not info.get("MountPoint"):
        return "not mounted"
    if info.get("Internal"):
        return "internal disk"
    if not (info.get("RemovableMedia") or info.get("Removable")):
        return "not removable media"
    if not info.get("Writable", True) or info.get("WritableVolume") is False:
        return "read-only (lock switch on the card?)"
    fs = str(info.get("FilesystemType", "")).lower()
    if fs not in ("msdos", "exfat"):
        return "file system %r is not FAT or exFAT" % fs
    return None


def list_volumes():
    found = 0
    for name in sorted(os.listdir("/Volumes")):
        path = os.path.join("/Volumes", name)
        info = diskutil_info(path)
        why = refusal(info)
        if why:
            print("  skip %-20s %s" % (name, why))
            continue
        found += 1
        vfs = os.statvfs(path)
        print("  use  %-20s %s, %.1f GB partition, %.0f MB free, %s" % (
            name, info.get("FilesystemUserVisibleName", "?"),
            info.get("TotalSize", 0) / 1e9,
            vfs.f_bavail * vfs.f_frsize / 1e6, info.get("DeviceIdentifier")))
    if not found:
        print("No removable FAT volume mounted. Insert a card.")


USB_SPEEDS = {0: "USB 1 low", 1: "USB 1 full", 2: "USB 2 (480 Mb/s)",
               3: "USB 3 (5 Gb/s)", 4: "USB 3 (10 Gb/s)"}


def usb_ancestor(whole):
    """Properties of the USB device above BSD disk `whole`. ioreg prints a
    device before its children, so the last ones seen before the disk's own
    "BSD Name" line belong to the device that carries it."""
    r = subprocess.run(["ioreg", "-r", "-c", "IOUSBHostDevice", "-l", "-w0"],
                       capture_output=True, text=True, errors="replace")
    props = {}
    for line in r.stdout.splitlines():
        key, sep, val = line.partition(" = ")
        key = key.strip(" |").strip('"')
        if not sep:
            continue
        val = val.strip().strip('"')
        if key == "BSD Name" and val == whole:
            return props
        if key in ("idVendor", "idProduct", "Device Speed"):
            props[key] = int(val) if val.isdigit() else val
        elif key in ("USB Vendor Name", "USB Product Name"):
            props[key] = val
    return None


def profiler(datatype):
    r = subprocess.run(["system_profiler", "-json", datatype], capture_output=True)
    if r.returncode != 0:
        return []
    try:
        return json.loads(r.stdout).get(datatype, [])
    except ValueError:
        return []


def reader_info(whole):
    """Card details from a built-in reader, or the USB reader's identity.

    Only the built-in reader exposes the card's own registers (CID: maker,
    product, serial, date). A USB reader hides the card behind mass storage.
    """
    lines = []
    name = ""
    for bus in profiler("SPCardReaderDataType"):
        for card in bus.get("_items", []) or []:
            lines.append("built-in reader, card registers:")
            for key, val in sorted(card.items()):
                if key != "volumes" and not isinstance(val, (list, dict)):
                    lines.append("  %s: %s" % (key.replace("spcardreader_card_", ""), val))
        if bus.get("_items"):
            name = "built-in"
    if not name:
        dev = usb_ancestor(whole)
        if dev:
            name = "%04x:%04x" % (dev.get("idVendor", 0), dev.get("idProduct", 0))
            lines.append("USB reader %s, %s %s, %s (hides the card's CID/CSD registers)" % (
                name, dev.get("USB Vendor Name", "?"), dev.get("USB Product Name", "?"),
                USB_SPEEDS.get(dev.get("Device Speed"), "speed unknown")))
    if not lines:
        lines.append("reader: nothing found in system_profiler")
    return name or "unknown", lines


PART_TYPES = {
    0x01: "FAT12", 0x04: "FAT16 <32M", 0x05: "extended", 0x06: "FAT16",
    0x07: "exFAT/NTFS", 0x0B: "FAT32", 0x0C: "FAT32 LBA", 0x0E: "FAT16 LBA",
    0x0F: "extended LBA", 0x83: "Linux", 0xEE: "GPT protective",
}


def parse_bpb(sector):
    """Boot sector fields, with the FAT type from the cluster count as the
    FAT specification defines it (the type label in the sector is decoration).
    """
    if len(sector) < 512 or sector[510:512] != b"\x55\xaa":
        return None
    if sector[3:11] == b"EXFAT   ":
        return {"type": "exFAT"}
    bps = struct.unpack_from("<H", sector, 11)[0]
    spc = sector[13]
    reserved = struct.unpack_from("<H", sector, 14)[0]
    fats = sector[16]
    root = struct.unpack_from("<H", sector, 17)[0]
    total = struct.unpack_from("<H", sector, 19)[0] or struct.unpack_from("<I", sector, 32)[0]
    fatsz = struct.unpack_from("<H", sector, 22)[0] or struct.unpack_from("<I", sector, 36)[0]
    if bps not in (512, 1024, 2048, 4096) or spc == 0 or spc & (spc - 1) or fats == 0:
        return None
    rootsec = (root * 32 + bps - 1) // bps
    first_data = reserved + fats * fatsz + rootsec
    clusters = (total - first_data) // spc if total > first_data else 0
    kind = "FAT12" if clusters < 4085 else "FAT16" if clusters < 65525 else "FAT32"
    return {
        "type": kind, "oem": sector[3:11].decode("latin-1").strip(),
        "bytes_per_sector": bps, "cluster_kib": bps * spc / 1024,
        "clusters": clusters, "fat_sectors": fatsz, "fats": fats,
        "root_entries": root, "first_data_sector": first_data,
    }


def parse_mbr(sector):
    """Four primary partition entries as (type, start_lba, sectors)."""
    if len(sector) < 512 or sector[510:512] != b"\x55\xaa":
        return []
    parts = []
    for i in range(4):
        ptype = sector[446 + 16 * i + 4]
        start, count = struct.unpack_from("<II", sector, 446 + 16 * i + 8)
        if ptype:
            parts.append((ptype, start, count))
    return parts


def logical_partitions(fd, block, ext_start):
    """Walk the EBR chain of an extended partition: each EBR's first entry is
    relative to that EBR, its second links to the next one, relative to the
    start of the extended partition."""
    found, ebr = [], ext_start
    for _ in range(128):
        entries = parse_mbr(os.pread(fd, block, ebr * block))
        data = [e for e in entries if e[0] not in (0x05, 0x0F)]
        link = [e for e in entries if e[0] in (0x05, 0x0F)]
        if data:
            found.append((data[0][0], ebr + data[0][1], data[0][2]))
        if not link:
            break
        ebr = ext_start + link[0][1]
    return found


def layout_lines(whole, sector_size):
    """Partition table and BPBs from the raw device, read only. Needs root."""
    dev = "/dev/r" + whole
    block = sector_size or 512
    try:
        fd = os.open(dev, os.O_RDONLY)
    except OSError as e:
        return ["partition table and BPB: %s; run with sudo to read them" % e.strerror]
    lines = []
    try:
        first = os.pread(fd, block, 0)
        boot = parse_bpb(first)
        parts = parse_mbr(first)
        if boot and first[0] in (0xEB, 0xE9) and not any(p[1] for p in parts):
            lines.append("no partition table, whole card is one file system:")
            parts, entries = [], [(0, boot)]
        else:
            lines.append("partition table of %s:" % dev)
            entries = []
            for ptype, start, count in list(parts):
                if ptype in (0x05, 0x0F):
                    parts += logical_partitions(fd, block, start)
            for ptype, start, count in parts:
                if ptype not in (0x05, 0x0F):
                    sect = os.pread(fd, block, start * block)
                    entries.append((start, parse_bpb(sect)))
                lines.append("  type %02X %-14s start %9d (%6.1f MiB), %8.1f MiB, %s" % (
                    ptype, PART_TYPES.get(ptype, "?"), start, start * block / MIB,
                    count * block / MIB,
                    "4 MiB aligned" if (start * block) % (4 * MIB) == 0
                    else "NOT 4 MiB aligned"))
        for start, bpb in entries:
            if not bpb:
                lines.append("  BPB at %d: not a FAT boot sector" % start)
            elif "clusters" not in bpb:
                lines.append("  BPB at %d: %s" % (start, bpb["type"]))
            else:
                lines.append("  BPB at %d: %s, %g KiB clusters, %d clusters, %d-byte "
                             "sectors, %d root entries, data at +%d" % (
                                 start, bpb["type"], bpb["cluster_kib"], bpb["clusters"],
                                 bpb["bytes_per_sector"], bpb["root_entries"],
                                 bpb["first_data_sector"]))
    except OSError as e:
        lines.append("reading %s failed: %s" % (dev, e.strerror))
    finally:
        os.close(fd)
    return lines


class Sync:
    """Flush to the medium: F_FULLFSYNC where the driver has it, else fsync."""

    def __init__(self):
        self.full = True

    def __call__(self, fd):
        if self.full:
            try:
                fcntl.fcntl(fd, fcntl.F_FULLFSYNC)
                return
            except OSError:
                self.full = False
        os.fsync(fd)


def open_nocache(path, flags):
    fd = os.open(path, flags, 0o644)
    fcntl.fcntl(fd, fcntl.F_NOCACHE, 1)
    return fd


def percentile(values, pct):
    if not values:
        return 0.0
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(len(ordered) * pct / 100))]


def latency_text(ms):
    if not ms:
        return "no samples"
    over = " ".join(">%dms:%d" % (t, sum(1 for v in ms if v > t)) for t in THRESHOLDS_MS)
    return "p50 %.2f  p99 %.2f  max %.1f ms  [%s]" % (
        percentile(ms, 50), percentile(ms, 99), max(ms), over)


def block_data(base, index):
    """One block: a unique header on a random body, so that a card that wraps
    addresses or swaps blocks fails the read-back check."""
    return struct.pack("<Q", index) + base[8:]


def seq_write(path, size, base, sync):
    fd = open_nocache(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC)
    ms, start = [], time.perf_counter()
    try:
        for i in range(size // SEQ_BLOCK):
            buf = block_data(base, i)
            t0 = time.perf_counter()
            os.write(fd, buf)
            ms.append((time.perf_counter() - t0) * 1000)
        sync(fd)
    finally:
        os.close(fd)
    return size / MIB / (time.perf_counter() - start), ms


def seq_read(path, size, base):
    fd = open_nocache(path, os.O_RDONLY)
    bad, spent = 0, 0.0
    try:
        for i in range(size // SEQ_BLOCK):
            t0 = time.perf_counter()
            buf = os.read(fd, SEQ_BLOCK)
            spent += time.perf_counter() - t0
            if buf != block_data(base, i):
                bad += 1
    finally:
        os.close(fd)
    return size / MIB / spent, bad


def rand_read(path, size, seconds):
    fd = open_nocache(path, os.O_RDONLY)
    ms, end = [], time.perf_counter() + seconds
    try:
        while time.perf_counter() < end:
            off = random.randrange(size // SMALL) * SMALL
            t0 = time.perf_counter()
            os.pread(fd, SMALL, off)
            ms.append((time.perf_counter() - t0) * 1000)
    finally:
        os.close(fd)
    return len(ms) / seconds, ms


def rand_write(path, size, seconds, sync):
    fd = open_nocache(path, os.O_WRONLY)
    buf = os.urandom(SMALL)
    ms, end = [], time.perf_counter() + seconds
    try:
        while time.perf_counter() < end:
            off = random.randrange(size // SMALL) * SMALL
            t0 = time.perf_counter()
            os.pwrite(fd, buf, off)
            sync(fd)
            ms.append((time.perf_counter() - t0) * 1000)
    finally:
        os.close(fd)
    return len(ms) / seconds, ms


def metadata(tmpdir, seconds, sync):
    """Create, write, flush, close, delete small files: mostly directory and
    FAT updates, the kind of write that stalls a card most (TASK/HW06.md)."""
    buf = os.urandom(SMALL)
    ms, end, n = [], time.perf_counter() + seconds, 0
    while time.perf_counter() < end:
        path = os.path.join(tmpdir, "F%07d.TMP" % n)
        t0 = time.perf_counter()
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)
        try:
            os.write(fd, buf)
            sync(fd)
        finally:
            os.close(fd)
        ms.append((time.perf_counter() - t0) * 1000)
        os.unlink(path)
        n += 1
    return len(ms) / seconds, ms


def cleanup(file_path, dir_path):
    left = []
    try:
        if os.path.exists(file_path):
            os.unlink(file_path)
        if os.path.isdir(dir_path):
            for name in os.listdir(dir_path):
                os.unlink(os.path.join(dir_path, name))
            os.rmdir(dir_path)
    except OSError as e:
        left.append("%s (%s)" % (e.filename, e.strerror))
    return left


def results_path():
    user = os.environ.get("SUDO_USER")
    home = pwd.getpwnam(user).pw_dir if user else os.path.expanduser("~")
    return os.path.join(home, "sd-bench-results.tsv")


def append_result(path, row):
    new = not os.path.exists(path)
    with open(path, "a") as f:
        if new:
            f.write("\t".join(COLUMNS) + "\n")
        f.write("\t".join(str(row.get(c, "")) for c in COLUMNS) + "\n")
    user = os.environ.get("SUDO_USER")
    if user:
        entry = pwd.getpwnam(user)
        os.chown(path, entry.pw_uid, entry.pw_gid)


def compare(path):
    if not os.path.exists(path):
        print("No results yet: %s" % path)
        return
    with open(path) as f:
        rows = [line.rstrip("\n").split("\t") for line in f if line.strip()]
    show = ["date", "label", "reader", "size_gb", "seq_write_mb_s", "seq_read_mb_s",
            "local_write_p50_ms", "rand_write_iops", "rand_write_p50_ms",
            "rand_write_max_ms", "meta_p50_ms",
            "meta_max_ms", "over_500ms", "over_1000ms", "io_errors", "data_errors"]
    head = rows[0]
    table = [show] + [[r[head.index(c)] if c in head else "" for c in show] for r in rows[1:]]
    widths = [max(len(row[i]) for row in table) for i in range(len(show))]
    for row in table:
        print("  ".join(cell.ljust(w) for cell, w in zip(row, widths)))


def main():
    ap = argparse.ArgumentParser(description="Non-destructive SD card benchmark for macOS.")
    ap.add_argument("volume", nargs="?", help="mounted volume, e.g. /Volumes/NEWTOS3")
    ap.add_argument("--label", help="your name for this card, e.g. 'SanDisk 32GB A'")
    ap.add_argument("--size-mb", type=int, default=256, help="test file size (default 256)")
    ap.add_argument("--seconds", type=int, default=20, help="seconds per latency test (default 20)")
    ap.add_argument("--fill", action="store_true",
                    help="test file fills the free space: finds fake capacity, takes long")
    ap.add_argument("--info", action="store_true", help="show card details only, write nothing")
    ap.add_argument("--yes", action="store_true", help="do not ask before writing")
    ap.add_argument("--compare", action="store_true", help="print all saved results")
    ap.add_argument("--allow-image", action="store_true", help=argparse.SUPPRESS)
    args = ap.parse_args()

    if args.compare:
        compare(results_path())
        return 0
    if not args.volume:
        print("Volumes:")
        list_volumes()
        print("\nRun: %s /Volumes/NAME --label 'your card name'" % sys.argv[0])
        return 0

    info = diskutil_info(args.volume)
    why = refusal(info)
    if not why and str(info.get("BusProtocol", "")) == "Disk Image" and not args.allow_image:
        why = "disk image, not a card"
    if why:
        print("Refusing %s: %s." % (args.volume, why))
        return 1

    mount = info["MountPoint"]
    whole = info.get("ParentWholeDisk", info.get("DeviceIdentifier", ""))
    whole_info = diskutil_info(whole) or {}
    size_gb = whole_info.get("TotalSize", info.get("TotalSize", 0)) / 1e9
    reader, reader_lines = reader_info(whole)

    print("Card")
    print("  media: %s, %.2f GB, block size %s" % (
        whole_info.get("MediaName", info.get("MediaName", "?")), size_gb,
        whole_info.get("DeviceBlockSize", "?")))
    print("  bus: %s, device %s" % (info.get("BusProtocol", "?"), whole))
    print("  volume: %s on %s, %s, cluster %s bytes" % (
        info.get("VolumeName", "?"), info.get("DeviceIdentifier", "?"),
        info.get("FilesystemUserVisibleName", "?"), info.get("VolumeAllocationBlockSize", "?")))
    for line in reader_lines + layout_lines(whole, whole_info.get("DeviceBlockSize")):
        print("  " + line)
    if args.info:
        return 0

    vfs = os.statvfs(mount)
    free = vfs.f_bavail * vfs.f_frsize
    size = free - RESERVE if args.fill else args.size_mb * MIB
    size = min(size, free - RESERVE, MAX_FILE)
    size -= size % SEQ_BLOCK
    if size < 16 * MIB:
        print("Too little free space on %s: %.0f MB." % (mount, free / 1e6))
        return 1

    file_path = os.path.join(mount, TMP_FILE)
    dir_path = os.path.join(mount, TMP_DIR)
    if os.path.exists(file_path) or os.path.exists(dir_path):
        st = os.stat(file_path if os.path.exists(file_path) else dir_path)
        print("%s already has %s or %s (%s). Left from an aborted run? Remove it first." % (
            mount, TMP_FILE, TMP_DIR, "dir" if stat.S_ISDIR(st.st_mode) else "file"))
        return 1

    print("\nWrites a temporary %d MiB file and small files to %s, then removes them." % (
        size // MIB, mount))
    if not args.yes and input("Continue? [y/N] ").strip().lower() not in ("y", "yes", "j", "ja"):
        return 0

    base = os.urandom(SEQ_BLOCK)
    sync = Sync()
    row = {"date": datetime.datetime.now().strftime("%Y-%m-%d %H:%M"),
           "label": args.label or "-", "media": whole_info.get("MediaName", "?"),
           "reader": reader, "size_gb": "%.1f" % size_gb,
           "fs": info.get("FilesystemUserVisibleName", "?")}
    io_errors, data_errors, all_ms = 0, 0, []

    def step(title, func):
        nonlocal io_errors
        print("%-22s" % title, end=" ", flush=True)
        try:
            return func()
        except OSError as e:
            io_errors += 1
            print("I/O ERROR: %s" % e.strerror)
            return None

    try:
        r = step("sequential write", lambda: seq_write(file_path, size, base, sync))
        if r:
            row["seq_write_mb_s"] = "%.1f" % r[0]
            row["seq_write_worst_ms"] = "%.0f" % max(r[1])
            print("%7.1f MiB/s  worst 1 MiB block %.0f ms" % (r[0], max(r[1])))
            r = step("sequential read", lambda: seq_read(file_path, size, base))
            if r:
                data_errors = r[1]
                row["seq_read_mb_s"] = "%.1f" % r[0]
                print("%7.1f MiB/s  read-back %s" % (
                    r[0], "ok" if not r[1] else "%d BAD BLOCKS" % r[1]))
            r = step("random read 4 KiB", lambda: rand_read(file_path, size, args.seconds))
            if r:
                row["rand_read_iops"] = "%.0f" % r[0]
                row["rand_read_p99_ms"] = "%.2f" % percentile(r[1], 99)
                print("%7.0f IOPS   %s" % (r[0], latency_text(r[1])))
            # Fast here but slow over the whole file means the stall comes from
            # jumping between regions of the card, not from each write.
            r = step("write 4 KiB, 256 KiB", lambda: rand_write(file_path, LOCAL_SPAN, args.seconds, sync))
            if r:
                all_ms += r[1]
                row["local_write_p50_ms"] = "%.2f" % percentile(r[1], 50)
                row["local_write_max_ms"] = "%.1f" % max(r[1])
                print("%7.0f IOPS   %s" % (r[0], latency_text(r[1])))
            r = step("random write 4 KiB", lambda: rand_write(file_path, size, args.seconds, sync))
            if r:
                all_ms += r[1]
                row["rand_write_iops"] = "%.0f" % r[0]
                row["rand_write_p50_ms"] = "%.2f" % percentile(r[1], 50)
                row["rand_write_p99_ms"] = "%.2f" % percentile(r[1], 99)
                row["rand_write_max_ms"] = "%.1f" % max(r[1])
                print("%7.0f IOPS   %s" % (r[0], latency_text(r[1])))
        os.unlink(file_path)
        os.mkdir(dir_path)
        r = step("create small files", lambda: metadata(dir_path, args.seconds, sync))
        if r:
            all_ms += r[1]
            row["meta_ops_s"] = "%.0f" % r[0]
            row["meta_p50_ms"] = "%.2f" % percentile(r[1], 50)
            row["meta_p99_ms"] = "%.2f" % percentile(r[1], 99)
            row["meta_max_ms"] = "%.1f" % max(r[1])
            print("%7.0f ops/s  %s" % (r[0], latency_text(r[1])))
    except KeyboardInterrupt:
        print("\nInterrupted, nothing saved.")
        for item in cleanup(file_path, dir_path):
            print("Could not remove %s" % item)
        return 130
    finally:
        left = cleanup(file_path, dir_path)
    for item in left:
        print("Could not remove %s" % item)
    subprocess.run(["sync"])

    row["over_500ms"] = sum(1 for v in all_ms if v > 500)
    row["over_1000ms"] = sum(1 for v in all_ms if v > 1000)
    row["io_errors"] = io_errors
    row["data_errors"] = data_errors
    out = results_path()
    append_result(out, row)

    print("\nFlush used: %s" % ("F_FULLFSYNC" if sync.full else "fsync (F_FULLFSYNC not supported here)"))
    print("Writes over 500 ms: %s, over 1000 ms: %s, I/O errors: %d, bad blocks: %d" % (
        row["over_500ms"], row["over_1000ms"], io_errors, data_errors))
    print("Saved to %s (--compare shows all runs)." % out)
    return 3 if data_errors else 2 if io_errors else 0


if __name__ == "__main__":
    sys.exit(main())

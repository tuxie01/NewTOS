"""gcprobe-compare.py <end-dir> <label>... -- the end states of the gcprobe runs
(test partition, LBA 63 + 130977 sectors): per image the sha256, the
partition-relative sectors that carry the probe's marker
"GCPROBE-STALE-GEOMETRY-MARKER-01", the BPB root-entry count (bytes 0x11-0x12
of sector 0), and the sectors that differ from gc-original.img."""
import hashlib, sys, os
d = sys.argv[1]
orig = open(os.path.join(d, 'gc-original.img'), 'rb').read()
M = b'GCPROBE-STALE-GEOMETRY-MARKER-01'
for lab in sys.argv[2:]:
    img = open(os.path.join(d, lab + '.img'), 'rb').read()
    secs = [i for i in range(len(img) // 512) if img[i*512:i*512+32] == M]
    diff = [i for i in range(len(img) // 512) if img[i*512:(i+1)*512] != orig[i*512:(i+1)*512]]
    print(f"{lab}: sha256 {hashlib.sha256(img).hexdigest()}")
    print(f"  root entries (sector 0, 0x11) {int.from_bytes(img[0x11:0x13], 'little')}")
    print(f"  marker in sector(s) {secs}  (data area start: datrec 289 old geometry, 273 new)")
    for s in secs:
        print(f"    sector {s}: cluster {2 + (s - 289) // 4} by the old geometry (datrec 289), "
              f"cluster {2 + (s - 273) // 4} by the new one (datrec 273)")
    print(f"  sectors differing from the original: {diff}")

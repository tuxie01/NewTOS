# ide-commands.py <trace.txt> -- condense a Hatari "--trace ide,bios,gemdos" file:
# every IDE command with its drive and LBA (ATA task-file registers as the trace
# shows them written: Sector Number = LBA 7..0, Cylinder Low = 15..8, Cylinder
# High = 23..16, Device/Head bits 3..0 = 27..24, bit 4 = drive 0 master / 1
# slave, bit 6 = LBA mode), interleaved with the BIOS and GEMDOS lines and
# Hatari's own GEMDOS routing lines ("-> to TOS", "-> FD 64", "GEMDOS: x ->
# host"). Console BIOS calls are dropped. After every READ command the first
# four data words the CPU read from the data register ($f00000/$f00002) are
# printed, so the byte order Hatari delivers can be compared with the image
# file (`xxd -s <lba*512> -l 8`). Run with python3 -I.
import re, sys
CMDS = {0x20: 'READ SECTORS', 0xC4: 'READ MULTIPLE', 0x30: 'WRITE SECTORS',
        0xC5: 'WRITE MULTIPLE', 0xEC: 'IDENTIFY DEVICE', 0xC6: 'SET MULTIPLE MODE',
        0xE7: 'FLUSH CACHE'}
QUIET = ('Bconstat', 'Bconin', 'Bconout', 'Bcostat', 'Kbshift', 'Setexc', 'Tickcal')
reg = {}
words = None    # data words of the current READ, None outside a READ
w = re.compile(r"IDE: write addr=0x([0-7]) reg='([^']+)' val=0x([0-9a-f]+)")
d = re.compile(r"IDE: wget\(\$f0000[02]\) = \$([0-9a-f]{4})")
for line in open(sys.argv[1], errors='replace'):
    line = line.rstrip('\n')
    m = d.match(line)
    if m and words is not None:
        words.append(m.group(1))
        if len(words) == 4:
            print(f"     first data words: {' '.join(words)}")
            words = None
        continue
    m = w.match(line)
    if m:
        name, val = m.group(2), int(m.group(3), 16)
        reg[name] = val
        if name == 'Command':
            dh = reg.get('Device/Head', 0)
            lba = (((dh & 0x0F) << 24) | (reg.get('Cylinder High', 0) << 16)
                   | (reg.get('Cylinder Low', 0) << 8) | reg.get('Sector Number', 0))
            print(f"IDE  {CMDS.get(val, hex(val)):18s} drive {(dh >> 4) & 1} "
                  f"{'LBA' if dh & 0x40 else 'CHS'} {lba:7d} count {reg.get('Sector Count', 0)}")
            words = [] if val in (0x20, 0xC4) else None
        continue
    if line.startswith(('BIOS ', 'GEMDOS ', '-> ', 'GEMDOS: ', 'GEMDOS didn')):
        if not any(q in line for q in QUIET):
            print(line)

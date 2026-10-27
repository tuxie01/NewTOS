# Generates tosdump/cases/bios-r1.tsv (round 1 of the BIOS + XBIOS table).
import sys
H = r'''# bios-r1.tsv -- DUMPBIOS round 1: the BIOS (trap #13) and XBIOS (trap #14)
# queries, against EmuTOS 1a7484ab (references/emutos, docs/analysis/ORACLE.md).
#
# Form (docs/TOS-REFERENCE-DUMPER.md section 6, "The call table, as built for
# DUMPBIOS"; tosdump/core/cases.h "Call tables"):
#
#   trap:function <TAB> args <TAB> retclass <TAB> capture <TAB> comment
#
#   args      w:<v> WORD, l:<v> LONG, b:<n> the dumper's buffer n (0, 1)
#   retclass  EXACT IGNORE NONZERO HANDLE PTR_RAM, ".W"/".B" = the width of
#             the EmuTOS handler's return type (no suffix: LONG)
#   capture   <argindex>:<length>[.B|.W|.L][@<offset>]
#
# ROUND 1 = QUERIES ONLY.  Every line is a call whose effect does not outlive
# it, in the arm that only reads -- Kbshift(-1), Setcolor(n,-1), Rsconf(-1..)
# -- and the guard in tosdump/core/cases.c refuses anything else.  The
# ledger with every one of the 12 BIOS and 142 XBIOS slots, its status and
# the reason: tosdump/cases/COVERAGE-BIOS.md.  Each comment cites the EmuTOS
# handler the function number reaches on the pin (bios/bios.c:1252-1265,
# bios/xbios.c:1285-1515 are the two dispatch tables).
#
# Measured on the oracle's TT profile (tools/hatari-oracle.sh tt): 4 MiB
# ST-RAM, 16 MiB TT-RAM, TT-Medium, no blitter, no DSP, no Falcon video.
# Where a call is profile dependent the comment says so.
#
# One table per round: a case id is the physical line number and the dump
# header carries the CRC of this file, so it is frozen once measured.
'''
rows = []
def sec(title):
    rows.append(None); rows.append('# ---- ' + title)
def r(call, args, rc, cap, comment):
    rows.append('\t'.join([call, args, rc, cap, comment]))

sec('BIOS 1 Bconstat(dev) -- LONG bconstat, bios/bios.c:914')
for d, n in [(0,'PRT'),(1,'AUX'),(2,'CON'),(3,'MIDI'),(4,'IKBD'),(5,'RAWCON'),(6,'Bconmap 6'),(10,'past the map'),(-1,'negative')]:
    r('13:1', 'w:%d' % d, 'EXACT', '-', 'Bconstat(%d) %s -- bios/bios.c:914' % (d, n))
sec('BIOS 2 Bconin(dev) -- only the handles that return at once, bios/bios.c:958')
for d, n in [(4,'IKBD char_dummy'),(5,'RAWCON char_dummy'),(10,'past the map'),(-1,'negative')]:
    r('13:2', 'w:%d' % d, 'EXACT', '-', 'Bconin(%d) %s -- bios/bios.c:958' % (d, n))
sec('BIOS 3 Bconout(dev,c) -- printable byte to CON / raw CON, no device; bios/bios.c:991')
r('13:3', 'w:2,w:0x2A', 'EXACT', '-', "Bconout(2,'*') CON -- bios/bios.c:991, chardev.c:91")
r('13:3', 'w:5,w:0x2A', 'EXACT', '-', "Bconout(5,'*') raw CON -- bios/bios.c:991, chardev.c:98")
r('13:3', 'w:10,w:0x41', 'EXACT', '-', "Bconout(10,'A') past the map -- bios/bios.c:1001-1003")
r('13:3', 'w:-1,w:0x41', 'EXACT', '-', "Bconout(-1,'A') negative -- bios/bios.c:1008-1010")
sec('BIOS 5 Setexc(n,-1) -- the inquiry, LONG setexc, bios/bios.c:1070')
for v, n in [(2,'bus error'),(0x21,'trap #1'),(0x100,'etv_timer'),(0x101,'etv_critic'),(0x102,'etv_term')]:
    r('13:5', 'w:0x%X,l:-1' % v, 'NONZERO', '-', 'Setexc(0x%X,-1) %s -- bios/bios.c:1070' % (v, n))
sec('BIOS 6 Tickcal -- LONG tickcal, bios/bios.c:1096')
r('13:6', '-', 'EXACT', '-', 'Tickcal() -- bios/bios.c:1096 returns 20')
sec('BIOS 8 Bcostat(dev) -- LONG bcostat, bios/bios.c:1143 (3/4 swapped, chardev.c:38-42)')
for d, n in [(0,'PRT'),(1,'AUX'),(2,'CON'),(3,'IKBD slot'),(4,'MIDI slot'),(5,'RAWCON'),(6,'Bconmap 6'),(7,'Bconmap 7'),(-1,'negative')]:
    r('13:8', 'w:%d' % d, 'EXACT', '-', 'Bcostat(%d) %s -- bios/bios.c:1143' % (d, n))
sec('BIOS 10 Drvmap -- blkdev_drvmap, bios/blkdev.c:833; the drive set is the machine\'s')
r('13:10', '-', 'NONZERO', '-', 'Drvmap() -- bios/bios.c:1206, blkdev.c:833')
sec('BIOS 11 Kbshift(-1) -- the inquiry, LONG kbshift, bios/ikbd.c:145')
r('13:11', 'w:-1', 'EXACT', '-', 'Kbshift(-1) -- bios/ikbd.c:145')
sec('BIOS past the table: d0 keeps the number, bios/vectors.S:1323-1326')
for f in (12, 0x7F, 0xFF):
    r('13:%d' % f, '-', 'EXACT', '-', 'BIOS %d past bios_ent -- bios/vectors.S:1325-1326' % f)

sec('XBIOS 2-4 Physbase Logbase Getrez -- bios/screen.c:1042,1071,1076')
r('14:0x02', '-', 'PTR_RAM', '-', 'Physbase() -- bios/screen.c:1042')
r('14:0x03', '-', 'PTR_RAM', '-', 'Logbase() -- bios/screen.c:1071')
r('14:0x04', '-', 'EXACT.W', '-', 'Getrez() -- WORD getrez, bios/screen.c:1076 (TT-Medium: profile)')
sec('XBIOS 5 Setscreen(-1,-1,-1) -- rez -1 returns 0 at once, bios/screen.c:1115-1117')
r('14:0x05', 'l:-1,l:-1,w:-1', 'EXACT.W', '-', 'Setscreen(-1,-1,-1) -- WORD setscreen, bios/screen.c:1102')
sec('XBIOS 7 Setcolor(n,-1) -- the inquiry, WORD atari_setcolor, bios/screen.c:1192,1015')
for n in (0, 1, 15):
    r('14:0x07', 'w:%d,w:-1' % n, 'EXACT.W', '-', 'Setcolor(%d,-1) -- bios/screen.c:1192 (palette: profile)' % n)
sec('XBIOS 14 Iorec(dev) -- LONG iorec, bios/xbios.c:336')
for d, n, c in [(0,'RS-232','PTR_RAM'),(1,'IKBD','PTR_RAM'),(2,'MIDI','PTR_RAM'),(3,'invalid','EXACT'),(-1,'invalid','EXACT')]:
    r('14:0x0E', 'w:%d' % d, c, '-', 'Iorec(%d) %s -- bios/xbios.c:336' % (d, n))
sec('XBIOS 15 Rsconf inquiries -- rsconf_mfp, bios/serport.c:217-229')
r('14:0x0F', 'w:-1,w:-1,w:-1,w:-1,w:-1,w:-1', 'EXACT', '-', 'Rsconf(-1 x6) old ucr/rsr/tsr -- bios/serport.c:226')
r('14:0x0F', 'w:-2,w:-1,w:-1,w:-1,w:-1,w:-1', 'EXACT', '-', 'Rsconf(-2,..) baud -- bios/serport.c:220-221')
sec('XBIOS 16 Keytbl(-1,-1,-1) -- the inquiry, bios/ikbd.c:122')
r('14:0x10', 'l:-1,l:-1,l:-1', 'PTR_RAM', '-', 'Keytbl(-1,-1,-1) -- bios/ikbd.c:122')
sec('XBIOS 17 Random -- bios/xbios.c:421, a pseudo-random number')
r('14:0x11', '-', 'IGNORE', '-', 'Random() -- bios/xbios.c:421')
sec('XBIOS 18 Protobt -- builds a boot sector in buffer 0, bios/floppy.c:690 (void)')
r('14:0x12', 'b:0,l:-1,w:-1,w:-1', 'IGNORE', '0:2@510', 'Protobt(b0,-1,-1,-1) checksum -- bios/floppy.c:725-735')
r('14:0x12', 'b:0,l:-1,w:-1,w:1', 'IGNORE', '0:2@510', 'Protobt(b0,-1,-1,1) exec checksum -- bios/floppy.c:725-735')
r('14:0x12', 'b:0,l:0x123456,w:1,w:0', 'IGNORE', '0:32', 'Protobt(b0,0x123456,1,0) serial+BPB -- bios/floppy.c:703-717')
r('14:0x12', 'b:0,l:0x123456,w:1,w:0', 'IGNORE', '0:2@510', 'Protobt(b0,0x123456,1,0) checksum -- bios/floppy.c:731-735')
r('14:0x12', 'b:0,l:-1,w:7,w:-1', 'IGNORE', '0:32', 'Protobt(b0,-1,7,-1) type out of range -- bios/floppy.c:711')
sec('XBIOS 21 Cursconf(5,0) -- the blink-rate inquiry, bios/vt52.c:893')
r('14:0x15', 'w:5,w:0', 'EXACT.W', '-', 'Cursconf(5,0) -- WORD cursconf, bios/vt52.c:893,918')
sec('XBIOS 23 Gettime -- LONG gettime, bios/clock.c:1266; the date is the machine\'s')
r('14:0x17', '-', 'NONZERO', '-', 'Gettime() -- bios/clock.c:1266')
sec('XBIOS 28 Giaccess(0,reg) -- register read, no write bit; LONG giaccess, bios/sound.c:94')
r('14:0x1C', 'w:0,w:7', 'EXACT', '-', 'Giaccess(0,7) mixer -- bios/sound.c:94')
r('14:0x1C', 'w:0,w:14', 'EXACT', '-', 'Giaccess(0,14) port A -- bios/sound.c:94')
sec('XBIOS 32 Dosound(-1) -- the inquiry, LONG dosound, bios/sound.c:145')
r('14:0x20', 'l:-1', 'EXACT', '-', 'Dosound(-1) idle table -- bios/sound.c:145-157')
sec('XBIOS 33-37 Setprt Kbdvbase Kbrate Vsync')
r('14:0x21', 'w:-1', 'EXACT.W', '-', 'Setprt(-1) -- WORD setprt, bios/parport.c:49')
r('14:0x22', '-', 'PTR_RAM', '-', 'Kbdvbase() -- bios/xbios.c:716')
r('14:0x23', 'w:-1,w:-1', 'EXACT.W', '-', 'Kbrate(-1,-1) -- WORD kbrate, bios/ikbd.c:439')
r('14:0x25', '-', 'IGNORE', '-', 'Vsync() -- void, bios/screen.c:1205')
sec('XBIOS empty slots: xbios_unimpl returns the number, bios/vectors.S:1366, xbios.c:1251')
empties = [0x28, 0x2D] + list(range(0x2F, 0x40)) + list(range(0x41, 0x50)) + [0x5C]
for f in empties:
    if f == 0x29: continue
    r('14:0x%02X' % f, '-', 'EXACT', '-', 'XBIOS 0x%02X empty slot -- bios/xbios.c:1251-1257' % f)
sec('XBIOS 41 Floprate inquiries -- LONG floprate, bios/floppy.c:1039')
r('14:0x29', 'w:0,w:-1', 'EXACT', '-', 'Floprate(0,-1) rate -- bios/floppy.c:1058-1061')
r('14:0x29', 'w:0,w:-4', 'EXACT', '-', 'Floprate(0,-4) HD? -- bios/floppy.c:1050-1051')
r('14:0x29', 'w:2,w:-1', 'EXACT', '-', 'Floprate(2,-1) no floppy 2 -- bios/floppy.c:1044-1045')
sec('XBIOS 44 Bconmap inquiries -- LONG bconmap, bios/serport.c:1082')
r('14:0x2C', 'w:-1', 'EXACT', '-', 'Bconmap(-1) mapped device -- bios/serport.c:1089-1090')
r('14:0x2C', 'w:-2', 'PTR_RAM', '-', 'Bconmap(-2) BCONMAP pointer -- bios/serport.c:1092-1093')
r('14:0x2C', 'w:0', 'EXACT', '-', 'Bconmap(0) maps nothing -- bios/serport.c:1095-1097')
r('14:0x2C', 'w:10', 'EXACT', '-', 'Bconmap(10) past the table -- bios/serport.c:1095-1097')
sec('XBIOS 46 NVMaccess read and invalid -- WORD nvmaccess, bios/nvram.c:155')
r('14:0x2E', 'w:0,w:0,w:48,b:0', 'EXACT.W', '3:48', 'NVMaccess(0,0,48,b0) read all 48 -- bios/nvram.c:30,183-197')
r('14:0x2E', 'w:0,w:0,w:0,b:0', 'EXACT.W', '-', 'NVMaccess(0,0,0,b0) count 0 -- bios/nvram.c:180-181')
r('14:0x2E', 'w:0,w:-1,w:1,b:0', 'EXACT.W', '-', 'NVMaccess(0,-1,1,b0) start<0 -- bios/nvram.c:180-181')
r('14:0x2E', 'w:0,w:47,w:2,b:0', 'EXACT.W', '-', 'NVMaccess(0,47,2,b0) past 48 -- bios/nvram.c:180-181')
r('14:0x2E', 'w:0,w:0,w:1,l:0', 'EXACT.W', '-', 'NVMaccess(0,0,1,NULL) -- bios/nvram.c:180-181')
r('14:0x2E', 'w:3,w:0,w:1,b:0', 'EXACT.W', '-', 'NVMaccess(3,..) bad op -- bios/nvram.c:206-208')
sec('XBIOS 64 Blitmode(-1) -- WORD blitmode, bios/xbios.c:886')
r('14:0x40', 'w:-1', 'EXACT.W', '-', 'Blitmode(-1) -- bios/xbios.c:886')
sec('XBIOS 81-87 TT shifter inquiries -- bios/screen.c:210-345 (TT only: profile)')
r('14:0x51', '-', 'EXACT.W', '-', 'EgetShift() -- bios/screen.c:210')
r('14:0x52', 'w:-1', 'EXACT.W', '-', 'EsetBank(-1) -- bios/screen.c:222')
for n in (0, 15, 255):
    r('14:0x53', 'w:%d,w:-1' % n, 'EXACT.W', '-', 'EsetColor(%d,-1) -- bios/screen.c:241' % n)
r('14:0x55', 'w:0,w:16,b:0', 'IGNORE', '2:32.W', 'EgetPalette(0,16,b0) -- bios/screen.c:290')
r('14:0x55', 'w:240,w:16,b:0', 'IGNORE', '2:32.W', 'EgetPalette(240,16,b0) -- bios/screen.c:290')
r('14:0x56', 'w:-1', 'EXACT.W', '-', 'EsetGray(-1) -- bios/screen.c:313')
r('14:0x57', 'w:-1', 'EXACT.W', '-', 'EsetSmear(-1) -- bios/screen.c:334')
sec('XBIOS 88-95 Videl inquiries -- bios/videl.c (Falcon only: profile)')
r('14:0x58', 'w:-1', 'EXACT.W', '-', 'VsetMode(-1) -- bios/videl.c:619')
r('14:0x59', '-', 'EXACT.W', '-', 'VgetMonitor() -- bios/videl.c:664')
r('14:0x5B', 'w:0x1C', 'EXACT', '-', 'VgetSize(0x1C) -- LONG vgetsize, bios/videl.c:724')
r('14:0x5E', 'w:0,w:16,b:0', 'EXACT.W', '2:64.L', 'VgetRGB(0,16,b0) -- WORD vgetrgb, bios/videl.c:862')
r('14:0x5F', 'w:0x1C', 'EXACT.W', '-', 'Vfixmode(0x1C) -- bios/videl.c:917')
sec('XBIOS 96-127 DSP inquiries -- bios/dsp.c (Falcon only: profile)')
r('14:0x67', '-', 'EXACT.W', '-', 'Dsp_GetWordSize() -- bios/dsp.c:508')
r('14:0x6A', 'b:0,b:1', 'IGNORE', '0:4.L', 'Dsp_Available(b0,b1) x -- bios/dsp.c:619 (no has_dsp test)')
r('14:0x6A', 'b:0,b:1', 'IGNORE', '1:4.L', 'Dsp_Available(b0,b1) y -- bios/dsp.c:619')
r('14:0x72', '-', 'EXACT.W', '-', 'Dsp_GetProgAbility() -- bios/dsp.c:976')
r('14:0x75', 'w:0', 'EXACT.W', '-', 'Dsp_InqSubrAbility(0) -- bios/dsp.c:1146')
r('14:0x77', 'w:-1', 'EXACT.W', '-', 'Dsp_Hf0(-1) -- bios/dsp.c:1200')
r('14:0x78', 'w:-1', 'EXACT.W', '-', 'Dsp_Hf1(-1) -- bios/dsp.c:1222')
r('14:0x79', '-', 'EXACT.W', '-', 'Dsp_Hf2() -- bios/dsp.c:1247')
r('14:0x7A', '-', 'EXACT.W', '-', 'Dsp_Hf3() -- bios/dsp.c:1257')
r('14:0x7D', '-', 'EXACT.B', '-', 'Dsp_Hstat() -- UBYTE dsp_hstat, bios/dsp.c:1273')
sec('XBIOS 128-141 DMA sound inquiries -- bios/dmasound.c (STe/TT vs Falcon: profile)')
for m in (0, 1, 6):
    r('14:0x82', 'w:%d,w:-1' % m, 'EXACT', '-', 'Soundcmd(%d,-1) -- bios/dmasound.c:342' % m)
r('14:0x88', 'w:-1', 'EXACT', '-', 'Buffoper(-1) -- bios/dmasound.c:499')
r('14:0x8C', 'w:0', 'EXACT', '-', 'Sndstatus(0) -- bios/dmasound.c:751')
r('14:0x8D', 'b:0', 'EXACT', '0:16.L', 'Buffptr(b0) -- bios/dmasound.c:772')
sec('XBIOS past the table: d0 keeps the number, bios/vectors.S:1323-1326')
r('14:0x8E', '-', 'EXACT', '-', 'XBIOS 0x8E past xbios_ent -- bios/vectors.S:1325-1326')

out = H
for x in rows:
    out += ('\n' if x is None else x + '\n')
sys.stdout.write(out)

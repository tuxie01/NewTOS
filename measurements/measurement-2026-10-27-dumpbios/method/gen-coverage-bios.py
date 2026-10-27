import collections, re, sys
R1, LATER, EXCL = '**round 1**', 'later', 'excluded'
bios = [
 (0x00,'Getmpb','bios/biosmem.c:134',EXCL,'rewrites `themd`, the memory descriptor GEMDOS built its pool on (`biosmem.c:143-150`); its capture would be three addresses'),
 (0x01,'Bconstat','bios/bios.c:914',R1,'9 cases: handles 0-6, 10, -1 (0..7 `bconstat_vec`, 6.. Bconmap, `bios/bios.c:923-933`)'),
 (0x02,'Bconin','bios/bios.c:958',R1,'4 cases: only handles that return at once on both machines -- 4, 5 (`char_dummy`, `chardev.c:34-36`), 10, -1. Handles 0-3 and the Bconmap devices 6-9 wait for input: never'),
 (0x03,'Bconout','bios/bios.c:991',R1,'4 cases: a printable byte to CON (2) and raw CON (5), handles 10 and -1. PRT (0) waits up to 30 s for a printer (`parport.c:150-168`), IKBD (4) and MIDI (3) are commands to a device, AUX (1) is NewTOS\' serial console: later, with their own reasons'),
 (0x04,'Rwabs','bios/bios.c:1045',LATER,'write: never (media). Read: needs a BIOS drive on the oracle -- its C: is a Hatari GEMDOS drive, not a block device; a disk image is an oracle change (`docs/analysis/ORACLE.md` §7)'),
 (0x05,'Setexc','bios/bios.c:1070',R1,'5 cases, the inquiry `Setexc(n,-1)` (`bios/bios.c:1076-1078`), NONZERO: a vector address differs per machine. Setting a vector: excluded'),
 (0x06,'Tickcal','bios/bios.c:1096',R1,'1 case, EmuTOS answers 20'),
 (0x07,'Getbpb','bios/bios.c:1120',LATER,'reads the boot sector of a BIOS drive: same oracle precondition as Rwabs; A:/B: would touch the floppy'),
 (0x08,'Bcostat','bios/bios.c:1143',R1,'9 cases: handles 0-7, -1 (IKBD/MIDI slots swapped, `chardev.c:38-42`)'),
 (0x09,'Mediach','bios/bios.c:1184',LATER,'same oracle precondition as Getbpb'),
 (0x0A,'Drvmap','bios/bios.c:1206',R1,'1 case, NONZERO: the drive set is the machine\'s (`blkdev.c:833`)'),
 (0x0B,'Kbshift','bios/ikbd.c:145',R1,'1 case, the inquiry -1; setting the shift state: later, with a restoring prelude'),
]
xb = {}
def x(n, name, cite, st, why): xb[n] = (n, name, cite, st, why)
x(0x00,'Initmous','bios/mouse.c:40',EXCL,'installs a mouse packet handler')
x(0x01,'Ssbrk','`xbios_unimpl`, bios/xbios.c:1287',EXCL,'empty on the pin, but a real TOS reserves memory at the top of RAM; the table may be carried to one')
x(0x02,'Physbase','bios/screen.c:1042',R1,'1 case, PTR_RAM')
x(0x03,'Logbase','bios/screen.c:1071',R1,'1 case, PTR_RAM')
x(0x04,'Getrez','bios/screen.c:1076',R1,'1 case, EXACT.W (`WORD getrez`); profile dependent (TT-Medium 4, Falcon 2)')
x(0x05,'Setscreen','bios/screen.c:1102',R1,'1 case, `Setscreen(-1,-1,-1)` returns 0 before anything is read (`screen.c:1115-1117`). Any other value moves or reprograms the screen the run draws on: excluded')
x(0x06,'Setpalette','bios/screen.c:1168',LATER,'reprograms the palette at the next VBL; needs a prelude that restores it')
x(0x07,'Setcolor','bios/screen.c:1192',R1,'3 cases, the inquiry `Setcolor(n,-1)`; setting: later (restoring prelude)')
x(0x08,'Floprd','bios/floppy.c:742',LATER,'reads a floppy; the answer is the drive\'s state on the oracle, an environment decision')
x(0x09,'Flopwr','bios/floppy.c:749',EXCL,'writes media')
x(0x0A,'Flopfmt','bios/floppy.c:898',EXCL,'formats media; nine arguments, one more than the record and the NewTOS ABI carry')
x(0x0B,'(used by BIOS)','`xbios_unimpl`, bios/xbios.c:1297',LATER,'empty on the pin; what a real TOS has there is not established')
x(0x0C,'Midiws','bios/midi.c:104',LATER,'sends bytes to the MIDI port')
x(0x0D,'Mfpint','bios/mfp.c:114',EXCL,'installs an interrupt vector')
x(0x0E,'Iorec','bios/xbios.c:336',R1,'5 cases: devices 0-2 PTR_RAM, 3 and -1 EXACT (-1)')
x(0x0F,'Rsconf','bios/xbios.c:376, bios/serport.c:217',R1,'2 cases, all -1 and -2 (baud), `serport.c:220-226`; a real setting reprograms AUX, NewTOS\' serial console: excluded')
x(0x10,'Keytbl','bios/ikbd.c:122',R1,'1 case, `Keytbl(-1,-1,-1)` PTR_RAM; installing tables: later')
x(0x11,'Random','bios/xbios.c:421',R1,'1 case, IGNORE')
x(0x12,'Protobt','bios/floppy.c:690',R1,'5 cases: a boot sector built in buffer 0, captured as bytes (serial + BPB, checksum at 510)')
x(0x13,'Flopver','bios/floppy.c:797',LATER,'same as Floprd')
x(0x14,'Scrdmp','bios/xbios.c:490',LATER,'runs `dump_vec`; a real TOS prints the screen')
x(0x15,'Cursconf','bios/vt52.c:893',R1,'1 case, function 5 (blink-rate inquiry); 0-4 change the cursor: later')
x(0x16,'Settime','bios/clock.c:1218',EXCL,'sets the machine clock')
x(0x17,'Gettime','bios/clock.c:1266',R1,'1 case, NONZERO: the date is the machine\'s')
x(0x18,'Bioskeys','bios/ikbd.c:136',LATER,'restores the key tables; measurable only after a Keytbl set')
x(0x19,'Ikbdws','bios/ikbd.c:948',EXCL,'commands the keyboard processor')
x(0x1A,'Jdisint','bios/mfp.c:122',EXCL,'switches an MFP interrupt')
x(0x1B,'Jenabint','bios/mfp.c:127',EXCL,'switches an MFP interrupt')
x(0x1C,'Giaccess','bios/sound.c:94',R1,'2 cases, register reads 7 and 14 (no write bit 0x80); a write: excluded')
x(0x1D,'Offgibit','bios/sound.c:130',EXCL,'writes PSG port A (drive select, strobe)')
x(0x1E,'Ongibit','bios/sound.c:115',EXCL,'writes PSG port A')
x(0x1F,'Xbtimer','bios/mfp.c:163',EXCL,'programs an MFP timer and its vector')
x(0x20,'Dosound','bios/sound.c:145',R1,'1 case, the inquiry -1 (`sound.c:150`); playing: later')
x(0x21,'Setprt','bios/parport.c:49',R1,'1 case, -1; setting: later')
x(0x22,'Kbdvbase','bios/xbios.c:716',R1,'1 case, PTR_RAM')
x(0x23,'Kbrate','bios/ikbd.c:439',R1,'1 case, (-1,-1); setting: later')
x(0x24,'Prtblk','`xbios_unimpl`, bios/xbios.c:1339',EXCL,'empty on the pin; a real TOS prints a block')
x(0x25,'Vsync','bios/screen.c:1205',R1,'1 case, IGNORE (void)')
x(0x26,'Supexec','bios/vectors.S:1376',LATER,'runs code; a table can name buffers, not code -- needs a dumper-owned function')
x(0x27,'Puntaes','`xbios_unimpl`, bios/xbios.c:1342',EXCL,'empty on the pin; a real TOS discards the AES and reboots')
x(0x29,'Floprate','bios/floppy.c:1039',R1,'3 cases: rate -1 and -4 (the inquiries) on drive 0, -1 on drive 2; setting: later')
x(0x2A,'DMAread','bios/disk.c:1241',LATER,'reads an ACSI/SCSI unit: environment')
x(0x2B,'DMAwrite','bios/disk.c:1254',EXCL,'writes media')
x(0x2C,'Bconmap','bios/serport.c:1082',R1,'4 cases: -1, -2 (PTR_RAM), 0 and 10 (map nothing); remapping AUX: later')
x(0x2E,'NVMaccess','bios/nvram.c:155',R1,'6 cases: a 48-byte read (`NVRAM_USER_SIZE`, `nvram.c:30`) captured, five error arms; write and reset: excluded')
x(0x40,'Blitmode','bios/xbios.c:886',R1,'1 case, -1; setting: later')
x(0x50,'EsetShift','bios/screen.c:180',EXCL,'changes the resolution the run draws in')
x(0x51,'EgetShift','bios/screen.c:210',R1,'1 case; TT only (profile)')
x(0x52,'EsetBank','bios/screen.c:222',R1,'1 case, -1; TT only')
x(0x53,'EsetColor','bios/screen.c:241',R1,'3 cases, (n,-1); TT only')
x(0x54,'EsetPalette','bios/screen.c:265',LATER,'reprograms the palette')
x(0x55,'EgetPalette','bios/screen.c:290',R1,'2 cases, 16 WORDs captured; TT only')
x(0x56,'EsetGray','bios/screen.c:313',R1,'1 case, -1; TT only')
x(0x57,'EsetSmear','bios/screen.c:334',R1,'1 case, -1; TT only')
x(0x58,'VsetMode','bios/videl.c:619',R1,'1 case, -1; Falcon only. A mode: excluded (resolution)')
x(0x59,'VgetMonitor','bios/videl.c:664',R1,'1 case; Falcon only')
x(0x5A,'VsetSync','bios/videl.c:675',EXCL,'changes the video sync')
x(0x5B,'VgetSize','bios/videl.c:724',R1,'1 case, mode 0x1C; Falcon only')
x(0x5D,'VsetRGB','bios/videl.c:806',LATER,'reprograms the palette')
x(0x5E,'VgetRGB','bios/videl.c:862',R1,'1 case, 16 LONGs captured; Falcon only')
x(0x5F,'Vfixmode','bios/videl.c:917',R1,'1 case, mode 0x1C; Falcon only')
dsp_r1 = {0x67:('Dsp_GetWordSize','bios/dsp.c:508','1 case'),0x6A:('Dsp_Available','bios/dsp.c:619','2 cases (x, y captured as LONGs); writes even without a DSP'),
          0x72:('Dsp_GetProgAbility','bios/dsp.c:976','1 case'),0x75:('Dsp_InqSubrAbility','bios/dsp.c:1146','1 case, ability 0'),
          0x77:('Dsp_Hf0','bios/dsp.c:1200','1 case, -1 (read)'),0x78:('Dsp_Hf1','bios/dsp.c:1222','1 case, -1 (read)'),
          0x79:('Dsp_Hf2','bios/dsp.c:1247','1 case'),0x7A:('Dsp_Hf3','bios/dsp.c:1257','1 case'),0x7D:('Dsp_HStat','bios/dsp.c:1273','1 case, EXACT.B (`UBYTE`)')}
dsp_other = {0x60:('Dsp_DoBlock','bios/dsp.c:222'),0x61:('Dsp_BlkHandShake','bios/dsp.c:259'),0x62:('Dsp_BlkUnpacked','bios/dsp.c:296'),
             0x63:('Dsp_InStream','bios/dsp.c:326'),0x64:('Dsp_OutStream','bios/dsp.c:350'),0x65:('Dsp_IOStream','bios/dsp.c:425'),
             0x66:('Dsp_RemoveInterrupts','bios/dsp.c:494'),0x68:('Dsp_Lock','bios/dsp.c:599'),0x69:('Dsp_Unlock','bios/dsp.c:611'),
             0x6B:('Dsp_Reserve','bios/dsp.c:628'),0x6C:('Dsp_LoadProg','bios/dsp.c:650'),0x6D:('Dsp_ExecProg','bios/dsp.c:670'),
             0x6E:('Dsp_ExecBoot','bios/dsp.c:693'),0x6F:('Dsp_LodToBinary','bios/dsp.c:922'),0x70:('Dsp_TriggerHC','bios/dsp.c:954'),
             0x71:('Dsp_RequestUniqueAbility','bios/dsp.c:965'),0x73:('Dsp_FlushSubroutines','bios/dsp.c:987'),0x74:('Dsp_LoadSubroutine','bios/dsp.c:1083'),
             0x76:('Dsp_RunSubroutine','bios/dsp.c:1166'),0x7B:('Dsp_BlkWords','bios/dsp.c:522'),0x7C:('Dsp_BlkBytes','bios/dsp.c:558'),
             0x7E:('Dsp_SetVectors','bios/dsp.c:1286'),0x7F:('Dsp_MultBlocks','bios/dsp.c:1338')}
for n,(nm,c,w) in dsp_r1.items(): x(n,nm,c,R1,w+'; Falcon only (TT: the function number)')
for n,(nm,c) in dsp_other.items():
    if n == 0x7E: x(n,nm,c,EXCL,'installs interrupt handlers')
    elif n in (0x68,0x69,0x6B,0x71,0x73,0x66): x(n,nm,c,LATER,'changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call')
    else: x(n,nm,c,LATER,'transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one')
snd = {0x80:('Locksnd','bios/dmasound.c:215',LATER,'locks the sound system; needs the paired Unlocksnd'),
       0x81:('Unlocksnd','bios/dmasound.c:230',LATER,'pairs with Locksnd'),
       0x82:('Soundcmd','bios/dmasound.c:342',R1,'3 cases, modes 0, 1, 6 with -1 (the inquiries); setting: later'),
       0x83:('Setbuffer','bios/dmasound.c:369',LATER,'sets DMA frame pointers'),
       0x84:('Setsndmode','bios/dmasound.c:401',LATER,'setter'),0x85:('Settracks','bios/dmasound.c:424',LATER,'setter'),
       0x86:('Setmontracks','bios/dmasound.c:441',LATER,'setter'),0x87:('Setinterrupt','bios/dmasound.c:457',LATER,'arms an end-of-frame interrupt'),
       0x88:('Buffoper','bios/dmasound.c:499',R1,'1 case, -1 (inquiry); starting playback: later'),
       0x89:('Dsptristate','bios/dmasound.c:521',LATER,'setter'),0x8A:('Gpio','bios/dmasound.c:545',LATER,'drives the DSP connector GPIO'),
       0x8B:('Devconnect','bios/dmasound.c:721',LATER,'rewires the sound matrix'),
       0x8C:('Sndstatus','bios/dmasound.c:751',R1,'1 case, 0 (no reset)'),
       0x8D:('Buffptr','bios/dmasound.c:772',R1,'1 case, 16 bytes captured as LONGs')}
for n,(nm,c,st,w) in snd.items(): x(n,nm,c,st,w)
# empties
for n in [0x28,0x2D]+list(range(0x2F,0x40))+list(range(0x41,0x50))+[0x5C]:
    x(n,'(empty)','`xbios_unimpl`, bios/xbios.c:1251-1257, vectors.S:1366',R1,'1 case: answers its own number')
assert sorted(xb) == list(range(0x8E)), [n for n in range(0x8E) if n not in xb]

def row(n,nm,c,st,w,hexw=2):
    return '| 0x%02X | %s | `%s` | %s | %s |' % (n, nm, c.replace('`',''), st, w)
out=[]
out.append(open(sys.argv[1]).read())
out.append('## BIOS, trap #13 -- `bios_vecs[]`, `bios/bios.c:1252-1265` (12 slots)\n')
out.append('| # | Name | EmuTOS handler | Status | Cases / reason |\n|---|---|---|---|---|')
for b in bios: out.append(row(*b))
out.append('| 0x0C-0xFF | past `bios_ent` | `bios/vectors.S:1325-1326` | **round 1** | 3 cases (12, 127, 255): the dispatcher leaves the number in d0 |')
out.append('\n## XBIOS, trap #14 -- `xbios_vecs[]`, `bios/xbios.c:1285-1515` (142 slots, `LAST_ENTRY` 0x8D)\n')
out.append('| # | Name | EmuTOS handler | Status | Cases / reason |\n|---|---|---|---|---|')
for n in range(0x8E): out.append(row(*xb[n]))
out.append('| 0x8E-0xFF | past `xbios_ent` | `bios/vectors.S:1325-1326` | **round 1** | 1 case (0x8E). Not 0x96: TOS 4 has `VsetMask` there |')
cnt=collections.Counter()
for b in bios: cnt[('BIOS',b[3])]+=1
for n in range(0x8E): cnt[('XBIOS',xb[n][3])]+=1
out.append('\n## Summary\n')
out.append('| Table | Slots | round 1 | later | excluded |\n|---|---|---|---|---|')
for t,tot in (('BIOS',12),('XBIOS',142)):
    out.append('| %s | %d | %d | %d | %d |' % (t,tot,cnt[(t,R1)],cnt[(t,LATER)],cnt[(t,EXCL)]))
ncases = sum(1 for l in open(sys.argv[2]) if l.strip() and not l.lstrip().startswith('#'))
out.append('\nPlus the probes past each table (3 BIOS, 1 XBIOS). `bios-r1.tsv` holds %d cases; every round-1 row above names how many are its own. 35 of the 81 XBIOS round-1 slots are empty (`xbios_unimpl`) and answer their own number.\n' % ncases)
sys.stdout.write('\n'.join(out)+'\n')

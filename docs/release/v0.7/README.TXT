NewTOS 0.7 - alpha test release
===============================

(Deutsch: siehe LIESMICH.TXT)

NewTOS is Atari TOS (GEMDOS, BIOS, XBIOS, VDI, AES) as an operating system
of its own for the Raspberry Pi 4. It is not an emulator: Atari programs are
recompiled from their sources for ARM and run natively. The model and the
yardstick is EmuTOS (Falcon/TOS 4 profile).

This is an ALPHA release, meant for trying out and finding bugs, not for
daily use. Please use a spare SD card with nothing on it you still need.


1. What you need
----------------

- Raspberry Pi 4 Model B (please tell us its RAM size in your feedback)
- A monitor on HDMI 0 (the socket next to the USB-C power), Full HD
- A USB keyboard and a USB mouse (German or US layout); wireless sets
  with one USB receiver for keyboard and mouse work too
- A microSD card you may wipe completely (2 GB is enough)
- A power supply for the Pi 4


2. Installation
---------------

IMPORTANT - the card must look like this:

- Partition table: MBR (not GPT).
- First partition: FAT16, AT MOST 2 GB. NewTOS goes there (drive C:).
- NewTOS does not read FAT32 or exFAT yet. The Raspberry Pi Imager and the
  normal format dialogs of Windows/macOS create FAT32 or exFAT - that does
  NOT work.
- The 2 GB limit is hard: FAT16 above 2 GB needs 64 KB clusters (Windows
  allows that up to 4 GB); NewTOS accepts at most 32 KB clusters, like
  Atari TOS.
- Optional: more partitions (primary or logical), also FAT16 up to 2 GB,
  appear as drives D:, E:, ... up to P:. The rest of the card may stay
  unused.

macOS (Terminal; replace DISK with the card, e.g. disk4 - check with
"diskutil list", or you erase the wrong disk):

    diskutil partitionDisk /dev/DISK 2 MBR "MS-DOS FAT16" NEWTOS 2G "Free Space" FREE R

Windows (command prompt as administrator; N = the card's number from
"list disk" - check carefully, "clean" erases the whole disk!):

    diskpart
    list disk
    select disk N
    clean
    convert mbr
    create partition primary size=2000
    format fs=fat quick label=NEWTOS
    assign
    exit

Linux (sdX = the card, check with "lsblk"):

    sudo parted /dev/sdX --script mklabel msdos mkpart primary fat16 1MiB 2000MiB
    sudo mkfs.fat -F 16 -n NEWTOS /dev/sdX1

Then copy the CONTENTS of the ZIP folder "KARTE" ("card") onto the
partition (the files must sit in the root directory, not in a subfolder),
eject the card, put it into the Pi and switch on.

After a few seconds the boot screen appears, then the desktop.

Keyboard: the German layout is preset (file KEYBOARD.KBD). For a US
keyboard, delete KEYBOARD.KBD from the card.


3. What is on the card
----------------------

    kernel8.img, config.txt,        NewTOS itself and the Raspberry Pi
    start4.elf, fixup4.dat,         firmware boot files
    bcm2711-rpi-4-b.dtb
    ACC\SYSACC.NAC                  accessory "NewTOS System Information"
                                    (Desk menu)
    APPS\TERADESK\DESKTOP.NAP       TeraDesk (alternative desktop)
    APPS\QED\QED.NAP                QED (text editor; German menus)
    APPS\ZVIEW\ZVIEW.NAP            zView (image viewer, 10 formats)
    APPS\DOOM\DOOM.NAP              Doom (shareware episode)
    APPS\COREMARK\CORE.NCP          CoreMark (CPU benchmark, console)
    APPS\HYPVIEW\HYP_VIEW.NAP       HypView (hypertext viewer)

The help texts of QED, zView and TeraDesk are .HYP files in each
program's folder (e.g. APPS\QED\QED.HYP) and open in HypView.

Program extensions: .NAP = GEM program, .NCP = console program, .NTP =
console program taking parameters, .NAC = accessory. The Atari extensions
.PRG/.TOS/.TTP/.ACC stay reserved for real 68000 programs, which do NOT run
in this release.


4. New in 0.7
-------------

- Crash fixed: zView (and any program that starts helper modules) could
  crash right after a module returned - "Memory violation ... execute
  (translation fault, level 3)". The cause was in the kernel's page-table
  handling on the real Pi. 60 zView starts in a row without a crash here;
  if you still see it, please send a photo (see 5.).
- File selector: the slider works (click into the bar to page, drag the
  slider), and the mouse wheel scrolls the list (GitHub #9).
- USB: wireless receivers that carry keyboard and mouse (e.g. Logitech
  Unifying, Rii X8, Cherry DW 9000, MX Ergo) now give a working mouse
  (GitHub #3). A limit that could drop devices in larger USB setups
  (hubs with several devices) is gone.
- A program that touches address 0 now ends with a bus error as on an
  Atari, instead of silently running on; NewTOS itself keeps running.
- Console programs: VT52 screen control (cursor, clear, inverse) works.
- Without a battery clock the date starts at the build date of the
  kernel instead of 1980.
- Many smaller corrections measured against EmuTOS (BIOS, XBIOS, GEMDOS,
  AES), e.g. the archive bit after writing a file, drive re-login after a
  card change, window locking and menu messages.
- TeraDesk: no more overscan setting (no Atari hardware).
- The Ball demo is no longer included.

Earlier releases: see https://github.com/tuxie01/NewTOS/releases


5. Known limitations
--------------------

- No 68000 programs (.PRG/.TOS/.TTP/.ACC) - only programs recompiled for
  NewTOS run.
- One program at a time (plus desktop and accessories), like an Atari
  without MiNT.
- No sound, no network, no battery clock (after switching on, the date
  starts at the kernel's build date).
- FAT16 only; Raspberry Pi 4 only.
- No printing, no serial port.
- "Shutdown" in TeraDesk halts the Pi on purpose (like the original):
  switch the power off and on afterwards.
- A program stuck in an endless loop cannot be ended with Ctrl+Alt+Del -
  switch the power off and on.
- Mouse wheel of some wired mice (Logitech B100, Raspberry Pi mouse):
  does not scroll yet; the middle button is not used yet (GitHub #8).
- Menus: one tester needs several clicks on a menu entry and sees the
  black highlight stay; we cannot reproduce it yet - a short video helps.
- Doom can freeze in its menu when the X-Shot accessory is installed.
- When a program crashes, a crash report appears; a key takes you back
  to the desktop. In 0.7 the report ends with a diagnosis block (lines
  starting with "crash-walk", "model" and "ladder"). Please take a photo
  of the WHOLE screen before pressing a key!


6. Feedback
-----------

Just try it (ideas in TESTING.TXT) and report on GitHub:
https://github.com/tuxie01/NewTOS/issues -> New issue -> Bug report
(English or German). FEEDBACK-EN.TXT lists what helps us. Attach photos of
crash reports. Every report helps, including "everything worked".


7. Licence and sources
----------------------

NewTOS is licensed under the GNU GPL v2 or later. The included programs
carry their own licences (TeraDesk, QED, zView, HypView, Doom: GPL). The complete
sources (NewTOS, the SDK and the program adaptations) are at
https://github.com/tuxie01/NewTOS, tag v0.7. The Raspberry Pi firmware
(start4.elf, fixup4.dat, .dtb) comes from the Raspberry Pi Foundation under
its own licence (LICENCE.broadcom, included in the ZIP). DOOM1.WAD is the
freely distributable shareware episode by id Software.

Project and bug reports: https://github.com/tuxie01/NewTOS

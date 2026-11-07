NewTOS 0.7.1 - alpha test release
=================================

(Deutsch: siehe LIESMICH.TXT)

NewTOS is Atari TOS (GEMDOS, BIOS, XBIOS, VDI, AES) as an operating system
of its own for the Raspberry Pi 4. It is not an emulator: Atari programs are
recompiled from their sources for ARM and run natively. The model and the
yardstick is EmuTOS (Falcon/TOS 4 profile).

This is an ALPHA release, meant for trying out and finding bugs, not for
daily use. Please use a spare SD card with nothing on it you still need.


IMPORTANT for users of 0.7 and 0.6.1 - data loss when copying or moving
------------------------------------------------------------------------

NewTOS 0.7 (and 0.6.1) can lose file contents without any error message:
copying or moving a file onto ANOTHER drive (e.g. from C: to D:) and
replacing a file of the same name there could leave the copy empty
(0 bytes); when moving, the original was deleted as well, so both
contents were gone. It happened only by chance (it depends on where the
files sit in their folders). Seen in the NewTOS desktop; TeraDesk takes
the same path. 0.7.1 fixes this.

If you used 0.7 or 0.6.1 with more than one drive: check the files
you copied or moved onto another drive over an existing file. Is one of
them 0 bytes long? After a move, the original is gone too - take it from a backup.
Copies within one drive and copies that did not replace a file are not
affected.


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
    APPS\COREMARK\CORE.NCP          CoreMark (CPU benchmark, console;
                                    waits for a key at the end)
    APPS\HYPVIEW\HYP_VIEW.NAP       HypView (hypertext viewer)

The help texts of QED, zView and TeraDesk are .HYP files in each
program's folder (e.g. APPS\QED\QED.HYP) and open in HypView.

Program extensions: .NAP = GEM program, .NCP = console program, .NTP =
console program taking parameters, .NAC = accessory. The Atari extensions
.PRG/.TOS/.TTP/.ACC stay reserved for real 68000 programs, which do NOT run
in this release.


4. New in 0.7.1
---------------

4.1 Operating system (NewTOS)

- GEMDOS: copying and moving between drives no longer loses data (see
  the box at the top).
- GEMDOS and SD card: a single card error no longer makes every later
  save fail until a restart. The program receives an error code, and
  saving works again as soon as the card answers. Larger files are
  written to the card when they are closed (as on EmuTOS), with far
  fewer card writes, so saving and copying are faster.
- AES: window scroll bars behave like EmuTOS: dragging the slider shows
  an outline instead of stripes, holding the button on an arrow or in
  the bar repeats (GitHub #2).
- AES: the window shadow has the frame colour (black) like EmuTOS
  instead of a grey line (GitHub #7).
- AES: a quick click on a menu entry is no longer lost. In the desktop
  a menu click acts after the double-click time (about 0.2 s), as on
  EmuTOS.
- AES: the file selector's title bar shows the file mask (e.g. *.*).
- VDI: many smaller drawing corrections, measured against EmuTOS
  (lines, fills, flood fill, text underline, GDOS fonts).

4.2 SDK (libraries for ported programs)

- cflib: the library's built-in images (check boxes, radio buttons,
  alert icons) were stored in 68000 byte order and therefore drawn
  wrongly. Every program using cflib was affected; it showed in QED
  (GitHub #21).

4.3 Included programs

- HypView: no longer crashes when opened a second time as an accessory
  (GitHub #22).
- HypView: shows the black-and-white pictures of .HYP files (e.g. the
  QED.HYP logo) correctly.
- HypView: the toolbar icons were drawn for NVDI's palette. The port now
  adapts them to the standard TOS palette, so they show the intended
  colours as closely as that palette allows (GitHub #23). The operating
  system is unchanged; it draws colour icons like EmuTOS.
- CoreMark: keeps its result on the screen until you press a key
  (GitHub #12). The desktop does not wait after a console program, like
  EmuTOS.

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
- Menus: a tester reported for 0.7 that a menu entry needed several
  clicks and the black highlight stayed. We cannot reproduce it; if you
  still see it in 0.7.1, please tell us in GitHub #19 (a short video
  helps).
- Switching off: wait until a program has finished saving. Data a
  program has written but not yet closed is lost, as on an Atari.
- Doom can freeze in its menu when the X-Shot accessory is installed.
- When a program crashes, a crash report appears; a key takes you back
  to the desktop. The report ends with a diagnosis block (lines
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
https://github.com/tuxie01/NewTOS, tag v0.7.1. The Raspberry Pi firmware
(start4.elf, fixup4.dat, .dtb) comes from the Raspberry Pi Foundation under
its own licence (LICENCE.broadcom, included in the ZIP). DOOM1.WAD is the
freely distributable shareware episode by id Software.

Project and bug reports: https://github.com/tuxie01/NewTOS

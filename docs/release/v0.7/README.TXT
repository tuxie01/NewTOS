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
TODO-REL07: USB input: wireless receivers with the mouse on a second interface work since 0.7 (USB01 F3, GitHub #3) - mention here?
- A USB keyboard and a USB mouse (German or US layout)
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
    APPS\BALL\BALL.NAP              Ball (graphics demo)
TODO-REL07: HypView is not new any more in 0.7 - drop "new".
    APPS\HYPVIEW\HYP_VIEW.NAP       HypView (hypertext viewer, new)

The help texts of QED, zView and TeraDesk are .HYP files in each
program's folder (e.g. APPS\QED\QED.HYP) and open in HypView.

Program extensions: .NAP = GEM program, .NCP = console program, .NTP =
console program taking parameters, .NAC = accessory. The Atari extensions
.PRG/.TOS/.TTP/.ACC stay reserved for real 68000 programs, which do NOT run
in this release.


TODO-REL07: insert "New in 0.7" before this section (see TASK/REL07 Anlass); decide whether "New in 0.6.1" and "New in 0.6" stay, shrink or go.
4. New in 0.6.1
---------------

- Bug fixed (GitHub #1): a popup in an accessory dialog let the window
  behind it draw over the dialog. Screen locking and mouse clicks now
  behave as in EmuTOS.
- Bug fixed: renaming an open file could lose its contents.
- Deleting an open file now behaves as in EmuTOS.
- New: HypView, with the help texts of QED, zView and TeraDesk. Pictures
  in hypertexts and links into other files do not work yet.
- For developers: an SDK with a guide to porting your own programs
  (in the sources under docs/sdk/).


New in 0.6 (since 0.5, Atari meeting, October 2026)
---------------------------------------------------

- The desktop runs as an ordinary program; if it fails, the console
  (EmuCON) starts instead of a crash.
- Accessories have their own ids (ap_id 2, 3, ...) and can exchange
  messages with programs; timers and messages now wake programs and
  accessories correctly.
- Restart: Ctrl+Alt+Del (warm) and Ctrl+Alt+right Shift+Del (cold) reboot
  the Pi. TeraDesk "Reboot" works.
- Left and right Shift are told apart.
- Every program has its own current drive and directory.
- 75 files open at once (was 16).
- Output redirection for console programs (">" and "<"); Ctrl-S/Ctrl-Q
  pause the console output.
- Background dialog with preview, as in EmuTOS.
- Many desktop fixes (selection, redraw, dropping onto shortcuts).


5. Known limitations
--------------------

TODO-REL07: check every limitation below against 0.7 (still true? new ones, e.g. the wheel of wired mice, USB03 / GitHub #8).
- No 68000 programs (.PRG/.TOS/.TTP/.ACC) - only programs recompiled for
  NewTOS run.
- One program at a time (plus desktop and accessories), like an Atari
  without MiNT.
- No sound, no network, no battery clock (date/time at boot come from the
  kernel file).
- FAT16 only; Raspberry Pi 4 only.
- No printing, no serial port.
- "Shutdown" in TeraDesk halts the Pi on purpose (like the original):
  switch the power off and on afterwards.
- A program stuck in an endless loop cannot be ended with Ctrl+Alt+Del -
  switch the power off and on.
TODO-REL07: the 0.7 crash report carries a diagnosis block (crash-walk) - say so and ask for a photo of the whole screen.
- When a program crashes, a crash report appears; a key takes you back
  to the desktop. Please take a photo of it!


6. Feedback
-----------

Just try it (ideas in TESTING.TXT) and fill in FEEDBACK-EN.TXT: as an issue
on GitHub (https://github.com/tuxie01/NewTOS/issues) or as a reply on the
mailing list. Every report helps, including "everything worked".


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

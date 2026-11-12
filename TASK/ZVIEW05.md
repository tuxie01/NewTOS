# ZVIEW05 — zView: faster JPEG decoding in software (scaled decode, libjpeg-turbo)

Status: OPEN (2026-11-12) | Priority: P2

Type: developer (SDK: zView JPG codec and the JPEG library), with a real-Pi measurement before and after. Owner
2026-11-12: opening pictures in zView takes long ("gerade wenn man zView nimmt und Bilder öffnen will das dauer einfach
ganz schön lange", especially when you take zView and want to open pictures it just takes quite a long time); "pack es
erstmal in ticket" (put it into a ticket for now). Software path of the study
`docs/analysis/pi4-hw-jpeg-feasibility-2026-11-12.md` §6; the hardware path is [HW10](HW10.md).

## Facts

- The owner's pictures `C:\Bilder\IMG_4410.jpeg` (6 962 413 B) and `IMG_4831.jpeg` (9 749 869 B) are 5712 x 4284
  pixels (24 MP); `unnamed.png` is 480 x 480 (lead, read from the card 2026-11-12).
- Reading costs about 0.75-1 s per picture at the measured 9.4 MB/s
  (`measurements/measurement-2026-11-06-hw08-pi2/RUNINFO.TXT`; [FS08](FS08.md) removed the read bottlenecks). How long
  decoding takes is not measured.
- The codec `JPG.NLD` (`sdk/zview/overlay/zview/plugins/jpg/jpgldg.c`) uses IJG libjpeg 9f (`sdk/libjpeg/`), plain C
  without SIMD, built with -O2; it sets `JDCT_IFAST` and no fancy upsampling and does not use libjpeg's scaled decode
  (`scale_num`/`scale_denom`).
- libjpeg-turbo states SIMD makes it "generally 2-6x as fast as libjpeg" and has AArch64 NEON decompression; no figure
  for the Cortex-A72 (study §6). Porting it is an SDK C library port (ADR-019).

## Work

1. Baseline on the real Pi: open time of the two JPEGs and the PNG in zView (owner's stopwatch, or a timing probe),
   split into read and decode where the codec allows.
2. Scaled decode: when the picture is larger than what zView displays, let libjpeg decode at 1/2, 1/4 or 1/8 (first
   check how zView's codec interface hands the size to the codec and whether zView accepts a reduced image; zoom to
   100 % must still be possible).
3. libjpeg-turbo with NEON as the SDK's JPEG library, replacing IJG 9f (pinned source with checksum like the other SDK
   libraries; same API for every codec that links libjpeg).
4. Real-Pi measurement after each step, recorded as runs.

## Acceptance

- Open time of `IMG_4831.jpeg` measured before and after on the real Pi; the picture looks the same (owner).
- Progressive and baseline JPEGs still open; other codecs unchanged; SDK host tests green.

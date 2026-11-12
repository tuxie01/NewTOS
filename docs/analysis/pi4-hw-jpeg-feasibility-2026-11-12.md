# Pi 4 hardware JPEG decoder for zView: feasibility study (2026-11-12)

Status: dated snapshot, desk study. No code, no commit, no measurement. Nothing was run on a Pi.

Question (owner): can NewTOS (bare-metal AArch64, no Linux) use the Pi 4's VideoCore hardware
JPEG decoder so that zView's JPG codec decodes 24 MP / 7-10 MB JPEGs fast? Owner's argument:
the documentation must exist, otherwise there would be no Linux driver.

Labels used below: **FACT** = read this session at the cited location. **INFERENCE** = my reasoning
from facts. **UNVERIFIED** = not established by anything read.

Method note: public files were read over HTTPS (WebFetch for pages; plain `curl` of raw.githubusercontent.com
and api.github.com for files that had to be grepped; every request was a generic public URL, no
project or personal data). Downloads sit in the scratch dir
`/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a93bb553e549e20b8/dl/` (gitignored, removed with the ticket).
Nothing was fetched into `references/`. WebFetch answers are summaries by a small model; where a claim
rests only on such a summary it says so, and everything quoted from a downloaded file was grepped by me.

---

## 0. Short answer

**Feasible in principle, expensive, and not on the owner's premise.** There is hardware (a block named
JPEG exists in the BCM2711 datasheet's interrupt list) and Raspberry Pi publishes a path to it, but:

1. The only reachable path is a **proprietary firmware service** ("image_decode" in the VPU firmware),
   spoken to over **VCHIQ + MMAL messages**. There is **no register-level documentation** of the JPEG
   block (the datasheet only lists its interrupt number). The Linux "driver" the owner points to
   (`bcm2835-codec`) does **not** drive the decoder at all; it is a V4L2 wrapper around firmware components and
   exposes JPEG **encode** only. FACT, §1.
2. What is "documented" is **source code, not documents**: the MMAL message formats and the VCHIQ slot
   protocol exist only as C headers/implementation (BSD-3 or GPL-2.0 OR BSD-3). No prose spec of the
   transport was found. §2.
3. Known limits are bad for the use case: progressive JPEG not supported (per Kodi's use of it);
   output is I420/I422 (RGBA via aggregator in userland's brcmjpeg); the V4L2 driver caps codec roles at
   1920x1920; the firmware's own size limit is **UNVERIFIED**. No speed figure found anywhere. §3.
4. Status is poor: `raspberrypi/userland` is archived ("ancient and deprecated", 2025-08-27) and no longer
   installed on Bookworm; MMAL is on the way out. The Linux side is still alive in `rpi-6.18.y`, and the
   firmware repo is active (2026-11-09). §4.
5. The one precedent for a Linux-free VCHIQ client (Circle) ports the Linux driver and implements only
   the **audio** service, no MMAL. §5.
6. Cheap alternative with a known effect: the SDK's libjpeg is plain-C IJG 9f; swapping to libjpeg-turbo with
   NEON gives "generally 2-6x" per its own README (SIMD in general, not NEON-specific). §6.

---

## 1. Which path reaches the HW JPEG decoder on the Pi 4?

### 1.1 Is it hardware or firmware software?

- FACT: BCM2711 datasheet `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`, section 6.2
  "Interrupt sources", Table 102: VC peripheral IRQ **7 = "JPEG"** (next to H264 0-2, ISP, V3D, Transposer). I
  extracted the page text from the PDF streams myself (there is no pdftotext on the host); the same table
  lists "RPIVID" (HEVC) as IRQ 34. So the chip has a peripheral called JPEG that raises a VideoCore interrupt.
  The datasheet describes **no register** of it (extracted text contains "JPEG" once, only in that table).
  Page text extraction of tables is not fully reliable (INFERENCE on my method).
- FACT (staff statement, summary of a GitHub thread): in `raspberrypi/firmware` issue #452 (2015-07-27,
  maintainer 6by9) "the JPEG hardware block can only write YUV420 (I420) or YUV422 (I422) for non-RGB
  colourspaces" and the `image_decode` component accepts several compressed input formats.
  URL: https://github.com/raspberrypi/firmware/issues/452 (WebFetch summary; I did not read the raw thread).
- FACT: the userland client library header says "Jpeg encoder and decoder library using the hardware jpeg codec":
  *interface/mmal/client/brcmjpeg/brcmjpeg.h* (external, see §7), userland commit `a54a0dbb2b8dcf9bafdddfc9a9374fb51d97e976`
  (2024-12-23, master tip when read).
- INFERENCE: the decode work is done by a fixed-function block driven by VPU firmware, not by a VPU software
  decoder. Evidence is the two statements above plus the IRQ line; no document states the split. Whether
  the Pi 4 (VideoCore VI) block is the same as the 2015-era one (Pi 2/3): **UNVERIFIED**; the staff
  statement is from the Pi 2/3 era, the datasheet only shows that a JPEG block exists on the BCM2711.

### 1.2 Path 1: VCHIQ + MMAL component from the ARM

- FACT: userland's `brcmjpeg.c` creates the decoder as MMAL component
  `"vc.aggregator.pipeline:ril.image_decode:video_convert"` (line 41) and the encoder as `"vc.ril.image_encode"`
  (line 42). Input port encoding `MMAL_ENCODING_JPEG`; output I420, I422 or RGBA only (lines 268-296, 386-432). The
  `video_convert` stage after `image_decode` is what makes RGBA possible (consistent with the staff
  statement that the block itself writes only YUV).
  https://github.com/raspberrypi/userland/blob/a54a0dbb2b8dcf9bafdddfc9a9374fb51d97e976/interface/mmal/client/brcmjpeg/brcmjpeg.c
- FACT: the same decoder is reachable as OpenMAX IL component `"image_decode"`
  (*hello_pi/hello_jpeg/jpeg.c* (external, see §7), WebFetch summary) and as `"OMX.broadcom.image_decode"` (Kodi Leia
  *xbmc/cores/omxplayer/OMXImage.cpp:1089,1779,2137* (external, see §7), commit `0655c2c71821567e4c21c1c5a508a39ab72f0ef1`).
- FACT: Kodi requests `OMX_COLOR_FormatYUV420PackedPlanar` and has an unimplemented `@todo jpeg decoder can
  decimate by factors of 2` (OMXImage.cpp:1538, 1998). Kodi is GPL-2.0-or-later, a consumer, not a Broadcom document.

### 1.3 Path 2: V4L2 `bcm2835-codec` staging driver

- FACT: *drivers/staging/vc04_services/bcm2835-codec/bcm2835-v4l2-codec.c* (external, see §7) (rpi-6.6.y, tip
  `bba53a117a4a5c29da892962332ff1605990e17a` 2025-03-26; same constants in rpi-6.18.y) defines five roles
  (lines 92-99, 100-115): `ril.video_decode`, `ril.video_encode`, `ril.isp`, `ril.image_fx`,
  **`ril.image_encode`**. There is **no `ril.image_decode` role**. JPEG appears as format for `ENCODE_IMAGE`
  (and in the format table, lines 609-617).
- FACT: the driver caps `ril.video_*` roles at `MAX_W_CODEC/MAX_H_CODEC 1920` and only `ril.isp` at 16384 (lines 120-125, 3806-3833).
- FACT: PR https://github.com/raspberrypi/linux/pull/4923 (merged 2022-03-16) added `/dev/video31` for the
  JPEG **encoder** only (WebFetch summary). In that thread 6by9 notes that `image_encode` and `image_decode`
  "have been in the normal start.elf firmware since 2011" (summary; paraphrase, not a verbatim quote).
- INFERENCE: for a JPEG **still** decode there is no kernel-side V4L2 path. The MJPEG format in the `DECODE`
  role is a video-decode path capped at 1920x1920, useless for 24 MP.

### 1.4 Does the Pi 4's standard firmware provide it?

- FACT: `start4.elf`, `start4cd.elf`, `start4db.elf`, `start4x.elf` exist in `raspberrypi/firmware/boot`
  (api.github.com contents listing, 2026-11-12).
- FACT: `references/rpi4-boot/config-txt-boot.adoc:12`: the cut-down firmware (`start*cd.elf`) "removes support
  for codecs, 3D and debug logging"; the only way to select it is `gpu_mem=16`. NewTOS's `boot/rpi4/config.txt:25@gpu_mem` sets `gpu_mem=256`,
  so the full firmware is used.
- UNVERIFIED: that `image_decode` is present in plain `start4.elf` (vs. only `start4x.elf`). Evidence for
  "in normal start.elf" is the 2022 staff remark above (about `start.elf`, Pi 4 not named); the PR
  description mentions `start_x=1` being needed for the encoder in one test, which the thread leaves unresolved.
  Needs a measurement: create `vc.ril.image_decode` on the Pi and see.

---

## 2. What is published, under which license; is the transport documented?

| Piece | Where | License (FACT, header read) |
|---|---|---|
| MMAL client API + `brcmjpeg` + `mmal_vc_msgs.h` (host<->VC message defs) | raspberrypi/userland `interface/mmal/**`, commit `a54a0db` | BSD-3-Clause (Broadcom Europe / Raspberry Pi), `LICENCE` at repo root; repo "BSD-3-Clause" per GitHub |
| VCHIQ userspace header `vchiq_if.h`, `vchiq_ioctl.h` | userland *interface/vchiq_arm/* (external, see §7) | BSD-3-Clause |
| VCHIQ kernel core (`vchiq_core.c/.h`, `vchiq_arm.c`, `vchiq_pagelist.h`) | raspberrypi/linux *drivers/staging/vc04_services/interface/vchiq_arm/* (external, see §7), rpi-6.6.y | `GPL-2.0 OR BSD-3-Clause` (SPDX tags, first lines) |
| MMAL kernel client (`mmal-vchiq.c`, `mmal-msg*.h`, `mmal-encodings.h`, `mmal-parameters.h`) | *drivers/staging/vc04_services/vchiq-mmal/* (external, see §7) | `GPL-2.0` only |
| `bcm2835-codec` | *drivers/staging/vc04_services/bcm2835-codec/* (external, see §7) | `GPL-2.0` |
| Mailbox tag `RPI_FIRMWARE_VCHIQ_INIT = 0x00048010` | *include/soc/bcm2835/raspberrypi-firmware.h:154* (external, see §7) | `GPL-2.0` |
| Firmware blobs (`start4*.elf`, `fixup4*.dat`) | raspberrypi/firmware `boot/` | `LICENCE.broadcom`: **binary redistribution only, no modification, only for developing for/using a Raspberry Pi device** |
| Circle's port of vchiq (see §5) | `references/circle/addon/vc4/` | GPLv2 (its README) |

Additional FACTs:
- userland README (raw, read): "This repo is ancient and deprecated. It largely contains code using
  proprietary APIs to interface to the VideoCore firmware. We have since move to standard linux APIs.
  If you are using code from here you should rethink your solution. Consider this repo closed."
  https://raw.githubusercontent.com/raspberrypi/userland/master/README.md
- Documentation as prose: the Raspberry Pi documentation page on camera software
  (https://www.raspberrypi.com/documentation/computers/camera_software.html, WebFetch summary, first 100k of
  240k characters) does not mention MMAL or hardware JPEG decode. The firmware wiki
  (`references/rpi4-boot/firmware-wiki-mailbox-property-interface.md`) has **no entry for tag 0x00048010
  (VCHIQ init)**: grepping the local file for `vchiq` and `48010` returns nothing; the highest tag family
  present there is 0x0004800x. So the handshake that starts VCHIQ is **not** in the one document the project already accepts.
- Nothing like a "VCHIQ protocol specification" or MMAL-over-VCHI specification as a document was found: the
  repo trees I listed (userland, linux rpi-6.6.y) contain only DT binding docs
  (*Documentation/devicetree/bindings/soc/bcm/brcm,bcm2835-vchiq.yaml* (external, see §7)) besides code. The tree listing of
  linux was truncated by the GitHub API (`truncated: true`), so a document elsewhere in the tree cannot be excluded.
  Absence of a document: **INFERENCE**, not proven.
- The BCM2711 datasheet documents the ARM side's mailbox and doorbell **interrupts** (Table 101, section 2.3
  "ARMC interrupts": Mailbox, Doorbell 0/1, VPU0/1 halted; registers `BELL_IRQ0/1`) but not the VCHIQ doorbell
  registers/offset themselves (no hit for `B840` in the extracted text; extraction reliability as above).
  Circle's `references/circle/include/circle/bcm2835.h:369` has `ARM_VCHIQ_BASE (ARM_IO_BASE + 0xB840)` and the Linux driver uses `BELL0 0x00`, `BELL2 0x08`
  (*vchiq_arm.c:50-51* (external, see §7)): the register offsets are known only from code.

### 2.1 What a bare-metal driver would need (all FACT from code, location given)

1. **Start VCHIQ**: allocate zeroed slot memory (`TOTAL_SLOTS * 4096`, page aligned, bus-addressable, plus a
   fragments area), build `vchiq_slot_zero` (*vchiq_core.h:320-332* (external, see §7): magic, version, slot sizes, `platform_data[2]` = fragments
   offset/count, master and slave shared state, slot info array), then send its bus address to the VPU with the
   mailbox property tag `RPI_FIRMWARE_VCHIQ_INIT` (*vchiq_arm.c:629* (external, see §7); the firmware must answer 0, line 636) and wait for the
   connect. Reference implementation: *vchiq_arm.c:539-640* (external, see §7) (Linux rpi-6.6.y), `references/circle/addon/vc4/vchiq/vchiq_2835_arm.c:140-210` (Circle).
2. **Slot/message protocol**: `vchiq_core.c` (service open/close, data messages with 4 KiB slots, bulk transfers via page-lists
   `vchiq_pagelist.h`, remote-event semaphores, doorbell ring `BELL2` to wake the VPU, doorbell IRQ `BELL0` bit 2 from the VPU, lines
   *vchiq_arm.c:182-190* (external, see §7)). Several thousand lines.
3. **MMAL control service** (FOURCC `'mmal'`, `mmal_vc_msgs.h:MMAL_CONTROL_FOURCC`, `WORKER_VER_MAJOR 16`, minimum 10; the kernel
   client uses `VC_MMAL_VER 15`, `VC_MMAL_MIN_VER 10`, *mmal-msg.h:29-30* (external, see §7)) with message types `GET_VERSION`, `COMPONENT_CREATE`,
   `PORT_INFO_SET`, `PORT_ACTION`, `BUFFER_FROM_HOST`, `BUFFER_TO_HOST`, `PORT_PARAMETER_SET` (*mmal-msg.h:43-69* (external, see §7)), plus the port/format
   structs (`mmal-msg-port.h`, `mmal-msg-format.h`, `mmal-msg-common.h`, `mmal-encodings.h`) and the buffer
   path (userland's `mmal_vc_shm.c` is the "shared memory" variant: data passed as VPU-visible handles).
4. A **component driver** for `ril.image_decode`/aggregator pipeline following *brcmjpeg.c:268-432* (external, see §7): create, set
   JPEG input, commit output format with size 0x0 for autodetect, enable ports, push the compressed buffer in chunks of
   `buffer_size_min` (3 buffers), wait for an output buffer or for the "format changed" event.

---

## 3. Known limits (what could be established)

| Item | Finding | Label / source |
|---|---|---|
| Max image size of `image_decode` | not found | **UNVERIFIED** (`brcmjpeg.c`, `hello_jpeg` and `jpeg.c` state none) |
| V4L2 driver cap, codec roles | 1920x1920 (`MAX_W_CODEC/MAX_H_CODEC`), ISP role 16384 | FACT, *bcm2835-v4l2-codec.c:120-125* (external, see §7); this limits the V4L2 wrapper, **not** necessarily firmware `image_decode` |
| Compressed buffer size the codec driver assumes for JPEG | 4 MiB default (`DEF_COMP_BUF_SIZE_JPEG`); PR thread mentions JPEGs reaching a 2.5 MB limit (encoder output) | FACT, *bcm2835-v4l2-codec.c:150-151* (external, see §7); PR 4923 summary |
| Progressive JPEG | "progressive images not supported by decoder": Kodi detects SOF2 and refuses the HW path | FACT about Kodi's behaviour (*OMXImage.cpp:853-856* (external, see §7), commit above); that the firmware rejects it is INFERENCE from this |
| Components > 3 (CMYK) | Kodi bails too (line 858 follows the progressive check) | not fully read: UNVERIFIED |
| Subsampling | decoder output I420 or I422 (block writes only these); input subsampling variants other than 4:2:0/4:2:2 not documented | FACT (output); input UNVERIFIED |
| Output formats | I420, I422 (hardware); RGBA via a `video_convert` stage; BGRA refused by `image_decode` | FACT, firmware #452 and *brcmjpeg.c:41,396-402* (external, see §7) |
| Orientation, DCT scaling | decimation by 2 is noted as a possibility ("@todo ... decimate by factors of 2") but whether the firmware exposes it is not established | FACT that Kodi wrote the todo; UNVERIFIED that it works |
| Speed | **no measured figure found** in any source read | UNVERIFIED. The Kodi use, PiKVM's use for encoding and the 2015 thread give none |
| Memory | VPU needs shared buffers inside the GPU split; 24 MP as RGBA is 96 MB, I420 36 MB. NewTOS sets `gpu_mem=256` | INFERENCE (arithmetic); `boot/rpi4/config.txt:25@gpu_mem` |

---

## 4. Deprecation / firmware-drop risk

- FACT: `raspberrypi/userland` archived 2025-08-27, read-only, "no longer installed on latest RPiOS Bookworm
  images" (repo README via WebFetch), recommends V4L2/DRM/Mesa. https://github.com/raspberrypi/userland
- FACT: the legacy camera stack "has been deprecated for many years, and is unsupported"
  (https://www.raspberrypi.com/documentation/computers/camera_software.html, WebFetch summary). That is the camera stack, not
  the codec components.
- FACT: the kernel keeps the VCHIQ/MMAL client: `bcm2835-codec` and *vchiq-mmal/mmal-vchiq.c* (external, see §7) exist in
  `rpi-6.6.y`, `rpi-6.12.y` and `rpi-6.18.y` (HTTP 200 on each raw file, 2026-11-12), with `ril.image_encode` still
  in the 6.18 role list. `raspberrypi/firmware` master tip is "kernel: Bump to 6.18.60" of 2026-11-09 (api.github.com),
  i.e. firmware blobs are shipped and maintained.
- INFERENCE: the codec components are not announced for removal in anything read, but there is also **no
  stability promise**: the interface is staging code and a proprietary firmware service with its own version numbers
  (MMAL worker version 16/10, `mmal_vc_msgs.h`); `image_decode` has no V4L2 user, so it would be the first thing to rot
  unnoticed. The risk of a firmware update dropping it is **real but unquantified**; the mitigation is to pin the
  firmware (NewTOS pins its boot files anyway) and to keep the software decoder as fallback.
- The Pi 5 has a different VideoCore generation; whatever is built for the Pi 4 is a dead end for later hardware: INFERENCE (not researched).

---

## 5. Bare-metal precedents

- FACT (read): Circle contains a bare-metal VCHIQ interface driver: `references/circle/addon/vc4/` (README: "driver for
  the VCHIQ interface device ... VCHIQ audio service driver", sources taken from Linux, GPLv2; Circle `references/circle/CHANGELOG.md:821-825`
  introduced it in the step "VCHIQ audio service driver and a VCHIQ interface driver ... ported from Linux", with a
  Linux-kernel-API emulation library in `references/circle/addon/linux/`). `CVCHIQDevice` (`vchiqdevice.cpp`) wraps the Linux `vchiq_probe`.
- FACT: the services Circle implements on top are **audio** (`addon/vc4/sound`) and a partial userland port for
  EGL/OpenGL ES/OpenVG/Dispmanx (`addon/vc4/interface/README`: "AARCH = 32 only"), the latter via `vc_vchi_dispmanx.c`, `vc_vchi_gencmd.c`,
  `vc_vchi_tvservice.c`. A recursive grep of `references/circle/addon/vc4` for `mmal`, `image_decode`, `vc.ril` finds one irrelevant hit
  (`eglext_brcm.h`): **Circle has no MMAL**, and no JPEG.
- INFERENCE: Circle proves that the slot transport works without Linux on Pi hardware (sound via VCHIQ is a listed
  option for Pi 4 in `sample/34-sounddevices/README`), and gives a reusable licensing-compatible blueprint only if GPLv2 code may be consumed (an owner decision;
  the NewTOS kernel is a clean-room reimplementation, CLAUDE.md, so Circle/Linux would be a reading reference for behaviour, not source to copy).
  Whether Circle's VCHIQ works with AArch64 and the current firmware: UNVERIFIED (its graphics add-on states 32-bit only; the VCHIQ README does not say).
- Other bare-metal precedents (rpi-open-firmware, Ultibo, etc.): not searched; none known to me with certainty: UNVERIFIED.

---

## 6. Effort estimate and alternatives (all INFERENCE unless a source is given)

### 6.1 The hardware route

Components to write, with the facts they stand on from §2.1:

1. VCHIQ transport: slot memory + `vchiq_slot_zero`, mailbox tag 0x00048010, doorbell, IRQ, connect, service open, bulk and data messages, remote events. The Linux and Circle ports are 6-7 files of core C (`vchiq_core.c` alone is several thousand lines).
2. MMAL message layer: about 10 message types, port/format structs, buffer handling and the opaque-handle (shared memory) variant.
3. A decode session: create aggregator pipeline, format negotiation with size autodetect, chunked input, event handling, error paths.
4. Memory plumbing: bus addresses (VC view vs. ARM view), cache maintenance for buffers shared with the VPU, an MMU mapping with the right attributes, a VPU-visible output buffer large enough.
5. A `_JPD` driver published from the kernel (or the codec bypassing it): `jpgdh.h` shows the Brainstorm interface (`JPGDDRV_STRUCT`: OpenDriver, CloseDriver, GetStructSize, GetImageInfo, GetImageSize, DecodeImage) taking `InPointer/InSize`, `OutPointer`, `OutPixelSize` and callbacks. `sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:258@jpgdrv` falls back to libjpeg for progressive or DSP errors. YUV to RGB conversion and EXIF orientation would still be needed on the CPU (the block writes only I420/I422).
6. Verification against the hardware, as there is no emulation in QEMU `raspi4b` (INFERENCE: not checked).

Order of magnitude (INFERENCE, not a commitment): the transport is the large risk (protocol details only in code, behaviour of the VPU side
unobservable), a multi-week effort for one developer, and every step needs the real Pi. Payoff is unquantified because no speed figures exist (§3) and progressive JPEGs would still go through software.

### 6.2 Software alternatives

Facts about the current codec:
- `sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:199@dct_method,339@dct_method` sets `jpeg->dct_method = JDCT_IFAST` and `do_fancy_upsampling = FALSE`.
  **The file contains no `scale_num` / `scale_denom`** (grep: zero hits). So the DCT-scaling lever is not used today.
- The SDK links IJG libjpeg **9f** (`sdk/libjpeg/jpeg-9f`), built by `sdk/libjpeg/build-libjpeg.sh`; the unpacked tree contains no SIMD sources
  (no `jsimd*` file; an `ls | grep simd` returned nothing).

Options:
1. **libjpeg DCT scaling** (`scale_num/scale_denom`): libjpeg-turbo's *doc/libjpeg.txt* (external, see §7): "Currently, the only supported scaling ratios
   are M/8 with all M from 1 to 16, or any reduced fraction thereof (such as 1/2, 3/4, etc.)"
   (https://github.com/libjpeg-turbo/libjpeg-turbo/blob/main/doc/libjpeg.txt, main tip `3ef3367eb68aabbc25ce91bf2136c87ed4ec7218`, 2026-11-09).
   The tool-level behaviour of 9f's scaling was not re-read here (UNVERIFIED for 9f; the quote is libjpeg-turbo's). INFERENCE: for a viewer that shows a 24 MP
   image on a screen far smaller than that, decoding at 1/2, 1/4 or 1/8 skips most IDCT and colour work, with no new hardware. It changes
   what zView shows only if the zView pipeline accepts a reduced size (not examined; out of scope here).
2. **libjpeg-turbo with NEON**: README (https://github.com/libjpeg-turbo/libjpeg-turbo/blob/main/README.md, same tip): "uses SIMD instructions
   to accelerate baseline and progressive JPEG compression and decompression ... generally 2-6x as fast as libjpeg, all else being equal".
   Its ChangeLog.md records the AArch64 NEON implementation: "Optimized the ARM 64-bit (ARMv8) NEON SIMD decompression routines
   ... speeds up the decompression of full-color JPEGs by nearly 2x on average on a Cavium ThunderX processor and by about 15% on average
   on a Cortex-A53 core" (ChangeLog.md line 2106); also "Added a complete intrinsics implementation of the Arm Neon SIMD extensions" (line 1321) and
   NEON SIMD for progressive Huffman encoding (line 1274). These are 2015-era figures on other cores; the Pi 4's Cortex-A72 is not measured.
   Pro: works for progressive and CMYK, no firmware dependency, no GPL/proprietary issue (libjpeg-turbo is BSD-style/IJG/zlib, not re-checked here).
   Con: a port/swap in the SDK (ADR-019 covers the SDK's libraries; a change would need a ticket), and ADR-036 allows NEON in raster kernels (not a
   statement about libjpeg).
   Both can be combined: scaling plus SIMD.
3. A measurement is missing for both: the Pi 4 time of today's 9f decode of a 24 MP image, and the share spent in Huffman vs. IDCT vs. colour conversion. That
   number decides whether any acceleration is needed (INFERENCE; no measurement exists in this study).

---

## 7. What the owner would have to sanction as a source

Only needed if the hardware route is pursued. Proposed list, exact documents (all read-only, pinned by commit):

1. **raspberrypi/userland @ `a54a0dbb2b8dcf9bafdddfc9a9374fb51d97e976`** (BSD-3-Clause; archived):
   *interface/mmal/client/brcmjpeg/brcmjpeg.h* (external, see §7) + `.c` (the one worked example of the decoder pipeline),
   *interface/mmal/vc/mmal_vc_msgs.h* (external, see §7) (host<->VC message definitions), *interface/vchiq_arm/vchiq_if.h* (external, see §7),
   `vchiq_ioctl.h` (service API view).
2. **raspberrypi/linux, branch `rpi-6.6.y` (tip `bba53a117a4a5c29da892962332ff1605990e17a`) or a newer pinned branch**:
   `drivers/staging/vc04_services/interface/vchiq_arm/{vchiq_core.c,vchiq_core.h,vchiq_arm.c,vchiq_pagelist.h,vchiq_cfg.h}` (slot protocol and start-up; `GPL-2.0 OR BSD-3-Clause`),
   `drivers/staging/vc04_services/vchiq-mmal/{mmal-msg.h,mmal-msg-common.h,mmal-msg-port.h,mmal-msg-format.h,mmal-encodings.h,mmal-parameters.h,mmal-vchiq.c}` (the MMAL wire format; `GPL-2.0`),
   *include/soc/bcm2835/raspberrypi-firmware.h* (external, see §7) (tag 0x00048010) and, as a consumer example, *bcm2835-codec/bcm2835-v4l2-codec.c* (external, see §7).
   CLAUDE.md rule 6 calls Linux a cross-reference, not an authority; the owner would have to waive that for this one interface, because it
   has no vendor document otherwise. The GPL-only files would be read for behaviour only; copying them is not compatible with a clean-room rule (INFERENCE on the owner's policy: owner decision).
3. **BCM2711 datasheet** (already accepted), only for: VC IRQ 7 = JPEG (Table 102, section 6.2), ARMC doorbell/mailbox IRQs (Table 101, section 2.3).
   It does **not** supply the VCHIQ doorbell register addresses.
4. **Firmware property wiki** (already accepted) **must be extended or supplemented**: it lacks tag 0x00048010 (VCHIQ init), so item 2's
   `raspberrypi-firmware.h` would be the only source for that tag.
5. **Firmware blobs** `start4.elf` (or `start4x.elf`) and its licence *boot/LICENCE.broadcom* (external, see §7) (binary redistribution only), as the thing
   being talked to. The behaviour of `image_decode` (size limit, progressive, speed) is a property of the blob and can only be settled by measurement.
6. Optional cross-reference only, not a source: Kodi `OMXImage.cpp` (Leia, `0655c2c7…`, GPL-2.0-or-later), Circle `addon/vc4` (GPLv2),
   firmware issue #452 and linux PR #4923 (conversation, not specification).

For the software route: libjpeg-turbo `README.md`, `ChangeLog.md`, *doc/libjpeg.txt* (external, see §7) at `3ef3367eb68aabbc25ce91bf2136c87ed4ec7218`
(or a release tag) would be the cited documents; they concern a library, not hardware, so they fall under no Track B restriction.

---

## 8. Open questions / STOP-blocks

1. `image_decode` maximum dimensions and whether 6000x4000 decodes at all: needs a probe on the Pi (a Linux boot is not a primary source for NewTOS, but
   a read-only measurement is). UNVERIFIED.
2. Progressive and 4:4:4/4:1:1 input handling by the firmware itself. UNVERIFIED (only Kodi's pre-check seen).
3. Presence of `ril.image_decode` in plain `start4.elf`. UNVERIFIED.
4. Any speed figure for `image_decode` at 24 MP. None found; internet search through WebFetch (DuckDuckGo, Bing) returned nothing usable.
5. Whether the datasheet documents ARM-side doorbell registers in a table my extraction missed. Needs a person or tool to read the PDF's mailbox chapter.
6. Whether the Pi 4 JPEG block equals the 2015-era one. UNVERIFIED.
7. Owner decision: may Linux/Circle GPL sources be read as the specification of the VCHIQ/MMAL wire protocol?

## 9. Out-of-scope observations

- `jpgldg.c` does not use libjpeg scaling today (see 6.2); whether zView can consume a down-scaled decode was not examined.
- `jpgldg.c:22@_JPD` comment says NewTOS publishes no `_JPD` cookie; consequently the DSP path is dead code on NewTOS (as already stated in that file).
- `references/README.md:334` still lists the RPi 4 boot documentation as "TODO"; `references/rpi4-boot/` now holds three files.

## 10. Scope re-anchor

Scope: vendor-published documentation/interface definitions for the Pi 4 hardware JPEG decoder and the cost/alternatives for zView's JPG codec. Every
statement above carries a URL, a file:line in this repo or a label (INFERENCE/UNVERIFIED). Statements resting only on WebFetch summaries are marked as such.

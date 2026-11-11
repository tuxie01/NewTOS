# HW-TEST-LOG — Real Raspberry Pi 4 bring-up

> Manual hardware verification log + setup guide for booting NewTOS on
> a physical Raspberry Pi 4 (BCM2711) over a serial console. Closes
> PHASE-01 **EC2** once the boot banner appears on real hardware.
>
> **First metal boot succeeded 2026-06-17** (graphics path: VideoCore
> framebuffer test pattern on HDMI; serial console not yet wired). The
> boot/MMU/heap path and the BCM2711 mailbox framebuffer run on real
> silicon. Serial-console verification (boot banner + self-tests over
> PL011) still pending the USB-TTL adapter — see §7.

---

## 1. What you need

- A Raspberry Pi 4 (any RAM size — the kernel currently uses the first
  1 GiB unless the firmware-supplied DTB reports more).
- A micro-SD card (any size; we only use a small FAT32 boot partition).
- A **3.3 V** USB-to-TTL serial adapter (e.g. CP2102 / FT232 / CH340).
  **Do not** drive the Pi's RX with 5 V signalling — the GPIO is 3.3 V
  and 5 V will damage it. If the adapter is level-selectable
  (e.g. a "3.3 / 5 V" cable), select **3.3 V**.
  - **In use here:** Waveshare USB-TO-TTL (D), **FT232RNL**, USB-A,
    SH1.0 3-pin (TXD / RXD / GND), 3.3 / 5 V selectable — use the
    **3.3 V** setting. FTDI wire convention: black = GND, yellow = the
    adapter's RXD, orange = the adapter's TXD (verify against the
    cable's own labels).
- A host terminal at **115200 baud, 8N1, no flow control**.
  - **macOS** (FT232R works with the built-in driver — no install on
    recent macOS):
    ```sh
    ls /dev/tty.usbserial-*                  # find the device
    screen /dev/tty.usbserial-XXXX 115200    # exit: Ctrl-A then K
    ```
  - Alternatives: `minicom`, `picocom -b 115200 /dev/tty.usbserial-XXXX`,
    or PuTTY.

## 2. Wiring (USB-TTL ↔ Pi 4 GPIO header)

The kernel drives **PL011 / UART0** on GPIO 14/15. Connect three wires
(TX↔RX crossed), sharing a common ground. **Leave the adapter's 5 V /
3.3 V power pin disconnected** — the Pi is powered separately.

| USB-TTL pin | Pi 4 GPIO (BCM) | Pi 4 header pin |
|-------------|-----------------|-----------------|
| RX  (adapter in)  | GPIO 14 — TXD0 | pin 8  |
| TX  (adapter out) | GPIO 15 — RXD0 | pin 10 |
| GND               | GND            | pin 6  |

(Adapter RX ← Pi TX, adapter TX → Pi RX. If you only see output but
typing does nothing, you likely swapped TX/RX or left GND off.)

## 3. SD-card layout (FAT32 boot partition)

Format the card with a single FAT32 partition and copy:

1. **Raspberry Pi 4 firmware** (from the official `raspberrypi/firmware`
   `boot/` folder): `start4.elf` and `fixup4.dat` (the minimum for a
   bare-metal 64-bit kernel; `bootcode.bin` is **not** used on the Pi 4).
2. **`config.txt`** — use [`boot/rpi4/config.txt`](../boot/rpi4/config.txt)
   from this repo. The `init_uart_clock=48000000` line is mandatory.
3. **`kernel8.img`** — built below.

That's it. No `cmdline.txt` is required for a bare-metal kernel.

## 4. Build `kernel8.img`

```sh
cargo xtask build --board rpi4 --release
# → produces target/aarch64-unknown-none-softfloat/release/kernel8.img
#   (raw binary via objcopy; the path is the build's last line)
```

Copy that `kernel8.img` to the SD card's boot partition.

## 5. Boot

1. Insert the SD card, connect the USB-TTL wires, open the host terminal
   at 115200 8N1.
2. Power the Pi.
3. **Expected first output** (within a second or two):

   ```
   RB
   NewTOS booting on rpi4
   Phase 02 reached.
   [mm] ...
   ```

   `RB` is the **pre-MMU boot heartbeat** — the boot stub writes it to
   the PL011 data register as its very first action, before any MMU /
   stack / BSS setup. Seeing `RB` confirms the firmware loaded
   `kernel8.img`, the CPU reached our entry point, and the serial path
   + baud are correct. The banner and self-test sequence follow (the
   same one QEMU prints: `[selftest p04] ... 10 passed`, `EC4/D3/D6/D7
   PASS`, `D3/D4D5 PEXEC PASS`, `EINVFN OK`, then the `[cconin]` and
   `RX echo` prompts — type a key to see it echoed).

## 6. Troubleshooting

| Symptom | Likely cause |
|---|---|
| **Nothing at all — not even `RB`** | The CPU never reached our entry, or serial is dead: wrong baud (must be 115200); TX/GND not connected or TX/RX swapped; `config.txt` missing (`enable_uart=1` / `arm_64bit=1`); firmware files (`start4.elf`/`fixup4.dat`) or `kernel8.img` missing/misnamed. |
| **`RB` then nothing** | Serial + firmware + entry are **fine** — a boot stage *after* the heartbeat faulted (MMU stage-1, BSS, stack, or the high-half jump). This narrows it to `boot.rs` / `mmu.rs`; capture exactly how far it got. |
| **Garbage / random bytes** | UART clock mismatch — confirm `init_uart_clock=48000000` is in `config.txt`. This is the #1 Pi 4 cause. |
| **Output but typing does nothing** | Adapter TX → Pi RX (GPIO 15) not connected, or TX/RX swapped. |
| **Banner then hangs / faults** | A real-hardware MMU / cache / boot-stage difference vs QEMU — capture the full log and the `[EXC]` line if any, and record it below. |

The pre-MMU `RB` heartbeat (boot stub) makes the silent-early-failure
case diagnosable: **no `RB`** = serial/firmware/entry problem; **`RB`
then silence** = a fault in our own early boot code.

---

## 7. Test log

Record each metal boot attempt here (date, firmware version, RAM size,
what appeared, any fault).

| Date | Pi model / RAM | Firmware | Result | Notes |
|------|----------------|----------|--------|-------|
| 2026-06-17 | RPi 4 Model B | `start4.elf`/`fixup4.dat` (card, 2026-04-17) | ✅ **First metal boot — VideoCore framebuffer test pattern visible on HDMI** (red / green+white-box / blue, 640×480 on a 1080p screen) | **Graphics path only** (no serial yet). Confirms on real silicon: EL2→EL1 drop (firmware enters at EL2), mailbox property-interface FB allocation against real VideoCore firmware, **Non-Cacheable FB mapping** (without it the display DMA would not see the cached pixels — the exact failure QEMU's TCG cannot reproduce), BGR pixel order, and the whole MMU/heap boot path. SD card written from this repo (`kernel8.img` + `init_uart_clock=48000000` added to an existing RPi4 `config.txt`). |
| 2026-06-17 | RPi 4 Model B | as above | ✅ **First full VDI frame on metal** — the Phase-12 VDI demo (16 GEM palette tiles via `vr_recfl` + line grid/border via `v_pline`, every pixel through `svc #2`) renders correctly on HDMI. | **Found + fixed a real HW-only cache bug en route:** the first attempt showed the filled tiles sharp but the **lines as dashes / missing** — dense `fill_rect` writes evict whole cache lines (visible), sparse single-pixel `put_pixel` writes stay stranded in untouched lines (invisible). The Non-Cacheable FB mapping was *not* effective on real silicon for these writes; QEMU's TCG has no cache so it could never surface there. Fix: `mmu::clean_dcache_range` + clean each drawn region (`dc cvac` to PoC) after `fill_rect`/`draw_line`. After the fix the grid lines are solid. The diagnostic signature (dense OK, sparse broken) pinpointed it without a serial console. |
| 2026-09-05 | RPi 4 Model B | as above | ✅ **BIOS group proven from EL0** — `APITEST.NCP` (kernel `2908bbe`): **86 PASS / 0 FAIL / 1 SKIP**. All twelve trap-13 functions, 80 checks, documented return *and* documented error case each. Better than QEMU's 82/0/5: the five interactive `Bconin(2)` checks ran (BIOS-CHR 25 → 30) and `Drvmap` saw four drives. The one skip is `T29.3` (needs a child program that is not on the card). Yesterday this group stood at **zero** proven functions of twelve. |
| 2026-09-05 | RPi 4 Model B | as above | ✅ **An application can draw to the screen itself** — `PHYSBASE.NCP`: asks `Physbase()` over the TOS API, writes three colour bands with a plain store loop, **no cache maintenance**. Bands appear. This is ADR-023 (`2cf4fac`) plus the uncached exposure (`92e6276`) working on real caches, which QEMU cannot test — it has no cache model. |
| 2026-09-05 | RPi 4 Model B | as above | ✅ **Page flipping on real VideoCore firmware** — `SCRFLIP.NCP` (`2908bbe`): painted the hidden page, screen showed **nothing** (`HIDDEN PAGE PAINTED — screendump now must NOT show the bands`), then `Setscreen` flipped and the image stood full-screen. Nothing copied, only the scan-out moved. Mailbox tag `0x0004_8009`, sourced from `references/circle`. The last piece the October demo needed. |
| 2026-09-09 | RPi 4 Model B | 1920x1080, `#Z` autorun | ⚠️ **`Setscreen` does not lock the physical base to the vertical retrace** | Observed with an external application that changes a fixed screen region: the page flip takes effect immediately (synchronous VideoCore mailbox write `0x0004_8009`), there is no pending register and no retrace interrupt that applies it. The tear line stands still and is impossible to overlook. No application program can work around it. Described with call chain, evidence and two open reference questions as `# KNOWN DEFECT` at `tos-api/src/xbios/screen.rs:221@KNOWN DEFECT`. No behaviour changed. |
| 2026-09-11 | RPi 4 Model B | 1920x1080 | ✅ **Overlapping windows: no residue, no error** — the rectangle-list repair FIX-01…06 accepted interactively | Run on real hardware by the owner and confirmed: let windows overlap, switch the focus between them, drag them over one another — **all without errors**, no leftover bands, no overdrawing across the frame. This is the D2/R2 residue case, the lead symptom of the whole pass, and it is structurally gone. Covers checklist D2 (`docs/api/desktop/_hw-test-checklist.md`) and [A32](../TASK/A32.md) points 1 and 2. QEMU cannot produce this evidence: the dumper opens and closes the windows within milliseconds, and `raspi4b` has no input device (our USB runs only over PCIe+xHCI). **Added on 2026-09-12** — the test ran on 11.09. but was not logged at the time; the empty checkbox had wrongly kept it as open. |
| 2026-09-11 | RPi 4 Model B | 1920x1080, `#Z` autorun | ✅ **TeraDesk starts again** — acceptance of the alignment fix (finding B2) on real hardware | Kernel `39b293a4…` (release, the same one that was checked in QEMU in both build profiles) plus a newly generated `icons.rsc` (13284 B, 0 of 76 raster pointers odd; before: 36). Before, TeraDesk aborted at once in the debug build with `slice::from_raw_parts requires the pointer to be aligned`; in the release build the same access passed silently as UB. The card carried the dev kernel `3c6d7a4a…` before. Remaining open: QED on real hardware (finding B1, proven in QEMU) has not been run yet. |
| 2026-09-11 | RPi 4 Model B | 1920x1080, `#Z` autorun | ✅ **AES round 3 measured on real hardware** — 39 cases, `tosdiff --validate` accepted | Case table `aes-r3-wind-state.tsv` (CRC `979FAC60`), dump `measurements/measurement-2026-09-11-pi4-r3/`. Against the same run in QEMU `raspi4b`: **38 of 39 cases bit-identical**; the one difference is case 603, `int_out[3]/[4]` = 1920x1080 versus 640x480 — screen geometry, not a defect, and both correctly return the whole screen (`0,0,0,0` would have meant the early return fails). Confirms once more that the emulation matches the hardware. The comparison of content runs against the QEMU run because that one sits on the TT's 640x480 profile. |
| 2026-09-12 | RPi 4 Model B | 1920x1080, double-click in the desktop | ✅ **QED loads its resource again** — acceptance of the path-resolution fix (finding B1) on real hardware · ⚠️ **two other, known defects behind it** | Kernel `39b293a4…`. The alert „Resource nicht gefunden!" (resource not found!) is **gone**: `rsrc_load` now mirrors EmuTOS' `rs_load` (`gemrslib.c:835-837`) and resolves a qualified path via `sh_find`. Note the launch route: **by double-click in the desktop**, not via the `#Z` autostart that QEMU had checked — so the fix holds on both routes. **Visible behind it, both already known and ticketed, deliberately deferred by the owner:** (1) QED reports it cannot find the `CLIPBOARD` folder although it exists — this is the TOS1-3-versus-TOS4 fork at `Fattrib` on directories (project note `cflib-apps-expect-tos4-semantics`); (2) typed text in a new file produces **no drawn characters** — [Q06](../TASK/Q06.md) and `docs/analysis/qed-typing-redraw/`, where the original hypothesis has already been refuted. Neither is a regression of this fix. |
| 2026-09-18 | RPi 4 Model B | 1920x1080, `#Z` autorun TeraDesk | ✅ **H02 + H03 accepted on real hardware** — file info with a file selected, TeraDesk exits regularly | Kernel `cd1b1af9…` (release from `b186d6b`), `DESKTOP.NAP` `db31aae1…` (gemlib `a_evnt_timer` overlay). Run by the owner: select a file, *File → Info* → the selection stays, the file dialog appears and can be edited (H02); *File → Quit* → dialog → built-in desktop (H03). Owner: „das ist schon ein erfolgreicher Meilenstein" (that is already a successful milestone). Further findings observed, **not** named at the owner's request — the task list takes priority. The remaining items of the list `docs/api/desktop/_hw-test-2026-09-17.md` (A63–A65, T01, A66, A67) were not reported back individually and stay *HW open*. The old kernel is on the card as `kernel8.914b86e.img`. |
| 2026-09-20 | RPi 4 Model B | 1920x1080, `#Z` autorun TeraDesk | ✅ **Card accepted on `4aa3bf2`: colour icons, menu separators, full-width bars, start-and-return** | Kernel `0d959619…` (release), `DESKTOP.NAP` `fb37bb3b…` (A60), `desktop.rsc` `409f69a0…`/`icons.rsc` `8d4bc8a2…`/`cicons.rsc` `ee7de7f3…`/`qed.rsc` `30ac6bf2…` (v13, A40/A41). Owner: „perfekt, funktioniert" (perfect, works) in response to the announced expectation — colour icons for drives/trash can and yellow folders in the window (A41), menu separators as dotted lines (A46), menu bars across the full width (A40), start a program and return with intact windows and info line (H05, A39). Thereby confirmed on hardware: A39, A40, A41, A46, H05; A60 (TeraDesk starts/runs with the overlay) implicitly. Not reported back individually: A43/A44 (not visible), A63–A65, T01, A66, A67. |
| 2026-10-17 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **A112 + DESK48 + ARCH02 accepted, no regressions** | Kernel `bff063c5…` (release from `f2c68a31`; predecessor `715462e5…` as `KERN17B.IMG`). Owner: desktop (windows, folders, menus, dialogs), SYSACC, TeraDesk, QED as before, an open menu halts everything (A91); DESK48 selection disappears, several desktop icons are all processed, two windows on the same directory both update. *„soweit alles bestätigt, keine bugs“* (everything confirmed so far, no bugs). Not shown directly: accessory with its own timer (none on the card), ARCH02 probe `FPPEXEC` not run. |
| 2026-10-18 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **RST01 accepted — restart as planned** | Kernel `f000303a…` (release from `3717b49e`; predecessor `bff063c5…` as `KERN1018.IMG`), TeraDesk `DESKTOP.NAP` `fdaada9e…` (overlay, old one as `DESKTOP.BAK`). Owner: *„reset funktioniert, wie geplant“* (reset works as planned) — PM watchdog restart confirmed on real hardware (the source was only Circle, not the datasheet). Individual items of the test list (Ctrl+Alt+Del / +RShift, negative cases, TeraDesk „Reboot“, Shift L/R, SYSACC, folders) not reported back individually. |
| 2026-10-18 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **Release candidate NewTOS 0.6 accepted** | Kernel `4bc49cf3…` (release from `f7cb0fd9`; before: `f000303a…` as `KERN18B.IMG`). Owner short test (start/version, EmuCON `type` with Ctrl-S/Q and typeahead — speed, QED, Doom, TeraDesk, Ctrl+Alt+Del): *„laeuft wie erwartet“* (runs as expected). Thereby on a real Pi: PROC04 points 1/4, CON01 (USB poll per output byte without visible slowdown). |
| 2026-10-22 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **GH1 accepted, HypView on the Pi for the first time** | Kernel `1a86e475…` (release from `d40f4f16`, GH1 fix `7b295ab3`; predecessor as `KERN18C.IMG`), `HYP_VIEW.NAP` `200a7d78…` with `QED.HYP`/`ZVIEW.HYP`/`TERADESK.HYP` each in the program folder. Owner short test per checklist (SYSACC popup over `C:\ACC\`, QED menu + click into the text, desktop/windows/accessory, TeraDesk, HypView, zView): *„scheint all das zu funktionieren, auch das NAC funktioniert“* (all of that seems to work, the NAC works too). zView loading (FS08): *„das Laden ist ok“* (loading is ok) — SD on the Pi delivers about 5–10 MB/s, measured 10 MB in 975 ms. TeraDesk info (MEM01): about 10 MB ST-RAM, about 730 MB TT-RAM, matching the boot screen. Not run: changing values in SYSACC; renaming with a long name (the 8.3 input masks do not allow a long name, no priority). |
| 2026-10-22 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **Release NewTOS 0.6.1 accepted** | Kernel `eaba7c5e…` (release from `26193203`, only the version text differs from `d40f4f16`). Owner: boot screen and About show 0.6.1, desktop/SYSACC/HypView as before — *„passt, stimmt alles“* (fits, everything is right). |
| 2026-10-23 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **USB01 F1/F2/F8 + CON04 VT52 accepted** | Kernel `86205b81…` (usb-diag release from `7c1f621d`; predecessor `eaba7c5e…` as `KERN1022.IMG`), `VT52PI.NAP` `d7861cfa…`. Apple keyboard (with its own hub) + ROCCAT mouse in two plug positions (mouse on port 2 resp. 4 of the VL805-internal hub): 4 devices each, keyboard + mouse configured, 13 commands, no error; pointer, keys, typing worked both times, `C:\USBDIAG.LOG` written and renamed on the desktop. VT52PI at 240x67: T1–T5 and read-back are right, T6 blinks, key -> desktop back at once. Not covered: >15 commands, report-only mice. Evidence `measurements/measurement-2026-10-23-usb01-con04-pi/RUNINFO.TXT`. |
| 2026-11-04 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **Release candidate NewTOS 0.7 accepted (kernel), PROC08 fix and A128 in daily use** | Kernel `8ec56ee7…` (release with `crash-walk` from `d7711c42`, byte-identical in `NewTOS-0.7.zip`, `measurements/measurement-2026-11-04-rel07-card/RUNINFO.TXT`; predecessor `d8ab0c47…`). Owner: *„soweit passt alles, die Version wird auch richtig reported“* (so far everything fits, the version is also reported correctly). Earlier the same day: PROC08 fix 60 zView starts without a crash, file selector with wheel. Only the kernel on the owner's card, the other card files are the owner's. |
| 2026-11-05 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **A127 D1 on the Pi: menu click works the first time** | Kernel `ec66b9e7…` (release with `crash-walk` from `7b14ffe2`, `measurements/measurement-2026-11-05-a127-d1-card/RUNINFO.TXT`; predecessor `8ec56ee7…` = v0.7). Checklist from the RUNINFO: click selection in the desktop (e.g. ^N), drag selection, leave the menu and re-enter without a stuck highlight, the same in TeraDesk and QED, with a USB mouse. Owner: *„passt funktioniert“* (fits, works). Ragnar's counter-test (GitHub #3) is pending. |
| 2026-11-05 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **A130 (#2) on the Pi: window scroll bars without pattern bands, page click scrolls continuously** | Kernel `02c833a5…` (release with `crash-walk` from `8040abeb`, `measurements/measurement-2026-11-05-a130-card/RUNINFO.TXT`; predecessor `ec66b9e7…`). Checklist 1–4 from the RUNINFO; owner: "funktioniert wie erwartet" (works as expected). |
| 2026-11-05 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **A131 (#7) on the Pi: window shadow black (border colour of `W_BOX`) like EmuTOS, no grey line** | Kernel `e56021e7…` (release with `crash-walk` from `10978982`, `measurements/measurement-2026-11-05-a131-card/RUNINFO.TXT`; predecessor `02c833a5…`). Checklist 1–3 from the RUNINFO; owner: "es entspricht EmuTOS 1:1" (it matches EmuTOS 1:1) (the owner personally does not like the look, no assignment). |
| 2026-11-07 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **NewTOS 0.7.1 accepted (from the ZIP), HW08/A140/A141/A142/DESK70 on the Pi** | Release kernel `b0543dd4…` from `189401b2` (the acceptance ZIP; published ZIP from `6baafc92`, kernel differs only in the build revision, `measurements/measurement-2026-11-07-rel071-final2/RUNINFO.TXT`). Owner card, six FAT16 partitions C:-H:. Passed: version, CoreMark 7330,89, QED dialogs, HypView as program and accessory, file selector, shadows, scroll bars, menus, copying, FSWBENCH on D:; test kernel `ccd73cc4…` (`sd-fault`): WFPROBE t/d/u/b on D:, `fsck` clean. Evidence `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`. |
| 2026-11-08 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **NEON raster kernels V49–V51 on the Pi: Fbench faster, apps good in daily use** | Kernel `6cd6eb57…` (release from test branch = `main` `5be660c2` + V50/V51; equal to `main` `20c23349` except for the embedded commit hash; predecessor 0.7.1 `b0543dd4…`). Fbench against the V49 run: text x1,8-2,6, mono blit x3,4, scrolling/copying x1,5-2,3 (`measurements/measurement-2026-11-08-v50-v51-pi/RUNINFO.TXT`); V49 on 2026-11-07: fills up to x16 (`measurements/measurement-2026-11-07-v49-pi/RUNINFO.TXT`). Owner: *„ich habe einige apps getestet, das sieht wirklich gut aus“* (I tested some apps, that really looks good) — individual items (USB mouse after program switch V53, desktop wheel V56) not reported back individually. |
| 2026-11-08 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **V58 (lines) and V59 (mouse cursor) on the Pi: Fbench faster, daily use without findings** | Kernel `69487a4e…` (release from test branch `pitest2-2026-11-08` = `main` `6821179c` + V58 + V59; equal to `main` `64bf181e` except for the embedded commit hash; predecessor `6cd6eb57…`). Fbench against the V50/V51 run: Hide/show mouse x5,46, graf_mouse off+on x4,51, form_dial redraw x1,94, XOR line +57 %, User line style +18 %, Polyline 32 +13 %; Line, 2 points and Dashed line unchanged (`measurements/measurement-2026-11-08-v58-v59-pi/RUNINFO.TXT`). Owner: ran Fbench completely and daily use (mouse, dragging windows, rubber band), no finding reported. |
| 2026-11-11 | RPi 4 Model B | — | ⚠️ **Correction (owner, 2026-11-11) to the 2026-09-20 entry above (card `4aa3bf2`):** colour icons were never tested on the real Pi; A41 not confirmed on hardware. A39, A40, A46, H05 from this entry are unconfirmed until retested. The 2026-09-20 row stays as written (history). Retest package: TASK/A41.md. | — |
| 2026-11-11 | RPi 4 Model B (per owner) | TeraDesk, `cicons.rsc` | ✅ **A41: owner confirms the colour icons work in TeraDesk** (owner, 2026-11-11; device and card not stated; no photos) — `measurements/measurement-2026-11-11-a41-owner/RUNINFO.TXT`. A39, A40, A46, H05 stay unconfirmed until retested. | — |
| 2026-11-11 | RPi 4 Model B | 1920x1080, TeraDesk, card kernel `e788dc58…` (b744febc release) | ✅ **Retest of the 2026-09-20 entry (owner): A41 colour icons, A46 dotted separators (TeraDesk), A40 full-width bars (TeraDesk and QED), A39/H05 launch QED and return with the window restored** — screenshots and photo: `measurements/measurement-2026-11-11-a41-screenshot/RUNINFO.TXT`, `measurements/measurement-2026-11-11-a46-photo/RUNINFO.TXT`, `measurements/measurement-2026-11-11-a40-fullscreen/RUNINFO.TXT`, `measurements/measurement-2026-11-11-a39-h05-shots/RUNINFO.TXT`, owner statements `measurements/measurement-2026-11-11-a41-owner/RUNINFO.TXT`, `measurements/measurement-2026-11-11-a39-h05-owner/RUNINFO.TXT`. Not shown: TeraDesk Quit back to the NewTOS desktop; A46 in the NewTOS desktop. | — |
| 2026-11-12 | RPi 4 Model B | 1920x1080, EmuDESK, card kernel `ac44f0b2…` (kern1112, `main` `f1fb97b59`) | ✅ **BIOS06: `Getmpb(NULL)` shows the "Bus Error" crash screen (user and super state), a real key brings the desktop back; BIOS09: `Bconin(1)` waits on AUX without a serial device, the kbdvec hook still sees keys, Ctrl+Alt+Del reboots** (owner photos and answers): `measurements/measurement-2026-11-12-bios06-bios09-pi/RUNINFO.TXT`, package `measurements/measurement-2026-11-12-pi-checks/RUNINFO.TXT`. BIOS11 (shift state in `Bconin(2)`) not measured: the probe needs injected keys; owner declined a variant. Not testable: BIOS09 reception of real bytes. | — |
| _pending_ | | | _serial console_ | Boot banner + self-tests over PL011 once the Waveshare FT232RNL USB-TTL (D) adapter is wired (ordered 2026-06-16). Heartbeat will read `RB2` (EL2 entry digit). |

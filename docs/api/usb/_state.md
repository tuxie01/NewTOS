# USB-HID spec extraction — state

> Spec group for **Phase 16** (USB-HID input). Unlike the other `docs/api/`
> groups this is **not a TOS API** — it is the **FreeMiNT USB stack**
> (`references/freemint/sys/usb`), a sanctioned extension source, extracted
> per `SOURCE-ANALYSIS-PROTOCOL.md`. Decision + scope: [`ADR-015`](../../MEMORY.md),
> plan: [`PHASE-16`](../../../phases/PHASE-16-usb-hid-input.md).

## Contract vs reference (per COMPATIBILITY.md)

- **Observable / preserved:** the **TOS keyboard/IKBD behaviour** the desktop
  and GEMDOS already consume — Atari scancodes, the keyboard vector, the VDI
  mouse cursor. USB HID is translated *into* this; nothing above the input
  layer changes.
- **Internal / our own:** the entire USB stack implementation. FreeMiNT uses
  C `struct`s of function pointers + loadable modules (`ucdif`/`uddif`);
  NewTOS uses **Rust traits** in one binary. We mirror the *architecture and
  the HID→scancode mapping*, not the C ABI.

## Source map (FreeMiNT `sys/usb/src.km/`)

| Area | FreeMiNT file | NewTOS spec entry | Status |
|---|---|---|---|
| Host-controller contract (`ucdif`) | `usb_api.h`, `usb.h` | [`host-controller-interface.md`](host-controller-interface.md) | ✅ |
| Core + enumeration (+ `uddif` binding) | `usb.c`, `udd.c` | [`core-enumeration.md`](core-enumeration.md) | ✅ |
| Hub | `hub.c` | [`hub.md`](hub.md) | ✅ |
| HID keyboard → scancode → IKBD | `FM/usb/src.km/udd/hid/keyboard/usb_keyboard.c` | [`hid-keyboard.md`](hid-keyboard.md) | ✅ |
| HID mouse → relative IKBD packet | `FM/usb/src.km/udd/hid/mouse-boot/usb_mouse.c` | [`hid-mouse.md`](hid-mouse.md) | ✅ |
| HID report-descriptor parser | `FM/usb/src.km/udd/hid/hid-common/hidparse.c`, `hid.c` | [`hid-mouse-protocols.md`](hid-mouse-protocols.md) (analysis, USB01) | ⏸ not built — boot protocol only; USB01 shows report-only / composite / absolute mice need it (§1.2, fix F4) |

**D0 status: complete for the boot-protocol HID scope.** The upper stack is
specified; the integration point is `ikbd::feed` (synthesise IKBD bytes — both
HID drivers reuse the existing, verified decoder → console/AES + VDI cursor).

Track-B (hardware): [`xhci-bringup.md`](xhci-bringup.md) ✅ — controller
discovery (QEMU PCIe ECAM `0x3f000000` / Pi-4 fixed `0xFE9C0000` + VL805
firmware), the `rust-osdev/xhci` `Registers` + high-half `Mapper`, the
xHCI-spec init sequence, and the DMA-buffer requirements. The `xhci` crate
(v0.9.2, no_std) is added to the kernel and builds.

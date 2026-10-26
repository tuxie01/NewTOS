# xHCI controller bring-up (Track-B: hardware)

> The hardware side of [`host-controller-interface.md`](host-controller-interface.md):
> how to find the controller, what `rust-osdev/xhci` gives us, and the
> xHCI-spec init sequence `HostController::open` runs. **Authority:** the
> *eXtensible Host Controller Interface (xHCI) spec* (Intel) + the QEMU
> hw/arm/virt.c memory map + the Raspberry Pi firmware VL805 interface;
> Circle (`rsta2/circle`) is the cross-check. PHASE-16 D1.

## 1. Finding the controller (the only QEMU-vs-Pi-4 divergence)

### QEMU `virt` — PCIe ECAM
`qemu-system-aarch64 -device qemu-xhci` puts an xHCI on the virt PCIe bus.
From QEMU hw/arm/virt.c `base_memmap`:

| Window | Base | Size |
|---|---|---|
| `VIRT_PCIE_ECAM` (config space) | `0x3f00_0000` | `0x0100_0000` (16 MiB) |
| `VIRT_PCIE_MMIO` (32-bit BARs)  | `0x1000_0000` | `0x2eff_0000` |
| `VIRT_PCIE_PIO`                 | `0x3eff_0000` | `0x0001_0000` |

Discovery: walk ECAM (bus/dev/fn → `ECAM + (bus<<20｜dev<<15｜fn<<12)`), match
**class code `0x0C0330`** (Serial Bus / USB / xHCI). Read **BAR0** (a 64-bit
memory BAR into the `0x1000_0000` window), set **Command.MEM_SPACE (bit 1)** to
enable memory decode; BAR0's address is the xHCI MMIO base. The robust source
for these windows is the **DTB** QEMU passes (the `pcie` node `reg`/`ranges`)
— the kernel already parses the DTB at boot — but the fixed virt addresses
above are a fine start. Add the three windows to the virt `layout::MMIO` so
Stage-2 maps them (reached high-half, like all MMIO — ADR-014).

### Real Pi-4 — fixed MMIO + VL805 firmware
The firmware maps the VL805 xHCI at a **fixed `0xFE9C_0000`** (no PCIe walk),
but its **firmware must be bootstrapped via a mailbox call first** (the
existing `bcm2711::mailbox`). Until that lands (D7), QEMU is the target.

→ Abstract as `locate_xhci() -> mmio_base`; everything below is shared.

## 2. `rust-osdev/xhci` integration

```rust
use xhci::Registers;
use xhci::accessor::Mapper;

/// Our MMIO is reached high-half (drivers::mmio_va): phys → phys + VA_OFFSET.
#[derive(Clone)]
struct HighHalf;
impl Mapper for HighHalf {
    unsafe fn map(&mut self, phys: usize, _bytes: usize) -> core::num::NonZeroUsize {
        core::num::NonZeroUsize::new(phys + VA_OFFSET as usize).unwrap()
    }
    fn unmap(&mut self, _virt: usize, _bytes: usize) {}
}

let mut regs = unsafe { Registers::new(mmio_base, HighHalf) };
// regs.capability / .operational / .runtime / .doorbell / .port_register_set
```

The crate is `#![no_std]`, register/structure-only — it does **not** do
transfers or enumeration (those are our `core`/`hub`/`hid`).

## 3. Init sequence — `HostController::open` (xHCI spec ch. 4.2)

Operational registers live at `mmio_base + CAPLENGTH`. Steps:

1. **Read capabilities** (`regs.capability`): `CAPLENGTH`, `HCSPARAMS1`
   (MaxSlots, MaxPorts, MaxIntrs), `HCSPARAMS2` (Max Scratchpad Buffers),
   `HCCPARAMS1` (64-bit addressing `AC64`, context size `CSZ`).
2. **Wait for ready**: spin until `USBSTS.CNR` (Controller Not Ready) = 0.
3. **Reset**: `USBCMD.HCRST = 1`; wait until it clears **and** `CNR` = 0.
4. **Max slots**: `CONFIG.MaxSlotsEn = HCSPARAMS1.MaxSlots`.
5. **DCBAAP**: allocate the Device Context Base Address Array (one 64-bit
   pointer per slot + index 0 = scratchpad-array pointer); write its phys to
   `operational.DCBAAP`. Allocate `HCSPARAMS2` scratchpad buffers (page-sized)
   if required, fill the scratchpad array.
6. **Command ring**: allocate a TRB ring; write its phys + cycle bit to
   `operational.CRCR`.
7. **Event ring**: allocate one segment + an Event Ring Segment Table (ERST);
   program the primary interrupter (`runtime.interrupter[0]`): `ERSTSZ`,
   `ERSTBA`, `ERDP`. (Polling: leave the IRQ masked; we read the event ring
   in `usb::poll`.)
8. **Run**: `USBCMD.RS = 1`; wait `USBSTS.HCH` (Halted) = 0.
9. **Ports**: read `port_register_set[i].PORTSC` — `CCS` (Current Connect
   Status) shows a connected device, `PED` enabled, `PLS`/speed fields the
   speed. This is what `root_hub(PortStatus)` returns. D1 success =
   `qemu-xhci` + `-device usb-kbd` shows one port `CCS = 1`.

Device-level (D2, after a port reset): **Enable Slot** command → slot ID →
allocate Input/Device Contexts → **Address Device** command (sets up EP0's
transfer ring) → control transfers via EP0. TRB / context formats: xHCI spec
ch. 6 — encoded by the `xhci` crate's structures.

## 4. DMA memory (important)

The controller **DMAs** to the command ring, event ring + ERST, DCBAA,
scratchpad, contexts and transfer buffers — so each must be:
- **physically contiguous** and aligned per the spec (rings 64 B, contexts
  per `CSZ`, DCBAA 64 B, ERST 64 B);
- reachable by the controller at a **physical/bus address** we hand it (we
  know phys via `VA_OFFSET`; on Pi-4 the VL805 sees ARM physical);
- **coherent** — map the buffers **Normal-NC** (reuse `mmu::remap_normal_nc`,
  the framebuffer path) or clean/invalidate around each use (ADR-014 cache
  discipline). Start with Normal-NC for simplicity.

A small bump/page allocator for these (from the existing frame allocator) is
the first thing D1 builds.

**Sources:** xHCI spec (Intel) ch. 4–6; QEMU
[hw/arm/virt.c](https://github.com/qemu/qemu/blob/master/hw/arm/virt.c)
`base_memmap`; `rust-osdev/xhci`; `rsta2/circle` (xHCI + VL805 reference).

# Real-Pi-4 USB: BCM2711 PCIe root complex + VL805 xHCI (PHASE-16 D7)

> **Track-B (hardware).** The four USB-A sockets of a Pi-4B hang off the **VL805**
> (VIA USB-3 controller), a PCIe endpoint behind the **BCM2711 PCIe root
> complex** (brcmstb). To use them bare-metal we must bring the RC up, train the
> link, enumerate the VL805, map its xHCI BAR, and load its firmware — then point
> the existing `drivers::usb::xhci` driver at it.
>
> **Authority / source:** `references/circle/lib/bcmpciehostbridge.cpp`
> (`pcie_probe`/`pcie_setup`/`enable_bridge`/`enable_device` + the RDB register
> field names) and `references/circle/lib/usb/xhcidevice.cpp` (orchestration + VL805 firmware
> mailbox). The official BCM2711 datasheet does **not** document the PCIe RC, so
> circle (which mirrors the Linux `pcie-brcmstb` driver) is the Track-B reference
> per the phase doc. The internal-xHCI alternative (`0xFE9C0000`, USB-C only,
> `otg_mode=1`) is **not** this path.
>
> **No emulation:** QEMU `raspi4b` does not model the VL805/PCIe xHCI, so D7 is
> verified on real hardware only.

## 1. Addresses (RASPPI == 4)

| Name | Value | Note |
|---|---|---|
| `ARM_PCIE_HOST_BASE` (RC regs) | `0xFD50_0000` | RC register block, ~0x9310 long |
| Outbound window, CPU side | `0x6_0000_0000` | `MEM_PCIE_RANGE_START`; 64 MiB (`0x400_0000`) — a 35-bit addr → `map_device_high` |
| Outbound window, PCIe side | `0xF800_0000` | `MEM_PCIE_RANGE_PCIE_START`; what the VL805 BAR is programmed to |
| VL805 xHCI MMIO (CPU view) | outbound-window base `0x6_0000_0000` | `ARM_XHCI0_BASE = MEM_PCIE_RANGE_START_VIRTUAL` (external controller) |
| VL805 on the bus | bus 1, slot 0, func 0 | `XHCI_PCIE_BUS=1`, `XHCI_PCIE_SLOT=0`, `XHCI_PCIE_FUNC=0` |
| VL805 class code | `0x0C0330` | xHCI; checked in `enable_device` |

DMA limit: PCIe can only reach **< 0xC000_0000** (3 GiB). Our xHCI DMA structs sit
in the kernel image (low RAM), so OK. Inbound window (`RC_BAR2`) maps PCIe-addr 0
→ system RAM 1:1, so we keep programming **physical** addresses into the xHCI
(unchanged from the QEMU path).

**DMA coherency:** Pi-4 PCIe DMA is **not** cache-coherent (QEMU was). The xHCI
DMA region (`XhciDma`/`DevDma`) must be mapped **normal non-cacheable**
(`mmu::remap_normal_nc`) on rpi4, or carry explicit clean/invalidate around every
ring/buffer access.

## 2. RC register map (offsets from `0xFD50_0000`)

```
PCIE_RGR1_SW_INIT_1                       0x9210
  INIT_GENERIC  mask 0x2     shift 1      // bridge soft reset
  PERST         mask 0x1     shift 0      // fundamental reset (RASPPI==4)
PCIE_MISC_MISC_CTRL                       0x4008
  SCB_ACCESS_EN    mask 0x1000     shift 0xc
  CFG_READ_UR_MODE mask 0x2000     shift 0xd
  MAX_BURST_SIZE   mask 0x300000   shift 0x14   // = BURST_SIZE_128 (0)
  SCB0_SIZE        mask 0xf8000000 shift 0x1b
PCIE_MISC_CPU_2_PCIE_MEM_WIN0_LO          0x400c   // +win*8
PCIE_MISC_CPU_2_PCIE_MEM_WIN0_HI          0x4010   // +win*8
PCIE_MISC_RC_BAR1_CONFIG_LO              0x402c   SIZE mask 0x1f shift 0  // disable (0)
PCIE_MISC_RC_BAR2_CONFIG_LO              0x4034   SIZE mask 0x1f shift 0
PCIE_MISC_RC_BAR2_CONFIG_HI              0x4038
PCIE_MISC_RC_BAR3_CONFIG_LO              0x403c   SIZE mask 0x1f shift 0  // disable (0)
PCIE_MISC_PCIE_CTRL                       0x4064
PCIE_MISC_PCIE_STATUS                     0x4068
  PCIE_PORT       mask 0x80  shift 7   // RC mode
  PCIE_DL_ACTIVE  mask 0x20  shift 5   // link up part 1
  PCIE_PHYLINKUP  mask 0x10  shift 4   // link up part 2
PCIE_MISC_REVISION                        0x406c   MAJMIN mask 0xffff
PCIE_MISC_CPU_2_PCIE_MEM_WIN0_BASE_LIMIT  0x4070   // +win*4
  LIMIT mask 0xfff00000 shift 0x14 ; BASE mask 0xfff0 shift 0x4 ; NUM_MASK_BITS 0xc
PCIE_MISC_CPU_2_PCIE_MEM_WIN0_BASE_HI     0x4080   BASE  mask 0xff   // +win*8
PCIE_MISC_CPU_2_PCIE_MEM_WIN0_LIMIT_HI    0x4084   LIMIT mask 0xff   // +win*8
PCIE_MISC_HARD_PCIE_HARD_DEBUG            0x4204
  SERDES_IDDQ          mask 0x08000000 shift 0x1b
  CLKREQ_DEBUG_ENABLE  mask 0x2        shift 0x1
PCIE_INTR2_CPU_BASE                       0x4300   (CLR +0x8, MASK_SET +0x10)
PCIE_RC_CFG_VENDOR_VENDOR_SPECIFIC_REG1  0x0188   ENDIAN_MODE_BAR2 mask 0xc shift 0x2 (=DATA_ENDIAN 0)
PCIE_RC_CFG_PRIV1_ID_VAL3                0x043c   CLASS_CODE mask 0xffffff shift 0 (=0x060400)
BRCM_PCIE_CAP_REGS                        0x00ac   // PCIe cap; +PCI_EXP_LNKCAP/LNKCTL2/LNKSTA
PCIE_EXT_CFG_INDEX                        0x9000
PCIE_EXT_CFG_DATA                         0x8000
```

`wr_fld(p,mask,shift,val)` = read-modify-write `reg = (reg & ~mask) | ((val<<shift)&mask)`.
`wr_fld_rb` adds a read-back. `PCIE_GEN = 2`. `BURST_SIZE_128 = 0`.

## 3. Bring-up sequence (`pcie_setup`, RASPPI==4)

1. `sw_init_set(1)`: `wr_fld_rb(SW_INIT_1, INIT_GENERIC, 1)` — reset bridge.
2. `perst_set(1)`: `wr_fld_rb(SW_INIT_1, PERST, 1)` — assert fundamental reset.
3. delay ~100–200 µs.
4. `sw_init_set(0)` — bridge out of reset.
5. `wr_fld_rb(HARD_PCIE_HARD_DEBUG, SERDES_IDDQ, 0)` — power up SerDes; delay ~100–200 µs.
6. read `REVISION.MAJMIN` (log only).
7. `MISC_CTRL`: set `SCB_ACCESS_EN=1`, `CFG_READ_UR_MODE=1`, `MAX_BURST_SIZE=BURST_SIZE_128`.
8. Inbound window `RC_BAR2`: `offset=0`, `size=pow2(RAM)`; write `RC_BAR2_CONFIG_LO = lower32(offset) | (encode_ibar_size(size)<<0)`, `RC_BAR2_CONFIG_HI = upper32(offset)`. Then `MISC_CTRL.SCB0_SIZE = ilog2(size)-15`.
9. `RC_BAR1.SIZE = 0`, `RC_BAR3.SIZE = 0` (disable).
10. clear+mask INTR2: write `0xffffffff` to `INTR2+CLR` then `INTR2+MASK_SET` (read-back each).
11. `set_gen(GEN=2)`: `LNKCAP = (LNKCAP & ~SLS)|gen`; `LNKCTL2 = (LNKCTL2 & ~0xf)|gen` (in `BRCM_PCIE_CAP_REGS`).
12. `perst_set(0)` — de-assert fundamental reset.
13. delay 100 ms.
14. poll `pcie_link_up()` (DL_ACTIVE && PHYLINKUP) up to ~100 ms (exponential backoff). Fail → "Link down".
15. check `pcie_rc_mode()` (`PCIE_PORT`). Fail → "EP mode".
16. `pcie_set_outbound_win(0, cpu=0x6_0000_0000, pcie=0xF800_0000, size=0x400_0000)` (see §4).
17. `wr_fld_rb(PRIV1_ID_VAL3, CLASS_CODE, 0x060400)` — present as PCIe-PCIe bridge.
18. read `LNKSTA` (speed/width, log).
19. `wr_fld_rb(VENDOR_SPECIFIC_REG1, ENDIAN_MODE_BAR2, 0)` ; `wr_fld_rb(HARD_PCIE_HARD_DEBUG, CLKREQ_DEBUG_ENABLE, 1)`.

`pcie_set_outbound_win(win, cpu, pcie, size)`:
- `MEM_WIN0_LO+win*8 = lower32(pcie)`, `MEM_WIN0_HI+win*8 = upper32(pcie)`.
- `cpu_mb = cpu>>20`, `limit_mb = (cpu+size-1)>>20`.
- `MEM_WIN0_BASE_LIMIT(+win*4).BASE = cpu_mb`, `.LIMIT = limit_mb`.
- `MEM_WIN0_BASE_HI(+win*8).BASE = cpu_mb >> 12`, `MEM_WIN0_LIMIT_HI(+win*8).LIMIT = limit_mb >> 12`.

`encode_ibar_size(size)`: `l2 = ilog2(size)`; if `l2 in 12..=15` → `l2-12+0x1c`, else if `l2 in 16..=35` → `l2-15`, else `0`.

## 4. Config-space access

`pcie_map_conf(bus, devfn, where)`:
- bus 0: RC itself — `slot!=0 → none`, else `RC_base + where` (direct).
- bus ≥1: write `cfg_index(bus,devfn,0)` to `EXT_CFG_INDEX (0x9000)`; data at `EXT_CFG_DATA (0x8000) + where`.

`cfg_index(bus,devfn,reg) = (slot&0x1f)<<15 | (func&7)<<12 | bus<<20 | (reg&~3)`.

Standard PCI config offsets used: `PCI_CLASS_REVISION 0x08`, `PCI_HEADER_TYPE 0x0e`,
`PCI_CACHE_LINE_SIZE 0x0c`, `PCI_SECONDARY_BUS 0x19`, `PCI_SUBORDINATE_BUS 0x1a`,
`PCI_MEMORY_BASE 0x20`, `PCI_MEMORY_LIMIT 0x22`, `PCI_BRIDGE_CONTROL 0x3e`,
`PCI_COMMAND 0x04`, `PCI_BASE_ADDRESS_0 0x10`/`_1 0x14`, `PCI_INTERRUPT_PIN 0x3d`.
`PCI_COMMAND` bits: MEMORY 0x2, MASTER 0x4, PARITY 0x40, SERR 0x100.
`PCI_HEADER_TYPE_BRIDGE 1`, `PCI_HEADER_TYPE_NORMAL 0`. Bridge ctl PARITY 0x01.

## 5. enable_bridge (bus 0) then enable_device (bus 1)

`enable_bridge`: conf = map_conf(0,0,_). Verify `class>>8==0x060400` && header==bridge.
`CACHE_LINE_SIZE=16`; `SECONDARY_BUS=1`, `SUBORDINATE_BUS=1`;
`MEMORY_BASE=MEMORY_LIMIT = pcie_addr>>16 (0xF800)`; `BRIDGE_CONTROL=PARITY`;
`PCI_EXP_RTCTL = CRSSVE`; `COMMAND = MEMORY|MASTER|PARITY|SERR`.

`enable_device(0x0C0330, slot 0, func 0)`: conf = map_conf(1,0,_). Verify
`class>>8==0x0C0330` && header==normal. `CACHE_LINE_SIZE=16`;
`BASE_ADDRESS_0 = lower32(pcie_addr) | MEM_TYPE_64 (0x4)`, `BASE_ADDRESS_1 = upper32(pcie_addr)`;
ensure `INTERRUPT_PIN=1`; `COMMAND = MEMORY|MASTER|PARITY|SERR`.

After this the VL805 xHCI BAR is decoded at CPU `0x6_0000_0000` (the outbound
window) → that is `Xhci::open`'s base.

## 6. VL805 firmware (mailbox), order vs PCIe

`xhcidevice.cpp` order: `PCIeHostBridge.Initialize()` → **then** the mailbox
`PROPTAG_NOTIFY_XHCI_RESET = 0x00030058` with
`value = (bus<<20)|(slot<<15)|(func<<12)` (here `1<<20`) → **then**
`EnableDevice(...)`. The mailbox asks the VC firmware to (re)load the VL805
firmware after the PCIe reset. Property channel = existing `bcm2711::mailbox`.

## 7. Wiring

`usb::init(dtb)` rpi4 branch: build the RC (`pcie::brcmstb`), `Initialize()`,
NOTIFY_XHCI_RESET, `enable_device()`, then `Xhci::open(0x6_0000_0000)` +
`scan_ports`/`enumerate_all` exactly as on qemu-virt (controller code is shared).
xHCI MMIO + the outbound window reach the kernel high-half (`mmio_va` /
`map_device_high`, ADR-014); the DMA region is normal-NC (§1).

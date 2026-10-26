# USB core — device enumeration + class-driver binding

> Extracted from FreeMiNT `sys/usb/src.km/usb.c` + `udd.c`. This is the
> hardware-independent middle of the stack: turn a freshly-connected port into
> a configured device bound to a class driver. The **authority is the USB 2.0
> spec** (chapters 9 + 11); FreeMiNT is the reference implementation we mirror
> in Rust. Spec group: [`_state.md`](_state.md).

## Enumeration sequence — `usb_new_device` (`usb.c:872`)

After the hub resets a port and reports a device:

1. **Device descriptor (partial)** — `usb_get_descriptor(DEVICE)` (`usb.c:460`),
   first 8 bytes, to learn `bMaxPacketSize0` for EP0.
2. **Set address** — `usb_set_address` (`usb.c:517`): `SET_ADDRESS` control
   request; the device leaves the default address 0.
3. **Device descriptor (full)** — re-read all 18 bytes (vendor/product/class).
4. **Configuration descriptor** — `usb_get_descriptor(CONFIG)` (`usb.c:479`):
   read 9 bytes to learn `wTotalLength`, then the whole config.
5. **Parse** — `usb_parse_config` (`usb.c:330`): walk interface + endpoint
   descriptors into the `usb_device` (interfaces, endpoints, intervals).
6. **Set configuration** — `usb_set_configuration` (`usb.c:566`):
   `SET_CONFIGURATION`; the device becomes usable.
7. **Bind a driver** — `usb_find_interface_driver`: match each interface to a
   registered class driver and call its `probe` (below).

All steps are `control` transfers via the HCD ([`host-controller-interface.md`](host-controller-interface.md)).
Standard requests/constants (`USB_REQ_GET_DESCRIPTOR`, `USB_DT_DEVICE`/
`USB_DT_CONFIG`/`USB_DT_INTERFACE`/`USB_DT_ENDPOINT`, `USB_DIR_IN`, …) come
from the USB spec; FreeMiNT mirrors them in `usb_defs.h`.

## `usb_device` model

The core keeps a fixed table (`usb_dev[]`, capacity `max_devices`). Each entry
holds: address, speed, `bMaxPacketSize0`, the device + active-config
descriptors, the parsed interface/endpoint list, parent hub + port, and
`maxchild` (non-zero for hubs). NewTOS reimplements this as a Rust
`UsbDevice` struct + a slot table.

## Class-driver binding (`uddif`) — `udd.c`

A class driver registers a `struct uddif` via `udd_register` (`udd.c:40`):

```c
struct uddif {            /* usb_api.h:107 */
    long  api_version;  long class;  char *lname;  char name[16];  short unit;
    unsigned short flags;
    long (*probe)     (struct usb_device *, unsigned int ifnum);  /* bind   */
    long (*disconnect)(struct usb_device *);                      /* unbind */
    long (*ioctl)     (struct uddif *, short cmd, long arg);
};
```

`probe(dev, ifnum)` inspects the interface's class/subclass/protocol and
returns success to claim it (e.g. HID boot keyboard = class 3, subclass 1,
protocol 1; boot mouse = protocol 2). On unplug the core calls `disconnect`.

**NewTOS shape:** a `trait DeviceDriver { fn probe(&self, dev, ifnum) ->
bool; fn disconnect(&self, dev); }`; the HID keyboard + mouse drivers
([`hid-keyboard.md`](hid-keyboard.md), [`hid-mouse.md`](hid-mouse.md))
implement it and, once bound, poll their interrupt-IN endpoint and feed
`ikbd::feed`.

## Boot protocol → no report-descriptor parser (yet)

Both HID drivers request the **boot protocol** (`SET_PROTOCOL` = 0), whose
report layout is fixed (see the HID specs). So the full HID **report-descriptor
parser** (`FM/usb/src.km/udd/hid/hid-common/hidparse.c`) is **not needed for D4/D5** and is deferred
(needed only for non-boot devices / gamepads later).

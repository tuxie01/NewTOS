# SYSINFO03 — System Info: choose the VDI depth (`vdiplanes`) among the start settings

Status: OPEN (2026-11-19) | Priority: P2

Type: developer, then tester. Owner decision 2026-11-19.

## Owner decision 2026-11-19

After the real-Pi session of TASK/PITEST02.md the card's `CMDLINE.TXT` read `vdiplanes=32`. Asked whether the lead
should set it back, the owner answered: no, rather build the choice into the System Info accessory ("eher in unser
sysinfo accessory einbauen").

## Facts

- The VDI depth is a boot argument, `vdiplanes=16|32`, 16 the default (ADR-038, TASK/V62.md).
- On the Pi the argument stands in `C:\cmdline.txt`; switching it today needs the Mac or a text editor
  (`measurements/measurement-2026-11-19-pitest02-prep/TESTBLATT.md`, part 0).
- System Info, version 2 (TASK/SYSINFO02.md) already offers the start settings as labelled choices and saves
  `C:\cmdline.txt` with a backup; its token handling is in `sdk/examples/sysacc-cmdline.h`, the accessory in
  `sdk/examples/sysacc.c`, the host test in `sdk/examples/tests/sysacc-cmdline-host.c`.

## Work

1. Add the VDI depth to the start settings of System Info: a labelled choice (16 or 32 planes, in words a user
   understands, never the raw token), read from and saved to `C:\cmdline.txt` by the rules of TASK/SYSINFO02.md
   (unknown tokens are kept, a backup is written first).
2. The dialog says that the setting takes effect at the next cold start.
3. The read-only page shows the depth the running system uses.

## Acceptance

- Host test of the token handling: `vdiplanes` absent, 16, 32, an invalid value, and beside other tokens.
- QEMU raspi4b: choose 32, save, the image's `cmdline.txt` holds `vdiplanes=32` and the other tokens unchanged;
  choose 16 again likewise. The dialog at both depths as a screendump.
- Real Pi 4: the owner switches the depth from the accessory and the next cold start uses it.
- `cargo xtask test` green; SDK host tests green; doc-check of the edited docs 0 failures.

## Open before the start

- Whether choosing 16 writes `vdiplanes=16` or removes the token is fixed with the spec of the dialog (the two boot
  alike except for one boot line).

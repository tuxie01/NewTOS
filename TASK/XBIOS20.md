# XBIOS20 — XBIOS: oracle cases for the 13 mapped entries without case or ticket

Status: PARKED (2026-11-11) | Priority: P3 | Reason: proposal modelled on XBIOS16 and XBIOS19 (parked with no program calls) and XBIOS08 to XBIOS10 (until a port needs it); for the owner | Reopen: a program calls one of them

Type: measurement. From the inventory snapshot docs/analysis/inventory-2026-11-11.md, gap G12 (T11).

## Facts

- Mapped XBIOS entries without a case: `Initmous`, `Mfpint`, `Midiws`, `Offgibit`, `Ongibit`, `Setpalette`, `Xbtimer`, `Floprd`, `Flopwr`, `Flopfmt`, `Flopver`, `DMAread`, `DMAwrite` (inventory section 2.2). `Bioskeys`, `Ikbdws`, `Scrdmp`, `Settime` have live tickets.
- The six floppy and DMA entries answer an error code and are cheap to add.
- Owner decision 2026-11-12: the floppy routines are obsolete (no floppy; reopen only for USB floppy support). `Floprd`, `Flopwr`, `Flopfmt`, `Flopver`, `DMAread`, `DMAwrite` answer `EUNDEV` as a sanctioned deviation (docs/COMPATIBILITY.md, "XBIOS floppy group") and leave this ticket; seven entries remain.

## Work

1. Write the cases, run on the oracle, compare.

## Acceptance

- The 7 remaining entries (`Initmous`, `Mfpint`, `Midiws`, `Offgibit`, `Ongibit`, `Setpalette`, `Xbtimer`) appear in a table with an oracle dump.

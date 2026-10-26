# References — NewTOS Source Material

This directory tracks the **source material** required by the
[Source Analysis Protocol](../docs/SOURCE-ANALYSIS-PROTOCOL.md). Each
contributor acquires the copyrighted/external sources independently;
nothing in this directory is redistributable as part of the NewTOS repo.

The directory itself is **gitignored** (`/references/`) except for this
README.

---

## Layout

```
references/
├── README.md                 # this file (committed)
├── emutos/                   # EmuTOS upstream checkout (Track A primary)
├── freemint/                 # FreeMiNT checkout (Track A secondary)
├── datasheets/               # BCM2711 ARM Peripherals datasheet (PDF)
├── sd-spec/                  # SD Simplified Specifications (PDF, see below)
│   └── archive/              #   superseded editions kept for old citations
├── circle/                   # rsta2/circle — tertiary RPi4 cross-reference
├── usb/                      # USB-IF PDFs: hid1_11.pdf (HID 1.11), hut1_5.pdf (HID Usage Tables 1.5)
├── compendium/               # Atari Compendium PDF (acquire separately)
├── profibuch/                # Atari Profibuch ST/STE (acquire separately)
├── tos-release-notes/        # Original Atari TOS release notes per version
├── atari-st-internals/       # Abacus / Data Becker book
├── rpi4-boot/                # Raspberry Pi 4 boot documentation
├── rpi-os-tutorials/         # Andre Richter's tutorials (git submodule recommended)
├── arm-arm/                  # ARM Architecture Reference Manual (ARMv8-A)
└── gabi/                     # System V gABI ELF chapters + Arm SYSVABI64
```

EmuTOS source lives under `references/emutos/` as a full upstream
checkout (see "EmuTOS source" below). It is the **primary** source for
Track A per the Source-Analysis-Protocol.

---

## EmuTOS source

- **Location:** `references/emutos/` (a full clone of
  https://github.com/emutos/emutos, default branch `master`).
- **Role:** Primary source for Track A (TOS API semantics). The TOS API
  spec in `docs/api/` is extracted from this tree during Phase 00.
- **Read-only:** the checkout is never modified by NewTOS work. It is
  referenced and quoted, never patched.
- **Not part of the Cargo workspace:** contains a separate Makefile-
  driven C build that we do not invoke. Documentation artifact only.
- **Versioning:** the SHA of the checkout is recorded in
  [docs/STATUS.md](../docs/STATUS.md) "Recent Decisions" each time it
  is updated, so spec entries stay reproducible.

To update the EmuTOS reference checkout:

```bash
(cd references/emutos && git fetch origin && git checkout origin/master)
git -C references/emutos rev-parse HEAD   # record this SHA in docs/STATUS.md
```

When citing EmuTOS in a spec entry or doc-comment, use the form:

```
EmuTOS @ <short-sha> — bdos/conout.c:bdos_2()
```

so that future readers can
`git -C references/emutos show <sha>:bdos/conout.c` and see exactly
what was being referenced.

### EmuTOS oracle builds (`references/emutos-oracle/`)

Separate clones of `references/emutos`, one per pinned commit, built by
`tools/oracle/build-emutos.sh` (the checkout above stays read-only and is
never built in). What they are and why: `docs/analysis/ORACLE.md`.

| Dir | Commit | ROM | sha256 |
|---|---|---|---|
| `1a7484ab/` | `1a7484ab` (pin) | `etos512de.img` | `323c87bbff65096e54a8dc00b474f6924a950ebf25cc9513bd53d495a2d2750c` |
| `v1.4/` | `978e3756` (tag `VERSION_1_4`, calibration) | `etos512de.img` | `67bc59a6e182e3399eaff479184d17017e34789cda4c814e08530874dbec34cf` |
| `release-1.4/` | — (EmuTOS 1.4 release image, copied from the release package) | `etos512de.img` | `5805c0a4834f8935c427454785d32c59e796b11ad6802101052b84fb1eb7a745` |

---

## FreeMiNT source

- **Location:** `references/freemint/` (a clone of
  https://github.com/freemint/freemint).
- **Role:** **Secondary** Track-A source — used only for kernel
  **extensions that EmuTOS does not implement** (e.g. `Ssystem` /
  GEMDOS 0x154 and its cookie subfunctions, later MiNT syscalls). EmuTOS
  remains primary for everything it covers; FreeMiNT is consulted when
  EmuTOS lacks the function. Adopted 2026-06-20 so NewTOS can run the
  MiNT/GEM software base (see ADR-012 and the Source-Analysis-Protocol).
- **Read-only / not in the workspace:** same rules as EmuTOS — quoted,
  never patched, not built by us.
- **Versioning:** record the checkout SHA in `docs/STATUS.md` when used.
  Current checkout: **`f906416`** (cloned 2026-06-20).

Cite FreeMiNT in spec entries / doc-comments as:

```
FreeMiNT @ f906416 — sys/ssystem.c:s_system(), sys/cookie.c:get_cookie()
```

---

## SD specifications (Track B, storage)

- **Location:** `references/sd-spec/` — the current editions; superseded
  editions that older citations in the tree rely on live in
  `references/sd-spec/archive/`.
- **Role:** Primary Track-B source for everything the SD card and the
  SDHCI controller do. The BCM2711 datasheet **has no EMMC chapter** — its
  only mention of the block is the interrupt table (Table 102, §6.2:
  IRQ 56 SDHOST, IRQ 62 "EMMC & EMMC2"). For register and protocol
  semantics the SD specifications are therefore not a cross-reference but
  *the* source. See `docs/hardware/emmc-sd.md`.
- **Read-only:** never converted, never re-hosted, never committed
  (see "Licence" below).

### The pinned editions

`/references/*` is gitignored, so "pinning" a PDF cannot mean committing
it. It means: **this table is the pin.** Edition, date, canonical
download and SHA-256 are recorded here, in the one committed file of this
directory, so that any contributor can verify byte-for-byte that they are
reading the document a citation was written against.

| Role | Document | Edition | Date | Local path | Bytes | SHA-256 |
|---|---|---|---|---|---|---|
| **reference** | SD Specifications Part 1, Physical Layer Simplified Specification | **9.10** | 2023-12-01 | `sd-spec/Part1PhysicalLayerSimplifiedSpecificationVer9.10Fin_20231201.pdf` | 6 130 804 | `ee86b20ec02b30797f14ceaf9f760feadb321de12cceb278b917aab7a65380e3` |
| **reference** | SD Specifications Part A2, SD Host Controller Simplified Specification | **3.00** | 2011-02-25 | `sd-spec/archive/PartA2_SD_Host_Controller_Simplified_Specification_Ver3.00.pdf` | 4 464 643 | `cc1574634d1644b24eb356dc07c11d8c5b91a48d7eb7175c02096168e0ec3e04` |
| current edition | SD Host Controller Simplified Specification | 4.20 | 2018-07-25 | `sd-spec/PartA2_SD_Host_Controller_Simplified_Specification_Ver4.20.pdf` | 2 459 579 | `e430d1d149696580274f876d1cc0624bc9b9dffc4e5a62cd76f1f658f3e6667d` |
| superseded, kept | Physical Layer Simplified Specification | 6.00 | 2018-08-29 | `sd-spec/archive/Part1_Physical_Layer_Simplified_Specification_Ver6.00.pdf` | 3 314 152 | `d19a722138e153df86980f7a7ea413caa4126030403f6b56eef10b6382e70c0d` |

**Why the Host Controller reference is 3.00 and not the current 4.20.**
The EMMC2 block reports its Host Controller version in `SLOTISR_VER`
bits 23:16; the only value we have measured is `0x02` = Host Controller
3.00 (QEMU `raspi4b`, audit §2.2 — on real hardware this is still
unmeasured, audit §5 M1). 4.20 adds UHS-II, ADMA3 and 64-bit addressing,
none of which EMMC2 offers us, and it **renumbers the register
sections** (see the mapping table in `docs/hardware/emmc-sd.md` §8). The
document that describes the controller we actually drive is 3.00; 4.20 is
kept alongside because it is the edition a newcomer will download.

**Why the Physical Layer reference is 9.10 and not the 6.00 the FS04
audit cited.** The two passages the driver depends on are *identical*:
§4.6.2 (read/write timeouts) differs only by the addition of "SDUC" to
three sentences, and §4.10.1 with Table 4-42 (card status) is unchanged
in bit assignment and type column — only the wording of `WP_VIOLATION`
and `WP_ERASE_SKIP` was extended by "write protected until power cycle
card". Both were diffed text-for-text on 2026-09-28 (REF02). Every
existing `6.00 §4.6.2.x` citation in the tree is therefore also a correct
9.10 citation; 6.00 is kept only so that the diff stays re-runnable.

### Licence — what the SDA terms actually say

Read on the download page (the "SIMPLIFIED SPECIFICATION TERMS AND
CONDITIONS" shown behind the Download button) and in the PDFs' own
"Conditions for publication" page, 2026-09-28:

- **§2 Limited license for non-members** (that is us): a personal licence
  to *access through the Site, download and view* the Simplified Specs
  "solely for Your individual, internal business use", and to print only
  as far as that use needs. Expressly **not** permitted: "(a) copy
  portions of the Simplified Specs (whether by screen scraping, screen
  shots, electronically or otherwise) … or (c) distribute the Simplified
  Specs or any portion thereof."
- **§3** grants the copy-portions-into-your-documentation right (with a
  copyright notice and a link back to the download page) **only to
  members of the SDA**. NewTOS is not a member.
- **§4** repeats it for all licensees: no sublicensing, no derivative
  works, no "framing" or "mirroring" on another server.
- The PDF says the same in its own words: "Distribution of the Simplified
  Specification, other than through the Site, is a violation of the Terms
  and the intellectual property rights of the SDA and its licensors."

**The answers, then:**

1. **The PDFs must not be committed** and must not be mirrored anywhere —
   which is what `/references/*` already enforces. This table is the
   substitute, and it is a complete one: it lets a reader verify the file,
   not obtain it.
2. **Every contributor fetches their own copy from the Site**, accepting
   the Terms there. That is the only authorised route.
3. **Facts are free, expression is not.** A limit ("250 ms maximum write
   busy for SDHC") and a section number are facts and belong in our
   documents. Long verbatim reproductions of tables or paragraphs are
   "copying portions" under §2(a) and do not, so our hardware notes
   paraphrase and cite the section instead of reprinting it. This is our
   own reading of the Terms, not legal advice; where a passage is so
   load-bearing that only the wording will do, keep it to a sentence and
   name the section.

### Lesson from REF04 (2026-09-29) — where "paraphrase" stops being enough

`docs/hardware/emmc-sd.md` §5 had turned SD Phys Table 4-42's entire
*Description* column (15 rows, one per R1 error bit) into a table of
verbatim quotes. That is not the "keep it to a sentence" case point 3
above allows for a single load-bearing passage — it is a whole table
column reproduced row for row, which is squarely "copy portions … by
screen scraping, screen shots, electronically or otherwise" under §2(a)
of the licence. REF04 replaced it: bit number and identifier stay (they
are identifiers our code already uses, not the specification's
expression), and each row now says, in our own words, what the bit does
to *our* driver, with the section reference moved to the end as a
pointer rather than a quote framer.

Two things for whoever next extracts from this source:

- **A whole table's Description/Note column, copied row by row, is the
  line — even when each individual row is short.** Single isolated
  sentence-quotes elsewhere in `emmc-sd.md` §5 (e.g. the short R1
  quotations above the Table 4-42 section) are the smaller, still-allowed
  case and were correctly left alone by REF04's scope.
- **Check driver doc-comments too, not only `docs/*.md`.** The same
  Table 4-42 Description-column wording sat a second time in
  `kernel/src/drivers/bcm2711/emmc.rs`, one sentence per `R1_*` constant
  — found only while writing this entry, after `emmc-sd.md` had already
  been fixed. It was rewritten in the same unit of work; a sweep of the
  whole tree afterwards found no third copy. Had the sweep been skipped,
  REF04 would have closed with the reproduction still in the public
  repository, just in a different file.

### Acquisition

The Site is Cloudflare-protected; a plain `curl` without browser headers
is rejected with 403. Two steps, both from
<https://www.sdcard.org/downloads/pls/> (archive:
<https://www.sdcard.org/downloads/pls/archive/>) — read and accept the
Terms there first:

1. Open the entry for the document; the page's "Download" link points at
   `https://www.sdcard.org/wp-content/themes/sdcard-org/dl.php?f=<file>`.
2. Fetch that link; verify with `shasum -a 256` against the table above.

Cite an SD specification as, e.g.:

```
SD Phys 9.10 §4.6.2.2        (Physical Layer Simplified Specification Ver. 9.10)
SD-HC 3.00 §2.2.16           (SD Host Controller Simplified Specification Ver. 3.00)
```

Always **section numbers, never page numbers** — the same section sits on
a different page in every edition.

---

## Raspberry Pi firmware device tree

Board wiring that the BCM2711 datasheet does not document (which SD
controller drives which port, on which pins) is described by the device
tree the Raspberry Pi firmware ships and the VideoCore hands to us.

- **Location on this machine:**
  `sd-card-backup-20260618/boot-files/bcm2711-rpi-4-b.dtb` (56 407 bytes,
  sha256 `75761b73c284e26623e4d1624bff13e67bce2ae620880efd81d6571a3739fcfb`,
  dated 2026-04-17) — a copy of the boot card's own `/boot` file, i.e. the
  artefact that actually ran on our Pi 4.
- **Read it with:** `dtc -I dtb -O dts <file>` (read-only; never write
  one back to the card).
- **Role:** Track B, board level. It ranks below the datasheet for
  anything the datasheet covers, and above every third-party source for
  wiring, because it is Raspberry Pi's own description of this board.
  Used in `docs/hardware/emmc-sd.md` §"Which controller".
- Upstream equivalent: `boot/bcm2711-rpi-4-b.dtb` in
  <https://github.com/raspberrypi/firmware> (record the firmware release
  when citing a fetched copy rather than the card's).

---

## Required-source checklist

Source-Analysis-Protocol step 1 demands that every task verify its
sources are present. The following table tracks acquisition status on
**this** development machine. Each contributor maintains their own copy.

| Source                        | Status (this machine) | Path                          |
|-------------------------------|-----------------------|-------------------------------|
| EmuTOS (master)               | present               | `emutos/`                     |
| FreeMiNT (`f906416`)          | present               | `freemint/`                   |
| Atari Compendium              | TODO                  | `compendium/`                 |
| Atari Profibuch ST/STE        | TODO                  | `profibuch/`                  |
| TOS Release Notes             | TODO                  | `tos-release-notes/`          |
| Atari ST Internals            | TODO (optional)       | `atari-st-internals/`         |
| BCM2711 ARM Peripherals       | present (Release 4, 2022-01-18) | `datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf` |
| SD Physical Layer Simpl. 9.10 | present               | `sd-spec/`                    |
| SD Host Controller Simpl. 3.00| present               | `sd-spec/archive/`            |
| SD Host Controller Simpl. 4.20| present               | `sd-spec/`                    |
| SD Physical Layer Simpl. 6.00 | present (superseded)  | `sd-spec/archive/`            |
| RPi 4 firmware device tree    | present (card copy, 2026-04-17) | `../sd-card-backup-20260618/boot-files/bcm2711-rpi-4-b.dtb` |
| circle (`6177984e`)           | present               | `circle/`                     |
| RPi 4 Boot Documentation      | TODO                  | `rpi4-boot/`                  |
| Andre Richter Tutorials       | TODO                  | `rpi-os-tutorials/`           |
| ARM ARM (A-profile) DDI 0487 M.d | present (2026-10-04) | `arm-arm/DDI0487M_d_a-profile_architecture_reference_manual.pdf` |
| Cortex-A72 MPCore TRM 100095 r0p3 issue 06 | present (2026-10-04) | `arm-arm/cortex_a72_mpcore_trm_100095_0003_06_en.pdf` |
| ELF for the Arm 64-bit Architecture (AAELF64), abi-aa 2025Q4 | present (2026-10-04) | `arm-arm/aaelf64.pdf` |
| System V ABI gABI, ELF chapters (DRAFT 2013-06-10) | present (2026-10-05) | `gabi/ch4.*.html`, `gabi/ch5.*.html` |
| System V ABI for the Arm 64-bit Architecture (SYSVABI64), abi-aa 2025Q4 | present (2026-10-05) | `gabi/sysvabi64.pdf` |

Status meanings:

- **present** — file/directory exists and matches the expected layout
- **TODO** — must be acquired before tasks that depend on it begin
- **N/A** — not needed for any planned phase

---

## Acquisition pointers (no warranty, copyright respect)

- **Atari Compendium** — Dan Hollis, 1992. Long out of print; archived
  PDF scans circulate on Atari preservation sites (atariarchives.org,
  atari-forum.com). Personal-archive use only.
- **Atari Profibuch ST/STE** — Jankowski / Reschke (Sybex / Markt &
  Technik). Out of print; second-hand, German.
- **TOS Release Notes** — distributed with original Atari developer
  toolkits; mirrored by the Atari developer community.
- **Atari ST Internals** — Abacus / Data Becker. Out of print.
- **BCM2711 ARM Peripherals** — official PDF on
  https://datasheets.raspberrypi.com/bcm2711/bcm2711-peripherals.pdf
- **RPi 4 Boot Documentation** —
  https://www.raspberrypi.com/documentation/computers/raspberry-pi.html
- **Andre Richter Tutorials** —
  https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials
- **ARM ARM (A-profile)** — DDI 0487 from
  https://developer.arm.com/documentation/ddi0487 ("Download"). Pinned copy:
  edition M.d, `DDI0487M_d_a-profile_architecture_reference_manual.pdf`,
  129 014 701 bytes, sha256
  `80d589a4645feba6bdb9d77edfaf04d4394d5d5444a4c41f97cf4fdd20eee3d5`.
  Cite section numbers from this edition only.
- **Cortex-A72 MPCore TRM** — ARM 100095 from
  https://developer.arm.com/documentation/100095 . Pinned copy: r0p3 issue
  06, `cortex_a72_mpcore_trm_100095_0003_06_en.pdf`, 575 pages, sha256
  `47f52c93806507962c2cd0b77991d7aebacfaae5734ac617fc45a5babdff738f`.
- **ELF for the Arm 64-bit Architecture (AAELF64)** — from
  https://github.com/ARM-software/abi-aa/releases (release 2025Q4,
  `aaelf64.pdf`), sha256
  `0f203e6b07216e84dde98bb28dbebef385fe7d911e4e82c2cec5eecdfee9c1e4`.
- **System V ABI — generic ELF (gABI)** — the `[SCO-ELF]` reference AAELF64
  cites (§2.2, "http://www.sco.com/developers/gabi/"); AAELF64 6.3–6.5 defer
  program loading and the dynamic section to it. Pinned copy: the
  "System V Application Binary Interface - DRAFT - 10 June 2013" pages from
  https://www.sco.com/developers/gabi/latest/ , fetched 2026-10-05 into
  `gabi/` (ARCH03 stage 0d: `Elf64_Rela`, `ELF64_R_TYPE/SYM`, `Elf64_Dyn`,
  `DT_*`, `PT_DYNAMIC`/`PT_INTERP`), sha256:
  `contents.html` `860a69166779d1c2264867df76c750e21af9f9f185fd451659be64e794da18b0`,
  `ch4.intro.html` `6362d902360f49d929e3d65bb7320392adf7adf0106e9c3d1f74649f416e72eb`,
  `ch4.eheader.html` `7b2d4920ce3dd63ab754427d52fe2eaea7f693080b2151979ac0af89a7c9c088`,
  `ch4.sheader.html` `0f7178d014050c3f022fbe9e269cac2bcb2e77217dc9d1fb00f6d886a2e69891`,
  `ch4.reloc.html` `1400b40eb00c022869f81509acf2da20e49a6c0211c9712f4cf88a3a23221ffe`,
  `ch5.intro.html` `9e0daef001866ee8a0d694dded65dbed98ab7aab402f5920509a09c622107019`,
  `ch5.pheader.html` `656798000adc23608001017f3c126303734a441eb6b400474125ba66cfc223b9`,
  `ch5.dynamic.html` `7e9e7587b90069398ae5830bbef0e40646a229531705cec86b69bee74e0e0602`.
  The draft does **not** define `DT_RELACOUNT` or `DT_RELR`; cite tags from
  this copy only.
- **System V ABI for the Arm 64-bit Architecture (SYSVABI64)** — the
  `[SYSVABI64]` AAELF64 6.2–6.5 now point to; from
  https://github.com/ARM-software/abi-aa/releases (release 2025Q4,
  `sysvabi64.pdf`, issued 2026-01-23), fetched 2026-10-05 into `gabi/`,
  sha256 `d7ba723c811170adb4b0c9e3d366d514826754b22b5a6447e17710fefb7fcaec`.
  §9 (p. 23): position-independent executables are `ET_DYN`.
- **SD Simplified Specifications** — https://www.sdcard.org/downloads/pls/
  and .../pls/archive/. Free of charge, but **not** redistributable; see
  "SD specifications" above for the terms and for the exact editions.
- **RPi 4 firmware device tree** — `boot/bcm2711-rpi-4-b.dtb` from
  https://github.com/raspberrypi/firmware, or the copy on the boot card.

---

## Pre-flight check

The protocol mentions `cargo xtask check-references`. This subcommand is
**not yet implemented** (tracked under STATUS.md). Until it lands, the
acquisition table above is the authoritative manual checklist. Tasks
that require a missing source must be marked blocked in
`docs/STATUS.md`.

For the sources pinned by checksum (the SD specifications, the firmware
device tree) the check is one line and needs no tooling:

```bash
shasum -a 256 references/sd-spec/*.pdf references/sd-spec/archive/*.pdf
```

Compare against the pin table above. A mismatch means you are reading a
different edition than the citations in `docs/` were written against —
that, not a missing file, is the failure mode that produced audit finding
S1.

---

## libshare (Arnaud Bercegeay) — added 2026-10-06

`references/libshare/` (gitignored, like the rest). Needed by the zView LDG
codecs (`-lshare`, `libshare_init/exit`). Analysis: `docs/analysis/libshare-2026-10-06.md`.

| Subdir / file | Origin | sha256 |
|---|---|---|
| `libshare-0.1.1.tgz` (original release 0.1.1, 2005-05-02; contains sources + prebuilt m68k `libshare.a` + test) | http://arnaud.bercegeay.free.fr/libshare/libshare-0.1.1.tgz (fetched 2026-10-06) | `2a6c58aaf0e99e2ac14c652b4333a504405fdef90e5a007878fc69750e2bdecb` |
| `orig-0.1.1/` | extracted from the tgz | — |
| `th-otto-zview-plugins-libshare/` (11 files, in-tree copy, 2 small patches vs. original) | https://github.com/th-otto/zview/tree/master/zview/plugins/libshare, repo HEAD `1cf03620ec3517f9a9a88e0d4f56cdabdf186ee7` at fetch time (fetched 2026-10-06) | per file, see `shasum -a 256` in that dir (e.g. libshare.c `788273ee…1d`, libshare.h `eb80c8e6…0f`) |

No license text in the sources; see the analysis file.
Since TASK/LDG-M3-B2 the same tgz (identical sha256) is pinned in the SDK as
`sdk/libshare/libshare-0.1.1.tgz` and built by `sdk/libshare/build-libshare.sh`;
the copy here stays the reference.

---

## USB-IF documents (`references/usb/`)

- `hid1_11.pdf` — Device Class Definition for HID 1.11 (usb.org).
- `hut1_5.pdf` — HID Usage Tables 1.5 (usb.org, `https://usb.org/sites/default/files/hut1_5.pdf`,
  fetched 2026-10-24, sha256 `f4ead217b01134f89199155ed616d4eeae9ad40a5f73dad9544002b592c07db3`).
  Keyboard/Keypad page (0x07): printed p. 89 (table), p. 95 (footnotes). Role: normative
  usage names; it states nothing about host behaviour (see `TASK/USB02.md` item 10).

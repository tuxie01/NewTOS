# EmuTOS boot flow — source extraction (cross-check for NewTOS)

> Phase-00-style extraction of how the **original EmuTOS** boots: what it
> searches for, what it boots from, and how the AUTO folder, the desktop
> and EmuCON relate. Every claim cites `file:line` in `references/emutos/`.
> The last section cross-checks NewTOS against it. Per CLAUDE.md Rule 1.

## Sources read

- `bios/startup.S` (reset → `_biosmain`), `bios/bios.c` (`biosmain`,
  `autoexec`, `bootstrap`, `exec_os`)
- `bios/blkdev.c` (`blkdev_boot`)
- `aes/gemstart.S` (`ui_start`), `aes/geminit.c` (`gem_main`, `count_accs`,
  `accdesk_start`), `aes/gemshlib.c` (`sh_main`, `sh_ldapp`)
- `desk/deskmain.c` (CLIITEM = "Execute EmuCON")
- `include/sysconf.h`, `include/aesdefs.h`, `aes/geminit.h`

## 1. EmuTOS *is* the ROM — it does not "boot from" a disk

EmuTOS is a **ROM operating system**: the CPU resets into ROM
(`startup.S`) and the OS code runs in place. So "booting" is not loading an
OS off a disk — the OS is already there. Reset → `startup.S` (hardware/
memory/exception init) → `jmp _biosmain` (`bios/startup.S:535`).

## 2. `biosmain()` — the boot sequence (`bios/bios.c:781`)

In order:

1. **`bios_init()`** (`:786`) — initialise the whole BIOS: hardware,
   drivers, GEMDOS, memory map, screen, the cookie jar, etc.
2. **`Drvmap()`** (`:791`) — the drive bitmap.
3. **Boot-device selection** (`:798`): on the *first* boot,
   `bootdev = blkdev_avail(DEFAULT_BOOTDEV) ? DEFAULT_BOOTDEV : FLOPPY_BOOTDEV`
   — i.e. the hard disk (C:) if present, else the floppy (A:). On warm
   boots the previously chosen `bootdev` persists (`:793-799`).
4. **Welcome / boot-select screen** (`:809`): if `show_initinfo`,
   `bootdev = initinfo(&shiftbits)` shows the EmuTOS banner and lets the
   user pick a boot drive; otherwise it just reads the shift keys.
5. **Shift-key boot flags** (`:816-820`): **ALT** held → `SKIP_HDD_BOOT`;
   **CTRL** held → `SKIP_AUTO_ACC` (skip the AUTO folder *and* the desk
   accessories).
6. **`blkdev_boot()`** (`:825`) — read the boot device's **boot sector**;
   if it carries an executable signature (checksum `== 0x1234`,
   `blkdev.c:284`) the boot-sector code is executed (`blkdev.c:223,250`).
   This is for disk-resident boot code / drivers, **not** the OS itself.
7. **`Dsetdrv(bootdev)`** (`:827`) + **`init_default_environment()`**
   (`:828`) — set the current drive and build the default `PATH=` env.
8. (optional) **early EmuCON** (`:834-844`, `BOOTFLAG_EARLY_CLI`) — runs
   the console *before* the GUI for debugging.
9. **`autoexec()`** (`:846`) — run the AUTO folder (§3).
10. **The shell** (`:850-866`):
    - if `cmdload` → `Pexec(PE_LOADGO, "COMMAND.PRG", …)` — a user shell on
      the boot drive (like Atari TOS), or
    - else `exec_os` → start the **default ROM shell**. `exec_os` is the OS
      header's `gm_init` (`bios/bios.c:534`) which is `ui_start` (`gemstart.S:46`)
      → **`gem_main`** (`geminit.c:753`) — i.e. the **AES + desktop**.
11. **`shutdown()`** (`:870`) if the shell ever returns.

## 3. The AUTO folder (`autoexec`, `bios/bios.c:697`)

- Skipped entirely if **CTRL** was held (`SKIP_AUTO_ACC`, `:703`).
- `Fsfirst("\\AUTO\\*.PRG", 7)` then `Fsnext` (`:714,731`) — every `*.PRG`
  in `\AUTO\` is run with `Pexec(PE_LOADGO, "\AUTO\<name>", "", NULL)`
  (`run_auto_program`, `:685-694`).
- **Order = directory order** (the order `Fsfirst`/`Fsnext` returns them),
  *not* alphabetical — same as Atari TOS.
- Runs **before** the GUI, on the GEMDOS default basepage. AUTO programs
  are typically TSRs (drivers, RAM disks) that stay resident.
- `bootstrap()` (`:707`, `DETECT_NATIVE_FEATURES`) tries an
  emulator-provided "native" TOS kernel first (ARAnyM/Hatari `nf_bootstrap`,
  `bios/bios.c:584`) — irrelevant on real hardware / NewTOS.

### The working directory an AUTO program starts in (DESK28, 2026-09-24)

**[CONTRACT]** — it is the **root of the boot drive**, and `autoexec` does
nothing to make it so.

`run_auto_program` (`bios/bios.c:685-695`) builds the *full* path
`"\AUTO\<name>"` and hands it to `Pexec(PE_LOADGO, path, "", NULL)`; there is
no `Dsetdrv` and no `Dsetpath` anywhere in `autoexec` (`:697-733`). The
child's directory is therefore purely inherited: `Pexec` copies the current
drive and the whole per-drive current-directory table out of the calling
process's basepage into the child's (`init_pd_files`, `bdos/proc.c:400-410`),
and the caller is the BIOS, whose state is `Dsetdrv(bootdev)`
(`bios/bios.c:827`) plus — for every drive it never `Dsetpath`ed — the **root**
DND that `ckdrv` installs on first use (`bdos/fsdrive.c:119-131`). An AUTO
program that opens `FOO.TXT` by a bare name reads `<bootdrive>:\FOO.TXT`, not
`\AUTO\FOO.TXT`.

The *copy* is the part that does not survive the translation to NewTOS: the
current drive and the per-drive current directories are single globals here,
the documented stand-in for `p_curdrv`/`p_curdir`
(`tos-api/src/bdos/fs/drive.rs`, `…/fs/path.rs`). On EmuTOS one AUTO
program's `Dsetpath` touches only its own copy, so the next one still starts
at the root; on a global it would be seen by the next program and by the
desktop after them. `crate::launch::set_auto_default_path` re-establishes the
BIOS's state — boot drive, root on every drive — before each program and once
after the loop, which is what makes the global behave like the copy.

Measured both ways with two AUTO programs, the first of which `Dsetpath`s
(`measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/`, §2): without the step
the second program reports `C:\SUB` and reads `C:\SUB\PROOF.TXT`, with it
`C:` and `C:\PROOF.TXT`.

## 4. `gem_main` → the AES, accessories and the shell loop

`gem_main` (`geminit.c:753`):

1. Clears the screen (`:763`).
2. **Reads `EMUDESK.INF`** (`INF_FILE_NAME = "A:\\EMUDESK.INF"`,
   `sysconf.h:44`) into `infbuf` (`:769`) — unless `SKIP_AUTO_ACC`.
3. `process_inf1()` (`:776`) parses the resolution/autorun part of the INF;
   may trigger a resolution change.
4. `count_accs()` (`:810`, `geminit.c:264`) scans **`\*.ACC`** in the root
   (`strcpy(D.g_work,"\\*.ACC")`, `:272`) — the desk accessories to load.
5. `set_aestrap()` (`:822`) installs the **AES trap (#2)**.
6. Inits the AES processes + the **SCRENMGR** process (`:888`, owns the
   mouse + keyboard).
7. **`aes_run_rom_program(accdesk_start)`** (`:894`) — loads the ACCs and
   enters the **shell loop** (`accdesk_start` → `sh_main`, `geminit.c:739`).

### The shell loop (`sh_main`, `gemshlib.c:692`)

```
sh_nextapp = isauto ? AUTORUN_APP : DESKTOP_APP   (:698)
do {
    rc = sh_ldapp(psh);          // run desktop / console / app  (:725)
} while (sh_doexec != SHW_SHUTDOWN && !gl_changerez)   (:727)
```

`sh_ldapp` (`gemshlib.c:592`) dispatches on `sh_nextapp`:

- **`DESKTOP_APP`** (`:600`) — runs the built-in desktop
  (`aes_run_rom_program(deskstart)`; default name `DEF_DESKTOP = "EMUDESK"`,
  `geminit.h:13`). This is the **default shell**.
- **`CONSOLE_APP`** (`:615`, `WITH_CLI`) — runs **EmuCON**
  (`aes_run_rom_program(coma_start)`); when it exits, `set_default_desktop`
  returns control to the desktop (`:618-619`). `DEF_CONSOLE = "EMUCON"`
  (`aesdefs.h:627`).
- **`NORMAL_APP`** (`:624`) — `sh_find` + `dos_exec(PE_LOADGO, …)` a normal
  application; afterwards `sh_nextapp` defaults back to the desktop
  (`:639-643`).
- **`AUTORUN_APP`** — a program named in the INF `#Z` autorun line, run
  without `sh_find`; on quit it returns to the desktop (`:659-664`).

The **desktop sets `sh_nextapp` for the next loop** by calling `shel_write`
— that is how clicking a program, or **"Execute EmuCON"** in the File menu
(`deskmain.c:751` `CLIITEM` → `pro_run(FALSE, DEF_CONSOLE, …)`), switches
what runs next. So **desktop and EmuCON are never both active**: it is one
sequential shell loop, the desktop is primary, EmuCON is on-demand and
returns to the desktop.

## 5. Files EmuTOS looks for (summary)

| What | Where | When |
|---|---|---|
| Boot sector | `bootdev` first sector | `blkdev_boot`, executed if `cksum==0x1234` |
| `\AUTO\*.PRG` | boot drive | before the GUI, directory order, CTRL skips |
| `A:\EMUDESK.INF` | drive A / boot | desktop config (rez, autorun, patterns, windows, icons) |
| `\*.ACC` | root | desk accessories, CTRL skips |
| `COMMAND.PRG` | boot drive | only if `cmdload` |
| `EMUDESK` / `EMUCON` | ROM (built-in) | default shell / on-demand console |

## 6. Cross-check with NewTOS (gaps)

NewTOS boots **AArch64-native** (`kernel8.img` via the RPi firmware), not
from ROM, so the *mechanism* differs — but the *post-init application flow*
is what we should mirror. Status of each EmuTOS step in NewTOS:

| EmuTOS step | NewTOS today | Gap |
|---|---|---|
| `bios_init` | `kernel_main_rust` (MMU, drivers, GEMDOS, FB, cookie jar) | ✅ equivalent |
| boot-device select + `initinfo` welcome | fixed; `initinfo::run()` banner only | no boot-drive picker / shift-key boot flags |
| `blkdev_boot` (boot sector) | n/a (native kernel) | — (not needed) |
| **`\AUTO\*.PRG`** autoexec | **none** | 🔴 no AUTO-folder execution at boot |
| `EMUDESK.INF` read | partial — only the `#Q` (g_patcol) line from `NEWDESK.INF` | 🔴 `#E`/`#W`/`#Z`/`#M`/`#T`/… not parsed |
| `\*.ACC` desk accessories | **none** | 🔴 no ACC loading |
| AES trap always live | ✅ AES trap installed | ✅ |
| **shell loop** (`sh_main`, `sh_nextapp`) | **none** — `desktop::run()` is hardwired | 🔴 no shell loop / `shel_write` dispatch; app launch + return-to-desktop missing |
| `EMUDESK` desktop | `desktop::run()` (our desktop) | ✅ runs as the shell |
| **`EMUCON`** console / "Execute EmuCON" | **not wired** | 🔴 menu item logs only; no console-app path |
| `COMMAND.PRG` shell | none | minor (optional in EmuTOS too) |

**Implications / next steps suggested by this cross-check:**

1. **AUTO-folder execution** — run `\AUTO\*.PRG` via `Pexec` before
   `desktop::run()` (needs the `Pexec` shell-loop path, which ties into the
   cooperative scheduler / Pwait work).
2. **A real shell loop** — replace the hardwired `desktop::run()` with a
   `sh_main`-style loop driven by `shel_write`/`sh_nextapp`, so the desktop
   can launch apps (and **EmuCON**) and have them return to the desktop.
   This is what makes "Execute EmuCON" and program launch work.
3. **Fuller `NEWDESK.INF`** — parse `#E` (settings), `#W` (window
   positions), `#Z` (autorun), `#M`/`#T`/`#O` (icons) beyond the current
   `#Q`.
4. **ACC loading** (`\*.ACC`) — later; needs the multi-process model.
5. (optional) boot-drive picker + shift-key boot flags (skip AUTO, etc.).

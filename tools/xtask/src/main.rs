//! NewTOS host-side build/run/test driver.
//!
//! Subcommands (PHASE-01 §D7):
//!   xtask build --board {rpi4|qemu-virt} [--release]
//!   xtask build-programs --board {rpi4|qemu-virt}   — programs/ (DESK43 U6b)
//!   xtask run [--release]                 — qemu-virt in QEMU, serial → stdout
//!   xtask test                             — host-side unit tests (tos-types, xtask)
//!
//! All commands shell out to `cargo`; this binary holds only the argument
//! plumbing and the QEMU invocation.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Resource-file invariants the build tools must keep (`tools/mkrsc.py`).
#[cfg(test)]
mod rsc_align_tests;
/// A40 — the tool leaves OBJECT coordinates as char-grid words; the loader
/// converts (`tools/mkrsc.py`, `tos-api/src/aes/resource.rs`).
#[cfg(test)]
mod rsc_obfix_tests;
/// A41 — the tool carries a new-format file's colour icons (CICONBLK/CICON)
/// as the application's C structs into NRSC v11/v12.
#[cfg(test)]
mod rsc_cicon_tests;
/// SER01 — the kernel's boot-argument token parser (`serial=on`). The kernel
/// crate cannot be built for the host, so its pure, dependency-free module is
/// compiled here by path and its own `#[cfg(test)]` tests run with the host
/// suite.
#[cfg(test)]
#[path = "../../../kernel/src/bootargs.rs"]
mod kernel_bootargs;
/// KEYS01 — the kernel's key auto-repeat state machine (EmuTOS
/// `do_key_repeat`, run polled). Pure module, compiled here by path like
/// `kernel_bootargs` so its tests run with the host suite.
#[cfg(test)]
#[path = "../../../kernel/src/key_repeat.rs"]
mod kernel_key_repeat;
/// INPUT01 — when the timer interrupt may run the input pump (the decision
/// and the one-at-a-time guard). Pure, compiled here by path like
/// `kernel_bootargs`.
#[cfg(test)]
#[path = "../../../kernel/src/input_gate.rs"]
mod kernel_input_gate;
/// FS08 — the SD driver's pure transfer planning (multi-block chunking,
/// Transfer Mode bits, SCR bus widths, the CMD18 OUT_OF_RANGE rule).
/// Dependency-free, compiled here by path like `kernel_bootargs`.
#[cfg(test)]
#[path = "../../../kernel/src/drivers/bcm2711/emmc_xfer.rs"]
mod kernel_emmc_xfer;
/// FS08 part 3 — the firmware ARM-clock property tags (max / measured / set
/// clock rate): message encoding and reply decoding. Pure, compiled here by
/// path like `kernel_bootargs`.
#[cfg(test)]
#[path = "../../../kernel/src/drivers/bcm2711/clock_tags.rs"]
mod kernel_clock_tags;
/// USB01 F1 — the xHCI producer ring (Link TRB + Toggle Cycle, PCS flip on
/// wrap). Pure, compiled here by path like `kernel_bootargs`.
#[cfg(test)]
#[path = "../../../kernel/src/drivers/usb/xhci/ring.rs"]
mod kernel_xhci_ring;
/// USB01 F2/F8 — the configuration-descriptor scan (HID boot interface
/// selection by class/subclass/protocol, skipped interfaces). Pure, compiled
/// here by path like `kernel_bootargs`.
#[cfg(test)]
#[path = "../../../kernel/src/drivers/usb/config_desc.rs"]
mod kernel_usb_config_desc;
/// USB01 step 2 (K1) — the USB snapshot store and the `Ssystem(2022)`
/// encoder (`docs/api/usb/usb-snapshot.md` §2). Pure, compiled here by path
/// like `kernel_bootargs`.
#[cfg(test)]
#[path = "../../../kernel/src/drivers/usb/snapshot.rs"]
#[allow(dead_code)]
mod kernel_usb_snapshot;

/// DESK43 U5a (guard G3) — the desktop's integer constants against a
/// checked-in snapshot of EmuTOS' `#define`s.
mod emutos_consts;

/// DESK43 U6b (decision D3) — the kernel image's embedded programs: the list,
/// the per-board program directory and the generated table, shared with
/// `kernel/build.rs` and compiled here by path. `build-programs` writes into
/// [`kernel_embed::programs_dir`]; the module's own `#[cfg(test)]` tests (the
/// hard error on a missing blob, the generated `include_bytes!`) run with the
/// host suite.
#[path = "../../../kernel/embed.rs"]
#[allow(dead_code)]
mod kernel_embed;

/// DESK43 U6b — the programs `build-programs` builds, their board features
/// (X03 pass-through) and the version/machine strings of the desktop vs the
/// boot screen (review §6, X03 row).
#[cfg(test)]
mod programs_tests;

/// ARCH03 E8b — the loader's ELF parser, compiled here by path (it is pure,
/// `no_std`, dependency-free) so `xtask prgflags` reads the PRGFLAGS note
/// exactly the way the kernel does.
#[path = "../../../tos-api/src/bdos/process/elf.rs"]
#[allow(dead_code)]
mod loader_elf;

const KERNEL_TARGET: &str = "aarch64-unknown-none-softfloat";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = match args.next() {
        Some(s) => s,
        None => {
            print_usage();
            return ExitCode::from(2);
        }
    };

    let rest: Vec<String> = args.collect();
    let result = match cmd.as_str() {
        "build" => cmd_build(&rest),
        "build-programs" => cmd_build_programs(&rest),
        "emutos-consts" => emutos_consts::cmd(&workspace_root(), &rest),
        "gen-font" => cmd_gen_font(),
        "gen-fsel-rsc" => cmd_gen_fsel_rsc(),
        "prgflags" => cmd_prgflags(&rest),
        "run" => cmd_run(&rest),
        "test" => cmd_test(&rest),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown subcommand: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_usage() {
    eprintln!(
        "\
Usage: cargo xtask <command> [options]

Commands:
  build --board {{rpi4|qemu-virt}} [--release] [--features a,b]
                                 — the board's programs (build-programs),
                                   then the kernel, which embeds them
  build-programs --board {{rpi4|qemu-virt}}
                                 — the native Rust programs in programs/
                                   (programs/build.sh), built with the
                                   board feature, into
                                   target/programs/<board>/ (DESK43 U6b)
  emutos-consts [--update] [-v]  — compare the desktop's integer constants
                                   with EmuTOS' #defines of the same name
                                   (snapshot tools/xtask/emutos-defines.snapshot;
                                   --update regenerates it from a local
                                   references/emutos; DESK43 U5a, guard G3)
  gen-font                       — extract the EmuTOS 6x6/8x8/8x16 ST system
                                   fonts into tos-api/src/vdi/font_data.rs
  gen-fsel-rsc                   — convert the EmuTOS AES file-selector
                                   resource (references/emutos/aes/gem.rsc)
                                   into the embedded native blob
                                   tos-api/src/aes/gem_fsel.nrsc
  prgflags <file> [<value>]      — show a NewTOS program's PRGFLAGS (its
                                   ELF note, ARCH03 E8b) or set them to
                                   <value> (0x... or decimal) in place;
                                   the role of Atari's PRGFLAGS.PRG
  run [--board {{rpi4|qemu-virt}}] [--release] [--window] [--debug]
      [--sd <img>] [--features a,b] [--usb-extra] [--usb-dev <spec>]...
                                   — boot in QEMU (virt by default, or
                                     raspi4b with --board rpi4); --window
                                     shows the framebuffer, --debug halts
                                     at reset with a gdbstub on :1234;
                                     --features adds optional kernel
                                     features (e.g. aes-trace);
                                     qemu-virt only (USB01 K11): besides the
                                     default usb-kbd + usb-mouse,
                                     --usb-extra adds a usb-tablet (root
                                     port 3) and a usb-hub (root port 4)
                                     with a usb-mouse + usb-kbd behind it,
                                     --usb-dev adds one QEMU -device spec
                                     on the xHCI bus `usb.0` (repeatable,
                                     e.g. usb-tablet or
                                     usb-mouse,bus=usb.0,port=4.3)
  test                           — run every host-runnable test in the
                                   workspace (tos-api, tos-types, xtask);
                                   the kernel is not host-linkable
  help
"
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Board {
    Rpi4,
    QemuVirt,
}

impl Board {
    fn feature(self) -> &'static str {
        match self {
            Board::Rpi4 => "board-rpi4",
            Board::QemuVirt => "board-qemu-virt",
        }
    }
    /// The board's name as the kernel spells it (`BOARD_NAME`, `lib.rs`) and
    /// as its program directory is called (`kernel/embed.rs`).
    fn name(self) -> &'static str {
        match self {
            Board::Rpi4 => "rpi4",
            Board::QemuVirt => "qemu-virt",
        }
    }
    /// Every board, for the checks that must hold for each.
    #[cfg(test)]
    const ALL: [Board; 2] = [Board::Rpi4, Board::QemuVirt];
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "rpi4" => Ok(Board::Rpi4),
            "qemu-virt" => Ok(Board::QemuVirt),
            other => Err(format!("unknown board: {other}")),
        }
    }
}

/// Parsed common build options: board, release profile, and any extra cargo
/// features (`--features a,b`) to enable on top of the board feature. The
/// extra list is what turns optional instruments on — e.g. `--features
/// aes-trace` for the AES trap-path tracer (`tos_api::aes::trace`).
struct BuildOpts {
    board: Option<Board>,
    release: bool,
    features: Vec<String>,
}

fn parse_board_and_profile(args: &[String]) -> Result<BuildOpts, String> {
    let mut board = None;
    let mut release = false;
    let mut features: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--board" => {
                let v = it.next().ok_or("--board requires a value")?;
                board = Some(Board::from_str(v)?);
            }
            "--release" => release = true,
            "--features" => {
                let v = it.next().ok_or("--features requires a value")?;
                features.extend(
                    v.split([',', ' '])
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                );
            }
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    Ok(BuildOpts { board, release, features })
}

fn cargo() -> Command {
    let cargo_bin = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    Command::new(cargo_bin)
}

fn workspace_root() -> PathBuf {
    // tools/xtask/src/main.rs → ../../..
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap().parent().unwrap().to_path_buf()
}

fn run(mut cmd: Command) -> Result<(), String> {
    let printable = format!("{cmd:?}");
    let status = cmd
        .status()
        .map_err(|e| format!("failed to spawn `{printable}`: {e}"))?;
    if !status.success() {
        return Err(format!("`{printable}` exited with {status}"));
    }
    Ok(())
}

fn cmd_build(args: &[String]) -> Result<(), String> {
    let opts = parse_board_and_profile(args)?;
    let board = opts.board.ok_or("build: --board {rpi4|qemu-virt} is required")?;
    build_kernel(board, opts.release, &opts.features)?;
    if board == Board::Rpi4 {
        produce_kernel8_img(opts.release)?;
    }
    Ok(())
}

/// Extract the three EmuTOS Atari-ST system-font rasters (6×6, 8×8, 8×16)
/// into a committed Rust module (`tos-api/src/vdi/font_data.rs`).
///
/// Each source `dat_table` (`references/emutos/bios/fnt_st_{6x6,8x8,8x16}.c`)
/// is a monochrome raster of `form_height` scanlines, each `form_width`
/// bytes wide (big-endian UWORDs), with all 256 characters laid out
/// horizontally at `max_cell_width` px each:
///   - 8×16: 128 words/row × 16 rows, 8 px/char  → one `[u8; 16]` per char
///   - 8×8 : 128 words/row × 8 rows,  8 px/char  → one `[u8; 8]`  per char
///   - 6×6 : 96 words/row  × 6 rows,  6 px/char  → one `[u8; 6]`  per char
///
/// For the byte-aligned 8-px fonts, glyph row r of char c is simply byte c
/// of raster row r. For the 6-px font, char c occupies bits `[c*6, c*6+6)`
/// of the scanline; we pack those 6 pixels MSB-left into a byte (bits 7..2),
/// matching the `bit 7 = leftmost pixel` convention used by the raster
/// backend for every size. The 8×16 output is byte-identical to the
/// previous single-font extraction.
fn cmd_gen_font() -> Result<(), String> {
    let root = workspace_root();
    let bios = root.join("references").join("emutos").join("bios");

    // Pull the `dat_table` initialiser out of a font source and collect its
    // hex words (the array size may be spelled `[]` or `[N]`).
    let extract = |file: &str, want: usize| -> Result<Vec<u16>, String> {
        let path = bios.join(file);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("read {}: {e} (is the EmuTOS reference checked out?)", path.display()))?;
        let start = text
            .find("dat_table")
            .ok_or_else(|| format!("dat_table not found in {file}"))?;
        let brace = text[start..].find('{').ok_or("dat_table: no '{'")? + start;
        let end = text[brace..].find("};").ok_or("dat_table: no '};'")? + brace;
        let mut words: Vec<u16> = Vec::with_capacity(want);
        for tok in text[brace..end].split(|c: char| !c.is_ascii_hexdigit() && c != 'x' && c != 'X') {
            let t = tok.trim();
            if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                if let Ok(v) = u16::from_str_radix(hex, 16) {
                    words.push(v);
                }
            }
        }
        if words.len() != want {
            return Err(format!("expected {want} dat_table words in {file}, got {}", words.len()));
        }
        Ok(words)
    };

    // Byte-aligned fonts (8 px/char): high byte of word c/2 if c even, low
    // byte if odd (big-endian).
    let byte_at = |w: &[u16], words_per_row: usize, row: usize, c: usize| -> u8 {
        let word = w[row * words_per_row + c / 2];
        if c % 2 == 0 {
            (word >> 8) as u8
        } else {
            (word & 0xFF) as u8
        }
    };

    let f8x16 = extract("fnt_st_8x16.c", 2048)?;
    let f8x8 = extract("fnt_st_8x8.c", 1024)?;
    let f6x6 = extract("fnt_st_6x6.c", 576)?;

    let mut s = String::new();
    s.push_str("//! GENERATED by `cargo xtask gen-font` — do not edit by hand.\n");
    s.push_str("//! Atari ST system fonts (6×6, 8×8, 8×16), transposed from EmuTOS\n");
    s.push_str("//! `bios/fnt_st_{6x6,8x8,8x16}.c` (`dat_table`). One byte-array\n");
    s.push_str("//! (`form_height` rows) per character; bit 7 of each byte is the\n");
    s.push_str("//! leftmost pixel. The 6×6 glyphs pack their 6 pixels into bits 7..2.\n\n");

    s.push_str("/// 256 glyphs of the 8×16 ST system font (row-major, MSB = left).\n");
    s.push_str("pub static SYSTEM_FONT_8X16: [[u8; 16]; 256] = [\n");
    for c in 0..256usize {
        s.push_str("    [");
        for r in 0..16usize {
            s.push_str(&format!("0x{:02x},", byte_at(&f8x16, 128, r, c)));
        }
        s.push_str("],\n");
    }
    s.push_str("];\n\n");

    s.push_str("/// 256 glyphs of the 8×8 ST system font (row-major, MSB = left).\n");
    s.push_str("pub static SYSTEM_FONT_8X8: [[u8; 8]; 256] = [\n");
    for c in 0..256usize {
        s.push_str("    [");
        for r in 0..8usize {
            s.push_str(&format!("0x{:02x},", byte_at(&f8x8, 128, r, c)));
        }
        s.push_str("],\n");
    }
    s.push_str("];\n\n");

    s.push_str("/// 256 glyphs of the 6×6 ST system font (row-major, 6 px in bits 7..2).\n");
    s.push_str("pub static SYSTEM_FONT_6X6: [[u8; 6]; 256] = [\n");
    for c in 0..256usize {
        s.push_str("    [");
        for r in 0..6usize {
            // Char c occupies bits [c*6, c*6+6) of the 96-word (1536-bit) row.
            let mut val = 0u8;
            for p in 0..6usize {
                let gbit = c * 6 + p;
                let word = f6x6[r * 96 + gbit / 16];
                let pix = ((word >> (15 - (gbit % 16))) & 1) as u8;
                val |= pix << (7 - p);
            }
            s.push_str(&format!("0x{val:02x},"));
        }
        s.push_str("],\n");
    }
    s.push_str("];\n");

    let out = root
        .join("tos-api")
        .join("src")
        .join("vdi")
        .join("font_data.rs");
    std::fs::write(&out, &s).map_err(|e| format!("write {}: {e}", out.display()))?;
    println!("xtask: wrote {} (6×6 + 8×8 + 8×16 glyphs)", out.display());
    Ok(())
}

/// Convert the EmuTOS AES file-selector resource into the native NRSC blob
/// embedded by `tos-api/src/aes/fsel.rs` (`GEM_FSEL_RSC`).
///
/// Like [`cmd_gen_font`], this regenerates a committed build artifact from an
/// EmuTOS reference. The conversion itself is `tools/mkrsc.py convert` (the
/// classic big-endian `gem.rsc` → little-endian LP64 NRSC with byte-offset
/// pointers); we only drive it so the regeneration command is reproducible and
/// discoverable next to the other `gen-*` commands. The kernel then embeds the
/// resulting blob and relocates it in place — no disk read, exactly as EmuTOS
/// links `gem.rsc` into ROM.
fn cmd_gen_fsel_rsc() -> Result<(), String> {
    let root = workspace_root();
    let script = root.join("tools").join("mkrsc.py");
    let src = root.join("references").join("emutos").join("aes").join("gem.rsc");
    let out = root.join("tos-api").join("src").join("aes").join("gem_fsel.nrsc");
    let python = env::var_os("PYTHON").unwrap_or_else(|| "python3".into());
    let mut c = Command::new(python);
    c.current_dir(&root)
        .arg(&script)
        .arg("convert")
        .arg(&src)
        .arg(&out);
    println!("xtask: converting {} -> {}", src.display(), out.display());
    run(c)
}

/// The native Rust programs in `programs/` (DESK43 U6a/U6b): the crate and
/// the file `build-programs` writes it to in the board's directory. Every
/// `staticlib` member of `programs/Cargo.toml` is listed here and declares the
/// two board features (host test `programs_tests`). `NEWDESK.NAP` (DESK43 U9)
/// is the desktop program the kernel image embeds (`kernel/embed.rs`).
const PROGRAMS: &[(&str, &str)] = &[("hello-gem", "HELLOGEM.NAP"), ("newdesk", "NEWDESK.NAP")];

/// Kernel features a program build takes over (DESK43 U9-4): `(crate,
/// feature)`. Since the desktop is the EL0 program `NEWDESK`, the kernel's
/// `selftest-desk` must reach that program — the self-tests run inside the
/// desktop (`programs/desktop/src/lib.rs`), and since DESK43 U10 the kernel
/// has no copy of the desktop of its own (the kernel feature is only this
/// switch). Every listed feature is declared by the crate (host test
/// `programs_tests`).
const PROGRAM_FEATURES: &[(&str, &str)] = &[("newdesk", "selftest-desk")];

fn cmd_build_programs(args: &[String]) -> Result<(), String> {
    let opts = parse_board_and_profile(args)?;
    let board = opts.board.ok_or("build-programs: --board {rpi4|qemu-virt} is required")?;
    if opts.release || !opts.features.is_empty() {
        return Err("build-programs: programs are always release builds; only --board is accepted".into());
    }
    build_programs(board, &[])
}

/// Build every program of [`PROGRAMS`] for `board` with `programs/build.sh`
/// (toolchain T1: `sdk/crt0.S` + the crate's `staticlib` + `sdk/newtos.ld`),
/// passing the board feature through (X03: the desktop's About-box machine
/// line is a per-board build constant, so a separately built desktop needs the
/// board as the kernel has it). One output directory per board,
/// `target/programs/<board>/` — the directory the kernel build embeds from
/// (`kernel/embed.rs`). `kernel_features` are the kernel build's optional
/// features; those in [`PROGRAM_FEATURES`] reach their program.
fn build_programs(board: Board, kernel_features: &[String]) -> Result<(), String> {
    let root = workspace_root();
    let dir = kernel_embed::programs_dir(&root, board.name());
    std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    for (krate, file) in PROGRAMS {
        println!("xtask: building program {krate} for {} -> {}", board.name(), dir.join(file).display());
        run(program_build_command(&root, board, krate, file, kernel_features))?;
    }
    Ok(())
}

/// The `--features` list of one program build: the board feature, then every
/// kernel feature [`PROGRAM_FEATURES`] hands to `krate`.
fn program_features(board: Board, krate: &str, kernel_features: &[String]) -> String {
    let mut feats = board.feature().to_string();
    for (k, f) in PROGRAM_FEATURES {
        if *k == krate && kernel_features.iter().any(|kf| kf == f) {
            feats.push(',');
            feats.push_str(f);
        }
    }
    feats
}

/// `bash programs/build.sh <crate> target/programs/<board>/<file> --
/// --features board-<board>[,…]` — one program for one board.
fn program_build_command(root: &Path, board: Board, krate: &str, file: &str, kernel_features: &[String]) -> Command {
    let out = kernel_embed::programs_dir(root, board.name()).join(file);
    let mut c = Command::new("bash");
    c.current_dir(root)
        .arg(root.join("programs").join("build.sh"))
        .arg(krate)
        .arg(out)
        .arg("--")
        .arg("--features")
        .arg(program_features(board, krate, kernel_features));
    c
}

fn build_kernel(board: Board, release: bool, extra: &[String]) -> Result<(), String> {
    // The kernel build depends on the program build (DESK43 D3): the image
    // embeds the board's programs (`kernel/embed.rs`), and `kernel/build.rs`
    // stops with a hard error when one has not been built. Build them first,
    // with the kernel features that belong to them (`selftest-desk`, U9-4).
    build_programs(board, extra)?;
    // The board feature is always present; `extra` appends optional ones
    // (e.g. `aes-trace`) to the same comma-separated `--features` list.
    let mut feats = board.feature().to_string();
    for f in extra {
        feats.push(',');
        feats.push_str(f);
    }
    let mut c = cargo();
    c.current_dir(workspace_root());
    c.arg("build")
        .arg("--package")
        .arg("kernel")
        .arg("--no-default-features")
        .arg("--features")
        .arg(&feats);
    if release {
        c.arg("--release");
    }
    println!("xtask: building kernel for {} ({})",
        feats,
        if release { "release" } else { "debug" });
    run(c)
}

fn kernel_elf_path(release: bool) -> PathBuf {
    let profile = if release { "release" } else { "debug" };
    workspace_root()
        .join("target")
        .join(KERNEL_TARGET)
        .join(profile)
        .join("kernel")
}

/// Convert the kernel ELF to `kernel8.img` (raw binary) using `rust-objcopy`
/// from the `llvm-tools-preview` component (installed via rust-toolchain).
fn produce_kernel8_img(release: bool) -> Result<(), String> {
    let elf = kernel_elf_path(release);
    if !elf.exists() {
        return Err(format!("kernel ELF not found at {}", elf.display()));
    }
    let out = workspace_root().join("kernel8.img");
    let objcopy = locate_rust_objcopy()?;
    let mut c = Command::new(objcopy);
    c.arg("-O").arg("binary").arg(&elf).arg(&out);
    println!("xtask: objcopy → {}", out.display());
    run(c)
}

fn locate_rust_objcopy() -> Result<PathBuf, String> {
    // `rustc --print sysroot` then look for
    // `lib/rustlib/<host>/bin/llvm-objcopy`.
    let out = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .map_err(|e| format!("rustc --print sysroot failed: {e}"))?;
    if !out.status.success() {
        return Err("rustc --print sysroot failed".into());
    }
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let host = host_triple()?;
    let candidate = Path::new(&sysroot)
        .join("lib")
        .join("rustlib")
        .join(&host)
        .join("bin")
        .join("llvm-objcopy");
    if !candidate.exists() {
        return Err(format!(
            "llvm-objcopy not found at {}. Install with `rustup component add llvm-tools-preview`.",
            candidate.display()
        ));
    }
    Ok(candidate)
}

fn host_triple() -> Result<String, String> {
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|e| format!("rustc -vV failed: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            return Ok(rest.trim().to_string());
        }
    }
    Err("could not parse host triple from rustc -vV".into())
}

/// The QEMU USB devices of a `qemu-virt` run (USB01 K11,
/// `docs/api/usb/usb-snapshot.md` §5): the xHCI controller with the default
/// `usb-kbd` + `usb-mouse` — the default line is unchanged; only when devices
/// are added does the controller get `id=usb` (bus `usb.0`, so specs can name
/// ports) and the default pair takes root ports 1 and 2 —, then with `extra` the T5 topology (a report-protocol
/// `usb-tablet` on root port 3; a `usb-hub` on root port 4 with a `usb-mouse`
/// on its port 1 and a `usb-kbd` on its port 2), then every `devs` entry as
/// one more `-device`.
fn qemu_usb_args(extra: bool, devs: &[String]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let mut dev = |spec: &str| {
        v.push("-device".into());
        v.push(spec.into());
    };
    dev(if extra || !devs.is_empty() { "qemu-xhci,id=usb" } else { "qemu-xhci" });
    dev("usb-kbd");
    dev("usb-mouse");
    if extra {
        dev("usb-tablet,bus=usb.0,port=3");
        dev("usb-hub,bus=usb.0,port=4");
        dev("usb-mouse,bus=usb.0,port=4.1");
        dev("usb-kbd,bus=usb.0,port=4.2");
    }
    for d in devs {
        dev(d);
    }
    v
}

#[cfg(test)]
mod qemu_usb_args_tests {
    use super::qemu_usb_args;

    #[test]
    fn default_is_keyboard_and_mouse() {
        assert_eq!(
            qemu_usb_args(false, &[]),
            ["-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse"]
        );
    }

    #[test]
    fn extra_adds_tablet_and_hub_tree_after_the_default_pair() {
        let v = qemu_usb_args(true, &["usb-tablet".to_string()]);
        let devs: Vec<&str> = v.iter().skip(1).step_by(2).map(|s| s.as_str()).collect();
        assert_eq!(
            devs,
            [
                "qemu-xhci,id=usb",
                "usb-kbd",
                "usb-mouse",
                "usb-tablet,bus=usb.0,port=3",
                "usb-hub,bus=usb.0,port=4",
                "usb-mouse,bus=usb.0,port=4.1",
                "usb-kbd,bus=usb.0,port=4.2",
                "usb-tablet",
            ]
        );
        assert!(v.iter().step_by(2).all(|s| s == "-device"));
    }
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    let window = args.iter().any(|a| a == "--window");
    let debug = args.iter().any(|a| a == "--debug");
    // USB01 K11: extra USB devices on qemu-virt (`qemu_usb_args`).
    let usb_extra = args.iter().any(|a| a == "--usb-extra");
    let mut usb_devs: Vec<String> = Vec::new();
    {
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if a == "--usb-dev" {
                usb_devs.push(it.next().ok_or("--usb-dev requires a QEMU -device spec")?.clone());
            }
        }
    }
    // `--sd <path>` attaches a raw SD-card image (the kernel's C: drive).
    // Defaults to /tmp/sd.img if present, so the desktop sees its drives.
    let sd = args
        .iter()
        .position(|a| a == "--sd")
        .and_then(|i| args.get(i + 1).cloned())
        .or_else(|| {
            let def = std::path::Path::new("/tmp/sd.img");
            def.exists().then(|| def.to_string_lossy().into_owned())
        });
    let filtered: Vec<String> = {
        let mut out = Vec::new();
        let mut skip = false;
        for a in args {
            if skip {
                skip = false;
                continue;
            }
            if a == "--window" || a == "--debug" || a == "--usb-extra" {
                continue;
            }
            if a == "--sd" || a == "--usb-dev" {
                skip = true;
                continue;
            }
            out.push(a.clone());
        }
        out
    };
    let opts = parse_board_and_profile(&filtered)?;
    let release = opts.release;
    if opts.board == Some(Board::Rpi4) {
        if usb_extra || !usb_devs.is_empty() {
            return Err("--usb-extra / --usb-dev: qemu-virt only (raspi4b has no USB controller)".into());
        }
        return cmd_run_rpi4(release, window, debug, sd.as_deref(), &opts.features);
    }
    build_kernel(Board::QemuVirt, release, &opts.features)?;
    let elf = kernel_elf_path(release);

    // QEMU's `-kernel` only sets up the FDT bootloader (DTB pointer in
    // `x0`, Linux ARM64 boot protocol) when the kernel is loaded as a
    // flat binary. With an ELF the loader jumps directly to the entry
    // and `x0` is zero, so Phase 03 D8 (runtime DRAM detection) cannot
    // see the auto-generated device tree. Convert to a flat image.
    let img = workspace_root()
        .join("target")
        .join(KERNEL_TARGET)
        .join(if release { "release" } else { "debug" })
        .join("kernel.img");
    let objcopy = locate_rust_objcopy()?;
    let mut c = Command::new(objcopy);
    c.arg("-O").arg("binary").arg(&elf).arg(&img);
    println!("xtask: objcopy → {}", img.display());
    run(c)?;

    // Notes:
    //   * `gic-version=2` — our kernel/src/drivers/gic.rs targets GICv2;
    //     QEMU's `virt` machine defaults to GICv3 with cortex-a72.
    //   * `-m 272M` — 16 MiB ST-RAM slot + 256 MiB TT-RAM slot, the
    //     Atari-style additive memory map. See
    //     `kernel/src/mm/layout.rs` board constants. The kernel reads
    //     the actual size from the firmware-supplied DTB at boot
    //     (Phase 03 D8) and adjusts TT-RAM accordingly.
    //   * `-display none -serial stdio` — bind the PL011 directly to the
    //     controlling TTY without QEMU's mux chardev (`mon:stdio`),
    //     which buffers piped stdin and breaks PL011 RX.
    let mut c = Command::new("qemu-system-aarch64");
    c.args([
        "-M", "virt,gic-version=2",
        "-cpu", "cortex-a72",
        "-m", "272M",
        // `ramfb` gives the otherwise-display-less virt machine a simple
        // linear framebuffer (configured by the kernel via fw_cfg). It
        // is inert under `-display none` but its surface can still be
        // captured with a QMP `screendump`; pass `-display cocoa`
        // (macOS) to watch it live.
        "-device", "ramfb",
    ]);
    // USB host stack (PHASE-16): an xHCI controller on the PCIe bus with a
    // keyboard + mouse, so the desktop is interactive in the emulator; more
    // devices on request (USB01 K11).
    c.args(qemu_usb_args(usb_extra, &usb_devs));
    if window {
        // Native macOS window so the framebuffer is visible. Serial
        // still goes to this terminal.
        c.args(["-display", "cocoa"]);
    } else {
        c.args(["-display", "none"]);
    }
    c.args([
        "-serial", "stdio",
        "-no-reboot",
        "-kernel",
    ])
    .arg(&img);
    if debug {
        // Halt at reset and expose QEMU's gdbstub on tcp::1234.
        c.args(["-S", "-s"]);
    }
    println!(
        "xtask: launching QEMU{}{} (Ctrl-C to quit)",
        if window { " with display window" } else { "" },
        if debug { " [gdbstub :1234, halted]" } else { "" },
    );
    run(c)
}

/// `xtask run --board rpi4`: boot the RPi4 kernel on QEMU's `raspi4b`
/// machine (the debuggable first vehicle for the VideoCore framebuffer).
/// `--window` shows the emulated HDMI output; `--debug` halts at reset
/// with a gdbstub on :1234. There is no serial dependency — the kernel's
/// PL011 is wired to serial0 (`-serial stdio`), but the framebuffer is
/// the intended output channel on real hardware.
fn cmd_run_rpi4(
    release: bool,
    window: bool,
    debug: bool,
    sd: Option<&str>,
    features: &[String],
) -> Result<(), String> {
    build_kernel(Board::Rpi4, release, features)?;
    produce_kernel8_img(release)?;
    let img = workspace_root().join("kernel8.img");

    let mut c = Command::new("qemu-system-aarch64");
    c.args(["-M", "raspi4b", "-no-reboot"]);
    if window {
        c.args(["-display", "cocoa"]);
    } else {
        c.args(["-display", "none"]);
    }
    // serial0 on raspi4b is the PL011 (UART0) the kernel drives.
    c.args(["-serial", "stdio", "-kernel"]).arg(&img);
    // SER01: board-rpi4 keeps its serial output off unless the boot
    // arguments contain `serial=on`. The kernel reads them with the mailbox
    // tag "Get command line", which QEMU's raspi4b firmware model answers
    // with the `-append` string — the emulator's stand-in for `cmdline.txt`.
    c.args(["-append", "serial=on"]);
    // Attach the SD-card image (the kernel's C: drive) so the desktop sees
    // its drives + directories.
    if let Some(path) = sd {
        c.args(["-drive", &format!("file={path},if=sd,format=raw")]);
    }
    if debug {
        c.args(["-S", "-s"]);
    }
    println!(
        "xtask: launching QEMU raspi4b{}{}{} (Ctrl-C to quit)",
        if window { " with display window" } else { "" },
        match sd {
            Some(p) => format!(" [sd: {p}]"),
            None => String::new(),
        },
        if debug { " [gdbstub :1234, halted]" } else { "" },
    );
    run(c)
}

/// The workspace packages whose tests can run on the host.
///
/// `kernel` is not one of them and cannot become one: it is `#![no_std]`,
/// pinned to `aarch64-unknown-none-softfloat` and linked against a board
/// linker script, so `cargo test -p kernel` fails in its build script before
/// it ever compiles a test. Everything else in the workspace is listed here —
/// leaving `tos-api` out (as this command did until 2026-09-04) meant the
/// command ran the two packages that have no tests at all and reported
/// success without executing a single one.
const HOST_TEST_PACKAGES: [&str; 3] = ["tos-api", "tos-types", "xtask"];

fn cmd_test(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err(format!("test: unexpected arguments: {args:?}"));
    }
    // Host-runnable tests only: kernel cannot be linked for the host.
    let mut c = cargo();
    c.current_dir(workspace_root());
    c.arg("test");
    for p in HOST_TEST_PACKAGES {
        c.args(["--package", p]);
    }
    run(c)
}

// ── prgflags (ARCH03 E8b) ───────────────────────────────────────────────

/// The PRGFLAGS note of a NewTOS program: its value and the file offset of
/// the descriptor word. Uses the kernel's own parser ([`loader_elf`]), so a
/// file this tool accepts is a file the loader accepts the note of.
fn prgflags_locate(bytes: &[u8]) -> Result<(u32, usize), String> {
    let img = loader_elf::parse(bytes, bytes.len() as u64)
        .map_err(|e| format!("not a loadable NewTOS program: {e:?}"))?;
    let mut found: Option<(u32, usize)> = None;
    for seg in img.notes() {
        let start = seg.file_offset as usize;
        let notes = &bytes[start..start + seg.file_size as usize];
        if let Some((v, off)) = loader_elf::find_prgflags_note(notes, seg.align)
            .map_err(|e| format!("malformed note segment: {e:?}"))?
        {
            if found.is_some() {
                return Err("two PRGFLAGS notes (the loader refuses this file)".into());
            }
            found = Some((v, start + off));
        }
    }
    found.ok_or_else(|| {
        "no PRGFLAGS note (link the program with the SDK crt0.S; the loader \
         gives it PF_STANDARD)"
            .into()
    })
}

/// Rewrite the PRGFLAGS note value in `bytes`; returns the old value.
fn prgflags_patch(bytes: &mut [u8], v: u32) -> Result<u32, String> {
    let (old, at) = prgflags_locate(bytes)?;
    bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    Ok(old)
}

/// The `PF_*` bits by name (EmuTOS `include/bdosdefs.h:100-103`; bits
/// 31-28 = the TT-RAM "would-like" quantum, `bdos/proc.c:482`).
fn prgflags_describe(v: u32) -> String {
    let mut parts = Vec::new();
    for (bit, name) in [(1, "PF_FASTLOAD"), (2, "PF_TTRAMLOAD"), (4, "PF_TTRAMMEM")] {
        if v & bit != 0 {
            parts.push(name.to_string());
        }
    }
    let q = v >> 28;
    if q != 0 {
        parts.push(format!("TPA quantum {} (x 128 KiB)", q + 1));
    }
    let rest = v & 0x0FFF_FFF8;
    if rest != 0 {
        parts.push(format!("other bits 0x{rest:08x}"));
    }
    if parts.is_empty() {
        "none".into()
    } else {
        parts.join(" | ")
    }
}

fn parse_u32(s: &str) -> Result<u32, String> {
    let r = match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u32::from_str_radix(h, 16),
        None => s.parse::<u32>(),
    };
    r.map_err(|_| format!("not a 32-bit value: {s}"))
}

fn cmd_prgflags(args: &[String]) -> Result<(), String> {
    let (file, value) = match args {
        [f] => (f, None),
        [f, v] => (f, Some(parse_u32(v)?)),
        _ => return Err("usage: cargo xtask prgflags <file> [<value>]".into()),
    };
    let mut bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
    match value {
        None => {
            let (v, _) = prgflags_locate(&bytes).map_err(|e| format!("{file}: {e}"))?;
            println!("{file}: PRGFLAGS 0x{v:08x} ({})", prgflags_describe(v));
        }
        Some(v) => {
            let old = prgflags_patch(&mut bytes, v).map_err(|e| format!("{file}: {e}"))?;
            std::fs::write(file, &bytes).map_err(|e| format!("{file}: {e}"))?;
            println!(
                "{file}: PRGFLAGS 0x{old:08x} -> 0x{v:08x} ({})",
                prgflags_describe(v)
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod prgflags_tests {
    use super::*;

    /// A minimal ET_DYN with one PT_LOAD and one PT_NOTE holding the SDK's
    /// 24-byte note (value `v`) at file offset 0x1040.
    fn image(v: Option<u32>) -> Vec<u8> {
        let mut b = vec![0u8; 0x1100];
        b[0..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        b[6] = 1;
        b[16..18].copy_from_slice(&3u16.to_le_bytes());
        b[18..20].copy_from_slice(&183u16.to_le_bytes());
        b[20..24].copy_from_slice(&1u32.to_le_bytes());
        b[24..32].copy_from_slice(&0x1000u64.to_le_bytes());
        b[32..40].copy_from_slice(&64u64.to_le_bytes());
        b[54..56].copy_from_slice(&56u16.to_le_bytes());
        let phnum: u16 = if v.is_some() { 2 } else { 1 };
        b[56..58].copy_from_slice(&phnum.to_le_bytes());
        let ph = |b: &mut Vec<u8>, i: usize, t: u32, off: u64, sz: u64, al: u64| {
            let o = 64 + i * 56;
            b[o..o + 4].copy_from_slice(&t.to_le_bytes());
            b[o + 8..o + 16].copy_from_slice(&off.to_le_bytes());
            b[o + 16..o + 24].copy_from_slice(&off.to_le_bytes());
            b[o + 32..o + 40].copy_from_slice(&sz.to_le_bytes());
            b[o + 40..o + 48].copy_from_slice(&sz.to_le_bytes());
            b[o + 48..o + 56].copy_from_slice(&al.to_le_bytes());
        };
        ph(&mut b, 0, 1, 0x1000, 0x100, 0x1000);
        if let Some(v) = v {
            ph(&mut b, 1, 4, 0x1040, 24, 4);
            let n = &mut b[0x1040..0x1058];
            n[0..4].copy_from_slice(&7u32.to_le_bytes());
            n[4..8].copy_from_slice(&4u32.to_le_bytes());
            n[8..12].copy_from_slice(&1u32.to_le_bytes());
            n[12..19].copy_from_slice(b"NewTOS\0");
            n[20..24].copy_from_slice(&v.to_le_bytes());
        }
        b
    }

    #[test]
    fn reads_and_rewrites_the_note_value_in_place() {
        let mut b = image(Some(7));
        assert_eq!(prgflags_locate(&b), Ok((7, 0x1054)));
        let before = b.clone();
        assert_eq!(prgflags_patch(&mut b, 0x1000_0002), Ok(7));
        assert_eq!(prgflags_locate(&b), Ok((0x1000_0002, 0x1054)));
        let changed: Vec<usize> = (0..b.len()).filter(|&i| b[i] != before[i]).collect();
        assert_eq!(changed, vec![0x1054, 0x1057], "only bytes of the value word change");
    }

    #[test]
    fn a_file_without_the_note_is_refused() {
        let mut b = image(None);
        assert!(prgflags_locate(&b).is_err());
        assert!(prgflags_patch(&mut b, 1).is_err());
    }

    #[test]
    fn values_and_names() {
        assert_eq!(parse_u32("0x7"), Ok(7));
        assert_eq!(parse_u32("16"), Ok(16));
        assert!(parse_u32("0x1_0000_0000").is_err());
        assert_eq!(prgflags_describe(7), "PF_FASTLOAD | PF_TTRAMLOAD | PF_TTRAMMEM");
        assert_eq!(prgflags_describe(0), "none");
        assert_eq!(prgflags_describe(0x3000_0002), "PF_TTRAMLOAD | TPA quantum 4 (x 128 KiB)");
    }
}

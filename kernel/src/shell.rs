//! NewTOS interactive console — a small EmuCON-style command interpreter
//! built on the GEMDOS console + FAT16 file system. It reads a line
//! (echoing through `Cconout`, so it renders on the framebuffer console),
//! parses a command and runs it against the file system.
//!
//! Console I/O uses the **value-based** GEMDOS calls (`Cnecin` for input,
//! `Cconout` for output): `Cconws` / `Cconrs` read their string with the
//! unprivileged `ldtrb` load and would fault on a kernel pointer. The
//! file-system calls take ordinary kernel buffers (normal accesses).

const LINE_MAX: usize = 128;

/// The console's first lines.
const BANNER: &[u8] = b"NewTOS console (EmuCON) - 'help' for commands, 'exit' to return.\r\n\r\n";

/// Run the console REPL until the user types `exit`/`quit`, then return.
/// This is the EmuCON-style console launched from the desktop ("Execute
/// EmuCON"); on return the caller redraws the desktop.
///
/// It runs as its own process, the EmuCON PD (EmuTOS
/// `aes_run_rom_program(coma_start)`, `aes/gemshlib.c:614-620`; the
/// stand-in `tos_api::bdos::process::standin::enter_emucon`): while it
/// executes that PD is `run`, so a program it starts with `run` gets it as
/// `p_parent` (`cli/cmdexec.c:52`); `exit` is its `Pterm0` and `run` is its
/// parent (the accdesk PD) again (TASK/PROC03 stage 1).
pub fn run_console() {
    tos_api::bdos::process::standin::enter_emucon();
    crate::fbcon::clear_and_home();
    puts(BANNER);
    repl();
    tos_api::bdos::process::standin::leave_emucon();
}

/// [`run_console`] in place of a desktop program that could not be loaded
/// (`desktop` its name, `rc` the negative GEMDOS code of the load; DESK43
/// D6-d, `crate::shell_loop`): the same console, with one line under the
/// banner that says why it is there. It goes through `Cconout` like the rest
/// of the console, so it is on the screen as well as on the serial line — a
/// machine without a serial cable still tells its user.
pub fn run_console_without_desktop(desktop: &[u8], rc: i32) {
    tos_api::bdos::process::standin::enter_emucon();
    crate::fbcon::clear_and_home();
    puts(BANNER);
    puts(b"No desktop: ");
    puts(desktop);
    puts(b" could not be loaded (GEMDOS error ");
    if rc < 0 {
        cconout(b'-');
    }
    put_u32(rc.unsigned_abs());
    puts(b").\r\n\r\n");
    repl();
    tos_api::bdos::process::standin::leave_emucon();
}

/// The console's read-execute loop, until `exit`/`quit`.
fn repl() {
    let mut line = [0u8; LINE_MAX];
    loop {
        prompt();
        let n = read_line(&mut line);
        if execute(&mut line, n) {
            return;
        }
    }
}

/// Run the console REPL as the boot shell (never returns; `exit` halts).
pub fn run() -> ! {
    run_console();
    crate::arch::aarch64::halt();
}

// ── line editing ──────────────────────────────────────────────────────

/// Read a line into `buf`, echoing as it goes. Returns the length
/// (excluding the terminator). Handles backspace.
fn read_line(buf: &mut [u8; LINE_MAX]) -> usize {
    let mut len = 0;
    loop {
        let c = (cnecin() & 0xFF) as u8;
        match c {
            b'\r' | b'\n' => {
                puts(b"\r\n");
                return len;
            }
            0x08 | 0x7F if len > 0 => {
                len -= 1;
                puts(b"\x08 \x08"); // erase the echoed char
            }
            0x20..=0x7E if len < LINE_MAX - 1 => {
                buf[len] = c;
                len += 1;
                cconout(c);
            }
            _ => {}
        }
    }
}

// ── command dispatch ──────────────────────────────────────────────────

/// Execute one command line. Returns `true` if the user asked to leave the
/// console (`exit`/`quit`), so `run_console` can return to the desktop.
fn execute(line: &mut [u8; LINE_MAX], n: usize) -> bool {
    // Split into the command word and the (trimmed) argument.
    let s = &line[..n];
    let cmd_end = s.iter().position(|&b| b == b' ').unwrap_or(n);
    let arg_start = (cmd_end..n).find(|&i| line[i] != b' ').unwrap_or(n);
    let cmd = &line[..cmd_end];

    // A bare "X:" switches drive.
    if cmd_end == 2 && line[1] == b':' && line[0].is_ascii_alphabetic() {
        let drv = line[0].to_ascii_uppercase() - b'A';
        dsetdrv(drv as u16);
        return false;
    }

    match cmd {
        b"" => {}
        b"exit" | b"quit" => return true,
        b"help" => cmd_help(),
        b"ver" => {
            // The boot screen's `Version` text, from the one version source
            // (REL07, `initinfo::VERSION_STRING`).
            puts(crate::initinfo::VERSION_STRING.as_bytes());
            puts(b" - clean-room TOS for AArch64\r\n");
        }
        b"cls" => crate::fbcon::clear_and_home(),
        b"pwd" => cmd_pwd(),
        b"cd" => cmd_cd(line, arg_start, n),
        b"dir" | b"ls" => cmd_dir(line, arg_start, n),
        b"type" | b"cat" => cmd_type(line, arg_start, n),
        b"run" => cmd_run(line, arg_start, n),
        b"bench" => crate::bench::run(),
        b"init" => cmd_init(),
        _ => {
            puts(b"Unknown command: ");
            puts(cmd);
            puts(b"\r\n");
        }
    }
    false
}

fn cmd_help() {
    puts(b"Commands:\r\n");
    puts(b"  help            this list\r\n");
    puts(b"  ver             version\r\n");
    puts(b"  cls             clear screen\r\n");
    puts(b"  pwd             current drive and path\r\n");
    puts(b"  cd <path>       change directory\r\n");
    puts(b"  dir [pattern]   list directory\r\n");
    puts(b"  type <file>     show a file\r\n");
    puts(b"  run <file>      load and run a program (.PRG)\r\n");
    puts(b"  bench           memory read/write/copy benchmark\r\n");
    puts(b"  init            create base dirs on C: (ACC/AUTO/GEMSYS)\r\n");
    puts(b"  X:              change drive (e.g. d:)\r\n");
}

/// Load and run a program from the file system via Pexec (PE_LOADGO).
/// Control returns here when the program calls Pterm.
fn cmd_run(line: &mut [u8; LINE_MAX], start: usize, n: usize) {
    if start >= n {
        puts(b"Usage: run <file>\r\n");
        return;
    }
    line[n] = 0; // NUL-terminate the path argument
    let rc = pexec(0, line[start..].as_ptr() as u64);
    // A negative result is a Pexec error (file not found, no slot, bad
    // format); otherwise the program ran and returned.
    if rc < 0 {
        puts(b"Cannot run program (error ");
        put_u32((-rc) as u32);
        puts(b").\r\n");
    } else {
        puts(b"[program exited]\r\n");
    }
}

/// Create the standard Atari base directories on the boot volume (C:).
fn cmd_init() {
    puts(b"Creating base directories on boot volume C:\r\n");
    mkdir(c"C:\\ACC".as_ptr() as u64, b"ACC");
    mkdir(c"C:\\AUTO".as_ptr() as u64, b"AUTO");
    mkdir(c"C:\\GEMSYS".as_ptr() as u64, b"GEMSYS");
    puts(b"Done.\r\n");
}

/// Dcreate one directory and report the outcome.
fn mkdir(path: u64, name: &[u8]) {
    puts(b"  ");
    puts(name);
    match dcreate(path) {
        0 => puts(b" - created\r\n"),
        -36 => puts(b" - already exists\r\n"), // EACCDN (name in use)
        _ => puts(b" - FAILED\r\n"),
    }
}

fn cmd_pwd() {
    let drv = (dgetdrv() & 0xFF) as u8;
    cconout(b'A' + drv);
    cconout(b':');
    cconout(b'\\');
    let mut buf = [0u8; 128];
    if dgetpath(buf.as_mut_ptr() as u64, 0) == 0 {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
        // Strip the leading backslash (we already printed the root "\").
        let path = if end > 0 && buf[0] == b'\\' { &buf[1..end] } else { &buf[..end] };
        puts(path);
    }
    puts(b"\r\n");
}

fn cmd_cd(line: &mut [u8; LINE_MAX], start: usize, n: usize) {
    if start >= n {
        cmd_pwd();
        return;
    }
    line[n] = 0; // NUL-terminate the argument in place
    let rc = dsetpath(line[start..].as_ptr() as u64);
    if rc < 0 {
        puts(b"No such directory.\r\n");
    }
}

fn cmd_dir(line: &mut [u8; LINE_MAX], start: usize, n: usize) {
    // Build the search pattern: the argument, or "*.*".
    let mut pat = [0u8; LINE_MAX];
    if start < n {
        let len = n - start;
        pat[..len].copy_from_slice(&line[start..n]);
        pat[len] = 0;
    } else {
        pat[..4].copy_from_slice(b"*.*\0");
    }

    let mut dta = [0u8; 44];
    fsetdta(dta.as_mut_ptr() as u64);
    let mut rc = fsfirst(pat.as_ptr() as u64, 0x10); // include sub-dirs
    if rc < 0 {
        puts(b"No matching files.\r\n");
        return;
    }
    let mut count = 0u32;
    while rc == 0 {
        let attr = dta[21];
        let size = u32::from_le_bytes([dta[26], dta[27], dta[28], dta[29]]);
        let end = dta[30..44].iter().position(|&b| b == 0).map_or(14, |p| 30 + p);
        let name = &dta[30..end];
        puts(name);
        // Pad the name column to 14, then attribute + size.
        for _ in name.len()..14 {
            cconout(b' ');
        }
        if attr & 0x10 != 0 {
            puts(b"<DIR>\r\n");
        } else {
            put_u32(size);
            puts(b"\r\n");
        }
        count += 1;
        rc = fsnext();
    }
    put_u32(count);
    puts(b" item(s).\r\n");
}

fn cmd_type(line: &mut [u8; LINE_MAX], start: usize, n: usize) {
    if start >= n {
        puts(b"Usage: type <file>\r\n");
        return;
    }
    line[n] = 0;
    let h = fopen(line[start..].as_ptr() as u64, 0);
    if h < 6 {
        puts(b"Cannot open file.\r\n");
        return;
    }
    let mut buf = [0u8; 512];
    loop {
        let got = fread(h as u16, buf.len() as i64, buf.as_mut_ptr() as u64);
        if got <= 0 {
            break;
        }
        for &b in &buf[..got as usize] {
            // Translate bare LF to CR/LF for the console.
            if b == b'\n' {
                cconout(b'\r');
            }
            cconout(b);
        }
    }
    fclose(h as u16);
    puts(b"\r\n");
}

// ── output helpers ────────────────────────────────────────────────────

fn prompt() {
    let drv = (dgetdrv() & 0xFF) as u8;
    cconout(b'A' + drv);
    cconout(b':');
    cconout(b'\\');
    let mut buf = [0u8; 128];
    if dgetpath(buf.as_mut_ptr() as u64, 0) == 0 {
        let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
        if end > 1 {
            puts(&buf[1..end]); // skip the leading backslash
        }
    }
    puts(b"> ");
}

fn puts(s: &[u8]) {
    for &b in s {
        cconout(b);
    }
}

fn put_u32(mut v: u32) {
    let mut tmp = [0u8; 10];
    let mut i = tmp.len();
    loop {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    puts(&tmp[i..]);
}

// ── GEMDOS svc wrappers (Trap 1) ──────────────────────────────────────

fn cnecin() -> i64 {
    let r: i64;
    // SAFETY: svc #1, GEMDOS Cnecin (0x08) — blocking, no echo.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x08u64, out("x0") r, options(nostack)); }
    r
}
fn cconout(c: u8) {
    // SAFETY: svc #1, GEMDOS Cconout (0x02); char in x0.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x02u64, inout("x0") c as u64 => _, options(nostack)); }
}
fn dgetdrv() -> i64 {
    let r: i64;
    // SAFETY: svc #1, Dgetdrv (0x19).
    unsafe { core::arch::asm!("svc #1", in("x8") 0x19u64, out("x0") r, options(nostack)); }
    r
}
fn dsetdrv(d: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Dsetdrv (0x0E).
    unsafe { core::arch::asm!("svc #1", in("x8") 0x0Eu64, inout("x0") d as u64 => r, options(nostack)); }
    r
}
fn dsetpath(p: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Dsetpath (0x3B); kernel path pointer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Bu64, inout("x0") p => r, options(nostack)); }
    r
}
fn dcreate(p: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Dcreate (0x39); kernel path pointer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x39u64, inout("x0") p => r, options(nostack)); }
    r
}
/// GEMDOS `Pexec` (0x4B): x0=mode, x1=path, x2=tail, x3=env. Returns the
/// child's exit code (PE_LOADGO) when it terminates.
fn pexec(mode: u16, path: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Pexec ABI; `path` is a kernel string (handled by
    // handle_pexec's EL1 path read).
    unsafe {
        core::arch::asm!(
            "svc #1",
            in("x8") 0x4Bu64,
            inout("x0") mode as u64 => r,
            in("x1") path,
            in("x2") 0u64,
            in("x3") 0u64,
            options(nostack),
        );
    }
    r
}
fn dgetpath(buf: u64, drv: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Dgetpath (0x47); kernel buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x47u64, inout("x0") buf => r, in("x1") drv as u64, options(nostack)); }
    r
}
fn fsetdta(buf: u64) {
    // SAFETY: svc #1, Fsetdta (0x1A); kernel DTA buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x1Au64, in("x0") buf, lateout("x0") _, options(nostack)); }
}
fn fsfirst(pat: u64, attr: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Fsfirst (0x4E); kernel pattern pointer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x4Eu64, inout("x0") pat => r, in("x1") attr as u64, options(nostack)); }
    r
}
fn fsnext() -> i64 {
    let r: i64;
    // SAFETY: svc #1, Fsnext (0x4F).
    unsafe { core::arch::asm!("svc #1", in("x8") 0x4Fu64, out("x0") r, options(nostack)); }
    r
}
fn fopen(name: u64, mode: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Fopen (0x3D); kernel path pointer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Du64, inout("x0") name => r, in("x1") mode as u64, options(nostack)); }
    r
}
fn fread(h: u16, len: i64, buf: u64) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Fread (0x3F); kernel buffer.
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Fu64, inout("x0") h as u64 => r, in("x1") len, in("x2") buf, options(nostack)); }
    r
}
fn fclose(h: u16) -> i64 {
    let r: i64;
    // SAFETY: svc #1, Fclose (0x3E).
    unsafe { core::arch::asm!("svc #1", in("x8") 0x3Eu64, inout("x0") h as u64 => r, options(nostack)); }
    r
}

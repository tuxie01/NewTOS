//! BIOS/XBIOS argument tracer (TOOL08) — the NewTOS half of the auditor's
//! input check ("inputs before outputs", `.claude/agents/auditor.md`).
//!
//! A debug instrument, compiled only with the kernel's cargo feature
//! `bios-arg-trace` (the module itself is `#[cfg]`-gated in `lib.rs`, its two
//! hook sites in the `svc` handler likewise), so a build without the feature
//! does not contain a byte of it.
//!
//! ## What it records
//!
//! One serial-console line per trap #13 (BIOS) and trap #14 (XBIOS) call that
//! **completed** — written after the dispatch, from the [`TrapArgs`] the
//! handler built before it, so the arguments are the registers as they
//! arrived at the dispatcher:
//!
//! ```text
//! [biostrace] 000042 U0  13:0x04 Rwabs       w:0x0000 p:0x0000000001a0c010 w:0x0001 w:0x0000 w:0x0002 l:0x00000000 -> 0x0000000000000000
//! ```
//!
//! | Field | Meaning |
//! |---|---|
//! | `000042` | running number of the record since boot (decimal) |
//! | `U0` / `K` | the caller: `U<slot>` an EL0 process (its process-table slot, `U?` none), `K` an `svc` from EL1 |
//! | `13:0x04` | trap and function number (`x8`, low 16 bits, as the dispatcher reads it) |
//! | `Rwabs` | the slot's name; `(empty)` for an `xbios_unimpl` slot of the EmuTOS table, `(beyond)` past it |
//! | `w:… l:… p:…` | the arguments, **at the width of the EmuTOS call form** (below); `-` when the form has none |
//! | `-> 0x…` | the full 64-bit `x0` the dispatcher returned |
//!
//! An argument is `x0`, `x1`, … in order, printed at the width its slot has in
//! the EmuTOS handler's prototype — the TOS call form, because that is the
//! width the m68k stack frame is laid in (`docs/TOS-REFERENCE-DUMPER.md` §6,
//! "The call table"):
//!
//! * `w:0x%04x` — a `WORD`/`UWORD` slot, the register's low 16 bits;
//! * `l:0x%08x` — a `LONG`/`ULONG` slot, the low 32 bits;
//! * `p:0x%016x` — a pointer slot, the whole register (a native pointer).
//!
//! A `w:`/`l:` value carries `=0x%016x`, the whole register, when the register
//! is **not** the sign extension of the printed value: whatever the dispatcher
//! drops above the call-form width stays visible. The dumper's backend passes
//! every WORD sign-narrowed and every LONG sign-extended
//! (`tosdump/abi/newtos_svc.c`), so in a dumper run an `=` is itself a finding.
//! The function number gets the same annotation when `x8` is not the sign
//! extension of its low 16 bits.
//!
//! Registers beyond the call form are not printed: no handler reads them.
//!
//! ## Where the call forms come from
//!
//! [`form`] holds the parameter list of every slot of the two EmuTOS dispatch
//! tables at the pin (`references/emutos` @ 1a7484ab, `docs/analysis/ORACLE.md`
//! §1), read off the debug wrappers `bios_N` / `xbios_N` that `VEC()` puts in
//! the tables under `DBGBIOS` / `DBG_XBIOS` (`bios/bios.c:1244-1265`,
//! `bios/xbios.c:1266-1515`). A wrapper sits in the same table slot as the
//! handler it forwards to, so its prototype is the slot's call form. Slots
//! that are `xbios_unimpl` read no argument (`bios/xbios.c:1251-1257`), nor
//! does anything past the table (`bios/vectors.S:1324-1326`).
//!
//! ## Where a record comes from, and the one that comes from elsewhere
//!
//! [`record`] is called from the `svc` handler after the dispatch and after
//! the KEYS02 restart check, so a `Bconin(2)` rewound to its `svc` is recorded
//! once, when it completes. `Supexec` (XBIOS 0x26) from an EL0 program never
//! reaches the dispatcher: the handler turns it into an upcall
//! (`handle_supexec`, MEM12 S6) whose value comes back through `Psigreturn`.
//! [`record_supexec_upcall`] records it **before** the upcall, with
//! `-> upcall` instead of a result. (Should `handle_supexec` decline — only
//! when the running process has no slot, which an EL0 caller always has — the
//! call reaches the dispatcher and is recorded a second time.)
//!
//! ## Why the serial console, not a file
//!
//! The input check of a disk table (`tosdump/cases/bios-r2.tsv`) runs on a
//! card whose C: is the test image, compared byte for byte after the run
//! (`measurements/measurement-2026-10-28-tool07/method/compare-testpart.sh`).
//! A trace file on C: (`svc-seq-trace` writes `C:\SVCSEQ.LOG`) would change
//! that image, and a flush before the dumper's `Rwabs` cases would change what
//! they read. On the console the instrument touches no drive, needs no flush
//! point and is not bounded by a buffer; a QEMU run captures it with
//! `-append serial=on -serial file:…` (a `raspi4b` kernel is UART-silent
//! without `serial=on`). The dumper's own console lines (`running N cases`)
//! and the kernel's `[launch]` lines land in the same file.
//!
//! The checker is `tools/biostrace-check.py`; the procedure is in
//! `docs/TOS-REFERENCE-DUMPER.md` §6, "Input check on NewTOS".

use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};

use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::uaccess::Space;

/// The width of one argument slot of a call form.
#[derive(Clone, Copy)]
enum Slot {
    /// `WORD` / `UWORD`: 16 bits.
    W,
    /// `LONG` / `ULONG`: 32 bits.
    L,
    /// A pointer (data or function): the native register.
    P,
}

use Slot::{L, P, W};

/// Name and parameter list of one dispatch-table slot.
type Form = (&'static str, &'static [Slot]);

/// What an `xbios_unimpl` slot reads: nothing (`bios/xbios.c:1251-1257`).
const EMPTY: Form = ("(empty)", &[]);
/// What a number past the table reads: nothing — the trampoline returns the
/// number itself (`bios/vectors.S:1324-1326`).
const BEYOND: Form = ("(beyond)", &[]);

/// The call form of `trap` (13 or 14), function `func`. Every line cites the
/// EmuTOS wrapper prototype the form is read from (module doc).
fn form(trap: u16, func: u16) -> Form {
    if trap == 13 {
        // bios/bios.c, table bios_vecs :1252-1265
        return match func {
            0x00 => ("Getmpb", &[P]),                    // :895  bios_0(MPB *)
            0x01 => ("Bconstat", &[W]),                  // :937  bios_1(WORD)
            0x02 => ("Bconin", &[W]),                    // :981  bios_2(WORD)
            0x03 => ("Bconout", &[W, W]),                // :1014 bios_3(WORD, WORD)
            0x04 => ("Rwabs", &[W, P, W, W, W, L]),      // :1051 bios_4(WORD, UBYTE *, WORD, WORD, WORD, LONG)
            0x05 => ("Setexc", &[W, L]),                 // :1083 bios_5(WORD, LONG)
            0x06 => ("Tickcal", &[]),                    // :1102 bios_6(void)
            0x07 => ("Getbpb", &[W]),                    // :1126 bios_7(WORD)
            0x08 => ("Bcostat", &[W]),                   // :1166 bios_8(WORD)
            0x09 => ("Mediach", &[W]),                   // :1190 bios_9(WORD)
            0x0A => ("Drvmap", &[]),                     // :1212 bios_a(void)
            0x0B => ("Kbshift", &[W]),                   // :1235 bios_b(WORD)
            _ => BEYOND,
        };
    }
    // bios/xbios.c, table xbios_vecs :1285-1515 (LAST_ENTRY 0x8d, :1273-1283)
    match func {
        0x00 => ("Initmous", &[W, P, P]),                // :49   xbios_0(WORD, struct param *, PFVOID)
        0x02 => ("Physbase", &[]),                       // :77
        0x03 => ("Logbase", &[]),                        // :93
        0x04 => ("Getrez", &[]),                         // :109
        0x05 => ("Setscreen", &[P, P, W, W]),            // :143  xbios_5(UBYTE *, const UBYTE *, WORD, WORD)
        0x06 => ("Setpalette", &[P]),                    // :162  xbios_6(const UWORD *)
        0x07 => ("Setcolor", &[W, W]),                   // :182
        0x08 => ("Floprd", &[P, L, W, W, W, W, W]),      // :210  xbios_8(UBYTE *, LONG, WORD x5)
        0x09 => ("Flopwr", &[P, L, W, W, W, W, W]),      // :233
        0x0A => ("Flopfmt", &[P, P, W, W, W, W, W, W, L]), // :276 xbios_a(UBYTE *, WORD *, WORD x6, LONG)
        0x0C => ("Midiws", &[W, P]),                     // :303
        0x0D => ("Mfpint", &[W, L]),                     // :322  xbios_d(WORD, LONG)
        0x0E => ("Iorec", &[W]),                         // :351
        0x0F => ("Rsconf", &[W, W, W, W, W, W]),         // :382
        0x10 => ("Keytbl", &[P, P, P]),                  // :403
        0x11 => ("Random", &[]),                         // :433
        0x12 => ("Protobt", &[P, L, W, W]),              // :462
        0x13 => ("Flopver", &[P, L, W, W, W, W, W]),     // :476
        0x14 => ("Scrdmp", &[]),                         // :497
        0x15 => ("Cursconf", &[W, W]),                   // :511
        0x16 => ("Settime", &[L]),                       // :528  xbios_16(ULONG)
        0x17 => ("Gettime", &[]),                        // :545
        0x18 => ("Bioskeys", &[]),                       // :561
        0x19 => ("Ikbdws", &[W, P]),                     // :575
        0x1A => ("Jdisint", &[W]),                       // :590
        0x1B => ("Jenabint", &[W]),                      // :604
        0x1C => ("Giaccess", &[W, W]),                   // :619
        0x1D => ("Offgibit", &[W]),                      // :633
        0x1E => ("Ongibit", &[W]),                       // :647
        0x1F => ("Xbtimer", &[W, W, W, L]),              // :662  xbios_1f(WORD x3, LONG)
        0x20 => ("Dosound", &[P]),                       // :680
        0x21 => ("Setprt", &[W]),                        // :700
        0x22 => ("Kbdvbase", &[]),                       // :722
        0x23 => ("Kbrate", &[W, W]),                     // :736
        0x25 => ("Vsync", &[]),                          // :757
        0x26 => ("Supexec", &[P]),                       // :787  xbios_26(PFLONG)
        0x29 => ("Floprate", &[W, W]),                   // :825
        0x2A => ("DMAread", &[L, W, P, W]),              // :837
        0x2B => ("DMAwrite", &[L, W, P, W]),             // :849
        0x2C => ("Bconmap", &[W]),                       // :861
        0x2E => ("NVMaccess", &[W, W, W, P]),            // :875
        0x40 => ("Blitmode", &[W]),                      // :906
        0x50 => ("EsetShift", &[W]),                     // :917
        0x51 => ("EgetShift", &[]),                      // :922
        0x52 => ("EsetBank", &[W]),                      // :927
        0x53 => ("EsetColor", &[W, W]),                  // :932
        0x54 => ("EsetPalette", &[W, W, P]),             // :937
        0x55 => ("EgetPalette", &[W, W, P]),             // :942
        0x56 => ("EsetGray", &[W]),                      // :947
        0x57 => ("EsetSmear", &[W]),                     // :952
        0x58 => ("VsetMode", &[W]),                      // :963
        0x59 => ("VgetMonitor", &[]),                    // :968
        0x5A => ("VsetSync", &[W]),                      // :973
        0x5B => ("VgetSize", &[W]),                      // :978
        0x5D => ("VsetRGB", &[W, W, P]),                 // :983
        0x5E => ("VgetRGB", &[W, W, P]),                 // :988
        0x5F => ("Vfixmode", &[W]),                      // :993
        0x60 => ("Dsp_DoBlock", &[P, L, P, L]),          // :1005
        0x61 => ("Dsp_BlkHandShake", &[P, L, P, L]),     // :1010
        0x62 => ("Dsp_BlkUnpacked", &[P, L, P, L]),      // :1015
        0x63 => ("Dsp_InStream", &[P, L, L, P]),         // :1020
        0x64 => ("Dsp_OutStream", &[P, L, L, P]),        // :1025
        0x65 => ("Dsp_IOStream", &[P, P, L, L, L, P]),   // :1030
        0x66 => ("Dsp_RemoveInterrupts", &[W]),          // :1035
        0x67 => ("Dsp_GetWordSize", &[]),                // :1040
        0x68 => ("Dsp_Lock", &[]),                       // :1045
        0x69 => ("Dsp_Unlock", &[]),                     // :1050
        0x6A => ("Dsp_Available", &[P, P]),              // :1055
        0x6B => ("Dsp_Reserve", &[L, L]),                // :1060
        0x6C => ("Dsp_LoadProg", &[P, W, P]),            // :1065
        0x6D => ("Dsp_ExecProg", &[P, L, W]),            // :1070
        0x6E => ("Dsp_ExecBoot", &[P, L, W]),            // :1075
        0x6F => ("Dsp_LodToBinary", &[P, P]),            // :1080
        0x70 => ("Dsp_TriggerHC", &[W]),                 // :1085
        0x71 => ("Dsp_RequestUniqueAbility", &[]),       // :1090
        0x72 => ("Dsp_GetProgAbility", &[]),             // :1095
        0x73 => ("Dsp_FlushSubroutines", &[]),           // :1100
        0x74 => ("Dsp_LoadSubroutine", &[P, L, W]),      // :1105
        0x75 => ("Dsp_InqSubrAbility", &[W]),            // :1110
        0x76 => ("Dsp_RunSubroutine", &[W]),             // :1115
        0x77 => ("Dsp_Hf0", &[W]),                       // :1120
        0x78 => ("Dsp_Hf1", &[W]),                       // :1125
        0x79 => ("Dsp_Hf2", &[]),                        // :1130
        0x7A => ("Dsp_Hf3", &[]),                        // :1135
        0x7B => ("Dsp_BlkWords", &[P, L, P, L]),         // :1140
        0x7C => ("Dsp_BlkBytes", &[P, L, P, L]),         // :1145
        0x7D => ("Dsp_HStat", &[]),                      // :1150
        0x7E => ("Dsp_SetVectors", &[P, P]),             // :1155 two function pointers
        0x7F => ("Dsp_MultBlocks", &[L, L, P, P]),       // :1160
        0x80 => ("Locksnd", &[]),                        // :1171
        0x81 => ("Unlocksnd", &[]),                      // :1176
        0x82 => ("Soundcmd", &[W, W]),                   // :1181
        0x83 => ("Setbuffer", &[W, L, L]),               // :1186 xbios_83(UWORD, ULONG, ULONG)
        0x84 => ("Setsndmode", &[W]),                    // :1191
        0x85 => ("Settracks", &[W, W]),                  // :1196
        0x86 => ("Setmontracks", &[W]),                  // :1201
        0x87 => ("Setinterrupt", &[W, W]),               // :1206
        0x88 => ("Buffoper", &[W]),                      // :1211
        0x89 => ("Dsptristate", &[W, W]),                // :1216
        0x8A => ("Gpio", &[W, W]),                       // :1221
        0x8B => ("Devconnect", &[W, W, W, W, W]),        // :1226
        0x8C => ("Sndstatus", &[W]),                     // :1231
        0x8D => ("Buffptr", &[L]),                       // :1236 xbios_8d(LONG)
        // The xbios_unimpl slots of the table: 0x01, 0x0B, 0x24, 0x27, 0x28,
        // 0x2D, 0x2F-0x3F, 0x41-0x4F, 0x5C (:1287, :1297, :1339-1343, :1348,
        // :1354-1370, :1374-1388, :1416).
        0x01..=0x8D => EMPTY,
        _ => BEYOND,
    }
}

/// Running number of the records since boot.
static SEQ: AtomicU32 = AtomicU32::new(0);

/// A fixed line buffer on the stack: the line goes to the console in one
/// write, so nothing printed in between can split it.
struct LineBuf {
    buf: [u8; 512],
    len: usize,
}

impl Write for LineBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let room = self.buf.len() - self.len;
        let n = s.len().min(room);
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        if n < s.len() { Err(core::fmt::Error) } else { Ok(()) }
    }
}

/// Called by the `svc` handler for every completed trap (module doc); records
/// trap #13 and #14 and ignores the rest.
pub fn record(class: TrapClass, args: &TrapArgs, result: i64) {
    let trap = match class {
        TrapClass::Bios => 13,
        TrapClass::Xbios => 14,
        _ => return,
    };
    emit(trap, args.func, &args.regs, args.space, Some(result));
}

/// Called by the `svc` handler for an EL0 `Supexec`, before it becomes an
/// upcall (module doc). `gpr` is the trap frame's `x0..x29`.
pub fn record_supexec_upcall(gpr: &[u64; 30]) {
    let mut regs = [0u64; 8];
    regs.copy_from_slice(&gpr[..8]);
    emit(14, gpr[8], &regs, Space::User, None);
}

/// Format and print one record.
fn emit(trap: u16, x8: u64, regs: &[u64; 8], space: Space, result: Option<i64>) {
    let func = x8 as u16;
    let (name, slots) = form(trap, func);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut l = LineBuf { buf: [0; 512], len: 0 };
    let _ = write!(l, "[biostrace] {:06} ", seq);
    match space {
        Space::Kernel => {
            let _ = write!(l, "K   ");
        }
        Space::User => {
            // `running_slot()` is `MAX_PROCESSES` when `RUN` names no slot.
            let slot = tos_api::bdos::process::sched::running_slot();
            if slot >= tos_api::bdos::process::MAX_PROCESSES {
                let _ = write!(l, "U?  ");
            } else {
                let _ = write!(l, "U{:<2} ", slot);
            }
        }
    }
    let _ = write!(l, "{}:0x{:02x}", trap, func);
    if x8 != func as i16 as i64 as u64 {
        let _ = write!(l, "=0x{:016x}", x8);
    }
    let _ = write!(l, " {:<11}", name);
    if slots.is_empty() {
        let _ = write!(l, " -");
    }
    for (slot, &raw) in slots.iter().zip(regs.iter()) {
        // The value at the call-form width, and the register it would be
        // if the caller had sign-extended that value.
        let (value, sext) = match slot {
            W => (raw & 0xFFFF, raw as i16 as i64 as u64),
            L => (raw & 0xFFFF_FFFF, raw as i32 as i64 as u64),
            P => (raw, raw),
        };
        let _ = match slot {
            W => write!(l, " w:0x{:04x}", value),
            L => write!(l, " l:0x{:08x}", value),
            P => write!(l, " p:0x{:016x}", value),
        };
        if raw != sext {
            let _ = write!(l, "=0x{:016x}", raw);
        }
    }
    let _ = match result {
        Some(r) => writeln!(l, " -> 0x{:016x}", r as u64),
        None => writeln!(l, " -> upcall"),
    };
    // A line that did not fit still ends in a newline.
    if l.len == l.buf.len() {
        l.buf[l.len - 1] = b'\n';
    }
    // The buffer holds only what `write_str` copied from `&str`s, cut at a
    // byte count; every byte written here is ASCII, so it is valid UTF-8.
    let text = core::str::from_utf8(&l.buf[..l.len]).unwrap_or("[biostrace] (unprintable)\n");
    crate::kprint!("{}", text);
}

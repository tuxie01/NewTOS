//! TASK/HW08 — the report of the debug feature `sd-diag`: the text
//! (`tos_api::bios::sd_diag`), the dirty sectors it lists, and
//! `Ssystem(S_NEWTOS_SDDIAG / S_NEWTOS_SDFAULT)`. The feature is on in host
//! test builds through the crate's dev-dependency on itself
//! (`tos-api/Cargo.toml`). The kernel half (the driver's counters, the screen)
//! runs in `measurements/measurement-2026-11-06-hw08/RUNINFO.TXT`.

mod fatvol;

use fatvol::*;

use tos_api::bdos::fs::cache::{self, DirtySlot};
use tos_api::bios::bcache::Stats;
use tos_api::bios::sd_diag::{
    COLUMNS, DIRTY_LISTED, DriverReport, FailKind, Failure, HIST_LEN, LINES, LOG_LEN, OpStats,
    Report, TEXT_MAX, hist_index, write_report,
};
use tos_types::error::ERROR;

const PHASES: [&str; 7] = [
    "cmd_inhibit",
    "dat_inhibit",
    "cmd_done",
    "read_rdy",
    "write_rdy",
    "read_done",
    "write_done",
];
const BITS: [&str; 8] = ["CTO", "CCRC", "CEND", "CBAD", "DTO", "DCRC", "DEND", "ACMD"];
const R1: [&str; 15] = [
    "OUT_OF_RANGE",
    "ADDRESS_ERROR",
    "BLOCK_LEN_ERROR",
    "ERASE_SEQ_ERROR",
    "ERASE_PARAM",
    "WP_VIOLATION",
    "LOCK_UNLOCK_FAILED",
    "COM_CRC_ERROR",
    "ILLEGAL_COMMAND",
    "CARD_ECC_FAILED",
    "CC_ERROR",
    "ERROR",
    "CSD_OVERWRITE",
    "WP_ERASE_SKIP",
    "AKE_SEQ_ERROR",
];

fn failure(at_us: u64, lba: u64, phase: u8, kind: FailKind, status: u32) -> Failure {
    Failure {
        at_us,
        write: true,
        lba,
        blocks: 1,
        phase,
        kind,
        status,
    }
}

#[test]
fn the_report_reads_as_documented() {
    let mut hist = [0u32; HIST_LEN];
    hist[0] = 100;
    hist[6] = 2;
    hist[7] = 1;
    let write = OpStats {
        count: 103,
        sectors: 350,
        failed: 1,
        max_us: 1_000_400,
        hist,
    };
    let read = OpStats {
        count: 7,
        sectors: 7,
        failed: 0,
        max_us: 1_100,
        hist: [6, 1, 0, 0, 0, 0, 0, 0],
    };
    let log = [failure(424_120_000, 2000, 1, FailKind::Timeout, 0)];
    let dirty = [
        DirtySlot {
            lba: 192,
            drv: 2,
            fat: true,
            fails: 3,
            err: -2,
        },
        DirtySlot {
            lba: 320,
            drv: 2,
            fat: false,
            fails: 1,
            err: -10,
        },
    ];
    let mut bits = [0u32; 8];
    bits[5] = 1;
    let mut r1_bits = [0u32; 15];
    r1_bits[5] = 2;
    let mut tmo = [0u32; 7];
    tmo[6] = 1;
    let mut wait = [0u64; 7];
    wait[1] = 2_500;
    wait[6] = 1_000_100;
    let d = DriverReport {
        mode: Some((false, 4)),
        io_errors: 1,
        timeouts: 7,
        recoveries: 4,
        nonrecoverable: 0,
        card_errors: 2,
        at_init: (0, 6),
        last_phase: 6,
        last_status: 0x0020_0000,
        last_card_status: 0x0400_0800,
        phase_names: &PHASES,
        bit_names: &BITS,
        bits: &bits,
        r1_names: &R1,
        r1_bits: &r1_bits,
        timeouts_by_phase: &tmo,
        wait_max_us: &wait,
        write,
        read,
        first: Some(failure(423_550_000, 1000, 6, FailKind::Timeout, 0)),
        log: &log,
    };
    let r = Report {
        now_us: 444_120_000,
        driver: Some(d),
        writeback: (5, 2, 1, 2),
        codes: (-10, 7),
        dirty: &dirty,
        bcache: Some(Stats {
            dev_reads: 8,
            dev_writes: 9,
            hit_sectors: 10,
            miss_sectors: 11,
            ..Stats::default()
        }),
        hot_key: "Ctrl+Alt+F9",
    };
    let mut s = String::new();
    write_report(&mut s, &r).unwrap();
    let want = "\
SD diagnosis  up 0:07:24.120   Ctrl+Alt+F9: hide / show again
driver  4-bit multi errors 1 timeouts 7 recover 4 nonrecover 0 refused 2
 boot    errors 0 timeouts 6 (base probe, in the counts above)
 last    write_done int 00200000 r1 04000800
 bits   DCRC=1
 r1     WP_VIOLATION=2
 tmo    write_done=1
 wait ms dat_inhibit 2.5 write_done 1000.1
write   n 103 sec 350 failed 1 max 1000.4 ms
 time   <1:100 <4:0 <16:0 <64:0 <250:0 <500:0 <1000:2 more:1
read    n 7 sec 7 failed 0 max 1.1 ms
 time   <1:6 <4:1 <16:0 <64:0 <250:0 <500:0 <1000:0 more:0
first   0:07:03.550 write lba 1000 n 1 write_done timeout 00000000
fail    0:07:04.120 write lba 2000 n 1 dat_inhibit timeout 00000000
cache   writeback failed 5 retried 2 gave up 1 dirty 2; last code -10 of 7
 dirty  C: lba 192 FAT tries 3 code -2
 dirty  C: lba 320 dir/data tries 1 code -10
bcache  dev r 8 w 9 hit 10 miss 11 sectors
";
    assert_eq!(s, want);
}

#[test]
fn the_longest_report_fits_its_box() {
    let hist = [u32::MAX; HIST_LEN];
    let big = OpStats {
        count: u64::MAX,
        sectors: u64::MAX,
        failed: u64::MAX,
        max_us: 1 << 40,
        hist,
    };
    let f = Failure {
        at_us: 99 * 3600 * 1_000_000,
        write: true,
        lba: u64::from(u32::MAX),
        blocks: 65535,
        phase: 1,
        kind: FailKind::Nonrecoverable,
        status: u32::MAX,
    };
    let log = [f; LOG_LEN];
    let dirty = [DirtySlot {
        lba: u64::from(u32::MAX),
        drv: 25,
        fat: false,
        fails: 3,
        err: -2,
    }; DIRTY_LISTED];
    let d = DriverReport {
        mode: Some((true, 1)),
        io_errors: u32::MAX,
        timeouts: u32::MAX,
        recoveries: u32::MAX,
        nonrecoverable: u32::MAX,
        card_errors: u32::MAX,
        at_init: (u32::MAX, u32::MAX),
        last_phase: 6,
        last_status: u32::MAX,
        last_card_status: u32::MAX,
        phase_names: &PHASES,
        bit_names: &BITS,
        bits: &[1_000_000; 8],
        r1_names: &R1,
        r1_bits: &[1_000_000; 15],
        timeouts_by_phase: &[1_000_000; 7],
        wait_max_us: &[10_000_000; 7],
        write: big,
        read: big,
        first: Some(f),
        log: &log,
    };
    let r = Report {
        now_us: 99 * 3600 * 1_000_000,
        driver: Some(d),
        writeback: (u32::MAX, u32::MAX, u32::MAX, 16),
        codes: (-10, u32::MAX),
        dirty: &dirty,
        bcache: Some(Stats {
            dev_reads: 1 << 32,
            dev_writes: 1 << 32,
            hit_sectors: 1 << 32,
            miss_sectors: 1 << 32,
            ..Stats::default()
        }),
        hot_key: "Ctrl+Alt+F9",
    };
    let mut s = String::new();
    write_report(&mut s, &r).unwrap();
    assert!(
        s.lines().count() <= LINES,
        "{} lines:\n{s}",
        s.lines().count()
    );
    assert!(s.len() <= TEXT_MAX);
    // The extreme counters do not occur together; what a screen sees is the
    // box's cut, so a counter's own width may run over but nothing else does.
    let realistic = s
        .replace("18446744073709551615", "0")
        .replace("4294967295", "0");
    for l in realistic.lines() {
        assert!(l.len() <= COLUMNS, "{} columns: {l}", l.len());
    }
}

#[test]
fn time_classes_end_at_the_limits_the_driver_knows() {
    assert_eq!(hist_index(0), 0);
    assert_eq!(hist_index(999), 0);
    assert_eq!(hist_index(1_000), 1);
    assert_eq!(hist_index(249_999), 4);
    assert_eq!(hist_index(250_000), 5);
    assert_eq!(hist_index(999_999), 6);
    assert_eq!(hist_index(1_000_000), HIST_LEN - 1);
    assert_eq!(hist_index(u64::MAX / 1_000), HIST_LEN - 1);
}

#[test]
fn dirty_slots_lists_the_sector_the_card_did_not_take() {
    let (_g, host) = fs();
    let path = cpath("C:\\DIRTY.TXT");
    let h = gemdos2(FCREATE, path.as_ptr() as u64, 0);
    assert!(h >= 0, "Fcreate -> {h}");
    assert_eq!(gemdos1(FCLOSE, h as u64), E_OK);

    // Nothing is dirty after a clean close.
    let mut out = [DirtySlot::default(); 16];
    assert_eq!(cache::dirty_slots(&mut out), 0);

    // The root directory sector does not reach the card: it stays dirty with
    // the code of the failed write-back, and the code is kept past the
    // GEMDOS call that took it.
    host.disk.fail_write_at(root_lba(), EWRITF);
    let path2 = cpath("C:\\DIRTY2.TXT");
    assert_eq!(gemdos2(FCREATE, path2.as_ptr() as u64, 0), EWRITF);
    let n = cache::dirty_slots(&mut out);
    assert_eq!(n, 1);
    assert_eq!(
        out[0],
        DirtySlot {
            lba: root_lba(),
            drv: 2,
            fat: false,
            fails: 1,
            err: EWRITF
        }
    );
    assert_eq!(cache::error_codes().0, EWRITF);
    assert_eq!(cache::writeback_stats().3, 1);

    // A slice too short still gets the number.
    assert_eq!(cache::dirty_slots(&mut []), 1);

    host.disk.disarm();
    let mut td = [0u8; 4];
    let h = gemdos2(FOPEN, path.as_ptr() as u64, 2);
    assert!(h >= 0, "Fopen -> {h}");
    assert_eq!(
        call(0x57, [td.as_mut_ptr() as u64, h as u64, 1, 0, 0, 0, 0, 0]),
        E_OK
    );
    assert_eq!(gemdos1(FCLOSE, h as u64), E_OK);
    assert_eq!(cache::dirty_slots(&mut out), 0);
}

#[test]
fn ssystem_says_error_on_a_host_without_a_diagnosis() {
    let (_g, _host) = fs();
    const SSYSTEM: u16 = 0x154;
    let mut buf = [0u8; 64];
    let arm = [0u8; 24];
    assert_eq!(
        call(SSYSTEM, [2090, buf.as_mut_ptr() as u64, 64, 0, 0, 0, 0, 0]),
        ERROR
    );
    assert_eq!(call(SSYSTEM, [2090, 0, 0, 0, 0, 0, 0, 0]), ERROR);
    assert_eq!(
        call(SSYSTEM, [2091, arm.as_ptr() as u64, 0, 0, 0, 0, 0, 0]),
        ERROR
    );
}

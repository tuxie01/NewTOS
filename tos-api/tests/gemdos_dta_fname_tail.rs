//! `Fsfirst`/`Fsnext` write into `d_fname` only the name and its NUL (FS23).
//!
//! EmuTOS `makbuf` packs the found name with `packit`
//! (`references/emutos/bdos/fsdir.c:1851-1862`, `:1297-1324`): it copies
//! `NAME`, optionally `.EXT`, then stores one `'\0'` and returns. The bytes of
//! the 14-byte `d_fname` after that NUL are never written, so they keep what
//! the DTA held before -- the caller's own bytes before the first search, the
//! tail of a longer earlier name after the next one. The oracle shows exactly
//! that (`measurements/measurement-2026-10-29-tool20/RUNINFO.TXT`, finding F1;
//! tables `tosdump/cases/gemdos-mc-fs22.tsv`, `gemdos-mc-paths.tsv`).
//!
//! Each test presets the whole DTA with a poison byte and checks, byte by byte,
//! that everything after the NUL is still the poison (or the earlier name).

mod fatvol;

use fatvol::*;

const POISON: u8 = 0xA5;
/// `d_fname` offset in the 44-byte DTA (`include/bdosdefs.h:48-55`).
const FNAME: usize = 30;
const FA_SUBDIR: u8 = 0x10;

fn set_dta(dta: &mut [u8; 44]) {
    assert_eq!(gemdos1(FSETDTA, dta.as_mut_ptr() as u64), E_OK);
}

/// `d_fname` must be `name`, its NUL at `FNAME + name.len()`, and the bytes
/// after it equal to `tail` (the bytes the DTA held there before the call).
fn assert_fname(dta: &[u8; 44], name: &str, tail: &[u8]) {
    let n = name.len();
    assert_eq!(&dta[FNAME..FNAME + n], name.as_bytes(), "d_fname name bytes");
    assert_eq!(dta[FNAME + n], 0, "d_fname NUL after {name:?}");
    assert_eq!(
        &dta[FNAME + n + 1..44],
        tail,
        "d_fname bytes after the NUL of {name:?} must keep their earlier content"
    );
}

#[test]
fn fsfirst_leaves_the_bytes_after_the_nul_as_they_were() {
    let (_g, _h) = fs();
    let mut dta = [POISON; 44];
    set_dta(&mut dta);

    let p = cpath("C:\\HELLO.TXT");
    assert_eq!(gemdos2(FSFIRST, p.as_ptr() as u64, 0), E_OK);

    // The public fields in front of d_fname are still written in full.
    assert_eq!(dta[21], 0x20, "d_attrib of HELLO.TXT");
    assert_eq!(u32::from_le_bytes([dta[26], dta[27], dta[28], dta[29]]), 5, "d_length");
    // "HELLO.TXT\0" = 10 bytes; d_fname[10..14] untouched.
    assert_fname(&dta, "HELLO.TXT", &[POISON; 4]);
}

#[test]
fn a_shorter_name_leaves_the_tail_of_the_longer_one_behind() {
    let (_g, _h) = fs();
    let mut dta = [POISON; 44];
    set_dta(&mut dta);

    let file = cpath("C:\\HELLO.TXT");
    assert_eq!(gemdos2(FSFIRST, file.as_ptr() as u64, 0), E_OK);
    assert_fname(&dta, "HELLO.TXT", &[POISON; 4]);

    // Same DTA, a shorter name: "SUB\0" overwrites d_fname[0..4]; "O.TXT\0"
    // from HELLO.TXT and the poison stay behind it, as on EmuTOS.
    let dir = cpath("C:\\SUB");
    assert_eq!(gemdos2(FSFIRST, dir.as_ptr() as u64, FA_SUBDIR as u64), E_OK);
    assert_eq!(dta[21], FA_SUBDIR, "d_attrib of SUB");
    let mut tail = b"O.TXT\0".to_vec();
    tail.extend_from_slice(&[POISON; 4]);
    assert_fname(&dta, "SUB", &tail);
}

#[test]
fn fsnext_leaves_the_bytes_after_the_nul_as_they_were() {
    let (_g, _h) = fs();
    let mut dta = [POISON; 44];
    set_dta(&mut dta);

    // C:\SUB holds ".", "..", "INNER.TXT" in this order.
    let star = cpath("C:\\SUB\\*.*");
    assert_eq!(gemdos2(FSFIRST, star.as_ptr() as u64, FA_ANY), E_OK);
    assert_fname(&dta, ".", &[POISON; 12]);

    assert_eq!(gemdos1(FSNEXT, 0), E_OK);
    assert_fname(&dta, "..", &[POISON; 11]);

    assert_eq!(gemdos1(FSNEXT, 0), E_OK);
    assert_fname(&dta, "INNER.TXT", &[POISON; 4]);

    // End of the directory: ENMFIL, and the last match stays as it was.
    assert_eq!(gemdos1(FSNEXT, 0), ENMFIL);
    assert_fname(&dta, "INNER.TXT", &[POISON; 4]);
}

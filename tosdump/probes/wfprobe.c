/*
 * wfprobe.c -- what a failed sector write does to GEMDOS (TASK/HW08).  A
 * permanent probe of the tosdump tree: GEMDOS calls, a text log, same C source
 * on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), as gcprobe.c.
 *
 * The fault:
 *   NewTOS  Ssystem(S_NEWTOS_SDFAULT) of the debug feature `sd-fault`
 *           (kernel/src/drivers/bcm2711/emmc_fault.rs): inside the SD driver,
 *           so recovery and everything above it run for real.
 *   EmuTOS  a hook on hdv_rw ($476): a write of C: answers the case's code
 *           instead of reaching blkdev_rwabs.  That skips etv_critic, so it
 *           models the critical-error answer "cancel" (bios/blkdev.c:520-522);
 *           BDOS sees exactly what Rwabs returns after that.  hdv_rw is called
 *           for every BDOS access (bios/bios.c:1047).
 *
 * One case per boot.  Chosen at build time (-DWFCASE="a1") it runs on C:, a
 * fresh clone of the oracle's test image (tools/mktestdisk.sh: FAT16, LBA 63,
 * 2-KiB clusters, FATs at 1 and 129, root at 257, data at 289), log on D:.
 * Built without WFCASE (the real Pi, TASK/HW08 Pi run) it asks on the console
 * for the case and for the drive to run on (a test partition, D:..F:), log on
 * C:.  The regions come from the drive's BPB and, on NewTOS, the partition's
 * start from the MBR, read physically (unit 26, docs/api/bios/04-rwabs.md).
 *
 * A "save" is what a program does to save one file: Fcreate, Fwrite, Fclose,
 * the program's view of it is the three codes.  V is the victim:
 *   37156 bytes in the two chunks of a 32-KiB stdio buffer, RESULTS.PDF of the
 *   ticket's first run.
 *
 * Steps: B a baseline save; arm; V; (persistent cases) three more saves with
 * the fault still armed; disarm; four saves R1..R4 ("the card has recovered");
 * then every file again: Fsfirst size, Fopen + Fread + compare.  The state on
 * the card itself is the run's: the image after the run, without any shutdown
 * (tosdump/probes/run-wfprobe.sh), against fsck_msdos -n.
 *
 * Output: D:\WFPROBE.LOG, created BEFORE the fault (an Fcreate may never
 * succeed again: that is what is measured), and every line also to the
 * console, so the serial log has it whatever happens to the disk.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

/* faults (emmc_fault.rs Mode) */
#define F_NONE  0
#define F_R1    1
#define F_DCRC  2
#define F_TORDY 3
#define F_BUSY  4
#define F_WEDGE 5
#define F_OOR   6
/* EmuTOS only: the hook answers E_CHNG, what sd_rw gives a timeout after it
 * re-initialised the card (bios/sd.c:192-199) */
#define F_CHNG  7

/* regions, logical sectors of C: */
#define R_ALL  0
#define R_FAT  1
#define R_ROOT 2
#define R_DATA 3

#define EWRITF (-10L)
#define EDRVNR (-2L)
#define E_CHNG (-14L)

struct wfcase {
    const char *id;
    int fault;      /* what fires */
    int region;     /* where */
    int count;      /* how many writes, 0 = until disarmed */
    int ms;         /* F_BUSY: how long */
    int saves;      /* saves with the fault still armed */
    const char *what;
};

static const struct wfcase cases[] = {
    { "a1", F_R1,    R_DATA, 1, 0, 0, "one refused write, file data" },
    { "a2", F_R1,    R_FAT,  1, 0, 0, "one refused write, FAT sector" },
    { "a3", F_R1,    R_ROOT, 1, 0, 0, "one refused write, directory sector" },
    { "a4", F_R1,    R_ROOT, 3, 0, 0, "three refused writes, directory sector" },
    { "a5", F_R1,    R_FAT,  3, 0, 0, "three refused writes, FAT sector" },
    { "b1", F_R1,    R_ALL,  0, 0, 3, "every write refused for a while, all of C:" },
    { "b2", F_R1,    R_ROOT, 0, 0, 3, "every write refused for a while, directory" },
    { "b3", F_R1,    R_FAT,  0, 0, 3, "every write refused for a while, FAT" },
    { "b4", F_R1,    R_DATA, 0, 0, 3, "every write refused for a while, file data" },
    { "b5", F_R1,    R_FAT,  0, 0, 3, "as b3, then an Fdelete on the same FAT sector" },
    { "d1", F_DCRC,  R_ALL,  0, 0, 3, "data CRC error on every write for a while" },
    { "t1", F_TORDY, R_ROOT, 1, 0, 0, "one write times out, directory" },
    { "t2", F_TORDY, R_ALL,  0, 0, 3, "every write times out for a while" },
    { "u1", F_BUSY,  R_ROOT, 3, 900, 0, "busy 900 ms, under the limit, directory" },
    { "u2", F_BUSY,  R_ROOT, 1, 1200, 0, "busy 1200 ms, over the limit, directory" },
    { "u3", F_BUSY,  R_ALL,  0, 1200, 3, "busy 1200 ms on every write for a while" },
    { "u4", F_BUSY,  R_ROOT, 1, 2200, 0, "busy 2200 ms, directory" },
    { "u5", F_BUSY,  R_ROOT, 1, 3500, 0, "busy 3500 ms, directory" },
    { "u6", F_BUSY,  R_FAT,  1, 2200, 0, "busy 2200 ms, FAT sector" },
    { "w1", F_WEDGE, R_ALL,  0, 0, 3, "controller stuck for a while (DAT inhibit)" },
    { "o1", F_OOR,   R_ROOT, 1, 0, 0, "a real command behind the end of the card" },
    { "c1", F_CHNG,  R_ROOT, 1, 0, 0, "EmuTOS only: E_CHNG once, directory" },
    { 0, 0, 0, 0, 0, 0, 0 }
};

static char line[512];
static long loghandle = -1;
static char drv = 'C';
static char pbuf[32];
static unsigned long part_start;
static long b_fsiz, b_fatrec, b_rdlen, b_datrec;
static char chunk[37156 + 5281] __attribute__((aligned(2)));
static char back[37156 + 5281] __attribute__((aligned(2)));

/* Log to the console now, to the pre-created file at the end. */
static char out[120000];
static long olen;

static void emit(const char *fmt, ...)
{
    va_list ap;
    int n;

    va_start(ap, fmt);
    n = vsnprintf(line, sizeof line, fmt, ap);
    va_end(ap);
    if (n < 0)
        return;
    if (n >= (int)sizeof line)
        n = sizeof line - 1;
    if (olen + n < (long)sizeof out) {
        memcpy(out + olen, line, n);
        olen += n;
    }
    if (n > 0 && line[n - 1] == '\n') {
        line[n - 1] = 0;
        Cconws(line);
        Cconws("\r\n");
    } else {
        Cconws(line);
    }
}

static void fill(void)
{
    long i;

    for (i = 0; i < (long)sizeof chunk; i++)
        chunk[i] = (char)('A' + (i / 509 + i) % 26);
}

/* ---------------------------------------------------------------- fault */

static const char *fn(const char *name)
{
    sprintf(pbuf, "%c:\\%s", drv, name);
    return pbuf;
}

/* The drive's layout: the BPB, and on NewTOS the partition's first LBA. */
static int layout(void)
{
    const unsigned short *w = (const unsigned short *)trap_13_ww((short)7, (short)(drv - 'A'));

    if (!w) {
        emit("Getbpb(%c:) = 0\n", drv);
        return 0;
    }
    b_rdlen = w[3];
    b_fsiz = w[4];
    b_fatrec = w[5];
    b_datrec = w[6];
#ifdef __NEWTOS__
    {
        static unsigned char mbr[512] __attribute__((aligned(16)));
        const unsigned char *e;
        long r = trap_13_wwlwwwl((short)4, (short)(8 | 2), (long)mbr, (short)1, (short)0, (short)26, 0L);

        if (r != 0) {
            emit("MBR read = %ld\n", r);
            return 0;
        }
        e = mbr + 0x1BE + 16 * (drv - 'C');
        part_start = (unsigned long)e[8] | ((unsigned long)e[9] << 8) | ((unsigned long)e[10] << 16)
                     | ((unsigned long)e[11] << 24);
    }
#endif
    emit("%c: partition at %lu, FATs at %ld and %ld, root at %ld, data at %ld\n", drv, part_start,
         b_fatrec - b_fsiz, b_fatrec, b_fatrec + b_fsiz, b_datrec);
    return 1;
}

/* Logical sectors of a region; hi 0 = to the end. */
static void region_bounds(int region, unsigned long *lo, unsigned long *hi)
{
    switch (region) {
    case R_FAT:  *lo = b_fatrec - b_fsiz; *hi = b_fatrec + b_fsiz; break;
    case R_ROOT: *lo = b_fatrec + b_fsiz; *hi = b_datrec;          break;
    case R_DATA: *lo = b_datrec;          *hi = 0;                 break;
    default:     *lo = 0;                 *hi = 0;                 break;
    }
}

#ifdef __NEWTOS__
#define S_NEWTOS_SDDIAG  2090
#define S_NEWTOS_SDFAULT 2091

static char diagtext[4096];

static long ssystem(int mode, unsigned long a1, unsigned long a2)
{
    return trap_1_wwll((short)0x154, (short)mode, (long)a1, (long)a2);
}

static void arm(const struct wfcase *c)
{
    unsigned int a[6];
    unsigned long lo, hi;
    long r;

    region_bounds(c->region, &lo, &hi);
    a[0] = (unsigned int)c->fault;
    a[1] = 0;
    a[2] = (unsigned int)c->count;
    a[3] = (unsigned int)c->ms;
    /* Device LBAs; only this partition, the log lives on another one. */
    a[4] = (unsigned int)(lo + part_start);
    a[5] = (unsigned int)(hi ? hi + part_start : part_start + 0x10000000UL);
    r = ssystem(S_NEWTOS_SDFAULT, (unsigned long)a, 24);
    emit("arm fault %d: Ssystem=%ld\n", c->fault, r);
}

static void disarm(void)
{
    unsigned int a[6];
    long r;

    memset(a, 0, sizeof a);
    r = ssystem(S_NEWTOS_SDFAULT, (unsigned long)a, 24);
    emit("disarm: fired %ld\n", r);
}

static void diag(const char *when)
{
    long n = ssystem(S_NEWTOS_SDDIAG, (unsigned long)diagtext, sizeof diagtext - 1);

    emit("--- diag %s (%ld bytes)\n", when, n);
    if (n > 0) {
        long i = 0;

        diagtext[n] = 0;
        while (i < n) {
            long e = i;

            while (e < n && diagtext[e] != '\n')
                e++;
            diagtext[e] = 0;
            emit("  %s\n", diagtext + i);
            i = e + 1;
        }
    }
}
#else /* m68k: the hdv_rw hook */
static long (*old_rw)(void);
static volatile long h_left, h_on, h_lo, h_hi, h_code, h_fired;
static long newvec;

/* args as bytes on the stack at entry: 4 rw.w 6 buf.l 10 cnt.w 12 recnr.w
 * 14 dev.w 16 lrecnr.l */
__asm__(
    ".text\n"
    "_wf_hook:\n"
    "    move.l  16(%sp),-(%sp)\n"
    "    moveq   #0,%d0\n"
    "    move.w  18(%sp),%d0\n"        /* dev */
    "    move.l  %d0,-(%sp)\n"
    "    moveq   #0,%d0\n"
    "    move.w  20(%sp),%d0\n"        /* recnr */
    "    move.l  %d0,-(%sp)\n"
    "    moveq   #0,%d0\n"
    "    move.w  16(%sp),%d0\n"        /* rw */
    "    move.l  %d0,-(%sp)\n"
    "    jsr     _wf_decide\n"
    "    lea     16(%sp),%sp\n"
    "    tst.l   %d0\n"
    "    jne     1f\n"
    "    move.l  _old_rw,%a0\n"
    "    jmp     (%a0)\n"
    "1:  rts\n");
extern void wf_hook(void);
long wf_decide(long rw, long recnr, long dev, long lrecnr);

long wf_decide(long rw, long recnr, long dev, long lrecnr)
{
    unsigned long rec;

    if (!h_on || dev != 2 || !(rw & 1) || (rw & 8))
        return 0;
    rec = recnr == 0xFFFFL ? (unsigned long)lrecnr : (unsigned long)recnr;
    if (rec < (unsigned long)h_lo)
        return 0;
    if (h_hi && rec >= (unsigned long)h_hi)
        return 0;
    if (h_left == 0)
        return 0;
    if (h_left > 0)
        h_left--;
    h_fired++;
    return h_code;
}

static long install(void)
{
    old_rw = *(long (**)(void))0x476;
    *(long *)0x476 = newvec;
    return 0;
}

static long uninstall(void)
{
    *(long (**)(void))0x476 = old_rw;
    return 0;
}

static void arm(const struct wfcase *c)
{
    unsigned long lo, hi;

    region_bounds(c->region, &lo, &hi);
    h_lo = (long)lo;
    h_hi = (long)hi;
    h_left = c->count ? c->count : -1;
    h_fired = 0;
    switch (c->fault) {
    case F_TORDY: h_code = EDRVNR; break;
    case F_CHNG:  h_code = E_CHNG; break;
    default:      h_code = EWRITF; break;
    }
    h_on = 1;
    emit("arm: hdv_rw hook answers %ld on sectors %lu..%lu count %d\n", h_code, lo, hi, c->count);
}

static void disarm(void)
{
    h_on = 0;
    emit("disarm: fired %ld\n", h_fired);
}

static void diag(const char *when)
{
    (void)when;
}
#endif

/* ---------------------------------------------------------------- saves */

/* The three codes a saving program sees. `split`: two chunks as stdio does. */
static void save(const char *name, long size, long split)
{
    long fh, w1 = 0, w2 = 0, cl = 0;

    fh = Fcreate(fn(name), 0);
    if (fh >= 0) {
        short h = (short)fh;

        w1 = Fwrite(h, split ? split : size, chunk);
        if (split && w1 >= 0)
            w2 = Fwrite(h, size - split, chunk + split);
        cl = Fclose(h);
    }
    emit("save %-14s Fcreate=%ld Fwrite=%ld,%ld Fclose=%ld\n", name, fh, w1, w2, cl);
}

/* What the file system says now, without the program's own memory. */
static void look(const char *name, long want)
{
    static _DTA dta __attribute__((aligned(2)));
    long r, fh, n;
    long size = -1;

    Fsetdta(&dta);
    r = Fsfirst(fn(name), 0);
    if (r == 0)
        size = (long)dta.dta_size;
    fh = Fopen(fn(name), 0);
    n = -1;
    if (fh >= 0) {
        n = Fread((short)fh, sizeof back, back);
        Fclose((short)fh);
    }
    emit("look %-14s Fsfirst=%ld size=%ld want=%ld Fopen=%ld Fread=%ld same=%d\n", name, r, size,
         want, fh, n, n > 0 && memcmp(back, chunk, n) == 0);
}

static const char *const rnames[] = { "R1.FBP", "R2.FBP", "R3.FBP", "R4.FBP" };
static const char *const pnames[] = { "P1.FBP", "P2.FBP", "P3.FBP" };

static void run_case(const struct wfcase *c)
{
    int i;

    emit("=== WFPROBE case %s: %s\n", c->id, c->what);
    if (!layout())
        return;
    save("BASE.DAT", 5281, 0);
    diag("after the baseline");
    arm(c);
    save("RESULTS.PDF", 37156, 32768);
    diag("after the victim");
    for (i = 0; i < c->saves; i++)
        save(pnames[i], 5281, 0);
    if (c->saves)
        diag("after the saves under fault");
    disarm();
    for (i = 0; i < 4; i++)
        save(rnames[i], 5281, 0);
    diag("after the saves with the fault gone");
    look("BASE.DAT", 5281);
    look("RESULTS.PDF", 37156);
    for (i = 0; i < c->saves; i++)
        look(pnames[i], 5281);
    for (i = 0; i < 4; i++)
        look(rnames[i], 5281);
    {
        _DISKINFO di;
        long r;

        memset(&di, 0, sizeof di);
        r = Dfree(&di, drv - 'A' + 1);
        emit("Dfree(%c:)=%ld free=%lu total=%lu\n", drv, r, (unsigned long)di.b_free,
             (unsigned long)di.b_total);
    }
    diag("at the end");
    if (!strcmp(c->id, "b5")) {
        /* A write that touches the stuck FAT sector again: does it free the
         * system?  (RESULTS.PDF's chain lies in that sector.) */
        long r = Fdelete(fn("RESULTS.PDF"));

        emit("Fdelete(RESULTS.PDF)=%ld\n", r);
        save("X1.FBP", 5281, 0);
        look("X1.FBP", 5281);
        diag("after the Fdelete");
    }
}

int main(void)
{
    const struct wfcase *c = cases;
    long r;

    fill();
#ifdef __NEWTOS__
    /* Before the fault: the log's drive shares the cache and the driver with
     * the drive under test. */
#ifdef WFCASE
    loghandle = Fcreate("D:\\WFPROBE.LOG", 0);
#else
    loghandle = Fcreate("C:\\WFPROBE.LOG", 0);
#endif
#else
    newvec = (long)wf_hook;
    Supexec(install);
#endif
#ifdef WFCASE
    while (c->id && strcmp(c->id, WFCASE))
        c++;
    if (c->id)
        run_case(c);
    else
        emit("unknown case %s\n", WFCASE);
#else
    {
        static const struct { int key; const char *id; } menu[] = {
            { 'o', "o1" }, { '3', "a3" }, { 't', "t1" }, { 'd', "d1" },
            { 'u', "u2" }, { 'v', "u4" }, { 'w', "u5" }, { 'f', "u6" },
            { 'b', "b3" }, { 'x', "b5" }, { 0, 0 }
        };
        int key, i;

        emit("WFPROBE (TASK/HW08), test card, sd-fault kernel. Boot fresh for every run.\n");
        emit("o out of range, real command (M-A)      t one write timeout, CMD12 in a transfer (M-C)\n");
        emit("d data CRC errors for a while (M-C)     u busy 1.2 s  v busy 2.2 s  w busy 3.5 s (dir)\n");
        emit("f busy 2.2 s, FAT    3 one refused write   b FAT refused for a while  x b + Fdelete\n");
        emit("key (other: end): ");
        key = (int)(Cconin() & 0xFF);
        emit("\ndrive to run on, D, E or F: ");
        i = (int)(Cconin() & 0xFF);
        if (i >= 'a')
            i -= 32;
        emit("\n");
        if (i < 'D' || i > 'F') {
            emit("end\n");
            key = 0;
        }
        drv = (char)i;
        for (i = 0; menu[i].key && menu[i].key != key; i++)
            ;
        if (menu[i].key) {
            while (c->id && strcmp(c->id, menu[i].id))
                c++;
            if (c->id)
                run_case(c);
        }
    }
#endif
#ifndef __NEWTOS__
    Supexec(uninstall);
    /* D: is Hatari's own drive here, no fault reaches it; the oracle script
     * takes the file's appearance as the end of the run. */
    emit("END\n");
    loghandle = Fcreate("D:\\WFPROBE.LOG", 0);
#else
    emit("END\n");
#endif
    if (loghandle >= 0) {
        r = Fwrite((short)loghandle, olen, out);
        Cconws("log Fwrite done\r\n");
        (void)r;
        r = Fclose((short)loghandle);
        (void)r;
    }
    Cconws("WFPROBE done\r\n");
    return 0;
}

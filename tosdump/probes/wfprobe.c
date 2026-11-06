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
 * Three cases have no fault, they ask what the file system writes and when:
 *   n1  commands and sectors each call sends to the device, per Fwrite pattern
 *       (the pattern table of sdk/examples/fswbench.c, L2)
 *   n2  a file written, not closed yet: Fsfirst, Dfree, a second handle,
 *       Fdatime, Fattrib, and what each of them writes
 *   n3  a child that writes and ends with Pterm, without Fclose
 *   n4  a power failure: files written and not closed, then the probe stops
 *       dead and the run kills the machine (no log on NewTOS: its Fclose would
 *       write back C: too; the serial line has the text).
 * One more has a read fault:
 *   r1  FAT reads fail: Dfree's code, then the same Dfree with the fault gone
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
/* a card still programming after the write's timeout: CMD13 says prg, a data
 * command gets no answer (emmc_fault.rs Mode::Prg, NewTOS only) */
#define F_PRG   8
/* reads of the region fail: EREADF (emmc_fault.rs Mode::ReadErr; the hook) */
#define F_RDERR 9

/* regions, logical sectors of C: */
#define R_ALL  0
#define R_FAT  1
#define R_ROOT 2
#define R_DATA 3

#define EWRITF (-10L)
#define EDRVNR (-2L)
#define EREADF (-11L)
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
    { "p1", F_PRG,   R_ROOT, 1, 1200, 0, "card programs 1200 ms, directory" },
    { "p2", F_PRG,   R_ROOT, 1, 2600, 0, "card programs 2600 ms, directory" },
    { "p3", F_PRG,   R_FAT,  1, 2600, 0, "card programs 2600 ms, FAT sector" },
    { "p4", F_PRG,   R_ALL,  0, 1200, 3, "card programs 1200 ms after every write for a while" },
    { "n1", F_NONE,  R_ALL,  0, 0, 0, "writes per Fwrite pattern, no fault" },
    { "n2", F_NONE,  R_ALL,  0, 0, 0, "a file written and not closed yet, no fault" },
    { "n3", F_NONE,  R_ALL,  0, 0, 0, "a child ends without Fclose, no fault" },
    { "n4", F_NONE,  R_ALL,  0, 0, 0, "power failure with files not closed, no fault" },
    { "r1", F_RDERR, R_FAT,  0, 0, 0, "FAT reads fail: Dfree" },
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

/* Write commands and sectors the driver has sent, from the SD report. */
static void wcount(unsigned long *cmds, unsigned long *secs)
{
    long n = ssystem(S_NEWTOS_SDDIAG, (unsigned long)diagtext, sizeof diagtext - 1);
    long i;

    *cmds = *secs = 0;
    for (i = 0; i + 8 < n; i++) {
        if ((i == 0 || diagtext[i - 1] == '\n') && !strncmp(diagtext + i, "write", 5)) {
            i += 5;
            while (i < n && diagtext[i] != 'n')
                i++;
            for (i += 2; i < n && diagtext[i] >= '0' && diagtext[i] <= '9'; i++)
                *cmds = *cmds * 10 + (unsigned long)(diagtext[i] - '0');
            for (i += 5; i < n && diagtext[i] >= '0' && diagtext[i] <= '9'; i++)
                *secs = *secs * 10 + (unsigned long)(diagtext[i] - '0');
            return;
        }
    }
}
#else /* m68k: the hdv_rw hook */
static long (*old_rw)(void);
static volatile long h_left, h_on, h_lo, h_hi, h_code, h_fired, h_reads;
static volatile unsigned long h_wcmds, h_wsecs;
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
    "    move.w  22(%sp),%d0\n"        /* cnt */
    "    move.l  %d0,-(%sp)\n"
    "    moveq   #0,%d0\n"
    "    move.w  20(%sp),%d0\n"        /* rw */
    "    move.l  %d0,-(%sp)\n"
    "    jsr     _wf_decide\n"
    "    lea     20(%sp),%sp\n"
    "    tst.l   %d0\n"
    "    jne     1f\n"
    "    move.l  _old_rw,%a0\n"
    "    jmp     (%a0)\n"
    "1:  rts\n");
extern void wf_hook(void);
long wf_decide(long rw, long cnt, long recnr, long dev, long lrecnr);

long wf_decide(long rw, long cnt, long recnr, long dev, long lrecnr)
{
    unsigned long rec;

    if (dev != 2 || (rw & 8))
        return 0;
    if (rw & 1) {
        h_wcmds++;
        h_wsecs += (unsigned long)cnt;
    }
    if (!h_on || (h_reads ? (rw & 1) : !(rw & 1)))
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
    h_reads = c->fault == F_RDERR;
    switch (c->fault) {
    case F_TORDY: h_code = EDRVNR; break;
    case F_CHNG:  h_code = E_CHNG; break;
    case F_RDERR: h_code = EREADF; break;
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

static void wcount(unsigned long *cmds, unsigned long *secs)
{
    *cmds = h_wcmds;
    *secs = h_wsecs;
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

/* ---------------------------------------------------------- no fault */

struct wc {
    unsigned long cmds, secs;
};

static struct wc wc_now(void)
{
    struct wc w;

    wcount(&w.cmds, &w.secs);
    return w;
}

/* Writes since *since, as "cmds/sectors", and move *since on. */
static const char *wc_since(struct wc *since)
{
    static char t[4][32];
    static int next;
    struct wc w = wc_now();
    char *o = t[next++ & 3];

    sprintf(o, "%lu/%lu", w.cmds - since->cmds, w.secs - since->secs);
    *since = w;
    return o;
}

static char *big;

/* One save, `step` bytes per Fwrite: commands/sectors of each stage. */
static void pattern(long total, long step)
{
    struct wc w = wc_now();
    long fh, rest = total, calls = 0, last = 0;

    fh = Fcreate(fn("PAT.DAT"), 0);
    emit("pat total %8ld step %6ld: Fcreate=%ld writes %s", total, step, fh, wc_since(&w));
    if (fh < 0) {
        emit("\n");
        return;
    }
    while (rest > 0) {
        long n = rest < step ? rest : step;

        last = Fwrite((short)fh, n, big);
        calls++;
        if (last != n)
            break;
        rest -= n;
    }
    emit(" Fwrite x%ld last=%ld writes %s", calls, last, wc_since(&w));
    last = Fclose((short)fh);
    emit(" Fclose=%ld writes %s\n", last, wc_since(&w));
    last = Fdelete(fn("PAT.DAT"));
    if (last)
        emit("Fdelete=%ld\n", last);
}

static void case_n1(void)
{
    static const long p[][2] = {
        { 10240, 100 },       { 10240, 512 },      { 102400, 512 },
        { 10240, 4096 },      { 102400, 4096 },    { 1048576, 4096 },
        { 102400, 1024 },     { 102400, 32768 },   { 1048576, 32768 },
        { 10485760, 32768 },  { 10240, 10240 },    { 102400, 102400 },
        { 1048576, 1048576 }, { 10485760, 10485760 },
        { 0, 0 }
    };
    int i;
    long fh;

    big = (char *)Mxalloc(10485760L, 3);
    if ((long)big <= 0) {
        emit("Mxalloc = %ld\n", (long)big);
        return;
    }
    for (i = 0; i < 10485760L; i += 4096)
        memcpy(big + i, chunk, 4096);
    for (i = 0; p[i][0]; i++)
        pattern(p[i][0], p[i][1]);

    /* Overwrite inside a closed file: partial sectors, nothing else changes. */
    fh = Fcreate(fn("OVER.DAT"), 0);
    if (fh >= 0) {
        struct wc w;
        long rc = Fwrite((short)fh, 1048576L, big);

        rc += Fclose((short)fh);
        fh = Fopen(fn("OVER.DAT"), 2);
        w = wc_now();
        for (i = 0; i < 64; i++) {
            Fseek(1024L * i + 3, (short)fh, 0);
            rc = Fwrite((short)fh, 100L, chunk + 1000);
        }
        emit("overwrite 64 x 100 B, 1 KiB apart: last Fwrite=%ld writes %s", rc, wc_since(&w));
        rc = Fclose((short)fh);
        emit(" Fclose=%ld writes %s\n", rc, wc_since(&w));
        rc = Fdelete(fn("OVER.DAT"));
        (void)rc;
    }
}

static long free_clusters(void)
{
    _DISKINFO di;
    long r;

    memset(&di, 0, sizeof di);
    r = Dfree(&di, drv - 'A' + 1);
    return r ? r : (long)di.b_free;
}

static void case_n2(void)
{
    static _DTA dta __attribute__((aligned(2)));
    struct wc w;
    long h1, h2, r, free0, n;
    unsigned short td[2];

    Fsetdta(&dta);
    free0 = free_clusters();
    w = wc_now();
    h1 = Fcreate(fn("SEMA.DAT"), 0);
    emit("Fcreate=%ld writes %s\n", h1, wc_since(&w));
    r = Fwrite((short)h1, 5000L, chunk);
    emit("Fwrite 5000=%ld writes %s\n", r, wc_since(&w));

    /* the file as other calls see it now */
    r = Fsfirst(fn("SEMA.DAT"), 0);
    emit("open: Fsfirst=%ld size=%ld writes %s\n", r, r ? -1L : (long)dta.dta_size, wc_since(&w));
    r = free_clusters();
    emit("open: Dfree free=%ld (before the file %ld) writes %s\n", r, free0, wc_since(&w));
    r = Fattrib(fn("SEMA.DAT"), 0, 0);
    emit("open: Fattrib get=%ld writes %s\n", r, wc_since(&w));
    h2 = Fopen(fn("SEMA.DAT"), 0);
    emit("open: second handle Fopen=%ld writes %s\n", h2, wc_since(&w));
    r = h2 >= 0 ? Fseek(0L, (short)h2, 2) : h2;
    emit("open: second handle Fseek end=%ld\n", r);
    if (h2 >= 0) {
        Fseek(0L, (short)h2, 0);
        n = Fread((short)h2, 5000L, back);
        emit("open: second handle Fread=%ld same=%d writes %s\n", n, n > 0 && !memcmp(back, chunk, n),
             wc_since(&w));
    }
    td[0] = 0x1234;
    td[1] = 0x5678;
    r = Fdatime((void *)td, (short)h1, 1);
    emit("open: Fdatime set=%ld writes %s\n", r, wc_since(&w));
    td[0] = td[1] = 0;
    r = h2 >= 0 ? Fdatime((void *)td, (short)h2, 0) : h2;
    emit("open: Fdatime get on the second handle=%ld %04x %04x writes %s\n", r, td[0], td[1], wc_since(&w));
    if (h2 >= 0) {
        r = Fclose((short)h2);
        emit("Fclose of the second handle=%ld writes %s\n", r, wc_since(&w));
    }
    r = Fsfirst(fn("SEMA.DAT"), 0);
    emit("then: Fsfirst=%ld size=%ld writes %s\n", r, r ? -1L : (long)dta.dta_size, wc_since(&w));

    r = Fwrite((short)h1, 3000L, chunk + 5000);
    emit("Fwrite 3000=%ld writes %s\n", r, wc_since(&w));
    r = Fclose((short)h1);
    emit("Fclose=%ld writes %s\n", r, wc_since(&w));
    r = Fsfirst(fn("SEMA.DAT"), 0);
    emit("closed: Fsfirst=%ld size=%ld time %04x date %04x writes %s\n", r, r ? -1L : (long)dta.dta_size,
         r ? 0 : (unsigned)dta.dta_time, r ? 0 : (unsigned)dta.dta_date, wc_since(&w));

    /* overwriting in place, read through a second handle before any close */
    h1 = Fopen(fn("SEMA.DAT"), 2);
    Fseek(100L, (short)h1, 0);
    r = Fwrite((short)h1, 50L, chunk + 2000);
    emit("overwrite 50 B at 100: Fwrite=%ld writes %s\n", r, wc_since(&w));
    h2 = Fopen(fn("SEMA.DAT"), 0);
    Fseek(100L, (short)h2, 0);
    n = Fread((short)h2, 50L, back);
    emit("overwrite: second handle Fread=%ld same=%d writes %s\n", n, n > 0 && !memcmp(back, chunk + 2000, n),
         wc_since(&w));
    r = Fclose((short)h2);
    emit("overwrite: Fclose of the second handle=%ld writes %s\n", r, wc_since(&w));
    r = Fclose((short)h1);
    emit("overwrite: Fclose=%ld writes %s\n", r, wc_since(&w));
}

#ifdef __NEWTOS__
#define SELF "D:\\WFPROBE.NAP"
#else
#define SELF "D:\\WFPROBE.PRG"
#endif

/* The child of n3: writes, partial last sector, and ends with no Fclose. */
static void child(void)
{
    long fh = Fcreate("C:\\CHILD.DAT", 0);

    if (fh >= 0) {
        Fwrite((short)fh, 5000L, chunk);
        Fwrite((short)fh, 281L, chunk + 5000);
    }
    Pterm(7);
}

static void case_n3(void)
{
    static _DTA dta __attribute__((aligned(2)));
    struct wc w = wc_now();
    long r, fh, n;

    Fsetdta(&dta);
    r = Pexec(0, SELF, "\005child", NULL);
    emit("Pexec child=%ld writes %s\n", r, wc_since(&w));
    r = Fsfirst("C:\\CHILD.DAT", 0);
    emit("after the child: Fsfirst=%ld size=%ld writes %s\n", r, r ? -1L : (long)dta.dta_size, wc_since(&w));
    fh = Fopen("C:\\CHILD.DAT", 0);
    n = fh >= 0 ? Fread((short)fh, 5281L, back) : fh;
    emit("after the child: Fopen=%ld Fread=%ld same=%d\n", fh, n, n > 0 && !memcmp(back, chunk, n));
    if (fh >= 0)
        Fclose((short)fh);
    emit("after the child: Dfree free=%ld\n", free_clusters());
}

/* What a power failure leaves behind.  EXIST.DAT and OLD.DAT are closed before,
 * so they are on the card; NEW.DAT is new; EXIST.DAT gets 100 B overwritten at
 * 100 and 3000 B at its end; OLD.DAT 700 B overwritten at 512, a whole record
 * and a bit. */
static void case_n4(void)
{
    static char x[3000];
    long a, b, c;

    memset(x, 'X', sizeof x);
    a = Fcreate(fn("EXIST.DAT"), 0);
    Fwrite((short)a, 20000L, chunk);
    Fclose((short)a);
    b = Fcreate(fn("OLD.DAT"), 0);
    Fwrite((short)b, 20000L, chunk);
    Fclose((short)b);

    c = Fcreate(fn("NEW.DAT"), 0);
    emit("Fcreate NEW.DAT=%ld Fwrite=%ld\n", c, Fwrite((short)c, 5281L, chunk));
    a = Fopen(fn("EXIST.DAT"), 2);
    Fseek(100L, (short)a, 0);
    emit("EXIST.DAT overwrite=%ld\n", Fwrite((short)a, 100L, x));
    Fseek(0L, (short)a, 2);
    emit("EXIST.DAT extend=%ld\n", Fwrite((short)a, 3000L, x));
    b = Fopen(fn("OLD.DAT"), 2);
    Fseek(512L, (short)b, 0);
    emit("OLD.DAT overwrite=%ld\n", Fwrite((short)b, 700L, x));
    emit("WFPROBE power off\n");
#ifndef __NEWTOS__
    /* D: is Hatari's own drive here: no EmuTOS buffer moves. */
    a = Fcreate("D:\\WFPROBE.LOG", 0);
    Fwrite((short)a, olen, out);
    Fclose((short)a);
#endif
    for (;;)
        ;
}

static void case_r1(const struct wfcase *c)
{
    _DISKINFO di;
    long r;

    arm(c);
    memset(&di, 0, sizeof di);
    r = Dfree(&di, drv - 'A' + 1);
    emit("Dfree under the fault=%ld free=%lu total=%lu\n", r, (unsigned long)di.b_free, (unsigned long)di.b_total);
    disarm();
    memset(&di, 0, sizeof di);
    r = Dfree(&di, drv - 'A' + 1);
    emit("Dfree with the fault gone=%ld free=%lu total=%lu\n", r, (unsigned long)di.b_free, (unsigned long)di.b_total);
}

static void run_any(const struct wfcase *c)
{
    if (c->fault == F_NONE || c->fault == F_RDERR) {
        emit("=== WFPROBE case %s: %s\n", c->id, c->what);
        if (!layout())
            return;
        if (!strcmp(c->id, "n1"))
            case_n1();
        else if (!strcmp(c->id, "n2"))
            case_n2();
        else if (!strcmp(c->id, "n3"))
            case_n3();
        else if (!strcmp(c->id, "n4"))
            case_n4();
        else
            case_r1(c);
        diag("at the end");
        return;
    }
    run_case(c);
}

int main(int argc, char **argv)
{
    const struct wfcase *c = cases;
    long r;

    fill();
    if (argc > 1 && !strcmp(argv[1], "child"))
        child();
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
        run_any(c);
    else
        emit("unknown case %s\n", WFCASE);
#else
    {
        static const struct { int key; const char *id; } menu[] = {
            { 'o', "o1" }, { '3', "a3" }, { 't', "t1" }, { 'd', "d1" },
            { 'u', "u2" }, { 'v', "u4" }, { 'w', "u5" }, { 'f', "u6" },
            { 'b', "b3" }, { 'x', "b5" }, { 'p', "p2" }, { 'q', "p3" }, { 0, 0 }
        };
        int key, i;

        emit("WFPROBE (TASK/HW08), test card, sd-fault kernel. Boot fresh for every run.\n");
        emit("o out of range, real command (M-A)      t one write timeout, CMD12 in a transfer (M-C)\n");
        emit("d data CRC errors for a while (M-C)     u busy 1.2 s  v busy 2.2 s  w busy 3.5 s (dir)\n");
        emit("f busy 2.2 s, FAT    3 one refused write   b FAT refused for a while  x b + Fdelete\n");
        emit("p programming 2.6 s, dir    q programming 2.6 s, FAT\n");
        emit("key (other: end): ");
#ifdef WFKEYS
        /* QEMU raspi4b has no USB keyboard: the two answers at build time, to
         * drive this path (measurements/measurement-2026-11-06-hw08). */
        key = WFKEYS[0];
        emit("\ndrive to run on, D, E or F: ");
        i = WFKEYS[1];
#else
        key = (int)(Cconin() & 0xFF);
        emit("\ndrive to run on, D, E or F: ");
        i = (int)(Cconin() & 0xFF);
#endif
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
                run_any(c);
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

/*
 * usbsnap.c -- USBSNAP.NCP, the USB device recorder (USB01 step 2, K10).
 *
 * A native NewTOS console program (`.NCP`, ADR-018) for non-developers: it
 * lists the USB devices the kernel enumerated at boot, records 30 seconds of
 * their interrupt reports while the user moves the mouse / presses keys, and
 * writes one self-describing text file per device to C:\USB\. The files go
 * to the developer, who reads from them what each mouse speaks.
 *
 * Spec: docs/api/usb/usb-snapshot.md
 *   §2   Ssystem(2022) wire layout (header, LIST, DEVICE, REPORTS, MUTE)
 *   §4   this program: dialog (§4.2), file names (§4.3), file content (§4.4)
 *   §7   user decisions (MUTE on, strings, C:\USB\, English, hubs = path only)
 *   §9   kernel implementation notes the parser below relies on
 *
 * Calls (all GEMDOS, svc #1):
 *   Ssystem  0x154  docs/api/gemdos/154-ssystem.md   modes 2019, 2022
 *   Cconws   0x09   Cconis 0x0B   Cnecin 0x08        docs/api/gemdos/console/
 *   Fattrib  0x43   Dcreate 0x39                      docs/api/gemdos/{file,directory}/
 *   Fopen    0x3D   Fcreate 0x3C  Fwrite 0x40  Fclose 0x3E
 *   Malloc   0x48   (only for a DEVICE answer larger than the static buffer)
 *
 * Everything the kernel hands over is little-endian with fixed offsets
 * (§2.1); it is read byte by byte, never through C structs.
 *
 * Decoded names come only from local sources: descriptor fields from HID 1.11
 * Appendix E (references/usb/hid1_11.pdf), report-descriptor items from HID
 * 1.11 §6.2.2.2-6.2.2.8, usage pages / usages from FreeMiNT's HID driver
 * (references/freemint/sys/usb/src.km/udd/hid/hid-common/hid.h), xHCI
 * completion codes from xHCI 1.2 Table 6-90 and speed ids from Table 7-13.
 * Anything else is printed as a number.
 *
 * Build: sdk/build.sh sdk/examples/usbsnap.c <out>/USBSNAP.NCP
 * Staged by tools/make-sd-image.sh as C:\USBSNAP.NCP (double-click from the
 * desktop; NOT in \AUTO\, it needs the user).
 *
 * Test seams (measurement builds only, never set by sdk/build.sh): a wrapper
 * that defines USBSNAP_TEST_SNAPCALL supplies snap_call() (replayed kernel
 * answers), one that defines USBSNAP_TEST_SINK supplies the out_* file
 * functions (file content to the console on a board without a drive). See
 * spec §9.1.
 */
#include <newtos.h>

/* ------------------------------------------------------------------------ */
/* Screen texts (English, §4.1 / §7 decision 6)                             */
/* ------------------------------------------------------------------------ */

#define T_TITLE     "USBSNAP - USB device recorder"
#define T_INTRO     "Writes one text file per USB device to C:\\USB\\ - " \
                    "send them to the developer."
#define T_TOO_OLD   "This kernel is too old (no USB snapshot call)."
#define T_NO_CTRL   "No USB controller in this system. Nothing to record."
#define T_NO_DEV1   "No USB device was detected at boot. Plug the device in and"
#define T_NO_DEV2   "RESTART the machine; devices plugged in later are not seen yet."
#define T_ONLY_HUBS "(Only USB hubs were found; nothing to record.)"
#define T_HUBNOTE   " (hubs are not listed as devices)"
#define T_REC_LEN   "Recording will run for 30 seconds."
#define T_REC_MOUSE "  - MOUSE   : move it, press EVERY button, turn the wheel."
#define T_REC_KBD   "  - KEYBOARD: press a few keys, also Shift/Ctrl/Alt."
#define T_REC_PTR   "The pointer will NOT move during the recording - that is normal."
#define T_START     "Press RETURN to start, or wait 15 seconds."
#define T_RECORDING " Recording...  time left: "
#define T_NOTHING   "   <- nothing received yet"
#define T_NOREC     "not recorded (no input interface)"
#define T_DONE      "Done. Wrote:"
#define T_COPY      "Copy these files from the SD card and send them."
#define T_ANYKEY    "Press any key to exit."
#define T_NOFILES   "No file was written."
#define T_FOLDER    "Cannot use the folder C:\\USB\\ : "
#define T_ISFILE    "C:\\USB exists but is a file, not a folder. Rename it and retry."

#define PROGRAM_VERSION "USBSNAP.NCP v1"

/* ------------------------------------------------------------------------ */
/* GEMDOS                                                                   */
/* ------------------------------------------------------------------------ */

static inline NLONG gemdos3(NLONG fn, NLONG a0, NLONG a1, NLONG a2)
{
    register NLONG x8 __asm__("x8") = fn;
    register NLONG r0 __asm__("x0") = a0;
    register NLONG r1 __asm__("x1") = a1;
    register NLONG r2 __asm__("x2") = a2;
    __asm__ volatile("svc #1"
                     : "+r"(r0)
                     : "r"(x8), "r"(r1), "r"(r2)
                     : "memory");
    return r0;
}

#define G_CNECIN   0x08
#define G_CCONIS   0x0B
#define G_DCREATE  0x39
#define G_FCREATE  0x3C
#define G_FOPEN    0x3D
#define G_FCLOSE   0x3E
#define G_FWRITE   0x40
#define G_FATTRIB  0x43
#define G_MALLOC   0x48
#define G_SSYSTEM  0x154

#define S_NEWTOS_KVERSION 2019
#define S_NEWTOS_USBSNAP  2022

/* tos-types/src/error.rs */
#define E_OK     0L
#define ERROR   (-1L)
#define EINVAL  (-25L)
#define EINVFN  (-32L)
#define EFILNF  (-33L)

#define FA_SUBDIR 0x10

/* clang may lower an aggregate copy/clear to these even freestanding. */
void *memset(void *d, int c, unsigned long n)
{
    unsigned char *p = d;
    while (n--)
        *p++ = (unsigned char)c;
    return d;
}

void *memcpy(void *d, const void *s, unsigned long n)
{
    unsigned char *p = d;
    const unsigned char *q = s;
    while (n--)
        *p++ = *q++;
    return d;
}

/* ------------------------------------------------------------------------ */
/* Snapshot wire layout (spec §2)                                           */
/* ------------------------------------------------------------------------ */

#define HDR_LEN        32
#define OP_LIST        1
#define OP_DEVICE      2
#define OP_REPORTS     3
#define OP_MUTE        4
#define HF_TRUNCATED   0x01
#define HF_MORE        0x02

#define ROLE_NONE      0
#define ROLE_KEYBOARD  1
#define ROLE_MOUSE     2
#define ROLE_OBSERVED  3
#define ROLE_HUB       4
#define ROLE_FAILED    5

#define SEC_DEVICE     1
#define SEC_CONFIG     2
#define SEC_REPORT     3
#define SEC_IFACES     4
#define SEC_MANUF      5
#define SEC_PRODUCT    6
#define SF_TRUNCATED   0x01
#define SF_FAILED      0x02
#define SF_SKIPPED     0x04

#define REC_LEN_V1     80
#define REC_CAPPED     0x01
#define REC_MUTED      0x02

#define MAX_DEV        8        /* MAX_HID, §2.7 */
#define REC_KEEP       1024     /* reports kept per device, §4.2 */
#define REC_TICKS      6000     /* 30 s at 200 Hz */
#define MUTE_TICKS     7000     /* 30 s + 5 s, §4.2 step 4 */
#define START_TICKS    3000     /* 15 s */
#define POLL_TICKS     10       /* 50 ms */
#define RING_V1        256      /* §2.7, layout v1 */

static unsigned rd16(const unsigned char *p) { return (unsigned)p[0] | ((unsigned)p[1] << 8); }
static unsigned rd32(const unsigned char *p) { return rd16(p) | (rd16(p + 2) << 16); }
static void wr16(unsigned char *p, unsigned v) { p[0] = (unsigned char)v; p[1] = (unsigned char)(v >> 8); }
static void wr32(unsigned char *p, unsigned v) { wr16(p, v & 0xFFFF); wr16(p + 2, v >> 16); }

#ifdef USBSNAP_TEST_SNAPCALL
/* Supplied by a measurement wrapper (replayed kernel answers). */
static long snap_call(unsigned char *buf, long len);
#else
static long snap_call(unsigned char *buf, long len)
{
    return gemdos3(G_SSYSTEM, S_NEWTOS_USBSNAP, (NLONG)buf, len);
}
#endif

/* Fill the 32-byte request header (§2.2). */
static void request(unsigned char *buf, unsigned op, unsigned index, unsigned arg)
{
    int k;
    for (k = 0; k < HDR_LEN; k++)
        buf[k] = 0;
    buf[0] = 'U'; buf[1] = 'S'; buf[2] = 'B'; buf[3] = 'S';
    wr16(buf + 4, 1);
    wr16(buf + 8, op);
    wr32(buf + 12, index);
    wr32(buf + 16, arg);
}

/* ------------------------------------------------------------------------ */
/* Line builder: one line in `lb`, sent to the screen or the file           */
/* ------------------------------------------------------------------------ */

static char lb[600];
static int ln;

static void S(const char *s) { while (*s && ln < (int)sizeof lb - 3) lb[ln++] = *s++; }
static void C(char c) { if (ln < (int)sizeof lb - 3) lb[ln++] = c; }
static void U(unsigned long v)
{
    char t[24];
    int i = 0;
    do { t[i++] = (char)('0' + (int)(v % 10)); v /= 10; } while (v);
    while (i) C(t[--i]);
}
static void D(long v) { if (v < 0) { C('-'); U((unsigned long)(-v)); } else U((unsigned long)v); }
static void X(unsigned long v, int digits)
{
    static const char hx[] = "0123456789abcdef";
    while (digits--) C(hx[(v >> (4 * digits)) & 15]);
}
static void XU(unsigned long v, int digits)  /* upper case, for file names */
{
    static const char hx[] = "0123456789ABCDEF";
    while (digits--) C(hx[(v >> (4 * digits)) & 15]);
}
static void PAD(int col) { while (ln < col) C(' '); }
/* Right-aligned unsigned in `w` columns. */
static void UR(unsigned long v, int w)
{
    char t[24];
    int i = 0;
    do { t[i++] = (char)('0' + (int)(v % 10)); v /= 10; } while (v);
    while (w-- > i) C(' ');
    while (i) C(t[--i]);
}
/* Signed with explicit sign, right-aligned in `w` columns. */
static void DR(long v, int w)
{
    char t[24];
    int i = 0;
    unsigned long u = v < 0 ? (unsigned long)(-v) : (unsigned long)v;
    do { t[i++] = (char)('0' + (int)(u % 10)); u /= 10; } while (u);
    t[i++] = v < 0 ? '-' : '+';
    while (w-- > i) C(' ');
    while (i) C(t[--i]);
}
/* Ticks (5 ms) as seconds with 3 decimals, signed. */
static void SECS(long ticks)
{
    long ms;
    if (ticks < 0) { C('-'); ticks = -ticks; }
    ms = ticks * 5;
    U((unsigned long)(ms / 1000));
    C('.');
    C((char)('0' + (int)(ms / 100 % 10)));
    C((char)('0' + (int)(ms / 10 % 10)));
    C((char)('0' + (int)(ms % 10)));
}

/* SECS right-aligned in `w` columns. */
static void SECSR(long ticks, int w)
{
    long a = ticks < 0 ? -ticks : ticks, whole = a * 5 / 1000;
    int width = 4 + (ticks < 0);
    do { width++; whole /= 10; } while (whole);
    while (w-- > width) C(' ');
    SECS(ticks);
}

static void con_line(void)
{
    lb[ln++] = '\r';
    lb[ln++] = '\n';
    lb[ln] = 0;
    Cconws(lb);
    ln = 0;
}
static void con(const char *s) { ln = 0; S(s); con_line(); }

static void esc_clear(void) { Cconws("\033E"); }
/* VT52 ESC Y row col (sdk/examples/vt52pi.c), then clear to end of line. */
static void esc_at(int row, int col)
{
    char e[8];
    e[0] = 033; e[1] = 'Y'; e[2] = (char)(32 + row); e[3] = (char)(32 + col);
    e[4] = 033; e[5] = 'K'; e[6] = 0;
    Cconws(e);
}

static void wait_any_key(void)
{
    con(T_ANYKEY);
    gemdos3(G_CNECIN, 0, 0, 0);
}

/* GEMDOS error names (tos-types/src/error.rs). */
static const char *err_name(long rc)
{
    switch (rc) {
    case -1: return "ERROR";   case -2: return "EDRVNR";  case -3: return "EUNCMD";
    case -4: return "E_CRC";   case -5: return "EBADRQ";  case -6: return "E_SEEK";
    case -7: return "EMEDIA";  case -8: return "ESECNF";  case -9: return "EPAPER";
    case -10: return "EWRITF"; case -11: return "EREADF"; case -13: return "EWRPRO";
    case -14: return "E_CHNG"; case -15: return "EUNDEV"; case -16: return "EBADSF";
    case -17: return "EOTHER"; case -25: return "EINVAL"; case -32: return "EINVFN";
    case -33: return "EFILNF"; case -34: return "EPTHNF"; case -35: return "ENHNDL";
    case -36: return "EACCDN"; case -37: return "EIHNDL"; case -39: return "ENSMEM";
    case -40: return "EIMBA";  case -46: return "EDRIVE"; case -48: return "ENSAME";
    case -49: return "ENMFIL"; case -64: return "ERANGE"; case -65: return "EINTRN";
    default: return "?";
    }
}
static void ERR(long rc) { D(rc); S(" ("); S(err_name(rc)); C(')'); }

/* ------------------------------------------------------------------------ */
/* State                                                                    */
/* ------------------------------------------------------------------------ */

static unsigned char lbuf[16384];             /* LIST answer */
static unsigned char dbuf[MAX_DEV][16384];    /* DEVICE answers */
static unsigned char *dans[MAX_DEV];          /* -> dbuf[i] or a Malloc block */
static long drc[MAX_DEV];                     /* DEVICE call result */
static unsigned char rbuf[8192];              /* REPORTS answer */
static unsigned char cbuf[64];                /* clock / MUTE calls */
static char kversion[96];

static unsigned char recs[MAX_DEV][REC_KEEP][REC_LEN_V1];
struct stat {
    unsigned next;          /* next seq to ask for */
    unsigned long total;    /* reports received */
    unsigned kept;          /* in recs[] */
    unsigned first_seq, last_seq;
    unsigned min_len, max_len;
    unsigned last_ts;
    unsigned long lost;     /* overwritten in the kernel ring before read */
    unsigned rec_len;
};
static struct stat st[MAX_DEV];

static unsigned n_dev, entry_len, root_ports, generation, enum_skipped;
static unsigned t_start, t_end;
static int muted_ok;
static int n_listed;                /* non-hub devices */
static int list_no[MAX_DEV];        /* 1-based screen number, 0 = hub */

static const unsigned char *entry(unsigned i) { return lbuf + HDR_LEN + 16 + i * entry_len; }
static unsigned e_role(unsigned i) { return entry(i)[14]; }

static unsigned now_ticks(void)
{
    request(cbuf, OP_LIST, 0, 0);
    snap_call(cbuf, HDR_LEN);   /* header only (TRUNCATED), carries now */
    return rd32(cbuf + 28);
}

/* ------------------------------------------------------------------------ */
/* Names                                                                    */
/* ------------------------------------------------------------------------ */

static const char *role_short(unsigned r)
{
    switch (r) {
    case ROLE_KEYBOARD: return "Keyboard";
    case ROLE_MOUSE:    return "Mouse";
    case ROLE_OBSERVED: return "HID (report)";
    case ROLE_HUB:      return "Hub";
    case ROLE_FAILED:   return "Failed";
    default:            return "Not input";
    }
}

/* §2.4 reason table (the texts of the kernel's F2/F8 log lines). */
static const char *reason_text(unsigned r)
{
    switch (r) {
    case 0:  return "used";
    case 1:  return "not a HID interface (class != 3)";
    case 2:  return "HID without boot subclass: report protocol only, no report parser yet";
    case 3:  return "HID boot subclass, protocol neither keyboard nor mouse";
    case 4:  return "HID boot interface without an interrupt-IN endpoint";
    case 5:  return "second HID boot interface: one interface per device";
    case 16: return "no HID interface - not an input device";
    case 17: return "configuration descriptor read failed";
    case 18: return "Configure Endpoint failed";
    case 19: return "SET_CONFIGURATION failed";
    case 20: return "SET_PROTOCOL(boot) failed";
    case 21: return "Address Device failed";
    case 22: return "device descriptor read failed";
    default: return "unknown reason code";
    }
}

/* xHCI 1.2 Table 6-90 (the transfer-relevant codes). */
static const char *cc_name(unsigned cc)
{
    switch (cc) {
    case 0:  return "none";
    case 1:  return "Success";
    case 2:  return "Data Buffer Error";
    case 3:  return "Babble Detected Error";
    case 4:  return "USB Transaction Error";
    case 5:  return "TRB Error";
    case 6:  return "Stall Error";
    case 13: return "Short Packet";
    default: return "see xHCI 1.2 Table 6-90";
    }
}

/* xHCI 1.2 Table 7-13. */
static void speed_text(unsigned id)
{
    switch (id) {
    case 1: S("full speed (12 Mbit/s)"); break;
    case 2: S("low speed (1.5 Mbit/s)"); break;
    case 3: S("high speed (480 Mbit/s)"); break;
    case 4: S("SuperSpeed (5 Gbit/s)"); break;
    default: S("unknown"); break;
    }
    S("  [xHCI PORTSC speed id "); U(id); C(']');
}

/* Usage page names (FreeMiNT hid.h U_PAGE_*). */
static const char *page_name(unsigned p)
{
    switch (p) {
    case 0x01: return "Generic Desktop";
    case 0x02: return "Simulation";
    case 0x03: return "VR";
    case 0x04: return "Sport";
    case 0x05: return "Game";
    case 0x06: return "Generic Device";
    case 0x07: return "Keyboard/Keypad";
    case 0x08: return "LEDs";
    case 0x09: return "Button";
    case 0x0A: return "Ordinal";
    case 0x0B: return "Telephony";
    case 0x0C: return "Consumer";
    case 0x0D: return "Digitizer";
    default:   return p >= 0xFF00 ? "Vendor-defined" : 0;
    }
}

/* Usage names (FreeMiNT hid.h USAGE_*; Rx..Rz from hidparse.c's page-1
 * sequence X, Y, Z, Rx, Ry, Rz, Slider, Dial, Wheel, Hat switch). */
static void usage_name(unsigned page, unsigned u)
{
    const char *n = 0;
    if (page == 0x01) {
        switch (u) {
        case 0x01: n = "Pointer"; break;   case 0x02: n = "Mouse"; break;
        case 0x04: n = "Joystick"; break;  case 0x05: n = "Game Pad"; break;
        case 0x06: n = "Keyboard"; break;  case 0x07: n = "Keypad"; break;
        case 0x30: n = "X"; break;         case 0x31: n = "Y"; break;
        case 0x32: n = "Z"; break;         case 0x33: n = "Rx"; break;
        case 0x34: n = "Ry"; break;        case 0x35: n = "Rz"; break;
        case 0x36: n = "Slider"; break;    case 0x37: n = "Dial"; break;
        case 0x38: n = "Wheel"; break;     case 0x39: n = "Hat Switch"; break;
        }
    } else if (page == 0x09) {
        S("Button "); U(u);
        return;
    } else if (page == 0x0C) {
        if (u == 0x238) n = "AC Pan";
    } else if (page == 0x0D) {
        switch (u) {
        case 0x30: n = "Tip Pressure"; break;  case 0x32: n = "In Range"; break;
        case 0x42: n = "Tip Switch"; break;    case 0x44: n = "Barrel Switch"; break;
        case 0x45: n = "Eraser"; break;
        }
    }
    if (n) S(n);
    else { S("0x"); X(u, u > 0xFF ? 4 : 2); }
}

/* ------------------------------------------------------------------------ */
/* Path of a device: root port -> hub V:P port n -> ...                     */
/* ------------------------------------------------------------------------ */

/* Hubs from the root down to the device's parent. */
static int hub_chain(unsigned i, unsigned *chain)
{
    unsigned c[MAX_DEV];
    int n = 0, k;
    unsigned p = entry(i)[3];
    while (p != 0xFF && p < n_dev && n < MAX_DEV) {
        c[n++] = p;
        p = entry(p)[3];
    }
    for (k = 0; k < n; k++)
        chain[k] = c[n - 1 - k];
    return n;
}

/* "root port 1 via hub 2109:3431 port 2" (screen) or with " -> " (file). */
static void path_text(unsigned i, const char *via)
{
    unsigned chain[MAX_DEV];
    int n = hub_chain(i, chain), k;
    unsigned route = rd32(entry(i) + 4);
    S("root port "); U(entry(i)[2]);
    for (k = 0; k < n; k++) {
        const unsigned char *h = entry(chain[k]);
        S(via); S("hub "); X(rd16(h + 8), 4); C(':'); X(rd16(h + 10), 4);
        S(" port "); U((route >> (4 * k)) & 15);
    }
}

/* ------------------------------------------------------------------------ */
/* DEVICE answer access                                                     */
/* ------------------------------------------------------------------------ */

struct sec { unsigned type, iface, flags, len; const unsigned char *p; };

/* The k-th section of device i's DEVICE answer; 0 when there is none. */
static int section(unsigned i, unsigned k, struct sec *s)
{
    const unsigned char *a = dans[i];
    unsigned used, n, o, j;
    if (!a || drc[i] != E_OK)
        return 0;
    used = rd32(a + 24);
    n = rd16(a + HDR_LEN + 32);
    if (k >= n)
        return 0;
    o = HDR_LEN + 36;
    for (j = 0; j <= k; j++) {
        if (o + 8 > used)
            return 0;
        s->type = rd16(a + o);
        s->iface = a[o + 2];
        s->flags = a[o + 3];
        s->len = rd32(a + o + 4);
        s->p = a + o + 8;
        if (o + 8 + s->len > used)
            return 0;
        o += 8 + ((s->len + 3) & ~3u);
    }
    return 1;
}

static int find_section(unsigned i, unsigned type, unsigned iface, struct sec *s)
{
    unsigned k;
    for (k = 0; section(i, k, s); k++)
        if (s->type == type && (type != SEC_REPORT || s->iface == iface))
            return 1;
    return 0;
}

/* ------------------------------------------------------------------------ */
/* Output file (buffered Fwrite)                                            */
/* ------------------------------------------------------------------------ */

static unsigned long f_size;
static long f_err;

#ifdef USBSNAP_TEST_SINK
/* Supplied by a measurement wrapper. */
static long out_prepare(void);
static long out_exists(const char *path);
static long out_create(const char *path);
static long out_write(long h, const unsigned char *b, long n);
static long out_close(long h);
#else
/* C:\USB\ present as a folder, or created (§4.3, §7 decision 4). */
static long out_prepare(void)
{
    long a = gemdos3(G_FATTRIB, (NLONG)"C:\\USB", 0, 0);
    if (a >= 0)
        return (a & FA_SUBDIR) ? E_OK : 1;   /* 1 = a file of that name */
    if (a != EFILNF)
        return a;
    return gemdos3(G_DCREATE, (NLONG)"C:\\USB", 0, 0);
}
/* 1 = exists, 0 = free, < 0 = error (§4.3 collision rule). */
static long out_exists(const char *path)
{
    long h = gemdos3(G_FOPEN, (NLONG)path, 0, 0);
    if (h >= 0) {
        gemdos3(G_FCLOSE, h, 0, 0);
        return 1;
    }
    return h == EFILNF ? 0 : h;
}
static long out_create(const char *path) { return gemdos3(G_FCREATE, (NLONG)path, 0, 0); }
static long out_write(long h, const unsigned char *b, long n) { return gemdos3(G_FWRITE, h, n, (NLONG)b); }
static long out_close(long h) { return gemdos3(G_FCLOSE, h, 0, 0); }
#endif

static unsigned char fbuf[4096];
static unsigned fpos;
static long fh;

static void f_flush(void)
{
    long r;
    if (fpos == 0)
        return;
    if (f_err == 0) {
        r = out_write(fh, fbuf, fpos);
        if (r < 0)
            f_err = r;
        else if ((unsigned long)r != fpos)
            f_err = -10;   /* EWRITF: short write (disk full) */
        else
            f_size += fpos;
    }
    fpos = 0;
}

#define FILE_COLS 78   /* §4.4: <= 78 columns */

static void f_bytes(const char *p, int n)
{
    int k;
    for (k = 0; k < n; k++) {
        if (fpos == sizeof fbuf)
            f_flush();
        fbuf[fpos++] = (unsigned char)p[k];
    }
}

/* The current line into the file, CRLF. A line longer than FILE_COLS is
 * broken at its last space before the limit; the rest continues indented by
 * four more columns than the line itself (under the value of a
 * "Label : value" line). */
static void F(void)
{
    int start = 0, indent = 0, first = 1;
    while (indent < ln && indent < 20 && lb[indent] == ' ')
        indent++;
    if (ln > 16 && lb[14] == ':' && lb[15] == ' ')
        indent = 12;   /* "Label         : value" continues under the value */
    while (ln - start + (first ? 0 : indent + 4) > FILE_COLS) {
        int room = FILE_COLS - (first ? 0 : indent + 4), cut = start + room;
        while (cut > start + room / 2 && lb[cut] != ' ')
            cut--;
        if (lb[cut] != ' ')
            cut = start + room;
        if (!first) { int k; for (k = 0; k < indent + 4; k++) f_bytes(" ", 1); }
        f_bytes(lb + start, cut - start);
        f_bytes("\r\n", 2);
        start = cut;
        while (start < ln && lb[start] == ' ')
            start++;
        first = 0;
    }
    if (!first) { int k; for (k = 0; k < indent + 4; k++) f_bytes(" ", 1); }
    f_bytes(lb + start, ln - start);
    f_bytes("\r\n", 2);
    ln = 0;
}
static void FS(const char *s) { ln = 0; S(s); F(); }

/* ------------------------------------------------------------------------ */
/* File sections                                                            */
/* ------------------------------------------------------------------------ */

static void hexdump(const unsigned char *p, unsigned n)
{
    unsigned o, k;
    for (o = 0; o < n; o += 16) {
        ln = 0;
        X(o, 4); C(':');
        for (k = 0; k < 16; k++) {
            if (o + k < n) { C(' '); X(p[o + k], 2); }
            else S("   ");
        }
        S("  |");
        for (k = 0; k < 16 && o + k < n; k++)
            C(p[o + k] >= 0x20 && p[o + k] < 0x7F ? (char)p[o + k] : '.');
        C('|');
        F();
    }
}

static void sec_state(unsigned flags)
{
    if (flags & SF_FAILED) S(" - read FAILED");
    if (flags & SF_SKIPPED) S(" - not read (SKIPPED)");
    if (flags & SF_TRUNCATED) S(" - TRUNCATED by the kernel limit");
}

static void string_line(unsigned i, const char *label, unsigned type)
{
    struct sec s;
    unsigned k;
    ln = 0; S(label);
    if (!find_section(i, type, 0, &s)) {
        S("-   (the device names no string)");
    } else if (s.flags & (SF_FAILED | SF_SKIPPED)) {
        S("-  ");
        sec_state(s.flags);
    } else {
        C('"');
        for (k = 0; k < s.len; k++) C((char)s.p[k]);
        C('"');
        if (s.flags & SF_TRUNCATED) S("  (cut at 31 characters)");
    }
    F();
}

static void bcd(unsigned v)   /* 0x0200 -> 2.00 */
{
    if (v >> 12) X(v >> 12, 1);
    X((v >> 8) & 15, 1); C('.'); X((v >> 4) & 15, 1); X(v & 15, 1);
}

static void file_head(unsigned i)
{
    const unsigned char *e = entry(i);
    struct sec s;

    FS("=== 0. FILE ===");
    FS("Format        : NewTOS USBSNAP text report v1");
    FS("Program       : " PROGRAM_VERSION);
    ln = 0; S("Kernel        : "); S(kversion); F();
    ln = 0; S("Snapshot API  : Ssystem 2022, layout v1, generation "); U(generation); F();
    ln = 0; S("Recorded      : "); U((t_end - t_start) / 200); C('.'); U((t_end - t_start) / 20 % 10);
    S(" s, pointer muted: "); S(muted_ok ? "yes" : "NO"); S(", ring "); U(RING_V1);
    S(", reports listed <= "); U(REC_KEEP); F();
    ln = 0; S("Device        : "); U((unsigned)list_no[i]); S(" of "); U((unsigned)n_listed);
    S(" (index "); U(i); C(')'); F();
    FS("Timestamps    : time of collection by the host, NOT the time the device sent");
    FS("                the report (resolution = how often the kernel polls USB).");
    FS("");

    FS("=== 1. IDENTITY ===");
    ln = 0; S("VID:PID       : "); X(rd16(e + 8), 4); C(':'); X(rd16(e + 10), 4); F();
    string_line(i, "Manufacturer  : ", SEC_MANUF);
    string_line(i, "Product       : ", SEC_PRODUCT);
    if (find_section(i, SEC_DEVICE, 0, &s) && s.len >= 18) {
        ln = 0; S("bcdUSB "); bcd(rd16(s.p + 2)); S("  class "); U(s.p[4]); S("  subclass ");
        U(s.p[5]); S("  protocol "); U(s.p[6]); S("  EP0 max "); U(s.p[7]);
        S("  configurations "); U(s.p[17]); F();
    } else {
        ln = 0; S("Device descriptor: not available");
        if (find_section(i, SEC_DEVICE, 0, &s)) sec_state(s.flags);
        F();
    }
    FS("");

    FS("=== 2. POSITION ===");
    ln = 0; S("Controller    : xHCI, "); U(root_ports); S(" root ports");
    if (enum_skipped) { S(", "); U(enum_skipped); S(" device(s) not enumerated"); }
    F();
    ln = 0; S("Path          : "); path_text(i, " -> "); F();
    ln = 0; S("Route string  : 0x"); X(rd32(e + 4), 8);
    S(e[3] == 0xFF ? "  (on a root port)" : "  (behind a hub, parent index ");
    if (e[3] != 0xFF) { U(e[3]); C(')'); }
    F();
    ln = 0; S("Speed         : "); speed_text(e[13]); F();
    ln = 0; S("xHCI slot     : "); U(e[1]); F();
    FS("");
}

static const char *proto_name(unsigned cls, unsigned sub, unsigned proto)
{
    if (cls != 3 || sub != 1) return 0;
    return proto == 1 ? "keyboard" : proto == 2 ? "mouse" : proto == 0 ? "none" : 0;
}

static void file_decision(unsigned i)
{
    const unsigned char *e = entry(i);
    struct sec t;
    unsigned r, n, role = e[14], fl = e[16];

    FS("=== 3. DRIVER DECISION ===");
    ln = 0; S("Role          : ");
    switch (role) {
    case ROLE_KEYBOARD: S("KEYBOARD (HID boot protocol, decoded -> IKBD)"); break;
    case ROLE_MOUSE:    S("MOUSE (HID boot protocol, decoded -> IKBD)"); break;
    case ROLE_OBSERVED: S("OBSERVED ONLY (HID report protocol: recorded, not decoded)"); break;
    case ROLE_FAILED:   S("FAILED (enumeration failed, device unusable)"); break;
    case ROLE_HUB:      S("HUB"); break;
    default:            S("NONE (not an input device / not configured)"); break;
    }
    F();
    ln = 0; S("Role codes    : 0 none / 1 keyboard / 2 mouse / 3 observed-only / 4 hub /"); F();
    ln = 0; S("                5 failed; this device: "); U(role); F();
    ln = 0; S("Reason        : ");
    if (e[15] == 0) S("-");
    else { U(e[15]); S(" = "); S(reason_text(e[15])); }
    F();
    ln = 0; S("Flags         : 0x"); X(fl, 2); S(" = ");
    S(fl & 1 ? "device descriptor ok" : "no device descriptor");
    S(fl & 2 ? ", configuration ok" : ", no configuration");
    if (fl & 4) S(", configuration CUT");
    F();
    ln = 0; S("                "); S(fl & 8 ? "SET_CONFIGURATION done" : "not configured (no SET_CONFIGURATION)");
    F();
    ln = 0; S("HID interfaces: "); U(e[17]);
    S("   recorded interface: ");
    if (e[18] == 0xFF) S("none");
    else { U(e[18]); S(", endpoint 0x"); X(e[19], 2); }
    F();
    FS("Interfaces:");
    if (!find_section(i, SEC_IFACES, 0, &t)) {
        FS("  (no interface table: configuration not read)");
    } else {
        n = t.len / 16;
        for (r = 0; r < n; r++) {
            const unsigned char *w = t.p + 16 * r;
            const char *pn = proto_name(w[2], w[3], w[4]);
            ln = 0; S("  if "); U(w[0]); S(" alt "); U(w[1]); S("  class "); U(w[2]);
            S(" sub "); U(w[3]); S(" proto "); U(w[4]);
            if (w[6]) {
                S("  int-IN 0x"); X(w[6], 2); S(" max "); U(rd16(w + 8) & 0x7FF);
                S(" interval "); U(w[7]);
            } else {
                S("  no int-IN endpoint");
            }
            F();
            ln = 0; S("      ");
            if (w[5] == 0) {
                if (role == ROLE_KEYBOARD || role == ROLE_MOUSE) {
                    S("USED ("); S(pn ? pn : "boot"); S(", decoded)");
                } else {
                    S("USED (recorded, not decoded)");
                }
            } else {
                S("not used: "); S(reason_text(w[5]));
            }
            if (w[14]) S("  [recorded]");
            F();
            if (w[2] == 3) {
                ln = 0; S("      report descriptor declared "); U(rd16(w + 10));
                S(" bytes, kept "); U(rd16(w + 12)); F();
            }
        }
        if (t.flags & SF_TRUNCATED)
            FS("  (more interfaces than the kernel table holds - list cut)");
    }
    ln = 0; S("Interrupt transfer errors: "); U(rd16(e + 28));
    if (rd16(e + 28)) { S(" (last completion code "); U(e[30]); S(" = "); S(cc_name(e[30])); C(')'); }
    F();
    FS("");
}

static unsigned declared_len(unsigned i, unsigned iface)
{
    struct sec t;
    unsigned r;
    if (!find_section(i, SEC_IFACES, 0, &t)) return 0;
    for (r = 0; r < t.len / 16; r++)
        if (t.p[16 * r] == iface && t.p[16 * r + 1] == 0 && t.p[16 * r + 2] == 3)
            return rd16(t.p + 16 * r + 10);
    return 0;
}

static void file_raw(unsigned i)
{
    struct sec s;
    unsigned k;
    FS("=== 4. RAW DESCRIPTORS (hex, offset: 16 bytes) ===");
    for (k = 0; section(i, k, &s); k++) {
        if (s.type == SEC_DEVICE) {
            ln = 0; S("Device descriptor, "); U(s.len); S(" bytes"); sec_state(s.flags); F();
        } else if (s.type == SEC_CONFIG) {
            ln = 0; S("Configuration descriptor, "); U(s.len); S(" bytes");
            if (s.len >= 4) { S(" (wTotalLength "); U(rd16(s.p + 2)); C(')'); }
            sec_state(s.flags); F();
        } else if (s.type == SEC_REPORT) {
            ln = 0; S("HID report descriptor, interface "); U(s.iface); S(", "); U(s.len);
            S(" bytes (declared "); U(declared_len(i, s.iface)); C(')'); sec_state(s.flags); F();
        } else {
            continue;   /* interface table: section 3; strings: section 1 */
        }
        hexdump(s.p, s.len);
    }
    FS("");
}

/* Configuration descriptor stream (HID 1.11 Appendix E field names). */
static void decode_config(const unsigned char *c, unsigned n)
{
    unsigned o = 0;
    while (o + 2 <= n) {
        unsigned bl = c[o], ty = c[o + 1];
        const unsigned char *d = c + o;
        if (bl < 2) {
            ln = 0; S("  bLength "); U(bl); S(" at offset "); U(o); S(": malformed, stop"); F();
            return;
        }
        if (o + bl > n) {
            ln = 0; S("  descriptor type 0x"); X(ty, 2); S(" at offset "); U(o);
            S(" runs past the end ("); U(bl); S(" bytes)"); F();
            return;
        }
        ln = 0;
        if (ty == 2 && bl >= 9) {
            S("Configuration: wTotalLength "); U(rd16(d + 2)); S(" bNumInterfaces "); U(d[4]);
            S(" bConfigurationValue "); U(d[5]); F();
            ln = 0; S("  iConfiguration "); U(d[6]); S(" MaxPower "); U(d[8]); S(" ("); U(d[8] * 2u);
            S(" mA) bmAttributes 0x"); X(d[7], 2);
            if (d[7] & 0x80) S(" bus-powered");
            if (d[7] & 0x40) S(" self-powered");
            if (d[7] & 0x20) S(" remote-wakeup");
        } else if (ty == 4 && bl >= 9) {
            const char *pn = proto_name(d[5], d[6], d[7]);
            S("  Interface "); U(d[2]); S(" alt "); U(d[3]); S(": bNumEndpoints "); U(d[4]);
            S(" iInterface "); U(d[8]); F();
            ln = 0; S("    class "); U(d[5]); if (d[5] == 3) S(" (HID)"); else if (d[5] == 9) S(" (hub)");
            S(" sub "); U(d[6]); if (d[5] == 3) S(d[6] == 1 ? " (boot)" : d[6] == 0 ? " (none)" : "");
            S(" protocol "); U(d[7]); if (pn) { S(" ("); S(pn); C(')'); }
        } else if (ty == 0x21 && bl >= 6) {
            unsigned k, nd = d[5];
            S("    HID: bcdHID "); bcd(rd16(d + 2)); S(" bCountryCode "); U(d[4]);
            S(" bNumDescriptors "); U(nd);
            for (k = 0; k < nd && 6 + 3 * k + 3 <= bl; k++) {
                F(); ln = 0;
                S("      class descriptor type 0x"); X(d[6 + 3 * k], 2);
                if (d[6 + 3 * k] == 0x22) S(" (report)");
                else if (d[6 + 3 * k] == 0x23) S(" (physical)");
                S(" length "); U(rd16(d + 7 + 3 * k));
            }
        } else if (ty == 5 && bl >= 7) {
            static const char *const tt[4] = { "control", "isochronous", "bulk", "interrupt" };
            unsigned mp = rd16(d + 4);
            S("    Endpoint 0x"); X(d[2], 2); S(d[2] & 0x80 ? ": IN, " : ": OUT, ");
            S(tt[d[3] & 3]); S(", wMaxPacketSize 0x"); X(mp, 4); S(" ("); U(mp & 0x7FF);
            if (mp >> 11) { S(", bits 12:11 = "); U((mp >> 11) & 3); }
            S("), bInterval "); U(d[6]);
        } else {
            S("  descriptor type 0x"); X(ty, 2); S(", "); U(bl); S(" bytes (not decoded)");
        }
        F();
        o += bl;
    }
}

static void decode_device(const unsigned char *d)
{
    ln = 0; S("Device: bLength "); U(d[0]); S(" bDescriptorType "); U(d[1]);
    S(" bcdUSB 0x"); X(rd16(d + 2), 4); S(" bDeviceClass "); U(d[4]); F();
    ln = 0; S("  bDeviceSubClass "); U(d[5]); S(" bDeviceProtocol "); U(d[6]);
    S(" bMaxPacketSize0 "); U(d[7]); F();
    ln = 0; S("  idVendor 0x"); X(rd16(d + 8), 4); S(" idProduct 0x"); X(rd16(d + 10), 4);
    S(" bcdDevice 0x"); X(rd16(d + 12), 4); F();
    ln = 0; S("  iManufacturer "); U(d[14]); S(" iProduct "); U(d[15]);
    S(" iSerialNumber "); U(d[16]); S(" bNumConfigurations "); U(d[17]); F();
}

/* HID 1.11 §6.2.2: short items (bSize 0/1/2/4, bType Main/Global/Local),
 * long items skipped by their length. Names only; no field layout. */
static void decode_report(const unsigned char *r, unsigned n)
{
    static const char *const coll[7] = { "Physical", "Application", "Logical", "Report",
                                         "Named Array", "Usage Switch", "Usage Modifier" };
    static const char *const gname[12] = { "Usage Page", "Logical Minimum", "Logical Maximum",
        "Physical Minimum", "Physical Maximum", "Unit Exponent", "Unit", "Report Size",
        "Report ID", "Report Count", "Push", "Pop" };
    static const char *const lname[11] = { "Usage", "Usage Minimum", "Usage Maximum",
        "Designator Index", "Designator Minimum", "Designator Maximum", 0, "String Index",
        "String Minimum", "String Maximum", "Delimiter" };
    unsigned o = 0, depth = 0, page = 0, stack[8], sp = 0;
    unsigned items = 0, n_in = 0, n_out = 0, n_feat = 0, n_coll = 0, unbalanced = 0;
    unsigned ids[16], n_ids = 0, k;

    while (o < n) {
        unsigned p = r[o], size, type, tag, len, uv = 0;
        long sv = 0;
        items++;
        ln = 0; S("  ");
        if (p == 0xFE) {   /* long item, §6.2.2.3 */
            unsigned dsz = o + 1 < n ? r[o + 1] : 0;
            len = 3 + dsz;
            for (k = 0; k < 3 && o + k < n; k++) { X(r[o + k], 2); C(' '); }
            PAD(18);
            S("Long item (tag 0x"); if (o + 2 < n) X(r[o + 2], 2); S(", "); U(dsz);
            S(" data bytes) - skipped");
            if (o + len > n) S(" [runs past the end]");
            F();
            o += len;
            continue;
        }
        size = p & 3; size = size == 3 ? 4 : size;
        type = (p >> 2) & 3;
        tag = p >> 4;
        len = 1 + size;
        if (o + len > n) {
            X(p, 2); PAD(18); S("item runs past the end of the descriptor - stop"); F();
            break;
        }
        for (k = 0; k < size; k++) uv |= (unsigned)r[o + 1 + k] << (8 * k);
        sv = size == 1 ? (long)(signed char)uv : size == 2 ? (long)(short)uv
           : size == 4 ? (long)(int)uv : 0;
        for (k = 0; k < len; k++) { X(r[o + k], 2); C(' '); }
        PAD(18);
        if (type == 0 && tag == 0xC && depth) depth--;
        for (k = 0; k < depth && k < 8; k++) S("  ");
        if (type == 0) {   /* Main, §6.2.2.4 */
            if (tag == 0x8 || tag == 0x9 || tag == 0xB) {
                S(tag == 0x8 ? "Input" : tag == 0x9 ? "Output" : "Feature");
                S(uv & 1 ? " (Const" : " (Data"); S(uv & 2 ? ",Var" : ",Array");
                S(uv & 4 ? ",Rel" : ",Abs");
                if (uv & 8) S(",Wrap");
                if (uv & 0x10) S(",NonLinear");
                if (uv & 0x20) S(",NoPreferred");
                if (uv & 0x40) S(",NullState");
                if ((uv & 0x80) && tag != 0x8) S(",Volatile");
                if (uv & 0x100) S(",Buffered");
                C(')');
                if (tag == 0x8) n_in++; else if (tag == 0x9) n_out++; else n_feat++;
            } else if (tag == 0xA) {
                S("Collection (");
                if (uv < 7) S(coll[uv]); else if (uv >= 0x80) S("Vendor-defined"); else S("Reserved");
                C(')');
                n_coll++;
                depth++;
            } else if (tag == 0xC) {
                S("End Collection");
                if (n_coll == 0) unbalanced = 1;
            } else {
                S("Main item, reserved tag 0x"); X(tag, 1);
            }
        } else if (type == 1) {   /* Global, §6.2.2.7 */
            if (tag < 12) S(gname[tag]); else { S("Global item, reserved tag 0x"); X(tag, 1); }
            if (tag == 0) {
                const char *pn = page_name(uv);
                page = uv;
                S(" ("); if (pn) S(pn); else { S("0x"); X(uv, 4); } C(')');
            } else if (tag >= 1 && tag <= 5) {
                /* signed as HID 1.11 reads it; a negative value also raw,
                 * since some devices mean it unsigned (25 ff = 255) */
                S(" ("); D(sv);
                if (sv < 0) { S(", raw 0x"); X(uv, size * 2); }
                C(')');
            } else if (tag == 6) {
                S(" (0x"); X(uv, size * 2 ? size * 2 : 1); C(')');
            } else if (tag >= 7 && tag <= 9) {
                S(" ("); U(uv); C(')');
                if (tag == 8 && n_ids < 16) {
                    for (k = 0; k < n_ids && ids[k] != uv; k++) {}
                    if (k == n_ids) ids[n_ids++] = uv;
                }
            } else if (tag == 0xA) {
                if (sp < 8) stack[sp++] = page;
            } else if (tag == 0xB) {
                if (sp) page = stack[--sp];
            }
        } else if (type == 2) {   /* Local, §6.2.2.8 */
            if (tag < 11 && lname[tag]) S(lname[tag]);
            else { S("Local item, reserved tag 0x"); X(tag, 1); }
            if (tag <= 2) {
                unsigned pg = size == 4 ? uv >> 16 : page, us = size == 4 ? uv & 0xFFFF : uv;
                S(" (");
                if (size == 4) { X(pg, 4); C(':'); }
                usage_name(pg, us);
                C(')');
            } else if (tag == 0xA) {
                S(uv == 1 ? " (open set)" : uv == 0 ? " (close set)" : " (?)");
            } else if (tag < 11) {
                S(" ("); U(uv); C(')');
            }
        } else {
            S("Reserved item type");
        }
        F();
        o += len;
    }
    ln = 0; S("  Items "); U(items); S(": Input "); U(n_in); S(", Output "); U(n_out);
    S(", Feature "); U(n_feat); S(", Collections "); U(n_coll);
    if (depth || unbalanced) S(" (collections NOT balanced)");
    F();
    ln = 0;
    if (n_ids == 0) {
        S("  Report ID: none (reports carry no ID byte)");
    } else {
        S("  Report ID: used, IDs");
        for (k = 0; k < n_ids; k++) { C(' '); U(ids[k]); }
        F(); ln = 0;
        S("  -> every report starts with a 1-byte report ID (HID 1.11 6.2.2.7)");
    }
    F();
}

static void file_decoded(unsigned i)
{
    struct sec s;
    unsigned k;
    FS("=== 5. DECODED DESCRIPTORS ===");
    if (find_section(i, SEC_DEVICE, 0, &s) && s.len >= 18)
        decode_device(s.p);
    else
        FS("Device: no device descriptor");
    if (find_section(i, SEC_CONFIG, 0, &s) && s.len)
        decode_config(s.p, s.len);
    else
        FS("Configuration: not read");
    for (k = 0; section(i, k, &s); k++) {
        if (s.type != SEC_REPORT)
            continue;
        ln = 0; S("Report descriptor items (interface "); U(s.iface); S("):");
        if (!s.len) { S(" none"); sec_state(s.flags); }
        F();
        if (s.len)
            decode_report(s.p, s.len);
        if (s.len && (s.flags & SF_TRUNCATED))
            FS("  (descriptor cut by the kernel limit - items above are incomplete)");
    }
    FS("");
}

static void file_reports(unsigned i)
{
    const unsigned char *e = entry(i);
    struct stat *t = &st[i];
    unsigned role = e[14], k, b;
    FS("=== 6. REPORTS ===");
    if (e[18] == 0xFF) {
        FS("not recorded (not an input device / not configured)");
        FS("");
        return;
    }
    ln = 0; S("Total "); U(t->total);
    if (t->total) { S(" (seq "); U(t->first_seq); S(".."); U(t->last_seq); C(')'); }
    S(", listed "); U(t->kept); S(", lost "); U(t->lost);
    S(", interrupt errors "); U(rd16(e + 28)); F();
    if (t->total > t->kept) {
        ln = 0; S("First "); U(t->kept); S(" of "); U(t->total); S(" listed; all: length ");
        U(t->min_len); S(".."); U(t->max_len); S(", last at "); SECS((long)(t->last_ts - t_start));
        S(" s"); F();
    }
    if (role == ROLE_OBSERVED)
        FS("report-protocol device, not decoded by NewTOS yet (bytes only)");
    FS("Flags: M = collected while the pointer was muted, C = report may be cut");
    ln = 0; S("   seq"); PAD(11); S("t(s)  len fl  bytes");
    if (role == ROLE_MOUSE) { PAD(49); S("decoded (boot mouse)"); }
    if (role == ROLE_KEYBOARD) { PAD(49); S("decoded (boot keyboard)"); }
    F();
    for (k = 0; k < t->kept; k++) {
        const unsigned char *r = recs[i][k];
        unsigned len = rd16(r + 8), fl = r[10], cc = r[11];
        const unsigned char *d = r + 16;
        if (len > 64) len = 64;
        ln = 0;
        UR(rd32(r), 6); C(' ');
        SECSR((long)(int)(rd32(r + 4) - t_start), 8);
        C(' '); UR(len, 3); C(' ');
        C(fl & REC_MUTED ? 'M' : '-'); C(fl & REC_CAPPED ? 'C' : '-'); S("  ");
        for (b = 0; b < len && b < 8; b++) { X(d[b], 2); C(' '); }
        if (len == 0) S("(empty)");
        if (role == ROLE_MOUSE && len >= 3) {
            PAD(49);
            C(d[0] & 1 ? 'L' : '-'); C(d[0] & 2 ? 'R' : '-'); C(d[0] & 4 ? 'M' : '-');
            S(" dx"); DR((signed char)d[1], 5); S(" dy"); DR((signed char)d[2], 5);
            if (len > 3) { S(" wh"); DR((signed char)d[3], 4); }
        } else if (role == ROLE_KEYBOARD && len >= 3) {
            PAD(49);
            S("mod "); X(d[0], 2); S(" keys");
            for (b = 2; b < len && b < 8; b++) if (d[b]) { C(' '); X(d[b], 2); }
        }
        if (cc != 1 && cc != 13) { S(" cc "); U(cc); }
        F();
        for (b = 8; b < len; b += 16) {
            unsigned m;
            ln = 0; PAD(24);
            for (m = b; m < len && m < b + 16; m++) { X(d[m], 2); C(' '); }
            F();
        }
    }
    FS("");
}

/* ------------------------------------------------------------------------ */
/* Recording                                                                */
/* ------------------------------------------------------------------------ */

static void poll_device(unsigned i)
{
    struct stat *t = &st[i];
    int round;
    for (round = 0; round < 64; round++) {
        long rc;
        unsigned count, rl, lost, k;
        request(rbuf, OP_REPORTS, i, t->next);
        rc = snap_call(rbuf, sizeof rbuf);
        if (rc != E_OK || (rd16(rbuf + 10) & HF_TRUNCATED))
            return;
        lost = rd32(rbuf + HDR_LEN + 8);
        count = rd16(rbuf + HDR_LEN + 12);
        rl = rd16(rbuf + HDR_LEN + 14);
        if (rl < 16 || rl > REC_LEN_V1)
            return;
        t->rec_len = rl;
        t->lost += lost;
        for (k = 0; k < count; k++) {
            const unsigned char *r = rbuf + HDR_LEN + 16 + k * rl;
            unsigned seq = rd32(r), len = rd16(r + 8), m;
            if (t->total == 0) { t->first_seq = seq; t->min_len = len; t->max_len = len; }
            if (len < t->min_len) t->min_len = len;
            if (len > t->max_len) t->max_len = len;
            t->last_seq = seq;
            t->last_ts = rd32(r + 4);
            t->total++;
            if (t->kept < REC_KEEP) {
                for (m = 0; m < rl; m++) recs[i][t->kept][m] = r[m];
                for (; m < REC_LEN_V1; m++) recs[i][t->kept][m] = 0;
                t->kept++;
            }
            t->next = seq + 1;
        }
        if (!(rd16(rbuf + 10) & HF_MORE))
            return;
    }
}

static void poll_all(void)
{
    unsigned i;
    for (i = 0; i < n_dev; i++)
        if (list_no[i] && entry(i)[18] != 0xFF)
            poll_device(i);
}

static long mute(unsigned ticks, unsigned *now)
{
    long rc;
    request(cbuf, OP_MUTE, 0, ticks);
    rc = snap_call(cbuf, sizeof cbuf);
    if (now) *now = rd32(cbuf + 28);
    return rc;
}

static void device_label(unsigned i)   /* "  1  046d:c077  Mouse       " */
{
    const unsigned char *e = entry(i);
    S("  "); U((unsigned)list_no[i]); PAD(5);
    X(rd16(e + 8), 4); C(':'); X(rd16(e + 10), 4); S("  "); S(role_short(e[14]));
    PAD(29);
}

static void draw_counters(unsigned left_s)
{
    unsigned i;
    int row = 1;
    esc_at(0, 0);
    ln = 0; S(T_RECORDING); U(left_s); S(" s"); lb[ln] = 0; Cconws(lb);
    for (i = 0; i < n_dev; i++) {
        if (!list_no[i])
            continue;
        esc_at(row++, 0);
        ln = 0; device_label(i);
        if (entry(i)[18] == 0xFF) {
            S(T_NOREC);
        } else {
            S("reports: "); U(st[i].total);
            if (st[i].total == 0) S(T_NOTHING);
        }
        lb[ln] = 0; Cconws(lb);
    }
    esc_at(row + 1, 0);
    Cconws("Move the mouse, press EVERY button, turn the wheel; press some keys.");
    esc_at(row + 2, 0);
}

static void record(void)
{
    unsigned i, now, last_poll, last_draw;
    long rc;

    /* Start every ring read at the report that comes next (LIST: ring_next). */
    request(lbuf, OP_LIST, 0, 0);
    if (snap_call(lbuf, sizeof lbuf) == E_OK)
        for (i = 0; i < n_dev; i++)
            st[i].next = rd32(entry(i) + 24);

    rc = mute(MUTE_TICKS, &now);
    muted_ok = rc == E_OK;
    t_start = now;
    esc_clear();
    draw_counters(REC_TICKS / 200);
    last_poll = last_draw = now;
    for (;;) {
        if (gemdos3(G_CCONIS, 0, 0, 0))   /* pumps USB; a key (unmuted case) is dropped */
            gemdos3(G_CNECIN, 0, 0, 0);
        now = now_ticks();
        if (now - t_start >= REC_TICKS)
            break;
        if (now - last_poll >= POLL_TICKS) {
            poll_all();
            last_poll = now;
        }
        if (now - last_draw >= 200) {
            draw_counters((REC_TICKS - (now - t_start) + 199) / 200);
            last_draw = now;
        }
    }
    mute(0, &now);
    poll_all();
    t_end = now;
    draw_counters(0);
    con("");
}

/* ------------------------------------------------------------------------ */
/* Files                                                                    */
/* ------------------------------------------------------------------------ */

static char path[40];

/* Base name per §4.3: VVVVPPPP, or NOID + root port + index for no IDs. */
static void base_name(unsigned i)
{
    const unsigned char *e = entry(i);
    S("C:\\USB\\");
    if (e[14] == ROLE_FAILED || (rd16(e + 8) == 0 && rd16(e + 10) == 0)) {
        S("NOID");
        C((char)('0' + e[2] / 10 % 10)); C((char)('0' + e[2] % 10));
        C((char)('0' + i / 10 % 10)); C((char)('0' + i % 10));
    } else {
        XU(rd16(e + 8), 4); XU(rd16(e + 10), 4);
    }
}

/* Pick a free name: .TXT, then .T01 .. .T99 (never overwrite). 0 = none. */
static int pick_name(unsigned i)
{
    int ext, k;
    for (ext = 0; ext <= 99; ext++) {
        long ex;
        ln = 0;
        base_name(i);
        if (ext == 0) S(".TXT");
        else { S(".T"); C((char)('0' + ext / 10)); C((char)('0' + ext % 10)); }
        for (k = 0; k < ln; k++) path[k] = lb[k];
        path[ln] = 0;
        ln = 0;
        ex = out_exists(path);
        if (ex == 0)
            return 1;
        if (ex < 0) {
            ln = 0; S("  "); S(path); S("   cannot check the name: "); ERR(ex); con_line();
            return 0;
        }
    }
    ln = 0; S("  "); base_name(i); S(".T99 exists already - file skipped"); con_line();
    return 0;
}

static int write_file(unsigned i)
{
    long rc;
    if (!pick_name(i))
        return 0;
    fh = out_create(path);
    if (fh < 0) {
        ln = 0; S("  "); S(path); S("   cannot create: "); ERR(fh); con_line();
        return 0;
    }
    f_size = 0; f_err = 0; fpos = 0;
    file_head(i);
    file_decision(i);
    file_raw(i);
    file_decoded(i);
    file_reports(i);
    FS("=== END ===");
    f_flush();
    rc = out_close(fh);
    if (f_err == 0 && rc < 0)
        f_err = rc;
    ln = 0; S("  "); S(path); PAD(25);
    if (f_err) {
        S("WRITE FAILED: "); ERR(f_err);
        con_line();
        return 0;
    }
    S("("); U(f_size / 1024); C('.'); U(f_size * 10 / 1024 % 10); S(" KB)");
    con_line();
    return 1;
}

/* ------------------------------------------------------------------------ */
/* main                                                                     */
/* ------------------------------------------------------------------------ */

static void fail(const char *a, const char *b)
{
    con("");
    con(a);
    if (b) con(b);
    con("");
    wait_any_key();
    Pterm(1);
}

int main(void)
{
    long rc;
    unsigned i, now, t0;
    int written = 0;

    kversion[0] = 0;
    if (gemdos3(G_SSYSTEM, S_NEWTOS_KVERSION, (NLONG)kversion, sizeof kversion) != E_OK
        || kversion[0] == 0) {
        const char *u = "unknown (Ssystem 2019 not answered)";
        for (i = 0; u[i]; i++) kversion[i] = u[i];
        kversion[i] = 0;
    }

    esc_clear();
    ln = 0; S(T_TITLE); PAD(43); S(kversion); con_line();
    con(T_INTRO);
    con("");

    request(lbuf, OP_LIST, 0, 0);
    rc = snap_call(lbuf, sizeof lbuf);
    if (rc == EINVFN)
        fail(T_TOO_OLD, 0);
    if (rc == ERROR)
        fail(T_NO_CTRL, 0);
    if (rc == EINVAL && lbuf[0] == 'U' && rd16(lbuf + 4) != 1) {
        ln = 0; S("The kernel speaks USB snapshot layout v"); U(rd16(lbuf + 4));
        S("; this program reads v1."); con_line();
        fail("Please use the USBSNAP.NCP that came with this kernel.", 0);
    }
    if (rc != E_OK || (rd16(lbuf + 10) & HF_TRUNCATED)) {
        ln = 0; S("Unexpected answer from the kernel: "); ERR(rc);
        S(" flags 0x"); X(rd16(lbuf + 10), 4); con_line();
        fail("Nothing to record.", 0);
    }
    generation = rd32(lbuf + HDR_LEN);
    n_dev = rd16(lbuf + HDR_LEN + 4);
    entry_len = rd16(lbuf + HDR_LEN + 6);
    root_ports = lbuf[HDR_LEN + 8];
    enum_skipped = rd16(lbuf + HDR_LEN + 10);
    if (n_dev > MAX_DEV) n_dev = MAX_DEV;
    if (entry_len < 32) n_dev = 0;
    for (i = 0; i < n_dev; i++)
        list_no[i] = e_role(i) == ROLE_HUB ? 0 : ++n_listed;
    if (n_listed == 0)
        fail(T_NO_DEV1, n_dev ? T_NO_DEV2 "\r\n" T_ONLY_HUBS : T_NO_DEV2);

    ln = 0; S("Found "); U((unsigned)n_listed); S(n_listed == 1 ? " USB device:" : " USB devices:");
    con_line();
    for (i = 0; i < n_dev; i++) {
        if (!list_no[i]) continue;
        ln = 0; device_label(i); path_text(i, " via "); con_line();
    }
    for (i = 0; i < n_dev; i++)
        if (e_role(i) == ROLE_HUB) { con(T_HUBNOTE); break; }

    /* DEVICE answers now (they carry the strings shown below). */
    for (i = 0; i < n_dev; i++) {
        if (!list_no[i]) continue;
        dans[i] = dbuf[i];
        request(dans[i], OP_DEVICE, i, 0);
        drc[i] = snap_call(dans[i], sizeof dbuf[i]);
        if (drc[i] == E_OK && (rd16(dans[i] + 10) & HF_TRUNCATED)) {
            unsigned need = rd32(dans[i] + 20);
            long blk = gemdos3(G_MALLOC, need, 0, 0);
            if (blk > 0) {
                dans[i] = (unsigned char *)blk;
                request(dans[i], OP_DEVICE, i, 0);
                drc[i] = snap_call(dans[i], need);
            }
            if (drc[i] == E_OK && (rd16(dans[i] + 10) & HF_TRUNCATED))
                drc[i] = ERROR;   /* still too big: file says "not available" */
        }
    }
    con("");
    for (i = 0; i < n_dev; i++) {
        struct sec m, p;
        int hm, hp;
        unsigned k;
        if (!list_no[i]) continue;
        hm = find_section(i, SEC_MANUF, 0, &m) && m.len;
        hp = find_section(i, SEC_PRODUCT, 0, &p) && p.len;
        if (!hm && !hp) continue;
        ln = 0; S("  "); U((unsigned)list_no[i]); PAD(5);
        if (hm) { C('"'); for (k = 0; k < m.len; k++) C((char)m.p[k]); S("\" "); }
        if (hp) { C('"'); for (k = 0; k < p.len; k++) C((char)p.p[k]); C('"'); }
        con_line();
    }

    rc = out_prepare();
    if (rc == 1)
        fail(T_ISFILE, 0);
    if (rc < 0) {
        ln = 0; S(T_FOLDER); ERR(rc); con_line();
        fail(T_NOFILES, 0);
    }

    con("");
    con(T_REC_LEN);
    con(T_REC_MOUSE);
    con(T_REC_KBD);
    con(T_REC_PTR);
    con(T_START);
    t0 = now_ticks();
    for (;;) {
        if (gemdos3(G_CCONIS, 0, 0, 0)) {
            long k = gemdos3(G_CNECIN, 0, 0, 0);
            if ((k & 0xFF) == '\r' || (k & 0xFF) == '\n')
                break;
        }
        now = now_ticks();
        if (now - t0 >= START_TICKS)
            break;
    }

    record();

    con(T_DONE);
    for (i = 0; i < n_dev; i++)
        if (list_no[i])
            written += write_file(i);
    con("");
    con(written ? T_COPY : T_NOFILES);
    wait_any_key();
    Pterm0();
}

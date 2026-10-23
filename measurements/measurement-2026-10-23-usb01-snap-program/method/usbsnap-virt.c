/*
 * usbsnap-virt.c -- MEASUREMENT build of USBSNAP for qemu-virt (not a product
 * program). qemu-virt has the xHCI controller but no drive, so the files
 * cannot be written there. This wrapper compiles the unchanged
 * sdk/examples/usbsnap.c with its two test seams:
 *   USBSNAP_TEST_SINK      the out_* functions below print each file to the
 *                          console (serial) between FILE BEGIN / FILE END
 *                          markers; the collision rule works on the names
 *                          created in this run;
 *   USBSNAP_TEST_SNAPCALL  snap_call() calls the real Ssystem(2022) and dumps
 *                          every non-empty LIST / DEVICE / REPORTS / MUTE
 *                          answer as hex (USBSNAP-RAW / USBSNAP-HEX lines),
 *                          the input of mkreplay.py for the raspi4b replay.
 * Built by method/build.sh, embedded as NEWDESK.NAP in a scratch worktree's
 * kernel (the program the shell starts at boot), as the kernel part's probes.
 */
#define USBSNAP_TEST_SINK 1
#define USBSNAP_TEST_SNAPCALL 1
#include "../../../sdk/examples/usbsnap.c"

static void hexline(const unsigned char *p, unsigned n)
{
    static const char hx[] = "0123456789abcdef";
    char t[200];
    unsigned o, k, j;
    for (o = 0; o < n; o += 64) {
        j = 0;
        t[j++] = 'U'; t[j++] = 'S'; t[j++] = 'B'; t[j++] = 'S'; t[j++] = 'N'; t[j++] = 'A';
        t[j++] = 'P'; t[j++] = '-'; t[j++] = 'H'; t[j++] = 'E'; t[j++] = 'X'; t[j++] = ' ';
        for (k = o; k < n && k < o + 64; k++) { t[j++] = hx[p[k] >> 4]; t[j++] = hx[p[k] & 15]; }
        t[j++] = '\r'; t[j++] = '\n'; t[j] = 0;
        Cconws(t);
    }
}

static long snap_call(unsigned char *buf, long len)
{
    unsigned op = rd16(buf + 8), idx = rd32(buf + 12), arg = rd32(buf + 16);
    long rc = gemdos3(G_SSYSTEM, S_NEWTOS_USBSNAP, (NLONG)buf, len);
    unsigned used = buf[0] == 'U' ? rd32(buf + 24) : 0;
    int dump = rc == 0 && used > HDR_LEN;
    if (op == OP_REPORTS && dump && rd16(buf + HDR_LEN + 12) == 0)
        dump = 0;
    if (dump || op == OP_MUTE) {
        ln = 0; S("USBSNAP-RAW op="); U(op); S(" idx="); U(idx); S(" arg="); U(arg);
        S(" rc="); D(rc); S(" used="); U(used); S(" now="); U(buf[0] == 'U' ? rd32(buf + 28) : 0);
        con_line();
        if (dump)
            hexline(buf, used);
    }
    return rc;
}

static char made[16][40];
static int n_made;

static long out_prepare(void)
{
    con("USBSNAP-TEST: no drive on qemu-virt - files go to the console");
    return E_OK;
}
static long out_exists(const char *path)
{
    int k, m;
    for (k = 0; k < n_made; k++) {
        for (m = 0; path[m] && path[m] == made[k][m]; m++) {}
        if (path[m] == 0 && made[k][m] == 0)
            return 1;
    }
    return 0;
}
static long out_create(const char *path)
{
    int m;
    if (n_made < 16) {
        for (m = 0; path[m] && m < 39; m++) made[n_made][m] = path[m];
        made[n_made++][m] = 0;
    }
    Cconws("=====USBSNAP FILE BEGIN ");
    Cconws(path);
    Cconws("=====\r\n");
    return 100;
}
static long out_write(long h, const unsigned char *b, long n)
{
    char t[257];
    long o, k;
    (void)h;
    for (o = 0; o < n; o += 256) {
        for (k = 0; k < 256 && o + k < n; k++) t[k] = (char)b[o + k];
        t[k] = 0;
        Cconws(t);
    }
    return n;
}
static long out_close(long h)
{
    (void)h;
    Cconws("=====USBSNAP FILE END=====\r\n");
    return E_OK;
}

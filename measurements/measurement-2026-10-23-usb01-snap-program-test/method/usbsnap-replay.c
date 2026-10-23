/*
 * usbsnap-replay.c -- MEASUREMENT build of USBSNAP for QEMU raspi4b (not a product program). raspi4b has the SD card
 * but no USB controller, qemu-virt has USB but no drive; this build exercises the file half (C:\USB\, names, the
 * .T01.. collision rule, Fcreate/Fwrite, section content) on a real FAT card. It compiles the unchanged
 * sdk/examples/usbsnap.c with the USBSNAP_TEST_SNAPCALL seam: snap_call() answers from the raw kernel answers a
 * qemu-virt run recorded (replay.h, made by mkreplay.py from serial/virt-extra.log). The clock is one tick per call;
 * recorded reports become visible at their original offset from the recording start.
 * Build: method/build.sh (sdk/build.sh method/usbsnap-replay.c USBSNAPR.NCP).
 */
#define USBSNAP_TEST_SNAPCALL 1
#include "../../../sdk/examples/usbsnap.c"
#include "replay.h"

static unsigned clk = 20000, fake_t0, recording;

static void head(unsigned char *buf, unsigned flags, unsigned total, unsigned used)
{
    wr16(buf + 4, 1); wr16(buf + 6, HDR_LEN); wr16(buf + 10, flags);
    wr32(buf + 20, total); wr32(buf + 24, used); wr32(buf + 28, clk);
}

static long whole(unsigned char *buf, long len, const unsigned char *body, unsigned n)
{
    unsigned k;
    if ((unsigned long)len < HDR_LEN + n) { head(buf, HF_TRUNCATED, HDR_LEN + n, HDR_LEN); return E_OK; }
    for (k = 0; k < n; k++) buf[HDR_LEN + k] = body[k];
    head(buf, 0, HDR_LEN + n, HDR_LEN + n);
    return E_OK;
}

static long snap_call(unsigned char *buf, long len)
{
    unsigned op = rd16(buf + 8), idx = rd32(buf + 12), arg = rd32(buf + 16);
    clk++;
    if (op == OP_LIST) return whole(buf, len, R_LIST, R_LIST_LEN);
    if (op == OP_DEVICE) {
        if (idx >= R_NDEV || !R_DEVLEN[idx]) return EINVAL;
        return whole(buf, len, R_DEV[idx], R_DEVLEN[idx]);
    }
    if (op == OP_MUTE) {
        unsigned char b8[8] = { 0 };
        if (arg) { fake_t0 = clk; recording = 1; wr32(b8, clk + arg); }
        return whole(buf, len, b8, 8);
    }
    if (op == OP_REPORTS) {
        unsigned k, count = 0, oldest = 0xFFFFFFFF, next = arg, room = (unsigned)len - HDR_LEN - 16, more = 0;
        for (k = 0; k < R_NREC; k++) {
            unsigned seq = rd32(R_REC[k]), ts = rd32(R_REC[k] + 4) - R_T0 + fake_t0, m;
            if (R_RECIDX[k] != idx || !recording) continue;
            if ((int)(clk - ts) < 0) continue;          /* not "arrived" yet */
            if (seq < oldest) oldest = seq;
            if (seq + 1 > next) next = seq + 1;
            if (seq < arg) continue;
            if ((count + 1) * REC_LEN_V1 > room) { more = 1; continue; }
            for (m = 0; m < REC_LEN_V1; m++) buf[HDR_LEN + 16 + count * REC_LEN_V1 + m] = R_REC[k][m];
            wr32(buf + HDR_LEN + 16 + count * REC_LEN_V1 + 4, ts);
            count++;
        }
        if (oldest == 0xFFFFFFFF) oldest = arg;
        wr32(buf + HDR_LEN, oldest); wr32(buf + HDR_LEN + 4, next); wr32(buf + HDR_LEN + 8, 0);
        wr16(buf + HDR_LEN + 12, count); wr16(buf + HDR_LEN + 14, REC_LEN_V1);
        head(buf, more ? HF_MORE : 0, HDR_LEN + 16 + count * REC_LEN_V1, HDR_LEN + 16 + count * REC_LEN_V1);
        return E_OK;
    }
    return EINVAL;
}

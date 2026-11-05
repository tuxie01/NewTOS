/*
 * dsfeed.c -- EmuTOS-side scripted mouse feeder for desktop scenes (TASK/V20 acceptance).
 * A resident AUTO program: reads DSFEED.INJ (`<ticks> 0xF8|buttons dx dy`, IKBD relative mouse
 * packets, bit 1 = left) and DSFEED.SHT (`<ticks> <name>`), writes the address of its shot counter
 * to DSFEED.ADR, waits until the host has created DSFEED.GO (the host sets a Hatari tracking
 * breakpoint on the counter meanwhile), then hooks etv_timer (Setexc 0x100) and stays resident
 * (Ptermres).  The hook feeds each packet through Kbdvbase()->mousevec at its tick (200 Hz _hz_200
 * relative to the hook's installation) and increments dsf_shot_n at each shot tick; the breakpoint
 * on that long takes the screenshot.  Same mechanism as the etv_timer feeder of slprobe.c /
 * mgprobe.c, without a window; the desktop of EmuTOS then runs undisturbed.
 * Built by tosdump/probes/run-deskedge.sh.  Test code only.
 */
#include <osbind.h>
#include <mint/basepage.h>
#include <stdint.h>

#define NSTEP 1024
#define NSHOT 64

static uint32_t st_tick[NSTEP];
static unsigned char st_pkt[NSTEP][3];
static int nstep;
static uint32_t sh_tick[NSHOT];
static int nshot;

volatile uint32_t inj_t0 __asm__("dsf_inj_t0");
volatile int inj_next __asm__("dsf_inj_next");
volatile int inj_armed __asm__("dsf_inj_armed");
void *volatile inj_old __asm__("dsf_inj_old");
volatile int shot_next __asm__("dsf_shot_next");
volatile uint32_t shot_n __asm__("dsf_shot_n");
static long *kvecs;
static unsigned char inj_buf[3];

void inj_timer(void) __asm__("dsf_inj_timer");
void inj_mousevec(void *fn, unsigned char *pkt) __asm__("dsf_inj_mousevec");
void inj_step(void) __asm__("dsf_inj_step");

__asm__(
"	.text\n"
"	.even\n"
"dsf_inj_timer:\n"
"	movem.l	d0-d7/a0-a6,-(sp)\n"
"	jsr	dsf_inj_step\n"
"	movem.l	(sp)+,d0-d7/a0-a6\n"
"	move.l	dsf_inj_old,-(sp)\n"
"	rts\n"
"dsf_inj_mousevec:\n"
"	move.l	4(sp),a1\n"
"	move.l	8(sp),a0\n"
"	movem.l	d2-d7/a2-a6,-(sp)\n"
"	jsr	(a1)\n"
"	movem.l	(sp)+,d2-d7/a2-a6\n"
"	rts\n"
);

void inj_step(void)
{
	uint32_t now;

	if (!inj_armed)
		return;
	now = *(volatile uint32_t *)0x4BAUL;
	while (inj_next < nstep && now - inj_t0 >= st_tick[inj_next]) {
		inj_buf[0] = st_pkt[inj_next][0];
		inj_buf[1] = st_pkt[inj_next][1];
		inj_buf[2] = st_pkt[inj_next][2];
		inj_mousevec((void *)kvecs[4], inj_buf);
		inj_next++;
	}
	if (shot_next < nshot && now - inj_t0 >= sh_tick[shot_next]) {
		shot_next++;
		shot_n = (uint32_t)shot_next;	/* one change per timer call at most */
	}
}

static long arm(void)
{
	inj_next = 0;
	shot_next = 0;
	inj_t0 = *(volatile uint32_t *)0x4BAUL;
	inj_armed = 1;
	return (long)inj_t0;
}

static char buf[32768];

static int num(const char **p, long *v)
{
	const char *s = *p;
	long n = 0;
	int base = 10, any = 0;

	while (*s == ' ' || *s == '\t')
		s++;
	if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) {
		base = 16;
		s += 2;
	}
	for (;; s++) {
		int d;

		if (*s >= '0' && *s <= '9')
			d = *s - '0';
		else if (base == 16 && *s >= 'a' && *s <= 'f')
			d = *s - 'a' + 10;
		else if (base == 16 && *s >= 'A' && *s <= 'F')
			d = *s - 'A' + 10;
		else
			break;
		n = n * base + d;
		any = 1;
	}
	*p = s;
	*v = n;
	return any;
}

/* want = 4 (packets) or 1 (shot ticks; the rest of the line is the name, ignored) */
static int load(const char *name, int want)
{
	long fh = Fopen(name, 0), n;
	const char *p, *end;
	int cnt = 0;

	if (fh < 0)
		return -1;
	n = Fread((short)fh, (long)sizeof(buf) - 1, buf);
	Fclose((short)fh);
	if (n <= 0 || n >= (long)sizeof(buf) - 1)
		return -2;
	buf[n] = 0;
	p = buf;
	end = buf + n;
	while (p < end) {
		const char *eol = p;
		long v[4];
		int k = 0;

		while (eol < end && *eol != '\n')
			eol++;
		while (k < want && p < eol && *p != '#' && *p != '\r' && num(&p, &v[k]))
			k++;
		if (k == want) {
			if (want == 4 && nstep < NSTEP) {
				st_tick[nstep] = (uint32_t)v[0];
				st_pkt[nstep][0] = (unsigned char)v[1];
				st_pkt[nstep][1] = (unsigned char)v[2];
				st_pkt[nstep][2] = (unsigned char)v[3];
				nstep++;
			} else if (want == 1 && nshot < NSHOT) {
				sh_tick[nshot++] = (uint32_t)v[0];
			} else {
				return -4;
			}
			cnt++;
		} else if (k != 0) {
			return -3;
		}
		p = eol + 1;
	}
	return cnt;
}

static void hex8(char *d, unsigned long v)
{
	int i;

	for (i = 7; i >= 0; i--) {
		d[i] = "0123456789abcdef"[v & 15];
		v >>= 4;
	}
	d[8] = '\n';
}

int main(void)
{
	int r = load("DSFEED.INJ", 4), s = load("DSFEED.SHT", 1), i;
	long fh;
	char a[9];

	if (r < 0 || s < 0)
		return 1;
	hex8(a, (unsigned long)(uintptr_t)&shot_n);
	fh = Fcreate("DSFEED.ADR", 0);
	if (fh >= 0) {
		Fwrite((short)fh, 9L, a);
		Fclose((short)fh);
	}
	for (i = 0; i < 2000000; i++) {		/* wait for the host's breakpoint (DSFEED.GO) */
		long g = Fopen("DSFEED.GO", 0);

		if (g >= 0) {
			Fclose((short)g);
			break;
		}
	}
	kvecs = (long *)Kbdvbase();
	inj_old = (void *)Setexc(0x100, -1L);
	Setexc(0x100, (long)inj_timer);
	Supexec(arm);
	Ptermres(0x100L + _base->p_tlen + _base->p_dlen + _base->p_blen, 0);
	return 0;
}

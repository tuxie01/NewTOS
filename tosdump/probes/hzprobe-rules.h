/*
 * hzprobe-rules.h -- the pure part of tosdump/probes/hzprobe.c (TASK/PITEST01):
 * Tgettime arithmetic, the step bookkeeping of the _hz_200 reads and the PASS
 * rule. No TOS call in here, so tosdump/tests/test_hzprobe.c checks it on the
 * host. The rules themselves are stated in the probe's header.
 */
#ifndef HZPROBE_RULES_H
#define HZPROBE_RULES_H

/* Seconds of the day from a GEMDOS time word (hour<<11 | min<<5 | sec/2). */
static inline unsigned long hz_secs_of(unsigned long t)
{
	return ((t >> 11) & 31) * 3600UL + ((t >> 5) & 63) * 60UL + (t & 31) * 2UL;
}

/* Seconds from a to b on the clock of the day (one midnight wrap at most). */
static inline unsigned long hz_since(unsigned long a, unsigned long b)
{
	return b >= a ? b - a : b + 86400UL - a;
}

/* The reads of one state. Histogram: [0] step 1, [1] 2, [2] 3..10, [3] >10. */
struct hz_side {
	unsigned long reads, steps, hist[4], maxstep;
};

/* The state shared by both sides: the previous value and the backward steps. */
struct hz_track {
	unsigned long last, backwards;
};

/*
 * One read v of the 32-bit counter, by side s. A step is the difference to
 * the previous read (mod 2^32); 0 is no step, a difference with bit 31 set a
 * backward step (counted, not a step).
 */
static inline void hz_observe(struct hz_track *t, struct hz_side *s, unsigned long v)
{
	unsigned long d = (v - t->last) & 0xFFFFFFFFUL;

	s->reads++;
	if (d == 0)
		return;
	t->last = v;
	if (d & 0x80000000UL) {
		t->backwards++;
		return;
	}
	s->steps++;
	s->hist[d == 1 ? 0 : d == 2 ? 1 : d <= 10 ? 2 : 3]++;
	if (d > s->maxstep)
		s->maxstep = d;
}

#define HZ_FAIL_P1 0x01 /* a step > 2: the counter lagged by more than one tick */
#define HZ_FAIL_P2 0x02 /* total ticks off by more than 1 % of 200/s */
#define HZ_FAIL_P3 0x04 /* a backward step */
#define HZ_FAIL_P4 0x08 /* no step seen in user or in super state */
#define HZ_FAIL_P5 0x10 /* a bad Super token or a bad Super(1L) inquiry */

/* The PASS rule: 0 = PASS, else the HZ_FAIL_* bits of the failed rules. */
static inline unsigned hz_verdict(const struct hz_side *user, const struct hz_side *sup,
				  const struct hz_track *t, unsigned long total,
				  unsigned long expected, unsigned long bad_super)
{
	unsigned f = 0;
	unsigned long maxstep = user->maxstep > sup->maxstep ? user->maxstep : sup->maxstep;
	unsigned long diff = total > expected ? total - expected : expected - total;

	if (maxstep > 2)
		f |= HZ_FAIL_P1;
	if (expected == 0 || diff > expected / 100)
		f |= HZ_FAIL_P2;
	if (t->backwards)
		f |= HZ_FAIL_P3;
	if (!user->steps || !sup->steps)
		f |= HZ_FAIL_P4;
	if (bad_super)
		f |= HZ_FAIL_P5;
	return f;
}

#endif /* HZPROBE_RULES_H */

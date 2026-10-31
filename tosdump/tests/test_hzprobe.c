/*
 * test_hzprobe.c -- host test of tosdump/probes/hzprobe-rules.h, the pure
 * part of the _hz_200 user-state probe (TASK/PITEST01): Tgettime arithmetic,
 * step bookkeeping and the PASS rule (P1 step <= 2, P2 total within 1 %,
 * P3 no backward step, P4 steps in both states, P5 Super tokens).
 */
#include "tdtest.h"
#include "../probes/hzprobe-rules.h"

static unsigned long tw(unsigned h, unsigned m, unsigned s)
{
	return ((unsigned long)h << 11) | ((unsigned long)m << 5) | (s / 2);
}

static void test_time(void)
{
	TD_CHECK_EQ(hz_secs_of(tw(0, 0, 0)), 0);
	TD_CHECK_EQ(hz_secs_of(tw(0, 0, 16)), 16);
	TD_CHECK_EQ(hz_secs_of(tw(13, 45, 58)), 13 * 3600 + 45 * 60 + 58);
	TD_CHECK_EQ(hz_secs_of(tw(23, 59, 58)), 86398);
	TD_CHECK_EQ(hz_since(16, 46), 30);
	TD_CHECK_EQ(hz_since(86398, 0), 2);  /* midnight */
	TD_CHECK_EQ(hz_since(86390, 20), 30);
}

static void test_observe(void)
{
	struct hz_side u = { 0 };
	struct hz_track t = { 100, 0 };

	hz_observe(&t, &u, 100);           /* no step */
	hz_observe(&t, &u, 101);           /* 1 */
	hz_observe(&t, &u, 103);           /* 2 */
	hz_observe(&t, &u, 108);           /* 5 */
	hz_observe(&t, &u, 120);           /* 12 */
	TD_CHECK_EQ(u.reads, 5);
	TD_CHECK_EQ(u.steps, 4);
	TD_CHECK_EQ(u.hist[0], 1);
	TD_CHECK_EQ(u.hist[1], 1);
	TD_CHECK_EQ(u.hist[2], 1);
	TD_CHECK_EQ(u.hist[3], 1);
	TD_CHECK_EQ(u.maxstep, 12);
	TD_CHECK_EQ(t.backwards, 0);

	hz_observe(&t, &u, 119);           /* backwards: counted, not a step */
	TD_CHECK_EQ(t.backwards, 1);
	TD_CHECK_EQ(u.steps, 4);
	TD_CHECK_EQ(t.last, 119);

	/* the 32-bit wrap is a step of 1, not a backward step */
	t.last = 0xFFFFFFFFUL;
	hz_observe(&t, &u, 0);
	TD_CHECK_EQ(t.backwards, 1);
	TD_CHECK_EQ(u.steps, 5);
	TD_CHECK_EQ(u.hist[0], 2);
}

static void test_verdict(void)
{
	struct hz_side u = { 10, 5, { 5, 0, 0, 0 }, 1 };
	struct hz_side s = { 10, 5, { 4, 1, 0, 0 }, 2 };
	struct hz_track t = { 0, 0 };

	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6000, 6000, 0), 0);
	/* 1 % of 6000 = 60: 60 off passes, 61 off fails P2 */
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6060, 6000, 0), 0);
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 5940, 6000, 0), 0);
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6061, 6000, 0), HZ_FAIL_P2);
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 5939, 6000, 0), HZ_FAIL_P2);
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 0, 0, 0), HZ_FAIL_P2);
	/* a step of 3 = the counter lagged two ticks: P1 */
	s.maxstep = 3;
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6000, 6000, 0), HZ_FAIL_P1);
	s.maxstep = 2;
	t.backwards = 1;
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6000, 6000, 0), HZ_FAIL_P3);
	t.backwards = 0;
	u.steps = 0;
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6000, 6000, 0), HZ_FAIL_P4);
	u.steps = 5;
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 6000, 6000, 2), HZ_FAIL_P5);
	u.maxstep = 400;
	TD_CHECK_EQ(hz_verdict(&u, &s, &t, 0, 6000, 1), HZ_FAIL_P1 | HZ_FAIL_P2 | HZ_FAIL_P5);
}

int main(void)
{
	test_time();
	test_observe();
	test_verdict();
	return TD_TEST_RESULT();
}

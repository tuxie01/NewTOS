/* veccli.c - PORT12 Q5: a later process with no relation to vectsr.c but the common parent:
 * can it read, write and execute what the resident program kept? Log after each step. */
#include "vec-common.h"
int main(void)
{
    long v = 0, r, c;
    struct shared *s;
    strcpy(lf_name, "C:\\VECCLI.LOG");
    r = SETX(34, -1L);
    lg("Setexc(34,-1) -> %#lx; page-0 cell $88 = %#x\n", r, *(volatile unsigned int *)0x88L);
    c = Ssystem(S_GETCOOKIE, COOKIE_XGTS, (long)&v);
    lg("Ssystem(S_GETCOOKIE,XGTS) -> %ld value %#lx\n", c, v);
    if (!v) return 1;
    s = (struct shared *)v;
    lg("read shared struct magic %#lx\n", s->magic);
    lg("read TSR Malloc block: '%s'\n", s->priv);
    s->priv[0] = 'P'; lg("wrote TSR Malloc block: '%s'\n", s->priv);
    lg("read TSR Mxalloc(global) block: '%s'\n", s->glob);
    s->glob[0] = 'G'; lg("wrote TSR Mxalloc(global) block: '%s'\n", s->glob);
    lg("TSR counter before call: %ld\n", *s->count);
    r = s->fn();
    lg("called TSR code at %p -> %#lx, TSR counter now %ld\n", (void *)s->fn, r, *s->count);
    return 0;
}

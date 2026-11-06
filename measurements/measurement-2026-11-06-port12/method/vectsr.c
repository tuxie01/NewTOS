/* vectsr.c - PORT12 Q5: stays resident like XGDOS.PRG (Ptermres), with vector 34 set, a cookie, private
 * and global blocks. Its code and data are then used by others (veccli.c, vecp.c). */
#include "vec-common.h"
static struct shared sh;
static volatile long tsr_count;
static long tsr_fn(void) { tsr_count++; return 0x1234; }
int main(void)
{
    long r, old, keep;
    strcpy(lf_name, "C:\\VECTSR.LOG");
    old = SETX(34, -1L);
    lg("base %p lowtpa %#x hitpa %#x tbase %#x bbase %#x blen %#x\n", (void *)_base, _base->p_lowtpa, _base->p_hitpa, _base->p_tbase, _base->p_bbase, _base->p_blen);
    sh.magic = COOKIE_XGTS; sh.fn = tsr_fn; sh.count = &tsr_count;
    sh.priv = (char *)Malloc(8192);
    sh.glob = (char *)Mxalloc(8192, 0x20 | 3);
    strcpy(sh.priv, "priv"); strcpy(sh.glob, "glob");
    lg("tsr_fn %p &tsr_count %p priv %p glob %p sp ~%p\n", (void *)tsr_fn, (void *)&tsr_count, sh.priv, sh.glob, (void *)&r);
    r = SETX(34, (long)tsr_fn);
    lg("Setexc(34,tsr_fn) -> %#lx (query before: %#lx), now %#lx\n", r, old, SETX(34, -1L));
    r = Ssystem(S_SETCOOKIE, COOKIE_XGTS, (long)&sh);
    lg("Ssystem(S_SETCOOKIE,XGTS,%p) -> %ld\n", (void *)&sh, r);
    keep = (long)_base->p_bbase + _base->p_blen - (long)_base->p_lowtpa;
    lg("Ptermres(%#lx,0)\n", keep);
    Ptermres(keep, 0);
    return 0;
}

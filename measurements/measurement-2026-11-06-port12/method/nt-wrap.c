/* PORT12: wrapper around an upstream host test on NewTOS. stdout goes to a file on the card;
 * the upstream test source is compiled unchanged with -Dmain=xg_test_main. */
#include <stdio.h>
int xg_test_main(void);
/* The tests write out_screen.pgm / out_memory.pgm, longer than 8.3: NewTOS' GEMDOS refuses such a name, EmuTOS'
 * truncates it (see RUNINFO, side finding). The test source stays unchanged: its fopen comes here (-Dfopen=nt_fopen). */
#include <string.h>
FILE *nt_fopen(const char *n, const char *m)
{
    if (!strcmp(n, "out_screen.pgm")) n = "OUTSCR.PGM";
    if (!strcmp(n, "out_memory.pgm")) n = "OUTMEM.PGM";
    return fopen(n, m);
}
#ifndef OUTFILE
#error OUTFILE
#endif
int main(void)
{
    int rc;
    if (!freopen(OUTFILE, "w", stdout)) return 99;
    rc = xg_test_main();
    fprintf(stdout, "rc=%d\n", rc);
    fclose(stdout);
    return rc;
}

/* vt52col.c -- tester scratch program (V67 run): VT52 colour, reverse video, scroll, clear and cursor on the console. Not part of the product. */
#include <newtos.h>
static void put(const char *s) { Cconws(s); }
static void col(int fg, int bg) { char b[8]; b[0]=27; b[1]='b'; b[2]=(char)fg; b[3]=27; b[4]='c'; b[5]=(char)bg; b[6]=0; Cconws(b); }
int main(void)
{
    int i;
    put("\033E\033e");                          /* clear, cursor on */
    for (i = 0; i < 16; i++) {                  /* 16 stripes: fg/bg pairs over all pens */
        char n[4];
        col(15 - i, i);
        n[0] = "0123456789ABCDEF"[i]; n[1] = 0;
        put("pen "); put(n); put(" text on pen background -------------------------------\r\n");
    }
    col(1, 0); put("\033p reverse video line \033q normal line\r\n");
    col(1, 0);
    for (i = 0; i < 16; i++) put("scroll line ......................................\r\n"); /* scrolls the top rows off */
    col(1, 0);
    put("\033Y\040\040" "after scroll: cursor here ->");
    for (;;) Cconin();
}

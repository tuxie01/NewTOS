/*
 * frprobe.c -- XBIOS01 retest: Fread on CON (GEMDOS handle 0, standard input =
 * CON) reads the keys of the IKBD IOREC ring. The run types a, b, Return
 * (method/keys-fread-ikbd.txt, injector armed launch:FRPROBE); the probe logs
 * Fread's count and bytes, then Bconstat(2), to C:\FRPROBE.LOG.
 */
#include <mint/osbind.h>
#include <stdio.h>
#include <string.h>

int main(void)
{
    char buf[32], line[160];
    long n, fh;
    int i, k;
    memset(buf, 0, sizeof buf);
    n = Fread(0, sizeof buf, buf);
    k = sprintf(line, "R01 Fread(0,%d) = %ld:", (int)sizeof buf, n);
    for (i = 0; i < n && i < (int)sizeof buf; i++)
        k += sprintf(line + k, " %02x", (unsigned char)buf[i]);
    k += sprintf(line + k, "\r\nR02 Bconstat(2) = %ld\r\n", (long)Bconstat(2));
    fh = Fcreate("C:\\FRPROBE.LOG", 0);
    if (fh >= 0) {
        Fwrite((short)fh, k, line);
        Fclose((short)fh);
    }
    return 0;
}

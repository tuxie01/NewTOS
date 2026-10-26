/*
 * emit_record.c -- byte-order test emitter (spec section 2.1).
 *
 * Writes a header plus one record of every layout, filled with a known
 * pattern, to the file named by argv[1].  The Makefile builds this twice --
 * once with TD_BIG_ENDIAN_TARGET=1 (the m68k configuration) and once with
 * TD_BIG_ENDIAN_TARGET=0 (the AArch64 configuration) -- and the two outputs
 * must be byte-identical.  A swap bug in either configuration silently
 * fabricates or hides API differences, so this runs before any dump is
 * trusted.
 */
#include <stdio.h>
#include <string.h>

#include "record.h"

static void fill16(int16_t *p, int n, uint16_t seed)
{
	int i;
	for (i = 0; i < n; i++)
		p[i] = (int16_t)(uint16_t)(seed + (uint16_t)(i * 0x0101u));
}

int main(int argc, char **argv)
{
	struct td_header h;
	struct td_vdi_arrays v;
	struct td_aes_arrays a;
	struct td_call_frame c;
	int16_t before[TD_VDI_CONTRL_N];
	uint8_t hdr[TD_HEADER_SIZE];
	uint8_t vrec[TD_VDI_RECORD_SIZE];
	uint8_t arec[TD_AES_RECORD_SIZE];
	uint8_t crec[TD_CALL_RECORD_SIZE];
	FILE *f;
	int i;

	if (argc != 2) {
		fprintf(stderr, "usage: emit_record OUT.BIN\n");
		return 2;
	}

	memset(&h, 0, sizeof h);
	h.version      = TD_FORMAT_VERSION;
	h.area         = TD_AREA_VDI;
	h.record_size  = TD_VDI_RECORD_SIZE;
	h.record_count = 1;
	memcpy(h.build_id, "beef", 4);
	h.case_crc     = 0x89ABCDEFuL;
	/* stated, not measured: the two configurations are built on the same
	 * host, and what is under test is that both WRITE the same bytes */
	h.ptr_width    = 4;
	h.word_order   = TD_WORD_ORDER_MSW_FIRST;
	td_header_write(hdr, &h);

	td_poison_vdi(&v);
	fill16(v.contrl, TD_VDI_CONTRL_N, 0x8001u);
	fill16(v.intin,  TD_VDI_INTIN_N,  0x1000u);
	fill16(v.ptsin,  TD_VDI_PTSIN_N,  0x2000u);
	fill16(v.intout, TD_VDI_INTOUT_N, 0xF000u);
	fill16(v.ptsout, TD_VDI_PTSOUT_N, 0x7FF0u);
	fill16(before,   TD_VDI_CONTRL_N, 0x00FFu);
	td_vdi_record_write(vrec, 0x1234u, 0x0006u, -1, &v, before);

	td_poison_aes(&a);
	fill16(a.contrl,  TD_AES_CONTRL_N, 0x0A0Bu);
	fill16(a.global,  TD_AES_GLOBAL_N, 0x0140u);
	fill16(a.int_in,  TD_AES_INTIN_N,  0xC0C1u);
	fill16(a.int_out, TD_AES_INTOUT_N, 0x0001u);
	for (i = 0; i < TD_AES_ADDRIN_N; i++)
		a.addr_in[i] = 0x01020304uL + (uint32_t)i;
	a.addr_out[0] = 0xFEDCBA98uL;
	td_aes_record_write(arec, 0x0002u, 0x000Au, 0x7FFFFFFFL, &a);

	td_poison_call(&c);
	for (i = 0; i < TD_CALL_ARGS_N; i++)
		c.args[i] = 0x11223344uL + (uint32_t)(i * 0x01010101uL);
	c.ret = -33;
	for (i = 0; i < TD_CALL_CAPTURE_LEN; i++)
		c.capture[i] = (uint8_t)(i * 7u + 3u);
	td_call_record_write(crec, 0x0003u, 0x003Du, &c);

	f = fopen(argv[1], "wb");
	if (!f) {
		perror(argv[1]);
		return 1;
	}
	if (fwrite(hdr,  1, sizeof hdr,  f) != sizeof hdr  ||
	    fwrite(vrec, 1, sizeof vrec, f) != sizeof vrec ||
	    fwrite(arec, 1, sizeof arec, f) != sizeof arec ||
	    fwrite(crec, 1, sizeof crec, f) != sizeof crec) {
		perror("fwrite");
		fclose(f);
		return 1;
	}
	fclose(f);
	return 0;
}

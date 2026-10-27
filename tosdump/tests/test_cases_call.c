/*
 * test_cases_call.c -- the call-table loader and its guard (task T7).
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "The call table, as built
 * for DUMPBIOS".  The expected values are written out here, not read from
 * core/: a parser that agreed with itself would prove nothing.
 */
#include "tdtest.h"
#include "cases.h"

static struct td_call_case_table t;

static int parse(const char *text, struct td_case_error *err)
{
	memset(err, 0, sizeof *err);
	return td_call_cases_parse(&t, TD_AREA_BIOS, text, strlen(text), err);
}

/* One line, wrapped in a comment so that the case id is not 1. */
static int parse1(const char *line, struct td_case_error *err)
{
	char buf[512];

	snprintf(buf, sizeof buf, "# one case\n%s\n", line);
	return parse(buf, err);
}

static void expect_refused(const char *line, int want_rc, const char *why)
{
	struct td_case_error err;
	int rc = parse1(line, &err);

	if (rc != want_rc) {
		printf("  FAIL %s: \"%s\" gave %d, want %d\n", why, line, rc,
		       want_rc);
		td_test_failures++;
		return;
	}
	if (err.line != 2) {
		printf("  FAIL %s: refusal names line %u, want 2\n", why, err.line);
		td_test_failures++;
	}
	if (!err.what || !*err.what) {
		printf("  FAIL %s: refusal carries no reason\n", why);
		td_test_failures++;
	}
}

static void expect_accepted(const char *line, const char *why)
{
	struct td_case_error err;
	int rc = parse1(line, &err);

	if (rc != 0 || t.count != 1) {
		printf("  FAIL %s: \"%s\" refused (%d, line %u: %s)\n", why, line,
		       rc, err.line, err.what ? err.what : "");
		td_test_failures++;
	}
}

static void test_well_formed(void)
{
	struct td_case_error err;
	const struct td_call_case *c;
	int rc;
	static const char *text =
		"# a comment\n"
		"\n"
		"14:0x04\t-\tEXACT.W\t-\tGetrez\n"
		"13:11\tw:-1\tEXACT\t-\tKbshift query\twith a tab in the comment\n"
		"14:0x2E\tw:0,w:0,w:50,b:1\tEXACT.W\t3:50\tNVMaccess read\n"
		"14:0x12\tb:0,l:0x123456,w:1,w:0\tIGNORE\t0:2@510\tProtobt checksum\n"
		"14:0x6A\tb:0,b:1\tIGNORE\t1:4.L\tDsp_Available y\n"
		"14:0x7D\t-\tEXACT.B\t-\tDsp_HStat\n"
		"14:0x02\t-\tPTR_RAM\t-\tPhysbase\n"
		"13:10\t-\tNONZERO\t-\tDrvmap\n"
		"14:0x55\tw:0,w:16,b:0\tIGNORE\t2:32.W\tEgetPalette\n";

	printf("a well formed call table\n");
	rc = parse(text, &err);
	TD_CHECK_EQ(rc, 0);
	if (rc != 0) {
		printf("    line %u: %s\n", err.line, err.what ? err.what : "");
		return;
	}
	TD_CHECK_EQ(t.count, 9);
	TD_CHECK_EQ(t.area, TD_AREA_BIOS);
	TD_CHECK_EQ(t.crc, td_crc32((const uint8_t *)text, strlen(text)));

	c = &t.cases[0];
	TD_CHECK_EQ(c->line, 3);
	TD_CHECK_EQ(c->trap, 14);
	TD_CHECK_EQ(c->opcode, 4);
	TD_CHECK_EQ(c->nargs, 0);
	TD_CHECK_EQ(c->retclass, TD_RET_EXACT);
	TD_CHECK_EQ(c->retwidth, TD_RET_WIDTH_WORD);
	TD_CHECK_EQ(c->cap_arg, -1);
	TD_CHECK(strcmp(c->comment, "Getrez") == 0);

	c = &t.cases[1];
	TD_CHECK_EQ(c->line, 4);
	TD_CHECK_EQ(c->trap, 13);
	TD_CHECK_EQ(c->opcode, 11);
	TD_CHECK_EQ(c->nargs, 1);
	TD_CHECK_EQ(c->arg[0].kind, TD_CALL_ARG_WORD);
	TD_CHECK_EQ(c->arg[0].value, -1);
	TD_CHECK_EQ(c->retclass, TD_RET_EXACT);
	TD_CHECK_EQ(c->retwidth, TD_RET_WIDTH_LONG);
	TD_CHECK(strcmp(c->comment, "Kbshift query\twith a tab in the comment") == 0);

	c = &t.cases[2];
	TD_CHECK_EQ(c->opcode, 0x2E);
	TD_CHECK_EQ(c->nargs, 4);
	TD_CHECK_EQ(c->arg[2].kind, TD_CALL_ARG_WORD);
	TD_CHECK_EQ(c->arg[2].value, 50);
	TD_CHECK_EQ(c->arg[3].kind, TD_CALL_ARG_BUF);
	TD_CHECK_EQ(c->arg[3].value, 1);
	TD_CHECK_EQ(c->cap_arg, 3);
	TD_CHECK_EQ(c->cap_unit, 1);
	TD_CHECK_EQ(c->cap_off, 0);
	TD_CHECK_EQ(c->cap_len, 50);

	c = &t.cases[3];
	TD_CHECK_EQ(c->arg[1].kind, TD_CALL_ARG_LONG);
	TD_CHECK_EQ(c->arg[1].value, 0x123456);
	TD_CHECK_EQ(c->retclass, TD_RET_IGNORE);
	TD_CHECK_EQ(c->cap_arg, 0);
	TD_CHECK_EQ(c->cap_off, 510);
	TD_CHECK_EQ(c->cap_len, 2);

	c = &t.cases[4];
	TD_CHECK_EQ(c->cap_arg, 1);
	TD_CHECK_EQ(c->cap_unit, 4);
	TD_CHECK_EQ(c->cap_len, 4);

	c = &t.cases[5];
	TD_CHECK_EQ(c->retwidth, TD_RET_WIDTH_BYTE);

	c = &t.cases[6];
	TD_CHECK_EQ(c->retclass, TD_RET_PTR_RAM);
	TD_CHECK_EQ(c->retwidth, TD_RET_WIDTH_LONG);

	c = &t.cases[7];
	TD_CHECK_EQ(c->retclass, TD_RET_NONZERO);

	c = &t.cases[8];
	TD_CHECK_EQ(c->cap_unit, 2);
	TD_CHECK_EQ(c->cap_len, 32);

	TD_CHECK(strcmp(td_retclass_name(TD_RET_PTR_RAM), "PTR_RAM") == 0);
	TD_CHECK(strcmp(td_retclass_name(TD_RET_HANDLE), "HANDLE") == 0);
}

static void test_values(void)
{
	printf("argument values and widths\n");
	/* a LONG takes the full 32-bit range, both spellings */
	expect_accepted("14:0x10\tl:-1,l:-1,l:0xFFFFFFFF\tPTR_RAM\t-\tKeytbl",
	                "LONG -1 and 0xFFFFFFFF");
	if (t.count == 1)
		TD_CHECK_EQ(t.cases[0].arg[2].value, -1);
	expect_refused("14:0x10\tl:-1,l:-1,l:0x100000000\tPTR_RAM\t-\tKeytbl",
	               TD_CASE_ERR_RANGE, "a LONG past 32 bits");
	expect_refused("13:11\tw:65536\tEXACT\t-\tKbshift", TD_CASE_ERR_RANGE,
	               "a WORD past 16 bits");
	expect_refused("13:11\tx:1\tEXACT\t-\tKbshift", TD_CASE_ERR_VALUE,
	               "an unknown argument kind");
	expect_refused("13:11\tw:\tEXACT\t-\tKbshift", TD_CASE_ERR_VALUE,
	               "an argument without a value");
	expect_refused("14:0x2E\tw:0,w:0,w:50,b:2\tEXACT.W\t-\tNVMaccess",
	               TD_CASE_ERR_RANGE, "a buffer that does not exist");
	expect_refused("14:0x04\tw:1,w:2,w:3,w:4,w:5,w:6,w:7,w:8,w:9\tEXACT.W\t-\tx",
	               TD_CASE_ERR_TOO_MANY_VALUES, "nine arguments");
}

static void test_columns(void)
{
	printf("the call column, the column count, preludes and directives\n");
	expect_refused("14:0x04\t-\tEXACT.W\t-", TD_CASE_ERR_COLUMNS,
	               "four columns");
	expect_refused("0x04\t-\tEXACT.W\t-\tGetrez", TD_CASE_ERR_VALUE,
	               "a call without its trap");
	expect_refused("2:0x04\t-\tEXACT.W\t-\tGetrez", TD_CASE_ERR_UNSAFE_OPCODE,
	               "the GEM trap in a BIOS table");
	expect_refused("1:0x19\t-\tEXACT.W\t-\tDgetdrv", TD_CASE_ERR_UNSAFE_OPCODE,
	               "GEMDOS in a BIOS table");
	expect_refused("14:256\t-\tEXACT\t-\tpast a byte", TD_CASE_ERR_RANGE,
	               "a function number past 255");
	expect_refused("+14:0x04\t-\tEXACT.W\t-\tGetrez", TD_CASE_ERR_DIRECTIVE,
	               "a prelude");
	expect_refused("!reset\tnone", TD_CASE_ERR_DIRECTIVE, "a directive");
	{
		struct td_case_error err;
		int rc;

		memset(&err, 0, sizeof err);
		rc = td_call_cases_parse(&t, TD_AREA_VDI, "14:0x04\t-\tEXACT.W\t-\tx\n",
		                         strlen("14:0x04\t-\tEXACT.W\t-\tx\n"), &err);
		TD_CHECK_EQ(rc, TD_CASE_ERR_UNSAFE_OPCODE);
		memset(&err, 0, sizeof err);
		rc = td_call_cases_parse(&t, TD_AREA_GEMDOS, "1:0x19\t-\tEXACT\t-\tx\n",
		                         strlen("1:0x19\t-\tEXACT\t-\tx\n"), &err);
		TD_CHECK_EQ(rc, TD_CASE_ERR_UNSAFE_OPCODE);
	}
}

static void test_retclass(void)
{
	printf("the retclass column\n");
	expect_accepted("14:0x04\t-\tHANDLE.W\t-\tGetrez", "HANDLE.W");
	expect_accepted("14:0x04\t-\tNONZERO.B\t-\tGetrez", "NONZERO.B");
	expect_refused("14:0x04\t-\tSAME\t-\tGetrez", TD_CASE_ERR_RETCLASS,
	               "an unknown class");
	expect_refused("14:0x04\t-\tEXACT.Q\t-\tGetrez", TD_CASE_ERR_RETCLASS,
	               "an unknown width");
	expect_refused("14:0x04\t-\tEXACT.L\t-\tGetrez", TD_CASE_ERR_RETCLASS,
	               ".L is the default and is not spelt");
	expect_refused("14:0x04\t-\tIGNORE.W\t-\tGetrez", TD_CASE_ERR_RETCLASS,
	               "IGNORE has no width");
	expect_refused("14:0x02\t-\tPTR_RAM.W\t-\tPhysbase", TD_CASE_ERR_RETCLASS,
	               "a pointer is a LONG");
	expect_refused("14:0x04\t-\t-\t-\tGetrez", TD_CASE_ERR_RETCLASS,
	               "a missing class");
}

static void test_capture(void)
{
	printf("the capture column\n");
	expect_accepted("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:256\tProtobt",
	                "the whole record field");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:257\tProtobt",
	               TD_CASE_ERR_CAPTURE, "more than the record holds");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:0\tProtobt",
	               TD_CASE_ERR_CAPTURE, "a zero length");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t1:4\tProtobt",
	               TD_CASE_ERR_CAPTURE, "an argument that is not a buffer");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t4:4\tProtobt",
	               TD_CASE_ERR_CAPTURE, "an argument that does not exist");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:3.W\tProtobt",
	               TD_CASE_ERR_CAPTURE, "a length that is not whole WORDs");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:4.L@2\tProtobt",
	               TD_CASE_ERR_CAPTURE, "an offset that is not LONG aligned");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:16@1016\tProtobt",
	               TD_CASE_ERR_CAPTURE, "past the end of the buffer");
	expect_accepted("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:8@1016\tProtobt",
	                "up to the end of the buffer");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0:4.Q\tProtobt",
	               TD_CASE_ERR_CAPTURE, "an unknown unit");
	expect_refused("14:0x12\tb:0,l:-1,w:-1,w:-1\tIGNORE\t0\tProtobt",
	               TD_CASE_ERR_CAPTURE, "no length");
}

/*
 * The guard.  Each refusal here is a call that is fatal, destructive or
 * blocking on at least one of the two machines (COVERAGE-BIOS.md); each
 * acceptance is its query arm.
 */
static void test_guard(void)
{
	printf("the guard: only established calls, only their query arm\n");

	expect_refused("13:0\tb:0\tIGNORE\t-\tGetmpb", TD_CASE_ERR_UNSAFE_OPCODE,
	               "Getmpb rewrites the TPA descriptor");
	expect_refused("13:4\tw:0,b:0,w:1,w:0,w:2\tEXACT\t-\tRwabs",
	               TD_CASE_ERR_UNSAFE_OPCODE, "Rwabs is not established");
	expect_refused("14:0x09\tb:0,l:0,w:0,w:1,w:0,w:0,w:1\tEXACT.W\t-\tFlopwr",
	               TD_CASE_ERR_UNSAFE_OPCODE, "Flopwr writes a disk");
	expect_refused("14:0x26\tb:0\tEXACT\t-\tSupexec", TD_CASE_ERR_SIGNATURE,
	               "Supexec runs code: only a dumper callback (MEM12)");
	expect_refused("14:0x27\t-\tEXACT\t-\tPuntaes", TD_CASE_ERR_UNSAFE_OPCODE,
	               "Puntaes reboots a real TOS");
	expect_refused("14:0x00\tw:0,l:0,l:0\tIGNORE\t-\tInitmous",
	               TD_CASE_ERR_UNSAFE_OPCODE, "Initmous installs a handler");

	/* the value decides */
	expect_accepted("13:11\tw:-1\tEXACT\t-\tKbshift query", "Kbshift(-1)");
	expect_refused("13:11\tw:5\tEXACT\t-\tKbshift set", TD_CASE_ERR_UNSAFE_FIELD,
	               "Kbshift with a flag sets the shift state");
	expect_accepted("13:5\tw:2,l:-1\tNONZERO\t-\tSetexc query", "Setexc(n,-1)");
	expect_refused("13:5\tw:2,l:0\tNONZERO\t-\tSetexc set",
	               TD_CASE_ERR_UNSAFE_FIELD, "Setexc with a vector installs it");
	expect_accepted("13:2\tw:4\tEXACT\t-\tBconin IKBD", "Bconin(4)");
	expect_accepted("13:2\tw:-1\tEXACT\t-\tBconin -1", "Bconin(-1)");
	expect_refused("13:2\tw:2\tEXACT\t-\tBconin CON", TD_CASE_ERR_UNSAFE_FIELD,
	               "Bconin(2) waits for a key");
	expect_refused("13:2\tw:6\tEXACT\t-\tBconin mapped", TD_CASE_ERR_UNSAFE_FIELD,
	               "Bconin(6) is a Bconmap device and waits");
	expect_refused("13:3\tw:0,w:0x41\tEXACT\t-\tBconout PRT",
	               TD_CASE_ERR_UNSAFE_FIELD, "Bconout(0) waits 30 s for a printer");
	expect_refused("13:3\tw:2,w:27\tEXACT\t-\tBconout ESC",
	               TD_CASE_ERR_UNSAFE_FIELD, "Bconout(2, ESC) leaves VT52 state");
	expect_accepted("13:3\tw:2,w:0x41\tEXACT\t-\tBconout CON A", "Bconout(2, 'A')");
	expect_accepted("14:0x07\tw:0,w:-1\tEXACT.W\t-\tSetcolor query",
	                "Setcolor(n,-1)");
	expect_refused("14:0x07\tw:0,w:0x777\tEXACT.W\t-\tSetcolor set",
	               TD_CASE_ERR_UNSAFE_FIELD, "Setcolor with a colour sets it");
	expect_accepted("14:0x2E\tw:0,w:0,w:50,b:0\tEXACT.W\t3:50\tNVMaccess read",
	                "NVMaccess read");
	expect_refused("14:0x2E\tw:1,w:0,w:50,b:0\tEXACT.W\t-\tNVMaccess write",
	               TD_CASE_ERR_UNSAFE_FIELD, "NVMaccess write");
	expect_refused("14:0x2E\tw:2,w:0,w:0,b:0\tEXACT.W\t-\tNVMaccess reset",
	               TD_CASE_ERR_UNSAFE_FIELD, "NVMaccess reset");
	expect_accepted("14:0x05\tl:-1,l:-1,w:-1\tEXACT.W\t-\tSetscreen no-op",
	                "Setscreen(-1,-1,-1)");
	expect_refused("14:0x05\tl:-1,l:-1,w:0\tEXACT.W\t-\tSetscreen rez",
	               TD_CASE_ERR_UNSAFE_FIELD, "Setscreen with a resolution");
	expect_accepted("14:0x0F\tw:-2,w:-1,w:-1,w:-1,w:-1,w:-1\tEXACT\t-\tRsconf baud",
	                "Rsconf(-2, ...)");
	expect_refused("14:0x0F\tw:0,w:-1,w:-1,w:-1,w:-1,w:-1\tEXACT\t-\tRsconf set",
	               TD_CASE_ERR_UNSAFE_FIELD, "Rsconf with a baud rate");

	/* the widths are the binding's */
	expect_refused("13:11\tl:-1\tEXACT\t-\tKbshift", TD_CASE_ERR_SIGNATURE,
	               "Kbshift takes a WORD");
	expect_refused("13:5\tw:2,w:-1\tNONZERO\t-\tSetexc", TD_CASE_ERR_SIGNATURE,
	               "Setexc's vector is a LONG");
	expect_refused("14:0x04\tw:0\tEXACT.W\t-\tGetrez", TD_CASE_ERR_SIGNATURE,
	               "Getrez takes nothing");
	expect_refused("14:0x2E\tw:0,w:0,w:50,w:0\tEXACT.W\t-\tNVMaccess",
	               TD_CASE_ERR_SIGNATURE, "NVMaccess's buffer is a pointer");
	expect_accepted("14:0x2E\tw:0,w:0,w:1,l:0\tEXACT.W\t-\tNVMaccess NULL",
	                "a pointer may be a LONG (NULL)");

	/* an empty slot answers its own number on EmuTOS and reads nothing */
	expect_accepted("14:0x30\t-\tEXACT\t-\tempty slot", "an empty XBIOS slot");
	expect_refused("14:0x30\tw:1\tEXACT\t-\tempty slot", TD_CASE_ERR_SIGNATURE,
	               "an empty slot takes no arguments");
	expect_accepted("13:12\t-\tEXACT\t-\tpast the BIOS table",
	                "a BIOS number past the table");
}

/*
 * Super (1:0x20) and Supexec (14:0x26) -- MEM12, docs/TOS-REFERENCE-DUMPER.md
 * section 6, "Super and Supexec in a BIOS-area table".
 */
static void test_super_supexec(void)
{
	struct td_case_error err;
	const struct td_call_case *c;
	int rc;
	static const char *bracket =
		"# super bracket\n"
		"1:0x20\tl:1\tEXACT\t-\tSuper(1L) user\n"
		"1:0x20\tl:0\tNONZERO\t-\tSuper(0L) opens\n"
		"1:0x20\tl:1\tEXACT\t-\tSuper(1L) super\n"
		"1:0x20\tt:0\tIGNORE\t-\tSuperToUser closes\n"
		"14:0x26\tf:2\tEXACT\t-\tSupexec(cb2)\n"
		"1:0x20\tl:0\tNONZERO\t-\ta second bracket\n"
		"1:0x20\tt:0\tIGNORE\t-\tclosed again\n";

	printf("Super and Supexec: the bracket, the callbacks, the guard\n");
	rc = parse(bracket, &err);
	TD_CHECK_EQ(rc, 0);
	if (rc != 0) {
		printf("    line %u: %s\n", err.line, err.what ? err.what : "");
		return;
	}
	TD_CHECK_EQ(t.count, 7);
	c = &t.cases[0];
	TD_CHECK_EQ(c->trap, 1);
	TD_CHECK_EQ(c->opcode, 0x20);
	TD_CHECK_EQ(c->arg[0].kind, TD_CALL_ARG_LONG);
	TD_CHECK_EQ(c->arg[0].value, 1);
	c = &t.cases[3];
	TD_CHECK_EQ(c->line, 5);
	TD_CHECK_EQ(c->nargs, 1);
	TD_CHECK_EQ(c->arg[0].kind, TD_CALL_ARG_TOKEN);
	TD_CHECK_EQ(c->arg[0].value, 0);
	TD_CHECK_EQ(c->retclass, TD_RET_IGNORE);
	c = &t.cases[4];
	TD_CHECK_EQ(c->trap, 14);
	TD_CHECK_EQ(c->opcode, 0x26);
	TD_CHECK_EQ(c->arg[0].kind, TD_CALL_ARG_FUNC);
	TD_CHECK_EQ(c->arg[0].value, 2);

	expect_accepted("1:0x20\tl:1\tEXACT\t-\tSuper inquiry", "Super(1L)");
	expect_accepted("14:0x26\tf:0\tEXACT\t-\tSupexec cb0", "Supexec(cb0)");
	expect_accepted("14:0x26\tf:1\tEXACT\t-\tSupexec cb1", "Supexec(cb1)");

	/* the bracket */
	expect_refused("1:0x20\tl:0\tNONZERO\t-\tnever closed",
	               TD_CASE_ERR_UNSAFE_STATE, "a bracket left open");
	expect_refused("1:0x20\tt:0\tIGNORE\t-\tnothing to close",
	               TD_CASE_ERR_UNSAFE_STATE, "SuperToUser outside a bracket");
	rc = parse("# nested\n"
	           "1:0x20\tl:0\tNONZERO\t-\topens\n"
	           "1:0x20\tl:0\tNONZERO\t-\topens again\n"
	           "1:0x20\tt:0\tIGNORE\t-\tcloses\n", &err);
	TD_CHECK_EQ(rc, TD_CASE_ERR_UNSAFE_STATE);
	TD_CHECK_EQ(err.line, 3);

	/* the values and widths */
	expect_refused("1:0x20\tl:0x1000\tNONZERO\t-\texplicit stack",
	               TD_CASE_ERR_UNSAFE_FIELD,
	               "Super with a stack address runs on that address");
	expect_refused("1:0x20\tl:-1\tEXACT\t-\tSuper(-1)",
	               TD_CASE_ERR_UNSAFE_FIELD, "Super(-1) is a stack address");
	expect_refused("1:0x20\tw:1\tEXACT\t-\tSuper WORD",
	               TD_CASE_ERR_SIGNATURE, "Super's argument is a LONG");
	expect_refused("1:0x20\t-\tEXACT\t-\tSuper nothing",
	               TD_CASE_ERR_SIGNATURE, "Super takes one argument");
	expect_refused("1:0x20\tt:1\tIGNORE\t-\tSuper t:1",
	               TD_CASE_ERR_RANGE, "there is one token, t:0");
	expect_refused("14:0x26\tf:3\tEXACT\t-\tSupexec cb3",
	               TD_CASE_ERR_RANGE, "there are three callbacks");
	expect_refused("14:0x26\tl:0\tEXACT\t-\tSupexec NULL",
	               TD_CASE_ERR_SIGNATURE, "Supexec only with a callback");
	expect_refused("14:0x26\tf:0,f:1\tEXACT\t-\tSupexec two",
	               TD_CASE_ERR_SIGNATURE, "Supexec takes one argument");
	expect_refused("13:5\tw:2,f:0\tNONZERO\t-\tSetexc callback",
	               TD_CASE_ERR_SIGNATURE, "a callback only where f is in the "
	               "signature");
	expect_refused("14:0x2E\tw:0,w:0,w:50,t:0\tEXACT.W\t-\tNVM token",
	               TD_CASE_ERR_SIGNATURE, "a token only for Super");

	/* the rest of GEMDOS waits for DUMPGDOS */
	expect_refused("1:0x19\t-\tEXACT.W\t-\tDgetdrv",
	               TD_CASE_ERR_UNSAFE_OPCODE, "GEMDOS other than Super");
	expect_refused("1:0x30\t-\tEXACT.W\t-\tSversion",
	               TD_CASE_ERR_UNSAFE_OPCODE, "GEMDOS other than Super");
	expect_refused("2:0x20\tl:1\tEXACT\t-\ttrap 2",
	               TD_CASE_ERR_UNSAFE_OPCODE, "trap 2 is no call trap");
}

/* The shipped table is carried to a machine with no debugger: it must pass. */
static void test_shipped(void)
{
	struct td_case_error err;
	int rc;

	printf("the shipped table cases/bios-r1.tsv passes the guard\n");
	memset(&err, 0, sizeof err);
	rc = td_call_cases_load(&t, TD_AREA_BIOS, "../cases/bios-r1.tsv", &err);
	if (rc != 0) {
		printf("  FAIL cases/bios-r1.tsv line %u: %s (%s)\n", err.line,
		       err.what ? err.what : "", td_case_error_text(rc));
		td_test_failures++;
		return;
	}
	TD_CHECK(t.count > 0);
}

static void test_shipped_super(void)
{
	struct td_case_error err;
	int rc;

	printf("the shipped table cases/super-r1.tsv passes the guard\n");
	memset(&err, 0, sizeof err);
	rc = td_call_cases_load(&t, TD_AREA_BIOS, "../cases/super-r1.tsv", &err);
	if (rc != 0) {
		printf("  FAIL cases/super-r1.tsv line %u: %s (%s)\n", err.line,
		       err.what ? err.what : "", td_case_error_text(rc));
		td_test_failures++;
		return;
	}
	TD_CHECK(t.count > 0);
}

int main(void)
{
	test_well_formed();
	test_values();
	test_columns();
	test_retclass();
	test_capture();
	test_guard();
	test_super_supexec();
	test_shipped();
	test_shipped_super();
	return TD_TEST_RESULT();
}

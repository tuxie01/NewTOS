# Makefile -- host-side build and tests for the TOS reference dumper.
#
# `make test` is the T1 acceptance command (spec section 9).

CC      ?= cc
CFLAGS  ?= -std=c99 -O1 -g
CFLAGS  += -Wall -Wextra -Werror -Wno-unused-parameter
CFLAGS  += -I../core -I../host -I../tests

BUILD   := build
CORE    := ../core
TESTS   := ../tests

.PHONY: all test run-tests clean
all: $(BUILD)/tosdiff $(BUILD)/casecheck $(BUILD)/mkrusttests \
     $(BUILD)/layoutcheck

$(BUILD):
	@mkdir -p $(BUILD)

$(BUILD)/record.o: $(CORE)/record.c $(CORE)/record.h $(CORE)/tosdump.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/cases.o: $(CORE)/cases.c $(CORE)/cases.h $(CORE)/record.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/gem.o: $(CORE)/gem.c $(CORE)/gem.h $(CORE)/abi.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/runner.o: $(CORE)/runner.c $(CORE)/runner.h $(CORE)/abi.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/runner_aes.o: $(CORE)/runner_aes.c $(CORE)/runner_aes.h $(CORE)/abi.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/runner_rast.o: $(CORE)/runner_rast.c $(CORE)/runner_rast.h $(CORE)/abi.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/runner_call.o: $(CORE)/runner_call.c $(CORE)/runner_call.h $(CORE)/abi.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/dump.o: $(CORE)/dump.c $(CORE)/dump.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/log.o: $(CORE)/log.c $(CORE)/log.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/stub_abi.o: $(TESTS)/stub_abi.c $(TESTS)/stub_abi.h $(CORE)/abi.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@

CORE_OBJ := $(BUILD)/record.o $(BUILD)/cases.o $(BUILD)/gem.o \
            $(BUILD)/runner.o $(BUILD)/runner_aes.o $(BUILD)/runner_rast.o \
            $(BUILD)/runner_call.o $(BUILD)/dump.o $(BUILD)/log.o

# The same writer, built for both target configurations (spec section 2.1).
$(BUILD)/record_be.o: $(CORE)/record.c $(CORE)/record.h $(CORE)/tosdump.h | $(BUILD)
	$(CC) $(CFLAGS) -DTD_BIG_ENDIAN_TARGET=1 -c $< -o $@
$(BUILD)/record_le.o: $(CORE)/record.c $(CORE)/record.h $(CORE)/tosdump.h | $(BUILD)
	$(CC) $(CFLAGS) -DTD_BIG_ENDIAN_TARGET=0 -c $< -o $@

$(BUILD)/casefile.o: casefile.c casefile.h $(CORE)/record.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@

$(BUILD)/tosdiff.o: tosdiff.c casefile.h $(CORE)/record.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@

$(BUILD)/tosdiff: $(BUILD)/tosdiff.o $(BUILD)/casefile.o $(BUILD)/record.o \
                  $(BUILD)/cases.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_record.o: $(TESTS)/test_record.c $(TESTS)/tdtest.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_record: $(BUILD)/test_record.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/mkdump.o: $(TESTS)/mkdump.c casefile.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/mkdump: $(BUILD)/mkdump.o $(BUILD)/casefile.o $(BUILD)/record.o \
                 $(BUILD)/cases.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/casecheck.o: casecheck.c $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/casecheck: $(BUILD)/casecheck.o $(BUILD)/cases.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

# TOOL16: the tables' "!layout" lines are the docs/api/ specs' lines.
$(BUILD)/layoutcheck.o: layoutcheck.c $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/layoutcheck: $(BUILD)/layoutcheck.o $(BUILD)/cases.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_log.o: $(TESTS)/test_log.c $(TESTS)/tdtest.h $(CORE)/log.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_log: $(BUILD)/test_log.o $(BUILD)/log.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_casefile.o: $(TESTS)/test_casefile.c $(TESTS)/tdtest.h \
                          casefile.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_casefile: $(BUILD)/test_casefile.o $(BUILD)/casefile.o \
                        $(BUILD)/cases.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/mkrusttests.o: mkrusttests.c casefile.h $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/mkrusttests: $(BUILD)/mkrusttests.o $(BUILD)/cases.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases.o: $(TESTS)/test_cases.c $(TESTS)/tdtest.h $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases: $(BUILD)/test_cases.o $(BUILD)/cases.o $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner.o: $(TESTS)/test_runner.c $(TESTS)/tdtest.h \
                        $(TESTS)/stub_abi.h $(CORE)/runner.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner: $(BUILD)/test_runner.o $(CORE_OBJ) $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases_aes.o: $(TESTS)/test_cases_aes.c $(TESTS)/tdtest.h \
                           $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases_aes: $(BUILD)/test_cases_aes.o $(BUILD)/cases.o \
                         $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner_aes.o: $(TESTS)/test_runner_aes.c $(TESTS)/tdtest.h \
                            $(TESTS)/stub_abi.h $(CORE)/runner_aes.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner_aes: $(BUILD)/test_runner_aes.o $(CORE_OBJ) \
                          $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases_rast.o: $(TESTS)/test_cases_rast.c $(TESTS)/tdtest.h \
                           $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases_rast: $(BUILD)/test_cases_rast.o $(BUILD)/cases.o \
                          $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner_rast.o: $(TESTS)/test_runner_rast.c $(TESTS)/tdtest.h \
                             $(TESTS)/stub_abi.h $(CORE)/runner_rast.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner_rast: $(BUILD)/test_runner_rast.o $(CORE_OBJ) \
                           $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases_call.o: $(TESTS)/test_cases_call.c $(TESTS)/tdtest.h \
                           $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases_call: $(BUILD)/test_cases_call.o $(BUILD)/cases.o \
                          $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner_call.o: $(TESTS)/test_runner_call.c $(TESTS)/tdtest.h \
                             $(TESTS)/stub_abi.h $(CORE)/runner_call.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner_call: $(BUILD)/test_runner_call.o $(CORE_OBJ) \
                           $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases_disk.o: $(TESTS)/test_cases_disk.c $(TESTS)/tdtest.h \
                           $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases_disk: $(BUILD)/test_cases_disk.o $(BUILD)/cases.o \
                          $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner_disk.o: $(TESTS)/test_runner_disk.c $(TESTS)/tdtest.h \
                             $(TESTS)/stub_abi.h $(CORE)/runner_call.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner_disk: $(BUILD)/test_runner_disk.o $(CORE_OBJ) \
                           $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases_ptr.o: $(TESTS)/test_cases_ptr.c $(TESTS)/tdtest.h \
                          $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases_ptr: $(BUILD)/test_cases_ptr.o $(BUILD)/cases.o \
                         $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner_ptr.o: $(TESTS)/test_runner_ptr.c $(TESTS)/tdtest.h \
                            $(TESTS)/stub_abi.h $(CORE)/runner_call.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner_ptr: $(BUILD)/test_runner_ptr.o $(CORE_OBJ) \
                          $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_cases_gdos.o: $(TESTS)/test_cases_gdos.c $(TESTS)/tdtest.h \
                           $(CORE)/cases.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_cases_gdos: $(BUILD)/test_cases_gdos.o $(BUILD)/cases.o \
                          $(BUILD)/record.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/test_runner_gdos.o: $(TESTS)/test_runner_gdos.c $(TESTS)/tdtest.h \
                             $(TESTS)/stub_abi.h $(CORE)/runner_call.h | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/test_runner_gdos: $(BUILD)/test_runner_gdos.o $(CORE_OBJ) \
                           $(BUILD)/stub_abi.o
	$(CC) $(CFLAGS) $^ -o $@

$(BUILD)/emit_record.o: $(TESTS)/emit_record.c | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@
$(BUILD)/emit_be: $(BUILD)/emit_record.o $(BUILD)/record_be.o
	$(CC) $(CFLAGS) $^ -o $@
$(BUILD)/emit_le: $(BUILD)/emit_record.o $(BUILD)/record_le.o
	$(CC) $(CFLAGS) $^ -o $@

# `make test` always rebuilds from scratch.  The make that ships with macOS
# compares modification times at one-second resolution, so an edit and a
# re-run inside the same second silently test the previous object file -- a
# false green, which is the one failure mode this whole exercise exists to
# rule out.  The full build takes well under a second; correctness wins.
test:
	@$(MAKE) --no-print-directory clean
	@$(MAKE) --no-print-directory run-tests

run-tests: $(BUILD)/test_record $(BUILD)/test_cases $(BUILD)/test_log \
      $(BUILD)/test_runner $(BUILD)/test_cases_aes $(BUILD)/test_runner_aes \
      $(BUILD)/test_cases_rast $(BUILD)/test_runner_rast \
      $(BUILD)/test_cases_call $(BUILD)/test_runner_call \
      $(BUILD)/test_cases_disk $(BUILD)/test_runner_disk \
      $(BUILD)/test_cases_ptr $(BUILD)/test_runner_ptr $(BUILD)/layoutcheck \
      $(BUILD)/test_cases_gdos $(BUILD)/test_runner_gdos \
      $(BUILD)/test_casefile $(BUILD)/tosdiff $(BUILD)/mkdump \
      $(BUILD)/emit_be $(BUILD)/emit_le
	@echo "== record format"
	@$(BUILD)/test_record
	@echo "== case table"
	@$(BUILD)/test_cases
	@echo "== run log"
	@$(BUILD)/test_log $(BUILD)/test.log
	@echo "== runner against the stub backend"
	@$(BUILD)/test_runner $(BUILD)/stub-vdi.bin
	@$(BUILD)/tosdiff --validate $(BUILD)/stub-vdi.bin | sed 's/^/  /'
	@echo "== AES case table"
	@$(BUILD)/test_cases_aes
	@echo "== AES runner against the stub backend"
	@$(BUILD)/test_runner_aes $(BUILD)/stub-aes.bin $(BUILD)/stub-aes.tsv
	@$(BUILD)/tosdiff --validate $(BUILD)/stub-aes.bin $(BUILD)/stub-aes.tsv \
	    | sed 's/^/  /'
	@echo "== VDI raster case table and its guard"
	@$(BUILD)/test_cases_rast
	@echo "== VDI raster runner against the stub backend"
	@$(BUILD)/test_runner_rast $(BUILD)/stub-raster.bin $(BUILD)/stub-raster.tsv
	@$(BUILD)/tosdiff --validate $(BUILD)/stub-raster.bin | sed 's/^/  /'
	@echo "== call case table (BIOS + XBIOS) and its guard"
	@$(BUILD)/test_cases_call
	@echo "== call runner against the stub backend"
	@$(BUILD)/test_runner_call $(BUILD)/stub-bios.bin $(BUILD)/stub-bios.tsv
	@$(BUILD)/tosdiff --validate $(BUILD)/stub-bios.bin $(BUILD)/stub-bios.tsv \
	    | sed 's/^/  /'
	@echo "== disk round: !disk, preludes, Rwabs/Getbpb/Mediach guard, r: capture"
	@$(BUILD)/test_cases_disk
	@echo "== disk round: runner against the stub backend"
	@$(BUILD)/test_runner_disk
	@echo "== structures behind a pointer: !layout, layout captures, +put, Getmpb"
	@$(BUILD)/test_cases_ptr
	@echo "== structures behind a pointer: runner against the stub backend"
	@$(BUILD)/test_runner_ptr $(BUILD)/stub-ptr.bin $(BUILD)/stub-ptr.tsv
	@$(BUILD)/tosdiff --validate $(BUILD)/stub-ptr.bin $(BUILD)/stub-ptr.tsv \
	    | sed 's/^/  /'
	@echo "== GEMDOS after a media change: :packed, x<n>, arrays, strings, tokens, the DTA, the guard"
	@$(BUILD)/test_cases_gdos
	@echo "== GEMDOS after a media change: runner against the stub backend"
	@$(BUILD)/test_runner_gdos
	@echo "== layouts: the tables' are the docs/api/ specs'"
	@sh $(TESTS)/test_layoutcheck.sh $(BUILD)
	@echo "== case table reader, per area"
	@$(BUILD)/test_casefile $(BUILD)
	@echo "== byte order: both target configurations emit identical records"
	@$(BUILD)/emit_be $(BUILD)/be.bin
	@$(BUILD)/emit_le $(BUILD)/le.bin
	@cmp $(BUILD)/be.bin $(BUILD)/le.bin && echo "  ok"
	@echo "== comparator"
	@sh $(TESTS)/test_tosdiff.sh $(BUILD)
	@echo "== self-check: every relational rule holds on the pinned oracle"
	@sh $(TESTS)/test_selfcheck.sh $(BUILD)

clean:
	rm -rf $(BUILD)

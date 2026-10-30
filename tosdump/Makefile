# Makefile -- cross builds of the TOS reference dumper.
#
#   make dumpvdi-m68k     DUMPVDI.PRG for TOS 3.06 on the TT
#   make dumpvdi-newtos   DUMPVDI.NAP for NewTOS on the Pi 4 (AArch64)
#   make dumpaes-m68k     DUMPAES.PRG for TOS 3.06 on the TT
#   make dumpaes-newtos   DUMPAES.NAP for NewTOS on the Pi 4 (AArch64)
#   make dumprast-m68k    DUMPRAST.PRG for TOS/EmuTOS on the TT
#   make dumprast-newtos  DUMPRAST.NAP for NewTOS on the Pi 4 (AArch64)
#   make dumpbios-m68k    DUMPBIOS.PRG, BIOS + XBIOS, for TOS/EmuTOS (m68k)
#   make dumpbios-newtos  DUMPBIOS.NAP, BIOS + XBIOS, for NewTOS (AArch64)
#   make scrninfo-m68k    SCRNINFO.TTP, the EdDI / vq_scrninfo probe (TOS)
#   make scrninfo-newtos  SCRNINFO.NAP, the same probe for NewTOS (AArch64)
#   make gcprobe-m68k     GCPROBE.PRG, the media-change probe (probes/gcprobe.c)
#   make gcprobe-newtos   GCPROBE.NAP, the same probe for NewTOS (AArch64)
#   make pxprobe-m68k     PXPROBE.PRG, the Pexec/Fsfirst media-change probe
#                         (probes/pxprobe.c)
#   make pxprobe-newtos   PXPROBE.NAP, the same probe for NewTOS (AArch64)
#   make hkprobe-m68k     HKPROBE.PRG, the OS-hook probe (probes/hkprobe.c)
#   make hkprobe-newtos   HKPROBE.NAP, the same probe for NewTOS (AArch64)
#   make axprobe-m68k     AXPROBE.PRG, a kbdvec hook during Bconin(1)
#                         (probes/axprobe.c)
#   make axprobe-newtos   AXPROBE.NAP, the same probe for NewTOS (AArch64)
#   make mdprobe-m68k     MDPROBE.PRG, BIOS devices 3 (MIDI) and 4 (IKBD)
#                         (probes/mdprobe.c)
#   make mdprobe-newtos   MDPROBE.NAP, the same probe for NewTOS (AArch64)
#   make sxprobe-m68k     SXPROBE.PRG, the Setexc probe (probes/sxprobe.c)
#   make sxprobe-newtos   SXPROBE.NAP, the same probe for NewTOS (AArch64)
#   make vwprobe-m68k     VWPROBE.PRG, the page-0 vector-write probe (probes/vwprobe.c)
#   make vwprobe-newtos   VWPROBE.NAP, the same probe for NewTOS (AArch64)
#   make ksprobe-m68k     KSPROBE.PRG, the Kbshift probe (probes/ksprobe.c)
#   make ksprobe-newtos   KSPROBE.NAP, the same probe for NewTOS (AArch64)
#   make host-test        the host-side suite (same as make -C host test)
#
# Same core sources for both; only abi/ differs (spec section 2).

# Toolchain paths come from the oracle's one shared configuration
# (docs/analysis/ORACLE.md); nothing is taken from $PATH.
include $(dir $(lastword $(MAKEFILE_LIST)))../tools/oracle/oracle.env
CROSS_MINT ?= $(ORACLE_CROSS_MINT)
M68K_CC    := $(CROSS_MINT)gcc

# 4 bytes of the git hash, ASCII, as the record header carries it.
# git widens --short when 4 hex digits are not unique; the header field is
# exactly 4 bytes, so cut it to what actually gets stored.
BUILD_ID := $(shell git rev-parse HEAD 2>/dev/null | cut -c1-4 || echo ----)

# The m68k is big-endian, so the record order is the machine order: no swap.
M68K_CFLAGS := -std=c99 -O2 -Wall -Wextra -Werror -Icore \
               -DTD_BIG_ENDIAN_TARGET=1 -DTD_BUILD_ID='"$(BUILD_ID)"'

# Shared by both dumpers.  The per-area runner and main are added on top, so
# neither binary carries the other area's runner around on a 4 MB TT.
CORE_SRC := core/record.c core/cases.c core/gem.c core/dump.c core/log.c
VDI_SRC  := $(CORE_SRC) core/runner.c core/dumpvdi.c
AES_SRC  := $(CORE_SRC) core/runner_aes.c core/dumpaes.c
# The raster dumper reuses runner.c for td_reset_vdi: the reset before every
# case is the same sequence, and stating it twice is how two runs stop being
# comparable.
RAST_SRC := $(CORE_SRC) core/runner.c core/runner_rast.c core/dumprast.c
# The BIOS + XBIOS dumper is no AES client: no gem.c, no VDI reset.
BIOS_SRC := core/record.c core/cases.c core/dump.c core/log.c \
            core/runner_call.c core/dumpbios.c
# The EdDI probe needs only the workstation handling; the cookie jar, the
# EdDI dispatcher and the clock are TOS-only and sit in their own backend.
SCRN_SRC := core/gem.c core/scrninfo.c abi/m68k_scrninfo.c

BUILD := build

.PHONY: all dumpvdi-m68k dumpvdi-newtos dumpaes-m68k dumpaes-newtos \
        dumprast-m68k dumprast-newtos dumpbios-m68k dumpbios-newtos \
        scrninfo-m68k scrninfo-newtos gcprobe-m68k gcprobe-newtos \
        pxprobe-m68k pxprobe-newtos hkprobe-m68k hkprobe-newtos \
        axprobe-m68k axprobe-newtos mdprobe-m68k mdprobe-newtos \
        sxprobe-m68k sxprobe-newtos ksprobe-m68k ksprobe-newtos \
        vwprobe-m68k vwprobe-newtos \
        host-test clean
all: dumpvdi-m68k dumpvdi-newtos dumpaes-m68k dumpaes-newtos \
     dumprast-m68k dumprast-newtos dumpbios-m68k dumpbios-newtos \
     scrninfo-m68k scrninfo-newtos gcprobe-m68k gcprobe-newtos \
     pxprobe-m68k pxprobe-newtos hkprobe-m68k hkprobe-newtos \
     axprobe-m68k axprobe-newtos mdprobe-m68k mdprobe-newtos \
     sxprobe-m68k sxprobe-newtos ksprobe-m68k ksprobe-newtos \
     vwprobe-m68k vwprobe-newtos host-test

$(BUILD):
	@mkdir -p $(BUILD)

# .PRG, not .TOS: the dumper is an AES client (appl_init / graf_handle /
# v_opnvwk), and the desktop only gives a GEM environment to a .PRG.
dumpvdi-m68k: $(BUILD)/DUMPVDI.PRG
$(BUILD)/DUMPVDI.PRG: $(VDI_SRC) abi/m68k_trap.c $(wildcard core/*.h) | $(BUILD)
	$(M68K_CC) $(M68K_CFLAGS) $(VDI_SRC) abi/m68k_trap.c -o $@
	@$(CROSS_MINT)size $@

# The NewTOS half goes through its own script, which reuses the proven client
# link path of the other native apps (sdk/libc + sdk/newtos.ld).
dumpvdi-newtos: $(BUILD)/DUMPVDI.NAP
$(BUILD)/DUMPVDI.NAP: $(VDI_SRC) abi/newtos_svc.c $(wildcard core/*.h) | $(BUILD)
	./build-dumpvdi-newtos.sh $@

# Same .PRG reasoning as DUMPVDI: the dumper is an AES client, and the
# desktop only gives a GEM environment to a .PRG.
dumpaes-m68k: $(BUILD)/DUMPAES.PRG
$(BUILD)/DUMPAES.PRG: $(AES_SRC) abi/m68k_trap.c $(wildcard core/*.h) | $(BUILD)
	$(M68K_CC) $(M68K_CFLAGS) $(AES_SRC) abi/m68k_trap.c -o $@
	@$(CROSS_MINT)size $@

dumpaes-newtos: $(BUILD)/DUMPAES.NAP
$(BUILD)/DUMPAES.NAP: $(AES_SRC) abi/newtos_svc.c $(wildcard core/*.h) | $(BUILD)
	./build-dumpaes-newtos.sh $@

dumprast-m68k: $(BUILD)/DUMPRAST.PRG
$(BUILD)/DUMPRAST.PRG: $(RAST_SRC) abi/m68k_trap.c $(wildcard core/*.h) | $(BUILD)
	$(M68K_CC) $(M68K_CFLAGS) $(RAST_SRC) abi/m68k_trap.c -o $@
	@$(CROSS_MINT)size $@

dumprast-newtos: $(BUILD)/DUMPRAST.NAP
$(BUILD)/DUMPRAST.NAP: $(RAST_SRC) abi/newtos_svc.c $(wildcard core/*.h) | $(BUILD)
	./build-dumprast-newtos.sh $@

# .PRG like the others, so the oracle's --auto and the desktop start it the
# same way; it makes no AES call.  A jar case (TOOL18) reads the cookie jar
# through the SCRNINFO backend's td_jar_read (Supexec), the one copy of it.
dumpbios-m68k: $(BUILD)/DUMPBIOS.PRG
$(BUILD)/DUMPBIOS.PRG: $(BIOS_SRC) abi/m68k_trap.c abi/m68k_scrninfo.c $(wildcard core/*.h) | $(BUILD)
	$(M68K_CC) $(M68K_CFLAGS) $(BIOS_SRC) abi/m68k_trap.c abi/m68k_scrninfo.c -o $@
	@$(CROSS_MINT)size $@

dumpbios-newtos: $(BUILD)/DUMPBIOS.NAP
$(BUILD)/DUMPBIOS.NAP: $(BIOS_SRC) abi/newtos_svc.c abi/newtos_scrninfo.c $(wildcard core/*.h) | $(BUILD)
	./build-dumpbios-newtos.sh $@

# .TTP: it takes the report file name as its argument.  appl_init works from
# a TOS program as well, which is all the probe needs of the AES.  The TOS
# backend reads the jar via Supexec and calls an m68k EdDI dispatcher.
scrninfo-m68k: $(BUILD)/SCRNINFO.TTP
$(BUILD)/SCRNINFO.TTP: $(SCRN_SRC) abi/m68k_trap.c $(wildcard core/*.h) | $(BUILD)
	$(M68K_CC) $(M68K_CFLAGS) $(SCRN_SRC) abi/m68k_trap.c -o $@
	@$(CROSS_MINT)size $@

# The NewTOS build: same core, abi/newtos_scrninfo.c + abi/newtos_svc.c.  A
# .NAP, because it is an AES client; NewTOS' EdDI cookie is a non-callable
# marker (spec E2), which the core does not call.
scrninfo-newtos: $(BUILD)/SCRNINFO.NAP
$(BUILD)/SCRNINFO.NAP: core/gem.c core/scrninfo.c abi/newtos_scrninfo.c abi/newtos_svc.c $(wildcard core/*.h) | $(BUILD)
	./build-scrninfo-newtos.sh $@

# The media-change probe (TASK/FS20): GEMDOS calls, so no dumper and no core;
# a .PRG for the oracle's --auto (tools/hatari-oracle.sh profile disk), a .NAP
# for the desktop's #Z autorun. Run by probes/run-gcprobe.sh.
gcprobe-m68k: $(BUILD)/GCPROBE.PRG
$(BUILD)/GCPROBE.PRG: probes/gcprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/gcprobe.c -o $@
	@$(CROSS_MINT)size $@

gcprobe-newtos: $(BUILD)/GCPROBE.NAP
$(BUILD)/GCPROBE.NAP: probes/gcprobe.c build-gcprobe-newtos.sh | $(BUILD)
	./build-gcprobe-newtos.sh $@

# The Pexec/Fsfirst media-change probe (TASK/FS22): the same shape as
# gcprobe; each build is also the other side's test-image file (C:\PXPROBE.PRG
# and C:\PXPROBE.NAP). Run by probes/run-pxprobe.sh.
pxprobe-m68k: $(BUILD)/PXPROBE.PRG
$(BUILD)/PXPROBE.PRG: probes/pxprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/pxprobe.c -o $@
	@$(CROSS_MINT)size $@

pxprobe-newtos: $(BUILD)/PXPROBE.NAP
$(BUILD)/PXPROBE.NAP: probes/pxprobe.c build-pxprobe-newtos.sh | $(BUILD)
	./build-pxprobe-newtos.sh $@

# The OS-hook probe (TASK/BIOS15): no disk, any oracle profile. Run by
# probes/run-hkprobe.sh.
hkprobe-m68k: $(BUILD)/HKPROBE.PRG
$(BUILD)/HKPROBE.PRG: probes/hkprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/hkprobe.c -o $@
	@$(CROSS_MINT)size $@

hkprobe-newtos: $(BUILD)/HKPROBE.NAP
$(BUILD)/HKPROBE.NAP: probes/hkprobe.c build-hkprobe-newtos.sh | $(BUILD)
	./build-hkprobe-newtos.sh $@

# The kbdvec-hook-during-Bconin(1) probe (TASK/XBIOS18): no disk; needs a key
# and a serial byte injected while it waits. Run by probes/run-axprobe.sh.
axprobe-m68k: $(BUILD)/AXPROBE.PRG
$(BUILD)/AXPROBE.PRG: probes/axprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/axprobe.c -o $@
	@$(CROSS_MINT)size $@

axprobe-newtos: $(BUILD)/AXPROBE.NAP
$(BUILD)/AXPROBE.NAP: probes/axprobe.c build-axprobe-newtos.sh | $(BUILD)
	./build-axprobe-newtos.sh $@

# The MIDI/IKBD device probe (TASK/BIOS04): no disk; ends waiting in
# Bconin(3). Run by probes/run-mdprobe.sh.
mdprobe-m68k: $(BUILD)/MDPROBE.PRG
$(BUILD)/MDPROBE.PRG: probes/mdprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/mdprobe.c -o $@
	@$(CROSS_MINT)size $@

mdprobe-newtos: $(BUILD)/MDPROBE.NAP
$(BUILD)/MDPROBE.NAP: probes/mdprobe.c build-mdprobe-newtos.sh | $(BUILD)
	./build-mdprobe-newtos.sh $@

# The Setexc probe (TASK/BIOS07): no disk, any oracle profile. Run by
# probes/run-sxprobe.sh.
sxprobe-m68k: $(BUILD)/SXPROBE.PRG
$(BUILD)/SXPROBE.PRG: probes/sxprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/sxprobe.c -o $@
	@$(CROSS_MINT)size $@

sxprobe-newtos: $(BUILD)/SXPROBE.NAP
$(BUILD)/SXPROBE.NAP: probes/sxprobe.c build-sxprobe-newtos.sh | $(BUILD)
	./build-sxprobe-newtos.sh $@

# The page-0 vector-write probe (TASK/BIOS16): no disk, any oracle profile.
# Run by probes/run-vwprobe.sh.
vwprobe-m68k: $(BUILD)/VWPROBE.PRG
$(BUILD)/VWPROBE.PRG: probes/vwprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/vwprobe.c -o $@
	@$(CROSS_MINT)size $@

vwprobe-newtos: $(BUILD)/VWPROBE.NAP
$(BUILD)/VWPROBE.NAP: probes/vwprobe.c build-vwprobe-newtos.sh | $(BUILD)
	./build-vwprobe-newtos.sh $@

# The Kbshift probe (TASK/BIOS10): no disk, key strokes injected. Run by
# probes/run-ksprobe.sh.
ksprobe-m68k: $(BUILD)/KSPROBE.PRG
$(BUILD)/KSPROBE.PRG: probes/ksprobe.c | $(BUILD)
	$(M68K_CC) -O2 -Wall -Wextra -Werror probes/ksprobe.c -o $@
	@$(CROSS_MINT)size $@

ksprobe-newtos: $(BUILD)/KSPROBE.NAP
$(BUILD)/KSPROBE.NAP: probes/ksprobe.c build-ksprobe-newtos.sh | $(BUILD)
	./build-ksprobe-newtos.sh $@

host-test:
	@$(MAKE) --no-print-directory -C host test

clean:
	rm -rf $(BUILD)
	@$(MAKE) --no-print-directory -C host clean

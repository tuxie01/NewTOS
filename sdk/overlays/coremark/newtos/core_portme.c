/*
Copyright 2018 Embedded Microprocessor Benchmark Consortium (EEMBC)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

Original Author: Shay Gal-on
*/

/*
 * NewTOS/AArch64 port of CoreMark 1.0 -- porting layer implementation.
 * See core_portme.h for the run-rule background and the list of deltas
 * against sdk/apps/coremark/atari/core_portme.c, which this file is
 * derived from. In short:
 *
 *   - The seeds come from volatile variables (SEED_VOLATILE), because NewTOS
 *     cannot hand a program a command tail interactively. PERFORMANCE_RUN /
 *     VALIDATION_RUN pick the seed set at compile time.
 *   - Timing is USE_CLOCK, i.e. mintlib's unmodified clock() over the 200 Hz
 *     _hz_200 system variable -- the same clock source the Atari port uses,
 *     and only ever as a DIFFERENCE (MYTIMEDIFF).
 *   - The MULTITHREAD>1 implementations (pthreads / fork+shm / fork+socket)
 *     are gone: a single-tasking kernel has none of those primitives, and an
 *     #error is the honest answer to MULTITHREAD>1 rather than a fake context.
 *   - portable_init() prints a one-line banner, and portable_fini() flushes
 *     stdout and, when the program was started from the desktop, holds the
 *     report on screen until a key. Reasons at each site.
 */

#include <stdio.h>
#include <stdlib.h>
#include <mint/osbind.h> /* Dgetpath, Crawcin -- portable_fini */
#include "coremark.h"

#if (MULTITHREAD > 1)
#error "NewTOS is single-tasking (no fork/pthreads): MULTITHREAD must be 1."
#endif

#if (MEM_METHOD == MEM_MALLOC)
#include <malloc.h>
/* Function: portable_malloc
        Provide malloc() functionality in a platform specific way.

        mintlib's malloc, whose arena is the free TPA region between the
        program image and the stack (sdk/libc/newtos-arch/heap_init.c, called
        from sdk/libc/crt0.S). Ordinary RAM -- see MEM_LOCATION.
*/
void *
portable_malloc(size_t size)
{
    return malloc(size);
}
/* Function: portable_free
        Provide free() functionality in a platform specific way.
*/
void
portable_free(void *p)
{
    free(p);
}
#else
#error "NewTOS has a working malloc; MEM_METHOD must be MEM_MALLOC."
#endif

#if (SEED_METHOD == SEED_VOLATILE)
#if VALIDATION_RUN
volatile ee_s32 seed1_volatile = 0x3415;
volatile ee_s32 seed2_volatile = 0x3415;
volatile ee_s32 seed3_volatile = 0x66;
#endif
#if PERFORMANCE_RUN
volatile ee_s32 seed1_volatile = 0x0;
volatile ee_s32 seed2_volatile = 0x0;
volatile ee_s32 seed3_volatile = 0x66;
#endif
#if PROFILE_RUN
volatile ee_s32 seed1_volatile = 0x8;
volatile ee_s32 seed2_volatile = 0x8;
volatile ee_s32 seed3_volatile = 0x8;
#endif
volatile ee_s32 seed4_volatile = ITERATIONS;
volatile ee_s32 seed5_volatile = 0;
#else
#error "NewTOS cannot pass a command tail interactively; use SEED_VOLATILE."
#endif

/* Porting: Timing functions
        How to capture time and convert to seconds must be ported to whatever
   is supported by the platform. e.g. Read value from on board RTC, read value
   from cpu clock cycles performance counter etc.
*/
/* Define: TIMER_RES_DIVIDER
        Divider to trade off timer resolution and total time that can be
   measured.

        1 here: CLOCKS_PER_SEC is 200 on NewTOS (as on TOS), so the resolution
        is already only 5 ms -- there is nothing to trade away. Over the
        rule-mandated >= 10 s run that is <= 0.05 % of the measured time.
        clock_t is a signed 32-bit count of 200 Hz ticks, so the difference
        below is exact for any run shorter than ~124 days.
        */
#if USE_CLOCK
#define NSECS_PER_SEC             CLOCKS_PER_SEC
#define EE_TIMER_TICKER_RATE      1000
#define CORETIMETYPE              clock_t
#define GETMYTIME(_t)             (*_t = clock())
#define MYTIMEDIFF(fin, ini)      ((fin) - (ini))
#define TIMER_RES_DIVIDER         1
#define SAMPLE_TIME_IMPLEMENTATION 1
#else
#define SAMPLE_TIME_IMPLEMENTATION 0
#endif
#define EE_TICKS_PER_SEC (NSECS_PER_SEC / TIMER_RES_DIVIDER)

#if SAMPLE_TIME_IMPLEMENTATION
/** Define Host specific (POSIX), or target specific global time variables. */
static CORETIMETYPE start_time_val, stop_time_val;

/* Function: start_time
        This function will be called right before starting the timed portion of
   the benchmark.
*/
void
start_time(void)
{
    GETMYTIME(&start_time_val);
}
/* Function: stop_time
        This function will be called right after ending the timed portion of the
   benchmark.
*/
void
stop_time(void)
{
    GETMYTIME(&stop_time_val);
}
/* Function: get_time
        Return an abstract "ticks" number that signifies time on the system.

        200 Hz ticks here, and always as a difference: mintlib's clock() counts
        from BOOT on NewTOS (our crt0 does not run .init_array, so time/clock.c's
        _starttime constructor never sets the origin). The offset cancels in
        MYTIMEDIFF, which is why this port must not use clock() absolutely.
*/
CORE_TICKS
get_time(void)
{
    CORE_TICKS elapsed = (CORE_TICKS)(MYTIMEDIFF(stop_time_val, start_time_val));
    return elapsed;
}
/* Function: time_in_secs
        Convert the value returned by get_time to seconds.

        The <secs_ret> type is used to accommodate systems with no support for
   floating point. Default implementation implemented by the EE_TICKS_PER_SEC
   macro above.
*/
secs_ret
time_in_secs(CORE_TICKS ticks)
{
    secs_ret retval = ((secs_ret)ticks) / (secs_ret)EE_TICKS_PER_SEC;
    return retval;
}
#else
#error "Please implement timing functionality in core_portme.c"
#endif /* SAMPLE_TIME_IMPLEMENTATION */

ee_u32 default_num_contexts = MULTITHREAD;

/* Function: portable_init
        Target specific initialization code
        Test for some common mistakes.
*/
void
portable_init(core_portable *p, int *argc, char *argv[])
{
#if PRINT_ARGS
    int i;
    for (i = 0; i < *argc; i++)
    {
        ee_printf("Arg[%d]=%s\n", i, argv[i]);
    }
#else
    (void)argc;
    (void)argv;
#endif

    /* NewTOS-specific: a banner, printed before the timed portion begins.
       A CoreMark run is >= 10 s of silence by construction, and on NewTOS the
       console program owns the whole screen while it runs -- without this the
       machine looks hung. This is target-specific init code (CoreMark run
       rules: changing the implementation in core_portme.c is allowed) and it
       runs long before start_time(), so it cannot influence the score. */
    ee_printf("CoreMark 1.0 -- NewTOS/AArch64 (%s run)\n",
#if VALIDATION_RUN
              "validation"
#elif PROFILE_RUN
              "profile"
#else
              "performance"
#endif
    );
    ee_printf("Benchmarking, about half a minute. The screen stays empty\n"
              "until the result appears.\n\n");

    if (sizeof(ee_ptr_int) != sizeof(ee_u8 *))
    {
        ee_printf(
            "ERROR! Please define ee_ptr_int to a type that holds a pointer!\n");
    }
    if (sizeof(ee_u32) != 4)
    {
        ee_printf("ERROR! Please define ee_u32 to a 32b unsigned type!\n");
    }
#if (MAIN_HAS_NOARGC && (SEED_METHOD == SEED_ARG))
    ee_printf("ERROR! Main has no argc, but SEED_METHOD defined to SEED_ARG!\n");
#endif

    p->portable_id = 1;
}
/* Function: portable_fini
        Target specific final code
*/
void
portable_fini(core_portable *p)
{
    p->portable_id = 0;

    /* NewTOS-specific: flush stdout before returning to the launcher.
       sdk/libc/crt0.S calls GEMDOS Pterm directly with main's return value; it
       does not go through exit(), so the stdio atexit flush never runs. stdout
       is line buffered here (the console is a character device, see
       sdk/libc/newtos-arch/do_fstat.c) and every CoreMark result line ends in
       '\n', so nothing is pending in practice -- but the LAST line of a report
       is exactly the one that must not be lost, and a flush costs nothing
       outside the timed region. */
    fflush(stdout);

    /* NewTOS-specific: keep the report readable when started from the
       desktop (GitHub #12, TASK/DESK70). Neither EmuTOS' desktop nor ours
       waits after a text program ends -- the desktop redraws at once
       (measurements/measurement-2026-11-07-desk70/RUNINFO.TXT) -- so the
       program has to wait itself. Like the flush above, this runs after
       stop_time() and after the last result line: it cannot touch the score.

       It must NOT wait when run from \AUTO\ at boot: measurement images
       start CORE.NCP / COREVAL.NCP there and nobody presses a key.
       The test is the current directory, Dgetpath(buf, 0):
         - An \AUTO\ program starts in the ROOT of the boot drive. EmuTOS'
           run_auto_program sets no directory (bios/bios.c:685-695), and the
           child inherits initial_basepage's drive and directories
           (bdos/proc.c:396-410), all roots. Spec:
           docs/api/gemdos/process/_run-ownership-curdir.md, sections 4.2
           and 6.5. Dgetpath returns "" for the root
           (docs/api/gemdos/directory/47-dgetpath.md, "Output buffer").
         - The desktop sets the current directory to the application's
           directory (or the top window's) before it launches the program
           (desk/desksupp.c:880-915), and the program inherits it through
           shel_write's sh_apdir (aes/gemshlib.c:109-125,174-175; spec
           section 4.4).
       So the root means "do not wait". A program started from \AUTO\ never
       waits, and a desktop launch from C:\COREMARK\ (where
       tools/make-sd-image.sh installs it) always does. What the test
       cannot tell apart: a desktop launch of a copy in the root does not
       wait (the old behaviour), and a start from EmuCON in a subdirectory
       does wait. Crawcin: any key, no echo, and no Ctrl-C termination, so
       the exit code stays the benchmark's. */
    {
        char cwd[128]; /* Dgetpath writes at most LEN_ZPATH = 114 bytes */

        if (Dgetpath(cwd, 0) == 0 && cwd[0] != '\0')
        {
            ee_printf("\nPress any key to exit.\n");
            fflush(stdout);
            (void)Crawcin();
        }
    }
}

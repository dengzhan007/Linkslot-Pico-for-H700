/*
 * Bind the H700 newlib adapter to the pinned LK heap API.
 *
 * Platform callbacks below intentionally remain external: H700 must supply
 * verified heap-init, secondary-core, console, RTC and fatal/panic services.
 * This file contains no MMIO addresses and does not start/park any CPU.
 */
#include "newlib_syscalls.h"

#include <lib/heap.h>

extern bool h700_platform_heap_ready(void);
extern bool h700_platform_single_cpu_active(void);
extern int h700_platform_console_read(char *buffer, size_t length);
extern int h700_platform_console_write(const char *buffer, size_t length);
extern int h700_platform_rtc_gettimeofday(struct timeval *tv);
extern void h700_platform_fatal(int reason_or_status);

int h700_newlib_install_lk_bindings(void)
{
    static const struct h700_newlib_ops ops = {
        .heap_alloc = heap_alloc,
        .heap_free = heap_free,
        .heap_ready = h700_platform_heap_ready,
        .single_cpu_active = h700_platform_single_cpu_active,
        .console_read = h700_platform_console_read,
        .console_write = h700_platform_console_write,
        .rtc_gettimeofday = h700_platform_rtc_gettimeofday,
        .fatal = h700_platform_fatal,
    };

    return h700_newlib_install(&ops);
}

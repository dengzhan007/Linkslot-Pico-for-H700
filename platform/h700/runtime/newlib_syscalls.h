#ifndef H700_NEWLIB_SYSCALLS_H
#define H700_NEWLIB_SYSCALLS_H

#include <stdbool.h>
#include <stddef.h>
#include <sys/time.h>

struct h700_newlib_ops {
    void *(*heap_alloc)(size_t size, unsigned int alignment);
    void (*heap_free)(void *ptr);
    bool (*heap_ready)(void);
    bool (*single_cpu_active)(void);
    int (*console_read)(char *buffer, size_t length);
    int (*console_write)(const char *buffer, size_t length);
    int (*rtc_gettimeofday)(struct timeval *tv);
    void (*fatal)(int reason_or_status);
};

/*
 * Install once before Linkslot/mGBA starts. The caller must provide a
 * heap_ready callback that reflects heap_init completion and a
 * single_cpu_active callback that is true only while no other CPU can touch
 * the LK heap. The adapter deliberately fails allocation closed otherwise.
 */
int h700_newlib_install(const struct h700_newlib_ops *ops);

/* Install the table using the pinned LK heap and H700 platform callbacks. */
int h700_newlib_install_lk_bindings(void);

#endif

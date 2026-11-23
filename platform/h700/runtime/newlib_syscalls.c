/*
 * H700 / LK newlib syscall and allocator adapter.
 *
 * This is an integration boundary, not a complete H700 runtime. The platform
 * must install services before starting Linkslot/mGBA. Heap allocation is
 * fail-closed unless heap_init has completed AND every secondary CPU remains
 * parked for the lifetime of this allocator. The pinned LK allocator masks
 * interrupts locally; it is not SMP-safe.
 */
#include "newlib_syscalls.h"

#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>

#define H700_FATAL_HEAP_UNSAFE 0x7001

static struct h700_newlib_ops g_ops;
static int g_ops_installed;

int h700_newlib_install(const struct h700_newlib_ops *ops)
{
    if (!ops || g_ops_installed ||
        !ops->heap_alloc || !ops->heap_realloc || !ops->heap_free ||
        !ops->heap_ready || !ops->single_cpu_active || !ops->fatal) {
        errno = EINVAL;
        return -1;
    }

    /* Must be called once during single-threaded platform initialization. */
    g_ops = *ops;
    g_ops_installed = 1;
    return 0;
}

static int heap_is_safe(void)
{
    return g_ops_installed &&
           g_ops.heap_ready() &&
           g_ops.single_cpu_active();
}

static void stop_forever(void)
{
#if defined(__arm__)
    for (;;) {
        __asm__ volatile("wfi");
    }
#else
    __builtin_trap();
#endif
}

static void fatal_if_unsafe_heap(void)
{
    if (g_ops_installed && g_ops.fatal) {
        g_ops.fatal(H700_FATAL_HEAP_UNSAFE);
    }
    stop_forever();
}

void *malloc(size_t size)
{
    void *p;

    if (!heap_is_safe()) {
        errno = ENOMEM;
        return NULL;
    }

    /* LK heap accepts a power-of-two alignment; 16 exceeds AAPCS32 alignment. */
    p = g_ops.heap_alloc(size, 16);
    if (!p) {
        errno = ENOMEM;
    }
    return p;
}

void free(void *ptr)
{
    if (!ptr) {
        return;
    }
    if (!heap_is_safe()) {
        /* Continuing would corrupt the shared LK heap or silently leak. */
        fatal_if_unsafe_heap();
        return;
    }
    g_ops.heap_free(ptr);
}

void *calloc(size_t count, size_t size)
{
    size_t total;
    void *p;

    if (size != 0 && count > SIZE_MAX / size) {
        errno = ENOMEM;
        return NULL;
    }
    total = count * size;
    p = malloc(total);
    if (!p) {
        return NULL;
    }
    memset(p, 0, total);
    return p;
}

void *realloc(void *ptr, size_t size)
{
    void *p;

    if (!ptr) {
        return size ? malloc(size) : NULL;
    }
    if (size == 0) {
        free(ptr);
        return NULL;
    }
    if (!heap_is_safe()) {
        errno = ENOMEM;
        return NULL;
    }

    /*
     * LK heap_realloc(ptr, size) copies the old allocation and frees it.
     * Do not pass NULL: this LK implementation forms an allocation-header
     * pointer before checking ptr, so realloc(NULL, n) must use heap_alloc.
     */
    p = g_ops.heap_realloc(ptr, size);
    if (!p) {
        errno = ENOMEM;
    }
    return p;
}

/* No contiguous newlib heap arena is reserved. malloc-family calls use LK heap. */
void *_sbrk(ptrdiff_t increment)
{
    (void)increment;
    errno = ENOMEM;
    return (void *)-1;
}

int _close(int fd)
{
    if (fd >= 0 && fd <= 2) {
        /* stdin/stdout/stderr are permanent console aliases, not closable files. */
        return 0;
    }
    errno = EBADF;
    return -1;
}

int _fstat(int fd, struct stat *st)
{
    if (!st) {
        errno = EFAULT;
        return -1;
    }
    if (fd < 0 || fd > 2) {
        errno = EBADF;
        return -1;
    }

    memset(st, 0, sizeof(*st));
    st->st_mode = S_IFCHR;
    st->st_nlink = 1;
    return 0;
}

int _isatty(int fd)
{
    if (fd >= 0 && fd <= 2) {
        return 1;
    }
    errno = EBADF;
    return 0;
}

off_t _lseek(int fd, off_t offset, int whence)
{
    (void)offset;
    (void)whence;
    errno = (fd >= 0 && fd <= 2) ? ESPIPE : EBADF;
    return (off_t)-1;
}

int _read(int fd, char *buffer, int length)
{
    int result;

    if (fd != 0) {
        errno = EBADF;
        return -1;
    }
    if (length < 0 || (!buffer && length != 0)) {
        errno = EINVAL;
        return -1;
    }
    if (length == 0) {
        return 0;
    }
    if (!g_ops_installed || !g_ops.console_read) {
        errno = ENOSYS;
        return -1;
    }

    errno = 0;
    result = g_ops.console_read(buffer, (size_t)length);
    if (result < 0) {
        if (errno == 0) {
            errno = EIO;
        }
        return -1;
    }
    if (result > length) {
        errno = EIO;
        return -1;
    }
    return result;
}

int _write(int fd, const char *buffer, int length)
{
    int result;

    if (fd != 1 && fd != 2) {
        errno = EBADF;
        return -1;
    }
    if (length < 0 || (!buffer && length != 0)) {
        errno = EINVAL;
        return -1;
    }
    if (length == 0) {
        return 0;
    }
    if (!g_ops_installed || !g_ops.console_write) {
        errno = ENOSYS;
        return -1;
    }

    errno = 0;
    result = g_ops.console_write(buffer, (size_t)length);
    if (result < 0) {
        if (errno == 0) {
            errno = EIO;
        }
        return -1;
    }
    if (result > length) {
        errno = EIO;
        return -1;
    }
    return result;
}

int _gettimeofday(struct timeval *tv, void *tz)
{
    (void)tz;

    if (!tv) {
        errno = EFAULT;
        return -1;
    }
    if (!g_ops_installed || !g_ops.rtc_gettimeofday) {
        /* Monotonic LK ticks are not Unix wall-clock time. */
        errno = ENOSYS;
        return -1;
    }
    errno = 0;
    if (g_ops.rtc_gettimeofday(tv) != 0) {
        if (errno == 0) {
            errno = EIO;
        }
        return -1;
    }
    return 0;
}

int _getpid(void)
{
    /* Single bare-metal runtime; this is an ABI token, not process isolation. */
    return 1;
}

int _kill(int pid, int sig)
{
    (void)sig;
    errno = (pid == 1) ? ENOSYS : ESRCH;
    return -1;
}

void _exit(int status)
{
    if (g_ops_installed && g_ops.fatal) {
        g_ops.fatal(status);
    }
    stop_forever();
}

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
#include <reent.h>
#include <limits.h>
#include <stdint.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/types.h>

#define H700_FATAL_HEAP_UNSAFE 0x7001
#define H700_ALLOC_MAGIC 0x48373030u
#define H700_LK_HEAP_INTERNAL_OVERHEAD 128u

typedef struct __attribute__((aligned(16))) {
    size_t size;
    uint32_t magic;
    uint32_t reserved[2];
} h700_alloc_header_t;

_Static_assert((sizeof(h700_alloc_header_t) % 16) == 0,
               "H700 allocation header must preserve 16-byte alignment");

static h700_alloc_header_t *allocation_header(void *ptr)
{
    return ((h700_alloc_header_t *)ptr) - 1;
}

static struct h700_newlib_ops g_ops;
static int g_ops_installed;

int h700_newlib_install(const struct h700_newlib_ops *ops)
{
    if (!ops || g_ops_installed ||
        !ops->heap_alloc || !ops->heap_free ||
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

    /* A private aligned header retains the requested size and raw LK pointer. */
    if (size > SIZE_MAX - sizeof(h700_alloc_header_t) - H700_LK_HEAP_INTERNAL_OVERHEAD) {
        errno = ENOMEM;
        return NULL;
    }
    p = g_ops.heap_alloc(sizeof(h700_alloc_header_t) + size, 16);
    if (!p) {
        errno = ENOMEM;
        return NULL;
    }
    h700_alloc_header_t *header = (h700_alloc_header_t *)p;
    header->size = size;
    header->magic = H700_ALLOC_MAGIC;
    header->reserved[0] = 0;
    header->reserved[1] = 0;
    return (void *)(header + 1);
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
    h700_alloc_header_t *header = allocation_header(ptr);
    if (header->magic != H700_ALLOC_MAGIC) {
        fatal_if_unsafe_heap();
        return;
    }
    header->magic = 0;
    g_ops.heap_free(header);
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

    h700_alloc_header_t *old_header = allocation_header(ptr);
    if (old_header->magic != H700_ALLOC_MAGIC) {
        fatal_if_unsafe_heap();
        return NULL;
    }

    /*
     * The pinned LK heap_realloc() internally calls heap_alloc(size, 0),
     * losing the caller's requested alignment. Allocate a new 16-byte-aligned
     * block, copy only the recorded user length, then free the old raw block.
     * On allocation failure the old block remains untouched.
     */
    p = malloc(size);
    if (!p) {
        return NULL;
    }
    size_t copy_size = old_header->size < size ? old_header->size : size;
    memcpy(p, ptr, copy_size);
    free(ptr);
    return p;
}

/*
 * Newlib's public malloc/free/realloc/calloc wrappers call their reentrant
 * counterparts. Override those as well, so internal libc callers cannot
 * fall back to newlib's separate _sbrk_r-backed heap.
 */
void *_malloc_r(struct _reent *reent, size_t size)
{
    void *p = malloc(size);
    if (!p && reent) {
        reent->_errno = errno ? errno : ENOMEM;
    }
    return p;
}

void *_calloc_r(struct _reent *reent, size_t count, size_t size)
{
    void *p = calloc(count, size);
    if (!p && reent) {
        reent->_errno = errno ? errno : ENOMEM;
    }
    return p;
}

void *_realloc_r(struct _reent *reent, void *ptr, size_t size)
{
    void *p = realloc(ptr, size);
    if (!p && size != 0 && reent) {
        reent->_errno = errno ? errno : ENOMEM;
    }
    return p;
}

void _free_r(struct _reent *reent, void *ptr)
{
    (void)reent;
    free(ptr);
}

/*
 * The reentrant wrappers above use only the LK heap, whose free-list access
 * is protected inside heap_alloc/heap_free. The H700 build must keep secondary
 * CPUs parked until an SMP-safe heap lock is available. No separate newlib
 * malloc pool is used, so these hooks intentionally carry no second lock.
 */
void __malloc_lock(struct _reent *reent)
{
    (void)reent;
}

void __malloc_unlock(struct _reent *reent)
{
    (void)reent;
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

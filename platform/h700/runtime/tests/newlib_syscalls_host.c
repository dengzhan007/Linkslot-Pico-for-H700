#include "../newlib_syscalls.h"

#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <setjmp.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/time.h>
#include <reent.h>

extern void *h700_test_malloc(size_t);
extern void *h700_test_calloc(size_t, size_t);
extern void *h700_test_realloc(void *, size_t);
extern void h700_test_free(void *);
extern void *h700_test_sbrk(ptrdiff_t);
extern int h700_test_close(int);
extern int h700_test_fstat(int, struct stat *);
extern int h700_test_isatty(int);
extern off_t h700_test_lseek(int, off_t, int);
extern int h700_test_read(int, char *, int);
extern int h700_test_write(int, const char *, int);
extern int h700_test_gettimeofday(struct timeval *, void *);
extern int h700_test_getpid(void);
extern int h700_test_kill(int, int);
extern void *h700_test_malloc_r(struct _reent *, size_t);
extern void *h700_test_calloc_r(struct _reent *, size_t, size_t);
extern void *h700_test_realloc_r(struct _reent *, void *, size_t);
extern void h700_test_free_r(struct _reent *, void *);
extern void h700_test_malloc_lock(struct _reent *);
extern void h700_test_malloc_unlock(struct _reent *);
extern void h700_test_exit(int);

static int g_heap_ready;
static int g_fail_alloc;
static jmp_buf g_fatal_jump;
static int g_expect_fatal;
static int g_single_cpu;
static unsigned g_allocs, g_frees;
static unsigned g_fatal_calls;
static int g_read_mode;
static int g_time_mode;
static char g_output[256];
static size_t g_output_len;

static void *fake_heap_alloc(size_t size, unsigned int alignment)
{
    void *p = NULL;
    g_allocs++;
    if (g_fail_alloc) return NULL;
    if (alignment > 16 || (alignment & (alignment - 1)) != 0) {
        return NULL;
    }
    if (posix_memalign(&p, alignment < sizeof(void *) ? sizeof(void *) : alignment,
                       size ? size : 1) != 0) {
        return NULL;
    }
    return p;
}

static void fake_heap_free(void *ptr)
{
    g_frees++;
    free(ptr);
}

static bool fake_heap_ready(void) { return g_heap_ready != 0; }
static bool fake_single_cpu(void) { return g_single_cpu != 0; }

static int fake_console_write(const char *buffer, size_t length)
{
    if (length > sizeof(g_output) - g_output_len) {
        errno = ENOSPC;
        return -1;
    }
    memcpy(g_output + g_output_len, buffer, length);
    g_output_len += length;
    return (int)length;
}

static int fake_console_read(char *buffer, size_t length)
{
    static const char input[] = "xyz";
    size_t n;
    if (g_read_mode == 1) {
        errno = EAGAIN;
        return -1;
    }
    if (g_read_mode == 2) {
        errno = 0;
        return -1;
    }
    n = length < sizeof(input) - 1 ? length : sizeof(input) - 1;
    memcpy(buffer, input, n);
    return (int)n;
}

static int fake_rtc_gettimeofday(struct timeval *tv)
{
    if (g_time_mode) {
        errno = EIO;
        return -1;
    }
    tv->tv_sec = 123456789;
    tv->tv_usec = 321000;
    return 0;
}

static void fake_fatal(int reason)
{
    (void)reason;
    g_fatal_calls++;
    if (g_expect_fatal) {
        longjmp(g_fatal_jump, 1);
    }
}

/*
 * Host stand-ins for the pinned LK heap and the future H700 platform callbacks.
 * The test exercises the real lk_bindings.c table construction; it does not
 * claim these stand-ins are safe target implementations.
 */
void *heap_alloc(size_t size, unsigned int alignment)
{
    return fake_heap_alloc(size, alignment);
}

void heap_free(void *ptr)
{
    fake_heap_free(ptr);
}

bool h700_platform_heap_ready(void)
{
    return fake_heap_ready();
}

bool h700_platform_single_cpu_active(void)
{
    return fake_single_cpu();
}

int h700_platform_console_read(char *buffer, size_t length)
{
    return fake_console_read(buffer, length);
}

int h700_platform_console_write(const char *buffer, size_t length)
{
    return fake_console_write(buffer, length);
}

int h700_platform_rtc_gettimeofday(struct timeval *tv)
{
    return fake_rtc_gettimeofday(tv);
}

void h700_platform_fatal(int reason_or_status)
{
    fake_fatal(reason_or_status);
}

#define CHECK(cond, message) do { \
    if (!(cond)) { \
        fprintf(stderr, "FAIL: %s (line %d, errno=%d)\n", message, __LINE__, errno); \
        return __LINE__; \
    } \
} while (0)

int main(void)
{
    struct h700_newlib_ops bad = {0};
    struct stat st;
    struct timeval tv;
    struct _reent reent = {0};
    char input[8] = {0};
    char *p, *q;
    unsigned alloc_before, free_before;

    errno = 0;
    CHECK(h700_test_malloc(16) == NULL && errno == ENOMEM,
          "allocation fails closed before service installation");
    reent._errno = 0;
    CHECK(h700_test_malloc_r(&reent, 16) == NULL && reent._errno == ENOMEM,
          "newlib _malloc_r fails closed and sets the passed reent errno");
    errno = 0;
    CHECK(h700_newlib_install(&bad) == -1 && errno == EINVAL,
          "reject incomplete platform services");
    CHECK(h700_newlib_install_lk_bindings() == 0,
          "install services through the real LK binding table");
    errno = 0;
    CHECK(h700_newlib_install_lk_bindings() == -1 && errno == EINVAL,
          "LK binding table cannot be installed twice");

    errno = 0;
    CHECK(h700_test_malloc(16) == NULL && errno == ENOMEM,
          "allocation blocked before LK heap_init");
    g_heap_ready = 1;
    errno = 0;
    CHECK(h700_test_malloc(16) == NULL && errno == ENOMEM,
          "allocation blocked while secondary CPUs are not parked");

    g_single_cpu = 1;
    reent._errno = 0;
    q = (char *)h700_test_calloc_r(&reent, 2, 16);
    CHECK(q != NULL && reent._errno == 0, "newlib _calloc_r uses LK allocator");
    for (unsigned i = 0; i < 32; ++i) {
        CHECK(q[i] == 0, "newlib _calloc_r zeroes its allocation");
    }
    h700_test_free_r(&reent, q);
    q = (char *)h700_test_malloc_r(&reent, 16);
    CHECK(q != NULL, "newlib _malloc_r allocates through LK heap");
    strcpy(q, "reent");
    reent._errno = 0;
    g_fail_alloc = 1;
    CHECK(h700_test_realloc_r(&reent, q, 64) == NULL &&
          reent._errno == ENOMEM && !strcmp(q, "reent"),
          "newlib _realloc_r preserves old allocation and reent errno on failure");
    g_fail_alloc = 0;
    char *rq = (char *)h700_test_realloc_r(&reent, q, 48);
    CHECK(rq != NULL && !strcmp(rq, "reent") &&
          ((uintptr_t)rq & 15u) == 0,
          "newlib _realloc_r preserves content and alignment");
    h700_test_free_r(&reent, rq);
    reent._errno = 0;
    CHECK(h700_test_calloc_r(&reent, SIZE_MAX, 2) == NULL &&
          reent._errno == ENOMEM, "newlib _calloc_r propagates overflow errno");
    h700_test_malloc_lock(&reent);
    h700_test_malloc_unlock(&reent);

    p = (char *)h700_test_malloc(32);
    CHECK(p != NULL && ((uintptr_t)p & 15u) == 0,
          "malloc requests and receives 16-byte alignment");
    memset(p, 0x5a, 32);
    CHECK(((uintptr_t)p & 15u) == 0, "malloc result remains 16-byte aligned");

    q = (char *)h700_test_calloc(4, 8);
    CHECK(q != NULL, "calloc succeeds");
    for (unsigned i = 0; i < 32; ++i) {
        CHECK(q[i] == 0, "calloc zeroes allocation");
    }

    errno = 0;
    CHECK(h700_test_calloc(SIZE_MAX, 2) == NULL && errno == ENOMEM,
          "calloc detects multiplication overflow");

    h700_test_free(q);
    alloc_before = g_allocs;
    free_before = g_frees;
    q = (char *)h700_test_realloc(NULL, 24);
    CHECK(q != NULL && g_allocs == alloc_before + 1,
          "realloc(NULL,n) uses aligned LK heap_alloc");

    strcpy(q, "old");
    alloc_before = g_allocs;
    free_before = g_frees;
    g_fail_alloc = 1;
    errno = 0;
    CHECK(h700_test_realloc(q, 128) == NULL && errno == ENOMEM &&
          !strcmp(q, "old"), "failed realloc leaves the original allocation intact");
    g_fail_alloc = 0;

    char *grown = (char *)h700_test_realloc(q, 64);
    CHECK(grown != NULL && !strcmp(grown, "old"),
          "realloc preserves original contents");
    CHECK(((uintptr_t)grown & 15u) == 0,
          "realloc preserves 16-byte newlib alignment");
    CHECK(g_allocs == alloc_before + 2 && g_frees == free_before + 1,
          "realloc allocates aligned replacement and releases old raw block");

    free_before = g_frees;
    h700_test_free(NULL);
    CHECK(g_frees == free_before, "free(NULL) is a no-op");
    h700_test_free(p);

    errno = 0;
    CHECK(h700_test_sbrk(4096) == (void *)-1 && errno == ENOMEM,
          "_sbrk refuses an unreserved contiguous arena");

    CHECK(h700_test_write(1, "out", 3) == 3, "stdout write");
    CHECK(h700_test_write(2, "err", 3) == 3, "stderr write");
    CHECK(g_output_len == 6 && !memcmp(g_output, "outerr", 6),
          "console writes reach installed sink");
    errno = 0;
    CHECK(h700_test_write(3, "bad", 3) == -1 && errno == EBADF,
          "reject unsupported file descriptors");

    CHECK(h700_test_read(0, input, sizeof(input)) == 3 &&
          !memcmp(input, "xyz", 3), "stdin read");
    g_read_mode = 1;
    errno = 0;
    CHECK(h700_test_read(0, input, sizeof(input)) == -1 && errno == EAGAIN,
          "propagate nonblocking console EAGAIN");
    g_read_mode = 2;
    errno = 0;
    CHECK(h700_test_read(0, input, sizeof(input)) == -1 && errno == EIO,
          "map missing callback errno to EIO");

    CHECK(h700_test_close(0) == 0 && h700_test_close(1) == 0 &&
          h700_test_close(2) == 0, "standard console descriptors are persistent");
    errno = 0;
    CHECK(h700_test_close(7) == -1 && errno == EBADF,
          "close rejects unknown descriptor");

    CHECK(h700_test_fstat(0, &st) == 0 && S_ISCHR(st.st_mode),
          "fstat reports standard streams as character devices");
    errno = 0;
    CHECK(h700_test_fstat(4, &st) == -1 && errno == EBADF,
          "fstat rejects unsupported descriptor");
    CHECK(h700_test_isatty(2) == 1, "isatty standard error");
    errno = 0;
    CHECK(h700_test_isatty(8) == 0 && errno == EBADF,
          "isatty rejects unknown descriptor");
    errno = 0;
    CHECK(h700_test_lseek(0, 0, SEEK_SET) == (off_t)-1 && errno == ESPIPE,
          "console is not seekable");
    errno = 0;
    CHECK(h700_test_lseek(9, 0, SEEK_SET) == (off_t)-1 && errno == EBADF,
          "lseek rejects unknown descriptor");

    CHECK(h700_test_gettimeofday(&tv, NULL) == 0 &&
          tv.tv_sec == 123456789 && tv.tv_usec == 321000,
          "gettimeofday delegates to installed RTC callback");
    g_time_mode = 1;
    errno = 0;
    CHECK(h700_test_gettimeofday(&tv, NULL) == -1 && errno == EIO,
          "propagate RTC failure");

    CHECK(h700_test_getpid() == 1, "single bare-metal pid token");
    errno = 0;
    CHECK(h700_test_kill(1, 15) == -1 && errno == ENOSYS,
          "signal delivery explicitly unsupported");
    errno = 0;
    CHECK(h700_test_kill(2, 15) == -1 && errno == ESRCH,
          "unknown pid rejected");

    g_expect_fatal = 1;
    if (setjmp(g_fatal_jump) == 0) {
        g_single_cpu = 0;
        h700_test_free(grown);
        CHECK(0, "free must not proceed while secondary CPUs are active");
    }
    CHECK(g_fatal_calls == 1,
          "unsafe heap free is routed to the fatal/panic hook");
    g_single_cpu = 1;
    h700_test_free(grown);
    g_expect_fatal = 1;
    if (setjmp(g_fatal_jump) == 0) {
        h700_test_exit(23);
        CHECK(0, "_exit must be non-returning");
    }
    CHECK(g_fatal_calls == 2, "_exit forwards status to fatal/panic hook");

    puts("PASS: H700 newlib allocator gate and 11 syscall hooks");
    return 0;
}

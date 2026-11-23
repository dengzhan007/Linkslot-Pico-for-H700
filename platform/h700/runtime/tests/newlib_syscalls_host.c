#include "../newlib_syscalls.h"

#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/time.h>

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

static int g_heap_ready;
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
    struct h700_newlib_ops ops = {
        .heap_alloc = fake_heap_alloc,
        .heap_free = fake_heap_free,
        .heap_ready = fake_heap_ready,
        .single_cpu_active = fake_single_cpu,
        .console_read = fake_console_read,
        .console_write = fake_console_write,
        .rtc_gettimeofday = fake_rtc_gettimeofday,
        .fatal = fake_fatal,
    };
    struct stat st;
    struct timeval tv;
    char input[8] = {0};
    char *p, *q;
    unsigned alloc_before, free_before;

    errno = 0;
    CHECK(h700_test_malloc(16) == NULL && errno == ENOMEM,
          "allocation fails closed before service installation");
    errno = 0;
    CHECK(h700_newlib_install(&bad) == -1 && errno == EINVAL,
          "reject incomplete platform services");
    CHECK(h700_newlib_install(&ops) == 0, "install valid services");

    errno = 0;
    CHECK(h700_test_malloc(16) == NULL && errno == ENOMEM,
          "allocation blocked before LK heap_init");
    g_heap_ready = 1;
    errno = 0;
    CHECK(h700_test_malloc(16) == NULL && errno == ENOMEM,
          "allocation blocked while secondary CPUs are not parked");

    g_single_cpu = 1;
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
    char *grown = (char *)h700_test_realloc(q, 64);
    CHECK(grown != NULL && !strcmp(grown, "old"),
          "realloc preserves original contents");
    CHECK(((uintptr_t)grown & 15u) == 0,
          "realloc preserves 16-byte newlib alignment");
    CHECK(g_allocs == alloc_before + 1 && g_frees == free_before + 1,
          "realloc allocates an aligned block and releases the old raw block");

    free_before = g_frees;
    h700_test_free(grown);
    h700_test_free(NULL);
    CHECK(g_frees == free_before + 1, "free(NULL) is a no-op");
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

    puts("PASS: H700 newlib allocator gate and 11 syscall hooks");
    return 0;
}

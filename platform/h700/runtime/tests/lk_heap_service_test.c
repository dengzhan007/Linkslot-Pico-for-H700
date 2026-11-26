#include "lk_heap_service.h"

#include <assert.h>
#include <stddef.h>
#include <stdio.h>

static size_t g_free_total;
static size_t g_largest;
static unsigned int g_heap_queries;
static unsigned int g_led_early_init_calls;
static unsigned int g_led_ready_queries;

void heap_get_info(size_t *free_total, size_t *largest)
{
    ++g_heap_queries;
    if (free_total != NULL)
        *free_total = g_free_total;
    if (largest != NULL)
        *largest = g_largest;
}

int h700_rg34xx_power_led_early_init(void)
{
    ++g_led_early_init_calls;
    return 0;
}

int h700_rg34xx_power_led_is_ready(void)
{
    ++g_led_ready_queries;
    return 1;
}

void target_early_init(void);
void target_init(void);

int main(void)
{
    assert(!h700_platform_heap_ready());
    assert(g_heap_queries == 0u);

    /* The pre-heap target hook may touch the LED but must not query the heap. */
    target_early_init();
    assert(g_led_early_init_calls == 1u);
    assert(g_heap_queries == 0u);
    assert(!h700_platform_heap_ready());
    assert(g_heap_queries == 0u);

    /* target_init is post-heap in the normal pinned LK startup sequence. */
#if defined(ENABLE_NANDWRITE) && ENABLE_NANDWRITE
    /* This alternate LK bootstrap calls target_init without heap_init(). */
    target_init();
    assert(g_led_ready_queries == 1u);
    assert(g_heap_queries == 0u);
    assert(!h700_platform_heap_ready());
    assert(g_heap_queries == 0u);
    puts("PASS: NAND-write bootstrap never marks the uninitialized LK heap ready");
    return 0;
#else
    target_init();
    assert(g_led_ready_queries == 1u);
    assert(g_heap_queries == 0u);
    assert(!h700_platform_heap_ready());
    assert(g_heap_queries == 1u);

    g_free_total = 4096u;
    g_largest = 2048u;
    assert(h700_platform_heap_ready());
    assert(g_heap_queries == 2u);

    /* Do not cache a stale ready result after the heap becomes unavailable. */
    g_free_total = 0u;
    g_largest = 0u;
    assert(!h700_platform_heap_ready());
    assert(g_heap_queries == 3u);

    puts("PASS: LK heap is never queried before target_init");
    puts("PASS: readiness requires the post-heap target lifecycle marker");
    puts("PASS: readiness reflects current free_total/largest values");
    return 0;
#endif
}

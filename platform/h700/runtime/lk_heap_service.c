#include "lk_heap_service.h"

#include <stddef.h>
#include <lib/heap.h>

/* Never touch LK's heap free-list before target_init() marks the post-heap phase. */
static bool g_target_init_after_heap;

void h700_lk_heap_service_mark_target_init_complete(void)
{
    g_target_init_after_heap = true;
}

bool h700_platform_heap_ready(void)
{
    size_t free_total = 0u;
    size_t largest = 0u;

    if (!g_target_init_after_heap)
        return false;

    heap_get_info(&free_total, &largest);
    return free_total != 0u && largest != 0u;
}

#ifndef H700_LK_HEAP_SERVICE_H
#define H700_LK_HEAP_SERVICE_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Called only from target_init(), after heap_init() in the pinned LK flow. */
void h700_lk_heap_service_mark_target_init_complete(void);

/*
 * True only after target_init() has marked the post-heap phase and the LK
 * allocator currently reports a non-empty free list. This is not a memory
 * budget guarantee; callers must still handle allocation failure.
 */
bool h700_platform_heap_ready(void);

#ifdef __cplusplus
}
#endif

#endif

#ifndef H700_ARCH_TIMER_H
#define H700_ARCH_TIMER_H

#include <stdint.h>
#include "time_service.h"

#ifdef __cplusplus
extern "C" {
#endif

#define H700_ARCH_TIMER_STATE_MAGIC 0x4837544Du

enum h700_arch_timer_status {
    H700_ARCH_TIMER_OK = 0,
    H700_ARCH_TIMER_EINVAL = -1,
    H700_ARCH_TIMER_EFREQUENCY = -2
};

typedef struct h700_arch_timer_state {
    uint64_t start_count;
    uint32_t frequency_hz;
    uint32_t magic;
} h700_arch_timer_state_t;

/*
 * Read-only ARM Generic Timer provider. The caller-owned state must remain
 * alive while the service descriptor is in use. No timer/IRQ configuration.
 */
int h700_arch_timer_init(
    h700_arch_timer_state_t *state,
    h700_time_service_v1_t *service
);

uint64_t h700_arch_timer_ticks_to_ms(uint64_t elapsed_ticks, uint32_t frequency_hz);

#ifdef __cplusplus
}
#endif

#endif

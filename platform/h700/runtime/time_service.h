#ifndef H700_TIME_SERVICE_H
#define H700_TIME_SERVICE_H

#include <stddef.h>
#include <stdint.h>

#define H700_TIME_ABI_VERSION 1u

typedef uint64_t (*h700_monotonic_ms_fn)(void *context);

/* Monotonic milliseconds only; never return wall-clock/RTC time. */
typedef struct h700_time_service_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    void *context;
    h700_monotonic_ms_fn monotonic_ms;
} h700_time_service_v1_t;

#endif /* H700_TIME_SERVICE_H */

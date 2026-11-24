#ifndef H700_CPU_PERFORMANCE_RUNTIME_H
#define H700_CPU_PERFORMANCE_RUNTIME_H

#include <stdint.h>

#include "h700_cpu_performance_service.h"

#define H700_CPU_PERF_RUNTIME_ABI_VERSION 1u
#define H700_CPU_PERF_RUNTIME_PROFILE_APPROVED (1u << 0)
#define H700_CPU_PERF_RUNTIME_BOARD_RG34XX 0x52473334u /* "RG34" */

enum h700_cpu_perf_runtime_status {
    H700_CPU_PERF_RUNTIME_OK = 0,
    H700_CPU_PERF_RUNTIME_ERR_UNAVAILABLE = -200,
    H700_CPU_PERF_RUNTIME_ERR_PROFILE = -201,
    H700_CPU_PERF_RUNTIME_ERR_GATES = -202,
    H700_CPU_PERF_RUNTIME_ERR_POLICY = -203,
    H700_CPU_PERF_RUNTIME_ERR_SERVICE = -204
};

/*
 * Immutable, board-owned configuration. A profile must be checked into the
 * H700 board port only after its board revision, CPU rail, speed bin, OPP
 * table, thermal limits and BL31 secondary-core contract have been verified.
 * A runtime profile is not a user-supplied overclock table.
 */
typedef struct h700_cpu_perf_board_profile_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t board_id;
    uint32_t board_revision;
    uint32_t profile_flags;
    uint32_t reserved;
    const struct h700_cpu_opp *opps;
    uint32_t opp_count;
    struct h700_cpu_perf_limits limits;
    uint32_t reserved_words[2];
} h700_cpu_perf_board_profile_v1_t;

/*
 * Initialize the one statically owned H700 CPU service before the Linkslot
 * event loop starts. Passing NULL explicitly leaves the service unavailable.
 * The service storage is invalidated before every attempt, so a rejected
 * profile cannot leave a prior instance active through a stale handle.
 *
 * This is a single-core startup API; do not call it concurrently with a
 * CpuPerformanceClient or from the frame loop.
 */
int h700_cpu_perf_runtime_init_v1(
    const h700_cpu_perf_board_profile_v1_t *profile
);

int h700_cpu_perf_runtime_status_v1(void);

/* Returns NULL until a fully validated, board-approved profile is loaded. */
h700_cpu_perf_service_v1_t *h700_cpu_perf_runtime_get_service_v1(void);

#endif /* H700_CPU_PERFORMANCE_RUNTIME_H */

#ifndef H700_CPU_PERFORMANCE_SERVICE_H
#define H700_CPU_PERFORMANCE_SERVICE_H

#include <stddef.h>
#include <stdint.h>

#include "h700_cpu_performance_policy.h"

#define H700_CPU_PERF_SERVICE_ABI_VERSION 1u
#define H700_CPU_PERF_SERVICE_MAX_OPPS H700_CPU_PERF_MAX_OPPS

#define H700_CPU_PERF_TELEMETRY_CLOCK_VALID   (1u << 0)
#define H700_CPU_PERF_TELEMETRY_VOLTAGE_VALID (1u << 1)
#define H700_CPU_PERF_TELEMETRY_TEMP_VALID    (1u << 2)
#define H700_CPU_PERF_TELEMETRY_VALID_MASK \
    (H700_CPU_PERF_TELEMETRY_CLOCK_VALID | \
     H700_CPU_PERF_TELEMETRY_VOLTAGE_VALID | \
     H700_CPU_PERF_TELEMETRY_TEMP_VALID)

/*
 * The v1 service is intentionally a policy/telemetry boundary only.
 * It never maps MMIO, accesses the PMIC, changes the PLL or claims that a
 * planned OPP has been applied. A future H700 clock/PMIC HAL may implement
 * the actuator behind this ABI after all board and handoff gates are closed.
 */
enum h700_cpu_perf_service_status {
    H700_CPU_PERF_SERVICE_OK = 0,
    H700_CPU_PERF_SERVICE_POLICY_ONLY = 1,
    H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT = -100,
    H700_CPU_PERF_SERVICE_ERR_ABI = -101,
    H700_CPU_PERF_SERVICE_ERR_NOT_INITIALIZED = -102
};

typedef struct h700_cpu_perf_service_config_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    const struct h700_cpu_opp *opps;
    uint32_t opp_count;
    uint32_t reserved;
    struct h700_cpu_perf_limits limits;
} h700_cpu_perf_service_config_v1_t;

typedef struct h700_cpu_perf_telemetry_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t valid_flags;
    uint32_t actual_frequency_hz;
    uint32_t actual_voltage_uv;
    int32_t temperature_mc;
    uint32_t reserved;
} h700_cpu_perf_telemetry_v1_t;

typedef struct h700_cpu_perf_state_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t generation;
    uint32_t requested_mode;
    uint32_t planned_frequency_hz;
    uint32_t planned_voltage_uv;
    uint32_t policy_flags;
    int32_t last_policy_status;
    uint32_t telemetry_valid_flags;
    uint32_t actual_frequency_hz;
    uint32_t actual_voltage_uv;
    int32_t temperature_mc;
    uint32_t hardware_applied;
    uint32_t reserved[3];
} h700_cpu_perf_state_v1_t;

/*
 * Caller-owned service storage. Keep this object static or in long-lived
 * platform-owned memory. The OPP table is copied during init, so the caller's
 * input table may be released afterwards. No heap or threading is required.
 */
typedef struct h700_cpu_perf_service_v1 {
    uint32_t abi_version;
    uint32_t struct_size;
    uint32_t magic;
    uint32_t initialized;
    uint32_t opp_count;
    uint32_t reserved;
    struct h700_cpu_opp opps[H700_CPU_PERF_SERVICE_MAX_OPPS];
    struct h700_cpu_perf_limits limits;
    h700_cpu_perf_state_v1_t state;
} h700_cpu_perf_service_v1_t;

int h700_cpu_perf_service_init_v1(
    h700_cpu_perf_service_v1_t *service,
    uint32_t service_size,
    const h700_cpu_perf_service_config_v1_t *config
);

int h700_cpu_perf_service_update_limits_v1(
    h700_cpu_perf_service_v1_t *service,
    const struct h700_cpu_perf_limits *limits
);

/*
 * Returns POLICY_ONLY when an OPP was selected successfully. This does NOT
 * mean the CPU frequency or voltage changed; inspect state.actual_* telemetry
 * separately. All hardware_applied values remain zero in service ABI v1.
 */
int h700_cpu_perf_service_request_mode_v1(
    h700_cpu_perf_service_v1_t *service,
    enum h700_cpu_perf_mode mode
);

int h700_cpu_perf_service_update_telemetry_v1(
    h700_cpu_perf_service_v1_t *service,
    const h700_cpu_perf_telemetry_v1_t *telemetry
);

int h700_cpu_perf_service_get_state_v1(
    const h700_cpu_perf_service_v1_t *service,
    h700_cpu_perf_state_v1_t *state,
    uint32_t state_size
);

#endif /* H700_CPU_PERFORMANCE_SERVICE_H */

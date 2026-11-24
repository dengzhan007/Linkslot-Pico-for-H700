#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_cpu_performance_service.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        return EXIT_FAILURE; \
    } \
} while (0)

static const struct h700_cpu_opp test_opps[] = {
    { 600000000u,  { 900000u, 900000u, 0u, 0u, 0u, 0u }, 0x03u, 0u, 0u },
    { 800000000u,  { 950000u, 950000u, 0u, 0u, 0u, 0u }, 0x03u, 0u, 0u },
    { 1000000000u, { 1000000u, 1000000u, 0u, 0u, 0u, 0u }, 0x03u, 0u, 0u },
    { 1200000000u, { 1050000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u },
    { 1400000000u, { 1100000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u }
};

static struct h700_cpu_perf_limits ready_limits(void)
{
    struct h700_cpu_perf_limits limits;
    memset(&limits, 0, sizeof(limits));
    limits.verified_gates = H700_CPU_PERF_REQUIRED_GATES;
    limits.board_max_voltage_uv = 1160000u;
    limits.thermal_max_frequency_hz = 1500000000u;
    limits.auto_target_frequency_hz = 1050000000u;
    limits.speed_bin = 0u;
    return limits;
}

int main(void)
{
    h700_cpu_perf_service_v1_t service;
    h700_cpu_perf_service_config_v1_t config;
    h700_cpu_perf_state_v1_t state;
    h700_cpu_perf_telemetry_v1_t telemetry;
    struct h700_cpu_opp bad_opps[sizeof(test_opps) / sizeof(test_opps[0])];
    int rc;

    memset(&service, 0, sizeof(service));
    memset(&config, 0, sizeof(config));
    config.abi_version = H700_CPU_PERF_SERVICE_ABI_VERSION;
    config.struct_size = (uint32_t)sizeof(config);
    config.opps = test_opps;
    config.opp_count = (uint32_t)(sizeof(test_opps) / sizeof(test_opps[0]));
    config.limits = ready_limits();

    CHECK(h700_cpu_perf_service_init_v1(&service, (uint32_t)sizeof(service) - 1u,
          &config) == H700_CPU_PERF_SERVICE_ERR_ABI,
          "undersized service storage is rejected");
    config.abi_version++;
    CHECK(h700_cpu_perf_service_init_v1(&service, (uint32_t)sizeof(service),
          &config) == H700_CPU_PERF_SERVICE_ERR_ABI,
          "unknown service ABI version is rejected");
    config.abi_version = H700_CPU_PERF_SERVICE_ABI_VERSION;

    CHECK(h700_cpu_perf_service_init_v1(&service, (uint32_t)sizeof(service),
          &config) == H700_CPU_PERF_SERVICE_OK,
          "service initializes with copied board profile");
    CHECK(h700_cpu_perf_service_get_state_v1(&service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.hardware_applied == 0u &&
          state.planned_frequency_hz == 0u,
          "initial state does not claim an applied clock");

    CHECK(h700_cpu_perf_service_request_mode_v1(&service, H700_CPU_PERF_MODE_PERFORMANCE) ==
          H700_CPU_PERF_STATUS_OK,
          "mode request is blocked while evidence gates are not ready");
    CHECK(h700_cpu_perf_service_get_state_v1(&service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.hardware_applied == 0u &&
          state.planned_frequency_hz == 0u,
          "blocked request clears its stale planned OPP");

    config.limits = ready_limits();
    CHECK(h700_cpu_perf_service_update_limits_v1(&service, &config.limits) ==
          H700_CPU_PERF_SERVICE_OK,
          "verified policy limits can be refreshed");
    rc = h700_cpu_perf_service_request_mode_v1(&service, H700_CPU_PERF_MODE_PERFORMANCE);
    CHECK(rc == H700_CPU_PERF_SERVICE_POLICY_ONLY,
          "successful request returns explicit policy-only status");
    CHECK(h700_cpu_perf_service_get_state_v1(&service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.planned_frequency_hz == 1400000000u &&
          state.planned_voltage_uv == 1100000u && state.hardware_applied == 0u &&
          state.generation == 1u,
          "planned OPP is visible but remains separate from hardware state");

    memset(&telemetry, 0, sizeof(telemetry));
    telemetry.abi_version = H700_CPU_PERF_SERVICE_ABI_VERSION;
    telemetry.struct_size = (uint32_t)sizeof(telemetry);
    telemetry.valid_flags = H700_CPU_PERF_TELEMETRY_CLOCK_VALID |
                            H700_CPU_PERF_TELEMETRY_VOLTAGE_VALID |
                            H700_CPU_PERF_TELEMETRY_TEMP_VALID;
    telemetry.actual_frequency_hz = 1000000000u;
    telemetry.actual_voltage_uv = 1000000u;
    telemetry.temperature_mc = 62500;
    CHECK(h700_cpu_perf_service_update_telemetry_v1(&service, &telemetry) ==
          H700_CPU_PERF_SERVICE_OK,
          "read-only hardware telemetry can be published independently");
    CHECK(h700_cpu_perf_service_get_state_v1(&service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.actual_frequency_hz == 1000000000u &&
          state.actual_voltage_uv == 1000000u && state.temperature_mc == 62500 &&
          state.planned_frequency_hz == 1400000000u && state.hardware_applied == 0u,
          "actual telemetry never overwrites or masquerades as planned OPP");

    config.limits = ready_limits();
    config.limits.thermal_max_frequency_hz = 1000000000u;
    CHECK(h700_cpu_perf_service_update_limits_v1(&service, &config.limits) ==
          H700_CPU_PERF_SERVICE_POLICY_ONLY,
          "limits update re-evaluates the current requested mode");
    CHECK(h700_cpu_perf_service_get_state_v1(&service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.planned_frequency_hz == 1000000000u &&
          (state.policy_flags & H700_CPU_PERF_RESULT_THERMAL_CLAMPED) != 0u &&
          state.hardware_applied == 0u,
          "thermal cap is applied to the planned OPP");

    CHECK(h700_cpu_perf_service_request_mode_v1(&service, H700_CPU_PERF_MODE_BOOST) ==
          H700_CPU_PERF_ERR_MODE_UNAVAILABLE,
          "Boost remains unavailable without its explicit approval gate");

    telemetry.valid_flags = 0x80000000u;
    CHECK(h700_cpu_perf_service_update_telemetry_v1(&service, &telemetry) ==
          H700_CPU_PERF_SERVICE_ERR_ABI,
          "unknown telemetry flags are rejected");

    memcpy(bad_opps, test_opps, sizeof(test_opps));
    bad_opps[2].frequency_hz = bad_opps[1].frequency_hz;
    config.opps = bad_opps;
    config.limits = ready_limits();
    CHECK(h700_cpu_perf_service_init_v1(&service, (uint32_t)sizeof(service),
          &config) == H700_CPU_PERF_SERVICE_OK,
          "service structurally accepts an OPP table for policy validation");
    CHECK(h700_cpu_perf_service_request_mode_v1(&service, H700_CPU_PERF_MODE_BALANCED) ==
          H700_CPU_PERF_ERR_BAD_TABLE,
          "invalid OPP table is rejected by the policy selector");

    puts("PASS: H700 CPU performance service v1 ABI, policy-only semantics and telemetry separation");
    return EXIT_SUCCESS;
}

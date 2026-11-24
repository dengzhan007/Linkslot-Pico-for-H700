#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_cpu_performance_runtime.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\n", message); \
        return EXIT_FAILURE; \
    } \
} while (0)

/* Host-only synthetic fixture. These are deliberately not RG34XX OPP data. */
static const struct h700_cpu_opp synthetic_opps[] = {
    { 100000000u, { 800000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u },
    { 200000000u, { 850000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u },
    { 300000000u, { 900000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u },
    { 400000000u, { 950000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u }
};

static h700_cpu_perf_board_profile_v1_t synthetic_profile(void)
{
    h700_cpu_perf_board_profile_v1_t profile;
    memset(&profile, 0, sizeof(profile));
    profile.abi_version = H700_CPU_PERF_RUNTIME_ABI_VERSION;
    profile.struct_size = (uint32_t)sizeof(profile);
    profile.board_id = H700_CPU_PERF_RUNTIME_BOARD_RG34XX;
    profile.board_revision = 1u; /* Synthetic test revision only. */
    profile.profile_flags = H700_CPU_PERF_RUNTIME_PROFILE_APPROVED;
    profile.opps = synthetic_opps;
    profile.opp_count = (uint32_t)(sizeof(synthetic_opps) / sizeof(synthetic_opps[0]));
    profile.limits.verified_gates = H700_CPU_PERF_REQUIRED_GATES;
    profile.limits.board_max_voltage_uv = 1000000u;
    profile.limits.thermal_max_frequency_hz = 400000000u;
    profile.limits.auto_target_frequency_hz = 300000000u;
    profile.limits.speed_bin = 0u;
    return profile;
}

int main(void)
{
    h700_cpu_perf_board_profile_v1_t profile;
    h700_cpu_perf_service_v1_t *service;
    h700_cpu_perf_state_v1_t state;
    int rc;

    CHECK(h700_cpu_perf_runtime_status_v1() == H700_CPU_PERF_RUNTIME_ERR_UNAVAILABLE,
          "runtime starts unavailable without a board profile");
    CHECK(h700_cpu_perf_runtime_get_service_v1() == NULL,
          "no service pointer is exposed before approved initialization");
    CHECK(h700_cpu_perf_runtime_init_v1(NULL) == H700_CPU_PERF_RUNTIME_ERR_UNAVAILABLE,
          "NULL board profile leaves runtime unavailable");

    profile = synthetic_profile();
    profile.profile_flags = 0u;
    CHECK(h700_cpu_perf_runtime_init_v1(&profile) == H700_CPU_PERF_RUNTIME_ERR_PROFILE,
          "profile must carry explicit board approval");
    CHECK(h700_cpu_perf_runtime_get_service_v1() == NULL,
          "unapproved profile cannot expose a service");

    profile = synthetic_profile();
    profile.board_id ^= 1u;
    CHECK(h700_cpu_perf_runtime_init_v1(&profile) == H700_CPU_PERF_RUNTIME_ERR_PROFILE,
          "profile for another board is rejected");

    profile = synthetic_profile();
    profile.limits.verified_gates &= ~H700_CPU_PERF_GATE_CPU_RAIL_CONFIRMED;
    CHECK(h700_cpu_perf_runtime_init_v1(&profile) == H700_CPU_PERF_RUNTIME_ERR_GATES,
          "missing hardware evidence gates keep the service unavailable");

    profile = synthetic_profile();
    profile.opps = NULL;
    CHECK(h700_cpu_perf_runtime_init_v1(&profile) == H700_CPU_PERF_RUNTIME_ERR_POLICY,
          "missing OPP data cannot initialize the service");

    profile = synthetic_profile();
    service = h700_cpu_perf_runtime_get_service_v1();
    CHECK(service == NULL, "failed attempts leave no service available");
    CHECK(h700_cpu_perf_runtime_init_v1(&profile) == H700_CPU_PERF_RUNTIME_OK,
          "fully approved and policy-valid board profile initializes owner");
    service = h700_cpu_perf_runtime_get_service_v1();
    CHECK(service != NULL && service == h700_cpu_perf_runtime_get_service_v1(),
          "runtime exposes one stable statically owned service instance");
    CHECK(h700_cpu_perf_service_get_state_v1(service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.hardware_applied == 0u,
          "owned service starts without claiming hardware application");

    rc = h700_cpu_perf_service_request_mode_v1(service, H700_CPU_PERF_MODE_PERFORMANCE);
    CHECK(rc == H700_CPU_PERF_SERVICE_POLICY_ONLY,
          "owner provides the policy-only service after initialization");
    CHECK(h700_cpu_perf_service_get_state_v1(service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_OK && state.planned_frequency_hz == 400000000u &&
          state.actual_frequency_hz == 0u && state.hardware_applied == 0u,
          "synthetic policy plan is not reported as measured or applied hardware");

    CHECK(h700_cpu_perf_runtime_init_v1(NULL) == H700_CPU_PERF_RUNTIME_ERR_UNAVAILABLE,
          "removing the profile revokes runtime readiness");
    CHECK(h700_cpu_perf_runtime_get_service_v1() == NULL,
          "revoked runtime no longer exposes the owner");
    CHECK(h700_cpu_perf_service_get_state_v1(service, &state, (uint32_t)sizeof(state)) ==
          H700_CPU_PERF_SERVICE_ERR_NOT_INITIALIZED,
          "a previously borrowed service pointer is invalidated fail-closed");

    puts("PASS: H700 CPU runtime owner, profile validation and fail-closed lifecycle");
    return EXIT_SUCCESS;
}

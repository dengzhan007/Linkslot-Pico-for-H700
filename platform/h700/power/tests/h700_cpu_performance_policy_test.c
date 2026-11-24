#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../h700_cpu_performance_policy.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL: %s\\n", message); \
        return EXIT_FAILURE; \
    } \
} while (0)

static const struct h700_cpu_opp test_opps[] = {
    { 600000000u,  { 900000u, 900000u, 0u, 0u, 0u, 0u }, 0x03u, 0u, 0u },
    { 800000000u,  { 950000u, 950000u, 0u, 0u, 0u, 0u }, 0x03u, 0u, 0u },
    { 1000000000u, { 1000000u, 1000000u, 0u, 0u, 0u, 0u }, 0x03u, 0u, 0u },
    { 1200000000u, { 1050000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u },
    { 1400000000u, { 1100000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, 0u, 0u },
    { 1500000000u, { 1150000u, 0u, 0u, 0u, 0u, 0u }, 0x01u, H700_CPU_OPP_FLAG_BOOST_ONLY, 0u }
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
    struct h700_cpu_perf_limits limits;
    struct h700_cpu_perf_selection selected;
    struct h700_cpu_opp bad_opps[sizeof(test_opps) / sizeof(test_opps[0])];
    int rc;

    limits = ready_limits();
    limits.verified_gates = 0u;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_ERR_NOT_READY, "missing evidence gates fail closed");

    limits = ready_limits();
    limits.speed_bin = H700_CPU_PERF_SPEED_BIN_COUNT;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_ERR_NOT_READY, "unknown speed bin is rejected");

    limits = ready_limits();
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_POWER_SAVER, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 600000000u,
          "Power Saver selects lowest eligible continuous OPP");

    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_BALANCED, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1000000000u,
          "Balanced selects lower median continuous OPP");

    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1400000000u,
          "Performance selects highest continuous OPP, excluding boost-only");

    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_AUTO, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1200000000u,
          "Auto selects lowest OPP meeting the target floor");

    limits.auto_target_frequency_hz = 1600000000u;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_AUTO, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1400000000u &&
          (selected.flags & H700_CPU_PERF_RESULT_AUTO_TARGET_UNMET) != 0u,
          "Auto reports when the target cannot be met");

    limits = ready_limits();
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_BOOST, &selected);
    CHECK(rc == H700_CPU_PERF_ERR_MODE_UNAVAILABLE,
          "Boost is unavailable without its separate approval gate");

    limits.verified_gates |= H700_CPU_PERF_GATE_BOOST_APPROVED;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_BOOST, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1500000000u &&
          (selected.flags & H700_CPU_PERF_RESULT_BOOST_SELECTED) != 0u,
          "Boost selects an explicitly marked boost OPP");

    limits = ready_limits();
    limits.speed_bin = 1u;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1000000000u,
          "speed-bin mask excludes unsupported higher OPPs");

    limits = ready_limits();
    limits.board_max_voltage_uv = 1000000u;
    limits.thermal_max_frequency_hz = 1200000000u;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_STATUS_OK && selected.frequency_hz == 1000000000u &&
          (selected.flags & H700_CPU_PERF_RESULT_BOARD_VOLTAGE_CLAMPED) != 0u &&
          (selected.flags & H700_CPU_PERF_RESULT_THERMAL_CLAMPED) != 0u,
          "board voltage and thermal ceilings clamp OPP selection");

    limits = ready_limits();
    limits.thermal_max_frequency_hz = 0u;
    rc = h700_cpu_perf_select_opp(test_opps, sizeof(test_opps) / sizeof(test_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_ERR_NOT_READY, "missing thermal limit fails closed");

    memcpy(bad_opps, test_opps, sizeof(test_opps));
    bad_opps[2].frequency_hz = bad_opps[1].frequency_hz;
    limits = ready_limits();
    rc = h700_cpu_perf_select_opp(bad_opps, sizeof(bad_opps) / sizeof(bad_opps[0]),
        &limits, H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_ERR_BAD_TABLE, "unordered or duplicate OPPs are rejected");

    rc = h700_cpu_perf_select_opp(NULL, 0u, &limits,
        H700_CPU_PERF_MODE_PERFORMANCE, &selected);
    CHECK(rc == H700_CPU_PERF_ERR_INVALID_ARGUMENT, "null/empty OPP input is rejected");

    puts("PASS: H700 CPU OPP policy validates evidence gates, speed bins, voltage and thermal ceilings");
    return EXIT_SUCCESS;
}
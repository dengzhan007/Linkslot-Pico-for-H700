/*
 * Pure H700 CPU OPP policy selection.
 *
 * This module deliberately performs no MMIO, PLL, PMIC, timer, or thermal
 * sensor access. The caller must supply a board-approved OPP table and
 * verified capability gates. Hardware transitions are a separate HAL layer
 * and are not implemented or enabled by this module.
 */
#ifndef H700_CPU_PERFORMANCE_POLICY_H
#define H700_CPU_PERFORMANCE_POLICY_H

#include <stddef.h>
#include <stdint.h>

#define H700_CPU_PERF_SPEED_BIN_COUNT 6u
#define H700_CPU_PERF_MAX_OPPS 32u

enum h700_cpu_perf_mode {
    H700_CPU_PERF_MODE_POWER_SAVER = 0,
    H700_CPU_PERF_MODE_BALANCED = 1,
    H700_CPU_PERF_MODE_PERFORMANCE = 2,
    H700_CPU_PERF_MODE_AUTO = 3,
    H700_CPU_PERF_MODE_BOOST = 4
};

enum h700_cpu_perf_gate {
    H700_CPU_PERF_GATE_CPU_RAIL_CONFIRMED = 1u << 0,
    H700_CPU_PERF_GATE_SPEED_BIN_VERIFIED = 1u << 1,
    H700_CPU_PERF_GATE_OPP_TABLE_APPROVED = 1u << 2,
    H700_CPU_PERF_GATE_PMIC_READBACK_VERIFIED = 1u << 3,
    H700_CPU_PERF_GATE_CPUX_PLL_SEQUENCE_VERIFIED = 1u << 4,
    H700_CPU_PERF_GATE_THERMAL_CALIBRATION_VERIFIED = 1u << 5,
    H700_CPU_PERF_GATE_SECONDARY_CPUS_QUIESCED = 1u << 6,
    H700_CPU_PERF_GATE_BOOST_APPROVED = 1u << 7
};

#define H700_CPU_PERF_REQUIRED_GATES \
    ((uint32_t)(H700_CPU_PERF_GATE_CPU_RAIL_CONFIRMED | \
                H700_CPU_PERF_GATE_SPEED_BIN_VERIFIED | \
                H700_CPU_PERF_GATE_OPP_TABLE_APPROVED | \
                H700_CPU_PERF_GATE_PMIC_READBACK_VERIFIED | \
                H700_CPU_PERF_GATE_CPUX_PLL_SEQUENCE_VERIFIED | \
                H700_CPU_PERF_GATE_THERMAL_CALIBRATION_VERIFIED | \
                H700_CPU_PERF_GATE_SECONDARY_CPUS_QUIESCED))

enum h700_cpu_opp_flags {
    H700_CPU_OPP_FLAG_BOOST_ONLY = 1u << 0
};

enum h700_cpu_perf_result_flags {
    H700_CPU_PERF_RESULT_BOARD_VOLTAGE_CLAMPED = 1u << 0,
    H700_CPU_PERF_RESULT_THERMAL_CLAMPED = 1u << 1,
    H700_CPU_PERF_RESULT_AUTO_TARGET_UNMET = 1u << 2,
    H700_CPU_PERF_RESULT_BOOST_SELECTED = 1u << 3
};

enum h700_cpu_perf_status {
    H700_CPU_PERF_STATUS_OK = 0,
    H700_CPU_PERF_ERR_INVALID_ARGUMENT = -1,
    H700_CPU_PERF_ERR_NOT_READY = -2,
    H700_CPU_PERF_ERR_NO_SAFE_OPP = -3,
    H700_CPU_PERF_ERR_MODE_UNAVAILABLE = -4,
    H700_CPU_PERF_ERR_BAD_TABLE = -5
};

/*
 * OPP values are supplied by the board profile only after validation.
 * A speed-bin bit is (1u << bin). Voltages are in microvolts.
 * Frequency entries must be strictly ascending and unique.
 */
struct h700_cpu_opp {
    uint32_t frequency_hz;
    uint32_t voltage_uv[H700_CPU_PERF_SPEED_BIN_COUNT];
    uint8_t speed_bin_mask;
    uint8_t flags;
    uint16_t reserved;
};

struct h700_cpu_perf_limits {
    uint32_t verified_gates;
    uint32_t board_max_voltage_uv;
    uint32_t thermal_max_frequency_hz;
    uint32_t auto_target_frequency_hz;
    uint8_t speed_bin;
    uint8_t reserved[3];
};

struct h700_cpu_perf_selection {
    uint32_t frequency_hz;
    uint32_t voltage_uv;
    uint32_t flags;
    uint16_t opp_index;
    uint8_t mode;
    uint8_t reserved;
};

/*
 * Select from an already approved board OPP table. No hardware is touched.
 * Power Saver selects the lowest eligible continuous OPP.
 * Balanced selects the lower median eligible continuous OPP.
 * Performance selects the highest eligible continuous OPP.
 * Auto selects the lowest eligible OPP meeting auto_target_frequency_hz;
 * if none meets the target, it returns the highest and sets AUTO_TARGET_UNMET.
 * Boost selects the highest BOOST_ONLY OPP and requires BOOST_APPROVED.
 *
 * The mode selector never invents a frequency or voltage. Unknown speed bins,
 * missing capability gates, missing thermal limits and invalid OPP tables
 * fail closed.
 */
int h700_cpu_perf_select_opp(
    const struct h700_cpu_opp *opps,
    size_t opp_count,
    const struct h700_cpu_perf_limits *limits,
    enum h700_cpu_perf_mode mode,
    struct h700_cpu_perf_selection *selection
);

#endif /* H700_CPU_PERFORMANCE_POLICY_H */
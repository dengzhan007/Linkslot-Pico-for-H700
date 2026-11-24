#include "h700_cpu_performance_policy.h"

#define H700_CPU_PERF_VALID_SPEED_MASK \
    ((uint8_t)((1u << H700_CPU_PERF_SPEED_BIN_COUNT) - 1u))

static int h700_cpu_perf_validate_table(
    const struct h700_cpu_opp *opps,
    size_t opp_count
)
{
    size_t i;
    size_t bin;

    if (opps == NULL || opp_count == 0u ||
        opp_count > H700_CPU_PERF_MAX_OPPS) {
        return H700_CPU_PERF_ERR_INVALID_ARGUMENT;
    }

    for (i = 0u; i < opp_count; ++i) {
        const struct h700_cpu_opp *opp = &opps[i];

        if (opp->frequency_hz == 0u || opp->speed_bin_mask == 0u ||
            (opp->speed_bin_mask & (uint8_t)~H700_CPU_PERF_VALID_SPEED_MASK) != 0u ||
            (opp->flags & (uint8_t)~H700_CPU_OPP_FLAG_BOOST_ONLY) != 0u ||
            opp->reserved != 0u) {
            return H700_CPU_PERF_ERR_BAD_TABLE;
        }

        if (i != 0u && opps[i - 1u].frequency_hz >= opp->frequency_hz) {
            return H700_CPU_PERF_ERR_BAD_TABLE;
        }

        for (bin = 0u; bin < H700_CPU_PERF_SPEED_BIN_COUNT; ++bin) {
            const uint8_t bit = (uint8_t)(1u << bin);
            if ((opp->speed_bin_mask & bit) != 0u &&
                opp->voltage_uv[bin] == 0u) {
                return H700_CPU_PERF_ERR_BAD_TABLE;
            }
        }
    }

    return H700_CPU_PERF_STATUS_OK;
}

int h700_cpu_perf_select_opp(
    const struct h700_cpu_opp *opps,
    size_t opp_count,
    const struct h700_cpu_perf_limits *limits,
    enum h700_cpu_perf_mode mode,
    struct h700_cpu_perf_selection *selection
)
{
    size_t eligible[H700_CPU_PERF_MAX_OPPS];
    size_t eligible_count = 0u;
    size_t boost_index = H700_CPU_PERF_MAX_OPPS;
    size_t i;
    uint8_t speed_bit;
    uint32_t result_flags = 0u;
    int table_status;

    if (limits == NULL || selection == NULL) {
        return H700_CPU_PERF_ERR_INVALID_ARGUMENT;
    }

    selection->frequency_hz = 0u;
    selection->voltage_uv = 0u;
    selection->flags = 0u;
    selection->opp_index = 0u;
    selection->mode = (uint8_t)mode;
    selection->reserved = 0u;

    if (mode < H700_CPU_PERF_MODE_POWER_SAVER ||
        mode > H700_CPU_PERF_MODE_BOOST) {
        return H700_CPU_PERF_ERR_INVALID_ARGUMENT;
    }

    if ((limits->verified_gates & H700_CPU_PERF_REQUIRED_GATES) !=
        H700_CPU_PERF_REQUIRED_GATES) {
        return H700_CPU_PERF_ERR_NOT_READY;
    }

    if (limits->speed_bin >= H700_CPU_PERF_SPEED_BIN_COUNT ||
        limits->board_max_voltage_uv == 0u ||
        limits->thermal_max_frequency_hz == 0u ||
        (mode == H700_CPU_PERF_MODE_AUTO &&
         limits->auto_target_frequency_hz == 0u)) {
        return H700_CPU_PERF_ERR_NOT_READY;
    }

    table_status = h700_cpu_perf_validate_table(opps, opp_count);
    if (table_status != H700_CPU_PERF_STATUS_OK) {
        return table_status;
    }

    if (mode == H700_CPU_PERF_MODE_BOOST &&
        (limits->verified_gates & H700_CPU_PERF_GATE_BOOST_APPROVED) == 0u) {
        return H700_CPU_PERF_ERR_MODE_UNAVAILABLE;
    }

    speed_bit = (uint8_t)(1u << limits->speed_bin);

    for (i = 0u; i < opp_count; ++i) {
        const struct h700_cpu_opp *opp = &opps[i];
        uint32_t voltage_uv;

        if ((opp->speed_bin_mask & speed_bit) == 0u) {
            continue;
        }

        int rejected_by_limits = 0;

        voltage_uv = opp->voltage_uv[limits->speed_bin];
        if (voltage_uv > limits->board_max_voltage_uv) {
            result_flags |= H700_CPU_PERF_RESULT_BOARD_VOLTAGE_CLAMPED;
            rejected_by_limits = 1;
        }

        if (opp->frequency_hz > limits->thermal_max_frequency_hz) {
            result_flags |= H700_CPU_PERF_RESULT_THERMAL_CLAMPED;
            rejected_by_limits = 1;
        }

        if (rejected_by_limits) {
            continue;
        }

        if ((opp->flags & H700_CPU_OPP_FLAG_BOOST_ONLY) != 0u) {
            if (mode == H700_CPU_PERF_MODE_BOOST) {
                boost_index = i;
            }
            continue;
        }

        eligible[eligible_count++] = i;
    }

    if (mode == H700_CPU_PERF_MODE_BOOST) {
        if (boost_index == H700_CPU_PERF_MAX_OPPS) {
            return H700_CPU_PERF_ERR_MODE_UNAVAILABLE;
        }

        selection->frequency_hz = opps[boost_index].frequency_hz;
        selection->voltage_uv = opps[boost_index].voltage_uv[limits->speed_bin];
        selection->opp_index = (uint16_t)boost_index;
        selection->flags = result_flags | H700_CPU_PERF_RESULT_BOOST_SELECTED;
        return H700_CPU_PERF_STATUS_OK;
    }

    if (eligible_count == 0u) {
        return H700_CPU_PERF_ERR_NO_SAFE_OPP;
    }

    switch (mode) {
    case H700_CPU_PERF_MODE_POWER_SAVER:
        i = eligible[0];
        break;

    case H700_CPU_PERF_MODE_BALANCED:
        i = eligible[(eligible_count - 1u) / 2u];
        break;

    case H700_CPU_PERF_MODE_PERFORMANCE:
        i = eligible[eligible_count - 1u];
        break;

    case H700_CPU_PERF_MODE_AUTO: {
        size_t candidate = eligible[eligible_count - 1u];
        int found = 0;

        for (i = 0u; i < eligible_count; ++i) {
            const size_t index = eligible[i];
            if (opps[index].frequency_hz >= limits->auto_target_frequency_hz) {
                candidate = index;
                found = 1;
                break;
            }
        }

        if (!found) {
            result_flags |= H700_CPU_PERF_RESULT_AUTO_TARGET_UNMET;
        }
        i = candidate;
        break;
    }

    case H700_CPU_PERF_MODE_BOOST:
    default:
        return H700_CPU_PERF_ERR_INVALID_ARGUMENT;
    }

    selection->frequency_hz = opps[i].frequency_hz;
    selection->voltage_uv = opps[i].voltage_uv[limits->speed_bin];
    selection->opp_index = (uint16_t)i;
    selection->flags = result_flags;
    return H700_CPU_PERF_STATUS_OK;
}
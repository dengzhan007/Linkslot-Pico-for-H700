#include "h700_cpu_performance_runtime.h"

#define H700_CPU_PERF_KNOWN_GATES \
    ((uint32_t)(H700_CPU_PERF_REQUIRED_GATES | \
                H700_CPU_PERF_GATE_BOOST_APPROVED))

static h700_cpu_perf_service_v1_t g_cpu_perf_service;
static int g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_UNAVAILABLE;

static void invalidate_service_storage(void)
{
    unsigned char *bytes = (unsigned char *)&g_cpu_perf_service;
    uint32_t i;

    for (i = 0u; i < (uint32_t)sizeof(g_cpu_perf_service); ++i) {
        bytes[i] = 0u;
    }
}

static int profile_reserved_are_zero(
    const h700_cpu_perf_board_profile_v1_t *profile
)
{
    return profile->reserved == 0u &&
           profile->reserved_words[0] == 0u &&
           profile->reserved_words[1] == 0u &&
           profile->limits.reserved[0] == 0u &&
           profile->limits.reserved[1] == 0u &&
           profile->limits.reserved[2] == 0u;
}

static int profile_has_valid_policy(
    const h700_cpu_perf_board_profile_v1_t *profile
)
{
    static const enum h700_cpu_perf_mode exposed_modes[] = {
        H700_CPU_PERF_MODE_POWER_SAVER,
        H700_CPU_PERF_MODE_BALANCED,
        H700_CPU_PERF_MODE_PERFORMANCE,
        H700_CPU_PERF_MODE_AUTO
    };
    struct h700_cpu_perf_selection selection;
    uint32_t i;

    if (profile->opps == NULL || profile->opp_count == 0u ||
        profile->opp_count > H700_CPU_PERF_SERVICE_MAX_OPPS) {
        return 0;
    }

    /* Validate the table and every mode intended to be selectable in UI. */
    for (i = 0u; i < (uint32_t)(sizeof(exposed_modes) / sizeof(exposed_modes[0])); ++i) {
        if (h700_cpu_perf_select_opp(
                profile->opps,
                profile->opp_count,
                &profile->limits,
                exposed_modes[i],
                &selection
            ) != H700_CPU_PERF_STATUS_OK) {
            return 0;
        }
    }

    return 1;
}

int h700_cpu_perf_runtime_init_v1(
    const h700_cpu_perf_board_profile_v1_t *profile
)
{
    h700_cpu_perf_service_config_v1_t config;
    uint32_t known_profile_flags = H700_CPU_PERF_RUNTIME_PROFILE_APPROVED;
    uint32_t gates;
    int status;

    /* Always revoke previous readiness before inspecting a new profile. */
    invalidate_service_storage();
    g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_UNAVAILABLE;

    if (profile == NULL) {
        return g_cpu_perf_runtime_status;
    }

    if (profile->abi_version != H700_CPU_PERF_RUNTIME_ABI_VERSION ||
        profile->struct_size < (uint32_t)sizeof(*profile) ||
        profile->board_id != H700_CPU_PERF_RUNTIME_BOARD_RG34XX ||
        profile->board_revision == 0u ||
        profile->profile_flags != known_profile_flags ||
        !profile_reserved_are_zero(profile)) {
        g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_PROFILE;
        return g_cpu_perf_runtime_status;
    }

    gates = profile->limits.verified_gates;
    if ((gates & ~H700_CPU_PERF_KNOWN_GATES) != 0u) {
        g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_PROFILE;
        return g_cpu_perf_runtime_status;
    }
    if ((gates & H700_CPU_PERF_REQUIRED_GATES) != H700_CPU_PERF_REQUIRED_GATES) {
        g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_GATES;
        return g_cpu_perf_runtime_status;
    }

    if (!profile_has_valid_policy(profile)) {
        g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_POLICY;
        return g_cpu_perf_runtime_status;
    }

    config.abi_version = H700_CPU_PERF_SERVICE_ABI_VERSION;
    config.struct_size = (uint32_t)sizeof(config);
    config.opps = profile->opps;
    config.opp_count = profile->opp_count;
    config.reserved = 0u;
    config.limits = profile->limits;

    status = h700_cpu_perf_service_init_v1(
        &g_cpu_perf_service,
        (uint32_t)sizeof(g_cpu_perf_service),
        &config
    );
    if (status != H700_CPU_PERF_SERVICE_OK) {
        g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_ERR_SERVICE;
        invalidate_service_storage();
        return g_cpu_perf_runtime_status;
    }

    g_cpu_perf_runtime_status = H700_CPU_PERF_RUNTIME_OK;
    return g_cpu_perf_runtime_status;
}

int h700_cpu_perf_runtime_status_v1(void)
{
    return g_cpu_perf_runtime_status;
}

h700_cpu_perf_service_v1_t *h700_cpu_perf_runtime_get_service_v1(void)
{
    if (g_cpu_perf_runtime_status != H700_CPU_PERF_RUNTIME_OK) {
        return NULL;
    }
    return &g_cpu_perf_service;
}

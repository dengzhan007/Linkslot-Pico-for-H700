#include "h700_cpu_performance_service.h"

#define H700_CPU_PERF_SERVICE_MAGIC 0x48504331u /* "HPC1" */

static int limits_reserved_are_zero(const struct h700_cpu_perf_limits *limits)
{
    return limits->reserved[0] == 0u &&
           limits->reserved[1] == 0u &&
           limits->reserved[2] == 0u;
}

static void clear_plan(h700_cpu_perf_state_v1_t *state)
{
    state->planned_frequency_hz = 0u;
    state->planned_voltage_uv = 0u;
    state->policy_flags = 0u;
}

static int service_is_ready(const h700_cpu_perf_service_v1_t *service)
{
    return service != NULL &&
           service->magic == H700_CPU_PERF_SERVICE_MAGIC &&
           service->abi_version == H700_CPU_PERF_SERVICE_ABI_VERSION &&
           service->initialized == 1u;
}

int h700_cpu_perf_service_init_v1(
    h700_cpu_perf_service_v1_t *service,
    uint32_t service_size,
    const h700_cpu_perf_service_config_v1_t *config
)
{
    uint32_t i;

    if (service == NULL || config == NULL) {
        return H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT;
    }
    if (service_size < (uint32_t)sizeof(*service) ||
        config->abi_version != H700_CPU_PERF_SERVICE_ABI_VERSION ||
        config->struct_size < (uint32_t)sizeof(*config) ||
        config->reserved != 0u) {
        return H700_CPU_PERF_SERVICE_ERR_ABI;
    }
    if (config->opps == NULL || config->opp_count == 0u ||
        config->opp_count > H700_CPU_PERF_SERVICE_MAX_OPPS ||
        !limits_reserved_are_zero(&config->limits)) {
        return H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT;
    }

    service->abi_version = H700_CPU_PERF_SERVICE_ABI_VERSION;
    service->struct_size = (uint32_t)sizeof(*service);
    service->magic = H700_CPU_PERF_SERVICE_MAGIC;
    service->initialized = 0u;
    service->opp_count = config->opp_count;
    service->reserved = 0u;

    for (i = 0u; i < config->opp_count; ++i) {
        service->opps[i] = config->opps[i];
    }
    for (; i < H700_CPU_PERF_SERVICE_MAX_OPPS; ++i) {
        service->opps[i].frequency_hz = 0u;
        service->opps[i].voltage_uv[0] = 0u;
        service->opps[i].voltage_uv[1] = 0u;
        service->opps[i].voltage_uv[2] = 0u;
        service->opps[i].voltage_uv[3] = 0u;
        service->opps[i].voltage_uv[4] = 0u;
        service->opps[i].voltage_uv[5] = 0u;
        service->opps[i].speed_bin_mask = 0u;
        service->opps[i].flags = 0u;
        service->opps[i].reserved = 0u;
    }

    service->limits = config->limits;

    service->state.abi_version = H700_CPU_PERF_SERVICE_ABI_VERSION;
    service->state.struct_size = (uint32_t)sizeof(service->state);
    service->state.generation = 0u;
    service->state.requested_mode = (uint32_t)H700_CPU_PERF_MODE_BALANCED;
    clear_plan(&service->state);
    service->state.last_policy_status = H700_CPU_PERF_ERR_NOT_READY;
    service->state.telemetry_valid_flags = 0u;
    service->state.actual_frequency_hz = 0u;
    service->state.actual_voltage_uv = 0u;
    service->state.temperature_mc = 0;
    service->state.hardware_applied = 0u;
    service->state.reserved[0] = 0u;
    service->state.reserved[1] = 0u;
    service->state.reserved[2] = 0u;

    service->initialized = 1u;
    return H700_CPU_PERF_SERVICE_OK;
}

int h700_cpu_perf_service_update_limits_v1(
    h700_cpu_perf_service_v1_t *service,
    const struct h700_cpu_perf_limits *limits
)
{
    if (!service_is_ready(service)) {
        return H700_CPU_PERF_SERVICE_ERR_NOT_INITIALIZED;
    }
    if (limits == NULL) {
        return H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT;
    }
    if (!limits_reserved_are_zero(limits)) {
        return H700_CPU_PERF_SERVICE_ERR_ABI;
    }

    service->limits = *limits;
    if (service->state.generation != 0u) {
        return h700_cpu_perf_service_request_mode_v1(
            service,
            (enum h700_cpu_perf_mode)service->state.requested_mode
        );
    }
    return H700_CPU_PERF_SERVICE_OK;
}

int h700_cpu_perf_service_request_mode_v1(
    h700_cpu_perf_service_v1_t *service,
    enum h700_cpu_perf_mode mode
)
{
    struct h700_cpu_perf_selection selection;
    int status;

    if (!service_is_ready(service)) {
        return H700_CPU_PERF_SERVICE_ERR_NOT_INITIALIZED;
    }

    service->state.requested_mode = (uint32_t)mode;
    clear_plan(&service->state);

    status = h700_cpu_perf_select_opp(
        service->opps,
        service->opp_count,
        &service->limits,
        mode,
        &selection
    );
    service->state.last_policy_status = status;
    service->state.hardware_applied = 0u;
    if (status != H700_CPU_PERF_STATUS_OK) {
        return status;
    }

    service->state.planned_frequency_hz = selection.frequency_hz;
    service->state.planned_voltage_uv = selection.voltage_uv;
    service->state.policy_flags = selection.flags;
    service->state.generation++;
    if (service->state.generation == 0u) {
        service->state.generation = 1u;
    }

    return H700_CPU_PERF_SERVICE_POLICY_ONLY;
}

int h700_cpu_perf_service_update_telemetry_v1(
    h700_cpu_perf_service_v1_t *service,
    const h700_cpu_perf_telemetry_v1_t *telemetry
)
{
    if (!service_is_ready(service)) {
        return H700_CPU_PERF_SERVICE_ERR_NOT_INITIALIZED;
    }
    if (telemetry == NULL) {
        return H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT;
    }
    if (telemetry->abi_version != H700_CPU_PERF_SERVICE_ABI_VERSION ||
        telemetry->struct_size < (uint32_t)sizeof(*telemetry) ||
        telemetry->reserved != 0u ||
        (telemetry->valid_flags & ~H700_CPU_PERF_TELEMETRY_VALID_MASK) != 0u) {
        return H700_CPU_PERF_SERVICE_ERR_ABI;
    }
    if (((telemetry->valid_flags & H700_CPU_PERF_TELEMETRY_CLOCK_VALID) != 0u &&
         telemetry->actual_frequency_hz == 0u) ||
        ((telemetry->valid_flags & H700_CPU_PERF_TELEMETRY_VOLTAGE_VALID) != 0u &&
         telemetry->actual_voltage_uv == 0u)) {
        return H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT;
    }

    service->state.telemetry_valid_flags = telemetry->valid_flags;
    service->state.actual_frequency_hz =
        (telemetry->valid_flags & H700_CPU_PERF_TELEMETRY_CLOCK_VALID) != 0u
            ? telemetry->actual_frequency_hz : 0u;
    service->state.actual_voltage_uv =
        (telemetry->valid_flags & H700_CPU_PERF_TELEMETRY_VOLTAGE_VALID) != 0u
            ? telemetry->actual_voltage_uv : 0u;
    service->state.temperature_mc =
        (telemetry->valid_flags & H700_CPU_PERF_TELEMETRY_TEMP_VALID) != 0u
            ? telemetry->temperature_mc : 0;
    return H700_CPU_PERF_SERVICE_OK;
}

int h700_cpu_perf_service_get_state_v1(
    const h700_cpu_perf_service_v1_t *service,
    h700_cpu_perf_state_v1_t *state,
    uint32_t state_size
)
{
    if (!service_is_ready(service)) {
        return H700_CPU_PERF_SERVICE_ERR_NOT_INITIALIZED;
    }
    if (state == NULL) {
        return H700_CPU_PERF_SERVICE_ERR_INVALID_ARGUMENT;
    }
    if (state_size < (uint32_t)sizeof(*state)) {
        return H700_CPU_PERF_SERVICE_ERR_ABI;
    }

    *state = service->state;
    return H700_CPU_PERF_SERVICE_OK;
}

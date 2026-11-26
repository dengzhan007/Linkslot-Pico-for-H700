#include "h700_arch_timer.h"

#include <stdint.h>

#if defined(H700_ARCH_TIMER_HOST_TEST)

extern volatile uint64_t h700_arch_timer_test_counter;
extern volatile uint32_t h700_arch_timer_test_frequency;

static uint64_t read_virtual_counter(void)
{
    return h700_arch_timer_test_counter;
}

static uint32_t read_counter_frequency(void)
{
    return h700_arch_timer_test_frequency;
}

#elif defined(__aarch64__)

static uint64_t read_virtual_counter(void)
{
    uint64_t value;
    __asm__ volatile("isb\n\tmrs %0, cntvct_el0"
                     : "=r"(value) : : "memory");
    return value;
}

static uint32_t read_counter_frequency(void)
{
    uint64_t value;
    __asm__ volatile("mrs %0, cntfrq_el0" : "=r"(value));
    return (uint32_t)value;
}

#elif defined(__arm__)

static uint64_t read_virtual_counter(void)
{
    uint32_t low;
    uint32_t high;
    __asm__ volatile("isb\n\tmrrc p15, 1, %0, %1, c14"
                     : "=r"(low), "=r"(high) : : "memory");
    return ((uint64_t)high << 32) | (uint64_t)low;
}

static uint32_t read_counter_frequency(void)
{
    uint32_t value;
    __asm__ volatile("mrc p15, 0, %0, c14, c0, 0" : "=r"(value));
    return value;
}

#else
#error "H700 Generic Timer provider requires AArch32 or AArch64"
#endif

uint64_t h700_arch_timer_ticks_to_ms(uint64_t elapsed_ticks, uint32_t frequency_hz)
{
    uint64_t whole_seconds;
    uint64_t remainder;
    uint64_t whole_ms;
    uint64_t fractional_ms;

    if (frequency_hz == 0u)
        return 0u;

    /* Split before scaling; remainder is less than a 32-bit frequency. */
    whole_seconds = elapsed_ticks / (uint64_t)frequency_hz;
    remainder = elapsed_ticks % (uint64_t)frequency_hz;

    if (whole_seconds > (UINT64_MAX / 1000u))
        return UINT64_MAX;

    whole_ms = whole_seconds * 1000u;
    fractional_ms = (remainder * 1000u) / (uint64_t)frequency_hz;

    if (whole_ms > (UINT64_MAX - fractional_ms))
        return UINT64_MAX;

    return whole_ms + fractional_ms;
}

static uint64_t h700_arch_timer_monotonic_ms(void *context)
{
    const h700_arch_timer_state_t *state =
        (const h700_arch_timer_state_t *)context;
    uint64_t current_count;

    if ((state == (const h700_arch_timer_state_t *)0) ||
        (state->magic != H700_ARCH_TIMER_STATE_MAGIC) ||
        (state->frequency_hz == 0u))
        return 0u;

    current_count = read_virtual_counter();

    /* Unsigned subtraction also handles a single 64-bit counter wrap. */
    return h700_arch_timer_ticks_to_ms(
        current_count - state->start_count,
        state->frequency_hz
    );
}

int h700_arch_timer_init(
    h700_arch_timer_state_t *state,
    h700_time_service_v1_t *service)
{
    uint32_t frequency_hz;
    uint64_t start_count;

    if ((state == (h700_arch_timer_state_t *)0) ||
        (service == (h700_time_service_v1_t *)0))
        return H700_ARCH_TIMER_EINVAL;

    state->start_count = 0u;
    state->frequency_hz = 0u;
    state->magic = 0u;

    service->abi_version = 0u;
    service->struct_size = 0u;
    service->context = (void *)0;
    service->monotonic_ms = (h700_monotonic_ms_fn)0;

    frequency_hz = read_counter_frequency();
    if (frequency_hz == 0u)
        return H700_ARCH_TIMER_EFREQUENCY;

    start_count = read_virtual_counter();

    state->start_count = start_count;
    state->frequency_hz = frequency_hz;
    state->magic = H700_ARCH_TIMER_STATE_MAGIC;

    service->abi_version = H700_TIME_ABI_VERSION;
    service->struct_size = (uint32_t)sizeof(*service);
    service->context = state;
    service->monotonic_ms = h700_arch_timer_monotonic_ms;

    return H700_ARCH_TIMER_OK;
}

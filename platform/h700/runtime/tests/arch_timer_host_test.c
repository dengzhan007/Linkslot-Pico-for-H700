#include "h700_arch_timer.h"

#include <assert.h>
#include <stdint.h>
#include <stdio.h>

volatile uint64_t h700_arch_timer_test_counter;
volatile uint32_t h700_arch_timer_test_frequency;

static uint64_t now_ms(const h700_time_service_v1_t *service)
{
    assert(service != NULL);
    assert(service->monotonic_ms != NULL);
    return service->monotonic_ms(service->context);
}

static void test_tick_conversion(void)
{
    assert(h700_arch_timer_ticks_to_ms(0u, 24000000u) == 0u);
    assert(h700_arch_timer_ticks_to_ms(23999u, 24000000u) == 0u);
    assert(h700_arch_timer_ticks_to_ms(24000u, 24000000u) == 1u);
    assert(h700_arch_timer_ticks_to_ms(12000000u, 24000000u) == 500u);
    assert(h700_arch_timer_ticks_to_ms(24000000u, 24000000u) == 1000u);
    assert(h700_arch_timer_ticks_to_ms(UINT64_MAX, 0u) == 0u);
    assert(h700_arch_timer_ticks_to_ms(UINT64_MAX, 1u) == UINT64_MAX);
}

static void test_service_lifecycle(void)
{
    h700_arch_timer_state_t state;
    h700_time_service_v1_t service;

    h700_arch_timer_test_frequency = 0u;
    h700_arch_timer_test_counter = 0x123456789ABCDEF0ull;
    assert(h700_arch_timer_init(&state, &service) == H700_ARCH_TIMER_EFREQUENCY);
    assert(service.abi_version == 0u);
    assert(service.monotonic_ms == NULL);
    assert(state.magic == 0u);

    h700_arch_timer_test_frequency = 24000000u;
    assert(h700_arch_timer_init(&state, &service) == H700_ARCH_TIMER_OK);
    assert(service.abi_version == H700_TIME_ABI_VERSION);
    assert(service.struct_size == sizeof(service));
    assert(service.context == &state);
    assert(now_ms(&service) == 0u);

    h700_arch_timer_test_counter += 24000u;
    assert(now_ms(&service) == 1u);
    h700_arch_timer_test_counter += 11976000u;
    assert(now_ms(&service) == 500u);
    h700_arch_timer_test_counter += 12000000u;
    assert(now_ms(&service) == 1000u);
    assert(now_ms(&service) == 1000u);

    assert(h700_arch_timer_init(&state, &service) == H700_ARCH_TIMER_OK);
    assert(now_ms(&service) == 0u);
}

static void test_counter_wrap(void)
{
    h700_arch_timer_state_t state;
    h700_time_service_v1_t service;

    h700_arch_timer_test_frequency = 32u;
    h700_arch_timer_test_counter = UINT64_MAX - 15u;
    assert(h700_arch_timer_init(&state, &service) == H700_ARCH_TIMER_OK);

    h700_arch_timer_test_counter = 16u;
    assert(now_ms(&service) == 1000u);
}

static void test_invalid_arguments(void)
{
    h700_arch_timer_state_t state;
    h700_time_service_v1_t service;

    assert(h700_arch_timer_init(NULL, &service) == H700_ARCH_TIMER_EINVAL);
    assert(h700_arch_timer_init(&state, NULL) == H700_ARCH_TIMER_EINVAL);
}

int main(void)
{
    test_tick_conversion();
    test_service_lifecycle();
    test_counter_wrap();
    test_invalid_arguments();

    puts("PASS: CNTVCT/CNTFRQ time conversion and ABI lifecycle");
    puts("PASS: zero-frequency fail-closed behavior");
    puts("PASS: sub-millisecond rounding, large-value saturation and 64-bit wrap");
    puts("PASS: host-only register reader seam");
    return 0;
}

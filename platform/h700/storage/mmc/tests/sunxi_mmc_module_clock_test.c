#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "../sunxi_mmc_module_clock.h"
#define OSC_HZ 24000000u
#define PLL6_EFFECTIVE_HZ 600000000u
#define REG_ENABLE (1u << 31)
#define REG_PLL6_MUX (1u << 24)
#define CHECK(condition, message) do { \
    if (!(condition)) { fprintf(stderr, "FAIL: %s (line %d)\n", message, __LINE__); return 0; } \
} while (0)
static int expect_clock(uint32_t target, uint8_t source, uint8_t n,
                        uint8_t m, uint32_t actual, uint32_t reg)
{
    struct sunxi_mmc_module_clock_config config;
    int result = sunxi_mmc_calculate_module_clock(
        target, OSC_HZ, PLL6_EFFECTIVE_HZ, &config);
    CHECK(result == SUNXI_MMC_MODULE_CLOCK_OK, "clock rate is representable");
    CHECK(config.source == source, "selected clock parent matches");
    CHECK(config.n_exponent == n, "N exponent matches");
    CHECK(config.m_divider == m, "M divider matches");
    CHECK(config.actual_hz == actual, "actual module rate matches");
    CHECK(config.register_value == reg, "CCU register value matches");
    CHECK(config.actual_hz <= target, "calculated rate does not overspeed target");
    return 1;
}
static int test_reference_rates(void)
{
    CHECK(expect_clock(24000000u, SUNXI_MMC_MODULE_CLOCK_OSC24M,
                       0u, 1u, 24000000u, REG_ENABLE), "24 MHz oscillator");
    CHECK(expect_clock(400000u, SUNXI_MMC_MODULE_CLOCK_OSC24M,
                       2u, 15u, 400000u, REG_ENABLE | 0x0000020eu),
          "400 kHz identification clock");
    CHECK(expect_clock(1000000u, SUNXI_MMC_MODULE_CLOCK_OSC24M,
                       1u, 12u, 1000000u, REG_ENABLE | 0x0000010bu),
          "1 MHz oscillator-derived clock");
    CHECK(expect_clock(25000000u, SUNXI_MMC_MODULE_CLOCK_PLL6,
                       1u, 12u, 25000000u,
                       REG_ENABLE | REG_PLL6_MUX | 0x0000010bu),
          "25 MHz PLL6-derived clock");
    CHECK(expect_clock(50000000u, SUNXI_MMC_MODULE_CLOCK_PLL6,
                       0u, 12u, 50000000u,
                       REG_ENABLE | REG_PLL6_MUX | 0x0000000bu),
          "50 MHz PLL6-derived clock");
    CHECK(expect_clock(52000000u, SUNXI_MMC_MODULE_CLOCK_PLL6,
                       0u, 12u, 50000000u,
                       REG_ENABLE | REG_PLL6_MUX | 0x0000000bu),
          "52 MHz request is rounded down without overspeed");
    return 1;
}
static int test_range_boundaries(void)
{
    struct sunxi_mmc_module_clock_config config;
    CHECK(sunxi_mmc_calculate_module_clock(187500u, OSC_HZ,
          PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_OK,
          "minimum oscillator-derived rate accepted");
    CHECK(config.n_exponent == 3u && config.m_divider == 16u &&
          config.actual_hz == 187500u, "minimum oscillator divider");
    CHECK(sunxi_mmc_calculate_module_clock(187499u, OSC_HZ,
          PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_OUT_OF_RANGE,
          "rate below oscillator minimum rejected");
    CHECK(config.register_value == 0u && config.actual_hz == 0u,
          "failed calculation publishes zero result");
    CHECK(sunxi_mmc_calculate_module_clock(24000001u, OSC_HZ,
          PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_OK,
          "first rate above OSC24M threshold selects PLL6");
    CHECK(config.source == SUNXI_MMC_MODULE_CLOCK_PLL6 &&
          config.n_exponent == 1u && config.m_divider == 13u &&
          config.actual_hz == 23076923u,
          "PLL6 threshold transition uses the expected rounded divider");
    return 1;
}
static int test_invalid_arguments(void)
{
    struct sunxi_mmc_module_clock_config config;
    CHECK(sunxi_mmc_calculate_module_clock(400000u, OSC_HZ,
          PLL6_EFFECTIVE_HZ, NULL) == SUNXI_MMC_MODULE_CLOCK_INVALID,
          "null output pointer rejected");
    CHECK(sunxi_mmc_calculate_module_clock(0u, OSC_HZ,
          PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_INVALID,
          "zero target rate rejected");
    CHECK(sunxi_mmc_calculate_module_clock(400000u, 0u,
          PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_INVALID,
          "zero oscillator rate rejected");
    CHECK(sunxi_mmc_calculate_module_clock(50000000u, OSC_HZ, 0u,
          &config) == SUNXI_MMC_MODULE_CLOCK_INVALID,
          "missing PLL6 rate rejected");
    return 1;
}
static int test_no_overspeed_for_rate_sweep(void)
{
    struct sunxi_mmc_module_clock_config config;
    uint32_t target;
    for (target = 187500u; target <= 24000000u; target += 7919u) {
        CHECK(sunxi_mmc_calculate_module_clock(target, OSC_HZ,
              PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_OK,
              "oscillator rate sweep remains representable");
        CHECK(config.actual_hz <= target, "oscillator sweep avoids overspeed");
    }
    for (target = 24000001u; target <= 600000000u; target += 1000003u) {
        CHECK(sunxi_mmc_calculate_module_clock(target, OSC_HZ,
              PLL6_EFFECTIVE_HZ, &config) == SUNXI_MMC_MODULE_CLOCK_OK,
              "PLL6 rate sweep remains representable");
        CHECK(config.actual_hz <= target, "PLL6 sweep avoids overspeed");
    }
    return 1;
}
int main(void)
{
    if (!test_reference_rates() || !test_range_boundaries() ||
        !test_invalid_arguments() || !test_no_overspeed_for_rate_sweep())
        return EXIT_FAILURE;
    puts("PASS: H616 MMC module-clock parent, divider, range and no-overspeed tests");
    return EXIT_SUCCESS;
}

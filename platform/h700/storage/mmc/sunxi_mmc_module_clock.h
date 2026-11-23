#ifndef H700_SUNXI_MMC_MODULE_CLOCK_H
#define H700_SUNXI_MMC_MODULE_CLOCK_H
#include <stdint.h>
#define SUNXI_MMC_MODULE_CLOCK_OSC24M_LIMIT_HZ 24000000u
enum sunxi_mmc_module_clock_source {
    SUNXI_MMC_MODULE_CLOCK_OSC24M = 0,
    SUNXI_MMC_MODULE_CLOCK_PLL6 = 1
};
enum sunxi_mmc_module_clock_result {
    SUNXI_MMC_MODULE_CLOCK_OK = 0,
    SUNXI_MMC_MODULE_CLOCK_INVALID = -1,
    SUNXI_MMC_MODULE_CLOCK_OUT_OF_RANGE = -2
};
struct sunxi_mmc_module_clock_config {
    uint32_t register_value;
    uint32_t parent_hz;
    uint32_t actual_hz;
    uint8_t source;
    uint8_t m_divider;
    uint8_t n_exponent;
};
/*
 * Pure H616/H700 clock calculator. pll6_effective_hz must match the
 * effective PLL6 rate returned by the pinned U-Boot clock_get_pll6() path;
 * do not pass the raw doubled clock input. No MMIO/CCU access is performed.
 */
int sunxi_mmc_calculate_module_clock(
    uint32_t target_hz,
    uint32_t osc24m_hz,
    uint32_t pll6_effective_hz,
    struct sunxi_mmc_module_clock_config *config);
#endif

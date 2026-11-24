#include "sunxi_mmc_module_clock.h"
#define MMC_CLK_REG_M_MASK    0x0000000fu
#define MMC_CLK_REG_N_SHIFT   8u
#define MMC_CLK_REG_MUX_SHIFT 24u
#define MMC_CLK_REG_ENABLE    (1u << 31)
int sunxi_mmc_calculate_module_clock(
    uint32_t target_hz,
    uint32_t osc24m_hz,
    uint32_t pll6_effective_hz,
    struct sunxi_mmc_module_clock_config *config)
{
    uint32_t parent_hz;
    uint32_t source;
    uint64_t divider;
    uint32_t n = 0u;
    uint32_t m_divider;
    uint32_t actual_hz;
    if (config != 0) {
        config->register_value = 0u;
        config->parent_hz = 0u;
        config->actual_hz = 0u;
        config->source = 0u;
        config->m_divider = 0u;
        config->n_exponent = 0u;
    }
    if (config == 0 || target_hz == 0u || osc24m_hz == 0u)
        return SUNXI_MMC_MODULE_CLOCK_INVALID;
    if (target_hz <= SUNXI_MMC_MODULE_CLOCK_OSC24M_LIMIT_HZ) {
        parent_hz = osc24m_hz;
        source = SUNXI_MMC_MODULE_CLOCK_OSC24M;
    } else {
        parent_hz = pll6_effective_hz;
        source = SUNXI_MMC_MODULE_CLOCK_PLL6;
    }
    if (parent_hz == 0u)
        return SUNXI_MMC_MODULE_CLOCK_INVALID;
    divider = (uint64_t)(parent_hz / target_hz);
    if ((parent_hz % target_hz) != 0u)
        ++divider;
    if (divider == 0u)
        divider = 1u;
    /* Mirror U-Boot: encode power-of-two factors in N, remainder in M. */
    while (divider > 16u && n <= 3u) {
        ++n;
        divider = (divider + 1u) / 2u;
    }
    if (n > 3u || divider == 0u || divider > 16u)
        return SUNXI_MMC_MODULE_CLOCK_OUT_OF_RANGE;
    m_divider = (uint32_t)divider;
    actual_hz = parent_hz / (1u << n) / m_divider;
    if (actual_hz == 0u || actual_hz > target_hz)
        return SUNXI_MMC_MODULE_CLOCK_OUT_OF_RANGE;
    config->register_value = MMC_CLK_REG_ENABLE |
                             (source << MMC_CLK_REG_MUX_SHIFT) |
                             (n << MMC_CLK_REG_N_SHIFT) |
                             ((m_divider - 1u) & MMC_CLK_REG_M_MASK);
    config->parent_hz = parent_hz;
    config->actual_hz = actual_hz;
    config->source = (uint8_t)source;
    config->m_divider = (uint8_t)m_divider;
    config->n_exponent = (uint8_t)n;
    return SUNXI_MMC_MODULE_CLOCK_OK;
}

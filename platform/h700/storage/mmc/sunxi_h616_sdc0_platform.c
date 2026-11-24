#include "sunxi_h616_sdc0_platform.h"
#include "sunxi_sd_card_init.h"

#include "sunxi_mmc_module_clock.h"
#include "sunxi_mmc_regs.h"
#include "sunxi_mmc_clock.h"

#define SUNXI_CCU_APB1_CFG          0x520u
#define SUNXI_CCU_APB1_M_MASK       (0x3u << 0)
#define SUNXI_CCU_APB1_N_MASK       (0x3u << 8)
#define SUNXI_CCU_APB1_MUX_MASK     (0x3u << 24)
#define SUNXI_CCU_APB1_CFG_MASK     (SUNXI_CCU_APB1_M_MASK | \
                                    SUNXI_CCU_APB1_N_MASK | \
                                    SUNXI_CCU_APB1_MUX_MASK)
#define SUNXI_H616_OSC24M_HZ        24000000u

static int platform_valid(const struct sunxi_h616_sdc0_platform *platform)
{
    return platform != 0 &&
           platform->mmc_io != 0 &&
           platform->ccu.read32 != 0 &&
           platform->ccu.write32 != 0 &&
           platform->osc24m_hz != 0u;
}

int sunxi_h616_pio_enable_apb1_24m(
    const struct sunxi_h616_sdc0_platform *platform)
{
    uint32_t value;

    if (!platform_valid(platform) ||
        platform->osc24m_hz != SUNXI_H616_OSC24M_HZ)
        return -1;

    /*
     * APB1 parent 0 is osc24M; M=0 encodes divide-by-1 and N=0 encodes /1.
     * Preserve unrelated CCU bits, then verify the selected clock fields.
     */
    value = platform->ccu.read32(platform->ccu.context,
                                 SUNXI_CCU_APB1_CFG);
    value &= ~SUNXI_CCU_APB1_CFG_MASK;
    platform->ccu.write32(platform->ccu.context,
                          SUNXI_CCU_APB1_CFG, value);

    value = platform->ccu.read32(platform->ccu.context,
                                 SUNXI_CCU_APB1_CFG);
    return (value & SUNXI_CCU_APB1_CFG_MASK) == 0u ? 0 : -1;
}

int sunxi_h616_sdc0_enable_bus(
    const struct sunxi_h616_sdc0_platform *platform)
{
    uint32_t value;

    if (!platform_valid(platform))
        return -1;

    value = platform->ccu.read32(platform->ccu.context,
                                 SUNXI_CCU_MMC_GATE_RESET);
    value |= SUNXI_CCU_MMC_GATE_BIT(0);
    value |= SUNXI_CCU_MMC_RESET_BIT(0);
    platform->ccu.write32(platform->ccu.context,
                          SUNXI_CCU_MMC_GATE_RESET, value);

    value = platform->ccu.read32(platform->ccu.context,
                                 SUNXI_CCU_MMC_GATE_RESET);
    if ((value & (SUNXI_CCU_MMC_GATE_BIT(0) |
                  SUNXI_CCU_MMC_RESET_BIT(0))) !=
        (SUNXI_CCU_MMC_GATE_BIT(0) | SUNXI_CCU_MMC_RESET_BIT(0)))
        return -1;

    return 0;
}

int sunxi_h616_sdc0_set_clock_hz(
    void *context,
    uint32_t target_hz,
    uint32_t update_timeout_ms)
{
    const struct sunxi_h616_sdc0_platform *platform =
        (const struct sunxi_h616_sdc0_platform *)context;
    struct sunxi_mmc_module_clock_config module_clock;
    uint32_t clkcr;
    int result;

    if (!platform_valid(platform) || update_timeout_ms == 0u ||
        target_hz != SUNXI_SD_IDENTIFICATION_CLOCK_HZ)
        return -1;

    result = sunxi_mmc_calculate_module_clock(
        target_hz,
        platform->osc24m_hz,
        platform->pll6_effective_hz,
        &module_clock);
    if (result != SUNXI_MMC_MODULE_CLOCK_OK)
        return -1;

    /*
     * Mirror U-Boot's clock change ordering. H616 requires MASK_DATA0 for
     * both update commands; the helper clears it on success and timeout.
     */
    result = sunxi_mmc_set_output_clock(platform->mmc_io, false, true,
                                        update_timeout_ms);
    if (result != SUNXI_MMC_CLOCK_OK)
        return -1;

    platform->ccu.write32(platform->ccu.context,
                          SUNXI_CCU_MMC0_CLK_CFG,
                          module_clock.register_value);

    clkcr = platform->mmc_io->read32(platform->mmc_io->context,
                                     SUNXI_MMC_REG_CLKCR);
    clkcr &= ~SUNXI_MMC_CLKCR_DIVIDER_MASK;
    platform->mmc_io->write32(platform->mmc_io->context,
                              SUNXI_MMC_REG_CLKCR, clkcr);

    result = sunxi_mmc_set_output_clock(platform->mmc_io, true, true,
                                        update_timeout_ms);
    if (result != SUNXI_MMC_CLOCK_OK)
        return -1;

    return 0;
}
